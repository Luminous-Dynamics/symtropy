// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use jsonschema::draft202012;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

fn built_environment_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../docs/built-environment")
}

fn schema_and_example() -> (Value, Value) {
    let root = built_environment_root();
    let schema: Value = serde_json::from_slice(
        &fs::read(root.join("home-design-brief.schema.json"))
            .expect("home design brief schema exists"),
    )
    .expect("home design brief schema parses as JSON");
    let example: Value = serde_json::from_slice(
        &fs::read(root.join("fixtures/example-home-design-brief.json"))
            .expect("synthetic home design brief fixture exists"),
    )
    .expect("home design brief fixture parses as JSON");
    (schema, example)
}

fn validation_errors(schema: &Value, instance: &Value) -> Vec<String> {
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .expect("home design brief schema compiles");
    validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect()
}

fn all_refs_are_local(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().all(|(key, child)| {
            (key != "$ref"
                || child
                    .as_str()
                    .is_some_and(|reference| reference.starts_with("#/")))
                && all_refs_are_local(child)
        }),
        Value::Array(values) => values.iter().all(all_refs_are_local),
        _ => true,
    }
}

#[test]
fn home_design_brief_schema_is_valid_draft_2020_12() {
    let (schema, _) = schema_and_example();
    assert!(
        draft202012::meta::validate(&schema).is_ok(),
        "schema must pass Draft 2020-12 meta-validation"
    );
    assert!(
        all_refs_are_local(&schema),
        "schema must not require remote $ref retrieval"
    );
}

#[test]
fn synthetic_home_design_brief_fixture_is_valid() {
    let (schema, example) = schema_and_example();
    let errors = validation_errors(&schema, &example);
    assert!(
        errors.is_empty(),
        "synthetic fixture failed schema validation: {errors:#?}"
    );
    assert_eq!(example["schema_version"], "0.1.0");
    assert_eq!(
        example["conflict_policy"]["on_conflict"],
        "preserve_and_report"
    );
}

#[test]
fn unknown_properties_and_attempted_safety_override_are_rejected() {
    let (schema, mut example) = schema_and_example();
    example["safety_override"] = json!(true);
    assert!(
        !validation_errors(&schema, &example).is_empty(),
        "unknown top-level fields must fail closed"
    );

    let (schema, mut example) = schema_and_example();
    example["conflict_policy"]["cannot_override_safety_or_law"] = json!(false);
    assert!(
        !validation_errors(&schema, &example).is_empty(),
        "conflict policy must not be weakened by a brief"
    );
}

#[test]
fn referenced_dimension_constraint_requires_specific_source_metadata() {
    let (schema, mut example) = schema_and_example();
    example["spaces"][0]["dimension_constraints"][0]["origin"] = json!("authority_reference");
    example["spaces"][0]["dimension_constraints"][0]["source_reference"] = Value::Null;
    example["spaces"][0]["dimension_constraints"][0]["source_edition"] = Value::Null;
    example["spaces"][0]["dimension_constraints"][0]["source_location"] = Value::Null;
    assert!(
        !validation_errors(&schema, &example).is_empty(),
        "authority-origin dimension constraints need a reference, edition, and location"
    );

    example["spaces"][0]["dimension_constraints"][0]["source_reference"] =
        json!("https://example.invalid/fixture-source");
    example["spaces"][0]["dimension_constraints"][0]["source_edition"] =
        json!("fixture-edition-1");
    example["spaces"][0]["dimension_constraints"][0]["source_location"] =
        json!("fixture-location-1");
    assert!(
        validation_errors(&schema, &example).is_empty(),
        "explicit fixture metadata should satisfy the schema without authenticating the source"
    );
}

#[test]
fn out_of_range_priority_is_rejected() {
    let (schema, mut example) = schema_and_example();
    example["priorities"][0]["importance"] = json!(6);
    let errors = validation_errors(&schema, &example);
    assert!(!errors.is_empty(), "importance outside 1..=5 must fail");
}

#[test]
fn authority_requirement_requires_a_source_pointer() {
    let (schema, mut example) = schema_and_example();
    example["requirements"][0]["origin"] = json!("authority_reference");
    example["requirements"][0]["source_reference"] = Value::Null;
    let errors = validation_errors(&schema, &example);
    assert!(
        !errors.is_empty(),
        "authority-origin requirement without a source pointer must fail"
    );
}

#[test]
fn professional_or_authority_review_requires_separate_evidence_reference() {
    let (schema, mut example) = schema_and_example();
    example["requirements"][0]["verification_state"] = json!("authority_confirmed");
    example["requirements"][0]["verification_evidence_refs"] = json!([]);
    assert!(
        !validation_errors(&schema, &example).is_empty(),
        "authority confirmation without an evidence reference must fail"
    );
}
