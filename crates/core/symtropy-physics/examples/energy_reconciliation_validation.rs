// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Validate state-versus-ledger reservoir reconciliation.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, EnergyStateSnapshot, EnergyTransferLedger, PhysicsWorld, ThermalBody,
    ThermalMaterial, ThermalState, exchange_external_heat_audited,
};

fn thermal_body(temp_k: f64) -> ThermalBody {
    ThermalBody::new(
        ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
        ThermalState::new(temp_k).unwrap(),
        1.0,
    )
    .unwrap()
}

fn world() -> (PhysicsWorld<3>, BodyHandle) {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    world
        .body_mut(handle)
        .unwrap()
        .set_thermal(thermal_body(300.0));
    (world, handle)
}

fn audited_case() -> (f64, f64, usize, usize) {
    let (mut world, handle) = world();
    let before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
    let mut ledger = EnergyTransferLedger::new();
    exchange_external_heat_audited(
        handle,
        world.body_mut(handle).unwrap(),
        1_000.0,
        11,
        &mut ledger,
    )
    .unwrap();
    let after = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
    let audit = before.reconcile(&after, &ledger).unwrap();
    assert!(audit.fully_reconciled(1e-10));
    (
        audit.max_abs_residual_joules(),
        audit.total_closure_error_joules,
        audit.untracked_ledger_ports.len(),
        audit.reservoir_presence_changes.len(),
    )
}

fn unjournaled_heat_case() -> (f64, f64) {
    let (mut world, handle) = world();
    let before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
    world.body_mut(handle).unwrap().add_heat_joules(1_000.0).unwrap();
    let after = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
    let audit = before
        .reconcile(&after, &EnergyTransferLedger::new())
        .unwrap();
    assert!(!audit.fully_reconciled(1e-10));
    (audit.max_abs_residual_joules(), audit.total_closure_error_joules)
}

fn zero_energy_reservoir_appearance_case() -> (f64, usize, bool) {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    let before = EnergyStateSnapshot::capture(&world, 0.0).unwrap();

    world.body_mut(handle).unwrap().set_thermal(thermal_body(0.0));
    let after = EnergyStateSnapshot::capture(&world, 0.0).unwrap();
    let audit = before
        .reconcile(&after, &EnergyTransferLedger::new())
        .unwrap();

    (
        audit.total_closure_error_joules,
        audit.reservoir_presence_changes.len(),
        audit.fully_reconciled(0.0),
    )
}

fn main() {
    let (audited_residual, audited_total, untracked_ports, presence_changes) = audited_case();
    let (unjournaled_residual, unjournaled_total) = unjournaled_heat_case();
    let (appearance_total, appearance_changes, appearance_reconciled) =
        zero_energy_reservoir_appearance_case();

    println!("metric,value");
    println!("audited_max_residual_j,{audited_residual:.17e}");
    println!("audited_total_closure_j,{audited_total:.17e}");
    println!("audited_untracked_ports,{untracked_ports}");
    println!("audited_presence_changes,{presence_changes}");
    println!("unjournaled_heat_max_residual_j,{unjournaled_residual:.17e}");
    println!("unjournaled_heat_total_closure_j,{unjournaled_total:.17e}");
    println!("zero_energy_appearance_total_closure_j,{appearance_total:.17e}");
    println!("zero_energy_appearance_presence_changes,{appearance_changes}");
    println!(
        "zero_energy_appearance_fully_reconciled,{}",
        if appearance_reconciled { 1 } else { 0 }
    );

    assert!(audited_residual <= 1e-10);
    assert!(audited_total.abs() <= 1e-10);
    assert_eq!(untracked_ports, 0);
    assert_eq!(presence_changes, 0);

    assert!((unjournaled_residual - 1_000.0).abs() <= 1e-10);
    assert!((unjournaled_total - 1_000.0).abs() <= 1e-10);

    // The new reservoir contains exactly 0 J at the chosen reference, so a
    // purely numeric boundary audit closes. Reconciliation must still fail
    // because reservoir identity/lifetime changed without provenance.
    assert_eq!(appearance_total, 0.0);
    assert_eq!(appearance_changes, 1);
    assert!(!appearance_reconciled);
}
