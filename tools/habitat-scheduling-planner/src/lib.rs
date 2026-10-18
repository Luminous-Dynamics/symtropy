// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const FIXTURE_SCHEMA: &str = "luminous.habitat_schedule_micro_oracle.v0.1";
const REPORT_SCHEMA: &str = "luminous.habitat_schedule_planner_report.v0.1";
const CLAIM_CEILING: &str =
    "synthetic_heuristic_schedule_candidates_not_optimality_construction_or_safety_authority";
const MAX_CASES: usize = 64;
const MAX_TASKS: usize = 128;
const MAX_TIME_BOUND: u32 = 10_000;
const PRIORITY_RULES: usize = 5;
const MAX_FALLBACK_STATES: usize = 20_000;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    suite_id: String,
    status: String,
    synthetic_only: bool,
    claim_ceiling: String,
    conventions: Conventions,
    cases: Vec<Case>,
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
struct Case {
    case_id: String,
    purpose: String,
    tasks: Vec<Task>,
    precedence: Vec<Precedence>,
    resources: Vec<Resource>,
    declared_gates: Vec<String>,
    available_gates: Vec<String>,
    hard_deadline: Option<u32>,
    // The micro-oracle fixture stores expected results for differential tests.
    // The scheduling heuristic deliberately never reads this field.
    #[serde(rename = "expected")]
    _expected: Value,
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ScheduleStep {
    pub task_id: String,
    pub start: u32,
    pub finish: u32,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ScenarioResult {
    pub status: String,
    pub makespan: Option<u32>,
    pub schedule: Vec<ScheduleStep>,
    pub validation_status: String,
    pub rejection_reason: Option<String>,
    pub priority_rules_tried: usize,
    pub priority_only_status: String,
    pub priority_only_makespan: Option<u32>,
    pub fallback_status: String,
    pub fallback_states_expanded: usize,
    pub fallback_orders_evaluated: usize,
    pub fallback_search_complete: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub nominal: ScenarioResult,
    pub upper_duration_unconstrained: ScenarioResult,
    pub upper_duration_with_deadline: ScenarioResult,
    pub robust_status: String,
    pub optimality_claim: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct PlannerReport {
    pub schema_version: String,
    pub suite_id: String,
    pub input_sha256: String,
    pub input_bytes: usize,
    pub algorithm: String,
    pub case_count: usize,
    pub all_returned_schedules_independently_validated: bool,
    pub claim_ceiling: String,
    pub case_results: Vec<CaseResult>,
}

#[derive(Clone, Debug)]
struct Candidate {
    makespan: u32,
    schedule: Vec<ScheduleStep>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut result, "{byte:02x}");
    }
    result
}

fn durations_for(case: &Case, upper: bool) -> Vec<u32> {
    case.tasks
        .iter()
        .map(|task| {
            if upper {
                task.upper_duration
            } else {
                task.nominal_duration
            }
        })
        .collect()
}

fn task_index(case: &Case) -> BTreeMap<&str, usize> {
    case.tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id.as_str(), index))
        .collect()
}

fn validate_case_shape(case: &Case) -> Result<(), String> {
    if case.case_id.trim().is_empty()
        || case.purpose.trim().is_empty()
        || case.tasks.is_empty()
        || case.tasks.len() > MAX_TASKS
    {
        return Err("case identity, purpose, or task count is invalid".to_owned());
    }
    let mut task_ids = BTreeSet::new();
    let mut resource_ids = BTreeSet::new();
    let mut declared_gates = BTreeSet::new();
    for resource in &case.resources {
        if resource.id.trim().is_empty()
            || resource.capacity == 0
            || !resource_ids.insert(resource.id.as_str())
        {
            return Err(format!("{}: invalid resource declaration", case.case_id));
        }
    }
    for gate in &case.declared_gates {
        if gate.trim().is_empty() || !declared_gates.insert(gate.as_str()) {
            return Err(format!("{}: invalid declared gate", case.case_id));
        }
    }
    if case
        .available_gates
        .iter()
        .any(|gate| !declared_gates.contains(gate.as_str()))
    {
        return Err(format!("{}: available gate is undeclared", case.case_id));
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
            return Err(format!("{}: invalid task declaration", case.case_id));
        }
        if task
            .required_gates
            .iter()
            .any(|gate| !declared_gates.contains(gate.as_str()))
        {
            return Err(format!("{}: task references undeclared gate", case.case_id));
        }
        for (resource, demand) in &task.resource_demands {
            if !resource_ids.contains(resource.as_str()) || *demand == 0 {
                return Err(format!("{}: task references invalid resource demand", case.case_id));
            }
        }
    }
    let index = task_index(case);
    let mut edges = BTreeSet::new();
    for edge in &case.precedence {
        if !index.contains_key(edge.before.as_str())
            || !index.contains_key(edge.after.as_str())
            || edge.before == edge.after
            || !edges.insert((edge.before.as_str(), edge.after.as_str()))
        {
            return Err(format!("{}: invalid precedence declaration", case.case_id));
        }
    }
    if case.hard_deadline.is_some_and(|deadline| deadline > MAX_TIME_BOUND) {
        return Err(format!("{}: hard deadline exceeds supported time bound", case.case_id));
    }
    Ok(())
}

fn topological_order(case: &Case) -> Option<Vec<usize>> {
    let indices = task_index(case);
    let mut indegree = vec![0usize; case.tasks.len()];
    let mut successors = vec![Vec::new(); case.tasks.len()];
    for edge in &case.precedence {
        let before = *indices.get(edge.before.as_str())?;
        let after = *indices.get(edge.after.as_str())?;
        indegree[after] += 1;
        successors[before].push(after);
    }
    let mut ready: VecDeque<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect();
    let mut order = Vec::with_capacity(case.tasks.len());
    while let Some(index) = ready.pop_front() {
        order.push(index);
        for successor in &successors[index] {
            indegree[*successor] -= 1;
            if indegree[*successor] == 0 {
                ready.push_back(*successor);
            }
        }
    }
    (order.len() == case.tasks.len()).then_some(order)
}

fn lower_bound_rejection(
    case: &Case,
    durations: &[u32],
    enforce_deadline: bool,
) -> Option<String> {
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
    if case.tasks.iter().any(|task| {
        task.resource_demands
            .iter()
            .any(|(resource, demand)| {
                capacities
                    .get(resource.as_str())
                    .is_some_and(|cap| demand > cap)
            })
    }) {
        return Some("task_demand_exceeds_resource_capacity".to_owned());
    }
    let Some(order) = topological_order(case) else {
        return Some("precedence_cycle".to_owned());
    };
    let indices = task_index(case);
    let mut earliest_finish = vec![0u32; case.tasks.len()];
    for index in order {
        let task = &case.tasks[index];
        let prerequisite_finish = case
            .precedence
            .iter()
            .filter(|edge| edge.after == task.id)
            .filter_map(|edge| indices.get(edge.before.as_str()))
            .map(|prior| earliest_finish[*prior])
            .max()
            .unwrap_or(0);
        let start = task.earliest_start.max(prerequisite_finish);
        let Some(finish) = start.checked_add(durations[index]) else {
            return Some("time_arithmetic_overflow".to_owned());
        };
        if task.latest_start.is_some_and(|latest| start > latest)
            || task.latest_finish.is_some_and(|latest| finish > latest)
        {
            return Some("inspection_window_conflicts_with_precedence".to_owned());
        }
        earliest_finish[index] = finish;
    }
    if enforce_deadline
        && case
            .hard_deadline
            .is_some_and(|deadline| earliest_finish.iter().any(|finish| *finish > deadline))
    {
        return Some("hard_deadline_lower_bound_exceeds_deadline".to_owned());
    }
    None
}

fn critical_path_lengths(case: &Case, durations: &[u32], order: &[usize]) -> Vec<u32> {
    let indices = task_index(case);
    let mut successors = vec![Vec::new(); case.tasks.len()];
    for edge in &case.precedence {
        if let (Some(before), Some(after)) = (
            indices.get(edge.before.as_str()),
            indices.get(edge.after.as_str()),
        ) {
            successors[*before].push(*after);
        }
    }
    let mut critical = vec![0u32; case.tasks.len()];
    for index in order.iter().rev() {
        let tail = successors[*index]
            .iter()
            .map(|next| critical[*next])
            .max()
            .unwrap_or(0);
        critical[*index] = durations[*index].saturating_add(tail);
    }
    critical
}

fn ordered_by_rule(
    case: &Case,
    durations: &[u32],
    rule: usize,
) -> Option<Vec<usize>> {
    let indices = task_index(case);
    let mut indegree = vec![0usize; case.tasks.len()];
    let mut successors = vec![Vec::new(); case.tasks.len()];
    for edge in &case.precedence {
        let before = *indices.get(edge.before.as_str())?;
        let after = *indices.get(edge.after.as_str())?;
        indegree[after] += 1;
        successors[before].push(after);
    }
    let mut ready: Vec<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(index, degree)| (*degree == 0).then_some(index))
        .collect();
    let base_order = topological_order(case)?;
    let critical = critical_path_lengths(case, durations, &base_order);
    let mut order = Vec::with_capacity(case.tasks.len());
    while !ready.is_empty() {
        ready.sort_by(|left, right| {
            let a = &case.tasks[*left];
            let b = &case.tasks[*right];
            let tie = || a.id.cmp(&b.id);
            let primary = match rule {
                0 => Ordering::Equal,
                1 => critical[*right].cmp(&critical[*left]),
                2 => a
                    .latest_start
                    .unwrap_or(u32::MAX)
                    .cmp(&b.latest_start.unwrap_or(u32::MAX)),
                3 => durations[*right].cmp(&durations[*left]),
                4 => durations[*left].cmp(&durations[*right]),
                _ => Ordering::Equal,
            };
            if primary == Ordering::Equal {
                tie()
            } else {
                primary
            }
        });
        let chosen = ready.remove(0);
        order.push(chosen);
        for successor in &successors[chosen] {
            indegree[*successor] -= 1;
            if indegree[*successor] == 0 {
                ready.push(*successor);
            }
        }
    }
    (order.len() == case.tasks.len()).then_some(order)
}

fn resource_fits(
    case: &Case,
    task_idx: usize,
    start: u32,
    finish: u32,
    existing: &[ScheduleStep],
) -> bool {
    let task = &case.tasks[task_idx];
    let indexed = task_index(case);
    let scheduled_by_id: BTreeMap<&str, &ScheduleStep> = existing
        .iter()
        .map(|step| (step.task_id.as_str(), step))
        .collect();
    let capacities: BTreeMap<&str, u32> = case
        .resources
        .iter()
        .map(|resource| (resource.id.as_str(), resource.capacity))
        .collect();

    for resource in task.resource_demands.keys() {
        let capacity = capacities[resource.as_str()];
        for time in start..finish {
            let mut used = u64::from(task.resource_demands[resource]);
            for (id, step) in &scheduled_by_id {
                let index = indexed[*id];
                if step.start <= time && time < step.finish {
                    used += u64::from(
                        case.tasks[index]
                            .resource_demands
                            .get(resource)
                            .copied()
                            .unwrap_or(0),
                    );
                }
            }
            if used > u64::from(capacity) {
                return false;
            }
        }
    }
    // Existing tasks using resources absent from this task's demand map do not
    // conflict with it because this task consumes none of that resource.
    true
}

fn independently_validate_schedule(
    case: &Case,
    schedule: &[ScheduleStep],
    durations: &[u32],
    enforce_deadline: bool,
) -> Result<(), String> {
    validate_case_shape(case)?;
    if schedule.len() != case.tasks.len() || durations.len() != case.tasks.len() {
        return Err("schedule omits required tasks or duration entries".to_owned());
    }
    let mut steps = BTreeMap::new();
    for step in schedule {
        if steps.insert(step.task_id.as_str(), step).is_some() {
            return Err(format!("duplicate scheduled task {}", step.task_id));
        }
    }
    let indices = task_index(case);
    for (index, task) in case.tasks.iter().enumerate() {
        let step = steps
            .get(task.id.as_str())
            .ok_or_else(|| format!("task {} missing from schedule", task.id))?;
        if step.finish.checked_sub(step.start) != Some(durations[index])
            || step.start < task.earliest_start
            || task.latest_start.is_some_and(|latest| step.start > latest)
            || task.latest_finish.is_some_and(|latest| step.finish > latest)
        {
            return Err(format!("task {} violates duration or time window", task.id));
        }
        if enforce_deadline && case.hard_deadline.is_some_and(|deadline| step.finish > deadline) {
            return Err(format!("task {} violates hard deadline", task.id));
        }
    }
    for edge in &case.precedence {
        let before = steps[edge.before.as_str()];
        let after = steps[edge.after.as_str()];
        if before.finish > after.start {
            return Err(format!(
                "precedence violation: {} must finish before {} starts",
                edge.before, edge.after
            ));
        }
    }
    let available: BTreeSet<&str> = case.available_gates.iter().map(String::as_str).collect();
    if case
        .tasks
        .iter()
        .flat_map(|task| task.required_gates.iter())
        .any(|gate| !available.contains(gate.as_str()))
    {
        return Err("required gate unavailable".to_owned());
    }
    let capacities: BTreeMap<&str, u32> = case
        .resources
        .iter()
        .map(|resource| (resource.id.as_str(), resource.capacity))
        .collect();
    let horizon = schedule.iter().map(|step| step.finish).max().unwrap_or(0);
    for time in 0..horizon {
        for resource in &case.resources {
            let mut used = 0u64;
            for step in schedule {
                if step.start <= time && time < step.finish {
                    let index = indices[step.task_id.as_str()];
                    used += u64::from(
                        case.tasks[index]
                            .resource_demands
                            .get(&resource.id)
                            .copied()
                            .unwrap_or(0),
                    );
                }
            }
            if used > u64::from(capacities[resource.id.as_str()]) {
                return Err(format!("resource {} over capacity at time {time}", resource.id));
            }
        }
    }
    Ok(())
}

fn schedule_one_order(
    case: &Case,
    durations: &[u32],
    order: &[usize],
    enforce_deadline: bool,
) -> Option<Candidate> {
    let indices = task_index(case);
    let horizon_bound = durations.iter().copied().try_fold(0u32, u32::checked_add)?;
    let latest_bound = case
        .tasks
        .iter()
        .flat_map(|task| {
            [
                task.earliest_start,
                task.latest_start.unwrap_or(0),
                task.latest_finish.unwrap_or(0),
            ]
        })
        .max()
        .unwrap_or(0);
    let max_duration = durations.iter().copied().max().unwrap_or(0);
    let horizon = horizon_bound
        .checked_add(latest_bound)?
        .checked_add(max_duration)?
        .checked_add(1)?;
    if horizon > MAX_TIME_BOUND {
        return None;
    }

    let mut schedule: Vec<ScheduleStep> = Vec::with_capacity(case.tasks.len());
    let mut finish_by_id: BTreeMap<&str, u32> = BTreeMap::new();
    for index in order {
        let task = &case.tasks[*index];
        let prerequisite_finish = case
            .precedence
            .iter()
            .filter(|edge| edge.after == task.id)
            .filter_map(|edge| finish_by_id.get(edge.before.as_str()).copied())
            .max()
            .unwrap_or(0);
        let earliest = task.earliest_start.max(prerequisite_finish);
        let duration = durations[*index];
        let mut latest_start = horizon.saturating_sub(duration);
        if let Some(latest) = task.latest_start {
            latest_start = latest_start.min(latest);
        }
        if let Some(latest_finish) = task.latest_finish {
            if latest_finish < duration {
                return None;
            }
            latest_start = latest_start.min(latest_finish - duration);
        }
        if enforce_deadline {
            if let Some(deadline) = case.hard_deadline {
                if deadline < duration {
                    return None;
                }
                latest_start = latest_start.min(deadline - duration);
            }
        }
        let mut placed = None;
        if earliest <= latest_start {
            for start in earliest..=latest_start {
                let Some(finish) = start.checked_add(duration) else {
                    return None;
                };
                if resource_fits(case, *index, start, finish, &schedule) {
                    placed = Some(ScheduleStep {
                        task_id: task.id.clone(),
                        start,
                        finish,
                    });
                    break;
                }
            }
        }
        let step = placed?;
        finish_by_id.insert(task.id.as_str(), step.finish);
        schedule.push(step);
    }
    schedule.sort();
    independently_validate_schedule(case, &schedule, durations, enforce_deadline).ok()?;
    let makespan = schedule.iter().map(|step| step.finish).max().unwrap_or(0);
    Some(Candidate { makespan, schedule })
}

fn candidate_key(candidate: &Candidate) -> (u32, Vec<ScheduleStep>) {
    (candidate.makespan, candidate.schedule.clone())
}

#[derive(Default)]
struct FallbackSearchState {
    states_expanded: usize,
    orders_evaluated: usize,
    budget_exhausted: bool,
    best: Option<Candidate>,
}

fn enumerate_topological_orders(
    case: &Case,
    durations: &[u32],
    enforce_deadline: bool,
    prefix: &mut Vec<usize>,
    selected: &mut [bool],
    indegree: &mut [usize],
    successors: &[Vec<usize>],
    state: &mut FallbackSearchState,
) {
    if state.states_expanded >= MAX_FALLBACK_STATES {
        state.budget_exhausted = true;
        return;
    }
    state.states_expanded += 1;
    if prefix.len() == case.tasks.len() {
        state.orders_evaluated += 1;
        if let Some(candidate) = schedule_one_order(case, durations, prefix, enforce_deadline) {
            if state
                .best
                .as_ref()
                .is_none_or(|existing| candidate_key(&candidate) < candidate_key(existing))
            {
                state.best = Some(candidate);
            }
        }
        return;
    }

    let mut ready: Vec<usize> = (0..case.tasks.len())
        .filter(|index| !selected[*index] && indegree[*index] == 0)
        .collect();
    ready.sort_by(|left, right| case.tasks[*left].id.cmp(&case.tasks[*right].id));

    for index in ready {
        selected[index] = true;
        prefix.push(index);
        for child in &successors[index] {
            indegree[*child] -= 1;
        }
        enumerate_topological_orders(
            case,
            durations,
            enforce_deadline,
            prefix,
            selected,
            indegree,
            successors,
            state,
        );
        for child in &successors[index] {
            indegree[*child] += 1;
        }
        prefix.pop();
        selected[index] = false;
        if state.budget_exhausted {
            break;
        }
    }
}

fn bounded_topological_order_fallback(
    case: &Case,
    durations: &[u32],
    enforce_deadline: bool,
) -> FallbackSearchState {
    let indices = task_index(case);
    let mut indegree = vec![0usize; case.tasks.len()];
    let mut successors = vec![Vec::new(); case.tasks.len()];
    for edge in &case.precedence {
        let before = indices[edge.before.as_str()];
        let after = indices[edge.after.as_str()];
        indegree[after] += 1;
        successors[before].push(after);
    }
    let mut state = FallbackSearchState::default();
    enumerate_topological_orders(
        case,
        durations,
        enforce_deadline,
        &mut Vec::with_capacity(case.tasks.len()),
        &mut vec![false; case.tasks.len()],
        &mut indegree,
        &successors,
        &mut state,
    );
    state
}

fn solve_scenario(case: &Case, upper: bool, enforce_deadline: bool) -> ScenarioResult {
    let durations = durations_for(case, upper);
    if let Some(reason) = lower_bound_rejection(case, &durations, enforce_deadline) {
        return ScenarioResult {
            status: "PROVEN_INFEASIBLE_PRECHECK".to_owned(),
            makespan: None,
            schedule: Vec::new(),
            validation_status: "NOT_APPLICABLE_NO_CANDIDATE".to_owned(),
            rejection_reason: Some(reason),
            priority_rules_tried: 0,
            priority_only_status: "PROVEN_INFEASIBLE_PRECHECK".to_owned(),
            priority_only_makespan: None,
            fallback_status: "NOT_RUN_PRECHECK_PROVEN".to_owned(),
            fallback_states_expanded: 0,
            fallback_orders_evaluated: 0,
            fallback_search_complete: true,
        };
    }

    let mut best: Option<Candidate> = None;
    let mut tried = 0;
    for rule in 0..PRIORITY_RULES {
        let Some(order) = ordered_by_rule(case, &durations, rule) else {
            continue;
        };
        tried += 1;
        if let Some(candidate) = schedule_one_order(case, &durations, &order, enforce_deadline) {
            if best
                .as_ref()
                .is_none_or(|existing| candidate_key(&candidate) < candidate_key(existing))
            {
                best = Some(candidate);
            }
        }
    }
    if let Some(candidate) = best {
        let validation = independently_validate_schedule(
            case,
            &candidate.schedule,
            &durations,
            enforce_deadline,
        );
        if validation.is_err() {
            return ScenarioResult {
                status: "INTERNAL_VALIDATOR_DISAGREEMENT".to_owned(),
                makespan: None,
                schedule: Vec::new(),
                validation_status: "FAILED".to_owned(),
                rejection_reason: validation.err(),
                priority_rules_tried: tried,
                priority_only_status: "INTERNAL_VALIDATOR_DISAGREEMENT".to_owned(),
                priority_only_makespan: None,
                fallback_status: "NOT_RUN_VALIDATOR_FAILURE".to_owned(),
                fallback_states_expanded: 0,
                fallback_orders_evaluated: 0,
                fallback_search_complete: true,
            };
        }
        return ScenarioResult {
            status: "HEURISTIC_FEASIBLE_NOT_PROVEN_OPTIMAL".to_owned(),
            makespan: Some(candidate.makespan),
            schedule: candidate.schedule,
            validation_status: "INDEPENDENT_VALIDATOR_PASSED".to_owned(),
            rejection_reason: None,
            priority_rules_tried: tried,
            priority_only_status: "HEURISTIC_FEASIBLE_NOT_PROVEN_OPTIMAL".to_owned(),
            priority_only_makespan: Some(candidate.makespan),
            fallback_status: "NOT_NEEDED_PRIORITY_CANDIDATE_FOUND".to_owned(),
            fallback_states_expanded: 0,
            fallback_orders_evaluated: 0,
            fallback_search_complete: true,
        };
    }

    let priority_only_status = "NO_CANDIDATE_FOUND_NOT_PROOF_OF_INFEASIBILITY".to_owned();
    let fallback = bounded_topological_order_fallback(case, &durations, enforce_deadline);
    if let Some(candidate) = fallback.best {
        let validation = independently_validate_schedule(
            case,
            &candidate.schedule,
            &durations,
            enforce_deadline,
        );
        if let Err(error) = validation {
            return ScenarioResult {
                status: "INTERNAL_VALIDATOR_DISAGREEMENT".to_owned(),
                makespan: None,
                schedule: Vec::new(),
                validation_status: "FAILED".to_owned(),
                rejection_reason: Some(error),
                priority_rules_tried: tried,
                priority_only_status,
                priority_only_makespan: None,
                fallback_status: "CANDIDATE_REJECTED_BY_INDEPENDENT_VALIDATOR".to_owned(),
                fallback_states_expanded: fallback.states_expanded,
                fallback_orders_evaluated: fallback.orders_evaluated,
                fallback_search_complete: !fallback.budget_exhausted,
            };
        }
        return ScenarioResult {
            status: "BOUNDED_FALLBACK_FEASIBLE_NOT_PROVEN_OPTIMAL".to_owned(),
            makespan: Some(candidate.makespan),
            schedule: candidate.schedule,
            validation_status: "INDEPENDENT_VALIDATOR_PASSED".to_owned(),
            rejection_reason: None,
            priority_rules_tried: tried,
            priority_only_status,
            priority_only_makespan: None,
            fallback_status: if fallback.budget_exhausted {
                "CANDIDATE_FOUND_BUDGET_EXHAUSTED".to_owned()
            } else {
                "CANDIDATE_FOUND_SEARCH_COMPLETE_NOT_OPTIMAL_PROVEN".to_owned()
            },
            fallback_states_expanded: fallback.states_expanded,
            fallback_orders_evaluated: fallback.orders_evaluated,
            fallback_search_complete: !fallback.budget_exhausted,
        };
    }

    ScenarioResult {
        status: if fallback.budget_exhausted {
            "FALLBACK_SEARCH_INCOMPLETE_NO_CANDIDATE_NOT_PROOF_OF_INFEASIBILITY".to_owned()
        } else {
            "FALLBACK_EXHAUSTED_NO_CANDIDATE_NOT_PROOF_OF_INFEASIBILITY".to_owned()
        },
        makespan: None,
        schedule: Vec::new(),
        validation_status: "NOT_APPLICABLE_NO_CANDIDATE".to_owned(),
        rejection_reason: Some(if fallback.budget_exhausted {
            "bounded topological-order fallback exhausted its state budget".to_owned()
        } else {
            "bounded topological-order fallback found no candidate".to_owned()
        }),
        priority_rules_tried: tried,
        priority_only_status,
        priority_only_makespan: None,
        fallback_status: if fallback.budget_exhausted {
            "BUDGET_EXHAUSTED_NO_CANDIDATE".to_owned()
        } else {
            "SEARCH_COMPLETE_NO_CANDIDATE_NOT_PROOF_OF_INFEASIBILITY".to_owned()
        },
        fallback_states_expanded: fallback.states_expanded,
        fallback_orders_evaluated: fallback.orders_evaluated,
        fallback_search_complete: !fallback.budget_exhausted,
    }
}

pub fn plan_fixture_bytes(bytes: &[u8]) -> Result<PlannerReport, String> {
    let fixture: Fixture = serde_json::from_slice(bytes)
        .map_err(|error| format!("scheduling planner input rejected: {error}"))?;
    if fixture.schema_version != FIXTURE_SCHEMA
        || fixture.suite_id.trim().is_empty()
        || fixture.status.trim().is_empty()
        || !fixture.synthetic_only
        || fixture.claim_ceiling.trim().is_empty()
        || fixture.conventions.time_origin != 0
        || fixture.conventions.time_unit != "abstract integer time units"
        || fixture.conventions.intervals != "half_open_start_inclusive_finish_exclusive"
        || fixture.conventions.tasks != "nonpreemptive"
        || fixture.conventions.precedence != "finish(before) <= start(after)"
        || fixture.conventions.objective != "minimize makespan = maximum task finish"
        || fixture.conventions.resources
            != "renewable capacity; a task's demand must not exceed capacity at any occupied time"
        || fixture.conventions.uncertainty
            != concat!(
                "nominal and upper_duration are disclosed scenarios; robust feasibility checks ",
                "the upper scenario against the same hard constraints"
            )
        || fixture.conventions.gates
            != "required gates are hard feasibility constraints, not soft penalties"
        || fixture.conventions.schedule_is_executed_work
        || fixture.cases.is_empty()
        || fixture.cases.len() > MAX_CASES
    {
        return Err("unsupported, ambiguous, or out-of-bounds fixture".to_owned());
    }
    let mut case_ids = BTreeSet::new();
    for case in &fixture.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("fixture case IDs must be unique".to_owned());
        }
        validate_case_shape(case)?;
    }

    let mut results = Vec::with_capacity(fixture.cases.len());
    let mut every_schedule_valid = true;
    for case in &fixture.cases {
        let nominal = solve_scenario(case, false, true);
        let upper_unconstrained = solve_scenario(case, true, false);
        let upper_with_deadline = solve_scenario(case, true, true);
        every_schedule_valid &= [
            &nominal,
            &upper_unconstrained,
            &upper_with_deadline,
        ]
        .iter()
        .all(|result| {
            if result.schedule.is_empty() {
                result.status != "INTERNAL_VALIDATOR_DISAGREEMENT"
                    && result.validation_status == "NOT_APPLICABLE_NO_CANDIDATE"
            } else {
                result.validation_status == "INDEPENDENT_VALIDATOR_PASSED"
            }
        });
        let robust_status = match upper_with_deadline.status.as_str() {
            "HEURISTIC_FEASIBLE_NOT_PROVEN_OPTIMAL" => {
                "ROBUST_FEASIBLE_CANDIDATE_NOT_PROVEN_OPTIMAL"
            }
            "BOUNDED_FALLBACK_FEASIBLE_NOT_PROVEN_OPTIMAL" => {
                "ROBUST_FEASIBLE_CANDIDATE_NOT_PROVEN_OPTIMAL"
            }
            "PROVEN_INFEASIBLE_PRECHECK" => {
                "ROBUST_INFEASIBILITY_PROVEN_BY_LOWER_BOUND_OR_HARD_CONSTRAINT"
            }
            _ => "ROBUST_FEASIBILITY_NOT_ESTABLISHED",
        }
        .to_owned();
        results.push(CaseResult {
            case_id: case.case_id.clone(),
            nominal,
            upper_duration_unconstrained: upper_unconstrained,
            upper_duration_with_deadline: upper_with_deadline,
            robust_status,
            optimality_claim: "NOT_PROVEN".to_owned(),
        });
    }

    Ok(PlannerReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        suite_id: fixture.suite_id,
        input_sha256: sha256_hex(bytes),
        input_bytes: bytes.len(),
        algorithm: "priority_portfolio_with_bounded_order_recovery_and_independent_validation"
            .to_owned(),
        case_count: results.len(),
        all_returned_schedules_independently_validated: every_schedule_valid,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        case_results: results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] =
        include_bytes!("../../../docs/research/fixtures/HABITAT_SCHEDULING_MICRO_ORACLE_V0_1.json");

    #[test]
    fn constructs_independently_validated_candidates_for_feasible_micro_cases() {
        let report = plan_fixture_bytes(FIXTURE).unwrap();
        assert!(report.all_returned_schedules_independently_validated);
        for result in &report.case_results {
            let case_feasible = serde_json::from_slice::<Value>(FIXTURE)
                .unwrap()["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|case| case["case_id"] == result.case_id)
                .unwrap()["expected"]["feasible"]
                .as_bool()
                .unwrap();
            if case_feasible
                && result.case_id == "priority_rules_miss_feasible_schedule"
            {
                assert_eq!(
                    result.nominal.status,
                    "BOUNDED_FALLBACK_FEASIBLE_NOT_PROVEN_OPTIMAL"
                );
                assert_eq!(result.nominal.makespan, Some(19));
                assert_eq!(
                    result.nominal.priority_only_status,
                    "NO_CANDIDATE_FOUND_NOT_PROOF_OF_INFEASIBILITY"
                );
                assert_eq!(
                    result.nominal.fallback_status,
                    "CANDIDATE_FOUND_SEARCH_COMPLETE_NOT_OPTIMAL_PROVEN"
                );
                assert!(result.nominal.fallback_states_expanded > 0);
                assert!(result.nominal.fallback_search_complete);
                assert_eq!(
                    result.nominal.validation_status,
                    "INDEPENDENT_VALIDATOR_PASSED"
                );
            } else if case_feasible {
                assert_ne!(
                    result.nominal.status,
                    "HEURISTIC_FEASIBLE_NOT_PROVEN_OPTIMAL",
                    "{}",
                    result.case_id
                );
            }
            assert_eq!(result.optimality_claim, "NOT_PROVEN");
        }
    }

    #[test]
    fn bounded_fallback_recovers_a_schedule_after_priority_rules_miss() {
        let report = plan_fixture_bytes(FIXTURE).unwrap();
        let result = report
            .case_results
            .iter()
            .find(|case| case.case_id == "priority_rules_miss_feasible_schedule")
            .unwrap();
        assert_eq!(
            result.nominal.priority_only_status,
            "NO_CANDIDATE_FOUND_NOT_PROOF_OF_INFEASIBILITY"
        );
        assert_eq!(
            result.nominal.status,
            "BOUNDED_FALLBACK_FEASIBLE_NOT_PROVEN_OPTIMAL"
        );
        assert_eq!(result.nominal.makespan, Some(19));
        assert_eq!(
            result.robust_status,
            "ROBUST_FEASIBLE_CANDIDATE_NOT_PROVEN_OPTIMAL"
        );
        assert_eq!(result.optimality_claim, "NOT_PROVEN");
    }

    #[test]
    fn parallelism_and_shared_resource_semantics_are_preserved() {
        let report = plan_fixture_bytes(FIXTURE).unwrap();
        let distinct = report.case_results.iter()
            .find(|case| case.case_id == "independent_tasks_distinct_crews").unwrap();
        let shared = report.case_results.iter()
            .find(|case| case.case_id == "independent_tasks_shared_exclusive_crew").unwrap();
        assert_eq!(distinct.nominal.makespan, Some(6));
        assert_eq!(shared.nominal.makespan, Some(10));
    }

    #[test]
    fn heuristic_never_claims_optimality_and_robust_deadline_is_separate() {
        let report = plan_fixture_bytes(FIXTURE).unwrap();
        let uncertain = report.case_results.iter()
            .find(|case| case.case_id == "nominally_fast_but_not_upper_scenario_robust").unwrap();
        assert_eq!(uncertain.nominal.makespan, Some(4));
        assert_eq!(uncertain.upper_duration_unconstrained.makespan, Some(8));
        assert_eq!(
            uncertain.robust_status,
            "ROBUST_INFEASIBILITY_PROVEN_BY_LOWER_BOUND_OR_HARD_CONSTRAINT"
        );
        assert_eq!(uncertain.optimality_claim, "NOT_PROVEN");
    }

    #[test]
    fn independent_validator_rejects_a_resource_overlapping_schedule() {
        let fixture: Fixture = serde_json::from_slice(FIXTURE).unwrap();
        let case = fixture
            .cases
            .iter()
            .find(|case| case.case_id == "independent_tasks_shared_exclusive_crew")
            .unwrap();
        let durations = durations_for(case, false);
        let valid = solve_scenario(case, false, true);
        assert_eq!(valid.status, "HEURISTIC_FEASIBLE_NOT_PROVEN_OPTIMAL");
        let mut invalid = valid.schedule;
        let second = invalid
            .iter_mut()
            .find(|step| step.task_id == "B")
            .unwrap();
        second.start = 0;
        second.finish = 6;
        assert!(independently_validate_schedule(case, &invalid, &durations, true).is_err());
    }

    #[test]
    fn impossible_gates_windows_and_cycles_are_proven_prechecks() {
        let report = plan_fixture_bytes(FIXTURE).unwrap();
        for case_id in [
            "inspection_window_conflicts_with_precedence",
            "missing_hard_approval_gate",
            "resource_demand_exceeds_capacity",
            "cycle_in_precedence_graph",
        ] {
            let result = report
                .case_results
                .iter()
                .find(|result| result.case_id == case_id)
                .unwrap();
            assert_eq!(result.nominal.status, "PROVEN_INFEASIBLE_PRECHECK");
        }
    }

    #[test]
    fn unknown_fields_fail_closed_and_fixture_digest_is_recorded() {
        let mut value: Value = serde_json::from_slice(FIXTURE).unwrap();
        value["new_approval_override"] = Value::Bool(true);
        assert!(plan_fixture_bytes(value.to_string().as_bytes()).is_err());
        let report = plan_fixture_bytes(FIXTURE).unwrap();
        assert_eq!(report.input_bytes, FIXTURE.len());
        assert_eq!(report.input_sha256, sha256_hex(FIXTURE));
    }
}
