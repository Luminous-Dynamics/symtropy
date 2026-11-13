#![forbid(unsafe_code)]

//! Event-level diagnosis metrics that complement sample-level competition scores.
//!
//! A row-level detection score can reward a detector that alarms too late or
//! emits many repeated alarms. This module converts the frozen sample stream
//! into debounced alarm episodes, matches them one-to-one to fault episodes,
//! and reports latency and false alarms per observed normal hour. All timing
//! decisions are explicit in EventMetricConfig; missing observations are not
//! forward-filled and do not contribute to monitored exposure.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const EVENT_ISOLATION_SLOTS: usize = 5;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RecordingWindow {
    pub recording_id: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FaultEvent {
    pub recording_id: String,
    pub event_id: String,
    pub start_seconds: f64,
    pub end_seconds: f64,
    /// Index into the five benchmark isolation slots, including slot 4 for unknown.
    pub target_isolation_index: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DetectionObservation {
    pub recording_id: String,
    pub sample_index: u64,
    pub time_seconds: f64,
    pub detection: bool,
    /// Official isolation vector; all-zero when detection is false.
    pub isolation_probabilities: [f64; EVENT_ISOLATION_SLOTS],
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EventMetricConfig {
    /// Number of positive samples needed before an alarm episode is confirmed.
    pub minimum_positive_samples: usize,
    /// Merge positive samples into one alarm when their timestamp gap is at most this value.
    pub alarm_merge_gap_seconds: f64,
    /// A fault is detected only when an alarm confirms within this time after onset.
    pub maximum_detection_latency_seconds: f64,
    /// Observation intervals larger than this are treated as unmonitored gaps.
    pub maximum_observation_gap_seconds: f64,
    pub probability_sum_tolerance: f64,
}

impl EventMetricConfig {
    /// Conservative starting protocol for the public 20 Hz LiU-ICE stream.
    /// These values are benchmark protocol choices, not learned or optimized values.
    pub fn liu_ice_v0_1() -> Self {
        Self {
            minimum_positive_samples: 3,
            alarm_merge_gap_seconds: 0.15,
            maximum_detection_latency_seconds: 10.0,
            maximum_observation_gap_seconds: 0.10,
            probability_sum_tolerance: 1e-6,
        }
    }

    fn validate(&self) -> Result<(), EventMetricError> {
        if self.minimum_positive_samples == 0
            || !self.alarm_merge_gap_seconds.is_finite()
            || self.alarm_merge_gap_seconds < 0.0
            || !self.maximum_detection_latency_seconds.is_finite()
            || self.maximum_detection_latency_seconds <= 0.0
            || !self.maximum_observation_gap_seconds.is_finite()
            || self.maximum_observation_gap_seconds <= 0.0
            || !self.probability_sum_tolerance.is_finite()
            || self.probability_sum_tolerance <= 0.0
            || self.probability_sum_tolerance >= 0.1
        {
            return Err(EventMetricError::InvalidConfig);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct AlarmEpisode {
    pub recording_id: String,
    pub start_seconds: f64,
    /// The time at which the configured positive-sample requirement was met.
    pub confirmation_seconds: f64,
    pub end_seconds: f64,
    pub positive_samples: usize,
    pub isolation_probabilities_at_confirmation: [f64; EVENT_ISOLATION_SLOTS],
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct EventOutcome {
    pub recording_id: String,
    pub event_id: String,
    pub onset_seconds: f64,
    pub detection_confirmation_seconds: Option<f64>,
    pub detection_latency_seconds: Option<f64>,
    pub target_isolation_index: usize,
    pub predicted_isolation_index: Option<usize>,
    pub true_class_probability: Option<f64>,
    pub isolation_top1_correct: Option<bool>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct UnmatchedAlarm {
    pub recording_id: String,
    pub start_seconds: f64,
    pub confirmation_seconds: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct EventEvaluationSummary {
    pub recording_count: usize,
    pub fault_event_count: usize,
    pub detected_event_count: usize,
    pub missed_event_count: usize,
    pub event_recall: Option<f64>,
    pub mean_detection_latency_seconds: Option<f64>,
    pub median_detection_latency_seconds: Option<f64>,
    /// Nearest-rank 95th percentile across detected fault events.
    pub p95_detection_latency_seconds: Option<f64>,
    pub maximum_detection_latency_seconds: Option<f64>,
    pub false_alarm_count: usize,
    pub false_alarms_per_observed_normal_hour: Option<f64>,
    pub duplicate_alarm_count_during_detected_events: usize,
    pub late_alarm_count_during_missed_events: usize,
    /// Positive bursts shorter than the confirmation requirement; not counted as alarms.
    pub transient_burst_count: usize,
    pub observed_normal_exposure_seconds: f64,
    pub mean_true_class_probability: Option<f64>,
    pub isolation_top1_accuracy: Option<f64>,
    pub event_outcomes: Vec<EventOutcome>,
    pub false_alarm_episodes: Vec<UnmatchedAlarm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventMetricError {
    EmptyInput,
    InvalidConfig,
    EmptyIdentity,
    DuplicateRecording(String),
    UnknownRecording(String),
    InvalidRecordingWindow(String),
    DuplicateEvent(String),
    InvalidFaultEvent(String),
    OverlappingFaultEvents(String),
    NonMonotonicObservation(String),
    DuplicateObservation(String),
    ObservationOutsideWindow(String),
    InvalidPrediction(String),
}

impl fmt::Display for EventMetricError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(
                formatter,
                "event evaluation needs recording windows and observations"
            ),
            Self::InvalidConfig => write!(formatter, "event metric configuration is invalid"),
            Self::EmptyIdentity => write!(formatter, "recording or event identity is empty"),
            Self::DuplicateRecording(id) => write!(formatter, "duplicate recording window: {id}"),
            Self::UnknownRecording(id) => write!(
                formatter,
                "event or observation references unknown recording: {id}"
            ),
            Self::InvalidRecordingWindow(id) => write!(formatter, "invalid recording window: {id}"),
            Self::DuplicateEvent(id) => write!(formatter, "duplicate fault event identity: {id}"),
            Self::InvalidFaultEvent(id) => write!(formatter, "invalid fault event: {id}"),
            Self::OverlappingFaultEvents(id) => {
                write!(formatter, "fault events overlap in recording: {id}")
            }
            Self::NonMonotonicObservation(id) => write!(
                formatter,
                "sample index or timestamp is not strictly increasing: {id}"
            ),
            Self::DuplicateObservation(id) => write!(formatter, "duplicate sample identity: {id}"),
            Self::ObservationOutsideWindow(id) => {
                write!(formatter, "observation lies outside recording window: {id}")
            }
            Self::InvalidPrediction(id) => {
                write!(formatter, "invalid detection/isolation output: {id}")
            }
        }
    }
}

impl std::error::Error for EventMetricError {}

#[derive(Clone)]
struct PositiveRun {
    recording_id: String,
    samples: Vec<DetectionObservation>,
}

impl PositiveRun {
    fn to_alarm(&self, minimum_positive_samples: usize) -> Option<AlarmEpisode> {
        if self.samples.len() < minimum_positive_samples {
            return None;
        }
        let confirmation = &self.samples[minimum_positive_samples - 1];
        let last = self.samples.last().expect("positive run is non-empty");
        Some(AlarmEpisode {
            recording_id: self.recording_id.clone(),
            start_seconds: self.samples[0].time_seconds,
            confirmation_seconds: confirmation.time_seconds,
            end_seconds: last.time_seconds,
            positive_samples: self.samples.len(),
            isolation_probabilities_at_confirmation: confirmation.isolation_probabilities,
        })
    }
}

/// Evaluate fault-event recall, alarm latency, duplicate/late alarms and
/// false alarms per observed normal hour. Fault events must not overlap within
/// a recording so one-to-one matching is deterministic and auditable.
///
/// Observations are streaming outputs, not labels. Each alarm can match at
/// most one event; the matching alarm must begin at or after true onset and
/// confirm no later than both the configured deadline and the fault end time.
/// Alarms before onset remain false alarms even if they continue into a fault.
pub fn evaluate_event_metrics(
    recording_windows: &[RecordingWindow],
    fault_events: &[FaultEvent],
    observations: &[DetectionObservation],
    config: &EventMetricConfig,
) -> Result<EventEvaluationSummary, EventMetricError> {
    config.validate()?;
    if recording_windows.is_empty() || observations.is_empty() {
        return Err(EventMetricError::EmptyInput);
    }

    let mut windows = BTreeMap::<String, RecordingWindow>::new();
    for window in recording_windows {
        if window.recording_id.trim().is_empty() {
            return Err(EventMetricError::EmptyIdentity);
        }
        if !window.start_seconds.is_finite()
            || !window.end_seconds.is_finite()
            || window.start_seconds < 0.0
            || window.end_seconds <= window.start_seconds
        {
            return Err(EventMetricError::InvalidRecordingWindow(
                window.recording_id.clone(),
            ));
        }
        if windows
            .insert(window.recording_id.clone(), window.clone())
            .is_some()
        {
            return Err(EventMetricError::DuplicateRecording(
                window.recording_id.clone(),
            ));
        }
    }

    let mut events_by_recording = BTreeMap::<String, Vec<FaultEvent>>::new();
    let mut event_ids = BTreeSet::new();
    for event in fault_events {
        if event.recording_id.trim().is_empty() || event.event_id.trim().is_empty() {
            return Err(EventMetricError::EmptyIdentity);
        }
        let Some(window) = windows.get(&event.recording_id) else {
            return Err(EventMetricError::UnknownRecording(
                event.recording_id.clone(),
            ));
        };
        if !event.start_seconds.is_finite()
            || !event.end_seconds.is_finite()
            || event.start_seconds < window.start_seconds
            || event.end_seconds > window.end_seconds
            || event.end_seconds <= event.start_seconds
            || event.target_isolation_index >= EVENT_ISOLATION_SLOTS
        {
            return Err(EventMetricError::InvalidFaultEvent(event.event_id.clone()));
        }
        let identity = format!("{}::{}", event.recording_id, event.event_id);
        if !event_ids.insert(identity.clone()) {
            return Err(EventMetricError::DuplicateEvent(identity));
        }
        events_by_recording
            .entry(event.recording_id.clone())
            .or_default()
            .push(event.clone());
    }
    for (recording_id, events) in &mut events_by_recording {
        events.sort_by(|left, right| left.start_seconds.total_cmp(&right.start_seconds));
        if events
            .windows(2)
            .any(|pair| pair[1].start_seconds < pair[0].end_seconds)
        {
            return Err(EventMetricError::OverlappingFaultEvents(
                recording_id.clone(),
            ));
        }
    }

    let mut observations_by_recording = BTreeMap::<String, Vec<DetectionObservation>>::new();
    for observation in observations {
        if observation.recording_id.trim().is_empty() {
            return Err(EventMetricError::EmptyIdentity);
        }
        let Some(window) = windows.get(&observation.recording_id) else {
            return Err(EventMetricError::UnknownRecording(
                observation.recording_id.clone(),
            ));
        };
        if !observation.time_seconds.is_finite()
            || observation.time_seconds < window.start_seconds
            || observation.time_seconds > window.end_seconds
        {
            return Err(EventMetricError::ObservationOutsideWindow(
                observation.recording_id.clone(),
            ));
        }
        validate_prediction(observation, config.probability_sum_tolerance)?;
        observations_by_recording
            .entry(observation.recording_id.clone())
            .or_default()
            .push(observation.clone());
    }
    for (recording_id, rows) in &mut observations_by_recording {
        rows.sort_by_key(|row| row.sample_index);
        if rows.windows(2).any(|pair| {
            pair[1].sample_index <= pair[0].sample_index
                || pair[1].time_seconds <= pair[0].time_seconds
        }) {
            return Err(EventMetricError::NonMonotonicObservation(
                recording_id.clone(),
            ));
        }
        let mut timestamps = BTreeSet::new();
        if rows
            .iter()
            .any(|row| !timestamps.insert(row.time_seconds.to_bits()))
        {
            return Err(EventMetricError::DuplicateObservation(recording_id.clone()));
        }
    }

    let mut all_alarms = Vec::<AlarmEpisode>::new();
    let mut transient_burst_count = 0usize;
    for (recording_id, rows) in &observations_by_recording {
        let mut current: Option<PositiveRun> = None;
        for row in rows {
            if !row.detection {
                if let Some(completed) = current.take() {
                    if let Some(alarm) = completed.to_alarm(config.minimum_positive_samples) {
                        all_alarms.push(alarm);
                    } else {
                        transient_burst_count += 1;
                    }
                }
                continue;
            }
            let can_merge = current.as_ref().is_some_and(|run| {
                let observation_gap =
                    row.time_seconds - run.samples.last().expect("run has a sample").time_seconds;
                observation_gap <= config.alarm_merge_gap_seconds
                    && observation_gap <= config.maximum_observation_gap_seconds
            });
            if can_merge {
                current
                    .as_mut()
                    .expect("merge predicate requires an open run")
                    .samples
                    .push(row.clone());
                continue;
            }
            if let Some(completed) = current.take() {
                if let Some(alarm) = completed.to_alarm(config.minimum_positive_samples) {
                    all_alarms.push(alarm);
                } else {
                    transient_burst_count += 1;
                }
            }
            current = Some(PositiveRun {
                recording_id: recording_id.clone(),
                samples: vec![row.clone()],
            });
        }
        if let Some(completed) = current {
            if let Some(alarm) = completed.to_alarm(config.minimum_positive_samples) {
                all_alarms.push(alarm);
            } else {
                transient_burst_count += 1;
            }
        }
    }
    all_alarms.sort_by(|left, right| {
        left.recording_id
            .cmp(&right.recording_id)
            .then_with(|| left.start_seconds.total_cmp(&right.start_seconds))
    });

    let mut used_alarm_indices = BTreeSet::<usize>::new();
    let mut event_outcomes = Vec::with_capacity(fault_events.len());
    let mut detection_latencies = Vec::<f64>::new();
    let mut true_class_probabilities = Vec::<f64>::new();
    let mut isolation_correct_count = 0usize;
    let mut detected_event_ids = BTreeSet::<String>::new();

    for (recording_id, events) in &events_by_recording {
        for event in events {
            let deadline = (event.start_seconds + config.maximum_detection_latency_seconds)
                .min(event.end_seconds);
            let matching_alarm = all_alarms.iter().enumerate().find(|(index, alarm)| {
                !used_alarm_indices.contains(index)
                    && alarm.recording_id == *recording_id
                    && alarm.start_seconds >= event.start_seconds
                    && alarm.start_seconds < event.end_seconds
                    && alarm.confirmation_seconds <= deadline
            });
            let identity = format!("{}::{}", event.recording_id, event.event_id);
            let outcome = if let Some((index, alarm)) = matching_alarm {
                used_alarm_indices.insert(index);
                detected_event_ids.insert(identity);
                let latency = alarm.confirmation_seconds - event.start_seconds;
                detection_latencies.push(latency);
                let probabilities = alarm.isolation_probabilities_at_confirmation;
                let predicted = argmax_lowest_index(&probabilities);
                let true_probability = probabilities[event.target_isolation_index];
                true_class_probabilities.push(true_probability);
                let top1_correct = predicted == event.target_isolation_index;
                if top1_correct {
                    isolation_correct_count += 1;
                }
                EventOutcome {
                    recording_id: event.recording_id.clone(),
                    event_id: event.event_id.clone(),
                    onset_seconds: event.start_seconds,
                    detection_confirmation_seconds: Some(alarm.confirmation_seconds),
                    detection_latency_seconds: Some(latency),
                    target_isolation_index: event.target_isolation_index,
                    predicted_isolation_index: Some(predicted),
                    true_class_probability: Some(true_probability),
                    isolation_top1_correct: Some(top1_correct),
                }
            } else {
                EventOutcome {
                    recording_id: event.recording_id.clone(),
                    event_id: event.event_id.clone(),
                    onset_seconds: event.start_seconds,
                    detection_confirmation_seconds: None,
                    detection_latency_seconds: None,
                    target_isolation_index: event.target_isolation_index,
                    predicted_isolation_index: None,
                    true_class_probability: None,
                    isolation_top1_correct: None,
                }
            };
            event_outcomes.push(outcome);
        }
    }

    let mut false_alarm_episodes = Vec::new();
    let mut duplicate_alarm_count = 0usize;
    let mut late_alarm_count = 0usize;
    for (index, alarm) in all_alarms.iter().enumerate() {
        if used_alarm_indices.contains(&index) {
            continue;
        }
        let active_event = events_by_recording
            .get(&alarm.recording_id)
            .and_then(|events| {
                events.iter().find(|event| {
                    alarm.start_seconds >= event.start_seconds
                        && alarm.start_seconds < event.end_seconds
                })
            });
        if let Some(event) = active_event {
            let identity = format!("{}::{}", event.recording_id, event.event_id);
            if detected_event_ids.contains(&identity) {
                duplicate_alarm_count += 1;
            } else {
                late_alarm_count += 1;
            }
        } else {
            false_alarm_episodes.push(UnmatchedAlarm {
                recording_id: alarm.recording_id.clone(),
                start_seconds: alarm.start_seconds,
                confirmation_seconds: alarm.confirmation_seconds,
            });
        }
    }

    let mut observed_normal_exposure_seconds = 0.0;
    for (recording_id, rows) in &observations_by_recording {
        let events = events_by_recording
            .get(recording_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for pair in rows.windows(2) {
            let start = pair[0].time_seconds;
            let end = pair[1].time_seconds;
            let interval = end - start;
            if interval <= config.maximum_observation_gap_seconds {
                let fault_overlap = events
                    .iter()
                    .map(|event| {
                        (end.min(event.end_seconds) - start.max(event.start_seconds)).max(0.0)
                    })
                    .sum::<f64>();
                observed_normal_exposure_seconds += (interval - fault_overlap).max(0.0);
            }
        }
    }

    let fault_event_count = fault_events.len();
    let detected_event_count = detection_latencies.len();
    detection_latencies.sort_by(f64::total_cmp);
    true_class_probabilities.sort_by(f64::total_cmp);
    let mean_detection_latency_seconds = mean(&detection_latencies);
    let median_detection_latency_seconds = quantile_nearest_rank(&detection_latencies, 0.50);
    let p95_detection_latency_seconds = quantile_nearest_rank(&detection_latencies, 0.95);
    let maximum_detection_latency_seconds = detection_latencies.last().copied();
    let false_alarm_count = false_alarm_episodes.len();
    let false_alarms_per_observed_normal_hour = (observed_normal_exposure_seconds > 0.0)
        .then_some(false_alarm_count as f64 / observed_normal_exposure_seconds * 3600.0);

    Ok(EventEvaluationSummary {
        recording_count: windows.len(),
        fault_event_count,
        detected_event_count,
        missed_event_count: fault_event_count - detected_event_count,
        event_recall: (fault_event_count > 0)
            .then_some(detected_event_count as f64 / fault_event_count as f64),
        mean_detection_latency_seconds,
        median_detection_latency_seconds,
        p95_detection_latency_seconds,
        maximum_detection_latency_seconds,
        false_alarm_count,
        false_alarms_per_observed_normal_hour,
        duplicate_alarm_count_during_detected_events: duplicate_alarm_count,
        late_alarm_count_during_missed_events: late_alarm_count,
        transient_burst_count,
        observed_normal_exposure_seconds,
        mean_true_class_probability: mean(&true_class_probabilities),
        isolation_top1_accuracy: (!true_class_probabilities.is_empty())
            .then_some(isolation_correct_count as f64 / true_class_probabilities.len() as f64),
        event_outcomes,
        false_alarm_episodes,
    })
}

fn validate_prediction(
    observation: &DetectionObservation,
    tolerance: f64,
) -> Result<(), EventMetricError> {
    let probabilities = observation.isolation_probabilities;
    if probabilities
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
    {
        return Err(EventMetricError::InvalidPrediction(
            observation.recording_id.clone(),
        ));
    }
    let sum = probabilities.iter().sum::<f64>();
    if observation.detection {
        if (sum - 1.0).abs() > tolerance {
            return Err(EventMetricError::InvalidPrediction(
                observation.recording_id.clone(),
            ));
        }
    } else if probabilities.iter().any(|value| *value != 0.0) {
        return Err(EventMetricError::InvalidPrediction(
            observation.recording_id.clone(),
        ));
    }
    Ok(())
}

fn argmax_lowest_index(values: &[f64; EVENT_ISOLATION_SLOTS]) -> usize {
    let mut best = 0usize;
    for index in 1..EVENT_ISOLATION_SLOTS {
        if values[index] > values[best] {
            best = index;
        }
    }
    best
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then_some(values.iter().sum::<f64>() / values.len() as f64)
}

/// Nearest-rank quantile: q_p = sorted[ceil(p*n)-1].
fn quantile_nearest_rank(values: &[f64], quantile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let rank = (quantile * values.len() as f64).ceil() as usize;
    Some(values[rank.max(1).min(values.len()) - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> RecordingWindow {
        RecordingWindow {
            recording_id: "run-1".into(),
            start_seconds: 0.0,
            end_seconds: 12.0,
        }
    }

    fn event() -> FaultEvent {
        FaultEvent {
            recording_id: "run-1".into(),
            event_id: "fault-1".into(),
            start_seconds: 10.0,
            end_seconds: 12.0,
            target_isolation_index: 1,
        }
    }

    fn config() -> EventMetricConfig {
        EventMetricConfig {
            minimum_positive_samples: 3,
            alarm_merge_gap_seconds: 0.15,
            maximum_detection_latency_seconds: 10.0,
            maximum_observation_gap_seconds: 0.10,
            probability_sum_tolerance: 1e-6,
        }
    }

    fn stream(positive_indices: &[usize], class_index: usize) -> Vec<DetectionObservation> {
        (0..=240)
            .map(|index| {
                let detection = positive_indices.contains(&index);
                let mut isolation_probabilities = [0.0; EVENT_ISOLATION_SLOTS];
                if detection {
                    isolation_probabilities[class_index] = 1.0;
                }
                DetectionObservation {
                    recording_id: "run-1".into(),
                    sample_index: index as u64,
                    time_seconds: index as f64 * 0.05,
                    detection,
                    isolation_probabilities,
                }
            })
            .collect()
    }

    #[test]
    fn latency_uses_confirmation_time_and_duplicate_alarm_episodes_do_not_inflate_recall() {
        let observations = stream(&[202, 203, 204, 210, 211, 212], 1);
        let summary =
            evaluate_event_metrics(&[window()], &[event()], &observations, &config()).unwrap();
        assert_eq!(summary.fault_event_count, 1);
        assert_eq!(summary.detected_event_count, 1);
        assert_eq!(summary.missed_event_count, 0);
        assert_eq!(summary.event_recall, Some(1.0));
        assert!((summary.event_outcomes[0].detection_latency_seconds.unwrap() - 0.2).abs() < 1e-9);
        assert_eq!(summary.duplicate_alarm_count_during_detected_events, 1);
        assert_eq!(summary.isolation_top1_accuracy, Some(1.0));
    }

    #[test]
    fn pre_onset_alarm_is_false_and_cannot_be_reused_as_an_early_fault_detection() {
        let observations = stream(&[194, 195, 196], 1);
        let summary =
            evaluate_event_metrics(&[window()], &[event()], &observations, &config()).unwrap();
        assert_eq!(summary.detected_event_count, 0);
        assert_eq!(summary.missed_event_count, 1);
        assert_eq!(summary.false_alarm_count, 1);
        assert_eq!(summary.late_alarm_count_during_missed_events, 0);
        assert!(summary.false_alarms_per_observed_normal_hour.unwrap() > 0.0);
    }

    #[test]
    fn late_alarm_during_fault_is_not_misreported_as_a_normal_period_false_alarm() {
        let observations = stream(&[220, 221, 222], 1);
        let mut short_deadline = config();
        short_deadline.maximum_detection_latency_seconds = 0.5;
        let summary =
            evaluate_event_metrics(&[window()], &[event()], &observations, &short_deadline)
                .unwrap();
        assert_eq!(summary.detected_event_count, 0);
        assert_eq!(summary.late_alarm_count_during_missed_events, 1);
        assert_eq!(summary.false_alarm_count, 0);
    }

    #[test]
    fn alarm_episode_cannot_bridge_a_gap_that_is_not_valid_observation_exposure() {
        let observations = stream(&[202, 205, 208], 1);
        let summary =
            evaluate_event_metrics(&[window()], &[event()], &observations, &config()).unwrap();
        assert_eq!(summary.detected_event_count, 0);
        assert_eq!(summary.missed_event_count, 1);
        assert_eq!(summary.transient_burst_count, 3);
        assert_eq!(summary.false_alarm_count, 0);
    }

    #[test]
    fn explicit_negative_sample_resets_alarm_debounce_even_when_timestamps_are_close() {
        let observations = stream(&[202, 203, 205], 1);
        let summary =
            evaluate_event_metrics(&[window()], &[event()], &observations, &config()).unwrap();
        assert_eq!(summary.detected_event_count, 0);
        assert_eq!(summary.missed_event_count, 1);
        assert_eq!(summary.transient_burst_count, 2);
        assert_eq!(summary.false_alarm_count, 0);
    }

    #[test]
    fn short_positive_bursts_are_visible_but_not_counted_as_confirmed_alarms() {
        let observations = stream(&[100], 1);
        let summary = evaluate_event_metrics(&[window()], &[], &observations, &config()).unwrap();
        assert_eq!(summary.transient_burst_count, 1);
        assert_eq!(summary.false_alarm_count, 0);
        assert_eq!(summary.event_recall, None);
    }

    #[test]
    fn missing_observation_gaps_do_not_inflate_normal_exposure() {
        let mut observations = stream(&[], 0);
        observations.retain(|row| !(row.sample_index > 40 && row.sample_index < 200));
        let summary =
            evaluate_event_metrics(&[window()], &[event()], &observations, &config()).unwrap();
        assert!(summary.observed_normal_exposure_seconds < 4.1);
        assert_eq!(summary.false_alarms_per_observed_normal_hour, Some(0.0));
    }

    #[test]
    fn overlapping_fault_events_are_rejected_before_matching() {
        let mut second = event();
        second.event_id = "fault-2".into();
        second.start_seconds = 11.0;
        assert_eq!(
            evaluate_event_metrics(&[window()], &[event(), second], &stream(&[], 0), &config()),
            Err(EventMetricError::OverlappingFaultEvents("run-1".into()))
        );
    }

    #[test]
    fn invalid_no_detection_isolation_vector_is_rejected() {
        let mut observations = stream(&[], 0);
        observations[0].isolation_probabilities[0] = 0.25;
        assert_eq!(
            evaluate_event_metrics(&[window()], &[], &observations, &config()),
            Err(EventMetricError::InvalidPrediction("run-1".into()))
        );
    }

    #[test]
    fn six_shift_axes_remain_separate_and_bind_data_limits() {
        let matrix: serde_json::Value =
            serde_json::from_str(include_str!("../shift-matrix-v0.1.json")).unwrap();
        let axes = matrix["axes"].as_array().unwrap();
        let ids = axes
            .iter()
            .map(|axis| axis["id"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 6);
        assert!(ids.contains("C_SHIFT_DRIVING_CYCLE"));
        assert!(ids.contains("F_SHIFT_HELD_OUT_FAULT_FAMILY"));
        assert!(ids.contains("C_PLUS_F_SHIFT"));
        assert_eq!(matrix["framing"]["no_row_randomization"], true);
        assert!(
            matrix["cross_engine_and_fleet"]["status"]
                .as_str()
                .unwrap()
                .contains("unsupported")
        );
        assert_eq!(matrix["status"], "protocol_only_not_executed");
    }

    #[test]
    fn five_way_unknown_fault_slot_is_supported() {
        let mut unknown = event();
        unknown.target_isolation_index = 4;
        let observations = stream(&[202, 203, 204], 4);
        let summary =
            evaluate_event_metrics(&[window()], &[unknown], &observations, &config()).unwrap();
        assert_eq!(summary.event_recall, Some(1.0));
        assert_eq!(summary.event_outcomes[0].predicted_isolation_index, Some(4));
        assert_eq!(summary.event_outcomes[0].isolation_top1_correct, Some(true));
    }

    #[test]
    fn event_protocol_is_frozen_in_tournament_and_run_schema() {
        let tournament: serde_json::Value =
            serde_json::from_str(include_str!("../tournament-v0.1.json")).unwrap();
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../benchmark-run-manifest.schema.json")).unwrap();

        assert_eq!(
            tournament["event_evaluation"]["schema_version"],
            "liu-ice-event-metrics-v0.1"
        );
        assert_eq!(
            tournament["event_evaluation"]["status"],
            "protocol_only_not_executed"
        );
        assert_eq!(
            tournament["event_evaluation"]["alarm_debounce"]["minimum_positive_samples"],
            3
        );
        assert!(
            tournament["research_targets_not_achieved"]
                .as_array()
                .unwrap()
                .iter()
                .all(|target| {
                    target["status"] == "aspirational"
                        || target["status"] == "blocked_on_full_data_and_split_provenance"
                        || target["status"] == "blocked_on_dataset_access_and_protocol_audit"
                })
        );
        assert!(
            schema["properties"]["protocol"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == "event_evaluation")
        );
        assert!(
            schema["properties"]["metrics"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field == "false_alarms_per_observed_normal_hour")
        );
    }
}
