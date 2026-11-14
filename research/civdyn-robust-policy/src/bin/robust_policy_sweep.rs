// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite-design robust policy comparison over the existing CivDyn resilience model.
//! LHS rows are design points, not probabilistic future scenarios.

use civdyn_resilience_uq::{InterventionPolicy, OutcomeMetric, ShockKind};
use civdyn_robust_policy::{
    DEFAULT_LHS_POINTS, PolicyRun, RobustPolicyReport, ServiceConstraints, execute_policy_design,
};

const CSV_COLUMNS: usize = 40;
const SEED: u64 = 0x4353_535F_524F_4231;

fn fmt(value: f64) -> String {
    format!("{value:.8}")
}

fn opt(value: Option<f64>) -> String {
    value.map_or_else(String::new, fmt)
}

fn emit(row: Vec<String>) {
    assert_eq!(row.len(), CSV_COLUMNS, "robust-policy CSV schema must remain fixed");
    println!("{}", row.join(","));
}

fn blank(record_type: &str) -> Vec<String> {
    let mut row = vec![String::new(); CSV_COLUMNS];
    row[0] = record_type.into();
    row[1] = SEED.to_string();
    row[6] = DEFAULT_LHS_POINTS.to_string();
    row
}

fn raw_row(run: &PolicyRun) -> Vec<String> {
    let mut row = blank("design_run");
    row[2] = run.design_point.design_id.clone();
    row[3] = run.shock_kind.name().into();
    row[4] = run.policy.name().into();
    row[26] = fmt(run.design_point.parameters.water_supply_multiplier);
    row[27] = fmt(run.design_point.parameters.energy_supply_multiplier);
    row[28] = fmt(run.design_point.parameters.demand_multiplier);
    row[29] = fmt(run.design_point.parameters.distributed_reserve_multiplier);
    row[30] = fmt(run.design_point.parameters.water_shock_retention);
    row[31] = fmt(run.design_point.parameters.energy_shock_retention);
    row[32] = run.design_point.parameters.shock_duration_ticks.to_string();
    if let Some(metrics) = run.metrics {
        row[33] = fmt(metrics.total_unmet_water_m3);
        row[34] = fmt(metrics.total_unmet_energy_kwh);
        row[35] = fmt(metrics.minimum_water_coverage);
        row[36] = fmt(metrics.minimum_energy_coverage);
        row[37] = fmt(metrics.water_floor_breach_fraction);
        row[38] = fmt(metrics.energy_floor_breach_fraction);
        row[39] = fmt(metrics.recovery_success);
    } else {
        row[8] = "1".into();
    }
    row
}

fn constraint_rows(report: &RobustPolicyReport) {
    for summary in &report.constraint_summaries {
        let mut row = blank("constraint_summary");
        row[3] = summary.shock_kind.name().into();
        row[4] = summary.policy.name().into();
        row[7] = (summary.pass_count + summary.fail_count).to_string();
        row[8] = summary.unknown_count.to_string();
        row[9] = summary.pass_count.to_string();
        row[10] = summary.fail_count.to_string();
        row[11] = summary.unknown_count.to_string();
        row[12] = summary
            .finite_design_robust_feasible
            .map_or_else(|| "unknown".into(), |value| value.to_string());
        row[13] = opt(summary.worst_water_coverage_margin);
        row[14] = opt(summary.worst_energy_coverage_margin);
        row[15] = opt(summary.worst_water_breach_margin);
        row[16] = opt(summary.worst_energy_breach_margin);
        emit(row);
    }
}

fn metric_regret_rows(report: &RobustPolicyReport) {
    for summary in &report.metric_regrets {
        let mut row = blank("metric_regret");
        row[3] = summary.shock_kind.name().into();
        row[4] = summary.policy.name().into();
        row[5] = summary.metric.name().into();
        row[7] = summary.valid_point_count.to_string();
        row[8] = summary.unknown_point_count.to_string();
        row[17] = opt(summary.mean_regret);
        row[18] = opt(summary.p95_regret);
        row[19] = opt(summary.worst_regret);
        row[20] = summary.best_count.to_string();
        row[21] = fmt(summary.best_fraction);
        emit(row);
    }
}

fn winner_rows(report: &RobustPolicyReport) {
    for summary in &report.distinct_winner_counts {
        let mut row = blank("winner_instability");
        row[3] = summary.shock_kind.name().into();
        row[5] = summary.metric.name().into();
        row[22] = summary.distinct_best_policy_count.to_string();
        row[23] = summary.design_points_with_unique_winner.to_string();
        row[24] = summary.design_points_with_tied_winners.to_string();
        row[25] = summary.design_points_with_unknown_comparison.to_string();
        emit(row);
    }
}

fn main() {
    println!(
        "record_type,seed,design_id,shock_kind,policy,metric,lhs_point_count,valid_point_count,unknown_point_count,constraint_pass_count,constraint_fail_count,constraint_unknown_count,finite_design_robust_feasible,worst_water_coverage_margin,worst_energy_coverage_margin,worst_water_breach_margin,worst_energy_breach_margin,mean_regret,p95_regret,worst_regret,best_count,best_fraction,distinct_best_policy_count,unique_winner_points,tied_winner_points,unknown_comparison_points,water_supply_multiplier,energy_supply_multiplier,demand_multiplier,distributed_reserve_multiplier,water_shock_retention,energy_shock_retention,shock_duration_ticks,total_unmet_water_m3,total_unmet_energy_kwh,minimum_water_coverage,minimum_energy_coverage,water_floor_breach_fraction,energy_floor_breach_fraction,recovery_success"
    );

    let (runs, report) = execute_policy_design(SEED, DEFAULT_LHS_POINTS, ServiceConstraints::default())
        .expect("declared CivDyn LHS policy experiment must validate");

    for run in &runs {
        emit(raw_row(run));
    }
    constraint_rows(&report);
    metric_regret_rows(&report);
    winner_rows(&report);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_policy_taxonomy_is_four_by_three_with_nine_metrics() {
        assert_eq!(InterventionPolicy::ALL.len(), 4);
        assert_eq!(ShockKind::ALL.len(), 3);
        assert_eq!(OutcomeMetric::ALL.len(), 9);
        assert_eq!(CSV_COLUMNS, 40);
    }

    #[test]
    fn small_finite_design_emits_metric_specific_not_aggregate_regret() {
        let (runs, report) =
            execute_policy_design(SEED, 2, ServiceConstraints::default()).unwrap();
        assert_eq!(runs.len(), 2 * 3 * 4);
        assert_eq!(report.constraint_summaries.len(), 3 * 4);
        assert_eq!(report.metric_regrets.len(), 3 * 4 * 9);
        assert_eq!(report.distinct_winner_counts.len(), 3 * 9);
        assert!(report.metric_regrets.iter().all(|entry| entry.metric.name() != "universal_score"));
    }
}
