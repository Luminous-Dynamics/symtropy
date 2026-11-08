// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Timing-aware drift-event evaluation.
//!
//! The evaluator implements an inclusive acceptable detection window:
//! [drift_start - delta_pre, drift_end + max_delay]. A matched alarm is a
//! detector event, not a causal diagnosis. Inputs use one-based stream ticks.

use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriftEvent {
    pub event_id: String,
    pub start_tick: usize,
    pub end_tick: usize,
    pub delta_pre_ticks: usize,
    pub max_delay_ticks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectionMatch {
    pub event_id: String,
    pub window_start_tick: usize,
    pub window_end_tick: usize,
    pub matched_alarm_tick: Option<usize>,
    pub delay_ticks: Option<i64>,
    pub normalized_detection_time: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DetectionEvaluation {
    pub stream_ticks: usize,
    pub true_positives: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
    /// False-positive alarms per stream tick (not a probability).
    pub false_alarm_rate_per_tick: f64,
    pub missed_drift_rate: Option<f64>,
    /// Average d_alarm - d_start among matched events; may be negative for
    /// accepted early alarms.
    pub mean_detection_delay_ticks: Option<f64>,
    /// Average (d_alarm - d_start) / max_delay among matched events.
    /// Negative values are preserved for accepted early alarms.
    pub mean_normalized_detection_time: Option<f64>,
    pub matches: Vec<DetectionMatch>,
    pub false_alarm_ticks: Vec<usize>,
    pub missed_event_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionEvaluationError {
    EmptyStream,
    EmptyEventId,
    InvalidEventInterval { event_id: String },
    ZeroMaximumDelay { event_id: String },
    EventOutsideStream { event_id: String },
    EventOrderViolation,
    OverlappingAcceptanceWindows { earlier: String, later: String },
    AlarmOutsideStream { alarm_tick: usize },
    AlarmOrderViolation,
}

impl fmt::Display for DetectionEvaluationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStream => write!(formatter, "stream_ticks must be positive"),
            Self::EmptyEventId => write!(formatter, "drift event id must not be empty"),
            Self::InvalidEventInterval { event_id } => {
                write!(formatter, "drift event {event_id} has end before start")
            }
            Self::ZeroMaximumDelay { event_id } => {
                write!(formatter, "drift event {event_id} has zero maximum detection delay")
            }
            Self::EventOutsideStream { event_id } => {
                write!(formatter, "drift event {event_id} lies outside the stream")
            }
            Self::EventOrderViolation => {
                write!(formatter, "drift events must be sorted by strictly increasing start tick")
            }
            Self::OverlappingAcceptanceWindows { earlier, later } => write!(
                formatter,
                "acceptable detection windows overlap for events {earlier} and {later}"
            ),
            Self::AlarmOutsideStream { alarm_tick } => {
                write!(formatter, "alarm at tick {alarm_tick} lies outside the stream")
            }
            Self::AlarmOrderViolation => {
                write!(formatter, "alarm ticks must be sorted in nondecreasing order")
            }
        }
    }
}

impl std::error::Error for DetectionEvaluationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EventWindow {
    event: DriftEvent,
    start_tick: usize,
    end_tick: usize,
}

/// Evaluate a sorted sequence of detector alarm ticks against sorted ground
/// truth drift intervals.
///
/// Matching rule: each event receives the earliest still-unmatched alarm in
/// its inclusive acceptable window. Any alarm not assigned to an event is a
/// false positive, including duplicates in the same window. To avoid ambiguous
/// attribution, overlapping acceptable windows are rejected.
pub fn evaluate_drift_detection(
    stream_ticks: usize,
    events: &[DriftEvent],
    alarm_ticks: &[usize],
) -> Result<DetectionEvaluation, DetectionEvaluationError> {
    if stream_ticks == 0 {
        return Err(DetectionEvaluationError::EmptyStream);
    }

    for (index, alarm_tick) in alarm_ticks.iter().copied().enumerate() {
        if alarm_tick == 0 || alarm_tick > stream_ticks {
            return Err(DetectionEvaluationError::AlarmOutsideStream { alarm_tick });
        }
        if index > 0 && alarm_tick < alarm_ticks[index - 1] {
            return Err(DetectionEvaluationError::AlarmOrderViolation);
        }
    }

    let mut windows = Vec::with_capacity(events.len());
    for event in events {
        if event.event_id.trim().is_empty() {
            return Err(DetectionEvaluationError::EmptyEventId);
        }
        if event.end_tick < event.start_tick {
            return Err(DetectionEvaluationError::InvalidEventInterval {
                event_id: event.event_id.clone(),
            });
        }
        if event.max_delay_ticks == 0 {
            return Err(DetectionEvaluationError::ZeroMaximumDelay {
                event_id: event.event_id.clone(),
            });
        }
        if event.start_tick == 0 || event.end_tick > stream_ticks {
            return Err(DetectionEvaluationError::EventOutsideStream {
                event_id: event.event_id.clone(),
            });
        }

        let window_start_tick = event.start_tick.saturating_sub(event.delta_pre_ticks).max(1);
        let window_end_tick = event
            .end_tick
            .checked_add(event.max_delay_ticks)
            .ok_or_else(|| DetectionEvaluationError::EventOutsideStream {
                event_id: event.event_id.clone(),
            })?
            .min(stream_ticks);
        windows.push(EventWindow {
            event: event.clone(),
            start_tick: window_start_tick,
            end_tick: window_end_tick,
        });
    }

    for index in 1..windows.len() {
        let previous = &windows[index - 1];
        let current = &windows[index];
        if current.event.start_tick <= previous.event.start_tick {
            return Err(DetectionEvaluationError::EventOrderViolation);
        }
        if current.start_tick <= previous.end_tick {
            return Err(DetectionEvaluationError::OverlappingAcceptanceWindows {
                earlier: previous.event.event_id.clone(),
                later: current.event.event_id.clone(),
            });
        }
    }

    let mut alarm_matched = vec![false; alarm_ticks.len()];
    let mut matches = Vec::with_capacity(windows.len());
    let mut missed_event_ids = Vec::new();
    let mut delays = Vec::new();
    let mut ndts = Vec::new();

    for window in &windows {
        let matched_index = alarm_ticks
            .iter()
            .enumerate()
            .find(|(index, tick)| {
                !alarm_matched[*index]
                    && **tick >= window.start_tick
                    && **tick <= window.end_tick
            })
            .map(|(index, _)| index);

        let matched_alarm_tick = matched_index.map(|index| {
            alarm_matched[index] = true;
            alarm_ticks[index]
        });

        let delay_ticks = matched_alarm_tick.map(|alarm_tick| {
            alarm_tick as i64 - window.event.start_tick as i64
        });
        let normalized_detection_time = delay_ticks.map(|delay| {
            delay as f64 / window.event.max_delay_ticks as f64
        });

        if let Some(delay) = delay_ticks {
            delays.push(delay as f64);
            ndts.push(normalized_detection_time.expect("NDT follows a matched alarm"));
        } else {
            missed_event_ids.push(window.event.event_id.clone());
        }

        matches.push(DetectionMatch {
            event_id: window.event.event_id.clone(),
            window_start_tick: window.start_tick,
            window_end_tick: window.end_tick,
            matched_alarm_tick,
            delay_ticks,
            normalized_detection_time,
        });
    }

    let false_alarm_ticks: Vec<usize> = alarm_ticks
        .iter()
        .enumerate()
        .filter_map(|(index, tick)| (!alarm_matched[index]).then_some(*tick))
        .collect();

    let true_positives = matches
        .iter()
        .filter(|item| item.matched_alarm_tick.is_some())
        .count();
    let false_positives = false_alarm_ticks.len();
    let false_negatives = missed_event_ids.len();

    let precision_denominator = true_positives + false_positives;
    let recall_denominator = true_positives + false_negatives;
    let precision = (precision_denominator > 0)
        .then_some(true_positives as f64 / precision_denominator as f64);
    let recall = (recall_denominator > 0)
        .then_some(true_positives as f64 / recall_denominator as f64);
    let f1 = match (precision, recall) {
        (Some(p), Some(r)) if p + r > 0.0 => Some(2.0 * p * r / (p + r)),
        (Some(_), Some(_)) => Some(0.0),
        (None, Some(0.0)) => Some(0.0),
        (Some(0.0), None) => Some(0.0),
        _ => None,
    };

    let matched_count = delays.len();
    Ok(DetectionEvaluation {
        stream_ticks,
        true_positives,
        false_positives,
        false_negatives,
        precision,
        recall,
        f1,
        false_alarm_rate_per_tick: false_positives as f64 / stream_ticks as f64,
        missed_drift_rate: (recall_denominator > 0)
            .then_some(false_negatives as f64 / recall_denominator as f64),
        mean_detection_delay_ticks: (matched_count > 0)
            .then_some(delays.iter().sum::<f64>() / matched_count as f64),
        mean_normalized_detection_time: (matched_count > 0)
            .then_some(ndts.iter().sum::<f64>() / matched_count as f64),
        matches,
        false_alarm_ticks,
        missed_event_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(
        id: &str,
        start: usize,
        end: usize,
        delta_pre: usize,
        max_delay: usize,
    ) -> DriftEvent {
        DriftEvent {
            event_id: id.to_string(),
            start_tick: start,
            end_tick: end,
            delta_pre_ticks: delta_pre,
            max_delay_ticks: max_delay,
        }
    }

    #[test]
    fn inclusive_window_counts_pre_onset_alarm_as_true_positive_and_preserves_negative_ndt() {
        let result = evaluate_drift_detection(
            100,
            &[event("drift-a", 50, 50, 5, 10)],
            &[45],
        )
        .expect("valid evaluation");
        assert_eq!(result.true_positives, 1);
        assert_eq!(result.false_positives, 0);
        assert_eq!(result.false_negatives, 0);
        assert_eq!(result.matches[0].delay_ticks, Some(-5));
        assert_eq!(result.matches[0].normalized_detection_time, Some(-0.5));
        assert_eq!(result.f1, Some(1.0));
    }

    #[test]
    fn late_alarm_on_last_acceptable_tick_is_a_true_positive() {
        let result = evaluate_drift_detection(
            100,
            &[event("drift-a", 20, 25, 2, 10)],
            &[35],
        )
        .expect("valid evaluation");
        assert_eq!(result.matches[0].matched_alarm_tick, Some(35));
        assert_eq!(result.matches[0].normalized_detection_time, Some(1.5));
        // Detection is accepted because the window extends from end_tick.
    }

    #[test]
    fn alarms_outside_windows_and_duplicate_alarms_are_false_positives() {
        let result = evaluate_drift_detection(
            100,
            &[event("drift-a", 20, 20, 2, 5)],
            &[10, 20, 20, 26],
        )
        .expect("valid evaluation");
        assert_eq!(result.true_positives, 1);
        assert_eq!(result.false_positives, 3);
        assert_eq!(result.false_alarm_ticks, vec![10, 20, 26]);
        assert_eq!(result.false_alarm_rate_per_tick, 0.03);
    }

    #[test]
    fn missing_detection_is_false_negative_and_counts_in_missed_rate() {
        let result = evaluate_drift_detection(
            100,
            &[event("drift-a", 20, 20, 2, 5), event("drift-b", 60, 65, 3, 4)],
            &[1],
        )
        .expect("valid evaluation");
        assert_eq!(result.true_positives, 0);
        assert_eq!(result.false_negatives, 2);
        assert_eq!(result.missed_drift_rate, Some(1.0));
        assert_eq!(result.f1, Some(0.0));
    }

    #[test]
    fn multiple_non_overlapping_drift_windows_match_alarms_one_to_one() {
        let result = evaluate_drift_detection(
            100,
            &[
                event("drift-a", 20, 20, 2, 5),
                event("drift-b", 60, 60, 2, 5),
            ],
            &[19, 63],
        )
        .expect("valid evaluation");
        assert_eq!(result.true_positives, 2);
        assert_eq!(result.false_positives, 0);
        assert_eq!(result.false_negatives, 0);
    }

    #[test]
    fn overlapping_detection_windows_fail_closed() {
        let error = evaluate_drift_detection(
            100,
            &[event("drift-a", 20, 30, 5, 10), event("drift-b", 40, 45, 5, 10)],
            &[25, 42],
        )
        .expect_err("overlap must be rejected");
        assert!(matches!(
            error,
            DetectionEvaluationError::OverlappingAcceptanceWindows { .. }
        ));
    }

    #[test]
    fn unsorted_events_or_alarms_fail_closed() {
        assert!(matches!(
            evaluate_drift_detection(
                100,
                &[event("b", 50, 50, 2, 5), event("a", 20, 20, 2, 5)],
                &[20],
            ),
            Err(DetectionEvaluationError::EventOrderViolation)
        ));
        assert!(matches!(
            evaluate_drift_detection(
                100,
                &[event("a", 20, 20, 2, 5)],
                &[30, 10],
            ),
            Err(DetectionEvaluationError::AlarmOrderViolation)
        ));
    }

    #[test]
    fn empty_detection_problem_has_undefined_precision_recall_and_f1() {
        let result = evaluate_drift_detection(100, &[], &[]).expect("valid empty problem");
        assert_eq!(result.precision, None);
        assert_eq!(result.recall, None);
        assert_eq!(result.f1, None);
        assert_eq!(result.false_alarm_rate_per_tick, 0.0);
        assert_eq!(result.missed_drift_rate, None);
    }

    #[test]
    fn zero_delay_or_malformed_events_fail_closed() {
        assert!(matches!(
            evaluate_drift_detection(100, &[event("a", 10, 10, 0, 0)], &[]),
            Err(DetectionEvaluationError::ZeroMaximumDelay { .. })
        ));
        assert!(matches!(
            evaluate_drift_detection(100, &[event("a", 10, 9, 0, 2)], &[]),
            Err(DetectionEvaluationError::InvalidEventInterval { .. })
        ));
    }
}
