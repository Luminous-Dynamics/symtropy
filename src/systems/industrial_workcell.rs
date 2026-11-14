// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Deterministic feasibility checks for fabrication-cell and shipyard schedules.
//!
//! This is an additive, pure-Rust planning primitive: it validates a proposed
//! schedule for well-formed intervals, predecessor constraints, and exclusive
//! resource conflicts. It does not optimize a schedule, simulate weld distortion,
//! model variable-capacity resources, or establish physical safety. Those must
//! be validated separately before production use.

use std::collections::{HashMap, HashSet};

/// One scheduled operation that occupies one exclusive resource for an interval.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledOperation {
    /// Globally unique operation identifier within this schedule.
    pub id: String,
    /// Job, part, hull block, or assembly identifier.
    pub job_id: String,
    /// Exclusive resource identifier (machine, welding bay, crane, dry dock, etc.).
    pub resource_id: String,
    /// Planned start time in seconds from a common schedule origin.
    pub start_seconds: f64,
    /// Positive operation duration in seconds.
    pub duration_seconds: f64,
    /// Operation IDs that must finish before this operation starts.
    pub predecessors: Vec<String>,
}

impl ScheduledOperation {
    pub fn end_seconds(&self) -> Option<f64> {
        if !self.start_seconds.is_finite()
            || self.start_seconds < 0.0
            || !self.duration_seconds.is_finite()
            || self.duration_seconds <= 0.0
        {
            return None;
        }
        let end = self.start_seconds + self.duration_seconds;
        (end.is_finite() && end > self.start_seconds).then_some(end)
    }
}

/// Machine-readable reason a proposed schedule is not feasible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScheduleViolationCode {
    EmptyOperationId,
    DuplicateOperationId,
    EmptyJobId,
    EmptyResourceId,
    InvalidStartTime,
    InvalidDuration,
    NonFiniteEndTime,
    NonIncreasingEndTime,
    DuplicatePredecessor,
    SelfPredecessor,
    MissingPredecessor,
    PrecedenceViolation,
    ExclusiveResourceOverlap,
    EmptyResourceCapacityId,
    DuplicateResourceCapacity,
    ZeroResourceCapacity,
    UnknownResourceCapacity,
    ResourceCapacityExceeded,
    EmptyDurationBoundOperationId,
    MissingDurationBounds,
    DuplicateDurationBounds,
    InvalidDurationBounds,
    UnknownDurationBoundOperation,
}

/// A concrete violation tied to the affected operation IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleViolation {
    pub code: ScheduleViolationCode,
    pub operation_ids: Vec<String>,
    pub detail: String,
}

/// Result of validating a proposed schedule.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScheduleValidationReport {
    pub violations: Vec<ScheduleViolation>,
}

impl ScheduleValidationReport {
    pub fn is_feasible(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn violation_count(&self, code: ScheduleViolationCode) -> usize {
        self.violations
            .iter()
            .filter(|violation| violation.code == code)
            .count()
    }
}

/// Validate a proposed schedule.
///
/// Intervals are half-open: [start, end). Thus one operation may begin exactly
/// when another operation on the same resource finishes. The function returns
/// all discoverable violations in deterministic sorted order. Unknown predecessor
/// IDs and duplicate operation IDs are reported, never silently ignored.
///
/// Each resource ID denotes one exclusive resource. If real equipment or crews
/// have capacity greater than one, model them as distinct resource IDs or extend
/// this contract with an explicit, separately tested capacity constraint.
pub fn validate_schedule(operations: &[ScheduledOperation]) -> ScheduleValidationReport {
    validate_schedule_internal(operations, true)
}

fn validate_schedule_internal(
    operations: &[ScheduledOperation],
    check_exclusive_overlaps: bool,
) -> ScheduleValidationReport {
    let mut violations = Vec::new();
    let mut by_id: HashMap<&str, &ScheduledOperation> = HashMap::with_capacity(operations.len());

    for operation in operations {
        if operation.id.trim().is_empty() {
            push_violation(
                &mut violations,
                ScheduleViolationCode::EmptyOperationId,
                vec![operation.id.clone()],
                "operation ID cannot be empty",
            );
        } else if by_id.insert(operation.id.as_str(), operation).is_some() {
            push_violation(
                &mut violations,
                ScheduleViolationCode::DuplicateOperationId,
                vec![operation.id.clone()],
                "operation ID must be unique within the schedule",
            );
        }

        if operation.job_id.trim().is_empty() {
            push_violation(
                &mut violations,
                ScheduleViolationCode::EmptyJobId,
                vec![operation.id.clone()],
                "job ID cannot be empty",
            );
        }
        if operation.resource_id.trim().is_empty() {
            push_violation(
                &mut violations,
                ScheduleViolationCode::EmptyResourceId,
                vec![operation.id.clone()],
                "exclusive resource ID cannot be empty",
            );
        }
        if !operation.start_seconds.is_finite() || operation.start_seconds < 0.0 {
            push_violation(
                &mut violations,
                ScheduleViolationCode::InvalidStartTime,
                vec![operation.id.clone()],
                "start time must be finite and non-negative",
            );
        }
        if !operation.duration_seconds.is_finite() || operation.duration_seconds <= 0.0 {
            push_violation(
                &mut violations,
                ScheduleViolationCode::InvalidDuration,
                vec![operation.id.clone()],
                "duration must be finite and greater than zero",
            );
        } else if operation.start_seconds.is_finite()
            && operation.start_seconds >= 0.0
            && !(operation.start_seconds + operation.duration_seconds).is_finite()
        {
            push_violation(
                &mut violations,
                ScheduleViolationCode::NonFiniteEndTime,
                vec![operation.id.clone()],
                "start plus duration must produce a finite end time",
            );
        } else if operation.start_seconds.is_finite()
            && operation.start_seconds >= 0.0
            && (operation.start_seconds + operation.duration_seconds).is_finite()
            && operation.start_seconds + operation.duration_seconds <= operation.start_seconds
        {
            push_violation(
                &mut violations,
                ScheduleViolationCode::NonIncreasingEndTime,
                vec![operation.id.clone()],
                "start plus duration must advance representable schedule time",
            );
        }

        let mut seen_predecessors: HashSet<&str> =
            HashSet::with_capacity(operation.predecessors.len());
        for predecessor_id in &operation.predecessors {
            if !seen_predecessors.insert(predecessor_id.as_str()) {
                push_violation(
                    &mut violations,
                    ScheduleViolationCode::DuplicatePredecessor,
                    vec![operation.id.clone(), predecessor_id.clone()],
                    "predecessor ID is repeated",
                );
            }
            if predecessor_id == &operation.id {
                push_violation(
                    &mut violations,
                    ScheduleViolationCode::SelfPredecessor,
                    vec![operation.id.clone()],
                    "operation cannot depend on itself",
                );
            }
        }
    }

    for operation in operations {
        let Some(start) = valid_start(operation) else {
            continue;
        };
        for predecessor_id in &operation.predecessors {
            let Some(predecessor) = by_id.get(predecessor_id.as_str()).copied() else {
                push_violation(
                    &mut violations,
                    ScheduleViolationCode::MissingPredecessor,
                    vec![operation.id.clone(), predecessor_id.clone()],
                    "predecessor operation does not exist in this schedule",
                );
                continue;
            };
            if let Some(predecessor_end) = predecessor.end_seconds()
                && predecessor_end > start
            {
                push_violation(
                    &mut violations,
                    ScheduleViolationCode::PrecedenceViolation,
                    vec![predecessor.id.clone(), operation.id.clone()],
                    "predecessor must finish no later than the dependent operation starts",
                );
            }
        }
    }

    if check_exclusive_overlaps {
        append_exclusive_resource_overlaps(operations, &mut violations);
    }

    for violation in &mut violations {
        violation.operation_ids.sort();
        violation.operation_ids.dedup();
    }
    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.operation_ids.cmp(&right.operation_ids))
            .then_with(|| left.detail.cmp(&right.detail))
    });
    violations.dedup();

    ScheduleValidationReport { violations }
}

/// Declared concurrent capacity for an interchangeable resource pool.
///
/// Unlike a one-resource-per-machine schedule, this allows a shared pool such as
/// a set of equivalent cranes, inspectors, or welding stations to serve multiple
/// simultaneous operations up to an explicit limit. It does not claim that the
/// units are interchangeable for every job; capability matching is a separate
/// qualification obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceCapacity {
    pub resource_id: String,
    pub capacity: usize,
}

/// Conservative duration envelope for one scheduled operation.
///
/// The nominal `ScheduledOperation::duration_seconds` must lie within this
/// finite, positive interval. Robust validation uses `maximum_seconds` for
/// precedence and resource-occupancy checks. These are deterministic bounds,
/// not a probabilistic confidence interval unless a separately validated model
/// establishes that interpretation.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationDurationBounds {
    pub operation_id: String,
    pub minimum_seconds: f64,
    pub maximum_seconds: f64,
}

/// Validate a schedule against explicit resource-pool capacities.
///
/// Every non-empty resource identifier used by an operation must have exactly one
/// capacity declaration. Missing or duplicated declarations fail closed. Time
/// intervals are half-open ([start, end)); end events are processed before start
/// events at the same instant so back-to-back use is permitted.
///
/// The ordinary single-capacity conflict findings are replaced by pool-capacity
/// findings for this validation path; other structural and precedence checks from
/// `validate_schedule` are retained.
pub fn validate_schedule_with_resource_capacities(
    operations: &[ScheduledOperation],
    resource_capacities: &[ResourceCapacity],
) -> ScheduleValidationReport {
    let mut report = validate_schedule_internal(operations, false);

    let mut declarations: HashMap<&str, Vec<usize>> = HashMap::new();
    for declaration in resource_capacities {
        if declaration.resource_id.trim().is_empty() {
            push_violation(
                &mut report.violations,
                ScheduleViolationCode::EmptyResourceCapacityId,
                vec![declaration.resource_id.clone()],
                "resource capacity ID cannot be empty",
            );
            continue;
        }
        if declaration.capacity == 0 {
            push_violation(
                &mut report.violations,
                ScheduleViolationCode::ZeroResourceCapacity,
                vec![declaration.resource_id.clone()],
                "resource capacity must be greater than zero",
            );
        }
        declarations
            .entry(declaration.resource_id.as_str())
            .or_default()
            .push(declaration.capacity);
    }

    let mut valid_capacities: HashMap<&str, usize> = HashMap::new();
    let mut ambiguous = std::collections::HashSet::new();
    for (resource_id, capacities) in &declarations {
        if capacities.len() > 1 {
            ambiguous.insert(*resource_id);
            push_violation(
                &mut report.violations,
                ScheduleViolationCode::DuplicateResourceCapacity,
                vec![(*resource_id).to_string()],
                "resource ID must have exactly one capacity declaration",
            );
        } else if let Some(capacity) = capacities.first().copied()
            && capacity > 0
        {
            valid_capacities.insert(resource_id, capacity);
        }
    }

    let mut by_resource: HashMap<&str, Vec<(usize, &ScheduledOperation)>> = HashMap::new();
    for (index, operation) in operations.iter().enumerate() {
        if operation.resource_id.trim().is_empty() {
            continue;
        }
        if !declarations.contains_key(operation.resource_id.as_str()) {
            push_violation(
                &mut report.violations,
                ScheduleViolationCode::UnknownResourceCapacity,
                vec![operation.id.clone()],
                &format!(
                    "resource '{}' has no explicit capacity declaration",
                    operation.resource_id
                ),
            );
        } else if !ambiguous.contains(operation.resource_id.as_str())
            && valid_capacities.contains_key(operation.resource_id.as_str())
        {
            by_resource
                .entry(operation.resource_id.as_str())
                .or_default()
                .push((index, operation));
        }
    }

    #[derive(Clone, Copy)]
    struct CapacityEvent<'a> {
        time: f64,
        starts: bool,
        index: usize,
        operation: &'a ScheduledOperation,
    }

    for (resource_id, resource_operations) in by_resource {
        let Some(capacity) = valid_capacities.get(resource_id).copied() else {
            continue;
        };
        let mut events = Vec::with_capacity(resource_operations.len() * 2);
        for (index, operation) in resource_operations {
            let (Some(start), Some(end)) = (valid_start(operation), operation.end_seconds()) else {
                continue;
            };
            events.push(CapacityEvent {
                time: start,
                starts: true,
                index,
                operation,
            });
            events.push(CapacityEvent {
                time: end,
                starts: false,
                index,
                operation,
            });
        }
        events.sort_by(|left, right| {
            left.time
                .total_cmp(&right.time)
                // false (end) precedes true (start): half-open interval semantics.
                .then_with(|| left.starts.cmp(&right.starts))
                .then_with(|| left.operation.id.cmp(&right.operation.id))
                .then_with(|| left.index.cmp(&right.index))
        });

        // Track only the active operation set. End events remove in expected O(1)
        // time; violations name the deterministic operation that pushes occupancy
        // over the limit, avoiding repeated copies of every active ID on hot pools.
        let mut active: HashMap<usize, &ScheduledOperation> = HashMap::new();
        for event in events {
            if !event.starts {
                active.remove(&event.index);
                continue;
            }
            if active.len() >= capacity {
                push_violation(
                    &mut report.violations,
                    ScheduleViolationCode::ResourceCapacityExceeded,
                    vec![event.operation.id.clone()],
                    &format!(
                        "operation '{}' starts at {} while resource '{}' already has {} active operations (capacity {})",
                        event.operation.id,
                        event.time,
                        resource_id,
                        active.len(),
                        capacity
                    ),
                );
            }
            active.insert(event.index, event.operation);
        }
    }

    normalize_violations(&mut report.violations);
    report
}

/// Append all pairwise conflicts using a per-resource sweep line.
///
/// For schedules with sparse overlap, this avoids comparing operations that use
/// different resources and avoids a full all-pairs scan. Runtime is
/// O(n log n + k) for n valid intervals and k reported overlapping pairs; k can
/// itself be quadratic when the input truly contains quadratic many conflicts.
/// Sorting plus final report normalization keeps the externally visible report
/// deterministic despite the hash-map grouping.
fn append_exclusive_resource_overlaps(
    operations: &[ScheduledOperation],
    violations: &mut Vec<ScheduleViolation>,
) {
    let mut by_resource: HashMap<&str, Vec<(&ScheduledOperation, f64, f64)>> = HashMap::new();

    for operation in operations {
        let (Some(start), Some(end)) = (valid_start(operation), operation.end_seconds()) else {
            continue;
        };
        by_resource
            .entry(operation.resource_id.as_str())
            .or_default()
            .push((operation, start, end));
    }

    for (_resource_id, mut intervals) in by_resource {
        intervals.sort_by(|left, right| {
            left.1
                .total_cmp(&right.1)
                .then_with(|| left.2.total_cmp(&right.2))
                .then_with(|| left.0.id.cmp(&right.0.id))
        });

        // Only active intervals can overlap the current interval. Remove intervals
        // ending at the current start because the model uses half-open [start, end).
        let mut active: Vec<(&ScheduledOperation, f64)> = Vec::new();
        for (operation, start, end) in intervals {
            active.retain(|(_, active_end)| *active_end > start);
            for (other, _) in &active {
                let mut ids = vec![operation.id.clone(), other.id.clone()];
                ids.sort();
                push_violation(
                    violations,
                    ScheduleViolationCode::ExclusiveResourceOverlap,
                    ids,
                    "two operations overlap on one exclusive resource",
                );
            }
            active.push((operation, end));
        }
    }
}

/// Validate a schedule using the maximum duration in a required envelope for
/// every operation. Missing, duplicate, malformed, or unknown bounds fail closed.
///
/// Upper-bound durations are used for both predecessor completion and resource
/// overlap checks. A robust-feasible result means the schedule satisfies the
/// modeled constraints for these declared deterministic duration bounds; it
/// does not establish that the bounds cover all real-world outcomes.
pub fn validate_schedule_with_duration_bounds(
    operations: &[ScheduledOperation],
    duration_bounds: &[OperationDurationBounds],
) -> ScheduleValidationReport {
    let (bounded_operations, mut bound_violations) =
        apply_duration_bounds(operations, duration_bounds);
    let mut report = validate_schedule(&bounded_operations);
    report.violations.append(&mut bound_violations);
    normalize_violations(&mut report.violations);
    report
}

/// Capacity-aware robust validation: apply duration upper bounds before checking
/// shared resource-pool occupancy and precedence constraints.
pub fn validate_schedule_with_resource_capacities_and_duration_bounds(
    operations: &[ScheduledOperation],
    resource_capacities: &[ResourceCapacity],
    duration_bounds: &[OperationDurationBounds],
) -> ScheduleValidationReport {
    let (bounded_operations, mut bound_violations) =
        apply_duration_bounds(operations, duration_bounds);
    let mut report =
        validate_schedule_with_resource_capacities(&bounded_operations, resource_capacities);
    report.violations.append(&mut bound_violations);
    normalize_violations(&mut report.violations);
    report
}

fn apply_duration_bounds(
    operations: &[ScheduledOperation],
    duration_bounds: &[OperationDurationBounds],
) -> (Vec<ScheduledOperation>, Vec<ScheduleViolation>) {
    let mut violations = Vec::new();
    let mut by_operation: HashMap<&str, Vec<&OperationDurationBounds>> = HashMap::new();
    let known_operation_ids: std::collections::HashSet<&str> = operations
        .iter()
        .map(|operation| operation.id.as_str())
        .collect();

    for bound in duration_bounds {
        if bound.operation_id.trim().is_empty() {
            push_violation(
                &mut violations,
                ScheduleViolationCode::EmptyDurationBoundOperationId,
                vec![bound.operation_id.clone()],
                "duration-bound operation ID cannot be empty",
            );
            continue;
        }
        if !bound.minimum_seconds.is_finite()
            || !bound.maximum_seconds.is_finite()
            || bound.minimum_seconds <= 0.0
            || bound.maximum_seconds < bound.minimum_seconds
        {
            push_violation(
                &mut violations,
                ScheduleViolationCode::InvalidDurationBounds,
                vec![bound.operation_id.clone()],
                "duration bounds must be finite, positive, and maximum must be at least minimum",
            );
        }
        if !known_operation_ids.contains(bound.operation_id.as_str()) {
            push_violation(
                &mut violations,
                ScheduleViolationCode::UnknownDurationBoundOperation,
                vec![bound.operation_id.clone()],
                "duration bounds reference an operation absent from the schedule",
            );
        }
        by_operation
            .entry(bound.operation_id.as_str())
            .or_default()
            .push(bound);
    }

    let mut bounded_operations = Vec::with_capacity(operations.len());
    for operation in operations {
        if operation.id.trim().is_empty() {
            bounded_operations.push(operation.clone());
            continue;
        }
        let Some(bounds) = by_operation.get(operation.id.as_str()) else {
            push_violation(
                &mut violations,
                ScheduleViolationCode::MissingDurationBounds,
                vec![operation.id.clone()],
                "operation requires exactly one explicit duration envelope",
            );
            bounded_operations.push(operation.clone());
            continue;
        };
        if bounds.len() != 1 {
            push_violation(
                &mut violations,
                ScheduleViolationCode::DuplicateDurationBounds,
                vec![operation.id.clone()],
                "operation must have exactly one duration envelope",
            );
            bounded_operations.push(operation.clone());
            continue;
        }

        let bound = bounds[0];
        let bounds_well_formed = bound.minimum_seconds.is_finite()
            && bound.maximum_seconds.is_finite()
            && bound.minimum_seconds > 0.0
            && bound.maximum_seconds >= bound.minimum_seconds;
        if !bounds_well_formed {
            // The malformed-bound violation was emitted during declaration intake.
            bounded_operations.push(operation.clone());
            continue;
        }
        if !operation.duration_seconds.is_finite()
            || operation.duration_seconds < bound.minimum_seconds
            || operation.duration_seconds > bound.maximum_seconds
        {
            push_violation(
                &mut violations,
                ScheduleViolationCode::InvalidDurationBounds,
                vec![operation.id.clone()],
                "duration envelope must contain the nominal operation duration",
            );
            bounded_operations.push(operation.clone());
            continue;
        }

        let mut robust_operation = operation.clone();
        robust_operation.duration_seconds = bound.maximum_seconds;
        bounded_operations.push(robust_operation);
    }

    normalize_violations(&mut violations);
    (bounded_operations, violations)
}

fn normalize_violations(violations: &mut Vec<ScheduleViolation>) {
    for violation in violations.iter_mut() {
        violation.operation_ids.sort();
        violation.operation_ids.dedup();
    }
    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.operation_ids.cmp(&right.operation_ids))
            .then_with(|| left.detail.cmp(&right.detail))
    });
    violations.dedup();
}

fn valid_start(operation: &ScheduledOperation) -> Option<f64> {
    (operation.start_seconds.is_finite() && operation.start_seconds >= 0.0)
        .then_some(operation.start_seconds)
}

fn push_violation(
    violations: &mut Vec<ScheduleViolation>,
    code: ScheduleViolationCode,
    operation_ids: Vec<String>,
    detail: &str,
) {
    violations.push(ScheduleViolation {
        code,
        operation_ids,
        detail: detail.to_string(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(
        id: &str,
        resource: &str,
        start: f64,
        duration: f64,
        predecessors: &[&str],
    ) -> ScheduledOperation {
        ScheduledOperation {
            id: id.into(),
            job_id: format!("job-{id}"),
            resource_id: resource.into(),
            start_seconds: start,
            duration_seconds: duration,
            predecessors: predecessors.iter().map(|value| (*value).into()).collect(),
        }
    }

    #[test]
    fn accepts_non_overlapping_resource_use_and_satisfied_precedence() {
        let operations = vec![
            operation("machine", "cnc-1", 10.0, 5.0, &["cut"]),
            operation("cut", "cnc-1", 0.0, 10.0, &[]),
            operation("inspect", "cmm-1", 15.0, 2.0, &["machine"]),
        ];
        let report = validate_schedule(&operations);
        assert!(report.is_feasible(), "{:?}", report.violations);
    }

    #[test]
    fn detects_exclusive_resource_overlap_but_allows_back_to_back() {
        let overlapping = vec![
            operation("weld-a", "bay-1", 0.0, 12.0, &[]),
            operation("weld-b", "bay-1", 11.0, 4.0, &[]),
        ];
        let report = validate_schedule(&overlapping);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::ExclusiveResourceOverlap),
            1
        );

        let back_to_back = vec![
            operation("weld-a", "bay-1", 0.0, 12.0, &[]),
            operation("weld-b", "bay-1", 12.0, 4.0, &[]),
        ];
        assert!(validate_schedule(&back_to_back).is_feasible());
    }

    #[test]
    fn detects_missing_and_unsatisfied_predecessors() {
        let operations = vec![
            operation("fit-up", "bay-1", 8.0, 3.0, &["missing"]),
            operation("weld", "bay-1", 4.0, 6.0, &["fit-up"]),
        ];
        let report = validate_schedule(&operations);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::MissingPredecessor),
            1
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::PrecedenceViolation),
            1
        );
    }

    #[test]
    fn rejects_non_finite_and_non_positive_time_values() {
        let operations = vec![
            operation("nan", "cnc-1", f64::NAN, 2.0, &[]),
            operation("negative-duration", "cnc-2", 0.0, -1.0, &[]),
            operation("overflow", "cnc-3", f64::MAX, f64::MAX, &[]),
        ];
        let report = validate_schedule(&operations);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::InvalidStartTime),
            1
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::InvalidDuration),
            1
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::NonFiniteEndTime),
            1
        );

        let lost_precision = vec![operation("lost-precision", "cnc-4", f64::MAX, 1.0, &[])];
        let report = validate_schedule(&lost_precision);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::NonIncreasingEndTime),
            1
        );
    }

    #[test]
    fn rejects_duplicate_ids_self_dependencies_and_empty_resources() {
        let operations = vec![
            operation("same", "bay-1", 0.0, 2.0, &["same"]),
            operation("same", "", 2.0, 2.0, &[]),
        ];
        let report = validate_schedule(&operations);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::DuplicateOperationId),
            1
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::SelfPredecessor),
            1
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::EmptyResourceId),
            1
        );
    }

    #[test]
    fn repeated_predecessors_are_reported() {
        let operations = vec![
            operation("prepare", "table-1", 0.0, 1.0, &[]),
            operation("assemble", "bay-1", 1.0, 4.0, &["prepare", "prepare"]),
        ];
        let report = validate_schedule(&operations);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::DuplicatePredecessor),
            1
        );
    }

    #[test]
    fn resource_pool_capacity_allows_parallel_work_until_limit() {
        let operations = vec![
            operation("inspect-a", "ndt-team", 0.0, 10.0, &[]),
            operation("inspect-b", "ndt-team", 0.0, 5.0, &[]),
            operation("inspect-c", "ndt-team", 4.0, 8.0, &[]),
        ];
        let capacities = vec![ResourceCapacity {
            resource_id: "ndt-team".into(),
            capacity: 2,
        }];
        let report = validate_schedule_with_resource_capacities(&operations, &capacities);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::ResourceCapacityExceeded),
            1
        );
        assert_eq!(
            report
                .violations
                .iter()
                .find(|v| v.code == ScheduleViolationCode::ResourceCapacityExceeded)
                .unwrap()
                .operation_ids,
            vec!["inspect-c"]
        );
        assert!(
            report
                .violations
                .iter()
                .find(|v| v.code == ScheduleViolationCode::ResourceCapacityExceeded)
                .unwrap()
                .detail
                .contains("already has 2 active operations")
        );
    }

    #[test]
    fn resource_pool_exact_capacity_is_feasible() {
        let operations = vec![
            operation("inspect-a", "ndt-team", 0.0, 10.0, &[]),
            operation("inspect-b", "ndt-team", 0.0, 5.0, &[]),
        ];
        let capacities = vec![ResourceCapacity {
            resource_id: "ndt-team".into(),
            capacity: 2,
        }];
        let report = validate_schedule_with_resource_capacities(&operations, &capacities);
        assert!(report.is_feasible(), "{:?}", report.violations);
    }

    #[test]
    fn capacity_report_is_invariant_to_input_order_for_unique_operation_ids() {
        let operations = vec![
            operation("cut-a", "cnc-pool", 0.0, 8.0, &[]),
            operation("cut-b", "cnc-pool", 1.0, 8.0, &[]),
            operation("cut-c", "cnc-pool", 2.0, 8.0, &[]),
            operation("cut-d", "cnc-pool", 10.0, 2.0, &[]),
        ];
        let mut reversed = operations.clone();
        reversed.reverse();
        let capacities = vec![ResourceCapacity {
            resource_id: "cnc-pool".into(),
            capacity: 2,
        }];

        let forward = validate_schedule_with_resource_capacities(&operations, &capacities);
        let backward = validate_schedule_with_resource_capacities(&reversed, &capacities);
        assert_eq!(forward, backward);
    }

    #[test]
    fn exhaustive_small_pools_match_independent_peak_concurrency_oracle() {
        // Synthetic exhaustive check only: operation intervals are tiny integer
        // fixtures; they are not measurements of real manufacturing durations.
        // Six choices per operation cover starts 0..=2 and durations 1..=2.
        for operation_count in 1usize..=4 {
            let cases = 6usize.pow(operation_count as u32);
            for mut encoded in 0..cases {
                let mut operations = Vec::with_capacity(operation_count);
                for index in 0..operation_count {
                    let choice = encoded % 6;
                    encoded /= 6;
                    let start = (choice % 3) as f64;
                    let duration = if choice / 3 == 0 { 1.0 } else { 2.0 };
                    operations.push(operation(
                        &format!("op-{index}"),
                        "pool",
                        start,
                        duration,
                        &[],
                    ));
                }

                let peak_concurrency = (0..=3)
                    .map(|time| {
                        let time = time as f64;
                        operations
                            .iter()
                            .filter(|item| {
                                item.start_seconds <= time
                                    && item.end_seconds().is_some_and(|end| time < end)
                            })
                            .count()
                    })
                    .max()
                    .unwrap_or(0);

                for capacity in 1usize..=3 {
                    let declarations = vec![ResourceCapacity {
                        resource_id: "pool".into(),
                        capacity,
                    }];
                    let report =
                        validate_schedule_with_resource_capacities(&operations, &declarations);
                    assert_eq!(
                        report.is_feasible(),
                        peak_concurrency <= capacity,
                        "operation_count={operation_count}, capacity={capacity}, operations={operations:?}, report={:?}",
                        report.violations
                    );
                }
            }
        }
    }

    #[test]
    fn resource_pool_capacity_respects_half_open_back_to_back_intervals() {
        let operations = vec![
            operation("weld-a", "bay-pool", 0.0, 10.0, &[]),
            operation("weld-b", "bay-pool", 10.0, 7.0, &[]),
        ];
        let capacities = vec![ResourceCapacity {
            resource_id: "bay-pool".into(),
            capacity: 1,
        }];
        assert!(validate_schedule_with_resource_capacities(&operations, &capacities).is_feasible());
    }

    #[test]
    fn missing_capacity_declaration_fails_closed() {
        let report = validate_schedule_with_resource_capacities(
            &[operation("cut", "cnc-pool", 0.0, 2.0, &[])],
            &[],
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::UnknownResourceCapacity),
            1
        );
    }

    #[test]
    fn ambiguous_and_zero_capacity_declarations_are_rejected() {
        let operations = vec![operation("cut", "cnc-pool", 0.0, 2.0, &[])];
        let duplicate = vec![
            ResourceCapacity {
                resource_id: "cnc-pool".into(),
                capacity: 1,
            },
            ResourceCapacity {
                resource_id: "cnc-pool".into(),
                capacity: 2,
            },
        ];
        let report = validate_schedule_with_resource_capacities(&operations, &duplicate);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::DuplicateResourceCapacity),
            1
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::ResourceCapacityExceeded),
            0
        );

        let zero = vec![ResourceCapacity {
            resource_id: "cnc-pool".into(),
            capacity: 0,
        }];
        let report = validate_schedule_with_resource_capacities(&operations, &zero);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::ZeroResourceCapacity),
            1
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::ResourceCapacityExceeded),
            0
        );
    }

    #[test]
    fn resource_capacity_reports_are_deterministically_ordered() {
        let operations = vec![
            operation("c", "crane-pool", 2.0, 10.0, &[]),
            operation("a", "crane-pool", 0.0, 10.0, &[]),
            operation("b", "crane-pool", 1.0, 10.0, &[]),
        ];
        let capacities = vec![ResourceCapacity {
            resource_id: "crane-pool".into(),
            capacity: 1,
        }];
        let first = validate_schedule_with_resource_capacities(&operations, &capacities);
        let second = validate_schedule_with_resource_capacities(&operations, &capacities);
        assert_eq!(first, second);
        assert!(!first.is_feasible());
    }

    #[test]
    fn exclusive_sweep_matches_independent_pairwise_oracle_for_tiny_schedules() {
        // Exhaustive synthetic test over start, duration, and two resource labels.
        // The expected conflict set is computed independently with direct pair checks.
        for operation_count in 1usize..=4 {
            let cases = 6usize.pow(operation_count as u32);
            for mut encoded in 0..cases {
                let mut operations = Vec::with_capacity(operation_count);
                for index in 0..operation_count {
                    let choice = encoded % 6;
                    encoded /= 6;
                    let start = (choice % 3) as f64;
                    let duration = if choice / 3 == 0 { 1.0 } else { 2.0 };
                    let resource = if index % 2 == 0 { "pool-a" } else { "pool-b" };
                    operations.push(operation(
                        &format!("op-{index}"),
                        resource,
                        start,
                        duration,
                        &[],
                    ));
                }

                let mut expected = Vec::new();
                for left_index in 0..operations.len() {
                    for right_index in (left_index + 1)..operations.len() {
                        let left = &operations[left_index];
                        let right = &operations[right_index];
                        let left_end = left.end_seconds().unwrap();
                        let right_end = right.end_seconds().unwrap();
                        if left.resource_id == right.resource_id
                            && left.start_seconds < right_end
                            && right.start_seconds < left_end
                        {
                            let mut ids = vec![left.id.clone(), right.id.clone()];
                            ids.sort();
                            expected.push(ids);
                        }
                    }
                }
                expected.sort();

                let mut actual: Vec<Vec<String>> = validate_schedule(&operations)
                    .violations
                    .into_iter()
                    .filter(|v| v.code == ScheduleViolationCode::ExclusiveResourceOverlap)
                    .map(|v| v.operation_ids)
                    .collect();
                actual.sort();
                assert_eq!(
                    actual, expected,
                    "operation_count={operation_count}, operations={operations:?}"
                );
            }
        }
    }

    #[test]
    fn sparse_schedule_with_many_distinct_resources_is_feasible() {
        // This is a scalability regression fixture, not a timing assertion. A
        // full all-pairs scan would perform about 72 million pair checks here.
        let operations: Vec<ScheduledOperation> = (0..12_000)
            .map(|index| {
                operation(
                    &format!("op-{index}"),
                    &format!("resource-{index}"),
                    index as f64 * 2.0,
                    1.0,
                    &[],
                )
            })
            .collect();
        let report = validate_schedule(&operations);
        assert!(report.is_feasible(), "{:?}", report.violations);
    }

    #[test]
    fn duration_upper_bounds_reject_nominally_feasible_overlap() {
        let operations = vec![
            operation("weld-a", "bay-1", 0.0, 5.0, &[]),
            operation("weld-b", "bay-1", 5.0, 2.0, &[]),
        ];
        let bounds = vec![
            OperationDurationBounds {
                operation_id: "weld-a".into(),
                minimum_seconds: 5.0,
                maximum_seconds: 8.0,
            },
            OperationDurationBounds {
                operation_id: "weld-b".into(),
                minimum_seconds: 2.0,
                maximum_seconds: 4.0,
            },
        ];
        let report = validate_schedule_with_duration_bounds(&operations, &bounds);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::ExclusiveResourceOverlap),
            1
        );
        assert!(!report.is_feasible());
    }

    #[test]
    fn duration_upper_bounds_allow_robust_back_to_back_precedence() {
        let operations = vec![
            operation("fit-up", "bay-1", 0.0, 5.0, &[]),
            operation("weld", "bay-1", 6.0, 3.0, &["fit-up"]),
        ];
        let bounds = vec![
            OperationDurationBounds {
                operation_id: "fit-up".into(),
                minimum_seconds: 4.0,
                maximum_seconds: 6.0,
            },
            OperationDurationBounds {
                operation_id: "weld".into(),
                minimum_seconds: 3.0,
                maximum_seconds: 4.0,
            },
        ];
        let report = validate_schedule_with_duration_bounds(&operations, &bounds);
        assert!(report.is_feasible(), "{:?}", report.violations);
    }

    #[test]
    fn capacity_aware_duration_bounds_detect_worst_case_pool_overload() {
        let operations = vec![
            operation("inspect-a", "ndt-pool", 0.0, 1.0, &[]),
            operation("inspect-b", "ndt-pool", 1.0, 1.0, &[]),
        ];
        let capacities = vec![ResourceCapacity {
            resource_id: "ndt-pool".into(),
            capacity: 1,
        }];
        let bounds = vec![
            OperationDurationBounds {
                operation_id: "inspect-a".into(),
                minimum_seconds: 1.0,
                maximum_seconds: 3.0,
            },
            OperationDurationBounds {
                operation_id: "inspect-b".into(),
                minimum_seconds: 1.0,
                maximum_seconds: 2.0,
            },
        ];
        let report = validate_schedule_with_resource_capacities_and_duration_bounds(
            &operations,
            &capacities,
            &bounds,
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::ResourceCapacityExceeded),
            1
        );
        assert!(!report.is_feasible());
    }

    #[test]
    fn duration_upper_bounds_enforce_predecessor_completion() {
        let operations = vec![
            operation("cut", "cnc-1", 0.0, 2.0, &[]),
            operation("inspect", "cmm-1", 2.0, 1.0, &["cut"]),
        ];
        let bounds = vec![
            OperationDurationBounds {
                operation_id: "cut".into(),
                minimum_seconds: 2.0,
                maximum_seconds: 4.0,
            },
            OperationDurationBounds {
                operation_id: "inspect".into(),
                minimum_seconds: 1.0,
                maximum_seconds: 2.0,
            },
        ];
        let report = validate_schedule_with_duration_bounds(&operations, &bounds);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::PrecedenceViolation),
            1
        );
    }

    #[test]
    fn duration_bounds_fail_closed_when_missing_duplicate_invalid_or_unknown() {
        let operations = vec![operation("cut", "cnc-1", 0.0, 2.0, &[])];

        let missing = validate_schedule_with_duration_bounds(&operations, &[]);
        assert_eq!(
            missing.violation_count(ScheduleViolationCode::MissingDurationBounds),
            1
        );

        let duplicate = vec![
            OperationDurationBounds {
                operation_id: "cut".into(),
                minimum_seconds: 1.0,
                maximum_seconds: 3.0,
            },
            OperationDurationBounds {
                operation_id: "cut".into(),
                minimum_seconds: 1.0,
                maximum_seconds: 4.0,
            },
        ];
        let report = validate_schedule_with_duration_bounds(&operations, &duplicate);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::DuplicateDurationBounds),
            1
        );

        let invalid = vec![OperationDurationBounds {
            operation_id: "cut".into(),
            minimum_seconds: 4.0,
            maximum_seconds: 3.0,
        }];
        let report = validate_schedule_with_duration_bounds(&operations, &invalid);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::InvalidDurationBounds),
            1
        );

        let nominal_outside = vec![OperationDurationBounds {
            operation_id: "cut".into(),
            minimum_seconds: 3.0,
            maximum_seconds: 4.0,
        }];
        let report = validate_schedule_with_duration_bounds(&operations, &nominal_outside);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::InvalidDurationBounds),
            1
        );

        let malformed = vec![
            OperationDurationBounds {
                operation_id: "cut".into(),
                minimum_seconds: 1.0,
                maximum_seconds: f64::NAN,
            },
            OperationDurationBounds {
                operation_id: " ".into(),
                minimum_seconds: 1.0,
                maximum_seconds: 2.0,
            },
        ];
        let report = validate_schedule_with_duration_bounds(&operations, &malformed);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::InvalidDurationBounds),
            1
        );
        assert_eq!(
            report.violation_count(ScheduleViolationCode::EmptyDurationBoundOperationId),
            1
        );

        let unknown = vec![
            OperationDurationBounds {
                operation_id: "cut".into(),
                minimum_seconds: 1.0,
                maximum_seconds: 3.0,
            },
            OperationDurationBounds {
                operation_id: "missing".into(),
                minimum_seconds: 1.0,
                maximum_seconds: 2.0,
            },
        ];
        let report = validate_schedule_with_duration_bounds(&operations, &unknown);
        assert_eq!(
            report.violation_count(ScheduleViolationCode::UnknownDurationBoundOperation),
            1
        );
        assert!(!report.is_feasible());
    }

    #[test]
    fn wide_precedence_set_accepts_many_unique_predecessors() {
        // Wide fan-in exercises duplicate tracking over thousands of distinct
        // predecessors without requiring a wall-clock assertion.
        let predecessor_count = 2_048usize;
        let mut operations: Vec<ScheduledOperation> = (0..predecessor_count)
            .map(|index| {
                operation(
                    &format!("prep-{index}"),
                    &format!("resource-{index}"),
                    0.0,
                    1.0,
                    &[],
                )
            })
            .collect();
        let predecessor_ids: Vec<String> = (0..predecessor_count)
            .map(|index| format!("prep-{index}"))
            .collect();
        operations.push(ScheduledOperation {
            id: "join".into(),
            job_id: "job-join".into(),
            resource_id: "join-resource".into(),
            start_seconds: 1.0,
            duration_seconds: 1.0,
            predecessors: predecessor_ids,
        });

        let report = validate_schedule(&operations);
        assert!(report.is_feasible(), "{:?}", report.violations);
    }

    #[test]
    fn violation_order_is_stable_for_same_input() {
        let operations = vec![
            operation("b", "crane-1", 0.0, 10.0, &[]),
            operation("a", "crane-1", 5.0, 10.0, &[]),
        ];
        let first = validate_schedule(&operations);
        let second = validate_schedule(&operations);
        assert_eq!(first, second);
    }
}
