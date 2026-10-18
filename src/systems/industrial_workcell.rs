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

use std::collections::HashMap;

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
        end.is_finite().then_some(end)
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
        }

        let mut seen_predecessors: Vec<&str> = Vec::new();
        for predecessor_id in &operation.predecessors {
            if !seen_predecessors.push_unique(predecessor_id) {
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

    for left_index in 0..operations.len() {
        let left = &operations[left_index];
        let Some(left_start) = valid_start(left) else {
            continue;
        };
        let Some(left_end) = left.end_seconds() else {
            continue;
        };
        for right in operations.iter().skip(left_index + 1) {
            if left.resource_id != right.resource_id {
                continue;
            }
            let Some(right_start) = valid_start(right) else {
                continue;
            };
            let Some(right_end) = right.end_seconds() else {
                continue;
            };
            if left_start < right_end && right_start < left_end {
                let mut ids = vec![left.id.clone(), right.id.clone()];
                ids.sort();
                push_violation(
                    &mut violations,
                    ScheduleViolationCode::ExclusiveResourceOverlap,
                    ids,
                    "two operations overlap on one exclusive resource",
                );
            }
        }
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

/// Validate a schedule against explicit resource-pool capacities.
///
/// Every non-empty resource identifier used by an operation must have exactly one
/// capacity declaration. Missing or duplicated declarations fail closed. Time
/// intervals are half-open ([start, end)); end events are processed before start
/// events at the same instant so back-to-back use is permitted.
///
/// The ordinary single-capacity conflict findings are replaced by pool-capacity
/// findings for this validation path; other structural and precedence checks from
/// \`validate_schedule\` are retained.
pub fn validate_schedule_with_resource_capacities(
    operations: &[ScheduledOperation],
    resource_capacities: &[ResourceCapacity],
) -> ScheduleValidationReport {
    let mut report = validate_schedule(operations);
    report
        .violations
        .retain(|violation| violation.code != ScheduleViolationCode::ExclusiveResourceOverlap);

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
        } else if let Some(capacity) = capacities.first().copied() {
            if capacity > 0 {
                valid_capacities.insert(resource_id, capacity);
            }
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
            events.push(CapacityEvent { time: start, starts: true, index, operation });
            events.push(CapacityEvent { time: end, starts: false, index, operation });
        }
        events.sort_by(|left, right| {
            left.time
                .total_cmp(&right.time)
                // false (end) precedes true (start): half-open interval semantics.
                .then_with(|| left.starts.cmp(&right.starts))
                .then_with(|| left.operation.id.cmp(&right.operation.id))
                .then_with(|| left.index.cmp(&right.index))
        });

        let mut active: Vec<(usize, &ScheduledOperation)> = Vec::new();
        for event in events {
            if !event.starts {
                active.retain(|(index, _)| *index != event.index);
                continue;
            }
            if active.len() >= capacity {
                let mut operation_ids: Vec<String> = active
                    .iter()
                    .map(|(_, operation)| operation.id.clone())
                    .chain(std::iter::once(event.operation.id.clone()))
                    .collect();
                operation_ids.sort();
                operation_ids.dedup();
                push_violation(
                    &mut report.violations,
                    ScheduleViolationCode::ResourceCapacityExceeded,
                    operation_ids,
                    &format!(
                        "resource '{}' capacity {} is exceeded by concurrent operations",
                        resource_id, capacity
                    ),
                );
            }
            active.push((event.index, event.operation));
        }
    }

    normalize_violations(&mut report.violations);
    report
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

trait PushUnique<T> {
    fn push_unique(&mut self, value: T) -> bool;
}

impl<T: PartialEq> PushUnique<T> for Vec<T> {
    fn push_unique(&mut self, value: T) -> bool {
        if self.contains(&value) {
            false
        } else {
            self.push(value);
            true
        }
    }
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
            report.violations
                .iter()
                .find(|v| v.code == ScheduleViolationCode::ResourceCapacityExceeded)
                .unwrap()
                .operation_ids,
            vec!["inspect-a", "inspect-b", "inspect-c"]
        );
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
            ResourceCapacity { resource_id: "cnc-pool".into(), capacity: 1 },
            ResourceCapacity { resource_id: "cnc-pool".into(), capacity: 2 },
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

        let zero = vec![ResourceCapacity { resource_id: "cnc-pool".into(), capacity: 0 }];
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
