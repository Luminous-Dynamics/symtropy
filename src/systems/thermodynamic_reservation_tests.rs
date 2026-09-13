// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::thermodynamic_runtime::{
    RuntimeFrictionError, RuntimeFrictionGateError, ThermodynamicRuntimeError,
    ThermodynamicTransactionRuntime,
};
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;
use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, FrictionApplicationError, FrictionEvidenceError, FrictionSolverCoordinates,
    FrictionTransactionId, RigidBody,
};

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

fn request() -> (SVector<f64, 3>, SVector<f64, 3>, FrictionSolverCoordinates) {
    (
        SVector::zeros(),
        SVector::from([0.5, 0.0, 0.0]),
        FrictionSolverCoordinates::new(2, 3, 4),
    )
}

#[test]
fn reservation_mints_tick_identity_and_blocks_finalize_before_mechanics() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let (point, impulse, coordinates) = request();
    let reservation = runtime
        .reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(2),
            &point,
            &impulse,
            coordinates,
        )
        .unwrap();

    assert_eq!(reservation.transaction_id(), FrictionTransactionId::new(0, 2, 3, 4));
    assert_eq!(runtime.pending_friction_reservation_count(), 1);
    assert!(runtime.friction_journal().is_empty());
    assert!(matches!(
        runtime.prepare_finalize(ThermodynamicConsequenceStatus::Executed),
        Err(ThermodynamicRuntimeError::PendingFrictionReservations { count: 1 })
    ));

    runtime.cancel_friction_reservation(reservation).unwrap();
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
    let permit = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();
    runtime.rollback_finalize(permit).unwrap();
}

#[test]
fn duplicate_reservation_is_rejected_before_any_mechanical_mutation() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let (point, impulse, coordinates) = request();
    let first = runtime
        .reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(2),
            &point,
            &impulse,
            coordinates,
        )
        .unwrap();

    let mut a = body(1, 1.0);
    let mut b = body(2, 0.0);
    let state_a = (a.linear_velocity, a.angular_velocity);
    let state_b = (b.linear_velocity, b.angular_velocity);

    assert!(matches!(
        runtime.reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(2),
            &point,
            &impulse,
            coordinates,
        ),
        Err(RuntimeFrictionError::Application(
            FrictionApplicationError::DuplicateTransaction
        ))
    ));
    assert_eq!((a.linear_velocity, a.angular_velocity), state_a);
    assert_eq!((b.linear_velocity, b.angular_velocity), state_b);

    let applied = runtime
        .apply_reserved_friction_impulse(&mut a, &mut b, first)
        .unwrap();
    assert_eq!(applied.transaction_id(), FrictionTransactionId::new(0, 2, 3, 4));
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}

#[test]
fn reservation_binds_body_identity_and_releases_on_substitution_rejection() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let (point, impulse, coordinates) = request();
    let reservation = runtime
        .reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(2),
            &point,
            &impulse,
            coordinates,
        )
        .unwrap();

    let mut a = body(1, 1.0);
    let mut substituted_b = body(9, 0.0);
    let state_a = (a.linear_velocity, a.angular_velocity);
    let state_b = (substituted_b.linear_velocity, substituted_b.angular_velocity);

    assert!(matches!(
        runtime.apply_reserved_friction_impulse(&mut a, &mut substituted_b, reservation),
        Err(RuntimeFrictionError::Gate(
            RuntimeFrictionGateError::ReservationBodyMismatch {
                expected_a: BodyHandle(1),
                expected_b: BodyHandle(2),
                actual_a: BodyHandle(1),
                actual_b: BodyHandle(9),
            }
        ))
    ));
    assert_eq!((a.linear_velocity, a.angular_velocity), state_a);
    assert_eq!(
        (substituted_b.linear_velocity, substituted_b.angular_velocity),
        state_b
    );
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
    assert!(runtime.friction_journal().is_empty());

    // The released identity can be admitted again because no mechanics/evidence
    // was authored by the rejected substitution attempt.
    let retry = runtime
        .reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(2),
            &point,
            &impulse,
            coordinates,
        )
        .unwrap();
    runtime.cancel_friction_reservation(retry).unwrap();
}

#[test]
fn invalid_mechanical_state_rolls_back_and_releases_reservation() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let (point, impulse, coordinates) = request();
    let reservation = runtime
        .reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(2),
            &point,
            &impulse,
            coordinates,
        )
        .unwrap();

    let mut a = body(1, 1.0);
    let mut b = body(2, 0.0);
    a.linear_velocity[0] = f64::NAN;
    let state_b = (b.linear_velocity, b.angular_velocity);

    assert!(matches!(
        runtime.apply_reserved_friction_impulse(&mut a, &mut b, reservation),
        Err(RuntimeFrictionError::Application(
            FrictionApplicationError::Evidence(FrictionEvidenceError::NonFiniteMechanicalState)
        ))
    ));
    assert!(a.linear_velocity[0].is_nan());
    assert_eq!((b.linear_velocity, b.angular_velocity), state_b);
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
    assert!(runtime.friction_journal().is_empty());
}

#[test]
fn reservation_rejects_invalid_identity_inputs_before_admission() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let coordinates = FrictionSolverCoordinates::new(0, 0, 0);

    assert!(matches!(
        runtime.reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(1),
            &SVector::zeros(),
            &SVector::zeros(),
            coordinates,
        ),
        Err(RuntimeFrictionError::Application(
            FrictionApplicationError::Evidence(FrictionEvidenceError::SameBody)
        ))
    ));
    assert_eq!(runtime.pending_friction_reservation_count(), 0);

    let nonfinite_point = SVector::from([f64::NAN, 0.0, 0.0]);
    assert!(matches!(
        runtime.reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(2),
            &nonfinite_point,
            &SVector::zeros(),
            coordinates,
        ),
        Err(RuntimeFrictionError::Application(
            FrictionApplicationError::Evidence(FrictionEvidenceError::NonFiniteContactPoint)
        ))
    ));
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}
