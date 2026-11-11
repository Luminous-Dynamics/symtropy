//! Fault-conditioned degradation and recovery envelope auditor.
//!
//! The auditor checks paired clean/faulted traces and reports each recovery axis
//! separately. It is a record auditor, not a vehicle controller or safety authority.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    pub fault_detected: bool,
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
    pub benchmark_revision: String,
    pub evaluator_revision: String,
    pub clock_id: String,
    pub clean_control: RunTrace,
    pub faulted_run: RunTrace,
    pub events: Vec<FaultEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventMetrics {
    pub event_id: String,
    pub kind: FaultKind,
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
    pub disposition: String,
    pub claim_ceiling: String,
    pub expected_ticks: u64,
    pub clean_control_observed_ticks: usize,
    pub faulted_observed_ticks: usize,
    pub fault_event_count: usize,
    pub clean_control_false_positive_ticks: u64,
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
    DetectionDeadlineMissed,
    ContainmentDeadlineMissed,
    RecoveryNotObserved,
    PrematureRecovery,
    FalsePositiveDegradation,
}

impl Finding {
    fn as_str(self) -> &'static str {
        match self {
            Self::SchemaVersionMismatch => "schema_version_mismatch",
            Self::EmptyIdentity => "empty_identity",
            Self::DuplicateRunId => "duplicate_run_id",
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
            Self::DetectionDeadlineMissed => "detection_deadline_missed",
            Self::ContainmentDeadlineMissed => "containment_deadline_missed",
            Self::RecoveryNotObserved => "recovery_not_observed",
            Self::PrematureRecovery => "premature_recovery",
            Self::FalsePositiveDegradation => "false_positive_degradation",
        }
    }
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
    conditions.iter().any(|condition| targets.contains(condition))
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

fn is_healthy_tick(tick: &TickTrace) -> bool {
    tick.conditions.as_slice() == [ObservationCondition::Clean]
        && !tick.fault_detected
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

fn containment_active(tick: &TickTrace) -> bool {
    tick.degraded_mode || tick.fallback_active || tick.safe_stop_complete
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
        if tick.tick == 0 || tick.tick > expected_ticks || !seen.insert(tick.tick) {
            findings.insert(if !seen.contains(&tick.tick) {
                Finding::TickSequenceMismatch
            } else {
                Finding::DuplicateTick
            });
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
    if seen.len() as u64 != expected_ticks
        || seen.iter().copied().ne(1..=expected_ticks)
    {
        findings.insert(Finding::TickSequenceMismatch);
    }
}

fn event_metrics(
    event: &FaultEvent,
    run_ticks: &std::collections::BTreeMap<u64, &TickTrace>,
    findings: &mut BTreeSet<Finding>,
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
        .range(event.start_tick..=event.end_tick.saturating_add(event.detection_deadline_ticks))
        .find_map(|(tick, record)| record.fault_detected.then_some(*tick));
    let containment_tick = run_ticks
        .range(event.start_tick..=event.end_tick.saturating_add(event.containment_deadline_ticks))
        .find_map(|(tick, record)| containment_active(record).then_some(*tick));

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
    for (tick, record) in run_ticks.range((event.end_tick + 1)..) {
        if is_healthy_tick(record) {
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
                    &[
                        ObservationCondition::Stale,
                        ObservationCondition::Corrupt,
                    ],
                )
        })
        .count() as u64;

    EventMetrics {
        event_id: event.event_id.clone(),
        kind: event.kind,
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
            || containment_latency.is_some_and(|latency| latency > event.containment_deadline_ticks),
        recovery_observed: recovery_tick.is_some(),
    }
}

pub fn audit_campaign(campaign: &Campaign) -> AuditReport {
    let mut findings = BTreeSet::new();
    if campaign.schema_version != 1 {
        findings.insert(Finding::SchemaVersionMismatch);
    }
    for identity in [
        campaign.campaign_id.as_str(),
        campaign.benchmark_revision.as_str(),
        campaign.evaluator_revision.as_str(),
        campaign.clock_id.as_str(),
        campaign.clean_control.run_id.as_str(),
        campaign.faulted_run.run_id.as_str(),
        campaign.clean_control.scenario_seed_sha256.as_str(),
        campaign.faulted_run.scenario_seed_sha256.as_str(),
        campaign.clean_control.environment_randomness_sha256.as_str(),
        campaign.faulted_run.environment_randomness_sha256.as_str(),
    ] {
        if identity.trim().is_empty() {
            findings.insert(Finding::EmptyIdentity);
        }
    }
    if campaign.clean_control.run_id == campaign.faulted_run.run_id {
        findings.insert(Finding::DuplicateRunId);
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
            tick.fault_detected || tick.degraded_mode || tick.fallback_active || tick.safe_stop_complete
        })
        .count() as u64;
    if control_false_positive_ticks > 0 {
        findings.insert(Finding::FalsePositiveDegradation);
    }
    for tick in &campaign.faulted_run.ticks {
        let active_fault = campaign
            .events
            .iter()
            .any(|event| (event.start_tick..=event.end_tick).contains(&tick.tick));
        if active_fault && tick.normal_mode {
            findings.insert(Finding::NormalModeDuringFault);
        }
        let age = tick.delivered_timestamp_us.saturating_sub(tick.capture_timestamp_us);
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
        events.push(event_metrics(event, &faulted_tick_map, &mut findings));
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
    let faulted_deadline_misses = campaign
        .faulted_run
        .ticks
        .iter()
        .filter(|tick| {
            tick.decision_latency_us
                .is_some_and(|latency| latency > 100_000)
        })
        .count() as u64;

    let clean_latency = latency_summary(&campaign.clean_control.ticks);
    let faulted_latency = latency_summary(&campaign.faulted_run.ticks);
    let clean_energy = energy_summary(&campaign.clean_control.ticks);
    let faulted_energy = energy_summary(&campaign.faulted_run.ticks);

    AuditReport {
        schema_version: 1,
        campaign_id: campaign.campaign_id.clone(),
        disposition: if findings.is_empty() { "valid" } else { "rejected" }.to_string(),
        claim_ceiling: "Finite paired trace consistency and declared software fault accounting only. This audit does not prove physical fault fidelity, trained-policy detection, safe-stop performance, real-road transfer or vehicle safety.".to_string(),
        expected_ticks,
        clean_control_observed_ticks: campaign.clean_control.ticks.len(),
        faulted_observed_ticks: campaign.faulted_run.ticks.len(),
        fault_event_count: campaign.events.len(),
        clean_control_false_positive_ticks: control_false_positive_ticks,
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
            fault_detected,
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
            tick(1, vec![ObservationCondition::Clean], true, false, false, false, true),
            tick(2, vec![ObservationCondition::Clean], true, false, false, false, true),
            tick(3, vec![ObservationCondition::Missing], false, true, true, true, false),
            tick(4, vec![ObservationCondition::Missing], false, false, true, true, false),
            tick(5, vec![ObservationCondition::Clean], true, false, true, true, false),
            tick(6, vec![ObservationCondition::Clean], true, false, true, true, false),
            tick(7, vec![ObservationCondition::Clean], true, false, false, false, true),
            tick(8, vec![ObservationCondition::Clean], true, false, false, false, true),
        ];
        Campaign {
            schema_version: 1,
            campaign_id: "paired-camera-drop-fixture-v1".to_string(),
            benchmark_revision: "degradation-recovery-envelope-v0.1".to_string(),
            evaluator_revision: "auditor-v1".to_string(),
            clock_id: "monotonic-test-clock".to_string(),
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
        assert_eq!(report.events[0].recovery_latency_after_clearance_ticks, Some(3));
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
    fn rejects_premature_return_to_normal_mode() {
        let mut input = campaign();
        input.faulted_run.ticks[4].normal_mode = true;
        input.faulted_run.ticks[4].degraded_mode = false;
        input.faulted_run.ticks[5].normal_mode = false;
        let report = audit_campaign(&input);
        assert!(has(&report, "premature_recovery"));
    }

    #[test]
    fn rejects_unobserved_fault_detection_and_containment() {
        let mut input = campaign();
        for record in &mut input.faulted_run.ticks[2..4] {
            record.fault_detected = false;
            record.degraded_mode = false;
            record.fallback_active = false;
        }
        let report = audit_campaign(&input);
        assert!(has(&report, "fault_undetected"));
        assert!(has(&report, "containment_deadline_missed"));
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
    fn reports_resource_axes_separately_without_composite_score() {
        let report = audit_campaign(&campaign());
        assert_eq!(report.clean_control_decision_latency.sample_count, 8);
        assert_eq!(report.faulted_decision_latency.sample_count, 8);
        assert_eq!(report.clean_control_inference_energy.p99_uj, Some(48));
        assert_eq!(report.faulted_inference_energy.p99_uj, Some(48));
    }

    #[test]
    fn rejects_normal_mode_while_fault_is_active() {
        let mut input = campaign();
        input.faulted_run.ticks[2].normal_mode = true;
        let report = audit_campaign(&input);
        assert!(has(&report, "normal_mode_during_fault"));
    }
}
