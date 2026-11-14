// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

pub mod external_audit;
pub mod provenance;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const FIXTURE_SCHEMA: &str = "luminous.habitat_calendar_disruption_micro_oracle.v0.1";
pub const REPORT_SCHEMA: &str = "luminous.habitat_calendar_oracle_report.v0.1";
pub const FROZEN_FIXTURE_SHA256: &str = "934ff338850ddd2a5e161b5fab10ce824e7f08d56ca8308b950a395280d3762c";
const CLAIM_CEILING: &str =
    "bounded_synthetic_calendar_micro_oracle_not_real_project_performance_or_safety_authority";
const MAX_CASES: usize = 64;
const MAX_TASKS_PER_CASE: usize = 12;
const MAX_HORIZON: u32 = 48;
const MAX_CANDIDATES_PER_TASK: usize = 20_000;
const MAX_SEARCH_STATES: usize = 2_000_000;

type Interval = [u32; 2];

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
    task_duration: String,
    execution_modes: BTreeMap<String, String>,
    resource_occupancy: String,
    precedence: String,
    scenario_blackouts: String,
    objective: String,
    schedule_is_executed_work: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScheduleCase {
    case_id: String,
    purpose: String,
    horizon_end: u32,
    hard_deadline: Option<u32>,
    tasks: Vec<Task>,
    precedence: Vec<Precedence>,
    resources: Vec<Resource>,
    scenarios: Vec<Scenario>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Task {
    id: String,
    duration: u32,
    execution_mode: String,
    release_time: u32,
    task_windows: Vec<Interval>,
    resource_demands: BTreeMap<String, u32>,
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
    work_windows: Vec<Interval>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    resource_blackouts: Vec<Blackout>,
    expected: Expected,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Blackout {
    resource_id: String,
    start: u32,
    finish: u32,
    event_type: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    status: String,
    optimal_makespan: Option<u32>,
    canonical_witness: Vec<WitnessStep>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct WitnessStep {
    pub task_id: String,
    pub start: u32,
    pub finish: u32,
    pub segments: Vec<Interval>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ScenarioResult {
    pub case_id: String,
    pub scenario_id: String,
    pub status: String,
    pub optimal_makespan: Option<u32>,
    pub canonical_witness: Vec<WitnessStep>,
    pub search_states_expanded: usize,
    pub search_complete: bool,
    pub independent_validator_status: String,
    pub expected_outcome_match: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct BenchmarkReport {
    pub schema_version: String,
    pub suite_id: String,
    pub input_sha256: String,
    pub input_bytes: usize,
    pub algorithm: String,
    pub case_count: usize,
    pub scenario_count: usize,
    pub search_states_expanded_total: usize,
    pub all_searches_complete: bool,
    pub all_returned_schedules_independently_validated: bool,
    pub all_expected_outcomes_match: bool,
    pub claim_ceiling: String,
    pub results: Vec<ScenarioResult>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Candidate {
    step: WitnessStep,
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

fn windows_are_valid(windows: &[Interval], horizon_end: u32) -> bool {
    if windows.is_empty() {
        return false;
    }
    let mut previous_end = None;
    for [start, finish] in windows {
        if start >= finish || *finish > horizon_end {
            return false;
        }
        if previous_end.is_some_and(|end| end >= *start) {
            // Overlapping and adjacent windows are rejected: adjacent windows
            // should be canonicalized into one continuous availability interval.
            return false;
        }
        previous_end = Some(*finish);
    }
    true
}

fn interval_is_covered(windows: &[Interval], start: u32, finish: u32) -> bool {
    start < finish
        && windows
            .iter()
            .any(|window| window[0] <= start && finish <= window[1])
}

fn interval_overlaps_blackout(
    scenario: &Scenario,
    resource_id: &str,
    start: u32,
    finish: u32,
) -> bool {
    scenario.resource_blackouts.iter().any(|blackout| {
        blackout.resource_id == resource_id && start < blackout.finish && blackout.start < finish
    })
}

fn validate_fixture(fixture: &Fixture) -> Result<(), String> {
    if fixture.schema_version != FIXTURE_SCHEMA {
        return Err(format!(
            "unsupported fixture schema: {}",
            fixture.schema_version
        ));
    }
    if fixture.suite_id.trim().is_empty()
        || fixture.status.trim().is_empty()
        || !fixture.synthetic_only
        || fixture.claim_ceiling.trim().is_empty()
    {
        return Err("fixture identity and synthetic-only claim ceiling are required".to_owned());
    }
    let c = &fixture.conventions;
    let expected_modes = BTreeMap::from([
        (
            "uninterrupted".to_owned(),
            "all required work occupies one contiguous segment".to_owned(),
        ),
        (
            "pause_resume".to_owned(),
            "work may be split across available segments; active segment lengths sum exactly to duration"
                .to_owned(),
        ),
    ]);
    if c.time_origin != 0
        || c.time_unit
            != "abstract integer work units and calendar units on the same finite horizon"
        || c.intervals != "half_open_start_inclusive_finish_exclusive"
        || c.task_duration != "active_work_units_not_elapsed_calendar_time"
        || c.execution_modes != expected_modes
        || c.resource_occupancy != "capacity is consumed only during active work segments"
        || c.precedence != "finish(before) <= start(after)"
        || c.scenario_blackouts
            != "subtract the declared resource intervals from that resource's working windows; overlapping blackouts are invalid input"
        || c.objective
            != "minimize maximum task finish subject to all hard constraints and horizon"
        || c.schedule_is_executed_work
    {
        return Err("fixture uses unsupported or ambiguous scheduling conventions".to_owned());
    }
    if fixture.cases.is_empty() || fixture.cases.len() > MAX_CASES {
        return Err("fixture case count is outside the supported bound".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    for case in &fixture.cases {
        if case.case_id.trim().is_empty()
            || case.purpose.trim().is_empty()
            || !case_ids.insert(case.case_id.as_str())
        {
            return Err("case IDs and purposes must be nonempty and case IDs unique".to_owned());
        }
        if case.horizon_end == 0 || case.horizon_end > MAX_HORIZON {
            return Err(format!("{} has an unsupported horizon", case.case_id));
        }
        if case
            .hard_deadline
            .is_some_and(|deadline| deadline > case.horizon_end)
        {
            return Err(format!("{} deadline exceeds its horizon", case.case_id));
        }
        if case.tasks.is_empty() || case.tasks.len() > MAX_TASKS_PER_CASE {
            return Err(format!("{} has an unsupported task count", case.case_id));
        }

        let mut task_ids = BTreeSet::new();
        for task in &case.tasks {
            if task.id.trim().is_empty()
                || !task_ids.insert(task.id.as_str())
                || task.duration == 0
                || task.duration > case.horizon_end
                || task.release_time >= case.horizon_end
                || !matches!(
                    task.execution_mode.as_str(),
                    "uninterrupted" | "pause_resume"
                )
                || !windows_are_valid(&task.task_windows, case.horizon_end)
                || task.resource_demands.is_empty()
            {
                return Err(format!("{} contains an invalid task", case.case_id));
            }
        }

        let mut resource_ids = BTreeSet::new();
        let mut resource_by_id = BTreeMap::new();
        for resource in &case.resources {
            if resource.id.trim().is_empty()
                || resource.capacity == 0
                || !resource_ids.insert(resource.id.as_str())
                || !windows_are_valid(&resource.work_windows, case.horizon_end)
            {
                return Err(format!("{} contains an invalid resource", case.case_id));
            }
            resource_by_id.insert(resource.id.as_str(), resource);
        }
        if resource_by_id.is_empty() {
            return Err(format!("{} has no resources", case.case_id));
        }

        for task in &case.tasks {
            for (resource_id, demand) in &task.resource_demands {
                let resource = resource_by_id.get(resource_id.as_str()).ok_or_else(|| {
                    format!("{} references unknown resource {resource_id}", case.case_id)
                })?;
                if *demand == 0 || *demand > resource.capacity {
                    return Err(format!(
                        "{} task {} has impossible demand on {}",
                        case.case_id, task.id, resource_id
                    ));
                }
            }
        }

        let mut edges = BTreeSet::new();
        let mut indegree: BTreeMap<&str, usize> =
            task_ids.iter().map(|task_id| (*task_id, 0)).collect();
        let mut successors: BTreeMap<&str, Vec<&str>> =
            task_ids.iter().map(|task_id| (*task_id, Vec::new())).collect();
        for edge in &case.precedence {
            if edge.before == edge.after
                || !task_ids.contains(edge.before.as_str())
                || !task_ids.contains(edge.after.as_str())
                || !edges.insert((edge.before.as_str(), edge.after.as_str()))
            {
                return Err(format!("{} contains an invalid precedence edge", case.case_id));
            }
            *indegree.get_mut(edge.after.as_str()).expect("task ID checked") += 1;
            successors
                .get_mut(edge.before.as_str())
                .expect("task ID checked")
                .push(edge.after.as_str());
        }
        let mut ready: BTreeSet<&str> = indegree
            .iter()
            .filter_map(|(task_id, degree)| (*degree == 0).then_some(*task_id))
            .collect();
        let mut visited = 0usize;
        while let Some(task_id) = ready.pop_first() {
            visited += 1;
            for successor in successors.get(task_id).expect("task ID checked") {
                let degree = indegree.get_mut(successor).expect("task ID checked");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(successor);
                }
            }
        }
        if visited != case.tasks.len() {
            return Err(format!("{} contains a precedence cycle", case.case_id));
        }

        if case.scenarios.is_empty() {
            return Err(format!("{} has no scenarios", case.case_id));
        }
        let mut scenario_ids = BTreeSet::new();
        for scenario in &case.scenarios {
            if scenario.scenario_id.trim().is_empty()
                || !scenario_ids.insert(scenario.scenario_id.as_str())
            {
                return Err(format!("{} has duplicate/empty scenario IDs", case.case_id));
            }
            let mut blackout_intervals: BTreeMap<&str, Vec<(u32, u32)>> = BTreeMap::new();
            for blackout in &scenario.resource_blackouts {
                if !resource_ids.contains(blackout.resource_id.as_str())
                    || blackout.start >= blackout.finish
                    || blackout.finish > case.horizon_end
                    || blackout.event_type.trim().is_empty()
                {
                    return Err(format!(
                        "{} / {} contains an invalid blackout",
                        case.case_id, scenario.scenario_id
                    ));
                }
                blackout_intervals
                    .entry(blackout.resource_id.as_str())
                    .or_default()
                    .push((blackout.start, blackout.finish));
            }
            for (resource_id, intervals) in &mut blackout_intervals {
                intervals.sort_unstable();
                for pair in intervals.windows(2) {
                    if pair[1].0 < pair[0].1 {
                        return Err(format!(
                            "{} / {} has overlapping blackouts on {}",
                            case.case_id, scenario.scenario_id, resource_id
                        ));
                    }
                }
            }
            match scenario.expected.status.as_str() {
                "OPTIMAL_EXACT"
                    if scenario.expected.optimal_makespan.is_some()
                        && scenario.expected.canonical_witness.len() == case.tasks.len() => {}
                "PROVEN_INFEASIBLE"
                    if scenario.expected.optimal_makespan.is_none()
                        && scenario.expected.canonical_witness.is_empty() => {}
                _ => {
                    return Err(format!(
                        "{} / {} has an unsupported or inconsistent reference outcome",
                        case.case_id, scenario.scenario_id
                    ));
                }
            }
        }
    }
    Ok(())
}

fn task_slot_available(
    case: &ScheduleCase,
    task: &Task,
    scenario: &Scenario,
    slot: u32,
) -> bool {
    let finish = slot + 1;
    if !interval_is_covered(&task.task_windows, slot, finish) {
        return false;
    }
    for (resource_id, _) in &task.resource_demands {
        let Some(resource) = case.resources.iter().find(|r| r.id == *resource_id) else {
            return false;
        };
        if !interval_is_covered(&resource.work_windows, slot, finish)
            || interval_overlaps_blackout(scenario, resource_id, slot, finish)
        {
            return false;
        }
    }
    true
}

fn uninterrupted_interval_available(
    case: &ScheduleCase,
    task: &Task,
    scenario: &Scenario,
    start: u32,
    finish: u32,
) -> bool {
    if !interval_is_covered(&task.task_windows, start, finish) {
        return false;
    }
    for (resource_id, _) in &task.resource_demands {
        let Some(resource) = case.resources.iter().find(|r| r.id == *resource_id) else {
            return false;
        };
        if !interval_is_covered(&resource.work_windows, start, finish)
            || interval_overlaps_blackout(scenario, resource_id, start, finish)
        {
            return false;
        }
    }
    true
}

fn compress_slots(task_id: &str, slots: &[u32]) -> Option<WitnessStep> {
    let first = *slots.first()?;
    let last = *slots.last()?;
    let mut segments: Vec<Interval> = Vec::new();
    for slot in slots {
        if let Some(last_segment) = segments.last_mut() {
            if last_segment[1] == *slot {
                last_segment[1] += 1;
                continue;
            }
        }
        segments.push([*slot, *slot + 1]);
    }
    Some(WitnessStep {
        task_id: task_id.to_owned(),
        start: first,
        finish: last + 1,
        segments,
    })
}

fn enumerate_slot_combinations(
    task_id: &str,
    slots: &[u32],
    needed: usize,
    next_index: usize,
    current: &mut Vec<u32>,
    output: &mut Vec<WitnessStep>,
    budget_exceeded: &mut bool,
) {
    if *budget_exceeded {
        return;
    }
    if current.len() == needed {
        if output.len() >= MAX_CANDIDATES_PER_TASK {
            *budget_exceeded = true;
            return;
        }
        if let Some(candidate) = compress_slots(task_id, current) {
            output.push(candidate);
        }
        return;
    }
    let remaining = needed - current.len();
    if slots.len().saturating_sub(next_index) < remaining {
        return;
    }
    let last_start = slots.len() - remaining;
    for index in next_index..=last_start {
        current.push(slots[index]);
        enumerate_slot_combinations(
            task_id,
            slots,
            needed,
            index + 1,
            current,
            output,
            budget_exceeded,
        );
        current.pop();
        if *budget_exceeded {
            return;
        }
    }
}

fn candidates_for_task(
    case: &ScheduleCase,
    task: &Task,
    scenario: &Scenario,
) -> Result<Vec<Candidate>, ()> {
    let mut steps = Vec::new();
    if task.execution_mode == "uninterrupted" {
        if task.duration <= case.horizon_end {
            let last_start = case.horizon_end - task.duration;
            for start in task.release_time..=last_start {
                let finish = start + task.duration;
                if case.hard_deadline.is_some_and(|deadline| finish > deadline) {
                    continue;
                }
                if uninterrupted_interval_available(case, task, scenario, start, finish) {
                    steps.push(WitnessStep {
                        task_id: task.id.clone(),
                        start,
                        finish,
                        segments: vec![[start, finish]],
                    });
                    if steps.len() > MAX_CANDIDATES_PER_TASK {
                        return Err(());
                    }
                }
            }
        }
    } else {
        let slots: Vec<u32> = (task.release_time..case.horizon_end)
            .filter(|slot| task_slot_available(case, task, scenario, *slot))
            .collect();
        if slots.len() >= task.duration as usize {
            let mut budget_exceeded = false;
            enumerate_slot_combinations(
                &task.id,
                &slots,
                task.duration as usize,
                0,
                &mut Vec::new(),
                &mut steps,
                &mut budget_exceeded,
            );
            if budget_exceeded {
                return Err(());
            }
        }
        steps.retain(|step| {
            !case
                .hard_deadline
                .is_some_and(|deadline| step.finish > deadline)
        });
    }
    steps.sort();
    steps.dedup();
    Ok(steps.into_iter().map(|step| Candidate { step }).collect())
}

fn candidate_occupies_slot(candidate: &Candidate, slot: u32) -> bool {
    candidate
        .step
        .segments
        .iter()
        .any(|segment| segment[0] <= slot && slot < segment[1])
}

fn partial_assignment_is_feasible(
    case: &ScheduleCase,
    selected: &[Option<Candidate>],
) -> bool {
    for candidate in selected.iter().flatten() {
        if case
            .hard_deadline
            .is_some_and(|deadline| candidate.step.finish > deadline)
        {
            return false;
        }
    }

    for edge in &case.precedence {
        let before = case
            .tasks
            .iter()
            .position(|task| task.id == edge.before)
            .and_then(|index| selected[index].as_ref());
        let after = case
            .tasks
            .iter()
            .position(|task| task.id == edge.after)
            .and_then(|index| selected[index].as_ref());
        if let (Some(before), Some(after)) = (before, after) {
            if before.step.finish > after.step.start {
                return false;
            }
        }
    }

    for slot in 0..case.horizon_end {
        for resource in &case.resources {
            let mut used = 0u32;
            for (task_index, task) in case.tasks.iter().enumerate() {
                if let Some(candidate) = selected[task_index].as_ref() {
                    if candidate_occupies_slot(candidate, slot) {
                        used = used.saturating_add(
                            task.resource_demands.get(&resource.id).copied().unwrap_or(0),
                        );
                    }
                }
            }
            if used > resource.capacity {
                return false;
            }
        }
    }
    true
}

fn search_combinations(
    case: &ScheduleCase,
    candidate_sets: &[Vec<Candidate>],
    index: usize,
    selected: &mut [Option<Candidate>],
    states_expanded: &mut usize,
    budget_exceeded: &mut bool,
    best: &mut Option<(u32, Vec<WitnessStep>)>,
) {
    if *budget_exceeded {
        return;
    }
    if index == candidate_sets.len() {
        let mut witness: Vec<WitnessStep> = selected
            .iter()
            .filter_map(|item| item.as_ref().map(|candidate| candidate.step.clone()))
            .collect();
        witness.sort();
        let makespan = witness.iter().map(|step| step.finish).max().unwrap_or(0);
        let proposal = (makespan, witness);
        if best.as_ref().is_none_or(|current| proposal < *current) {
            *best = Some(proposal);
        }
        return;
    }

    for candidate in &candidate_sets[index] {
        if *states_expanded >= MAX_SEARCH_STATES {
            *budget_exceeded = true;
            return;
        }
        *states_expanded += 1;
        selected[index] = Some(candidate.clone());
        if partial_assignment_is_feasible(case, selected) {
            search_combinations(
                case,
                candidate_sets,
                index + 1,
                selected,
                states_expanded,
                budget_exceeded,
                best,
            );
        }
        selected[index] = None;
        if *budget_exceeded {
            return;
        }
    }
}

fn validate_witness(
    case: &ScheduleCase,
    scenario: &Scenario,
    witness: &[WitnessStep],
) -> Result<(), String> {
    if witness.len() != case.tasks.len() {
        return Err("witness does not schedule every task exactly once".to_owned());
    }
    let tasks_by_id: BTreeMap<&str, &Task> =
        case.tasks.iter().map(|task| (task.id.as_str(), task)).collect();
    let mut steps_by_id: BTreeMap<&str, &WitnessStep> = BTreeMap::new();

    for step in witness {
        let task = tasks_by_id
            .get(step.task_id.as_str())
            .ok_or_else(|| format!("witness references unknown task {}", step.task_id))?;
        if steps_by_id.insert(step.task_id.as_str(), step).is_some() {
            return Err(format!("witness duplicates task {}", step.task_id));
        }
        if step.segments.is_empty()
            || step.start != step.segments[0][0]
            || step.finish != step.segments.last().expect("nonempty").get(1).copied().unwrap_or(0)
            || step.start < task.release_time
            || step.finish > case.horizon_end
        {
            return Err(format!("task {} has inconsistent segment bounds", step.task_id));
        }
        if task.execution_mode == "uninterrupted" && step.segments.len() != 1 {
            return Err(format!(
                "uninterrupted task {} has multiple segments",
                step.task_id
            ));
        }

        let mut active_work = 0u32;
        let mut previous_finish = None;
        for segment in &step.segments {
            let [start, finish] = *segment;
            if start >= finish
                || finish > case.horizon_end
                || previous_finish.is_some_and(|previous| previous >= start)
                || !interval_is_covered(&task.task_windows, start, finish)
            {
                return Err(format!("task {} has an invalid segment", step.task_id));
            }
            for (resource_id, _) in &task.resource_demands {
                let resource = case
                    .resources
                    .iter()
                    .find(|resource| resource.id == *resource_id)
                    .ok_or_else(|| format!("task {} references unknown resource", task.id))?;
                if !interval_is_covered(&resource.work_windows, start, finish)
                    || interval_overlaps_blackout(scenario, resource_id, start, finish)
                {
                    return Err(format!(
                        "task {} executes outside {} availability",
                        task.id, resource_id
                    ));
                }
            }
            active_work = active_work.saturating_add(finish - start);
            previous_finish = Some(finish);
        }
        if active_work != task.duration {
            return Err(format!(
                "task {} active work {} differs from required duration {}",
                task.id, active_work, task.duration
            ));
        }
    }

    for edge in &case.precedence {
        let before = steps_by_id
            .get(edge.before.as_str())
            .ok_or_else(|| "missing predecessor in witness".to_owned())?;
        let after = steps_by_id
            .get(edge.after.as_str())
            .ok_or_else(|| "missing successor in witness".to_owned())?;
        if before.finish > after.start {
            return Err(format!(
                "precedence violation: {} must finish before {} starts",
                edge.before, edge.after
            ));
        }
    }

    if case
        .hard_deadline
        .is_some_and(|deadline| witness.iter().any(|step| step.finish > deadline))
    {
        return Err("schedule violates its hard deadline".to_owned());
    }

    // Independent unit-slot accounting: recompute resource use from the returned
    // segments rather than trusting the search's partial-assignment counters.
    for slot in 0..case.horizon_end {
        for resource in &case.resources {
            let mut used = 0u32;
            for step in witness {
                if step
                    .segments
                    .iter()
                    .any(|segment| segment[0] <= slot && slot < segment[1])
                {
                    let task = tasks_by_id[step.task_id.as_str()];
                    used = used.saturating_add(
                        task.resource_demands
                            .get(&resource.id)
                            .copied()
                            .unwrap_or(0),
                    );
                }
            }
            if used > resource.capacity {
                return Err(format!(
                    "resource {} capacity exceeded at time {}",
                    resource.id, slot
                ));
            }
        }
    }
    Ok(())
}

fn solve_scenario(
    case: &ScheduleCase,
    scenario: &Scenario,
) -> Result<ScenarioResult, String> {
    let mut candidate_sets = Vec::with_capacity(case.tasks.len());
    for task in &case.tasks {
        match candidates_for_task(case, task, scenario) {
            Ok(candidates) => candidate_sets.push(candidates),
            Err(()) => {
                return Ok(ScenarioResult {
                    case_id: case.case_id.clone(),
                    scenario_id: scenario.scenario_id.clone(),
                    status: "ORACLE_BUDGET_EXCEEDED_NOT_CLASSIFIED".to_owned(),
                    optimal_makespan: None,
                    canonical_witness: Vec::new(),
                    search_states_expanded: 0,
                    search_complete: false,
                    independent_validator_status: "NOT_APPLICABLE_NO_CANDIDATE".to_owned(),
                    expected_outcome_match: false,
                });
            }
        }
    }
    if candidate_sets.iter().any(Vec::is_empty) {
        return Ok(ScenarioResult {
            case_id: case.case_id.clone(),
            scenario_id: scenario.scenario_id.clone(),
            status: "PROVEN_INFEASIBLE".to_owned(),
            optimal_makespan: None,
            canonical_witness: Vec::new(),
            search_states_expanded: 0,
            search_complete: true,
            independent_validator_status: "NOT_APPLICABLE_NO_CANDIDATE".to_owned(),
            expected_outcome_match: scenario.expected.status == "PROVEN_INFEASIBLE"
                && scenario.expected.optimal_makespan.is_none()
                && scenario.expected.canonical_witness.is_empty(),
        });
    }

    let mut selected = vec![None; case.tasks.len()];
    let mut states_expanded = 0usize;
    let mut budget_exceeded = false;
    let mut best: Option<(u32, Vec<WitnessStep>)> = None;
    search_combinations(
        case,
        &candidate_sets,
        0,
        &mut selected,
        &mut states_expanded,
        &mut budget_exceeded,
        &mut best,
    );

    let search_complete = !budget_exceeded;
    let (status, optimal_makespan, canonical_witness) = match best {
        Some((makespan, witness)) if search_complete => {
            ("OPTIMAL_EXACT".to_owned(), Some(makespan), witness)
        }
        Some((makespan, witness)) => (
            "SEARCH_INCOMPLETE_CANDIDATE_NOT_PROVEN_OPTIMAL".to_owned(),
            Some(makespan),
            witness,
        ),
        None if search_complete => ("PROVEN_INFEASIBLE".to_owned(), None, Vec::new()),
        None => (
            "ORACLE_BUDGET_EXCEEDED_NOT_CLASSIFIED".to_owned(),
            None,
            Vec::new(),
        ),
    };

    let validator_status = if canonical_witness.is_empty() {
        "NOT_APPLICABLE_NO_CANDIDATE".to_owned()
    } else {
        validate_witness(case, scenario, &canonical_witness)
            .map_err(|error| {
                format!(
                    "internal oracle candidate failed independent validation for {} / {}: {error}",
                    case.case_id, scenario.scenario_id
                )
            })?;
        "INDEPENDENT_VALIDATOR_PASSED".to_owned()
    };

    let expected_match = status == scenario.expected.status
        && optimal_makespan == scenario.expected.optimal_makespan
        && canonical_witness == scenario.expected.canonical_witness;

    Ok(ScenarioResult {
        case_id: case.case_id.clone(),
        scenario_id: scenario.scenario_id.clone(),
        status,
        optimal_makespan,
        canonical_witness,
        search_states_expanded: states_expanded,
        search_complete,
        independent_validator_status: validator_status,
        expected_outcome_match: expected_match,
    })
}

/// Run the bounded exact reference over the exact input bytes.
pub fn evaluate_fixture_bytes(bytes: &[u8]) -> Result<BenchmarkReport, String> {
    let fixture: Fixture = serde_json::from_slice(bytes)
        .map_err(|error| format!("calendar fixture rejected: {error}"))?;
    validate_fixture(&fixture)?;

    let mut results = Vec::new();
    let mut search_states_expanded_total = 0usize;
    let mut all_searches_complete = true;
    let mut all_returned_schedules_independently_validated = true;
    let mut all_expected_outcomes_match = true;

    for case in &fixture.cases {
        for scenario in &case.scenarios {
            let result = solve_scenario(case, scenario)?;
            search_states_expanded_total = search_states_expanded_total
                .saturating_add(result.search_states_expanded);
            all_searches_complete &= result.search_complete;
            if !result.canonical_witness.is_empty() {
                all_returned_schedules_independently_validated &=
                    result.independent_validator_status == "INDEPENDENT_VALIDATOR_PASSED";
            }
            all_expected_outcomes_match &= result.expected_outcome_match;
            results.push(result);
        }
    }

    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        suite_id: fixture.suite_id,
        input_sha256: sha256_hex(bytes),
        input_bytes: bytes.len(),
        algorithm: "bounded exhaustive discrete active-work slot combinations with independent witness validation"
            .to_owned(),
        case_count: fixture.cases.len(),
        scenario_count: results.len(),
        search_states_expanded_total,
        all_searches_complete,
        all_returned_schedules_independently_validated,
        all_expected_outcomes_match,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        results,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &[u8] =
        include_bytes!("../../../docs/research/fixtures/HABITAT_CALENDAR_DISRUPTION_V0_1.json");

    #[test]
    fn frozen_calendar_scenarios_match_exact_reference_outcomes() {
        assert_eq!(sha256_hex(FIXTURE), FROZEN_FIXTURE_SHA256);
        let report = evaluate_fixture_bytes(FIXTURE).expect("fixture should be valid");
        assert_eq!(report.input_sha256, FROZEN_FIXTURE_SHA256);
        assert_eq!(report.case_count, 9);
        assert_eq!(report.scenario_count, 12);
        assert!(report.all_searches_complete);
        assert!(report.all_returned_schedules_independently_validated);
        assert!(report.all_expected_outcomes_match);
        assert_eq!(
            report
                .results
                .iter()
                .filter(|result| result.status == "PROVEN_INFEASIBLE")
                .count(),
            2
        );
    }

    #[test]
    fn pause_resume_and_uninterrupted_semantics_are_distinct() {
        let report = evaluate_fixture_bytes(FIXTURE).expect("fixture should be valid");
        let pausable = report
            .results
            .iter()
            .find(|result| result.case_id == "CAL-003-pause-and-resume-across-lunch")
            .expect("pause/resume case");
        let uninterrupted = report
            .results
            .iter()
            .find(|result| result.case_id == "CAL-004-uninterrupted-task-cannot-cross-lunch")
            .expect("uninterrupted case");
        assert_eq!(pausable.optimal_makespan, Some(7));
        assert_eq!(pausable.canonical_witness[0].segments, vec![[2, 4], [6, 7]]);
        assert_eq!(uninterrupted.optimal_makespan, Some(9));
        assert_eq!(
            uninterrupted.canonical_witness[0].segments,
            vec![[6, 9]]
        );
    }

    #[test]
    fn outage_replanning_keeps_baseline_separate() {
        let report = evaluate_fixture_bytes(FIXTURE).expect("fixture should be valid");
        let baseline = report
            .results
            .iter()
            .find(|result| {
                result.case_id == "CAL-006-replan-after-machine-outage"
                    && result.scenario_id == "baseline"
            })
            .expect("baseline");
        let outage = report
            .results
            .iter()
            .find(|result| {
                result.case_id == "CAL-006-replan-after-machine-outage"
                    && result.scenario_id == "machine_outage"
            })
            .expect("outage");
        assert_eq!(baseline.optimal_makespan, Some(5));
        assert_eq!(outage.optimal_makespan, Some(10));
        assert_eq!(
            outage.optimal_makespan.unwrap() - baseline.optimal_makespan.unwrap(),
            5
        );
    }

    #[test]
    fn strict_deserialization_rejects_unknown_fields() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["unreviewed_field"] = Value::Bool(true);
        let mutated = serde_json::to_vec(&value).expect("JSON serialization");
        assert!(evaluate_fixture_bytes(&mutated).is_err());
    }

    #[test]
    fn overlapping_blackouts_are_rejected_instead_of_silently_merged() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["cases"][5]["scenarios"][1]["resource_blackouts"]
            .as_array_mut()
            .expect("blackout array")
            .push(serde_json::json!({
                "resource_id": "machine",
                "start": 4,
                "finish": 6,
                "event_type": "overlapping_failure"
            }));
        let mutated = serde_json::to_vec(&value).expect("JSON serialization");
        assert!(evaluate_fixture_bytes(&mutated)
            .unwrap_err()
            .contains("overlapping blackouts"));
    }

    #[test]
    fn independent_validator_rejects_resource_double_booking() {
        let fixture: Fixture = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        let case = fixture
            .cases
            .iter()
            .find(|case| case.case_id == "CAL-002-exclusive-crew-serializes")
            .expect("exclusive crew case");
        let scenario = &case.scenarios[0];
        let invalid = vec![
            WitnessStep {
                task_id: "A".to_owned(),
                start: 0,
                finish: 3,
                segments: vec![[0, 3]],
            },
            WitnessStep {
                task_id: "B".to_owned(),
                start: 2,
                finish: 7,
                segments: vec![[2, 7]],
            },
        ];
        assert!(validate_witness(case, scenario, &invalid)
            .unwrap_err()
            .contains("capacity exceeded"));
    }
}
