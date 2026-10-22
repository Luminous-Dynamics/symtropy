use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

pub const DRC_POLICY_SCHEMA: &str = "kicad-drc-routing-policy-v0.1";
pub const DRC_RECEIPT_SCHEMA: &str = "kicad-drc-evidence-receipt-v0.1";
pub const KICAD_DRC_SCHEMA_SUFFIX: &str = "/drc.v1.json";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct KicadDrcRoutingPolicy {
    pub schema: String,
    pub kicad_version: String,
    pub schema_source_url: String,
    pub schema_source_blob_sha1: String,
    pub taxonomy_source_url: String,
    pub taxonomy_source_blob_sha1: String,
    pub routing_violation_types: Vec<String>,
    pub nonrouting_violation_types: Vec<String>,
    pub blocking_severities: Vec<String>,
    pub block_unconnected_items: bool,
    pub block_schematic_parity: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct BoardKicadDrcVerification {
    pub board_id: String,
    pub drc_report_relative_path: String,
    pub evidence: KicadDrcEvidenceReceipt,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct KicadRunArtifactVerificationReceipt {
    pub schema: String,
    pub evaluator_git_head: String,
    pub run_artifact_verification: crate::RunArtifactVerificationReceipt,
    pub policy_sha256: String,
    pub verified_drc_board_count: usize,
    pub boards: Vec<BoardKicadDrcVerification>,
    pub authority: String,
    pub claim_ceiling: String,
}

/// Verify actual output/DRC file bytes, then parse each completed attempt's
/// KiCad DRC JSON and compare derived findings with the normalized run report.
pub fn verify_run_artifacts_with_kicad_drc_policy(
    report_bytes: &[u8],
    manifest_bytes: &[u8],
    run_root: &Path,
    policy_bytes: &[u8],
    evaluator_git_head: &str,
) -> Result<KicadRunArtifactVerificationReceipt, String> {
    verify_run_artifacts_with_kicad_drc_policy_inner(
        report_bytes,
        manifest_bytes,
        run_root,
        policy_bytes,
        evaluator_git_head,
        || {},
    )
}

fn verify_run_artifacts_with_kicad_drc_policy_inner<F>(
    report_bytes: &[u8],
    manifest_bytes: &[u8],
    run_root: &Path,
    policy_bytes: &[u8],
    evaluator_git_head: &str,
    after_initial_hash_verification: F,
) -> Result<KicadRunArtifactVerificationReceipt, String>
where
    F: FnOnce(),
{
    let validated = crate::evaluate_report_bytes(report_bytes, manifest_bytes, evaluator_git_head)?;
    let policy_sha256 = sha256_hex(policy_bytes);
    if validated.report.drc_policy_sha256 != policy_sha256 {
        return Err(
            "run report DRC policy SHA-256 does not match the supplied policy bytes".into(),
        );
    }

    let artifact_verification =
        crate::verify_run_artifacts(report_bytes, manifest_bytes, run_root, evaluator_git_head)?;
    let canonical_root = run_root
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize run-artifact root: {error}"))?;
    // Test seam: deliberately model the filesystem changing after the first
    // hash verification but before the policy parser re-reads each artifact.
    // Production always calls the public wrapper with a no-op callback.
    after_initial_hash_verification();
    let mut boards = Vec::new();

    for case in &validated.report.cases {
        if case.execution_status != crate::ExecutionStatus::Completed {
            continue;
        }
        if !case.drc_evaluated {
            return Err(format!(
                "{}: completed run has raw DRC evidence but says drc_evaluated=false",
                case.board_id
            ));
        }
        let normalized_unconnected = case.unconnected_item_count.ok_or_else(|| {
            format!(
                "{}: completed run lacks a normalized unconnected_item_count",
                case.board_id
            )
        })?;
        let output_relative_path = case
            .output_board_relative_path
            .as_deref()
            .ok_or_else(|| format!("{}: output-board path is missing", case.board_id))?;
        let expected_source = Path::new(output_relative_path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("{}: output-board path has no valid filename", case.board_id))?;
        let drc_relative_path = case
            .drc_report_relative_path
            .as_deref()
            .ok_or_else(|| format!("{}: DRC-report path is missing", case.board_id))?;
        let expected_output_sha = case
            .output_board_sha256
            .as_deref()
            .ok_or_else(|| format!("{}: output-board SHA-256 is missing", case.board_id))?;
        let output_path = canonical_root
            .join(output_relative_path)
            .canonicalize()
            .map_err(|error| {
                format!(
                    "{}: cannot re-resolve output board {}: {error}",
                    case.board_id, output_relative_path
                )
            })?;
        if !output_path.starts_with(&canonical_root) || !output_path.is_file() {
            return Err(format!(
                "{}: output board escaped the verified artifact root or is not a regular file",
                case.board_id
            ));
        }
        let output_bytes = fs::read(&output_path).map_err(|error| {
            format!("{}: cannot re-read output board bytes: {error}", case.board_id)
        })?;
        if sha256_hex(&output_bytes) != expected_output_sha {
            return Err(format!(
                "{}: output board changed after run-artifact verification",
                case.board_id
            ));
        }

        let expected_drc_sha = case
            .drc_report_sha256
            .as_deref()
            .ok_or_else(|| format!("{}: DRC-report SHA-256 is missing", case.board_id))?;
        let drc_path = canonical_root
            .join(drc_relative_path)
            .canonicalize()
            .map_err(|error| {
                format!(
                    "{}: cannot re-resolve DRC report {}: {error}",
                    case.board_id, drc_relative_path
                )
            })?;
        if !drc_path.starts_with(&canonical_root) || !drc_path.is_file() {
            return Err(format!(
                "{}: DRC report escaped the verified artifact root or is not a regular file",
                case.board_id
            ));
        }
        let drc_bytes = fs::read(&drc_path).map_err(|error| {
            format!(
                "{}: cannot re-read DRC report {}: {error}",
                case.board_id, drc_relative_path
            )
        })?;
        if sha256_hex(&drc_bytes) != expected_drc_sha {
            return Err(format!(
                "{}: DRC report changed after run-artifact verification",
                case.board_id
            ));
        }

        let evidence = evaluate_kicad_drc_report_bytes(
            &drc_bytes,
            policy_bytes,
            expected_source,
            &validated.report.drc_engine.version,
        )
        .map_err(|error| format!("{}: raw KiCad DRC report rejected: {error}", case.board_id))?;

        if normalized_unconnected != evidence.unconnected_item_count as u32 {
            return Err(format!(
                "{}: normalized unconnected-item count {} disagrees with raw KiCad DRC count {}",
                case.board_id, normalized_unconnected, evidence.unconnected_item_count
            ));
        }
        let mut normalized_routing_types = case.routing_drc_violation_types.clone();
        normalized_routing_types.sort();
        if normalized_routing_types != evidence.routing_blocking_violation_types {
            return Err(format!(
                "{}: normalized routing DRC types {:?} disagree with raw KiCad DRC types {:?}",
                case.board_id, normalized_routing_types, evidence.routing_blocking_violation_types
            ));
        }
        if case.upstream_nonrouting_drc_after as usize
            != evidence.nonrouting_blocking_violation_types.len()
        {
            return Err(format!(
                "{}: normalized non-routing DRC count {} disagrees with raw KiCad DRC count {}",
                case.board_id,
                case.upstream_nonrouting_drc_after,
                evidence.nonrouting_blocking_violation_types.len()
            ));
        }
        let evaluated_case = validated
            .receipt
            .cases
            .iter()
            .find(|evaluated| evaluated.board_id == case.board_id)
            .ok_or_else(|| format!("{}: evaluated case is missing", case.board_id))?;
        if evaluated_case.valid_route_completion && !evidence.route_drc_pass {
            return Err(format!(
                "{}: normalized report claims valid completion but raw KiCad DRC policy fails",
                case.board_id
            ));
        }

        boards.push(BoardKicadDrcVerification {
            board_id: case.board_id.clone(),
            drc_report_relative_path: drc_relative_path.to_string(),
            evidence,
        });
    }

    boards.sort_by(|left, right| left.board_id.cmp(&right.board_id));
    Ok(KicadRunArtifactVerificationReceipt {
        schema: "pcb-route-kicad-run-artifact-verification-v0.1".into(),
        evaluator_git_head: evaluator_git_head.to_ascii_lowercase(),
        run_artifact_verification: artifact_verification,
        policy_sha256,
        verified_drc_board_count: boards.len(),
        boards,
        authority: "raw_kicad_drc_json_and_output_byte_verification_under_pinned_policy_only".into(),
        claim_ceiling: "This receipt verifies that output/DRC file bytes match their declared hashes and that normalized routing findings/unconnected counts match the raw KiCad JSON under the policy digest. It does not prove policy sufficiency, schematic intent, electrical integrity, manufacturing quality, physical conformity, safety or production readiness.".into(),
    })
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KicadDrcReport {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub source: String,
    pub date: String,
    pub kicad_version: String,
    pub violations: Vec<KicadViolation>,
    pub unconnected_items: Vec<KicadViolation>,
    pub schematic_parity: Vec<KicadViolation>,
    pub coordinate_units: String,
    #[serde(default)]
    pub included_severities: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KicadViolation {
    #[serde(rename = "type")]
    pub violation_type: String,
    pub description: String,
    pub severity: String,
    pub items: Vec<KicadAffectedItem>,
    #[serde(default)]
    pub excluded: bool,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KicadAffectedItem {
    pub uuid: String,
    pub description: String,
    pub pos: KicadCoordinate,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct KicadCoordinate {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct KicadDrcEvidenceReceipt {
    pub schema: String,
    pub raw_report_sha256: String,
    pub policy_sha256: String,
    pub source: String,
    pub kicad_version: String,
    pub coordinate_units: String,
    pub violations_total: usize,
    pub violations_by_severity: BTreeMap<String, usize>,
    pub violations_by_type: BTreeMap<String, usize>,
    pub routing_blocking_violation_types: Vec<String>,
    pub nonrouting_blocking_violation_types: Vec<String>,
    pub unconnected_item_count: usize,
    pub schematic_parity_count: usize,
    pub route_drc_pass: bool,
    pub board_drc_clean: bool,
    pub authority: String,
    pub claim_ceiling: String,
}

/// Parse the bytes produced by KiCad's documented DRC JSON schema and derive
/// route-facing counts using a separately pinned policy. Router-provided
/// "success" flags are not inputs to this function.
pub fn evaluate_kicad_drc_report_bytes(
    report_bytes: &[u8],
    policy_bytes: &[u8],
    expected_source: &str,
    expected_kicad_version: &str,
) -> Result<KicadDrcEvidenceReceipt, String> {
    let report: KicadDrcReport = serde_json::from_slice(report_bytes)
        .map_err(|error| format!("KiCad DRC JSON is invalid or incomplete: {error}"))?;
    let policy: KicadDrcRoutingPolicy = serde_json::from_slice(policy_bytes)
        .map_err(|error| format!("KiCad DRC routing policy JSON invalid: {error}"))?;

    validate_policy(&policy)?;
    if policy.kicad_version != expected_kicad_version {
        return Err(format!(
            "policy KiCad version mismatch: policy pins {:?}, expected {:?}",
            policy.kicad_version, expected_kicad_version
        ));
    }
    if !report.schema.ends_with(KICAD_DRC_SCHEMA_SUFFIX) {
        return Err(format!(
            "unexpected KiCad DRC report schema: {}",
            report.schema
        ));
    }
    if report.source.trim().is_empty() || report.source != expected_source {
        return Err(format!(
            "KiCad DRC source mismatch: expected {expected_source:?}, observed {:?}",
            report.source
        ));
    }
    if report.kicad_version != expected_kicad_version {
        return Err(format!(
            "KiCad DRC version mismatch: expected {expected_kicad_version:?}, observed {:?}",
            report.kicad_version
        ));
    }
    if report.date.trim().is_empty() || !report.date.contains('T') {
        return Err("KiCad DRC report date must be a non-empty date-time string".into());
    }
    if !matches!(report.coordinate_units.as_str(), "mm" | "mils" | "in") {
        return Err(format!(
            "unsupported KiCad DRC coordinate_units: {}",
            report.coordinate_units
        ));
    }
    for severity in &report.included_severities {
        if !matches!(severity.as_str(), "error" | "warning" | "exclusion") {
            return Err(format!("unknown KiCad DRC included severity: {severity}"));
        }
    }
    for required_severity in &policy.blocking_severities {
        if !report.included_severities.contains(required_severity) {
            return Err(format!(
                "KiCad DRC report omits required blocking severity {required_severity:?}"
            ));
        }
    }

    let routing_types: BTreeSet<&str> = policy
        .routing_violation_types
        .iter()
        .map(String::as_str)
        .collect();
    let nonrouting_types: BTreeSet<&str> = policy
        .nonrouting_violation_types
        .iter()
        .map(String::as_str)
        .collect();
    let blocking_severities: BTreeSet<&str> = policy
        .blocking_severities
        .iter()
        .map(String::as_str)
        .collect();

    let mut violations_by_severity = BTreeMap::new();
    let mut violations_by_type = BTreeMap::new();
    let mut routing_blocking_violation_types = Vec::new();
    let mut nonrouting_blocking_violation_types = Vec::new();

    for violation in &report.violations {
        validate_violation(violation)?;
        let kind = violation.violation_type.as_str();
        if !routing_types.contains(kind) && !nonrouting_types.contains(kind) {
            return Err(format!(
                "unclassified KiCad DRC violation type {kind:?}; update the pinned policy explicitly"
            ));
        }
        if routing_types.contains(kind) && nonrouting_types.contains(kind) {
            return Err(format!(
                "KiCad DRC type {kind:?} is classified as both routing and non-routing"
            ));
        }

        *violations_by_severity
            .entry(violation.severity.clone())
            .or_insert(0) += 1;
        *violations_by_type.entry(kind.to_string()).or_insert(0) += 1;

        if blocking_severities.contains(violation.severity.as_str()) {
            if routing_types.contains(kind) {
                routing_blocking_violation_types.push(kind.to_string());
            } else {
                nonrouting_blocking_violation_types.push(kind.to_string());
            }
        }
    }

    for violation in report
        .unconnected_items
        .iter()
        .chain(report.schematic_parity.iter())
    {
        validate_violation(violation)?;
    }

    routing_blocking_violation_types.sort();
    nonrouting_blocking_violation_types.sort();

    let route_drc_pass = routing_blocking_violation_types.is_empty()
        && (!policy.block_unconnected_items || report.unconnected_items.is_empty())
        && (!policy.block_schematic_parity || report.schematic_parity.is_empty());
    let board_drc_clean = report.violations.is_empty()
        && report.unconnected_items.is_empty()
        && report.schematic_parity.is_empty();

    Ok(KicadDrcEvidenceReceipt {
        schema: DRC_RECEIPT_SCHEMA.into(),
        raw_report_sha256: sha256_hex(report_bytes),
        policy_sha256: sha256_hex(policy_bytes),
        source: report.source,
        kicad_version: report.kicad_version,
        coordinate_units: report.coordinate_units,
        violations_total: report.violations.len(),
        violations_by_severity,
        violations_by_type,
        routing_blocking_violation_types,
        nonrouting_blocking_violation_types,
        unconnected_item_count: report.unconnected_items.len(),
        schematic_parity_count: report.schematic_parity.len(),
        route_drc_pass,
        board_drc_clean,
        authority: "native_kicad_drc_json_interpretation_under_pinned_policy_only_no_physical_qualification_or_release_authority".into(),
        claim_ceiling: "The receipt derives counts from the raw KiCad DRC JSON bytes under the policy digest. It does not prove that the chosen policy is sufficient, nor schematic intent, signal/power integrity, manufacturability, physical quality, electrical safety or production readiness.".into(),
    })
}

fn validate_policy(policy: &KicadDrcRoutingPolicy) -> Result<(), String> {
    if policy.schema != DRC_POLICY_SCHEMA {
        return Err(format!(
            "unsupported KiCad DRC policy schema: {}",
            policy.schema
        ));
    }
    if policy.kicad_version.trim().is_empty() {
        return Err("policy kicad_version must not be empty".into());
    }
    for (label, url) in [
        ("schema_source_url", policy.schema_source_url.as_str()),
        ("taxonomy_source_url", policy.taxonomy_source_url.as_str()),
    ] {
        if !url.starts_with("https://") {
            return Err(format!("policy {label} must be an HTTPS URL"));
        }
    }
    for (label, hash) in [
        (
            "schema_source_blob_sha1",
            policy.schema_source_blob_sha1.as_str(),
        ),
        (
            "taxonomy_source_blob_sha1",
            policy.taxonomy_source_blob_sha1.as_str(),
        ),
    ] {
        if hash.len() != 40 || !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "policy {label} must be a 40-character Git blob SHA-1"
            ));
        }
    }
    if policy.routing_violation_types.is_empty() {
        return Err("policy routing_violation_types must not be empty".into());
    }
    if policy.blocking_severities.is_empty() {
        return Err("policy blocking_severities must not be empty".into());
    }
    let mut routing = BTreeSet::new();
    for kind in &policy.routing_violation_types {
        if !valid_rule_type(kind) || !routing.insert(kind.as_str()) {
            return Err(format!("invalid or duplicate routing DRC type: {kind:?}"));
        }
    }
    let mut nonrouting = BTreeSet::new();
    for kind in &policy.nonrouting_violation_types {
        if !valid_rule_type(kind) || !nonrouting.insert(kind.as_str()) {
            return Err(format!(
                "invalid or duplicate non-routing DRC type: {kind:?}"
            ));
        }
        if routing.contains(kind.as_str()) {
            return Err(format!(
                "DRC type {kind:?} is present in both policy categories"
            ));
        }
    }
    let mut severities = BTreeSet::new();
    for severity in &policy.blocking_severities {
        if !matches!(severity.as_str(), "error" | "warning")
            || !severities.insert(severity.as_str())
        {
            return Err(format!(
                "invalid or duplicate blocking severity: {severity:?}"
            ));
        }
    }
    Ok(())
}

fn validate_violation(violation: &KicadViolation) -> Result<(), String> {
    if !valid_rule_type(&violation.violation_type)
        || violation.description.trim().is_empty()
        || !matches!(violation.severity.as_str(), "error" | "warning")
        || violation.items.iter().any(|item| {
            item.uuid.trim().is_empty()
                || item.description.trim().is_empty()
                || !item.pos.x.is_finite()
                || !item.pos.y.is_finite()
        })
    {
        return Err(format!(
            "malformed KiCad DRC violation record of type {:?}",
            violation.violation_type
        ));
    }
    Ok(())
}

fn valid_rule_type(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(result, "{byte:02x}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const EVALUATOR_HEAD: &str = "abababababababababababababababababababab";

    const POLICY: &str = r#"{
      "schema":"kicad-drc-routing-policy-v0.1",
      "kicad_version":"9.0.9",
      "schema_source_url":"https://github.com/KiCad/kicad-source-mirror/blob/9.0.9/resources/schemas/drc.v1.json",
      "schema_source_blob_sha1":"3db8d82fc807c0690fd231eea9aefdb562576ce4",
      "taxonomy_source_url":"https://github.com/KiCad/kicad-source-mirror/blob/9.0.9/pcbnew/drc/drc_item.cpp",
      "taxonomy_source_blob_sha1":"f6822d5faf1ad17f859d6c8eb13eb85c917f1f61",
      "routing_violation_types":["tracks_crossing","shorting_items","clearance"],
      "nonrouting_violation_types":["silk_over_copper","lib_footprint_mismatch"],
      "blocking_severities":["error"],
      "block_unconnected_items":true,
      "block_schematic_parity":true
    }"#;

    fn report(violations: &str, unconnected: &str, parity: &str) -> String {
        format!(
            r#"{{
              "$schema":"https://schemas.kicad.org/drc.v1.json",
              "source":"coupon.kicad_pcb",
              "date":"2026-10-21T10:00:00+00:00",
              "kicad_version":"9.0.9",
              "coordinate_units":"mm",
              "included_severities":["error","warning"],
              "violations":{violations},
              "unconnected_items":{unconnected},
              "schematic_parity":{parity},
              "ignored_checks":[]
            }}"#
        )
    }

    fn violation(kind: &str, severity: &str) -> String {
        format!(
            r#"[{{"type":"{kind}","description":"fixture {kind}","severity":"{severity}","items":[{{"uuid":"11111111-1111-4111-8111-111111111111","description":"track","pos":{{"x":1.0,"y":2.0}}}}]}}]"#
        )
    }

    fn synthetic_drc_report(
        source: &str,
        routing_types: &[String],
        unconnected_count: u32,
        nonrouting_count: u32,
    ) -> Vec<u8> {
        let mut violations = Vec::new();
        let mut next_id = 1u32;
        for kind in routing_types {
            violations.push(serde_json::json!({
                "type": kind,
                "description": format!("synthetic route violation: {kind}"),
                "severity": "error",
                "items": [{
                    "uuid": format!("11111111-1111-4111-8111-{:012x}", next_id),
                    "description": "synthetic track",
                    "pos": {"x": 1.0, "y": 2.0}
                }]
            }));
            next_id += 1;
        }
        for index in 0..nonrouting_count {
            violations.push(serde_json::json!({
                "type": "lib_footprint_mismatch",
                "description": "synthetic non-routing finding",
                "severity": "error",
                "items": [{
                    "uuid": format!("22222222-2222-4222-8222-{:012x}", index + 1),
                    "description": "synthetic footprint",
                    "pos": {"x": 3.0, "y": 4.0}
                }]
            }));
        }
        let unconnected = (0..unconnected_count)
            .map(|index| {
                serde_json::json!({
                    "type": "unconnected_items",
                    "description": "synthetic unconnected item",
                    "severity": "error",
                    "items": [{
                        "uuid": format!("33333333-3333-4333-8333-{:012x}", index + 1),
                        "description": "synthetic pad",
                        "pos": {"x": 5.0, "y": 6.0}
                    }]
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_vec(&serde_json::json!({
            "$schema": "https://schemas.kicad.org/drc.v1.json",
            "source": source,
            "date": "2026-10-21T10:00:00+00:00",
            "kicad_version": "9.0.9",
            "coordinate_units": "mm",
            "included_severities": ["error", "warning"],
            "violations": violations,
            "unconnected_items": unconnected,
            "schematic_parity": [],
            "ignored_checks": []
        }))
        .unwrap()
    }

    #[test]
    fn strict_run_artifact_verifier_reconciles_normalized_metrics_with_raw_drc_json() {
        let policy_bytes = include_bytes!("../fixtures/kicad-drc-routing-policy-v0_1.json");
        let policy_sha256 = sha256_hex(policy_bytes);
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/synthetic-baseline-run-v0_1.json"))
                .unwrap();
        value["dataset_manifest_sha256"] = serde_json::Value::String(sha256_hex(include_bytes!(
            "../fixtures/synthetic-dataset-manifest-v0_1.json"
        )));
        value["drc_policy_sha256"] = serde_json::Value::String(policy_sha256.clone());

        let root =
            std::env::temp_dir().join(format!("pcb-route-kicad-artifacts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("outputs")).unwrap();
        fs::create_dir_all(root.join("drc")).unwrap();

        for case in value["cases"].as_array_mut().unwrap() {
            if case["execution_status"] != "completed" {
                case["output_board_relative_path"] = serde_json::Value::Null;
                case["drc_report_relative_path"] = serde_json::Value::Null;
                continue;
            }
            let board_id = case["board_id"].as_str().unwrap().to_string();
            let output_relative = format!("outputs/{board_id}.kicad_pcb");
            let drc_relative = format!("drc/{board_id}.json");
            let output_bytes = format!("synthetic output board bytes for {board_id}\n");
            let routing_types: Vec<String> =
                serde_json::from_value(case["routing_drc_violation_types"].clone()).unwrap();
            let unconnected = case["unconnected_item_count"].as_u64().unwrap_or(0) as u32;
            let nonrouting = if board_id == "upstream-drc-006" { 3 } else { 0 };
            let drc_bytes = synthetic_drc_report(
                &format!("{board_id}.kicad_pcb"),
                &routing_types,
                unconnected,
                nonrouting,
            );

            fs::write(root.join(&output_relative), output_bytes.as_bytes()).unwrap();
            fs::write(root.join(&drc_relative), &drc_bytes).unwrap();
            case["output_board_sha256"] =
                serde_json::Value::String(sha256_hex(output_bytes.as_bytes()));
            case["drc_report_sha256"] = serde_json::Value::String(sha256_hex(&drc_bytes));
            case["output_board_relative_path"] = serde_json::Value::String(output_relative);
            case["drc_report_relative_path"] = serde_json::Value::String(drc_relative);
        }

        let report_bytes = serde_json::to_vec(&value).unwrap();
        let receipt = verify_run_artifacts_with_kicad_drc_policy(
            &report_bytes,
            include_bytes!("../fixtures/synthetic-dataset-manifest-v0_1.json"),
            &root,
            policy_bytes,
            EVALUATOR_HEAD,
        )
        .expect("raw DRC evidence matches normalized report");
        assert_eq!(receipt.verified_drc_board_count, 4);
        let crossing = receipt
            .boards
            .iter()
            .find(|board| board.board_id == "crossing-003")
            .unwrap();
        assert_eq!(
            crossing.evidence.routing_blocking_violation_types,
            vec!["tracks_crossing", "tracks_crossing"]
        );
        assert!(!crossing.evidence.route_drc_pass);
        let upstream = receipt
            .boards
            .iter()
            .find(|board| board.board_id == "upstream-drc-006")
            .unwrap();
        assert_eq!(
            upstream.evidence.nonrouting_blocking_violation_types.len(),
            3
        );
        assert!(upstream.evidence.route_drc_pass);
        assert!(!upstream.evidence.board_drc_clean);

        value["cases"][2]["routing_drc_violation_types"] = serde_json::json!([]);
        let dishonest_report = serde_json::to_vec(&value).unwrap();
        assert!(verify_run_artifacts_with_kicad_drc_policy(
            &dishonest_report,
            include_bytes!("../fixtures/synthetic-dataset-manifest-v0_1.json"),
            &root,
            policy_bytes,
            EVALUATOR_HEAD,
        )
        .unwrap_err()
        .contains("disagree with raw KiCad DRC types"));

        // Re-run the valid report, mutate a verified DRC file exactly after the
        // initial source/output/DRC hash pass, then ensure the parse-boundary
        // re-hash rejects the changed bytes before any DRC result is issued.
        let race_report = report_bytes.clone();
        let race_root = root.clone();
        let error = verify_run_artifacts_with_kicad_drc_policy_inner(
            &race_report,
            include_bytes!("../fixtures/synthetic-dataset-manifest-v0_1.json"),
            &race_root,
            policy_bytes,
            EVALUATOR_HEAD,
            || {
                fs::write(race_root.join("drc/easy-001.json"), b"changed after first hash").unwrap();
            },
        )
        .unwrap_err();
        assert!(
            error.contains("DRC report changed after run-artifact verification"),
            "{error}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn clean_native_report_has_zero_findings_and_raw_byte_receipt() {
        let bytes = report("[]", "[]", "[]");
        let receipt = evaluate_kicad_drc_report_bytes(
            bytes.as_bytes(),
            POLICY.as_bytes(),
            "coupon.kicad_pcb",
            "9.0.9",
        )
        .unwrap();
        assert!(receipt.route_drc_pass);
        assert!(receipt.board_drc_clean);
        assert_eq!(receipt.violations_total, 0);
        assert_eq!(receipt.unconnected_item_count, 0);
        assert_eq!(receipt.schematic_parity_count, 0);
        assert_eq!(receipt.raw_report_sha256, sha256_hex(bytes.as_bytes()));
        assert_eq!(receipt.policy_sha256, sha256_hex(POLICY.as_bytes()));
    }

    #[test]
    fn derives_routing_and_nonrouting_types_and_does_not_hide_open_items() {
        // Use an explicit report matching KiCad's documented drc.v1.json schema.
        let json = format!(
            r#"{{"$schema":"https://schemas.kicad.org/drc.v1.json","source":"coupon.kicad_pcb","date":"2026-10-21T10:00:00+00:00","kicad_version":"9.0.9","coordinate_units":"mm","included_severities":["error","warning"],"violations":[{{"type":"tracks_crossing","description":"crossing","severity":"error","items":[{{"uuid":"11111111-1111-4111-8111-111111111111","description":"track","pos":{{"x":1,"y":2}}}}]}},{{"type":"silk_over_copper","description":"silk","severity":"warning","items":[{{"uuid":"22222222-2222-4222-8222-222222222222","description":"text","pos":{{"x":3,"y":4}}}}]}}],"unconnected_items":[{{"type":"unconnected_items","description":"open net","severity":"error","items":[{{"uuid":"33333333-3333-4333-8333-333333333333","description":"pad","pos":{{"x":5,"y":6}}}}]}}],"schematic_parity":[]}}"#
        );
        let receipt = evaluate_kicad_drc_report_bytes(
            json.as_bytes(),
            POLICY.as_bytes(),
            "coupon.kicad_pcb",
            "9.0.9",
        )
        .unwrap();
        assert!(!receipt.route_drc_pass);
        assert!(!receipt.board_drc_clean);
        assert_eq!(
            receipt.routing_blocking_violation_types,
            vec!["tracks_crossing"]
        );
        assert_eq!(
            receipt.nonrouting_blocking_violation_types,
            Vec::<String>::new()
        );
        assert_eq!(receipt.unconnected_item_count, 1);
    }

    #[test]
    fn refuses_reports_that_omit_a_policy_required_severity() {
        let raw = report("[]", "[]", "[]").replace(
            r#""included_severities":["error","warning"]"#,
            r#""included_severities":["error"]"#,
        );
        assert!(evaluate_kicad_drc_report_bytes(
            raw.as_bytes(),
            POLICY.as_bytes(),
            "coupon.kicad_pcb",
            "9.0.9",
        )
        .unwrap_err()
        .contains("omits required blocking severity"));
    }

    #[test]
    fn rejects_unknown_types_source_mismatch_version_mismatch_and_invalid_json() {
        let unknown = report(&violation("future_unknown_rule", "error"), "[]", "[]");
        assert!(evaluate_kicad_drc_report_bytes(
            unknown.as_bytes(),
            POLICY.as_bytes(),
            "coupon.kicad_pcb",
            "9.0.9"
        )
        .unwrap_err()
        .contains("unclassified"));

        let clean = report("[]", "[]", "[]");
        assert!(evaluate_kicad_drc_report_bytes(
            clean.as_bytes(),
            POLICY.as_bytes(),
            "wrong-board.kicad_pcb",
            "9.0.9"
        )
        .unwrap_err()
        .contains("source mismatch"));
        assert!(evaluate_kicad_drc_report_bytes(
            clean.as_bytes(),
            POLICY.as_bytes(),
            "coupon.kicad_pcb",
            "9.0.8"
        )
        .unwrap_err()
        .contains("version mismatch"));
        assert!(evaluate_kicad_drc_report_bytes(
            b"{broken",
            POLICY.as_bytes(),
            "coupon.kicad_pcb",
            "9.0.9"
        )
        .unwrap_err()
        .contains("invalid or incomplete"));
    }

    #[test]
    fn pinned_kicad_policy_fixtures_have_canonical_git_blob_ids() {
        let policies = [
            (
                include_bytes!("../fixtures/kicad-drc-routing-policy-9_0_8-v0_1.json").as_slice(),
                "9.0.8",
            ),
            (
                include_bytes!("../fixtures/kicad-drc-routing-policy-v0_1.json").as_slice(),
                "9.0.9",
            ),
        ];
        for (bytes, version) in policies {
            let policy: KicadDrcRoutingPolicy =
                serde_json::from_slice(bytes).expect("version-pinned policy JSON");
            assert_eq!(policy.kicad_version, version);
            assert_eq!(
                policy.schema_source_blob_sha1,
                "3db8d82fc807c0690fd231eea9aefdb562576ce4"
            );
            assert_eq!(
                policy.taxonomy_source_blob_sha1,
                "f6822d5faf1ad17f859d6c8eb13eb85c917f1f61"
            );
            assert_eq!(policy.schema_source_blob_sha1.len(), 40);
            assert_eq!(policy.taxonomy_source_blob_sha1.len(), 40);
            assert!(
                policy.schema_source_url.contains(&format!("/{version}/resources/schemas/drc.v1.json"))
            );
            assert!(
                policy.taxonomy_source_url.contains(&format!("/{version}/pcbnew/drc/drc_item.cpp"))
            );
        }
    }

}
