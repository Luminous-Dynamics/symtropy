// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Numerically stable 2D kinetic-energy change across one mechanical transition.
//!
//! Absolute `K_after - K_before` loses small transitions when a body carries a
//! much larger pre-existing mechanical-energy baseline. This module instead
//! evaluates the same current solver-consistent quadratic in factored,
//! transition-local form.
//!
//! Each changed scalar coordinate uses
//!
//! `0.5 * coefficient * (q_after - q_before) * (q_after + q_before)`.
//!
//! Unchanged coordinates contribute exactly zero without forming the potentially
//! overflowing `q_after + q_before` sum. This is measurement-only: the result
//! does not authorize heat, mutate a ledger, or apply mechanics.

use nalgebra::SVector;

use crate::body::{BodyHandle, RigidBody};
use crate::body_energy_2d::RigidBodyEnergy2dError;

const ENERGY_EPSILON: f64 = 1.0e-15;
const RECIPROCAL_REL_TOLERANCE: f64 = 1.0e-10;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrictionBodyTransitionBasis2d {
    pub body: BodyHandle,
    pub mass: f64,
    pub effective_inertia: f64,
    pub linear_velocity: SVector<f64, 2>,
    pub angular_velocity: f64,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrictionPairTransitionBasis2d {
    pub body_a: FrictionBodyTransitionBasis2d,
    pub body_b: FrictionBodyTransitionBasis2d,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FrictionTransitionDelta2d {
    DissipationCandidate { solver_energy: f64 },
    SolverInjection { solver_energy: f64 },
    Neutral,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrictionPairTransitionEnergy2d {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub kinetic_change_a_solver: f64,
    pub kinetic_change_b_solver: f64,
    pub pair_delta_solver: f64,
    pub delta: FrictionTransitionDelta2d,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrictionTransitionEnergy2dError {
    SameBody,
    BodyA(RigidBodyEnergy2dError),
    BodyB(RigidBodyEnergy2dError),
    IdentityMismatch,
    MechanicalParametersChanged,
    UnrepresentableTransition,
}

#[inline]
fn reciprocal_is_consistent(value: f64, inverse: f64) -> bool {
    if !value.is_finite() || value <= 0.0 || !inverse.is_finite() || inverse <= 0.0 {
        return false;
    }
    let product = value * inverse;
    product.is_finite()
        && (product - 1.0).abs()
            <= RECIPROCAL_REL_TOLERANCE * product.abs().max(1.0)
}

fn capture_body(
    body: &RigidBody<2>,
) -> Result<FrictionBodyTransitionBasis2d, RigidBodyEnergy2dError> {
    if !body.linear_velocity.iter().all(|value| value.is_finite()) {
        return Err(RigidBodyEnergy2dError::NonFiniteLinearVelocity);
    }
    if !body.angular_velocity.is_finite() {
        return Err(RigidBodyEnergy2dError::NonFiniteAngularVelocity);
    }

    if !body.is_dynamic() {
        return Ok(FrictionBodyTransitionBasis2d {
            body: body.handle,
            mass: 0.0,
            effective_inertia: 0.0,
            linear_velocity: body.linear_velocity,
            angular_velocity: body.angular_velocity.get(0, 1),
        });
    }

    if !body.mass.is_finite()
        || body.mass <= 0.0
        || !body.inv_mass.is_finite()
        || body.inv_mass <= 0.0
    {
        return Err(RigidBodyEnergy2dError::InvalidMass);
    }
    if !reciprocal_is_consistent(body.mass, body.inv_mass) {
        return Err(RigidBodyEnergy2dError::InconsistentInverseMass);
    }

    // Reuse only the checked inverse-inertia theorem. Unlike
    // `kinetic_energy_2d_solver_checked`, this does not square the potentially
    // enormous angular velocity or require the absolute energy baseline to fit.
    let inverse_inertia = body.solver_inverse_inertia_2d_checked()?;
    let effective_inertia = 1.0 / inverse_inertia;
    if !effective_inertia.is_finite() || effective_inertia <= 0.0 {
        return Err(RigidBodyEnergy2dError::UnrepresentableAngularEnergy);
    }

    Ok(FrictionBodyTransitionBasis2d {
        body: body.handle,
        mass: body.mass,
        effective_inertia,
        linear_velocity: body.linear_velocity,
        angular_velocity: body.angular_velocity.get(0, 1),
    })
}

pub fn capture_friction_pair_transition_basis_2d_checked(
    body_a: &RigidBody<2>,
    body_b: &RigidBody<2>,
) -> Result<FrictionPairTransitionBasis2d, FrictionTransitionEnergy2dError> {
    if body_a.handle == body_b.handle {
        return Err(FrictionTransitionEnergy2dError::SameBody);
    }
    Ok(FrictionPairTransitionBasis2d {
        body_a: capture_body(body_a).map_err(FrictionTransitionEnergy2dError::BodyA)?,
        body_b: capture_body(body_b).map_err(FrictionTransitionEnergy2dError::BodyB)?,
    })
}

#[inline]
fn stable_scalar_square_delta(
    coefficient: f64,
    before: f64,
    after: f64,
) -> Result<f64, FrictionTransitionEnergy2dError> {
    if before == after {
        return Ok(0.0);
    }
    let delta = after - before;
    let sum = after + before;
    let value = 0.5 * coefficient * delta * sum;
    if !delta.is_finite() || !sum.is_finite() || !value.is_finite() {
        return Err(FrictionTransitionEnergy2dError::UnrepresentableTransition);
    }
    Ok(value)
}

#[inline]
fn stable_body_delta(
    before: FrictionBodyTransitionBasis2d,
    after: FrictionBodyTransitionBasis2d,
) -> Result<f64, FrictionTransitionEnergy2dError> {
    if before.body != after.body {
        return Err(FrictionTransitionEnergy2dError::IdentityMismatch);
    }
    if before.mass != after.mass || before.effective_inertia != after.effective_inertia {
        return Err(FrictionTransitionEnergy2dError::MechanicalParametersChanged);
    }

    let mut linear = 0.0;
    for axis in 0..2 {
        let term = stable_scalar_square_delta(
            before.mass,
            before.linear_velocity[axis],
            after.linear_velocity[axis],
        )?;
        linear += term;
        if !linear.is_finite() {
            return Err(FrictionTransitionEnergy2dError::UnrepresentableTransition);
        }
    }

    let angular = stable_scalar_square_delta(
        before.effective_inertia,
        before.angular_velocity,
        after.angular_velocity,
    )?;
    let total = linear + angular;
    if !total.is_finite() {
        return Err(FrictionTransitionEnergy2dError::UnrepresentableTransition);
    }
    Ok(total)
}

pub fn classify_friction_pair_transition_2d_checked(
    before: FrictionPairTransitionBasis2d,
    after: FrictionPairTransitionBasis2d,
) -> Result<FrictionPairTransitionEnergy2d, FrictionTransitionEnergy2dError> {
    if before.body_a.body != after.body_a.body || before.body_b.body != after.body_b.body {
        return Err(FrictionTransitionEnergy2dError::IdentityMismatch);
    }

    let change_a = stable_body_delta(before.body_a, after.body_a)?;
    let change_b = stable_body_delta(before.body_b, after.body_b)?;
    let pair_delta = change_a + change_b;
    if !pair_delta.is_finite() {
        return Err(FrictionTransitionEnergy2dError::UnrepresentableTransition);
    }

    let delta = if pair_delta < -ENERGY_EPSILON {
        FrictionTransitionDelta2d::DissipationCandidate {
            solver_energy: -pair_delta,
        }
    } else if pair_delta > ENERGY_EPSILON {
        FrictionTransitionDelta2d::SolverInjection {
            solver_energy: pair_delta,
        }
    } else {
        FrictionTransitionDelta2d::Neutral
    };

    Ok(FrictionPairTransitionEnergy2d {
        body_a: before.body_a.body,
        body_b: before.body_b.body,
        kinetic_change_a_solver: change_a,
        kinetic_change_b_solver: change_b,
        pair_delta_solver: pair_delta,
        delta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BodyType, RigidBody};
    use crate::friction_energy_2d::{
        capture_friction_pair_energy_2d_checked, classify_friction_pair_energy_change_2d_checked,
    };
    use crate::integrator;
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

    fn apply_centered_pair_impulse(a: &mut RigidBody<2>, b: &mut RigidBody<2>) {
        let impulse = SVector::from([0.5, 0.0]);
        integrator::apply_impulse(a, &(-impulse));
        integrator::apply_impulse(b, &impulse);
    }

    #[test]
    fn normal_scale_transition_matches_absolute_reference() {
        let mut a = anisotropic_body(1, 1.0, 2.0);
        let mut b = anisotropic_body(2, 0.0, -1.0);
        let absolute_before = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        let stable_before = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();

        apply_centered_pair_impulse(&mut a, &mut b);

        let absolute_after = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        let stable_after = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();
        let absolute =
            classify_friction_pair_energy_change_2d_checked(absolute_before, absolute_after).unwrap();
        let stable = classify_friction_pair_transition_2d_checked(stable_before, stable_after).unwrap();

        assert_eq!(stable.pair_delta_solver, -0.25);
        assert_eq!(absolute.pair_delta_joules, stable.pair_delta_solver);
        assert_eq!(stable.kinetic_change_a_solver, -0.375);
        assert_eq!(stable.kinetic_change_b_solver, 0.125);
    }

    #[test]
    fn huge_rotational_baseline_cannot_double_small_centered_loss() {
        let mut a = anisotropic_body(1, 1.0, 30_000_000.0);
        let mut b = anisotropic_body(2, 0.0, 0.0);
        let absolute_before = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        let stable_before = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();

        apply_centered_pair_impulse(&mut a, &mut b);

        let absolute_after = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        let stable_after = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();
        let absolute =
            classify_friction_pair_energy_change_2d_checked(absolute_before, absolute_after).unwrap();
        let stable = classify_friction_pair_transition_2d_checked(stable_before, stable_after).unwrap();

        assert_eq!(stable.pair_delta_solver, -0.25);
        assert_eq!(absolute.pair_delta_joules, -0.5);
        assert_eq!(
            stable.delta,
            FrictionTransitionDelta2d::DissipationCandidate { solver_energy: 0.25 }
        );
    }

    #[test]
    fn huge_rotational_baseline_cannot_erase_small_centered_loss() {
        let mut a = anisotropic_body(1, 1.0, 100_000_000.0);
        let mut b = anisotropic_body(2, 0.0, 0.0);
        let absolute_before = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        let stable_before = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();

        apply_centered_pair_impulse(&mut a, &mut b);

        let absolute_after = capture_friction_pair_energy_2d_checked(&a, &b).unwrap();
        let stable_after = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();
        let absolute =
            classify_friction_pair_energy_change_2d_checked(absolute_before, absolute_after).unwrap();
        let stable = classify_friction_pair_transition_2d_checked(stable_before, stable_after).unwrap();

        assert_eq!(stable.pair_delta_solver, -0.25);
        assert_eq!(absolute.pair_delta_joules, 0.0);
        assert_eq!(
            stable.delta,
            FrictionTransitionDelta2d::DissipationCandidate { solver_energy: 0.25 }
        );
    }

    #[test]
    fn stable_transition_does_not_require_representable_absolute_energy() {
        let huge_unchanged_omega = f64::MAX / 2.0;
        let mut a = anisotropic_body(1, 1.0, huge_unchanged_omega);
        let mut b = anisotropic_body(2, 0.0, 0.0);

        assert_eq!(
            a.kinetic_energy_2d_solver_checked(),
            Err(RigidBodyEnergy2dError::UnrepresentableAngularEnergy)
        );
        let stable_before = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();

        apply_centered_pair_impulse(&mut a, &mut b);

        let stable_after = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();
        let stable = classify_friction_pair_transition_2d_checked(stable_before, stable_after).unwrap();
        assert_eq!(stable.pair_delta_solver, -0.25);
        assert_eq!(stable.kinetic_change_a_solver, -0.375);
        assert_eq!(stable.kinetic_change_b_solver, 0.125);
    }

    #[test]
    fn changed_mechanical_parameters_fail_closed() {
        let a = anisotropic_body(1, 1.0, 2.0);
        let b = anisotropic_body(2, 0.0, 0.0);
        let before = capture_friction_pair_transition_basis_2d_checked(&a, &b).unwrap();
        let mut after_a = a;
        let after_b = b;
        after_a.mass = 2.0;
        after_a.inv_mass = 0.5;
        let after = capture_friction_pair_transition_basis_2d_checked(&after_a, &after_b).unwrap();

        assert_eq!(
            classify_friction_pair_transition_2d_checked(before, after),
            Err(FrictionTransitionEnergy2dError::MechanicalParametersChanged)
        );
    }
}
