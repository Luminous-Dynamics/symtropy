// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Reference 3D angular dynamics for asymmetric rigid bodies.
//!
//! The production N-D integrator currently collapses principal inertia to a
//! scalar mean. This module deliberately does **not** change that hot path yet.
//! Instead it provides a small, auditable 3D reference implementation with
//! diagonal body-space principal inertia and explicit world angular momentum.
//!
//! Keeping the reference separate lets validation establish the convention,
//! invariants, and expected asymmetric-top behaviour before the contact solver,
//! impulse application, and `RigidBody` hot path are migrated.
//!
//! # Bivector sign convention
//!
//! Symtropy's [`symtropy_math::Rotor::from_bivector`] constructs `exp(-B)`.
//! Therefore the physical/kinematic angular-velocity vector `omega` associated
//! with a 3D bivector is the axial vector of `-B`, not of `B`:
//!
//! `omega = [b12, -b02, b01]`.
//!
//! This convention is chosen so that a positive `e01` bivector rotates +x
//! toward +y, matching `Rotor`. It also makes the instantaneous point velocity
//! consistent with the orientation derivative: `v = -B r = omega x r`.
//! Existing code that treats `B.apply_to_vector(r)` as point velocity has the
//! opposite sign and should be migrated only after dedicated contact tests.

use nalgebra::SVector;
use symtropy_math::{Bivector, Rotor};

/// Principal moments of inertia about a body's local x/y/z axes.
///
/// This first reference requires finite, strictly positive moments. It does not
/// enforce the triangle inequalities of a realizable mass distribution because
/// callers may intentionally use abstract positive-definite inertia models for
/// validation. Shape-derived constructors can enforce stronger physical rules
/// later.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrincipalInertia3 {
    moments: [f64; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AngularDynamicsError {
    InvalidPrincipalMoment { axis: usize },
    NonFiniteAngularVelocity,
    NonFiniteAngularMomentum,
    NonFiniteTorque,
    InvalidTimestep,
    NonFiniteRotation,
}

impl PrincipalInertia3 {
    pub fn new(moments: [f64; 3]) -> Result<Self, AngularDynamicsError> {
        for (axis, moment) in moments.iter().copied().enumerate() {
            if !moment.is_finite() || moment <= 0.0 {
                return Err(AngularDynamicsError::InvalidPrincipalMoment { axis });
            }
        }
        Ok(Self { moments })
    }

    #[inline]
    pub const fn moments(self) -> [f64; 3] {
        self.moments
    }

    #[inline]
    fn apply(self, vector: &SVector<f64, 3>) -> SVector<f64, 3> {
        SVector::from([
            self.moments[0] * vector[0],
            self.moments[1] * vector[1],
            self.moments[2] * vector[2],
        ])
    }

    #[inline]
    fn apply_inverse(self, vector: &SVector<f64, 3>) -> SVector<f64, 3> {
        SVector::from([
            vector[0] / self.moments[0],
            vector[1] / self.moments[1],
            vector[2] / self.moments[2],
        ])
    }
}

/// Convert Symtropy's 3D angular-rate bivector into the kinematic world vector
/// whose cross-product matrix is the generator used by `Rotor`.
#[inline]
pub fn bivector_to_angular_vector(bivector: &Bivector<3>) -> SVector<f64, 3> {
    SVector::from([
        bivector.get(1, 2),
        -bivector.get(0, 2),
        bivector.get(0, 1),
    ])
}

/// Inverse of [`bivector_to_angular_vector`].
#[inline]
pub fn angular_vector_to_bivector(vector: &SVector<f64, 3>) -> Bivector<3> {
    let mut bivector = Bivector::<3>::zero();
    bivector.set(0, 1, vector[2]);
    bivector.set(0, 2, -vector[1]);
    bivector.set(1, 2, vector[0]);
    bivector
}

/// Instantaneous velocity induced at a world-space offset from the center of
/// mass, consistent with `Rotor::from_bivector`'s orientation convention.
#[inline]
pub fn angular_velocity_at_offset(
    angular_velocity: &Bivector<3>,
    offset_world: &SVector<f64, 3>,
) -> SVector<f64, 3> {
    -angular_velocity.apply_to_vector(offset_world)
}

/// Compute world angular momentum from orientation, angular velocity, and
/// body-space principal inertia.
pub fn world_angular_momentum(
    rotation: &Rotor<3>,
    angular_velocity: &Bivector<3>,
    inertia: PrincipalInertia3,
) -> Result<SVector<f64, 3>, AngularDynamicsError> {
    validate_rotation(rotation)?;
    if !angular_velocity.is_finite() {
        return Err(AngularDynamicsError::NonFiniteAngularVelocity);
    }

    let omega_world = bivector_to_angular_vector(angular_velocity);
    let omega_body = rotation.reverse().rotate_vector(&omega_world);
    let momentum_body = inertia.apply(&omega_body);
    Ok(rotation.rotate_vector(&momentum_body))
}

/// Recover world angular velocity from conserved world angular momentum.
pub fn angular_velocity_from_world_momentum(
    rotation: &Rotor<3>,
    momentum_world: &SVector<f64, 3>,
    inertia: PrincipalInertia3,
) -> Result<Bivector<3>, AngularDynamicsError> {
    validate_rotation(rotation)?;
    if !momentum_world.iter().all(|value| value.is_finite()) {
        return Err(AngularDynamicsError::NonFiniteAngularMomentum);
    }

    let momentum_body = rotation.reverse().rotate_vector(momentum_world);
    let omega_body = inertia.apply_inverse(&momentum_body);
    let omega_world = rotation.rotate_vector(&omega_body);
    Ok(angular_vector_to_bivector(&omega_world))
}

/// Rotational kinetic energy `0.5 * omega_body^T I_body omega_body`.
pub fn rotational_kinetic_energy(
    rotation: &Rotor<3>,
    angular_velocity: &Bivector<3>,
    inertia: PrincipalInertia3,
) -> Result<f64, AngularDynamicsError> {
    validate_rotation(rotation)?;
    if !angular_velocity.is_finite() {
        return Err(AngularDynamicsError::NonFiniteAngularVelocity);
    }

    let omega_world = bivector_to_angular_vector(angular_velocity);
    let omega_body = rotation.reverse().rotate_vector(&omega_world);
    Ok(0.5 * omega_body.dot(&inertia.apply(&omega_body)))
}

/// Output of one reference angular-momentum step.
#[derive(Debug, Clone)]
pub struct AngularStep3 {
    pub rotation: Rotor<3>,
    pub angular_velocity: Bivector<3>,
    pub world_angular_momentum: SVector<f64, 3>,
}

/// Advance a 3D body using world angular momentum as the conserved state.
///
/// Torque is supplied as a conventional world-space axial vector in N*m. The
/// update is intentionally simple and deterministic:
///
/// 1. derive current world angular momentum from `(R, omega, I_body)`,
/// 2. apply the exact angular impulse `tau * dt`,
/// 3. derive `omega` from the updated momentum and current orientation,
/// 4. update orientation through Symtropy's SO(3) exponential,
/// 5. re-derive `omega` from the same world momentum at the new orientation.
///
/// With zero torque this keeps world angular momentum constant by construction.
/// Rotational energy is not projected; its integration error should converge
/// under timestep refinement and is part of the validation protocol.
pub fn step_principal_inertia(
    rotation: &Rotor<3>,
    angular_velocity: &Bivector<3>,
    inertia: PrincipalInertia3,
    torque_world: &SVector<f64, 3>,
    dt_seconds: f64,
) -> Result<AngularStep3, AngularDynamicsError> {
    validate_rotation(rotation)?;
    if !angular_velocity.is_finite() {
        return Err(AngularDynamicsError::NonFiniteAngularVelocity);
    }
    if !torque_world.iter().all(|value| value.is_finite()) {
        return Err(AngularDynamicsError::NonFiniteTorque);
    }
    if !dt_seconds.is_finite() || dt_seconds < 0.0 {
        return Err(AngularDynamicsError::InvalidTimestep);
    }

    let mut momentum_world = world_angular_momentum(rotation, angular_velocity, inertia)?;
    momentum_world += torque_world * dt_seconds;

    let omega_before_rotation =
        angular_velocity_from_world_momentum(rotation, &momentum_world, inertia)?;
    let delta = Rotor::from_bivector(&omega_before_rotation.scale(dt_seconds));
    let rotation_next = delta.compose(rotation);
    let angular_velocity_next =
        angular_velocity_from_world_momentum(&rotation_next, &momentum_world, inertia)?;

    Ok(AngularStep3 {
        rotation: rotation_next,
        angular_velocity: angular_velocity_next,
        world_angular_momentum: momentum_world,
    })
}

fn validate_rotation(rotation: &Rotor<3>) -> Result<(), AngularDynamicsError> {
    if rotation.to_matrix().iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(AngularDynamicsError::NonFiniteRotation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vec_close(a: &SVector<f64, 3>, b: &SVector<f64, 3>, tolerance: f64) -> bool {
        (a - b).norm() <= tolerance
    }

    #[test]
    fn bivector_vector_mapping_roundtrips_and_matches_rotor_direction() {
        let omega = SVector::from([0.7, -1.1, 2.3]);
        let bivector = angular_vector_to_bivector(&omega);
        assert!(vec_close(
            &bivector_to_angular_vector(&bivector),
            &omega,
            1.0e-14
        ));

        let x = SVector::from([1.0, 0.0, 0.0]);
        let positive_z = angular_vector_to_bivector(&SVector::from([0.0, 0.0, 1.0]));
        let point_velocity = angular_velocity_at_offset(&positive_z, &x);
        assert!(vec_close(
            &point_velocity,
            &SVector::from([0.0, 1.0, 0.0]),
            1.0e-14
        ));

        let rotated = Rotor::from_bivector(&positive_z.scale(1.0e-5)).rotate_vector(&x);
        assert!(rotated[1] > 0.0, "positive z must rotate +x toward +y");
    }

    #[test]
    fn invalid_principal_inertia_is_rejected() {
        assert_eq!(
            PrincipalInertia3::new([1.0, 0.0, 2.0]),
            Err(AngularDynamicsError::InvalidPrincipalMoment { axis: 1 })
        );
        assert_eq!(
            PrincipalInertia3::new([1.0, f64::NAN, 2.0]),
            Err(AngularDynamicsError::InvalidPrincipalMoment { axis: 1 })
        );
    }

    #[test]
    fn isotropic_torque_free_body_preserves_world_omega_and_momentum() {
        let inertia = PrincipalInertia3::new([2.0, 2.0, 2.0]).unwrap();
        let mut rotation = Rotor::<3>::identity();
        let mut omega = angular_vector_to_bivector(&SVector::from([0.4, -0.7, 1.2]));
        let initial_vector = bivector_to_angular_vector(&omega);
        let initial_l = world_angular_momentum(&rotation, &omega, inertia).unwrap();

        for _ in 0..2_000 {
            let step = step_principal_inertia(
                &rotation,
                &omega,
                inertia,
                &SVector::zeros(),
                0.001,
            )
            .unwrap();
            rotation = step.rotation;
            omega = step.angular_velocity;
        }

        let final_vector = bivector_to_angular_vector(&omega);
        let final_l = world_angular_momentum(&rotation, &omega, inertia).unwrap();
        assert!(vec_close(&initial_vector, &final_vector, 1.0e-9));
        assert!(vec_close(&initial_l, &final_l, 1.0e-9));
        assert!(rotation.is_proper_rotation(1.0e-9));
    }

    #[test]
    fn asymmetric_top_tumbles_while_world_momentum_stays_fixed() {
        let inertia = PrincipalInertia3::new([1.0, 2.0, 3.0]).unwrap();
        let mut rotation = Rotor::<3>::identity();
        let mut omega = angular_vector_to_bivector(&SVector::from([1.0, 0.7, 0.2]));
        let initial_omega = bivector_to_angular_vector(&omega);
        let initial_l = world_angular_momentum(&rotation, &omega, inertia).unwrap();

        for _ in 0..1_000 {
            let step = step_principal_inertia(
                &rotation,
                &omega,
                inertia,
                &SVector::zeros(),
                0.001,
            )
            .unwrap();
            rotation = step.rotation;
            omega = step.angular_velocity;
        }

        let final_omega = bivector_to_angular_vector(&omega);
        let final_l = world_angular_momentum(&rotation, &omega, inertia).unwrap();
        assert!(
            (final_omega - initial_omega).norm() > 1.0e-3,
            "an off-principal asymmetric top should not keep constant world omega"
        );
        assert!(vec_close(&initial_l, &final_l, 1.0e-9));
        assert!(rotation.is_proper_rotation(1.0e-9));
    }

    fn energy_drift(dt: f64, duration: f64) -> f64 {
        let inertia = PrincipalInertia3::new([1.0, 1.7, 2.8]).unwrap();
        let mut rotation = Rotor::<3>::identity();
        let mut omega = angular_vector_to_bivector(&SVector::from([0.9, -0.6, 0.35]));
        let initial = rotational_kinetic_energy(&rotation, &omega, inertia).unwrap();
        let steps = (duration / dt).round() as usize;
        for _ in 0..steps {
            let step = step_principal_inertia(
                &rotation,
                &omega,
                inertia,
                &SVector::zeros(),
                dt,
            )
            .unwrap();
            rotation = step.rotation;
            omega = step.angular_velocity;
        }
        let final_energy = rotational_kinetic_energy(&rotation, &omega, inertia).unwrap();
        (final_energy - initial).abs() / initial.abs().max(1.0e-15)
    }

    #[test]
    fn asymmetric_top_energy_error_converges_with_timestep_refinement() {
        let coarse = energy_drift(0.01, 1.0);
        let fine = energy_drift(0.0025, 1.0);
        assert!(
            fine < coarse,
            "finer timestep should reduce energy drift: coarse={coarse:e}, fine={fine:e}"
        );
        assert!(fine < 0.02, "fine-step energy drift too large: {fine:e}");
    }

    #[test]
    fn body_orientation_changes_world_response_for_anisotropic_inertia() {
        let inertia = PrincipalInertia3::new([1.0, 4.0, 9.0]).unwrap();
        let momentum = SVector::from([1.0, 0.0, 0.0]);
        let identity = Rotor::<3>::identity();
        let quarter_turn = Rotor::from_bivector(
            &angular_vector_to_bivector(&SVector::from([
                0.0,
                0.0,
                std::f64::consts::FRAC_PI_2,
            ])),
        );

        let omega_identity = bivector_to_angular_vector(
            &angular_velocity_from_world_momentum(&identity, &momentum, inertia).unwrap(),
        );
        let omega_rotated = bivector_to_angular_vector(
            &angular_velocity_from_world_momentum(&quarter_turn, &momentum, inertia).unwrap(),
        );

        assert!(
            (omega_identity - omega_rotated).norm() > 0.5,
            "rotating an anisotropic body must change its world inverse-inertia response"
        );
    }

    #[test]
    fn torque_changes_world_momentum_by_exact_angular_impulse() {
        let inertia = PrincipalInertia3::new([1.2, 2.3, 3.4]).unwrap();
        let rotation = Rotor::<3>::identity();
        let omega = angular_vector_to_bivector(&SVector::from([0.2, 0.4, -0.1]));
        let initial_l = world_angular_momentum(&rotation, &omega, inertia).unwrap();
        let torque = SVector::from([3.0, -2.0, 5.0]);
        let dt = 0.025;

        let step = step_principal_inertia(&rotation, &omega, inertia, &torque, dt).unwrap();
        let expected = initial_l + torque * dt;
        assert!(vec_close(
            &step.world_angular_momentum,
            &expected,
            1.0e-13
        ));
        let reconstructed =
            world_angular_momentum(&step.rotation, &step.angular_velocity, inertia).unwrap();
        assert!(vec_close(&reconstructed, &expected, 1.0e-10));
    }

    #[test]
    fn negative_or_non_finite_timestep_is_rejected() {
        let inertia = PrincipalInertia3::new([1.0, 2.0, 3.0]).unwrap();
        let rotation = Rotor::<3>::identity();
        let omega = Bivector::<3>::zero();
        assert!(matches!(
            step_principal_inertia(&rotation, &omega, inertia, &SVector::zeros(), -0.1),
            Err(AngularDynamicsError::InvalidTimestep)
        ));
        assert!(matches!(
            step_principal_inertia(
                &rotation,
                &omega,
                inertia,
                &SVector::zeros(),
                f64::NAN
            ),
            Err(AngularDynamicsError::InvalidTimestep)
        ));
    }
}
