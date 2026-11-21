// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline validation and independent semantic checks for Foundry architecture profiles.
//!
//! The JSON Schema validator has remote and file reference resolution disabled.
//! The benchmark schema is self-contained and must remain self-contained: validation
//! must never perform implicit network or filesystem reference retrieval.

use jsonschema::draft202012;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSummary {
    pub profile_id: String,
    pub layer_count: usize,
    pub source_class: String,
    pub execution_scope: String,
}

pub fn parse_json(bytes: &[u8], label: &str) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("{label}: invalid JSON: {error}"))
}

pub fn validate_profile(schema: &Value, profile: &Value) -> Result<ProfileSummary, Vec<String>> {
    if schema.get("$schema").and_then(Value::as_str) != Some(DRAFT_2020_12) {
        return Err(vec![format!(
            "schema: expected JSON Schema Draft 2020-12 identifier {DRAFT_2020_12}"
        )]);
    }

    if let Err(error) = draft202012::meta::validate(schema) {
        return Err(vec![format!(
            "schema: invalid Draft 2020-12 schema: {error}"
        )]);
    }

    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| vec![format!("schema: validator construction failed: {error}")])?;

    let mut errors: Vec<String> = validator
        .iter_errors(profile)
        .map(|error| format!("schema{}: {error}", error.instance_path()))
        .collect();

    errors.extend(validate_semantics(profile));
    if !errors.is_empty() {
        return Err(errors);
    }

    let profile_id = profile
        .get("profile_id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let layer_count = profile
        .get("layers")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let source_class = profile
        .get("source_class")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    let execution_scope = profile
        .get("execution_scope")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();

    Ok(ProfileSummary {
        profile_id,
        layer_count,
        source_class,
        execution_scope,
    })
}

fn string_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn bool_at(value: &Value, pointer: &str) -> Option<bool> {
    value.pointer(pointer).and_then(Value::as_bool)
}

fn strings_at(value: &Value, pointer: &str) -> BTreeSet<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn require_read_only(value: &Value, pointer: &str, errors: &mut Vec<String>) {
    if bool_at(value, &format!("{pointer}/read")) != Some(true) {
        errors.push(format!("{pointer}: read must be true"));
    }
    for field in ["write", "command", "product_release"] {
        if bool_at(value, &format!("{pointer}/{field}")) != Some(false) {
            errors.push(format!("{pointer}/{field}: must be false"));
        }
    }
}

fn validate_semantics(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    require_read_only(profile, "/access_contract", &mut errors);

    let execution_scope = string_at(profile, "/execution_scope").unwrap_or_default();
    let source_class = string_at(profile, "/source_class").unwrap_or_default();
    let evaluation_status = string_at(profile, "/evaluation_status").unwrap_or_default();
    let source_refs = profile
        .pointer("/input_contract/source_refs")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    match execution_scope {
        "reference_only" => {
            if source_class != "none" || source_refs != 0 {
                errors.push(
                    "reference_only: source_class must be none and source_refs must be empty"
                        .to_owned(),
                );
            }
            if evaluation_status != "not_run" {
                errors.push(
                    "reference_only: evaluation_status must remain not_run until an implementation is executed"
                        .to_owned(),
                );
            }
        }
        "synthetic_fixture" => {
            if source_class != "synthetic" || source_refs == 0 {
                errors.push(
                    "synthetic_fixture: source_class must be synthetic and source_refs must be non-empty"
                        .to_owned(),
                );
            }
            if matches!(
                evaluation_status,
                "conformance_tested" | "independently_evaluated"
            ) {
                errors.push(
                    "synthetic_fixture: cannot claim conformance_tested or independently_evaluated"
                        .to_owned(),
                );
            }
        }
        "recorded_replay" | "live_read_only" => {
            if source_class == "none" || source_refs == 0 {
                errors.push(format!(
                    "{execution_scope}: source_class and source_refs must identify admitted inputs"
                ));
            }
            if execution_scope == "live_read_only"
                && bool_at(profile, "/access_contract/subscribe") != Some(true)
            {
                errors.push("live_read_only: subscribe must be explicitly declared".to_owned());
            }
        }
        _ => errors.push(format!("unsupported execution_scope {execution_scope:?}")),
    }

    let mut reference_roles = BTreeMap::new();
    let mut reference_ids = BTreeSet::new();
    if let Some(references) = profile.get("standard_references").and_then(Value::as_array) {
        for (index, reference) in references.iter().enumerate() {
            let id = string_at(reference, "/standard_ref_id").unwrap_or_default();
            let role = string_at(reference, "/role").unwrap_or_default();
            if !reference_ids.insert(id.to_owned()) {
                errors.push(format!(
                    "/standard_references/{index}/standard_ref_id: duplicate ID {id:?}"
                ));
            }
            reference_roles.insert(id.to_owned(), role.to_owned());
        }
    }

    if let Some(references) = profile.get("standard_references").and_then(Value::as_array) {
        for (index, reference) in references.iter().enumerate() {
            let Some(artifact) = reference.get("nodeset_artifact") else {
                continue;
            };
            let base = format!("/standard_references/{index}/nodeset_artifact");
            let namespace_uri =
                string_at(reference, "/namespace_metadata/namespace_uri").unwrap_or_default();
            let namespace_version =
                string_at(reference, "/namespace_metadata/namespace_version").unwrap_or_default();
            let model_uri = string_at(artifact, "/node_set_model/model_uri").unwrap_or_default();
            let model_version = string_at(artifact, "/node_set_model/version").unwrap_or_default();
            if model_uri != namespace_uri {
                errors.push(format!(
                    "{base}/node_set_model/model_uri: must match NamespaceMetadata URI"
                ));
            }
            if model_version != namespace_version {
                errors.push(format!(
                    "{base}/node_set_model/version: must match NamespaceMetadata NamespaceVersion"
                ));
            }
            let uri_list = strings_at(artifact, "/namespace_uris");
            if uri_list
                .iter()
                .filter(|uri| uri.as_str() == namespace_uri)
                .count()
                != 1
            {
                errors.push(format!(
                    "{base}/namespace_uris: expected NamespaceMetadata URI exactly once"
                ));
            }
        }
    }

    let mut layer_ids = BTreeSet::new();
    if let Some(layers) = profile.get("layers").and_then(Value::as_array) {
        for (index, layer) in layers.iter().enumerate() {
            let base = format!("/layers/{index}");
            let layer_id = string_at(layer, "/layer_id").unwrap_or_default();
            if !layer_ids.insert(layer_id.to_owned()) {
                errors.push(format!("{base}/layer_id: duplicate ID {layer_id:?}"));
            }

            require_read_only(layer, "/access_contract", &mut errors);

            let standard_ref_id = string_at(layer, "/standard_ref_id").unwrap_or_default();
            match reference_roles.get(standard_ref_id) {
                None => errors.push(format!(
                    "{base}/standard_ref_id: unresolved reference {standard_ref_id:?}"
                )),
                Some(role) => {
                    let layer_kind = string_at(layer, "/layer_kind").unwrap_or_default();
                    if role != layer_kind {
                        errors.push(format!(
                            "{base}/layer_kind: {layer_kind:?} does not match standard reference role {role:?}"
                        ));
                    }
                }
            }

            let implementation_status =
                string_at(layer, "/implementation/implementation_status").unwrap_or_default();
            let admission_status = string_at(layer, "/admission/status").unwrap_or_default();
            let preserved = strings_at(layer, "/semantic_contract/preserved_fields");
            let required = strings_at(layer, "/semantic_contract/required_fields_to_preserve");
            let unsupported = strings_at(layer, "/semantic_contract/unsupported_fields");

            if !preserved.is_subset(&required) {
                errors.push(format!(
                    "{base}/semantic_contract/preserved_fields: a preserved field is not declared in required_fields_to_preserve"
                ));
            }

            if let Some(field) = preserved.intersection(&unsupported).next() {
                errors.push(format!(
                    "{base}/semantic_contract: field {field:?} is both preserved and unsupported"
                ));
            }

            if bool_at(layer, "/semantic_contract/source_values_immutable") != Some(true) {
                errors.push(format!(
                    "{base}/semantic_contract/source_values_immutable: must be true"
                ));
            }

            if implementation_status == "not_implemented" && !preserved.is_empty() {
                errors.push(format!(
                    "{base}/semantic_contract/preserved_fields: must be empty while implementation_status is not_implemented"
                ));
            }

            if implementation_status == "not_implemented"
                && matches!(
                    admission_status,
                    "official_conformance_tested" | "independently_verified"
                )
            {
                errors.push(format!(
                    "{base}/admission/status: cannot claim implementation conformance or verification when implementation_status is not_implemented"
                ));
            }

            if execution_scope == "reference_only"
                && (implementation_status != "not_implemented"
                    || admission_status != "source_reviewed")
            {
                errors.push(format!(
                    "{base}: reference_only profiles require not_implemented + source_reviewed"
                ));
            }

            if execution_scope == "synthetic_fixture"
                && matches!(
                    admission_status,
                    "official_conformance_tested" | "independently_verified"
                )
            {
                errors.push(format!(
                    "{base}/admission/status: synthetic fixture cannot claim official conformance or independent physical verification"
                ));
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!("../../profile.schema.json");
    const MQTT: &str = include_str!("../../profiles/uns-mqtt.synthetic.json");
    const I3X: &str = include_str!("../../profiles/i3x-1.0.reference.json");
    const OPC_MACHINERY: &str =
        include_str!("../../profiles/opcua-machinery-1.04.2.reference.json");
    const OPC_ROBOTICS: &str = include_str!("../../profiles/opcua-robotics-1.02.reference.json");

    fn schema() -> Value {
        serde_json::from_str(SCHEMA).expect("schema JSON must parse")
    }

    fn parse_profile(source: &str) -> Value {
        serde_json::from_str(source).expect("profile JSON must parse")
    }

    fn assert_valid(profile: &Value) {
        let summary = validate_profile(&schema(), profile)
            .unwrap_or_else(|errors| panic!("profile unexpectedly invalid: {errors:#?}"));
        assert!(!summary.profile_id.is_empty());
        assert!(summary.layer_count > 0);
    }

    fn assert_invalid(profile: &Value, expected_fragment: &str) {
        let errors =
            validate_profile(&schema(), profile).expect_err("mutated profile must fail validation");
        let joined = errors.join("\n");
        assert!(
            joined.contains(expected_fragment),
            "expected error containing {expected_fragment:?}, got: {joined}"
        );
    }

    #[test]
    fn schema_is_valid_draft_2020_12() {
        let value = schema();
        draft202012::meta::validate(&value)
            .expect("profile schema must pass Draft 2020-12 meta-schema validation");
    }

    #[test]
    fn all_declared_profiles_validate() {
        assert_valid(&parse_profile(MQTT));
        assert_valid(&parse_profile(I3X));
        assert_valid(&parse_profile(OPC_MACHINERY));
        assert_valid(&parse_profile(OPC_ROBOTICS));
    }

    #[test]
    fn rejects_undeclared_intelligence_scope() {
        let mut profile = parse_profile(MQTT);
        profile["layers"][0]["intelligence_scope"] = json!("global_factory_superintelligence");
        assert_invalid(&profile, "intelligence_scope");
    }

    #[test]
    fn rejects_nodeset_artifact_namespace_or_version_mismatch() {
        let mut profile = parse_profile(OPC_MACHINERY);
        profile["standard_references"][0]["nodeset_artifact"]["node_set_model"]["version"] =
            json!("1.04.1");
        assert_invalid(&profile, "must match NamespaceMetadata NamespaceVersion");

        let mut wrong_uri = parse_profile(OPC_MACHINERY);
        wrong_uri["standard_references"][0]["nodeset_artifact"]["node_set_model"]["model_uri"] =
            json!("http://vendor.example/machinery/");
        assert_invalid(&wrong_uri, "must match NamespaceMetadata URI");
    }

    #[test]
    fn rejects_profile_write_access() {
        let mut profile = parse_profile(MQTT);
        profile["access_contract"]["write"] = json!(true);
        assert_invalid(&profile, "write");
    }

    #[test]
    fn rejects_layer_command_access() {
        let mut profile = parse_profile(MQTT);
        profile["layers"][0]["access_contract"]["command"] = json!(true);
        assert_invalid(&profile, "command");
    }

    #[test]
    fn rejects_duplicate_layer_identity() {
        let mut profile = parse_profile(MQTT);
        let mut second = profile["layers"][0].clone();
        second["standard_ref_id"] = json!("mqtt-5.0");
        profile["layers"].as_array_mut().unwrap().push(second);
        assert_invalid(&profile, "duplicate ID");
    }

    #[test]
    fn rejects_unresolved_standard_reference() {
        let mut profile = parse_profile(MQTT);
        profile["layers"][0]["standard_ref_id"] = json!("missing-standard");
        assert_invalid(&profile, "unresolved reference");
    }

    #[test]
    fn rejects_layer_kind_reference_role_mismatch() {
        let mut profile = parse_profile(MQTT);
        profile["layers"][0]["layer_kind"] = json!("machine_information_model");
        assert_invalid(&profile, "does not match standard reference role");
    }

    #[test]
    fn rejects_conflicting_preserved_and_unsupported_field() {
        let mut profile = parse_profile(MQTT);
        let preserved = profile["layers"][0]["semantic_contract"]["preserved_fields"]
            .as_array_mut()
            .unwrap();
        preserved.push(json!("wire_qos_delivery_semantics"));
        profile["layers"][0]["semantic_contract"]["required_fields_to_preserve"]
            .as_array_mut()
            .unwrap()
            .push(json!("wire_qos_delivery_semantics"));
        assert_invalid(&profile, "both preserved and unsupported");
    }

    #[test]
    fn rejects_reference_profile_claiming_implementation() {
        let mut profile = parse_profile(I3X);
        profile["layers"][0]["implementation"]["implementation_status"] = json!("implemented");
        assert_invalid(&profile, "reference_only profiles require");
    }

    #[test]
    fn rejects_reference_profile_with_recorded_source() {
        let mut profile = parse_profile(I3X);
        profile["source_class"] = json!("recorded");
        profile["input_contract"]["source_class"] = json!("recorded");
        profile["input_contract"]["source_refs"] = json!(["unadmitted://source"]);
        assert_invalid(&profile, "reference_only: source_class must be none");
    }

    #[test]
    fn rejects_synthetic_profile_promotion_to_conformance() {
        let mut profile = parse_profile(MQTT);
        profile["evaluation_status"] = json!("conformance_tested");
        profile["layers"][0]["admission"]["status"] = json!("official_conformance_tested");
        assert_invalid(
            &profile,
            "synthetic_fixture: cannot claim conformance_tested",
        );
    }

    #[test]
    fn rejects_preserved_fields_not_declared_as_required() {
        let mut profile = parse_profile(MQTT);
        profile["layers"][0]["semantic_contract"]["required_fields_to_preserve"] = json!([]);
        assert_invalid(
            &profile,
            "preserved_fields: a preserved field is not declared",
        );
    }

    #[test]
    fn rejects_duplicate_standard_reference_ids() {
        let mut profile = parse_profile(MQTT);
        let mut second = profile["standard_references"][0].clone();
        second["name"] = json!("Duplicated MQTT reference");
        profile["standard_references"]
            .as_array_mut()
            .unwrap()
            .push(second);
        assert_invalid(&profile, "duplicate ID");
    }

    #[test]
    fn rejects_unverified_profile_claim_when_no_implementation_exists() {
        let mut profile = parse_profile(OPC_MACHINERY);
        profile["layers"][0]["admission"]["status"] = json!("independently_verified");
        assert_invalid(&profile, "cannot claim implementation conformance");
    }
}

pub mod semantic_evaluator;

pub mod inspection_evaluator;

pub mod nodeset_admission;

pub mod bench_metrics;
