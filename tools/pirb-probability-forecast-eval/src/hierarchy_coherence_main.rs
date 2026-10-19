// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Audits hierarchical count-forecast coherence only. Accuracy, decision utility,
// source authorization, and causal impact are separate evidence classes.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process,
};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-hierarchy-forecast-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-hierarchy-forecast-synthetic-v0.1.json");

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

fn semantic_errors(registration: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let origin = registration["forecast_origin_index"].as_i64().unwrap_or_default();
    let target = registration["target_period_index"].as_i64().unwrap_or_default();
    let cutoff = registration["reconciliation_policy"]["parameter_fit_cutoff_index"]
        .as_i64()
        .unwrap_or_default();

    if cutoff >= origin {
        errors.push(
            "reconciliation parameter fit cutoff must be strictly before forecast origin"
                .to_owned(),
        );
    }
    if target <= origin {
        errors.push("target period must be later than forecast origin".to_owned());
    }

    let Some(forecasts) = registration.get("forecasts").and_then(Value::as_array) else {
        return vec!["forecasts must be an array after schema validation".to_owned()];
    };
    let Some(constraints) = registration
        .get("aggregation_constraints")
        .and_then(Value::as_array)
    else {
        return vec![
            "aggregation_constraints must be an array after schema validation".to_owned(),
        ];
    };

    let mut forecast_by_id = BTreeMap::new();
    for forecast in forecasts {
        let id = string_at(forecast, "forecast_id").unwrap_or_default();
        if forecast_by_id.insert(id.to_owned(), forecast).is_some() {
            errors.push(format!("forecast ID is duplicated: {id}"));
        }
        if forecast["period_index"].as_i64() != Some(target) {
            errors.push(format!(
                "forecast {id} period_index must equal registered target_period_index"
            ));
        }
    }

    let mut constraint_ids = BTreeSet::new();
    for constraint in constraints {
        let id = string_at(constraint, "constraint_id").unwrap_or_default();
        if !constraint_ids.insert(id.to_owned()) {
            errors.push(format!("constraint ID is duplicated: {id}"));
        }

        let parent_id = string_at(constraint, "parent_forecast_id").unwrap_or_default();
        let parent = forecast_by_id.get(parent_id).copied();
        if parent.is_none() {
            errors.push(format!(
                "constraint {id} references unknown parent forecast {parent_id}"
            ));
        }

        let child_ids = constraint["child_forecast_ids"]
            .as_array()
            .expect("schema validation requires child_forecast_ids");
        let completeness = string_at(constraint, "completeness").unwrap_or_default();
        for child_id in child_ids.iter().filter_map(Value::as_str) {
            if child_id == parent_id {
                errors.push(format!(
                    "constraint {id} cannot use its parent as a child"
                ));
            }
            if !forecast_by_id.contains_key(child_id) && completeness == "complete" {
                errors.push(format!(
                    "constraint {id} claims complete child coverage but forecast {child_id} is missing"
                ));
            }
        }

        let Some(parent) = parent else {
            continue;
        };
        let kind = string_at(constraint, "kind").unwrap_or_default();
        let mut seen_child_units = BTreeSet::new();
        let mut seen_child_events = BTreeSet::new();
        for child_id in child_ids.iter().filter_map(Value::as_str) {
            let Some(child) = forecast_by_id.get(child_id).copied() else {
                continue;
            };
            if child["period_index"] != parent["period_index"] {
                errors.push(format!(
                    "constraint {id} contains parent/child forecasts from different periods"
                ));
            }
            match kind {
                "spatial" => {
                    if string_at(child, "event_type_id") != string_at(parent, "event_type_id") {
                        errors.push(format!(
                            "spatial constraint {id} mixes event_type_id values"
                        ));
                    }
                    if !seen_child_units.insert(string_at(child, "unit_id").unwrap_or_default()) {
                        errors.push(format!(
                            "spatial constraint {id} repeats a child unit"
                        ));
                    }
                }
                "event_type" => {
                    if string_at(child, "unit_id") != string_at(parent, "unit_id") {
                        errors.push(format!(
                            "event-type constraint {id} mixes unit_id values"
                        ));
                    }
                    if string_at(child, "event_type_id") == string_at(parent, "event_type_id") {
                        errors.push(format!(
                            "event-type constraint {id} repeats the parent event_type_id as a child"
                        ));
                    }
                    if !seen_child_events
                        .insert(string_at(child, "event_type_id").unwrap_or_default())
                    {
                        errors.push(format!(
                            "event-type constraint {id} repeats a child event type"
                        ));
                    }
                }
                _ => errors.push(format!("constraint {id} has an unsupported aggregation kind")),
            }
        }
    }

    errors.sort();
    errors
}

fn validate_and_audit(input: &[u8], source_commit: &str) -> Result<Value, Vec<String>> {
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
        .map_err(|error| vec![format!("hierarchy registration JSON is invalid: {error}")])?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES)
        .map_err(|error| vec![format!("embedded hierarchy schema JSON is invalid: {error}")])?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!("Draft 2020-12 schema meta-validation failed: {error}")]
    })?;
    let validator = jsonschema::draft202012::new(&schema).map_err(|error| {
        vec![format!("cannot build hierarchy schema validator: {error}")]
    })?;
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

    let forecasts = registration["forecasts"]
        .as_array()
        .expect("schema validation requires forecasts");
    let forecast_by_id = forecasts
        .iter()
        .map(|forecast| {
            (
                string_at(forecast, "forecast_id")
                    .expect("schema validation requires forecast_id"),
                forecast,
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut diagnostics = Vec::new();
    let mut assessable = 0usize;
    let mut raw_coherent = 0usize;
    let mut reconciled_coherent = 0usize;
    let mut reconciled_incoherent = 0usize;
    let mut not_assessable = 0usize;
    let mut absolute_raw_residuals = Vec::new();
    let mut absolute_reconciled_residuals = Vec::new();

    for constraint in registration["aggregation_constraints"]
        .as_array()
        .expect("schema validation requires aggregation_constraints")
    {
        let constraint_id =
            string_at(constraint, "constraint_id").expect("schema validated constraint_id");
        let completeness =
            string_at(constraint, "completeness").expect("schema validated completeness");
        let tolerance = number_at(constraint, "absolute_tolerance")
            .expect("schema validated absolute_tolerance");
        let parent_id = string_at(constraint, "parent_forecast_id")
            .expect("schema validated parent_forecast_id");
        let parent = forecast_by_id
            .get(parent_id)
            .expect("semantic validation requires parent forecast");
        let child_ids = constraint["child_forecast_ids"]
            .as_array()
            .expect("schema validated child_forecast_ids");
        let missing_child_ids = child_ids
            .iter()
            .filter_map(Value::as_str)
            .filter(|child_id| !forecast_by_id.contains_key(child_id))
            .collect::<Vec<_>>();

        if completeness != "complete" {
            not_assessable += 1;
            diagnostics.push(json!({
                "constraint_id": constraint_id,
                "kind": constraint["kind"],
                "completeness": completeness,
                "status": "not_assessable",
                "raw_parent_count": Value::Null,
                "raw_child_sum": Value::Null,
                "raw_signed_residual": Value::Null,
                "raw_status": "not_assessable",
                "reconciled_parent_count": Value::Null,
                "reconciled_child_sum": Value::Null,
                "reconciled_signed_residual": Value::Null,
                "reconciled_status": "not_assessable",
                "absolute_tolerance": tolerance,
                "missing_child_forecast_ids": missing_child_ids
            }));
            continue;
        }

        let children = child_ids
            .iter()
            .map(|child_id| {
                forecast_by_id
                    .get(child_id.as_str().expect("schema validated child ID"))
                    .copied()
                    .expect("complete constraints have every child")
            })
            .collect::<Vec<_>>();
        let raw_sum = children
            .iter()
            .map(|child| number_at(child, "raw_count").expect("schema validated raw_count"))
            .sum::<f64>();
        let reconciled_sum = children
            .iter()
            .map(|child| {
                number_at(child, "reconciled_count").expect("schema validated reconciled_count")
            })
            .sum::<f64>();
        let raw_parent = number_at(parent, "raw_count").expect("schema validated raw_count");
        let reconciled_parent =
            number_at(parent, "reconciled_count").expect("schema validated reconciled_count");
        let raw_residual = raw_parent - raw_sum;
        let reconciled_residual = reconciled_parent - reconciled_sum;
        let raw_status = if raw_residual.abs() <= tolerance {
            raw_coherent += 1;
            "coherent"
        } else {
            "incoherent"
        };
        let reconciled_status = if reconciled_residual.abs() <= tolerance {
            reconciled_coherent += 1;
            "coherent"
        } else {
            reconciled_incoherent += 1;
            "incoherent"
        };
        assessable += 1;
        absolute_raw_residuals.push(raw_residual.abs());
        absolute_reconciled_residuals.push(reconciled_residual.abs());
        diagnostics.push(json!({
            "constraint_id": constraint_id,
            "kind": constraint["kind"],
            "completeness": completeness,
            "status": reconciled_status,
            "raw_parent_count": raw_parent,
            "raw_child_sum": raw_sum,
            "raw_signed_residual": raw_residual,
            "raw_status": raw_status,
            "reconciled_parent_count": reconciled_parent,
            "reconciled_child_sum": reconciled_sum,
            "reconciled_signed_residual": reconciled_residual,
            "reconciled_status": reconciled_status,
            "absolute_tolerance": tolerance,
            "missing_child_forecast_ids": missing_child_ids
        }));
    }

    let total_absolute_adjustment = forecasts
        .iter()
        .map(|forecast| {
            (number_at(forecast, "reconciled_count").unwrap_or_default()
                - number_at(forecast, "raw_count").unwrap_or_default())
            .abs()
        })
        .sum::<f64>();
    let maximum_absolute_adjustment = forecasts
        .iter()
        .map(|forecast| {
            (number_at(forecast, "reconciled_count").unwrap_or_default()
                - number_at(forecast, "raw_count").unwrap_or_default())
            .abs()
        })
        .fold(0.0_f64, f64::max);
    let mean_absolute_raw_residual = if absolute_raw_residuals.is_empty() {
        Value::Null
    } else {
        json!(
            absolute_raw_residuals.iter().sum::<f64>() / absolute_raw_residuals.len() as f64
        )
    };
    let mean_absolute_reconciled_residual = if absolute_reconciled_residuals.is_empty() {
        Value::Null
    } else {
        json!(
            absolute_reconciled_residuals.iter().sum::<f64>()
                / absolute_reconciled_residuals.len() as f64
        )
    };
    let reconciled_coherence_rate = if assessable == 0 {
        Value::Null
    } else {
        json!(reconciled_coherent as f64 / assessable as f64)
    };
    let raw_coherence_rate = if assessable == 0 {
        Value::Null
    } else {
        json!(raw_coherent as f64 / assessable as f64)
    };

    let registration_id =
        string_at(&registration, "registration_id").expect("schema validated registration_id");
    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let input_sha256 = sha256_hex(input);
    let receipt_commitment_sha256 = sha256_hex(
        format!(
            "pirb-hierarchy-coherence-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "hierarchy_coherence_audit",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "provider": registration["provider"]["provider_id"],
        "forecast_release_id": registration["provider"]["forecast_release_id"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "forecast_artifact_sha256": registration["provider"]["forecast_artifact_sha256"],
        "hierarchy_sha256": registration["provider"]["hierarchy_sha256"],
        "reconciliation_method_sha256": registration["reconciliation_policy"]["method_sha256"],
        "constraint_diagnostics": diagnostics,
        "summary": {
            "constraint_count": diagnostics.len(),
            "assessable_constraint_count": assessable,
            "raw_coherent_constraint_count": raw_coherent,
            "reconciled_coherent_constraint_count": reconciled_coherent,
            "reconciled_incoherent_constraint_count": reconciled_incoherent,
            "not_assessable_constraint_count": not_assessable,
            "raw_coherence_rate": raw_coherence_rate,
            "reconciled_coherence_rate": reconciled_coherence_rate,
            "mean_absolute_raw_residual": mean_absolute_raw_residual,
            "mean_absolute_reconciled_residual": mean_absolute_reconciled_residual,
            "total_absolute_reconciliation_adjustment": total_absolute_adjustment,
            "maximum_absolute_reconciliation_adjustment": maximum_absolute_adjustment
        },
        "claims": {
            "contract_validated": true,
            "hierarchy_coherence_measured": true,
            "forecast_accuracy_established": false,
            "decision_utility_established": false,
            "causal_impact_established": false,
            "live_decision_authority": false
        },
        "receipt_commitment_sha256": receipt_commitment_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?
        .ok_or_else(|| {
            "usage: pirb-hierarchy-coherence-audit --input REGISTRATION.json [--source-commit SHA]"
                .to_owned()
        })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read hierarchy registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let receipt = validate_and_audit(&input, &source_commit)
        .map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize hierarchy receipt: {error}"))?
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
        serde_json::from_slice(FIXTURE_BYTES).expect("checked-in synthetic fixture is valid JSON")
    }

    fn audit(value: &Value) -> Result<Value, Vec<String>> {
        validate_and_audit(
            &serde_json::to_vec(value).expect("fixture serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    #[test]
    fn fixture_reports_spatial_and_event_type_coherence_without_filling_missing_children() {
        let receipt = validate_and_audit(
            FIXTURE_BYTES,
            "0123456789abcdef0123456789abcdef01234567",
        )
        .expect("synthetic fixture contract is valid");
        assert_eq!(receipt["summary"]["constraint_count"], 3);
        assert_eq!(receipt["summary"]["assessable_constraint_count"], 2);
        assert_eq!(receipt["summary"]["reconciled_coherent_constraint_count"], 2);
        assert_eq!(receipt["summary"]["not_assessable_constraint_count"], 1);
        assert_eq!(
            receipt["constraint_diagnostics"][2]["status"],
            "not_assessable"
        );
        assert_eq!(
            receipt["constraint_diagnostics"][2]["missing_child_forecast_ids"][0],
            "fc-admin1-d-organized"
        );
        assert_eq!(receipt["claims"]["forecast_accuracy_established"], false);
    }

    #[test]
    fn raw_and_reconciled_residuals_are_reported_separately() {
        let receipt = validate_and_audit(
            FIXTURE_BYTES,
            "0123456789abcdef0123456789abcdef01234567",
        )
        .expect("synthetic fixture contract is valid");
        assert_eq!(
            receipt["constraint_diagnostics"][0]["raw_signed_residual"],
            0.0
        );
        assert_eq!(
            receipt["constraint_diagnostics"][0]["reconciled_signed_residual"],
            0.0
        );
        assert_eq!(
            receipt["summary"]["total_absolute_reconciliation_adjustment"],
            3.0
        );
    }

    #[test]
    fn post_reconciliation_incoherence_is_reported_not_hidden() {
        let mut value = fixture();
        value["forecasts"][2]["reconciled_count"] = json!(10);
        let receipt = audit(&value).expect("a score failure is a reportable benchmark result");
        assert_eq!(
            receipt["constraint_diagnostics"][0]["reconciled_status"],
            "incoherent"
        );
        assert_eq!(
            receipt["constraint_diagnostics"][0]["reconciled_signed_residual"],
            -1.0
        );
    }

    #[test]
    fn complete_constraint_with_a_missing_child_is_rejected() {
        let mut value = fixture();
        value["aggregation_constraints"][2]["completeness"] = json!("complete");
        let errors = audit(&value).expect_err("complete coverage cannot hide a missing child");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("claims complete child coverage"))
        );
    }

    #[test]
    fn future_reconciliation_fit_cutoff_is_rejected() {
        let mut value = fixture();
        value["reconciliation_policy"]["parameter_fit_cutoff_index"] = json!(10);
        let errors = audit(&value).expect_err("future fit data is leakage");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("fit cutoff must be strictly before forecast origin"))
        );
    }

    #[test]
    fn spatial_aggregation_cannot_mix_event_type_ids() {
        let mut value = fixture();
        value["forecasts"][1]["event_type_id"] = json!("event-battles");
        let errors = audit(&value).expect_err("spatial sums require one target event type");
        assert!(errors.iter().any(|error| error.contains("mixes event_type_id")));
    }

    #[test]
    fn event_type_aggregation_cannot_mix_units() {
        let mut value = fixture();
        value["forecasts"][3]["unit_id"] = json!("unit-country-b");
        let errors = audit(&value).expect_err("event-type sums require one spatial unit");
        assert!(errors.iter().any(|error| error.contains("mixes unit_id")));
    }

    #[test]
    fn duplicate_forecast_ids_are_rejected() {
        let mut value = fixture();
        value["forecasts"][1]["forecast_id"] = value["forecasts"][0]["forecast_id"].clone();
        let errors = audit(&value).expect_err("duplicate IDs must not silently overwrite rows");
        assert!(errors.iter().any(|error| error.contains("forecast ID is duplicated")));
    }
}
