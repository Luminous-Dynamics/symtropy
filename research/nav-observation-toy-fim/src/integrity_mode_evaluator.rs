//! Evaluator-only metrics for integrity-mode transitions.
//!
//! Controller mode and evidence are produced without access to evaluator truth.
//! After the decision stream is sealed, this module joins it to truth/fault
//! labels and measures false-NORMAL exposure, unnecessary degradation, fault
//! detection latency and recovery latency.
//!
//! These are benchmark metrics, not a runtime controller and not a statistical
//! or physical safety certificate. Time intervals are explicit and gaps remain
//! unobserved rather than being counted as safe.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::integrity_mode_controller::IntegrityMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluatorTruthStatus {
    /// Ground truth says the estimate would be supported for the evaluated
    /// decision and no disqualifying fault is active.
    QualifiedSafe,
    /// Ground truth says a disqualifying fault or unsupported state is active.
    UnsafeOrFaulted,
    /// Truth is unavailable, ambiguous or not independently qualified.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvaluatorFaultEvent {
    Start { fault_id: u64 },
    Clear { fault_id: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegrityModeEvaluationEvent {
    /// Strictly increasing evaluator event sequence.
    pub sequence_id: u64,
    /// Start time of this mode/truth segment in the declared monotonic clock.
    pub timestamp_ms: u64,
    /// Duration for which this mode and truth label are valid.
    pub duration_ms: u64,
    pub mode: IntegrityMode,
    pub truth_status: EvaluatorTruthStatus,
    /// At most one fault lifecycle boundary is attached to an event.
    pub fault_event: Option<EvaluatorFaultEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntegrityModeEvaluationReport {
    pub event_count: u64,
    pub observed_span_ms: u64,
    pub unobserved_gap_ms: u64,
    pub unknown_mode_ms: u64,
    pub unavailable_mode_ms: u64,
    pub degraded_mode_ms: u64,
    pub normal_mode_ms: u64,
    /// NORMAL exposure while evaluator truth says unsafe/faulted.
    pub false_normal_exposure_ms: u64,
    /// NORMAL exposure while evaluator truth is unknown.
    pub unknown_normal_exposure_ms: u64,
    /// Non-NORMAL exposure while evaluator truth says qualified/safe.
    pub unnecessary_non_normal_exposure_ms: u64,
    pub mode_transition_count: u64,
    pub fault_episodes_started: u64,
    pub fault_episodes_cleared: u64,
    pub fault_episodes_detected: u64,
    pub fault_episodes_cleared_without_detection: u64,
    pub active_faults_undetected_at_end: u64,
    pub active_faults_detected_at_end: u64,
    pub recoveries_completed: u64,
    pub recoveries_pending_at_end: u64,
    pub mean_fault_detection_latency_ms: Option<f64>,
    pub p95_fault_detection_latency_ms: Option<u64>,
    pub maximum_fault_detection_latency_ms: Option<u64>,
    pub mean_recovery_latency_ms: Option<f64>,
    pub p95_recovery_latency_ms: Option<u64>,
    pub maximum_recovery_latency_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrityModeEvaluationError {
    ZeroSequenceOrDuration,
    SequenceReplayOrReordering,
    TimestampReplayOrReordering,
    OverlappingIntervals,
    IntervalOverflow,
    ZeroFaultIdentity,
    DuplicateFaultStart(u64),
    ClearUnknownFault(u64),
    DuplicateFaultClear(u64),
    CounterOverflow,
}

impl fmt::Display for IntegrityModeEvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSequenceOrDuration => write!(f, "event sequence and duration must be nonzero"),
            Self::SequenceReplayOrReordering => write!(f, "evaluator event sequence is not strictly increasing"),
            Self::TimestampReplayOrReordering => write!(f, "evaluator event timestamp is not strictly increasing"),
            Self::OverlappingIntervals => write!(f, "evaluator time intervals overlap"),
            Self::IntervalOverflow => write!(f, "evaluator interval end time overflowed"),
            Self::ZeroFaultIdentity => write!(f, "fault identity must be nonzero"),
            Self::DuplicateFaultStart(id) => write!(f, "fault identity was started more than once: {id}"),
            Self::ClearUnknownFault(id) => write!(f, "cannot clear a fault that was not started: {id}"),
            Self::DuplicateFaultClear(id) => write!(f, "fault identity was cleared more than once: {id}"),
            Self::CounterOverflow => write!(f, "integrity-mode evaluator counter overflowed"),
        }
    }
}

impl std::error::Error for IntegrityModeEvaluationError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FaultTrack {
    onset_ms: u64,
    detected: bool,
    cleared: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegrityModeEvaluator {
    event_count: u64,
    first_timestamp_ms: Option<u64>,
    last_sequence_id: Option<u64>,
    last_timestamp_ms: Option<u64>,
    last_interval_end_ms: Option<u64>,
    previous_mode: Option<IntegrityMode>,
    unobserved_gap_ms: u64,
    unknown_mode_ms: u64,
    unavailable_mode_ms: u64,
    degraded_mode_ms: u64,
    normal_mode_ms: u64,
    false_normal_exposure_ms: u64,
    unknown_normal_exposure_ms: u64,
    unnecessary_non_normal_exposure_ms: u64,
    mode_transition_count: u64,
    fault_tracks: BTreeMap<u64, FaultTrack>,
    seen_fault_ids: BTreeSet<u64>,
    recovery_start_ms: BTreeMap<u64, u64>,
    fault_detection_latencies_ms: Vec<u64>,
    recovery_latencies_ms: Vec<u64>,
    fault_episodes_started: u64,
    fault_episodes_cleared: u64,
    fault_episodes_detected: u64,
    fault_episodes_cleared_without_detection: u64,
}

impl Default for IntegrityModeEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrityModeEvaluator {
    pub fn new() -> Self {
        Self {
            event_count: 0,
            first_timestamp_ms: None,
            last_sequence_id: None,
            last_timestamp_ms: None,
            last_interval_end_ms: None,
            previous_mode: None,
            unobserved_gap_ms: 0,
            unknown_mode_ms: 0,
            unavailable_mode_ms: 0,
            degraded_mode_ms: 0,
            normal_mode_ms: 0,
            false_normal_exposure_ms: 0,
            unknown_normal_exposure_ms: 0,
            unnecessary_non_normal_exposure_ms: 0,
            mode_transition_count: 0,
            fault_tracks: BTreeMap::new(),
            seen_fault_ids: BTreeSet::new(),
            recovery_start_ms: BTreeMap::new(),
            fault_detection_latencies_ms: Vec::new(),
            recovery_latencies_ms: Vec::new(),
            fault_episodes_started: 0,
            fault_episodes_cleared: 0,
            fault_episodes_detected: 0,
            fault_episodes_cleared_without_detection: 0,
        }
    }

    /// Add one sealed controller-mode segment and its evaluator-only truth.
    ///
    /// If there is a gap between segments, that gap is counted as unobserved.
    /// Gaps are never inferred to be NORMAL, safe or fault-free.
    pub fn observe(
        &mut self,
        event: IntegrityModeEvaluationEvent,
    ) -> Result<(), IntegrityModeEvaluationError> {
        if event.sequence_id == 0 || event.duration_ms == 0 {
            return Err(IntegrityModeEvaluationError::ZeroSequenceOrDuration);
        }
        if self
            .last_sequence_id
            .is_some_and(|last| event.sequence_id <= last)
        {
            return Err(IntegrityModeEvaluationError::SequenceReplayOrReordering);
        }
        if self
            .last_timestamp_ms
            .is_some_and(|last| event.timestamp_ms <= last)
        {
            return Err(IntegrityModeEvaluationError::TimestampReplayOrReordering);
        }
        if self
            .last_interval_end_ms
            .is_some_and(|end| event.timestamp_ms < end)
        {
            return Err(IntegrityModeEvaluationError::OverlappingIntervals);
        }
        let interval_end = event
            .timestamp_ms
            .checked_add(event.duration_ms)
            .ok_or(IntegrityModeEvaluationError::IntervalOverflow)?;

        // Prevalidate lifecycle identity before committing any metrics.
        match event.fault_event {
            Some(EvaluatorFaultEvent::Start { fault_id }) => {
                if fault_id == 0 {
                    return Err(IntegrityModeEvaluationError::ZeroFaultIdentity);
                }
                if self.seen_fault_ids.contains(&fault_id) {
                    return Err(IntegrityModeEvaluationError::DuplicateFaultStart(fault_id));
                }
            }
            Some(EvaluatorFaultEvent::Clear { fault_id }) => {
                if fault_id == 0 {
                    return Err(IntegrityModeEvaluationError::ZeroFaultIdentity);
                }
                let track = self
                    .fault_tracks
                    .get(&fault_id)
                    .ok_or(IntegrityModeEvaluationError::ClearUnknownFault(fault_id))?;
                if track.cleared {
                    return Err(IntegrityModeEvaluationError::DuplicateFaultClear(fault_id));
                }
            }
            None => {}
        }

        let gap_increment = self
            .last_interval_end_ms
            .map_or(0, |end| event.timestamp_ms.saturating_sub(end));
        let mut next_unobserved_gap_ms = self.unobserved_gap_ms;
        add(&mut next_unobserved_gap_ms, gap_increment)?;

        let mut next_event_count = self.event_count;
        add(&mut next_event_count, 1)?;

        let mut next_unknown_mode_ms = self.unknown_mode_ms;
        let mut next_unavailable_mode_ms = self.unavailable_mode_ms;
        let mut next_degraded_mode_ms = self.degraded_mode_ms;
        let mut next_normal_mode_ms = self.normal_mode_ms;
        let mut next_false_normal_ms = self.false_normal_exposure_ms;
        let mut next_unknown_normal_ms = self.unknown_normal_exposure_ms;
        let mut next_unnecessary_non_normal_ms = self.unnecessary_non_normal_exposure_ms;
        let mut next_transitions = self.mode_transition_count;

        match event.mode {
            IntegrityMode::Unknown => add(&mut next_unknown_mode_ms, event.duration_ms)?,
            IntegrityMode::Unavailable => {
                add(&mut next_unavailable_mode_ms, event.duration_ms)?
            }
            IntegrityMode::Degraded => add(&mut next_degraded_mode_ms, event.duration_ms)?,
            IntegrityMode::Normal => add(&mut next_normal_mode_ms, event.duration_ms)?,
        }

        match (event.mode, event.truth_status) {
            (IntegrityMode::Normal, EvaluatorTruthStatus::UnsafeOrFaulted) => {
                add(&mut next_false_normal_ms, event.duration_ms)?
            }
            (IntegrityMode::Normal, EvaluatorTruthStatus::Unknown) => {
                add(&mut next_unknown_normal_ms, event.duration_ms)?
            }
            (mode, EvaluatorTruthStatus::QualifiedSafe) if mode != IntegrityMode::Normal => {
                add(&mut next_unnecessary_non_normal_ms, event.duration_ms)?
            }
            _ => {}
        }

        if self
            .previous_mode
            .is_some_and(|previous| previous != event.mode)
        {
            add(&mut next_transitions, 1)?;
        }

        let mut next_fault_tracks = self.fault_tracks.clone();
        let mut next_seen_fault_ids = self.seen_fault_ids.clone();
        let mut next_recovery_start = self.recovery_start_ms.clone();
        let mut next_detection_latencies = self.fault_detection_latencies_ms.clone();
        let mut next_recovery_latencies = self.recovery_latencies_ms.clone();
        let mut next_fault_started = self.fault_episodes_started;
        let mut next_fault_cleared = self.fault_episodes_cleared;
        let mut next_fault_detected = self.fault_episodes_detected;
        let mut next_fault_undetected = self.fault_episodes_cleared_without_detection;

        if let Some(fault_event) = event.fault_event {
            match fault_event {
                EvaluatorFaultEvent::Start { fault_id } => {
                    next_seen_fault_ids.insert(fault_id);
                    next_fault_tracks.insert(
                        fault_id,
                        FaultTrack {
                            onset_ms: event.timestamp_ms,
                            detected: false,
                            cleared: false,
                        },
                    );
                    add(&mut next_fault_started, 1)?;
                }
                EvaluatorFaultEvent::Clear { fault_id } => {
                    let mut track = next_fault_tracks
                        .get(&fault_id)
                        .copied()
                        .ok_or(IntegrityModeEvaluationError::ClearUnknownFault(fault_id))?;
                    track.cleared = true;
                    if !track.detected {
                        add(&mut next_fault_undetected, 1)?;
                    } else if event.mode == IntegrityMode::Normal {
                        next_recovery_latencies.push(0);
                    } else {
                        next_recovery_start.insert(fault_id, event.timestamp_ms);
                    }
                    next_fault_tracks.insert(fault_id, track);
                    add(&mut next_fault_cleared, 1)?;
                }
            }
        }

        // The first non-NORMAL mode after an active fault onset is the observed
        // downgrade time. If the controller was already non-NORMAL at the
        // fault-onset event, the measured latency is zero.
        if event.mode != IntegrityMode::Normal {
            for (fault_id, track) in next_fault_tracks.iter_mut() {
                if !track.cleared && !track.detected {
                    track.detected = true;
                    next_detection_latencies.push(event.timestamp_ms.saturating_sub(track.onset_ms));
                    add(&mut next_fault_detected, 1)?;
                    let _ = fault_id;
                }
            }
        }

        // Recovery is complete at the first NORMAL segment after the evaluator
        // has declared the fault cleared.
        if event.mode == IntegrityMode::Normal {
            let recovered = next_recovery_start.keys().copied().collect::<Vec<_>>();
            for fault_id in recovered {
                if let Some(start_ms) = next_recovery_start.remove(&fault_id) {
                    next_recovery_latencies.push(event.timestamp_ms.saturating_sub(start_ms));
                }
            }
        }

        self.event_count = next_event_count;
        self.first_timestamp_ms.get_or_insert(event.timestamp_ms);
        self.last_sequence_id = Some(event.sequence_id);
        self.last_timestamp_ms = Some(event.timestamp_ms);
        self.last_interval_end_ms = Some(interval_end);
        self.previous_mode = Some(event.mode);
        self.unobserved_gap_ms = next_unobserved_gap_ms;
        self.unknown_mode_ms = next_unknown_mode_ms;
        self.unavailable_mode_ms = next_unavailable_mode_ms;
        self.degraded_mode_ms = next_degraded_mode_ms;
        self.normal_mode_ms = next_normal_mode_ms;
        self.false_normal_exposure_ms = next_false_normal_ms;
        self.unknown_normal_exposure_ms = next_unknown_normal_ms;
        self.unnecessary_non_normal_exposure_ms = next_unnecessary_non_normal_ms;
        self.mode_transition_count = next_transitions;
        self.fault_tracks = next_fault_tracks;
        self.seen_fault_ids = next_seen_fault_ids;
        self.recovery_start_ms = next_recovery_start;
        self.fault_detection_latencies_ms = next_detection_latencies;
        self.recovery_latencies_ms = next_recovery_latencies;
        self.fault_episodes_started = next_fault_started;
        self.fault_episodes_cleared = next_fault_cleared;
        self.fault_episodes_detected = next_fault_detected;
        self.fault_episodes_cleared_without_detection = next_fault_undetected;
        Ok(())
    }

    pub fn report(&self) -> IntegrityModeEvaluationReport {
        let active_faults_undetected_at_end = self
            .fault_tracks
            .values()
            .filter(|track| !track.cleared && !track.detected)
            .count() as u64;
        let active_faults_detected_at_end = self
            .fault_tracks
            .values()
            .filter(|track| !track.cleared && track.detected)
            .count() as u64;
        let fault_detection_latencies_ms = sorted(&self.fault_detection_latencies_ms);
        let recovery_latencies_ms = sorted(&self.recovery_latencies_ms);

        IntegrityModeEvaluationReport {
            event_count: self.event_count,
            observed_span_ms: match (self.first_timestamp_ms, self.last_interval_end_ms) {
                (Some(start), Some(end)) => end.saturating_sub(start),
                _ => 0,
            },
            unobserved_gap_ms: self.unobserved_gap_ms,
            unknown_mode_ms: self.unknown_mode_ms,
            unavailable_mode_ms: self.unavailable_mode_ms,
            degraded_mode_ms: self.degraded_mode_ms,
            normal_mode_ms: self.normal_mode_ms,
            false_normal_exposure_ms: self.false_normal_exposure_ms,
            unknown_normal_exposure_ms: self.unknown_normal_exposure_ms,
            unnecessary_non_normal_exposure_ms: self.unnecessary_non_normal_exposure_ms,
            mode_transition_count: self.mode_transition_count,
            fault_episodes_started: self.fault_episodes_started,
            fault_episodes_cleared: self.fault_episodes_cleared,
            fault_episodes_detected: self.fault_episodes_detected,
            fault_episodes_cleared_without_detection: self
                .fault_episodes_cleared_without_detection,
            active_faults_undetected_at_end,
            active_faults_detected_at_end,
            recoveries_completed: self.recovery_latencies_ms.len() as u64,
            recoveries_pending_at_end: self.recovery_start_ms.len() as u64,
            mean_fault_detection_latency_ms: mean(&fault_detection_latencies_ms),
            p95_fault_detection_latency_ms: percentile_95(&fault_detection_latencies_ms),
            maximum_fault_detection_latency_ms: fault_detection_latencies_ms.last().copied(),
            mean_recovery_latency_ms: mean(&recovery_latencies_ms),
            p95_recovery_latency_ms: percentile_95(&recovery_latencies_ms),
            maximum_recovery_latency_ms: recovery_latencies_ms.last().copied(),
        }
    }
}

fn add(target: &mut u64, amount: u64) -> Result<(), IntegrityModeEvaluationError> {
    *target = target
        .checked_add(amount)
        .ok_or(IntegrityModeEvaluationError::CounterOverflow)?;
    Ok(())
}

fn sorted(values: &[u64]) -> Vec<u64> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted
}

fn mean(sorted_values: &[u64]) -> Option<f64> {
    if sorted_values.is_empty() {
        return None;
    }
    let sum: u128 = sorted_values.iter().map(|value| *value as u128).sum();
    Some(sum as f64 / sorted_values.len() as f64)
}

fn percentile_95(sorted_values: &[u64]) -> Option<u64> {
    if sorted_values.is_empty() {
        return None;
    }
    let rank = (sorted_values.len() * 95).div_ceil(100).max(1);
    sorted_values.get(rank - 1).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(
        sequence_id: u64,
        timestamp_ms: u64,
        duration_ms: u64,
        mode: IntegrityMode,
        truth_status: EvaluatorTruthStatus,
        fault_event: Option<EvaluatorFaultEvent>,
    ) -> IntegrityModeEvaluationEvent {
        IntegrityModeEvaluationEvent {
            sequence_id,
            timestamp_ms,
            duration_ms,
            mode,
            truth_status,
            fault_event,
        }
    }

    #[test]
    fn measures_false_normal_unknown_normal_and_unnecessary_non_normal_exposure() {
        let mut evaluator = IntegrityModeEvaluator::new();
        evaluator
            .observe(event(
                1,
                0,
                100,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::UnsafeOrFaulted,
                None,
            ))
            .unwrap();
        evaluator
            .observe(event(
                2,
                100,
                40,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::Unknown,
                None,
            ))
            .unwrap();
        evaluator
            .observe(event(
                3,
                140,
                80,
                IntegrityMode::Degraded,
                EvaluatorTruthStatus::QualifiedSafe,
                None,
            ))
            .unwrap();

        let report = evaluator.report();
        assert_eq!(report.false_normal_exposure_ms, 100);
        assert_eq!(report.unknown_normal_exposure_ms, 40);
        assert_eq!(report.unnecessary_non_normal_exposure_ms, 80);
        assert_eq!(report.normal_mode_ms, 140);
        assert_eq!(report.degraded_mode_ms, 80);
        assert_eq!(report.mode_transition_count, 1);
    }

    #[test]
    fn measures_fault_detection_and_recovery_latency() {
        let mut evaluator = IntegrityModeEvaluator::new();
        evaluator
            .observe(event(
                1,
                0,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::QualifiedSafe,
                None,
            ))
            .unwrap();
        evaluator
            .observe(event(
                2,
                10,
                5,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::UnsafeOrFaulted,
                Some(EvaluatorFaultEvent::Start { fault_id: 7 }),
            ))
            .unwrap();
        evaluator
            .observe(event(
                3,
                15,
                5,
                IntegrityMode::Degraded,
                EvaluatorTruthStatus::UnsafeOrFaulted,
                None,
            ))
            .unwrap();
        evaluator
            .observe(event(
                4,
                20,
                5,
                IntegrityMode::Degraded,
                EvaluatorTruthStatus::QualifiedSafe,
                Some(EvaluatorFaultEvent::Clear { fault_id: 7 }),
            ))
            .unwrap();
        evaluator
            .observe(event(
                5,
                25,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::QualifiedSafe,
                None,
            ))
            .unwrap();

        let report = evaluator.report();
        assert_eq!(report.fault_episodes_started, 1);
        assert_eq!(report.fault_episodes_detected, 1);
        assert_eq!(report.fault_episodes_cleared, 1);
        assert_eq!(report.recoveries_completed, 1);
        assert_eq!(report.mean_fault_detection_latency_ms, Some(5.0));
        assert_eq!(report.p95_fault_detection_latency_ms, Some(5));
        assert_eq!(report.mean_recovery_latency_ms, Some(5.0));
        assert_eq!(report.p95_recovery_latency_ms, Some(5));
    }

    #[test]
    fn exposes_fault_episodes_that_clear_without_a_downgrade() {
        let mut evaluator = IntegrityModeEvaluator::new();
        evaluator
            .observe(event(
                1,
                0,
                100,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::UnsafeOrFaulted,
                Some(EvaluatorFaultEvent::Start { fault_id: 4 }),
            ))
            .unwrap();
        evaluator
            .observe(event(
                2,
                100,
                20,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::QualifiedSafe,
                Some(EvaluatorFaultEvent::Clear { fault_id: 4 }),
            ))
            .unwrap();

        let report = evaluator.report();
        assert_eq!(report.fault_episodes_cleared_without_detection, 1);
        assert_eq!(report.fault_episodes_detected, 0);
        assert_eq!(report.false_normal_exposure_ms, 100);
        assert_eq!(report.recoveries_completed, 0);
    }

    #[test]
    fn unobserved_gaps_are_not_counted_as_safe_or_normal() {
        let mut evaluator = IntegrityModeEvaluator::new();
        evaluator
            .observe(event(
                1,
                100,
                50,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::QualifiedSafe,
                None,
            ))
            .unwrap();
        evaluator
            .observe(event(
                2,
                200,
                50,
                IntegrityMode::Unknown,
                EvaluatorTruthStatus::Unknown,
                None,
            ))
            .unwrap();

        let report = evaluator.report();
        assert_eq!(report.unobserved_gap_ms, 50);
        assert_eq!(report.normal_mode_ms, 50);
        assert_eq!(report.unknown_mode_ms, 50);
        assert_eq!(report.observed_span_ms, 150);
    }

    #[test]
    fn rejects_replayed_overlapping_and_invalid_fault_events() {
        let mut evaluator = IntegrityModeEvaluator::new();
        evaluator
            .observe(event(
                1,
                0,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::QualifiedSafe,
                None,
            ))
            .unwrap();
        assert_eq!(
            evaluator.observe(event(
                1,
                10,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::QualifiedSafe,
                None
            )),
            Err(IntegrityModeEvaluationError::SequenceReplayOrReordering)
        );
        assert_eq!(
            evaluator.observe(event(
                2,
                5,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::QualifiedSafe,
                None
            )),
            Err(IntegrityModeEvaluationError::OverlappingIntervals)
        );
        assert_eq!(
            evaluator.observe(event(
                2,
                10,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::UnsafeOrFaulted,
                Some(EvaluatorFaultEvent::Start { fault_id: 0 })
            )),
            Err(IntegrityModeEvaluationError::ZeroFaultIdentity)
        );
        assert_eq!(evaluator.report().event_count, 1);
    }

    #[test]
    fn duplicate_fault_starts_and_unknown_clears_are_rejected() {
        let mut evaluator = IntegrityModeEvaluator::new();
        evaluator
            .observe(event(
                1,
                0,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::UnsafeOrFaulted,
                Some(EvaluatorFaultEvent::Start { fault_id: 1 }),
            ))
            .unwrap();
        assert_eq!(
            evaluator.observe(event(
                2,
                10,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::UnsafeOrFaulted,
                Some(EvaluatorFaultEvent::Start { fault_id: 1 }),
            )),
            Err(IntegrityModeEvaluationError::DuplicateFaultStart(1))
        );
        assert_eq!(
            evaluator.observe(event(
                2,
                10,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::QualifiedSafe,
                Some(EvaluatorFaultEvent::Clear { fault_id: 2 }),
            )),
            Err(IntegrityModeEvaluationError::ClearUnknownFault(2))
        );
        assert_eq!(evaluator.report().event_count, 1);
    }

    #[test]
    fn leaves_uncleared_and_undetected_faults_explicit_at_end() {
        let mut evaluator = IntegrityModeEvaluator::new();
        evaluator
            .observe(event(
                1,
                0,
                10,
                IntegrityMode::Normal,
                EvaluatorTruthStatus::UnsafeOrFaulted,
                Some(EvaluatorFaultEvent::Start { fault_id: 1 }),
            ))
            .unwrap();
        evaluator
            .observe(event(
                2,
                10,
                10,
                IntegrityMode::Degraded,
                EvaluatorTruthStatus::UnsafeOrFaulted,
                Some(EvaluatorFaultEvent::Start { fault_id: 2 }),
            ))
            .unwrap();

        let report = evaluator.report();
        assert_eq!(report.active_faults_undetected_at_end, 1);
        assert_eq!(report.active_faults_detected_at_end, 1);
        assert_eq!(report.recoveries_pending_at_end, 0);
    }
}
