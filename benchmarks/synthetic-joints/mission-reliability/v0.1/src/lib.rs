#![forbid(unsafe_code)]

//! Dependency-free first-event survival and competing-risk reference.
//!
//! This module only analyzes one time-to-first-event observation per
//! independent specimen. Recurrent events, interval/left censoring, clustered
//! estimators and accelerated-life extrapolation are deliberately not
//! implemented here. Repeated cycles must never be passed as independent
//! specimens.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExposureUnit {
    Seconds,
    Minutes,
    Hours,
    Cycles,
    Metres,
    Joules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationStatus {
    Event { cause: String },
    RightCensored,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimeObservation {
    pub specimen_id: String,
    pub time: f64,
    pub unit: ExposureUnit,
    pub status: ObservationStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KaplanMeierStep {
    pub time: f64,
    pub unit: ExposureUnit,
    pub at_risk: usize,
    pub events: usize,
    pub right_censored: usize,
    /// Product-limit survival immediately after all events at this time.
    pub survival: f64,
    /// Greenwood standard error; None when the final risk set is exhausted.
    pub greenwood_standard_error: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompetingRiskStep {
    pub time: f64,
    pub unit: ExposureUnit,
    pub at_risk: usize,
    pub events_by_cause: Vec<(String, usize)>,
    pub right_censored: usize,
    pub event_free_survival: f64,
    /// Aalen–Johansen cumulative incidence by cause after this time.
    pub cumulative_incidence_by_cause: Vec<(String, f64)>,
}

fn validate_observations(observations: &[TimeObservation]) -> Result<ExposureUnit, String> {
    if observations.is_empty() {
        return Err("at least one independent specimen observation is required".to_owned());
    }

    let unit = observations[0].unit;
    let mut specimen_ids = BTreeSet::new();

    for observation in observations {
        if observation.specimen_id.trim().is_empty() {
            return Err("specimen_id must not be empty".to_owned());
        }
        if !specimen_ids.insert(observation.specimen_id.as_str()) {
            return Err(format!(
                "duplicate specimen_id {:?}; repeated cycles are not independent specimens",
                observation.specimen_id
            ));
        }
        if !observation.time.is_finite() || observation.time < 0.0 {
            return Err(format!(
                "specimen {:?} has invalid nonnegative exposure time",
                observation.specimen_id
            ));
        }
        if observation.unit != unit {
            return Err("all observations in one analysis must use the same exposure unit".to_owned());
        }
        if let ObservationStatus::Event { cause } = &observation.status {
            if cause.trim().is_empty() {
                return Err(format!(
                    "specimen {:?} has an event with an empty cause",
                    observation.specimen_id
                ));
            }
        }
    }

    Ok(unit)
}

fn sorted_observations(
    observations: &[TimeObservation],
) -> Result<(ExposureUnit, Vec<TimeObservation>), String> {
    let unit = validate_observations(observations)?;
    let mut sorted = observations.to_vec();
    sorted.sort_by(|left, right| left.time.total_cmp(&right.time));
    Ok((unit, sorted))
}

/// Computes the Kaplan–Meier product-limit estimator for time to first event.
///
/// Any observed event is treated as the event of interest (all-cause
/// time-to-first-event). Right-censored observations leave the risk set after
/// their censoring time. Event and censor observations tied at a time are
/// included in the risk set immediately before events at that time.
///
/// This function requires one row per independent specimen and right
/// censoring only. It does not estimate cause-specific incidence.
pub fn kaplan_meier(
    observations: &[TimeObservation],
) -> Result<Vec<KaplanMeierStep>, String> {
    let (unit, sorted) = sorted_observations(observations)?;
    let mut steps = Vec::new();
    let mut survival = 1.0_f64;
    let mut greenwood_sum = 0.0_f64;
    let mut index = 0usize;

    while index < sorted.len() {
        let time = sorted[index].time;
        let start = index;
        while index < sorted.len() && sorted[index].time == time {
            index += 1;
        }

        let at_risk = sorted.len() - start;
        let mut events = 0usize;
        let mut right_censored = 0usize;
        for observation in &sorted[start..index] {
            match observation.status {
                ObservationStatus::Event { .. } => events += 1,
                ObservationStatus::RightCensored => right_censored += 1,
            }
        }

        if events > 0 {
            survival *= 1.0 - events as f64 / at_risk as f64;
            if events < at_risk {
                greenwood_sum += events as f64
                    / (at_risk as f64 * (at_risk - events) as f64);
            }
        }

        let standard_error = if events == at_risk {
            None
        } else {
            let variance = survival * survival * greenwood_sum;
            if variance.is_finite() && variance >= 0.0 {
                Some(variance.sqrt())
            } else {
                return Err("Greenwood variance is non-finite".to_owned());
            }
        };

        steps.push(KaplanMeierStep {
            time,
            unit,
            at_risk,
            events,
            right_censored,
            survival,
            greenwood_standard_error: standard_error,
        });
    }

    Ok(steps)
}

/// Computes Aalen–Johansen cumulative incidence for observed event causes.
///
/// Censored observations leave the risk set after their censoring time.
/// Competing causes remain in the total event-free survival decrement, while
/// each cause receives its own cumulative incidence increment. This is not a
/// recurrent-event estimator; each specimen contributes its first event only.
pub fn cumulative_incidence(
    observations: &[TimeObservation],
) -> Result<Vec<CompetingRiskStep>, String> {
    let (unit, sorted) = sorted_observations(observations)?;
    let mut steps = Vec::new();
    let mut event_free_survival = 1.0_f64;
    let mut cumulative_incidence: BTreeMap<String, f64> = BTreeMap::new();
    let mut index = 0usize;

    while index < sorted.len() {
        let time = sorted[index].time;
        let start = index;
        while index < sorted.len() && sorted[index].time == time {
            index += 1;
        }

        let at_risk = sorted.len() - start;
        let mut events_by_cause: BTreeMap<String, usize> = BTreeMap::new();
        let mut right_censored = 0usize;

        for observation in &sorted[start..index] {
            match &observation.status {
                ObservationStatus::Event { cause } => {
                    *events_by_cause.entry(cause.clone()).or_default() += 1;
                }
                ObservationStatus::RightCensored => right_censored += 1,
            }
        }

        let total_events: usize = events_by_cause.values().sum();
        let survival_before = event_free_survival;

        if total_events > 0 {
            for (cause, events) in &events_by_cause {
                let increment =
                    survival_before * (*events as f64 / at_risk as f64);
                *cumulative_incidence.entry(cause.clone()).or_default() += increment;
            }
            event_free_survival *= 1.0 - total_events as f64 / at_risk as f64;
        }

        let incidence_sum: f64 = cumulative_incidence.values().sum();
        let probability_sum = incidence_sum + event_free_survival;
        if !probability_sum.is_finite()
            || (probability_sum - 1.0).abs() > 1.0e-10
        {
            return Err(format!(
                "Aalen–Johansen probability mass does not sum to one at time {time}"
            ));
        }

        steps.push(CompetingRiskStep {
            time,
            unit,
            at_risk,
            events_by_cause: events_by_cause.into_iter().collect(),
            right_censored,
            event_free_survival,
            cumulative_incidence_by_cause: cumulative_incidence
                .iter()
                .map(|(cause, incidence)| (cause.clone(), *incidence))
                .collect(),
        });
    }

    Ok(steps)
}

/// Computes restricted mean event-free time up to a prespecified horizon.
///
/// The horizon must not exceed the maximum observed follow-up time. This
/// prevents silently extrapolating a flat survival tail beyond the data.
pub fn restricted_mean_event_free_time(
    observations: &[TimeObservation],
    horizon: f64,
) -> Result<f64, String> {
    if !horizon.is_finite() || horizon < 0.0 {
        return Err("restricted-mean horizon must be finite and nonnegative".to_owned());
    }
    let (unit, sorted) = sorted_observations(observations)?;
    let maximum_follow_up = sorted.last().map(|row| row.time).unwrap_or(0.0);
    if horizon > maximum_follow_up {
        return Err(format!(
            "horizon {horizon} exceeds maximum observed follow-up {maximum_follow_up} {:?}",
            unit
        ));
    }

    let steps = kaplan_meier(observations)?;
    let mut area = 0.0_f64;
    let mut previous_time = 0.0_f64;
    let mut survival = 1.0_f64;

    for step in steps {
        if step.time >= horizon {
            area += survival * (horizon - previous_time);
            return Ok(area);
        }
        area += survival * (step.time - previous_time);
        previous_time = step.time;
        survival = step.survival;
    }

    area += survival * (horizon - previous_time);
    if area.is_finite() {
        Ok(area)
    } else {
        Err("restricted mean calculation produced a non-finite result".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(id: &str, time: f64, status: ObservationStatus) -> TimeObservation {
        TimeObservation {
            specimen_id: id.to_owned(),
            time,
            unit: ExposureUnit::Cycles,
            status,
        }
    }

    fn fixture() -> Vec<TimeObservation> {
        vec![
            obs("S1", 1.0, ObservationStatus::Event { cause: "thermal".to_owned() }),
            obs("S2", 2.0, ObservationStatus::Event { cause: "mechanical".to_owned() }),
            obs("S3", 2.0, ObservationStatus::RightCensored),
            obs("S4", 3.0, ObservationStatus::Event { cause: "thermal".to_owned() }),
            obs("S5", 4.0, ObservationStatus::RightCensored),
        ]
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "expected {expected:.12e}, got {actual:.12e}, tolerance {tolerance:.3e}"
        );
    }

    #[test]
    fn reliability_001_kaplan_meier_known_product_limit_and_risk_table() {
        let steps = kaplan_meier(&fixture()).unwrap();
        assert_eq!(steps.len(), 4);
        assert_eq!((steps[0].at_risk, steps[0].events, steps[0].right_censored), (5, 1, 0));
        assert_eq!((steps[1].at_risk, steps[1].events, steps[1].right_censored), (4, 1, 1));
        assert_eq!((steps[2].at_risk, steps[2].events, steps[2].right_censored), (2, 1, 0));
        assert_eq!((steps[3].at_risk, steps[3].events, steps[3].right_censored), (1, 0, 1));
        close(steps[0].survival, 0.8, 1.0e-12);
        close(steps[1].survival, 0.6, 1.0e-12);
        close(steps[2].survival, 0.3, 1.0e-12);
        close(steps[3].survival, 0.3, 1.0e-12);
    }

    #[test]
    fn reliability_002_greenwood_standard_error_has_known_value() {
        let observations = vec![
            obs("A", 1.0, ObservationStatus::Event { cause: "failure".to_owned() }),
            obs("B", 2.0, ObservationStatus::RightCensored),
            obs("C", 3.0, ObservationStatus::RightCensored),
        ];
        let steps = kaplan_meier(&observations).unwrap();
        // S(1)=2/3; Greenwood variance = S^2 * 1/(3*2) = 2/27.
        close(
            steps[0].greenwood_standard_error.unwrap(),
            (2.0_f64 / 27.0).sqrt(),
            1.0e-12,
        );
    }

    #[test]
    fn reliability_003_competing_risks_use_aalen_johansen_mass_balance() {
        let steps = cumulative_incidence(&fixture()).unwrap();
        assert_eq!(steps.len(), 4);
        close(steps[0].event_free_survival, 0.8, 1.0e-12);
        close(steps[1].event_free_survival, 0.6, 1.0e-12);
        close(steps[2].event_free_survival, 0.3, 1.0e-12);

        let third = &steps[2].cumulative_incidence_by_cause;
        let thermal = third.iter().find(|(cause, _)| cause == "thermal").unwrap().1;
        let mechanical = third.iter().find(|(cause, _)| cause == "mechanical").unwrap().1;
        close(thermal, 0.5, 1.0e-12);
        close(mechanical, 0.2, 1.0e-12);
        close(thermal + mechanical + steps[2].event_free_survival, 1.0, 1.0e-12);
    }

    #[test]
    fn reliability_004_same_time_events_and_censoring_share_pre_event_risk_set() {
        let observations = vec![
            obs("A", 1.0, ObservationStatus::Event { cause: "thermal".to_owned() }),
            obs("B", 1.0, ObservationStatus::Event { cause: "mechanical".to_owned() }),
            obs("C", 1.0, ObservationStatus::RightCensored),
            obs("D", 2.0, ObservationStatus::RightCensored),
        ];
        let steps = kaplan_meier(&observations).unwrap();
        assert_eq!(steps[0].at_risk, 4);
        assert_eq!(steps[0].events, 2);
        assert_eq!(steps[0].right_censored, 1);
        close(steps[0].survival, 0.5, 1.0e-12);
    }

    #[test]
    fn reliability_005_restricted_mean_survival_time_known_answer() {
        close(
            restricted_mean_event_free_time(&fixture(), 4.0).unwrap(),
            2.7,
            1.0e-12,
        );
    }

    #[test]
    fn reliability_006_restricted_mean_refuses_extrapolation_beyond_follow_up() {
        assert!(restricted_mean_event_free_time(&fixture(), 5.0).is_err());
    }

    #[test]
    fn reliability_007_duplicate_specimen_ids_are_not_independent_replicates() {
        let observations = vec![
            obs("same", 1.0, ObservationStatus::Event { cause: "A".to_owned() }),
            obs("same", 2.0, ObservationStatus::RightCensored),
        ];
        assert!(kaplan_meier(&observations).is_err());
        assert!(cumulative_incidence(&observations).is_err());
    }

    #[test]
    fn reliability_008_empty_mixed_units_invalid_time_and_empty_cause_reject() {
        assert!(kaplan_meier(&[]).is_err());
        let mixed_units = vec![
            obs("A", 1.0, ObservationStatus::RightCensored),
            TimeObservation {
                specimen_id: "B".to_owned(),
                time: 2.0,
                unit: ExposureUnit::Hours,
                status: ObservationStatus::RightCensored,
            },
        ];
        assert!(kaplan_meier(&mixed_units).is_err());

        let negative_time = vec![obs("A", -1.0, ObservationStatus::RightCensored)];
        assert!(kaplan_meier(&negative_time).is_err());

        let empty_cause = vec![obs("A", 1.0, ObservationStatus::Event { cause: " ".to_owned() })];
        assert!(kaplan_meier(&empty_cause).is_err());
    }

    #[test]
    fn reliability_009_final_failure_exhausts_risk_set_and_marks_greenwood_se_unknown() {
        let observations = vec![
            obs("A", 1.0, ObservationStatus::Event { cause: "failure".to_owned() }),
            obs("B", 1.0, ObservationStatus::Event { cause: "failure".to_owned() }),
        ];
        let steps = kaplan_meier(&observations).unwrap();
        close(steps[0].survival, 0.0, 1.0e-12);
        assert!(steps[0].greenwood_standard_error.is_none());
    }
}
