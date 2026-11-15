//! Synthetic-only B0 verifier for PROTECTIVE-SYSTEMS-001.
//!
//! The verifier qualifies record semantics only. It cannot establish physical
//! performance, test validity, certification, regulatory compliance, or safety.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub mod damage;
pub mod robust;
pub mod voi;

pub const ACCEPT_RECORD_ONLY: &str = "ACCEPT_RECORD_ONLY";
pub const REJECT_SCOPE: &str = "REJECT_SCOPE";
pub const REJECT_STALE_OR_REVOKED: &str = "REJECT_STALE_OR_REVOKED";
pub const REJECT_MISSING_OR_INVALID: &str = "REJECT_MISSING_OR_INVALID";
pub const ABSTAIN_UNKNOWN: &str = "ABSTAIN_UNKNOWN";
pub const REQUIRE_INDEPENDENT_REVIEW: &str = "REQUIRE_INDEPENDENT_REVIEW";

const DECISIONS: [&str; 6] = [
    ACCEPT_RECORD_ONLY,
    REJECT_SCOPE,
    REJECT_STALE_OR_REVOKED,
    REJECT_MISSING_OR_INVALID,
    ABSTAIN_UNKNOWN,
    REQUIRE_INDEPENDENT_REVIEW,
];

#[derive(Debug, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub expected_decision: String,
    pub actual_decision: String,
    pub passed: bool,
    pub reason_code: String,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub verifier: &'static str,
    pub verifier_version: &'static str,
    pub schema_version: String,
    pub evidence_class: &'static str,
    pub claim_ceiling: &'static str,
    pub canonical_corpus_sha256: String,
    pub case_count: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub conformance_pass: bool,
    pub cases: Vec<CaseResult>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ValidationError {
    pub messages: Vec<String>,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "corpus validation failed: {}", self.messages.join("; "))
    }
}

impl std::error::Error for ValidationError {}

fn field<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object()?.get(key)
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    field(value, key)?.as_str()
}

fn nested_text<'a>(value: &'a Value, object: &str, key: &str) -> Option<&'a str> {
    text(field(value, object)?, key)
}

fn nested_bool(value: &Value, object: &str, key: &str) -> Option<bool> {
    field(value, object)?.get(key)?.as_bool()
}

fn is_true(value: &Value, object: &str, key: &str) -> bool {
    nested_bool(value, object, key) == Some(true)
}

fn is_null_or_missing(value: &Value, object: &str, key: &str) -> bool {
    matches!(
        field(value, object).and_then(|v| v.get(key)),
        None | Some(Value::Null)
    )
}

fn has_nonempty_text(value: &Value, object: &str, key: &str) -> bool {
    nested_text(value, object, key)
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
}

fn structural_errors(root: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if !root.is_object() {
        return vec!["root must be an object".to_owned()];
    }

    if text(root, "schema_version") != Some("protective-systems-fixtures/0.1") {
        errors.push("schema_version must equal protective-systems-fixtures/0.1".to_owned());
    }
    if text(root, "evidence_class") != Some("synthetic") {
        errors.push("evidence_class must equal synthetic".to_owned());
    }
    if text(root, "intended_use")
        .map(str::trim)
        .unwrap_or("")
        .is_empty()
    {
        errors.push("intended_use must be a non-empty string".to_owned());
    }

    let declared = match field(root, "decisions").and_then(Value::as_array) {
        Some(items) if !items.is_empty() => items,
        _ => {
            errors.push("decisions must be a non-empty array".to_owned());
            return errors;
        }
    };
    let mut declared_set = BTreeSet::new();
    for (index, decision) in declared.iter().enumerate() {
        match decision.as_str() {
            Some(name) if DECISIONS.contains(&name) => {
                if !declared_set.insert(name) {
                    errors.push(format!("decisions[{index}] duplicates {name}"));
                }
            }
            _ => errors.push(format!("decisions[{index}] is not a supported decision")),
        }
    }

    let cases = match field(root, "cases").and_then(Value::as_array) {
        Some(items) if !items.is_empty() => items,
        _ => {
            errors.push("cases must be a non-empty array".to_owned());
            return errors;
        }
    };

    let mut ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let prefix = format!("cases[{index}]");
        if !case.is_object() {
            errors.push(format!("{prefix} must be an object"));
            continue;
        }

        match text(case, "case_id").filter(|s| !s.trim().is_empty()) {
            Some(id) => {
                if !ids.insert(id.to_owned()) {
                    errors.push(format!("{prefix}.case_id duplicates {id}"));
                }
            }
            None => errors.push(format!("{prefix}.case_id must be non-empty")),
        }

        for (object, key) in [
            ("subject", "id"),
            ("subject", "configuration_revision"),
            ("claim", "proposition"),
            ("standard", "id"),
            ("standard", "currentness"),
            ("standard", "scope"),
            ("evidence", "class"),
        ] {
            if !has_nonempty_text(case, object, key) {
                errors.push(format!("{prefix}.{object}.{key} must be non-empty"));
            }
        }

        if nested_bool(case, "claim", "performance_claim_permitted").is_none() {
            errors.push(format!(
                "{prefix}.claim.performance_claim_permitted must be boolean"
            ));
        }
        match field(case, "standard")
            .and_then(|v| v.get("addenda"))
            .and_then(Value::as_array)
        {
            Some(addenda) => {
                let mut seen = BTreeSet::new();
                for (addendum_index, addendum) in addenda.iter().enumerate() {
                    match addendum.as_str().filter(|s| !s.trim().is_empty()) {
                        Some(value) => {
                            if !seen.insert(value) {
                                errors.push(format!(
                                    "{prefix}.standard.addenda contains duplicate {value}"
                                ));
                            }
                        }
                        None => errors.push(format!(
                            "{prefix}.standard.addenda[{addendum_index}] must be non-empty text"
                        )),
                    }
                }
            }
            None => errors.push(format!("{prefix}.standard.addenda must be an array")),
        }
        match text(case, "domain") {
            Some("human_worn" | "vehicle" | "housing" | "cross_domain") => {}
            _ => errors.push(format!("{prefix}.domain is not a supported domain")),
        }
        if text(case, "fixture_class")
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            errors.push(format!("{prefix}.fixture_class must be non-empty"));
        }
        if text(case, "rationale")
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
        {
            errors.push(format!("{prefix}.rationale must be non-empty"));
        }
        match text(case, "expected_decision") {
            Some(name) if DECISIONS.contains(&name) && declared_set.contains(name) => {}
            _ => errors.push(format!(
                "{prefix}.expected_decision is unsupported or undeclared"
            )),
        }
    }

    errors
}

fn decision(name: &str, reason: &str) -> (String, String) {
    (name.to_owned(), reason.to_owned())
}

/// Evaluate one structurally valid record using conservative B0 semantics.
pub fn evaluate_case(case: &Value) -> (String, String) {
    let domain = text(case, "domain").unwrap_or("");
    let evidence_class = nested_text(case, "evidence", "class").unwrap_or("");
    let standard_id = nested_text(case, "standard", "id").unwrap_or("");
    let standard_scope = nested_text(case, "standard", "scope").unwrap_or("");
    let currentness = nested_text(case, "standard", "currentness").unwrap_or("");
    let proposition = nested_text(case, "claim", "proposition")
        .unwrap_or("")
        .to_ascii_lowercase();

    // A signature authenticates a record; it does not provide physical evidence.
    let signature_only = evidence_class == "signed_hash_only"
        || (is_true(case, "evidence", "signature_valid")
            && is_null_or_missing(case, "evidence", "physical_test_report")
            && is_null_or_missing(case, "evidence", "measurement_data"));
    if signature_only {
        return decision(
            REJECT_MISSING_OR_INVALID,
            "SIGNATURE_WITHOUT_PHYSICAL_CORROBORATION",
        );
    }

    // Test methods must be applied only to their declared article and proposition.
    if domain == "human_worn" && standard_id == "NIJ-0101.07" {
        let form_factor = nested_text(case, "subject", "form_factor").unwrap_or("");
        if standard_scope == "human_torso_body_armor" && form_factor != "human_torso" {
            return decision(REJECT_SCOPE, "TORSO_METHOD_USED_OUTSIDE_TORSO_SCOPE");
        }
    }
    if domain == "housing"
        && standard_id == "ISO-11925-2:2026"
        && standard_scope == "small_flame_ignitability_test"
        && (proposition.contains("whole_wall")
            || proposition.contains("whole_assembly")
            || nested_text(case, "subject", "assembly_scope")
                .map(|s| s.contains("complete_wall") || s.contains("complete_assembly"))
                .unwrap_or(false))
        && nested_text(case, "evidence", "specimen_scope") != Some("complete_wall_assembly")
    {
        return decision(
            REJECT_SCOPE,
            "SMALL_FLAME_TEST_NOT_WHOLE_ASSEMBLY_FIRE_RESISTANCE",
        );
    }

    let declared_addenda = field(case, "standard")
        .and_then(|standard| standard.get("addenda"))
        .and_then(Value::as_array);
    match currentness {
        "INCOMPLETE_REFERENCE" | "SUPERSEDED" | "REVOKED" | "STALE" => {
            return decision(
                REJECT_STALE_OR_REVOKED,
                "STANDARD_REFERENCE_NOT_CURRENT_OR_COMPLETE",
            );
        }
        "OUT_OF_SCOPE" | "CONTEXT_ONLY" => {
            return decision(REJECT_SCOPE, "STANDARD_REFERENCE_OUT_OF_SCOPE");
        }
        "UNKNOWN" | "DRAFT" | "UNVERIFIED" | "VERIFY_LOCALLY" | "DRAFT_OR_ENQUIRY" => {
            return decision(ABSTAIN_UNKNOWN, "STANDARD_CURRENTNESS_UNKNOWN");
        }
        "CURRENT_WITH_AMENDMENTS" => {
            if declared_addenda.map(Vec::is_empty).unwrap_or(true) {
                return decision(
                    REJECT_STALE_OR_REVOKED,
                    "CURRENT_AMENDED_REFERENCE_MISSING_APPLICABLE_ADDENDA",
                );
            }
        }
        "CURRENT" | "CURRENT_SNAPSHOT" | "CURRENT_REFERENCE" | "CURRENT_PUBLISHED"
        | "SNAPSHOT_GUIDANCE" => {}
        _ => return decision(ABSTAIN_UNKNOWN, "UNRECOGNIZED_STANDARD_CURRENTNESS"),
    }
    if standard_id == "UNSPECIFIED" || standard_scope == "UNKNOWN" || standard_scope.is_empty() {
        return decision(ABSTAIN_UNKNOWN, "STANDARD_ID_OR_APPLICABILITY_UNKNOWN");
    }

    // NIJ 0123.00 defines protection levels and associated test threats. It
    // does not itself specify a product-performance test method. A torso
    // performance claim based on 0123.00 alone must therefore abstain.
    if domain == "human_worn"
        && standard_id == "NIJ-0123.00"
        && nested_bool(case, "claim", "performance_claim_permitted") == Some(true)
    {
        return decision(
            ABSTAIN_UNKNOWN,
            "THREAT_SPECIFICATION_NOT_PRODUCT_TEST_METHOD",
        );
    }

    // Configuration changes do not inherit prior evidence automatically.
    let tested_revision = nested_text(case, "evidence", "tested_configuration_revision");
    let current_revision = nested_text(case, "subject", "configuration_revision");
    let changed_revision = tested_revision
        .zip(current_revision)
        .map(|(tested, current)| tested != current)
        .unwrap_or(false);
    if changed_revision || is_true(case, "evidence", "material_lot_changed") {
        return decision(
            REQUIRE_INDEPENDENT_REVIEW,
            "CONFIGURATION_CHANGED_AFTER_TEST",
        );
    }

    // Model-predicted results do not substantiate physical performance claims.
    if evidence_class == "model_predicted"
        && nested_bool(case, "claim", "performance_claim_permitted") == Some(true)
    {
        return decision(
            ABSTAIN_UNKNOWN,
            "MODEL_PREDICTION_NOT_COMPLETE_ARTICLE_TEST",
        );
    }

    // Ballistic compliance does not establish wearer fit, mobility, task
    // performance or thermal/moisture usability. If one of those outcomes is
    // claimed, require a separate wearer-assessment record bound to this
    // configuration. These references establish record completeness only.
    let wearer_usability_claim = [
        "wearer",
        "fit",
        "mobility",
        "thermal",
        "moisture",
        "task_performance",
    ]
    .iter()
    .any(|term| proposition.contains(*term));
    if domain == "human_worn"
        && wearer_usability_claim
        && nested_bool(case, "claim", "performance_claim_permitted") == Some(true)
    {
        let wearer_evidence_complete = has_nonempty_text(case, "evidence", "wearer_assessment_ref")
            && has_nonempty_text(case, "evidence", "anthropometry_profile_ref")
            && has_nonempty_text(case, "evidence", "movement_task_protocol_ref")
            && has_nonempty_text(case, "evidence", "mobility_outcomes_ref")
            && has_nonempty_text(case, "evidence", "thermal_moisture_outcomes_ref");
        if !wearer_evidence_complete {
            return decision(
                ABSTAIN_UNKNOWN,
                "WEARER_USABILITY_CLAIM_MISSING_WEARER_EVIDENCE",
            );
        }
    }

    // Explicitly incomplete records represent unknown dimensions, not zeros.
    if evidence_class == "incomplete" || is_true(case, "evidence", "missing_values_encoded_as_zero")
    {
        return decision(
            ABSTAIN_UNKNOWN,
            "INCOMPLETE_OR_UNKNOWN_PROTECTION_DIMENSIONS",
        );
    }
    if evidence_class != "synthetic_test_report" {
        return decision(
            ABSTAIN_UNKNOWN,
            "EVIDENCE_CLASS_NOT_ACCEPTED_BY_SYNTHETIC_VERIFIER",
        );
    }

    // Domain-specific minimum record requirements; this is not a test-result evaluator.
    let complete = match domain {
        "human_worn" => {
            has_nonempty_text(case, "evidence", "report_id")
                && has_nonempty_text(case, "evidence", "lab_scope")
                && has_nonempty_text(case, "evidence", "calibration_ref")
                && has_nonempty_text(case, "evidence", "conditioning_record")
                && is_true(case, "evidence", "units_present")
                && is_true(case, "evidence", "uncertainty_present")
                && is_true(case, "evidence", "configuration_bound")
        }
        "vehicle" => {
            has_nonempty_text(case, "evidence", "report_id")
                && is_true(case, "evidence", "vehicle_identity_bound")
                && is_true(case, "evidence", "mass_recorded")
                && is_true(case, "evidence", "restraint_configuration_bound")
                && is_true(case, "evidence", "units_present")
                && is_true(case, "evidence", "uncertainty_present")
        }
        "housing" => {
            has_nonempty_text(case, "evidence", "report_id")
                && is_true(case, "evidence", "drawings_bound")
                && is_true(case, "evidence", "assembly_bound")
                && is_true(case, "evidence", "egress_recorded")
                && is_true(case, "evidence", "ventilation_recorded")
                && is_true(case, "evidence", "uncertainty_present")
        }
        "cross_domain" => has_nonempty_text(case, "evidence", "report_id"),
        _ => false,
    };

    if !complete {
        return decision(
            REJECT_MISSING_OR_INVALID,
            "DOMAIN_REQUIRED_RECORD_FIELDS_MISSING",
        );
    }

    // A positive B0 outcome accepts only the completeness of the record.
    decision(
        ACCEPT_RECORD_ONLY,
        "RECORD_FIELDS_COMPLETE_NO_PHYSICAL_CLAIM",
    )
}

/// Validate the corpus and return deterministic case-level conformance results.
pub fn evaluate_corpus(root: &Value) -> Result<Report, ValidationError> {
    let errors = structural_errors(root);
    if !errors.is_empty() {
        return Err(ValidationError { messages: errors });
    }

    let cases = root["cases"].as_array().expect("validated cases array");
    let canonical = serde_json::to_vec(root).expect("JSON Value serialization cannot fail");
    let digest = Sha256::digest(&canonical);
    let digest_hex = format!("{digest:x}");

    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let case_id = text(case, "case_id").unwrap_or_default().to_owned();
        let expected = text(case, "expected_decision")
            .unwrap_or_default()
            .to_owned();
        let (actual, reason_code) = evaluate_case(case);
        let passed = actual == expected;
        results.push(CaseResult {
            case_id,
            expected_decision: expected,
            actual_decision: actual,
            passed,
            reason_code,
        });
    }

    let passed_count = results.iter().filter(|r| r.passed).count();
    let failed_count = results.len() - passed_count;
    Ok(Report {
        verifier: "protective-systems-verifier",
        verifier_version: env!("CARGO_PKG_VERSION"),
        schema_version: text(root, "schema_version").unwrap_or_default().to_owned(),
        evidence_class: "synthetic",
        claim_ceiling: "record_conformance_only_no_physical_performance_claim",
        canonical_corpus_sha256: digest_hex,
        case_count: results.len(),
        passed_count,
        failed_count,
        conformance_pass: failed_count == 0 && !results.is_empty(),
        cases: results,
    })
}

pub fn parse_corpus(input: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: &str = include_str!("../../fixtures/benchmark-cases-v0.1.json");
    const BODY_ARMOR_FIXTURES: &str =
        include_str!("../../fixtures/body-armor-cases-v0.1.json");

    fn body_armor_corpus() -> Value {
        parse_corpus(BODY_ARMOR_FIXTURES).expect("body-armour fixture corpus must parse")
    }

    fn corpus() -> Value {
        parse_corpus(FIXTURES).expect("embedded fixture corpus must parse")
    }

    #[test]
    fn frozen_v01_corpus_matches_every_expected_decision() {
        let report = evaluate_corpus(&corpus()).expect("corpus must validate");
        assert_eq!(report.case_count, 10);
        assert_eq!(report.passed_count, 10);
        assert_eq!(report.failed_count, 0);
        assert!(report.conformance_pass);
        assert_eq!(
            report.claim_ceiling,
            "record_conformance_only_no_physical_performance_claim"
        );
    }

    #[test]
    fn reject_all_and_accept_all_policies_fail_the_mixed_control_corpus() {
        let report = evaluate_corpus(&corpus()).expect("corpus must validate");
        let accept_all_matches = report
            .cases
            .iter()
            .all(|case| case.expected_decision == ACCEPT_RECORD_ONLY);
        let reject_all_matches = report
            .cases
            .iter()
            .all(|case| case.expected_decision == REJECT_MISSING_OR_INVALID);

        assert!(!accept_all_matches);
        assert!(!reject_all_matches);
        assert!(
            report
                .cases
                .iter()
                .any(|c| c.expected_decision == ACCEPT_RECORD_ONLY)
        );
        assert!(
            report
                .cases
                .iter()
                .any(|c| c.expected_decision == ABSTAIN_UNKNOWN)
        );
        assert!(
            report
                .cases
                .iter()
                .any(|c| c.expected_decision == REJECT_SCOPE)
        );
        assert!(
            report
                .cases
                .iter()
                .any(|c| c.expected_decision == REQUIRE_INDEPENDENT_REVIEW)
        );
    }

    #[test]
    fn signature_does_not_substitute_for_measurement() {
        let root = corpus();
        let result = evaluate_case(&root["cases"][7]);
        assert_eq!(result.0, REJECT_MISSING_OR_INVALID);
        assert_eq!(result.1, "SIGNATURE_WITHOUT_PHYSICAL_CORROBORATION");
    }

    #[test]
    fn unknown_is_not_zero_or_pass() {
        let root = corpus();
        let result = evaluate_case(&root["cases"][8]);
        assert_eq!(result.0, ABSTAIN_UNKNOWN);
    }

    #[test]
    fn configuration_change_requires_review() {
        let root = corpus();
        let result = evaluate_case(&root["cases"][9]);
        assert_eq!(result.0, REQUIRE_INDEPENDENT_REVIEW);
    }

    #[test]
    fn torso_standard_cannot_qualify_limb_accessory() {
        let root = corpus();
        let result = evaluate_case(&root["cases"][1]);
        assert_eq!(result.0, REJECT_SCOPE);
    }

    #[test]
    fn incomplete_standard_reference_is_rejected() {
        let root = corpus();
        let result = evaluate_case(&root["cases"][2]);
        assert_eq!(result.0, REJECT_STALE_OR_REVOKED);
    }

    #[test]
    fn deterministic_digest_and_ordered_results() {
        let first = evaluate_corpus(&corpus()).expect("first run");
        let second = evaluate_corpus(&corpus()).expect("second run");
        assert_eq!(
            first.canonical_corpus_sha256,
            second.canonical_corpus_sha256
        );
        let first_json = serde_json::to_string(&first).expect("serialize");
        let second_json = serde_json::to_string(&second).expect("serialize");
        assert_eq!(first_json, second_json);
    }

    #[test]
    fn missing_units_are_not_accepted() {
        let mut root = corpus();
        root["cases"][0]["evidence"]["units_present"] = serde_json::json!(false);
        let result = evaluate_case(&root["cases"][0]);
        assert_eq!(result.0, REJECT_MISSING_OR_INVALID);
    }

    #[test]
    fn unrecognized_standard_state_abstains_closed() {
        let mut root = corpus();
        root["cases"][0]["standard"]["currentness"] = serde_json::json!("MAYBE_CURRENT");
        let result = evaluate_case(&root["cases"][0]);
        assert_eq!(result.0, ABSTAIN_UNKNOWN);
        assert_eq!(result.1, "UNRECOGNIZED_STANDARD_CURRENTNESS");
    }

    #[test]
    fn supplier_declaration_is_not_a_synthetic_test_report() {
        let mut root = corpus();
        root["cases"][0]["evidence"]["class"] = serde_json::json!("supplier_declared");
        let result = evaluate_case(&root["cases"][0]);
        assert_eq!(result.0, ABSTAIN_UNKNOWN);
    }

    #[test]
    fn duplicate_addenda_are_rejected_structurally() {
        let mut root = corpus();
        root["cases"][0]["standard"]["addenda"] = serde_json::json!(["1", "1", "2", "3"]);
        let error = evaluate_corpus(&root).expect_err("duplicate addenda must fail");
        assert!(error.messages.iter().any(|m| m.contains("duplicate 1")));
    }

    #[test]
    fn standards_registry_current_published_status_is_supported() {
        let mut root = corpus();
        root["cases"][0]["standard"]["currentness"] = serde_json::json!("CURRENT_PUBLISHED");
        assert_eq!(evaluate_case(&root["cases"][0]).0, ACCEPT_RECORD_ONLY);
    }

    #[test]
    fn amended_registry_reference_requires_explicit_addenda() {
        let mut root = corpus();
        root["cases"][0]["standard"]["currentness"] = serde_json::json!("CURRENT_WITH_AMENDMENTS");
        root["cases"][0]["standard"]["addenda"] = serde_json::json!([]);
        let result = evaluate_case(&root["cases"][0]);
        assert_eq!(result.0, REJECT_STALE_OR_REVOKED);
        assert_eq!(
            result.1,
            "CURRENT_AMENDED_REFERENCE_MISSING_APPLICABLE_ADDENDA"
        );

        root["cases"][0]["standard"]["addenda"] = serde_json::json!(["Addendum 1"]);
        assert_eq!(evaluate_case(&root["cases"][0]).0, ACCEPT_RECORD_ONLY);
    }

    #[test]
    fn verify_locally_status_abstains_and_context_only_rejects_scope() {
        let mut root = corpus();
        root["cases"][0]["standard"]["currentness"] = serde_json::json!("VERIFY_LOCALLY");
        assert_eq!(evaluate_case(&root["cases"][0]).0, ABSTAIN_UNKNOWN);

        root["cases"][0]["standard"]["currentness"] = serde_json::json!("CONTEXT_ONLY");
        assert_eq!(evaluate_case(&root["cases"][0]).0, REJECT_SCOPE);
    }

    #[test]
    fn unknown_as_zero_is_rejected_even_when_standard_is_current() {
        let mut root = corpus();
        root["cases"][8]["standard"]["id"] = serde_json::json!("DECLARED-CURRENT-METHOD");
        root["cases"][8]["standard"]["currentness"] = serde_json::json!("CURRENT");
        root["cases"][8]["standard"]["scope"] = serde_json::json!("declared_product_scope");
        let result = evaluate_case(&root["cases"][8]);
        assert_eq!(result.0, ABSTAIN_UNKNOWN);
        assert_eq!(result.1, "INCOMPLETE_OR_UNKNOWN_PROTECTION_DIMENSIONS");
    }

    #[test]
    fn body_armor_v01_corpus_matches_every_expected_decision() {
        let report = evaluate_corpus(&body_armor_corpus()).expect("body-armour corpus must validate");
        assert_eq!(report.case_count, 8);
        assert_eq!(report.passed_count, 8);
        assert_eq!(report.failed_count, 0);
        assert!(report.conformance_pass);
        assert_eq!(
            report.claim_ceiling,
            "record_conformance_only_no_physical_performance_claim"
        );
    }

    #[test]
    fn nij_0123_threat_spec_is_not_a_product_test_method() {
        let root = body_armor_corpus();
        let result = evaluate_case(&root["cases"][1]);
        assert_eq!(result.0, ABSTAIN_UNKNOWN);
        assert_eq!(result.1, "THREAT_SPECIFICATION_NOT_PRODUCT_TEST_METHOD");
    }

    #[test]
    fn south_african_sans_currentness_must_be_verified_locally() {
        let root = body_armor_corpus();
        let result = evaluate_case(&root["cases"][2]);
        assert_eq!(result.0, ABSTAIN_UNKNOWN);
        assert_eq!(result.1, "STANDARD_CURRENTNESS_UNKNOWN");
    }

    #[test]
    fn body_armor_configuration_change_requires_independent_review() {
        let root = body_armor_corpus();
        let result = evaluate_case(&root["cases"][3]);
        assert_eq!(result.0, REQUIRE_INDEPENDENT_REVIEW);
        assert_eq!(result.1, "CONFIGURATION_CHANGED_AFTER_TEST");
    }

    #[test]
    fn wearer_usability_claim_requires_separate_wearer_evidence() {
        let root = body_armor_corpus();
        let missing = evaluate_case(&root["cases"][4]);
        assert_eq!(missing.0, ABSTAIN_UNKNOWN);
        assert_eq!(missing.1, "WEARER_USABILITY_CLAIM_MISSING_WEARER_EVIDENCE");

        let complete = evaluate_case(&root["cases"][5]);
        assert_eq!(complete.0, ACCEPT_RECORD_ONLY);
        assert_eq!(complete.1, "RECORD_FIELDS_COMPLETE_NO_PHYSICAL_CLAIM");
    }

    #[test]
    fn model_only_injury_reduction_claim_abstains() {
        let root = body_armor_corpus();
        let result = evaluate_case(&root["cases"][6]);
        assert_eq!(result.0, ABSTAIN_UNKNOWN);
        assert_eq!(result.1, "MODEL_PREDICTION_NOT_COMPLETE_ARTICLE_TEST");
    }

    #[test]
    fn body_armor_current_amended_standard_requires_addenda() {
        let root = body_armor_corpus();
        let result = evaluate_case(&root["cases"][7]);
        assert_eq!(result.0, REJECT_STALE_OR_REVOKED);
        assert_eq!(result.1, "CURRENT_AMENDED_REFERENCE_MISSING_APPLICABLE_ADDENDA");
    }

    #[test]
    fn structural_validator_rejects_duplicate_case_ids() {
        let mut root = corpus();
        root["cases"][1]["case_id"] = serde_json::json!("HUMAN-POSITIVE-TORSO-001");
        let error = evaluate_corpus(&root).expect_err("duplicate IDs must fail");
        assert!(error.messages.iter().any(|m| m.contains("duplicates")));
    }
}
