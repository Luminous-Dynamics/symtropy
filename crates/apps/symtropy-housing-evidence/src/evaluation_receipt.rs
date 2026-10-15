// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Verifies producer-declared rule-evaluation receipt bindings.
//! It verifies provenance and artifact identity; it never executes a building rule.

use crate::jurisdiction_registry;
use jsonschema::draft202012;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::Path,
};

const EMBEDDED_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/rule-evaluation-receipt.schema.json");
const VERIFIED: &str = "RECEIPT_INTEGRITY_VERIFIED";
const NOT_VERIFIED: &str = "NOT_VERIFIED";

#[derive(Debug, Serialize)]
pub struct ReceiptValidationReport {
    pub validator_version: String,
    pub schema_sha256: String,
    pub receipt_sha256: Option<String>,
    pub registry_sha256_actual: Option<String>,
    pub schema_valid: bool,
    pub registry_contract_status: String,
    pub registry_digest_matches: bool,
    pub source_binding_matches: bool,
    pub eligible_to_evaluate: bool,
    pub producer_declared_status: Option<String>,
    pub receipt_integrity_status: String,
    pub effective_status: String,
    pub artifact_digests_verified: usize,
    pub issues: Vec<String>,
}

pub fn validate_paths(
    receipt_path: &Path,
    registry_path: &Path,
) -> Result<ReceiptValidationReport, String> {
    let receipt_bytes = fs::read(receipt_path)
        .map_err(|error| format!("cannot read {}: {error}", receipt_path.display()))?;
    let receipt: Value = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", receipt_path.display()))?;

    let registry_bytes = fs::read(registry_path)
        .map_err(|error| format!("cannot read {}: {error}", registry_path.display()))?;
    let registry: Value = serde_json::from_slice(&registry_bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", registry_path.display()))?;

    let artifact_root = registry_path.parent().unwrap_or_else(|| Path::new("."));
    validate_values(
        &receipt,
        &registry,
        &receipt_bytes,
        &registry_bytes,
        artifact_root,
    )
}

fn validate_values(
    receipt: &Value,
    registry: &Value,
    receipt_bytes: &[u8],
    registry_bytes: &[u8],
    artifact_root: &Path,
) -> Result<ReceiptValidationReport, String> {
    let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA)
        .map_err(|error| format!("embedded receipt schema is invalid JSON: {error}"))?;
    if !all_refs_are_local(&schema) {
        return Err("rule evaluation receipt schema contains an external reference".to_string());
    }
    draft202012::meta::validate(&schema)
        .map_err(|error| format!("receipt schema fails Draft 2020-12 meta-validation: {error}"))?;
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("cannot compile receipt schema: {error}"))?;
    let schema_issues: Vec<String> = validator
        .iter_errors(receipt)
        .map(|error| format!("schema validation: {error}"))
        .collect();
    let schema_valid = schema_issues.is_empty();

    let registry_report = jurisdiction_registry::validate_value(registry, artifact_root)?;
    let actual_registry_sha256 = sha256_hex(registry_bytes);
    let mut issues = schema_issues;
    let mut artifact_digests_verified = 0usize;
    let producer_declared_status = receipt
        .get("producer_declared_status")
        .and_then(Value::as_str)
        .map(str::to_owned);

    let registry_digest_matches = receipt["registry"]["sha256"]
        .as_str()
        .is_some_and(|value| value == actual_registry_sha256);
    if !registry_digest_matches {
        issues.push(format!(
            "receipt registry digest mismatch (actual {actual_registry_sha256})"
        ));
    }
    if registry_report.registry_contract_status != "REGISTRY_VALID" {
        issues.push(format!(
            "referenced registry is not contract-valid: {}",
            registry_report.registry_contract_status
        ));
    }

    let mut source_binding_matches = schema_valid
        && registry_digest_matches
        && registry_report.registry_contract_status == "REGISTRY_VALID";

    compare_string(
        receipt["registry"]["registry_id"].as_str(),
        registry["registry_id"].as_str(),
        "registry_id",
        &mut source_binding_matches,
        &mut issues,
    );
    if receipt["registry"]["revision"].as_u64() != registry["revision"].as_u64() {
        source_binding_matches = false;
        issues.push("receipt registry revision does not match supplied registry".to_string());
    }
    if receipt["evaluation_date"].as_str() != registry["evaluation_date"].as_str() {
        source_binding_matches = false;
        issues.push("receipt evaluation_date does not match supplied registry".to_string());
    }
    for key in ["country_code", "region", "municipality", "resolution_status"] {
        if receipt["jurisdiction"].get(key) != registry["jurisdiction"].get(key) {
            source_binding_matches = false;
            issues.push(format!(
                "receipt jurisdiction field {key:?} does not match supplied registry"
            ));
        }
    }

    let source_id = receipt["source_binding"]["record_id"]
        .as_str()
        .unwrap_or_default();
    let record = registry["source_records"]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["record_id"].as_str() == Some(source_id))
        });
    let Some(record) = record else {
        source_binding_matches = false;
        issues.push(format!(
            "receipt source record {source_id:?} does not exist in the registry"
        ));
        return Ok(build_report(
            receipt,
            receipt_bytes,
            registry_bytes,
            schema_valid,
            &registry_report,
            registry_digest_matches,
            source_binding_matches,
            false,
            artifact_digests_verified,
            issues,
        ));
    };

    compare_string(
        receipt["source_binding"]["edition"].as_str(),
        record["edition"].as_str(),
        "source edition",
        &mut source_binding_matches,
        &mut issues,
    );
    compare_string(
        receipt["source_binding"]["clause_location"].as_str(),
        record["clause_location"].as_str(),
        "source clause_location",
        &mut source_binding_matches,
        &mut issues,
    );
    compare_string(
        receipt["source_binding"]["source_snapshot_sha256"].as_str(),
        record["source_snapshot"]["artifact"]["sha256"].as_str(),
        "source snapshot digest",
        &mut source_binding_matches,
        &mut issues,
    );
    compare_string(
        receipt["source_binding"]["rule_id"].as_str(),
        record["rule_translation"]["rule_id"].as_str(),
        "rule_id",
        &mut source_binding_matches,
        &mut issues,
    );
    compare_string(
        receipt["source_binding"]["rule_version"].as_str(),
        record["rule_translation"]["version"].as_str(),
        "rule version",
        &mut source_binding_matches,
        &mut issues,
    );
    compare_string(
        receipt["source_binding"]["implementation_sha256"].as_str(),
        record["rule_translation"]["implementation_artifact"]["sha256"].as_str(),
        "rule implementation digest",
        &mut source_binding_matches,
        &mut issues,
    );
    compare_string(
        receipt["source_binding"]["test_corpus_sha256"].as_str(),
        record["rule_translation"]["test_corpus_artifact"]["sha256"].as_str(),
        "rule test corpus digest",
        &mut source_binding_matches,
        &mut issues,
    );
    compare_string(
        receipt["source_binding"]["translation_review_sha256"].as_str(),
        record["rule_translation"]["translation_review"]["evidence_artifact"]["sha256"].as_str(),
        "translation review digest",
        &mut source_binding_matches,
        &mut issues,
    );

    let mut artifact_issues = Vec::new();
    let project_verified = jurisdiction_registry::verify_artifact(
        receipt.get("project_input_manifest"),
        artifact_root,
        &mut artifact_issues,
        "receipt project input manifest",
    );
    let evaluator_verified = jurisdiction_registry::verify_artifact(
        receipt
            .get("evaluator")
            .and_then(|value| value.get("executable_artifact")),
        artifact_root,
        &mut artifact_issues,
        "receipt evaluator executable",
    );
    let output_verified = jurisdiction_registry::verify_artifact(
        receipt.get("output_artifact"),
        artifact_root,
        &mut artifact_issues,
        "receipt output artifact",
    );
    artifact_digests_verified += [
        project_verified,
        evaluator_verified,
        output_verified,
    ]
    .into_iter()
    .filter(|verified| *verified)
    .count();
    issues.extend(artifact_issues);

    let digest_roles = [
        ("registry", Some(actual_registry_sha256.as_str())),
        (
            "source snapshot",
            record["source_snapshot"]["artifact"]["sha256"].as_str(),
        ),
        (
            "applicability review",
            record["review_record"]["evidence_artifact"]["sha256"].as_str(),
        ),
        (
            "translation review",
            record["rule_translation"]["translation_review"]["evidence_artifact"]["sha256"]
                .as_str(),
        ),
        (
            "rule implementation",
            record["rule_translation"]["implementation_artifact"]["sha256"].as_str(),
        ),
        (
            "test corpus",
            record["rule_translation"]["test_corpus_artifact"]["sha256"].as_str(),
        ),
        (
            "project input manifest",
            receipt["project_input_manifest"]["sha256"].as_str(),
        ),
        (
            "evaluator executable",
            receipt["evaluator"]["executable_artifact"]["sha256"].as_str(),
        ),
        (
            "output artifact",
            receipt["output_artifact"]["sha256"].as_str(),
        ),
    ];
    let mut used_digests = BTreeSet::new();
    for (role, digest) in digest_roles {
        if let Some(digest) = digest
            && !used_digests.insert(digest)
        {
            source_binding_matches = false;
            issues.push(format!(
                "receipt reuses SHA-256 digest across distinct artifact roles (duplicate at {role})"
            ));
        }
    }

    let readiness_record = registry_report
        .records
        .iter()
        .find(|item| item.record_id == source_id);
    let record_ready = readiness_record.is_some_and(|item| item.ready_for_rule_evaluation);
    if let Some(item) = readiness_record {
        artifact_digests_verified += [
            item.source_snapshot_digest_verified,
            item.review_evidence_digest_verified,
            item.translation_review_evidence_digest_verified,
            item.rule_implementation_digest_verified,
            item.rule_test_corpus_digest_verified,
        ]
        .into_iter()
        .filter(|verified| *verified)
        .count();
    }

    let eligible_to_evaluate = source_binding_matches
        && record_ready
        && project_verified
        && evaluator_verified
        && output_verified;
    let receipt_integrity_verified = schema_valid
        && registry_digest_matches
        && source_binding_matches
        && project_verified
        && evaluator_verified
        && output_verified
        && issues.is_empty();

    let mut report = build_report(
        receipt,
        receipt_bytes,
        registry_bytes,
        schema_valid,
        &registry_report,
        registry_digest_matches,
        source_binding_matches,
        eligible_to_evaluate,
        artifact_digests_verified,
        issues,
    );
    report.receipt_integrity_status = if receipt_integrity_verified {
        VERIFIED.to_string()
    } else {
        NOT_VERIFIED.to_string()
    };
    report.effective_status = if receipt_integrity_verified {
        "NOT_REPRODUCED".to_string()
    } else {
        NOT_VERIFIED.to_string()
    };
    Ok(report)
}

fn build_report(
    receipt: &Value,
    receipt_bytes: &[u8],
    registry_bytes: &[u8],
    schema_valid: bool,
    registry_report: &jurisdiction_registry::RegistryValidationReport,
    registry_digest_matches: bool,
    source_binding_matches: bool,
    eligible_to_evaluate: bool,
    artifact_digests_verified: usize,
    issues: Vec<String>,
) -> ReceiptValidationReport {
    ReceiptValidationReport {
        validator_version: env!("CARGO_PKG_VERSION").to_string(),
        schema_sha256: sha256_hex(EMBEDDED_SCHEMA.as_bytes()),
        receipt_sha256: Some(sha256_hex(receipt_bytes)),
        registry_sha256_actual: Some(sha256_hex(registry_bytes)),
        schema_valid,
        registry_contract_status: registry_report.registry_contract_status.clone(),
        registry_digest_matches,
        source_binding_matches,
        eligible_to_evaluate,
        producer_declared_status: receipt
            .get("producer_declared_status")
            .and_then(Value::as_str)
            .map(str::to_owned),
        receipt_integrity_status: NOT_VERIFIED.to_string(),
        // This command checks provenance and binding only; it does not run the rule evaluator.
        effective_status: NOT_VERIFIED.to_string(),
        artifact_digests_verified,
        issues,
    }
}

fn compare_string(
    supplied: Option<&str>,
    expected: Option<&str>,
    field: &str,
    matched: &mut bool,
    issues: &mut Vec<String>,
) {
    if supplied.is_none() || supplied != expected {
        *matched = false;
        issues.push(format!(
            "receipt {field} does not match referenced registry record"
        ));
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
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
                    "symtropy-rule-evaluation-receipt-{}-{id}",
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

    fn write_artifact(root: &Path, name: &str, bytes: &[u8]) -> Value {
        fs::write(root.join(name), bytes).expect("test artifact writes");
        json!({"relative_path": name, "sha256": sha256_hex(bytes)})
    }

    fn positive_case(root: &Path) -> (Value, Value, PathBuf) {
        let fixture_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../../docs/built-environment/fixtures/example-jurisdiction-evidence-registry.json",
        );
        let mut registry = serde_json::from_slice::<Value>(
            &fs::read(fixture_path).expect("registry fixture exists"),
        )
        .expect("registry fixture parses");
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
            "artifact": write_artifact(root, "source.txt", b"controlled source snapshot\n"),
            "captured_at": "2026-10-14T12:00:00Z",
            "media_type": "text/plain",
            "access_status": "public"
        });
        record["review_record"] = json!({
            "review_id": "controlled-test-review-1",
            "reviewer_role": "qualified_professional",
            "reviewed_at": "2026-10-14T12:30:00Z",
            "decision": "applicable_for_evaluation",
            "evidence_artifact": write_artifact(
                root,
                "applicability-review.txt",
                b"applicability review evidence\n"
            )
        });
        record["rule_translation"] = json!({
            "rule_id": "controlled-test-rule-1",
            "version": "1.0.0",
            "status": "independently_reviewed",
            "author_reference": "translator:controlled-author",
            "validity_domain": "test only; no legal meaning",
            "acceptance_criteria": "controlled known-answer fixture",
            "implementation_artifact": write_artifact(
                root,
                "rule.rs",
                b"fn controlled_rule() -> bool { true }\n"
            ),
            "test_corpus_artifact": write_artifact(
                root,
                "corpus.json",
                b"{\"cases\":[{\"expected\":true}]}\n"
            ),
            "translation_review": {
                "review_id": "controlled-translation-review-1",
                "reviewer_reference": "reviewer:independent-reviewer",
                "reviewer_role": "standards_specialist",
                "reviewed_at": "2026-10-14T13:00:00Z",
                "decision": "approved_for_evaluation",
                "evidence_artifact": write_artifact(
                    root,
                    "translation-review.txt",
                    b"translation review evidence\n"
                )
            }
        });

        let registry_bytes = serde_json::to_vec_pretty(&registry).expect("registry serializes");
        fs::write(root.join("registry.json"), &registry_bytes).expect("registry artifact writes");
        let input = write_artifact(root, "project-inputs.json", b"{\"project_revision\":7}\n");
        let evaluator = write_artifact(root, "evaluator.bin", b"synthetic evaluator executable\n");
        let output = write_artifact(root, "claimed-output.json", b"{\"declared\":\"pass\"}\n");

        let receipt = json!({
            "schema_version": "0.1.0",
            "receipt_id": "test/receipt-1",
            "evaluated_at": "2026-10-15T14:00:00Z",
            "evaluation_date": "2026-10-15",
            "producer_declared_status": "CLAIMED_PASS",
            "registry": {
                "registry_id": registry["registry_id"],
                "revision": registry["revision"],
                "sha256": sha256_hex(&registry_bytes)
            },
            "jurisdiction": {
                "country_code": "ZA",
                "region": "Gauteng",
                "municipality": "Johannesburg",
                "resolution_status": "confirmed"
            },
            "source_binding": {
                "record_id": "za-nbrbsa-act-103-1977-reference",
                "edition": "controlled-test-edition-1",
                "clause_location": "controlled-test-section-4",
                "source_snapshot_sha256": registry["source_records"][0]
                    ["source_snapshot"]["artifact"]["sha256"],
                "rule_id": "controlled-test-rule-1",
                "rule_version": "1.0.0",
                "implementation_sha256": registry["source_records"][0]
                    ["rule_translation"]["implementation_artifact"]["sha256"],
                "test_corpus_sha256": registry["source_records"][0]
                    ["rule_translation"]["test_corpus_artifact"]["sha256"],
                "translation_review_sha256": registry["source_records"][0]
                    ["rule_translation"]["translation_review"]["evidence_artifact"]["sha256"]
            },
            "project_input_manifest": input,
            "evaluator": {
                "id": "controlled-test-evaluator",
                "version": "0.0.0-test",
                "executable_artifact": evaluator
            },
            "output_artifact": output
        });

        (registry, receipt, root.join("registry.json"))
    }

    #[test]
    fn receipt_binds_exact_evidence_but_does_not_reproduce_result() {
        let root = TestRoot::new();
        let (_registry, receipt, registry_path) = positive_case(root.path());
        let receipt_bytes = serde_json::to_vec_pretty(&receipt).expect("receipt serializes");
        let receipt_path = root.path().join("receipt.json");
        fs::write(&receipt_path, &receipt_bytes).expect("receipt writes");

        let report =
            validate_paths(&receipt_path, &registry_path).expect("receipt schema compiles");
        assert!(report.schema_valid, "{:?}", report.issues);
        assert!(report.registry_digest_matches, "{:?}", report.issues);
        assert!(report.source_binding_matches, "{:?}", report.issues);
        assert!(report.eligible_to_evaluate, "{:?}", report.issues);
        assert_eq!(report.receipt_integrity_status, VERIFIED, "{:?}", report.issues);
        assert_eq!(report.artifact_digests_verified, 8);
        assert_eq!(report.producer_declared_status.as_deref(), Some("CLAIMED_PASS"));
        assert_eq!(report.effective_status, "NOT_REPRODUCED");
    }

    #[test]
    fn changed_registry_bytes_invalidate_receipt_binding() {
        let root = TestRoot::new();
        let (_registry, receipt, registry_path) = positive_case(root.path());
        let receipt_path = root.path().join("receipt.json");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
        fs::write(&registry_path, b"{\"tampered\":true}\n").expect("registry tamper writes");

        let report = validate_paths(&receipt_path, &registry_path).expect("both files readable");
        assert!(!report.registry_digest_matches);
        assert_ne!(report.receipt_integrity_status, VERIFIED);
        assert!(!report.eligible_to_evaluate);
    }

    #[test]
    fn changing_the_rule_binding_cannot_be_reused_for_another_version() {
        let root = TestRoot::new();
        let (_registry, mut receipt, registry_path) = positive_case(root.path());
        receipt["source_binding"]["rule_version"] = json!("9.9.9");
        let receipt_path = root.path().join("receipt.json");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();

        let report =
            validate_paths(&receipt_path, &registry_path).expect("receipt schema compiles");
        assert!(!report.source_binding_matches);
        assert_ne!(report.receipt_integrity_status, VERIFIED);
        assert!(!report.eligible_to_evaluate);
    }

    #[test]
    fn mutated_output_artifact_invalidates_receipt_integrity() {
        let root = TestRoot::new();
        let (_registry, receipt, registry_path) = positive_case(root.path());
        let receipt_path = root.path().join("receipt.json");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap())
            .expect("receipt writes");
        fs::write(
            root.path().join("claimed-output.json"),
            b"modified after declared digest\n",
        )
        .expect("output tamper writes");

        let report =
            validate_paths(&receipt_path, &registry_path).expect("receipt schema compiles");
        assert_ne!(report.receipt_integrity_status, VERIFIED);
        assert!(!report.eligible_to_evaluate);
    }

    #[test]
    fn output_artifact_path_traversal_is_rejected() {
        let root = TestRoot::new();
        let (_registry, mut receipt, registry_path) = positive_case(root.path());
        receipt["output_artifact"]["relative_path"] = json!("../claimed-output.json");
        let receipt_path = root.path().join("receipt.json");
        fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap())
            .expect("receipt writes");

        let report =
            validate_paths(&receipt_path, &registry_path).expect("receipt schema compiles");
        assert_ne!(report.receipt_integrity_status, VERIFIED);
        assert!(!report.eligible_to_evaluate);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.contains("unsafe non-relative artifact path")));
    }
}
