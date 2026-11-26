// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Scenario-separated minimax-regret decisions from simultaneous defensive
//! portfolio confidence reports.
//!
//! Aleatory uncertainty is integrated within one target measure per scenario.
//! Epistemic scenarios remain separate: no scenario probabilities are invented
//! and no physical-world alternatives are silently averaged.
//!
//! For portfolio i, scenario s and competitor j, let
//! Delta(i,j,s) = E_s[L_i - L_j]. The worst-case regret is
//!
//! R_i = max(0, max_s,j Delta(i,j,s)).
//!
//! Given simultaneous intervals [l,u] for every scenario/pair,
//!
//! [max(0,max l), max(0,max u)]
//!
//! is a valid interval for this maximum. A minimax-regret winner is named only
//! when its regret upper endpoint is strictly below every competitor's lower
//! endpoint. This is a robust decision wrapper, not a safety/authority gate.

use std::collections::BTreeMap;

use crate::simultaneous_defensive_selection::SimultaneousDefensivePortfolioReport;

/// One interval for a portfolio's worst-case epistemic-scenario regret.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortfolioWorstCaseRegretInterval {
    /// Stable portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Lower endpoint for max_s,j E_s[L_i - L_j].
    pub lower_worst_case_regret_ticks: f64,
    /// Upper endpoint for max_s,j E_s[L_i - L_j].
    pub upper_worst_case_regret_ticks: f64,
    /// Scenario attaining the largest lower endpoint (ties break by scenario ID).
    pub lower_witness_scenario_id: [u8; 32],
    /// Scenario attaining the largest upper endpoint (ties break by scenario ID).
    pub upper_witness_scenario_id: [u8; 32],
}

/// Compact, scenario-specific provenance retained by the robust result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EpistemicScenarioEvidence {
    /// Stable scenario identity.
    pub scenario_id: [u8; 32],
    /// Scenario-specific target probability measure.
    pub target_measure_id: [u8; 32],
    /// Density space for the sampled state.
    pub density_space_id: [u8; 32],
    /// Scenario-specific proposal configuration digest.
    pub proposal_configuration_digest: [u8; 32],
    /// Number of accepted draws.
    pub draw_count: u64,
    /// Defensive target fraction.
    pub effective_defensive_fraction: f64,
    /// Family-wise error budget allocated to this scenario.
    pub scenario_family_alpha: f64,
}

/// Complete epistemic minimax-regret report.
#[derive(Debug, Clone, PartialEq)]
pub struct EpistemicMinimaxRegretReport {
    /// Total family-wise confidence error fixed before sampling.
    pub total_family_alpha: f64,
    /// Number of epistemic scenarios in the frozen universe.
    pub scenario_count: usize,
    /// Number of portfolio arms.
    pub portfolio_count: usize,
    /// Number of scenario × pair contrasts.
    pub total_contrast_count: usize,
    /// Per-pair alpha after scenario and pair allocation.
    pub contrast_alpha: f64,
    /// Common consequence unit / sign / tick-scale identity.
    pub consequence_unit_id: [u8; 32],
    /// Scenario-specific source and sampler identities, in canonical order.
    pub scenario_evidence: Vec<EpistemicScenarioEvidence>,
    /// Full worst-case regret intervals in canonical portfolio order.
    pub worst_case_regret_intervals: Vec<PortfolioWorstCaseRegretInterval>,
    /// Unique minimax-regret portfolio only when strict interval separation holds.
    pub unique_minimax_regret_portfolio_id: Option<[u8; 32]>,
    /// Portfolios not proven to have higher worst-case regret than another.
    pub minimax_regret_undominated_portfolio_ids: Vec<[u8; 32]>,
}

/// Errors validating a complete epistemic scenario / portfolio interval matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpistemicMinimaxRegretError {
    /// Total family alpha must lie strictly in (0,1).
    InvalidFamilyAlpha,
    /// Scenario universe is empty, too large or contains invalid identities.
    InvalidScenarioUniverse,
    /// Scenario report is missing, duplicated or unexpected.
    IncompleteOrDuplicateScenarioReport,
    /// Portfolio universe is empty, invalid or too large.
    InvalidPortfolioUniverse,
    /// Portfolio count or pair count in a scenario report is inconsistent.
    IncompletePortfolioPairFamily,
    /// A pair identity is zero, noncanonical, missing, duplicated or unexpected.
    InvalidPortfolioPairIdentity,
    /// The scenario report uses a different family-wise alpha allocation.
    InvalidScenarioAlphaAllocation,
    /// The scenario report uses a different consequence unit or scale.
    ConsequenceUnitMismatch,
    /// A provenance identity is zero or the defensive fraction is invalid.
    InvalidScenarioProvenance,
    /// Pair interval has invalid sample count, alpha, endpoints or numeric values.
    InvalidPairInterval,
    /// Confidence or regret calculations failed numerically.
    NumericalFailure,
}

impl core::fmt::Display for EpistemicMinimaxRegretError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidFamilyAlpha => "total family-wise alpha must lie in (0, 1)",
            Self::InvalidScenarioUniverse => "epistemic scenario universe is invalid",
            Self::IncompleteOrDuplicateScenarioReport => {
                "epistemic scenario report universe is incomplete or duplicated"
            }
            Self::InvalidPortfolioUniverse => "portfolio universe is invalid",
            Self::IncompletePortfolioPairFamily => {
                "scenario report does not contain the complete pairwise portfolio family"
            }
            Self::InvalidPortfolioPairIdentity => {
                "portfolio pair identity is zero, duplicated, noncanonical or unexpected"
            }
            Self::InvalidScenarioAlphaAllocation => {
                "scenario confidence level does not match the frozen family allocation"
            }
            Self::ConsequenceUnitMismatch => {
                "scenario reports do not share one consequence unit and tick scale"
            }
            Self::InvalidScenarioProvenance => "scenario target or proposal provenance is invalid",
            Self::InvalidPairInterval => "scenario pair confidence interval is invalid",
            Self::NumericalFailure => "minimax-regret interval calculation failed numerically",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for EpistemicMinimaxRegretError {}

#[derive(Debug, Clone, PartialEq)]
struct RegretAccumulator {
    portfolio_id: [u8; 32],
    lower: f64,
    upper: f64,
    lower_witness: [u8; 32],
    upper_witness: [u8; 32],
}

/// Frozen epistemic scenario and portfolio universe for a robust minimax-regret decision.
#[derive(Debug, Clone, PartialEq)]
pub struct EpistemicMinimaxRegret {
    total_family_alpha: f64,
    scenario_family_alpha: f64,
    contrast_alpha: f64,
    expected_scenario_ids: Vec<[u8; 32]>,
    portfolio_ids: Vec<[u8; 32]>,
    consequence_unit_id: [u8; 32],
}

impl EpistemicMinimaxRegret {
    /// Freeze the total family alpha, complete scenario universe, portfolio universe
    /// and common consequence-unit identity before inspecting scenario results.
    pub fn new(
        total_family_alpha: f64,
        mut expected_scenario_ids: Vec<[u8; 32]>,
        mut portfolio_ids: Vec<[u8; 32]>,
        consequence_unit_id: [u8; 32],
    ) -> Result<Self, EpistemicMinimaxRegretError> {
        if !total_family_alpha.is_finite()
            || total_family_alpha <= 0.0
            || total_family_alpha >= 1.0
        {
            return Err(EpistemicMinimaxRegretError::InvalidFamilyAlpha);
        }
        if expected_scenario_ids.is_empty()
            || expected_scenario_ids.len() > 1_024
            || expected_scenario_ids.iter().any(|id| id.iter().all(|b| *b == 0))
            || consequence_unit_id.iter().all(|b| *b == 0)
        {
            return Err(EpistemicMinimaxRegretError::InvalidScenarioUniverse);
        }
        expected_scenario_ids.sort_unstable();
        if expected_scenario_ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(EpistemicMinimaxRegretError::InvalidScenarioUniverse);
        }

        if portfolio_ids.len() < 2
            || portfolio_ids.len() > 32
            || portfolio_ids.iter().any(|id| id.iter().all(|b| *b == 0))
        {
            return Err(EpistemicMinimaxRegretError::InvalidPortfolioUniverse);
        }
        portfolio_ids.sort_unstable();
        if portfolio_ids.windows(2).any(|w| w[0] == w[1]) {
            return Err(EpistemicMinimaxRegretError::InvalidPortfolioUniverse);
        }

        let scenario_family_alpha = total_family_alpha / expected_scenario_ids.len() as f64;
        let pair_count = portfolio_ids
            .len()
            .checked_mul(portfolio_ids.len() - 1)
            .and_then(|n| n.checked_div(2))
            .ok_or(EpistemicMinimaxRegretError::NumericalFailure)?;
        let contrast_alpha = scenario_family_alpha / pair_count as f64;
        if !scenario_family_alpha.is_finite()
            || scenario_family_alpha <= 0.0
            || !contrast_alpha.is_finite()
            || contrast_alpha <= 0.0
        {
            return Err(EpistemicMinimaxRegretError::NumericalFailure);
        }

        Ok(Self {
            total_family_alpha,
            scenario_family_alpha,
            contrast_alpha,
            expected_scenario_ids,
            portfolio_ids,
            consequence_unit_id,
        })
    }

    /// Required scenario family alpha for each independently sampled scenario.
    pub fn scenario_family_alpha(&self) -> f64 {
        self.scenario_family_alpha
    }

    /// Required per-pair alpha inside each scenario coordinator.
    pub fn contrast_alpha(&self) -> f64 {
        self.contrast_alpha
    }

    /// Aggregate one complete report per epistemic scenario into worst-case regret intervals.
    ///
    /// Reports may arrive in any order. The exact frozen scenario and pair
    /// universes must be present once each. No scenario weighting or averaging
    /// is performed.
    pub fn aggregate(
        &self,
        reports: &[SimultaneousDefensivePortfolioReport],
    ) -> Result<EpistemicMinimaxRegretReport, EpistemicMinimaxRegretError> {
        if reports.len() != self.expected_scenario_ids.len() {
            return Err(EpistemicMinimaxRegretError::IncompleteOrDuplicateScenarioReport);
        }

        let expected_pair_count = self
            .portfolio_ids
            .len()
            .checked_mul(self.portfolio_ids.len() - 1)
            .and_then(|n| n.checked_div(2))
            .ok_or(EpistemicMinimaxRegretError::NumericalFailure)?;

        let mut by_scenario = BTreeMap::new();
        for report in reports {
            if report.scenario_id.iter().all(|b| *b == 0)
                || by_scenario.insert(report.scenario_id, report).is_some()
            {
                return Err(EpistemicMinimaxRegretError::IncompleteOrDuplicateScenarioReport);
            }
        }
        if self
            .expected_scenario_ids
            .iter()
            .any(|id| !by_scenario.contains_key(id))
        {
            return Err(EpistemicMinimaxRegretError::IncompleteOrDuplicateScenarioReport);
        }

        let mut scenario_evidence = Vec::with_capacity(reports.len());
        let mut accumulators = self
            .portfolio_ids
            .iter()
            .map(|portfolio_id| RegretAccumulator {
                portfolio_id: *portfolio_id,
                lower: 0.0,
                upper: 0.0,
                lower_witness: self.expected_scenario_ids[0],
                upper_witness: self.expected_scenario_ids[0],
            })
            .collect::<Vec<_>>();

        for scenario_id in &self.expected_scenario_ids {
            let report = by_scenario
                .get(scenario_id)
                .copied()
                .ok_or(EpistemicMinimaxRegretError::IncompleteOrDuplicateScenarioReport)?;

            if report.consequence_unit_id != self.consequence_unit_id {
                return Err(EpistemicMinimaxRegretError::ConsequenceUnitMismatch);
            }
            if report.portfolio_count != self.portfolio_ids.len()
                || report.contrast_count != expected_pair_count
                || report.pairwise_intervals.len() != expected_pair_count
            {
                return Err(EpistemicMinimaxRegretError::IncompletePortfolioPairFamily);
            }
            if !close_enough(report.family_alpha, self.scenario_family_alpha)
                || !close_enough(report.contrast_alpha, self.contrast_alpha)
            {
                return Err(EpistemicMinimaxRegretError::InvalidScenarioAlphaAllocation);
            }
            if report.target_measure_id.iter().all(|b| *b == 0)
                || report.density_space_id.iter().all(|b| *b == 0)
                || report.configuration_digest.iter().all(|b| *b == 0)
                || !report.effective_defensive_fraction.is_finite()
                || report.effective_defensive_fraction <= 0.0
                || report.effective_defensive_fraction >= 1.0
            {
                return Err(EpistemicMinimaxRegretError::InvalidScenarioProvenance);
            }

            let mut intervals = BTreeMap::new();
            for interval in &report.pairwise_intervals {
                if interval.left_portfolio_id.iter().all(|b| *b == 0)
                    || interval.right_portfolio_id.iter().all(|b| *b == 0)
                    || interval.left_portfolio_id >= interval.right_portfolio_id
                    || interval.sample_count != report.draw_count
                    || !close_enough(interval.contrast_alpha, self.contrast_alpha)
                    || !interval.lower_expected_difference_ticks.is_finite()
                    || !interval.upper_expected_difference_ticks.is_finite()
                    || !interval.lower_normalized_expected_difference.is_finite()
                    || !interval.upper_normalized_expected_difference.is_finite()
                    || !(0.0..=1.0).contains(&interval.lower_normalized_expected_difference)
                    || !(0.0..=1.0).contains(&interval.upper_normalized_expected_difference)
                    || interval.lower_normalized_expected_difference
                        > interval.upper_normalized_expected_difference
                    || interval.lower_expected_difference_ticks
                        > interval.upper_expected_difference_ticks
                {
                    return Err(EpistemicMinimaxRegretError::InvalidPairInterval);
                }
                let key = (interval.left_portfolio_id, interval.right_portfolio_id);
                if intervals.insert(key, interval).is_some() {
                    return Err(EpistemicMinimaxRegretError::InvalidPortfolioPairIdentity);
                }
            }

            for (i, left_id) in self.portfolio_ids.iter().enumerate() {
                for right_id in self.portfolio_ids.iter().skip(i + 1) {
                    let interval = intervals
                        .get(&(*left_id, *right_id))
                        .copied()
                        .ok_or(EpistemicMinimaxRegretError::InvalidPortfolioPairIdentity)?;
                    for portfolio in &mut accumulators {
                        let (lower, upper) = if portfolio.portfolio_id == *left_id {
                            (
                                interval.lower_expected_difference_ticks,
                                interval.upper_expected_difference_ticks,
                            )
                        } else if portfolio.portfolio_id == *right_id {
                            (
                                -interval.upper_expected_difference_ticks,
                                -interval.lower_expected_difference_ticks,
                            )
                        } else {
                            continue;
                        };
                        if lower > portfolio.lower {
                            portfolio.lower = lower;
                            portfolio.lower_witness = *scenario_id;
                        }
                        if upper > portfolio.upper {
                            portfolio.upper = upper;
                            portfolio.upper_witness = *scenario_id;
                        }
                    }
                }
            }

            scenario_evidence.push(EpistemicScenarioEvidence {
                scenario_id: report.scenario_id,
                target_measure_id: report.target_measure_id,
                density_space_id: report.density_space_id,
                proposal_configuration_digest: report.configuration_digest,
                draw_count: report.draw_count,
                effective_defensive_fraction: report.effective_defensive_fraction,
                scenario_family_alpha: report.family_alpha,
            });
        }

        let worst_case_regret_intervals = accumulators
            .iter()
            .map(|accumulator| {
                if !accumulator.lower.is_finite()
                    || !accumulator.upper.is_finite()
                    || accumulator.lower > accumulator.upper
                {
                    return Err(EpistemicMinimaxRegretError::NumericalFailure);
                }
                Ok(PortfolioWorstCaseRegretInterval {
                    portfolio_id: accumulator.portfolio_id,
                    lower_worst_case_regret_ticks: accumulator.lower.max(0.0),
                    upper_worst_case_regret_ticks: accumulator.upper.max(0.0),
                    lower_witness_scenario_id: accumulator.lower_witness,
                    upper_witness_scenario_id: accumulator.upper_witness,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let mut unique_minimax = Vec::new();
        let mut undominated = Vec::new();
        for candidate in &worst_case_regret_intervals {
            let beats_all = worst_case_regret_intervals.iter().all(|opponent| {
                candidate.portfolio_id == opponent.portfolio_id
                    || candidate.upper_worst_case_regret_ticks
                        < opponent.lower_worst_case_regret_ticks
            });
            let is_dominated = worst_case_regret_intervals.iter().any(|opponent| {
                candidate.portfolio_id != opponent.portfolio_id
                    && opponent.upper_worst_case_regret_ticks
                        < candidate.lower_worst_case_regret_ticks
            });
            if beats_all {
                unique_minimax.push(candidate.portfolio_id);
            }
            if !is_dominated {
                undominated.push(candidate.portfolio_id);
            }
        }

        let total_contrast_count = expected_pair_count
            .checked_mul(self.expected_scenario_ids.len())
            .ok_or(EpistemicMinimaxRegretError::NumericalFailure)?;

        Ok(EpistemicMinimaxRegretReport {
            total_family_alpha: self.total_family_alpha,
            scenario_count: self.expected_scenario_ids.len(),
            portfolio_count: self.portfolio_ids.len(),
            total_contrast_count,
            contrast_alpha: self.contrast_alpha,
            consequence_unit_id: self.consequence_unit_id,
            scenario_evidence,
            worst_case_regret_intervals,
            unique_minimax_regret_portfolio_id: if unique_minimax.len() == 1 {
                Some(unique_minimax[0])
            } else {
                None
            },
            minimax_regret_undominated_portfolio_ids: undominated,
        })
    }
}

fn close_enough(left: f64, right: f64) -> bool {
    left.is_finite()
        && right.is_finite()
        && (left - right).abs() <= 16.0 * f64::EPSILON * (1.0 + left.abs().max(right.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simultaneous_defensive_selection::DefensivePortfolioPairInterval;

    const UNIT_ID: [u8; 32] = [92; 32];
    const SCENARIO_A: [u8; 32] = [1; 32];
    const SCENARIO_B: [u8; 32] = [2; 32];
    const PORTFOLIO_A: [u8; 32] = [10; 32];
    const PORTFOLIO_B: [u8; 32] = [20; 32];
    const PORTFOLIO_C: [u8; 32] = [30; 32];

    fn scenario_report(
        scenario_id: [u8; 32],
        means: [f64; 3],
        family_alpha: f64,
    ) -> SimultaneousDefensivePortfolioReport {
        let pairs = [
            (PORTFOLIO_A, PORTFOLIO_B),
            (PORTFOLIO_A, PORTFOLIO_C),
            (PORTFOLIO_B, PORTFOLIO_C),
        ];
        let alpha = family_alpha / 3.0;
        let intervals = pairs
            .iter()
            .map(|(left, right)| {
                let left_index = if *left == PORTFOLIO_A {
                    0
                } else if *left == PORTFOLIO_B {
                    1
                } else {
                    2
                };
                let right_index = if *right == PORTFOLIO_A {
                    0
                } else if *right == PORTFOLIO_B {
                    1
                } else {
                    2
                };
                let difference = means[left_index] - means[right_index];
                DefensivePortfolioPairInterval {
                    left_portfolio_id: *left,
                    right_portfolio_id: *right,
                    sample_count: 100,
                    contrast_alpha: alpha,
                    lower_expected_difference_ticks: difference,
                    upper_expected_difference_ticks: difference,
                    lower_normalized_expected_difference: 0.5,
                    upper_normalized_expected_difference: 0.5,
                }
            })
            .collect::<Vec<_>>();
        SimultaneousDefensivePortfolioReport {
            consequence_unit_id: UNIT_ID,
            family_alpha,
            portfolio_count: 3,
            contrast_count: 3,
            contrast_alpha: alpha,
            draw_count: 100,
            pairwise_intervals: intervals,
            unique_best_portfolio_id: None,
            statistically_undominated_portfolio_ids: vec![
                PORTFOLIO_A,
                PORTFOLIO_B,
                PORTFOLIO_C,
            ],
            target_measure_id: [80; 32],
            density_space_id: [81; 32],
            scenario_id,
            configuration_digest: if scenario_id == SCENARIO_A {
                [82; 32]
            } else {
                [83; 32]
            },
            effective_defensive_fraction: 0.2,
        }
    }

    fn evaluator() -> EpistemicMinimaxRegret {
        EpistemicMinimaxRegret::new(
            0.05,
            vec![SCENARIO_B, SCENARIO_A],
            vec![PORTFOLIO_C, PORTFOLIO_A, PORTFOLIO_B],
            UNIT_ID,
        )
        .unwrap()
    }

    #[test]
    fn scenario_alpha_is_allocated_over_scenarios_and_all_pairs() {
        let process = evaluator();
        assert_eq!(process.scenario_family_alpha(), 0.025);
        assert!((process.contrast_alpha() - 0.025 / 3.0).abs() < 1.0e-15);
    }

    #[test]
    fn minimax_regret_selects_portfolio_that_is_not_nominally_best_in_every_scenario() {
        // Scenario A prefers A; scenario B prefers B. C is never the best.
        // Worst-case regrets: A=20, B=10, C=20. Minimax regret is B.
        let reports = [
            scenario_report(SCENARIO_A, [0.0, 10.0, 20.0], 0.025),
            scenario_report(SCENARIO_B, [20.0, 0.0, 10.0], 0.025),
        ];
        let report = evaluator().aggregate(&reports).unwrap();

        assert_eq!(report.unique_minimax_regret_portfolio_id, Some(PORTFOLIO_B));
        assert_eq!(
            report.minimax_regret_undominated_portfolio_ids,
            vec![PORTFOLIO_B]
        );
        let regrets = &report.worst_case_regret_intervals;
        assert_eq!(regrets[0].portfolio_id, PORTFOLIO_A);
        assert_eq!(regrets[0].lower_worst_case_regret_ticks, 20.0);
        assert_eq!(regrets[0].upper_worst_case_regret_ticks, 20.0);
        assert_eq!(regrets[1].portfolio_id, PORTFOLIO_B);
        assert_eq!(regrets[1].lower_worst_case_regret_ticks, 10.0);
        assert_eq!(regrets[1].upper_worst_case_regret_ticks, 10.0);
        assert_eq!(regrets[2].portfolio_id, PORTFOLIO_C);
        assert_eq!(regrets[2].lower_worst_case_regret_ticks, 20.0);
        assert_eq!(regrets[2].upper_worst_case_regret_ticks, 20.0);
        assert_eq!(regrets[1].upper_witness_scenario_id, SCENARIO_A);
    }

    #[test]
    fn input_scenario_order_does_not_change_robust_report() {
        let a = scenario_report(SCENARIO_A, [0.0, 10.0, 20.0], 0.025);
        let b = scenario_report(SCENARIO_B, [20.0, 0.0, 10.0], 0.025);
        let forward = evaluator().aggregate(&[a.clone(), b.clone()]).unwrap();
        let reverse = evaluator().aggregate(&[b, a]).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn tied_minimax_regret_arms_remain_undominated() {
        let reports = [
            scenario_report(SCENARIO_A, [0.0, 0.0, 0.0], 0.025),
            scenario_report(SCENARIO_B, [0.0, 0.0, 0.0], 0.025),
        ];
        let report = evaluator().aggregate(&reports).unwrap();
        assert_eq!(report.unique_minimax_regret_portfolio_id, None);
        assert_eq!(
            report.minimax_regret_undominated_portfolio_ids,
            vec![PORTFOLIO_A, PORTFOLIO_B, PORTFOLIO_C]
        );
    }

    #[test]
    fn missing_duplicate_or_substituted_scenarios_are_rejected() {
        let a = scenario_report(SCENARIO_A, [0.0, 10.0, 20.0], 0.025);
        let b = scenario_report(SCENARIO_B, [20.0, 0.0, 10.0], 0.025);
        assert_eq!(
            evaluator().aggregate(&[a.clone()]),
            Err(EpistemicMinimaxRegretError::IncompleteOrDuplicateScenarioReport)
        );
        assert_eq!(
            evaluator().aggregate(&[a.clone(), a.clone()]),
            Err(EpistemicMinimaxRegretError::IncompleteOrDuplicateScenarioReport)
        );
        let substituted = scenario_report([3; 32], [20.0, 0.0, 10.0], 0.025);
        assert_eq!(
            evaluator().aggregate(&[a, substituted]),
            Err(EpistemicMinimaxRegretError::IncompleteOrDuplicateScenarioReport)
        );
        let _ = b;
    }

    #[test]
    fn alpha_unit_and_pair_matrix_mismatches_fail_closed() {
        let a = scenario_report(SCENARIO_A, [0.0, 10.0, 20.0], 0.025);
        let mut b = scenario_report(SCENARIO_B, [20.0, 0.0, 10.0], 0.025);
        b.family_alpha = 0.05;
        assert_eq!(
            evaluator().aggregate(&[a.clone(), b]),
            Err(EpistemicMinimaxRegretError::InvalidScenarioAlphaAllocation)
        );

        let mut b = scenario_report(SCENARIO_B, [20.0, 0.0, 10.0], 0.025);
        b.consequence_unit_id = [93; 32];
        assert_eq!(
            evaluator().aggregate(&[a.clone(), b]),
            Err(EpistemicMinimaxRegretError::ConsequenceUnitMismatch)
        );

        let mut b = scenario_report(SCENARIO_B, [20.0, 0.0, 10.0], 0.025);
        b.pairwise_intervals.pop();
        assert_eq!(
            evaluator().aggregate(&[a, b]),
            Err(EpistemicMinimaxRegretError::IncompletePortfolioPairFamily)
        );
    }
}
