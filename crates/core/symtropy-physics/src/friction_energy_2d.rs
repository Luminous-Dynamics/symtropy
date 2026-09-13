// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked 2D pair-energy evidence for production friction diagnostics.
//!
//! This module deliberately stops short of physical heat authority. It captures
//! ordered A/B mechanical-energy snapshots using the current solver-consistent 2D
//! kinetic-energy interpretation and classifies the signed pair delta. Promotion
//! into heat still requires the separate regime/transaction/thermal authority.

use crate::body::{BodyHandle, RigidBody};
use crate::body_energy_2d::RigidBodyEnergy2dError;

const ENERGY_EPSILON_J: f64 = 1.0e-15;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrictionPairEnergy2dSnapshot {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub kinetic_a_joules: f64,
    pub kinetic_b_joules: f64,
    pub pair_total_joules: f64,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FrictionPairEnergyDelta2d {
    DissipationCandidate { joules: f64 },
    SolverInjection { joules: f64 },
    Neutral,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrictionPairEnergyChange2d {
    pub before: FrictionPairEnergy2dSnapshot,
    pub after: FrictionPairEnergy2dSnapshot,
    /// `K_after_pair - K_before_pair`.
    pub pair_delta_joules: f64,
    pub delta: FrictionPairEnergyDelta2d,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionPairEnergy2dError {
    SameBody,
    BodyA(RigidBodyEnergy2dError),
    BodyB(RigidBodyEnergy2dError),
    UnrepresentablePairTotal,
    SnapshotIdentityMismatch,
    UnrepresentablePairDelta,
}

pub fn capture_friction_pair_energy_2d_checked(
    body_a: &RigidBody<2>,
    body_b: &RigidBody<2>,
) -> Result<FrictionPairEnergy2dSnapshot, FrictionPairEnergy2dError> {
    if body_a.handle == body_b.handle {
        return Err(FrictionPairEnergy2dError::SameBody);
    }
    let kinetic_a = body_a
        .kinetic_energy_2d_solver_checked()
        .map_err(FrictionPairEnergy2dError::BodyA)?;
    let kinetic_b = body_b
        .kinetic_energy_2d_solver_checked()
        .map_err(FrictionPairEnergy2dError::BodyB)?;
    let pair_total = kinetic_a + kinetic_b;
    if !pair_total.is_finite() || pair_total < 0.0 {
        return Err(FrictionPairEnergy2dError::UnrepresentablePairTotal);
    }

    Ok(FrictionPairEnergy2dSnapshot {
        body_a: body_a.handle,
        body_b: body_b.handle,
        kinetic_a_joules: kinetic_a,
        kinetic_b_joules: kinetic_b,
        pair_total_joules: pair_total,
    })
}

pub fn classify_friction_pair_energy_change_2d_checked(
    before: FrictionPairEnergy2dSnapshot,
    after: FrictionPairEnergy2dSnapshot,
) -> Result<FrictionPairEnergyChange2d, FrictionPairEnergy2dError> {
    if before.body_a != after.body_a || before.body_b != after.body_b {
        return Err(FrictionPairEnergy2dError::SnapshotIdentityMismatch);
    }

    let pair_delta = after.pair_total_joules - before.pair_total_joules;
    if !pair_delta.is_finite() {
        return Err(FrictionPairEnergy2dError::UnrepresentablePairDelta);
    }

    let delta = if pair_delta < -ENERGY_EPSILON_J {
        FrictionPairEnergyDelta2d::DissipationCandidate {
            joules: -pair_delta,
        }
    } else if pair_delta > ENERGY_EPSILON_J {
        FrictionPairEnergyDelta2d::SolverInjection { joules: pair_delta }
    } else {
        FrictionPairEnergyDelta2d::Neutral
    };

    Ok(FrictionPairEnergyChange2d {
        before,
        after,
        pair_delta_joules: pair_delta,
        delta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BodyType, RigidBody};
    use crate::integrator;
    use nalgebra::SVector;
    use symtropy_math::{Point, Sphere, Transform};

    fn body(handle: usize, velocity: [f64; 2], inertia: [f64; 2]) -> RigidBody<2> {
        let mut body = RigidBody::new(
            BodyHandle(handle),
            BodyType::Dynamic,
            Transform::from_translation(Point::origin()),
            Box::new(Sphere::<2>::unit()),
            2.0,
            SVector::from(inertia),
        );
        body.linear_velocity = SVector::from(velocity);
        body
    }

    #[test]
    fn anisotropic_pair_snapshot_uses_checked_solver_energy() {
        let mut a = body(1, [1.0, 0.0], [2.0, 8.0]);
        let b = body(2, [0.0, 0.0], [3.0, 3.0]);
        a.angular_velocity.set(0, 1, 2.0);

        let snapshot = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        assert_eq!(snapshot.body_a, BodyHandle(1));
        assert_eq!(snapshot.body_b, BodyHandle(2));
        assert!((snapshot.kinetic_a_joules - a.kinetic_energy()).abs() > 1.0);
    }

    #[test]
    fn equal_and_opposite_friction_like_impulse_classifies_signed_loss() {
        let mut a = body(1, [1.0, 0.0], [2.0, 2.0]);
        let mut b = body(2, [0.0, 0.0], [2.0, 2.0]);
        let before = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();

        let impulse_on_b = SVector::from([0.5, 0.0]);
        integrator::apply_impulse(&mut a, &(-impulse_on_b));
        integrator::apply_impulse(&mut b, &impulse_on_b);

        let after = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        let change = classify_friction_pair_energy_change_2d_checked(before, after).unwrap();
        assert_eq!(
            change.delta,
            FrictionPairEnergyDelta2d::DissipationCandidate { joules: 0.375 }
        );
        assert_eq!(change.pair_delta_joules, -0.375);
    }

    #[test]
    fn solver_energy_gain_is_not_clamped_into_fake_dissipation() {
        let mut a = body(1, [0.0, 0.0], [2.0, 2.0]);
        let mut b = body(2, [0.0, 0.0], [2.0, 2.0]);
        let before = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();

        let impulse_on_b = SVector::from([0.5, 0.0]);
        integrator::apply_impulse(&mut a, &(-impulse_on_b));
        integrator::apply_impulse(&mut b, &impulse_on_b);

        let after = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        let change = classify_friction_pair_energy_change_2d_checked(before, after).unwrap();
        assert_eq!(
            change.delta,
            FrictionPairEnergyDelta2d::SolverInjection { joules: 0.125 }
        );
        assert_eq!(change.pair_delta_joules, 0.125);
    }

    #[test]
    fn ordered_identity_mismatch_is_rejected() {
        let a = body(1, [1.0, 0.0], [2.0, 2.0]);
        let b = body(2, [0.0, 0.0], [2.0, 2.0]);
        let before = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        let after_swapped = capture_friction_pair_energy_2d_checked(&b, &a).unwrap();
        assert_eq!(
            classify_friction_pair_energy_change_2d_checked(before, after_swapped),
            Err(FrictionPairEnergy2dError::SnapshotIdentityMismatch)
        );
    }

    #[test]
    fn invalid_body_state_is_attributed_to_exact_participant() {
        let a = body(1, [1.0, 0.0], [2.0, 2.0]);
        let mut b = body(2, [0.0, 0.0], [2.0, 2.0]);
        b.inv_mass *= 0.5;
        assert_eq!(
            capture_friction_pair_energy_2d_checked(&a, &b),
            Err(FrictionPairEnergy2dError::BodyB(
                RigidBodyEnergy2dError::InconsistentInverseMass
            ))
        );
    }
}
