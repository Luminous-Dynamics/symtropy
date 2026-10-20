// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic resource-constrained scheduling for multi-dish cooking.
//!
//! Time is integer milliseconds. Tasks have precedence constraints, bounded
//! gaps, release times, deadlines, and exclusive resources.
//!
//! The V1 scheduler is a deterministic greedy baseline, not an optimal solver.
//! Failure means that this heuristic found no schedule, not that no feasible
//! schedule exists. Every result is independently checked against its input.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const KITCHEN_PLAN_SCHEMA_VERSION: u32 = 1;
pub const KITCHEN_SCHEDULE_SCHEMA_VERSION: u32 = 1;
pub const GREEDY_SCHEDULER_ID: &str = "stable_topological_earliest_gap_v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TaskDependencyV1 {
    pub predecessor_task_id: String,
    pub min_lag_ms: u64,
    pub max_lag_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenTaskV1 {
    pub task_id: String,
    pub duration_ms: u64,
    pub resource_ids: Vec<String>,
    pub dependencies: Vec<TaskDependencyV1>,
    pub earliest_start_ms: Option<u64>,
    pub latest_finish_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenPlanV1 {
    pub schema_version: u32,
    pub plan_id: String,
    pub tasks: Vec<KitchenTaskV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanViolationCode {
    UnsupportedSchemaVersion,
    MissingRequiredValue,
    DuplicateTaskId,
    InvalidDuration,
    DuplicateResource,
    UnknownDependency,
    SelfDependency,
    DuplicateDependency,
    InvalidLagWindow,
    InvalidTaskWindow,
    TimeOverflow,
    DependencyCycle,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlanViolation {
    pub code: PlanViolationCode,
    pub task_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlanValidationReport {
    pub plan_id: String,
    pub topological_order: Vec<String>,
    pub violations: Vec<PlanViolation>,
}

impl PlanValidationReport {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn plan_violation(
    violations: &mut Vec<PlanViolation>,
    code: PlanViolationCode,
    task_id: Option<&str>,
    detail: impl Into<String>,
) {
    violations.push(PlanViolation {
        code,
        task_id: task_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn task_index<'a>(plan: &'a KitchenPlanV1) -> BTreeMap<&'a str, &'a KitchenTaskV1> {
    let mut index = BTreeMap::new();
    for task in &plan.tasks {
        if nonempty(&task.task_id) {
            index.entry(task.task_id.as_str()).or_insert(task);
        }
    }
    index
}

/// Validate IDs, timing constraints and the dependency DAG. Topological ties
/// are broken lexicographically to keep the result deterministic.
pub fn validate_plan(plan: &KitchenPlanV1) -> PlanValidationReport {
    let mut violations = Vec::new();
    if plan.schema_version != KITCHEN_PLAN_SCHEMA_VERSION {
        plan_violation(
            &mut violations,
            PlanViolationCode::UnsupportedSchemaVersion,
            None,
            format!("unsupported schema version {}", plan.schema_version),
        );
    }
    if !nonempty(&plan.plan_id) {
        plan_violation(
            &mut violations,
            PlanViolationCode::MissingRequiredValue,
            None,
            "plan_id must be non-empty",
        );
    }
    if plan.tasks.is_empty() {
        plan_violation(
            &mut violations,
            PlanViolationCode::MissingRequiredValue,
            None,
            "plan must contain at least one task",
        );
    }

    let mut ids = BTreeSet::new();
    let mut index: BTreeMap<&str, &KitchenTaskV1> = BTreeMap::new();

    for task in &plan.tasks {
        if !nonempty(&task.task_id) {
            plan_violation(
                &mut violations,
                PlanViolationCode::MissingRequiredValue,
                None,
                "task_id must be non-empty",
            );
        } else if !ids.insert(task.task_id.as_str()) {
            plan_violation(
                &mut violations,
                PlanViolationCode::DuplicateTaskId,
                Some(&task.task_id),
                format!("duplicate task_id {}", task.task_id),
            );
        } else {
            index.insert(task.task_id.as_str(), task);
        }

        if task.duration_ms == 0 {
            plan_violation(
                &mut violations,
                PlanViolationCode::InvalidDuration,
                Some(&task.task_id),
                "duration_ms must be greater than zero",
            );
        }

        let mut resources = BTreeSet::new();
        for resource_id in &task.resource_ids {
            if !nonempty(resource_id) {
                plan_violation(
                    &mut violations,
                    PlanViolationCode::MissingRequiredValue,
                    Some(&task.task_id),
                    "resource IDs may not be empty",
                );
            } else if !resources.insert(resource_id.as_str()) {
                plan_violation(
                    &mut violations,
                    PlanViolationCode::DuplicateResource,
                    Some(&task.task_id),
                    format!("resource {} is duplicated", resource_id),
                );
            }
        }

        let mut dependencies = BTreeSet::new();
        for dep in &task.dependencies {
            if !nonempty(&dep.predecessor_task_id) {
                plan_violation(
                    &mut violations,
                    PlanViolationCode::MissingRequiredValue,
                    Some(&task.task_id),
                    "dependency predecessor ID must be non-empty",
                );
            }
            if dep.predecessor_task_id == task.task_id {
                plan_violation(
                    &mut violations,
                    PlanViolationCode::SelfDependency,
                    Some(&task.task_id),
                    "task may not depend on itself",
                );
            }
            if !dependencies.insert(dep.predecessor_task_id.as_str()) {
                plan_violation(
                    &mut violations,
                    PlanViolationCode::DuplicateDependency,
                    Some(&task.task_id),
                    format!("dependency {} is duplicated", dep.predecessor_task_id),
                );
            }
            if dep.max_lag_ms.is_some_and(|max| max < dep.min_lag_ms) {
                plan_violation(
                    &mut violations,
                    PlanViolationCode::InvalidLagWindow,
                    Some(&task.task_id),
                    "maximum dependency lag is below minimum lag",
                );
            }
        }

        if let Some(deadline) = task.latest_finish_ms {
            if let Some(release) = task.earliest_start_ms {
                match release.checked_add(task.duration_ms) {
                    Some(finish) if finish <= deadline => {}
                    Some(_) => plan_violation(
                        &mut violations,
                        PlanViolationCode::InvalidTaskWindow,
                        Some(&task.task_id),
                        "earliest start plus duration exceeds latest finish",
                    ),
                    None => plan_violation(
                        &mut violations,
                        PlanViolationCode::TimeOverflow,
                        Some(&task.task_id),
                        "earliest start plus duration overflows u64",
                    ),
                }
            } else if task.duration_ms > deadline {
                plan_violation(
                    &mut violations,
                    PlanViolationCode::InvalidTaskWindow,
                    Some(&task.task_id),
                    "task cannot meet its deadline even when started at zero",
                );
            }
        }
    }

    for task in &plan.tasks {
        for dep in &task.dependencies {
            if !index.contains_key(dep.predecessor_task_id.as_str()) {
                plan_violation(
                    &mut violations,
                    PlanViolationCode::UnknownDependency,
                    Some(&task.task_id),
                    format!("unknown predecessor {}", dep.predecessor_task_id),
                );
            }
        }
    }

    let mut indegree: BTreeMap<&str, usize> =
        index.keys().map(|id| (*id, 0_usize)).collect();
    let mut children: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for task in &plan.tasks {
        if !index.contains_key(task.task_id.as_str()) {
            continue;
        }
        for dep in &task.dependencies {
            let parent = dep.predecessor_task_id.as_str();
            if parent != task.task_id && index.contains_key(parent) {
                let inserted = children
                    .entry(parent)
                    .or_default()
                    .insert(task.task_id.as_str());
                if inserted {
                    if let Some(degree) = indegree.get_mut(task.task_id.as_str()) {
                        *degree += 1;
                    }
                }
            }
        }
    }

    let mut ready: BTreeSet<&str> = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect();
    let mut topological_order = Vec::with_capacity(index.len());

    while let Some(id) = ready.pop_first() {
        topological_order.push(id.to_owned());
        if let Some(next) = children.get(id) {
            for child in next {
                if let Some(degree) = indegree.get_mut(child) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        ready.insert(child);
                    }
                }
            }
        }
    }

    if topological_order.len() != index.len() {
        plan_violation(
            &mut violations,
            PlanViolationCode::DependencyCycle,
            None,
            "dependency graph contains a cycle",
        );
    }

    violations.sort_by(|a, b| {
        (a.code, a.task_id.as_deref().unwrap_or(""), a.detail.as_str()).cmp(&(
            b.code,
            b.task_id.as_deref().unwrap_or(""),
            b.detail.as_str(),
        ))
    });
    PlanValidationReport {
        plan_id: plan.plan_id.clone(),
        topological_order,
        violations,
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScheduledTaskV1 {
    pub task_id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub resource_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KitchenScheduleReceiptV1 {
    pub schema_version: u32,
    pub plan_id: String,
    pub scheduler_id: String,
    pub tasks: Vec<ScheduledTaskV1>,
    pub makespan_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ScheduleError {
    InvalidPlan(Vec<PlanViolation>),
    NoScheduleFoundByHeuristic { task_id: String, reason: String },
    VerificationFailed(Vec<ScheduleViolation>),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleViolationCode {
    UnsupportedSchemaVersion,
    PlanIdMismatch,
    DuplicateTask,
    UnknownTask,
    MissingTask,
    DurationMismatch,
    ResourceAssignmentMismatch,
    EarliestStartViolation,
    LatestFinishViolation,
    DependencyLowerLagViolation,
    DependencyUpperLagViolation,
    ResourceOverlap,
    TimeOverflow,
    MakespanMismatch,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScheduleViolation {
    pub code: ScheduleViolationCode,
    pub task_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScheduleValidationReport {
    pub plan_id: String,
    pub violations: Vec<ScheduleViolation>,
}

impl ScheduleValidationReport {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }
}

fn schedule_violation(
    violations: &mut Vec<ScheduleViolation>,
    code: ScheduleViolationCode,
    task_id: Option<&str>,
    detail: impl Into<String>,
) {
    violations.push(ScheduleViolation {
        code,
        task_id: task_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn shares_resource(left: &[String], right: &[String]) -> bool {
    left.iter().any(|resource| right.contains(resource))
}

fn no_schedule(task_id: &str, reason: impl Into<String>) -> ScheduleError {
    ScheduleError::NoScheduleFoundByHeuristic {
        task_id: task_id.to_owned(),
        reason: reason.into(),
    }
}

/// Build a stable earliest-gap schedule. This is an executable baseline, not
/// an optimality claim; a resource ordering choice can cause false negatives.
pub fn build_schedule(
    plan: &KitchenPlanV1,
) -> Result<KitchenScheduleReceiptV1, ScheduleError> {
    let validation = validate_plan(plan);
    if !validation.is_valid() {
        return Err(ScheduleError::InvalidPlan(validation.violations));
    }
    let tasks = task_index(plan);
    let mut assigned: BTreeMap<String, ScheduledTaskV1> = BTreeMap::new();

    for id in validation.topological_order {
        let task = tasks
            .get(id.as_str())
            .copied()
            .ok_or_else(|| no_schedule(&id, "task missing from validated index"))?;

        let mut lower = task.earliest_start_ms.unwrap_or(0);
        let mut upper = match task.latest_finish_ms {
            Some(deadline) => Some(deadline.checked_sub(task.duration_ms).ok_or_else(|| {
                no_schedule(&task.task_id, "duration exceeds latest-finish deadline")
            })?),
            None => None,
        };

        for dep in &task.dependencies {
            let predecessor = assigned.get(&dep.predecessor_task_id).ok_or_else(|| {
                no_schedule(&task.task_id, "predecessor was not scheduled before successor")
            })?;
            let min_start = predecessor
                .end_ms
                .checked_add(dep.min_lag_ms)
                .ok_or_else(|| no_schedule(&task.task_id, "minimum lag overflows time"))?;
            lower = lower.max(min_start);

            if let Some(max_lag) = dep.max_lag_ms {
                let max_start = predecessor
                    .end_ms
                    .checked_add(max_lag)
                    .ok_or_else(|| no_schedule(&task.task_id, "maximum lag overflows time"))?;
                upper = Some(upper.map_or(max_start, |current| current.min(max_start)));
            }
        }
        if upper.is_some_and(|latest| lower > latest) {
            return Err(no_schedule(&task.task_id, "dependency and deadline windows conflict"));
        }

        let mut start = lower;
        loop {
            if upper.is_some_and(|latest| start > latest) {
                return Err(no_schedule(
                    &task.task_id,
                    "resource conflict exceeds latest permitted start",
                ));
            }
            let end = start
                .checked_add(task.duration_ms)
                .ok_or_else(|| no_schedule(&task.task_id, "task end overflows time"))?;

            let mut conflict_end: Option<u64> = None;
            for existing in assigned.values() {
                if shares_resource(&task.resource_ids, &existing.resource_ids)
                    && start < existing.end_ms
                    && existing.start_ms < end
                {
                    conflict_end = Some(
                        conflict_end.map_or(existing.end_ms, |old| old.max(existing.end_ms)),
                    );
                }
            }

            if let Some(next) = conflict_end {
                if next <= start {
                    return Err(no_schedule(&task.task_id, "resource conflict made no progress"));
                }
                start = next;
            } else {
                assigned.insert(
                    task.task_id.clone(),
                    ScheduledTaskV1 {
                        task_id: task.task_id.clone(),
                        start_ms: start,
                        end_ms: end,
                        resource_ids: task.resource_ids.clone(),
                    },
                );
                break;
            }
        }
    }

    let mut scheduled: Vec<ScheduledTaskV1> = assigned.into_values().collect();
    scheduled.sort_by(|a, b| {
        (a.start_ms, a.task_id.as_str()).cmp(&(b.start_ms, b.task_id.as_str()))
    });
    let makespan_ms = scheduled.iter().map(|task| task.end_ms).max().unwrap_or(0);
    let receipt = KitchenScheduleReceiptV1 {
        schema_version: KITCHEN_SCHEDULE_SCHEMA_VERSION,
        plan_id: plan.plan_id.clone(),
        scheduler_id: GREEDY_SCHEDULER_ID.to_owned(),
        tasks: scheduled,
        makespan_ms,
    };
    let report = validate_schedule(plan, &receipt);
    if !report.is_valid() {
        return Err(ScheduleError::VerificationFailed(report.violations));
    }
    Ok(receipt)
}

/// Build a simple serial baseline over the same plan. It uses one synthetic
/// exclusive resource across all tasks, then removes that implementation-only
/// resource from the receipt. The result is checked against the original plan.
/// Tight maximum-lag constraints may make a serial baseline unavailable; that
/// must be reported rather than relaxing the input constraints.
pub fn build_serial_baseline(
    plan: &KitchenPlanV1,
) -> Result<KitchenScheduleReceiptV1, ScheduleError> {
    let mut serial_plan = plan.clone();
    let mut serial_resource = "__symtropy_serial_baseline_resource_v1".to_owned();
    while plan
        .tasks
        .iter()
        .any(|task| task.resource_ids.contains(&serial_resource))
    {
        serial_resource.push('_');
    }

    for task in &mut serial_plan.tasks {
        task.resource_ids.push(serial_resource.clone());
    }

    let mut receipt = build_schedule(&serial_plan)?;
    for task in &mut receipt.tasks {
        task.resource_ids.retain(|resource| resource != &serial_resource);
    }
    receipt.scheduler_id = "stable_serial_topological_baseline_v1".to_owned();

    let report = validate_schedule(plan, &receipt);
    if !report.is_valid() {
        return Err(ScheduleError::VerificationFailed(report.violations));
    }
    Ok(receipt)
}

/// Check any proposed schedule independently against all declared constraints.
pub fn validate_schedule(
    plan: &KitchenPlanV1,
    receipt: &KitchenScheduleReceiptV1,
) -> ScheduleValidationReport {
    let mut violations = Vec::new();
    if receipt.schema_version != KITCHEN_SCHEDULE_SCHEMA_VERSION {
        schedule_violation(
            &mut violations,
            ScheduleViolationCode::UnsupportedSchemaVersion,
            None,
            "unsupported schedule schema version",
        );
    }
    if receipt.plan_id != plan.plan_id {
        schedule_violation(
            &mut violations,
            ScheduleViolationCode::PlanIdMismatch,
            None,
            "receipt plan ID differs from plan ID",
        );
    }
    if !validate_plan(plan).is_valid() {
        schedule_violation(
            &mut violations,
            ScheduleViolationCode::UnknownTask,
            None,
            "source plan itself is invalid",
        );
    }

    let tasks = task_index(plan);
    let mut by_id: BTreeMap<&str, &ScheduledTaskV1> = BTreeMap::new();
    for scheduled in &receipt.tasks {
        if !tasks.contains_key(scheduled.task_id.as_str()) {
            schedule_violation(
                &mut violations,
                ScheduleViolationCode::UnknownTask,
                Some(&scheduled.task_id),
                "scheduled task is not present in source plan",
            );
        } else if by_id.insert(scheduled.task_id.as_str(), scheduled).is_some() {
            schedule_violation(
                &mut violations,
                ScheduleViolationCode::DuplicateTask,
                Some(&scheduled.task_id),
                "scheduled task appears more than once",
            );
        }
    }

    for task in &plan.tasks {
        let Some(scheduled) = by_id.get(task.task_id.as_str()).copied() else {
            schedule_violation(
                &mut violations,
                ScheduleViolationCode::MissingTask,
                Some(&task.task_id),
                "task is absent from schedule",
            );
            continue;
        };

        match scheduled.start_ms.checked_add(task.duration_ms) {
            Some(expected) if expected == scheduled.end_ms => {}
            Some(expected) => schedule_violation(
                &mut violations,
                ScheduleViolationCode::DurationMismatch,
                Some(&task.task_id),
                format!("end {} differs from expected {}", scheduled.end_ms, expected),
            ),
            None => schedule_violation(
                &mut violations,
                ScheduleViolationCode::TimeOverflow,
                Some(&task.task_id),
                "task end overflows u64 milliseconds",
            ),
        }

        let required: BTreeSet<&str> = task.resource_ids.iter().map(String::as_str).collect();
        let used: BTreeSet<&str> = scheduled.resource_ids.iter().map(String::as_str).collect();
        if required != used {
            schedule_violation(
                &mut violations,
                ScheduleViolationCode::ResourceAssignmentMismatch,
                Some(&task.task_id),
                "scheduled resource IDs differ from plan",
            );
        }
        if task.earliest_start_ms.is_some_and(|earliest| scheduled.start_ms < earliest) {
            schedule_violation(
                &mut violations,
                ScheduleViolationCode::EarliestStartViolation,
                Some(&task.task_id),
                "task starts before release time",
            );
        }
        if task.latest_finish_ms.is_some_and(|deadline| scheduled.end_ms > deadline) {
            schedule_violation(
                &mut violations,
                ScheduleViolationCode::LatestFinishViolation,
                Some(&task.task_id),
                "task finishes after deadline",
            );
        }

        for dep in &task.dependencies {
            let Some(predecessor) = by_id.get(dep.predecessor_task_id.as_str()).copied() else {
                continue;
            };
            match predecessor.end_ms.checked_add(dep.min_lag_ms) {
                Some(earliest) if scheduled.start_ms >= earliest => {}
                Some(earliest) => schedule_violation(
                    &mut violations,
                    ScheduleViolationCode::DependencyLowerLagViolation,
                    Some(&task.task_id),
                    format!(
                        "start {} is earlier than min-lag bound {}",
                        scheduled.start_ms, earliest
                    ),
                ),
                None => schedule_violation(
                    &mut violations,
                    ScheduleViolationCode::TimeOverflow,
                    Some(&task.task_id),
                    "minimum dependency lag overflows time",
                ),
            }
            if let Some(max_lag) = dep.max_lag_ms {
                match predecessor.end_ms.checked_add(max_lag) {
                    Some(latest) if scheduled.start_ms <= latest => {}
                    Some(latest) => schedule_violation(
                        &mut violations,
                        ScheduleViolationCode::DependencyUpperLagViolation,
                        Some(&task.task_id),
                        format!(
                            "start {} is later than max-lag bound {}",
                            scheduled.start_ms, latest
                        ),
                    ),
                    None => schedule_violation(
                        &mut violations,
                        ScheduleViolationCode::TimeOverflow,
                        Some(&task.task_id),
                        "maximum dependency lag overflows time",
                    ),
                }
            }
        }
    }

    for i in 0..receipt.tasks.len() {
        for j in (i + 1)..receipt.tasks.len() {
            let left = &receipt.tasks[i];
            let right = &receipt.tasks[j];
            if left.task_id != right.task_id
                && shares_resource(&left.resource_ids, &right.resource_ids)
                && left.start_ms < right.end_ms
                && right.start_ms < left.end_ms
            {
                schedule_violation(
                    &mut violations,
                    ScheduleViolationCode::ResourceOverlap,
                    Some(&right.task_id),
                    format!("overlaps task {} on an exclusive resource", left.task_id),
                );
            }
        }
    }

    let computed_makespan = receipt.tasks.iter().map(|task| task.end_ms).max().unwrap_or(0);
    if computed_makespan != receipt.makespan_ms {
        schedule_violation(
            &mut violations,
            ScheduleViolationCode::MakespanMismatch,
            None,
            format!("reported makespan {} != computed {}", receipt.makespan_ms, computed_makespan),
        );
    }
    violations.sort_by(|a, b| {
        (a.code, a.task_id.as_deref().unwrap_or(""), a.detail.as_str()).cmp(&(
            b.code,
            b.task_id.as_deref().unwrap_or(""),
            b.detail.as_str(),
        ))
    });
    ScheduleValidationReport {
        plan_id: plan.plan_id.clone(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(
        id: &str,
        duration_ms: u64,
        resources: &[&str],
        dependencies: Vec<TaskDependencyV1>,
    ) -> KitchenTaskV1 {
        KitchenTaskV1 {
            task_id: id.to_owned(),
            duration_ms,
            resource_ids: resources.iter().map(|value| (*value).to_owned()).collect(),
            dependencies,
            earliest_start_ms: None,
            latest_finish_ms: None,
        }
    }

    fn dep(id: &str, min_lag_ms: u64, max_lag_ms: Option<u64>) -> TaskDependencyV1 {
        TaskDependencyV1 {
            predecessor_task_id: id.to_owned(),
            min_lag_ms,
            max_lag_ms,
        }
    }

    fn plan(tasks: Vec<KitchenTaskV1>) -> KitchenPlanV1 {
        KitchenPlanV1 {
            schema_version: KITCHEN_PLAN_SCHEMA_VERSION,
            plan_id: "test-meal-v1".to_owned(),
            tasks,
        }
    }

    fn start_at(receipt: &KitchenScheduleReceiptV1, id: &str) -> u64 {
        receipt.tasks.iter().find(|task| task.task_id == id).unwrap().start_ms
    }

    #[test]
    fn independent_tasks_run_in_parallel() {
        let source = plan(vec![
            task("chop", 5, &["knife"], vec![]),
            task("preheat", 10, &["oven"], vec![]),
        ]);
        let receipt = build_schedule(&source).unwrap();
        assert_eq!(receipt.makespan_ms, 10);
        assert!(validate_schedule(&source, &receipt).is_valid());
    }

    #[test]
    fn shared_resource_serializes_tasks() {
        let source = plan(vec![
            task("saute", 8, &["burner-1"], vec![]),
            task("sauce", 6, &["burner-1"], vec![]),
        ]);
        let receipt = build_schedule(&source).unwrap();
        assert_eq!(receipt.makespan_ms, 14);
        assert_ne!(start_at(&receipt, "saute"), start_at(&receipt, "sauce"));
        assert!(validate_schedule(&source, &receipt).is_valid());
    }

    #[test]
    fn minimum_lag_preserves_rest_window() {
        let source = plan(vec![
            task("mix", 4, &["mixer"], vec![]),
            task("bake", 6, &["oven"], vec![dep("mix", 10, Some(20))]),
        ]);
        let receipt = build_schedule(&source).unwrap();
        assert_eq!(start_at(&receipt, "bake"), 14);
    }

    #[test]
    fn repeated_runs_are_identical() {
        let source = plan(vec![
            task("z-last", 5, &["z"], vec![]),
            task("a-first", 3, &["a"], vec![]),
            task("m-middle", 4, &["m"], vec![]),
        ]);
        assert_eq!(build_schedule(&source).unwrap(), build_schedule(&source).unwrap());
    }

    #[test]
    fn dependency_cycle_is_rejected() {
        let source = plan(vec![
            task("a", 1, &[], vec![dep("b", 0, None)]),
            task("b", 1, &[], vec![dep("a", 0, None)]),
        ]);
        assert!(
            validate_plan(&source)
                .violations
                .iter()
                .any(|v| v.code == PlanViolationCode::DependencyCycle)
        );
        assert!(matches!(build_schedule(&source), Err(ScheduleError::InvalidPlan(_))));
    }

    #[test]
    fn unknown_dependency_is_rejected() {
        let source = plan(vec![task("serve", 1, &[], vec![dep("missing", 0, None)])]);
        assert!(
            validate_plan(&source)
                .violations
                .iter()
                .any(|v| v.code == PlanViolationCode::UnknownDependency)
        );
    }

    #[test]
    fn impossible_deadline_is_rejected() {
        let mut roast = task("roast", 10, &["oven"], vec![]);
        roast.latest_finish_ms = Some(8);
        assert!(
            validate_plan(&plan(vec![roast]))
                .violations
                .iter()
                .any(|v| v.code == PlanViolationCode::InvalidTaskWindow)
        );
    }

    #[test]
    fn independent_verifier_rejects_resource_overlap() {
        let source = plan(vec![
            task("a", 10, &["oven"], vec![]),
            task("b", 10, &["oven"], vec![]),
        ]);
        let receipt = KitchenScheduleReceiptV1 {
            schema_version: KITCHEN_SCHEDULE_SCHEMA_VERSION,
            plan_id: source.plan_id.clone(),
            scheduler_id: GREEDY_SCHEDULER_ID.to_owned(),
            tasks: vec![
                ScheduledTaskV1 {
                    task_id: "a".to_owned(),
                    start_ms: 0,
                    end_ms: 10,
                    resource_ids: vec!["oven".to_owned()],
                },
                ScheduledTaskV1 {
                    task_id: "b".to_owned(),
                    start_ms: 5,
                    end_ms: 15,
                    resource_ids: vec!["oven".to_owned()],
                },
            ],
            makespan_ms: 15,
        };
        assert!(
            validate_schedule(&source, &receipt)
                .violations
                .iter()
                .any(|v| v.code == ScheduleViolationCode::ResourceOverlap)
        );
    }

    #[test]
    fn bounded_lag_failure_does_not_claim_global_infeasibility() {
        let source = plan(vec![
            task("a-prep", 5, &["pan"], vec![]),
            task("b-blocker", 5, &["pan"], vec![]),
            task("c-finish", 2, &["pan"], vec![dep("a-prep", 0, Some(0))]),
        ]);
        assert!(matches!(
            build_schedule(&source),
            Err(ScheduleError::NoScheduleFoundByHeuristic { .. })
        ));
    }

    #[test]
    fn parallel_scheduler_is_compared_with_serial_baseline() {
        let source = plan(vec![
            task("long-prep", 10, &["counter"], vec![]),
            task("short-sauce", 5, &["burner"], vec![]),
        ]);
        let parallel = build_schedule(&source).unwrap();
        let serial = build_serial_baseline(&source).unwrap();

        assert_eq!(parallel.makespan_ms, 10);
        assert_eq!(serial.makespan_ms, 15);
        assert_eq!(
            serial.scheduler_id,
            "stable_serial_topological_baseline_v1"
        );
        assert!(validate_schedule(&source, &serial).is_valid());
    }

}
