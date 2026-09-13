// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exactly-once lifecycle authority for friction solver transactions.
//!
//! A deterministic friction transaction advances monotonically from `Absent`
//! to `Applied`, then to exactly one terminal outcome:
//!
//! - `Promoted` when a centered closed-pair measured loss is committed as heat;
//! - `DiagnosticOnly(reason)` when the observation itself proves physical heat
//!   promotion is not currently admissible.
//!
//! A centered measured loss cannot be terminalized as diagnostic-only.

use std::collections::BTreeMap;

use nalgebra::SVector;
use serde::{Deserialize, Serialize};

use crate::body::RigidBody;
use crate::friction_evidence::{
    BoundFrictionMechanicalObservation, FrictionEvidenceError, FrictionEvidenceRegime,
    FrictionMechanicalDelta, FrictionMechanicalObservation, FrictionTransactionId,
    apply_friction_impulse_measured_bound, classify_friction_evidence_regime,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrictionDiagnosticReason {
    OffCenterUnqualified,
    ExternalBoundaryUnqualified,
    SolverInjection,
    Neutral,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrictionTransactionPhase {
    Applied,
    Promoted,
    DiagnosticOnly(FrictionDiagnosticReason),
}

impl FrictionTransactionPhase {
    pub fn is_terminal(self) -> bool {
        !matches!(self, Self::Applied)
    }
}

/// Canonical lifecycle journal for friction transactions within a physics run.
///
/// The map is deterministic and serializable so the owning fixed-tick world can
/// include it in replay/checkpoint evidence. Public APIs are read-only except for
/// applying a new mechanical transaction and safely terminalizing an observation
/// that is provably non-promotable. Only the in-crate physical promotion layer
/// can advance `Applied -> Promoted`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrictionTransactionJournal {
    phases: BTreeMap<FrictionTransactionId, FrictionTransactionPhase>,
}

impl FrictionTransactionJournal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn phase(
        &self,
        transaction_id: FrictionTransactionId,
    ) -> Option<FrictionTransactionPhase> {
        self.phases.get(&transaction_id).copied()
    }

    pub fn len(&self) -> usize {
        self.phases.len()
    }

    pub fn is_empty(&self) -> bool {
        self.phases.is_empty()
    }

    /// Number of transactions that have applied mechanics but have not yet
    /// reached an admissible terminal physical/diagnostic outcome.
    pub fn pending_application_count(&self) -> usize {
        self.phases
            .values()
            .filter(|phase| **phase == FrictionTransactionPhase::Applied)
            .count()
    }

    pub fn is_complete_for_finalize(&self) -> bool {
        self.pending_application_count() == 0
    }

    pub(crate) fn mark_promoted(
        &mut self,
        transaction_id: FrictionTransactionId,
    ) -> Result<(), FrictionTransactionTransitionError> {
        match self.phases.get_mut(&transaction_id) {
            Some(phase @ FrictionTransactionPhase::Applied) => {
                *phase = FrictionTransactionPhase::Promoted;
                Ok(())
            }
            Some(FrictionTransactionPhase::Promoted)
            | Some(FrictionTransactionPhase::DiagnosticOnly(_)) => {
                Err(FrictionTransactionTransitionError::AlreadyTerminal)
            }
            None => Err(FrictionTransactionTransitionError::UnknownTransaction),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionTransactionTransitionError {
    UnknownTransaction,
    AlreadyTerminal,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionApplicationError {
    DuplicateTransaction,
    Evidence(FrictionEvidenceError),
}

impl From<FrictionEvidenceError> for FrictionApplicationError {
    fn from(value: FrictionEvidenceError) -> Self {
        Self::Evidence(value)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionDiagnosticFinalizeError {
    UnknownTransaction,
    AlreadyTerminal,
    RequiresPhysicalPromotion,
    ObservationStateMismatch,
}

/// Non-cloneable proof that the canonical lifecycle journal admitted one
/// mechanical friction application for this exact deterministic transaction.
#[derive(Debug, PartialEq)]
pub struct AppliedFrictionTransaction<const D: usize> {
    bound: BoundFrictionMechanicalObservation<D>,
}

impl<const D: usize> AppliedFrictionTransaction<D> {
    pub const fn transaction_id(&self) -> FrictionTransactionId {
        self.bound.transaction_id()
    }

    pub fn observation(&self) -> &FrictionMechanicalObservation<D> {
        self.bound.observation()
    }

    pub(crate) fn matches_post_state(
        &self,
        body_a: &RigidBody<D>,
        body_b: &RigidBody<D>,
    ) -> bool {
        self.bound.matches_post_state(body_a, body_b)
    }
}

/// Apply one friction impulse only if its deterministic transaction identity is
/// absent from the canonical lifecycle journal.
///
/// Duplicate identity is rejected before any mechanical mutation. If evidence
/// construction fails, the underlying bound primitive rolls the mechanical
/// state back and the journal remains unchanged. On success the journal advances
/// `Absent -> Applied` and returns the only token accepted by physical promotion.
pub fn apply_friction_impulse_once<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    contact_point: &SVector<f64, D>,
    impulse_on_b: &SVector<f64, D>,
    transaction_id: FrictionTransactionId,
    journal: &mut FrictionTransactionJournal,
) -> Result<AppliedFrictionTransaction<D>, FrictionApplicationError> {
    if journal.phase(transaction_id).is_some() {
        return Err(FrictionApplicationError::DuplicateTransaction);
    }

    let bound = apply_friction_impulse_measured_bound(
        body_a,
        body_b,
        contact_point,
        impulse_on_b,
        transaction_id,
    )?;

    let previous = journal
        .phases
        .insert(transaction_id, FrictionTransactionPhase::Applied);
    debug_assert!(previous.is_none());

    Ok(AppliedFrictionTransaction { bound })
}

fn diagnostic_reason<const D: usize>(
    applied: &AppliedFrictionTransaction<D>,
) -> Result<FrictionDiagnosticReason, FrictionDiagnosticFinalizeError> {
    let observation = applied.observation();
    match observation.regime {
        FrictionEvidenceRegime::OffCenterUnqualified => {
            Ok(FrictionDiagnosticReason::OffCenterUnqualified)
        }
        FrictionEvidenceRegime::ExternalBoundaryUnqualified => {
            Ok(FrictionDiagnosticReason::ExternalBoundaryUnqualified)
        }
        FrictionEvidenceRegime::CenteredClosedDynamicPair => match observation.delta {
            FrictionMechanicalDelta::SolverInjection { .. } => {
                Ok(FrictionDiagnosticReason::SolverInjection)
            }
            FrictionMechanicalDelta::Neutral => Ok(FrictionDiagnosticReason::Neutral),
            FrictionMechanicalDelta::DissipationCandidate { .. } => {
                Err(FrictionDiagnosticFinalizeError::RequiresPhysicalPromotion)
            }
        },
    }
}

/// Terminalize a transaction without physical heat only when its bound
/// observation proves that heat promotion is not currently admissible.
///
/// This is not a caller-selected escape hatch: the reason is derived from the
/// immutable observation. A centered measured loss is rejected and remains
/// pending until it is physically promoted or the enclosing tick is rolled back.
pub fn finalize_friction_diagnostic<const D: usize>(
    body_a: &RigidBody<D>,
    body_b: &RigidBody<D>,
    applied: &AppliedFrictionTransaction<D>,
    journal: &mut FrictionTransactionJournal,
) -> Result<FrictionDiagnosticReason, FrictionDiagnosticFinalizeError> {
    let id = applied.transaction_id();
    match journal.phase(id) {
        Some(FrictionTransactionPhase::Applied) => {}
        Some(FrictionTransactionPhase::Promoted)
        | Some(FrictionTransactionPhase::DiagnosticOnly(_)) => {
            return Err(FrictionDiagnosticFinalizeError::AlreadyTerminal);
        }
        None => return Err(FrictionDiagnosticFinalizeError::UnknownTransaction),
    }

    if !applied.matches_post_state(body_a, body_b) {
        return Err(FrictionDiagnosticFinalizeError::ObservationStateMismatch);
    }

    let observation = applied.observation();
    let current_regime = classify_friction_evidence_regime(
        body_a,
        body_b,
        &observation.contact_point,
    )
    .map_err(|_| FrictionDiagnosticFinalizeError::ObservationStateMismatch)?;
    if current_regime != observation.regime {
        return Err(FrictionDiagnosticFinalizeError::ObservationStateMismatch);
    }

    let reason = diagnostic_reason(applied)?;
    let previous = journal
        .phases
        .insert(id, FrictionTransactionPhase::DiagnosticOnly(reason));
    debug_assert_eq!(previous, Some(FrictionTransactionPhase::Applied));
    Ok(reason)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use symtropy_math::Point;

    fn body(handle: usize, position: [f64; 3], velocity_x: f64) -> RigidBody<3> {
        let mut body = RigidBody::dynamic_sphere(
            BodyHandle(handle),
            Point::new(position),
            0.5,
            1.0,
        );
        body.linear_velocity[0] = velocity_x;
        body
    }

    #[test]
    fn duplicate_identity_is_rejected_before_second_mechanical_mutation() {
        let id = FrictionTransactionId::new(12, 1, 3, 0);
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let mut journal = FrictionTransactionJournal::new();

        let first = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            id,
            &mut journal,
        )
        .unwrap();
        assert_eq!(first.transaction_id(), id);
        assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));

        let state_a = (a.linear_velocity, a.angular_velocity);
        let state_b = (b.linear_velocity, b.angular_velocity);
        assert_eq!(
            apply_friction_impulse_once(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.5, 0.0, 0.0]),
                id,
                &mut journal,
            ),
            Err(FrictionApplicationError::DuplicateTransaction)
        );
        assert_eq!((a.linear_velocity, a.angular_velocity), state_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), state_b);
        assert_eq!(journal.pending_application_count(), 1);
        assert!(!journal.is_complete_for_finalize());
    }

    #[test]
    fn off_center_observation_can_terminalize_as_diagnostic() {
        let id = FrictionTransactionId::new(13, 0, 1, 2);
        let mut a = body(1, [-1.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [1.0, 0.0, 0.0], 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            id,
            &mut journal,
        )
        .unwrap();

        assert_eq!(
            finalize_friction_diagnostic(&a, &b, &applied, &mut journal).unwrap(),
            FrictionDiagnosticReason::OffCenterUnqualified
        );
        assert_eq!(
            journal.phase(id),
            Some(FrictionTransactionPhase::DiagnosticOnly(
                FrictionDiagnosticReason::OffCenterUnqualified
            ))
        );
        assert!(journal.is_complete_for_finalize());
    }

    #[test]
    fn diagnostic_finalize_rejects_changed_contact_regime() {
        let id = FrictionTransactionId::new(14, 0, 1, 0);
        let mut a = body(1, [-1.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [1.0, 0.0, 0.0], 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            id,
            &mut journal,
        )
        .unwrap();

        // Move both centers onto the recorded contact point without changing
        // the bound velocities. Geometry is now centered, so the old off-center
        // classification is stale and cannot be terminalized.
        a.transform.translation = Point::new([0.0, 0.5, 0.0]);
        b.transform.translation = Point::new([0.0, 0.5, 0.0]);
        assert_eq!(
            finalize_friction_diagnostic(&a, &b, &applied, &mut journal),
            Err(FrictionDiagnosticFinalizeError::ObservationStateMismatch)
        );
        assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));
    }

    #[test]
    fn centered_measured_loss_cannot_escape_through_diagnostic_path() {
        let id = FrictionTransactionId::new(15, 0, 0, 0);
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            id,
            &mut journal,
        )
        .unwrap();

        assert_eq!(
            finalize_friction_diagnostic(&a, &b, &applied, &mut journal),
            Err(FrictionDiagnosticFinalizeError::RequiresPhysicalPromotion)
        );
        assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));
        assert!(!journal.is_complete_for_finalize());
    }

    #[test]
    fn solver_injection_terminalizes_without_becoming_heat() {
        let id = FrictionTransactionId::new(16, 0, 0, 0);
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([2.0, 0.0, 0.0]),
            id,
            &mut journal,
        )
        .unwrap();

        assert_eq!(
            finalize_friction_diagnostic(&a, &b, &applied, &mut journal).unwrap(),
            FrictionDiagnosticReason::SolverInjection
        );
        assert!(journal.is_complete_for_finalize());
    }

    #[test]
    fn failed_evidence_application_does_not_claim_transaction_identity() {
        let id = FrictionTransactionId::new(17, 0, 0, 0);
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity);
        let before_b = (b.linear_velocity, b.angular_velocity);
        let mut journal = FrictionTransactionJournal::new();

        let result = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([f64::NAN, 0.0, 0.0]),
            id,
            &mut journal,
        );
        assert!(matches!(
            result,
            Err(FrictionApplicationError::Evidence(
                FrictionEvidenceError::NonFiniteImpulse
            ))
        ));
        assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
        assert_eq!(journal.phase(id), None);
        assert!(journal.is_complete_for_finalize());
    }
}