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
//! A freshly `Applied` transaction may be rolled back to `Absent` only by
//! consuming its unique application token while its exact post-mechanical state
//! is still current. This is the narrow abort path used by an outer atomic
//! authority when terminalization itself rejects.

use std::collections::BTreeMap;

use nalgebra::SVector;
use serde::{Deserialize, Serialize};
use symtropy_math::Bivector;

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
/// can advance `Applied -> Promoted`. The exact rollback API can remove only a
/// still-`Applied` entry while consuming its unique application token.
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

    /// True only when every recorded transaction belongs to `fixed_tick`.
    ///
    /// Empty journals are vacuously valid. This read-only theorem lets the outer
    /// #824 fixed-tick authority reject cross-tick friction evidence without
    /// exposing or making the journal's internal phase map caller-mutable.
    pub fn matches_fixed_tick(&self, fixed_tick: u64) -> bool {
        self.phases
            .keys()
            .all(|transaction_id| transaction_id.fixed_tick == fixed_tick)
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionApplicationRollbackError {
    UnknownTransaction,
    AlreadyTerminal,
    ObservationStateMismatch,
}

/// Non-cloneable proof that the canonical lifecycle journal admitted one
/// mechanical friction application for this exact deterministic transaction.
///
/// The token additionally retains the exact pre-application linear/angular
/// velocities needed by [`rollback_applied_friction_impulse`]. Those snapshots
/// are private and cannot be caller-authored. Rollback consumes this token.
#[derive(Debug, PartialEq)]
pub struct AppliedFrictionTransaction<const D: usize> {
    bound: BoundFrictionMechanicalObservation<D>,
    pre_linear_velocity_a: SVector<f64, D>,
    pre_linear_velocity_b: SVector<f64, D>,
    pre_angular_velocity_a: Bivector<D>,
    pre_angular_velocity_b: Bivector<D>,
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
/// `Absent -> Applied` and returns the only token accepted by physical promotion
/// or exact application rollback.
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

    let pre_linear_velocity_a = body_a.linear_velocity;
    let pre_linear_velocity_b = body_b.linear_velocity;
    let pre_angular_velocity_a = body_a.angular_velocity;
    let pre_angular_velocity_b = body_b.angular_velocity;

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

    Ok(AppliedFrictionTransaction {
        bound,
        pre_linear_velocity_a,
        pre_linear_velocity_b,
        pre_angular_velocity_a,
        pre_angular_velocity_b,
    })
}

/// Undo exactly one freshly applied friction transaction.
///
/// This is deliberately narrower than a generic journal deletion API. The unique
/// [`AppliedFrictionTransaction`] token must still match the exact live
/// post-mechanical A/B velocity state and the journal must still be `Applied`.
/// Only then are the private pre-application velocity snapshots restored and that
/// exact journal entry removed.
///
/// The primitive is intended for immediate outer transaction rollback after a
/// later terminalization step rejects. It refuses stale state and refuses to erase
/// already-terminal evidence.
pub fn rollback_applied_friction_impulse<const D: usize>(
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    applied: AppliedFrictionTransaction<D>,
    journal: &mut FrictionTransactionJournal,
) -> Result<(), FrictionApplicationRollbackError> {
    let id = applied.transaction_id();
    match journal.phase(id) {
        Some(FrictionTransactionPhase::Applied) => {}
        Some(FrictionTransactionPhase::Promoted)
        | Some(FrictionTransactionPhase::DiagnosticOnly(_)) => {
            return Err(FrictionApplicationRollbackError::AlreadyTerminal);
        }
        None => return Err(FrictionApplicationRollbackError::UnknownTransaction),
    }

    if !applied.matches_post_state(body_a, body_b) {
        return Err(FrictionApplicationRollbackError::ObservationStateMismatch);
    }

    body_a.linear_velocity = applied.pre_linear_velocity_a;
    body_b.linear_velocity = applied.pre_linear_velocity_b;
    body_a.angular_velocity = applied.pre_angular_velocity_a;
    body_b.angular_velocity = applied.pre_angular_velocity_b;

    let removed = journal.phases.remove(&id);
    debug_assert_eq!(removed, Some(FrictionTransactionPhase::Applied));
    Ok(())
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
        assert!(journal.matches_fixed_tick(12));
        assert!(!journal.matches_fixed_tick(13));

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
    fn exact_applied_rollback_restores_pre_state_and_only_removes_its_entry() {
        let prior_id = FrictionTransactionId::new(12, 0, 0, 0);
        let rollback_id = FrictionTransactionId::new(12, 0, 1, 0);
        let mut a = body(1, [-1.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [1.0, 0.0, 0.0], 0.0);
        let mut journal = FrictionTransactionJournal::new();

        let prior = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            prior_id,
            &mut journal,
        )
        .unwrap();
        assert_eq!(
            finalize_friction_diagnostic(&a, &b, &prior, &mut journal).unwrap(),
            FrictionDiagnosticReason::OffCenterUnqualified
        );

        let before_a = (a.linear_velocity, a.angular_velocity);
        let before_b = (b.linear_velocity, b.angular_velocity);
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.05, 0.0]),
            rollback_id,
            &mut journal,
        )
        .unwrap();
        assert_eq!(journal.phase(rollback_id), Some(FrictionTransactionPhase::Applied));

        rollback_applied_friction_impulse(&mut a, &mut b, applied, &mut journal).unwrap();

        assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
        assert_eq!(journal.phase(rollback_id), None);
        assert_eq!(
            journal.phase(prior_id),
            Some(FrictionTransactionPhase::DiagnosticOnly(
                FrictionDiagnosticReason::OffCenterUnqualified
            ))
        );
        assert!(journal.is_complete_for_finalize());
    }

    #[test]
    fn rollback_refuses_stale_post_mechanical_state() {
        let id = FrictionTransactionId::new(12, 0, 2, 0);
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
        a.linear_velocity[1] = 1.0;
        let stale_a = (a.linear_velocity, a.angular_velocity);
        let stale_b = (b.linear_velocity, b.angular_velocity);

        assert_eq!(
            rollback_applied_friction_impulse(&mut a, &mut b, applied, &mut journal),
            Err(FrictionApplicationRollbackError::ObservationStateMismatch)
        );
        assert_eq!((a.linear_velocity, a.angular_velocity), stale_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), stale_b);
        assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));
    }

    #[test]
    fn rollback_refuses_already_terminal_evidence() {
        let id = FrictionTransactionId::new(12, 0, 3, 0);
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
        finalize_friction_diagnostic(&a, &b, &applied, &mut journal).unwrap();
        let terminal_a = (a.linear_velocity, a.angular_velocity);
        let terminal_b = (b.linear_velocity, b.angular_velocity);

        assert_eq!(
            rollback_applied_friction_impulse(&mut a, &mut b, applied, &mut journal),
            Err(FrictionApplicationRollbackError::AlreadyTerminal)
        );
        assert_eq!((a.linear_velocity, a.angular_velocity), terminal_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), terminal_b);
        assert!(matches!(
            journal.phase(id),
            Some(FrictionTransactionPhase::DiagnosticOnly(_))
        ));
    }

    #[test]
    fn rollback_refuses_unknown_journal_identity_without_mutating_bodies() {
        let id = FrictionTransactionId::new(12, 0, 4, 0);
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let mut source_journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            id,
            &mut source_journal,
        )
        .unwrap();
        let post_a = (a.linear_velocity, a.angular_velocity);
        let post_b = (b.linear_velocity, b.angular_velocity);
        let mut wrong_journal = FrictionTransactionJournal::new();

        assert_eq!(
            rollback_applied_friction_impulse(&mut a, &mut b, applied, &mut wrong_journal),
            Err(FrictionApplicationRollbackError::UnknownTransaction)
        );
        assert_eq!((a.linear_velocity, a.angular_velocity), post_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), post_b);
        assert!(wrong_journal.is_empty());
        assert_eq!(source_journal.phase(id), Some(FrictionTransactionPhase::Applied));
    }

    #[test]
    fn mixed_fixed_ticks_are_visible_to_outer_authority() {
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let _first = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.25, 0.0, 0.0]),
            FrictionTransactionId::new(20, 0, 0, 0),
            &mut journal,
        )
        .unwrap();
        let _second = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.1, 0.0, 0.0]),
            FrictionTransactionId::new(21, 0, 1, 0),
            &mut journal,
        )
        .unwrap();

        assert!(!journal.matches_fixed_tick(20));
        assert!(!journal.matches_fixed_tick(21));
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
