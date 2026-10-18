// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Value, json};

const SCHEMA: &str =
    include_str!("../../../docs/schemas/peacebuilding-outcome-governance-v0.2.schema.json");
const EXAMPLE: &str =
    include_str!("../../../docs/fixtures/peacebuilding-outcome-governance-v0.2.json");

fn parsed_schema() -> Value {
    serde_json::from_str(SCHEMA).expect("checked-in v0.2 JSON Schema must parse")
}

fn example_contract() -> Value {
    serde_json::from_str(EXAMPLE).expect("checked-in v0.2 example must parse")
}

fn validation_errors(instance: &Value) -> Vec<String> {
    let validator = jsonschema::draft202012::new(&parsed_schema())
        .expect("checked-in v0.2 schema must compile");
    validator
        .iter_errors(instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect()
}

#[test]
fn schema_validates_against_draft_2020_12_meta_schema() {
    let schema = parsed_schema();
    let result = jsonschema::draft202012::meta::validate(&schema);
    assert!(result.is_ok(), "schema meta-validation failed: {result:?}");
}

#[test]
fn checked_in_synthetic_example_validates() {
    let errors = validation_errors(&example_contract());
    assert!(errors.is_empty(), "example rejected: {errors:#?}");
}

#[test]
fn active_metric_requires_approved_authority_and_timestamp() {
    let mut value = example_contract();
    value["approval"]["state"] = json!("contested");
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "active metric cannot use contested approval"
    );
}

#[test]
fn active_metric_requires_assessed_geographic_scope() {
    let mut value = example_contract();
    value["geographic_scope"]["coverage_complete"] = json!(false);
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "active metric cannot hide unassessed scope"
    );
}

#[test]
fn post_observation_amendment_requires_prior_version_and_reason() {
    let mut value = example_contract();
    value["version_control"]["post_observation_amendment"] = json!(true);
    value["version_control"]["outcomes_observed"] = json!(false);
    value["version_control"]["predecessor_version"] = Value::Null;
    value["version_control"]["amendment_reason"] = Value::Null;
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "post-observation change must be versioned and explained"
    );
}

#[test]
fn second_metric_version_requires_predecessor() {
    let mut value = example_contract();
    value["version"] = json!(2);
    value["version_control"]["predecessor_version"] = Value::Null;
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "version 2 cannot omit predecessor"
    );
}

#[test]
fn person_level_sensitive_fields_are_rejected() {
    let mut value = example_contract();
    value["political_affiliation"] = json!("not allowed");
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "unknown identity-bearing fields must be rejected"
    );
}

#[test]
fn unknown_metric_status_is_rejected() {
    let mut value = example_contract();
    value["status"] = json!("success");
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "unregistered lifecycle states must be rejected"
    );
}

#[test]
fn malformed_schema_fails_meta_validation() {
    let mut schema = parsed_schema();
    schema["properties"]["status"]["enum"] = json!("active");
    assert!(
        jsonschema::draft202012::meta::validate(&schema).is_err(),
        "invalid JSON Schema must fail closed"
    );
}

#[test]
fn active_metric_requires_preregistration_reference() {
    let mut value = example_contract();
    value["version_control"]["preregistration_reference"] = Value::Null;
    let errors = validation_errors(&value);
    assert!(
        !errors.is_empty(),
        "active metric must bind to preregistration"
    );
}
