// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Time-uniform confidence sequences for defensively bounded importance samples.
//!
//! Under a predictable proposal
//!
//! q_t(x) = epsilon * p(x) + (1 - epsilon) * r_t(x)
//!
//! the scaled importance contribution
//!
//! Y_t = epsilon * 1_A(X_t) * p(X_t) / q_t(X_t)
//!
//! lies in [0, 1] and has constant conditional mean epsilon * P_p(A).
//! This module applies the predictable plug-in empirical-Bernstein confidence
//! sequence for bounded observations to that process.
//!
//! The caller remains responsible for ensuring that each draw is sampled from
//! the declared conditional proposal, that r_t is selected before the draw,
//! and that log_proposal_density is the full defensive-mixture log density.
//! The accumulator cannot verify those sampling assumptions. It rejects
//! out-of-range contributions rather than clipping them.
//!
//! The confidence-sequence theorem is due to Waudby-Smith and Ramdas (2024),
//! Theorem 2, Journal of the Royal Statistical Society: Series B,
//! https://doi.org/10.1093/jrsssb/qkad009. This implementation is a
//! composition of that established bounded-mean result with defensive
//! importance sampling, not a new confidence-sequence theorem.
//!
//! Floating-point evaluation is preliminary numerical bookkeeping, not a
//! formally machine-verified implementation of the theorem's real arithmetic.

use crate::KernelError;

/// A time-uniform probability interval for defensive importance sampling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefensiveImportanceConfidenceReport {
    /// Number of bounded weighted observations used.
    pub sample_count: u64,
    /// Fixed nominal-distribution mass in the defensive proposal.
    pub defensive_mass: f64,
    /// Overall time-uniform error probability.
    pub alpha: f64,
    /// Lower endpoint for the bounded mean epsilon * Pc, before restricting
    /// to the known parameter range [0, epsilon].
    pub bounded_mean_lower: f64,
    /// Upper endpoint for the bounded mean epsilon * Pc, before restricting
    /// to the known parameter range [0, epsilon].
    pub bounded_mean_upper: f64,
    /// Lower endpoint for Pc after intersecting with [0, epsilon] and
    /// dividing by the fixed defensive mass.
    pub target_probability_lower: f64,
    /// Upper endpoint for Pc after intersecting with [0, epsilon] and
    /// dividing by the fixed defensive mass.
    pub target_probability_upper: f64,
}

/// Streaming predictable plug-in empirical-Bernstein confidence sequence.
///
/// The process is valid for observations Y_t in [0, 1] whose conditional
/// mean given all prior observations is a fixed constant. For orbital
/// importance sampling, use observe_importance_sample with a fixed
/// defensive mass and the complete proposal mixture density.
///
/// The running intersection is maintained after each update. The confidence
/// sequence is valid at arbitrary stopping times under the stated
/// conditional-mean and predictability assumptions.
#[derive(Debug, Clone, PartialEq)]
pub struct DefensiveImportanceConfidenceAccumulator {
    defensive_mass: f64,
    alpha: f64,
    lambda_cap: f64,
    sample_count: u64,
    sum_observations: f64,
    sum_squared_predictable_residuals: f64,
    sum_lambda: f64,
    sum_lambda_observations: f64,
    sum_empirical_bernstein_penalties: f64,
    mean_lower: f64,
    mean_upper: f64,
}

impl DefensiveImportanceConfidenceAccumulator {
    /// Create an empty accumulator with a fixed defensive mixture mass.
    ///
    /// defensive_mass must lie strictly between zero and one. alpha is the
    /// desired total time-uniform error probability. The predictable
    /// plug-in step-size cap defaults to 0.5, following the reasonable
    /// defaults discussed by Waudby-Smith and Ramdas.
    pub fn new(defensive_mass: f64, alpha: f64) -> Result<Self, KernelError> {
        Self::with_lambda_cap(defensive_mass, alpha, 0.5)
    }

    /// Create an empty accumulator with an explicit predictable step-size cap.
    ///
    /// The cap must lie strictly between zero and one. It is not selected
    /// using the current observation; each step size is predictable.
    pub fn with_lambda_cap(
        defensive_mass: f64,
        alpha: f64,
        lambda_cap: f64,
    ) -> Result<Self, KernelError> {
        if !defensive_mass.is_finite() || defensive_mass <= 0.0 || defensive_mass >= 1.0 {
            return Err(KernelError::InvalidDefensiveMass);
        }
        if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
            return Err(KernelError::InvalidAlpha);
        }
        if !lambda_cap.is_finite() || lambda_cap <= 0.0 || lambda_cap >= 1.0 {
            return Err(KernelError::InvalidStepSize);
        }

        Ok(Self {
            defensive_mass,
            alpha,
            lambda_cap,
            sample_count: 0,
            sum_observations: 0.0,
            sum_squared_predictable_residuals: 0.0,
            sum_lambda: 0.0,
            sum_lambda_observations: 0.0,
            sum_empirical_bernstein_penalties: 0.0,
            mean_lower: 0.0,
            mean_upper: 1.0,
        })
    }

    /// Add a bounded observation Y_t directly.
    ///
    /// This method is useful for analytic tests and for callers that have
    /// already computed the defensively scaled contribution. Values outside
    /// [0, 1], non-finite values, numerical overflow, or an empty
    /// intersection fail closed and leave the accumulator unchanged.
    pub fn observe_bounded_contribution(
        &mut self,
        contribution: f64,
    ) -> Result<DefensiveImportanceConfidenceReport, KernelError> {
        if !contribution.is_finite() || !(0.0..=1.0).contains(&contribution) {
            return Err(KernelError::InvalidProbability);
        }

        let next_count = self
            .sample_count
            .checked_add(1)
            .ok_or(KernelError::CounterOverflow)?;
        let time = next_count as f64;
        let previous_count_plus_one = self.sample_count as f64 + 1.0;
        let previous_mean = (0.5 + self.sum_observations) / previous_count_plus_one;
        let previous_variance =
            (0.25 + self.sum_squared_predictable_residuals) / previous_count_plus_one;
        let log_alpha_term = 2.0_f64.ln() - self.alpha.ln();
        let denominator = previous_variance * time * (1.0 + time).ln();
        if !previous_mean.is_finite()
            || !previous_variance.is_finite()
            || previous_variance <= 0.0
            || !denominator.is_finite()
            || denominator <= 0.0
            || !log_alpha_term.is_finite()
            || log_alpha_term <= 0.0
        {
            return Err(KernelError::NumericalOverflow);
        }

        // Predictable plug-in empirical-Bernstein step size.
        let lambda = (2.0 * log_alpha_term / denominator)
            .sqrt()
            .min(self.lambda_cap);
        if !lambda.is_finite() || lambda <= 0.0 || lambda >= 1.0 {
            return Err(KernelError::NumericalOverflow);
        }

        let residual = contribution - previous_mean;
        let variance_term = 4.0 * residual * residual;
        let psi = (-(-lambda).ln_1p() - lambda) / 4.0;
        let penalty = variance_term * psi;

        let next_sum_observations = self.sum_observations + contribution;
        // The empirical-Bernstein variance proxy sums predictable residuals:
        // the current observation is centred at the prior smoothed mean, not
        // at a mean that has already incorporated this observation.
        let next_squared_residuals =
            self.sum_squared_predictable_residuals + residual * residual;
        let next_sum_lambda = self.sum_lambda + lambda;
        let next_sum_lambda_observations = self.sum_lambda_observations + lambda * contribution;
        let next_penalties = self.sum_empirical_bernstein_penalties + penalty;

        if !variance_term.is_finite()
            || !psi.is_finite()
            || !penalty.is_finite()
            || !next_sum_observations.is_finite()
            || !current_smoothed_mean.is_finite()
            || !next_squared_residuals.is_finite()
            || !next_sum_lambda.is_finite()
            || next_sum_lambda <= 0.0
            || !next_sum_lambda_observations.is_finite()
            || !next_penalties.is_finite()
        {
            return Err(KernelError::NumericalOverflow);
        }

        let centre = next_sum_lambda_observations / next_sum_lambda;
        let radius = (log_alpha_term + next_penalties) / next_sum_lambda;
        if !centre.is_finite() || !radius.is_finite() || radius < 0.0 {
            return Err(KernelError::NumericalOverflow);
        }
        let candidate_lower = next_down(centre - radius).max(0.0);
        let candidate_upper = next_up(centre + radius).min(1.0);
        let next_lower = self.mean_lower.max(candidate_lower);
        let next_upper = self.mean_upper.min(candidate_upper);

        if !next_lower.is_finite()
            || !next_upper.is_finite()
            || next_lower < 0.0
            || next_upper > 1.0
            || next_lower > next_upper
        {
            return Err(KernelError::EmptyConfidenceSet);
        }

        let restricted_mean_lower = next_lower.max(0.0);
        let restricted_mean_upper = next_upper.min(self.defensive_mass);
        if restricted_mean_lower > restricted_mean_upper {
            return Err(KernelError::EmptyConfidenceSet);
        }
        let target_probability_lower = (restricted_mean_lower / self.defensive_mass).max(0.0);
        let target_probability_upper = (restricted_mean_upper / self.defensive_mass).min(1.0);
        if !target_probability_lower.is_finite()
            || !target_probability_upper.is_finite()
            || target_probability_lower > target_probability_upper
        {
            return Err(KernelError::EmptyConfidenceSet);
        }

        // Commit only after every calculation and confidence-set check succeeds.
        self.sample_count = next_count;
        self.sum_observations = next_sum_observations;
        self.sum_squared_predictable_residuals = next_squared_residuals;
        self.sum_lambda = next_sum_lambda;
        self.sum_lambda_observations = next_sum_lambda_observations;
        self.sum_empirical_bernstein_penalties = next_penalties;
        self.mean_lower = next_lower;
        self.mean_upper = next_upper;

        Ok(DefensiveImportanceConfidenceReport {
            sample_count: next_count,
            defensive_mass: self.defensive_mass,
            alpha: self.alpha,
            bounded_mean_lower: next_lower,
            bounded_mean_upper: next_upper,
            target_probability_lower,
            target_probability_upper,
        })
    }

    /// Add one importance-sampling draw using full defensive-mixture densities.
    ///
    /// The caller must have sampled from
    /// q_t = epsilon * p + (1 - epsilon) * r_t, with r_t selected using
    /// only past observations. log_proposal_density must be the log density
    /// of the entire q_t, not merely its adapted component.
    ///
    /// A non-event contributes zero. An event contributes
    /// epsilon * exp(log_target_density - log_proposal_density).
    /// The method rejects a ratio above one and rejects positive contributions
    /// that underflow to zero; it never clips a weight to fit the bound.
    pub fn observe_importance_sample(
        &mut self,
        event_occurred: bool,
        log_target_density: f64,
        log_proposal_density: f64,
    ) -> Result<DefensiveImportanceConfidenceReport, KernelError> {
        if log_proposal_density == f64::NEG_INFINITY {
            return Err(KernelError::ProposalHasZeroDensity);
        }
        if !log_proposal_density.is_finite()
            || log_target_density.is_nan()
            || log_target_density == f64::INFINITY
        {
            return Err(KernelError::NonFiniteValue);
        }

        let contribution = if event_occurred {
            if log_target_density == f64::NEG_INFINITY {
                0.0
            } else {
                let log_contribution =
                    self.defensive_mass.ln() + log_target_density - log_proposal_density;
                if !log_contribution.is_finite() {
                    return Err(KernelError::NumericalOverflow);
                }
                if log_contribution > 0.0 {
                    return Err(KernelError::InvalidProbability);
                }
                let value = log_contribution.exp();
                if value == 0.0 {
                    return Err(KernelError::NumericalUnderflow);
                }
                value
            }
        } else {
            0.0
        };

        self.observe_bounded_contribution(contribution)
    }

    /// Number of bounded importance-sampling contributions accumulated.
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Return the current confidence sequence without adding an observation.
    ///
    /// Before the first observation, the confidence sequence is [0, 1] for
    /// the target event probability.
    pub fn confidence_sequence(&self) -> Result<DefensiveImportanceConfidenceReport, KernelError> {
        let restricted_mean_lower = self.mean_lower.max(0.0);
        let restricted_mean_upper = self.mean_upper.min(self.defensive_mass);
        if restricted_mean_lower > restricted_mean_upper {
            return Err(KernelError::EmptyConfidenceSet);
        }
        Ok(DefensiveImportanceConfidenceReport {
            sample_count: self.sample_count,
            defensive_mass: self.defensive_mass,
            alpha: self.alpha,
            bounded_mean_lower: self.mean_lower,
            bounded_mean_upper: self.mean_upper,
            target_probability_lower: restricted_mean_lower / self.defensive_mass,
            target_probability_upper: (restricted_mean_upper / self.defensive_mass).min(1.0),
        })
    }
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

    #[test]
    fn empty_accumulator_has_full_target_probability_interval() {
        let accumulator =
            DefensiveImportanceConfidenceAccumulator::new(0.2, 0.05).expect("valid settings");
        let report = accumulator.confidence_sequence().expect("empty CS");
        assert_eq!(report.sample_count, 0);
        assert_eq!(report.bounded_mean_lower, 0.0);
        assert_eq!(report.bounded_mean_upper, 1.0);
        assert_eq!(report.target_probability_lower, 0.0);
        assert_eq!(report.target_probability_upper, 1.0);
    }

    #[test]
    fn defensive_mixture_weight_uses_full_proposal_density() {
        let epsilon = 0.2;
        let target_density: f64 = 0.5;
        let proposal_density: f64 = epsilon * 0.5 + (1.0 - epsilon) * 0.9;
        let mut accumulator =
            DefensiveImportanceConfidenceAccumulator::new(epsilon, 0.05).expect("valid settings");
        let report = accumulator
            .observe_importance_sample(true, target_density.ln(), proposal_density.ln())
            .expect("bounded defensive contribution");
        let expected = epsilon * target_density / proposal_density;
        assert!((report.target_probability_lower - 0.0).abs() < 1.0e-15);
        assert!(report.target_probability_upper <= 1.0);
        assert!((accumulator.sum_observations - expected).abs() < 1.0e-14);
    }

    #[test]
    fn adaptive_proposal_keeps_the_same_conditional_mean() {
        let epsilon = 0.2;
        let target = [0.5_f64, 0.5_f64];
        let event = [true, false];
        let adapted_proposals = [[0.9_f64, 0.1_f64], [0.1_f64, 0.9_f64]];

        for adapted in adapted_proposals {
            let proposal = [
                epsilon * target[0] + (1.0 - epsilon) * adapted[0],
                epsilon * target[1] + (1.0 - epsilon) * adapted[1],
            ];
            let mut conditional_mean = 0.0;
            for (is_event, (target_density, proposal_density)) in event
                .into_iter()
                .zip(target.into_iter().zip(proposal))
            {
                if is_event {
                    let contribution = epsilon * target_density / proposal_density;
                    conditional_mean += proposal_density * contribution;
                }
            }
            let expected = epsilon * target[0];
            assert!((conditional_mean - expected).abs() < 1.0e-14);
        }
    }

    #[test]
    fn running_intersection_never_widens() {
        let mut accumulator =
            DefensiveImportanceConfidenceAccumulator::new(0.2, 0.05).expect("valid settings");
        let mut previous_lower = 0.0;
        let mut previous_upper = 1.0;
        for contribution in [0.0, 0.0, 0.05, 0.0, 0.1, 0.0, 0.02, 0.0, 0.01] {
            let report = accumulator
                .observe_bounded_contribution(contribution)
                .expect("valid contribution");
            assert!(report.target_probability_lower >= previous_lower);
            assert!(report.target_probability_upper <= previous_upper);
            previous_lower = report.target_probability_lower;
            previous_upper = report.target_probability_upper;
        }
    }

    #[test]
    fn out_of_range_contribution_fails_without_mutation() {
        let mut accumulator =
            DefensiveImportanceConfidenceAccumulator::new(0.2, 0.05).expect("valid settings");
        accumulator.observe_bounded_contribution(0.1).unwrap();
        let before = accumulator.clone();
        assert_eq!(
            accumulator.observe_bounded_contribution(1.000_001),
            Err(KernelError::InvalidProbability)
        );
        assert_eq!(accumulator, before);
    }

    #[test]
    fn zero_proposal_density_and_underflow_fail_without_mutation() {
        let mut accumulator =
            DefensiveImportanceConfidenceAccumulator::new(0.1, 0.05).expect("valid settings");
        assert_eq!(
            accumulator.observe_importance_sample(true, 0.0, f64::NEG_INFINITY),
            Err(KernelError::ProposalHasZeroDensity)
        );
        let before = accumulator.clone();
        assert_eq!(
            accumulator.observe_importance_sample(true, -1000.0, 0.0),
            Err(KernelError::NumericalUnderflow)
        );
        assert_eq!(accumulator, before);
    }

    #[test]
    fn invalid_constructors_fail_closed() {
        assert_eq!(
            DefensiveImportanceConfidenceAccumulator::new(0.0, 0.05),
            Err(KernelError::InvalidDefensiveMass)
        );
        assert_eq!(
            DefensiveImportanceConfidenceAccumulator::new(1.0, 0.05),
            Err(KernelError::InvalidDefensiveMass)
        );
        assert_eq!(
            DefensiveImportanceConfidenceAccumulator::new(0.2, 0.0),
            Err(KernelError::InvalidAlpha)
        );
        assert_eq!(
            DefensiveImportanceConfidenceAccumulator::with_lambda_cap(0.2, 0.05, 1.0),
            Err(KernelError::InvalidStepSize)
        );
    }

    #[test]
    fn exhaustive_optional_stopping_coverage_for_short_binary_horizon() {
        let event_mean = 0.2_f64;
        let alpha = 0.1_f64;
        let horizon = 12_u32;
        let epsilon = 0.8_f64;
        let mut rejected_path_probability = 0.0_f64;

        for encoded_path in 0_u32..(1_u32 << horizon) {
            let mut accumulator = DefensiveImportanceConfidenceAccumulator::new(epsilon, alpha)
                .expect("valid settings");
            let mut path_probability = 1.0_f64;
            let mut ever_excluded = false;

            for time in 0..horizon {
                let event = ((encoded_path >> time) & 1) == 1;
                path_probability *= if event { event_mean } else { 1.0 - event_mean };
                let observation = if event { 1.0 } else { 0.0 };
                match accumulator.observe_bounded_contribution(observation) {
                    Ok(report) => {
                        if event_mean < report.bounded_mean_lower
                            || event_mean > report.bounded_mean_upper
                        {
                            ever_excluded = true;
                        }
                    }
                    Err(KernelError::EmptyConfidenceSet) => ever_excluded = true,
                    Err(error) => panic!("unexpected CS update error: {error}"),
                }
            }

            if ever_excluded {
                rejected_path_probability += path_probability;
            }
        }

        assert!(
            rejected_path_probability <= alpha + 1.0e-12,
            "optional-stopping rejection probability {rejected_path_probability} exceeded alpha {alpha}"
        );
    }
}
