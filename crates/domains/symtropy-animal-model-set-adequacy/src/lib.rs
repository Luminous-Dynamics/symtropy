// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed semantic validation for the blinded model-set adequacy oracle.
//!
//! This checks the contract corpus and schema agreement. It does not simulate
//! ecology, prove a candidate model set complete, or validate biological truth.

use serde_json::{Map, Value};
use std::collections::BTreeSet;

/// Checked-in synthetic challenge oracle.
pub const EMBEDDED_ORACLE: &str =
    include_str!("../../../../docs/living-world/ANIMAL_MODEL_SET_ADEQUACY_ORACLE_V1.json");

/// Frozen JSON Schema paired with the checked-in oracle.
pub const EMBEDDED_SCHEMA: &str =
    include_str!("../../../../docs/living-world/ANIMAL_MODEL_SET_ADEQUACY_ORACLE_V1.schema.json");

const EXPECTED_CASE_COUNT: usize = 52;

const REQUIRED_ROOT_FIELDS: &[&str] = &[
    "schema_version",
    "status",
    "research_snapshot",
    "tracking_issue",
    "note",
    "case_count",
    "independent_unit",
    "required_run_identity",
    "uncertainty_layers",
    "comparison_policy",
    "cases",
];

const REQUIRED_RUN_IDENTITY: &[&str] = &[
    "source_commit",
    "candidate_model_set_digest",
    "model_lineage_registry_digest",
    "candidate_equation_registry_digest",
    "challenge_family_manifest_sha256",
    "calibration_split_sha256",
    "development_split_sha256",
    "blind_challenge_split_sha256",
    "independent_unit_manifest_sha256",
    "observation_catalog_sha256",
    "experiment_budget",
    "compute_budget",
    "decision_loss_version",
    "threshold_policy_digest",
    "evaluator_version",
    "preregistered_margins",
    "pre_reveal_prediction_digest",
];

const REQUIRED_UNCERTAINTY_LAYERS: &[&str] = &[
    "aleatoric",
    "parameter",
    "structural_registered",
    "observation_or_state",
    "omission_misspecification",
];

const REQUIRED_COMPARISON_POLICY: &[&str] = &[
    "test_model_set_adequacy_separately_from_within_set_risk",
    "challenge_generator_is_blinded",
    "missing_metric_is_not_zero",
    "failed_selected_experiments_remain_counted",
    "budget_mismatch_is_cost_tradeoff",
    "keep_model_revision_receipts_immutable",
    "synthetic_success_is_not_empirical_validation",
];

const REQUIRED_CASE_FIELDS: &[&str] = &[
    "id",
    "kind",
    "setup",
    "challenge",
    "expected",
    "fail_if",
    "reason",
];

/// Validate a candidate oracle against the frozen schema contract and
/// corpus-level invariants. Returns the case count only on complete success.
pub fn validate_oracle_json(raw: &str) -> Result<usize, Vec<String>> {
    let oracle: Value =
        serde_json::from_str(raw).map_err(|error| vec![format!("invalid oracle JSON: {error}")])?;
    let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA)
        .map_err(|error| vec![format!("invalid embedded schema JSON: {error}")])?;

    validate_oracle_value(&oracle, &schema)
}

/// Validate the checked-in oracle through the same code path used by callers.
pub fn validate_embedded_oracle() -> Result<usize, Vec<String>> {
    validate_oracle_json(EMBEDDED_ORACLE)
}

fn validate_oracle_value(oracle: &Value, schema: &Value) -> Result<usize, Vec<String>> {
    let mut errors = Vec::new();
    let Some(root) = oracle.as_object() else {
        return Err(vec!["oracle root must be an object".to_owned()]);
    };

    check_only_fields(root, REQUIRED_ROOT_FIELDS, "oracle", &mut errors);
    require_string_equals(
        root,
        "schema_version",
        "ANIMAL-MODEL-SET-ADEQUACY-ORACLE-V1",
        &mut errors,
    );
    require_string_equals(root, "status", "synthetic_contract_fixture", &mut errors);
    for field in [
        "tracking_issue",
        "note",
        "independent_unit",
        "research_snapshot",
    ] {
        require_nonempty_string(root, field, &mut errors);
    }

    if !root
        .get("research_snapshot")
        .and_then(Value::as_str)
        .is_some_and(valid_iso_date)
    {
        errors.push("research_snapshot must be a valid ISO date (YYYY-MM-DD)".to_owned());
    }
    if !root
        .get("tracking_issue")
        .and_then(Value::as_str)
        .is_some_and(valid_tracking_issue_url)
    {
        errors.push("tracking_issue must be a canonical Symtropy GitHub issue URL".to_owned());
    }

    let cases = match root.get("cases").and_then(Value::as_array) {
        Some(cases) if !cases.is_empty() => Some(cases),
        Some(_) => {
            errors.push("cases must not be empty".to_owned());
            None
        }
        None => {
            errors.push("cases must be an array".to_owned());
            None
        }
    };
    let Some(cases) = cases else {
        return Err(errors);
    };

    let declared_count = root
        .get("case_count")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok());
    if declared_count != Some(cases.len()) {
        errors.push(format!(
            "case_count must match cases.length (declared {declared_count:?}, actual {})",
            cases.len()
        ));
    }

    if cases.len() != EXPECTED_CASE_COUNT {
        errors.push(format!(
            "V1 fixture must contain exactly {EXPECTED_CASE_COUNT} cases, got {}",
            cases.len()
        ));
    }

    let mut case_ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        validate_case(case, index, &mut case_ids, &mut errors);
    }

    validate_unique_string_array(
        root.get("required_run_identity"),
        "required_run_identity",
        &mut errors,
    );
    check_required_members(
        root.get("required_run_identity"),
        REQUIRED_RUN_IDENTITY,
        "required_run_identity",
        &mut errors,
    );

    validate_unique_string_array(
        root.get("uncertainty_layers"),
        "uncertainty_layers",
        &mut errors,
    );
    check_exact_members(
        root.get("uncertainty_layers"),
        REQUIRED_UNCERTAINTY_LAYERS,
        "uncertainty_layers",
        &mut errors,
    );

    validate_unique_string_array(
        root.get("comparison_policy"),
        "comparison_policy",
        &mut errors,
    );
    check_required_members(
        root.get("comparison_policy"),
        REQUIRED_COMPARISON_POLICY,
        "comparison_policy",
        &mut errors,
    );

    validate_schema_contract(schema, EXPECTED_CASE_COUNT, &mut errors);

    if errors.is_empty() {
        Ok(cases.len())
    } else {
        Err(errors)
    }
}

fn validate_case(
    value: &Value,
    index: usize,
    seen: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    let prefix = format!("cases[{index}]");
    let Some(case) = value.as_object() else {
        errors.push(format!("{prefix} must be an object"));
        return;
    };
    check_only_fields(case, REQUIRED_CASE_FIELDS, &prefix, errors);

    for field in ["kind", "setup", "challenge", "reason"] {
        require_nonempty_string(case, field, errors);
    }

    match case.get("id").and_then(Value::as_str) {
        Some(id) if valid_case_id(id) => {
            if !seen.insert(id.to_owned()) {
                errors.push(format!("{prefix}.id duplicates '{id}'"));
            }
        }
        Some(_) => errors.push(format!(
            "{prefix}.id must be lower snake case with no empty segments"
        )),
        None => errors.push(format!("{prefix}.id must be a string")),
    }

    let Some(expected) = case.get("expected").and_then(Value::as_object) else {
        errors.push(format!("{prefix}.expected must be an object"));
        return;
    };
    check_only_fields(
        expected,
        &["required_properties"],
        &format!("{prefix}.expected"),
        errors,
    );
    validate_unique_string_array(
        expected.get("required_properties"),
        &format!("{prefix}.expected.required_properties"),
        errors,
    );
    validate_unique_string_array(case.get("fail_if"), &format!("{prefix}.fail_if"), errors);
}

fn validate_schema_contract(schema: &Value, case_count: usize, errors: &mut Vec<String>) {
    require_schema_string(
        schema,
        "/$schema",
        "https://json-schema.org/draft/2020-12/schema",
        "schema dialect",
        errors,
    );
    require_schema_string(
        schema,
        "/properties/schema_version/const",
        "ANIMAL-MODEL-SET-ADEQUACY-ORACLE-V1",
        "schema version const",
        errors,
    );
    require_schema_string(schema, "/type", "object", "schema root type", errors);
    require_schema_boolean(
        schema,
        "/additionalProperties",
        false,
        "schema root additionalProperties",
        errors,
    );
    require_schema_string_array_exact(
        schema,
        "/required",
        REQUIRED_ROOT_FIELDS,
        "schema root required fields",
        errors,
    );

    require_schema_string(
        schema,
        "/properties/status/const",
        "synthetic_contract_fixture",
        "schema status const",
        errors,
    );
    require_schema_string(
        schema,
        "/properties/research_snapshot/format",
        "date",
        "schema research_snapshot format",
        errors,
    );
    require_schema_string(
        schema,
        "/properties/tracking_issue/format",
        "uri",
        "schema tracking_issue format",
        errors,
    );

    require_schema_string(
        schema,
        "/properties/research_snapshot/type",
        "string",
        "schema research_snapshot type",
        errors,
    );
    require_schema_unsigned(
        schema,
        "/properties/research_snapshot/minLength",
        1,
        "schema research_snapshot minLength",
        errors,
    );
    require_schema_string(
        schema,
        "/properties/tracking_issue/type",
        "string",
        "schema tracking_issue type",
        errors,
    );
    require_schema_unsigned(
        schema,
        "/properties/tracking_issue/minLength",
        1,
        "schema tracking_issue minLength",
        errors,
    );
    require_schema_unsigned(
        schema,
        "/properties/case_count/const",
        case_count,
        "schema case_count const",
        errors,
    );

    require_schema_string(
        schema,
        "/properties/cases/type",
        "array",
        "schema cases type",
        errors,
    );
    require_schema_unsigned(
        schema,
        "/properties/cases/minItems",
        case_count,
        "schema cases minItems",
        errors,
    );
    require_schema_unsigned(
        schema,
        "/properties/cases/maxItems",
        case_count,
        "schema cases maxItems",
        errors,
    );

    require_schema_string(
        schema,
        "/properties/cases/items/type",
        "object",
        "schema case item type",
        errors,
    );
    require_schema_boolean(
        schema,
        "/properties/cases/items/additionalProperties",
        false,
        "schema case item additionalProperties",
        errors,
    );
    require_schema_string_array_exact(
        schema,
        "/properties/cases/items/required",
        REQUIRED_CASE_FIELDS,
        "schema case required fields",
        errors,
    );
    require_schema_string(
        schema,
        "/properties/cases/items/properties/id/pattern",
        "^[a-z][a-z0-9_]*$",
        "schema case ID pattern",
        errors,
    );

    for (pointer, label) in [
        (
            "/properties/cases/items/properties/id/type",
            "schema case ID type",
        ),
        (
            "/properties/cases/items/properties/kind/type",
            "schema case kind type",
        ),
        (
            "/properties/cases/items/properties/setup/type",
            "schema case setup type",
        ),
        (
            "/properties/cases/items/properties/challenge/type",
            "schema case challenge type",
        ),
        (
            "/properties/cases/items/properties/reason/type",
            "schema case reason type",
        ),
        (
            "/properties/cases/items/properties/expected/properties/required_properties/items/type",
            "schema expectation item type",
        ),
        (
            "/properties/cases/items/properties/fail_if/items/type",
            "schema fail_if item type",
        ),
        (
            "/properties/required_run_identity/items/type",
            "schema run identity item type",
        ),
        (
            "/properties/comparison_policy/items/type",
            "schema comparison policy item type",
        ),
    ] {
        require_schema_string(schema, pointer, "string", label, errors);
    }
    for (pointer, label) in [
        (
            "/properties/cases/items/properties/kind/minLength",
            "schema case kind minLength",
        ),
        (
            "/properties/cases/items/properties/setup/minLength",
            "schema case setup minLength",
        ),
        (
            "/properties/cases/items/properties/challenge/minLength",
            "schema case challenge minLength",
        ),
        (
            "/properties/cases/items/properties/reason/minLength",
            "schema case reason minLength",
        ),
        (
            "/properties/cases/items/properties/expected/properties/required_properties/items/minLength",
            "schema expectation item minLength",
        ),
        (
            "/properties/cases/items/properties/fail_if/items/minLength",
            "schema fail_if item minLength",
        ),
        (
            "/properties/required_run_identity/items/minLength",
            "schema run identity item minLength",
        ),
        (
            "/properties/comparison_policy/items/minLength",
            "schema comparison policy item minLength",
        ),
    ] {
        require_schema_unsigned(schema, pointer, 1, label, errors);
    }

    require_schema_string(
        schema,
        "/properties/cases/items/properties/expected/type",
        "object",
        "schema expected type",
        errors,
    );
    require_schema_boolean(
        schema,
        "/properties/cases/items/properties/expected/additionalProperties",
        false,
        "schema expected additionalProperties",
        errors,
    );
    require_schema_string_array_exact(
        schema,
        "/properties/cases/items/properties/expected/required",
        &["required_properties"],
        "schema expected required fields",
        errors,
    );
    require_schema_string(
        schema,
        "/properties/cases/items/properties/expected/properties/required_properties/type",
        "array",
        "schema expected.required_properties type",
        errors,
    );
    require_schema_unsigned(
        schema,
        "/properties/cases/items/properties/expected/properties/required_properties/minItems",
        1,
        "schema expected.required_properties minItems",
        errors,
    );
    require_schema_boolean(
        schema,
        "/properties/cases/items/properties/expected/properties/required_properties/uniqueItems",
        true,
        "schema expected.required_properties uniqueItems",
        errors,
    );

    require_schema_string(
        schema,
        "/properties/cases/items/properties/fail_if/type",
        "array",
        "schema fail_if type",
        errors,
    );
    require_schema_unsigned(
        schema,
        "/properties/cases/items/properties/fail_if/minItems",
        1,
        "schema fail_if minItems",
        errors,
    );
    require_schema_boolean(
        schema,
        "/properties/cases/items/properties/fail_if/uniqueItems",
        true,
        "schema fail_if uniqueItems",
        errors,
    );

    require_schema_string(
        schema,
        "/properties/required_run_identity/type",
        "array",
        "schema required_run_identity type",
        errors,
    );
    require_schema_unsigned(
        schema,
        "/properties/required_run_identity/minItems",
        1,
        "schema required_run_identity minItems",
        errors,
    );
    require_schema_boolean(
        schema,
        "/properties/required_run_identity/uniqueItems",
        true,
        "schema required_run_identity uniqueItems",
        errors,
    );

    require_schema_unsigned(
        schema,
        "/properties/comparison_policy/minItems",
        REQUIRED_COMPARISON_POLICY.len(),
        "schema comparison_policy minItems",
        errors,
    );
    require_schema_boolean(
        schema,
        "/properties/comparison_policy/uniqueItems",
        true,
        "schema comparison_policy uniqueItems",
        errors,
    );
    require_schema_string(
        schema,
        "/properties/uncertainty_layers/type",
        "array",
        "schema uncertainty_layers type",
        errors,
    );
    require_schema_unsigned(
        schema,
        "/properties/uncertainty_layers/minItems",
        REQUIRED_UNCERTAINTY_LAYERS.len(),
        "schema uncertainty_layers minItems",
        errors,
    );
    require_schema_boolean(
        schema,
        "/properties/uncertainty_layers/uniqueItems",
        true,
        "schema uncertainty_layers uniqueItems",
        errors,
    );

    let schema_layers = schema
        .pointer("/properties/uncertainty_layers/items/enum")
        .and_then(Value::as_array);
    if let Some(schema_layers) = schema_layers {
        let actual: BTreeSet<&str> = schema_layers.iter().filter_map(Value::as_str).collect();
        let expected: BTreeSet<&str> = REQUIRED_UNCERTAINTY_LAYERS.iter().copied().collect();
        if actual != expected || schema_layers.len() != expected.len() {
            errors.push(
                "schema uncertainty_layers enum does not match validator contract".to_owned(),
            );
        }
    } else {
        errors.push("schema uncertainty_layers enum is missing".to_owned());
    }

    // JSON Schema describes structure; corpus-level uniqueness is semantic and
    // remains enforced by validate_oracle_value. Keeping the two layers explicit
    // prevents a permissive schema edit from quietly weakening the validator.
}

fn require_schema_string(
    schema: &Value,
    pointer: &str,
    expected: &str,
    label: &str,
    errors: &mut Vec<String>,
) {
    if schema.pointer(pointer).and_then(Value::as_str) != Some(expected) {
        errors.push(format!("{label} must equal '{expected}'"));
    }
}

fn require_schema_boolean(
    schema: &Value,
    pointer: &str,
    expected: bool,
    label: &str,
    errors: &mut Vec<String>,
) {
    if schema.pointer(pointer).and_then(Value::as_bool) != Some(expected) {
        errors.push(format!("{label} must be {expected}"));
    }
}

fn require_schema_unsigned(
    schema: &Value,
    pointer: &str,
    expected: usize,
    label: &str,
    errors: &mut Vec<String>,
) {
    if schema.pointer(pointer).and_then(Value::as_u64) != Some(expected as u64) {
        errors.push(format!("{label} must equal {expected}"));
    }
}

fn require_schema_string_array_exact(
    schema: &Value,
    pointer: &str,
    expected: &[&str],
    label: &str,
    errors: &mut Vec<String>,
) {
    let Some(values) = schema.pointer(pointer).and_then(Value::as_array) else {
        errors.push(format!("{label} must be an array"));
        return;
    };
    let actual: Vec<&str> = values.iter().filter_map(Value::as_str).collect();
    let actual_set: BTreeSet<&str> = actual.iter().copied().collect();
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    if actual.len() != values.len() || actual.len() != expected.len() || actual_set != expected_set
    {
        errors.push(format!("{label} do not match the frozen contract"));
    }
}

fn check_only_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
    context: &str,
    errors: &mut Vec<String>,
) {
    for field in object.keys() {
        if !allowed.contains(&field.as_str()) {
            errors.push(format!("{context} contains unknown field '{field}'"));
        }
    }
    for field in allowed {
        if !object.contains_key(*field) {
            errors.push(format!("{context} is missing required field '{field}'"));
        }
    }
}

fn require_string_equals(
    object: &Map<String, Value>,
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

fn require_nonempty_string(object: &Map<String, Value>, field: &str, errors: &mut Vec<String>) {
    match object.get(field).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => {}
        _ => errors.push(format!("{field} must be a non-empty string")),
    }
}

fn validate_unique_string_array(value: Option<&Value>, field: &str, errors: &mut Vec<String>) {
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

fn check_required_members(
    value: Option<&Value>,
    required: &[&str],
    field: &str,
    errors: &mut Vec<String>,
) {
    let Some(values) = value.and_then(Value::as_array) else {
        return;
    };
    let members: BTreeSet<&str> = values.iter().filter_map(Value::as_str).collect();
    for required_value in required {
        if !members.contains(required_value) {
            errors.push(format!(
                "{field} is missing required member '{required_value}'"
            ));
        }
    }
}

fn check_exact_members(
    value: Option<&Value>,
    expected: &[&str],
    field: &str,
    errors: &mut Vec<String>,
) {
    let Some(values) = value.and_then(Value::as_array) else {
        return;
    };
    let actual: BTreeSet<&str> = values.iter().filter_map(Value::as_str).collect();
    let required: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != required {
        errors.push(format!("{field} members do not match the frozen contract"));
    }
}

fn valid_tracking_issue_url(value: &str) -> bool {
    let Some(issue_number) =
        value.strip_prefix("https://github.com/Luminous-Dynamics/symtropy/issues/")
    else {
        return false;
    };
    !issue_number.is_empty() && issue_number.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_iso_date(value: &str) -> bool {
    let mut parts = value.split('-');
    let (Some(year), Some(month), Some(day), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if year.len() != 4
        || month.len() != 2
        || day.len() != 2
        || !year.bytes().all(|byte| byte.is_ascii_digit())
        || !month.bytes().all(|byte| byte.is_ascii_digit())
        || !day.bytes().all(|byte| byte.is_ascii_digit())
    {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        year.parse::<u32>(),
        month.parse::<u32>(),
        day.parse::<u32>(),
    ) else {
        return false;
    };
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed_fixture() -> Value {
        serde_json::from_str(EMBEDDED_ORACLE).expect("committed fixture should parse")
    }

    #[test]
    fn embedded_oracle_passes_semantic_contract() {
        assert_eq!(
            validate_embedded_oracle().expect("oracle must validate"),
            52
        );
    }

    #[test]
    fn case_count_must_match_actual_cases() {
        let mut fixture = parsed_fixture();
        fixture["case_count"] = Value::from(999);
        let errors = validate_oracle_json(&fixture.to_string())
            .expect_err("case-count mismatch must fail closed");
        assert!(errors.iter().any(|e| e.contains("case_count")));
    }

    #[test]
    fn duplicate_case_ids_fail_closed() {
        let mut fixture = parsed_fixture();
        let first_id = fixture["cases"][0]["id"].clone();
        fixture["cases"][1]["id"] = first_id;
        let errors =
            validate_oracle_json(&fixture.to_string()).expect_err("duplicate IDs must fail closed");
        assert!(errors.iter().any(|e| e.contains("duplicates")));
    }

    #[test]
    fn malformed_case_expectations_fail_closed() {
        let mut fixture = parsed_fixture();
        fixture["cases"][0]["expected"]["required_properties"] = Value::Array(Vec::new());
        let errors = validate_oracle_json(&fixture.to_string())
            .expect_err("empty expected properties must fail");
        assert!(errors.iter().any(|e| e.contains("required_properties")));
    }

    #[test]
    fn required_receipt_identity_cannot_be_removed() {
        let mut fixture = parsed_fixture();
        fixture["required_run_identity"]
            .as_array_mut()
            .expect("identity list")
            .retain(|value| value.as_str() != Some("pre_reveal_prediction_digest"));
        let errors =
            validate_oracle_json(&fixture.to_string()).expect_err("missing identity must fail");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("pre_reveal_prediction_digest"))
        );
    }

    #[test]
    fn unknown_root_and_case_fields_are_rejected() {
        let mut fixture = parsed_fixture();
        fixture["unreviewed_root_field"] = Value::Bool(true);
        let errors =
            validate_oracle_json(&fixture.to_string()).expect_err("unknown root field must fail");
        assert!(errors.iter().any(|e| e.contains("unreviewed_root_field")));

        let mut fixture = parsed_fixture();
        fixture["cases"][0]["oracle_truth"] = Value::Bool(true);
        let errors =
            validate_oracle_json(&fixture.to_string()).expect_err("unknown case field must fail");
        assert!(errors.iter().any(|e| e.contains("oracle_truth")));
    }

    #[test]
    fn schema_count_is_pinned_to_the_fixture() {
        let oracle = parsed_fixture();
        let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA).expect("schema must parse");
        let count = oracle["cases"].as_array().expect("cases").len() as u64;
        assert_eq!(oracle["case_count"].as_u64(), Some(count));
        assert_eq!(
            schema["properties"]["case_count"]["const"].as_u64(),
            Some(count)
        );
        assert_eq!(
            schema["properties"]["cases"]["minItems"].as_u64(),
            Some(count)
        );
        assert_eq!(
            schema["properties"]["cases"]["maxItems"].as_u64(),
            Some(count)
        );
    }

    #[test]
    fn uncertainty_layers_are_frozen_not_user_relabelled() {
        let mut fixture = parsed_fixture();
        fixture["uncertainty_layers"][4] = Value::String("confidence".to_owned());
        let errors = validate_oracle_json(&fixture.to_string())
            .expect_err("changed uncertainty layer must fail");
        assert!(errors.iter().any(|e| e.contains("uncertainty_layers")));
    }

    #[test]
    fn v1_case_count_cannot_be_redefined_with_a_matching_schema() {
        let mut fixture = parsed_fixture();
        let mut additional = fixture["cases"][0].clone();
        additional["id"] = Value::String("additional_case_52".to_owned());
        fixture["cases"]
            .as_array_mut()
            .expect("cases array")
            .push(additional);
        fixture["case_count"] = Value::from(53);
        let mut schema: Value = serde_json::from_str(EMBEDDED_SCHEMA).expect("schema should parse");
        schema["properties"]["case_count"]["const"] = Value::from(53);
        schema["properties"]["cases"]["minItems"] = Value::from(53);
        schema["properties"]["cases"]["maxItems"] = Value::from(53);

        let errors = validate_oracle_value(&fixture, &schema)
            .expect_err("V1 size must be frozen in code as well as schema");
        assert!(errors.iter().any(|e| e.contains("exactly 52 cases")));
    }

    #[test]
    fn schema_permissive_root_is_rejected() {
        let oracle = parsed_fixture();
        let mut schema: Value = serde_json::from_str(EMBEDDED_SCHEMA).expect("schema should parse");
        schema["additionalProperties"] = Value::Bool(true);
        let errors = validate_oracle_value(&oracle, &schema)
            .expect_err("permissive root schema must fail closed");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("root additionalProperties"))
        );
    }

    #[test]
    fn schema_missing_required_field_is_rejected() {
        let oracle = parsed_fixture();
        let mut schema: Value = serde_json::from_str(EMBEDDED_SCHEMA).expect("schema should parse");
        schema["required"]
            .as_array_mut()
            .expect("root required list")
            .retain(|value| value.as_str() != Some("cases"));
        let errors = validate_oracle_value(&oracle, &schema)
            .expect_err("schema root required mutation must fail");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("schema root required fields"))
        );
    }

    #[test]
    fn schema_missing_fail_conditions_is_rejected() {
        let oracle = parsed_fixture();
        let mut schema: Value = serde_json::from_str(EMBEDDED_SCHEMA).expect("schema should parse");
        schema["properties"]["cases"]["items"]["properties"]["fail_if"]["minItems"] =
            Value::from(0);
        let errors = validate_oracle_value(&oracle, &schema)
            .expect_err("fail_if must remain non-empty in the schema");
        assert!(errors.iter().any(|e| e.contains("schema fail_if minItems")));
    }

    #[test]
    fn schema_must_remain_closed_at_case_and_expectation_levels() {
        let oracle = parsed_fixture();
        let mut schema: Value = serde_json::from_str(EMBEDDED_SCHEMA).expect("schema should parse");
        schema["properties"]["cases"]["items"]["additionalProperties"] = Value::Bool(true);
        let errors = validate_oracle_value(&oracle, &schema)
            .expect_err("case schema must reject unreviewed properties");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("case item additionalProperties"))
        );

        schema["properties"]["cases"]["items"]["additionalProperties"] = Value::Bool(false);
        schema["properties"]["cases"]["items"]["properties"]["expected"]["additionalProperties"] =
            Value::Bool(true);
        let errors = validate_oracle_value(&oracle, &schema)
            .expect_err("expected schema must reject unreviewed properties");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("expected additionalProperties"))
        );
    }

    #[test]
    fn required_comparison_policies_cannot_be_removed() {
        let mut fixture = parsed_fixture();
        fixture["comparison_policy"]
            .as_array_mut()
            .expect("comparison policy")
            .retain(|value| value.as_str() != Some("challenge_generator_is_blinded"));
        let errors = validate_oracle_json(&fixture.to_string())
            .expect_err("removing blinding policy must fail closed");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("challenge_generator_is_blinded"))
        );
    }

    #[test]
    fn malformed_research_date_is_rejected() {
        let mut fixture = parsed_fixture();
        fixture["research_snapshot"] = Value::String("2026-02-30".to_owned());
        let errors = validate_oracle_json(&fixture.to_string())
            .expect_err("impossible calendar date must fail closed");
        assert!(errors.iter().any(|e| e.contains("ISO date")));
    }

    #[test]
    fn noncanonical_tracking_issue_url_is_rejected() {
        let mut fixture = parsed_fixture();
        fixture["tracking_issue"] = Value::String("https://example.com/issue/2334".to_owned());
        let errors =
            validate_oracle_json(&fixture.to_string()).expect_err("foreign URL must fail closed");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("canonical Symtropy GitHub issue URL"))
        );
    }

    #[test]
    fn date_parser_handles_leap_year_boundary() {
        assert!(valid_iso_date("2024-02-29"));
        assert!(!valid_iso_date("2023-02-29"));
        assert!(valid_iso_date("2000-02-29"));
        assert!(!valid_iso_date("1900-02-29"));
    }

    #[test]
    fn only_lowercase_snake_case_ids_are_valid() {
        for bad in [
            "",
            "_leading",
            "trailing_",
            "double__underscore",
            "BadCase",
            "valid-id",
        ] {
            assert!(!valid_case_id(bad), "{bad}");
        }
        assert!(valid_case_id("heldout_mechanism_01"));
    }

    #[test]
    fn invalid_json_is_rejected() {
        let errors = validate_oracle_json("{not-json").expect_err("invalid JSON must fail");
        assert!(errors[0].starts_with("invalid oracle JSON"));
    }
}
