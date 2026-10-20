// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Scores finite univariate count samples against one frozen outcome snapshot.
// Coherence, CRPS, calibration, decision utility and causal impact are separate
// claims. This binary only emits descriptive raw-vs-reconciled marginal CRPS.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process,
};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-hierarchy-forecast-v0.2.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-hierarchy-sample-score-synthetic-v0.2.json");
const UNPINNED: &str = "UNPINNED_LOCAL_RUN";

#[derive(Debug, Clone)]
struct ScoreRuntime {
    dependency_lock_sha256: String,
    rustc_version: String,
    cargo_version: String,
}

impl ScoreRuntime {
    #[cfg(test)]
    fn unpinned() -> Self {
        Self {
            dependency_lock_sha256: UNPINNED.to_owned(),
            rustc_version: UNPINNED.to_owned(),
            cargo_version: UNPINNED.to_owned(),
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn canonical_json_sha256(value: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("cannot serialize canonical digest input: {error}"))?;
    Ok(sha256_hex(&bytes))
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

fn nonnegative_integer_at(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn validate_source_commit(source_commit: &str) -> Result<(), String> {
    if source_commit != UNPINNED
        && (source_commit.len() != 40
            || !source_commit
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()))
    {
        return Err("source commit must be a 40-character lowercase Git SHA or UNPINNED_LOCAL_RUN".to_owned());
    }
    Ok(())
}

fn validate_digest_or_unpinned(value: &str, label: &str) -> Result<(), String> {
    if value != UNPINNED
        && (value.len() != 64
            || !value
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()))
    {
        return Err(format!("{label} must be 64 lowercase hexadecimal characters or UNPINNED_LOCAL_RUN"));
    }
    Ok(())
}

/// Empirical CRPS for a finite sample distribution.
///
/// CRPS(F,y) = mean_i |x_i-y| - (1/(2m^2)) sum_ij |x_i-x_j|.
/// The pairwise term uses the sorted-sample identity in O(m log m), avoiding
/// a quadratic loop over user-controlled sample counts.
fn empirical_crps(samples: &[u64], observed: u64) -> Result<f64, String> {
    if samples.is_empty() {
        return Err("CRPS requires at least one predictive sample".to_owned());
    }
    let sample_count = samples.len() as f64;
    let observed = observed as f64;
    let mean_absolute_error = samples
        .iter()
        .map(|sample| (*sample as f64 - observed).abs())
        .sum::<f64>()
        / sample_count;

    let mut ordered = samples.to_vec();
    ordered.sort_unstable();
    let n = ordered.len() as f64;
    let pairwise_half_term = ordered
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let coefficient = 2.0 * index as f64 - n + 1.0;
            coefficient * *sample as f64
        })
        .sum::<f64>()
        / (n * n);

    let score = mean_absolute_error - pairwise_half_term;
    if !score.is_finite() || score < -1.0e-9 {
        return Err(format!("empirical CRPS produced an invalid value: {score}"));
    }
    Ok(score.max(0.0))
}

fn validate_registration(input: &[u8]) -> Result<Value, Vec<String>> {
    let registration: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("registration is not valid JSON: {error}")])?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES)
        .map_err(|error| vec![format!("embedded schema JSON is invalid: {error}")])?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!("Draft 2020-12 schema meta-validation failed: {error}")]
    })?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build scoring schema validator: {error}")])?;
    let mut errors = validator
        .iter_errors(&registration)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        errors.sort();
        return Err(errors);
    }

    let target_index = registration["target_period_index"].as_i64().unwrap_or_default();
    let origin_index = registration["forecast_origin_index"].as_i64().unwrap_or_default();
    let evaluation_as_of = registration["outcome_snapshot"]["evaluation_as_of_index"]
        .as_i64()
        .unwrap_or_default();
    if target_index <= origin_index {
        errors.push("target period must be later than forecast origin".to_owned());
    }
    if evaluation_as_of < target_index {
        errors.push("outcome evaluation-as-of index must be on or after target period".to_owned());
    }
    let fit_cutoff = registration["reconciliation_policy"]["parameter_fit_cutoff_index"]
        .as_i64()
        .unwrap_or_default();
    if fit_cutoff >= origin_index {
        errors.push("reconciliation parameter fit cutoff must be strictly before forecast origin".to_owned());
    }

    let forecasts = registration["forecasts"]
        .as_array()
        .expect("schema validation requires forecasts");
    let forecast_ids = forecasts
        .iter()
        .map(|forecast| {
            string_at(forecast, "forecast_id")
                .expect("schema validation requires forecast_id")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();
    if forecast_ids.len() != forecasts.len() {
        errors.push("forecast inventory contains duplicate forecast IDs".to_owned());
    }
    for forecast in forecasts {
        let forecast_id = string_at(forecast, "forecast_id").unwrap_or("<missing>");
        if forecast["period_index"].as_i64() != Some(target_index) {
            errors.push(format!(
                "forecast {forecast_id} period_index must equal registered target_period_index"
            ));
        }
    }

    let mut rows_by_draw: BTreeMap<String, BTreeMap<String, &Value>> = BTreeMap::new();
    for row in registration["predictive_samples"]["rows"]
        .as_array()
        .expect("schema validation requires predictive sample rows")
    {
        let draw_id = string_at(row, "draw_id").expect("schema validated draw_id");
        let forecast_id = string_at(row, "forecast_id").expect("schema validated forecast_id");
        if !forecast_ids.contains(forecast_id) {
            errors.push(format!(
                "predictive sample {draw_id} references unknown forecast {forecast_id}"
            ));
        }
        let draw_rows = rows_by_draw.entry(draw_id.to_owned()).or_default();
        if draw_rows.insert(forecast_id.to_owned(), row).is_some() {
            errors.push(format!(
                "predictive draw {draw_id} repeats forecast row {forecast_id}"
            ));
        }
    }
    if rows_by_draw.len() < 2 {
        errors.push("predictive sample matrix must contain at least two distinct draws".to_owned());
    }
    for (draw_id, rows) in &rows_by_draw {
        let actual = rows.keys().cloned().collect::<BTreeSet<_>>();
        if actual != forecast_ids {
            let missing = forecast_ids.difference(&actual).cloned().collect::<Vec<_>>();
            let unexpected = actual.difference(&forecast_ids).cloned().collect::<Vec<_>>();
            errors.push(format!(
                "predictive draw {draw_id} must contain exactly one row for every declared forecast; missing {missing:?}, unexpected {unexpected:?}"
            ));
        }
    }

    let mut outcomes_by_forecast: BTreeMap<String, &Value> = BTreeMap::new();
    let outcome_rows = registration["outcome_snapshot"]["rows"]
        .as_array()
        .expect("schema validation requires outcome rows");
    for row in outcome_rows {
        let forecast_id = string_at(row, "forecast_id").expect("schema validated outcome forecast_id");
        if !forecast_ids.contains(forecast_id) {
            errors.push(format!("outcome snapshot references unknown forecast {forecast_id}"));
        }
        if outcomes_by_forecast.insert(forecast_id.to_owned(), row).is_some() {
            errors.push(format!("outcome snapshot repeats forecast row {forecast_id}"));
        }
        let status = string_at(row, "status").expect("schema validated outcome status");
        let observed_count = row.get("observed_count").unwrap_or(&Value::Null);
        let available_index = row.get("outcome_available_index").unwrap_or(&Value::Null);
        match status {
            "matured" => {
                if nonnegative_integer_at(row, "observed_count").is_none() {
                    errors.push(format!(
                        "matured outcome for {forecast_id} requires a nonnegative integer observed_count"
                    ));
                }
                let availability = available_index.as_i64();
                if availability.is_none() {
                    errors.push(format!(
                        "matured outcome for {forecast_id} requires outcome_available_index"
                    ));
                } else if let Some(availability) = availability {
                    if availability <= target_index {
                        errors.push(format!(
                            "outcome for {forecast_id} must become available strictly after target period"
                        ));
                    }
                    if availability > evaluation_as_of {
                        errors.push(format!(
                            "outcome for {forecast_id} was not available by evaluation-as-of index"
                        ));
                    }
                }
            }
            "unknown" => {
                if !observed_count.is_null() || !available_index.is_null() {
                    errors.push(format!(
                        "unknown outcome for {forecast_id} must keep observed_count and outcome_available_index null"
                    ));
                }
            }
            _ => errors.push(format!("outcome for {forecast_id} has unsupported status")),
        }
    }
    for forecast_id in &forecast_ids {
        if !outcomes_by_forecast.contains_key(forecast_id) {
            errors.push(format!(
                "outcome snapshot is missing a row for forecast {forecast_id}"
            ));
        }
    }

    errors.sort();
    if errors.is_empty() {
        Ok(registration)
    } else {
        Err(errors)
    }
}

fn evaluate_with_runtime(
    input: &[u8],
    source_commit: &str,
    runtime: &ScoreRuntime,
) -> Result<Value, Vec<String>> {
    validate_source_commit(source_commit).map_err(|error| vec![error])?;
    validate_digest_or_unpinned(&runtime.dependency_lock_sha256, "dependency lock digest")
        .map_err(|error| vec![error])?;
    let registration = validate_registration(input)?;

    let forecasts = registration["forecasts"]
        .as_array()
        .expect("schema validation requires forecasts");
    let forecast_by_id = forecasts
        .iter()
        .map(|forecast| {
            (
                string_at(forecast, "forecast_id").expect("schema validated forecast_id"),
                forecast,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut sample_rows_by_draw: BTreeMap<String, BTreeMap<String, &Value>> = BTreeMap::new();
    for row in registration["predictive_samples"]["rows"]
        .as_array()
        .expect("schema validation requires sample rows")
    {
        let draw_id = string_at(row, "draw_id").expect("schema validated draw_id");
        let forecast_id = string_at(row, "forecast_id").expect("schema validated forecast_id");
        sample_rows_by_draw
            .entry(draw_id.to_owned())
            .or_default()
            .insert(forecast_id.to_owned(), row);
    }

    let outcomes_by_forecast = registration["outcome_snapshot"]["rows"]
        .as_array()
        .expect("schema validation requires outcome rows")
        .iter()
        .map(|row| {
            (
                string_at(row, "forecast_id").expect("schema validated outcome forecast_id"),
                row,
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut per_forecast_scores = Vec::new();
    let mut eligible_forecast_ids = Vec::new();
    let mut scored_count = 0usize;
    let mut unknown_outcome_count = 0usize;
    let mut raw_crps_total = 0.0_f64;
    let mut reconciled_crps_total = 0.0_f64;
    let mut improvement_total = 0.0_f64;

    for forecast in forecasts {
        let forecast_id = string_at(forecast, "forecast_id").expect("schema validated forecast_id");
        let outcome = outcomes_by_forecast
            .get(forecast_id)
            .copied()
            .expect("semantic validation requires every outcome row");
        let outcome_status = string_at(outcome, "status").expect("schema validated outcome status");
        if outcome_status == "unknown" {
            unknown_outcome_count += 1;
            per_forecast_scores.push(json!({
                "forecast_id": forecast_id,
                "outcome_status": "unknown",
                "observed_count": Value::Null,
                "outcome_available_index": Value::Null,
                "raw_empirical_crps": Value::Null,
                "reconciled_empirical_crps": Value::Null,
                "raw_minus_reconciled_crps": Value::Null,
                "score_status": "not_scored_unknown_outcome"
            }));
            continue;
        }

        let observed = nonnegative_integer_at(outcome, "observed_count")
            .expect("semantic validation requires mature integer outcomes");
        let mut raw_samples = Vec::with_capacity(sample_rows_by_draw.len());
        let mut reconciled_samples = Vec::with_capacity(sample_rows_by_draw.len());
        for (draw_id, draw_rows) in &sample_rows_by_draw {
            let row = draw_rows
                .get(forecast_id)
                .unwrap_or_else(|| panic!("semantic validation requires forecast {forecast_id} in draw {draw_id}"));
            raw_samples.push(
                nonnegative_integer_at(row, "raw_count")
                    .expect("schema validation requires integer raw sample counts"),
            );
            reconciled_samples.push(
                nonnegative_integer_at(row, "reconciled_count")
                    .expect("schema validation requires integer reconciled sample counts"),
            );
        }
        let raw_crps = empirical_crps(&raw_samples, observed)
            .map_err(|error| vec![format!("cannot score raw samples for {forecast_id}: {error}")])?;
        let reconciled_crps = empirical_crps(&reconciled_samples, observed)
            .map_err(|error| vec![format!("cannot score reconciled samples for {forecast_id}: {error}")])?;
        let improvement = raw_crps - reconciled_crps;
        eligible_forecast_ids.push(forecast_id.to_owned());
        raw_crps_total += raw_crps;
        reconciled_crps_total += reconciled_crps;
        improvement_total += improvement;
        scored_count += 1;
        per_forecast_scores.push(json!({
            "forecast_id": forecast_id,
            "outcome_status": "matured",
            "observed_count": observed,
            "outcome_available_index": outcome["outcome_available_index"],
            "raw_empirical_crps": raw_crps,
            "reconciled_empirical_crps": reconciled_crps,
            "raw_minus_reconciled_crps": improvement,
            "score_status": "scored"
        }));
    }

    let mean_raw_crps = if scored_count == 0 {
        Value::Null
    } else {
        json!(raw_crps_total / scored_count as f64)
    };
    let mean_reconciled_crps = if scored_count == 0 {
        Value::Null
    } else {
        json!(reconciled_crps_total / scored_count as f64)
    };
    let mean_improvement = if scored_count == 0 {
        Value::Null
    } else {
        json!(improvement_total / scored_count as f64)
    };

    let forecast_inventory = json!({
        "schema_version": registration["schema_version"],
        "provider": registration["provider"],
        "forecast_origin_index": registration["forecast_origin_index"],
        "target_period_index": registration["target_period_index"],
        "temporal_index_policy": registration["temporal_index_policy"],
        "reconciliation_policy": registration["reconciliation_policy"],
        "forecasts": registration["forecasts"],
        "aggregation_constraints": registration["aggregation_constraints"],
        "predictive_samples": registration["predictive_samples"]
    });
    let forecast_inventory_sha256 =
        canonical_json_sha256(&forecast_inventory).map_err(|error| vec![error])?;
    let outcome_snapshot_sha256 = canonical_json_sha256(&registration["outcome_snapshot"])
        .map_err(|error| vec![error])?;
    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let input_sha256 = sha256_hex(input);
    let registration_id =
        string_at(&registration, "registration_id").expect("schema validated registration_id");
    let receipt_commitment_sha256 = sha256_hex(
        format!(
            "pirb-hierarchy-sample-score-receipt-v0.2\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}\n{forecast_inventory_sha256}\n{outcome_snapshot_sha256}\n{}\n{}\n{}",
            runtime.dependency_lock_sha256,
            runtime.rustc_version,
            runtime.cargo_version
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.2",
        "receipt_type": "hierarchy_sample_score_audit",
        "registration_id": registration_id,
        "evidence_scope": registration["evidence_scope"],
        "provider": registration["provider"]["provider_id"],
        "forecast_release_id": registration["provider"]["forecast_release_id"],
        "outcome_release_id": registration["outcome_snapshot"]["release_id"],
        "outcome_release_sha256": registration["outcome_snapshot"]["release_sha256"],
        "outcome_evaluation_as_of_index": registration["outcome_snapshot"]["evaluation_as_of_index"],
        "forecast_origin_index": registration["forecast_origin_index"],
        "target_period_index": registration["target_period_index"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != UNPINNED,
        "dependency_lock_sha256": runtime.dependency_lock_sha256,
        "rustc_version": runtime.rustc_version,
        "cargo_version": runtime.cargo_version,
        "schema_sha256": schema_sha256,
        "input_sha256": input_sha256,
        "forecast_inventory_sha256": forecast_inventory_sha256,
        "outcome_snapshot_sha256": outcome_snapshot_sha256,
        "score": "empirical_crps",
        "score_orientation": "lower_is_better",
        "score_interpretation": "marginal predictive-distribution score; not a joint dependence score",
        "forecast_count": forecasts.len(),
        "predictive_sample_count": sample_rows_by_draw.len(),
        "matured_scored_count": scored_count,
        "score_denominator": scored_count,
        "eligible_forecast_ids": eligible_forecast_ids,
        "unknown_outcome_count": unknown_outcome_count,
        "mean_raw_empirical_crps": mean_raw_crps,
        "mean_reconciled_empirical_crps": mean_reconciled_crps,
        "mean_raw_minus_reconciled_crps": mean_improvement,
        "per_forecast_scores": per_forecast_scores,
        "claims": {
            "contract_validated": true,
            "marginal_predictive_score_calculated": true,
            "forecast_skill_superiority_established": false,
            "calibration_established": false,
            "decision_utility_established": false,
            "causal_impact_established": false,
            "live_decision_authority": false
        },
        "receipt_commitment_sha256": receipt_commitment_sha256
    }))
}

#[cfg(test)]
fn evaluate(input: &[u8]) -> Result<Value, Vec<String>> {
    evaluate_with_runtime(
        input,
        "0123456789abcdef0123456789abcdef01234567",
        &ScoreRuntime::unpinned(),
    )
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-hierarchy-sample-score --input REGISTRATION.json [--source-commit SHA] [--dependency-lock-sha256 SHA256] [--rustc-version VERSION] [--cargo-version VERSION]"
            .to_owned()
    })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read scoring registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| UNPINNED.to_owned());
    let runtime = ScoreRuntime {
        dependency_lock_sha256: option_value(&args, "--dependency-lock-sha256")?
            .unwrap_or_else(|| UNPINNED.to_owned()),
        rustc_version: option_value(&args, "--rustc-version")?
            .unwrap_or_else(|| UNPINNED.to_owned()),
        cargo_version: option_value(&args, "--cargo-version")?
            .unwrap_or_else(|| UNPINNED.to_owned()),
    };
    validate_source_commit(&source_commit)?;
    validate_digest_or_unpinned(&runtime.dependency_lock_sha256, "dependency lock digest")?;
    let receipt = evaluate_with_runtime(&input, &source_commit, &runtime)
        .map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize CRPS receipt: {error}"))?
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
        serde_json::from_slice(FIXTURE_BYTES).expect("synthetic v0.2 score fixture is valid JSON")
    }

    fn audit(value: &Value) -> Result<Value, Vec<String>> {
        evaluate(&serde_json::to_vec(value).expect("test registration serializes"))
    }

    fn crps_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-12,
            "expected {expected:.16}, got {actual:.16}"
        );
    }

    fn independent_quadratic_crps(samples: &[u64], observed: u64) -> f64 {
        let sample_count = samples.len() as f64;
        let first = samples
            .iter()
            .map(|sample| (*sample as f64 - observed as f64).abs())
            .sum::<f64>()
            / sample_count;
        let mut pairwise = 0.0;
        for left in samples {
            for right in samples {
                pairwise += (*left as f64 - *right as f64).abs();
            }
        }
        first - pairwise / (2.0 * sample_count * sample_count)
    }

    #[test]
    fn crps_golden_cases_match_independent_quadratic_oracle() {
        let cases: Vec<(Vec<u64>, u64, f64)> = vec![
            (vec![2, 2], 2, 0.0),
            (vec![2, 2], 3, 1.0),
            (vec![0, 2], 1, 0.5),
            (vec![1, 1, 3], 2, 5.0 / 9.0),
            (vec![1, 4, 7], 5, 1.0),
        ];
        for (samples, observed, expected) in cases {
            let actual = empirical_crps(&samples, observed).expect("valid CRPS case");
            crps_close(actual, expected);
            crps_close(actual, independent_quadratic_crps(&samples, observed));
        }
    }

    #[test]
    fn synthetic_fixture_scores_same_mature_rows_for_raw_and_reconciled_samples() {
        let receipt = evaluate(FIXTURE_BYTES).expect("v0.2 synthetic outcome snapshot is valid");
        assert_eq!(receipt["forecast_count"], 8);
        assert_eq!(receipt["predictive_sample_count"], 2);
        assert_eq!(receipt["matured_scored_count"], 7);
        assert_eq!(receipt["unknown_outcome_count"], 1);
        crps_close(
            receipt["mean_raw_empirical_crps"].as_f64().unwrap(),
            2.0 / 7.0,
        );
        crps_close(
            receipt["mean_reconciled_empirical_crps"].as_f64().unwrap(),
            1.5 / 7.0,
        );
        crps_close(
            receipt["mean_raw_minus_reconciled_crps"].as_f64().unwrap(),
            0.5 / 7.0,
        );
        assert_eq!(
            receipt["claims"]["forecast_skill_superiority_established"],
            false
        );
        assert_eq!(receipt["claims"]["calibration_established"], false);
    }

    #[test]
    fn unknown_outcomes_are_null_and_never_scored_as_zero() {
        let receipt = evaluate(FIXTURE_BYTES).expect("valid synthetic score fixture");
        let unknown = receipt["per_forecast_scores"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["forecast_id"] == "fc-admin1-c-organized")
            .expect("unknown fixture outcome row");
        assert_eq!(unknown["score_status"], "not_scored_unknown_outcome");
        assert!(unknown["raw_empirical_crps"].is_null());
        assert!(unknown["reconciled_empirical_crps"].is_null());
        assert_eq!(unknown["observed_count"], Value::Null);
    }

    #[test]
    fn outcome_only_revision_preserves_forecast_inventory_commitment() {
        let original = evaluate(FIXTURE_BYTES).expect("valid fixture");
        let mut changed = fixture();
        changed["outcome_snapshot"]["rows"][0]["observed_count"] = json!(16);
        let changed_bytes = serde_json::to_vec(&changed).expect("changed fixture serializes");
        let revised = evaluate(&changed_bytes).expect("revised synthetic outcome snapshot remains valid");
        assert_eq!(
            original["forecast_inventory_sha256"],
            revised["forecast_inventory_sha256"]
        );
        assert_ne!(
            original["outcome_snapshot_sha256"],
            revised["outcome_snapshot_sha256"]
        );
        assert_ne!(original["input_sha256"], revised["input_sha256"]);
        assert_ne!(
            original["receipt_commitment_sha256"],
            revised["receipt_commitment_sha256"]
        );
    }

    #[test]
    fn immature_and_future_outcomes_are_rejected() {
        let mut early = fixture();
        early["outcome_snapshot"]["rows"][0]["outcome_available_index"] = json!(11);
        let errors = audit(&early).expect_err("outcome cannot predate publication after target");
        assert!(errors.iter().any(|error| error.contains("strictly after target period")));

        let mut future = fixture();
        future["outcome_snapshot"]["rows"][0]["outcome_available_index"] = json!(13);
        let errors = audit(&future).expect_err("outcome cannot postdate evaluation as-of");
        assert!(errors.iter().any(|error| error.contains("not available by evaluation-as-of")));
    }

    #[test]
    fn missing_and_duplicate_outcome_rows_are_rejected() {
        let mut missing = fixture();
        missing["outcome_snapshot"]["rows"]
            .as_array_mut()
            .unwrap()
            .pop();
        let errors = audit(&missing).expect_err("all forecast IDs require an explicit outcome row");
        assert!(errors.iter().any(|error| error.contains("is missing a row for forecast")));

        let mut duplicate = fixture();
        let row = duplicate["outcome_snapshot"]["rows"][0].clone();
        duplicate["outcome_snapshot"]["rows"]
            .as_array_mut()
            .unwrap()
            .push(row);
        let errors = audit(&duplicate).expect_err("duplicate outcome labels are ambiguous");
        assert!(errors.iter().any(|error| error.contains("repeats forecast row")));
    }

    #[test]
    fn inconsistent_unknown_and_mature_outcome_state_is_rejected() {
        let mut value = fixture();
        value["outcome_snapshot"]["rows"][7]["observed_count"] = json!(0);
        let errors = audit(&value).expect_err("unknown labels cannot use zero as a placeholder");
        assert!(errors.iter().any(|error| error.contains("must keep observed_count")));
    }

    #[test]
    fn receipt_binds_runtime_provenance() {
        let runtime = ScoreRuntime {
            dependency_lock_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            rustc_version: "rustc 1.96.0".to_owned(),
            cargo_version: "cargo 1.96.0".to_owned(),
        };
        let receipt = evaluate_with_runtime(
            FIXTURE_BYTES,
            "0123456789abcdef0123456789abcdef01234567",
            &runtime,
        )
        .expect("valid pinned score fixture");
        assert_eq!(
            receipt["dependency_lock_sha256"],
            runtime.dependency_lock_sha256
        );
        assert_eq!(receipt["rustc_version"], runtime.rustc_version);
        assert_eq!(receipt["cargo_version"], runtime.cargo_version);
        let changed = ScoreRuntime {
            dependency_lock_sha256:
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
            ..runtime
        };
        let altered = evaluate_with_runtime(
            FIXTURE_BYTES,
            "0123456789abcdef0123456789abcdef01234567",
            &changed,
        )
        .expect("valid separately pinned provenance");
        assert_ne!(receipt["receipt_commitment_sha256"], altered["receipt_commitment_sha256"]);
    }
}
