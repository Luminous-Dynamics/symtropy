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
    BodyHandle, EnergyForm, EnergyOwner, EnergyPort, EnergyTransferLedger, ThermalBody,
    ThermalMaterial, ThermalState, constant_cp_pair_entropy_audit,
    conductive_exchange_bodies_audited,
};

#[derive(Copy, Clone, Debug)]
struct CaseResult {
    dt_seconds: f64,
    steps: usize,
    max_temperature_error_kelvin: f64,
    energy_closure_error_joules: f64,
    relative_energy_closure_error: f64,
    max_reservoir_reconciliation_error_joules: f64,
    relative_reservoir_reconciliation_error: f64,
    min_step_entropy_delta_j_per_k: f64,
    entropy_delta_j_per_k: f64,
    ledger_entries: usize,
    equilibrium_limited_steps: usize,
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

    // Use the same finite-derived-capacity validity contract as the runtime, but
    // keep the transient reference analytically independent of the numerical
    // conduction step itself.
    let capacity_a_j_per_k = material_a.heat_capacity(mass_a_kg).unwrap();
    let capacity_b_j_per_k = material_b.heat_capacity(mass_b_kg).unwrap();
    let capacity_scale = capacity_a_j_per_k.max(capacity_b_j_per_k);
    let scaled_capacity_a = capacity_a_j_per_k / capacity_scale;
    let scaled_capacity_b = capacity_b_j_per_k / capacity_scale;
    let scaled_capacity_total = scaled_capacity_a + scaled_capacity_b;

    let equilibrium_kelvin = (scaled_capacity_a * initial_a_kelvin
        + scaled_capacity_b * initial_b_kelvin)
        / scaled_capacity_total;
    let decay_rate_per_second = conductance_w_per_k / capacity_a_j_per_k
        + conductance_w_per_k / capacity_b_j_per_k;
    let initial_delta_kelvin = initial_a_kelvin - initial_b_kelvin;
    let analytical_delta_kelvin =
        initial_delta_kelvin * (-decay_rate_per_second * duration_seconds).exp();
    let analytical_a_kelvin = equilibrium_kelvin
        + scaled_capacity_b / scaled_capacity_total * analytical_delta_kelvin;
    let analytical_b_kelvin = equilibrium_kelvin
        - scaled_capacity_a / scaled_capacity_total * analytical_delta_kelvin;

    let timesteps = [0.5, 0.25, 0.125, 0.0625];
    let mut results = Vec::with_capacity(timesteps.len());

    for dt_seconds in timesteps {
        let steps_exact = duration_seconds / dt_seconds;
        let steps = steps_exact.round() as usize;
        assert!(
            ((steps as f64) * dt_seconds - duration_seconds).abs() <= f64::EPSILON,
            "campaign duration is not an integer number of steps at dt={dt_seconds}"
        );

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
        let initial_a_energy_joules = thermal_a.sensible_energy_joules(0.0).unwrap();
        let initial_b_energy_joules = thermal_b.sensible_energy_joules(0.0).unwrap();
        let initial_energy_joules = initial_a_energy_joules + initial_b_energy_joules;
        assert!(initial_energy_joules.is_finite());

        let mut ledger = EnergyTransferLedger::new();
        let mut min_step_entropy_delta_j_per_k = f64::INFINITY;
        let mut equilibrium_limited_steps = 0usize;

        for step in 0..steps {
            let step_before_a = thermal_a;
            let step_before_b = thermal_b;
            let exchange = conductive_exchange_bodies_audited(
                BodyHandle(0),
                &mut thermal_a,
                BodyHandle(1),
                &mut thermal_b,
                conductance_w_per_k,
                dt_seconds,
                &mut ledger,
            )
            .unwrap();

            if exchange.equilibrium_limited {
                equilibrium_limited_steps += 1;
            }

            let step_entropy = constant_cp_pair_entropy_audit(
                step_before_a,
                thermal_a,
                step_before_b,
                thermal_b,
            )
            .unwrap();
            assert!(
                step_entropy.is_second_law_consistent(1e-12),
                "second-law step audit failed at dt={dt_seconds}, step={step}: delta_S={} J/K",
                step_entropy.total_delta_j_per_k
            );
            min_step_entropy_delta_j_per_k =
                min_step_entropy_delta_j_per_k.min(step_entropy.total_delta_j_per_k);
        }

        let final_a_energy_joules = thermal_a.sensible_energy_joules(0.0).unwrap();
        let final_b_energy_joules = thermal_b.sensible_energy_joules(0.0).unwrap();
        let final_energy_joules = final_a_energy_joules + final_b_energy_joules;
        assert!(final_energy_joules.is_finite());

        // Both declared reservoirs were successfully evaluated at both endpoints,
        // so this interval is eligible for the strict complete-accounting audit.
        let energy_audit = ledger
            .audit_internal_energy_complete(
                initial_energy_joules,
                final_energy_joules,
                true,
                true,
            )
            .unwrap();
        let relative_energy_closure_error = energy_audit.closure_error_joules.abs()
            / initial_energy_joules.abs().max(1.0);

        // Total closure can hide an equal-and-opposite bookkeeping mistake. Reconcile
        // each body reservoir independently against its ledger net change.
        let port_a = EnergyPort::new(
            EnergyOwner::Body(BodyHandle(0)),
            EnergyForm::ThermalSensible,
        );
        let port_b = EnergyPort::new(
            EnergyOwner::Body(BodyHandle(1)),
            EnergyForm::ThermalSensible,
        );
        let residual_a_joules =
            (final_a_energy_joules - initial_a_energy_joules) - ledger.net_change_for(port_a);
        let residual_b_joules =
            (final_b_energy_joules - initial_b_energy_joules) - ledger.net_change_for(port_b);
        let max_reservoir_reconciliation_error_joules =
            residual_a_joules.abs().max(residual_b_joules.abs());
        let relative_reservoir_reconciliation_error =
            max_reservoir_reconciliation_error_joules / initial_energy_joules.abs().max(1.0);

        let entropy_audit =
            constant_cp_pair_entropy_audit(before_a, thermal_a, before_b, thermal_b).unwrap();
        let max_temperature_error_kelvin = (thermal_a.state.temperature_kelvin
            - analytical_a_kelvin)
            .abs()
            .max((thermal_b.state.temperature_kelvin - analytical_b_kelvin).abs());

        assert_eq!(ledger.net_external_joules(), 0.0);
        assert!(
            relative_energy_closure_error <= 1e-12,
            "first-law closure failed at dt={dt_seconds}: relative error={relative_energy_closure_error:e}"
        );
        assert!(
            relative_reservoir_reconciliation_error <= 1e-12,
            "state/ledger reservoir reconciliation failed at dt={dt_seconds}: relative error={relative_reservoir_reconciliation_error:e}"
        );
        assert!(
            entropy_audit.is_second_law_consistent(1e-12),
            "second-law interval audit failed at dt={dt_seconds}: delta_S={} J/K",
            entropy_audit.total_delta_j_per_k
        );
        assert_eq!(
            equilibrium_limited_steps, 0,
            "convergence case entered the equilibrium limiter at dt={dt_seconds}; this campaign must measure the explicit transient, not the safety clamp"
        );

        results.push(CaseResult {
            dt_seconds,
            steps,
            max_temperature_error_kelvin,
            energy_closure_error_joules: energy_audit.closure_error_joules,
            relative_energy_closure_error,
            max_reservoir_reconciliation_error_joules,
            relative_reservoir_reconciliation_error,
            min_step_entropy_delta_j_per_k,
            entropy_delta_j_per_k: entropy_audit.total_delta_j_per_k,
            ledger_entries: ledger.len(),
            equilibrium_limited_steps,
        });
    }

    for pair in results.windows(2) {
        assert!(
            pair[1].max_temperature_error_kelvin < pair[0].max_temperature_error_kelvin,
            "transient error did not improve under timestep refinement: dt {} -> {}",
            pair[0].dt_seconds,
            pair[1].dt_seconds
        );
        let observed_order = (pair[0].max_temperature_error_kelvin
            / pair[1].max_temperature_error_kelvin)
            .ln()
            / 2.0_f64.ln();
        assert!(
            (0.8..=1.2).contains(&observed_order),
            "expected first-order temporal convergence, got order={observed_order} for dt {} -> {}",
            pair[0].dt_seconds,
            pair[1].dt_seconds
        );
    }

    println!(
        "dt_seconds,steps,max_temperature_error_kelvin,energy_closure_error_joules,relative_energy_closure_error,max_reservoir_reconciliation_error_joules,relative_reservoir_reconciliation_error,min_step_entropy_delta_j_per_k,entropy_delta_j_per_k,ledger_entries,equilibrium_limited_steps,observed_order"
    );
    for (index, result) in results.iter().enumerate() {
        let observed_order = if index == 0 {
            f64::NAN
        } else {
            let previous = results[index - 1].max_temperature_error_kelvin;
            (previous / result.max_temperature_error_kelvin).ln() / 2.0_f64.ln()
        };
        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            result.dt_seconds,
            result.steps,
            result.max_temperature_error_kelvin,
            result.energy_closure_error_joules,
            result.relative_energy_closure_error,
            result.max_reservoir_reconciliation_error_joules,
            result.relative_reservoir_reconciliation_error,
            result.min_step_entropy_delta_j_per_k,
            result.entropy_delta_j_per_k,
            result.ledger_entries,
            result.equilibrium_limited_steps,
            observed_order
        );
    }
}
