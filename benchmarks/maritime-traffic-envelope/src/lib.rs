#![forbid(unsafe_code)]

//! Independent robust traffic-occupancy verification for a selected fleet plan.
//!
//! This crate composes a FleetPlan produced by the shared service-chain fleet
//! oracle. It verifies direction windows and conservative occupancy envelopes
//! over declared one-way channels, conflict zones and merge zones. It does not
//! schedule vessels, calculate navigability, simulate collision dynamics,
//! infer spatial conflicts, or grant VTS/navigation authority.

use std::collections::{BTreeMap, BTreeSet};

use maritime_service_chain_contract::{ResourceUnit, TickWindow};
use maritime_service_chain_fleet::{FleetPlan, OperationPlacement};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceState {
    Current,
    Stale,
    Unknown,
    Disputed,
    Revoked,
    Unsupported,
}

/// Evidence identity is scoped to the exact subject and revision.
/// It is an intake/review record, not source authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReceipt {
    pub evidence_id: String,
    pub source_id: String,
    pub source_revision: String,
    pub subject_id: String,
    pub subject_revision: String,
    pub state: EvidenceState,
    /// Inclusive validity interval in the workload's tick basis.
    pub valid_from_tick: u64,
    pub valid_through_tick: u64,
    pub claim_ceiling: String,
}

impl EvidenceReceipt {
    fn validate(&self) -> Result<(), TrafficError> {
        if self.evidence_id.trim().is_empty()
            || self.source_id.trim().is_empty()
            || self.source_revision.trim().is_empty()
            || self.subject_id.trim().is_empty()
            || self.subject_revision.trim().is_empty()
            || self.claim_ceiling.trim().is_empty()
            || self.valid_from_tick > self.valid_through_tick
        {
            return Err(TrafficError::InvalidEvidenceReceipt(
                self.evidence_id.clone(),
            ));
        }
        Ok(())
    }

    fn covers(&self, interval: TickWindow) -> bool {
        let Some(last_active_tick) = interval.end_tick.checked_sub(1) else {
            return false;
        };
        interval.is_valid()
            && self.valid_from_tick <= interval.start_tick
            && self.valid_through_tick >= last_active_tick
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrafficResourceKind {
    /// A channel segment whose direction is switched only in declared windows.
    OneWayChannel,
    /// An abstract crossing/turning conflict region with exclusive occupancy.
    ConflictZone,
    /// An abstract merge/join region with exclusive occupancy.
    MergeZone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficWindow {
    pub window: TickWindow,
    /// Required for OneWayChannel; forbidden for ConflictZone and MergeZone.
    pub direction_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficResource {
    /// Exact corridor namespace for this resource profile.
    pub corridor_id: String,
    /// Tick epoch/resolution namespace used by every window in this profile.
    pub time_basis_id: String,
    pub resource_id: String,
    pub revision: String,
    pub kind: TrafficResourceKind,
    /// Direction availability / traffic-service operating windows.
    pub windows: Vec<TrafficWindow>,
    /// Conservative guards applied to every occupancy before conflict checks.
    pub clearance_before_ticks: u64,
    pub clearance_after_ticks: u64,
    pub evidence: EvidenceReceipt,
}

/// A use of a traffic resource by one selected fleet operation.
///
/// early_entry_ticks and late_clearance_ticks define the admitted timing
/// uncertainty envelope around the scheduled operation. The verifier uses the
/// worst-case half-open interval from (start - early_entry) to
/// (end + late_clearance).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficOccupancy {
    /// Must equal the enclosing workload's corridor namespace.
    pub corridor_id: String,
    pub occupancy_id: String,
    pub resource_id: String,
    pub route_revision: String,
    /// Must be byte-for-byte equal to an operation in the selected FleetPlan.
    pub operation: OperationPlacement,
    /// Required for a one-way channel; None for conflict/merge zones.
    pub direction_id: Option<String>,
    pub early_entry_ticks: u64,
    pub late_clearance_ticks: u64,
    pub evidence: EvidenceReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredTrafficUse {
    pub occupancy_id: String,
    pub resource_id: String,
    pub direction_id: Option<String>,
}

/// One exact FleetPlan operation and its complete mapping into the reviewed
/// traffic-resource projection. Empty required_uses is allowed only with a
/// non-empty no_resource_basis attestation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficCoverageEntry {
    pub operation: OperationPlacement,
    pub route_revision: String,
    pub required_uses: Vec<RequiredTrafficUse>,
    pub no_resource_basis: Option<String>,
}

/// Completeness receipt for every operation in one selected FleetPlan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficCoverageManifest {
    pub schema_version: String,
    pub corridor_id: String,
    pub time_basis_id: String,
    pub graph_revision: String,
    pub entries: Vec<TrafficCoverageEntry>,
    pub evidence: EvidenceReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficWorkload {
    pub corridor_id: String,
    pub time_basis_id: String,
    pub decision_tick: u64,
    pub coverage: TrafficCoverageManifest,
    pub resources: Vec<TrafficResource>,
    pub occupancies: Vec<TrafficOccupancy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficDisposition {
    ReadyForComparison,
    NeedsEvidence,
    Infeasible,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TrafficViolation {
    ResourceEvidenceNotCurrent {
        resource_id: String,
        evidence_id: String,
        state: EvidenceState,
    },
    OccupancyEvidenceNotCurrent {
        occupancy_id: String,
        evidence_id: String,
        state: EvidenceState,
    },
    EvidenceWindowInsufficient {
        subject_id: String,
        evidence_id: String,
    },
    TrafficWindowInsufficient(String),
    DirectionNotPermitted(String),
    Conflict {
        resource_id: String,
        first_occupancy_id: String,
        second_occupancy_id: String,
    },
    CoverageEvidenceNotCurrent {
        evidence_id: String,
        state: EvidenceState,
    },
}

impl TrafficViolation {
    fn is_unsupported(&self) -> bool {
        matches!(
            self,
            Self::ResourceEvidenceNotCurrent {
                state: EvidenceState::Revoked | EvidenceState::Unsupported,
                ..
            } | Self::OccupancyEvidenceNotCurrent {
                state: EvidenceState::Revoked | EvidenceState::Unsupported,
                ..
            } | Self::CoverageEvidenceNotCurrent {
                state: EvidenceState::Revoked | EvidenceState::Unsupported,
                ..
            }
        )
    }

    fn needs_evidence(&self) -> bool {
        matches!(
            self,
            Self::ResourceEvidenceNotCurrent {
                state: EvidenceState::Stale | EvidenceState::Unknown | EvidenceState::Disputed,
                ..
            } | Self::OccupancyEvidenceNotCurrent {
                state: EvidenceState::Stale | EvidenceState::Unknown | EvidenceState::Disputed,
                ..
            } | Self::EvidenceWindowInsufficient { .. }
                | Self::CoverageEvidenceNotCurrent {
                    state: EvidenceState::Stale | EvidenceState::Unknown | EvidenceState::Disputed,
                    ..
                }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrafficReport {
    pub disposition: TrafficDisposition,
    pub violations: Vec<TrafficViolation>,
    pub resources_checked: usize,
    pub occupancies_checked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrafficError {
    EmptyCorridor,
    EmptyTimeBasis,
    EmptyResources,
    EmptyOccupancies,
    DuplicateResource(String),
    DuplicateOccupancy(String),
    UnknownResource(String),
    InvalidResource(String),
    InvalidTrafficWindow(String),
    TrafficWindowsOverlap(String),
    DirectionSemanticsMismatch(String),
    InvalidOccupancy(String),
    InvalidEvidenceReceipt(String),
    ResourceEvidenceSubjectMismatch(String),
    ResourceScopeMismatch(String),
    OccupancyEvidenceSubjectMismatch(String),
    OccupancyScopeMismatch(String),
    PlanOperationMismatch(String),
    MissingDemandSelection(String),
    PlanSelectionMismatch(String),
    EmptyPlanOperations,
    InvalidCoverageManifest,
    CoverageScopeMismatch,
    CoverageManifestSubjectMismatch,
    DuplicatePlanOperation(String),
    DuplicateCoverageOperation(String),
    UnknownCoverageOperation(String),
    MissingCoverageOperation(String),
    InvalidCoverageEntry(String),
    DuplicateRequiredTrafficUse(String),
    UnknownCoverageResource(String),
    MissingRequiredOccupancy(String),
    UnmappedOccupancy(String),
    CoverageRouteRevisionMismatch(String),
}

/// Append a UTF-8 byte-length-prefixed field to an identity string.
/// This avoids delimiter ambiguity for opaque source-controlled identifiers.
fn push_field(identity: &mut String, value: &str) {
    identity.push(':');
    identity.push_str(&value.len().to_string());
    identity.push(':');
    identity.push_str(value);
}

fn resource_kind_code(kind: TrafficResourceKind) -> &'static str {
    match kind {
        TrafficResourceKind::OneWayChannel => "one-way-channel",
        TrafficResourceKind::ConflictZone => "conflict-zone",
        TrafficResourceKind::MergeZone => "merge-zone",
    }
}

fn resource_unit_code(unit: ResourceUnit) -> &'static str {
    match unit {
        ResourceUnit::TransitSlots => "transit-slots",
        ResourceUnit::Lockages => "lockages",
        ResourceUnit::BerthSlots => "berth-slots",
        ResourceUnit::Tonnes => "tonnes",
        ResourceUnit::Teu => "teu",
        ResourceUnit::Passengers => "passengers",
        ResourceUnit::Vehicles => "vehicles",
        ResourceUnit::Containers => "containers",
        ResourceUnit::WattHours => "watt-hours",
        ResourceUnit::Millilitres => "millilitres",
    }
}

/// Canonical evidence subject for the complete traffic resource profile.
/// Changing its revision, type, windows, directions or clearance guards changes
/// the subject ID and requires a fresh scoped receipt.
pub fn canonical_resource_subject_id(resource: &TrafficResource) -> String {
    let mut identity = String::from("maritime-traffic-resource-v1");
    push_field(&mut identity, &resource.corridor_id);
    push_field(&mut identity, &resource.time_basis_id);
    push_field(&mut identity, &resource.resource_id);
    push_field(&mut identity, &resource.revision);
    push_field(&mut identity, resource_kind_code(resource.kind));
    push_field(&mut identity, &resource.clearance_before_ticks.to_string());
    push_field(&mut identity, &resource.clearance_after_ticks.to_string());
    push_field(&mut identity, &resource.windows.len().to_string());
    for window in &resource.windows {
        push_field(&mut identity, &window.window.start_tick.to_string());
        push_field(&mut identity, &window.window.end_tick.to_string());
        push_field(&mut identity, window.direction_id.as_deref().unwrap_or(""));
    }
    identity
}

/// Canonical evidence subject for one occupancy envelope. It binds the complete
/// selected operation placement, route revision, resource/direction, uncertainty
/// bounds and time basis. A schedule or uncertainty edit invalidates the receipt.
pub fn canonical_occupancy_subject_id(occupancy: &TrafficOccupancy, time_basis_id: &str) -> String {
    let operation = &occupancy.operation;
    let mut identity = String::from("maritime-traffic-occupancy-v1");
    let start_tick = operation.start_tick.to_string();
    let end_tick = operation.end_tick.to_string();
    let early_entry = occupancy.early_entry_ticks.to_string();
    let late_clearance = occupancy.late_clearance_ticks.to_string();
    for value in [
        occupancy.corridor_id.as_str(),
        occupancy.occupancy_id.as_str(),
        occupancy.resource_id.as_str(),
        occupancy.route_revision.as_str(),
        operation.demand_id.as_str(),
        operation.participant_id.as_str(),
        operation.alternative_id.as_str(),
        operation.segment_id.as_str(),
        start_tick.as_str(),
        end_tick.as_str(),
        occupancy.direction_id.as_deref().unwrap_or(""),
        early_entry.as_str(),
        late_clearance.as_str(),
        time_basis_id,
    ] {
        push_field(&mut identity, value);
    }
    push_field(&mut identity, &operation.resource_uses.len().to_string());
    for usage in &operation.resource_uses {
        push_field(&mut identity, &usage.resource_id);
        push_field(&mut identity, &usage.amount.to_string());
        push_field(&mut identity, resource_unit_code(usage.unit));
    }
    identity
}

#[derive(Debug, Clone, Copy)]
struct EffectiveOccupancy {
    start_tick: u64,
    end_tick: u64,
}

impl EffectiveOccupancy {
    fn window(self) -> TickWindow {
        TickWindow {
            start_tick: self.start_tick,
            end_tick: self.end_tick,
        }
    }
}

fn effective_occupancy(
    occupancy: &TrafficOccupancy,
    resource: &TrafficResource,
) -> Result<EffectiveOccupancy, TrafficError> {
    if occupancy.operation.start_tick >= occupancy.operation.end_tick {
        return Err(TrafficError::InvalidOccupancy(
            occupancy.occupancy_id.clone(),
        ));
    }
    let Some(uncertain_start) = occupancy
        .operation
        .start_tick
        .checked_sub(occupancy.early_entry_ticks)
    else {
        return Err(TrafficError::InvalidOccupancy(
            occupancy.occupancy_id.clone(),
        ));
    };
    let Some(start_tick) = uncertain_start.checked_sub(resource.clearance_before_ticks) else {
        return Err(TrafficError::InvalidOccupancy(
            occupancy.occupancy_id.clone(),
        ));
    };
    let Some(uncertain_end) = occupancy
        .operation
        .end_tick
        .checked_add(occupancy.late_clearance_ticks)
    else {
        return Err(TrafficError::InvalidOccupancy(
            occupancy.occupancy_id.clone(),
        ));
    };
    let Some(end_tick) = uncertain_end.checked_add(resource.clearance_after_ticks) else {
        return Err(TrafficError::InvalidOccupancy(
            occupancy.occupancy_id.clone(),
        ));
    };
    if start_tick >= end_tick {
        return Err(TrafficError::InvalidOccupancy(
            occupancy.occupancy_id.clone(),
        ));
    }
    Ok(EffectiveOccupancy {
        start_tick,
        end_tick,
    })
}

fn validate_windows(resource: &TrafficResource) -> Result<(), TrafficError> {
    if resource.windows.is_empty() {
        return Err(TrafficError::InvalidResource(resource.resource_id.clone()));
    }
    for window in &resource.windows {
        if !window.window.is_valid() {
            return Err(TrafficError::InvalidTrafficWindow(
                resource.resource_id.clone(),
            ));
        }
        match resource.kind {
            TrafficResourceKind::OneWayChannel => {
                if window
                    .direction_id
                    .as_ref()
                    .is_none_or(|direction| direction.trim().is_empty())
                {
                    return Err(TrafficError::DirectionSemanticsMismatch(
                        resource.resource_id.clone(),
                    ));
                }
            }
            TrafficResourceKind::ConflictZone | TrafficResourceKind::MergeZone => {
                if window.direction_id.is_some() {
                    return Err(TrafficError::DirectionSemanticsMismatch(
                        resource.resource_id.clone(),
                    ));
                }
            }
        }
    }
    for pair in resource.windows.windows(2) {
        if pair[0].window.start_tick > pair[1].window.start_tick {
            return Err(TrafficError::InvalidTrafficWindow(
                resource.resource_id.clone(),
            ));
        }
        if pair[0].window.end_tick > pair[1].window.start_tick {
            return Err(TrafficError::TrafficWindowsOverlap(
                resource.resource_id.clone(),
            ));
        }
    }
    Ok(())
}

fn check_plan_binding(plan: &FleetPlan, occupancy: &TrafficOccupancy) -> Result<(), TrafficError> {
    if !plan
        .operations
        .iter()
        .any(|operation| operation == &occupancy.operation)
    {
        return Err(TrafficError::PlanOperationMismatch(
            occupancy.occupancy_id.clone(),
        ));
    }
    let Some(selection) = plan
        .selections
        .iter()
        .find(|selection| selection.demand_id == occupancy.operation.demand_id)
    else {
        return Err(TrafficError::MissingDemandSelection(
            occupancy.operation.demand_id.clone(),
        ));
    };
    if selection.participant_id != occupancy.operation.participant_id
        || selection.alternative_id != occupancy.operation.alternative_id
    {
        return Err(TrafficError::PlanSelectionMismatch(
            occupancy.occupancy_id.clone(),
        ));
    }
    Ok(())
}

/// Verify one fixed FleetPlan against an independently declared traffic graph
/// projection and timing-uncertainty envelope. This is a verifier, not a planner.
pub fn evaluate_traffic_plan(
    plan: &FleetPlan,
    workload: &TrafficWorkload,
) -> Result<TrafficReport, TrafficError> {
    if workload.corridor_id.trim().is_empty() {
        return Err(TrafficError::EmptyCorridor);
    }
    if workload.time_basis_id.trim().is_empty() {
        return Err(TrafficError::EmptyTimeBasis);
    }
    if workload.resources.is_empty() {
        return Err(TrafficError::EmptyResources);
    }
    if workload.occupancies.is_empty() {
        return Err(TrafficError::EmptyOccupancies);
    }

    let mut resources = BTreeMap::<&str, &TrafficResource>::new();
    for resource in &workload.resources {
        if resource.resource_id.trim().is_empty() || resource.revision.trim().is_empty() {
            return Err(TrafficError::InvalidResource(resource.resource_id.clone()));
        }
        if resource.corridor_id != workload.corridor_id
            || resource.time_basis_id != workload.time_basis_id
        {
            return Err(TrafficError::ResourceScopeMismatch(
                resource.resource_id.clone(),
            ));
        }
        validate_windows(resource)?;
        resource.evidence.validate()?;
        if resource.evidence.subject_id != canonical_resource_subject_id(resource)
            || resource.evidence.subject_revision != resource.revision
        {
            return Err(TrafficError::ResourceEvidenceSubjectMismatch(
                resource.resource_id.clone(),
            ));
        }
        if resources
            .insert(resource.resource_id.as_str(), resource)
            .is_some()
        {
            return Err(TrafficError::DuplicateResource(
                resource.resource_id.clone(),
            ));
        }
    }

    let mut occupancy_ids = BTreeSet::new();
    let mut effective_by_resource =
        BTreeMap::<&str, Vec<(&TrafficOccupancy, EffectiveOccupancy)>>::new();
    let mut violations = BTreeSet::new();

    for occupancy in &workload.occupancies {
        if occupancy.corridor_id != workload.corridor_id {
            return Err(TrafficError::OccupancyScopeMismatch(
                occupancy.occupancy_id.clone(),
            ));
        }
        if occupancy.occupancy_id.trim().is_empty()
            || occupancy.route_revision.trim().is_empty()
            || occupancy.operation.demand_id.trim().is_empty()
            || occupancy.operation.participant_id.trim().is_empty()
            || occupancy.operation.alternative_id.trim().is_empty()
            || occupancy.operation.segment_id.trim().is_empty()
            || occupancy.operation.start_tick < workload.decision_tick
        {
            return Err(TrafficError::InvalidOccupancy(
                occupancy.occupancy_id.clone(),
            ));
        }
        if !occupancy_ids.insert(occupancy.occupancy_id.as_str()) {
            return Err(TrafficError::DuplicateOccupancy(
                occupancy.occupancy_id.clone(),
            ));
        }
        check_plan_binding(plan, occupancy)?;

        let Some(resource) = resources.get(occupancy.resource_id.as_str()).copied() else {
            return Err(TrafficError::UnknownResource(occupancy.resource_id.clone()));
        };
        match resource.kind {
            TrafficResourceKind::OneWayChannel => {
                if occupancy
                    .direction_id
                    .as_ref()
                    .is_none_or(|direction| direction.trim().is_empty())
                {
                    return Err(TrafficError::DirectionSemanticsMismatch(
                        occupancy.occupancy_id.clone(),
                    ));
                }
            }
            TrafficResourceKind::ConflictZone | TrafficResourceKind::MergeZone => {
                if occupancy.direction_id.is_some() {
                    return Err(TrafficError::DirectionSemanticsMismatch(
                        occupancy.occupancy_id.clone(),
                    ));
                }
            }
        }
        occupancy.evidence.validate()?;
        if occupancy.evidence.subject_id
            != canonical_occupancy_subject_id(occupancy, &workload.time_basis_id)
            || occupancy.evidence.subject_revision != occupancy.route_revision
        {
            return Err(TrafficError::OccupancyEvidenceSubjectMismatch(
                occupancy.occupancy_id.clone(),
            ));
        }

        let effective = effective_occupancy(occupancy, resource)?;
        if effective.start_tick < workload.decision_tick {
            return Err(TrafficError::InvalidOccupancy(
                occupancy.occupancy_id.clone(),
            ));
        }
        let effective_window = effective.window();

        if resource.evidence.state != EvidenceState::Current {
            violations.insert(TrafficViolation::ResourceEvidenceNotCurrent {
                resource_id: resource.resource_id.clone(),
                evidence_id: resource.evidence.evidence_id.clone(),
                state: resource.evidence.state,
            });
        } else if !resource.evidence.covers(effective_window) {
            violations.insert(TrafficViolation::EvidenceWindowInsufficient {
                subject_id: resource.resource_id.clone(),
                evidence_id: resource.evidence.evidence_id.clone(),
            });
        }

        if occupancy.evidence.state != EvidenceState::Current {
            violations.insert(TrafficViolation::OccupancyEvidenceNotCurrent {
                occupancy_id: occupancy.occupancy_id.clone(),
                evidence_id: occupancy.evidence.evidence_id.clone(),
                state: occupancy.evidence.state,
            });
        } else if !occupancy.evidence.covers(effective_window) {
            violations.insert(TrafficViolation::EvidenceWindowInsufficient {
                subject_id: occupancy.occupancy_id.clone(),
                evidence_id: occupancy.evidence.evidence_id.clone(),
            });
        }

        let has_compatible_window = resource.windows.iter().any(|window| {
            window.window.covers(effective_window) && window.direction_id == occupancy.direction_id
        });
        if !has_compatible_window {
            let has_covering_window = resource
                .windows
                .iter()
                .any(|window| window.window.covers(effective_window));
            if resource.kind == TrafficResourceKind::OneWayChannel && has_covering_window {
                violations.insert(TrafficViolation::DirectionNotPermitted(
                    occupancy.occupancy_id.clone(),
                ));
            } else {
                violations.insert(TrafficViolation::TrafficWindowInsufficient(
                    occupancy.occupancy_id.clone(),
                ));
            }
        }

        effective_by_resource
            .entry(occupancy.resource_id.as_str())
            .or_default()
            .push((occupancy, effective));
    }

    for (resource_id, entries) in &mut effective_by_resource {
        entries.sort_by(|left, right| {
            left.1
                .start_tick
                .cmp(&right.1.start_tick)
                .then_with(|| left.1.end_tick.cmp(&right.1.end_tick))
                .then_with(|| left.0.occupancy_id.cmp(&right.0.occupancy_id))
        });
        for left_index in 0..entries.len() {
            let (left_occupancy, left_effective) = entries[left_index];
            for (right_occupancy, right_effective) in entries.iter().skip(left_index + 1) {
                if right_effective.start_tick >= left_effective.end_tick {
                    break;
                }
                if left_occupancy.occupancy_id == right_occupancy.occupancy_id {
                    continue;
                }
                let (first, second) = if left_occupancy.occupancy_id <= right_occupancy.occupancy_id
                {
                    (
                        left_occupancy.occupancy_id.clone(),
                        right_occupancy.occupancy_id.clone(),
                    )
                } else {
                    (
                        right_occupancy.occupancy_id.clone(),
                        left_occupancy.occupancy_id.clone(),
                    )
                };
                violations.insert(TrafficViolation::Conflict {
                    resource_id: (*resource_id).to_owned(),
                    first_occupancy_id: first,
                    second_occupancy_id: second,
                });
            }
        }
    }

    let disposition = if violations.iter().any(TrafficViolation::is_unsupported) {
        TrafficDisposition::Unsupported
    } else if violations.iter().any(TrafficViolation::needs_evidence) {
        TrafficDisposition::NeedsEvidence
    } else if !violations.is_empty() {
        TrafficDisposition::Infeasible
    } else {
        TrafficDisposition::ReadyForComparison
    };

    Ok(TrafficReport {
        disposition,
        violations: violations.into_iter().collect(),
        resources_checked: resources.len(),
        occupancies_checked: workload.occupancies.len(),
    })
}
