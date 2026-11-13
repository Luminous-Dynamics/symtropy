// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Event-conditioned observability evaluation.
//!
//! This module refuses to infer coverage from nominal route length. Callers
//! must allocate every in-scope route metre to an explicit observability state
//! and supply independent event truth for POD/false-alarm reporting.

use core::fmt;

/// Errors in event-conditioned coverage input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoverageError {
    /// A required segment/class/profile identifier is empty.
    EmptyIdentifier(&'static str),
    /// A declared duration/length is invalid.
    InvalidPositiveValue(&'static str),
    /// An exposure length is negative or non-finite.
    InvalidExposureLength,
    /// An observability state is missing or duplicated.
    IncompleteStatePartition,
    /// Allocated length does not equal the declared in-scope route length.
    LengthAccountingMismatch,
    /// Detection count exceeds independent eligible event trials.
    DetectionsExceedTrials,
    /// Event trials were declared but no observable exposure exists.
    TrialsWithoutObservableExposure,
}

impl fmt::Display for CoverageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(name) => write!(f, "coverage identifier is empty: {name}"),
            Self::InvalidPositiveValue(name) => write!(f, "coverage value must be finite and positive: {name}"),
            Self::InvalidExposureLength => write!(f, "exposure length must be finite and non-negative"),
            Self::IncompleteStatePartition => write!(f, "each observability state must occur exactly once"),
            Self::LengthAccountingMismatch => write!(f, "observability-state lengths do not account for the route"),
            Self::DetectionsExceedTrials => write!(f, "detected events cannot exceed independent eligible event trials"),
            Self::TrialsWithoutObservableExposure => {
                write!(f, "eligible event trials require non-zero observable exposure")
            }
        }
    }
}

impl std::error::Error for CoverageError {}

/// Result type for coverage evaluation.
pub type CoverageResult<T> = Result<T, CoverageError>;

/// Explicit disposition for an event-conditioned coverage cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageDisposition {
    /// Independent truth and observable exposure support the declared evaluation.
    Qualified,
    /// No eligible event trials exist for a POD estimate.
    NotAssessed,
    /// Evidence/observability is insufficient for a reliable metric.
    Indeterminate,
}

/// Mutually exclusive length states for one route segment and time interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservabilityState {
    /// Qualified for this exact event class/magnitude/profile/configuration.
    Observable,
    /// Some capability exists, but the qualification envelope is incomplete or degraded.
    Degraded,
    /// No measurement was made during this interval.
    Unobserved,
    /// The physical/sensing section cannot support the declared observation.
    Unusable,
    /// The sensor/interrogator/communications path was unavailable.
    Unavailable,
    /// No assessment has been performed.
    NotAssessed,
    /// State is unknown or evidence is conflicting.
    Unknown,
}

impl ObservabilityState {
    fn all() -> [Self; 7] {
        [
            Self::Observable,
            Self::Degraded,
            Self::Unobserved,
            Self::Unusable,
            Self::Unavailable,
            Self::NotAssessed,
            Self::Unknown,
        ]
    }
}

/// Length allocated to one observability state for a route segment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LengthAllocation {
    /// State for this segment's length fraction.
    pub state: ObservabilityState,
    /// Length in kilometres.
    pub length_km: f64,
}

/// Independent evidence and exposure for one event-conditioned route cell.
#[derive(Clone, Debug, PartialEq)]
pub struct EventConditionedCoverageInput {
    /// Stable route segment identity.
    pub segment_id: String,
    /// Event class being qualified.
    pub event_class: String,
    /// Declared minimum relevant event magnitude and unit/definition.
    pub minimum_magnitude: String,
    /// Installation/environment profile identity.
    pub profile_id: String,
    /// Exact interrogator/configuration revision.
    pub interrogator_config_revision: String,
    /// Start of this coverage interval, Unix milliseconds.
    pub interval_start_unix_ms: i64,
    /// End of this coverage interval, Unix milliseconds.
    pub interval_end_unix_ms: i64,
    /// Total in-scope route length, in kilometres.
    pub total_route_length_km: f64,
    /// Duration of the evaluated interval, in days.
    pub duration_days: f64,
    /// Mutually exclusive, complete length allocation.
    pub allocations: Vec<LengthAllocation>,
    /// True only when event labels/truth are independently sourced for POD.
    pub independent_event_truth_available: bool,
    /// True only when no-event/background exposure is independently labelled for false-alarm rate.
    pub independent_background_truth_available: bool,
    /// Event trials eligible for POD: truth events occurring in the declared observable envelope.
    pub eligible_event_trials: u64,
    /// Number of eligible events detected by the candidate method.
    pub detected_events: u64,
    /// False event alarms in the same observable exposure window.
    pub false_alarm_count: u64,
}

/// A two-sided Wilson score confidence interval for a binomial POD estimate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WilsonInterval {
    /// Lower endpoint, clamped to [0, 1].
    pub lower: f64,
    /// Upper endpoint, clamped to [0, 1].
    pub upper: f64,
    /// Confidence level represented by the z-score.
    pub confidence_level: f64,
}

/// Per-state exposure report.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateExposure {
    /// Explicit observability state.
    pub state: ObservabilityState,
    /// Route length in kilometres.
    pub length_km: f64,
    /// Length multiplied by interval duration, in km-days.
    pub exposure_km_days: f64,
}

/// Event-conditioned qualification report. No composite score is provided.
#[derive(Clone, Debug, PartialEq)]
pub struct CoverageAssessment {
    /// Segment/profile cell identity.
    pub segment_id: String,
    /// Event class.
    pub event_class: String,
    /// Minimum event magnitude profile.
    pub minimum_magnitude: String,
    /// Profile identity.
    pub profile_id: String,
    /// Overall disposition: Qualified only when both POD and false-alarm metrics are qualified.
    pub disposition: CoverageDisposition,
    /// Independent-truth disposition for the POD metric.
    pub pod_disposition: CoverageDisposition,
    /// Independent-background-truth disposition for the false-alarm metric.
    pub false_alarm_disposition: CoverageDisposition,
    /// Per-state length/exposure accounting.
    pub state_exposures: Vec<StateExposure>,
    /// Fraction of nominal route length in the fully observable state.
    pub observable_length_fraction: f64,
    /// Fully observable exposure in km-days.
    pub observable_exposure_km_days: f64,
    /// Degraded exposure in km-days, reported separately.
    pub degraded_exposure_km_days: f64,
    /// Eligible event trials.
    pub eligible_event_trials: u64,
    /// Detected event count.
    pub detected_events: u64,
    /// Critical miss count for the declared event class.
    pub missed_events: u64,
    /// POD point estimate. None when independent truth or trials are absent.
    pub probability_of_detection: Option<f64>,
    /// 95% Wilson interval for POD, when estimable.
    pub pod_95_interval: Option<WilsonInterval>,
    /// False alarms per fully observable fibre-km-day.
    pub false_alarms_per_observable_km_day: Option<f64>,
}

impl EventConditionedCoverageInput {
    /// Evaluate one event-conditioned cell with complete length accounting.
    ///
    /// The caller must ensure eligible_event_trials are independently labelled
    /// events occurring within the observable portion/profile and not within
    /// blind/unavailable sections. This method does not infer truth from DAS.
    pub fn evaluate(&self) -> CoverageResult<CoverageAssessment> {
        for (name, value) in [
            ("segment_id", self.segment_id.as_str()),
            ("event_class", self.event_class.as_str()),
            ("minimum_magnitude", self.minimum_magnitude.as_str()),
            ("profile_id", self.profile_id.as_str()),
            ("interrogator_config_revision", self.interrogator_config_revision.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(CoverageError::EmptyIdentifier(name));
            }
        }
        if self.interval_start_unix_ms >= self.interval_end_unix_ms {
            return Err(CoverageError::InvalidPositiveValue("coverage interval"));
        }
        for (name, value) in [
            ("total_route_length_km", self.total_route_length_km),
            ("duration_days", self.duration_days),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(CoverageError::InvalidPositiveValue(name));
            }
        }
        if self.detected_events > self.eligible_event_trials {
            return Err(CoverageError::DetectionsExceedTrials);
        }
        if self.allocations.len() != ObservabilityState::all().len() {
            return Err(CoverageError::IncompleteStatePartition);
        }

        for required_state in ObservabilityState::all() {
            if self.allocations.iter().filter(|item| item.state == required_state).count() != 1 {
                return Err(CoverageError::IncompleteStatePartition);
            }
        }
        let mut allocated_length_km = 0.0;
        for allocation in &self.allocations {
            if !allocation.length_km.is_finite() || allocation.length_km < 0.0 {
                return Err(CoverageError::InvalidExposureLength);
            }
            allocated_length_km += allocation.length_km;
        }
        let tolerance = 1e-9_f64.max(self.total_route_length_km * 1e-9);
        if (allocated_length_km - self.total_route_length_km).abs() > tolerance {
            return Err(CoverageError::LengthAccountingMismatch);
        }

        let state_exposures: Vec<StateExposure> = ObservabilityState::all()
            .iter()
            .map(|state| {
                let length_km = self.allocations.iter()
                    .find(|allocation| allocation.state == *state)
                    .map(|allocation| allocation.length_km)
                    .unwrap_or(0.0);
                StateExposure {
                    state: *state,
                    length_km,
                    exposure_km_days: length_km * self.duration_days,
                }
            })
            .collect();

        let observable_exposure_km_days = state_exposures.iter()
            .find(|item| item.state == ObservabilityState::Observable)
            .map(|item| item.exposure_km_days)
            .unwrap_or(0.0);
        let degraded_exposure_km_days = state_exposures.iter()
            .find(|item| item.state == ObservabilityState::Degraded)
            .map(|item| item.exposure_km_days)
            .unwrap_or(0.0);
        let observable_length_km = state_exposures.iter()
            .find(|item| item.state == ObservabilityState::Observable)
            .map(|item| item.length_km)
            .unwrap_or(0.0);
        let observable_length_fraction = observable_length_km / self.total_route_length_km;

        if self.eligible_event_trials > 0 && observable_exposure_km_days <= 0.0 {
            return Err(CoverageError::TrialsWithoutObservableExposure);
        }

        let pod = if self.independent_event_truth_available && self.eligible_event_trials > 0 {
            Some(self.detected_events as f64 / self.eligible_event_trials as f64)
        } else {
            None
        };
        let pod_disposition = if !self.independent_event_truth_available {
            CoverageDisposition::Indeterminate
        } else if self.eligible_event_trials == 0 {
            CoverageDisposition::NotAssessed
        } else {
            CoverageDisposition::Qualified
        };
        let pod_95_interval = if pod.is_some() {
            Some(wilson_interval(self.detected_events, self.eligible_event_trials, 1.959_963_984_540_054))
        } else {
            None
        };
        let false_alarms_per_observable_km_day =
            if self.independent_background_truth_available && observable_exposure_km_days > 0.0 {
                Some(self.false_alarm_count as f64 / observable_exposure_km_days)
            } else {
                None
            };
        let false_alarm_disposition = if !self.independent_background_truth_available || observable_exposure_km_days <= 0.0 {
            CoverageDisposition::Indeterminate
        } else {
            CoverageDisposition::Qualified
        };

        let disposition = if pod_disposition == CoverageDisposition::Qualified
            && false_alarm_disposition == CoverageDisposition::Qualified
        {
            CoverageDisposition::Qualified
        } else if pod_disposition == CoverageDisposition::Indeterminate
            || false_alarm_disposition == CoverageDisposition::Indeterminate
        {
            CoverageDisposition::Indeterminate
        } else {
            CoverageDisposition::NotAssessed
        };

        Ok(CoverageAssessment {
            segment_id: self.segment_id.clone(),
            event_class: self.event_class.clone(),
            minimum_magnitude: self.minimum_magnitude.clone(),
            profile_id: self.profile_id.clone(),
            disposition,
            pod_disposition,
            false_alarm_disposition,
            state_exposures,
            observable_length_fraction,
            observable_exposure_km_days,
            degraded_exposure_km_days,
            eligible_event_trials: self.eligible_event_trials,
            detected_events: self.detected_events,
            missed_events: self.eligible_event_trials - self.detected_events,
            probability_of_detection: pod,
            pod_95_interval,
            false_alarms_per_observable_km_day,
        })
    }
}

fn wilson_interval(successes: u64, trials: u64, z: f64) -> WilsonInterval {
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half = z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
    WilsonInterval {
        lower: (center - half).clamp(0.0, 1.0),
        upper: (center + half).clamp(0.0, 1.0),
        confidence_level: 0.95,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allocations(observable: f64, degraded: f64, unobserved: f64) -> Vec<LengthAllocation> {
        vec![
            LengthAllocation { state: ObservabilityState::Observable, length_km: observable },
            LengthAllocation { state: ObservabilityState::Degraded, length_km: degraded },
            LengthAllocation { state: ObservabilityState::Unobserved, length_km: unobserved },
            LengthAllocation { state: ObservabilityState::Unusable, length_km: 0.0 },
            LengthAllocation { state: ObservabilityState::Unavailable, length_km: 0.0 },
            LengthAllocation { state: ObservabilityState::NotAssessed, length_km: 0.0 },
            LengthAllocation { state: ObservabilityState::Unknown, length_km: 0.0 },
        ]
    }

    fn input() -> EventConditionedCoverageInput {
        EventConditionedCoverageInput {
            segment_id: "segment-01".to_owned(),
            event_class: "excavation_vibration".to_owned(),
            minimum_magnitude: "declared-threshold-v1".to_owned(),
            profile_id: "direct-buried-fibre-v1".to_owned(),
            interrogator_config_revision: "config-r1".to_owned(),
            interval_start_unix_ms: 1_000,
            interval_end_unix_ms: 86_401_000,
            total_route_length_km: 10.0,
            duration_days: 1.0,
            allocations: allocations(6.0, 2.0, 2.0),
            independent_event_truth_available: true,
            independent_background_truth_available: true,
            eligible_event_trials: 10,
            detected_events: 8,
            false_alarm_count: 3,
        }
    }

    #[test]
    fn computes_pod_wilson_interval_and_exposure_normalized_false_alarms() {
        let report = input().evaluate().expect("valid coverage input");
        assert_eq!(report.disposition, CoverageDisposition::Qualified);
        assert!((report.probability_of_detection.expect("POD") - 0.8).abs() < 1e-12);
        let interval = report.pod_95_interval.expect("95% interval");
        assert!(interval.lower < 0.8 && interval.upper > 0.8);
        assert!((report.observable_length_fraction - 0.6).abs() < 1e-12);
        assert!((report.observable_exposure_km_days - 6.0).abs() < 1e-12);
        assert!((report.degraded_exposure_km_days - 2.0).abs() < 1e-12);
        assert!((report.false_alarms_per_observable_km_day.expect("false alarm rate") - 0.5).abs() < 1e-12);
        assert_eq!(report.missed_events, 2);
    }

    #[test]
    fn pod_requires_independent_event_truth_but_background_metric_can_still_qualify() {
        let report = EventConditionedCoverageInput {
            independent_event_truth_available: false,
            ..input()
        }.evaluate().expect("input is structurally valid");
        assert_eq!(report.disposition, CoverageDisposition::Indeterminate);
        assert_eq!(report.pod_disposition, CoverageDisposition::Indeterminate);
        assert_eq!(report.false_alarm_disposition, CoverageDisposition::Qualified);
        assert!(report.probability_of_detection.is_none());
        assert!(report.pod_95_interval.is_none());
        assert!(report.false_alarms_per_observable_km_day.is_some());
    }

    #[test]
    fn zero_trials_are_not_reported_as_zero_pod() {
        let report = EventConditionedCoverageInput {
            eligible_event_trials: 0,
            detected_events: 0,
            false_alarm_count: 0,
            ..input()
        }.evaluate().expect("input is structurally valid");
        assert_eq!(report.disposition, CoverageDisposition::NotAssessed);
        assert!(report.probability_of_detection.is_none());
        assert!(report.false_alarms_per_observable_km_day.is_some());
    }

    #[test]
    fn blind_length_is_not_counted_as_observable_exposure() {
        let report = input().evaluate().expect("valid input");
        assert!((report.observable_exposure_km_days - 6.0).abs() < 1e-12);
        assert_eq!(
            report.state_exposures.iter().find(|x| x.state == ObservabilityState::Unobserved).unwrap().length_km,
            2.0
        );
    }

    #[test]
    fn rejects_unaccounted_route_length() {
        let mut value = input();
        value.allocations = allocations(6.0, 1.0, 2.0);
        assert_eq!(value.evaluate(), Err(CoverageError::LengthAccountingMismatch));
    }

    #[test]
    fn rejects_duplicate_observability_state() {
        let mut value = input();
        value.allocations[1].state = ObservabilityState::Observable;
        assert_eq!(value.evaluate(), Err(CoverageError::IncompleteStatePartition));
    }

    #[test]
    fn rejects_missing_observability_state() {
        let mut value = input();
        value.allocations.pop();
        assert_eq!(value.evaluate(), Err(CoverageError::IncompleteStatePartition));
    }

    #[test]
    fn rejects_detections_greater_than_event_trials() {
        let value = EventConditionedCoverageInput {
            detected_events: 11,
            ..input()
        };
        assert_eq!(value.evaluate(), Err(CoverageError::DetectionsExceedTrials));
    }

    #[test]
    fn rejects_negative_length() {
        let mut value = input();
        value.allocations[0].length_km = -1.0;
        assert_eq!(value.evaluate(), Err(CoverageError::InvalidExposureLength));
    }

    #[test]
    fn does_not_claim_events_in_a_fully_blind_cell() {
        let value = EventConditionedCoverageInput {
            allocations: allocations(0.0, 0.0, 10.0),
            eligible_event_trials: 1,
            detected_events: 0,
            ..input()
        };
        assert_eq!(value.evaluate(), Err(CoverageError::TrialsWithoutObservableExposure));
    }

    #[test]
    fn no_independent_truth_is_not_eligible_for_false_alarm_rate() {
        let report = EventConditionedCoverageInput {
            independent_background_truth_available: false,
            false_alarm_count: 3,
            ..input()
        }.evaluate().expect("valid input");
        assert!(report.false_alarms_per_observable_km_day.is_none());
        assert_eq!(report.false_alarm_disposition, CoverageDisposition::Indeterminate);
    }
}
