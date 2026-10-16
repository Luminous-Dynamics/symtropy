// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-first housing manifest verifier.
//!
//! This tool verifies manifest shape, an independently pinned case/model/rule-pack
//! contract, mandatory claim definitions, and SHA-256 digests for local artifacts.
//! It does not interpret building codes, validate engineering calculations,
//! prove physical truth, or grant professional/statutory approval.
//!
//! Usage:
//!   housing-evidence-verify <manifest.json>
//!   housing-evidence-verify --validate-benchmark-result <benchmark.json> --record <result-record.json>
//!       --benchmark-sha256 <trusted-sha256-pin> --checker-sha256 <trusted-checker-sha256> --artifact-root <directory>
//!       [--expected-claims <requirements.json>]
//!       [--expected-claims-sha256 <sha256-pinned-out-of-band>]
//!       [--artifact-root <directory>]
//!       [--require-pass]
//!
//! The independent expected-claims file pins the embedded schema digest, verifier
//! version and executable digest, case ID, exact model and rule-pack objects, and
//! each mandatory claim's type, statement, scope, requirement, domain, criteria,
//! and exact input/output artifact digests.
//!
//! Artifact URIs used by a PASS manifest must be safe, relative paths beneath
//! --artifact-root. Remote retrieval is intentionally not supported.

mod benchmark_result;
mod home_design_brief;
mod jurisdiction_registry;
mod layout_candidate;
mod evaluation_receipt;
mod evaluation_scope_profile;

use jsonschema::draft202012;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

const STATUS_PASS: &str = "PASS";
const STATUS_EVIDENCE_INTEGRITY_VERIFIED: &str = "EVIDENCE_INTEGRITY_VERIFIED";
const STATUS_EVIDENCE_INTEGRITY_NOT_VERIFIED: &str = "NOT_VERIFIED";
const STATUS_FAIL: &str = "FAIL";
const EMBEDDED_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/housing-evidence-manifest.schema.json");
const STATUS_NOT_EVALUATED: &str = "NOT_EVALUATED";
const STATUS_BLOCKED: &str = "BLOCKED";
const STATUS_INSUFFICIENT_EVIDENCE: &str = "INSUFFICIENT_EVIDENCE";
const STATUS_REQUIRES_REVIEW: &str = "REQUIRES_REVIEW";

#[derive(Debug, Serialize)]
struct VerificationReport {
    verifier_version: String,
    schema_sha256: Option<String>,
    verifier_binary_sha256: Option<String>,
    manifest_sha256: Option<String>,
    expected_claims_sha256: Option<String>,
    expected_claims_digest_pinned: bool,
    schema_valid: bool,
    case_id: Option<String>,
    declared_status: Option<String>,
    effective_status: String,
    evidence_integrity_status: String,
    artifact_digests_verified: usize,
    issues: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedClaimsFile {
    schema_version: String,
    schema_sha256: String,
    verifier_version: String,
    verifier_binary_sha256: String,
    case_id: String,
    jurisdiction: Value,
    model: Value,
    model_artifacts: Vec<ExpectedModelArtifact>,
    rule_pack: Value,
    toolchain: Value,
    mandatory_claims: Vec<ExpectedClaimContract>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedModelArtifact {
    artifact_id: String,
    sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedClaimContract {
    claim_id: String,
    claim_type: String,
    statement: String,
    subject_element_ids: Vec<String>,
    // Explicit null is required when the claim is not tied to a named code requirement.
    requirement: Value,
    tool_id: String,
    input_artifact_sha256: Vec<String>,
    output_artifact_sha256: Vec<String>,
    validity_domain: String,
    acceptance_criteria: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--validate-brief") {
        if args.len() != 2 {
            eprintln!("Usage: housing-evidence-verify --validate-brief <brief.json>");
            return ExitCode::from(2);
        }

        return match home_design_brief::validate_path(Path::new(&args[1])) {
            Ok(report) => {
                match serde_json::to_string_pretty(&report) {
                    Ok(output) => println!("{output}"),
                    Err(error) => {
                        eprintln!("failed to serialize brief validation report: {error}");
                        return ExitCode::from(2);
                    }
                }

                if report.schema_valid && report.semantic_valid {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                }
            }
            Err(error) => {
                eprintln!("home design brief validation failed: {error}");
                ExitCode::from(2)
            }
        };
    }

    if args.first().map(String::as_str) == Some("--validate-jurisdiction-registry") {
        if args.len() != 2 {
            eprintln!("Usage: housing-evidence-verify --validate-jurisdiction-registry <registry.json>");
            return ExitCode::from(2);
        }

        return match jurisdiction_registry::validate_path(Path::new(&args[1])) {
            Ok(report) => {
                match serde_json::to_string_pretty(&report) {
                    Ok(output) => println!("{output}"),
                    Err(error) => {
                        eprintln!("failed to serialize jurisdiction registry report: {error}");
                        return ExitCode::from(2);
                    }
                }
                if report.registry_contract_status == "REGISTRY_VALID" {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                }
            }
            Err(error) => {
                eprintln!("jurisdiction registry validation failed: {error}");
                ExitCode::from(2)
            }
        };
    }

    if args.first().map(String::as_str) == Some("--validate-receipt") {
        if args.len() != 6
            || args.get(2).map(String::as_str) != Some("--registry")
            || args.get(4).map(String::as_str) != Some("--scope-profile-sha256")
            || !args.get(5).is_some_and(|value| {
                value.len() == 64
                    && value.bytes().all(|byte| {
                        byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
                    })
            })
        {
            eprintln!(
                "Usage: housing-evidence-verify --validate-receipt <receipt.json> --registry <registry.json> --scope-profile-sha256 <trusted-sha256-pin>"
            );
            return ExitCode::from(2);
        }
        return match evaluation_receipt::validate_paths(
            Path::new(&args[1]),
            Path::new(&args[3]),
            &args[5],
        ) {
            Ok(report) => {
                match serde_json::to_string_pretty(&report) {
                    Ok(output) => println!("{output}"),
                    Err(error) => {
                        eprintln!("failed to serialize rule evaluation receipt report: {error}");
                        return ExitCode::from(2);
                    }
                }
                if report.receipt_integrity_status == "RECEIPT_INTEGRITY_VERIFIED" {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                }
            }
            Err(error) => {
                eprintln!("rule evaluation receipt validation failed: {error}");
                ExitCode::from(2)
            }
        };
    }


    if args.first().map(String::as_str) == Some("--validate-benchmark-result") {
        if args.len() != 10
            || args.get(2).map(String::as_str) != Some("--record")
            || args.get(4).map(String::as_str) != Some("--benchmark-sha256")
            || args.get(6).map(String::as_str) != Some("--checker-sha256")
            || args.get(8).map(String::as_str) != Some("--artifact-root")
            || ![args.get(5), args.get(7)].into_iter().all(|value| {
                value.is_some_and(|value| {
                    value.len() == 64
                        && value.bytes().all(|byte| {
                            byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
                        })
                })
            })
        {
            eprintln!(
                "Usage: housing-evidence-verify --validate-benchmark-result <benchmark.json> --record <result-record.json> --benchmark-sha256 <trusted-sha256-pin> --checker-sha256 <trusted-checker-sha256> --artifact-root <directory>"
            );
            return ExitCode::from(2);
        }

        return match benchmark_result::validate_paths(
            Path::new(&args[1]),
            Path::new(&args[3]),
            &args[5],
            &args[7],
            Path::new(&args[9]),
        ) {
            Ok(report) => {
                let accepted = report.independent_result_status == "REFERENCE_MATCH"
                    && report.declared_status_consistent == Some(true);
                match serde_json::to_string_pretty(&report) {
                    Ok(output) => println!("{output}"),
                    Err(error) => {
                        eprintln!("failed to serialize benchmark result report: {error}");
                        return ExitCode::from(2);
                    }
                }
                if accepted {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                }
            }
            Err(error) => {
                eprintln!("benchmark result verification failed: {error}");
                ExitCode::from(2)
            }
        };
    }

    if args.first().map(String::as_str) == Some("--validate-layout") {
        if args.len() != 4 || args.get(2).map(String::as_str) != Some("--brief") {
            eprintln!(
                "Usage: housing-evidence-verify --validate-layout <candidate.json> --brief <brief.json>"
            );
            return ExitCode::from(2);
        }

        return match layout_candidate::validate_paths(Path::new(&args[1]), Path::new(&args[3])) {
            Ok(report) => {
                match serde_json::to_string_pretty(&report) {
                    Ok(output) => println!("{output}"),
                    Err(error) => {
                        eprintln!("failed to serialize layout validation report: {error}");
                        return ExitCode::from(2);
                    }
                }
                if report.contract_status == "LAYOUT_CANDIDATE_CONTRACT_VALID" {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(2)
                }
            }
            Err(error) => {
                eprintln!("home layout candidate validation failed: {error}");
                ExitCode::from(2)
            }
        };
    }

    match run(args) {
        Ok((report, require_pass)) => {
            match serde_json::to_string_pretty(&report) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("failed to serialize verification report: {error}");
                    return ExitCode::from(2);
                }
            }

            if !report.schema_valid || report.effective_status == STATUS_BLOCKED {
                return ExitCode::from(2);
            }
            if report.declared_status.as_deref() == Some(STATUS_PASS)
                && (report.effective_status != STATUS_PASS
                    || report.evidence_integrity_status != STATUS_EVIDENCE_INTEGRITY_VERIFIED)
            {
                return ExitCode::from(2);
            }
            if require_pass
                && (report.declared_status.as_deref() != Some(STATUS_PASS)
                    || report.effective_status != STATUS_PASS
                    || report.evidence_integrity_status != STATUS_EVIDENCE_INTEGRITY_VERIFIED)
            {
                return ExitCode::from(3);
            }
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("housing evidence verification failed: {error}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<(VerificationReport, bool), String> {
    if args.is_empty() {
        return Err(usage());
    }

    let manifest_path = PathBuf::from(&args[0]);
    let mut expected_claims_path: Option<PathBuf> = None;
    let mut expected_claims_sha256: Option<String> = None;
    let mut artifact_root: Option<PathBuf> = None;
    let mut require_pass = false;

    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--expected-claims" => {
                if expected_claims_path.is_some() {
                    return Err("--expected-claims may only be supplied once".to_string());
                }
                index += 1;
                let value = args.get(index).ok_or_else(usage)?;
                expected_claims_path = Some(PathBuf::from(value));
            }
            "--expected-claims-sha256" => {
                if expected_claims_sha256.is_some() {
                    return Err("--expected-claims-sha256 may only be supplied once".to_string());
                }
                index += 1;
                let value = args.get(index).ok_or_else(usage)?;
                if !is_sha256(value) {
                    return Err(
                        "--expected-claims-sha256 must be 64 lowercase hexadecimal characters"
                            .to_string(),
                    );
                }
                expected_claims_sha256 = Some(value.to_string());
            }
            "--artifact-root" => {
                if artifact_root.is_some() {
                    return Err("--artifact-root may only be supplied once".to_string());
                }
                index += 1;
                let value = args.get(index).ok_or_else(usage)?;
                artifact_root = Some(PathBuf::from(value));
            }
            "--require-pass" => {
                if require_pass {
                    return Err("--require-pass may only be supplied once".to_string());
                }
                require_pass = true;
            }
            unknown => return Err(format!("unknown argument {unknown:?}\n{}", usage())),
        }
        index += 1;
    }

    let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA)
        .map_err(|error| format!("embedded evidence schema is not valid JSON: {error}"))?;
    reject_external_refs(&schema)?;
    draft202012::meta::validate(&schema)
        .map_err(|error| format!("schema is invalid under JSON Schema 2020-12: {error}"))?;

    if expected_claims_sha256.is_some() && expected_claims_path.is_none() {
        return Err("--expected-claims-sha256 requires --expected-claims".to_string());
    }

    let schema_sha256 = sha256_hex(EMBEDDED_SCHEMA.as_bytes());
    let verifier_binary_sha256 = current_verifier_binary_sha256()?;

    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", manifest_path.display()))?;
    let manifest_sha256 = sha256_hex(&manifest_bytes);

    let expected_claim_data = expected_claims_path
        .as_deref()
        .map(read_expected_claims_file)
        .transpose()?;
    let (expected_claims, expected_claims_actual_sha256) = match expected_claim_data {
        Some((contracts, digest)) => (Some(contracts), Some(digest)),
        None => (None, None),
    };
    let expected_claims_pinned = match (
        expected_claims_sha256.as_deref(),
        expected_claims_actual_sha256.as_deref(),
    ) {
        (Some(expected), Some(actual)) if expected == actual => true,
        (Some(_), Some(actual)) => {
            return Err(format!(
                "mandatory-claims file SHA-256 mismatch; actual digest is {actual}"
            ));
        }
        (None, Some(_)) | (None, None) => false,
        (Some(_), None) => {
            return Err("--expected-claims-sha256 requires --expected-claims".to_string());
        }
    };

    let mut report = evaluate_manifest(
        &schema,
        &manifest,
        expected_claims.as_ref(),
        expected_claims_pinned,
        artifact_root.as_deref(),
    );
    report.schema_sha256 = Some(schema_sha256);
    report.verifier_binary_sha256 = Some(verifier_binary_sha256);
    report.manifest_sha256 = Some(manifest_sha256);
    report.expected_claims_sha256 = expected_claims_actual_sha256;
    report.expected_claims_digest_pinned = expected_claims_pinned;
    Ok((report, require_pass))
}

fn usage() -> String {
    [
        "Usage: housing-evidence-verify <manifest.json>",
        "       housing-evidence-verify --validate-brief <brief.json>",
        "       housing-evidence-verify --validate-jurisdiction-registry <registry.json>",
        "       housing-evidence-verify --validate-receipt <receipt.json> --registry <registry.json> --scope-profile-sha256 <trusted-sha256-pin>",
        "       housing-evidence-verify --validate-layout <candidate.json> --brief <brief.json>",
        "       housing-evidence-verify --validate-benchmark-result <benchmark.json> --record <result-record.json> --benchmark-sha256 <trusted-sha256-pin> --checker-sha256 <trusted-checker-sha256> --artifact-root <directory>",
        "  [--expected-claims <requirements.json>]",
        "  [--expected-claims-sha256 <sha256-pinned-out-of-band>]",
        "  [--artifact-root <directory>] [--require-pass]",
    ]
    .join("\n")
}

fn read_expected_claims_file(path: &Path) -> Result<(ExpectedClaimsFile, String), String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let expected: ExpectedClaimsFile = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "{} is not a valid expected-claims contract: {error}",
            path.display()
        )
    })?;

    if expected.schema_version.trim().is_empty() {
        return Err("expected-claims schema_version must not be empty".to_string());
    }
    if !is_sha256(&expected.schema_sha256) {
        return Err(
            "expected-claims schema_sha256 must be 64 lowercase hexadecimal characters".to_string(),
        );
    }
    if expected.verifier_version.trim().is_empty() {
        return Err("expected-claims verifier_version must not be empty".to_string());
    }
    if !is_sha256(&expected.verifier_binary_sha256) {
        return Err(
            "expected-claims verifier_binary_sha256 must be 64 lowercase hexadecimal characters"
                .to_string(),
        );
    }
    if expected.case_id.trim().is_empty() {
        return Err("expected-claims case_id must not be empty".to_string());
    }
    if !expected.jurisdiction.is_object() {
        return Err("expected-claims jurisdiction must be an object".to_string());
    }
    if !expected.model.is_object() {
        return Err("expected-claims model must be an object".to_string());
    }
    if expected.model_artifacts.is_empty() {
        return Err("expected-claims model_artifacts must not be empty".to_string());
    }
    let mut model_artifact_ids = BTreeSet::new();
    for pin in &expected.model_artifacts {
        if pin.artifact_id.trim().is_empty() {
            return Err("expected model artifact IDs must not be empty".to_string());
        }
        if !is_sha256(&pin.sha256) {
            return Err(format!(
                "malformed expected model SHA-256 for artifact {}: {}",
                pin.artifact_id, pin.sha256
            ));
        }
        if !model_artifact_ids.insert(pin.artifact_id.as_str()) {
            return Err(format!(
                "duplicate expected model artifact ID: {}",
                pin.artifact_id
            ));
        }
    }
    if !expected.rule_pack.is_object() {
        return Err("expected-claims rule_pack must be an object".to_string());
    }
    if !expected.toolchain.is_array()
        || expected
            .toolchain
            .as_array()
            .is_some_and(|tools| tools.is_empty())
    {
        return Err("expected-claims toolchain must be a non-empty array".to_string());
    }
    if expected.mandatory_claims.is_empty() {
        return Err("expected-claims mandatory_claims must not be empty".to_string());
    }

    let mut result = BTreeSet::new();
    for claim in &expected.mandatory_claims {
        if claim.claim_id.trim().is_empty()
            || claim.claim_type.trim().is_empty()
            || claim.statement.trim().is_empty()
            || claim.tool_id.trim().is_empty()
            || claim.input_artifact_sha256.is_empty()
            || claim.output_artifact_sha256.is_empty()
            || claim.validity_domain.trim().is_empty()
            || claim.acceptance_criteria.trim().is_empty()
        {
            return Err("mandatory claim fields must be non-empty".to_string());
        }
        if !(claim.requirement.is_null() || claim.requirement.is_object()) {
            return Err(format!(
                "expected claim {} requirement must be an object or explicit null",
                claim.claim_id
            ));
        }
        let mut input_digests = BTreeSet::new();
        for digest in &claim.input_artifact_sha256 {
            if !is_sha256(digest) {
                return Err(format!(
                    "expected claim {} has a malformed input artifact SHA-256: {digest}",
                    claim.claim_id
                ));
            }
            if !input_digests.insert(digest) {
                return Err(format!(
                    "expected claim {} has duplicate input artifact digest: {digest}",
                    claim.claim_id
                ));
            }
        }
        let mut output_digests = BTreeSet::new();
        for digest in &claim.output_artifact_sha256 {
            if !is_sha256(digest) {
                return Err(format!(
                    "expected claim {} has a malformed output artifact SHA-256: {digest}",
                    claim.claim_id
                ));
            }
            if !output_digests.insert(digest) {
                return Err(format!(
                    "expected claim {} has duplicate output artifact digest: {digest}",
                    claim.claim_id
                ));
            }
        }
        let mut elements = BTreeSet::new();
        for element_id in &claim.subject_element_ids {
            if element_id.trim().is_empty() {
                return Err(format!(
                    "expected claim {} has an empty subject element ID",
                    claim.claim_id
                ));
            }
            if !elements.insert(element_id) {
                return Err(format!(
                    "expected claim {} has duplicate subject element ID: {element_id}",
                    claim.claim_id
                ));
            }
        }
        if !result.insert(claim.claim_id.clone()) {
            return Err(format!(
                "duplicate expected mandatory claim ID: {}",
                claim.claim_id
            ));
        }
    }

    Ok((expected, sha256_hex(&bytes)))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn reject_external_refs(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str)
                && reference != "#"
                && !reference.starts_with("#/")
            {
                return Err(format!(
                    "external JSON Schema reference is disabled: {reference}"
                ));
            }
            for child in object.values() {
                reject_external_refs(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                reject_external_refs(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn evaluate_manifest(
    schema: &Value,
    manifest: &Value,
    expected_claims: Option<&ExpectedClaimsFile>,
    expected_claims_pinned: bool,
    artifact_root: Option<&Path>,
) -> VerificationReport {
    let case_id = manifest
        .get("case_id")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let declared_status = manifest
        .get("overall_status")
        .and_then(Value::as_str)
        .map(ToString::to_string);

    let validator = match draft202012::options()
        .should_validate_formats(true)
        .build(schema)
    {
        Ok(validator) => validator,
        Err(error) => {
            return report(
                false,
                case_id,
                declared_status,
                STATUS_BLOCKED,
                0,
                vec![format!("cannot compile schema: {error}")],
            );
        }
    };

    let schema_issues: Vec<String> = validator
        .iter_errors(manifest)
        .map(|error| format!("schema validation: {error}"))
        .collect();
    if !schema_issues.is_empty() {
        return report(
            false,
            case_id,
            declared_status,
            STATUS_BLOCKED,
            0,
            schema_issues,
        );
    }

    let mut issues = Vec::new();
    let artifacts = manifest["evidence_artifacts"]
        .as_array()
        .expect("schema requires evidence_artifacts array");
    let claims = manifest["claims"]
        .as_array()
        .expect("schema requires claims array");

    let mut artifacts_by_id: BTreeMap<&str, &Value> = BTreeMap::new();
    let mut artifacts_by_digest: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for artifact in artifacts {
        let id = artifact["artifact_id"]
            .as_str()
            .expect("schema requires artifact_id");
        let digest = artifact["sha256"]
            .as_str()
            .expect("schema requires SHA-256 digest");

        if artifacts_by_id.insert(id, artifact).is_some() {
            issues.push(format!("duplicate artifact_id: {id}"));
        }
        artifacts_by_digest
            .entry(digest)
            .or_default()
            .push(artifact);
    }

    let toolchain = manifest["toolchain"]
        .as_array()
        .expect("schema requires toolchain array");
    let mut tools_by_id: BTreeMap<&str, &Value> = BTreeMap::new();
    for tool in toolchain {
        let id = tool["tool_id"].as_str().expect("schema requires tool_id");
        if tools_by_id.insert(id, tool).is_some() {
            issues.push(format!("duplicate tool_id in toolchain: {id}"));
        }
    }

    let source_references = manifest["rule_pack"]["source_references"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut source_ids = BTreeSet::new();
    for source in &source_references {
        if let Some(id) = source["source_id"].as_str()
            && !source_ids.insert(id.to_string())
        {
            issues.push(format!("duplicate source_id in rule pack: {id}"));
        }
    }

    for id in manifest["model"]["artifact_ids"]
        .as_array()
        .expect("schema requires model.artifact_ids")
    {
        let id = id.as_str().expect("schema requires string artifact IDs");
        if !artifacts_by_id.contains_key(id) {
            issues.push(format!("model references unknown artifact_id: {id}"));
        }
    }

    let mut claim_ids = BTreeSet::new();
    for claim in claims {
        let id = claim["claim_id"]
            .as_str()
            .expect("schema requires claim_id");
        if !claim_ids.insert(id.to_string()) {
            issues.push(format!("duplicate claim_id: {id}"));
        }

        if claim["status"]
            .as_str()
            .is_some_and(|status| status == STATUS_PASS || status == STATUS_FAIL)
        {
            let Some(tool_id) = claim["tool_id"].as_str() else {
                issues.push(format!("evaluated claim {id} has no tool_id"));
                continue;
            };
            let Some(tool) = tools_by_id.get(tool_id) else {
                issues.push(format!(
                    "evaluated claim {id} references unknown tool_id: {tool_id}"
                ));
                continue;
            };

            let Some(executable_digest) = tool
                .get("executable_or_container_sha256")
                .and_then(Value::as_str)
            else {
                issues.push(format!(
                    "evaluated claim {id} has no executable/container digest for tool {tool_id}"
                ));
                continue;
            };
            if !artifacts_by_digest.contains_key(executable_digest) {
                issues.push(format!(
                    "tool {tool_id} executable/container digest is absent from evidence_artifacts: {executable_digest}"
                ));
            }

            if let Some(config_digest) = tool
                .get("configuration_artifact_sha256")
                .and_then(Value::as_str)
                && !artifacts_by_digest.contains_key(config_digest)
            {
                issues.push(format!(
                    "tool {tool_id} configuration digest is absent from evidence_artifacts: {config_digest}"
                ));
            }
        }

        if claim["claim_type"].as_str() == Some("code_compliance") {
            let source_id = claim["requirement"]["source_id"].as_str();
            match source_id {
                Some(source_id) if source_ids.contains(source_id) => {
                    if claim["mandatory"].as_bool() == Some(true)
                        && claim["status"].as_str().is_some_and(|status| {
                            status == STATUS_PASS || status == STATUS_FAIL
                        })
                    {
                        let source = source_references
                            .iter()
                            .find(|source| source["source_id"].as_str() == Some(source_id));
                        if let Some(source) = source {
                            if !matches!(
                                source["source_type"].as_str(),
                                Some(
                                    "legislation"
                                        | "standard"
                                        | "regulation"
                                        | "technical_guidance"
                                )
                            ) {
                                issues.push(format!(
                                    "mandatory code compliance claim {id} source {source_id} is not a legislation, standard, regulation, or technical-guidance source"
                                ));
                            }

                            let has_edition = source["edition"]
                                .as_str()
                                .is_some_and(|value| !value.trim().is_empty());
                            let has_effective_date = source["effective_from"]
                                .as_str()
                                .is_some_and(|value| !value.trim().is_empty());
                            if !has_edition && !has_effective_date {
                                issues.push(format!(
                                    "mandatory code compliance claim {id} source {source_id} has no edition or effective_from date"
                                ));
                            }
                            if source["clause_or_location"]
                                .as_str()
                                .is_none_or(|value| value.trim().is_empty())
                            {
                                issues.push(format!(
                                    "mandatory code compliance claim {id} source {source_id} has no clause_or_location"
                                ));
                            }
                            if !matches!(
                                source["rights_status"].as_str(),
                                Some("public" | "licensed")
                            ) {
                                issues.push(format!(
                                    "mandatory code compliance claim {id} source {source_id} has unresolved or restricted rights status"
                                ));
                            }

                            match source["source_sha256"].as_str() {
                                Some(digest) if artifacts_by_digest.contains_key(digest) => {}
                                Some(digest) => issues.push(format!(
                                    "mandatory code compliance claim {id} source digest {digest} is absent from evidence_artifacts"
                                )),
                                None => issues.push(format!(
                                    "mandatory code compliance claim {id} source {source_id} has no source_sha256"
                                )),
                            }
                        } else {
                            issues.push(format!(
                                "mandatory code compliance claim {id} source {source_id} could not be resolved"
                            ));
                        }
                    }
                }
                Some(source_id) => issues.push(format!(
                    "code compliance claim {id} references source_id {source_id} absent from the rule pack"
                )),
                None => issues.push(format!("code compliance claim {id} has no source_id")),
            }
        }

        for field in ["input_artifact_sha256", "output_artifact_sha256"] {
            if let Some(digests) = claim.get(field).and_then(Value::as_array) {
                for digest in digests.iter().filter_map(Value::as_str) {
                    if !artifacts_by_digest.contains_key(digest) {
                        issues.push(format!(
                            "claim {id} references {field} digest absent from evidence_artifacts: {digest}"
                        ));
                    }
                }
            }
        }
    }

    if let Some(digest) = manifest["model"]
        .get("unit_mapping_artifact_sha256")
        .and_then(Value::as_str)
        && !artifacts_by_digest.contains_key(digest)
    {
        issues.push(format!(
            "unit_mapping_artifact_sha256 is absent from evidence_artifacts: {digest}"
        ));
    }

    if !issues.is_empty() {
        return report(true, case_id, declared_status, STATUS_BLOCKED, 0, issues);
    }

    // A stronger mandatory claim state must not be hidden by a weaker
    // producer-declared aggregate. This is conservative aggregation, not a
    // re-evaluation of the underlying engineering claim.
    if declared_status.as_deref() != Some(STATUS_PASS) {
        let declared = declared_status
            .as_deref()
            .unwrap_or(STATUS_BLOCKED)
            .to_string();
        let highest_claim_status = claims
            .iter()
            .filter(|claim| claim["mandatory"].as_bool() == Some(true))
            .filter_map(|claim| claim["status"].as_str())
            .max_by_key(|status| status_precedence(status))
            .unwrap_or(declared.as_str());

        if status_precedence(highest_claim_status) > status_precedence(&declared) {
            let highest_claim_ids: Vec<String> = claims
                .iter()
                .filter(|claim| claim["mandatory"].as_bool() == Some(true))
                .filter(|claim| claim["status"].as_str() == Some(highest_claim_status))
                .filter_map(|claim| claim["claim_id"].as_str().map(ToString::to_string))
                .collect();
            let issue = format!(
                "declared aggregate status {declared:?} understates mandatory claim state {highest_claim_status:?} from claim(s) {highest_claim_ids:?}; conservative status takes precedence"
            );
            return report(
                true,
                case_id,
                declared_status,
                highest_claim_status,
                0,
                vec![issue],
            );
        }

        return report(true, case_id, declared_status, &declared, 0, Vec::new());
    }

    let Some(expected_claims) = expected_claims else {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_INSUFFICIENT_EVIDENCE,
            0,
            vec![
                "overall PASS requires an independent --expected-claims contract; manifest self-description cannot prove case, model, rule-pack, or requirement completeness"
                    .to_string(),
            ],
        );
    };

    if !expected_claims_pinned {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_INSUFFICIENT_EVIDENCE,
            0,
            vec![
                "overall PASS requires --expected-claims-sha256 pinned outside the manifest"
                    .to_string(),
            ],
        );
    }

    let actual_model_artifacts: BTreeMap<String, String> = manifest["model"]["artifact_ids"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|id| {
            artifacts_by_id
                .get(id)
                .and_then(|artifact| artifact["sha256"].as_str())
                .map(|digest| (id.to_string(), digest.to_string()))
        })
        .collect();
    let expected_model_artifacts: BTreeMap<String, String> = expected_claims
        .model_artifacts
        .iter()
        .map(|pin| (pin.artifact_id.clone(), pin.sha256.clone()))
        .collect();

    let expected_schema_sha256 = sha256_hex(EMBEDDED_SCHEMA.as_bytes());
    if expected_claims.schema_sha256 != expected_schema_sha256
        || expected_claims.verifier_version != env!("CARGO_PKG_VERSION")
    {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_INSUFFICIENT_EVIDENCE,
            0,
            vec![
                "embedded schema digest or verifier package version differs from the independently pinned contract".to_string(),
            ],
        );
    }

    let actual_verifier_binary_sha256 = match current_verifier_binary_sha256() {
        Ok(digest) => digest,
        Err(error) => {
            return report(
                true,
                case_id,
                declared_status,
                STATUS_INSUFFICIENT_EVIDENCE,
                0,
                vec![format!("cannot hash current verifier executable: {error}")],
            );
        }
    };
    if expected_claims.verifier_binary_sha256 != actual_verifier_binary_sha256 {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_INSUFFICIENT_EVIDENCE,
            0,
            vec![
                "verifier executable digest differs from the independently pinned contract"
                    .to_string(),
            ],
        );
    }

    let Some(artifact_root) = artifact_root else {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_BLOCKED,
            0,
            vec![
                "overall PASS requires --artifact-root so referenced artifacts can be read and hashed"
                    .to_string(),
            ],
        );
    };
    let referenced_digests = referenced_digests(manifest);
    let verified_artifact_count =
        match verify_local_artifacts(&referenced_digests, &artifacts_by_digest, artifact_root) {
            Ok(verified_count) => verified_count,
            Err(mut artifact_issues) => {
                issues.append(&mut artifact_issues);
                return report(true, case_id, declared_status, STATUS_BLOCKED, 0, issues);
            }
        };

    if expected_claims.schema_version != manifest["schema_version"].as_str().unwrap_or_default()
        || expected_claims.case_id != case_id.as_deref().unwrap_or_default()
        || expected_claims.jurisdiction != manifest["jurisdiction"]
        || expected_claims.model != manifest["model"]
        || actual_model_artifacts != expected_model_artifacts
        || expected_claims.rule_pack != manifest["rule_pack"]
        || expected_claims.toolchain != manifest["toolchain"]
    {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_INSUFFICIENT_EVIDENCE,
            0,
            vec![
                "manifest schema/case/jurisdiction/model object, model artifact ID-to-digest mapping, rule-pack, or toolchain differs from the independently pinned contract"
                    .to_string(),
            ],
        );
    }

    let expected_claim_ids: BTreeSet<String> = expected_claims
        .mandatory_claims
        .iter()
        .map(|claim| claim.claim_id.clone())
        .collect();
    let actual_mandatory_ids: BTreeSet<String> = claims
        .iter()
        .filter(|claim| claim["mandatory"].as_bool() == Some(true))
        .filter_map(|claim| claim["claim_id"].as_str().map(ToString::to_string))
        .collect();

    if actual_mandatory_ids != expected_claim_ids {
        let missing: Vec<_> = expected_claim_ids
            .difference(&actual_mandatory_ids)
            .cloned()
            .collect();
        let unexpected: Vec<_> = actual_mandatory_ids
            .difference(&expected_claim_ids)
            .cloned()
            .collect();
        return report(
            true,
            case_id,
            declared_status,
            STATUS_INSUFFICIENT_EVIDENCE,
            0,
            vec![format!(
                "mandatory claim set does not match the independent requirements file; missing={missing:?}, unexpected={unexpected:?}"
            )],
        );
    }

    let mut contract_issues = Vec::new();
    for expected in &expected_claims.mandatory_claims {
        let Some(actual) = claims
            .iter()
            .find(|claim| claim["claim_id"].as_str() == Some(expected.claim_id.as_str()))
        else {
            contract_issues.push(format!(
                "expected mandatory claim {} is missing from the manifest",
                expected.claim_id
            ));
            continue;
        };

        let actual_elements: BTreeSet<String> = actual["subject_element_ids"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect();
        let expected_elements: BTreeSet<String> =
            expected.subject_element_ids.iter().cloned().collect();

        if actual["claim_type"].as_str() != Some(expected.claim_type.as_str()) {
            contract_issues.push(format!(
                "mandatory claim {} differs from the pinned claim_type",
                expected.claim_id
            ));
        }
        if actual["statement"].as_str() != Some(expected.statement.as_str()) {
            contract_issues.push(format!(
                "mandatory claim {} differs from the pinned statement",
                expected.claim_id
            ));
        }
        if actual_elements != expected_elements {
            contract_issues.push(format!(
                "mandatory claim {} differs from the pinned subject element scope",
                expected.claim_id
            ));
        }
        if actual.get("requirement") != Some(&expected.requirement) {
            contract_issues.push(format!(
                "mandatory claim {} differs from the pinned requirement reference",
                expected.claim_id
            ));
        }
        if actual["tool_id"].as_str() != Some(expected.tool_id.as_str()) {
            contract_issues.push(format!(
                "mandatory claim {} differs from the pinned tool_id",
                expected.claim_id
            ));
        }
        let actual_inputs: BTreeSet<String> = actual["input_artifact_sha256"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect();
        let expected_inputs: BTreeSet<String> =
            expected.input_artifact_sha256.iter().cloned().collect();
        if actual_inputs != expected_inputs {
            contract_issues.push(format!(
                "mandatory claim {} differs from the pinned input artifact digests",
                expected.claim_id
            ));
        }
        let actual_outputs: BTreeSet<String> = actual["output_artifact_sha256"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect();
        let expected_outputs: BTreeSet<String> =
            expected.output_artifact_sha256.iter().cloned().collect();
        if actual_outputs != expected_outputs {
            contract_issues.push(format!(
                "mandatory claim {} differs from the pinned output artifact digests",
                expected.claim_id
            ));
        }
        if actual["validity_domain"].as_str() != Some(expected.validity_domain.as_str()) {
            contract_issues.push(format!(
                "mandatory claim {} differs from the pinned validity domain",
                expected.claim_id
            ));
        }
        if actual["acceptance_criteria"].as_str() != Some(expected.acceptance_criteria.as_str()) {
            contract_issues.push(format!(
                "mandatory claim {} differs from the pinned acceptance criteria",
                expected.claim_id
            ));
        }
    }

    if !contract_issues.is_empty() {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_INSUFFICIENT_EVIDENCE,
            0,
            contract_issues,
        );
    }

    if actual_mandatory_ids.is_empty() {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_INSUFFICIENT_EVIDENCE,
            0,
            vec!["at least one mandatory claim is required for PASS".to_string()],
        );
    }

    let mut result = report(
        true,
        case_id,
        declared_status,
        STATUS_PASS,
        verified_artifact_count,
        Vec::new(),
    );
    result.evidence_integrity_status = STATUS_EVIDENCE_INTEGRITY_VERIFIED.to_string();
    result
}

fn referenced_digests(manifest: &Value) -> BTreeSet<String> {
    let mut result = BTreeSet::new();

    for artifact_id in manifest["model"]["artifact_ids"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if let Some(digest) = manifest["evidence_artifacts"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|artifact| artifact["artifact_id"].as_str() == Some(artifact_id))
            .and_then(|artifact| artifact["sha256"].as_str())
        {
            result.insert(digest.to_string());
        }
    }

    if let Some(digest) = manifest["model"]
        .get("unit_mapping_artifact_sha256")
        .and_then(Value::as_str)
    {
        result.insert(digest.to_string());
    }

    for source in manifest["rule_pack"]["source_references"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if let Some(digest) = source.get("source_sha256").and_then(Value::as_str) {
            result.insert(digest.to_string());
        }
    }

    for tool in manifest["toolchain"].as_array().into_iter().flatten() {
        for field in [
            "executable_or_container_sha256",
            "configuration_artifact_sha256",
        ] {
            if let Some(digest) = tool.get(field).and_then(Value::as_str) {
                result.insert(digest.to_string());
            }
        }
    }

    for claim in manifest["claims"].as_array().into_iter().flatten() {
        for field in ["input_artifact_sha256", "output_artifact_sha256"] {
            for digest in claim[field]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                result.insert(digest.to_string());
            }
        }
    }
    result
}

fn verify_local_artifacts(
    digests: &BTreeSet<String>,
    artifacts_by_digest: &BTreeMap<&str, Vec<&Value>>,
    artifact_root: &Path,
) -> Result<usize, Vec<String>> {
    let canonical_root = fs::canonicalize(artifact_root).map_err(|error| {
        vec![format!(
            "cannot resolve artifact root {}: {error}",
            artifact_root.display()
        )]
    })?;

    if !canonical_root.is_dir() {
        return Err(vec![format!(
            "artifact root is not a directory: {}",
            canonical_root.display()
        )]);
    }

    let mut issues = Vec::new();
    let mut verified = BTreeSet::new();

    for digest in digests {
        let Some(artifact_records) = artifacts_by_digest.get(digest.as_str()) else {
            issues.push(format!(
                "referenced digest has no artifact record: {digest}"
            ));
            continue;
        };

        let mut found_verified_bytes = false;
        let mut local_paths = Vec::new();

        for artifact in artifact_records {
            let artifact_id = artifact["artifact_id"].as_str().unwrap_or("<unknown>");
            let Some(uri) = artifact["uri"].as_str() else {
                continue;
            };
            match safe_local_artifact_path(&canonical_root, uri) {
                Ok(path) => local_paths.push((artifact_id.to_string(), path)),
                Err(error) => issues.push(format!("artifact {artifact_id}: {error}")),
            }
        }

        if local_paths.is_empty() {
            issues.push(format!(
                "no local artifact URI is available under --artifact-root for digest {digest}"
            ));
            continue;
        }

        for (artifact_id, path) in local_paths {
            match fs::read(&path) {
                Ok(bytes) if sha256_hex(&bytes).as_str() == digest.as_str() => {
                    found_verified_bytes = true;
                    verified.insert(digest.clone());
                }
                Ok(_) => issues.push(format!(
                    "SHA-256 mismatch for artifact {artifact_id} at {}",
                    path.display()
                )),
                Err(error) => issues.push(format!(
                    "cannot read artifact {artifact_id} at {}: {error}",
                    path.display()
                )),
            }
        }

        if !found_verified_bytes {
            issues.push(format!(
                "no readable artifact bytes matched referenced digest {digest}"
            ));
        }
    }

    if issues.is_empty() {
        Ok(verified.len())
    } else {
        Err(issues)
    }
}

fn safe_local_artifact_path(root: &Path, uri: &str) -> Result<PathBuf, String> {
    if uri.is_empty()
        || uri.contains('%')
        || uri.contains('\\')
        || uri.contains('?')
        || uri.contains('#')
        || uri.contains(':')
    {
        return Err(format!(
            "artifact URI must be an unescaped relative path, got {uri:?}"
        ));
    }

    let relative = Path::new(uri);
    if relative.is_absolute() {
        return Err(format!("absolute artifact paths are forbidden: {uri:?}"));
    }

    if relative
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "artifact path traversal or non-normal component is forbidden: {uri:?}"
        ));
    }

    let joined = root.join(relative);
    let canonical = fs::canonicalize(&joined)
        .map_err(|error| format!("cannot resolve artifact path {uri:?}: {error}"))?;
    if !canonical.starts_with(root) {
        return Err(format!(
            "artifact path escapes the declared artifact root: {uri:?}"
        ));
    }
    if !canonical.is_file() {
        return Err(format!("artifact path is not a regular file: {uri:?}"));
    }
    Ok(canonical)
}

fn current_verifier_binary_sha256() -> Result<String, String> {
    let executable = env::current_exe()
        .map_err(|error| format!("cannot resolve verifier executable: {error}"))?;
    let bytes = fs::read(&executable).map_err(|error| {
        format!(
            "cannot read verifier executable {}: {error}",
            executable.display()
        )
    })?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn status_precedence(status: &str) -> u8 {
    match status {
        STATUS_PASS => 0,
        STATUS_NOT_EVALUATED => 1,
        STATUS_REQUIRES_REVIEW => 2,
        STATUS_INSUFFICIENT_EVIDENCE => 3,
        STATUS_BLOCKED => 4,
        STATUS_FAIL => 5,
        _ => 4,
    }
}

fn report(
    schema_valid: bool,
    case_id: Option<String>,
    declared_status: Option<String>,
    effective_status: &str,
    artifact_digests_verified: usize,
    issues: Vec<String>,
) -> VerificationReport {
    VerificationReport {
        verifier_version: env!("CARGO_PKG_VERSION").to_string(),
        schema_sha256: None,
        verifier_binary_sha256: None,
        manifest_sha256: None,
        expected_claims_sha256: None,
        expected_claims_digest_pinned: false,
        schema_valid,
        case_id,
        declared_status,
        effective_status: effective_status.to_string(),
        evidence_integrity_status: STATUS_EVIDENCE_INTEGRITY_NOT_VERIFIED.to_string(),
        artifact_digests_verified,
        issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_values() -> (Value, Value) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/built-environment");
        let schema: Value = serde_json::from_slice(
            &fs::read(root.join("housing-evidence-manifest.schema.json"))
                .expect("schema fixture exists"),
        )
        .expect("schema parses");
        let manifest: Value = serde_json::from_slice(
            &fs::read(root.join("fixtures/example-not-evidence.json"))
                .expect("manifest fixture exists"),
        )
        .expect("manifest parses");
        (schema, manifest)
    }

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = env::temp_dir().join(format!(
            "symtropy-housing-evidence-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp directory");
        path
    }

    fn set_up_pass_manifest() -> (Value, Value, PathBuf, ExpectedClaimsFile) {
        let (schema, mut manifest) = fixture_values();
        let root = temp_dir("valid");
        let artifact_bytes = b"synthetic test artifact - not engineering evidence";
        fs::write(root.join("model.bin"), artifact_bytes).expect("write test artifact");
        let digest = sha256_hex(artifact_bytes);
        let tool_bytes = b"synthetic verifier tool - not a solver";
        fs::write(root.join("tool.bin"), tool_bytes).expect("write tool artifact");
        let tool_digest = sha256_hex(tool_bytes);

        manifest["evidence_artifacts"][0]["artifact_id"] = json!("model-fixture");
        manifest["model"]["artifact_ids"] = json!(["model-fixture"]);
        manifest["jurisdiction"]["resolution_status"] = json!("confirmed");
        manifest["rule_pack"]["status"] = json!("reviewed_for_use");
        manifest["evidence_artifacts"][0]["sha256"] = json!(digest);
        manifest["evidence_artifacts"][0]["uri"] = json!("model.bin");
        manifest["evidence_artifacts"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "artifact_id": "tool-executable",
                "media_type": "application/octet-stream",
                "sha256": tool_digest,
                "role": "other",
                "uri": "tool.bin",
                "retention_class": "test-only"
            }));
        manifest["toolchain"] = json!([{
            "tool_id": "synthetic-tool",
            "name": "Synthetic Test Tool",
            "version": "0.0-test",
            "executable_or_container_sha256": tool_digest,
            "configuration_artifact_sha256": null,
            "runtime": "unit-test only"
        }]);
        manifest["claims"][0]["status"] = json!(STATUS_PASS);
        manifest["claims"][0]["input_artifact_sha256"] = json!([digest]);
        manifest["claims"][0]["output_artifact_sha256"] = json!([digest]);
        manifest["claims"][0]["tool_id"] = json!("synthetic-tool");
        manifest["claims"][0]["validity_domain"] = json!("test-only");
        manifest["claims"][0]["acceptance_criteria"] = json!("test-only");
        manifest["claims"][0]["measured_result"] = json!("test-only");
        manifest["overall_status"] = json!(STATUS_PASS);

        let expected = ExpectedClaimsFile {
            schema_version: manifest["schema_version"].as_str().unwrap().to_string(),
            schema_sha256: sha256_hex(EMBEDDED_SCHEMA.as_bytes()),
            verifier_version: env!("CARGO_PKG_VERSION").to_string(),
            verifier_binary_sha256: current_verifier_binary_sha256()
                .expect("test executable digest available"),
            case_id: manifest["case_id"].as_str().unwrap().to_string(),
            jurisdiction: manifest["jurisdiction"].clone(),
            model: manifest["model"].clone(),
            model_artifacts: vec![ExpectedModelArtifact {
                artifact_id: manifest["evidence_artifacts"][0]["artifact_id"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                sha256: manifest["evidence_artifacts"][0]["sha256"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            }],
            rule_pack: manifest["rule_pack"].clone(),
            toolchain: manifest["toolchain"].clone(),
            mandatory_claims: vec![ExpectedClaimContract {
                claim_id: "example-structural-claim".to_string(),
                claim_type: "structural_analysis".to_string(),
                statement: manifest["claims"][0]["statement"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                subject_element_ids: Vec::new(),
                requirement: Value::Null,
                tool_id: manifest["claims"][0]["tool_id"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                input_artifact_sha256: vec![
                    manifest["claims"][0]["input_artifact_sha256"][0]
                        .as_str()
                        .unwrap()
                        .to_string(),
                ],
                output_artifact_sha256: vec![
                    manifest["claims"][0]["output_artifact_sha256"][0]
                        .as_str()
                        .unwrap()
                        .to_string(),
                ],
                validity_domain: "test-only".to_string(),
                acceptance_criteria: "test-only".to_string(),
            }],
        };
        (schema, manifest, root, expected)
    }

    #[test]
    fn duplicate_cli_options_are_rejected() {
        let cases = vec![
            vec![
                "manifest.json",
                "--expected-claims",
                "one.json",
                "--expected-claims",
                "two.json",
            ],
            vec![
                "manifest.json",
                "--expected-claims-sha256",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "--expected-claims-sha256",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ],
            vec![
                "manifest.json",
                "--artifact-root",
                "one",
                "--artifact-root",
                "two",
            ],
            vec!["manifest.json", "--require-pass", "--require-pass"],
        ];

        for case in cases {
            let args = case.into_iter().map(ToString::to_string).collect();
            let error = run(args).expect_err("duplicate option must be rejected");
            assert!(error.contains("may only be supplied once"), "{error}");
        }
    }

    #[test]
    fn usage_mentions_the_out_of_band_contract_digest() {
        let usage = usage();
        assert!(usage.contains("--expected-claims-sha256"));
        assert!(usage.contains("pinned-out-of-band"));
    }

    #[test]
    fn embedded_schema_matches_checked_in_schema() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/built-environment");
        let disk_schema = fs::read_to_string(root.join("housing-evidence-manifest.schema.json"))
            .expect("checked-in schema exists");
        assert_eq!(EMBEDDED_SCHEMA, disk_schema);
    }

    #[test]
    fn schema_is_valid_json_schema_2020_12() {
        let (schema, _) = fixture_values();
        assert!(draft202012::meta::validate(&schema).is_ok());
    }

    #[test]
    fn evaluated_fail_claim_requires_result_artifact_digests() {
        let (schema, mut manifest, root, _) = set_up_pass_manifest();
        manifest["claims"][0]["status"] = json!(STATUS_FAIL);
        manifest["overall_status"] = json!(STATUS_FAIL);
        manifest["claims"][0]["output_artifact_sha256"] = json!([]);

        let result = evaluate_manifest(&schema, &manifest, None, false, Some(&root));
        assert!(!result.schema_valid);
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("schema validation"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn evaluated_fail_claim_requires_non_empty_measured_result() {
        let (schema, mut manifest, root, _) = set_up_pass_manifest();
        manifest["claims"][0]["status"] = json!(STATUS_FAIL);
        manifest["overall_status"] = json!(STATUS_FAIL);
        manifest["claims"][0]["measured_result"] = json!("");

        let result = evaluate_manifest(&schema, &manifest, None, false, Some(&root));
        assert!(!result.schema_valid);
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("schema validation"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn evaluated_fail_claim_rejects_null_measured_result() {
        let (schema, mut manifest, root, _) = set_up_pass_manifest();
        manifest["claims"][0]["status"] = json!(STATUS_FAIL);
        manifest["overall_status"] = json!(STATUS_FAIL);
        manifest["claims"][0]["measured_result"] = Value::Null;

        let result = evaluate_manifest(&schema, &manifest, None, false, Some(&root));
        assert!(!result.schema_valid);
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("schema validation"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn synthetic_not_evaluated_fixture_is_valid_but_not_a_pass() {
        let (schema, manifest) = fixture_values();
        let result = evaluate_manifest(&schema, &manifest, None, false, None);
        assert!(result.schema_valid, "{:?}", result.issues);
        assert_eq!(result.effective_status, STATUS_NOT_EVALUATED);
        assert_eq!(result.artifact_digests_verified, 0);
    }

    #[test]
    fn cli_pass_requires_pinned_requirements_and_emits_provenance_digests() {
        let (_schema, manifest, root, expected) = set_up_pass_manifest();

        let manifest_path = root.join("manifest.json");
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).expect("serialize manifest");
        fs::write(&manifest_path, &manifest_bytes).expect("write manifest");

        let requirements_path = root.join("requirements.json");
        let requirements_bytes =
            serde_json::to_vec_pretty(&expected).expect("serialize requirements");
        fs::write(&requirements_path, &requirements_bytes).expect("write requirements");
        let expected_digest = sha256_hex(&requirements_bytes);

        let args = vec![
            manifest_path.display().to_string(),
            "--expected-claims".to_string(),
            requirements_path.display().to_string(),
            "--expected-claims-sha256".to_string(),
            expected_digest.clone(),
            "--artifact-root".to_string(),
            root.display().to_string(),
            "--require-pass".to_string(),
        ];
        let (report, require_pass) = run(args).expect("verify pinned pass");
        assert!(require_pass);
        assert_eq!(report.effective_status, STATUS_PASS, "{:?}", report.issues);
        assert_eq!(report.declared_status.as_deref(), Some(STATUS_PASS));
        assert_eq!(
            report.evidence_integrity_status,
            STATUS_EVIDENCE_INTEGRITY_VERIFIED
        );
        assert!(report.schema_sha256.is_some());
        assert_eq!(
            report.schema_sha256.as_deref(),
            Some(expected.schema_sha256.as_str())
        );
        assert_eq!(
            report.verifier_binary_sha256.as_deref(),
            Some(expected.verifier_binary_sha256.as_str())
        );
        let manifest_digest = sha256_hex(&manifest_bytes);
        assert_eq!(
            report.manifest_sha256.as_deref(),
            Some(manifest_digest.as_str())
        );
        assert_eq!(
            report.expected_claims_sha256.as_deref(),
            Some(expected_digest.as_str())
        );
        assert!(report.expected_claims_digest_pinned);
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn mandatory_failure_cannot_be_hidden_by_not_evaluated_aggregate() {
        let (schema, mut manifest, root, _) = set_up_pass_manifest();
        manifest["claims"][0]["status"] = json!(STATUS_FAIL);
        manifest["overall_status"] = json!(STATUS_NOT_EVALUATED);

        let result = evaluate_manifest(&schema, &manifest, None, false, Some(&root));
        assert!(result.schema_valid, "{:?}", result.issues);
        assert_eq!(result.effective_status, STATUS_FAIL);
        assert!(result.issues.iter().any(|issue| {
            issue.contains("understates mandatory claim state")
                && issue.contains("FAIL")
                && issue.contains("example-structural-claim")
        }));
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn weaker_aggregate_cannot_hide_mandatory_blocked_state() {
        let (schema, mut manifest, root, _) = set_up_pass_manifest();
        manifest["claims"][0]["status"] = json!(STATUS_BLOCKED);
        manifest["overall_status"] = json!(STATUS_NOT_EVALUATED);

        let result = evaluate_manifest(&schema, &manifest, None, false, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert_eq!(
            result.evidence_integrity_status,
            STATUS_EVIDENCE_INTEGRITY_NOT_VERIFIED
        );
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("understates mandatory claim state") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn pass_is_insufficient_without_independent_expected_claim_set() {
        let (schema, manifest, root, _) = set_up_pass_manifest();
        let result = evaluate_manifest(&schema, &manifest, None, false, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert!(!result.issues.is_empty());
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn pass_is_insufficient_when_expected_claims_digest_is_not_pinned() {
        let (schema, manifest, root, expected) = set_up_pass_manifest();
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), false, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("--expected-claims-sha256"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn valid_shaped_pass_requires_and_verifies_artifact_bytes() {
        let (schema, manifest, root, expected) = set_up_pass_manifest();
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert!(result.schema_valid, "{:?}", result.issues);
        assert_eq!(result.effective_status, STATUS_PASS, "{:?}", result.issues);
        assert_eq!(result.declared_status.as_deref(), Some(STATUS_PASS));
        assert_eq!(
            result.evidence_integrity_status,
            STATUS_EVIDENCE_INTEGRITY_VERIFIED
        );
        assert_eq!(result.artifact_digests_verified, 2);
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn evaluated_claim_rejects_unknown_toolchain_id() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        manifest["claims"][0]["tool_id"] = json!("unregistered-tool");
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("unknown tool_id"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn evaluated_claim_requires_tool_executable_digest() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        manifest["toolchain"][0]
            .as_object_mut()
            .unwrap()
            .remove("executable_or_container_sha256");
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("no executable/container digest"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn mandatory_code_compliance_claim_requires_edition_clause_and_usable_rights() {
        let (schema, mut manifest, root, _) = set_up_pass_manifest();
        manifest["claims"][0]["claim_type"] = json!("code_compliance");
        manifest["claims"][0]["requirement"] = json!({
            "requirement_id": "SYNTHETIC-CLAUSE",
            "source_id": "za-nbr-act",
            "applicability_notes": "test fixture only"
        });
        let digest = manifest["evidence_artifacts"][0]["sha256"]
            .as_str()
            .unwrap()
            .to_string();
        manifest["rule_pack"]["source_references"][0]["source_sha256"] = json!(digest);
        manifest["rule_pack"]["source_references"][0]["rights_status"] = json!("unknown");

        let result = evaluate_manifest(&schema, &manifest, None, false, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("no edition or effective_from date") })
        );
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("no clause_or_location") })
        );
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("unresolved or restricted rights status") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn valid_mandatory_code_compliance_claim_requires_a_pinned_source_snapshot() {
        let (schema, mut manifest, root, mut expected) = set_up_pass_manifest();
        let source_bytes = b"synthetic public legislation excerpt - not real law";
        fs::write(root.join("rule-source.bin"), source_bytes).expect("write source fixture");
        let source_digest = sha256_hex(source_bytes);

        manifest["rule_pack"]["source_references"][0]["source_id"] = json!("synthetic-rule-source");
        manifest["rule_pack"]["source_references"][0]["uri"] =
            json!("https://example.invalid/synthetic-rule-source");
        manifest["rule_pack"]["source_references"][0]["source_type"] = json!("legislation");
        manifest["rule_pack"]["source_references"][0]["edition"] = json!("synthetic-edition-v1");
        manifest["rule_pack"]["source_references"][0]["effective_from"] = json!("2026-01-01");
        manifest["rule_pack"]["source_references"][0]["clause_or_location"] =
            json!("synthetic clause 1.1");
        manifest["rule_pack"]["source_references"][0]["rights_status"] = json!("public");
        manifest["rule_pack"]["source_references"][0]["source_sha256"] = json!(source_digest);
        manifest["evidence_artifacts"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "artifact_id": "rule-source-snapshot",
                "media_type": "application/octet-stream",
                "sha256": source_digest,
                "role": "other",
                "uri": "rule-source.bin",
                "retention_class": "test-only"
            }));
        manifest["claims"][0]["claim_type"] = json!("code_compliance");
        manifest["claims"][0]["requirement"] = json!({
            "requirement_id": "SYNTHETIC-CLAUSE-1.1",
            "source_id": "synthetic-rule-source",
            "applicability_notes": "synthetic test only"
        });
        expected.rule_pack = manifest["rule_pack"].clone();
        expected.mandatory_claims[0].claim_type = "code_compliance".to_string();
        expected.mandatory_claims[0].requirement = manifest["claims"][0]["requirement"].clone();

        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_PASS, "{:?}", result.issues);
        assert_eq!(
            result.evidence_integrity_status,
            STATUS_EVIDENCE_INTEGRITY_VERIFIED
        );
        assert_eq!(result.artifact_digests_verified, 3);
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn code_compliance_claim_requires_a_rule_pack_source() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        manifest["claims"][0]["claim_type"] = json!("code_compliance");
        manifest["claims"][0]["requirement"] = json!({
            "requirement_id": "SYNTHETIC-CLAUSE",
            "source_id": "missing-source",
            "applicability_notes": "test fixture only"
        });
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("source absent from the rule pack"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn missing_mandatory_claim_is_not_a_pass() {
        let (schema, manifest, root, mut expected) = set_up_pass_manifest();
        expected.mandatory_claims.push(ExpectedClaimContract {
            claim_id: "independently-required-fire-claim".to_string(),
            claim_type: "building_physics".to_string(),
            statement: "Synthetic expected fire-performance claim.".to_string(),
            subject_element_ids: Vec::new(),
            requirement: Value::Null,
            tool_id: "synthetic-tool".to_string(),
            input_artifact_sha256: vec![
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            ],
            output_artifact_sha256: vec![
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            ],
            validity_domain: "test-only".to_string(),
            acceptance_criteria: "test-only".to_string(),
        });
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("mandatory claim set"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn a_mandatory_code_compliance_claim_needs_a_hashed_source_artifact() {
        let (schema, mut manifest, root, _) = set_up_pass_manifest();
        manifest["claims"][0]["claim_type"] = json!("code_compliance");
        manifest["claims"][0]["requirement"] = json!({
            "requirement_id": "SYNTHETIC-CLAUSE",
            "source_id": "za-nbr-act",
            "applicability_notes": "test fixture only"
        });
        let result = evaluate_manifest(&schema, &manifest, None, false, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("has no source_sha256"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn provided_rule_source_digests_are_verified_against_local_artifacts() {
        let (schema, mut manifest, root, mut expected) = set_up_pass_manifest();
        let source_bytes = b"synthetic source snapshot - not a real legal standard";
        fs::write(root.join("source.bin"), source_bytes).expect("write source fixture");
        let source_digest = sha256_hex(source_bytes);
        manifest["rule_pack"]["source_references"][0]["source_sha256"] = json!(source_digest);
        manifest["evidence_artifacts"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "artifact_id": "rule-source-snapshot",
                "media_type": "application/octet-stream",
                "sha256": source_digest,
                "role": "other",
                "uri": "source.bin",
                "retention_class": "test-only"
            }));
        expected.rule_pack = manifest["rule_pack"].clone();

        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_PASS, "{:?}", result.issues);
        assert_eq!(result.declared_status.as_deref(), Some(STATUS_PASS));
        assert_eq!(
            result.evidence_integrity_status,
            STATUS_EVIDENCE_INTEGRITY_VERIFIED
        );
        assert_eq!(result.artifact_digests_verified, 3);
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn changed_claim_outputs_cannot_reuse_a_pinned_requirements_contract() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        let substituted_digest = manifest["toolchain"][0]["executable_or_container_sha256"]
            .as_str()
            .unwrap()
            .to_string();
        manifest["claims"][0]["output_artifact_sha256"] = json!([substituted_digest]);

        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert_eq!(
            result.evidence_integrity_status,
            STATUS_EVIDENCE_INTEGRITY_NOT_VERIFIED
        );
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("pinned output artifact digests") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn changed_acceptance_criteria_cannot_reuse_a_pinned_claim_id() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        manifest["claims"][0]["acceptance_criteria"] =
            json!("a weakened criterion that was never independently approved");
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("pinned acceptance criteria") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn changed_claim_inputs_cannot_reuse_a_pinned_requirements_contract() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        let substituted_digest = manifest["toolchain"][0]["executable_or_container_sha256"]
            .as_str()
            .unwrap()
            .to_string();
        manifest["claims"][0]["input_artifact_sha256"] = json!([substituted_digest]);

        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("pinned input artifact digests") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn swapped_model_digest_assignments_are_rejected_even_when_the_set_is_unchanged() {
        let (schema, mut manifest, root, mut expected) = set_up_pass_manifest();
        let first_digest = manifest["evidence_artifacts"][0]["sha256"]
            .as_str()
            .unwrap()
            .to_string();
        let second_bytes = b"second synthetic model artifact - not engineering evidence";
        fs::write(root.join("model-b.bin"), second_bytes).expect("write second model artifact");
        let second_digest = sha256_hex(second_bytes);

        // Keep the model's artifact IDs stable, but validly swap which bytes each ID names.
        manifest["evidence_artifacts"][0]["sha256"] = json!(second_digest.clone());
        manifest["evidence_artifacts"][0]["uri"] = json!("model-b.bin");
        manifest["evidence_artifacts"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "artifact_id": "model-fixture-secondary",
                "media_type": "application/octet-stream",
                "sha256": first_digest.clone(),
                "role": "model",
                "uri": "model.bin",
                "retention_class": "test-only"
            }));
        manifest["model"]["artifact_ids"] = json!(["model-fixture", "model-fixture-secondary"]);

        // The expected contract now includes both IDs but retains the original binding.
        expected.model = manifest["model"].clone();
        expected.model_artifacts = vec![
            ExpectedModelArtifact {
                artifact_id: "model-fixture".to_string(),
                sha256: first_digest,
            },
            ExpectedModelArtifact {
                artifact_id: "model-fixture-secondary".to_string(),
                sha256: second_digest,
            },
        ];

        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("model artifact ID-to-digest mapping") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn changed_model_artifact_bytes_cannot_reuse_a_stable_artifact_id() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        let substituted_digest = manifest["toolchain"][0]["executable_or_container_sha256"]
            .as_str()
            .unwrap()
            .to_string();
        manifest["evidence_artifacts"][0]["sha256"] = json!(substituted_digest.clone());
        manifest["claims"][0]["input_artifact_sha256"] = json!([substituted_digest.clone()]);
        manifest["claims"][0]["output_artifact_sha256"] = json!([substituted_digest]);

        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("SHA-256 mismatch") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn changed_schema_digest_cannot_reuse_a_pinned_requirements_contract() {
        let (schema, manifest, root, mut expected) = set_up_pass_manifest();
        expected.schema_sha256 =
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_string();

        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert_eq!(
            result.evidence_integrity_status,
            STATUS_EVIDENCE_INTEGRITY_NOT_VERIFIED
        );
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("embedded schema digest") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn changed_verifier_executable_cannot_reuse_a_pinned_requirements_contract() {
        let (schema, manifest, root, mut expected) = set_up_pass_manifest();
        expected.verifier_binary_sha256 =
            "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_string();

        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert_eq!(
            result.evidence_integrity_status,
            STATUS_EVIDENCE_INTEGRITY_NOT_VERIFIED
        );
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("verifier executable digest") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn changed_model_revision_cannot_reuse_a_pinned_requirements_contract() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        manifest["model"]["revision_id"] = json!("substituted-model-revision");
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| { issue.contains("model object") })
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn digest_mismatch_blocks_a_declared_pass() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        manifest["evidence_artifacts"][0]["sha256"] =
            json!("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd");
        let digest = manifest["evidence_artifacts"][0]["sha256"]
            .as_str()
            .unwrap()
            .to_string();
        manifest["claims"][0]["input_artifact_sha256"] = json!([digest]);
        manifest["claims"][0]["output_artifact_sha256"] = json!([digest]);
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.contains("SHA-256 mismatch"))
        );
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn artifact_path_traversal_is_rejected() {
        let root = temp_dir("traversal");
        let canonical_root = fs::canonicalize(&root).expect("canonical temp root");
        let result = safe_local_artifact_path(&canonical_root, "../outside.bin");
        assert!(result.is_err());
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[cfg(unix)]
    #[test]
    fn artifact_link_target_must_remain_under_declared_root() {
        use std::os::unix::fs::symlink;

        let root = temp_dir("link-root");
        let target_root = temp_dir("link-target");
        fs::write(target_root.join("target.bin"), b"fixture bytes").expect("write test target");
        symlink(target_root.join("target.bin"), root.join("alias.bin"))
            .expect("create test file link");
        let canonical_root = fs::canonicalize(&root).expect("canonical temp root");

        let result = safe_local_artifact_path(&canonical_root, "alias.bin");
        assert!(matches!(result, Err(error) if error.contains("declared artifact root")));

        fs::remove_dir_all(root).expect("clean root");
        fs::remove_dir_all(target_root).expect("clean target");
    }

    #[test]
    fn schema_cannot_fetch_external_references() {
        let value = json!({"$ref": "https://example.invalid/external.json"});
        assert!(reject_external_refs(&value).is_err());
    }
}
