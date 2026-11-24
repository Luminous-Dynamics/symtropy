// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Anytime-valid bounded-mean betting sequences for adaptive importance sampling.
//!
//! A predictable defensive proposal q_t = a p + (1-a) r_t gives
//! 0 <= a * 1_E(X_t) * p(X_t) / q_t(X_t) <= 1 and conditional mean a * P_p(E),
//! provided q_t really is sampled from the declared mixture and the complete
//! mixture density is used in the likelihood ratio. The normalized weighted
//! event indicators therefore form a bounded sequence with a constant
//! conditional mean, even when r_t is selected predictably from past data.
//!
//! The bounded-mean confidence sequence below uses fixed-stake mixtures of
//! nonnegative betting supermartingales and running maxima. It is a transparent
//! baseline, not an implementation of the empirical-Bernstein or Bayes-assisted
//! procedures in the current literature. The importance-sampling adapter checks
//! per-draw identities and observed weight bounds; it cannot prove that the
//! sampler actually generated draws from the declared proposal or that the
//! defensive-mixture domination holds over the full continuous state space.
//!
//! The statistical guarantees assume exact arithmetic and the stated sampling
//! conditions. The f64 implementation uses stable log-domain accumulation and
//! outward endpoint nudges, but is not a formally verified interval-arithmetic
//! implementation.

use std::collections::BTreeSet;

/// Fixed nonnegative betting stakes. Each stake defines a valid one-sided
/// supermartingale for a bounded-mean null; their equal mixture is also valid.
const BET_STAKES: [f64; 8] = [0.05, 0.1, 0.2, 0.35, 0.5, 0.7, 0.85, 1.0];

/// Result of inverting two one-sided anytime-valid bounded-mean tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundedMeanConfidenceSequence {
    /// Number of observations incorporated.
    pub sample_count: u64,
    /// Arithmetic mean of the normalized observations.
    pub empirical_mean: f64,
    /// Lower endpoint, widened outward by one representable f64 value.
    pub lower: f64,
    /// Upper endpoint, widened outward by one representable f64 value.
    pub upper: f64,
    /// Total two-sided error budget; each one-sided test uses half.
    pub alpha: f64,
}

/// Fixed-stake betting confidence sequence for observations in [0, 1].
///
/// The validity condition is conditional-mean stationarity:
/// E[X_t | F_(t-1)] = mu for a fixed mu in [0,1]. Independence and identical
/// distribution are not required. Each positive stake tests H0: mu <= m;
/// each negative stake tests H0: mu >= m. The products are nonnegative
/// supermartingales under their respective composite nulls. Inverting their
/// running maxima and allocating alpha/2 to each side yields simultaneous
/// time-uniform coverage by Ville's inequality and a union bound.
///
/// The finite stake grid affects efficiency, not validity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoundedMeanBettingSequence {
    observations: Vec<f64>,
}

impl BoundedMeanBettingSequence {
    /// Create an empty bounded-mean process.
    pub fn new() -> Self {
        Self {
            observations: Vec::new(),
        }
    }

    /// Add one normalized observation in [0,1].
    ///
    /// Invalid values and counter overflow are rejected before mutation.
    pub fn observe(&mut self, value: f64) -> Result<(), AdaptiveImportanceSamplingError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(AdaptiveImportanceSamplingError::ObservationOutsideUnitInterval);
        }
        if self.observations.len() >= u64::MAX as usize {
            return Err(AdaptiveImportanceSamplingError::CounterOverflow);
        }
        self.observations.push(value);
        Ok(())
    }

    /// Number of normalized observations.
    pub fn sample_count(&self) -> u64 {
        self.observations.len() as u64
    }

    /// Invert the running-max one-sided betting processes at error level alpha.
    ///
    /// The result is nested in time because each test uses the maximum e-value
    /// observed at any previous sample. A numerically empty intersection is a
    /// fail-closed error rather than a manufactured interval.
    pub fn confidence_sequence(
        &self,
        alpha: f64,
    ) -> Result<BoundedMeanConfidenceSequence, AdaptiveImportanceSamplingError> {
        validate_alpha(alpha)?;
        if self.observations.is_empty() {
            return Ok(BoundedMeanConfidenceSequence {
                sample_count: 0,
                empirical_mean: 0.0,
                lower: 0.0,
                upper: 1.0,
                alpha,
            });
        }

        let threshold = 2.0_f64.ln() - alpha.ln();
        let lower = self.lower_bound(threshold)?;
        let upper = self.upper_bound(threshold)?;
        if lower > upper {
            return Err(AdaptiveImportanceSamplingError::EmptyConfidenceSet);
        }

        let empirical_mean =
            self.observations.iter().copied().sum::<f64>() / self.observations.len() as f64;
        Ok(BoundedMeanConfidenceSequence {
            sample_count: self.sample_count(),
            empirical_mean,
            lower,
            upper,
            alpha,
        })
    }

    fn lower_bound(&self, log_threshold: f64) -> Result<f64, AdaptiveImportanceSamplingError> {
        // Positive stakes test H0: mu <= candidate_mean. Since their e-values
        // decrease as candidate_mean increases, rejection at a candidate
        // proves the true mean is above it and supplies a lower endpoint.
        let at_zero = self.max_log_e_value(0.0, true)?;
        if at_zero < log_threshold {
            return Ok(0.0);
        }

        let mut rejected = 0.0;
        let mut accepted = 1.0;
        for _ in 0..80 {
            let midpoint = 0.5 * rejected + 0.5 * accepted;
            if midpoint <= rejected || midpoint >= accepted {
                break;
            }
            if self.max_log_e_value(midpoint, true)? >= log_threshold {
                rejected = midpoint;
            } else {
                accepted = midpoint;
            }
        }
        Ok(next_down(rejected).max(0.0))
    }

    fn upper_bound(&self, log_threshold: f64) -> Result<f64, AdaptiveImportanceSamplingError> {
        // Negative stakes test H0: mu >= candidate_mean. Their e-values
        // increase as candidate_mean increases, so rejection proves the true
        // mean is below the rejected candidate and supplies an upper endpoint.
        let at_one = self.max_log_e_value(1.0, false)?;
        if at_one < log_threshold {
            return Ok(1.0);
        }

        let mut accepted = 0.0;
        let mut rejected = 1.0;
        for _ in 0..80 {
            let midpoint = 0.5 * accepted + 0.5 * rejected;
            if midpoint <= accepted || midpoint >= rejected {
                break;
            }
            if self.max_log_e_value(midpoint, false)? >= log_threshold {
                rejected = midpoint;
            } else {
                accepted = midpoint;
            }
        }
        Ok(next_up(rejected).min(1.0))
    }

    /// Directional e-process: positive stakes test mu <= candidate, negative
    /// stakes test mu >= candidate. Returns the maximum log-mixture e-value
    /// over all times, including time zero.
    fn max_log_e_value(
        &self,
        candidate_mean: f64,
        positive_stakes: bool,
    ) -> Result<f64, AdaptiveImportanceSamplingError> {
        let mut log_wealth = [0.0_f64; BET_STAKES.len()];
        let mut maximum = 0.0_f64;
        let log_stake_count = (BET_STAKES.len() as f64).ln();

        for observation in &self.observations {
            for (index, stake) in BET_STAKES.iter().copied().enumerate() {
                let centered = *observation - candidate_mean;
                let factor = if positive_stakes {
                    1.0 + stake * centered
                } else {
                    1.0 - stake * centered
                };
                if !factor.is_finite() || factor < 0.0 {
                    return Err(AdaptiveImportanceSamplingError::NumericalFailure);
                }
                if factor == 0.0 {
                    log_wealth[index] = f64::NEG_INFINITY;
                } else if log_wealth[index] != f64::NEG_INFINITY {
                    log_wealth[index] += factor.ln();
                    if !log_wealth[index].is_finite() {
                        return Err(AdaptiveImportanceSamplingError::NumericalFailure);
                    }
                }
            }

            let current = log_sum_exp(&log_wealth) - log_stake_count;
            if current.is_nan() {
                return Err(AdaptiveImportanceSamplingError::NumericalFailure);
            }
            maximum = maximum.max(current);
        }

        Ok(maximum)
    }
}

/// One independently generated importance-sampling draw with immutable evidence identities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveImportanceDraw {
    /// Strictly increasing zero-based sequence index.
    pub sequence_index: u64,
    /// Stable, nonzero identity of this sample.
    pub sample_id: [u8; 32],
    /// Collision event and revision identity.
    pub event_digest: [u8; 32],
    /// Target probability-measure identity.
    pub target_measure_digest: [u8; 32],
    /// Digest of the complete per-draw proposal density q_t, including its defensive component.
    pub proposal_digest: [u8; 32],
    /// Digest of the fixed defensive component, expected to identify the target measure p.
    pub defensive_component_digest: [u8; 32],
    /// Defensive mixture weight used by this draw; must match the accumulator's fixed value.
    pub defensive_mixture_weight: f64,
    /// Log target density log p(X_t); negative infinity is allowed when p(X_t)=0.
    pub log_target_density: f64,
    /// Log of the complete proposal density log q_t(X_t); must be finite.
    pub log_proposal_density: f64,
    /// Whether this sample belongs to the collision event.
    pub event_occurred: bool,
}

/// Adaptive importance-sampling probability estimate and time-uniform confidence sequence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AdaptiveImportanceSamplingSummary {
    /// Number of accepted importance-sampling draws.
    pub sample_count: u64,
    /// Fixed defensive-mixture weight alpha_mix.
    pub defensive_mixture_weight: f64,
    /// Error probability used by the two-sided confidence sequence.
    pub confidence_alpha: f64,
    /// Empirical mean of alpha_mix * 1_E * p(X)/q_t(X).
    pub normalized_mean_estimate: f64,
    /// Importance-sampling point estimate of P_p(E); not clipped to [0,1].
    pub target_probability_estimate: f64,
    /// Lower endpoint for P_p(E), clipped to the probability domain.
    pub lower: f64,
    /// Upper endpoint for P_p(E), clipped to the probability domain.
    pub upper: f64,
}

/// Fail-closed validation and numerical errors for adaptive importance sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdaptiveImportanceSamplingError {
    /// Normalized bounded-mean observation is outside [0,1] or non-finite.
    ObservationOutsideUnitInterval,
    /// Defensive-mixture or confidence error probability is invalid.
    InvalidProbability,
    /// One or more required SHA-256 identities are all zero.
    InvalidDigest,
    /// Event identity differs from the accumulator's fixed event revision.
    EventIdentityMismatch,
    /// Target probability measure differs from the fixed target.
    TargetMeasureMismatch,
    /// Defensive component identity differs from the fixed target component.
    DefensiveComponentMismatch,
    /// Per-draw defensive mixture weight differs from the fixed weight.
    DefensiveMixtureWeightMismatch,
    /// Proposal density is zero, NaN, or infinite at a sampled state.
    InvalidProposalDensity,
    /// Target log density is NaN or positive infinity.
    InvalidTargetDensity,
    /// The observed normalized importance weight exceeds its defensive bound.
    DefensiveWeightBoundViolation,
    /// Sample sequence index is not exactly the next expected index.
    NonSequentialSampleIndex,
    /// A sample identity has already been accepted.
    DuplicateSampleIdentity,
    /// Sample counter would overflow.
    CounterOverflow,
    /// A betting update or inversion produced an invalid numerical value.
    NumericalFailure,
    /// The two one-sided inversions produced an empty numerical intersection.
    EmptyConfidenceSet,
}

/// Streaming adaptive importance-sampling confidence sequence.
///
/// Each accepted draw contributes Y_t = alpha_mix * 1_E(X_t) * p(X_t)/q_t(X_t).
/// Under the defensive-mixture domination and predictable sampling conditions,
/// Y_t is in [0,1] and E[Y_t | F_(t-1)] = alpha_mix * P_p(E). The bounded-mean
/// betting sequence therefore supplies a time-uniform interval for that mean;
/// dividing by alpha_mix gives an interval for the target event probability.
///
/// Proposal densities may change between draws. The target measure, event
/// identity, defensive reference and mixture weight are fixed for the entire
/// sequence. The adapter verifies each observed density ratio, not the global
/// domination condition or the fact that the sampler generated X_t from q_t.
#[derive(Debug, Clone, PartialEq)]
pub struct AdaptiveImportanceSamplingConfidenceSequence {
    event_digest: [u8; 32],
    target_measure_digest: [u8; 32],
    defensive_component_digest: [u8; 32],
    defensive_mixture_weight: f64,
    confidence_alpha: f64,
    next_sequence_index: u64,
    sample_ids: BTreeSet<[u8; 32]>,
    bounded_mean: BoundedMeanBettingSequence,
}

impl AdaptiveImportanceSamplingConfidenceSequence {
    /// Bind a new adaptive importance-sampling sequence to one event, target
    /// measure and defensive component.
    pub fn new(
        event_digest: [u8; 32],
        target_measure_digest: [u8; 32],
        defensive_component_digest: [u8; 32],
        defensive_mixture_weight: f64,
        confidence_alpha: f64,
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
        validate_alpha(confidence_alpha)?;

        Ok(Self {
            event_digest,
            target_measure_digest,
            defensive_component_digest,
            defensive_mixture_weight,
            confidence_alpha,
            next_sequence_index: 0,
            sample_ids: BTreeSet::new(),
            bounded_mean: BoundedMeanBettingSequence::new(),
        })
    }

    /// Accept one draw after validating identities, order, proposal density
    /// and the observed defensive importance-weight bound.
    ///
    /// The update is transactional: all validation and numerical operations
    /// complete before sample identity or sequence state is advanced.
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
        self.bounded_mean.observe(observation)?;
        self.sample_ids.insert(draw.sample_id);
        self.next_sequence_index = next_sequence_index;
        Ok(())
    }

    /// Number of accepted draws.
    pub fn sample_count(&self) -> u64 {
        self.bounded_mean.sample_count()
    }

    /// Return the target event-probability estimate and time-uniform interval.
    pub fn summary(
        &self,
    ) -> Result<AdaptiveImportanceSamplingSummary, AdaptiveImportanceSamplingError> {
        let normalized = self
            .bounded_mean
            .confidence_sequence(self.confidence_alpha)?;
        let scale = self.defensive_mixture_weight.recip();
        let target_probability_estimate = normalized.empirical_mean * scale;
        if !target_probability_estimate.is_finite() {
            return Err(AdaptiveImportanceSamplingError::NumericalFailure);
        }

        Ok(AdaptiveImportanceSamplingSummary {
            sample_count: normalized.sample_count,
            defensive_mixture_weight: self.defensive_mixture_weight,
            confidence_alpha: self.confidence_alpha,
            normalized_mean_estimate: normalized.empirical_mean,
            target_probability_estimate,
            lower: (normalized.lower * scale).clamp(0.0, 1.0),
            upper: (normalized.upper * scale).clamp(0.0, 1.0),
        })
    }
}

fn validate_alpha(alpha: f64) -> Result<(), AdaptiveImportanceSamplingError> {
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        Err(AdaptiveImportanceSamplingError::InvalidProbability)
    } else {
        Ok(())
    }
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

fn log_sum_exp(values: &[f64]) -> f64 {
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if maximum == f64::NEG_INFINITY {
        return f64::NEG_INFINITY;
    }
    let scaled_sum = values
        .iter()
        .map(|value| (value - maximum).exp())
        .sum::<f64>();
    maximum + scaled_sum.ln()
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

    const EVENT: [u8; 32] = [1; 32];
    const TARGET: [u8; 32] = [2; 32];
    const DEFENSIVE: [u8; 32] = [3; 32];

    fn draw(
        sequence_index: u64,
        sample_id: [u8; 32],
        proposal_id: [u8; 32],
        event_occurred: bool,
        target_density: f64,
        proposal_density: f64,
    ) -> AdaptiveImportanceDraw {
        AdaptiveImportanceDraw {
            sequence_index,
            sample_id,
            event_digest: EVENT,
            target_measure_digest: TARGET,
            proposal_digest: proposal_id,
            defensive_component_digest: DEFENSIVE,
            defensive_mixture_weight: 0.1,
            log_target_density: target_density.ln(),
            log_proposal_density: proposal_density.ln(),
            event_occurred,
        }
    }

    fn accumulator() -> AdaptiveImportanceSamplingConfidenceSequence {
        AdaptiveImportanceSamplingConfidenceSequence::new(EVENT, TARGET, DEFENSIVE, 0.1, 0.05)
            .unwrap()
    }

    #[test]
    fn empty_bounded_mean_sequence_is_the_unit_interval() {
        let sequence = BoundedMeanBettingSequence::new();
        let result = sequence.confidence_sequence(0.05).unwrap();
        assert_eq!(result.sample_count, 0);
        assert_eq!(result.lower, 0.0);
        assert_eq!(result.upper, 1.0);
    }

    #[test]
    fn bounded_mean_sequence_rejects_values_outside_unit_interval_transactionally() {
        let mut sequence = BoundedMeanBettingSequence::new();
        assert_eq!(
            sequence.observe(1.0001),
            Err(AdaptiveImportanceSamplingError::ObservationOutsideUnitInterval)
        );
        assert_eq!(sequence.sample_count(), 0);
    }

    #[test]
    fn bounded_mean_confidence_sequence_is_nested_over_time() {
        let mut sequence = BoundedMeanBettingSequence::new();
        let mut previous = sequence.confidence_sequence(0.05).unwrap();
        for value in [0.9, 0.1, 0.8, 0.2, 0.7, 0.3, 0.6, 0.4] {
            sequence.observe(value).unwrap();
            let current = sequence.confidence_sequence(0.05).unwrap();
            assert!(current.lower >= previous.lower);
            assert!(current.upper <= previous.upper);
            assert!(current.lower <= current.upper);
            previous = current;
        }
    }

    #[test]
    fn bounded_mean_confidence_sequence_contains_a_constant_sequence_mean() {
        let mut sequence = BoundedMeanBettingSequence::new();
        for value in [0.25; 100] {
            sequence.observe(value).unwrap();
        }
        let result = sequence.confidence_sequence(0.05).unwrap();
        assert!(result.lower <= 0.25);
        assert!(result.upper >= 0.25);
        assert!(result.upper - result.lower < 0.25);
    }

    #[test]
    fn adaptive_importance_sequence_accepts_changing_predictable_proposal_digests() {
        let mut sequence = accumulator();
        sequence
            .observe(draw(0, [4; 32], [10; 32], true, 0.2, 0.74))
            .unwrap();
        sequence
            .observe(draw(1, [5; 32], [11; 32], false, 0.8, 0.26))
            .unwrap();
        sequence
            .observe(draw(2, [6; 32], [12; 32], true, 0.2, 0.065))
            .unwrap();

        assert_eq!(sequence.sample_count(), 3);
        let summary = sequence.summary().unwrap();
        assert_eq!(summary.sample_count, 3);
        assert!(summary.target_probability_estimate.is_finite());
        assert!(summary.lower <= summary.upper);
    }

    #[test]
    fn observed_importance_weight_above_defensive_bound_is_rejected() {
        let mut sequence = accumulator();
        // alpha * p / q = 0.1 * 0.2 / 0.01 = 2 > 1.
        assert_eq!(
            sequence.observe(draw(0, [4; 32], [10; 32], true, 0.2, 0.01)),
            Err(AdaptiveImportanceSamplingError::DefensiveWeightBoundViolation)
        );
        assert_eq!(sequence.sample_count(), 0);
    }

    #[test]
    fn event_measure_and_defensive_identities_fail_closed() {
        let mut sequence = accumulator();
        let mut changed_event = draw(0, [4; 32], [10; 32], true, 0.2, 0.74);
        changed_event.event_digest = [9; 32];
        assert_eq!(
            sequence.observe(changed_event),
            Err(AdaptiveImportanceSamplingError::EventIdentityMismatch)
        );

        let mut changed_measure = draw(0, [4; 32], [10; 32], true, 0.2, 0.74);
        changed_measure.target_measure_digest = [9; 32];
        assert_eq!(
            sequence.observe(changed_measure),
            Err(AdaptiveImportanceSamplingError::TargetMeasureMismatch)
        );

        let mut changed_defensive = draw(0, [4; 32], [10; 32], true, 0.2, 0.74);
        changed_defensive.defensive_component_digest = [9; 32];
        assert_eq!(
            sequence.observe(changed_defensive),
            Err(AdaptiveImportanceSamplingError::DefensiveComponentMismatch)
        );
        assert_eq!(sequence.sample_count(), 0);
    }

    #[test]
    fn sample_order_duplicates_and_mixture_weight_changes_fail_closed() {
        let mut sequence = accumulator();
        assert_eq!(
            sequence.observe(draw(1, [4; 32], [10; 32], true, 0.2, 0.74)),
            Err(AdaptiveImportanceSamplingError::NonSequentialSampleIndex)
        );

        sequence
            .observe(draw(0, [4; 32], [10; 32], true, 0.2, 0.74))
            .unwrap();
        assert_eq!(
            sequence.observe(draw(1, [4; 32], [11; 32], true, 0.2, 0.74)),
            Err(AdaptiveImportanceSamplingError::DuplicateSampleIdentity)
        );

        let mut changed_alpha = draw(1, [5; 32], [11; 32], true, 0.2, 0.74);
        changed_alpha.defensive_mixture_weight = 0.2;
        assert_eq!(
            sequence.observe(changed_alpha),
            Err(AdaptiveImportanceSamplingError::DefensiveMixtureWeightMismatch)
        );
        assert_eq!(sequence.sample_count(), 1);
    }

    #[test]
    fn invalid_confidence_and_defensive_weights_are_rejected() {
        assert_eq!(
            BoundedMeanBettingSequence::new().confidence_sequence(0.0),
            Err(AdaptiveImportanceSamplingError::InvalidProbability)
        );
        assert_eq!(
            AdaptiveImportanceSamplingConfidenceSequence::new(EVENT, TARGET, DEFENSIVE, 0.0, 0.05,),
            Err(AdaptiveImportanceSamplingError::InvalidProbability)
        );
        assert_eq!(
            AdaptiveImportanceSamplingConfidenceSequence::new(EVENT, TARGET, DEFENSIVE, 0.1, 1.0,),
            Err(AdaptiveImportanceSamplingError::InvalidProbability)
        );
    }

    #[test]
    fn adaptive_finite_support_proposals_have_anytime_valid_exact_path_coverage() {
        // Target X ~ Bernoulli(0.2), event E={X=1}, defensive mass 0.1.
        // The residual proposal changes predictably: if the previous draw was
        // an event it uses r_t(1)=0.8, otherwise r_t(1)=0.05. Therefore q_t
        // changes with the history while q_t(x) >= 0.1 p(x) exactly.
        const TARGET_EVENT_PROBABILITY: f64 = 0.2;
        const MIXTURE_WEIGHT: f64 = 0.1;
        const HORIZON: u32 = 6;
        let mut total_miss_probability = 0.0;

        for encoded_path in 0_u32..(1_u32 << HORIZON) {
            let mut sequence = AdaptiveImportanceSamplingConfidenceSequence::new(
                EVENT,
                TARGET,
                DEFENSIVE,
                MIXTURE_WEIGHT,
                0.05,
            )
            .unwrap();
            let mut path_probability = 1.0;
            let mut previous_event = false;
            let mut ever_missed = false;

            for time in 0..HORIZON {
                let residual_event_probability = if previous_event { 0.8 } else { 0.05 };
                let proposal_event_probability = MIXTURE_WEIGHT * TARGET_EVENT_PROBABILITY
                    + (1.0 - MIXTURE_WEIGHT) * residual_event_probability;
                let event_occurred = ((encoded_path >> time) & 1) == 1;

                if event_occurred {
                    path_probability *= proposal_event_probability;
                } else {
                    path_probability *= 1.0 - proposal_event_probability;
                }

                let (target_density, proposal_density) = if event_occurred {
                    (TARGET_EVENT_PROBABILITY, proposal_event_probability)
                } else {
                    (
                        1.0 - TARGET_EVENT_PROBABILITY,
                        1.0 - proposal_event_probability,
                    )
                };

                sequence
                    .observe(draw(
                        time as u64,
                        [time as u8 + 1; 32],
                        [time as u8 + 20; 32],
                        event_occurred,
                        target_density,
                        proposal_density,
                    ))
                    .unwrap();

                let summary = sequence.summary().unwrap();
                if summary.lower > TARGET_EVENT_PROBABILITY
                    || summary.upper < TARGET_EVENT_PROBABILITY
                {
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
    fn zero_target_density_produces_zero_normalized_importance_weight() {
        let mut sequence = accumulator();
        let mut no_target_mass = draw(0, [4; 32], [10; 32], true, 0.0, 0.74);
        no_target_mass.log_target_density = f64::NEG_INFINITY;
        sequence.observe(no_target_mass).unwrap();
        assert_eq!(sequence.summary().unwrap().target_probability_estimate, 0.0);
    }

    #[test]
    fn non_finite_proposal_density_is_rejected_without_mutation() {
        let mut sequence = accumulator();
        let mut invalid = draw(0, [4; 32], [10; 32], false, 0.8, 0.26);
        invalid.log_proposal_density = f64::NEG_INFINITY;
        assert_eq!(
            sequence.observe(invalid),
            Err(AdaptiveImportanceSamplingError::InvalidProposalDensity)
        );
        assert_eq!(sequence.sample_count(), 0);
    }
    #[test]
    fn all_zero_path_keeps_zero_lower_bound_and_positive_upper_bound() {
        let mut sequence = BoundedMeanBettingSequence::new();
        for _ in 0..100 {
            sequence.observe(0.0).unwrap();
        }
        let report = sequence.confidence_sequence(0.05).unwrap();
        assert_eq!(report.lower, 0.0);
        assert!(report.upper > 0.0);
        assert!(report.upper < 0.1);
    }

    #[test]
    fn all_one_path_keeps_one_upper_bound_and_nontrivial_lower_bound() {
        let mut sequence = BoundedMeanBettingSequence::new();
        for _ in 0..100 {
            sequence.observe(1.0).unwrap();
        }
        let report = sequence.confidence_sequence(0.05).unwrap();
        assert!(report.lower < 1.0);
        assert!(report.lower > 0.9);
        assert_eq!(report.upper, 1.0);
    }
}
