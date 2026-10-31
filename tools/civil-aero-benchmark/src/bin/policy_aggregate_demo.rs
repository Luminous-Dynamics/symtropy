use std::process;

use civil_aero_benchmark::policy_aggregate::{
    PolicyAggregateConfig, aggregate_policy_benchmark, report_to_csv,
};
use civil_aero_benchmark::policy_benchmark::PolicyBenchmarkRow;

const BASELINE_SCORES: [f64; 10] = [
    0.22, 0.38, 0.28, 0.42, 0.18, 0.35, 0.31, 0.27, 0.40, 0.21,
];
const VOI_DELTAS: [f64; 10] = [
    0.05, 0.04, 0.03, -0.02, 0.06, -0.03, 0.02, 0.02, -0.04, 0.01,
];
const ADJOINT_DELTAS: [f64; 10] = [
    0.12, -0.02, -0.04, 0.06, -0.01, -0.02, 0.03, -0.03, 0.09, -0.05,
];
const REPLICATE_JITTER: [f64; 3] = [-0.01, 0.0, 0.01];

fn scorecard_row(
    task_index: usize,
    replicate_index: usize,
    policy: &str,
    score: f64,
) -> PolicyBenchmarkRow {
    let cap = 10.0 + task_index as f64;
    let utilization = match policy {
        "uniform" => 0.75,
        "voi" => 0.64,
        "adjoint" => 0.86,
        _ => unreachable!("demo policy set is fixed"),
    };
    let total_cost = cap * utilization;
    PolicyBenchmarkRow {
        task_id: format!("synthetic-family-{:02}", task_index + 1),
        replicate_id: format!("seed-{}", replicate_index + 1),
        policy_id: policy.to_owned(),
        objective_1_name: "normalized_drag".to_owned(),
        objective_2_name: "normalized_mission_energy".to_owned(),
        ideal_point_1: 0.0,
        ideal_point_2: 0.0,
        reference_point_1: 100.0,
        reference_point_2: 100.0,
        budget_cap_units: cap,
        total_cost_units: total_cost,
        remaining_budget_units: cap - total_cost,
        budget_utilization_fraction: utilization,
        wall_seconds: total_cost * 20.0,
        cpu_seconds: total_cost * 16.0,
        operation_count: 8,
        successful_operations: 7,
        failed_operations: 1,
        high_fidelity_attempts: 5,
        successful_high_fidelity_operations: 4,
        admitted_candidates: 3,
        not_admitted_candidates: 1,
        feasible_admitted_candidates: 3,
        feasible_yield: 0.6,
        admission_yield: 0.8,
        validated_hypervolume_2d: score * 10_000.0,
        normalized_validated_hypervolume_fraction: score,
        validated_hypervolume_per_cost: Some(score * 10_000.0 / total_cost),
        normalized_hypervolume_per_cost: Some(score / total_cost),
        evidence_level: "synthetic_policy_scorecard_only",
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic task-aggregate demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut rows = Vec::with_capacity(10 * 3 * 3);
    for task_index in 0..10 {
        for (replicate_index, jitter) in REPLICATE_JITTER.iter().enumerate() {
            rows.push(scorecard_row(
                task_index,
                replicate_index,
                "uniform",
                BASELINE_SCORES[task_index] + *jitter,
            ));
            rows.push(scorecard_row(
                task_index,
                replicate_index,
                "voi",
                BASELINE_SCORES[task_index] + VOI_DELTAS[task_index] + *jitter,
            ));
            rows.push(scorecard_row(
                task_index,
                replicate_index,
                "adjoint",
                BASELINE_SCORES[task_index] + ADJOINT_DELTAS[task_index] + *jitter,
            ));
        }
    }

    let config = PolicyAggregateConfig {
        baseline_policy_id: "uniform".to_owned(),
        cost_unit_basis: "synthetic_normalized_compute_credit_v0.1".to_owned(),
        bootstrap_samples: 10_000,
        bootstrap_seed: 20_261_031,
    };
    let report = aggregate_policy_benchmark(&rows, &config)?;
    eprintln!(
        "synthetic_policy_task_aggregate_only tasks=10 replicates_per_task=3 policies=3 bootstrap_samples={} baseline={} task_cluster_bootstrap=true no_solver_evaluations=true",
        report.bootstrap_samples,
        report.baseline_policy_id
    );
    print!("{}", report_to_csv(&report));
    Ok(())
}
