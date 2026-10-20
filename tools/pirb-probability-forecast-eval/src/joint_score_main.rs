// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Computes empirical energy and variogram scores for synthetic joint samples.
// The registered comparison is restricted to scores reviewed for this setting.
// Numerical oracles do not establish calibration, predictive superiority or impact.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    env, fs, process,
};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-joint-score-registration-v0.1.schema.json");
const FOUR_DRAW_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-joint-score-four-draw-synthetic-v0.1.json");
const THREE_DRAW_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-joint-score-three-draw-synthetic-v0.1.json");
const IMPLEMENTATION_BYTES: &[u8] = include_bytes!("joint_score_main.rs");

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

fn energy_score(samples: &[Vec<f64>], observed: &[f64]) -> f64 {
    let count = samples.len() as f64;
    let mean_distance_to_observation = samples
        .iter()
        .map(|sample| {
            sample
                .iter()
                .zip(observed)
                .map(|(value, target)| (value - target).powi(2))
                .sum::<f64>()
                .sqrt()
        })
        .sum::<f64>()
        / count;
    let ordered_pairwise_distance_sum = samples
        .iter()
        .flat_map(|left| {
            samples.iter().map(move |right| {
                left.iter()
                    .zip(right)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt()
            })
        })
        .sum::<f64>();
    mean_distance_to_observation - ordered_pairwise_distance_sum / (2.0 * count * count)
}

fn variogram_score(samples: &[Vec<f64>], observed: &[f64], exponent: f64) -> f64 {
    let mut result = 0.0;
    for left in 0..observed.len() {
        for right in (left + 1)..observed.len() {
            let observed_difference = (observed[left] - observed[right]).abs().powf(exponent);
            let forecast_difference = samples
                .iter()
                .map(|sample| (sample[left] - sample[right]).abs().powf(exponent))
                .sum::<f64>()
                / samples.len() as f64;
            result += (observed_difference - forecast_difference).powi(2);
        }
    }
    result
}

fn schema_errors(registration: &Value) -> Vec<String> {
    let schema: Value = match serde_json::from_slice(SCHEMA_BYTES) {
        Ok(value) => value,
        Err(error) => return vec![format!("embedded joint-score schema is invalid: {error}")],
    };
    if let Err(error) = jsonschema::draft202012::meta::validate(&schema) {
        return vec![format!("Draft 2020-12 schema meta-validation failed: {error}")];
    }
    let validator = match jsonschema::draft202012::new(&schema) {
        Ok(value) => value,
        Err(error) => return vec![format!("cannot build joint-score validator: {error}")],
    };
    let mut errors = validator
        .iter_errors(registration)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    errors.sort();
    errors
}

fn graph_has_cycle(
    nodes: &BTreeMap<String, &Value>,
    constraints: &[Value],
) -> bool {
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
                    .expect("graph nodes were registered");
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
    let origin = registration["forecast_origin_index"].as_i64().unwrap_or_default();
    let target = registration["target_period_index"].as_i64().unwrap_or_default();
    let cutoff = registration["sample_policy"]["parameter_fit_cutoff_index"]
        .as_i64()
        .unwrap_or_default();
    if target <= origin {
        errors.push("target period must be later than forecast origin".to_owned());
    }
    if cutoff >= origin {
        errors.push("sample fit cutoff must be strictly before forecast origin".to_owned());
    }

    let draw_ids = registration["sample_policy"]["draw_ids"]
        .as_array()
        .expect("schema requires draw_ids")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    let unique_draw_ids = draw_ids.iter().copied().collect::<BTreeSet<_>>();
    if unique_draw_ids.len() != draw_ids.len() {
        errors.push("declared draw IDs must be unique".to_owned());
    }
    if registration["sample_policy"]["expected_draw_count"].as_u64()
        != Some(unique_draw_ids.len() as u64)
    {
        errors.push("expected_draw_count does not match declared draw_ids".to_owned());
    }

    let node_values = registration["forecast_nodes"]
        .as_array()
        .expect("schema requires forecast_nodes");
    let constraints = registration["aggregation_constraints"]
        .as_array()
        .expect("schema requires aggregation_constraints");
    let sample_rows = registration["sample_rows"]
        .as_array()
        .expect("schema requires sample_rows");
    let scored_ids = registration["scored_node_ids"]
        .as_array()
        .expect("schema requires scored_node_ids")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let scored_set = scored_ids.iter().cloned().collect::<BTreeSet<_>>();

    let mut nodes: BTreeMap<String, &Value> = BTreeMap::new();
    for node in node_values {
        let id = string_at(node, "forecast_id").unwrap_or_default().to_owned();
        if nodes.insert(id.clone(), node).is_some() {
            errors.push(format!("forecast node ID is duplicated: {id}"));
        }
        if node["period_index"].as_i64() != Some(target) {
            errors.push(format!("forecast node {id} period_index must equal target_period_index"));
        }
    }
    for id in &scored_ids {
        if !nodes.contains_key(id) {
            errors.push(format!("scored vector references unknown forecast node {id}"));
        }
    }

    let observed = registration["observed_vector"]
        .as_array()
        .expect("schema requires observed_vector");
    let observed_ids = observed
        .iter()
        .filter_map(|row| string_at(row, "forecast_id"))
        .collect::<Vec<_>>();
    if observed_ids != scored_ids.iter().map(String::as_str).collect::<Vec<_>>() {
        errors.push("observed_vector IDs and order must exactly match scored_node_ids".to_owned());
    }
    let unique_observed = observed_ids.iter().copied().collect::<BTreeSet<_>>();
    if unique_observed.len() != observed_ids.len() {
        errors.push("observed_vector contains duplicate forecast IDs".to_owned());
    }

    let mut constraint_ids = BTreeSet::new();
    for constraint in constraints {
        let id = string_at(constraint, "constraint_id").unwrap_or_default();
        if !constraint_ids.insert(id.to_owned()) {
            errors.push(format!("constraint ID is duplicated: {id}"));
        }
        let parent_id = string_at(constraint, "parent_forecast_id").unwrap_or_default();
        let Some(parent) = nodes.get(parent_id).copied() else {
            errors.push(format!("constraint {id} references unknown parent {parent_id}"));
            continue;
        };
        let child_ids = constraint["child_forecast_ids"]
            .as_array()
            .expect("schema requires child_forecast_ids")
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        let mut actual_keys = BTreeSet::new();
        for child_id in &child_ids {
            if child_id == &parent_id {
                errors.push(format!("constraint {id} uses its parent as a child"));
                continue;
            }
            let Some(child) = nodes.get(*child_id).copied() else {
                errors.push(format!("constraint {id} references unknown child {child_id}"));
                continue;
            };
            match string_at(constraint, "kind").unwrap_or_default() {
                "spatial" => {
                    if string_at(child, "event_type_id") != string_at(parent, "event_type_id") {
                        errors.push(format!("spatial constraint {id} mixes event types"));
                    }
                    actual_keys.insert(string_at(child, "unit_id").unwrap_or_default().to_owned());
                }
                "event_type" => {
                    if string_at(child, "unit_id") != string_at(parent, "unit_id") {
                        errors.push(format!("event-type constraint {id} mixes units"));
                    }
                    actual_keys.insert(string_at(child, "event_type_id").unwrap_or_default().to_owned());
                }
                _ => errors.push(format!("constraint {id} has unsupported kind")),
            }
        }
        let expected_keys = constraint["expected_child_keys"]
            .as_array()
            .expect("schema requires expected_child_keys")
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect::<BTreeSet<_>>();
        if actual_keys != expected_keys {
            errors.push(format!("constraint {id} child rows do not match expected_child_keys"));
        }

        let parent_selected = scored_set.contains(parent_id);
        let selected_child = child_ids.iter().any(|child_id| scored_set.contains(*child_id));
        if parent_selected && selected_child {
            errors.push(format!(
                "scored vector double-counts aggregate parent {parent_id} alongside one or more children in constraint {id}"
            ));
        }
    }
    if graph_has_cycle(&nodes, constraints) {
        errors.push("aggregation constraint graph contains a cycle".to_owned());
    }

    let mut rows_by_key: BTreeMap<(String, String), &Value> = BTreeMap::new();
    for row in sample_rows {
        let draw_id = string_at(row, "draw_id").unwrap_or_default().to_owned();
        let forecast_id = string_at(row, "forecast_id").unwrap_or_default().to_owned();
        if !unique_draw_ids.contains(draw_id.as_str()) {
            errors.push(format!("sample row references undeclared draw {draw_id}"));
        }
        if !nodes.contains_key(&forecast_id) {
            errors.push(format!("sample row references undeclared forecast node {forecast_id}"));
        }
        if rows_by_key.insert((draw_id.clone(), forecast_id.clone()), row).is_some() {
            errors.push(format!("duplicate sample row for draw {draw_id}, node {forecast_id}"));
        }
    }
    let expected_row_count = unique_draw_ids.len().saturating_mul(nodes.len());
    if sample_rows.len() != expected_row_count {
        errors.push(format!(
            "sample row count {} does not match expected draw-by-node grid {expected_row_count}",
            sample_rows.len()
        ));
    }
    for draw_id in &unique_draw_ids {
        for forecast_id in nodes.keys() {
            if !rows_by_key.contains_key(&(draw_id.to_string(), forecast_id.clone())) {
                errors.push(format!(
                    "missing sample cell for draw {draw_id}, node {forecast_id}"
                ));
            }
        }
    }
    if errors.is_empty() {
        return errors;
    }
    errors.sort();
    errors
}

fn score_matrix(
    samples: &[Vec<f64>],
    observed: &[f64],
    exponent: f64,
) -> Value {
    json!({
        "energy_score": energy_score(samples, observed),
        "variogram_score": variogram_score(samples, observed, exponent)
    })
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
    let mut errors = schema_errors(&registration);
    if !errors.is_empty() {
        return Err(errors);
    }
    errors = semantic_errors(&registration);
    if !errors.is_empty() {
        return Err(errors);
    }

    let scored_ids = registration["scored_node_ids"]
        .as_array()
        .expect("validated scored_node_ids")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let draw_ids = registration["sample_policy"]["draw_ids"]
        .as_array()
        .expect("validated draw_ids")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let rows = registration["sample_rows"]
        .as_array()
        .expect("validated sample_rows");
    let rows_by_key = rows
        .iter()
        .map(|row| {
            (
                (
                    string_at(row, "draw_id").expect("schema validated draw_id").to_owned(),
                    string_at(row, "forecast_id")
                        .expect("schema validated forecast_id")
                        .to_owned(),
                ),
                row,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let observed = registration["observed_vector"]
        .as_array()
        .expect("validated observed_vector")
        .iter()
        .map(|row| number_at(row, "observed_count").expect("schema validated observed_count"))
        .collect::<Vec<_>>();
    let mut raw_samples = Vec::new();
    let mut reconciled_samples = Vec::new();
    for draw_id in &draw_ids {
        let mut raw_vector = Vec::new();
        let mut reconciled_vector = Vec::new();
        for forecast_id in &scored_ids {
            let row = rows_by_key
                .get(&(draw_id.clone(), forecast_id.clone()))
                .expect("semantic validation requires complete sample grid");
            raw_vector.push(number_at(row, "raw_count").expect("schema validated raw_count"));
            reconciled_vector.push(
                number_at(row, "reconciled_count").expect("schema validated reconciled_count"),
            );
        }
        raw_samples.push(raw_vector);
        reconciled_samples.push(reconciled_vector);
    }

    let exponent = number_at(&registration["score_policy"], "variogram_exponent")
        .expect("schema validated variogram_exponent");
    let raw_scores = score_matrix(&raw_samples, &observed, exponent);
    let reconciled_scores = score_matrix(&reconciled_samples, &observed, exponent);
    let score_delta = json!({
        "energy_score_reconciled_minus_base":
            reconciled_scores["energy_score"].as_f64().expect("energy score is numeric")
            - raw_scores["energy_score"].as_f64().expect("energy score is numeric"),
        "variogram_score_reconciled_minus_base":
            reconciled_scores["variogram_score"].as_f64().expect("variogram score is numeric")
            - raw_scores["variogram_score"].as_f64().expect("variogram score is numeric"),
        "lower_is_better_for_both_registered_scores": true,
        "superiority_established": false
    });

    let tolerance = number_at(&registration["score_policy"], "numeric_tolerance")
        .expect("schema validated numeric_tolerance");
    let constraints = registration["aggregation_constraints"]
        .as_array()
        .expect("validated aggregation_constraints");
    let node_by_id = registration["forecast_nodes"]
        .as_array()
        .expect("validated forecast_nodes")
        .iter()
        .map(|node| (
            string_at(node, "forecast_id").expect("validated forecast_id"),
            node,
        ))
        .collect::<BTreeMap<_, _>>();
    let mut coherence = Vec::new();
    let mut base_coherent_draw_count = 0usize;
    let mut reconciled_coherent_draw_count = 0usize;
    let mut raw_abs_residuals = Vec::new();
    let mut reconciled_abs_residuals = Vec::new();
    for draw_id in &draw_ids {
        let mut base_draw_coherent = true;
        let mut reconciled_draw_coherent = true;
        for constraint in constraints {
            let parent_id = string_at(constraint, "parent_forecast_id")
                .expect("validated parent_forecast_id");
            let parent_node = node_by_id.get(parent_id).expect("validated node");
            let parent_row = rows_by_key
                .get(&(draw_id.clone(), parent_id.to_owned()))
                .expect("complete sample grid");
            let child_ids = constraint["child_forecast_ids"]
                .as_array()
                .expect("validated child_forecast_ids");
            let child_rows = child_ids
                .iter()
                .filter_map(Value::as_str)
                .map(|child_id| {
                    rows_by_key
                        .get(&(draw_id.clone(), child_id.to_owned()))
                        .copied()
                        .expect("complete sample grid")
                })
                .collect::<Vec<_>>();
            let raw_parent = number_at(parent_row, "raw_count").expect("raw_count");
            let reconciled_parent = number_at(parent_row, "reconciled_count").expect("reconciled_count");
            let raw_child_sum = child_rows
                .iter()
                .map(|row| number_at(row, "raw_count").expect("raw_count"))
                .sum::<f64>();
            let reconciled_child_sum = child_rows
                .iter()
                .map(|row| number_at(row, "reconciled_count").expect("reconciled_count"))
                .sum::<f64>();
            let raw_residual = raw_parent - raw_child_sum;
            let reconciled_residual = reconciled_parent - reconciled_child_sum;
            let raw_status = if raw_residual.abs() <= tolerance {
                "coherent"
            } else {
                base_draw_coherent = false;
                "incoherent"
            };
            let reconciled_status = if reconciled_residual.abs() <= tolerance {
                "coherent"
            } else {
                reconciled_draw_coherent = false;
                "incoherent"
            };
            raw_abs_residuals.push(raw_residual.abs());
            reconciled_abs_residuals.push(reconciled_residual.abs());
            coherence.push(json!({
                "draw_id": draw_id,
                "constraint_id": constraint["constraint_id"],
                "kind": constraint["kind"],
                "parent_forecast_id": parent_id,
                "parent_unit_id": parent_node["unit_id"],
                "raw_parent_count": raw_parent,
                "raw_child_sum": raw_child_sum,
                "raw_signed_residual": raw_residual,
                "raw_status": raw_status,
                "reconciled_parent_count": reconciled_parent,
                "reconciled_child_sum": reconciled_child_sum,
                "reconciled_signed_residual": reconciled_residual,
                "reconciled_status": reconciled_status,
                "absolute_tolerance": tolerance
            }));
        }
        if base_draw_coherent {
            base_coherent_draw_count += 1;
        }
        if reconciled_draw_coherent {
            reconciled_coherent_draw_count += 1;
        }
    }

    let total_count = draw_ids.len();
    let mean_abs_raw_residual =
        raw_abs_residuals.iter().sum::<f64>() / raw_abs_residuals.len() as f64;
    let mean_abs_reconciled_residual =
        reconciled_abs_residuals.iter().sum::<f64>() / reconciled_abs_residuals.len() as f64;
    let total_abs_adjustment = rows
        .iter()
        .map(|row| {
            (number_at(row, "reconciled_count").expect("reconciled_count")
                - number_at(row, "raw_count").expect("raw_count"))
            .abs()
        })
        .sum::<f64>();
    let max_abs_adjustment = rows
        .iter()
        .map(|row| {
            (number_at(row, "reconciled_count").expect("reconciled_count")
                - number_at(row, "raw_count").expect("raw_count"))
            .abs()
        })
        .fold(0.0_f64, f64::max);

    let registration_id = string_at(&registration, "registration_id")
        .expect("schema validated registration_id");
    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let input_sha256 = sha256_hex(input);
    let implementation_sha256 = sha256_hex(IMPLEMENTATION_BYTES);
    let receipt_commitment_sha256 = sha256_hex(
        format!(
            "pirb-joint-score-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}\n{implementation_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "joint_forecast_score_oracle",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "provider": registration["provider"]["provider_id"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "score_implementation_sha256": implementation_sha256,
        "forecast_artifact_sha256": registration["provider"]["forecast_artifact_sha256"],
        "outcome_snapshot_sha256": registration["provider"]["outcome_snapshot_sha256"],
        "hierarchy_sha256": registration["provider"]["hierarchy_sha256"],
        "method_sha256": registration["sample_policy"]["method_sha256"],
        "score_policy": registration["score_policy"],
        "scored_node_ids": scored_ids,
        "observed_vector": observed,
        "draw_count": total_count,
        "score_results": {
            "base_unreconciled": raw_scores,
            "reconciled": reconciled_scores,
            "paired_delta": score_delta
        },
        "coherence_diagnostics": coherence,
        "coherence_summary": {
            "draw_count": total_count,
            "base_coherent_draw_count": base_coherent_draw_count,
            "reconciled_coherent_draw_count": reconciled_coherent_draw_count,
            "base_coherent_draw_rate": base_coherent_draw_count as f64 / total_count as f64,
            "reconciled_coherent_draw_rate": reconciled_coherent_draw_count as f64 / total_count as f64,
            "mean_absolute_base_residual": mean_abs_raw_residual,
            "mean_absolute_reconciled_residual": mean_abs_reconciled_residual,
            "total_absolute_row_adjustment": total_abs_adjustment,
            "maximum_absolute_row_adjustment": max_abs_adjustment,
            "numeric_tolerance": tolerance
        },
        "metric_semantics": {
            "energy_score": "mean Euclidean distance to observation minus ordered pairwise Euclidean-distance V-statistic divided by 2*M^2",
            "variogram_score": "sum over non-overlapping vector coordinate pairs of the squared gap between observed and ensemble-mean absolute pair difference; exponent=1, all weights=1",
            "score_orientation": "lower_is_better",
            "comparison_type": "reconciled_vs_unreconciled_distribution",
            "log_score_used": false,
            "scored_vector_parent_child_overlap": false,
            "shared_draw_ids_authenticate_statistical_coupling": false
        },
        "claims": {
            "contract_validated": true,
            "score_formulas_computed": true,
            "formula_oracle_validation_scope": "synthetic hand-calculated fixtures only",
            "forecast_calibration_established": false,
            "real_world_forecast_superiority_established": false,
            "decision_utility_established": false,
            "causal_impact_established": false,
            "live_decision_authority": false
        },
        "receipt_commitment_sha256": receipt_commitment_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-joint-score-eval --input REGISTRATION.json [--source-commit SHA]"
            .to_owned()
    })?;
    let input = fs::read(&input_path).map_err(|error| {
        format!("cannot read joint-score registration {input_path}: {error}")
    })?;
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

    fn parse(bytes: &[u8]) -> Value {
        serde_json::from_slice(bytes).expect("synthetic score fixture is valid JSON")
    }

    fn audit(value: &Value) -> Result<Value, Vec<String>> {
        validate_and_score(
            &serde_json::to_vec(value).expect("registration serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-12,
            "actual {actual:.16}, expected {expected:.16}"
        );
    }

    #[test]
    fn four_draw_two_dimensional_scores_match_hand_calculated_golden_values() {
        let receipt = validate_and_score(
            FOUR_DRAW_BYTES,
            "0123456789abcdef0123456789abcdef01234567",
        )
        .expect("four-draw oracle fixture validates");
        let base = &receipt["score_results"]["base_unreconciled"];
        let reconciled = &receipt["score_results"]["reconciled"];
        let root_two = 2.0_f64.sqrt();
        close(
            base["energy_score"].as_f64().unwrap(),
            (2.0 + root_two) / 8.0,
        );
        close(base["variogram_score"].as_f64().unwrap(), 0.25);
        close(
            reconciled["energy_score"].as_f64().unwrap(),
            (2.0 + root_two) / 4.0,
        );
        close(reconciled["variogram_score"].as_f64().unwrap(), 1.0);
        assert_eq!(receipt["draw_count"], 4);
        assert_eq!(
            receipt["coherence_summary"]["reconciled_coherent_draw_count"],
            4
        );
        assert_eq!(receipt["coherence_summary"]["base_coherent_draw_count"], 1);
        assert_eq!(
            receipt["claims"]["real_world_forecast_superiority_established"],
            false
        );
    }

    #[test]
    fn independent_three_draw_scores_match_hand_calculated_golden_values() {
        let receipt = validate_and_score(
            THREE_DRAW_BYTES,
            "0123456789abcdef0123456789abcdef01234567",
        )
        .expect("three-draw oracle fixture validates");
        let scores = &receipt["score_results"]["base_unreconciled"];
        close(
            scores["energy_score"].as_f64().unwrap(),
            2.0 * (2.0 + 2.0_f64.sqrt()) / 9.0,
        );
        close(scores["variogram_score"].as_f64().unwrap(), 4.0 / 9.0);
        assert_eq!(receipt["draw_count"], 3);
    }

    #[test]
    fn parent_and_child_cannot_be_scored_in_the_same_vector() {
        let mut value = parse(FOUR_DRAW_BYTES);
        value["scored_node_ids"] = json!([
            "fc-country-a-organized",
            "fc-admin1-a-organized"
        ]);
        value["observed_vector"] = json!([
            {"forecast_id":"fc-country-a-organized","observed_count":0},
            {"forecast_id":"fc-admin1-a-organized","observed_count":0}
        ]);
        let errors = audit(&value).expect_err("aggregate plus child double-counts a target");
        assert!(errors.iter().any(|error| error.contains("double-counts aggregate parent")));
    }

    #[test]
    fn missing_sample_cell_is_rejected_not_imputed() {
        let mut value = parse(FOUR_DRAW_BYTES);
        value["sample_rows"]
            .as_array_mut()
            .expect("sample_rows is an array")
            .remove(4);
        let errors = audit(&value).expect_err("every draw-node cell is required");
        assert!(errors.iter().any(|error| error.contains("sample row count")));
        assert!(errors.iter().any(|error| error.contains("missing sample cell")));
    }

    #[test]
    fn duplicated_draw_node_pair_is_rejected() {
        let mut value = parse(FOUR_DRAW_BYTES);
        let duplicate = value["sample_rows"][0].clone();
        value["sample_rows"]
            .as_array_mut()
            .expect("sample_rows is an array")
            .push(duplicate);
        let errors = audit(&value).expect_err("duplicate sample cell is ambiguous");
        assert!(errors.iter().any(|error| error.contains("duplicate sample row")));
    }

    #[test]
    fn fit_cutoff_at_forecast_origin_is_rejected() {
        let mut value = parse(FOUR_DRAW_BYTES);
        value["sample_policy"]["parameter_fit_cutoff_index"] = json!(10);
        let errors = audit(&value).expect_err("held-out fit data must precede issue time");
        assert!(errors.iter().any(|error| error.contains("fit cutoff")));
    }

    #[test]
    fn observed_vector_order_must_be_explicit_and_stable() {
        let mut value = parse(FOUR_DRAW_BYTES);
        value["observed_vector"]
            .as_array_mut()
            .expect("observed_vector is an array")
            .reverse();
        let errors = audit(&value).expect_err("coordinate ordering must be pinned");
        assert!(errors.iter().any(|error| error.contains("order must exactly match")));
    }

    #[test]
    fn receipt_metrics_are_invariant_to_sample_row_order() {
        let mut value = parse(FOUR_DRAW_BYTES);
        let first = audit(&value).expect("first order is valid");
        value["sample_rows"]
            .as_array_mut()
            .expect("sample_rows is an array")
            .reverse();
        let second = audit(&value).expect("reordered rows are valid");
        assert_eq!(
            first["score_results"]["base_unreconciled"],
            second["score_results"]["base_unreconciled"]
        );
        assert_eq!(first["score_results"]["reconciled"], second["score_results"]["reconciled"]);
        assert_ne!(first["input_sha256"], second["input_sha256"]);
    }

    #[test]
    fn unknown_or_duplicate_node_references_are_rejected() {
        let mut value = parse(FOUR_DRAW_BYTES);
        value["scored_node_ids"][1] = json!("fc-missing-node");
        value["observed_vector"][1]["forecast_id"] = json!("fc-missing-node");
        let errors = audit(&value).expect_err("unknown target node must fail closed");
        assert!(errors.iter().any(|error| error.contains("unknown forecast node")));
    }
}
