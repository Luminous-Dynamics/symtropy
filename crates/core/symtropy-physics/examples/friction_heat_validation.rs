// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Analytical validation of measured friction dissipation -> heat conversion.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, EnergyForm, EnergyOwner, EnergyPort, EnergyTransferLedger, HeatPartition, RigidBody,
    ThermalBody, ThermalMaterial, ThermalState, apply_friction_impulse_with_heat,
};

const MASS: f64 = 1.0;
const CP: f64 = 1_000.0;
const T0: f64 = 300.0;
const IMPULSE: f64 = 0.5;

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
            ThermalMaterial::new(CP, 1.0, 0.5).unwrap(),
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

    let initial_kinetic = a.kinetic_energy() + b.kinetic_energy();
    let initial_thermal = a.thermal_energy_joules(0.0).unwrap()
        + b.thermal_energy_joules(0.0).unwrap();
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
    let expected_temperature = T0 + expected_heat_each / (MASS * CP);

    let final_kinetic = a.kinetic_energy() + b.kinetic_energy();
    let final_thermal = a.thermal_energy_joules(0.0).unwrap()
        + b.thermal_energy_joules(0.0).unwrap();
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

    println!("metric,value");
    println!("initial_kinetic_j,{initial_kinetic:.17e}");
    println!("expected_final_kinetic_j,{expected_final_kinetic:.17e}");
    println!("observed_final_kinetic_j,{final_kinetic:.17e}");
    println!("expected_dissipation_j,{expected_dissipation:.17e}");
    println!("observed_dissipation_j,{:.17e}", result.dissipated_joules);
    println!("temperature_a_k,{ta:.17e}");
    println!("temperature_b_k,{tb:.17e}");
    println!("expected_temperature_k,{expected_temperature:.17e}");
    println!("total_energy_closure_j,{:.17e}", final_total - initial_total);
    println!("ledger_external_j,{:.17e}", ledger.net_external_joules());
    println!("ledger_delta_kinetic_a_j,{:.17e}", ledger.net_change_for(kinetic_a));
    println!("ledger_delta_kinetic_b_j,{:.17e}", ledger.net_change_for(kinetic_b));
    println!("ledger_delta_thermal_a_j,{:.17e}", ledger.net_change_for(thermal_a));
    println!("ledger_delta_thermal_b_j,{:.17e}", ledger.net_change_for(thermal_b));

    assert!((final_kinetic - expected_final_kinetic).abs() <= 1e-12);
    assert!((result.dissipated_joules - expected_dissipation).abs() <= 1e-12);
    assert!((ta - expected_temperature).abs() <= 1e-12);
    assert!((tb - expected_temperature).abs() <= 1e-12);
    assert!((final_total - initial_total).abs() <= 1e-9);
    assert_eq!(ledger.net_external_joules(), 0.0);
    assert!((ledger.net_change_for(kinetic_a) + 0.375).abs() <= 1e-12);
    assert!((ledger.net_change_for(kinetic_b) - 0.125).abs() <= 1e-12);
    assert!((ledger.net_change_for(thermal_a) - 0.125).abs() <= 1e-12);
    assert!((ledger.net_change_for(thermal_b) - 0.125).abs() <= 1e-12);
}
