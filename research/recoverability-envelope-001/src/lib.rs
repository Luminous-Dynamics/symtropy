//! Sequence-level fault, graceful-degradation and recovery evaluator.
//!
//! This crate evaluates finite declared traces. It does not run vehicle dynamics,
//! authenticate injected faults, determine physical safety, or authorize actuation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SCHEMA_VERSION: u32 = 1;
pub const MAX_TICKS: usize = 100_000;
pub const MAX_FAULT_EVENTS: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultKind {
    CameraFrameDrop,
    CameraPartialObservation,
    CameraStaleness,
    CameraCalibrationShift,
    GpsNoise,
    SpeedBias,
    OdometryDrift,
    InferenceDelay,
    DeadlineOverrun,
    CommunicationDrop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingMode {
    Nominal,
    Degraded,
    Fallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleOutcome {
    WithinEnvelope,
    DegradedWithinEnvelope,
    ConstraintViolation,
    Unscored,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PairingIdentity {
    pub pair_id: String,
    pub baseline_run_id: String,
    pub world_seed_sha256: String,
    pub baseline_checkpoint_sha256: String,
    pub candidate_checkpoint_sha256: String,
    pub simulator_revision: String,
    pub evaluator_revision: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryContract {
    pub max_detection_delay_ticks: u64,
    pub max_fallback_delay_ticks: u64,
    pub max_recovery_after_end_ticks: u64,
    pub min_stable_nominal_ticks: u64,
    pub max_camera_age_us: u64,
    pub max_vehicle_state_age_us: u64,
    pub max_deadline_miss_ticks_during_fault: u64,
    pub max_fault_concurrency: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FaultEvent {
    pub fault_id: String,
    pub kind: FaultKind,
    pub channel: String,
    pub onset_tick: u64,
    /// Exclusive end tick. An event affects onset_tick .. end_tick_exclusive.
    pub end_tick_exclusive: u64,
    pub severity_ppm: u32,
    pub correlation_group: Option<String>,
    pub critical_phase: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TickTrace {
    pub tick: u64,
    /// Frozen scenario phase label, e.g. approach, merge, yield, emergency_braking.
    pub maneuver_phase: String,
    pub camera_age_us: Option<u64>,
    pub vehicle_state_age_us: Option<u64>,
    pub inference_duration_us: u64,
    pub control_deadline_us: u64,
    pub detected_fault_ids: Vec<String>,
    pub fallback_reason_fault_ids: Vec<String>,
    pub mode: OperatingMode,
    pub oracle_outcome: OracleOutcome,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryEpisode {
    pub schema_version: u32,
    pub run_id: String,
    pub model_arm_id: String,
    pub benchmark_id: String,
    pub scenario_family: String,
    pub policy_revision: String,
    pub pairing: PairingIdentity,
    pub expected_ticks: usize,
    pub contract: RecoveryContract,
    pub fault_events: Vec<FaultEvent>,
    pub trace: Vec<TickTrace>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FaultResult {
    pub fault_id: String,
    pub kind: FaultKind,
    pub channel: String,
    pub onset_tick: u64,
    pub end_tick_exclusive: u64,
    pub duration_ticks: u64,
    pub severity_ppm: u32,
    pub correlation_group: Option<String>,
    pub critical_phase: Option<String>,
    pub onset_in_critical_phase: Option<bool>,
    pub critical_phase_overlap_ticks: Option<u64>,
    pub critical_phase_fallback_ticks: Option<u64>,
    pub critical_phase_deadline_miss_ticks: Option<u64>,
    pub critical_phase_constraint_violation_ticks: Option<u64>,
    pub detected: bool,
    pub detection_tick: Option<u64>,
    pub detection_delay_ticks: Option<u64>,
    pub fallback_observed: bool,
    pub fallback_tick: Option<u64>,
    pub fallback_delay_ticks: Option<u64>,
    pub recovery_tick: Option<u64>,
    pub recovery_latency_ticks: Option<u64>,
    pub fallback_ticks_until_recovery: u64,
    pub degraded_ticks_during_fault: u64,
    pub deadline_miss_ticks_during_fault: u64,
    pub constraint_violation_ticks: u64,
    pub first_constraint_violation_tick: Option<u64>,
    pub contract_met: bool,
    pub unmet_conditions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecoveryReport {
    pub schema_version: u32,
    pub status: String,
    pub run_id: String,
    pub model_arm_id: String,
    pub benchmark_id: String,
    pub scenario_family: String,
    pub pair_id: String,
    pub baseline_run_id: String,
    pub expected_ticks: usize,
    pub observed_ticks: usize,
    pub fault_event_count: usize,
    pub correlation_group_count: usize,
    pub maximum_concurrent_faults: usize,
    pub detected_fault_count: usize,
    pub fallback_fault_count: usize,
    pub recovered_fault_count: usize,
    pub contract_met_fault_count: usize,
    pub total_constraint_violation_ticks: u64,
    pub deadline_miss_ticks: u64,
    pub outcome_unscored_ticks: u64,
    pub results: Vec<FaultResult>,
    pub claim_ceiling: String,
}

impl RecoveryEpisode {
    pub fn from_json(input: &str) -> Result<Self, String> {
        serde_json::from_str(input).map_err(|error| format!("recovery episode JSON: {error}"))
    }

    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.schema_version != SCHEMA_VERSION {
            errors.push(format!(
                "schema_version must be {SCHEMA_VERSION}, got {}",
                self.schema_version
            ));
        }

        for (name, value) in [
            ("run_id", self.run_id.as_str()),
            ("model_arm_id", self.model_arm_id.as_str()),
            ("benchmark_id", self.benchmark_id.as_str()),
            ("scenario_family", self.scenario_family.as_str()),
            ("policy_revision", self.policy_revision.as_str()),
            ("pairing.pair_id", self.pairing.pair_id.as_str()),
            (
                "pairing.baseline_run_id",
                self.pairing.baseline_run_id.as_str(),
            ),
            (
                "pairing.simulator_revision",
                self.pairing.simulator_revision.as_str(),
            ),
            (
                "pairing.evaluator_revision",
                self.pairing.evaluator_revision.as_str(),
            ),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("{name} must be non-empty"));
            }
        }

        for (name, digest) in [
            ("pairing.world_seed_sha256", &self.pairing.world_seed_sha256),
            (
                "pairing.baseline_checkpoint_sha256",
                &self.pairing.baseline_checkpoint_sha256,
            ),
            (
                "pairing.candidate_checkpoint_sha256",
                &self.pairing.candidate_checkpoint_sha256,
            ),
        ] {
            if !is_sha256(digest) {
                errors.push(format!("{name} must be a lowercase SHA-256 digest"));
            }
        }

        if self.expected_ticks == 0 || self.expected_ticks > MAX_TICKS {
            errors.push(format!("expected_ticks must be in 1..={MAX_TICKS}"));
        }
        if self.trace.len() != self.expected_ticks {
            errors.push(format!(
                "tick denominator mismatch: expected {}, observed {}",
                self.expected_ticks,
                self.trace.len()
            ));
        }
        if self.fault_events.len() > MAX_FAULT_EVENTS {
            errors.push(format!("fault event count exceeds {MAX_FAULT_EVENTS}"));
        }
        if self.contract.min_stable_nominal_ticks == 0 {
            errors.push("min_stable_nominal_ticks must be at least 1".into());
        }
        if self.contract.max_fault_concurrency == 0 {
            errors.push("max_fault_concurrency must be at least 1".into());
        }

        for (index, tick) in self.trace.iter().enumerate() {
            let expected_tick = index as u64 + 1;
            if tick.tick != expected_tick {
                errors.push(format!(
                    "trace tick at index {index} must be {expected_tick}, got {}",
                    tick.tick
                ));
            }
            if tick.maneuver_phase.trim().is_empty() {
                errors.push(format!("tick {} has an empty maneuver_phase", tick.tick));
            }
            if tick.control_deadline_us == 0 {
                errors.push(format!("tick {} has zero control deadline", tick.tick));
            }
            if duplicate(&tick.detected_fault_ids) {
                errors.push(format!(
                    "tick {} contains duplicate detected fault IDs",
                    tick.tick
                ));
            }
            if duplicate(&tick.fallback_reason_fault_ids) {
                errors.push(format!(
                    "tick {} contains duplicate fallback-reason fault IDs",
                    tick.tick
                ));
            }
            if tick.mode == OperatingMode::Fallback && tick.fallback_reason_fault_ids.is_empty() {
                errors.push(format!(
                    "fallback mode at tick {} must declare at least one fault reason",
                    tick.tick
                ));
            }
            if tick.mode != OperatingMode::Fallback && !tick.fallback_reason_fault_ids.is_empty() {
                errors.push(format!(
                    "non-fallback mode at tick {} cannot declare fallback reasons",
                    tick.tick
                ));
            }
        }

        let mut ids = BTreeSet::new();
        for event in &self.fault_events {
            if event.fault_id.trim().is_empty()
                || event.channel.trim().is_empty()
                || !ids.insert(event.fault_id.as_str())
            {
                errors.push(format!(
                    "fault event has empty or duplicate identity {:?}",
                    event.fault_id
                ));
            }
            if event.onset_tick == 0
                || event.end_tick_exclusive <= event.onset_tick
                || event.end_tick_exclusive > self.expected_ticks as u64 + 1
            {
                errors.push(format!(
                    "fault {} has invalid interval [{}, {}) for {} expected ticks",
                    event.fault_id, event.onset_tick, event.end_tick_exclusive, self.expected_ticks
                ));
            }
            if event.severity_ppm == 0 || event.severity_ppm > 1_000_000 {
                errors.push(format!(
                    "fault {} severity_ppm must be in 1..=1000000",
                    event.fault_id
                ));
            }
            if let Some(critical_phase) = event.critical_phase.as_deref() {
                let phase_overlap = self.trace.iter().any(|tick| {
                    tick.tick >= event.onset_tick
                        && tick.tick < event.end_tick_exclusive
                        && tick.maneuver_phase == critical_phase
                });
                if !phase_overlap {
                    errors.push(format!(
                        "fault {} declares critical_phase {:?} but does not overlap that phase",
                        event.fault_id, critical_phase
                    ));
                }
            }
            if event
                .correlation_group
                .as_ref()
                .is_some_and(|group| group.trim().is_empty())
            {
                errors.push(format!(
                    "fault {} has an empty correlation_group",
                    event.fault_id
                ));
            }
            if event
                .critical_phase
                .as_ref()
                .is_some_and(|phase| phase.trim().is_empty())
            {
                errors.push(format!(
                    "fault {} has an empty critical_phase",
                    event.fault_id
                ));
            }
        }

        for tick in &self.trace {
            for fault_id in &tick.detected_fault_ids {
                match self
                    .fault_events
                    .iter()
                    .find(|event| event.fault_id == *fault_id)
                {
                    None => errors.push(format!(
                        "tick {} detects unknown fault ID {:?}",
                        tick.tick, fault_id
                    )),
                    Some(event) => {
                        let last_detection_tick = event
                            .end_tick_exclusive
                            .saturating_add(self.contract.max_detection_delay_ticks);
                        if tick.tick < event.onset_tick || tick.tick > last_detection_tick {
                            errors.push(format!(
                                "fault {} is detected outside its onset/detection window at tick {}",
                                fault_id, tick.tick
                            ));
                        }
                    }
                }
            }
            for fault_id in &tick.fallback_reason_fault_ids {
                match self
                    .fault_events
                    .iter()
                    .find(|event| event.fault_id == *fault_id)
                {
                    None => errors.push(format!(
                        "tick {} attributes fallback to unknown fault ID {:?}",
                        tick.tick, fault_id
                    )),
                    Some(event) => {
                        let last_fallback_tick = event
                            .end_tick_exclusive
                            .saturating_add(self.contract.max_fallback_delay_ticks);
                        if tick.tick < event.onset_tick || tick.tick > last_fallback_tick {
                            errors.push(format!(
                                "fault {} is attributed as a fallback reason outside its onset/fallback window at tick {}",
                                fault_id, tick.tick
                            ));
                        }
                    }
                }
            }
        }

        if self.expected_ticks > 0 && self.expected_ticks <= MAX_TICKS {
            let maximum = maximum_concurrency(&self.fault_events, self.expected_ticks as u64);
            if maximum > self.contract.max_fault_concurrency {
                errors.push(format!(
                    "fault concurrency {maximum} exceeds contract maximum {}",
                    self.contract.max_fault_concurrency
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            errors.sort();
            errors.dedup();
            Err(errors)
        }
    }

    pub fn evaluate(&self) -> Result<RecoveryReport, Vec<String>> {
        self.validate()?;
        let mut results = Vec::with_capacity(self.fault_events.len());

        for event in &self.fault_events {
            results.push(evaluate_fault(self, event));
        }

        let correlation_groups = self
            .fault_events
            .iter()
            .filter_map(|event| event.correlation_group.as_deref())
            .collect::<BTreeSet<_>>();
        let maximum_concurrent_faults =
            maximum_concurrency(&self.fault_events, self.expected_ticks as u64);

        let detected_fault_count = results.iter().filter(|result| result.detected).count();
        let fallback_fault_count = results
            .iter()
            .filter(|result| result.fallback_observed)
            .count();
        let recovered_fault_count = results
            .iter()
            .filter(|result| result.recovery_tick.is_some())
            .count();
        let contract_met_fault_count = results.iter().filter(|result| result.contract_met).count();

        let total_constraint_violation_ticks = self
            .trace
            .iter()
            .filter(|tick| tick.oracle_outcome == OracleOutcome::ConstraintViolation)
            .count() as u64;
        let deadline_miss_ticks = self
            .trace
            .iter()
            .filter(|tick| tick.inference_duration_us > tick.control_deadline_us)
            .count() as u64;
        let outcome_unscored_ticks = self
            .trace
            .iter()
            .filter(|tick| tick.oracle_outcome == OracleOutcome::Unscored)
            .count() as u64;

        Ok(RecoveryReport {
            schema_version: SCHEMA_VERSION,
            status: "structurally_valid_trace".to_string(),
            run_id: self.run_id.clone(),
            model_arm_id: self.model_arm_id.clone(),
            benchmark_id: self.benchmark_id.clone(),
            scenario_family: self.scenario_family.clone(),
            pair_id: self.pairing.pair_id.clone(),
            baseline_run_id: self.pairing.baseline_run_id.clone(),
            expected_ticks: self.expected_ticks,
            observed_ticks: self.trace.len(),
            fault_event_count: self.fault_events.len(),
            correlation_group_count: correlation_groups.len(),
            maximum_concurrent_faults,
            detected_fault_count,
            fallback_fault_count,
            recovered_fault_count,
            contract_met_fault_count,
            total_constraint_violation_ticks,
            deadline_miss_ticks,
            outcome_unscored_ticks,
            results,
            claim_ceiling: "Finite trace and declared-oracle evaluation only. This report does not run a simulator, authenticate fault injection, prove a physical safety envelope, estimate real-world collision risk, validate a learned controller, or authorize actuation.".to_string(),
        })
    }
}

fn evaluate_fault(episode: &RecoveryEpisode, event: &FaultEvent) -> FaultResult {
    let detection_tick = episode
        .trace
        .iter()
        .find(|tick| {
            tick.detected_fault_ids
                .iter()
                .any(|id| id == &event.fault_id)
        })
        .map(|tick| tick.tick);
    let detection_delay_ticks = detection_tick.map(|tick| tick.saturating_sub(event.onset_tick));

    let fallback_tick = episode
        .trace
        .iter()
        .find(|tick| {
            tick.tick >= event.onset_tick
                && tick.mode == OperatingMode::Fallback
                && tick
                    .fallback_reason_fault_ids
                    .iter()
                    .any(|id| id == &event.fault_id)
        })
        .map(|tick| tick.tick);
    let fallback_delay_ticks = fallback_tick.map(|tick| tick.saturating_sub(event.onset_tick));

    let first_constraint_violation_tick = episode
        .trace
        .iter()
        .find(|tick| {
            tick.tick >= event.onset_tick
                && tick.tick < event.end_tick_exclusive
                && tick.oracle_outcome == OracleOutcome::ConstraintViolation
        })
        .map(|tick| tick.tick);
    let constraint_violation_ticks = episode
        .trace
        .iter()
        .filter(|tick| {
            tick.tick >= event.onset_tick
                && tick.tick < event.end_tick_exclusive
                && tick.oracle_outcome == OracleOutcome::ConstraintViolation
        })
        .count() as u64;

    let degraded_ticks_during_fault = episode
        .trace
        .iter()
        .filter(|tick| {
            tick.tick >= event.onset_tick
                && tick.tick < event.end_tick_exclusive
                && tick.mode != OperatingMode::Nominal
        })
        .count() as u64;
    let deadline_miss_ticks_during_fault = episode
        .trace
        .iter()
        .filter(|tick| {
            tick.tick >= event.onset_tick
                && tick.tick < event.end_tick_exclusive
                && tick.inference_duration_us > tick.control_deadline_us
        })
        .count() as u64;

    let critical_phase_metrics = event.critical_phase.as_deref().map(|phase| {
        let phase_ticks = episode.trace.iter().filter(|tick| {
            tick.tick >= event.onset_tick
                && tick.tick < event.end_tick_exclusive
                && tick.maneuver_phase == phase
        });
        let ticks = phase_ticks.collect::<Vec<_>>();
        (
            ticks
                .first()
                .is_some_and(|tick| tick.tick == event.onset_tick),
            ticks.len() as u64,
            ticks
                .iter()
                .filter(|tick| {
                    tick.mode == OperatingMode::Fallback
                        && tick
                            .fallback_reason_fault_ids
                            .iter()
                            .any(|id| id == &event.fault_id)
                })
                .count() as u64,
            ticks
                .iter()
                .filter(|tick| tick.inference_duration_us > tick.control_deadline_us)
                .count() as u64,
            ticks
                .iter()
                .filter(|tick| tick.oracle_outcome == OracleOutcome::ConstraintViolation)
                .count() as u64,
        )
    });

    let recovery_tick = find_stable_recovery(episode, event);
    let recovery_latency_ticks =
        recovery_tick.map(|tick| tick.saturating_sub(event.end_tick_exclusive));

    let fallback_ticks_until_recovery = episode
        .trace
        .iter()
        .filter(|tick| {
            tick.tick >= event.onset_tick
                && tick.tick < recovery_tick.unwrap_or(u64::MAX)
                && tick.mode == OperatingMode::Fallback
                && tick
                    .fallback_reason_fault_ids
                    .iter()
                    .any(|id| id == &event.fault_id)
        })
        .count() as u64;

    let mut unmet_conditions = Vec::new();
    match detection_delay_ticks {
        None => unmet_conditions.push("fault_not_detected".to_string()),
        Some(delay) if delay > episode.contract.max_detection_delay_ticks => {
            unmet_conditions.push("detection_delay_exceeded".to_string())
        }
        Some(_) => {}
    }
    match fallback_delay_ticks {
        None => unmet_conditions.push("fallback_not_entered".to_string()),
        Some(delay) if delay > episode.contract.max_fallback_delay_ticks => {
            unmet_conditions.push("fallback_delay_exceeded".to_string())
        }
        Some(_) => {}
    }
    match recovery_latency_ticks {
        None => unmet_conditions.push("stable_recovery_not_observed".to_string()),
        Some(delay) if delay > episode.contract.max_recovery_after_end_ticks => {
            unmet_conditions.push("recovery_deadline_exceeded".to_string())
        }
        Some(_) => {}
    }
    if constraint_violation_ticks > 0 {
        unmet_conditions.push("constraint_violation_during_fault".to_string());
    }
    if deadline_miss_ticks_during_fault > episode.contract.max_deadline_miss_ticks_during_fault {
        unmet_conditions.push("deadline_miss_budget_exceeded".to_string());
    }

    let contract_met = unmet_conditions.is_empty();
    FaultResult {
        fault_id: event.fault_id.clone(),
        kind: event.kind,
        channel: event.channel.clone(),
        onset_tick: event.onset_tick,
        end_tick_exclusive: event.end_tick_exclusive,
        duration_ticks: event.end_tick_exclusive.saturating_sub(event.onset_tick),
        severity_ppm: event.severity_ppm,
        correlation_group: event.correlation_group.clone(),
        critical_phase: event.critical_phase.clone(),
        onset_in_critical_phase: critical_phase_metrics.map(|metrics| metrics.0),
        critical_phase_overlap_ticks: critical_phase_metrics.map(|metrics| metrics.1),
        critical_phase_fallback_ticks: critical_phase_metrics.map(|metrics| metrics.2),
        critical_phase_deadline_miss_ticks: critical_phase_metrics.map(|metrics| metrics.3),
        critical_phase_constraint_violation_ticks: critical_phase_metrics.map(|metrics| metrics.4),
        detected: detection_tick.is_some(),
        detection_tick,
        detection_delay_ticks,
        fallback_observed: fallback_tick.is_some(),
        fallback_tick,
        fallback_delay_ticks,
        recovery_tick,
        recovery_latency_ticks,
        fallback_ticks_until_recovery,
        degraded_ticks_during_fault,
        deadline_miss_ticks_during_fault,
        constraint_violation_ticks,
        first_constraint_violation_tick,
        contract_met,
        unmet_conditions,
    }
}

fn find_stable_recovery(episode: &RecoveryEpisode, event: &FaultEvent) -> Option<u64> {
    let needed = episode.contract.min_stable_nominal_ticks as usize;
    let start_tick = event.end_tick_exclusive;
    let mut stable_count = 0_u64;

    for tick in episode.trace.iter().filter(|tick| tick.tick >= start_tick) {
        let no_scheduled_fault_active = !episode
            .fault_events
            .iter()
            .any(|other| tick.tick >= other.onset_tick && tick.tick < other.end_tick_exclusive);
        let stable = no_scheduled_fault_active
            && tick.mode == OperatingMode::Nominal
            && tick.oracle_outcome == OracleOutcome::WithinEnvelope
            && tick.inference_duration_us <= tick.control_deadline_us
            && tick
                .camera_age_us
                .is_some_and(|age| age <= episode.contract.max_camera_age_us)
            && tick
                .vehicle_state_age_us
                .is_some_and(|age| age <= episode.contract.max_vehicle_state_age_us);

        if stable {
            stable_count += 1;
            if stable_count >= needed as u64 {
                return Some(tick.tick);
            }
        } else {
            stable_count = 0;
        }
    }
    None
}

fn maximum_concurrency(events: &[FaultEvent], expected_ticks: u64) -> usize {
    (1..=expected_ticks)
        .map(|tick| {
            events
                .iter()
                .filter(|event| tick >= event.onset_tick && tick < event.end_tick_exclusive)
                .count()
        })
        .max()
        .unwrap_or(0)
}

fn duplicate<T: Ord + Clone>(values: &[T]) -> bool {
    let mut sorted = values.to_vec();
    sorted.sort();
    sorted.windows(2).any(|pair| pair[0] == pair[1])
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_episode() -> RecoveryEpisode {
        let events = vec![
            FaultEvent {
                fault_id: "camera-drop".into(),
                kind: FaultKind::CameraFrameDrop,
                channel: "front-camera".into(),
                onset_tick: 2,
                end_tick_exclusive: 4,
                severity_ppm: 600_000,
                correlation_group: Some("compound-sensor-compute-burst".into()),
                critical_phase: Some("merge".into()),
            },
            FaultEvent {
                fault_id: "inference-delay".into(),
                kind: FaultKind::InferenceDelay,
                channel: "policy-compute".into(),
                onset_tick: 3,
                end_tick_exclusive: 6,
                severity_ppm: 450_000,
                correlation_group: Some("compound-sensor-compute-burst".into()),
                critical_phase: Some("merge".into()),
            },
        ];

        let mut trace = Vec::new();
        for tick in 1..=10 {
            let active = events
                .iter()
                .any(|event| tick >= event.onset_tick && tick < event.end_tick_exclusive);
            let mode = match tick {
                1 => OperatingMode::Nominal,
                2 => OperatingMode::Degraded,
                3..=5 => OperatingMode::Fallback,
                _ => OperatingMode::Nominal,
            };
            let detected_fault_ids = match tick {
                2 => vec!["camera-drop".into()],
                3 => vec!["inference-delay".into()],
                _ => vec![],
            };
            trace.push(TickTrace {
                tick,
                maneuver_phase: match tick {
                    1 => "approach".into(),
                    2..=5 => "merge".into(),
                    6 => "exit-merge".into(),
                    _ => "recovery".into(),
                },
                camera_age_us: if tick == 2 || tick == 3 {
                    None
                } else {
                    Some(1_000)
                },
                vehicle_state_age_us: Some(1_000),
                inference_duration_us: if tick == 3 { 15_000 } else { 4_000 },
                control_deadline_us: 10_000,
                detected_fault_ids,
                fallback_reason_fault_ids: match tick {
                    3 => vec!["camera-drop".into(), "inference-delay".into()],
                    4..=5 => vec!["inference-delay".into()],
                    _ => vec![],
                },
                mode,
                oracle_outcome: if active {
                    OracleOutcome::DegradedWithinEnvelope
                } else {
                    OracleOutcome::WithinEnvelope
                },
            });
        }

        RecoveryEpisode {
            schema_version: 1,
            run_id: "run-hdc-ltc-seq-001".into(),
            model_arm_id: "E".into(),
            benchmark_id: "recoverability-envelope-v0.1".into(),
            scenario_family: "merge-with-compound-fault".into(),
            policy_revision: "policy-rev-001".into(),
            pairing: PairingIdentity {
                pair_id: "paired-merge-seed-001".into(),
                baseline_run_id: "baseline-run-001".into(),
                world_seed_sha256: "a".repeat(64),
                baseline_checkpoint_sha256: "b".repeat(64),
                candidate_checkpoint_sha256: "c".repeat(64),
                simulator_revision: "snapshot-backend-v1".into(),
                evaluator_revision: "recovery-evaluator-v0.1".into(),
            },
            expected_ticks: 10,
            contract: RecoveryContract {
                max_detection_delay_ticks: 1,
                max_fallback_delay_ticks: 1,
                max_recovery_after_end_ticks: 3,
                min_stable_nominal_ticks: 2,
                max_camera_age_us: 5_000,
                max_vehicle_state_age_us: 5_000,
                max_deadline_miss_ticks_during_fault: 1,
                max_fault_concurrency: 2,
            },
            fault_events: events,
            trace,
        }
    }

    #[test]
    fn reports_phase_conditioned_fault_impact_without_weighted_score() {
        let report = valid_episode().evaluate().unwrap();
        let camera = report
            .results
            .iter()
            .find(|result| result.fault_id == "camera-drop")
            .unwrap();
        let compute = report
            .results
            .iter()
            .find(|result| result.fault_id == "inference-delay")
            .unwrap();
        assert_eq!(camera.onset_in_critical_phase, Some(true));
        assert_eq!(camera.critical_phase_overlap_ticks, Some(2));
        assert_eq!(camera.critical_phase_fallback_ticks, Some(1));
        assert_eq!(camera.critical_phase_deadline_miss_ticks, Some(0));
        assert_eq!(compute.critical_phase_overlap_ticks, Some(3));
        assert_eq!(compute.critical_phase_deadline_miss_ticks, Some(1));
        assert_eq!(compute.critical_phase_constraint_violation_ticks, Some(0));
    }

    #[test]
    fn distinguishes_fault_onset_in_phase_from_later_phase_overlap() {
        let mut episode = valid_episode();
        episode.trace[1].maneuver_phase = "approach".into();
        let report = episode.evaluate().unwrap();
        let camera = report
            .results
            .iter()
            .find(|result| result.fault_id == "camera-drop")
            .unwrap();
        assert_eq!(camera.onset_in_critical_phase, Some(false));
        assert_eq!(camera.critical_phase_overlap_ticks, Some(1));
    }

    #[test]
    fn rejects_critical_phase_that_does_not_overlap_fault_window() {
        let mut episode = valid_episode();
        episode.fault_events[0].critical_phase = Some("emergency_braking".into());
        let errors = episode.validate().unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.contains("does not overlap that phase"))
        );
    }

    #[test]
    fn phase_conditioned_violations_are_separated_from_off_phase_violations() {
        let mut episode = valid_episode();
        episode.trace[1].maneuver_phase = "approach".into();
        episode.trace[1].oracle_outcome = OracleOutcome::ConstraintViolation;
        episode.trace[4].oracle_outcome = OracleOutcome::ConstraintViolation;
        let report = episode.evaluate().unwrap();
        let camera = report
            .results
            .iter()
            .find(|result| result.fault_id == "camera-drop")
            .unwrap();
        assert_eq!(camera.constraint_violation_ticks, 1);
        assert_eq!(camera.critical_phase_overlap_ticks, Some(1));
        assert_eq!(camera.critical_phase_constraint_violation_ticks, Some(0));
        let compute = report
            .results
            .iter()
            .find(|result| result.fault_id == "inference-delay")
            .unwrap();
        assert_eq!(compute.critical_phase_constraint_violation_ticks, Some(1));
    }

    #[test]
    fn fallback_is_attributed_only_to_the_faults_that_caused_it() {
        let mut episode = valid_episode();
        episode.trace[2].fallback_reason_fault_ids = vec!["inference-delay".into()];
        episode.trace[3].fallback_reason_fault_ids = vec!["inference-delay".into()];
        episode.trace[4].fallback_reason_fault_ids = vec!["inference-delay".into()];
        let report = episode.evaluate().unwrap();
        let camera = report
            .results
            .iter()
            .find(|item| item.fault_id == "camera-drop")
            .unwrap();
        let compute = report
            .results
            .iter()
            .find(|item| item.fault_id == "inference-delay")
            .unwrap();
        assert!(!camera.fallback_observed);
        assert!(compute.fallback_observed);
    }

    #[test]
    fn checked_in_compound_fault_fixture_parses_and_evaluates() {
        let input = include_str!("../fixtures/compound-merge-recovery-v1.json");
        let episode = RecoveryEpisode::from_json(input).unwrap();
        let report = episode.evaluate().unwrap();
        assert_eq!(report.expected_ticks, 10);
        assert_eq!(report.fault_event_count, 2);
        assert_eq!(report.maximum_concurrent_faults, 2);
        assert_eq!(report.contract_met_fault_count, 2);
    }

    #[test]
    fn checked_in_episode_schema_is_valid_json_and_versioned() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/episode-v1.schema.json")).unwrap();
        let report_schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/report-v1.schema.json")).unwrap();
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(schema["title"], "Recoverability envelope episode v1");
        assert_eq!(report_schema["title"], "Recoverability envelope report v1");
        assert!(
            schema["$defs"]["tick_trace"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == "maneuver_phase")
        );
        for field in [
            "onset_in_critical_phase",
            "critical_phase_overlap_ticks",
            "critical_phase_fallback_ticks",
            "critical_phase_deadline_miss_ticks",
            "critical_phase_constraint_violation_ticks",
        ] {
            assert!(
                report_schema["$defs"]["fault_result"]["required"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item == field)
            );
        }
    }

    #[test]
    fn deadline_miss_budget_is_explicit_per_fault() {
        let mut episode = valid_episode();
        episode.contract.max_deadline_miss_ticks_during_fault = 0;
        let report = episode.evaluate().unwrap();
        let delayed = report
            .results
            .iter()
            .find(|item| item.fault_id == "inference-delay")
            .unwrap();
        assert_eq!(delayed.deadline_miss_ticks_during_fault, 1);
        assert!(!delayed.contract_met);
        assert!(
            delayed
                .unmet_conditions
                .contains(&"deadline_miss_budget_exceeded".to_string())
        );
    }

    #[test]
    fn validates_exact_tick_denominator_and_paired_identity() {
        let episode = valid_episode();
        assert!(episode.validate().is_ok());
        assert_eq!(episode.pairing.pair_id, "paired-merge-seed-001");
    }

    #[test]
    fn evaluates_detection_fallback_and_stable_recovery_for_compound_faults() {
        let report = valid_episode().evaluate().unwrap();
        assert_eq!(report.expected_ticks, 10);
        assert_eq!(report.observed_ticks, 10);
        assert_eq!(report.fault_event_count, 2);
        assert_eq!(report.correlation_group_count, 1);
        assert_eq!(report.maximum_concurrent_faults, 2);
        assert_eq!(report.detected_fault_count, 2);
        assert_eq!(report.fallback_fault_count, 2);
        assert_eq!(report.recovered_fault_count, 2);
        assert_eq!(report.contract_met_fault_count, 2);
        assert_eq!(report.results[0].detection_delay_ticks, Some(0));
        assert_eq!(report.results[0].fallback_delay_ticks, Some(1));
        assert_eq!(report.results[0].recovery_tick, Some(7));
        assert_eq!(report.results[0].recovery_latency_ticks, Some(3));
    }

    #[test]
    fn rejects_missing_tick_denominator() {
        let mut episode = valid_episode();
        episode.trace.pop();
        assert!(
            episode
                .validate()
                .unwrap_err()
                .iter()
                .any(|e| e.contains("denominator mismatch"))
        );
    }

    #[test]
    fn rejects_duplicate_fault_ids() {
        let mut episode = valid_episode();
        episode.fault_events[1].fault_id = episode.fault_events[0].fault_id.clone();
        assert!(
            episode
                .validate()
                .unwrap_err()
                .iter()
                .any(|e| e.contains("duplicate identity"))
        );
    }

    #[test]
    fn rejects_unknown_detected_fault_ids() {
        let mut episode = valid_episode();
        episode.trace[1]
            .detected_fault_ids
            .push("not-scheduled".into());
        assert!(
            episode
                .validate()
                .unwrap_err()
                .iter()
                .any(|e| e.contains("unknown fault ID"))
        );
    }

    #[test]
    fn rejects_faults_outside_episode_or_with_zero_duration() {
        let mut episode = valid_episode();
        episode.fault_events[0].end_tick_exclusive = 99;
        assert!(
            episode
                .validate()
                .unwrap_err()
                .iter()
                .any(|e| e.contains("invalid interval"))
        );
        let mut zero = valid_episode();
        zero.fault_events[0].end_tick_exclusive = zero.fault_events[0].onset_tick;
        assert!(zero.validate().is_err());
    }

    #[test]
    fn detects_recovery_interrupted_by_a_second_fault() {
        let mut episode = valid_episode();
        episode.fault_events[1].end_tick_exclusive = 10;
        episode.contract.max_fault_concurrency = 2;
        for tick in episode
            .trace
            .iter_mut()
            .filter(|tick| tick.tick >= 6 && tick.tick < 10)
        {
            tick.mode = OperatingMode::Degraded;
            tick.oracle_outcome = OracleOutcome::DegradedWithinEnvelope;
        }
        let report = episode.evaluate().unwrap();
        assert!(
            report
                .results
                .iter()
                .all(|result| result.recovery_tick.is_none())
        );
    }

    #[test]
    fn records_constraint_violations_without_hiding_them_in_one_score() {
        let mut episode = valid_episode();
        episode.trace[2].oracle_outcome = OracleOutcome::ConstraintViolation;
        let report = episode.evaluate().unwrap();
        assert!(report.total_constraint_violation_ticks >= 1);
        assert_eq!(report.results[0].constraint_violation_ticks, 1);
        assert!(!report.results[0].contract_met);
        assert!(
            report.results[0]
                .unmet_conditions
                .contains(&"constraint_violation_during_fault".to_string())
        );
    }

    #[test]
    fn does_not_treat_unobserved_sensor_age_as_fresh() {
        let mut episode = valid_episode();
        for tick in episode.trace.iter_mut().filter(|tick| tick.tick >= 6) {
            tick.camera_age_us = None;
        }
        let report = episode.evaluate().unwrap();
        assert!(
            report
                .results
                .iter()
                .all(|result| result.recovery_tick.is_none())
        );
    }

    #[test]
    fn reports_deadline_misses_separately_from_fault_recovery() {
        let report = valid_episode().evaluate().unwrap();
        assert_eq!(report.deadline_miss_ticks, 1);
        assert_eq!(report.results[1].deadline_miss_ticks_during_fault, 1);
        assert!(
            report
                .results
                .iter()
                .all(|resul