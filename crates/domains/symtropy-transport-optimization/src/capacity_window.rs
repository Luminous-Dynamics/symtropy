// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic schedule-aware capacity qualification.
//!
//! This layer evaluates proposed operation windows against versioned capacity
//! evidence and prevents overlapping assignments from double-spending the same
//! physical resource. All times are Unix epoch seconds under an explicit time
//! profile. Windows are half-open: [start, end). A capacity limit is a planning
//! claim, not an accepted reservation, live dispatch authorization, or proof of
//! physical truth. Issuer authentication and source-digest verification belong
//! to the caller's evidence-verification boundary.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{
    CargoTransferCapacityEvidenceStatus, CargoTransferCapacityEvidenceV1,
    CargoTransferCapacityResourceKind, QuantityUnit,
};

/// An operation interval with an explicit, non-inferred clock/time profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationWindowV1 {
    pub time_profile_id: String,
    pub planned_start_epoch_seconds: u64,
    pub planned_end_epoch_seconds: u64,
}

impl OperationWindowV1 {
    fn validate(&self) -> Result<(), ScheduledCapacityPlanError> {
        if self.time_profile_id.trim().is_empty()
            || self.planned_start_epoch_seconds >= self.planned_end_epoch_seconds
        {
            return Err(ScheduledCapacityPlanError::InvalidOperationWindow);
        }
        Ok(())
    }

    fn overlaps(&self, other: &Self) -> bool {
        self.planned_start_epoch_seconds < other.planned_end_epoch_seconds
            && other.planned_start_epoch_seconds < self.planned_end_epoch_seconds
    }
}

/// Policy is explicit input: the validator does not read a system clock or
/// infer freshness limits from a transport mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledCapacityPolicyV1 {
    pub policy_id: String,
    pub expected_time_profile_id: String,
    pub max_observation_age_seconds: u64,
    pub allowed_future_clock_skew_seconds: u64,
}

impl ScheduledCapacityPolicyV1 {
    fn validate(&self) -> Result<(), ScheduledCapacityPlanError> {
        if self.policy_id.trim().is_empty() || self.expected_time_profile_id.trim().is_empty() {
            return Err(ScheduledCapacityPlanError::InvalidPolicy);
        }
        Ok(())
    }
}

/// Generic capacity evidence for a physical resource. Existing equipment
/// evidence can be converted with From; edge-capacity evidence uses the same
/// envelope with resource_kind = TransferEdge and resource_id = the edge ID.
///
/// A source digest is preserved, not recomputed here. A nonempty authority ID
/// is provenance, not proof that the source was authorized to make the claim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledCapacityEvidenceV1 {
    pub budget_id: String,
    pub resource_kind: CargoTransferCapacityResourceKind,
    pub resource_id: String,
    pub terminal_id: Option<String>,
    pub capacity_scope_id: String,
    pub capacity_units: u64,
    pub capacity_unit: QuantityUnit,
    pub authority_id: String,
    pub authority_profile_id: String,
    pub source_record_id: String,
    pub source_schema_profile_id: String,
    pub source_record_digest: String,
    pub revision: u64,
    pub supersedes_revision: Option<u64>,
    /// Preserved for lineage only. It is never substituted for observation time.
    pub source_event_time_epoch_seconds: Option<u64>,
    pub capacity_observed_at_epoch_seconds: Option<u64>,
    /// Preserved bookkeeping time. It is not used as the observation time.
    pub repository_recorded_at_epoch_seconds: Option<u64>,
    pub valid_from_epoch_seconds: Option<u64>,
    /// Exclusive endpoint of the capacity-evidence validity interval.
    pub valid_until_epoch_seconds: Option<u64>,
    pub time_profile_id: String,
    pub status: CargoTransferCapacityEvidenceStatus,
}

impl From<&CargoTransferCapacityEvidenceV1> for ScheduledCapacityEvidenceV1 {
    fn from(evidence: &CargoTransferCapacityEvidenceV1) -> Self {
        Self {
            budget_id: evidence.budget_id.clone(),
            resource_kind: CargoTransferCapacityResourceKind::Equipment,
            resource_id: evidence.budget.equipment_id.clone(),
            terminal_id: Some(evidence.budget.terminal_id.clone()),
            capacity_scope_id: evidence.budget.capacity_scope_id.clone(),
            capacity_units: evidence.budget.capacity_units,
            capacity_unit: evidence.budget.capacity_unit,
            authority_id: evidence.authority_id.clone(),
            authority_profile_id: evidence.authority_profile_id.clone(),
            source_record_id: evidence.source_record_id.clone(),
            source_schema_profile_id: evidence.source_schema_profile_id.clone(),
            source_record_digest: evidence.source_record_digest.clone(),
            revision: evidence.revision,
            supersedes_revision: evidence.supersedes_revision,
            source_event_time_epoch_seconds: evidence.source_event_time_epoch_seconds,
            capacity_observed_at_epoch_seconds: evidence.capacity_observed_at_epoch_seconds,
            repository_recorded_at_epoch_seconds: evidence.repository_recorded_at_epoch_seconds,
            valid_from_epoch_seconds: evidence.valid_from_epoch_seconds,
            valid_until_epoch_seconds: evidence.valid_until_epoch_seconds,
            time_profile_id: evidence.time_profile_id.clone(),
            status: evidence.status,
        }
    }
}

/// One proposed use of a capacity-bearing resource during an operation window.
/// evidence_id identifies the exact evidence record; capacity_scope_id remains
/// an opaque source/accounting identifier, never a timestamp encoding.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledCapacityAssignmentV1 {
    pub assignment_id: String,
    pub evidence_id: String,
    pub resource_kind: CargoTransferCapacityResourceKind,
    pub resource_id: String,
    pub capacity_scope_id: String,
    pub capacity_units: u64,
    pub capacity_unit: QuantityUnit,
    pub terminal_id: Option<String>,
    pub window: OperationWindowV1,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduledCapacityPlanError {
    InvalidPolicy,
    InvalidIdentityOrProvenance,
    InvalidRevisionLineage,
    TimeProfileMismatch,
    MissingObservationTime,
    MissingRepositoryRecordTime,
    MissingValidityInterval,
    InvalidValidityInterval,
    TimestampInFuture,
    ObservationTooOld,
    ObservationTooOldAtOperationStart,
    NotCurrent,
    InvalidOperationWindow,
    OperationWindowAlreadyStarted,
    WindowNotCoveredByEvidence,
    DuplicateEvidenceIdentity,
    AmbiguousCapacityEvidence,
    DuplicateAssignmentIdentity,
    UnknownEvidence,
    ResourceBindingMismatch,
    CapacityUnitMismatch,
    InvalidCapacity,
    CapacityExceeded,
    CapacityArithmeticOverflow,
    UnreferencedEvidence,
}

impl fmt::Display for ScheduledCapacityPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidPolicy => "capacity policy identity or time profile is incomplete",
            Self::InvalidIdentityOrProvenance => {
                "capacity evidence identity or provenance is incomplete"
            },
            Self::InvalidRevisionLineage => "capacity evidence revision lineage is invalid",
            Self::TimeProfileMismatch => {
                "capacity evidence, operation window, and policy time profiles must match"
            },
            Self::MissingObservationTime => "capacity evidence has no explicit observation time",
            Self::MissingRepositoryRecordTime => "capacity evidence has no repository record time",
            Self::MissingValidityInterval => "capacity evidence has no explicit validity interval",
            Self::InvalidValidityInterval => {
                "capacity evidence validity interval is empty or reversed"
            },
            Self::TimestampInFuture => {
                "capacity evidence timestamp exceeds allowed future clock skew"
            },
            Self::ObservationTooOld => {
                "capacity observation exceeds the freshness limit at plan evaluation"
            },
            Self::ObservationTooOldAtOperationStart => {
                "capacity observation will be too old by planned operation start"
            },
            Self::NotCurrent => "capacity evidence is superseded, revoked, or conflicted",
            Self::InvalidOperationWindow => {
                "operation window has a blank time profile or empty/reversed interval"
            },
            Self::OperationWindowAlreadyStarted => {
                "operation window starts before the plan evaluation instant"
            },
            Self::WindowNotCoveredByEvidence => {
                "capacity evidence validity does not contain the full operation window"
            },
            Self::DuplicateEvidenceIdentity => {
                "capacity evidence repeats an evidence ID or has incompatible lineage"
            },
            Self::AmbiguousCapacityEvidence => {
                "overlapping current capacity evidence for the same physical resource is ambiguous"
            },
            Self::DuplicateAssignmentIdentity => "capacity plan repeats an assignment ID",
            Self::UnknownEvidence => "capacity assignment references unknown evidence",
            Self::ResourceBindingMismatch => {
                "capacity assignment does not match its referenced resource evidence"
            },
            Self::CapacityUnitMismatch => {
                "capacity assignments for one resource use incompatible units"
            },
            Self::InvalidCapacity => "capacity or assigned quantity must be positive",
            Self::CapacityExceeded => {
                "overlapping capacity assignments exceed the qualified resource limit"
            },
            Self::CapacityArithmeticOverflow => "capacity accounting overflowed",
            Self::UnreferencedEvidence => {
                "capacity evidence is not referenced by a proposed assignment"
            },
        };
        f.write_str(message)
    }
}

impl std::error::Error for ScheduledCapacityPlanError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledCapacityAssignmentReceiptV1 {
    pub assignment_id: String,
    pub evidence_id: String,
    pub resource_kind: CargoTransferCapacityResourceKind,
    pub resource_id: String,
    pub terminal_id: Option<String>,
    pub capacity_scope_id: String,
    pub capacity_units: u64,
    pub capacity_unit: QuantityUnit,
    pub window: OperationWindowV1,
    pub authority_id: String,
    pub authority_profile_id: String,
    pub source_record_id: String,
    pub source_schema_profile_id: String,
    pub source_record_digest: String,
    pub source_event_time_epoch_seconds: Option<u64>,
    pub evidence_revision: u64,
    pub supersedes_revision: Option<u64>,
    pub capacity_observed_at_epoch_seconds: u64,
    pub repository_recorded_at_epoch_seconds: u64,
    pub evidence_valid_from_epoch_seconds: u64,
    pub evidence_valid_until_epoch_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledCapacityResourceSummaryV1 {
    pub evidence_id: String,
    pub resource_kind: CargoTransferCapacityResourceKind,
    pub resource_id: String,
    pub capacity_scope_id: String,
    pub capacity_limit_units: u64,
    pub capacity_unit: QuantityUnit,
    pub maximum_concurrent_units: u64,
    pub peak_at_epoch_seconds: u64,
    pub peak_assignment_ids: Vec<String>,
    pub first_planned_start_epoch_seconds: u64,
    pub last_planned_end_epoch_seconds: u64,
    pub assignment_ids: Vec<String>,
}

/// Deterministic result for a proposed plan. This report contains no reservation
/// identifier, consume receipt, dispatch authorization, or claim of physical
/// truth; those require separate owner-authoritative systems.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledCapacityReportV1 {
    pub evaluation_time_epoch_seconds: u64,
    pub policy_id: String,
    pub time_profile_id: String,
    pub max_observation_age_seconds: u64,
    pub allowed_future_clock_skew_seconds: u64,
    pub assignments: Vec<ScheduledCapacityAssignmentReceiptV1>,
    pub resources: Vec<ScheduledCapacityResourceSummaryV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct PhysicalResourceKey {
    kind: CargoTransferCapacityResourceKind,
    resource_id: String,
}

#[derive(Clone, Debug)]
struct CapacityEvent {
    time: u64,
    is_start: bool,
    assignment_id: String,
    evidence_id: String,
    units: u64,
    limit: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PeakCapacityUsage {
    units: u64,
    at_epoch_seconds: u64,
    assignment_ids: Vec<String>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_evidence(
    evidence: &ScheduledCapacityEvidenceV1,
    evaluation_time_epoch_seconds: u64,
    policy: &ScheduledCapacityPolicyV1,
) -> Result<(), ScheduledCapacityPlanError> {
    if !nonempty(&evidence.budget_id)
        || !nonempty(&evidence.resource_id)
        || !nonempty(&evidence.capacity_scope_id)
        || !nonempty(&evidence.authority_id)
        || !nonempty(&evidence.authority_profile_id)
        || !nonempty(&evidence.source_record_id)
        || !nonempty(&evidence.source_schema_profile_id)
        || !nonempty(&evidence.source_record_digest)
        || !nonempty(&evidence.time_profile_id)
        || evidence
            .terminal_id
            .as_deref()
            .is_some_and(|id| !nonempty(id))
        || (evidence.resource_kind == CargoTransferCapacityResourceKind::Equipment
            && evidence.terminal_id.is_none())
        || evidence.capacity_units == 0
        || evidence.revision == 0
    {
        return Err(ScheduledCapacityPlanError::InvalidIdentityOrProvenance);
    }
    if evidence
        .supersedes_revision
        .is_some_and(|previous| previous >= evidence.revision)
    {
        return Err(ScheduledCapacityPlanError::InvalidRevisionLineage);
    }
    if evidence.status != CargoTransferCapacityEvidenceStatus::ReportedCurrent {
        return Err(ScheduledCapacityPlanError::NotCurrent);
    }
    if evidence.time_profile_id != policy.expected_time_profile_id {
        return Err(ScheduledCapacityPlanError::TimeProfileMismatch);
    }

    let observed = evidence
        .capacity_observed_at_epoch_seconds
        .ok_or(ScheduledCapacityPlanError::MissingObservationTime)?;
    let recorded = evidence
        .repository_recorded_at_epoch_seconds
        .ok_or(ScheduledCapacityPlanError::MissingRepositoryRecordTime)?;
    let valid_from = evidence
        .valid_from_epoch_seconds
        .ok_or(ScheduledCapacityPlanError::MissingValidityInterval)?;
    let valid_until = evidence
        .valid_until_epoch_seconds
        .ok_or(ScheduledCapacityPlanError::MissingValidityInterval)?;
    if valid_from >= valid_until {
        return Err(ScheduledCapacityPlanError::InvalidValidityInterval);
    }

    let latest_accepted_time =
        evaluation_time_epoch_seconds.saturating_add(policy.allowed_future_clock_skew_seconds);
    if observed > latest_accepted_time || recorded > latest_accepted_time {
        return Err(ScheduledCapacityPlanError::TimestampInFuture);
    }
    if evaluation_time_epoch_seconds.saturating_sub(observed) > policy.max_observation_age_seconds {
        return Err(ScheduledCapacityPlanError::ObservationTooOld);
    }
    Ok(())
}

/// Validate a time-dependent capacity plan using only the supplied snapshot and
/// policy. A valid-from timestamp may be in the future (for a published future
/// schedule); validity must still contain the complete operation window.
///
/// The capacity evidence must be fresh at evaluation and must remain within the
/// configured observation-age limit at each planned operation's start. The
/// evidence validity interval is half-open and must contain the entire
/// half-open operation interval. For example, evidence [100, 200) covers an
/// operation [100, 200), but not one ending after 200.
///
/// Competing assignments are checked with a deterministic event sweep. End
/// events precede start events at an equal timestamp, so adjacent windows do
/// not overlap. Capacity is measured as concurrent assigned units, not
/// throughput per hour/shift. Run this alongside the domain-specific route and
/// cargo validators; passing it does not certify safe loading or establish a
/// live reservation.
pub fn validate_scheduled_capacity_plan(
    assignments: &[ScheduledCapacityAssignmentV1],
    evidence: &[ScheduledCapacityEvidenceV1],
    evaluation_time_epoch_seconds: u64,
    policy: &ScheduledCapacityPolicyV1,
) -> Result<ScheduledCapacityReportV1, ScheduledCapacityPlanError> {
    policy.validate()?;

    let mut evidence_by_id = BTreeMap::new();
    let mut referenced_evidence = BTreeSet::new();
    let mut evidence_intervals: BTreeMap<
        PhysicalResourceKey,
        Vec<(u64, u64, String, QuantityUnit)>,
    > = BTreeMap::new();
    let mut units_by_resource: BTreeMap<PhysicalResourceKey, QuantityUnit> = BTreeMap::new();

    for item in evidence {
        validate_evidence(item, evaluation_time_epoch_seconds, policy)?;
        let valid_from = item
            .valid_from_epoch_seconds
            .ok_or(ScheduledCapacityPlanError::MissingValidityInterval)?;
        let valid_until = item
            .valid_until_epoch_seconds
            .ok_or(ScheduledCapacityPlanError::MissingValidityInterval)?;

        if evidence_by_id.contains_key(&item.budget_id) {
            return Err(ScheduledCapacityPlanError::DuplicateEvidenceIdentity);
        }

        let key = PhysicalResourceKey {
            kind: item.resource_kind,
            resource_id: item.resource_id.clone(),
        };
        if units_by_resource
            .insert(key.clone(), item.capacity_unit)
            .is_some_and(|previous| previous != item.capacity_unit)
        {
            return Err(ScheduledCapacityPlanError::CapacityUnitMismatch);
        }

        evidence_intervals.entry(key).or_default().push((
            valid_from,
            valid_until,
            item.budget_id.clone(),
            item.capacity_unit,
        ));
        evidence_by_id.insert(item.budget_id.clone(), item);
    }

    for intervals in evidence_intervals.values_mut() {
        intervals.sort_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)));
        for pair in intervals.windows(2) {
            if pair[1].0 < pair[0].1 {
                return Err(ScheduledCapacityPlanError::AmbiguousCapacityEvidence);
            }
        }
    }

    let mut assignment_ids = BTreeSet::new();
    let mut assignments_by_resource: BTreeMap<PhysicalResourceKey, Vec<CapacityEvent>> =
        BTreeMap::new();
    let mut assignments_for_evidence: BTreeMap<String, Vec<ScheduledCapacityAssignmentV1>> =
        BTreeMap::new();
    let mut receipts = Vec::with_capacity(assignments.len());

    for assignment in assignments {
        if !nonempty(&assignment.assignment_id)
            || !nonempty(&assignment.evidence_id)
            || !nonempty(&assignment.resource_id)
            || !nonempty(&assignment.capacity_scope_id)
        {
            return Err(ScheduledCapacityPlanError::InvalidIdentityOrProvenance);
        }
        if !assignment_ids.insert(assignment.assignment_id.clone()) {
            return Err(ScheduledCapacityPlanError::DuplicateAssignmentIdentity);
        }
        assignment.window.validate()?;
        if assignment.window.time_profile_id != policy.expected_time_profile_id {
            return Err(ScheduledCapacityPlanError::TimeProfileMismatch);
        }
        if assignment.window.planned_start_epoch_seconds < evaluation_time_epoch_seconds {
            return Err(ScheduledCapacityPlanError::OperationWindowAlreadyStarted);
        }
        if assignment.capacity_units == 0 {
            return Err(ScheduledCapacityPlanError::InvalidCapacity);
        }

        let item = evidence_by_id
            .get(&assignment.evidence_id)
            .ok_or(ScheduledCapacityPlanError::UnknownEvidence)?;
        if item.resource_kind != assignment.resource_kind
            || item.resource_id != assignment.resource_id
            || item.capacity_scope_id != assignment.capacity_scope_id
            || item.capacity_unit != assignment.capacity_unit
            || assignment.terminal_id != item.terminal_id
        {
            return Err(ScheduledCapacityPlanError::ResourceBindingMismatch);
        }
        if assignment.window.time_profile_id != item.time_profile_id {
            return Err(ScheduledCapacityPlanError::TimeProfileMismatch);
        }
        let valid_from = item
            .valid_from_epoch_seconds
            .ok_or(ScheduledCapacityPlanError::MissingValidityInterval)?;
        let valid_until = item
            .valid_until_epoch_seconds
            .ok_or(ScheduledCapacityPlanError::MissingValidityInterval)?;
        if valid_from > assignment.window.planned_start_epoch_seconds
            || assignment.window.planned_end_epoch_seconds > valid_until
        {
            return Err(ScheduledCapacityPlanError::WindowNotCoveredByEvidence);
        }
        let observed = item
            .capacity_observed_at_epoch_seconds
            .ok_or(ScheduledCapacityPlanError::MissingObservationTime)?;
        if assignment
            .window
            .planned_start_epoch_seconds
            .saturating_sub(observed)
            > policy.max_observation_age_seconds
        {
            return Err(ScheduledCapacityPlanError::ObservationTooOldAtOperationStart);
        }
        if assignment.capacity_units > item.capacity_units {
            return Err(ScheduledCapacityPlanError::CapacityExceeded);
        }

        let key = PhysicalResourceKey {
            kind: assignment.resource_kind,
            resource_id: assignment.resource_id.clone(),
        };
        let events = assignments_by_resource.entry(key).or_default();
        events.push(CapacityEvent {
            time: assignment.window.planned_start_epoch_seconds,
            is_start: true,
            assignment_id: assignment.assignment_id.clone(),
            evidence_id: assignment.evidence_id.clone(),
            units: assignment.capacity_units,
            limit: item.capacity_units,
        });
        events.push(CapacityEvent {
            time: assignment.window.planned_end_epoch_seconds,
            is_start: false,
            assignment_id: assignment.assignment_id.clone(),
            evidence_id: assignment.evidence_id.clone(),
            units: assignment.capacity_units,
            limit: item.capacity_units,
        });

        referenced_evidence.insert(assignment.evidence_id.clone());
        assignments_for_evidence
            .entry(assignment.evidence_id.clone())
            .or_default()
            .push(assignment.clone());

        receipts.push(ScheduledCapacityAssignmentReceiptV1 {
            assignment_id: assignment.assignment_id.clone(),
            evidence_id: assignment.evidence_id.clone(),
            resource_kind: assignment.resource_kind,
            resource_id: assignment.resource_id.clone(),
            terminal_id: assignment.terminal_id.clone(),
            capacity_scope_id: assignment.capacity_scope_id.clone(),
            capacity_units: assignment.capacity_units,
            capacity_unit: assignment.capacity_unit,
            window: assignment.window.clone(),
            authority_id: item.authority_id.clone(),
            authority_profile_id: item.authority_profile_id.clone(),
            source_record_id: item.source_record_id.clone(),
            source_schema_profile_id: item.source_schema_profile_id.clone(),
            source_record_digest: item.source_record_digest.clone(),
            source_event_time_epoch_seconds: item.source_event_time_epoch_seconds,
            evidence_revision: item.revision,
            supersedes_revision: item.supersedes_revision,
            capacity_observed_at_epoch_seconds: observed,
            repository_recorded_at_epoch_seconds: item
                .repository_recorded_at_epoch_seconds
                .ok_or(ScheduledCapacityPlanError::MissingRepositoryRecordTime)?,
            evidence_valid_from_epoch_seconds: valid_from,
            evidence_valid_until_epoch_seconds: valid_until,
        });
    }

    if referenced_evidence.len() != evidence_by_id.len() {
        return Err(ScheduledCapacityPlanError::UnreferencedEvidence);
    }

    let mut maximum_by_evidence: BTreeMap<String, PeakCapacityUsage> = BTreeMap::new();
    for events in assignments_by_resource.values_mut() {
        // Half-open interval semantics: end (false) sorts before start (true).
        events.sort_by(|a, b| {
            (a.time, a.is_start, &a.assignment_id).cmp(&(b.time, b.is_start, &b.assignment_id))
        });
        let mut active: BTreeMap<String, u64> = BTreeMap::new();
        let mut active_units = 0_u64;
        let mut active_limit: Option<u64> = None;
        let mut active_evidence: Option<String> = None;

        for event in events {
            if !event.is_start {
                let units = active
                    .remove(&event.assignment_id)
                    .ok_or(ScheduledCapacityPlanError::CapacityArithmeticOverflow)?;
                active_units = active_units
                    .checked_sub(units)
                    .ok_or(ScheduledCapacityPlanError::CapacityArithmeticOverflow)?;
                if active.is_empty() {
                    active_limit = None;
                    active_evidence = None;
                }
                continue;
            }

            if active_units == 0 {
                active_limit = Some(event.limit);
                active_evidence = Some(event.evidence_id.clone());
            } else if active_limit != Some(event.limit)
                || active_evidence.as_deref() != Some(event.evidence_id.as_str())
            {
                // Two overlapping schedules backed by different evidence should
                // already be rejected by interval validation. Keep a defensive
                // refusal here in case the input model changes later.
                return Err(ScheduledCapacityPlanError::AmbiguousCapacityEvidence);
            }

            active_units = active_units
                .checked_add(event.units)
                .ok_or(ScheduledCapacityPlanError::CapacityArithmeticOverflow)?;
            if active_units > event.limit {
                return Err(ScheduledCapacityPlanError::CapacityExceeded);
            }
            active.insert(event.assignment_id.clone(), event.units);
            let current_peak = maximum_by_evidence.get(&event.evidence_id);
            if current_peak.is_none_or(|peak| active_units > peak.units) {
                maximum_by_evidence.insert(
                    event.evidence_id.clone(),
                    PeakCapacityUsage {
                        units: active_units,
                        at_epoch_seconds: event.time,
                        assignment_ids: active.keys().cloned().collect(),
                    },
                );
            }
        }
        if !active.is_empty() || active_units != 0 {
            return Err(ScheduledCapacityPlanError::CapacityArithmeticOverflow);
        }
    }

    receipts.sort_by(|a, b| a.assignment_id.cmp(&b.assignment_id));

    let mut resources = Vec::with_capacity(assignments_for_evidence.len());
    for (evidence_id, mut resource_assignments) in assignments_for_evidence {
        resource_assignments.sort_by(|a, b| {
            (a.window.planned_start_epoch_seconds, &a.assignment_id)
                .cmp(&(b.window.planned_start_epoch_seconds, &b.assignment_id))
        });
        let item = evidence_by_id
            .get(&evidence_id)
            .ok_or(ScheduledCapacityPlanError::UnknownEvidence)?;
        let first_start = resource_assignments
            .iter()
            .map(|a| a.window.planned_start_epoch_seconds)
            .min()
            .ok_or(ScheduledCapacityPlanError::InvalidCapacity)?;
        let last_end = resource_assignments
            .iter()
            .map(|a| a.window.planned_end_epoch_seconds)
            .max()
            .ok_or(ScheduledCapacityPlanError::InvalidCapacity)?;
        let peak = maximum_by_evidence
            .get(&evidence_id)
            .ok_or(ScheduledCapacityPlanError::InvalidCapacity)?;
        resources.push(ScheduledCapacityResourceSummaryV1 {
            evidence_id: evidence_id.clone(),
            resource_kind: item.resource_kind,
            resource_id: item.resource_id.clone(),
            capacity_scope_id: item.capacity_scope_id.clone(),
            capacity_limit_units: item.capacity_units,
            capacity_unit: item.capacity_unit,
            maximum_concurrent_units: peak.units,
            peak_at_epoch_seconds: peak.at_epoch_seconds,
            peak_assignment_ids: peak.assignment_ids.clone(),
            first_planned_start_epoch_seconds: first_start,
            last_planned_end_epoch_seconds: last_end,
            assignment_ids: resource_assignments
                .into_iter()
                .map(|assignment| assignment.assignment_id)
                .collect(),
        });
    }
    resources.sort_by(|a, b| {
        (a.resource_kind, &a.resource_id, &a.evidence_id).cmp(&(
            b.resource_kind,
            &b.resource_id,
            &b.evidence_id,
        ))
    });

    Ok(ScheduledCapacityReportV1 {
        evaluation_time_epoch_seconds,
        policy_id: policy.policy_id.clone(),
        time_profile_id: policy.expected_time_profile_id.clone(),
        max_observation_age_seconds: policy.max_observation_age_seconds,
        allowed_future_clock_skew_seconds: policy.allowed_future_clock_skew_seconds,
        assignments: receipts,
        resources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIME_PROFILE: &str = "UTC_UNIX_SECONDS_V1";

    fn policy(max_age: u64) -> ScheduledCapacityPolicyV1 {
        ScheduledCapacityPolicyV1 {
            policy_id: "terminal-capacity-policy-v1".to_owned(),
            expected_time_profile_id: TIME_PROFILE.to_owned(),
            max_observation_age_seconds: max_age,
            allowed_future_clock_skew_seconds: 5,
        }
    }

    fn evidence(
        id: &str,
        kind: CargoTransferCapacityResourceKind,
        resource: &str,
        capacity: u64,
        valid_from: u64,
        valid_until: u64,
        observed_at: u64,
    ) -> ScheduledCapacityEvidenceV1 {
        ScheduledCapacityEvidenceV1 {
            budget_id: id.to_owned(),
            resource_kind: kind,
            resource_id: resource.to_owned(),
            terminal_id: (kind == CargoTransferCapacityResourceKind::Equipment)
                .then(|| "terminal-joburg-inland-1".to_owned()),
            capacity_scope_id: "scenario-shift-alpha".to_owned(),
            capacity_units: capacity,
            capacity_unit: QuantityUnit::Shipments,
            authority_id: "operator-authority".to_owned(),
            authority_profile_id: "operator-profile-v1".to_owned(),
            source_record_id: format!("source-{id}"),
            source_schema_profile_id: "capacity-source-v1".to_owned(),
            source_record_digest: format!("sha256-{id}"),
            revision: 1,
            supersedes_revision: None,
            source_event_time_epoch_seconds: Some(observed_at.saturating_sub(1)),
            capacity_observed_at_epoch_seconds: Some(observed_at),
            repository_recorded_at_epoch_seconds: Some(observed_at + 1),
            valid_from_epoch_seconds: Some(valid_from),
            valid_until_epoch_seconds: Some(valid_until),
            time_profile_id: TIME_PROFILE.to_owned(),
            status: CargoTransferCapacityEvidenceStatus::ReportedCurrent,
        }
    }

    fn assignment(
        id: &str,
        evidence_id: &str,
        kind: CargoTransferCapacityResourceKind,
        resource: &str,
        units: u64,
        start: u64,
        end: u64,
    ) -> ScheduledCapacityAssignmentV1 {
        ScheduledCapacityAssignmentV1 {
            assignment_id: id.to_owned(),
            evidence_id: evidence_id.to_owned(),
            resource_kind: kind,
            resource_id: resource.to_owned(),
            capacity_scope_id: "scenario-shift-alpha".to_owned(),
            capacity_units: units,
            capacity_unit: QuantityUnit::Shipments,
            terminal_id: (kind == CargoTransferCapacityResourceKind::Equipment)
                .then(|| "terminal-joburg-inland-1".to_owned()),
            window: OperationWindowV1 {
                time_profile_id: TIME_PROFILE.to_owned(),
                planned_start_epoch_seconds: start,
                planned_end_epoch_seconds: end,
            },
        }
    }

    fn plan(
        assignments: &[ScheduledCapacityAssignmentV1],
        evidence: &[ScheduledCapacityEvidenceV1],
        evaluation_time: u64,
        max_age: u64,
    ) -> Result<ScheduledCapacityReportV1, ScheduledCapacityPlanError> {
        validate_scheduled_capacity_plan(assignments, evidence, evaluation_time, &policy(max_age))
    }

    #[test]
    fn future_published_evidence_is_accepted_before_its_valid_from_time() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            120,
            200,
            90,
        );
        let assignment = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            1,
            120,
            130,
        );
        let report = plan(&[assignment], &[ev], 100, 100).expect("future-published schedule");

        assert_eq!(
            report.assignments[0].window.planned_start_epoch_seconds,
            120
        );
        assert_eq!(report.resources[0].capacity_limit_units, 5);
    }

    #[test]
    fn overlapping_capacity_aggregation_overflow_fails_closed() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            u64::MAX,
            80,
            200,
            90,
        );
        let first = assignment(
            "transfer-a",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            u64::MAX,
            100,
            120,
        );
        let second = assignment(
            "transfer-b",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            1,
            110,
            130,
        );

        assert_eq!(
            plan(&[first, second], &[ev], 100, 100),
            Err(ScheduledCapacityPlanError::CapacityArithmeticOverflow)
        );
    }

    #[test]
    fn future_schedule_must_cover_the_entire_operation_window() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            80,
            150,
            90,
        );
        let assignment = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            1,
            140,
            160,
        );
        assert_eq!(
            plan(&[assignment], &[ev], 100, 100),
            Err(ScheduledCapacityPlanError::WindowNotCoveredByEvidence)
        );
    }

    #[test]
    fn fresh_at_evaluation_but_stale_by_operation_start_is_rejected() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            80,
            200,
            80,
        );
        let assignment = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            1,
            120,
            130,
        );
        assert_eq!(
            plan(&[assignment], &[ev], 100, 30),
            Err(ScheduledCapacityPlanError::ObservationTooOldAtOperationStart)
        );
    }

    #[test]
    fn half_open_evidence_and_operation_windows_allow_equal_endpoints() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            100,
            200,
            90,
        );
        let assignment = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            100,
            200,
        );
        assert!(plan(&[assignment], &[ev], 90, 120).is_ok());
    }

    #[test]
    fn empty_reversed_and_wrong_profile_windows_fail_closed() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::TransferEdge,
            "rail-leg-1",
            5,
            80,
            200,
            90,
        );
        let mut empty = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::TransferEdge,
            "rail-leg-1",
            1,
            100,
            100,
        );
        assert_eq!(
            plan(&[empty.clone()], std::slice::from_ref(&ev), 100, 100),
            Err(ScheduledCapacityPlanError::InvalidOperationWindow)
        );
        empty.window.planned_start_epoch_seconds = 110;
        empty.window.planned_end_epoch_seconds = 100;
        assert_eq!(
            plan(&[empty.clone()], std::slice::from_ref(&ev), 100, 100),
            Err(ScheduledCapacityPlanError::InvalidOperationWindow)
        );
        empty.window.planned_start_epoch_seconds = 100;
        empty.window.planned_end_epoch_seconds = 110;
        empty.window.time_profile_id = "local-wall-clock-unqualified".to_owned();
        assert_eq!(
            plan(&[empty], &[ev], 100, 100),
            Err(ScheduledCapacityPlanError::TimeProfileMismatch)
        );
    }

    #[test]
    fn operation_window_that_started_before_evaluation_fails_closed() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            80,
            200,
            90,
        );
        let already_started = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            1,
            90,
            110,
        );
        assert_eq!(
            plan(&[already_started], &[ev], 100, 100),
            Err(ScheduledCapacityPlanError::OperationWindowAlreadyStarted)
        );
    }

    #[test]
    fn adjacent_windows_reuse_capacity_without_false_overlap() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::TransferEdge,
            "road-rail-handoff",
            5,
            80,
            200,
            90,
        );
        let first = assignment(
            "transfer-a",
            "budget-1",
            CargoTransferCapacityResourceKind::TransferEdge,
            "road-rail-handoff",
            5,
            100,
            110,
        );
        let second = assignment(
            "transfer-b",
            "budget-1",
            CargoTransferCapacityResourceKind::TransferEdge,
            "road-rail-handoff",
            5,
            110,
            120,
        );
        let report = plan(&[second, first], &[ev], 100, 100).expect("valid adjacent windows");
        assert_eq!(report.resources[0].maximum_concurrent_units, 5);
        assert_eq!(
            report.resources[0].assignment_ids,
            vec!["transfer-a".to_owned(), "transfer-b".to_owned()]
        );
    }

    #[test]
    fn report_identifies_peak_time_and_exact_contributing_assignments() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            80,
            200,
            90,
        );
        let first = assignment(
            "transfer-a",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            2,
            100,
            120,
        );
        let second = assignment(
            "transfer-b",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            3,
            110,
            130,
        );
        let report = plan(&[first, second], &[ev], 100, 100).expect("valid overlapping capacity");
        let resource = &report.resources[0];

        assert_eq!(resource.maximum_concurrent_units, 5);
        assert_eq!(resource.peak_at_epoch_seconds, 110);
        assert_eq!(
            resource.peak_assignment_ids,
            vec!["transfer-a".to_owned(), "transfer-b".to_owned()]
        );
    }

    #[test]
    fn intersecting_assignments_cannot_each_spend_full_capacity() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            80,
            200,
            90,
        );
        let first = assignment(
            "transfer-a",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            3,
            100,
            120,
        );
        let second = assignment(
            "transfer-b",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            3,
            110,
            130,
        );
        assert_eq!(
            plan(&[first, second], &[ev], 100, 100),
            Err(ScheduledCapacityPlanError::CapacityExceeded)
        );
    }

    #[test]
    fn nonoverlapping_bookings_can_reuse_shared_equipment_capacity() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            80,
            200,
            90,
        );
        let first = assignment(
            "transfer-a",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            100,
            110,
        );
        let second = assignment(
            "transfer-b",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            120,
            130,
        );
        let report = plan(&[first, second], &[ev], 100, 100).expect("nonoverlapping bookings");
        assert_eq!(report.resources[0].maximum_concurrent_units, 5);
    }

    #[test]
    fn overlapping_current_evidence_for_one_physical_resource_is_ambiguous() {
        let first = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            80,
            160,
            90,
        );
        let second = evidence(
            "budget-2",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            4,
            150,
            200,
            100,
        );
        let assignment = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            1,
            120,
            130,
        );
        assert_eq!(
            plan(&[assignment], &[first, second], 100, 100),
            Err(ScheduledCapacityPlanError::AmbiguousCapacityEvidence)
        );
    }

    #[test]
    fn report_preserves_policy_clock_window_and_source_lineage() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::TransferEdge,
            "rail-leg-1",
            7,
            80,
            200,
            90,
        );
        let assignment = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::TransferEdge,
            "rail-leg-1",
            4,
            110,
            120,
        );
        let report = plan(&[assignment], &[ev], 100, 100).expect("valid schedule");
        assert_eq!(report.evaluation_time_epoch_seconds, 100);
        assert_eq!(report.policy_id, "terminal-capacity-policy-v1");
        assert_eq!(report.time_profile_id, TIME_PROFILE);
        assert_eq!(report.assignments[0].capacity_observed_at_epoch_seconds, 90);
        assert_eq!(
            report.assignments[0].repository_recorded_at_epoch_seconds,
            91
        );
        assert_eq!(report.assignments[0].evidence_valid_from_epoch_seconds, 80);
        assert_eq!(
            report.assignments[0].evidence_valid_until_epoch_seconds,
            200
        );
        assert_eq!(
            report.assignments[0].window.planned_start_epoch_seconds,
            110
        );
        assert_eq!(
            report.assignments[0].source_record_digest,
            "sha256-budget-1"
        );
    }

    #[test]
    fn replay_is_deterministic_and_a_changed_window_changes_the_receipt() {
        let ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::TransferEdge,
            "rail-leg-1",
            7,
            80,
            200,
            90,
        );
        let initial = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::TransferEdge,
            "rail-leg-1",
            4,
            110,
            120,
        );
        let first = plan(
            std::slice::from_ref(&initial),
            std::slice::from_ref(&ev),
            100,
            100,
        )
        .expect("initial plan");
        let replay = plan(
            std::slice::from_ref(&initial),
            std::slice::from_ref(&ev),
            100,
            100,
        )
        .expect("replayed plan");
        assert_eq!(first, replay);

        let mut changed = initial;
        changed.window.planned_start_epoch_seconds = 111;
        let changed_report = plan(&[changed], &[ev], 100, 100).expect("changed plan");
        assert_ne!(first, changed_report);
    }

    #[test]
    fn conflicted_evidence_cannot_be_used_as_current_capacity() {
        let mut ev = evidence(
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            5,
            80,
            200,
            90,
        );
        ev.status = CargoTransferCapacityEvidenceStatus::Conflicted;
        let assignment = assignment(
            "transfer-1",
            "budget-1",
            CargoTransferCapacityResourceKind::Equipment,
            "crane-1",
            1,
            100,
            110,
        );
        assert_eq!(
            plan(&[assignment], &[ev], 100, 100),
            Err(ScheduledCapacityPlanError::NotCurrent)
        );
    }
}
