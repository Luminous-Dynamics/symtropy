// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, FrictionPromotionError, FrictionSolverCoordinates, HeatPartition, RigidBody,
    ThermalBody, ThermalMaterial, ThermalState,
};

use super::thermodynamic_runtime::{RuntimeFrictionError, ThermodynamicTransactionRuntime};

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

fn thermal_body(handle: usize, velocity_x: f64) -> RigidBody<3> {
    let mut body = body(handle, velocity_x);
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
fn terminalization_failure_preserves_unrelated_pending_reservation_exactly() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();

    let outstanding = runtime
        .reserve_friction_impulse_at(
            BodyHandle(40),
            BodyHandle(41),
            &SVector::<f64, 3>::zeros(),
            &SVector::from([0.1, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 90, 0),
        )
        .unwrap();
    let pending_before = runtime.pending_friction_reservation_ids();

    let mut a = body(1, 1.0);
    let mut b = body(2, 0.0);
    let before_a = (a.linear_velocity, a.angular_velocity);
    let before_b = (b.linear_velocity, b.angular_velocity);

    assert!(matches!(
        runtime.execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 91, 0),
            HeatPartition::equal(),
        ),
        Err(RuntimeFrictionError::Promotion(
            FrictionPromotionError::MissingThermalState
        ))
    ));

    assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
    assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
    assert_eq!(runtime.pending_friction_reservation_ids(), pending_before);
    assert!(runtime.friction_journal().is_empty());
    assert!(runtime.physical_energy_ledger().is_empty());

    runtime.cancel_friction_reservation(outstanding).unwrap();
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}

#[test]
fn failed_transaction_after_large_prior_history_preserves_history_exactly() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();

    // Build enough prior successful evidence that a whole-history rollback design
    // would have work proportional to unrelated history. Each request uses fresh
    // bodies but the same runtime journal/physical ledger.
    for contact_sequence in 0..32_u32 {
        let base = 100 + contact_sequence as usize * 2;
        let mut a = thermal_body(base, 1.0);
        let mut b = thermal_body(base + 1, 0.0);
        runtime
            .execute_terminal_friction_impulse_at(
                &mut a,
                &mut b,
                &SVector::zeros(),
                &SVector::from([0.5, 0.0, 0.0]),
                FrictionSolverCoordinates::new(0, contact_sequence, 0),
                HeatPartition::equal(),
            )
            .unwrap();
    }

    assert_eq!(runtime.friction_journal().len(), 32);
    assert_eq!(runtime.physical_energy_ledger().len(), 96);
    let prior_journal = runtime.friction_journal().clone();
    let prior_ledger = runtime.physical_energy_ledger().clone();

    let mut bad_a = body(500, 1.0);
    let mut bad_b = body(501, 0.0);
    let before_a = (bad_a.linear_velocity, bad_a.angular_velocity);
    let before_b = (bad_b.linear_velocity, bad_b.angular_velocity);

    assert!(matches!(
        runtime.execute_terminal_friction_impulse_at(
            &mut bad_a,
            &mut bad_b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 100, 0),
            HeatPartition::equal(),
        ),
        Err(RuntimeFrictionError::Promotion(
            FrictionPromotionError::MissingThermalState
        ))
    ));

    assert_eq!((bad_a.linear_velocity, bad_a.angular_velocity), before_a);
    assert_eq!((bad_b.linear_velocity, bad_b.angular_velocity), before_b);
    assert_eq!(runtime.friction_journal(), &prior_journal);
    assert_eq!(runtime.physical_energy_ledger(), &prior_ledger);
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}

#[test]
fn successful_exact_rollback_returns_original_terminalization_error() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = body(1, 1.0);
    let mut b = body(2, 0.0);

    let error = runtime
        .execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 0, 0),
            HeatPartition::equal(),
        )
        .unwrap_err();

    assert!(matches!(
        &error,
        RuntimeFrictionError::Promotion(FrictionPromotionError::MissingThermalState)
    ));
    assert!(!matches!(&error, RuntimeFrictionError::Rollback { .. }));
    assert!(runtime.friction_journal().is_empty());
    assert!(runtime.physical_energy_ledger().is_empty());
}
