// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Black-box controls for sticky runtime-level thermodynamic authority poison.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, FrictionDiagnosticReason, FrictionSolverCoordinates,
    FrictionTransactionId, FrictionTransactionPhase, HeatPartition, RigidBody,
};

use super::thermodynamic::ThermodynamicHudState;
use super::thermodynamic_commit::{
    ThermodynamicCommitError, finalize_operational_tick_transaction,
};
use super::thermodynamic_runtime::{
    RuntimeFrictionError, RuntimeFrictionGateError, ThermodynamicRuntimeError,
    ThermodynamicTransactionRuntime,
};
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;
use crate::resources::PhysicsWorldRes;

fn off_center_pair() -> (RigidBody<3>, RigidBody<3>) {
    let mut a = RigidBody::<3>::dynamic_sphere(
        BodyHandle(1),
        Point::new([-1.0, 0.0, 0.0]),
        0.5,
        1.0,
    );
    let b = RigidBody::<3>::dynamic_sphere(
        BodyHandle(2),
        Point::new([1.0, 0.0, 0.0]),
        0.5,
        1.0,
    );
    a.linear_velocity[0] = 1.0;
    (a, b)
}

#[test]
fn poison_is_sticky_and_blocks_new_forward_authority() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();

    assert!(runtime.poison_authority());
    assert!(!runtime.poison_authority());
    assert!(runtime.is_authority_poisoned());
    assert_eq!(runtime.open_tick_id(), Some(0));

    assert_eq!(
        runtime.prepare_finalize(ThermodynamicConsequenceStatus::Executed),
        Err(ThermodynamicRuntimeError::AuthorityPoisoned)
    );
    assert_eq!(
        runtime.begin_next_tick(),
        Err(ThermodynamicRuntimeError::AuthorityPoisoned)
    );

    let error = runtime
        .reserve_friction_impulse_at(
            BodyHandle(1),
            BodyHandle(2),
            &SVector::<f64, 3>::zeros(),
            &SVector::from([0.1, 0.0, 0.0]),
            FrictionSolverCoordinates::new(0, 0, 0),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        RuntimeFrictionError::Gate(RuntimeFrictionGateError::AuthorityPoisoned)
    ));
}

#[test]
fn terminal_friction_journal_cannot_be_committed_after_poison() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let (mut a, mut b) = off_center_pair();

    let receipt = runtime
        .execute_terminal_friction_impulse_at(
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            FrictionSolverCoordinates::new(0, 2, 0),
            HeatPartition::equal(),
        )
        .unwrap();
    assert_eq!(
        receipt.outcome,
        super::thermodynamic_runtime::TerminalFrictionOutcome::Diagnostic(
            FrictionDiagnosticReason::OffCenterUnqualified
        )
    );

    let id = FrictionTransactionId::new(0, 0, 2, 0);
    assert_eq!(
        runtime.friction_journal().phase(id),
        Some(FrictionTransactionPhase::DiagnosticOnly(
            FrictionDiagnosticReason::OffCenterUnqualified
        ))
    );
    assert!(runtime.friction_journal().is_complete_for_finalize());

    runtime.poison_authority();
    assert_eq!(
        runtime.prepare_finalize(ThermodynamicConsequenceStatus::Executed),
        Err(ThermodynamicRuntimeError::AuthorityPoisoned)
    );

    // Poison does not destroy evidence needed for diagnosis/recovery.
    assert_eq!(runtime.open_tick_id(), Some(0));
    assert_eq!(
        runtime.friction_journal().phase(id),
        Some(FrictionTransactionPhase::DiagnosticOnly(
            FrictionDiagnosticReason::OffCenterUnqualified
        ))
    );
    assert!(runtime.physical_energy_ledger().is_empty());
}

#[test]
fn poison_after_prepare_blocks_validate_and_commit_but_allows_rollback() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    let permit = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::Executed)
        .unwrap();

    runtime.poison_authority();
    assert_eq!(
        runtime.validate_prepared_finalize(&permit),
        Err(ThermodynamicRuntimeError::AuthorityPoisoned)
    );
    assert_eq!(
        runtime.commit_finalize_and_rotate(&permit),
        Err(ThermodynamicRuntimeError::AuthorityPoisoned)
    );

    runtime.rollback_finalize(permit).unwrap();
    assert_eq!(runtime.open_tick_id(), Some(0));
    assert!(runtime.is_authority_poisoned());
    assert_eq!(
        runtime.prepare_finalize(ThermodynamicConsequenceStatus::Rejected),
        Err(ThermodynamicRuntimeError::AuthorityPoisoned)
    );
}

#[test]
fn operational_finalizer_propagates_poison_without_hud_mutation_or_panic() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    runtime.poison_authority();

    let mut physics = PhysicsWorldRes::default();
    let handle = physics.world.add_sphere(Point::origin(), 1.0, 1.0);
    physics.consciousness.register(handle, 100.0, 10.0);
    let mut hud = ThermodynamicHudState::default();
    let hud_before = hud.ticks_accumulated;

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
        ThermodynamicCommitError::Runtime(ThermodynamicRuntimeError::AuthorityPoisoned)
    ));
    assert_eq!(hud.ticks_accumulated, hud_before);
    assert_eq!(runtime.open_tick_id(), Some(0));
    assert!(runtime.is_authority_poisoned());
}

#[test]
fn successful_unpoisoned_runtime_path_remains_finalizable() {
    let mut runtime = ThermodynamicTransactionRuntime::new();
    runtime.begin_next_tick().unwrap();
    assert!(!runtime.is_authority_poisoned());

    let permit = runtime
        .prepare_finalize(ThermodynamicConsequenceStatus::IntentionallyAbsent)
        .unwrap();
    runtime.validate_prepared_finalize(&permit).unwrap();
    let receipt = runtime.commit_finalize_and_rotate(&permit).unwrap();

    assert_eq!(receipt.tick_id, 0);
    assert_eq!(receipt.consequence_status, ThermodynamicConsequenceStatus::IntentionallyAbsent);
    assert_eq!(runtime.open_tick_id(), None);
    assert_eq!(runtime.next_tick_id(), Some(1));
    assert!(!runtime.is_authority_poisoned());
}
