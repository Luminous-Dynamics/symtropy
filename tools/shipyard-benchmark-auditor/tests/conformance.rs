use serde_json::Value;
use shipyard_benchmark_auditor::{audit_bytes, sha256_hex, validate_embedded_schemas};

fn fixture(name: &str) -> &'static [u8] {
    match name {
        "valid-manifest.json" => include_bytes!("fixtures/valid-manifest.json"),
        "valid-report.json" => include_bytes!("fixtures/valid-report.json"),
        "invalid-manifest-missing-generator.json" => include_bytes!("fixtures/invalid-manifest-missing-generator.json"),
        "invalid-manifest-bad-digest.json" => include_bytes!("fixtures/invalid-manifest-bad-digest.json"),
        "invalid-report-summary-mismatch.json" => include_bytes!("fixtures/invalid-report-summary-mismatch.json"),
        "invalid-report-oracle-lie.json" => include_bytes!("fixtures/invalid-report-oracle-lie.json"),
        "invalid-report-timeout-as-infeasible.json" => include_bytes!("fixtures/invalid-report-timeout-as-infeasible.json"),
        "invalid-report-duplicate-case.json" => include_bytes!("fixtures/invalid-report-duplicate-case.json"),
        "incomplete-report-with-unknown.json" => include_bytes!("fixtures/incomplete-report-with-unknown.json"),
        "invalid-manifest-source-digest.json" => include_bytes!("fixtures/invalid-manifest-source-digest.json"),
        "invalid-report-wrong-revision.json" => include_bytes!("fixtures/invalid-report-wrong-revision.json"),
        "invalid-report-missing-case.json" => include_bytes!("fixtures/invalid-report-missing-case.json"),
        _ => panic!("unknown fixture {name}"),
    }
}

fn report_for_manifest(manifest: &[u8], report_name: &str) -> Vec<u8> {
    let mut report: Value = serde_json::from_slice(fixture(report_name)).unwrap();
    report["manifest_sha256"] = Value::String(sha256_hex(manifest));
    serde_json::to_vec(&report).unwrap()
}

fn assert_invalid(manifest_name: &str, report_name: &str, expected: &str) {
    let manifest = fixture(manifest_name);
    let report = report_for_manifest(manifest, report_name);
    let errors = audit_bytes(manifest, &report).expect_err("negative fixture must fail");
    assert!(errors.iter().any(|error| error.contains(expected)),
        "expected diagnostic containing {expected:?}, got {errors:#?}");
}

#[test]
fn valid_exact_oracle_fixture_is_accepted() {
    let manifest = fixture("valid-manifest.json");
    let report = report_for_manifest(manifest, "valid-report.json");
    let summary = audit_bytes(manifest, &report).expect("valid fixture should pass");
    assert_eq!(summary.case_count, 2);
    assert_eq!(summary.verified_feasible_count, 1);
    assert_eq!(summary.verified_infeasible_count, 1);
    assert_eq!(summary.false_feasible_count, 0);
    assert_eq!(summary.false_infeasible_count, 0);
}

#[test]
fn embedded_schemas_are_valid_json_and_have_contract_required_fields() {
    validate_embedded_schemas().expect("schemas should be structurally consistent");
}

#[test]
fn generated_corpus_requires_generator_recipe() {
    assert_invalid("invalid-manifest-missing-generator.json", "valid-report.json",
        "manifest.generator: required when source.kind is generated");
}

#[test]
fn malformed_configuration_digest_is_rejected() {
    assert_invalid("invalid-manifest-bad-digest.json", "valid-report.json",
        "manifest.candidate.config_sha256: expected hexadecimal digest/revision");
}

#[test]
fn inconsistent_summary_counts_are_rejected() {
    assert_invalid("valid-manifest.json", "invalid-report-summary-mismatch.json",
        "report.summary.verified_feasible_count: inconsistent");
}

#[test]
fn oracle_mismatch_cannot_be_reported_as_a_success() {
    assert_invalid("valid-manifest.json", "invalid-report-oracle-lie.json",
        "report.case_results[0].verifier.oracle_outcome: inconsistent");
}

#[test]
fn timeout_cannot_be_relabelled_as_infeasibility_or_a_match() {
    assert_invalid("valid-manifest.json", "invalid-report-timeout-as-infeasible.json",
        "report.case_results[0].verifier.oracle_outcome: inconsistent");
}

#[test]
fn duplicate_case_identifiers_are_rejected() {
    assert_invalid("valid-manifest.json", "invalid-report-duplicate-case.json",
        "duplicate case identifier");
}

#[test]
fn incomplete_report_preserves_unknowns_without_claiming_completion() {
    let manifest = fixture("valid-manifest.json");
    let report = report_for_manifest(manifest, "incomplete-report-with-unknown.json");
    let summary = audit_bytes(manifest, &report).expect("well-labelled incomplete report should pass");
    assert_eq!(summary.unknown_count, 1);
    assert_eq!(summary.case_count, 2);
}

#[test]
fn malformed_source_digest_is_rejected() {
    assert_invalid("invalid-manifest-source-digest.json", "valid-report.json",
        "manifest.source.sha256: expected hexadecimal digest/revision");
}

#[test]
fn unknown_report_commit_is_rejected() {
    assert_invalid("valid-manifest.json", "invalid-report-wrong-revision.json",
        "report.run_metadata.git_commit_sha: does not match manifest candidate revision");
}

#[test]
fn complete_report_must_include_every_declared_case() {
    assert_invalid("valid-manifest.json", "invalid-report-missing-case.json",
        "report.status: complete requires exactly protocol.expected_case_count case records");
}

#[test]
fn manifest_bytes_are_pinned_exactly() {
    let manifest = fixture("valid-manifest.json");
    let mut report: Value = serde_json::from_slice(fixture("valid-report.json")).unwrap();
    report["manifest_sha256"] = Value::String("f".repeat(64));
    let errors = audit_bytes(manifest, &serde_json::to_vec(&report).unwrap()).unwrap_err();
    assert!(errors.iter().any(|error| error.contains("manifest.sha256")));
}
