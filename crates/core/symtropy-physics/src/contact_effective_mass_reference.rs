// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked contact-point impulse response and directional effective-mass
//! reference built on the N-D rotational inertia operator.
//!
//! This module pins the physical sign convention end-to-end without changing
//! the production contact solver:
//!
//! `Delta L = r ∧ J`
//!
//! `Delta omega = I_world^-1 Delta L`
//!
//! `Delta v_point = Delta v_com - Delta B r`.
//!
//! The final minus sign is required by Symtropy's `Rotor::from_bivector`
//! convention (`R = exp(-B)`).

use nalgebra::SVector;
use symtropy_math::{Bivector, Rotor};

use crate::rotational_inertia_frame::{RotationalFrameError, RotationalInertiaFrameExt};
use crate::rotational_inertia_operator::RotationalInertiaOperator;

const UNIT_DIRECTION_TOLERANCE: f64 = 1.0e-10;

#[derive(Copy, Clone, Debug)]
pub enum ContactBodyResponseRef<'a, const D: usize> {
    /// Infinite-mass/inertia contact partner. Contribution to inverse effective
    /// mass is exactly zero.
    Fixed,
    /// Dynamic body contribution at a world-space contact offset.
    Dynamic {
        inverse_mass: f64,
        offset_world: &'a SVector<f64, D>,
        body_inertia: &'a RotationalInertiaOperator<D>,
        rotation: &'a Rotor<D>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactPointImpulseResponse<const D: usize> {
    pub linear_velocity_delta: SVector<f64, D>,
    pub angular_momentum_delta: Bivector<D>,
    pub angular_velocity_delta: Bivector<D>,
    pub angular_point_velocity_delta: SVector<f64, D>,
    pub point_velocity_delta: SVector<f64, D>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ContactEffectiveMassError {
    InvalidInverseMass,
    NonFiniteOffset,
    NonFiniteImpulse,
    NonFiniteDirection,
    NonUnitDirection,
    UnrepresentableAngularImpulse,
    UnrepresentableVelocityResponse,
    NegativeDirectionalResponse,
    UnrepresentablePairResponse,
    Frame(RotationalFrameError),
}

impl From<RotationalFrameError> for ContactEffectiveMassError {
    fn from(value: RotationalFrameError) -> Self {
        Self::Frame(value)
    }
}

/// Physical angular impulse at a world-space offset.
///
/// For `r=(1,0,0)` and `J=(0,1,0)` this yields positive `e01`, which maps to
/// positive physical z angular momentum under Symtropy's Rotor convention.
pub fn angular_impulse_from_point_impulse_checked<const D: usize>(
    offset_world: &SVector<f64, D>,
    impulse_world: &SVector<f64, D>,
) -> Result<Bivector<D>, ContactEffectiveMassError> {
    if !offset_world.iter().all(|value| value.is_finite()) {
        return Err(ContactEffectiveMassError::NonFiniteOffset);
    }
    if !impulse_world.iter().all(|value| value.is_finite()) {
        return Err(ContactEffectiveMassError::NonFiniteImpulse);
    }

    let angular_impulse = Bivector::from_wedge(offset_world, impulse_world);
    if !angular_impulse.is_finite() {
        return Err(ContactEffectiveMassError::UnrepresentableAngularImpulse);
    }
    Ok(angular_impulse)
}

/// Checked point-velocity response of one dynamic body to an arbitrary
/// world-space impulse applied at a world-space offset from its COM.
pub fn point_impulse_response_checked<const D: usize>(
    inverse_mass: f64,
    offset_world: &SVector<f64, D>,
    impulse_world: &SVector<f64, D>,
    body_inertia: &RotationalInertiaOperator<D>,
    rotation: &Rotor<D>,
) -> Result<ContactPointImpulseResponse<D>, ContactEffectiveMassError> {
    if !inverse_mass.is_finite() || inverse_mass < 0.0 {
        return Err(ContactEffectiveMassError::InvalidInverseMass);
    }
    if !offset_world.iter().all(|value| value.is_finite()) {
        return Err(ContactEffectiveMassError::NonFiniteOffset);
    }
    if !impulse_world.iter().all(|value| value.is_finite()) {
        return Err(ContactEffectiveMassError::NonFiniteImpulse);
    }

    let linear_velocity_delta = impulse_world * inverse_mass;
    if !linear_velocity_delta.iter().all(|value| value.is_finite()) {
        return Err(ContactEffectiveMassError::UnrepresentableVelocityResponse);
    }

    let angular_momentum_delta =
        angular_impulse_from_point_impulse_checked(offset_world, impulse_world)?;
    let angular_velocity_delta = body_inertia
        .world_angular_velocity_from_momentum_checked(rotation, &angular_momentum_delta)?;

    // Physical point velocity is -B*r for Symtropy's exp(-B) Rotor convention.
    let angular_point_velocity_delta = -angular_velocity_delta.apply_to_vector(offset_world);
    if !angular_point_velocity_delta
        .iter()
        .all(|value| value.is_finite())
    {
        return Err(ContactEffectiveMassError::UnrepresentableVelocityResponse);
    }

    let point_velocity_delta = linear_velocity_delta + angular_point_velocity_delta;
    if !point_velocity_delta.iter().all(|value| value.is_finite()) {
        return Err(ContactEffectiveMassError::UnrepresentableVelocityResponse);
    }

    Ok(ContactPointImpulseResponse {
        linear_velocity_delta,
        angular_momentum_delta,
        angular_velocity_delta,
        angular_point_velocity_delta,
        point_velocity_delta,
    })
}

/// Per-body contribution to inverse effective mass along a **unit** world-space
/// contact direction.
///
/// A unit impulse `J=n` is applied at the contact offset and the resulting
/// contact-point velocity change is projected back onto `n`.
pub fn directional_inverse_effective_mass_checked<const D: usize>(
    body: &ContactBodyResponseRef<'_, D>,
    direction_world: &SVector<f64, D>,
) -> Result<f64, ContactEffectiveMassError> {
    validate_unit_direction(direction_world)?;

    let response = match body {
        ContactBodyResponseRef::Fixed => return Ok(0.0),
        ContactBodyResponseRef::Dynamic {
            inverse_mass,
            offset_world,
            body_inertia,
            rotation,
        } => point_impulse_response_checked(
            *inverse_mass,
            offset_world,
            direction_world,
            body_inertia,
            rotation,
        )?,
    };

    let inverse_effective_mass = direction_world.dot(&response.point_velocity_delta);
    if !inverse_effective_mass.is_finite() {
        return Err(ContactEffectiveMassError::UnrepresentableVelocityResponse);
    }
    if inverse_effective_mass < 0.0 {
        return Err(ContactEffectiveMassError::NegativeDirectionalResponse);
    }
    Ok(inverse_effective_mass)
}

/// Pair inverse effective mass for a contact impulse `+J` on B and `-J` on A.
///
/// By linearity, the relative velocity change along `n` is the sum of each
/// body's positive unit-impulse response magnitude.
pub fn pair_directional_inverse_effective_mass_checked<const D: usize>(
    body_a: &ContactBodyResponseRef<'_, D>,
    body_b: &ContactBodyResponseRef<'_, D>,
    direction_world: &SVector<f64, D>,
) -> Result<f64, ContactEffectiveMassError> {
    let contribution_a = directional_inverse_effective_mass_checked(body_a, direction_world)?;
    let contribution_b = directional_inverse_effective_mass_checked(body_b, direction_world)?;
    let total = contribution_a + contribution_b;
    if !total.is_finite() {
        return Err(ContactEffectiveMassError::UnrepresentablePairResponse);
    }
    Ok(total)
}

fn validate_unit_direction<const D: usize>(
    direction_world: &SVector<f64, D>,
) -> Result<(), ContactEffectiveMassError> {
    if !direction_world.iter().all(|value| value.is_finite()) {
        return Err(ContactEffectiveMassError::NonFiniteDirection);
    }
    let norm_squared = direction_world.norm_squared();
    if !norm_squared.is_finite() {
        return Err(ContactEffectiveMassError::NonFiniteDirection);
    }
    if (norm_squared - 1.0).abs() > UNIT_DIRECTION_TOLERANCE {
        return Err(ContactEffectiveMassError::NonUnitDirection);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::SMatrix;

    use crate::angular_dynamics::{angular_vector_to_bivector, bivector_to_angular_vector};

    #[test]
    fn canonical_wedge_and_point_velocity_sign_match_cross_product() {
        let offset = SVector::from([1.0, 0.0, 0.0]);
        let impulse = SVector::from([0.0, 1.0, 0.0]);
        let angular_impulse =
            angular_impulse_from_point_impulse_checked(&offset, &impulse).unwrap();
        assert!((angular_impulse.get(0, 1) - 1.0).abs() < 1.0e-14);
        assert!((angular_impulse.get(0, 2)).abs() < 1.0e-14);
        assert!((angular_impulse.get(1, 2)).abs() < 1.0e-14);

        let tensor = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([2.0, 3.0, 4.0]));
        let inertia = RotationalInertiaOperator::<3>::from_body_tensor_3d_checked(&tensor).unwrap();
        let response = point_impulse_response_checked(
            0.0,
            &offset,
            &impulse,
            &inertia,
            &Rotor::identity(),
        )
        .unwrap();
        let delta_omega = bivector_to_angular_vector(&response.angular_velocity_delta);
        assert!((delta_omega[2] - 0.25).abs() < 1.0e-12);
        assert!((response.angular_point_velocity_delta[1] - 0.25).abs() < 1.0e-12);
        assert!(response.angular_point_velocity_delta[1] > 0.0);
    }

    #[test]
    fn analytical_three_dimensional_effective_mass_matches_classical_formula() {
        let tensor = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([2.0, 3.0, 4.0]));
        let inertia = RotationalInertiaOperator::<3>::from_body_tensor_3d_checked(&tensor).unwrap();
        let offset = SVector::from([1.0, 0.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);
        let rotation = Rotor::<3>::identity();
        let body = ContactBodyResponseRef::Dynamic {
            inverse_mass: 0.5,
            offset_world: &offset,
            body_inertia: &inertia,
            rotation: &rotation,
        };

        let inverse_effective =
            directional_inverse_effective_mass_checked(&body, &normal).unwrap();
        // Classical 3D result: inv_m + n·((I^-1(r×n))×r)
        // = 0.5 + 1/Izz = 0.5 + 0.25.
        assert!((inverse_effective - 0.75).abs() < 1.0e-12);
    }

    #[test]
    fn anisotropic_directional_response_depends_on_body_orientation() {
        let tensor = SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([1.0, 4.0, 9.0]));
        let inertia = RotationalInertiaOperator::<3>::from_body_tensor_3d_checked(&tensor).unwrap();
        let offset = SVector::from([1.0, 0.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);
        let identity = Rotor::<3>::identity();
        let rotated = Rotor::from_bivector(&angular_vector_to_bivector(&SVector::from([
            0.0,
            std::f64::consts::FRAC_PI_2,
            0.0,
        ])));

        let identity_body = ContactBodyResponseRef::Dynamic {
            inverse_mass: 0.5,
            offset_world: &offset,
            body_inertia: &inertia,
            rotation: &identity,
        };
        let rotated_body = ContactBodyResponseRef::Dynamic {
            inverse_mass: 0.5,
            offset_world: &offset,
            body_inertia: &inertia,
            rotation: &rotated,
        };
        let a = directional_inverse_effective_mass_checked(&identity_body, &normal).unwrap();
        let b = directional_inverse_effective_mass_checked(&rotated_body, &normal).unwrap();
        assert!((a - b).abs() > 0.5, "identity={a}, rotated={b}");
    }

    #[test]
    fn pair_response_adds_two_body_contributions() {
        let inertia = RotationalInertiaOperator::<3>::diagonal_checked(&[4.0, 3.0, 2.0]).unwrap();
        let offset_a = SVector::from([1.0, 0.0, 0.0]);
        let offset_b = SVector::from([-1.0, 0.0, 0.0]);
        let normal = SVector::from([0.0, 1.0, 0.0]);
        let rotation = Rotor::<3>::identity();
        let a = ContactBodyResponseRef::Dynamic {
            inverse_mass: 0.5,
            offset_world: &offset_a,
            body_inertia: &inertia,
            rotation: &rotation,
        };
        let b = ContactBodyResponseRef::Dynamic {
            inverse_mass: 0.25,
            offset_world: &offset_b,
            body_inertia: &inertia,
            rotation: &rotation,
        };
        let ka = directional_inverse_effective_mass_checked(&a, &normal).unwrap();
        let kb = directional_inverse_effective_mass_checked(&b, &normal).unwrap();
        let pair = pair_directional_inverse_effective_mass_checked(&a, &b, &normal).unwrap();
        assert!((pair - (ka + kb)).abs() < 1.0e-14);
    }

    #[test]
    fn fixed_partner_contributes_zero() {
        let normal = SVector::<f64, 3>::from([0.0, 1.0, 0.0]);
        assert_eq!(
            directional_inverse_effective_mass_checked(&ContactBodyResponseRef::Fixed, &normal),
            Ok(0.0)
        );
    }

    #[test]
    fn four_dimensional_six_plane_response_is_finite_and_positive() {
        let inertia = RotationalInertiaOperator::<4>::diagonal_checked(&[2.0, 3.0, 4.0, 5.0, 6.0, 7.0]).unwrap();
        let offset = SVector::from([1.0, 0.0, 0.0, 0.0]);
        let direction = SVector::from([0.0, 1.0, 0.0, 0.0]);
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 2, 0.3);
        generator.set(1, 3, -0.4);
        let rotation = Rotor::from_bivector(&generator);
        let body = ContactBodyResponseRef::Dynamic {
            inverse_mass: 0.25,
            offset_world: &offset,
            body_inertia: &inertia,
            rotation: &rotation,
        };
        let response = directional_inverse_effective_mass_checked(&body, &direction).unwrap();
        assert!(response.is_finite());
        assert!(response > 0.0);
    }

    #[test]
    fn invalid_direction_and_impulse_inputs_fail_closed() {
        let inertia = RotationalInertiaOperator::<3>::diagonal_checked(&[1.0, 2.0, 3.0]).unwrap();
        let offset = SVector::from([1.0, 0.0, 0.0]);
        let rotation = Rotor::<3>::identity();
        let body = ContactBodyResponseRef::Dynamic {
            inverse_mass: 1.0,
            offset_world: &offset,
            body_inertia: &inertia,
            rotation: &rotation,
        };
        assert_eq!(
            directional_inverse_effective_mass_checked(
                &body,
                &SVector::from([0.0, 2.0, 0.0]),
            ),
            Err(ContactEffectiveMassError::NonUnitDirection)
        );
        assert_eq!(
            angular_impulse_from_point_impulse_checked(
                &offset,
                &SVector::from([f64::NAN, 0.0, 0.0]),
            ),
            Err(ContactEffectiveMassError::NonFiniteImpulse)
        );
    }

    #[test]
    fn finite_inputs_that_overflow_wedge_fail_closed() {
        let offset = SVector::from([f64::MAX, 0.0, 0.0]);
        let impulse = SVector::from([0.0, f64::MAX, 0.0]);
        assert_eq!(
            angular_impulse_from_point_impulse_checked(&offset, &impulse),
            Err(ContactEffectiveMassError::UnrepresentableAngularImpulse)
        );
    }
}
