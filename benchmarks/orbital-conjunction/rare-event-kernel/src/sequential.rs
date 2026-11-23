// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Anytime-valid inference for bounded Bernoulli event observations.
//!
//! This module implements a beta(1,1)-mixture likelihood-ratio e-process for
//! testing a Bernoulli event probability and inverts it to produce a
//! time-uniform confidence sequence.
//!
//! The guarantee is specific to independent, identically distributed
//! Bernoulli observations with a fixed event probability. It does not apply
//! automatically to unbounded importance-sampling contributions, adaptive
//! proposals, dependent samples, or model-misspecified event labels.

use crate::KernelError;

/// A closed confidence interval at the current observation time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BernoulliConfidenceSequence {
    /// Number of Bernoulli observations used.
    pub sample_count: u64,
    /// Number of observed events.
    pub event_count: u64,
    /// Lower endpoint of the time-uniform confidence sequence.
    pub lower: f64,
    /// Upper endpoint of the time-uniform confidence sequence.
    pub upper: f64,
    /// One-sided error budget used by the e-process inversion.
    pub alpha: f64,
}

/// A beta(1,1)-mixture likelihood-ratio e-process for a Bernoulli event rate.
///
/// Under any fixed Bernoulli null probability p0, the mixture likelihood
/// ratio is a nonnegative martingale with initial value one. Ville's
/// inequality therefore controls the probability that it ever crosses
/// 1/alpha. Inverting this family of tests yields a confidence sequence
/// whose coverage holds simultaneously over all observation times.
///
/// The alternative mixture is beta(1,1), i.e. uniform on [0,1]. The
/// implementation accumulates the mixture predictive likelihood one
/// observation at a time, avoiding large gamma-function differences.
#[derive(Debug, Clone, PartialEq)]
pub struct BernoulliMixtureEProcess {
    sample_count: u64,
    event_count: u64,
    log_mixture_likelihood: f64,
}

impl Default for BernoulliMixtureEProcess {
    fn default() -> Self {
        Self::new()
    }
}

impl BernoulliMixtureEProcess {
    /// Create an empty Bernoulli mixture e-process.
    pub fn new() -> Self {
        Self {
            sample_count: 0,
            event_count: 0,
            log_mixture_likelihood: 0.0,
        }
    }

    /// Add one independent Bernoulli observation.
    ///
    /// true denotes an event and false denotes a non-event. The update is
    /// transactional: invalid counters or numerical updates do not mutate
    /// the accumulator.
    pub fn observe(&mut self, event_occurred: bool) -> Result<(), KernelError> {
        let next_sample_count = self
            .sample_count
            .checked_add(1)
            .ok_or(KernelError::CounterOverflow)?;
        let next_event_count = if event_occurred {
            self.event_count
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?
        } else {
            self.event_count
        };
        let failures_before = self.sample_count - self.event_count;
        let predictive_numerator = if event_occurred {
            self.event_count + 1
        } else {
            failures_before + 1
        };
        let predictive_denominator = self
            .sample_count
            .checked_add(2)
            .ok_or(KernelError::CounterOverflow)?;

        let increment = (predictive_numerator as f64).ln() - (predictive_denominator as f64).ln();
        let next_log_mixture_likelihood = self.log_mixture_likelihood + increment;
        if !next_log_mixture_likelihood.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }

        self.sample_count = next_sample_count;
        self.event_count = next_event_count;
        self.log_mixture_likelihood = next_log_mixture_likelihood;
        Ok(())
    }

    /// Number of observations accumulated.
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Number of observed event indicators.
    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    /// Evaluate the log e-value against a fixed Bernoulli null probability.
    ///
    /// A value greater than ln(1/alpha) is evidence against the null at
    /// anytime-valid level alpha when monitored over an arbitrary number of
    /// observations. Probabilities zero and one are handled exactly when
    /// the observed path is compatible; an impossible null path returns
    /// positive infinity.
    pub fn log_e_value(&self, null_probability: f64) -> Result<f64, KernelError> {
        validate_probability(null_probability)?;

        if self.sample_count == 0 {
            return Ok(0.0);
        }

        if null_probability == 0.0 {
            return if self.event_count > 0 {
                Ok(f64::INFINITY)
            } else {
                Ok(self.log_mixture_likelihood)
            };
        }

        if null_probability == 1.0 {
            return if self.event_count < self.sample_count {
                Ok(f64::INFINITY)
            } else {
                Ok(self.log_mixture_likelihood)
            };
        }

        let failures = self.sample_count - self.event_count;
        let null_log_likelihood = (self.event_count as f64) * null_probability.ln()
            + (failures as f64) * (-null_probability).ln_1p();
        let log_e = self.log_mixture_likelihood - null_log_likelihood;
        if log_e.is_finite() {
            Ok(log_e)
        } else {
            Err(KernelError::NumericalOverflow)
        }
    }

    /// Construct a time-uniform (1-alpha) confidence sequence at the current
    /// observation time.
    ///
    /// The interval is obtained by inverting the beta(1,1)-mixture e-process.
    /// The endpoints are computed by bisection and widened outward by one
    /// representable f64 value. This endpoint rounding is a numerical guard,
    /// not a formal proof of all floating-point operations used by the
    /// e-process.
    pub fn confidence_sequence(
        &self,
        alpha: f64,
    ) -> Result<BernoulliConfidenceSequence, KernelError> {
        validate_alpha(alpha)?;
        if self.sample_count == 0 {
            return Ok(BernoulliConfidenceSequence {
                sample_count: 0,
                event_count: 0,
                lower: 0.0,
                upper: 1.0,
                alpha,
            });
        }

        let threshold = -alpha.ln();
        let empirical_mean = self.event_count as f64 / self.sample_count as f64;
        let minimum_log_e = self.log_e_value(empirical_mean)?;
        if minimum_log_e > threshold {
            return Err(KernelError::NumericalOverflow);
        }

        let lower = if self.event_count == 0 {
            0.0
        } else {
            let mut excluded = 0.0;
            let mut included = empirical_mean;
            for _ in 0..80 {
                let midpoint = 0.5 * excluded + 0.5 * included;
                if midpoint <= excluded || midpoint >= included {
                    break;
                }
                if self.log_e_value(midpoint)? > threshold {
                    excluded = midpoint;
                } else {
                    included = midpoint;
                }
            }
            next_down(excluded).max(0.0)
        };

        let upper = if self.event_count == self.sample_count {
            1.0
        } else {
            let mut included = empirical_mean;
            let mut excluded = 1.0;
            for _ in 0..80 {
                let midpoint = 0.5 * included + 0.5 * excluded;
                if midpoint <= included || midpoint >= excluded {
                    break;
                }
                if self.log_e_value(midpoint)? > threshold {
                    excluded = midpoint;
                } else {
                    included = midpoint;
                }
            }
            next_up(excluded).min(1.0)
        };

        if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || upper > 1.0 || lower > upper {
            return Err(KernelError::NumericalOverflow);
        }

        Ok(BernoulliConfidenceSequence {
            sample_count: self.sample_count,
            event_count: self.event_count,
            lower,
            upper,
            alpha,
        })
    }
}

fn validate_probability(probability: f64) -> Result<(), KernelError> {
    if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        Err(KernelError::InvalidProbability)
    } else {
        Ok(())
    }
}

fn validate_alpha(alpha: f64) -> Result<(), KernelError> {
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        Err(KernelError::InvalidAlpha)
    } else {
        Ok(())
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
    fn empty_process_has_full_confidence_sequence() {
        let process = BernoulliMixtureEProcess::new();
        let sequence = process.confidence_sequence(0.05).unwrap();
        assert_eq!(sequence.sample_count, 0);
        assert_eq!(sequence.event_count, 0);
        assert_eq!(sequence.lower, 0.0);
        assert_eq!(sequence.upper, 1.0);
    }

    #[test]
    fn one_success_log_e_value_matches_beta_mixture_likelihood_ratio() {
        let mut process = BernoulliMixtureEProcess::new();
        process.observe(true).unwrap();
        assert!((process.log_e_value(0.1).unwrap() - 5.0_f64.ln()).abs() < 1.0e-14);
    }

    #[test]
    fn success_then_failure_mixture_likelihood_is_one_sixth() {
        let mut process = BernoulliMixtureEProcess::new();
        process.observe(true).unwrap();
        process.observe(false).unwrap();
        assert!((process.log_mixture_likelihood - (1.0_f64 / 6.0).ln()).abs() < 1.0e-14);
        assert!((process.log_e_value(0.25).unwrap() - (8.0_f64 / 9.0).ln()).abs() < 1.0e-14);
    }

    #[test]
    fn incompatible_endpoint_nulls_have_infinite_e_value() {
        let mut process = BernoulliMixtureEProcess::new();
        process.observe(true).unwrap();
        assert_eq!(process.log_e_value(0.0).unwrap(), f64::INFINITY);
        assert!(process.log_e_value(1.0).unwrap().is_finite());

        let mut failures = BernoulliMixtureEProcess::new();
        failures.observe(false).unwrap();
        assert_eq!(failures.log_e_value(1.0).unwrap(), f64::INFINITY);
        assert!(failures.log_e_value(0.0).unwrap().is_finite());
    }

    #[test]
    fn confidence_sequence_contains_the_empirical_mean() {
        let mut process = BernoulliMixtureEProcess::new();
        for event in [
            true, false, true, false, true, false, false, true, false, false,
        ] {
            process.observe(event).unwrap();
        }
        let sequence = process.confidence_sequence(0.05).unwrap();
        let empirical_mean = 4.0 / 10.0;
        assert!(sequence.lower <= empirical_mean);
        assert!(sequence.upper >= empirical_mean);
    }

    #[test]
    fn confidence_sequence_respects_probability_endpoints() {
        let mut all_events = BernoulliMixtureEProcess::new();
        let mut no_events = BernoulliMixtureEProcess::new();
        for _ in 0..100 {
            all_events.observe(true).unwrap();
            no_events.observe(false).unwrap();
        }
        let all = all_events.confidence_sequence(0.05).unwrap();
        let none = no_events.confidence_sequence(0.05).unwrap();
        assert_eq!(all.upper, 1.0);
        assert_eq!(none.lower, 0.0);
        assert!(all.lower > 0.0);
        assert!(none.upper < 1.0);
    }

    #[test]
    fn confidence_sequence_narrows_for_a_long_all_failure_path() {
        let mut short = BernoulliMixtureEProcess::new();
        let mut long = BernoulliMixtureEProcess::new();
        for _ in 0..10 {
            short.observe(false).unwrap();
        }
        for _ in 0..100 {
            long.observe(false).unwrap();
        }
        assert!(
            long.confidence_sequence(0.05).unwrap().upper
                < short.confidence_sequence(0.05).unwrap().upper
        );
    }

    #[test]
    fn invalid_alpha_and_probability_are_rejected() {
        let process = BernoulliMixtureEProcess::new();
        assert_eq!(
            process.confidence_sequence(0.0),
            Err(KernelError::InvalidAlpha)
        );
        assert_eq!(
            process.confidence_sequence(1.0),
            Err(KernelError::InvalidAlpha)
        );
        assert_eq!(
            process.log_e_value(f64::NAN),
            Err(KernelError::InvalidProbability)
        );
    }

    #[test]
    fn exhaustive_optional_stopping_coverage_for_short_horizon() {
        let event_probability = 0.2_f64;
        let alpha = 0.05_f64;
        let log_threshold = (1.0 / alpha).ln();
        let horizon = 14_u32;
        let mut rejected_path_probability = 0.0_f64;

        for encoded_path in 0_u32..(1_u32 << horizon) {
            let mut process = BernoulliMixtureEProcess::new();
            let mut path_successes = 0_u32;
            let mut ever_rejected = false;

            for time in 0..horizon {
                let event = ((encoded_path >> time) & 1) == 1;
                if event {
                    path_successes += 1;
                }
                process.observe(event).unwrap();
                if process.log_e_value(event_probability).unwrap() >= log_threshold {
                    ever_rejected = true;
                }
            }

            if ever_rejected {
                let path_failures = horizon - path_successes;
                let path_probability = event_probability.powi(path_successes as i32)
                    * (1.0 - event_probability).powi(path_failures as i32);
                rejected_path_probability += path_probability;
            }
        }

        assert!(
            rejected_path_probability <= alpha + 1.0e-12,
            "time-uniform rejection probability {rejected_path_probability} exceeds alpha {alpha}"
        );
    }

    #[test]
    fn counter_overflow_is_fail_closed() {
        let mut process = BernoulliMixtureEProcess {
            sample_count: u64::MAX,
            event_count: 0,
            log_mixture_likelihood: 0.0,
        };
        assert_eq!(process.observe(false), Err(KernelError::CounterOverflow));
        assert_eq!(process.sample_count(), u64::MAX);
    }
}
