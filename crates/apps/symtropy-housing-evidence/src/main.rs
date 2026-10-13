// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-first housing manifest verifier.
//!
//! This tool verifies document shape, manifest-level invariants, expected
//! mandatory-claim coverage, and SHA-256 digests for referenced local files.
//! It does not interpret building codes, validate engineering calculations,
//! prove physical truth, or grant professional/statutory approval.
//!
//! Usage:
//!   housing-evidence-verify <manifest.json>
//!       [--expected-claims <requirements.json>]
//!       [--artifact-root <directory>]
//!       [--require-pass]
//!
//! The expected-claims file is JSON:
//!   { "mandatory_claim_ids": ["stable-claim-id", "..."] }
//!
//! Artifact URIs used by a PASS manifest must be safe, relative paths beneath
//! --artifact-root. Remote retrieval is intentionally not supported.

use jsonschema::draft202012;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs,
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

const STATUS_PASS: &str = "PASS";
const STATUS_FAIL: &str = "FAIL";
const EMBEDDED_SCHEMA: &str = include_str!("../../../../docs/built-environment/housing-evidence-manifest.schema.json");
const STATUS_NOT_EVALUATED: &str = "NOT_EVALUATED";
const STATUS_BLOCKED: &str = "BLOCKED";
const STATUS_INSUFFICIENT_EVIDENCE: &str = "INSUFFICIENT_EVIDENCE";

#[derive(Debug, Serialize)]
struct VerificationReport {
    verifier_version: String,
    schema_sha256: Option<String>,
    manifest_sha256: Option<String>,
    expected_claims_sha256: Option<String>,
    expected_claims_digest_pinned: bool,
    schema_valid: bool,
    case_id: Option<String>,
    declared_status: Option<String>,
    effective_status: String,
    artifact_digests_verified: usize,
    issues: Vec<String>,
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
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
                && report.effective_status != STATUS_PASS
            {
                return ExitCode::from(2);
            }
            if require_pass && report.effective_status != STATUS_PASS {
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
                index += 1;
                let value = args.get(index).ok_or_else(usage)?;
                expected_claims_path = Some(PathBuf::from(value));
            }
            "--expected-claims-sha256" => {
                index += 1;
                let value = args.get(index).ok_or_else(usage)?;
                if !is_sha256(value) {
                    return Err("--expected-claims-sha256 must be 64 lowercase hexadecimal characters".to_string());
                }
                expected_claims_sha256 = Some(value.to_string());
            }
            "--artifact-root" => {
                index += 1;
                let value = args.get(index).ok_or_else(usage)?;
                artifact_root = Some(PathBuf::from(value));
            }
            "--require-pass" => require_pass = true,
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

    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    let manifest: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", manifest_path.display()))?;
    let manifest_sha256 = sha256_hex(&manifest_bytes);

    let expected_claim_data = expected_claims_path
        .as_deref()
        .map(read_expected_claim_ids)
        .transpose()?;
    let (expected_claim_ids, expected_claims_actual_sha256) = match expected_claim_data {
        Some((ids, digest)) => (Some(ids), Some(digest)),
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
        expected_claim_ids.as_ref(),
        expected_claims_pinned,
        artifact_root.as_deref(),
    );
    report.schema_sha256 = Some(sha256_hex(EMBEDDED_SCHEMA.as_bytes()));
    report.manifest_sha256 = Some(manifest_sha256);
    report.expected_claims_sha256 = expected_claims_actual_sha256;
    report.expected_claims_digest_pinned = expected_claims_pinned;
    Ok((report, require_pass))
}

fn usage() -> String {
    "Usage: housing-evidence-verify <manifest.json> \\
     [--expected-claims <requirements.json>] [--artifact-root <directory>] [--require-pass]"
        .to_string()
}

fn read_expected_claim_ids(path: &Path) -> Result<(BTreeSet<String>, String), String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    let ids = value
        .get("mandatory_claim_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            format!(
                "{} must contain a mandatory_claim_ids array",
                path.display()
            )
        })?;

    if ids.is_empty() {
        return Err("mandatory_claim_ids must not be empty".to_string());
    }

    let mut result = BTreeSet::new();
    for id in ids {
        let id = id
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "mandatory_claim_ids entries must be non-empty strings".to_string())?;
        if !result.insert(id.to_string()) {
            return Err(format!("duplicate expected mandatory claim ID: {id}"));
        }
    }
    Ok((result, sha256_hex(&bytes)))
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
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                if reference != "#" && !reference.starts_with("#/") {
                    return Err(format!(
                        "external JSON Schema reference is disabled: {reference}"
                    ));
                }
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
    expected_claim_ids: Option<&BTreeSet<String>>,
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
        artifacts_by_digest.entry(digest).or_default().push(artifact);
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
        if let Some(id) = source["source_id"].as_str() {
            if !source_ids.insert(id.to_string()) {
                issues.push(format!("duplicate source_id in rule pack: {id}"));
            }
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

        if claim["status"].as_str().is_some_and(|status| status == STATUS_PASS || status == STATUS_FAIL) {
            let Some(tool_id) = claim["tool_id"].as_str() else {
                issues.push(format!("evaluated claim {id} has no tool_id"));
                continue;
            };
            let Some(tool) = tools_by_id.get(tool_id) else {
                issues.push(format!("evaluated claim {id} references unknown tool_id: {tool_id}"));
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
            {
                if !artifacts_by_digest.contains_key(config_digest) {
                    issues.push(format!(
                        "tool {tool_id} configuration digest is absent from evidence_artifacts: {config_digest}"
                    ));
                }
            }
        }

        if claim["claim_type"].as_str() == Some("code_compliance") {
            let source_id = claim["requirement"]["source_id"].as_str();
            match source_id {
                Some(source_id) if source_ids.contains(source_id) => {}
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
    {
        if !artifacts_by_digest.contains_key(digest) {
            issues.push(format!(
                "unit_mapping_artifact_sha256 is absent from evidence_artifacts: {digest}"
            ));
        }
    }

    if !issues.is_empty() {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_BLOCKED,
            0,
            issues,
        );
    }

    // A mandatory failure must never be hidden by a more ambiguous aggregate.
    // The manifest is not trusted to choose a weaker top-level status.
    let failed_mandatory_ids: Vec<String> = claims
        .iter()
        .filter(|claim| claim["mandatory"].as_bool() == Some(true))
        .filter(|claim| claim["status"].as_str() == Some(STATUS_FAIL))
        .filter_map(|claim| claim["claim_id"].as_str().map(ToString::to_string))
        .collect();

    if !failed_mandatory_ids.is_empty()
        && declared_status.as_deref() != Some(STATUS_FAIL)
    {
        let issue = format!(
            "declared aggregate status {:?} conflicts with failed mandatory claim(s) {:?}; FAIL takes precedence",
            declared_status.as_deref(),
            failed_mandatory_ids
        );
        return report(
            true,
            case_id,
            declared_status,
            STATUS_FAIL,
            0,
            vec![issue],
        );
    }

    if declared_status.as_deref() != Some(STATUS_PASS) {
        return report(
            true,
            case_id,
            declared_status.clone(),
            declared_status.as_deref().unwrap_or(STATUS_BLOCKED),
            0,
            Vec::new(),
        );
    }

    let Some(expected_claim_ids) = expected_claim_ids else {
        return report(
            true,
            case_id,
            declared_status,
            STATUS_INSUFFICIENT_EVIDENCE,
            0,
            vec![
                "overall PASS requires an independent --expected-claims file; manifest self-description cannot prove requirement-set completeness"
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

    let actual_mandatory_ids: BTreeSet<String> = claims
        .iter()
        .filter(|claim| claim["mandatory"].as_bool() == Some(true))
        .filter_map(|claim| claim["claim_id"].as_str().map(ToString::to_string))
        .collect();

    if &actual_mandatory_ids != expected_claim_ids {
        let missing: Vec<_> = expected_claim_ids
            .difference(&actual_mandatory_ids)
            .cloned()
            .collect();
        let unexpected: Vec<_> = actual_mandatory_ids
            .difference(expected_claim_ids)
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
    match verify_local_artifacts(&referenced_digests, &artifacts_by_digest, artifact_root) {
        Ok(verified_count) => report(
            true,
            case_id,
            declared_status,
            STATUS_PASS,
            verified_count,
            Vec::new(),
        ),
        Err(mut artifact_issues) => {
            issues.append(&mut artifact_issues);
            report(
                true,
                case_id,
                declared_status,
                STATUS_BLOCKED,
                0,
                issues,
            )
        }
    }
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

    for tool in manifest["toolchain"].as_array().into_iter().flatten() {
        for field in ["executable_or_container_sha256", "configuration_artifact_sha256"] {
            if let Some(digest) = tool.get(field).and_then(Value::as_str) {
                result.insert(digest.to_string());
            }
        }
    }

    for claim in manifest["claims"].as_array().into_iter().flatten() {
        for field in ["input_artifact_sha256", "output_artifact_sha256"] {
            for digest in claim[field].as_array().into_iter().flatten().filter_map(Value::as_str) {
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
            issues.push(format!("referenced digest has no artifact record: {digest}"));
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
        return Err(format!(
            "absolute artifact paths are forbidden: {uri:?}"
        ));
    }

    if relative.components().any(|component| {
        !matches!(component, Component::Normal(_))
    }) {
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

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
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
        manifest_sha256: None,
        expected_claims_sha256: None,
        expected_claims_digest_pinned: false,
        schema_valid,
        case_id,
        declared_status,
        effective_status: effective_status.to_string(),
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

    fn set_up_pass_manifest() -> (Value, Value, PathBuf, BTreeSet<String>) {
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
        manifest["evidence_artifacts"].as_array_mut().unwrap().push(json!({
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

        let expected = BTreeSet::from(["example-structural-claim".to_string()]);
        (schema, manifest, root, expected)
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
    fn synthetic_not_evaluated_fixture_is_valid_but_not_a_pass() {
        let (schema, manifest) = fixture_values();
        let result = evaluate_manifest(&schema, &manifest, None, false, None);
        assert!(result.schema_valid, "{:?}", result.issues);
        assert_eq!(result.effective_status, STATUS_NOT_EVALUATED);
        assert_eq!(result.artifact_digests_verified, 0);
    }

    #[test]
    fn cli_pass_requires_pinned_requirements_and_emits_provenance_digests() {
        let (schema, manifest, root, _) = set_up_pass_manifest();

        let manifest_path = root.join("manifest.json");
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).expect("serialize manifest");
        fs::write(&manifest_path, &manifest_bytes).expect("write manifest");

        let requirements_path = root.join("requirements.json");
        let requirements_bytes = serde_json::to_vec_pretty(&json!({
            "mandatory_claim_ids": ["example-structural-claim"]
        }))
        .expect("serialize requirements");
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
        assert!(report.schema_sha256.is_some());
        let manifest_digest = sha256_hex(&manifest_bytes);
        assert_eq!(report.manifest_sha256.as_deref(), Some(manifest_digest.as_str()));
        assert_eq!(report.expected_claims_sha256.as_deref(), Some(expected_digest.as_str()));
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
            issue.contains("FAIL takes precedence")
                && issue.contains("example-structural-claim")
        }));
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
        assert!(result.issues.iter().any(|issue| issue.contains("--expected-claims-sha256")));
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn valid_shaped_pass_requires_and_verifies_artifact_bytes() {
        let (schema, manifest, root, expected) = set_up_pass_manifest();
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert!(result.schema_valid, "{:?}", result.issues);
        assert_eq!(result.effective_status, STATUS_PASS, "{:?}", result.issues);
        assert_eq!(result.artifact_digests_verified, 2);
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn evaluated_claim_rejects_unknown_toolchain_id() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        manifest["claims"][0]["tool_id"] = json!("unregistered-tool");
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(result.issues.iter().any(|issue| issue.contains("unknown tool_id")));
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn evaluated_claim_requires_tool_executable_digest() {
        let (schema, mut manifest, root, expected) = set_up_pass_manifest();
        manifest["toolchain"][0].as_object_mut().unwrap().remove("executable_or_container_sha256");
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_BLOCKED);
        assert!(result.issues.iter().any(|issue| issue.contains("no executable/container digest")));
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
        assert!(result.issues.iter().any(|issue| issue.contains("source absent from the rule pack")));
        fs::remove_dir_all(root).expect("clean temp dir");
    }

    #[test]
    fn missing_mandatory_claim_is_not_a_pass() {
        let (schema, manifest, root, _) = set_up_pass_manifest();
        let expected = BTreeSet::from([
            "example-structural-claim".to_string(),
            "independently-required-fire-claim".to_string(),
        ]);
        let result = evaluate_manifest(&schema, &manifest, Some(&expected), true, Some(&root));
        assert_eq!(result.effective_status, STATUS_INSUFFICIENT_EVIDENCE);
        assert!(result.issues.iter().any(|issue| issue.contains("mandatory claim set")));
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
        assert!(result.issues.iter().any(|issue| issue.contains("SHA-256 mismatch")));
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

    #[test]
    fn schema_cannot_fetch_external_references() {
        let value = json!({"$ref": "https://example.invalid/external.json"});
        assert!(reject_external_refs(&value).is_err());
    }
}
