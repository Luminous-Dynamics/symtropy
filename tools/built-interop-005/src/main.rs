// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage-aware declared-schema validation gate for IFC STEP artifacts.
//!
//! This executable emits a gate object compatible with BUILT-INTEROP-004.
//! It does not claim complete EXPRESS, IDS, geometry, code or engineering
//! qualification.

use ifc_model::Codec;
use ifc_step::StepCodec;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

const PROFILE_ID: &str = "DECLARED-IFC-SCHEMA-SUPPORTED-RULES-v0.1";
const POLICY_ID: &str = "BUILT-INTEROP-005/v0.1;step=ifc-step-0.5.0-strict;model=ifc-model-0.3.1;schema=ifc-schema-0.3.2-ifc4-only;validator=ifc-validate-0.6.0;schema-selection=declared-only;errors=FAIL;evaluation-errors-unsupported-truncated=UNRESOLVED;findings=stable-sorted;lock-sha256=cdf5e081cacba012ed1e52246b399f1062934b0e71eeeb5f6743f4f81a279c7c";
const CLAIM_SCOPE: &str = "Checks implemented by ifc-validate 0.6.0 against the schema tables in ifc-schema 0.3.2 selected from the artifact's declared schema token.";
const LIMITATIONS: [&str; 4] = [
    "Not every normative EXPRESS WHERE or INVERSE rule is implemented; unsupported checks are reported and prevent PASS.",
    "PASS means only that this named validator profile found no errors, evaluation errors, unsupported findings or truncation; it is not proof of complete normative schema semantics.",
    "This gate does not evaluate IDS requirements, geometry preservation, building codes, structural safety or physical construction.",
    "Input identity is hashed from the bytes read by this process; the report contract does not independently authenticate a caller-supplied locator.",
];

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn config_sha256() -> String {
    sha256_hex(POLICY_ID.as_bytes())
}

fn classify_gate(
    errors: usize,
    evaluation_errors: usize,
    unsupported: usize,
    truncated: bool,
) -> &'static str {
    if errors > 0 {
        "FAIL"
    } else if evaluation_errors > 0 || unsupported > 0 || truncated {
        "UNRESOLVED"
    } else {
        "PASS"
    }
}

fn assemble_output(
    bytes: &[u8],
    locator: &str,
    status: &str,
    schema_token: Option<String>,
    decision_reason: Value,
    summary: Value,
    findings: Vec<Value>,
) -> Value {
    let artifact_sha256 = sha256_hex(bytes);
    let evidence_payload = json!({
        "profile_id": PROFILE_ID,
        "input_sha256": artifact_sha256,
        "input_byte_length": bytes.len(),
        "declared_schema_token": schema_token,
        "decision_reason": decision_reason,
        "summary": summary,
        "findings": findings,
    });
    let evidence_bytes = serde_json::to_vec(&evidence_payload)
        .expect("JSON evidence payload serialization is infallible for these fields");
    let evidence_sha256 = sha256_hex(&evidence_bytes);
    let config_sha256 = config_sha256();

    let gate = json!({
        "gate_id": "ifc_schema_conformance",
        "mandatory": true,
        "status": status,
        "claim_scope": CLAIM_SCOPE,
        "limitations": LIMITATIONS,
        "tool": {
            "name": "ifc-validate+ifc-schema",
            "version": "ifc-validate=0.6.0;ifc-schema=0.3.2;ifc-model=0.3.1;ifc-step=0.5.0",
            "config_sha256": config_sha256,
        },
        "evidence": [{
            "evidence_id": format!("ifc-schema-{artifact_sha256}"),
            "kind": "validator_report",
            "sha256": evidence_sha256,
            "locator": "report-json-pointer:/evidence_payload",
        }],
    });

    json!({
        "schema_version": "1.0.0",
        "benchmark_id": "BUILT-INTEROP-005",
        "artifact": {
            "role": "source_ifc",
            "sha256": artifact_sha256,
            "byte_length": bytes.len(),
            "media_type": "model/ifc",
            "locator": locator,
        },
        "evidence_payload": evidence_payload,
        "gate": gate,
    })
}

fn evaluate_bytes(bytes: &[u8], locator: &str) -> Value {
    let model = match StepCodec.read_bytes(bytes) {
        Ok(model) => model,
        Err(error) => {
            return assemble_output(
                bytes,
                locator,
                "UNRESOLVED",
                None,
                json!({
                    "code": "STEP_PARSE_FAILED",
                    "detail": error.to_string(),
                }),
                Value::Null,
                Vec::new(),
            );
        }
    };

    let schema_token = model.header().schema_token().map(str::to_string);
    if !model.is_complete() {
        return assemble_output(
            bytes,
            locator,
            "UNRESOLVED",
            schema_token,
            json!({
                "code": "INCOMPLETE_STEP_MODEL",
                "diagnostic_count": model.diagnostics().len(),
            }),
            Value::Null,
            Vec::new(),
        );
    }

    let report = match ifc_validate::validate_declared(&model) {
        Ok(report) => report,
        Err(error) => {
            return assemble_output(
                bytes,
                locator,
                "UNRESOLVED",
                schema_token,
                json!({
                    "code": "DECLARED_SCHEMA_UNAVAILABLE",
                    "detail": error.to_string(),
                }),
                Value::Null,
                Vec::new(),
            );
        }
    };

    let summary = report.summary();
    let truncated = report.is_truncated();
    let status = classify_gate(
        summary.errors,
        summary.evaluation_errors,
        summary.unsupported,
        truncated,
    );
    let summary_json = json!({
        "errors": summary.errors,
        "evaluation_errors": summary.evaluation_errors,
        "warnings": summary.warnings,
        "unsupported": summary.unsupported,
        "truncated": truncated,
        "validator_is_conformant": report.is_conformant(),
    });
    let findings = report
        .sorted()
        .into_iter()
        .map(|finding| {
            json!({
                "severity": finding.severity.to_string(),
                "rule": finding.rule,
                "path": finding.path.to_string(),
                "message": finding.message,
            })
        })
        .collect();

    assemble_output(
        bytes,
        locator,
        status,
        schema_token,
        json!({"code": "VALIDATION_COMPLETED"}),
        summary_json,
        findings,
    )
}

fn usage() -> &'static str {
    "Usage: built-interop-ifc-schema-gate [PATH]\nReads a strict IFC STEP file (default: synthetic fixture), validates against its declared schema, emits a BUILT-INTEROP-004-compatible gate JSON object, and exits 0=PASS, 1=FAIL, 2=UNRESOLVED."
}

fn main() {
    let mut args = env::args().skip(1);
    let first = args.next();
    if matches!(first.as_deref(), Some("--help" | "-h")) {
        println!("{}", usage());
        return;
    }
    if args.next().is_some() {
        eprintln!("{}", usage());
        process::exit(64);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = first
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("fixtures/panel.ifc"));
    let bytes = fs::read(&path).unwrap_or_else(|error| {
        eprintln!("CANNOT_READ_INPUT: {}: {error}", path.display());
        process::exit(66);
    });
    let output = evaluate_bytes(&bytes, &path.display().to_string());
    let status = output["gate"]["status"].as_str().unwrap_or("UNRESOLVED");
    println!(
        "{}",
        serde_json::to_string_pretty(&output).expect("JSON report serialization must succeed")
    );
    match status {
        "PASS" => process::exit(0),
        "FAIL" => process::exit(1),
        _ => process::exit(2),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_IFC: &[u8] = include_bytes!("../fixtures/panel.ifc");

    fn status(output: &Value) -> &str {
        output["gate"]["status"]
            .as_str()
            .expect("gate status must be a string")
    }

    fn summary(output: &Value) -> &Value {
        &output["evidence_payload"]["summary"]
    }

    #[test]
    fn policy_fails_on_errors_and_fails_closed_on_unsupported_checks() {
        assert_eq!(classify_gate(1, 0, 0, false), "FAIL");
        assert_eq!(classify_gate(1, 1, 2, true), "FAIL");
        assert_eq!(classify_gate(0, 1, 0, false), "UNRESOLVED");
        assert_eq!(classify_gate(0, 0, 1, false), "UNRESOLVED");
        assert_eq!(classify_gate(0, 0, 0, true), "UNRESOLVED");
        assert_eq!(classify_gate(0, 0, 0, false), "PASS");
    }

    #[test]
    fn valid_step_keeps_declared_schema_and_byte_provenance() {
        let output = evaluate_bytes(VALID_IFC, "fixture://panel.ifc");
        assert_eq!(output["artifact"]["sha256"], sha256_hex(VALID_IFC));
        assert_eq!(output["artifact"]["byte_length"], VALID_IFC.len());
        assert_eq!(output["artifact"]["locator"], "fixture://panel.ifc");
        assert_eq!(output["evidence_payload"]["declared_schema_token"], "IFC4");
        assert_eq!(output["gate"]["gate_id"], "ifc_schema_conformance");
        assert_eq!(output["gate"]["mandatory"], true);
        assert_eq!(output["gate"]["evidence"][0]["kind"], "validator_report");
        assert_eq!(
            output["gate"]["evidence"][0]["sha256"],
            sha256_hex(&serde_json::to_vec(&output["evidence_payload"]).unwrap())
        );
        assert!(
            output["gate"]["status"] == "PASS"
                || output["gate"]["status"] == "FAIL"
                || output["gate"]["status"] == "UNRESOLVED"
        );
        assert!(
            output["gate"]["tool"]["config_sha256"]
                .as_str()
                .unwrap()
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
        assert!(
            output["gate"]["tool"]["config_sha256"]
                .as_str()
                .unwrap()
                .len()
                == 64
        );
    }

    #[test]
    fn malformed_step_is_unresolved_not_schema_pass() {
        let output = evaluate_bytes(b"not an IFC STEP file", "fixture://malformed.ifc");
        assert_eq!(status(&output), "UNRESOLVED");
        assert_eq!(
            output["evidence_payload"]["decision_reason"]["code"],
            "STEP_PARSE_FAILED"
        );
    }

    #[test]
    fn unknown_declared_schema_is_unresolved_not_guessed_as_ifc4() {
        let mutated = String::from_utf8(VALID_IFC.to_vec())
            .expect("synthetic fixture is UTF-8")
            .replace(
                "FILE_SCHEMA(('IFC4'));",
                "FILE_SCHEMA(('IFC_FUTURE_TEST'));",
            );
        let output = evaluate_bytes(mutated.as_bytes(), "fixture://unknown-schema.ifc");
        assert_eq!(status(&output), "UNRESOLVED");
        assert_eq!(
            output["evidence_payload"]["declared_schema_token"],
            "IFC_FUTURE_TEST"
        );
        assert_eq!(
            output["evidence_payload"]["decision_reason"]["code"],
            "DECLARED_SCHEMA_UNAVAILABLE"
        );
    }

    #[test]
    fn known_but_unbundled_schema_is_unresolved_not_guessed() {
        let mutated = String::from_utf8(VALID_IFC.to_vec())
            .expect("synthetic fixture is UTF-8")
            .replace("FILE_SCHEMA(('IFC4'));", "FILE_SCHEMA(('IFC4X3_ADD2'));");
        let output = evaluate_bytes(mutated.as_bytes(), "fixture://unbundled-schema.ifc");
        assert_eq!(status(&output), "UNRESOLVED");
        assert_eq!(
            output["evidence_payload"]["declared_schema_token"],
            "IFC4X3_ADD2"
        );
        assert_eq!(
            output["evidence_payload"]["decision_reason"]["code"],
            "DECLARED_SCHEMA_UNAVAILABLE"
        );
    }

    #[test]
    fn absent_schema_declaration_is_unresolved() {
        let mutated = String::from_utf8(VALID_IFC.to_vec())
            .expect("synthetic fixture is UTF-8")
            .replace("FILE_SCHEMA(('IFC4'));", "FILE_SCHEMA(());");
        let output = evaluate_bytes(mutated.as_bytes(), "fixture://missing-schema.ifc");
        assert_eq!(status(&output), "UNRESOLVED");
        assert_eq!(
            output["evidence_payload"]["decision_reason"]["code"],
            "DECLARED_SCHEMA_UNAVAILABLE"
        );
    }

    #[test]
    fn malformed_global_id_is_a_schema_failure_when_validator_reports_it() {
        let mutated = String::from_utf8(VALID_IFC.to_vec())
            .expect("synthetic fixture is UTF-8")
            .replace("'0000000000000000000000'", "'not-a-guid'");
        let output = evaluate_bytes(mutated.as_bytes(), "fixture://invalid-guid.ifc");
        assert!(summary(&output)["errors"].as_u64().unwrap() > 0);
        assert_eq!(status(&output), "FAIL");
        assert!(
            output["evidence_payload"]["findings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|finding| {
                    finding["severity"] == "error"
                        && finding["path"].as_str().unwrap().contains("#1")
                })
        );
    }

    #[test]
    fn findings_and_evidence_digest_are_order_stable() {
        let first = evaluate_bytes(VALID_IFC, "fixture://panel.ifc");
        let second = evaluate_bytes(VALID_IFC, "fixture://panel.ifc");
        assert_eq!(first, second);
        let evidence_bytes = serde_json::to_vec(&first["evidence_payload"]).unwrap();
        assert_eq!(
            first["gate"]["evidence"][0]["sha256"],
            sha256_hex(&evidence_bytes)
        );
    }

    #[test]
    fn evidence_preserves_rule_ids_and_paths_as_structured_fields() {
        let output = evaluate_bytes(VALID_IFC, "fixture://panel.ifc");
        for finding in output["evidence_payload"]["findings"]
            .as_array()
            .expect("findings is an array")
        {
            assert!(finding["rule"].is_string());
            assert!(finding["path"].is_string());
            assert!(finding["severity"].is_string());
            assert!(finding["message"].is_string());
        }
    }
}
