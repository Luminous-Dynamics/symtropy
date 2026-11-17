// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic B0-B3 hydrology benchmark metric and split-integrity evaluator.
//!
//! This crate does not implement or calibrate a hydrologic solver. It recomputes
//! metrics from frozen arrays and audits whether a benchmark split can support
//! the claim being made.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CASE_COUNT: usize = 37;
pub const SCHEMA_VERSION: &str = "hydro-contextual-t0-v0.1";
pub const CLAIM_CEILING: &str =
    "synthetic_metric_and_split_integrity_only_no_hydrology_model_or_field_qualification";

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field: {key}"))
}

fn text(value: &Value, key: &str) -> Result<String, String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{key}: expected non-empty string"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{key}: expected finite number"))
}

fn positive(value: f64, name: &str) -> Result<f64, String> {
    if value <= 0.0 {
        Err(format!("{name}: must be positive"))
    } else {
        Ok(value)
    }
}

fn boolean(value: &Value, key: &str) -> Result<bool, String> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("{key}: expected boolean"))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key}: expected array"))
}

fn strings(value: &Value, key: &str) -> Result<Vec<String>, String> {
    array(value, key)?
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key}[{i}]: expected string"))
        })
        .collect()
}

fn numeric_array(value: &Value, key: &str) -> Result<Vec<f64>, String> {
    let values = array(value, key)?;
    if values.is_empty() {
        return Err(format!("{key}: array must not be empty"));
    }
    values
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.as_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("{key}[{i}]: expected finite numeric value"))
        })
        .collect()
}

fn nearly_equal(a: f64, b: f64) -> bool {
    let scale = a.abs().max(b.abs()).max(1.0);
    (a - b).abs() <= 1.0e-9 * scale
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

pub fn score_series(observed: &[f64], predicted: &[f64], tolerance: f64) -> Result<Value, String> {
    if observed.len() != predicted.len() || observed.len() < 2 {
        return Err("observed and predicted arrays must have equal length >= 2".to_owned());
    }
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err("denominator_tolerance must be finite and non-negative".to_owned());
    }
    let mean_obs = mean(observed);
    let mean_pred = mean(predicted);
    let obs_ss = observed.iter().map(|v| (v - mean_obs).powi(2)).sum::<f64>();
    let pred_ss = predicted
        .iter()
        .map(|v| (v - mean_pred).powi(2))
        .sum::<f64>();
    let residual_ss = observed
        .iter()
        .zip(predicted)
        .map(|(o, p)| (p - o).powi(2))
        .sum::<f64>();
    let mae = observed
        .iter()
        .zip(predicted)
        .map(|(o, p)| (p - o).abs())
        .sum::<f64>()
        / observed.len() as f64;
    let sum_obs = observed.iter().sum::<f64>();
    let sum_pred = predicted.iter().sum::<f64>();

    let nse = if obs_ss <= tolerance {
        None
    } else {
        Some(1.0 - residual_ss / obs_ss)
    };
    let volume_bias_pct = if sum_obs.abs() <= tolerance {
        None
    } else {
        Some((sum_pred - sum_obs) / sum_obs * 100.0)
    };

    let kge = if obs_ss <= tolerance || pred_ss <= tolerance || mean_obs.abs() <= tolerance {
        None
    } else {
        let covariance = observed
            .iter()
            .zip(predicted)
            .map(|(o, p)| (o - mean_obs) * (p - mean_pred))
            .sum::<f64>();
        let correlation = covariance / (obs_ss * pred_ss).sqrt();
        let alpha = (pred_ss / obs_ss).sqrt();
        let beta = mean_pred / mean_obs;
        Some(
            1.0 - ((correlation - 1.0).powi(2) + (alpha - 1.0).powi(2) + (beta - 1.0).powi(2))
                .sqrt(),
        )
    };

    Ok(json!({
        "sample_count": observed.len(),
        "mean_observed": mean_obs,
        "mean_predicted": mean_pred,
        "nse": nse,
        "kge": kge,
        "volume_bias_pct": volume_bias_pct,
        "mae": mae,
        "nse_defined": nse.is_some(),
        "kge_defined": kge.is_some(),
        "volume_bias_defined": volume_bias_pct.is_some()
    }))
}

fn compute_baseline(input: &Value) -> Result<Value, String> {
    let observed = numeric_array(input, "observed")?;
    let predicted = numeric_array(input, "predicted")?;
    let tolerance = number(input, "denominator_tolerance")?;
    let mut scores = score_series(&observed, &predicted, tolerance)?;
    let baseline_id = text(input, "baseline_id")?;
    scores["baseline_id"] = json!(baseline_id);
    scores["baseline_scope"] = json!(text(input, "baseline_scope")?);
    Ok(scores)
}

fn compute_ensemble(input: &Value) -> Result<Value, String> {
    let observed = numeric_array(input, "observed")?;
    let tolerance = number(input, "denominator_tolerance")?;
    let report_invalid = boolean(input, "invalid_member_reporting")?;
    let members = array(input, "members")?;
    if members.is_empty() {
        return Err("members: at least one prior member is required".to_owned());
    }
    let mut ids = BTreeSet::new();
    let mut valid_nse = Vec::new();
    let mut invalid_ids = Vec::new();
    let mut member_results = Vec::new();

    for member in members {
        let id = text(member, "member_id")?;
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate prior member ID: {id}"));
        }
        let valid = boolean(member, "run_valid")?;
        let mass_balance = boolean(member, "mass_balance_pass")?;
        let predictions = numeric_array(member, "predicted")?;
        let scores = score_series(&observed, &predictions, tolerance)?;
        let member_valid = valid && mass_balance;
        if member_valid {
            if let Some(nse) = scores["nse"].as_f64() {
                valid_nse.push(nse);
            }
        } else {
            invalid_ids.push(id.clone());
        }
        member_results.push(json!({
            "member_id": id,
            "run_valid": valid,
            "mass_balance_pass": mass_balance,
            "member_disposition": if member_valid { "VALID" } else { "INVALID_REPORTED" },
            "scores": scores
        }));
    }

    if !invalid_ids.is_empty() && !report_invalid {
        return Err("invalid prior members hidden from the reported ensemble".to_owned());
    }
    valid_nse.sort_by(f64::total_cmp);
    let median = if valid_nse.is_empty() {
        None
    } else if valid_nse.len() % 2 == 1 {
        Some(valid_nse[valid_nse.len() / 2])
    } else {
        let mid = valid_nse.len() / 2;
        Some((valid_nse[mid - 1] + valid_nse[mid]) / 2.0)
    };
    Ok(json!({
        "member_count": members.len(),
        "valid_member_count": members.len() - invalid_ids.len(),
        "invalid_member_count": invalid_ids.len(),
        "invalid_member_ids": invalid_ids,
        "valid_member_nse": valid_nse,
        "median_valid_member_nse": median,
        "members": member_results,
        "invalid_members_are_visible": true
    }))
}

fn compute_calibration(input: &Value) -> Result<Value, String> {
    let fit_scope = text(input, "fit_scope")?;
    let test_accessed = boolean(input, "test_targets_accessed")?;
    if fit_scope != "train_only" {
        return Ok(json!({
            "calibration_disposition": "REJECT",
            "reason": "CALIBRATION_FIT_SCOPE_NOT_TRAIN_ONLY",
            "start_count": null,
            "mean_score": null,
            "median_score": null,
            "minimum_score": null,
            "maximum_score": null
        }));
    }
    if test_accessed {
        return Ok(json!({
            "calibration_disposition": "REJECT",
            "reason": "TEST_TARGETS_ACCESSED_DURING_CALIBRATION",
            "start_count": null,
            "mean_score": null,
            "median_score": null,
            "minimum_score": null,
            "maximum_score": null
        }));
    }
    let mut scores = numeric_array(input, "start_scores")?;
    scores.sort_by(f64::total_cmp);
    let median = if scores.len() % 2 == 1 {
        scores[scores.len() / 2]
    } else {
        (scores[scores.len() / 2 - 1] + scores[scores.len() / 2]) / 2.0
    };
    Ok(json!({
        "calibration_disposition": "ADMITTED_TRAINING_REFERENCE",
        "reason": "REPEATED_TRAIN_ONLY_STARTS",
        "start_count": scores.len(),
        "mean_score": mean(&scores),
        "median_score": median,
        "minimum_score": scores[0],
        "maximum_score": scores[scores.len() - 1]
    }))
}

fn compute_attainment(input: &Value) -> Result<Value, String> {
    let model = number(input, "model_score")?;
    let lower = number(input, "lower_reference")?;
    let upper = number(input, "upper_reference")?;
    let tolerance = number(input, "denominator_tolerance")?;
    let comparable = boolean(input, "references_comparable")?;
    if tolerance < 0.0 {
        return Err("denominator_tolerance must be non-negative".to_owned());
    }
    if !comparable {
        return Ok(json!({
            "attainment_disposition": "NOT_COMPARABLE",
            "reason": "REFERENCE_POPULATION_OR_TASK_MISMATCH",
            "model_score": model,
            "lower_reference": lower,
            "upper_reference": upper,
            "attainment": null
        }));
    }
    let denominator = upper - lower;
    if denominator.abs() <= tolerance {
        return Ok(json!({
            "attainment_disposition": "UNDEFINED",
            "reason": "REFERENCE_GAP_ZERO_OR_WITHIN_TOLERANCE",
            "model_score": model,
            "lower_reference": lower,
            "upper_reference": upper,
            "attainment": null
        }));
    }
    Ok(json!({
        "attainment_disposition": "DEFINED",
        "reason": "COMPARABLE_REFERENCES",
        "model_score": model,
        "lower_reference": lower,
        "upper_reference": upper,
        "attainment": (model - lower) / denominator
    }))
}

fn split_result(disposition: &str, reason: &str) -> Value {
    json!({
        "split_disposition": disposition,
        "reason": reason,
        "predictive_claim_eligible": disposition == "ADMITTED"
    })
}

fn compute_split(case_id: &str, input: &Value) -> Result<Value, String> {
    match case_id {
        "HC-T0-008-event-overlap-reject" => {
            let train = strings(input, "train_event_ids")?;
            let test = strings(input, "test_event_ids")?;
            if train.iter().any(|id| test.contains(id)) {
                Ok(split_result("REJECT", "EVENT_GROUP_OVERLAP"))
            } else {
                Ok(split_result("ADMITTED", "NO_EVENT_GROUP_OVERLAP"))
            }
        }
        "HC-T0-009-future-fit-scope-reject" => {
            for key in [
                "climatology_fit_scope",
                "normalization_fit_scope",
                "imputation_fit_scope",
            ] {
                if text(input, key)? != "train_only" {
                    return Ok(split_result(
                        "REJECT",
                        "PREPROCESSING_FIT_SCOPE_NOT_TRAIN_ONLY",
                    ));
                }
            }
            Ok(split_result("ADMITTED", "ALL_TRANSFORMS_TRAIN_ONLY"))
        }
        "HC-T0-010-nested-catchment-reject" => {
            let train = strings(input, "train_catchment_ids")?;
            let test = strings(input, "test_catchment_ids")?;
            for pair in array(input, "nested_pairs")? {
                let pair = pair
                    .as_array()
                    .ok_or_else(|| "nested_pairs item must be pair".to_owned())?;
                if pair.len() != 2 {
                    return Err("nested_pairs item must have exactly two IDs".to_owned());
                }
                let upstream = pair[0]
                    .as_str()
                    .ok_or_else(|| "upstream ID must be string".to_owned())?;
                let downstream = pair[1]
                    .as_str()
                    .ok_or_else(|| "downstream ID must be string".to_owned())?;
                if (train.iter().any(|id| id == upstream) && test.iter().any(|id| id == downstream))
                    || (train.iter().any(|id| id == downstream)
                        && test.iter().any(|id| id == upstream))
                {
                    return Ok(split_result("REJECT", "NESTED_CATCHMENT_LEAKAGE"));
                }
            }
            Ok(split_result(
                "ADMITTED",
                "NO_DECLARED_NESTED_CATCHMENT_CROSSING",
            ))
        }
        "HC-T0-011-heldout-target-calibration-reject" => {
            if boolean(input, "heldout_targets_accessed")? {
                Ok(split_result(
                    "REJECT",
                    "HELDOUT_TARGETS_USED_FOR_CALIBRATION",
                ))
            } else {
                Ok(split_result("ADMITTED", "HELDOUT_TARGETS_NOT_ACCESSED"))
            }
        }
        "HC-T0-012-missing-flow-zero-reject" => {
            let state = text(input, "observation_state")?;
            let value = number(input, "value")?;
            let policy = text(input, "imputation_policy")?;
            if state == "MISSING" && value == 0.0 && policy != "source_backed_zero" {
                Ok(split_result("REJECT", "MISSING_FLOW_COERCED_TO_ZERO"))
            } else {
                Ok(split_result("ADMITTED", "MISSINGNESS_SEMANTICS_PRESERVED"))
            }
        }
        "HC-T0-013-unknown-anthropogenic-abstain" => {
            let dam_release = boolean(input, "dam_release_known")?;
            let abstraction = boolean(input, "abstraction_known")?;
            let diversion = boolean(input, "diversion_known")?;
            if !dam_release || !abstraction || !diversion {
                Ok(split_result(
                    "ABSTAIN_OR_DOWNGRADE",
                    "ANTHROPOGENIC_BOUNDARY_UNKNOWN",
                ))
            } else {
                Ok(split_result("ADMITTED", "ANTHROPOGENIC_BOUNDARY_DECLARED"))
            }
        }
        "HC-T0-014-timebase-mismatch-reject" => {
            let forcing = text(input, "forcing_time_semantics")?;
            let observation = text(input, "observation_time_semantics")?;
            if forcing != observation {
                Ok(split_result("REJECT", "TIMEBASE_OR_AGGREGATION_MISMATCH"))
            } else {
                Ok(split_result("ADMITTED", "TIMEBASE_MATCHED"))
            }
        }
        "HC-T0-015-physical-gate-reject" => {
            if !boolean(input, "water_balance_pass")? {
                Ok(split_result(
                    "REJECT_PHYSICAL_GATE",
                    "WATER_BALANCE_GATE_FAILED",
                ))
            } else {
                Ok(split_result("ADMITTED", "PHYSICAL_GATE_PASSED"))
            }
        }
        "HC-T0-016-invalid-members-hidden-reject" => {
            let has_invalid = boolean(input, "has_invalid_members")?;
            let reported = boolean(input, "invalid_members_reported")?;
            if has_invalid && !reported {
                Ok(split_result("REJECT", "INVALID_ENSEMBLE_MEMBERS_HIDDEN"))
            } else {
                Ok(split_result(
                    "ADMITTED",
                    "INVALID_MEMBER_POPULATION_VISIBLE",
                ))
            }
        }
        "HC-T0-017-synthetic-field-claim-reject" => {
            let evidence = text(input, "evidence_class")?;
            let claim = text(input, "requested_claim_class")?;
            if evidence == "synthetic_only" && claim == "field_performance" {
                Ok(split_result(
                    "REJECT_CLAIM_PROMOTION",
                    "SYNTHETIC_EVIDENCE_CANNOT_SUPPORT_FIELD_CLAIM",
                ))
            } else {
                Ok(split_result("ADMITTED", "CLAIM_WITHIN_EVIDENCE_CLASS"))
            }
        }
        "HC-T0-018-duplicate-time-reject" => {
            let timestamps = strings(input, "timestamps")?;
            let mut unique = BTreeSet::new();
            if timestamps.iter().any(|stamp| !unique.insert(stamp.clone())) {
                Ok(split_result("REJECT", "DUPLICATE_TIMESTAMP"))
            } else if timestamps.windows(2).any(|pair| pair[0] >= pair[1]) {
                Ok(split_result("REJECT", "NONMONOTONIC_TIMESTAMP"))
            } else {
                Ok(split_result("ADMITTED", "TIMESTAMPS_UNIQUE_AND_MONOTONIC"))
            }
        }
        "HC-T0-019-overlapping-event-windows-reject" => {
            let windows = array(input, "windows")?;
            for (index, window) in windows.iter().enumerate() {
                let event_id = text(window, "event_id")?;
                let start = number(window, "start_index")?;
                let end = number(window, "end_index")?;
                if end <= start {
                    return Err("event window end must exceed start".to_owned());
                }
                for other in windows.iter().skip(index + 1) {
                    if text(other, "event_id")? == event_id {
                        let other_start = number(other, "start_index")?;
                        let other_end = number(other, "end_index")?;
                        if start.max(other_start) < end.min(other_end) {
                            return Ok(split_result(
                                "REJECT",
                                "OVERLAPPING_WINDOWS_DOUBLE_COUNT_EVENT",
                            ));
                        }
                    }
                }
            }
            Ok(split_result("ADMITTED", "EVENT_WINDOWS_NONOVERLAPPING"))
        }
        "HC-T0-021-final-test-model-selection-reject" => {
            if boolean(input, "model_selected_using_test_scores")? {
                Ok(split_result(
                    "REJECT",
                    "FINAL_TEST_USED_FOR_MODEL_SELECTION",
                ))
            } else {
                Ok(split_result(
                    "ADMITTED",
                    "FINAL_TEST_NOT_USED_FOR_MODEL_SELECTION",
                ))
            }
        }
        _ => Err(format!("unsupported split-control case: {case_id}")),
    }
}

fn event_peak_metrics(input: &Value) -> Result<Value, String> {
    let event_id = text(input, "event_id")?;
    let observed = numeric_array(input, "observed")?;
    let predicted = numeric_array(input, "predicted")?;
    let start = field(input, "event_start_index")?
        .as_u64()
        .ok_or_else(|| "event_start_index: expected non-negative integer".to_owned())?
        as usize;
    let end = field(input, "event_end_index")?
        .as_u64()
        .ok_or_else(|| "event_end_index: expected non-negative integer".to_owned())?
        as usize;
    let step_hours = number(input, "step_hours")?;
    let tolerance = number(input, "denominator_tolerance")?;
    if observed.len() != predicted.len() || observed.len() < 2 {
        return Err("event observed/predicted arrays must have equal length >= 2".to_owned());
    }
    if start >= end || end > observed.len() {
        return Err("event window must be non-empty and within the supplied arrays".to_owned());
    }
    if step_hours <= 0.0 || tolerance < 0.0 {
        return Err(
            "step_hours must be positive and denominator tolerance non-negative".to_owned(),
        );
    }
    let obs_window = &observed[start..end];
    let pred_window = &predicted[start..end];
    let observed_peak = obs_window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let predicted_peak = pred_window
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let obs_peak_offset = obs_window
        .iter()
        .position(|value| *value == observed_peak)
        .ok_or_else(|| "event window has no observations".to_owned())?;
    let pred_peak_offset = pred_window
        .iter()
        .position(|value| *value == predicted_peak)
        .ok_or_else(|| "event window has no predictions".to_owned())?;
    let magnitude_error_pct = if observed_peak.abs() <= tolerance {
        None
    } else {
        Some((predicted_peak - observed_peak) / observed_peak * 100.0)
    };
    let timing_error_hours = (pred_peak_offset as f64 - obs_peak_offset as f64) * step_hours;
    let event_scores = score_series(obs_window, pred_window, tolerance)?;
    Ok(json!({
        "event_id": event_id,
        "event_start_index": start,
        "event_end_index_exclusive": end,
        "sample_count": end - start,
        "observed_peak_flow": observed_peak,
        "predicted_peak_flow": predicted_peak,
        "peak_magnitude_error_pct": magnitude_error_pct,
        "observed_peak_index": start + obs_peak_offset,
        "predicted_peak_index": start + pred_peak_offset,
        "signed_peak_timing_error_hours": timing_error_hours,
        "absolute_peak_timing_error_hours": timing_error_hours.abs(),
        "event_nse": event_scores["nse"],
        "event_volume_bias_pct": event_scores["volume_bias_pct"]
    }))
}

fn low_flow_log_metrics(input: &Value) -> Result<Value, String> {
    let observed = numeric_array(input, "observed")?;
    let predicted = numeric_array(input, "predicted")?;
    let threshold = positive(number(input, "low_flow_threshold")?, "low_flow_threshold")?;
    let transform = text(input, "log_transform")?;
    if observed.len() != predicted.len() {
        return Err("low-flow observed/predicted arrays must have equal length".to_owned());
    }
    if transform != "log10" {
        return Err("unsupported low-flow transform; expected log10".to_owned());
    }
    let indices = observed
        .iter()
        .enumerate()
        .filter_map(|(index, flow)| (*flow <= threshold).then_some(index))
        .collect::<Vec<_>>();
    if indices.is_empty() {
        return Ok(json!({
            "low_flow_disposition": "UNDEFINED",
            "reason": "NO_OBSERVATIONS_AT_OR_BELOW_DECLARED_LOW_FLOW_THRESHOLD",
            "low_flow_threshold": threshold,
            "log_transform": transform,
            "low_flow_sample_count": 0,
            "mean_absolute_log10_error": null,
            "mean_signed_log10_error": null
        }));
    }
    let mut absolute_errors = Vec::with_capacity(indices.len());
    let mut signed_errors = Vec::with_capacity(indices.len());
    for index in &indices {
        let obs = observed[*index];
        let pred = predicted[*index];
        if obs <= 0.0 || pred <= 0.0 {
            return Ok(json!({
                "low_flow_disposition": "INDETERMINATE",
                "reason": "LOG10_TRANSFORM_REQUIRES_STRICTLY_POSITIVE_OBSERVED_AND_PREDICTED_FLOW",
                "low_flow_threshold": threshold,
                "log_transform": transform,
                "low_flow_sample_count": indices.len(),
                "invalid_flow_index": index,
                "mean_absolute_log10_error": null,
                "mean_signed_log10_error": null
            }));
        }
        let error = pred.log10() - obs.log10();
        absolute_errors.push(error.abs());
        signed_errors.push(error);
    }
    Ok(json!({
        "low_flow_disposition": "ADMITTED",
        "reason": "POSITIVE_FLOWS_AND_DECLARED_LOG10_TRANSFORM",
        "low_flow_threshold": threshold,
        "log_transform": transform,
        "low_flow_sample_count": indices.len(),
        "low_flow_indices": indices,
        "mean_absolute_log10_error": mean(&absolute_errors),
        "mean_signed_log10_error": mean(&signed_errors)
    }))
}

fn interval_calibration_metrics(input: &Value) -> Result<Value, String> {
    let observed = numeric_array(input, "observed")?;
    let lower = numeric_array(input, "lower_bound")?;
    let upper = numeric_array(input, "upper_bound")?;
    let nominal = number(input, "nominal_coverage")?;
    if observed.len() != lower.len() || observed.len() != upper.len() {
        return Err("observed and interval-bound arrays must have equal length".to_owned());
    }
    if !(0.0 < nominal && nominal < 1.0) {
        return Err("nominal_coverage must be between zero and one".to_owned());
    }
    if let Some(index) = lower.iter().zip(&upper).position(|(lo, hi)| lo > hi) {
        return Ok(json!({
            "interval_disposition": "REJECT",
            "reason": "LOWER_BOUND_EXCEEDS_UPPER_BOUND",
            "invalid_interval_index": index,
            "sample_count": observed.len(),
            "nominal_coverage": nominal,
            "empirical_coverage": null,
            "coverage_error": null,
            "mean_interval_width": null
        }));
    }
    let covered = observed
        .iter()
        .zip(&lower)
        .zip(&upper)
        .filter(|((obs, lo), hi)| **lo <= **obs && **obs <= **hi)
        .count();
    let widths = lower
        .iter()
        .zip(&upper)
        .map(|(lo, hi)| hi - lo)
        .collect::<Vec<_>>();
    let empirical = covered as f64 / observed.len() as f64;
    Ok(json!({
        "interval_disposition": "ADMITTED",
        "reason": "VALID_ORDERED_INTERVALS",
        "sample_count": observed.len(),
        "covered_count": covered,
        "nominal_coverage": nominal,
        "empirical_coverage": empirical,
        "coverage_error": empirical - nominal,
        "absolute_coverage_error": (empirical - nominal).abs(),
        "mean_interval_width": mean(&widths)
    }))
}

fn ensemble_crps(input: &Value) -> Result<Value, String> {
    let observed = number(input, "observed")?;
    let members = numeric_array(input, "ensemble_members")?;
    if observed < 0.0 || members.iter().any(|value| *value < 0.0) {
        return Ok(json!({
            "probabilistic_disposition": "REJECT",
            "reason": "NEGATIVE_STREAMFLOW_VALUE",
            "member_count": members.len(),
            "observed": observed,
            "crps": null
        }));
    }
    let member_count = members.len() as f64;
    let first_term = members
        .iter()
        .map(|value| (value - observed).abs())
        .sum::<f64>()
        / member_count;
    let pairwise_sum = members
        .iter()
        .flat_map(|left| members.iter().map(move |right| (left - right).abs()))
        .sum::<f64>();
    let crps = first_term - pairwise_sum / (2.0 * member_count * member_count);
    let ensemble_mean = mean(&members);
    let ensemble_variance = members
        .iter()
        .map(|value| (value - ensemble_mean).powi(2))
        .sum::<f64>()
        / member_count;
    Ok(json!({
        "probabilistic_disposition": "ADMITTED",
        "reason": "EMPIRICAL_ENSEMBLE_CRPS",
        "member_count": members.len(),
        "observed": observed,
        "ensemble_mean": ensemble_mean,
        "ensemble_population_stddev": ensemble_variance.sqrt(),
        "crps": crps
    }))
}

fn brier_score(input: &Value) -> Result<Value, String> {
    let probability = number(input, "forecast_probability")?;
    let observed_event = number(input, "observed_event")?;
    let event_id = text(input, "event_definition_id")?;
    if !(0.0..=1.0).contains(&probability) {
        return Ok(json!({
            "probabilistic_disposition": "REJECT",
            "reason": "FORECAST_PROBABILITY_OUTSIDE_ZERO_ONE",
            "event_definition_id": event_id,
            "forecast_probability": probability,
            "observed_event": observed_event,
            "brier_score": null
        }));
    }
    if observed_event != 0.0 && observed_event != 1.0 {
        return Ok(json!({
            "probabilistic_disposition": "REJECT",
            "reason": "OBSERVED_EVENT_MUST_BE_BINARY_ZERO_OR_ONE",
            "event_definition_id": event_id,
            "forecast_probability": probability,
            "observed_event": observed_event,
            "brier_score": null
        }));
    }
    Ok(json!({
        "probabilistic_disposition": "ADMITTED",
        "reason": "BINARY_EVENT_BRIER_SCORE",
        "event_definition_id": event_id,
        "forecast_probability": probability,
        "observed_event": observed_event,
        "brier_score": (probability - observed_event).powi(2)
    }))
}

fn brier_reliability(input: &Value) -> Result<Value, String> {
    let probabilities = numeric_array(input, "forecast_probabilities")?;
    let outcomes = numeric_array(input, "observed_events")?;
    let event_id = text(input, "event_definition_id")?;
    if probabilities.len() != outcomes.len() {
        return Err("forecast probability and event arrays must have equal length".to_owned());
    }
    if let Some(index) = probabilities
        .iter()
        .position(|probability| !(0.0..=1.0).contains(probability))
    {
        return Ok(json!({
            "probabilistic_disposition": "REJECT",
            "reason": "FORECAST_PROBABILITY_OUTSIDE_ZERO_ONE",
            "event_definition_id": event_id,
            "sample_count": probabilities.len(),
            "invalid_probability_index": index,
            "brier_score": null,
            "reliability_bins": null
        }));
    }
    if let Some(index) = outcomes
        .iter()
        .position(|outcome| *outcome != 0.0 && *outcome != 1.0)
    {
        return Ok(json!({
            "probabilistic_disposition": "REJECT",
            "reason": "OBSERVED_EVENT_MUST_BE_BINARY_ZERO_OR_ONE",
            "event_definition_id": event_id,
            "sample_count": outcomes.len(),
            "invalid_outcome_index": index,
            "brier_score": null,
            "reliability_bins": null
        }));
    }
    let brier = probabilities
        .iter()
        .zip(&outcomes)
        .map(|(probability, outcome)| (probability - outcome).powi(2))
        .sum::<f64>()
        / probabilities.len() as f64;
    let mut counts = [0_usize; 5];
    let mut probability_sums = [0.0_f64; 5];
    let mut event_sums = [0.0_f64; 5];
    for (probability, outcome) in probabilities.iter().zip(&outcomes) {
        let index = ((*probability * 5.0).floor() as usize).min(4);
        counts[index] += 1;
        probability_sums[index] += probability;
        event_sums[index] += outcome;
    }
    let bins = (0..5)
        .map(|index| {
            let count = counts[index];
            let mean_probability = if count == 0 {
                None
            } else {
                Some(probability_sums[index] / count as f64)
            };
            let observed_frequency = if count == 0 {
                None
            } else {
                Some(event_sums[index] / count as f64)
            };
            let calibration_error = match (mean_probability, observed_frequency) {
                (Some(probability), Some(frequency)) => Some(frequency - probability),
                _ => None,
            };
            json!({
                "bin_index": index,
                "lower_inclusive": index as f64 / 5.0,
                "upper_exclusive": if index == 4 { Value::Null } else { json!((index + 1) as f64 / 5.0) },
                "sample_count": count,
                "mean_forecast_probability": mean_probability,
                "observed_event_frequency": observed_frequency,
                "observed_minus_forecast_frequency": calibration_error
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "probabilistic_disposition": "ADMITTED",
        "reason": "MEAN_BRIER_AND_FIXED_FIVE_BIN_RELIABILITY_DIAGNOSTIC",
        "event_definition_id": event_id,
        "sample_count": probabilities.len(),
        "event_count": outcomes.iter().filter(|outcome| **outcome == 1.0).count(),
        "brier_score": brier,
        "reliability_bins": bins,
        "reliability_warning": "Five-bin reliability is descriptive only; assess sampling uncertainty and do not interpret sparse bins as calibrated."
    }))
}


fn crps_for_members(observed: f64, members: &[f64]) -> f64 {
    let count = members.len() as f64;
    let first_term = members
        .iter()
        .map(|value| (value - observed).abs())
        .sum::<f64>()
        / count;
    let pairwise_sum = members
        .iter()
        .flat_map(|left| members.iter().map(move |right| (left - right).abs()))
        .sum::<f64>();
    first_term - pairwise_sum / (2.0 * count * count)
}

fn crps_skill(input: &Value) -> Result<Value, String> {
    let observed = number(input, "observed")?;
    let forecast_members = numeric_array(input, "forecast_ensemble_members")?;
    let reference_members = numeric_array(input, "reference_ensemble_members")?;
    let reference_id = text(input, "reference_forecast_id")?;
    let tolerance = number(input, "denominator_tolerance")?;
    if tolerance < 0.0 {
        return Err("denominator_tolerance must be non-negative".to_owned());
    }
    if observed < 0.0
        || forecast_members.iter().any(|value| *value < 0.0)
        || reference_members.iter().any(|value| *value < 0.0)
    {
        return Ok(json!({
            "probabilistic_disposition": "REJECT",
            "reason": "NEGATIVE_STREAMFLOW_VALUE",
            "reference_forecast_id": reference_id,
            "forecast_crps": null,
            "reference_crps": null,
            "crps_skill_score": null
        }));
    }
    let forecast_score = crps_for_members(observed, &forecast_members);
    let reference_score = crps_for_members(observed, &reference_members);
    if reference_score <= tolerance {
        return Ok(json!({
            "probabilistic_disposition": "UNDEFINED_REFERENCE_SCORE",
            "reason": "REFERENCE_CRPS_ZERO_OR_WITHIN_TOLERANCE",
            "reference_forecast_id": reference_id,
            "forecast_member_count": forecast_members.len(),
            "reference_member_count": reference_members.len(),
            "forecast_crps": forecast_score,
            "reference_crps": reference_score,
            "crps_skill_score": null
        }));
    }
    Ok(json!({
        "probabilistic_disposition": "ADMITTED",
        "reason": "CRPS_SKILL_AGAINST_DECLARED_REFERENCE",
        "reference_forecast_id": reference_id,
        "forecast_member_count": forecast_members.len(),
        "reference_member_count": reference_members.len(),
        "forecast_crps": forecast_score,
        "reference_crps": reference_score,
        "crps_skill_score": 1.0 - forecast_score / reference_score
    }))
}

fn brier_skill(input: &Value) -> Result<Value, String> {
    let forecast = numeric_array(input, "forecast_probabilities")?;
    let reference = numeric_array(input, "reference_probabilities")?;
    let outcomes = numeric_array(input, "observed_events")?;
    let event_id = text(input, "event_definition_id")?;
    let reference_id = text(input, "reference_forecast_id")?;
    let tolerance = number(input, "denominator_tolerance")?;
    if forecast.len() != reference.len() || forecast.len() != outcomes.len() {
        return Err(
            "forecast, reference and observed-event arrays must have equal length".to_owned(),
        );
    }
    if tolerance < 0.0 {
        return Err("denominator_tolerance must be non-negative".to_owned());
    }
    if let Some(index) = forecast
        .iter()
        .chain(&reference)
        .position(|probability| !(0.0..=1.0).contains(probability))
    {
        return Ok(json!({
            "probabilistic_disposition": "REJECT",
            "reason": "FORECAST_PROBABILITY_OUTSIDE_ZERO_ONE",
            "event_definition_id": event_id,
            "reference_forecast_id": reference_id,
            "sample_count": forecast.len(),
            "invalid_probability_index": index,
            "forecast_brier_score": null,
            "reference_brier_score": null,
            "brier_skill_score": null
        }));
    }
    if let Some(index) = outcomes
        .iter()
        .position(|outcome| *outcome != 0.0 && *outcome != 1.0)
    {
        return Ok(json!({
            "probabilistic_disposition": "REJECT",
            "reason": "OBSERVED_EVENT_MUST_BE_BINARY_ZERO_OR_ONE",
            "event_definition_id": event_id,
            "reference_forecast_id": reference_id,
            "sample_count": forecast.len(),
            "invalid_outcome_index": index,
            "forecast_brier_score": null,
            "reference_brier_score": null,
            "brier_skill_score": null
        }));
    }
    let forecast_score = forecast
        .iter()
        .zip(&outcomes)
        .map(|(probability, outcome)| (probability - outcome).powi(2))
        .sum::<f64>()
        / forecast.len() as f64;
    let reference_score = reference
        .iter()
        .zip(&outcomes)
        .map(|(probability, outcome)| (probability - outcome).powi(2))
        .sum::<f64>()
        / reference.len() as f64;
    if reference_score <= tolerance {
        return Ok(json!({
            "probabilistic_disposition": "UNDEFINED_REFERENCE_SCORE",
            "reason": "REFERENCE_BRIER_ZERO_OR_WITHIN_TOLERANCE",
            "event_definition_id": event_id,
            "reference_forecast_id": reference_id,
            "sample_count": forecast.len(),
            "forecast_brier_score": forecast_score,
            "reference_brier_score": reference_score,
            "brier_skill_score": null
        }));
    }
    Ok(json!({
        "probabilistic_disposition": "ADMITTED",
        "reason": "BRIER_SKILL_AGAINST_DECLARED_REFERENCE",
        "event_definition_id": event_id,
        "reference_forecast_id": reference_id,
        "sample_count": forecast.len(),
        "forecast_brier_score": forecast_score,
        "reference_brier_score": reference_score,
        "brier_skill_score": 1.0 - forecast_score / reference_score
    }))
}

pub fn compute_case(case: &Value) -> Result<Value, String> {
    let id = text(case, "case_id")?;
    let input = field(case, "inputs")?;
    match id.as_str() {
        "HC-T0-001-b0-persistence-metrics"
        | "HC-T0-002-b0-seasonal-climatology-metrics"
        | "HC-T0-005-b3-future-holdout-metrics"
        | "HC-T0-020-degenerate-observations-undefined" => compute_baseline(input),
        "HC-T0-003-b1-prior-ensemble" => compute_ensemble(input),
        "HC-T0-004-b2-calibration-reference" => compute_calibration(input),
        "HC-T0-006-contextual-attainment" | "HC-T0-007-attainment-degenerate-bounds" => {
            compute_attainment(input)
        }

        "HC-T0-022-event-peak-magnitude-and-timing"
        | "HC-T0-027-event-peak-ties-earliest-index" => event_peak_metrics(input),
        "HC-T0-023-low-flow-log10-error" | "HC-T0-026-low-flow-log-zero-abstains" => {
            low_flow_log_metrics(input)
        }
        "HC-T0-024-probabilistic-interval-coverage-width"
        | "HC-T0-025-invalid-probabilistic-interval-reject" => interval_calibration_metrics(input),

        "HC-T0-028-ensemble-crps" | "HC-T0-032-ensemble-crps-negative-flow-reject" => {
            ensemble_crps(input)
        }
        "HC-T0-029-threshold-event-brier" => brier_score(input),
        "HC-T0-030-brier-reliability-diagnostic" => brier_reliability(input),
        "HC-T0-031-invalid-event-probability-reject" => brier_score(input),

        "HC-T0-033-ensemble-crps-skill"
        | "HC-T0-035-crps-skill-zero-reference-undefined"
        | "HC-T0-036-crps-skill-negative-retained" => crps_skill(input),
        "HC-T0-034-threshold-event-brier-skill" | "HC-T0-037-brier-skill-negative-retained" => {
            brier_skill(input)
        }
        _ => compute_split(&id, input),
    }
}

fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => nearly_equal(a, b),
            _ => false,
        },
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| compare_json(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| compare_json(value, other)))
        }
        _ => actual == expected,
    }
}

fn ids(rows: &[Value], name: &str) -> Result<BTreeSet<String>, String> {
    let mut out = BTreeSet::new();
    for row in rows {
        let id = text(row, "case_id")?;
        if !out.insert(id.clone()) {
            return Err(format!("duplicate {name} case ID: {id}"));
        }
    }
    Ok(out)
}

pub fn evaluate(inputs: &Value, manifest: &Value, oracle: &Value) -> Result<Value, String> {
    for (name, value) in [
        ("inputs", inputs),
        ("manifest", manifest),
        ("oracle", oracle),
    ] {
        if text(value, "schema_version")? != SCHEMA_VERSION
            || text(value, "evidence_class")? != "synthetic_only"
            || text(value, "claim_ceiling")? != CLAIM_CEILING
        {
            return Err(format!("{name}: schema/evidence/claim ceiling mismatch"));
        }
    }
    let cases = array(inputs, "cases")?;
    let expected_cases = array(oracle, "cases")?;
    let manifest_ids = strings(manifest, "required_case_ids")?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let input_ids = ids(cases, "input")?;
    let oracle_ids = ids(expected_cases, "oracle")?;
    let declared_count = field(manifest, "case_count")?.as_u64().unwrap_or(0) as usize;
    if declared_count != CASE_COUNT
        || cases.len() != CASE_COUNT
        || expected_cases.len() != CASE_COUNT
        || input_ids != manifest_ids
        || input_ids != oracle_ids
    {
        return Err(format!(
            "case identity/count mismatch: inputs={}, manifest={}, oracle={}, declared={declared_count}",
            input_ids.len(),
            manifest_ids.len(),
            oracle_ids.len()
        ));
    }
    let mut expected_by_id = BTreeMap::new();
    for row in expected_cases {
        expected_by_id.insert(text(row, "case_id")?, row);
    }

    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    for case in cases {
        let id = text(case, "case_id")?;
        let family = text(case, "family")?;
        let actual = compute_case(case)?;
        let expected_row = expected_by_id
            .get(&id)
            .ok_or_else(|| format!("oracle missing {id}"))?;
        let expected_family = text(expected_row, "family")?;
        let expected = field(expected_row, "expected")?;
        let matched = family == expected_family && compare_json(&actual, expected);
        if !matched {
            mismatches.push(json!({
                "case_id": id,
                "family": family,
                "expected_family": expected_family,
                "expected": expected,
                "computed": actual
            }));
        }
        results.push(json!({"case_id":id,"family":family,"matched":matched,"computed":actual}));
    }
    results.sort_by(|a, b| a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let matched = results.len() - mismatches.len();
    Ok(json!({
        "schema_version":"hydro-contextual-t0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty(){"MatchedFrozenOracle"}else{"OracleMismatch"},
        "qualification_status":"NotRun",
        "evidence_class":"synthetic_only",
        "claim_ceiling":CLAIM_CEILING,
        "result_count":results.len(),
        "matched_case_count":matched,
        "mismatches":mismatches,
        "results":results,
        "nonclaims":[
            "This evaluator recomputes synthetic benchmark metrics and split-integrity semantics only; it is not a hydrology solver or calibration engine.",
            "No South African or Highveld station data are acquired or qualified by this corpus.",
            "No field forecast skill, catchment realism, water-balance, flood, groundwater or ecological performance is established.",
            "Physical gates are consumed as declared synthetic booleans here; the existing #461/#445 physical authorities remain separate."
        ]
    }))
}

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let raw = match name {
            "inputs" => include_str!("../../../research/hydrology/hydro-contextual-t0-v0.1.json"),
            "manifest" => {
                include_str!("../../../research/hydrology/hydro-contextual-manifest-v0.1.json")
            }
            "oracle" => {
                include_str!("../../../research/hydrology/hydro-contextual-oracle-v0.1.json")
            }
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(raw).expect("fixture JSON")
    }

    #[test]
    fn all_thirty_seven_cases_match_separate_oracle() {
        let result = evaluate(&fixture("inputs"), &fixture("manifest"), &fixture("oracle"))
            .expect("evaluate");
        assert_eq!(result["result_count"], CASE_COUNT);
        assert_eq!(result["matched_case_count"], CASE_COUNT);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
    }
    #[test]
    fn duplicate_case_identity_rejected() {
        let mut inputs = fixture("inputs");
        let first = inputs["cases"][0].clone();
        inputs["cases"].as_array_mut().unwrap().push(first);
        assert!(evaluate(&inputs, &fixture("manifest"), &fixture("oracle")).is_err());
    }
    #[test]
    fn attainment_is_not_clipped() {
        let out = compute_case(&fixture("inputs")["cases"][5]).expect("attainment");
        assert!((out["attainment"].as_f64().expect("attainment value") - 1.2).abs() < 1.0e-12);
    }
    #[test]
    fn zero_width_attainment_abstains() {
        let out = compute_case(&fixture("inputs")["cases"][6]).expect("undefined attainment");
        assert_eq!(out["attainment_disposition"], "UNDEFINED");
        assert_eq!(out["attainment"], Value::Null);
    }
    #[test]
    fn missing_flow_never_becomes_zero() {
        let out = compute_case(&fixture("inputs")["cases"][11]).expect("negative control");
        assert_eq!(out["split_disposition"], "REJECT");
    }
    #[test]
    fn unknown_anthropogenic_boundary_abstains() {
        let out = compute_case(&fixture("inputs")["cases"][12]).expect("unknown boundary");
        assert_eq!(out["split_disposition"], "ABSTAIN_OR_DOWNGRADE");
    }
    #[test]
    fn physical_gate_overrides_good_predictive_score() {
        let out = compute_case(&fixture("inputs")["cases"][14]).expect("physical gate");
        assert_eq!(out["split_disposition"], "REJECT_PHYSICAL_GATE");
    }

    #[test]
    fn event_peak_metrics_report_magnitude_and_signed_timing() {
        let out = compute_case(&fixture("inputs")["cases"][21]).expect("event peak metrics");
        assert_eq!(out["observed_peak_flow"], 8.0);
        assert_eq!(out["predicted_peak_flow"], 7.0);
        assert_eq!(out["peak_magnitude_error_pct"], -12.5);
        assert_eq!(out["signed_peak_timing_error_hours"], -6.0);
    }

    #[test]
    fn low_flow_log_metrics_use_only_predeclared_low_flow_observations() {
        let out = compute_case(&fixture("inputs")["cases"][22]).expect("low-flow metrics");
        assert_eq!(out["low_flow_sample_count"], 4);
        assert_eq!(out["low_flow_indices"], json!([0, 1, 4, 5]));
        assert_eq!(out["low_flow_disposition"], "ADMITTED");
    }

    #[test]
    fn probabilistic_intervals_report_coverage_and_width() {
        let out = compute_case(&fixture("inputs")["cases"][23]).expect("interval metrics");
        assert_eq!(out["covered_count"], 4);
        assert_eq!(out["empirical_coverage"], 1.0);
        assert_eq!(out["mean_interval_width"], 1.375);
    }

    #[test]
    fn crossing_probabilistic_interval_bounds_are_rejected() {
        let out = compute_case(&fixture("inputs")["cases"][24]).expect("invalid interval");
        assert_eq!(out["interval_disposition"], "REJECT");
        assert_eq!(out["reason"], "LOWER_BOUND_EXCEEDS_UPPER_BOUND");
    }

    #[test]
    fn event_peak_ties_select_the_earliest_time_step() {
        let out = compute_case(&fixture("inputs")["cases"][26]).expect("tied event peaks");
        assert_eq!(out["observed_peak_index"], 1);
        assert_eq!(out["predicted_peak_index"], 1);
        assert_eq!(out["signed_peak_timing_error_hours"], 0.0);
    }

    #[test]
    fn log_low_flow_metrics_abstain_on_zero_flow_without_pseudocount() {
        let out =
            compute_case(&fixture("inputs")["cases"][25]).expect("zero-flow low-flow control");
        assert_eq!(out["low_flow_disposition"], "INDETERMINATE");
        assert_eq!(
            out["reason"],
            "LOG10_TRANSFORM_REQUIRES_STRICTLY_POSITIVE_OBSERVED_AND_PREDICTED_FLOW"
        );
    }

    #[test]
    fn ensemble_crps_uses_full_member_distribution() {
        let out = compute_case(&fixture("inputs")["cases"][27]).expect("ensemble CRPS");
        assert_eq!(out["member_count"], 3);
        assert!((out["crps"].as_f64().expect("CRPS") - 2.0 / 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn threshold_event_brier_score_is_a_proper_probability_score() {
        let out = compute_case(&fixture("inputs")["cases"][28]).expect("Brier score");
        assert_eq!(out["probabilistic_disposition"], "ADMITTED");
        assert!((out["brier_score"].as_f64().expect("Brier score") - 0.09).abs() < 1.0e-12);
    }

    #[test]
    fn brier_reliability_reports_empty_bins_as_null() {
        let out = compute_case(&fixture("inputs")["cases"][29]).expect("Brier reliability");
        assert_eq!(out["sample_count"], 5);
        assert!((out["brier_score"].as_f64().expect("Brier score") - 0.0575).abs() < 1.0e-12);
        assert_eq!(out["reliability_bins"][0]["sample_count"], 1);
        assert_eq!(out["reliability_bins"][2]["sample_count"], 1);
        assert_eq!(out["reliability_bins"][3]["sample_count"], 0);
        assert_eq!(
            out["reliability_bins"][3]["observed_event_frequency"],
            Value::Null
        );
        assert_eq!(out["reliability_bins"][4]["sample_count"], 2);
    }

    #[test]
    fn invalid_event_probability_is_rejected_not_clipped() {
        let out = compute_case(&fixture("inputs")["cases"][30]).expect("invalid probability");
        assert_eq!(out["probabilistic_disposition"], "REJECT");
        assert_eq!(out["reason"], "FORECAST_PROBABILITY_OUTSIDE_ZERO_ONE");
        assert_eq!(out["brier_score"], Value::Null);
    }

    #[test]
    fn negative_ensemble_streamflow_is_rejected() {
        let out = compute_case(&fixture("inputs")["cases"][31]).expect("negative ensemble member");
        assert_eq!(out["probabilistic_disposition"], "REJECT");
        assert_eq!(out["reason"], "NEGATIVE_STREAMFLOW_VALUE");
    }

    #[test]
    fn crps_skill_is_relative_to_a_declared_reference_forecast() {
        let out = compute_case(&fixture("inputs")["cases"][32]).expect("CRPS skill");
        assert_eq!(out["reference_forecast_id"], "seasonal_climatology-v1");
        assert!(
            (out["forecast_crps"].as_f64().expect("forecast CRPS") - 2.0 / 3.0).abs() < 1.0e-12
        );
        assert!(
            (out["reference_crps"].as_f64().expect("reference CRPS") - 4.0 / 3.0).abs() < 1.0e-12
        );
        assert!((out["crps_skill_score"].as_f64().expect("CRPSS") - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn brier_skill_compares_to_the_same_event_reference() {
        let out = compute_case(&fixture("inputs")["cases"][33]).expect("Brier skill");
        assert!(
            (out["forecast_brier_score"].as_f64().expect("forecast BS") - 0.29 / 3.0).abs()
                < 1.0e-12
        );
        assert_eq!(out["reference_brier_score"], 0.25);
        assert!(
            (out["brier_skill_score"].as_f64().expect("BSS") - 0.6133333333333333).abs() < 1.0e-12
        );
    }

    #[test]
    fn zero_reference_brier_does_not_create_infinite_skill() {
        let out = compute_case(&fixture("inputs")["cases"][34]).expect("zero reference Brier");
        assert_eq!(
            out["probabilistic_disposition"],
            "UNDEFINED_REFERENCE_SCORE"
        );
        assert_eq!(out["reference_brier_score"], 0.0);
        assert_eq!(out["brier_skill_score"], Value::Null);
    }

    #[test]
    fn negative_crps_skill_is_not_clipped() {
        let out = compute_case(&fixture("inputs")["cases"][35]).expect("negative CRPSS");
        let skill = out["crps_skill_score"].as_f64().expect("CRPSS");
        assert!(skill < 0.0);
        assert!((skill + 4.0 / 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn negative_brier_skill_is_not_clipped() {
        let out = compute_case(&fixture("inputs")["cases"][36]).expect("negative BSS");
        let skill = out["brier_skill_score"].as_f64().expect("BSS");
        assert!(skill < 0.0);
        assert!((skill - (1.0 - (2.26 / 3.0) / 0.25)).abs() < 1.0e-12);
    }

    #[test]
    fn final_test_cannot_select_the_model() {
        let out =
            compute_case(&fixture("inputs")["cases"][20]).expect("final-test selection control");
        assert_eq!(out["split_disposition"], "REJECT");
        assert_eq!(out["reason"], "FINAL_TEST_USED_FOR_MODEL_SELECTION");
    }

    #[test]
    fn degenerate_observations_make_efficiency_metrics_undefined() {
        let out = compute_case(&fixture("inputs")["cases"][19]).expect("degenerate observations");
        assert_eq!(out["nse"], Value::Null);
        assert_eq!(out["kge"], Value::Null);
    }
}
