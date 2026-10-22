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
                vec![constraint.constraint_id.clone(), constraint.predecessor_operation_id.clone()],
                "an operation cannot precede itself",
            );
        }

        let predecessor = windows.get(&constraint.predecessor_operation_id);
        let successor = windows.get(&constraint.successor_operation_id);
        if predecessor.is_none() && !constraint.predecessor_operation_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::UnknownPredecessorOperation,
                vec![constraint.constraint_id.clone(), constraint.predecessor_operation_id.clone()],
                "precedence predecessor is absent from the resource schedule",
            );
        }
        if successor.is_none() && !constraint.successor_operation_id.trim().is_empty() {
            push(
                &mut violations,
                ResourceViolationCode::UnknownSuccessorOperation,
                vec![constraint.constraint_id.clone(), constraint.successor_operation_id.clone()],
                "precedence successor is absent from the resource schedule",
            );
        }

        if let (Some((_, predecessor_end)), Some((successor_start, _))) =
            (predecessor, successor)
        {
            let earliest_successor_start = predecessor_end.checked_add(constraint.minimum_lag_ticks);
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
                edges.entry(edge.0.clone()).or_default().insert(edge.1.clone());
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
    let precedence_unchanged =
        normalized_precedence(baseline) == normalized_precedence(candidate);
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
    let baseline_feasible_after_disruption =
        baseline_feasible_before_disruption && outages_valid && baseline_outage_conflicts.is_empty();
    let mut candidate_feasible_after_disruption = precedence_unchanged
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
                recovery_completion_tick
                    .map_or(reservation.interval.end_tick, |prior: u64| {
                        prior.max(reservation.interval.end_tick)
                    }),
            );
        }
    }
    let recovery_horizon_ticks = recovery_completion_tick
        .map(|tick| tick.saturating_sub(observed_at_tick));

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointRecoveryInput {
    pub spatial: RecoveryInput,
    pub baseline_resources: ResourceSchedule,
    pub candidate_resources: ResourceSchedule,
    pub resource_outages: Vec<ResourceOutage>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointRecoveryReport {
    /// Candidate is accepted only when both the space-time and unary-resource evaluators accept.
    pub accepted: bool,
    pub spatial: RecoveryReport,
    pub resources: ResourceRecoveryReport,
}

/// Compose independent spatial and resource checks without allowing either to override the other.
pub fn evaluate_joint_recovery(input: &JointRecoveryInput) -> JointRecoveryReport {
    let spatial = evaluate_recovery(&input.spatial);
    let resources = evaluate_resource_recovery(
        &input.baseline_resources,
        &input.candidate_resources,
        input.spatial.disruption.observed_at_tick,
        &input.resource_outages,
    );
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
            worst_unaffected_resource_churn,
            worst_resource_time_shift_ticks,
        },
        violations,
        scenarios: reports,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shipyard_space_time::{Footprint, Placement, Rect, Scenario, TimeRange};
    use crate::Disruption;

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
            interval: TimeRange { start_tick: start, end_tick: end },
        }
    }

    fn outage(id: &str, resource: &str, start: u64, end: u64) -> ResourceOutage {
        ResourceOutage {
            outage_id: id.to_owned(),
            resource_id: resource.to_owned(),
            unavailable: TimeRange { start_tick: start, end_tick: end },
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

    #[test]
    fn empty_resource_schedule_fails_closed() {
        let empty = ResourceSchedule::default();
        let report = evaluate_resource_recovery(&empty, &empty, 0, &[]);
        assert!(!report.accepted);
        assert!(report.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::EmptyResourceSchedule
        }));
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
            reservations: vec![reservation("move-a", "move-a", "crane-1", "gantry-crane", 2, 8)],
        };
        let candidate = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("move-a", "move-a", "crane-2", "gantry-crane", 2, 8)],
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
            reservations: vec![reservation("weld-a", "weld-a", "station-1", "workstation", 0, 4)],
        };
        let candidate = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![reservation("weld-a", "weld-a", "station-2", "workstation", 0, 4)],
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
            &scenarios,
        );

        assert!(report.robustly_accepted);
        assert_eq!(report.metrics.total_scenarios, 2);
        assert_eq!(report.metrics.accepted_scenarios, 2);
        assert_eq!(
            report.scenarios.iter().map(|item| item.scenario_id.as_str()).collect::<Vec<_>>(),
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
        assert!(failed.report.resources.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::ResourceUnavailable
        }));
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
        );
        assert!(!empty.robustly_accepted);
        assert!(empty.violations.iter().any(|violation| {
            violation.code == JointEnvelopeViolationCode::EmptyScenarioSet
        }));

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
            reservations: vec![reservation("stay", "inspect-b", "inspector-1", "inspector", 0, 10)],
        };
        let report = evaluate_resource_recovery(&baseline, &candidate, 4, &[]);

        assert!(!report.accepted);
        assert!(!report.metrics.candidate_feasible_after_disruption);
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
            reservations: vec![reservation("stay", "inspect-b", "inspector-1", "inspector", 0, 10)],
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
        assert!(report.violations.iter().any(|violation| {
            violation.code == ResourceViolationCode::UnknownOutageResource
        }));
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
        });
        assert!(report.spatial.accepted);
        assert!(!report.resources.accepted);
        assert!(!report.accepted);
    }
}
