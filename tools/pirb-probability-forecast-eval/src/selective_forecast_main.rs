// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Audits selective forecast serving and reason-coded abstention. An abstention
// is not a negative event prediction and candidate probabilities are never
// copied to served output for rows withheld by the registered policy.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-selective-forecast-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-selective-forecast-synthetic-v0.1.json");
const LOG_EPSILON: f64 = 1.0e-15;
const SCORE_EPSILON: f64 = 1.0e-12;

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

fn is_known_outcome(row: &Value) -> bool {
    string_at(row, "outcome_status") == Some("known")
        && matches!(integer_at(row, "outcome"), Some(0 | 1))
}

fn matches_registered_target(row: &Value, target_id: &str) -> bool {
    string_at(row, "target_id") == Some(target_id)
}

fn expected_reason(row: &Value, policy: &Value, threshold: f64) -> Option<&'static str> {
    let target_id = string_at(policy, "target_id").unwrap_or_default();
    if !matches_registered_target(row, target_id) {
        return Some("abstain_target_mismatch");
    }
    match string_at(row, "input_status").unwrap_or_default() {
        "stale" => return Some("abstain_stale_inputs"),
        "unknown" => return Some("abstain_unknown_inputs"),
        _ => {}
    }
    match string_at(row, "regime_status").unwrap_or_default() {
        "shifted" => return Some("abstain_out_of_support"),
        "unknown" => return Some("abstain_unknown_regime"),
        _ => {}
    }
    if string_at(row, "evidence_status") != Some("sufficient") {
        return Some("abstain_insufficient_evidence");
    }
    if number_at(row, "selection_score").unwrap_or_default() + SCORE_EPSILON < threshold {
        return Some("abstain_low_selection_score");
    }
    None
}

fn log_loss(probability: f64, outcome: i64) -> f64 {
    let p = probability.clamp(LOG_EPSILON, 1.0 - LOG_EPSILON);
    if outcome == 1 {
        -p.ln()
    } else {
        -(1.0 - p).ln()
    }
}

fn score_metrics<'a>(rows: impl IntoIterator<Item = &'a Value>) -> Value {
    let mut count = 0usize;
    let mut brier_total = 0.0;
    let mut log_loss_total = 0.0;
    for row in rows {
        if !is_known_outcome(row) {
            continue;
        }
        let Some(probability) = number_at(row, "offline_candidate_probability") else {
            continue;
        };
        let outcome = integer_at(row, "outcome").expect("known outcomes are binary integers");
        let error = probability - outcome as f64;
        brier_total += error * error;
        log_loss_total += log_loss(probability, outcome);
        count += 1;
    }
    if count == 0 {
        json!({
            "scored_count": 0,
            "brier_score": Value::Null,
            "log_loss": Value::Null
        })
    } else {
        json!({
            "scored_count": count,
            "brier_score": brier_total / count as f64,
            "log_loss": log_loss_total / count as f64
        })
    }
}

fn selective_metrics<'a>(
    rows: impl IntoIterator<Item = &'a Value>,
    target_id: &str,
    policy: &Value,
    threshold: f64,
) -> Value {
    let rows = rows.into_iter().collect::<Vec<_>>();
    let mut total = 0usize;
    let mut issued = 0usize;
    let mut abstained = 0usize;
    let mut scored_issued = 0usize;
    let mut issued_unknown_outcome = 0usize;
    let mut abstained_unknown_outcome = 0usize;
    let mut target_mismatch_count = 0usize;
    let mut reason_counts = BTreeMap::<String, usize>::new();
    let mut brier_total = 0.0;
    let mut log_loss_total = 0.0;
    let mut eligible_known_outcomes = 0usize;
    let mut all_candidate_brier = 0.0;
    let mut all_candidate_log_loss = 0.0;

    for row in &rows {
        total += 1;
        let target_matches = matches_registered_target(row, target_id);
        if !target_matches {
            target_mismatch_count += 1;
        }
        let known_outcome = is_known_outcome(row);

        if target_matches && known_outcome {
            eligible_known_outcomes += 1;
            let probability = number_at(row, "offline_candidate_probability")
                .expect("schema validated offline candidate probability");
            let outcome = integer_at(row, "outcome").expect("known outcomes are binary");
            let error = probability - outcome as f64;
            all_candidate_brier += error * error;
            all_candidate_log_loss += log_loss(probability, outcome);
        }

        match expected_reason(row, policy, threshold) {
            None => {
                issued += 1;
                if known_outcome && target_matches {
                    let probability = number_at(row, "offline_candidate_probability")
                        .expect("schema validated offline candidate probability");
                    let outcome = integer_at(row, "outcome").expect("known outcomes are binary");
                    let error = probability - outcome as f64;
                    brier_total += error * error;
                    log_loss_total += log_loss(probability, outcome);
                    scored_issued += 1;
                } else {
                    issued_unknown_outcome += usize::from(target_matches);
                }
            }
            Some(reason) => {
                abstained += 1;
                *reason_counts.entry(reason.to_owned()).or_default() += 1;
                if string_at(row, "outcome_status") != Some("known") {
                    abstained_unknown_outcome += 1;
                }
            }
        }
    }

    let coverage = if total == 0 {
        Value::Null
    } else {
        json!(issued as f64 / total as f64)
    };
    let scored_coverage = if eligible_known_outcomes == 0 {
        Value::Null
    } else {
        json!(scored_issued as f64 / eligible_known_outcomes as f64)
    };
    let score = if scored_issued == 0 {
        json!({
            "scored_count": 0,
            "brier_score": Value::Null,
            "log_loss": Value::Null
        })
    } else {
        json!({
            "scored_count": scored_issued,
            "brier_score": brier_total / scored_issued as f64,
            "log_loss": log_loss_total / scored_issued as f64
        })
    };
    let candidate_baseline = if eligible_known_outcomes == 0 {
        json!({
            "eligible_known_outcome_count": 0,
            "brier_score": Value::Null,
            "log_loss": Value::Null
        })
    } else {
        json!({
            "eligible_known_outcome_count": eligible_known_outcomes,
            "brier_score": all_candidate_brier / eligible_known_outcomes as f64,
            "log_loss": all_candidate_log_loss / eligible_known_outcomes as f64
        })
    };
    let reason_counts = reason_counts
        .into_iter()
        .map(|(reason, count)| json!({"reason_code": reason, "count": count}))
        .collect::<Vec<_>>();

    json!({
        "threshold": threshold,
        "total_case_count": total,
        "forecast_issued_count": issued,
        "abstained_count": abstained,
        "decision_coverage": coverage,
        "target_mismatch_count": target_mismatch_count,
        "scored_issued_count": scored_issued,
        "eligible_known_target_outcome_count": eligible_known_outcomes,
        "scored_coverage_of_known_target_outcomes": scored_coverage,
        "issued_but_unscored_unknown_or_immature_outcome_count": issued_unknown_outcome,
        "abstained_unknown_or_immature_outcome_count": abstained_unknown_outcome,
        "abstention_reason_counts": reason_counts,
        "issued_case_scores": score,
        "offline_candidate_full_coverage_reference": candidate_baseline
    })
}

fn stratum_summary(
    rows: &[&Value],
    policy: &Value,
    target_id: &str,
    threshold: f64,
    minimum_group_count: usize,
) -> Value {
    if rows.len() < minimum_group_count {
        return json!({
            "suppressed": true,
            "suppression_reason": "below_minimum_group_report_count"
        });
    }
    let mut issued = 0usize;
    let mut abstained = 0usize;
    let mut scored = Vec::new();
    let mut unknown_outcome = 0usize;
    let mut reason_counts = BTreeMap::<String, usize>::new();

    for row in rows {
        match expected_reason(row, policy, threshold) {
            None => {
                issued += 1;
                if is_known_outcome(row) && matches_registered_target(row, target_id) {
                    scored.push(*row);
                } else if string_at(row, "outcome_status") != Some("known") {
                    unknown_outcome += 1;
                }
            }
            Some(reason) => {
                abstained += 1;
                *reason_counts.entry(reason.to_owned()).or_default() += 1;
            }
        }
    }
    let mut score = score_metrics(scored.iter().copied());
    let reason_counts = reason_counts
        .into_iter()
        .map(|(reason, count)| json!({"reason_code": reason, "count": count}))
        .collect::<Vec<_>>();
    if !score.is_object() {
        score = Value::Null;
    }
    json!({
        "suppressed": false,
        "case_count": rows.len(),
        "forecast_issued_count": issued,
        "abstained_count": abstained,
        "decision_coverage": issued as f64 / rows.len() as f64,
        "issued_unknown_or_immature_outcome_count": unknown_outcome,
        "issued_case_scores": score,
        "abstention_reason_counts": reason_counts
    })
}

fn semantic_errors(registration: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let provider = &registration["provider"];
    let policy = &registration["policy"];
    let origin = integer_at(policy, "forecast_origin_index").unwrap_or_default();
    let target_period = integer_at(policy, "target_period_index").unwrap_or_default();
    let feature_cutoff = integer_at(policy, "selection_features_cutoff_index").unwrap_or_default();
    if feature_cutoff >= origin {
        errors.push(
            "selection score features must be cut off strictly before forecast origin".to_owned(),
        );
    }
    if target_period <= origin {
        errors.push("target period must be later than forecast origin".to_owned());
    }
    if string_at(provider, "provider_id").is_none() {
        errors.push("provider_id is required".to_owned());
    }

    let Some(grid) = policy["threshold_grid"].as_array() else {
        return vec!["threshold_grid must be an array after schema validation".to_owned()];
    };
    let thresholds = grid.iter().filter_map(Value::as_f64).collect::<Vec<_>>();
    if thresholds.len() != grid.len() || thresholds.windows(2).any(|pair| pair[0] >= pair[1]) {
        errors.push("threshold_grid must be strictly increasing with no duplicates".to_owned());
    }
    let deployed_threshold = number_at(policy, "deployed_threshold").unwrap_or_default();
    if !thresholds
        .iter()
        .any(|value| (*value - deployed_threshold).abs() <= SCORE_EPSILON)
    {
        errors.push(
            "deployed_threshold must be an entry in the preregistered threshold_grid".to_owned(),
        );
    }

    if string_at(registration, "evidence_scope") == Some("authorized_historical") {
        for (field, label) in [
            ("forecast_artifact_sha256", "forecast artifact"),
            ("outcome_snapshot_sha256", "outcome snapshot"),
        ] {
            let digest = string_at(provider, field).unwrap_or_default();
            if digest.chars().all(|ch| ch == '0') || digest.chars().all(|ch| ch == '1') {
                errors.push(format!(
                    "authorized historical evaluation requires a non-placeholder {label} digest"
                ));
            }
        }
        let selection_digest = string_at(policy, "selection_model_sha256").unwrap_or_default();
        if selection_digest.chars().all(|ch| ch == '0')
            || selection_digest.chars().all(|ch| ch == '1')
        {
            errors.push(
                "authorized historical evaluation requires a non-placeholder selection model digest"
                    .to_owned(),
            );
        }
        let governance = &registration["governance"];
        if string_at(governance, "data_authority_reference").is_none_or(str::is_empty)
            || string_at(governance, "disclosure_review_reference").is_none_or(str::is_empty)
        {
            errors.push(
                "authorized historical evaluation requires non-empty authority and disclosure references"
                    .to_owned(),
            );
        }
    }

    let Some(rows) = registration.get("rows").and_then(Value::as_array) else {
        return vec!["rows must be an array after schema validation".to_owned()];
    };
    let mut case_ids = BTreeSet::new();
    for row in rows {
        let case_id = string_at(row, "case_id").unwrap_or_default();
        if !case_ids.insert(case_id.to_owned()) {
            errors.push(format!("case_id is duplicated: {case_id}"));
        }
        let reason = expected_reason(row, policy, deployed_threshold);
        let expected_disposition = if reason.is_some() {
            "abstained"
        } else {
            "forecast_issued"
        };
        if string_at(row, "policy_disposition") != Some(expected_disposition) {
            errors.push(format!(
                "case {case_id} records policy_disposition inconsistent with the pinned policy; expected {expected_disposition}"
            ));
        }
        let recorded_reason = string_at(row, "reason_code");
        if recorded_reason != reason {
            errors.push(format!(
                "case {case_id} records reason_code inconsistent with the pinned policy; expected {:?}",
                reason
            ));
        }

        match expected_disposition {
            "forecast_issued" => {
                let candidate = number_at(row, "offline_candidate_probability");
                let served = number_at(row, "served_probability");
                if candidate
                    .zip(served)
                    .is_none_or(|(candidate, served)| (candidate - served).abs() > SCORE_EPSILON)
                {
                    errors.push(format!(
                        "issued case {case_id} must serve its registered candidate probability"
                    ));
                }
            }
            _ => {
                if !row["served_probability"].is_null() {
                    errors.push(format!(
                        "abstained case {case_id} must not expose a served probability"
                    ));
                }
            }
        }

        if string_at(row, "outcome_status") == Some("known") {
            if !matches!(integer_at(row, "outcome"), Some(0 | 1)) {
                errors.push(format!("known outcome for case {case_id} must be 0 or 1"));
            }
        } else if !row["outcome"].is_null() {
            errors.push(format!(
                "unknown or immature outcome for case {case_id} must be null"
            ));
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
            "source commit must be a 40-character lowercase Git SHA or UNPINNED_LOCAL_RUN"
                .to_owned(),
        ]);
    }

    let registration: Value = serde_json::from_slice(input).map_err(|error| {
        vec![format!(
            "selective forecast registration JSON is invalid: {error}"
        )]
    })?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES).map_err(|error| {
        vec![format!(
            "embedded selective schema JSON is invalid: {error}"
        )]
    })?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )]
    })?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build selective schema validator: {error}")])?;
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

    let policy = &registration["policy"];
    let target_id = string_at(policy, "target_id").expect("schema validated target_id");
    let deployed_threshold =
        number_at(policy, "deployed_threshold").expect("schema validated deployed_threshold");
    let thresholds = policy["threshold_grid"]
        .as_array()
        .expect("schema validated threshold_grid")
        .iter()
        .map(|threshold| threshold.as_f64().expect("schema validated thresholds"))
        .collect::<Vec<_>>();
    let rows = registration["rows"]
        .as_array()
        .expect("schema validated rows")
        .iter()
        .collect::<Vec<_>>();

    let curve = thresholds
        .iter()
        .map(|threshold| selective_metrics(rows.iter().copied(), target_id, policy, *threshold))
        .collect::<Vec<_>>();
    let deployed = selective_metrics(rows.iter().copied(), target_id, policy, deployed_threshold);
    let candidate_reference = score_metrics(
        rows.iter()
            .copied()
            .filter(|row| matches_registered_target(row, target_id) && is_known_outcome(row)),
    );

    let mut group_rows: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    let mut regime_rows: BTreeMap<String, Vec<&Value>> = BTreeMap::new();
    let mut global_reason_counts = BTreeMap::<String, usize>::new();
    let mut unknown_outcome_count = 0usize;
    let mut immature_outcome_count = 0usize;
    let mut target_mismatch_count = 0usize;
    for row in &rows {
        group_rows
            .entry(
                string_at(row, "group_id")
                    .expect("schema validated group_id")
                    .to_owned(),
            )
            .or_default()
            .push(row);
        regime_rows
            .entry(
                string_at(row, "regime_status")
                    .expect("schema validated regime_status")
                    .to_owned(),
            )
            .or_default()
            .push(row);
        if string_at(row, "target_id") != Some(target_id) {
            target_mismatch_count += 1;
        }
        match string_at(row, "outcome_status").unwrap_or_default() {
            "unknown" => unknown_outcome_count += 1,
            "immature" => immature_outcome_count += 1,
            _ => {}
        }
        if let Some(reason) = expected_reason(row, policy, deployed_threshold) {
            *global_reason_counts.entry(reason.to_owned()).or_default() += 1;
        }
    }

    let minimum_group_count = integer_at(policy, "minimum_group_report_count")
        .expect("schema validated suppression threshold") as usize;
    let groups = group_rows
        .iter()
        .map(|(group_id, group)| {
            let mut summary = stratum_summary(
                group,
                policy,
                target_id,
                deployed_threshold,
                minimum_group_count,
            );
            if summary["suppressed"] == false {
                summary["group_id"] = json!(group_id);
            }
            summary
        })
        .collect::<Vec<_>>();
    let regimes = regime_rows
        .iter()
        .map(|(regime, group)| {
            let mut summary = stratum_summary(
                group,
                policy,
                target_id,
                deployed_threshold,
                minimum_group_count,
            );
            summary["stratum_type"] = json!("regime_status");
            if summary["suppressed"] == false {
                summary["regime_status"] = json!(regime);
            }
            summary
        })
        .collect::<Vec<_>>();
    let suppressed_group_count = groups
        .iter()
        .filter(|group| group["suppressed"] == true)
        .count();
    let suppressed_regime_strata_count = regimes
        .iter()
        .filter(|group| group["suppressed"] == true)
        .count();
    let global_reason_counts = global_reason_counts
        .into_iter()
        .map(|(reason, count)| json!({"reason_code": reason, "count": count}))
        .collect::<Vec<_>>();

    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let input_sha256 = sha256_hex(input);
    let registration_id =
        string_at(&registration, "registration_id").expect("schema validated registration_id");
    let receipt_commitment_sha256 = sha256_hex(
        format!(
            "pirb-selective-forecast-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "selective_forecast_risk_coverage_audit",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "provider_id": registration["provider"]["provider_id"],
        "model_version": registration["provider"]["model_version"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "forecast_artifact_sha256": registration["provider"]["forecast_artifact_sha256"],
        "outcome_snapshot_sha256": registration["provider"]["outcome_snapshot_sha256"],
        "selection_model_sha256": policy["selection_model_sha256"],
        "target_id": target_id,
        "forecast_origin_index": policy["forecast_origin_index"],
        "target_period_index": policy["target_period_index"],
        "deployed_threshold": deployed_threshold,
        "case_count": rows.len(),
        "mature_known_target_outcome_count": candidate_reference["scored_count"],
        "unknown_outcome_count": unknown_outcome_count,
        "immature_outcome_count": immature_outcome_count,
        "target_mismatch_count": target_mismatch_count,
        "offline_candidate_full_coverage_reference": candidate_reference,
        "deployed_policy": deployed,
        "pre_registered_risk_coverage_grid": curve,
        "aggregate_abstention_reason_counts": global_reason_counts,
        "group_summaries": groups,
        "group_suppressed_count": suppressed_group_count,
        "regime_summaries": regimes,
        "regime_strata_suppressed_count": suppressed_regime_strata_count,
        "claims": {
            "schema_and_policy_contract_validated": true,
            "risk_coverage_measured_on_supplied_outcomes": true,
            "distribution_shift_detection_quality_established": false,
            "group_fairness_established": false,
            "forecast_superiority_established": false,
            "decision_utility_established": false,
            "causal_impact_established": false,
            "abstention_means_no_event": false,
            "abstention_authorizes_withholding_aid": false,
            "live_decision_authority": false
        },
        "receipt_commitment_sha256": receipt_commitment_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-selective-forecast-audit --input REGISTRATION.json [--source-commit SHA]"
            .to_owned()
    })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read selective registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let receipt =
        validate_and_evaluate(&input, &source_commit).map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize selective audit receipt: {error}"))?
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
            .expect("checked-in synthetic selective forecast fixture is valid JSON")
    }

    fn evaluate(value: &Value) -> Result<Value, Vec<String>> {
        validate_and_evaluate(
            &serde_json::to_vec(value).expect("registration serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    #[test]
    fn fixture_evaluates_preregistered_risk_coverage_grid_and_claim_ceiling() {
        let receipt =
            validate_and_evaluate(FIXTURE_BYTES, "0123456789abcdef0123456789abcdef01234567")
                .expect("synthetic selective registration is valid");
        assert_eq!(receipt["case_count"], 12);
        assert_eq!(receipt["mature_known_target_outcome_count"], 9);
        assert_eq!(receipt["target_mismatch_count"], 1);
        assert_eq!(receipt["deployed_policy"]["forecast_issued_count"], 4);
        assert_eq!(receipt["deployed_policy"]["abstained_count"], 8);
        assert_eq!(
            receipt["deployed_policy"]["issued_case_scores"]["scored_count"],
            3
        );
        let deployed_brier = receipt["deployed_policy"]["issued_case_scores"]["brier_score"]
            .as_f64()
            .expect("scored issued rows have a Brier score");
        assert!((deployed_brier - 0.03).abs() < 1.0e-12);
        assert_eq!(
            receipt["pre_registered_risk_coverage_grid"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert_eq!(receipt["claims"]["abstention_means_no_event"], false);
    }

    #[test]
    fn abstained_rows_never_expose_a_served_candidate_probability() {
        let mut value = fixture();
        value["rows"][3]["served_probability"] = json!(0.4);
        let errors = evaluate(&value).expect_err("abstained candidate cannot be served");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("/rows/3/served_probability"))
        );
    }

    #[test]
    fn disposition_and_reason_must_match_the_registered_policy() {
        let mut value = fixture();
        value["rows"][4]["reason_code"] = json!("abstain_low_selection_score");
        let errors = evaluate(&value).expect_err("reason cannot contradict stale-input gate");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("reason_code inconsistent"))
        );
    }

    #[test]
    fn an_unknown_or_immature_outcome_is_never_imputed_to_no_event() {
        let mut value = fixture();
        value["rows"][7]["outcome"] = json!(0);
        let errors = evaluate(&value).expect_err("unknown outcome must stay null");
        assert!(errors.iter().any(|error| error.contains("/rows/7/outcome")));
    }

    #[test]
    fn target_mismatch_must_force_abstention_even_with_a_high_score() {
        let mut value = fixture();
        value["rows"][6]["policy_disposition"] = json!("forecast_issued");
        value["rows"][6]["reason_code"] = Value::Null;
        value["rows"][6]["served_probability"] = json!(0.7);
        let errors = evaluate(&value).expect_err("wrong estimand cannot be served");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("inconsistent with the pinned policy"))
        );
    }

    #[test]
    fn threshold_grid_must_be_preregistered_ordered_and_include_deployed_threshold() {
        let mut value = fixture();
        value["policy"]["threshold_grid"] = json!([0.0, 0.6, 0.4, 0.95]);
        let errors =
            evaluate(&value).expect_err("threshold selection must be deterministic and pinned");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("strictly increasing"))
        );
    }

    #[test]
    fn historical_scope_requires_authority_and_disclosure_approval() {
        let mut value = fixture();
        value["evidence_scope"] = json!("authorized_historical");
        let errors =
            evaluate(&value).expect_err("real historical rows require governance clearance");
        assert!(errors.iter().any(|error| error.contains("authorized")));
    }

    #[test]
    fn future_selection_features_are_rejected() {
        let mut value = fixture();
        value["policy"]["selection_features_cutoff_index"] = json!(10);
        let errors = evaluate(&value).expect_err("selection model features cannot use future data");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("strictly before forecast origin"))
        );
    }

    #[test]
    fn small_group_summaries_are_suppressed() {
        let mut value = fixture();
        value["policy"]["minimum_group_report_count"] = json!(7);
        let receipt = evaluate(&value).expect("valid privacy threshold yields a receipt");
        assert_eq!(receipt["group_suppressed_count"], 2);
        assert_eq!(receipt["regime_strata_suppressed_count"], 2);
        assert_eq!(receipt["group_summaries"][0]["group_id"], Value::Null);
    }

    #[test]
    fn score_formula_matches_binary_brier_and_log_loss_golden_values() {
        let value = fixture();
        let rows = value["rows"].as_array().unwrap();
        let summary = score_metrics(rows.iter().filter(|row| {
            matches_registered_target(row, "state-based-country-month-brd25")
                && is_known_outcome(row)
                && string_at(row, "case_id") == Some("case-001")
        }));
        assert_eq!(summary["scored_count"], 1);
        let brier = summary["brier_score"].as_f64().unwrap();
        assert!((brier - 0.04).abs() < 1.0e-12);
        let ll = summary["log_loss"].as_f64().unwrap();
        assert!((ll - 0.2231435513142097).abs() < 1.0e-12);
    }
}
