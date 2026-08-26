// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    EnergyLedgerError, EnergyTransferLedger, PhysicsWorld, ThermalBody, ThermalMaterial,
    ThermalState,
};

fn thermal_world() -> (PhysicsWorld<3>, symtropy_physics::BodyHandle) {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    let thermal = ThermalBody::new(
        ThermalMaterial::new(500.0, 1.0, 0.5).unwrap(),
        ThermalState::new(300.0).unwrap(),
        1.0,
    )
    .unwrap();
    world.body_mut(handle).unwrap().set_thermal(thermal);
    (world, handle)
}

#[test]
fn complete_snapshot_window_can_close_strict_boundary_audit() {
    let (world, _) = thermal_world();
    let before = world.invariant_snapshot();
    let after = world.invariant_snapshot();
    let ledger = EnergyTransferLedger::new();

    assert!(before.has_complete_modeled_energy_accounting());
    assert!(after.has_complete_modeled_energy_accounting());

    let audit = ledger
        .audit_internal_energy_complete(
            before.modeled_total_energy,
            after.modeled_total_energy,
            before.has_complete_modeled_energy_accounting(),
            after.has_complete_modeled_energy_accounting(),
        )
        .unwrap();

    assert_eq!(audit.closure_error_joules, 0.0);
    assert!(audit.within_absolute_tolerance(0.0));
}

#[test]
fn invalid_attached_reservoir_blocks_strict_audit_before_false_pass() {
    let (mut world, handle) = thermal_world();
    let before = world.invariant_snapshot();

    // Thermal fields remain public for experimentation, so validation must be
    // repeated at accounting boundaries rather than trusting construction forever.
    world
        .body_mut(handle)
        .unwrap()
        .thermal
        .as_mut()
        .unwrap()
        .state
        .temperature_kelvin = -1.0;

    let after = world.invariant_snapshot();
    let ledger = EnergyTransferLedger::new();

    assert!(before.has_complete_modeled_energy_accounting());
    assert!(!after.has_complete_modeled_energy_accounting());
    assert_eq!(after.invalid_thermal_body_count, 1);

    assert_eq!(
        ledger.audit_internal_energy_complete(
            before.modeled_total_energy,
            after.modeled_total_energy,
            before.has_complete_modeled_energy_accounting(),
            after.has_complete_modeled_energy_accounting(),
        ),
        Err(EnergyLedgerError::IncompleteAccounting)
    );
}
