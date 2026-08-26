// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Tier-B validation for constant external heat input and removal.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    EXTERNAL_HEAT_TRANSFER_KIND, EnergyForm, EnergyOwner, EnergyPort, EnergyTransferLedger,
    PhysicsWorld, ThermalBody, ThermalMaterial, ThermalState, WorldCommand, WorldSnapshot,
    apply_commands_audited,
};

const DT: f64 = 0.125;
const STEPS: usize = 80;
const MASS_KG: f64 = 5.0;
const CP: f64 = 800.0;

#[derive(Clone, Copy)]
struct Scenario {
    name: &'static str,
    initial_temperature_kelvin: f64,
    power_watts: f64,
    source_id: u64,
}

struct RunResult {
    final_temperature_kelvin: f64,
    net_external_joules: f64,
    closure_error_joules: f64,
    reservoir_reconciliation_error_joules: f64,
    snapshot: WorldSnapshot<3>,
    ledger: EnergyTransferLedger,
}

fn material() -> ThermalMaterial {
    ThermalMaterial::new(CP, 10.0, 0.5).unwrap()
}

fn build_world(initial_temperature_kelvin: f64) -> (PhysicsWorld<3>, symtropy_physics::BodyHandle) {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    world.body_mut(handle).unwrap().set_thermal(
        ThermalBody::new(
            material(),
            ThermalState::new(initial_temperature_kelvin).unwrap(),
            MASS_KG,
        )
        .unwrap(),
    );
    (world, handle)
}

fn run(scenario: Scenario) -> RunResult {
    let (mut world, handle) = build_world(scenario.initial_temperature_kelvin);
    let mut ledger = EnergyTransferLedger::new();
    let initial_energy = world
        .body(handle)
        .unwrap()
        .thermal_energy_joules(0.0)
        .unwrap();

    for _ in 0..STEPS {
        let command = WorldCommand::ApplyExternalHeat {
            body: handle,
            signed_joules: scenario.power_watts * DT,
            external_source_id: scenario.source_id,
        };
        apply_commands_audited(&mut world, &[command], &mut ledger).unwrap();
    }

    let final_body = world.body(handle).unwrap();
    let final_energy = final_body.thermal_energy_joules(0.0).unwrap();
    let final_temperature_kelvin = final_body.thermal.unwrap().state.temperature_kelvin;
    let audit = ledger
        .audit_internal_energy_complete(initial_energy, final_energy, true, true)
        .unwrap();

    let body_port = EnergyPort::new(EnergyOwner::Body(handle), EnergyForm::ThermalSensible);
    let reservoir_reconciliation_error_joules =
        (final_energy - initial_energy) - ledger.net_change_for(body_port);

    assert_eq!(ledger.len(), STEPS);
    for entry in ledger.entries() {
        assert_eq!(entry.kind, EXTERNAL_HEAT_TRANSFER_KIND);
        if scenario.power_watts > 0.0 {
            assert_eq!(entry.source.owner, EnergyOwner::External(scenario.source_id));
            assert_eq!(entry.destination.owner, EnergyOwner::Body(handle));
        } else {
            assert_eq!(entry.source.owner, EnergyOwner::Body(handle));
            assert_eq!(entry.destination.owner, EnergyOwner::External(scenario.source_id));
        }
    }

    RunResult {
        final_temperature_kelvin,
        net_external_joules: audit.net_external_joules,
        closure_error_joules: audit.closure_error_joules,
        reservoir_reconciliation_error_joules,
        snapshot: WorldSnapshot::capture(&world),
        ledger,
    }
}

fn main() {
    let scenarios = [
        Scenario {
            name: "heating",
            initial_temperature_kelvin: 290.0,
            power_watts: 2_500.0,
            source_id: 42,
        },
        Scenario {
            name: "cooling",
            initial_temperature_kelvin: 310.0,
            power_watts: -1_000.0,
            source_id: 43,
        },
    ];

    let elapsed = DT * STEPS as f64;
    let heat_capacity = material().heat_capacity(MASS_KG).unwrap();

    println!(
        "scenario,power_watts,elapsed_s,expected_temperature_k,observed_temperature_k,temperature_abs_error_k,expected_external_joules,ledger_external_joules,first_law_closure_joules,first_law_relative_closure,reservoir_reconciliation_error_joules,reservoir_reconciliation_relative_error,ledger_entries,deterministic_replay"
    );

    for scenario in scenarios {
        let expected_external_joules = scenario.power_watts * elapsed;
        let expected_temperature_kelvin = scenario.initial_temperature_kelvin
            + expected_external_joules / heat_capacity;

        let result_a = run(scenario);
        let result_b = run(scenario);
        let deterministic =
            result_a.snapshot == result_b.snapshot && result_a.ledger == result_b.ledger;

        let temperature_error =
            (result_a.final_temperature_kelvin - expected_temperature_kelvin).abs();
        let energy_scale = expected_external_joules.abs().max(1.0);
        let relative_closure = result_a.closure_error_joules.abs() / energy_scale;
        let relative_reconciliation =
            result_a.reservoir_reconciliation_error_joules.abs() / energy_scale;

        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            scenario.name,
            scenario.power_watts,
            elapsed,
            expected_temperature_kelvin,
            result_a.final_temperature_kelvin,
            temperature_error,
            expected_external_joules,
            result_a.net_external_joules,
            result_a.closure_error_joules,
            relative_closure,
            result_a.reservoir_reconciliation_error_joules,
            relative_reconciliation,
            result_a.ledger.len(),
            if deterministic { 1 } else { 0 }
        );

        assert!(temperature_error <= 1e-12);
        assert!((result_a.net_external_joules - expected_external_joules).abs() <= 1e-10);
        assert!(relative_closure <= 1e-12);
        assert!(relative_reconciliation <= 1e-12);
        assert!(deterministic);
    }
}
