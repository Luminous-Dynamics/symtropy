//! Exact finite-horizon adaptive passive-measurement and stopping policy.
//!
//! Belief weights remain unnormalized along outcome branches. This avoids
//! silently renormalizing probability mass and makes expected branch utility
//! additive. Each candidate measurement is used at most once per path.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const PROB_TOL: f64 = 1.0e-9;
const MAX_STATES: usize = 8;
const MAX_ACTIONS: usize = 8;
const MAX_EXPERIMENTS: usize = 5;
const MAX_OUTCOMES: usize = 3;
const MAX_HORIZON: usize = 3;
const MAX_NODES: usize = 10_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolicyBranch {
    pub outcome_id: String,
    pub predictive_probability: f64,
    pub child: Box<PolicyNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolicyNode {
    pub node_kind: String,
    pub expected_value_contribution: f64,
    pub belief_mass: f64,
    pub selected_action_id: Option<String>,
    pub selected_experiment_id: Option<String>,
    pub experiment_cost: f64,
    pub stop_value: f64,
    pub continuation_value: Option<f64>,
    pub branches: Vec<PolicyBranch>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub evidence_disposition: String,
    pub utility_unit: String,
    pub current_stop_value: f64,
    pub policy_expected_value: f64,
    pub expected_net_improvement_over_stopping: f64,
    pub selected_first_experiment_id: Option<String>,
    pub stopping_or_review_disposition: String,
    pub remaining_budget: f64,
    pub maximum_measurements: usize,
    pub maximum_horizon: usize,
    pub evaluated_nodes: usize,
    pub policy: PolicyNode,
    pub limitations: Vec<String>,
}

#[derive(Clone)]
struct CompiledAction {
    id: String,
    utility_by_state: Vec<f64>,
}

#[derive(Clone)]
struct CompiledOutcome {
    id: String,
    likelihood_by_state: Vec<f64>,
}

#[derive(Clone)]
struct CompiledExperiment {
    id: String,
    cost: f64,
    independence_group_ref: String,
    outcomes: Vec<CompiledOutcome>,
}

struct CompiledCase {
    case_id: String,
    evidence_disposition: String,
    utility_unit: String,
    priors: Vec<f64>,
    actions: Vec<CompiledAction>,
    experiments: Vec<CompiledExperiment>,
    budget: f64,
    max_measurements: usize,
    horizon: usize,
    minimum_net_value_to_continue: f64,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn valid_evidence_role(role: &str) -> bool {
    matches!(
        role,
        "INDEPENDENT_MEASUREMENT"
            | "EXTERNAL_CALIBRATION"
            | "EXPERT_ELICITATION"
            | "SIMULATION_ONLY"
    )
}

fn validate_evidence(value: &Value, path: &str, errors: &mut Vec<Violation>, synthetic: &mut bool) {
    match text(value, "evidence_role") {
        Some(role) if valid_evidence_role(role) => {
            if role == "SIMULATION_ONLY" {
                *synthetic = true;
            }
        }
        _ => push(
            errors,
            "INVALID_EVIDENCE_ROLE",
            &format!("{path}.evidence_role"),
            "evidence role must be explicit: independent measurement, external calibration, expert elicitation or simulation only",
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

/// Validate all finite-state priors, utilities, likelihoods, costs and limits.
pub fn validate_case(case: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();
    let mut any_synthetic = text(case, "evidence_class") == Some("SYNTHETIC_CONFORMANCE_FIXTURE");

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA_VERSION", "$.schema_version", "schema_version must be exactly 1");
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    if !matches!(
        text(case, "evidence_class"),
        Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")
    ) {
        push(&mut errors, "INVALID_EVIDENCE_CLASS", "$.evidence_class", "evidence class must be SYNTHETIC_CONFORMANCE_FIXTURE or MEASURED");
    }

    let Some(profile) = case.get("decision_profile") else {
        push(&mut errors, "MISSING_DECISION_PROFILE", "$.decision_profile", "decision profile is required");
        return errors;
    };
    for key in ["decision_profile_ref", "decision_owner_ref", "utility_validation_ref", "utility_unit"] {
        if text(profile, key).is_none() {
            push(&mut errors, "MISSING_DECISION_PROFILE_FIELD", &format!("$.decision_profile.{key}"), "decision owner, validated utility profile and unit are required");
        }
    }
    let utility_unit = text(profile, "utility_unit");

    let Some(states) = case.get("states").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_STATES", "$.states", "states must be an array");
        return errors;
    };
    if states.is_empty() || states.len() > MAX_STATES {
        push(&mut errors, "SEARCH_SPACE_LIMIT", "$.states", "v0.1 requires 1..=8 hypotheses");
    }
    let mut state_ids = BTreeSet::new();
    let mut prior_sum = 0.0;
    for (index, state) in states.iter().enumerate() {
        let path = format!("$.states[{index}]");
        match text(state, "state_id") {
            Some(id) if state_ids.insert(id.to_owned()) => {}
            Some(_) => push(&mut errors, "DUPLICATE_STATE_ID", &format!("{path}.state_id"), "state identifiers must be unique"),
            None => push(&mut errors, "MISSING_STATE_ID", &format!("{path}.state_id"), "state_id is required"),
        }
        match finite(state, "prior_probability") {
            Some(p) if (0.0..=1.0).contains(&p) => prior_sum += p,
            _ => push(&mut errors, "INVALID_PRIOR_PROBABILITY", &format!("{path}.prior_probability"), "prior probability must be finite and in [0,1]"),
        }
        match state.get("evidence") {
            Some(evidence) => validate_evidence(evidence, &format!("{path}.evidence"), &mut errors, &mut any_synthetic),
            None => push(&mut errors, "MISSING_PRIOR_EVIDENCE", &format!("{path}.evidence"), "prior distribution requires evidence provenance"),
        }
    }
    if (prior_sum - 1.0).abs() > PROB_TOL {
        push(&mut errors, "PRIOR_NOT_NORMALIZED", "$.states", format!("prior probabilities sum to {prior_sum}, expected 1"));
    }

    let Some(actions) = case.get("actions").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_ACTIONS", "$.actions", "actions must be an array");
        return errors;
    };
    if actions.is_empty() || actions.len() > MAX_ACTIONS {
        push(&mut errors, "SEARCH_SPACE_LIMIT", "$.actions", "v0.1 requires 1..=8 terminal actions");
    }
    let mut action_ids = BTreeSet::new();
    for (ai, action) in actions.iter().enumerate() {
        let path = format!("$.actions[{ai}]");
        let Some(id) = text(action, "action_id") else {
            push(&mut errors, "MISSING_ACTION_ID", &format!("{path}.action_id"), "action_id is required");
            continue;
        };
        if !action_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_ACTION_ID", &format!("{path}.action_id"), "action IDs must be unique");
        }
        let Some(utilities) = action.get("utilities").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_ACTION_UTILITIES", &format!("{path}.utilities"), "utilities must be an array");
            continue;
        };
        let mut seen = BTreeSet::new();
        for (ui, utility) in utilities.iter().enumerate() {
            let upath = format!("{path}.utilities[{ui}]");
            let Some(state_id) = text(utility, "state_id") else {
                push(&mut errors, "MISSING_UTILITY_STATE", &format!("{upath}.state_id"), "utility row requires a state ID");
                continue;
            };
            if !state_ids.contains(state_id) {
                push(&mut errors, "UNKNOWN_UTILITY_STATE", &format!("{upath}.state_id"), "utility row references an undeclared state");
            }
            if !seen.insert(state_id.to_owned()) {
                push(&mut errors, "DUPLICATE_UTILITY_STATE", &format!("{upath}.state_id"), "action may have only one utility per state");
            }
            if finite(utility, "value").is_none() {
                push(&mut errors, "INVALID_UTILITY_VALUE", &format!("{upath}.value"), "utility must be finite");
            }
        }
        for state_id in &state_ids {
            if !seen.contains(state_id) {
                push(&mut errors, "INCOMPLETE_UTILITY_TABLE", &path, format!("action {id} is missing utility for state {state_id}"));
            }
        }
    }

    let Some(experiments) = case.get("experiments").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_EXPERIMENTS", "$.experiments", "experiments must be an array");
        return errors;
    };
    if experiments.is_empty() || experiments.len() > MAX_EXPERIMENTS {
        push(&mut errors, "SEARCH_SPACE_LIMIT", "$.experiments", "v0.1 requires 1..=5 candidate measurements");
    }
    let mut experiment_ids = BTreeSet::new();
    for (ei, experiment) in experiments.iter().enumerate() {
        let path = format!("$.experiments[{ei}]");
        let Some(id) = text(experiment, "experiment_id") else {
            push(&mut errors, "MISSING_EXPERIMENT_ID", &format!("{path}.experiment_id"), "experiment_id is required");
            continue;
        };
        if !experiment_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_EXPERIMENT_ID", &format!("{path}.experiment_id"), "measurement IDs must be unique");
        }
        let Some(cost) = experiment.get("cost") else {
            push(&mut errors, "MISSING_EXPERIMENT_COST", &format!("{path}.cost"), "full measurement cost is required");
            continue;
        };
        match finite(cost, "value") {
            Some(v) if v >= 0.0 => {}
            _ => push(&mut errors, "INVALID_EXPERIMENT_COST", &format!("{path}.cost.value"), "measurement cost must be finite and non-negative"),
        }
        if text(cost, "unit") != utility_unit {
            push(&mut errors, "COST_UNIT_MISMATCH", &format!("{path}.cost.unit"), "measurement cost must use the exact declared decision utility unit");
        }
        for key in ["boundary_ref", "source_ref"] {
            if text(cost, key).is_none() {
                push(&mut errors, "MISSING_COST_PROVENANCE", &format!("{path}.cost.{key}"), "cost boundary and source references are required");
            }
        }
        if text(experiment, "domain_ref").is_none() {
            push(&mut errors, "MISSING_EXPERIMENT_DOMAIN", &format!("{path}.domain_ref"), "measurement likelihood domain is required");
        }
        for key in ["independence_group_ref", "dependence_evidence_ref"] {
            if text(experiment, key).is_none() {
                push(&mut errors, "MISSING_EXPERIMENT_DEPENDENCE_EVIDENCE", &format!("{path}.{key}"), "measurement independence group and dependence evidence are required");
            }
        }
        match experiment.get("likelihood_evidence") {
            Some(evidence) => validate_evidence(evidence, &format!("{path}.likelihood_evidence"), &mut errors, &mut any_synthetic),
            None => push(&mut errors, "MISSING_LIKELIHOOD_EVIDENCE", &format!("{path}.likelihood_evidence"), "likelihood requires provenance"),
        }
        let Some(outcomes) = experiment.get("outcomes").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_EXPERIMENT_OUTCOMES", &format!("{path}.outcomes"), "outcomes must be an array");
            continue;
        };
        if outcomes.is_empty() || outcomes.len() > MAX_OUTCOMES {
            push(&mut errors, "SEARCH_SPACE_LIMIT", &format!("{path}.outcomes"), "v0.1 permits 1..=3 outcomes per measurement");
        }
        let mut outcome_ids = BTreeSet::new();
        let mut likelihood_sums: BTreeMap<String, f64> = state_ids.iter().map(|s| (s.clone(), 0.0)).collect();
        for (oi, outcome) in outcomes.iter().enumerate() {
            let opath = format!("{path}.outcomes[{oi}]");
            let Some(outcome_id) = text(outcome, "outcome_id") else {
                push(&mut errors, "MISSING_OUTCOME_ID", &format!("{opath}.outcome_id"), "outcome_id is required");
                continue;
            };
            if !outcome_ids.insert(outcome_id.to_owned()) {
                push(&mut errors, "DUPLICATE_OUTCOME_ID", &format!("{opath}.outcome_id"), "outcome IDs must be unique within a measurement");
            }
            let Some(likelihoods) = outcome.get("likelihoods").and_then(Value::as_array) else {
                push(&mut errors, "MISSING_OUTCOME_LIKELIHOODS", &format!("{opath}.likelihoods"), "likelihood rows are required");
                continue;
            };
            let mut seen_states = BTreeSet::new();
            for (li, likelihood) in likelihoods.iter().enumerate() {
                let lpath = format!("{opath}.likelihoods[{li}]");
                let Some(state_id) = text(likelihood, "state_id") else {
                    push(&mut errors, "MISSING_LIKELIHOOD_STATE", &format!("{lpath}.state_id"), "likelihood row requires a state ID");
                    continue;
                };
                if !state_ids.contains(state_id) {
                    push(&mut errors, "UNKNOWN_LIKELIHOOD_STATE", &format!("{lpath}.state_id"), "likelihood references an undeclared state");
                }
                if !seen_states.insert(state_id.to_owned()) {
                    push(&mut errors, "DUPLICATE_LIKELIHOOD_STATE", &format!("{lpath}.state_id"), "one likelihood row per state/outcome is permitted");
                }
                match finite(likelihood, "probability") {
                    Some(p) if (0.0..=1.0).contains(&p) => {
                        if let Some(sum) = likelihood_sums.get_mut(state_id) {
                            *sum += p;
                        }
                    }
                    _ => push(&mut errors, "INVALID_LIKELIHOOD_PROBABILITY", &format!("{lpath}.probability"), "likelihood must be finite and in [0,1]"),
                }
            }
            for state_id in &state_ids {
                if !seen_states.contains(state_id) {
                    push(&mut errors, "INCOMPLETE_LIKELIHOOD_TABLE", &opath, format!("outcome {outcome_id} is missing likelihood for state {state_id}"));
                }
            }
        }
        for (state_id, sum) in likelihood_sums {
            if (sum - 1.0).abs() > PROB_TOL {
                push(&mut errors, "LIKELIHOOD_NOT_NORMALIZED", &path, format!("measurement {id} likelihoods for state {state_id} sum to {sum}, expected 1"));
            }
        }
    }

    let Some(policy) = case.get("policy") else {
        push(&mut errors, "MISSING_POLICY", "$.policy", "finite-horizon policy settings are required");
        return errors;
    };
    for key in ["stop_consequence_ref", "human_review_authority_ref", "conditional_independence_assumption_ref"] {
        if text(policy, key).is_none() {
            push(&mut errors, "MISSING_POLICY_PROVENANCE", &format!("$.policy.{key}"), "stopping consequence, human review authority and explicit conditional-independence assumption are required");
        }
    }
    match finite(policy, "remaining_budget") {
        Some(v) if v >= 0.0 => {}
        _ => push(&mut errors, "INVALID_REMAINING_BUDGET", "$.policy.remaining_budget", "remaining budget must be finite and non-negative"),
    }
    if text(policy, "budget_unit") != utility_unit {
        push(&mut errors, "BUDGET_UNIT_MISMATCH", "$.policy.budget_unit", "budget must use the exact utility unit");
    }
    let max_steps = policy.get("maximum_measurements_remaining").and_then(Value::as_u64);
    if max_steps.is_none_or(|n| n as usize > MAX_HORIZON || n as usize > experiments.len()) {
        push(&mut errors, "SEARCH_SPACE_LIMIT", "$.policy.maximum_measurements_remaining", "maximum measurements must be within horizon and candidate count");
    }
    let horizon = policy.get("maximum_horizon").and_then(Value::as_u64);
    if horizon.is_none_or(|n| n as usize > MAX_HORIZON || n as usize > max_steps.unwrap_or(0) as usize) {
        push(&mut errors, "SEARCH_SPACE_LIMIT", "$.policy.maximum_horizon", "horizon must be 0..=3 and no greater than maximum measurements");
    }
    match finite(policy, "minimum_net_value_to_continue") {
        Some(v) if v >= 0.0 => {}
        _ => push(&mut errors, "INVALID_CONTINUATION_THRESHOLD", "$.policy.minimum_net_value_to_continue", "continuation threshold must be finite and non-negative"),
    }

    if text(case, "evidence_class") == Some("MEASURED") && any_synthetic {
        push(
            &mut errors,
            "MEASURED_CASE_CONTAINS_SIMULATION_ONLY_EVIDENCE",
            "$.evidence_class",
            "a MEASURED case cannot contain a SIMULATION_ONLY prior or likelihood source",
        );
    }
    errors
}

fn compile_case(case: &Value) -> CompiledCase {
    let mut states: Vec<&Value> = case["states"].as_array().unwrap().iter().collect();
    states.sort_by(|a, b| text(a, "state_id").unwrap().cmp(text(b, "state_id").unwrap()));
    let state_ids: Vec<String> = states.iter().map(|s| text(s, "state_id").unwrap().to_owned()).collect();
    let state_index: BTreeMap<&str, usize> = state_ids.iter().enumerate().map(|(i, id)| (id.as_str(), i)).collect();
    let priors: Vec<f64> = states.iter().map(|s| finite(s, "prior_probability").unwrap()).collect();

    let mut actions: Vec<&Value> = case["actions"].as_array().unwrap().iter().collect();
    actions.sort_by(|a, b| text(a, "action_id").unwrap().cmp(text(b, "action_id").unwrap()));
    let compiled_actions = actions.iter().map(|action| {
        let mut utility = vec![0.0; state_ids.len()];
        for row in action["utilities"].as_array().unwrap() {
            utility[state_index[text(row, "state_id").unwrap()]] = finite(row, "value").unwrap();
        }
        CompiledAction {
            id: text(action, "action_id").unwrap().to_owned(),
            utility_by_state: utility,
        }
    }).collect();

    let mut experiments: Vec<&Value> = case["experiments"].as_array().unwrap().iter().collect();
    experiments.sort_by(|a, b| text(a, "experiment_id").unwrap().cmp(text(b, "experiment_id").unwrap()));
    let compiled_experiments = experiments.iter().map(|experiment| {
        let mut outcomes: Vec<&Value> = experiment["outcomes"].as_array().unwrap().iter().collect();
        outcomes.sort_by(|a, b| text(a, "outcome_id").unwrap().cmp(text(b, "outcome_id").unwrap()));
        let compiled_outcomes = outcomes.iter().map(|outcome| {
            let mut likelihood = vec![0.0; state_ids.len()];
            for row in outcome["likelihoods"].as_array().unwrap() {
                likelihood[state_index[text(row, "state_id").unwrap()]] = finite(row, "probability").unwrap();
            }
            CompiledOutcome {
                id: text(outcome, "outcome_id").unwrap().to_owned(),
                likelihood_by_state: likelihood,
            }
        }).collect();
        CompiledExperiment {
            id: text(experiment, "experiment_id").unwrap().to_owned(),
            cost: finite(&experiment["cost"], "value").unwrap(),
            independence_group_ref: text(experiment, "independence_group_ref").unwrap().to_owned(),
            outcomes: compiled_outcomes,
        }
    }).collect();

    let policy = &case["policy"];
    CompiledCase {
        case_id: text(case, "case_id").unwrap().to_owned(),
        evidence_disposition: if any_synthetic_evidence(case) { "SYNTHETIC_ONLY" } else { "MEASURED_EVIDENCE_REVIEW_REQUIRED" }.to_owned(),
        utility_unit: text(&case["decision_profile"], "utility_unit").unwrap().to_owned(),
        priors,
        actions: compiled_actions,
        experiments: compiled_experiments,
        budget: finite(policy, "remaining_budget").unwrap(),
        max_measurements: policy["maximum_measurements_remaining"].as_u64().unwrap() as usize,
        horizon: policy["maximum_horizon"].as_u64().unwrap() as usize,
        minimum_net_value_to_continue: finite(policy, "minimum_net_value_to_continue").unwrap(),
    }
}

fn any_synthetic_evidence(case: &Value) -> bool {
    if text(case, "evidence_class") == Some("SYNTHETIC_CONFORMANCE_FIXTURE") {
        return true;
    }
    case["states"].as_array().into_iter().flatten().any(|s| {
        text(&s["evidence"], "evidence_role") == Some("SIMULATION_ONLY")
    }) || case["experiments"].as_array().into_iter().flatten().any(|e| {
        text(&e["likelihood_evidence"], "evidence_role") == Some("SIMULATION_ONLY")
    })
}

struct Solver<'a> {
    compiled: &'a CompiledCase,
    nodes: usize,
}

impl<'a> Solver<'a> {
    fn stop_node(&self, weights: &[f64]) -> (f64, String) {
        let mut best_value = f64::NEG_INFINITY;
        let mut best_action = String::new();
        for action in &self.compiled.actions {
            let value: f64 = weights.iter()
                .zip(action.utility_by_state.iter())
                .map(|(weight, utility)| weight * utility)
                .sum();
            if value > best_value {
                best_value = value;
                best_action.clone_from(&action.id);
            }
        }
        (best_value, best_action)
    }

    fn solve(
        &mut self,
        weights: &[f64],
        remaining_budget: f64,
        remaining_steps: usize,
        remaining_horizon: usize,
        used_groups: &BTreeSet<String>,
    ) -> Result<PolicyNode, Violation> {
        self.nodes += 1;
        if self.nodes > MAX_NODES {
            return Err(Violation {
                code: "SEARCH_SPACE_LIMIT".to_owned(),
                path: "$.policy".to_owned(),
                detail: format!("exact policy search exceeded {MAX_NODES} nodes"),
            });
        }
        let mass: f64 = weights.iter().sum();
        if !mass.is_finite() || weights.iter().any(|w| !w.is_finite() || *w < 0.0) {
            return Err(Violation {
                code: "INVALID_BELIEF_MASS".to_owned(),
                path: "$.policy".to_owned(),
                detail: "belief weights must remain finite and non-negative".to_owned(),
            });
        }
        let (stop_value, stop_action) = self.stop_node(weights);
        let mut best_node = PolicyNode {
            node_kind: "STOP".to_owned(),
            expected_value_contribution: stop_value,
            belief_mass: mass,
            selected_action_id: Some(stop_action),
            selected_experiment_id: None,
            experiment_cost: 0.0,
            stop_value,
            continuation_value: None,
            branches: Vec::new(),
        };

        if mass == 0.0 || remaining_steps == 0 || remaining_horizon == 0 {
            return Ok(best_node);
        }

        for experiment_index in 0..self.compiled.experiments.len() {
            let experiment = self.compiled.experiments[experiment_index].clone();
            if used_groups.contains(&experiment.independence_group_ref) {
                continue;
            }
            if experiment.cost > remaining_budget {
                continue;
            }
            let child_budget = remaining_budget - experiment.cost;
            let mut child_used_groups = used_groups.clone();
            child_used_groups.insert(experiment.independence_group_ref.clone());
            let mut branches = Vec::new();
            let mut expected_children = 0.0;
            for outcome in &experiment.outcomes {
                let mut child_weights = Vec::with_capacity(weights.len());
                let mut predictive_mass = 0.0;
                for (weight, likelihood) in weights.iter().zip(outcome.likelihood_by_state.iter()) {
                    let joint = weight * likelihood;
                    if *weight > 0.0 && *likelihood > 0.0 && joint == 0.0 {
                        return Err(Violation {
                            code: "PROBABILITY_UNDERFLOW".to_owned(),
                            path: format!("$.experiments[{}].outcomes[{}]", experiment.id, outcome.id),
                            detail: "positive belief × positive likelihood underflowed to zero; no silent impossible-outcome substitution is allowed".to_owned(),
                        });
                    }
                    if !joint.is_finite() {
                        return Err(Violation {
                            code: "NONFINITE_POSTERIOR_WEIGHT".to_owned(),
                            path: format!("$.experiments[{}].outcomes[{}]", experiment.id, outcome.id),
                            detail: "joint posterior weight is not finite".to_owned(),
                        });
                    }
                    child_weights.push(joint);
                    predictive_mass += joint;
                }
                if predictive_mass == 0.0 {
                    // Unreachable outcome under the current belief. No posterior
                    // is formed and no policy branch is required for it.
                    continue;
                }
                let child = self.solve(
                    &child_weights,
                    child_budget,
                    remaining_steps - 1,
                    remaining_horizon - 1,
                    &child_used_groups,
                )?;
                expected_children += child.expected_value_contribution;
                branches.push(PolicyBranch {
                    outcome_id: outcome.id.clone(),
                    predictive_probability: predictive_mass / mass,
                    child: Box::new(child),
                });
            }
            let continuation_value = expected_children - mass * experiment.cost;
            let required_improvement = mass * self.compiled.minimum_net_value_to_continue;
            if continuation_value > stop_value + required_improvement
                && continuation_value > best_node.expected_value_contribution
            {
                best_node = PolicyNode {
                    node_kind: "MEASURE".to_owned(),
                    expected_value_contribution: continuation_value,
                    belief_mass: mass,
                    selected_action_id: None,
                    selected_experiment_id: Some(experiment.id.clone()),
                    experiment_cost: experiment.cost,
                    stop_value,
                    continuation_value: Some(continuation_value),
                    branches,
                };
            }
        }
        Ok(best_node)
    }
}

/// Evaluate an exact finite-horizon passive measurement/stop policy.
pub fn evaluate_case(case: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case);
    if !errors.is_empty() {
        return Err(errors);
    }
    let compiled = compile_case(case);
    let mut solver = Solver { compiled: &compiled, nodes: 0 };
    let policy = solver.solve(
        &compiled.priors,
        compiled.budget,
        compiled.max_measurements,
        compiled.horizon,
        &BTreeSet::new(),
    ).map_err(|error| vec![error])?;
    let current_stop_value = solver.stop_node(&compiled.priors).0;
    let expected_net_improvement = policy.expected_value_contribution - current_stop_value;
    let selected_first_experiment_id = policy.selected_experiment_id.clone();
    let stopping_or_review_disposition = if compiled.evidence_disposition == "SYNTHETIC_ONLY" {
        "SYNTHETIC_ONLY_NO_DECISION_AUTHORITY"
    } else if selected_first_experiment_id.is_some() {
        "PASSIVE_MEASUREMENT_CANDIDATE_HUMAN_REVIEW_REQUIRED"
    } else {
        "STOP_OR_HUMAN_REVIEW"
    };

    Ok(EvaluationReport {
        case_id: compiled.case_id,
        evidence_disposition: compiled.evidence_disposition,
        utility_unit: compiled.utility_unit,
        current_stop_value,
        policy_expected_value: policy.expected_value_contribution,
        expected_net_improvement_over_stopping: expected_net_improvement,
        selected_first_experiment_id,
        stopping_or_review_disposition: stopping_or_review_disposition.to_owned(),
        remaining_budget: compiled.budget,
        maximum_measurements: compiled.max_measurements,
        maximum_horizon: compiled.horizon,
        evaluated_nodes: solver.nodes,
        policy,
        limitations: vec![
            "Exact finite-horizon policy only; not a general POMDP or unbounded sequential solver".to_owned(),
            "Candidate measurements are passive; no machine action or physical intervention is issued".to_owned(),
            "Synthetic priors, likelihoods, utilities and costs never become decision-qualified evidence".to_owned(),
            "Likelihood dependence across repeated measurements is not modeled; each experiment may be used at most once per policy path".to_owned(),
            "Human and competent engineering review remain required before any real-world measurement campaign".to_owned(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-sequential-diagnostic-case.v0_1.json"
        )).expect("fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn measure_branch<'a>(node: &'a PolicyNode, outcome: &str) -> &'a PolicyNode {
        node.branches.iter().find(|branch| branch.outcome_id == outcome).unwrap().child.as_ref()
    }

    #[test]
    fn synthetic_adaptive_fixture_is_valid() {
        assert!(validate_case(&fixture()).is_empty());
    }

    #[test]
    fn coarse_measurement_then_outcome_specific_second_measurement() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(report.selected_first_experiment_id.as_deref(), Some("coarse-passive-observation"));
        assert_eq!(report.policy.node_kind, "MEASURE");
        assert_eq!(measure_branch(&report.policy, "A-like").node_kind, "STOP");
        assert_eq!(
            measure_branch(&report.policy, "B-or-C-like").selected_experiment_id.as_deref(),
            Some("B-vs-C-passive-sample")
        );
        assert!(report.policy_expected_value > report.current_stop_value);
    }

    #[test]
    fn synthetic_case_never_becomes_decision_qualified() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY");
        assert_eq!(report.stopping_or_review_disposition, "SYNTHETIC_ONLY_NO_DECISION_AUTHORITY");
    }

    #[test]
    fn zero_information_positive_cost_measurement_is_not_selected() {
        let mut case = fixture();
        case["experiments"].as_array_mut().unwrap().truncate(1);
        case["policy"]["maximum_measurements_remaining"] = serde_json::json!(1);
        case["policy"]["maximum_horizon"] = serde_json::json!(1);
        let outcomes = case["experiments"][0]["outcomes"].as_array_mut().unwrap();
        outcomes[0]["likelihoods"] = serde_json::json!([
            {"state_id":"A-fragmentation-dominant","probability":0.5},
            {"state_id":"B-transport-restriction","probability":0.5},
            {"state_id":"C-filter-loading","probability":0.5}
        ]);
        outcomes[1]["likelihoods"] = serde_json::json!([
            {"state_id":"A-fragmentation-dominant","probability":0.5},
            {"state_id":"B-transport-restriction","probability":0.5},
            {"state_id":"C-filter-loading","probability":0.5}
        ]);
        let report = evaluate_case(&case).unwrap();
        assert!(report.selected_first_experiment_id.is_none());
        assert_eq!(report.policy.node_kind, "STOP");
    }

    #[test]
    fn perfect_information_upper_bound_is_not_exceeded() {
        let case = fixture();
        let report = evaluate_case(&case).unwrap();
        let states = case["states"].as_array().unwrap();
        let actions = case["actions"].as_array().unwrap();
        let perfect: f64 = states.iter().map(|state| {
            let id = state["state_id"].as_str().unwrap();
            let best = actions.iter().map(|action| {
                action["utilities"].as_array().unwrap().iter()
                    .find(|utility| utility["state_id"] == id).unwrap()["value"].as_f64().unwrap()
            }).fold(f64::NEG_INFINITY, f64::max);
            state["prior_probability"].as_f64().unwrap() * best
        }).sum();
        assert!(report.policy_expected_value <= perfect + 1.0e-9);
    }

    #[test]
    fn no_measurement_is_reused_on_any_policy_path() {
        fn visit(node: &PolicyNode, used: &mut Vec<String>) {
            if let Some(id) = &node.selected_experiment_id {
                assert!(!used.contains(id));
                used.push(id.clone());
            }
            for branch in &node.branches {
                visit(&branch.child, used);
            }
            if node.node_kind == "MEASURE" {
                used.pop();
            }
        }
        let report = evaluate_case(&fixture()).unwrap();
        visit(&report.policy, &mut Vec::new());
    }

    #[test]
    fn cost_is_paid_only_on_branches_where_measurement_is_selected() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(measure_branch(&report.policy, "A-like").node_kind, "STOP");
        assert_eq!(measure_branch(&report.policy, "B-or-C-like").node_kind, "MEASURE");
        assert!(report.policy.branches.iter().all(|branch| branch.predictive_probability > 0.0));
    }

    #[test]
    fn action_ids_and_outcome_ids_are_deterministically_sorted() {
        let mut case = fixture();
        case["actions"].as_array_mut().unwrap().reverse();
        case["states"].as_array_mut().unwrap().reverse();
        case["experiments"].as_array_mut().unwrap().reverse();
        for experiment in case["experiments"].as_array_mut().unwrap() {
            experiment["outcomes"].as_array_mut().unwrap().reverse();
        }
        let report_a = evaluate_case(&fixture()).unwrap();
        let report_b = evaluate_case(&case).unwrap();
        assert!((report_a.policy_expected_value - report_b.policy_expected_value).abs() < 1.0e-9);
        assert_eq!(report_a.selected_first_experiment_id, report_b.selected_first_experiment_id);
        assert_eq!(report_a.policy, report_b.policy);
    }

    #[test]
    fn one_step_horizon_matches_direct_evsi_recurrence() {
        let mut case = fixture();
        case["policy"]["maximum_measurements_remaining"] = serde_json::json!(1);
        case["policy"]["maximum_horizon"] = serde_json::json!(1);
        let report = evaluate_case(&case).unwrap();

        let states = case["states"].as_array().unwrap();
        let actions = case["actions"].as_array().unwrap();
        let priors: Vec<f64> = states.iter().map(|s| s["prior_probability"].as_f64().unwrap()).collect();
        let current = actions.iter().map(|action| {
            action["utilities"].as_array().unwrap().iter().zip(priors.iter())
                .map(|(u, p)| u["value"].as_f64().unwrap() * p)
                .sum::<f64>()
        }).fold(f64::NEG_INFINITY, f64::max);

        let mut best = current;
        for experiment in case["experiments"].as_array().unwrap() {
            let mut expected_after = 0.0;
            for outcome in experiment["outcomes"].as_array().unwrap() {
                let likelihoods: Vec<f64> = outcome["likelihoods"].as_array().unwrap()
                    .iter().map(|l| l["probability"].as_f64().unwrap()).collect();
                let joint: Vec<f64> = priors.iter().zip(likelihoods.iter()).map(|(p, l)| p * l).collect();
                let outcome_value = actions.iter().map(|action| {
                    action["utilities"].as_array().unwrap().iter().zip(joint.iter())
                        .map(|(u, w)| u["value"].as_f64().unwrap() * w)
                        .sum::<f64>()
                }).fold(f64::NEG_INFINITY, f64::max);
                expected_after += outcome_value;
            }
            let net = expected_after - experiment["cost"]["value"].as_f64().unwrap();
            if net > best {
                best = net;
            }
        }
        assert!((report.policy_expected_value - best).abs() < 1.0e-9);
    }

    #[test]
    fn prior_must_be_normalized() {
        let mut case = fixture();
        case["states"][0]["prior_probability"] = serde_json::json!(0.5);
        assert!(has(&validate_case(&case), "PRIOR_NOT_NORMALIZED"));
    }

    #[test]
    fn utility_table_must_be_complete() {
        let mut case = fixture();
        case["actions"][0]["utilities"].as_array_mut().unwrap().pop();
        assert!(has(&validate_case(&case), "INCOMPLETE_UTILITY_TABLE"));
    }

    #[test]
    fn likelihoods_must_be_normalized_per_state() {
        let mut case = fixture();
        case["experiments"][0]["outcomes"][0]["likelihoods"][0]["probability"] =
            serde_json::json!(0.8);
        assert!(has(&validate_case(&case), "LIKELIHOOD_NOT_NORMALIZED"));
    }

    #[test]
    fn duplicate_experiment_id_is_rejected() {
        let mut case = fixture();
        let clone = case["experiments"][0].clone();
        case["experiments"].as_array_mut().unwrap().push(clone);
        assert!(has(&validate_case(&case), "DUPLICATE_EXPERIMENT_ID"));
    }

    #[test]
    fn duplicate_outcome_id_is_rejected() {
        let mut case = fixture();
        case["experiments"][0]["outcomes"][1]["outcome_id"] =
            case["experiments"][0]["outcomes"][0]["outcome_id"].clone();
        assert!(has(&validate_case(&case), "DUPLICATE_OUTCOME_ID"));
    }

    #[test]
    fn cost_unit_must_match_utility_unit() {
        let mut case = fixture();
        case["experiments"][0]["cost"]["unit"] = serde_json::json!("USD");
        assert!(has(&validate_case(&case), "COST_UNIT_MISMATCH"));
    }

    #[test]
    fn budget_unit_must_match_utility_unit() {
        let mut case = fixture();
        case["policy"]["budget_unit"] = serde_json::json!("hours");
        assert!(has(&validate_case(&case), "BUDGET_UNIT_MISMATCH"));
    }

    #[test]
    fn oversized_horizon_is_rejected_not_approximated() {
        let mut case = fixture();
        case["policy"]["maximum_horizon"] = serde_json::json!(4);
        case["policy"]["maximum_measurements_remaining"] = serde_json::json!(4);
        assert!(has(&validate_case(&case), "SEARCH_SPACE_LIMIT"));
    }

    #[test]
    fn zero_budget_forces_stop() {
        let mut case = fixture();
        case["policy"]["remaining_budget"] = serde_json::json!(0.0);
        let report = evaluate_case(&case).unwrap();
        assert!(report.selected_first_experiment_id.is_none());
    }

    #[test]
    fn zero_horizon_forces_stop() {
        let mut case = fixture();
        case["policy"]["maximum_horizon"] = serde_json::json!(0);
        case["policy"]["maximum_measurements_remaining"] = serde_json::json!(0);
        let report = evaluate_case(&case).unwrap();
        assert_eq!(report.policy.node_kind, "STOP");
    }

    #[test]
    fn impossible_outcomes_are_skipped_without_posterior_renormalization() {
        let mut case = fixture();
        case["experiments"][0]["outcomes"][0]["likelihoods"][0]["probability"] =
            serde_json::json!(1.0);
        case["experiments"][0]["outcomes"][0]["likelihoods"][1]["probability"] =
            serde_json::json!(0.0);
        case["experiments"][0]["outcomes"][0]["likelihoods"][2]["probability"] =
            serde_json::json!(0.0);
        case["experiments"][0]["outcomes"][1]["likelihoods"][0]["probability"] =
            serde_json::json!(0.0);
        case["experiments"][0]["outcomes"][1]["likelihoods"][1]["probability"] =
            serde_json::json!(1.0);
        case["experiments"][0]["outcomes"][1]["likelihoods"][2]["probability"] =
            serde_json::json!(1.0);
        let report = evaluate_case(&case).unwrap();
        assert!(report.policy_expected_value.is_finite());
    }

    #[test]
    fn same_measurement_independence_group_cannot_be_used_twice_on_a_path() {
        let mut case = fixture();
        let group = case["experiments"][0]["independence_group_ref"].clone();
        case["experiments"][1]["independence_group_ref"] = group;
        let report = evaluate_case(&case).unwrap();

        fn assert_unique_groups(node: &PolicyNode, inherited: &mut BTreeSet<String>, case: &Value) {
            if let Some(id) = &node.selected_experiment_id {
                let group = case["experiments"].as_array().unwrap().iter()
                    .find(|experiment| experiment["experiment_id"] == id).unwrap()["independence_group_ref"]
                    .as_str().unwrap().to_owned();
                assert!(inherited.insert(group.clone()));
                for branch in &node.branches {
                    let mut child_groups = inherited.clone();
                    assert_unique_groups(&branch.child, &mut child_groups, case);
                }
                inherited.remove(&group);
            }
        }
        assert_unique_groups(&report.policy, &mut BTreeSet::new(), &case);
    }

    #[test]
    fn measured_case_cannot_contain_simulation_only_likelihoods() {
        let mut case = fixture();
        case["evidence_class"] = serde_json::json!("MEASURED");
        assert!(has(
            &validate_case(&case),
            "MEASURED_CASE_CONTAINS_SIMULATION_ONLY_EVIDENCE"
        ));
    }

    #[test]
    fn deterministic_replay() {
        let case = fixture();
        assert_eq!(evaluate_case(&case).unwrap(), evaluate_case(&case).unwrap());
    }
}
