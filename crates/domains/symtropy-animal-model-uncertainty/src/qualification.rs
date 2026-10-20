// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decision qualification relative to a source-bound model-set adequacy receipt.
//!
//! Numerical risk scores remain conditional on the model set supplied to the
//! decision kernel. This module prevents those scores from silently becoming
//! a claim of decision robustness when blind challenge adequacy is missing,
//! failed, unresolved, incomplete, or bound to a different declared scope.

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Frozen shape contract for an externally generated adequacy assessment receipt.
pub const EMBEDDED_RECEIPT_SCHEMA: &str = include_str!(
    "../../../../docs/living-world/ANIMAL_MODEL_SET_ADEQUACY_ASSESSMENT_RECEIPT_V1.schema.json"
);

const RECEIPT_SCHEMA_VERSION: &str = "ANIMAL-MODEL-SET-ADEQUACY-ASSESSMENT-RECEIPT-V1";
const RECEIPT_STATUS: &str = "blind_challenge_assessment_receipt";
const RECEIPT_FIELDS: &[&str] = &[
    "schema_version",
    "status",
    "assessment_id",
    "disposition",
    "candidate_model_set_digest",
    "challenge_family_manifest_sha256",
    "source_commit",
    "evaluator_version",
    "scope",
    "independent_unit_manifest_sha256",
    "pre_reveal_prediction_digest",
    "challenger_case_count",
    "independent_unit_count",
];

const SCOPE_STATEMENT: &str =
    "Qualification applies only to the declared model registry and blinded challenge scope; it is not universal model completeness or empirical biological validation.";

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionQualification {
    pub assessment_disposition: String,
    pub qualification: String,
    pub reason_codes: Vec<String>,
    pub assessment_receipt_sha256: Option<String>,
    pub candidate_model_set_digest: Option<String>,
    pub challenge_family_manifest_sha256: Option<String>,
    pub source_commit: Option<String>,
    pub evaluator_version: Option<String>,
    pub scope: Option<String>,
}

impl DecisionQualification {
    /// Evaluate an optional receipt against the expected model-registry and
    /// challenger-family identities declared by the current decision input.
    ///
    /// Absence means an unassessed result; malformed supplied evidence is
    /// rejected. A matching receipt qualifies only the named scope and depends
    /// on a trusted external process to establish issuer authenticity.
    pub fn evaluate(
        receipt_raw: Option<&str>,
        expected_model_registry_digest: Option<&str>,
        expected_challenge_manifest_digest: Option<&str>,
    ) -> Result<Self, Vec<String>> {
        let Some(receipt_raw) = receipt_raw else {
            return Ok(Self::not_assessed("assessment_receipt_not_supplied"));
        };

        let receipt: Value = serde_json::from_str(receipt_raw)
            .map_err(|error| vec![format!("invalid adequacy receipt JSON: {error}")])?;
        let Some(object) = receipt.as_object() else {
            return Err(vec!["adequacy receipt root must be an object".to_owned()]);
        };

        let mut errors = validate_receipt_object(object);
        let schema: Value = serde_json::from_str(EMBEDDED_RECEIPT_SCHEMA)
            .map_err(|error| vec![format!("invalid embedded adequacy receipt schema: {error}")])?;
        errors.extend(validate_embedded_schema_contract(&schema));
        if !errors.is_empty() {
            return Err(errors);
        }

        let disposition = required_string(object, "disposition");
        let model_digest = required_string(object, "candidate_model_set_digest");
        let challenge_digest = required_string(object, "challenge_family_manifest_sha256");
        let source_commit = required_string(object, "source_commit");
        let evaluator_version = required_string(object, "evaluator_version");
        let scope = required_string(object, "scope");
        let receipt_sha256 = sha256_hex(receipt_raw.as_bytes());

        let (qualification, reason_codes) = match (
            expected_model_registry_digest,
            expected_challenge_manifest_digest,
        ) {
            (None, _) | (_, None) => (
                "not_assessed",
                vec!["expected_scope_identity_missing".to_owned()],
            ),
            (Some(expected_model), Some(expected_challenge))
                if expected_model != model_digest || expected_challenge != challenge_digest =>
            {
                let mut mismatches = Vec::new();
                if expected_model != model_digest {
                    mismatches.push("candidate_model_registry_digest_mismatch".to_owned());
                }
                if expected_challenge != challenge_digest {
                    mismatches.push("challenge_family_manifest_digest_mismatch".to_owned());
                }
                ("blocked_unresolved", mismatches)
            }
            (Some(_), Some(_)) => match disposition {
                "qualified_within_challenge_scope" => ("within_scope", Vec::new()),
                "omission_detected" => (
                    "blocked_by_omission",
                    vec!["model_set_omission_detected".to_owned()],
                ),
                "unresolved" => (
                    "blocked_unresolved",
                    vec!["assessment_unresolved".to_owned()],
                ),
                "incomplete" => (
                    "blocked_incomplete",
                    vec!["assessment_incomplete".to_owned()],
                ),
                _ => unreachable!("receipt validation rejects unknown dispositions"),
            },
        };

        Ok(Self {
            assessment_disposition: disposition.to_owned(),
            qualification: qualification.to_owned(),
            reason_codes,
            assessment_receipt_sha256: Some(receipt_sha256),
            candidate_model_set_digest: Some(model_digest.to_owned()),
            challenge_family_manifest_sha256: Some(challenge_digest.to_owned()),
            source_commit: Some(source_commit.to_owned()),
            evaluator_version: Some(evaluator_version.to_owned()),
            scope: Some(scope.to_owned()),
        })
    }

    fn not_assessed(reason: &str) -> Self {
        Self {
            assessment_disposition: "not_assessed".to_owned(),
            qualification: "not_assessed".to_owned(),
            reason_codes: vec![reason.to_owned()],
            assessment_receipt_sha256: None,
            candidate_model_set_digest: None,
            challenge_family_manifest_sha256: None,
            source_commit: None,
            evaluator_version: None,
            scope: None,
        }
    }

    /// Serialize a qualification envelope for inclusion in a decision report.
    pub fn to_value(&self) -> Value {
        json!({
            "schema_version": "ANIMAL-DECISION-QUALIFICATION-V1",
            "assessment_disposition": self.assessment_disposition,
            "qualification": self.qualification,
            "reason_codes": self.reason_codes,
            "assessment_receipt_sha256": self.assessment_receipt_sha256,
            "candidate_model_set_digest": self.candidate_model_set_digest,
            "challenge_family_manifest_sha256": self.challenge_family_manifest_sha256,
            "source_commit": self.source_commit,
            "evaluator_version": self.evaluator_version,
            "scope": self.scope,
            "scope_statement": SCOPE_STATEMENT,
            "does_not_validate_empirical_biology": true
        })
    }
}

fn validate_receipt_object(object: &Map<String, Value>) -> Vec<String> {
    let mut errors = Vec::new();
    let allowed: BTreeSet<&str> = RECEIPT_FIELDS.iter().copied().collect();
    for key in object.keys() {
        if !allowed.contains(key.as_str()) {
            errors.push(format!("adequacy receipt contains unknown field '{key}'"));
        }
    }
    for field in RECEIPT_FIELDS {
        if !object.contains_key(*field) {
            errors.push(format!("adequacy receipt is missing required field '{field}'"));
        }
    }

    for (field, expected) in [
        ("schema_version", RECEIPT_SCHEMA_VERSION),
        ("status", RECEIPT_STATUS),
    ] {
        if object.get(field).and_then(Value::as_str) != Some(expected) {
            errors.push(format!("adequacy receipt {field} must equal '{expected}'"));
        }
    }

    for field in ["assessment_id", "evaluator_version", "scope"] {
        if !object
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        {
            errors.push(format!("adequacy receipt {field} must be a non-empty string"));
        }
    }

    if let Some(disposition) = object.get("disposition").and_then(Value::as_str) {
        if !matches!(
            disposition,
            "qualified_within_challenge_scope" | "omission_detected" | "unresolved" | "incomplete"
        ) {
            errors.push("adequacy receipt disposition is not recognized".to_owned());
        }
    } else {
        errors.push("adequacy receipt disposition must be a string".to_owned());
    }

    for field in [
        "candidate_model_set_digest",
        "challenge_family_manifest_sha256",
        "independent_unit_manifest_sha256",
        "pre_reveal_prediction_digest",
    ] {
        if !object
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(is_sha256_hex)
        {
            errors.push(format!("adequacy receipt {field} must be 64 lowercase hex characters"));
        }
    }
    if !object
        .get("source_commit")
        .and_then(Value::as_str)
        .is_some_and(is_git_commit_hex)
    {
        errors.push("adequacy receipt source_commit must be a 40-64 character lowercase Git SHA".to_owned());
    }
    for field in ["challenger_case_count", "independent_unit_count"] {
        if !object.get(field).and_then(Value::as_u64).is_some_and(|value| value > 0) {
            errors.push(format!("adequacy receipt {field} must be a positive integer"));
        }
    }
    errors
}

fn validate_embedded_schema_contract(schema: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        errors.push("embedded adequacy receipt schema must use JSON Schema Draft 2020-12".to_owned());
    }
    if schema.pointer("/properties/schema_version/const").and_then(Value::as_str)
        != Some(RECEIPT_SCHEMA_VERSION)
    {
        errors.push("embedded adequacy receipt schema version does not match validator".to_owned());
    }
    if schema.get("type").and_then(Value::as_str) != Some("object") {
        errors.push("embedded adequacy receipt schema root must be an object".to_owned());
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        errors.push("embedded adequacy receipt schema must reject unknown root fields".to_owned());
    }
    for (pointer, expected, label) in [
        (
            "/properties/candidate_model_set_digest/pattern",
            "^[0-9a-f]{64}$",
            "candidate model-set digest pattern",
        ),
        (
            "/properties/challenge_family_manifest_sha256/pattern",
            "^[0-9a-f]{64}$",
            "challenge manifest digest pattern",
        ),
        (
            "/properties/independent_unit_manifest_sha256/pattern",
            "^[0-9a-f]{64}$",
            "independent unit digest pattern",
        ),
        (
            "/properties/pre_reveal_prediction_digest/pattern",
            "^[0-9a-f]{64}$",
            "pre-reveal prediction digest pattern",
        ),
        (
            "/properties/source_commit/pattern",
            "^[0-9a-f]{40,64}$",
            "source commit pattern",
        ),
    ] {
        if schema.pointer(pointer).and_then(Value::as_str) != Some(expected) {
            errors.push(format!("embedded adequacy receipt schema {label} changed"));
        }
    }
    let schema_required: BTreeSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let expected_required: BTreeSet<&str> = RECEIPT_FIELDS.iter().copied().collect();
    if schema_required != expected_required {
        errors.push("embedded adequacy receipt schema required fields do not match validator".to_owned());
    }
    let schema_dispositions: BTreeSet<&str> = schema
        .pointer("/properties/disposition/enum")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let expected_dispositions: BTreeSet<&str> = [
        "qualified_within_challenge_scope",
        "omission_detected",
        "unresolved",
        "incomplete",
    ]
    .into_iter()
    .collect();
    if schema_dispositions != expected_dispositions {
        errors.push("embedded adequacy receipt schema dispositions do not match validator".to_owned());
    }
    errors
}

fn required_string<'a>(object: &'a Map<String, Value>, field: &str) -> &'a str {
    object
        .get(field)
        .and_then(Value::as_str)
        .expect("required strings checked by validate_receipt_object")
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_git_commit_hex(value: &str) -> bool {
    (40..=64).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing to String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODEL_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CHALLENGE_DIGEST: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn receipt(disposition: &str) -> String {
        json!({
            "schema_version": RECEIPT_SCHEMA_VERSION,
            "status": RECEIPT_STATUS,
            "assessment_id": "test-assessment-1",
            "disposition": disposition,
            "candidate_model_set_digest": MODEL_DIGEST,
            "challenge_family_manifest_sha256": CHALLENGE_DIGEST,
            "source_commit": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "evaluator_version": "test-evaluator-v1",
            "scope": "synthetic trophic-link omission challenge only",
            "independent_unit_manifest_sha256": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "pre_reveal_prediction_digest": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "challenger_case_count": 52,
            "independent_unit_count": 20
        })
        .to_string()
    }

    #[test]
    fn missing_receipt_is_not_assessed() {
        let result = DecisionQualification::evaluate(None, Some(MODEL_DIGEST), Some(CHALLENGE_DIGEST))
            .expect("absence is an explicit unassessed result");
        assert_eq!(result.assessment_disposition, "not_assessed");
        assert_eq!(result.qualification, "not_assessed");
        assert_eq!(result.reason_codes, vec!["assessment_receipt_not_supplied".to_owned()]);
        assert!(result.assessment_receipt_sha256.is_none());
    }

    #[test]
    fn matching_assessment_receipt_qualifies_only_its_declared_scope() {
        let raw = receipt("qualified_within_challenge_scope");
        let result = DecisionQualification::evaluate(
            Some(&raw),
            Some(MODEL_DIGEST),
            Some(CHALLENGE_DIGEST),
        )
        .expect("valid bound assessment receipt");
        assert_eq!(result.assessment_disposition, "qualified_within_challenge_scope");
        assert_eq!(result.qualification, "within_scope");
        assert!(result.reason_codes.is_empty());
        assert_eq!(result.assessment_receipt_sha256.as_deref(), Some(sha256_hex(raw.as_bytes()).as_str()));
        let output = result.to_value();
        assert_eq!(output["does_not_validate_empirical_biology"], true);
        assert!(output["scope_statement"].as_str().unwrap().contains("not universal model completeness"));
    }

    #[test]
    fn registry_mismatch_blocks_qualification() {
        let raw = receipt("qualified_within_challenge_scope");
        let result = DecisionQualification::evaluate(
            Some(&raw),
            Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
            Some(CHALLENGE_DIGEST),
        )
        .expect("scope mismatch is reported, not accepted");
        assert_eq!(result.qualification, "blocked_unresolved");
        assert_eq!(
            result.reason_codes,
            vec!["candidate_model_registry_digest_mismatch".to_owned()]
        );
    }

    #[test]
    fn challenge_family_mismatch_blocks_qualification() {
        let raw = receipt("qualified_within_challenge_scope");
        let result = DecisionQualification::evaluate(
            Some(&raw),
            Some(MODEL_DIGEST),
            Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),
        )
        .expect("scope mismatch is reported, not accepted");
        assert_eq!(result.qualification, "blocked_unresolved");
        assert_eq!(
            result.reason_codes,
            vec!["challenge_family_manifest_digest_mismatch".to_owned()]
        );
    }

    #[test]
    fn missing_expected_scope_identity_is_not_assessed() {
        let raw = receipt("qualified_within_challenge_scope");
        let result = DecisionQualification::evaluate(Some(&raw), None, Some(CHALLENGE_DIGEST))
            .expect("missing comparison identity maps to unassessed");
        assert_eq!(result.assessment_disposition, "qualified_within_challenge_scope");
        assert_eq!(result.qualification, "not_assessed");
        assert_eq!(result.reason_codes, vec!["expected_scope_identity_missing".to_owned()]);
    }

    #[test]
    fn every_nonpositive_disposition_blocks_qualification() {
        for (disposition, expected) in [
            ("omission_detected", "blocked_by_omission"),
            ("unresolved", "blocked_unresolved"),
            ("incomplete", "blocked_incomplete"),
        ] {
            let raw = receipt(disposition);
            let result = DecisionQualification::evaluate(
                Some(&raw),
                Some(MODEL_DIGEST),
                Some(CHALLENGE_DIGEST),
            )
            .expect("valid negative assessment is retained");
            assert_eq!(result.assessment_disposition, disposition);
            assert_eq!(result.qualification, expected);
        }
    }

    #[test]
    fn malformed_receipt_digest_is_rejected() {
        let mut value: Value = serde_json::from_str(&receipt("qualified_within_challenge_scope"))
            .expect("receipt JSON");
        value["candidate_model_set_digest"] = Value::String("not-a-digest".to_owned());
        let errors = DecisionQualification::evaluate(
            Some(&value.to_string()),
            Some(MODEL_DIGEST),
            Some(CHALLENGE_DIGEST),
        )
        .expect_err("malformed digest must fail closed");
        assert!(errors.iter().any(|error| error.contains("candidate_model_set_digest")));
    }

    #[test]
    fn unknown_receipt_fields_are_rejected() {
        let mut value: Value = serde_json::from_str(&receipt("qualified_within_challenge_scope"))
            .expect("receipt JSON");
        value["hidden_truth"] = Value::String("unreviewed".to_owned());
        let errors = DecisionQualification::evaluate(
            Some(&value.to_string()),
            Some(MODEL_DIGEST),
            Some(CHALLENGE_DIGEST),
        )
        .expect_err("unknown field must fail closed");
        assert!(errors.iter().any(|error| error.contains("hidden_truth")));
    }

    #[test]
    fn receipt_digest_binds_exact_supplied_bytes() {
        let raw = receipt("qualified_within_challenge_scope");
        let with_newline = format!("{raw}\n");
        let first = DecisionQualification::evaluate(
            Some(&raw),
            Some(MODEL_DIGEST),
            Some(CHALLENGE_DIGEST),
        )
        .expect("valid receipt");
        let second = DecisionQualification::evaluate(
            Some(&with_newline),
            Some(MODEL_DIGEST),
            Some(CHALLENGE_DIGEST),
        )
        .expect("whitespace does not change receipt semantics");
        assert_ne!(first.assessment_receipt_sha256, second.assessment_receipt_sha256);
    }

    #[test]
    fn schema_cannot_be_weakened_to_allow_unknown_fields() {
        let mut schema: Value =
            serde_json::from_str(EMBEDDED_RECEIPT_SCHEMA).expect("schema JSON should parse");
        schema["additionalProperties"] = Value::Bool(true);
        let errors = validate_embedded_schema_contract(&schema);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("reject unknown root fields"))
        );

        schema["additionalProperties"] = Value::Bool(false);
        schema["properties"]["candidate_model_set_digest"]["pattern"] = Value::String(".*".to_owned());
        let errors = validate_embedded_schema_contract(&schema);
        assert!(
            errors
                .iter()
                .any(|error| error.contains("candidate model-set digest pattern"))
        );
    }

    #[test]
    fn schema_contract_has_frozen_fields_and_dispositions() {
        let schema: Value =
            serde_json::from_str(EMBEDDED_RECEIPT_SCHEMA).expect("schema JSON should parse");
        assert_eq!(
            schema["properties"]["schema_version"]["const"].as_str(),
            Some(RECEIPT_SCHEMA_VERSION)
        );
        assert_eq!(
            schema["properties"]["disposition"]["enum"].as_array().map(Vec::len),
            Some(4)
        );
        assert!(validate_embedded_schema_contract(&schema).is_empty());
    }
}
