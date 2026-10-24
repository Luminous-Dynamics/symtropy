// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Catalogs jurisdiction source records without turning metadata into a legal verdict.
//! Local snapshot, review, implementation, and test artifacts are hash-checked when
//! present. Readiness is computed and never trusted from an input boolean.

use jsonschema::draft202012;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path},
};

const EMBEDDED_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/jurisdiction-evidence-registry.schema.json");

#[derive(Debug, Serialize)]
pub struct RecordReadiness {
    pub record_id: String,
    pub lifecycle_status: String,
    pub applicability_status: String,
    pub source_snapshot_digest_verified: bool,
    pub review_evidence_digest_verified: bool,
    pub translation_review_evidence_digest_verified: bool,
    pub rule_implementation_digest_verified: bool,
    pub rule_test_corpus_digest_verified: bool,
    pub ready_for_rule_evaluation: bool,
    pub readiness_reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RegistryValidationReport {
    pub validator_version: String,
    pub schema_sha256: String,
    pub registry_id: Option<String>,
    pub schema_valid: bool,
    pub semantic_valid: bool,
    pub registry_contract_status: String,
    pub source_snapshot_digests_verified: usize,
    pub review_evidence_digests_verified: usize,
    pub translation_review_evidence_digests_verified: usize,
    pub rule_artifact_digests_verified: usize,
    pub ready_for_rule_evaluation_count: usize,
    pub schema_issues: Vec<String>,
    pub semantic_issues: Vec<String>,
    pub records: Vec<RecordReadiness>,
    pub limitations: Vec<String>,
}

pub fn validate_path(path: &Path) -> Result<RegistryValidationReport, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let registry: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    validate_value(&registry, path.parent().unwrap_or_else(|| Path::new(".")))
}

pub fn validate_value(
    registry: &Value,
    artifact_root: &Path,
) -> Result<RegistryValidationReport, String> {
    let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA)
        .map_err(|error| format!("embedded jurisdiction registry schema is invalid JSON: {error}"))?;
    if !all_refs_are_local(&schema) {
        return Err("jurisdiction registry schema contains an external reference".to_string());
    }
    draft202012::meta::validate(&schema).map_err(|error| {
        format!("jurisdiction registry schema fails Draft 2020-12 meta-validation: {error}")
    })?;
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("cannot compile jurisdiction registry schema: {error}"))?;
    let schema_issues: Vec<String> = validator
        .iter_errors(registry)
        .map(|error| format!("schema validation: {error}"))
        .collect();
    let schema_valid = schema_issues.is_empty();
    let mut semantic_issues = Vec::new();
    let mut records = Vec::new();
    let mut ids = BTreeSet::new();
    let mut source_snapshot_digests_verified = 0;
    let mut review_evidence_digests_verified = 0;
    let mut translation_review_evidence_digests_verified = 0;
    let mut rule_artifact_digests_verified = 0;

    if schema_valid {
        let evaluation_date = registry["evaluation_date"].as_str().unwrap_or_default();
        let jurisdiction_confirmed =
            registry["jurisdiction"]["resolution_status"].as_str() == Some("confirmed")
                && registry["jurisdiction"]["country_code"].as_str().is_some()
                && registry["jurisdiction"]["municipality"]
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty());

        for (index, record) in registry["source_records"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let record_id = record["record_id"].as_str().unwrap_or_default();
            if !ids.insert(record_id.to_owned()) {
                semantic_issues.push(format!(
                    "source_records[{index}].record_id duplicates {record_id:?}"
                ));
            }
            let lifecycle_status = record["lifecycle_status"].as_str().unwrap_or("unknown");
            let applicability_status =
                record["applicability_status"].as_str().unwrap_or("not_assessed");
            let publication_postdates_evaluation = record["publication_date"]
                .as_str()
                .is_some_and(|date| date > evaluation_date);
            let snapshot_capture_postdates_evaluation = record["source_snapshot"]["captured_at"]
                .as_str()
                .is_some_and(|value| value.get(..10).unwrap_or_default() > evaluation_date);
            let review_postdates_evaluation = record["review_record"]["reviewed_at"]
                .as_str()
                .is_some_and(|value| value.get(..10).unwrap_or_default() > evaluation_date);
            let translation_review_postdates_evaluation = record["rule_translation"]
                ["translation_review"]["reviewed_at"]
                .as_str()
                .is_some_and(|value| value.get(..10).unwrap_or_default() > evaluation_date);

            if publication_postdates_evaluation {
                semantic_issues.push(format!(
                    "source record {record_id:?} has a publication date after registry evaluation_date"
                ));
            }
            if snapshot_capture_postdates_evaluation {
                semantic_issues.push(format!(
                    "source record {record_id:?} has a snapshot capture after registry evaluation_date"
                ));
            }

            let artifact_roles = [
                ("source snapshot", record["source_snapshot"]["artifact"]["sha256"].as_str()),
                ("applicability review evidence", record["review_record"]["evidence_artifact"]["sha256"].as_str()),
                ("translation review evidence", record["rule_translation"]["translation_review"]["evidence_artifact"]["sha256"].as_str()),
                ("rule implementation", record["rule_translation"]["implementation_artifact"]["sha256"].as_str()),
                ("rule test corpus", record["rule_translation"]["test_corpus_artifact"]["sha256"].as_str()),
            ];
            let mut declared_artifact_digests = BTreeSet::new();
            let mut artifact_roles_distinct = true;
            for (role, digest) in artifact_roles {
                if let Some(digest) = digest
                    && !declared_artifact_digests.insert(digest)
                {
                    artifact_roles_distinct = false;
                    semantic_issues.push(format!(
                        "source record {record_id:?} reuses a SHA-256 digest across evidence roles (duplicate at {role})"
                    ));
                }
            }

            let effective_from = record["effective_from"].as_str();
            let effective_to = record["effective_to"].as_str();
            let mut date_window_valid = true;

            if let (Some(from), Some(to)) = (effective_from, effective_to)
                && from > to
            {
                date_window_valid = false;
                semantic_issues.push(format!(
                    "source record {record_id:?} has effective_to before effective_from"
                ));
            }
            if let Some(from) = effective_from
                && from > evaluation_date
            {
                date_window_valid = false;
                if lifecycle_status == "reported_in_force" {
                    semantic_issues.push(format!(
                        "source record {record_id:?} is reported in force before effective_from"
                    ));
                }
            }
            if let Some(to) = effective_to
                && to < evaluation_date
                && lifecycle_status == "reported_in_force"
            {
                date_window_valid = false;
                semantic_issues.push(format!(
                    "source record {record_id:?} is reported in force after effective_to"
                ));
            }
            if lifecycle_status == "proposed"
                && record["review_record"]["decision"].as_str()
                    == Some("applicable_for_evaluation")
            {
                semantic_issues.push(format!(
                    "proposed source record {record_id:?} cannot be marked applicable for rule evaluation"
                ));
            }

            let mut readiness_reasons = Vec::new();
            if publication_postdates_evaluation {
                readiness_reasons.push("SOURCE_PUBLICATION_POSTDATES_EVALUATION_DATE".to_string());
            }
            if snapshot_capture_postdates_evaluation {
                readiness_reasons.push("SOURCE_SNAPSHOT_POSTDATES_EVALUATION_DATE".to_string());
            }
            let snapshot_verified = verify_artifact(
                record.get("source_snapshot").and_then(|value| value.get("artifact")),
                artifact_root,
                &mut semantic_issues,
                &format!("source record {record_id:?} source snapshot"),
            );
            if snapshot_verified {
                source_snapshot_digests_verified += 1;
            }
            let review_verified = verify_artifact(
                record.get("review_record").and_then(|value| value.get("evidence_artifact")),
                artifact_root,
                &mut semantic_issues,
                &format!("source record {record_id:?} applicability review evidence"),
            );
            if review_verified {
                review_evidence_digests_verified += 1;
            }
            let translation_review_verified = verify_artifact(
                record.get("rule_translation")
                    .and_then(|value| value.get("translation_review"))
                    .and_then(|value| value.get("evidence_artifact")),
                artifact_root,
                &mut semantic_issues,
                &format!("source record {record_id:?} translation review evidence"),
            );
            if translation_review_verified {
                translation_review_evidence_digests_verified += 1;
            }
            let implementation_verified = verify_artifact(
                record.get("rule_translation").and_then(|value| value.get("implementation_artifact")),
                artifact_root,
                &mut semantic_issues,
                &format!("source record {record_id:?} rule implementation"),
            );
            let corpus_verified = verify_artifact(
                record.get("rule_translation").and_then(|value| value.get("test_corpus_artifact")),
                artifact_root,
                &mut semantic_issues,
                &format!("source record {record_id:?} rule test corpus"),
            );
            if implementation_verified {
                rule_artifact_digests_verified += 1;
            }
            if corpus_verified {
                rule_artifact_digests_verified += 1;
            }

            if lifecycle_status == "proposed" {
                readiness_reasons.push("PROPOSED_INSTRUMENT_NOT_EXECUTABLE".to_string());
            } else if lifecycle_status != "reported_in_force" {
                readiness_reasons.push("CURRENT_LEGAL_STATUS_UNVERIFIED".to_string());
            }
            if !date_window_valid {
                readiness_reasons.push("EFFECTIVE_WINDOW_INVALID".to_string());
            }
            if !jurisdiction_confirmed {
                readiness_reasons.push("JURISDICTION_NOT_RESOLVED_TO_MUNICIPALITY".to_string());
            }
            if applicability_status != "confirmed_for_project" {
                readiness_reasons.push("PROJECT_APPLICABILITY_NOT_CONFIRMED".to_string());
            }
            if !artifact_roles_distinct {
                readiness_reasons.push("EVIDENCE_ARTIFACT_REUSED_ACROSS_ROLES".to_string());
            }
            if !snapshot_verified {
                readiness_reasons.push("SOURCE_SNAPSHOT_MISSING_OR_UNVERIFIED".to_string());
            }
            if !review_verified
                || record["review_record"]["decision"].as_str()
                    != Some("applicable_for_evaluation")
            {
                readiness_reasons
                    .push("QUALIFIED_APPLICABILITY_REVIEW_NOT_EVIDENCED".to_string());
            }
            let access = record["source_snapshot"]["access_status"]
                .as_str()
                .unwrap_or("not_assessed");
            if !matches!(access, "public" | "licensed") {
                readiness_reasons.push("SOURCE_ACCESS_RIGHTS_NOT_CLEARED".to_string());
            }
            let translation_status =
                record["rule_translation"]["status"].as_str().unwrap_or("not_translated");
            let translation_review = &record["rule_translation"]["translation_review"];
            let author_reference = record["rule_translation"]["author_reference"].as_str();
            let reviewer_reference = translation_review["reviewer_reference"].as_str();
            let translation_review_approved = translation_review["decision"].as_str()
                == Some("approved_for_evaluation");
            let reviewer_independent = author_reference.is_some()
                && reviewer_reference.is_some()
                && author_reference != reviewer_reference;
            if translation_status != "independently_reviewed" {
                readiness_reasons.push("RULE_TRANSLATION_NOT_INDEPENDENTLY_REVIEWED".to_string());
            }
            if translation_status == "independently_reviewed"
                && (!translation_review_verified || !translation_review_approved)
            {
                readiness_reasons.push("RULE_TRANSLATION_REVIEW_NOT_EVIDENCED".to_string());
            }
            if translation_status == "independently_reviewed" && !reviewer_independent {
                readiness_reasons.push("RULE_TRANSLATION_REVIEWER_NOT_INDEPENDENT".to_string());
                semantic_issues.push(format!(
                    "source record {record_id:?} translation reviewer must differ from the translation author"
                ));
            }
            if record["rule_translation"].is_object() {
                if record["edition"].as_str().is_none_or(str::is_empty) {
                    readiness_reasons.push("SOURCE_EDITION_MISSING".to_string());
                }
                if record["clause_location"].as_str().is_none_or(str::is_empty) {
                    readiness_reasons.push("SOURCE_CLAUSE_LOCATION_MISSING".to_string());
                }
            }
            if !implementation_verified {
                readiness_reasons.push("RULE_IMPLEMENTATION_DIGEST_NOT_VERIFIED".to_string());
            }
            if !corpus_verified {
                readiness_reasons.push("RULE_TEST_CORPUS_DIGEST_NOT_VERIFIED".to_string());
            }
            if review_postdates_evaluation {
                semantic_issues.push(format!(
                    "source record {record_id:?} has a review after registry evaluation_date"
                ));
                readiness_reasons.push("REVIEW_POSTDATES_EVALUATION_DATE".to_string());
            }
            if translation_review_postdates_evaluation {
                semantic_issues.push(format!(
                    "source record {record_id:?} has a translation review after registry evaluation_date"
                ));
                readiness_reasons.push("TRANSLATION_REVIEW_POSTDATES_EVALUATION_DATE".to_string());
            }

            records.push(RecordReadiness {
                record_id: record_id.to_owned(),
                lifecycle_status: lifecycle_status.to_owned(),
                applicability_status: applicability_status.to_owned(),
                source_snapshot_digest_verified: snapshot_verified,
                review_evidence_digest_verified: review_verified,
                translation_review_evidence_digest_verified: translation_review_verified,
                rule_implementation_digest_verified: implementation_verified,
                rule_test_corpus_digest_verified: corpus_verified,
                ready_for_rule_evaluation: readiness_reasons.is_empty(),
                readiness_reasons,
            });
        }
    }

    let semantic_valid = semantic_issues.is_empty();
    if !semantic_valid {
        for record in &mut records {
            record.ready_for_rule_evaluation = false;
            if !record
                .readiness_reasons
                .contains(&"REGISTRY_SEMANTIC_VALIDATION_FAILED".to_string())
            {
                record
                    .readiness_reasons
                    .push("REGISTRY_SEMANTIC_VALIDATION_FAILED".to_string());
            }
        }
    }
    let registry_contract_status = if schema_valid && semantic_valid {
        "REGISTRY_VALID"
    } else {
        "REGISTRY_INVALID"
    };
    let ready_for_rule_evaluation_count = records
        .iter()
        .filter(|record| record.ready_for_rule_evaluation)
        .count();
    let mut hasher = Sha256::new();
    hasher.update(EMBEDDED_SCHEMA.as_bytes());
    let schema_sha256 = format!("{:x}", hasher.finalize());

    Ok(RegistryValidationReport {
        validator_version: env!("CARGO_PKG_VERSION").to_string(),
        schema_sha256,
        registry_id: registry
            .get("registry_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        schema_valid,
        semantic_valid,
        registry_contract_status: registry_contract_status.to_string(),
        source_snapshot_digests_verified,
        review_evidence_digests_verified,
        translation_review_evidence_digests_verified,
        rule_artifact_digests_verified,
        ready_for_rule_evaluation_count,
        schema_issues,
        semantic_issues,
        records,
        limitations: vec![
            "REGISTRY_VALID means only schema and implemented cross-field checks passed; it is not a legal opinion or code-compliance result.".to_string(),
            "A record is ready only when local source, review, implementation, and test artifacts hash correctly and all declared lifecycle, jurisdiction, applicability, review and rights gates are satisfied.".to_string(),
            "Reviewer qualifications, signature authenticity, source authority, legal interpretation, and project-specific applicability are not independently authenticated by this verifier.".to_string(),
            "A ready record merely satisfies declared prerequisites for a separate deterministic rule evaluator; this command executes no building-code rule.".to_string(),
            "Proposals, unknown lifecycle states, missing snapshots, unresolved municipalities, and unreviewed translations remain ineligible for rule execution.".to_string(),
        ],
    })
}

pub(crate) fn verify_artifact(
    artifact: Option<&Value>,
    artifact_root: &Path,
    issues: &mut Vec<String>,
    context: &str,
) -> bool {
    let Some(artifact) = artifact else {
        return false;
    };
    let Some(relative_path) = artifact["relative_path"].as_str() else {
        return false;
    };
    let Some(expected_digest) = artifact["sha256"].as_str() else {
        return false;
    };
    let relative = Path::new(relative_path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        issues.push(format!(
            "{context} uses an unsafe non-relative artifact path"
        ));
        return false;
    }
    let base = match artifact_root.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            issues.push(format!("{context}: cannot resolve artifact root: {error}"));
            return false;
        }
    };
    let path = match base.join(relative).canonicalize() {
        Ok(path) => path,
        Err(error) => {
            issues.push(format!(
                "{context}: cannot read artifact path {relative_path:?}: {error}"
            ));
            return false;
        }
    };
    if !path.starts_with(&base) {
        issues.push(format!("{context} resolves outside the artifact root"));
        return false;
    }
    match fs::read(&path) {
        Ok(bytes) => {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let actual = format!("{:x}", hasher.finalize());
            if actual == expected_digest {
                true
            } else {
                issues.push(format!(
                    "{context} SHA-256 mismatch: expected {expected_digest}, actual {actual}"
                ));
                false
            }
        }
        Err(error) => {
            issues.push(format!("{context}: cannot read local artifact: {error}"));
            false
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEST_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            loop {
                let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
                let path = std::env::temp_dir().join(format!(
                    "symtropy-jurisdiction-registry-{}-{id}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("cannot create test directory: {error}"),
                }
            }
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_hashed_artifact(root: &Path, relative_path: &str, bytes: &[u8]) -> Value {
        let destination = root.join(relative_path);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("artifact parent directory exists");
        }
        fs::write(&destination, bytes).expect("test artifact writes");
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        json!({
            "relative_path": relative_path,
            "sha256": format!("{:x}", hasher.finalize())
        })
    }

    fn ready_candidate_registry(root: &Path) -> Value {
        let mut registry = fixture();
        registry["jurisdiction"]["municipality"] = json!("Johannesburg");
        registry["jurisdiction"]["resolution_status"] = json!("confirmed");
        let record = &mut registry["source_records"][0];
        record["lifecycle_status"] = json!("reported_in_force");
        record["effective_from"] = json!("2020-01-01");
        record["effective_to"] = Value::Null;
        record["edition"] = json!("controlled-test-edition-1");
        record["clause_location"] = json!("controlled-test-section-4");
        record["applicability_status"] = json!("confirmed_for_project");
        record["source_snapshot"] = json!({
            "artifact": write_hashed_artifact(root, "source.txt", b"controlled source snapshot\n"),
            "captured_at": "2026-10-14T12:00:00Z",
            "media_type": "text/plain",
            "access_status": "public"
        });
        record["review_record"] = json!({
            "review_id": "controlled-test-review-1",
            "reviewer_role": "qualified_professional",
            "reviewed_at": "2026-10-14T12:30:00Z",
            "decision": "applicable_for_evaluation",
            "evidence_artifact": write_hashed_artifact(root, "review.txt", b"controlled review evidence\n")
        });
        record["rule_translation"] = json!({
            "rule_id": "controlled-test-rule-1",
            "version": "1.0.0",
            "status": "independently_reviewed",
            "author_reference": "translator:controlled-author",
            "validity_domain": "test only; no legal meaning",
            "acceptance_criteria": "controlled known-answer fixture",
            "implementation_artifact": write_hashed_artifact(root, "rule.rs", b"fn controlled_rule() -> bool { true }\n"),
            "test_corpus_artifact": write_hashed_artifact(root, "corpus.json", b"{\"cases\":[{\"expected\":true}]}\n"),
            "translation_review": {
                "review_id": "controlled-translation-review-1",
                "reviewer_reference": "reviewer:independent-reviewer",
                "reviewer_role": "standards_specialist",
                "reviewed_at": "2026-10-14T13:00:00Z",
                "decision": "approved_for_evaluation",
                "evidence_artifact": write_hashed_artifact(root, "translation-review.txt", b"independent translation review evidence\n")
            }
        });
        registry
    }



    fn fixture() -> Value {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../docs/built-environment/fixtures/example-jurisdiction-evidence-registry.json",
        );
        serde_json::from_slice(&fs::read(path).expect("jurisdiction registry fixture exists"))
            .expect("jurisdiction registry fixture parses")
    }

    #[test]
    fn verified_prerequisites_can_make_a_controlled_record_evaluation_ready() {
        let root = TestRoot::new();
        let registry = ready_candidate_registry(root.path());
        let report = validate_value(&registry, root.path()).expect("schema compiles");

        assert!(report.schema_valid, "{:?}", report.schema_issues);
        assert!(report.semantic_valid, "{:?}", report.semantic_issues);
        assert_eq!(report.registry_contract_status, "REGISTRY_VALID");
        assert_eq!(report.ready_for_rule_evaluation_count, 1);
        let record = report.records.iter()
            .find(|record| record.record_id == "za-nbrbsa-act-103-1977-reference")
            .expect("controlled source record appears");
        assert!(record.ready_for_rule_evaluation, "{:?}", record.readiness_reasons);
        assert!(record.source_snapshot_digest_verified);
        assert!(record.review_evidence_digest_verified);
        assert!(record.rule_implementation_digest_verified);
        assert!(record.rule_test_corpus_digest_verified);
    }

    #[test]
    fn source_snapshot_captured_after_evaluation_date_is_rejected() {
        let root = TestRoot::new();
        let mut registry = ready_candidate_registry(root.path());
        registry["source_records"][0]["source_snapshot"]["captured_at"] =
            json!("2026-10-16T09:00:00Z");

        let report = validate_value(&registry, root.path()).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        assert!(report.semantic_issues.iter().any(|issue| {
            issue.contains("snapshot capture after registry evaluation_date")
        }));
        let record = report.records.iter()
            .find(|record| record.record_id == "za-nbrbsa-act-103-1977-reference")
            .expect("controlled source record appears");
        assert!(record.readiness_reasons.contains(
            &"SOURCE_SNAPSHOT_POSTDATES_EVALUATION_DATE".to_string()
        ));
    }

    #[test]
    fn source_publication_after_evaluation_date_is_rejected() {
        let mut registry = fixture();
        registry["source_records"][0]["publication_date"] = json!("2026-10-16");
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/built-environment/fixtures");

        let report = validate_value(&registry, &root).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        assert!(report.semantic_issues.iter().any(|issue| {
            issue.contains("publication date after registry evaluation_date")
        }));
        let record = report.records.iter()
            .find(|record| record.record_id == "za-nbrbsa-act-103-1977-reference")
            .expect("source record appears");
        assert!(record.readiness_reasons.contains(
            &"SOURCE_PUBLICATION_POSTDATES_EVALUATION_DATE".to_string()
        ));
    }

    #[test]
    fn review_after_evaluation_date_is_rejected() {
        let root = TestRoot::new();
        let mut registry = ready_candidate_registry(root.path());
        registry["source_records"][0]["review_record"]["reviewed_at"] =
            json!("2026-10-16T09:00:00Z");

        let report = validate_value(&registry, root.path()).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        assert!(report.semantic_issues.iter().any(|issue| {
            issue.contains("review after registry evaluation_date")
        }));
        let record = report.records.iter()
            .find(|record| record.record_id == "za-nbrbsa-act-103-1977-reference")
            .expect("controlled source record appears");
        assert!(record.readiness_reasons.contains(
            &"REVIEW_POSTDATES_EVALUATION_DATE".to_string()
        ));
    }

    #[test]
    fn independently_reviewed_translation_requires_a_verifiable_review_receipt() {
        let root = TestRoot::new();
        let mut registry = ready_candidate_registry(root.path());
        registry["source_records"][0]["rule_translation"]["translation_review"] = Value::Null;

        let report = validate_value(&registry, root.path()).expect("schema compiles");
        assert!(!report.schema_valid, "reviewed status needs a review receipt");
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
    }

    #[test]
    fn translation_author_cannot_serve_as_independent_reviewer() {
        let root = TestRoot::new();
        let mut registry = ready_candidate_registry(root.path());
        registry["source_records"][0]["rule_translation"]["translation_review"]["reviewer_reference"] =
            json!("translator:controlled-author");

        let report = validate_value(&registry, root.path()).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        assert!(report.semantic_issues.iter().any(|issue| {
            issue.contains("translation reviewer must differ from the translation author")
        }));
        let record = report.records.iter()
            .find(|record| record.record_id == "za-nbrbsa-act-103-1977-reference")
            .expect("controlled source record appears");
        assert!(record.readiness_reasons.contains(
            &"RULE_TRANSLATION_REVIEWER_NOT_INDEPENDENT".to_string()
        ));
    }

    #[test]
    fn one_artifact_digest_cannot_satisfy_multiple_evidence_roles() {
        let root = TestRoot::new();
        let mut registry = ready_candidate_registry(root.path());
        let source_artifact = registry["source_records"][0]["source_snapshot"]["artifact"].clone();
        registry["source_records"][0]["review_record"]["evidence_artifact"] = source_artifact;

        let report = validate_value(&registry, root.path()).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        assert!(report.semantic_issues.iter().any(|issue| {
            issue.contains("reuses a SHA-256 digest across evidence roles")
        }));
        let record = report.records.iter()
            .find(|record| record.record_id == "za-nbrbsa-act-103-1977-reference")
            .expect("controlled source record appears");
        assert!(record.readiness_reasons.contains(
            &"EVIDENCE_ARTIFACT_REUSED_ACROSS_ROLES".to_string()
        ));
    }

    #[test]
    fn translation_review_after_evaluation_date_is_rejected() {
        let root = TestRoot::new();
        let mut registry = ready_candidate_registry(root.path());
        registry["source_records"][0]["rule_translation"]["translation_review"]["reviewed_at"] =
            json!("2026-10-16T09:00:00Z");

        let report = validate_value(&registry, root.path()).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        assert!(report.semantic_issues.iter().any(|issue| {
            issue.contains("translation review after registry evaluation_date")
        }));
        let record = report.records.iter()
            .find(|record| record.record_id == "za-nbrbsa-act-103-1977-reference")
            .expect("controlled source record appears");
        assert!(record.readiness_reasons.contains(
            &"TRANSLATION_REVIEW_POSTDATES_EVALUATION_DATE".to_string()
        ));
    }

    #[test]
    fn modified_source_snapshot_fails_digest_and_blocks_readiness() {
        let root = TestRoot::new();
        let registry = ready_candidate_registry(root.path());
        fs::write(root.path().join("source.txt"), b"modified after its digest was pinned\n")
            .expect("tampered test artifact writes");

        let report = validate_value(&registry, root.path()).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        assert!(report.semantic_issues.iter().any(|issue| issue.contains("SHA-256 mismatch")));
        assert!(report.records.iter().all(|record| !record.ready_for_rule_evaluation));
    }

    #[test]
    fn parent_directory_artifact_path_is_rejected() {
        let root = TestRoot::new();
        let mut registry = ready_candidate_registry(root.path());
        registry["source_records"][0]["source_snapshot"]["artifact"]["relative_path"] =
            json!("../outside.txt");

        let report = validate_value(&registry, root.path()).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        assert!(report.semantic_issues.iter().any(|issue| issue.contains("unsafe non-relative artifact path")));
    }

    #[test]
    fn translated_rule_needs_an_edition_and_precise_clause_location() {
        let root = TestRoot::new();
        let mut registry = ready_candidate_registry(root.path());
        registry["source_records"][0]["edition"] = Value::Null;
        registry["source_records"][0]["clause_location"] = Value::Null;

        let report = validate_value(&registry, root.path()).expect("schema compiles");
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        let record = report.records.iter()
            .find(|record| record.record_id == "za-nbrbsa-act-103-1977-reference")
            .expect("controlled source record appears");
        assert!(record.readiness_reasons.contains(&"SOURCE_EDITION_MISSING".to_string()));
        assert!(record.readiness_reasons.contains(&"SOURCE_CLAUSE_LOCATION_MISSING".to_string()));
    }

    #[test]
    fn input_cannot_inject_derived_readiness_flag() {
        let mut registry = fixture();
        registry["source_records"][0]["ready_for_rule_evaluation"] = json!(true);
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/built-environment/fixtures");
        let report = validate_value(&registry, &root).expect("schema compiles");
        assert!(!report.schema_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
    }

    #[test]
    fn metadata_catalog_is_valid_but_has_no_rule_ready_records() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/built-environment/fixtures");
        let report = validate_value(&fixture(), &root).expect("embedded registry schema compiles");
        assert!(report.schema_valid, "{:?}", report.schema_issues);
        assert!(report.semantic_valid, "{:?}", report.semantic_issues);
        assert_eq!(report.registry_contract_status, "REGISTRY_VALID");
        assert_eq!(report.records.len(), 2);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
        assert!(report.records.iter().all(|record| !record.ready_for_rule_evaluation));
        assert!(report.records.iter().any(|record| {
            record
                .readiness_reasons
                .contains(&"PROPOSED_INSTRUMENT_NOT_EXECUTABLE".to_string())
        }));
    }

    #[test]
    fn schema_is_draft_2020_12_with_local_references_only() {
        let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA).expect("schema parses");
        assert!(draft202012::meta::validate(&schema).is_ok());
        assert!(all_refs_are_local(&schema));
    }

    #[test]
    fn proposal_cannot_be_recorded_as_in_force() {
        let mut registry = fixture();
        registry["source_records"][1]["lifecycle_status"] =
            serde_json::json!("reported_in_force");
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/built-environment/fixtures");
        let report = validate_value(&registry, &root).expect("schema compiles");
        assert!(!report.schema_valid, "proposal and lifecycle state must agree");
    }

    #[test]
    fn duplicate_source_record_ids_are_rejected() {
        let mut registry = fixture();
        registry["source_records"][1]["record_id"] =
            registry["source_records"][0]["record_id"].clone();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/built-environment/fixtures");
        let report = validate_value(&registry, &root).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert_eq!(report.ready_for_rule_evaluation_count, 0);
    }

    #[test]
    fn reversed_effective_window_is_rejected() {
        let mut registry = fixture();
        registry["source_records"][0]["effective_from"] =
            serde_json::json!("2030-01-01");
        registry["source_records"][0]["effective_to"] =
            serde_json::json!("2020-01-01");
        registry["source_records"][0]["lifecycle_status"] =
            serde_json::json!("reported_in_force");
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../docs/built-environment/fixtures");
        let report = validate_value(&registry, &root).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert!(report.semantic_issues.iter().any(|issue| {
            issue.contains("effective_to before effective_from")
        }));
    }
}
