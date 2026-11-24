// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Hedged-capital confidence sequence for bounded conditional-mean processes.
//!
//! Implements the max-capital confidence sequence of Waudby-Smith and Ramdas
//! (JRSS B, 2024, Theorem 3, equations 25-27). Two directional capital
//! processes are maintained for each candidate mean m, and the hedged capital
//! is their weighted maximum. The theorem establishes that the sublevel set
//! is an interval-valued time-uniform confidence sequence.
//!
//! This implementation evaluates the sublevel set on a caller-supplied finite
//! parameter grid. If accepted grid points exist, the first and last accepted
//! points are expanded to their immediately adjacent grid points. Since the
//! exact sublevel set is an interval, this outer bracket contains the exact
//! set. If no grid point is accepted, the implementation abstains to [0,1].
//! Grid resolution is therefore an explicit computational trade-off.
//!
//! Validity assumes X_t in [0,1] with a fixed conditional mean
//! E[X_t | F_(t-1)] = mu. Independence is not required. The default predictable
//! stake follows the PrPl-H rule used with the hedged process:
//! sqrt(2 log(2/alpha) / (sigma_hat_(t-1)^2 * t * log(1+t))).
//!
//! Reference: https://doi.org/10.1093/jrsssb/qkad009
//! Theorem 3 and equations (25)-(27).

use crate::adaptive_importance_sampling::BoundedMeanConfidenceSequence;

/// Default cap c for the parameter-dependent hedged stakes.
pub const DEFAULT_HEDGED_STAKE_CAP: f64 = 0.5;

/// Errors in the hedged sequence configuration or numeric update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HedgedConfidenceSequenceError {
    /// Confidence error probability must lie strictly in (0,1).
    InvalidAlpha,
    /// Stake cap c must lie in [0,1).
    InvalidStakeCap,
    /// Hedge weight theta must lie strictly in (0,1).
    InvalidHedgeWeight,
    /// Parameter grid must contain at least three strictly increasing values from 0 to 1.
    InvalidParameterGrid,
    /// Observation must be finite and lie in [0,1].
    InvalidObservation,
    /// Observation counter overflowed.
    CounterOverflow,
    /// Hedged capital became non-finite or negative.
    NumericalFailure,
    /// Accepted grid points were not contiguous, contradicting the interval property.
    NonContiguousAcceptedGrid,
    /// Running intersection became empty.
    EmptyConfidenceSet,
}

impl core::fmt::Display for HedgedConfidenceSequenceError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidAlpha => "confidence alpha must lie strictly between zero and one",
            Self::InvalidStakeCap => "hedged stake cap must lie in [0, 1)",
            Self::InvalidHedgeWeight => "hedge weight must lie strictly between zero and one",
            Self::InvalidParameterGrid => {
                "parameter grid must be strictly increasing from zero to one"
            }
            Self::InvalidObservation => "bounded-mean observation must lie in [0, 1]",
            Self::CounterOverflow => "bounded-mean observation counter overflowed",
            Self::NumericalFailure => "hedged capital update failed numerically",
            Self::NonContiguousAcceptedGrid => {
                "accepted hedged parameter-grid points are not contiguous"
            }
            Self::EmptyConfidenceSet => "running hedged confidence-set intersection is empty",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HedgedConfidenceSequenceError {}

/// Streaming hedged-capital confidence sequence over a fixed parameter grid.
///
/// Update time and memory are O(G), where G is the number of candidate means.
/// The grid is immutable for the life of the process. Capital is stored in
/// the log domain to avoid product overflow and underflow.
#[derive(Debug, Clone, PartialEq)]
pub struct HedgedCapitalConfidenceSequence {
    alpha: f64,
    stake_cap: f64,
    hedge_weight: f64,
    parameter_grid: Vec<f64>,
    log_capital_plus: Vec<f64>,
    log_capital_minus: Vec<f64>,
    sample_count: u64,
    sum_observations: f64,
    predictive_mean: f64,
    predictive_variance: f64,
    lower: f64,
    upper: f64,
    resolution_limited: bool,
    accepted_grid_point_count: usize,
}

impl HedgedCapitalConfidenceSequence {
    /// Construct a hedged-capital sequence with default c=0.5 and theta=0.5.
    pub fn new(
        alpha: f64,
        parameter_grid: Vec<f64>,
    ) -> Result<Self, HedgedConfidenceSequenceError> {
        Self::with_parameters(alpha, parameter_grid, DEFAULT_HEDGED_STAKE_CAP, 0.5)
    }

    /// Construct a hedged-capital sequence with explicit stake cap and capital split.
    pub fn with_parameters(
        alpha: f64,
        parameter_grid: Vec<f64>,
        stake_cap: f64,
        hedge_weight: f64,
    ) -> Result<Self, HedgedConfidenceSequenceError> {
        if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
            return Err(HedgedConfidenceSequenceError::InvalidAlpha);
        }
        if !stake_cap.is_finite() || !(0.0..1.0).contains(&stake_cap) {
            return Err(HedgedConfidenceSequenceError::InvalidStakeCap);
        }
        if !hedge_weight.is_finite() || hedge_weight <= 0.0 || hedge_weight >= 1.0 {
            return Err(HedgedConfidenceSequenceError::InvalidHedgeWeight);
        }
        if parameter_grid.len() < 3
            || parameter_grid.first() != Some(&0.0)
            || parameter_grid.last() != Some(&1.0)
            || parameter_grid
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            || parameter_grid.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(HedgedConfidenceSequenceError::InvalidParameterGrid);
        }

        let grid_len = parameter_grid.len();
        Ok(Self {
            alpha,
            stake_cap,
            hedge_weight,
            parameter_grid,
            log_capital_plus: vec![0.0; grid_len],
            log_capital_minus: vec![0.0; grid_len],
            sample_count: 0,
            sum_observations: 0.0,
            predictive_mean: 0.5,
            predictive_variance: 0.25,
            lower: 0.0,
            upper: 1.0,
            resolution_limited: false,
            accepted_grid_point_count: grid_len,
        })
    }

    /// Generate a grid with geometric tails and a linearly spaced central region.
    ///
    /// The lower tail runs from tail_floor to tail_ceiling, the central region
    /// runs from tail_ceiling to 1-tail_ceiling, and the upper tail mirrors
    /// the lower tail. Exact endpoints 0 and 1 are included.
    pub fn log_tailed_parameter_grid(
        tail_floor: f64,
        tail_ceiling: f64,
        tail_steps: usize,
        central_steps: usize,
    ) -> Result<Vec<f64>, HedgedConfidenceSequenceError> {
        if !tail_floor.is_finite()
            || !tail_ceiling.is_finite()
            || tail_floor <= 0.0
            || tail_floor >= tail_ceiling
            || tail_ceiling >= 0.5
            || tail_steps < 2
            || central_steps < 2
        {
            return Err(HedgedConfidenceSequenceError::InvalidParameterGrid);
        }

        let log_floor = tail_floor.ln();
        let log_ceiling = tail_ceiling.ln();
        let mut lower_tail = Vec::with_capacity(tail_steps + 1);
        for index in 0..=tail_steps {
            let fraction = index as f64 / tail_steps as f64;
            lower_tail.push((log_floor + fraction * (log_ceiling - log_floor)).exp());
        }

        let mut grid = Vec::with_capacity(2 * tail_steps + central_steps + 3);
        grid.push(0.0);
        grid.extend(lower_tail.iter().copied());

        for index in 1..=central_steps {
            let fraction = index as f64 / central_steps as f64;
            grid.push(tail_ceiling + fraction * (1.0 - 2.0 * tail_ceiling));
        }

        for value in lower_tail.iter().rev().skip(1) {
            grid.push(1.0 - value);
        }
        grid.push(1.0);

        if grid.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(HedgedConfidenceSequenceError::InvalidParameterGrid);
        }
        Ok(grid)
    }

    /// Add one bounded observation and update the running hedged confidence sequence.
    ///
    /// The update is transactional: no state is mutated unless all candidate
    /// capital updates, grid checks and variance calculations succeed.
    pub fn observe(&mut self, observation: f64) -> Result<(), HedgedConfidenceSequenceError> {
        if !observation.is_finite() || !(0.0..=1.0).contains(&observation) {
            return Err(HedgedConfidenceSequenceError::InvalidObservation);
        }
        let next_count = self
            .sample_count
            .checked_add(1)
            .ok_or(HedgedConfidenceSequenceError::CounterOverflow)?;
        let time = next_count as f64;
        if !time.is_finite() || time <= 0.0 {
            return Err(HedgedConfidenceSequenceError::NumericalFailure);
        }

        // Predictable PrPl-H stake. Any predictable stake is valid when the
        // parameter-dependent caps preserve nonnegative wealth; this choice
        // adapts to the estimated variance and horizon.
        let log_budget = 2.0_f64.ln() - self.alpha.ln();
        let denominator = self.predictive_variance * time * time.ln_1p();
        let raw_stake = (2.0 * log_budget / denominator).sqrt();
        if !raw_stake.is_finite() || raw_stake <= 0.0 {
            return Err(HedgedConfidenceSequenceError::NumericalFailure);
        }

        let log_theta = self.hedge_weight.ln();
        let log_one_minus_theta = (-self.hedge_weight).ln_1p();
        let log_threshold = -self.alpha.ln();
        let mut next_log_plus = Vec::with_capacity(self.parameter_grid.len());
        let mut next_log_minus = Vec::with_capacity(self.parameter_grid.len());
        let mut first_accepted = None;
        let mut last_accepted = None;
        let mut accepted_count = 0_usize;

        for (index, mean) in self.parameter_grid.iter().copied().enumerate() {
            let positive_cap = if mean == 0.0 {
                f64::INFINITY
            } else {
                self.stake_cap / mean
            };
            let negative_cap = if mean == 1.0 {
                f64::INFINITY
            } else {
                self.stake_cap / (1.0 - mean)
            };
            let positive_stake = raw_stake.min(positive_cap);
            let negative_stake = raw_stake.min(negative_cap);

            let positive_factor = 1.0 + positive_stake * (observation - mean);
            let negative_factor = 1.0 - negative_stake * (observation - mean);
            if !positive_factor.is_finite()
                || !negative_factor.is_finite()
                || positive_factor <= 0.0
                || negative_factor <= 0.0
            {
                return Err(HedgedConfidenceSequenceError::NumericalFailure);
            }

            let plus = self.log_capital_plus[index] + positive_factor.ln();
            let minus = self.log_capital_minus[index] + negative_factor.ln();
            if !plus.is_finite() || !minus.is_finite() {
                return Err(HedgedConfidenceSequenceError::NumericalFailure);
            }
            next_log_plus.push(plus);
            next_log_minus.push(minus);

            let log_hedged_capital = (log_theta + plus).max(log_one_minus_theta + minus);
            if log_hedged_capital < log_threshold {
                first_accepted.get_or_insert(index);
                last_accepted = Some(index);
                accepted_count = accepted_count
                    .checked_add(1)
                    .ok_or(HedgedConfidenceSequenceError::CounterOverflow)?;
            }
        }

        let (current_lower, current_upper, resolution_limited) =
            if let (Some(first), Some(last)) = (first_accepted, last_accepted) {
                if accepted_count != last - first + 1 {
                    return Err(HedgedConfidenceSequenceError::NonContiguousAcceptedGrid);
                }
                let lower_index = first.saturating_sub(1);
                let upper_index = (last + 1).min(self.parameter_grid.len() - 1);
                (
                    self.parameter_grid[lower_index],
                    self.parameter_grid[upper_index],
                    first == 0 || last + 1 == self.parameter_grid.len(),
                )
            } else {
                (0.0, 1.0, true)
            };

        let next_lower = self.lower.max(current_lower);
        let next_upper = self.upper.min(current_upper);
        if !next_lower.is_finite() || !next_upper.is_finite() || next_lower > next_upper {
            return Err(HedgedConfidenceSequenceError::EmptyConfidenceSet);
        }

        let next_sum = self.sum_observations + observation;
        let next_predictive_mean = (0.5 + next_sum) / (time + 1.0);
        let previous_residual_sum =
            self.predictive_variance * (self.sample_count as f64 + 1.0) - 0.25;
        let residual = observation - next_predictive_mean;
        let next_predictive_variance =
            (0.25 + previous_residual_sum + residual * residual) / (time + 1.0);
        if !next_sum.is_finite()
            || !next_predictive_mean.is_finite()
            || !(0.0..=1.0).contains(&next_predictive_mean)
            || !next_predictive_variance.is_finite()
            || next_predictive_variance <= 0.0
        {
            return Err(HedgedConfidenceSequenceError::NumericalFailure);
        }

        self.log_capital_plus = next_log_plus;
        self.log_capital_minus = next_log_minus;
        self.sample_count = next_count;
        self.sum_observations = next_sum;
        self.predictive_mean = next_predictive_mean;
        self.predictive_variance = next_predictive_variance;
        self.lower = next_lower;
        self.upper = next_upper;
        self.resolution_limited = resolution_limited;
        self.accepted_grid_point_count = accepted_count;
        Ok(())
    }

    /// Number of accepted observations.
    pub fn sample_count(&self) -> u64 {
        self.sample_count
    }

    /// Number of candidate means accepted at the latest update.
    pub fn accepted_grid_point_count(&self) -> usize {
        self.accepted_grid_point_count
    }

    /// Whether the latest update used the full domain or touched a grid boundary.
    pub fn resolution_limited(&self) -> bool {
        self.resolution_limited
    }

    /// Number of candidate means in the immutable parameter grid.
    pub fn grid_size(&self) -> usize {
        self.parameter_grid.len()
    }

    /// Current predictable mean estimate used for the next stake.
    pub fn predictive_mean(&self) -> f64 {
        self.predictive_mean
    }

    /// Current predictable variance estimate used for the next stake.
    pub fn predictive_variance(&self) -> f64 {
        self.predictive_variance
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
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVENT_PROBABILITY: f64 = 0.2;
    const DEFENSIVE_WEIGHT: f64 = 0.1;

    fn grid() -> Vec<f64> {
        HedgedCapitalConfidenceSequence::log_tailed_parameter_grid(1.0e-8, 0.02, 128, 128).unwrap()
    }

    #[test]
    fn empty_hedged_sequence_is_the_unit_interval() {
        let sequence = HedgedCapitalConfidenceSequence::new(0.05, grid()).unwrap();
        let interval = sequence.confidence_sequence();
        assert_eq!(interval.sample_count, 0);
        assert_eq!(interval.lower, 0.0);
        assert_eq!(interval.upper, 1.0);
    }

    #[test]
    fn log_tailed_grid_is_strictly_ordered_and_covers_rare_and_central_means() {
        let values = grid();
        assert_eq!(values.first(), Some(&0.0));
        assert_eq!(values.last(), Some(&1.0));
        assert!(values.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(values.iter().any(|value| (*value - 1.0e-6).abs() < 1.0e-7));
        assert!(values.iter().any(|value| (*value - 0.5).abs() < 1.0e-12));
    }

    #[test]
    fn constant_mean_sequence_is_covered_and_contracts() {
        let mut sequence = HedgedCapitalConfidenceSequence::new(0.05, grid()).unwrap();
        for _ in 0..500 {
            sequence.observe(0.25).unwrap();
        }
        let interval = sequence.confidence_sequence();
        assert!(interval.lower <= 0.25);
        assert!(interval.upper >= 0.25);
        assert!(interval.upper - interval.lower < 0.1);
    }

    #[test]
    fn running_intersection_is_nested_over_time() {
        let mut sequence = HedgedCapitalConfidenceSequence::new(0.05, grid()).unwrap();
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
    fn exact_finite_support_adaptive_proposals_have_anytime_coverage() {
        // Target event probability 0.2, with q_t = a*p + (1-a)*r_t.
        // r_t(event)=0.8 after an event, else 0.05. The normalized weighted
        // event indicator has fixed conditional mean a*P(E)=0.02.
        const HORIZON: u32 = 6;
        let target_mean = DEFENSIVE_WEIGHT * EVENT_PROBABILITY;
        let mut total_miss_probability = 0.0;

        for encoded_path in 0_u32..(1_u32 << HORIZON) {
            let mut sequence = HedgedCapitalConfidenceSequence::new(0.05, grid()).unwrap();
            let mut path_probability = 1.0;
            let mut previous_event = false;
            let mut ever_missed = false;

            for time in 0..HORIZON {
                let residual_event_probability = if previous_event { 0.8 } else { 0.05 };
                let proposal_event_probability = DEFENSIVE_WEIGHT * EVENT_PROBABILITY
                    + (1.0 - DEFENSIVE_WEIGHT) * residual_event_probability;
                let event_occurred = ((encoded_path >> time) & 1) == 1;
                path_probability *= if event_occurred {
                    proposal_event_probability
                } else {
                    1.0 - proposal_event_probability
                };

                let observation = if event_occurred {
                    DEFENSIVE_WEIGHT * EVENT_PROBABILITY / proposal_event_probability
                } else {
                    0.0
                };
                sequence.observe(observation).unwrap();
                let interval = sequence.confidence_sequence();
                if interval.lower > target_mean || interval.upper < target_mean {
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
    fn all_zero_and_all_one_paths_preserve_boundary_coverage() {
        let mut zeros = HedgedCapitalConfidenceSequence::new(0.05, grid()).unwrap();
        let mut ones = HedgedCapitalConfidenceSequence::new(0.05, grid()).unwrap();
        for _ in 0..200 {
            zeros.observe(0.0).unwrap();
            ones.observe(1.0).unwrap();
        }
        let zero_interval = zeros.confidence_sequence();
        let one_interval = ones.confidence_sequence();
        assert_eq!(zero_interval.lower, 0.0);
        assert!(zero_interval.upper > 0.0);
        assert!(one_interval.lower < 1.0);
        assert_eq!(one_interval.upper, 1.0);
    }

    #[test]
    fn coarse_grid_never_returns_a_tighter_than_bracketed_interval() {
        let mut sequence = HedgedCapitalConfidenceSequence::new(0.05, vec![0.0, 0.5, 1.0]).unwrap();
        sequence.observe(0.25).unwrap();
        let interval = sequence.confidence_sequence();
        assert!(interval.lower <= 0.5);
        assert!(interval.upper >= 0.5);
        assert!(interval.lower <= interval.upper);
    }

    #[test]
    fn invalid_inputs_and_parameter_grids_fail_closed() {
        assert!(HedgedCapitalConfidenceSequence::new(0.0, grid()).is_err());
        assert!(HedgedCapitalConfidenceSequence::new(1.0, grid()).is_err());
        assert!(HedgedCapitalConfidenceSequence::with_parameters(0.05, grid(), -0.1, 0.5).is_err());
        assert!(HedgedCapitalConfidenceSequence::with_parameters(0.05, grid(), 0.5, 1.0).is_err());
        assert!(HedgedCapitalConfidenceSequence::new(0.05, vec![0.0, 0.5, 0.5, 1.0]).is_err());
        assert!(
            HedgedCapitalConfidenceSequence::log_tailed_parameter_grid(0.0, 0.02, 16, 16).is_err()
        );

        let mut sequence = HedgedCapitalConfidenceSequence::new(0.05, grid()).unwrap();
        assert_eq!(
            sequence.observe(f64::NAN),
            Err(HedgedConfidenceSequenceError::InvalidObservation)
        );
        assert_eq!(sequence.sample_count(), 0);
    }
}
