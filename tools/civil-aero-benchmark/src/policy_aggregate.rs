//! Paired multi-task aggregation for matched-budget policy scorecards.
//!
//! The unit of generalization is the benchmark task, not the individual replicate. Each task's
//! policy score is the median across its replicates; paired policy-minus-baseline deltas are then
//! aggregated across tasks with equal task weight. Bootstrap resampling is clustered by task.
//! This module does not authenticate evidence receipts or convert synthetic tasks into CFD data.

use std::collections::{BTreeMap, BTreeSet};

use crate::policy_benchmark::PolicyBenchmarkRow;

const MINIMUM_TASKS: usize = 5;
const MINIMUM_REPLICATES: usize = 2;
const MINIMUM_BOOTSTRAP_SAMPLES: usize = 1_000;
const MAXIMUM_BOOTSTRAP_SAMPLES: usize = 1_000_000;
const DELTA_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyAggregateConfig {
    pub baseline_policy_id: String,
    pub cost_unit_basis: String,
    pub bootstrap_samples: usize,
    pub bootstrap_seed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyAggregateRow {
    pub policy_id: String,
    pub baseline_policy_id: String,
    pub task_count: usize,
    pub replicates_per_task: usize,
    pub mean_task_median_normalized_hypervolume: f64,
    pub median_task_median_normalized_hypervolume: f64,
    pub p10_task_median_normalized_hypervolume: f64,
    pub worst_task_normalized_hypervolume: f64,
    pub mean_paired_delta_vs_baseline: f64,
    pub median_paired_delta_vs_baseline: f64,
    pub p10_paired_delta_vs_baseline: f64,
    pub worst_task_delta_vs_baseline: f64,
    pub tasks_better: usize,
    pub tasks_tied: usize,
    pub tasks_worse: usize,
    pub bootstrap_95_ci_low: f64,
    pub bootstrap_95_ci_high: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyAggregateReport {
    pub baseline_policy_id: String,
    pub cost_unit_basis: String,
    pub bootstrap_samples: usize,
    pub bootstrap_seed: u64,
    pub evidence_level: &'static str,
    pub rows: Vec<PolicyAggregateRow>,
}

type Matrix = BTreeMap<String, BTreeMap<String, BTreeMap<String, f64>>>;

pub fn aggregate_policy_benchmark(
    scorecard_rows: &[PolicyBenchmarkRow],
    config: &PolicyAggregateConfig,
) -> Result<PolicyAggregateReport, String> {
    if config.baseline_policy_id.trim().is_empty() || config.cost_unit_basis.trim().is_empty() {
        return Err("baseline_policy_id and cost_unit_basis must be non-empty".to_owned());
    }
    if config.bootstrap_samples < MINIMUM_BOOTSTRAP_SAMPLES
        || config.bootstrap_samples > MAXIMUM_BOOTSTRAP_SAMPLES
    {
        return Err(format!(
            "bootstrap_samples must be in [{MINIMUM_BOOTSTRAP_SAMPLES}, {MAXIMUM_BOOTSTRAP_SAMPLES}]"
        ));
    }
    if config.bootstrap_seed == 0 {
        return Err("bootstrap_seed must be non-zero".to_owned());
    }
    if scorecard_rows.is_empty() {
        return Err("at least one policy scorecard row is required".to_owned());
    }

    let mut matrix: Matrix = BTreeMap::new();
    let mut tasks = BTreeSet::new();
    let mut replicates = BTreeSet::new();
    let mut policies = BTreeSet::new();
    let mut task_budget_caps = BTreeMap::<String, f64>::new();

    for row in scorecard_rows {
        if row.task_id.trim().is_empty()
            || row.replicate_id.trim().is_empty()
            || row.policy_id.trim().is_empty()
        {
            return Err("task, replicate, and policy identifiers must be non-empty".to_owned());
        }
        if !row.budget_cap_units.is_finite()
            || row.budget_cap_units <= 0.0
            || !row.total_cost_units.is_finite()
            || row.total_cost_units <= 0.0
            || row.total_cost_units > row.budget_cap_units + 1.0e-9
        {
            return Err(format!(
                "task {} replicate {} policy {} has invalid or over-budget cost",
                row.task_id, row.replicate_id, row.policy_id
            ));
        }
        let score = row.normalized_validated_hypervolume_fraction;
        if !score.is_finite() || !(0.0..=1.0).contains(&score) {
            return Err(format!(
                "task {} replicate {} policy {} has normalized hypervolume outside [0, 1]",
                row.task_id, row.replicate_id, row.policy_id
            ));
        }

        match task_budget_caps.get(&row.task_id) {
            Some(cap) if (*cap - row.budget_cap_units).abs() > 1.0e-12 => {
                return Err(format!(
                    "task {} does not use one fixed budget cap across policies and replicates",
                    row.task_id
                ));
            }
            None => {
                task_budget_caps.insert(row.task_id.clone(), row.budget_cap_units);
            }
            _ => {}
        }

        let task_matrix = matrix.entry(row.task_id.clone()).or_default();
        let replicate_matrix = task_matrix.entry(row.replicate_id.clone()).or_default();
        if replicate_matrix.insert(row.policy_id.clone(), score).is_some() {
            return Err(format!(
                "duplicate task/replicate/policy scorecard row: {}/{}/{}",
                row.task_id, row.replicate_id, row.policy_id
            ));
        }
        tasks.insert(row.task_id.clone());
        replicates.insert(row.replicate_id.clone());
        policies.insert(row.policy_id.clone());
    }

    if tasks.len() < MINIMUM_TASKS {
        return Err(format!(
            "at least {MINIMUM_TASKS} independent task families are required; found {}",
            tasks.len()
        ));
    }
    if replicates.len() < MINIMUM_REPLICATES {
        return Err(format!(
            "at least {MINIMUM_REPLICATES} independent replicates per task are required"
        ));
    }
    if policies.len() < 2 || !policies.contains(&config.baseline_policy_id) {
        return Err("at least two policies and the declared baseline policy are required".to_owned());
    }

    for task in &tasks {
        let task_matrix = matrix
            .get(task)
            .ok_or_else(|| format!("task {task} is missing from the scorecard matrix"))?;
        if task_matrix.keys().cloned().collect::<BTreeSet<_>>() != replicates {
            return Err(format!("task {task} has an incomplete replicate matrix"));
        }
        for (replicate, policy_matrix) in task_matrix {
            if policy_matrix.keys().cloned().collect::<BTreeSet<_>>() != policies {
                return Err(format!(
                    "task {task} replicate {replicate} has an incomplete policy matrix"
                ));
            }
        }
    }

    let mut task_scores = BTreeMap::<String, BTreeMap<String, f64>>::new();
    for task in &tasks {
        let task_matrix = matrix.get(task).expect("task was collected from matrix");
        let mut scores_by_policy = BTreeMap::<String, Vec<f64>>::new();
        for policy_matrix in task_matrix.values() {
            for (policy, score) in policy_matrix {
                scores_by_policy
                    .entry(policy.clone())
                    .or_default()
                    .push(*score);
            }
        }
        let medians = scores_by_policy
            .into_iter()
            .map(|(policy, values)| (policy, median(values)))
            .collect();
        task_scores.insert(task.clone(), medians);
    }

    let baseline_scores = tasks
        .iter()
        .map(|task| task_scores[task][&config.baseline_policy_id])
        .collect::<Vec<_>>();
    let mut output_rows = Vec::with_capacity(policies.len());

    for policy in &policies {
        let mut scores = Vec::with_capacity(tasks.len());
        let mut deltas = Vec::with_capacity(tasks.len());
        let mut tasks_better = 0;
        let mut tasks_tied = 0;
        let mut tasks_worse = 0;

        for (index, task) in tasks.iter().enumerate() {
            let score = task_scores[task][policy];
            scores.push(score);
            let delta = score - baseline_scores[index];
            deltas.push(delta);
            if delta > DELTA_TOLERANCE {
                tasks_better += 1;
            } else if delta < -DELTA_TOLERANCE {
                tasks_worse += 1;
            } else {
                tasks_tied += 1;
            }
        }

        let seed = config.bootstrap_seed ^ stable_policy_hash(policy);
        let ci = bootstrap_mean_interval(&deltas, config.bootstrap_samples, seed);
        output_rows.push(PolicyAggregateRow {
            policy_id: policy.clone(),
            baseline_policy_id: config.baseline_policy_id.clone(),
            task_count: tasks.len(),
            replicates_per_task: replicates.len(),
            mean_task_median_normalized_hypervolume: mean(&scores),
            median_task_median_normalized_hypervolume: median(scores.clone()),
            p10_task_median_normalized_hypervolume: nearest_rank(&scores, 0.10),
            worst_task_normalized_hypervolume: scores.iter().copied().fold(f64::INFINITY, f64::min),
            mean_paired_delta_vs_baseline: mean(&deltas),
            median_paired_delta_vs_baseline: median(deltas.clone()),
            p10_paired_delta_vs_baseline: nearest_rank(&deltas, 0.10),
            worst_task_delta_vs_baseline: deltas.iter().copied().fold(f64::INFINITY, f64::min),
            tasks_better,
            tasks_tied,
            tasks_worse,
            bootstrap_95_ci_low: ci.0,
            bootstrap_95_ci_high: ci.1,
        });
    }
    output_rows.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));

    Ok(PolicyAggregateReport {
        baseline_policy_id: config.baseline_policy_id.clone(),
        cost_unit_basis: config.cost_unit_basis.clone(),
        bootstrap_samples: config.bootstrap_samples,
        bootstrap_seed: config.bootstrap_seed,
        evidence_level: "task_clustered_scorecard_aggregation_not_new_solver_evidence",
        rows: output_rows,
    })
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        0.5 * (values[middle - 1] + values[middle])
    } else {
        values[middle]
    }
}

fn nearest_rank(values: &[f64], probability: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (probability * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted[rank - 1]
}

fn bootstrap_mean_interval(values: &[f64], samples: usize, seed: u64) -> (f64, f64) {
    let mut rng = XorShift64::new(if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    });
    let mut means = Vec::with_capacity(samples);
    for _ in 0..samples {
        let mut total = 0.0;
        for _ in 0..values.len() {
            let index = (rng.next() % values.len() as u64) as usize;
            total += values[index];
        }
        means.push(total / values.len() as f64);
    }
    means.sort_by(f64::total_cmp);
    let low_index = (0.025 * means.len() as f64).ceil().max(1.0) as usize - 1;
    let high_index = (0.975 * means.len() as f64).ceil().max(1.0) as usize - 1;
    (means[low_index], means[high_index])
}

#[derive(Debug, Clone, Copy)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }
}

fn stable_policy_hash(value: &str) -> u64 {
    value.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

pub const POLICY_AGGREGATE_CSV_HEADER: &str = "policy_id,baseline_policy_id,task_count,replicates_per_task,mean_task_median_normalized_hypervolume,median_task_median_normalized_hypervolume,p10_task_median_normalized_hypervolume,worst_task_normalized_hypervolume,mean_paired_delta_vs_baseline,median_paired_delta_vs_baseline,p10_paired_delta_vs_baseline,worst_task_delta_vs_baseline,tasks_better,tasks_tied,tasks_worse,bootstrap_95_ci_low,bootstrap_95_ci_high";

pub fn report_to_csv(report: &PolicyAggregateReport) -> String {
    let mut output = format!("{POLICY_AGGREGATE_CSV_HEADER}\n");
    for row in &report.rows {
        output.push_str(&format!(
            "{},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{},{},{},{:.12},{:.12}\n",
            csv_escape(&row.policy_id),
            csv_escape(&row.baseline_policy_id),
            row.task_count,
            row.replicates_per_task,
            row.mean_task_median_normalized_hypervolume,
            row.median_task_median_normalized_hypervolume,
            row.p10_task_median_normalized_hypervolume,
            row.worst_task_normalized_hypervolume,
            row.mean_paired_delta_vs_baseline,
            row.median_paired_delta_vs_baseline,
            row.p10_paired_delta_vs_baseline,
            row.worst_task_delta_vs_baseline,
            row.tasks_better,
            row.tasks_tied,
            row.tasks_worse,
            row.bootstrap_95_ci_low,
            row.bootstrap_95_ci_high
        ));
    }
    output
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_rows() -> Vec<PolicyBenchmarkRow> {
        let baseline = [0.22, 0.38, 0.28, 0.42, 0.18];
        let improvement = [0.05, 0.04, 0.03, -0.02, 0.06];
        let mut rows = Vec::new();
        for task_index in 0..5 {
            for replicate_index in 0..3 {
                let jitter = (replicate_index as f64 - 1.0) * 0.01;
                for (policy, score) in [
                    ("uniform", baseline[task_index] + jitter),
                    ("voi", baseline[task_index] + improvement[task_index] + jitter),
                ] {
                    rows.push(PolicyBenchmarkRow {
                        task_id: format!("task-{task_index}"),
                        replicate_id: format!("seed-{replicate_index}"),
                        policy_id: policy.to_owned(),
                        objective_1_name: "objective_a".to_owned(),
                        objective_2_name: "objective_b".to_owned(),
                        ideal_point_1: 0.0,
                        ideal_point_2: 0.0,
                        reference_point_1: 1.0,
                        reference_point_2: 1.0,
                        budget_cap_units: 10.0,
                        total_cost_units: 8.0,
                        remaining_budget_units: 2.0,
                        budget_utilization_fraction: 0.8,
                        wall_seconds: 80.0,
                        cpu_seconds: 60.0,
                        operation_count: 5,
                        successful_operations: 4,
                        failed_operations: 1,
                        high_fidelity_attempts: 3,
                        successful_high_fidelity_operations: 2,
                        admitted_candidates: 2,
                        not_admitted_candidates: 0,
                        feasible_admitted_candidates: 2,
                        feasible_yield: 2.0 / 3.0,
                        admission_yield: 2.0 / 3.0,
                        validated_hypervolume_2d: score * 100.0,
                        normalized_validated_hypervolume_fraction: score,
                        validated_hypervolume_per_cost: Some(score * 12.5),
                        normalized_hypervolume_per_cost: Some(score / 8.0),
                        evidence_level: "synthetic_policy_scorecard_only",
                    });
                }
            }
        }
        rows
    }

    fn config() -> PolicyAggregateConfig {
        PolicyAggregateConfig {
            baseline_policy_id: "uniform".to_owned(),
            cost_unit_basis: "synthetic_normalized_compute_credit_v0.1".to_owned(),
            bootstrap_samples: 2_000,
            bootstrap_seed: 20_261_031,
        }
    }

    #[test]
    fn task_clustered_aggregate_pairs_policy_deltas_and_bootstraps_tasks() {
        let rows = fixture_rows();
        let report = aggregate_policy_benchmark(&rows, &config()).expect("valid task matrix");
        assert_eq!(report.rows.len(), 2);
        let voi = report.rows.iter().find(|row| row.policy_id == "voi").unwrap();
        assert_eq!(voi.task_count, 5);
        assert_eq!(voi.replicates_per_task, 3);
        assert_eq!(voi.tasks_better, 4);
        assert_eq!(voi.tasks_worse, 1);
        assert_eq!(voi.tasks_tied, 0);
        assert!((voi.mean_paired_delta_vs_baseline - 0.032).abs() < 1.0e-12);
        assert!((voi.worst_task_delta_vs_baseline + 0.02).abs() < 1.0e-12);
        assert!(voi.bootstrap_95_ci_low <= voi.mean_paired_delta_vs_baseline);
        assert!(voi.bootstrap_95_ci_high >= voi.mean_paired_delta_vs_baseline);
        let baseline = report.rows.iter().find(|row| row.policy_id == "uniform").unwrap();
        assert_eq!(baseline.tasks_tied, 5);
        assert_eq!(baseline.mean_paired_delta_vs_baseline, 0.0);
        assert_eq!(baseline.bootstrap_95_ci_low, 0.0);
        assert_eq!(baseline.bootstrap_95_ci_high, 0.0);
    }

    #[test]
    fn rejects_too_few_tasks_missing_policy_or_inconsistent_budget_caps() {
        let mut rows = fixture_rows();
        rows.retain(|row| row.task_id != "task-4");
        assert!(aggregate_policy_benchmark(&rows, &config())
            .expect_err("four tasks are insufficient")
            .contains("at least 5"));

        let mut rows = fixture_rows();
        rows.retain(|row| !(row.task_id == "task-2" && row.policy_id == "voi"));
        assert!(aggregate_policy_benchmark(&rows, &config())
            .expect_err("missing policy row rejected")
            .contains("incomplete policy matrix"));

        let mut rows = fixture_rows();
        rows[0].budget_cap_units = 9.0;
        assert!(aggregate_policy_benchmark(&rows, &config())
            .expect_err("different task budget cap rejected")
            .contains("fixed budget cap"));
    }

    #[test]
    fn rejects_single_replicate_and_invalid_bootstrap_configuration() {
        let mut rows = fixture_rows();
        rows.retain(|row| row.replicate_id == "seed-0");
        assert!(aggregate_policy_benchmark(&rows, &config())
            .expect_err("single replicate rejected")
            .contains("at least 2 independent"));

        let mut invalid = config();
        invalid.bootstrap_seed = 0;
        assert!(aggregate_policy_benchmark(&fixture_rows(), &invalid).is_err());
        invalid = config();
        invalid.bootstrap_samples = 999;
        assert!(aggregate_policy_benchmark(&fixture_rows(), &invalid).is_err());
    }

    #[test]
    fn bootstrap_is_reproducible_and_policy_order_does_not_change_result() {
        let rows = fixture_rows();
        let first = aggregate_policy_benchmark(&rows, &config()).expect("valid");
        let mut reversed = rows;
        reversed.reverse();
        let second = aggregate_policy_benchmark(&reversed, &config()).expect("valid");
        assert_eq!(first, second);
        assert_eq!(report_to_csv(&first), report_to_csv(&second));
    }
}
