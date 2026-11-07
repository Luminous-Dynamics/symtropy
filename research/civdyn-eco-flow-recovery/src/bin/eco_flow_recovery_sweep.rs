// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! CSV export for the synthetic environmental-flow / reserve-recovery portfolio.

use civdyn_eco_flow_recovery::{
    AllocationPriority, Phase, RecoveryOutcome, RecoveryPath, RecoveryTick, run_portfolio,
};

const CSV_COLUMNS: usize = 29;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn blank() -> String {
    String::new()
}

fn summary_row(outcome: &RecoveryOutcome) -> Vec<String> {
    vec![
        "summary".into(),
        outcome.path.name().into(),
        outcome.priority.name().into(),
        blank(),
        blank(),
        fmt(outcome.first_shock_household_unmet_m3),
        fmt(outcome.first_shock_environmental_flow_unmet_m3),
        fmt(outcome.recovery_household_unmet_m3),
        fmt(outcome.recovery_environmental_flow_unmet_m3),
        fmt(outcome.repeat_shock_household_unmet_m3),
        fmt(outcome.repeat_shock_environmental_flow_unmet_m3),
        fmt(outcome.reserve_before_first_shock_m3),
        fmt(outcome.reserve_after_first_shock_m3),
        fmt(outcome.reserve_after_service_recovery_m3),
        fmt(outcome.reserve_rebuilding_m3),
        fmt(outcome.reserve_recovery_ratio),
        outcome
            .household_recovery_after_ticks
            .map_or_else(blank, |v| v.to_string()),
        outcome
            .environmental_flow_recovery_after_ticks
            .map_or_else(blank, |v| v.to_string()),
        fmt(outcome.minimum_repeat_household_coverage),
        fmt(outcome.minimum_repeat_environmental_flow_coverage),
        fmt(outcome.final_reserve_m3),
        fmt(outcome.maximum_conservation_residual_m3),
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
    ]
}

fn tick_row(path: RecoveryPath, priority: AllocationPriority, tick: &RecoveryTick) -> Vec<String> {
    let (first_household, first_ecological) = if tick.phase == Phase::FirstShock {
        (
            fmt(tick.household_unmet_m3),
            fmt(tick.environmental_flow_unmet_m3),
        )
    } else {
        (blank(), blank())
    };
    let (recovery_household, recovery_ecological) = if tick.phase == Phase::ServiceRecovery {
        (
            fmt(tick.household_unmet_m3),
            fmt(tick.environmental_flow_unmet_m3),
        )
    } else {
        (blank(), blank())
    };
    let (repeat_household, repeat_ecological) = if tick.phase == Phase::RepeatedShock {
        (
            fmt(tick.household_unmet_m3),
            fmt(tick.environmental_flow_unmet_m3),
        )
    } else {
        (blank(), blank())
    };

    vec![
        "tick".into(),
        path.name().into(),
        priority.name().into(),
        tick.phase.name().into(),
        tick.tick.to_string(),
        first_household,
        first_ecological,
        recovery_household,
        recovery_ecological,
        repeat_household,
        repeat_ecological,
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
        blank(),
        fmt(tick.household_a_coverage),
        fmt(tick.household_b_coverage),
        fmt(tick.environmental_flow_coverage),
        fmt(tick.household_unmet_m3),
        fmt(tick.environmental_flow_unmet_m3),
        fmt(tick.basin_closing_stock_m3),
        fmt(tick.maximum_conservation_residual_m3),
    ]
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "eco-flow recovery CSV schema must remain fixed"
    );
    println!("{}", fields.join(","));
}

fn main() {
    println!(
        "record_type,path,priority,phase,tick,first_shock_household_unmet_m3,first_shock_environmental_flow_unmet_m3,recovery_household_unmet_m3,recovery_environmental_flow_unmet_m3,repeat_shock_household_unmet_m3,repeat_shock_environmental_flow_unmet_m3,reserve_before_first_shock_m3,reserve_after_first_shock_m3,reserve_after_service_recovery_m3,reserve_rebuilding_m3,reserve_recovery_ratio,household_recovery_after_ticks,environmental_flow_recovery_after_ticks,minimum_repeat_household_coverage,minimum_repeat_environmental_flow_coverage,final_reserve_m3,maximum_conservation_residual_m3,household_a_coverage,household_b_coverage,environmental_flow_coverage,household_unmet_m3,environmental_flow_unmet_m3,basin_closing_stock_m3,tick_conservation_residual_m3"
    );

    for outcome in run_portfolio().expect("synthetic eco-flow portfolio must validate") {
        emit(summary_row(&outcome));
        for tick in &outcome.ticks {
            emit(tick_row(outcome.path, outcome.priority, tick));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_rows_have_fixed_schema() {
        let outcomes = run_portfolio().expect("synthetic portfolio valid");
        assert_eq!(
            outcomes.len(),
            RecoveryPath::ALL.len() * AllocationPriority::ALL.len()
        );
        for outcome in &outcomes {
            assert_eq!(summary_row(outcome).len(), CSV_COLUMNS);
            for tick in &outcome.ticks {
                assert_eq!(
                    tick_row(outcome.path, outcome.priority, tick).len(),
                    CSV_COLUMNS
                );
            }
        }
    }

    #[test]
    fn portfolio_emits_four_summaries_and_fifty_six_tick_rows() {
        let outcomes = run_portfolio().expect("synthetic portfolio valid");
        assert_eq!(outcomes.len(), 4);
        assert_eq!(
            outcomes.iter().map(|run| run.ticks.len()).sum::<usize>(),
            56
        );
    }
}
