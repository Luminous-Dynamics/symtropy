// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Audits joint predictive sample coherence only. It does not establish forecast
// quality, interval calibration, operational utility, or causal impact.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env, fs, process,
};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-joint-sample-forecast-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-joint-sample-forecast-synthetic-v0.1.json");

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

fn graph_has_cycle(nodes: &BTreeMap<String, &Value>, constraints: &[Value]) -> bool {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = nodes
        .keys()
        .map(|id| (id.clone(), BTreeSet::new()))
        .collect();

    for constraint in constraints {
        let Some(parent_id) = string_at(constraint, "parent_forecast_id") else {
            continue;
        };
        if !nodes.contains_key(parent_id) {
            continue;
        }
        for child_id in constraint["child_forecast_ids"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if nodes.contains_key(child_id) {
                adjacency
                    .entry(parent_id.to_owned())
                    .or_default()
                    .insert(child_id.to_owned());
            }
        }
    }

    let mut indegree: BTreeMap<String, usize> = nodes.keys().map(|id| (id.clone(), 0)).collect();
    for children in adjacency.values() {
        for child in children {
            *indegree.entry(child.clone()).or_default() += 1;
        }
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(node, degree)| (*degree == 0).then_some(node.clone()))
        .collect::<VecDeque<_>>();
    let mut visited = 0usize;
    while let Some(node) = ready.pop_front() {
        visited += 1;
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                let degree = indegree
                    .get_mut(child)
                    .expect("graph child originates from a declared forecast node");
                *degree -= 1;
                if *degree == 0 {
                    ready.push_back(child.clone());
                }
            }
        }
    }
    visited != nodes.len()
}

fn semantic_errors(registration: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let origin = registration["forecast_origin_index"]
        .as_i64()
        .unwrap_or_default();
    let target = registration["target_period_index"]
        .as_i64()
        .unwrap_or_default();
    let policy = &registration["sample_policy"];
    let cutoff = policy["parameter_fit_cutoff_index"]
        .as_i64()
        .unwrap_or_default();
    if target <= origin {
        errors.push("target period must be later than forecast origin".to_owned());
    }
    if cutoff >= origin {
        errors.push(
            "sample/reconciliation fit cutoff must be strictly before forecast origin".to_owned(),
        );
    }

    let declared_draw_ids = policy["draw_ids"]
        .as_array()
        .expect("schema validation requires draw_ids");
    let draw_ids = declared_draw_ids
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if policy["expected_draw_count"].as_u64() != Some(draw_ids.len() as u64) {
        errors.push("expected_draw_count does not match the unique declared draw_ids".to_owned());
    }

    let forecast_nodes = registration["forecast_nodes"]
        .as_array()
        .expect("schema validation requires forecast_nodes");
    let constraints = registration["aggregation_constraints"]
        .as_array()
        .expect("schema validation requires aggregation_constraints");
    let sample_rows = registration["sample_rows"]
        .as_array()
        .expect("schema validation requires sample_rows");

    let mut node_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    for node in forecast_nodes {
        let id = string_at(node, "forecast_id").unwrap_or_default();
        if node_by_id.insert(id.to_owned(), node).is_some() {
            errors.push(format!("forecast node ID is duplicated: {id}"));
        }
        if node["period_index"].as_i64() != Some(target) {
            errors.push(format!(
                "forecast node {id} period_index must equal target_period_index"
            ));
        }
    }

    let mut rows_by_key: BTreeMap<(String, String), &Value> = BTreeMap::new();
    for row in sample_rows {
        let draw_id = string_at(row, "draw_id").unwrap_or_default();
        let forecast_id = string_at(row, "forecast_id").unwrap_or_default();
        if !draw_ids.contains(draw_id) {
            errors.push(format!(
                "sample row references undeclared draw_id {draw_id}"
            ));
        }
        if !node_by_id.contains_key(forecast_id) {
            errors.push(format!(
                "sample row references unknown forecast_id {forecast_id}"
            ));
        }
        let key = (draw_id.to_owned(), forecast_id.to_owned());
        if rows_by_key.insert(key, row).is_some() {
            errors.push(format!(
                "duplicate sample row for draw_id {draw_id}, forecast_id {forecast_id}"
            ));
        }
    }

    for draw_id in &draw_ids {
        for forecast_id in node_by_id.keys() {
            if !rows_by_key.contains_key(&((*draw_id).to_owned(), forecast_id.clone())) {
                errors.push(format!(
                    "missing sample row for draw_id {draw_id}, forecast_id {forecast_id}; missing rows are never filled with zero"
                ));
            }
        }
    }

    let mut constraint_ids = BTreeSet::new();
    for constraint in constraints {
        let id = string_at(constraint, "constraint_id").unwrap_or_default();
        if !constraint_ids.insert(id.to_owned()) {
            errors.push(format!("constraint ID is duplicated: {id}"));
        }
        let kind = string_at(constraint, "kind").unwrap_or_default();
        let completeness = string_at(constraint, "completeness").unwrap_or_default();
        let parent_id = string_at(constraint, "parent_forecast_id").unwrap_or_default();
        let parent = node_by_id.get(parent_id).copied();
        if parent.is_none() {
            errors.push(format!(
                "constraint {id} references unknown parent forecast {parent_id}"
            ));
        }

        let child_ids = constraint["child_forecast_ids"]
            .as_array()
            .expect("schema validation requires child_forecast_ids");
        for child_id in child_ids.iter().filter_map(Value::as_str) {
            if child_id == parent_id {
                errors.push(format!("constraint {id} cannot use parent as a child"));
            }
            if completeness == "complete" && !node_by_id.contains_key(child_id) {
                errors.push(format!(
                    "constraint {id} claims complete coverage but child forecast {child_id} is missing"
                ));
            }
        }
        let expected_child_keys = constraint["expected_child_keys"]
            .as_array()
            .expect("schema validation requires expected_child_keys")
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        let actual_child_keys = child_ids
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|child_id| node_by_id.get(child_id).copied())
            .filter_map(|child| match kind {
                "spatial" => string_at(child, "unit_id"),
                "event_type" => string_at(child, "event_type_id"),
                _ => None,
            })
            .collect::<BTreeSet<_>>();

        if completeness == "complete" && actual_child_keys != expected_child_keys {
            let missing = expected_child_keys
                .difference(&actual_child_keys)
                .copied()
                .collect::<Vec<_>>();
            let unexpected = actual_child_keys
                .difference(&expected_child_keys)
                .copied()
                .collect::<Vec<_>>();
            errors.push(format!(
                "constraint {id} claims complete coverage but expected_child_keys differ; missing {missing:?}, unexpected {unexpected:?}"
            ));
        }

        if let Some(parent) = parent {
            let mut seen_child_units = BTreeSet::new();
            let mut seen_child_events = BTreeSet::new();
            for child_id in child_ids.iter().filter_map(Value::as_str) {
                let Some(child) = node_by_id.get(child_id).copied() else {
                    continue;
                };
                if child["period_index"] != parent["period_index"] {
                    errors.push(format!(
                        "constraint {id} contains parent/child nodes from different periods"
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
                            errors.push(format!("spatial constraint {id} repeats a child unit"));
                        }
                    }
                    "event_type" => {
                        if string_at(child, "unit_id") != string_at(parent, "unit_id") {
                            errors.push(format!("event-type constraint {id} mixes unit_id values"));
                        }
                        if string_at(child, "event_type_id") == string_at(parent, "event_type_id") {
                            errors.push(format!(
                                "event-type constraint {id} repeats the parent event_type_id"
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
                    _ => errors.push(format!(
                        "constraint {id} has unsupported aggregation kind {kind}"
                    )),
                }
            }
        }
    }

    if graph_has_cycle(&node_by_id, constraints) {
        errors.push("aggregation constraint graph contains a cycle".to_owned());
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

    let registration: Value = serde_json::from_slice(input).map_err(|error| {
        vec![format!(
            "joint-sample registration JSON is invalid: {error}"
        )]
    })?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES).map_err(|error| {
        vec![format!(
            "embedded joint-sample schema JSON is invalid: {error}"
        )]
    })?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )]
    })?;
    let validator = jsonschema::draft202012::new(&schema).map_err(|error| {
        vec![format!(
            "cannot build joint-sample schema validator: {error}"
        )]
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

    let draw_ids = registration["sample_policy"]["draw_ids"]
        .as_array()
        .expect("schema validated draw_ids");
    let forecasts = registration["forecast_nodes"]
        .as_array()
        .expect("schema validated forecast_nodes");
    let constraints = registration["aggregation_constraints"]
        .as_array()
        .expect("schema validated aggregation_constraints");
    let rows = registration["sample_rows"]
        .as_array()
        .expect("schema validated sample_rows");
    let nodes_by_id = forecasts
        .iter()
        .map(|node| {
            (
                string_at(node, "forecast_id").expect("schema validated forecast_id"),
                node,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let rows_by_key = rows
        .iter()
        .map(|row| {
            (
                (
                    string_at(row, "draw_id").expect("schema validated draw_id"),
                    string_at(row, "forecast_id").expect("schema validated forecast_id"),
                ),
                row,
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut constraint_diagnostics = Vec::new();
    let mut assessed_constraint_draw_pairs = 0usize;
    let mut raw_coherent_constraint_draw_pairs = 0usize;
    let mut reconciled_coherent_constraint_draw_pairs = 0usize;
    let mut absolute_raw_residuals = Vec::new();
    let mut absolute_reconciled_residuals = Vec::new();

    let mut raw_all_by_draw: BTreeMap<&str, bool> = draw_ids
        .iter()
        .filter_map(Value::as_str)
        .map(|draw_id| (draw_id, true))
        .collect();
    let mut reconciled_all_by_draw = raw_all_by_draw.clone();

    for constraint in constraints {
        let id = string_at(constraint, "constraint_id").expect("schema validated constraint_id");
        let kind = string_at(constraint, "kind").expect("schema validated kind");
        let completeness =
            string_at(constraint, "completeness").expect("schema validated completeness");
        let tolerance = number_at(constraint, "absolute_tolerance")
            .expect("schema validated absolute_tolerance");
        let parent_id = string_at(constraint, "parent_forecast_id")
            .expect("schema validated parent_forecast_id");
        let child_ids = constraint["child_forecast_ids"]
            .as_array()
            .expect("schema validated child_forecast_ids");
        let expected_child_keys = constraint["expected_child_keys"]
            .as_array()
            .expect("schema validated expected_child_keys")
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        let actual_child_keys = child_ids
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|child_id| nodes_by_id.get(child_id).copied())
            .filter_map(|child| match kind {
                "spatial" => string_at(child, "unit_id"),
                "event_type" => string_at(child, "event_type_id"),
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let missing_child_forecast_ids = child_ids
            .iter()
            .filter_map(Value::as_str)
            .filter(|child_id| !nodes_by_id.contains_key(child_id))
            .collect::<Vec<_>>();
        let missing_expected_child_keys = expected_child_keys
            .difference(&actual_child_keys)
            .copied()
            .collect::<Vec<_>>();
        let unexpected_child_keys = actual_child_keys
            .difference(&expected_child_keys)
            .copied()
            .collect::<Vec<_>>();

        if completeness != "complete" {
            constraint_diagnostics.push(json!({
                "constraint_id": id,
                "kind": kind,
                "completeness": completeness,
                "status": "not_assessable",
                "draw_diagnostics": Value::Null,
                "assessed_draw_count": 0,
                "raw_coherent_draw_count": 0,
                "reconciled_coherent_draw_count": 0,
                "missing_child_forecast_ids": missing_child_forecast_ids,
                "expected_child_keys": expected_child_keys,
                "missing_expected_child_keys": missing_expected_child_keys,
                "unexpected_child_keys": unexpected_child_keys,
                "absolute_tolerance": tolerance
            }));
            continue;
        }

        let mut draw_diagnostics = Vec::new();
        let mut constraint_raw_coherent = 0usize;
        let mut constraint_reconciled_coherent = 0usize;
        for draw_value in draw_ids {
            let draw_id = draw_value.as_str().expect("schema validated draw_id");
            let parent_row = rows_by_key
                .get(&(draw_id, parent_id))
                .expect("semantic validation enforces a complete sample matrix");
            let raw_parent =
                number_at(parent_row, "raw_count").expect("schema validated raw_count");
            let reconciled_parent = number_at(parent_row, "reconciled_count")
                .expect("schema validated reconciled_count");
            let children = child_ids
                .iter()
                .map(|child_value| {
                    let child_id = child_value
                        .as_str()
                        .expect("schema validated child forecast ID");
                    rows_by_key
                        .get(&(draw_id, child_id))
                        .expect("semantic validation enforces child sample rows")
                })
                .collect::<Vec<_>>();
            let raw_child_sum = children
                .iter()
                .map(|row| number_at(row, "raw_count").expect("schema validated raw_count"))
                .sum::<f64>();
            let reconciled_child_sum = children
                .iter()
                .map(|row| {
                    number_at(row, "reconciled_count").expect("schema validated reconciled_count")
                })
                .sum::<f64>();
            let raw_residual = raw_parent - raw_child_sum;
            let reconciled_residual = reconciled_parent - reconciled_child_sum;
            let raw_status = if raw_residual.abs() <= tolerance {
                constraint_raw_coherent += 1;
                raw_coherent_constraint_draw_pairs += 1;
                "coherent"
            } else {
                *raw_all_by_draw
                    .get_mut(draw_id)
                    .expect("registered draw has a joint status") = false;
                "incoherent"
            };
            let reconciled_status = if reconciled_residual.abs() <= tolerance {
                constraint_reconciled_coherent += 1;
                reconciled_coherent_constraint_draw_pairs += 1;
                "coherent"
            } else {
                *reconciled_all_by_draw
                    .get_mut(draw_id)
                    .expect("registered draw has a joint status") = false;
                "incoherent"
            };
            assessed_constraint_draw_pairs += 1;
            absolute_raw_residuals.push(raw_residual.abs());
            absolute_reconciled_residuals.push(reconciled_residual.abs());
            draw_diagnostics.push(json!({
                "draw_id": draw_id,
                "raw_parent_count": raw_parent,
                "raw_child_sum": raw_child_sum,
                "raw_signed_residual": raw_residual,
                "raw_status": raw_status,
                "reconciled_parent_count": reconciled_parent,
                "reconciled_child_sum": reconciled_child_sum,
                "reconciled_signed_residual": reconciled_residual,
                "reconciled_status": reconciled_status
            }));
        }

        constraint_diagnostics.push(json!({
            "constraint_id": id,
            "kind": kind,
            "completeness": completeness,
            "status": if constraint_reconciled_coherent == draw_ids.len() { "coherent_for_all_draws" } else { "incoherent_draws_detected" },
            "assessed_draw_count": draw_ids.len(),
            "raw_coherent_draw_count": constraint_raw_coherent,
            "reconciled_coherent_draw_count": constraint_reconciled_coherent,
            "raw_coherence_rate": constraint_raw_coherent as f64 / draw_ids.len() as f64,
            "reconciled_coherence_rate": constraint_reconciled_coherent as f64 / draw_ids.len() as f64,
            "draw_diagnostics": draw_diagnostics,
            "missing_child_forecast_ids": missing_child_forecast_ids,
            "expected_child_keys": expected_child_keys,
            "missing_expected_child_keys": missing_expected_child_keys,
            "unexpected_child_keys": unexpected_child_keys,
            "absolute_tolerance": tolerance
        }));
    }

    let draw_coherence = draw_ids
        .iter()
        .filter_map(Value::as_str)
        .map(|draw_id| {
            json!({
                "draw_id": draw_id,
                "raw_all_complete_constraints_coherent": raw_all_by_draw[draw_id],
                "reconciled_all_complete_constraints_coherent": reconciled_all_by_draw[draw_id]
            })
        })
        .collect::<Vec<_>>();
    let raw_all_draw_coherent_count = raw_all_by_draw
        .values()
        .filter(|coherent| **coherent)
        .count();
    let reconciled_all_draw_coherent_count = reconciled_all_by_draw
        .values()
        .filter(|coherent| **coherent)
        .count();
    let raw_rate = if assessed_constraint_draw_pairs == 0 {
        Value::Null
    } else {
        json!(raw_coherent_constraint_draw_pairs as f64 / assessed_constraint_draw_pairs as f64)
    };
    let reconciled_rate = if assessed_constraint_draw_pairs == 0 {
        Value::Null
    } else {
        json!(
            reconciled_coherent_constraint_draw_pairs as f64
                / assessed_constraint_draw_pairs as f64
        )
    };
    let mean_absolute_raw_residual = if absolute_raw_residuals.is_empty() {
        Value::Null
    } else {
        json!(absolute_raw_residuals.iter().sum::<f64>() / absolute_raw_residuals.len() as f64)
    };
    let mean_absolute_reconciled_residual = if absolute_reconciled_residuals.is_empty() {
        Value::Null
    } else {
        json!(
            absolute_reconciled_residuals.iter().sum::<f64>()
                / absolute_reconciled_residuals.len() as f64
        )
    };
    let total_absolute_adjustment = rows
        .iter()
        .map(|row| {
            (number_at(row, "reconciled_count").unwrap_or_default()
                - number_at(row, "raw_count").unwrap_or_default())
            .abs()
        })
        .sum::<f64>();
    let maximum_absolute_adjustment = rows
        .iter()
        .map(|row| {
            (number_at(row, "reconciled_count").unwrap_or_default()
                - number_at(row, "raw_count").unwrap_or_default())
            .abs()
        })
        .fold(0.0_f64, f64::max);

    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let input_sha256 = sha256_hex(input);
    let sample_rows_sha256 = sha256_hex(
        &serde_json::to_vec(&registration["sample_rows"])
            .expect("validated sample rows serialize deterministically"),
    );
    let registration_id =
        string_at(&registration, "registration_id").expect("schema validated registration_id");
    let source_commit_sha256 = sha256_hex(
        format!(
            "pirb-joint-sample-coherence-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}\n{sample_rows_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "joint_predictive_sample_coherence_audit",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "provider_id": registration["provider"]["provider_id"],
        "forecast_release_id": registration["provider"]["forecast_release_id"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "sample_rows_sha256": sample_rows_sha256,
        "forecast_artifact_sha256": registration["provider"]["forecast_artifact_sha256"],
        "hierarchy_sha256": registration["provider"]["hierarchy_sha256"],
        "reconciliation_method_sha256": registration["sample_policy"]["method_sha256"],
        "draw_count": draw_ids.len(),
        "forecast_node_count": forecasts.len(),
        "sample_row_count": rows.len(),
        "constraint_diagnostics": constraint_diagnostics,
        "draw_coherence": draw_coherence,
        "summary": {
            "constraint_count": constraints.len(),
            "assessable_constraint_count": constraint_diagnostics.iter().filter(|d| d["completeness"] == "complete").count(),
            "not_assessable_constraint_count": constraint_diagnostics.iter().filter(|d| d["status"] == "not_assessable").count(),
            "assessed_constraint_draw_pair_count": assessed_constraint_draw_pairs,
            "raw_coherent_constraint_draw_pair_count": raw_coherent_constraint_draw_pairs,
            "reconciled_coherent_constraint_draw_pair_count": reconciled_coherent_constraint_draw_pairs,
            "raw_constraint_draw_coherence_rate": raw_rate,
            "reconciled_constraint_draw_coherence_rate": reconciled_rate,
            "draws_coherent_across_all_complete_constraints_raw_count": raw_all_draw_coherent_count,
            "draws_coherent_across_all_complete_constraints_reconciled_count": reconciled_all_draw_coherent_count,
            "raw_joint_draw_coherence_rate": raw_all_draw_coherent_count as f64 / draw_ids.len() as f64,
            "reconciled_joint_draw_coherence_rate": reconciled_all_draw_coherent_count as f64 / draw_ids.len() as f64,
            "mean_absolute_raw_constraint_residual": mean_absolute_raw_residual,
            "mean_absolute_reconciled_constraint_residual": mean_absolute_reconciled_residual,
            "total_absolute_sample_adjustment": total_absolute_adjustment,
            "maximum_absolute_sample_adjustment": maximum_absolute_adjustment
        },
        "claims": {
            "sample_matrix_validated": true,
            "joint_sample_coherence_measured": true,
            "marginal_calibration_established": false,
            "joint_calibration_established": false,
            "forecast_accuracy_established": false,
            "decision_utility_established": false,
            "causal_impact_established": false,
            "live_decision_authority": false
        },
        "receipt_commitment_sha256": source_commit_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-joint-sample-coherence-audit --input REGISTRATION.json [--source-commit SHA]"
            .to_owned()
    })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read joint-sample registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let receipt = validate_and_audit(&input, &source_commit).map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize joint-sample receipt: {error}"))?
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
        serde_json::from_slice(FIXTURE_BYTES).expect("synthetic joint-sample fixture is valid JSON")
    }

    fn audit(value: &Value) -> Result<Value, Vec<String>> {
        validate_and_audit(
            &serde_json::to_vec(value).expect("fixture serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    #[test]
    fn fixture_detects_base_sample_errors_and_reconciled_draw_coherence() {
        let receipt = validate_and_audit(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("synthetic sample registration is valid");
        assert_eq!(receipt["summary"]["constraint_count"], 3);
        assert_eq!(receipt["summary"]["assessable_constraint_count"], 2);
        assert_eq!(receipt["summary"]["not_assessable_constraint_count"], 1);
        assert_eq!(receipt["summary"]["assessed_constraint_draw_pair_count"], 8);
        assert_eq!(
            receipt["summary"]["raw_coherent_constraint_draw_pair_count"],
            6
        );
        assert_eq!(
            receipt["summary"]["reconciled_coherent_constraint_draw_pair_count"],
            8
        );
        assert_eq!(
            receipt["summary"]["raw_constraint_draw_coherence_rate"],
            0.75
        );
        assert_eq!(
            receipt["summary"]["reconciled_constraint_draw_coherence_rate"],
            1.0
        );
        assert_eq!(
            receipt["summary"]["draws_coherent_across_all_complete_constraints_raw_count"],
            2
        );
        assert_eq!(
            receipt["summary"]["draws_coherent_across_all_complete_constraints_reconciled_count"],
            4
        );
        assert_eq!(receipt["summary"]["raw_joint_draw_coherence_rate"], 0.5);
        assert_eq!(receipt["summary"]["reconciled_joint_draw_coherence_rate"], 1.0);
        assert_eq!(receipt["summary"]["mean_absolute_raw_constraint_residual"], 0.25);
        assert_eq!(receipt["summary"]["mean_absolute_reconciled_constraint_residual"], 0.0);
        assert_eq!(receipt["summary"]["total_absolute_sample_adjustment"], 10.0);
        assert_eq!(receipt["summary"]["maximum_absolute_sample_adjustment"], 1.0);
        assert_eq!(receipt["claims"]["marginal_calibration_established"], false);
        assert_eq!(receipt["constraint_diagnostics"][2]["status"], "not_assessable");
        assert_eq!(receipt["constraint_diagnostics"][2]["missing_child_forecast_ids"][0], "fc-admin1-d-organized");
        assert_eq!(receipt["constraint_diagnostics"][2]["missing_expected_child_keys"][0], "unit-admin1-d");
    }

    #[test]
    fn coherent_reconciled_samples_do_not_hide_incoherent_base_samples() {
        let receipt = validate_and_audit(
            FIXTURE_BYTES,
            "0123456789abcdef0123456789abcdef01234567",
        )
        .expect("synthetic sample registration is valid");
        let draw_two = &receipt["constraint_diagnostics"][0]["draw_diagnostics"][1];
        assert_eq!(draw_two["raw_signed_residual"], 1.0);
        assert_eq!(draw_two["raw_status"], "incoherent");
        assert_eq!(draw_two["reconciled_signed_residual"], 0.0);
        assert_eq!(draw_two["reconciled_status"], "coherent");
    }

    #[test]
    fn incoherent_reconciled_draw_is_reported_not_dropped() {
        let mut value = fixture();
        let row = value["sample_rows"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| {
                row["draw_id"] == "draw-002" && row["forecast_id"] == "fc-admin1-b-organized"
            })
            .unwrap();
        row["reconciled_count"] = json!(12);
        let receipt = audit(&value).expect("incoherence is a reportable audit result");
        assert_eq!(
            receipt["constraint_diagnostics"][0]["draw_diagnostics"][1]["reconciled_signed_residual"],
            -1.0
        );
        assert_eq!(
            receipt["constraint_diagnostics"][0]["draw_diagnostics"][1]["reconciled_status"],
            "incoherent"
        );
        assert_eq!(
            receipt["summary"]["reconciled_coherent_constraint_draw_pair_count"],
            7
        );
        assert_eq!(
            receipt["summary"]["reconciled_joint_draw_coherence_rate"],
            0.75
        );
    }

    #[test]
    fn incomplete_constraint_is_not_scored_or_imputed() {
        let receipt = validate_and_audit(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
            .expect("incomplete constraint is reportable");
        let diagnostic = &receipt["constraint_diagnostics"][2];
        assert_eq!(diagnostic["status"], "not_assessable");
        assert_eq!(diagnostic["draw_diagnostics"], Value::Null);
        assert_eq!(diagnostic["assessed_draw_count"], 0);
    }

    #[test]
    fn complete_constraint_cannot_omit_an_existing_expected_child() {
        let mut value = fixture();
        value["aggregation_constraints"][0]["child_forecast_ids"] =
            json!(["fc-admin1-a-organized"]);
        let errors = audit(&value).expect_err("omitted expected child must fail closed");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("expected_child_keys"))
        );
    }

    #[test]
    fn complete_constraint_cannot_reference_missing_child_node() {
        let mut value = fixture();
        value["aggregation_constraints"][2]["completeness"] = json!("complete");
        let errors = audit(&value).expect_err("missing child forecast must fail closed");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("claims complete coverage"))
        );
    }

    #[test]
    fn every_declared_node_must_have_every_declared_draw() {
        let mut value = fixture();
        let rows = value["sample_rows"].as_array_mut().unwrap();
        rows.retain(|row| {
            !(row["draw_id"] == "draw-004" && row["forecast_id"] == "fc-admin1-b-organized")
        });
        let errors = audit(&value).expect_err("missing cell cannot be filled with zero");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("missing sample row"))
        );
    }

    #[test]
    fn duplicate_sample_pairs_are_rejected() {
        let mut value = fixture();
        let row = value["sample_rows"][0].clone();
        value["sample_rows"].as_array_mut().unwrap().push(row);
        let errors = audit(&value).expect_err("duplicate draw/node pair cannot overwrite");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate sample row"))
        );
    }

    #[test]
    fn unknown_draw_ids_are_rejected() {
        let mut value = fixture();
        value["sample_rows"][0]["draw_id"] = json!("draw-unknown");
        let errors = audit(&value).expect_err("samples must belong to the registered draw set");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("undeclared draw_id"))
        );
    }

    #[test]
    fn aggregation_constraint_cycles_are_rejected_iteratively() {
        let mut value = fixture();
        let mut cycle = value["aggregation_constraints"][0].clone();
        cycle["constraint_id"] = json!("constraint-cycle-test");
        cycle["parent_forecast_id"] = json!("fc-admin1-a-organized");
        cycle["child_forecast_ids"] = json!(["fc-country-a-organized"]);
        cycle["expected_child_keys"] = json!(["unit-country-a"]);
        value["aggregation_constraints"]
            .as_array_mut()
            .unwrap()
            .push(cycle);
        let errors = audit(&value).expect_err("hierarchy must be acyclic");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("aggregation constraint graph contains a cycle"))
        );
    }

    #[test]
    fn future_fit_cutoff_is_rejected() {
        let mut value = fixture();
        value["sample_policy"]["parameter_fit_cutoff_index"] = json!(10);
        let errors = audit(&value).expect_err("future fit data leaks evaluation information");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("fit cutoff must be strictly before forecast origin"))
        );
    }
}
