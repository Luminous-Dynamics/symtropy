//! Climate scenario experiment selection with explicit probability provenance.
//!
//! Analytical scenario weights are decision priorities, not probabilities.
//! Expected value of information requires a separately justified probability
//! basis and an explicit decision objective/authority.

use crate::climate_robustness::{
    ClimateRobustnessError, ClimateScenarioSet, ScenarioWeightSemantics,
};
use crate::experiment_selection::{
    DecisionProblem, ExperimentAction, ExperimentMetrics, ExperimentSelectionPolicy,
    PolicyOutcome, DesignDecision, VoiError,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ClimateExperimentSelectionRequest {
    pub scenarios: ClimateScenarioSet,
    /// Explicit decision objective and authority; never inferred by this adapter.
    pub decision_objective_id: String,
    pub decision_authority_id: String,
    pub decisions: Vec<DesignDecision>,
    pub experiments: Vec<ExperimentAction>,
    pub experiment_budget: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClimateExperimentSelectionReport {
    pub probability_basis_id: String,
    pub decision_objective_id: String,
    pub decision_authority_id: String,
    pub policy_outcome: PolicyOutcome,
    pub experiment_metrics: Vec<ExperimentMetrics>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClimateVoiError {
    InvalidScenarioSet(ClimateRobustnessError),
    AnalyticalWeightsCannotBeUsedAsProbabilities,
    MissingProbabilityBasis,
    MissingDecisionObjectiveId,
    MissingDecisionAuthorityId,
    InvalidDecisionProblem(VoiError),
}

impl std::fmt::Display for ClimateVoiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidScenarioSet(error) => {
                write!(f, "invalid climate scenario set: {error}")
            }
            Self::AnalyticalWeightsCannotBeUsedAsProbabilities => f.write_str(
                "analytical climate scenario weights cannot be used as probability priors for expected value of information",
            ),
            Self::MissingProbabilityBasis => {
                f.write_str("climate probability priors require a non-empty independent basis ID")
            }
            Self::MissingDecisionObjectiveId => {
                f.write_str("climate experiment selection requires an explicit decision-objective ID")
            }
            Self::MissingDecisionAuthorityId => {
                f.write_str("climate experiment selection requires an explicit decision-authority ID")
            }
            Self::InvalidDecisionProblem(error) => {
                write!(f, "invalid climate decision/experiment model: {error}")
            }
        }
    }
}

impl std::error::Error for ClimateVoiError {}

impl From<VoiError> for ClimateVoiError {
    fn from(value: VoiError) -> Self {
        Self::InvalidDecisionProblem(value)
    }
}

impl ClimateExperimentSelectionRequest {
    fn to_decision_problem(&self) -> Result<(String, DecisionProblem), ClimateVoiError> {
        self.scenarios
            .validate()
            .map_err(ClimateVoiError::InvalidScenarioSet)?;

        let probability_basis_id = match &self.scenarios.semantics {
            ScenarioWeightSemantics::AnalyticalDecisionWeights => {
                return Err(ClimateVoiError::AnalyticalWeightsCannotBeUsedAsProbabilities);
            }
            ScenarioWeightSemantics::ProbabilityWeights { basis_id } => {
                if basis_id.trim().is_empty() {
                    return Err(ClimateVoiError::MissingProbabilityBasis);
                }
                basis_id.clone()
            }
        };

        if self.decision_objective_id.trim().is_empty() {
            return Err(ClimateVoiError::MissingDecisionObjectiveId);
        }
        if self.decision_authority_id.trim().is_empty() {
            return Err(ClimateVoiError::MissingDecisionAuthorityId);
        }

        let problem = DecisionProblem {
            world_state_ids: self
                .scenarios
                .scenarios
                .iter()
                .map(|scenario| scenario.scenario_id.clone())
                .collect(),
            prior: self
                .scenarios
                .scenarios
                .iter()
                .map(|scenario| scenario.decision_weight)
                .collect(),
            decisions: self.decisions.clone(),
            experiments: self.experiments.clone(),
            experiment_budget: self.experiment_budget,
        };
        problem
            .validate()
            .map_err(ClimateVoiError::InvalidDecisionProblem)?;

        Ok((probability_basis_id, problem))
    }

    pub fn evaluate_policy(
        &self,
        policy: ExperimentSelectionPolicy,
        seed: u64,
    ) -> Result<ClimateExperimentSelectionReport, ClimateVoiError> {
        let (probability_basis_id, problem) = self.to_decision_problem()?;
        let policy_outcome = problem
            .run_policy(policy, seed)
            .map_err(ClimateVoiError::InvalidDecisionProblem)?;
        let experiment_metrics = (0..problem.experiments.len())
            .map(|index| {
                problem
                    .evaluate_experiment(index)
                    .map_err(ClimateVoiError::InvalidDecisionProblem)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ClimateExperimentSelectionReport {
            probability_basis_id,
            decision_objective_id: self.decision_objective_id.clone(),
            decision_authority_id: self.decision_authority_id.clone(),
            policy_outcome,
            experiment_metrics,
        })
    }
}

/// Four-state climate decision fixture:
/// - hot/moderate exposure is decision-relevant;
/// - dry/wet state is irrelevant to the declared design utilities.
///
/// The exact humidity test has more mutual information (1 bit) than the
/// imperfect heat-exposure test, but only the latter improves the decision.
pub fn synthetic_climate_voi_request() -> ClimateExperimentSelectionRequest {
    let scenarios = ClimateScenarioSet {
        semantics: ScenarioWeightSemantics::ProbabilityWeights {
            basis_id: "synthetic-equal-prior-demonstration-v1".to_owned(),
        },
        scenarios: vec![
            crate::climate_robustness::ClimateScenario {
                scenario_id: "moderate_dry".to_owned(),
                decision_weight: 0.25,
            },
            crate::climate_robustness::ClimateScenario {
                scenario_id: "moderate_wet".to_owned(),
                decision_weight: 0.25,
            },
            crate::climate_robustness::ClimateScenario {
                scenario_id: "hot_dry".to_owned(),
                decision_weight: 0.25,
            },
            crate::climate_robustness::ClimateScenario {
                scenario_id: "hot_wet".to_owned(),
                decision_weight: 0.25,
            },
        ],
    };

    let decisions = vec![
        DesignDecision {
            decision_id: "passive_shading".to_owned(),
            utility_by_world: vec![0.65, 0.65, 0.95, 0.95],
        },
        DesignDecision {
            decision_id: "active_cooling".to_owned(),
            utility_by_world: vec![0.90, 0.90, 0.55, 0.55],
        },
    ];

    let experiments = vec![
        ExperimentAction {
            experiment_id: "measure_heat_exposure_70pct".to_owned(),
            cost: 0.01,
            likelihoods: vec![
                vec![0.7, 0.3],
                vec![0.7, 0.3],
                vec![0.3, 0.7],
                vec![0.3, 0.7],
            ],
        },
        ExperimentAction {
            experiment_id: "measure_humidity_exact".to_owned(),
            cost: 0.01,
            likelihoods: vec![
                vec![1.0, 0.0],
                vec![0.0, 1.0],
                vec![1.0, 0.0],
                vec![0.0, 1.0],
            ],
        },
    ];

    ClimateExperimentSelectionRequest {
        scenarios,
        decision_objective_id: "synthetic_thermal_safety_tradeoff_v1".to_owned(),
        decision_authority_id: "synthetic_design_review_panel_v1".to_owned(),
        decisions,
        experiments,
        experiment_budget: 0.02,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::climate_robustness::ClimateRobustnessError;

    #[test]
    fn analytical_scenario_weights_cannot_be_used_as_voi_priors() {
        let mut request = synthetic_climate_voi_request();
        request.scenarios.semantics = ScenarioWeightSemantics::AnalyticalDecisionWeights;

        assert_eq!(
            request.evaluate_policy(ExperimentSelectionPolicy::ExpectedDecisionValue, 7),
            Err(ClimateVoiError::AnalyticalWeightsCannotBeUsedAsProbabilities)
        );
    }

    #[test]
    fn higher_information_measurement_can_be_decision_irrelevant() {
        let request = synthetic_climate_voi_request();
        let information = request
            .evaluate_policy(ExperimentSelectionPolicy::MaximumInformationGain, 7)
            .unwrap();
        let decision_value = request
            .evaluate_policy(ExperimentSelectionPolicy::ExpectedDecisionValue, 7)
            .unwrap();

        assert_eq!(
            information
                .policy_outcome
                .selected_experiment_id
                .as_deref(),
            Some("measure_humidity_exact")
        );
        assert_eq!(
            decision_value
                .policy_outcome
                .selected_experiment_id
                .as_deref(),
            Some("measure_heat_exposure_70pct")
        );
        assert!(
            information.policy_outcome.mutual_information_bits
                > decision_value.policy_outcome.mutual_information_bits
        );
        assert!(
            decision_value.policy_outcome.net_expected_decision_value
                > information.policy_outcome.net_expected_decision_value
        );
        assert!(
            decision_value.policy_outcome.expected_decision_regret
                < information.policy_outcome.expected_decision_regret
        );

        let heat = decision_value
            .experiment_metrics
            .iter()
            .find(|metric| metric.experiment_id == "measure_heat_exposure_70pct")
            .unwrap();
        let humidity = decision_value
            .experiment_metrics
            .iter()
            .find(|metric| metric.experiment_id == "measure_humidity_exact")
            .unwrap();
        assert!((heat.expected_value_improvement - 0.0275).abs() < 1.0e-12);
        assert!((heat.net_expected_decision_value - 0.0175).abs() < 1.0e-12);
        assert!((heat.expected_decision_regret - 0.0975).abs() < 1.0e-12);
        assert!((humidity.expected_value_improvement - 0.0).abs() < 1.0e-12);
        assert!((humidity.net_expected_decision_value + 0.01).abs() < 1.0e-12);
        assert!((humidity.expected_decision_regret - 0.125).abs() < 1.0e-12);
        assert!((humidity.mutual_information_bits - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn report_binds_probability_basis_objective_and_authority() {
        let request = synthetic_climate_voi_request();
        let report = request
            .evaluate_policy(ExperimentSelectionPolicy::ExpectedDecisionValue, 17)
            .unwrap();

        assert_eq!(
            report.probability_basis_id,
            "synthetic-equal-prior-demonstration-v1"
        );
        assert_eq!(
            report.decision_objective_id,
            "synthetic_thermal_safety_tradeoff_v1"
        );
        assert_eq!(
            report.decision_authority_id,
            "synthetic_design_review_panel_v1"
        );
        assert_eq!(
            report.policy_outcome.selected_experiment_id.as_deref(),
            Some("measure_heat_exposure_70pct")
        );
        assert_eq!(report.experiment_metrics.len(), 2);
    }

    #[test]
    fn missing_objective_or_authority_fails_closed() {
        let mut request = synthetic_climate_voi_request();
        request.decision_objective_id.clear();
        assert_eq!(
            request.evaluate_policy(ExperimentSelectionPolicy::ExpectedDecisionValue, 1),
            Err(ClimateVoiError::MissingDecisionObjectiveId)
        );

        let mut request = synthetic_climate_voi_request();
        request.decision_authority_id.clear();
        assert_eq!(
            request.evaluate_policy(ExperimentSelectionPolicy::ExpectedDecisionValue, 1),
            Err(ClimateVoiError::MissingDecisionAuthorityId)
        );
    }

    #[test]
    fn malformed_scenario_probability_basis_is_rejected() {
        let mut request = synthetic_climate_voi_request();
        request.scenarios.semantics = ScenarioWeightSemantics::ProbabilityWeights {
            basis_id: " ".to_owned(),
        };
        assert!(matches!(
            request.evaluate_policy(ExperimentSelectionPolicy::ExpectedDecisionValue, 1),
            Err(ClimateVoiError::InvalidScenarioSet(
                ClimateRobustnessError::MissingProbabilityBasis
            ))
        ));
    }
}
