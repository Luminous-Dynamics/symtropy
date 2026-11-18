//! Finite-scenario robust design evaluation for HoloFrac.
//!
//! Scenario weights are explicitly tagged as analytical decision weights or
//! probability weights. Unweighted climate storylines are never silently
//! interpreted as probabilities.

pub const ROBUST_SCENARIO_SCHEMA_VERSION: u32 = 1;
pub const ROBUST_SCENARIO_GENERATOR: &str = "holofrac-robust-scenario-fixture-v0";
pub const OBJECTIVE_COUNT: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScenarioWeightBasis {
    /// Weights encode a declared decision preference across stress scenarios.
    /// These values are not probabilities.
    Analytical,
    /// Weights are externally justified probabilities for the scenario set.
    Probability,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FutureScenario {
    pub scenario_id: String,
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveWeights {
    /// Higher is better. The normalized objectives are energy efficiency,
    /// thermal/IEQ resilience, and affordability/adaptability.
    pub energy_efficiency: f64,
    pub comfort_resilience: f64,
    pub affordability: f64,
}

impl ObjectiveWeights {
    pub fn as_array(&self) -> [f64; OBJECTIVE_COUNT] {
        [
            self.energy_efficiency,
            self.comfort_resilience,
            self.affordability,
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioOutcome {
    /// Each normalized outcome is in [0, 1], higher is better.
    pub objectives: [f64; OBJECTIVE_COUNT],
    /// Independent Symtropy hard-constraint result in this scenario.
    pub hard_constraints_satisfied: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RobustDesignCandidate {
    pub candidate_id: String,
    /// One outcome for each scenario, in problem.scenarios order.
    pub scenario_outcomes: Vec<ScenarioOutcome>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RobustScenarioProblem {
    pub scenarios: Vec<FutureScenario>,
    pub candidates: Vec<RobustDesignCandidate>,
    pub objective_weights: ObjectiveWeights,
    pub scenario_weight_basis: ScenarioWeightBasis,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioRegret {
    pub scenario_id: String,
    pub best_feasible_utility: Option<f64>,
    pub candidate_feasible: bool,
    pub candidate_utility: Option<f64>,
    pub regret: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateRobustness {
    pub candidate_id: String,
    pub weighted_mean_objectives: [f64; OBJECTIVE_COUNT],
    pub worst_case_objectives: [f64; OBJECTIVE_COUNT],
    pub weighted_mean_utility: f64,
    pub worst_case_utility: f64,
    pub feasible_scenario_count: usize,
    /// Sum of scenario weights where hard constraints are satisfied.
    /// This is not a probability unless scenario_weight_basis is Probability.
    pub weighted_feasible_share: f64,
    pub probability_of_feasibility: Option<f64>,
    pub guaranteed_feasible: bool,
    pub scenario_frontier_membership_count: usize,
    pub scenario_regrets: Vec<ScenarioRegret>,
    /// Defined only for candidates feasible in every declared scenario.
    pub weighted_regret: Option<f64>,
    /// Defined only for candidates feasible in every declared scenario.
    pub maximum_regret: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioParetoFrontier {
    pub scenario_id: String,
    pub candidate_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RobustScenarioReport {
    pub scenario_weight_basis: ScenarioWeightBasis,
    pub scenario_frontiers: Vec<ScenarioParetoFrontier>,
    /// Candidates non-dominated in every scenario-specific feasible frontier.
    pub universal_pareto_candidate_ids: Vec<String>,
    /// Pareto frontier computed from candidate-wise worst-case objective vectors,
    /// restricted to candidates feasible in every declared scenario.
    pub robust_worst_case_frontier_ids: Vec<String>,
    /// Pareto frontier of weighted-mean objective vectors, restricted to
    /// candidates feasible in every declared scenario.
    pub weighted_mean_frontier_ids: Vec<String>,
    pub candidates: Vec<CandidateRobustness>,
}

impl RobustScenarioReport {
    pub fn rank_guaranteed_by_maximin(&self) -> Vec<String> {
        let mut rows: Vec<_> = self
            .candidates
            .iter()
            .filter(|candidate| candidate.guaranteed_feasible)
            .collect();
        rows.sort_by(|left, right| {
            right
                .worst_case_utility
                .total_cmp(&left.worst_case_utility)
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });
        rows.into_iter()
            .map(|candidate| candidate.candidate_id.clone())
            .collect()
    }

    pub fn rank_guaranteed_by_weighted_mean(&self) -> Vec<String> {
        let mut rows: Vec<_> = self
            .candidates
            .iter()
            .filter(|candidate| candidate.guaranteed_feasible)
            .collect();
        rows.sort_by(|left, right| {
            right
                .weighted_mean_utility
                .total_cmp(&left.weighted_mean_utility)
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });
        rows.into_iter()
            .map(|candidate| candidate.candidate_id.clone())
            .collect()
    }

    pub fn rank_guaranteed_by_minimax_regret(&self) -> Vec<String> {
        let mut rows: Vec<_> = self
            .candidates
            .iter()
            .filter(|candidate| candidate.guaranteed_feasible)
            .filter(|candidate| candidate.maximum_regret.is_some())
            .collect();
        rows.sort_by(|left, right| {
            left.maximum_regret
                .unwrap_or(f64::INFINITY)
                .total_cmp(&right.maximum_regret.unwrap_or(f64::INFINITY))
                .then_with(|| left.candidate_id.cmp(&right.candidate_id))
        });
        rows.into_iter()
            .map(|candidate| candidate.candidate_id.clone())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RobustScenarioError {
    EmptyScenarioSet,
    EmptyCandidateSet,
    EmptyScenarioId,
    EmptyCandidateId,
    DuplicateScenarioId(String),
    DuplicateCandidateId(String),
    InvalidScenarioWeights,
    InvalidObjectiveWeights,
    InvalidScenarioOutcomeCount(String),
    InvalidObjectiveOutcome(String),
}

impl std::fmt::Display for RobustScenarioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyScenarioSet => f.write_str("at least one future scenario is required"),
            Self::EmptyCandidateSet => f.write_str("at least one design candidate is required"),
            Self::EmptyScenarioId => f.write_str("scenario ID must not be empty"),
            Self::EmptyCandidateId => f.write_str("candidate ID must not be empty"),
            Self::DuplicateScenarioId(id) => write!(f, "duplicate scenario ID: {id}"),
            Self::DuplicateCandidateId(id) => write!(f, "duplicate candidate ID: {id}"),
            Self::InvalidScenarioWeights => {
                f.write_str("scenario weights must be finite, non-negative and sum to one")
            }
            Self::InvalidObjectiveWeights => {
                f.write_str("objective weights must be finite, non-negative and sum to one")
            }
            Self::InvalidScenarioOutcomeCount(id) => {
                write!(f, "candidate has wrong number of scenario outcomes: {id}")
            }
            Self::InvalidObjectiveOutcome(id) => {
                write!(f, "objective values must be finite and in [0, 1]: {id}")
            }
        }
    }
}

impl std::error::Error for RobustScenarioError {}

fn utility(objectives: &[f64; OBJECTIVE_COUNT], weights: &[f64; OBJECTIVE_COUNT]) -> f64 {
    objectives
        .iter()
        .zip(weights.iter())
        .map(|(objective, weight)| objective * weight)
        .sum()
}

fn dominates(left: &[f64; OBJECTIVE_COUNT], right: &[f64; OBJECTIVE_COUNT]) -> bool {
    left.iter().zip(right.iter()).all(|(a, b)| a >= b)
        && left.iter().zip(right.iter()).any(|(a, b)| a > b)
}

fn pareto_frontier_ids(
    candidates: &[(&str, [f64; OBJECTIVE_COUNT])],
) -> Vec<String> {
    let mut result = Vec::new();
    for (candidate_id, objectives) in candidates {
        let is_dominated = candidates.iter().any(|(other_id, other_objectives)| {
            other_id != candidate_id && dominates(other_objectives, objectives)
        });
        if !is_dominated {
            result.push((*candidate_id).to_owned());
        }
    }
    result.sort();
    result
}

impl RobustScenarioProblem {
    pub fn validate(&self) -> Result<(), RobustScenarioError> {
        if self.scenarios.is_empty() {
            return Err(RobustScenarioError::EmptyScenarioSet);
        }
        if self.candidates.is_empty() {
            return Err(RobustScenarioError::EmptyCandidateSet);
        }

        let mut scenario_ids = std::collections::BTreeSet::new();
        for scenario in &self.scenarios {
            if scenario.scenario_id.trim().is_empty() {
                return Err(RobustScenarioError::EmptyScenarioId);
            }
            if !scenario_ids.insert(&scenario.scenario_id) {
                return Err(RobustScenarioError::DuplicateScenarioId(
                    scenario.scenario_id.clone(),
                ));
            }
        }
        if self
            .scenarios
            .iter()
            .any(|scenario| !scenario.weight.is_finite() || scenario.weight < 0.0)
            || (self.scenarios.iter().map(|scenario| scenario.weight).sum::<f64>() - 1.0)
                .abs()
                > 1.0e-9
        {
            return Err(RobustScenarioError::InvalidScenarioWeights);
        }

        let weights = self.objective_weights.as_array();
        if weights
            .iter()
            .any(|weight| !weight.is_finite() || *weight < 0.0)
            || (weights.iter().sum::<f64>() - 1.0).abs() > 1.0e-9
        {
            return Err(RobustScenarioError::InvalidObjectiveWeights);
        }

        let mut candidate_ids = std::collections::BTreeSet::new();
        for candidate in &self.candidates {
            if candidate.candidate_id.trim().is_empty() {
                return Err(RobustScenarioError::EmptyCandidateId);
            }
            if !candidate_ids.insert(&candidate.candidate_id) {
                return Err(RobustScenarioError::DuplicateCandidateId(
                    candidate.candidate_id.clone(),
                ));
            }
            if candidate.scenario_outcomes.len() != self.scenarios.len() {
                return Err(RobustScenarioError::InvalidScenarioOutcomeCount(
                    candidate.candidate_id.clone(),
                ));
            }
            for outcome in &candidate.scenario_outcomes {
                if outcome
                    .objectives
                    .iter()
                    .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
                {
                    return Err(RobustScenarioError::InvalidObjectiveOutcome(
                        candidate.candidate_id.clone(),
                    ));
                }
            }
        }

        Ok(())
    }

    pub fn evaluate(&self) -> Result<RobustScenarioReport, RobustScenarioError> {
        self.validate()?;
        let objective_weights = self.objective_weights.as_array();
        let scenario_count = self.scenarios.len();

        // Determine the best feasible utility in each scenario. Regret is
        // undefined if no candidate is feasible under that scenario.
        let best_scenario_utility: Vec<Option<f64>> = (0..scenario_count)
            .map(|scenario_index| {
                self.candidates
                    .iter()
                    .filter(|candidate| {
                        candidate.scenario_outcomes[scenario_index].hard_constraints_satisfied
                    })
                    .map(|candidate| {
                        utility(
                            &candidate.scenario_outcomes[scenario_index].objectives,
                            &objective_weights,
                        )
                    })
                    .reduce(f64::max)
            })
            .collect();

        let mut scenario_frontiers = Vec::with_capacity(scenario_count);
        for (scenario_index, scenario) in self.scenarios.iter().enumerate() {
            let feasible_points: Vec<_> = self
                .candidates
                .iter()
                .filter(|candidate| {
                    candidate.scenario_outcomes[scenario_index].hard_constraints_satisfied
                })
                .map(|candidate| {
                    (
                        candidate.candidate_id.as_str(),
                        candidate.scenario_outcomes[scenario_index].objectives,
                    )
                })
                .collect();
            scenario_frontiers.push(ScenarioParetoFrontier {
                scenario_id: scenario.scenario_id.clone(),
                candidate_ids: pareto_frontier_ids(&feasible_points),
            });
        }

        let mut candidates = Vec::with_capacity(self.candidates.len());
        for candidate in &self.candidates {
            let mut weighted_mean_objectives = [0.0; OBJECTIVE_COUNT];
            let mut worst_case_objectives = [f64::INFINITY; OBJECTIVE_COUNT];
            let mut weighted_mean_utility = 0.0;
            let mut worst_case_utility = f64::INFINITY;
            let mut feasible_scenario_count = 0;
            let mut weighted_feasible_share = 0.0;
            let mut scenario_regrets = Vec::with_capacity(scenario_count);

            for (scenario_index, (scenario, outcome)) in self
                .scenarios
                .iter()
                .zip(candidate.scenario_outcomes.iter())
                .enumerate()
            {
                let outcome_utility = utility(&outcome.objectives, &objective_weights);
                weighted_mean_utility += scenario.weight * outcome_utility;
                worst_case_utility = worst_case_utility.min(outcome_utility);
                for objective_index in 0..OBJECTIVE_COUNT {
                    weighted_mean_objectives[objective_index] +=
                        scenario.weight * outcome.objectives[objective_index];
                    worst_case_objectives[objective_index] =
                        worst_case_objectives[objective_index].min(outcome.objectives[objective_index]);
                }

                if outcome.hard_constraints_satisfied {
                    feasible_scenario_count += 1;
                    weighted_feasible_share += scenario.weight;
                }

                let best_utility = best_scenario_utility[scenario_index];
                let candidate_utility = outcome
                    .hard_constraints_satisfied
                    .then_some(outcome_utility);
                let regret = match (candidate_utility, best_utility) {
                    (Some(candidate_value), Some(best_value)) => {
                        Some((best_value - candidate_value).max(0.0))
                    }
                    _ => None,
                };
                scenario_regrets.push(ScenarioRegret {
                    scenario_id: scenario.scenario_id.clone(),
                    best_feasible_utility: best_utility,
                    candidate_feasible: outcome.hard_constraints_satisfied,
                    candidate_utility,
                    regret,
                });
            }

            let guaranteed_feasible = feasible_scenario_count == scenario_count;
            let weighted_regret = guaranteed_feasible.then(|| {
                scenario_regrets
                    .iter()
                    .zip(self.scenarios.iter())
                    .map(|(regret, scenario)| scenario.weight * regret.regret.unwrap_or(0.0))
                    .sum()
            });
            let maximum_regret = guaranteed_feasible.then(|| {
                scenario_regrets
                    .iter()
                    .filter_map(|regret| regret.regret)
                    .fold(0.0, f64::max)
            });
            let scenario_frontier_membership_count = scenario_frontiers
                .iter()
                .filter(|frontier| frontier.candidate_ids.contains(&candidate.candidate_id))
                .count();
            let probability_of_feasibility = match self.scenario_weight_basis {
                ScenarioWeightBasis::Analytical => None,
                ScenarioWeightBasis::Probability => Some(weighted_feasible_share),
            };

            candidates.push(CandidateRobustness {
                candidate_id: candidate.candidate_id.clone(),
                weighted_mean_objectives,
                worst_case_objectives,
                weighted_mean_utility,
                worst_case_utility,
                feasible_scenario_count,
                weighted_feasible_share,
                probability_of_feasibility,
                guaranteed_feasible,
                scenario_frontier_membership_count,
                scenario_regrets,
                weighted_regret,
                maximum_regret,
            });
        }

        let guaranteed_points: Vec<_> = candidates
            .iter()
            .filter(|candidate| candidate.guaranteed_feasible)
            .map(|candidate| {
                (
                    candidate.candidate_id.as_str(),
                    candidate.worst_case_objectives,
                )
            })
            .collect();
        let robust_worst_case_frontier_ids = pareto_frontier_ids(&guaranteed_points);

        let weighted_mean_points: Vec<_> = candidates
            .iter()
            .filter(|candidate| candidate.guaranteed_feasible)
            .map(|candidate| {
                (
                    candidate.candidate_id.as_str(),
                    candidate.weighted_mean_objectives,
                )
            })
            .collect();
        let weighted_mean_frontier_ids = pareto_frontier_ids(&weighted_mean_points);

        let universal_pareto_candidate_ids: Vec<_> = candidates
            .iter()
            .filter(|candidate| {
                candidate.guaranteed_feasible
                    && candidate.scenario_frontier_membership_count == scenario_count
            })
            .map(|candidate| candidate.candidate_id.clone())
            .collect();

        Ok(RobustScenarioReport {
            scenario_weight_basis: self.scenario_weight_basis,
            scenario_frontiers,
            universal_pareto_candidate_ids,
            robust_worst_case_frontier_ids,
            weighted_mean_frontier_ids,
            candidates,
        })
    }
}

/// A four-candidate fixture where the nominal specialist loses under hotter
/// futures, while a balanced candidate retains feasibility and limits regret.
pub fn synthetic_robust_scenario_problem(
    weight_basis: ScenarioWeightBasis,
) -> RobustScenarioProblem {
    let scenarios = vec![
        FutureScenario {
            scenario_id: "present_tmy".to_owned(),
            weight: 0.2,
        },
        FutureScenario {
            scenario_id: "2050_hot_ssp".to_owned(),
            weight: 0.5,
        },
        FutureScenario {
            scenario_id: "2090_compound_heat".to_owned(),
            weight: 0.3,
        },
    ];

    let candidates = vec![
        RobustDesignCandidate {
            candidate_id: "present_specialist".to_owned(),
            scenario_outcomes: vec![
                ScenarioOutcome {
                    objectives: [0.98, 0.90, 0.80],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.40, 0.50, 0.80],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.20, 0.30, 0.70],
                    hard_constraints_satisfied: true,
                },
            ],
        },
        RobustDesignCandidate {
            candidate_id: "balanced_robust".to_owned(),
            scenario_outcomes: vec![
                ScenarioOutcome {
                    objectives: [0.85, 0.80, 0.70],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.78, 0.76, 0.68],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.70, 0.70, 0.65],
                    hard_constraints_satisfied: true,
                },
            ],
        },
        RobustDesignCandidate {
            candidate_id: "cost_specialist".to_owned(),
            scenario_outcomes: vec![
                ScenarioOutcome {
                    objectives: [0.70, 0.60, 0.95],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.68, 0.58, 0.94],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.65, 0.52, 0.92],
                    hard_constraints_satisfied: true,
                },
            ],
        },
        RobustDesignCandidate {
            candidate_id: "fragile_high_score".to_owned(),
            scenario_outcomes: vec![
                ScenarioOutcome {
                    objectives: [0.90, 0.95, 0.60],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.80, 0.85, 0.60],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.95, 0.90, 0.80],
                    hard_constraints_satisfied: false,
                },
            ],
        },
        RobustDesignCandidate {
            candidate_id: "dominated_baseline".to_owned(),
            scenario_outcomes: vec![
                ScenarioOutcome {
                    objectives: [0.30, 0.30, 0.30],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.30, 0.30, 0.30],
                    hard_constraints_satisfied: true,
                },
                ScenarioOutcome {
                    objectives: [0.30, 0.30, 0.30],
                    hard_constraints_satisfied: true,
                },
            ],
        },
    ];

    RobustScenarioProblem {
        scenarios,
        candidates,
        objective_weights: ObjectiveWeights {
            energy_efficiency: 0.5,
            comfort_resilience: 0.3,
            affordability: 0.2,
        },
        scenario_weight_basis: weight_basis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(basis: ScenarioWeightBasis) -> RobustScenarioReport {
        synthetic_robust_scenario_problem(basis).evaluate().unwrap()
    }

    fn candidate<'a>(
        report: &'a RobustScenarioReport,
        id: &str,
    ) -> &'a CandidateRobustness {
        report
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == id)
            .unwrap()
    }

    #[test]
    fn nominal_specialist_and_robust_choice_are_not_the_same() {
        let report = report(ScenarioWeightBasis::Analytical);
        assert_eq!(
            report.rank_guaranteed_by_maximin()[0],
            "balanced_robust"
        );
        assert_eq!(
            report.rank_guaranteed_by_minimax_regret()[0],
            "balanced_robust"
        );
        assert_eq!(
            report.rank_guaranteed_by_weighted_mean()[0],
            "balanced_robust"
        );

        let specialist = candidate(&report, "present_specialist");
        let balanced = candidate(&report, "balanced_robust");
        assert!(specialist.worst_case_utility < balanced.worst_case_utility);
        assert!(specialist.maximum_regret.unwrap() > balanced.maximum_regret.unwrap());
    }

    #[test]
    fn one_failed_hard_scenario_prevents_guaranteed_feasibility() {
        let report = report(ScenarioWeightBasis::Analytical);
        let fragile = candidate(&report, "fragile_high_score");

        assert_eq!(fragile.feasible_scenario_count, 2);
        assert!(!fragile.guaranteed_feasible);
        assert!((fragile.weighted_feasible_share - 0.7).abs() < 1.0e-12);
        assert_eq!(fragile.probability_of_feasibility, None);
        assert_eq!(fragile.maximum_regret, None);
        assert!(!report
            .rank_guaranteed_by_maximin()
            .contains(&"fragile_high_score".to_owned()));
    }

    #[test]
    fn feasibility_probability_is_only_exposed_for_probability_grounded_weights() {
        let report = report(ScenarioWeightBasis::Probability);
        let fragile = candidate(&report, "fragile_high_score");

        assert_eq!(fragile.probability_of_feasibility, Some(0.7));
        let robust = candidate(&report, "balanced_robust");
        assert_eq!(robust.probability_of_feasibility, Some(1.0));
    }

    #[test]
    fn dominated_candidate_is_absent_from_each_scenario_and_robust_frontier() {
        let report = report(ScenarioWeightBasis::Analytical);
        assert!(report.scenario_frontiers.iter().all(|frontier| {
            !frontier
                .candidate_ids
                .contains(&"dominated_baseline".to_owned())
        }));
        assert!(!report
            .robust_worst_case_frontier_ids
            .contains(&"dominated_baseline".to_owned()));
    }

    #[test]
    fn independent_scenario_frontier_oracle_matches_frozen_fixture() {
        let report = report(ScenarioWeightBasis::Analytical);
        let expected = [
            (
                "present_tmy",
                vec![
                    "cost_specialist",
                    "fragile_high_score",
                    "present_specialist",
                ],
            ),
            (
                "2050_hot_ssp",
                vec![
                    "balanced_robust",
                    "cost_specialist",
                    "fragile_high_score",
                ],
            ),
            (
                "2090_compound_heat",
                vec![
                    "balanced_robust",
                    "cost_specialist",
                    "present_specialist",
                ],
            ),
        ];

        for (frontier, (scenario_id, expected_ids)) in
            report.scenario_frontiers.iter().zip(expected.iter())
        {
            assert_eq!(frontier.scenario_id, *scenario_id);
            assert_eq!(
                frontier.candidate_ids,
                expected_ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>()
            );
        }

        assert_eq!(
            report.universal_pareto_candidate_ids,
            vec!["cost_specialist".to_owned()]
        );
    }

    #[test]
    fn invalid_weight_basis_inputs_fail_closed() {
        let mut problem =
            synthetic_robust_scenario_problem(ScenarioWeightBasis::Analytical);
        problem.scenarios[0].weight = 0.4;

        assert_eq!(
            problem.validate(),
            Err(RobustScenarioError::InvalidScenarioWeights)
        );

        let mut invalid_objectives =
            synthetic_robust_scenario_problem(ScenarioWeightBasis::Analytical);
        invalid_objectives.objective_weights.energy_efficiency = 0.8;

        assert_eq!(
            invalid_objectives.validate(),
            Err(RobustScenarioError::InvalidObjectiveWeights)
        );
    }
}
