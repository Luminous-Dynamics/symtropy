// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Matched-receipt benchmark for fixed-stake, PrPl-EB and Hedged adaptive IS.
//!
//! Every method consumes the same immutable AdaptiveImportanceDraw values and
//! independently validates event/measure/proposal/sample identities, sequence
//! order and the observed defensive-weight bound. All are queried after every
//! receipt, so decision timing and wall-clock include online interval updates.
//!
//! This harness compares the statistical adapters on an identical declared
//! receipt stream. It does not prove that the external sampler generated the
//! receipts from the declared q_t or that the global defensive mixture
//! dominates p over the continuous target domain.

use std::hint::black_box;
use std::time::Instant;

use crate::adaptive_importance_sampling::{
    AdaptiveImportanceDraw, AdaptiveImportanceSamplingConfidenceSequence,
    AdaptiveImportanceSamplingError, AdaptiveImportanceSamplingSummary,
};
use crate::hedged_capital_sequence::{
    AdaptiveImportanceSamplingHedgedSequence, HedgedCapitalConfidenceSequence,
};
use crate::predictable_empirical_bernstein::AdaptiveImportanceSamplingPrPlEbSequence;

/// Statistical adapter evaluated on the matched receipt stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdaptiveImportanceComparisonMethod {
    /// Fixed-stake mixture betting with one-sided inversion.
    FixedStake,
    /// Predictable plug-in empirical-Bernstein confidence sequence.
    PrPlEb,
    /// Hedged max-capital confidence sequence on a finite parameter grid.
    Hedged,
}

/// First threshold decision made by an adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveImportanceThresholdDecision {
    /// The upper confidence endpoint is strictly below the threshold.
    BelowThreshold,
    /// The lower confidence endpoint is at or above the threshold.
    AtOrAboveThreshold,
}

/// State-complexity profile of an adaptive importance-sampling adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveImportanceStateProfile {
    /// Exact sample-ID ledger plus all bounded observations.
    ObservationHistoryAndSampleIds,
    /// Constant-size numerical CS state plus an exact sample-ID ledger.
    ConstantNumericalStateAndSampleIds,
    /// O(G) grid/capital numerical state plus an exact sample-ID ledger.
    GridNumericalStateAndSampleIds,
}

/// Common summary at one predeclared receipt-count checkpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveImportanceComparisonSnapshot {
    /// Number of accepted receipts.
    pub sample_count: u64,
    /// Importance-sampling point estimate of target event probability.
    pub estimate: f64,
    /// Lower endpoint of the time-uniform confidence interval.
    pub lower: f64,
    /// Upper endpoint of the time-uniform confidence interval.
    pub upper: f64,
    /// Upper minus lower.
    pub width: f64,
    /// Whether the interval contains the independently supplied fixture truth.
    ///
    /// This is fixture-level containment, not a population coverage estimate.
    pub contains_fixture_truth: bool,
}

/// Result for one method on the common receipt stream.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveImportanceMethodComparison {
    /// Adapter identity.
    pub method: AdaptiveImportanceComparisonMethod,
    /// Confidence intervals at common receipt-count checkpoints.
    pub snapshots: Vec<AdaptiveImportanceComparisonSnapshot>,
    /// First sample count at which the threshold decision became determinate.
    pub first_decision_sample_count: Option<u64>,
    /// First determinate threshold decision, if any.
    pub first_decision: Option<AdaptiveImportanceThresholdDecision>,
    /// Median complete-stream update-plus-summary time over repeated replays.
    pub median_elapsed_nanoseconds: u128,
    /// Number of identical full-stream timing replays.
    pub timing_repetitions: usize,
    /// Every accepted receipt retains one exact sample identity for duplicate rejection.
    pub retained_sample_identity_count: usize,
    /// Bounded-observation history retained by the numerical method.
    pub retained_observation_count: usize,
    /// Candidate-mean grid size; zero for methods without a grid.
    pub candidate_mean_grid_size: usize,
    /// Complexity profile of numerical state and receipt identity ledger.
    pub state_profile: AdaptiveImportanceStateProfile,
}

/// Reproducible comparison of all three AIS confidence-sequence adapters.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveImportanceComparisonReport {
    /// Event identity used by every receipt.
    pub event_digest: [u8; 32],
    /// Target-measure identity used by every receipt.
    pub target_measure_digest: [u8; 32],
    /// Fixed defensive-component identity.
    pub defensive_component_digest: [u8; 32],
    /// Defensive mixture weight shared by every receipt and adapter.
    pub defensive_mixture_weight: f64,
    /// Overall confidence error probability.
    pub confidence_alpha: f64,
    /// Independently known event probability for the synthetic fixture.
    pub fixture_truth: f64,
    /// Optional event-probability threshold used for first-decision timing.
    pub decision_threshold: Option<f64>,
    /// Number of common receipts.
    pub receipt_count: usize,
    /// Common receipt-count checkpoints.
    pub checkpoints: Vec<usize>,
    /// Methods, in deterministic fixed-stake / PrPl-EB / Hedged order.
    pub methods: Vec<AdaptiveImportanceMethodComparison>,
}

/// Immutable parameters shared by every adapter in a matched receipt comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveImportanceComparisonConfig {
    /// Event identity used by every receipt.
    pub event_digest: [u8; 32],
    /// Target-measure identity used by every receipt.
    pub target_measure_digest: [u8; 32],
    /// Fixed defensive-component identity.
    pub defensive_component_digest: [u8; 32],
    /// Defensive mixture weight shared by every receipt and adapter.
    pub defensive_mixture_weight: f64,
    /// Overall confidence error probability.
    pub confidence_alpha: f64,
    /// Independently known event probability for the synthetic fixture.
    pub fixture_truth: f64,
    /// Caller-configured Hedged candidate-mean grid.
    pub parameter_grid: Vec<f64>,
    /// Common receipt-count checkpoints.
    pub checkpoints: Vec<usize>,
    /// Optional event-probability threshold used for first-decision timing.
    pub decision_threshold: Option<f64>,
    /// Number of identical full-stream timing replays.
    pub timing_repetitions: usize,
}

/// Validation or adapter failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveImportanceComparisonError {
    /// Receipt stream must contain at least one draw.
    EmptyReceiptStream,
    /// Fixture truth must be finite and lie in [0,1].
    InvalidFixtureTruth,
    /// Decision threshold must be finite and lie in [0,1].
    InvalidDecisionThreshold,
    /// Confidence alpha must lie strictly in (0,1).
    InvalidConfidenceAlpha,
    /// Defensive mixture weight must lie in (0,1].
    InvalidDefensiveMixtureWeight,
    /// A required event/measure/defensive identity is all zero.
    InvalidIdentity,
    /// A receipt has an event, target or defensive-component identity different from the portfolio.
    ReceiptScopeMismatch,
    /// Checkpoints must be nonempty, strictly increasing, positive and in range.
    InvalidCheckpoints,
    /// Timing repetition count must be nonzero.
    InvalidTimingRepetitions,
    /// The caller-supplied Hedged candidate grid is invalid.
    InvalidHedgedGrid,
    /// One adapter rejected a receipt or failed during summary calculation.
    AdapterFailure,
    /// An adapter returned a non-finite, reversed or out-of-domain interval.
    InvalidConfidenceInterval,
    /// Checked sample count or timing arithmetic overflowed.
    CounterOverflow,
}

impl core::fmt::Display for AdaptiveImportanceComparisonError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::EmptyReceiptStream => "adaptive importance receipt stream is empty",
            Self::InvalidFixtureTruth => "fixture truth must lie in [0, 1]",
            Self::InvalidDecisionThreshold => "decision threshold must lie in [0, 1]",
            Self::InvalidConfidenceAlpha => {
                "confidence alpha must lie strictly between zero and one"
            }
            Self::InvalidDefensiveMixtureWeight => "defensive mixture weight must lie in (0, 1]",
            Self::InvalidIdentity => "a required comparison identity digest is invalid",
            Self::ReceiptScopeMismatch => "receipt stream mixes event or target-measure identities",
            Self::InvalidCheckpoints => "receipt checkpoints are invalid",
            Self::InvalidTimingRepetitions => "timing repetition count must be nonzero",
            Self::InvalidHedgedGrid => "Hedged parameter grid is invalid",
            Self::AdapterFailure => "an adaptive importance confidence-sequence adapter failed",
            Self::InvalidConfidenceInterval => "adapter returned an invalid confidence interval",
            Self::CounterOverflow => "comparison count or timing arithmetic overflowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for AdaptiveImportanceComparisonError {}

/// Compare all three adaptive-IS confidence-sequence adapters on identical receipts.
///
/// Receipts are copied unchanged to each adapter. The same fixed event,
/// target-measure and defensive-component digests, mixture weight, alpha and
/// checkpoints are used for every method. Each adapter updates and is queried
/// after every receipt, while snapshots and first-decision information are
/// captured outside the timed full-stream replays.
///
/// Timing includes construction of the adapter plus per-draw validation,
/// update and summary calculation. It excludes receipt generation, proposal
/// fitting, orbital propagation and reporting. Benchmark results are local
/// to the compiler, CPU, operating system and runtime environment.
pub fn compare_adaptive_importance_sequences(
    receipts: &[AdaptiveImportanceDraw],
    config: AdaptiveImportanceComparisonConfig,
) -> Result<AdaptiveImportanceComparisonReport, AdaptiveImportanceComparisonError> {
    if receipts.is_empty() {
        return Err(AdaptiveImportanceComparisonError::EmptyReceiptStream);
    }
    if !config.fixture_truth.is_finite() || !(0.0..=1.0).contains(&config.fixture_truth) {
        return Err(AdaptiveImportanceComparisonError::InvalidFixtureTruth);
    }
    if !config.confidence_alpha.is_finite()
        || config.confidence_alpha <= 0.0
        || config.confidence_alpha >= 1.0
    {
        return Err(AdaptiveImportanceComparisonError::InvalidConfidenceAlpha);
    }
    if !config.defensive_mixture_weight.is_finite()
        || config.defensive_mixture_weight <= 0.0
        || config.defensive_mixture_weight > 1.0
        || !config.defensive_mixture_weight.recip().is_finite()
    {
        return Err(AdaptiveImportanceComparisonError::InvalidDefensiveMixtureWeight);
    }
    if is_zero_digest(&config.event_digest)
        || is_zero_digest(&config.target_measure_digest)
        || is_zero_digest(&config.defensive_component_digest)
    {
        return Err(AdaptiveImportanceComparisonError::InvalidIdentity);
    }
    if config.checkpoints.is_empty()
        || config.checkpoints[0] == 0
        || config.checkpoints.last().copied().unwrap_or(0) > receipts.len()
        || config.checkpoints.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(AdaptiveImportanceComparisonError::InvalidCheckpoints);
    }
    if config.timing_repetitions == 0 {
        return Err(AdaptiveImportanceComparisonError::InvalidTimingRepetitions);
    }
    if config
        .decision_threshold
        .is_some_and(|threshold| !threshold.is_finite() || !(0.0..=1.0).contains(&threshold))
    {
        return Err(AdaptiveImportanceComparisonError::InvalidDecisionThreshold);
    }
    if receipts.iter().any(|draw| {
        draw.event_digest != config.event_digest
            || draw.target_measure_digest != config.target_measure_digest
            || draw.defensive_component_digest != config.defensive_component_digest
            || draw.defensive_mixture_weight.to_bits() != config.defensive_mixture_weight.to_bits()
    }) {
        return Err(AdaptiveImportanceComparisonError::ReceiptScopeMismatch);
    }

    // Validate the caller-selected grid before benchmarking any adapter.
    HedgedCapitalConfidenceSequence::new(config.confidence_alpha, config.parameter_grid.clone())
        .map_err(|_| AdaptiveImportanceComparisonError::InvalidHedgedGrid)?;

    let fixed = run_fixed_stake(receipts, &config)?;
    let prpl = run_prpl_eb(receipts, &config)?;
    let hedged = run_hedged(receipts, &config)?;

    Ok(AdaptiveImportanceComparisonReport {
        event_digest: config.event_digest,
        target_measure_digest: config.target_measure_digest,
        defensive_component_digest: config.defensive_component_digest,
        defensive_mixture_weight: config.defensive_mixture_weight,
        confidence_alpha: config.confidence_alpha,
        fixture_truth: config.fixture_truth,
        decision_threshold: config.decision_threshold,
        receipt_count: receipts.len(),
        checkpoints: config.checkpoints,
        methods: vec![fixed, prpl, hedged],
    })
}

fn run_fixed_stake(
    receipts: &[AdaptiveImportanceDraw],
    config: &AdaptiveImportanceComparisonConfig,
) -> Result<AdaptiveImportanceMethodComparison, AdaptiveImportanceComparisonError> {
    let mut sequence = AdaptiveImportanceSamplingConfidenceSequence::new(
        config.event_digest,
        config.target_measure_digest,
        config.defensive_component_digest,
        config.defensive_mixture_weight,
        config.confidence_alpha,
    )
    .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
    let mut snapshots = Vec::with_capacity(config.checkpoints.len());
    let mut first_decision = None;
    let mut first_decision_sample_count = None;
    let mut checkpoint_index = 0;

    for (index, receipt) in receipts.iter().copied().enumerate() {
        sequence
            .observe(receipt)
            .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
        let summary = sequence
            .summary()
            .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
        validate_summary(summary)?;
        record_decision(
            config.decision_threshold,
            summary.lower,
            summary.upper,
            index + 1,
            &mut first_decision,
            &mut first_decision_sample_count,
        );
        record_snapshot(
            index + 1,
            summary,
            config.fixture_truth,
            &config.checkpoints,
            &mut checkpoint_index,
            &mut snapshots,
        );
    }

    let elapsed = median_elapsed(config.timing_repetitions, || {
        let mut adapter = AdaptiveImportanceSamplingConfidenceSequence::new(
            config.event_digest,
            config.target_measure_digest,
            config.defensive_component_digest,
            config.defensive_mixture_weight,
            config.confidence_alpha,
        )
        .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
        for receipt in receipts {
            adapter
                .observe(black_box(*receipt))
                .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
            black_box(
                adapter
                    .summary()
                    .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?,
            );
        }
        Ok::<(), AdaptiveImportanceComparisonError>(())
    })?;

    Ok(AdaptiveImportanceMethodComparison {
        method: AdaptiveImportanceComparisonMethod::FixedStake,
        snapshots,
        first_decision_sample_count,
        first_decision,
        median_elapsed_nanoseconds: elapsed,
        timing_repetitions: config.timing_repetitions,
        retained_sample_identity_count: receipts.len(),
        retained_observation_count: receipts.len(),
        candidate_mean_grid_size: 0,
        state_profile: AdaptiveImportanceStateProfile::ObservationHistoryAndSampleIds,
    })
}

fn run_prpl_eb(
    receipts: &[AdaptiveImportanceDraw],
    config: &AdaptiveImportanceComparisonConfig,
) -> Result<AdaptiveImportanceMethodComparison, AdaptiveImportanceComparisonError> {
    let mut sequence = AdaptiveImportanceSamplingPrPlEbSequence::new(
        config.event_digest,
        config.target_measure_digest,
        config.defensive_component_digest,
        config.defensive_mixture_weight,
        config.confidence_alpha,
    )
    .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
    let mut snapshots = Vec::with_capacity(config.checkpoints.len());
    let mut first_decision = None;
    let mut first_decision_sample_count = None;
    let mut checkpoint_index = 0;

    for (index, receipt) in receipts.iter().copied().enumerate() {
        sequence
            .observe(receipt)
            .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
        let summary = sequence
            .summary()
            .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
        validate_summary(summary)?;
        record_decision(
            config.decision_threshold,
            summary.lower,
            summary.upper,
            index + 1,
            &mut first_decision,
            &mut first_decision_sample_count,
        );
        record_snapshot(
            index + 1,
            summary,
            config.fixture_truth,
            &config.checkpoints,
            &mut checkpoint_index,
            &mut snapshots,
        );
    }

    let elapsed = median_elapsed(config.timing_repetitions, || {
        let mut adapter = AdaptiveImportanceSamplingPrPlEbSequence::new(
            config.event_digest,
            config.target_measure_digest,
            config.defensive_component_digest,
            config.defensive_mixture_weight,
            config.confidence_alpha,
        )
        .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
        for receipt in receipts {
            adapter
                .observe(black_box(*receipt))
                .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
            black_box(
                adapter
                    .summary()
                    .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?,
            );
        }
        Ok::<(), AdaptiveImportanceComparisonError>(())
    })?;

    Ok(AdaptiveImportanceMethodComparison {
        method: AdaptiveImportanceComparisonMethod::PrPlEb,
        snapshots,
        first_decision_sample_count,
        first_decision,
        median_elapsed_nanoseconds: elapsed,
        timing_repetitions: config.timing_repetitions,
        retained_sample_identity_count: receipts.len(),
        retained_observation_count: 0,
        candidate_mean_grid_size: 0,
        state_profile: AdaptiveImportanceStateProfile::ConstantNumericalStateAndSampleIds,
    })
}

fn run_hedged(
    receipts: &[AdaptiveImportanceDraw],
    config: &AdaptiveImportanceComparisonConfig,
) -> Result<AdaptiveImportanceMethodComparison, AdaptiveImportanceComparisonError> {
    let grid_size = config.parameter_grid.len();
    let mut sequence = AdaptiveImportanceSamplingHedgedSequence::new(
        config.event_digest,
        config.target_measure_digest,
        config.defensive_component_digest,
        config.defensive_mixture_weight,
        config.confidence_alpha,
        config.parameter_grid.clone(),
    )
    .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
    let mut snapshots = Vec::with_capacity(config.checkpoints.len());
    let mut first_decision = None;
    let mut first_decision_sample_count = None;
    let mut checkpoint_index = 0;

    for (index, receipt) in receipts.iter().copied().enumerate() {
        sequence
            .observe(receipt)
            .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
        let summary = sequence
            .summary()
            .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
        validate_summary(summary)?;
        record_decision(
            config.decision_threshold,
            summary.lower,
            summary.upper,
            index + 1,
            &mut first_decision,
            &mut first_decision_sample_count,
        );
        record_snapshot(
            index + 1,
            summary,
            config.fixture_truth,
            &config.checkpoints,
            &mut checkpoint_index,
            &mut snapshots,
        );
    }

    let elapsed = median_elapsed(config.timing_repetitions, || {
        let mut adapter = AdaptiveImportanceSamplingHedgedSequence::new(
            config.event_digest,
            config.target_measure_digest,
            config.defensive_component_digest,
            config.defensive_mixture_weight,
            config.confidence_alpha,
            config.parameter_grid.clone(),
        )
        .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
        for receipt in receipts {
            adapter
                .observe(black_box(*receipt))
                .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?;
            black_box(
                adapter
                    .summary()
                    .map_err(|_| AdaptiveImportanceComparisonError::AdapterFailure)?,
            );
        }
        Ok::<(), AdaptiveImportanceComparisonError>(())
    })?;

    Ok(AdaptiveImportanceMethodComparison {
        method: AdaptiveImportanceComparisonMethod::Hedged,
        snapshots,
        first_decision_sample_count,
        first_decision,
        median_elapsed_nanoseconds: elapsed,
        timing_repetitions: config.timing_repetitions,
        retained_sample_identity_count: receipts.len(),
        retained_observation_count: 0,
        candidate_mean_grid_size: grid_size,
        state_profile: AdaptiveImportanceStateProfile::GridNumericalStateAndSampleIds,
    })
}

fn record_snapshot(
    sample_count: usize,
    summary: AdaptiveImportanceSamplingSummary,
    fixture_truth: f64,
    checkpoints: &[usize],
    checkpoint_index: &mut usize,
    snapshots: &mut Vec<AdaptiveImportanceComparisonSnapshot>,
) {
    if *checkpoint_index < checkpoints.len() && checkpoints[*checkpoint_index] == sample_count {
        snapshots.push(AdaptiveImportanceComparisonSnapshot {
            sample_count: sample_count as u64,
            estimate: summary.target_probability_estimate,
            lower: summary.lower,
            upper: summary.upper,
            width: summary.upper - summary.lower,
            contains_fixture_truth: summary.lower <= fixture_truth
                && fixture_truth <= summary.upper,
        });
        *checkpoint_index += 1;
    }
}

fn record_decision(
    threshold: Option<f64>,
    lower: f64,
    upper: f64,
    sample_count: usize,
    first_decision: &mut Option<AdaptiveImportanceThresholdDecision>,
    first_decision_sample_count: &mut Option<u64>,
) {
    if first_decision.is_some() {
        return;
    }
    let Some(threshold) = threshold else {
        return;
    };
    let decision = if upper < threshold {
        Some(AdaptiveImportanceThresholdDecision::BelowThreshold)
    } else if lower >= threshold {
        Some(AdaptiveImportanceThresholdDecision::AtOrAboveThreshold)
    } else {
        None
    };
    if let Some(decision) = decision {
        *first_decision = Some(decision);
        *first_decision_sample_count = Some(sample_count as u64);
    }
}

fn validate_summary(
    summary: AdaptiveImportanceSamplingSummary,
) -> Result<(), AdaptiveImportanceComparisonError> {
    if !summary.target_probability_estimate.is_finite()
        || !summary.lower.is_finite()
        || !summary.upper.is_finite()
        || summary.lower < 0.0
        || summary.upper > 1.0
        || summary.lower > summary.upper
    {
        return Err(AdaptiveImportanceComparisonError::InvalidConfidenceInterval);
    }
    Ok(())
}

fn median_elapsed(
    repetitions: usize,
    mut run: impl FnMut() -> Result<(), AdaptiveImportanceComparisonError>,
) -> Result<u128, AdaptiveImportanceComparisonError> {
    let mut durations = Vec::with_capacity(repetitions);
    for _ in 0..repetitions {
        let started = Instant::now();
        run()?;
        durations.push(started.elapsed().as_nanos());
    }
    durations.sort_unstable();
    durations
        .get(durations.len() / 2)
        .copied()
        .ok_or(AdaptiveImportanceComparisonError::CounterOverflow)
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENT: [u8; 32] = [1; 32];
    const TARGET: [u8; 32] = [2; 32];
    const DEFENSIVE: [u8; 32] = [3; 32];
    const MIXTURE_WEIGHT: f64 = 0.1;
    const TARGET_EVENT_PROBABILITY: f64 = 0.2;

    fn hedged_grid() -> Vec<f64> {
        HedgedCapitalConfidenceSequence::log_tailed_parameter_grid(1.0e-8, 0.02, 48, 48).unwrap()
    }

    fn config(
        checkpoints: &[usize],
        decision_threshold: Option<f64>,
        timing_repetitions: usize,
    ) -> AdaptiveImportanceComparisonConfig {
        AdaptiveImportanceComparisonConfig {
            event_digest: EVENT,
            target_measure_digest: TARGET,
            defensive_component_digest: DEFENSIVE,
            defensive_mixture_weight: MIXTURE_WEIGHT,
            confidence_alpha: 0.05,
            fixture_truth: TARGET_EVENT_PROBABILITY,
            parameter_grid: hedged_grid(),
            checkpoints: checkpoints.to_vec(),
            decision_threshold,
            timing_repetitions,
        }
    }

    fn adaptive_receipts(horizon: usize, encoded_path: u32) -> (Vec<AdaptiveImportanceDraw>, f64) {
        let mut receipts = Vec::with_capacity(horizon);
        let mut path_probability = 1.0;
        let mut previous_event = false;
        for time in 0..horizon {
            let residual_event_probability = if previous_event { 0.8 } else { 0.05 };
            let proposal_event_probability = MIXTURE_WEIGHT * TARGET_EVENT_PROBABILITY
                + (1.0 - MIXTURE_WEIGHT) * residual_event_probability;
            let bit_index = (time % u32::BITS as usize) as u32;
            let event_occurred = ((encoded_path >> bit_index) & 1) == 1;
            path_probability *= if event_occurred {
                proposal_event_probability
            } else {
                1.0 - proposal_event_probability
            };
            let (target_density, proposal_density) = if event_occurred {
                (TARGET_EVENT_PROBABILITY, proposal_event_probability)
            } else {
                (
                    1.0 - TARGET_EVENT_PROBABILITY,
                    1.0 - proposal_event_probability,
                )
            };
            receipts.push(AdaptiveImportanceDraw {
                sequence_index: time as u64,
                sample_id: [time as u8 + 1; 32],
                event_digest: EVENT,
                target_measure_digest: TARGET,
                proposal_digest: if previous_event { [51; 32] } else { [52; 32] },
                defensive_component_digest: DEFENSIVE,
                defensive_mixture_weight: MIXTURE_WEIGHT,
                log_target_density: target_density.ln(),
                log_proposal_density: proposal_density.ln(),
                event_occurred,
            });
            previous_event = event_occurred;
        }
        (receipts, path_probability)
    }

    #[test]
    fn all_adapters_receive_identical_receipts_and_checkpoints() {
        let (receipts, _) = adaptive_receipts(48, 0b0010_1101_0011_1001);
        let report =
            compare_adaptive_importance_sequences(&receipts, config(&[12, 24, 48], Some(0.5), 2))
                .unwrap();

        assert_eq!(report.receipt_count, 48);
        assert_eq!(report.checkpoints, vec![12, 24, 48]);
        assert_eq!(report.methods.len(), 3);
        for method in &report.methods {
            assert_eq!(
                method
                    .snapshots
                    .iter()
                    .map(|s| s.sample_count)
                    .collect::<Vec<_>>(),
                vec![12, 24, 48]
            );
            assert_eq!(method.retained_sample_identity_count, 48);
            assert!(method.snapshots.iter().all(|s| {
                s.estimate.is_finite()
                    && s.lower.is_finite()
                    && s.upper.is_finite()
                    && s.width >= 0.0
                    && s.lower <= s.upper
            }));
            assert_eq!(method.timing_repetitions, 2);
        }
    }

    #[test]
    fn adapters_share_the_same_decision_rule_and_receipt_stream_scope() {
        let (receipts, _) = adaptive_receipts(96, 0b1010_0101_1001_1100);
        let report =
            compare_adaptive_importance_sequences(&receipts, config(&[24, 48, 96], Some(0.1), 1))
                .unwrap();

        for method in &report.methods {
            if let Some(decision) = method.first_decision {
                assert!(matches!(
                    decision,
                    AdaptiveImportanceThresholdDecision::BelowThreshold
                        | AdaptiveImportanceThresholdDecision::AtOrAboveThreshold
                ));
                assert!(method.first_decision_sample_count.unwrap() > 0);
                assert!(method.first_decision_sample_count.unwrap() <= 96);
            }
        }
    }

    #[test]
    fn state_profiles_make_identity_and_numerical_storage_distinct() {
        let (receipts, _) = adaptive_receipts(16, 0b0101_0110_1100_0011);
        let report =
            compare_adaptive_importance_sequences(&receipts, config(&[16], None, 1)).unwrap();

        assert_eq!(
            report.methods[0].state_profile,
            AdaptiveImportanceStateProfile::ObservationHistoryAndSampleIds
        );
        assert_eq!(
            report.methods[1].state_profile,
            AdaptiveImportanceStateProfile::ConstantNumericalStateAndSampleIds
        );
        assert_eq!(
            report.methods[2].state_profile,
            AdaptiveImportanceStateProfile::GridNumericalStateAndSampleIds
        );
        assert_eq!(report.methods[0].retained_observation_count, 16);
        assert_eq!(report.methods[1].retained_observation_count, 0);
        assert_eq!(report.methods[2].retained_observation_count, 0);
        assert_eq!(
            report.methods[2].candidate_mean_grid_size,
            hedged_grid().len()
        );
    }

    #[test]
    fn exhaustive_adaptive_receipt_paths_cover_all_three_adapters_at_every_time() {
        const HORIZON: u32 = 6;
        let checkpoints = (1..=HORIZON as usize).collect::<Vec<_>>();
        let mut miss_probability = [0.0_f64; 3];

        for encoded_path in 0_u32..(1_u32 << HORIZON) {
            let (receipts, path_probability) = adaptive_receipts(HORIZON as usize, encoded_path);
            let report =
                compare_adaptive_importance_sequences(&receipts, config(&checkpoints, None, 1))
                    .unwrap();

            for (index, method) in report.methods.iter().enumerate() {
                if method
                    .snapshots
                    .iter()
                    .any(|snapshot| !snapshot.contains_fixture_truth)
                {
                    miss_probability[index] += path_probability;
                }
            }
        }

        for (method, probability) in [
            (
                AdaptiveImportanceComparisonMethod::FixedStake,
                miss_probability[0],
            ),
            (
                AdaptiveImportanceComparisonMethod::PrPlEb,
                miss_probability[1],
            ),
            (
                AdaptiveImportanceComparisonMethod::Hedged,
                miss_probability[2],
            ),
        ] {
            assert!(
                probability <= 0.05 + 1.0e-10,
                "{method:?} receipt-path miss probability was {probability}"
            );
        }
    }

    #[test]
    fn threshold_decision_uses_strict_upper_and_inclusive_lower_endpoints() {
        let mut decision = None;
        let mut sample_count = None;

        record_decision(Some(0.1), 0.1, 0.2, 3, &mut decision, &mut sample_count);
        assert_eq!(
            decision,
            Some(AdaptiveImportanceThresholdDecision::AtOrAboveThreshold)
        );
        assert_eq!(sample_count, Some(3));

        decision = None;
        sample_count = None;
        record_decision(Some(0.1), 0.0, 0.1, 4, &mut decision, &mut sample_count);
        assert_eq!(decision, None);
        assert_eq!(sample_count, None);

        record_decision(Some(0.1), 0.0, 0.099, 5, &mut decision, &mut sample_count);
        assert_eq!(
            decision,
            Some(AdaptiveImportanceThresholdDecision::BelowThreshold)
        );
        assert_eq!(sample_count, Some(5));
    }

    #[test]
    fn invalid_receipt_scopes_checkpoints_and_timings_fail_closed() {
        let (receipts, _) = adaptive_receipts(4, 0b1010);
        let mut mismatched_scope = config(&[4], None, 1);
        mismatched_scope.event_digest = [99; 32];
        assert_eq!(
            compare_adaptive_importance_sequences(&receipts, mismatched_scope),
            Err(AdaptiveImportanceComparisonError::ReceiptScopeMismatch)
        );

        let mut duplicate = receipts.clone();
        duplicate[1].sample_id = duplicate[0].sample_id;
        assert_eq!(
            compare_adaptive_importance_sequences(&duplicate, config(&[4], None, 1)),
            Err(AdaptiveImportanceComparisonError::AdapterFailure)
        );

        assert_eq!(
            compare_adaptive_importance_sequences(&receipts, config(&[0], None, 1)),
            Err(AdaptiveImportanceComparisonError::InvalidCheckpoints)
        );
        assert_eq!(
            compare_adaptive_importance_sequences(&receipts, config(&[4], None, 0)),
            Err(AdaptiveImportanceComparisonError::InvalidTimingRepetitions)
        );
    }

    #[test]
    fn exhaustive_adaptive_paths_preserve_coverage_at_decision_triggered_stopping_times() {
        const HORIZON: u32 = 6;
        const POLICY_COUNT: usize = 6;
        const THRESHOLD: f64 = 0.5;
        const WIDTH_TARGET: f64 = 0.9;
        let checkpoints = (1..=HORIZON as usize).collect::<Vec<_>>();
        let mut miss_probability = [[0.0_f64; POLICY_COUNT]; 3];

        for encoded_path in 0_u32..(1_u32 << HORIZON) {
            let (receipts, path_probability) = adaptive_receipts(HORIZON as usize, encoded_path);
            let report =
                compare_adaptive_importance_sequences(&receipts, config(&checkpoints, Some(THRESHOLD), 1))
                    .unwrap();

            for (method_index, method) in report.methods.iter().enumerate() {
                let last_index = method.snapshots.len() - 1;
                let below_threshold_stop = method
                    .snapshots
                    .iter()
                    .position(|snapshot| snapshot.upper < THRESHOLD)
                    .unwrap_or(last_index);
                let above_threshold_stop = method
                    .snapshots
                    .iter()
                    .position(|snapshot| snapshot.lower >= THRESHOLD)
                    .unwrap_or(last_index);
                let width_stop = method
                    .snapshots
                    .iter()
                    .position(|snapshot| snapshot.width <= WIDTH_TARGET)
                    .unwrap_or(last_index);
                let two_draw_budget_stop = method
                    .snapshots
                    .iter()
                    .position(|snapshot| snapshot.sample_count >= 2)
                    .unwrap_or(last_index);
                let stop_indices = [
                    last_index,
                    0,
                    below_threshold_stop,
                    above_threshold_stop,
                    width_stop,
                    two_draw_budget_stop,
                ];

                for (policy_index, snapshot_index) in stop_indices.into_iter().enumerate() {
                    if !method.snapshots[snapshot_index].contains_fixture_truth {
                        miss_probability[method_index][policy_index] += path_probability;
                    }
                }
            }
        }

        for method_index in 0..3 {
            for policy_index in 0..POLICY_COUNT {
                assert!(
                    miss_probability[method_index][policy_index] <= 0.05 + 1.0e-10,
                    "method index {method_index}, stop-policy index {policy_index} had miss probability {}",
                    miss_probability[method_index][policy_index]
                );
            }
        }
    }

}
