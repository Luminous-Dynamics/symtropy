//! Deterministic GMD black-start and service-restoration schedule verifier.
//!
//! Evaluates a supplied schedule against declared, externally qualified
//! service-bus capability, energy, precedence and exclusive-resource bounds.
//! It does not perform load flow, EMT/RMS dynamics, protection or actuation.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceState {
    Qualified,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityState {
    Approved,
    Pending,
    Denied,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityEnvelope {
    pub source_ref: String,
    pub source_sha256: String,
    pub evidence_state: EvidenceState,
    pub maximum_active_kw: f64,
    pub maximum_reactive_kvar: f64,
    pub maximum_apparent_kva: f64,
    /// Total net AC energy budget available at the same service-bus boundary.
    pub available_energy_kwh: f64,
    /// Energy that must remain unused throughout the declared horizon.
    pub reserve_floor_kwh: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaselineLoad {
    pub active_kw: f64,
    pub reactive_kvar_magnitude: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExclusiveResource {
    pub resource_id: String,
}

/// Transient load applies during execution. At completion it is replaced by
/// the steady incremental load until the end of the scenario horizon.
#[derive(Debug, Clone, PartialEq)]
pub struct RestorationStep {
    pub step_id: String,
    pub service_id: String,
    pub start_offset_seconds: u64,
    pub duration_seconds: u64,
    pub prerequisite_step_ids: Vec<String>,
    pub exclusive_resource_ids: Vec<String>,
    pub transient_active_kw: f64,
    pub transient_reactive_kvar_magnitude: f64,
    pub steady_active_kw: f64,
    pub steady_reactive_kvar_magnitude: f64,
    pub authority_state: AuthorityState,
}

/// All times are elapsed seconds from t0. P0 is a single declared service bus.
#[derive(Debug, Clone, PartialEq)]
pub struct RestorationScenario {
    pub scenario_id: String,
    pub owner_ref: String,
    pub topology_revision: String,
    pub topology_sha256: String,
    pub procedure_revision: String,
    pub procedure_sha256: String,
    pub horizon_seconds: u64,
    pub baseline_load: BaselineLoad,
    pub capability: CapabilityEnvelope,
    pub exclusive_resources: Vec<ExclusiveResource>,
    pub steps: Vec<RestorationStep>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeasibilityStatus {
    FeasibleAgainstQualifiedEnvelope,
    InfeasibleAgainstQualifiedEnvelope,
    IndeterminateCapabilityEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorReadiness {
    ReadyForOperatorReview,
    AwaitingHumanApproval,
    BlockedByAuthority,
    StaleCapabilityEvidence,
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationKind {
    DependencyNotFinished,
    SharedResourceConflict,
    ActivePowerExceeded,
    ReactivePowerExceeded,
    ApparentPowerExceeded,
    EnergyReserveBreached,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleViolation {
    pub kind: ViolationKind,
    pub start_offset_seconds: u64,
    pub end_offset_seconds: u64,
    pub subject_step_ids: Vec<String>,
    pub actual: f64,
    pub limit: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntervalAssessment {
    pub start_offset_seconds: u64,
    pub end_offset_seconds: u64,
    pub active_kw: f64,
    pub reactive_kvar_magnitude: f64,
    pub apparent_kva: f64,
    pub interval_energy_kwh: f64,
    pub cumulative_energy_kwh: f64,
    pub remaining_energy_kwh: f64,
    pub contributing_step_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServicePickup {
    pub service_id: String,
    pub first_planned_pickup_after_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepAuthority {
    pub step_id: String,
    pub authority_state: AuthorityState,
}

/// An assessment is never an executable command.
#[derive(Debug, Clone, PartialEq)]
pub struct RestorationAssessment {
    pub scenario_id: String,
    pub horizon_seconds: u64,
    pub capability_evidence_state: EvidenceState,
    pub feasibility: FeasibilityStatus,
    pub operator_readiness: OperatorReadiness,
    pub execution_authorized: bool,
    pub maximum_active_kw: f64,
    pub maximum_reactive_kvar_magnitude: f64,
    pub maximum_apparent_kva: f64,
    pub total_energy_used_kwh: f64,
    pub minimum_remaining_energy_kwh: f64,
    pub reserve_floor_kwh: f64,
    pub interval_assessments: Vec<IntervalAssessment>,
    pub violations: Vec<ScheduleViolation>,
    pub service_pickups: Vec<ServicePickup>,
    pub step_authorities: Vec<StepAuthority>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestorationError {
    EmptyRequiredField,
    InvalidDigest,
    InvalidHorizon,
    NoSteps,
    InvalidCapability,
    InvalidBaselineLoad,
    InvalidResourceId,
    DuplicateResourceId,
    InvalidStepId,
    DuplicateStepId,
    InvalidServiceId,
    InvalidStepDuration,
    StepOutsideHorizon,
    InvalidStepLoad,
    DuplicatePrerequisite,
    UnknownPrerequisite,
    SelfDependency,
    DependencyCycle,
    DuplicateStepResource,
    UnknownStepResource,
    NumericalOverflow,
}

impl fmt::Display for RestorationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyRequiredField => "required scenario, owner, revision or source reference is empty",
            Self::InvalidDigest => "SHA-256 digest must be 64 lowercase hexadecimal characters",
            Self::InvalidHorizon => "scenario horizon must be positive",
            Self::NoSteps => "restoration schedule must contain at least one step",
            Self::InvalidCapability => "capability and finite energy/reserve envelope is invalid",
            Self::InvalidBaselineLoad => "baseline active and reactive load must be finite and non-negative",
            Self::InvalidResourceId => "exclusive resource ID must be non-empty and trimmed",
            Self::DuplicateResourceId => "exclusive resource IDs must be unique",
            Self::InvalidStepId => "step ID must be non-empty and trimmed",
            Self::DuplicateStepId => "step IDs must be unique",
            Self::InvalidServiceId => "service ID must be non-empty and trimmed",
            Self::InvalidStepDuration => "restoration step duration must be positive",
            Self::StepOutsideHorizon => "restoration step must finish within the scenario horizon",
            Self::InvalidStepLoad => "transient and steady load quantities must be finite and non-negative",
            Self::DuplicatePrerequisite => "a step cannot list the same prerequisite twice",
            Self::UnknownPrerequisite => "a prerequisite step ID does not exist",
            Self::SelfDependency => "a restoration step cannot depend on itself",
            Self::DependencyCycle => "restoration prerequisites contain a cycle",
            Self::DuplicateStepResource => "a step cannot reserve the same resource twice",
            Self::UnknownStepResource => "a step references an undeclared exclusive resource",
            Self::NumericalOverflow => "load, apparent-power or energy arithmetic overflowed",
        };
        f.write_str(message)
    }
}

impl Error for RestorationError {}

/// Verify a supplied restoration schedule against declared envelope bounds.
pub fn assess_restoration(
    scenario: &RestorationScenario,
) -> Result<RestorationAssessment, RestorationError> {
    validate_scenario(scenario)?;

    let step_by_id: BTreeMap<&str, &RestorationStep> = scenario
        .steps
        .iter()
        .map(|step| (step.step_id.as_str(), step))
        .collect();

    let mut violations = Vec::new();
    validate_prerequisite_timing(&scenario.steps, &step_by_id, &mut violations)?;
    validate_exclusive_resources(&scenario.steps, &mut violations);

    let mut boundaries = BTreeSet::new();
    boundaries.insert(0_u64);
    boundaries.insert(scenario.horizon_seconds);
    for step in &scenario.steps {
        let finish = step
            .start_offset_seconds
            .checked_add(step.duration_seconds)
            .ok_or(RestorationError::StepOutsideHorizon)?;
        boundaries.insert(step.start_offset_seconds);
        boundaries.insert(finish);
    }
    let ordered_boundaries: Vec<u64> = boundaries.into_iter().collect();

    let mut interval_assessments = Vec::new();
    let mut maximum_active_kw: f64 = scenario.baseline_load.active_kw;
    let mut maximum_reactive_kvar_magnitude: f64 =
        scenario.baseline_load.reactive_kvar_magnitude;
    let mut maximum_apparent_kva =
        scenario.baseline_load.active_kw.hypot(scenario.baseline_load.reactive_kvar_magnitude);
    let mut cumulative_energy_kwh = 0.0_f64;
    let mut minimum_remaining_energy_kwh = scenario.capability.available_energy_kwh;

    for window in ordered_boundaries.windows(2) {
        let start = window[0];
        let end = window[1];
        if start == end {
            continue;
        }

        let mut active_kw = scenario.baseline_load.active_kw;
        let mut reactive_kvar_magnitude = scenario.baseline_load.reactive_kvar_magnitude;
        let mut contributing_step_ids = Vec::new();

        for step in &scenario.steps {
            let finish = step.start_offset_seconds + step.duration_seconds;
            if finish <= start {
                active_kw += step.steady_active_kw;
                reactive_kvar_magnitude += step.steady_reactive_kvar_magnitude;
                contributing_step_ids.push(step.step_id.clone());
            } else if step.start_offset_seconds <= start && finish > start {
                active_kw += step.transient_active_kw;
                reactive_kvar_magnitude += step.transient_reactive_kvar_magnitude;
                contributing_step_ids.push(step.step_id.clone());
            }
        }
        if !active_kw.is_finite() || !reactive_kvar_magnitude.is_finite() {
            return Err(RestorationError::NumericalOverflow);
        }

        contributing_step_ids.sort();
        let apparent_kva = active_kw.hypot(reactive_kvar_magnitude);
        let duration_seconds = end - start;
        let interval_energy_kwh = active_kw * (duration_seconds as f64) / 3_600.0;
        if !apparent_kva.is_finite() || !interval_energy_kwh.is_finite() {
            return Err(RestorationError::NumericalOverflow);
        }

        cumulative_energy_kwh += interval_energy_kwh;
        let remaining_energy_kwh =
            scenario.capability.available_energy_kwh - cumulative_energy_kwh;
        if !cumulative_energy_kwh.is_finite() || !remaining_energy_kwh.is_finite() {
            return Err(RestorationError::NumericalOverflow);
        }

        maximum_active_kw = maximum_active_kw.max(active_kw);
        maximum_reactive_kvar_magnitude =
            maximum_reactive_kvar_magnitude.max(reactive_kvar_magnitude);
        maximum_apparent_kva = maximum_apparent_kva.max(apparent_kva);
        minimum_remaining_energy_kwh =
            minimum_remaining_energy_kwh.min(remaining_energy_kwh);

        if active_kw > scenario.capability.maximum_active_kw {
            violations.push(make_violation(
                ViolationKind::ActivePowerExceeded,
                start,
                end,
                &contributing_step_ids,
                active_kw,
                scenario.capability.maximum_active_kw,
            ));
        }
        if reactive_kvar_magnitude > scenario.capability.maximum_reactive_kvar {
            violations.push(make_violation(
                ViolationKind::ReactivePowerExceeded,
                start,
                end,
                &contributing_step_ids,
                reactive_kvar_magnitude,
                scenario.capability.maximum_reactive_kvar,
            ));
        }
        if apparent_kva > scenario.capability.maximum_apparent_kva {
            violations.push(make_violation(
                ViolationKind::ApparentPowerExceeded,
                start,
                end,
                &contributing_step_ids,
                apparent_kva,
                scenario.capability.maximum_apparent_kva,
            ));
        }
        if remaining_energy_kwh < scenario.capability.reserve_floor_kwh {
            violations.push(make_violation(
                ViolationKind::EnergyReserveBreached,
                start,
                end,
                &contributing_step_ids,
                remaining_energy_kwh,
                scenario.capability.reserve_floor_kwh,
            ));
        }

        interval_assessments.push(IntervalAssessment {
            start_offset_seconds: start,
            end_offset_seconds: end,
            active_kw,
            reactive_kvar_magnitude,
            apparent_kva,
            interval_energy_kwh,
            cumulative_energy_kwh,
            remaining_energy_kwh,
            contributing_step_ids,
        });
    }

    violations.sort_by(|left, right| {
        (
            left.start_offset_seconds,
            left.end_offset_seconds,
            left.kind,
            &left.subject_step_ids,
        )
            .cmp(&(
                right.start_offset_seconds,
                right.end_offset_seconds,
                right.kind,
                &right.subject_step_ids,
            ))
    });

    let feasibility = match scenario.capability.evidence_state {
        EvidenceState::Stale | EvidenceState::Unknown => {
            FeasibilityStatus::IndeterminateCapabilityEvidence
        }
        EvidenceState::Qualified if violations.is_empty() => {
            FeasibilityStatus::FeasibleAgainstQualifiedEnvelope
        }
        EvidenceState::Qualified => FeasibilityStatus::InfeasibleAgainstQualifiedEnvelope,
    };
    let operator_readiness = readiness(scenario);

    let mut pickup_by_service = BTreeMap::<String, u64>::new();
    for step in &scenario.steps {
        let finish = step.start_offset_seconds + step.duration_seconds;
        pickup_by_service
            .entry(step.service_id.clone())
            .and_modify(|current| *current = (*current).min(finish))
            .or_insert(finish);
    }
    let service_pickups = pickup_by_service
        .into_iter()
        .map(|(service_id, first_planned_pickup_after_seconds)| ServicePickup {
            service_id,
            first_planned_pickup_after_seconds,
        })
        .collect();

    let mut step_authorities: Vec<StepAuthority> = scenario
        .steps
        .iter()
        .map(|step| StepAuthority {
            step_id: step.step_id.clone(),
            authority_state: step.authority_state,
        })
        .collect();
    step_authorities.sort_by(|left, right| left.step_id.cmp(&right.step_id));

    Ok(RestorationAssessment {
        scenario_id: scenario.scenario_id.clone(),
        horizon_seconds: scenario.horizon_seconds,
        capability_evidence_state: scenario.capability.evidence_state,
        feasibility,
        operator_readiness,
        execution_authorized: false,
        maximum_active_kw,
        maximum_reactive_kvar_magnitude,
        maximum_apparent_kva,
        total_energy_used_kwh: cumulative_energy_kwh,
        minimum_remaining_energy_kwh,
        reserve_floor_kwh: scenario.capability.reserve_floor_kwh,
        interval_assessments,
        violations,
        service_pickups,
        step_authorities,
    })
}

fn readiness(scenario: &RestorationScenario) -> OperatorReadiness {
    if scenario.steps.iter().any(|step| step.authority_state == AuthorityState::Denied) {
        return OperatorReadiness::BlockedByAuthority;
    }
    match scenario.capability.evidence_state {
        EvidenceState::Stale => return OperatorReadiness::StaleCapabilityEvidence,
        EvidenceState::Unknown => return OperatorReadiness::Indeterminate,
        EvidenceState::Qualified => {}
    }
    if scenario.steps.iter().any(|step| step.authority_state == AuthorityState::Unknown) {
        return OperatorReadiness::Indeterminate;
    }
    if scenario.steps.iter().any(|step| step.authority_state == AuthorityState::Pending) {
        return OperatorReadiness::AwaitingHumanApproval;
    }
    OperatorReadiness::ReadyForOperatorReview
}

fn validate_scenario(
    scenario: &RestorationScenario,
) -> Result<(), RestorationError> {
    for value in [
        scenario.scenario_id.as_str(),
        scenario.owner_ref.as_str(),
        scenario.topology_revision.as_str(),
        scenario.procedure_revision.as_str(),
        scenario.capability.source_ref.as_str(),
    ] {
        if value.trim().is_empty() || value.trim() != value {
            return Err(RestorationError::EmptyRequiredField);
        }
    }
    for digest in [
        scenario.topology_sha256.as_str(),
        scenario.procedure_sha256.as_str(),
        scenario.capability.source_sha256.as_str(),
    ] {
        if !valid_sha256(digest) {
            return Err(RestorationError::InvalidDigest);
        }
    }
    if scenario.horizon_seconds == 0 {
        return Err(RestorationError::InvalidHorizon);
    }
    if scenario.steps.is_empty() {
        return Err(RestorationError::NoSteps);
    }

    let capability = &scenario.capability;
    if !all_finite_non_negative(&[
        capability.maximum_active_kw,
        capability.maximum_reactive_kvar,
        capability.maximum_apparent_kva,
        capability.available_energy_kwh,
        capability.reserve_floor_kwh,
    ]) || capability.maximum_active_kw <= 0.0
        || capability.maximum_apparent_kva <= 0.0
        || capability.available_energy_kwh <= capability.reserve_floor_kwh
    {
        return Err(RestorationError::InvalidCapability);
    }
    if !all_finite_non_negative(&[
        scenario.baseline_load.active_kw,
        scenario.baseline_load.reactive_kvar_magnitude,
    ]) {
        return Err(RestorationError::InvalidBaselineLoad);
    }

    let mut resource_ids = BTreeSet::new();
    for resource in &scenario.exclusive_resources {
        if resource.resource_id.trim().is_empty()
            || resource.resource_id.trim() != resource.resource_id
        {
            return Err(RestorationError::InvalidResourceId);
        }
        if !resource_ids.insert(resource.resource_id.as_str()) {
            return Err(RestorationError::DuplicateResourceId);
        }
    }

    let mut step_ids = BTreeSet::new();
    for step in &scenario.steps {
        if step.step_id.trim().is_empty() || step.step_id.trim() != step.step_id {
            return Err(RestorationError::InvalidStepId);
        }
        if !step_ids.insert(step.step_id.as_str()) {
            return Err(RestorationError::DuplicateStepId);
        }
        if step.service_id.trim().is_empty() || step.service_id.trim() != step.service_id {
            return Err(RestorationError::InvalidServiceId);
        }
        if step.duration_seconds == 0 {
            return Err(RestorationError::InvalidStepDuration);
        }
        let finish = step
            .start_offset_seconds
            .checked_add(step.duration_seconds)
            .ok_or(RestorationError::StepOutsideHorizon)?;
        if finish > scenario.horizon_seconds {
            return Err(RestorationError::StepOutsideHorizon);
        }
        if !all_finite_non_negative(&[
            step.transient_active_kw,
            step.transient_reactive_kvar_magnitude,
            step.steady_active_kw,
            step.steady_reactive_kvar_magnitude,
        ]) {
            return Err(RestorationError::InvalidStepLoad);
        }

        let mut local_prerequisites = BTreeSet::new();
        for prerequisite in &step.prerequisite_step_ids {
            if prerequisite == &step.step_id {
                return Err(RestorationError::SelfDependency);
            }
            if !local_prerequisites.insert(prerequisite.as_str()) {
                return Err(RestorationError::DuplicatePrerequisite);
            }
        }

        let mut local_resources = BTreeSet::new();
        for resource_id in &step.exclusive_resource_ids {
            if !local_resources.insert(resource_id.as_str()) {
                return Err(RestorationError::DuplicateStepResource);
            }
            if !resource_ids.contains(resource_id.as_str()) {
                return Err(RestorationError::UnknownStepResource);
            }
        }
    }

    for step in &scenario.steps {
        for prerequisite in &step.prerequisite_step_ids {
            if !step_ids.contains(prerequisite.as_str()) {
                return Err(RestorationError::UnknownPrerequisite);
            }
        }
    }
    ensure_acyclic(&scenario.steps)?;
    Ok(())
}

fn ensure_acyclic(steps: &[RestorationStep]) -> Result<(), RestorationError> {
    let mut indegree: BTreeMap<&str, usize> = steps
        .iter()
        .map(|step| (step.step_id.as_str(), step.prerequisite_step_ids.len()))
        .collect();
    let mut children = BTreeMap::<&str, Vec<&str>>::new();
    for step in steps {
        for prerequisite in &step.prerequisite_step_ids {
            children
                .entry(prerequisite.as_str())
                .or_default()
                .push(step.step_id.as_str());
        }
    }

    let mut ready: VecDeque<&str> = indegree
        .iter()
        .filter_map(|(step_id, count)| (*count == 0).then_some(*step_id))
        .collect();
    let mut visited = 0_usize;

    while let Some(step_id) = ready.pop_front() {
        visited += 1;
        if let Some(dependents) = children.get(step_id) {
            for dependent in dependents {
                let count = indegree
                    .get_mut(dependent)
                    .expect("all child IDs are validated");
                *count -= 1;
                if *count == 0 {
                    ready.push_back(dependent);
                }
            }
        }
    }

    if visited == steps.len() {
        Ok(())
    } else {
        Err(RestorationError::DependencyCycle)
    }
}

fn validate_prerequisite_timing(
    steps: &[RestorationStep],
    step_by_id: &BTreeMap<&str, &RestorationStep>,
    violations: &mut Vec<ScheduleViolation>,
) -> Result<(), RestorationError> {
    for step in steps {
        for prerequisite_id in &step.prerequisite_step_ids {
            let prerequisite = step_by_id
                .get(prerequisite_id.as_str())
                .ok_or(RestorationError::UnknownPrerequisite)?;
            let prerequisite_finish = prerequisite
                .start_offset_seconds
                .checked_add(prerequisite.duration_seconds)
                .ok_or(RestorationError::StepOutsideHorizon)?;
            if prerequisite_finish > step.start_offset_seconds {
                violations.push(make_violation(
                    ViolationKind::DependencyNotFinished,
                    step.start_offset_seconds,
                    step.start_offset_seconds + step.duration_seconds,
                    &[prerequisite.step_id.clone(), step.step_id.clone()],
                    prerequisite_finish as f64,
                    step.start_offset_seconds as f64,
                ));
            }
        }
    }
    Ok(())
}

fn validate_exclusive_resources(
    steps: &[RestorationStep],
    violations: &mut Vec<ScheduleViolation>,
) {
    for (left_index, left) in steps.iter().enumerate() {
        let left_finish = left.start_offset_seconds + left.duration_seconds;
        for right in steps.iter().skip(left_index + 1) {
            let right_finish = right.start_offset_seconds + right.duration_seconds;
            let overlap_start = left.start_offset_seconds.max(right.start_offset_seconds);
            let overlap_end = left_finish.min(right_finish);
            if overlap_start >= overlap_end {
                continue;
            }
            for resource_id in &left.exclusive_resource_ids {
                if right.exclusive_resource_ids.contains(resource_id) {
                    violations.push(make_violation(
                        ViolationKind::SharedResourceConflict,
                        overlap_start,
                        overlap_end,
                        &[left.step_id.clone(), right.step_id.clone()],
                        2.0,
                        1.0,
                    ));
                }
            }
        }
    }
}

fn make_violation(
    kind: ViolationKind,
    start_offset_seconds: u64,
    end_offset_seconds: u64,
    subjects: &[String],
    actual: f64,
    limit: f64,
) -> ScheduleViolation {
    let mut subject_step_ids = subjects.to_vec();
    subject_step_ids.sort();
    subject_step_ids.dedup();
    ScheduleViolation {
        kind,
        start_offset_seconds,
        end_offset_seconds,
        subject_step_ids,
        actual,
        limit,
    }
}

fn all_finite_non_negative(values: &[f64]) -> bool {
    values
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    fn step(
        step_id: &str,
        service_id: &str,
        start: u64,
        duration: u64,
        transient_load: (f64, f64),
        steady_load: (f64, f64),
        resources: &[&str],
    ) -> RestorationStep {
        RestorationStep {
            step_id: step_id.to_owned(),
            service_id: service_id.to_owned(),
            start_offset_seconds: start,
            duration_seconds: duration,
            prerequisite_step_ids: Vec::new(),
            exclusive_resource_ids: resources.iter().map(|id| (*id).to_owned()).collect(),
            transient_active_kw: transient_load.0,
            transient_reactive_kvar_magnitude: transient_load.1,
            steady_active_kw: steady_load.0,
            steady_reactive_kvar_magnitude: steady_load.1,
            authority_state: AuthorityState::Approved,
        }
    }

    fn scenario(steps: Vec<RestorationStep>) -> RestorationScenario {
        RestorationScenario {
            scenario_id: "gmd-blackstart-synthetic-01".to_owned(),
            owner_ref: "urn:test:operator:regional-grid".to_owned(),
            topology_revision: "island-topology-r3".to_owned(),
            topology_sha256: DIGEST_A.to_owned(),
            procedure_revision: "restoration-procedure-r5".to_owned(),
            procedure_sha256: DIGEST_B.to_owned(),
            horizon_seconds: 3_600,
            baseline_load: BaselineLoad {
                active_kw: 10.0,
                reactive_kvar_magnitude: 5.0,
            },
            capability: CapabilityEnvelope {
                source_ref: "urn:test:qualified-envelope:microgrid-r2".to_owned(),
                source_sha256: DIGEST_A.to_owned(),
                evidence_state: EvidenceState::Qualified,
                maximum_active_kw: 100.0,
                maximum_reactive_kvar: 60.0,
                maximum_apparent_kva: 120.0,
                available_energy_kwh: 100.0,
                reserve_floor_kwh: 20.0,
            },
            exclusive_resources: vec![
                ExclusiveResource { resource_id: "crew-a".to_owned() },
                ExclusiveResource { resource_id: "switching-kit".to_owned() },
            ],
            steps,
        }
    }

    fn two_step_schedule() -> RestorationScenario {
        let hospital = step("hospital", "hospital-critical-power", 0, 600, (30.0, 10.0), (20.0, 10.0), &["crew-a"]);
        let mut water = step("water", "hospital-water", 600, 600, (20.0, 10.0), (40.0, 20.0), &["crew-a"]);
        water.prerequisite_step_ids.push("hospital".to_owned());
        scenario(vec![hospital, water])
    }

    #[test]
    fn known_answer_two_step_schedule_is_feasible_and_energy_balanced() {
        let result = assess_restoration(&two_step_schedule()).unwrap();
        assert_eq!(result.feasibility, FeasibilityStatus::FeasibleAgainstQualifiedEnvelope);
        assert_eq!(result.maximum_active_kw, 70.0);
        assert_eq!(result.maximum_reactive_kvar_magnitude, 35.0);
        assert!((result.maximum_apparent_kva - 78.26237921249263).abs() < 1e-10);
        assert!((result.total_energy_used_kwh - 61.66666666666667).abs() < 1e-10);
        assert!((result.minimum_remaining_energy_kwh - 38.33333333333333).abs() < 1e-10);
        assert!(!result.execution_authorized);
        assert_eq!(result.service_pickups[0].service_id, "hospital-critical-power");
        assert_eq!(result.service_pickups[0].first_planned_pickup_after_seconds, 600);
    }

    #[test]
    fn active_power_exceedance_is_reported_with_exact_interval() {
        let mut input = two_step_schedule();
        input.capability.maximum_active_kw = 60.0;
        let result = assess_restoration(&input).unwrap();
        assert_eq!(result.feasibility, FeasibilityStatus::InfeasibleAgainstQualifiedEnvelope);
        assert!(result.violations.iter().any(|item| item.kind == ViolationKind::ActivePowerExceeded && item.start_offset_seconds == 1_200 && item.end_offset_seconds == 3_600));
    }

    #[test]
    fn reactive_power_exceedance_is_not_hidden_by_active_headroom() {
        let mut input = two_step_schedule();
        input.capability.maximum_active_kw = 100.0;
        input.capability.maximum_reactive_kvar = 30.0;
        let result = assess_restoration(&input).unwrap();
        assert!(result.violations.iter().any(|item| item.kind == ViolationKind::ReactivePowerExceeded && item.start_offset_seconds == 1_200));
    }

    #[test]
    fn apparent_power_exceedance_is_checked_independently() {
        let mut input = two_step_schedule();
        input.capability.maximum_apparent_kva = 75.0;
        let result = assess_restoration(&input).unwrap();
        assert!(result.violations.iter().any(|item| item.kind == ViolationKind::ApparentPowerExceeded && item.start_offset_seconds == 1_200));
    }

    #[test]
    fn energy_reserve_breach_is_reported_when_power_envelope_still_passes() {
        let mut input = two_step_schedule();
        input.capability.maximum_active_kw = 100.0;
        input.capability.maximum_reactive_kvar = 100.0;
        input.capability.maximum_apparent_kva = 150.0;
        input.capability.available_energy_kwh = 60.0;
        input.capability.reserve_floor_kwh = 10.0;
        let result = assess_restoration(&input).unwrap();
        assert!(result.violations.iter().any(|item| item.kind == ViolationKind::EnergyReserveBreached));
    }

    #[test]
    fn overlapping_use_of_one_crew_is_rejected() {
        let first = step("first", "hospital", 0, 600, (10.0, 0.0), (10.0, 0.0), &["crew-a"]);
        let second = step("second", "water", 300, 600, (10.0, 0.0), (10.0, 0.0), &["crew-a"]);
        let result = assess_restoration(&scenario(vec![first, second])).unwrap();
        assert!(result.violations.iter().any(|item| item.kind == ViolationKind::SharedResourceConflict && item.start_offset_seconds == 300 && item.end_offset_seconds == 600));
    }

    #[test]
    fn exact_resource_handoff_at_completion_is_not_an_overlap() {
        let first = step("first", "hospital", 0, 600, (10.0, 0.0), (5.0, 0.0), &["crew-a"]);
        let second = step("second", "water", 600, 600, (10.0, 0.0), (5.0, 0.0), &["crew-a"]);
        let result = assess_restoration(&scenario(vec![first, second])).unwrap();
        assert!(!result.violations.iter().any(|item| item.kind == ViolationKind::SharedResourceConflict));
    }

    #[test]
    fn prerequisite_must_finish_before_dependent_step_starts() {
        let first = step("first", "hospital", 0, 600, (10.0, 0.0), (5.0, 0.0), &[]);
        let mut second = step("second", "water", 300, 300, (10.0, 0.0), (5.0, 0.0), &[]);
        second.prerequisite_step_ids.push("first".to_owned());
        let result = assess_restoration(&scenario(vec![first, second])).unwrap();
        assert!(result.violations.iter().any(|item| item.kind == ViolationKind::DependencyNotFinished));
    }

    #[test]
    fn unknown_prerequisite_is_rejected() {
        let mut item = step("first", "hospital", 0, 600, (10.0, 0.0), (5.0, 0.0), &[]);
        item.prerequisite_step_ids.push("missing".to_owned());
        assert_eq!(assess_restoration(&scenario(vec![item])), Err(RestorationError::UnknownPrerequisite));
    }

    #[test]
    fn cyclic_dependencies_are_rejected() {
        let mut first = step("first", "hospital", 0, 300, (10.0, 0.0), (5.0, 0.0), &[]);
        let mut second = step("second", "water", 300, 300, (10.0, 0.0), (5.0, 0.0), &[]);
        first.prerequisite_step_ids.push("second".to_owned());
        second.prerequisite_step_ids.push("first".to_owned());
        assert_eq!(assess_restoration(&scenario(vec![first, second])), Err(RestorationError::DependencyCycle));
    }

    #[test]
    fn stale_capability_evidence_cannot_produce_a_positive_feasibility_claim() {
        let mut input = two_step_schedule();
        input.capability.evidence_state = EvidenceState::Stale;
        let result = assess_restoration(&input).unwrap();
        assert_eq!(result.feasibility, FeasibilityStatus::IndeterminateCapabilityEvidence);
        assert_eq!(result.operator_readiness, OperatorReadiness::StaleCapabilityEvidence);
    }

    #[test]
    fn unknown_capability_evidence_is_indeterminate() {
        let mut input = two_step_schedule();
        input.capability.evidence_state = EvidenceState::Unknown;
        let result = assess_restoration(&input).unwrap();
        assert_eq!(result.feasibility, FeasibilityStatus::IndeterminateCapabilityEvidence);
        assert_eq!(result.operator_readiness, OperatorReadiness::Indeterminate);
    }

    #[test]
    fn pending_authority_does_not_grant_execution() {
        let mut input = two_step_schedule();
        input.steps[0].authority_state = AuthorityState::Pending;
        let result = assess_restoration(&input).unwrap();
        assert_eq!(result.operator_readiness, OperatorReadiness::AwaitingHumanApproval);
        assert!(!result.execution_authorized);
    }

    #[test]
    fn denied_authority_blocks_operator_readiness() {
        let mut input = two_step_schedule();
        input.steps[0].authority_state = AuthorityState::Denied;
        let result = assess_restoration(&input).unwrap();
        assert_eq!(result.operator_readiness, OperatorReadiness::BlockedByAuthority);
    }

    #[test]
    fn unknown_authority_is_indeterminate_and_preserved() {
        let mut input = two_step_schedule();
        input.steps[0].authority_state = AuthorityState::Unknown;
        let result = assess_restoration(&input).unwrap();
        assert_eq!(result.operator_readiness, OperatorReadiness::Indeterminate);
        assert!(result.step_authorities.iter().any(|state| state.step_id == "hospital" && state.authority_state == AuthorityState::Unknown));
    }

    #[test]
    fn step_input_permutation_produces_identical_assessment() {
        let original = two_step_schedule();
        let mut permuted = original.clone();
        permuted.steps.reverse();
        assert_eq!(assess_restoration(&original).unwrap(), assess_restoration(&permuted).unwrap());
    }

    #[test]
    fn duplicate_resource_ids_are_rejected() {
        let mut input = two_step_schedule();
        input.exclusive_resources.push(ExclusiveResource { resource_id: "crew-a".to_owned() });
        assert_eq!(assess_restoration(&input), Err(RestorationError::DuplicateResourceId));
    }

    #[test]
    fn undeclared_step_resource_is_rejected() {
        let mut input = two_step_schedule();
        input.steps[0].exclusive_resource_ids.push("crew-unknown".to_owned());
        assert_eq!(assess_restoration(&input), Err(RestorationError::UnknownStepResource));
    }

    #[test]
    fn step_beyond_horizon_is_rejected() {
        let item = step("late", "hospital", 3_000, 1_000, (10.0, 0.0), (5.0, 0.0), &[]);
        assert_eq!(assess_restoration(&scenario(vec![item])), Err(RestorationError::StepOutsideHorizon));
    }

    #[test]
    fn zero_duration_step_is_rejected() {
        let item = step("instant", "hospital", 0, 0, (10.0, 0.0), (5.0, 0.0), &[]);
        assert_eq!(assess_restoration(&scenario(vec![item])), Err(RestorationError::InvalidStepDuration));
    }

    #[test]
    fn invalid_digest_is_rejected() {
        let mut input = two_step_schedule();
        input.procedure_sha256 = "bad".to_owned();
        assert_eq!(assess_restoration(&input), Err(RestorationError::InvalidDigest));
    }

    #[test]
    fn negative_or_non_finite_loads_are_rejected() {
        let mut input = two_step_schedule();
        input.steps[0].transient_active_kw = f64::NAN;
        assert_eq!(assess_restoration(&input), Err(RestorationError::InvalidStepLoad));
    }

    #[test]
    fn baseline_power_is_included_in_every_interval() {
        let mut input = two_step_schedule();
        input.steps.clear();
        input.steps.push(step("single", "hospital", 0, 600, (20.0, 0.0), (0.0, 0.0), &[]));
        input.baseline_load.active_kw = 15.0;
        let result = assess_restoration(&input).unwrap();
        assert_eq!(result.maximum_active_kw, 35.0);
    }

    #[test]
    fn reserve_floor_exactly_met_is_not_a_breach() {
        let mut input = two_step_schedule();
        input.capability.available_energy_kwh = 81.66666666666667;
        input.capability.reserve_floor_kwh = 20.0;
        let result = assess_restoration(&input).unwrap();
        assert!(result.minimum_remaining_energy_kwh >= 20.0 - 1e-10);
        assert!(!result.violations.iter().any(|item| item.kind == ViolationKind::EnergyReserveBreached));
    }

    #[test]
    fn empty_schedule_is_rejected() {
        let mut input = two_step_schedule();
        input.steps.clear();
        assert_eq!(assess_restoration(&input), Err(RestorationError::NoSteps));
    }
}
