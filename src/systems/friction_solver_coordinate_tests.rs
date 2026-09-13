// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, FrictionApplicationError, FrictionDiagnosticReason, FrictionSolverCoordinates,
    FrictionTransactionId, RigidBody,
};

use super::thermodynamic_runtime::{RuntimeFrictionError, ThermodynamicTransactionRuntime};
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;

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
fn typed_coordinates_cannot_choose_fixed_tick_identity() {
    let coordinates = FrictionSolverCoordinates::new(2, 3, 4);
    let mut runtime = ThermodynamicTransactionRuntime::new();

    runtime.begin_next_tick().unwrap();
    let mut a0 = body(1, [-1.0, 0.0, 0.0], 1.0);
    let mut b0 = body(2, [1.0, 0.0, 0.0], 0.0);
    let applied0 = runtime
        .apply_friction_impulse_at(
            &mut a0,
            &mut b0,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            coordinates,
        )
        .unwrap();
    assert_eq!(applied0.transaction_id(), FrictionTransactionId::new(0, 2, 3, 4));
    assert_eq!(
        runtime
            .finalize_friction_diagnostic(&a0, &b0, &applied0)
            .unwrap(),
        FrictionDiagnosticReason::OffCenterUnqualified
    );
    let permit0 = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();
    runtime.commit_finalize_and_rotate(&permit0).unwrap();

    runtime.begin_next_tick().unwrap();
    let mut a1 = body(1, [-1.0, 0.0, 0.0], 1.0);
    let mut b1 = body(2, [1.0, 0.0, 0.0], 0.0);
    let applied1 = runtime
        .apply_friction_impulse_at(
            &mut a1,
            &mut b1,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            coordinates,
        )
        .unwrap();
    assert_eq!(applied1.transaction_id(), FrictionTransactionId::new(1, 2, 3, 4));
}

#[test]
fn duplicate_typed_coordinates_reject_before_second_mechanical_mutation() {
    let coordinates = FrictionSolverCoordinates::new(0, 0, 0);
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
    let mut b = body(2, [0.0, 0.0, 0.0], 0.0);

    let _first = runtime
        .apply_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            coordinates,
        )
        .unwrap();
    let state_a = (a.linear_velocity, a.angular_velocity);
    let state_b = (b.linear_velocity, b.angular_velocity);

    let error = runtime
        .apply_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            coordinates,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        RuntimeFrictionError::Application(FrictionApplicationError::DuplicateTransaction)
    ));
    assert_eq!((a.linear_velocity, a.angular_velocity), state_a);
    assert_eq!((b.linear_velocity, b.angular_velocity), state_b);
}
