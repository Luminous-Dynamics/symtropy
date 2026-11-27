// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Atomic, simultaneous portfolio selection under one frozen defensive proposal.
//!
//! One common target-state draw is evaluated once under the full defensive
//! mixture. Its direct portfolio consequences produce every pairwise contrast.
//! Each contrast updates its own PrPl-EB sequence with a Bonferroni-allocated
//! error budget. Updates are staged and committed atomically.
//!
//! The coordinator stores draw IDs once globally, avoiding one large duplicate
//! replay ledger per pair. The fixed-mixture and one-scenario assumptions are
//! inherited from PortfolioDefensiveGaussianMixture.

use std::collections::{BTreeMap, BTreeSet};

use crate::paired_counterfactual::MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS;
use crate::portfolio_proposal::{
    PortfolioDefensiveGaussianMixture, PortfolioEventModeKey, PortfolioProposalError,
};
use crate::portfolio_selection::PortfolioConsequenceBound;
use crate::predictable_empirical_bernstein::{PrPlEbConfidenceSequence, PrPlEbError};

/// Maximum complete mission-state draws in one simultaneous defensive campaign.
pub const MAX_SIMULTANEOUS_DEFENSIVE_DRAWS: u64 = 1_000_000;

/// One complete direct portfolio consequence row on a shared state draw.
#[derive(Debug, Clone, PartialEq)]
pub struct DefensivePortfolioDraw {
    /// One-based, consecutive draw index beginning at one.
    pub draw_index: u64,
    /// Stable nonzero identity for this target-state realization.
    ///
    /// Uniqueness is checked once by this coordinator. This is not an
    /// authenticated sampler receipt and does not prevent relabelling a replay.
    pub draw_id: [u8; 32],
    /// Declared portfolio/scenario/event-mode identity for this draw.
    pub mode_key: PortfolioEventModeKey,
    /// State in the frozen proposal density space.
    pub point: Vec<f64>,
    /// Directly evaluated consequence for every frozen portfolio exactly once.
    pub portfolio_consequences: Vec<crate::portfolio_selection::PortfolioConsequenceObservation>,
}

/// One pairwise normalized contrast and bounded defensive observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefensivePortfolioPairObservation {
    /// Canonically smaller portfolio identity.
    pub left_portfolio_id: [u8; 32],
    /// Canonically larger portfolio identity.
    pub right_portfolio_id: [u8; 32],
    /// Directly evaluated left-minus-right consequence in integer ticks.
    pub consequence_difference_ticks: i64,
    /// Frozen-bound normalized contrast in [0,1].
    pub normalized_consequence_difference: f64,
    /// Bounded defensive observation in [0,1].
    pub defensive_contribution: f64,
}

/// Receipt for one complete shared defensive portfolio draw.
#[derive(Debug, Clone, PartialEq)]
pub struct DefensivePortfolioDrawReceipt {
    /// One-based draw index.
    pub draw_index: u64,
    /// Unique local draw identity.
    pub draw_id: [u8; 32],
    /// Frozen mode identity for this draw.
    pub mode_key: PortfolioEventModeKey,
    /// Stable identity for the consequence unit, sign convention and tick scale.
    pub consequence_unit_id: [u8; 32],
    /// Target-measure identity.
    pub target_measure_id: [u8; 32],
    /// Density-space identity.
    pub density_space_id: [u8; 32],
    /// Frozen proposal configuration digest.
    pub configuration_digest: [u8; 32],
    /// Log target density evaluated once for the common draw.
    pub log_target_density: f64,
    /// Log full defensive proposal density evaluated once for the common draw.
    pub log_full_proposal_density: f64,
    /// Log target/proposal likelihood ratio evaluated once for the common draw.
    pub log_importance_weight: f64,
    /// Complete direct portfolio outcomes in canonical portfolio-ID order.
    pub portfolio_consequences: Vec<crate::portfolio_selection::PortfolioConsequenceObservation>,
    /// All direct pairwise contrasts and defensive contributions.
    pub pair_observations: Vec<DefensivePortfolioPairObservation>,
}

/// Simultaneous confidence interval for one defensive portfolio pair.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefensivePortfolioPairInterval {
    /// Canonically smaller portfolio identity.
    pub left_portfolio_id: [u8; 32],
    /// Canonically larger portfolio identity.
    pub right_portfolio_id: [u8; 32],
    /// Number of complete shared mission-state draws.
    pub sample_count: u64,
    /// Bonferroni-allocated two-sided error budget for this contrast.
    pub contrast_alpha: f64,
    /// Lower target expected left-minus-right consequence endpoint.
    pub lower_expected_difference_ticks: f64,
    /// Upper target expected left-minus-right consequence endpoint.
    pub upper_expected_difference_ticks: f64,
    /// Lower normalized target expected contrast endpoint.
    pub lower_normalized_expected_difference: f64,
    /// Upper normalized target expected contrast endpoint.
    pub upper_normalized_expected_difference: f64,
}

/// Family-wise simultaneous selection report for the frozen portfolio universe.
#[derive(Debug, Clone, PartialEq)]
pub struct SimultaneousDefensivePortfolioReport {
    /// Stable identity for the consequence unit, sign convention and tick scale.
    pub consequence_unit_id: [u8; 32],
    /// Family-wise error budget frozen before sampling.
    pub family_alpha: f64,
    /// Number of frozen candidate portfolio arms.
    pub portfolio_count: usize,
    /// Number of simultaneous pairwise contrasts.
    pub contrast_count: usize,
    /// Bonferroni-allocated error budget per pair.
    pub contrast_alpha: f64,
    /// Number of complete shared draws accepted.
    pub draw_count: u64,
    /// All pairwise intervals in canonical portfolio identity order.
    pub pairwise_intervals: Vec<DefensivePortfolioPairInterval>,
    /// Present only when one arm is demonstrated to have lower expected
    /// consequence than every other arm.
    pub unique_best_portfolio_id: Option<[u8; 32]>,
    /// Arms not yet proven worse than at least one competitor.
    pub statistically_undominated_portfolio_ids: Vec<[u8; 32]>,
    /// Target-measure identity.
    pub target_measure_id: [u8; 32],
    /// Density-space identity.
    pub density_space_id: [u8; 32],
    /// Single epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Frozen defensive configuration digest.
    pub configuration_digest: [u8; 32],
    /// Exact defensive target fraction.
    pub effective_defensive_fraction: f64,
}

/// Construction, validation or statistical update errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimultaneousDefensiveSelectionError {
    /// Family error probability must lie strictly in (0,1).
    InvalidFamilyAlpha,
    /// Consequence unit identity is zero or invalid.
    InvalidConsequenceUnitIdentity,
    /// At least two portfolios are required.
    TooFewPortfolios,
    /// Portfolio count exceeds the bounded pairwise comparison limit.
    TooManyPortfolios,
    /// Portfolio identity is zero or duplicated.
    InvalidPortfolioIdentity,
    /// A portfolio consequence bound is invalid.
    InvalidConsequenceBound,
    /// The proposal mode universe does not cover a frozen portfolio.
    IncompleteProposalPortfolioCoverage,
    /// Pair count arithmetic overflowed.
    ContrastCountOverflow,
    /// Draw index is skipped, repeated, reordered or out of bound.
    InvalidDrawIndex,
    /// Draw identity is zero or already seen by this family.
    InvalidOrDuplicateDrawIdentity,
    /// A draw does not contain the complete frozen portfolio universe.
    IncompletePortfolioDraw,
    /// A portfolio is duplicated, zero or substituted in one draw.
    InvalidDrawPortfolioIdentity,
    /// A direct portfolio consequence is outside its frozen arm bound.
    ConsequenceOutsideFrozenBounds,
    /// The shared proposal density evaluation failed.
    ProposalEvaluationFailure,
    /// One pairwise PrPl-EB update failed.
    ConfidenceSequenceFailure,
    /// Pairwise interval transform or selection report failed numerically.
    NumericalFailure,
}

impl core::fmt::Display for SimultaneousDefensiveSelectionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidFamilyAlpha => "family-wise alpha must lie strictly between zero and one",
            Self::InvalidConsequenceUnitIdentity => "consequence unit identity is zero or invalid",
            Self::TooFewPortfolios => "at least two portfolio arms are required",
            Self::TooManyPortfolios => "portfolio universe exceeds the bounded comparison limit",
            Self::InvalidPortfolioIdentity => "portfolio identity is zero or duplicated",
            Self::InvalidConsequenceBound => "portfolio consequence bound is invalid",
            Self::IncompleteProposalPortfolioCoverage => {
                "proposal mode universe omits a frozen portfolio"
            }
            Self::ContrastCountOverflow => "pairwise contrast count overflowed",
            Self::InvalidDrawIndex => "draw index is not the next consecutive sequence index",
            Self::InvalidOrDuplicateDrawIdentity => "draw identity is zero or duplicated",
            Self::IncompletePortfolioDraw => {
                "draw does not contain the complete portfolio universe"
            }
            Self::InvalidDrawPortfolioIdentity => {
                "draw portfolio identity is zero, duplicated or substituted"
            }
            Self::ConsequenceOutsideFrozenBounds => {
                "portfolio consequence lies outside its frozen bound"
            }
            Self::ProposalEvaluationFailure => "shared defensive proposal evaluation failed",
            Self::ConfidenceSequenceFailure => "one pairwise confidence update failed",
            Self::NumericalFailure => "simultaneous defensive portfolio report failed numerically",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SimultaneousDefensiveSelectionError {}

impl From<PortfolioProposalError> for SimultaneousDefensiveSelectionError {
    fn from(_: PortfolioProposalError) -> Self {
        Self::ProposalEvaluationFailure
    }
}

impl From<PrPlEbError> for SimultaneousDefensiveSelectionError {
    fn from(_: PrPlEbError) -> Self {
        Self::ConfidenceSequenceFailure
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DefensivePairProcess {
    left_portfolio_id: [u8; 32],
    right_portfolio_id: [u8; 32],
    minimum_difference_ticks: i64,
    maximum_difference_ticks: i64,
    sequence: Option<PrPlEbConfidenceSequence>,
}

/// Coordinates every pairwise portfolio contrast on one shared defensive draw.
#[derive(Debug, Clone, PartialEq)]
pub struct SimultaneousDefensivePortfolioSelection {
    family_alpha: f64,
    contrast_alpha: f64,
    consequence_unit_id: [u8; 32],
    portfolio_bounds: Vec<PortfolioConsequenceBound>,
    proposal: PortfolioDefensiveGaussianMixture,
    pair_processes: Vec<DefensivePairProcess>,
    draw_count: u64,
    seen_draw_ids: BTreeSet<[u8; 32]>,
}

impl SimultaneousDefensivePortfolioSelection {
    /// Construct a simultaneous family from frozen arm consequence bounds.
    ///
    /// Each portfolio must appear in the proposal's frozen mode universe. The
    /// proposal itself is fixed and represents exactly one epistemic scenario.
    /// Family alpha is split uniformly across K(K-1)/2 pairwise PrPl-EB sequences.
    pub fn new(
        family_alpha: f64,
        consequence_unit_id: [u8; 32],
        mut portfolio_bounds: Vec<PortfolioConsequenceBound>,
        proposal: PortfolioDefensiveGaussianMixture,
    ) -> Result<Self, SimultaneousDefensiveSelectionError> {
        if !family_alpha.is_finite() || family_alpha <= 0.0 || family_alpha >= 1.0 {
            return Err(SimultaneousDefensiveSelectionError::InvalidFamilyAlpha);
        }
        if consequence_unit_id.iter().all(|byte| *byte == 0) {
            return Err(SimultaneousDefensiveSelectionError::InvalidConsequenceUnitIdentity);
        }
        if portfolio_bounds.len() < 2 {
            return Err(SimultaneousDefensiveSelectionError::TooFewPortfolios);
        }
        if portfolio_bounds.len() > MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS {
            return Err(SimultaneousDefensiveSelectionError::TooManyPortfolios);
        }

        portfolio_bounds.sort_unstable_by_key(|bound| bound.portfolio_id);
        for (index, bound) in portfolio_bounds.iter().enumerate() {
            if bound.portfolio_id.iter().all(|byte| *byte == 0)
                || (index > 0 && portfolio_bounds[index - 1].portfolio_id == bound.portfolio_id)
            {
                return Err(SimultaneousDefensiveSelectionError::InvalidPortfolioIdentity);
            }
            if bound.lower_ticks > bound.upper_ticks {
                return Err(SimultaneousDefensiveSelectionError::InvalidConsequenceBound);
            }
            if !proposal
                .expected_modes()
                .iter()
                .any(|mode| mode.portfolio_id == bound.portfolio_id)
            {
                return Err(
                    SimultaneousDefensiveSelectionError::IncompleteProposalPortfolioCoverage,
                );
            }
        }

        let portfolio_count = portfolio_bounds.len();
        let contrast_count = portfolio_count
            .checked_mul(portfolio_count - 1)
            .and_then(|count| count.checked_div(2))
            .ok_or(SimultaneousDefensiveSelectionError::ContrastCountOverflow)?;
        let contrast_alpha = family_alpha / contrast_count as f64;
        if !contrast_alpha.is_finite() || contrast_alpha <= 0.0 || contrast_alpha >= 1.0 {
            return Err(SimultaneousDefensiveSelectionError::NumericalFailure);
        }

        let mut pair_processes = Vec::with_capacity(contrast_count);
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
                pair_processes.push(DefensivePairProcess {
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
            consequence_unit_id,
            portfolio_bounds,
            proposal,
            pair_processes,
            draw_count: 0,
            seen_draw_ids: BTreeSet::new(),
        })
    }

    /// Add one complete shared state draw and update every pair atomically.
    ///
    /// The full target/proposal densities are evaluated once for the shared
    /// point. Every pair receives its own normalized consequence contrast and
    /// bounded defensive contribution. All pair sequences are cloned and
    /// staged before any state or draw identity is committed.
    pub fn observe(
        &mut self,
        draw: &DefensivePortfolioDraw,
    ) -> Result<DefensivePortfolioDrawReceipt, SimultaneousDefensiveSelectionError> {
        let expected_index = self
            .draw_count
            .checked_add(1)
            .ok_or(SimultaneousDefensiveSelectionError::InvalidDrawIndex)?;
        if draw.draw_index != expected_index || draw.draw_index > MAX_SIMULTANEOUS_DEFENSIVE_DRAWS {
            return Err(SimultaneousDefensiveSelectionError::InvalidDrawIndex);
        }
        if draw.draw_id.iter().all(|byte| *byte == 0) || self.seen_draw_ids.contains(&draw.draw_id)
        {
            return Err(SimultaneousDefensiveSelectionError::InvalidOrDuplicateDrawIdentity);
        }
        if draw.portfolio_consequences.len() != self.portfolio_bounds.len() {
            return Err(SimultaneousDefensiveSelectionError::IncompletePortfolioDraw);
        }

        let mut observed = BTreeMap::new();
        for item in &draw.portfolio_consequences {
            if item.portfolio_id.iter().all(|byte| *byte == 0)
                || observed
                    .insert(item.portfolio_id, item.consequence_ticks)
                    .is_some()
            {
                return Err(SimultaneousDefensiveSelectionError::InvalidDrawPortfolioIdentity);
            }
        }
        if observed.len() != self.portfolio_bounds.len()
            || self
                .portfolio_bounds
                .iter()
                .any(|bound| !observed.contains_key(&bound.portfolio_id))
        {
            return Err(SimultaneousDefensiveSelectionError::IncompletePortfolioDraw);
        }
        for bound in &self.portfolio_bounds {
            let value = observed
                .get(&bound.portfolio_id)
                .ok_or(SimultaneousDefensiveSelectionError::IncompletePortfolioDraw)?;
            if *value < bound.lower_ticks || *value > bound.upper_ticks {
                return Err(SimultaneousDefensiveSelectionError::ConsequenceOutsideFrozenBounds);
            }
        }

        // Evaluate q and p once. A unit normalized contrast validates the common
        // likelihood ratio bound; pair-specific contributions are derived from
        // the same log ratio without re-evaluating the Gaussian mixture.
        let common_evaluation = self
            .proposal
            .evaluate_draw(draw.mode_key, &draw.point, 1.0)?;
        let epsilon = self.proposal.defensive_fraction();
        let log_epsilon = epsilon.ln();

        let mut next_sequences = self
            .pair_processes
            .iter()
            .map(|pair| pair.sequence.clone())
            .collect::<Vec<_>>();
        let mut pair_observations = Vec::with_capacity(self.pair_processes.len());

        let mut portfolio_consequence_receipts = Vec::with_capacity(self.portfolio_bounds.len());
        for bound in &self.portfolio_bounds {
            let value = observed
                .get(&bound.portfolio_id)
                .copied()
                .ok_or(SimultaneousDefensiveSelectionError::IncompletePortfolioDraw)?;
            portfolio_consequence_receipts.push(
                crate::portfolio_selection::PortfolioConsequenceObservation {
                    portfolio_id: bound.portfolio_id,
                    consequence_ticks: value,
                },
            );
        }

        for (index, pair) in self.pair_processes.iter().enumerate() {
            let left = i64::from(
                *observed
                    .get(&pair.left_portfolio_id)
                    .ok_or(SimultaneousDefensiveSelectionError::IncompletePortfolioDraw)?,
            );
            let right = i64::from(
                *observed
                    .get(&pair.right_portfolio_id)
                    .ok_or(SimultaneousDefensiveSelectionError::IncompletePortfolioDraw)?,
            );
            let difference = left - right;
            let width = pair.maximum_difference_ticks - pair.minimum_difference_ticks;
            let normalized = if width == 0 {
                0.0
            } else {
                (difference - pair.minimum_difference_ticks) as f64 / width as f64
            };
            if !normalized.is_finite() || !(0.0..=1.0).contains(&normalized) {
                return Err(SimultaneousDefensiveSelectionError::NumericalFailure);
            }

            let log_contribution = if normalized == 0.0
                || common_evaluation.log_importance_weight == f64::NEG_INFINITY
            {
                f64::NEG_INFINITY
            } else {
                log_epsilon + normalized.ln() + common_evaluation.log_importance_weight
            };
            if log_contribution.is_nan()
                || log_contribution == f64::INFINITY
                || log_contribution > 0.0
            {
                return Err(SimultaneousDefensiveSelectionError::NumericalFailure);
            }
            let defensive_contribution = if log_contribution == f64::NEG_INFINITY {
                0.0
            } else {
                log_contribution.exp()
            };
            if !defensive_contribution.is_finite() || !(0.0..=1.0).contains(&defensive_contribution)
            {
                return Err(SimultaneousDefensiveSelectionError::NumericalFailure);
            }

            if let Some(sequence) = next_sequences[index].as_mut() {
                sequence.observe(defensive_contribution)?;
            }

            pair_observations.push(DefensivePortfolioPairObservation {
                left_portfolio_id: pair.left_portfolio_id,
                right_portfolio_id: pair.right_portfolio_id,
                consequence_difference_ticks: difference,
                normalized_consequence_difference: normalized,
                defensive_contribution,
            });
        }

        for (pair, sequence) in self.pair_processes.iter_mut().zip(next_sequences) {
            pair.sequence = sequence;
        }
        self.draw_count = expected_index;
        self.seen_draw_ids.insert(draw.draw_id);

        Ok(DefensivePortfolioDrawReceipt {
            draw_index: draw.draw_index,
            consequence_unit_id: self.consequence_unit_id,
            draw_id: draw.draw_id,
            mode_key: draw.mode_key,
            target_measure_id: common_evaluation.target_measure_id,
            density_space_id: common_evaluation.density_space_id,
            configuration_digest: common_evaluation.configuration_digest,
            log_target_density: common_evaluation.log_target_density,
            log_full_proposal_density: common_evaluation.log_full_proposal_density,
            log_importance_weight: common_evaluation.log_importance_weight,
            portfolio_consequences: portfolio_consequence_receipts,
            pair_observations,
        })
    }

    /// Number of accepted complete shared draws.
    pub fn draw_count(&self) -> u64 {
        self.draw_count
    }

    /// Return all simultaneous pair intervals and the undominated portfolio set.
    pub fn report(
        &self,
    ) -> Result<SimultaneousDefensivePortfolioReport, SimultaneousDefensiveSelectionError> {
        let epsilon = self.proposal.defensive_fraction();
        let mut pairwise_intervals = Vec::with_capacity(self.pair_processes.len());

        for pair in &self.pair_processes {
            let (normalized_lower, normalized_upper) = if let Some(sequence) = &pair.sequence {
                let current = sequence.confidence_sequence();
                let lower = next_down(current.lower / epsilon).clamp(0.0, 1.0);
                let upper = next_up(current.upper / epsilon).clamp(0.0, 1.0);
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(SimultaneousDefensiveSelectionError::NumericalFailure);
                }
                (lower, upper)
            } else {
                (0.0, 0.0)
            };

            let width = (pair.maximum_difference_ticks - pair.minimum_difference_ticks) as f64;
            let lower = if width == 0.0 {
                pair.minimum_difference_ticks as f64
            } else {
                next_down(pair.minimum_difference_ticks as f64 + width * normalized_lower)
                    .max(pair.minimum_difference_ticks as f64)
            };
            let upper = if width == 0.0 {
                pair.maximum_difference_ticks as f64
            } else {
                next_up(pair.minimum_difference_ticks as f64 + width * normalized_upper)
                    .min(pair.maximum_difference_ticks as f64)
            };
            if !lower.is_finite() || !upper.is_finite() || lower > upper {
                return Err(SimultaneousDefensiveSelectionError::NumericalFailure);
            }
            pairwise_intervals.push(DefensivePortfolioPairInterval {
                left_portfolio_id: pair.left_portfolio_id,
                right_portfolio_id: pair.right_portfolio_id,
                sample_count: self.draw_count,
                contrast_alpha: self.contrast_alpha,
                lower_expected_difference_ticks: lower,
                upper_expected_difference_ticks: upper,
                lower_normalized_expected_difference: normalized_lower,
                upper_normalized_expected_difference: normalized_upper,
            });
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
                    .ok_or(SimultaneousDefensiveSelectionError::NumericalFailure)?;
                let candidate_is_left = candidate.portfolio_id == left_id;
                let candidate_is_proven_better = if candidate_is_left {
                    interval.upper_expected_difference_ticks < 0.0
                } else {
                    interval.lower_expected_difference_ticks > 0.0
                };
                let opponent_is_proven_better = if candidate_is_left {
                    interval.lower_expected_difference_ticks > 0.0
                } else {
                    interval.upper_expected_difference_ticks < 0.0
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

        Ok(SimultaneousDefensivePortfolioReport {
            consequence_unit_id: self.consequence_unit_id,
            family_alpha: self.family_alpha,
            portfolio_count: self.portfolio_bounds.len(),
            contrast_count: self.pair_processes.len(),
            contrast_alpha: self.contrast_alpha,
            draw_count: self.draw_count,
            pairwise_intervals,
            unique_best_portfolio_id: if uniquely_better.len() == 1 {
                Some(uniquely_better[0])
            } else {
                None
            },
            statistically_undominated_portfolio_ids: undominated,
            target_measure_id: self.proposal.target_measure_id(),
            density_space_id: self.proposal.density_space_id(),
            scenario_id: self.proposal.scenario_id(),
            configuration_digest: self.proposal.configuration_digest(),
            effective_defensive_fraction: epsilon,
        })
    }
}

fn next_down(value: f64) -> f64 {
    if !value.is_finite() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

fn next_up(value: f64) -> f64 {
    if !value.is_finite() || value == f64::INFINITY {
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
    use crate::{
        DefensiveProposalComponent, Gaussian, GaussianComponent, GaussianMixture,
        PortfolioConsequenceObservation,
    };

    const TARGET_MEASURE_ID: [u8; 32] = [90; 32];
    const DENSITY_SPACE_ID: [u8; 32] = [91; 32];
    const CONSEQUENCE_UNIT_ID: [u8; 32] = [92; 32];

    fn gaussian(mean: f64, variance: f64) -> Gaussian {
        Gaussian::new(vec![mean], vec![variance]).unwrap()
    }

    fn key(portfolio: u8) -> PortfolioEventModeKey {
        PortfolioEventModeKey {
            portfolio_id: [portfolio; 32],
            scenario_id: [1; 32],
            event_mode_id: [portfolio; 32],
        }
    }

    fn proposal_component(
        component: u8,
        portfolio: u8,
        weight: f64,
        mean: f64,
    ) -> DefensiveProposalComponent {
        DefensiveProposalComponent {
            component_id: [component; 32],
            mode_key: key(portfolio),
            allocation_weight: weight,
            distribution: gaussian(mean, 1.0),
        }
    }

    fn proposal_three_modes() -> PortfolioDefensiveGaussianMixture {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target,
            0.2,
            vec![key(10), key(20), key(30)],
            vec![
                proposal_component(1, 10, 1.0 / 3.0, 0.0),
                proposal_component(2, 20, 1.0 / 3.0, 0.0),
                proposal_component(3, 30, 1.0 / 3.0, 0.0),
            ],
        )
        .unwrap()
    }

    fn bounds() -> Vec<PortfolioConsequenceBound> {
        vec![
            PortfolioConsequenceBound {
                portfolio_id: [30; 32],
                lower_ticks: 0,
                upper_ticks: 20,
            },
            PortfolioConsequenceBound {
                portfolio_id: [10; 32],
                lower_ticks: 0,
                upper_ticks: 20,
            },
            PortfolioConsequenceBound {
                portfolio_id: [20; 32],
                lower_ticks: 0,
                upper_ticks: 20,
            },
        ]
    }

    fn draw(index: u64, draw_id: u64, outcomes: &[(u8, i32)]) -> DefensivePortfolioDraw {
        let mut identity = [0_u8; 32];
        identity[..8].copy_from_slice(&draw_id.to_be_bytes());
        DefensivePortfolioDraw {
            draw_index: index,
            draw_id: identity,
            mode_key: key(10),
            point: vec![0.0],
            portfolio_consequences: outcomes
                .iter()
                .map(|(id, consequence_ticks)| PortfolioConsequenceObservation {
                    portfolio_id: [*id; 32],
                    consequence_ticks: *consequence_ticks,
                })
                .collect(),
        }
    }

    #[test]
    fn family_alpha_is_split_over_all_pairwise_contrasts() {
        let process = SimultaneousDefensivePortfolioSelection::new(
            0.06,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            proposal_three_modes(),
        )
        .unwrap();
        let report = process.report().unwrap();

        assert_eq!(report.portfolio_count, 3);
        assert_eq!(report.contrast_count, 3);
        assert_eq!(report.family_alpha, 0.06);
        assert_eq!(report.contrast_alpha, 0.02);
        assert_eq!(report.pairwise_intervals.len(), 3);
        assert_eq!(report.unique_best_portfolio_id, None);
        assert_eq!(
            report.statistically_undominated_portfolio_ids,
            vec![[10; 32], [20; 32], [30; 32]]
        );
    }

    #[test]
    fn one_shared_draw_updates_every_pair_and_evaluates_density_once() {
        let mut process = SimultaneousDefensivePortfolioSelection::new(
            0.05,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            proposal_three_modes(),
        )
        .unwrap();
        let receipt = process
            .observe(&draw(1, 1, &[(30, 12), (10, 2), (20, 7)]))
            .unwrap();

        assert_eq!(receipt.draw_index, 1);
        let mut expected_id = [0_u8; 32];
        expected_id[..8].copy_from_slice(&1_u64.to_be_bytes());
        assert_eq!(receipt.draw_id, expected_id);
        assert_eq!(receipt.portfolio_consequences.len(), 3);
        assert_eq!(
            receipt
                .portfolio_consequences
                .iter()
                .map(|item| (item.portfolio_id[0], item.consequence_ticks))
                .collect::<Vec<_>>(),
            vec![(10, 2), (20, 7), (30, 12)]
        );
        assert_eq!(receipt.pair_observations.len(), 3);
        assert!(receipt.log_target_density.is_finite());
        assert!(receipt.log_full_proposal_density.is_finite());
        assert!(receipt.pair_observations.iter().all(|item| {
            (0.0..=1.0).contains(&item.normalized_consequence_difference)
                && (0.0..=1.0).contains(&item.defensive_contribution)
        }));
        assert_eq!(process.draw_count(), 1);
        assert_eq!(process.report().unwrap().draw_count, 1);
    }

    #[test]
    fn shared_draw_ledger_rejects_duplicate_ids_and_duplicate_indices() {
        let mut process = SimultaneousDefensivePortfolioSelection::new(
            0.05,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            proposal_three_modes(),
        )
        .unwrap();
        process
            .observe(&draw(1, 9, &[(10, 1), (20, 2), (30, 3)]))
            .unwrap();

        assert_eq!(
            process.observe(&draw(2, 9, &[(10, 1), (20, 2), (30, 3)])),
            Err(SimultaneousDefensiveSelectionError::InvalidOrDuplicateDrawIdentity)
        );
        assert_eq!(
            process.observe(&draw(1, 10, &[(10, 1), (20, 2), (30, 3)])),
            Err(SimultaneousDefensiveSelectionError::InvalidDrawIndex)
        );
        assert_eq!(process.draw_count(), 1);
    }

    #[test]
    fn missing_duplicate_and_out_of_bound_portfolio_rows_are_rejected_transactionally() {
        let mut process = SimultaneousDefensivePortfolioSelection::new(
            0.05,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            proposal_three_modes(),
        )
        .unwrap();

        assert_eq!(
            process.observe(&draw(1, 1, &[(10, 1), (20, 2)])),
            Err(SimultaneousDefensiveSelectionError::IncompletePortfolioDraw)
        );
        assert_eq!(
            process.observe(&draw(1, 1, &[(10, 1), (20, 2), (20, 3)])),
            Err(SimultaneousDefensiveSelectionError::InvalidDrawPortfolioIdentity)
        );
        assert_eq!(
            process.observe(&draw(1, 1, &[(10, 1), (20, 2), (30, 21)])),
            Err(SimultaneousDefensiveSelectionError::ConsequenceOutsideFrozenBounds)
        );
        assert_eq!(process.draw_count(), 0);

        process
            .observe(&draw(1, 1, &[(10, 1), (20, 2), (30, 3)]))
            .unwrap();
        assert_eq!(process.draw_count(), 1);
    }

    #[test]
    fn zero_consequence_unit_identity_is_rejected() {
        assert_eq!(
            SimultaneousDefensivePortfolioSelection::new(
                0.05,
                [0; 32],
                bounds(),
                proposal_three_modes(),
            )
            .unwrap_err(),
            SimultaneousDefensiveSelectionError::InvalidConsequenceUnitIdentity
        );
    }

    #[test]
    fn omitted_proposal_portfolio_is_rejected_at_construction() {
        let one_target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        let restricted = PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            one_target,
            0.2,
            vec![key(10), key(20)],
            vec![
                proposal_component(1, 10, 0.5, -5.0),
                proposal_component(2, 20, 0.5, 5.0),
            ],
        )
        .unwrap();
        assert_eq!(
            SimultaneousDefensivePortfolioSelection::new(
                0.05,
                CONSEQUENCE_UNIT_ID,
                bounds(),
                restricted
            )
            .unwrap_err(),
            SimultaneousDefensiveSelectionError::IncompleteProposalPortfolioCoverage
        );
    }

    #[test]
    fn separated_best_portfolio_is_reported_only_after_all_pairs_are_resolved() {
        let mut process = SimultaneousDefensivePortfolioSelection::new(
            0.05,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            proposal_three_modes(),
        )
        .unwrap();
        for index in 1..=2_000 {
            process
                .observe(&draw(index, index, &[(10, 0), (20, 10), (30, 20)]))
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
        panic!("well-separated defensive best portfolio was not certified within 2,000 draws");
    }

    #[test]
    fn degenerate_pair_contrast_produces_exact_interval() {
        let fixed = vec![
            PortfolioConsequenceBound {
                portfolio_id: [10; 32],
                lower_ticks: 5,
                upper_ticks: 5,
            },
            PortfolioConsequenceBound {
                portfolio_id: [20; 32],
                lower_ticks: 2,
                upper_ticks: 2,
            },
        ];
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        let proposal = PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target,
            0.2,
            vec![key(10), key(20)],
            vec![
                proposal_component(1, 10, 0.5, -2.0),
                proposal_component(2, 20, 0.5, 2.0),
            ],
        )
        .unwrap();
        let process = SimultaneousDefensivePortfolioSelection::new(
            0.05,
            CONSEQUENCE_UNIT_ID,
            fixed,
            proposal,
        )
        .unwrap();
        let report = process.report().unwrap();
        assert_eq!(
            report.pairwise_intervals[0].lower_expected_difference_ticks,
            3.0
        );
        assert_eq!(
            report.pairwise_intervals[0].upper_expected_difference_ticks,
            3.0
        );
    }
}
