// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Pluggable authority boundary for one solver friction impulse.
//!
//! The physics solver owns *when* and *where* a friction request occurs. The
//! authority owns the mechanical application/evidence transaction for that exact
//! request. This keeps solver traversal independent from launcher-specific
//! thermodynamic journals while still allowing production integration to replace
//! direct mechanics with a stronger exactly-once transaction.

use std::convert::Infallible;

use nalgebra::SVector;

use crate::body::RigidBody;
use crate::friction_coordinates::FrictionSolverCoordinates;
use crate::friction_evidence::apply_friction_impulse_mechanics;

/// Execute exactly one friction impulse at one deterministic solver-local
/// coordinate.
///
/// Implementations may author stronger evidence/receipts than the solver itself
/// understands. Returning `Err` means the requested friction transition did not
/// reach the implementation's required authority boundary and the caller must not
/// pretend it succeeded.
pub trait FrictionImpulseAuthority<const D: usize> {
    type Error;

    fn execute_friction_impulse(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        coordinates: FrictionSolverCoordinates,
    ) -> Result<(), Self::Error>;
}

/// Compatibility authority that performs the exact existing linear + lever-arm
/// angular mechanical transition and authors no thermodynamic evidence.
///
/// This is intentionally infallible and keeps standalone/core callers behaviorally
/// compatible while production launcher code can supply a stronger authority.
#[derive(Copy, Clone, Debug, Default)]
pub struct DirectFrictionImpulseAuthority;

impl<const D: usize> FrictionImpulseAuthority<D> for DirectFrictionImpulseAuthority {
    type Error = Infallible;

    fn execute_friction_impulse(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        _coordinates: FrictionSolverCoordinates,
    ) -> Result<(), Self::Error> {
        apply_friction_impulse_mechanics(body_a, body_b, contact_point, impulse_on_b);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use symtropy_math::Point;

    fn body(handle: usize, position: [f64; 3], velocity_x: f64) -> RigidBody<3> {
        let mut body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(handle),
            Point::new(position),
            0.5,
            1.0,
        );
        body.linear_velocity[0] = velocity_x;
        body
    }

    #[test]
    fn direct_authority_executes_exact_mechanical_transition() {
        let mut authority = DirectFrictionImpulseAuthority;
        let mut a = body(1, [-1.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [1.0, 0.0, 0.0], 0.0);
        let mut expected_a = body(1, [-1.0, 0.0, 0.0], 1.0);
        let mut expected_b = body(2, [1.0, 0.0, 0.0], 0.0);
        let contact = SVector::from([0.0, 0.5, 0.0]);
        let impulse = SVector::from([0.0, -0.1, 0.0]);

        apply_friction_impulse_mechanics(
            &mut expected_a,
            &mut expected_b,
            &contact,
            &impulse,
        );
        authority
            .execute_friction_impulse(
                &mut a,
                &mut b,
                &contact,
                &impulse,
                FrictionSolverCoordinates::new(7, 9, 2),
            )
            .unwrap();

        assert_eq!(a.linear_velocity, expected_a.linear_velocity);
        assert_eq!(b.linear_velocity, expected_b.linear_velocity);
        assert_eq!(a.angular_velocity, expected_a.angular_velocity);
        assert_eq!(b.angular_velocity, expected_b.angular_velocity);
    }
}
