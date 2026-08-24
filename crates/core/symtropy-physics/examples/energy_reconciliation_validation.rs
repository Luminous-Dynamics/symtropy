// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Validate state-versus-ledger reservoir reconciliation.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, EnergyStateSnapshot, EnergyTransferLedger, PhysicsWorld, ThermalBody,
    ThermalMaterial, ThermalState, exchange_external_heat_audited,
};

fn world() -> (PhysicsWorld<3>, BodyHandle) {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    world.body_mut(handle).unwrap().set_thermal(
        ThermalBody::new(
            ThermalMaterial::new(1_000.0, 1.0, 0.5).unwrap(),
            ThermalState::new(300.0).unwrap(),
            1.0,
        )
        .unwrap(),
    );
    (world, handle)
}

fn audited_case() -> (f64, f64, usize) {
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
    )
}

fn untracked_case() -> (f64, f64) {
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

fn main() {
    let (audited_residual, audited_total, untracked_ports) = audited_case();
    let (untracked_residual, untracked_total) = untracked_case();

    println!("metric,value");
    println!("audited_max_residual_j,{audited_residual:.17e}");
    println!("audited_total_closure_j,{audited_total:.17e}");
    println!("audited_untracked_ports,{untracked_ports}");
    println!("untracked_max_residual_j,{untracked_residual:.17e}");
    println!("untracked_total_closure_j,{untracked_total:.17e}");

    assert!(audited_residual <= 1e-10);
    assert!(audited_total.abs() <= 1e-10);
    assert_eq!(untracked_ports, 0);
    assert!((untracked_residual - 1_000.0).abs() <= 1e-10);
    assert!((untracked_total - 1_000.0).abs() <= 1e-10);
}
