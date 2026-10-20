// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Evaluates predeclared policies under identical capacity, cost and effectiveness
// assumptions. Assumed avoided harm is a model quantity, never causal evidence.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-capacity-action-value-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-capacity-action-value-synthetic-v0.1.json");
const IMPLEMENTATION_BYTES: &[u8] = include_bytes!("capacity_action_value_main.rs");

#[derive(Clone, Copy)]
enum Policy {
    NoAction,
    RiskRanked,
    ExpectedNetBenefit,
}

impl Policy {
    const ALL: [Self; 3] = [Self::NoAction, Self::RiskRanked, Self::ExpectedNetBenefit];

    fn as_str(self) -> &'static str {
        match self {
            Self::NoAction => "no_action",
            Self::RiskRanked => "risk_ranked_top_k",
            Self::ExpectedNetBenefit => "expected_net_benefit_top_k",
        }
    }
}

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

fn schema_errors(registration: &Value) -> Vec<String> {
    let schema: Value = match serde_json::from_slice(SCHEMA_BYTES) {
        Ok(value) => value,
        Err(error) => return vec![format!("embedded action-value schema is invalid: {error}")],
    };
    if let Err(error) = jsonschema::draft202012::meta::validate(&schema) {
        return vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )];
    }
    let validator = match jsonschema::draft202012::new(&schema) {
        Ok(value) => value,
        Err(error) => return vec![format!("cannot build action-value validator: {error}")],
    };
    let mut errors = validator
        .iter_errors(registration)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    errors.sort();
    errors
}

fn strictly_increasing(values: &[f64]) -> bool {
    !values.is_empty() && values.windows(2).all(|pair| pair[0] < pair[1])
}

fn semantic_errors(registration: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let origin = registration["forecast_origin_index"]
        .as_i64()
        .unwrap_or_default();
    let target = registration["target_period_index"]
        .as_i64()
        .unwrap_or_default();
    if target <= origin {
        errors.push("target period must be later than forecast origin".to_owned());
    }

    let records = registration["records"]
        .as_array()
        .expect("schema validation requires records");
    let capacity_levels = registration["preregistration"]["capacity_levels"]
        .as_array()
        .expect("schema validation requires capacity_levels")
        .iter()
        .filter_map(Value::as_u64)
        .collect::<Vec<_>>();
    if capacity_levels.windows(2).any(|pair| pair[0] >= pair[1]) {
        errors.push("capacity_levels must be strictly increasing".to_owned());
    }
    if capacity_levels
        .iter()
        .any(|capacity| *capacity as usize > records.len())
    {
        errors.push("capacity level cannot exceed the common candidate pool".to_owned());
    }

    let cost_multipliers = registration["preregistration"]["action_cost_multipliers"]
        .as_array()
        .expect("schema validation requires action_cost_multipliers")
        .iter()
        .filter_map(Value::as_f64)
        .collect::<Vec<_>>();
    if !strictly_increasing(&cost_multipliers) {
        errors.push("action_cost_multipliers must be strictly increasing".to_owned());
    }

    let effectiveness_multipliers =
        registration["preregistration"]["effectiveness_multipliers"]
            .as_array()
            .expect("schema validation requires effectiveness_multipliers")
            .iter()
            .filter_map(Value::as_f64)
            .collect::<Vec<_>>();
    if !strictly_increasing(&effectiveness_multipliers) {
        errors.push("effectiveness_multipliers must be strictly increasing".to_owned());
    }
    let maximum_effectiveness_multiplier =
        effectiveness_multipliers.iter().copied().fold(0.0_f64, f64::max);

    let mut record_ids = BTreeSet::new();
    for record in records {
        let id = string_at(record, "record_id").unwrap_or_default().to_owned();
        if !record_ids.insert(id.clone()) {
            errors.push(format!("record ID is duplicated: {id}"));
        }
        if record["target_matched"].as_bool() != Some(true) {
            errors.push(format!("record {id} does not match the registered target"));
        }
        let effectiveness = number_at(record, "base_intervention_effectiveness")
            .expect("schema validation requires base_intervention_effectiveness");
        if effectiveness * maximum_effectiveness_multiplier > 1.0 + f64::EPSILON {
            errors.push(format!(
                "record {id} has assumed effectiveness above 1 for a preregistered scenario"
            ));
        }
        if !matches!(
            string_at(record, "outcome"),
            Some("event" | "no_event" | "unknown")
        ) {
            errors.push(format!("record {id} has unsupported outcome status"));
        }
    }
    errors.sort();
    errors
}

fn record_id(record: &Value) -> &str {
    string_at(record, "record_id").expect("validated record_id")
}

fn expected_avoided_harm(record: &Value, effectiveness_multiplier: f64) -> f64 {
    number_at(record, "forecast_probability").expect("validated forecast_probability")
        * number_at(record, "expected_harm_per_event").expect("validated expected_harm_per_event")
        * number_at(record, "base_intervention_effectiveness")
            .expect("validated base_intervention_effectiveness")
        * effectiveness_multiplier
}

fn expected_action_cost(record: &Value, action_cost_multiplier: f64) -> f64 {
    number_at(record, "base_action_cost").expect("validated base_action_cost")
        * action_cost_multiplier
}

fn choose_records<'a>(
    records: &'a [Value],
    policy: Policy,
    capacity: usize,
    action_cost_multiplier: f64,
    effectiveness_multiplier: f64,
) -> Vec<&'a Value> {
    if matches!(policy, Policy::NoAction) || capacity == 0 {
        return Vec::new();
    }
    let mut ranked = records.iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        let ordering = match policy {
            Policy::NoAction => Ordering::Equal,
            Policy::RiskRanked => {
                let left_probability =
                    number_at(left, "forecast_probability").expect("validated probability");
                let right_probability =
                    number_at(right, "forecast_probability").expect("validated probability");
                right_probability
                    .partial_cmp(&left_probability)
                    .unwrap_or(Ordering::Equal)
            }
            Policy::ExpectedNetBenefit => {
                let left_net_benefit = expected_avoided_harm(left, effectiveness_multiplier)
                    - expected_action_cost(left, action_cost_multiplier);
                let right_net_benefit = expected_avoided_harm(right, effectiveness_multiplier)
                    - expected_action_cost(right, action_cost_multiplier);
                right_net_benefit
                    .partial_cmp(&left_net_benefit)
                    .unwrap_or(Ordering::Equal)
            }
        };
        ordering.then_with(|| record_id(left).cmp(record_id(right)))
    });

    if matches!(policy, Policy::ExpectedNetBenefit) {
        ranked.retain(|record| {
            expected_avoided_harm(record, effectiveness_multiplier)
                - expected_action_cost(record, action_cost_multiplier)
                > 0.0
        });
    }
    ranked.truncate(capacity);
    ranked
}

fn count_outcomes(records: &[Value]) -> Value {
    let event_count = records
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("event"))
        .count();
    let no_event_count = records
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("no_event"))
        .count();
    let unknown_count = records
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("unknown"))
        .count();
    json!({
        "event_count": event_count,
        "no_event_count": no_event_count,
        "unknown_outcome_count": unknown_count,
        "known_outcome_count": event_count + no_event_count,
        "outcome_label_coverage": (event_count + no_event_count) as f64 / records.len() as f64
    })
}

fn group_allocation(
    records: &[Value],
    selected_ids: &BTreeSet<String>,
    minimum_count: usize,
) -> (Vec<Value>, usize) {
    let mut grouped: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    for record in records {
        grouped
            .entry(
                string_at(record, "group_id")
                    .expect("validated group_id")
                    .to_owned(),
            )
            .or_default()
            .push(record);
    }
    let mut summaries = Vec::new();
    let mut suppressed = 0usize;
    for (group_id, group_records) in grouped {
        if group_records.len() < minimum_count {
            suppressed += 1;
            continue;
        }
        let selected = group_records
            .iter()
            .filter(|record| selected_ids.contains(record_id(record).to_owned().as_str()))
            .copied()
            .collect::<Vec<_>>();
        let event_count = group_records
            .iter()
            .filter(|record| string_at(record, "outcome") == Some("event"))
            .count();
        let selected_event_count = selected
            .iter()
            .filter(|record| string_at(record, "outcome") == Some("event"))
            .count();
        let unknown_count = group_records
            .iter()
            .filter(|record| string_at(record, "outcome") == Some("unknown"))
            .count();
        let selected_unknown_count = selected
            .iter()
            .filter(|record| string_at(record, "outcome") == Some("unknown"))
            .count();
        summaries.push(json!({
            "group_id": group_id,
            "candidate_count": group_records.len(),
            "selected_count": selected.len(),
            "selected_fraction": selected.len() as f64 / group_records.len() as f64,
            "known_event_count": event_count,
            "selected_known_event_count": selected_event_count,
            "unknown_outcome_count": unknown_count,
            "selected_unknown_outcome_count": selected_unknown_count
        }));
    }
    (summaries, suppressed)
}

fn evaluate_policy(
    records: &[Value],
    policy: Policy,
    capacity: usize,
    action_cost_multiplier: f64,
    effectiveness_multiplier: f64,
    minimum_group_count: usize,
) -> Value {
    let selected = choose_records(
        records,
        policy,
        capacity,
        action_cost_multiplier,
        effectiveness_multiplier,
    );
    let selected_ids = selected
        .iter()
        .map(|record| record_id(record).to_owned())
        .collect::<BTreeSet<_>>();
    let selected_count = selected.len();
    let selected_event_count = selected
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("event"))
        .count();
    let selected_no_event_count = selected
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("no_event"))
        .count();
    let selected_unknown_count = selected
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("unknown"))
        .count();
    let selected_known_count = selected_event_count + selected_no_event_count;
    let all_known_event_count = records
        .iter()
        .filter(|record| string_at(record, "outcome") == Some("event"))
        .count();
    let selected_precision = if selected_known_count == 0 {
        Value::Null
    } else {
        json!(selected_event_count as f64 / selected_known_count as f64)
    };
    let observed_event_capture = if all_known_event_count == 0 {
        Value::Null
    } else {
        json!(selected_event_count as f64 / all_known_event_count as f64)
    };

    let baseline_expected_harm = records
        .iter()
        .map(|record| {
            number_at(record, "forecast_probability").expect("probability")
                * number_at(record, "expected_harm_per_event").expect("expected harm")
        })
        .sum::<f64>();
    let avoided_harm = selected
        .iter()
        .map(|record| expected_avoided_harm(record, effectiveness_multiplier))
        .sum::<f64>();
    let action_cost = selected
        .iter()
        .map(|record| expected_action_cost(record, action_cost_multiplier))
        .sum::<f64>();
    let net_benefit = avoided_harm - action_cost;
    let residual_expected_harm = baseline_expected_harm - avoided_harm;
    let (groups, suppressed_group_count) =
        group_allocation(records, &selected_ids, minimum_group_count);

    json!({
        "policy": policy.as_str(),
        "capacity": capacity,
        "selected_count": selected_count,
        "unallocated_capacity": capacity.saturating_sub(selected_count),
        "selected_record_ids": selected.iter().map(|record| record_id(record)).collect::<Vec<_>>(),
        "expected_baseline_harm": baseline_expected_harm,
        "assumed_avoided_harm": avoided_harm,
        "expected_action_cost": action_cost,
        "expected_net_benefit": net_benefit,
        "residual_expected_harm": residual_expected_harm,
        "observed_outcome_diagnostics": {
            "selected_known_event_count": selected_event_count,
            "selected_known_no_event_count": selected_no_event_count,
            "selected_unknown_outcome_count": selected_unknown_count,
            "selected_known_outcome_count": selected_known_count,
            "precision_among_selected_known_outcomes": selected_precision,
            "event_capture_among_all_known_outcome_events": observed_event_capture,
            "all_outcome_known_event_count": all_known_event_count,
            "interpretation": "Observed label diagnostics only; unknown outcomes remain unknown and are not assumed to be no_event."
        },
        "group_allocation": groups,
        "suppressed_group_count": suppressed_group_count,
        "claims": {
            "forecast_skill_recomputed": false,
            "expected_value_is_model_based": true,
            "assumed_avoided_harm_is_observed_impact": false,
            "causal_impact_established": false,
            "live_action_authority": false
        }
    })
}

fn validate_and_evaluate(input: &[u8], source_commit: &str) -> Result<Value, Vec<String>> {
    if source_commit != "UNPINNED_LOCAL_RUN"
        && (source_commit.len() != 40
            || !source_commit
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()))
    {
        return Err(vec![
            "source commit must be a 40-character lowercase Git SHA or UNPINNED_LOCAL_RUN"
                .to_owned(),
        ]);
    }
    let registration: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("action-value registration JSON is invalid: {error}")])?;
    let mut errors = schema_errors(&registration);
    if !errors.is_empty() {
        return Err(errors);
    }
    errors = semantic_errors(&registration);
    if !errors.is_empty() {
        return Err(errors);
    }

    let records = registration["records"]
        .as_array()
        .expect("schema requires records");
    let capacities = registration["preregistration"]["capacity_levels"]
        .as_array()
        .expect("validated capacity_levels")
        .iter()
        .filter_map(Value::as_u64)
        .map(|value| value as usize)
        .collect::<Vec<_>>();
    let cost_multipliers = registration["preregistration"]["action_cost_multipliers"]
        .as_array()
        .expect("validated action_cost_multipliers")
        .iter()
        .filter_map(Value::as_f64)
        .collect::<Vec<_>>();
    let effectiveness_multipliers =
        registration["preregistration"]["effectiveness_multipliers"]
            .as_array()
            .expect("validated effectiveness_multipliers")
            .iter()
            .filter_map(Value::as_f64)
            .collect::<Vec<_>>();
    let minimum_group_count = registration["privacy"]["group_min_count"]
        .as_u64()
        .expect("validated group_min_count") as usize;

    let mut results = Vec::new();
    for capacity in &capacities {
        for action_cost_multiplier in &cost_multipliers {
            for effectiveness_multiplier in &effectiveness_multipliers {
                for policy in Policy::ALL {
                    results.push(evaluate_policy(
                        records,
                        policy,
                        *capacity,
                        *action_cost_multiplier,
                        *effectiveness_multiplier,
                        minimum_group_count,
                    ));
                }
            }
        }
    }

    let registration_id = string_at(&registration, "registration_id")
        .expect("validated registration_id");
    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let input_sha256 = sha256_hex(input);
    let implementation_sha256 = sha256_hex(IMPLEMENTATION_BYTES);
    let outcome_counts = count_outcomes(records);
    let total_expected_harm = records
        .iter()
        .map(|record| {
            number_at(record, "forecast_probability").expect("probability")
                * number_at(record, "expected_harm_per_event").expect("expected harm")
        })
        .sum::<f64>();
    let receipt_commitment_sha256 = sha256_hex(
        format!(
            "pirb-capacity-action-value-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}\n{implementation_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "capacity_constrained_action_value_evaluation",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "provider": registration["provider"]["provider_id"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "implementation_sha256": implementation_sha256,
        "forecast_archive_sha256": registration["provider"]["forecast_archive_sha256"],
        "outcome_snapshot_sha256": registration["provider"]["outcome_snapshot_sha256"],
        "capacity_levels": capacities,
        "action_cost_multipliers": cost_multipliers,
        "effectiveness_multipliers": effectiveness_multipliers,
        "policy_set": registration["preregistration"]["policy_set"],
        "candidate_count": records.len(),
        "outcome_counts": outcome_counts,
        "expected_baseline_harm_from_registered_probabilities": total_expected_harm,
        "scenario_count": capacities.len() * cost_multipliers.len() * effectiveness_multipliers.len(),
        "policy_result_count": results.len(),
        "policy_results": results,
        "metric_semantics": {
            "policy_comparison_uses_same_candidate_pool": true,
            "policy_comparison_uses_same_capacity_levels": true,
            "cost_and_effectiveness_scenarios_preregistered": true,
            "net_benefit_definition": "assumed_avoided_harm_minus_action_cost",
            "avoided_harm_definition": "sum of forecast_probability * expected_harm_per_event * base_effectiveness * effectiveness_multiplier for selected rows",
            "unknown_outcomes_imputed_to_no_event": false,
            "effectiveness_is_assumed_not_estimated": true,
            "expected_value_is_counterfactual_fact": false,
            "threshold_selected_on_test": false
        },
        "claims": {
            "contract_validated": true,
            "capacity_constrained_policy_value_computed_under_assumptions": true,
            "real_world_action_effectiveness_established": false,
            "causal_impact_established": false,
            "forecast_superiority_established": false,
            "live_dispatch_authority": false
        },
        "receipt_commitment_sha256": receipt_commitment_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-capacity-action-value-eval --input REGISTRATION.json [--source-commit SHA]"
            .to_owned()
    })?;
    let input = fs::read(&input_path).map_err(|error| {
        format!("cannot read action-value registration {input_path}: {error}")
    })?;
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
        serde_json::from_slice(FIXTURE_BYTES).expect("synthetic action-value fixture is valid JSON")
    }

    fn audit(value: &Value) -> Result<Value, Vec<String>> {
        validate_and_evaluate(
            &serde_json::to_vec(value).expect("registration serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    fn policy_result<'a>(
        receipt: &'a Value,
        capacity: usize,
        cost_multiplier: f64,
        effectiveness_multiplier: f64,
        policy: &str,
    ) -> &'a Value {
        receipt["policy_results"]
            .as_array()
            .expect("results array")
            .iter()
            .find(|result| {
                result["capacity"].as_u64() == Some(capacity as u64)
                    && result["action_cost_multiplier"].is_null()
                    && result["action_cost_multiplier"] != json!(cost_multiplier)
                    && result["policy"] == policy
                    && result["scenario"]["action_cost_multiplier"] == json!(cost_multiplier)
                    && result["scenario"]["effectiveness_multiplier"] == json!(effectiveness_multiplier)
            })
            .expect("policy scenario found")
    }

    #[test]
    fn full_scenario_grid_keeps_policy_capacity_and_assumption_lanes_explicit() {
        let receipt = validate_and_evaluate(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("synthetic scenario grid validates");
        assert_eq!(receipt["candidate_count"], 12);
        assert_eq!(receipt["scenario_count"], 27);
        assert_eq!(receipt["policy_result_count"], 81);
        assert_eq!(receipt["outcome_counts"]["event_count"], 4);
        assert_eq!(receipt["outcome_counts"]["no_event_count"], 6);
        assert_eq!(receipt["outcome_counts"]["unknown_outcome_count"], 2);
        assert_eq!(receipt["claims"]["causal_impact_established"], false);
        assert_eq!(receipt["metric_semantics"]["unknown_outcomes_imputed_to_no_event"], false);
    }

    #[test]
    fn one_action_golden_reconciles_expected_harm_cost_and_observed_capture() {
        let receipt = validate_and_evaluate(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("fixture validates");
        let result = receipt["policy_results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|result| {
                result["capacity"] == 1
                    && result["scenario"]["action_cost_multiplier"] == 1.0
                    && result["scenario"]["effectiveness_multiplier"] == 1.0
                    && result["policy"] == "risk_ranked_top_k"
            })
            .unwrap();
        assert_eq!(result["selected_record_ids"], json!(["row-01"]));
        assert!((result["expected_baseline_harm"].as_f64().unwrap() - 400.5).abs() < 1e-12);
        assert!((result["assumed_avoided_harm"].as_f64().unwrap() - 36.0).abs() < 1e-12);
        assert!((result["expected_action_cost"].as_f64().unwrap() - 12.0).abs() < 1e-12);
        assert!((result["expected_net_benefit"].as_f64().unwrap() - 24.0).abs() < 1e-12);
        assert!((result["residual_expected_harm"].as_f64().unwrap() - 364.5).abs() < 1e-12);
        assert!((result["observed_outcome_diagnostics"]["event_capture_among_all_known_outcome_events"].as_f64().unwrap() - 0.25).abs() < 1e-12);
        assert_eq!(result["observed_outcome_diagnostics"]["selected_unknown_outcome_count"], 0);
    }

    #[test]
    fn expected_net_benefit_policy_abstains_when_all_actions_are_nonpositive() {
        let receipt = validate_and_evaluate(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("fixture validates");
        let result = receipt["policy_results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|result| {
                result["capacity"] == 2
                    && result["scenario"]["action_cost_multiplier"] == 1.5
                    && result["scenario"]["effectiveness_multiplier"] == 0.5
                    && result["policy"] == "expected_net_benefit_top_k"
            })
            .unwrap();
        assert_eq!(result["selected_count"], 0);
        assert_eq!(result["expected_net_benefit"], 0.0);
        assert_eq!(result["unallocated_capacity"], 2);
    }

    #[test]
    fn risk_ranked_and_value_ranked_policies_are_compared_under_same_capacity() {
        let receipt = validate_and_evaluate(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("fixture validates");
        let results = receipt["policy_results"].as_array().unwrap();
        for capacity in [1, 2, 3] {
            let risk_count = results.iter().filter(|result| {
                result["capacity"] == capacity && result["policy"] == "risk_ranked_top_k"
            }).count();
            let value_count = results.iter().filter(|result| {
                result["capacity"] == capacity && result["policy"] == "expected_net_benefit_top_k"
            }).count();
            assert_eq!(risk_count, 9);
            assert_eq!(value_count, 9);
        }
        assert!(results.iter().all(|result| {
            result["selected_count"].as_u64().unwrap() <= result["capacity"].as_u64().unwrap()
        }));
    }

    #[test]
    fn risk_ranked_ties_are_deterministic_by_opaque_record_id() {
        let mut value = fixture();
        for record in value["records"].as_array_mut().unwrap() {
            record["forecast_probability"] = json!(0.5);
        }
        let receipt = audit(&value).expect("tie fixture validates");
        let result = receipt["policy_results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|result| {
                result["capacity"] == 1
                    && result["scenario"]["action_cost_multiplier"] == 1.0
                    && result["scenario"]["effectiveness_multiplier"] == 1.0
                    && result["policy"] == "risk_ranked_top_k"
            })
            .unwrap();
        assert_eq!(result["selected_record_ids"], json!(["row-01"]));
    }

    #[test]
    fn unknown_outcomes_are_reported_and_never_promoted_to_no_event() {
        let receipt = validate_and_evaluate(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("fixture validates");
        assert_eq!(receipt["outcome_counts"]["unknown_outcome_count"], 2);
        let results = receipt["policy_results"].as_array().unwrap();
        assert!(results.iter().any(|result| {
            result["observed_outcome_diagnostics"]["selected_unknown_outcome_count"].as_u64().unwrap() > 0
        }));
        assert_eq!(receipt["metric_semantics"]["unknown_outcomes_imputed_to_no_event"], false);
    }

    #[test]
    fn target_mismatch_and_policy_threshold_tuning_fail_closed() {
        let mut value = fixture();
        value["records"][0]["target_matched"] = json!(false);
        let errors = audit(&value).expect_err("target mismatch must fail closed");
        assert!(errors.iter().any(|error| error.contains("target_matched")));
        let mut value = fixture();
        value["preregistration"]["threshold_selection_on_test"] = json!(true);
        let errors = audit(&value).expect_err("test-selected threshold is inadmissible");
        assert!(errors.iter().any(|error| error.contains("threshold_selection_on_test")));
    }

    #[test]
    fn nonmonotone_scenario_grid_is_rejected() {
        let mut value = fixture();
        value["preregistration"]["capacity_levels"] = json!([2, 1, 3]);
        let errors = audit(&value).expect_err("capacity grid must be ordered");
        assert!(errors.iter().any(|error| error.contains("strictly increasing")));
    }

    #[test]
    fn effectiveness_assumptions_cannot_exceed_one_under_any_registered_scenario() {
        let mut value = fixture();
        value["records"][0]["base_intervention_effectiveness"] = json!(0.7);
        let errors = audit(&value).expect_err("assumed effectiveness cannot exceed 1");
        assert!(errors.iter().any(|error| error.contains("above 1")));
    }

    #[test]
    fn fixed_input_replay_yields_identical_receipt_and_policy_order() {
        let first = validate_and_evaluate(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("first replay validates");
        let second = validate_and_evaluate(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("second replay validates");
        assert_eq!(first, second);
    }
}
