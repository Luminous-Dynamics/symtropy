use shipyard_benchmark_audit::{
    audit_report_json, validate_json_syntax, validate_manifest_json,
};

const MANIFEST: &str = include_str!("../fixtures/valid-manifest.json");
const REPORT: &str = include_str!("../fixtures/valid-report.json");
const INCOMPLETE: &str = include_str!("../fixtures/incomplete-report.json");

fn has_error(errors: &[String], needle: &str) -> bool {
    errors.iter().any(|message| message.contains(needle))
}

#[test]
fn contract_schemas_are_well_formed_json() {
    assert!(validate_json_syntax(include_str!("../../shipyard-benchmark-contract/benchmark-manifest.schema.json")).is_ok());
    assert!(validate_json_syntax(include_str!("../../shipyard-benchmark-contract/run-report.schema.json")).is_ok());
}

#[test]
fn hand_authored_manifest_fixture_is_valid() {
    let errors = validate_manifest_json(MANIFEST);
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn exact_oracle_report_fixture_is_valid_and_reproducible() {
    let errors = audit_report_json(MANIFEST, REPORT);
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn report_digest_binds_the_exact_manifest_bytes() {
    let changed = MANIFEST.replace("Hand-authored tiny yard conformance fixture", "Changed title");
    assert!(!changed.is_empty());
    let errors = audit_report_json(&changed, REPORT);
    assert!(has_error(&errors, "does not match exact manifest bytes"), "{errors:#?}");
}

#[test]
fn run_revision_must_match_the_manifest_candidate() {
    let wrong_revision = REPORT.replace(
        &format!("\"git_commit_sha\": \"{}\"", "a".repeat(40)),
        &format!("\"git_commit_sha\": \"{}\"", "b".repeat(40)),
    );
    let errors = audit_report_json(MANIFEST, &wrong_revision);
    assert!(has_error(&errors, "does not match manifest candidate revision"), "{errors:#?}");
}

#[test]
fn generated_corpus_without_generator_recipe_is_rejected() {
    let invalid = include_str!("../fixtures/invalid-missing-generator.json");
    let errors = validate_manifest_json(invalid);
    assert!(has_error(&errors, "required when source.kind is generated"), "{errors:#?}");
}

#[test]
fn malformed_configuration_digest_is_rejected() {
    let invalid = include_str!("../fixtures/invalid-digest.json");
    let errors = validate_manifest_json(invalid);
    assert!(has_error(&errors, "$.candidate.config_sha256"), "{errors:#?}");
}

#[test]
fn summaries_are_recomputed_from_case_records() {
    let invalid = include_str!("../fixtures/invalid-summary-counts.json");
    let errors = audit_report_json(MANIFEST, invalid);
    assert!(has_error(&errors, "$report.summary.false_feasible_count"), "{errors:#?}");
}

#[test]
fn an_oracle_match_cannot_contradict_a_known_reference() {
    let invalid = include_str!("../fixtures/invalid-oracle-claim.json");
    let errors = audit_report_json(MANIFEST, invalid);
    assert!(has_error(&errors, "contradicts candidate and reference outcomes"), "{errors:#?}");
}

#[test]
fn exhaustion_cannot_be_promoted_to_a_proven_infeasibility() {
    let invalid = include_str!("../fixtures/invalid-infeasibility-claim.json");
    let errors = audit_report_json(MANIFEST, invalid);
    assert!(has_error(&errors, "infeasibility cannot be inferred"), "{errors:#?}");
}

#[test]
fn complete_report_must_include_every_expected_case() {
    let invalid = include_str!("../fixtures/invalid-missing-case.json");
    let errors = audit_report_json(MANIFEST, invalid);
    assert!(has_error(&errors, "manifest requires 2"), "{errors:#?}");
}

#[test]
fn duplicate_case_identifiers_are_rejected() {
    let invalid = include_str!("../fixtures/invalid-duplicate-case.json");
    let errors = audit_report_json(MANIFEST, invalid);
    assert!(has_error(&errors, "duplicate case identifier"), "{errors:#?}");
}

#[test]
fn incomplete_report_preserves_unknowns_without_claiming_completion() {
    let errors = audit_report_json(MANIFEST, INCOMPLETE);
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn oracle_measured_false_feasibility_is_counted_not_hidden() {
    let measured = include_str!("../fixtures/measured-false-feasible.json");
    let errors = audit_report_json(MANIFEST, measured);
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn oracle_measured_false_infeasibility_is_counted_not_hidden() {
    let measured = include_str!("../fixtures/measured-false-infeasible.json");
    let errors = audit_report_json(MANIFEST, measured);
    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn parser_rejects_duplicate_json_object_keys() {
    let errors = validate_manifest_json(r#"{"schema_version":"shipyard-benchmark-manifest/v0","schema_version":"shipyard-benchmark-manifest/v0"}"#);
    assert!(has_error(&errors, "duplicate object key"), "{errors:#?}");
}

#[test]
fn malformed_unicode_and_trailing_content_are_rejected() {
    assert!(validate_json_syntax(r#"{"key":"\uD800"}"#).is_err());
    assert!(validate_json_syntax(r#"{"key":true} trailing"#).is_err());
}


#[test]
fn heuristic_exhaustion_is_not_an_infeasibility_proof() {
    let report = include_str!("../fixtures/heuristic-no-solution.json");
    let errors = audit_report_json(MANIFEST, report);
    assert!(errors.is_empty(), "{errors:#?}");
    assert!(report.contains("\"no_solution_count\": 1"));
    assert!(report.contains("\"feasible_reference_cases_missed_count\": 1"));
}


#[test]
fn published_aggregate_reference_is_valid_json_and_not_reproduction_claim() {
    let data = include_str!("../../shipyard-benchmark-contract/fixtures/published-case9-table6-summary.json");
    assert!(validate_json_syntax(data).is_ok());
    assert!(data.contains("\\\"is_local_reproduction\\\": false") || data.contains("\"is_local_reproduction\": false"));
    assert!(data.contains("\"reproduction_status\": \"not-reproduced\""));
    assert_eq!(data.matches("\"case\": ").count(), 6);
}
