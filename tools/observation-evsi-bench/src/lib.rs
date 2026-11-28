#![forbid(unsafe_code)]

//! Finite-state expected value of sample information (EVSI) scorer.
//!
//! This is a decision-theoretic benchmark primitive, not a sensor model, pose
//! estimator, physical feasibility proof, or action-authority mechanism.
//! Observation likelihoods, calibration scope, decision losses, and resource
//! costs are caller-supplied and must be independently justified.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const EVSI_SCHEMA_VERSION: u32 = 1;
const SUM_TOLERANCE: f64 = 1e-10;
const MAX_STATES: usize = 10_000;
const MAX_DECISIONS: usize = 1_000;
const MAX_ACTIONS: usize = 10_000;
const MAX_OUTCOMES_PER_ACTION: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionProblem {
    pub schema_version: u32,
    pub problem_id: String,
    pub loss_unit: String,
    pub state_ids: Vec<String>,
    pub prior: Vec<f64>,
    pub decisions: Vec<Decision>,
    pub actions: Vec<ObservationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    pub decision_id: String,
    /// Non-negative loss for each world state, aligned to state_ids.
    pub loss_by_state: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationAction {
    pub action_id: String,
    pub observation_model_sha256: String,
    pub calibration_scope_sha256: String,
    pub calibration_status: CalibrationStatus,
    pub feasibility_status: FeasibilityStatus,
    pub feasibility_receipt_sha256: String,
    pub observation_cost: f64,
    pub risk_cost: f64,
    pub outcomes: Vec<ObservationOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationStatus {
    Valid,
    OutOfScope,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FeasibilityStatus {
    Feasible,
    Infeasible,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationOutcome {
    pub outcome_id: String,
    /// Likelihood P(outcome | state, action), aligned to state_ids.
    pub likelihood_by_state: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutcomeValue {
    pub outcome_id: String,
    pub probability: f64,
    pub posterior_decision_id: Option<String>,
    pub posterior_bayes_risk: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionValue {
    pub action_id: String,
    pub eligible: bool,
    pub exclusion_reason: Option<String>,
    pub prior_bayes_risk: f64,
    pub expected_posterior_bayes_risk: Option<f64>,
    pub gross_evsi: Option<f64>,
    pub observation_cost: f64,
    pub risk_cost: f64,
    pub net_evsi: Option<f64>,
    pub mutual_information_bits: Option<f64>,
    pub prior_optimal_decision_id: String,
    pub outcome_values: Vec<OutcomeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvsiResult {
    pub schema_version: u32,
    pub problem_id: String,
    pub loss_unit: String,
    pub prior_bayes_risk: f64,
    pub prior_optimal_decision_id: String,
    pub action_values: Vec<ActionValue>,
    /// Advisory ranking only. This is not a hardware action, command or authority.
    pub top_candidate_action_id: Option<String>,
    pub actuation_authority: bool,
    pub execution_authority: bool,
}

fn nonempty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_distribution(values: &[f64], name: &str) -> Result<(), String> {
    if values.is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    if values
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(format!("{name} contains a non-finite or negative probability"));
    }
    let total = values.iter().sum::<f64>();
    if !total.is_finite() || (total - 1.0).abs() > SUM_TOLERANCE {
        return Err(format!("{name} must sum to one within {SUM_TOLERANCE}"));
    }
    Ok(())
}

pub fn validate_problem(problem: &DecisionProblem) -> Result<(), String> {
    if problem.schema_version != EVSI_SCHEMA_VERSION {
        return Err(format!("unsupported EVSI schema version {}", problem.schema_version));
    }
    nonempty(&problem.problem_id, "problem_id")?;
    nonempty(&problem.loss_unit, "loss_unit")?;
    if problem.state_ids.is_empty() || problem.state_ids.len() > MAX_STATES {
        return Err(format!("state count must be in 1..={MAX_STATES}"));
    }
    if problem.decisions.is_empty() || problem.decisions.len() > MAX_DECISIONS {
        return Err(format!("decision count must be in 1..={MAX_DECISIONS}"));
    }
    if problem.actions.is_empty() || problem.actions.len() > MAX_ACTIONS {
        return Err(format!("action count must be in 1..={MAX_ACTIONS}"));
    }
    let mut state_ids = BTreeSet::new();
    for state in &problem.state_ids {
        nonempty(state, "state_id")?;
        if !state_ids.insert(state) {
            return Err(format!("duplicate state_id {state}"));
        }
    }
    validate_distribution(&problem.prior, "prior")?;
    if problem.prior.len() != problem.state_ids.len() {
        return Err("prior length must match state_ids".to_string());
    }

    let mut decision_ids = BTreeSet::new();
    for decision in &problem.decisions {
        nonempty(&decision.decision_id, "decision_id")?;
        if !decision_ids.insert(&decision.decision_id) {
            return Err(format!("duplicate decision_id {}", decision.decision_id));
        }
        if decision.loss_by_state.len() != problem.state_ids.len()
            || decision
                .loss_by_state
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
        {
            return Err(format!(
                "decision {} loss vector must be finite, non-negative and state-aligned",
                decision.decision_id
            ));
        }
    }

    let mut action_ids = BTreeSet::new();
    for action in &problem.actions {
        nonempty(&action.action_id, "action_id")?;
        if !action_ids.insert(&action.action_id) {
            return Err(format!("duplicate action_id {}", action.action_id));
        }
        for (field, digest) in [
            ("observation_model_sha256", action.observation_model_sha256.as_str()),
            ("calibration_scope_sha256", action.calibration_scope_sha256.as_str()),
            ("feasibility_receipt_sha256", action.feasibility_receipt_sha256.as_str()),
        ] {
            if !is_sha256(digest) {
                return Err(format!(
                    "action {} {field} must be a lowercase SHA-256 digest",
                    action.action_id
                ));
            }
        }
        if !action.observation_cost.is_finite()
            || action.observation_cost < 0.0
            || !action.risk_cost.is_finite()
            || action.risk_cost < 0.0
        {
            return Err(format!("action {} has invalid costs", action.action_id));
        }
        if action.outcomes.is_empty() || action.outcomes.len() > MAX_OUTCOMES_PER_ACTION {
            return Err(format!(
                "action {} outcome count must be in 1..={MAX_OUTCOMES_PER_ACTION}",
                action.action_id
            ));
        }
        let mut outcome_ids = BTreeSet::new();
        let mut sums = vec![0.0_f64; problem.state_ids.len()];
        for outcome in &action.outcomes {
            nonempty(&outcome.outcome_id, "outcome_id")?;
            if !outcome_ids.insert(&outcome.outcome_id) {
                return Err(format!(
                    "action {} repeats outcome {}",
                    action.action_id, outcome.outcome_id
                ));
            }
            if outcome.likelihood_by_state.len() != problem.state_ids.len()
                || outcome
                    .likelihood_by_state
                    .iter()
                    .any(|value| !value.is_finite() || *value < 0.0 || *value > 1.0)
            {
                return Err(format!(
                    "action {} outcome {} has an invalid state likelihood vector",
                    action.action_id, outcome.outcome_id
                ));
            }
            for (index, likelihood) in outcome.likelihood_by_state.iter().enumerate() {
                sums[index] += likelihood;
            }
        }
        for (index, sum) in sums.iter().enumerate() {
            if !sum.is_finite() || (*sum - 1.0).abs() > SUM_TOLERANCE {
                return Err(format!(
                    "action {} likelihoods for state {} must sum to one",
                    action.action_id, problem.state_ids[index]
                ));
            }
        }
    }
    Ok(())
}

fn entropy_bits(probabilities: &[f64]) -> f64 {
    probabilities
        .iter()
        .filter(|value| **value > 0.0)
        .map(|value| -value * value.log2())
        .sum()
}

fn decision_risk(decision: &Decision, probabilities: &[f64]) -> f64 {
    decision
        .loss_by_state
        .iter()
        .zip(probabilities.iter())
        .map(|(loss, probability)| loss * probability)
        .sum()
}

fn bayes_risk(
    decisions: &[Decision],
    probabilities: &[f64],
) -> (f64, String) {
    let mut best_risk = f64::INFINITY;
    let mut best_id = String::new();
    for decision in decisions {
        let risk = decision_risk(decision, probabilities);
        // Deterministic tie-break: first decision in declared order.
        if risk < best_risk {
            best_risk = risk;
            best_id.clone_from(&decision.decision_id);
        }
    }
    (best_risk, best_id)
}

fn evaluate_action(
    problem: &DecisionProblem,
    action: &ObservationAction,
    prior_risk: f64,
    prior_decision_id: &str,
) -> Result<ActionValue, String> {
    let exclusion_reason = if action.calibration_status != CalibrationStatus::Valid {
        Some(format!("calibration_status_{:?}", action.calibration_status).to_lowercase())
    } else if action.feasibility_status != FeasibilityStatus::Feasible {
        Some(format!("feasibility_status_{:?}", action.feasibility_status).to_lowercase())
    } else {
        None
    };
    if let Some(reason) = exclusion_reason {
        return Ok(ActionValue {
            action_id: action.action_id.clone(),
            eligible: false,
            exclusion_reason: Some(reason),
            prior_bayes_risk: prior_risk,
            expected_posterior_bayes_risk: None,
            gross_evsi: None,
            observation_cost: action.observation_cost,
            risk_cost: action.risk_cost,
            net_evsi: None,
            mutual_information_bits: None,
            prior_optimal_decision_id: prior_decision_id.to_string(),
            outcome_values: Vec::new(),
        });
    }

    let prior_entropy = entropy_bits(&problem.prior);
    let mut expected_posterior_risk = 0.0;
    let mut expected_posterior_entropy = 0.0;
    let mut outcome_values = Vec::with_capacity(action.outcomes.len());

    for outcome in &action.outcomes {
        let probability = problem
            .prior
            .iter()
            .zip(outcome.likelihood_by_state.iter())
            .map(|(prior, likelihood)| prior * likelihood)
            .sum::<f64>();
        if !probability.is_finite() || probability < 0.0 {
            return Err(format!(
                "action {} outcome {} has invalid marginal probability",
                action.action_id, outcome.outcome_id
            ));
        }
        if probability <= 1e-15 {
            outcome_values.push(OutcomeValue {
                outcome_id: outcome.outcome_id.clone(),
                probability: 0.0,
                posterior_decision_id: None,
                posterior_bayes_risk: 0.0,
            });
            continue;
        }
        let mut posterior: Vec<f64> = problem
            .prior
            .iter()
            .zip(outcome.likelihood_by_state.iter())
            .map(|(prior, likelihood)| prior * likelihood / probability)
            .collect();
        let posterior_sum = posterior.iter().sum::<f64>();
        if !posterior_sum.is_finite() || (posterior_sum - 1.0).abs() > 1e-9 {
            return Err(format!(
                "action {} outcome {} produced an invalid posterior",
                action.action_id, outcome.outcome_id
            ));
        }
        for value in &mut posterior {
            *value /= posterior_sum;
        }
        let (posterior_risk, posterior_decision_id) =
            bayes_risk(&problem.decisions, &posterior);
        let posterior_entropy = entropy_bits(&posterior);
        if !posterior_risk.is_finite() || !posterior_entropy.is_finite() {
            return Err("posterior decision calculation produced non-finite values".to_string());
        }
        expected_posterior_risk += probability * posterior_risk;
        expected_posterior_entropy += probability * posterior_entropy;
        outcome_values.push(OutcomeValue {
            outcome_id: outcome.outcome_id.clone(),
            probability,
            posterior_decision_id: Some(posterior_decision_id),
            posterior_bayes_risk: posterior_risk,
        });
    }

    let probability_sum = outcome_values.iter().map(|outcome| outcome.probability).sum::<f64>();
    if !probability_sum.is_finite() || (probability_sum - 1.0).abs() > 1e-9 {
        return Err(format!(
            "action {} marginal outcome probabilities do not sum to one",
            action.action_id
        ));
    }
    let gross_evsi = (prior_risk - expected_posterior_risk).max(0.0);
    let net_evsi = gross_evsi - action.observation_cost - action.risk_cost;
    let mutual_information_bits = (prior_entropy - expected_posterior_entropy).max(0.0);
    if !expected_posterior_risk.is_finite()
        || !gross_evsi.is_finite()
        || !net_evsi.is_finite()
        || !mutual_information_bits.is_finite()
    {
        return Err(format!("action {} EVSI calculation overflowed", action.action_id));
    }

    Ok(ActionValue {
        action_id: action.action_id.clone(),
        eligible: true,
        exclusion_reason: None,
        prior_bayes_risk: prior_risk,
        expected_posterior_bayes_risk: Some(expected_posterior_risk),
        gross_evsi: Some(gross_evsi),
        observation_cost: action.observation_cost,
        risk_cost: action.risk_cost,
        net_evsi: Some(net_evsi),
        mutual_information_bits: Some(mutual_information_bits),
        prior_optimal_decision_id: prior_decision_id.to_string(),
        outcome_values,
    })
}

pub fn evaluate(problem: &DecisionProblem) -> Result<EvsiResult, String> {
    validate_problem(problem)?;
    let (prior_bayes_risk, prior_optimal_decision_id) =
        bayes_risk(&problem.decisions, &problem.prior);
    if !prior_bayes_risk.is_finite() {
        return Err("prior Bayes risk is non-finite".to_string());
    }

    let mut action_values = Vec::with_capacity(problem.actions.len());
    for action in &problem.actions {
        action_values.push(evaluate_action(
            problem,
            action,
            prior_bayes_risk,
            &prior_optimal_decision_id,
        )?);
    }
    let top_candidate_action_id = action_values
        .iter()
        .filter(|action| action.eligible)
        .filter_map(|action| action.net_evsi.map(|value| (value, action.action_id.as_str())))
        .filter(|(value, _)| *value > 0.0)
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, action_id)| action_id.to_string());

    Ok(EvsiResult {
        schema_version: EVSI_SCHEMA_VERSION,
        problem_id: problem.problem_id.clone(),
        loss_unit: problem.loss_unit.clone(),
        prior_bayes_risk,
        prior_optimal_decision_id,
        action_values,
        top_candidate_action_id,
        actuation_authority: false,
        execution_authority: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(ch: char) -> String {
        ch.to_string().repeat(64)
    }

    fn action(
        id: &str,
        outcomes: Vec<ObservationOutcome>,
        observation_cost: f64,
    ) -> ObservationAction {
        ObservationAction {
            action_id: id.to_string(),
            observation_model_sha256: sha('a'),
            calibration_scope_sha256: sha('b'),
            calibration_status: CalibrationStatus::Valid,
            feasibility_status: FeasibilityStatus::Feasible,
            feasibility_receipt_sha256: sha('c'),
            observation_cost,
            risk_cost: 0.0,
            outcomes,
        }
    }

    fn outcome(id: &str, likelihoods: &[f64]) -> ObservationOutcome {
        ObservationOutcome {
            outcome_id: id.to_string(),
            likelihood_by_state: likelihoods.to_vec(),
        }
    }

    fn irrelevant_information_problem() -> DecisionProblem {
        DecisionProblem {
            schema_version: 1,
            problem_id: "irrelevant-information".to_string(),
            loss_unit: "mission-risk-units".to_string(),
            state_ids: vec![
                "safe-n0".into(),
                "safe-n1".into(),
                "hazard-n0".into(),
                "hazard-n1".into(),
            ],
            prior: vec![0.45, 0.45, 0.05, 0.05],
            decisions: vec![
                Decision {
                    decision_id: "accept".into(),
                    loss_by_state: vec![0.0, 0.0, 100.0, 100.0],
                },
                Decision {
                    decision_id: "hold".into(),
                    loss_by_state: vec![10.0, 10.0, 0.0, 0.0],
                },
            ],
            actions: vec![action(
                "observe-nuisance",
                vec![outcome("n0", &[1.0, 0.0, 1.0, 0.0]), outcome("n1", &[0.0, 1.0, 0.0, 1.0])],
                0.0,
            )],
        }
    }

    fn relevant_hazard_problem(cost: f64) -> DecisionProblem {
        DecisionProblem {
            schema_version: 1,
            problem_id: "hazard-relevant".to_string(),
            loss_unit: "mission-risk-units".to_string(),
            state_ids: vec!["safe".into(), "hazard".into()],
            prior: vec![0.9, 0.1],
            decisions: vec![
                Decision {
                    decision_id: "accept".into(),
                    loss_by_state: vec![0.0, 100.0],
                },
                Decision {
                    decision_id: "hold".into(),
                    loss_by_state: vec![10.0, 0.0],
                },
            ],
            actions: vec![action(
                "hazard-sensor",
                vec![outcome("alert", &[0.1, 0.9]), outcome("clear", &[0.9, 0.1])],
                cost,
            )],
        }
    }

    #[test]
    fn irrelevant_one_bit_information_has_zero_decision_value() {
        let result = evaluate(&irrelevant_information_problem()).unwrap();
        let action = &result.action_values[0];
        assert_eq!(result.prior_optimal_decision_id, "hold");
        assert!(action.eligible);
        assert_eq!(action.mutual_information_bits, Some(1.0));
        assert!(action.gross_evsi.unwrap().abs() < 1e-12);
        assert!(action.net_evsi.unwrap().abs() < 1e-12);
        assert!(result.top_candidate_action_id.is_none());
    }

    #[test]
    fn decision_relevant_information_has_positive_gross_and_net_evsi() {
        let result = evaluate(&relevant_hazard_problem(3.0)).unwrap();
        let action = &result.action_values[0];
        assert_eq!(result.prior_optimal_decision_id, "hold");
        assert!(action.gross_evsi.unwrap() > 7.0);
        assert!(action.net_evsi.unwrap() > 4.0);
        assert_eq!(result.top_candidate_action_id.as_deref(), Some("hazard-sensor"));
        let alert = action.outcome_values.iter().find(|row| row.outcome_id == "alert").unwrap();
        let clear = action.outcome_values.iter().find(|row| row.outcome_id == "clear").unwrap();
        assert_eq!(alert.posterior_decision_id.as_deref(), Some("hold"));
        assert_eq!(clear.posterior_decision_id.as_deref(), Some("accept"));
    }

    #[test]
    fn acquisition_cost_can_make_informative_action_net_negative() {
        let result = evaluate(&relevant_hazard_problem(9.0)).unwrap();
        let action = &result.action_values[0];
        assert!(action.gross_evsi.unwrap() > 7.0);
        assert!(action.net_evsi.unwrap() < 0.0);
        assert!(result.top_candidate_action_id.is_none());
    }

    #[test]
    fn unknown_or_infeasible_candidates_are_not_given_evsi() {
        let mut problem = relevant_hazard_problem(0.0);
        problem.actions[0].feasibility_status = FeasibilityStatus::Unknown;
        let result = evaluate(&problem).unwrap();
        assert!(!result.action_values[0].eligible);
        assert_eq!(result.action_values[0].net_evsi, None);
        assert_eq!(result.top_candidate_action_id, None);

        let mut problem = relevant_hazard_problem(0.0);
        problem.actions[0].calibration_status = CalibrationStatus::Unknown;
        let result = evaluate(&problem).unwrap();
        assert!(!result.action_values[0].eligible);
        assert_eq!(result.action_values[0].gross_evsi, None);
    }

    #[test]
    fn invalid_prior_loss_and_likelihoods_fail_closed() {
        let mut problem = relevant_hazard_problem(0.0);
        problem.prior = vec![0.8, 0.1];
        assert!(evaluate(&problem).is_err());

        let mut problem = relevant_hazard_problem(0.0);
        problem.decisions[0].loss_by_state[0] = f64::NAN;
        assert!(evaluate(&problem).is_err());

        let mut problem = relevant_hazard_problem(0.0);
        problem.actions[0].outcomes[0].likelihood_by_state[0] = 0.8;
        assert!(evaluate(&problem).is_err());
    }
}
