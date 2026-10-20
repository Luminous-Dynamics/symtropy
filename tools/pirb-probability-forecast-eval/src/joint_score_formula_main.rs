// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Computes empirical energy and variogram scores for registered joint samples.
// The target vector is explicitly restricted to non-overlapping scored leaves.
// Computed synthetic scores are formula oracles, not evidence of model superiority.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-joint-score-evaluation-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-joint-score-two-leaf-four-draw-golden-v0.1.json");

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

fn is_placeholder_digest(digest: &str) -> bool {
    digest
        .chars()
        .next()
        .is_some_and(|first| digest.chars().all(|ch| ch == first))
}

fn distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn empirical_energy_score(draws: &[Vec<f64>], observed: &[f64]) -> f64 {
    let draw_count = draws.len();
    let mean_distance_to_observation = draws
        .iter()
        .map(|draw| distance(draw, observed))
        .sum::<f64>()
        / draw_count as f64;

    // Since the ordered double sum is symmetric, this is exactly
    // (1 / (2 M^2)) * sum_i sum_j ||X_i - X_j||.
    let mut unordered_pair_distance_sum = 0.0;
    for left in 0..draw_count {
        for right in (left + 1)..draw_count {
            unordered_pair_distance_sum += distance(&draws[left], &draws[right]);
        }
    }
    mean_distance_to_observation
        - unordered_pair_distance_sum / (draw_count * draw_count) as f64
}

fn empirical_variogram_score(
    draws: &[Vec<f64>],
    observed: &[f64],
    node_ids: &[String],
    pair_weights: &[(&str, &str, f64)],
    exponent: f64,
) -> f64 {
    let index_by_node = node_ids
        .iter()
        .enumerate()
        .map(|(index, node_id)| (node_id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    pair_weights
        .iter()
        .map(|(left_id, right_id, weight)| {
            let left = index_by_node[left_id];
            let right = index_by_node[right_id];
            let observed_difference = (observed[left] - observed[right]).abs().powf(exponent);
            let predicted_difference = draws
                .iter()
                .map(|draw| (draw[left] - draw[right]).abs().powf(exponent))
                .sum::<f64>()
                / draws.len() as f64;
            weight * (observed_difference - predicted_difference).powi(2)
        })
        .sum()
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
    if cutoff >= origin {
        errors.push("sample parameter fit cutoff must be strictly before forecast origin".to_owned());
    }
    if target <= origin {
        errors.push("target period must be later than forecast origin".to_owned());
    }
    let declared_draw_ids = policy["draw_ids"]
        .as_array()
        .expect("schema validation requires draw_ids");
    let draw_ids = declared_draw_ids
        .iter()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    if policy["expected_draw_count"].as_u64() != Some(draw_ids.len() as u64) {
        errors.push("expected_draw_count does not match unique declared draw_ids".to_owned());
    }

    let nodes = registration["forecast_nodes"]
        .as_array()
        .expect("schema validation requires forecast_nodes");
    let rows = registration["sample_rows"]
        .as_array()
        .expect("schema validation requires sample_rows");
    let mut nodes_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    for node in nodes {
        let id = string_at(node, "forecast_id").unwrap_or_default();
        if nodes_by_id.insert(id.to_owned(), node).is_some() {
            errors.push(format!("forecast node ID is duplicated: {id}"));
        }
        if node["period_index"].as_i64() != Some(target) {
            errors.push(format!(
                "forecast node {id} period_index must equal target_period_index"
            ));
        }
    }

    let score_node_ids = registration["score_vector"]["node_ids"]
        .as_array()
        .expect("schema validation requires score_vector.node_ids")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let scored_leaf_ids = nodes
        .iter()
        .filter(|node| node["scoring_role"] == "scored_leaf")
        .filter_map(|node| string_at(node, "forecast_id"))
        .collect::<BTreeSet<_>>();
    let score_node_set = score_node_ids.iter().copied().collect::<BTreeSet<_>>();
    if scored_leaf_ids != score_node_set {
        errors.push(
            "score vector must select exactly the registered scored_leaf nodes; aggregate nodes and omitted leaf dimensions are forbidden"
                .to_owned(),
        );
    }
    for node_id in &score_node_ids {
        if !nodes_by_id.contains_key(*node_id) {
            errors.push(format!("score vector references unknown forecast node {node_id}"));
        } else if nodes_by_id[*node_id]["scoring_role"] != "scored_leaf" {
            errors.push(format!(
                "score vector includes non-leaf/aggregate node {node_id}; this would double-count hierarchy totals"
            ));
        }
    }

    let observed_rows = registration["score_vector"]["observed_values"]
        .as_array()
        .expect("schema validation requires observed_values");
    let mut observed_ids = BTreeSet::new();
    for row in observed_rows {
        let id = string_at(row, "forecast_id").unwrap_or_default();
        if !observed_ids.insert(id.to_owned()) {
            errors.push(format!("observed outcome ID is duplicated: {id}"));
        }
        if !score_node_set.contains(id) {
            errors.push(format!("observed outcome vector contains unscored/unknown node {id}"));
        }
    }
    if observed_ids != score_node_set.iter().map(|id| (*id).to_owned()).collect() {
        errors.push("observed outcome vector must contain exactly one value for every scored leaf".to_owned());
    }

    let mut rows_by_key: BTreeMap<(String, String), &Value> = BTreeMap::new();
    for row in rows {
        let draw_id = string_at(row, "draw_id").unwrap_or_default();
        let forecast_id = string_at(row, "forecast_id").unwrap_or_default();
        if !draw_ids.contains(draw_id) {
            errors.push(format!("sample row references undeclared draw_id {draw_id}"));
        }
        if !nodes_by_id.contains_key(forecast_id) {
            errors.push(format!("sample row references unknown forecast_id {forecast_id}"));
        }
        let key = (draw_id.to_owned(), forecast_id.to_owned());
        if rows_by_key.insert(key, row).is_some() {
            errors.push(format!(
                "duplicate sample row for draw_id {draw_id}, forecast_id {forecast_id}"
            ));
        }
    }
    for draw_id in &draw_ids {
        for forecast_id in nodes_by_id.keys() {
            if !rows_by_key.contains_key(&((*draw_id).to_owned(), forecast_id.clone())) {
                errors.push(format!(
                    "missing sample row for draw_id {draw_id}, forecast_id {forecast_id}; missing values are not imputed"
                ));
            }
        }
    }

    let constraints = registration["aggregation_constraints"]
        .as_array()
        .expect("schema validation requires aggregation_constraints");
    let mut constraint_ids = BTreeSet::new();
    for constraint in constraints {
        let id = string_at(constraint, "constraint_id").unwrap_or_default();
        if !constraint_ids.insert(id.to_owned()) {
            errors.push(format!("constraint ID is duplicated: {id}"));
        }
        let parent_id = string_at(constraint, "parent_forecast_id").unwrap_or_default();
        if nodes_by_id
            .get(parent_id)
            .is_none_or(|node| node["scoring_role"] != "aggregate")
        {
            errors.push(format!(
                "constraint {id} parent must reference a declared aggregate node"
            ));
        }
        let child_ids = constraint["child_forecast_ids"]
            .as_array()
            .expect("schema validation requires child_forecast_ids");
        let completeness = string_at(constraint, "completeness").unwrap_or_default();
        let expected_child_keys = constraint["expected_child_keys"]
            .as_array()
            .expect("schema validation requires expected_child_keys")
            .iter()
            .filter_map(Value::as_str)
            .collect::<BTreeSet<_>>();
        let actual_child_keys = child_ids
            .iter()
            .filter_map(Value::as_str)
            .filter_map(|child_id| nodes_by_id.get(child_id).copied())
            .filter_map(|node| string_at(node, "unit_id"))
            .collect::<BTreeSet<_>>();
        if completeness == "complete" && actual_child_keys != expected_child_keys {
            errors.push(format!(
                "constraint {id} complete child set differs from expected_child_keys"
            ));
        }
        for child_id in child_ids.iter().filter_map(Value::as_str) {
            if !nodes_by_id.contains_key(child_id) {
                if completeness == "complete" {
                    errors.push(format!(
                        "constraint {id} claims complete coverage but child node {child_id} is missing"
                    ));
                }
                continue;
            }
            let child = nodes_by_id[child_id];
            let parent = nodes_by_id
                .get(parent_id)
                .expect("parent node was checked above");
            if child["scoring_role"] != "scored_leaf" {
                errors.push(format!(
                    "constraint {id} child {child_id} must be a scored leaf, not an aggregate"
                ));
            }
            if string_at(child, "event_type_id") != string_at(parent, "event_type_id") {
                errors.push(format!(
                    "spatial constraint {id} mixes event_type_id values"
                ));
            }
            if child["period_index"] != parent["period_index"] {
                errors.push(format!(
                    "constraint {id} mixes parent/child forecast periods"
                ));
            }
        }
    }

    let weights = registration["variogram_score_policy"]["pair_weights"]
        .as_array()
        .expect("schema validation requires pair_weights");
    let mut pair_set = BTreeSet::new();
    for pair in weights {
        let left = string_at(pair, "left_node_id").unwrap_or_default();
        let right = string_at(pair, "right_node_id").unwrap_or_default();
        if left == right {
            errors.push("variogram pair cannot use the same node twice".to_owned());
            continue;
        }
        if !score_node_set.contains(left) || !score_node_set.contains(right) {
            errors.push("variogram pair must reference scored-leaf nodes".to_owned());
            continue;
        }
        let key = if left < right {
            (left.to_owned(), right.to_owned())
        } else {
            (right.to_owned(), left.to_owned())
        };
        if !pair_set.insert(key) {
            errors.push(format!("variogram pair {left}/{right} is duplicated or reversed"));
        }
    }
    let expected_pair_count = score_node_ids.len() * (score_node_ids.len() - 1) / 2;
    if pair_set.len() != expected_pair_count {
        errors.push("variogram weights must cover each unordered scored-node pair exactly once".to_owned());
    }

    if registration["evidence_scope"] == "authorized_historical" {
        let policy = &registration["outcome_policy"];
        if policy["common_rows_verified"].as_bool() != Some(true) {
            errors.push("historical scoring requires verified common outcome rows".to_owned());
        }
        if policy["heldout_outcomes"].as_bool() != Some(true) {
            errors.push("historical scoring requires held-out outcomes".to_owned());
        }
        if string_at(policy, "outcome_maturity_status") != Some("mature_frozen_vintage") {
            errors.push("historical scoring requires mature frozen outcomes".to_owned());
        }
        if is_placeholder_digest(
            string_at(&registration["score_vector"], "outcome_release_sha256").unwrap_or_default(),
        ) {
            errors.push("historical scoring requires a non-placeholder outcome release digest".to_owned());
        }
        if string_at(&registration["governance"], "data_authority_status") != Some("authorized") {
            errors.push("historical scoring requires authorized data use".to_owned());
        }
        if string_at(&registration["governance"], "disclosure_review_status") != Some("approved") {
            errors.push("historical scoring requires approved disclosure review".to_owned());
        }
        for (object, digest_key) in [
            ("energy_score_policy", "formula_sha256"),
            ("energy_score_policy", "implementation_sha256"),
            ("variogram_score_policy", "formula_sha256"),
            ("variogram_score_policy", "implementation_sha256"),
        ] {
            if string_at(&registration[object], digest_key).is_none_or(is_placeholder_digest) {
                errors.push(format!(
                    "historical scoring requires non-placeholder {object}.{digest_key}"
                ));
            }
        }
    }

    errors.sort();
    errors
}

fn compute_scores(registration: &Value) -> Result<Value, Vec<String>> {
    let draw_ids = registration["sample_policy"]["draw_ids"]
        .as_array()
        .expect("schema validated draw_ids")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let sample_rows = registration["sample_rows"]
        .as_array()
        .expect("schema validated sample_rows");
    let rows_by_key = sample_rows
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
    let score_node_ids = registration["score_vector"]["node_ids"]
        .as_array()
        .expect("schema validated score vector")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let observed_map = registration["score_vector"]["observed_values"]
        .as_array()
        .expect("schema validated observed_values")
        .iter()
        .map(|row| {
            (
                string_at(row, "forecast_id").expect("schema validated observation ID"),
                number_at(row, "observed_count").expect("schema validated observed_count"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let observed = score_node_ids
        .iter()
        .map(|node_id| {
            observed_map
                .get(node_id.as_str())
                .copied()
                .expect("semantic validation enforces complete observed vector")
        })
        .collect::<Vec<_>>();

    let mut raw_draws = Vec::new();
    let mut reconciled_draws = Vec::new();
    for draw_id in &draw_ids {
        let mut raw = Vec::new();
        let mut reconciled = Vec::new();
        for node_id in &score_node_ids {
            let row = rows_by_key
                .get(&((*draw_id).to_owned(), node_id.clone()))
                .expect("semantic validation enforces complete sample matrix");
            raw.push(number_at(row, "raw_count").expect("schema validated raw_count"));
            reconciled.push(
                number_at(row, "reconciled_count").expect("schema validated reconciled_count"),
            );
        }
        raw_draws.push(raw);
        reconciled_draws.push(reconciled);
    }

    let energy_policy = &registration["energy_score_policy"];
    let variogram_policy = &registration["variogram_score_policy"];
    let pair_weights = variogram_policy["pair_weights"]
        .as_array()
        .expect("schema validated pair_weights")
        .iter()
        .map(|pair| {
            (
                string_at(pair, "left_node_id").expect("schema validated left_node_id"),
                string_at(pair, "right_node_id").expect("schema validated right_node_id"),
                number_at(pair, "weight").expect("schema validated weight"),
            )
        })
        .collect::<Vec<_>>();
    let exponent = number_at(variogram_policy, "exponent").expect("schema validated exponent");

    let raw_energy = empirical_energy_score(&raw_draws, &observed);
    let reconciled_energy = empirical_energy_score(&reconciled_draws, &observed);
    let raw_variogram =
        empirical_variogram_score(&raw_draws, &observed, &score_node_ids, &pair_weights, exponent);
    let reconciled_variogram = empirical_variogram_score(
        &reconciled_draws,
        &observed,
        &score_node_ids,
        &pair_weights,
        exponent,
    );

    let nodes_by_id = registration["forecast_nodes"]
        .as_array()
        .expect("schema validated forecast_nodes")
        .iter()
        .map(|node| {
            (
                string_at(node, "forecast_id").expect("schema validated forecast_id"),
                node,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut constraint_diagnostics = Vec::new();
    for constraint in registration["aggregation_constraints"]
        .as_array()
        .expect("schema validated aggregation_constraints")
    {
        let id = string_at(constraint, "constraint_id").expect("schema validated constraint ID");
        let completeness =
            string_at(constraint, "completeness").expect("schema validated completeness");
        let tolerance =
            number_at(constraint, "absolute_tolerance").expect("schema validated tolerance");
        if completeness != "complete" {
            constraint_diagnostics.push(json!({
                "constraint_id": id,
                "completeness": completeness,
                "status": "not_assessable",
                "assessed_draw_count": 0,
                "raw_coherent_draw_count": 0,
                "reconciled_coherent_draw_count": 0
            }));
            continue;
        }
        let parent_id = string_at(constraint, "parent_forecast_id").unwrap();
        let parent = nodes_by_id[parent_id];
        let child_ids = constraint["child_forecast_ids"]
            .as_array()
            .expect("schema validated child IDs")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        let mut raw_coherent = 0usize;
        let mut reconciled_coherent = 0usize;
        let mut per_draw = Vec::new();
        for draw_id in &draw_ids {
            let parent_row = rows_by_key
                .get(&((*draw_id).to_owned(), parent_id.to_owned()))
                .expect("sample matrix complete");
            let raw_parent = number_at(parent_row, "raw_count").unwrap();
            let reconciled_parent = number_at(parent_row, "reconciled_count").unwrap();
            let mut raw_sum = 0.0;
            let mut reconciled_sum = 0.0;
            for child_id in &child_ids {
                let child_row = rows_by_key
                    .get(&((*draw_id).to_owned(), (*child_id).to_owned()))
                    .expect("sample matrix complete");
                raw_sum += number_at(child_row, "raw_count").unwrap();
                reconciled_sum += number_at(child_row, "reconciled_count").unwrap();
            }
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
                "incoherent"
            };
            per_draw.push(json!({
                "draw_id": draw_id,
                "raw_parent_count": raw_parent,
                "raw_child_sum": raw_sum,
                "raw_signed_residual": raw_residual,
                "raw_status": raw_status,
                "reconciled_parent_count": reconciled_parent,
                "reconciled_child_sum": reconciled_sum,
                "reconciled_signed_residual": reconciled_residual,
                "reconciled_status": reconciled_status
            }));
        }
        constraint_diagnostics.push(json!({
            "constraint_id": id,
            "completeness": completeness,
            "status": if reconciled_coherent == draw_ids.len() { "reconciled_coherent_for_all_draws" } else { "reconciled_incoherence_detected" },
            "assessed_draw_count": draw_ids.len(),
            "raw_coherent_draw_count": raw_coherent,
            "reconciled_coherent_draw_count": reconciled_coherent,
            "raw_coherence_rate": raw_coherent as f64 / draw_ids.len() as f64,
            "reconciled_coherence_rate": reconciled_coherent as f64 / draw_ids.len() as f64,
            "draw_diagnostics": per_draw
        }));
        let _ = parent;
    }

    Ok(json!({
        "scores": {
            "energy_score": {
                "orientation": "lower_is_better",
                "finite_ensemble_convention": energy_policy["finite_ensemble_convention"],
                "base_raw": raw_energy,
                "reconciled": reconciled_energy,
                "base_minus_reconciled": raw_energy - reconciled_energy
            },
            "variogram_score": {
                "orientation": "lower_is_better",
                "exponent": exponent,
                "pair_weights": pair_weights,
                "base_raw": raw_variogram,
                "reconciled": reconciled_variogram,
                "base_minus_reconciled": raw_variogram - reconciled_variogram
            }
        },
        "sample_hierarchy_diagnostics": constraint_diagnostics
    }))
}

fn validate_and_score(input: &[u8], source_commit: &str) -> Result<Value, Vec<String>> {
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
        .map_err(|error| vec![format!("joint-score registration JSON is invalid: {error}")])?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES).map_err(|error| {
        vec![format!(
            "embedded joint-score schema JSON is invalid: {error}"
        )]
    })?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )]
    })?;
    let validator = jsonschema::draft202012::new(&schema).map_err(|error| {
        vec![format!("cannot build joint-score schema validator: {error}")]
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

    let score_results = compute_scores(&registration)
        .map_err(|errors| errors)?;
    let registration_id =
        string_at(&registration, "registration_id").expect("schema validated registration_id");
    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let input_sha256 = sha256_hex(input);
    let score_commitment_sha256 = sha256_hex(
        format!(
            "pirb-joint-score-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "joint_sample_score_formula_oracle",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "forecast_artifact_sha256": registration["provider"]["forecast_artifact_sha256"],
        "hierarchy_sha256": registration["provider"]["hierarchy_sha256"],
        "outcome_release_sha256": registration["score_vector"]["outcome_release_sha256"],
        "energy_formula_sha256": registration["energy_score_policy"]["formula_sha256"],
        "energy_implementation_sha256": registration["energy_score_policy"]["implementation_sha256"],
        "variogram_formula_sha256": registration["variogram_score_policy"]["formula_sha256"],
        "variogram_implementation_sha256": registration["variogram_score_policy"]["implementation_sha256"],
        "scored_node_ids": registration["score_vector"]["node_ids"],
        "observed_values": registration["score_vector"]["observed_values"],
        "draw_count": registration["sample_policy"]["expected_draw_count"],
        "forecast_node_count": registration["forecast_nodes"].as_array().map_or(0, Vec::len),
        "sample_row_count": registration["sample_rows"].as_array().map_or(0, Vec::len),
        "score_results": score_results["scores"],
        "sample_hierarchy_diagnostics": score_results["sample_hierarchy_diagnostics"],
        "claims": {
            "scores_computed": true,
            "formula_oracle_only": registration["evidence_scope"] == "synthetic",
            "forecast_superiority_established": false,
            "ranking_permitted": false,
            "marginal_calibration_established": false,
            "joint_calibration_established": false,
            "decision_value_established": false,
            "causal_impact_established": false,
            "live_decision_authority": false
        },
        "claim_ceiling": "registered_formula_computation_only",
        "receipt_commitment_sha256": score_commitment_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-joint-score-formula-oracle --input REGISTRATION.json [--source-commit SHA]"
            .to_owned()
    })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read joint-score registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let receipt =
        validate_and_score(&input, &source_commit).map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize joint-score receipt: {error}"))?
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
        serde_json::from_slice(FIXTURE_BYTES).expect("joint-score golden fixture is valid JSON")
    }

    fn score(value: &Value) -> Result<Value, Vec<String>> {
        validate_and_score(
            &serde_json::to_vec(value).expect("registration serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.15}, expected {expected:.15}, tolerance {tolerance}"
        );
    }

    #[test]
    fn independent_energy_and_variogram_golden_values_match() {
        let receipt = validate_and_score(
            FIXTURE_BYTES,
            "0123456789abcdef0123456789abcdef01234567",
        )
        .expect("golden fixture must pass registration and score validation");
        assert_close(
            receipt["score_results"]["energy_score"]["base_raw"]
                .as_f64()
                .unwrap(),
            0.5 + 2.0_f64.sqrt() / 4.0,
            1e-12,
        );
        assert_close(
            receipt["score_results"]["energy_score"]["reconciled"]
                .as_f64()
                .unwrap(),
            (2.0 + 2.0_f64.sqrt()) / 8.0,
            1e-12,
        );
        assert_close(
            receipt["score_results"]["variogram_score"]["base_raw"]
                .as_f64()
                .unwrap(),
            1.0,
            1e-12,
        );
        assert_close(
            receipt["score_results"]["variogram_score"]["reconciled"]
                .as_f64()
                .unwrap(),
            0.25,
            1e-12,
        );
        assert_eq!(receipt["claims"]["forecast_superiority_established"], false);
        assert_eq!(receipt["claims"]["ranking_permitted"], false);
        assert_eq!(receipt["claims"]["causal_impact_established"], false);
    }

    #[test]
    fn sample_row_order_does_not_change_computed_scores() {
        let baseline = score(&fixture()).expect("fixture scores");
        let mut reordered = fixture();
        reordered["sample_rows"].as_array_mut().unwrap().reverse();
        let permuted = score(&reordered).expect("permuted rows should produce same score");
        assert_eq!(
            baseline["score_results"]["energy_score"]["base_raw"],
            permuted["score_results"]["energy_score"]["base_raw"]
        );
        assert_eq!(
            baseline["score_results"]["energy_score"]["reconciled"],
            permuted["score_results"]["energy_score"]["reconciled"]
        );
        assert_eq!(
            baseline["score_results"]["variogram_score"]["base_raw"],
            permuted["score_results"]["variogram_score"]["base_raw"]
        );
    }

    #[test]
    fn aggregate_parent_and_children_cannot_all_enter_the_scored_vector() {
        let mut value = fixture();
        value["score_vector"]["node_ids"] =
            json!(["fc-country-total", "fc-admin1-a", "fc-admin1-b"]);
        value["score_vector"]["observed_values"].as_array_mut().unwrap().push(json!({
            "forecast_id":"fc-country-total",
            "observed_count":0
        }));
        let errors = score(&value).expect_err("aggregate plus children would double count");
        assert!(errors.iter().any(|error| error.contains("exactly the registered scored_leaf nodes")));
        assert!(errors.iter().any(|error| error.contains("double-count hierarchy totals")));
    }

    #[test]
    fn every_draw_must_have_every_node_sample() {
        let mut value = fixture();
        value["sample_rows"].as_array_mut().unwrap().retain(|row| {
            !(row["draw_id"] == "draw-004" && row["forecast_id"] == "fc-admin1-b")
        });
        let errors = score(&value).expect_err("missing sample cells must fail closed");
        assert!(errors.iter().any(|error| error.contains("missing sample row")));
    }

    #[test]
    fn variogram_pairs_must_cover_each_unordered_pair_exactly_once() {
        let mut value = fixture();
        value["variogram_score_policy"]["pair_weights"] = json!([
            {"left_node_id":"fc-admin1-a","right_node_id":"fc-admin1-b","weight":1.0},
            {"left_node_id":"fc-admin1-b","right_node_id":"fc-admin1-a","weight":1.0}
        ]);
        let errors = score(&value).expect_err("reversed duplicate pair is invalid");
        assert!(errors.iter().any(|error| error.contains("duplicated or reversed")));
        assert!(errors.iter().any(|error| error.contains("cover each unordered")));
    }

    #[test]
    fn score_vector_must_include_all_scored_leaves_and_no_aggregate_nodes() {
        let mut value = fixture();
        value["score_vector"]["node_ids"] = json!(["fc-admin1-a"]);
        value["score_vector"]["observed_values"] =
            json!([{"forecast_id":"fc-admin1-a","observed_count":0}]);
        let errors = score(&value).expect_err("partial vector is not the registered target");
        assert!(errors.iter().any(|error| error.contains("exactly the registered scored_leaf nodes")));
    }

    #[test]
    fn historical_scoring_requires_mature_authorized_heldout_outcomes() {
        let mut value = fixture();
        value["evidence_scope"] = json!("authorized_historical");
        let errors = score(&value).expect_err("synthetic data cannot stand in for mature history");
        assert!(errors.iter().any(|error| error.contains("mature frozen outcomes")));
        assert!(errors.iter().any(|error| error.contains("authorized data use")));
    }
}
