// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Direct multi-portfolio consequence comparison using the repository's
//! independently replayed finite-support adaptive defensive sampler.
//!
//! For target mass p(x), predictable defensive proposal q_t(x) >= epsilon*p(x)
//! and frozen-bound normalized consequence g_ab(x) in [0,1], observe
//!
//!     Z_ab,t = epsilon * p(x_t) / q_t(x_t) * g_ab(x_t).
//!
//! The finite proposal verifier reconstructs q_t from recorded branch/atom
//! tickets and accepted prior union-event history. It does not prove that the
//! tickets were uniformly or independently generated.
//!
//! Every shared atom updates all pairwise PrPl-EB processes transactionally.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::paired_counterfactual::MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS;
use crate::predictable_empirical_bernstein::{PrPlEbConfidenceSequence, PrPlEbError};
use crate::{
    FiniteAdaptiveProposalConfig, FiniteProposalExecutionError, FiniteProposalExecutionReceipt,
    FiniteProposalExecutionVerifier, PortfolioConsequenceBound, PortfolioConsequenceObservation,
    SimultaneousDefensivePortfolioReport, TICKET_SPACE, VerifiedFiniteProposalDraw,
};

/// One portfolio's event indicator for a finite target-support atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FinitePortfolioEventFlag {
    /// Stable portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Whether this portfolio's complete mission-event definition occurs on this atom.
    pub event_occurred: bool,
}

/// Direct, complete portfolio outcomes for one atom in the frozen target support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinitePortfolioAtomOutcome {
    /// Stable atom identity matching one atom in the finite proposal configuration.
    pub atom_id: [u8; 32],
    /// Directly evaluated consequence for every portfolio exactly once.
    pub portfolio_consequences: Vec<PortfolioConsequenceObservation>,
    /// Event flag for every portfolio exactly once.
    pub portfolio_events: Vec<FinitePortfolioEventFlag>,
}

/// One direct pairwise consequence and adaptive defensive observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteAdaptivePortfolioPairObservation {
    /// Canonically smaller portfolio identity.
    pub left_portfolio_id: [u8; 32],
    /// Canonically larger portfolio identity.
    pub right_portfolio_id: [u8; 32],
    /// Direct left-minus-right consequence in integer ticks.
    pub consequence_difference_ticks: i64,
    /// Frozen-bound normalized contrast in [0,1].
    pub normalized_consequence_difference: f64,
    /// Reconstructed epsilon*p(x)/q_t(x), from exact integer mixture masses.
    pub defensive_likelihood_ratio: f64,
    /// Bounded defensive PrPl-EB observation.
    pub defensive_contribution: f64,
}

/// Complete local evidence for one accepted adaptive portfolio draw.
#[derive(Debug, Clone, PartialEq)]
pub struct FiniteAdaptivePortfolioDrawReceipt {
    /// Original sampler execution receipt with branch and atom tickets.
    pub execution_receipt: FiniteProposalExecutionReceipt,
    /// Independent exact reconstruction of selected atom and complete q_t mass.
    pub verified_draw: VerifiedFiniteProposalDraw,
    /// Campaign digest over support, proposal policy, scenario and outcome table.
    pub campaign_digest: [u8; 32],
    /// Canonical direct portfolio outcomes for the selected atom.
    pub portfolio_outcome: FinitePortfolioAtomOutcome,
    /// All portfolio-pair observations on this one shared finite atom.
    pub pair_observations: Vec<FiniteAdaptivePortfolioPairObservation>,
}

/// Fail-closed validation, sampler replay or statistical update errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiniteAdaptivePortfolioError {
    /// Total per-scenario family alpha must lie strictly in (0,1).
    InvalidFamilyAlpha,
    /// Scenario, density-space or consequence-unit identity is zero.
    InvalidCampaignIdentity,
    /// Defensive proposal fraction must lie in (0,1].
    InvalidDefensiveFraction,
    /// Portfolio universe is empty, duplicated, zero or too large.
    InvalidPortfolioUniverse,
    /// A consequence bound is reversed.
    InvalidConsequenceBound,
    /// Outcome atom universe does not exactly match finite proposal support.
    IncompleteAtomOutcomeUniverse,
    /// Atom identity is zero, duplicated or unexpected.
    InvalidAtomOutcomeIdentity,
    /// Consequence rows do not contain every portfolio exactly once.
    IncompletePortfolioOutcomeRow,
    /// Event rows do not contain every portfolio exactly once.
    IncompletePortfolioEventRow,
    /// One direct consequence lies outside its frozen arm bound.
    ConsequenceOutsideFrozenBounds,
    /// Declared sampler event bit differs from OR of portfolio event flags.
    UnionEventDefinitionMismatch,
    /// Sampler verifier rejected its receipt or prior-history proposal.
    SamplerVerificationFailure,
    /// Reconstructed defensive likelihood ratio is invalid or exceeds one.
    DefensiveLikelihoodRatioInvalid,
    /// One bounded pairwise defensive contribution is invalid.
    DefensiveContributionInvalid,
    /// Pairwise PrPl-EB update failed.
    ConfidenceSequenceFailure,
    /// Pairwise interval or selection report failed numerically.
    NumericalFailure,
}

impl core::fmt::Display for FiniteAdaptivePortfolioError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidFamilyAlpha => "portfolio family-wise alpha must lie in (0,1)",
            Self::InvalidCampaignIdentity => {
                "scenario, density-space or consequence identity is zero"
            }
            Self::InvalidDefensiveFraction => "defensive proposal fraction must lie in (0,1]",
            Self::InvalidPortfolioUniverse => "portfolio universe is invalid",
            Self::InvalidConsequenceBound => "portfolio consequence bound is reversed",
            Self::IncompleteAtomOutcomeUniverse => {
                "atom outcome universe does not match target support"
            }
            Self::InvalidAtomOutcomeIdentity => {
                "atom outcome identity is zero, duplicated or unexpected"
            }
            Self::IncompletePortfolioOutcomeRow => {
                "atom consequence row omits or duplicates a portfolio"
            }
            Self::IncompletePortfolioEventRow => "atom event row omits or duplicates a portfolio",
            Self::ConsequenceOutsideFrozenBounds => {
                "direct consequence is outside its frozen portfolio bound"
            }
            Self::UnionEventDefinitionMismatch => {
                "sampler event bit differs from the portfolio event union"
            }
            Self::SamplerVerificationFailure => {
                "finite adaptive sampler receipt failed independent replay"
            }
            Self::DefensiveLikelihoodRatioInvalid => {
                "exact defensive likelihood ratio is outside [0,1]"
            }
            Self::DefensiveContributionInvalid => {
                "defensive portfolio observation is outside [0,1]"
            }
            Self::ConfidenceSequenceFailure => "pairwise PrPl-EB confidence update failed",
            Self::NumericalFailure => "finite adaptive portfolio calculation failed numerically",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for FiniteAdaptivePortfolioError {}

impl From<FiniteProposalExecutionError> for FiniteAdaptivePortfolioError {
    fn from(_: FiniteProposalExecutionError) -> Self {
        Self::SamplerVerificationFailure
    }
}

impl From<PrPlEbError> for FiniteAdaptivePortfolioError {
    fn from(_: PrPlEbError) -> Self {
        Self::ConfidenceSequenceFailure
    }
}

#[derive(Debug, Clone, PartialEq)]
struct FinitePortfolioPairProcess {
    left_portfolio_id: [u8; 32],
    right_portfolio_id: [u8; 32],
    minimum_difference_ticks: i64,
    maximum_difference_ticks: i64,
    sequence: Option<PrPlEbConfidenceSequence>,
}

/// Predictable finite-support adaptive defensive portfolio selector.
///
/// The sampler verifier chooses a residual proposal using only previously
/// accepted union-event history. A new receipt is replayed on a cloned verifier;
/// all pairwise confidence processes are cloned too. No campaign state commits
/// unless every sampler and statistical check succeeds.
#[derive(Debug, Clone)]
pub struct FiniteAdaptivePortfolioSelection {
    family_alpha: f64,
    contrast_alpha: f64,
    scenario_id: [u8; 32],
    density_space_id: [u8; 32],
    consequence_unit_id: [u8; 32],
    target_measure_id: [u8; 32],
    support_manifest_digest: [u8; 32],
    event_digest: [u8; 32],
    effective_defensive_fraction: f64,
    campaign_digest: [u8; 32],
    portfolio_bounds: Vec<PortfolioConsequenceBound>,
    atom_outcomes: BTreeMap<[u8; 32], FinitePortfolioAtomOutcome>,
    verifier: FiniteProposalExecutionVerifier,
    pair_processes: Vec<FinitePortfolioPairProcess>,
}

impl FiniteAdaptivePortfolioSelection {
    /// Freeze finite target support, complete portfolio outcomes and predictable
    /// defensive sampler policy.
    ///
    /// The family alpha is the per-scenario allocation. When multiple epistemic
    /// scenarios are evaluated, allocate total alpha across scenarios first.
    pub fn new(
        config: FiniteAdaptiveProposalConfig,
        family_alpha: f64,
        scenario_id: [u8; 32],
        density_space_id: [u8; 32],
        consequence_unit_id: [u8; 32],
        mut portfolio_bounds: Vec<PortfolioConsequenceBound>,
        atom_outcomes: Vec<FinitePortfolioAtomOutcome>,
    ) -> Result<Self, FiniteAdaptivePortfolioError> {
        if !family_alpha.is_finite() || family_alpha <= 0.0 || family_alpha >= 1.0 {
            return Err(FiniteAdaptivePortfolioError::InvalidFamilyAlpha);
        }
        if scenario_id.iter().all(|b| *b == 0)
            || density_space_id.iter().all(|b| *b == 0)
            || consequence_unit_id.iter().all(|b| *b == 0)
        {
            return Err(FiniteAdaptivePortfolioError::InvalidCampaignIdentity);
        }
        if config.defensive_mixture_weight_ticks == 0
            || config.defensive_mixture_weight_ticks > TICKET_SPACE
        {
            return Err(FiniteAdaptivePortfolioError::InvalidDefensiveFraction);
        }
        if portfolio_bounds.len() < 2
            || portfolio_bounds.len() > MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS
        {
            return Err(FiniteAdaptivePortfolioError::InvalidPortfolioUniverse);
        }

        let verifier = FiniteProposalExecutionVerifier::new(config.clone())?;
        portfolio_bounds.sort_unstable_by_key(|bound| bound.portfolio_id);
        for (index, bound) in portfolio_bounds.iter().enumerate() {
            if bound.portfolio_id.iter().all(|b| *b == 0)
                || (index > 0 && portfolio_bounds[index - 1].portfolio_id == bound.portfolio_id)
            {
                return Err(FiniteAdaptivePortfolioError::InvalidPortfolioUniverse);
            }
            if bound.lower_ticks > bound.upper_ticks {
                return Err(FiniteAdaptivePortfolioError::InvalidConsequenceBound);
            }
        }

        if atom_outcomes.len() != config.atoms.len() || atom_outcomes.is_empty() {
            return Err(FiniteAdaptivePortfolioError::IncompleteAtomOutcomeUniverse);
        }
        let mut expected_atom_ids = BTreeSet::new();
        for atom in &config.atoms {
            if atom.atom_id.iter().all(|b| *b == 0) || !expected_atom_ids.insert(atom.atom_id) {
                return Err(FiniteAdaptivePortfolioError::InvalidAtomOutcomeIdentity);
            }
        }

        let mut outcomes_by_atom = BTreeMap::new();
        for mut outcome in atom_outcomes {
            if outcome.atom_id.iter().all(|b| *b == 0)
                || !expected_atom_ids.contains(&outcome.atom_id)
                || outcomes_by_atom.contains_key(&outcome.atom_id)
            {
                return Err(FiniteAdaptivePortfolioError::InvalidAtomOutcomeIdentity);
            }

            if outcome.portfolio_consequences.len() != portfolio_bounds.len() {
                return Err(FiniteAdaptivePortfolioError::IncompletePortfolioOutcomeRow);
            }
            outcome
                .portfolio_consequences
                .sort_unstable_by_key(|item| item.portfolio_id);
            for (index, item) in outcome.portfolio_consequences.iter().enumerate() {
                let Some(bound) = portfolio_bounds.get(index) else {
                    return Err(FiniteAdaptivePortfolioError::IncompletePortfolioOutcomeRow);
                };
                if item.portfolio_id != bound.portfolio_id
                    || (index > 0
                        && outcome.portfolio_consequences[index - 1].portfolio_id
                            == item.portfolio_id)
                {
                    return Err(FiniteAdaptivePortfolioError::IncompletePortfolioOutcomeRow);
                }
                if item.consequence_ticks < bound.lower_ticks
                    || item.consequence_ticks > bound.upper_ticks
                {
                    return Err(FiniteAdaptivePortfolioError::ConsequenceOutsideFrozenBounds);
                }
            }

            if outcome.portfolio_events.len() != portfolio_bounds.len() {
                return Err(FiniteAdaptivePortfolioError::IncompletePortfolioEventRow);
            }
            outcome
                .portfolio_events
                .sort_unstable_by_key(|item| item.portfolio_id);
            let mut union_event = false;
            for (index, item) in outcome.portfolio_events.iter().enumerate() {
                if item.portfolio_id != portfolio_bounds[index].portfolio_id
                    || (index > 0
                        && outcome.portfolio_events[index - 1].portfolio_id == item.portfolio_id)
                {
                    return Err(FiniteAdaptivePortfolioError::IncompletePortfolioEventRow);
                }
                union_event |= item.event_occurred;
            }

            let declared_event = config
                .atoms
                .iter()
                .find(|atom| atom.atom_id == outcome.atom_id)
                .map(|atom| atom.is_event)
                .ok_or(FiniteAdaptivePortfolioError::InvalidAtomOutcomeIdentity)?;
            if union_event != declared_event {
                return Err(FiniteAdaptivePortfolioError::UnionEventDefinitionMismatch);
            }

            outcomes_by_atom.insert(outcome.atom_id, outcome);
        }

        if outcomes_by_atom.len() != expected_atom_ids.len()
            || expected_atom_ids
                .iter()
                .any(|atom_id| !outcomes_by_atom.contains_key(atom_id))
        {
            return Err(FiniteAdaptivePortfolioError::IncompleteAtomOutcomeUniverse);
        }

        let portfolio_count = portfolio_bounds.len();
        let pair_count = portfolio_count
            .checked_mul(portfolio_count - 1)
            .and_then(|n| n.checked_div(2))
            .ok_or(FiniteAdaptivePortfolioError::NumericalFailure)?;
        let contrast_alpha = family_alpha / pair_count as f64;
        if !contrast_alpha.is_finite() || contrast_alpha <= 0.0 || contrast_alpha >= 1.0 {
            return Err(FiniteAdaptivePortfolioError::NumericalFailure);
        }

        let mut pair_processes = Vec::with_capacity(pair_count);
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
                pair_processes.push(FinitePortfolioPairProcess {
                    left_portfolio_id: left.portfolio_id,
                    right_portfolio_id: right.portfolio_id,
                    minimum_difference_ticks,
                    maximum_difference_ticks,
                    sequence,
                });
            }
        }

        let effective_defensive_fraction =
            f64::from(config.defensive_mixture_weight_ticks) / f64::from(TICKET_SPACE);
        if !effective_defensive_fraction.is_finite()
            || effective_defensive_fraction <= 0.0
            || effective_defensive_fraction > 1.0
        {
            return Err(FiniteAdaptivePortfolioError::NumericalFailure);
        }
        let campaign_digest = campaign_digest(
            &config,
            family_alpha,
            scenario_id,
            density_space_id,
            consequence_unit_id,
            &portfolio_bounds,
            &outcomes_by_atom,
        );

        Ok(Self {
            family_alpha,
            contrast_alpha,
            scenario_id,
            density_space_id,
            consequence_unit_id,
            target_measure_id: config.target_measure_digest,
            support_manifest_digest: config.support_manifest_digest,
            event_digest: config.event_digest,
            effective_defensive_fraction,
            campaign_digest,
            portfolio_bounds,
            atom_outcomes: outcomes_by_atom,
            verifier,
            pair_processes,
        })
    }

    /// Replay, validate and atomically accept one sampler execution receipt.
    ///
    /// The exact target/proposal likelihood ratio is reconstructed as
    /// (defensive_weight_ticks * target_mass_ticks) / mixture_mass_numerator.
    /// It is intentionally not taken from the event-indicator-specific
    /// normalized contribution, because general portfolio consequences are
    /// evaluated on every target atom, including non-event atoms.
    pub fn observe(
        &mut self,
        receipt: FiniteProposalExecutionReceipt,
    ) -> Result<FiniteAdaptivePortfolioDrawReceipt, FiniteAdaptivePortfolioError> {
        let mut next_verifier = self.verifier.clone();
        let verified_draw = next_verifier.verify_next(receipt)?;
        let outcome = self
            .atom_outcomes
            .get(&verified_draw.selected_atom_id)
            .cloned()
            .ok_or(FiniteAdaptivePortfolioError::InvalidAtomOutcomeIdentity)?;

        if verified_draw.support_manifest_digest != self.support_manifest_digest
            || verified_draw.event_digest != self.event_digest
            || verified_draw.target_measure_digest != self.target_measure_id
        {
            return Err(FiniteAdaptivePortfolioError::SamplerVerificationFailure);
        }

        let defensive_numerator = u64::from(verified_draw.defensive_mixture_weight_ticks)
            .checked_mul(u64::from(verified_draw.target_mass_ticks))
            .ok_or(FiniteAdaptivePortfolioError::NumericalFailure)?;
        let defensive_denominator = verified_draw.mixture_mass_numerator;
        if defensive_denominator == 0 || defensive_numerator > defensive_denominator {
            return Err(FiniteAdaptivePortfolioError::DefensiveLikelihoodRatioInvalid);
        }
        let defensive_likelihood_ratio = defensive_numerator as f64 / defensive_denominator as f64;
        if !defensive_likelihood_ratio.is_finite()
            || !(0.0..=1.0).contains(&defensive_likelihood_ratio)
        {
            return Err(FiniteAdaptivePortfolioError::DefensiveLikelihoodRatioInvalid);
        }

        let observed = outcome
            .portfolio_consequences
            .iter()
            .map(|item| (item.portfolio_id, item.consequence_ticks))
            .collect::<BTreeMap<_, _>>();
        let mut next_sequences = self
            .pair_processes
            .iter()
            .map(|pair| pair.sequence.clone())
            .collect::<Vec<_>>();
        let mut pair_observations = Vec::with_capacity(self.pair_processes.len());

        for (index, pair) in self.pair_processes.iter().enumerate() {
            let left = i64::from(
                *observed
                    .get(&pair.left_portfolio_id)
                    .ok_or(FiniteAdaptivePortfolioError::IncompletePortfolioOutcomeRow)?,
            );
            let right = i64::from(
                *observed
                    .get(&pair.right_portfolio_id)
                    .ok_or(FiniteAdaptivePortfolioError::IncompletePortfolioOutcomeRow)?,
            );
            let difference = left - right;
            let width = i128::from(pair.maximum_difference_ticks)
                - i128::from(pair.minimum_difference_ticks);
            let offset = i128::from(difference) - i128::from(pair.minimum_difference_ticks);
            let normalized = if width == 0 {
                0.0
            } else {
                offset as f64 / width as f64
            };
            if !normalized.is_finite() || !(0.0..=1.0).contains(&normalized) {
                return Err(FiniteAdaptivePortfolioError::NumericalFailure);
            }

            let defensive_contribution = defensive_likelihood_ratio * normalized;
            if !defensive_contribution.is_finite() || !(0.0..=1.0).contains(&defensive_contribution)
            {
                return Err(FiniteAdaptivePortfolioError::DefensiveContributionInvalid);
            }
            if let Some(sequence) = next_sequences[index].as_mut() {
                sequence.observe(defensive_contribution)?;
            }

            pair_observations.push(FiniteAdaptivePortfolioPairObservation {
                left_portfolio_id: pair.left_portfolio_id,
                right_portfolio_id: pair.right_portfolio_id,
                consequence_difference_ticks: difference,
                normalized_consequence_difference: normalized,
                defensive_likelihood_ratio,
                defensive_contribution,
            });
        }

        for (pair, sequence) in self.pair_processes.iter_mut().zip(next_sequences) {
            pair.sequence = sequence;
        }
        self.verifier = next_verifier;

        Ok(FiniteAdaptivePortfolioDrawReceipt {
            execution_receipt: receipt,
            verified_draw,
            campaign_digest: self.campaign_digest,
            portfolio_outcome: outcome,
            pair_observations,
        })
    }

    /// Number of independently replay-verified accepted finite-support draws.
    pub fn draw_count(&self) -> u64 {
        self.verifier.sample_count()
    }

    /// Return pairwise target-mean intervals in the common portfolio report format.
    pub fn report(
        &self,
    ) -> Result<SimultaneousDefensivePortfolioReport, FiniteAdaptivePortfolioError> {
        let mut pairwise_intervals = Vec::with_capacity(self.pair_processes.len());
        for pair in &self.pair_processes {
            let (normalized_lower, normalized_upper) = if let Some(sequence) = &pair.sequence {
                let current = sequence.confidence_sequence();
                let lower =
                    next_down(current.lower / self.effective_defensive_fraction).clamp(0.0, 1.0);
                let upper =
                    next_up(current.upper / self.effective_defensive_fraction).clamp(0.0, 1.0);
                if !lower.is_finite() || !upper.is_finite() || lower > upper {
                    return Err(FiniteAdaptivePortfolioError::NumericalFailure);
                }
                (lower, upper)
            } else {
                (0.0, 0.0)
            };

            let width = (i128::from(pair.maximum_difference_ticks)
                - i128::from(pair.minimum_difference_ticks)) as f64;
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
                return Err(FiniteAdaptivePortfolioError::NumericalFailure);
            }
            pairwise_intervals.push(crate::DefensivePortfolioPairInterval {
                left_portfolio_id: pair.left_portfolio_id,
                right_portfolio_id: pair.right_portfolio_id,
                sample_count: self.draw_count(),
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
            let mut beats_every_other = self.draw_count() > 0;
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
                    .ok_or(FiniteAdaptivePortfolioError::NumericalFailure)?;
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
            draw_count: self.draw_count(),
            pairwise_intervals,
            unique_best_portfolio_id: if uniquely_better.len() == 1 {
                Some(uniquely_better[0])
            } else {
                None
            },
            statistically_undominated_portfolio_ids: undominated,
            target_measure_id: self.target_measure_id,
            density_space_id: self.density_space_id,
            scenario_id: self.scenario_id,
            configuration_digest: self.campaign_digest,
            effective_defensive_fraction: self.effective_defensive_fraction,
        })
    }
}

fn campaign_digest(
    config: &FiniteAdaptiveProposalConfig,
    family_alpha: f64,
    scenario_id: [u8; 32],
    density_space_id: [u8; 32],
    consequence_unit_id: [u8; 32],
    portfolio_bounds: &[PortfolioConsequenceBound],
    atom_outcomes: &BTreeMap<[u8; 32], FinitePortfolioAtomOutcome>,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital.finite-adaptive-portfolio.v0.1\0");
    hasher.update(family_alpha.to_bits().to_be_bytes());
    hasher.update(scenario_id);
    hasher.update(density_space_id);
    hasher.update(consequence_unit_id);
    hasher.update(config.support_manifest_digest);
    hasher.update(config.event_digest);
    hasher.update(config.target_measure_digest);
    hasher.update(config.defensive_component_digest);
    hasher.update(config.proposal_policy_digest);
    hasher.update(config.sampler_implementation_digest);
    hasher.update(config.rng_algorithm_digest);
    hasher.update(config.rng_seed_commitment_digest);
    hasher.update(config.branch_stream_digest);
    hasher.update(config.atom_stream_digest);
    hasher.update(config.residual_before_event_digest);
    hasher.update(config.residual_after_event_digest);
    hasher.update(config.complete_proposal_before_event_digest);
    hasher.update(config.complete_proposal_after_event_digest);
    hasher.update(config.defensive_mixture_weight_ticks.to_be_bytes());

    // The atom order is part of the sampler contract: inverse-CDF tickets
    // map through this exact order. Bind each atom identity next to its event
    // bit rather than relying on the order-independent outcome table.
    hasher.update((config.atoms.len() as u64).to_be_bytes());
    for atom in &config.atoms {
        hasher.update(atom.atom_id);
        hasher.update([u8::from(atom.is_event)]);
    }
    hasher.update((portfolio_bounds.len() as u64).to_be_bytes());
    for bound in portfolio_bounds {
        hasher.update(bound.portfolio_id);
        hasher.update(bound.lower_ticks.to_be_bytes());
        hasher.update(bound.upper_ticks.to_be_bytes());
    }
    hasher.update((atom_outcomes.len() as u64).to_be_bytes());
    for (atom_id, outcome) in atom_outcomes {
        hasher.update(atom_id);
        hasher.update((outcome.portfolio_consequences.len() as u64).to_be_bytes());
        for item in &outcome.portfolio_consequences {
            hasher.update(item.portfolio_id);
            hasher.update(item.consequence_ticks.to_be_bytes());
        }
        hasher.update((outcome.portfolio_events.len() as u64).to_be_bytes());
        for item in &outcome.portfolio_events {
            hasher.update(item.portfolio_id);
            hasher.update([u8::from(item.event_occurred)]);
        }
    }
    hasher.update((config.target_mass_ticks.len() as u64).to_be_bytes());
    for mass in &config.target_mass_ticks {
        hasher.update(mass.to_be_bytes());
    }
    hasher.update((config.residual_before_event_ticks.len() as u64).to_be_bytes());
    for mass in &config.residual_before_event_ticks {
        hasher.update(mass.to_be_bytes());
    }
    hasher.update((config.residual_after_event_ticks.len() as u64).to_be_bytes());
    for mass in &config.residual_after_event_ticks {
        hasher.update(mass.to_be_bytes());
    }
    hasher.finalize().into()
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
    use crate::FiniteProposalAtom;

    const SCENARIO_ID: [u8; 32] = [31; 32];
    const DENSITY_SPACE_ID: [u8; 32] = [32; 32];
    const CONSEQUENCE_UNIT_ID: [u8; 32] = [33; 32];
    const ATOM_A: [u8; 32] = [41; 32];
    const ATOM_B: [u8; 32] = [42; 32];

    fn config() -> FiniteAdaptiveProposalConfig {
        FiniteAdaptiveProposalConfig {
            support_manifest_digest: [1; 32],
            event_digest: [2; 32],
            target_measure_digest: [3; 32],
            defensive_component_digest: [4; 32],
            proposal_policy_digest: [5; 32],
            sampler_implementation_digest: [6; 32],
            rng_algorithm_digest: [7; 32],
            rng_seed_commitment_digest: [8; 32],
            branch_stream_digest: [9; 32],
            atom_stream_digest: [10; 32],
            residual_before_event_digest: [11; 32],
            residual_after_event_digest: [12; 32],
            complete_proposal_before_event_digest: [13; 32],
            complete_proposal_after_event_digest: [14; 32],
            defensive_mixture_weight_ticks: 128,
            atoms: vec![
                FiniteProposalAtom {
                    atom_id: ATOM_A,
                    is_event: false,
                },
                FiniteProposalAtom {
                    atom_id: ATOM_B,
                    is_event: true,
                },
            ],
            target_mass_ticks: vec![128, 128],
            residual_before_event_ticks: vec![256, 0],
            residual_after_event_ticks: vec![0, 256],
        }
    }

    fn outcomes() -> Vec<FinitePortfolioAtomOutcome> {
        vec![
            FinitePortfolioAtomOutcome {
                atom_id: ATOM_A,
                portfolio_consequences: vec![
                    PortfolioConsequenceObservation {
                        portfolio_id: [10; 32],
                        consequence_ticks: 0,
                    },
                    PortfolioConsequenceObservation {
                        portfolio_id: [20; 32],
                        consequence_ticks: 10,
                    },
                    PortfolioConsequenceObservation {
                        portfolio_id: [30; 32],
                        consequence_ticks: 5,
                    },
                ],
                portfolio_events: vec![
                    FinitePortfolioEventFlag {
                        portfolio_id: [10; 32],
                        event_occurred: false,
                    },
                    FinitePortfolioEventFlag {
                        portfolio_id: [20; 32],
                        event_occurred: false,
                    },
                    FinitePortfolioEventFlag {
                        portfolio_id: [30; 32],
                        event_occurred: false,
                    },
                ],
            },
            FinitePortfolioAtomOutcome {
                atom_id: ATOM_B,
                portfolio_consequences: vec![
                    PortfolioConsequenceObservation {
                        portfolio_id: [10; 32],
                        consequence_ticks: 10,
                    },
                    PortfolioConsequenceObservation {
                        portfolio_id: [20; 32],
                        consequence_ticks: 0,
                    },
                    PortfolioConsequenceObservation {
                        portfolio_id: [30; 32],
                        consequence_ticks: 5,
                    },
                ],
                portfolio_events: vec![
                    FinitePortfolioEventFlag {
                        portfolio_id: [10; 32],
                        event_occurred: true,
                    },
                    FinitePortfolioEventFlag {
                        portfolio_id: [20; 32],
                        event_occurred: false,
                    },
                    FinitePortfolioEventFlag {
                        portfolio_id: [30; 32],
                        event_occurred: false,
                    },
                ],
            },
        ]
    }

    fn bounds() -> Vec<PortfolioConsequenceBound> {
        vec![
            PortfolioConsequenceBound {
                portfolio_id: [30; 32],
                lower_ticks: 0,
                upper_ticks: 10,
            },
            PortfolioConsequenceBound {
                portfolio_id: [10; 32],
                lower_ticks: 0,
                upper_ticks: 10,
            },
            PortfolioConsequenceBound {
                portfolio_id: [20; 32],
                lower_ticks: 0,
                upper_ticks: 10,
            },
        ]
    }

    fn campaign() -> FiniteAdaptivePortfolioSelection {
        FiniteAdaptivePortfolioSelection::new(
            config(),
            0.05,
            SCENARIO_ID,
            DENSITY_SPACE_ID,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            outcomes(),
        )
        .unwrap()
    }

    fn receipt_for_config(
        c: &FiniteAdaptiveProposalConfig,
        sequence_index: u64,
        sample_id: u8,
        prior_event: bool,
        branch_ticket: u16,
        atom_ticket: u16,
    ) -> FiniteProposalExecutionReceipt {
        FiniteProposalExecutionReceipt {
            sequence_index,
            sample_id: [sample_id; 32],
            support_manifest_digest: c.support_manifest_digest,
            event_digest: c.event_digest,
            target_measure_digest: c.target_measure_digest,
            defensive_component_digest: c.defensive_component_digest,
            proposal_policy_digest: c.proposal_policy_digest,
            sampler_implementation_digest: c.sampler_implementation_digest,
            rng_algorithm_digest: c.rng_algorithm_digest,
            rng_seed_commitment_digest: c.rng_seed_commitment_digest,
            branch_stream_digest: c.branch_stream_digest,
            atom_stream_digest: c.atom_stream_digest,
            proposal_digest: if prior_event {
                c.residual_after_event_digest
            } else {
                c.residual_before_event_digest
            },
            complete_proposal_digest: if prior_event {
                c.complete_proposal_after_event_digest
            } else {
                c.complete_proposal_before_event_digest
            },
            defensive_mixture_weight_ticks: c.defensive_mixture_weight_ticks,
            branch_ticket,
            atom_ticket,
            selected_atom_id: ATOM_B,
        }
    }

    fn receipt(
        sequence_index: u64,
        sample_id: u8,
        prior_event: bool,
        branch_ticket: u16,
        atom_ticket: u16,
    ) -> FiniteProposalExecutionReceipt {
        receipt_for_config(
            &config(),
            sequence_index,
            sample_id,
            prior_event,
            branch_ticket,
            atom_ticket,
        )
    }

    #[test]
    fn finite_adaptive_sampler_updates_every_portfolio_pair_from_one_verified_atom() {
        let mut process = campaign();
        let accepted = process.observe(receipt(0, 51, false, 0, 200)).unwrap();

        assert_eq!(accepted.verified_draw.selected_atom_id, ATOM_B);
        assert!(accepted.verified_draw.defensive_component_selected);
        assert_eq!(accepted.portfolio_outcome.atom_id, ATOM_B);
        assert_eq!(accepted.pair_observations.len(), 3);
        assert_eq!(
            accepted.pair_observations[0].consequence_difference_ticks,
            10
        );
        assert!((accepted.pair_observations[0].defensive_likelihood_ratio - 1.0).abs() < 1.0e-15);
        assert!((accepted.pair_observations[0].defensive_contribution - 1.0).abs() < 1.0e-15);
        assert!(
            accepted
                .pair_observations
                .iter()
                .all(|pair| (0.0..=1.0).contains(&pair.defensive_contribution))
        );
        assert_eq!(process.draw_count(), 1);

        let report = process.report().unwrap();
        assert_eq!(report.scenario_id, SCENARIO_ID);
        assert_eq!(report.consequence_unit_id, CONSEQUENCE_UNIT_ID);
        assert_eq!(report.contrast_count, 3);
        assert_eq!(report.draw_count, 1);
    }

    #[test]
    fn adaptive_residual_proposal_is_selected_from_prior_union_event_history() {
        let mut process = campaign();
        let first = process.observe(receipt(0, 51, false, 0, 200)).unwrap();
        assert_eq!(first.verified_draw.complete_proposal_digest, [13; 32]);

        let second = process.observe(receipt(1, 52, true, 200, 200)).unwrap();
        assert_eq!(second.verified_draw.complete_proposal_digest, [14; 32]);
        assert!(!second.verified_draw.defensive_component_selected);
        assert_eq!(
            second.verified_draw.mixture_mass_numerator,
            128_u64 * 128 + 128_u64 * 256
        );
        assert!(
            (second.pair_observations[0].defensive_likelihood_ratio - 1.0 / 3.0).abs() < 1.0e-15
        );
        assert_eq!(process.draw_count(), 2);
    }

    #[test]
    fn event_indicator_weight_is_not_mistaken_for_general_consequence_weight() {
        let mut non_event_config = config();
        non_event_config.atoms[1].is_event = false;
        non_event_config.residual_before_event_ticks = vec![0, 256];
        non_event_config.residual_after_event_ticks = vec![0, 256];
        let non_event_outcomes = outcomes()
            .into_iter()
            .map(|mut atom| {
                if atom.atom_id == ATOM_B {
                    for event in &mut atom.portfolio_events {
                        event.event_occurred = false;
                    }
                }
                atom
            })
            .collect();
        let mut non_event_campaign = FiniteAdaptivePortfolioSelection::new(
            non_event_config,
            0.05,
            SCENARIO_ID,
            DENSITY_SPACE_ID,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            non_event_outcomes,
        )
        .unwrap();

        let accepted = non_event_campaign
            .observe(receipt(0, 53, false, 0, 200))
            .unwrap();
        assert!(!accepted.verified_draw.event_occurred);
        assert_eq!(accepted.verified_draw.normalized_weight_numerator, 0);
        assert!(accepted.pair_observations[0].defensive_likelihood_ratio > 0.0);
        assert!(accepted.pair_observations[0].defensive_contribution > 0.0);
    }

    #[test]
    fn invalid_sampler_receipt_does_not_advance_any_portfolio_sequence() {
        let mut process = campaign();
        let mut invalid = receipt(0, 51, false, 0, 200);
        invalid.complete_proposal_digest = [99; 32];

        assert_eq!(
            process.observe(invalid),
            Err(FiniteAdaptivePortfolioError::SamplerVerificationFailure)
        );
        assert_eq!(process.draw_count(), 0);

        process.observe(receipt(0, 51, false, 0, 200)).unwrap();
        assert_eq!(process.draw_count(), 1);
    }

    #[test]
    fn duplicate_ids_and_out_of_range_outcomes_are_rejected() {
        let mut process = campaign();
        process.observe(receipt(0, 51, false, 0, 200)).unwrap();
        assert_eq!(
            process.observe(receipt(1, 51, true, 200, 200)),
            Err(FiniteAdaptivePortfolioError::SamplerVerificationFailure)
        );
        assert_eq!(process.draw_count(), 1);

        let mut changed = outcomes();
        changed[1].portfolio_consequences[0].consequence_ticks = 11;
        assert_eq!(
            FiniteAdaptivePortfolioSelection::new(
                config(),
                0.05,
                SCENARIO_ID,
                DENSITY_SPACE_ID,
                CONSEQUENCE_UNIT_ID,
                bounds(),
                changed,
            )
            .unwrap_err(),
            FiniteAdaptivePortfolioError::ConsequenceOutsideFrozenBounds
        );
    }

    #[test]
    fn incomplete_duplicate_and_union_event_mismatch_fail_before_sampling() {
        let mut missing = outcomes();
        missing[0].portfolio_consequences.pop();
        assert_eq!(
            FiniteAdaptivePortfolioSelection::new(
                config(),
                0.05,
                SCENARIO_ID,
                DENSITY_SPACE_ID,
                CONSEQUENCE_UNIT_ID,
                bounds(),
                missing,
            )
            .unwrap_err(),
            FiniteAdaptivePortfolioError::IncompletePortfolioOutcomeRow
        );

        let mut union_mismatch = outcomes();
        union_mismatch[1].portfolio_events[0].event_occurred = false;
        assert_eq!(
            FiniteAdaptivePortfolioSelection::new(
                config(),
                0.05,
                SCENARIO_ID,
                DENSITY_SPACE_ID,
                CONSEQUENCE_UNIT_ID,
                bounds(),
                union_mismatch,
            )
            .unwrap_err(),
            FiniteAdaptivePortfolioError::UnionEventDefinitionMismatch
        );
    }

    #[test]
    fn target_only_epsilon_one_is_a_valid_reference_baseline() {
        let mut target_only = config();
        target_only.defensive_mixture_weight_ticks = TICKET_SPACE;
        target_only.residual_before_event_ticks = vec![0, 256];
        target_only.residual_after_event_ticks = vec![256, 0];

        let target_only_receipt = receipt_for_config(&target_only, 0, 54, false, 255, 200);
        let mut process = FiniteAdaptivePortfolioSelection::new(
            target_only,
            0.05,
            SCENARIO_ID,
            DENSITY_SPACE_ID,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            outcomes(),
        )
        .unwrap();
        let accepted = process.observe(target_only_receipt).unwrap();

        assert!(accepted.verified_draw.defensive_component_selected);
        assert_eq!(accepted.verified_draw.mixture_mass_numerator, 128 * 256);
        assert!((accepted.pair_observations[0].defensive_likelihood_ratio - 1.0).abs() < 1.0e-15);
        assert!((accepted.pair_observations[0].defensive_contribution - 1.0).abs() < 1.0e-15);
        assert_eq!(process.report().unwrap().effective_defensive_fraction, 1.0);
    }

    #[test]
    fn campaign_digest_binds_sampler_atom_order() {
        let a = campaign();
        let mut reordered_config = config();
        reordered_config.atoms.reverse();
        reordered_config.target_mass_ticks.reverse();
        reordered_config.residual_before_event_ticks.reverse();
        reordered_config.residual_after_event_ticks.reverse();
        let b = FiniteAdaptivePortfolioSelection::new(
            reordered_config,
            0.05,
            SCENARIO_ID,
            DENSITY_SPACE_ID,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            outcomes(),
        )
        .unwrap();
        assert_ne!(a.campaign_digest, b.campaign_digest);
    }

    #[test]
    fn campaign_digest_is_independent_of_input_outcome_order() {
        let a = campaign();
        let mut reversed = outcomes();
        for atom in &mut reversed {
            atom.portfolio_consequences.reverse();
            atom.portfolio_events.reverse();
        }
        reversed.reverse();
        let b = FiniteAdaptivePortfolioSelection::new(
            config(),
            0.05,
            SCENARIO_ID,
            DENSITY_SPACE_ID,
            CONSEQUENCE_UNIT_ID,
            bounds(),
            reversed,
        )
        .unwrap();
        assert_eq!(a.campaign_digest, b.campaign_digest);
    }
}
