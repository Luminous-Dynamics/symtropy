//! Replay timestamped execution evidence against a frozen production plan.
//!
//! This measures execution adherence; it does not authorize a physical operation or decide whether
//! a schedule is safe. All ticks must already be normalized to a common monotonic time domain.
//!
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedOperation {
    pub operation_id: String,
    pub planned_start_tick: u64,
    pub planned_end_tick: u64,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExecutionEventKind {
    Started,
    Completed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionEvent {
    pub event_id: String,
    pub operation_id: String,
    pub source_id: String,
    /// Reference to the immutable source event or evidence object; not a cryptographic attestation.
    pub evidence_ref: String,
    pub kind: ExecutionEventKind,
    /// When the activity is asserted to have happened, on the normalized plan timebase.
    pub occurred_tick: u64,
    /// When this evaluator could first have known the event, on the same timebase.
    pub recorded_tick: u64,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExecutionViolationCode {
    EmptyPlanSet,
    EmptyOperationId,
    DuplicateOperationId,
    InvalidPlannedWindow,
    EmptyEventId,
    DuplicateEventId,
    EmptyEventOperationId,
    UnknownOperation,
    EmptySourceId,
    EmptyEvidenceReference,
    RecordBeforeOccurrence,
    DuplicateStartEvent,
    DuplicateCompletionEvent,
    CompletionBeforeStart,
}

impl ExecutionViolationCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::EmptyPlanSet => "EmptyPlanSet",
            Self::EmptyOperationId => "EmptyOperationId",
            Self::DuplicateOperationId => "DuplicateOperationId",
            Self::InvalidPlannedWindow => "InvalidPlannedWindow",
            Self::EmptyEventId => "EmptyEventId",
            Self::DuplicateEventId => "DuplicateEventId",
            Self::EmptyEventOperationId => "EmptyEventOperationId",
            Self::UnknownOperation => "UnknownOperation",
            Self::EmptySourceId => "EmptySourceId",
            Self::EmptyEvidenceReference => "EmptyEvidenceReference",
            Self::RecordBeforeOccurrence => "RecordBeforeOccurrence",
            Self::DuplicateStartEvent => "DuplicateStartEvent",
            Self::DuplicateCompletionEvent => "DuplicateCompletionEvent",
            Self::CompletionBeforeStart => "CompletionBeforeStart",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionViolation {
    pub code: ExecutionViolationCode,
    pub subject_ids: Vec<String>,
    pub detail: String,
}

fn push_violation(
    violations: &mut Vec<ExecutionViolation>,
    code: ExecutionViolationCode,
    mut subject_ids: Vec<String>,
    detail: impl Into<String>,
) {
    subject_ids.sort();
    violations.push(ExecutionViolation {
        code,
        subject_ids,
        detail: detail.into(),
    });
}

fn sort_dedup_violations(violations: &mut Vec<ExecutionViolation>) {
    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.subject_ids.cmp(&right.subject_ids))
            .then_with(|| left.detail.cmp(&right.detail))
    });
    violations.dedup();
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationExecutionAssessment {
    pub operation_id: String,
    pub planned_start_tick: u64,
    pub planned_end_tick: u64,
    pub observed_start_tick: Option<u64>,
    pub observed_completion_tick: Option<u64>,
    /// Signed observed-minus-planned displacement. Positive means late; negative means early.
    pub start_delta_ticks: Option<i128>,
    pub completion_delta_ticks: Option<i128>,
    pub planned_duration_ticks: u64,
    pub observed_duration_ticks: Option<u64>,
    pub start_lateness_ticks: u64,
    pub completion_lateness_ticks: u64,
    pub duration_overrun_ticks: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExecutionReplayMetrics {
    pub planned_operations: usize,
    pub operations_started: usize,
    pub operations_completed: usize,
    pub operations_fully_observed: usize,
    pub missing_start_events: usize,
    pub missing_completion_events: usize,
    pub late_start_operations: usize,
    pub late_completion_operations: usize,
    pub total_start_lateness_ticks: u128,
    pub total_completion_lateness_ticks: u128,
    pub total_duration_overrun_ticks: u128,
    pub total_absolute_start_deviation_ticks: u128,
    pub total_absolute_completion_deviation_ticks: u128,
    pub total_event_recording_latency_ticks: u128,
    pub events_considered: usize,
    pub events_held_back_by_recorded_time: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionReplayReport {
    /// False only when plan/evidence consistency violations were found. Lateness is a metric, not
    /// a malformed-evidence violation, and should not be hidden by turning it into a boolean gate.
    pub evidence_valid: bool,
    pub as_of_recorded_tick: u64,
    pub metrics: ExecutionReplayMetrics,
    pub operations: Vec<OperationExecutionAssessment>,
    pub violations: Vec<ExecutionViolation>,
}

/// Replay only events known by the supplied recorded-time cutoff.
///
/// Late-arriving events are withheld from earlier as-of snapshots even when their event-time tick
/// is earlier than the cutoff. Re-running with a later recorded-time cutoff can revise the measured
/// history, but does not mutate the prior report. Duplicate starts/completions are rejected rather
/// than silently selecting one source as truth.
pub fn replay_execution(
    plan: &[PlannedOperation],
    events: &[ExecutionEvent],
    as_of_recorded_tick: u64,
) -> ExecutionReplayReport {
    let mut violations = Vec::new();
    if plan.is_empty() {
        push_violation(
            &mut violations,
            ExecutionViolationCode::EmptyPlanSet,
            Vec::new(),
            "execution replay requires an explicit non-empty planned-operation set",
        );
    }

    let mut plan_by_id: BTreeMap<&str, &PlannedOperation> = BTreeMap::new();
    for operation in plan {
        if operation.operation_id.trim().is_empty() {
            push_violation(
                &mut violations,
                ExecutionViolationCode::EmptyOperationId,
                vec![operation.operation_id.clone()],
                "planned operation identity is required",
            );
        } else if plan_by_id
            .insert(operation.operation_id.as_str(), operation)
            .is_some()
        {
            push_violation(
                &mut violations,
                ExecutionViolationCode::DuplicateOperationId,
                vec![operation.operation_id.clone()],
                "planned operation IDs must be unique",
            );
        }
        if operation.planned_start_tick >= operation.planned_end_tick {
            push_violation(
                &mut violations,
                ExecutionViolationCode::InvalidPlannedWindow,
                vec![operation.operation_id.clone()],
                "planned operation window must be non-empty and half-open",
            );
        }
    }

    let visible_events: Vec<&ExecutionEvent> = events
        .iter()
        .filter(|event| event.recorded_tick <= as_of_recorded_tick)
        .collect();
    let held_back_events = events.len().saturating_sub(visible_events.len());
    let mut event_id_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for event in &visible_events {
        if !event.event_id.trim().is_empty() {
            *event_id_counts.entry(event.event_id.as_str()).or_default() += 1;
        }
    }
    let duplicate_event_ids: BTreeSet<&str> = event_id_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(event_id, _)| *event_id)
        .collect();
    let mut seen_event_ids = BTreeSet::new();
    let mut start_by_operation: BTreeMap<String, u64> = BTreeMap::new();
    let mut completion_by_operation: BTreeMap<String, u64> = BTreeMap::new();
    let mut ambiguous_starts = BTreeSet::new();
    let mut ambiguous_completions = BTreeSet::new();
    let mut total_event_recording_latency_ticks = 0u128;

    for event in &visible_events {
        if event.event_id.trim().is_empty() {
            push_violation(
                &mut violations,
                ExecutionViolationCode::EmptyEventId,
                vec![event.operation_id.clone()],
                "execution event identity is required",
            );
        } else if !seen_event_ids.insert(event.event_id.as_str()) {
            push_violation(
                &mut violations,
                ExecutionViolationCode::DuplicateEventId,
                vec![event.event_id.clone()],
                "event IDs must be unique within one replay",
            );
        }
        if event.operation_id.trim().is_empty() {
            push_violation(
                &mut violations,
                ExecutionViolationCode::EmptyEventOperationId,
                vec![event.event_id.clone()],
                "event operation identity is required",
            );
        } else if !plan_by_id.contains_key(event.operation_id.as_str()) {
            push_violation(
                &mut violations,
                ExecutionViolationCode::UnknownOperation,
                vec![event.event_id.clone(), event.operation_id.clone()],
                "execution event references an operation absent from the frozen plan",
            );
        }
        if event.source_id.trim().is_empty() {
            push_violation(
                &mut violations,
                ExecutionViolationCode::EmptySourceId,
                vec![event.event_id.clone()],
                "source identity is required",
            );
        }
        if event.evidence_ref.trim().is_empty() {
            push_violation(
                &mut violations,
                ExecutionViolationCode::EmptyEvidenceReference,
                vec![event.event_id.clone()],
                "source evidence reference is required",
            );
        }
        if event.recorded_tick < event.occurred_tick {
            push_violation(
                &mut violations,
                ExecutionViolationCode::RecordBeforeOccurrence,
                vec![event.event_id.clone()],
                "recorded and occurrence ticks must share a normalized timebase",
            );
        } else {
            total_event_recording_latency_ticks = total_event_recording_latency_ticks
                .saturating_add(u128::from(event.recorded_tick - event.occurred_tick));
        }

        if duplicate_event_ids.contains(event.event_id.as_str())
            || event.event_id.trim().is_empty()
            || event.operation_id.trim().is_empty()
            || !plan_by_id.contains_key(event.operation_id.as_str())
            || event.source_id.trim().is_empty()
            || event.evidence_ref.trim().is_empty()
            || event.recorded_tick < event.occurred_tick
        {
            continue;
        }
        match event.kind {
            ExecutionEventKind::Started => {
                if ambiguous_starts.contains(&event.operation_id) {
                    continue;
                }
                if start_by_operation.contains_key(&event.operation_id) {
                    start_by_operation.remove(&event.operation_id);
                    ambiguous_starts.insert(event.operation_id.clone());
                    push_violation(
                        &mut violations,
                        ExecutionViolationCode::DuplicateStartEvent,
                        vec![event.operation_id.clone()],
                        "an operation cannot have two accepted start events in V0",
                    );
                } else {
                    start_by_operation
                        .insert(event.operation_id.clone(), event.occurred_tick);
                }
            }
            ExecutionEventKind::Completed => {
                if ambiguous_completions.contains(&event.operation_id) {
                    continue;
                }
                if completion_by_operation.contains_key(&event.operation_id) {
                    completion_by_operation.remove(&event.operation_id);
                    ambiguous_completions.insert(event.operation_id.clone());
                    push_violation(
                        &mut violations,
                        ExecutionViolationCode::DuplicateCompletionEvent,
                        vec![event.operation_id.clone()],
                        "an operation cannot have two accepted completion events in V0",
                    );
                } else {
                    completion_by_operation
                        .insert(event.operation_id.clone(), event.occurred_tick);
                }
            }
        }
    }

    let mut assessments = Vec::with_capacity(plan.len());
    let mut metrics = ExecutionReplayMetrics {
        planned_operations: plan.len(),
        events_considered: visible_events.len(),
        events_held_back_by_recorded_time: held_back_events,
        total_event_recording_latency_ticks,
        ..ExecutionReplayMetrics::default()
    };

    for operation in plan {
        let observed_start = start_by_operation.get(&operation.operation_id).copied();
        let observed_completion = completion_by_operation.get(&operation.operation_id).copied();
        if observed_start.is_some() {
            metrics.operations_started += 1;
        } else {
            metrics.missing_start_events += 1;
        }
        if observed_completion.is_some() {
            metrics.operations_completed += 1;
        } else {
            metrics.missing_completion_events += 1;
        }
        if observed_start.is_some() && observed_completion.is_some() {
            metrics.operations_fully_observed += 1;
        }
        if let (Some(start), Some(completion)) = (observed_start, observed_completion) {
            if completion < start {
                push_violation(
                    &mut violations,
                    ExecutionViolationCode::CompletionBeforeStart,
                    vec![operation.operation_id.clone()],
                    "observed completion precedes observed start",
                );
            }
        }

        let start_delta = observed_start
            .map(|tick| i128::from(tick) - i128::from(operation.planned_start_tick));
        let completion_delta = observed_completion
            .map(|tick| i128::from(tick) - i128::from(operation.planned_end_tick));
        let planned_duration = operation
            .planned_end_tick
            .saturating_sub(operation.planned_start_tick);
        let observed_duration = match (observed_start, observed_completion) {
            (Some(start), Some(completion)) if completion >= start => Some(completion - start),
            _ => None,
        };
        let start_lateness = start_delta.map_or(0, |delta| {
            if delta > 0 { delta as u128 } else { 0 }
        });
        let completion_lateness = completion_delta.map_or(0, |delta| {
            if delta > 0 { delta as u128 } else { 0 }
        });
        let duration_overrun = observed_duration.map_or(0, |duration| {
            u128::from(duration.saturating_sub(planned_duration))
        });

        if start_lateness > 0 {
            metrics.late_start_operations += 1;
            metrics.total_start_lateness_ticks =
                metrics.total_start_lateness_ticks.saturating_add(start_lateness);
        }
        if completion_lateness > 0 {
            metrics.late_completion_operations += 1;
            metrics.total_completion_lateness_ticks = metrics
                .total_completion_lateness_ticks
                .saturating_add(completion_lateness);
        }
        if let Some(delta) = start_delta {
            metrics.total_absolute_start_deviation_ticks = metrics
                .total_absolute_start_deviation_ticks
                .saturating_add(delta.unsigned_abs());
        }
        if let Some(delta) = completion_delta {
            metrics.total_absolute_completion_deviation_ticks = metrics
                .total_absolute_completion_deviation_ticks
                .saturating_add(delta.unsigned_abs());
        }
        metrics.total_duration_overrun_ticks = metrics
            .total_duration_overrun_ticks
            .saturating_add(duration_overrun);

        assessments.push(OperationExecutionAssessment {
            operation_id: operation.operation_id.clone(),
            planned_start_tick: operation.planned_start_tick,
            planned_end_tick: operation.planned_end_tick,
            observed_start_tick: observed_start,
            observed_completion_tick: observed_completion,
            start_delta_ticks: start_delta,
            completion_delta_ticks: completion_delta,
            planned_duration_ticks: planned_duration,
            observed_duration_ticks: observed_duration,
            start_lateness_ticks: u64::try_from(start_lateness).unwrap_or(u64::MAX),
            completion_lateness_ticks: u64::try_from(completion_lateness).unwrap_or(u64::MAX),
            duration_overrun_ticks: u64::try_from(duration_overrun).unwrap_or(u64::MAX),
        });
    }

    assessments.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
    sort_dedup_violations(&mut violations);
    ExecutionReplayReport {
        evidence_valid: violations.is_empty(),
        as_of_recorded_tick,
        metrics,
        operations: assessments,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(id: &str, start: u64, end: u64) -> PlannedOperation {
        PlannedOperation {
            operation_id: id.to_owned(),
            planned_start_tick: start,
            planned_end_tick: end,
        }
    }

    fn event(
        id: &str,
        operation: &str,
        kind: ExecutionEventKind,
        occurred: u64,
        recorded: u64,
    ) -> ExecutionEvent {
        ExecutionEvent {
            event_id: id.to_owned(),
            operation_id: operation.to_owned(),
            source_id: "mes-import-1".to_owned(),
            evidence_ref: format!("event://{id}"),
            kind,
            occurred_tick: occurred,
            recorded_tick: recorded,
        }
    }

    #[test]
    fn replay_separates_occurrence_time_from_recorded_time() {
        let plan = [op("weld-a", 3, 8)];
        let events = [
            event("start-a", "weld-a", ExecutionEventKind::Started, 5, 6),
            event("finish-a", "weld-a", ExecutionEventKind::Completed, 10, 12),
        ];

        let before_late_event = replay_execution(&plan, &events, 10);
        assert!(before_late_event.evidence_valid);
        assert_eq!(before_late_event.metrics.operations_started, 1);
        assert_eq!(before_late_event.metrics.operations_completed, 0);
        assert_eq!(before_late_event.metrics.missing_completion_events, 1);
        assert_eq!(before_late_event.metrics.events_held_back_by_recorded_time, 1);
        assert_eq!(before_late_event.operations[0].start_delta_ticks, Some(2));
        assert_eq!(before_late_event.operations[0].observed_completion_tick, None);

        let after_late_event = replay_execution(&plan, &events, 12);
        assert!(after_late_event.evidence_valid);
        assert_eq!(after_late_event.metrics.operations_completed, 1);
        assert_eq!(after_late_event.metrics.total_completion_lateness_ticks, 2);
        assert_eq!(after_late_event.metrics.total_duration_overrun_ticks, 0);
        assert_eq!(after_late_event.metrics.total_event_recording_latency_ticks, 3);
    }

    #[test]
    fn event_input_order_does_not_change_replay_output() {
        let plan = [op("weld-a", 0, 5), op("inspect-a", 6, 8)];
        let events = [
            event("finish-inspect", "inspect-a", ExecutionEventKind::Completed, 10, 10),
            event("start-weld", "weld-a", ExecutionEventKind::Started, 1, 2),
            event("finish-weld", "weld-a", ExecutionEventKind::Completed, 7, 8),
            event("start-inspect", "inspect-a", ExecutionEventKind::Started, 8, 9),
        ];
        let mut shuffled = events.clone();
        shuffled.reverse();

        assert_eq!(
            replay_execution(&plan, &events, 12),
            replay_execution(&plan, &shuffled, 12)
        );
    }

    #[test]
    fn duplicate_event_ids_fail_closed() {
        let plan = [op("weld-a", 0, 5)];
        let events = [
            event("same-id", "weld-a", ExecutionEventKind::Started, 1, 1),
            event("same-id", "weld-a", ExecutionEventKind::Started, 2, 2),
        ];
        let report = replay_execution(&plan, &events, 5);
        assert!(!report.evidence_valid);
        assert!(report.violations.iter().any(|v| {
            v.code == ExecutionViolationCode::DuplicateEventId
        }));
    }

    #[test]
    fn ambiguous_duplicate_event_claims_do_not_depend_on_input_order() {
        let plan = [op("weld-a", 0, 5)];
        let events = [
            event("start-a", "weld-a", ExecutionEventKind::Started, 1, 1),
            event("start-b", "weld-a", ExecutionEventKind::Started, 2, 2),
        ];
        let mut reversed = events.clone();
        reversed.reverse();
        let left = replay_execution(&plan, &events, 5);
        let right = replay_execution(&plan, &reversed, 5);

        assert_eq!(left, right);
        assert!(!left.evidence_valid);
        assert_eq!(left.operations[0].observed_start_tick, None);
        assert!(left.violations.iter().any(|v| {
            v.code == ExecutionViolationCode::DuplicateStartEvent
        }));
    }

    #[test]
    fn unknown_operations_and_recorded_time_before_occurrence_fail_closed() {
        let plan = [op("weld-a", 0, 5)];
        let events = [
            event("bad-operation", "missing-op", ExecutionEventKind::Started, 1, 2),
            event("bad-clock", "weld-a", ExecutionEventKind::Completed, 4, 3),
        ];
        let report = replay_execution(&plan, &events, 5);
        assert!(!report.evidence_valid);
        assert!(report.violations.iter().any(|v| {
            v.code == ExecutionViolationCode::UnknownOperation
        }));
        assert!(report.violations.iter().any(|v| {
            v.code == ExecutionViolationCode::RecordBeforeOccurrence
        }));
    }

    #[test]
    fn missing_events_are_reported_as_observability_gaps_not_zero_duration() {
        let plan = [op("cut-a", 0, 3), op("weld-a", 4, 8)];
        let events = [event("start-cut", "cut-a", ExecutionEventKind::Started, 1, 1)];
        let report = replay_execution(&plan, &events, 2);

        assert_eq!(report.metrics.planned_operations, 2);
        assert_eq!(report.metrics.operations_started, 1);
        assert_eq!(report.metrics.operations_completed, 0);
        assert_eq!(report.metrics.missing_start_events, 1);
        assert_eq!(report.metrics.missing_completion_events, 2);
        assert_eq!(report.operations[0].observed_duration_ticks, None);
        assert!(report.evidence_valid);
    }

    #[test]
    fn completion_before_start_invalidates_the_trace() {
        let plan = [op("weld-a", 0, 5)];
        let events = [
            event("start", "weld-a", ExecutionEventKind::Started, 4, 4),
            event("complete", "weld-a", ExecutionEventKind::Completed, 3, 5),
        ];
        let report = replay_execution(&plan, &events, 5);
        assert!(!report.evidence_valid);
        assert!(report.violations.iter().any(|v| {
            v.code == ExecutionViolationCode::CompletionBeforeStart
        }));
    }
}
