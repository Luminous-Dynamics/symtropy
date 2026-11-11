// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Building-specific capture evidence adapter over the existing finite-state
//! spatial decision engine. This crate does not implement a second VOI/NBV
//! algorithm or duplicate survey, geometry, identity, or Pareto ownership.

use jsonschema::validator_for;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use spatial_resilience_qualifier::{
    EMBEDDED_DECISION_IMPACT_CORPUS, EMBEDDED_DECISION_IMPACT_SCHEMA,
    validate_decision_impact_bytes,
};
use std::collections::{BTreeMap, BTreeSet};

pub const EMBEDDED_ADAPTER_CORPUS: &str = include_str!(
    "../../../docs/research/built-asset/scan-bim/SCAN_BIM_DECISION_CAPTURE_ADAPTER_V0_1.json"
);
pub const EMBEDDED_ADAPTER_SCHEMA: &str = include_str!(
    "../../../docs/research/built-asset/scan-bim/SCAN_BIM_DECISION_CAPTURE_ADAPTER_V0_1.schema.json"
);

const EXPECTED_CASES: usize = 8;
const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_CAPTURE_BINDINGS: usize = 1600;

#[derive(Debug, Clone, Serialize)]
pub struct AdapterFailure {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaptureGateResult {
    pub action_id: String,
    pub status: String,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterCaseResult {
    pub case_id: String,
    pub building_id: String,
    pub scene_id: String,
    pub capture_bindings: Vec<CaptureGateResult>,
    pub eligible_capture_action_ids: Vec<String>,
    pub recommended_action_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterCore {
    pub corpus_id: Option<String>,
    pub schema_version: Option<String>,
    pub case_count: usize,
    pub capture_binding_count: usize,
    pub qualified_capture_count: usize,
    pub unqualified_capture_count: usize,
    pub decision_impact_cases_passed: usize,
    pub decision_impact_passed: bool,
    pub passed: bool,
    pub cases: Vec<AdapterCaseResult>,
    pub decision_impact_receipt: Value,
    pub failures: Vec<AdapterFailure>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterReport {
    pub input_sha256: String,
    pub schema_sha256: String,
    pub receipt_sha256: String,
    pub core: AdapterCore,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn fail(
    failures: &mut Vec<AdapterFailure>,
    code: &str,
    path: impl Into<String>,
    detail: impl Into<String>,
) {
    failures.push(AdapterFailure {
        code: code.to_owned(),
        path: path.into(),
        detail: detail.into(),
    });
}

fn capture_gate(
    binding: &Value,
    action: &Value,
    asset: &Value,
    decision_profile_id: &str,
) -> CaptureGateResult {
    let action_id = string_at(binding, "action_id").unwrap_or("").to_owned();
    let mut reasons = BTreeSet::new();

    let survey_revision = string_at(asset, "survey_revision").unwrap_or("");
    let action_revision = string_at(action, "source_revision").unwrap_or("");
    let source_revision = string_at(binding, "source_revision").unwrap_or("");
    let frame_revision = string_at(binding, "frame_revision").unwrap_or("");
    let frame_id = string_at(binding, "frame_id").unwrap_or("");
    let expected_frame_id = string_at(asset, "coordinate_frame_id").unwrap_or("");
    let task_profile = string_at(binding, "task_profile_id").unwrap_or("");
    let model_revision = string_at(binding, "model_revision").unwrap_or("");
    let expected_model_revision = string_at(asset, "model_revision").unwrap_or("");
    let requirement_revision = string_at(binding, "requirement_revision").unwrap_or("");
    let expected_requirement_revision = string_at(asset, "requirement_revision").unwrap_or("");
    let measurement_receipt_sha = string_at(binding, "measurement_receipt_sha256").unwrap_or("");

    let sensor_profile_id = string_at(binding, "sensor_profile_id").unwrap_or("");
    let sensor_profile_sha = string_at(binding, "sensor_profile_sha256").unwrap_or("");
    let pose_revision = string_at(binding, "pose_revision").unwrap_or("");
    let pose_sha = string_at(binding, "pose_evidence_sha256").unwrap_or("");
    let visibility_sha = string_at(binding, "visibility_receipt_sha256").unwrap_or("");

    if sensor_profile_id.trim().is_empty() || !is_sha256(sensor_profile_sha) {
        reasons.insert("SENSOR_PROFILE_BINDING_INVALID");
    }
    if pose_revision.trim().is_empty() || !is_sha256(pose_sha) {
        reasons.insert("POSE_EVIDENCE_BINDING_INVALID");
    }
    if !is_sha256(visibility_sha) {
        reasons.insert("VISIBILITY_RECEIPT_BINDING_INVALID");
    }
    if !is_sha256(measurement_receipt_sha) {
        reasons.insert("MEASUREMENT_RECEIPT_BINDING_INVALID");
    }
    if model_revision != expected_model_revision {
        reasons.insert("CAPTURE_MODEL_REVISION_MISMATCH");
    }
    if requirement_revision != expected_requirement_revision {
        reasons.insert("CAPTURE_REQUIREMENT_REVISION_MISMATCH");
    }
    if source_revision != action_revision || source_revision != survey_revision {
        reasons.insert("CAPTURE_SOURCE_REVISION_MISMATCH");
    }
    if frame_revision != survey_revision {
        reasons.insert("CAPTURE_FRAME_REVISION_MISMATCH");
    }
    if frame_id != expected_frame_id {
        reasons.insert("CAPTURE_COORDINATE_FRAME_MISMATCH");
    }
    if string_at(binding, "pose_status") != Some("qualified") {
        reasons.insert("CAPTURE_POSE_NOT_QUALIFIED");
    }
    if binding.get("reachable").and_then(Value::as_bool) != Some(true) {
        reasons.insert("CAPTURE_POSE_NOT_REACHABLE");
    }
    if string_at(binding, "visibility_status") != Some("observable") {
        reasons.insert("CAPTURE_VISIBILITY_NOT_OBSERVABLE");
    }
    if string_at(binding, "measurement_qualification") != Some("qualified") {
        reasons.insert("CAPTURE_MEASUREMENT_NOT_QUALIFIED");
    }
    if task_profile != decision_profile_id {
        reasons.insert("CAPTURE_TASK_PROFILE_MISMATCH");
    }

    let reason_codes = reasons.into_iter().map(str::to_owned).collect::<Vec<_>>();
    CaptureGateResult {
        action_id,
        status: if reason_codes.is_empty() {
            "qualified".to_owned()
        } else {
            "unqualified".to_owned()
        },
        reason_codes,
    }
}

pub fn validate_adapter_bytes(corpus_bytes: &[u8], schema_bytes: &[u8]) -> AdapterReport {
    let mut failures = Vec::new();

    if corpus_bytes.len() > MAX_INPUT_BYTES || schema_bytes.len() > MAX_INPUT_BYTES {
        fail(
            &mut failures,
            "INPUT_SIZE_LIMIT_EXCEEDED",
            "$",
            format!("Each input must be at most {MAX_INPUT_BYTES} bytes."),
        );
        return empty_report(corpus_bytes, schema_bytes, failures);
    }

    let schema_value: Value = match serde_json::from_slice(schema_bytes) {
        Ok(value) => value,
        Err(error) => {
            fail(&mut failures, "SCHEMA_JSON_INVALID", "$", error.to_string());
            return empty_report(corpus_bytes, schema_bytes, failures);
        }
    };
    let corpus_value: Value = match serde_json::from_slice(corpus_bytes) {
        Ok(value) => value,
        Err(error) => {
            fail(&mut failures, "CORPUS_JSON_INVALID", "$", error.to_string());
            return empty_report(corpus_bytes, schema_bytes, failures);
        }
    };

    let validator = match validator_for(&schema_value) {
        Ok(validator) => validator,
        Err(error) => {
            fail(
                &mut failures,
                "SCHEMA_COMPILE_FAILED",
                "$",
                error.to_string(),
            );
            return empty_report(corpus_bytes, schema_bytes, failures);
        }
    };
    let schema_errors: Vec<_> = validator.iter_errors(&corpus_value).collect();
    if !schema_errors.is_empty() {
        for error in schema_errors {
            fail(
                &mut failures,
                "SCHEMA_VALIDATION_FAILED",
                error.instance_path().to_string(),
                error.to_string(),
            );
        }
        return empty_report(corpus_bytes, schema_bytes, failures);
    }

    let Some(root) = corpus_value.as_object() else {
        fail(
            &mut failures,
            "CORPUS_ROOT_INVALID",
            "$",
            "Expected object.",
        );
        return empty_report(corpus_bytes, schema_bytes, failures);
    };
    let corpus_id = string_at(&corpus_value, "corpus_id").map(ToOwned::to_owned);
    let schema_version = string_at(&corpus_value, "schema_version").map(ToOwned::to_owned);
    let Some(cases) = root.get("cases").and_then(Value::as_array) else {
        fail(
            &mut failures,
            "CASES_MISSING",
            "$.cases",
            "Expected cases array.",
        );
        return empty_report(corpus_bytes, schema_bytes, failures);
    };
    if cases.len() != EXPECTED_CASES {
        fail(
            &mut failures,
            "CASE_COUNT_MISMATCH",
            "$.cases",
            format!("Expected exactly {EXPECTED_CASES} cases."),
        );
    }

    let mut generic_cases = Vec::with_capacity(cases.len());
    let mut adapter_cases = Vec::with_capacity(cases.len());
    let mut case_ids = BTreeSet::new();
    let mut total_bindings = 0usize;
    let mut qualified_count = 0usize;
    let mut unqualified_count = 0usize;

    for (case_index, case) in cases.iter().enumerate() {
        let case_path = format!("$.cases[{case_index}]");
        let case_id = string_at(case, "case_id").unwrap_or("").to_owned();
        if case_id.trim().is_empty() || !case_ids.insert(case_id.clone()) {
            fail(
                &mut failures,
                "CASE_ID_MISSING_OR_DUPLICATE",
                format!("{case_path}.case_id"),
                "Case IDs must be present and unique.",
            );
        }

        let asset = case.get("asset_binding").unwrap_or(&Value::Null);
        let decision_case = case.get("decision_case").unwrap_or(&Value::Null);
        let Some(bindings) = case.get("capture_bindings").and_then(Value::as_array) else {
            fail(
                &mut failures,
                "CAPTURE_BINDINGS_MISSING",
                format!("{case_path}.capture_bindings"),
                "Expected capture bindings array.",
            );
            continue;
        };
        generic_cases.push(decision_case.clone());

        let generic_case_id = string_at(decision_case, "id").unwrap_or("");
        if generic_case_id != case_id {
            fail(
                &mut failures,
                "CASE_ID_DECISION_ID_MISMATCH",
                format!("{case_path}.decision_case.id"),
                "Adapter case ID must match the delegated decision case ID.",
            );
        }

        let survey_revision = string_at(asset, "survey_revision").unwrap_or("");
        let decision_revision = string_at(decision_case, "state_revision").unwrap_or("");
        if survey_revision.is_empty() || survey_revision != decision_revision {
            fail(
                &mut failures,
                "ASSET_SURVEY_REVISION_MISMATCH",
                format!("{case_path}.asset_binding.survey_revision"),
                "Asset survey revision must equal the decision engine state revision.",
            );
        }

        let decision_profile_id = string_at(asset, "decision_profile_id").unwrap_or("");
        if decision_profile_id.trim().is_empty() {
            fail(
                &mut failures,
                "DECISION_PROFILE_MISSING",
                format!("{case_path}.asset_binding.decision_profile_id"),
                "A building decision profile is required.",
            );
        }

        let Some(actions) = decision_case.get("actions").and_then(Value::as_array) else {
            fail(
                &mut failures,
                "DECISION_ACTIONS_MISSING",
                format!("{case_path}.decision_case.actions"),
                "Delegated decision case must contain an action array.",
            );
            adapter_cases.push(AdapterCaseResult {
                case_id,
                building_id: string_at(asset, "building_id").unwrap_or("").to_owned(),
                scene_id: string_at(asset, "scene_id").unwrap_or("").to_owned(),
                capture_bindings: Vec::new(),
                eligible_capture_action_ids: Vec::new(),
                recommended_action_order: Vec::new(),
            });
            continue;
        };
        let mut actions_by_id = BTreeMap::new();
        for action in actions {
            if let Some(id) = string_at(action, "action_id") {
                if actions_by_id.insert(id.to_owned(), action).is_some() {
                    fail(
                        &mut failures,
                        "DECISION_ACTION_ID_DUPLICATE",
                        format!("{case_path}.decision_case.actions"),
                        format!("Duplicate decision action {id:?}."),
                    );
                }
            }
        }

        let mut seen_binding_ids = BTreeSet::new();
        let mut capture_results = Vec::new();
        for (binding_index, binding) in bindings.iter().enumerate() {
            total_bindings += 1;
            if total_bindings > MAX_CAPTURE_BINDINGS {
                fail(
                    &mut failures,
                    "CAPTURE_BINDING_LIMIT_EXCEEDED",
                    "$.cases",
                    format!("More than {MAX_CAPTURE_BINDINGS} capture bindings."),
                );
                break;
            }
            let action_id = string_at(binding, "action_id").unwrap_or("");
            let binding_path = format!("{case_path}.capture_bindings[{binding_index}]");
            if action_id.is_empty() || !seen_binding_ids.insert(action_id.to_owned()) {
                fail(
                    &mut failures,
                    "CAPTURE_BINDING_ID_MISSING_OR_DUPLICATE",
                    format!("{binding_path}.action_id"),
                    "Capture action IDs must be present and unique within a case.",
                );
                continue;
            }
            let Some(action) = actions_by_id.get(action_id) else {
                fail(
                    &mut failures,
                    "CAPTURE_BINDING_ACTION_UNKNOWN",
                    format!("{binding_path}.action_id"),
                    format!("No decision action {action_id:?} exists in this case."),
                );
                continue;
            };
            let result = capture_gate(binding, action, asset, decision_profile_id);
            if result.status == "qualified" {
                qualified_count += 1;
            } else {
                unqualified_count += 1;
            }
            capture_results.push(result);
        }

        let action_ids: BTreeSet<&str> = actions_by_id.keys().map(String::as_str).collect();
        let binding_ids: BTreeSet<&str> = seen_binding_ids.iter().map(String::as_str).collect();
        if action_ids != binding_ids {
            fail(
                &mut failures,
                "CAPTURE_BINDING_COVERAGE_MISMATCH",
                format!("{case_path}.capture_bindings"),
                "There must be exactly one capture-evidence binding for every decision action.",
            );
        }

        capture_results.sort_by(|a, b| a.action_id.cmp(&b.action_id));
        adapter_cases.push(AdapterCaseResult {
            case_id,
            building_id: string_at(asset, "building_id").unwrap_or("").to_owned(),
            scene_id: string_at(asset, "scene_id").unwrap_or("").to_owned(),
            capture_bindings: capture_results,
            eligible_capture_action_ids: Vec::new(),
            recommended_action_order: Vec::new(),
        });
    }

    // Preserve the existing decision corpus envelope, substituting only the
    // building-specific cases. The existing owner remains authoritative for
    // partitions, robust feasibility, Pareto and action recommendations.
    let mut generic_root: Value = match serde_json::from_str(EMBEDDED_DECISION_IMPACT_CORPUS) {
        Ok(value) => value,
        Err(error) => {
            fail(
                &mut failures,
                "GENERIC_CORPUS_EMBEDDED_INVALID",
                "$",
                error.to_string(),
            );
            return empty_report(corpus_bytes, schema_bytes, failures);
        }
    };
    generic_root["cases"] = Value::Array(generic_cases);
    let generic_bytes = serde_json::to_vec(&generic_root).unwrap_or_default();
    let decision_summary =
        validate_decision_impact_bytes(&generic_bytes, EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes());
    let decision_impact_passed = decision_summary.passed;

    if !decision_impact_passed {
        fail(
            &mut failures,
            "DELEGATED_DECISION_QUALIFIER_FAILED",
            "$.cases[].decision_case",
            format!(
                "The existing #1892 decision engine rejected the nested corpus with {} failure(s).",
                decision_summary.failures.len()
            ),
        );
    }

    let mut decision_case_result_ids = BTreeMap::new();
    for result in &decision_summary.results {
        decision_case_result_ids.insert(result.case_id.clone(), result);
    }
    for adapter_case in &mut adapter_cases {
        if let Some(decision_result) = decision_case_result_ids.get(&adapter_case.case_id) {
            adapter_case.recommended_action_order =
                decision_result.recommended_action_order.clone();
            let qualified_ids: BTreeSet<&str> = adapter_case
                .capture_bindings
                .iter()
                .filter(|binding| binding.status == "qualified")
                .map(|binding| binding.action_id.as_str())
                .collect();
            adapter_case.eligible_capture_action_ids =
                qualified_ids.iter().map(|id| (*id).to_owned()).collect();
            adapter_case.eligible_capture_action_ids.sort();

            for recommended in &adapter_case.recommended_action_order {
                if !qualified_ids.contains(recommended.as_str()) {
                    fail(
                        &mut failures,
                        "DECISION_RECOMMENDS_UNQUALIFIED_CAPTURE",
                        format!("$.cases[{}].capture_bindings", adapter_case.case_id),
                        format!(
                            "Decision engine recommends {recommended:?}, but its capture evidence gate is not qualified."
                        ),
                    );
                }
            }
        }
    }

    let decision_impact_receipt = serde_json::to_value(&decision_summary).unwrap_or(Value::Null);
    let decision_impact_cases_passed = decision_summary.cases_passed;
    failures.sort_by(|a, b| (&a.path, &a.code, &a.detail).cmp(&(&b.path, &b.code, &b.detail)));
    let core = AdapterCore {
        corpus_id,
        schema_version,
        case_count: adapter_cases.len(),
        capture_binding_count: total_bindings,
        qualified_capture_count: qualified_count,
        unqualified_capture_count: unqualified_count,
        decision_impact_cases_passed,
        decision_impact_passed,
        passed: failures.is_empty() && decision_impact_passed,
        cases: adapter_cases,
        decision_impact_receipt,
        failures,
    };
    let core_bytes = serde_json::to_vec(&core).unwrap_or_default();
    AdapterReport {
        input_sha256: sha256_hex(corpus_bytes),
        schema_sha256: sha256_hex(schema_bytes),
        receipt_sha256: sha256_hex(&core_bytes),
        core,
    }
}

fn empty_report(corpus: &[u8], schema: &[u8], failures: Vec<AdapterFailure>) -> AdapterReport {
    let core = AdapterCore {
        corpus_id: None,
        schema_version: None,
        case_count: 0,
        capture_binding_count: 0,
        qualified_capture_count: 0,
        unqualified_capture_count: 0,
        decision_impact_cases_passed: 0,
        decision_impact_passed: false,
        passed: false,
        cases: Vec::new(),
        decision_impact_receipt: json!({}),
        failures,
    };
    let core_bytes = serde_json::to_vec(&core).unwrap_or_default();
    AdapterReport {
        input_sha256: sha256_hex(corpus),
        schema_sha256: sha256_hex(schema),
        receipt_sha256: sha256_hex(&core_bytes),
        core,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(EMBEDDED_ADAPTER_CORPUS).expect("adapter corpus parses")
    }

    fn bytes(value: &Value) -> Vec<u8> {
        serde_json::to_vec(value).expect("fixture serializes")
    }

    fn has_code(report: &AdapterReport, expected: &str) -> bool {
        report
            .core
            .failures
            .iter()
            .any(|failure| failure.code == expected)
    }

    #[test]
    fn embedded_adapter_delegates_all_eight_cases_and_eleven_actions() {
        let report = validate_adapter_bytes(
            EMBEDDED_ADAPTER_CORPUS.as_bytes(),
            EMBEDDED_ADAPTER_SCHEMA.as_bytes(),
        );
        assert!(report.core.passed, "{:#?}", report.core.failures);
        assert_eq!(report.core.case_count, 8);
        assert_eq!(report.core.capture_binding_count, 11);
        assert_eq!(report.core.qualified_capture_count, 10);
        assert_eq!(report.core.unqualified_capture_count, 1);
        assert_eq!(report.core.decision_impact_cases_passed, 8);
        assert_eq!(report.receipt_sha256.len(), 64);
    }

    #[test]
    fn externally_occluded_capture_cannot_be_recommended() {
        let mut corpus = fixture();
        corpus["cases"][0]["capture_bindings"][0]["visibility_status"] =
            Value::String("externally_occluded".to_owned());
        let report = validate_adapter_bytes(&bytes(&corpus), EMBEDDED_ADAPTER_SCHEMA.as_bytes());
        assert!(!report.core.passed);
        assert!(has_code(&report, "DECISION_RECOMMENDS_UNQUALIFIED_CAPTURE"));
    }

    #[test]
    fn stale_frame_revision_cannot_qualify_a_capture() {
        let mut corpus = fixture();
        corpus["cases"][0]["capture_bindings"][0]["frame_revision"] =
            Value::String("stale-survey-revision".to_owned());
        let report = validate_adapter_bytes(&bytes(&corpus), EMBEDDED_ADAPTER_SCHEMA.as_bytes());
        assert!(!report.core.passed);
        assert!(has_code(&report, "DECISION_RECOMMENDS_UNQUALIFIED_CAPTURE"));
    }

    #[test]
    fn unreachable_or_unqualified_pose_cannot_qualify_capture() {
        let mut corpus = fixture();
        corpus["cases"][0]["capture_bindings"][0]["reachable"] = Value::Bool(false);
        corpus["cases"][0]["capture_bindings"][0]["pose_status"] =
            Value::String("unqualified".to_owned());
        let report = validate_adapter_bytes(&bytes(&corpus), EMBEDDED_ADAPTER_SCHEMA.as_bytes());
        assert!(!report.core.passed);
        assert!(has_code(&report, "DECISION_RECOMMENDS_UNQUALIFIED_CAPTURE"));
    }

    #[test]
    fn missing_action_binding_is_rejected() {
        let mut corpus = fixture();
        corpus["cases"][0]["capture_bindings"]
            .as_array_mut()
            .expect("bindings")
            .remove(0);
        let report = validate_adapter_bytes(&bytes(&corpus), EMBEDDED_ADAPTER_SCHEMA.as_bytes());
        assert!(!report.core.passed);
        assert!(has_code(&report, "CAPTURE_BINDING_COVERAGE_MISMATCH"));
    }

    #[test]
    fn model_or_requirement_revision_mismatch_blocks_recommended_capture() {
        let mut corpus = fixture();
        corpus["cases"][0]["capture_bindings"][0]["model_revision"] =
            Value::String("superseded-model".to_owned());
        let report = validate_adapter_bytes(&bytes(&corpus), EMBEDDED_ADAPTER_SCHEMA.as_bytes());
        assert!(!report.core.passed);
        assert!(has_code(&report, "DECISION_RECOMMENDS_UNQUALIFIED_CAPTURE"));
    }

    #[test]
    fn malformed_nested_decision_case_returns_failure_without_panicking() {
        let mut corpus = fixture();
        corpus["cases"][0]["decision_case"] = json!({
            "id": "SPR-V-001",
            "state_revision": "access-survey-v1"
        });
        let report = validate_adapter_bytes(&bytes(&corpus), EMBEDDED_ADAPTER_SCHEMA.as_bytes());
        assert!(!report.core.passed);
        assert!(has_code(&report, "DECISION_ACTIONS_MISSING"));
    }

    #[test]
    fn measurement_receipt_digest_is_required_by_schema() {
        let mut corpus = fixture();
        corpus["cases"][0]["capture_bindings"][0]["measurement_receipt_sha256"] =
            Value::String("not-a-digest".to_owned());
        let report = validate_adapter_bytes(&bytes(&corpus), EMBEDDED_ADAPTER_SCHEMA.as_bytes());
        assert!(!report.core.passed);
        assert!(has_code(&report, "SCHEMA_VALIDATION_FAILED"));
    }

    #[test]
    fn closed_schema_rejects_unknown_fields() {
        let mut corpus = fixture();
        corpus["unexpected"] = Value::Bool(true);
        let report = validate_adapter_bytes(&bytes(&corpus), EMBEDDED_ADAPTER_SCHEMA.as_bytes());
        assert!(!report.core.passed);
        assert!(has_code(&report, "SCHEMA_VALIDATION_FAILED"));
    }
}
