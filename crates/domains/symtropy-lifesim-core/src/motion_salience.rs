// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! OLI motion-salience admission and source-linked pulse-propagation assays.
//!
//! This is an explicitly versioned OLI operationalization inspired by the Motion
//! Salience Threshold (MST) literature. The 2026 source describes salience as the
//! rate of change of a neighbour's relative bearing, but its complete equation and
//! source data have not yet been admitted. This module therefore uses the absolute
//! wrapped finite-difference bearing rate; it is not a source-faithful reproduction.
//!
//! It separates cue admission, matched pulse-versus-sham response, and outward temporal
//! ordering. High global alignment alone is insufficient to pass this assay.

use std::collections::HashSet;
use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

/// OLI-v0 motion-salience threshold, in radians per second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSalienceGateConfig {
    pub threshold_rad_per_second: f64,
}

impl MotionSalienceGateConfig {
    pub fn validate(self) -> Result<(), MotionSalienceError> {
        if !self.threshold_rad_per_second.is_finite() {
            return Err(MotionSalienceError::NonFiniteParameter(
                "threshold_rad_per_second",
            ));
        }
        if self.threshold_rad_per_second < 0.0 {
            return Err(MotionSalienceError::NegativeParameter(
                "threshold_rad_per_second",
            ));
        }
        Ok(())
    }
}

/// Two observations of a focal agent's relative bearing to a candidate neighbour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativeBearingSample {
    pub observer_id: u32,
    pub target_id: u32,
    pub previous_bearing_rad: f64,
    pub current_bearing_rad: f64,
    pub sample_interval_seconds: f64,
    pub within_interaction_range: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SalienceGateStatus {
    Admitted,
    BelowThreshold,
    OutsideInteractionRange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSalienceDecision {
    pub observer_id: u32,
    pub target_id: u32,
    pub signed_bearing_rate_rad_per_second: f64,
    pub motion_salience_rad_per_second: f64,
    pub threshold_rad_per_second: f64,
    /// Warns when the wrapped angle change approaches the pi-radian sample limit.
    /// False does not prove that faster-than-Nyquist motion was absent.
    pub near_angular_sampling_limit: bool,
}

impl MotionSalienceDecision {
    pub const fn status(&self, within_interaction_range: bool) -> SalienceGateStatus {
        if !within_interaction_range {
            SalienceGateStatus::OutsideInteractionRange
        } else if self.motion_salience_rad_per_second >= self.threshold_rad_per_second {
            SalienceGateStatus::Admitted
        } else {
            SalienceGateStatus::BelowThreshold
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MotionSalienceError {
    NonFiniteParameter(&'static str),
    NegativeParameter(&'static str),
    NonPositiveSampleInterval,
    SelfInteraction {
        agent_id: u32,
    },
    DuplicateAgentId(u32),
    MissingSourceAgent(u32),
    SourceMustRespondAtTimeZero(u32),
    MissingMatchedSham,
    CommonRandomNumbersRequired,
    EmptyPopulation,
    InvalidObservationValue {
        agent_id: u32,
        field: &'static str,
    },
    DuplicateSweepCoordinate {
        noise_level_bits: u64,
        threshold_bits: u64,
    },
    SweepReportConfigurationMismatch,
}

impl fmt::Display for MotionSalienceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter(name) => write!(f, "{name} must be finite"),
            Self::NegativeParameter(name) => write!(f, "{name} must be non-negative"),
            Self::NonPositiveSampleInterval => write!(f, "sample interval must be finite and > 0"),
            Self::SelfInteraction { agent_id } => write!(
                f,
                "agent {agent_id} cannot see itself as a distinct neighbour"
            ),
            Self::DuplicateAgentId(id) => write!(f, "duplicate agent id {id}"),
            Self::MissingSourceAgent(id) => write!(f, "source agent {id} is missing"),
            Self::SourceMustRespondAtTimeZero(id) => {
                write!(f, "source agent {id} must respond at time zero")
            }
            Self::MissingMatchedSham => write!(f, "a matched sham is required"),
            Self::CommonRandomNumbersRequired => {
                write!(f, "pulse and sham must use common random numbers")
            }
            Self::EmptyPopulation => write!(f, "pulse propagation population must not be empty"),
            Self::InvalidObservationValue { agent_id, field } => {
                write!(f, "agent {agent_id} has invalid {field}")
            }
            Self::DuplicateSweepCoordinate {
                noise_level_bits,
                threshold_bits,
            } => write!(
                f,
                "duplicate passability sweep coordinate noise_bits={noise_level_bits:#x}, threshold_bits={threshold_bits:#x}"
            ),
            Self::SweepReportConfigurationMismatch => {
                write!(
                    f,
                    "sweep point and embedded report configuration do not match"
                )
            }
        }
    }
}

impl Error for MotionSalienceError {}

/// Shortest signed angular difference in (-pi, pi].
pub fn wrapped_angular_difference(current_rad: f64, previous_rad: f64) -> f64 {
    (current_rad - previous_rad + PI).rem_euclid(2.0 * PI) - PI
}

/// Evaluate the OLI absolute-relative-bearing-rate gate for one candidate neighbour.
pub fn evaluate_motion_salience(
    sample: RelativeBearingSample,
    config: MotionSalienceGateConfig,
) -> Result<MotionSalienceDecision, MotionSalienceError> {
    config.validate()?;
    if sample.observer_id == sample.target_id {
        return Err(MotionSalienceError::SelfInteraction {
            agent_id: sample.observer_id,
        });
    }
    if !sample.previous_bearing_rad.is_finite() {
        return Err(MotionSalienceError::InvalidObservationValue {
            agent_id: sample.observer_id,
            field: "previous_bearing_rad",
        });
    }
    if !sample.current_bearing_rad.is_finite() {
        return Err(MotionSalienceError::InvalidObservationValue {
            agent_id: sample.observer_id,
            field: "current_bearing_rad",
        });
    }
    if !sample.sample_interval_seconds.is_finite() || sample.sample_interval_seconds <= 0.0 {
        return Err(MotionSalienceError::NonPositiveSampleInterval);
    }

    let delta = wrapped_angular_difference(sample.current_bearing_rad, sample.previous_bearing_rad);
    let signed_rate = delta / sample.sample_interval_seconds;
    Ok(MotionSalienceDecision {
        observer_id: sample.observer_id,
        target_id: sample.target_id,
        signed_bearing_rate_rad_per_second: signed_rate,
        motion_salience_rad_per_second: signed_rate.abs(),
        threshold_rad_per_second: config.threshold_rad_per_second,
        near_angular_sampling_limit: delta.abs() >= 0.9 * PI,
    })
}

/// Paired pulse/sham response for one non-source agent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PulseAgentResponse {
    pub agent_id: u32,
    pub distance_from_source: f64,
    pub treatment_response_time_seconds: Option<f64>,
    pub sham_response_time_seconds: Option<f64>,
}

/// One matched pulse/sham run. The source is included as a zero-distance, time-zero
/// treatment response and is excluded from non-source response fractions.
#[derive(Debug, Clone, PartialEq)]
pub struct PulsePropagationRun {
    pub source_agent_id: u32,
    pub seed: u64,
    pub noise_level: f64,
    pub salience_threshold_rad_per_second: f64,
    pub has_matched_sham: bool,
    pub uses_common_random_numbers: bool,
    pub agents: Vec<PulseAgentResponse>,
}

/// Registered criteria for a candidate passability window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassabilityCriteria {
    pub minimum_treatment_response_fraction: f64,
    pub maximum_sham_response_fraction: f64,
    pub minimum_incremental_response_fraction: f64,
    pub minimum_outward_order_score: f64,
}

impl Default for PassabilityCriteria {
    fn default() -> Self {
        Self {
            minimum_treatment_response_fraction: 0.10,
            maximum_sham_response_fraction: 0.05,
            minimum_incremental_response_fraction: 0.10,
            minimum_outward_order_score: 0.75,
        }
    }
}

impl PassabilityCriteria {
    fn validate(self) -> Result<(), MotionSalienceError> {
        for (name, value) in [
            (
                "minimum_treatment_response_fraction",
                self.minimum_treatment_response_fraction,
            ),
            (
                "maximum_sham_response_fraction",
                self.maximum_sham_response_fraction,
            ),
            (
                "minimum_incremental_response_fraction",
                self.minimum_incremental_response_fraction,
            ),
            (
                "minimum_outward_order_score",
                self.minimum_outward_order_score,
            ),
        ] {
            if !value.is_finite() {
                return Err(MotionSalienceError::NonFiniteParameter(name));
            }
            if !(0.0..=1.0).contains(&value) {
                return Err(MotionSalienceError::InvalidObservationValue {
                    agent_id: u32::MAX,
                    field: name,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassabilityFailure {
    InsufficientTreatmentParticipation,
    ExcessiveShamActivation,
    InsufficientIncrementalResponse,
    NoOutwardOrderingEstimate,
    InsufficientOutwardOrdering,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PulsePropagationReport {
    pub source_agent_id: u32,
    pub seed: u64,
    pub noise_level: f64,
    pub salience_threshold_rad_per_second: f64,
    pub non_source_population: usize,
    pub treatment_response_count: usize,
    pub sham_response_count: usize,
    pub treatment_response_fraction: f64,
    pub sham_response_fraction: f64,
    pub incremental_response_fraction: f64,
    pub maximum_treatment_response_distance: f64,
    pub median_treatment_response_time_seconds: Option<f64>,
    /// Pairwise concordance for farther responders activating later than nearer ones.
    pub outward_order_score: Option<f64>,
    pub passable: bool,
    pub failures: Vec<PassabilityFailure>,
}

/// One independently seeded passability result at a noise/threshold coordinate.
#[derive(Debug, Clone, PartialEq)]
pub struct PassabilitySweepPoint {
    pub noise_level: f64,
    pub threshold_rad_per_second: f64,
    pub report: PulsePropagationReport,
}

/// A contiguous interval of thresholds that passed the source-linked pulse assay.
///
/// A bounded window requires at least one failed threshold below and above the passing
/// interval in the same noise slice. This is evidence for a bounded passability regime
/// only within the tested parameter grid; it does not establish a universal phase boundary.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassabilityWindow {
    pub noise_level: f64,
    pub lower_threshold_rad_per_second: f64,
    pub upper_threshold_rad_per_second: f64,
    pub passing_coordinate_count: usize,
    pub bounded_below: bool,
    pub bounded_above: bool,
}

impl PassabilityWindow {
    pub const fn is_bounded(self) -> bool {
        self.bounded_below && self.bounded_above
    }
}

/// Identify contiguous passable threshold intervals independently for each noise level.
///
/// The input may be unordered. Duplicate (noise, threshold) coordinates are rejected;
/// every embedded report must agree with the coordinate that contains it. Threshold
/// values are sorted numerically, not assumed to be equally spaced.
pub fn identify_passability_windows(
    points: &[PassabilitySweepPoint],
) -> Result<Vec<PassabilityWindow>, MotionSalienceError> {
    let mut coordinates = HashSet::with_capacity(points.len());
    let mut by_noise: std::collections::BTreeMap<u64, Vec<&PassabilitySweepPoint>> =
        std::collections::BTreeMap::new();

    for point in points {
        if !point.noise_level.is_finite() || point.noise_level < 0.0 {
            return Err(MotionSalienceError::InvalidObservationValue {
                agent_id: u32::MAX,
                field: "sweep_noise_level",
            });
        }
        if !point.threshold_rad_per_second.is_finite() || point.threshold_rad_per_second < 0.0 {
            return Err(MotionSalienceError::InvalidObservationValue {
                agent_id: u32::MAX,
                field: "sweep_threshold",
            });
        }
        if point.report.noise_level.to_bits() != point.noise_level.to_bits()
            || point.report.salience_threshold_rad_per_second.to_bits()
                != point.threshold_rad_per_second.to_bits()
        {
            return Err(MotionSalienceError::SweepReportConfigurationMismatch);
        }
        let key = (
            point.noise_level.to_bits(),
            point.threshold_rad_per_second.to_bits(),
        );
        if !coordinates.insert(key) {
            return Err(MotionSalienceError::DuplicateSweepCoordinate {
                noise_level_bits: key.0,
                threshold_bits: key.1,
            });
        }
        by_noise.entry(key.0).or_default().push(point);
    }

    let mut windows = Vec::new();
    for (noise_bits, slice) in &mut by_noise {
        slice.sort_by(|left, right| {
            left.threshold_rad_per_second
                .total_cmp(&right.threshold_rad_per_second)
        });
        let mut index = 0usize;
        while index < slice.len() {
            if !slice[index].report.passable {
                index += 1;
                continue;
            }

            let start = index;
            while index + 1 < slice.len() && slice[index + 1].report.passable {
                index += 1;
            }
            let end = index;
            windows.push(PassabilityWindow {
                noise_level: f64::from_bits(*noise_bits),
                lower_threshold_rad_per_second: slice[start].threshold_rad_per_second,
                upper_threshold_rad_per_second: slice[end].threshold_rad_per_second,
                passing_coordinate_count: end - start + 1,
                bounded_below: start > 0,
                bounded_above: end + 1 < slice.len(),
            });
            index += 1;
        }
    }
    Ok(windows)
}

fn validate_response_time(
    agent_id: u32,
    field: &'static str,
    value: Option<f64>,
) -> Result<(), MotionSalienceError> {
    if value.is_some_and(|time| !time.is_finite() || time < 0.0) {
        return Err(MotionSalienceError::InvalidObservationValue { agent_id, field });
    }
    Ok(())
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some(0.5 * (values[middle - 1] + values[middle]))
    } else {
        Some(values[middle])
    }
}

/// Analyze source-linked propagation relative to a matched sham. The outward-order score
/// excludes equal-distance and equal-time pairs; it is not a causal estimator by itself.
pub fn analyze_pulse_propagation(
    run: &PulsePropagationRun,
    criteria: PassabilityCriteria,
) -> Result<PulsePropagationReport, MotionSalienceError> {
    criteria.validate()?;
    if !run.has_matched_sham {
        return Err(MotionSalienceError::MissingMatchedSham);
    }
    if !run.uses_common_random_numbers {
        return Err(MotionSalienceError::CommonRandomNumbersRequired);
    }
    for (name, value) in [
        ("noise_level", run.noise_level),
        (
            "salience_threshold_rad_per_second",
            run.salience_threshold_rad_per_second,
        ),
    ] {
        if !value.is_finite() {
            return Err(MotionSalienceError::NonFiniteParameter(name));
        }
        if value < 0.0 {
            return Err(MotionSalienceError::NegativeParameter(name));
        }
    }
    if run.agents.is_empty() {
        return Err(MotionSalienceError::EmptyPopulation);
    }

    let mut ids = HashSet::with_capacity(run.agents.len());
    let mut source_found = false;
    let mut non_source = Vec::new();
    for agent in &run.agents {
        if !ids.insert(agent.agent_id) {
            return Err(MotionSalienceError::DuplicateAgentId(agent.agent_id));
        }
        if !agent.distance_from_source.is_finite() || agent.distance_from_source < 0.0 {
            return Err(MotionSalienceError::InvalidObservationValue {
                agent_id: agent.agent_id,
                field: "distance_from_source",
            });
        }
        validate_response_time(
            agent.agent_id,
            "treatment_response_time_seconds",
            agent.treatment_response_time_seconds,
        )?;
        validate_response_time(
            agent.agent_id,
            "sham_response_time_seconds",
            agent.sham_response_time_seconds,
        )?;

        if agent.agent_id == run.source_agent_id {
            source_found = true;
            if agent.distance_from_source > 1.0e-9
                || agent
                    .treatment_response_time_seconds
                    .is_none_or(|time| time > 1.0e-9)
            {
                return Err(MotionSalienceError::SourceMustRespondAtTimeZero(
                    run.source_agent_id,
                ));
            }
        } else {
            non_source.push(*agent);
        }
    }
    if !source_found {
        return Err(MotionSalienceError::MissingSourceAgent(run.source_agent_id));
    }

    let population = non_source.len();
    let treatment: Vec<_> = non_source
        .iter()
        .filter(|agent| agent.treatment_response_time_seconds.is_some())
        .collect();
    let sham_count = non_source
        .iter()
        .filter(|agent| agent.sham_response_time_seconds.is_some())
        .count();
    let treatment_fraction = if population == 0 {
        0.0
    } else {
        treatment.len() as f64 / population as f64
    };
    let sham_fraction = if population == 0 {
        0.0
    } else {
        sham_count as f64 / population as f64
    };
    let incremental_fraction = treatment_fraction - sham_fraction;
    let maximum_distance = treatment
        .iter()
        .map(|agent| agent.distance_from_source)
        .fold(0.0, f64::max);
    let median_time = median(
        treatment
            .iter()
            .filter_map(|agent| agent.treatment_response_time_seconds)
            .collect(),
    );

    let mut concordant = 0usize;
    let mut discordant = 0usize;
    for left in 0..treatment.len() {
        for right in (left + 1)..treatment.len() {
            let a = treatment[left];
            let b = treatment[right];
            let time_a = a
                .treatment_response_time_seconds
                .expect("filtered responder");
            let time_b = b
                .treatment_response_time_seconds
                .expect("filtered responder");
            let distance_delta = a.distance_from_source - b.distance_from_source;
            let time_delta = time_a - time_b;
            if distance_delta * time_delta > 0.0 {
                concordant += 1;
            } else if distance_delta * time_delta < 0.0 {
                discordant += 1;
            }
        }
    }
    let outward_order_score = (concordant + discordant > 0)
        .then_some(concordant as f64 / (concordant + discordant) as f64);

    let mut failures = Vec::new();
    if treatment_fraction < criteria.minimum_treatment_response_fraction {
        failures.push(PassabilityFailure::InsufficientTreatmentParticipation);
    }
    if sham_fraction > criteria.maximum_sham_response_fraction {
        failures.push(PassabilityFailure::ExcessiveShamActivation);
    }
    if incremental_fraction < criteria.minimum_incremental_response_fraction {
        failures.push(PassabilityFailure::InsufficientIncrementalResponse);
    }
    match outward_order_score {
        None => failures.push(PassabilityFailure::NoOutwardOrderingEstimate),
        Some(score) if score < criteria.minimum_outward_order_score => {
            failures.push(PassabilityFailure::InsufficientOutwardOrdering);
        }
        Some(_) => {}
    }

    Ok(PulsePropagationReport {
        source_agent_id: run.source_agent_id,
        seed: run.seed,
        noise_level: run.noise_level,
        salience_threshold_rad_per_second: run.salience_threshold_rad_per_second,
        non_source_population: population,
        treatment_response_count: treatment.len(),
        sham_response_count: sham_count,
        treatment_response_fraction: treatment_fraction,
        sham_response_fraction: sham_fraction,
        incremental_response_fraction: incremental_fraction,
        maximum_treatment_response_distance: maximum_distance,
        median_treatment_response_time_seconds: median_time,
        outward_order_score,
        passable: failures.is_empty(),
        failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn response_run(
        threshold: f64,
        treatment_times: &[Option<f64>],
        sham_times: &[Option<f64>],
    ) -> PulsePropagationRun {
        let mut agents = vec![PulseAgentResponse {
            agent_id: 0,
            distance_from_source: 0.0,
            treatment_response_time_seconds: Some(0.0),
            sham_response_time_seconds: None,
        }];
        for index in 0..treatment_times.len() {
            agents.push(PulseAgentResponse {
                agent_id: index as u32 + 1,
                distance_from_source: (index + 1) as f64,
                treatment_response_time_seconds: treatment_times[index],
                sham_response_time_seconds: sham_times[index],
            });
        }
        PulsePropagationRun {
            source_agent_id: 0,
            seed: 42,
            noise_level: 0.05,
            salience_threshold_rad_per_second: threshold,
            has_matched_sham: true,
            uses_common_random_numbers: true,
            agents,
        }
    }

    #[test]
    fn wrapped_bearing_rate_uses_shortest_angular_difference() {
        let sample = RelativeBearingSample {
            observer_id: 1,
            target_id: 2,
            previous_bearing_rad: 179.0_f64.to_radians(),
            current_bearing_rad: -179.0_f64.to_radians(),
            sample_interval_seconds: 0.5,
            within_interaction_range: true,
        };
        let decision = evaluate_motion_salience(
            sample,
            MotionSalienceGateConfig {
                threshold_rad_per_second: 0.05,
            },
        )
        .unwrap();
        assert!((decision.motion_salience_rad_per_second - 4.0_f64.to_radians()).abs() < 1.0e-12);
        assert_eq!(decision.status(true), SalienceGateStatus::Admitted);
    }

    #[test]
    fn gate_is_inclusive_at_threshold_and_excludes_out_of_range_neighbors() {
        let sample = RelativeBearingSample {
            observer_id: 1,
            target_id: 2,
            previous_bearing_rad: 0.0,
            current_bearing_rad: 0.5,
            sample_interval_seconds: 1.0,
            within_interaction_range: true,
        };
        let config = MotionSalienceGateConfig {
            threshold_rad_per_second: 0.5,
        };
        let decision = evaluate_motion_salience(sample, config).unwrap();
        assert_eq!(decision.status(true), SalienceGateStatus::Admitted);
        assert_eq!(
            decision.status(false),
            SalienceGateStatus::OutsideInteractionRange
        );
    }

    #[test]
    fn gate_warns_near_the_angular_sampling_limit() {
        let decision = evaluate_motion_salience(
            RelativeBearingSample {
                observer_id: 1,
                target_id: 2,
                previous_bearing_rad: 0.0,
                current_bearing_rad: 0.95 * PI,
                sample_interval_seconds: 0.1,
                within_interaction_range: true,
            },
            MotionSalienceGateConfig {
                threshold_rad_per_second: 1.0,
            },
        )
        .unwrap();
        assert!(decision.near_angular_sampling_limit);
    }

    #[test]
    fn rejects_invalid_gate_inputs() {
        for dt in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let result = evaluate_motion_salience(
                RelativeBearingSample {
                    observer_id: 1,
                    target_id: 2,
                    previous_bearing_rad: 0.0,
                    current_bearing_rad: 0.5,
                    sample_interval_seconds: dt,
                    within_interaction_range: true,
                },
                MotionSalienceGateConfig {
                    threshold_rad_per_second: 0.1,
                },
            );
            assert!(matches!(
                result,
                Err(MotionSalienceError::NonPositiveSampleInterval)
            ));
        }
    }

    #[test]
    fn source_specific_outward_pulse_passes_the_registered_assay() {
        let run = response_run(
            1.5,
            &[Some(1.0), Some(2.0), Some(3.0), Some(4.0), None],
            &[None, None, None, None, None],
        );
        let report = analyze_pulse_propagation(&run, PassabilityCriteria::default()).unwrap();
        assert!(report.passable);
        assert_eq!(report.treatment_response_count, 4);
        assert_eq!(report.sham_response_count, 0);
        assert_eq!(report.outward_order_score, Some(1.0));
        assert_eq!(report.maximum_treatment_response_distance, 4.0);
    }

    #[test]
    fn common_driver_sham_fails_source_specificity_even_with_full_participation() {
        let run = response_run(
            0.0,
            &[Some(1.0), Some(2.0), Some(3.0), Some(4.0), Some(5.0)],
            &[Some(1.0), Some(2.0), Some(3.0), Some(4.0), Some(5.0)],
        );
        let report = analyze_pulse_propagation(&run, PassabilityCriteria::default()).unwrap();
        assert_eq!(report.treatment_response_fraction, 1.0);
        assert_eq!(report.sham_response_fraction, 1.0);
        assert_eq!(report.incremental_response_fraction, 0.0);
        assert!(!report.passable);
        assert!(
            report
                .failures
                .contains(&PassabilityFailure::ExcessiveShamActivation)
        );
        assert!(
            report
                .failures
                .contains(&PassabilityFailure::InsufficientIncrementalResponse)
        );
    }

    #[test]
    fn closed_gate_fails_for_insufficient_participation() {
        let run = response_run(
            5.0,
            &[None, None, None, None, None],
            &[None, None, None, None, None],
        );
        let report = analyze_pulse_propagation(&run, PassabilityCriteria::default()).unwrap();
        assert!(!report.passable);
        assert_eq!(report.treatment_response_fraction, 0.0);
        assert!(
            report
                .failures
                .contains(&PassabilityFailure::InsufficientTreatmentParticipation)
        );
    }

    fn synthetic_report(noise: f64, threshold: f64, passable: bool) -> PulsePropagationReport {
        PulsePropagationReport {
            source_agent_id: 0,
            seed: 1,
            noise_level: noise,
            salience_threshold_rad_per_second: threshold,
            non_source_population: 10,
            treatment_response_count: if passable { 5 } else { 0 },
            sham_response_count: 0,
            treatment_response_fraction: if passable { 0.5 } else { 0.0 },
            sham_response_fraction: 0.0,
            incremental_response_fraction: if passable { 0.5 } else { 0.0 },
            maximum_treatment_response_distance: if passable { 5.0 } else { 0.0 },
            median_treatment_response_time_seconds: passable.then_some(2.0),
            outward_order_score: passable.then_some(1.0),
            passable,
            failures: if passable {
                Vec::new()
            } else {
                vec![PassabilityFailure::InsufficientTreatmentParticipation]
            },
        }
    }

    #[test]
    fn sweep_finds_bounded_intermediate_passability_window_per_noise() {
        let thresholds = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5];
        let points: Vec<_> = thresholds
            .into_iter()
            .map(|threshold| {
                let passable = (1.5..=2.5).contains(&threshold);
                PassabilitySweepPoint {
                    noise_level: 0.1,
                    threshold_rad_per_second: threshold,
                    report: synthetic_report(0.1, threshold, passable),
                }
            })
            .collect();
        let windows = identify_passability_windows(&points).unwrap();
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].lower_threshold_rad_per_second, 1.5);
        assert_eq!(windows[0].upper_threshold_rad_per_second, 2.5);
        assert_eq!(windows[0].passing_coordinate_count, 3);
        assert!(windows[0].is_bounded());
    }

    #[test]
    fn sweep_keeps_noise_slices_separate_and_rejects_duplicate_coordinates() {
        let mut points = Vec::new();
        for noise in [0.0, 0.1] {
            for threshold in [0.5, 1.0, 1.5] {
                let passable = threshold == 1.0;
                points.push(PassabilitySweepPoint {
                    noise_level: noise,
                    threshold_rad_per_second: threshold,
                    report: synthetic_report(noise, threshold, passable),
                });
            }
        }
        let windows = identify_passability_windows(&points).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(
            windows
                .iter()
                .map(|window| window.noise_level)
                .collect::<Vec<_>>(),
            vec![0.0, 0.1]
        );
        assert!(windows.iter().all(|window| window.is_bounded()));

        let mut duplicated = points.clone();
        duplicated.push(points[0].clone());
        assert!(matches!(
            identify_passability_windows(&duplicated),
            Err(MotionSalienceError::DuplicateSweepCoordinate { .. })
        ));
    }

    #[test]
    fn sweep_rejects_report_coordinate_mismatch() {
        let point = PassabilitySweepPoint {
            noise_level: 0.1,
            threshold_rad_per_second: 2.0,
            report: synthetic_report(0.1, 2.5, true),
        };
        assert!(matches!(
            identify_passability_windows(&[point]),
            Err(MotionSalienceError::SweepReportConfigurationMismatch)
        ));
    }

    #[test]
    fn propagation_rejects_unmatched_or_nonreproducible_shams() {
        let mut run = response_run(1.0, &[Some(1.0), Some(2.0)], &[None, None]);
        run.has_matched_sham = false;
        assert!(matches!(
            analyze_pulse_propagation(&run, PassabilityCriteria::default()),
            Err(MotionSalienceError::MissingMatchedSham)
        ));
        run.has_matched_sham = true;
        run.uses_common_random_numbers = false;
        assert!(matches!(
            analyze_pulse_propagation(&run, PassabilityCriteria::default()),
            Err(MotionSalienceError::CommonRandomNumbersRequired)
        ));
    }
}
