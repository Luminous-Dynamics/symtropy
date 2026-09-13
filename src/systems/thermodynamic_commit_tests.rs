// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::thermodynamic::ThermodynamicHudState;
use super::thermodynamic_commit::{
    ThermodynamicCommitError, finalize_operational_tick_transaction,
    retry_operational_tick_transaction,
};
use super::thermodynamic_close::OperationalThermodynamicCloseError;
use super::thermodynamic_runtime::{
    RuntimeFrictionError, RuntimeFrictionGateError, ThermodynamicRuntimeError,
    ThermodynamicTransactionRuntime,
};
use super::thermodynamic_transaction::{
    ThermodynamicConsequenceStatus, ThermodynamicTickError,
};
use crate::resources::PhysicsWorldRes;
use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{BodyHandle, RigidBody};

fn registered_agent(energy: f64) -> (PhysicsWorldRes, BodyHandle) {
    let mut physics = PhysicsWorldRes::default();
    let handle = physics.world.add_sphere(Point::origin(), 1.0, 1.0);
    physics.consciousness.register(handle, energy, 10.0);
    (physics, handle)
}

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

#[test]
fn happy_path_binds_lifecycle_and_operational_close_receipts() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let (mut physics, handle) = registered_agent(100.0);
    let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
    entity.energy.tick_reset();
    let _ = entity.energy.consume(5.0);
    let mut hud = ThermodynamicHudState::default();

    let receipt = finalize_operational_tick_transaction(
        &mut runtime,
        &mut physics,
        &mut hud,
        &[handle],
        ThermodynamicConsequenceStatus::Executed,
    )
    .unwrap();

    assert_eq!(receipt.transaction.tick_id, 0);
    assert_eq!(
        receipt.transaction.consequence_status,
        ThermodynamicConsequenceStatus::Executed
    );
    assert_eq!(receipt.operational_close.sampled_consumed, 5.0);
    #[cfg(feature = "consciousness-runtime")]
    {
        assert_eq!(receipt.operational_close.legacy_tick_count_before, Some(0));
        assert_eq!(receipt.operational_close.legacy_tick_count_after, Some(1));
    }
    #[cfg(not(feature = "consciousness-runtime"))]
    {
        assert_eq!(receipt.operational_close.legacy_tick_count_before, None);
        assert_eq!(receipt.operational_close.legacy_tick_count_after, None);
    }
    assert_eq!(runtime.open_tick_id(), None);
    assert_eq!(runtime.next_tick_id(), Some(1));
    assert!(runtime.friction_journal().is_empty());
}

#[test]
fn close_rejection_preserves_bound_status_until_retry() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut physics = PhysicsWorldRes::default();
    let handle = physics.world.add_sphere(Point::origin(), 1.0, 1.0);
    let mut hud = ThermodynamicHudState::default();

    let retry = match finalize_operational_tick_transaction(
        &mut runtime,
        &mut physics,
        &mut hud,
        &[handle],
        ThermodynamicConsequenceStatus::Rejected,
    )
    .unwrap_err()
    {
        ThermodynamicCommitError::OperationalClose(retry) => retry,
        other => panic!("unexpected error: {other:?}"),
    };
    assert_eq!(
        retry.error(),
        OperationalThermodynamicCloseError::MissingOperationalEntity(handle)
    );
    assert_eq!(retry.tick_id(), 0);
    assert_eq!(runtime.open_tick_id(), Some(0));
    assert_eq!(runtime.next_tick_id(), Some(0));
    assert_eq!(hud.ticks_accumulated, 0);

    assert!(matches!(
        runtime.prepare_finalize(ThermodynamicConsequenceStatus::Executed),
        Err(ThermodynamicRuntimeError::Tick(
            ThermodynamicTickError::FinalizeInProgress { tick_id: 0 }
        ))
    ));
    let mut a = body(1, 1.0);
    let mut b = body(2, 0.0);
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

    physics.consciousness.register(handle, 100.0, 10.0);
    let receipt = retry_operational_tick_transaction(
        &mut runtime,
        &mut physics,
        &mut hud,
        &[handle],
        retry,
    )
    .unwrap();
    assert_eq!(receipt.transaction.tick_id, 0);
    assert_eq!(
        receipt.transaction.consequence_status,
        ThermodynamicConsequenceStatus::Rejected
    );
    assert_eq!(runtime.next_tick_id(), Some(1));
}

#[test]
fn unresolved_friction_blocks_before_operational_close_mutates() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = body(1, 1.0);
    let mut b = body(2, 0.0);
    let _pending = runtime
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

    let (mut physics, handle) = registered_agent(100.0);
    let mut hud = ThermodynamicHudState::default();
    let error = finalize_operational_tick_transaction(
        &mut runtime,
        &mut physics,
        &mut hud,
        &[handle],
        ThermodynamicConsequenceStatus::Executed,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ThermodynamicCommitError::Runtime(ThermodynamicRuntimeError::Tick(
            ThermodynamicTickError::PendingFrictionTransactions { count: 1 }
        ))
    ));
    assert_eq!(hud.ticks_accumulated, 0);
    assert_eq!(runtime.open_tick_id(), Some(0));
}

#[test]
fn rejected_consequence_closes_exactly_once_and_preserves_status() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let (mut physics, handle) = registered_agent(100.0);
    let mut hud = ThermodynamicHudState::default();

    let receipt = finalize_operational_tick_transaction(
        &mut runtime,
        &mut physics,
        &mut hud,
        &[handle],
        ThermodynamicConsequenceStatus::Rejected,
    )
    .unwrap();
    assert_eq!(
        receipt.transaction.consequence_status,
        ThermodynamicConsequenceStatus::Rejected
    );

    let second = finalize_operational_tick_transaction(
        &mut runtime,
        &mut physics,
        &mut hud,
        &[handle],
        ThermodynamicConsequenceStatus::Rejected,
    )
    .unwrap_err();
    assert!(matches!(
        second,
        ThermodynamicCommitError::Runtime(ThermodynamicRuntimeError::Tick(
            ThermodynamicTickError::NoOpenTick
        ))
    ));
}