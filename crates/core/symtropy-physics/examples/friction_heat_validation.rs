// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Analytical validation of centered measured friction dissipation -> heat conversion.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, EnergyForm, EnergyOwner, EnergyPort, EnergyTransferKind, EnergyTransferLedger,
    HeatPartition, RigidBody, ThermalBody, ThermalMaterial, ThermalState,
    apply_friction_impulse_with_heat,
};

const MASS: f64 = 1.0;
const CP: f64 = 1_000.0;
const T0: f64 = 300.0;
const IMPULSE: f64 = 0.5;

fn material() -> ThermalMaterial {
    ThermalMaterial::new(CP, 1.0, 0.5).unwrap()
}

fn body(handle: usize, velocity_x: f64) -> RigidBody<3> {
    let mut body = RigidBody::dynamic_sphere(
        BodyHandle(handle),
        Point::origin(),
        0.5,
        MASS,
    );
    body.linear_velocity[0] = velocity_x;
    body.set_thermal(
        ThermalBody::new(
            material(),
            ThermalState::new(T0).unwrap(),
            MASS,
        )
        .unwrap(),
    );
    body
}

fn main() {
    let mut a = body(1, 1.0);
    let mut b = body(2, 0.0);
    let mut ledger = EnergyTransferLedger::new();

    let initial_kinetic_a = a.kinetic_energy();
    let initial_kinetic_b = b.kinetic_energy();
    let initial_thermal_a = a.thermal_energy_joules(0.0).unwrap();
    let initial_thermal_b = b.thermal_energy_joules(0.0).unwrap();
    let initial_kinetic = initial_kinetic_a + initial_kinetic_b;
    let initial_thermal = initial_thermal_a + initial_thermal_b;
    let initial_total = initial_kinetic + initial_thermal;

    let result = apply_friction_impulse_with_heat(
        &mut a,
        &mut b,
        &SVector::zeros(),
        &SVector::from([IMPULSE, 0.0, 0.0]),
        HeatPartition::equal(),
        &mut ledger,
    )
    .unwrap();

    // Closed-form centered, equal-mass impulse:
    // vA' = 1 - J/m = 0.5, vB' = J/m = 0.5.
    let expected_final_kinetic = 0.5 * MASS * 0.5_f64.powi(2)
        + 0.5 * MASS * 0.5_f64.powi(2);
    let expected_dissipation = initial_kinetic - expected_final_kinetic;
    let expected_heat_each = expected_dissipation * 0.5;
    let heat_capacity = material().heat_capacity(MASS).unwrap();
    let expected_temperature = T0 + expected_heat_each / heat_capacity;

    let final_kinetic_a = a.kinetic_energy();
    let final_kinetic_b = b.kinetic_energy();
    let final_thermal_a = a.thermal_energy_joules(0.0).unwrap();
    let final_thermal_b = b.thermal_energy_joules(0.0).unwrap();
    let final_kinetic = final_kinetic_a + final_kinetic_b;
    let final_thermal = final_thermal_a + final_thermal_b;
    let final_total = final_kinetic + final_thermal;
    let ta = a.thermal.unwrap().state.temperature_kelvin;
    let tb = b.thermal.unwrap().state.temperature_kelvin;

    let kinetic_a = EnergyPort::new(EnergyOwner::Body(BodyHandle(1)), EnergyForm::Kinetic);
    let kinetic_b = EnergyPort::new(EnergyOwner::Body(BodyHandle(2)), EnergyForm::Kinetic);
    let thermal_a = EnergyPort::new(
        EnergyOwner::Body(BodyHandle(1)),
        EnergyForm::ThermalSensible,
    );
    let thermal_b = EnergyPort::new(
        EnergyOwner::Body(BodyHandle(2)),
        EnergyForm::ThermalSensible,
    );

    let measured_deltas = [
        final_kinetic_a - initial_kinetic_a,
        final_kinetic_b - initial_kinetic_b,
        final_thermal_a - initial_thermal_a,
        final_thermal_b - initial_thermal_b,
    ];
    let ledger_deltas = [
        ledger.net_change_for(kinetic_a),
        ledger.net_change_for(kinetic_b),
        ledger.net_change_for(thermal_a),
        ledger.net_change_for(thermal_b),
    ];
    let reconciliation_residuals = std::array::from_fn::<_, 4, _>(|index| {
        measured_deltas[index] - ledger_deltas[index]
    });
    let max_reconciliation_residual = reconciliation_residuals
        .iter()
        .fold(0.0_f64, |max, residual| max.max(residual.abs()));

    let total_audit = ledger
        .audit_internal_energy_complete(initial_total, final_total, true, true)
        .unwrap();
    let relative_total_closure =
        total_audit.closure_error_joules.abs() / initial_total.abs().max(1.0);
    let relative_reconciliation = max_reconciliation_residual / initial_total.abs().max(1.0);

    println!("metric,value");
    println!("initial_kinetic_j,{initial_kinetic:.17e}");
    println!("expected_final_kinetic_j,{expected_final_kinetic:.17e}");
    println!("observed_final_kinetic_j,{final_kinetic:.17e}");
    println!("expected_dissipation_j,{expected_dissipation:.17e}");
    println!("observed_dissipation_j,{:.17e}", result.dissipated_joules);
    println!("temperature_a_k,{ta:.17e}");
    println!("temperature_b_k,{tb:.17e}");
    println!("expected_temperature_k,{expected_temperature:.17e}");
    println!("total_energy_closure_j,{:.17e}", total_audit.closure_error_joules);
    println!("total_energy_relative_closure,{relative_total_closure:.17e}");
    println!("ledger_external_j,{:.17e}", ledger.net_external_joules());
    println!("ledger_delta_kinetic_a_j,{:.17e}", ledger_deltas[0]);
    println!("ledger_delta_kinetic_b_j,{:.17e}", ledger_deltas[1]);
    println!("ledger_delta_thermal_a_j,{:.17e}", ledger_deltas[2]);
    println!("ledger_delta_thermal_b_j,{:.17e}", ledger_deltas[3]);
    println!("max_reservoir_reconciliation_residual_j,{max_reconciliation_residual:.17e}");
    println!("relative_reservoir_reconciliation_residual,{relative_reconciliation:.17e}");
    println!("ledger_entries,{}", ledger.len());

    assert!((final_kinetic - expected_final_kinetic).abs() <= 1e-12);
    assert!((result.dissipated_joules - expected_dissipation).abs() <= 1e-12);
    assert!((result.kinetic_change_a_joules - measured_deltas[0]).abs() <= 1e-12);
    assert!((result.kinetic_change_b_joules - measured_deltas[1]).abs() <= 1e-12);
    assert!((result.heat_to_a_joules - measured_deltas[2]).abs() <= 1e-9);
    assert!((result.heat_to_b_joules - measured_deltas[3]).abs() <= 1e-9);
    assert!((ta - expected_temperature).abs() <= 1e-12);
    assert!((tb - expected_temperature).abs() <= 1e-12);
    assert!(relative_total_closure <= 1e-12);
    assert_eq!(ledger.net_external_joules(), 0.0);
    assert!(relative_reconciliation <= 1e-12);
    assert_eq!(ledger.len(), 3);
    assert!(
        ledger
            .entries()
            .iter()
            .all(|entry| entry.kind == EnergyTransferKind::Friction)
    );
    assert!((ledger_deltas[0] + 0.375).abs() <= 1e-12);
    assert!((ledger_deltas[1] - 0.125).abs() <= 1e-12);
    assert!((ledger_deltas[2] - 0.125).abs() <= 1e-12);
    assert!((ledger_deltas[3] - 0.125).abs() <= 1e-12);
}
