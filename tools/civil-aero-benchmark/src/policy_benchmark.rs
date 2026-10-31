//! Evidence-gated policy comparison under matched total compute budgets.
//!
//! Only admitted and feasible candidate observations contribute to the Pareto hypervolume.
//! Every ledger operation contributes cost, including failed solves and tuning/orchestration.
//! This scorecard evaluates caller-supplied admission receipts; it does not authenticate them,
//! independently recompute CFD outputs, or prove experimental validity.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::budget::{LedgerOperation, PolicyBudgetSummary, summarize_matched_budget};

const ADMITTED_STATUS: &str = "admitted";
const NOT_ADMITTED_STATUS: &str = "not_admitted";

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyBenchmarkTask {
    pub task_id: String,
    pub objective_1_name: String,
    pub objective_2_name: String,
    /// Declared lower/ideal corner of the objective box, fixed before policy runs.
    pub ideal_point_1: f64,
    pub ideal_point_2: f64,
    /// Fixed task-specific upper/reference point for two-objective minimization.
    pub reference_point_1: f64,
    pub reference_point_2: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateAdmissionStatus {
    Admitted,
    NotAdmitted,
}

impl CandidateAdmissionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => ADMITTED_STATUS,
            Self::NotAdmitted => NOT_ADMITTED_STATUS,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateObservation {
    pub task_id: String,
    pub replicate_id: String,
    pub policy_id: String,
    pub operation_id: String,
    pub candidate_id: String,
    /// Must equal the SHA-256 of the matching successful high-fidelity ledger operation.
    pub output_sha256: String,
    pub admission_status: CandidateAdmissionStatus,
    /// Digest of the independently produced admission/constraint receipt. This is not a signature.
    pub admission_receipt_sha256: String,
    /// Present only when admission succeeded.
    pub feasible: Option<bool>,
    /// Present only when admission succeeded; both objectives are lower-is-better.
    pub objective_1: Option<f64>,
    pub objective_2: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyBenchmarkRow {
    pub task_id: String,
    pub replicate_id: String,
    pub policy_id: String,
    pub objective_1_name: String,
    pub objective_2_name: String,
    pub ideal_point_1: f64,
    pub ideal_point_2: f64,
    pub reference_point_1: f64,
    pub reference_point_2: f64,
    pub budget_cap_units: f64,
    pub total_cost_units: f64,
    pub remaining_budget_units: f64,
    pub budget_utilization_fraction: f64,
    pub wall_seconds: f64,
    pub cpu_seconds: f64,
    pub operation_count: usize,
    pub successful_operations: usize,
    pub failed_operations: usize,
    pub high_fidelity_attempts: usize,
    pub successful_high_fidelity_operations: usize,
    pub admitted_candidates: usize,
    pub not_admitted_candidates: usize,
    pub feasible_admitted_candidates: usize,
    /// Feasible admitted candidates divided by all high-fidelity attempts, including failures.
    pub feasible_yield: f64,
    /// Admitted candidates divided by all high-fidelity attempts, including failures.
    pub admission_yield: f64,
    pub validated_hypervolume_2d: f64,
    /// Raw hypervolume divided by the task-declared ideal/reference box area.
    pub normalized_validated_hypervolume_fraction: f64,
    pub validated_hypervolume_per_cost: Option<f64>,
    pub normalized_hypervolume_per_cost: Option<f64>,
    pub evidence_level: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyBenchmarkReport {
    pub rows: Vec<PolicyBenchmarkRow>,
}

pub fn evaluate_policy_benchmark(
    tasks: &[PolicyBenchmarkTask],
    operations: &[LedgerOperation],
    candidates: &[CandidateObservation],
) -> Result<PolicyBenchmarkReport, String> {
    if tasks.is_empty() || operations.is_empty() {
        return Err("policy benchmark requires tasks and a non-empty operation ledger".to_owned());
    }

    let mut task_by_id = BTreeMap::new();
    for task in tasks {
        if task.task_id.trim().is_empty()
            || task.objective_1_name.trim().is_empty()
            || task.objective_2_name.trim().is_empty()
            || task.objective_1_name == task.objective_2_name
            || !task.ideal_point_1.is_finite()
            || !task.ideal_point_2.is_finite()
            || !task.reference_point_1.is_finite()
            || !task.reference_point_2.is_finite()
            || task.ideal_point_1 >= task.reference_point_1
            || task.ideal_point_2 >= task.reference_point_2
        {
            return Err("task IDs, distinct objective names, and finite reference points are required".to_owned());
        }
        if task_by_id.insert(task.task_id.clone(), task).is_some() {
            return Err(format!("duplicate benchmark task {}", task.task_id));
        }
    }

    let budget_summaries = summarize_matched_budget(operations)?;
    let mut operation_by_key = BTreeMap::new();
    let mut high_fidelity_attempts = BTreeMap::<(String, String, String), usize>::new();
    let mut successful_high_fidelity = BTreeMap::<(String, String, String), usize>::new();
    let mut seen_operation_keys = HashSet::new();

    for operation in operations {
        if !task_by_id.contains_key(&operation.task_id) {
            return Err(format!("operation references undeclared task {}", operation.task_id));
        }
        let key = (
            operation.task_id.as_str(),
            operation.replicate_id.as_str(),
            operation.policy_id.as_str(),
            operation.operation_id.as_str(),
        );
        if !seen_operation_keys.insert(key) {
            return Err(format!(
                "duplicate operation key: task={} replicate={} policy={} operation={}",
                operation.task_id,
                operation.replicate_id,
                operation.policy_id,
                operation.operation_id
            ));
        }
        operation_by_key.insert(key, operation);
        let group = (
            operation.task_id.clone(),
            operation.replicate_id.clone(),
            operation.policy_id.clone(),
        );
        if operation.operation_kind == "high_fidelity" {
            *high_fidelity_attempts.entry(group.clone()).or_default() += 1;
            if operation.success {
                *successful_high_fidelity.entry(group).or_default() += 1;
            }
        }
    }

    validate_matched_policy_matrix(&budget_summaries)?;

    let mut candidate_by_group =
        BTreeMap::<(String, String, String), Vec<&CandidateObservation>>::new();
    let mut seen_candidate_ids = HashSet::new();
    let mut seen_candidate_operations = HashSet::new();

    for candidate in candidates {
        if candidate.candidate_id.trim().is_empty()
            || candidate.task_id.trim().is_empty()
            || candidate.replicate_id.trim().is_empty()
            || candidate.policy_id.trim().is_empty()
            || candidate.operation_id.trim().is_empty()
        {
            return Err("candidate and group identifiers must be non-empty".to_owned());
        }
        let task = task_by_id.get(&candidate.task_id).ok_or_else(|| {
            format!("candidate references undeclared task {}", candidate.task_id)
        })?;
        let candidate_key = (
            candidate.task_id.as_str(),
            candidate.replicate_id.as_str(),
            candidate.policy_id.as_str(),
            candidate.operation_id.as_str(),
        );
        let operation = operation_by_key.get(&candidate_key).ok_or_else(|| {
            format!(
                "candidate {} has no matching operation ledger row",
                candidate.candidate_id
            )
        })?;
        if operation.operation_kind != "high_fidelity" || !operation.success {
            return Err(format!(
                "candidate {} must bind to a successful high_fidelity operation",
                candidate.candidate_id
            ));
        }
        if candidate.output_sha256 != operation.output_sha256 {
            return Err(format!(
                "candidate {} output SHA-256 differs from its operation ledger row",
                candidate.candidate_id
            ));
        }
        if !is_sha256_hex(&candidate.output_sha256) {
            return Err(format!("candidate {} has a malformed output SHA-256", candidate.candidate_id));
        }
        if !seen_candidate_operations.insert(candidate_key) {
            return Err(format!(
                "multiple candidate records bind to operation {}",
                candidate.operation_id
            ));
        }
        if !seen_candidate_ids.insert((
            candidate.task_id.clone(),
            candidate.replicate_id.clone(),
            candidate.policy_id.clone(),
            candidate.candidate_id.clone(),
        )) {
            return Err(format!("duplicate candidate ID {}", candidate.candidate_id));
        }

        match candidate.admission_status {
            CandidateAdmissionStatus::Admitted => {
                if !is_sha256_hex(&candidate.admission_receipt_sha256) {
                    return Err(format!(
                        "admitted candidate {} requires a 64-character receipt SHA-256",
                        candidate.candidate_id
                    ));
                }
                let feasible = candidate.feasible.ok_or_else(|| {
                    format!("admitted candidate {} has no constraint-feasibility result", candidate.candidate_id)
                })?;
                let objective_1 = candidate.objective_1.ok_or_else(|| {
                    format!("admitted candidate {} has no objective_1", candidate.candidate_id)
                })?;
                let objective_2 = candidate.objective_2.ok_or_else(|| {
                    format!("admitted candidate {} has no objective_2", candidate.candidate_id)
                })?;
                if !objective_1.is_finite() || !objective_2.is_finite() {
                    return Err(format!("admitted candidate {} has non-finite objectives", candidate.candidate_id));
                }
                if feasible
                    && (objective_1 < task.ideal_point_1
                        || objective_2 < task.ideal_point_2
                        || objective_1 > task.reference_point_1
                        || objective_2 > task.reference_point_2)
                {
                    return Err(format!(
                        "feasible candidate {} lies outside task {} reference box",
                        candidate.candidate_id, task.task_id
                    ));
                }
            }
            CandidateAdmissionStatus::NotAdmitted => {
                if candidate.admission_receipt_sha256 != "-"
                    || candidate.feasible.is_some()
                    || candidate.objective_1.is_some()
                    || candidate.objective_2.is_some()
                {
                    return Err(format!(
                        "not-admitted candidate {} must not carry a receipt or usable objective values",
                        candidate.candidate_id
                    ));
                }
            }
        }

        candidate_by_group
            .entry((
                candidate.task_id.clone(),
                candidate.replicate_id.clone(),
                candidate.policy_id.clone(),
            ))
            .or_default()
            .push(candidate);
    }

    // Every successful high-fidelity solve must either be admitted or explicitly not admitted.
    for operation in operations
        .iter()
        .filter(|operation| operation.operation_kind == "high_fidelity" && operation.success)
    {
        let key = (
            operation.task_id.as_str(),
            operation.replicate_id.as_str(),
            operation.policy_id.as_str(),
            operation.operation_id.as_str(),
        );
        if !seen_candidate_operations.contains(&key) {
            return Err(format!(
                "successful high-fidelity operation {} has no candidate admission record",
                operation.operation_id
            ));
        }
    }

    let mut rows = Vec::with_capacity(budget_summaries.len());
    for budget in budget_summaries {
        let task = task_by_id
            .get(&budget.task_id)
            .expect("all budget tasks were checked against task definitions");
        let group = (
            budget.task_id.clone(),
            budget.replicate_id.clone(),
            budget.policy_id.clone(),
        );
        let observations = candidate_by_group.get(&group).cloned().unwrap_or_default();

        let mut admitted_candidates = 0;
        let mut not_admitted_candidates = 0;
        let mut feasible_admitted_candidates = 0;
        let mut feasible_points = Vec::new();

        for candidate in observations {
            match candidate.admission_status {
                CandidateAdmissionStatus::Admitted => {
                    admitted_candidates += 1;
                    if candidate.feasible == Some(true) {
                        feasible_admitted_candidates += 1;
                        feasible_points.push((
                            candidate.objective_1.expect("admitted objective checked"),
                            candidate.objective_2.expect("admitted objective checked"),
                        ));
                    }
                }
                CandidateAdmissionStatus::NotAdmitted => not_admitted_candidates += 1,
            }
        }

        let hf_attempts = high_fidelity_attempts.get(&group).copied().unwrap_or(0);
        let successful_hf = successful_high_fidelity.get(&group).copied().unwrap_or(0);
        if hf_attempts == 0 {
            return Err(format!(
                "task {} replicate {} policy {} has no high-fidelity attempts",
                budget.task_id, budget.replicate_id, budget.policy_id
            ));
        }
        if successful_hf != admitted_candidates + not_admitted_candidates {
            return Err(format!(
                "candidate disposition count does not match successful high-fidelity operations for task {} replicate {} policy {}",
                budget.task_id, budget.replicate_id, budget.policy_id
            ));
        }

        let validated_hypervolume_2d = hypervolume_2d(
            &feasible_points,
            task.reference_point_1,
            task.reference_point_2,
        )?;
        let feasible_yield = feasible_admitted_candidates as f64 / hf_attempts as f64;
        let admission_yield = admitted_candidates as f64 / hf_attempts as f64;
        let objective_box_area = (task.reference_point_1 - task.ideal_point_1)
            * (task.reference_point_2 - task.ideal_point_2);
        if !objective_box_area.is_finite() || objective_box_area <= 0.0 {
            return Err(format!("task {} has invalid objective reference-box area", task.task_id));
        }
        let normalized_hypervolume_fraction =
            validated_hypervolume_2d / objective_box_area;
        if !normalized_hypervolume_fraction.is_finite()
            || !(-1.0e-12..=1.0 + 1.0e-12).contains(&normalized_hypervolume_fraction)
        {
            return Err(format!("task {} normalized hypervolume is outside [0, 1]", task.task_id));
        }

        rows.push(PolicyBenchmarkRow {
            task_id: budget.task_id,
            replicate_id: budget.replicate_id,
            policy_id: budget.policy_id,
            objective_1_name: task.objective_1_name.clone(),
            objective_2_name: task.objective_2_name.clone(),
            ideal_point_1: task.ideal_point_1,
            ideal_point_2: task.ideal_point_2,
            reference_point_1: task.reference_point_1,
            reference_point_2: task.reference_point_2,
            budget_cap_units: budget.budget_cap_units,
            total_cost_units: budget.total_cost_units,
            remaining_budget_units: budget.remaining_budget_units,
            budget_utilization_fraction: budget.total_cost_units / budget.budget_cap_units,
            wall_seconds: budget.wall_seconds,
            cpu_seconds: budget.cpu_seconds,
            operation_count: budget.operation_count,
            successful_operations: budget.successful_operations,
            failed_operations: budget.failed_operations,
            high_fidelity_attempts: hf_attempts,
            successful_high_fidelity_operations: successful_hf,
            admitted_candidates,
            not_admitted_candidates,
            feasible_admitted_candidates,
            feasible_yield,
            admission_yield,
            validated_hypervolume_2d,
            normalized_validated_hypervolume_fraction: normalized_hypervolume_fraction,
            validated_hypervolume_per_cost: if budget.total_cost_units > 0.0 {
                Some(validated_hypervolume_2d / budget.total_cost_units)
            } else {
                None
            },
            normalized_hypervolume_per_cost: if budget.total_cost_units > 0.0 {
                Some(normalized_hypervolume_fraction / budget.total_cost_units)
            } else {
                None
            },
            evidence_level: "caller_bound_admission_receipt_scorecard_not_solver_validation",
        });
    }

    Ok(PolicyBenchmarkReport { rows })
}

fn validate_matched_policy_matrix(budgets: &[PolicyBudgetSummary]) -> Result<(), String> {
    let mut policy_sets = BTreeMap::<String, BTreeSet<String>>::new();
    let mut replicate_sets =
        BTreeMap::<(String, String), BTreeSet<String>>::new();
    for budget in budgets {
        policy_sets
            .entry(budget.task_id.clone())
            .and_modify(|policies| {
                policies.insert(budget.policy_id.clone());
            })
            .or_insert_with(|| BTreeSet::from([budget.policy_id.clone()]));
        replicate_sets
            .entry((budget.task_id.clone(), budget.policy_id.clone()))
            .or_default()
            .insert(budget.replicate_id.clone());
    }

    for (task_id, policies) in &policy_sets {
        if policies.len() < 2 {
            return Err(format!("task {task_id} requires at least two matched policies"));
        }
        let first_policy = policies.iter().next().expect("at least one policy");
        let reference_replicates = replicate_sets
            .get(&(task_id.clone(), first_policy.clone()))
            .expect("policy replicate set exists");
        if reference_replicates.len() < 2 {
            return Err(format!(
                "task {task_id} requires at least two independent replicates per policy"
            ));
        }
        let mut expected_replicates: Option<&BTreeSet<String>> = None;
        for policy_id in policies {
            let replicates = replicate_sets
                .get(&(task_id.clone(), policy_id.clone()))
                .expect("policy replicate set exists");
            if let Some(expected) = expected_replicates {
                if expected != replicates {
                    return Err(format!(
                        "policy {policy_id} does not cover the same replicate set for task {task_id}"
                    ));
                }
            } else {
                expected_replicates = Some(replicates);
            }
        }
    }
    Ok(())
}

fn hypervolume_2d(
    points: &[(f64, f64)],
    reference_1: f64,
    reference_2: f64,
) -> Result<f64, String> {
    if !reference_1.is_finite() || !reference_2.is_finite() {
        return Err("hypervolume reference point must be finite".to_owned());
    }
    let mut sorted = points.to_vec();
    if sorted.iter().any(|(first, second)| !first.is_finite() || !second.is_finite()) {
        return Err("hypervolume objectives must be finite".to_owned());
    }
    sorted.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.total_cmp(&right.1))
    });

    // For minimization, retain points that improve objective 2 as objective 1 worsens.
    let mut frontier = Vec::new();
    let mut best_objective_2 = f64::INFINITY;
    for point in sorted {
        if point.0 > reference_1 || point.1 > reference_2 {
            return Err("hypervolume point lies outside the declared reference box".to_owned());
        }
        if point.1 < best_objective_2 {
            best_objective_2 = point.1;
            frontier.push(point);
        }
    }

    let mut area = 0.0;
    for (index, point) in frontier.iter().enumerate() {
        let next_objective_1 = frontier
            .get(index + 1)
            .map(|next| next.0)
            .unwrap_or(reference_1);
        let width = next_objective_1 - point.0;
        let height = reference_2 - point.1;
        area += width * height;
    }
    if !area.is_finite() || area < 0.0 {
        return Err("hypervolume calculation produced an invalid area".to_owned());
    }
    Ok(area)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub const POLICY_BENCHMARK_CSV_HEADER: &str = "task_id,replicate_id,policy_id,objective_1_name,objective_2_name,ideal_point_1,ideal_point_2,reference_point_1,reference_point_2,budget_cap_units,total_cost_units,remaining_budget_units,budget_utilization_fraction,wall_seconds,cpu_seconds,operation_count,successful_operations,failed_operations,high_fidelity_attempts,successful_high_fidelity_operations,admitted_candidates,not_admitted_candidates,feasible_admitted_candidates,feasible_yield,admission_yield,validated_hypervolume_2d,normalized_validated_hypervolume_fraction,validated_hypervolume_per_cost,normalized_hypervolume_per_cost,evidence_level";

pub fn report_to_csv(report: &PolicyBenchmarkReport) -> String {
    let mut output = format!("{POLICY_BENCHMARK_CSV_HEADER}\n");
    for row in &report.rows {
        output.push_str(&format!(
            "{},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{},{},{},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{},{},{}\n",
            csv_escape(&row.task_id),
            csv_escape(&row.replicate_id),
            csv_escape(&row.policy_id),
            csv_escape(&row.objective_1_name),
            csv_escape(&row.objective_2_name),
            row.ideal_point_1,
            row.ideal_point_2,
            row.reference_point_1,
            row.reference_point_2,
            row.budget_cap_units,
            row.total_cost_units,
            row.remaining_budget_units,
            row.budget_utilization_fraction,
            row.wall_seconds,
            row.cpu_seconds,
            row.operation_count,
            row.successful_operations,
            row.failed_operations,
            row.high_fidelity_attempts,
            row.successful_high_fidelity_operations,
            row.admitted_candidates,
            row.not_admitted_candidates,
            row.feasible_admitted_candidates,
            row.feasible_yield,
            row.admission_yield,
            row.validated_hypervolume_2d,
            row.normalized_validated_hypervolume_fraction,
            row.validated_hypervolume_per_cost
                .map(|value| format!("{value:.12}"))
                .unwrap_or_default(),
            row.normalized_hypervolume_per_cost
                .map(|value| format!("{value:.12}"))
                .unwrap_or_default(),
            row.evidence_level
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn operation(
        task: &str,
        replicate: &str,
        policy: &str,
        id: &str,
        kind: &str,
        cost: f64,
        success: bool,
        hash: char,
        cap: f64,
    ) -> LedgerOperation {
        LedgerOperation {
            task_id: task.to_owned(),
            replicate_id: replicate.to_owned(),
            policy_id: policy.to_owned(),
            operation_id: id.to_owned(),
            operation_kind: kind.to_owned(),
            fidelity: if kind == "high_fidelity" { "high" } else { "low" }.to_owned(),
            cost_units: cost,
            budget_cap_units: cap,
            wall_seconds: cost * 10.0,
            cpu_seconds: cost * 8.0,
            success,
            output_sha256: if success { digest(hash) } else { "-".to_owned() },
            reason_code: if success { "-".to_owned() } else { "solver_nonconverged".to_owned() },
        }
    }

    fn candidate(
        task: &str,
        replicate: &str,
        policy: &str,
        op: &str,
        id: &str,
        output: char,
        admitted: bool,
        feasible: Option<bool>,
        objective_1: Option<f64>,
        objective_2: Option<f64>,
        receipt: &str,
    ) -> CandidateObservation {
        CandidateObservation {
            task_id: task.to_owned(),
            replicate_id: replicate.to_owned(),
            policy_id: policy.to_owned(),
            operation_id: op.to_owned(),
            candidate_id: id.to_owned(),
            output_sha256: digest(output),
            admission_status: if admitted {
                CandidateAdmissionStatus::Admitted
            } else {
                CandidateAdmissionStatus::NotAdmitted
            },
            admission_receipt_sha256: receipt.to_owned(),
            feasible,
            objective_1,
            objective_2,
        }
    }

    fn fixture() -> (Vec<PolicyBenchmarkTask>, Vec<LedgerOperation>, Vec<CandidateObservation>) {
        let task = PolicyBenchmarkTask {
            task_id: "airfoil-multipoint-smoke".to_owned(),
            objective_1_name: "drag_coefficient".to_owned(),
            objective_2_name: "mission_energy_wh".to_owned(),
            ideal_point_1: 0.0,
            ideal_point_2: 0.0,
            reference_point_1: 0.05,
            reference_point_2: 1000.0,
        };
        let mut operations = Vec::new();
        let mut candidates = Vec::new();
        for replicate in ["seed-1", "seed-2"] {
            for (policy, costs, hashes, points) in [
                (
                    "uniform",
                    [0.5, 2.0, 2.5, 1.0, 0.75],
                    ['a', 'b', 'c', 'd', 'e'],
                    [(0.03, 600.0), (0.035, 500.0)],
                ),
                (
                    "voi",
                    [0.25, 1.75, 2.0, 0.5, 0.5],
                    ['f', 'g', 'h', 'i', 'j'],
                    [(0.028, 650.0), (0.032, 520.0)],
                ),
            ] {
                operations.push(operation("airfoil-multipoint-smoke", replicate, policy, "screen", "low_fidelity", costs[0], true, hashes[0], 8.0));
                operations.push(operation("airfoil-multipoint-smoke", replicate, policy, "hf-1", "high_fidelity", costs[1], true, hashes[1], 8.0));
                operations.push(operation("airfoil-multipoint-smoke", replicate, policy, "hf-2", "high_fidelity", costs[2], true, hashes[2], 8.0));
                operations.push(operation("airfoil-multipoint-smoke", replicate, policy, "hf-rejected", "high_fidelity", costs[3], true, hashes[3], 8.0));
                operations.push(operation("airfoil-multipoint-smoke", replicate, policy, "hf-failed", "high_fidelity", costs[4], false, hashes[4], 8.0));
                candidates.push(candidate("airfoil-multipoint-smoke", replicate, policy, "hf-1", "candidate-1", hashes[1], true, Some(true), Some(points[0].0), Some(points[0].1), &digest('k')));
                candidates.push(candidate("airfoil-multipoint-smoke", replicate, policy, "hf-2", "candidate-2", hashes[2], true, Some(true), Some(points[1].0), Some(points[1].1), &digest('l')));
                candidates.push(candidate("airfoil-multipoint-smoke", replicate, policy, "hf-rejected", "candidate-not-admitted", hashes[3], false, None, None, None, "-"));
            }
        }
        (vec![task], operations, candidates)
    }

    #[test]
    fn matched_budget_scorecard_counts_failures_and_excludes_not_admitted_outputs() {
        let (tasks, operations, candidates) = fixture();
        let report = evaluate_policy_benchmark(&tasks, &operations, &candidates)
            .expect("valid matched policy benchmark");
        assert_eq!(report.rows.len(), 4);
        let uniform = report.rows.iter().find(|row| row.replicate_id == "seed-1" && row.policy_id == "uniform").unwrap();
        let voi = report.rows.iter().find(|row| row.replicate_id == "seed-1" && row.policy_id == "voi").unwrap();
        assert_eq!(uniform.failed_operations, 1);
        assert_eq!(uniform.high_fidelity_attempts, 4);
        assert_eq!(uniform.admitted_candidates, 2);
        assert_eq!(uniform.not_admitted_candidates, 1);
        assert_eq!(uniform.feasible_admitted_candidates, 2);
        assert!((uniform.total_cost_units - 6.75).abs() < 1.0e-12);
        assert!((voi.total_cost_units - 5.0).abs() < 1.0e-12);
        assert!((uniform.validated_hypervolume_2d - 9.5).abs() < 1.0e-10);
        assert!((voi.validated_hypervolume_2d - 10.04).abs() < 1.0e-10);
        assert!((uniform.normalized_validated_hypervolume_fraction - 0.19).abs() < 1.0e-12);
        assert!((voi.normalized_validated_hypervolume_fraction - 0.2008).abs() < 1.0e-12);
        assert!(voi.validated_hypervolume_per_cost.unwrap() > uniform.validated_hypervolume_per_cost.unwrap());
        assert!(voi.normalized_hypervolume_per_cost.unwrap() > uniform.normalized_hypervolume_per_cost.unwrap());
    }

    #[test]
    fn mismatched_output_hash_and_missing_receipt_fail_closed() {
        let (tasks, operations, mut candidates) = fixture();
        candidates[0].output_sha256 = digest('z');
        assert!(evaluate_policy_benchmark(&tasks, &operations, &candidates)
            .expect_err("output hash mismatch must fail")
            .contains("output SHA-256 differs"));

        let (tasks, operations, mut candidates) = fixture();
        candidates[0].admission_receipt_sha256 = "-".to_owned();
        assert!(evaluate_policy_benchmark(&tasks, &operations, &candidates)
            .expect_err("admitted result without receipt must fail")
            .contains("requires a 64-character"));
    }

    #[test]
    fn successful_high_fidelity_operation_requires_disposition_and_reference_box_is_enforced() {
        let (tasks, operations, mut candidates) = fixture();
        candidates.pop();
        assert!(evaluate_policy_benchmark(&tasks, &operations, &candidates)
            .expect_err("unreported successful high-fidelity result must fail")
            .contains("has no candidate admission record"));

        let (tasks, operations, mut candidates) = fixture();
        candidates[0].objective_1 = Some(0.06);
        assert!(evaluate_policy_benchmark(&tasks, &operations, &candidates)
            .expect_err("out-of-reference objective must fail")
            .contains("outside task"));

        let (tasks, operations, mut candidates) = fixture();
        candidates[0].objective_1 = Some(-0.001);
        assert!(evaluate_policy_benchmark(&tasks, &operations, &candidates)
            .expect_err("below-ideal objective must fail")
            .contains("outside task"));
    }

    #[test]
    fn policy_replicate_matrix_must_be_complete_and_identical() {
        let (tasks, mut operations, candidates) = fixture();
        operations.retain(|operation| !(operation.replicate_id == "seed-2" && operation.policy_id == "voi"));
        assert!(evaluate_policy_benchmark(&tasks, &operations, &candidates).is_err());
    }

    #[test]
    fn one_seed_is_not_enough_to_claim_policy_generalization() {
        let (tasks, mut operations, _) = fixture();
        operations.retain(|operation| operation.replicate_id == "seed-1");
        assert!(evaluate_policy_benchmark(&tasks, &operations, &[])
            .expect_err("one seed must not count as policy benchmark")
            .contains("at least two independent replicates"));
    }

    #[test]
    fn hypervolume_ignores_dominated_points_and_is_order_invariant() {
        let a = vec![(1.0, 8.0), (4.0, 3.0), (5.0, 9.0)];
        let b = vec![(5.0, 9.0), (4.0, 3.0), (1.0, 8.0)];
        assert!((hypervolume_2d(&a, 10.0, 10.0).unwrap() - 48.0).abs() < 1.0e-12);
        assert_eq!(hypervolume_2d(&a, 10.0, 10.0), hypervolume_2d(&b, 10.0, 10.0));
    }
}
