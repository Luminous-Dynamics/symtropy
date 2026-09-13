// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked 2D kinetic-energy evidence consistent with the current solver convention.
//!
//! The production ND angular impulse path currently uses the mean of the two stored
//! inverse inertias for the single 2D rotation plane. Therefore the quadratic
//! rotational energy conjugate to that solver is based on the reciprocal of that
//! mean inverse inertia, not the arithmetic mean of the stored inertias.
//!
//! This is **solver-consistent evidence**, not a claim that the current generic
//! `RigidBody<2>` inertia representation is an exact real-world inertia tensor for
//! arbitrary anisotropic shapes.

use crate::body::RigidBody;

const RECIPROCAL_REL_TOLERANCE: f64 = 1.0e-10;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RigidBodyEnergy2dError {
    InvalidMass,
    InconsistentInverseMass,
    NonFiniteLinearVelocity,
    UnrepresentableLinearEnergy,
    InvalidInertia,
    InconsistentInverseInertia,
    NonFiniteAngularVelocity,
    UnrepresentableAngularEnergy,
    UnrepresentableTotalEnergy,
}

fn reciprocal_is_consistent(value: f64, inverse: f64) -> bool {
    if !value.is_finite() || value <= 0.0 || !inverse.is_finite() || inverse <= 0.0 {
        return false;
    }
    let product = value * inverse;
    product.is_finite()
        && (product - 1.0).abs()
            <= RECIPROCAL_REL_TOLERANCE * product.abs().max(1.0)
}

impl RigidBody<2> {
    /// Effective scalar inverse inertia used by the current 2D angular impulse solver.
    ///
    /// This is the exact convention used by `integrator::apply_angular_impulse` for
    /// `D = 2`: arithmetic mean of the two stored inverse-inertia components.
    pub fn solver_inverse_inertia_2d_checked(&self) -> Result<f64, RigidBodyEnergy2dError> {
        if !self.is_dynamic() {
            return Ok(0.0);
        }
        for axis in 0..2 {
            if !self.inertia[axis].is_finite() || self.inertia[axis] <= 0.0 {
                return Err(RigidBodyEnergy2dError::InvalidInertia);
            }
            if !reciprocal_is_consistent(self.inertia[axis], self.inv_inertia[axis]) {
                return Err(RigidBodyEnergy2dError::InconsistentInverseInertia);
            }
        }

        let inverse = (self.inv_inertia[0] + self.inv_inertia[1]) * 0.5;
        if !inverse.is_finite() || inverse <= 0.0 {
            return Err(RigidBodyEnergy2dError::InvalidInertia);
        }
        Ok(inverse)
    }

    /// Checked kinetic energy for `RigidBody<2>` under the *current solver's*
    /// linear and angular impulse convention.
    ///
    /// Dynamic bodies require coherent reciprocal mass/inertia fields. Static and
    /// kinematic bodies return zero, matching the existing compatibility surface.
    pub fn kinetic_energy_2d_solver_checked(&self) -> Result<f64, RigidBodyEnergy2dError> {
        if !self.is_dynamic() {
            return Ok(0.0);
        }

        if !self.mass.is_finite() || self.mass <= 0.0 || !self.inv_mass.is_finite() || self.inv_mass <= 0.0 {
            return Err(RigidBodyEnergy2dError::InvalidMass);
        }
        if !reciprocal_is_consistent(self.mass, self.inv_mass) {
            return Err(RigidBodyEnergy2dError::InconsistentInverseMass);
        }
        if !self.linear_velocity.iter().all(|value| value.is_finite()) {
            return Err(RigidBodyEnergy2dError::NonFiniteLinearVelocity);
        }

        let speed_squared = self.linear_velocity.norm_squared();
        let linear = 0.5 * self.mass * speed_squared;
        if !speed_squared.is_finite() || !linear.is_finite() || linear < 0.0 {
            return Err(RigidBodyEnergy2dError::UnrepresentableLinearEnergy);
        }

        if !self.angular_velocity.is_finite() {
            return Err(RigidBodyEnergy2dError::NonFiniteAngularVelocity);
        }
        let inverse_inertia = self.solver_inverse_inertia_2d_checked()?;
        let effective_inertia = 1.0 / inverse_inertia;
        let omega = self.angular_velocity.get(0, 1);
        let angular = 0.5 * effective_inertia * omega * omega;
        if !effective_inertia.is_finite() || !angular.is_finite() || angular < 0.0 {
            return Err(RigidBodyEnergy2dError::UnrepresentableAngularEnergy);
        }

        let total = linear + angular;
        if !total.is_finite() || total < 0.0 {
            return Err(RigidBodyEnergy2dError::UnrepresentableTotalEnergy);
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BodyHandle, BodyType};
    use crate::integrator;
    use nalgebra::SVector;
    use symtropy_math::{Bivector, Point, Sphere, Transform};

    fn anisotropic_body() -> RigidBody<2> {
        let mut body = RigidBody::new(
            BodyHandle(1),
            BodyType::Dynamic,
            Transform::from_translation(Point::origin()),
            Box::new(Sphere::<2>::unit()),
            2.0,
            SVector::from([2.0, 8.0]),
        );
        body.linear_velocity = SVector::from([3.0, 4.0]);
        body.angular_velocity = Bivector::zero();
        body.angular_velocity.set(0, 1, 2.0);
        body
    }

    #[test]
    fn isotropic_body_matches_legacy_scalar_energy() {
        let mut body = RigidBody::<2>::dynamic_sphere(
            BodyHandle(1),
            Point::origin(),
            0.5,
            2.0,
        );
        body.linear_velocity = SVector::from([2.0, -1.0]);
        body.angular_velocity.set(0, 1, 3.0);

        let checked = body.kinetic_energy_2d_solver_checked().unwrap();
        let legacy = body.kinetic_energy();
        assert!((checked - legacy).abs() < 1.0e-12);
    }

    #[test]
    fn anisotropic_energy_uses_solver_effective_inertia_not_mean_inertia() {
        let body = anisotropic_body();
        let checked = body.kinetic_energy_2d_solver_checked().unwrap();
        let legacy = body.kinetic_energy();

        let linear = 0.5 * 2.0 * 25.0;
        let inverse_inertia = (0.5 + 0.125) * 0.5;
        let expected_angular = 0.5 * (1.0 / inverse_inertia) * 4.0;
        let expected = linear + expected_angular;

        assert!((checked - expected).abs() < 1.0e-12);
        assert!((checked - legacy).abs() > 1.0);
    }

    #[test]
    fn measured_delta_matches_current_angular_impulse_solver_quadratic() {
        let mut body = anisotropic_body();
        body.linear_velocity = SVector::zeros();
        let before = body.kinetic_energy_2d_solver_checked().unwrap();
        let omega_before = body.angular_velocity.get(0, 1);
        let inv_i = body.solver_inverse_inertia_2d_checked().unwrap();

        let mut angular_impulse = Bivector::<2>::zero();
        angular_impulse.set(0, 1, 0.75);
        integrator::apply_angular_impulse(&mut body, &angular_impulse);

        let after = body.kinetic_energy_2d_solver_checked().unwrap();
        let generalized_impulse = 0.75;
        let expected_delta = omega_before * generalized_impulse
            + 0.5 * generalized_impulse * generalized_impulse * inv_i;
        assert!(((after - before) - expected_delta).abs() < 1.0e-12);
    }

    #[test]
    fn inconsistent_inverse_mass_fails_closed() {
        let mut body = anisotropic_body();
        body.inv_mass *= 0.5;
        assert_eq!(
            body.kinetic_energy_2d_solver_checked(),
            Err(RigidBodyEnergy2dError::InconsistentInverseMass)
        );
    }

    #[test]
    fn inconsistent_inverse_inertia_fails_closed() {
        let mut body = anisotropic_body();
        body.inv_inertia[1] *= 0.5;
        assert_eq!(
            body.kinetic_energy_2d_solver_checked(),
            Err(RigidBodyEnergy2dError::InconsistentInverseInertia)
        );
    }

    #[test]
    fn static_body_reports_zero_without_inventing_dynamic_energy() {
        let body = RigidBody::<2>::static_body(
            BodyHandle(7),
            Point::origin(),
            Box::new(Sphere::<2>::unit()),
        );
        assert_eq!(body.kinetic_energy_2d_solver_checked().unwrap(), 0.0);
    }
}
