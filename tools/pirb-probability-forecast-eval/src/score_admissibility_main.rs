// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Determines whether a registered score is admissible for the declared
// forecast representation and comparison. It does not calculate scores.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{env, fs, process};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-score-admissibility-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] = include_bytes!(
    "../../../docs/fixtures/pirb-score-admissibility-log-score-reconciliation-synthetic-v0.1.json"
);

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

fn is_placeholder_digest(digest: &str) -> bool {
    digest
        .chars()
        .next()
        .is_some_and(|first| digest.chars().all(|ch| ch == first))
}

fn representation_and_outcome_match(registration: &Value) -> bool {
    let representation = string_at(registration, "prediction_representation").unwrap_or_default();
    let outcome = string_at(registration, "outcome_type").unwrap_or_default();
    let score = string_at(&registration["score"], "score_name").unwrap_or_default();
    matches!(
        (representation, outcome, score),
        ("binary_probability", "binary_outcome", "brier_score")
            | ("binary_probability", "binary_outcome", "log_score")
            | ("quantile_forecast", "scalar_outcome", "pinball_loss")
            | ("quantile_forecast", "scalar_outcome", "interval_score")
            | (
                "multivariate_joint_samples",
                "multivariate_outcome",
                "energy_score"
            )
            | (
                "multivariate_joint_samples",
                "multivariate_outcome",
                "variogram_score"
            )
    )
}

fn orientation_matches(registration: &Value) -> bool {
    let score = string_at(&registration["score"], "score_name").unwrap_or_default();
    let orientation = string_at(&registration["score"], "score_orientation").unwrap_or_default();
    matches!(
        (score, orientation),
        ("brier_score", "lower_is_better")
            | ("pinball_loss", "lower_is_better")
            | ("interval_score", "lower_is_better")
            | ("energy_score", "lower_is_better")
            | ("variogram_score", "lower_is_better")
            | ("log_score", "higher_is_better")
    )
}

fn admissibility_reasons(registration: &Value) -> Vec<&'static str> {
    let mut reasons = Vec::new();
    let score_name = string_at(&registration["score"], "score_name").unwrap_or_default();
    let representation = string_at(registration, "prediction_representation").unwrap_or_default();
    let comparison = string_at(registration, "comparison_type").unwrap_or_default();
    let evidence_scope = string_at(registration, "evidence_scope").unwrap_or_default();

    if !representation_and_outcome_match(registration) {
        reasons.push("SCORE_REPRESENTATION_OUTCOME_MISMATCH");
    }
    if !orientation_matches(registration) {
        reasons.push("SCORE_ORIENTATION_MISMATCH");
    }
    if comparison == "reconciled_vs_unreconciled_distribution" && score_name == "log_score" {
        reasons.push("LOG_SCORE_NOT_ADMISSIBLE_FOR_RECONCILED_VS_UNRECONCILED_DISTRIBUTIONS");
    }
    if comparison == "reconciled_vs_unreconciled_distribution"
        && representation != "multivariate_joint_samples"
    {
        reasons.push("RECONCILIATION_COMPARISON_REQUIRES_MULTIVARIATE_JOINT_SAMPLES");
    }

    if evidence_scope == "authorized_historical" {
        let policy = &registration["common_evaluation_policy"];
        if policy["common_rows_verified"].as_bool() != Some(true) {
            reasons.push("COMMON_EVALUATION_ROWS_NOT_VERIFIED");
        }
        if policy["heldout_outcomes"].as_bool() != Some(true) {
            reasons.push("OUTCOMES_NOT_HELD_OUT");
        }
        if string_at(policy, "outcome_maturity_status") != Some("mature_frozen_vintage") {
            reasons.push("OUTCOME_VINTAGE_NOT_MATURE_AND_FROZEN");
        }
        if string_at(policy, "outcome_release_sha256").is_none_or(is_placeholder_digest) {
            reasons.push("OUTCOME_RELEASE_DIGEST_NOT_PINNED");
        }
        if string_at(&registration["score"], "formula_sha256").is_none_or(is_placeholder_digest) {
            reasons.push("SCORE_FORMULA_DIGEST_NOT_PINNED");
        }
        if string_at(&registration["score"], "implementation_sha256")
            .is_none_or(is_placeholder_digest)
        {
            reasons.push("SCORE_IMPLEMENTATION_DIGEST_NOT_PINNED");
        }
        if string_at(&registration["governance"], "data_authority_status") != Some("authorized") {
            reasons.push("DATA_AUTHORITY_NOT_VERIFIED");
        }
        if string_at(&registration["governance"], "disclosure_review_status") != Some("approved") {
            reasons.push("DISCLOSURE_REVIEW_NOT_APPROVED");
        }
    }
    reasons.sort_unstable();
    reasons
}

fn validate_contract(input: &[u8], source_commit: &str) -> Result<Value, Vec<String>> {
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
            "score-admissibility registration JSON is invalid: {error}"
        )]
    })?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES).map_err(|error| {
        vec![format!(
            "embedded score-admissibility schema JSON is invalid: {error}"
        )]
    })?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )]
    })?;
    let validator = jsonschema::draft202012::new(&schema).map_err(|error| {
        vec![format!(
            "cannot build score-admissibility schema validator: {error}"
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

    let reasons = admissibility_reasons(&registration);
    let admissible = reasons.is_empty();
    let evidence_scope =
        string_at(&registration, "evidence_scope").expect("schema validated evidence_scope");
    let comparison_status = if admissible {
        "admissible_for_score_computation"
    } else {
        "not_admissible"
    };
    let evaluation_readiness = if evidence_scope == "synthetic" {
        "synthetic_contract_test_only"
    } else if admissible {
        "eligible_for_independent_scoring_review"
    } else {
        "blocked"
    };
    let registration_id =
        string_at(&registration, "registration_id").expect("schema validated registration_id");
    let input_sha256 = sha256_hex(input);
    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let source_commit_sha256 = sha256_hex(
        format!(
            "pirb-score-admissibility-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}\n{comparison_status}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "score_admissibility",
        "registration_id": registration_id,
        "evidence_scope": evidence_scope,
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "forecast_estimand_id": registration["forecast_estimand_id"],
        "prediction_representation": registration["prediction_representation"],
        "outcome_type": registration["outcome_type"],
        "comparison_type": registration["comparison_type"],
        "score_name": registration["score"]["score_name"],
        "score_orientation": registration["score"]["score_orientation"],
        "comparison_status": comparison_status,
        "evaluation_readiness": evaluation_readiness,
        "reason_codes": reasons,
        "score_calculated": false,
        "ranking_permitted": false,
        "comparison_superiority_established": false,
        "decision_value_established": false,
        "causal_impact_established": false,
        "live_decision_authority": false,
        "claim_ceiling": "metric_admissibility_only",
        "receipt_commitment_sha256": source_commit_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-score-admissibility --input REGISTRATION.json [--source-commit SHA]".to_owned()
    })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read score registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let receipt = validate_contract(&input, &source_commit).map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize score-admissibility receipt: {error}"))?
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
            .expect("synthetic log-score admissibility fixture is valid JSON")
    }

    fn validate(value: &Value) -> Result<Value, Vec<String>> {
        validate_contract(
            &serde_json::to_vec(value).expect("registration serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    #[test]
    fn log_score_is_rejected_for_reconciled_vs_unreconciled_distributions() {
        let receipt =
            validate(&fixture()).expect("negative control is a structurally valid registration");
        assert_eq!(receipt["comparison_status"], "not_admissible");
        assert_eq!(receipt["score_name"], "log_score");
        assert!(
            receipt["reason_codes"]
                .as_array()
                .unwrap()
                .contains(&json!("LOG_SCORE_NOT_ADMISSIBLE_FOR_RECONCILED_VS_UNRECONCILED_DISTRIBUTIONS"))
        );
        assert_eq!(receipt["score_calculated"], false);
        assert_eq!(receipt["ranking_permitted"], false);
        assert_eq!(receipt["comparison_superiority_established"], false);
        assert_eq!(receipt["causal_impact_established"], false);
    }

    #[test]
    fn brier_score_is_admissible_for_binary_probability_verification_but_not_scored() {
        let mut value = fixture();
        value["prediction_representation"] = json!("binary_probability");
        value["outcome_type"] = json!("binary_outcome");
        value["comparison_type"] = json!("single_system_verification");
        value["score"]["score_name"] = json!("brier_score");
        value["score"]["score_orientation"] = json!("lower_is_better");
        let receipt = validate(&value).expect("compatible synthetic score registration is valid");
        assert_eq!(
            receipt["comparison_status"],
            "admissible_for_score_computation"
        );
        assert_eq!(
            receipt["evaluation_readiness"],
            "synthetic_contract_test_only"
        );
        assert_eq!(receipt["score_calculated"], false);
        assert_eq!(receipt["ranking_permitted"], false);
    }

    #[test]
    fn energy_score_is_admissible_for_joint_sample_reconciliation_comparison() {
        let mut value = fixture();
        value["score"]["score_name"] = json!("energy_score");
        value["score"]["score_orientation"] = json!("lower_is_better");
        let receipt = validate(&value).expect("joint energy score registration is compatible");
        assert_eq!(
            receipt["comparison_status"],
            "admissible_for_score_computation"
        );
        assert_eq!(receipt["score_calculated"], false);
        assert_eq!(receipt["ranking_permitted"], false);
    }

    #[test]
    fn binary_score_cannot_be_registered_for_multivariate_samples() {
        let mut value = fixture();
        value["score"]["score_name"] = json!("brier_score");
        value["score"]["score_orientation"] = json!("lower_is_better");
        let receipt = validate(&value).expect("unsupported pair is reported, not executed");
        assert_eq!(receipt["comparison_status"], "not_admissible");
        assert!(
            receipt["reason_codes"]
                .as_array()
                .unwrap()
                .contains(&json!("SCORE_REPRESENTATION_OUTCOME_MISMATCH"))
        );
    }

    #[test]
    fn score_orientation_must_match_registered_metric() {
        let mut value = fixture();
        value["score"]["score_name"] = json!("energy_score");
        value["score"]["score_orientation"] = json!("higher_is_better");
        let receipt = validate(&value).expect("wrong orientation must be reported");
        assert_eq!(receipt["comparison_status"], "not_admissible");
        assert!(
            receipt["reason_codes"]
                .as_array()
                .unwrap()
                .contains(&json!("SCORE_ORIENTATION_MISMATCH"))
        );
    }

    #[test]
    fn historical_ranking_requires_mature_authorized_common_heldout_rows_and_pins() {
        let mut value = fixture();
        value["evidence_scope"] = json!("authorized_historical");
        value["score"]["score_name"] = json!("energy_score");
        value["score"]["score_orientation"] = json!("lower_is_better");
        let receipt = validate(&value).expect("missing history prerequisites should be reported");
        assert_eq!(receipt["comparison_status"], "not_admissible");
        let reasons = receipt["reason_codes"].as_array().unwrap();
        assert!(reasons.contains(&json!("OUTCOME_VINTAGE_NOT_MATURE_AND_FROZEN")));
        assert!(reasons.contains(&json!("DATA_AUTHORITY_NOT_VERIFIED")));
        assert_eq!(receipt["ranking_permitted"], false);
    }

    #[test]
    fn user_input_cannot_claim_score_or_superiority_already_established() {
        let mut value = fixture();
        value["claims"]["score_calculated"] = json!(true);
        let errors = validate(&value).expect_err("schema must reject self-asserted score results");
        assert!(!errors.is_empty());
    }
}
