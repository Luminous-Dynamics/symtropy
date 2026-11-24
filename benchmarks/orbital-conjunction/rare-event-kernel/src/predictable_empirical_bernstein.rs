// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Predictable plug-in empirical-Bernstein confidence sequence for bounded means.
//!
//! This implements the PrPl-EB confidence sequence of Waudby-Smith and Ramdas
//! (JRSS B, 2024, Theorem 2 / Equation 15). It assumes observations X_t in
//! [0,1] with a fixed conditional mean E[X_t | F_(t-1)] = mu. Independence is
//! not required. This conditional-mean contract is compatible with predictable
//! adaptive importance proposals when the complete density is used and the
//! normalized weighted event indicator has the required constant conditional
//! mean.
//!
//! The sequence uses predictable, variance-adaptive betting fractions and
//! intersects the current empirical-Bernstein interval with all previous
//! intervals. It maintains only constant-size streaming state.
//!
//! Reference: https://doi.org/10.1093/jrsssb/qkad009
//! https://arxiv.org/abs/2010.09686

use crate::adaptive_importance_sampling::{
    AdaptiveImportanceDraw, AdaptiveImportanceSamplingError, AdaptiveImportanceSamplingSummary,
    BoundedMeanConfidenceSequence,
};

/// Default PrPl-EB betting-fraction cap recommended by Waudby-Smith & Ramdas.
pub const DEFAULT_BET_FRACTION_CAP: f64 = 0.5;

/// Configuration, numerical or sequence errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrPlEbError {
    /// Confidence error probability must be finite and lie strictly in (0,1).
    InvalidAlpha,
    /// Betting-fraction cap must be finite and lie strictly in (0,1).
    InvalidBetFractionCap,
    /// Observation must be finite and lie in [0,1].
    InvalidObservation,
    /// Streaming observation counter overflow.
    CounterOverflow,
    /// Non-finite or empty numerical confidence intersection.
    NumericalFailure,
}

impl core::fmt::Display for PrPlEbError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidAlpha => "confidence alpha must lie strictly between zero and one",
            Self::InvalidBetFractionCap => {
                "betting-fraction cap must lie strictly between zero and one"
            }
            Self::InvalidObservation => "bounded-mean observation must lie in [0, 1]",
            Self::CounterOverflow => "bounded-mean observation counter overflowed",
            Self::NumericalFailure => {
                "PrPl-EB update or confidence intersection failed numerically"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PrPlEbError {}

/// Streaming predictable plug-in empirical-Bernstein confidence sequence.
///
/// The internal state is constant-size: each update is O(1) time and O(1)
/// memory. The reported interval is the running intersection of the current
/// PrPl-EB interval with every earlier interval, so it is nested over time.
///
/// Statistical validity assumes the observations have a fixed conditional
/// mean, even if their conditional distributions change predictably.
#[derive(Debug, Clone, PartialEq)]
pub struct PrPlEbConfidenceSequence {
    alpha: f64,
    bet_fraction_cap: f64,
    sample_count: u64,
    sum_observations: f64,
    sum_lambda: f64,
    sum_lambda_observations: f64,
    sum_predictable_variance_penalty: f64,
    predictive_mean: f64,
    predictive_variance: f64,
    lower: f64,
    upper: f64,
}

impl PrPlEbConfidenceSequence {
    /// Construct a PrPl-EB process with the paper's recommended default cap 0.5.
    pub fn new(alpha: f64) -> Result<Self, PrPlEbError> {
        Self::with_bet_fraction_cap(alpha, DEFAULT_BET_FRACTION_CAP)
    }

    /// Construct a PrPl-EB process with an explicitly selected cap c in (0,1).
    ///
    /// The paper recommends caps such as 0.5 or 0.75. The cap controls efficiency,
    /// not the validity contract, provided it remains strictly below one.
    pub fn with_bet_fraction_cap(alpha: f64, bet_fraction_cap: f64) -> Result<Self, PrPlEbError> {
        if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
            return Err(PrPlEbError::InvalidAlpha);
        }
        if !bet_fraction_cap.is_finite() || bet_fraction_cap <= 0.0 || bet_fraction_cap >= 1.0 {
            return Err(PrPlEbError::InvalidBetFractionCap);
        }

        Ok(Self {
            alpha,
            bet_fraction_cap,
            sample_count: 0,
            sum_observations: 0.0,
            sum_lambda: 0.0,
            sum_lambda_observations: 0.0,
            sum_predictable_variance_penalty: 0.0,
            predictive_mean: 0.5,
            predictive_variance: 0.25,
            lower: 0.0,
            upper: 1.0,
        })
    }

    /// Add one bounded observation, updating the anytime-valid interval.
    ///
    /// Validation and all arithmetic are completed before any state mutation.
    pub fn observe(&mut self, observation: f64) -> Result<(), PrPlEbError> {
        if !observation.is_finite() || !(0.0..=1.0).contains(&observation) {
            return Err(PrPlEbError::InvalidObservation);
        }

        let next_count = self
            .sample_count
            .checked_add(1)
            .ok_or(PrPlEbError::CounterOverflow)?;
        let time = next_count as f64;
        if !time.is_finite() || time <= 0.0 {
            return Err(PrPlEbError::NumericalFailure);
        }

        // Equation (15): lambda_t = min(sqrt(2 log(2/alpha) /
        // (sigma_hat_{t-1}^2 * t * log(1+t))), c).
        let log_budget = 2.0_f64.ln() - self.alpha.ln();
        let denominator = self.predictive_variance * time * time.ln_1p();
        let raw_lambda = (2.0 * log_budget / denominator).sqrt();
        let lambda = raw_lambda.min(self.bet_fraction_cap);
        if !lambda.is_finite() || lambda <= 0.0 || lambda >= 1.0 {
            return Err(PrPlEbError::NumericalFailure);
        }

        // Predictable plug-in empirical-Bernstein penalty:
        // (X_t - mu_hat_{t-1})^2 * (-log(1-lambda_t) - lambda_t).
        let centered_predictable = observation - self.predictive_mean;
        let penalty_factor = -(-lambda).ln_1p() - lambda;
        let penalty_increment = centered_predictable * centered_predictable * penalty_factor;

        let next_sum_observations = self.sum_observations + observation;
        let next_sum_lambda = self.sum_lambda + lambda;
        let next_sum_lambda_observations = self.sum_lambda_observations + lambda * observation;
        let next_penalty = self.sum_predictable_variance_penalty + penalty_increment;
        if !next_sum_observations.is_finite()
            || !next_sum_lambda.is_finite()
            || !next_sum_lambda_observations.is_finite()
            || !next_penalty.is_finite()
            || next_sum_lambda <= 0.0
            || penalty_increment < 0.0
        {
            return Err(PrPlEbError::NumericalFailure);
        }

        let center = next_sum_lambda_observations / next_sum_lambda;
        let radius = (log_budget + next_penalty) / next_sum_lambda;
        if !center.is_finite() || !radius.is_finite() || radius < 0.0 {
            return Err(PrPlEbError::NumericalFailure);
        }

        // Widen endpoints outwards by one representable value, then intersect
        // with the previous sequence member. This is a numerical guard rather
        // than a formal proof of all floating-point operations.
        let current_lower = next_down(center - radius).max(0.0);
        let current_upper = next_up(center + radius).min(1.0);
        let next_lower = self.lower.max(current_lower);
        let next_upper = self.upper.min(current_upper);
        if !next_lower.is_finite() || !next_upper.is_finite() || next_lower > next_upper {
            return Err(PrPlEbError::NumericalFailure);
        }

        // Equation (15): regularized predictable mean and variance estimates.
        let next_time_denominator = time + 1.0;
        let next_predictive_mean = (0.5 + next_sum_observations) / next_time_denominator;
        let next_residual = observation - next_predictive_mean;
        let previous_residual_sum =
            self.predictive_variance * (self.sample_count as f64 + 1.0) - 0.25;
        let next_predictive_variance =
            (0.25 + previous_residual_sum + next_residual * next_residual) / next_time_denominator;

        if !next_predictive_mean.is_finite()
            || !(0.0..=1.0).contains(&next_predictive_mean)
            || !next_predictive_variance.is_finite()
            || next_predictive_variance <= 0.0
        {
            return Err(PrPlEbError::NumericalFailure);
        }

        self.sample_count = next_count;
        self.sum_observations = next_sum_observations;
        self.sum_lambda = next_sum_lambda;
        self.sum_lambda_observations = next_sum_lambda_observations;
        self.sum_predictable_variance_penalty = next_penalty;
        self.predictive_mean = next_predictive_mean;
        self.predictive_variance = next_predictive_variance;
        self.lower = next_lower;
        self.upper = next_upper;
        Ok(())
    }

    /// Number of accepted observations.
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Current predictable regularized mean estimate.
    pub fn predictive_mean(&self) -> f64 {
        self.predictive_mean
    }

    /// Current predictable regularized variance estimate.
    pub fn predictive_variance(&self) -> f64 {
        self.predictive_variance
    }

    /// Current PrPl-EB betting fraction cap.
    pub fn bet_fraction_cap(&self) -> f64 {
        self.bet_fraction_cap
    }

    /// Return the current running-intersection confidence sequence.
    pub fn confidence_sequence(&self) -> BoundedMeanConfidenceSequence {
        let empirical_mean = if self.sample_count == 0 {
            0.0
        } else {
            self.sum_observations / self.sample_count as f64
        };
        BoundedMeanConfidenceSequence {
            sample_count: self.sample_count,
            empirical_mean,
            lower: self.lower,
            upper: self.upper,
            alpha: self.alpha,
        }
    }

    /// Return the current predictable variance residual sum.
    ///
    /// This diagnostic is exposed so an independent replay can reproduce the
    /// variance plug-in update without storing the complete observation stream.
    pub fn variance_residual_sum(&self) -> f64 {
        self.predictive_variance * (self.sample_count as f64 + 1.0) - 0.25
    }
}

/// Streaming PrPl-EB confidence sequence for predictable defensive importance sampling.
///
/// This adapter applies the same evidence-identity, sample-order and observed
/// defensive-weight checks as the fixed-stake adaptive-IS prototype, but updates
/// the O(1)-state PrPl-EB sequence. It does not prove the external sampler used
/// the declared proposal or that the declared defensive mixture dominates the
/// target distribution globally.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveImportanceSamplingPrPlEbSequence {
    event_digest: [u8; 32],
    target_measure_digest: [u8; 32],
    defensive_component_digest: [u8; 32],
    defensive_mixture_weight: f64,
    next_sequence_index: u64,
    sample_ids: std::collections::BTreeSet<[u8; 32]>,
    sequence: PrPlEbConfidenceSequence,
}

impl AdaptiveImportanceSamplingPrPlEbSequence {
    /// Bind a new PrPl-EB sequence to one event, target measure and defensive component.
    pub fn new(
        event_digest: [u8; 32],
        target_measure_digest: [u8; 32],
        defensive_component_digest: [u8; 32],
        defensive_mixture_weight: f64,
        confidence_alpha: f64,
    ) -> Result<Self, AdaptiveImportanceSamplingError> {
        Self::with_bet_fraction_cap(
            event_digest,
            target_measure_digest,
            defensive_component_digest,
            defensive_mixture_weight,
            confidence_alpha,
            DEFAULT_BET_FRACTION_CAP,
        )
    }

    /// Bind a new sequence with an explicit PrPl-EB betting-fraction cap.
    pub fn with_bet_fraction_cap(
        event_digest: [u8; 32],
        target_measure_digest: [u8; 32],
        defensive_component_digest: [u8; 32],
        defensive_mixture_weight: f64,
        confidence_alpha: f64,
        bet_fraction_cap: f64,
    ) -> Result<Self, AdaptiveImportanceSamplingError> {
        if is_zero_digest(&event_digest)
            || is_zero_digest(&target_measure_digest)
            || is_zero_digest(&defensive_component_digest)
        {
            return Err(AdaptiveImportanceSamplingError::InvalidDigest);
        }
        if !defensive_mixture_weight.is_finite()
            || defensive_mixture_weight <= 0.0
            || defensive_mixture_weight > 1.0
            || !defensive_mixture_weight.recip().is_finite()
        {
            return Err(AdaptiveImportanceSamplingError::InvalidProbability);
        }

        let sequence =
            PrPlEbConfidenceSequence::with_bet_fraction_cap(confidence_alpha, bet_fraction_cap)
                .map_err(|_| AdaptiveImportanceSamplingError::InvalidProbability)?;

        Ok(Self {
            event_digest,
            target_measure_digest,
            defensive_component_digest,
            defensive_mixture_weight,
            next_sequence_index: 0,
            sample_ids: std::collections::BTreeSet::new(),
            sequence,
        })
    }

    /// Accept one draw after validating its evidence identities and full-density weight.
    ///
    /// The update is transactional. The observed normalized contribution
    /// alpha_mix * 1_E(X) * p(X)/q_t(X) must be in [0,1]. The global defensive
    /// mixture condition and the fact that X was actually sampled from q_t
    /// remain external assumptions.
    pub fn observe(
        &mut self,
        draw: AdaptiveImportanceDraw,
    ) -> Result<(), AdaptiveImportanceSamplingError> {
        if draw.event_digest != self.event_digest {
            return Err(AdaptiveImportanceSamplingError::EventIdentityMismatch);
        }
        if draw.target_measure_digest != self.target_measure_digest {
            return Err(AdaptiveImportanceSamplingError::TargetMeasureMismatch);
        }
        if draw.defensive_component_digest != self.defensive_component_digest {
            return Err(AdaptiveImportanceSamplingError::DefensiveComponentMismatch);
        }
        if !draw.defensive_mixture_weight.is_finite()
            || draw.defensive_mixture_weight.to_bits() != self.defensive_mixture_weight.to_bits()
        {
            return Err(AdaptiveImportanceSamplingError::DefensiveMixtureWeightMismatch);
        }
        if is_zero_digest(&draw.sample_id) || is_zero_digest(&draw.proposal_digest) {
            return Err(AdaptiveImportanceSamplingError::InvalidDigest);
        }
        if draw.sequence_index != self.next_sequence_index {
            return Err(AdaptiveImportanceSamplingError::NonSequentialSampleIndex);
        }
        if self.sample_ids.contains(&draw.sample_id) {
            return Err(AdaptiveImportanceSamplingError::DuplicateSampleIdentity);
        }
        if !draw.log_proposal_density.is_finite() {
            return Err(AdaptiveImportanceSamplingError::InvalidProposalDensity);
        }
        if draw.log_target_density.is_nan() || draw.log_target_density == f64::INFINITY {
            return Err(AdaptiveImportanceSamplingError::InvalidTargetDensity);
        }

        let normalized_weight = if draw.log_target_density == f64::NEG_INFINITY {
            0.0
        } else {
            let log_normalized_weight = self.defensive_mixture_weight.ln()
                + draw.log_target_density
                - draw.log_proposal_density;
            if !log_normalized_weight.is_finite() && log_normalized_weight != f64::NEG_INFINITY {
                return Err(AdaptiveImportanceSamplingError::DefensiveWeightBoundViolation);
            }
            if log_normalized_weight > 0.0 {
                return Err(AdaptiveImportanceSamplingError::DefensiveWeightBoundViolation);
            }
            log_normalized_weight.exp()
        };

        if !normalized_weight.is_finite() || !(0.0..=1.0).contains(&normalized_weight) {
            return Err(AdaptiveImportanceSamplingError::DefensiveWeightBoundViolation);
        }
        let observation = if draw.event_occurred {
            normalized_weight
        } else {
            0.0
        };
        let next_sequence_index = self
            .next_sequence_index
            .checked_add(1)
            .ok_or(AdaptiveImportanceSamplingError::CounterOverflow)?;

        self.sequence
            .observe(observation)
            .map_err(|error| match error {
                PrPlEbError::CounterOverflow => AdaptiveImportanceSamplingError::CounterOverflow,
                PrPlEbError::InvalidObservation => {
                    AdaptiveImportanceSamplingError::DefensiveWeightBoundViolation
                }
                PrPlEbError::InvalidAlpha
                | PrPlEbError::InvalidBetFractionCap
                | PrPlEbError::NumericalFailure => {
                    AdaptiveImportanceSamplingError::NumericalFailure
                }
            })?;
        self.sample_ids.insert(draw.sample_id);
        self.next_sequence_index = next_sequence_index;
        Ok(())
    }

    /// Number of accepted adaptive importance-sampling draws.
    pub fn sample_count(&self) -> u64 {
        self.sequence.sample_count()
    }

    /// Return the importance-sampling estimate and time-uniform target-probability interval.
    pub fn summary(
        &self,
    ) -> Result<AdaptiveImportanceSamplingSummary, AdaptiveImportanceSamplingError> {
        let normalized = self.sequence.confidence_sequence();
        let scale = self.defensive_mixture_weight.recip();
        let target_probability_estimate = normalized.empirical_mean * scale;
        if !target_probability_estimate.is_finite() {
            return Err(AdaptiveImportanceSamplingError::NumericalFailure);
        }

        Ok(AdaptiveImportanceSamplingSummary {
            sample_count: normalized.sample_count,
            defensive_mixture_weight: self.defensive_mixture_weight,
            confidence_alpha: normalized.alpha,
            normalized_mean_estimate: normalized.empirical_mean,
            target_probability_estimate,
            lower: (normalized.lower * scale).clamp(0.0, 1.0),
            upper: (normalized.upper * scale).clamp(0.0, 1.0),
        })
    }
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENT_PROBABILITY: f64 = 0.2;
    const DEFENSIVE_WEIGHT: f64 = 0.1;
    const EVENT_ID: [u8; 32] = [1; 32];
    const TARGET_ID: [u8; 32] = [2; 32];
    const DEFENSIVE_ID: [u8; 32] = [3; 32];

    #[test]
    fn empty_prpl_eb_sequence_is_the_unit_interval() {
        let sequence = PrPlEbConfidenceSequence::new(0.05).unwrap();
        let interval = sequence.confidence_sequence();
        assert_eq!(interval.sample_count, 0);
        assert_eq!(interval.lower, 0.0);
        assert_eq!(interval.upper, 1.0);
    }

    #[test]
    fn constant_sequence_is_covered_and_contracts() {
        let mut sequence = PrPlEbConfidenceSequence::new(0.05).unwrap();
        for _ in 0..500 {
            sequence.observe(0.25).unwrap();
        }
        let interval = sequence.confidence_sequence();
        assert!(interval.lower <= 0.25);
        assert!(interval.upper >= 0.25);
        assert!(interval.upper - interval.lower < 0.1);
    }

    #[test]
    fn prpl_eb_confidence_sequence_is_nested_over_time() {
        let mut sequence = PrPlEbConfidenceSequence::new(0.05).unwrap();
        let mut previous = sequence.confidence_sequence();
        for observation in [0.9, 0.1, 0.8, 0.2, 0.7, 0.3, 0.6, 0.4] {
            sequence.observe(observation).unwrap();
            let current = sequence.confidence_sequence();
            assert!(current.lower >= previous.lower);
            assert!(current.upper <= previous.upper);
            assert!(current.lower <= current.upper);
            previous = current;
        }
    }

    #[test]
    fn all_zero_and_all_one_paths_preserve_boundary_coverage() {
        let mut zeros = PrPlEbConfidenceSequence::new(0.05).unwrap();
        let mut ones = PrPlEbConfidenceSequence::new(0.05).unwrap();
        for _ in 0..200 {
            zeros.observe(0.0).unwrap();
            ones.observe(1.0).unwrap();
        }
        let zero_interval = zeros.confidence_sequence();
        let one_interval = ones.confidence_sequence();
        assert_eq!(zero_interval.lower, 0.0);
        assert!(zero_interval.upper > 0.0 && zero_interval.upper < 0.1);
        assert!(one_interval.lower < 1.0 && one_interval.lower > 0.9);
        assert_eq!(one_interval.upper, 1.0);
    }

    #[test]
    fn invalid_observations_and_configuration_fail_closed() {
        let mut sequence = PrPlEbConfidenceSequence::new(0.05).unwrap();
        assert_eq!(
            sequence.observe(f64::NAN),
            Err(PrPlEbError::InvalidObservation)
        );
        assert_eq!(sequence.observe(1.01), Err(PrPlEbError::InvalidObservation));
        assert_eq!(sequence.sample_count(), 0);
        assert!(PrPlEbConfidenceSequence::new(0.0).is_err());
        assert!(PrPlEbConfidenceSequence::new(1.0).is_err());
        assert!(PrPlEbConfidenceSequence::with_bet_fraction_cap(0.05, 0.0).is_err());
        assert!(PrPlEbConfidenceSequence::with_bet_fraction_cap(0.05, 1.0).is_err());
    }

    #[test]
    fn exact_finite_support_adaptive_proposals_have_anytime_coverage() {
        // Target event probability 0.2, with q_t = a*p + (1-a)*r_t.
        // r_t(event)=0.8 after an event, else 0.05. The normalized weighted
        // event indicator has fixed conditional mean a*P(E)=0.02.
        const HORIZON: u32 = 6;
        let mut total_miss_probability = 0.0;

        for encoded_path in 0_u32..(1_u32 << HORIZON) {
            let mut sequence = AdaptiveImportanceSamplingPrPlEbSequence::new(
                EVENT_ID,
                TARGET_ID,
                DEFENSIVE_ID,
                DEFENSIVE_WEIGHT,
                0.05,
            )
            .unwrap();
            let mut path_probability = 1.0;
            let mut previous_event = false;
            let mut ever_missed = false;

            for time in 0..HORIZON {
                let residual_event_probability = if previous_event { 0.8 } else { 0.05 };
                let proposal_event_probability = DEFENSIVE_WEIGHT * EVENT_PROBABILITY
                    + (1.0 - DEFENSIVE_WEIGHT) * residual_event_probability;
                let event_occurred = ((encoded_path >> time) & 1) == 1;

                if event_occurred {
                    path_probability *= proposal_event_probability;
                } else {
                    path_probability *= 1.0 - proposal_event_probability;
                }

                let (target_density, proposal_density) = if event_occurred {
                    (EVENT_PROBABILITY, proposal_event_probability)
                } else {
                    (1.0 - EVENT_PROBABILITY, 1.0 - proposal_event_probability)
                };
                let draw = AdaptiveImportanceDraw {
                    sequence_index: time as u64,
                    sample_id: [time as u8 + 1; 32],
                    event_digest: EVENT_ID,
                    target_measure_digest: TARGET_ID,
                    proposal_digest: [time as u8 + 20; 32],
                    defensive_component_digest: DEFENSIVE_ID,
                    defensive_mixture_weight: DEFENSIVE_WEIGHT,
                    log_target_density: target_density.ln(),
                    log_proposal_density: proposal_density.ln(),
                    event_occurred,
                };
                sequence.observe(draw).unwrap();

                let interval = sequence.summary().unwrap();
                if interval.lower > EVENT_PROBABILITY || interval.upper < EVENT_PROBABILITY {
                    ever_missed = true;
                }
                previous_event = event_occurred;
            }

            if ever_missed {
                total_miss_probability += path_probability;
            }
        }

        assert!(
            total_miss_probability <= 0.05 + 1.0e-10,
            "exact adaptive-path miss probability was {total_miss_probability}"
        );
    }

    #[test]
    fn adaptive_prpl_eb_rejects_duplicate_samples_and_weight_bound_violations() {
        let mut sequence =
            AdaptiveImportanceSamplingPrPlEbSequence::new([1; 32], [2; 32], [3; 32], 0.1, 0.05)
                .unwrap();
        let valid = AdaptiveImportanceDraw {
            sequence_index: 0,
            sample_id: [4; 32],
            event_digest: [1; 32],
            target_measure_digest: [2; 32],
            proposal_digest: [5; 32],
            defensive_component_digest: [3; 32],
            defensive_mixture_weight: 0.1,
            log_target_density: 0.2_f64.ln(),
            log_proposal_density: 0.74_f64.ln(),
            event_occurred: true,
        };
        sequence.observe(valid).unwrap();
        let mut duplicate = valid;
        duplicate.sequence_index = 1;
        assert_eq!(
            sequence.observe(duplicate),
            Err(AdaptiveImportanceSamplingError::DuplicateSampleIdentity)
        );

        let mut overweight = valid;
        overweight.sequence_index = 1;
        overweight.sample_id = [6; 32];
        overweight.proposal_digest = [7; 32];
        overweight.log_proposal_density = 0.01_f64.ln();
        assert_eq!(
            sequence.observe(overweight),
            Err(AdaptiveImportanceSamplingError::DefensiveWeightBoundViolation)
        );
        assert_eq!(sequence.sample_count(), 1);
    }

    #[test]
    fn observed_mean_and_variance_diagnostics_are_consistent() {
        let mut sequence = PrPlEbConfidenceSequence::new(0.05).unwrap();
        let observations = [0.2, 0.3, 0.7, 0.9, 0.1];
        let mut sum = 0.0;
        let mut residual_sum = 0.0;
        for (index, observation) in observations.iter().copied().enumerate() {
            sequence.observe(observation).unwrap();
            sum += observation;
            let predictive_mean = (0.5 + sum) / (index as f64 + 2.0);
            residual_sum += (observation - predictive_mean).powi(2);
            assert!((sequence.predictive_mean() - predictive_mean).abs() < 1.0e-14);
            assert!((sequence.variance_residual_sum() - residual_sum).abs() < 1.0e-13);
        }
    }
}
