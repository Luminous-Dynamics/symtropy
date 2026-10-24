// SPDX-License-Identifier: AGPL-3.0-or-later

//! Semantic validation for the frozen animal model-structure uncertainty oracle.
//!
//! This validates corpus integrity and fail-closed contract shape. It does not
//! fit ecological models or qualify empirical biological claims.

pub mod decision;
pub mod experiment_design;

use serde_json::Value;
use std::collections::BTreeSet;

/// Embedded synthetic oracle bound to the source tree and CI.
pub const EMBEDDED_ORACLE: &str =
    include_str!("../../../../docs/living-world/ANIMAL_MODEL_STRUCTURE_UNCERTAINTY_ORACLE_V1.json");

/// Embedded schema whose frozen corpus count must match the checked-in oracle.
pub const EMBEDDED_SCHEMA: &str = include_str!(
    "../../../../docs/living-world/ANIMAL_MODEL_STRUCTURE_UNCERTAINTY_ORACLE_V1.schema.json"
);

const REQUIRED_RUN_IDENTITY: &[&str] = &[
    "source_commit",
    "model_structure_id",
    "model_schema_digest",
    "equation_or_rule_registry_digest",
    "parameter_config_digest",
    "calibration_data_sha256",
    "test_data_sha256",
    "independent_unit_manifest_sha256",
    "model_set_manifest_sha256",
    "model_weighting_rule",
    "stochastic_coupling_policy",
    "evaluator_version",
    "decision_loss_version",
    "forecast_horizon",
    "compute_budget",
    "uncertainty_method",
    "preregistered_margins",
];

/// Validate the oracle's corpus-level invariants beyond basic JSON parsing.
///
/// Returns the actual case count only when the whole corpus passes. Errors are
/// collected rather than accepting partial corpora or treating missing values
/// as zero.
pub fn validate_oracle_json(raw: &str) -> Result<usize, Vec<String>> {
    let root: Value =
        serde_json::from_str(raw).map_err(|error| vec![format!("invalid JSON: {error}")])?;
    let mut errors = Vec::new();

    let Some(object) = root.as_object() else {
        return Err(vec!["oracle root must be an object".to_owned()]);
    };

    require_string_equals(
        object,
        "schema_version",
        "ANIMAL-MODEL-STRUCTURE-UNCERTAINTY-ORACLE-V1",
        &mut errors,
    );
    require_string_equals(object, "status", "synthetic_contract_fixture", &mut errors);
    for field in ["tracking_issue", "note", "independent_unit"] {
        require_nonempty_string(object, field, &mut errors);
    }

    let declared_count = object
        .get("case_count")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    if declared_count.is_none() {
        errors.push("case_count must be a non-negative integer".to_owned());
    }

    for field in [
        "required_run_identity",
        "uncertainty_components",
        "validation_levels",
        "comparison_policy",
    ] {
        validate_nonempty_unique_string_array(object.get(field), field, &mut errors);
    }
    require_string_array_members(
        object.get("required_run_identity"),
        REQUIRED_RUN_IDENTITY,
        "required_run_identity",
        &mut errors,
    );
    require_string_array_members(
        object.get("uncertainty_components"),
        &[
            "aleatoric",
            "parameter",
            "structural_model",
            "observation_or_state",
        ],
        "uncertainty_components",
        &mut errors,
    );
    require_string_array_members(
        object.get("validation_levels"),
        &[
            "conceptual_structure",
            "state",
            "process",
            "system",
            "decision_robustness",
        ],
        "validation_levels",
        &mut errors,
    );

    let Some(cases) = object.get("cases").and_then(Value::as_array) else {
        errors.push("cases must be an array".to_owned());
        return Err(errors);
    };

    if let Some(declared) = declared_count
        && declared != cases.len()
    {
        errors.push(format!(
            "case_count declares {declared}, but cases contains {} records",
            cases.len()
        ));
    }

    let mut ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let prefix = format!("cases[{index}]");
        let Some(case_object) = case.as_object() else {
            errors.push(format!("{prefix} must be an object"));
            continue;
        };

        match case_object.get("id").and_then(Value::as_str) {
            Some(id) if valid_case_id(id) => {
                if !ids.insert(id.to_owned()) {
                    errors.push(format!("{prefix}.id duplicates '{id}'"));
                }
            }
            Some(_) => errors.push(format!("{prefix}.id must be lower snake_case")),
            None => errors.push(format!("{prefix}.id must be a string")),
        }

        for field in ["kind", "setup", "perturbation", "reason"] {
            match case_object.get(field).and_then(Value::as_str) {
                Some(value) if !value.trim().is_empty() => {}
                _ => errors.push(format!("{prefix}.{field} must be a non-empty string")),
            }
        }

        match case_object.get("expected").and_then(Value::as_object) {
            Some(expected) => validate_nonempty_unique_string_array(
                expected.get("required_properties"),
                &format!("{prefix}.expected.required_properties"),
                &mut errors,
            ),
            None => errors.push(format!("{prefix}.expected must be an object")),
        }

        validate_nonempty_unique_string_array(
            case_object.get("fail_if"),
            &format!("{prefix}.fail_if"),
            &mut errors,
        );
    }

    if cases.is_empty() {
        errors.push("oracle must contain at least one case".to_owned());
    }

    if errors.is_empty() {
        Ok(cases.len())
    } else {
        Err(errors)
    }
}

/// Validate the committed fixture through the consumer code path.
pub fn validate_embedded_oracle() -> Result<usize, Vec<String>> {
    validate_oracle_json(EMBEDDED_ORACLE)
}

fn valid_case_id(id: &str) -> bool {
    !id.is_empty()
        && id.as_bytes()[0].is_ascii_lowercase()
        && id.split('_').all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn require_string_equals(
    object: &serde_json::Map<String, Value>,
    field: &str,
    expected: &str,
    errors: &mut Vec<String>,
) {
    match object.get(field).and_then(Value::as_str) {
        Some(actual) if actual == expected => {}
        Some(actual) => errors.push(format!("{field} must be '{expected}', got '{actual}'")),
        None => errors.push(format!("{field} must be a string")),
    }
}

fn require_nonempty_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
    errors: &mut Vec<String>,
) {
    match object.get(field).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => {}
        _ => errors.push(format!("{field} must be a non-empty string")),
    }
}

fn validate_nonempty_unique_string_array(
    value: Option<&Value>,
    field: &str,
    errors: &mut Vec<String>,
) {
    let Some(values) = value.and_then(Value::as_array) else {
        errors.push(format!("{field} must be an array"));
        return;
    };
    if values.is_empty() {
        errors.push(format!("{field} must not be empty"));
        return;
    }
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        match value.as_str() {
            Some(text) if !text.trim().is_empty() => {
                if !seen.insert(text) {
                    errors.push(format!("{field}[{index}] duplicates '{text}'"));
                }
            }
            _ => errors.push(format!("{field}[{index}] must be a non-empty string")),
        }
    }
}

fn require_string_array_members(
    value: Option<&Value>,
    required: &[&str],
    field: &str,
    errors: &mut Vec<String>,
) {
    let Some(values) = value.and_then(Value::as_array) else {
        return;
    };
    for required_value in required {
        if !values
            .iter()
            .any(|value| value.as_str() == Some(*required_value))
        {
            errors.push(format!(
                "{field} is missing required member '{required_value}'"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed_fixture() -> Value {
        serde_json::from_str(EMBEDDED_ORACLE).expect("committed fixture should parse")
    }

    #[test]
    fn embedded_oracle_passes_semantic_contract() {
        let count = validate_embedded_oracle().expect("embedded oracle should validate");
        assert_eq!(count, 35);
    }

    #[test]
    fn declared_count_must_match_actual_cases() {
        let mut fixture = parsed_fixture();
        fixture["case_count"] = Value::from(999);
        let error =
            validate_oracle_json(&fixture.to_string()).expect_err("count mismatch must fail");
        assert!(
            error
                .iter()
                .any(|message| message.contains("case_count declares"))
        );
    }

    #[test]
    fn duplicate_case_ids_fail_closed() {
        let mut fixture = parsed_fixture();
        let duplicate = fixture["cases"][0]["id"].clone();
        fixture["cases"][1]["id"] = duplicate;
        let error =
            validate_oracle_json(&fixture.to_string()).expect_err("duplicate IDs must fail");
        assert!(error.iter().any(|message| message.contains("duplicates")));
    }

    #[test]
    fn empty_expected_properties_fail_closed() {
        let mut fixture = parsed_fixture();
        fixture["cases"][0]["expected"]["required_properties"] = Value::Array(Vec::new());
        let error =
            validate_oracle_json(&fixture.to_string()).expect_err("empty expectation must fail");
        assert!(
            error
                .iter()
                .any(|message| message.contains("required_properties"))
        );
    }

    #[test]
    fn missing_run_identity_fails_closed() {
        let mut fixture = parsed_fixture();
        fixture["required_run_identity"] = Value::Array(Vec::new());
        let error =
            validate_oracle_json(&fixture.to_string()).expect_err("missing identity must fail");
        assert!(
            error
                .iter()
                .any(|message| message.contains("required_run_identity"))
        );
    }

    #[test]
    fn missing_required_run_identity_member_fails_closed() {
        let mut fixture = parsed_fixture();
        fixture["required_run_identity"]
            .as_array_mut()
            .expect("identity array")
            .retain(|value| value.as_str() != Some("model_structure_id"));
        let error = validate_oracle_json(&fixture.to_string())
            .expect_err("required model identity field must fail");
        assert!(
            error
                .iter()
                .any(|message| message.contains("missing required member 'model_structure_id'"))
        );
    }

    #[test]
    fn unknown_schema_version_is_rejected() {
        let mut fixture = parsed_fixture();
        fixture["schema_version"] = Value::String("ANIMAL-MODEL-UNCERTAINTY-ORACLE-V0".to_owned());
        let error =
            validate_oracle_json(&fixture.to_string()).expect_err("unknown schema must fail");
        assert!(
            error
                .iter()
                .any(|message| message.contains("schema_version"))
        );
    }

    #[test]
    fn malformed_json_is_rejected() {
        let error = validate_oracle_json("{not-json").expect_err("malformed JSON must fail");
        assert!(error[0].starts_with("invalid JSON"));
    }

    #[test]
    fn schema_freezes_the_same_case_count_as_the_oracle() {
        let oracle = parsed_fixture();
        let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA).expect("schema should parse");
        let expected_count = oracle["cases"].as_array().expect("cases array").len();
        assert_eq!(oracle["case_count"].as_u64(), Some(expected_count as u64));
        assert_eq!(
            schema["properties"]["case_count"]["const"].as_u64(),
            Some(expected_count as u64)
        );
        assert_eq!(
            schema["properties"]["cases"]["minItems"].as_u64(),
            Some(expected_count as u64)
        );
        assert_eq!(
            schema["properties"]["cases"]["maxItems"].as_u64(),
            Some(expected_count as u64)
        );
    }
}
