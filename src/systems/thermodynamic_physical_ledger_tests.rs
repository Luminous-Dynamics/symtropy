// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, EnergyTransferKind, FrictionPromotionError, FrictionTransactionPhase,
    HeatPartition, RigidBody, ThermalBody, ThermalMaterial, ThermalState,
};

use super::thermodynamic_runtime::{RuntimeFrictionError, RuntimeFrictionGateError, ThermodynamicTransactionRuntime};
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;

fn thermal_body(handle: usize, velocity_x: f64) -> RigidBody<3> {
    let mut body = RigidBody::<3>::dynamic_sphere(
        BodyHandle(handle),
        Point::origin(),
        0.5,
        1.0,
    );
    body.linear_velocity[0] = velocity_x;
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

fn promote_centered(
    runtime: &mut ThermodynamicTransactionRuntime,
    tick_local_sequence: u32,
) -> symtropy_physics::FrictionTransactionId {
    let mut a = thermal_body(1, 1.0);
    let mut b = thermal_body(2, 0.0);
    let applied = runtime
        .apply_friction_impulse(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            0,
            tick_local_sequence,
            0,
        )
        .unwrap();
    let id = applied.transaction_id();
    runtime
        .promote_friction_loss_owned(
            &mut a,
            &mut b,
            &applied,
            HeatPartition::equal(),
        )
        .unwrap();
    id
}

#[test]
fn owned_promotion_is_bound_exactly_once_into_tick_receipt() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let id = promote_centered(&mut runtime, 0);

    assert_eq!(
        runtime.friction_journal().phase(id),
        Some(FrictionTransactionPhase::Promoted)
    );
    assert_eq!(runtime.physical_energy_ledger().len(), 3);

    let permit = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();
    let receipt = runtime.commit_finalize_and_rotate(&permit).unwrap();

    assert_eq!(receipt.tick_id, 0);
    assert_eq!(receipt.physical_energy_transfers.len(), 3);
    assert!(
        receipt
            .physical_energy_transfers
            .iter()
            .all(|entry| entry.kind == EnergyTransferKind::Friction)
    );
    assert_eq!(
        receipt
            .physical_energy_transfers
            .iter()
            .map(|entry| entry.sequence)
            .collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    assert_eq!(runtime.last_runtime_receipt(), Some(&receipt));
    assert_eq!(runtime.physical_energy_ledger().len(), 3);
}

#[test]
fn consecutive_ticks_receive_disjoint_physical_transfer_segments() {
    let mut runtime = ThermodynamicTransactionRuntime::new();

    runtime.begin_next_tick().unwrap();
    promote_centered(&mut runtime, 0);
    let permit0 = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();
    let receipt0 = runtime.commit_finalize_and_rotate(&permit0).unwrap();
    assert_eq!(receipt0.physical_energy_transfers.len(), 3);

    runtime.begin_next_tick().unwrap();
    let permit1 = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::IntentionallyAbsent)
        .unwrap();
    let receipt1 = runtime.commit_finalize_and_rotate(&permit1).unwrap();
    assert_eq!(receipt1.tick_id, 1);
    assert!(receipt1.physical_energy_transfers.is_empty());

    runtime.begin_next_tick().unwrap();
    promote_centered(&mut runtime, 1);
    let permit2 = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();
    let receipt2 = runtime.commit_finalize_and_rotate(&permit2).unwrap();
    assert_eq!(receipt2.tick_id, 2);
    assert_eq!(
        receipt2
            .physical_energy_transfers
            .iter()
            .map(|entry| entry.sequence)
            .collect::<Vec<_>>(),
        vec![3, 4, 5]
    );
    assert_eq!(runtime.physical_energy_ledger().len(), 6);
}

#[test]
fn failed_owned_promotion_leaves_private_ledger_unchanged() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();

    let mut a = RigidBody::<3>::dynamic_sphere(
        BodyHandle(1),
        Point::origin(),
        0.5,
        1.0,
    );
    let mut b = RigidBody::<3>::dynamic_sphere(
        BodyHandle(2),
        Point::origin(),
        0.5,
        1.0,
    );
    a.linear_velocity[0] = 1.0;
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

    assert!(matches!(
        runtime.promote_friction_loss_owned(
            &mut a,
            &mut b,
            &applied,
            HeatPartition::equal(),
        ),
        Err(RuntimeFrictionError::Promotion(
            FrictionPromotionError::MissingThermalState
        ))
    ));
    assert!(runtime.physical_energy_ledger().is_empty());
    assert_eq!(
        runtime.friction_journal().phase(applied.transaction_id()),
        Some(FrictionTransactionPhase::Applied)
    );
}

#[test]
fn prepared_finalize_blocks_late_owned_physical_promotion() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let permit = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::IntentionallyAbsent)
        .unwrap();

    let mut a = thermal_body(1, 1.0);
    let mut b = thermal_body(2, 0.0);
    // No friction application can be admitted while finalization is reserved,
    // so the private ledger cannot change after the tick segment is frozen.
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
    assert!(runtime.physical_energy_ledger().is_empty());
    runtime.rollback_finalize(permit).unwrap();
}
