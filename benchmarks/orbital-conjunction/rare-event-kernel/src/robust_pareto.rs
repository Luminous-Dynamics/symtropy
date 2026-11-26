// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite Pareto frontiers with per-scenario hard feasibility gates.
//!
//! This reference kernel retains each epistemic scenario separately, computes
//! a frontier within each scenario, and computes a second robust frontier from
//! worst-case objective vectors over the scenarios. It never averages scenario
//! values or converts unresolved hard constraints into favorable scores.
//!
//! Objective values and feasibility assessments are supplied by upstream
//! evidence-producing systems. This module performs exact finite comparison;
//! it does not authenticate those sources, propagate trajectories, calculate
//! collision probability or grant operational authority.

use std::collections::{BTreeMap, BTreeSet};

/// Maximum number of portfolios in one exact finite Pareto campaign.
pub const MAX_PARETO_PORTFOLIOS: usize = 4_096;

/// Maximum number of epistemic scenarios in one exact finite Pareto campaign.
pub const MAX_PARETO_SCENARIOS: usize = 256;

/// Maximum number of objective dimensions in one exact finite Pareto campaign.
pub const MAX_PARETO_OBJECTIVES: usize = 32;

/// Maximum complete portfolio × scenario assessment rows in one campaign.
pub const MAX_PARETO_ASSESSMENT_ROWS: usize = 65_536;

/// Direction of preference for one frozen objective.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectiveDirection {
    /// Lower values are preferred.
    Minimize,
    /// Higher values are preferred.
    Maximize,
}

/// Frozen identity, unit and direction for one campaign objective.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParetoObjectiveDefinition {
    /// Stable, nonzero objective identity.
    pub objective_id: [u8; 32],
    /// Stable, nonzero unit / scale identity for integer ticks.
    pub unit_id: [u8; 32],
    /// Whether lower or higher values are preferred.
    pub direction: ObjectiveDirection,
}

/// One integer-valued objective observation for one portfolio and scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParetoObjectiveValue {
    /// Objective identity matching the frozen campaign schema.
    pub objective_id: [u8; 32],
    /// Exact value in the objective's declared integer tick unit.
    pub value_ticks: i64,
}

/// Hard feasibility state for one portfolio under one epistemic scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioFeasibility {
    /// Every upstream hard constraint required for this scenario is qualified.
    Feasible,
    /// At least one hard constraint is affirmatively violated.
    Infeasible {
        /// Nonzero digest identifying the source finding / reason.
        reason_digest: [u8; 32],
    },
    /// Feasibility cannot be established from the supplied evidence.
    Unresolved {
        /// Nonzero digest identifying the missing / ambiguous evidence.
        reason_digest: [u8; 32],
    },
}

/// Complete objective and hard-feasibility assessment for one portfolio/scenario pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioScenarioAssessment {
    /// Stable portfolio identity in the frozen campaign.
    pub portfolio_id: [u8; 32],
    /// Stable epistemic scenario identity in the frozen campaign.
    pub scenario_id: [u8; 32],
    /// Hard feasibility disposition; never replaced by an objective score.
    pub feasibility: ScenarioFeasibility,
    /// Exactly one value for every frozen objective, in any input order.
    pub objective_values: Vec<ParetoObjectiveValue>,
}

/// Frozen finite campaign and its complete portfolio × scenario assessment product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobustParetoConfig {
    /// Exact portfolio identities to compare.
    pub portfolio_ids: Vec<[u8; 32]>,
    /// Exact epistemic scenarios which must remain separately represented.
    pub scenario_ids: Vec<[u8; 32]>,
    /// Frozen objective definitions, including units and direction.
    pub objectives: Vec<ParetoObjectiveDefinition>,
    /// Exactly one assessment for every portfolio × scenario pair.
    pub assessments: Vec<PortfolioScenarioAssessment>,
}

/// One portfolio known to be dominated in a particular comparison set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParetoDominance {
    /// Portfolio whose objective vector is dominated.
    pub portfolio_id: [u8; 32],
    /// Every compared portfolio proven to dominate it; sorted and unique.
    pub dominated_by: Vec<[u8; 32]>,
}

/// Exact Pareto frontier for one admitted epistemic scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioParetoFrontier {
    /// Scenario this frontier belongs to.
    pub scenario_id: [u8; 32],
    /// Portfolios whose hard feasibility is qualified in this scenario.
    pub feasible_portfolio_ids: Vec<[u8; 32]>,
    /// Nondominated portfolios among the scenario-feasible set.
    pub frontier_portfolio_ids: Vec<[u8; 32]>,
    /// Dominance results for scenario-feasible portfolios.
    pub dominance: Vec<ParetoDominance>,
}

/// Worst-case objective vector for one portfolio feasible in every scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobustPortfolioObjectiveSummary {
    /// Portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Worst-case value per objective: max for minimized objectives, min for maximized.
    pub worst_case_objectives: Vec<ParetoObjectiveValue>,
    /// Other robust-feasible portfolios which dominate this worst-case vector.
    pub dominated_by: Vec<[u8; 32]>,
}

/// Exact scenario-separated and robust worst-case Pareto report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobustParetoReport {
    /// Scenario frontier results in canonical scenario order.
    pub scenario_frontiers: Vec<ScenarioParetoFrontier>,
    /// Portfolios proven feasible in every declared epistemic scenario.
    pub robust_feasible_portfolio_ids: Vec<[u8; 32]>,
    /// Portfolios with at least one unresolved scenario and no known infeasible scenario.
    pub unresolved_portfolio_ids: Vec<[u8; 32]>,
    /// Portfolios affirmatively infeasible in at least one declared scenario.
    pub infeasible_portfolio_ids: Vec<[u8; 32]>,
    /// Worst-case objective summaries for robust-feasible portfolios.
    pub robust_portfolio_summaries: Vec<RobustPortfolioObjectiveSummary>,
    /// Nondominated portfolios under the worst-case objective vectors.
    pub robust_frontier_portfolio_ids: Vec<[u8; 32]>,
    /// Whether any portfolio could not be classified as robust feasible/infeasible.
    pub has_unresolved_feasibility: bool,
}

/// Validation and arithmetic errors for the finite robust Pareto kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobustParetoError {
    /// Portfolio, scenario or objective universe is empty.
    EmptyUniverse,
    /// A frozen universe exceeds its configured finite bound.
    UniverseTooLarge,
    /// A required stable identity is zero.
    ZeroIdentity,
    /// A frozen identity is duplicated.
    DuplicateIdentity,
    /// An assessment references an undeclared portfolio or scenario.
    UnexpectedAssessment,
    /// An objective identity is absent, duplicated or extra in an assessment.
    InvalidObjectiveProduct,
    /// One portfolio/scenario assessment is missing or duplicated.
    IncompleteAssessmentProduct,
    /// Infeasible or unresolved assessment lacks a reason digest.
    InvalidFeasibilityReason,
}

impl core::fmt::Display for RobustParetoError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::EmptyUniverse => "portfolio, scenario and objective universes must be nonempty",
            Self::UniverseTooLarge => "finite Pareto campaign exceeds a configured universe bound",
            Self::ZeroIdentity => {
                "a frozen portfolio, scenario, objective or unit identity is zero"
            }
            Self::DuplicateIdentity => {
                "a frozen portfolio, scenario or objective identity is duplicated"
            }
            Self::UnexpectedAssessment => {
                "assessment references an undeclared portfolio or scenario"
            }
            Self::InvalidObjectiveProduct => {
                "assessment does not contain the exact frozen objective product"
            }
            Self::IncompleteAssessmentProduct => {
                "assessment product lacks exactly one row per portfolio and scenario"
            }
            Self::InvalidFeasibilityReason => {
                "infeasible or unresolved assessment lacks a reason digest"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RobustParetoError {}

/// Compute scenario-separated and robust worst-case Pareto frontiers.
///
/// The function first validates the complete finite portfolio × scenario ×
/// objective product. A portfolio is robust-feasible only when every declared
/// scenario marks it Feasible. A known Infeasible scenario excludes it from
/// the robust set even if another scenario is Unresolved; otherwise unresolved
/// feasibility is retained in a separate output set.
///
/// Scenario frontiers compare only portfolios feasible in that scenario.
/// The robust frontier compares only portfolios feasible in all scenarios,
/// using a worst-case value per objective (maximum for minimized objectives,
/// minimum for maximized objectives). No cross-scenario averaging occurs.
///
/// Pareto dominance is exact over the supplied integer ticks. Equal vectors
/// do not dominate one another. No scalar weighted score is constructed.
pub fn evaluate_robust_pareto(
    mut config: RobustParetoConfig,
) -> Result<RobustParetoReport, RobustParetoError> {
    if config.portfolio_ids.is_empty()
        || config.scenario_ids.is_empty()
        || config.objectives.is_empty()
    {
        return Err(RobustParetoError::EmptyUniverse);
    }
    if config.portfolio_ids.len() > MAX_PARETO_PORTFOLIOS
        || config.scenario_ids.len() > MAX_PARETO_SCENARIOS
        || config.objectives.len() > MAX_PARETO_OBJECTIVES
    {
        return Err(RobustParetoError::UniverseTooLarge);
    }

    config.portfolio_ids.sort_unstable();
    config.scenario_ids.sort_unstable();
    config
        .objectives
        .sort_unstable_by_key(|objective| objective.objective_id);

    validate_unique_nonzero(&config.portfolio_ids)?;
    validate_unique_nonzero(&config.scenario_ids)?;
    let objective_ids = config
        .objectives
        .iter()
        .map(|objective| objective.objective_id)
        .collect::<Vec<_>>();
    validate_unique_nonzero(&objective_ids)?;
    if config
        .objectives
        .iter()
        .any(|objective| objective.unit_id.iter().all(|byte| *byte == 0))
    {
        return Err(RobustParetoError::ZeroIdentity);
    }

    let expected_rows = config
        .portfolio_ids
        .len()
        .checked_mul(config.scenario_ids.len())
        .ok_or(RobustParetoError::UniverseTooLarge)?;
    if expected_rows > MAX_PARETO_ASSESSMENT_ROWS {
        return Err(RobustParetoError::UniverseTooLarge);
    }
    if config.assessments.len() != expected_rows {
        return Err(RobustParetoError::IncompleteAssessmentProduct);
    }

    let portfolio_set = config
        .portfolio_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let scenario_set = config.scenario_ids.iter().copied().collect::<BTreeSet<_>>();
    let mut rows = BTreeMap::new();
    for mut assessment in config.assessments {
        if !portfolio_set.contains(&assessment.portfolio_id)
            || !scenario_set.contains(&assessment.scenario_id)
        {
            return Err(RobustParetoError::UnexpectedAssessment);
        }
        if let ScenarioFeasibility::Infeasible { reason_digest }
        | ScenarioFeasibility::Unresolved { reason_digest } = assessment.feasibility
        {
            if reason_digest.iter().all(|byte| *byte == 0) {
                return Err(RobustParetoError::InvalidFeasibilityReason);
            }
        }

        assessment
            .objective_values
            .sort_unstable_by_key(|value| value.objective_id);
        let observed_ids = assessment
            .objective_values
            .iter()
            .map(|value| value.objective_id)
            .collect::<Vec<_>>();
        if observed_ids != objective_ids {
            return Err(RobustParetoError::InvalidObjectiveProduct);
        }
        if rows
            .insert(
                (assessment.portfolio_id, assessment.scenario_id),
                assessment,
            )
            .is_some()
        {
            return Err(RobustParetoError::IncompleteAssessmentProduct);
        }
    }
    if rows.len() != expected_rows {
        return Err(RobustParetoError::IncompleteAssessmentProduct);
    }

    let mut scenario_frontiers = Vec::with_capacity(config.scenario_ids.len());
    for scenario_id in &config.scenario_ids {
        let feasible = config
            .portfolio_ids
            .iter()
            .filter_map(|portfolio_id| {
                let row = rows.get(&(*portfolio_id, *scenario_id))?;
                (row.feasibility == ScenarioFeasibility::Feasible)
                    .then_some((*portfolio_id, row.objective_values.clone()))
            })
            .collect::<Vec<_>>();
        let (frontier_portfolio_ids, dominance) = calculate_frontier(&feasible, &config.objectives);
        scenario_frontiers.push(ScenarioParetoFrontier {
            scenario_id: *scenario_id,
            feasible_portfolio_ids: feasible.iter().map(|(id, _)| *id).collect(),
            frontier_portfolio_ids,
            dominance,
        });
    }

    let mut robust_feasible_portfolio_ids = Vec::new();
    let mut unresolved_portfolio_ids = Vec::new();
    let mut infeasible_portfolio_ids = Vec::new();
    for portfolio_id in &config.portfolio_ids {
        let assessments = config
            .scenario_ids
            .iter()
            .filter_map(|scenario_id| rows.get(&(*portfolio_id, *scenario_id)))
            .collect::<Vec<_>>();
        let known_infeasible = assessments
            .iter()
            .any(|row| matches!(row.feasibility, ScenarioFeasibility::Infeasible { .. }));
        let unresolved = assessments
            .iter()
            .any(|row| matches!(row.feasibility, ScenarioFeasibility::Unresolved { .. }));
        if known_infeasible {
            infeasible_portfolio_ids.push(*portfolio_id);
        } else if unresolved {
            unresolved_portfolio_ids.push(*portfolio_id);
        } else {
            robust_feasible_portfolio_ids.push(*portfolio_id);
        }
    }

    let mut robust_summaries = Vec::with_capacity(robust_feasible_portfolio_ids.len());
    for portfolio_id in &robust_feasible_portfolio_ids {
        let mut worst_case_objectives = Vec::with_capacity(config.objectives.len());
        for objective in &config.objectives {
            let values = config
                .scenario_ids
                .iter()
                .map(|scenario_id| {
                    rows.get(&(*portfolio_id, *scenario_id))
                        .and_then(|row| {
                            row.objective_values
                                .binary_search_by_key(&objective.objective_id, |value| {
                                    value.objective_id
                                })
                                .ok()
                                .map(|index| row.objective_values[index].value_ticks)
                        })
                        .ok_or(RobustParetoError::IncompleteAssessmentProduct)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let worst_case = match objective.direction {
                ObjectiveDirection::Minimize => *values
                    .iter()
                    .max()
                    .ok_or(RobustParetoError::IncompleteAssessmentProduct)?,
                ObjectiveDirection::Maximize => *values
                    .iter()
                    .min()
                    .ok_or(RobustParetoError::IncompleteAssessmentProduct)?,
            };
            worst_case_objectives.push(ParetoObjectiveValue {
                objective_id: objective.objective_id,
                value_ticks: worst_case,
            });
        }
        robust_summaries.push(((*portfolio_id), worst_case_objectives));
    }

    let robust_vectors = robust_summaries
        .iter()
        .map(|(id, values)| (*id, values.clone()))
        .collect::<Vec<_>>();
    let (robust_frontier_portfolio_ids, robust_dominance) =
        calculate_frontier(&robust_vectors, &config.objectives);
    let dominated_by = robust_dominance
        .into_iter()
        .map(|row| (row.portfolio_id, row.dominated_by))
        .collect::<BTreeMap<_, _>>();
    let robust_portfolio_summaries = robust_summaries
        .into_iter()
        .map(
            |(portfolio_id, worst_case_objectives)| RobustPortfolioObjectiveSummary {
                portfolio_id,
                worst_case_objectives,
                dominated_by: dominated_by.get(&portfolio_id).cloned().unwrap_or_default(),
            },
        )
        .collect();

    Ok(RobustParetoReport {
        scenario_frontiers,
        robust_feasible_portfolio_ids,
        unresolved_portfolio_ids,
        infeasible_portfolio_ids,
        robust_portfolio_summaries,
        robust_frontier_portfolio_ids,
        has_unresolved_feasibility: !rows
            .values()
            .all(|row| !matches!(row.feasibility, ScenarioFeasibility::Unresolved { .. })),
    })
}

fn calculate_frontier(
    portfolios: &[([u8; 32], Vec<ParetoObjectiveValue>)],
    objectives: &[ParetoObjectiveDefinition],
) -> (Vec<[u8; 32]>, Vec<ParetoDominance>) {
    let mut frontier = Vec::new();
    let mut dominance = Vec::new();
    for (candidate_id, candidate_values) in portfolios {
        let mut dominated_by = Vec::new();
        for (opponent_id, opponent_values) in portfolios {
            if candidate_id == opponent_id {
                continue;
            }
            if dominates(opponent_values, candidate_values, objectives) {
                dominated_by.push(*opponent_id);
            }
        }
        if dominated_by.is_empty() {
            frontier.push(*candidate_id);
        } else {
            dominance.push(ParetoDominance {
                portfolio_id: *candidate_id,
                dominated_by,
            });
        }
    }
    (frontier, dominance)
}

fn dominates(
    left: &[ParetoObjectiveValue],
    right: &[ParetoObjectiveValue],
    objectives: &[ParetoObjectiveDefinition],
) -> bool {
    let mut strictly_better = false;
    for (index, objective) in objectives.iter().enumerate() {
        let left_value = left[index].value_ticks;
        let right_value = right[index].value_ticks;
        match objective.direction {
            ObjectiveDirection::Minimize => {
                if left_value > right_value {
                    return false;
                }
                strictly_better |= left_value < right_value;
            }
            ObjectiveDirection::Maximize => {
                if left_value < right_value {
                    return false;
                }
                strictly_better |= left_value > right_value;
            }
        }
    }
    strictly_better
}

fn validate_unique_nonzero(values: &[[u8; 32]]) -> Result<(), RobustParetoError> {
    if values
        .iter()
        .any(|value| value.iter().all(|byte| *byte == 0))
    {
        return Err(RobustParetoError::ZeroIdentity);
    }
    if values.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(RobustParetoError::DuplicateIdentity);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn objective(id: u8, unit: u8, direction: ObjectiveDirection) -> ParetoObjectiveDefinition {
        ParetoObjectiveDefinition {
            objective_id: [id; 32],
            unit_id: [unit; 32],
            direction,
        }
    }

    fn row(
        portfolio: u8,
        scenario: u8,
        feasibility: ScenarioFeasibility,
        values: &[(u8, i64)],
    ) -> PortfolioScenarioAssessment {
        PortfolioScenarioAssessment {
            portfolio_id: [portfolio; 32],
            scenario_id: [scenario; 32],
            feasibility,
            objective_values: values
                .iter()
                .map(|(objective_id, value_ticks)| ParetoObjectiveValue {
                    objective_id: [*objective_id; 32],
                    value_ticks: *value_ticks,
                })
                .collect(),
        }
    }

    fn two_scenario_config() -> RobustParetoConfig {
        RobustParetoConfig {
            portfolio_ids: vec![[30; 32], [10; 32], [20; 32]],
            scenario_ids: vec![[2; 32], [1; 32]],
            objectives: vec![
                objective(1, 11, ObjectiveDirection::Minimize),
                objective(2, 12, ObjectiveDirection::Minimize),
            ],
            assessments: vec![
                row(10, 1, ScenarioFeasibility::Feasible, &[(1, 1), (2, 3)]),
                row(20, 1, ScenarioFeasibility::Feasible, &[(1, 2), (2, 2)]),
                row(30, 1, ScenarioFeasibility::Feasible, &[(1, 3), (2, 4)]),
                row(10, 2, ScenarioFeasibility::Feasible, &[(1, 1), (2, 4)]),
                row(20, 2, ScenarioFeasibility::Feasible, &[(1, 2), (2, 1)]),
                row(30, 2, ScenarioFeasibility::Feasible, &[(1, 3), (2, 5)]),
            ],
        }
    }

    #[test]
    fn exact_scenario_frontiers_and_robust_worst_case_frontier_are_computed() {
        let report = evaluate_robust_pareto(two_scenario_config()).unwrap();
        assert_eq!(report.scenario_frontiers.len(), 2);
        assert_eq!(report.scenario_frontiers[0].scenario_id, [1; 32]);
        assert_eq!(
            report.scenario_frontiers[0].frontier_portfolio_ids,
            vec![[10; 32], [20; 32]]
        );
        assert_eq!(report.scenario_frontiers[1].scenario_id, [2; 32]);
        assert_eq!(
            report.scenario_frontiers[1].frontier_portfolio_ids,
            vec![[10; 32], [20; 32]]
        );
        assert_eq!(
            report.robust_frontier_portfolio_ids,
            vec![[10; 32], [20; 32]]
        );
        assert_eq!(
            report.robust_feasible_portfolio_ids,
            vec![[10; 32], [20; 32], [30; 32]]
        );
        let a = &report.robust_portfolio_summaries[0];
        assert_eq!(a.portfolio_id, [10; 32]);
        assert_eq!(
            a.worst_case_objectives,
            vec![
                ParetoObjectiveValue {
                    objective_id: [1; 32],
                    value_ticks: 1
                },
                ParetoObjectiveValue {
                    objective_id: [2; 32],
                    value_ticks: 4
                },
            ]
        );
        assert!(a.dominated_by.is_empty());
        assert_eq!(
            report.robust_portfolio_summaries[2].dominated_by,
            vec![[10; 32], [20; 32]]
        );
    }

    #[test]
    fn identical_objective_vectors_do_not_dominate_each_other() {
        let mut config = two_scenario_config();
        for assessment in &mut config.assessments {
            if assessment.portfolio_id == [20; 32] {
                assessment.objective_values = vec![
                    ParetoObjectiveValue {
                        objective_id: [1; 32],
                        value_ticks: 1,
                    },
                    ParetoObjectiveValue {
                        objective_id: [2; 32],
                        value_ticks: 3,
                    },
                ];
            }
        }
        let report = evaluate_robust_pareto(config).unwrap();
        assert_eq!(
            report.scenario_frontiers[0].frontier_portfolio_ids,
            vec![[10; 32], [20; 32]]
        );
    }

    #[test]
    fn infeasible_portfolio_is_excluded_before_objective_comparison() {
        let mut config = two_scenario_config();
        config.assessments[0].feasibility = ScenarioFeasibility::Infeasible {
            reason_digest: [77; 32],
        };
        let report = evaluate_robust_pareto(config).unwrap();
        assert!(!report.robust_feasible_portfolio_ids.contains(&[10; 32]));
        assert!(report.infeasible_portfolio_ids.contains(&[10; 32]));
        assert!(!report.robust_frontier_portfolio_ids.contains(&[10; 32]));
    }

    #[test]
    fn unresolved_scenario_is_retained_and_blocks_robust_admission() {
        let mut config = two_scenario_config();
        config.assessments[0].feasibility = ScenarioFeasibility::Unresolved {
            reason_digest: [78; 32],
        };
        let report = evaluate_robust_pareto(config).unwrap();
        assert!(report.unresolved_portfolio_ids.contains(&[10; 32]));
        assert!(!report.robust_feasible_portfolio_ids.contains(&[10; 32]));
        assert!(!report.robust_frontier_portfolio_ids.contains(&[10; 32]));
        assert!(report.has_unresolved_feasibility);
    }

    #[test]
    fn maximized_objective_uses_minimum_as_the_robust_worst_case() {
        let config = RobustParetoConfig {
            portfolio_ids: vec![[10; 32], [20; 32]],
            scenario_ids: vec![[1; 32], [2; 32]],
            objectives: vec![objective(1, 11, ObjectiveDirection::Maximize)],
            assessments: vec![
                row(10, 1, ScenarioFeasibility::Feasible, &[(1, 10)]),
                row(10, 2, ScenarioFeasibility::Feasible, &[(1, 7)]),
                row(20, 1, ScenarioFeasibility::Feasible, &[(1, 9)]),
                row(20, 2, ScenarioFeasibility::Feasible, &[(1, 8)]),
            ],
        };
        let report = evaluate_robust_pareto(config).unwrap();
        assert_eq!(
            report.robust_portfolio_summaries[0].worst_case_objectives[0].value_ticks,
            7
        );
        assert_eq!(
            report.robust_frontier_portfolio_ids,
            vec![[10; 32], [20; 32]]
        );
    }

    #[test]
    fn objectives_may_share_a_unit_identity() {
        let config = RobustParetoConfig {
            portfolio_ids: vec![[10; 32], [20; 32]],
            scenario_ids: vec![[1; 32]],
            objectives: vec![
                objective(1, 11, ObjectiveDirection::Minimize),
                objective(2, 11, ObjectiveDirection::Minimize),
            ],
            assessments: vec![
                row(10, 1, ScenarioFeasibility::Feasible, &[(1, 1), (2, 3)]),
                row(20, 1, ScenarioFeasibility::Feasible, &[(1, 2), (2, 2)]),
            ],
        };
        let report = evaluate_robust_pareto(config).unwrap();
        assert_eq!(
            report.robust_frontier_portfolio_ids,
            vec![[10; 32], [20; 32]]
        );
    }

    #[test]
    fn conflicting_scenario_preferences_remain_separate_and_both_robustly_nondominated() {
        let config = RobustParetoConfig {
            portfolio_ids: vec![[10; 32], [20; 32]],
            scenario_ids: vec![[1; 32], [2; 32]],
            objectives: vec![objective(1, 11, ObjectiveDirection::Minimize)],
            assessments: vec![
                row(10, 1, ScenarioFeasibility::Feasible, &[(1, 1)]),
                row(20, 1, ScenarioFeasibility::Feasible, &[(1, 5)]),
                row(10, 2, ScenarioFeasibility::Feasible, &[(1, 5)]),
                row(20, 2, ScenarioFeasibility::Feasible, &[(1, 1)]),
            ],
        };
        let report = evaluate_robust_pareto(config).unwrap();
        assert_eq!(
            report.scenario_frontiers[0].frontier_portfolio_ids,
            vec![[10; 32]]
        );
        assert_eq!(
            report.scenario_frontiers[1].frontier_portfolio_ids,
            vec![[20; 32]]
        );
        assert_eq!(
            report.robust_frontier_portfolio_ids,
            vec![[10; 32], [20; 32]]
        );
        assert_eq!(
            report.robust_portfolio_summaries[0].worst_case_objectives[0].value_ticks,
            5
        );
        assert_eq!(
            report.robust_portfolio_summaries[1].worst_case_objectives[0].value_ticks,
            5
        );
    }

    #[test]
    fn input_order_does_not_change_pareto_report() {
        let forward = two_scenario_config();
        let mut reverse = forward.clone();
        reverse.portfolio_ids.reverse();
        reverse.scenario_ids.reverse();
        reverse.objectives.reverse();
        reverse.assessments.reverse();
        assert_eq!(
            evaluate_robust_pareto(forward),
            evaluate_robust_pareto(reverse)
        );
    }

    #[test]
    fn incomplete_duplicate_and_invalid_objective_products_are_rejected() {
        let mut missing = two_scenario_config();
        missing.assessments.pop();
        assert_eq!(
            evaluate_robust_pareto(missing),
            Err(RobustParetoError::IncompleteAssessmentProduct)
        );

        let mut duplicate = two_scenario_config();
        duplicate.assessments.push(duplicate.assessments[0].clone());
        assert_eq!(
            evaluate_robust_pareto(duplicate),
            Err(RobustParetoError::IncompleteAssessmentProduct)
        );

        let mut wrong_objectives = two_scenario_config();
        wrong_objectives.assessments[0].objective_values.pop();
        assert_eq!(
            evaluate_robust_pareto(wrong_objectives),
            Err(RobustParetoError::InvalidObjectiveProduct)
        );
    }

    #[test]
    fn unresolved_or_infeasible_rows_need_nonzero_reason_digests() {
        let mut config = two_scenario_config();
        config.assessments[0].feasibility = ScenarioFeasibility::Unresolved {
            reason_digest: [0; 32],
        };
        assert_eq!(
            evaluate_robust_pareto(config),
            Err(RobustParetoError::InvalidFeasibilityReason)
        );
    }
}
