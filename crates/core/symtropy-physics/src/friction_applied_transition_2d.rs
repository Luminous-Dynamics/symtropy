// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Stable 2D transition evidence bound to an exact applied friction transaction.
//!
//! The caller supplies no before-state snapshot. The non-cloneable
//! `AppliedFrictionTransaction` must still match the complete bound mechanical
//! post-state, and its privately retained pre-impulse velocities are combined
//! with the still-bound mass/inertia interpretation to reconstruct the stable
//! transition basis.

use crate::body::RigidBody;
use crate::friction_transaction::AppliedFrictionTransaction;
use crate::friction_transition_energy_2d::{
    FrictionPairTransitionBasis2d, FrictionPairTransitionEnergy2d,
    FrictionTransitionEnergy2dError, capture_friction_pair_transition_basis_2d_checked,
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

/// Measure the stable solver-consistent 2D energy transition authored by one
/// exact applied friction token.
///
/// This is measurement-only. It does not terminalize the friction journal,
/// mutate heat, write the physical ledger, or calibrate solver energy to Joules.
pub fn measure_applied_friction_transition_2d_checked(
    applied: &AppliedFrictionTransaction<2>,
    body_a: &RigidBody<2>,
    body_b: &RigidBody<2>,
) -> Result<FrictionPairTransitionEnergy2d, AppliedFrictionTransition2dError> {
    if !applied.matches_post_state(body_a, body_b) {
        return Err(AppliedFrictionTransition2dError::StaleMechanicalState);
    }

    let after = capture_friction_pair_transition_basis_2d_checked(body_a, body_b)?;
    let observation = applied.observation();

    // #1041 proves the mechanical parameters/positions/body types represented by
    // `after` are exactly the state bound at application time. The friction
    // mechanics kernel mutates only linear/angular velocity, so reconstructing
    // `before` requires replacing only those privately retained coordinates.
    let mut before: FrictionPairTransitionBasis2d = after;
    before.body_a.linear_velocity = observation.pre_linear_velocity_a();
    before.body_b.linear_velocity = observation.pre_linear_velocity_b();
    before.body_a.angular_velocity = observation.pre_angular_velocity_a().get(0, 1);
    before.body_b.angular_velocity = observation.pre_angular_velocity_b().get(0, 1);

    classify_friction_pair_transition_2d_checked(before, after).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BodyHandle, BodyType};
    use crate::friction_evidence::{FrictionMechanicalDelta, FrictionTransactionId};
    use crate::friction_transaction::{FrictionTransactionJournal, apply_friction_impulse_once};
    use crate::friction_transition_energy_2d::FrictionTransitionDelta2d;
    use nalgebra::SVector;
    use symtropy_math::{Point, Sphere, Transform};

    fn anisotropic_body(handle: usize, velocity_x: f64, angular_velocity: f64) -> RigidBody<2> {
        let mut body = RigidBody::new(
            BodyHandle(handle),
            BodyType::Dynamic,
            Transform::from_translation(Point::origin()),
            Box::new(Sphere::<2>::unit()),
            1.0,
            SVector::from([2.0, 8.0]),
        );
        body.linear_velocity[0] = velocity_x;
        body.angular_velocity.set(0, 1, angular_velocity);
        body
    }

    fn apply_centered(
        angular_velocity: f64,
    ) -> (
        RigidBody<2>,
        RigidBody<2>,
        AppliedFrictionTransaction<2>,
        FrictionTransactionJournal,
    ) {
        let mut a = anisotropic_body(1, 1.0, angular_velocity);
        let mut b = anisotropic_body(2, 0.0, 0.0);
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0]),
            FrictionTransactionId::new(7, 1, 2, 3),
            &mut journal,
        )
        .unwrap();
        (a, b, applied, journal)
    }

    #[test]
    fn nominal_applied_transition_is_exactly_bound_to_private_pre_state() {
        let (a, b, applied, _) = apply_centered(2.0);
        let stable = measure_applied_friction_transition_2d_checked(&applied, &a, &b).unwrap();

        assert_eq!(stable.kinetic_change_a_solver, -0.375);
        assert_eq!(stable.kinetic_change_b_solver, 0.125);
        assert_eq!(stable.pair_delta_solver, -0.25);
        assert_eq!(
            stable.delta,
            FrictionTransitionDelta2d::DissipationCandidate { solver_energy: 0.25 }
        );
    }

    #[test]
    fn applied_stable_measurement_corrects_wrong_but_representable_absolute_amount() {
        let (a, b, applied, _) = apply_centered(40_000_000.0);
        assert_eq!(
            applied.observation().delta,
            FrictionMechanicalDelta::DissipationCandidate { joules: 0.5 }
        );

        let stable = measure_applied_friction_transition_2d_checked(&applied, &a, &b).unwrap();
        assert_eq!(stable.pair_delta_solver, -0.25);
        assert_eq!(
            stable.delta,
            FrictionTransitionDelta2d::DissipationCandidate { solver_energy: 0.25 }
        );
    }

    #[test]
    fn applied_stable_measurement_recovers_loss_erased_to_neutral_by_absolute_subtraction() {
        let (a, b, applied, _) = apply_centered(100_000_000.0);
        assert_eq!(applied.observation().delta, FrictionMechanicalDelta::Neutral);

        let stable = measure_applied_friction_transition_2d_checked(&applied, &a, &b).unwrap();
        assert_eq!(stable.pair_delta_solver, -0.25);
        assert_eq!(
            stable.delta,
            FrictionTransitionDelta2d::DissipationCandidate { solver_energy: 0.25 }
        );
    }

    #[test]
    fn later_inertia_reinterpretation_is_stale_before_stable_measurement() {
        let (mut a, b, applied, _) = apply_centered(2.0);
        a.inertia[0] *= 2.0;
        a.inv_inertia[0] *= 0.5;

        assert_eq!(
            measure_applied_friction_transition_2d_checked(&applied, &a, &b),
            Err(AppliedFrictionTransition2dError::StaleMechanicalState)
        );
    }
}
