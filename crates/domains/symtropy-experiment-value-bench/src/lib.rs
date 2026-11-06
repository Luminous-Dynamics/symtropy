//! Finite-state one-step expected value of sample information (EVSI).
//!
//! This is a transparent decision calculation, not a learned experiment policy.
//! Priors, likelihoods, utilities and costs must be declared and evidenced by
//! the caller. Synthetic/simulation-only inputs never become decision advice.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const PROBABILITY_TOLERANCE: f64 = 1.0e-9;
const NUMERIC_TOLERANCE: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentValue {
    pub experiment_id: String,
    pub expected_value_before: f64,
    pub expected_value_after: f64,
    pub evsi: f64,
    pub cost: f64,
    pub net_evsi: f64,
    pub within_budget: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub utility_unit: String,
    pub evaluation_scope: String,
    pub evidence_disposition: String,
    pub current_best_value: f64,
    pub current_best_actions: Vec<String>,
    pub experiment_values: Vec<ExperimentValue>,
    pub selected_experiment_id: Option<String>,
    pub stopping_disposition: String,
    pub stopping_reason: String,
    pub remaining_budget: f64,
    pub minimum_net_evsi_to_continue: f64,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn evidence_role_is_valid(role: &str) -> bool {
    matches!(
        role,
        "INDEPENDENT_MEASUREMENT"
            | "EXTERNAL_CALIBRATION"
            | "EXPERT_ELICITATION"
            | "SIMULATION_ONLY"
    )
}

fn validate_evidence(value: &Value, path: &str, errors: &mut Vec<Violation>) {
    match text(value, "evidence_role") {
        Some(role) if evidence_role_is_valid(role) => {}
        _ => push(
            errors,
            "INVALID_EVIDENCE_ROLE",
            &format!("{path}.evidence_role"),
            "evidence_role must be explicit: independent measurement, external calibration, expert elicitation or simulation only",
        ),
    }
    for key in ["method_ref", "source_ref"] {
        if text(value, key).is_none() {
            push(
                errors,
                "MISSING_EVIDENCE_PROVENANCE",
                &format!("{path}.{key}"),
                "method and source references are required",
            );
        }
    }
}

/// Validate priors, utilities, likelihoods, costs, units and stopping policy.
pub fn validate_case(case: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(
            &mut errors,
            "UNSUPPORTED_SCHEMA_VERSION",
            "$.schema_version",
            "schema_version must be exactly 1",
        );
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    if !matches!(
        text(case, "evidence_class"),
        Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")
    ) {
        push(
            &mut errors,
            "INVALID_EVIDENCE_CLASS",
            "$.evidence_class",
            "evidence_class must be SYNTHETIC_CONFORMANCE_FIXTURE or MEASURED",
        );
    }

    let Some(profile) = case.get("decision_profile") else {
        push(
            &mut errors,
            "MISSING_DECISION_PROFILE",
            "$.decision_profile",
            "decision profile is required",
        );
        return errors;
    };
    for key in [
        "decision_profile_ref",
        "decision_owner_ref",
        "utility_validation_ref",
        "utility_unit",
    ] {
        if text(profile, key).is_none() {
            push(
                &mut errors,
                "MISSING_DECISION_PROFILE_FIELD",
                &format!("$.decision_profile.{key}"),
                "decision owner, validated utility profile and utility unit must be explicit",
            );
        }
    }
    let utility_unit = text(profile, "utility_unit");

    let Some(states) = case.get("states").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_STATES",
            "$.states",
            "states must be an array",
        );
        return errors;
    };
    if states.is_empty() {
        push(
            &mut errors,
            "EMPTY_STATES",
            "$.states",
            "at least one decision uncertainty state is required",
        );
    }

    let mut state_ids = BTreeSet::new();
    let mut prior_sum = 0.0;
    for (index, state) in states.iter().enumerate() {
        let path = format!("$.states[{index}]");
        let Some(id) = text(state, "state_id") else {
            push(
                &mut errors,
                "MISSING_STATE_ID",
                &format!("{path}.state_id"),
                "state_id is required",
            );
            continue;
        };
        if !state_ids.insert(id.to_owned()) {
            push(
                &mut errors,
                "DUPLICATE_STATE_ID",
                &format!("{path}.state_id"),
                "state identifiers must be unique",
            );
        }
        match finite(state, "prior_probability") {
            Some(p) if (0.0..=1.0).contains(&p) => prior_sum += p,
            _ => push(
                &mut errors,
                "INVALID_PRIOR_PROBABILITY",
                &format!("{path}.prior_probability"),
                "prior probability must be finite and in [0,1]",
            ),
        }
        if let Some(evidence) = state.get("evidence") {
            validate_evidence(evidence, &format!("{path}.evidence"), &mut errors);
        } else {
            push(
                &mut errors,
                "MISSING_PRIOR_EVIDENCE",
                &format!("{path}.evidence"),
                "prior distribution requires declared evidence provenance",
            );
        }
    }
    if (prior_sum - 1.0).abs() > PROBABILITY_TOLERANCE {
        push(
            &mut errors,
            "PRIOR_NOT_NORMALIZED",
            "$.states",
            format!("prior probabilities sum to {prior_sum}, expected 1 within tolerance"),
        );
    }

    let Some(actions) = case.get("actions").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_ACTIONS",
            "$.actions",
            "actions must be an array",
        );
        return errors;
    };
    if actions.is_empty() {
        push(
            &mut errors,
            "EMPTY_ACTIONS",
            "$.actions",
            "at least one admissible decision action is required",
        );
    }

    let mut action_ids = BTreeSet::new();
    for (action_index, action) in actions.iter().enumerate() {
        let path = format!("$.actions[{action_index}]");
        let Some(action_id) = text(action, "action_id") else {
            push(
                &mut errors,
                "MISSING_ACTION_ID",
                &format!("{path}.action_id"),
                "action_id is required",
            );
            continue;
        };
        if !action_ids.insert(action_id.to_owned()) {
            push(
                &mut errors,
                "DUPLICATE_ACTION_ID",
                &format!("{path}.action_id"),
                "action identifiers must be unique",
            );
        }

        let Some(utilities) = action.get("utilities").and_then(Value::as_array) else {
            push(
                &mut errors,
                "MISSING_ACTION_UTILITIES",
                &format!("{path}.utilities"),
                "utilities must be an array",
            );
            continue;
        };

        let mut seen_states = BTreeSet::new();
        for (utility_index, utility) in utilities.iter().enumerate() {
            let utility_path = format!("{path}.utilities[{utility_index}]");
            let Some(state_id) = text(utility, "state_id") else {
                push(
                    &mut errors,
                    "MISSING_UTILITY_STATE",
                    &format!("{utility_path}.state_id"),
                    "utility row must reference a declared state",
                );
                continue;
            };
            if !state_ids.contains(state_id) {
                push(
                    &mut errors,
                    "UNKNOWN_UTILITY_STATE",
                    &format!("{utility_path}.state_id"),
                    "utility row references an undeclared state",
                );
            }
            if !seen_states.insert(state_id.to_owned()) {
                push(
                    &mut errors,
                    "DUPLICATE_UTILITY_STATE",
                    &format!("{utility_path}.state_id"),
                    "action may have exactly one utility per state",
                );
            }
            if finite(utility, "value").is_none() {
                push(
                    &mut errors,
                    "INVALID_UTILITY_VALUE",
                    &format!("{utility_path}.value"),
                    "utility must be finite",
                );
            }
        }

        if seen_states.len() != state_ids.len()
            || state_ids.iter().any(|id| !seen_states.contains(id))
        {
            push(
                &mut errors,
                "INCOMPLETE_UTILITY_TABLE",
                &format!("{path}.utilities"),
                "each action requires exactly one utility for every declared state",
            );
        }
    }

    let Some(experiments) = case.get("experiments").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_EXPERIMENTS",
            "$.experiments",
            "experiments must be an array",
        );
        return errors;
    };
    if experiments.is_empty() {
        push(
            &mut errors,
            "EMPTY_EXPERIMENTS",
            "$.experiments",
            "at least one candidate experiment is required",
        );
    }

    let mut experiment_ids = BTreeSet::new();
    for (experiment_index, experiment) in experiments.iter().enumerate() {
        let path = format!("$.experiments[{experiment_index}]");
        let Some(experiment_id) = text(experiment, "experiment_id") else {
            push(
                &mut errors,
                "MISSING_EXPERIMENT_ID",
                &format!("{path}.experiment_id"),
                "experiment_id is required",
            );
            continue;
        };
        if !experiment_ids.insert(experiment_id.to_owned()) {
            push(
                &mut errors,
                "DUPLICATE_EXPERIMENT_ID",
                &format!("{path}.experiment_id"),
                "experiment identifiers must be unique",
            );
        }

        let Some(cost) = experiment.get("cost") else {
            push(
                &mut errors,
                "MISSING_EXPERIMENT_COST",
                &format!("{path}.cost"),
                "experiment cost is mandatory",
            );
            continue;
        };
        match finite(cost, "value") {
            Some(value) if value >= 0.0 => {}
            _ => push(
                &mut errors,
                "INVALID_EXPERIMENT_COST",
                &format!("{path}.cost.value"),
                "experiment cost must be finite and non-negative",
            ),
        }
        if text(cost, "unit") != utility_unit {
            push(
                &mut errors,
                "EXPERIMENT_COST_UNIT_MISMATCH",
                &format!("{path}.cost.unit"),
                "cost must use the exact declared utility unit; any conversion requires an externally justified profile",
            );
        }
        for key in ["boundary_ref", "source_ref"] {
            if text(cost, key).is_none() {
                push(
                    &mut errors,
                    "MISSING_COST_PROVENANCE",
                    &format!("{path}.cost.{key}"),
                    "cost boundary and source references are required",
                );
            }
        }

        if let Some(evidence) = experiment.get("likelihood_evidence") {
            validate_evidence(evidence, &format!("{path}.likelihood_evidence"), &mut errors);
        } else {
            push(
                &mut errors,
                "MISSING_LIKELIHOOD_EVIDENCE",
                &format!("{path}.likelihood_evidence"),
                "observation likelihoods require explicit evidence provenance",
            );
        }

        let Some(outcomes) = experiment.get("outcomes").and_then(Value::as_array) else {
            push(
                &mut errors,
                "MISSING_EXPERIMENT_OUTCOMES",
                &format!("{path}.outcomes"),
                "outcomes must be an array",
            );
            continue;
        };
        if outcomes.is_empty() {
            push(
                &mut errors,
                "EMPTY_EXPERIMENT_OUTCOMES",
                &format!("{path}.outcomes"),
                "at least one possible experiment outcome is required",
            );
            continue;
        }

        let mut outcome_ids = BTreeSet::new();
        let mut probability_sums: BTreeMap<String, f64> =
            state_ids.iter().map(|id| (id.clone(), 0.0)).collect();

        for (outcome_index, outcome) in outcomes.iter().enumerate() {
            let outcome_path = format!("{path}.outcomes[{outcome_index}]");
            let Some(outcome_id) = text(outcome, "outcome_id") else {
                push(
                    &mut errors,
                    "MISSING_OUTCOME_ID",
                    &format!("{outcome_path}.outcome_id"),
                    "outcome_id is required",
                );
                continue;
            };
            if !outcome_ids.insert(outcome_id.to_owned()) {
                push(
                    &mut errors,
                    "DUPLICATE_OUTCOME_ID",
                    &format!("{outcome_path}.outcome_id"),
                    "outcome identifiers must be unique within an experiment",
                );
            }

            let Some(likelihoods) = outcome.get("likelihoods").and_then(Value::as_array) else {
                push(
                    &mut errors,
                    "MISSING_OUTCOME_LIKELIHOODS",
                    &format!("{outcome_path}.likelihoods"),
                    "each outcome requires P(outcome | state) for every state",
                );
                continue;
            };
            let mut seen = BTreeSet::new();
            for (likelihood_index, likelihood) in likelihoods.iter().enumerate() {
                let likelihood_path = format!("{outcome_path}.likelihoods[{likelihood_index}]");
                let Some(state_id) = text(likelihood, "state_id") else {
                    push(
                        &mut errors,
                        "MISSING_LIKELIHOOD_STATE",
                        &format!("{likelihood_path}.state_id"),
                        "likelihood row must identify a declared state",
                    );
                    continue;
                };
                if !state_ids.contains(state_id) {
                    push(
                        &mut errors,
                        "UNKNOWN_LIKELIHOOD_STATE",
                        &format!("{likelihood_path}.state_id"),
                        "likelihood row references an undeclared state",
                    );
                }
                if !seen.insert(state_id.to_owned()) {
                    push(
                        &mut errors,
                        "DUPLICATE_LIKELIHOOD_STATE",
                        &format!("{likelihood_path}.state_id"),
                        "each outcome has exactly one conditional likelihood per state",
                    );
                }
                match finite(likelihood, "probability") {
                    Some(probability) if (0.0..=1.0).contains(&probability) => {
                        if let Some(sum) = probability_sums.get_mut(state_id) {
                            *sum += probability;
                        }
                    }
                    _ => push(
                        &mut errors,
                        "INVALID_LIKELIHOOD_PROBABILITY",
                        &format!("{likelihood_path}.probability"),
                        "conditional probability must be finite and in [0,1]",
                    ),
                }
            }
            if seen.len() != state_ids.len()
                || state_ids.iter().any(|id| !seen.contains(id))
            {
                push(
                    &mut errors,
                    "INCOMPLETE_LIKELIHOOD_TABLE",
                    &format!("{outcome_path}.likelihoods"),
                    "each outcome requires exactly one likelihood for every declared state",
                );
            }
        }

        for (state_id, sum) in probability_sums {
            if (sum - 1.0).abs() > PROBABILITY_TOLERANCE {
                push(
                    &mut errors,
                    "LIKELIHOODS_NOT_NORMALIZED",
                    &format!("{path}.outcomes"),
                    format!("for state {state_id}, outcome likelihoods sum to {sum}, expected 1"),
                );
            }
        }
    }

    let Some(stopping) = case.get("stopping_policy") else {
        push(
            &mut errors,
            "MISSING_STOPPING_POLICY",
            "$.stopping_policy",
            "explicit stopping policy required",
        );
        return errors;
    };
    for key in [
        "stop_consequence_ref",
        "continue_authority_ref",
        "budget_unit",
    ] {
        if text(stopping, key).is_none() {
            push(
                &mut errors,
                "MISSING_STOPPING_POLICY_FIELD",
                &format!("$.stopping_policy.{key}"),
                "stopping consequence, continue authority and budget unit are required",
            );
        }
    }
    if text(stopping, "budget_unit") != utility_unit {
        push(
            &mut errors,
            "BUDGET_UNIT_MISMATCH",
            "$.stopping_policy.budget_unit",
            "remaining budget must use the exact declared utility unit",
        );
    }
    match finite(stopping, "minimum_net_evsi_to_continue") {
        Some(_) => {}
        _ => push(
            &mut errors,
            "INVALID_STOP_THRESHOLD",
            "$.stopping_policy.minimum_net_evsi_to_continue",
            "minimum net EVSI threshold must be finite",
        ),
    }
    match finite(stopping, "remaining_budget") {
        Some(value) if value >= 0.0 => {}
        _ => push(
            &mut errors,
            "INVALID_REMAINING_BUDGET",
            "$.stopping_policy.remaining_budget",
            "remaining budget must be finite and non-negative",
        ),
    }
    if stopping
        .get("maximum_experiments_remaining")
        .and_then(Value::as_u64)
        .is_none()
    {
        push(
            &mut errors,
            "INVALID_EXPERIMENT_BUDGET",
            "$.stopping_policy.maximum_experiments_remaining",
            "maximum_experiments_remaining must be a non-negative integer",
        );
    }

    errors
}

fn current_decision_value(case: &Value) -> (f64, Vec<String>) {
    let states = case["states"].as_array().expect("validated states");
    let actions = case["actions"].as_array().expect("validated actions");
    let mut best_value = f64::NEG_INFINITY;
    let mut best_actions = Vec::new();

    for action in actions {
        let utilities: BTreeMap<&str, f64> = action["utilities"]
            .as_array()
            .expect("validated utilities")
            .iter()
            .map(|utility| {
                (
                    text(utility, "state_id").expect("validated utility state"),
                    finite(utility, "value").expect("validated utility value"),
                )
            })
            .collect();

        let value = states
            .iter()
            .map(|state| {
                let id = text(state, "state_id").expect("validated state id");
                let probability = finite(state, "prior_probability")
                    .expect("validated prior probability");
                probability * utilities[id]
            })
            .sum::<f64>();

        let action_id = text(action, "action_id").expect("validated action id");
        if value > best_value + NUMERIC_TOLERANCE {
            best_value = value;
            best_actions.clear();
            best_actions.push(action_id.to_owned());
        } else if (value - best_value).abs() <= NUMERIC_TOLERANCE {
            best_actions.push(action_id.to_owned());
        }
    }

    best_actions.sort();
    (best_value, best_actions)
}

fn expected_post_experiment_value(
    case: &Value,
    experiment: &Value,
) -> Result<f64, Vec<Violation>> {
    let states = case["states"].as_array().expect("validated states");
    let actions = case["actions"].as_array().expect("validated actions");
    let outcomes = experiment["outcomes"].as_array().expect("validated outcomes");
    let prior: Vec<f64> = states
        .iter()
        .map(|state| finite(state, "prior_probability").expect("validated prior"))
        .collect();

    let mut expected_value = 0.0;
    for outcome in outcomes {
        let likelihoods = outcome["likelihoods"].as_array().expect("validated likelihoods");
        let mut likelihood_by_state = BTreeMap::new();
        for likelihood in likelihoods {
            let id = text(likelihood, "state_id").expect("validated state id");
            let probability = finite(likelihood, "probability").expect("validated likelihood");
            likelihood_by_state.insert(id, probability);
        }

        let mut predictive_probability = 0.0;
        let mut nonzero_product_underflowed = false;
        for (state, prior_probability) in states.iter().zip(prior.iter()) {
            let state_id = text(state, "state_id").expect("validated state");
            let likelihood = likelihood_by_state[state_id];
            let product = prior_probability * likelihood;
            if *prior_probability > 0.0 && likelihood > 0.0 && product == 0.0 {
                nonzero_product_underflowed = true;
            }
            predictive_probability += product;
        }

        if nonzero_product_underflowed {
            return Err(vec![Violation {
                code: "PREDICTIVE_PROBABILITY_UNDERFLOW".to_owned(),
                path: "$.experiments".to_owned(),
                detail: "a mathematically non-zero prior-times-likelihood product underflowed to zero; refusing to treat it as an impossible outcome".to_owned(),
            }]);
        }

        if !predictive_probability.is_finite() {
            return Err(vec![Violation {
                code: "NONFINITE_PREDICTIVE_PROBABILITY".to_owned(),
                path: "$.experiments".to_owned(),
                detail: "predictive outcome probability overflowed".to_owned(),
            }]);
        }

        if predictive_probability <= 0.0 {
            // This outcome has zero prior predictive support. Its posterior is
            // undefined, but it contributes exactly zero expected value.
            continue;
        }

        let posterior: Vec<f64> = states
            .iter()
            .zip(prior.iter())
            .map(|(state, p)| {
                let id = text(state, "state_id").expect("validated state");
                p * likelihood_by_state[id] / predictive_probability
            })
            .collect();

        let mut best_after = f64::NEG_INFINITY;
        for action in actions {
            let utilities: BTreeMap<&str, f64> = action["utilities"]
                .as_array()
                .expect("validated utilities")
                .iter()
                .map(|utility| {
                    (
                        text(utility, "state_id").expect("validated utility state"),
                        finite(utility, "value").expect("validated utility"),
                    )
                })
                .collect();

            let value: f64 = states
                .iter()
                .zip(posterior.iter())
                .map(|(state, p)| {
                    let id = text(state, "state_id").expect("validated state");
                    p * utilities[id]
                })
                .sum();
            best_after = best_after.max(value);
        }

        expected_value += predictive_probability * best_after;
    }

    if !expected_value.is_finite() {
        return Err(vec![Violation {
            code: "NONFINITE_POST_EXPERIMENT_VALUE".to_owned(),
            path: "$.experiments".to_owned(),
            detail: "expected post-experiment decision value overflowed".to_owned(),
        }]);
    }
    Ok(expected_value)
}

fn has_synthetic_inputs(case: &Value) -> bool {
    if text(case, "evidence_class") != Some("MEASURED") {
        return true;
    }
    let state_synthetic = case["states"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|state| {
            text(&state["evidence"], "evidence_role") == Some("SIMULATION_ONLY")
        });
    let experiment_synthetic = case["experiments"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|experiment| {
            text(&experiment["likelihood_evidence"], "evidence_role")
                == Some("SIMULATION_ONLY")
        });
    state_synthetic || experiment_synthetic
}

/// Calculate myopic one-step EVSI per candidate and apply the declared stopping
/// policy. A positive candidate is only a request for human review, never an
/// instruction to perform physical work or change machine settings.
pub fn evaluate_case(case: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case);
    if !errors.is_empty() {
        return Err(errors);
    }

    let profile = &case["decision_profile"];
    let utility_unit = text(profile, "utility_unit")
        .expect("validated utility unit")
        .to_owned();
    let (current_value, current_best_actions) = current_decision_value(case);
    let remaining_budget = finite(&case["stopping_policy"], "remaining_budget")
        .expect("validated budget");
    let threshold =
        finite(&case["stopping_policy"], "minimum_net_evsi_to_continue")
            .expect("validated threshold");
    let max_experiments = case["stopping_policy"]["maximum_experiments_remaining"]
        .as_u64()
        .expect("validated experiment budget");
    let mut values = Vec::new();

    for experiment in case["experiments"]
        .as_array()
        .expect("validated experiments")
    {
        let cost = finite(&experiment["cost"], "value").expect("validated cost");
        let within_budget = cost <= remaining_budget + NUMERIC_TOLERANCE;
        let after = expected_post_experiment_value(case, experiment)?;
        let raw_evsi = after - current_value;
        if raw_evsi < -NUMERIC_TOLERANCE {
            return Err(vec![Violation {
                code: "NEGATIVE_EVSI_NUMERIC_CONTRADICTION".to_owned(),
                path: format!(
                    "$.experiments.{}",
                    text(experiment, "experiment_id").unwrap_or("unknown")
                ),
                detail: "expected post-experiment value is materially below current value; inspect numerical inputs".to_owned(),
            }]);
        }
        let evsi = raw_evsi.max(0.0);
        let net_evsi = evsi - cost;
        values.push(ExperimentValue {
            experiment_id: text(experiment, "experiment_id")
                .expect("validated experiment id")
                .to_owned(),
            expected_value_before: current_value,
            expected_value_after: after,
            evsi,
            cost,
            net_evsi,
            within_budget,
        });
    }

    values.sort_by(|a, b| {
        b.net_evsi
            .total_cmp(&a.net_evsi)
            .then_with(|| a.experiment_id.cmp(&b.experiment_id))
    });

    let mut selected = None;
    let mut reason = String::new();
    let disposition;

    if max_experiments == 0 {
        disposition = "STOP";
        reason = "NO_EXPERIMENTS_REMAINING".to_owned();
    } else {
        let candidate = values.iter().find(|value| value.within_budget);
        match candidate {
            None => {
                disposition = "STOP";
                reason = "NO_CANDIDATE_WITHIN_REMAINING_BUDGET".to_owned();
            }
            Some(best) if best.net_evsi <= threshold + NUMERIC_TOLERANCE => {
                disposition = "STOP";
                reason = "BEST_NET_EVSI_DOES_NOT_EXCEED_STOP_THRESHOLD".to_owned();
            }
            Some(best) => {
                selected = Some(best.experiment_id.clone());
                disposition = "HUMAN_REVIEW_CANDIDATE";
                reason = "BEST_ONE_STEP_NET_EVSI_EXCEEDS_THRESHOLD_WITHIN_BUDGET".to_owned();
            }
        }
    }

    let evidence_disposition = if has_synthetic_inputs(case) {
        "SYNTHETIC_ONLY_NOT_DECISION_QUALIFIED"
    } else {
        "MEASURED_INPUTS_REVIEW_REQUIRED"
    };

    Ok(EvaluationReport {
        case_id: text(case, "case_id").expect("validated case id").to_owned(),
        utility_unit,
        evaluation_scope: "MYOPIC_ONE_STEP_EVSI_NOT_A_MULTI_STEP_OPTIMAL_POLICY".to_owned(),
        evidence_disposition: evidence_disposition.to_owned(),
        current_best_value: current_value,
        current_best_actions,
        experiment_values: values,
        selected_experiment_id: selected,
        stopping_disposition: disposition.to_owned(),
        stopping_reason: reason,
        remaining_budget,
        minimum_net_evsi_to_continue: threshold,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-experiment-value-case.v0_1.json"
        ))
        .expect("synthetic fixture parses")
    }

    fn has_code(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn experiment_mut(case: &mut Value, index: usize) -> &mut Value {
        &mut case["experiments"][index]
    }

    fn first_outcome_mut(case: &mut Value, experiment: usize) -> &mut Value {
        &mut case["experiments"][experiment]["outcomes"][0]
    }

    fn golden_experiment(report: &EvaluationReport, id: &str) -> &ExperimentValue {
        report
            .experiment_values
            .iter()
            .find(|value| value.experiment_id == id)
            .expect("experiment result")
    }

    #[test]
    fn synthetic_case_is_semantically_valid() {
        assert!(validate_case(&fixture()).is_empty());
    }

    #[test]
    fn golden_vector_computes_one_step_evsi_and_net_value() {
        let report = evaluate_case(&fixture()).expect("synthetic case evaluates");
        let pressure = golden_experiment(&report, "pressure-differential-probe");
        assert!((report.current_best_value - 48.0).abs() < 1e-9);
        assert!((pressure.expected_value_after - 65.2).abs() < 1e-9);
        assert!((pressure.evsi - 17.2).abs() < 1e-9);
        assert!((pressure.net_evsi - 12.2).abs() < 1e-9);
        assert!(report.current_best_actions.contains(&"improve-transport".to_owned()));
    }

    #[test]
    fn no_decision_change_means_zero_evsi_even_when_outcomes_differ() {
        let report = evaluate_case(&fixture()).expect("synthetic case evaluates");
        let weak = golden_experiment(&report, "weak-discriminator-probe");
        assert!(weak.evsi.abs() < 1e-9);
        assert!((weak.net_evsi + 2.0).abs() < 1e-9);
    }

    #[test]
    fn positive_net_evsi_only_yields_a_human_review_candidate() {
        let report = evaluate_case(&fixture()).expect("synthetic case evaluates");
        assert_eq!(
            report.stopping_disposition,
            "HUMAN_REVIEW_CANDIDATE"
        );
        assert_eq!(
            report.selected_experiment_id.as_deref(),
            Some("pressure-differential-probe")
        );
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY_NOT_DECISION_QUALIFIED");
    }

    #[test]
    fn increasing_experiment_cost_does_not_change_evsi() {
        let mut case = fixture();
        let before = evaluate_case(&case).expect("before").experiment_values;
        experiment_mut(&mut case, 0)["cost"]["value"] = serde_json::json!(9.0);
        let after = evaluate_case(&case).expect("after").experiment_values;
        let before_pressure = before.iter().find(|v| v.experiment_id == "pressure-differential-probe").unwrap();
        let after_pressure = after.iter().find(|v| v.experiment_id == "pressure-differential-probe").unwrap();
        assert!((before_pressure.evsi - after_pressure.evsi).abs() < 1e-9);
        assert!((after_pressure.net_evsi - (before_pressure.net_evsi - 4.0)).abs() < 1e-9);
    }

    #[test]
    fn cost_over_budget_excludes_candidate() {
        let mut case = fixture();
        case["stopping_policy"]["remaining_budget"] = serde_json::json!(4.0);
        let report = evaluate_case(&case).expect("case evaluates");
        assert_eq!(report.stopping_disposition, "STOP");
        assert!(report.selected_experiment_id.is_none());
        assert_eq!(report.stopping_reason, "NO_CANDIDATE_WITHIN_REMAINING_BUDGET");
    }

    #[test]
    fn stop_threshold_is_enforced() {
        let mut case = fixture();
        case["stopping_policy"]["minimum_net_evsi_to_continue"] = serde_json::json!(20.0);
        let report = evaluate_case(&case).expect("case evaluates");
        assert_eq!(report.stopping_disposition, "STOP");
        assert_eq!(
            report.stopping_reason,
            "BEST_NET_EVSI_DOES_NOT_EXCEED_STOP_THRESHOLD"
        );
    }

    #[test]
    fn zero_remaining_experiment_budget_stops() {
        let mut case = fixture();
        case["stopping_policy"]["maximum_experiments_remaining"] = serde_json::json!(0);
        let report = evaluate_case(&case).expect("case evaluates");
        assert_eq!(report.stopping_disposition, "STOP");
        assert_eq!(report.stopping_reason, "NO_EXPERIMENTS_REMAINING");
    }

    #[test]
    fn prior_must_normalize() {
        let mut case = fixture();
        case["states"][0]["prior_probability"] = serde_json::json!(0.7);
        assert!(has_code(&validate_case(&case), "PRIOR_NOT_NORMALIZED"));
    }

    #[test]
    fn duplicate_state_id_is_rejected() {
        let mut case = fixture();
        case["states"][1]["state_id"] = case["states"][0]["state_id"].clone();
        assert!(has_code(&validate_case(&case), "DUPLICATE_STATE_ID"));
    }

    #[test]
    fn probabilities_outside_unit_interval_are_rejected() {
        let mut case = fixture();
        case["states"][0]["prior_probability"] = serde_json::json!(1.1);
        assert!(has_code(&validate_case(&case), "INVALID_PRIOR_PROBABILITY"));
    }

    #[test]
    fn utility_table_must_cover_each_state_exactly_once() {
        let mut case = fixture();
        case["actions"][0]["utilities"].as_array_mut().unwrap().pop();
        assert!(has_code(&validate_case(&case), "INCOMPLETE_UTILITY_TABLE"));
    }

    #[test]
    fn duplicate_utility_state_is_rejected() {
        let mut case = fixture();
        case["actions"][0]["utilities"][1]["state_id"] =
            case["actions"][0]["utilities"][0]["state_id"].clone();
        assert!(has_code(&validate_case(&case), "DUPLICATE_UTILITY_STATE"));
    }

    #[test]
    fn non_finite_or_unparseable_utility_is_rejected() {
        let mut case = fixture();
        case["actions"][0]["utilities"][0]["value"] = serde_json::json!("not-a-number");
        assert!(has_code(&validate_case(&case), "INVALID_UTILITY_VALUE"));
    }

    #[test]
    fn likelihood_must_cover_every_state() {
        let mut case = fixture();
        first_outcome_mut(&mut case, 0)["likelihoods"].as_array_mut().unwrap().pop();
        assert!(has_code(&validate_case(&case), "INCOMPLETE_LIKELIHOOD_TABLE"));
    }

    #[test]
    fn likelihoods_must_normalize_for_each_state() {
        let mut case = fixture();
        case["experiments"][0]["outcomes"][0]["likelihoods"][0]["probability"] =
            serde_json::json!(0.9);
        assert!(has_code(&validate_case(&case), "LIKELIHOODS_NOT_NORMALIZED"));
    }

    #[test]
    fn duplicate_outcome_id_is_rejected() {
        let mut case = fixture();
        let duplicate = case["experiments"][0]["outcomes"][0].clone();
        case["experiments"][0]["outcomes"].as_array_mut().unwrap().push(duplicate);
        assert!(has_code(&validate_case(&case), "DUPLICATE_OUTCOME_ID"));
    }

    #[test]
    fn cost_unit_must_match_utility_unit() {
        let mut case = fixture();
        case["experiments"][0]["cost"]["unit"] = serde_json::json!("USD");
        assert!(has_code(&validate_case(&case), "EXPERIMENT_COST_UNIT_MISMATCH"));
    }

    #[test]
    fn budget_unit_must_match_utility_unit() {
        let mut case = fixture();
        case["stopping_policy"]["budget_unit"] = serde_json::json!("USD");
        assert!(has_code(&validate_case(&case), "BUDGET_UNIT_MISMATCH"));
    }

    #[test]
    fn synthetic_probability_sources_never_become_decision_qualified() {
        let report = evaluate_case(&fixture()).expect("case evaluates");
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY_NOT_DECISION_QUALIFIED");
    }

    #[test]
    fn zero_predictive_probability_outcome_is_skipped_without_division_by_zero() {
        let mut case = fixture();
        let outcome = serde_json::json!({
            "outcome_id":"impossible-observation",
            "likelihoods":[
                {"state_id":"transport-limited","probability":0.0},
                {"state_id":"fragmentation-limited","probability":0.0}
            ]
        });
        case["experiments"][0]["outcomes"].as_array_mut().unwrap().push(outcome);
        for state in 0..2 {
            case["experiments"][0]["outcomes"][0]["likelihoods"][state]["probability"] =
                serde_json::json!(if state == 0 {0.8} else {0.2});
            case["experiments"][0]["outcomes"][1]["likelihoods"][state]["probability"] =
                serde_json::json!(if state == 0 {0.2} else {0.8});
        }
        let report = evaluate_case(&case).expect("zero predictive outcome is harmless");
        let pressure = golden_experiment(&report, "pressure-differential-probe");
        assert!((pressure.evsi - 17.2).abs() < 1e-9);
    }

    #[test]
    fn tiny_nonzero_prior_underflow_is_rejected() {
        let mut case = fixture();
        case["states"][0]["prior_probability"] = serde_json::json!(5.0e-324);
        case["states"][1]["prior_probability"] = serde_json::json!(1.0);
        let result = evaluate_case(&case);
        assert!(matches!(
            result,
            Err(errors) if has_code(&errors, "PREDICTIVE_PROBABILITY_UNDERFLOW")
        ));
    }

    #[test]
    fn splitting_one_outcome_preserves_evsi() {
        let mut case = fixture();
        let original = evaluate_case(&case).expect("original");
        let first = case["experiments"][0]["outcomes"][0].clone();
        let mut second = first.clone();
        second["outcome_id"] = serde_json::json!("high-differential-pressure-copy");
        for likelihood in second["likelihoods"].as_array_mut().unwrap() {
            likelihood["probability"] =
                serde_json::json!(likelihood["probability"].as_f64().unwrap() / 2.0);
        }
        for likelihood in case["experiments"][0]["outcomes"][0]["likelihoods"]
            .as_array_mut()
            .unwrap()
        {
            likelihood["probability"] =
                serde_json::json!(likelihood["probability"].as_f64().unwrap() / 2.0);
        }
        case["experiments"][0]["outcomes"]
            .as_array_mut().unwrap().push(second);
        let split = evaluate_case(&case).expect("split outcomes");
        let original_evsi = golden_experiment(&original, "pressure-differential-probe").evsi;
        let split_evsi = golden_experiment(&split, "pressure-differential-probe").evsi;
        assert!((original_evsi - split_evsi).abs() < 1e-9);
    }

    #[test]
    fn state_and_action_permutation_preserves_evsi() {
        let mut case = fixture();
        let before = evaluate_case(&case).expect("before");
        case["states"].as_array_mut().unwrap().reverse();
        case["actions"].as_array_mut().unwrap().reverse();
        for experiment in case["experiments"].as_array_mut().unwrap() {
            for outcome in experiment["outcomes"].as_array_mut().unwrap() {
                outcome["likelihoods"].as_array_mut().unwrap().reverse();
            }
        }
        for action in case["actions"].as_array_mut().unwrap() {
            action["utilities"].as_array_mut().unwrap().reverse();
        }
        let after = evaluate_case(&case).expect("after");
        let before_evsi = golden_experiment(&before, "pressure-differential-probe").evsi;
        let after_evsi = golden_experiment(&after, "pressure-differential-probe").evsi;
        assert!((before_evsi - after_evsi).abs() < 1e-9);
    }

    #[test]
    fn perfect_observation_evsi_does_not_exceed_evpi() {
        let mut case = fixture();
        case["experiments"][0]["outcomes"] = serde_json::json!([
            {"outcome_id":"observe-transport-limited","likelihoods":[
                {"state_id":"transport-limited","probability":1.0},
                {"state_id":"fragmentation-limited","probability":0.0}
            ]},
            {"outcome_id":"observe-fragmentation-limited","likelihoods":[
                {"state_id":"transport-limited","probability":0.0},
                {"state_id":"fragmentation-limited","probability":1.0}
            ]}
        ]);
        let report = evaluate_case(&case).expect("perfect observation");
        let result = golden_experiment(&report, "pressure-differential-probe");
        let evpi = 0.6_f64 * 100.0 + 0.4_f64 * 80.0 - report.current_best_value;
        assert!(result.evsi <= evpi + 1e-9);
        assert!((result.evsi - evpi).abs() < 1e-9);
    }

    #[test]
    fn zero_information_experiment_has_zero_evsi() {
        let mut case = fixture();
        case["experiments"][0]["outcomes"] = serde_json::json!([
            {"outcome_id":"outcome-a","likelihoods":[
                {"state_id":"transport-limited","probability":0.5},
                {"state_id":"fragmentation-limited","probability":0.5}
            ]},
            {"outcome_id":"outcome-b","likelihoods":[
                {"state_id":"transport-limited","probability":0.5},
                {"state_id":"fragmentation-limited","probability":0.5}
            ]}
        ]);
        let report = evaluate_case(&case).expect("zero information experiment");
        let result = golden_experiment(&report, "pressure-differential-probe");
        assert!(result.evsi.abs() < 1e-9);
    }

    #[test]
    fn tie_breaking_is_deterministic() {
        let mut case = fixture();
        case["experiments"][1]["cost"]["value"] = serde_json::json!(5.0);
        case["experiments"][1]["outcomes"] =
            case["experiments"][0]["outcomes"].clone();
        let report = evaluate_case(&case).expect("stable sort");
        assert!((report.experiment_values[0].net_evsi - report.experiment_values[1].net_evsi).abs() < 1e-9);
        assert_eq!(
            report.experiment_values[0].experiment_id,
            "pressure-differential-probe"
        );
    }

    #[test]
    fn replay_is_deterministic() {
        let case = fixture();
        assert_eq!(
            evaluate_case(&case).expect("first"),
            evaluate_case(&case).expect("second")
        );
    }
}
