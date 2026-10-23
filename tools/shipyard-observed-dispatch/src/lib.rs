//! Deterministic, observed-history scheduling baselines for shipyard recovery.
//!
//! This crate proposes a resource/precedence schedule from the current input
//! snapshot. Call it again at each decision epoch with updated observations.
//! Existing commitments are retained unchanged. Hidden/unobserved disturbances
//! are not represented in the policy input and therefore cannot influence a
//! plan. Geometry, travel paths, capacity, lift charts, and physical safety are
//! outside this crate and require independent evaluators.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub type Tick = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub id: String,
    /// Explicit source-owned tie-break rank for the fixed-order baseline.
    pub order: u64,
    pub release_tick: Tick,
    pub due_tick: Tick,
    pub duration_ticks: Tick,
    pub resource_kind: String,
    pub predecessor_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub id: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub operation_id: String,
    pub completed_at_tick: Tick,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commitment {
    pub operation_id: String,
    pub resource_id: String,
    pub start_tick: Tick,
    pub end_tick: Tick,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleOutage {
    pub event_id: String,
    pub resource_id: String,
    pub effective_from_tick: Tick,
    pub effective_until_tick: Tick,
    pub observed_at_tick: Tick,
}

/// Only events delivered to the policy by the current decision tick may appear
/// in this collection. The planner independently checks the observation time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchInput {
    pub decision_tick: Tick,
    /// Exclusive horizon boundary; an assignment must finish at or before it.
    pub horizon_end_tick: Tick,
    pub operations: Vec<Operation>,
    pub resources: Vec<Resource>,
    pub completed: Vec<Completion>,
    pub commitments: Vec<Commitment>,
    pub visible_outages: Vec<VisibleOutage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchRule {
    /// Preserve the source-owned order rank wherever readiness permits.
    FixedOrder,
    /// Prefer the smallest due tick.
    EarliestDueDate,
    /// Prefer the smallest (due tick - decision tick) / duration ratio.
    CriticalRatio,
    /// Prefer least remaining slack at the earliest precedence-ready tick.
    LeastSlack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    /// All mandatory operations are completed, committed, or newly assigned.
    Complete,
    /// A valid partial plan was built; at least one operation remains unassigned.
    Partial,
    /// The frozen input is internally inconsistent or malformed.
    InvalidInput,
    /// Existing frozen commitments conflict with one another or an observed outage.
    /// This status is not a feasible-plan claim even if all new work was assigned.
    FrozenConflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub operation_id: String,
    pub resource_id: String,
    pub start_tick: Tick,
    pub end_tick: Tick,
    pub frozen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchDiagnostic {
    pub code: String,
    pub operation_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchPlan {
    pub status: PlanStatus,
    pub rule: DispatchRule,
    pub decision_tick: Tick,
    pub horizon_end_tick: Tick,
    /// Includes frozen commitments plus new proposed assignments.
    pub assignments: Vec<Assignment>,
    pub unscheduled_operation_ids: Vec<String>,
    pub diagnostics: Vec<DispatchDiagnostic>,
}

#[derive(Debug, Clone)]
struct ValidatedInput {
    operations: BTreeMap<String, Operation>,
    resources: BTreeMap<String, Resource>,
    completed: BTreeMap<String, Tick>,
    commitments: BTreeMap<String, Assignment>,
    visible_outages: Vec<VisibleOutage>,
    frozen_conflicts: Vec<DispatchDiagnostic>,
}

fn diagnostic(
    code: &str,
    operation_id: Option<&str>,
    detail: impl Into<String>,
) -> DispatchDiagnostic {
    DispatchDiagnostic {
        code: code.to_owned(),
        operation_id: operation_id.map(str::to_owned),
        detail: detail.into(),
    }
}

fn sort_diagnostics(diagnostics: &mut [DispatchDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.operation_id.cmp(&right.operation_id))
            .then_with(|| left.detail.cmp(&right.detail))
    });
}

fn overlaps(start_a: Tick, end_a: Tick, start_b: Tick, end_b: Tick) -> bool {
    start_a < end_b && start_b < end_a
}

fn validate_input(input: &DispatchInput) -> Result<ValidatedInput, Vec<DispatchDiagnostic>> {
    let mut errors = Vec::new();
    if input.horizon_end_tick <= input.decision_tick {
        errors.push(diagnostic(
            "INVALID_HORIZON",
            None,
            "horizon_end_tick must be greater than decision_tick",
        ));
    }

    let mut operations = BTreeMap::new();
    for operation in &input.operations {
        if operation.id.trim().is_empty() {
            errors.push(diagnostic(
                "EMPTY_OPERATION_ID",
                None,
                "operation IDs must be non-empty",
            ));
            continue;
        }
        if operation.duration_ticks == 0 {
            errors.push(diagnostic(
                "ZERO_OPERATION_DURATION",
                Some(&operation.id),
                "duration_ticks must be greater than zero",
            ));
        }
        if operation.resource_kind.trim().is_empty() {
            errors.push(diagnostic(
                "EMPTY_OPERATION_RESOURCE_KIND",
                Some(&operation.id),
                "resource_kind must be non-empty",
            ));
        }
        let mut predecessors = BTreeSet::new();
        for predecessor in &operation.predecessor_ids {
            if predecessor == &operation.id {
                errors.push(diagnostic(
                    "SELF_PRECEDENCE",
                    Some(&operation.id),
                    "an operation cannot be its own predecessor",
                ));
            }
            if !predecessors.insert(predecessor.as_str()) {
                errors.push(diagnostic(
                    "DUPLICATE_PREDECESSOR",
                    Some(&operation.id),
                    format!("predecessor {predecessor} is duplicated"),
                ));
            }
        }
        if operations
            .insert(operation.id.clone(), operation.clone())
            .is_some()
        {
            errors.push(diagnostic(
                "DUPLICATE_OPERATION_ID",
                Some(&operation.id),
                "operation IDs must be unique",
            ));
        }
    }

    let mut resources = BTreeMap::new();
    for resource in &input.resources {
        if resource.id.trim().is_empty() || resource.kind.trim().is_empty() {
            errors.push(diagnostic(
                "EMPTY_RESOURCE_FIELD",
                None,
                "resource IDs and kinds must be non-empty",
            ));
            continue;
        }
        if resources
            .insert(resource.id.clone(), resource.clone())
            .is_some()
        {
            errors.push(diagnostic(
                "DUPLICATE_RESOURCE_ID",
                None,
                format!("resource ID {} is duplicated", resource.id),
            ));
        }
    }

    if !errors.is_empty() {
        sort_diagnostics(&mut errors);
        return Err(errors);
    }

    for operation in operations.values() {
        for predecessor in &operation.predecessor_ids {
            if !operations.contains_key(predecessor) {
                errors.push(diagnostic(
                    "UNKNOWN_PREDECESSOR",
                    Some(&operation.id),
                    format!("predecessor {predecessor} is not in the operation inventory"),
                ));
            }
        }
    }
    if has_precedence_cycle(&operations) {
        errors.push(diagnostic(
            "PRECEDENCE_CYCLE",
            None,
            "the operation precedence graph contains a cycle",
        ));
    }

    let mut completed = BTreeMap::new();
    for item in &input.completed {
        if !operations.contains_key(&item.operation_id) {
            errors.push(diagnostic(
                "UNKNOWN_COMPLETED_OPERATION",
                Some(&item.operation_id),
                "completed operation is not in the operation inventory",
            ));
            continue;
        }
        if item.completed_at_tick > input.decision_tick {
            errors.push(diagnostic(
                "FUTURE_COMPLETION",
                Some(&item.operation_id),
                "completed_at_tick must not be after the current decision tick",
            ));
        }
        if completed
            .insert(item.operation_id.clone(), item.completed_at_tick)
            .is_some()
        {
            errors.push(diagnostic(
                "DUPLICATE_COMPLETION",
                Some(&item.operation_id),
                "an operation may be completed only once",
            ));
        }
    }

    let mut commitments = BTreeMap::new();
    for item in &input.commitments {
        let Some(operation) = operations.get(&item.operation_id) else {
            errors.push(diagnostic(
                "UNKNOWN_COMMITTED_OPERATION",
                Some(&item.operation_id),
                "committed operation is not in the operation inventory",
            ));
            continue;
        };
        let Some(resource) = resources.get(&item.resource_id) else {
            errors.push(diagnostic(
                "UNKNOWN_COMMITMENT_RESOURCE",
                Some(&item.operation_id),
                format!("resource {} is not in the resource inventory", item.resource_id),
            ));
            continue;
        };
        if completed.contains_key(&item.operation_id) {
            errors.push(diagnostic(
                "OPERATION_BOTH_COMPLETED_AND_COMMITTED",
                Some(&item.operation_id),
                "an operation cannot be both completed and committed",
            ));
        }
        if item.end_tick <= item.start_tick {
            errors.push(diagnostic(
                "INVALID_COMMITMENT_INTERVAL",
                Some(&item.operation_id),
                "commitment end_tick must be greater than start_tick",
            ));
            continue;
        }
        if item.end_tick <= input.decision_tick {
            errors.push(diagnostic(
                "STALE_COMMITMENT",
                Some(&item.operation_id),
                "a commitment ending at or before decision_tick belongs in completed work",
            ));
        }
        if item.end_tick - item.start_tick != operation.duration_ticks {
            errors.push(diagnostic(
                "COMMITMENT_DURATION_MISMATCH",
                Some(&item.operation_id),
                "frozen commitment interval must equal the operation's declared duration",
            ));
        }
        if resource.kind != operation.resource_kind {
            errors.push(diagnostic(
                "COMMITMENT_RESOURCE_KIND_MISMATCH",
                Some(&item.operation_id),
                "frozen commitment uses a resource of the wrong kind",
            ));
        }
        if operation.release_tick > item.start_tick {
            errors.push(diagnostic(
                "COMMITMENT_BEFORE_RELEASE",
                Some(&item.operation_id),
                "frozen commitment starts before the operation's release tick",
            ));
        }
        let assignment = Assignment {
            operation_id: item.operation_id.clone(),
            resource_id: item.resource_id.clone(),
            start_tick: item.start_tick,
            end_tick: item.end_tick,
            frozen: true,
        };
        if commitments
            .insert(item.operation_id.clone(), assignment)
            .is_some()
        {
            errors.push(diagnostic(
                "DUPLICATE_COMMITMENT",
                Some(&item.operation_id),
                "an operation may have only one frozen commitment",
            ));
        }
    }

    let mut visible_outages = Vec::new();
    let mut outage_ids = BTreeSet::new();
    for outage in &input.visible_outages {
        if outage.event_id.trim().is_empty() {
            errors.push(diagnostic(
                "EMPTY_OUTAGE_EVENT_ID",
                None,
                "visible outage event IDs must be non-empty",
            ));
        }
        if !outage_ids.insert(outage.event_id.as_str()) {
            errors.push(diagnostic(
                "DUPLICATE_OUTAGE_EVENT_ID",
                None,
                format!("outage event ID {} is duplicated", outage.event_id),
            ));
        }
        if !resources.contains_key(&outage.resource_id) {
            errors.push(diagnostic(
                "UNKNOWN_OUTAGE_RESOURCE",
                None,
                format!("outage {} references an unknown resource", outage.event_id),
            ));
        }
        if outage.observed_at_tick > input.decision_tick {
            errors.push(diagnostic(
                "FUTURE_OBSERVATION_IN_POLICY_INPUT",
                None,
                format!(
                    "outage {} was observed at tick {} after decision tick {}",
                    outage.event_id, outage.observed_at_tick, input.decision_tick
                ),
            ));
        }
        if outage.effective_until_tick <= outage.effective_from_tick {
            errors.push(diagnostic(
                "INVALID_OUTAGE_INTERVAL",
                None,
                format!("outage {} must have a positive duration", outage.event_id),
            ));
        }
        visible_outages.push(outage.clone());
    }

    for operation in operations.values() {
        if let Some(completed_at) = completed.get(&operation.id) {
            if operation.release_tick > *completed_at {
                errors.push(diagnostic(
                    "COMPLETED_BEFORE_RELEASE",
                    Some(&operation.id),
                    "completion time is before the declared release tick",
                ));
            }
            for predecessor in &operation.predecessor_ids {
                match completed.get(predecessor) {
                    Some(predecessor_end) if predecessor_end <= completed_at => {}
                    Some(_) => errors.push(diagnostic(
                        "COMPLETED_PRECEDENCE_VIOLATION",
                        Some(&operation.id),
                        format!("predecessor {predecessor} completed after this operation"),
                    )),
                    None => errors.push(diagnostic(
                        "COMPLETED_PREDECESSOR_NOT_COMPLETED",
                        Some(&operation.id),
                        format!("completed operation has non-completed predecessor {predecessor}"),
                    )),
                }
            }
        }
        if let Some(commitment) = commitments.get(&operation.id) {
            for predecessor in &operation.predecessor_ids {
                if let Some(predecessor_completion) = completed.get(predecessor) {
                    if *predecessor_completion > commitment.start_tick {
                        errors.push(diagnostic(
                            "COMMITTED_PRECEDENCE_VIOLATION",
                            Some(&operation.id),
                            format!("predecessor {predecessor} completes after commitment start"),
                        ));
                    }
                } else if let Some(predecessor_commitment) = commitments.get(predecessor) {
                    if predecessor_commitment.end_tick > commitment.start_tick {
                        errors.push(diagnostic(
                            "COMMITTED_PRECEDENCE_VIOLATION",
                            Some(&operation.id),
                            format!("predecessor {predecessor} ends after commitment start"),
                        ));
                    }
                } else {
                    errors.push(diagnostic(
                        "FROZEN_COMMITMENT_NOT_PRECEDENCE_CLOSED",
                        Some(&operation.id),
                        format!(
                            "frozen commitment requires predecessor {predecessor} to be completed or frozen"
                        ),
                    ));
                }
            }
        }
    }

    if !errors.is_empty() {
        sort_diagnostics(&mut errors);
        return Err(errors);
    }

    let mut frozen_conflicts = Vec::new();
    let commitment_values: Vec<&Assignment> = commitments.values().collect();
    for (index, left) in commitment_values.iter().enumerate() {
        for right in commitment_values.iter().skip(index + 1) {
            if left.resource_id == right.resource_id
                && overlaps(left.start_tick, left.end_tick, right.start_tick, right.end_tick)
            {
                frozen_conflicts.push(diagnostic(
                    "FROZEN_COMMITMENTS_OVERLAP",
                    Some(&right.operation_id),
                    format!(
                        "frozen operations {} and {} overlap on resource {}",
                        left.operation_id, right.operation_id, left.resource_id
                    ),
                ));
            }
        }
        for outage in visible_outages
            .iter()
            .filter(|outage| outage.resource_id == left.resource_id)
        {
            if overlaps(
                left.start_tick,
                left.end_tick,
                outage.effective_from_tick,
                outage.effective_until_tick,
            ) {
                frozen_conflicts.push(diagnostic(
                    "FROZEN_COMMITMENT_OVERLAPS_VISIBLE_OUTAGE",
                    Some(&left.operation_id),
                    format!(
                        "frozen commitment overlaps observed outage {} on resource {}",
                        outage.event_id, left.resource_id
                    ),
                ));
            }
        }
    }
    sort_diagnostics(&mut frozen_conflicts);

    Ok(ValidatedInput {
        operations,
        resources,
        completed,
        commitments,
        visible_outages,
        frozen_conflicts,
    })
}

fn has_precedence_cycle(operations: &BTreeMap<String, Operation>) -> bool {
    let mut indegree: BTreeMap<String, usize> =
        operations.keys().map(|id| (id.clone(), 0usize)).collect();
    let mut children: BTreeMap<String, Vec<String>> =
        operations.keys().map(|id| (id.clone(), Vec::new())).collect();

    for operation in operations.values() {
        for predecessor in &operation.predecessor_ids {
            if let Some(count) = indegree.get_mut(&operation.id) {
                *count += 1;
            }
            if let Some(next) = children.get_mut(predecessor) {
                next.push(operation.id.clone());
            }
        }
    }
    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut visited = 0usize;
    while let Some(id) = ready.iter().next().cloned() {
        ready.remove(&id);
        visited += 1;
        if let Some(next_items) = children.get(&id) {
            for child in next_items {
                if let Some(count) = indegree.get_mut(child) {
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(child.clone());
                    }
                }
            }
        }
    }
    visited != operations.len()
}

fn signed_difference(left: Tick, right: Tick) -> i128 {
    left as i128 - right as i128
}

fn compare_nonnegative_fractions(
    left_numerator: u128,
    left_denominator: u64,
    right_numerator: u128,
    right_denominator: u64,
) -> Ordering {
    (left_numerator * right_denominator as u128)
        .cmp(&(right_numerator * left_denominator as u128))
}

fn compare_signed_fractions(
    left_numerator: i128,
    left_denominator: u64,
    right_numerator: i128,
    right_denominator: u64,
) -> Ordering {
    if left_numerator.is_negative() != right_numerator.is_negative() {
        return left_numerator.cmp(&right_numerator);
    }
    if left_numerator.is_negative() {
        compare_nonnegative_fractions(
            right_numerator.unsigned_abs(),
            right_denominator,
            left_numerator.unsigned_abs(),
            left_denominator,
        )
    } else {
        compare_nonnegative_fractions(
            left_numerator as u128,
            left_denominator,
            right_numerator as u128,
            right_denominator,
        )
    }
}

fn earliest_start(
    operation: &Operation,
    decision_tick: Tick,
    completed: &BTreeMap<String, Tick>,
    commitments: &BTreeMap<String, Assignment>,
    planned_ends: &BTreeMap<String, Tick>,
) -> Option<Tick> {
    let mut start = decision_tick.max(operation.release_tick);
    for predecessor in &operation.predecessor_ids {
        let end = planned_ends
            .get(predecessor)
            .or_else(|| completed.get(predecessor))
            .or_else(|| commitments.get(predecessor).map(|assignment| &assignment.end_tick))?;
        start = start.max(*end);
    }
    Some(start)
}

fn priority_cmp(
    rule: DispatchRule,
    left: &Operation,
    right: &Operation,
    left_earliest: Tick,
    right_earliest: Tick,
    decision_tick: Tick,
) -> Ordering {
    let primary = match rule {
        DispatchRule::FixedOrder => left.order.cmp(&right.order),
        DispatchRule::EarliestDueDate => left.due_tick.cmp(&right.due_tick),
        DispatchRule::CriticalRatio => compare_signed_fractions(
            signed_difference(left.due_tick, decision_tick),
            left.duration_ticks,
            signed_difference(right.due_tick, decision_tick),
            right.duration_ticks,
        ),
        DispatchRule::LeastSlack => {
            let left_slack = left.due_tick as i128
                - left_earliest as i128
                - left.duration_ticks as i128;
            let right_slack = right.due_tick as i128
                - right_earliest as i128
                - right.duration_ticks as i128;
            left_slack.cmp(&right_slack)
        }
    };
    primary
        .then_with(|| left.due_tick.cmp(&right.due_tick))
        .then_with(|| left.order.cmp(&right.order))
        .then_with(|| left.id.cmp(&right.id))
}

fn find_slot_on_resource(
    operation: &Operation,
    resource: &Resource,
    earliest: Tick,
    horizon_end_tick: Tick,
    assignments: &[Assignment],
    visible_outages: &[VisibleOutage],
) -> Option<(Tick, Tick)> {
    let mut start = earliest;
    loop {
        let end = start.checked_add(operation.duration_ticks)?;
        if end > horizon_end_tick {
            return None;
        }
        let mut next_start = start;
        for assignment in assignments
            .iter()
            .filter(|item| item.resource_id == resource.id)
        {
            if overlaps(start, end, assignment.start_tick, assignment.end_tick) {
                next_start = next_start.max(assignment.end_tick);
            }
        }
        for outage in visible_outages
            .iter()
            .filter(|item| item.resource_id == resource.id)
        {
            if overlaps(
                start,
                end,
                outage.effective_from_tick,
                outage.effective_until_tick,
            ) {
                next_start = next_start.max(outage.effective_until_tick);
            }
        }
        if next_start == start {
            return Some((start, end));
        }
        if next_start <= start {
            return None;
        }
        start = next_start;
    }
}

fn sort_assignments(assignments: &mut [Assignment]) {
    assignments.sort_by(|left, right| {
        left.start_tick
            .cmp(&right.start_tick)
            .then_with(|| left.resource_id.cmp(&right.resource_id))
            .then_with(|| left.operation_id.cmp(&right.operation_id))
            .then_with(|| right.frozen.cmp(&left.frozen))
    });
}

fn invalid_plan(
    input: &DispatchInput,
    rule: DispatchRule,
    diagnostics: Vec<DispatchDiagnostic>,
) -> DispatchPlan {
    let mut unscheduled: Vec<String> = input
        .operations
        .iter()
        .map(|operation| operation.id.clone())
        .collect();
    unscheduled.sort();
    let mut diagnostics = diagnostics;
    sort_diagnostics(&mut diagnostics);
    DispatchPlan {
        status: PlanStatus::InvalidInput,
        rule,
        decision_tick: input.decision_tick,
        horizon_end_tick: input.horizon_end_tick,
        assignments: Vec::new(),
        unscheduled_operation_ids: unscheduled,
        diagnostics,
    }
}

/// Build a deterministic rolling-horizon proposal from only the supplied
/// observation snapshot. Re-run at every decision epoch; freeze executed work
/// through the input's completed and commitment inventories.
///
/// The algorithm is a serial schedule-generation heuristic. It does not prove
/// infeasibility when work cannot be placed within the current horizon.
pub fn build_dispatch_plan(input: &DispatchInput, rule: DispatchRule) -> DispatchPlan {
    let valid = match validate_input(input) {
        Ok(valid) => valid,
        Err(diagnostics) => return invalid_plan(input, rule, diagnostics),
    };

    let mut assignments: Vec<Assignment> = valid.commitments.values().cloned().collect();
    sort_assignments(&mut assignments);
    let mut planned_ends = valid.completed.clone();
    for assignment in valid.commitments.values() {
        planned_ends.insert(assignment.operation_id.clone(), assignment.end_tick);
    }
    let mut remaining: BTreeSet<String> = valid
        .operations
        .keys()
        .filter(|id| !valid.completed.contains_key(*id) && !valid.commitments.contains_key(*id))
        .cloned()
        .collect();
    let mut blocked = BTreeMap::<String, String>::new();
    let mut diagnostics = valid.frozen_conflicts.clone();

    while !remaining.is_empty() {
        let mut ready: Vec<(String, Tick)> = remaining
            .iter()
            .filter(|id| !blocked.contains_key(*id))
            .filter_map(|id| {
                let operation = valid.operations.get(id)?;
                earliest_start(
                    operation,
                    input.decision_tick,
                    &valid.completed,
                    &valid.commitments,
                    &planned_ends,
                )
                .map(|start| (id.clone(), start))
            })
            .collect();
        if ready.is_empty() {
            break;
        }
        ready.sort_by(|(left_id, left_start), (right_id, right_start)| {
            let left = &valid.operations[left_id];
            let right = &valid.operations[right_id];
            priority_cmp(
                rule,
                left,
                right,
                *left_start,
                *right_start,
                input.decision_tick,
            )
        });

        let (operation_id, earliest) = ready[0].clone();
        let operation = &valid.operations[&operation_id];
        let mut best: Option<(Tick, String, Tick)> = None;
        let mut has_compatible_resource = false;
        for resource in valid
            .resources
            .values()
            .filter(|resource| resource.kind == operation.resource_kind)
        {
            has_compatible_resource = true;
            if let Some((start, end)) = find_slot_on_resource(
                operation,
                resource,
                earliest,
                input.horizon_end_tick,
                &assignments,
                &valid.visible_outages,
            ) {
                let candidate = (start, resource.id.clone(), end);
                let replace = best.as_ref().is_none_or(|current| {
                    candidate.0 < current.0
                        || (candidate.0 == current.0 && candidate.1 < current.1)
                });
                if replace {
                    best = Some(candidate);
                }
            }
        }

        match best {
            Some((start_tick, resource_id, end_tick)) => {
                assignments.push(Assignment {
                    operation_id: operation_id.clone(),
                    resource_id,
                    start_tick,
                    end_tick,
                    frozen: false,
                });
                planned_ends.insert(operation_id.clone(), end_tick);
                remaining.remove(&operation_id);
            }
            None => {
                let reason = if has_compatible_resource {
                    "no compatible resource has a slot within the declared horizon"
                } else {
                    "no resource of the required kind is in the declared inventory"
                };
                blocked.insert(operation_id.clone(), reason.to_owned());
                diagnostics.push(diagnostic(
                    if has_compatible_resource {
                        "NO_SLOT_WITHIN_HORIZON"
                    } else {
                        "NO_COMPATIBLE_RESOURCE"
                    },
                    Some(&operation_id),
                    reason,
                ));
            }
        }
    }

    let unscheduled_operation_ids: Vec<String> = remaining.iter().cloned().collect();
    for operation_id in &unscheduled_operation_ids {
        if !blocked.contains_key(operation_id) {
            diagnostics.push(diagnostic(
                "UNSCHEDULED_PREDECESSOR_OR_HORIZON_DEPENDENCY",
                Some(operation_id),
                "operation remains unscheduled because a predecessor could not be planned",
            ));
        }
    }

    sort_assignments(&mut assignments);
    sort_diagnostics(&mut diagnostics);
    let status = if !valid.frozen_conflicts.is_empty() {
        PlanStatus::FrozenConflict
    } else if unscheduled_operation_ids.is_empty() {
        PlanStatus::Complete
    } else {
        PlanStatus::Partial
    };

    DispatchPlan {
        status,
        rule,
        decision_tick: input.decision_tick,
        horizon_end_tick: input.horizon_end_tick,
        assignments,
        unscheduled_operation_ids,
        diagnostics,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(
        id: &str,
        order: u64,
        due_tick: Tick,
        duration_ticks: Tick,
        kind: &str,
        predecessors: &[&str],
    ) -> Operation {
        Operation {
            id: id.to_owned(),
            order,
            release_tick: 0,
            due_tick,
            duration_ticks,
            resource_kind: kind.to_owned(),
            predecessor_ids: predecessors.iter().map(|item| (*item).to_owned()).collect(),
        }
    }

    fn input(operations: Vec<Operation>, resources: Vec<Resource>) -> DispatchInput {
        DispatchInput {
            decision_tick: 0,
            horizon_end_tick: 30,
            operations,
            resources,
            completed: Vec::new(),
            commitments: Vec::new(),
            visible_outages: Vec::new(),
        }
    }

    fn resource(id: &str, kind: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            kind: kind.to_owned(),
        }
    }

    fn outage(
        id: &str,
        resource_id: &str,
        from: Tick,
        until: Tick,
        observed_at: Tick,
    ) -> VisibleOutage {
        VisibleOutage {
            event_id: id.to_owned(),
            resource_id: resource_id.to_owned(),
            effective_from_tick: from,
            effective_until_tick: until,
            observed_at_tick: observed_at,
        }
    }

    fn assignment<'a>(plan: &'a DispatchPlan, operation_id: &str) -> &'a Assignment {
        plan.assignments
            .iter()
            .find(|item| item.operation_id == operation_id)
            .expect("assignment should exist")
    }

    #[test]
    fn earliest_due_date_ignores_input_order_and_uses_stable_ties() {
        let operations = vec![
            operation("late", 0, 20, 1, "welder", &[]),
            operation("urgent-b", 2, 5, 1, "welder", &[]),
            operation("urgent-a", 1, 5, 1, "welder", &[]),
        ];
        let resources = vec![resource("weld-1", "welder")];
        let plan = build_dispatch_plan(
            &input(operations, resources.clone()),
            DispatchRule::EarliestDueDate,
        );
        assert_eq!(plan.status, PlanStatus::Complete);
        assert_eq!(assignment(&plan, "urgent-a").start_tick, 0);
        assert_eq!(assignment(&plan, "urgent-b").start_tick, 1);
        assert_eq!(assignment(&plan, "late").start_tick, 2);

        let reversed = input(
            vec![
                operation("urgent-a", 1, 5, 1, "welder", &[]),
                operation("urgent-b", 2, 5, 1, "welder", &[]),
                operation("late", 0, 20, 1, "welder", &[]),
            ],
            resources,
        );
        assert_eq!(plan, build_dispatch_plan(&reversed, DispatchRule::EarliestDueDate));
    }

    #[test]
    fn visible_future_outage_blocks_overlapping_assignment() {
        let mut scenario = input(
            vec![operation("long-weld", 0, 15, 4, "welder", &[])],
            vec![resource("weld-1", "welder")],
        );
        scenario.visible_outages.push(outage("out-1", "weld-1", 2, 5, 0));
        let plan = build_dispatch_plan(&scenario, DispatchRule::EarliestDueDate);
        assert_eq!(plan.status, PlanStatus::Complete);
        assert_eq!(assignment(&plan, "long-weld").start_tick, 5);
        assert_eq!(assignment(&plan, "long-weld").end_tick, 9);
    }

    #[test]
    fn not_yet_observed_outage_cannot_enter_policy_input() {
        let mut scenario = input(
            vec![operation("weld", 0, 10, 1, "welder", &[])],
            vec![resource("weld-1", "welder")],
        );
        scenario.visible_outages.push(outage("future-event", "weld-1", 5, 8, 3));
        scenario.decision_tick = 2;
        let plan = build_dispatch_plan(&scenario, DispatchRule::EarliestDueDate);
        assert_eq!(plan.status, PlanStatus::InvalidInput);
        assert!(plan
            .diagnostics
            .iter()
            .any(|item| item.code == "FUTURE_OBSERVATION_IN_POLICY_INPUT"));
    }

    #[test]
    fn same_visible_prefix_produces_the_same_plan() {
        let base = input(
            vec![
                operation("a", 0, 8, 2, "welder", &[]),
                operation("b", 1, 14, 3, "welder", &[]),
            ],
            vec![resource("weld-1", "welder")],
        );
        let first = build_dispatch_plan(&base, DispatchRule::EarliestDueDate);
        // Hidden future truth is deliberately not a field on DispatchInput.
        let second = build_dispatch_plan(&base, DispatchRule::EarliestDueDate);
        assert_eq!(first, second);
    }

    #[test]
    fn frozen_commitments_are_never_reassigned_or_shifted() {
        let mut scenario = input(
            vec![
                operation("in-flight", 0, 10, 3, "welder", &[]),
                operation("waiting", 1, 12, 2, "welder", &[]),
            ],
            vec![resource("weld-1", "welder")],
        );
        scenario.decision_tick = 1;
        scenario.commitments.push(Commitment {
            operation_id: "in-flight".to_owned(),
            resource_id: "weld-1".to_owned(),
            start_tick: 0,
            end_tick: 3,
        });
        let plan = build_dispatch_plan(&scenario, DispatchRule::EarliestDueDate);
        assert_eq!(plan.status, PlanStatus::Complete);
        let frozen = assignment(&plan, "in-flight");
        assert!(frozen.frozen);
        assert_eq!((frozen.start_tick, frozen.end_tick), (0, 3));
        assert_eq!(assignment(&plan, "waiting").start_tick, 3);
    }

    #[test]
    fn precedence_release_and_resource_constraints_are_respected() {
        let mut scenario = input(
            vec![
                operation("paint", 0, 20, 2, "paint", &["weld"]),
                operation("weld", 1, 12, 3, "welder", &[]),
            ],
            vec![resource("weld-1", "welder"), resource("paint-1", "paint")],
        );
        scenario.operations[0].release_tick = 1;
        let plan = build_dispatch_plan(&scenario, DispatchRule::EarliestDueDate);
        assert_eq!(plan.status, PlanStatus::Complete);
        assert!(assignment(&plan, "paint").start_tick >= assignment(&plan, "weld").end_tick);
        assert!(assignment(&plan, "paint").start_tick >= 1);
    }

    #[test]
    fn critical_ratio_and_least_slack_produce_deterministic_rules() {
        let scenario = input(
            vec![
                operation("short-urgent", 1, 7, 2, "welder", &[]),
                operation("longer-later", 0, 10, 5, "welder", &[]),
            ],
            vec![resource("weld-1", "welder")],
        );
        let critical = build_dispatch_plan(&scenario, DispatchRule::CriticalRatio);
        let slack = build_dispatch_plan(&scenario, DispatchRule::LeastSlack);
        assert_eq!(assignment(&critical, "longer-later").start_tick, 0);
        assert_eq!(assignment(&slack, "short-urgent").start_tick, 0);
        assert_eq!(critical, build_dispatch_plan(&scenario, DispatchRule::CriticalRatio));
    }

    #[test]
    fn no_resource_or_no_horizon_slot_is_partial_not_infeasibility() {
        let no_resource = input(
            vec![operation("needs-crane", 0, 10, 2, "crane", &[])],
            vec![resource("weld-1", "welder")],
        );
        let plan = build_dispatch_plan(&no_resource, DispatchRule::EarliestDueDate);
        assert_eq!(plan.status, PlanStatus::Partial);
        assert_eq!(plan.unscheduled_operation_ids, vec!["needs-crane"]);
        assert!(plan.diagnostics.iter().any(|item| item.code == "NO_COMPATIBLE_RESOURCE"));

        let mut too_short = input(
            vec![operation("long", 0, 10, 4, "welder", &[])],
            vec![resource("weld-1", "welder")],
        );
        too_short.horizon_end_tick = 3;
        let plan = build_dispatch_plan(&too_short, DispatchRule::EarliestDueDate);
        assert_eq!(plan.status, PlanStatus::Partial);
        assert!(plan.diagnostics.iter().any(|item| item.code == "NO_SLOT_WITHIN_HORIZON"));
        assert!(!plan.diagnostics.iter().any(|item| item.code == "INFEASIBLE_PROVEN"));
    }

    #[test]
    fn precedence_cycle_is_rejected() {
        let scenario = input(
            vec![
                operation("a", 0, 10, 1, "welder", &["b"]),
                operation("b", 1, 10, 1, "welder", &["a"]),
            ],
            vec![resource("weld-1", "welder")],
        );
        let plan = build_dispatch_plan(&scenario, DispatchRule::EarliestDueDate);
        assert_eq!(plan.status, PlanStatus::InvalidInput);
        assert!(plan.diagnostics.iter().any(|item| item.code == "PRECEDENCE_CYCLE"));
    }

    #[test]
    fn conflicting_frozen_commitments_are_visible_not_silently_repaired() {
        let mut scenario = input(
            vec![
                operation("a", 0, 10, 4, "welder", &[]),
                operation("b", 1, 12, 4, "welder", &[]),
                operation("c", 2, 15, 1, "welder", &[]),
            ],
            vec![resource("weld-1", "welder")],
        );
        scenario.decision_tick = 1;
        scenario.commitments = vec![
            Commitment {
                operation_id: "a".to_owned(),
                resource_id: "weld-1".to_owned(),
                start_tick: 0,
                end_tick: 4,
            },
            Commitment {
                operation_id: "b".to_owned(),
                resource_id: "weld-1".to_owned(),
                start_tick: 2,
                end_tick: 6,
            },
        ];
        let plan = build_dispatch_plan(&scenario, DispatchRule::EarliestDueDate);
        assert_eq!(plan.status, PlanStatus::FrozenConflict);
        assert_eq!((assignment(&plan, "a").start_tick, assignment(&plan, "a").end_tick), (0, 4));
        assert_eq!((assignment(&plan, "b").start_tick, assignment(&plan, "b").end_tick), (2, 6));
        assert!(plan.diagnostics.iter().any(|item| item.code == "FROZEN_COMMITMENTS_OVERLAP"));
    }

    #[test]
    fn commitment_outage_conflict_is_preserved_and_reported() {
        let mut scenario = input(
            vec![
                operation("in-flight", 0, 10, 5, "welder", &[]),
                operation("next", 1, 15, 1, "welder", &[]),
            ],
            vec![resource("weld-1", "welder")],
        );
        scenario.decision_tick = 3;
        scenario.commitments.push(Commitment {
            operation_id: "in-flight".to_owned(),
            resource_id: "weld-1".to_owned(),
            start_tick: 0,
            end_tick: 5,
        });
        scenario.visible_outages.push(outage("out", "weld-1", 2, 8, 3));
        let plan = build_dispatch_plan(&scenario, DispatchRule::EarliestDueDate);
        assert_eq!(plan.status, PlanStatus::FrozenConflict);
        assert!(assignment(&plan, "in-flight").frozen);
        assert_eq!(assignment(&plan, "next").start_tick, 8);
    }

    #[test]
    fn input_order_does_not_change_plan_or_diagnostics() {
        let first = input(
            vec![
                operation("b", 1, 7, 2, "welder", &[]),
                operation("a", 0, 6, 1, "welder", &[]),
            ],
            vec![resource("weld-2", "welder"), resource("weld-1", "welder")],
        );
        let second = input(
            vec![
                operation("a", 0, 6, 1, "welder", &[]),
                operation("b", 1, 7, 2, "welder", &[]),
            ],
            vec![resource("weld-1", "welder"), resource("weld-2", "welder")],
        );
        assert_eq!(
            build_dispatch_plan(&first, DispatchRule::EarliestDueDate),
            build_dispatch_plan(&second, DispatchRule::EarliestDueDate)
        );
    }
}
