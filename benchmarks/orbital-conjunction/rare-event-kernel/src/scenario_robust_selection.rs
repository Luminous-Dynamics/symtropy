// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Scenario-robust portfolio selection without epistemic scenario averaging.
//!
//! Each scenario owns one frozen target/proposal mixture and a simultaneous
//! defensive portfolio manager. Family alpha is split across scenarios, then
//! across all portfolio pairs in each scenario. A portfolio robustly dominates
//! another only when the pairwise evidence favours it in every scenario.
//!
//! This is a conservative partial-order decision. It does not assign scenario
//! probabilities or infer that the frozen scenario universe is complete.

use std::collections::{BTreeMap, BTreeSet};

use crate::paired_counterfactual::MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS;
use crate::portfolio_proposal::{PortfolioDefensiveGaussianMixture, PortfolioEventModeKey};
use crate::portfolio_selection::PortfolioConsequenceBound;
use crate::simultaneous_defensive_selection::{
    DefensivePortfolioDraw, DefensivePortfolioDrawReceipt, SimultaneousDefensivePortfolioReport,
    SimultaneousDefensivePortfolioSelection, SimultaneousDefensiveSelectionError,
};

/// Maximum number of explicit epistemic scenarios in one robust campaign.
pub const MAX_ROBUST_EPISTEMIC_SCENARIOS: usize = 64;

/// Maximum number of cross-scenario batches in one robust campaign.
pub const MAX_ROBUST_SCENARIO_BATCHES: u64 = 1_000_000;

/// One complete batch containing one state draw for every frozen scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioDefensiveDrawBatch {
    /// One-based, consecutive batch index beginning at one.
    pub batch_index: u64,
    /// Stable nonzero identity for this complete multi-scenario batch.
    pub batch_id: [u8; 32],
    /// One complete shared-portfolio draw for every scenario exactly once.
    pub scenario_draws: Vec<DefensivePortfolioDraw>,
}

/// Auditable receipt for one accepted scenario batch.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioDefensiveBatchReceipt {
    /// One-based batch index.
    pub batch_index: u64,
    /// Unique caller-supplied batch identity.
    pub batch_id: [u8; 32],
    /// One scenario-specific draw receipt for every frozen scenario.
    pub scenario_receipts: Vec<DefensivePortfolioDrawReceipt>,
}

/// Confidence evidence for one epistemic scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioPortfolioSnapshot {
    /// Stable epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Scenario-specific simultaneous portfolio report.
    pub report: SimultaneousDefensivePortfolioReport,
}

/// Robust portfolio decision across a finite, explicitly declared scenario set.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioRobustPortfolioReport {
    /// Family-wise error budget across every scenario/pair interval.
    pub family_alpha: f64,
    /// Number of separately evaluated epistemic scenarios.
    pub scenario_count: usize,
    /// Number of frozen portfolio arms.
    pub portfolio_count: usize,
    /// Pairwise comparisons per scenario.
    pub pairwise_contrasts_per_scenario: usize,
    /// Total simultaneous scenario/pair intervals.
    pub total_scenario_pair_intervals: usize,
    /// Error budget allocated to each scenario family.
    pub per_scenario_alpha: f64,
    /// Number of complete multi-scenario batches accepted.
    pub batch_count: u64,
    /// Per-scenario reports, canonically sorted by scenario identity.
    pub scenario_reports: Vec<ScenarioPortfolioSnapshot>,
    /// Present only when one portfolio robustly beats every competitor in
    /// every declared scenario.
    pub unique_robust_best_portfolio_id: Option<[u8; 32]>,
    /// Portfolios not robustly dominated by another portfolio in every scenario.
    pub robustly_undominated_portfolio_ids: Vec<[u8; 32]>,
}

/// Validation errors for the scenario-robust coordinator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioRobustSelectionError {
    /// Family alpha must lie strictly in (0,1).
    InvalidFamilyAlpha,
    /// At least one scenario is required.
    EmptyScenarioSet,
    /// Scenario count exceeds the configured limit.
    TooManyScenarios,
    /// Scenario identity is zero or duplicated.
    InvalidOrDuplicateScenarioIdentity,
    /// Different scenario proposal instances use inconsistent portfolio sets.
    InconsistentPortfolioUniverse,
    /// Portfolio count exceeds the configured limit.
    TooManyPortfolios,
    /// Batch index is skipped, repeated, reordered or out of range.
    InvalidBatchIndex,
    /// Batch ID is zero or already seen.
    InvalidOrDuplicateBatchIdentity,
    /// Batch does not contain every frozen scenario exactly once.
    IncompleteScenarioBatch,
    /// A draw's scenario identity is duplicated or does not match the scenario entry.
    InvalidScenarioDrawIdentity,
    /// One scenario manager rejected its draw.
    ScenarioUpdateFailure,
    /// Scenario comparison or robust-dominance report failed numerically.
    NumericalFailure,
}

impl core::fmt::Display for ScenarioRobustSelectionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidFamilyAlpha => {
                "robust family alpha must lie strictly between zero and one"
            }
            Self::EmptyScenarioSet => "at least one epistemic scenario is required",
            Self::TooManyScenarios => "epistemic scenario set exceeds its configured limit",
            Self::InvalidOrDuplicateScenarioIdentity => "scenario identity is zero or duplicated",
            Self::InconsistentPortfolioUniverse => {
                "scenario entries use inconsistent portfolio universes"
            }
            Self::TooManyPortfolios => "portfolio universe exceeds the bounded comparison limit",
            Self::InvalidBatchIndex => "scenario batch index is not the next consecutive index",
            Self::InvalidOrDuplicateBatchIdentity => {
                "scenario batch identity is zero or duplicated"
            }
            Self::IncompleteScenarioBatch => {
                "scenario batch does not contain every frozen scenario"
            }
            Self::InvalidScenarioDrawIdentity => {
                "scenario draw identity is duplicated or mismatched"
            }
            Self::ScenarioUpdateFailure => "one scenario portfolio update failed",
            Self::NumericalFailure => "robust scenario comparison failed numerically",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ScenarioRobustSelectionError {}

impl From<SimultaneousDefensiveSelectionError> for ScenarioRobustSelectionError {
    fn from(_: SimultaneousDefensiveSelectionError) -> Self {
        Self::ScenarioUpdateFailure
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ScenarioEntry {
    scenario_id: [u8; 32],
    manager: SimultaneousDefensivePortfolioSelection,
}

/// Coordinates fixed-scenario defensive portfolio managers with a family-wise
/// error budget across every scenario and pair.
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioRobustPortfolioSelection {
    family_alpha: f64,
    per_scenario_alpha: f64,
    portfolio_bounds: Vec<PortfolioConsequenceBound>,
    entries: Vec<ScenarioEntry>,
    batch_count: u64,
    seen_batch_ids: BTreeSet<[u8; 32]>,
}

impl ScenarioRobustPortfolioSelection {
    /// Construct one simultaneous portfolio manager per frozen scenario.
    ///
    /// All managers receive alpha divided by scenario_count. Each manager then
    /// divides that amount by K(K-1)/2 pairs, so total family alpha is bounded
    /// by the requested alpha across all scenario/pair confidence sequences.
    /// Scenarios are kept separate; no scenario weights or averaging occur.
    pub fn new(
        family_alpha: f64,
        mut portfolio_bounds: Vec<PortfolioConsequenceBound>,
        mut proposals: Vec<PortfolioDefensiveGaussianMixture>,
    ) -> Result<Self, ScenarioRobustSelectionError> {
        if !family_alpha.is_finite() || family_alpha <= 0.0 || family_alpha >= 1.0 {
            return Err(ScenarioRobustSelectionError::InvalidFamilyAlpha);
        }
        if proposals.is_empty() {
            return Err(ScenarioRobustSelectionError::EmptyScenarioSet);
        }
        if proposals.len() > MAX_ROBUST_EPISTEMIC_SCENARIOS {
            return Err(ScenarioRobustSelectionError::TooManyScenarios);
        }
        if portfolio_bounds.len() > MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS {
            return Err(ScenarioRobustSelectionError::TooManyPortfolios);
        }

        portfolio_bounds.sort_unstable_by_key(|bound| bound.portfolio_id);
        if portfolio_bounds.len() < 2 {
            return Err(ScenarioRobustSelectionError::InconsistentPortfolioUniverse);
        }

        proposals.sort_unstable_by_key(|proposal| proposal.scenario_id());
        for (index, proposal) in proposals.iter().enumerate() {
            let scenario_id = proposal.scenario_id();
            if scenario_id.iter().all(|byte| *byte == 0)
                || (index > 0 && proposals[index - 1].scenario_id() == scenario_id)
            {
                return Err(ScenarioRobustSelectionError::InvalidOrDuplicateScenarioIdentity);
            }
        }

        let mut portfolio_ids = BTreeSet::new();
        for bound in &portfolio_bounds {
            if bound.portfolio_id.iter().all(|byte| *byte == 0)
                || !portfolio_ids.insert(bound.portfolio_id)
                || bound.lower_ticks > bound.upper_ticks
            {
                return Err(ScenarioRobustSelectionError::InconsistentPortfolioUniverse);
            }
        }
        for pair in portfolio_bounds.windows(2) {
            if pair[0].portfolio_id == pair[1].portfolio_id {
                return Err(ScenarioRobustSelectionError::InconsistentPortfolioUniverse);
            }
        }

        let per_scenario_alpha = family_alpha / proposals.len() as f64;
        if !per_scenario_alpha.is_finite() || per_scenario_alpha <= 0.0 || per_scenario_alpha >= 1.0
        {
            return Err(ScenarioRobustSelectionError::NumericalFailure);
        }

        let mut entries = Vec::with_capacity(proposals.len());
        for proposal in proposals {
            let scenario_id = proposal.scenario_id();
            let manager = SimultaneousDefensivePortfolioSelection::new(
                per_scenario_alpha,
                portfolio_bounds.clone(),
                proposal,
            )?;
            entries.push(ScenarioEntry {
                scenario_id,
                manager,
            });
        }

        Ok(Self {
            family_alpha,
            per_scenario_alpha,
            portfolio_bounds,
            entries,
            batch_count: 0,
            seen_batch_ids: BTreeSet::new(),
        })
    }

    /// Add one complete batch and update all scenario managers atomically.
    ///
    /// Every scenario draw uses the same batch index but retains its own target
    /// distribution, density space, proposal configuration and random draw ID.
    /// No manager is committed unless every scenario update succeeds.
    pub fn observe(
        &mut self,
        batch: &ScenarioDefensiveDrawBatch,
    ) -> Result<ScenarioDefensiveBatchReceipt, ScenarioRobustSelectionError> {
        let expected_index = self
            .batch_count
            .checked_add(1)
            .ok_or(ScenarioRobustSelectionError::InvalidBatchIndex)?;
        if batch.batch_index != expected_index || batch.batch_index > MAX_ROBUST_SCENARIO_BATCHES {
        {
            return Err(ScenarioRobustSelectionError::InvalidBatchIndex);
        }
        if batch.batch_id.iter().all(|byte| *byte == 0)
            || self.seen_batch_ids.contains(&batch.batch_id)
        {
            return Err(ScenarioRobustSelectionError::InvalidOrDuplicateBatchIdentity);
        }
        if batch.scenario_draws.len() != self.entries.len() {
            return Err(ScenarioRobustSelectionError::IncompleteScenarioBatch);
        }

        let mut draws_by_scenario = BTreeMap::new();
        for draw in &batch.scenario_draws {
            let scenario_id = draw.mode_key.scenario_id;
            if draws_by_scenario.insert(scenario_id, draw).is_some() {
                return Err(ScenarioRobustSelectionError::InvalidScenarioDrawIdentity);
            }
        }
        if draws_by_scenario.len() != self.entries.len()
            || self
                .entries
                .iter()
                .any(|entry| !draws_by_scenario.contains_key(&entry.scenario_id))
        {
            return Err(ScenarioRobustSelectionError::IncompleteScenarioBatch);
        }

        let mut next_entries = self.entries.clone();
        let mut scenario_receipts = Vec::with_capacity(self.entries.len());
        for entry in &mut next_entries {
            let draw = draws_by_scenario
                .get(&entry.scenario_id)
                .ok_or(ScenarioRobustSelectionError::IncompleteScenarioBatch)?;
            if draw.draw_index != batch.batch_index
                || draw.mode_key.scenario_id != entry.scenario_id
            {
                return Err(ScenarioRobustSelectionError::InvalidScenarioDrawIdentity);
            }
            let receipt = entry.manager.observe(draw)?;
            scenario_receipts.push(receipt);
        }

        self.entries = next_entries;
        self.batch_count = expected_index;
        self.seen_batch_ids.insert(batch.batch_id);

        Ok(ScenarioDefensiveBatchReceipt {
            batch_index: batch.batch_index,
            batch_id: batch.batch_id,
            scenario_receipts,
        })
    }

    /// Number of complete multi-scenario batches accepted.
    pub fn batch_count(&self) -> u64 {
        self.batch_count
    }

    /// Report per-scenario evidence and robust dominance without scenario averaging.
    pub fn report(&self) -> Result<ScenarioRobustPortfolioReport, ScenarioRobustSelectionError> {
        let scenario_reports = self
            .entries
            .iter()
            .map(|entry| {
                let report = entry
                    .manager
                    .report()
                    .map_err(|_| ScenarioRobustSelectionError::NumericalFailure)?;
                Ok(ScenarioPortfolioSnapshot {
                    scenario_id: entry.scenario_id,
                    report,
                })
            })
            .collect::<Result<Vec<_>, ScenarioRobustSelectionError>>()?;

        let first_report = scenario_reports
            .first()
            .ok_or(ScenarioRobustSelectionError::EmptyScenarioSet)?;
        let pairwise_contrasts_per_scenario = first_report.report.contrast_count;
        let total_scenario_pair_intervals = pairwise_contrasts_per_scenario
            .checked_mul(scenario_reports.len())
            .ok_or(ScenarioRobustSelectionError::NumericalFailure)?;

        let mut robustly_best = Vec::new();
        let mut robustly_undominated = Vec::new();
        for candidate in &self.portfolio_bounds {
            let mut beats_every_other_in_every_scenario = self.batch_count > 0;
            let mut robustly_dominated = false;
            for opponent in &self.portfolio_bounds {
                if candidate.portfolio_id == opponent.portfolio_id {
                    continue;
                }

                let (left_id, right_id) = if candidate.portfolio_id < opponent.portfolio_id {
                    (candidate.portfolio_id, opponent.portfolio_id)
                } else {
                    (opponent.portfolio_id, candidate.portfolio_id)
                };
                let mut candidate_beats_opponent_in_every_scenario = true;
                let mut opponent_beats_candidate_in_every_scenario = true;

                for snapshot in &scenario_reports {
                    let interval = snapshot
                        .report
                        .pairwise_intervals
                        .iter()
                        .find(|interval| {
                            interval.left_portfolio_id == left_id
                                && interval.right_portfolio_id == right_id
                        })
                        .ok_or(ScenarioRobustSelectionError::NumericalFailure)?;
                    let candidate_is_left = candidate.portfolio_id == left_id;
                    let candidate_proven_better = if candidate_is_left {
                        interval.upper_expected_difference_ticks < 0.0
                    } else {
                        interval.lower_expected_difference_ticks > 0.0
                    };
                    let opponent_proven_better = if candidate_is_left {
                        interval.lower_expected_difference_ticks > 0.0
                    } else {
                        interval.upper_expected_difference_ticks < 0.0
                    };
                    candidate_beats_opponent_in_every_scenario &= candidate_proven_better;
                    opponent_beats_candidate_in_every_scenario &= opponent_proven_better;
                }

                beats_every_other_in_every_scenario &= candidate_beats_opponent_in_every_scenario;
                robustly_dominated |= opponent_beats_candidate_in_every_scenario;
            }

            if beats_every_other_in_every_scenario {
                robustly_best.push(candidate.portfolio_id);
            }
            if !robustly_dominated {
                robustly_undominated.push(candidate.portfolio_id);
            }
        }

        Ok(ScenarioRobustPortfolioReport {
            family_alpha: self.family_alpha,
            scenario_count: self.entries.len(),
            portfolio_count: self.portfolio_bounds.len(),
            pairwise_contrasts_per_scenario,
            total_scenario_pair_intervals,
            per_scenario_alpha: self.per_scenario_alpha,
            batch_count: self.batch_count,
            scenario_reports,
            unique_robust_best_portfolio_id: if robustly_best.len() == 1 {
                Some(robustly_best[0])
            } else {
                None
            },
            robustly_undominated_portfolio_ids: robustly_undominated,
        })
    }
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

    fn gaussian(mean: f64, variance: f64) -> Gaussian {
        Gaussian::new(vec![mean], vec![variance]).unwrap()
    }

    fn key(portfolio: u8, scenario: u8) -> PortfolioEventModeKey {
        PortfolioEventModeKey {
            portfolio_id: [portfolio; 32],
            scenario_id: [scenario; 32],
            event_mode_id: [portfolio; 32],
        }
    }

    fn component(component_id: u8, portfolio: u8, scenario: u8) -> DefensiveProposalComponent {
        DefensiveProposalComponent {
            component_id: [component_id; 32],
            mode_key: key(portfolio, scenario),
            allocation_weight: 1.0 / 3.0,
            distribution: gaussian(0.0, 1.0),
        }
    }

    fn proposal_for_scenario(scenario: u8) -> PortfolioDefensiveGaussianMixture {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target,
            0.2,
            vec![key(10, scenario), key(20, scenario), key(30, scenario)],
            vec![
                component(1, 10, scenario),
                component(2, 20, scenario),
                component(3, 30, scenario),
            ],
        )
        .unwrap()
    }

    fn bounds() -> Vec<PortfolioConsequenceBound> {
        vec![
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
            PortfolioConsequenceBound {
                portfolio_id: [30; 32],
                lower_ticks: 0,
                upper_ticks: 20,
            },
        ]
    }

    fn scenario_draw(
        batch_index: u64,
        draw_id: u64,
        scenario: u8,
        outcomes: &[(u8, i32)],
    ) -> DefensivePortfolioDraw {
        let mut id = [0_u8; 32];
        id[..8].copy_from_slice(&draw_id.to_be_bytes());
        DefensivePortfolioDraw {
            draw_index: batch_index,
            draw_id: id,
            mode_key: key(10, scenario),
            point: vec![0.0],
            portfolio_consequences: outcomes
                .iter()
                .map(
                    |(portfolio, consequence_ticks)| PortfolioConsequenceObservation {
                        portfolio_id: [*portfolio; 32],
                        consequence_ticks: *consequence_ticks,
                    },
                )
                .collect(),
        }
    }

    fn batch(
        index: u64,
        batch_id: u64,
        outcomes_a: &[(u8, i32)],
        outcomes_b: &[(u8, i32)],
    ) -> ScenarioDefensiveDrawBatch {
        let mut id = [0_u8; 32];
        id[..8].copy_from_slice(&batch_id.to_be_bytes());
        ScenarioDefensiveDrawBatch {
            batch_index: index,
            batch_id: id,
            scenario_draws: vec![
                scenario_draw(index, batch_id * 2, 1, outcomes_a),
                scenario_draw(index, batch_id * 2 + 1, 2, outcomes_b),
            ],
        }
    }

    fn coordinator() -> ScenarioRobustPortfolioSelection {
        ScenarioRobustPortfolioSelection::new(
            0.05,
            bounds(),
            vec![proposal_for_scenario(2), proposal_for_scenario(1)],
        )
        .unwrap()
    }

    #[test]
    fn family_alpha_is_allocated_across_scenarios_and_pairs() {
        let process = coordinator();
        let report = process.report().unwrap();

        assert_eq!(report.scenario_count, 2);
        assert_eq!(report.portfolio_count, 3);
        assert_eq!(report.pairwise_contrasts_per_scenario, 3);
        assert_eq!(report.total_scenario_pair_intervals, 6);
        assert_eq!(report.per_scenario_alpha, 0.025);
        assert_eq!(report.scenario_reports[0].scenario_id, [1; 32]);
        assert_eq!(report.scenario_reports[1].scenario_id, [2; 32]);
        assert_eq!(report.unique_robust_best_portfolio_id, None);
        assert_eq!(
            report.robustly_undominated_portfolio_ids,
            vec![[10; 32], [20; 32], [30; 32]]
        );
    }

    #[test]
    fn same_best_in_every_scenario_can_be_robustly_certified() {
        let mut process = coordinator();
        for index in 1..=2_000 {
            process
                .observe(&batch(
                    index,
                    index,
                    &[(10, 0), (20, 10), (30, 20)],
                    &[(10, 0), (20, 10), (30, 20)],
                ))
                .unwrap();
            let report = process.report().unwrap();
            if report.unique_robust_best_portfolio_id.is_some() {
                assert_eq!(report.unique_robust_best_portfolio_id, Some([10; 32]));
                assert_eq!(report.robustly_undominated_portfolio_ids, vec![[10; 32]]);
                assert!(report.scenario_reports.iter().all(|snapshot| {
                    snapshot.report.unique_best_portfolio_id == Some([10; 32])
                }));
                return;
            }
        }
        panic!("same best portfolio was not robustly certified within 2,000 batches");
    }

    #[test]
    fn conflicting_scenario_winners_remain_separate_and_no_robust_winner_is_claimed() {
        let mut process = coordinator();
        for index in 1..=2_000 {
            process
                .observe(&batch(
                    index,
                    index,
                    &[(10, 0), (20, 10), (30, 20)],
                    &[(10, 20), (20, 10), (30, 0)],
                ))
                .unwrap();
        }

        let report = process.report().unwrap();
        assert_eq!(
            report.scenario_reports[0].report.unique_best_portfolio_id,
            Some([10; 32])
        );
        assert_eq!(
            report.scenario_reports[1].report.unique_best_portfolio_id,
            Some([30; 32])
        );
        assert_eq!(report.unique_robust_best_portfolio_id, None);
        assert_eq!(
            report.robustly_undominated_portfolio_ids,
            vec![[10; 32], [20; 32], [30; 32]]
        );
    }

    #[test]
    fn missing_or_duplicate_scenario_draw_aborts_the_complete_batch() {
        let mut process = coordinator();
        let mut missing = batch(
            1,
            1,
            &[(10, 0), (20, 10), (30, 20)],
            &[(10, 0), (20, 10), (30, 20)],
        );
        missing.scenario_draws.pop();
        assert_eq!(
            process.observe(&missing),
            Err(ScenarioRobustSelectionError::IncompleteScenarioBatch)
        );
        assert_eq!(process.batch_count(), 0);

        let mut duplicate = batch(
            1,
            1,
            &[(10, 0), (20, 10), (30, 20)],
            &[(10, 0), (20, 10), (30, 20)],
        );
        duplicate.scenario_draws[1].mode_key.scenario_id = [1; 32];
        assert_eq!(
            process.observe(&duplicate),
            Err(ScenarioRobustSelectionError::InvalidScenarioDrawIdentity)
        );
        assert_eq!(process.batch_count(), 0);
    }

    #[test]
    fn one_scenario_failure_rolls_back_all_other_scenario_updates() {
        let mut process = coordinator();
        let mut invalid = batch(
            1,
            1,
            &[(10, 0), (20, 10), (30, 20)],
            &[(10, 0), (20, 10), (30, 20)],
        );
        invalid.scenario_draws[1].portfolio_consequences[2].consequence_ticks = 21;

        assert_eq!(
            process.observe(&invalid),
            Err(ScenarioRobustSelectionError::ScenarioUpdateFailure)
        );
        assert_eq!(process.batch_count(), 0);
        assert!(
            process
                .report()
                .unwrap()
                .scenario_reports
                .iter()
                .all(|snapshot| snapshot.report.draw_count == 0)
        );
    }

    #[test]
    fn duplicate_batch_id_and_reordered_batch_are_rejected() {
        let mut process = coordinator();
        process
            .observe(&batch(
                1,
                10,
                &[(10, 0), (20, 10), (30, 20)],
                &[(10, 0), (20, 10), (30, 20)],
            ))
            .unwrap();

        assert_eq!(
            process.observe(&batch(
                2,
                10,
                &[(10, 0), (20, 10), (30, 20)],
                &[(10, 0), (20, 10), (30, 20)],
            )),
            Err(ScenarioRobustSelectionError::InvalidOrDuplicateBatchIdentity)
        );
        assert_eq!(
            process.observe(&batch(
                1,
                11,
                &[(10, 0), (20, 10), (30, 20)],
                &[(10, 0), (20, 10), (30, 20)],
            )),
            Err(ScenarioRobustSelectionError::InvalidBatchIndex)
        );
        assert_eq!(process.batch_count(), 1);
    }

    #[test]
    fn duplicate_scenario_identity_is_rejected_at_construction() {
        assert_eq!(
            ScenarioRobustPortfolioSelection::new(
                0.05,
                bounds(),
                vec![proposal_for_scenario(1), proposal_for_scenario(1)],
            )
            .unwrap_err(),
            ScenarioRobustSelectionError::InvalidOrDuplicateScenarioIdentity
        );
    }
}
