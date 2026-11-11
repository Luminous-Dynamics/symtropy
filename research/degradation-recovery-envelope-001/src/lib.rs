//! Fault-conditioned degradation and recovery envelope auditor.
//!
//! The auditor checks paired clean/faulted traces and reports each recovery axis
//! separately. It is a record auditor, not a vehicle controller or safety authority.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelStatus {
    TrainedPolicy,
    UntrainedInputBridge,
    SyntheticReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunDisposition {
    Complete,
    Failed,
    Incomplete,
    Censored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultKind {
    CameraDrop,
    FrameDelay,
    StaleReplay,
    PixelCorruption,
    CalibrationShift,
    ClockDrift,
    ComputeOverrun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationCondition {
    Clean,
    Delayed,
    Missing,
    Stale,
    Corrupt,
    CalibrationMismatch,
    ClockViolation,
    ComputeOverrun,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultEvent {
    pub event_id: String,
    pub kind: FaultKind,
    pub start_tick: u64,
    pub end_tick: u64,
    pub severity_ppm: u32,
    pub detection_deadline_ticks: u64,
    pub containment_deadline_ticks: u64,
    pub max_accepted_frame_age_us: u64,
    pub recovery_healthy_quorum_ticks: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TickTrace {
    pub tick: u64,
    pub capture_timestamp_us: u64,
    pub delivered_timestamp_us: u64,
    pub conditions: Vec<ObservationCondition>,
    pub policy_input_accepted: bool,
    pub fault_detection_event_ids: Vec<String>,
    pub containment_event_ids: Vec<String>,
    pub degraded_mode: bool,
    pub fallback_active: bool,
    pub safe_stop_complete: bool,
    pub normal_mode: bool,
    pub decision_latency_us: Option<u64>,
    pub inference_energy_uj: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunTrace {
    pub run_id: String,
    pub disposition: RunDisposition,
    pub scenario_seed_sha256: String,
    pub environment_randomness_sha256: String,
    pub expected_tick_count: u64,
    pub ticks: Vec<TickTrace>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign {
    pub schema_version: u32,
    pub campaign_id: String,
    pub model_id: String,
    pub model_status: ModelStatus,
    pub model_revision: String,
    pub checkpoint_sha256: Option<String>,
    pub hardware_id: String,
    pub inference_precision: String,
    pub power_profile: String,
    pub energy_measurement_method: String,
    pub policy_visible_inputs: Vec<String>,
    pub benchmark_revision: String,
    pub evaluator_revision: String,
    pub clock_id: String,
    pub policy_deadline_us: u64,
    pub fault_schedule_visible_to_policy: bool,
    pub clean_control: RunTrace,
    pub faulted_run: RunTrace,
    pub events: Vec<FaultEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventMetrics {
    pub event_id: String,
    pub kind: FaultKind,
    pub start_tick: u64,
    pub end_tick: u64,
    pub severity_ppm: u32,
    pub detection_deadline_ticks: u64,
    pub containment_deadline_ticks: u64,
    pub max_accepted_frame_age_us: u64,
    pub exposure_ticks: u64,
    pub detection_latency_ticks: Option<u64>,
    pub containment_latency_ticks: Option<u64>,
    pub recovery_tick: Option<u64>,
    pub recovery_latency_after_clearance_ticks: Option<u64>,
    pub healthy_quorum_ticks_required: u64,
    pub stale_or_corrupt_acceptances: u64,
    pub missed_detection_deadline: bool,
    pub missed_containment_deadline: bool,
    pub recovery_observed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LatencySummary {
    pub sample_count: usize,
    pub p50_us: Option<u64>,
    pub p95_us: Option<u64>,
    pub p99_us: Option<u64>,
    pub max_us: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnergySummary {
    pub sample_count: usize,
    pub p50_uj: Option<u64>,
    pub p95_uj: Option<u64>,
    pub p99_uj: Option<u64>,
    pub max_uj: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditReport {
    pub schema_version: u32,
    pub campaign_id: String,
    pub model_id: String,
    pub model_status: ModelStatus,
    pub model_revision: String,
    pub checkpoint_sha256: Option<String>,
    pub hardware_id: String,
    pub inference_precision: String,
    pub power_profile: String,
    pub energy_measurement_method: String,
    pub policy_visible_inputs: Vec<String>,
    pub disposition: String,
    pub claim_ceiling: String,
    pub expected_ticks: u64,
    pub clean_control_observed_ticks: usize,
    pub faulted_observed_ticks: usize,
    pub fault_event_count: usize,
    pub clean_control_false_positive_ticks: u64,
    pub clean_control_deadline_miss_ticks: u64,
    pub faulted_stale_or_corrupt_acceptances: u64,
    pub faulted_deadline_miss_ticks: u64,
    pub clean_control_decision_latency: LatencySummary,
    pub faulted_decision_latency: LatencySummary,
    pub clean_control_inference_energy: EnergySummary,
    pub faulted_inference_energy: EnergySummary,
    pub events: Vec<EventMetrics>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Finding {
    SchemaVersionMismatch,
    EmptyIdentity,
    DuplicateRunId,
    NoFaultEvents,
    InvalidPolicyDeadline,
    InvalidDigest,
    FaultScheduleExposedToPolicy,
    RunNotComplete,
    ScenarioSeedMismatch,
    EnvironmentRandomnessMismatch,
    TickDenominatorMismatch,
    TickSequenceMismatch,
    DuplicateTick,
    NonMonotonicCaptureClock,
    DeliveryBeforeCapture,
    EmptyObservationConditions,
    CleanConditionMixedWithFault,
    FaultScheduleOutOfBounds,
    DuplicateFaultEvent,
    EmptyFaultEventId,
    InvalidFaultSeverity,
    ZeroRecoveryQuorum,
    ExpectedFaultConditionMissing,
    ForbiddenPolicyInputAccepted,
    DelayedFrameAcceptedTooOld,
    NormalModeDuringFault,
    FaultUndetected,
    UnknownEventReference,
    DuplicateEventReference,
    ContainmentReferenceWithoutState,
    NormalModeWithContainmentActive,
    DetectionDeadlineMissed,
    ContainmentDeadlineMissed,
    RecoveryNotObserved,
    PrematureRecovery,
    FalsePositiveDegradation,
    CleanControlNotClean,
    CleanControlNotNormal,
    DecisionLatencyMissing,
    InferenceEnergyMissing,
    UndeclaredFaultCondition,
    FaultEventReferenceBeforeOnset,
    MissingCheckpointForTrainedPolicy,
    EmptyPolicyInputSet,
    DuplicatePolicyInput,
}

impl Finding {
    fn as_str(self) -> &'static str {
        match self {
            Self::SchemaVersionMismatch => "schema_version_mismatch",
            Self::EmptyIdentity => "empty_identity",
            Self::DuplicateRunId => "duplicate_run_id",
            Self::NoFaultEvents => "no_fault_events",
            Self::InvalidPolicyDeadline => "invalid_policy_deadline",
            Self::InvalidDigest => "invalid_digest",
            Self::FaultScheduleExposedToPolicy => "fault_schedule_exposed_to_policy",
            Self::RunNotComplete => "run_not_complete",
            Self::ScenarioSeedMismatch => "scenario_seed_mismatch",
            Self::EnvironmentRandomnessMismatch => "environment_randomness_mismatch",
            Self::TickDenominatorMismatch => "tick_denominator_mismatch",
            Self::TickSequenceMismatch => "tick_sequence_mismatch",
            Self::DuplicateTick => "duplicate_tick",
            Self::NonMonotonicCaptureClock => "non_monotonic_capture_clock",
            Self::DeliveryBeforeCapture => "delivery_before_capture",
            Self::EmptyObservationConditions => "empty_observation_conditions",
            Self::CleanConditionMixedWithFault => "clean_condition_mixed_with_fault",
            Self::FaultScheduleOutOfBounds => "fault_schedule_out_of_bounds",
            Self::DuplicateFaultEvent => "duplicate_fault_event",
            Self::EmptyFaultEventId => "empty_fault_event_id",
            Self::InvalidFaultSeverity => "invalid_fault_severity",
            Self::ZeroRecoveryQuorum => "zero_recovery_quorum",
            Self::ExpectedFaultConditionMissing => "expected_fault_condition_missing",
            Self::ForbiddenPolicyInputAccepted => "forbidden_policy_input_accepted",
            Self::DelayedFrameAcceptedTooOld => "delayed_frame_accepted_too_old",
            Self::NormalModeDuringFault => "normal_mode_during_fault",
            Self::FaultUndetected => "fault_undetected",
            Self::UnknownEventReference => "unknown_event_reference",
            Self::DuplicateEventReference => "duplicate_event_reference",
            Self::ContainmentReferenceWithoutState => "containment_reference_without_state",
            Self::NormalModeWithContainmentActive => "normal_mode_with_containment_active",
            Self::DetectionDeadlineMissed => "detection_deadline_missed",
            Self::ContainmentDeadlineMissed => "containment_deadline_missed",
            Self::RecoveryNotObserved => "recovery_not_observed",
            Self::PrematureRecovery => "premature_recovery",
            Self::FalsePositiveDegradation => "false_positive_degradation",
            Self::CleanControlNotClean => "clean_control_not_clean",
            Self::CleanControlNotNormal => "clean_control_not_normal",
            Self::DecisionLatencyMissing => "decision_latency_missing",
            Self::InferenceEnergyMissing => "inference_energy_missing",
            Self::UndeclaredFaultCondition => "undeclared_fault_condition",
            Self::FaultEventReferenceBeforeOnset => "fault_event_reference_before_onset",
            Self::MissingCheckpointForTrainedPolicy => "missing_checkpoint_for_trained_policy",
            Self::EmptyPolicyInputSet => "empty_policy_input_set",
            Self::DuplicatePolicyInput => "duplicate_policy_input",
        }
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn expected_condition(kind: FaultKind) -> ObservationCondition {
    match kind {
        FaultKind::CameraDrop => ObservationCondition::Missing,
        FaultKind::FrameDelay => ObservationCondition::Delayed,
        FaultKind::StaleReplay => ObservationCondition::Stale,
        FaultKind::PixelCorruption => ObservationCondition::Corrupt,
        FaultKind::CalibrationShift => ObservationCondition::CalibrationMismatch,
        FaultKind::ClockDrift => ObservationCondition::ClockViolation,
        FaultKind::ComputeOverrun => ObservationCondition::ComputeOverrun,
    }
}

fn contains_any(conditions: &[ObservationCondition], targets: &[ObservationCondition]) -> bool {
    conditions
        .iter()
        .any(|condition| targets.contains(condition))
}

fn contains_forbidden_input(conditions: &[ObservationCondition]) -> bool {
    contains_any(
        conditions,
        &[
            ObservationCondition::Missing,
            ObservationCondition::Stale,
            ObservationCondition::Corrupt,
            ObservationCondition::CalibrationMismatch,
            ObservationCondition::ClockViolation,
        ],
    )
}

fn containment_active(tick: &TickTrace) -> bool {
    tick.degraded_mode || tick.fallback_active || tick.safe_stop_complete
}

fn is_healthy_tick(tick: &TickTrace, policy_deadline_us: u64) -> bool {
    tick.conditions.as_slice() == [ObservationCondition::Clean]
        && tick.policy_input_accepted
        && !tick.safe_stop_complete
        && tick
            .decision_latency_us
            .is_some_and(|latency| latency <= policy_deadline_us)
        && tick.fault_detection_event_ids.is_empty()
        && !contains_any(
            &tick.conditions,
            &[
                ObservationCondition::Delayed,
                ObservationCondition::Missing,
                ObservationCondition::Stale,
                ObservationCondition::Corrupt,
                ObservationCondition::CalibrationMismatch,
                ObservationCondition::ClockViolation,
                ObservationCondition::ComputeOverrun,
            ],
        )
}

fn percentile_nearest_rank(values: &[u64], percentile: usize) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (percentile * sorted.len()).div_ceil(100).max(1);
    sorted.get(rank.saturating_sub(1)).copied()
}

fn latency_summary(ticks: &[TickTrace]) -> LatencySummary {
    let values = ticks
        .iter()
        .filter_map(|tick| tick.decision_latency_us)
        .collect::<Vec<_>>();
    LatencySummary {
        sample_count: values.len(),
        p50_us: percentile_nearest_rank(&values, 50),
        p95_us: percentile_nearest_rank(&values, 95),
        p99_us: percentile_nearest_rank(&values, 99),
        max_us: values.iter().copied().max(),
    }
}

fn energy_summary(ticks: &[TickTrace]) -> EnergySummary {
    let values = ticks
        .iter()
        .filter_map(|tick| tick.inference_energy_uj)
        .collect::<Vec<_>>();
    EnergySummary {
        sample_count: values.len(),
        p50_uj: percentile_nearest_rank(&values, 50),
        p95_uj: percentile_nearest_rank(&values, 95),
        p99_uj: percentile_nearest_rank(&values, 99),
        max_uj: values.iter().copied().max(),
    }
}

fn tick_map(run: &RunTrace) -> std::collections::BTreeMap<u64, &TickTrace> {
    run.ticks.iter().map(|tick| (tick.tick, tick)).collect()
}

fn validate_run(run: &RunTrace, expected_ticks: u64, findings: &mut BTreeSet<Finding>) {
    if run.disposition != RunDisposition::Complete {
        findings.insert(Finding::RunNotComplete);
    }
    if run.expected_tick_count != expected_ticks || run.ticks.len() as u64 != expected_ticks {
        findings.insert(Finding::TickDenominatorMismatch);
    }
    let mut seen = BTreeSet::new();
    let mut previous_capture = None;
    for tick in &run.ticks {
        if tick.tick == 0 || tick.tick > expected_ticks {
            findings.insert(Finding::TickSequenceMismatch);
            continue;
        }
        if !seen.insert(tick.tick) {
            findings.insert(Finding::DuplicateTick);
        }
        if let Some(previous) = previous_capture {
            if tick.capture_timestamp_us <= previous {
                findings.insert(Finding::NonMonotonicCaptureClock);
            }
        }
        previous_capture = Some(tick.capture_timestamp_us);
        if tick.delivered_timestamp_us < tick.capture_timestamp_us {
            findings.insert(Finding::DeliveryBeforeCapture);
        }
        if tick.decision_latency_us.is_none() {
            findings.insert(Finding::DecisionLatencyMissing);
        }
        if tick.inference_energy_uj.is_none() {
            findings.insert(Finding::InferenceEnergyMissing);
        }
        if tick.conditions.is_empty() {
            findings.insert(Finding::EmptyObservationConditions);
        }
        if tick.conditions.contains(&ObservationCondition::Clean) && tick.conditions.len() > 1 {
            findings.insert(Finding::CleanConditionMixedWithFault);
        }
        if contains_forbidden_input(&tick.conditions) && tick.policy_input_accepted {
            findings.insert(Finding::ForbiddenPolicyInputAccepted);
        }
    }
    if seen.len() as u64 != expected_ticks || seen.iter().copied().ne(1..=expected_ticks) {
        findings.insert(Finding::TickSequenceMismatch);
    }
}

fn event_metrics(
    event: &FaultEvent,
    run_ticks: &std::collections::BTreeMap<u64, &TickTrace>,
    findings: &mut BTreeSet<Finding>,
    policy_deadline_us: u64,
) -> EventMetrics {
    let exposure_ticks = event.end_tick - event.start_tick + 1;
    let expected = expected_condition(event.kind);
    for tick in event.start_tick..=event.end_tick {
        if !run_ticks
            .get(&tick)
            .is_some_and(|record| record.conditions.contains(&expected))
        {
            findings.insert(Finding::ExpectedFaultConditionMissing);
        }
    }

    let detection_tick = run_ticks
        .range(event.start_tick..)
        .find_map(|(tick, record)| {
            record
                .fault_detection_event_ids
                .iter()
                .any(|id| id == &event.event_id)
                .then_some(*tick)
        });
    let containment_tick = run_ticks
        .range(event.start_tick..)
        .find_map(|(tick, record)| {
            record
                .containment_event_ids
                .iter()
                .any(|id| id == &event.event_id)
                .then_some(*tick)
                .filter(|_| containment_active(record))
        });

    let detection_latency = detection_tick.map(|tick| tick.saturating_sub(event.start_tick));
    let containment_latency = containment_tick.map(|tick| tick.saturating_sub(event.start_tick));
    if detection_tick.is_none() {
        findings.insert(Finding::FaultUndetected);
    } else if detection_latency.is_some_and(|latency| latency > event.detection_deadline_ticks) {
        findings.insert(Finding::DetectionDeadlineMissed);
    }
    if containment_tick.is_none()
        || containment_latency.is_some_and(|latency| latency > event.containment_deadline_ticks)
    {
        findings.insert(Finding::ContainmentDeadlineMissed);
    }

    let mut healthy_run = 0_u64;
    let mut recovery_tick = None;
    if let Some(first_recovery_tick) = event.end_tick.checked_add(1) {
        for (tick, record) in run_ticks.range(first_recovery_tick..) {
            if is_healthy_tick(record, policy_deadline_us) {
                healthy_run += 1;
            } else {
                healthy_run = 0;
            }
            if record.normal_mode && healthy_run < event.recovery_healthy_quorum_ticks {
                findings.insert(Finding::PrematureRecovery);
            }
            if record.normal_mode && healthy_run >= event.recovery_healthy_quorum_ticks {
                recovery_tick = Some(*tick);
                break;
            }
        }
    }
    if recovery_tick.is_none() {
        findings.insert(Finding::RecoveryNotObserved);
    }
    let recovery_latency = recovery_tick.map(|tick| tick.saturating_sub(event.end_tick));

    let stale_or_corrupt_acceptances = run_ticks
        .range(event.start_tick..=event.end_tick)
        .filter(|(_, record)| {
            record.policy_input_accepted
                && contains_any(
                    &record.conditions,
                    &[ObservationCondition::Stale, ObservationCondition::Corrupt],
                )
        })
        .count() as u64;

    EventMetrics {
        event_id: event.event_id.clone(),
        kind: event.kind,
        start_tick: event.start_tick,
        end_tick: event.end_tick,
        severity_ppm: event.severity_ppm,
        detection_deadline_ticks: event.detection_deadline_ticks,
        containment_deadline_ticks: event.containment_deadline_ticks,
        max_accepted_frame_age_us: event.max_accepted_frame_age_us,
        exposure_ticks,
        detection_latency_ticks: detection_latency,
        containment_latency_ticks: containment_latency,
        recovery_tick,
        recovery_latency_after_clearance_ticks: recovery_latency,
        healthy_quorum_ticks_required: event.recovery_healthy_quorum_ticks,
        stale_or_corrupt_acceptances,
        missed_detection_deadline: detection_tick.is_none()
            || detection_latency.is_some_and(|latency| latency > event.detection_deadline_ticks),
        missed_containment_deadline: containment_tick.is_none()
            || containment_latency
                .is_some_and(|latency| latency > event.containment_deadline_ticks),
        recovery_observed: recovery_tick.is_some(),
    }
}

/// Generate a deterministic seven-family schedule. The spacing rule reserves
/// the full exposure plus the configured healthy-recovery quorum before the next
/// fault starts, so the default campaign does not accidentally merge recovery
/// windows. This schedules interventions; it does not simulate their effects.
pub fn standard_fault_schedule(
    first_start_tick: u64,
    spacing_ticks: u64,
    exposure_ticks: u64,
    detection_deadline_ticks: u64,
    containment_deadline_ticks: u64,
    recovery_healthy_quorum_ticks: u64,
) -> Result<Vec<FaultEvent>, String> {
    if first_start_tick == 0 || spacing_ticks == 0 || exposure_ticks == 0 {
        return Err("start tick, spacing and exposure duration must be non-zero".into());
    }
    if recovery_healthy_quorum_ticks == 0 {
        return Err("recovery healthy-observation quorum must be non-zero".into());
    }
    let minimum_spacing = exposure_ticks
        .checked_add(recovery_healthy_quorum_ticks)
        .ok_or("fault spacing requirement overflow")?;
    if spacing_ticks < minimum_spacing {
        return Err(format!(
            "spacing_ticks {spacing_ticks} is too short; require at least exposure_ticks + recovery_quorum_ticks = {minimum_spacing}"
        ));
    }
    let kinds = [
        FaultKind::CameraDrop,
        FaultKind::FrameDelay,
        FaultKind::StaleReplay,
        FaultKind::PixelCorruption,
        FaultKind::CalibrationShift,
        FaultKind::ClockDrift,
        FaultKind::ComputeOverrun,
    ];
    let mut events = Vec::with_capacity(kinds.len());
    for (index, kind) in kinds.into_iter().enumerate() {
        let offset = (index as u64)
            .checked_mul(spacing_ticks)
            .ok_or("fault schedule offset overflow")?;
        let start_tick = first_start_tick
            .checked_add(offset)
            .ok_or("fault schedule start tick overflow")?;
        let end_tick = start_tick
            .checked_add(exposure_ticks - 1)
            .ok_or("fault schedule end tick overflow")?;
        let slug = match kind {
            FaultKind::CameraDrop => "camera-drop",
            FaultKind::FrameDelay => "frame-delay",
            FaultKind::StaleReplay => "stale-replay",
            FaultKind::PixelCorruption => "pixel-corruption",
            FaultKind::CalibrationShift => "calibration-shift",
            FaultKind::ClockDrift => "clock-drift",
            FaultKind::ComputeOverrun => "compute-overrun",
        };
        let severity_ppm = match kind {
            FaultKind::CameraDrop | FaultKind::StaleReplay => 1_000_000,
            FaultKind::FrameDelay | FaultKind::PixelCorruption | FaultKind::ComputeOverrun => {
                500_000
            },
            FaultKind::CalibrationShift | FaultKind::ClockDrift => 250_000,
        };
        events.push(FaultEvent {
            event_id: format!("{slug}-{start_tick}"),
            kind,
            start_tick,
            end_tick,
            severity_ppm,
            detection_deadline_ticks,
            containment_deadline_ticks,
            max_accepted_frame_age_us: if kind == FaultKind::FrameDelay {
                100_000
            } else {
                0
            },
            recovery_healthy_quorum_ticks,
        });
    }
    Ok(events)
}

pub fn audit_campaign(campaign: &Campaign) -> AuditReport {
    let mut findings = BTreeSet::new();
    if campaign.schema_version != 1 {
        findings.insert(Finding::SchemaVersionMismatch);
    }
    for identity in [
        campaign.campaign_id.as_str(),
        campaign.model_id.as_str(),
        campaign.model_revision.as_str(),
        campaign.hardware_id.as_str(),
        campaign.inference_precision.as_str(),
        campaign.power_profile.as_str(),
        campaign.energy_measurement_method.as_str(),
        campaign.benchmark_revision.as_str(),
        campaign.evaluator_revision.as_str(),
        campaign.clock_id.as_str(),
        campaign.clean_control.run_id.as_str(),
        campaign.faulted_run.run_id.as_str(),
        campaign.clean_control.scenario_seed_sha256.as_str(),
        campaign.faulted_run.scenario_seed_sha256.as_str(),
        campaign
            .clean_control
            .environment_randomness_sha256
            .as_str(),
        campaign.faulted_run.environment_randomness_sha256.as_str(),
    ] {
        if identity.trim().is_empty() {
            findings.insert(Finding::EmptyIdentity);
        }
    }
    if campaign.model_status == ModelStatus::TrainedPolicy && campaign.checkpoint_sha256.is_none() {
        findings.insert(Finding::MissingCheckpointForTrainedPolicy);
    }
    if campaign
        .checkpoint_sha256
        .as_deref()
        .is_some_and(|value| !is_sha256_hex(value))
    {
        findings.insert(Finding::InvalidDigest);
    }
    if campaign.policy_visible_inputs.is_empty() {
        findings.insert(Finding::EmptyPolicyInputSet);
    }
    if campaign
        .policy_visible_inputs
        .iter()
        .any(|input| input.trim().is_empty())
        || campaign
            .policy_visible_inputs
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != campaign.policy_visible_inputs.len()
    {
        findings.insert(Finding::DuplicatePolicyInput);
    }
    for digest in [
        campaign.clean_control.scenario_seed_sha256.as_str(),
        campaign.faulted_run.scenario_seed_sha256.as_str(),
        campaign
            .clean_control
            .environment_randomness_sha256
            .as_str(),
        campaign.faulted_run.environment_randomness_sha256.as_str(),
    ] {
        if !is_sha256_hex(digest) {
            findings.insert(Finding::InvalidDigest);
        }
    }
    if campaign.clean_control.run_id == campaign.faulted_run.run_id {
        findings.insert(Finding::DuplicateRunId);
    }
    if campaign.events.is_empty() {
        findings.insert(Finding::NoFaultEvents);
    }
    if campaign.policy_deadline_us == 0 {
        findings.insert(Finding::InvalidPolicyDeadline);
    }
    if campaign.fault_schedule_visible_to_policy {
        findings.insert(Finding::FaultScheduleExposedToPolicy);
    }
    if campaign.clean_control.scenario_seed_sha256 != campaign.faulted_run.scenario_seed_sha256 {
        findings.insert(Finding::ScenarioSeedMismatch);
    }
    if campaign.clean_control.environment_randomness_sha256
        != campaign.faulted_run.environment_randomness_sha256
    {
        findings.insert(Finding::EnvironmentRandomnessMismatch);
    }

    let expected_ticks = campaign.clean_control.expected_tick_count;
    validate_run(&campaign.clean_control, expected_ticks, &mut findings);
    validate_run(&campaign.faulted_run, expected_ticks, &mut findings);

    let control_false_positive_ticks = campaign
        .clean_control
        .ticks
        .iter()
        .filter(|tick| {
            !tick.fault_detection_event_ids.is_empty()
                || !tick.containment_event_ids.is_empty()
                || tick.degraded_mode
                || tick.fallback_active
                || tick.safe_stop_complete
        })
        .count() as u64;
    if control_false_positive_ticks > 0 {
        findings.insert(Finding::FalsePositiveDegradation);
    }
    for tick in &campaign.clean_control.ticks {
        if tick.conditions.as_slice() != [ObservationCondition::Clean]
            || !tick.policy_input_accepted
        {
            findings.insert(Finding::CleanControlNotClean);
        }
        if !tick.normal_mode || containment_active(tick) {
            findings.insert(Finding::CleanControlNotNormal);
        }
    }
    for tick in &campaign.faulted_run.ticks {
        let active_fault = campaign
            .events
            .iter()
            .any(|event| (event.start_tick..=event.end_tick).contains(&tick.tick));
        if active_fault && tick.normal_mode {
            findings.insert(Finding::NormalModeDuringFault);
        }
        let age = tick
            .delivered_timestamp_us
            .saturating_sub(tick.capture_timestamp_us);
        if tick.policy_input_accepted && tick.conditions.contains(&ObservationCondition::Delayed) {
            let allowed_age = campaign
                .events
                .iter()
                .filter(|event| {
                    event.kind == FaultKind::FrameDelay
                        && (event.start_tick..=event.end_tick).contains(&tick.tick)
                })
                .map(|event| event.max_accepted_frame_age_us)
                .min()
                .unwrap_or(0);
            if age > allowed_age {
                findings.insert(Finding::DelayedFrameAcceptedTooOld);
            }
        }
    }

    let mut event_ids = BTreeSet::new();
    let faulted_tick_map = tick_map(&campaign.faulted_run);
    let mut events = Vec::with_capacity(campaign.events.len());
    for event in &campaign.events {
        if event.event_id.trim().is_empty() {
            findings.insert(Finding::EmptyFaultEventId);
        }
        if !event_ids.insert(event.event_id.as_str()) {
            findings.insert(Finding::DuplicateFaultEvent);
        }
        if event.start_tick == 0
            || event.start_tick > event.end_tick
            || event.end_tick > expected_ticks
        {
            findings.insert(Finding::FaultScheduleOutOfBounds);
            continue;
        }
        if event.severity_ppm > 1_000_000 {
            findings.insert(Finding::InvalidFaultSeverity);
        }
        if event.recovery_healthy_quorum_ticks == 0 {
            findings.insert(Finding::ZeroRecoveryQuorum);
        }
        events.push(event_metrics(
            event,
            &faulted_tick_map,
            &mut findings,
            campaign.policy_deadline_us,
        ));
    }

    for tick in &campaign.faulted_run.ticks {
        for condition in tick
            .conditions
            .iter()
            .copied()
            .filter(|condition| *condition != ObservationCondition::Clean)
        {
            let declared = campaign.events.iter().any(|event| {
                (event.start_tick..=event.end_tick).contains(&tick.tick)
                    && expected_condition(event.kind) == condition
            });
            if !declared {
                findings.insert(Finding::UndeclaredFaultCondition);
            }
        }
    }

    let events_by_id = campaign
        .events
        .iter()
        .map(|event| (event.event_id.as_str(), event))
        .collect::<std::collections::BTreeMap<_, _>>();
    let known_event_ids = campaign
        .events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect::<BTreeSet<_>>();
    for tick in campaign
        .clean_control
        .ticks
        .iter()
        .chain(campaign.faulted_run.ticks.iter())
    {
        for references in [&tick.fault_detection_event_ids, &tick.containment_event_ids] {
            let mut seen_references = BTreeSet::new();
            for reference in references {
                if !seen_references.insert(reference.as_str()) {
                    findings.insert(Finding::DuplicateEventReference);
                }
                if !known_event_ids.contains(reference.as_str()) {
                    findings.insert(Finding::UnknownEventReference);
                }
                if events_by_id
                    .get(reference.as_str())
                    .is_some_and(|event| tick.tick < event.start_tick)
                {
                    findings.insert(Finding::FaultEventReferenceBeforeOnset);
                }
            }
        }
        if !tick.containment_event_ids.is_empty() && !containment_active(tick) {
            findings.insert(Finding::ContainmentReferenceWithoutState);
        }
        if tick.normal_mode && containment_active(tick) {
            findings.insert(Finding::NormalModeWithContainmentActive);
        }
    }

    let stale_or_corrupt_acceptances = campaign
        .faulted_run
        .ticks
        .iter()
        .filter(|tick| {
            tick.policy_input_accepted
                && contains_any(
                    &tick.conditions,
                    &[ObservationCondition::Stale, ObservationCondition::Corrupt],
                )
        })
        .count() as u64;
    let clean_control_deadline_misses = campaign
        .clean_control
        .ticks
        .iter()
        .filter(|tick| {
            tick.decision_latency_us
                .is_some_and(|latency| latency > campaign.policy_deadline_us)
        })
        .count() as u64;
    let faulted_deadline_misses = campaign
        .faulted_run
        .ticks
        .iter()
        .filter(|tick| {
            tick.decision_latency_us
                .is_some_and(|latency| latency > campaign.policy_deadline_us)
        })
        .count() as u64;

    let clean_latency = latency_summary(&campaign.clean_control.ticks);
    let faulted_latency = latency_summary(&campaign.faulted_run.ticks);
    let clean_energy = energy_summary(&campaign.clean_control.ticks);
    let faulted_energy = energy_summary(&campaign.faulted_run.ticks);

    AuditReport {
        schema_version: 1,
        campaign_id: campaign.campaign_id.clone(),
        model_id: campaign.model_id.clone(),
        model_status: campaign.model_status,
        model_revision: campaign.model_revision.clone(),
        checkpoint_sha256: campaign.checkpoint_sha256.clone(),
        hardware_id: campaign.hardware_id.clone(),
        inference_precision: campaign.inference_precision.clone(),
        power_profile: campaign.power_profile.clone(),
        energy_measurement_method: campaign.energy_measurement_method.clone(),
        policy_visible_inputs: campaign.policy_visible_inputs.clone(),
        disposition: if findings.is_empty() { "valid" } else { "rejected" }.to_string(),
        claim_ceiling: "Finite paired trace consistency and declared software fault accounting only. This audit does not prove physical fault fidelity, trained-policy detection, safe-stop performance, real-road transfer or vehicle safety.".to_string(),
        expected_ticks,
        clean_control_observed_ticks: campaign.clean_control.ticks.len(),
        faulted_observed_ticks: campaign.faulted_run.ticks.len(),
        fault_event_count: campaign.events.len(),
        clean_control_false_positive_ticks: control_false_positive_ticks,
        clean_control_deadline_miss_ticks: clean_control_deadline_misses,
        faulted_stale_or_corrupt_acceptances: stale_or_corrupt_acceptances,
        faulted_deadline_miss_ticks: faulted_deadline_misses,
        clean_control_decision_latency: clean_latency,
        faulted_decision_latency: faulted_latency,
        clean_control_inference_energy: clean_energy,
        faulted_inference_energy: faulted_energy,
        events,
        findings: findings.into_iter().map(Finding::as_str).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(
        sequence: u64,
        conditions: Vec<ObservationCondition>,
        accepted: bool,
        fault_detected: bool,
        degraded: bool,
        fallback: bool,
        normal: bool,
    ) -> TickTrace {
        let capture = 1_000_000 + sequence * 33_333;
        TickTrace {
            tick: sequence,
            capture_timestamp_us: capture,
            delivered_timestamp_us: capture + 100,
            conditions,
            policy_input_accepted: accepted,
            fault_detection_event_ids: if fault_detected {
                vec!["camera-drop-1".to_string()]
            } else {
                Vec::new()
            },
            containment_event_ids: if degraded {
                vec!["camera-drop-1".to_string()]
            } else {
                Vec::new()
            },
            degraded_mode: degraded,
            fallback_active: fallback,
            safe_stop_complete: false,
            normal_mode: normal,
            decision_latency_us: Some(10_000 + sequence * 100),
            inference_energy_uj: Some(40 + sequence),
        }
    }

    fn campaign() -> Campaign {
        let clean = (1..=8)
            .map(|sequence| {
                tick(
                    sequence,
                    vec![ObservationCondition::Clean],
                    true,
                    false,
                    false,
                    false,
                    true,
                )
            })
            .collect::<Vec<_>>();
        let faulted = vec![
            tick(
                1,
                vec![ObservationCondition::Clean],
                true,
                false,
                false,
                false,
                true,
            ),
            tick(
                2,
                vec![ObservationCondition::Clean],
                true,
                false,
                false,
                false,
                true,
            ),
            tick(
                3,
                vec![ObservationCondition::Missing],
                false,
                true,
                true,
                true,
                false,
            ),
            tick(
                4,
                vec![ObservationCondition::Missing],
                false,
                false,
                true,
                true,
                false,
            ),
            tick(
                5,
                vec![ObservationCondition::Clean],
                true,
                false,
                true,
                true,
                false,
            ),
            tick(
                6,
                vec![ObservationCondition::Clean],
                true,
                false,
                true,
                true,
                false,
            ),
            tick(
                7,
                vec![ObservationCondition::Clean],
                true,
                false,
                false,
                false,
                true,
            ),
            tick(
                8,
                vec![ObservationCondition::Clean],
                true,
                false,
                false,
                false,
                true,
            ),
        ];
        Campaign {
            schema_version: 1,
            campaign_id: "paired-camera-drop-fixture-v1".to_string(),
            model_id: "untrained-camera-driving-input-fixture".to_string(),
            model_status: ModelStatus::UntrainedInputBridge,
            model_revision: "symthaea-camera-driving-bridge-v0.1".to_string(),
            checkpoint_sha256: None,
            hardware_id: "synthetic-fixture-host-not-measured".to_string(),
            inference_precision: "fp32-fixture".to_string(),
            power_profile: "not-measured-fixture".to_string(),
            energy_measurement_method: "synthetic-fixture-values-not-hardware-measurement"
                .to_string(),
            policy_visible_inputs: vec![
                "front-camera-rgb8".to_string(),
                "vehicle-state-vector".to_string(),
            ],
            benchmark_revision: "degradation-recovery-envelope-v0.1".to_string(),
            evaluator_revision: "auditor-v1".to_string(),
            clock_id: "monotonic-test-clock".to_string(),
            policy_deadline_us: 100_000,
            fault_schedule_visible_to_policy: false,
            clean_control: RunTrace {
                run_id: "clean-control".to_string(),
                disposition: RunDisposition::Complete,
                scenario_seed_sha256: "a".repeat(64),
                environment_randomness_sha256: "b".repeat(64),
                expected_tick_count: 8,
                ticks: clean,
            },
            faulted_run: RunTrace {
                run_id: "faulted-run".to_string(),
                disposition: RunDisposition::Complete,
                scenario_seed_sha256: "a".repeat(64),
                environment_randomness_sha256: "b".repeat(64),
                expected_tick_count: 8,
                ticks: faulted,
            },
            events: vec![FaultEvent {
                event_id: "camera-drop-1".to_string(),
                kind: FaultKind::CameraDrop,
                start_tick: 3,
                end_tick: 4,
                severity_ppm: 1_000_000,
                detection_deadline_ticks: 1,
                containment_deadline_ticks: 1,
                max_accepted_frame_age_us: 0,
                recovery_healthy_quorum_ticks: 2,
            }],
        }
    }

    fn has(report: &AuditReport, expected: &str) -> bool {
        report.findings.iter().any(|finding| finding == expected)
    }

    #[test]
    fn accepts_paired_clean_and_faulted_trace_with_recovery_quorum() {
        let report = audit_campaign(&campaign());
        assert_eq!(report.disposition, "valid");
        assert!(report.findings.is_empty());
        assert_eq!(report.events[0].detection_latency_ticks, Some(0));
        assert_eq!(report.events[0].containment_latency_ticks, Some(0));
        assert_eq!(report.events[0].recovery_tick, Some(7));
        assert_eq!(
            report.events[0].recovery_latency_after_clearance_ticks,
            Some(3)
        );
    }

    #[test]
    fn rejects_stale_frame_accepted_by_policy() {
        let mut input = campaign();
        input.faulted_run.ticks[2].conditions = vec![ObservationCondition::Stale];
        input.faulted_run.ticks[2].policy_input_accepted = true;
        input.events[0].kind = FaultKind::StaleReplay;
        let report = audit_campaign(&input);
        assert!(has(&report, "forbidden_policy_input_accepted"));
    }

    #[test]
    fn rejects_corrupted_frame_accepted_by_policy() {
        let mut input = campaign();
        input.faulted_run.ticks[2].conditions = vec![ObservationCondition::Corrupt];
        input.faulted_run.ticks[2].policy_input_accepted = true;
        input.events[0].kind = FaultKind::PixelCorruption;
        let report = audit_campaign(&input);
        assert!(has(&report, "forbidden_policy_input_accepted"));
    }

    #[test]
    fn rejects_missing_expected_fault_condition() {
        let mut input = campaign();
        input.faulted_run.ticks[2].conditions = vec![ObservationCondition::Clean];
        let report = audit_campaign(&input);
        assert!(has(&report, "expected_fault_condition_missing"));
    }

    #[test]
    fn late_or_missing_decision_latency_does_not_advance_recovery_quorum() {
        let mut input = campaign();
        input.faulted_run.ticks[4].decision_latency_us = Some(150_000);
        input.faulted_run.ticks[5].decision_latency_us = None;
        input.faulted_run.ticks[6].normal_mode = false;
        input.faulted_run.ticks[7].normal_mode = true;
        let report = audit_campaign(&input);
        assert_eq!(report.events[0].recovery_tick, Some(8));
        assert!(!has(&report, "premature_recovery"));
    }

    #[test]
    fn rejects_premature_return_to_normal_mode() {
        let mut input = campaign();
        input.faulted_run.ticks[4].normal_mode = true;
        input.faulted_run.ticks[4].degraded_mode = false;
        input.faulted_run.ticks[5].normal_mode = false;
        let report = audit_campaign(&input);
        assert!(has(&report, "premature_recovery"));
    }

    #[test]
    fn late_detection_is_counted_as_a_deadline_miss_not_as_undetected() {
        let mut input = campaign();
        input.faulted_run.ticks[2].fault_detection_event_ids.clear();
        input.faulted_run.ticks[6].fault_detection_event_ids = vec!["camera-drop-1".to_string()];
        let report = audit_campaign(&input);
        assert!(has(&report, "detection_deadline_missed"));
        assert!(!has(&report, "fault_undetected"));
        assert_eq!(report.events[0].detection_latency_ticks, Some(4));
    }

    #[test]
    fn one_fault_event_cannot_claim_another_events_detection() {
        let mut input = campaign();
        input.faulted_run.ticks[2].fault_detection_event_ids = vec!["unrelated-event".to_string()];
        input.faulted_run.ticks[2].containment_event_ids = vec!["unrelated-event".to_string()];
        let report = audit_campaign(&input);
        assert!(has(&report, "fault_undetected"));
        assert!(has(&report, "unknown_event_reference"));
    }

    #[test]
    fn rejects_unobserved_fault_detection_and_containment() {
        let mut input = campaign();
        for record in &mut input.faulted_run.ticks[2..4] {
            record.fault_detection_event_ids.clear();
            record.containment_event_ids.clear();
            record.degraded_mode = false;
            record.fallback_active = false;
        }
        let report = audit_campaign(&input);
        assert!(has(&report, "fault_undetected"));
        assert!(has(&report, "containment_deadline_missed"));
    }

    #[test]
    fn rejects_clean_control_that_never_returns_to_normal_mode() {
        let mut input = campaign();
        input.clean_control.ticks[4].normal_mode = false;
        input.clean_control.ticks[4].degraded_mode = true;
        let report = audit_campaign(&input);
        assert!(has(&report, "clean_control_not_normal"));
        assert!(has(&report, "false_positive_degradation"));
    }

    #[test]
    fn rejects_missing_per_tick_latency_or_energy_evidence() {
        let mut input = campaign();
        input.faulted_run.ticks[2].decision_latency_us = None;
        input.faulted_run.ticks[3].inference_energy_uj = None;
        let report = audit_campaign(&input);
        assert!(has(&report, "decision_latency_missing"));
        assert!(has(&report, "inference_energy_missing"));
    }

    #[test]
    fn rejects_fault_condition_without_a_scheduled_event() {
        let mut input = campaign();
        input.faulted_run.ticks[5].conditions = vec![
            ObservationCondition::Clean,
            ObservationCondition::ComputeOverrun,
        ];
        let report = audit_campaign(&input);
        assert!(has(&report, "clean_condition_mixed_with_fault"));
        assert!(has(&report, "undeclared_fault_condition"));
    }

    #[test]
    fn rejects_detection_reference_before_fault_onset() {
        let mut input = campaign();
        input.faulted_run.ticks[0].fault_detection_event_ids = vec!["camera-drop-1".to_string()];
        let report = audit_campaign(&input);
        assert!(has(&report, "fault_event_reference_before_onset"));
    }

    #[test]
    fn rejects_clean_control_with_sensor_fault_conditions() {
        let mut input = campaign();
        input.clean_control.ticks[2].conditions = vec![ObservationCondition::Stale];
        input.clean_control.ticks[2].policy_input_accepted = false;
        let report = audit_campaign(&input);
        assert!(has(&report, "clean_control_not_clean"));
    }

    #[test]
    fn rejects_tick_zero_and_out_of_range_sequences_as_sequence_errors() {
        let mut input = campaign();
        input.faulted_run.ticks[0].tick = 0;
        input.faulted_run.ticks[1].tick = 99;
        let report = audit_campaign(&input);
        assert!(has(&report, "tick_sequence_mismatch"));
    }

    #[test]
    fn rejects_clean_control_false_positive_degradation() {
        let mut input = campaign();
        input.clean_control.ticks[3].degraded_mode = true;
        let report = audit_campaign(&input);
        assert!(has(&report, "false_positive_degradation"));
    }

    #[test]
    fn rejects_unmatched_randomness_between_paired_runs() {
        let mut input = campaign();
        input.faulted_run.environment_randomness_sha256 = "c".repeat(64);
        let report = audit_campaign(&input);
        assert!(has(&report, "environment_randomness_mismatch"));
    }

    #[test]
    fn rejects_fault_schedule_leakage_to_policy() {
        let mut input = campaign();
        input.fault_schedule_visible_to_policy = true;
        let report = audit_campaign(&input);
        assert!(has(&report, "fault_schedule_exposed_to_policy"));
    }

    #[test]
    fn trained_policy_requires_a_checkpoint_digest() {
        let mut input = campaign();
        input.model_status = ModelStatus::TrainedPolicy;
        input.checkpoint_sha256 = None;
        let report = audit_campaign(&input);
        assert!(has(&report, "missing_checkpoint_for_trained_policy"));
    }

    #[test]
    fn rejects_duplicate_policy_visible_input_declarations() {
        let mut input = campaign();
        input
            .policy_visible_inputs
            .push("front-camera-rgb8".to_string());
        let report = audit_campaign(&input);
        assert!(has(&report, "duplicate_policy_input"));
    }

    #[test]
    fn rejects_zero_policy_deadline_and_malformed_seed_digest() {
        let mut input = campaign();
        input.policy_deadline_us = 0;
        input.clean_control.scenario_seed_sha256 = "not-a-digest".to_string();
        let report = audit_campaign(&input);
        assert!(has(&report, "invalid_policy_deadline"));
        assert!(has(&report, "invalid_digest"));
    }

    #[test]
    fn delayed_frame_is_only_accepted_inside_declared_age_budget() {
        let mut input = campaign();
        input.events[0] = FaultEvent {
            event_id: "frame-delay-1".to_string(),
            kind: FaultKind::FrameDelay,
            start_tick: 3,
            end_tick: 4,
            severity_ppm: 500_000,
            detection_deadline_ticks: 1,
            containment_deadline_ticks: 1,
            max_accepted_frame_age_us: 1_000,
            recovery_healthy_quorum_ticks: 2,
        };
        for record in &mut input.faulted_run.ticks[2..4] {
            record.conditions = vec![ObservationCondition::Delayed];
            record.policy_input_accepted = true;
            record.delivered_timestamp_us = record.capture_timestamp_us + 700;
        }
        let report = audit_campaign(&input);
        assert!(!has(&report, "delayed_frame_accepted_too_old"));
        input.faulted_run.ticks[2].delivered_timestamp_us =
            input.faulted_run.ticks[2].capture_timestamp_us + 1_001;
        let report = audit_campaign(&input);
        assert!(has(&report, "delayed_frame_accepted_too_old"));
    }

    #[test]
    fn rejects_incomplete_tick_denominator() {
        let mut input = campaign();
        input.faulted_run.ticks.pop();
        let report = audit_campaign(&input);
        assert!(has(&report, "tick_denominator_mismatch"));
        assert!(has(&report, "tick_sequence_mismatch"));
    }

    #[test]
    fn rejects_duplicate_event_id_and_out_of_range_severity() {
        let mut input = campaign();
        let mut second = input.events[0].clone();
        second.severity_ppm = 1_000_001;
        input.events.push(second);
        let report = audit_campaign(&input);
        assert!(has(&report, "duplicate_fault_event"));
        assert!(has(&report, "invalid_fault_severity"));
    }

    #[test]
    fn standard_schedule_covers_seven_fault_families_deterministically() {
        let first = standard_fault_schedule(10, 5, 3, 1, 1, 2).unwrap();
        let second = standard_fault_schedule(10, 5, 3, 1, 1, 2).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 7);
        assert_eq!(first[0].kind, FaultKind::CameraDrop);
        assert_eq!(first[6].kind, FaultKind::ComputeOverrun);
        assert!(first.windows(2).all(|pair| {
            pair[1].start_tick >= pair[0].end_tick + pair[0].recovery_healthy_quorum_ticks
        }));
    }

    #[test]
    fn standard_schedule_rejects_overlapping_recovery_windows() {
        let error = standard_fault_schedule(10, 4, 3, 1, 1, 2).unwrap_err();
        assert!(error.contains("too short"));
    }

    #[test]
    fn standard_schedule_rejects_tick_arithmetic_overflow() {
        assert!(standard_fault_schedule(u64::MAX - 2, 5, 3, 1, 1, 2).is_err());
    }

    #[test]
    fn reports_resource_axes_separately_without_composite_score() {
        let report = audit_campaign(&campaign());
        assert_eq!(report.clean_control_decision_latency.sample_count, 8);
        assert_eq!(report.faulted_decision_latency.sample_count, 8);
        assert_eq!(report.clean_control_inference_energy.p99_uj, Some(48));
        assert_eq!(report.faulted_inference_energy.p99_uj, Some(48));
    }

    #[test]
    fn rejects_containment_event_reference_without_degraded_or_fallback_state() {
        let mut input = campaign();
        input.faulted_run.ticks[2].degraded_mode = false;
        input.faulted_run.ticks[2].fallback_active = false;
        input.faulted_run.ticks[3].containment_event_ids.clear();
        let report = audit_campaign(&input);
        assert!(has(&report, "containment_reference_without_state"));
        assert!(has(&report, "containment_deadline_missed"));
    }

    #[test]
    fn rejects_normal_mode_while_fallback_remains_active() {
        let mut input = campaign();
        input.faulted_run.ticks[6].fallback_active = true;
        let report = audit_campaign(&input);
        assert!(has(&report, "normal_mode_with_containment_active"));
    }

    #[test]
    fn rejects_normal_mode_while_fault_is_active() {
        let mut input = campaign();
        input.faulted_run.ticks[2].normal_mode = true;
        let report = audit_campaign(&input);
        assert!(has(&report, "normal_mode_during_fault"));
    }
}
