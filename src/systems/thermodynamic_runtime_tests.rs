// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::thermodynamic_runtime::{
    RuntimeFrictionError, RuntimeFrictionGateError, ThermodynamicTransactionRuntime,
};
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;
use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, EnergyTransferLedger, FrictionApplicationError, FrictionDiagnosticReason,
    FrictionTransactionId, FrictionTransactionPhase, HeatPartition, RigidBody, ThermalBody,
    ThermalMaterial, ThermalState,
};

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

fn thermal_body(handle: usize, velocity_x: f64) -> RigidBody<3> {
    let mut body = body(handle, [0.0, 0.0, 0.0], velocity_x);
    body.set_thermal(
        ThermalBody::new(
            ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
            ThermalState::new(300.0).unwrap(),
            1.0,
        )
        .unwrap(),
    );
    body
}

#[test]
fn runtime_mints_fixed_tick_and_rotates_only_after_commit() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = body(1, [-1.0, 0.0, 0.0], 1.0);
    let mut b = body(2, [1.0, 0.0, 0.0], 0.0);
    let applied = runtime
        .apply_friction_impulse(
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            2,
            3,
            4,
        )
        .unwrap();
    let id = applied.transaction_id();
    assert_eq!(id, FrictionTransactionId::new(0, 2, 3, 4));
    assert_eq!(
        runtime
            .finalize_friction_diagnostic(&a, &b, &applied)
            .unwrap(),
        FrictionDiagnosticReason::OffCenterUnqualified
    );

    let permit = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();
    let receipt = runtime.commit_finalize_and_rotate(&permit).unwrap();
    assert_eq!(receipt.tick_id, 0);
    assert_eq!(
        receipt.friction_transactions.phase(id),
        Some(FrictionTransactionPhase::DiagnosticOnly(
            FrictionDiagnosticReason::OffCenterUnqualified
        ))
    );
    assert!(runtime.friction_journal().is_empty());
    assert_eq!(runtime.next_tick_id(), Some(1));

    runtime.begin_next_tick().unwrap();
    let mut a2 = body(1, [-1.0, 0.0, 0.0], 1.0);
    let mut b2 = body(2, [1.0, 0.0, 0.0], 0.0);
    let next = runtime
        .apply_friction_impulse(
            &mut a2,
            &mut b2,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            2,
            3,
            4,
        )
        .unwrap();
    assert_eq!(next.transaction_id(), FrictionTransactionId::new(1, 2, 3, 4));
}

#[test]
fn duplicate_solver_coordinates_fail_before_second_mechanical_mutation() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
    let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
    let _first = runtime
        .apply_friction_impulse(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            0,
            0,
            0,
        )
        .unwrap();
    let state_a = (a.linear_velocity, a.angular_velocity);
    let state_b = (b.linear_velocity, b.angular_velocity);
    let error = runtime
        .apply_friction_impulse(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            0,
            0,
            0,
        )
        .unwrap_err();
    assert!(matches!(
        error,
        RuntimeFrictionError::Application(FrictionApplicationError::DuplicateTransaction)
    ));
    assert_eq!((a.linear_velocity, a.angular_velocity), state_a);
    assert_eq!((b.linear_velocity, b.angular_velocity), state_b);
}

#[test]
fn friction_is_rejected_outside_open_tick_window() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
    let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
    assert!(matches!(
        runtime.apply_friction_impulse(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            0,
            0,
            0,
        ),
        Err(RuntimeFrictionError::Gate(RuntimeFrictionGateError::NoOpenTick))
    ));

    runtime.begin_next_tick().unwrap();
    let permit = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::IntentionallyAbsent)
        .unwrap();
    assert!(matches!(
        runtime.apply_friction_impulse(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            0,
            0,
            0,
        ),
        Err(RuntimeFrictionError::Gate(
            RuntimeFrictionGateError::FinalizeInProgress
        ))
    ));
    runtime.rollback_finalize(permit).unwrap();
}

#[test]
fn old_applied_token_cannot_be_reused_in_later_tick() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = body(1, [-1.0, 0.0, 0.0], 1.0);
    let mut b = body(2, [1.0, 0.0, 0.0], 0.0);
    let old = runtime
        .apply_friction_impulse(
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            0,
            0,
            0,
        )
        .unwrap();
    runtime.finalize_friction_diagnostic(&a, &b, &old).unwrap();
    let permit = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();
    runtime.commit_finalize_and_rotate(&permit).unwrap();
    runtime.begin_next_tick().unwrap();

    assert!(matches!(
        runtime.finalize_friction_diagnostic(&a, &b, &old),
        Err(RuntimeFrictionError::Gate(RuntimeFrictionGateError::WrongFixedTick {
            open_tick_id: 1,
            transaction_tick_id: 0,
        }))
    ));
}

#[test]
fn physical_promotion_uses_private_same_tick_journal() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = thermal_body(1, 1.0);
    let mut b = thermal_body(2, 0.0);
    let applied = runtime
        .apply_friction_impulse(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            0,
            0,
            0,
        )
        .unwrap();
    let mut ledger = EnergyTransferLedger::new();
    let promotion = runtime
        .promote_friction_loss(
            &mut a,
            &mut b,
            &applied,
            HeatPartition::equal(),
            &mut ledger,
        )
        .unwrap();
    assert_eq!(promotion.transaction_id.fixed_tick, 0);
    assert_eq!(promotion.heat.dissipated_joules, 0.25);
    assert!(runtime.friction_journal().is_complete_for_finalize());

    let permit = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();
    let receipt = runtime.commit_finalize_and_rotate(&permit).unwrap();
    assert_eq!(receipt.tick_id, 0);
    assert_eq!(
        receipt
            .friction_transactions
            .phase(applied.transaction_id()),
        Some(FrictionTransactionPhase::Promoted)
    );
}