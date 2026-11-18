//! Cross-scenario climate robustness metrics for HoloFrac.
//!
//! Scenario weights are explicitly typed as analytical decision weights or
//! probability weights. Analytical weights must never be reported as
//! probabilities. All objectives are normalized to [0, 1] and higher is better.

pub const CLIMATE_CRITERION_COUNT: usize = 3;
pub const CLIMATE_ROBUSTNESS_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClimateCriterion {
    ThermalSafety,
    EnergyEfficiency,
    Affordability,
}

impl ClimateCriterion {
    pub const ALL: [Self; CLIMATE_CRITERION_COUNT] = [
        Self::ThermalSafety,
        Self::EnergyEfficiency,
        Self::Affordability,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ThermalSafety => "thermal_safety",
            Self::EnergyEfficiency => "energy_efficiency",
            Self::Affordability => "affordability",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScenarioWeightSemantics {
    /// Deliberative weights used to express a decision-maker's scenario priorities.
    /// They are not probabilities and do not support an expected-value claim.
    AnalyticalDecisionWeights,
    /// Probability weights are allowed only with an explicit provenance/basis ID.
    ProbabilityWeights { basis_id: String },
}

impl ScenarioWeightSemantics {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::AnalyticalDecisionWeights => "analytical_decision_weights",
            Self::ProbabilityWeights { .. } => "probability_weights",
        }
    }

    pub fn probability_basis(&self) -> Option<&str> {
        match self {
            Self::AnalyticalDecisionWeights => None,
            Self::ProbabilityWeights { basis_id } => Some(basis_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClimateScenario {
    pub scenario_id: String,
    pub decision_weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClimateScenarioSet {
    pub semantics: ScenarioWeightSemantics,
    pub scenarios: Vec<ClimateScenario>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClimateScenarioOutcome {
    pub scenario_id: String,
    pub outcomes: [f64; CLIMATE_CRITERION_COUNT],
    /// Independent Symtropy hard-feasibility result for this candidate/scenario.
    pub feasible: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClimateCandidate {
    pub candidate_id: String,
    pub scenario_outcomes: Vec<ClimateScenarioOutcome>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioFrontierReport {
    pub scenario_id: String,
    pub feasible_candidate_count: usize,
    pub frontier_candidate_ids: Vec<String>,
    pub ideal_outcomes: [f64; CLIMATE_CRITERION_COUNT],
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateRobustnessReport {
    pub candidate_id: String,
    pub weighted_mean_outcomes: [f64; CLIMATE_CRITERION_COUNT],
    pub worst_case_outcomes: [f64; CLIMATE_CRITERION_COUNT],
    /// Weighted fraction of declared scenario decision-weights where the
    /// candidate satisfies hard constraints. This is not a probability unless
    /// the scenario set explicitly uses probability weights.
    pub weighted_feasibility_coverage: f64,
    pub guaranteed_feasible: bool,
    pub scenario_frontier_membership_count: usize,
    pub universal_pareto_member: bool,
    pub weighted_regret_by_criterion: [f64; CLIMATE_CRITERION_COUNT],
    /// Largest criterion regret over any declared scenario.
    pub maximum_regret: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClimateRobustnessReport {
    pub schema_version: u32,
    pub weight_semantics: ScenarioWeightSemantics,
    pub scenario_count: usize,
    pub candidate_count: usize,
    pub scenario_frontiers: Vec<ScenarioFrontierReport>,
    pub union_frontier_candidate_ids: Vec<String>,
    pub universal_pareto_candidate_ids: Vec<String>,
    /// Pareto frontier computed on each candidate's worst-case objective vector,
    /// restricted to candidates feasible in every scenario.
    pub robust_frontier_candidate_ids: Vec<String>,
    /// Mean pairwise Jaccard similarity of scenario-specific frontier ID sets.
    pub mean_pairwise_frontier_jaccard: f64,
    pub candidate_reports: Vec<CandidateRobustnessReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClimateRobustnessError {
    EmptyScenarioSet,
    EmptyCandidateSet,
    InvalidScenarioId,
    DuplicateScenario(String),
    InvalidScenarioWeight(String),
    ScenarioWeightsDoNotSumToOne,
    MissingProbabilityBasis,
    EmptyCandidateId,
    DuplicateCandidate(String),
    ScenarioCoverageMismatch(String),
    DuplicateCandidateScenario { candidate: String, scenario: String },
    InvalidOutcome { candidate: String, scenario: String },
    NoFeasibleCandidateInScenario(String),
}

impl std::fmt::Display for ClimateRobustnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyScenarioSet => f.write_str("at least one climate scenario is required"),
            Self::EmptyCandidateSet => f.write_str("at least one climate candidate is required"),
            Self::InvalidScenarioId => f.write_str("scenario ID must not be empty or contain control characters"),
            Self::DuplicateScenario(id) => write!(f, "duplicate climate scenario ID: {id}"),
            Self::InvalidScenarioWeight(id) => write!(f, "scenario decision weight is invalid: {id}"),
            Self::ScenarioWeightsDoNotSumToOne => f.write_str("scenario decision weights must sum to one"),
            Self::MissingProbabilityBasis => f.write_str("probability weights require a non-empty provenance/basis ID"),
            Self::EmptyCandidateId => f.write_str("candidate ID must not be empty or contain control characters"),
            Self::DuplicateCandidate(id) => write!(f, "duplicate climate candidate ID: {id}"),
            Self::ScenarioCoverageMismatch(id) => write!(f, "candidate scenario coverage does not match the scenario set: {id}"),
            Self::DuplicateCandidateScenario { candidate, scenario } => {
                write!(f, "candidate {candidate} has duplicate result for scenario {scenario}")
            }
            Self::InvalidOutcome { candidate, scenario } => {
                write!(f, "candidate {candidate} has invalid outcome vector in scenario {scenario}")
            }
            Self::NoFeasibleCandidateInScenario(id) => {
                write!(f, "no candidate satisfies hard constraints in scenario {id}")
            }
        }
    }
}

impl std::error::Error for ClimateRobustnessError {}

fn dominates(
    left: &[f64; CLIMATE_CRITERION_COUNT],
    right: &[f64; CLIMATE_CRITERION_COUNT],
) -> bool {
    let no_worse = left.iter().zip(right.iter()).all(|(a, b)| a >= b);
    let strictly_better = left.iter().zip(right.iter()).any(|(a, b)| a > b);
    no_worse && strictly_better
}

fn frontier_ids<'a, I>(candidates: I) -> Vec<String>
where
    I: IntoIterator<Item = (&'a str, &'a [f64; CLIMATE_CRITERION_COUNT])>,
{
    let candidates: Vec<_> = candidates.into_iter().collect();
    let mut frontier = Vec::new();

    for (candidate_id, candidate_outcomes) in &candidates {
        let dominated = candidates.iter().any(|(other_id, other_outcomes)| {
            candidate_id != other_id && dominates(*other_outcomes, *candidate_outcomes)
        });
        if !dominated {
            frontier.push((*candidate_id).to_owned());
        }
    }

    frontier.sort();
    frontier
}

fn mean_pairwise_jaccard(frontiers: &[ScenarioFrontierReport]) -> f64 {
    if frontiers.len() <= 1 {
        return 1.0;
    }

    let mut total = 0.0;
    let mut comparisons = 0_usize;
    for left in 0..frontiers.len() {
        for right in (left + 1)..frontiers.len() {
            let a = &frontiers[left].frontier_candidate_ids;
            let b = &frontiers[right].frontier_candidate_ids;
            let intersection = a.iter().filter(|id| b.contains(id)).count();
            let union = a.len() + b.len() - intersection;
            let jaccard = if union == 0 {
                1.0
            } else {
                intersection as f64 / union as f64
            };
            total += jaccard;
            comparisons += 1;
        }
    }

    total / comparisons as f64
}

impl ClimateScenarioSet {
    pub fn validate(&self) -> Result<(), ClimateRobustnessError> {
        if self.scenarios.is_empty() {
            return Err(ClimateRobustnessError::EmptyScenarioSet);
        }
        if matches!(
            &self.semantics,
            ScenarioWeightSemantics::ProbabilityWeights { basis_id } if basis_id.trim().is_empty()
        ) {
            return Err(ClimateRobustnessError::MissingProbabilityBasis);
        }

        let mut ids = std::collections::BTreeSet::new();
        let mut weight_sum = 0.0_f64;

        for scenario in &self.scenarios {
            if scenario.scenario_id.trim().is_empty()
                || scenario.scenario_id.chars().any(char::is_control)
            {
                return Err(ClimateRobustnessError::InvalidScenarioId);
            }
            if !ids.insert(scenario.scenario_id.clone()) {
                return Err(ClimateRobustnessError::DuplicateScenario(
                    scenario.scenario_id.clone(),
                ));
            }
            if !scenario.decision_weight.is_finite() || scenario.decision_weight < 0.0 {
                return Err(ClimateRobustnessError::InvalidScenarioWeight(
                    scenario.scenario_id.clone(),
                ));
            }
            weight_sum += scenario.decision_weight;
        }

        if (weight_sum - 1.0).abs() > 1.0e-9 {
            return Err(ClimateRobustnessError::ScenarioWeightsDoNotSumToOne);
        }

        Ok(())
    }
}

pub fn evaluate_climate_robustness(
    scenario_set: &ClimateScenarioSet,
    candidates: &[ClimateCandidate],
) -> Result<ClimateRobustnessReport, ClimateRobustnessError> {
    scenario_set.validate()?;

    if candidates.is_empty() {
        return Err(ClimateRobustnessError::EmptyCandidateSet);
    }

    let scenario_ids: std::collections::BTreeSet<String> = scenario_set
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.clone())
        .collect();

    let mut candidate_ids = std::collections::BTreeSet::new();
    let mut outcomes_by_candidate: std::collections::BTreeMap<
        String,
        std::collections::BTreeMap<String, &ClimateScenarioOutcome>,
    > = std::collections::BTreeMap::new();

    for candidate in candidates {
        if candidate.candidate_id.trim().is_empty()
            || candidate.candidate_id.chars().any(char::is_control)
        {
            return Err(ClimateRobustnessError::EmptyCandidateId);
        }
        if !candidate_ids.insert(candidate.candidate_id.clone()) {
            return Err(ClimateRobustnessError::DuplicateCandidate(
                candidate.candidate_id.clone(),
            ));
        }
        if candidate.scenario_outcomes.len() != scenario_set.scenarios.len() {
            return Err(ClimateRobustnessError::ScenarioCoverageMismatch(
                candidate.candidate_id.clone(),
            ));
        }

        let mut scenario_map = std::collections::BTreeMap::new();
        for result in &candidate.scenario_outcomes {
            if !scenario_ids.contains(&result.scenario_id) {
                return Err(ClimateRobustnessError::ScenarioCoverageMismatch(
                    candidate.candidate_id.clone(),
                ));
            }
            if scenario_map
                .insert(result.scenario_id.clone(), result)
                .is_some()
            {
                return Err(ClimateRobustnessError::DuplicateCandidateScenario {
                    candidate: candidate.candidate_id.clone(),
                    scenario: result.scenario_id.clone(),
                });
            }
            if result
                .outcomes
                .iter()
                .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
            {
                return Err(ClimateRobustnessError::InvalidOutcome {
                    candidate: candidate.candidate_id.clone(),
                    scenario: result.scenario_id.clone(),
                });
            }
        }

        if scenario_map.len() != scenario_set.scenarios.len() {
            return Err(ClimateRobustnessError::ScenarioCoverageMismatch(
                candidate.candidate_id.clone(),
            ));
        }

        outcomes_by_candidate.insert(candidate.candidate_id.clone(), scenario_map);
    }

    let mut scenario_frontiers = Vec::with_capacity(scenario_set.scenarios.len());
    let mut scenario_ideal_by_id = std::collections::BTreeMap::new();

    for scenario in &scenario_set.scenarios {
        let feasible: Vec<_> = candidates
            .iter()
            .filter_map(|candidate| {
                let result = outcomes_by_candidate
                    .get(&candidate.candidate_id)
                    .and_then(|results| results.get(&scenario.scenario_id))?;
                result
                    .feasible
                    .then_some((candidate.candidate_id.as_str(), &result.outcomes))
            })
            .collect();

        if feasible.is_empty() {
            return Err(ClimateRobustnessError::NoFeasibleCandidateInScenario(
                scenario.scenario_id.clone(),
            ));
        }

        let mut ideal = [0.0_f64; CLIMATE_CRITERION_COUNT];
        for (_, outcomes) in &feasible {
            for index in 0..CLIMATE_CRITERION_COUNT {
                ideal[index] = ideal[index].max(outcomes[index]);
            }
        }

        let frontier_candidate_ids = frontier_ids(feasible.iter().copied());
        scenario_ideal_by_id.insert(scenario.scenario_id.clone(), ideal);
        scenario_frontiers.push(ScenarioFrontierReport {
            scenario_id: scenario.scenario_id.clone(),
            feasible_candidate_count: feasible.len(),
            frontier_candidate_ids,
            ideal_outcomes: ideal,
        });
    }

    let mut union_frontier = std::collections::BTreeSet::new();
    let mut universal_frontier: Option<std::collections::BTreeSet<String>> = None;
    for frontier in &scenario_frontiers {
        let current: std::collections::BTreeSet<_> =
            frontier.frontier_candidate_ids.iter().cloned().collect();
        union_frontier.extend(current.iter().cloned());
        universal_frontier = Some(match universal_frontier {
            None => current,
            Some(previous) => previous.intersection(&current).cloned().collect(),
        });
    }
    let universal_frontier = universal_frontier.unwrap_or_default();

    let mut candidate_reports = Vec::with_capacity(candidates.len());
    let mut robust_worst_case_rows: Vec<(String, [f64; CLIMATE_CRITERION_COUNT])> =
        Vec::new();

    for candidate in candidates {
        let mut weighted_mean = [0.0_f64; CLIMATE_CRITERION_COUNT];
        let mut worst_case = [1.0_f64; CLIMATE_CRITERION_COUNT];
        let mut weighted_feasibility_coverage = 0.0_f64;
        let mut guaranteed_feasible = true;
        let mut weighted_regret = [0.0_f64; CLIMATE_CRITERION_COUNT];
        let mut maximum_regret = 0.0_f64;
        let mut scenario_frontier_membership_count = 0_usize;

        for scenario in &scenario_set.scenarios {
            let result = outcomes_by_candidate
                .get(&candidate.candidate_id)
                .and_then(|results| results.get(&scenario.scenario_id))
                .expect("validated candidate covers every scenario");

            if result.feasible {
                weighted_feasibility_coverage += scenario.decision_weight;
            } else {
                guaranteed_feasible = false;
            }

            for index in 0..CLIMATE_CRITERION_COUNT {
                weighted_mean[index] += scenario.decision_weight * result.outcomes[index];
                worst_case[index] = worst_case[index].min(result.outcomes[index]);

                let ideal = scenario_ideal_by_id
                    .get(&scenario.scenario_id)
                    .expect("scenario ideal was calculated")[index];
                let regret = (ideal - result.outcomes[index]).max(0.0);
                weighted_regret[index] += scenario.decision_weight * regret;
                maximum_regret = maximum_regret.max(regret);
            }

            if scenario_frontiers.iter().any(|frontier| {
                frontier.scenario_id == scenario.scenario_id
                    && frontier.frontier_candidate_ids.contains(&candidate.candidate_id)
            }) {
                scenario_frontier_membership_count += 1;
            }
        }

        if guaranteed_feasible {
            robust_worst_case_rows.push((candidate.candidate_id.clone(), worst_case));
        }

        candidate_reports.push(CandidateRobustnessReport {
            candidate_id: candidate.candidate_id.clone(),
            weighted_mean_outcomes: weighted_mean,
            worst_case_outcomes: worst_case,
            weighted_feasibility_coverage,
            guaranteed_feasible,
            scenario_frontier_membership_count,
            universal_pareto_member: universal_frontier.contains(&candidate.candidate_id),
            weighted_regret_by_criterion: weighted_regret,
            maximum_regret,
        });
    }

    let robust_frontier_candidate_ids = frontier_ids(
        robust_worst_case_rows
            .iter()
            .map(|(candidate_id, outcomes)| (candidate_id.as_str(), outcomes)),
    );

    Ok(ClimateRobustnessReport {
        schema_version: CLIMATE_ROBUSTNESS_SCHEMA_VERSION,
        weight_semantics: scenario_set.semantics.clone(),
        scenario_count: scenario_set.scenarios.len(),
        candidate_count: candidates.len(),
        scenario_frontiers,
        union_frontier_candidate_ids: union_frontier.into_iter().collect(),
        universal_pareto_candidate_ids: universal_frontier.into_iter().collect(),
        robust_frontier_candidate_ids,
        mean_pairwise_frontier_jaccard: mean_pairwise_jaccard(&scenario_frontiers),
        candidate_reports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario_set() -> ClimateScenarioSet {
        ClimateScenarioSet {
            semantics: ScenarioWeightSemantics::AnalyticalDecisionWeights,
            scenarios: vec![
                ClimateScenario {
                    scenario_id: "present_tmy".to_owned(),
                    decision_weight: 0.5,
                },
                ClimateScenario {
                    scenario_id: "future_hot".to_owned(),
                    decision_weight: 0.3,
                },
                ClimateScenario {
                    scenario_id: "compound_heat_grid".to_owned(),
                    decision_weight: 0.2,
                },
            ],
        }
    }

    fn candidate(
        id: &str,
        present: [f64; CLIMATE_CRITERION_COUNT],
        future: [f64; CLIMATE_CRITERION_COUNT],
        compound: [f64; CLIMATE_CRITERION_COUNT],
        feasibility: [bool; 3],
    ) -> ClimateCandidate {
        ClimateCandidate {
            candidate_id: id.to_owned(),
            scenario_outcomes: vec![
                ClimateScenarioOutcome {
                    scenario_id: "present_tmy".to_owned(),
                    outcomes: present,
                    feasible: feasibility[0],
                },
                ClimateScenarioOutcome {
                    scenario_id: "future_hot".to_owned(),
                    outcomes: future,
                    feasible: feasibility[1],
                },
                ClimateScenarioOutcome {
                    scenario_id: "compound_heat_grid".to_owned(),
                    outcomes: compound,
                    feasible: feasibility[2],
                },
            ],
        }
    }

    fn fixture_candidates() -> Vec<ClimateCandidate> {
        vec![
            candidate(
                "present_specialist",
                [0.96, 0.91, 0.72],
                [0.58, 0.52, 0.68],
                [0.42, 0.36, 0.61],
                [true, false, false],
            ),
            candidate(
                "balanced",
                [0.80, 0.78, 0.80],
                [0.76, 0.73, 0.78],
                [0.70, 0.66, 0.73],
                [true, true, true],
            ),
            candidate(
                "future_specialist",
                [0.69, 0.72, 0.66],
                [0.94, 0.88, 0.72],
                [0.88, 0.82, 0.66],
                [true, true, true],
            ),
            candidate(
                "low_cost_brittle",
                [0.76, 0.62, 0.98],
                [0.34, 0.30, 0.94],
                [0.24, 0.20, 0.90],
                [true, true, true],
            ),
            candidate(
                "stable_moderate",
                [0.76, 0.75, 0.74],
                [0.74, 0.72, 0.72],
                [0.72, 0.70, 0.71],
                [true, true, true],
            ),
        ]
    }

    #[test]
    fn present_day_specialist_is_not_mislabelled_as_guaranteed_feasible() {
        let report =
            evaluate_climate_robustness(&scenario_set(), &fixture_candidates()).unwrap();
        let specialist = report
            .candidate_reports
            .iter()
            .find(|row| row.candidate_id == "present_specialist")
            .unwrap();

        assert!(!specialist.guaranteed_feasible);
        assert!((specialist.weighted_feasibility_coverage - 0.5).abs() < 1.0e-12);
        assert!(!report
            .robust_frontier_candidate_ids
            .contains(&"present_specialist".to_owned()));
    }

    #[test]
    fn robust_frontier_is_restricted_to_universally_feasible_candidates() {
        let report =
            evaluate_climate_robustness(&scenario_set(), &fixture_candidates()).unwrap();

        for candidate_id in &report.robust_frontier_candidate_ids {
            let row = report
                .candidate_reports
                .iter()
                .find(|row| &row.candidate_id == candidate_id)
                .unwrap();
            assert!(row.guaranteed_feasible);
        }

        let balanced = report
            .candidate_reports
            .iter()
            .find(|row| row.candidate_id == "balanced")
            .unwrap();
        assert!(balanced.guaranteed_feasible);
        assert!(balanced.maximum_regret.is_finite());
        assert!(balanced
            .weighted_regret_by_criterion
            .iter()
            .all(|value| value.is_finite()));
    }

    #[test]
    fn analytical_decision_weights_are_not_reported_as_probabilities() {
        let report =
            evaluate_climate_robustness(&scenario_set(), &fixture_candidates()).unwrap();
        assert_eq!(
            report.weight_semantics.as_str(),
            "analytical_decision_weights"
        );
        assert!(report.weight_semantics.probability_basis().is_none());
    }

    #[test]
    fn probability_weights_require_explicit_provenance() {
        let mut scenarios = scenario_set();
        scenarios.semantics = ScenarioWeightSemantics::ProbabilityWeights {
            basis_id: " ".to_owned(),
        };
        assert_eq!(
            scenarios.validate(),
            Err(ClimateRobustnessError::MissingProbabilityBasis)
        );
    }

    #[test]
    fn scenario_frontier_and_robust_frontier_match_independent_oracle() {
        let scenarios = scenario_set();
        let candidates = fixture_candidates();
        let report = evaluate_climate_robustness(&scenarios, &candidates).unwrap();

        // Independent direct finite oracle for each scenario.
        for scenario in &scenarios.scenarios {
            let feasible: Vec<_> = candidates
                .iter()
                .filter_map(|candidate| {
                    let row = candidate
                        .scenario_outcomes
                        .iter()
                        .find(|row| row.scenario_id == scenario.scenario_id)?;
                    row.feasible.then_some((candidate, row))
                })
                .collect();

            let mut oracle = Vec::new();
            for (candidate, row) in &feasible {
                let dominated = feasible.iter().any(|(other_candidate, other_row)| {
                    other_candidate.candidate_id != candidate.candidate_id
                        && (0..CLIMATE_CRITERION_COUNT).all(|index| {
                            other_row.outcomes[index] >= row.outcomes[index]
                        })
                        && (0..CLIMATE_CRITERION_COUNT).any(|index| {
                            other_row.outcomes[index] > row.outcomes[index]
                        })
                });
                if !dominated {
                    oracle.push(candidate.candidate_id.clone());
                }
            }
            oracle.sort();

            let actual = report
                .scenario_frontiers
                .iter()
                .find(|frontier| frontier.scenario_id == scenario.scenario_id)
                .unwrap();
            assert_eq!(actual.frontier_candidate_ids, oracle);
        }

        // Independent robust-frontier oracle over worst-case rows and the
        // intersection of hard feasibility across scenarios.
        let universally_feasible: Vec<_> = candidates
            .iter()
            .filter_map(|candidate| {
                let mut worst = [1.0_f64; CLIMATE_CRITERION_COUNT];
                for scenario in &scenarios.scenarios {
                    let row = candidate
                        .scenario_outcomes
                        .iter()
                        .find(|row| row.scenario_id == scenario.scenario_id)?;
                    if !row.feasible {
                        return None;
                    }
                    for index in 0..CLIMATE_CRITERION_COUNT {
                        worst[index] = worst[index].min(row.outcomes[index]);
                    }
                }
                Some((candidate, worst))
            })
            .collect();

        let mut oracle_robust = Vec::new();
        for (candidate, outcomes) in &universally_feasible {
            let dominated = universally_feasible
                .iter()
                .any(|(other_candidate, other_outcomes)| {
                    other_candidate.candidate_id != candidate.candidate_id
                        && (0..CLIMATE_CRITERION_COUNT)
                            .all(|index| other_outcomes[index] >= outcomes[index])
                        && (0..CLIMATE_CRITERION_COUNT)
                            .any(|index| other_outcomes[index] > outcomes[index])
                });
            if !dominated {
                oracle_robust.push(candidate.candidate_id.clone());
            }
        }
        oracle_robust.sort();
        assert_eq!(report.robust_frontier_candidate_ids, oracle_robust);
    }

    #[test]
    fn invalid_scenario_weights_and_incomplete_candidate_coverage_fail_closed() {
        let mut invalid_weights = scenario_set();
        invalid_weights.scenarios[0].decision_weight = 0.9;
        assert_eq!(
            evaluate_climate_robustness(&invalid_weights, &fixture_candidates()),
            Err(ClimateRobustnessError::ScenarioWeightsDoNotSumToOne)
        );

        let mut incomplete = fixture_candidates();
        incomplete[0].scenario_outcomes.pop();
        assert!(matches!(
            evaluate_climate_robustness(&scenario_set(), &incomplete),
            Err(ClimateRobustnessError::ScenarioCoverageMismatch(_))
        ));
    }

    #[test]
    fn every_scenario_requires_at_least_one_hard_feasible_candidate() {
        let mut candidates = fixture_candidates();
        for candidate in &mut candidates {
            candidate.scenario_outcomes[2].feasible = false;
        }
        assert_eq!(
            evaluate_climate_robustness(&scenario_set(), &candidates),
            Err(ClimateRobustnessError::NoFeasibleCandidateInScenario(
                "compound_heat_grid".to_owned()
            ))
        );
    }
}
