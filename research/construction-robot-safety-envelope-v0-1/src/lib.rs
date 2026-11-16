// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Portable construction-robot safety-envelope reference evaluator.
//!
//! This crate evaluates a supplied, revision-bound synthetic safety profile.
//! It emits advisory dispositions only. It has no actuator, PLC, emergency-stop,
//! command, authorization, or physical certification interface.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

const MAX_CONFIDENCE_PPM: u32 = 1_000_000;
const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_SENSOR_RECORDS: usize = 128;
const MAX_ODD_CONDITIONS: usize = 64;
const MAX_COVERAGE_CELLS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationalDomain {
    Surface,
    Underground,
    Marine,
    Factory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingMode {
    Manual,
    Supervised,
    Autonomous,
    Maintenance,
    Recovery,
    Emergency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyStopSignal {
    Inactive,
    Active,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensorModality {
    Camera,
    Lidar,
    Radar,
    Localization,
    Inertial,
    Encoder,
    Ultrasonic,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorHealth {
    Healthy,
    Degraded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanState {
    NoHumanDetected,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    WithinSyntheticEnvelope,
    RequireProgressiveMitigation,
    HoldForHumanReview,
    StopRequiredBySyntheticPolicy,
    EvidenceInconsistent,
    UnsupportedOdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestedResponse {
    ContinueAtDeclaredLimit,
    ReduceSpeedAndReassess,
    HoldForHumanReview,
    RequestControlledStop,
    MaintainExternalEmergencyStopLatch,
    DoNotEnterUnsupportedDomain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    EmergencyStopActive,
    EmergencyStopUnknown,
    EmergencyModeRequested,
    EmergencyRecoveryRequiresExternalAuthority,
    PolicyMalformed,
    PolicyDigestMalformed,
    SubjectBindingMismatch,
    DomainMismatch,
    StandardProfileMissing,
    PolicyOutsideValidityWindow,
    ObservationFromFuture,
    StateObservationStale,
    ClockUncertaintyExcessive,
    CommunicationsHeartbeatLost,
    ModeNotPermitted,
    HandoverUnacknowledged,
    OperatorOverrideRequiresReview,
    MaintenanceMotionDetected,
    SensorSetMalformed,
    OddConditionSetMalformed,
    UnexpectedOddCondition,
    CriticalSensorMissing,
    CriticalSensorUnhealthy,
    SensorDiversityInsufficient,
    SensorEvidenceDegraded,
    OddConditionMissing,
    OddConditionStale,
    OddConditionOutsideEnvelope,
    HumanStateUnknown,
    HumanTrackConflicting,
    HumanTrackStale,
    HumanTrackConfidenceInsufficient,
    HumanEvidenceMalformed,
    HumanEvidenceSensorMissing,
    HumanEvidenceSensorUnhealthy,
    HumanEvidenceCoverageInsufficient,
    HumanEvidenceRegionUncovered,
    HumanEvidenceCoverageUnsupported,
    CoverageGridMismatch,
    CoverageCellsMalformed,
    HumanModalityRegionUncovered,
    HumanFaultDomainRegionUncovered,
    HumanModalityDiversityInsufficient,
    HumanSeparationBreached,
    HumanSeparationNearBoundary,
    MotionLimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OddConditionLimit {
    pub condition_id: String,
    pub unit: String,
    pub minimum_milli: i64,
    pub maximum_milli: i64,
    pub maximum_age_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OddConditionObservation {
    pub condition_id: String,
    pub unit: String,
    pub value_milli: i64,
    pub age_ms: u64,
    pub source_sha256: String,
}

/// Cell coordinate in the policy-pinned site coverage grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CoverageCell {
    pub column: u32,
    pub row: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorEvidence {
    pub sensor_id: String,
    pub modality: SensorModality,
    pub health: SensorHealth,
    /// Sensors sharing a fault domain do not count as independent channels.
    pub fault_domain: String,
    pub age_ms: u64,
    pub confidence_ppm: u32,
    pub coverage_ppm: u32,
    pub coverage_grid_sha256: String,
    pub covered_cells: Vec<CoverageCell>,
    pub calibration_current: bool,
    pub source_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HumanDetectionEvidence {
    /// Sensor IDs that actually contributed to this human-presence observation.
    pub contributing_sensor_ids: Vec<String>,
    /// Digest of the source-side detection/coverage receipt, not an authenticity proof.
    pub source_sha256: String,
    pub observed_at_ms: u64,
    pub coverage_ppm: u32,
    pub coverage_grid_sha256: String,
    pub covered_cells: Vec<CoverageCell>,
    pub confidence_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HumanObservation {
    NoHumanDetected,
    Tracked {
        distance_mm: u64,
        position_uncertainty_mm: u64,
        relative_approach_speed_mm_s: u64,
        track_age_ms: u64,
        confidence_ppm: u32,
        conflicting_tracks: bool,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionState {
    pub motion_command_present: bool,
    pub speed_mm_s: u64,
    pub acceleration_mm_s2: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyPolicy {
    pub policy_id: String,
    pub policy_revision: String,
    pub policy_sha256: String,
    pub machine_id: String,
    pub machine_revision: String,
    pub tool_id: String,
    pub tool_revision: String,
    pub task_id: String,
    pub task_revision: String,
    pub site_snapshot_id: String,
    pub coordinate_frame: String,
    /// Digest of the versioned site-local monitoring grid geometry and frame.
    pub coverage_grid_sha256: String,
    /// Required cells in the declared human hazard zone.
    pub required_human_coverage_cells: Vec<CoverageCell>,
    pub jurisdiction: String,
    pub standards_profile: String,
    pub domain: OperationalDomain,
    pub valid_from_ms: u64,
    pub valid_until_ms: u64,
    pub maximum_state_age_ms: u64,
    pub maximum_sensor_age_ms: u64,
    pub maximum_heartbeat_age_ms: u64,
    pub maximum_clock_uncertainty_ms: u64,
    pub maximum_speed_autonomous_mm_s: u64,
    pub maximum_speed_supervised_mm_s: u64,
    pub maximum_speed_manual_mm_s: u64,
    pub maximum_speed_recovery_mm_s: u64,
    pub maximum_acceleration_mm_s2: u64,
    pub reduced_speed_mm_s: u64,
    pub base_separation_mm: u64,
    pub reaction_time_ms: u64,
    pub stopping_distance_mm: u64,
    pub uncertainty_margin_mm: u64,
    pub caution_margin_mm: u64,
    pub minimum_independent_sensor_groups: usize,
    pub minimum_human_sensor_modalities: usize,
    pub minimum_sensor_confidence_ppm: u32,
    pub minimum_sensor_coverage_ppm: u32,
    pub minimum_human_track_confidence_ppm: u32,
    pub permitted_modes: Vec<OperatingMode>,
    pub critical_sensor_ids: Vec<String>,
    pub odd_conditions: Vec<OddConditionLimit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeObservation {
    pub policy_id: String,
    pub policy_revision: String,
    pub policy_sha256: String,
    pub machine_id: String,
    pub machine_revision: String,
    pub tool_id: String,
    pub tool_revision: String,
    pub task_id: String,
    pub task_revision: String,
    pub site_snapshot_id: String,
    pub coordinate_frame: String,
    pub jurisdiction: String,
    pub standards_profile: String,
    pub domain: OperationalDomain,
    pub evaluated_at_ms: u64,
    pub observed_at_ms: u64,
    pub clock_uncertainty_ms: u64,
    pub communications_heartbeat_age_ms: u64,
    pub mode: OperatingMode,
    pub previous_mode: Option<OperatingMode>,
    pub handover_acknowledged: bool,
    pub operator_override: bool,
    pub emergency_stop: EmergencyStopSignal,
    pub motion: MotionState,
    pub human: HumanObservation,
    pub human_detection: HumanDetectionEvidence,
    pub sensors: Vec<SensorEvidence>,
    pub odd_conditions: Vec<OddConditionObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectBinding {
    pub machine_id: String,
    pub machine_revision: String,
    pub tool_id: String,
    pub tool_revision: String,
    pub task_id: String,
    pub task_revision: String,
    pub site_snapshot_id: String,
    pub coordinate_frame: String,
    pub coverage_grid_sha256: String,
    pub jurisdiction: String,
    pub standards_profile: String,
    pub domain: OperationalDomain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceRole {
    Sensor,
    HumanDetection,
    OperatingCondition,
    CoverageGrid,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct EvidenceReference {
    pub role: EvidenceRole,
    pub source_id: String,
    /// Syntactic reference only; no bytes or signatures are verified here.
    pub source_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyDecision {
    pub disposition: Disposition,
    pub requested_response: RequestedResponse,
    pub reasons: Vec<Reason>,
    pub policy_id: String,
    pub policy_revision: String,
    pub policy_sha256: String,
    pub expected_subject: SubjectBinding,
    pub observed_subject: SubjectBinding,
    pub mode: OperatingMode,
    pub evidence_references: Vec<EvidenceReference>,
    pub evaluated_at_ms: u64,
    pub observed_at_ms: u64,
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Validates digest syntax only. It does not authenticate or hash source bytes.
pub fn is_sha256_reference(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn policy_is_well_formed(policy: &SafetyPolicy) -> bool {
    let required_text = [
        policy.policy_id.as_str(),
        policy.policy_revision.as_str(),
        policy.machine_id.as_str(),
        policy.machine_revision.as_str(),
        policy.tool_id.as_str(),
        policy.tool_revision.as_str(),
        policy.task_id.as_str(),
        policy.task_revision.as_str(),
        policy.site_snapshot_id.as_str(),
        policy.coordinate_frame.as_str(),
        policy.jurisdiction.as_str(),
        policy.standards_profile.as_str(),
    ];

    if required_text
        .iter()
        .any(|value| !non_empty(value) || value.len() > MAX_IDENTIFIER_BYTES)
        || !is_sha256_reference(&policy.coverage_grid_sha256)
    {
        return false;
    }
    if policy.valid_from_ms > policy.valid_until_ms
        || policy.odd_conditions.len() > MAX_ODD_CONDITIONS
        || policy.critical_sensor_ids.len() > MAX_SENSOR_RECORDS
        || policy.required_human_coverage_cells.is_empty()
        || policy.required_human_coverage_cells.len() > MAX_COVERAGE_CELLS
        || policy.maximum_state_age_ms == 0
        || policy.maximum_sensor_age_ms == 0
        || policy.maximum_heartbeat_age_ms == 0
        || policy.minimum_independent_sensor_groups == 0
        || policy.minimum_human_sensor_modalities == 0
        || policy.minimum_sensor_confidence_ppm > MAX_CONFIDENCE_PPM
        || policy.minimum_sensor_coverage_ppm > MAX_CONFIDENCE_PPM
        || policy.minimum_human_track_confidence_ppm > MAX_CONFIDENCE_PPM
        || policy.permitted_modes.is_empty()
        || policy.critical_sensor_ids.is_empty()
    {
        return false;
    }

    let mut critical_ids = BTreeSet::new();
    if policy.critical_sensor_ids.iter().any(|id| {
        !non_empty(id) || id.len() > MAX_IDENTIFIER_BYTES || !critical_ids.insert(id.as_str())
    }) {
        return false;
    }

    let mut coverage_cells = BTreeSet::new();
    if policy
        .required_human_coverage_cells
        .iter()
        .any(|cell| !coverage_cells.insert(*cell))
    {
        return false;
    }

    let mut condition_ids = BTreeSet::new();
    policy.odd_conditions.iter().all(|condition| {
        non_empty(&condition.condition_id)
            && condition.condition_id.len() <= MAX_IDENTIFIER_BYTES
            && non_empty(&condition.unit)
            && condition.unit.len() <= MAX_IDENTIFIER_BYTES
            && condition.minimum_milli <= condition.maximum_milli
            && condition.maximum_age_ms > 0
            && condition_ids.insert(condition.condition_id.as_str())
    })
}

fn binding_matches(policy: &SafetyPolicy, observation: &RuntimeObservation) -> bool {
    policy.policy_id == observation.policy_id
        && policy.policy_revision == observation.policy_revision
        && policy.policy_sha256 == observation.policy_sha256
        && policy.machine_id == observation.machine_id
        && policy.machine_revision == observation.machine_revision
        && policy.tool_id == observation.tool_id
        && policy.tool_revision == observation.tool_revision
        && policy.task_id == observation.task_id
        && policy.task_revision == observation.task_revision
        && policy.site_snapshot_id == observation.site_snapshot_id
        && policy.coordinate_frame == observation.coordinate_frame
        && policy.jurisdiction == observation.jurisdiction
        && policy.standards_profile == observation.standards_profile
}

fn mode_speed_limit(policy: &SafetyPolicy, mode: OperatingMode) -> Option<u64> {
    match mode {
        OperatingMode::Autonomous => Some(policy.maximum_speed_autonomous_mm_s),
        OperatingMode::Supervised => Some(policy.maximum_speed_supervised_mm_s),
        OperatingMode::Manual => Some(policy.maximum_speed_manual_mm_s),
        OperatingMode::Recovery => Some(policy.maximum_speed_recovery_mm_s),
        OperatingMode::Maintenance | OperatingMode::Emergency => Some(0),
    }
}

fn make_decision(
    policy: &SafetyPolicy,
    observation: &RuntimeObservation,
    disposition: Disposition,
    requested_response: RequestedResponse,
    reasons: Vec<Reason>,
) -> SafetyDecision {
    let mut evidence_references = observation
        .sensors
        .iter()
        .map(|sensor| EvidenceReference {
            role: EvidenceRole::Sensor,
            source_id: sensor.sensor_id.clone(),
            source_sha256: sensor.source_sha256.clone(),
        })
        .collect::<Vec<_>>();
    evidence_references.push(EvidenceReference {
        role: EvidenceRole::CoverageGrid,
        source_id: "site-human-monitoring-grid".to_owned(),
        source_sha256: policy.coverage_grid_sha256.clone(),
    });
    evidence_references.push(EvidenceReference {
        role: EvidenceRole::HumanDetection,
        source_id: "human-detection".to_owned(),
        source_sha256: observation.human_detection.source_sha256.clone(),
    });
    evidence_references.extend(observation.odd_conditions.iter().map(|condition| {
        EvidenceReference {
            role: EvidenceRole::OperatingCondition,
            source_id: condition.condition_id.clone(),
            source_sha256: condition.source_sha256.clone(),
        }
    }));
    evidence_references.sort();

    SafetyDecision {
        disposition,
        requested_response,
        reasons,
        policy_id: policy.policy_id.clone(),
        policy_revision: policy.policy_revision.clone(),
        policy_sha256: policy.policy_sha256.clone(),
        expected_subject: SubjectBinding {
            machine_id: policy.machine_id.clone(),
            machine_revision: policy.machine_revision.clone(),
            tool_id: policy.tool_id.clone(),
            tool_revision: policy.tool_revision.clone(),
            task_id: policy.task_id.clone(),
            task_revision: policy.task_revision.clone(),
            site_snapshot_id: policy.site_snapshot_id.clone(),
            coordinate_frame: policy.coordinate_frame.clone(),
            coverage_grid_sha256: policy.coverage_grid_sha256.clone(),
            jurisdiction: policy.jurisdiction.clone(),
            standards_profile: policy.standards_profile.clone(),
            domain: policy.domain,
        },
        observed_subject: SubjectBinding {
            machine_id: observation.machine_id.clone(),
            machine_revision: observation.machine_revision.clone(),
            tool_id: observation.tool_id.clone(),
            tool_revision: observation.tool_revision.clone(),
            task_id: observation.task_id.clone(),
            task_revision: observation.task_revision.clone(),
            site_snapshot_id: observation.site_snapshot_id.clone(),
            coordinate_frame: observation.coordinate_frame.clone(),
            coverage_grid_sha256: observation.human_detection.coverage_grid_sha256.clone(),
            jurisdiction: observation.jurisdiction.clone(),
            standards_profile: observation.standards_profile.clone(),
            domain: observation.domain,
        },
        mode: observation.mode,
        evidence_references,
        evaluated_at_ms: observation.evaluated_at_ms,
        observed_at_ms: observation.observed_at_ms,
    }
}

fn stop(policy: &SafetyPolicy, observation: &RuntimeObservation, reason: Reason) -> SafetyDecision {
    make_decision(
        policy,
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
        if observation.emergency_stop == EmergencyStopSignal::Active {
            RequestedResponse::MaintainExternalEmergencyStopLatch
        } else {
            RequestedResponse::RequestControlledStop
        },
        vec![reason],
    )
}

fn hold(policy: &SafetyPolicy, observation: &RuntimeObservation, reason: Reason) -> SafetyDecision {
    make_decision(
        policy,
        observation,
        Disposition::HoldForHumanReview,
        RequestedResponse::HoldForHumanReview,
        vec![reason],
    )
}

fn unsupported(
    policy: &SafetyPolicy,
    observation: &RuntimeObservation,
    reason: Reason,
) -> SafetyDecision {
    make_decision(
        policy,
        observation,
        Disposition::UnsupportedOdd,
        RequestedResponse::DoNotEnterUnsupportedDomain,
        vec![reason],
    )
}

fn inconsistent(
    policy: &SafetyPolicy,
    observation: &RuntimeObservation,
    reason: Reason,
) -> SafetyDecision {
    make_decision(
        policy,
        observation,
        Disposition::EvidenceInconsistent,
        RequestedResponse::HoldForHumanReview,
        vec![reason],
    )
}

fn conditions_are_unique(observation: &RuntimeObservation) -> bool {
    let mut ids = BTreeSet::new();
    observation
        .odd_conditions
        .iter()
        .all(|condition| ids.insert(condition.condition_id.as_str()))
}

fn sensors_are_well_formed(observation: &RuntimeObservation) -> bool {
    if observation.sensors.len() > MAX_SENSOR_RECORDS {
        return false;
    }
    let mut ids = BTreeSet::new();
    observation.sensors.iter().all(|sensor| {
        non_empty(&sensor.sensor_id)
            && sensor.sensor_id.len() <= MAX_IDENTIFIER_BYTES
            && non_empty(&sensor.fault_domain)
            && sensor.fault_domain.len() <= MAX_IDENTIFIER_BYTES
            && is_sha256_reference(&sensor.source_sha256)
            && is_sha256_reference(&sensor.coverage_grid_sha256)
            && sensor.covered_cells.len() <= MAX_COVERAGE_CELLS
            && sensor
                .covered_cells
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                == sensor.covered_cells.len()
            && sensor.confidence_ppm <= MAX_CONFIDENCE_PPM
            && sensor.coverage_ppm <= MAX_CONFIDENCE_PPM
            && ids.insert(sensor.sensor_id.as_str())
    })
}

fn human_evidence_status(
    policy: &SafetyPolicy,
    observation: &RuntimeObservation,
) -> Result<(), Reason> {
    let evidence = &observation.human_detection;
    if evidence.contributing_sensor_ids.is_empty()
        || evidence.contributing_sensor_ids.len() > MAX_SENSOR_RECORDS
        || !is_sha256_reference(&evidence.source_sha256)
        || evidence.coverage_ppm > MAX_CONFIDENCE_PPM
        || evidence.confidence_ppm > MAX_CONFIDENCE_PPM
        || evidence.covered_cells.len() > MAX_COVERAGE_CELLS
    {
        return Err(Reason::HumanEvidenceMalformed);
    }
    if evidence.coverage_grid_sha256 != policy.coverage_grid_sha256 {
        return Err(Reason::CoverageGridMismatch);
    }
    if evidence.observed_at_ms > observation.evaluated_at_ms {
        return Err(Reason::HumanEvidenceMalformed);
    }
    if observation
        .evaluated_at_ms
        .saturating_sub(evidence.observed_at_ms)
        > policy.maximum_sensor_age_ms
    {
        return Err(Reason::HumanTrackStale);
    }
    if evidence.coverage_ppm < policy.minimum_sensor_coverage_ppm {
        return Err(Reason::HumanEvidenceCoverageInsufficient);
    }
    if evidence.confidence_ppm < policy.minimum_sensor_confidence_ppm {
        return Err(Reason::HumanTrackConfidenceInsufficient);
    }

    let required_cells = policy
        .required_human_coverage_cells
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let detected_cells = evidence
        .covered_cells
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if detected_cells.len() != evidence.covered_cells.len()
        || detected_cells
            .iter()
            .any(|cell| !required_cells.contains(cell))
    {
        return Err(Reason::CoverageCellsMalformed);
    }
    if !required_cells.is_subset(&detected_cells) {
        return Err(Reason::HumanEvidenceRegionUncovered);
    }

    let mut sensor_ids = BTreeSet::new();
    let mut fault_domain_coverage = BTreeMap::<String, BTreeSet<CoverageCell>>::new();
    let mut modality_coverage = BTreeMap::<SensorModality, BTreeSet<CoverageCell>>::new();
    let mut source_covered_cells = BTreeSet::new();

    for sensor_id in &evidence.contributing_sensor_ids {
        if !non_empty(sensor_id)
            || sensor_id.len() > MAX_IDENTIFIER_BYTES
            || !sensor_ids.insert(sensor_id.as_str())
        {
            return Err(Reason::HumanEvidenceMalformed);
        }
        let Some(sensor) = observation
            .sensors
            .iter()
            .find(|sensor| sensor.sensor_id == *sensor_id)
        else {
            return Err(Reason::HumanEvidenceSensorMissing);
        };
        if sensor.health != SensorHealth::Healthy
            || !sensor.calibration_current
            || sensor.age_ms > policy.maximum_sensor_age_ms
            || sensor.confidence_ppm < policy.minimum_sensor_confidence_ppm
            || sensor.coverage_ppm < policy.minimum_sensor_coverage_ppm
        {
            return Err(Reason::HumanEvidenceSensorUnhealthy);
        }
        if sensor.coverage_grid_sha256 != policy.coverage_grid_sha256 {
            return Err(Reason::CoverageGridMismatch);
        }

        let sensor_cells = sensor
            .covered_cells
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if sensor_cells.len() != sensor.covered_cells.len()
            || sensor_cells
                .iter()
                .any(|cell| !required_cells.contains(cell))
        {
            return Err(Reason::CoverageCellsMalformed);
        }
        source_covered_cells.extend(sensor_cells.iter().copied());
        fault_domain_coverage
            .entry(sensor.fault_domain.clone())
            .or_default()
            .extend(sensor_cells.iter().copied());
        modality_coverage
            .entry(sensor.modality)
            .or_default()
            .extend(sensor_cells.iter().copied());
    }

    if !detected_cells.is_subset(&source_covered_cells) {
        return Err(Reason::HumanEvidenceCoverageUnsupported);
    }

    let complete_fault_domains = fault_domain_coverage
        .values()
        .filter(|cells| required_cells.is_subset(*cells))
        .count();
    if complete_fault_domains < policy.minimum_independent_sensor_groups {
        return Err(Reason::HumanFaultDomainRegionUncovered);
    }

    let complete_modalities = modality_coverage
        .values()
        .filter(|cells| required_cells.is_subset(*cells))
        .count();
    if complete_modalities < policy.minimum_human_sensor_modalities {
        return Err(Reason::HumanModalityRegionUncovered);
    }
    Ok(())
}

fn condition_is_supported(
    policy: &SafetyPolicy,
    observation: &RuntimeObservation,
) -> Result<(), Reason> {
    if observation.odd_conditions.len() > MAX_ODD_CONDITIONS || !conditions_are_unique(observation)
    {
        return Err(Reason::OddConditionSetMalformed);
    }
    for value in &observation.odd_conditions {
        if !non_empty(&value.condition_id)
            || value.condition_id.len() > MAX_IDENTIFIER_BYTES
            || !non_empty(&value.unit)
            || value.unit.len() > MAX_IDENTIFIER_BYTES
            || !is_sha256_reference(&value.source_sha256)
        {
            return Err(Reason::OddConditionSetMalformed);
        }
    }

    for value in &observation.odd_conditions {
        if !policy
            .odd_conditions
            .iter()
            .any(|limit| limit.condition_id == value.condition_id)
        {
            return Err(Reason::UnexpectedOddCondition);
        }
    }

    for limit in &policy.odd_conditions {
        let mut matches = observation
            .odd_conditions
            .iter()
            .filter(|condition| condition.condition_id == limit.condition_id);
        let Some(value) = matches.next() else {
            return Err(Reason::OddConditionMissing);
        };
        if value.unit != limit.unit {
            return Err(Reason::OddConditionSetMalformed);
        }
        if value.age_ms > limit.maximum_age_ms {
            return Err(Reason::OddConditionStale);
        }
        if value.value_milli < limit.minimum_milli || value.value_milli > limit.maximum_milli {
            return Err(Reason::OddConditionOutsideEnvelope);
        }
    }
    Ok(())
}

fn sensor_status(policy: &SafetyPolicy, observation: &RuntimeObservation) -> Result<bool, Reason> {
    if !sensors_are_well_formed(observation) {
        return Err(Reason::SensorSetMalformed);
    }

    let mut independent_healthy_groups = BTreeSet::new();
    let mut degraded_noncritical_sensor = false;

    for critical_id in &policy.critical_sensor_ids {
        if !observation
            .sensors
            .iter()
            .any(|sensor| sensor.sensor_id == *critical_id)
        {
            return Err(Reason::CriticalSensorMissing);
        }
    }

    for sensor in &observation.sensors {
        let usable = sensor.health == SensorHealth::Healthy
            && sensor.calibration_current
            && sensor.age_ms <= policy.maximum_sensor_age_ms
            && sensor.confidence_ppm >= policy.minimum_sensor_confidence_ppm
            && sensor.coverage_ppm >= policy.minimum_sensor_coverage_ppm;

        let critical = policy
            .critical_sensor_ids
            .iter()
            .any(|critical_id| critical_id == &sensor.sensor_id);

        if critical && !usable {
            return Err(Reason::CriticalSensorUnhealthy);
        }
        if usable {
            independent_healthy_groups.insert(sensor.fault_domain.as_str());
        } else {
            degraded_noncritical_sensor = true;
        }
    }

    if independent_healthy_groups.len() < policy.minimum_independent_sensor_groups {
        return Err(Reason::SensorDiversityInsufficient);
    }
    Ok(degraded_noncritical_sensor)
}

/// Evaluates one runtime observation against one exact synthetic policy profile.
///
/// This is an advisory evidence evaluator, not a safety function. It does not
/// issue commands, stop equipment, reset an emergency stop, or authorize work.
pub fn evaluate(policy: &SafetyPolicy, observation: &RuntimeObservation) -> SafetyDecision {
    // External hard-stop state takes precedence over uncertainty elsewhere.
    match observation.emergency_stop {
        EmergencyStopSignal::Active => {
            return stop(policy, observation, Reason::EmergencyStopActive);
        }
        EmergencyStopSignal::Unknown => {
            return stop(policy, observation, Reason::EmergencyStopUnknown);
        }
        EmergencyStopSignal::Inactive => {}
    }

    if observation.mode == OperatingMode::Emergency {
        return stop(policy, observation, Reason::EmergencyModeRequested);
    }
    if observation.mode == OperatingMode::Maintenance
        && (observation.motion.motion_command_present || observation.motion.speed_mm_s > 0)
    {
        return stop(policy, observation, Reason::MaintenanceMotionDetected);
    }

    if !non_empty(&policy.standards_profile) || !non_empty(&policy.jurisdiction) {
        return unsupported(policy, observation, Reason::StandardProfileMissing);
    }
    if !policy_is_well_formed(policy) {
        return unsupported(policy, observation, Reason::PolicyMalformed);
    }
    if !is_sha256_reference(&policy.policy_sha256) {
        return inconsistent(policy, observation, Reason::PolicyDigestMalformed);
    }
    if !binding_matches(policy, observation) {
        return inconsistent(policy, observation, Reason::SubjectBindingMismatch);
    }
    if observation.domain != policy.domain {
        return unsupported(policy, observation, Reason::DomainMismatch);
    }

    if observation.evaluated_at_ms < policy.valid_from_ms
        || observation.evaluated_at_ms > policy.valid_until_ms
    {
        return unsupported(policy, observation, Reason::PolicyOutsideValidityWindow);
    }
    if observation.observed_at_ms
        > observation
            .evaluated_at_ms
            .saturating_add(observation.clock_uncertainty_ms)
    {
        return inconsistent(policy, observation, Reason::ObservationFromFuture);
    }
    if observation
        .evaluated_at_ms
        .saturating_sub(observation.observed_at_ms)
        > policy.maximum_state_age_ms
    {
        return stop(policy, observation, Reason::StateObservationStale);
    }
    if observation.clock_uncertainty_ms > policy.maximum_clock_uncertainty_ms {
        return stop(policy, observation, Reason::ClockUncertaintyExcessive);
    }
    if observation.communications_heartbeat_age_ms > policy.maximum_heartbeat_age_ms {
        return stop(policy, observation, Reason::CommunicationsHeartbeatLost);
    }
    if !policy.permitted_modes.contains(&observation.mode) {
        return unsupported(policy, observation, Reason::ModeNotPermitted);
    }

    let degraded_sensors = match sensor_status(policy, observation) {
        Ok(degraded) => degraded,
        Err(Reason::SensorSetMalformed) => {
            return inconsistent(policy, observation, Reason::SensorSetMalformed);
        }
        Err(reason) => return stop(policy, observation, reason),
    };

    if let Err(reason) = human_evidence_status(policy, observation) {
        if matches!(
            reason,
            Reason::HumanEvidenceMalformed
                | Reason::CoverageGridMismatch
                | Reason::CoverageCellsMalformed
                | Reason::HumanEvidenceCoverageUnsupported
        ) {
            return inconsistent(policy, observation, reason);
        }
        return stop(policy, observation, reason);
    }

    // Evaluate hard proximity and motion limits before ODD and mitigation
    // classifications. A missing environmental field cannot hide a hard stop.
    let mut near_separation_boundary = false;
    match observation.human {
        HumanObservation::NoHumanDetected => {}
        HumanObservation::Unknown => {
            return stop(policy, observation, Reason::HumanStateUnknown);
        }
        HumanObservation::Tracked {
            distance_mm,
            position_uncertainty_mm,
            relative_approach_speed_mm_s,
            track_age_ms,
            confidence_ppm,
            conflicting_tracks,
        } => {
            if conflicting_tracks {
                return stop(policy, observation, Reason::HumanTrackConflicting);
            }
            if track_age_ms > policy.maximum_sensor_age_ms {
                return stop(policy, observation, Reason::HumanTrackStale);
            }
            if confidence_ppm > MAX_CONFIDENCE_PPM {
                return inconsistent(policy, observation, Reason::SensorSetMalformed);
            }
            if confidence_ppm < policy.minimum_human_track_confidence_ppm {
                return stop(
                    policy,
                    observation,
                    Reason::HumanTrackConfidenceInsufficient,
                );
            }

            let approach_distance = (relative_approach_speed_mm_s as u128)
                .saturating_mul(policy.reaction_time_ms as u128)
                .saturating_add(999)
                / 1_000;
            let required_distance = (policy.base_separation_mm as u128)
                .saturating_add(policy.stopping_distance_mm as u128)
                .saturating_add(policy.uncertainty_margin_mm as u128)
                .saturating_add(approach_distance);
            let conservative_distance = distance_mm.saturating_sub(position_uncertainty_mm) as u128;

            if conservative_distance <= required_distance {
                return stop(policy, observation, Reason::HumanSeparationBreached);
            }
            if conservative_distance
                <= required_distance.saturating_add(policy.caution_margin_mm as u128)
            {
                near_separation_boundary = true;
            }
        }
    }

    let Some(maximum_speed) = mode_speed_limit(policy, observation.mode) else {
        return unsupported(policy, observation, Reason::ModeNotPermitted);
    };
    if observation.motion.speed_mm_s > maximum_speed
        || observation.motion.acceleration_mm_s2 > policy.maximum_acceleration_mm_s2
    {
        return stop(policy, observation, Reason::MotionLimitExceeded);
    }

    match condition_is_supported(policy, observation) {
        Ok(()) => {}
        Err(Reason::SensorSetMalformed) => {
            return inconsistent(policy, observation, Reason::SensorSetMalformed);
        }
        Err(Reason::OddConditionSetMalformed) => {
            return inconsistent(policy, observation, Reason::OddConditionSetMalformed);
        }
        Err(reason) => return unsupported(policy, observation, reason),
    }

    // A stop-to-operation transition is never performed by this module.
    if observation.previous_mode == Some(OperatingMode::Emergency)
        && observation.mode != OperatingMode::Emergency
    {
        return hold(
            policy,
            observation,
            Reason::EmergencyRecoveryRequiresExternalAuthority,
        );
    }
    if observation.previous_mode.is_some()
        && observation.previous_mode != Some(observation.mode)
        && !observation.handover_acknowledged
    {
        return hold(policy, observation, Reason::HandoverUnacknowledged);
    }
    if observation.operator_override {
        return hold(policy, observation, Reason::OperatorOverrideRequiresReview);
    }

    if degraded_sensors || near_separation_boundary {
        let reason = if near_separation_boundary {
            Reason::HumanSeparationNearBoundary
        } else {
            Reason::SensorEvidenceDegraded
        };
        let response = if observation.motion.speed_mm_s > policy.reduced_speed_mm_s {
            RequestedResponse::ReduceSpeedAndReassess
        } else {
            RequestedResponse::HoldForHumanReview
        };
        return make_decision(
            policy,
            observation,
            Disposition::RequireProgressiveMitigation,
            response,
            vec![reason],
        );
    }

    make_decision(
        policy,
        observation,
        Disposition::WithinSyntheticEnvelope,
        RequestedResponse::ContinueAtDeclaredLimit,
        Vec::new(),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticCase {
    pub id: &'static str,
    pub policy: SafetyPolicy,
    pub observation: RuntimeObservation,
    /// Independently pinned expected disposition; never generated by evaluate().
    pub expected: Disposition,
}

fn add_case(
    cases: &mut Vec<SyntheticCase>,
    id: &'static str,
    policy: SafetyPolicy,
    observation: RuntimeObservation,
    expected: Disposition,
) {
    cases.push(SyntheticCase {
        id,
        policy,
        observation,
        expected,
    });
}

fn digest(character: char) -> String {
    character.to_string().repeat(64)
}

fn baseline() -> (SafetyPolicy, RuntimeObservation) {
    let policy = SafetyPolicy {
        policy_id: "construction-safety-surface-v1".to_owned(),
        policy_revision: "revision-1".to_owned(),
        policy_sha256: digest('a'),
        machine_id: "machine-01".to_owned(),
        machine_revision: "as-built-07".to_owned(),
        tool_id: "tool-03".to_owned(),
        tool_revision: "tool-revision-4".to_owned(),
        task_id: "work-package-44".to_owned(),
        task_revision: "work-revision-2".to_owned(),
        site_snapshot_id: "site-grid-12".to_owned(),
        coordinate_frame: "site-local-mm".to_owned(),
        coverage_grid_sha256: digest('2'),
        required_human_coverage_cells: vec![
            CoverageCell { column: 0, row: 0 },
            CoverageCell { column: 0, row: 1 },
            CoverageCell { column: 1, row: 0 },
            CoverageCell { column: 1, row: 1 },
        ],
        jurisdiction: "ZA".to_owned(),
        standards_profile: "za-construction-profile-2026-01".to_owned(),
        domain: OperationalDomain::Surface,
        valid_from_ms: 1_000,
        valid_until_ms: 10_000,
        maximum_state_age_ms: 1_000,
        maximum_sensor_age_ms: 250,
        maximum_heartbeat_age_ms: 500,
        maximum_clock_uncertainty_ms: 50,
        maximum_speed_autonomous_mm_s: 1_500,
        maximum_speed_supervised_mm_s: 2_000,
        maximum_speed_manual_mm_s: 300,
        maximum_speed_recovery_mm_s: 100,
        maximum_acceleration_mm_s2: 500,
        reduced_speed_mm_s: 250,
        base_separation_mm: 1_200,
        reaction_time_ms: 500,
        stopping_distance_mm: 800,
        uncertainty_margin_mm: 250,
        caution_margin_mm: 1_000,
        minimum_independent_sensor_groups: 2,
        minimum_human_sensor_modalities: 2,
        minimum_sensor_confidence_ppm: 800_000,
        minimum_sensor_coverage_ppm: 900_000,
        minimum_human_track_confidence_ppm: 900_000,
        permitted_modes: vec![
            OperatingMode::Manual,
            OperatingMode::Supervised,
            OperatingMode::Autonomous,
            OperatingMode::Maintenance,
            OperatingMode::Recovery,
        ],
        critical_sensor_ids: vec!["lidar-front".to_owned(), "localization-unit".to_owned()],
        odd_conditions: vec![
            OddConditionLimit {
                condition_id: "visibility".to_owned(),
                unit: "mm".to_owned(),
                minimum_milli: 5_000,
                maximum_milli: 200_000,
                maximum_age_ms: 500,
            },
            OddConditionLimit {
                condition_id: "wind-speed".to_owned(),
                unit: "mm/s".to_owned(),
                minimum_milli: 0,
                maximum_milli: 15_000,
                maximum_age_ms: 500,
            },
        ],
    };

    let observation = RuntimeObservation {
        policy_id: policy.policy_id.clone(),
        policy_revision: policy.policy_revision.clone(),
        policy_sha256: policy.policy_sha256.clone(),
        machine_id: policy.machine_id.clone(),
        machine_revision: policy.machine_revision.clone(),
        tool_id: policy.tool_id.clone(),
        tool_revision: policy.tool_revision.clone(),
        task_id: policy.task_id.clone(),
        task_revision: policy.task_revision.clone(),
        site_snapshot_id: policy.site_snapshot_id.clone(),
        coordinate_frame: policy.coordinate_frame.clone(),
        jurisdiction: policy.jurisdiction.clone(),
        standards_profile: policy.standards_profile.clone(),
        domain: policy.domain,
        evaluated_at_ms: 5_000,
        observed_at_ms: 4_990,
        clock_uncertainty_ms: 10,
        communications_heartbeat_age_ms: 100,
        mode: OperatingMode::Autonomous,
        previous_mode: None,
        handover_acknowledged: true,
        operator_override: false,
        emergency_stop: EmergencyStopSignal::Inactive,
        motion: MotionState {
            motion_command_present: true,
            speed_mm_s: 900,
            acceleration_mm_s2: 100,
        },
        human: HumanObservation::Tracked {
            distance_mm: 6_000,
            position_uncertainty_mm: 100,
            relative_approach_speed_mm_s: 100,
            track_age_ms: 10,
            confidence_ppm: 980_000,
            conflicting_tracks: false,
        },
        human_detection: HumanDetectionEvidence {
            contributing_sensor_ids: vec!["lidar-front".to_owned(), "camera-front".to_owned()],
            source_sha256: digest('9'),
            observed_at_ms: 4_990,
            coverage_ppm: 980_000,
            coverage_grid_sha256: digest('2'),
            covered_cells: vec![
                CoverageCell { column: 0, row: 0 },
                CoverageCell { column: 0, row: 1 },
                CoverageCell { column: 1, row: 0 },
                CoverageCell { column: 1, row: 1 },
            ],
            confidence_ppm: 980_000,
        },
        sensors: vec![
            SensorEvidence {
                sensor_id: "lidar-front".to_owned(),
                modality: SensorModality::Lidar,
                health: SensorHealth::Healthy,
                fault_domain: "range-power-a".to_owned(),
                age_ms: 20,
                confidence_ppm: 980_000,
                coverage_ppm: 980_000,
                coverage_grid_sha256: digest('2'),
                covered_cells: vec![
                    CoverageCell { column: 0, row: 0 },
                    CoverageCell { column: 0, row: 1 },
                    CoverageCell { column: 1, row: 0 },
                    CoverageCell { column: 1, row: 1 },
                ],
                calibration_current: true,
                source_sha256: digest('b'),
            },
            SensorEvidence {
                sensor_id: "localization-unit".to_owned(),
                modality: SensorModality::Localization,
                health: SensorHealth::Healthy,
                fault_domain: "navigation-power-b".to_owned(),
                age_ms: 20,
                confidence_ppm: 980_000,
                coverage_ppm: 980_000,
                coverage_grid_sha256: digest('2'),
                covered_cells: Vec::new(),
                calibration_current: true,
                source_sha256: digest('c'),
            },
            SensorEvidence {
                sensor_id: "camera-front".to_owned(),
                modality: SensorModality::Camera,
                health: SensorHealth::Healthy,
                fault_domain: "perception-power-c".to_owned(),
                age_ms: 20,
                confidence_ppm: 980_000,
                coverage_ppm: 980_000,
                coverage_grid_sha256: digest('2'),
                covered_cells: vec![
                    CoverageCell { column: 0, row: 0 },
                    CoverageCell { column: 0, row: 1 },
                    CoverageCell { column: 1, row: 0 },
                    CoverageCell { column: 1, row: 1 },
                ],
                calibration_current: true,
                source_sha256: digest('d'),
            },
            SensorEvidence {
                sensor_id: "tool-encoder".to_owned(),
                modality: SensorModality::Encoder,
                health: SensorHealth::Healthy,
                fault_domain: "tool-feedback-power-d".to_owned(),
                age_ms: 20,
                confidence_ppm: 980_000,
                coverage_ppm: 980_000,
                coverage_grid_sha256: digest('2'),
                covered_cells: Vec::new(),
                calibration_current: true,
                source_sha256: digest('1'),
            },
        ],
        odd_conditions: vec![
            OddConditionObservation {
                condition_id: "visibility".to_owned(),
                unit: "mm".to_owned(),
                value_milli: 50_000,
                age_ms: 20,
                source_sha256: digest('e'),
            },
            OddConditionObservation {
                condition_id: "wind-speed".to_owned(),
                unit: "mm/s".to_owned(),
                value_milli: 5_000,
                age_ms: 20,
                source_sha256: digest('f'),
            },
        ],
    };

    (policy, observation)
}

/// Frozen synthetic corpus for exact disposition checks.
///
/// Values are invented, explicitly synthetic test inputs. Expected results are
/// pinned here independently of the evaluator and are not empirical rates.
pub fn frozen_cases() -> Vec<SyntheticCase> {
    let (base_policy, base_observation) = baseline();
    let mut cases = Vec::new();

    add_case(
        &mut cases,
        "SAFE-001-clear-baseline",
        base_policy.clone(),
        base_observation.clone(),
        Disposition::WithinSyntheticEnvelope,
    );

    let mut observation = base_observation.clone();
    observation.human = HumanObservation::Tracked {
        distance_mm: 3_200,
        position_uncertainty_mm: 100,
        relative_approach_speed_mm_s: 100,
        track_age_ms: 10,
        confidence_ppm: 980_000,
        conflicting_tracks: false,
    };
    add_case(
        &mut cases,
        "SAFE-002-near-separation-boundary",
        base_policy.clone(),
        observation,
        Disposition::RequireProgressiveMitigation,
    );

    let mut observation = base_observation.clone();
    observation.human = HumanObservation::Tracked {
        distance_mm: 2_200,
        position_uncertainty_mm: 100,
        relative_approach_speed_mm_s: 100,
        track_age_ms: 10,
        confidence_ppm: 980_000,
        conflicting_tracks: false,
    };
    add_case(
        &mut cases,
        "SAFE-003-separation-breach",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.sensors[1].age_ms = 800;
    add_case(
        &mut cases,
        "SAFE-004-stale-critical-localization",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.sensors[3].health = SensorHealth::Degraded;
    observation.sensors[3].confidence_ppm = 500_000;
    add_case(
        &mut cases,
        "SAFE-005-degraded-peripheral-sensor-independent-human-sensing",
        base_policy.clone(),
        observation,
        Disposition::RequireProgressiveMitigation,
    );

    let mut observation = base_observation.clone();
    for sensor in &mut observation.sensors {
        sensor.fault_domain = "shared-power-domain".to_owned();
    }
    add_case(
        &mut cases,
        "SAFE-006-common-mode-sensor-outage",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    if let HumanObservation::Tracked {
        conflicting_tracks, ..
    } = &mut observation.human
    {
        *conflicting_tracks = true;
    }
    add_case(
        &mut cases,
        "SAFE-007-contradictory-human-tracks",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.motion.speed_mm_s = 1_600;
    add_case(
        &mut cases,
        "SAFE-008-autonomous-overspeed",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut policy = base_policy.clone();
    policy.maximum_speed_autonomous_mm_s = 400;
    let mut observation = base_observation.clone();
    observation.motion.speed_mm_s = 600;
    add_case(
        &mut cases,
        "SAFE-009-precision-task-speed-envelope",
        policy,
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.tool_revision = "wrong-tool-revision".to_owned();
    add_case(
        &mut cases,
        "SAFE-010-wrong-tool-revision",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.machine_revision = "wrong-as-built-revision".to_owned();
    add_case(
        &mut cases,
        "SAFE-011-wrong-as-built-revision",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.site_snapshot_id = "stale-site-snapshot".to_owned();
    add_case(
        &mut cases,
        "SAFE-012-site-snapshot-mismatch",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.domain = OperationalDomain::Underground;
    add_case(
        &mut cases,
        "SAFE-013-underground-on-surface-profile",
        base_policy.clone(),
        observation,
        Disposition::UnsupportedOdd,
    );

    let mut policy = base_policy.clone();
    policy.domain = OperationalDomain::Marine;
    policy.odd_conditions.push(OddConditionLimit {
        condition_id: "wave-height".to_owned(),
        unit: "mm".to_owned(),
        minimum_milli: 0,
        maximum_milli: 1_000,
        maximum_age_ms: 500,
    });
    let mut observation = base_observation.clone();
    observation.domain = OperationalDomain::Marine;
    observation.odd_conditions.push(OddConditionObservation {
        condition_id: "wave-height".to_owned(),
        unit: "mm".to_owned(),
        value_milli: 2_000,
        age_ms: 20,
        source_sha256: digest('7'),
    });
    add_case(
        &mut cases,
        "SAFE-014-marine-wave-outside-envelope",
        policy,
        observation,
        Disposition::UnsupportedOdd,
    );

    let mut policy = base_policy.clone();
    policy.odd_conditions.push(OddConditionLimit {
        condition_id: "dust-obscuration".to_owned(),
        unit: "ppm".to_owned(),
        minimum_milli: 0,
        maximum_milli: 200_000,
        maximum_age_ms: 500,
    });
    let mut observation = base_observation.clone();
    observation.odd_conditions.push(OddConditionObservation {
        condition_id: "dust-obscuration".to_owned(),
        unit: "ppm".to_owned(),
        value_milli: 300_000,
        age_ms: 20,
        source_sha256: digest('8'),
    });
    add_case(
        &mut cases,
        "SAFE-015-dust-outside-envelope",
        policy,
        observation,
        Disposition::UnsupportedOdd,
    );

    let mut observation = base_observation.clone();
    observation.sensors[3].calibration_current = false;
    add_case(
        &mut cases,
        "SAFE-016-noncritical-tool-encoder-calibration-expired",
        base_policy.clone(),
        observation,
        Disposition::RequireProgressiveMitigation,
    );

    let mut observation = base_observation.clone();
    observation.communications_heartbeat_age_ms = 900;
    add_case(
        &mut cases,
        "SAFE-017-communications-heartbeat-loss",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.clock_uncertainty_ms = 100;
    add_case(
        &mut cases,
        "SAFE-018-excessive-clock-uncertainty",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.observed_at_ms = 5_100;
    add_case(
        &mut cases,
        "SAFE-019-future-runtime-observation",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.mode = OperatingMode::Maintenance;
    observation.motion.motion_command_present = true;
    add_case(
        &mut cases,
        "SAFE-020-maintenance-mode-motion-command",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.previous_mode = Some(OperatingMode::Autonomous);
    observation.mode = OperatingMode::Supervised;
    observation.handover_acknowledged = false;
    add_case(
        &mut cases,
        "SAFE-021-unacknowledged-mode-handover",
        base_policy.clone(),
        observation,
        Disposition::HoldForHumanReview,
    );

    let mut observation = base_observation.clone();
    observation.operator_override = true;
    add_case(
        &mut cases,
        "SAFE-022-operator-override-review",
        base_policy.clone(),
        observation,
        Disposition::HoldForHumanReview,
    );

    let mut observation = base_observation.clone();
    observation.emergency_stop = EmergencyStopSignal::Active;
    add_case(
        &mut cases,
        "SAFE-023-emergency-stop-active",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.emergency_stop = EmergencyStopSignal::Unknown;
    add_case(
        &mut cases,
        "SAFE-024-emergency-stop-unknown",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    add_case(
        &mut cases,
        "SAFE-025-fresh-evidence-restored",
        base_policy.clone(),
        base_observation.clone(),
        Disposition::WithinSyntheticEnvelope,
    );

    let mut observation = base_observation.clone();
    observation.sensors[3].health = SensorHealth::Failed;
    observation.human = HumanObservation::Tracked {
        distance_mm: 2_200,
        position_uncertainty_mm: 100,
        relative_approach_speed_mm_s: 100,
        track_age_ms: 10,
        confidence_ppm: 980_000,
        conflicting_tracks: false,
    };
    add_case(
        &mut cases,
        "SAFE-026-hard-stop-dominates-degraded-sensor-mitigation",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.previous_mode = Some(OperatingMode::Emergency);
    observation.mode = OperatingMode::Recovery;
    observation.motion.motion_command_present = false;
    observation.motion.speed_mm_s = 0;
    add_case(
        &mut cases,
        "SAFE-027-emergency-recovery-requires-external-authority",
        base_policy.clone(),
        observation,
        Disposition::HoldForHumanReview,
    );

    let mut observation = base_observation.clone();
    observation.policy_revision = "revision-2".to_owned();
    add_case(
        &mut cases,
        "SAFE-028-policy-revision-drift",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut policy = base_policy.clone();
    policy.standards_profile.clear();
    add_case(
        &mut cases,
        "SAFE-029-unknown-standards-applicability",
        policy,
        base_observation.clone(),
        Disposition::UnsupportedOdd,
    );

    let mut observation = base_observation.clone();
    observation.sensors.remove(1);
    add_case(
        &mut cases,
        "SAFE-030-missing-critical-localization",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.sensors[0].source_sha256 = "bad-digest".to_owned();
    add_case(
        &mut cases,
        "SAFE-031-malformed-sensor-evidence-digest",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.sensors[0].confidence_ppm = 1_000_001;
    add_case(
        &mut cases,
        "SAFE-032-inflated-sensor-confidence",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.odd_conditions.remove(0);
    add_case(
        &mut cases,
        "SAFE-033-missing-required-odd-condition",
        base_policy.clone(),
        observation,
        Disposition::UnsupportedOdd,
    );

    let mut observation = base_observation.clone();
    observation.odd_conditions[0].age_ms = 900;
    add_case(
        &mut cases,
        "SAFE-034-stale-odd-condition",
        base_policy.clone(),
        observation,
        Disposition::UnsupportedOdd,
    );

    let mut observation = base_observation.clone();
    observation.motion.acceleration_mm_s2 = 600;
    add_case(
        &mut cases,
        "SAFE-035-acceleration-limit-exceeded",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human = HumanObservation::Unknown;
    add_case(
        &mut cases,
        "SAFE-036-human-state-unknown",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.sensors[1].sensor_id = observation.sensors[0].sensor_id.clone();
    add_case(
        &mut cases,
        "SAFE-037-duplicate-sensor-identity",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.observed_at_ms = 3_000;
    add_case(
        &mut cases,
        "SAFE-038-stale-state-observation",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.evaluated_at_ms = 10_001;
    add_case(
        &mut cases,
        "SAFE-039-expired-policy-profile",
        base_policy.clone(),
        observation,
        Disposition::UnsupportedOdd,
    );

    let mut observation = base_observation.clone();
    observation.sensors[0].fault_domain = observation.sensors[1].fault_domain.clone();
    observation.sensors[2].fault_domain = observation.sensors[1].fault_domain.clone();
    add_case(
        &mut cases,
        "SAFE-040-insufficient-independent-fault-domains",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human = HumanObservation::Tracked {
        distance_mm: 2_400,
        position_uncertainty_mm: 100,
        relative_approach_speed_mm_s: 100,
        track_age_ms: 10,
        confidence_ppm: 980_000,
        conflicting_tracks: false,
    };
    add_case(
        &mut cases,
        "SAFE-041-separation-threshold-equality",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    if let HumanObservation::Tracked {
        relative_approach_speed_mm_s,
        ..
    } = &mut observation.human
    {
        *relative_approach_speed_mm_s = 10_000;
    }
    add_case(
        &mut cases,
        "SAFE-042-fast-approach-increases-required-distance",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human = HumanObservation::NoHumanDetected;
    observation.human_detection.coverage_ppm = 100_000;
    add_case(
        &mut cases,
        "SAFE-043-no-human-claim-with-insufficient-coverage",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human = HumanObservation::NoHumanDetected;
    observation.human_detection.contributing_sensor_ids =
        vec!["lidar-front".to_owned(), "unknown-sensor".to_owned()];
    add_case(
        &mut cases,
        "SAFE-044-no-human-claim-with-unresolved-sensor-source",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human = HumanObservation::NoHumanDetected;
    observation.human_detection.observed_at_ms = 4_000;
    add_case(
        &mut cases,
        "SAFE-045-no-human-claim-with-stale-coverage",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human = HumanObservation::NoHumanDetected;
    observation.human_detection.contributing_sensor_ids =
        vec!["lidar-front".to_owned(), "camera-front".to_owned()];
    observation.sensors[2].fault_domain = observation.sensors[0].fault_domain.clone();
    add_case(
        &mut cases,
        "SAFE-046-human-absence-from-common-mode-sensors",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human = HumanObservation::NoHumanDetected;
    observation.human_detection.contributing_sensor_ids =
        vec!["lidar-front".to_owned(), "localization-unit".to_owned()];
    observation.sensors[1].modality = SensorModality::Lidar;
    observation.sensors[1].fault_domain = "independent-navigation-power".to_owned();
    add_case(
        &mut cases,
        "SAFE-047-two-fault-domains-but-one-human-sensing-modality",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human_detection.covered_cells.pop();
    add_case(
        &mut cases,
        "SAFE-048-human-detection-misses-required-hazard-zone-cell",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.sensors[2].covered_cells.pop();
    add_case(
        &mut cases,
        "SAFE-049-independent-camera-modality-misses-required-cell",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human_detection.coverage_grid_sha256 = digest('3');
    add_case(
        &mut cases,
        "SAFE-050-human-receipt-uses-different-site-grid-revision",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.sensors[2].coverage_grid_sha256 = digest('3');
    add_case(
        &mut cases,
        "SAFE-051-camera-coverage-uses-different-site-grid-revision",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.sensors[0].covered_cells.pop();
    observation.sensors[2].covered_cells.pop();
    add_case(
        &mut cases,
        "SAFE-052-fused-receipt-claims-cell-without-source-support",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation
        .human_detection
        .covered_cells
        .push(CoverageCell { column: 9, row: 9 });
    add_case(
        &mut cases,
        "SAFE-053-human-receipt-includes-cell-outside-pinned-zone",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.sensors[2].covered_cells.pop();
    observation.sensors[2].fault_domain = observation.sensors[0].fault_domain.clone();
    add_case(
        &mut cases,
        "SAFE-054-single-fault-domain-and-modality-fail-partial-zone",
        base_policy.clone(),
        observation,
        Disposition::StopRequiredBySyntheticPolicy,
    );

    let mut observation = base_observation.clone();
    observation.human_detection.covered_cells.pop();
    observation
        .human_detection
        .covered_cells
        .push(CoverageCell { column: 0, row: 0 });
    add_case(
        &mut cases,
        "SAFE-055-duplicate-cell-in-human-coverage-receipt",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.sensors[2]
        .covered_cells
        .push(CoverageCell { column: 0, row: 0 });
    add_case(
        &mut cases,
        "SAFE-056-duplicate-cell-in-sensor-coverage-source",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut observation = base_observation.clone();
    observation.sensors[2].coverage_grid_sha256 = digest('3');
    add_case(
        &mut cases,
        "SAFE-057-contributing-sensor-grid-revision-mismatch",
        base_policy.clone(),
        observation,
        Disposition::EvidenceInconsistent,
    );

    let mut policy = base_policy.clone();
    policy.required_human_coverage_cells.clear();
    add_case(
        &mut cases,
        "SAFE-058-policy-does-not-declare-required-hazard-zone-cells",
        policy,
        base_observation.clone(),
        Disposition::UnsupportedOdd,
    );

    cases
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_corpus_has_at_least_thirty_cases_and_unique_ids() {
        let cases = frozen_cases();
        let ids = cases.iter().map(|case| case.id).collect::<BTreeSet<_>>();
        assert!(cases.len() >= 30);
        assert_eq!(ids.len(), cases.len());
    }

    #[test]
    fn every_case_matches_its_independently_pinned_disposition() {
        for case in frozen_cases() {
            let actual = evaluate(&case.policy, &case.observation);
            assert_eq!(
                actual.disposition, case.expected,
                "case {}: reasons={:?}",
                case.id, actual.reasons
            );
        }
    }

    #[test]
    fn no_negative_control_is_promoted_to_within_envelope() {
        for case in frozen_cases() {
            let actual = evaluate(&case.policy, &case.observation);
            if case.expected != Disposition::WithinSyntheticEnvelope {
                assert_ne!(
                    actual.disposition,
                    Disposition::WithinSyntheticEnvelope,
                    "false-safe disposition for {}",
                    case.id
                );
            }
        }
    }

    #[test]
    fn identical_inputs_produce_identical_decisions() {
        for case in frozen_cases() {
            let first = evaluate(&case.policy, &case.observation);
            let second = evaluate(&case.policy, &case.observation);
            assert_eq!(first, second, "nondeterministic case {}", case.id);
        }
    }

    #[test]
    fn sensor_input_order_does_not_change_the_disposition() {
        let (policy, mut observation) = baseline();
        let expected = evaluate(&policy, &observation);
        observation.sensors.reverse();
        observation.human_detection.covered_cells.reverse();
        for sensor in &mut observation.sensors {
            sensor.covered_cells.reverse();
        }
        let reversed = evaluate(&policy, &observation);
        assert_eq!(expected, reversed);
    }

    #[test]
    fn hard_stop_precedes_degraded_sensor_mitigation() {
        let (policy, mut observation) = baseline();
        observation.sensors[3].health = SensorHealth::Failed;
        observation.human = HumanObservation::Tracked {
            distance_mm: 2_000,
            position_uncertainty_mm: 100,
            relative_approach_speed_mm_s: 100,
            track_age_ms: 10,
            confidence_ppm: 980_000,
            conflicting_tracks: false,
        };
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reasons, vec![Reason::HumanSeparationBreached]);
    }

    #[test]
    fn emergency_stop_latch_is_never_reset_by_this_evaluator() {
        let (policy, mut observation) = baseline();
        observation.previous_mode = Some(OperatingMode::Emergency);
        observation.mode = OperatingMode::Recovery;
        observation.motion.speed_mm_s = policy.maximum_speed_recovery_mm_s;
        observation.handover_acknowledged = true;
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::HoldForHumanReview);
        assert_eq!(
            decision.reasons,
            vec![Reason::EmergencyRecoveryRequiresExternalAuthority]
        );
    }

    #[test]
    fn active_emergency_stop_keeps_external_latch_response() {
        let (policy, mut observation) = baseline();
        observation.emergency_stop = EmergencyStopSignal::Active;
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.requested_response,
            RequestedResponse::MaintainExternalEmergencyStopLatch
        );
    }

    #[test]
    fn source_digest_syntax_is_not_authentication() {
        assert!(is_sha256_reference(&"a".repeat(64)));
        assert!(!is_sha256_reference("a".repeat(63).as_str()));
        assert!(!is_sha256_reference(&"A".repeat(64)));
        // The API intentionally has no byte loader, signature verifier or trust store.
    }

    #[test]
    fn policy_digest_and_subject_revisions_are_exactly_bound() {
        let (policy, mut observation) = baseline();
        observation.policy_sha256 = digest('9');
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(decision.reasons, vec![Reason::SubjectBindingMismatch]);
    }

    #[test]
    fn unknown_human_state_never_becomes_clear() {
        let (policy, mut observation) = baseline();
        observation.human = HumanObservation::Unknown;
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn duplicate_odd_condition_ids_fail_closed() {
        let (policy, mut observation) = baseline();
        observation
            .odd_conditions
            .push(observation.odd_conditions[0].clone());
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
    }

    #[test]
    fn invalid_sensor_confidence_does_not_get_clamped() {
        let (policy, mut observation) = baseline();
        observation.sensors[0].confidence_ppm = MAX_CONFIDENCE_PPM + 1;
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
    }

    #[test]
    fn mode_transition_requires_acknowledgement() {
        let (policy, mut observation) = baseline();
        observation.previous_mode = Some(OperatingMode::Autonomous);
        observation.mode = OperatingMode::Supervised;
        observation.handover_acknowledged = false;
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::HoldForHumanReview);
    }

    #[test]
    fn maintenance_mode_rejects_any_motion_command() {
        let (policy, mut observation) = baseline();
        observation.mode = OperatingMode::Maintenance;
        observation.motion.motion_command_present = true;
        observation.motion.speed_mm_s = 0;
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn mode_specific_speed_envelopes_are_enforced() {
        let (policy, mut observation) = baseline();
        observation.mode = OperatingMode::Manual;
        observation.motion.speed_mm_s = 301;
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn policy_validity_window_is_enforced_at_evaluation_time() {
        let (policy, mut observation) = baseline();
        observation.evaluated_at_ms = policy.valid_until_ms + 1;
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::UnsupportedOdd);
    }

    #[test]
    fn near_boundary_uses_uncertainty_conservatively() {
        let (policy, mut observation) = baseline();
        observation.human = HumanObservation::Tracked {
            distance_mm: 2_800,
            position_uncertainty_mm: 600,
            relative_approach_speed_mm_s: 100,
            track_age_ms: 10,
            confidence_ppm: 980_000,
            conflicting_tracks: false,
        };
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn empty_sensor_coverage_cannot_imply_no_human_present() {
        let (policy, mut observation) = baseline();
        observation.sensors.clear();
        observation.human = HumanObservation::NoHumanDetected;
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
    }

    #[test]
    fn malformed_policy_digest_has_a_distinct_inconsistent_disposition() {
        let (mut policy, observation) = baseline();
        policy.policy_sha256 = "not-a-digest".to_owned();
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(decision.reasons, vec![Reason::PolicyDigestMalformed]);
    }

    #[test]
    fn missing_odd_condition_is_not_misreported_as_an_extra_condition() {
        let (policy, mut observation) = baseline();
        observation.odd_conditions.remove(0);
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::UnsupportedOdd);
        assert_eq!(decision.reasons, vec![Reason::OddConditionMissing]);
    }

    #[test]
    fn an_unexpected_odd_condition_is_rejected() {
        let (policy, mut observation) = baseline();
        observation.odd_conditions[0].condition_id = "unregistered-weather-field".to_owned();
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::UnsupportedOdd);
        assert_eq!(decision.reasons, vec![Reason::UnexpectedOddCondition]);
    }

    #[test]
    fn missing_odd_data_cannot_hide_a_hard_separation_stop() {
        let (policy, mut observation) = baseline();
        observation.odd_conditions.remove(0);
        observation.human = HumanObservation::Tracked {
            distance_mm: 2_000,
            position_uncertainty_mm: 100,
            relative_approach_speed_mm_s: 100,
            track_age_ms: 10,
            confidence_ppm: 980_000,
            conflicting_tracks: false,
        };
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reasons, vec![Reason::HumanSeparationBreached]);
    }

    #[test]
    fn degraded_sensor_at_reduced_speed_requests_hold_and_reassessment() {
        let (policy, mut observation) = baseline();
        observation.motion.speed_mm_s = policy.reduced_speed_mm_s;
        // Degrade a non-critical, non-human-detection sensor so this exercises
        // progressive mitigation rather than invalidating the required clear evidence.
        observation.sensors[3].health = SensorHealth::Degraded;
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::RequireProgressiveMitigation
        );
        assert_eq!(
            decision.requested_response,
            RequestedResponse::HoldForHumanReview
        );
    }

    #[test]
    fn all_policy_identity_strings_are_bounded() {
        let (mut policy, observation) = baseline();
        policy.machine_id = "m".repeat(MAX_IDENTIFIER_BYTES + 1);
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::UnsupportedOdd);
    }

    #[test]
    fn critical_sensor_identities_are_bounded() {
        let (mut policy, observation) = baseline();
        policy.critical_sensor_ids[0] = "s".repeat(MAX_IDENTIFIER_BYTES + 1);
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::UnsupportedOdd);
    }

    #[test]
    fn oversized_sensor_collections_fail_closed() {
        let (policy, mut observation) = baseline();
        let template = observation.sensors[0].clone();
        observation.sensors = (0..=MAX_SENSOR_RECORDS)
            .map(|index| {
                let mut sensor = template.clone();
                sensor.sensor_id = format!("sensor-{index}");
                sensor
            })
            .collect();
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(decision.reasons, vec![Reason::SensorSetMalformed]);
    }

    #[test]
    fn extra_sensor_does_not_replace_required_critical_sensor() {
        let (policy, mut observation) = baseline();
        observation.sensors[0].sensor_id = "untrusted-extra-sensor".to_owned();
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reasons, vec![Reason::CriticalSensorMissing]);
    }

    #[test]
    fn no_human_detected_requires_current_sensor_coverage_evidence() {
        let (policy, mut observation) = baseline();
        observation.human = HumanObservation::NoHumanDetected;
        observation.human_detection.coverage_ppm = 100_000;
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(
            decision.reasons,
            vec![Reason::HumanEvidenceCoverageInsufficient]
        );
    }

    #[test]
    fn human_presence_evidence_must_name_its_sensor_sources() {
        let (policy, mut observation) = baseline();
        observation.human = HumanObservation::NoHumanDetected;
        observation.human_detection.contributing_sensor_ids =
            vec!["lidar-front".to_owned(), "unknown-sensor".to_owned()];
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reasons, vec![Reason::HumanEvidenceSensorMissing]);
    }

    #[test]
    fn no_human_detected_cannot_reuse_stale_coverage_receipt() {
        let (policy, mut observation) = baseline();
        observation.human = HumanObservation::NoHumanDetected;
        observation.human_detection.observed_at_ms = 4_000;
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reasons, vec![Reason::HumanTrackStale]);
    }

    #[test]
    fn human_absence_evidence_requires_independent_sensor_fault_domains() {
        let (policy, mut observation) = baseline();
        observation.human = HumanObservation::NoHumanDetected;
        observation.human_detection.contributing_sensor_ids =
            vec!["lidar-front".to_owned(), "camera-front".to_owned()];
        observation.sensors[2].fault_domain = observation.sensors[0].fault_domain.clone();
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(
            decision.reasons,
            vec![Reason::HumanFaultDomainRegionUncovered]
        );
    }

    #[test]
    fn malformed_human_detection_receipt_is_inconsistent() {
        let (policy, mut observation) = baseline();
        observation.human_detection.source_sha256 = "invalid".to_owned();
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(decision.reasons, vec![Reason::HumanEvidenceMalformed]);
    }

    #[test]
    fn human_presence_evidence_requires_modality_diversity() {
        let (policy, mut observation) = baseline();
        observation.human = HumanObservation::NoHumanDetected;
        observation.sensors[2].modality = SensorModality::Lidar;
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reasons, vec![Reason::HumanModalityRegionUncovered]);
    }

    #[test]
    fn decision_binds_expected_and_observed_subjects_and_all_source_refs() {
        let (policy, mut observation) = baseline();
        observation.tool_revision = "unexpected-tool-revision".to_owned();
        let decision = evaluate(&policy, &observation);

        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(
            decision.expected_subject.tool_revision,
            policy.tool_revision
        );
        assert_eq!(
            decision.observed_subject.tool_revision,
            "unexpected-tool-revision"
        );
        assert_eq!(decision.policy_sha256, policy.policy_sha256);
        assert_eq!(
            decision.expected_subject.coverage_grid_sha256,
            policy.coverage_grid_sha256
        );
        assert_eq!(
            decision.observed_subject.coverage_grid_sha256,
            observation.human_detection.coverage_grid_sha256
        );
        assert_eq!(decision.mode, observation.mode);
        assert_eq!(decision.evaluated_at_ms, observation.evaluated_at_ms);
        assert_eq!(decision.observed_at_ms, observation.observed_at_ms);

        let mut expected = observation
            .sensors
            .iter()
            .map(|sensor| EvidenceReference {
                role: EvidenceRole::Sensor,
                source_id: sensor.sensor_id.clone(),
                source_sha256: sensor.source_sha256.clone(),
            })
            .collect::<Vec<_>>();
        expected.push(EvidenceReference {
            role: EvidenceRole::CoverageGrid,
            source_id: "site-human-monitoring-grid".to_owned(),
            source_sha256: policy.coverage_grid_sha256.clone(),
        });
        expected.push(EvidenceReference {
            role: EvidenceRole::HumanDetection,
            source_id: "human-detection".to_owned(),
            source_sha256: observation.human_detection.source_sha256.clone(),
        });
        expected.extend(
            observation
                .odd_conditions
                .iter()
                .map(|condition| EvidenceReference {
                    role: EvidenceRole::OperatingCondition,
                    source_id: condition.condition_id.clone(),
                    source_sha256: condition.source_sha256.clone(),
                }),
        );
        expected.sort();
        assert_eq!(decision.evidence_references, expected);
    }

    #[test]
    fn every_required_hazard_zone_cell_must_be_covered() {
        let (policy, mut observation) = baseline();
        observation.human_detection.covered_cells.pop();
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reasons, vec![Reason::HumanEvidenceRegionUncovered]);
    }

    #[test]
    fn each_independent_modality_must_cover_the_entire_hazard_zone() {
        let (mut policy, mut observation) = baseline();
        // Require three distinct modalities and make Localization an additional
        // full-zone source in its own fault domain. This isolates modality
        // coverage: two domains still cover the whole zone, but only two of
        // the three modalities do.
        policy.minimum_human_sensor_modalities = 3;
        observation
            .human_detection
            .contributing_sensor_ids
            .push("localization-unit".to_owned());
        observation.sensors[1].covered_cells = policy.required_human_coverage_cells.clone();
        observation.sensors[2].covered_cells.pop();
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(decision.reasons, vec![Reason::HumanModalityRegionUncovered]);
    }

    #[test]
    fn each_independent_fault_domain_must_cover_the_entire_hazard_zone() {
        let (policy, mut observation) = baseline();
        observation.sensors[0].fault_domain = "shared-range-domain".to_owned();
        observation.sensors[2].fault_domain = "shared-range-domain".to_owned();
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.disposition,
            Disposition::StopRequiredBySyntheticPolicy
        );
        assert_eq!(
            decision.reasons,
            vec![Reason::HumanFaultDomainRegionUncovered]
        );
    }

    #[test]
    fn coverage_grid_revision_must_match_the_policy() {
        let (policy, mut observation) = baseline();
        observation.human_detection.coverage_grid_sha256 = digest('3');
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(decision.reasons, vec![Reason::CoverageGridMismatch]);
    }

    #[test]
    fn fused_coverage_cannot_claim_cells_without_source_sensor_support() {
        let (policy, mut observation) = baseline();
        observation.sensors[0].covered_cells.pop();
        observation.sensors[2].covered_cells.pop();
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(
            decision.reasons,
            vec![Reason::HumanEvidenceCoverageUnsupported]
        );
    }

    #[test]
    fn coverage_cells_outside_the_pinned_hazard_zone_are_rejected() {
        let (policy, mut observation) = baseline();
        observation
            .human_detection
            .covered_cells
            .push(CoverageCell { column: 9, row: 9 });
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(decision.reasons, vec![Reason::CoverageCellsMalformed]);
    }

    #[test]
    fn policy_requires_unique_nonempty_hazard_zone_cells() {
        let (mut policy, observation) = baseline();
        policy.required_human_coverage_cells.clear();
        let empty = evaluate(&policy, &observation);
        assert_eq!(empty.disposition, Disposition::UnsupportedOdd);

        let (mut policy, observation) = baseline();
        policy
            .required_human_coverage_cells
            .push(policy.required_human_coverage_cells[0]);
        let duplicate = evaluate(&policy, &observation);
        assert_eq!(duplicate.disposition, Disposition::UnsupportedOdd);
    }

    #[test]
    fn sensor_coverage_grid_revision_must_match_policy() {
        let (policy, mut observation) = baseline();
        observation.sensors[2].coverage_grid_sha256 = digest('3');
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(decision.reasons, vec![Reason::CoverageGridMismatch]);
    }

    #[test]
    fn duplicate_sensor_coverage_cells_are_rejected() {
        let (policy, mut observation) = baseline();
        let covered_cell = observation.sensors[2].covered_cells[0];
        observation.sensors[2].covered_cells.push(covered_cell);
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(decision.reasons, vec![Reason::SensorSetMalformed]);
    }

    #[test]
    fn duplicate_human_receipt_cells_are_rejected() {
        let (policy, mut observation) = baseline();
        observation
            .human_detection
            .covered_cells
            .push(observation.human_detection.covered_cells[0]);
        let decision = evaluate(&policy, &observation);
        assert_eq!(decision.disposition, Disposition::EvidenceInconsistent);
        assert_eq!(decision.reasons, vec![Reason::CoverageCellsMalformed]);
    }

    #[test]
    fn no_actuator_or_emergency_reset_surface_is_exposed() {
        let (policy, observation) = baseline();
        let decision = evaluate(&policy, &observation);
        assert_eq!(
            decision.requested_response,
            RequestedResponse::ContinueAtDeclaredLimit
        );
        // RequestedResponse is a report label, not an executable command API.
    }
}
