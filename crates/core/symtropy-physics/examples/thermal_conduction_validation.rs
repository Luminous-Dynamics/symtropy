// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Analytical two-lump conduction convergence campaign.
//!
//! Run with:
//!   cargo run --release -p symtropy-physics --example thermal_conduction_validation
//!
//! The output is CSV so the same executable can feed CI evidence or an external
//! analysis script. The reference solution is the closed-form transient for two
//! finite lumped heat capacities coupled by constant conductance.

use symtropy_physics::{
    BodyHandle, EnergyTransferLedger, ThermalBody, ThermalMaterial, ThermalState,
    constant_cp_pair_entropy_audit, conductive_exchange_bodies_audited,
};

#[derive(Copy, Clone, Debug)]
struct CaseResult {
    dt_seconds: f64,
    steps: usize,
    max_temperature_error_kelvin: f64,
    energy_closure_error_joules: f64,
    relative_energy_closure_error: f64,
    entropy_delta_j_per_k: f64,
    ledger_entries: usize,
}

fn main() {
    let duration_seconds = 5.0;
    let conductance_w_per_k = 35.0;
    let initial_a_kelvin = 420.0;
    let initial_b_kelvin = 300.0;

    let material_a = ThermalMaterial::new(450.0, 10.0, 0.7).unwrap();
    let material_b = ThermalMaterial::new(1_000.0, 2.0, 0.4).unwrap();
    let mass_a_kg = 2.0;
    let mass_b_kg = 1.5;

    let capacity_a_j_per_k = material_a.specific_heat_capacity * mass_a_kg;
    let capacity_b_j_per_k = material_b.specific_heat_capacity * mass_b_kg;
    let total_capacity_j_per_k = capacity_a_j_per_k + capacity_b_j_per_k;

    let equilibrium_kelvin = (capacity_a_j_per_k * initial_a_kelvin
        + capacity_b_j_per_k * initial_b_kelvin)
        / total_capacity_j_per_k;
    let decay_rate_per_second = conductance_w_per_k
        * (capacity_a_j_per_k.recip() + capacity_b_j_per_k.recip());
    let initial_delta_kelvin = initial_a_kelvin - initial_b_kelvin;
    let analytical_delta_kelvin =
        initial_delta_kelvin * (-decay_rate_per_second * duration_seconds).exp();
    let analytical_a_kelvin = equilibrium_kelvin
        + capacity_b_j_per_k / total_capacity_j_per_k * analytical_delta_kelvin;
    let analytical_b_kelvin = equilibrium_kelvin
        - capacity_a_j_per_k / total_capacity_j_per_k * analytical_delta_kelvin;

    let timesteps = [0.5, 0.25, 0.125, 0.0625];
    let mut results = Vec::with_capacity(timesteps.len());

    for dt_seconds in timesteps {
        let steps = (duration_seconds / dt_seconds) as usize;
        let mut thermal_a = ThermalBody::new(
            material_a,
            ThermalState::new(initial_a_kelvin).unwrap(),
            mass_a_kg,
        )
        .unwrap();
        let mut thermal_b = ThermalBody::new(
            material_b,
            ThermalState::new(initial_b_kelvin).unwrap(),
            mass_b_kg,
        )
        .unwrap();
        let before_a = thermal_a;
        let before_b = thermal_b;
        let initial_energy_joules = thermal_a.sensible_energy_joules(0.0).unwrap()
            + thermal_b.sensible_energy_joules(0.0).unwrap();
        let mut ledger = EnergyTransferLedger::new();

        for _ in 0..steps {
            conductive_exchange_bodies_audited(
                BodyHandle(0),
                &mut thermal_a,
                BodyHandle(1),
                &mut thermal_b,
                conductance_w_per_k,
                dt_seconds,
                &mut ledger,
            )
            .unwrap();
        }

        let final_energy_joules = thermal_a.sensible_energy_joules(0.0).unwrap()
            + thermal_b.sensible_energy_joules(0.0).unwrap();
        let energy_audit = ledger
            .audit_internal_energy(initial_energy_joules, final_energy_joules)
            .unwrap();
        let relative_energy_closure_error = energy_audit.closure_error_joules.abs()
            / initial_energy_joules.abs().max(1.0);
        let entropy_audit =
            constant_cp_pair_entropy_audit(before_a, thermal_a, before_b, thermal_b).unwrap();
        let max_temperature_error_kelvin = (thermal_a.state.temperature_kelvin
            - analytical_a_kelvin)
            .abs()
            .max((thermal_b.state.temperature_kelvin - analytical_b_kelvin).abs());

        assert!(
            relative_energy_closure_error <= 1e-12,
            "first-law closure failed at dt={dt_seconds}: relative error={relative_energy_closure_error:e}"
        );
        assert!(
            entropy_audit.is_second_law_consistent(1e-12),
            "second-law audit failed at dt={dt_seconds}: delta_S={} J/K",
            entropy_audit.total_delta_j_per_k
        );

        results.push(CaseResult {
            dt_seconds,
            steps,
            max_temperature_error_kelvin,
            energy_closure_error_joules: energy_audit.closure_error_joules,
            relative_energy_closure_error,
            entropy_delta_j_per_k: entropy_audit.total_delta_j_per_k,
            ledger_entries: ledger.len(),
        });
    }

    for pair in results.windows(2) {
        assert!(
            pair[1].max_temperature_error_kelvin < pair[0].max_temperature_error_kelvin,
            "transient error did not improve under timestep refinement: dt {} -> {}",
            pair[0].dt_seconds,
            pair[1].dt_seconds
        );
    }

    println!(
        "dt_seconds,steps,max_temperature_error_kelvin,energy_closure_error_joules,relative_energy_closure_error,entropy_delta_j_per_k,ledger_entries,observed_order"
    );
    for (index, result) in results.iter().enumerate() {
        let observed_order = if index == 0 {
            f64::NAN
        } else {
            let previous = results[index - 1].max_temperature_error_kelvin;
            (previous / result.max_temperature_error_kelvin).ln() / 2.0_f64.ln()
        };
        println!(
            "{},{},{},{},{},{},{},{}",
            result.dt_seconds,
            result.steps,
            result.max_temperature_error_kelvin,
            result.energy_closure_error_joules,
            result.relative_energy_closure_error,
            result.entropy_delta_j_per_k,
            result.ledger_entries,
            observed_order
        );
    }
}
