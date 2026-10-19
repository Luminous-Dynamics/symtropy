// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Validates the cross-provider benchmark registration contract. This tool does
// not load provider forecast rows or calculate a forecast/impact score.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-cross-provider-benchmark-v0.1.schema.json");
#[cfg(test)]
const EXAMPLE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/pirb-cross-provider-benchmark-incompatible-v0.1.json");

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

fn object_string_at<'a>(value: &'a Value, object: &str, key: &str) -> Option<&'a str> {
    value.get(object).and_then(|child| string_at(child, key))
}

fn output_type_matches_measure(output_type: &str, target_measure: &str) -> bool {
    matches!(
        (output_type, target_measure),
        ("binary_probability", "binary_event_probability")
            | ("event_count", "event_count")
            | ("count_distribution", "event_count_distribution")
            | ("prediction_interval", "event_interval")
            | ("ordinal_risk", "ordinal_risk")
            | ("other", "other")
    )
}

fn push_unique_id(
    ids: &mut BTreeSet<String>,
    value: Option<&str>,
    label: &str,
    errors: &mut Vec<String>,
) {
    match value {
        Some(id) if ids.insert(id.to_owned()) => {}
        Some(id) => errors.push(format!("{label} ID is duplicated: {id}")),
        None => errors.push(format!("{label} has no string ID")),
    }
}

fn semantic_errors(registration: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(providers) = registration.get("providers").and_then(Value::as_array) else {
        return vec!["providers must be an array after schema validation".to_owned()];
    };
    let Some(comparisons) = registration.get("comparisons").and_then(Value::as_array) else {
        return vec!["comparisons must be an array after schema validation".to_owned()];
    };

    let mut provider_ids = BTreeSet::new();
    let mut provider_by_id = BTreeMap::new();
    for provider in providers {
        let id = string_at(provider, "provider_id");
        push_unique_id(&mut provider_ids, id, "provider", &mut errors);
        if let Some(id) = id {
            provider_by_id.insert(id.to_owned(), provider);
        }

        let output_type = string_at(provider, "output_type").unwrap_or_default();
        let metrics = registration
            .get("preregistration")
            .and_then(|v| v.get("primary_metrics_by_output_type"))
            .and_then(|v| v.get(output_type))
            .and_then(Value::as_array);

        if metrics.is_none() {
            errors.push(format!(
                "provider {} has no pre-registered metric family for output_type {output_type}",
                id.unwrap_or("<missing>")
            ));
        }

        if output_type != "binary_probability"
            && let Some(metrics) = metrics
        {
                for metric in metrics.iter().filter_map(Value::as_str) {
                    let normalized = metric.to_ascii_lowercase();
                    let probability_only = [
                        "brier score",
                        "brier skill",
                        "log loss",
                        "log-loss",
                        "roc-auc",
                        "roc auc",
                        "average precision",
                        "expected calibration error",
                        "ece",
                    ]
                    .iter()
                    .any(|needle| normalized.contains(needle));
                    if probability_only {
                        errors.push(format!(
                            "provider {} declares output_type {output_type} but registers binary-probability metric {metric}",
                            id.unwrap_or("<missing>")
                        ));
                    }
                }
        }

        if object_string_at(registration, "governance", "data_authority_status")
            == Some("authorized")
            && provider
                .get("independent_source_capture_verified")
                .and_then(Value::as_bool)
                != Some(true)
        {
            errors.push(format!(
                "provider {} lacks independent source-capture verification for authorized historical evidence",
                id.unwrap_or("<missing>")
            ));
        }

        let digest = string_at(provider, "artifact_digest").unwrap_or_default();
        if (digest.chars().all(|ch| ch == '0') || digest.chars().all(|ch| ch == '1'))
            && string_at(registration, "evidence_scope") == Some("authorized_historical_evaluation")
        {
                errors.push(format!(
                    "provider {} uses a placeholder artifact digest for historical evidence",
                    id.unwrap_or("<missing>")
                ));
        }
    }

    let evidence_scope = string_at(registration, "evidence_scope").unwrap_or_default();
    let governance = registration
        .get("governance")
        .expect("schema validation requires governance");
    if evidence_scope == "authorized_historical_evaluation" {
        if string_at(governance, "data_authority_status") != Some("authorized") {
            errors.push(
                "authorized_historical_evaluation requires data_authority_status=authorized"
                    .to_owned(),
            );
        }
        if string_at(governance, "disclosure_review_status") != Some("approved") {
            errors.push(
                "authorized_historical_evaluation requires disclosure_review_status=approved"
                    .to_owned(),
            );
        }
        if string_at(governance, "data_authority_reference").is_none_or(str::is_empty) {
            errors.push(
                "authorized historical evidence requires data_authority_reference".to_owned(),
            );
        }
        if string_at(governance, "disclosure_review_reference").is_none_or(str::is_empty) {
            errors.push(
                "authorized historical evidence requires disclosure_review_reference".to_owned(),
            );
        }
        if registration
            .get("outcome")
            .and_then(|v| v.get("release_sha256"))
            .is_none_or(Value::is_null)
        {
            errors.push(
                "authorized historical evidence requires a frozen outcome release digest"
                    .to_owned(),
            );
        }
        for provider in providers {
            if provider
                .get("independent_source_capture_verified")
                .and_then(Value::as_bool)
                != Some(true)
            {
                errors.push(format!(
                    "provider {} must have independently verified source capture",
                    string_at(provider, "provider_id").unwrap_or("<missing>")
                ));
            }
        }
    }

    let mut comparison_ids = BTreeSet::new();
    for comparison in comparisons {
        let comparison_id = string_at(comparison, "comparison_id");
        push_unique_id(
            &mut comparison_ids,
            comparison_id,
            "comparison",
            &mut errors,
        );

        let left_id = string_at(comparison, "left_provider_id");
        let right_id = string_at(comparison, "right_provider_id");
        let left = left_id.and_then(|id| provider_by_id.get(id).copied());
        let right = right_id.and_then(|id| provider_by_id.get(id).copied());

        if left.is_none() {
            errors.push(format!(
                "comparison {} references unknown left provider {:?}",
                comparison_id.unwrap_or("<missing>"),
                left_id
            ));
        }
        if right.is_none() {
            errors.push(format!(
                "comparison {} references unknown right provider {:?}",
                comparison_id.unwrap_or("<missing>"),
                right_id
            ));
        }
        if left_id.is_some() && left_id == right_id {
            errors.push(format!(
                "comparison {} must reference two different providers",
                comparison_id.unwrap_or("<missing>")
            ));
        }

        match string_at(comparison, "status").unwrap_or_default() {
            "not_comparable" => {
                if comparison
                    .get("forecast_skill_comparison_permitted")
                    .and_then(Value::as_bool)
                    != Some(false)
                    || comparison
                        .get("decision_value_comparison_permitted")
                        .and_then(Value::as_bool)
                        != Some(false)
                {
                    errors.push(format!(
                        "not-comparable pair {} must disable both scoring lanes",
                        comparison_id.unwrap_or("<missing>")
                    ));
                }
            }
            "directly_comparable" => {
                for field in [
                    "original_target",
                    "event_ontology",
                    "analysis_unit",
                    "temporal_aggregation",
                    "forecast_horizon",
                    "output_type",
                ] {
                    if left.and_then(|provider| string_at(provider, field))
                        != right.and_then(|provider| string_at(provider, field))
                    {
                        errors.push(format!(
                            "direct comparison {} has mismatched provider field {field}; use a reviewed mapping or mark not_comparable",
                            comparison_id.unwrap_or("<missing>")
                        ));
                    }
                }

                if let (Some(left), Some(right)) = (left, right) {
                    let shared_id =
                        object_string_at(registration, "shared_estimand", "estimand_id");
                    if string_at(left, "canonical_estimand_id") != shared_id
                        || string_at(right, "canonical_estimand_id") != shared_id
                    {
                        errors.push(format!(
                            "direct comparison {} requires both providers to use the registered canonical estimand ID",
                            comparison_id.unwrap_or("<missing>")
                        ));
                    }

                    if string_at(left, "analysis_unit")
                        != object_string_at(registration, "shared_estimand", "analysis_unit")
                        || string_at(right, "analysis_unit")
                            != object_string_at(registration, "shared_estimand", "analysis_unit")
                    {
                        errors.push(format!(
                            "direct comparison {} does not match the registered shared analysis unit",
                            comparison_id.unwrap_or("<missing>")
                        ));
                    }
                    if string_at(left, "temporal_aggregation")
                        != object_string_at(registration, "shared_estimand", "temporal_aggregation")
                        || string_at(right, "temporal_aggregation")
                            != object_string_at(
                                registration,
                                "shared_estimand",
                                "temporal_aggregation",
                            )
                    {
                        errors.push(format!(
                            "direct comparison {} does not match the registered shared temporal aggregation",
                            comparison_id.unwrap_or("<missing>")
                        ));
                    }

                    let target_measure =
                        object_string_at(registration, "shared_estimand", "target_measure")
                            .unwrap_or_default();
                    for provider in [left, right] {
                        let output_type = string_at(provider, "output_type").unwrap_or_default();
                        if !output_type_matches_measure(output_type, target_measure) {
                            errors.push(format!(
                                "direct comparison {} output_type {output_type} is incompatible with registered target_measure {target_measure}",
                                comparison_id.unwrap_or("<missing>")
                            ));
                        }
                    }
                }
            }
            "comparable_after_predeclared_mapping" => {
                let mapping_pinned =
                    object_string_at(registration, "shared_estimand", "mapping_policy_reference")
                        .is_some_and(|reference| !reference.is_empty())
                        && object_string_at(
                            registration,
                            "shared_estimand",
                            "mapping_policy_sha256",
                        )
                        .is_some_and(|digest| {
                            digest.len() == 64
                                && digest
                                    .chars()
                                    .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
                        });
                let mapping = comparison.get("mapping").unwrap_or(&Value::Null);
                let mapping_verified = mapping
                    .get("independently_verified")
                    .and_then(Value::as_bool)
                    == Some(true);
                let review_reference = string_at(mapping, "review_reference")
                    .is_some_and(|reference| !reference.is_empty());
                let row_policy = string_at(comparison, "common_row_policy")
                    .is_some_and(|policy| !policy.is_empty());
                let mapping_targets_registered_estimand = string_at(mapping, "target_estimand_id")
                    == object_string_at(registration, "shared_estimand", "estimand_id");
                let mapping_sources_match_providers =
                    left.zip(right).is_some_and(|(left, right)| {
                        string_at(mapping, "left_source_estimand_id")
                            == string_at(left, "canonical_estimand_id")
                            && string_at(mapping, "right_source_estimand_id")
                                == string_at(right, "canonical_estimand_id")
                    });
                if !(mapping_pinned
                    && mapping_verified
                    && review_reference
                    && row_policy
                    && mapping_targets_registered_estimand
                    && mapping_sources_match_providers)
                {
                    errors.push(format!(
                        "mapped comparison {} requires a pinned global mapping, independent review, common-row policy, matching source estimand IDs, and the registered target estimand ID",
                        comparison_id.unwrap_or("<missing>")
                    ));
                }
            }
            _ => errors.push(format!(
                "comparison {} has an unsupported status",
                comparison_id.unwrap_or("<missing>")
            )),
        }
    }

    errors.sort();
    errors
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

    let registration: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("input is not valid JSON: {error}")])?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES)
        .map_err(|error| vec![format!("embedded schema is not valid JSON: {error}")])?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )]
    })?;
    let validator = jsonschema::draft202012::new(&schema).map_err(|error| {
        vec![format!(
            "cannot build registration schema validator: {error}"
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

    let comparisons = registration["comparisons"]
        .as_array()
        .expect("structural schema checks comparisons");
    let comparison_receipts = comparisons
        .iter()
        .map(|comparison| {
            json!({
                "comparison_id": comparison["comparison_id"],
                "left_provider_id": comparison["left_provider_id"],
                "right_provider_id": comparison["right_provider_id"],
                "status": comparison["status"],
                "forecast_skill_comparison_permitted": comparison["forecast_skill_comparison_permitted"],
                "decision_value_comparison_permitted": comparison["decision_value_comparison_permitted"]
            })
        })
        .collect::<Vec<_>>();

    let registration_id = string_at(&registration, "registration_id").unwrap_or_default();
    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let registration_sha256 = sha256_hex(input);
    let receipt_commitment = sha256_hex(
        format!(
            "pirb-cross-provider-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{registration_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "cross_provider_registration_validation",
        "registration_id": registration_id,
        "registration_status": registration["status"],
        "evidence_scope": registration["evidence_scope"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "registration_sha256": registration_sha256,
        "schema_sha256": schema_sha256,
        "provider_count": registration["providers"].as_array().map_or(0, Vec::len),
        "comparison_count": comparisons.len(),
        "comparisons": comparison_receipts,
        "structural_validation": "passed",
        "semantic_validation": "passed",
        "scoring_performed": false,
        "forecast_skill_established": false,
        "causal_impact_established": false,
        "live_decision_authority": false,
        "receipt_commitment_sha256": receipt_commitment
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?
        .ok_or_else(|| {
            "usage: pirb-cross-provider-benchmark-validate --input REGISTRATION.json [--source-commit SHA]"
                .to_owned()
        })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read input {}: {error}", input_path))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());

    let receipt = validate_contract(&input, &source_commit).map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize validation receipt: {error}"))?
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

    fn example() -> Value {
        serde_json::from_slice(EXAMPLE_BYTES).expect("checked-in fixture is valid JSON")
    }

    fn validate(value: &Value) -> Result<Value, Vec<String>> {
        validate_contract(
            &serde_json::to_vec(value).expect("test value serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    #[test]
    fn incompatible_provider_pair_validates_without_scoring() {
        let receipt = validate(&example()).expect("metadata-only rejection example is valid");
        assert_eq!(receipt["scoring_performed"], false);
        assert_eq!(receipt["comparisons"][0]["status"], "not_comparable");
        assert_eq!(
            receipt["comparisons"][0]["forecast_skill_comparison_permitted"],
            false
        );
        assert_eq!(receipt["causal_impact_established"], false);
    }

    #[test]
    fn direct_comparison_rejects_target_unit_and_horizon_mismatches() {
        let mut value = example();
        let comparison = &mut value["comparisons"][0];
        comparison["status"] = json!("directly_comparable");
        comparison["forecast_skill_comparison_permitted"] = json!(true);
        comparison["common_row_policy"] = json!("same mature outcome rows");
        comparison["mapping"] = Value::Null;

        let errors = validate(&value).expect_err("mismatched provider claims must fail closed");
        assert!(errors.iter().any(|error| error.contains("original_target")));
        assert!(errors.iter().any(|error| error.contains("analysis_unit")));
        assert!(
            errors
                .iter()
                .any(|error| error.contains("temporal_aggregation"))
        );
    }

    #[test]
    fn direct_comparison_requires_canonical_estimand_identity() {
        let mut value = example();
        let comparison = &mut value["comparisons"][0];
        comparison["status"] = json!("directly_comparable");
        comparison["forecast_skill_comparison_permitted"] = json!(true);
        comparison["common_row_policy"] = json!("common mature outcome rows");
        comparison["mapping"] = Value::Null;
        value["providers"][1]["canonical_estimand_id"] =
            value["providers"][0]["canonical_estimand_id"].clone();
        value["shared_estimand"]["estimand_id"] = json!("different-registered-estimand");
        value["shared_estimand"]["analysis_unit"] = json!("country-month");
        value["shared_estimand"]["temporal_aggregation"] = json!("calendar month");
        value["shared_estimand"]["target_measure"] = json!("binary_event_probability");

        let errors =
            validate(&value).expect_err("canonical target identity must match registration");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("canonical estimand ID"))
        );
        assert!(errors.iter().any(|error| error.contains("target_measure")));
    }

    #[test]
    fn unknown_provider_references_are_rejected() {
        let mut value = example();
        value["comparisons"][0]["right_provider_id"] = json!("provider-does-not-exist");
        let errors = validate(&value).expect_err("dangling provider reference must fail closed");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("unknown right provider"))
        );
    }

    #[test]
    fn event_count_provider_cannot_register_binary_probability_metrics() {
        let mut value = example();
        value["preregistration"]["primary_metrics_by_output_type"]["event_count"][0] =
            json!("Brier score");
        let errors =
            validate(&value).expect_err("count forecast must not be scored as probability");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("binary-probability metric"))
        );
    }

    #[test]
    fn historical_evaluation_requires_authority_and_disclosure_approval() {
        let mut value = example();
        value["evidence_scope"] = json!("authorized_historical_evaluation");
        let errors = validate(&value).expect_err("historical data needs governance clearance");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("data_authority_status=authorized"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("disclosure_review_status=approved"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("frozen outcome release digest"))
        );
    }

    #[test]
    fn mapped_comparison_requires_independent_review_and_global_mapping_pin() {
        let mut value = example();
        let comparison = &mut value["comparisons"][0];
        comparison["status"] = json!("comparable_after_predeclared_mapping");
        comparison["forecast_skill_comparison_permitted"] = json!(true);
        comparison["common_row_policy"] = json!("pre-registered common rows");
        comparison["mapping"] = json!({
            "method_reference": "mapping-method-v1",
            "method_sha256": "2222222222222222222222222222222222222222222222222222222222222222",
            "independently_verified": true,
            "review_reference": "independent-review-record",
            "left_source_estimand_id": "views-country-month-brd25-probability",
            "right_source_estimand_id": "acled-cast-admin1-four-week-event-count",
            "target_estimand_id": "unresolved-no-common-estimand"
        });
        let errors = validate(&value).expect_err("global mapping pin must be required");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("pinned global mapping"))
        );
    }
}
