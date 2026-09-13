// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked dispatch from native solver traversal into friction authority.
//!
//! Production solver loops use `usize` indices, while replay-stable friction
//! identity uses `u32`. This module makes that conversion explicit and fail-closed,
//! then preserves exact body-pair + solver-coordinate identity if the selected
//! authority rejects the request.

use nalgebra::SVector;

use crate::body::{BodyHandle, RigidBody};
use crate::friction_authority::FrictionImpulseAuthority;
use crate::friction_coordinates::{FrictionSolverCoordinateError, FrictionSolverCoordinates};

/// Exact context for a friction-authority rejection after native solver
/// coordinates have been losslessly converted.
#[derive(Debug, PartialEq, Eq)]
pub struct FrictionAuthorityFailure<E> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub coordinates: FrictionSolverCoordinates,
    pub error: E,
}

/// Failures that can occur while dispatching one world-solver friction request
/// into an authority implementation.
#[derive(Debug, PartialEq, Eq)]
pub enum FrictionStepError<E> {
    /// A native solver index did not fit the replay-stable `u32` grammar. The
    /// authority has not been called and no friction mechanics have occurred.
    Coordinate(FrictionSolverCoordinateError),
    /// The authority rejected one exact body pair / solver-local transaction.
    Authority(FrictionAuthorityFailure<E>),
}

/// Convert native nested-loop coordinates without truncation, then execute one
/// exact friction request through the supplied authority.
///
/// Coordinate conversion is deliberately performed before calling the authority,
/// so an oversized iteration/contact/point index cannot alias an earlier event or
/// mutate mechanics under a lossy identity.
pub fn execute_friction_impulse_at_indices<const D: usize, A>(
    authority: &mut A,
    body_a: &mut RigidBody<D>,
    body_b: &mut RigidBody<D>,
    contact_point: &SVector<f64, D>,
    impulse_on_b: &SVector<f64, D>,
    solver_iteration: usize,
    contact_sequence: usize,
    point_sequence: usize,
) -> Result<(), FrictionStepError<A::Error>>
where
    A: FrictionImpulseAuthority<D>,
{
    let coordinates = FrictionSolverCoordinates::try_from_indices(
        solver_iteration,
        contact_sequence,
        point_sequence,
    )
    .map_err(FrictionStepError::Coordinate)?;

    let body_a_handle = body_a.handle;
    let body_b_handle = body_b.handle;
    authority
        .execute_friction_impulse(
            body_a,
            body_b,
            contact_point,
            impulse_on_b,
            coordinates,
        )
        .map_err(|error| {
            FrictionStepError::Authority(FrictionAuthorityFailure {
                body_a: body_a_handle,
                body_b: body_b_handle,
                coordinates,
                error,
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::friction_authority::DirectFrictionImpulseAuthority;
    use crate::friction_coordinates::FrictionSolverCoordinateComponent;
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

    #[derive(Default)]
    struct RecordingAuthority {
        seen: Vec<(BodyHandle, BodyHandle, FrictionSolverCoordinates)>,
    }

    impl FrictionImpulseAuthority<3> for RecordingAuthority {
        type Error = &'static str;

        fn execute_friction_impulse(
            &mut self,
            body_a: &mut RigidBody<3>,
            body_b: &mut RigidBody<3>,
            _contact_point: &SVector<f64, 3>,
            _impulse_on_b: &SVector<f64, 3>,
            coordinates: FrictionSolverCoordinates,
        ) -> Result<(), Self::Error> {
            self.seen.push((body_a.handle, body_b.handle, coordinates));
            Ok(())
        }
    }

    struct RejectingAuthority;

    impl FrictionImpulseAuthority<3> for RejectingAuthority {
        type Error = &'static str;

        fn execute_friction_impulse(
            &mut self,
            _body_a: &mut RigidBody<3>,
            _body_b: &mut RigidBody<3>,
            _contact_point: &SVector<f64, 3>,
            _impulse_on_b: &SVector<f64, 3>,
            _coordinates: FrictionSolverCoordinates,
        ) -> Result<(), Self::Error> {
            Err("blocked")
        }
    }

    #[test]
    fn checked_dispatch_preserves_nested_solver_identity_and_body_pair() {
        let mut authority = RecordingAuthority::default();
        let mut a = body(7, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(9, [0.0, 0.0, 0.0], 0.0);

        execute_friction_impulse_at_indices(
            &mut authority,
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.25, 0.0, 0.0]),
            3,
            11,
            2,
        )
        .unwrap();

        assert_eq!(
            authority.seen,
            vec![(
                BodyHandle(7),
                BodyHandle(9),
                FrictionSolverCoordinates::new(3, 11, 2),
            )]
        );
    }

    #[test]
    fn authority_rejection_carries_exact_pair_and_coordinates() {
        let mut authority = RejectingAuthority;
        let mut a = body(4, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(5, [0.0, 0.0, 0.0], 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity);
        let before_b = (b.linear_velocity, b.angular_velocity);

        let error = execute_friction_impulse_at_indices(
            &mut authority,
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.25, 0.0, 0.0]),
            6,
            8,
            1,
        )
        .unwrap_err();

        assert_eq!(
            error,
            FrictionStepError::Authority(FrictionAuthorityFailure {
                body_a: BodyHandle(4),
                body_b: BodyHandle(5),
                coordinates: FrictionSolverCoordinates::new(6, 8, 1),
                error: "blocked",
            })
        );
        // This rejecting test authority intentionally performs no mechanics.
        assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
    }

    #[test]
    fn oversized_native_coordinate_rejects_before_authority_call() {
        if usize::BITS <= u32::BITS {
            return;
        }

        let oversized = (u32::MAX as usize) + 1;
        let mut authority = RecordingAuthority::default();
        let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity);
        let before_b = (b.linear_velocity, b.angular_velocity);

        assert_eq!(
            execute_friction_impulse_at_indices(
                &mut authority,
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.25, 0.0, 0.0]),
                0,
                oversized,
                0,
            ),
            Err(FrictionStepError::Coordinate(
                FrictionSolverCoordinateError {
                    component: FrictionSolverCoordinateComponent::ContactSequence,
                    value: oversized,
                }
            ))
        );
        assert!(authority.seen.is_empty());
        assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
    }

    #[test]
    fn direct_authority_dispatch_remains_the_exact_compatibility_mechanics() {
        let mut authority = DirectFrictionImpulseAuthority;
        let mut a = body(1, [-1.0, 0.0, 0.0], 1.0);
        let mut b = body(2, [1.0, 0.0, 0.0], 0.0);
        let before_a = a.linear_velocity;
        let before_b = b.linear_velocity;

        execute_friction_impulse_at_indices(
            &mut authority,
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            1,
            2,
            3,
        )
        .unwrap();

        assert_ne!(a.linear_velocity, before_a);
        assert_ne!(b.linear_velocity, before_b);
        assert!(a.angular_velocity.norm_squared() > 0.0);
        assert!(b.angular_velocity.norm_squared() > 0.0);
    }
}
