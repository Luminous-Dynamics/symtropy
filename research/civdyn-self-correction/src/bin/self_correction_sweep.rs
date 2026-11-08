// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Emit the fixed six-run synthetic self-correction portfolio as CSV.

use civdyn_self_correction::{
    ControllerKind, DelayedExternalityOutcome, SensorDriftOutcome,
    run_delayed_externality_portfolio, run_sensor_drift_portfolio,
};

const CSV_COLUMNS: usize = 20;

fn number(value: f64) -> String {
    format!("{value:.6}")
}

fn optional_number(value: Option<f64>) -> String {
    value.map(number).unwrap_or_default()
}

fn optional_ticks(value: Option<usize>) -> String {
    value.map(|ticks| ticks.to_string()).unwrap_or_default()
}

fn common(
    scenario: &str,
    controller: ControllerKind,
    minimum_household_coverage: f64,
    household_unmet_m3: f64,
    industry_unmet_m3: Option<f64>,
    unnecessary_dispatch_m3: f64,
    reserve_before_m3: f64,
    reserve_after_growth_m3: Option<f64>,
    reserve_after_recovery_m3: Option<f64>,
    reserve_after_scenario_m3: f64,
    maintenance_energy_kwh: f64,
    counterfactual_industry_on_m3: Option<f64>,
    counterfactual_industry_off_m3: Option<f64>,
    counterfactual_no_maintenance_m3: Option<f64>,
    mechanism_identified: bool,
    correction_latency_ticks: Option<usize>,
    trace_valid: bool,
    trace_event_count: usize,
    maximum_water_residual_m3: f64,
    maximum_energy_residual_kwh: f64,
) -> Vec<String> {
    vec![
        scenario.to_string(),
        controller.name().to_string(),
        number(minimum_household_coverage),
        number(household_unmet_m3),
        optional_number(industry_unmet_m3),
        number(unnecessary_dispatch_m3),
        number(reserve_before_m3),
        optional_number(reserve_after_growth_m3),
        optional_number(reserve_after_recovery_m3),
        number(reserve_after_scenario_m3),
        number(maintenance_energy_kwh),
        optional_number(counterfactual_industry_on_m3),
        optional_number(counterfactual_industry_off_m3),
        optional_number(counterfactual_no_maintenance_m3),
        mechanism_identified.to_string(),
        optional_ticks(correction_latency_ticks),
        trace_valid.to_string(),
        trace_event_count.to_string(),
        number(maximum_water_residual_m3),
        number(maximum_energy_residual_kwh),
    ]
}

fn sensor_row(outcome: &SensorDriftOutcome) -> Vec<String> {
    common(
        "sensor_drift_sham",
        outcome.controller,
        outcome.minimum_true_household_coverage,
        outcome.household_unmet_m3,
        None,
        outcome.unnecessary_dispatch_m3,
        outcome.reserve_before_m3,
        None,
        None,
        outcome.reserve_after_m3,
        0.0,
        None,
        None,
        None,
        outcome.mechanism_identified,
        outcome.correction_latency_ticks,
        outcome.trace_valid,
        outcome.trace.len(),
        outcome.maximum_water_residual_m3,
        outcome.maximum_energy_residual_kwh,
    )
}

fn delayed_row(outcome: &DelayedExternalityOutcome) -> Vec<String> {
    common(
        "delayed_externality",
        outcome.controller,
        outcome.minimum_household_coverage,
        outcome.household_unmet_m3,
        Some(outcome.industry_unmet_m3),
        0.0,
        outcome.reserve_before_m3,
        Some(outcome.reserve_after_growth_m3),
        Some(outcome.reserve_after_recovery_m3),
        outcome.reserve_after_repeated_shock_m3,
        outcome.maintenance_energy_kwh,
        Some(outcome.counterfactual_household_unmet_industry_on_m3),
        Some(outcome.counterfactual_household_unmet_industry_off_m3),
        Some(outcome.counterfactual_without_maintenance_household_unmet_m3),
        outcome.mechanism_identified,
        outcome.correction_latency_ticks,
        outcome.trace_valid,
        outcome.trace.len(),
        outcome.maximum_water_residual_m3,
        outcome.maximum_energy_residual_kwh,
    )
}

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS, "self-correction CSV schema changed");
    println!("{}", fields.join(","));
}

fn main() {
    println!(
        "scenario,controller,minimum_household_coverage,household_unmet_m3,industry_unmet_m3,unnecessary_dispatch_m3,reserve_before_m3,reserve_after_growth_m3,reserve_after_recovery_m3,reserve_after_scenario_m3,maintenance_energy_kwh,counterfactual_industry_on_household_unmet_m3,counterfactual_industry_off_household_unmet_m3,counterfactual_without_maintenance_household_unmet_m3,mechanism_identified,correction_latency_ticks,full_trace_valid,trace_event_count,maximum_water_residual_m3,maximum_energy_residual_kwh"
    );

    for outcome in run_sensor_drift_portfolio()
        .expect("sensor drift portfolio must validate")
    {
        emit(sensor_row(&outcome));
    }
    for outcome in run_delayed_externality_portfolio()
        .expect("delayed externality portfolio must validate")
    {
        emit(delayed_row(&outcome));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_csv_row_has_the_fixed_twenty_column_contract() {
        for outcome in run_sensor_drift_portfolio().expect("sensor portfolio") {
            assert_eq!(sensor_row(&outcome).len(), CSV_COLUMNS);
        }
        for outcome in run_delayed_externality_portfolio().expect("delayed portfolio") {
            assert_eq!(delayed_row(&outcome).len(), CSV_COLUMNS);
        }
    }

    #[test]
    fn portfolio_has_three_controllers_for_each_of_two_scenarios() {
        assert_eq!(
            run_sensor_drift_portfolio().expect("sensor portfolio").len(),
            ControllerKind::ALL.len()
        );
        assert_eq!(
            run_delayed_externality_portfolio()
                .expect("delayed portfolio")
                .len(),
            ControllerKind::ALL.len()
        );
    }
}
