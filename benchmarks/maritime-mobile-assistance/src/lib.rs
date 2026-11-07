#![forbid(unsafe_code)]

//! Feasibility verification for named tug and pilot assistance resources.
//!
//! This crate consumes fixed operation placements from the shared-resource
//! fleet oracle. It does not schedule vessels, choose assignments, estimate
//! tug forces, simulate hydrodynamics, dispatch resources, or grant authority.

use std::collections::{BTreeMap, BTreeSet};

use maritime_service_chain_contract::TickWindow;
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

/// Evidence metadata is a reference and review record, not source authentication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReceipt {
    pub evidence_id: String,
    pub source_id: String,
    pub source_revision: String,
    /// Exact resource ID or directed route-leg key to which this receipt applies.
    pub subject_id: String,
    /// Exact resource configuration or route revision for that subject.
    pub subject_revision: String,
    pub state: EvidenceState,
    pub valid_from_tick: u64,
    pub valid_through_tick: u64,
    pub claim_ceiling: String,
}

impl EvidenceReceipt {
    fn validate(&self) -> Result<(), MobileError> {
        if self.evidence_id.trim().is_empty()
            || self.source_id.trim().is_empty()
            || self.source_revision.trim().is_empty()
            || self.subject_id.trim().is_empty()
            || self.subject_revision.trim().is_empty()
            || self.claim_ceiling.trim().is_empty()
            || self.valid_from_tick > self.valid_through_tick
        {
            return Err(MobileError::InvalidEvidenceReceipt(
                self.evidence_id.clone(),
            ));
        }
        Ok(())
    }

    /// Validity endpoints are inclusive; operation windows are half-open.
    /// An evidence receipt therefore needs to cover the last occupied tick,
    /// which is end_tick - 1, not the exclusive end boundary itself.
    fn covers(&self, window: TickWindow) -> bool {
        let Some(last_active_tick) = window.end_tick.checked_sub(1) else {
            return false;
        };
        window.is_valid()
            && self.valid_from_tick <= window.start_tick
            && self.valid_through_tick >= last_active_tick
    }

    fn covers_tick(&self, tick: u64) -> bool {
        self.valid_from_tick <= tick && tick <= self.valid_through_tick
    }
}

/// Versioned, unambiguous evidence subject for an exact reposition leg.
///
/// Each field is UTF-8 byte-length-prefixed, so delimiters inside IDs cannot
/// make two different resource/direction/duration tuples collide.
fn canonical_reposition_subject_id(leg: &RepositionLeg, time_basis_id: &str) -> String {
    let duration = leg.duration_ticks.to_string();
    let fields = [
        leg.leg_id.as_str(),
        leg.resource_id.as_str(),
        leg.from_node_id.as_str(),
        leg.to_node_id.as_str(),
        leg.route_revision.as_str(),
        duration.as_str(),
        time_basis_id,
    ];
    let mut subject = String::from("maritime-reposition-evidence-v1");
    for field in fields {
        subject.push(':');
        subject.push_str(&field.len().to_string());
        subject.push(':');
        subject.push_str(field);
    }
    subject
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssistanceKind {
    PilotTransfer,
    Escort,
    Berthing,
    Unberthing,
    LockApproach,
    TurningAssistance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TugResource {
    pub tug_id: String,
    pub revision: String,
    pub position_node_id: String,
    /// Opaque class identifier supplied by the port/vessel assistance profile.
    pub capability_class_id: String,
    /// Reviewed bollard-pull capability in kN; never inferred from vessel size.
    pub bollard_pull_kn: u32,
    pub capability_evidence: EvidenceReceipt,
    /// Position evidence must be current at the workload's decision tick.
    pub position_evidence: EvidenceReceipt,
    pub availability: Vec<TickWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PilotResource {
    pub pilot_id: String,
    pub revision: String,
    pub position_node_id: String,
    /// Qualification identifiers are opaque and must be supplied by the source owner.
    pub qualification_ids: Vec<String>,
    pub qualification_evidence: EvidenceReceipt,
    pub position_evidence: EvidenceReceipt,
    pub availability: Vec<TickWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositionLeg {
    /// Stable opaque identifier for this exact resource-specific directed leg.
    pub leg_id: String,
    /// Exact tug or pilot identity; travel duration is not shared across resources.
    pub resource_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    /// Fixed travel duration in the workload's integer tick basis.
    pub duration_ticks: u64,
    /// Route/network revision used to derive this leg.
    pub route_revision: String,
    pub evidence: EvidenceReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistanceTask {
    pub task_id: String,
    pub corridor_id: String,
    pub route_revision: String,
    /// Exact operation placement emitted by the existing fleet oracle.
    pub operation: OperationPlacement,
    pub location_node_id: String,
    pub kind: AssistanceKind,
    /// Minimum number of distinct tugs required by an upstream reviewed profile.
    pub required_tug_count: u8,
    /// Accepted tug classes from an upstream reviewed profile; exact identifiers.
    pub accepted_tug_class_ids: Vec<String>,
    /// Minimum aggregate bollard pull from an upstream reviewed profile.
    pub required_bollard_pull_kn: u32,
    /// When present, one pilot with this exact qualification is mandatory.
    pub required_pilot_qualification: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssistanceAssignment {
    pub task_id: String,
    pub tug_ids: Vec<String>,
    pub pilot_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileWorkload {
    pub corridor_id: String,
    pub time_basis_id: String,
    pub decision_tick: u64,
    pub tasks: Vec<AssistanceTask>,
    pub tugs: Vec<TugResource>,
    pub pilots: Vec<PilotResource>,
    pub reposition_legs: Vec<RepositionLeg>,
    pub assignments: Vec<AssistanceAssignment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobileDisposition {
    ReadyForComparison,
    NeedsEvidence,
    Infeasible,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MobileViolation {
    MissingTaskAssignment(String),
    InsufficientTugCount(String),
    TugClassMismatch(String),
    InsufficientBollardPull(String),
    UnexpectedTugAssignment(String),
    MissingPilotAssignment(String),
    UnexpectedPilotAssignment(String),
    PilotQualificationMismatch(String),
    EvidenceStateNotCurrent {
        resource_id: String,
        evidence_id: String,
        state: EvidenceState,
    },
    EvidenceWindowInsufficient {
        resource_id: String,
        evidence_id: String,
    },
    ResourceUnavailable(String),
    ResourceOverlap(String),
    MissingRepositionLeg {
        resource_id: String,
        from_node_id: String,
        to_node_id: String,
    },
    RepositionRouteRevisionMismatch(String),
    RepositionEvidenceStateNotCurrent {
        resource_id: String,
        evidence_id: String,
        state: EvidenceState,
    },
    RepositionEvidenceWindowInsufficient {
        resource_id: String,
        evidence_id: String,
    },
    RepositionTimeInsufficient(String),
}

impl MobileViolation {
    fn is_unsupported(&self) -> bool {
        matches!(
            self,
            Self::EvidenceStateNotCurrent {
                state: EvidenceState::Revoked | EvidenceState::Unsupported,
                ..
            } | Self::RepositionEvidenceStateNotCurrent {
                state: EvidenceState::Revoked | EvidenceState::Unsupported,
                ..
            }
        )
    }

    fn needs_evidence(&self) -> bool {
        matches!(
            self,
            Self::EvidenceStateNotCurrent {
                state: EvidenceState::Stale | EvidenceState::Unknown | EvidenceState::Disputed,
                ..
            } | Self::EvidenceWindowInsufficient { .. }
                | Self::RepositionRouteRevisionMismatch(_)
                | Self::RepositionEvidenceStateNotCurrent {
                    state: EvidenceState::Stale | EvidenceState::Unknown | EvidenceState::Disputed,
                    ..
                }
                | Self::RepositionEvidenceWindowInsufficient { .. }
                | Self::MissingRepositionLeg { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileReport {
    pub disposition: MobileDisposition,
    pub violations: Vec<MobileViolation>,
    pub tasks_checked: usize,
    pub assignments_checked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MobileError {
    EmptyCorridor,
    EmptyTimeBasis,
    DuplicateTask(String),
    DuplicateTug(String),
    DuplicatePilot(String),
    DuplicateAssignment(String),
    DuplicateRepositionLeg(String),
    UnknownTask(String),
    UnknownTug(String),
    UnknownPilot(String),
    UnknownResource(String),
    InvalidTask(String),
    InvalidTug(String),
    InvalidPilot(String),
    InvalidRepositionLeg(String),
    InvalidEvidenceReceipt(String),
    InvalidAvailability(String),
    AvailabilityWindowsOverlap(String),
    InvalidAssignment(String),
    FleetSelectionMismatch(String),
    FleetOperationNotFound(String),
}

fn unique_nonempty_identifiers(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && seen.insert(value))
}

fn validate_availability(resource_id: &str, windows: &[TickWindow]) -> Result<(), MobileError> {
    if windows.iter().any(|window| !window.is_valid()) {
        return Err(MobileError::InvalidAvailability(resource_id.to_owned()));
    }
    for pair in windows.windows(2) {
        if pair[0].start_tick > pair[1].start_tick {
            return Err(MobileError::InvalidAvailability(resource_id.to_owned()));
        }
        if pair[0].end_tick > pair[1].start_tick {
            return Err(MobileError::AvailabilityWindowsOverlap(
                resource_id.to_owned(),
            ));
        }
    }
    Ok(())
}

fn availability_covers(windows: &[TickWindow], interval: TickWindow) -> bool {
    windows.iter().any(|window| window.covers(interval))
}

fn evidence_violation(
    resource_id: &str,
    evidence: &EvidenceReceipt,
    required_window: TickWindow,
    reposition: bool,
    violations: &mut BTreeSet<MobileViolation>,
) {
    if evidence.state != EvidenceState::Current {
        let violation = if reposition {
            MobileViolation::RepositionEvidenceStateNotCurrent {
                resource_id: resource_id.to_owned(),
                evidence_id: evidence.evidence_id.clone(),
                state: evidence.state,
            }
        } else {
            MobileViolation::EvidenceStateNotCurrent {
                resource_id: resource_id.to_owned(),
                evidence_id: evidence.evidence_id.clone(),
                state: evidence.state,
            }
        };
        violations.insert(violation);
    } else if !evidence.covers(required_window) {
        let violation = if reposition {
            MobileViolation::RepositionEvidenceWindowInsufficient {
                resource_id: resource_id.to_owned(),
                evidence_id: evidence.evidence_id.clone(),
            }
        } else {
            MobileViolation::EvidenceWindowInsufficient {
                resource_id: resource_id.to_owned(),
                evidence_id: evidence.evidence_id.clone(),
            }
        };
        violations.insert(violation);
    }
}

fn position_evidence_violation(
    resource_id: &str,
    evidence: &EvidenceReceipt,
    decision_tick: u64,
    violations: &mut BTreeSet<MobileViolation>,
) {
    if evidence.state != EvidenceState::Current {
        violations.insert(MobileViolation::EvidenceStateNotCurrent {
            resource_id: resource_id.to_owned(),
            evidence_id: evidence.evidence_id.clone(),
            state: evidence.state,
        });
    } else if !evidence.covers_tick(decision_tick) {
        violations.insert(MobileViolation::EvidenceWindowInsufficient {
            resource_id: resource_id.to_owned(),
            evidence_id: evidence.evidence_id.clone(),
        });
    }
}

fn reposition_leg<'a>(
    legs: &'a [RepositionLeg],
    from_node_id: &str,
    to_node_id: &str,
    route_revision: &str,
    resource_id: &str,
    violations: &mut BTreeSet<MobileViolation>,
) -> Option<&'a RepositionLeg> {
    if from_node_id == to_node_id {
        return None;
    }

    let mut pair_found = false;
    for leg in legs {
        if leg.resource_id == resource_id
            && leg.from_node_id == from_node_id
            && leg.to_node_id == to_node_id
        {
            pair_found = true;
            if leg.route_revision == route_revision {
                return Some(leg);
            }
        }
    }

    if pair_found {
        violations.insert(MobileViolation::RepositionRouteRevisionMismatch(
            resource_id.to_owned(),
        ));
    } else {
        violations.insert(MobileViolation::MissingRepositionLeg {
            resource_id: resource_id.to_owned(),
            from_node_id: from_node_id.to_owned(),
            to_node_id: to_node_id.to_owned(),
        });
    }
    None
}

struct ResourceSequenceContext<'a> {
    resource_id: &'a str,
    initial_node_id: &'a str,
    availability: &'a [TickWindow],
    position_evidence: &'a EvidenceReceipt,
}

fn check_resource_sequence(
    context: ResourceSequenceContext<'_>,
    decision_tick: u64,
    route_legs: &[RepositionLeg],
    mut tasks: Vec<&AssistanceTask>,
    violations: &mut BTreeSet<MobileViolation>,
) {
    let ResourceSequenceContext {
        resource_id,
        initial_node_id,
        availability,
        position_evidence,
    } = context;
    position_evidence_violation(resource_id, position_evidence, decision_tick, violations);
    tasks.sort_by(|left, right| {
        left.operation
            .start_tick
            .cmp(&right.operation.start_tick)
            .then_with(|| left.operation.end_tick.cmp(&right.operation.end_tick))
            .then_with(|| left.task_id.cmp(&right.task_id))
    });

    let mut current_node = initial_node_id.to_owned();
    let mut ready_tick = decision_tick;
    let mut previous_end = None;

    for task in tasks {
        let operation_window = TickWindow {
            start_tick: task.operation.start_tick,
            end_tick: task.operation.end_tick,
        };

        if let Some(end_tick) = previous_end
            && task.operation.start_tick < end_tick
        {
            violations.insert(MobileViolation::ResourceOverlap(resource_id.to_owned()));
            continue;
        }

        if !availability_covers(availability, operation_window) {
            violations.insert(MobileViolation::ResourceUnavailable(resource_id.to_owned()));
        }

        if current_node != task.location_node_id {
            let Some(leg) = reposition_leg(
                route_legs,
                &current_node,
                &task.location_node_id,
                &task.route_revision,
                resource_id,
                violations,
            ) else {
                // Without a route leg, travel feasibility is unknown.
                current_node = task.location_node_id.clone();
                ready_tick = task.operation.end_tick;
                previous_end = Some(task.operation.end_tick);
                continue;
            };

            let Some(departure_tick) = task.operation.start_tick.checked_sub(leg.duration_ticks)
            else {
                violations.insert(MobileViolation::RepositionTimeInsufficient(
                    resource_id.to_owned(),
                ));
                current_node = task.location_node_id.clone();
                ready_tick = task.operation.end_tick;
                previous_end = Some(task.operation.end_tick);
                continue;
            };
            let Some(arrival_tick) = departure_tick.checked_add(leg.duration_ticks) else {
                violations.insert(MobileViolation::RepositionTimeInsufficient(
                    resource_id.to_owned(),
                ));
                current_node = task.location_node_id.clone();
                ready_tick = task.operation.end_tick;
                previous_end = Some(task.operation.end_tick);
                continue;
            };

            if departure_tick < ready_tick || arrival_tick > task.operation.start_tick {
                violations.insert(MobileViolation::RepositionTimeInsufficient(
                    resource_id.to_owned(),
                ));
            }

            if leg.duration_ticks > 0 {
                let travel_window = TickWindow {
                    start_tick: departure_tick,
                    end_tick: arrival_tick,
                };
                evidence_violation(resource_id, &leg.evidence, travel_window, true, violations);
                if !availability_covers(availability, travel_window) {
                    violations.insert(MobileViolation::ResourceUnavailable(resource_id.to_owned()));
                }
            }
        }

        current_node = task.location_node_id.clone();
        ready_tick = task.operation.end_tick;
        previous_end = Some(task.operation.end_tick);
    }
}

/// Recompute resource compatibility and reposition feasibility for fixed
/// fleet-oracle operation placements. It does not choose or optimize assignments.
pub fn evaluate_mobile_assistance(workload: &MobileWorkload) -> Result<MobileReport, MobileError> {
    if workload.corridor_id.trim().is_empty() {
        return Err(MobileError::EmptyCorridor);
    }
    if workload.time_basis_id.trim().is_empty() {
        return Err(MobileError::EmptyTimeBasis);
    }

    let mut task_by_id = BTreeMap::new();
    for task in &workload.tasks {
        if task.task_id.trim().is_empty()
            || task.corridor_id != workload.corridor_id
            || task.route_revision.trim().is_empty()
            || task.location_node_id.trim().is_empty()
            || task.operation.start_tick >= task.operation.end_tick
            || task.operation.start_tick < workload.decision_tick
            || (task.required_tug_count == 0
                && (task.required_bollard_pull_kn != 0 || !task.accepted_tug_class_ids.is_empty()))
            || (task.required_tug_count > 0
                && (task.required_bollard_pull_kn == 0 || task.accepted_tug_class_ids.is_empty()))
            || !unique_nonempty_identifiers(&task.accepted_tug_class_ids)
            || task
                .required_pilot_qualification
                .as_ref()
                .is_some_and(|qualification| qualification.trim().is_empty())
        {
            return Err(MobileError::InvalidTask(task.task_id.clone()));
        }
        if task_by_id.insert(task.task_id.as_str(), task).is_some() {
            return Err(MobileError::DuplicateTask(task.task_id.clone()));
        }
    }

    let mut tug_by_id = BTreeMap::new();
    for tug in &workload.tugs {
        if tug.tug_id.trim().is_empty()
            || tug.revision.trim().is_empty()
            || tug.position_node_id.trim().is_empty()
            || tug.capability_class_id.trim().is_empty()
            || tug.bollard_pull_kn == 0
        {
            return Err(MobileError::InvalidTug(tug.tug_id.clone()));
        }
        tug.capability_evidence.validate()?;
        tug.position_evidence.validate()?;
        if tug.capability_evidence.subject_id != tug.tug_id
            || tug.position_evidence.subject_id != tug.tug_id
            || tug.capability_evidence.subject_revision != tug.revision
            || tug.position_evidence.subject_revision != tug.revision
        {
            return Err(MobileError::InvalidEvidenceReceipt(
                tug.capability_evidence.evidence_id.clone(),
            ));
        }
        validate_availability(&tug.tug_id, &tug.availability)?;
        if tug_by_id.insert(tug.tug_id.as_str(), tug).is_some() {
            return Err(MobileError::DuplicateTug(tug.tug_id.clone()));
        }
    }

    let mut pilot_by_id = BTreeMap::new();
    for pilot in &workload.pilots {
        if pilot.pilot_id.trim().is_empty()
            || pilot.revision.trim().is_empty()
            || pilot.position_node_id.trim().is_empty()
            || pilot
                .qualification_ids
                .iter()
                .any(|item| item.trim().is_empty())
        {
            return Err(MobileError::InvalidPilot(pilot.pilot_id.clone()));
        }
        pilot.qualification_evidence.validate()?;
        pilot.position_evidence.validate()?;
        if pilot.qualification_evidence.subject_id != pilot.pilot_id
            || pilot.position_evidence.subject_id != pilot.pilot_id
            || pilot.qualification_evidence.subject_revision != pilot.revision
            || pilot.position_evidence.subject_revision != pilot.revision
        {
            return Err(MobileError::InvalidEvidenceReceipt(
                pilot.qualification_evidence.evidence_id.clone(),
            ));
        }
        validate_availability(&pilot.pilot_id, &pilot.availability)?;
        let mut qualifications = BTreeSet::new();
        if pilot
            .qualification_ids
            .iter()
            .any(|qualification| !qualifications.insert(qualification))
        {
            return Err(MobileError::InvalidPilot(pilot.pilot_id.clone()));
        }
        if pilot_by_id.insert(pilot.pilot_id.as_str(), pilot).is_some() {
            return Err(MobileError::DuplicatePilot(pilot.pilot_id.clone()));
        }
    }

    let mut leg_keys = BTreeSet::new();
    let mut leg_ids = BTreeSet::new();
    for leg in &workload.reposition_legs {
        if leg.leg_id.trim().is_empty()
            || leg.resource_id.trim().is_empty()
            || leg.from_node_id.trim().is_empty()
            || leg.to_node_id.trim().is_empty()
            || leg.from_node_id == leg.to_node_id
            || leg.duration_ticks == 0
            || leg.route_revision.trim().is_empty()
        {
            return Err(MobileError::InvalidRepositionLeg(
                leg.evidence.evidence_id.clone(),
            ));
        }
        leg.evidence.validate()?;
        if !tug_by_id.contains_key(leg.resource_id.as_str())
            && !pilot_by_id.contains_key(leg.resource_id.as_str())
        {
            return Err(MobileError::UnknownResource(leg.resource_id.clone()));
        }
        if leg.evidence.subject_id
            != canonical_reposition_subject_id(leg, &workload.time_basis_id)
            || leg.evidence.subject_revision != leg.route_revision
        {
            return Err(MobileError::InvalidEvidenceReceipt(
                leg.evidence.evidence_id.clone(),
            ));
        }
        if !leg_ids.insert(leg.leg_id.as_str()) {
            return Err(MobileError::DuplicateRepositionLeg(leg.leg_id.clone()));
        }
        let key = (
            leg.resource_id.as_str(),
            leg.from_node_id.as_str(),
            leg.to_node_id.as_str(),
            leg.route_revision.as_str(),
        );
        if !leg_keys.insert(key) {
            return Err(MobileError::DuplicateRepositionLeg(
                leg.evidence.evidence_id.clone(),
            ));
        }
    }

    let mut assignments_by_task = BTreeMap::new();
    for assignment in &workload.assignments {
        if !task_by_id.contains_key(assignment.task_id.as_str()) {
            return Err(MobileError::UnknownTask(assignment.task_id.clone()));
        }
        if assignments_by_task
            .insert(assignment.task_id.as_str(), assignment)
            .is_some()
        {
            return Err(MobileError::DuplicateAssignment(assignment.task_id.clone()));
        }
        let mut unique_tugs = BTreeSet::new();
        for tug_id in &assignment.tug_ids {
            if !unique_tugs.insert(tug_id.as_str()) {
                return Err(MobileError::InvalidAssignment(assignment.task_id.clone()));
            }
            if !tug_by_id.contains_key(tug_id.as_str()) {
                return Err(MobileError::UnknownTug(tug_id.clone()));
            }
        }
        if let Some(pilot_id) = &assignment.pilot_id
            && !pilot_by_id.contains_key(pilot_id.as_str())
        {
            return Err(MobileError::UnknownPilot(pilot_id.clone()));
        }
    }

    let mut violations = BTreeSet::new();
    let mut tug_tasks = BTreeMap::<&str, Vec<&AssistanceTask>>::new();
    let mut pilot_tasks = BTreeMap::<&str, Vec<&AssistanceTask>>::new();

    for task in &workload.tasks {
        let Some(assignment) = assignments_by_task.get(task.task_id.as_str()) else {
            violations.insert(MobileViolation::MissingTaskAssignment(task.task_id.clone()));
            continue;
        };

        if assignment.tug_ids.len() < usize::from(task.required_tug_count) {
            violations.insert(MobileViolation::InsufficientTugCount(task.task_id.clone()));
        }
        if task.required_tug_count == 0 && !assignment.tug_ids.is_empty() {
            violations.insert(MobileViolation::UnexpectedTugAssignment(
                task.task_id.clone(),
            ));
        }

        let total_pull: u128 = assignment
            .tug_ids
            .iter()
            .filter_map(|id| tug_by_id.get(id.as_str()))
            .map(|tug| u128::from(tug.bollard_pull_kn))
            .sum();
        if total_pull < u128::from(task.required_bollard_pull_kn) {
            violations.insert(MobileViolation::InsufficientBollardPull(
                task.task_id.clone(),
            ));
        }

        for tug_id in &assignment.tug_ids {
            let tug = tug_by_id[tug_id.as_str()];
            if !task
                .accepted_tug_class_ids
                .iter()
                .any(|class_id| class_id == &tug.capability_class_id)
            {
                violations.insert(MobileViolation::TugClassMismatch(task.task_id.clone()));
            }
            evidence_violation(
                &tug.tug_id,
                &tug.capability_evidence,
                task.operation_window(),
                false,
                &mut violations,
            );
            tug_tasks.entry(tug_id.as_str()).or_default().push(task);
        }

        match (
            task.required_pilot_qualification.as_deref(),
            assignment.pilot_id.as_deref(),
        ) {
            (Some(_), None) => {
                violations.insert(MobileViolation::MissingPilotAssignment(
                    task.task_id.clone(),
                ));
            }
            (None, Some(_)) => {
                violations.insert(MobileViolation::UnexpectedPilotAssignment(
                    task.task_id.clone(),
                ));
            }
            (Some(required), Some(pilot_id)) => {
                let pilot = pilot_by_id[pilot_id];
                if !pilot
                    .qualification_ids
                    .iter()
                    .any(|qualification| qualification == required)
                {
                    violations.insert(MobileViolation::PilotQualificationMismatch(
                        task.task_id.clone(),
                    ));
                }
                evidence_violation(
                    &pilot.pilot_id,
                    &pilot.qualification_evidence,
                    task.operation_window(),
                    false,
                    &mut violations,
                );
                pilot_tasks.entry(pilot_id).or_default().push(task);
            }
            (None, None) => {}
        }
    }

    for (tug_id, tasks) in tug_tasks {
        let tug = tug_by_id[tug_id];
        check_resource_sequence(
            ResourceSequenceContext {
                resource_id: tug_id,
                initial_node_id: &tug.position_node_id,
                availability: &tug.availability,
                position_evidence: &tug.position_evidence,
            },
            workload.decision_tick,
            &workload.reposition_legs,
            tasks,
            &mut violations,
        );
    }

    for (pilot_id, tasks) in pilot_tasks {
        let pilot = pilot_by_id[pilot_id];
        check_resource_sequence(
            ResourceSequenceContext {
                resource_id: pilot_id,
                initial_node_id: &pilot.position_node_id,
                availability: &pilot.availability,
                position_evidence: &pilot.position_evidence,
            },
            workload.decision_tick,
            &workload.reposition_legs,
            tasks,
            &mut violations,
        );
    }

    let disposition = if violations.iter().any(MobileViolation::is_unsupported) {
        MobileDisposition::Unsupported
    } else if violations.iter().any(MobileViolation::needs_evidence) {
        MobileDisposition::NeedsEvidence
    } else if !violations.is_empty() {
        MobileDisposition::Infeasible
    } else {
        MobileDisposition::ReadyForComparison
    };

    Ok(MobileReport {
        disposition,
        violations: violations.into_iter().collect(),
        tasks_checked: workload.tasks.len(),
        assignments_checked: assignments_by_task.len(),
    })
}

impl AssistanceTask {
    fn operation_window(&self) -> TickWindow {
        TickWindow {
            start_tick: self.operation.start_tick,
            end_tick: self.operation.end_tick,
        }
    }
}

/// Verify assistance only against operations selected by the exact fleet plan.
///
/// Every task must match one full operation placement (including time,
/// alternative, participant and resource uses) and the plan's selected demand
/// alternative. This prevents an otherwise valid tug/pilot report from being
/// attached to an operation that the fleet oracle did not actually select.
pub fn evaluate_mobile_assistance_for_plan(
    workload: &MobileWorkload,
    plan: &FleetPlan,
) -> Result<MobileReport, MobileError> {
    let mut selected_by_demand = BTreeMap::new();
    for selection in &plan.selections {
        if selected_by_demand
            .insert(selection.demand_id.as_str(), selection)
            .is_some()
        {
            return Err(MobileError::FleetSelectionMismatch(
                selection.demand_id.clone(),
            ));
        }
    }

    for task in &workload.tasks {
        let operation = &task.operation;
        let Some(selection) = selected_by_demand.get(operation.demand_id.as_str()) else {
            return Err(MobileError::FleetSelectionMismatch(
                operation.demand_id.clone(),
            ));
        };
        if selection.participant_id != operation.participant_id
            || selection.alternative_id != operation.alternative_id
        {
            return Err(MobileError::FleetSelectionMismatch(
                operation.demand_id.clone(),
            ));
        }

        if !plan
            .operations
            .iter()
            .any(|candidate| candidate == operation)
        {
            return Err(MobileError::FleetOperationNotFound(task.task_id.clone()));
        }
    }

    evaluate_mobile_assistance(workload)
}
