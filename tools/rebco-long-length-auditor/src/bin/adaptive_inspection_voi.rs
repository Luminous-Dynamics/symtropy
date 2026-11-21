// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! One-step, decision-aware value-of-information policy for synthetic REBCO
//! spatial inspection designs.
//!
//! The policy recommends one next action. It never executes a measurement,
//! releases a reel, or treats a synthetic scenario ensemble as physical truth.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const SCHEMA_VERSION: &str = "rebco-adaptive-inspection-voi.v0.1";
const CLAIM_CEILING: &str = "single-step-posterior-decision-voi-only";
const DEFAULT_FIXTURE: &str =
    "docs/research/fixtures/rebco_adaptive_inspection_voi_synthetic_v0_1.json";
const EPSILON: f64 = 1.0e-12;

#[derive(Debug, Clone)]
struct Scenario {
    id: String,
    weight: f64,
    weak_cells: Vec<bool>,
    failure: bool,
}

#[derive(Debug, Clone)]
struct Action {
    id: String,
    method: String,
    cell_index: usize,
    sensitivity: f64,
    specificity: f64,
    cost: f64,
    instrument_id: String,
    calibration_revision: String,
    validation_artifact_sha256: String,
}

#[derive(Debug, Clone)]
struct Profile {
    profile_id: String,
    condition_profile_id: String,
    minimum_contiguous_weak_cells: usize,
    false_accept_loss: f64,
    false_reject_loss: f64,
    total_budget: f64,
    minimum_positive_net_voi: f64,
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

fn exact_keys(
    value: &Value,
    required: &[&str],
    path: &str,
) -> Result<(), String> {
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

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
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
        .ok_or_else(|| fail(path, format!("{key} must be a finite number")))?;
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

fn nonnegative_number(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = number_field(value, key, path)?;
    if number < 0.0 {
        return Err(fail(path, format!("{key} must be non-negative")));
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
        "total_inspection_budget_loss_units",
        "minimum_positive_net_voi",
    ];
    exact_keys(value, KEYS, "$.application_profile")?;
    let path = "$.application_profile";
    let minimum_contiguous_weak_cells =
        usize_field(value, "minimum_contiguous_weak_cells", path)?;
    if minimum_contiguous_weak_cells == 0 {
        return Err(fail(
            path,
            "minimum_contiguous_weak_cells must be at least one",
        ));
    }
    Ok(Profile {
        profile_id: string_field(value, "profile_id", path)?.to_owned(),
        condition_profile_id: string_field(value, "condition_profile_id", path)?.to_owned(),
        minimum_contiguous_weak_cells,
        false_accept_loss: positive_number(value, "false_accept_loss", path)?,
        false_reject_loss: positive_number(value, "false_reject_loss", path)?,
        total_budget: positive_number(value, "total_inspection_budget_loss_units", path)?,
        minimum_positive_net_voi: nonnegative_number(value, "minimum_positive_net_voi", path)?,
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
        let id = string_field(value, "scenario_id", &path)?.to_owned();
        if !ids.insert(id.clone()) {
            return Err(fail(&path, "duplicate scenario_id"));
        }
        let weight = positive_number(value, "prior_weight", &path)?;
        let cells = array(
            value.get("weak_cells").ok_or_else(|| fail(&path, "missing weak_cells"))?,
            &format!("{path}.weak_cells"),
        )?;
        if cells.len() != cell_count {
            return Err(fail(
                &path,
                format!("weak_cells length {} does not match cell_count {cell_count}", cells.len()),
            ));
        }
        let mut weak_cells = Vec::with_capacity(cell_count);
        for (cell_index, cell) in cells.iter().enumerate() {
            weak_cells.push(
                cell.as_bool()
                    .ok_or_else(|| fail(&path, format!("weak_cells[{cell_index}] must be boolean")))?,
            );
        }
        total_weight += weight;
        scenarios.push(Scenario {
            id,
            weight,
            failure: maximum_true_run(&weak_cells) >= minimum_run,
            weak_cells,
        });
    }
    if (total_weight - 1.0).abs() > 1.0e-9 {
        return Err(fail(
            "$.prior_scenarios",
            format!("prior_weight values must sum to 1 (observed {total_weight})"),
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
    const KEYS: &[&str] = &[
        "action_id",
        "method",
        "cell_index",
        "condition_profile_id",
        "sensitivity",
        "specificity",
        "cost_loss_units",
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
        let id = string_field(value, "action_id", &path)?.to_owned();
        if !ids.insert(id.clone()) {
            return Err(fail(&path, "duplicate action_id"));
        }
        let method = string_field(value, "method", &path)?.to_owned();
        if !["direct_transport", "calibrated_noncontact_proxy"].contains(&method.as_str()) {
            return Err(fail(&path, "method must be direct_transport or calibrated_noncontact_proxy"));
        }
        let condition_profile_id = string_field(value, "condition_profile_id", &path)?;
        if condition_profile_id != profile.condition_profile_id {
            return Err(fail(
                &path,
                "action condition_profile_id does not match the application profile",
            ));
        }
        let cell_index = usize_field(value, "cell_index", &path)?;
        if cell_index >= cell_count {
            return Err(fail(&path, "cell_index lies outside the declared reel grid"));
        }
        let sensitivity = number_field(value, "sensitivity", &path)?;
        let specificity = number_field(value, "specificity", &path)?;
        if !(0.0..=1.0).contains(&sensitivity) || !(0.0..=1.0).contains(&specificity) {
            return Err(fail(&path, "sensitivity and specificity must be in [0, 1]"));
        }
        let calibration_status = string_field(value, "calibration_status", &path)?;
        let expected_status = if method == "direct_transport" {
            "valid"
        } else {
            "qualified"
        };
        if calibration_status != expected_status {
            return Err(fail(
                &path,
                format!("{method} requires calibration_status={expected_status}"),
            ));
        }
        let validation_artifact_sha256 =
            string_field(value, "validation_artifact_sha256", &path)?.to_owned();
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
            cost: nonnegative_number(value, "cost_loss_units", &path)?,
            instrument_id: string_field(value, "instrument_id", &path)?.to_owned(),
            calibration_revision: string_field(value, "calibration_revision", &path)?.to_owned(),
            validation_artifact_sha256,
        });
    }
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

fn minimum_decision_loss(
    failure_probability: f64,
    false_accept_loss: f64,
    false_reject_loss: f64,
) -> (f64, &'static str) {
    let accept_loss = false_accept_loss * failure_probability;
    let reject_loss = false_reject_loss * (1.0 - failure_probability);
    if accept_loss <= reject_loss {
        (accept_loss, "provisional_accept")
    } else {
        (reject_loss, "provisional_reject")
    }
}

fn failure_probability(scenarios: &[Scenario]) -> f64 {
    scenarios
        .iter()
        .filter(|scenario| scenario.failure)
        .map(|scenario| scenario.weight)
        .sum()
}

fn condition_failure_probability(
    scenarios: &[Scenario],
    action: &Action,
    positive: bool,
) -> (f64, f64) {
    let probability: f64 = scenarios
        .iter()
        .map(|scenario| {
            scenario.weight * likelihood(action, scenario.weak_cells[action.cell_index], positive)
        })
        .sum();
    if probability <= EPSILON {
        return (0.0, 0.0);
    }
    let joint_failure: f64 = scenarios
        .iter()
        .filter(|scenario| scenario.failure)
        .map(|scenario| {
            scenario.weight * likelihood(action, scenario.weak_cells[action.cell_index], positive)
        })
        .sum();
    (probability, (joint_failure / probability).clamp(0.0, 1.0))
}

fn evaluate_action(
    scenarios: &[Scenario],
    action: &Action,
    profile: &Profile,
) -> Value {
    let current_failure_probability = failure_probability(scenarios);
    let (current_loss, _) = minimum_decision_loss(
        current_failure_probability,
        profile.false_accept_loss,
        profile.false_reject_loss,
    );
    let (positive_probability, failure_if_positive) =
        condition_failure_probability(scenarios, action, true);
    let (negative_probability, failure_if_negative) =
        condition_failure_probability(scenarios, action, false);
    let (positive_loss, positive_disposition) = minimum_decision_loss(
        failure_if_positive,
        profile.false_accept_loss,
        profile.false_reject_loss,
    );
    let (negative_loss, negative_disposition) = minimum_decision_loss(
        failure_if_negative,
        profile.false_accept_loss,
        profile.false_reject_loss,
    );
    let expected_loss_after_measurement =
        positive_probability * positive_loss + negative_probability * negative_loss;
    let gross_voi = current_loss - expected_loss_after_measurement;
    let net_voi = gross_voi - action.cost;
    json!({
        "action_id": action.id,
        "method": action.method,
        "cell_index": action.cell_index,
        "instrument_id": action.instrument_id,
        "calibration_revision": action.calibration_revision,
        "validation_artifact_sha256": action.validation_artifact_sha256,
        "measurement_cost_loss_units": action.cost,
        "probability_positive": positive_probability,
        "failure_probability_if_positive": failure_if_positive,
        "provisional_disposition_if_positive": positive_disposition,
        "probability_negative": negative_probability,
        "failure_probability_if_negative": failure_if_negative,
        "provisional_disposition_if_negative": negative_disposition,
        "expected_decision_loss_after_measurement": expected_loss_after_measurement,
        "gross_value_of_information": gross_voi,
        "net_value_of_information": net_voi
    })
}

fn parse_observed_results(
    values: &[Value],
    actions: &[Action],
    scenarios: &mut [Scenario],
) -> Result<(BTreeSet<String>, f64), String> {
    let actions_by_id: BTreeMap<&str, &Action> =
        actions.iter().map(|action| (action.id.as_str(), action)).collect();
    let mut used = BTreeSet::new();
    let mut total_cost = 0.0;
    for (index, value) in values.iter().enumerate() {
        let path = format!("$.observed_results[{index}]");
        exact_keys(value, &["action_id", "outcome"], &path)?;
        let action_id = string_field(value, "action_id", &path)?;
        if !used.insert(action_id.to_owned()) {
            return Err(fail(&path, "duplicate observed action_id"));
        }
        let action = actions_by_id
            .get(action_id)
            .ok_or_else(|| fail(&path, "observed action_id is not a declared candidate"))?;
        let outcome = string_field(value, "outcome", &path)?;
        let positive = match outcome {
            "positive" => true,
            "negative" => false,
            _ => return Err(fail(&path, "outcome must be positive or negative")),
        };
        total_cost += action.cost;
        let mut evidence_probability = 0.0;
        for scenario in scenarios.iter() {
            evidence_probability +=
                scenario.weight * likelihood(action, scenario.weak_cells[action.cell_index], positive);
        }
        if evidence_probability <= EPSILON {
            return Err(fail(
                &path,
                "observed outcome has zero probability under every prior scenario",
            ));
        }
        for scenario in scenarios.iter_mut() {
            scenario.weight *= likelihood(action, scenario.weak_cells[action.cell_index], positive);
            scenario.weight /= evidence_probability;
        }
    }
    Ok((used, total_cost))
}

fn audit_document(value: &Value) -> Result<Value, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "evidence_class",
            "claim_ceiling",
            "application_profile",
            "cell_count",
            "prior_scenarios",
            "candidate_actions",
            "observed_results",
        ],
        "$",
    )?;
    if string_field(value, "schema_version", "$")? != SCHEMA_VERSION {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    if string_field(value, "evidence_class", "$")? != "synthetic" {
        return Err(fail("$.evidence_class", "v0.1 accepts synthetic fixtures only"));
    }
    if string_field(value, "claim_ceiling", "$")? != CLAIM_CEILING {
        return Err(fail("$.claim_ceiling", "claim ceiling does not match v0.1"));
    }
    let profile = parse_profile(
        value
            .get("application_profile")
            .ok_or_else(|| fail("$", "missing application_profile"))?,
    )?;
    let cell_count = usize_field(value, "cell_count", "$")?;
    if cell_count == 0 {
        return Err(fail("$.cell_count", "must be positive"));
    }
    if profile.minimum_contiguous_weak_cells > cell_count {
        return Err(fail(
            "$.application_profile.minimum_contiguous_weak_cells",
            "cannot exceed cell_count",
        ));
    }
    let scenario_values = array(
        value
            .get("prior_scenarios")
            .ok_or_else(|| fail("$", "missing prior_scenarios"))?,
        "$.prior_scenarios",
    )?;
    let mut scenarios = parse_scenarios(
        scenario_values,
        cell_count,
        profile.minimum_contiguous_weak_cells,
    )?;
    let action_values = array(
        value
            .get("candidate_actions")
            .ok_or_else(|| fail("$", "missing candidate_actions"))?,
        "$.candidate_actions",
    )?;
    let actions = parse_actions(action_values, &profile, cell_count)?;
    let observed_values = array(
        value
            .get("observed_results")
            .ok_or_else(|| fail("$", "missing observed_results"))?,
        "$.observed_results",
    )?;
    let (used_actions, used_cost) =
        parse_observed_results(observed_values, &actions, &mut scenarios)?;
    if used_cost > profile.total_budget + EPSILON {
        return Err(fail(
            "$.observed_results",
            "observed inspection cost exceeds the declared total budget",
        ));
    }
    let remaining_budget = (profile.total_budget - used_cost).max(0.0);
    let current_failure_probability = failure_probability(&scenarios);
    let (current_decision_loss, current_disposition) = minimum_decision_loss(
        current_failure_probability,
        profile.false_accept_loss,
        profile.false_reject_loss,
    );

    let mut evaluations = Vec::<Value>::new();
    let mut ranked = Vec::<(f64, String, usize)>::new();
    for (index, action) in actions.iter().enumerate() {
        if used_actions.contains(&action.id) || action.cost > remaining_budget + EPSILON {
            continue;
        }
        let evaluation = evaluate_action(&scenarios, action, &profile);
        let net_voi = evaluation["net_value_of_information"]
            .as_f64()
            .ok_or_else(|| "internal VOI calculation was not numeric".to_owned())?;
        ranked.push((net_voi, action.id.clone(), index));
        evaluations.push(evaluation);
    }
    ranked.sort_by(|left, right| {
        right
            .0
            .total_cmp(&left.0)
            .then_with(|| left.1.cmp(&right.1))
    });
    let selected = ranked.first().filter(|(net_voi, _, _)| {
        *net_voi > profile.minimum_positive_net_voi
    });
    let selected_action_id = selected.map(|(_, id, _)| id.as_str());
    let recommendation_status = if selected_action_id.is_some() {
        "RECOMMEND_ONE_INSPECTION"
    } else {
        "STOP_NO_POSITIVE_NET_VOI"
    };
    let recommendation_reason = if selected_action_id.is_some() {
        "highest positive net value of information within the remaining budget"
    } else if ranked.is_empty() {
        "no unused candidate action fits the remaining budget"
    } else {
        "all candidate actions have net value at or below the declared stop threshold"
    };

    let posterior = scenarios
        .iter()
        .map(|scenario| {
            json!({
                "scenario_id": scenario.id,
                "posterior_weight": scenario.weight,
                "application_failure": scenario.failure
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "schema_version": "rebco-adaptive-inspection-voi-receipt.v0.1",
        "application_profile_id": profile.profile_id,
        "condition_profile_id": profile.condition_profile_id,
        "cell_count": cell_count,
        "scenario_count": scenarios.len(),
        "used_action_count": used_actions.len(),
        "used_inspection_cost_loss_units": used_cost,
        "remaining_inspection_budget_loss_units": remaining_budget,
        "posterior_application_failure_probability": current_failure_probability,
        "current_minimum_expected_decision_loss": current_decision_loss,
        "current_loss_minimizing_disposition": current_disposition,
        "candidate_evaluations": evaluations,
        "recommendation": {
            "status": recommendation_status,
            "selected_action_id": selected_action_id,
            "reason": recommendation_reason,
            "claim_ceiling": CLAIM_CEILING
        },
        "posterior_scenarios": posterior,
        "policy_contract": {
            "one_action_per_observation_cycle": true,
            "hidden_physical_truth_is_not_an_input": true,
            "product_release_authority": "NOT_PROVIDED",
            "measurement_execution": "NOT_PERFORMED"
        }
    }))
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_FIXTURE.to_owned());
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read fixture {path}: {error}"))?;
    let value: Value = serde_json::from_str(&source)
        .map_err(|error| format!("invalid JSON in {path}: {error}"))?;
    let mut receipt = audit_document(&value)?;
    let fixture_sha256 = format!("{:x}", Sha256::digest(source.as_bytes()));
    receipt["fixture_sha256"] = json!(fixture_sha256);
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot encode receipt: {error}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("adaptive_inspection_voi: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/fixtures/rebco_adaptive_inspection_voi_synthetic_v0_1.json"
        ))
        .expect("adaptive inspection fixture must parse")
    }

    fn action_index(value: &Value, action_id: &str) -> usize {
        value["candidate_actions"]
            .as_array()
            .expect("candidate_actions array")
            .iter()
            .position(|action| action["action_id"] == action_id)
            .expect("declared action")
    }

    #[test]
    fn synthetic_policy_recommends_highest_positive_net_voi() {
        let report = audit_document(&fixture()).expect("fixture should validate");
        assert_eq!(
            report["recommendation"]["status"],
            "RECOMMEND_ONE_INSPECTION"
        );
        assert_eq!(
            report["recommendation"]["selected_action_id"],
            "DIRECT-TRANSPORT-CELL-003"
        );
        assert!((report["posterior_application_failure_probability"].as_f64().unwrap() - 0.40).abs() < 1.0e-9);
    }

    #[test]
    fn contiguous_weak_run_rule_is_not_replaced_by_weak_cell_count() {
        let mut value = fixture();
        value["application_profile"]["minimum_contiguous_weak_cells"] = json!(2);
        let report = audit_document(&value).expect("fixture should validate");
        assert!((report["posterior_application_failure_probability"].as_f64().unwrap() - 0.40).abs() < 1.0e-9);
        let scenarios = array(&value["prior_scenarios"], "prior_scenarios").unwrap();
        let isolated = scenarios
            .iter()
            .find(|scenario| scenario["scenario_id"] == "ISOLATED-WEAK-CELLS")
            .unwrap();
        let cells = isolated["weak_cells"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_bool().unwrap())
            .collect::<Vec<_>>();
        assert!(maximum_true_run(&cells) < 2);
    }

    #[test]
    fn observed_positive_measurement_updates_posterior_and_excludes_used_action() {
        let mut value = fixture();
        value["observed_results"] = json!([{
            "action_id": "DIRECT-TRANSPORT-CELL-003",
            "outcome": "positive"
        }]);
        let report = audit_document(&value).expect("posterior update should validate");
        assert_eq!(report["used_action_count"], 1);
        assert!(
            report["recommendation"]["selected_action_id"] != "DIRECT-TRANSPORT-CELL-003"
        );
        assert!(
            report["posterior_application_failure_probability"].as_f64().unwrap() > 0.90
        );
    }

    #[test]
    fn observed_negative_measurement_reduces_posterior_failure_probability() {
        let mut value = fixture();
        value["observed_results"] = json!([{
            "action_id": "DIRECT-TRANSPORT-CELL-003",
            "outcome": "negative"
        }]);
        let report = audit_document(&value).expect("posterior update should validate");
        assert!(
            report["posterior_application_failure_probability"].as_f64().unwrap() < 0.25
        );
    }

    #[test]
    fn rejects_action_from_different_condition_profile() {
        let mut value = fixture();
        value["candidate_actions"][0]["condition_profile_id"] = json!("20K-5T");
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("does not match the application profile")
        );
    }

    #[test]
    fn rejects_proxy_without_qualified_calibration() {
        let mut value = fixture();
        let index = action_index(&value, "PROXY-CELL-003");
        value["candidate_actions"][index]["calibration_status"] = json!("pending");
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("calibration_status=qualified")
        );
    }

    #[test]
    fn rejects_direct_transport_without_valid_calibration() {
        let mut value = fixture();
        value["candidate_actions"][0]["calibration_status"] = json!("unknown");
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("calibration_status=valid")
        );
    }

    #[test]
    fn rejects_malformed_validation_artifact_digest() {
        let mut value = fixture();
        value["candidate_actions"][0]["validation_artifact_sha256"] = json!("not-a-digest");
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("SHA-256")
        );
    }

    #[test]
    fn rejects_contiguous_failure_threshold_larger_than_grid() {
        let mut value = fixture();
        value["application_profile"]["minimum_contiguous_weak_cells"] = json!(9);
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("cannot exceed cell_count")
        );
    }

    #[test]
    fn rejects_cell_outside_reel_grid() {
        let mut value = fixture();
        value["candidate_actions"][0]["cell_index"] = json!(99);
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("outside the declared reel grid")
        );
    }

    #[test]
    fn rejects_prior_weights_that_do_not_sum_to_one() {
        let mut value = fixture();
        value["prior_scenarios"][0]["prior_weight"] = json!(0.2);
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("must sum to 1")
        );
    }

    #[test]
    fn rejects_impossible_observed_outcome() {
        let mut value = fixture();
        value["observed_results"] = json!([{
            "action_id": "DIRECT-TRANSPORT-CELL-001",
            "outcome": "positive"
        }]);
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("zero probability under every prior scenario")
        );
    }

    #[test]
    fn rejects_duplicate_observed_action() {
        let mut value = fixture();
        value["observed_results"] = json!([
            {"action_id": "DIRECT-TRANSPORT-CELL-003", "outcome": "positive"},
            {"action_id": "DIRECT-TRANSPORT-CELL-003", "outcome": "negative"}
        ]);
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("duplicate observed action_id")
        );
    }

    #[test]
    fn rejects_observed_inspection_cost_over_budget() {
        let mut value = fixture();
        value["application_profile"]["total_inspection_budget_loss_units"] = json!(1.0);
        value["observed_results"] = json!([{
            "action_id": "DIRECT-TRANSPORT-CELL-003",
            "outcome": "positive"
        }]);
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("exceeds the declared total budget")
        );
    }

    #[test]
    fn abstains_when_no_unused_action_fits_budget() {
        let mut value = fixture();
        value["application_profile"]["total_inspection_budget_loss_units"] = json!(0.4);
        let report = audit_document(&value).expect("fixture should validate");
        assert_eq!(
            report["recommendation"]["status"],
            "STOP_NO_POSITIVE_NET_VOI"
        );
        assert_eq!(report["recommendation"]["selected_action_id"], Value::Null);
    }

    #[test]
    fn abstains_when_all_actions_have_nonpositive_net_value() {
        let mut value = fixture();
        value["candidate_actions"][0]["cost_loss_units"] = json!(1000.0);
        value["candidate_actions"][1]["cost_loss_units"] = json!(1000.0);
        value["candidate_actions"][2]["cost_loss_units"] = json!(1000.0);
        value["candidate_actions"][3]["cost_loss_units"] = json!(1000.0);
        let report = audit_document(&value).expect("fixture should validate");
        assert_eq!(
            report["recommendation"]["status"],
            "STOP_NO_POSITIVE_NET_VOI"
        );
    }

    #[test]
    fn ties_are_resolved_by_action_id_not_input_order() {
        let mut value = fixture();
        let mut duplicate = value["candidate_actions"][0].clone();
        duplicate["action_id"] = json!("DIRECT-TRANSPORT-CELL-003-Z");
        duplicate["instrument_id"] = json!("SYNTH-TRANSPORT-RIG-02");
        value["candidate_actions"]
            .as_array_mut()
            .unwrap()
            .insert(0, duplicate);
        let report = audit_document(&value).expect("fixture should validate");
        assert_eq!(
            report["recommendation"]["selected_action_id"],
            "DIRECT-TRANSPORT-CELL-003"
        );
    }

    #[test]
    fn rejects_duplicate_scenario_ids() {
        let mut value = fixture();
        let first_id = value["prior_scenarios"][0]["scenario_id"].clone();
        value["prior_scenarios"][1]["scenario_id"] = first_id;
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("duplicate scenario_id")
        );
    }

    #[test]
    fn rejects_duplicate_candidate_action_ids() {
        let mut value = fixture();
        let first_id = value["candidate_actions"][0]["action_id"].clone();
        value["candidate_actions"][1]["action_id"] = first_id;
        assert!(
            audit_document(&value)
                .unwrap_err()
                .contains("duplicate action_id")
        );
    }
}
