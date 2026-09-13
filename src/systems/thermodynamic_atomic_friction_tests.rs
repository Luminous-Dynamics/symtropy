// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, FrictionApplicationError, FrictionDiagnosticReason, FrictionEvidenceError,
    FrictionPromotionError, FrictionSolverCoordinates, FrictionTransactionId,
    FrictionTransactionPhase, HeatPartition, RigidBody, ThermalBody, ThermalMaterial,
    ThermalState,
};

use super::thermodynamic_runtime::{
    RuntimeFrictionError, TerminalFrictionOutcome, ThermodynamicTransactionRuntime,
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

fn thermal_body(handle: usize, position: [f64; 3], velocity_x: f64) -> RigidBody<3> {
    let mut body = body(handle, position, velocity_x);
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
fn centered_loss_reaches_promoted_terminal_state_before_return() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = thermal_body(1, [0.0, 0.0, 0.0], 1.0);
    let mut b = thermal_body(2, [0.0, 0.0, 0.0], 0.0);

    let receipt = runtime
        .execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 0, 0),
            HeatPartition::equal(),
        )
        .unwrap();

    assert_eq!(receipt.transaction_id, FrictionTransactionId::new(0, 0, 0, 0));
    let TerminalFrictionOutcome::Promoted(promotion) = receipt.outcome else {
        panic!("centered measured loss must promote before returning");
    };
    assert_eq!(promotion.transaction_id, receipt.transaction_id);
    assert_eq!(promotion.heat.dissipated_joules, 0.25);
    assert_eq!(
        runtime.friction_journal().phase(receipt.transaction_id),
        Some(FrictionTransactionPhase::Promoted)
    );
    assert_eq!(runtime.physical_energy_ledger().len(), 3);
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
    assert!(runtime.friction_journal().is_complete_for_finalize());
}

#[test]
fn off_center_transaction_terminalizes_diagnostic_without_physical_ledger_entry() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = body(1, [-1.0, 0.0, 0.0], 1.0);
    let mut b = body(2, [1.0, 0.0, 0.0], 0.0);

    let receipt = runtime
        .execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            FrictionSolverCoordinates::new(0, 4, 2),
            HeatPartition::equal(),
        )
        .unwrap();

    assert_eq!(
        receipt.outcome,
        TerminalFrictionOutcome::Diagnostic(FrictionDiagnosticReason::OffCenterUnqualified)
    );
    assert_eq!(
        runtime.friction_journal().phase(receipt.transaction_id),
        Some(FrictionTransactionPhase::DiagnosticOnly(
            FrictionDiagnosticReason::OffCenterUnqualified
        ))
    );
    assert!(runtime.physical_energy_ledger().is_empty());
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}

#[test]
fn centered_solver_injection_is_terminal_diagnostic_not_heat() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
    let mut b = body(2, [0.0, 0.0, 0.0], 0.0);

    let receipt = runtime
        .execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([2.0, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 5, 0),
            HeatPartition::equal(),
        )
        .unwrap();

    assert_eq!(
        receipt.outcome,
        TerminalFrictionOutcome::Diagnostic(FrictionDiagnosticReason::SolverInjection)
    );
    assert!(runtime.physical_energy_ledger().is_empty());
    assert_eq!(
        runtime.friction_journal().phase(receipt.transaction_id),
        Some(FrictionTransactionPhase::DiagnosticOnly(
            FrictionDiagnosticReason::SolverInjection
        ))
    );
}

#[test]
fn missing_thermal_state_rewinds_mechanics_and_all_private_authority() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = body(1, [0.0, 0.0, 0.0], 1.0);
    let mut b = body(2, [0.0, 0.0, 0.0], 0.0);
    let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
    let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);
    let id = FrictionTransactionId::new(0, 0, 6, 0);

    assert!(matches!(
        runtime.execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 6, 0),
            HeatPartition::equal(),
        ),
        Err(RuntimeFrictionError::Promotion(
            FrictionPromotionError::MissingThermalState
        ))
    ));

    assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
    assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
    assert_eq!(runtime.friction_journal().phase(id), None);
    assert!(runtime.friction_journal().is_empty());
    assert!(runtime.physical_energy_ledger().is_empty());
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}

#[test]
fn failed_later_transaction_preserves_prior_terminal_evidence_exactly() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();

    let mut good_a = thermal_body(1, [0.0, 0.0, 0.0], 1.0);
    let mut good_b = thermal_body(2, [0.0, 0.0, 0.0], 0.0);
    let good = runtime
        .execute_terminal_friction_impulse_at(
            &mut good_a,
            &mut good_b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 7, 0),
            HeatPartition::equal(),
        )
        .unwrap();
    assert!(matches!(good.outcome, TerminalFrictionOutcome::Promoted(_)));

    let prior_journal = runtime.friction_journal().clone();
    let prior_ledger = runtime.physical_energy_ledger().clone();

    let mut bad_a = body(3, [0.0, 0.0, 0.0], 1.0);
    let mut bad_b = body(4, [0.0, 0.0, 0.0], 0.0);
    let before_a = (bad_a.linear_velocity, bad_a.angular_velocity, bad_a.thermal);
    let before_b = (bad_b.linear_velocity, bad_b.angular_velocity, bad_b.thermal);
    let failed_id = FrictionTransactionId::new(0, 0, 8, 0);

    assert!(runtime
        .execute_terminal_friction_impulse_at(
            &mut bad_a,
            &mut bad_b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 8, 0),
            HeatPartition::equal(),
        )
        .is_err());

    assert_eq!((bad_a.linear_velocity, bad_a.angular_velocity, bad_a.thermal), before_a);
    assert_eq!((bad_b.linear_velocity, bad_b.angular_velocity, bad_b.thermal), before_b);
    assert_eq!(runtime.friction_journal(), &prior_journal);
    assert_eq!(runtime.physical_energy_ledger(), &prior_ledger);
    assert_eq!(runtime.friction_journal().phase(failed_id), None);
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}

#[test]
fn invalid_partition_rewinds_centered_mechanics_even_with_valid_thermal_state() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = thermal_body(1, [0.0, 0.0, 0.0], 1.0);
    let mut b = thermal_body(2, [0.0, 0.0, 0.0], 0.0);
    let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
    let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);
    let id = FrictionTransactionId::new(0, 0, 9, 0);

    assert!(matches!(
        runtime.execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 9, 0),
            HeatPartition { fraction_to_a: f64::NAN },
        ),
        Err(RuntimeFrictionError::Promotion(
            FrictionPromotionError::InvalidHeatPartition
        ))
    ));

    assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
    assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
    assert_eq!(runtime.friction_journal().phase(id), None);
    assert!(runtime.physical_energy_ledger().is_empty());
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}

#[test]
fn duplicate_terminal_identity_is_rejected_before_second_mutation() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = thermal_body(1, [0.0, 0.0, 0.0], 1.0);
    let mut b = thermal_body(2, [0.0, 0.0, 0.0], 0.0);
    let coords = FrictionSolverCoordinates::new(0, 10, 0);

    let first = runtime
        .execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            coords,
            HeatPartition::equal(),
        )
        .unwrap();
    assert!(matches!(first.outcome, TerminalFrictionOutcome::Promoted(_)));

    let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
    let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);
    let before_journal = runtime.friction_journal().clone();
    let before_ledger = runtime.physical_energy_ledger().clone();

    assert!(matches!(
        runtime.execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            coords,
            HeatPartition::equal(),
        ),
        Err(RuntimeFrictionError::Application(
            FrictionApplicationError::DuplicateTransaction
        ))
    ));

    assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
    assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
    assert_eq!(runtime.friction_journal(), &before_journal);
    assert_eq!(runtime.physical_energy_ledger(), &before_ledger);
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}

#[test]
fn invalid_impulse_fails_before_authority_or_body_state_changes() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let mut a = thermal_body(1, [0.0, 0.0, 0.0], 1.0);
    let mut b = thermal_body(2, [0.0, 0.0, 0.0], 0.0);
    let before_a = (a.linear_velocity, a.angular_velocity, a.thermal);
    let before_b = (b.linear_velocity, b.angular_velocity, b.thermal);

    assert!(matches!(
        runtime.execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([f64::NAN, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 11, 0),
            HeatPartition::equal(),
        ),
        Err(RuntimeFrictionError::Application(
            FrictionApplicationError::Evidence(FrictionEvidenceError::NonFiniteImpulse)
        ))
    ));

    assert_eq!((a.linear_velocity, a.angular_velocity, a.thermal), before_a);
    assert_eq!((b.linear_velocity, b.angular_velocity, b.thermal), before_b);
    assert!(runtime.friction_journal().is_empty());
    assert!(runtime.physical_energy_ledger().is_empty());
    assert_eq!(runtime.pending_friction_reservation_count(), 0);
}
