// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact finite-horizon inspection planner for small synthetic REBCO reel grids.
//!
//! The planner minimizes inspection cost plus terminal application decision loss.
//! It returns a contingent policy tree and never executes a measurement, releases
//! a reel, or asserts that synthetic priors / likelihoods are physically calibrated.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const SCHEMA_VERSION: &str = "rebco-sequential-inspection-dp.v0.1";
const CLAIM_CEILING: &str = "finite-horizon-exact-dp-on-small-synthetic-grids-only";
const DEFAULT_FIXTURE: &str =
    "docs/research/fixtures/rebco_sequential_inspection_dp_synthetic_v0_1.json";
const EPSILON: f64 = 1.0e-12;
const MAX_ACTIONS: usize = 16;
const MAX_HORIZON: usize = 6;

#[derive(Debug, Clone)]
struct Profile {
    profile_id: String,
    condition_profile_id: String,
    minimum_contiguous_weak_cells: usize,
    false_accept_loss: f64,
    false_reject_loss: f64,
    total_budget_units: u64,
}

#[derive(Debug, Clone)]
struct Scenario {
    id: String,
    prior_weight: f64,
    weak_cells: Vec<bool>,
    application_failure: bool,
}

#[derive(Debug, Clone)]
struct Action {
    id: String,
    method: String,
    cell_index: usize,
    sensitivity: f64,
    specificity: f64,
    cost_units: u64,
    instrument_id: String,
    calibration_revision: String,
    validation_artifact_sha256: String,
}

#[derive(Debug, Clone)]
struct Plan {
    expected_total_loss: f64,
    selected_action_index: Option<usize>,
    terminal_disposition: Option<&'static str>,
    terminal_decision_loss: Option<f64>,
    probability_positive: Option<f64>,
    positive_child: Option<Box<Plan>>,
    negative_child: Option<Box<Plan>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct StateKey {
    posterior_weight_bits: Vec<u64>,
    available_action_mask: u64,
    remaining_budget_units: u64,
    remaining_horizon: usize,
}

#[derive(Debug)]
struct SolveResult {
    plan: Plan,
    explored_state_count: usize,
}

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| fail(path, "must be an array"))
}

fn exact_keys(value: &Value, required: &[&str], path: &str) -> Result<(), String> {
    let map = object(value, path)?;
    for key in required {
        if !map.contains_key(*key) {
            return Err(fail(path, format!("missing property {key}")));
        }
    }
    if let Some(key) = map.keys().find(|key| !required.contains(&key.as_str())) {
        return Err(fail(path, format!("unknown property {key}")));
    }
    Ok(())
}

fn text_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(path, format!("{key} must be a non-empty string")))
}

fn number_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| fail(path, format!("{key} must be finite")))?;
    if !number.is_finite() {
        return Err(fail(path, format!("{key} must be finite")));
    }
    Ok(number)
}

fn positive_number(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = number_field(value, key, path)?;
    if number <= 0.0 {
        return Err(fail(path, format!("{key} must be positive")));
    }
    Ok(number)
}

fn usize_field(value: &Value, key: &str, path: &str) -> Result<usize, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|number| usize::try_from(number).ok())
        .ok_or_else(|| fail(path, format!("{key} must be a non-negative integer")))
}

fn u64_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| fail(path, format!("{key} must be a non-negative integer")))
}

fn validate_digest(value: &str, path: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(fail(path, "must be lowercase 64-character SHA-256 hex"));
    }
    Ok(())
}

fn parse_profile(value: &Value) -> Result<Profile, String> {
    const KEYS: &[&str] = &[
        "profile_id",
        "condition_profile_id",
        "minimum_contiguous_weak_cells",
        "false_accept_loss",
        "false_reject_loss",
        "total_budget_units",
    ];
    exact_keys(value, KEYS, "$.application_profile")?;
    let minimum_contiguous_weak_cells =
        usize_field(value, "minimum_contiguous_weak_cells", "$.application_profile")?;
    if minimum_contiguous_weak_cells == 0 {
        return Err(fail(
            "$.application_profile",
            "minimum_contiguous_weak_cells must be at least one",
        ));
    }
    Ok(Profile {
        profile_id: text_field(value, "profile_id", "$.application_profile")?.to_owned(),
        condition_profile_id: text_field(
            value,
            "condition_profile_id",
            "$.application_profile",
        )?
        .to_owned(),
        minimum_contiguous_weak_cells,
        false_accept_loss: positive_number(
            value,
            "false_accept_loss",
            "$.application_profile",
        )?,
        false_reject_loss: positive_number(
            value,
            "false_reject_loss",
            "$.application_profile",
        )?,
        total_budget_units: u64_field(value, "total_budget_units", "$.application_profile")?,
    })
}

fn maximum_true_run(cells: &[bool]) -> usize {
    let mut current = 0;
    let mut maximum = 0;
    for weak in cells {
        if *weak {
            current += 1;
            maximum = maximum.max(current);
        } else {
            current = 0;
        }
    }
    maximum
}

fn parse_scenarios(
    values: &[Value],
    cell_count: usize,
    minimum_run: usize,
) -> Result<Vec<Scenario>, String> {
    if values.len() < 2 {
        return Err(fail("$.prior_scenarios", "must contain at least two scenarios"));
    }
    let mut ids = BTreeSet::new();
    let mut scenarios = Vec::with_capacity(values.len());
    let mut total_weight = 0.0;
    for (index, value) in values.iter().enumerate() {
        let path = format!("$.prior_scenarios[{index}]");
        exact_keys(value, &["scenario_id", "prior_weight", "weak_cells"], &path)?;
        let id = text_field(value, "scenario_id", &path)?.to_owned();
        if !ids.insert(id.clone()) {
            return Err(fail(&path, "duplicate scenario_id"));
        }
        let prior_weight = positive_number(value, "prior_weight", &path)?;
        let cell_values = array(
            value
                .get("weak_cells")
                .ok_or_else(|| fail(&path, "missing weak_cells"))?,
            &format!("{path}.weak_cells"),
        )?;
        if cell_values.len() != cell_count {
            return Err(fail(
                &path,
                format!(
                    "weak_cells length {} does not match cell_count {cell_count}",
                    cell_values.len()
                ),
            ));
        }
        let mut weak_cells = Vec::with_capacity(cell_count);
        for (cell_index, cell) in cell_values.iter().enumerate() {
            weak_cells.push(
                cell.as_bool().ok_or_else(|| {
                    fail(
                        &path,
                        format!("weak_cells[{cell_index}] must be boolean"),
                    )
                })?,
            );
        }
        total_weight += prior_weight;
        let application_failure = maximum_true_run(&weak_cells) >= minimum_run;
        scenarios.push(Scenario {
            id,
            prior_weight,
            weak_cells,
            application_failure,
        });
    }
    if (total_weight - 1.0).abs() > 1.0e-9 {
        return Err(fail(
            "$.prior_scenarios",
            format!("prior weights must sum to 1 (observed {total_weight})"),
        ));
    }
    Ok(scenarios)
}

fn parse_actions(
    values: &[Value],
    profile: &Profile,
    cell_count: usize,
) -> Result<Vec<Action>, String> {
    if values.is_empty() {
        return Err(fail("$.candidate_actions", "must contain at least one action"));
    }
    if values.len() > MAX_ACTIONS {
        return Err(fail(
            "$.candidate_actions",
            format!("v0.1 exact planner supports at most {MAX_ACTIONS} actions"),
        ));
    }
    const KEYS: &[&str] = &[
        "action_id",
        "method",
        "cell_index",
        "condition_profile_id",
        "sensitivity",
        "specificity",
        "cost_units",
        "calibration_status",
        "instrument_id",
        "calibration_revision",
        "validation_artifact_sha256",
    ];
    let mut ids = BTreeSet::new();
    let mut actions = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let path = format!("$.candidate_actions[{index}]");
        exact_keys(value, KEYS, &path)?;
        let id = text_field(value, "action_id", &path)?.to_owned();
        if !ids.insert(id.clone()) {
            return Err(fail(&path, "duplicate action_id"));
        }
        let method = text_field(value, "method", &path)?.to_owned();
        if !["direct_transport", "calibrated_noncontact_proxy"].contains(&method.as_str()) {
            return Err(fail(
                &path,
                "method must be direct_transport or calibrated_noncontact_proxy",
            ));
        }
        if text_field(value, "condition_profile_id", &path)?
            != profile.condition_profile_id
        {
            return Err(fail(
                &path,
                "action condition profile does not match the application profile",
            ));
        }
        let cell_index = usize_field(value, "cell_index", &path)?;
        if cell_index >= cell_count {
            return Err(fail(&path, "cell_index lies outside the reel grid"));
        }
        let sensitivity = number_field(value, "sensitivity", &path)?;
        let specificity = number_field(value, "specificity", &path)?;
        if !(0.0..=1.0).contains(&sensitivity) || !(0.0..=1.0).contains(&specificity) {
            return Err(fail(&path, "sensitivity and specificity must be in [0, 1]"));
        }
        let required_calibration = if method == "direct_transport" {
            "valid"
        } else {
            "qualified"
        };
        if text_field(value, "calibration_status", &path)? != required_calibration {
            return Err(fail(
                &path,
                format!("method requires calibration_status={required_calibration}"),
            ));
        }
        let validation_artifact_sha256 =
            text_field(value, "validation_artifact_sha256", &path)?.to_owned();
        validate_digest(
            &validation_artifact_sha256,
            &format!("{path}.validation_artifact_sha256"),
        )?;
        actions.push(Action {
            id,
            method,
            cell_index,
            sensitivity,
            specificity,
            cost_units: u64_field(value, "cost_units", &path)?,
            instrument_id: text_field(value, "instrument_id", &path)?.to_owned(),
            calibration_revision: text_field(value, "calibration_revision", &path)?.to_owned(),
            validation_artifact_sha256,
        });
    }
    actions.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(actions)
}

fn likelihood(action: &Action, weak: bool, positive: bool) -> f64 {
    match (weak, positive) {
        (true, true) => action.sensitivity,
        (true, false) => 1.0 - action.sensitivity,
        (false, true) => 1.0 - action.specificity,
        (false, false) => action.specificity,
    }
}

fn failure_probability(scenarios: &[Scenario], weights: &[f64]) -> f64 {
    scenarios
        .iter()
        .zip(weights)
        .filter(|(scenario, _)| scenario.application_failure)
        .map(|(_, weight)| *weight)
        .sum()
}

fn terminal_decision(
    failure_probability: f64,
    false_accept_loss: f64,
    false_reject_loss: f64,
) -> (f64, &'static str) {
    let accept_loss = false_accept_loss * failure_probability;
    let reject_loss = false_reject_loss * (1.0 - failure_probability);
    if accept_loss <= reject_loss {
        (accept_loss, "PROVISIONAL_ACCEPT")
    } else {
        (reject_loss, "PROVISIONAL_REJECT")
    }
}

fn posterior_after(
    scenarios: &[Scenario],
    weights: &[f64],
    action: &Action,
    positive: bool,
) -> Option<(f64, Vec<f64>)> {
    let probability = scenarios
        .iter()
        .zip(weights)
        .map(|(scenario, weight)| {
            weight * likelihood(action, scenario.weak_cells[action.cell_index], positive)
        })
        .sum::<f64>();
    if probability <= EPSILON {
        return None;
    }
    let posterior = scenarios
        .iter()
        .zip(weights)
        .map(|(scenario, weight)| {
            weight * likelihood(action, scenario.weak_cells[action.cell_index], positive)
                / probability
        })
        .collect::<Vec<_>>();
    Some((probability, posterior))
}

fn state_key(
    weights: &[f64],
    available_action_mask: u64,
    remaining_budget_units: u64,
    remaining_horizon: usize,
) -> StateKey {
    StateKey {
        posterior_weight_bits: weights.iter().map(|weight| weight.to_bits()).collect(),
        available_action_mask,
        remaining_budget_units,
        remaining_horizon,
    }
}

fn stop_plan(
    scenarios: &[Scenario],
    weights: &[f64],
    profile: &Profile,
) -> Plan {
    let risk = failure_probability(scenarios, weights);
    let (loss, disposition) = terminal_decision(
        risk,
        profile.false_accept_loss,
        profile.false_reject_loss,
    );
    Plan {
        expected_total_loss: loss,
        selected_action_index: None,
        terminal_disposition: Some(disposition),
        terminal_decision_loss: Some(loss),
        probability_positive: None,
        positive_child: None,
        negative_child: None,
    }
}

fn solve_state(
    scenarios: &[Scenario],
    actions: &[Action],
    profile: &Profile,
    weights: &[f64],
    available_action_mask: u64,
    remaining_budget_units: u64,
    remaining_horizon: usize,
    memo: &mut BTreeMap<StateKey, Plan>,
) -> Plan {
    let key = state_key(
        weights,
        available_action_mask,
        remaining_budget_units,
        remaining_horizon,
    );
    if let Some(cached) = memo.get(&key) {
        return cached.clone();
    }

    let mut best = stop_plan(scenarios, weights, profile);
    if remaining_horizon > 0 {
        for (index, action) in actions.iter().enumerate() {
            let bit = 1_u64 << index;
            if available_action_mask & bit == 0 || action.cost_units > remaining_budget_units {
                continue;
            }
            let next_mask = available_action_mask & !bit;
            let positive = posterior_after(scenarios, weights, action, true);
            let negative = posterior_after(scenarios, weights, action, false);
            let mut expected_value = action.cost_units as f64;
            let mut positive_child = None;
            let mut negative_child = None;
            let mut probability_positive = 0.0;

            if let Some((probability, posterior)) = positive {
                probability_positive = probability;
                let child = solve_state(
                    scenarios,
                    actions,
                    profile,
                    &posterior,
                    next_mask,
                    remaining_budget_units - action.cost_units,
                    remaining_horizon - 1,
                    memo,
                );
                expected_value += probability * child.expected_total_loss;
                positive_child = Some(Box::new(child));
            }
            if let Some((probability, posterior)) = negative {
                let child = solve_state(
                    scenarios,
                    actions,
                    profile,
                    &posterior,
                    next_mask,
                    remaining_budget_units - action.cost_units,
                    remaining_horizon - 1,
                    memo,
                );
                expected_value += probability * child.expected_total_loss;
                negative_child = Some(Box::new(child));
            }

            let is_better = expected_value + EPSILON < best.expected_total_loss;
            let tie_is_lexically_better = (expected_value - best.expected_total_loss).abs() <= EPSILON
                && best
                    .selected_action_index
                    .is_some_and(|best_index| action.id < actions[best_index].id);
            if is_better || tie_is_lexically_better {
                best = Plan {
                    expected_total_loss: expected_value,
                    selected_action_index: Some(index),
                    terminal_disposition: None,
                    terminal_decision_loss: None,
                    probability_positive: Some(probability_positive),
                    positive_child,
                    negative_child,
                };
            }
        }
    }
    memo.insert(key, best.clone());
    best
}

fn plan_to_json(plan: &Plan, actions: &[Action]) -> Value {
    if let Some(index) = plan.selected_action_index {
        let action = &actions[index];
        json!({
            "node_type": "inspect",
            "action_id": action.id,
            "method": action.method,
            "cell_index": action.cell_index,
            "instrument_id": action.instrument_id,
            "calibration_revision": action.calibration_revision,
            "validation_artifact_sha256": action.validation_artifact_sha256,
            "cost_units": action.cost_units,
            "expected_total_loss_from_node": plan.expected_total_loss,
            "probability_positive": plan.probability_positive,
            "positive_outcome_policy": plan.positive_child.as_ref().map(|child| plan_to_json(child, actions)),
            "negative_outcome_policy": plan.negative_child.as_ref().map(|child| plan_to_json(child, actions))
        })
    } else {
        json!({
            "node_type": "stop",
            "terminal_disposition": plan.terminal_disposition,
            "terminal_decision_loss": plan.terminal_decision_loss,
            "expected_total_loss_from_node": plan.expected_total_loss
        })
    }
}

fn solve(
    scenarios: &[Scenario],
    actions: &[Action],
    profile: &Profile,
    horizon: usize,
) -> SolveResult {
    let initial_weights = scenarios
        .iter()
        .map(|scenario| scenario.prior_weight)
        .collect::<Vec<_>>();
    let available_mask = (1_u64 << actions.len()) - 1;
    let mut memo = BTreeMap::new();
    let plan = solve_state(
        scenarios,
        actions,
        profile,
        &initial_weights,
        available_mask,
        profile.total_budget_units,
        horizon,
        &mut memo,
    );
    SolveResult {
        plan,
        explored_state_count: memo.len(),
    }
}

fn audit(value: &Value, fixture_bytes: &[u8]) -> Result<Value, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "evidence_class",
            "claim_ceiling",
            "application_profile",
            "cell_count",
            "planning_horizon",
            "prior_scenarios",
            "candidate_actions",
        ],
        "$",
    )?;
    if text_field(value, "schema_version", "$")? != SCHEMA_VERSION {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    if text_field(value, "evidence_class", "$")? != "synthetic" {
        return Err(fail("$.evidence_class", "v0.1 accepts synthetic fixture only"));
    }
    if text_field(value, "claim_ceiling", "$")? != CLAIM_CEILING {
        return Err(fail("$.claim_ceiling", "claim ceiling mismatch"));
    }
    let profile = parse_profile(&value["application_profile"])?;
    let cell_count = usize_field(value, "cell_count", "$")?;
    if cell_count == 0 || profile.minimum_contiguous_weak_cells > cell_count {
        return Err(fail(
            "$.cell_count",
            "cell_count must be positive and at least the weak-run threshold",
        ));
    }
    let planning_horizon = usize_field(value, "planning_horizon", "$")?;
    if planning_horizon == 0 || planning_horizon > MAX_HORIZON {
        return Err(fail(
            "$.planning_horizon",
            format!("must be between 1 and {MAX_HORIZON}"),
        ));
    }
    let scenarios = parse_scenarios(
        array(&value["prior_scenarios"], "$.prior_scenarios")?,
        cell_count,
        profile.minimum_contiguous_weak_cells,
    )?;
    let actions = parse_actions(
        array(&value["candidate_actions"], "$.candidate_actions")?,
        &profile,
        cell_count,
    )?;
    if planning_horizon > actions.len() {
        return Err(fail(
            "$.planning_horizon",
            "cannot exceed the number of unique candidate actions",
        ));
    }

    let dp = solve(&scenarios, &actions, &profile, planning_horizon);
    let myopic = solve(&scenarios, &actions, &profile, 1);
    if dp.plan.expected_total_loss > myopic.plan.expected_total_loss + 1.0e-9 {
        return Err("exact finite-horizon value exceeded its one-step restricted policy".to_owned());
    }
    let initial_weights = scenarios
        .iter()
        .map(|scenario| scenario.prior_weight)
        .collect::<Vec<_>>();
    let prior_failure_probability = failure_probability(&scenarios, &initial_weights);
    let (stop_loss, stop_disposition) = terminal_decision(
        prior_failure_probability,
        profile.false_accept_loss,
        profile.false_reject_loss,
    );
    Ok(json!({
        "schema_version": "rebco-sequential-inspection-dp-receipt.v0.1",
        "fixture_sha256": format!("{:x}", Sha256::digest(fixture_bytes)),
        "application_profile_id": profile.profile_id,
        "condition_profile_id": profile.condition_profile_id,
        "cell_count": cell_count,
        "scenario_count": scenarios.len(),
        "candidate_action_count": actions.len(),
        "planning_horizon": planning_horizon,
        "initial_failure_probability": prior_failure_probability,
        "stop_now_decision_loss": stop_loss,
        "stop_now_disposition": stop_disposition,
        "one_step_greedy_expected_total_loss": myopic.plan.expected_total_loss,
        "finite_horizon_optimal_expected_total_loss": dp.plan.expected_total_loss,
        "expected_loss_improvement_vs_one_step": myopic.plan.expected_total_loss - dp.plan.expected_total_loss,
        "one_step_greedy_first_action": myopic.plan.selected_action_index.map(|index| actions[index].id.clone()),
        "finite_horizon_first_action": dp.plan.selected_action_index.map(|index| actions[index].id.clone()),
        "finite_horizon_policy_tree": plan_to_json(&dp.plan, &actions),
        "memoized_state_count": dp.explored_state_count,
        "policy_contract": {
            "exact_dynamic_programming": true,
            "finite_horizon": true,
            "one_physical_action_per_policy_node": true,
            "hidden_physical_truth_is_not_an_input": true,
            "measurement_execution": "NOT_PERFORMED",
            "product_release_authority": "NOT_PROVIDED"
        },
        "claim_ceiling": CLAIM_CEILING
    }))
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_FIXTURE.to_owned());
    let source = fs::read(&path)
        .map_err(|error| format!("cannot read fixture {path}: {error}"))?;
    let value: Value = serde_json::from_slice(&source)
        .map_err(|error| format!("invalid JSON in {path}: {error}"))?;
    let receipt = audit(&value, &source)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("receipt serialization failed: {error}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("sequential_inspection_dp: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/fixtures/rebco_sequential_inspection_dp_synthetic_v0_1.json"
        ))
        .expect("sequential inspection fixture must parse")
    }

    fn run() -> Value {
        let value = fixture();
        let bytes = serde_json::to_vec(&value).expect("fixture serializes");
        audit(&value, &bytes).expect("fixture should audit")
    }

    fn path_action_count(value: &Value) -> usize {
        if value["node_type"] == "stop" {
            return 0;
        }
        let positive = value
            .get("positive_outcome_policy")
            .and_then(Value::as_object)
            .map(|child| path_action_count(&Value::Object(child.clone())))
            .unwrap_or(0);
        let negative = value
            .get("negative_outcome_policy")
            .and_then(Value::as_object)
            .map(|child| path_action_count(&Value::Object(child.clone())))
            .unwrap_or(0);
        1 + positive.max(negative)
    }

    #[test]
    fn exact_dp_never_exceeds_one_step_restricted_policy() {
        let receipt = run();
        let dp = receipt["finite_horizon_optimal_expected_total_loss"].as_f64().unwrap();
        let myopic = receipt["one_step_greedy_expected_total_loss"].as_f64().unwrap();
        assert!(dp <= myopic + 1.0e-9);
    }

    #[test]
    fn policy_tree_never_exceeds_declared_horizon() {
        let receipt = run();
        assert!(path_action_count(&receipt["finite_horizon_policy_tree"]) <= 3);
    }

    #[test]
    fn policy_tree_actions_are_unique_on_any_path() {
        let receipt = run();
        fn check(node: &Value, seen: &mut BTreeSet<String>) {
            if node["node_type"] == "stop" {
                return;
            }
            let id = node["action_id"].as_str().unwrap().to_owned();
            assert!(seen.insert(id));
            for key in ["positive_outcome_policy", "negative_outcome_policy"] {
                if let Some(child) = node.get(key).filter(|value| !value.is_null()) {
                    let mut branch_seen = seen.clone();
                    check(child, &mut branch_seen);
                }
            }
        }
        check(
            &receipt["finite_horizon_policy_tree"],
            &mut BTreeSet::new(),
        );
    }

    #[test]
    fn deterministic_tie_breaking_does_not_depend_on_input_action_order() {
        let mut value = fixture();
        let report_a = audit(
            &value,
            &serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        value["candidate_actions"].as_array_mut().unwrap().reverse();
        let report_b = audit(
            &value,
            &serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
        assert_eq!(
            report_a["finite_horizon_policy_tree"]["action_id"],
            report_b["finite_horizon_policy_tree"]["action_id"]
        );
    }

    #[test]
    fn rejects_action_condition_profile_mismatch() {
        let mut value = fixture();
        value["candidate_actions"][0]["condition_profile_id"] = json!("20K-5T");
        assert!(audit(&value, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("does not match the application profile"));
    }

    #[test]
    fn rejects_unqualified_proxy_action() {
        let mut value = fixture();
        let index = value["candidate_actions"]
            .as_array()
            .unwrap()
            .iter()
            .position(|action| action["method"] == "calibrated_noncontact_proxy")
            .unwrap();
        value["candidate_actions"][index]["calibration_status"] = json!("pending");
        assert!(audit(&value, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("calibration_status=qualified"));
    }

    #[test]
    fn rejects_budget_that_cannot_represent_nonnegative_integer_cost() {
        let mut value = fixture();
        value["application_profile"]["total_budget_units"] = json!(-1);
        assert!(audit(&value, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("non-negative integer"));
    }

    #[test]
    fn rejects_duplicate_action_ids() {
        let mut value = fixture();
        let first = value["candidate_actions"][0]["action_id"].clone();
        value["candidate_actions"][1]["action_id"] = first;
        assert!(audit(&value, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("duplicate action_id"));
    }

    #[test]
    fn rejects_prior_weights_that_do_not_sum_to_one() {
        let mut value = fixture();
        value["prior_scenarios"][0]["prior_weight"] = json!(0.3);
        assert!(audit(&value, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("prior weights must sum to 1"));
    }

    #[test]
    fn rejects_horizon_larger_than_action_count() {
        let mut value = fixture();
        value["planning_horizon"] = json!(100);
        assert!(audit(&value, &serde_json::to_vec(&value).unwrap())
            .unwrap_err()
            .contains("must be between 1"));
    }

    #[test]
    fn terminal_policy_can_stop_without_spending_budget() {
        let mut value = fixture();
        value["application_profile"]["total_budget_units"] = json!(0);
        let receipt = audit(&value, &serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(receipt["finite_horizon_policy_tree"]["node_type"], "stop");
        assert_eq!(receipt["finite_horizon_first_action"], Value::Null);
    }

    #[test]
    fn memoized_exact_and_exhaustive_oracle_agree_on_small_fixture() {
        let value = fixture();
        let profile = parse_profile(&value["application_profile"]).unwrap();
        let scenarios = parse_scenarios(
            array(&value["prior_scenarios"], "prior_scenarios").unwrap(),
            value["cell_count"].as_u64().unwrap() as usize,
            profile.minimum_contiguous_weak_cells,
        )
        .unwrap();
        let actions = parse_actions(
            array(&value["candidate_actions"], "candidate_actions").unwrap(),
            &profile,
            value["cell_count"].as_u64().unwrap() as usize,
        )
        .unwrap();
        let weights = scenarios.iter().map(|scenario| scenario.prior_weight).collect::<Vec<_>>();
        let mut memo = BTreeMap::new();
        let exact = solve_state(
            &scenarios,
            &actions,
            &profile,
            &weights,
            (1_u64 << actions.len()) - 1,
            profile.total_budget_units,
            value["planning_horizon"].as_u64().unwrap() as usize,
            &mut memo,
        );
        let exhaustive = solve_state_exhaustive(
            &scenarios,
            &actions,
            &profile,
            &weights,
            (1_u64 << actions.len()) - 1,
            profile.total_budget_units,
            value["planning_horizon"].as_u64().unwrap() as usize,
        );
        assert!((exact.expected_total_loss - exhaustive.expected_total_loss).abs() < 1.0e-9);
    }
}

fn solve_state_exhaustive(
    scenarios: &[Scenario],
    actions: &[Action],
    profile: &Profile,
    weights: &[f64],
    available_action_mask: u64,
    remaining_budget_units: u64,
    remaining_horizon: usize,
) -> Plan {
    let mut best = stop_plan(scenarios, weights, profile);
    if remaining_horizon == 0 {
        return best;
    }
    for (index, action) in actions.iter().enumerate() {
        let bit = 1_u64 << index;
        if available_action_mask & bit == 0 || action.cost_units > remaining_budget_units {
            continue;
        }
        let next_mask = available_action_mask & !bit;
        let positive = posterior_after(scenarios, weights, action, true);
        let negative = posterior_after(scenarios, weights, action, false);
        let mut expected_value = action.cost_units as f64;
        let mut positive_child = None;
        let mut negative_child = None;
        let mut probability_positive = 0.0;
        if let Some((probability, posterior)) = positive {
            probability_positive = probability;
            let child = solve_state_exhaustive(
                scenarios,
                actions,
                profile,
                &posterior,
                next_mask,
                remaining_budget_units - action.cost_units,
                remaining_horizon - 1,
            );
            expected_value += probability * child.expected_total_loss;
            positive_child = Some(Box::new(child));
        }
        if let Some((probability, posterior)) = negative {
            let child = solve_state_exhaustive(
                scenarios,
                actions,
                profile,
                &posterior,
                next_mask,
                remaining_budget_units - action.cost_units,
                remaining_horizon - 1,
            );
            expected_value += probability * child.expected_total_loss;
            negative_child = Some(Box::new(child));
        }
        let is_better = expected_value + EPSILON < best.expected_total_loss;
        let tie_is_lexically_better = (expected_value - best.expected_total_loss).abs() <= EPSILON
            && best
                .selected_action_index
                .is_some_and(|best_index| action.id < actions[best_index].id);
        if is_better || tie_is_lexically_better {
            best = Plan {
                expected_total_loss: expected_value,
                selected_action_index: Some(index),
                terminal_disposition: None,
                terminal_decision_loss: None,
                probability_positive: Some(probability_positive),
                positive_child,
                negative_child,
            };
        }
    }
    best
}
