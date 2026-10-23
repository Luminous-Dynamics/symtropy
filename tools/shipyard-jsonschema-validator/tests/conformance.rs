use serde_json::Value;
use shipyard_jsonschema_validator::{
    audit_benchmark_documents, bundled_manifest_schema, bundled_report_schema, manifest_sha256,
    validate_instance_text, validate_schema_document,
};

const MANIFEST: &str = include_str!("../../shipyard-benchmark-audit/fixtures/valid-manifest.json");
const REPORT: &str = include_str!("../../shipyard-benchmark-audit/fixtures/valid-report.json");

fn with_manifest_digest(report: &str, manifest: &str) -> String {
    let marker = "\"manifest_sha256\": \"";
    let start = report
        .find(marker)
        .expect("report fixture must include manifest digest")
        + marker.len();
    let end = report[start..]
        .find('"')
        .map(|offset| start + offset)
        .expect("digest must be quoted");
    format!(
        "{}{}{}",
        &report[..start],
        manifest_sha256(manifest),
        &report[end..]
    )
}

#[test]
fn both_embedded_schemas_are_valid_against_draft_2020_12_meta_schema() {
    validate_schema_document(bundled_manifest_schema()).unwrap();
    validate_schema_document(bundled_report_schema()).unwrap();
}

#[test]
fn valid_frozen_manifest_and_report_pass_full_schema_validation() {
    validate_instance_text(bundled_manifest_schema(), MANIFEST).unwrap();
    let report = with_manifest_digest(REPORT, MANIFEST);
    validate_instance_text(bundled_report_schema(), &report).unwrap();
}

#[test]
fn generated_manifest_requires_generator_under_all_of_condition() {
    let mut invalid: Value = serde_json::from_str(MANIFEST).unwrap();
    invalid["source"]["kind"] = Value::String("generated".to_owned());
    invalid.as_object_mut().unwrap().remove("generator");
    let text = serde_json::to_string(&invalid).unwrap();
    let errors = validate_instance_text(bundled_manifest_schema(), &text)
        .expect_err("generator must be required");
    assert!(
        errors
            .iter()
            .any(|error| error.instance_path.contains("generator"))
    );
}

#[test]
fn unknown_manifest_field_is_rejected() {
    let mut invalid: Value = serde_json::from_str(MANIFEST).unwrap();
    invalid["unexpected"] = Value::Bool(true);
    let text = serde_json::to_string(&invalid).unwrap();
    let errors = validate_instance_text(bundled_manifest_schema(), &text)
        .expect_err("unknown field must fail");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("unexpected"))
    );
}

#[test]
fn malformed_digest_is_rejected_by_pattern_keyword() {
    let mut invalid: Value = serde_json::from_str(MANIFEST).unwrap();
    invalid["candidate"]["config_sha256"] = Value::String("bad".to_owned());
    let text = serde_json::to_string(&invalid).unwrap();
    let errors =
        validate_instance_text(bundled_manifest_schema(), &text).expect_err("bad digest must fail");
    assert!(
        errors
            .iter()
            .any(|error| error.instance_path.contains("config_sha256"))
    );
}

#[test]
fn malformed_date_time_is_rejected_in_report() {
    let mut report: Value = serde_json::from_str(REPORT).unwrap();
    report["run_metadata"]["started_at_utc"] = Value::String("not a date-time".to_owned());
    let report_text = with_manifest_digest(&serde_json::to_string(&report).unwrap(), MANIFEST);
    let errors = validate_instance_text(bundled_report_schema(), &report_text)
        .expect_err("format must be checked");
    assert!(
        errors
            .iter()
            .any(|error| error.instance_path.contains("started_at_utc"))
    );
}

#[test]
fn semantic_auditor_still_rejects_duplicate_object_keys() {
    let duplicate_manifest = MANIFEST.replace(
        "\"schema_version\": \"shipyard-benchmark-manifest/v0\",",
        "\"schema_version\": \"shipyard-benchmark-manifest/v0\", \"schema_version\": \"shipyard-benchmark-manifest/v0\",",
    );
    let report = with_manifest_digest(REPORT, &duplicate_manifest);
    let errors = audit_benchmark_documents(&duplicate_manifest, &report)
        .expect_err("duplicate keys must fail");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("duplicate object key"))
    );
}

#[test]
fn schema_valid_report_is_still_checked_by_semantic_layer() {
    let mut report: Value = serde_json::from_str(REPORT).unwrap();
    report["summary"]["false_feasible_count"] = Value::from(99);
    let report_text = with_manifest_digest(&serde_json::to_string(&report).unwrap(), MANIFEST);
    let errors = audit_benchmark_documents(MANIFEST, &report_text)
        .expect_err("summary drift must fail semantic audit");
    assert!(
        errors.iter().any(|error| error.source == "semantic-audit"
            && error.message.contains("false_feasible_count"))
    );
}
