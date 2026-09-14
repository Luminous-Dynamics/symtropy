// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Stable 2D transition evidence derived from an exact applied friction token.
//!
//! This module closes the gap between caller-capturable before/after transition
//! snapshots and the non-cloneable exactly-once friction lifecycle token. The
//! pre-transition velocity state is reconstructed algebraically from the exact
//! impulse recorded by the applied transaction and its #1041-bound live post
//! state. No absolute kinetic-energy subtraction is used.

use symtropy_math::Bivector;

use crate::body::RigidBody;
use crate::friction_transaction::AppliedFrictionTransaction;
use crate::friction_transition_energy_2d::{
    FrictionPairTransitionEnergy2d, FrictionTransitionEnergy2dError,
    capture_friction_pair_transition_basis_2d_checked,
    classify_friction_pair_transition_2d_checked,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AppliedFrictionTransition2dError {
    StaleMechanicalState,
    Transition(FrictionTransitionEnergy2dError),
}

impl From<FrictionTransitionEnergy2dError> for AppliedFrictionTransition2dError {
    fn from(value: FrictionTransitionEnergy2dError) -> Self {
        Self::Transition(value)
    }
}

/// Derive the checked stable 2D mechanical-energy transition for one exact
/// already-applied friction transaction.
///
/// The applied token must still match the complete bound post-mechanical state.
/// The unique pre-transition velocities are then reconstructed by reversing the
/// exact current 2D solver impulse update:
///
/// - body A received `-J`, so `v_pre_a = v_post_a + J * inv_mass_a`;
/// - body B received `+J`, so `v_pre_b = v_post_b - J * inv_mass_b`;
/// - angular pre-state reverses the corresponding lever-arm bivector impulse
///   using the same mean-inverse-inertia convention as `apply_angular_impulse`.
///
/// #1041's bound-state theorem guarantees that body type, position, mass,
/// inverse mass, inertia and inverse inertia have not changed since application,
/// so these reconstructed coefficients are the coefficients that governed the
/// actual transition.
pub fn classify_applied_friction_transition_2d_checked(
    body_a: &RigidBody<2>,
    body_b: &RigidBody<2>,
    applied: &AppliedFrictionTransaction<2>,
) -> Result<FrictionPairTransitionEnergy2d, AppliedFrictionTransition2dError> {
    if !applied.matches_post_state(body_a, body_b) {
        return Err(AppliedFrictionTransition2dError::StaleMechanicalState);
    }

    let after = capture_friction_pair_transition_basis_2d_checked(body_a, body_b)?;
    let mut before = after;
    let observation = applied.observation();
    let impulse_on_b = observation.impulse_on_b;

    before.body_a.linear_velocity = after.body_a.linear_velocity + impulse_on_b * body_a.inv_mass;
    before.body_b.linear_velocity = after.body_b.linear_velocity - impulse_on_b * body_b.inv_mass;

    let r_a = observation.contact_point - body_a.position();
    let r_b = observation.contact_point - body_b.position();
    let torque_a = Bivector::from_wedge(&(-impulse_on_b), &r_a);
    let torque_b = Bivector::from_wedge(&impulse_on_b, &r_b);

    let inverse_inertia_a = if body_a.is_dynamic() {
        1.0 / after.body_a.effective_inertia
    } else {
        0.0
    };
    let inverse_inertia_b = if body_b.is_dynamic() {
        1.0 / after.body_b.effective_inertia
    } else {
        0.0
    };
    before.body_a.angular_velocity =
        after.body_a.angular_velocity - torque_a.get(0, 1) * inverse_inertia_a;
    before.body_b.angular_velocity =
        after.body_b.angular_velocity - torque_b.get(0, 1) * inverse_inertia_b;

    classify_friction_pair_transition_2d_checked(before, after).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BodyHandle, BodyType};
    use crate::friction_evidence::{FrictionMechanicalDelta, FrictionTransactionId};
    use crate::friction_transaction::{FrictionTransactionJournal, apply_friction_impulse_once};
    use crate::friction_transition_energy_2d::{
        FrictionTransitionDelta2d, capture_friction_pair_transition_basis_2d_checked,
        classify_friction_pair_transition_2d_checked,
    };
    use nalgebra::SVector;
    use symtropy_math::{Point, Sphere, Transform};

    fn anisotropic_body(
        handle: usize,
        position: [f64; 2],
        velocity: [f64; 2],
        angular_velocity: f64,
    ) -> RigidBody<2> {
        let mut body = RigidBody::new(
            BodyHandle(handle),
            BodyType::Dynamic,
            Transform::from_translation(Point::new(position)),
            Box::new(Sphere::<2>::unit()),
            1.0,
            SVector::from([2.0, 8.0]),
        );
        body.linear_velocity = SVector::from(velocity);
        body.angular_velocity.set(0, 1, angular_velocity);
        body
    }

    #[test]
    fn token_bound_transition_matches_independent_stable_reference() {
        let mut a = anisotropic_body(1, [-1.0, 0.0], [1.0, 0.2], 2.0);
        let mut b = anisotropic_body(2, [1.0, 0.0], [0.0, -0.1], -1.0);
        let contact = SVector::from([0.0, 0.5]);
        let impulse = SVector::from([0.0, -0.1]);
        let before = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();
        let mut journal = FrictionTransactionJournal::new();

        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &contact,
            &impulse,
            FrictionTransactionId::new(3, 1, 4, 0),
            &mut journal,
        )
        .unwrap();

        let after = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();
        let independent = classify_friction_pair_transition_2d_checked(before, after).unwrap();
        let bound = classify_applied_friction_transition_2d_checked(&a, &b, &applied).unwrap();
        assert_eq!(bound, independent);
    }

    #[test]
    fn token_bound_transition_recovers_loss_erased_by_absolute_observation() {
        let mut a = anisotropic_body(1, [0.0, 0.0], [1.0, 0.0], 100_000_000.0);
        let mut b = anisotropic_body(2, [0.0, 0.0], [0.0, 0.0], 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0]),
            FrictionTransactionId::new(4, 0, 0, 0),
            &mut journal,
        )
        .unwrap();

        assert_eq!(applied.observation().delta, FrictionMechanicalDelta::Neutral);
        let stable = classify_applied_friction_transition_2d_checked(&a, &b, &applied).unwrap();
        assert_eq!(stable.pair_delta_solver, -0.25);
        assert_eq!(stable.kinetic_change_a_solver, -0.375);
        assert_eq!(stable.kinetic_change_b_solver, 0.125);
        assert_eq!(
            stable.delta,
            FrictionTransitionDelta2d::DissipationCandidate { solver_energy: 0.25 }
        );
    }

    #[test]
    fn token_bound_transition_refuses_reinterpreted_post_state() {
        let mut a = anisotropic_body(1, [0.0, 0.0], [1.0, 0.0], 2.0);
        let mut b = anisotropic_body(2, [0.0, 0.0], [0.0, 0.0], 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0]),
            FrictionTransactionId::new(5, 0, 0, 0),
            &mut journal,
        )
        .unwrap();

        a.inertia *= 2.0;
        a.inv_inertia *= 0.5;
        assert_eq!(
            classify_applied_friction_transition_2d_checked(&a, &b, &applied),
            Err(AppliedFrictionTransition2dError::StaleMechanicalState)
        );
    }
}
