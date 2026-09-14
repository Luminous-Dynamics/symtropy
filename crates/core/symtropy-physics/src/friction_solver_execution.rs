// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Crate-private execution policy for world-solver friction.
//!
//! The legacy world API never needed replay identity, so it must not acquire a
//! new failure mode merely because a native solver index cannot fit the stable
//! `u32` friction-coordinate grammar. The authority-aware world API, by contrast,
//! must fail before mechanics when that conversion is not lossless.
//!
//! An exact zero impulse is not a friction transaction. Both executor paths
//! return before mechanics/identity/authority in that case, so zero-friction
//! controls do not manufacture neutral receipts or consume transaction ids.

use std::convert::Infallible;

use nalgebra::SVector;

use crate::body::RigidBody;
use crate::friction_authority::FrictionImpulseAuthority;
use crate::friction_evidence::apply_friction_impulse_mechanics;
use crate::friction_step::{FrictionStepError, execute_friction_impulse_at_indices};

#[inline]
fn is_exact_zero_impulse<const D: usize>(impulse: &SVector<f64, D>) -> bool {
    impulse.iter().all(|value| *value == 0.0)
}

/// Internal policy used by `PhysicsWorld` while resolving one friction request.
///
/// Native nested-loop coordinates stay `usize` until this boundary. Each executor
/// decides whether replay identity is required and whether the old pseudo-energy
/// callback telemetry should remain enabled for compatibility.
pub(crate) trait SolverFrictionExecutor<const D: usize> {
    type Error;

    fn execute(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        solver_iteration: usize,
        contact_sequence: usize,
        point_sequence: usize,
    ) -> Result<(), Self::Error>;

    /// Legacy `record_dissipation(|J| * 0.1)` survives only behind the old
    /// compatibility stepping surface. The authority-aware path must not emit it.
    fn records_legacy_dissipation_telemetry(&self) -> bool;
}

/// Existing world behavior: direct shared mechanics, no replay identity.
///
/// This executor intentionally ignores solver indices. Its error type is
/// `Infallible`, preserving the old void-returning world stepping contract even if
/// a native index is larger than the authority path's replay grammar can encode.
#[derive(Default)]
pub(crate) struct LegacyDirectFrictionExecutor;

impl<const D: usize> SolverFrictionExecutor<D> for LegacyDirectFrictionExecutor {
    type Error = Infallible;

    fn execute(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        _solver_iteration: usize,
        _contact_sequence: usize,
        _point_sequence: usize,
    ) -> Result<(), Self::Error> {
        if is_exact_zero_impulse(impulse_on_b) {
            return Ok(());
        }
        apply_friction_impulse_mechanics(body_a, body_b, contact_point, impulse_on_b);
        Ok(())
    }

    fn records_legacy_dissipation_telemetry(&self) -> bool {
        true
    }
}

/// Checked authority-aware world execution.
pub(crate) struct CheckedFrictionExecutor<'a, A> {
    authority: &'a mut A,
}

impl<'a, A> CheckedFrictionExecutor<'a, A> {
    pub(crate) fn new(authority: &'a mut A) -> Self {
        Self { authority }
    }
}

impl<const D: usize, A> SolverFrictionExecutor<D> for CheckedFrictionExecutor<'_, A>
where
    A: FrictionImpulseAuthority<D>,
{
    type Error = FrictionStepError<A::Error>;

    fn execute(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        solver_iteration: usize,
        contact_sequence: usize,
        point_sequence: usize,
    ) -> Result<(), Self::Error> {
        // No mechanical transition means no authority transaction and therefore
        // no replay identity to validate. This check deliberately precedes the
        // `usize -> u32` conversion below.
        if is_exact_zero_impulse(impulse_on_b) {
            return Ok(());
        }

        execute_friction_impulse_at_indices(
            self.authority,
            body_a,
            body_b,
            contact_point,
            impulse_on_b,
            solver_iteration,
            contact_sequence,
            point_sequence,
        )
    }

    fn records_legacy_dissipation_telemetry(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;
    use crate::friction_coordinates::{
        FrictionSolverCoordinateComponent, FrictionSolverCoordinates,
    };
    use crate::friction_step::{FrictionAuthorityFailure, FrictionStepError};
    use symtropy_math::Point;

    fn body(handle: usize, velocity_x: f64) -> RigidBody<3> {
        let mut body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(handle),
            Point::origin(),
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
            contact_point: &SVector<f64, 3>,
            impulse_on_b: &SVector<f64, 3>,
            coordinates: FrictionSolverCoordinates,
        ) -> Result<(), Self::Error> {
            self.seen.push((body_a.handle, body_b.handle, coordinates));
            apply_friction_impulse_mechanics(body_a, body_b, contact_point, impulse_on_b);
            Ok(())
        }
    }

    #[test]
    fn legacy_direct_execution_does_not_acquire_coordinate_overflow_failure() {
        if usize::BITS <= u32::BITS {
            return;
        }

        let oversized = (u32::MAX as usize) + 1;
        let mut executor = LegacyDirectFrictionExecutor;
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let before_a = a.linear_velocity;
        let before_b = b.linear_velocity;

        executor
            .execute(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.25, 0.0, 0.0]),
                oversized,
                oversized,
                oversized,
            )
            .unwrap();

        assert_ne!(a.linear_velocity, before_a);
        assert_ne!(b.linear_velocity, before_b);
        assert!(<LegacyDirectFrictionExecutor as SolverFrictionExecutor<3>>::records_legacy_dissipation_telemetry(&executor));
    }

    #[test]
    fn exact_zero_impulse_is_no_transaction_even_with_unrepresentable_coordinates() {
        let mut authority = RecordingAuthority::default();
        let mut executor = CheckedFrictionExecutor::new(&mut authority);
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity);
        let before_b = (b.linear_velocity, b.angular_velocity);

        executor
            .execute(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::zeros(),
                usize::MAX,
                usize::MAX,
                usize::MAX,
            )
            .unwrap();
        drop(executor);

        assert!(authority.seen.is_empty());
        assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
    }

    #[test]
    fn legacy_exact_zero_impulse_is_mechanically_noop() {
        let mut executor = LegacyDirectFrictionExecutor;
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity);
        let before_b = (b.linear_velocity, b.angular_velocity);

        executor
            .execute(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::zeros(),
                usize::MAX,
                usize::MAX,
                usize::MAX,
            )
            .unwrap();

        assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
    }

    #[test]
    fn checked_execution_preserves_exact_nested_identity_and_disables_pseudo_energy() {
        let mut authority = RecordingAuthority::default();
        let mut executor = CheckedFrictionExecutor::new(&mut authority);
        let mut a = body(7, 1.0);
        let mut b = body(9, 0.0);

        executor
            .execute(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.25, 0.0, 0.0]),
                3,
                11,
                2,
            )
            .unwrap();
        assert!(!<CheckedFrictionExecutor<'_, RecordingAuthority> as SolverFrictionExecutor<3>>::records_legacy_dissipation_telemetry(&executor));
        drop(executor);

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
    fn checked_overflow_rejects_before_authority_or_mechanical_mutation() {
        if usize::BITS <= u32::BITS {
            return;
        }

        let oversized = (u32::MAX as usize) + 1;
        let mut authority = RecordingAuthority::default();
        let mut executor = CheckedFrictionExecutor::new(&mut authority);
        let mut a = body(1, 1.0);
        let mut b = body(2, 0.0);
        let before_a = (a.linear_velocity, a.angular_velocity);
        let before_b = (b.linear_velocity, b.angular_velocity);

        let error = executor
            .execute(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.25, 0.0, 0.0]),
                0,
                oversized,
                0,
            )
            .unwrap_err();
        assert_eq!(
            error,
            FrictionStepError::Coordinate(crate::friction_coordinates::FrictionSolverCoordinateError {
                component: FrictionSolverCoordinateComponent::ContactSequence,
                value: oversized,
            })
        );
        drop(executor);

        assert!(authority.seen.is_empty());
        assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
        assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
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
    fn checked_authority_failure_preserves_body_pair_and_coordinates() {
        let mut authority = RejectingAuthority;
        let mut executor = CheckedFrictionExecutor::new(&mut authority);
        let mut a = body(4, 1.0);
        let mut b = body(5, 0.0);

        assert_eq!(
            executor.execute(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.25, 0.0, 0.0]),
                6,
                8,
                1,
            ),
            Err(FrictionStepError::Authority(FrictionAuthorityFailure {
                body_a: BodyHandle(4),
                body_b: BodyHandle(5),
                coordinates: FrictionSolverCoordinates::new(6, 8, 1),
                error: "blocked",
            }))
        );
    }
}
