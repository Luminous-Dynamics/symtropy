//! Exclusive-resource recovery checks for workstations, cranes, lifts and inspection stations.
//!
//! V0 models each resource ID as unary (at most one reservation at a time). It does not model
//! capacity >1, tandem lifts, swept volumes, load charts, crane interference geometry, or operator
//! qualifications. Those constraints must be added as explicit resource profiles before use.

use std::collections::{BTreeMap, BTreeSet};

use shipyard_space_time::TimeRange;

use super::{RecoveryInput, RecoveryReport, evaluate_recovery};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceReservation {
    pub reservation_id: String,
    pub operation_id: String,
    pub resource_id: String,
    /// Human-readable profile such as workstation, gantry-crane, mobile-crane, tug, inspector or lift-team.
    pub resource_kind: String,
    pub interval: TimeRange,
}

/// Finish-to-start dependency between two manufacturing operations.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OperationPrecedence {
    pub constraint_id: String,
    pub predecessor_operation_id: String,
    pub successor_operation_id: String,
    /// Minimum wait after the predecessor's final reserved resource interval.
    pub minimum_lag_ticks: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResourceSchedule {
    pub precedence_constraints: Vec<OperationPrecedence>,
    pub reservations: Vec<ResourceReservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceOutage {
    pub outage_id: String,
    pub resource_id: String,
    pub unavailable: TimeRange,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ResourceViolationCode {
    EmptyResourceSchedule,
    EmptyReservationId,
    EmptyOperationId,
    EmptyResourceId,
    EmptyResourceKind,
    DuplicateReservationId,
    InvalidReservationInterval,
    ConflictingResourceKinds,
    ResourceKindChangedAcrossSchedules,
    EmptySpatialBindingId,
    DuplicateSpatialBindingId,
    UnknownBoundResourceReservation,
    UnknownBoundTransfer,
    DuplicateSpatialBindingPair,
    TransferMissingResourceBinding,
    ResourceReservationDoesNotCoverTransfer,
    ResourceOperationTransferMismatch,
    TransferVehicleResourceMismatch,
    TransferVehicleReservationMissing,
    ExclusiveResourceOverlap,
    EmptyPrecedenceConstraintId,
    DuplicatePrecedenceConstraintId,
    EmptyPredecessorOperationId,
    EmptySuccessorOperationId,
    SelfPrecedence,
    DuplicatePrecedenceEdge,
    UnknownPredecessorOperation,
    UnknownSuccessorOperation,
    PrecedenceTimeConflict,
    PrecedenceCycle,
    PrecedenceConstraintsChanged,
    EmptyOutageId,
    EmptyOutageResourceId,
    DuplicateOutageId,
    InvalidOutageInterval,
    UnknownOutageResource,
    ResourceUnavailable,
    CompletedReservationChanged,
    RequiredReservationRemoved,
    ActiveReservationChanged,
    BaselineScheduleInvalid,
    CandidateScheduleInvalid,
}

impl ResourceViolationCode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::EmptyResourceSchedule => "EmptyResourceSchedule",
            Self::EmptyReservationId => "EmptyReservationId",
            Self::EmptyOperationId => "EmptyOperationId",
            Self::EmptyResourceId => "EmptyResourceId",
            Self::EmptyResourceKind => "EmptyResourceKind",
            Self::DuplicateReservationId => "DuplicateReservationId",
            Self::InvalidReservationInterval => "InvalidReservationInterval",
            Self::ConflictingResourceKinds => "ConflictingResourceKinds",
            Self::ResourceKindChangedAcrossSchedules => "ResourceKindChangedAcrossSchedules",
            Self::EmptySpatialBindingId => "EmptySpatialBindingId",
            Self::DuplicateSpatialBindingId => "DuplicateSpatialBindingId",
            Self::UnknownBoundResourceReservation => "UnknownBoundResourceReservation",
            Self::UnknownBoundTransfer => "UnknownBoundTransfer",
            Self::DuplicateSpatialBindingPair => "DuplicateSpatialBindingPair",
            Self::TransferMissingResourceBinding => "TransferMissingResourceBinding",
            Self::ResourceReservationDoesNotCoverTransfer => {
                "ResourceReservationDoesNotCoverTransfer"
            }
            Self::ResourceOperationTransferMismatch => "ResourceOperationTransferMismatch",
            Self::TransferVehicleResourceMismatch => "TransferVehicleResourceMismatch",
            Self::TransferVehicleReservationMissing => "TransferVehicleReservationMissing",
            Self::ExclusiveResourceOverlap => "ExclusiveResourceOverlap",
            Self::EmptyPrecedenceConstraintId => "EmptyPrecedenceConstraintId",
            Self::DuplicatePrecedenceConstraintId => "DuplicatePrecedenceConstraintId",
            Self::EmptyPredecessorOperationId => "EmptyPredecessorOperationId",
            Self::EmptySuccessorOperationId => "EmptySuccessorOperationId",
            Self::SelfPrecedence => "SelfPrecedence",
            Self::DuplicatePrecedenceEdge => "DuplicatePrecedenceEdge",
            Self::UnknownPredecessorOperation => "UnknownPredecessorOperation",
            Self::UnknownSuccessorOperation => "UnknownSuccessorOperation",
            Self::PrecedenceTimeConflict => "PrecedenceTimeConflict",
            Self::PrecedenceCycle => "PrecedenceCycle",
            Self::PrecedenceConstraintsChanged => "PrecedenceConstraintsChanged",
            Self::EmptyOutageId => "EmptyOutageId",
            Self::EmptyOutageResourceId => "EmptyOutageResourceId",
            Self::DuplicateOutageId => "DuplicateOutageId",
            Self::InvalidOutageInterval => "InvalidOutageInterval",
            Self::UnknownOutageResource => "UnknownOutageResource",
            Self::ResourceUnavailable => "ResourceUnavailable",
            Self::CompletedReservationChanged => "CompletedReservationChanged",
            Self::RequiredReservationRemoved => "RequiredReservationRemoved",
            Self::ActiveReservationChanged => "ActiveReservationChanged",
            Self::BaselineScheduleInvalid => "BaselineScheduleInvalid",
            Self::CandidateScheduleInvalid => "CandidateScheduleInvalid",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceViolation {
    pub code: ResourceViolationCode,
    pub subject_ids: Vec<String>,
    pub detail: String,
}

fn push(
    violations: &mut Vec<ResourceViolation>,
    code: ResourceViolationCode,
    mut subject_ids: Vec<String>,
    detail: impl Into<String>,
) {
    subject_ids.sort();
    violations.push(ResourceViolation {
        code,
        subject_ids,
        detail: detail.into(),
    });
}

fn sort_dedup(violations: &mut Vec<ResourceViolation>) {
    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.subject_ids.cmp(&right.subject_ids))
            .then_with(|| left.detail.cmp(&right.detail))
    });
    violations.dedup();
}

fn operation_windows(schedule: &ResourceSchedule) -> BTreeMap<String, (u64, u64)> {
    let mut windows: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for reservation in &schedule.reservations {
        if reservation.operation_id.trim().is_empty() || !reservation.interval.is_valid() {
            continue;
        }
        windows
            .entry(reservation.operation_id.clone())
            .and_modify(|window| {
                window.0 = window.0.min(reservation.interval.start_tick);
                window.1 = window.1.max(reservation.interval.end_tick);
            })
            .or_insert((
                reservation.interval.start_tick,
                reservation.interval.end_tick,
            ));
    }
    windows
}

fn normalized_precedence(schedule: &ResourceSchedule) -> Vec<OperationPrecedence> {
    let mut constraints = schedule.precedence_constraints.clone();
    constraints.sort();
    constraints
}

fn validate_precedence(schedule: &ResourceSchedule) -> Vec<ResourceViolation> {
    let mut violations = Vec::new();
    let windows = operation_windows(schedule);
    let mut seen_constraint_ids = BTreeSet::new();
    let mut seen_edges = BTreeSet::new();
    let mut edges: BTreeMap<String, BTreeSet<String>> = windows
        .keys()
        .map(|operation_id| (operation_id.clone(), BTreeSet::new()))
        .collect();
    let mut indegree: BTreeMap<String, usize> = windows
        .keys()
        .map(|operation_id| (operation_id.clone(), 0))
        .collect();

    for constraint in &schedule.precedence_constraints {
        if constraint.constraint_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptyPrecedenceConstraintId,
                vec![constraint.constraint_id.clone()],
                "precedence constraint ID is required",
            );
        } else if !seen_constraint_ids.insert(constraint.constraint_id.as_str()) {
            push(
                &mut violations,
                ResourceViolationCode::DuplicatePrecedenceConstraintId,
                vec![constraint.constraint_id.clone()],
                "precedence constraint IDs must be unique",
            );
        }

        if constraint.predecessor_operation_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptyPredecessorOperationId,
                vec![constraint.constraint_id.clone()],
                "predecessor operation ID is required",
            );
        }
        if constraint.successor_operation_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptySuccessorOperationId,
                vec![constraint.constraint_id.clone()],
                "successor operation ID is required",
            );
        }
        if !constraint.predecessor_operation_id.trim().is_empty()
            && constraint.predecessor_operation_id == constraint.successor_operation_id
        {
            push(
                &mut violations,
                ResourceViolationCode::SelfPrecedence,
                vec![
                    constraint.constraint_id.clone(),
                    constraint.predecessor_operation_id.clone(),
                ],
                "an operation cannot precede itself",
            );
        }

        let predecessor = windows.get(&constraint.predecessor_operation_id);
        let successor = windows.get(&constraint.successor_operation_id);
        if predecessor.is_none() && !constraint.predecessor_operation_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::UnknownPredecessorOperation,
                vec![
                    constraint.constraint_id.clone(),
                    constraint.predecessor_operation_id.clone(),
                ],
                "precedence predecessor is absent from the resource schedule",
            );
        }
        if successor.is_none() && !constraint.successor_operation_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::UnknownSuccessorOperation,
                vec![
                    constraint.constraint_id.clone(),
                    constraint.successor_operation_id.clone(),
                ],
                "precedence successor is absent from the resource schedule",
            );
        }

        if let (Some((_, predecessor_end)), Some((successor_start, _))) = (predecessor, successor) {
            let earliest_successor_start =
                predecessor_end.checked_add(constraint.minimum_lag_ticks);
            if earliest_successor_start.is_none_or(|earliest| *successor_start < earliest) {
                push(
                    &mut violations,
                    ResourceViolationCode::PrecedenceTimeConflict,
                    vec![
                        constraint.constraint_id.clone(),
                        constraint.predecessor_operation_id.clone(),
                        constraint.successor_operation_id.clone(),
                    ],
                    format!(
                        "successor starts at tick {successor_start}, before required predecessor completion plus lag {:?}",
                        earliest_successor_start
                    ),
                );
            }

            let edge = (
                constraint.predecessor_operation_id.clone(),
                constraint.successor_operation_id.clone(),
            );
            if !seen_edges.insert(edge.clone()) {
                push(
                    &mut violations,
                    ResourceViolationCode::DuplicatePrecedenceEdge,
                    vec![
                        constraint.predecessor_operation_id.clone(),
                        constraint.successor_operation_id.clone(),
                    ],
                    "operation dependency edge is duplicated",
                );
            } else if edge.0 != edge.1 {
                edges
                    .entry(edge.0.clone())
                    .or_default()
                    .insert(edge.1.clone());
                *indegree.entry(edge.1).or_default() += 1;
            }
        }
    }

    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(operation_id, _)| operation_id.clone())
        .collect();
    let mut visited = 0usize;
    while let Some(operation_id) = ready.pop_first() {
        visited += 1;
        if let Some(successors) = edges.get(&operation_id) {
            for successor in successors {
                if let Some(degree) = indegree.get_mut(successor) {
                    *degree = degree.saturating_sub(1);
                    if *degree == 0 {
                        ready.insert(successor.clone());
                    }
                }
            }
        }
    }

    if visited < indegree.len() {
        let cycle_nodes: Vec<String> = indegree
            .iter()
            .filter(|(_, degree)| **degree > 0)
            .map(|(operation_id, _)| operation_id.clone())
            .collect();
        push(
            &mut violations,
            ResourceViolationCode::PrecedenceCycle,
            cycle_nodes,
            "finish-to-start operation dependencies contain a directed cycle",
        );
    }

    sort_dedup(&mut violations);
    violations
}

fn validate_schedule(schedule: &ResourceSchedule) -> Vec<ResourceViolation> {
    let mut violations = Vec::new();
    if schedule.reservations.is_empty() {
        push(
            &mut violations,
            ResourceViolationCode::EmptyResourceSchedule,
            Vec::new(),
            "resource schedule must explicitly enumerate at least one required resource reservation",
        );
    }
    let mut seen_ids = BTreeSet::new();
    let mut kinds_by_resource: BTreeMap<&str, &str> = BTreeMap::new();

    for reservation in &schedule.reservations {
        if reservation.reservation_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptyReservationId,
                vec![reservation.reservation_id.clone()],
                "reservation identity is required",
            );
        } else if !seen_ids.insert(reservation.reservation_id.as_str()) {
            push(
                &mut violations,
                ResourceViolationCode::DuplicateReservationId,
                vec![reservation.reservation_id.clone()],
                "reservation identity must be unique",
            );
        }
        if reservation.operation_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptyOperationId,
                vec![reservation.reservation_id.clone()],
                "operation identity is required",
            );
        }
        if reservation.resource_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptyResourceId,
                vec![reservation.reservation_id.clone()],
                "resource identity is required",
            );
        }
        if reservation.resource_kind.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptyResourceKind,
                vec![reservation.reservation_id.clone()],
                "resource profile is required",
            );
        }
        if !reservation.interval.is_valid() {
            push(
                &mut violations,
                ResourceViolationCode::InvalidReservationInterval,
                vec![reservation.reservation_id.clone()],
                "reservation interval must be a non-empty half-open interval",
            );
        }

        if !reservation.resource_id.trim().is_empty()
            && !reservation.resource_kind.trim().is_empty()
        {
            if let Some(prior_kind) =
                kinds_by_resource.insert(&reservation.resource_id, &reservation.resource_kind)
            {
                if prior_kind != reservation.resource_kind {
                    push(
                        &mut violations,
                        ResourceViolationCode::ConflictingResourceKinds,
                        vec![
                            reservation.resource_id.clone(),
                            prior_kind.to_owned(),
                            reservation.resource_kind.clone(),
                        ],
                        "one resource ID cannot silently change kind/profile",
                    );
                }
            }
        }
    }

    for (index, left) in schedule.reservations.iter().enumerate() {
        for right in schedule.reservations.iter().skip(index + 1) {
            if left.resource_id == right.resource_id
                && left.interval.is_valid()
                && right.interval.is_valid()
                && left.interval.overlaps(right.interval)
            {
                push(
                    &mut violations,
                    ResourceViolationCode::ExclusiveResourceOverlap,
                    vec![left.reservation_id.clone(), right.reservation_id.clone()],
                    "unary resource has overlapping reservations",
                );
            }
        }
    }

    violations.extend(validate_precedence(schedule));
    sort_dedup(&mut violations);
    violations
}

fn reservation_map(schedule: &ResourceSchedule) -> BTreeMap<&str, &ResourceReservation> {
    schedule
        .reservations
        .iter()
        .map(|reservation| (reservation.reservation_id.as_str(), reservation))
        .collect()
}

fn validate_cross_schedule_resource_kinds(
    baseline: &ResourceSchedule,
    candidate: &ResourceSchedule,
) -> Vec<ResourceViolation> {
    let mut baseline_kinds: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for reservation in &baseline.reservations {
        if !reservation.resource_id.trim().is_empty()
            && !reservation.resource_kind.trim().is_empty()
        {
            baseline_kinds
                .entry(reservation.resource_id.as_str())
                .or_default()
                .insert(reservation.resource_kind.as_str());
        }
    }

    let mut violations = Vec::new();
    for reservation in &candidate.reservations {
        if let Some(kinds) = baseline_kinds.get(reservation.resource_id.as_str()) {
            if !kinds.contains(reservation.resource_kind.as_str()) {
                push(
                    &mut violations,
                    ResourceViolationCode::ResourceKindChangedAcrossSchedules,
                    vec![
                        reservation.reservation_id.clone(),
                        reservation.resource_id.clone(),
                        reservation.resource_kind.clone(),
                    ],
                    "candidate cannot reclassify a known resource ID",
                );
            }
        }
    }
    sort_dedup(&mut violations);
    violations
}

fn validate_outages(
    outages: &[ResourceOutage],
    baseline: &ResourceSchedule,
    candidate: &ResourceSchedule,
) -> Vec<ResourceViolation> {
    let mut violations = Vec::new();
    let known_resources: BTreeSet<&str> = baseline
        .reservations
        .iter()
        .chain(candidate.reservations.iter())
        .map(|reservation| reservation.resource_id.as_str())
        .collect();
    let mut seen_ids = BTreeSet::new();

    for outage in outages {
        if outage.outage_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptyOutageId,
                vec![outage.outage_id.clone()],
                "outage identity is required",
            );
        } else if !seen_ids.insert(outage.outage_id.as_str()) {
            push(
                &mut violations,
                ResourceViolationCode::DuplicateOutageId,
                vec![outage.outage_id.clone()],
                "outage identities must be unique",
            );
        }
        if outage.resource_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptyOutageResourceId,
                vec![outage.outage_id.clone()],
                "outage resource identity is required",
            );
        } else if !known_resources.contains(outage.resource_id.as_str()) {
            push(
                &mut violations,
                ResourceViolationCode::UnknownOutageResource,
                vec![outage.outage_id.clone(), outage.resource_id.clone()],
                "outage target is not present in either schedule",
            );
        }
        if !outage.unavailable.is_valid() {
            push(
                &mut violations,
                ResourceViolationCode::InvalidOutageInterval,
                vec![outage.outage_id.clone()],
                "outage interval must be a non-empty half-open interval",
            );
        }
    }

    sort_dedup(&mut violations);
    violations
}

fn resource_outage_conflicts(
    schedule: &ResourceSchedule,
    outages: &[ResourceOutage],
) -> Vec<ResourceViolation> {
    let mut violations = Vec::new();
    for reservation in &schedule.reservations {
        for outage in outages {
            if reservation.resource_id == outage.resource_id
                && reservation.interval.is_valid()
                && outage.unavailable.is_valid()
                && reservation.interval.overlaps(outage.unavailable)
            {
                push(
                    &mut violations,
                    ResourceViolationCode::ResourceUnavailable,
                    vec![reservation.reservation_id.clone(), outage.outage_id.clone()],
                    "reservation overlaps an unavailable resource interval",
                );
            }
        }
    }
    sort_dedup(&mut violations);
    violations
}

fn in_progress_and_history_mutations(
    baseline: &ResourceSchedule,
    candidate: &ResourceSchedule,
    observed_at_tick: u64,
) -> Vec<ResourceViolation> {
    let mut violations = Vec::new();
    let old_map = reservation_map(baseline);
    let new_map = reservation_map(candidate);

    for (id, old) in old_map {
        if old.interval.end_tick <= observed_at_tick {
            if new_map.get(id) != Some(&old) {
                push(
                    &mut violations,
                    ResourceViolationCode::CompletedReservationChanged,
                    vec![id.to_owned()],
                    "completed operation history is immutable",
                );
            }
        } else if !new_map.contains_key(id) {
            push(
                &mut violations,
                ResourceViolationCode::RequiredReservationRemoved,
                vec![id.to_owned()],
                "future or active operation cannot silently disappear from the candidate schedule",
            );
        } else if old.interval.start_tick <= observed_at_tick
            && observed_at_tick < old.interval.end_tick
        {
            let unchanged_while_active = new_map.get(id).is_some_and(|new| {
                new.operation_id == old.operation_id
                    && new.resource_id == old.resource_id
                    && new.resource_kind == old.resource_kind
                    && new.interval.start_tick == old.interval.start_tick
                    && new.interval.end_tick >= old.interval.end_tick
            });
            if !unchanged_while_active {
                push(
                    &mut violations,
                    ResourceViolationCode::ActiveReservationChanged,
                    vec![id.to_owned()],
                    "active resource operation cannot be moved or reassigned mid-operation",
                );
            }
        }
    }

    sort_dedup(&mut violations);
    violations
}

fn affected_reservations(
    baseline: &ResourceSchedule,
    outages: &[ResourceOutage],
) -> BTreeSet<String> {
    baseline
        .reservations
        .iter()
        .filter(|reservation| {
            outages.iter().any(|outage| {
                reservation.resource_id == outage.resource_id
                    && reservation.interval.overlaps(outage.unavailable)
            })
        })
        .map(|reservation| reservation.reservation_id.clone())
        .collect()
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResourceRecoveryMetrics {
    pub baseline_feasible_before_disruption: bool,
    pub baseline_feasible_after_disruption: bool,
    pub candidate_feasible_after_disruption: bool,
    pub affected_reservation_count: usize,
    pub changed_reservations: usize,
    pub unaffected_reservation_churn: usize,
    pub total_time_shift_ticks: u64,
    pub candidate_makespan_tick: Option<u64>,
    pub recovery_completion_tick: Option<u64>,
    pub recovery_horizon_ticks: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceRecoveryReport {
    pub accepted: bool,
    pub metrics: ResourceRecoveryMetrics,
    pub affected_reservation_ids: Vec<String>,
    pub violations: Vec<ResourceViolation>,
    pub baseline_violations: Vec<ResourceViolation>,
    pub candidate_violations: Vec<ResourceViolation>,
}

/// Evaluate a resource-only recovery. Every resource ID is treated as an exclusive unary
/// resource; overlapping intervals are rejected using half-open endpoint semantics.
pub fn evaluate_resource_recovery(
    baseline: &ResourceSchedule,
    candidate: &ResourceSchedule,
    observed_at_tick: u64,
    outages: &[ResourceOutage],
) -> ResourceRecoveryReport {
    let baseline_violations = validate_schedule(baseline);
    let candidate_violations = validate_schedule(candidate);
    let mut violations = validate_outages(outages, baseline, candidate);
    let outages_valid = violations.is_empty();
    let resource_kind_violations = validate_cross_schedule_resource_kinds(baseline, candidate);
    let resource_kinds_consistent = resource_kind_violations.is_empty();
    violations.extend(resource_kind_violations);
    let precedence_unchanged = normalized_precedence(baseline) == normalized_precedence(candidate);
    if !precedence_unchanged {
        push(
            &mut violations,
            ResourceViolationCode::PrecedenceConstraintsChanged,
            Vec::new(),
            "candidate must preserve authoritative baseline operation dependencies",
        );
    }
    violations.extend(candidate_violations.iter().cloned());

    let baseline_feasible_before_disruption = baseline_violations.is_empty();
    if !baseline_feasible_before_disruption {
        push(
            &mut violations,
            ResourceViolationCode::BaselineScheduleInvalid,
            Vec::new(),
            "baseline resource schedule is not feasible before disruption",
        );
    }
    if !candidate_violations.is_empty() {
        push(
            &mut violations,
            ResourceViolationCode::CandidateScheduleInvalid,
            Vec::new(),
            "candidate resource schedule violates a resource invariant",
        );
    }

    let baseline_outage_conflicts = resource_outage_conflicts(baseline, outages);
    let candidate_outage_conflicts = resource_outage_conflicts(candidate, outages);
    let baseline_feasible_after_disruption = baseline_feasible_before_disruption
        && outages_valid
        && baseline_outage_conflicts.is_empty();
    let mut candidate_feasible_after_disruption = resource_kinds_consistent
        && precedence_unchanged
        && outages_valid
        && candidate_violations.is_empty()
        && candidate_outage_conflicts.is_empty();

    violations.extend(candidate_outage_conflicts.iter().cloned());
    let mutation_violations =
        in_progress_and_history_mutations(baseline, candidate, observed_at_tick);
    if !mutation_violations.is_empty() {
        candidate_feasible_after_disruption = false;
    }
    violations.extend(mutation_violations);

    let affected = affected_reservations(baseline, outages);
    let old_map = reservation_map(baseline);
    let new_map = reservation_map(candidate);
    let reservation_ids: BTreeSet<&str> = old_map
        .keys()
        .copied()
        .chain(new_map.keys().copied())
        .collect();
    let mut changed_reservations = 0usize;
    let mut unaffected_reservation_churn = 0usize;
    let mut total_time_shift_ticks = 0u64;
    let mut changed_ids = BTreeSet::new();

    for id in reservation_ids {
        if old_map.get(id) != new_map.get(id) {
            changed_reservations += 1;
            changed_ids.insert(id.to_owned());
            if !affected.contains(id) {
                unaffected_reservation_churn += 1;
            }
            if let (Some(old), Some(new)) = (old_map.get(id), new_map.get(id)) {
                total_time_shift_ticks = total_time_shift_ticks
                    .saturating_add(old.interval.start_tick.abs_diff(new.interval.start_tick))
                    .saturating_add(old.interval.end_tick.abs_diff(new.interval.end_tick));
            }
        }
    }

    let candidate_makespan_tick = candidate
        .reservations
        .iter()
        .map(|reservation| reservation.interval.end_tick)
        .max();
    let mut recovery_completion_tick = None;
    for reservation in &candidate.reservations {
        if changed_ids.contains(&reservation.reservation_id)
            || affected.contains(&reservation.reservation_id)
        {
            recovery_completion_tick = Some(
                recovery_completion_tick.map_or(reservation.interval.end_tick, |prior: u64| {
                    prior.max(reservation.interval.end_tick)
                }),
            );
        }
    }
    let recovery_horizon_ticks =
        recovery_completion_tick.map(|tick| tick.saturating_sub(observed_at_tick));

    sort_dedup(&mut violations);
    let accepted = baseline_feasible_before_disruption
        && candidate_feasible_after_disruption
        && violations.is_empty();

    ResourceRecoveryReport {
        accepted,
        metrics: ResourceRecoveryMetrics {
            baseline_feasible_before_disruption,
            baseline_feasible_after_disruption,
            candidate_feasible_after_disruption,
            affected_reservation_count: affected.len(),
            changed_reservations,
            unaffected_reservation_churn,
            total_time_shift_ticks,
            candidate_makespan_tick,
            recovery_completion_tick,
            recovery_horizon_ticks,
        },
        affected_reservation_ids: affected.into_iter().collect(),
        violations,
        baseline_violations,
        candidate_violations,
    }
}

/// Bind a concrete resource reservation to a concrete spatial transfer.
/// Multiple resources may be bound to one transfer (for example a tug and a lifting crane).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ResourceSpatialBinding {
    pub binding_id: String,
    pub resource_reservation_id: String,
    pub transfer_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointRecoveryInput {
    pub spatial: RecoveryInput,
    pub baseline_resources: ResourceSchedule,
    pub candidate_resources: ResourceSchedule,
    pub resource_outages: Vec<ResourceOutage>,
    pub baseline_bindings: Vec<ResourceSpatialBinding>,
    pub candidate_bindings: Vec<ResourceSpatialBinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointRecoveryReport {
    /// Candidate is accepted only when both the space-time and unary-resource evaluators accept.
    pub accepted: bool,
    pub spatial: RecoveryReport,
    pub resources: ResourceRecoveryReport,
}

fn validate_spatial_bindings(
    scenario: &super::Scenario,
    resources: &ResourceSchedule,
    bindings: &[ResourceSpatialBinding],
) -> Vec<ResourceViolation> {
    let resource_by_id: BTreeMap<&str, &ResourceReservation> = resources
        .reservations
        .iter()
        .map(|reservation| (reservation.reservation_id.as_str(), reservation))
        .collect();
    let transfer_by_id: BTreeMap<&str, &shipyard_space_time::Transfer> = scenario
        .transfers
        .iter()
        .map(|transfer| (transfer.id.as_str(), transfer))
        .collect();

    let mut violations = Vec::new();
    let mut seen_binding_ids = BTreeSet::new();
    let mut seen_pairs = BTreeSet::new();
    let mut covered_transfers = BTreeSet::new();
    let mut vehicle_reserved_transfers = BTreeSet::new();

    for binding in bindings {
        if binding.binding_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::EmptySpatialBindingId,
                vec![
                    binding.resource_reservation_id.clone(),
                    binding.transfer_id.clone(),
                ],
                "resource-to-transfer binding identity is required",
            );
        } else if !seen_binding_ids.insert(binding.binding_id.as_str()) {
            push(
                &mut violations,
                ResourceViolationCode::DuplicateSpatialBindingId,
                vec![binding.binding_id.clone()],
                "resource-to-transfer binding IDs must be unique",
            );
        }

        let pair = (
            binding.resource_reservation_id.as_str(),
            binding.transfer_id.as_str(),
        );
        if !seen_pairs.insert(pair) {
            push(
                &mut violations,
                ResourceViolationCode::DuplicateSpatialBindingPair,
                vec![
                    binding.resource_reservation_id.clone(),
                    binding.transfer_id.clone(),
                ],
                "one resource reservation and transfer pair may only be bound once",
            );
        }

        let resource = resource_by_id.get(binding.resource_reservation_id.as_str());
        let transfer = transfer_by_id.get(binding.transfer_id.as_str());
        if resource.is_none() {
            push(
                &mut violations,
                ResourceViolationCode::UnknownBoundResourceReservation,
                vec![
                    binding.binding_id.clone(),
                    binding.resource_reservation_id.clone(),
                ],
                "binding references a resource reservation absent from the schedule",
            );
        }
        if transfer.is_none() {
            push(
                &mut violations,
                ResourceViolationCode::UnknownBoundTransfer,
                vec![binding.binding_id.clone(), binding.transfer_id.clone()],
                "binding references a spatial transfer absent from the scenario",
            );
        }

        if let (Some(resource), Some(transfer)) = (resource, transfer) {
            if resource.operation_id != transfer.id {
                push(
                    &mut violations,
                    ResourceViolationCode::ResourceOperationTransferMismatch,
                    vec![
                        binding.binding_id.clone(),
                        resource.operation_id.clone(),
                        transfer.id.clone(),
                    ],
                    "a resource reservation bound to a transfer must name that transfer as its operation",
                );
            }
            let covers_transfer = resource.interval.start_tick <= transfer.interval.start_tick
                && resource.interval.end_tick >= transfer.interval.end_tick;
            if covers_transfer && resource.operation_id == transfer.id {
                covered_transfers.insert(transfer.id.as_str());
                if resource.resource_kind.eq_ignore_ascii_case("tug")
                    && resource.resource_id == transfer.vehicle_id
                {
                    vehicle_reserved_transfers.insert(transfer.id.as_str());
                }
            } else {
                push(
                    &mut violations,
                    ResourceViolationCode::ResourceReservationDoesNotCoverTransfer,
                    vec![
                        binding.binding_id.clone(),
                        binding.resource_reservation_id.clone(),
                        binding.transfer_id.clone(),
                    ],
                    "resource reservation must name the transfer operation and contain its complete time interval",
                );
            }
            if resource.resource_kind.eq_ignore_ascii_case("tug")
                && resource.resource_id != transfer.vehicle_id
            {
                push(
                    &mut violations,
                    ResourceViolationCode::TransferVehicleResourceMismatch,
                    vec![
                        binding.binding_id.clone(),
                        transfer.id.clone(),
                        transfer.vehicle_id.clone(),
                        resource.resource_id.clone(),
                    ],
                    "a tug reservation bound to a transfer must name that transfer's vehicle ID",
                );
            }
        }
    }

    for transfer in &scenario.transfers {
        if !covered_transfers.contains(transfer.id.as_str()) {
            push(
                &mut violations,
                ResourceViolationCode::TransferMissingResourceBinding,
                vec![transfer.id.clone()],
                "every spatial transfer must be covered by at least one valid resource binding",
            );
        }
        if !vehicle_reserved_transfers.contains(transfer.id.as_str()) {
            push(
                &mut violations,
                ResourceViolationCode::TransferVehicleReservationMissing,
                vec![transfer.id.clone(), transfer.vehicle_id.clone()],
                "every spatial transfer must bind a tug reservation for its declared vehicle ID",
            );
        }
    }

    sort_dedup(&mut violations);
    violations
}

fn sort_resource_violations(violations: &mut Vec<ResourceViolation>) {
    sort_dedup(violations);
}

/// Compose independent spatial and resource checks without allowing either to override the other.
pub fn evaluate_joint_recovery(input: &JointRecoveryInput) -> JointRecoveryReport {
    let spatial = evaluate_recovery(&input.spatial);
    let mut resources = evaluate_resource_recovery(
        &input.baseline_resources,
        &input.candidate_resources,
        input.spatial.disruption.observed_at_tick,
        &input.resource_outages,
    );

    let baseline_binding_violations = validate_spatial_bindings(
        &input.spatial.baseline,
        &input.baseline_resources,
        &input.baseline_bindings,
    );
    let candidate_binding_violations = validate_spatial_bindings(
        &input.spatial.candidate,
        &input.candidate_resources,
        &input.candidate_bindings,
    );

    if !baseline_binding_violations.is_empty() {
        resources.metrics.baseline_feasible_before_disruption = false;
        resources.metrics.baseline_feasible_after_disruption = false;
        resources
            .baseline_violations
            .extend(baseline_binding_violations.iter().cloned());
        resources.violations.extend(baseline_binding_violations);
    }
    if !candidate_binding_violations.is_empty() {
        resources.metrics.candidate_feasible_after_disruption = false;
        resources
            .candidate_violations
            .extend(candidate_binding_violations.iter().cloned());
        resources.violations.extend(candidate_binding_violations);
    }
    sort_resource_violations(&mut resources.baseline_violations);
    sort_resource_violations(&mut resources.candidate_violations);
    sort_resource_violations(&mut resources.violations);
    resources.accepted = resources.metrics.baseline_feasible_before_disruption
        && resources.metrics.candidate_feasible_after_disruption
        && resources.violations.is_empty();

    let accepted = spatial.accepted && resources.accepted;
    JointRecoveryReport {
        accepted,
        spatial,
        resources,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointDisruptionScenario {
    pub scenario_id: String,
    pub spatial_disruption: super::Disruption,
    pub resource_outages: Vec<ResourceOutage>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum JointEnvelopeViolationCode {
    EmptyScenarioSet,
    EmptyScenarioId,
    DuplicateScenarioId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointEnvelopeViolation {
    pub code: JointEnvelopeViolationCode,
    pub scenario_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointScenarioReport {
    pub scenario_id: String,
    pub report: JointRecoveryReport,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JointRecoveryEnvelopeMetrics {
    pub total_scenarios: usize,
    pub accepted_scenarios: usize,
    pub worst_spatial_recovery_horizon_ticks: Option<u64>,
    pub worst_resource_recovery_horizon_ticks: Option<u64>,
    pub worst_changed_placement_reservations: usize,
    pub worst_changed_transfer_reservations: usize,
    pub worst_unaffected_placement_churn: usize,
    pub worst_unaffected_transfer_churn: usize,
    pub worst_spatial_time_shift_ticks: u64,
    pub worst_rectangle_edge_shift_units: u64,
    pub worst_changed_resource_reservations: usize,
    pub worst_unaffected_resource_churn: usize,
    pub worst_resource_time_shift_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointRecoveryEnvelopeReport {
    pub robustly_accepted: bool,
    pub metrics: JointRecoveryEnvelopeMetrics,
    pub violations: Vec<JointEnvelopeViolation>,
    pub scenarios: Vec<JointScenarioReport>,
}

/// Evaluate the same spatial and resource candidate under every listed joint disturbance.
///
/// Every scenario is a hard case. V0 assigns no probabilities and does not claim coverage of
/// unenumerated disturbances. A single spatial or resource failure rejects the whole envelope.
pub fn evaluate_joint_recovery_envelope(
    baseline_spatial: &super::Scenario,
    candidate_spatial: &super::Scenario,
    baseline_resources: &ResourceSchedule,
    candidate_resources: &ResourceSchedule,
    baseline_bindings: &[ResourceSpatialBinding],
    candidate_bindings: &[ResourceSpatialBinding],
    scenarios: &[JointDisruptionScenario],
) -> JointRecoveryEnvelopeReport {
    let mut violations = Vec::new();
    if scenarios.is_empty() {
        violations.push(JointEnvelopeViolation {
            code: JointEnvelopeViolationCode::EmptyScenarioSet,
            scenario_id: String::new(),
        });
    }

    let mut seen = BTreeSet::new();
    for scenario in scenarios {
        if scenario.scenario_id.trim().is_empty() {
            violations.push(JointEnvelopeViolation {
                code: JointEnvelopeViolationCode::EmptyScenarioId,
                scenario_id: scenario.scenario_id.clone(),
            });
        } else if !seen.insert(scenario.scenario_id.as_str()) {
            violations.push(JointEnvelopeViolation {
                code: JointEnvelopeViolationCode::DuplicateScenarioId,
                scenario_id: scenario.scenario_id.clone(),
            });
        }
    }

    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.scenario_id.cmp(&right.scenario_id))
    });
    violations.dedup();
    if !violations.is_empty() {
        return JointRecoveryEnvelopeReport {
            robustly_accepted: false,
            metrics: JointRecoveryEnvelopeMetrics {
                total_scenarios: scenarios.len(),
                ..JointRecoveryEnvelopeMetrics::default()
            },
            violations,
            scenarios: Vec::new(),
        };
    }

    let mut reports = Vec::with_capacity(scenarios.len());
    for scenario in scenarios {
        let joint = evaluate_joint_recovery(&JointRecoveryInput {
            spatial: RecoveryInput {
                baseline: baseline_spatial.clone(),
                candidate: candidate_spatial.clone(),
                disruption: scenario.spatial_disruption.clone(),
            },
            baseline_resources: baseline_resources.clone(),
            candidate_resources: candidate_resources.clone(),
            resource_outages: scenario.resource_outages.clone(),
            baseline_bindings: baseline_bindings.to_vec(),
            candidate_bindings: candidate_bindings.to_vec(),
        });
        reports.push(JointScenarioReport {
            scenario_id: scenario.scenario_id.clone(),
            report: joint,
        });
    }
    reports.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    let accepted_scenarios = reports
        .iter()
        .filter(|scenario| scenario.report.accepted)
        .count();
    let worst_spatial_recovery_horizon_ticks = reports
        .iter()
        .filter_map(|scenario| scenario.report.spatial.metrics.recovery_horizon_ticks)
        .max();
    let worst_resource_recovery_horizon_ticks = reports
        .iter()
        .filter_map(|scenario| scenario.report.resources.metrics.recovery_horizon_ticks)
        .max();
    let worst_changed_placement_reservations = reports
        .iter()
        .map(|scenario| scenario.report.spatial.metrics.changed_placement_reservations)
        .max()
        .unwrap_or_default();
    let worst_changed_transfer_reservations = reports
        .iter()
        .map(|scenario| scenario.report.spatial.metrics.changed_transfer_reservations)
        .max()
        .unwrap_or_default();
    let worst_unaffected_placement_churn = reports
        .iter()
        .map(|scenario| scenario.report.spatial.metrics.unaffected_placement_churn)
        .max()
        .unwrap_or_default();
    let worst_unaffected_transfer_churn = reports
        .iter()
        .map(|scenario| scenario.report.spatial.metrics.unaffected_transfer_churn)
        .max()
        .unwrap_or_default();
    let worst_spatial_time_shift_ticks = reports
        .iter()
        .map(|scenario| scenario.report.spatial.metrics.time_shift_ticks)
        .max()
        .unwrap_or_default();
    let worst_rectangle_edge_shift_units = reports
        .iter()
        .map(|scenario| scenario.report.spatial.metrics.rectangle_edge_shift_units)
        .max()
        .unwrap_or_default();
    let worst_changed_resource_reservations = reports
        .iter()
        .map(|scenario| scenario.report.resources.metrics.changed_reservations)
        .max()
        .unwrap_or_default();
    let worst_unaffected_resource_churn = reports
        .iter()
        .map(|scenario| scenario.report.resources.metrics.unaffected_reservation_churn)
        .max()
        .unwrap_or_default();
    let worst_resource_time_shift_ticks = reports
        .iter()
        .map(|scenario| scenario.report.resources.metrics.total_time_shift_ticks)
        .max()
        .unwrap_or_default();

    JointRecoveryEnvelopeReport {
        robustly_accepted: !scenarios.is_empty() && accepted_scenarios == scenarios.len(),
        metrics: JointRecoveryEnvelopeMetrics {
            total_scenarios: scenarios.len(),
            accepted_scenarios,
            worst_spatial_recovery_horizon_ticks,
            worst_resource_recovery_horizon_ticks,
            worst_changed_placement_reservations,
            worst_changed_transfer_reservations,
            worst_unaffected_placement_churn,
            worst_unaffected_transfer_churn,
            worst_spatial_time_shift_ticks,
            worst_rectangle_edge_shift_units,
            worst_changed_resource_reservations,
            worst_unaffected_resource_churn,
            worst_resource_time_shift_ticks,
        },
        violations,
        scenarios: reports,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointRecoveryEnvelopeAlternative {
    pub candidate_id: String,
    pub report: JointRecoveryEnvelopeReport,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum JointParetoViolationCode {
    EmptyCandidateId,
    DuplicateCandidateId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointParetoViolation {
    pub code: JointParetoViolationCode,
    pub candidate_id: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct JointParetoScore {
    pub worst_spatial_recovery_horizon_ticks: Option<u64>,
    pub worst_resource_recovery_horizon_ticks: Option<u64>,
    pub worst_changed_placement_reservations: usize,
    pub worst_changed_transfer_reservations: usize,
    pub worst_unaffected_placement_churn: usize,
    pub worst_unaffected_transfer_churn: usize,
    pub worst_spatial_time_shift_ticks: u64,
    pub worst_rectangle_edge_shift_units: u64,
    pub worst_changed_resource_reservations: usize,
    pub worst_unaffected_resource_churn: usize,
    pub worst_resource_time_shift_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointParetoCandidate {
    pub candidate_id: String,
    pub score: JointParetoScore,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JointParetoReport {
    pub frontier: Vec<JointParetoCandidate>,
    pub dominated_candidate_ids: Vec<String>,
    pub rejected_candidate_ids: Vec<String>,
    pub violations: Vec<JointParetoViolation>,
}

fn joint_pareto_score(report: &JointRecoveryEnvelopeReport) -> JointParetoScore {
    JointParetoScore {
        worst_spatial_recovery_horizon_ticks: report
            .metrics
            .worst_spatial_recovery_horizon_ticks,
        worst_resource_recovery_horizon_ticks: report
            .metrics
            .worst_resource_recovery_horizon_ticks,
        worst_changed_placement_reservations: report
            .metrics
            .worst_changed_placement_reservations,
        worst_changed_transfer_reservations: report
            .metrics
            .worst_changed_transfer_reservations,
        worst_unaffected_placement_churn: report.metrics.worst_unaffected_placement_churn,
        worst_unaffected_transfer_churn: report.metrics.worst_unaffected_transfer_churn,
        worst_spatial_time_shift_ticks: report.metrics.worst_spatial_time_shift_ticks,
        worst_rectangle_edge_shift_units: report.metrics.worst_rectangle_edge_shift_units,
        worst_changed_resource_reservations: report.metrics.worst_changed_resource_reservations,
        worst_unaffected_resource_churn: report.metrics.worst_unaffected_resource_churn,
        worst_resource_time_shift_ticks: report.metrics.worst_resource_time_shift_ticks,
    }
}

fn joint_optional_no_worse(left: Option<u64>, right: Option<u64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left <= right,
        (None, None) => true,
        _ => false,
    }
}

fn joint_optional_strictly_better(left: Option<u64>, right: Option<u64>) -> bool {
    matches!((left, right), (Some(left), Some(right)) if left < right)
}

fn joint_pareto_dominates(left: &JointParetoScore, right: &JointParetoScore) -> bool {
    let no_worse = joint_optional_no_worse(
        left.worst_spatial_recovery_horizon_ticks,
        right.worst_spatial_recovery_horizon_ticks,
    ) && joint_optional_no_worse(
        left.worst_resource_recovery_horizon_ticks,
        right.worst_resource_recovery_horizon_ticks,
    ) && left.worst_changed_placement_reservations
        <= right.worst_changed_placement_reservations
        && left.worst_changed_transfer_reservations <= right.worst_changed_transfer_reservations
        && left.worst_unaffected_placement_churn <= right.worst_unaffected_placement_churn
        && left.worst_unaffected_transfer_churn <= right.worst_unaffected_transfer_churn
        && left.worst_spatial_time_shift_ticks <= right.worst_spatial_time_shift_ticks
        && left.worst_rectangle_edge_shift_units <= right.worst_rectangle_edge_shift_units
        && left.worst_changed_resource_reservations <= right.worst_changed_resource_reservations
        && left.worst_unaffected_resource_churn <= right.worst_unaffected_resource_churn
        && left.worst_resource_time_shift_ticks <= right.worst_resource_time_shift_ticks;
    let strictly_better = joint_optional_strictly_better(
        left.worst_spatial_recovery_horizon_ticks,
        right.worst_spatial_recovery_horizon_ticks,
    ) || joint_optional_strictly_better(
        left.worst_resource_recovery_horizon_ticks,
        right.worst_resource_recovery_horizon_ticks,
    ) || left.worst_changed_placement_reservations
        < right.worst_changed_placement_reservations
        || left.worst_changed_transfer_reservations < right.worst_changed_transfer_reservations
        || left.worst_unaffected_placement_churn < right.worst_unaffected_placement_churn
        || left.worst_unaffected_transfer_churn < right.worst_unaffected_transfer_churn
        || left.worst_spatial_time_shift_ticks < right.worst_spatial_time_shift_ticks
        || left.worst_rectangle_edge_shift_units < right.worst_rectangle_edge_shift_units
        || left.worst_changed_resource_reservations < right.worst_changed_resource_reservations
        || left.worst_unaffected_resource_churn < right.worst_unaffected_resource_churn
        || left.worst_resource_time_shift_ticks < right.worst_resource_time_shift_ticks;
    no_worse && strictly_better
}

/// Pareto-rank joint plans only after each candidate passes every spatial and resource scenario.
///
/// No hidden scalar weight is used. Candidates rejected by any joint hard scenario are excluded
/// from the frontier. Missing recovery-horizon metrics are not treated as automatically better
/// than measured horizons; incomparable missing/measured pairs cannot dominate one another.
pub fn pareto_joint_recovery_candidates(
    alternatives: &[JointRecoveryEnvelopeAlternative],
) -> JointParetoReport {
    let mut result = JointParetoReport::default();
    let mut seen = BTreeSet::new();
    for alternative in alternatives {
        if alternative.candidate_id.trim().is_empty() {
            result.violations.push(JointParetoViolation {
                code: JointParetoViolationCode::EmptyCandidateId,
                candidate_id: alternative.candidate_id.clone(),
            });
        } else if !seen.insert(alternative.candidate_id.as_str()) {
            result.violations.push(JointParetoViolation {
                code: JointParetoViolationCode::DuplicateCandidateId,
                candidate_id: alternative.candidate_id.clone(),
            });
        }
    }
    result.violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });
    result.violations.dedup();
    if !result.violations.is_empty() {
        return result;
    }

    let mut eligible = Vec::new();
    for alternative in alternatives {
        if alternative.report.robustly_accepted {
            eligible.push(JointParetoCandidate {
                candidate_id: alternative.candidate_id.clone(),
                score: joint_pareto_score(&alternative.report),
            });
        } else {
            result
                .rejected_candidate_ids
                .push(alternative.candidate_id.clone());
        }
    }
    eligible.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    for candidate in &eligible {
        let dominated = eligible.iter().any(|other| {
            other.candidate_id != candidate.candidate_id
                && joint_pareto_dominates(&other.score, &candidate.score)
        });
        if dominated {
            result
                .dominated_candidate_ids
                .push(candidate.candidate_id.clone());
        } else {
            result.frontier.push(candidate.clone());
        }
    }
    result.dominated_candidate_ids.sort();
    result.rejected_candidate_ids.sort();
    result
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum JointDisruptionDimension {
    ClearanceOverrun,
    VehicleOutage,
    ExclusionZoneClosure,
    ResourceOutage,
    CrossLayerCombination,
    MultiDimensionCombination,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointDimensionReceipt {
    pub dimension: JointDisruptionDimension,
    pub scenario_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum JointCoverageViolationCode {
    EmptyScenarioId,
    DuplicateScenarioId,
    EmptyRequiredDimensionSet,
    DuplicateRequiredDimension,
    MissingRequiredDimension,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointCoverageViolation {
    pub code: JointCoverageViolationCode,
    pub dimension: Option<JointDisruptionDimension>,
    /// Concrete invalid or duplicated scenario identities when the violation is scenario-specific.
    pub scenario_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JointScenarioCoverageReport {
    pub complete: bool,
    pub scenario_count: usize,
    pub receipts: Vec<JointDimensionReceipt>,
    pub missing_required_dimensions: Vec<JointDisruptionDimension>,
    pub violations: Vec<JointCoverageViolation>,
}

fn scenario_dimensions(scenario: &JointDisruptionScenario) -> BTreeSet<JointDisruptionDimension> {
    let mut dimensions = BTreeSet::new();
    let disruption = &scenario.spatial_disruption;
    let has_clearance = !disruption.placement_occupancy_extensions.is_empty();
    let has_vehicle_outage = !disruption.vehicle_outages.is_empty();
    let has_zone_closure = !disruption.new_exclusion_zones.is_empty();
    let has_resource_outage = !scenario.resource_outages.is_empty();
    let spatial_dimensions = (if has_clearance { 1 } else { 0 })
        + (if has_vehicle_outage { 1 } else { 0 })
        + (if has_zone_closure { 1 } else { 0 });

    if has_clearance {
        dimensions.insert(JointDisruptionDimension::ClearanceOverrun);
    }
    if has_vehicle_outage {
        dimensions.insert(JointDisruptionDimension::VehicleOutage);
    }
    if has_zone_closure {
        dimensions.insert(JointDisruptionDimension::ExclusionZoneClosure);
    }
    if has_resource_outage {
        dimensions.insert(JointDisruptionDimension::ResourceOutage);
    }
    if has_resource_outage && spatial_dimensions > 0 {
        dimensions.insert(JointDisruptionDimension::CrossLayerCombination);
    }
    if spatial_dimensions + usize::from(has_resource_outage) > 1 {
        dimensions.insert(JointDisruptionDimension::MultiDimensionCombination);
    }
    dimensions
}

/// Audit whether a finite scenario set covers the disruption dimensions declared by a benchmark.
///
/// This is deliberately separate from robust feasibility: coverage can be complete while a plan
/// fails a case, and every case can pass while the declared coverage is incomplete. It does not
/// assign probabilities or claim coverage of disturbance types not declared in required.
pub fn audit_joint_scenario_coverage(
    scenarios: &[JointDisruptionScenario],
    required: &[JointDisruptionDimension],
) -> JointScenarioCoverageReport {
    let mut report = JointScenarioCoverageReport {
        scenario_count: scenarios.len(),
        ..JointScenarioCoverageReport::default()
    };
    if required.is_empty() {
        report.violations.push(JointCoverageViolation {
            code: JointCoverageViolationCode::EmptyRequiredDimensionSet,
            dimension: None,
            scenario_ids: Vec::new(),
        });
    }

    let mut seen_required = BTreeSet::new();
    for dimension in required {
        if !seen_required.insert(*dimension) {
            report.violations.push(JointCoverageViolation {
                code: JointCoverageViolationCode::DuplicateRequiredDimension,
                dimension: Some(*dimension),
                scenario_ids: Vec::new(),
            });
        }
    }

    let mut scenario_id_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for scenario in scenarios {
        if scenario.scenario_id.trim().is_empty() {
            report.violations.push(JointCoverageViolation {
                code: JointCoverageViolationCode::EmptyScenarioId,
                dimension: None,
                scenario_ids: vec![scenario.scenario_id.clone()],
            });
        } else {
            *scenario_id_counts
                .entry(scenario.scenario_id.as_str())
                .or_default() += 1;
        }
    }
    let duplicate_scenario_ids: BTreeSet<&str> = scenario_id_counts
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(scenario_id, _)| *scenario_id)
        .collect();
    for scenario_id in &duplicate_scenario_ids {
        report.violations.push(JointCoverageViolation {
            code: JointCoverageViolationCode::DuplicateScenarioId,
            dimension: None,
            scenario_ids: vec![scenario_id.to_string()],
        });
    }

    let mut scenario_ids_by_dimension: BTreeMap<JointDisruptionDimension, Vec<String>> =
        BTreeMap::new();
    for scenario in scenarios {
        if scenario.scenario_id.trim().is_empty()
            || duplicate_scenario_ids.contains(scenario.scenario_id.as_str())
        {
            continue;
        }
        for dimension in scenario_dimensions(scenario) {
            scenario_ids_by_dimension
                .entry(dimension)
                .or_default()
                .push(scenario.scenario_id.clone());
        }
    }
    report.receipts = scenario_ids_by_dimension
        .into_iter()
        .map(|(dimension, mut scenario_ids)| {
            scenario_ids.sort();
            scenario_ids.dedup();
            JointDimensionReceipt {
                dimension,
                scenario_ids,
            }
        })
        .collect();

    let observed: BTreeSet<JointDisruptionDimension> =
        report.receipts.iter().map(|receipt| receipt.dimension).collect();
    for dimension in &seen_required {
        if !observed.contains(dimension) {
            report.missing_required_dimensions.push(*dimension);
            report.violations.push(JointCoverageViolation {
                code: JointCoverageViolationCode::MissingRequiredDimension,
                dimension: Some(*dimension),
                scenario_ids: Vec::new(),
            });
        }
    }

    report.missing_required_dimensions.sort();
    report.violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.dimension.cmp(&right.dimension))
            .then_with(|| left.scenario_ids.cmp(&right.scenario_ids))
    });
    report.violations.dedup();
    report.complete = !scenarios.is_empty()
        && !required.is_empty()
        && report.violations.is_empty()
        && report.missing_required_dimensions.is_empty();
    report
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum JointAuditedSelectionViolationCode {
    RequiredCoverageIncomplete,
    CandidateScenarioSetMismatch,
    InvalidCandidateIdentity,
    NoRobustCandidate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointAuditedSelectionViolation {
    pub code: JointAuditedSelectionViolationCode,
    pub candidate_id: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct JointAuditedSelectionReport {
    /// True only if coverage is complete, all candidate receipts match the benchmark scenario set,
    /// candidate IDs are valid, and at least one robustly accepted candidate exists.
    pub decision_eligible: bool,
    pub coverage: JointScenarioCoverageReport,
    pub pareto: JointParetoReport,
    pub violations: Vec<JointAuditedSelectionViolation>,
}

/// Gate the joint Pareto decision on declared scenario-family coverage and identical candidate
/// scenario sets. This prevents ranking an easy-corpus candidate against a candidate tested against
/// harder/more numerous scenarios.
pub fn select_joint_recovery_frontier_with_coverage(
    alternatives: &[JointRecoveryEnvelopeAlternative],
    benchmark_scenarios: &[JointDisruptionScenario],
    required_dimensions: &[JointDisruptionDimension],
) -> JointAuditedSelectionReport {
    let coverage = audit_joint_scenario_coverage(benchmark_scenarios, required_dimensions);
    let mut result = JointAuditedSelectionReport {
        coverage,
        ..JointAuditedSelectionReport::default()
    };
    if !result.coverage.complete {
        result.violations.push(JointAuditedSelectionViolation {
            code: JointAuditedSelectionViolationCode::RequiredCoverageIncomplete,
            candidate_id: None,
        });
    }

    let expected_ids: BTreeSet<&str> = benchmark_scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect();
    for alternative in alternatives {
        let actual_ids: Vec<&str> = alternative
            .report
            .scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.as_str())
            .collect();
        let actual_set: BTreeSet<&str> = actual_ids.iter().copied().collect();
        if actual_ids.len() != expected_ids.len()
            || actual_set.len() != actual_ids.len()
            || actual_set != expected_ids
        {
            result
                .violations
                .push(JointAuditedSelectionViolation {
                    code: JointAuditedSelectionViolationCode::CandidateScenarioSetMismatch,
                    candidate_id: Some(alternative.candidate_id.clone()),
                });
        }
    }

    if !result.violations.is_empty() {
        sort_audited_selection_violations(&mut result.violations);
        return result;
    }

    result.pareto = pareto_joint_recovery_candidates(alternatives);
    if !result.pareto.violations.is_empty() {
        result.violations.push(JointAuditedSelectionViolation {
            code: JointAuditedSelectionViolationCode::InvalidCandidateIdentity,
            candidate_id: None,
        });
    }
    if result.pareto.frontier.is_empty() {
        result.violations.push(JointAuditedSelectionViolation {
            code: JointAuditedSelectionViolationCode::NoRobustCandidate,
            candidate_id: None,
        });
    }
    sort_audited_selection_violations(&mut result.violations);
    result.decision_eligible = result.violations.is_empty() && !result.pareto.frontier.is_empty();
    result
}

fn sort_audited_selection_violations(violations: &mut Vec<JointAuditedSelectionViolation>) {
    violations.sort_by(|left, right| {
        left.code
            .cmp(&right.code)
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });
    violations.dedup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Disruption, PlacementOccupancyExtension, VehicleOutage};
    use shipyard_space_time::{ExclusionZone, Footprint, Placement, Rect, Scenario, TimeRange, Transfer};

    fn reservation(
        id: &str,
        operation: &str,
        resource: &str,
        kind: &str,
        start: u64,
        end: u64,
    ) -> ResourceReservation {
        ResourceReservation {
            reservation_id: id.to_owned(),
            operation_id: operation.to_owned(),
            resource_id: resource.to_owned(),
            resource_kind: kind.to_owned(),
            interval: TimeRange {
                start_tick: start,
                end_tick: end,
            },
        }
    }

    fn outage(id: &str, resource: &str, start: u64, end: u64) -> ResourceOutage {
        ResourceOutage {
            outage_id: id.to_owned(),
            resource_id: resource.to_owned(),
            unavailable: TimeRange {
                start_tick: start,
                end_tick: end,
            },
        }
    }

    fn spatial_control() -> Scenario {
        Scenario {
            yard_bounds: Some(Rect {
                min_x: 0,
                min_y: 0,
                max_x: 10,
                max_y: 10,
            }),
            placements: vec![Placement {
                reservation_id: "p-a".to_owned(),
                block_id: "block-a".to_owned(),
                footprint: Footprint::AxisAlignedRect(Rect {
                    min_x: 0,
                    min_y: 0,
                    max_x: 2,
                    max_y: 2,
                }),
                occupied: TimeRange {
                    start_tick: 0,
                    end_tick: 10,
                },
            }],
            ..Scenario::default()
        }
    }

    fn no_spatial_disruption(tick: u64) -> Disruption {
        Disruption {
            observed_at_tick: tick,
            new_exclusion_zones: Vec::new(),
            vehicle_outages: Vec::new(),
            placement_occupancy_extensions: Vec::new(),
            explicitly_affected_reservation_ids: Vec::new(),
        }
    }

    fn spatial_transfer(id: &str, start: u64, end: u64, vehicle_id: &str) -> Transfer {
        Transfer {
            id: id.to_owned(),
            block_id: "block-a".to_owned(),
            corridor: Footprint::AxisAlignedRect(Rect {
                min_x: 8,
                min_y: 8,
                max_x: 9,
                max_y: 9,
            }),
            interval: TimeRange {
                start_tick: start,
                end_tick: end,
            },
            vehicle_id: vehicle_id.to_owned(),
            predecessor_move_ids: Vec::new(),
        }
    }

    fn transfer_binding() -> ResourceSpatialBinding {
        ResourceSpatialBinding {
            binding_id: "bind-move-a".to_owned(),
            resource_reservation_id: "tug-move-a".to_owned(),
            transfer_id: "move-a".to_owned(),
        }
    }

    #[test]
    fn empty_resource_schedule_fails_closed() {
        let empty = ResourceSchedule::default();
        let report = evaluate_resource_recovery(&empty, &empty, 0, &[]);
        assert!(!report.accepted);
        assert!(
            report.violations.iter().any(|violation| {
                violation.code == ResourceViolationCode::EmptyResourceSchedule
            })
        );
    }

    #[test]
    fn unary_resource_rejects_overlapping_reservations_but_allows_endpoint_touch() {
        let valid = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("r1", "weld-a", "station-1", "workstation", 0, 5),
                reservation("r2", "weld-b", "station-1", "workstation", 5, 9),
            ],
        };
        assert!(validate_schedule(&valid).is_empty());

        let invalid = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("r1", "weld-a", "station-1", "workstation", 0, 6),
                reservation("r2", "weld-b", "station-1", "workstation", 5, 9),
            ],
        };
        assert!(validate_schedule(&invalid).iter().any(|violation| {
            violation.code == ResourceViolationCode::ExclusiveResourceOverlap
        }));
    }

    #[test]
    fn resource_outage_recovers_future_operation_with_no_unaffected_churn() {
        let baseline = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("weld-a", "weld-a", "station-1", "workstation", 0, 5),
                reservation("weld-b", "weld-b", "station-1", "workstation", 5, 10),
                reservation("inspect-c", "inspect-c", "inspector-1", "inspector", 0, 10),
            ],
        };
        let candidate = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("weld-a", "weld-a", "station-1", "workstation", 0, 5),
                reservation("weld-b", "weld-b", "station-2", "workstation", 5, 10),
                reservation("inspect-c", "inspect-c", "inspector-1", "inspector", 0, 10),
            ],
        };
        let report = evaluate_resource_recovery(
            &baseline,
            &candidate,
            4,
            &[outage("station-1-out", "station-1", 5, 8)],
        );

        assert!(report.accepted, "{:?}", report.violations);
        assert!(!report.metrics.baseline_feasible_after_disruption);
        assert!(report.metrics.candidate_feasible_after_disruption);
        assert_eq!(report.affected_reservation_ids, vec!["weld-b"]);
        assert_eq!(report.metrics.changed_reservations, 1);
        assert_eq!(report.metrics.unaffected_reservation_churn, 0);
        assert_eq!(report.metrics.recovery_horizon_ticks, Some(6));
    }

    #[test]
    fn active_resource_operation_cannot_be_reassigned_mid_operation() {
        let baseline = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "move-a",
                "move-a",
                "crane-1",
                "gantry-crane",
                2,
                8,
            )],
        };
        let candidate = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "move-a",
                "move-a",
                "crane-2",
                "gantry-crane",
                2,
                8,
            )],
        };
        let report = evaluate_resource_recovery(&baseline, &candidate, 4, &[]);
        assert!(!report.accepted);
        assert!(report.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::ActiveReservationChanged
        }));
    }

    #[test]
    fn completed_resource_history_cannot_be_rewritten() {
        let baseline = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "weld-a",
                "weld-a",
                "station-1",
                "workstation",
                0,
                4,
            )],
        };
        let candidate = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "weld-a",
                "weld-a",
                "station-2",
                "workstation",
                0,
                4,
            )],
        };
        let report = evaluate_resource_recovery(&baseline, &candidate, 4, &[]);
        assert!(!report.accepted);
        assert!(report.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::CompletedReservationChanged
        }));
    }

    #[test]
    fn joint_robust_envelope_passes_only_when_same_candidate_survives_every_resource_scenario() {
        let spatial = spatial_control();
        let baseline_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("weld-a", "weld-a", "station-1", "workstation", 0, 5),
                reservation("weld-b", "weld-b", "station-1", "workstation", 5, 10),
                reservation("inspect-c", "inspect-c", "inspector-1", "inspector", 5, 10),
            ],
        };
        let candidate_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("weld-a", "weld-a", "station-1", "workstation", 0, 5),
                reservation("weld-b", "weld-b", "station-2", "workstation", 5, 10),
                reservation("inspect-c", "inspect-c", "inspector-2", "inspector", 5, 10),
            ],
        };
        let scenarios = vec![
            JointDisruptionScenario {
                scenario_id: "station-outage".to_owned(),
                spatial_disruption: no_spatial_disruption(4),
                resource_outages: vec![outage("station-1-out", "station-1", 5, 8)],
            },
            JointDisruptionScenario {
                scenario_id: "inspector-outage".to_owned(),
                spatial_disruption: no_spatial_disruption(4),
                resource_outages: vec![outage("inspector-1-out", "inspector-1", 5, 9)],
            },
        ];

        let report = evaluate_joint_recovery_envelope(
            &spatial,
            &spatial,
            &baseline_resources,
            &candidate_resources,
            &[],
            &[],
            &scenarios,
        );

        assert!(report.robustly_accepted);
        assert_eq!(report.metrics.total_scenarios, 2);
        assert_eq!(report.metrics.accepted_scenarios, 2);
        assert_eq!(
            report
                .scenarios
                .iter()
                .map(|item| item.scenario_id.as_str())
                .collect::<Vec<_>>(),
            vec!["inspector-outage", "station-outage"]
        );
    }

    #[test]
    fn joint_robust_envelope_rejects_candidate_that_only_handles_one_resource_outage() {
        let spatial = spatial_control();
        let baseline_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("weld-station", "weld-a", "station-1", "workstation", 5, 8),
                reservation("weld-crane", "weld-a", "crane-1", "gantry-crane", 5, 8),
            ],
        };
        let candidate_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("weld-station", "weld-a", "station-2", "workstation", 5, 8),
                reservation("weld-crane", "weld-a", "crane-1", "gantry-crane", 5, 8),
            ],
        };
        let scenarios = vec![
            JointDisruptionScenario {
                scenario_id: "station-down".to_owned(),
                spatial_disruption: no_spatial_disruption(4),
                resource_outages: vec![outage("station-down-event", "station-1", 5, 7)],
            },
            JointDisruptionScenario {
                scenario_id: "crane-down".to_owned(),
                spatial_disruption: no_spatial_disruption(4),
                resource_outages: vec![outage("crane-down-event", "crane-1", 5, 7)],
            },
        ];

        let report = evaluate_joint_recovery_envelope(
            &spatial,
            &spatial,
            &baseline_resources,
            &candidate_resources,
            &[],
            &[],
            &scenarios,
        );

        assert!(!report.robustly_accepted);
        assert_eq!(report.metrics.accepted_scenarios, 1);
        let failed = report
            .scenarios
            .iter()
            .find(|item| item.scenario_id == "crane-down")
            .unwrap();
        assert!(!failed.report.accepted);
        assert!(
            failed
                .report
                .resources
                .violations
                .iter()
                .any(|violation| { violation.code == ResourceViolationCode::ResourceUnavailable })
        );
    }

    #[test]
    fn joint_envelope_rejects_empty_and_duplicate_scenario_sets() {
        let spatial = spatial_control();
        let resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("r1", "op-a", "station-1", "workstation", 0, 5)],
        };
        let empty = evaluate_joint_recovery_envelope(
            &spatial,
            &spatial,
            &resources,
            &resources,
            &[],
            &[],
            &[],
        );
        assert!(!empty.robustly_accepted);
        assert!(
            empty.violations.iter().any(|violation| {
                violation.code == JointEnvelopeViolationCode::EmptyScenarioSet
            })
        );

        let duplicate = JointDisruptionScenario {
            scenario_id: "same".to_owned(),
            spatial_disruption: no_spatial_disruption(2),
            resource_outages: Vec::new(),
        };
        let report = evaluate_joint_recovery_envelope(
            &spatial,
            &spatial,
            &resources,
            &resources,
            &[],
            &[],
            &[duplicate.clone(), duplicate],
        );
        assert!(!report.robustly_accepted);
        assert!(report.scenarios.is_empty());
        assert!(report.violations.iter().any(|violation| {
            violation.code == JointEnvelopeViolationCode::DuplicateScenarioId
        }));
    }

    #[test]
    fn removed_future_resource_work_is_not_counted_as_candidate_feasible() {
        let baseline = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("required", "weld-a", "station-1", "workstation", 5, 10),
                reservation("stay", "inspect-b", "inspector-1", "inspector", 0, 10),
            ],
        };
        let candidate = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "stay",
                "inspect-b",
                "inspector-1",
                "inspector",
                0,
                10,
            )],
        };
        let report = evaluate_resource_recovery(&baseline, &candidate, 4, &[]);

        assert!(!report.accepted);
        assert!(!report.metrics.candidate_feasible_after_disruption);
    }

    #[test]
    fn finish_to_start_precedence_rejects_early_successor_and_accepts_valid_order() {
        let dependency = OperationPrecedence {
            constraint_id: "weld-before-paint".to_owned(),
            predecessor_operation_id: "weld-block-a".to_owned(),
            successor_operation_id: "paint-block-a".to_owned(),
            minimum_lag_ticks: 2,
        };
        let baseline = ResourceSchedule {
            precedence_constraints: vec![dependency.clone()],
            reservations: vec![
                reservation("weld", "weld-block-a", "station-1", "workstation", 0, 5),
                reservation("paint", "paint-block-a", "station-2", "workstation", 7, 10),
            ],
        };
        assert!(validate_schedule(&baseline).is_empty());

        let candidate = ResourceSchedule {
            precedence_constraints: vec![dependency],
            reservations: vec![
                reservation("weld", "weld-block-a", "station-1", "workstation", 0, 5),
                reservation("paint", "paint-block-a", "station-2", "workstation", 6, 10),
            ],
        };
        let report = evaluate_resource_recovery(&baseline, &candidate, 4, &[]);

        assert!(!report.accepted);
        assert!(!report.metrics.candidate_feasible_after_disruption);
        assert!(
            report.violations.iter().any(|violation| {
                violation.code == ResourceViolationCode::PrecedenceTimeConflict
            })
        );
    }

    #[test]
    fn precedence_cycles_and_unknown_operations_fail_closed() {
        let schedule = ResourceSchedule {
            precedence_constraints: vec![
                OperationPrecedence {
                    constraint_id: "a-before-b".to_owned(),
                    predecessor_operation_id: "op-a".to_owned(),
                    successor_operation_id: "op-b".to_owned(),
                    minimum_lag_ticks: 0,
                },
                OperationPrecedence {
                    constraint_id: "b-before-a".to_owned(),
                    predecessor_operation_id: "op-b".to_owned(),
                    successor_operation_id: "op-a".to_owned(),
                    minimum_lag_ticks: 0,
                },
                OperationPrecedence {
                    constraint_id: "c-before-missing".to_owned(),
                    predecessor_operation_id: "op-c".to_owned(),
                    successor_operation_id: "op-missing".to_owned(),
                    minimum_lag_ticks: 0,
                },
            ],
            reservations: vec![
                reservation("r-a", "op-a", "station-a", "workstation", 0, 2),
                reservation("r-b", "op-b", "station-b", "workstation", 3, 5),
                reservation("r-c", "op-c", "station-c", "workstation", 6, 8),
            ],
        };

        let violations = validate_schedule(&schedule);
        assert!(
            violations
                .iter()
                .any(|violation| { violation.code == ResourceViolationCode::PrecedenceCycle })
        );
        assert!(violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::UnknownSuccessorOperation
        }));
    }

    #[test]
    fn candidate_cannot_reclassify_a_known_physical_resource() {
        let baseline = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "inspect",
                "inspect",
                "resource-1",
                "inspector",
                5,
                8,
            )],
        };
        let candidate = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "inspect",
                "inspect",
                "resource-1",
                "gantry-crane",
                5,
                8,
            )],
        };
        let report = evaluate_resource_recovery(&baseline, &candidate, 4, &[]);

        assert!(!report.accepted);
        assert!(!report.metrics.candidate_feasible_after_disruption);
        assert!(report.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::ResourceKindChangedAcrossSchedules
        }));
    }

    #[test]
    fn candidate_cannot_delete_authoritative_operation_precedence() {
        let dependency = OperationPrecedence {
            constraint_id: "cut-before-inspection".to_owned(),
            predecessor_operation_id: "cut".to_owned(),
            successor_operation_id: "inspect".to_owned(),
            minimum_lag_ticks: 0,
        };
        let baseline = ResourceSchedule {
            precedence_constraints: vec![dependency],
            reservations: vec![
                reservation("cut-op", "cut", "station-1", "workstation", 0, 4),
                reservation("inspect-op", "inspect", "inspector-1", "inspector", 4, 8),
            ],
        };
        let candidate = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: baseline.reservations.clone(),
        };
        let report = evaluate_resource_recovery(&baseline, &candidate, 2, &[]);

        assert!(!report.accepted);
        assert!(!report.metrics.candidate_feasible_after_disruption);
        assert!(report.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::PrecedenceConstraintsChanged
        }));
    }

    #[test]
    fn joint_transfer_requires_a_covering_resource_binding() {
        let mut spatial = spatial_control();
        spatial
            .transfers
            .push(spatial_transfer("move-a", 4, 6, "tug-1"));
        let resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("tug-move-a", "move-a", "tug-1", "tug", 3, 7)],
        };
        let binding = transfer_binding();
        let make_input =
            |baseline_bindings: Vec<ResourceSpatialBinding>,
             candidate_bindings: Vec<ResourceSpatialBinding>| {
                JointRecoveryInput {
                    spatial: RecoveryInput {
                        baseline: spatial.clone(),
                        candidate: spatial.clone(),
                        disruption: no_spatial_disruption(2),
                    },
                    baseline_resources: resources.clone(),
                    candidate_resources: resources.clone(),
                    resource_outages: Vec::new(),
                    baseline_bindings,
                    candidate_bindings,
                }
            };

        let bound =
            evaluate_joint_recovery(&make_input(vec![binding.clone()], vec![binding.clone()]));
        assert!(bound.accepted, "{:?}", bound.resources.violations);

        let unbound = evaluate_joint_recovery(&make_input(Vec::new(), Vec::new()));
        assert!(!unbound.accepted);
        assert!(
            !unbound
                .resources
                .metrics
                .candidate_feasible_after_disruption
        );
        assert!(unbound.resources.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::TransferMissingResourceBinding
        }));
    }

    #[test]
    fn binding_cannot_attach_a_different_operation_to_a_transfer() {
        let mut spatial = spatial_control();
        spatial
            .transfers
            .push(spatial_transfer("move-a", 4, 6, "tug-1"));
        let baseline_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("tug-move-a", "move-a", "tug-1", "tug", 3, 7)],
        };
        let candidate_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "tug-move-a",
                "different-operation",
                "tug-1",
                "tug",
                3,
                7,
            )],
        };
        let binding = transfer_binding();
        let report = evaluate_joint_recovery(&JointRecoveryInput {
            spatial: RecoveryInput {
                baseline: spatial.clone(),
                candidate: spatial,
                disruption: no_spatial_disruption(2),
            },
            baseline_resources,
            candidate_resources,
            resource_outages: Vec::new(),
            baseline_bindings: vec![binding.clone()],
            candidate_bindings: vec![binding],
        });

        assert!(!report.accepted);
        assert!(report.resources.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::ResourceOperationTransferMismatch
        }));
    }

    #[test]
    fn bound_resource_reservation_must_cover_entire_transfer_window() {
        let mut spatial = spatial_control();
        spatial
            .transfers
            .push(spatial_transfer("move-a", 4, 6, "tug-1"));
        let baseline_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("tug-move-a", "move-a", "tug-1", "tug", 3, 7)],
        };
        let candidate_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("tug-move-a", "move-a", "tug-1", "tug", 3, 5)],
        };
        let binding = transfer_binding();
        let report = evaluate_joint_recovery(&JointRecoveryInput {
            spatial: RecoveryInput {
                baseline: spatial.clone(),
                candidate: spatial,
                disruption: no_spatial_disruption(2),
            },
            baseline_resources,
            candidate_resources,
            resource_outages: Vec::new(),
            baseline_bindings: vec![binding.clone()],
            candidate_bindings: vec![binding],
        });

        assert!(!report.accepted);
        assert!(!report.resources.metrics.candidate_feasible_after_disruption);
        assert!(report.resources.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::ResourceReservationDoesNotCoverTransfer
        }));
    }

    #[test]
    fn bound_tug_identity_must_match_the_transfer_vehicle() {
        let mut spatial = spatial_control();
        spatial
            .transfers
            .push(spatial_transfer("move-a", 4, 6, "tug-1"));
        let baseline_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("tug-move-a", "move-a", "tug-1", "tug", 3, 7)],
        };
        let candidate_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("tug-move-a", "move-a", "tug-2", "tug", 3, 7)],
        };
        let binding = transfer_binding();
        let report = evaluate_joint_recovery(&JointRecoveryInput {
            spatial: RecoveryInput {
                baseline: spatial.clone(),
                candidate: spatial,
                disruption: no_spatial_disruption(2),
            },
            baseline_resources,
            candidate_resources,
            resource_outages: Vec::new(),
            baseline_bindings: vec![binding.clone()],
            candidate_bindings: vec![binding],
        });

        assert!(!report.accepted);
        assert!(report.resources.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::TransferVehicleResourceMismatch
        }));
    }

    #[test]
    fn future_resource_reservation_cannot_be_silently_dropped() {
        let baseline = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("required", "weld-a", "station-1", "workstation", 5, 10),
                reservation("stay", "inspect-b", "inspector-1", "inspector", 0, 10),
            ],
        };
        let candidate = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "stay",
                "inspect-b",
                "inspector-1",
                "inspector",
                0,
                10,
            )],
        };
        let report = evaluate_resource_recovery(&baseline, &candidate, 4, &[]);

        assert!(!report.accepted);
        assert!(report.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::RequiredReservationRemoved
        }));
    }

    #[test]
    fn unknown_resource_outage_target_fails_closed() {
        let schedule = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("r1", "op-a", "station-1", "workstation", 0, 2)],
        };
        let report = evaluate_resource_recovery(
            &schedule,
            &schedule,
            1,
            &[outage("bad-outage", "station-missing", 1, 2)],
        );
        assert!(!report.accepted);
        assert!(
            report.violations.iter().any(|violation| {
                violation.code == ResourceViolationCode::UnknownOutageResource
            })
        );
    }

    #[test]
    fn joint_acceptance_requires_both_spatial_and_resource_feasibility() {
        let spatial_yard = Rect {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 10,
        };
        let spatial = RecoveryInput {
            baseline: Scenario {
                yard_bounds: Some(spatial_yard),
                placements: vec![Placement {
                    reservation_id: "p-a".to_owned(),
                    block_id: "block-a".to_owned(),
                    footprint: Footprint::AxisAlignedRect(Rect {
                        min_x: 0,
                        min_y: 0,
                        max_x: 2,
                        max_y: 2,
                    }),
                    occupied: TimeRange {
                        start_tick: 0,
                        end_tick: 10,
                    },
                }],
                ..Scenario::default()
            },
            candidate: Scenario {
                yard_bounds: Some(spatial_yard),
                placements: vec![Placement {
                    reservation_id: "p-a".to_owned(),
                    block_id: "block-a".to_owned(),
                    footprint: Footprint::AxisAlignedRect(Rect {
                        min_x: 0,
                        min_y: 0,
                        max_x: 2,
                        max_y: 2,
                    }),
                    occupied: TimeRange {
                        start_tick: 0,
                        end_tick: 10,
                    },
                }],
                ..Scenario::default()
            },
            disruption: Disruption {
                observed_at_tick: 0,
                new_exclusion_zones: Vec::new(),
                vehicle_outages: Vec::new(),
                placement_occupancy_extensions: Vec::new(),
                explicitly_affected_reservation_ids: Vec::new(),
            },
        };
        let baseline_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("op-a", "op-a", "crane-1", "gantry-crane", 0, 5),
                reservation("op-b", "op-b", "crane-1", "gantry-crane", 5, 10),
            ],
        };
        let candidate_resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![
                reservation("op-a", "op-a", "crane-1", "gantry-crane", 0, 5),
                reservation("op-b", "op-b", "crane-1", "gantry-crane", 4, 10),
            ],
        };
        let report = evaluate_joint_recovery(&JointRecoveryInput {
            spatial,
            baseline_resources,
            candidate_resources,
            resource_outages: Vec::new(),
            baseline_bindings: Vec::new(),
            candidate_bindings: Vec::new(),
        });
        assert!(report.spatial.accepted);
        assert!(!report.resources.accepted);
        assert!(!report.accepted);
    }
    #[test]
    fn joint_pareto_frontier_preserves_spatial_resource_tradeoffs() {
        #[derive(Clone, Copy)]
        struct ScoreSpec {
            spatial_horizon: u64,
            resource_horizon: u64,
            changed_placements: usize,
            spatial_shift: u64,
            rectangle_edge_shift: u64,
            changed_resources: usize,
            resource_shift: u64,
        }

        fn score(spec: ScoreSpec) -> JointParetoScore {
            JointParetoScore {
                worst_spatial_recovery_horizon_ticks: Some(spec.spatial_horizon),
                worst_resource_recovery_horizon_ticks: Some(spec.resource_horizon),
                worst_changed_placement_reservations: spec.changed_placements,
                worst_changed_transfer_reservations: 0,
                worst_unaffected_placement_churn: 0,
                worst_unaffected_transfer_churn: 0,
                worst_spatial_time_shift_ticks: spec.spatial_shift,
                worst_rectangle_edge_shift_units: spec.rectangle_edge_shift,
                worst_changed_resource_reservations: spec.changed_resources,
                worst_unaffected_resource_churn: 0,
                worst_resource_time_shift_ticks: spec.resource_shift,
            }
        }

        fn alternative(
            id: &str,
            robustly_accepted: bool,
            score: JointParetoScore,
        ) -> JointRecoveryEnvelopeAlternative {
            JointRecoveryEnvelopeAlternative {
                candidate_id: id.to_owned(),
                report: JointRecoveryEnvelopeReport {
                    robustly_accepted,
                    metrics: JointRecoveryEnvelopeMetrics {
                        worst_spatial_recovery_horizon_ticks: score
                            .worst_spatial_recovery_horizon_ticks,
                        worst_resource_recovery_horizon_ticks: score
                            .worst_resource_recovery_horizon_ticks,
                        worst_changed_placement_reservations: score
                            .worst_changed_placement_reservations,
                        worst_changed_transfer_reservations: score
                            .worst_changed_transfer_reservations,
                        worst_unaffected_placement_churn: score
                            .worst_unaffected_placement_churn,
                        worst_unaffected_transfer_churn: score
                            .worst_unaffected_transfer_churn,
                        worst_spatial_time_shift_ticks: score.worst_spatial_time_shift_ticks,
                        worst_rectangle_edge_shift_units: score.worst_rectangle_edge_shift_units,
                        worst_changed_resource_reservations: score
                            .worst_changed_resource_reservations,
                        worst_unaffected_resource_churn: score.worst_unaffected_resource_churn,
                        worst_resource_time_shift_ticks: score.worst_resource_time_shift_ticks,
                        ..JointRecoveryEnvelopeMetrics::default()
                    },
                    violations: Vec::new(),
                    scenarios: Vec::new(),
                },
            }
        }

        let alternatives = vec![
            alternative("fast-far", true, score(ScoreSpec { spatial_horizon: 5, resource_horizon: 8, changed_placements: 2, spatial_shift: 5, rectangle_edge_shift: 10, changed_resources: 1, resource_shift: 3 })),
            alternative("slow-near", true, score(ScoreSpec { spatial_horizon: 7, resource_horizon: 6, changed_placements: 1, spatial_shift: 6, rectangle_edge_shift: 2, changed_resources: 1, resource_shift: 2 })),
            alternative(
                "unnecessary-churn",
                true,
                score(ScoreSpec { spatial_horizon: 6, resource_horizon: 10, changed_placements: 3, spatial_shift: 7, rectangle_edge_shift: 12, changed_resources: 2, resource_shift: 5 }),
            ),
            alternative("fragile", false, score(ScoreSpec { spatial_horizon: 3, resource_horizon: 3, changed_placements: 0, spatial_shift: 0, rectangle_edge_shift: 0, changed_resources: 0, resource_shift: 0 })),
        ];

        let report = pareto_joint_recovery_candidates(&alternatives);

        assert_eq!(
            report.frontier.iter().map(|candidate| candidate.candidate_id.as_str()).collect::<Vec<_>>(),
            vec!["fast-far", "slow-near"]
        );
        assert_eq!(report.dominated_candidate_ids, vec!["unnecessary-churn"]);
        assert_eq!(report.rejected_candidate_ids, vec!["fragile"]);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn joint_pareto_requires_unique_candidate_ids_and_compares_missing_horizons_safely() {
        let score = JointParetoScore {
            worst_spatial_recovery_horizon_ticks: None,
            worst_resource_recovery_horizon_ticks: None,
            worst_changed_placement_reservations: 0,
            worst_changed_transfer_reservations: 0,
            worst_unaffected_placement_churn: 0,
            worst_unaffected_transfer_churn: 0,
            worst_spatial_time_shift_ticks: 0,
            worst_rectangle_edge_shift_units: 0,
            worst_changed_resource_reservations: 0,
            worst_unaffected_resource_churn: 0,
            worst_resource_time_shift_ticks: 0,
        };
        let measured = JointParetoScore {
            worst_spatial_recovery_horizon_ticks: Some(1),
            worst_resource_recovery_horizon_ticks: Some(1),
            ..score.clone()
        };
        assert!(!joint_pareto_dominates(&score, &measured));
        assert!(!joint_pareto_dominates(&measured, &score));

        let envelope = |id: &str| JointRecoveryEnvelopeAlternative {
            candidate_id: id.to_owned(),
            report: JointRecoveryEnvelopeReport {
                robustly_accepted: true,
                metrics: JointRecoveryEnvelopeMetrics::default(),
                violations: Vec::new(),
                scenarios: Vec::new(),
            },
        };
        let report = pareto_joint_recovery_candidates(&[envelope("duplicate"), envelope("duplicate")]);
        assert!(report.frontier.is_empty());
        assert!(report.violations.iter().any(|violation| {
            violation.code == JointParetoViolationCode::DuplicateCandidateId
        }));
    }


    #[test]
    fn coverage_audit_receipts_every_required_disruption_dimension() {
        let clearance = JointDisruptionScenario {
            scenario_id: "clearance-overrun".to_owned(),
            spatial_disruption: Disruption {
                placement_occupancy_extensions: vec![PlacementOccupancyExtension {
                    reservation_id: "p-a".to_owned(),
                    occupied_until_tick: 12,
                }],
                ..no_spatial_disruption(4)
            },
            resource_outages: Vec::new(),
        };
        let vehicle = JointDisruptionScenario {
            scenario_id: "vehicle-outage".to_owned(),
            spatial_disruption: Disruption {
                vehicle_outages: vec![VehicleOutage {
                    vehicle_id: "tug-1".to_owned(),
                    unavailable: TimeRange {
                        start_tick: 5,
                        end_tick: 8,
                    },
                }],
                ..no_spatial_disruption(4)
            },
            resource_outages: Vec::new(),
        };
        let zone = JointDisruptionScenario {
            scenario_id: "zone-closure".to_owned(),
            spatial_disruption: Disruption {
                new_exclusion_zones: vec![ExclusionZone {
                    id: "zone-a".to_owned(),
                    footprint: Footprint::AxisAlignedRect(Rect {
                        min_x: 3,
                        min_y: 3,
                        max_x: 4,
                        max_y: 4,
                    }),
                    active: TimeRange {
                        start_tick: 5,
                        end_tick: 8,
                    },
                }],
                ..no_spatial_disruption(4)
            },
            resource_outages: Vec::new(),
        };
        let resource = JointDisruptionScenario {
            scenario_id: "resource-outage".to_owned(),
            spatial_disruption: no_spatial_disruption(4),
            resource_outages: vec![outage("crane-down", "crane-1", 5, 8)],
        };
        let cross_layer = JointDisruptionScenario {
            scenario_id: "zone-plus-resource".to_owned(),
            spatial_disruption: zone.spatial_disruption.clone(),
            resource_outages: vec![outage("crane-down-2", "crane-1", 5, 8)],
        };
        let scenarios = vec![clearance, vehicle, zone, resource, cross_layer];
        let required = [
            JointDisruptionDimension::ClearanceOverrun,
            JointDisruptionDimension::VehicleOutage,
            JointDisruptionDimension::ExclusionZoneClosure,
            JointDisruptionDimension::ResourceOutage,
            JointDisruptionDimension::CrossLayerCombination,
            JointDisruptionDimension::MultiDimensionCombination,
        ];

        let report = audit_joint_scenario_coverage(&scenarios, &required);

        assert!(report.complete);
        assert_eq!(report.scenario_count, 5);
        assert!(report.missing_required_dimensions.is_empty());
        assert!(report.violations.is_empty());
        for dimension in required {
            assert!(report.receipts.iter().any(|receipt| receipt.dimension == dimension));
        }
        let cross = report
            .receipts
            .iter()
            .find(|receipt| receipt.dimension == JointDisruptionDimension::CrossLayerCombination)
            .unwrap();
        assert_eq!(cross.scenario_ids, vec!["zone-plus-resource"]);
    }

    #[test]
    fn coverage_audit_reports_missing_families_even_if_scenario_count_is_nonzero() {
        let scenarios = [JointDisruptionScenario {
            scenario_id: "only-resource".to_owned(),
            spatial_disruption: no_spatial_disruption(4),
            resource_outages: vec![outage("station-down", "station-1", 5, 8)],
        }];
        let report = audit_joint_scenario_coverage(
            &scenarios,
            &[
                JointDisruptionDimension::ResourceOutage,
                JointDisruptionDimension::ClearanceOverrun,
                JointDisruptionDimension::VehicleOutage,
            ],
        );

        assert!(!report.complete);
        assert_eq!(
            report.missing_required_dimensions,
            vec![
                JointDisruptionDimension::ClearanceOverrun,
                JointDisruptionDimension::VehicleOutage,
            ]
        );
        assert_eq!(
            report
                .violations
                .iter()
                .filter(|violation| {
                    violation.code == JointCoverageViolationCode::MissingRequiredDimension
                })
                .count(),
            2
        );
    }

    #[test]
    fn coverage_audit_rejects_empty_and_duplicate_scenario_ids() {
        let empty_id = JointDisruptionScenario {
            scenario_id: String::new(),
            spatial_disruption: no_spatial_disruption(4),
            resource_outages: vec![outage("out-a", "crane-1", 5, 8)],
        };
        let duplicate_a = JointDisruptionScenario {
            scenario_id: "duplicate".to_owned(),
            spatial_disruption: no_spatial_disruption(4),
            resource_outages: vec![outage("out-b", "crane-1", 5, 8)],
        };
        let duplicate_b = JointDisruptionScenario {
            scenario_id: "duplicate".to_owned(),
            spatial_disruption: no_spatial_disruption(6),
            resource_outages: vec![outage("out-c", "tug-1", 6, 9)],
        };
        let scenarios = [empty_id, duplicate_a, duplicate_b];
        let report = audit_joint_scenario_coverage(
            &scenarios,
            &[JointDisruptionDimension::ResourceOutage],
        );

        assert!(!report.complete);
        assert!(report.violations.iter().any(|violation| {
            violation.code == JointCoverageViolationCode::EmptyScenarioId
        }));
        assert!(report.violations.iter().any(|violation| {
            violation.code == JointCoverageViolationCode::DuplicateScenarioId
                && violation.scenario_ids == vec!["duplicate".to_owned()]
        }));
        assert!(report.receipts.iter().all(|receipt| {
            !receipt.scenario_ids.iter().any(|scenario_id| scenario_id.trim().is_empty())
                && !receipt.scenario_ids.iter().any(|scenario_id| scenario_id == "duplicate")
        }));
    }

    #[test]
    fn coverage_audit_rejects_empty_or_duplicated_requirements_and_is_order_invariant() {
        let scenarios = [
            JointDisruptionScenario {
                scenario_id: "resource-a".to_owned(),
                spatial_disruption: no_spatial_disruption(4),
                resource_outages: vec![outage("out-a", "crane-1", 5, 8)],
            },
            JointDisruptionScenario {
                scenario_id: "resource-b".to_owned(),
                spatial_disruption: no_spatial_disruption(6),
                resource_outages: vec![outage("out-b", "tug-1", 6, 9)],
            },
        ];
        let empty_required = audit_joint_scenario_coverage(&scenarios, &[]);
        assert!(!empty_required.complete);
        assert!(empty_required.violations.iter().any(|violation| {
            violation.code == JointCoverageViolationCode::EmptyRequiredDimensionSet
        }));

        let duplicated_required = audit_joint_scenario_coverage(
            &scenarios,
            &[
                JointDisruptionDimension::ResourceOutage,
                JointDisruptionDimension::ResourceOutage,
            ],
        );
        assert!(!duplicated_required.complete);
        assert!(duplicated_required.violations.iter().any(|violation| {
            violation.code == JointCoverageViolationCode::DuplicateRequiredDimension
        }));

        let forward = audit_joint_scenario_coverage(
            &scenarios,
            &[JointDisruptionDimension::ResourceOutage],
        );
        let reversed = audit_joint_scenario_coverage(
            &[scenarios[1].clone(), scenarios[0].clone()],
            &[JointDisruptionDimension::ResourceOutage],
        );
        assert_eq!(forward, reversed);
    }


    fn resource_coverage_fixture(id: &str) -> JointDisruptionScenario {
        JointDisruptionScenario {
            scenario_id: id.to_owned(),
            spatial_disruption: no_spatial_disruption(1),
            resource_outages: vec![outage("station-down", "station-1", 6, 8)],
        }
    }

    fn resource_coverage_alternative(
        candidate_id: &str,
        scenario: JointDisruptionScenario,
    ) -> JointRecoveryEnvelopeAlternative {
        let spatial = spatial_control();
        let resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation(
                "station-work",
                "weld-a",
                "station-1",
                "workstation",
                0,
                5,
            )],
        };
        let report = evaluate_joint_recovery_envelope(
            &spatial,
            &spatial,
            &resources,
            &resources,
            &[],
            &[],
            &[scenario],
        );
        JointRecoveryEnvelopeAlternative {
            candidate_id: candidate_id.to_owned(),
            report,
        }
    }

    #[test]
    fn audited_selector_allows_only_candidates_evaluated_on_the_required_corpus() {
        let scenario = resource_coverage_fixture("resource-outage-a");
        let alternative =
            resource_coverage_alternative("candidate-a", scenario.clone());
        assert!(alternative.report.robustly_accepted);

        let report = select_joint_recovery_frontier_with_coverage(
            &[alternative],
            &[scenario],
            &[JointDisruptionDimension::ResourceOutage],
        );

        assert!(report.decision_eligible);
        assert!(report.coverage.complete);
        assert_eq!(report.pareto.frontier.len(), 1);
        assert!(report.violations.is_empty());
    }

    #[test]
    fn audited_selector_blocks_incomplete_coverage_even_if_every_supplied_case_passes() {
        let scenario = resource_coverage_fixture("resource-outage-a");
        let alternative =
            resource_coverage_alternative("candidate-a", scenario.clone());
        assert!(alternative.report.robustly_accepted);

        let report = select_joint_recovery_frontier_with_coverage(
            &[alternative],
            &[scenario],
            &[
                JointDisruptionDimension::ResourceOutage,
                JointDisruptionDimension::ClearanceOverrun,
            ],
        );

        assert!(!report.decision_eligible);
        assert!(!report.coverage.complete);
        assert!(report.pareto.frontier.is_empty());
        assert!(report.violations.iter().any(|violation| {
            violation.code == JointAuditedSelectionViolationCode::RequiredCoverageIncomplete
        }));
    }

    #[test]
    fn audited_selector_rejects_candidate_receipt_from_an_easier_or_different_scenario_set() {
        let benchmark_scenario = resource_coverage_fixture("required-resource-outage");
        let easier_candidate = resource_coverage_alternative(
            "candidate-a",
            resource_coverage_fixture("different-resource-outage"),
        );
        assert!(easier_candidate.report.robustly_accepted);

        let report = select_joint_recovery_frontier_with_coverage(
            &[easier_candidate],
            &[benchmark_scenario],
            &[JointDisruptionDimension::ResourceOutage],
        );

        assert!(!report.decision_eligible);
        assert!(report.pareto.frontier.is_empty());
        assert!(report.violations.iter().any(|violation| {
            violation.code == JointAuditedSelectionViolationCode::CandidateScenarioSetMismatch
                && violation.candidate_id.as_deref() == Some("candidate-a")
        }));
    }


}
