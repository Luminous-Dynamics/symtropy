//! Hierarchical multi-task policy aggregation for matched-budget scorecards.
//!
//! Run-level replicates are collapsed to task medians. Tasks are then grouped into explicitly
//! declared families/environments and reduced to family medians. Bootstrap resampling operates
//! on family clusters, not individual tasks or runs. This avoids pseudo-replication when several
//! benchmark tasks share one geometry/physics environment. This module does not authenticate
//! evidence receipts or convert synthetic tasks into CFD data.

use std::collections::{BTreeMap, BTreeSet};

use crate::policy_benchmark::PolicyBenchmarkRow;

const MINIMUM_FAMILIES: usize = 5;
const MINIMUM_REPLICATES: usize = 2;
const MINIMUM_BOOTSTRAP_SAMPLES: usize = 1_000;
const MAXIMUM_BOOTSTRAP_SAMPLES: usize = 1_000_000;
const DELTA_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyAggregateConfig {
    pub baseline_policy_id: String,
    pub cost_unit_basis: String,
    /// Frozen task_id -> independent family/environment mapping.
    pub task_family_by_id: BTreeMap<String, String>,
    pub bootstrap_samples: usize,
    /// Non-zero seed for deterministic family-cluster bootstrap resampling.
    pub bootstrap_seed: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyAggregateRow {
    pub policy_id: String,
    pub baseline_policy_id: String,
    pub task_count: usize,
    pub family_count: usize,
    pub replicates_per_task: usize,
    pub mean_family_median_normalized_hypervolume: f64,
    pub median_family_median_normalized_hypervolume: f64,
    pub p10_family_median_normalized_hypervolume: f64,
    pub worst_family_normalized_hypervolume: f64,
    pub mean_paired_delta_vs_baseline: f64,
    pub median_paired_delta_vs_baseline: f64,
    pub p10_paired_delta_vs_baseline: f64,
    pub worst_family_delta_vs_baseline: f64,
    pub families_better: usize,
    pub families_tied: usize,
    pub families_worse: usize,
    pub bootstrap_95_ci_low: f64,
    pub bootstrap_95_ci_high: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyAggregateReport {
    pub baseline_policy_id: String,
    pub cost_unit_basis: String,
    pub task_count: usize,
    pub family_count: usize,
    pub task_family_mapping_sha256: String,
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
        if replicate_matrix
            .insert(row.policy_id.clone(), score)
            .is_some()
        {
            return Err(format!(
                "duplicate task/replicate/policy scorecard row: {}/{}/{}",
                row.task_id, row.replicate_id, row.policy_id
            ));
        }
        tasks.insert(row.task_id.clone());
        replicates.insert(row.replicate_id.clone());
        policies.insert(row.policy_id.clone());
    }

    if replicates.len() < MINIMUM_REPLICATES {
        return Err(format!(
            "at least {MINIMUM_REPLICATES} independent replicates per task are required"
        ));
    }
    if policies.len() < 2 || !policies.contains(&config.baseline_policy_id) {
        return Err(
            "at least two policies and the declared baseline policy are required".to_owned(),
        );
    }

    let mapped_tasks = config
        .task_family_by_id
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    if mapped_tasks != tasks {
        return Err("task-family mapping must contain exactly every benchmark task ID".to_owned());
    }
    if config
        .task_family_by_id
        .values()
        .any(|family| family.trim().is_empty())
    {
        return Err("task-family IDs must be non-empty".to_owned());
    }
    let families = config
        .task_family_by_id
        .values()
        .cloned()
        .collect::<BTreeSet<_>>();
    if families.len() < MINIMUM_FAMILIES {
        return Err(format!(
            "at least {MINIMUM_FAMILIES} independent task families/environments are required; found {}",
            families.len()
        ));
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

    // Stage 1: reduce independent seeds to one median score per task and policy.
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

    // Stage 2: reduce related tasks to an equal-weight family/environment median.
    let mut task_values_by_family = BTreeMap::<String, BTreeMap<String, Vec<f64>>>::new();
    for task in &tasks {
        let family = &config.task_family_by_id[task];
        for (policy, score) in &task_scores[task] {
            task_values_by_family
                .entry(family.clone())
                .or_default()
                .entry(policy.clone())
                .or_default()
                .push(*score);
        }
    }
    let mut family_scores = BTreeMap::<String, BTreeMap<String, f64>>::new();
    for (family, policy_values) in task_values_by_family {
        let scores = policy_values
            .into_iter()
            .map(|(policy, values)| (policy, median(values)))
            .collect();
        family_scores.insert(family, scores);
    }

    let canonical_mapping = config
        .task_family_by_id
        .iter()
        .map(|(task, family)| format!("{task}\t{family}\n"))
        .collect::<String>();
    let task_family_mapping_sha256 = crate::integrity::sha256_hex(canonical_mapping.as_bytes())?;

    let baseline_scores = families
        .iter()
        .map(|family| family_scores[family][&config.baseline_policy_id])
        .collect::<Vec<_>>();
    let mut output_rows = Vec::with_capacity(policies.len());

    for policy in &policies {
        let mut scores = Vec::with_capacity(families.len());
        let mut deltas = Vec::with_capacity(families.len());
        let mut families_better = 0;
        let mut families_tied = 0;
        let mut families_worse = 0;

        for (index, family) in families.iter().enumerate() {
            let score = family_scores[family][policy];
            scores.push(score);
            let delta = score - baseline_scores[index];
            deltas.push(delta);
            if delta > DELTA_TOLERANCE {
                families_better += 1;
            } else if delta < -DELTA_TOLERANCE {
                families_worse += 1;
            } else {
                families_tied += 1;
            }
        }

        let seed = config.bootstrap_seed ^ stable_policy_hash(policy);
        let ci = bootstrap_mean_interval(&deltas, config.bootstrap_samples, seed);
        output_rows.push(PolicyAggregateRow {
            policy_id: policy.clone(),
            baseline_policy_id: config.baseline_policy_id.clone(),
            task_count: tasks.len(),
            family_count: families.len(),
            replicates_per_task: replicates.len(),
            mean_family_median_normalized_hypervolume: mean(&scores),
            median_family_median_normalized_hypervolume: median(scores.clone()),
            p10_family_median_normalized_hypervolume: nearest_rank(&scores, 0.10),
            worst_family_normalized_hypervolume: scores
                .iter()
                .copied()
                .fold(f64::INFINITY, f64::min),
            mean_paired_delta_vs_baseline: mean(&deltas),
            median_paired_delta_vs_baseline: median(deltas.clone()),
            p10_paired_delta_vs_baseline: nearest_rank(&deltas, 0.10),
            worst_family_delta_vs_baseline: deltas.iter().copied().fold(f64::INFINITY, f64::min),
            families_better,
            families_tied,
            families_worse,
            bootstrap_95_ci_low: ci.0,
            bootstrap_95_ci_high: ci.1,
        });
    }
    output_rows.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));

    Ok(PolicyAggregateReport {
        baseline_policy_id: config.baseline_policy_id.clone(),
        cost_unit_basis: config.cost_unit_basis.clone(),
        task_count: tasks.len(),
        family_count: families.len(),
        task_family_mapping_sha256,
        bootstrap_samples: config.bootstrap_samples,
        bootstrap_seed: config.bootstrap_seed,
        evidence_level: "family_clustered_scorecard_aggregation_not_new_solver_evidence",
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

pub const POLICY_AGGREGATE_CSV_HEADER: &str = "policy_id,baseline_policy_id,cost_unit_basis,task_family_mapping_sha256,task_count,family_count,replicates_per_task,bootstrap_samples,bootstrap_seed,mean_family_median_normalized_hypervolume,median_family_median_normalized_hypervolume,p10_family_median_normalized_hypervolume,worst_family_normalized_hypervolume,mean_paired_delta_vs_baseline,median_paired_delta_vs_baseline,p10_paired_delta_vs_baseline,worst_family_delta_vs_baseline,families_better,families_tied,families_worse,bootstrap_95_ci_low,bootstrap_95_ci_high";

pub fn report_to_csv(report: &PolicyAggregateReport) -> String {
    let mut output = format!("{POLICY_AGGREGATE_CSV_HEADER}\n");
    for row in &report.rows {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{},{},{},{:.12},{:.12}\n",
            csv_escape(&row.policy_id),
            csv_escape(&row.baseline_policy_id),
            csv_escape(&report.cost_unit_basis),
            report.task_family_mapping_sha256,
            row.task_count,
            row.family_count,
            row.replicates_per_task,
            report.bootstrap_samples,
            report.bootstrap_seed,
            row.mean_family_median_normalized_hypervolume,
            row.median_family_median_normalized_hypervolume,
            row.p10_family_median_normalized_hypervolume,
            row.worst_family_normalized_hypervolume,
            row.mean_paired_delta_vs_baseline,
            row.median_paired_delta_vs_baseline,
            row.p10_paired_delta_vs_baseline,
            row.worst_family_delta_vs_baseline,
            row.families_better,
            row.families_tied,
            row.families_worse,
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
                    (
                        "voi",
                        baseline[task_index] + improvement[task_index] + jitter,
                    ),
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
                        aerodynamic_condition_calls_attempted: 12,
                        aerodynamic_condition_calls_succeeded: 11,
                        aerodynamic_condition_calls_failed: 1,
                        high_fidelity_condition_calls_attempted: 9,
                        high_fidelity_condition_calls_succeeded: 8,
                        high_fidelity_condition_calls_failed: 1,
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
                        validated_hypervolume_per_high_fidelity_condition_call: Some(
                            score * 100.0 / 9.0,
                        ),
                        normalized_hypervolume_per_high_fidelity_condition_call: Some(score / 9.0),
                        evidence_level: "synthetic_policy_scorecard_only",
                    });
                }
            }
        }
        rows
    }

    fn config() -> PolicyAggregateConfig {
        let task_family_by_id = (0..5)
            .map(|index| (format!("task-{index}"), format!("family-{index}")))
            .collect();
        PolicyAggregateConfig {
            baseline_policy_id: "uniform".to_owned(),
            cost_unit_basis: "synthetic_normalized_compute_credit_v0.1".to_owned(),
            task_family_by_id,
            bootstrap_samples: 2_000,
            bootstrap_seed: 20_261_031,
        }
    }

    #[test]
    fn family_clustered_aggregate_pairs_policy_deltas_and_bootstraps_families() {
        let rows = fixture_rows();
        let report = aggregate_policy_benchmark(&rows, &config()).expect("valid task matrix");
        assert_eq!(report.rows.len(), 2);
        let voi = report
            .rows
            .iter()
            .find(|row| row.policy_id == "voi")
            .unwrap();
        assert_eq!(voi.task_count, 5);
        assert_eq!(voi.family_count, 5);
        assert_eq!(voi.replicates_per_task, 3);
        assert_eq!(report.task_family_mapping_sha256.len(), 64);
        assert!(
            report
                .task_family_mapping_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        );
        assert_eq!(voi.families_better, 4);
        assert_eq!(voi.families_worse, 1);
        assert_eq!(voi.families_tied, 0);
        assert!((voi.mean_paired_delta_vs_baseline - 0.032).abs() < 1.0e-12);
        assert!((voi.worst_family_delta_vs_baseline + 0.02).abs() < 1.0e-12);
        assert!(voi.bootstrap_95_ci_low <= voi.mean_paired_delta_vs_baseline);
        assert!(voi.bootstrap_95_ci_high >= voi.mean_paired_delta_vs_baseline);
        let baseline = report
            .rows
            .iter()
            .find(|row| row.policy_id == "uniform")
            .unwrap();
        assert_eq!(baseline.families_tied, 5);
        assert_eq!(baseline.mean_paired_delta_vs_baseline, 0.0);
        assert_eq!(baseline.bootstrap_95_ci_low, 0.0);
        assert_eq!(baseline.bootstrap_95_ci_high, 0.0);
    }

    #[test]
    fn rejects_too_few_families_missing_policy_or_inconsistent_budget_caps() {
        let mut rows = fixture_rows();
        rows.retain(|row| row.task_id != "task-4");
        let mut invalid = config();
        invalid.task_family_by_id.remove("task-4");
        assert!(
            aggregate_policy_benchmark(&rows, &invalid)
                .expect_err("four families are insufficient")
                .contains("at least 5")
        );

        let mut rows = fixture_rows();
        rows.retain(|row| !(row.task_id == "task-2" && row.policy_id == "voi"));
        assert!(
            aggregate_policy_benchmark(&rows, &config())
                .expect_err("missing policy row rejected")
                .contains("incomplete policy matrix")
        );

        let mut rows = fixture_rows();
        rows[0].budget_cap_units = 9.0;
        assert!(
            aggregate_policy_benchmark(&rows, &config())
                .expect_err("different task budget cap rejected")
                .contains("fixed budget cap")
        );
    }

    #[test]
    fn task_family_mapping_and_replicate_configuration_are_mandatory() {
        let rows = fixture_rows();
        let mut invalid = config();
        invalid.task_family_by_id.remove("task-0");
        assert!(
            aggregate_policy_benchmark(&rows, &invalid)
                .expect_err("missing task-family mapping")
                .contains("exactly every benchmark task ID")
        );

        let mut invalid = config();
        invalid
            .task_family_by_id
            .insert("unknown-task".to_owned(), "family-x".to_owned());
        assert!(
            aggregate_policy_benchmark(&rows, &invalid)
                .expect_err("extra family mapping")
                .contains("exactly every benchmark task ID")
        );

        let mut one_seed = rows;
        one_seed.retain(|row| row.replicate_id == "seed-0");
        assert!(
            aggregate_policy_benchmark(&one_seed, &config())
                .expect_err("one seed rejected")
                .contains("at least 2 independent")
        );
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
