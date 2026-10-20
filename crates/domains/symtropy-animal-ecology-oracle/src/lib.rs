// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed structural validation for the frozen ecological-transfer oracle.
//!
//! Contract consistency is not a simulation run and is not biological validation.

use serde_json::Value;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

pub const ORACLE_SCHEMA_VERSION: &str = "ANIMAL-ECOLOGICAL-TRANSFER-ORACLE-V1";
pub const EXPECTED_CASE_COUNT: usize = 51;

pub const COMMITTED_ORACLE_JSON: &str =
    include_str!("../../../../docs/living-world/ANIMAL_ECOLOGICAL_TRANSFER_ORACLE_V1.json");
pub const COMMITTED_SCHEMA_JSON: &str =
    include_str!("../../../../docs/living-world/ANIMAL_ECOLOGICAL_TRANSFER_ORACLE_V1.schema.json");

const TOP_LEVEL_KEYS: &[&str] = &[
    "schema_version",
    "status",
    "research_snapshot",
    "tracking_issue",
    "note",
    "case_count",
    "independent_unit",
    "required_run_identity",
    "comparison_policy",
    "cases",
];

const CASE_KEYS: &[&str] = &[
    "id",
    "kind",
    "setup",
    "perturbation",
    "expected",
    "fail_if",
    "reason",
];

const EXPECTED_KEYS: &[&str] = &["required_properties"];

const REQUIRED_RUN_IDENTITY: &[&str] = &[
    "source_commit",
    "simulator_version",
    "policy_version",
    "ecology_config_sha256",
    "scenario_manifest_sha256",
    "independent_unit_manifest_sha256",
    "train_split_sha256",
    "validation_split_sha256",
    "test_split_sha256",
    "evaluator_version",
    "seed_family",
    "sensor_action_contract",
    "simulated_horizon",
    "interaction_budget",
    "compute_budget",
    "hardware_runtime",
    "uncertainty_method",
    "preregistered_margins",
];

const REQUIRED_COMPARISON_POLICY: &[&str] = &[
    "matched_budget_is_default",
    "extra_budget_is_cost_tradeoff",
    "missing_metric_is_not_zero",
    "failed_selected_rollout_remains_in_accounting",
    "synthetic_pass_is_not_empirical_validation",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    message: String,
}

impl ValidationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ValidationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    pub schema_version: String,
    pub case_count: usize,
    pub unique_case_ids: usize,
}

/// Validate the machine-readable oracle without claiming to execute its scenarios.
pub fn validate_oracle(input: &str) -> Result<ValidationReport, ValidationError> {
    let root: Value = serde_json::from_str(input)
        .map_err(|error| ValidationError::new(format!("oracle JSON is invalid: {error}")))?;
    let root_object = root
        .as_object()
        .ok_or_else(|| ValidationError::new("oracle root must be an object"))?;
    check_exact_keys(root_object, TOP_LEVEL_KEYS, "oracle")?;

    let schema_version = required_string(&root, "schema_version", "oracle")?;
    if schema_version != ORACLE_SCHEMA_VERSION {
        return Err(ValidationError::new(format!(
            "schema_version must be {ORACLE_SCHEMA_VERSION}, got {schema_version}"
        )));
    }
    require_exact_string(&root, "status", "synthetic_contract_fixture", "oracle")?;
    let research_snapshot = required_string(&root, "research_snapshot", "oracle")?;
    if !is_valid_iso_date(research_snapshot) {
        return Err(ValidationError::new(
            "oracle.research_snapshot must be a real YYYY-MM-DD calendar date",
        ));
    }
    require_exact_string(
        &root,
        "tracking_issue",
        "https://github.com/Luminous-Dynamics/symtropy/issues/2214",
        "oracle",
    )?;
    require_nonempty_string(&root, "note", "oracle")?;
    require_nonempty_string(&root, "independent_unit", "oracle")?;

    let declared_count = root
        .get("case_count")
        .and_then(Value::as_u64)
        .ok_or_else(|| ValidationError::new("oracle.case_count must be an integer"))?;
    if declared_count != EXPECTED_CASE_COUNT as u64 {
        return Err(ValidationError::new(format!(
            "oracle.case_count must be {EXPECTED_CASE_COUNT}, got {declared_count}"
        )));
    }

    require_unique_nonempty_string_array(
        &root,
        "required_run_identity",
        "oracle",
        REQUIRED_RUN_IDENTITY,
    )?;
    require_unique_nonempty_string_array(
        &root,
        "comparison_policy",
        "oracle",
        REQUIRED_COMPARISON_POLICY,
    )?;

    let cases = root
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new("oracle.cases must be an array"))?;
    if cases.len() != EXPECTED_CASE_COUNT {
        return Err(ValidationError::new(format!(
            "oracle.cases must contain {EXPECTED_CASE_COUNT} records, got {}",
            cases.len()
        )));
    }

    let mut ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let path = format!("oracle.cases[{index}]");
        let object = case
            .as_object()
            .ok_or_else(|| ValidationError::new(format!("{path} must be an object")))?;
        check_exact_keys(object, CASE_KEYS, &path)?;

        let id = required_string(case, "id", &path)?;
        if !is_snake_case_id(id) {
            return Err(ValidationError::new(format!(
                "{path}.id must match ^[a-z][a-z0-9_]*$, got {id}"
            )));
        }
        if !ids.insert(id.to_owned()) {
            return Err(ValidationError::new(format!(
                "duplicate oracle case id: {id}"
            )));
        }

        for key in ["kind", "setup", "perturbation", "reason"] {
            require_nonempty_string(case, key, &path)?;
        }
        let expected = case
            .get("expected")
            .ok_or_else(|| ValidationError::new(format!("{path}.expected is required")))?;
        let expected_object = expected
            .as_object()
            .ok_or_else(|| ValidationError::new(format!("{path}.expected must be an object")))?;
        check_exact_keys(expected_object, EXPECTED_KEYS, &format!("{path}.expected"))?;
        require_unique_nonempty_string_array(
            expected,
            "required_properties",
            &format!("{path}.expected"),
            &[],
        )?;
        require_unique_nonempty_string_array(case, "fail_if", &path, &[])?;
    }

    Ok(ValidationReport {
        schema_version,
        case_count: cases.len(),
        unique_case_ids: ids.len(),
    })
}

/// Validate the committed schema's critical invariants and the oracle against it.
///
/// This is a focused validator for this versioned contract, not a general JSON Schema engine.
pub fn validate_artifacts(
    oracle_json: &str,
    schema_json: &str,
) -> Result<ValidationReport, ValidationError> {
    let report = validate_oracle(oracle_json)?;
    let schema: Value = serde_json::from_str(schema_json)
        .map_err(|error| ValidationError::new(format!("schema JSON is invalid: {error}")))?;

    require_exact_string(
        &schema,
        "$schema",
        "https://json-schema.org/draft/2020-12/schema",
        "schema",
    )?;
    require_exact_string(
        &schema,
        "$id",
        "https://github.com/Luminous-Dynamics/symtropy/blob/main/docs/living-world/ANIMAL_ECOLOGICAL_TRANSFER_ORACLE_V1.schema.json",
        "schema",
    )?;
    require_exact_string(&schema, "type", "object", "schema")?;
    require_schema_bool(&schema, "additionalProperties", false, "schema")?;
    require_schema_required_fields(&schema, TOP_LEVEL_KEYS, "schema")?;

    let properties = schema
        .get("properties")
        .ok_or_else(|| ValidationError::new("schema.properties is required"))?;
    let property_object = properties
        .as_object()
        .ok_or_else(|| ValidationError::new("schema.properties must be an object"))?;
    check_exact_keys(property_object, TOP_LEVEL_KEYS, "schema.properties")?;

    require_schema_const(properties, "schema_version", ORACLE_SCHEMA_VERSION)?;
    require_schema_const(properties, "status", "synthetic_contract_fixture")?;
    require_schema_const(properties, "case_count", "51")?;
    require_schema_string(properties, "schema_version", "type", "string")?;
    require_schema_string(properties, "status", "type", "string")?;

    for key in ["note", "tracking_issue", "independent_unit"] {
        require_schema_string(properties, key, "type", "string")?;
        let property_schema = properties
            .get(key)
            .ok_or_else(|| ValidationError::new(format!("schema.properties.{key} is required")))?;
        require_schema_integer(property_schema, "minLength", 1)?;
    }
    require_schema_string(properties, "research_snapshot", "type", "string")?;
    require_schema_string(properties, "research_snapshot", "format", "date")?;
    let snapshot_schema = properties
        .get("research_snapshot")
        .ok_or_else(|| ValidationError::new("schema.properties.research_snapshot is required"))?;
    require_schema_integer(snapshot_schema, "minLength", 10)?;
    require_schema_string(properties, "tracking_issue", "format", "uri")?;
    require_schema_string(properties, "case_count", "type", "integer")?;
    require_string_array_schema(
        properties.get("required_run_identity").ok_or_else(|| {
            ValidationError::new("schema.properties.required_run_identity is required")
        })?,
        "schema.properties.required_run_identity",
    )?;
    require_string_array_schema(
        properties.get("comparison_policy").ok_or_else(|| {
            ValidationError::new("schema.properties.comparison_policy is required")
        })?,
        "schema.properties.comparison_policy",
    )?;

    let cases_schema = properties
        .get("cases")
        .ok_or_else(|| ValidationError::new("schema.properties.cases is required"))?;
    require_exact_string(cases_schema, "type", "array", "schema.properties.cases")?;
    require_schema_integer(cases_schema, "minItems", EXPECTED_CASE_COUNT)?;
    require_schema_integer(cases_schema, "maxItems", EXPECTED_CASE_COUNT)?;

    let item_schema = cases_schema
        .get("items")
        .ok_or_else(|| ValidationError::new("schema.properties.cases.items is required"))?;
    require_exact_string(
        item_schema,
        "type",
        "object",
        "schema.properties.cases.items",
    )?;
    require_schema_bool(
        item_schema,
        "additionalProperties",
        false,
        "schema.properties.cases.items",
    )?;
    require_schema_required_fields(item_schema, CASE_KEYS, "schema.properties.cases.items")?;

    let item_properties = item_schema
        .get("properties")
        .ok_or_else(|| ValidationError::new("schema cases.items.properties is required"))?;
    let item_property_object = item_properties
        .as_object()
        .ok_or_else(|| ValidationError::new("schema cases.items.properties must be an object"))?;
    check_exact_keys(
        item_property_object,
        CASE_KEYS,
        "schema cases.items.properties",
    )?;

    require_schema_string(item_properties, "id", "type", "string")?;
    require_schema_string(item_properties, "id", "pattern", "^[a-z][a-z0-9_]*$")?;
    for key in ["kind", "setup", "perturbation", "reason"] {
        require_schema_string(item_properties, key, "type", "string")?;
        let property_schema = item_properties.get(key).ok_or_else(|| {
            ValidationError::new(format!("schema case property {key} is required"))
        })?;
        require_schema_integer(property_schema, "minLength", 1)?;
    }

    let expected_schema = item_properties
        .get("expected")
        .ok_or_else(|| ValidationError::new("schema case property expected is required"))?;
    require_exact_string(
        expected_schema,
        "type",
        "object",
        "schema case property expected",
    )?;
    require_schema_bool(
        expected_schema,
        "additionalProperties",
        false,
        "schema case property expected",
    )?;
    require_schema_required_fields(
        expected_schema,
        EXPECTED_KEYS,
        "schema case property expected",
    )?;
    let expected_properties = expected_schema
        .get("properties")
        .ok_or_else(|| ValidationError::new("schema expected.properties is required"))?;
    let expected_property_object = expected_properties
        .as_object()
        .ok_or_else(|| ValidationError::new("schema expected.properties must be an object"))?;
    check_exact_keys(
        expected_property_object,
        EXPECTED_KEYS,
        "schema expected.properties",
    )?;
    require_string_array_schema(
        expected_properties
            .get("required_properties")
            .ok_or_else(|| ValidationError::new("schema required_properties is required"))?,
        "schema expected.required_properties",
    )?;
    require_string_array_schema(
        item_properties
            .get("fail_if")
            .ok_or_else(|| ValidationError::new("schema case property fail_if is required"))?,
        "schema case.fail_if",
    )?;

    Ok(report)
}

fn require_schema_bool(
    value: &Value,
    key: &str,
    expected: bool,
    path: &str,
) -> Result<(), ValidationError> {
    let actual = value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be boolean")))?;
    if actual != expected {
        return Err(ValidationError::new(format!(
            "{path}.{key} must be {expected}"
        )));
    }
    Ok(())
}

fn require_schema_required_fields(
    value: &Value,
    expected: &[&str],
    path: &str,
) -> Result<(), ValidationError> {
    let array = value
        .get("required")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new(format!("{path}.required must be an array")))?;
    let mut actual = BTreeSet::new();
    for item in array {
        let name = item
            .as_str()
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                ValidationError::new(format!("{path}.required entries must be nonempty strings"))
            })?;
        if !actual.insert(name) {
            return Err(ValidationError::new(format!(
                "{path}.required contains duplicate entry {name}"
            )));
        }
    }
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected_set {
        return Err(ValidationError::new(format!(
            "{path}.required does not match the frozen required-field set"
        )));
    }
    Ok(())
}

fn require_string_array_schema(value: &Value, path: &str) -> Result<(), ValidationError> {
    require_exact_string(value, "type", "array", path)?;
    require_schema_integer(value, "minItems", 1)?;
    require_schema_bool(value, "uniqueItems", true, path)?;
    let items = value
        .get("items")
        .ok_or_else(|| ValidationError::new(format!("{path}.items is required")))?;
    require_exact_string(items, "type", "string", &format!("{path}.items"))?;
    require_schema_integer(items, "minLength", 1)?;
    Ok(())
}

fn is_valid_iso_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let year = match date[..4].parse::<u32>() {
        Ok(value) if value > 0 => value,
        _ => return false,
    };
    let month = match date[5..7].parse::<u32>() {
        Ok(value) if (1..=12).contains(&value) => value,
        _ => return false,
    };
    let day = match date[8..].parse::<u32>() {
        Ok(value) => value,
        _ => return false,
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=month_days).contains(&day)
}

fn check_exact_keys(
    object: &serde_json::Map<String, Value>,
    expected: &[&str],
    path: &str,
) -> Result<(), ValidationError> {
    for key in expected {
        if !object.contains_key(*key) {
            return Err(ValidationError::new(format!(
                "{path} is missing required key {key}"
            )));
        }
    }
    for key in object.keys() {
        if !expected.contains(&key.as_str()) {
            return Err(ValidationError::new(format!(
                "{path} has unexpected key {key}"
            )));
        }
    }
    Ok(())
}

fn required_string<'a>(
    value: &'a Value,
    key: &str,
    path: &str,
) -> Result<&'a str, ValidationError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be a nonempty string")))
}

fn require_nonempty_string(value: &Value, key: &str, path: &str) -> Result<(), ValidationError> {
    required_string(value, key, path).map(|_| ())
}

fn require_exact_string(
    value: &Value,
    key: &str,
    expected: &str,
    path: &str,
) -> Result<(), ValidationError> {
    let actual = required_string(value, key, path)?;
    if actual != expected {
        return Err(ValidationError::new(format!(
            "{path}.{key} must be {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn require_unique_nonempty_string_array(
    value: &Value,
    key: &str,
    path: &str,
    required_members: &[&str],
) -> Result<(), ValidationError> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError::new(format!("{path}.{key} must be an array")))?;
    if array.is_empty() {
        return Err(ValidationError::new(format!(
            "{path}.{key} must not be empty"
        )));
    }

    let mut members = BTreeSet::new();
    for (index, item) in array.iter().enumerate() {
        let string = item
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| {
                ValidationError::new(format!("{path}.{key}[{index}] must be a nonempty string"))
            })?;
        if !members.insert(string) {
            return Err(ValidationError::new(format!(
                "{path}.{key} contains duplicate entry {string}"
            )));
        }
    }
    for required in required_members {
        if !members.contains(required) {
            return Err(ValidationError::new(format!(
                "{path}.{key} is missing required entry {required}"
            )));
        }
    }
    Ok(())
}

fn require_schema_const(
    properties: &Value,
    key: &str,
    expected: &str,
) -> Result<(), ValidationError> {
    let actual = properties
        .get(key)
        .and_then(|value| value.get("const"))
        .ok_or_else(|| {
            ValidationError::new(format!("schema.properties.{key}.const is required"))
        })?;
    if actual.as_str() != Some(expected)
        && !(key == "case_count" && actual.as_u64() == Some(EXPECTED_CASE_COUNT as u64))
    {
        return Err(ValidationError::new(format!(
            "schema.properties.{key}.const does not pin the expected contract"
        )));
    }
    Ok(())
}

fn require_schema_integer(
    value: &Value,
    key: &str,
    expected: usize,
) -> Result<(), ValidationError> {
    let actual = value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| ValidationError::new(format!("schema {key} must be an integer")))?;
    if actual != expected as u64 {
        return Err(ValidationError::new(format!(
            "schema {key} must be {expected}, got {actual}"
        )));
    }
    Ok(())
}

fn require_schema_string(
    properties: &Value,
    key: &str,
    constraint: &str,
    expected: &str,
) -> Result<(), ValidationError> {
    let actual = properties
        .get(key)
        .and_then(|value| value.get(constraint))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ValidationError::new(format!(
                "schema property {key} is missing string constraint {constraint}"
            ))
        })?;
    if actual != expected {
        return Err(ValidationError::new(format!(
            "schema property {key}.{constraint} must be {expected}"
        )));
    }
    Ok(())
}

fn is_snake_case_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn committed_oracle_and_schema_are_structurally_consistent() {
        let report = validate_artifacts(COMMITTED_ORACLE_JSON, COMMITTED_SCHEMA_JSON)
            .expect("committed benchmark artifacts validate");
        assert_eq!(report.schema_version, ORACLE_SCHEMA_VERSION);
        assert_eq!(report.case_count, EXPECTED_CASE_COUNT);
        assert_eq!(report.unique_case_ids, EXPECTED_CASE_COUNT);
    }

    #[test]
    fn duplicate_case_id_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        let first_id = oracle["cases"][0]["id"]
            .as_str()
            .expect("first case has an ID")
            .to_owned();
        oracle["cases"][1]["id"] = json!(first_id);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn declared_case_count_must_match_frozen_version() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["case_count"] = json!(50);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn missing_failure_condition_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["cases"][0]["fail_if"] = json!([]);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn undeclared_case_field_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["cases"][0]["unexpected"] = json!(true);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn invalid_case_id_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["cases"][0]["id"] = json!("Animal Case One");
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn omitted_run_identity_requirement_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["required_run_identity"] = json!(["source_commit"]);
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn weakened_schema_case_count_fails_closed() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["properties"]["case_count"]["const"] = json!(50);
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn schema_cannot_allow_extra_case_fields() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["properties"]["cases"]["items"]["additionalProperties"] = json!(true);
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn schema_cannot_drop_a_required_root_field() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["required"]
            .as_array_mut()
            .expect("required is an array")
            .retain(|value| value.as_str() != Some("cases"));
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn schema_cannot_weaken_failure_condition_requirement() {
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        schema["properties"]["cases"]["items"]["properties"]["fail_if"]["minItems"] = json!(0);
        let input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(COMMITTED_ORACLE_JSON, &input).is_err());
    }

    #[test]
    fn invalid_calendar_date_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["research_snapshot"] = json!("2026-02-30");
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn wrong_tracking_issue_fails_closed() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        oracle["tracking_issue"] = json!("https://github.com/Luminous-Dynamics/symtropy/issues/1");
        let input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        assert!(validate_oracle(&input).is_err());
    }

    #[test]
    fn oracle_and_schema_cannot_jointly_redefine_v1_case_count() {
        let mut oracle: Value =
            serde_json::from_str(COMMITTED_ORACLE_JSON).expect("oracle fixture parses");
        let mut schema: Value =
            serde_json::from_str(COMMITTED_SCHEMA_JSON).expect("schema fixture parses");
        oracle["case_count"] = json!(52);
        schema["properties"]["case_count"]["const"] = json!(52);
        schema["properties"]["cases"]["minItems"] = json!(52);
        schema["properties"]["cases"]["maxItems"] = json!(52);
        let oracle_input = serde_json::to_string(&oracle).expect("mutated oracle serializes");
        let schema_input = serde_json::to_string(&schema).expect("mutated schema serializes");
        assert!(validate_artifacts(&oracle_input, &schema_input).is_err());
    }
}
