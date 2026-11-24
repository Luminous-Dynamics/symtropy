// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Matched-path benchmark harness for bounded-mean confidence sequences.
//!
//! Fixed-stake, PrPl-EB and Hedged are evaluated on the same bounded
//! observations, alpha, stopping checkpoints and decision threshold. Each
//! method is queried after every observation, so timings include the cost of
//! maintaining its anytime-valid interval rather than only a final fixed-n CI.
//!
//! This harness reports interval widths, known-truth containment on the supplied
//! fixture, first threshold decision, wall-clock update/query time, and a
//! simple state-retention diagnostic. It does not estimate population coverage
//! from one fixture or certify an orbital target distribution.

use std::hint::black_box;
use std::time::{Duration, Instant};

use crate::adaptive_importance_sampling::{
    AdaptiveImportanceSamplingError, BoundedMeanBettingSequence,
};
use crate::hedged_capital_sequence::{
    HedgedCapitalConfidenceSequence, HedgedConfidenceSequenceError,
};
use crate::predictable_empirical_bernstein::{PrPlEbConfidenceSequence, PrPlEbError};

/// Confidence-sequence method evaluated by the matched-path harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfidenceSequenceMethod {
    /// Eight fixed betting stakes with one-sided root inversion.
    FixedStake,
    /// Predictable plug-in empirical-Bernstein closed-form sequence.
    PrPlEb,
    /// Theorem-backed max-hedged-capital sequence on a fixed parameter grid.
    Hedged,
}

/// Decision state induced by a two-sided confidence interval and a threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceThresholdDecision {
    /// Upper confidence endpoint is strictly below the threshold.
    BelowThreshold,
    /// Lower confidence endpoint is at or above the threshold.
    AtOrAboveThreshold,
}

/// One method's confidence interval at a predeclared observation checkpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfidenceSequenceBenchmarkSnapshot {
    /// Number of observations used.
    pub sample_count: u64,
    /// Lower confidence endpoint.
    pub lower: f64,
    /// Upper confidence endpoint.
    pub upper: f64,
    /// Upper minus lower.
    pub width: f64,
    /// Whether this interval contains the fixture's known conditional mean.
    ///
    /// This is fixture-level containment, not a population coverage estimate.
    pub contains_fixture_truth: bool,
}

/// Method-specific result from a matched observation path.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfidenceSequenceMethodBenchmark {
    /// Method identity.
    pub method: ConfidenceSequenceMethod,
    /// Confidence intervals at the requested checkpoints.
    pub snapshots: Vec<ConfidenceSequenceBenchmarkSnapshot>,
    /// First sample count at which a threshold decision was made, if any.
    pub first_decision_sample_count: Option<u64>,
    /// First threshold decision, if any.
    pub first_decision: Option<ConfidenceThresholdDecision>,
    /// Median elapsed nanoseconds across identical full-path replays.
    ///
    /// Includes every update and interval query after each observation.
    /// This is local machine/time-source dependent and is not a universal SOTA claim.
    pub median_elapsed_nanoseconds: u128,
    /// Number of complete-path timing repetitions included in the median.
    pub timing_repetitions: usize,
    /// Number of observations retained in the method's internal state.
    ///
    /// Fixed-stake retains the entire path; PrPl-EB and Hedged retain
    /// constant-size / grid-size state respectively.
    pub retained_observation_count: usize,
    /// Candidate-mean grid size; zero for methods without a candidate grid.
    pub candidate_mean_grid_size: usize,
}

/// Comparable results from all three methods on one exact input path.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfidenceSequenceBenchmarkReport {
    /// Confidence error probability.
    pub alpha: f64,
    /// Known conditional mean of the bounded fixture.
    pub fixture_truth: f64,
    /// Optional event-risk threshold used for decision timing.
    pub decision_threshold: Option<f64>,
    /// Number of observations in the common path.
    pub observation_count: usize,
    /// Identical checkpoints requested for all methods.
    pub checkpoints: Vec<usize>,
    /// Method results in deterministic order.
    pub methods: Vec<ConfidenceSequenceMethodBenchmark>,
}

/// Validation or method update errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceSequenceBenchmarkError {
    /// Fixture truth must be finite and in [0,1].
    InvalidFixtureTruth,
    /// Decision threshold must be finite and in [0,1].
    InvalidDecisionThreshold,
    /// At least one observation is required.
    EmptyObservationPath,
    /// An observation is non-finite or outside [0,1].
    InvalidObservation,
    /// Checkpoints must be nonempty, strictly increasing, positive and no greater than n.
    InvalidCheckpoints,
    /// Timing repetitions must be nonzero.
    InvalidTimingRepetitions,
    /// Fixed-stake sequence update or inversion failed.
    FixedStakeFailure,
    /// PrPl-EB sequence update failed.
    PrPlEbFailure,
    /// Hedged sequence update failed.
    HedgedFailure,
    /// A generated confidence interval is non-finite or reversed.
    InvalidConfidenceInterval,
}

impl core::fmt::Display for ConfidenceSequenceBenchmarkError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidFixtureTruth => "fixture truth must lie in [0, 1]",
            Self::InvalidDecisionThreshold => "decision threshold must lie in [0, 1]",
            Self::EmptyObservationPath => "benchmark observation path is empty",
            Self::InvalidObservation => "benchmark observation must lie in [0, 1]",
            Self::InvalidCheckpoints => "benchmark checkpoints are invalid",
            Self::InvalidTimingRepetitions => "timing repetitions must be nonzero",
            Self::FixedStakeFailure => "fixed-stake confidence sequence failed",
            Self::PrPlEbFailure => "PrPl-EB confidence sequence failed",
            Self::HedgedFailure => "hedged confidence sequence failed",
            Self::InvalidConfidenceInterval => "confidence interval is non-finite or reversed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ConfidenceSequenceBenchmarkError {}

impl From<AdaptiveImportanceSamplingError> for ConfidenceSequenceBenchmarkError {
    fn from(_: AdaptiveImportanceSamplingError) -> Self {
        Self::FixedStakeFailure
    }
}

impl From<PrPlEbError> for ConfidenceSequenceBenchmarkError {
    fn from(_: PrPlEbError) -> Self {
        Self::PrPlEbFailure
    }
}

impl From<HedgedConfidenceSequenceError> for ConfidenceSequenceBenchmarkError {
    fn from(_: HedgedConfidenceSequenceError) -> Self {
        Self::HedgedFailure
    }
}

/// Evaluate fixed-stake, PrPl-EB and Hedged on an identical bounded path.
///
/// Each method receives every observation in the same order and is queried
/// after every draw. Snapshot checkpoints are only a reporting filter; they do
/// not alter the update or stopping behavior. The optional decision threshold
/// is applied identically: BelowThreshold requires upper < threshold and
/// AtOrAboveThreshold requires lower >= threshold.
///
/// The Hedged grid is supplied explicitly so grid resolution is visible in the
/// report. Wall-clock results are useful for paired local comparisons, but
/// must be repeated on pinned hardware/toolchains before making performance
/// claims.
pub fn benchmark_confidence_sequences(
    observations: &[f64],
    fixture_truth: f64,
    alpha: f64,
    parameter_grid: Vec<f64>,
    checkpoints: &[usize],
    decision_threshold: Option<f64>,
    timing_repetitions: usize,
) -> Result<ConfidenceSequenceBenchmarkReport, ConfidenceSequenceBenchmarkError> {
    if !fixture_truth.is_finite() || !(0.0..=1.0).contains(&fixture_truth) {
        return Err(ConfidenceSequenceBenchmarkError::InvalidFixtureTruth);
    }
    if !observations.iter().all(|value| value.is_finite() && (0.0..=1.0).contains(value)) {
        return Err(ConfidenceSequenceBenchmarkError::InvalidObservation);
    }
    if observations.is_empty() {
        return Err(ConfidenceSequenceBenchmarkError::EmptyObservationPath);
    }
    if checkpoints.is_empty()
        || checkpoints[0] == 0
        || checkpoints.last().copied().unwrap_or(0) > observations.len()
        || checkpoints.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(ConfidenceSequenceBenchmarkError::InvalidCheckpoints);
    }
    if timing_repetitions == 0 {
        return Err(ConfidenceSequenceBenchmarkError::InvalidTimingRepetitions);
    }
    if let Some(threshold) = decision_threshold {
        if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
            return Err(ConfidenceSequenceBenchmarkError::InvalidDecisionThreshold);
        }
    }

    // Construct once up front to validate alpha and the candidate grid before
    // any benchmark method runs.
    HedgedCapitalConfidenceSequence::new(alpha, parameter_grid.clone())?;

    let fixed = run_fixed_stake(
        observations,
        fixture_truth,
        alpha,
        checkpoints,
        decision_threshold,
        timing_repetitions,
    )?;
    let prpl = run_prpl_eb(
        observations,
        fixture_truth,
        alpha,
        checkpoints,
        decision_threshold,
        timing_repetitions,
    )?;
    let hedged = run_hedged(
        observations,
        fixture_truth,
        alpha,
        parameter_grid.len(),
        parameter_grid,
        checkpoints,
        decision_threshold,
        timing_repetitions,
    )?;

    Ok(ConfidenceSequenceBenchmarkReport {
        alpha,
        fixture_truth,
        decision_threshold,
        observation_count: observations.len(),
        checkpoints: checkpoints.to_vec(),
        methods: vec![fixed, prpl, hedged],
    })
}

fn run_fixed_stake(
    observations: &[f64],
    fixture_truth: f64,
    alpha: f64,
    checkpoints: &[usize],
    threshold: Option<f64>,
    repetitions: usize,
) -> Result<ConfidenceSequenceMethodBenchmark, ConfidenceSequenceBenchmarkError> {
    let started = Instant::now();
    let mut sequence = BoundedMeanBettingSequence::new();
    let mut snapshots = Vec::with_capacity(checkpoints.len());
    let mut first_decision = None;
    let mut first_decision_sample_count = None;
    let mut checkpoint_index = 0;

    for (index, observation) in observations.iter().copied().enumerate() {
        sequence.observe(observation)?;
        let interval = sequence.confidence_sequence(alpha)?;
        validate_interval(interval.lower, interval.upper)?;
        record_decision(
            threshold,
            interval.lower,
            interval.upper,
            index + 1,
            &mut first_decision,
            &mut first_decision_sample_count,
        );
        if checkpoint_index < checkpoints.len() && checkpoints[checkpoint_index] == index + 1 {
            snapshots.push(snapshot(
                index + 1,
                interval.lower,
                interval.upper,
                fixture_truth,
            ));
            checkpoint_index += 1;
        }
    }
    let first_elapsed = started.elapsed();
    let elapsed = median_elapsed(first_elapsed, repetitions.saturating_sub(1), || {
        let mut sequence = BoundedMeanBettingSequence::new();
        for observation in observations {
            sequence.observe(black_box(*observation))?;
            let interval = sequence.confidence_sequence(alpha)?;
            black_box(interval.width());
        }
        Ok::<(), AdaptiveImportanceSamplingError>(())
    })
    .map_err(|_| ConfidenceSequenceBenchmarkError::FixedStakeFailure)?;

    Ok(ConfidenceSequenceMethodBenchmark {
        method: ConfidenceSequenceMethod::FixedStake,
        snapshots,
        first_decision_sample_count,
        first_decision,
        median_elapsed_nanoseconds: elapsed,
        timing_repetitions: repetitions,
        retained_observation_count: observations.len(),
        candidate_mean_grid_size: 0,
    })
}

fn run_prpl_eb(
    observations: &[f64],
    fixture_truth: f64,
    alpha: f64,
    checkpoints: &[usize],
    threshold: Option<f64>,
    repetitions: usize,
) -> Result<ConfidenceSequenceMethodBenchmark, ConfidenceSequenceBenchmarkError> {
    let started = Instant::now();
    let mut sequence = PrPlEbConfidenceSequence::new(alpha)?;
    let mut snapshots = Vec::with_capacity(checkpoints.len());
    let mut first_decision = None;
    let mut first_decision_sample_count = None;
    let mut checkpoint_index = 0;

    for (index, observation) in observations.iter().copied().enumerate() {
        sequence.observe(observation)?;
        let interval = sequence.confidence_sequence();
        validate_interval(interval.lower, interval.upper)?;
        record_decision(
            threshold,
            interval.lower,
            interval.upper,
            index + 1,
            &mut first_decision,
            &mut first_decision_sample_count,
        );
        if checkpoint_index < checkpoints.len() && checkpoints[checkpoint_index] == index + 1 {
            snapshots.push(snapshot(
                index + 1,
                interval.lower,
                interval.upper,
                fixture_truth,
            ));
            checkpoint_index += 1;
        }
    }
    let first_elapsed = started.elapsed();
    let elapsed = median_elapsed(first_elapsed, repetitions.saturating_sub(1), || {
        let mut sequence = PrPlEbConfidenceSequence::new(alpha)?;
        for observation in observations {
            sequence.observe(black_box(*observation))?;
            black_box(sequence.confidence_sequence().upper);
        }
        Ok::<(), PrPlEbError>(())
    })
    .map_err(|_| ConfidenceSequenceBenchmarkError::PrPlEbFailure)?;

    Ok(ConfidenceSequenceMethodBenchmark {
        method: ConfidenceSequenceMethod::PrPlEb,
        snapshots,
        first_decision_sample_count,
        first_decision,
        median_elapsed_nanoseconds: elapsed,
        timing_repetitions: repetitions,
        retained_observation_count: 0,
        candidate_mean_grid_size: 0,
    })
}

fn run_hedged(
    observations: &[f64],
    fixture_truth: f64,
    alpha: f64,
    grid_size: usize,
    parameter_grid: Vec<f64>,
    checkpoints: &[usize],
    threshold: Option<f64>,
    repetitions: usize,
) -> Result<ConfidenceSequenceMethodBenchmark, ConfidenceSequenceBenchmarkError> {
    let started = Instant::now();
    let mut sequence = HedgedCapitalConfidenceSequence::new(alpha, parameter_grid.clone())?;
    let mut snapshots = Vec::with_capacity(checkpoints.len());
    let mut first_decision = None;
    let mut first_decision_sample_count = None;
    let mut checkpoint_index = 0;

    for (index, observation) in observations.iter().copied().enumerate() {
        sequence.observe(observation)?;
        let interval = sequence.confidence_sequence();
        validate_interval(interval.lower, interval.upper)?;
        record_decision(
            threshold,
            interval.lower,
            interval.upper,
            index + 1,
            &mut first_decision,
            &mut first_decision_sample_count,
        );
        if checkpoint_index < checkpoints.len() && checkpoints[checkpoint_index] == index + 1 {
            snapshots.push(snapshot(
                index + 1,
                interval.lower,
                interval.upper,
                fixture_truth,
            ));
            checkpoint_index += 1;
        }
    }
    let first_elapsed = started.elapsed();
    let elapsed = median_elapsed(first_elapsed, repetitions.saturating_sub(1), || {
        let mut sequence = HedgedCapitalConfidenceSequence::new(alpha, parameter_grid.clone())?;
        for observation in observations {
            sequence.observe(black_box(*observation))?;
            black_box(sequence.confidence_sequence().upper);
        }
        Ok::<(), HedgedConfidenceSequenceError>(())
    })
    .map_err(|_| ConfidenceSequenceBenchmarkError::HedgedFailure)?;

    Ok(ConfidenceSequenceMethodBenchmark {
        method: ConfidenceSequenceMethod::Hedged,
        snapshots,
        first_decision_sample_count,
        first_decision,
        median_elapsed_nanoseconds: elapsed,
        timing_repetitions: repetitions,
        retained_observation_count: 0,
        candidate_mean_grid_size: grid_size,
    })
}

fn median_elapsed<E>(
    first: Duration,
    additional_repetitions: usize,
    mut run: impl FnMut() -> Result<(), E>,
) -> Result<u128, E> {
    let mut samples = Vec::with_capacity(additional_repetitions + 1);
    samples.push(first.as_nanos());
    for _ in 0..additional_repetitions {
        let started = Instant::now();
        run()?;
        samples.push(started.elapsed().as_nanos());
    }
    samples.sort_unstable();
    Ok(samples[samples.len() / 2])
}

fn snapshot(
    sample_count: usize,
    lower: f64,
    upper: f64,
    fixture_truth: f64,
) -> ConfidenceSequenceBenchmarkSnapshot {
    ConfidenceSequenceBenchmarkSnapshot {
        sample_count: sample_count as u64,
        lower,
        upper,
        width: upper - lower,
        contains_fixture_truth: lower <= fixture_truth && fixture_truth <= upper,
    }
}

fn validate_interval(
    lower: f64,
    upper: f64,
) -> Result<(), ConfidenceSequenceBenchmarkError> {
    if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || upper > 1.0 || lower > upper {
        Err(ConfidenceSequenceBenchmarkError::InvalidConfidenceInterval)
    } else {
        Ok(())
    }
}

fn record_decision(
    threshold: Option<f64>,
    lower: f64,
    upper: f64,
    sample_count: usize,
    first_decision: &mut Option<ConfidenceThresholdDecision>,
    first_decision_sample_count: &mut Option<u64>,
) {
    if first_decision.is_some() {
        return;
    }
    let Some(threshold) = threshold else {
        return;
    };
    let decision = if upper < threshold {
        Some(ConfidenceThresholdDecision::BelowThreshold)
    } else if lower >= threshold {
        Some(ConfidenceThresholdDecision::AtOrAboveThreshold)
    } else {
        None
    };
    if let Some(decision) = decision {
        *first_decision = Some(decision);
        *first_decision_sample_count = Some(sample_count as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameter_grid() -> Vec<f64> {
        HedgedCapitalConfidenceSequence::log_tailed_parameter_grid(
            1.0e-8, 0.02, 48, 48,
        )
        .unwrap()
    }

    #[test]
    fn all_methods_receive_identical_observation_path_and_checkpoints() {
        let observations = vec![0.25; 64];
        let report = benchmark_confidence_sequences(
            &observations,
            0.25,
            0.05,
            parameter_grid(),
            &[16, 32, 64],
            Some(0.5),
            2,
        )
        .unwrap();

        assert_eq!(report.observation_count, 64);
        assert_eq!(report.checkpoints, vec![16, 32, 64]);
        assert_eq!(report.methods.len(), 3);
        for method in &report.methods {
            assert_eq!(
                method
                    .snapshots
                    .iter()
                    .map(|snapshot| snapshot.sample_count)
                    .collect::<Vec<_>>(),
                vec![16, 32, 64]
            );
            assert!(method.snapshots.iter().all(|snapshot| {
                snapshot.contains_fixture_truth && snapshot.width >= 0.0
            }));
            assert_eq!(method.timing_repetitions, 2);
        }
    }

    #[test]
    fn fixed_stake_retains_observations_while_streaming_methods_do_not() {
        let observations = vec![0.1; 24];
        let report = benchmark_confidence_sequences(
            &observations,
            0.1,
            0.05,
            parameter_grid(),
            &[24],
            None,
            1,
        )
        .unwrap();

        assert_eq!(
            report.methods[0].method,
            ConfidenceSequenceMethod::FixedStake
        );
        assert_eq!(report.methods[0].retained_observation_count, 24);
        assert_eq!(report.methods[1].method, ConfidenceSequenceMethod::PrPlEb);
        assert_eq!(report.methods[1].retained_observation_count, 0);
        assert_eq!(report.methods[2].method, ConfidenceSequenceMethod::Hedged);
        assert_eq!(report.methods[2].retained_observation_count, 0);
        assert_eq!(
            report.methods[2].candidate_mean_grid_size,
            parameter_grid().len()
        );
    }

    #[test]
    fn decision_threshold_is_applied_identically_to_all_methods() {
        let observations = vec![0.05; 128];
        let report = benchmark_confidence_sequences(
            &observations,
            0.05,
            0.05,
            parameter_grid(),
            &[32, 64, 128],
            Some(0.2),
            1,
        )
        .unwrap();

        for method in &report.methods {
            if let Some(decision) = method.first_decision {
                assert_eq!(decision, ConfidenceThresholdDecision::BelowThreshold);
                assert!(method.first_decision_sample_count.unwrap() > 0);
                assert!(method.first_decision_sample_count.unwrap() <= 128);
            }
        }
    }

    #[test]
    fn no_method_can_decide_above_a_threshold_when_truth_is_below_it_on_a_covered_fixture() {
        let observations = vec![0.05; 96];
        let report = benchmark_confidence_sequences(
            &observations,
            0.05,
            0.05,
            parameter_grid(),
            &[96],
            Some(0.2),
            1,
        )
        .unwrap();

        for method in &report.methods {
            let final_snapshot = method.snapshots.last().unwrap();
            assert!(final_snapshot.contains_fixture_truth);
            assert_ne!(
                method.first_decision,
                Some(ConfidenceThresholdDecision::AtOrAboveThreshold)
            );
        }
    }

    #[test]
    fn intervals_are_nested_at_reported_checkpoints() {
        let observations = (0..128)
            .map(|index| if index % 7 == 0 { 0.8 } else { 0.1 })
            .collect::<Vec<_>>();
        let truth = observations.iter().sum::<f64>() / observations.len() as f64;
        let report = benchmark_confidence_sequences(
            &observations,
            truth,
            0.05,
            parameter_grid(),
            &[16, 32, 64, 128],
            None,
            1,
        )
        .unwrap();

        for method in &report.methods {
            for pair in method.snapshots.windows(2) {
                assert!(pair[1].lower >= pair[0].lower);
                assert!(pair[1].upper <= pair[0].upper);
            }
        }
    }

    #[test]
    fn invalid_observations_checkpoints_and_thresholds_fail_closed() {
        let grid = parameter_grid();
        assert_eq!(
            benchmark_confidence_sequences(&[], 0.2, 0.05, grid.clone(), &[1], None, 1),
            Err(ConfidenceSequenceBenchmarkError::EmptyObservationPath)
        );
        assert_eq!(
            benchmark_confidence_sequences(&[f64::NAN], 0.2, 0.05, grid.clone(), &[1], None, 1),
            Err(ConfidenceSequenceBenchmarkError::InvalidObservation)
        );
        assert_eq!(
            benchmark_confidence_sequences(&[0.2], 0.2, 0.05, grid.clone(), &[0], None, 1),
            Err(ConfidenceSequenceBenchmarkError::InvalidCheckpoints)
        );
        assert_eq!(
            benchmark_confidence_sequences(&[0.2], 0.2, 0.05, grid.clone(), &[1, 1], None, 1),
            Err(ConfidenceSequenceBenchmarkError::InvalidCheckpoints)
        );
        assert_eq!(
            benchmark_confidence_sequences(&[0.2], 0.2, 0.05, grid, &[1], Some(1.1), 1),
            Err(ConfidenceSequenceBenchmarkError::InvalidDecisionThreshold)
        );
    }

    #[test]
    fn zero_timing_repetitions_are_rejected() {
        assert_eq!(
            benchmark_confidence_sequences(
                &[0.2],
                0.2,
                0.05,
                parameter_grid(),
                &[1],
                None,
                0,
            ),
            Err(ConfidenceSequenceBenchmarkError::InvalidTimingRepetitions)
        );
    }
}
