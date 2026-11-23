// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Streaming log-domain diagnostics for independent importance-sampling
//! evaluation draws.
//!
//! These summaries are plug-in diagnostics. They do not provide finite-sample
//! guarantees for heavy-tailed importance weights, prove a finite second
//! moment, or replace independent repeated-run coverage experiments.

use crate::{KernelError, log_importance_weight};

/// A streaming accumulator for nonnegative importance-sampling event contributions.
///
/// Each evaluation contributes either zero or exp(log_contribution). The
/// accumulator stores the first and second raw sums in the log domain so very
/// small event probabilities and large likelihood ratios need not be
/// exponentiated during accumulation.
///
/// Samples must come from the declared frozen proposal and must be
/// independently evaluated. This type does not verify that statistical
/// assumption or the proposal-support condition.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightedEventAccumulator {
    sample_count: u64,
    nonzero_contribution_count: u64,
    log_sum: f64,
    log_sum_squares: f64,
}

impl Default for WeightedEventAccumulator {
    fn default() -> Self {
        Self::new()
    }
}

impl WeightedEventAccumulator {
    /// Create an empty accumulator.
    pub fn new() -> Self {
        Self {
            sample_count: 0,
            nonzero_contribution_count: 0,
            log_sum: f64::NEG_INFINITY,
            log_sum_squares: f64::NEG_INFINITY,
        }
    }

    /// Add one independent evaluation contribution.
    ///
    /// None means a zero event contribution. A finite Some(value) means the
    /// contribution is exp(value). Negative infinity is also treated as zero;
    /// NaN and positive infinity are rejected.
    pub fn push_log_contribution(
        &mut self,
        log_contribution: Option<f64>,
    ) -> Result<(), KernelError> {
        let next_sample_count = self
            .sample_count
            .checked_add(1)
            .ok_or(KernelError::CounterOverflow)?;

        if let Some(log_value) = log_contribution {
            if log_value.is_nan() || log_value == f64::INFINITY {
                return Err(KernelError::NonFiniteValue);
            }
            if log_value != f64::NEG_INFINITY {
                let doubled_log_value = 2.0 * log_value;
                if !doubled_log_value.is_finite() {
                    return Err(KernelError::NumericalOverflow);
                }

                let next_nonzero_count = self
                    .nonzero_contribution_count
                    .checked_add(1)
                    .ok_or(KernelError::CounterOverflow)?;
                let next_log_sum = log_add_exp(self.log_sum, log_value)?;
                let next_log_sum_squares = log_add_exp(self.log_sum_squares, doubled_log_value)?;

                self.nonzero_contribution_count = next_nonzero_count;
                self.log_sum = next_log_sum;
                self.log_sum_squares = next_log_sum_squares;
            }
        }

        self.sample_count = next_sample_count;
        Ok(())
    }

    /// Add one event contribution from nominal and proposal log densities.
    pub fn push_importance_sample(
        &mut self,
        event_occurred: bool,
        log_target_density: f64,
        log_proposal_density: f64,
    ) -> Result<(), KernelError> {
        let log_weight = log_importance_weight(log_target_density, log_proposal_density)?;
        if event_occurred {
            self.push_log_contribution(Some(log_weight))
        } else {
            self.push_log_contribution(None)
        }
    }

    /// Number of independent evaluation draws added.
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Number of draws with nonzero event contributions.
    pub fn nonzero_contribution_count(&self) -> u64 {
        self.nonzero_contribution_count
    }

    /// Return the current log-domain summary.
    pub fn summary(&self) -> Result<WeightedEventSummary, KernelError> {
        if self.sample_count == 0 {
            return Ok(WeightedEventSummary {
                sample_count: 0,
                nonzero_contribution_count: 0,
                log_estimate: None,
                log_contribution_effective_sample_size: None,
                log_relative_standard_error: None,
                normal_approximation_95_log_lower: None,
                normal_approximation_95_log_upper: None,
            });
        }

        if self.log_sum == f64::NEG_INFINITY {
            return Ok(WeightedEventSummary {
                sample_count: self.sample_count,
                nonzero_contribution_count: 0,
                log_estimate: None,
                log_contribution_effective_sample_size: None,
                log_relative_standard_error: None,
                normal_approximation_95_log_lower: None,
                normal_approximation_95_log_upper: None,
            });
        }

        let log_sample_count = (self.sample_count as f64).ln();
        let log_estimate = self.log_sum - log_sample_count;
        if !log_estimate.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }

        let Some(log_contribution_effective_sample_size) =
            self.log_contribution_effective_sample_size()?
        else {
            return Ok(WeightedEventSummary {
                sample_count: self.sample_count,
                nonzero_contribution_count: self.nonzero_contribution_count,
                log_estimate: Some(log_estimate),
                log_contribution_effective_sample_size: None,
                log_relative_standard_error: None,
                normal_approximation_95_log_lower: None,
                normal_approximation_95_log_upper: None,
            });
        };

        if self.sample_count < 2 {
            return Ok(WeightedEventSummary {
                sample_count: self.sample_count,
                nonzero_contribution_count: self.nonzero_contribution_count,
                log_estimate: Some(log_estimate),
                log_contribution_effective_sample_size: Some(
                    log_contribution_effective_sample_size,
                ),
                log_relative_standard_error: None,
                normal_approximation_95_log_lower: None,
                normal_approximation_95_log_upper: None,
            });
        }

        let log_correction = 2.0 * self.log_sum - log_sample_count;
        if !log_correction.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        let log_centered_sum_squares = log_difference_exp(self.log_sum_squares, log_correction)?;
        let log_standard_error = if log_centered_sum_squares == f64::NEG_INFINITY {
            f64::NEG_INFINITY
        } else {
            let log_denominator =
                (self.sample_count as f64).ln() + ((self.sample_count - 1) as f64).ln();
            0.5 * (log_centered_sum_squares - log_denominator)
        };

        let log_relative_standard_error = log_standard_error - log_estimate;
        let log_95_margin = 1.96_f64.ln() + log_relative_standard_error;

        let normal_approximation_95_log_lower = if log_95_margin >= 0.0 {
            None
        } else {
            let relative_margin = log_95_margin.exp();
            let log_lower = log_estimate + (-relative_margin).ln_1p();
            Some(log_lower.min(0.0))
        };

        let upper_log = if log_95_margin == f64::NEG_INFINITY {
            log_estimate
        } else if log_95_margin > 709.0 {
            0.0
        } else {
            let log_upper = log_estimate + log_95_margin.exp().ln_1p();
            log_upper.min(0.0)
        };

        Ok(WeightedEventSummary {
            sample_count: self.sample_count,
            nonzero_contribution_count: self.nonzero_contribution_count,
            log_estimate: Some(log_estimate),
            log_contribution_effective_sample_size: Some(log_contribution_effective_sample_size),
            log_relative_standard_error: Some(log_relative_standard_error),
            normal_approximation_95_log_lower,
            normal_approximation_95_log_upper: Some(upper_log),
        })
    }

    fn log_contribution_effective_sample_size(&self) -> Result<Option<f64>, KernelError> {
        if self.nonzero_contribution_count == 0 {
            return Ok(None);
        }
        let log_ess = 2.0 * self.log_sum - self.log_sum_squares;
        if !log_ess.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        Ok(Some(
            log_ess.min((self.nonzero_contribution_count as f64).ln()),
        ))
    }
}

/// Log-domain diagnostics for one independent importance-sampling evaluation run.
///
/// The effective sample size here is specifically the ESS of nonzero weighted
/// event contributions. It is not the ordinary ESS of all proposal weights.
/// The normal-approximation interval is a diagnostic only and is not reliable
/// under arbitrary heavy tails, dependence, or proposal adaptation.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightedEventSummary {
    /// Number of independent evaluation draws.
    pub sample_count: u64,
    /// Number of draws with a nonzero weighted event contribution.
    pub nonzero_contribution_count: u64,
    /// Log of the arithmetic-mean importance-sampling estimate. None means no
    /// nonzero event contribution was observed; it does not mean probability
    /// zero or provide an upper confidence bound.
    pub log_estimate: Option<f64>,
    /// Log ESS of nonzero weighted event contributions.
    pub log_contribution_effective_sample_size: Option<f64>,
    /// Log of the plug-in relative standard error. None means it could not be
    /// estimated because no event contribution or too few samples were seen.
    pub log_relative_standard_error: Option<f64>,
    /// Log lower endpoint of a normal-approximation 95% interval. None denotes
    /// a zero lower endpoint or an unavailable interval.
    pub normal_approximation_95_log_lower: Option<f64>,
    /// Log upper endpoint of a normal-approximation 95% interval.
    pub normal_approximation_95_log_upper: Option<f64>,
}

fn log_add_exp(left: f64, right: f64) -> Result<f64, KernelError> {
    if left == f64::NEG_INFINITY {
        return Ok(right);
    }
    if right == f64::NEG_INFINITY {
        return Ok(left);
    }
    if !left.is_finite() || !right.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }
    let maximum = left.max(right);
    let result = maximum + ((left - maximum).exp() + (right - maximum).exp()).ln();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

fn log_difference_exp(log_larger: f64, log_smaller: f64) -> Result<f64, KernelError> {
    if log_larger.is_nan() || log_smaller.is_nan() || log_smaller > log_larger {
        return Err(KernelError::NumericalOverflow);
    }
    if log_larger == log_smaller {
        return Ok(f64::NEG_INFINITY);
    }
    if log_smaller == f64::NEG_INFINITY {
        return Ok(log_larger);
    }
    let result = log_larger + (-(log_smaller - log_larger).exp()).ln_1p();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_accumulator_has_no_estimate_or_interval() {
        let summary = WeightedEventAccumulator::new().summary().unwrap();
        assert_eq!(summary.sample_count, 0);
        assert_eq!(summary.log_estimate, None);
        assert_eq!(summary.normal_approximation_95_log_upper, None);
    }

    #[test]
    fn zero_observed_event_contributions_are_not_reported_as_zero_probability() {
        let mut accumulator = WeightedEventAccumulator::new();
        for _ in 0..100 {
            accumulator.push_log_contribution(None).unwrap();
        }
        let summary = accumulator.summary().unwrap();
        assert_eq!(summary.sample_count, 100);
        assert_eq!(summary.nonzero_contribution_count, 0);
        assert_eq!(summary.log_estimate, None);
        assert_eq!(summary.normal_approximation_95_log_upper, None);
    }

    #[test]
    fn one_event_contribution_does_not_claim_an_interval() {
        let mut accumulator = WeightedEventAccumulator::new();
        accumulator.push_log_contribution(Some(0.0)).unwrap();
        let summary = accumulator.summary().unwrap();
        assert_eq!(summary.log_estimate, Some(0.0));
        assert_eq!(summary.log_relative_standard_error, None);
        assert_eq!(summary.normal_approximation_95_log_upper, None);
    }

    #[test]
    fn log_moments_preserve_extreme_small_contributions() {
        let mut accumulator = WeightedEventAccumulator::new();
        accumulator.push_log_contribution(Some(-1000.0)).unwrap();
        accumulator.push_log_contribution(None).unwrap();
        let summary = accumulator.summary().unwrap();
        assert!((summary.log_estimate.unwrap() - (-1000.0 - 2.0_f64.ln())).abs() < 1.0e-12);
        assert!(summary.log_relative_standard_error.unwrap().abs() < 1.0e-12);
    }

    #[test]
    fn contribution_ess_is_two_for_two_equal_nonzero_draws() {
        let mut accumulator = WeightedEventAccumulator::new();
        accumulator.push_log_contribution(Some(-2.0)).unwrap();
        accumulator.push_log_contribution(Some(-2.0)).unwrap();
        let summary = accumulator.summary().unwrap();
        assert!(
            (summary.log_contribution_effective_sample_size.unwrap() - 2.0_f64.ln()).abs()
                < 1.0e-12
        );
        assert_eq!(summary.log_relative_standard_error, Some(f64::NEG_INFINITY));
    }

    #[test]
    fn unequal_contributions_produce_positive_relative_standard_error() {
        let mut accumulator = WeightedEventAccumulator::new();
        accumulator.push_log_contribution(Some(0.0)).unwrap();
        accumulator.push_log_contribution(Some(-2.0)).unwrap();
        accumulator.push_log_contribution(None).unwrap();
        let summary = accumulator.summary().unwrap();
        assert!(summary.log_relative_standard_error.unwrap().is_finite());
        assert!(summary.log_contribution_effective_sample_size.unwrap() <= 2.0_f64.ln());
        assert!(summary.normal_approximation_95_log_upper.is_some());
    }

    #[test]
    fn importance_sample_path_uses_the_log_density_ratio() {
        let mut accumulator = WeightedEventAccumulator::new();
        accumulator
            .push_importance_sample(true, -3.0, -5.0)
            .unwrap();
        accumulator
            .push_importance_sample(false, -3.0, -5.0)
            .unwrap();
        let summary = accumulator.summary().unwrap();
        assert!((summary.log_estimate.unwrap() - (2.0 - 2.0_f64.ln())).abs() < 1.0e-12);
    }

    #[test]
    fn invalid_log_contribution_fails_without_mutating_count() {
        let mut accumulator = WeightedEventAccumulator::new();
        assert_eq!(
            accumulator.push_log_contribution(Some(f64::NAN)),
            Err(KernelError::NonFiniteValue)
        );
        assert_eq!(accumulator.sample_count(), 0);
    }

    #[test]
    fn zero_proposal_density_is_rejected_even_for_a_non_event_draw() {
        let mut accumulator = WeightedEventAccumulator::new();
        assert_eq!(
            accumulator.push_importance_sample(false, -3.0, f64::NEG_INFINITY),
            Err(KernelError::ProposalHasZeroDensity)
        );
        assert_eq!(accumulator.sample_count(), 0);
    }
    #[test]
    fn zero_proposal_density_fails_closed_for_an_event() {
        let mut accumulator = WeightedEventAccumulator::new();
        assert_eq!(
            accumulator.push_importance_sample(true, -3.0, f64::NEG_INFINITY),
            Err(KernelError::ProposalHasZeroDensity)
        );
        assert_eq!(accumulator.sample_count(), 0);
    }
}
