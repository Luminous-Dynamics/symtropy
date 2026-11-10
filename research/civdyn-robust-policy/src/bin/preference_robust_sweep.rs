// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Preference uncertainty over the existing finite CivDYN policy experiment.
//! Profile vertices are declared preferences, not random samples or probabilities.

use std::collections::BTreeMap;

use civdyn_resilience_uq::OutcomeMetric;
use civdyn_robust_policy::preference_robust::{
    ObjectiveDirection, PreferenceContract, PreferenceCriterion, PreferenceSetKind,
    PreferenceVertex, analyze_preference_robustness,
};
use civdyn_robust_policy::{
    DEFAULT_LHS_POINTS, PolicyRun, ServiceConstraints, execute_policy_design,
};

const CSV_COLUMNS: usize = 44;
const SEED: u64 = 0x4356_4459_4e50_5246;

fn fmt(value: f64) -> String {
    format!("{value:.10}")
}

fn opt(value: Option<f64>) -> String {
    value.map_or_else(String::new, fmt)
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(row: Vec<String>) {
    assert_eq!(
        row.len(),
        CSV_COLUMNS,
        "preference-robust CSV schema must remain fixed"
    );
    println!(
        "{}",
        row.iter().map(|field| csv_field(field)).collect::<Vec<_>>().join(",")
    );
}

fn blank(record_type: &str) -> Vec<String> {
    let mut row = vec![String::new(); CSV_COLUMNS];
    row[0] = record_type.to_string();
    row[1] = "municipal-service-preferences-v0.1".to_string();
    row[2] = "sha256-municipal-service-preferences-v0.1".to_string();
    row[39] = PreferenceSetKind::CompletePolytopeVertices.name().to_string();
    row[41] = "false".to_string();
    row
}

fn contract() -> PreferenceContract {
    let criteria = vec![
        PreferenceCriterion {
            metric: OutcomeMetric::TotalUnmetWaterM3,
            direction: ObjectiveDirection::Minimize,
            reference_scale: 100.0,
            scale_evidence_digest: "sha256-water-reference-scale".to_string(),
        },
        PreferenceCriterion {
            metric: OutcomeMetric::TotalUnmetEnergyKwh,
            direction: ObjectiveDirection::Minimize,
            reference_scale: 100.0,
            scale_evidence_digest: "sha256-energy-reference-scale".to_string(),
        },
        PreferenceCriterion {
            metric: OutcomeMetric::MinimumWaterCoverage,
            direction: ObjectiveDirection::Maximize,
            reference_scale: 1.0,
            scale_evidence_digest: "sha256-water-coverage-scale".to_string(),
        },
        PreferenceCriterion {
            metric: OutcomeMetric::MinimumEnergyCoverage,
            direction: ObjectiveDirection::Maximize,
            reference_scale: 1.0,
            scale_evidence_digest: "sha256-energy-coverage-scale".to_string(),
        },
        PreferenceCriterion {
            metric: OutcomeMetric::RecoverySuccess,
            direction: ObjectiveDirection::Maximize,
            reference_scale: 1.0,
            scale_evidence_digest: "sha256-recovery-scale".to_string(),
        },
    ];
    let vertices = vec![
        PreferenceVertex {
            profile_id: "water-priority".to_string(),
            stakeholder_group_id: "water-dependent-households".to_string(),
            weights: vec![0.80, 0.05, 0.05, 0.05, 0.05],
        },
        PreferenceVertex {
            profile_id: "energy-priority".to_string(),
            stakeholder_group_id: "energy-dependent-households".to_string(),
            weights: vec![0.05, 0.80, 0.05, 0.05, 0.05],
        },
        PreferenceVertex {
            profile_id: "service-reliability-priority".to_string(),
            stakeholder_group_id: "essential-service-providers".to_string(),
            weights: vec![0.05, 0.05, 0.15, 0.15, 0.60],
        },
    ];
    PreferenceContract {
        contract_id: "municipal-service-preferences".to_string(),
        version: "v0.1".to_string(),
        contract_digest: "sha256-municipal-service-preferences-v0.1".to_string(),
        preference_set_kind: PreferenceSetKind::CompletePolytopeVertices,
        criteria,
        vertices,
        service_constraints: ServiceConstraints::default(),
    }
}

fn run_index(runs: &[PolicyRun]) -> BTreeMap<(String, String, String), &PolicyRun> {
    runs.iter()
        .map(|run| {
            (
                (
                    run.design_point.design_id.clone(),
                    run.shock_kind.name().to_string(),
                    run.policy.name().to_string(),
                ),
                run,
            )
        })
        .collect()
}

fn scenario_result_rows(
    contract: &PreferenceContract,
    runs: &[PolicyRun],
    report: &civdyn_robust_policy::preference_robust::PreferenceRobustReport,
) {
    let index = run_index(runs);
    let criterion_order = contract
        .criteria
        .iter()
        .map(|criterion| criterion.metric.name())
        .collect::<Vec<_>>()
        .join(";");
    let directions = contract
        .criteria
        .iter()
        .map(|criterion| criterion.direction.name())
        .collect::<Vec<_>>()
        .join(";");
    let scales = contract
        .criteria
        .iter()
        .map(|criterion| fmt(criterion.reference_scale))
        .collect::<Vec<_>>()
        .join(";");

    for result in &report.results {
        let mut row = blank("scenario_profile_policy");
        let run = index[&(
            result.design_point_id.clone(),
            result.shock_kind.clone(),
            result.policy.clone(),
        )];
        let profile = contract
            .vertices
            .iter()
            .find(|profile| profile.profile_id == result.profile_id)
            .expect("result profile must be declared");
        row[3] = result.design_point_id.clone();
        row[4] = result.shock_kind.clone();
        row[5] = result.profile_id.clone();
        row[6] = profile.stakeholder_group_id.clone();
        row[7] = result.policy.clone();
        row[8] = result.eligibility.name().to_string();
        row[9] = result.scenario_status.name().to_string();
        row[10] = result.feasible_policy_count.to_string();
        row[11] = opt(result.normalized_utility);
        row[12] = opt(result.regret);
        row[13] = result.tied_best.to_string();
        row[14] = result.pareto_nondominated.map_or_else(String::new, |value| value.to_string());
        row[15] = criterion_order.clone();
        row[16] = directions.clone();
        row[17] = scales.clone();
        row[18] = contract
            .criteria
            .iter()
            .map(|criterion| criterion.scale_evidence_digest.clone())
            .collect::<Vec<_>>()
            .join(";");
        row[19] = profile.weights.iter().map(|weight| fmt(*weight)).collect::<Vec<_>>().join(";");
        if let Some(metrics) = run.metrics {
            row[20] = fmt(metrics.total_unmet_water_m3);
            row[21] = fmt(metrics.total_unmet_energy_kwh);
            row[22] = fmt(metrics.minimum_water_coverage);
            row[23] = fmt(metrics.minimum_energy_coverage);
            row[24] = fmt(metrics.recovery_success);
        }
        report_fields(&mut row, report);
        emit(row);
    }
}

fn report_fields(
    row: &mut [String],
    report: &civdyn_robust_policy::preference_robust::PreferenceRobustReport,
) {
    row[40] = report.claim_ceiling.name().to_string();
    row[42] = report.no_feasible_scenario_profile_count.to_string();
    row[43] = report.incomplete_scenario_profile_count.to_string();
}

fn summary_rows(report: &civdyn_robust_policy::preference_robust::PreferenceRobustReport) {
    for summary in &report.profile_policy_summaries {
        let mut row = blank("profile_policy_summary");
        row[5] = summary.profile_id.clone();
        row[6] = summary.stakeholder_group_id.clone();
        row[7] = summary.policy.clone();
        row[25] = summary.scenario_count.to_string();
        row[26] = summary.feasible_count.to_string();
        row[27] = summary.infeasible_count.to_string();
        row[28] = summary.unknown_count.to_string();
        row[29] = summary.best_scenario_count.to_string();
        row[30] = opt(summary.mean_regret);
        row[31] = opt(summary.p95_regret);
        row[32] = opt(summary.worst_regret);
        row[40] = report.claim_ceiling.name().to_string();
        row[42] = report.no_feasible_scenario_profile_count.to_string();
        row[43] = report.incomplete_scenario_profile_count.to_string();
        emit(row);
    }

    for summary in &report.policy_summaries {
        let mut row = blank("policy_preference_summary");
        row[7] = summary.policy.clone();
        row[25] = summary.scenario_count.to_string();
        row[26] = summary.pass_count.to_string();
        row[27] = summary.fail_count.to_string();
        row[28] = summary.unknown_count.to_string();
        row[33] = summary.finite_design_robust_feasible.map_or_else(
            || "unknown".to_string(),
            |value| value.to_string(),
        );
        row[34] = summary.worst_regret_across_declared_profiles.map_or_else(
            String::new,
            fmt,
        );
        row[35] = summary.worst_case_regret_over_preference_polytope.map_or_else(String::new, fmt);
        row[36] = summary.preference_vertices_best_count.to_string();
        row[37] = summary.preference_vertices_considered.to_string();
        report_fields(&mut row, report);
        emit(row);
    }

    for summary in &report.scenario_consensus {
        let mut row = blank("scenario_preference_consensus");
        row[3] = summary.design_point_id.clone();
        row[4] = summary.shock_kind.clone();
        row[25] = summary.profile_count.to_string();
        row[26] = summary.profiles_with_winner.to_string();
        row[27] = summary.distinct_winner_policy_count.to_string();
        row[28] = summary.profiles_with_different_winners.to_string();
        row[38] = summary.unanimous_single_winner.clone().unwrap_or_default();
        report_fields(&mut row, report);
        emit(row);
    }
}

fn main() {
    println!(
        "record_type,contract_id,contract_digest,design_point_id,shock_kind,profile_id,stakeholder_group_id,policy,eligibility,scenario_status,feasible_policy_count,normalized_utility,regret,tied_best,pareto_nondominated,criterion_order,criterion_directions,reference_scales,scale_evidence_digests,preference_weights,total_unmet_water_m3,total_unmet_energy_kwh,minimum_water_coverage,minimum_energy_coverage,recovery_success,scenario_count,feasible_count,infeasible_count,unknown_count,best_scenario_count,mean_regret,p95_regret,worst_regret,finite_design_robust_feasible,worst_regret_across_declared_profiles,worst_case_regret_over_preference_polytope,preference_vertices_best_count,preference_vertices_considered,unanimous_single_winner,preference_set_kind,claim_ceiling,profile_vertices_are_probabilities,no_feasible_scenario_profile_count,incomplete_scenario_profile_count"
    );
    let contract = contract();
    let (runs, _) = execute_policy_design(SEED, DEFAULT_LHS_POINTS, contract.service_constraints)
        .expect("declared finite policy design must execute");
    let report = analyze_preference_robustness(&contract, &runs)
        .expect("declared preference contract must validate");
    scenario_result_rows(&contract, &runs, &report);
    summary_rows(&report);
}
