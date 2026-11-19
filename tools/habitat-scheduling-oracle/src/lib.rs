// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const FIXTURE_SCHEMA: &str = "luminous.habitat_schedule_micro_oracle.v0.1";
pub const REPORT_SCHEMA: &str = "luminous.habitat_schedule_oracle_report.v0.1";
const CLAIM_CEILING: &str =
    "exhaustive_synthetic_micro_oracle_not_construction_estimates_or_safety_authority";
const MAX_CASES: usize = 64;
const MAX_TASKS_PER_CASE: usize = 12;
const MAX_TIME_BOUND: u32 = 10_000;
const MAX_START_ASSIGNMENT_SPACE: u128 = 2_000_000;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    suite_id: String,
    status: String,
    synthetic_only: bool,
    claim_ceiling: String,
    conventions: Conventions,
    cases: Vec<ScheduleCase>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Conventions {
    time_origin: u32,
    time_unit: String,
    intervals: String,
    tasks: String,
    precedence: String,
    objective: String,
    resources: String,
    uncertainty: String,
    gates: String,
    schedule_is_executed_work: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScheduleCase {
    case_id: String,
    purpose: String,
    tasks: Vec<Task>,
    precedence: Vec<Precedence>,
    resources: Vec<Resource>,
    declared_gates: Vec<String>,
    available_gates: Vec<String>,
    hard_deadline: Option<u32>,
    expected: Expected,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Task {
    id: String,
    nominal_duration: u32,
    upper_duration: u32,
    resource_demands: BTreeMap<String, u32>,
    required_gates: Vec<String>,
    earliest_start: u32,
    latest_start: Option<u32>,
    latest_finish: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Precedence {
    before: String,
    after: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Resource {
    id: String,
    capacity: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WitnessStep {
    pub task_id: String,
    pub start: u32,
    pub finish: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    feasible: bool,
    nominal_optimal_makespan: Option<u32>,
    upper_scenario_optimal_makespan: Option<u32>,
    robust_feasible: bool,
    canonical_witness: Vec<WitnessStep>,
    rejection_reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub feasible: bool,
    pub nominal_optimal_makespan: Option<u32>,
    pub upper_scenario_optimal_makespan: Option<u32>,
    pub robust_feasible: bool,
    pub canonical_witness: Vec<WitnessStep>,
    pub rejection_reason: Option<String>,
    pub expected_outcome_match: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct OracleReport {
    pub schema_version: String,
    pub suite_id: String,
    pub input_sha256: String,
    pub input_bytes: usize,
    pub algorithm: String,
    pub case_count: usize,
    pub all_expected_outcomes_match: bool,
    pub claim_ceiling: String,
    pub case_results: Vec<CaseResult>,
}

#[derive(Clone, Debug)]
struct SolvedSchedule {
    makespan: u32,
    witness: Vec<WitnessStep>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != FIXTURE_SCHEMA {
        return Err(format!("unsupported fixture schema: {}", fixture.schema_version));
    }
    if fixture.suite_id.trim().is_empty()
        || fixture.status.trim().is_empty()
        || !fixture.synthetic_only
        || fixture.claim_ceiling.trim().is_empty()
    {
        return Err("fixture must identify itself and restrict claims to synthetic use".to_owned());
    }
    let c = &fixture.conventions;
    if c.time_origin != 0
        || c.time_unit != "abstract integer time units"
        || c.intervals != "half_open_start_inclusive_finish_exclusive"
        || c.tasks != "nonpreemptive"
        || c.precedence != "finish(before) <= start(after)"
        || c.objective != "minimize makespan = maximum task finish"
        || c.schedule_is_executed_work
    {
        return Err("fixture scheduling conventions are unsupported or ambiguous".to_owned());
    }
    if c.resources.trim().is_empty()
        || c.uncertainty.trim().is_empty()
        || c.gates.trim().is_empty()
    {
        return Err("resource, uncertainty, and gate conventions must be explicit".to_owned());
    }
    if fixture.cases.is_empty() || fixture.cases.len() > MAX_CASES {
        return Err("fixture case count is outside the exhaustive oracle's safe bounds".to_owned());
    }
    let mut case_ids = BTreeSet::new();
    for case in &fixture.cases {
        if case.case_id.trim().is_empty() || !case_ids.insert(case.case_id.clone()) {
            return Err("case ids must be non-empty and unique".to_owned());
        }
        validate_case(case)?;
    }
    Ok(())
}

fn validate_case(case: &ScheduleCase) -> Result<(), String> {
    if case.tasks.is_empty() || case.tasks.len() > MAX_TASKS_PER_CASE {
        return Err(format!("{}: task count exceeds tiny-oracle bounds", case.case_id));
    }
    if case.purpose.trim().is_empty() {
        return Err(format!("{}: case purpose must be specified", case.case_id));
    }
    let mut task_ids = BTreeSet::new();
    let mut resource_ids = BTreeSet::new();
    let mut gate_ids = BTreeSet::new();
    for resource in &case.resources {
        if resource.id.trim().is_empty()
            || resource.capacity == 0
            || !resource_ids.insert(resource.id.as_str())
        {
            return Err(format!("{}: invalid or duplicate resource", case.case_id));
        }
    }
    for gate in &case.declared_gates {
        if gate.trim().is_empty() || !gate_ids.insert(gate.as_str()) {
            return Err(format!("{}: invalid or duplicate declared gate", case.case_id));
        }
    }
    let available: BTreeSet<&str> = case.available_gates.iter().map(String::as_str).collect();
    if available.iter().any(|gate| !gate_ids.contains(gate)) {
        return Err(format!("{}: available gate is not declared", case.case_id));
    }
    for task in &case.tasks {
        if task.id.trim().is_empty()
            || !task_ids.insert(task.id.as_str())
            || task.nominal_duration == 0
            || task.upper_duration < task.nominal_duration
            || task.upper_duration > MAX_TIME_BOUND
            || task.earliest_start > MAX_TIME_BOUND
            || task.latest_start.is_some_and(|value| value > MAX_TIME_BOUND)
            || task.latest_finish.is_some_and(|value| value > MAX_TIME_BOUND)
        {
            return Err(format!("{}: invalid task id, duration, or time bound", case.case_id));
        }
        if task.latest_start.is_some_and(|latest| latest < task.earliest_start) {
            // A contradictory window is valid input only for an infeasibility test case.
            if case.expected.feasible {
                return Err(format!(
                    "{}: feasible case contains a contradictory task window",
                    case.case_id
                ));
            }
        }
        for required_gate in &task.required_gates {
            if !gate_ids.contains(required_gate.as_str()) {
                return Err(format!("{}: task requires an undeclared gate", case.case_id));
            }
        }
        for (resource_id, demand) in &task.resource_demands {
            if !resource_ids.contains(resource_id.as_str()) || *demand == 0 {
                return Err(format!(
                    "{}: task has an unknown resource or zero demand",
                    case.case_id
                ));
            }
        }
    }
    let mut edges = BTreeSet::new();
    for edge in &case.precedence {
        if !task_ids.contains(edge.before.as_str())
            || !task_ids.contains(edge.after.as_str())
            || edge.before == edge.after
            || !edges.insert((edge.before.as_str(), edge.after.as_str()))
        {
            return Err(format!("{}: invalid or duplicate precedence edge", case.case_id));
        }
    }
    if case.hard_deadline.is_some_and(|deadline| deadline > MAX_TIME_BOUND) {
        return Err(format!("{}: hard deadline exceeds tiny-oracle bounds", case.case_id));
    }
    Ok(())
}

fn topological_order(case: &ScheduleCase) -> Option<Vec<usize>> {
    let index_by_id: BTreeMap<&str, usize> = case
        .tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.as_str(), index))
        .collect();
    let mut indegree = vec![0usize; case.tasks.len()];
    let mut next = vec![Vec::new(); case.tasks.len()];
    for edge in &case.precedence {
        let before = *index_by_id.get(edge.before.as_str())?;
        let after = *index_by_id.get(edge.after.as_str())?;
        indegree[after] += 1;
        next[before].push(after);
    }
    let mut ready: VecDeque<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect();
    let mut order = Vec::with_capacity(case.tasks.len());
    while let Some(index) = ready.pop_front() {
        order.push(index);
        for successor in &next[index] {
            indegree[*successor] -= 1;
            if indegree[*successor] == 0 {
                ready.push_back(*successor);
            }
        }
    }
    (order.len() == case.tasks.len()).then_some(order)
}

fn preflight_rejection(case: &ScheduleCase) -> Option<String> {
    let available: BTreeSet<&str> = case.available_gates.iter().map(String::as_str).collect();
    if case
        .tasks
        .iter()
        .flat_map(|task| task.required_gates.iter())
        .any(|gate| !available.contains(gate.as_str()))
    {
        return Some("required_gate_unavailable".to_owned());
    }
    let capacities: BTreeMap<&str, u32> = case
        .resources
        .iter()
        .map(|resource| (resource.id.as_str(), resource.capacity))
        .collect();
    for task in &case.tasks {
        for (resource, demand) in &task.resource_demands {
            if capacities.get(resource.as_str()).is_some_and(|capacity| demand > capacity) {
                return Some("task_demand_exceeds_resource_capacity".to_owned());
            }
        }
    }
    if topological_order(case).is_none() {
        return Some("precedence_cycle".to_owned());
    }
    None
}

fn temporal_conflict(case: &ScheduleCase, durations: &[u32]) -> bool {
    let Some(order) = topological_order(case) else {
        return false;
    };
    let task_index: BTreeMap<&str, usize> = case
        .tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.as_str(), index))
        .collect();
    let mut earliest_finish = vec![0u32; case.tasks.len()];
    for index in order {
        let task = &case.tasks[index];
        let predecessor_finish = case
            .precedence
            .iter()
            .filter(|edge| edge.after == task.id)
            .filter_map(|edge| task_index.get(edge.before.as_str()))
            .map(|prior| earliest_finish[*prior])
            .max()
            .unwrap_or(0);
        let start = task.earliest_start.max(predecessor_finish);
        let finish = start.saturating_add(durations[index]);
        if task.latest_start.is_some_and(|latest| start > latest)
            || task.latest_finish.is_some_and(|latest| finish > latest)
        {
            return true;
        }
        earliest_finish[index] = finish;
    }
    false
}

fn partial_schedule_valid(
    case: &ScheduleCase,
    durations: &[u32],
    starts: &[Option<u32>],
    horizon: u32,
    enforce_deadline: bool,
) -> bool {
    for (index, start) in starts.iter().enumerate() {
        let Some(start) = start else {
            continue;
        };
        let finish = *start + durations[index];
        let task = &case.tasks[index];
        if task.latest_finish.is_some_and(|latest| finish > latest)
            || (enforce_deadline
                && case.hard_deadline.is_some_and(|deadline| finish > deadline))
        {
            return false;
        }
    }

    let task_index: BTreeMap<&str, usize> = case
        .tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.as_str(), index))
        .collect();
    for edge in &case.precedence {
        let before = task_index[edge.before.as_str()];
        let after = task_index[edge.after.as_str()];
        if let (Some(before_start), Some(after_start)) = (starts[before], starts[after]) {
            if before_start + durations[before] > after_start {
                return false;
            }
        }
    }

    for time in 0..horizon {
        for resource in &case.resources {
            let mut used = 0u64;
            for (index, start) in starts.iter().enumerate() {
                let Some(start) = start else {
                    continue;
                };
                let finish = *start + durations[index];
                if *start <= time && time < finish {
                    used += u64::from(
                        case.tasks[index]
                            .resource_demands
                            .get(&resource.id)
                            .copied()
                            .unwrap_or(0),
                    );
                }
            }
            if used > u64::from(resource.capacity) {
                return false;
            }
        }
    }
    true
}

#[allow(clippy::too_many_arguments)]
fn enumerate_schedules(
    case: &ScheduleCase,
    durations: &[u32],
    starts: &mut [Option<u32>],
    index: usize,
    horizon: u32,
    enforce_deadline: bool,
    best: &mut Option<SolvedSchedule>,
) {
    if !partial_schedule_valid(case, durations, starts, horizon, enforce_deadline) {
        return;
    }
    if let Some(current_best) = best {
        let partial_makespan = starts
            .iter()
            .enumerate()
            .filter_map(|(i, start)| start.map(|start| start + durations[i]))
            .max()
            .unwrap_or(0);
        if partial_makespan > current_best.makespan {
            return;
        }
    }
    if index == case.tasks.len() {
        let makespan = starts
            .iter()
            .enumerate()
            .map(|(i, start)| start.expect("all start times are assigned") + durations[i])
            .max()
            .unwrap_or(0);
        if best.as_ref().is_none_or(|current| makespan < current.makespan) {
            let witness = starts
                .iter()
                .enumerate()
                .map(|(i, start)| {
                    let start = start.expect("all start times are assigned");
                    WitnessStep {
                        task_id: case.tasks[i].id.clone(),
                        start,
                        finish: start + durations[i],
                    }
                })
                .collect();
            *best = Some(SolvedSchedule { makespan, witness });
        }
        return;
    }

    let task = &case.tasks[index];
    let duration = durations[index];
    let mut max_start = horizon.saturating_sub(duration);
    if let Some(latest) = task.latest_start {
        max_start = max_start.min(latest);
    }
    if let Some(latest_finish) = task.latest_finish {
        if latest_finish < duration {
            return;
        }
        max_start = max_start.min(latest_finish - duration);
    }
    if task.earliest_start > max_start {
        return;
    }
    for start in task.earliest_start..=max_start {
        starts[index] = Some(start);
        enumerate_schedules(
            case,
            durations,
            starts,
            index + 1,
            horizon,
            enforce_deadline,
            best,
        );
        starts[index] = None;
    }
}

fn solve_scenario(
    case: &ScheduleCase,
    upper_duration: bool,
    enforce_deadline: bool,
) -> (Option<SolvedSchedule>, Option<String>) {
    if let Some(reason) = preflight_rejection(case) {
        return (None, Some(reason));
    }
    let durations: Vec<u32> = case
        .tasks
        .iter()
        .map(|task| {
            if upper_duration {
                task.upper_duration
            } else {
                task.nominal_duration
            }
        })
        .collect();
    let mut sum = 0u32;
    let mut max_bound = 0u32;
    for (index, task) in case.tasks.iter().enumerate() {
        sum = match sum.checked_add(durations[index]) {
            Some(value) => value,
            None => return (None, Some("time_arithmetic_overflow".to_owned())),
        };
        max_bound = max_bound
            .max(task.earliest_start)
            .max(task.latest_start.unwrap_or(0))
            .max(task.latest_finish.unwrap_or(0));
    }
    let max_duration = durations.iter().copied().max().unwrap_or(0);
    let Some(horizon) = sum
        .checked_add(max_bound)
        .and_then(|v| v.checked_add(max_duration))
        .and_then(|v| v.checked_add(1))
    else {
        return (None, Some("time_arithmetic_overflow".to_owned()));
    };
    if horizon > MAX_TIME_BOUND {
        return (None, Some("time_horizon_exceeds_tiny_oracle_bounds".to_owned()));
    }

    // The reference oracle refuses large Cartesian search spaces instead of
    // accidentally turning a tiny exact oracle into an unbounded production solver.
    let mut search_space = 1u128;
    for (index, task) in case.tasks.iter().enumerate() {
        let mut max_start = horizon.saturating_sub(durations[index]);
        if let Some(latest) = task.latest_start {
            max_start = max_start.min(latest);
        }
        if let Some(latest_finish) = task.latest_finish {
            if latest_finish < durations[index] {
                return (None, Some("no_feasible_schedule".to_owned()));
            }
            max_start = max_start.min(latest_finish - durations[index]);
        }
        if task.earliest_start > max_start {
            return (None, Some("no_feasible_schedule".to_owned()));
        }
        let domain = u128::from(max_start - task.earliest_start) + 1;
        search_space = search_space.saturating_mul(domain);
        if search_space > MAX_START_ASSIGNMENT_SPACE {
            return (
                None,
                Some("oracle_instance_exceeds_exhaustive_budget".to_owned()),
            );
        }
    }

    let mut starts = vec![None; case.tasks.len()];
    let mut best = None;
    enumerate_schedules(
        case,
        &durations,
        &mut starts,
        0,
        horizon,
        enforce_deadline,
        &mut best,
    );
    if best.is_some() {
        return (best, None);
    }
    if temporal_conflict(case, &durations) {
        return (
            None,
            Some("inspection_window_conflicts_with_precedence".to_owned()),
        );
    }
    if enforce_deadline && case.hard_deadline.is_some() {
        let without_deadline = solve_scenario(case, upper_duration, false).0;
        if without_deadline.is_some() {
            return (None, Some("hard_deadline_infeasible".to_owned()));
        }
    }
    (None, Some("no_feasible_schedule".to_owned()))
}

pub fn evaluate_fixture_bytes(bytes: &[u8]) -> Result<OracleReport, String> {
    let fixture: Fixture = serde_json::from_slice(bytes)
        .map_err(|error| format!("schedule micro-oracle fixture rejected: {error}"))?;
    validate_fixture(&fixture)?;

    let mut case_results = Vec::with_capacity(fixture.cases.len());
    for case in &fixture.cases {
        let (nominal, nominal_rejection) = solve_scenario(case, false, true);
        let (upper, _upper_rejection) = solve_scenario(case, true, false);
        let robust_feasible = upper.as_ref().is_some_and(|schedule| {
            case.hard_deadline
                .is_none_or(|deadline| schedule.makespan <= deadline)
        });
        let feasible = nominal.is_some();
        let nominal_makespan = nominal.as_ref().map(|schedule| schedule.makespan);
        let upper_makespan = upper.as_ref().map(|schedule| schedule.makespan);
        let witness = nominal
            .as_ref()
            .map(|schedule| schedule.witness.clone())
            .unwrap_or_default();
        let rejection_reason = if feasible { None } else { nominal_rejection };

        let expected_match = feasible == case.expected.feasible
            && nominal_makespan == case.expected.nominal_optimal_makespan
            && upper_makespan == case.expected.upper_scenario_optimal_makespan
            && robust_feasible == case.expected.robust_feasible
            && witness == case.expected.canonical_witness
            && rejection_reason == case.expected.rejection_reason;

        case_results.push(CaseResult {
            case_id: case.case_id.clone(),
            feasible,
            nominal_optimal_makespan: nominal_makespan,
            upper_scenario_optimal_makespan: upper_makespan,
            robust_feasible,
            canonical_witness: witness,
            rejection_reason,
            expected_outcome_match: expected_match,
        });
    }
    let all_expected_outcomes_match = case_results
        .iter()
        .all(|result| result.expected_outcome_match);
    Ok(OracleReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        suite_id: fixture.suite_id,
        input_sha256: sha256_hex(bytes),
        input_bytes: bytes.len(),
        algorithm: "exhaustive_integer_start_time_enumeration_with_pairwise_feasibility_checks"
            .to_owned(),
        case_count: case_results.len(),
        all_expected_outcomes_match,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        case_results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] = include_bytes!(
        "../../../docs/research/fixtures/HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json"
    );

    #[test]
    fn all_frozen_cases_match_the_exhaustive_reference_expectations() {
        let report = evaluate_fixture_bytes(FIXTURE).unwrap();
        assert_eq!(report.case_count, 10);
        assert!(report.all_expected_outcomes_match);
        assert!(report
            .case_results
            .iter()
            .all(|result| result.expected_outcome_match));
    }

    #[test]
    fn exact_oracle_finds_schedule_that_priority_baseline_misses() {
        let report = evaluate_fixture_bytes(FIXTURE).unwrap();
        let adversarial = report
            .case_results
            .iter()
            .find(|result| result.case_id == "priority_rules_miss_feasible_schedule")
            .unwrap();
        assert!(adversarial.feasible);
        assert_eq!(adversarial.nominal_optimal_makespan, Some(19));
        assert_eq!(adversarial.upper_scenario_optimal_makespan, Some(19));
        assert!(adversarial.robust_feasible);
        assert!(adversarial.expected_outcome_match);
    }

    #[test]
    fn shared_resource_and_uncertainty_cases_keep_their_distinction() {
        let report = evaluate_fixture_bytes(FIXTURE).unwrap();
        let shared = report
            .case_results
            .iter()
            .find(|result| result.case_id == "independent_tasks_shared_exclusive_crew")
            .unwrap();
        assert_eq!(shared.nominal_optimal_makespan, Some(10));
        let uncertain = report
            .case_results
            .iter()
            .find(|result| result.case_id == "nominally_fast_but_not_upper_scenario_robust")
            .unwrap();
        assert_eq!(uncertain.nominal_optimal_makespan, Some(4));
        assert_eq!(uncertain.upper_scenario_optimal_makespan, Some(8));
        assert!(!uncertain.robust_feasible);
    }

    #[test]
    fn unknown_fixture_fields_fail_closed() {
        let mut value: serde_json::Value = serde_json::from_slice(FIXTURE).unwrap();
        value["oracle_override"] = serde_json::Value::Bool(true);
        assert!(evaluate_fixture_bytes(value.to_string().as_bytes()).is_err());
    }

    #[test]
    fn excessive_cartesian_search_space_is_rejected_not_run_unbounded() {
        let mut value: serde_json::Value = serde_json::from_slice(FIXTURE).unwrap();
        let case = &mut value["cases"][0];
        let tasks: Vec<serde_json::Value> = (0..12)
            .map(|index| {
                serde_json::json!({
                    "id": format!("TASK-{index:02}"),
                    "nominal_duration": 1,
                    "upper_duration": 1,
                    "resource_demands": {},
                    "required_gates": [],
                    "earliest_start": 0,
                    "latest_start": null,
                    "latest_finish": null
                })
            })
            .collect();
        case["tasks"] = serde_json::Value::Array(tasks);
        case["precedence"] = serde_json::json!([]);
        case["resources"] = serde_json::json!([]);
        case["declared_gates"] = serde_json::json!([]);
        case["available_gates"] = serde_json::json!([]);
        case["hard_deadline"] = serde_json::Value::Null;
        let bytes = serde_json::to_vec(&value).unwrap();
        let report = evaluate_fixture_bytes(&bytes).unwrap();
        assert_eq!(
            report.case_results[0].rejection_reason.as_deref(),
            Some("oracle_instance_exceeds_exhaustive_budget")
        );
        assert!(!report.case_results[0].expected_outcome_match);
    }

    #[test]
    fn report_digest_pins_exact_fixture_bytes() {
        let report = evaluate_fixture_bytes(FIXTURE).unwrap();
        assert_eq!(report.input_bytes, FIXTURE.len());
        assert_eq!(report.input_sha256, sha256_hex(FIXTURE));
    }
}
