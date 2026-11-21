// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent, technology-neutral semantic comparison for portable evidence projections.
//!
//! The evaluator does not call candidate parsers, mappers, API clients or production logic.
//! Evaluator truth is supplied by the benchmark harness and is not embedded in release builds.

use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldDisposition {
    Match,
    MissingFromOracle,
    MissingFromCandidate,
    Mismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldResult {
    pub field: String,
    pub disposition: FieldDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticComparison {
    pub equivalent_on_required_fields: bool,
    pub compared_fields: usize,
    pub matched_fields: usize,
    pub results: Vec<FieldResult>,
    /// Additional candidate keys are surfaced rather than silently discarded.
    pub additional_candidate_fields: Vec<String>,
}

/// Compare a candidate projection against evaluator-owned truth.
///
/// This function intentionally does not normalize units, timestamps, identifiers or values.
/// Each comparison is exact for the declared fixture contract. Any tolerance or transformation
/// must be specified separately and must retain the native source value and its provenance.
pub fn compare_required_fields(
    oracle: &Value,
    candidate: &Value,
    required_fields: &[&str],
) -> Result<SemanticComparison, String> {
    if !oracle.is_object() || !candidate.is_object() {
        return Err("oracle and candidate projections must both be JSON objects".to_owned());
    }
    if required_fields.is_empty() {
        return Err("required_fields must not be empty".to_owned());
    }

    let mut seen = BTreeSet::new();
    for field in required_fields {
        if field.trim().is_empty() {
            return Err("required_fields contains an empty field name".to_owned());
        }
        if !seen.insert(*field) {
            return Err(format!(
                "required_fields contains duplicate field {field:?}"
            ));
        }
    }

    let mut results = Vec::with_capacity(required_fields.len());
    let mut matched_fields = 0usize;
    for field in required_fields {
        let disposition = match (oracle.get(*field), candidate.get(*field)) {
            (None, _) => FieldDisposition::MissingFromOracle,
            (Some(_), None) => FieldDisposition::MissingFromCandidate,
            (Some(expected), Some(actual)) if expected == actual => {
                matched_fields += 1;
                FieldDisposition::Match
            }
            (Some(_), Some(_)) => FieldDisposition::Mismatch,
        };
        results.push(FieldResult {
            field: (*field).to_owned(),
            disposition,
        });
    }

    let additional_candidate_fields = candidate
        .as_object()
        .expect("candidate object checked above")
        .keys()
        .filter(|key| !seen.contains(key.as_str()))
        .cloned()
        .collect();

    Ok(SemanticComparison {
        equivalent_on_required_fields: matched_fields == required_fields.len(),
        compared_fields: required_fields.len(),
        matched_fields,
        results,
        additional_candidate_fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const CANDIDATE: &str = include_str!("../tests/fixtures/mtconnect-observation.candidate.json");
    const PROJECTION_SCHEMA: &str = include_str!("../../portable-observation.schema.json");
    const ASSET_SCHEMA: &str = include_str!("../../portable-asset.schema.json");
    const ASSET_CANDIDATE: &str = include_str!("../tests/fixtures/mtconnect-asset.candidate.json");

    const FIELDS: &[&str] = &[
        "projection_schema_version",
        "adapter_profile_id",
        "source_schema_version",
        "evidence_class",
        "calibration_disposition",
        "calibration_reference_id",
        "calibration_reference_sha256",
        "measurement_uncertainty_disposition",
        "measurement_uncertainty_reference_id",
        "measurement_uncertainty_reference_sha256",
        "source_document_id",
        "source_document_sha256",
        "schema_identifier",
        "device_uuid",
        "data_item_id",
        "component_id",
        "sequence",
        "source_timestamp_ns",
        "reference_timestamp_ns",
        "reference_clock_id",
        "clock_mapping_sha256",
        "clock_uncertainty_ns",
        "native_kind",
        "native_unit",
        "value_text",
        "source_locator",
        "opaque_extensions",
    ];

    fn candidate() -> Value {
        serde_json::from_str(CANDIDATE).expect("candidate fixture JSON must parse")
    }

    /// The expected values are deliberately held in the test evaluator, not in the
    /// candidate fixture. This is a finite synthetic known-answer case only.
    fn oracle() -> Value {
        json!({
            "projection_schema_version": "0.1.0",
            "adapter_profile_id": "urn:luminous:symtropy:mfg-arch:uns-mqtt-synthetic-v0.1",
            "source_schema_version": "2.8",
            "evidence_class": "synthetic",
            "calibration_disposition": "unknown",
            "calibration_reference_id": null,
            "calibration_reference_sha256": null,
            "measurement_uncertainty_disposition": "unknown",
            "measurement_uncertainty_reference_id": null,
            "measurement_uncertainty_reference_sha256": null,
            "source_document_id": "MT-DOC-0001",
            "source_document_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "schema_identifier": "https://schemas.mtconnect.org/schemas/MTConnectStreams_2.8.schema.json",
            "device_uuid": "urn:uuid:machine-001",
            "data_item_id": "DID-000",
            "component_id": "SPINDLE-01",
            "sequence": 10,
            "source_timestamp_ns": 1750000100,
            "reference_timestamp_ns": 1750000100,
            "reference_clock_id": "PLANT-UTC-001",
            "clock_mapping_sha256": "6000000000000000000000000000000000000000000000000000000000000000",
            "clock_uncertainty_ns": 100,
            "native_kind": "SAMPLE",
            "native_unit": "rpm",
            "value_text": "1.20000000000000000e3",
            "source_locator": "/MTConnectStreams/Streams/ComponentStream/Observations/Observation[0]",
            "opaque_extensions": [{
                "namespace": "urn:vendor:extension:v1",
                "local_name": "VendorTrace",
                "raw_payload_sha256": "3000000000000000000000000000000000000000000000000000000000000000",
                "source_locator": "/Document/Extensions/VendorTrace[1]"
            }]
        })
    }

    #[test]
    fn candidate_projection_validates_against_strict_schema() {
        let schema: Value =
            serde_json::from_str(PROJECTION_SCHEMA).expect("projection schema JSON must parse");
        jsonschema::draft202012::meta::validate(&schema)
            .expect("projection schema must be valid Draft 2020-12");
        let validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("projection validator must build");
        assert!(validator.is_valid(&candidate()));
    }

    #[test]
    fn projection_schema_rejects_negative_sequence_and_unknown_root_fields() {
        let schema: Value =
            serde_json::from_str(PROJECTION_SCHEMA).expect("projection schema JSON must parse");
        let validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("projection validator must build");
        let mut bad_sequence = candidate();
        bad_sequence["sequence"] = json!(-1);
        assert!(!validator.is_valid(&bad_sequence));

        let mut unknown = candidate();
        unknown["unmapped_raw_value"] = json!("must not be discarded");
        assert!(!validator.is_valid(&unknown));
    }

    #[test]
    fn qualified_calibration_requires_exact_evidence_reference() {
        let schema: Value =
            serde_json::from_str(PROJECTION_SCHEMA).expect("projection schema JSON must parse");
        let validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("projection validator must build");
        let mut actual = candidate();
        actual["calibration_disposition"] = json!("qualified_reference");
        assert!(!validator.is_valid(&actual));

        actual["calibration_reference_id"] = json!("CAL-001");
        actual["calibration_reference_sha256"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert!(validator.is_valid(&actual));
    }

    #[test]
    fn unknown_calibration_and_uncertainty_remain_explicit_not_qualified() {
        let actual = candidate();
        assert_eq!(actual["evidence_class"], json!("synthetic"));
        assert_eq!(actual["calibration_disposition"], json!("unknown"));
        assert_eq!(
            actual["measurement_uncertainty_disposition"],
            json!("unknown")
        );
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(result.equivalent_on_required_fields);
    }

    #[test]
    fn mtconnect_asset_is_a_separate_top_level_projection_not_a_stream_observation() {
        let asset_schema: Value =
            serde_json::from_str(ASSET_SCHEMA).expect("asset schema JSON must parse");
        jsonschema::draft202012::meta::validate(&asset_schema)
            .expect("asset schema must be valid Draft 2020-12");
        let asset_validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&asset_schema)
            .expect("asset schema validator must build");
        let asset: Value = serde_json::from_str(ASSET_CANDIDATE).expect("asset fixture must parse");
        assert!(asset_validator.is_valid(&asset));

        let stream_schema: Value =
            serde_json::from_str(PROJECTION_SCHEMA).expect("stream observation schema must parse");
        let stream_validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&stream_schema)
            .expect("stream schema validator must build");
        assert!(!stream_validator.is_valid(&asset));

        let mut invalid_stream = candidate();
        invalid_stream["native_kind"] = json!("ASSET");
        assert!(!stream_validator.is_valid(&invalid_stream));
    }

    fn stream_variant(kind: &str) -> Value {
        let mut value = candidate();
        value["native_kind"] = json!(kind);
        match kind {
            "EVENT" => {
                value["data_item_id"] = json!("DID-001");
                value["component_id"] = json!("CONTROLLER-01");
                value["sequence"] = json!(11);
                value["source_timestamp_ns"] = json!(1750000200_u64);
                value["reference_timestamp_ns"] = json!(1750000200_u64);
                value["native_unit"] = json!("");
                value["value_text"] = json!("CYCLE_ACTIVE");
                value["source_locator"] =
                    json!("/MTConnectStreams/Streams/ComponentStream/Observations/Observation[1]");
            }
            "CONDITION" => {
                value["data_item_id"] = json!("DID-002");
                value["component_id"] = json!("CONTROLLER-01");
                value["sequence"] = json!(12);
                value["source_timestamp_ns"] = json!(1750000300_u64);
                value["reference_timestamp_ns"] = json!(1750000300_u64);
                value["native_unit"] = json!("");
                value["value_text"] = json!("level=NORMAL;code=OK;description=No fault");
                value["source_locator"] =
                    json!("/MTConnectStreams/Streams/ComponentStream/Observations/Observation[2]");
            }
            _ => panic!("unsupported test stream kind"),
        }
        value
    }

    fn stream_variant_oracle(kind: &str) -> Value {
        let base = oracle();
        let mut value = base;
        match kind {
            "EVENT" => {
                value["data_item_id"] = json!("DID-001");
                value["component_id"] = json!("CONTROLLER-01");
                value["sequence"] = json!(11);
                value["source_timestamp_ns"] = json!(1750000200_u64);
                value["reference_timestamp_ns"] = json!(1750000200_u64);
                value["native_kind"] = json!("EVENT");
                value["native_unit"] = json!("");
                value["value_text"] = json!("CYCLE_ACTIVE");
                value["source_locator"] =
                    json!("/MTConnectStreams/Streams/ComponentStream/Observations/Observation[1]");
            }
            "CONDITION" => {
                value["data_item_id"] = json!("DID-002");
                value["component_id"] = json!("CONTROLLER-01");
                value["sequence"] = json!(12);
                value["source_timestamp_ns"] = json!(1750000300_u64);
                value["reference_timestamp_ns"] = json!(1750000300_u64);
                value["native_kind"] = json!("CONDITION");
                value["native_unit"] = json!("");
                value["value_text"] = json!("level=NORMAL;code=OK;description=No fault");
                value["source_locator"] =
                    json!("/MTConnectStreams/Streams/ComponentStream/Observations/Observation[2]");
            }
            _ => panic!("unsupported test stream kind"),
        }
        value
    }

    #[test]
    fn sample_event_and_condition_each_match_their_independent_known_answer() {
        let sample = compare_required_fields(&oracle(), &candidate(), FIELDS).unwrap();
        assert!(sample.equivalent_on_required_fields);

        for kind in ["EVENT", "CONDITION"] {
            let result = compare_required_fields(
                &stream_variant_oracle(kind),
                &stream_variant(kind),
                FIELDS,
            )
            .unwrap();
            assert!(result.equivalent_on_required_fields, "{kind}");
            assert_eq!(result.matched_fields, FIELDS.len());
        }
    }

    #[test]
    fn event_and_condition_cannot_be_coerced_to_each_other() {
        let event_as_condition = stream_variant("EVENT");
        let condition_truth = stream_variant_oracle("CONDITION");
        let result =
            compare_required_fields(&condition_truth, &event_as_condition, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
    }

    #[test]
    fn exact_synthetic_projection_matches_independent_oracle() {
        let result = compare_required_fields(&oracle(), &candidate(), FIELDS)
            .expect("valid object comparison");
        assert!(result.equivalent_on_required_fields);
        assert_eq!(result.compared_fields, FIELDS.len());
        assert_eq!(result.matched_fields, FIELDS.len());
        assert!(
            result
                .results
                .iter()
                .all(|field| { field.disposition == FieldDisposition::Match })
        );
    }

    #[test]
    fn altered_machine_identity_is_not_hidden_by_other_matches() {
        let mut actual = candidate();
        actual["device_uuid"] = json!("urn:uuid:wrong-machine");
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
        assert_eq!(result.matched_fields, FIELDS.len() - 1);
        assert_eq!(
            result
                .results
                .iter()
                .find(|field| field.field == "device_uuid")
                .unwrap()
                .disposition,
            FieldDisposition::Mismatch
        );
    }

    #[test]
    fn missing_sequence_is_incomplete_not_zero() {
        let mut actual = candidate();
        actual.as_object_mut().unwrap().remove("sequence");
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
        assert_eq!(
            result
                .results
                .iter()
                .find(|field| field.field == "sequence")
                .unwrap()
                .disposition,
            FieldDisposition::MissingFromCandidate
        );
    }

    #[test]
    fn changed_unit_is_a_semantic_mismatch() {
        let mut actual = candidate();
        actual["native_unit"] = json!("Hz");
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
    }

    #[test]
    fn changed_clock_mapping_is_a_semantic_mismatch() {
        let mut actual = candidate();
        actual["clock_mapping_sha256"] =
            json!("7000000000000000000000000000000000000000000000000000000000000000");
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
    }

    #[test]
    fn dropped_opaque_extension_is_not_silently_accepted() {
        let mut actual = candidate();
        actual["opaque_extensions"] = json!([]);
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
    }

    #[test]
    fn additional_candidate_fields_are_reported() {
        let mut actual = candidate();
        actual["backend_vendor_metadata"] = json!("urn:luminous:test:adapter");
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(result.equivalent_on_required_fields);
        assert_eq!(
            result.additional_candidate_fields,
            vec!["backend_vendor_metadata".to_owned()]
        );
    }

    #[test]
    fn empty_or_duplicate_required_field_sets_are_rejected() {
        let oracle = oracle();
        let candidate = candidate();
        assert!(compare_required_fields(&oracle, &candidate, &[]).is_err());
        assert!(compare_required_fields(&oracle, &candidate, &["sequence", "sequence"]).is_err());
    }

    #[test]
    fn missing_oracle_field_is_not_satisfied_by_candidate_self_claim() {
        let mut expected = oracle();
        expected.as_object_mut().unwrap().remove("data_item_id");
        let result = compare_required_fields(&expected, &candidate(), FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
        assert_eq!(
            result
                .results
                .iter()
                .find(|field| field.field == "data_item_id")
                .unwrap()
                .disposition,
            FieldDisposition::MissingFromOracle
        );
    }
    #[test]
    fn source_document_digest_mismatch_is_hard_evidence_failure() {
        let mut actual = candidate();
        actual["source_document_sha256"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
    }

    #[test]
    fn reference_timestamp_mismatch_is_not_silently_aligned() {
        let mut actual = candidate();
        actual["reference_timestamp_ns"] = json!(1750000101);
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
    }

    #[test]
    fn missing_source_locator_is_incomplete_evidence() {
        let mut actual = candidate();
        actual.as_object_mut().unwrap().remove("source_locator");
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
        assert_eq!(
            result
                .results
                .iter()
                .find(|field| field.field == "source_locator")
                .unwrap()
                .disposition,
            FieldDisposition::MissingFromCandidate
        );
    }

    #[test]
    fn observation_kind_change_is_not_semantically_equivalent() {
        let mut actual = candidate();
        actual["native_kind"] = json!("EVENT");
        let result = compare_required_fields(&oracle(), &actual, FIELDS).unwrap();
        assert!(!result.equivalent_on_required_fields);
    }
}
