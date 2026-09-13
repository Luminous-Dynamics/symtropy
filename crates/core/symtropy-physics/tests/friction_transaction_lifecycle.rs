// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, EnergyForm, EnergyOwner, EnergyPort, EnergyTransferKind, EnergyTransferLedger,
    FrictionApplicationError, FrictionDiagnosticFinalizeError, FrictionDiagnosticReason,
    FrictionPromotionError, FrictionTransactionId, FrictionTransactionJournal,
    FrictionTransactionPhase, HeatPartition, RigidBody, ThermalBody, ThermalMaterial,
    ThermalState, apply_friction_impulse_once, apply_friction_impulse_with_heat,
    finalize_friction_diagnostic, promote_applied_friction_loss_to_heat,
};

fn body(handle: usize, position: [f64; 3], velocity_x: f64, thermal: bool) -> RigidBody<3> {
    let mut body = RigidBody::dynamic_sphere(
        BodyHandle(handle),
        Point::new(position),
        0.5,
        1.0,
    );
    body.linear_velocity[0] = velocity_x;
    if thermal {
        body.set_thermal(
            ThermalBody::new(
                ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
                ThermalState::new(300.0).unwrap(),
                1.0,
            )
            .unwrap(),
        );
    }
    body
}

fn seeded_ledger() -> EnergyTransferLedger {
    let mut ledger = EnergyTransferLedger::new();
    ledger
        .record(
            EnergyPort::new(EnergyOwner::External(77), EnergyForm::ThermalSensible),
            EnergyPort::new(
                EnergyOwner::Body(BodyHandle(99)),
                EnergyForm::ThermalSensible,
            ),
            3.0,
            EnergyTransferKind::ExternalHeat,
        )
        .unwrap();
    ledger
}

#[test]
fn public_centered_path_matches_existing_reference_with_seeded_history() {
    let id = FrictionTransactionId::new(100, 2, 7, 1);
    let mut staged_a = body(1, [0.0, 0.0, 0.0], 1.0, true);
    let mut staged_b = body(2, [0.0, 0.0, 0.0], 0.0, true);
    let mut journal = FrictionTransactionJournal::new();
    let applied = apply_friction_impulse_once(
        &mut staged_a,
        &mut staged_b,
        &SVector::zeros(),
        &SVector::from([0.5, 0.0, 0.0]),
        id,
        &mut journal,
    )
    .unwrap();
    let mut staged_ledger = seeded_ledger();
    let seed = staged_ledger.entries()[0].clone();

    let promoted = promote_applied_friction_loss_to_heat(
        &mut staged_a,
        &mut staged_b,
        &applied,
        HeatPartition::equal(),
        &mut staged_ledger,
        &mut journal,
    )
    .unwrap();

    let mut reference_a = body(1, [0.0, 0.0, 0.0], 1.0, true);
    let mut reference_b = body(2, [0.0, 0.0, 0.0], 0.0, true);
    let mut reference_ledger = seeded_ledger();
    let reference = apply_friction_impulse_with_heat(
        &mut reference_a,
        &mut reference_b,
        &SVector::zeros(),
        &SVector::from([0.5, 0.0, 0.0]),
        HeatPartition::equal(),
        &mut reference_ledger,
    )
    .unwrap();

    assert_eq!(promoted.heat, reference);
    assert_eq!(staged_a.linear_velocity, reference_a.linear_velocity);
    assert_eq!(staged_b.linear_velocity, reference_b.linear_velocity);
    assert_eq!(staged_a.angular_velocity, reference_a.angular_velocity);
    assert_eq!(staged_b.angular_velocity, reference_b.angular_velocity);
    assert_eq!(staged_a.thermal, reference_a.thermal);
    assert_eq!(staged_b.thermal, reference_b.thermal);
    assert_eq!(staged_ledger.entries()[0], seed);
    assert_eq!(staged_ledger, reference_ledger);
    assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Promoted));
    assert!(journal.is_complete_for_finalize());
}

#[test]
fn duplicate_application_is_rejected_before_mechanics() {
    let id = FrictionTransactionId::new(101, 0, 0, 0);
    let mut a = body(1, [0.0, 0.0, 0.0], 1.0, true);
    let mut b = body(2, [0.0, 0.0, 0.0], 0.0, true);
    let mut journal = FrictionTransactionJournal::new();
    let _ = apply_friction_impulse_once(
        &mut a,
        &mut b,
        &SVector::zeros(),
        &SVector::from([0.5, 0.0, 0.0]),
        id,
        &mut journal,
    )
    .unwrap();
    let before_a = (a.linear_velocity, a.angular_velocity);
    let before_b = (b.linear_velocity, b.angular_velocity);

    assert_eq!(
        apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            id,
            &mut journal,
        ),
        Err(FrictionApplicationError::DuplicateTransaction)
    );
    assert_eq!((a.linear_velocity, a.angular_velocity), before_a);
    assert_eq!((b.linear_velocity, b.angular_velocity), before_b);
}

#[test]
fn off_center_path_has_typed_nonphysical_terminal_outcome() {
    let id = FrictionTransactionId::new(102, 4, 3, 2);
    let mut a = body(1, [-1.0, 0.0, 0.0], 1.0, true);
    let mut b = body(2, [1.0, 0.0, 0.0], 0.0, true);
    let mut journal = FrictionTransactionJournal::new();
    let applied = apply_friction_impulse_once(
        &mut a,
        &mut b,
        &SVector::from([0.0, 0.5, 0.0]),
        &SVector::from([0.0, -0.1, 0.0]),
        id,
        &mut journal,
    )
    .unwrap();

    assert_eq!(
        finalize_friction_diagnostic(&a, &b, &applied, &mut journal).unwrap(),
        FrictionDiagnosticReason::OffCenterUnqualified
    );
    assert_eq!(
        journal.phase(id),
        Some(FrictionTransactionPhase::DiagnosticOnly(
            FrictionDiagnosticReason::OffCenterUnqualified
        ))
    );
    assert!(journal.is_complete_for_finalize());
}

#[test]
fn centered_loss_with_missing_thermal_remains_pending_and_cannot_be_downgraded() {
    let id = FrictionTransactionId::new(103, 0, 0, 0);
    let mut a = body(1, [0.0, 0.0, 0.0], 1.0, false);
    let mut b = body(2, [0.0, 0.0, 0.0], 0.0, true);
    let mut journal = FrictionTransactionJournal::new();
    let applied = apply_friction_impulse_once(
        &mut a,
        &mut b,
        &SVector::zeros(),
        &SVector::from([0.5, 0.0, 0.0]),
        id,
        &mut journal,
    )
    .unwrap();
    let mut ledger = EnergyTransferLedger::new();

    assert_eq!(
        promote_applied_friction_loss_to_heat(
            &mut a,
            &mut b,
            &applied,
            HeatPartition::equal(),
            &mut ledger,
            &mut journal,
        ),
        Err(FrictionPromotionError::MissingThermalState)
    );
    assert_eq!(
        finalize_friction_diagnostic(&a, &b, &applied, &mut journal),
        Err(FrictionDiagnosticFinalizeError::RequiresPhysicalPromotion)
    );
    assert_eq!(journal.phase(id), Some(FrictionTransactionPhase::Applied));
    assert!(!journal.is_complete_for_finalize());
    assert!(ledger.is_empty());
}

#[test]
fn thermal_failure_does_not_commit_ledger_or_lifecycle() {
    let id = FrictionTransactionId::new(104, 0, 0, 0);
    let mut a = body(1, [0.0, 0.0, 0.0], 1.0, true);
    let mut b = body(2, [0.0, 0.0, 0.0], 0.0, true);
    let mut journal = FrictionTransactionJournal::new();
    let applied = apply_friction_impulse_once(
        &mut a,
        &mut b,
        &SVector::zeros(),
        &SVector::from([0.5, 0.0, 0.0]),
        id,
        &mut journal,
    )
    .unwrap();

    a.thermal.as_mut().unwrap().material.specific_heat_capacity = f64::MIN_POSITIVE;
    a.thermal.unwrap().validate().unwrap();
    let before_a = a.thermal;
    let before_b = b.thermal;
    let before_mechanical_a = (a.linear_velocity, a.angular_velocity);
    let before_mechanical_b = (b.linear_velocity, b.angular_velocity);
    let mut ledger = seeded_ledger();
    let before_ledger = ledger.clone();
    let before_journal = journal.clone();

    let result = promote_applied_friction_loss_to_heat(
        &mut a,
        &mut b,
        &applied,
        HeatPartition::equal(),
        &mut ledger,
        &mut journal,
    );
    assert!(matches!(result, Err(FrictionPromotionError::Thermal(_))));
    assert_eq!(a.thermal, before_a);
    assert_eq!(b.thermal, before_b);
    assert_eq!(ledger, before_ledger);
    assert_eq!(journal, before_journal);
    assert_eq!((a.linear_velocity, a.angular_velocity), before_mechanical_a);
    assert_eq!((b.linear_velocity, b.angular_velocity), before_mechanical_b);
}