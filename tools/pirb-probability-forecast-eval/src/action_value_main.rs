// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Model-based, capacity-constrained policy comparison. Expected avoided harm
// depends on registered effectiveness assumptions and is never labelled as
// observed impact or causal effect.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-action-value-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-action-value-synthetic-v0.1.json");
const EPSILON: f64 = 1.0e-12;

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn option_value(args: &[String], name: &str) -> Result<Option<String>, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("argument {name} requires a value"))
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn integer_at(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn known_outcome(row: &Value) -> Option<i64> {
    if string_at(row, "outcome_status") != Some("known") {
        return None;
    }
    match integer_at(row, "outcome") {
        Some(value @ (0 | 1)) => Some(value),
        _ => None,
    }
}

fn expected_probability(row: &Value) -> Option<f64> {
    if string_at(row, "forecast_disposition") != Some("forecast_issued") {
        return None;
    }
    number_at(row, "served_probability")
}

fn effective_intervention_rate(row: &Value, multiplier: f64) -> f64 {
    (number_at(row, "assumed_effectiveness").unwrap_or_default() * multiplier).min(1.0)
}

fn expected_gross_harm_avoided(row: &Value, effect_multiplier: f64) -> f64 {
    let probability = expected_probability(row).unwrap_or_default();
    let harm = number_at(row, "expected_harm_per_event").unwrap_or_default();
    probability * harm * effective_intervention_rate(row, effect_multiplier)
}

fn expected_cost(row: &Value, cost_multiplier: f64) -> f64 {
    number_at(row, "action_cost").unwrap_or_default() * cost_multiplier
}

fn expected_net_benefit(row: &Value, cost_multiplier: f64, effect_multiplier: f64) -> f64 {
    expected_gross_harm_avoided(row, effect_multiplier) - expected_cost(row, cost_multiplier)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AllocationPolicy {
    NoAction,
    RiskRanked,
    ExpectedNetBenefit,
}

impl AllocationPolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::NoAction => "no_action",
            Self::RiskRanked => "risk_ranked_top_k",
            Self::ExpectedNetBenefit => "expected_net_benefit_top_k",
        }
    }
}

fn select_indices(
    rows: &[&Value],
    policy: AllocationPolicy,
    capacity: usize,
    cost_multiplier: f64,
    effect_multiplier: f64,
) -> Vec<usize> {
    if policy == AllocationPolicy::NoAction || capacity == 0 {
        return Vec::new();
    }

    let mut candidates = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let probability = expected_probability(row)?;
            let case_id = string_at(row, "case_id").unwrap_or_default();
            let ranking_value = match policy {
                AllocationPolicy::NoAction => 0.0,
                AllocationPolicy::RiskRanked => probability,
                AllocationPolicy::ExpectedNetBenefit => {
                    expected_net_benefit(row, cost_multiplier, effect_multiplier)
                }
            };
            if policy == AllocationPolicy::ExpectedNetBenefit && ranking_value <= 0.0 {
                return None;
            }
            Some((index, ranking_value, case_id))
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .1
            .partial_cmp(&left.1)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.2.cmp(right.2))
    });
    candidates
        .into_iter()
        .take(capacity)
        .map(|(index, _, _)| index)
        .collect()
}

fn ratio_or_null(numerator: usize, denominator: usize) -> Value {
    if denominator == 0 {
        Value::Null
    } else {
        json!(numerator as f64 / denominator as f64)
    }
}

fn group_allocations(
    rows: &[&Value],
    selected: &[usize],
    cost_multiplier: f64,
    effect_multiplier: f64,
    minimum_group_count: usize,
) -> Vec<Value> {
    let mut all_by_group: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (index, row) in rows.iter().enumerate() {
        all_by_group
            .entry(string_at(row, "group_id").unwrap_or_default().to_owned())
            .or_default()
            .push(index);
    }
    let selected_set = selected.iter().copied().collect::<BTreeSet<_>>();
    all_by_group
        .into_iter()
        .map(|(group_id, indices)| {
            if indices.len() < minimum_group_count {
                return json!({
                    "suppressed": true,
                    "suppression_reason": "below_minimum_group_report_count"
                });
            }
            let issued = indices
                .iter()
                .filter(|index| expected_probability(rows[**index]).is_some())
                .count();
            let abstained = indices.len() - issued;
            let selected_group = indices
                .iter()
                .filter(|index| selected_set.contains(index))
                .copied()
                .collect::<Vec<_>>();
            let gross = selected_group
                .iter()
                .map(|index| expected_gross_harm_avoided(rows[*index], effect_multiplier))
                .sum::<f64>();
            let cost = selected_group
                .iter()
                .map(|index| expected_cost(rows[*index], cost_multiplier))
                .sum::<f64>();
            json!({
                "suppressed": false,
                "group_id": group_id,
                "case_count": indices.len(),
                "forecast_issued_count": issued,
                "abstained_count": abstained,
                "selected_action_count": selected_group.len(),
                "model_expected_harm_avoided": gross,
                "assumed_action_cost": cost,
                "model_expected_net_benefit": gross - cost
            })
        })
        .collect()
}

fn policy_result(
    rows: &[&Value],
    policy: AllocationPolicy,
    capacity: usize,
    cost_multiplier: f64,
    effect_multiplier: f64,
    minimum_group_count: usize,
) -> Value {
    let selected = select_indices(rows, policy, capacity, cost_multiplier, effect_multiplier);
    let issued_indices = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| expected_probability(row).map(|_| index))
        .collect::<Vec<_>>();
    let baseline_expected_harm = issued_indices
        .iter()
        .map(|index| {
            expected_probability(rows[*index]).unwrap_or_default()
                * number_at(rows[*index], "expected_harm_per_event").unwrap_or_default()
        })
        .sum::<f64>();
    let expected_harm_avoided = selected
        .iter()
        .map(|index| expected_gross_harm_avoided(rows[*index], effect_multiplier))
        .sum::<f64>();
    let action_cost = selected
        .iter()
        .map(|index| expected_cost(rows[*index], cost_multiplier))
        .sum::<f64>();
    let selected_known_count = selected
        .iter()
        .filter(|index| known_outcome(rows[**index]).is_some())
        .count();
    let selected_event_count = selected
        .iter()
        .filter(|index| known_outcome(rows[**index]) == Some(1))
        .count();
    let selected_unknown_count = selected
        .iter()
        .filter(|index| string_at(rows[**index], "outcome_status") == Some("unknown"))
        .count();
    let selected_immature_count = selected
        .iter()
        .filter(|index| string_at(rows[**index], "outcome_status") == Some("immature"))
        .count();
    let all_known_count = rows.iter().filter(|row| known_outcome(row).is_some()).count();
    let all_known_events = rows
        .iter()
        .filter(|row| known_outcome(row) == Some(1))
        .count();
    let all_unknown_count = rows
        .iter()
        .filter(|row| string_at(row, "outcome_status") == Some("unknown"))
        .count();
    let all_immature_count = rows
        .iter()
        .filter(|row| string_at(row, "outcome_status") == Some("immature"))
        .count();
    let selected_case_count = selected.len();
    let unallocated_capacity = capacity.saturating_sub(selected_case_count);
    let expected_net = expected_harm_avoided - action_cost;
    let group_allocations = group_allocations(
        rows,
        &selected,
        cost_multiplier,
        effect_multiplier,
        minimum_group_count,
    );

    json!({
        "policy_id": policy.as_str(),
        "capacity": capacity,
        "selected_action_count": selected_case_count,
        "unallocated_capacity": unallocated_capacity,
        "eligible_forecast_issued_count": issued_indices.len(),
        "abstained_case_count": rows.len() - issued_indices.len(),
        "expected_harm_scope": "forecast_issued_pool_only",
        "model_expected_harm_if_no_action": baseline_expected_harm,
        "model_expected_harm_avoided_under_assumption": expected_harm_avoided,
        "assumed_action_cost": action_cost,
        "model_expected_net_benefit_under_assumption": expected_net,
        "model_expected_residual_harm_on_issued_pool": baseline_expected_harm - expected_harm_avoided,
        "all_case_known_outcome_count": all_known_count,
        "all_case_observed_event_count": all_known_events,
        "all_case_unknown_outcome_count": all_unknown_count,
        "all_case_immature_outcome_count": all_immature_count,
        "selected_known_outcome_count": selected_known_count,
        "selected_observed_event_count": selected_event_count,
        "selected_unknown_outcome_count": selected_unknown_count,
        "selected_immature_outcome_count": selected_immature_count,
        "selected_event_precision": ratio_or_null(selected_event_count, selected_known_count),
        "observed_event_capture_fraction": ratio_or_null(selected_event_count, all_known_events),
        "group_allocations": group_allocations
    })
}

fn semantic_errors(registration: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let policy = &registration["policy"];
    let provider = &registration["provider"];
    let origin = integer_at(policy, "forecast_origin_index").unwrap_or_default();
    let target_period = integer_at(policy, "target_period_index").unwrap_or_default();
    let feature_cutoff = integer_at(policy, "action_features_cutoff_index").unwrap_or_default();
    let outcome_as_of = integer_at(&registration, "outcome_as_of_index").unwrap_or_default();

    if feature_cutoff >= origin {
        errors.push("action-policy features must be cut off strictly before forecast origin".to_owned());
    }
    if target_period <= origin {
        errors.push("target period must be later than forecast origin".to_owned());
    }
    if outcome_as_of < target_period {
        errors.push("frozen outcome cutoff must not precede the forecast target period".to_owned());
    }

    let capacity_grid = policy["capacity_grid"].as_array().expect("schema validated capacity_grid");
    let capacities = capacity_grid.iter().filter_map(Value::as_i64).collect::<Vec<_>>();
    if capacities.len() != capacity_grid.len() || capacities.windows(2).any(|pair| pair[0] >= pair[1]) {
        errors.push("capacity_grid must be strictly increasing with no duplicates".to_owned());
    }
    for (field, label) in [
        ("action_cost_multiplier_grid", "action-cost multiplier grid"),
        ("effectiveness_multiplier_grid", "effectiveness multiplier grid"),
    ] {
        let grid = policy[field].as_array().expect("schema validated sensitivity grid");
        let values = grid.iter().filter_map(Value::as_f64).collect::<Vec<_>>();
        if values.len() != grid.len() || values.windows(2).any(|pair| pair[0] >= pair[1]) {
            errors.push(format!("{label} must be strictly increasing with no duplicates"));
        }
    }

    let target_id = string_at(policy, "target_id").unwrap_or_default();
    let rows = registration["rows"].as_array().expect("schema validated rows");
    let mut ids = BTreeSet::new();
    for row in rows {
        let case_id = string_at(row, "case_id").unwrap_or_default();
        if !ids.insert(case_id.to_owned()) {
            errors.push(format!("case_id is duplicated: {case_id}"));
        }
        if string_at(row, "target_id") != Some(target_id) {
            errors.push(format!(
                "case {case_id} target_id mismatches the registered target; fail closed rather than score a mixed estimand"
            ));
        }
        match string_at(row, "forecast_disposition").unwrap_or_default() {
            "forecast_issued" => {
                let candidate = number_at(row, "offline_candidate_probability");
                let served = number_at(row, "served_probability");
                if candidate.zip(served).is_none_or(|(candidate, served)| (candidate - served).abs() > EPSILON) {
                    errors.push(format!("issued case {case_id} must serve its registered candidate probability"));
                }
            }
            "abstained" => {
                if !row["served_probability"].is_null() {
                    errors.push(format!("abstained case {case_id} must not expose a served probability"));
                }
            }
            _ => {}
        }
        if string_at(row, "outcome_status") == Some("known") {
            if !matches!(integer_at(row, "outcome"), Some(0 | 1)) {
                errors.push(format!("known outcome for case {case_id} must be 0 or 1"));
            }
        } else if !row["outcome"].is_null() {
            errors.push(format!("unknown or immature outcome for case {case_id} must be null"));
        }
    }

    if string_at(&registration, "evidence_scope") == Some("authorized_historical") {
        for (field, label) in [
            ("forecast_artifact_sha256", "forecast artifact"),
            ("outcome_snapshot_sha256", "outcome snapshot"),
            ("action_assumptions_sha256", "action assumptions"),
        ] {
            let digest = string_at(provider, field).unwrap_or_default();
            if digest.chars().all(|ch| ch == '0') || digest.chars().all(|ch| ch == '1') {
                errors.push(format!(
                    "authorized historical evaluation requires a non-placeholder {label} digest"
                ));
            }
        }
        let governance = &registration["governance"];
        if string_at(governance, "data_authority_reference").is_none_or(str::is_empty)
            || string_at(governance, "disclosure_review_reference").is_none_or(str::is_empty)
        {
            errors.push("authorized historical evaluation requires authority and disclosure references".to_owned());
        }
    }

    errors.sort();
    errors
}

fn validate_and_evaluate(input: &[u8], source_commit: &str) -> Result<Value, Vec<String>> {
    if source_commit != "UNPINNED_LOCAL_RUN"
        && (source_commit.len() != 40
            || !source_commit
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()))
    {
        return Err(vec![
            "source commit must be a 40-character lowercase Git SHA or UNPINNED_LOCAL_RUN".to_owned(),
        ]);
    }

    let registration: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("action-value registration JSON is invalid: {error}")])?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES)
        .map_err(|error| vec![format!("embedded action-value schema JSON is invalid: {error}")])?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!("Draft 2020-12 schema meta-validation failed: {error}")]
    })?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build action-value schema validator: {error}")])?;
    let mut errors = validator
        .iter_errors(&registration)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        errors.sort();
        return Err(errors);
    }
    let semantic = semantic_errors(&registration);
    if !semantic.is_empty() {
        return Err(semantic);
    }

    let rows = registration["rows"].as_array().expect("schema validated rows").iter().collect::<Vec<_>>();
    let policy = &registration["policy"];
    let capacities = policy["capacity_grid"].as_array().expect("schema validated capacity grid")
        .iter().map(|v| v.as_i64().expect("schema validated capacities") as usize).collect::<Vec<_>>();
    let cost_grid = policy["action_cost_multiplier_grid"].as_array().expect("schema validated cost grid")
        .iter().map(|v| v.as_f64().expect("schema validated cost multipliers")).collect::<Vec<_>>();
    let effect_grid = policy["effectiveness_multiplier_grid"].as_array().expect("schema validated effect grid")
        .iter().map(|v| v.as_f64().expect("schema validated effect multipliers")).collect::<Vec<_>>();
    let minimum_group_count = integer_at(policy, "minimum_group_report_count").expect("schema validated group minimum") as usize;

    let baseline_expected_harm = rows.iter()
        .filter_map(|row| expected_probability(row).map(|p| p * number_at(row, "expected_harm_per_event").unwrap_or_default()))
        .sum::<f64>();
    let known_outcomes = rows.iter().filter(|row| known_outcome(row).is_some()).count();
    let known_events = rows.iter().filter(|row| known_outcome(row) == Some(1)).count();
    let abstained_count = rows.iter()
        .filter(|row| string_at(row, "forecast_disposition") == Some("abstained"))
        .count();
    let issued_count = rows.len() - abstained_count;

    let mut scenarios = Vec::new();
    for capacity in capacities.iter().copied() {
        for cost_multiplier in cost_grid.iter().copied() {
            for effect_multiplier in effect_grid.iter().copied() {
                let policy_results = [
                    AllocationPolicy::NoAction,
                    AllocationPolicy::RiskRanked,
                    AllocationPolicy::ExpectedNetBenefit,
                ]
                .iter()
                .map(|allocation_policy| {
                    policy_result(
                        &rows,
                        *allocation_policy,
                        capacity,
                        cost_multiplier,
                        effect_multiplier,
                        minimum_group_count,
                    )
                })
                .collect::<Vec<_>>();
                scenarios.push(json!({
                    "scenario_id": format!("capacity={capacity};cost_multiplier={cost_multiplier:.3};effectiveness_multiplier={effect_multiplier:.3}"),
                    "capacity": capacity,
                    "action_cost_multiplier": cost_multiplier,
                    "effectiveness_multiplier": effect_multiplier,
                    "assumption_note": "Effectiveness is multiplied by the scenario factor and capped at 1.0. Expected avoided harm and net benefit are conditional model outputs, not causal estimates.",
                    "policies": policy_results
                }));
            }
        }
    }

    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let input_sha256 = sha256_hex(input);
    let registration_id = string_at(&registration, "registration_id").expect("schema validated registration_id");
    let source_commit_pinned = source_commit != "UNPINNED_LOCAL_RUN";
    let receipt_commitment_sha256 = sha256_hex(
        format!(
            "pirb-action-value-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "capacity_constrained_action_value_audit",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "provider_id": registration["provider"]["provider_id"],
        "model_version": registration["provider"]["model_version"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit_pinned,
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "forecast_artifact_sha256": registration["provider"]["forecast_artifact_sha256"],
        "outcome_snapshot_sha256": registration["provider"]["outcome_snapshot_sha256"],
        "action_assumptions_sha256": registration["provider"]["action_assumptions_sha256"],
        "target_id": policy["target_id"],
        "outcome_as_of_index": registration["outcome_as_of_index"],
        "value_unit": policy["value_unit"],
        "capacity_grid": policy["capacity_grid"],
        "action_cost_multiplier_grid": policy["action_cost_multiplier_grid"],
        "effectiveness_multiplier_grid": policy["effectiveness_multiplier_grid"],
        "case_count": rows.len(),
        "forecast_issued_count": issued_count,
        "abstained_count": abstained_count,
        "all_case_known_outcome_count": known_outcomes,
        "all_case_observed_event_count": known_events,
        "baseline_expected_harm_on_issued_pool": baseline_expected_harm,
        "scenario_count": scenarios.len(),
        "scenarios": scenarios,
        "claims": {
            "schema_and_policy_contract_validated": true,
            "capacity_constrained_policies_compared": true,
            "expected_net_benefit_is_assumption_conditioned": true,
            "assumed_harm_avoided_is_observed_impact": false,
            "causal_impact_established": false,
            "forecast_superiority_established": false,
            "live_dispatch_authority": false,
            "person_level_targeting_authorized": false,
            "service_access_gating_authorized": false
        },
        "receipt_commitment_sha256": receipt_commitment_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-action-value-audit --input REGISTRATION.json [--source-commit SHA]".to_owned()
    })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read action-value registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let receipt =
        validate_and_evaluate(&input, &source_commit).map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize action-value receipt: {error}"))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_slice(FIXTURE_BYTES)
            .expect("checked-in synthetic action-value fixture is valid JSON")
    }

    fn evaluate(value: &Value) -> Result<Value, Vec<String>> {
        validate_and_evaluate(
            &serde_json::to_vec(value).expect("registration serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    fn scenario<'a>(receipt: &'a Value, capacity: i64, cost_multiplier: f64, effectiveness_multiplier: f64) -> &'a Value {
        receipt["scenarios"]
            .as_array()
            .expect("receipt includes scenarios")
            .iter()
            .find(|s| {
                integer_at(s, "capacity") == Some(capacity)
                    && number_at(s, "action_cost_multiplier").is_some_and(|v| (v-cost_multiplier).abs() < EPSILON)
                    && number_at(s, "effectiveness_multiplier").is_some_and(|v| (v-effectiveness_multiplier).abs() < EPSILON)
            })
            .expect("requested sensitivity scenario exists")
    }

    #[test]
    fn risk_ranked_and_net_benefit_policies_use_the_same_capacity() {
        let receipt = evaluate(&fixture()).expect("synthetic action-value contract is valid");
        let scenario = scenario(&receipt, 2, 1.0, 1.0);
        assert_eq!(scenario["policies"][1]["selected_action_count"], 2);
        assert_eq!(scenario["policies"][2]["selected_action_count"], 2);
        assert_eq!(scenario["policies"][1]["policy_id"], "risk_ranked_top_k");
        assert_eq!(scenario["policies"][2]["policy_id"], "expected_net_benefit_top_k");
    }

    #[test]
    fn expected_net_benefit_matches_golden_arithmetic_and_is_not_causal_impact() {
        let receipt = evaluate(&fixture()).expect("synthetic action-value contract is valid");
        let scenario = scenario(&receipt, 2, 1.0, 1.0);
        let risk = &scenario["policies"][1];
        let value_policy = &scenario["policies"][2];
        assert!((number_at(risk, "model_expected_net_benefit_under_assumption").unwrap() - 12.5).abs() < EPSILON);
        assert!((number_at(value_policy, "model_expected_harm_avoided_under_assumption").unwrap() - 71.5).abs() < EPSILON);
        assert!((number_at(value_policy, "assumed_action_cost").unwrap() - 25.0).abs() < EPSILON);
        assert!((number_at(value_policy, "model_expected_net_benefit_under_assumption").unwrap() - 46.5).abs() < EPSILON);
        assert!((number_at(value_policy, "model_expected_residual_harm_on_issued_pool").unwrap() - 216.5).abs() < EPSILON);
        assert_eq!(receipt["claims"]["causal_impact_established"], false);
    }

    #[test]
    fn no_action_baseline_has_zero_action_cost_and_net_benefit() {
        let receipt = evaluate(&fixture()).expect("synthetic action-value contract is valid");
        let scenario = scenario(&receipt, 2, 1.0, 1.0);
        let baseline = &scenario["policies"][0];
        assert_eq!(baseline["selected_action_count"], 0);
        assert_eq!(baseline["assumed_action_cost"], 0.0);
        assert_eq!(baseline["model_expected_net_benefit_under_assumption"], 0.0);
        assert!((number_at(baseline, "model_expected_residual_harm_on_issued_pool").unwrap() - 288.0).abs() < EPSILON);
    }

    #[test]
    fn abstained_rows_are_never_eligible_for_an_action() {
        let value = fixture();
        let rows = value["rows"].as_array().unwrap().iter().collect::<Vec<_>>();
        let selected = select_indices(&rows, AllocationPolicy::RiskRanked, 99, 1.0, 1.0);
        assert!(selected.iter().all(|index| rows[*index]["forecast_disposition"] == "forecast_issued"));
        assert!(!selected.iter().any(|index| rows[*index]["case_id"] == "case-006" || rows[*index]["case_id"] == "case-008"));
    }

    #[test]
    fn label_maturity_and_unknown_outcomes_are_not_imputed() {
        let receipt = evaluate(&fixture()).expect("synthetic action-value contract is valid");
        assert_eq!(receipt["all_case_known_outcome_count"], 7);
        assert_eq!(receipt["all_case_observed_event_count"], 5);
        assert_eq!(receipt["claims"]["assumed_harm_avoided_is_observed_impact"], false);
    }

    #[test]
    fn target_mismatch_is_rejected_before_policy_evaluation() {
        let mut value = fixture();
        value["rows"][0]["target_id"] = json!("different-estimand");
        let errors = evaluate(&value).expect_err("mixed estimands cannot enter one decision benchmark");
        assert!(errors.iter().any(|error| error.contains("mismatches the registered target")));
    }

    #[test]
    fn future_action_features_are_rejected() {
        let mut value = fixture();
        value["policy"]["action_features_cutoff_index"] = json!(10);
        let errors = evaluate(&value).expect_err("action features cannot use future information");
        assert!(errors.iter().any(|error| error.contains("strictly before forecast origin")));
    }

    #[test]
    fn unordered_sensitivity_grid_is_rejected() {
        let mut value = fixture();
        value["policy"]["capacity_grid"] = json!([0, 4, 2]);
        let errors = evaluate(&value).expect_err("capacity grid must be pinned in order");
        assert!(errors.iter().any(|error| error.contains("strictly increasing")));
    }

    #[test]
    fn historical_evaluation_requires_real_governance_and_digests() {
        let mut value = fixture();
        value["evidence_scope"] = json!("authorized_historical");
        let errors = evaluate(&value).expect_err("historical rows require approved authority");
        assert!(errors.iter().any(|error| error.contains("authorized")));
    }

    #[test]
    fn deterministic_tie_breaking_uses_case_id() {
        let mut value = fixture();
        for idx in [0, 1] {
            value["rows"][idx]["offline_candidate_probability"] = json!(0.8);
            value["rows"][idx]["served_probability"] = json!(0.8);
            value["rows"][idx]["expected_harm_per_event"] = json!(100.0);
            value["rows"][idx]["assumed_effectiveness"] = json!(0.5);
            value["rows"][idx]["action_cost"] = json!(10.0);
        }
        let rows = value["rows"].as_array().unwrap().iter().collect::<Vec<_>>();
        let selected = select_indices(&rows, AllocationPolicy::RiskRanked, 1, 1.0, 1.0);
        assert_eq!(rows[selected[0]]["case_id"], "case-001");
    }

    #[test]
    fn small_group_allocations_are_suppressed() {
        let mut value = fixture();
        value["policy"]["minimum_group_report_count"] = json!(5);
        let receipt = evaluate(&value).expect("valid group suppression rule produces receipt");
        assert_eq!(receipt["scenarios"][3]["policies"][2]["group_allocations"][0]["suppressed"], true);
        assert_eq!(receipt["scenarios"][3]["policies"][2]["group_allocations"][0]["group_id"], Value::Null);
    }
}
