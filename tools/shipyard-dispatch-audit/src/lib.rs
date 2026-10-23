//! Independent abstract-schedule evaluator for shipyard recovery policies.
//!
//! This crate deliberately does not depend on the dispatch-policy crate. It
//! checks a proposal against separately supplied instance facts and evaluator
//! truth, retaining every violation instead of repairing the candidate plan.
//! Its feasible status is abstract-model feasibility only, never a physical or
//! industrial safety certification.

use std::collections::{BTreeMap, BTreeSet};

pub type Tick = u64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operation {
    pub id: String,
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
    pub resource_id: String,
    pub start_tick: Tick,
    pub completed_at_tick: Tick,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenCommitment {
    pub operation_id: String,
    pub resource_id: String,
    pub start_tick: Tick,
    pub end_tick: Tick,
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
pub struct EvaluatorOutage {
    pub event_id: String,
    pub resource_id: String,
    pub effective_from_tick: Tick,
    pub effective_until_tick: Tick,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditInput {
    pub decision_tick: Tick,
    pub horizon_end_tick: Tick,
    pub operations: Vec<Operation>,
    pub resources: Vec<Resource>,
    pub completed: Vec<Completion>,
    /// Immutable expected frozen commitments from the source-owned state.
    pub frozen_commitments: Vec<FrozenCommitment>,
    /// Candidate output. Candidate records are untrusted.
    pub assignments: Vec<Assignment>,
    /// Evaluator-side truth, which may include not-yet-observed disturbances.
    pub evaluator_outages: Vec<EvaluatorOutage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditStatus {
    AbstractFeasible,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditDiagnostic {
    pub code: String,
    pub operation_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditMetrics {
    pub makespan_tick: Option<Tick>,
    pub total_tardiness_ticks: Tick,
    pub occupied_resource_ticks: Tick,
    pub completed_operation_count: usize,
    pub assigned_operation_count: usize,
    pub mandatory_operation_count: usize,
    pub missing_operation_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    pub status: AuditStatus,
    pub decision_tick: Tick,
    pub horizon_end_tick: Tick,
    pub diagnostics: Vec<AuditDiagnostic>,
    pub metrics: AuditMetrics,
}

fn push(
    diagnostics: &mut Vec<AuditDiagnostic>,
    code: &str,
    operation_id: Option<&str>,
    detail: impl Into<String>,
) {
    diagnostics.push(AuditDiagnostic {
        code: code.to_owned(),
        operation_id: operation_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn sort_diagnostics(diagnostics: &mut [AuditDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.operation_id.cmp(&right.operation_id))
            .then_with(|| left.detail.cmp(&right.detail))
    });
}

fn overlaps(a_start: Tick, a_end: Tick, b_start: Tick, b_end: Tick) -> bool {
    a_start < b_end && b_start < a_end
}

fn metric_add(total: &mut Tick, amount: Tick, diagnostics: &mut Vec<AuditDiagnostic>, code: &str) {
    match total.checked_add(amount) {
        Some(value) => *total = value,
        None => push(
            diagnostics,
            code,
            None,
            "metric sum overflowed u64; report is not admissible",
        ),
    }
}

/// Independently evaluate one candidate schedule under the declared abstract
/// resource/predecessence model. The result never modifies or repairs input.
pub fn audit_schedule(input: &AuditInput) -> AuditReport {
    let mut diagnostics = Vec::new();
    if input.horizon_end_tick <= input.decision_tick {
        push(
            &mut diagnostics,
            "INVALID_HORIZON",
            None,
            "horizon_end_tick must be greater than decision_tick",
        );
    }

    let mut operation_map = BTreeMap::<String, Operation>::new();
    for operation in &input.operations {
        if operation.id.trim().is_empty() {
            push(
                &mut diagnostics,
                "EMPTY_OPERATION_ID",
                None,
                "operation IDs must be non-empty",
            );
            continue;
        }
        if operation.duration_ticks == 0 {
            push(
                &mut diagnostics,
                "ZERO_OPERATION_DURATION",
                Some(&operation.id),
                "duration_ticks must be positive",
            );
        }
        if operation.resource_kind.trim().is_empty() {
            push(
                &mut diagnostics,
                "EMPTY_OPERATION_RESOURCE_KIND",
                Some(&operation.id),
                "resource kind must be non-empty",
            );
        }
        if operation_map
            .insert(operation.id.clone(), operation.clone())
            .is_some()
        {
            push(
                &mut diagnostics,
                "DUPLICATE_OPERATION_ID",
                Some(&operation.id),
                "instance operation IDs must be unique",
            );
        }
    }

    let mut resource_map = BTreeMap::<String, Resource>::new();
    for resource in &input.resources {
        if resource.id.trim().is_empty() || resource.kind.trim().is_empty() {
            push(
                &mut diagnostics,
                "EMPTY_RESOURCE_FIELD",
                None,
                "resource ID and kind must be non-empty",
            );
            continue;
        }
        if resource_map
            .insert(resource.id.clone(), resource.clone())
            .is_some()
        {
            push(
                &mut diagnostics,
                "DUPLICATE_RESOURCE_ID",
                None,
                format!("resource {} is duplicated", resource.id),
            );
        }
    }

    for operation in operation_map.values() {
        let mut seen = BTreeSet::new();
        for predecessor in &operation.predecessor_ids {
            if !operation_map.contains_key(predecessor) {
                push(
                    &mut diagnostics,
                    "UNKNOWN_PREDECESSOR",
                    Some(&operation.id),
                    format!("predecessor {predecessor} is not in the instance"),
                );
            }
            if predecessor == &operation.id {
                push(
                    &mut diagnostics,
                    "SELF_PRECEDENCE",
                    Some(&operation.id),
                    "an operation cannot precede itself",
                );
            }
            if !seen.insert(predecessor) {
                push(
                    &mut diagnostics,
                    "DUPLICATE_PREDECESSOR",
                    Some(&operation.id),
                    format!("predecessor {predecessor} is duplicated"),
                );
            }
        }
    }

    let mut completion_map = BTreeMap::<String, Tick>::new();
    let mut completion_intervals = BTreeMap::<String, Assignment>::new();
    for completion in &input.completed {
        let Some(operation) = operation_map.get(&completion.operation_id) else {
            push(
                &mut diagnostics,
                "UNKNOWN_COMPLETED_OPERATION",
                Some(&completion.operation_id),
                "completed operation is not in instance",
            );
            continue;
        };
        let Some(resource) = resource_map.get(&completion.resource_id) else {
            push(
                &mut diagnostics,
                "UNKNOWN_COMPLETED_RESOURCE",
                Some(&completion.operation_id),
                format!(
                    "completed work references unknown resource {}",
                    completion.resource_id
                ),
            );
            continue;
        };
        if completion.completed_at_tick > input.decision_tick {
            push(
                &mut diagnostics,
                "FUTURE_COMPLETION",
                Some(&completion.operation_id),
                "completion is after the declared decision tick",
            );
        }
        if completion.start_tick >= completion.completed_at_tick {
            push(
                &mut diagnostics,
                "INVALID_COMPLETION_INTERVAL",
                Some(&completion.operation_id),
                "completed work must have a positive start-to-completion interval",
            );
        } else if completion.completed_at_tick - completion.start_tick != operation.duration_ticks {
            push(
                &mut diagnostics,
                "COMPLETED_DURATION_MISMATCH",
                Some(&completion.operation_id),
                "historical completion interval differs from declared operation duration",
            );
        }
        if completion.start_tick < operation.release_tick {
            push(
                &mut diagnostics,
                "COMPLETED_STARTS_BEFORE_RELEASE",
                Some(&completion.operation_id),
                "historical operation starts before its release tick",
            );
        }
        if resource.kind != operation.resource_kind {
            push(
                &mut diagnostics,
                "COMPLETED_RESOURCE_KIND_MISMATCH",
                Some(&completion.operation_id),
                "historical completion uses a resource of the wrong kind",
            );
        }
        if completion_map
            .insert(
                completion.operation_id.clone(),
                completion.completed_at_tick,
            )
            .is_some()
        {
            push(
                &mut diagnostics,
                "DUPLICATE_COMPLETION",
                Some(&completion.operation_id),
                "an operation can be completed only once",
            );
        }
        completion_intervals.insert(
            completion.operation_id.clone(),
            Assignment {
                operation_id: completion.operation_id.clone(),
                resource_id: completion.resource_id.clone(),
                start_tick: completion.start_tick,
                end_tick: completion.completed_at_tick,
                frozen: false,
            },
        );
    }

    let mut expected_frozen = BTreeMap::<String, FrozenCommitment>::new();
    for frozen in &input.frozen_commitments {
        if !operation_map.contains_key(&frozen.operation_id) {
            push(
                &mut diagnostics,
                "UNKNOWN_EXPECTED_FROZEN_OPERATION",
                Some(&frozen.operation_id),
                "expected frozen operation is not in instance",
            );
        }
        if !resource_map.contains_key(&frozen.resource_id) {
            push(
                &mut diagnostics,
                "UNKNOWN_EXPECTED_FROZEN_RESOURCE",
                Some(&frozen.operation_id),
                "expected frozen resource is not in instance",
            );
        }
        if frozen.end_tick <= frozen.start_tick {
            push(
                &mut diagnostics,
                "INVALID_EXPECTED_FROZEN_INTERVAL",
                Some(&frozen.operation_id),
                "expected frozen interval must be positive",
            );
        }
        if expected_frozen
            .insert(frozen.operation_id.clone(), frozen.clone())
            .is_some()
        {
            push(
                &mut diagnostics,
                "DUPLICATE_EXPECTED_FROZEN_OPERATION",
                Some(&frozen.operation_id),
                "source frozen commitment inventory must be unique per operation",
            );
        }
    }

    let mut assignment_by_operation = BTreeMap::<String, Assignment>::new();
    for assignment in &input.assignments {
        if !operation_map.contains_key(&assignment.operation_id) {
            push(
                &mut diagnostics,
                "UNKNOWN_ASSIGNED_OPERATION",
                Some(&assignment.operation_id),
                "candidate assignment references an operation outside the instance",
            );
            continue;
        }
        if assignment_by_operation
            .insert(assignment.operation_id.clone(), assignment.clone())
            .is_some()
        {
            push(
                &mut diagnostics,
                "DUPLICATE_OPERATION_ASSIGNMENT",
                Some(&assignment.operation_id),
                "candidate may assign an operation at most once",
            );
        }
        let Some(operation) = operation_map.get(&assignment.operation_id) else {
            continue;
        };
        let Some(resource) = resource_map.get(&assignment.resource_id) else {
            push(
                &mut diagnostics,
                "UNKNOWN_ASSIGNED_RESOURCE",
                Some(&assignment.operation_id),
                format!("resource {} is not in instance", assignment.resource_id),
            );
            continue;
        };

        if completion_map.contains_key(&assignment.operation_id) {
            push(
                &mut diagnostics,
                "OPERATION_BOTH_COMPLETED_AND_ASSIGNED",
                Some(&assignment.operation_id),
                "completed operation must not also appear in candidate assignments",
            );
        }
        if assignment.end_tick <= assignment.start_tick {
            push(
                &mut diagnostics,
                "INVALID_ASSIGNMENT_INTERVAL",
                Some(&assignment.operation_id),
                "assignment end must be greater than start",
            );
            continue;
        }
        if assignment.end_tick - assignment.start_tick != operation.duration_ticks {
            push(
                &mut diagnostics,
                "DURATION_MISMATCH",
                Some(&assignment.operation_id),
                "candidate interval length differs from operation duration",
            );
        }
        if resource.kind != operation.resource_kind {
            push(
                &mut diagnostics,
                "RESOURCE_KIND_MISMATCH",
                Some(&assignment.operation_id),
                "candidate resource is not compatible with operation kind",
            );
        }
        if assignment.start_tick < operation.release_tick {
            push(
                &mut diagnostics,
                "STARTS_BEFORE_RELEASE",
                Some(&assignment.operation_id),
                "candidate starts before operation release",
            );
        }
        if !assignment.frozen && assignment.start_tick < input.decision_tick {
            push(
                &mut diagnostics,
                "NEW_ASSIGNMENT_BEFORE_DECISION",
                Some(&assignment.operation_id),
                "a new assignment cannot be backdated before the decision tick",
            );
        }
        if assignment.end_tick > input.horizon_end_tick {
            push(
                &mut diagnostics,
                "ASSIGNMENT_AFTER_HORIZON",
                Some(&assignment.operation_id),
                "assignment ends after the declared horizon",
            );
        }
    }

    // Exact preservation of the immutable commitment set.
    for (operation_id, expected) in &expected_frozen {
        match assignment_by_operation.get(operation_id) {
            Some(actual)
                if actual.frozen
                    && actual.resource_id == expected.resource_id
                    && actual.start_tick == expected.start_tick
                    && actual.end_tick == expected.end_tick => {}
            Some(_) => push(
                &mut diagnostics,
                "FROZEN_COMMITMENT_CHANGED",
                Some(operation_id),
                "candidate did not preserve the exact source-owned commitment",
            ),
            None => push(
                &mut diagnostics,
                "FROZEN_COMMITMENT_DROPPED",
                Some(operation_id),
                "candidate omitted a source-owned frozen commitment",
            ),
        }
    }
    for assignment in &input.assignments {
        if assignment.frozen && !expected_frozen.contains_key(&assignment.operation_id) {
            push(
                &mut diagnostics,
                "UNDECLARED_FROZEN_ASSIGNMENT",
                Some(&assignment.operation_id),
                "candidate marks an assignment frozen without a source-owned commitment",
            );
        }
    }

    // Every unfinished mandatory operation must have exactly one candidate assignment.
    let mut missing_operation_ids = Vec::new();
    for operation_id in operation_map.keys() {
        let completed = completion_map.contains_key(operation_id);
        let assigned = assignment_by_operation.contains_key(operation_id);
        if !completed && !assigned {
            missing_operation_ids.push(operation_id.clone());
            push(
                &mut diagnostics,
                "MANDATORY_OPERATION_OMITTED",
                Some(operation_id),
                "unfinished mandatory operation is absent from candidate schedule",
            );
        }
    }
    missing_operation_ids.sort();

    // Validate evaluator truth even if no candidate happened to use the resource.
    let mut outage_ids = BTreeSet::new();
    for outage in &input.evaluator_outages {
        if outage.event_id.trim().is_empty() {
            push(
                &mut diagnostics,
                "EMPTY_EVALUATOR_OUTAGE_ID",
                None,
                "evaluator outage IDs must be non-empty",
            );
        } else if !outage_ids.insert(outage.event_id.as_str()) {
            push(
                &mut diagnostics,
                "DUPLICATE_EVALUATOR_OUTAGE_ID",
                None,
                format!("evaluator outage ID {} is duplicated", outage.event_id),
            );
        }
        if !resource_map.contains_key(&outage.resource_id) {
            push(
                &mut diagnostics,
                "UNKNOWN_EVALUATOR_OUTAGE_RESOURCE",
                None,
                format!(
                    "evaluator outage {} references unknown resource {}",
                    outage.event_id, outage.resource_id
                ),
            );
        }
        if outage.effective_until_tick <= outage.effective_from_tick {
            push(
                &mut diagnostics,
                "INVALID_EVALUATOR_OUTAGE_INTERVAL",
                None,
                format!("outage {} has a non-positive interval", outage.event_id),
            );
        }
    }

    // Unary resources may never be occupied by overlapping intervals. Historical
    // completions are included so a partially completed episode is checked too.
    let mut occupied: Vec<Assignment> = input.assignments.clone();
    occupied.extend(completion_intervals.values().cloned());
    let mut assigned: Vec<&Assignment> = occupied.iter().collect();
    assigned.sort_by(|a, b| {
        a.resource_id
            .cmp(&b.resource_id)
            .then_with(|| a.start_tick.cmp(&b.start_tick))
            .then_with(|| a.end_tick.cmp(&b.end_tick))
            .then_with(|| a.operation_id.cmp(&b.operation_id))
    });
    for (index, left) in assigned.iter().enumerate() {
        for right in assigned.iter().skip(index + 1) {
            if left.resource_id == right.resource_id
                && overlaps(
                    left.start_tick,
                    left.end_tick,
                    right.start_tick,
                    right.end_tick,
                )
            {
                push(
                    &mut diagnostics,
                    "UNARY_RESOURCE_OVERLAP",
                    Some(&right.operation_id),
                    format!(
                        "operations {} and {} overlap on resource {}",
                        left.operation_id, right.operation_id, left.resource_id
                    ),
                );
            }
        }
        for outage in input.evaluator_outages.iter().filter(|outage| {
            outage.resource_id == left.resource_id
                && outage.effective_until_tick > outage.effective_from_tick
        }) {
            if overlaps(
                left.start_tick,
                left.end_tick,
                outage.effective_from_tick,
                outage.effective_until_tick,
            ) {
                push(
                    &mut diagnostics,
                    "ASSIGNMENT_OVERLAPS_EFFECTIVE_OUTAGE",
                    Some(&left.operation_id),
                    format!("assignment overlaps evaluator outage {}", outage.event_id),
                );
            }
        }
    }

    // Precedence is checked from the independent candidate and completion records.
    for operation in operation_map.values() {
        let successor_start = assignment_by_operation
            .get(&operation.id)
            .map(|assignment| assignment.start_tick)
            .or_else(|| {
                completion_intervals
                    .get(&operation.id)
                    .map(|item| item.start_tick)
            });
        if let Some(successor_start) = successor_start {
            for predecessor_id in &operation.predecessor_ids {
                let predecessor_end = assignment_by_operation
                    .get(predecessor_id)
                    .map(|assignment| assignment.end_tick)
                    .or_else(|| completion_map.get(predecessor_id).copied());
                match predecessor_end {
                    Some(end_tick) if end_tick <= successor_start => {}
                    Some(end_tick) => push(
                        &mut diagnostics,
                        "PRECEDENCE_VIOLATION",
                        Some(&operation.id),
                        format!(
                            "predecessor {predecessor_id} ends at {end_tick}, after successor starts/completes at {successor_start}"
                        ),
                    ),
                    None => push(
                        &mut diagnostics,
                        "PREDECESSOR_MISSING_FROM_REALIZED_STATE",
                        Some(&operation.id),
                        format!(
                            "predecessor {predecessor_id} has neither completion nor assignment"
                        ),
                    ),
                }
            }
        }
    }

    let mut makespan_tick = None;
    let mut total_tardiness_ticks = 0u64;
    let mut occupied_resource_ticks = 0u64;
    for assignment in &occupied {
        makespan_tick = Some(makespan_tick.unwrap_or(0).max(assignment.end_tick));
        metric_add(
            &mut occupied_resource_ticks,
            assignment.end_tick.saturating_sub(assignment.start_tick),
            &mut diagnostics,
            "OCCUPIED_TICK_METRIC_OVERFLOW",
        );
        if let Some(operation) = operation_map.get(&assignment.operation_id) {
            let tardiness = assignment.end_tick.saturating_sub(operation.due_tick);
            metric_add(
                &mut total_tardiness_ticks,
                tardiness,
                &mut diagnostics,
                "TARDINESS_METRIC_OVERFLOW",
            );
        }
    }
    for completion in &input.completed {
        makespan_tick = Some(makespan_tick.unwrap_or(0).max(completion.completed_at_tick));
    }

    sort_diagnostics(&mut diagnostics);
    let status = if diagnostics.is_empty() {
        AuditStatus::AbstractFeasible
    } else {
        AuditStatus::Rejected
    };
    AuditReport {
        status,
        decision_tick: input.decision_tick,
        horizon_end_tick: input.horizon_end_tick,
        diagnostics,
        metrics: AuditMetrics {
            makespan_tick,
            total_tardiness_ticks,
            occupied_resource_ticks,
            completed_operation_count: completion_map.len(),
            assigned_operation_count: assignment_by_operation.len(),
            mandatory_operation_count: operation_map.len(),
            missing_operation_ids,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(id: &str, due: Tick, duration: Tick, kind: &str, predecessors: &[&str]) -> Operation {
        Operation {
            id: id.to_owned(),
            release_tick: 0,
            due_tick: due,
            duration_ticks: duration,
            resource_kind: kind.to_owned(),
            predecessor_ids: predecessors
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        }
    }

    fn res(id: &str, kind: &str) -> Resource {
        Resource {
            id: id.to_owned(),
            kind: kind.to_owned(),
        }
    }

    fn assignment(id: &str, resource: &str, start: Tick, end: Tick) -> Assignment {
        Assignment {
            operation_id: id.to_owned(),
            resource_id: resource.to_owned(),
            start_tick: start,
            end_tick: end,
            frozen: false,
        }
    }

    fn base_input() -> AuditInput {
        AuditInput {
            decision_tick: 0,
            horizon_end_tick: 20,
            operations: vec![
                op("weld", 5, 2, "welder", &[]),
                op("paint", 10, 3, "paint", &["weld"]),
            ],
            resources: vec![res("w-1", "welder"), res("p-1", "paint")],
            completed: vec![],
            frozen_commitments: vec![],
            assignments: vec![
                assignment("weld", "w-1", 0, 2),
                assignment("paint", "p-1", 2, 5),
            ],
            evaluator_outages: vec![],
        }
    }

    fn has_code(report: &AuditReport, code: &str) -> bool {
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code)
    }

    #[test]
    fn accepts_complete_abstract_schedule_and_recomputes_metrics() {
        let report = audit_schedule(&base_input());
        assert_eq!(report.status, AuditStatus::AbstractFeasible);
        assert_eq!(report.metrics.makespan_tick, Some(5));
        assert_eq!(report.metrics.total_tardiness_ticks, 0);
        assert_eq!(report.metrics.occupied_resource_ticks, 5);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn rejects_mandatory_work_omission() {
        let mut input = base_input();
        input.assignments.remove(0);
        let report = audit_schedule(&input);
        assert_eq!(report.status, AuditStatus::Rejected);
        assert!(has_code(&report, "MANDATORY_OPERATION_OMITTED"));
    }

    #[test]
    fn rejects_resource_overlap_and_wrong_resource_kind() {
        let mut input = base_input();
        input.assignments[1].resource_id = "w-1".to_owned();
        input.assignments[1].start_tick = 1;
        input.assignments[1].end_tick = 4;
        let report = audit_schedule(&input);
        assert!(has_code(&report, "RESOURCE_KIND_MISMATCH"));
        assert!(has_code(&report, "UNARY_RESOURCE_OVERLAP"));
    }

    #[test]
    fn rejects_precedence_violation() {
        let mut input = base_input();
        input.assignments[1].start_tick = 0;
        input.assignments[1].end_tick = 3;
        let report = audit_schedule(&input);
        assert!(has_code(&report, "PRECEDENCE_VIOLATION"));
    }

    #[test]
    fn rejects_effective_outage_even_if_policy_might_not_have_seen_it() {
        let mut input = base_input();
        input.evaluator_outages.push(EvaluatorOutage {
            event_id: "outage-1".to_owned(),
            resource_id: "w-1".to_owned(),
            effective_from_tick: 1,
            effective_until_tick: 4,
        });
        let report = audit_schedule(&input);
        assert!(has_code(&report, "ASSIGNMENT_OVERLAPS_EFFECTIVE_OUTAGE"));
    }

    #[test]
    fn source_frozen_commitments_must_be_retained_exactly() {
        let mut input = base_input();
        input.frozen_commitments.push(FrozenCommitment {
            operation_id: "weld".to_owned(),
            resource_id: "w-1".to_owned(),
            start_tick: 0,
            end_tick: 2,
        });
        input.assignments[0].frozen = true;
        let accepted = audit_schedule(&input);
        assert_eq!(accepted.status, AuditStatus::AbstractFeasible);

        input.assignments[0].start_tick = 1;
        input.assignments[0].end_tick = 3;
        let rejected = audit_schedule(&input);
        assert!(has_code(&rejected, "FROZEN_COMMITMENT_CHANGED"));
    }

    #[test]
    fn source_commitment_cannot_be_dropped_or_forged() {
        let mut input = base_input();
        input.frozen_commitments.push(FrozenCommitment {
            operation_id: "weld".to_owned(),
            resource_id: "w-1".to_owned(),
            start_tick: 0,
            end_tick: 2,
        });
        input.assignments[0].frozen = true;
        input.assignments.remove(0);
        let report = audit_schedule(&input);
        assert!(has_code(&report, "FROZEN_COMMITMENT_DROPPED"));

        let mut forged = base_input();
        forged.assignments[0].frozen = true;
        assert!(has_code(
            &audit_schedule(&forged),
            "UNDECLARED_FROZEN_ASSIGNMENT"
        ));
    }

    #[test]
    fn fixed_input_has_deterministic_report() {
        let input = base_input();
        assert_eq!(audit_schedule(&input), audit_schedule(&input));
    }

    #[test]
    fn completed_history_is_checked_for_real_start_time_and_occupancy() {
        let mut input = base_input();
        input.decision_tick = 2;
        input.assignments.remove(0);
        input.completed.push(Completion {
            operation_id: "weld".to_owned(),
            resource_id: "w-1".to_owned(),
            start_tick: 0,
            completed_at_tick: 2,
        });
        let accepted = audit_schedule(&input);
        assert_eq!(accepted.status, AuditStatus::AbstractFeasible);
        assert_eq!(accepted.metrics.completed_operation_count, 1);
        assert_eq!(accepted.metrics.assigned_operation_count, 1);
        assert_eq!(accepted.metrics.occupied_resource_ticks, 5);
    }

    #[test]
    fn malformed_evaluator_outage_is_rejected_even_without_candidate_use() {
        let mut input = base_input();
        input.resources.push(res("unused-crane", "crane"));
        input.evaluator_outages.push(EvaluatorOutage {
            event_id: "bad-outage".to_owned(),
            resource_id: "unused-crane".to_owned(),
            effective_from_tick: 8,
            effective_until_tick: 3,
        });
        assert!(has_code(
            &audit_schedule(&input),
            "INVALID_EVALUATOR_OUTAGE_INTERVAL"
        ));
    }
}
