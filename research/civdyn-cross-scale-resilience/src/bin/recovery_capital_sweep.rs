// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic service-versus-reserve recovery portfolio.

use civdyn_cross_scale_resilience::{
    RecoveryCapitalOutcome, RecoveryCapitalTick, run_recovery_capital_portfolio,
};

const CSV_COLUMNS: usize = 16;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "recovery-capital CSV schema must remain fixed"
    );
    println!("{}", fields.join(","));
}

fn summary_row(outcome: &RecoveryCapitalOutcome) {
    emit(vec![
        "summary".into(),
        outcome.path.name().into(),
        String::new(),
        String::new(),
        fmt(outcome.first_shock_unmet_water_m3),
        fmt(outcome.service_recovery_unmet_water_m3),
        fmt(outcome.repeat_shock_unmet_water_m3),
        fmt(outcome.reserve_after_first_shock_m3),
        fmt(outcome.reserve_at_service_recovery_m3),
        fmt(outcome.reserve_replenishment_m3),
        fmt(outcome.reserve_recovery_ratio),
        outcome.service_recovery_after_ticks.map_or_else(String::new, |v| v.to_string()),
        String::new(),
        String::new(),
        fmt(outcome.final_reserve_m3),
        fmt(outcome.maximum_conservation_residual_m3),
    ]);
}

fn tick_row(path: RecoveryPath, tick: &RecoveryCapitalTick) {
    emit(vec![
        "tick".into(),
        path.name().into(),
        tick.phase.name().into(),
        tick.tick.to_string(),
        String::new(),
        String::new(),
        fmt(tick.unmet_water_m3),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        fmt(tick.household_a_coverage),
        fmt(tick.household_b_coverage),
        fmt(tick.basin_closing_stock_m3),
        fmt(tick.maximum_conservation_residual_m3),
    ]);
}

fn main() {
    println!(
        "record_type,path,phase,tick,first_shock_unmet_water_m3,recovery_unmet_water_m3,repeat_shock_unmet_water_m3,reserve_after_first_shock_m3,reserve_at_service_recovery_m3,reserve_replenishment_m3,reserve_recovery_ratio,service_recovery_after_ticks,household_a_coverage,household_b_coverage,final_or_tick_reserve_m3,max_conservation_residual_m3"
    );
    for outcome in run_recovery_capital_portfolio().expect("recovery capital fixtures must validate") {
        summary_row(&outcome);
        for tick in &outcome.ticks {
            tick_row(outcome.path, tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civdyn_cross_scale_resilience::RecoveryPath;

    #[test]
    fn two_paths_emit_two_summary_rows_and_all_ticks() {
        let outcomes = run_recovery_capital_portfolio().unwrap();
        assert_eq!(outcomes.len(), RecoveryPath::ALL.len());
        assert!(outcomes.iter().all(|outcome| outcome.ticks.len() == 14));
    }

    #[test]
    fn summary_csv_schema_has_sixteen_columns() {
        assert_eq!(CSV_COLUMNS, 16);
    }
}
