// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Tier-B validation for constant external heat input.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    EnergyTransferLedger, PhysicsWorld, ThermalBody, ThermalMaterial, ThermalState, WorldCommand,
    WorldSnapshot, apply_commands_audited,
};

const POWER_W: f64 = 2_500.0;
const DT: f64 = 0.125;
const STEPS: usize = 80;
const MASS_KG: f64 = 5.0;
const CP: f64 = 800.0;
const T0_K: f64 = 290.0;
const SOURCE_ID: u64 = 42;

fn build_world() -> (PhysicsWorld<3>, symtropy_physics::BodyHandle) {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    world.body_mut(handle).unwrap().set_thermal(
        ThermalBody::new(
            ThermalMaterial::new(CP, 10.0, 0.5).unwrap(),
            ThermalState::new(T0_K).unwrap(),
            MASS_KG,
        )
        .unwrap(),
    );
    (world, handle)
}

fn run() -> (f64, f64, f64, WorldSnapshot<3>, EnergyTransferLedger) {
    let (mut world, handle) = build_world();
    let mut ledger = EnergyTransferLedger::new();
    let initial_energy = world
        .body(handle)
        .unwrap()
        .thermal_energy_joules(0.0)
        .unwrap();

    for _ in 0..STEPS {
        let command = WorldCommand::ApplyExternalHeat {
            body: handle,
            signed_joules: POWER_W * DT,
            external_source_id: SOURCE_ID,
        };
        apply_commands_audited(&mut world, &[command], &mut ledger).unwrap();
    }

    let final_body = world.body(handle).unwrap();
    let final_energy = final_body.thermal_energy_joules(0.0).unwrap();
    let final_temperature = final_body.thermal.unwrap().state.temperature_kelvin;
    let audit = ledger
        .audit_internal_energy(initial_energy, final_energy)
        .unwrap();

    (
        final_temperature,
        audit.net_external_joules,
        audit.closure_error_joules,
        WorldSnapshot::capture(&world),
        ledger,
    )
}

fn main() {
    let elapsed = DT * STEPS as f64;
    let expected_energy = POWER_W * elapsed;
    let expected_temperature = T0_K + expected_energy / (MASS_KG * CP);

    let (temperature, external_energy, closure, snapshot_a, ledger_a) = run();
    let (_, _, _, snapshot_b, ledger_b) = run();

    let temperature_error = (temperature - expected_temperature).abs();
    let relative_closure = closure.abs() / expected_energy.max(1.0);
    let deterministic = snapshot_a == snapshot_b && ledger_a == ledger_b;

    println!("metric,value");
    println!("elapsed_s,{elapsed:.17e}");
    println!("expected_temperature_k,{expected_temperature:.17e}");
    println!("observed_temperature_k,{temperature:.17e}");
    println!("temperature_abs_error_k,{temperature_error:.17e}");
    println!("expected_external_joules,{expected_energy:.17e}");
    println!("ledger_external_joules,{external_energy:.17e}");
    println!("first_law_closure_joules,{closure:.17e}");
    println!("first_law_relative_closure,{relative_closure:.17e}");
    println!("ledger_entries,{}", ledger_a.len());
    println!("deterministic_replay,{}", if deterministic { 1 } else { 0 });

    assert!(temperature_error <= 1e-12);
    assert!((external_energy - expected_energy).abs() <= 1e-10);
    assert!(relative_closure <= 1e-12);
    assert!(deterministic);
}
