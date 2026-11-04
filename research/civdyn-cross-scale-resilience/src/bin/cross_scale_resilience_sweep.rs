// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic deterministic CSS resilience portfolio.
//! This executable imports no real geographic, utility or household data.

use civdyn_cross_scale_resilience::{
    InterventionPolicy, RunOutcome, ShockKind, ShockPhase, TickOutcome, run_reference_portfolio,
};

const CSV_COLUMNS: usize = 25;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS, "CSV row width must remain stable");
    println!("{}", fields.join(","));
}

fn summary_row(outcome: &RunOutcome) {
    emit(vec![
        "summary".into(),
        outcome.scenario_id.clone(),
        outcome.shock_kind.name().into(),
        outcome.policy.name().into(),
        String::new(),
        String::new(),
        fmt(outcome.total_unmet_water_m3),
        fmt(outcome.total_unmet_energy_kwh),
        fmt(outcome.water_deficit_area_household_steps),
        fmt(outcome.energy_deficit_area_household_steps),
        fmt(outcome.minimum_water_coverage),
        fmt(outcome.minimum_energy_coverage),
        outcome
            .recovery_after_ticks
            .map_or_else(String::new, |value| value.to_string()),
        fmt(outcome.final_basin_stock.water_m3),
        fmt(outcome.final_basin_stock.energy_kwh),
        fmt(outcome.maximum_water_conservation_residual_m3),
        fmt(outcome.maximum_energy_conservation_residual_kwh),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]);
}

fn tick_row(
    outcome: &RunOutcome,
    tick: &TickOutcome,
) {
    emit(vec![
        "tick".into(),
        outcome.scenario_id.clone(),
        outcome.shock_kind.name().into(),
        outcome.policy.name().into(),
        tick.tick.to_string(),
        phase_name(tick.phase).into(),
        fmt(tick.unmet_water_m3),
        fmt(tick.unmet_energy_kwh),
        String::new(),
        String::new(),
        fmt(tick.minimum_water_coverage),
        fmt(tick.minimum_energy_coverage),
        String::new(),
        fmt(tick.basin_closing_water_m3),
        fmt(tick.basin_closing_energy_kwh),
        fmt(tick.maximum_water_conservation_residual_m3),
        fmt(tick.maximum_energy_conservation_residual_kwh),
        fmt(tick.household_water_coverage[0]),
        fmt(tick.household_water_coverage[1]),
        fmt(tick.household_water_coverage[2]),
        fmt(tick.household_water_coverage[3]),
        fmt(tick.household_energy_coverage[0]),
        fmt(tick.household_energy_coverage[1]),
        fmt(tick.household_energy_coverage[2]),
        fmt(tick.household_energy_coverage[3]),
    ]);
}

fn phase_name(phase: ShockPhase) -> &'static str {
    phase.name()
}

fn main() {
    println!(
        "record_type,scenario,shock_kind,policy,tick,phase,total_or_tick_unmet_water_m3,total_or_tick_unmet_energy_kwh,water_deficit_area_household_steps,energy_deficit_area_household_steps,minimum_water_coverage,minimum_energy_coverage,recovery_after_ticks,basin_water_stock_m3,basin_energy_stock_kwh,max_water_conservation_residual_m3,max_energy_conservation_residual_kwh,water_coverage_h1,water_coverage_h2,water_coverage_h3,water_coverage_h4,energy_coverage_h1,energy_coverage_h2,energy_coverage_h3,energy_coverage_h4"
    );

    for outcome in run_reference_portfolio().expect("reference scenarios must validate") {
        summary_row(&outcome);
        for tick in &outcome.ticks {
            tick_row(&outcome, tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_portfolio_covers_all_shock_and_policy_combinations() {
        let outcomes = run_reference_portfolio().unwrap();
        assert_eq!(outcomes.len(), ShockKind::ALL.len() * InterventionPolicy::ALL.len());
    }

    #[test]
    fn csv_records_have_the_declared_column_count() {
        assert_eq!(CSV_COLUMNS, 25);
    }
}
