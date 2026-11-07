// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic Latin-hypercube and Morris screening evidence export.

use civdyn_cross_scale_resilience::{InterventionPolicy, ShockKind};
use civdyn_resilience_uq::{
    BenchmarkReport, FACTOR_COUNT, LHS_DEFAULT_POINTS, MORRIS_DEFAULT_TRAJECTORIES,
    MorrisEffectSummary, MorrisPointRecord, execute_benchmark,
};

const CSV_COLUMNS: usize = 42;
const SEED: u64 = 0x4353_535F_5551_3031;

fn fmt(value: f64) -> String {
    format!("{value:.8}")
}

fn blank_row() -> Vec<String> {
    vec![String::new(); CSV_COLUMNS]
}

fn emit(row: Vec<String>) {
    assert_eq!(
        row.len(),
        CSV_COLUMNS,
        "resilience-UQ CSV schema must remain fixed"
    );
    println!("{}", row.join(","));
}

fn shock_name(shock: ShockKind) -> &'static str {
    shock.name()
}

fn policy_name(policy: InterventionPolicy) -> &'static str {
    policy.name()
}

fn set_common(row: &mut [String], report: &BenchmarkReport, record_type: &str) {
    row[0] = record_type.into();
    row[18] = report.seed.to_string();
}

fn emit_lhs_runs(report: &BenchmarkReport) {
    for run in &report.lhs_runs {
        let mut row = blank_row();
        set_common(&mut row, report, "lhs_run");
        row[1] = run.design_point.design_id.clone();
        row[4] = shock_name(run.shock_kind).into();
        row[5] = policy_name(run.policy).into();
        row[6] = "run_bundle".into();
        row[7] = "1".into();

        let parameter_values = run.design_point.parameters.values();
        for (index, value) in parameter_values.iter().enumerate() {
            row[19 + index] = fmt(*value);
        }

        let metrics = run.metrics;
        let output_values = [
            metrics.total_unmet_water_m3,
            metrics.total_unmet_energy_kwh,
            metrics.water_deficit_area_household_steps,
            metrics.energy_deficit_area_household_steps,
            metrics.minimum_water_coverage,
            metrics.minimum_energy_coverage,
            metrics.water_floor_breach_fraction,
            metrics.energy_floor_breach_fraction,
            metrics.recovery_success,
        ];
        for (index, value) in output_values.iter().enumerate() {
            row[26 + index] = fmt(*value);
        }
        for (index, value) in run.design_point.unit_coordinates.iter().enumerate() {
            row[35 + index] = fmt(*value);
        }
        emit(row);
    }
}

fn emit_lhs_summaries(report: &BenchmarkReport) {
    for summary in &report.lhs_summaries {
        let mut row = blank_row();
        set_common(&mut row, report, "lhs_summary");
        row[1] = format!(
            "lhs_ensemble_{}",
            report.lhs_runs.len() / (ShockKind::ALL.len() * InterventionPolicy::ALL.len())
        );
        row[4] = shock_name(summary.shock_kind).into();
        row[5] = policy_name(summary.policy).into();
        row[6] = summary.metric.name().into();
        row[7] = summary.sample_count.to_string();
        row[8] = fmt(summary.mean);
        row[9] = fmt(summary.p05);
        row[10] = fmt(summary.p50);
        row[11] = fmt(summary.p95);
        row[12] = fmt(summary.lower_tail_mean);
        row[13] = fmt(summary.upper_tail_mean);
        row[14] = fmt(summary.run_breach_fraction);
        emit(row);
    }
}

fn emit_morris_points(report: &BenchmarkReport, point: &MorrisPointRecord) {
    let mut row = blank_row();
    set_common(&mut row, report, "morris_point");
    row[1] = format!(
        "morris_{:02}_{:02}",
        point.trajectory_index, point.point_index
    );
    row[2] = format!("morris_{:02}", point.trajectory_index);
    row[7] = (FACTOR_COUNT + 1).to_string();
    for (index, value) in point.unit_coordinates.iter().enumerate() {
        row[35 + index] = fmt(*value);
    }
    emit(row);
}

fn emit_morris_effect(report: &BenchmarkReport, effect: &MorrisEffectSummary) {
    let mut row = blank_row();
    set_common(&mut row, report, "morris_effect");
    row[1] = format!("morris_ensemble_{}", effect.trajectory_count);
    row[3] = effect.factor.name().into();
    row[4] = shock_name(effect.shock_kind).into();
    row[5] = policy_name(effect.policy).into();
    row[6] = effect.metric.name().into();
    row[7] = effect.trajectory_count.to_string();
    row[15] = fmt(effect.mu);
    row[16] = fmt(effect.mu_star);
    row[17] = fmt(effect.sigma);
    emit(row);
}

fn main() {
    println!(
        "record_type,design_id,trajectory_id,parameter_id,shock_kind,policy,metric_id,sample_count,mean,p05,p50,p95,lower_tail_mean,upper_tail_mean,run_breach_fraction,mu,mu_star,sigma,seed,water_supply_multiplier,energy_supply_multiplier,demand_multiplier,distributed_reserve_multiplier,water_shock_retention,energy_shock_retention,shock_duration_ticks,water_unmet_m3,energy_unmet_kwh,water_deficit_area_household_steps,energy_deficit_area_household_steps,minimum_water_coverage,minimum_energy_coverage,water_floor_breach_fraction,energy_floor_breach_fraction,recovery_success,u_water_supply,u_energy_supply,u_demand,u_reserve,u_water_retention,u_energy_retention,u_shock_duration"
    );

    let report = execute_benchmark(SEED, LHS_DEFAULT_POINTS, MORRIS_DEFAULT_TRAJECTORIES)
        .expect("default deterministic resilience-UQ benchmark must execute");
    emit_lhs_runs(&report);
    emit_lhs_summaries(&report);
    for point in &report.morris_points {
        emit_morris_points(&report, point);
    }
    for effect in &report.morris_effects {
        emit_morris_effect(&report, effect);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_default_design_sizes_are_frozen() {
        assert_eq!(LHS_DEFAULT_POINTS, 30);
        assert_eq!(MORRIS_DEFAULT_TRAJECTORIES, 30);
        assert_eq!(FACTOR_COUNT, 7);
        assert_eq!(CSV_COLUMNS, 42);
    }

    #[test]
    fn output_metric_taxonomy_is_nine_dimensional() {
        assert_eq!(civdyn_resilience_uq::OutcomeMetric::ALL.len(), 9);
        assert_eq!(civdyn_resilience_uq::ParameterFactor::ALL.len(), 7);
    }

    #[test]
    fn default_report_has_expected_design_and_summary_cardinality() {
        let report = execute_benchmark(SEED, 2, 2).unwrap();
        assert_eq!(
            report.lhs_runs.len(),
            2 * ShockKind::ALL.len() * InterventionPolicy::ALL.len()
        );
        assert_eq!(
            report.lhs_summaries.len(),
            ShockKind::ALL.len()
                * InterventionPolicy::ALL.len()
                * civdyn_resilience_uq::OutcomeMetric::ALL.len()
        );
        assert_eq!(report.morris_points.len(), 2 * (FACTOR_COUNT + 1));
        assert_eq!(
            report.morris_effects.len(),
            FACTOR_COUNT
                * ShockKind::ALL.len()
                * InterventionPolicy::ALL.len()
                * civdyn_resilience_uq::OutcomeMetric::ALL.len()
        );
    }
}
