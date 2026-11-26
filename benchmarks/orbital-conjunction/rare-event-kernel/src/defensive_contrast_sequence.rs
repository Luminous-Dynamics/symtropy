// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Anytime-valid confidence sequence for one bounded contrast under a fixed
//! defensive Gaussian proposal mixture.
//!
//! For a frozen target p, q(u) >= epsilon * p(u), and normalized contrast
//! g(u) in [0,1], observe Z = epsilon * g(u) * p(u)/q(u). Under draws from q,
//! E_q[Z] = epsilon * E_p[g] and Z lies in [0,1]. The PrPl-EB interval for
//! E[Z] is divided by the exact effective epsilon and intersected with the
//! known [0,1] support of E_p[g].
//!
//! Version 0.1 binds one immutable proposal mixture and one epistemic scenario.
//! It does not verify sampler execution or physical mode-universe completeness.

use std::collections::BTreeSet;

use crate::portfolio_proposal::{
    PortfolioDefensiveGaussianMixture, PortfolioEventModeKey, PortfolioProposalError,
};
use crate::predictable_empirical_bernstein::{PrPlEbConfidenceSequence, PrPlEbError};

/// Maximum accepted observations in one defensive contrast sequence.
pub const MAX_DEFENSIVE_CONTRAST_DRAWS: u64 = 1_000_000;

/// One caller-supplied draw identity and its direct paired consequence.
#[derive(Debug, Clone, PartialEq)]
pub struct DefensiveContrastDraw {
    /// One-based, consecutive draw index beginning at one.
    pub draw_index: u64,
    /// Stable nonzero identity for this target-state realization.
    ///
    /// Uniqueness is checked locally. It is not an authenticated sampler
    /// receipt and does not prevent relabelling a replay under a new ID.
    pub draw_id: [u8; 32],
    /// Declared portfolio/scenario/event-mode identity for this draw.
    pub mode_key: PortfolioEventModeKey,
    /// Realized state in the fixed proposal's density space.
    pub point: Vec<f64>,
    /// Directly evaluated left-minus-right portfolio consequence in ticks.
    pub consequence_difference_ticks: i64,
}

/// Auditable record of one accepted defensive contrast observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefensiveContrastObservation {
    /// One-based draw index.
    pub draw_index: u64,
    /// Stable caller-supplied draw identity.
    pub draw_id: [u8; 32],
    /// Mode identity used for the evaluation.
    pub mode_key: PortfolioEventModeKey,
    /// Target-measure identity from the frozen proposal.
    pub target_measure_id: [u8; 32],
    /// Density-space identity from the frozen proposal.
    pub density_space_id: [u8; 32],
    /// Frozen defensive configuration digest.
    pub configuration_digest: [u8; 32],
    /// Log target density.
    pub log_target_density: f64,
    /// Log full proposal density.
    pub log_full_proposal_density: f64,
    /// Log target/proposal likelihood ratio.
    pub log_importance_weight: f64,
    /// Normalized direct consequence in [0,1].
    pub normalized_consequence_difference: f64,
    /// Bounded defensive contribution in [0,1].
    pub defensive_contribution: f64,
}

/// Anytime-valid interval for the target expected consequence difference.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefensiveContrastConfidenceInterval {
    /// Stable identity of the left portfolio in the contrast.
    pub left_portfolio_id: [u8; 32],
    /// Stable identity of the right portfolio in the contrast.
    pub right_portfolio_id: [u8; 32],
    /// Number of accepted defensive observations.
    pub draw_count: u64,
    /// Per-contrast error probability, already family-adjusted.
    pub alpha: f64,
    /// Exact effective defensive fraction in the flattened proposal.
    pub effective_defensive_fraction: f64,
    /// Frozen lower consequence bound in ticks.
    pub lower_consequence_bound_ticks: i64,
    /// Frozen upper consequence bound in ticks.
    pub upper_consequence_bound_ticks: i64,
    /// Lower confidence endpoint for target expected left-minus-right consequence.
    pub lower_expected_difference_ticks: f64,
    /// Upper confidence endpoint for target expected left-minus-right consequence.
    pub upper_expected_difference_ticks: f64,
    /// Lower endpoint for normalized target expected contrast.
    pub lower_normalized_expected_difference: f64,
    /// Upper endpoint for normalized target expected contrast.
    pub upper_normalized_expected_difference: f64,
    /// Immutable target-measure identity.
    pub target_measure_id: [u8; 32],
    /// Immutable density-space identity.
    pub density_space_id: [u8; 32],
    /// Immutable proposal configuration digest.
    pub configuration_digest: [u8; 32],
    /// One epistemic scenario represented by this sequence.
    pub scenario_id: [u8; 32],
}

/// Validation, identity, or statistical-update errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefensiveContrastSequenceError {
    /// Per-contrast alpha must lie strictly in (0,1).
    InvalidAlpha,
    /// Portfolio contrast identities are zero, equal, or absent from the mode universe.
    InvalidPortfolioPair,
    /// Frozen lower consequence bound exceeds upper bound.
    InvalidConsequenceBounds,
    /// Draw ID is zero or already used in this sequence.
    InvalidOrDuplicateDrawIdentity,
    /// Draw index is not the next consecutive one-based index.
    InvalidDrawIndex,
    /// Direct consequence difference falls outside frozen bounds.
    ConsequenceOutsideFrozenBounds,
    /// Proposal evaluation rejected the mode, point, or density operation.
    ProposalEvaluationFailure,
    /// Underlying PrPl-EB process rejected an update.
    ConfidenceSequenceFailure,
    /// Interval transformation failed numerically.
    NumericalFailure,
}

impl core::fmt::Display for DefensiveContrastSequenceError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidAlpha => "per-contrast alpha must lie strictly between zero and one",
            Self::InvalidPortfolioPair => "portfolio contrast identity is invalid or absent from the mode universe",
            Self::InvalidConsequenceBounds => "frozen consequence bounds are invalid",
            Self::InvalidOrDuplicateDrawIdentity => "draw identity is zero or duplicated",
            Self::InvalidDrawIndex => "draw index is not the next consecutive sequence index",
            Self::ConsequenceOutsideFrozenBounds => {
                "direct consequence difference is outside its frozen bounds"
            }
            Self::ProposalEvaluationFailure => "defensive proposal evaluation failed",
            Self::ConfidenceSequenceFailure => "underlying PrPl-EB update failed",
            Self::NumericalFailure => "defensive confidence interval transformation failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for DefensiveContrastSequenceError {}

impl From<PortfolioProposalError> for DefensiveContrastSequenceError {
    fn from(_: PortfolioProposalError) -> Self {
        Self::ProposalEvaluationFailure
    }
}

impl From<PrPlEbError> for DefensiveContrastSequenceError {
    fn from(_: PrPlEbError) -> Self {
        Self::ConfidenceSequenceFailure
    }
}

/// One contrast's confidence sequence under a fixed, single-scenario defensive
/// proposal. The proposal is owned and cannot be changed mid-sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct DefensiveContrastConfidenceSequence {
    proposal: PortfolioDefensiveGaussianMixture,
    left_portfolio_id: [u8; 32],
    right_portfolio_id: [u8; 32],
    lower_consequence_bound_ticks: i64,
    upper_consequence_bound_ticks: i64,
    alpha: f64,
    sequence: Option<PrPlEbConfidenceSequence>,
    draw_count: u64,
    seen_draw_ids: BTreeSet<[u8; 32]>,
}

impl DefensiveContrastConfidenceSequence {
    /// Construct a confidence sequence for one frozen pairwise consequence range.
    ///
    /// Alpha must already be allocated across the simultaneous contrast family.
    /// Consequence bounds are frozen before the first observation.
    pub fn new(
        proposal: PortfolioDefensiveGaussianMixture,
        left_portfolio_id: [u8; 32],
        right_portfolio_id: [u8; 32],
        alpha: f64,
        lower_consequence_bound_ticks: i64,
        upper_consequence_bound_ticks: i64,
    ) -> Result<Self, DefensiveContrastSequenceError> {
        if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
            return Err(DefensiveContrastSequenceError::InvalidAlpha);
        }
        if left_portfolio_id.iter().all(|byte| *byte == 0)
            || right_portfolio_id.iter().all(|byte| *byte == 0)
            || left_portfolio_id == right_portfolio_id
            || !proposal
                .expected_modes()
                .iter()
                .any(|mode| mode.portfolio_id == left_portfolio_id)
            || !proposal
                .expected_modes()
                .iter()
                .any(|mode| mode.portfolio_id == right_portfolio_id)
        {
            return Err(DefensiveContrastSequenceError::InvalidPortfolioPair);
        }
        if lower_consequence_bound_ticks > upper_consequence_bound_ticks {
            return Err(DefensiveContrastSequenceError::InvalidConsequenceBounds);
        }
        let sequence = if lower_consequence_bound_ticks == upper_consequence_bound_ticks {
            None
        } else {
            Some(PrPlEbConfidenceSequence::new(alpha)?)
        };

        Ok(Self {
            proposal,
            left_portfolio_id,
            right_portfolio_id,
            lower_consequence_bound_ticks,
            upper_consequence_bound_ticks,
            alpha,
            sequence,
            draw_count: 0,
            seen_draw_ids: BTreeSet::new(),
        })
    }

    /// Add one direct paired consequence draw using the frozen full proposal density.
    ///
    /// The proposal and target identities are fixed at construction. The
    /// confidence update is transactional: draw identity and counter are
    /// committed only after density evaluation and PrPl-EB update succeed.
    pub fn observe(
        &mut self,
        draw: &DefensiveContrastDraw,
    ) -> Result<DefensiveContrastObservation, DefensiveContrastSequenceError> {
        let expected_index = self
            .draw_count
            .checked_add(1)
            .ok_or(DefensiveContrastSequenceError::InvalidDrawIndex)?;
        if draw.draw_index != expected_index || draw.draw_index > MAX_DEFENSIVE_CONTRAST_DRAWS {
            return Err(DefensiveContrastSequenceError::InvalidDrawIndex);
        }
        if draw.draw_id.iter().all(|byte| *byte == 0)
            || self.seen_draw_ids.contains(&draw.draw_id)
        {
            return Err(DefensiveContrastSequenceError::InvalidOrDuplicateDrawIdentity);
        }
        if draw.consequence_difference_ticks < self.lower_consequence_bound_ticks
            || draw.consequence_difference_ticks > self.upper_consequence_bound_ticks
        {
            return Err(DefensiveContrastSequenceError::ConsequenceOutsideFrozenBounds);
        }

        // The difference of two i64 bounds can exceed the i64 range.
        // Compute both offset and width in i128 before conversion to f64.
        let width = i128::from(self.upper_consequence_bound_ticks)
            - i128::from(self.lower_consequence_bound_ticks);
        let offset = i128::from(draw.consequence_difference_ticks)
            - i128::from(self.lower_consequence_bound_ticks);
        let normalized_consequence_difference = if width == 0 {
            0.0
        } else {
            offset as f64 / width as f64
        };
        if !normalized_consequence_difference.is_finite()
            || !(0.0..=1.0).contains(&normalized_consequence_difference)
        {
            return Err(DefensiveContrastSequenceError::NumericalFailure);
        }

        let evaluation = self.proposal.evaluate_draw(
            draw.mode_key,
            &draw.point,
            normalized_consequence_difference,
        )?;

        let mut next_sequence = self.sequence.clone();
        if let Some(sequence) = next_sequence.as_mut() {
            sequence.observe(evaluation.defensive_contribution)?;
        }

        self.sequence = next_sequence;
        self.draw_count = expected_index;
        self.seen_draw_ids.insert(draw.draw_id);

        Ok(DefensiveContrastObservation {
            draw_index: draw.draw_index,
            draw_id: draw.draw_id,
            mode_key: draw.mode_key,
            target_measure_id: evaluation.target_measure_id,
            density_space_id: evaluation.density_space_id,
            configuration_digest: evaluation.configuration_digest,
            log_target_density: evaluation.log_target_density,
            log_full_proposal_density: evaluation.log_full_proposal_density,
            log_importance_weight: evaluation.log_importance_weight,
            normalized_consequence_difference,
            defensive_contribution: evaluation.defensive_contribution,
        })
    }

    /// Number of complete accepted draws.
    pub fn draw_count(&self) -> u64 {
        self.draw_count
    }

    /// Return the transformed target-mean confidence interval.
    ///
    /// The PrPl-EB interval is divided by epsilon, intersected with the known
    /// [0,1] target support, and mapped to the frozen consequence bounds.
    /// Endpoints are expanded outward after each floating-point transformation.
    pub fn report(
        &self,
    ) -> Result<DefensiveContrastConfidenceInterval, DefensiveContrastSequenceError> {
        let epsilon = self.proposal.defensive_fraction();
        let (normalized_lower, normalized_upper) = if let Some(sequence) = &self.sequence {
            let current = sequence.confidence_sequence();
            let lower = next_down(current.lower / epsilon).clamp(0.0, 1.0);
            let upper = next_up(current.upper / epsilon).clamp(0.0, 1.0);
            if !lower.is_finite() || !upper.is_finite() || lower > upper {
                return Err(DefensiveContrastSequenceError::NumericalFailure);
            }
            (lower, upper)
        } else {
            // A zero-width consequence range maps to the constant normalized
            // contrast zero; no stochastic interval or proposal draw is needed.
            (0.0, 0.0)
        };

        let width = (i128::from(self.upper_consequence_bound_ticks)
            - i128::from(self.lower_consequence_bound_ticks)) as f64;
        let lower_ticks = if width == 0.0 {
            self.lower_consequence_bound_ticks as f64
        } else {
            next_down(self.lower_consequence_bound_ticks as f64 + width * normalized_lower)
                .max(self.lower_consequence_bound_ticks as f64)
        };
        let upper_ticks = if width == 0.0 {
            self.upper_consequence_bound_ticks as f64
        } else {
            next_up(self.lower_consequence_bound_ticks as f64 + width * normalized_upper)
                .min(self.upper_consequence_bound_ticks as f64)
        };
        if !lower_ticks.is_finite() || !upper_ticks.is_finite() || lower_ticks > upper_ticks {
            return Err(DefensiveContrastSequenceError::NumericalFailure);
        }

        Ok(DefensiveContrastConfidenceInterval {
            left_portfolio_id: self.left_portfolio_id,
            right_portfolio_id: self.right_portfolio_id,
            draw_count: self.draw_count,
            alpha: self.alpha,
            effective_defensive_fraction: epsilon,
            lower_consequence_bound_ticks: self.lower_consequence_bound_ticks,
            upper_consequence_bound_ticks: self.upper_consequence_bound_ticks,
            lower_expected_difference_ticks: lower_ticks,
            upper_expected_difference_ticks: upper_ticks,
            lower_normalized_expected_difference: normalized_lower,
            upper_normalized_expected_difference: normalized_upper,
            target_measure_id: self.proposal.target_measure_id(),
            density_space_id: self.proposal.density_space_id(),
            configuration_digest: self.proposal.configuration_digest(),
            scenario_id: self.proposal.scenario_id(),
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
    };

    const TARGET_MEASURE_ID: [u8; 32] = [90; 32];
    const DENSITY_SPACE_ID: [u8; 32] = [91; 32];

    fn gaussian(mean: f64, variance: f64) -> Gaussian {
        Gaussian::new(vec![mean], vec![variance]).unwrap()
    }

    fn key(portfolio: u8, scenario: u8, event: u8) -> PortfolioEventModeKey {
        PortfolioEventModeKey {
            portfolio_id: [portfolio; 32],
            scenario_id: [scenario; 32],
            event_mode_id: [event; 32],
        }
    }

    fn proposal(
        component: u8,
        mode_key: PortfolioEventModeKey,
        allocation_weight: f64,
        mean: f64,
    ) -> DefensiveProposalComponent {
        DefensiveProposalComponent {
            component_id: [component; 32],
            mode_key,
            allocation_weight,
            distribution: gaussian(mean, 1.0),
        }
    }

    fn fixture() -> PortfolioDefensiveGaussianMixture {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(0.5, gaussian(-1.0, 1.0)).unwrap(),
            GaussianComponent::new(0.5, gaussian(1.0, 1.0)).unwrap(),
        ])
        .unwrap();
        PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target,
            0.2,
            vec![key(10, 1, 1), key(20, 1, 2)],
            vec![
                proposal(1, key(10, 1, 1), 0.5, -5.0),
                proposal(2, key(20, 1, 2), 0.5, 5.0),
            ],
        )
        .unwrap()
    }

    fn draw(index: u64, draw_id: u8, difference: i64, mode_key: PortfolioEventModeKey) -> DefensiveContrastDraw {
        DefensiveContrastDraw {
            draw_index: index,
            draw_id: [draw_id; 32],
            mode_key,
            point: vec![0.0],
            consequence_difference_ticks: difference,
        }
    }

    #[test]
    fn first_report_is_full_support_and_bound_to_frozen_proposal() {
        let process =
            DefensiveContrastConfidenceSequence::new(
                fixture(),
                [10; 32],
                [20; 32],
                0.01,
                -10,
                10,
            )
            .unwrap();
        let report = process.report().unwrap();

        assert_eq!(report.left_portfolio_id, [10; 32]);
        assert_eq!(report.right_portfolio_id, [20; 32]);
        assert_eq!(report.draw_count, 0);
        assert_eq!(report.alpha, 0.01);
        assert_eq!(report.lower_expected_difference_ticks, -10.0);
        assert_eq!(report.upper_expected_difference_ticks, 10.0);
        assert!((report.effective_defensive_fraction - 0.2).abs() < 1.0e-15);
        assert_eq!(report.target_measure_id, TARGET_MEASURE_ID);
        assert_eq!(report.density_space_id, DENSITY_SPACE_ID);
        assert_eq!(report.scenario_id, [1; 32]);
    }

    #[test]
    fn observed_contribution_is_bounded_and_receipt_binds_the_full_proposal() {
        let mut process =
            DefensiveContrastConfidenceSequence::new(fixture(), [10; 32], [20; 32], 0.01, -10, 10).unwrap();
        let observation = process.observe(&draw(1, 1, 0, key(10, 1, 1))).unwrap();

        assert_eq!(observation.draw_index, 1);
        assert_eq!(observation.draw_id, [1; 32]);
        assert_eq!(observation.target_measure_id, TARGET_MEASURE_ID);
        assert_eq!(observation.density_space_id, DENSITY_SPACE_ID);
        assert!((0.0..=1.0).contains(&observation.defensive_contribution));
        assert!(observation.log_full_proposal_density.is_finite());
        assert_eq!(process.draw_count(), 1);
    }

    #[test]
    fn out_of_bound_consequence_and_bad_mode_do_not_mutate_the_sequence() {
        let mut process =
            DefensiveContrastConfidenceSequence::new(fixture(), [10; 32], [20; 32], 0.01, -10, 10).unwrap();

        assert_eq!(
            process.observe(&draw(1, 1, 11, key(10, 1, 1))),
            Err(DefensiveContrastSequenceError::ConsequenceOutsideFrozenBounds)
        );
        assert_eq!(
            process.observe(&draw(1, 1, 0, key(30, 1, 3))),
            Err(DefensiveContrastSequenceError::ProposalEvaluationFailure)
        );
        assert_eq!(process.draw_count(), 0);

        process.observe(&draw(1, 1, 0, key(10, 1, 1))).unwrap();
        assert_eq!(process.draw_count(), 1);
    }

    #[test]
    fn duplicate_draw_ids_and_reordered_indices_are_rejected() {
        let mut process =
            DefensiveContrastConfidenceSequence::new(fixture(), [10; 32], [20; 32], 0.01, -10, 10).unwrap();
        process.observe(&draw(1, 7, 0, key(10, 1, 1))).unwrap();

        assert_eq!(
            process.observe(&draw(2, 7, 0, key(10, 1, 1))),
            Err(DefensiveContrastSequenceError::InvalidOrDuplicateDrawIdentity)
        );
        assert_eq!(
            process.observe(&draw(3, 8, 0, key(10, 1, 1))),
            Err(DefensiveContrastSequenceError::InvalidDrawIndex)
        );
        assert_eq!(process.draw_count(), 1);
    }

    #[test]
    fn degenerate_consequence_bounds_are_exact() {
        let mut process =
            DefensiveContrastConfidenceSequence::new(
                fixture(),
                [10; 32],
                [20; 32],
                0.01,
                4,
                4,
            )
            .unwrap();
        let observation = process.observe(&draw(1, 1, 4, key(10, 1, 1))).unwrap();
        assert_eq!(observation.normalized_consequence_difference, 0.0);
        assert_eq!(observation.defensive_contribution, 0.0);

        let report = process.report().unwrap();
        assert_eq!(report.lower_expected_difference_ticks, 4.0);
        assert_eq!(report.upper_expected_difference_ticks, 4.0);
        assert_eq!(report.lower_normalized_expected_difference, 0.0);
        assert_eq!(report.upper_normalized_expected_difference, 0.0);
    }

    #[test]
    fn full_i64_consequence_range_does_not_overflow_normalization() {
        let mut process = DefensiveContrastConfidenceSequence::new(
            fixture(),
            [10; 32],
            [20; 32],
            0.01,
            i64::MIN,
            i64::MAX,
        )
        .unwrap();
        let observation = process
            .observe(&draw(1, 1, 0, key(10, 1, 1)))
            .unwrap();
        assert!((observation.normalized_consequence_difference - 0.5).abs() < 1.0e-15);

        let report = process.report().unwrap();
        assert!(report.lower_expected_difference_ticks >= i64::MIN as f64);
        assert!(report.upper_expected_difference_ticks <= i64::MAX as f64);
    }

    #[test]
    fn contrast_is_bound_to_two_distinct_portfolio_identities() {
        assert_eq!(
            DefensiveContrastConfidenceSequence::new(
                fixture(),
                [10; 32],
                [10; 32],
                0.01,
                -10,
                10,
            )
            .unwrap_err(),
            DefensiveContrastSequenceError::InvalidPortfolioPair
        );
        assert_eq!(
            DefensiveContrastConfidenceSequence::new(
                fixture(),
                [10; 32],
                [30; 32],
                0.01,
                -10,
                10,
            )
            .unwrap_err(),
            DefensiveContrastSequenceError::InvalidPortfolioPair
        );
    }

    #[test]
    fn invalid_bounds_and_alpha_fail_before_any_draw() {
        assert_eq!(
            DefensiveContrastConfidenceSequence::new(
                fixture(),
                [10; 32],
                [20; 32],
                0.0,
                -1,
                1,
            )
            .unwrap_err(),
            DefensiveContrastSequenceError::InvalidAlpha
        );
        assert_eq!(
            DefensiveContrastConfidenceSequence::new(
                fixture(),
                [10; 32],
                [20; 32],
                0.01,
                2,
                1,
            )
            .unwrap_err(),
            DefensiveContrastSequenceError::InvalidConsequenceBounds
        );
    }

    #[test]
    fn transformed_interval_stays_inside_the_declared_target_support() {
        let mut process =
            DefensiveContrastConfidenceSequence::new(fixture(), [10; 32], [20; 32], 0.01, -10, 10).unwrap();
        for index in 1..=250 {
            process
                .observe(&draw(index, (index % 250 + 1) as u8, 10, key(10, 1, 1)))
                .unwrap();
        }
        let report = process.report().unwrap();
        assert!(report.lower_normalized_expected_difference >= 0.0);
        assert!(report.upper_normalized_expected_difference <= 1.0);
        assert!(
            report.lower_normalized_expected_difference
                <= report.upper_normalized_expected_difference
        );
        assert!(report.lower_expected_difference_ticks >= -10.0);
        assert!(report.upper_expected_difference_ticks <= 10.0);
    }
}
