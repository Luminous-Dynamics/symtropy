// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exactly-once lifecycle authority for friction solver transactions.
//!
//! A deterministic friction transaction advances monotonically:
//!
//! `Absent -> Applied -> Promoted`.
//!
//! The `Applied` transition occurs only after the mechanical impulse has been
//! successfully applied and bound to its solver identity. Physical promotion is
//! performed by the sibling `friction_promotion` module, which may advance only
//! `Applied -> Promoted` after thermal + ledger reconciliation succeeds.

use std::collections::BTreeMap;

use nalgebra::SVector;
use serde::{Deserialize, Serialize};

use crate::body::RigidBody;
use crate::friction_evidence::{
    BoundFrictionMechanicalObservation, FrictionEvidenceError, FrictionMechanicalObservation,
    FrictionTransactionId, apply_friction_impulse_measured_bound,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrictionTransactionPhase {
    Applied,
    Promoted,
}

/// Canonical lifecycle journal for friction transactions within a physics run.
///
/// The map is deterministic and serializable so the owning fixed-tick world can
/// include it in replay/checkpoint evidence. Public APIs are read-only except for
/// applying a new mechanical transaction; only the in-crate physical promotion
/// layer can advance an `Applied` transaction to `Promoted`.
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

    /// Number of mechanical transactions that have not yet completed physical
    /// promotion. A fixed-tick physical finalize gate can require this to be 0.
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
            Some(FrictionTransactionPhase::Promoted) => {
                Err(FrictionTransactionTransitionError::AlreadyPromoted)
            }
            None => Err(FrictionTransactionTransitionError::UnknownTransaction),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionTransactionTransitionError {
    UnknownTransaction,
    AlreadyPromoted,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use symtropy_math::Point;

    fn body(handle: usize, velocity_x: f64) -> RigidBody<3> {
        let mut body = RigidBody::dynamic_sphere(
            BodyHandle(handle),
            Point::origin(),
            0.5,
            1.0,
        );
        body.linear_velocity[0] = velocity_x;
        body
    }

    #[test]
    fn duplicate_identity_is_rejected_before_second_mechanical_mutation() {
        let id = FrictionTransactionId::new(12, 1, 3, 0);
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
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
    fn failed_evidence_application_does_not_claim_transaction_identity() {
        let id = FrictionTransactionId::new(13, 0, 0, 0);
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
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

    #[test]
    fn lifecycle_transition_is_monotonic_and_finalize_visible() {
        let id = FrictionTransactionId::new(14, 2, 1, 4);
        let mut journal = FrictionTransactionJournal::new();
        journal
            .phases
            .insert(id, FrictionTransactionPhase::Applied);
        assert!(!journal.is_complete_for_finalize());
        journal.mark_promoted(id).unwrap();
        assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Promoted));
        assert!(journal.is_complete_for_finalize());
        assert_eq!(
            journal.mark_promoted(id),
            Err(FrictionTransactionTransitionError::AlreadyPromoted)
        );
    }
}