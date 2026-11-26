// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Simultaneous anytime-valid comparisons across paired portfolio arms.
//!
//! One PrPl-EB confidence sequence is maintained for each pair in a frozen
//! portfolio universe. A Bonferroni allocation of the family error budget
//! across all pairwise contrasts gives simultaneous coverage at every
//! observation time, provided each normalized contrast has a fixed
//! conditional mean and remains in [0, 1].
//!
//! This first implementation consumes direct, equally weighted paired
//! consequence rows from one fixed target distribution. It does not yet
//! implement importance weights, adaptive proposals, epistemic scenario
//! mixtures, or safety-gate integration. It cannot establish that the
//! supplied rows represent physically coupled orbital trajectories.

use std::collections::BTreeMap;

use crate::paired_counterfactual::MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS;
use crate::predictable_empirical_bernstein::{PrPlEbConfidenceSequence, PrPlEbError};

/// Maximum number of paired draws tracked by one process.
pub const MAX_SIMULTANEOUS_PORTFOLIO_DRAWS: u64 = 1_000_000;

/// Frozen lower and upper consequence bounds for one portfolio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioConsequenceBound {
    /// Stable, nonzero identity of the baseline or candidate portfolio.
    pub portfolio_id: [u8; 32],
    /// Inclusive minimum directly evaluated consequence.
    pub lower_ticks: i32,
    /// Inclusive maximum directly evaluated consequence.
    pub upper_ticks: i32,
}

/// One direct consequence observation for a portfolio in a paired draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioConsequenceObservation {
    /// Stable identity matching one frozen portfolio bound.
    pub portfolio_id: [u8; 32],
    /// Directly evaluated consequence for this portfolio and draw.
    pub consequence_ticks: i32,
}

/// One complete paired draw across the frozen portfolio universe.
///
/// draw_index is a monotone, one-based campaign sequence number. Replaying or
/// skipping a draw index is rejected. The consequence vector may arrive in
/// any order, but must contain every frozen portfolio exactly once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SimultaneousPortfolioDraw {
    /// Next expected one-based draw index, beginning at one.
    pub draw_index: u64,
    /// Direct consequence for every frozen portfolio arm.
    pub portfolio_consequences: Vec<PortfolioConsequenceObservation>,
}

/// Confidence interval for one oriented pairwise expected-consequence contrast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortfolioContrastConfidenceInterval {
    /// Lexicographically smaller portfolio identity (left arm).
    pub left_portfolio_id: [u8; 32],
    /// Lexicographically larger portfolio identity (right arm).
    pub right_portfolio_id: [u8; 32],
    /// Number of complete paired draws incorporated.
    pub sample_count: u64,
    /// Empirical mean of left consequence minus right consequence.
    pub empirical_mean_left_minus_right_ticks: f64,
    /// Simultaneous lower confidence endpoint for E[L(left) - L(right)].
    pub lower_left_minus_right_ticks: f64,
    /// Simultaneous upper confidence endpoint for E[L(left) - L(right)].
    pub upper_left_minus_right_ticks: f64,
    /// Bonferroni-allocated two-sided error budget for this contrast.
    pub contrast_alpha: f64,
}

/// Decision-oriented summary derived only from simultaneous pair intervals.
#[derive(Debug, Clone, PartialEq)]
pub struct SimultaneousPortfolioSelectionReport {
    /// Family-wise error budget declared before sampling.
    pub family_alpha: f64,
    /// Number of frozen portfolio arms, including the baseline if configured.
    pub portfolio_count: usize,
    /// Number of pairwise contrasts in the simultaneous family.
    pub contrast_count: usize,
    /// Per-contrast Bonferroni error allocation.
    pub contrast_alpha: f64,
    /// Number of complete paired draws incorporated.
    pub draw_count: u64,
    /// Pair intervals in canonical left/right identity order.
    pub pairwise_intervals: Vec<PortfolioContrastConfidenceInterval>,
    /// Present only when one arm is demonstrated to have lower expected
    /// consequence than every other arm at the simultaneous confidence level.
    pub unique_best_portfolio_id: Option<[u8; 32]>,
    /// Arms not yet proven worse than another arm.
    pub statistically_undominated_portfolio_ids: Vec<[u8; 32]>,
}

/// Validation and statistical-update errors for simultaneous portfolio inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioSelectionError {
    /// Confidence error probability must lie strictly between zero and one.
    InvalidFamilyAlpha,
    /// Fewer than two portfolios were supplied.
    TooFewPortfolios,
    /// Candidate count exceeds the bounded multi-portfolio limit.
    TooManyPortfolios,
    /// A portfolio identity is zero, duplicated or inconsistent.
    InvalidPortfolioIdentity,
    /// A consequence bound has lower endpoint greater than its upper endpoint.
    InvalidConsequenceBound,
    /// A paired draw does not contain the complete frozen portfolio universe.
    IncompletePortfolioDraw,
    /// A paired draw contains a consequence outside its frozen portfolio bound.
    ConsequenceOutsideBound,
    /// A draw index is duplicated, skipped, reordered or outside the configured limit.
    InvalidDrawIndex,
    /// Pairwise contrast count arithmetic overflowed.
    ContrastCountOverflow,
    /// Floating-point normalization or interval transformation failed.
    NumericalFailure,
    /// An underlying PrPl-EB update failed.
    PrPlEbFailure,
}

impl core::fmt::Display for PortfolioSelectionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidFamilyAlpha => "family-wise confidence error must lie in (0, 1)",
            Self::TooFewPortfolios => "simultaneous selection requires at least two portfolios",
            Self::TooManyPortfolios => "portfolio universe exceeds the exact comparison limit",
            Self::InvalidPortfolioIdentity => {
                "portfolio identity is zero, duplicated or inconsistent"
            }
            Self::InvalidConsequenceBound => "portfolio consequence bound is invalid",
            Self::IncompletePortfolioDraw => {
                "paired draw does not cover the frozen portfolio universe"
            }
            Self::ConsequenceOutsideBound => "portfolio consequence lies outside its frozen bound",
            Self::InvalidDrawIndex => "paired draw index is not the next expected sequence number",
            Self::ContrastCountOverflow => "pairwise contrast count overflowed",
            Self::NumericalFailure => {
                "portfolio contrast normalization or interval failed numerically"
            }
            Self::PrPlEbFailure => "underlying PrPl-EB confidence-sequence update failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PortfolioSelectionError {}

impl From<PrPlEbError> for PortfolioSelectionError {
    fn from(_: PrPlEbError) -> Self {
        Self::PrPlEbFailure
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ContrastProcess {
    left_portfolio_id: [u8; 32],
    right_portfolio_id: [u8; 32],
    minimum_difference_ticks: i64,
    maximum_difference_ticks: i64,
    sequence: Option<PrPlEbConfidenceSequence>,
}

/// Maintains a simultaneous anytime-valid comparison family for frozen portfolio arms.
///
/// The input bounds define the complete support of each arm's consequence.
/// For arms a and b, the contrast range is derived before sampling as:
///
/// L(a) - L(b) in [lower_a - upper_b, upper_a - lower_b].
///
/// Each non-degenerate contrast is affinely normalized into [0, 1] and passed
/// to an independent PrPl-EB process with alpha divided by the total number of
/// pairwise contrasts. A union bound then gives family-wise time-uniform
/// coverage, without requiring the contrasts within a paired draw to be
/// independent.
///
/// The guarantee requires a fixed conditional mean for every normalized
/// contrast. The current API is intended for draws from one fixed target
/// distribution; adaptive importance sampling needs a separately verified
/// bounded-weight adapter and proposal receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct SimultaneousPortfolioComparison {
    family_alpha: f64,
    contrast_alpha: f64,
    portfolio_bounds: Vec<PortfolioConsequenceBound>,
    contrasts: Vec<ContrastProcess>,
    draw_count: u64,
}

impl SimultaneousPortfolioComparison {
    /// Construct the complete pairwise comparison family.
    ///
    /// The portfolio bound list must contain at least two unique, nonzero IDs.
    /// Input order is irrelevant; the internal order is canonical by identity.
    pub fn new(
        family_alpha: f64,
        mut portfolio_bounds: Vec<PortfolioConsequenceBound>,
    ) -> Result<Self, PortfolioSelectionError> {
        if !family_alpha.is_finite() || family_alpha <= 0.0 || family_alpha >= 1.0 {
            return Err(PortfolioSelectionError::InvalidFamilyAlpha);
        }
        if portfolio_bounds.len() < 2 {
            return Err(PortfolioSelectionError::TooFewPortfolios);
        }
        if portfolio_bounds.len() > MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS {
            return Err(PortfolioSelectionError::TooManyPortfolios);
        }

        portfolio_bounds.sort_unstable_by_key(|bound| bound.portfolio_id);
        for (index, bound) in portfolio_bounds.iter().enumerate() {
            if bound.portfolio_id.iter().all(|byte| *byte == 0)
                || (index > 0 && portfolio_bounds[index - 1].portfolio_id == bound.portfolio_id)
            {
                return Err(PortfolioSelectionError::InvalidPortfolioIdentity);
            }
            if bound.lower_ticks > bound.upper_ticks {
                return Err(PortfolioSelectionError::InvalidConsequenceBound);
            }
        }

        let portfolio_count = portfolio_bounds.len();
        let contrast_count = portfolio_count
            .checked_mul(portfolio_count - 1)
            .and_then(|count| count.checked_div(2))
            .ok_or(PortfolioSelectionError::ContrastCountOverflow)?;
        // Allocate strictly below the real-valued quotient so the pairwise
        // family cannot exceed the declared error budget through rounding.
        let contrast_alpha = next_down_probability(family_alpha / contrast_count as f64);
        if !contrast_alpha.is_finite() || contrast_alpha <= 0.0 || contrast_alpha >= 1.0 {
            return Err(PortfolioSelectionError::NumericalFailure);
        }

        let mut contrasts = Vec::with_capacity(contrast_count);
        for left_index in 0..portfolio_count {
            for right_index in (left_index + 1)..portfolio_count {
                let left = portfolio_bounds[left_index];
                let right = portfolio_bounds[right_index];
                let minimum_difference_ticks =
                    i64::from(left.lower_ticks) - i64::from(right.upper_ticks);
                let maximum_difference_ticks =
                    i64::from(left.upper_ticks) - i64::from(right.lower_ticks);
                let sequence = if minimum_difference_ticks == maximum_difference_ticks {
                    None
                } else {
                    Some(PrPlEbConfidenceSequence::new(contrast_alpha)?)
                };
                contrasts.push(ContrastProcess {
                    left_portfolio_id: left.portfolio_id,
                    right_portfolio_id: right.portfolio_id,
                    minimum_difference_ticks,
                    maximum_difference_ticks,
                    sequence,
                });
            }
        }

        Ok(Self {
            family_alpha,
            contrast_alpha,
            portfolio_bounds,
            contrasts,
            draw_count: 0,
        })
    }

    /// Add one complete paired draw, updating every contrast transactionally.
    ///
    /// The draw index must equal the next expected index. Every portfolio
    /// consequence is checked against its predeclared bound before any
    /// confidence sequence is updated. All PrPl-EB states are updated on
    /// clones first; if any update fails, no state or draw counter is committed.
    pub fn observe(
        &mut self,
        draw: &SimultaneousPortfolioDraw,
    ) -> Result<(), PortfolioSelectionError> {
        let expected_index = self
            .draw_count
            .checked_add(1)
            .ok_or(PortfolioSelectionError::InvalidDrawIndex)?;
        if draw.draw_index != expected_index || draw.draw_index > MAX_SIMULTANEOUS_PORTFOLIO_DRAWS {
            return Err(PortfolioSelectionError::InvalidDrawIndex);
        }
        if draw.portfolio_consequences.len() != self.portfolio_bounds.len() {
            return Err(PortfolioSelectionError::IncompletePortfolioDraw);
        }

        let mut observed = BTreeMap::new();
        for item in &draw.portfolio_consequences {
            if item.portfolio_id.iter().all(|byte| *byte == 0)
                || observed
                    .insert(item.portfolio_id, item.consequence_ticks)
                    .is_some()
            {
                return Err(PortfolioSelectionError::InvalidPortfolioIdentity);
            }
        }
        if observed.len() != self.portfolio_bounds.len()
            || self
                .portfolio_bounds
                .iter()
                .any(|bound| !observed.contains_key(&bound.portfolio_id))
        {
            return Err(PortfolioSelectionError::IncompletePortfolioDraw);
        }
        for bound in &self.portfolio_bounds {
            let consequence = observed
                .get(&bound.portfolio_id)
                .ok_or(PortfolioSelectionError::IncompletePortfolioDraw)?;
            if *consequence < bound.lower_ticks || *consequence > bound.upper_ticks {
                return Err(PortfolioSelectionError::ConsequenceOutsideBound);
            }
        }

        let mut next_sequences = self
            .contrasts
            .iter()
            .map(|contrast| contrast.sequence.clone())
            .collect::<Vec<_>>();
        for (index, contrast) in self.contrasts.iter().enumerate() {
            let Some(sequence) = next_sequences[index].as_mut() else {
                continue;
            };
            let left = i64::from(
                *observed
                    .get(&contrast.left_portfolio_id)
                    .ok_or(PortfolioSelectionError::IncompletePortfolioDraw)?,
            );
            let right = i64::from(
                *observed
                    .get(&contrast.right_portfolio_id)
                    .ok_or(PortfolioSelectionError::IncompletePortfolioDraw)?,
            );
            let difference = left - right;
            let width = contrast.maximum_difference_ticks - contrast.minimum_difference_ticks;
            let normalized = (difference - contrast.minimum_difference_ticks) as f64 / width as f64;
            if !normalized.is_finite() || !(0.0..=1.0).contains(&normalized) {
                return Err(PortfolioSelectionError::NumericalFailure);
            }
            sequence.observe(normalized)?;
        }

        for (contrast, sequence) in self.contrasts.iter_mut().zip(next_sequences) {
            contrast.sequence = sequence;
        }
        self.draw_count = expected_index;
        Ok(())
    }

    /// Number of complete paired draws incorporated.
    pub fn draw_count(&self) -> u64 {
        self.draw_count
    }

    /// Return the simultaneous family-wise confidence report.
    ///
    /// A unique best portfolio is reported only after its upper contrast
    /// confidence bound is below zero against every competitor. The
    /// statistically undominated set is retained whenever no unique winner
    /// has been established. This is a consequence objective only; mission
    /// risk, source completeness and operational authority remain separate.
    pub fn report(&self) -> Result<SimultaneousPortfolioSelectionReport, PortfolioSelectionError> {
        let mut pairwise_intervals = Vec::with_capacity(self.contrasts.len());
        for contrast in &self.contrasts {
            let interval = if let Some(sequence) = &contrast.sequence {
                let current = sequence.confidence_sequence();
                let width_ticks =
                    (contrast.maximum_difference_ticks - contrast.minimum_difference_ticks) as f64;
                let minimum = contrast.minimum_difference_ticks as f64;
                let lower = outward_down(minimum + width_ticks * current.lower)
                    .max(contrast.minimum_difference_ticks as f64);
                let upper = outward_up(minimum + width_ticks * current.upper)
                    .min(contrast.maximum_difference_ticks as f64);
                let mean = minimum + width_ticks * current.empirical_mean;
                if !lower.is_finite() || !upper.is_finite() || !mean.is_finite() || lower > upper {
                    return Err(PortfolioSelectionError::NumericalFailure);
                }
                PortfolioContrastConfidenceInterval {
                    left_portfolio_id: contrast.left_portfolio_id,
                    right_portfolio_id: contrast.right_portfolio_id,
                    sample_count: self.draw_count,
                    empirical_mean_left_minus_right_ticks: mean,
                    lower_left_minus_right_ticks: lower,
                    upper_left_minus_right_ticks: upper,
                    contrast_alpha: self.contrast_alpha,
                }
            } else {
                let exact = contrast.minimum_difference_ticks as f64;
                PortfolioContrastConfidenceInterval {
                    left_portfolio_id: contrast.left_portfolio_id,
                    right_portfolio_id: contrast.right_portfolio_id,
                    sample_count: self.draw_count,
                    empirical_mean_left_minus_right_ticks: exact,
                    lower_left_minus_right_ticks: exact,
                    upper_left_minus_right_ticks: exact,
                    contrast_alpha: self.contrast_alpha,
                }
            };
            pairwise_intervals.push(interval);
        }

        let mut uniquely_better = Vec::new();
        let mut undominated = Vec::new();
        for candidate in &self.portfolio_bounds {
            let mut beats_every_other = self.draw_count > 0;
            let mut is_dominated = false;
            for opponent in &self.portfolio_bounds {
                if candidate.portfolio_id == opponent.portfolio_id {
                    continue;
                }
                let (left_id, right_id) = if candidate.portfolio_id < opponent.portfolio_id {
                    (candidate.portfolio_id, opponent.portfolio_id)
                } else {
                    (opponent.portfolio_id, candidate.portfolio_id)
                };
                let interval = pairwise_intervals
                    .iter()
                    .find(|interval| {
                        interval.left_portfolio_id == left_id
                            && interval.right_portfolio_id == right_id
                    })
                    .ok_or(PortfolioSelectionError::NumericalFailure)?;
                let candidate_is_left = candidate.portfolio_id == left_id;
                let candidate_is_proven_better = if candidate_is_left {
                    interval.upper_left_minus_right_ticks < 0.0
                } else {
                    interval.lower_left_minus_right_ticks > 0.0
                };
                let opponent_is_proven_better = if candidate_is_left {
                    interval.lower_left_minus_right_ticks > 0.0
                } else {
                    interval.upper_left_minus_right_ticks < 0.0
                };
                beats_every_other &= candidate_is_proven_better;
                is_dominated |= opponent_is_proven_better;
            }
            if beats_every_other {
                uniquely_better.push(candidate.portfolio_id);
            }
            if !is_dominated {
                undominated.push(candidate.portfolio_id);
            }
        }

        Ok(SimultaneousPortfolioSelectionReport {
            family_alpha: self.family_alpha,
            portfolio_count: self.portfolio_bounds.len(),
            contrast_count: self.contrasts.len(),
            contrast_alpha: self.contrast_alpha,
            draw_count: self.draw_count,
            pairwise_intervals,
            unique_best_portfolio_id: if uniquely_better.len() == 1 {
                Some(uniquely_better[0])
            } else {
                None
            },
            statistically_undominated_portfolio_ids: undominated,
        })
    }
}

fn outward_down(value: f64) -> f64 {
    if !value.is_finite() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

fn outward_up(value: f64) -> f64 {
    if !value.is_finite() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

/// Return the immediately smaller positive representable probability.
fn next_down_probability(value: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 {
        return value;
    }
    f64::from_bits(value.to_bits() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bound(id: u8, lower: i32, upper: i32) -> PortfolioConsequenceBound {
        PortfolioConsequenceBound {
            portfolio_id: [id; 32],
            lower_ticks: lower,
            upper_ticks: upper,
        }
    }

    fn draw(index: u64, consequences: &[(u8, i32)]) -> SimultaneousPortfolioDraw {
        SimultaneousPortfolioDraw {
            draw_index: index,
            portfolio_consequences: consequences
                .iter()
                .map(|(id, consequence_ticks)| PortfolioConsequenceObservation {
                    portfolio_id: [*id; 32],
                    consequence_ticks: *consequence_ticks,
                })
                .collect(),
        }
    }

    #[test]
    fn family_alpha_is_allocated_over_every_pair() {
        let process = SimultaneousPortfolioComparison::new(
            0.06,
            vec![bound(30, 0, 10), bound(10, 0, 10), bound(20, 0, 10)],
        )
        .unwrap();
        let report = process.report().unwrap();

        assert_eq!(report.portfolio_count, 3);
        assert_eq!(report.contrast_count, 3);
        assert_eq!(report.family_alpha, 0.06);
        assert_eq!(report.contrast_alpha, 0.02);
        assert_eq!(report.pairwise_intervals.len(), 3);
        assert_eq!(report.draw_count, 0);
        assert_eq!(report.unique_best_portfolio_id, None);
        assert_eq!(
            report.statistically_undominated_portfolio_ids,
            vec![[10; 32], [20; 32], [30; 32]]
        );
    }

    #[test]
    fn unique_best_requires_simultaneous_evidence_against_every_arm() {
        let mut process = SimultaneousPortfolioComparison::new(
            0.05,
            vec![bound(10, 0, 100), bound(20, 0, 100), bound(30, 0, 100)],
        )
        .unwrap();

        for index in 1..=2_000 {
            process
                .observe(&draw(index, &[(10, 10), (20, 20), (30, 30)]))
                .unwrap();
            let report = process.report().unwrap();
            if report.unique_best_portfolio_id.is_some() {
                assert_eq!(report.unique_best_portfolio_id, Some([10; 32]));
                assert_eq!(
                    report.statistically_undominated_portfolio_ids,
                    vec![[10; 32]]
                );
                return;
            }
        }
        panic!("a well-separated deterministic best arm was not certified within 2,000 draws");
    }

    #[test]
    fn equal_mean_arms_remain_undominated_and_no_unique_winner_is_claimed() {
        let mut process = SimultaneousPortfolioComparison::new(
            0.05,
            vec![bound(10, 0, 100), bound(20, 0, 100), bound(30, 0, 100)],
        )
        .unwrap();
        for index in 1..=500 {
            process
                .observe(&draw(index, &[(10, 10), (20, 10), (30, 20)]))
                .unwrap();
        }
        let report = process.report().unwrap();
        assert_eq!(report.unique_best_portfolio_id, None);
        assert_eq!(
            report.statistically_undominated_portfolio_ids,
            vec![[10; 32], [20; 32]]
        );
    }

    #[test]
    fn degenerate_bounds_produce_exact_pair_intervals() {
        let process = SimultaneousPortfolioComparison::new(
            0.05,
            vec![bound(10, 5, 5), bound(20, 9, 9), bound(30, 2, 2)],
        )
        .unwrap();
        let report = process.report().unwrap();

        assert_eq!(
            report.pairwise_intervals[0].lower_left_minus_right_ticks,
            -4.0
        );
        assert_eq!(
            report.pairwise_intervals[0].upper_left_minus_right_ticks,
            -4.0
        );
        assert_eq!(
            report.pairwise_intervals[1].lower_left_minus_right_ticks,
            3.0
        );
        assert_eq!(
            report.pairwise_intervals[1].upper_left_minus_right_ticks,
            3.0
        );
    }

    #[test]
    fn invalid_or_partial_draw_is_transactional() {
        let mut process =
            SimultaneousPortfolioComparison::new(0.05, vec![bound(10, 0, 10), bound(20, 0, 10)])
                .unwrap();

        assert_eq!(
            process.observe(&draw(1, &[(10, 5)])),
            Err(PortfolioSelectionError::IncompletePortfolioDraw)
        );
        assert_eq!(process.draw_count(), 0);

        assert_eq!(
            process.observe(&draw(1, &[(10, 11), (20, 5)])),
            Err(PortfolioSelectionError::ConsequenceOutsideBound)
        );
        assert_eq!(process.draw_count(), 0);

        process.observe(&draw(1, &[(20, 5), (10, 5)])).unwrap();
        assert_eq!(process.draw_count(), 1);
        assert_eq!(
            process.observe(&draw(1, &[(10, 5), (20, 5)])),
            Err(PortfolioSelectionError::InvalidDrawIndex)
        );
        assert_eq!(process.draw_count(), 1);
    }

    #[test]
    fn duplicate_and_substituted_portfolio_identities_are_rejected() {
        let mut process =
            SimultaneousPortfolioComparison::new(0.05, vec![bound(10, 0, 10), bound(20, 0, 10)])
                .unwrap();

        assert_eq!(
            process.observe(&draw(1, &[(10, 2), (10, 3)])),
            Err(PortfolioSelectionError::InvalidPortfolioIdentity)
        );
        assert_eq!(
            process.observe(&draw(1, &[(10, 2), (30, 3)])),
            Err(PortfolioSelectionError::IncompletePortfolioDraw)
        );
        assert_eq!(process.draw_count(), 0);
    }

    #[test]
    fn unordered_bounds_and_draw_rows_produce_canonical_report_order() {
        let mut process = SimultaneousPortfolioComparison::new(
            0.05,
            vec![bound(30, 0, 10), bound(10, 0, 10), bound(20, 0, 10)],
        )
        .unwrap();
        process
            .observe(&draw(1, &[(30, 2), (10, 1), (20, 4)]))
            .unwrap();
        let report = process.report().unwrap();

        assert_eq!(
            report
                .pairwise_intervals
                .iter()
                .map(|interval| (
                    interval.left_portfolio_id[0],
                    interval.right_portfolio_id[0]
                ))
                .collect::<Vec<_>>(),
            vec![(10, 20), (10, 30), (20, 30)]
        );
    }
}
