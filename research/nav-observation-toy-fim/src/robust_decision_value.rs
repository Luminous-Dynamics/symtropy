//! Finite-scenario decision-value qualification under model uncertainty.
//!
//! This module evaluates one physical sensing action under multiple frozen
//! observation/acquisition models. It reports nominal mean value, worst-case
//! scenario value, downside variability, an explicit risk premium and an
//! ignorance penalty as separate terms.
//!
//! The finite scenario set is not a continuous uncertainty certificate.
//! Model confidence must come from an independent calibration/monitoring
//! process; it is never inferred from evaluator-only truth.

use std::collections::BTreeSet;
use std::fmt;

use crate::decision_value::{
    evaluate_action_with_models, AcquisitionSuccessModel, ActionCost, BinaryBelief,
    DecisionLoss, DecisionValueError, DiagnosticAction, ResourceBudget,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ActionModelScenario {
    pub scenario_id: String,
    pub action: DiagnosticAction,
    pub acquisition: AcquisitionSuccessModel,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioDecisionValue {
    pub scenario_id: String,
    pub expected_decision_value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RobustValueAssessment {
    pub action_id: String,
    pub cost: ActionCost,
    pub feasible: bool,
    pub scenario_values: Vec<ScenarioDecisionValue>,
    pub mean_decision_value: f64,
    pub minimum_model_value: f64,
    pub downside_deviation: f64,
    pub risk_aversion: f64,
    pub risk_premium: f64,
    pub model_confidence: f64,
    pub ignorance_scale: f64,
    pub ignorance_penalty: f64,
    pub risk_adjusted_decision_value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RobustValueError {
    EmptyScenarioSet,
    EmptyScenarioId,
    DuplicateScenarioId(String),
    EmptyActionId,
    MixedActionIds,
    InconsistentActionCosts,
    InvalidRiskAversion,
    InvalidModelConfidence,
    InvalidIgnoranceScale,
    Evaluation(DecisionValueError),
}

impl fmt::Display for RobustValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyScenarioSet => write!(f, "at least one model scenario is required"),
            Self::EmptyScenarioId => write!(f, "model scenario ID must not be empty"),
            Self::DuplicateScenarioId(id) => write!(f, "duplicate model scenario ID: {id}"),
            Self::EmptyActionId => write!(f, "action ID must not be empty"),
            Self::MixedActionIds => write!(f, "scenarios do not describe the same action ID"),
            Self::InconsistentActionCosts => {
                write!(f, "one physical action has inconsistent costs across scenarios")
            }
            Self::InvalidRiskAversion => {
                write!(f, "risk aversion must be finite and nonnegative")
            }
            Self::InvalidModelConfidence => {
                write!(f, "model confidence must be finite and within [0, 1]")
            }
            Self::InvalidIgnoranceScale => {
                write!(f, "ignorance scale must be finite and nonnegative")
            }
            Self::Evaluation(error) => write!(f, "scenario evaluation failed: {error}"),
        }
    }
}
impl std::error::Error for RobustValueError {}
impl From<DecisionValueError> for RobustValueError {
    fn from(value: DecisionValueError) -> Self {
        Self::Evaluation(value)
    }
}

/// Evaluate one action against a frozen finite family of plausible models.
///
/// The scenario mean is an equally weighted descriptive average, not a
/// calibrated Bayesian model average. The downside deviation is the
/// population RMS shortfall below that mean:
///
///     downside = sqrt(mean(max(mean_value - scenario_value, 0)^2))
///
///     risk_premium = risk_aversion * downside
///
///     ignorance_penalty = (1 - model_confidence)
///                         * ignorance_scale * baseline_bayes_risk
///
///     adjusted_value = mean_value - risk_premium - ignorance_penalty
///
/// Keep all terms visible. Do not tune the model family or confidence against
/// evaluator-only outcomes.
pub fn assess_action_under_model_uncertainty(
    scenarios: &[ActionModelScenario],
    belief: BinaryBelief,
    budget: ResourceBudget,
    loss: DecisionLoss,
    risk_aversion: f64,
    model_confidence: f64,
    ignorance_scale: f64,
) -> Result<RobustValueAssessment, RobustValueError> {
    if scenarios.is_empty() {
        return Err(RobustValueError::EmptyScenarioSet);
    }
    if !risk_aversion.is_finite() || risk_aversion < 0.0 {
        return Err(RobustValueError::InvalidRiskAversion);
    }
    if !model_confidence.is_finite() || !(0.0..=1.0).contains(&model_confidence) {
        return Err(RobustValueError::InvalidModelConfidence);
    }
    if !ignorance_scale.is_finite() || ignorance_scale < 0.0 {
        return Err(RobustValueError::InvalidIgnoranceScale);
    }

    let action_id = scenarios[0].action.id.clone();
    if action_id.trim().is_empty() {
        return Err(RobustValueError::EmptyActionId);
    }
    let cost = scenarios[0].action.cost;
    let mut scenario_ids = BTreeSet::new();
    let mut scenario_values = Vec::with_capacity(scenarios.len());

    for scenario in scenarios {
        if scenario.scenario_id.trim().is_empty() {
            return Err(RobustValueError::EmptyScenarioId);
        }
        if !scenario_ids.insert(scenario.scenario_id.clone()) {
            return Err(RobustValueError::DuplicateScenarioId(
                scenario.scenario_id.clone(),
            ));
        }
        if scenario.action.id != action_id {
            return Err(RobustValueError::MixedActionIds);
        }
        if scenario.action.cost != cost {
            return Err(RobustValueError::InconsistentActionCosts);
        }

        let assessment = evaluate_action_with_models(
            &scenario.action,
            belief,
            budget,
            loss,
            scenario.acquisition,
        )?;
        scenario_values.push(ScenarioDecisionValue {
            scenario_id: scenario.scenario_id.clone(),
            expected_decision_value: assessment.expected_decision_value,
        });
    }

    scenario_values.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    let count = scenario_values.len() as f64;
    let mean_decision_value = scenario_values
        .iter()
        .map(|scenario| scenario.expected_decision_value)
        .sum::<f64>()
        / count;
    let minimum_model_value = scenario_values
        .iter()
        .map(|scenario| scenario.expected_decision_value)
        .fold(f64::INFINITY, f64::min);
    let downside_variance = scenario_values
        .iter()
        .map(|scenario| {
            let shortfall =
                (mean_decision_value - scenario.expected_decision_value).max(0.0);
            shortfall * shortfall
        })
        .sum::<f64>()
        / count;
    let downside_deviation = downside_variance.sqrt();
    let baseline_bayes_risk = belief.bayes_risk_with_loss(loss)?;

    let risk_premium = risk_aversion * downside_deviation;
    let ignorance_penalty =
        (1.0 - model_confidence) * ignorance_scale * baseline_bayes_risk;
    let risk_adjusted_decision_value =
        mean_decision_value - risk_premium - ignorance_penalty;

    if !mean_decision_value.is_finite()
        || !minimum_model_value.is_finite()
        || !downside_deviation.is_finite()
        || !risk_premium.is_finite()
        || !ignorance_penalty.is_finite()
        || !risk_adjusted_decision_value.is_finite()
    {
        return Err(RobustValueError::Evaluation(
            DecisionValueError::NonFiniteMetric(action_id),
        ));
    }

    Ok(RobustValueAssessment {
        action_id,
        cost,
        feasible: cost.fits(budget),
        scenario_values,
        mean_decision_value,
        minimum_model_value,
        downside_deviation,
        risk_aversion,
        risk_premium,
        model_confidence,
        ignorance_scale,
        ignorance_penalty,
        risk_adjusted_decision_value,
    })
}

/// Select the action with the greatest positive risk-adjusted decision value.
///
/// Every candidate must provide the same frozen scenario-set size and unique
/// action IDs. Exact scenario IDs are compared to prevent one action from
/// benefiting from an easier or incomplete model family.
pub fn select_robust_action(
    candidate_scenarios: &[Vec<ActionModelScenario>],
    belief: BinaryBelief,
    budget: ResourceBudget,
    loss: DecisionLoss,
    risk_aversion: f64,
    model_confidence: f64,
    ignorance_scale: f64,
) -> Result<Option<RobustValueAssessment>, RobustValueError> {
    if candidate_scenarios.is_empty() {
        return Ok(None);
    }
    let mut action_ids = BTreeSet::new();
    let mut expected_scenario_ids: Option<BTreeSet<String>> = None;
    let mut assessments = Vec::with_capacity(candidate_scenarios.len());

    for scenarios in candidate_scenarios {
        let assessment = assess_action_under_model_uncertainty(
            scenarios,
            belief,
            budget,
            loss,
            risk_aversion,
            model_confidence,
            ignorance_scale,
        )?;
        if !action_ids.insert(assessment.action_id.clone()) {
            return Err(RobustValueError::MixedActionIds);
        }
        let ids: BTreeSet<String> = assessment
            .scenario_values
            .iter()
            .map(|scenario| scenario.scenario_id.clone())
            .collect();
        if let Some(expected) = &expected_scenario_ids {
            if &ids != expected {
                return Err(RobustValueError::EmptyScenarioSet);
            }
        } else {
            expected_scenario_ids = Some(ids);
        }
        assessments.push(assessment);
    }

    Ok(assessments
        .into_iter()
        .filter(|assessment| {
            assessment.feasible && assessment.risk_adjusted_decision_value > 0.0
        })
        .max_by(|a, b| {
            a.risk_adjusted_decision_value
                .total_cmp(&b.risk_adjusted_decision_value)
                .then_with(|| {
                    b.cost
                        .energy_units
                        .cmp(&a.cost.energy_units)
                        .then_with(|| b.cost.latency_ticks.cmp(&a.cost.latency_ticks))
                        .then_with(|| b.cost.bandwidth_units.cmp(&a.cost.bandwidth_units))
                        .then_with(|| b.action_id.cmp(&a.action_id))
                })
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_value::ObservationOutcome;

    fn scenario(
        scenario_id: &str,
        action_id: &str,
        positive_state_zero: f64,
        positive_state_one: f64,
    ) -> ActionModelScenario {
        ActionModelScenario {
            scenario_id: scenario_id.to_owned(),
            action: DiagnosticAction {
                id: action_id.to_owned(),
                cost: ActionCost {
                    energy_units: 1,
                    latency_ticks: 2,
                    bandwidth_units: 1,
                },
                acquisition_success_probability: 1.0,
                outcomes: vec![
                    ObservationOutcome {
                        id: "positive".to_owned(),
                        likelihood_state_zero: positive_state_zero,
                        likelihood_state_one: positive_state_one,
                    },
                    ObservationOutcome {
                        id: "negative".to_owned(),
                        likelihood_state_zero: 1.0 - positive_state_zero,
                        likelihood_state_one: 1.0 - positive_state_one,
                    },
                ],
            },
            acquisition: AcquisitionSuccessModel::state_independent(1.0),
        }
    }

    fn common() -> (BinaryBelief, ResourceBudget, DecisionLoss) {
        (
            BinaryBelief::new(0.6).unwrap(),
            ResourceBudget {
                energy_units: 2,
                latency_ticks: 3,
                bandwidth_units: 2,
            },
            DecisionLoss::zero_one(),
        )
    }

    #[test]
    fn identical_models_have_zero_risk_premium() {
        let (belief, budget, loss) = common();
        let scenarios = vec![
            scenario("nominal-a", "anchor-a", 0.8, 0.2),
            scenario("nominal-b", "anchor-a", 0.8, 0.2),
        ];
        let result = assess_action_under_model_uncertainty(
            &scenarios, belief, budget, loss, 2.0, 1.0, 1.0,
        ).unwrap();
        assert_eq!(result.downside_deviation, 0.0);
        assert_eq!(result.risk_premium, 0.0);
        assert_eq!(result.ignorance_penalty, 0.0);
        assert!(result.risk_adjusted_decision_value > 0.0);
    }

    #[test]
    fn downside_model_disagreement_creates_an_explicit_risk_premium() {
        let (belief, budget, loss) = common();
        let scenarios = vec![
            scenario("optimistic", "anchor-a", 0.9, 0.1),
            scenario("weak", "anchor-a", 0.55, 0.45),
        ];
        let result = assess_action_under_model_uncertainty(
            &scenarios, belief, budget, loss, 1.5, 1.0, 1.0,
        ).unwrap();
        assert!(result.downside_deviation > 0.0);
        assert!(result.risk_premium > 0.0);
        assert!(result.risk_adjusted_decision_value < result.mean_decision_value);
        assert!(result.minimum_model_value <= result.mean_decision_value);
    }

    #[test]
    fn ignorance_penalty_increases_as_model_confidence_decays() {
        let (belief, budget, loss) = common();
        let scenarios = vec![
            scenario("cal-a", "anchor-a", 0.8, 0.2),
            scenario("cal-b", "anchor-a", 0.8, 0.2),
        ];
        let fresh = assess_action_under_model_uncertainty(
            &scenarios, belief, budget, loss, 1.0, 1.0, 2.0,
        ).unwrap();
        let stale = assess_action_under_model_uncertainty(
            &scenarios, belief, budget, loss, 1.0, 0.25, 2.0,
        ).unwrap();
        assert_eq!(fresh.ignorance_penalty, 0.0);
        assert!(stale.ignorance_penalty > fresh.ignorance_penalty);
        assert!(stale.risk_adjusted_decision_value < fresh.risk_adjusted_decision_value);
    }

    #[test]
    fn scenario_order_does_not_change_robust_assessment() {
        let (belief, budget, loss) = common();
        let mut scenarios = vec![
            scenario("model-b", "anchor-a", 0.9, 0.1),
            scenario("model-a", "anchor-a", 0.6, 0.4),
        ];
        let original = assess_action_under_model_uncertainty(
            &scenarios, belief, budget, loss, 1.0, 0.8, 1.0,
        ).unwrap();
        scenarios.reverse();
        let reversed = assess_action_under_model_uncertainty(
            &scenarios, belief, budget, loss, 1.0, 0.8, 1.0,
        ).unwrap();
        assert_eq!(original, reversed);
    }

    #[test]
    fn mismatched_scenario_sets_are_rejected_across_candidate_actions() {
        let (belief, budget, loss) = common();
        let candidates = vec![
            vec![
                scenario("model-a", "anchor-a", 0.8, 0.2),
                scenario("model-b", "anchor-a", 0.7, 0.3),
            ],
            vec![
                scenario("model-a", "anchor-b", 0.8, 0.2),
                scenario("model-c", "anchor-b", 0.7, 0.3),
            ],
        ];
        assert!(matches!(
            select_robust_action(&candidates, belief, budget, loss, 1.0, 0.9, 1.0),
            Err(RobustValueError::EmptyScenarioSet)
        ));
    }

    #[test]
    fn infeasible_actions_are_not_selected() {
        let (belief, _, loss) = common();
        let mut too_expensive = scenario("model-a", "anchor-a", 0.8, 0.2);
        too_expensive.action.cost.energy_units = 3;
        let result = select_robust_action(
            &[vec![too_expensive]],
            belief,
            ResourceBudget {
                energy_units: 2,
                latency_ticks: 3,
                bandwidth_units: 2,
            },
            loss,
            1.0,
            1.0,
            1.0,
        ).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn duplicate_and_missing_scenarios_fail_closed() {
        let (belief, budget, loss) = common();
        let duplicate = vec![
            scenario("model-a", "anchor-a", 0.8, 0.2),
            scenario("model-a", "anchor-a", 0.7, 0.3),
        ];
        assert!(matches!(
            assess_action_under_model_uncertainty(
                &duplicate, belief, budget, loss, 1.0, 0.9, 1.0
            ),
            Err(RobustValueError::DuplicateScenarioId(_))
        ));
        assert!(matches!(
            assess_action_under_model_uncertainty(
                &[], belief, budget, loss, 1.0, 0.9, 1.0
            ),
            Err(RobustValueError::EmptyScenarioSet)
        ));
    }
}
