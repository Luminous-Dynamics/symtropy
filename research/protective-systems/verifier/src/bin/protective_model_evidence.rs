//! Evidence-envelope integrity gate for external resilience model adapters.
//!
//! This validates metadata, dataset lineage, local artifact bytes and rights
//! declarations. A digest proves byte identity only; it does not prove that an
//! upstream model is correct or that a physical system is safe.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::process;

const FIXTURE: &str =
    include_str!("../../../fixtures/protective-model-evidence-envelope-v0.1.json");
const INPUT_BYTES: &[u8] =
    include_bytes!("../../../fixtures/model-evidence-payloads/synthetic-input.json");
const OUTPUT_BYTES: &[u8] =
    include_bytes!("../../../fixtures/model-evidence-payloads/synthetic-output.json");
const SCHEMA: &str = "protective-model-evidence-envelope/0.1";

#[derive(Debug, Serialize)]
struct ArtifactReceipt {
    dataset_id: String,
    relative_path: String,
    byte_length: usize,
    sha256: String,
    integrity_verified: bool,
}

#[derive(Debug, Serialize)]
struct GateReport {
    schema_version: &'static str,
    evidence_class: String,
    provider_id: String,
    analysis_id: String,
    execution_status: String,
    dataset_count: usize,
    input_dataset_count: usize,
    output_dataset_count: usize,
    artifact_receipts: Vec<ArtifactReceipt>,
    envelope_sha256: String,
    disposition: &'static str,
    structural_checks_passed: bool,
    claim_ceiling: &'static str,
}

fn required_text<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .ok_or_else(|| format!("{path}.{key} must be non-empty text"))
}

fn validate_version_field(value: &Value, path: &str) -> Result<bool, String> {
    let status = required_text(value, "status", path)?;
    let raw = value
        .get("value")
        .ok_or_else(|| format!("{path}.value is required"))?;
    match status {
        "KNOWN" => {
            if raw.as_str().map(str::trim).filter(|item| !item.is_empty()).is_none() {
                return Err(format!("{path}.value must be non-empty text when status is KNOWN"));
            }
            Ok(true)
        }
        "UNKNOWN" => {
            if !raw.is_null() {
                return Err(format!("{path}.value must be null when status is UNKNOWN"));
            }
            Ok(false)
        }
        other => Err(format!("{path}.status is unsupported: {other}")),
    }
}

fn artifact_bytes(path: &str, input: &[u8], output: &[u8]) -> Result<&'static str, String> {
    match path {
        "model-evidence-payloads/synthetic-input.json" => Ok("input"),
        "model-evidence-payloads/synthetic-output.json" => Ok("output"),
        other => Err(format!("artifact path is not in the frozen E1 fixture set: {other}")),
    }
}

fn validate_rights(value: &Value, path: &str, evidence_class: &str) -> Result<bool, String> {
    let status = required_text(value, "status", path)?;
    let license = value
        .get("license_id")
        .ok_or_else(|| format!("{path}.license_id is required"))?;
    required_text(value, "permitted_use", path)?;
    match status {
        "SYNTHETIC_OPEN_FIXTURE" if evidence_class == "synthetic" && license.is_null() => Ok(true),
        "LICENSED" if license.as_str().map(str::trim).filter(|item| !item.is_empty()).is_some() => Ok(true),
        "UNKNOWN" | "RESTRICTED" => Ok(false),
        "SYNTHETIC_OPEN_FIXTURE" => Err(format!(
            "{path}: synthetic fixture rights are only valid for evidence_class=synthetic"
        )),
        "LICENSED" => Err(format!("{path}.license_id is required for LICENSED rights")),
        other => Err(format!("{path}.status is unsupported: {other}")),
    }
}

fn validate_envelope(
    envelope_text: &str,
    input_bytes: &[u8],
    output_bytes: &[u8],
) -> Result<GateReport, String> {
    let root: Value = serde_json::from_str(envelope_text)
        .map_err(|error| format!("envelope JSON invalid: {error}"))?;
    if required_text(&root, "schema_version", "root")? != SCHEMA {
        return Err(format!("schema_version must equal {SCHEMA}"));
    }

    let evidence_class = required_text(&root, "evidence_class", "root")?;
    if !["synthetic", "external_observed"].contains(&evidence_class) {
        return Err(format!("unsupported evidence_class: {evidence_class}"));
    }
    let source = root
        .get("source")
        .ok_or_else(|| "source is required".to_owned())?;
    let provider_id = required_text(source, "provider_id", "source")?;
    required_text(source, "platform_name", "source")?;
    let provider_kind = required_text(source, "provider_kind", "source")?;
    if (evidence_class == "synthetic" && provider_kind != "SYNTHETIC_FIXTURE")
        || (evidence_class == "external_observed" && provider_kind != "EXTERNAL_MODEL")
    {
        return Err("provider_kind conflicts with evidence_class".to_owned());
    }
    let version_known = validate_version_field(
        source
            .get("software_version")
            .ok_or_else(|| "source.software_version is required".to_owned())?,
        "source.software_version",
    )?;
    if let Some(uri) = source.get("documentation_uri") {
        if !uri.is_null() && uri.as_str().map(str::trim).filter(|s| !s.is_empty()).is_none() {
            return Err("source.documentation_uri must be a URI string or null".to_owned());
        }
    } else {
        return Err("source.documentation_uri is required".to_owned());
    }

    let analysis = root
        .get("analysis")
        .ok_or_else(|| "analysis is required".to_owned())?;
    let analysis_id = required_text(analysis, "analysis_id", "analysis")?;
    let run_id_known = analysis
        .get("run_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .is_some();
    let execution_status = required_text(analysis, "execution_status", "analysis")?;
    if !["COMPLETED", "FAILED", "NOT_EXECUTED", "UNKNOWN"].contains(&execution_status) {
        return Err(format!("unsupported execution_status: {execution_status}"));
    }
    for key in ["started_at", "completed_at"] {
        let item = analysis
            .get(key)
            .ok_or_else(|| format!("analysis.{key} is required"))?;
        if !item.is_null() && item.as_str().map(str::trim).filter(|s| !s.is_empty()).is_none() {
            return Err(format!("analysis.{key} must be a string or null"));
        }
    }
    if execution_status == "COMPLETED" {
        if !run_id_known {
            return Err("completed analysis requires a non-empty analysis.run_id".to_owned());
        }
        if analysis["completed_at"].is_null() {
            return Err("completed analysis requires analysis.completed_at".to_owned());
        }
    }

    let scope = root
        .get("scope")
        .ok_or_else(|| "scope is required".to_owned())?;
    for key in [
        "context_id",
        "jurisdiction",
        "system_boundary_id",
        "scenario_id",
        "scenario_basis",
        "time_horizon",
    ] {
        required_text(scope, key, "scope")?;
    }
    let scope_crs_known = validate_version_field(
        scope
            .get("coordinate_reference")
            .ok_or_else(|| "scope.coordinate_reference is required".to_owned())?,
        "scope.coordinate_reference",
    )?;

    let datasets = root
        .get("datasets")
        .and_then(Value::as_array)
        .filter(|items| items.len() >= 2)
        .ok_or_else(|| "datasets must contain at least two entries".to_owned())?;
    let mut dataset_ids = BTreeSet::new();
    let mut input_ids = BTreeSet::new();
    let mut output_ids = BTreeSet::new();
    let mut artifact_receipts = Vec::new();
    let mut all_revisions_known = true;
    let mut all_rights_cleared = true;
    let mut all_dataset_crs_known = true;

    for (index, dataset) in datasets.iter().enumerate() {
        let path = format!("datasets[{index}]");
        let id = required_text(dataset, "dataset_id", &path)?;
        if !dataset_ids.insert(id.to_owned()) {
            return Err(format!("{path}: duplicate dataset_id {id}"));
        }
        let direction = required_text(dataset, "direction", &path)?;
        match direction {
            "INPUT" => {
                input_ids.insert(id.to_owned());
            }
            "OUTPUT" => {
                output_ids.insert(id.to_owned());
            }
            other => return Err(format!("{path}.direction is unsupported: {other}")),
        }
        required_text(dataset, "data_type", &path)?;
        required_text(dataset, "semantic_type", &path)?;
        required_text(dataset, "units", &path)?;

        let revision_known = validate_version_field(
            dataset
                .get("revision")
                .ok_or_else(|| format!("{path}.revision is required"))?,
            &format!("{path}.revision"),
        )?;
        all_revisions_known &= revision_known;

        let rights_cleared = validate_rights(
            dataset
                .get("rights")
                .ok_or_else(|| format!("{path}.rights is required"))?,
            &format!("{path}.rights"),
            evidence_class,
        )?;
        all_rights_cleared &= rights_cleared;

        let crs_known = validate_version_field(
            dataset
                .get("coordinate_reference")
                .ok_or_else(|| format!("{path}.coordinate_reference is required"))?,
            &format!("{path}.coordinate_reference"),
        )?;
        all_dataset_crs_known &= crs_known;

        let artifacts = dataset
            .get("artifacts")
            .and_then(Value::as_array)
            .filter(|items| !items.is_empty())
            .ok_or_else(|| format!("{path}.artifacts must be non-empty"))?;
        for (artifact_index, artifact) in artifacts.iter().enumerate() {
            let artifact_path = format!("{path}.artifacts[{artifact_index}]");
            let relative_path = required_text(artifact, "relative_path", &artifact_path)?;
            let expected_length = artifact
                .get("byte_length")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{artifact_path}.byte_length must be a non-negative integer"))?;
            let expected_digest = required_text(artifact, "sha256", &artifact_path)?;
            if expected_digest.len() != 64
                || !expected_digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(format!("{artifact_path}.sha256 must be lowercase hexadecimal SHA-256"));
            }

            let (label, bytes) = match relative_path {
                "model-evidence-payloads/synthetic-input.json" => ("input", input_bytes),
                "model-evidence-payloads/synthetic-output.json" => ("output", output_bytes),
                other => {
                    return Err(format!(
                        "{artifact_path}: path outside frozen fixture allow-list: {other}"
                    ));
                }
            };
            if expected_length != bytes.len() as u64 {
                return Err(format!(
                    "{artifact_path}: byte length mismatch for {relative_path}; expected {expected_length}, received {}",
                    bytes.len()
                ));
            }
            let digest = format!("{:x}", Sha256::digest(bytes));
            if digest != expected_digest {
                return Err(format!(
                    "{artifact_path}: SHA-256 mismatch for {relative_path}"
                ));
            }
            serde_json::from_slice::<Value>(bytes)
                .map_err(|error| format!("{artifact_path}: artifact JSON invalid: {error}"))?;
            artifact_receipts.push(ArtifactReceipt {
                dataset_id: id.to_owned(),
                relative_path: relative_path.to_owned(),
                byte_length: bytes.len(),
                sha256: digest,
                integrity_verified: true,
            });
            let _ = label;
        }
    }
    if input_ids.is_empty() || output_ids.is_empty() {
        return Err("at least one INPUT and one OUTPUT dataset are required".to_owned());
    }

    let edges = root
        .get("lineage_edges")
        .and_then(Value::as_array)
        .ok_or_else(|| "lineage_edges must be an array".to_owned())?;
    let mut edge_keys = BTreeSet::new();
    let mut expected_edges = BTreeSet::new();
    for id in &input_ids {
        expected_edges.insert(format!(
            "dataset:{id}|analysis:{analysis_id}|CONSUMED_BY"
        ));
    }
    for id in &output_ids {
        expected_edges.insert(format!(
            "analysis:{analysis_id}|dataset:{id}|PRODUCED_BY"
        ));
    }

    for (index, edge) in edges.iter().enumerate() {
        let path = format!("lineage_edges[{index}]");
        let from = required_text(edge, "from", &path)?;
        let to = required_text(edge, "to", &path)?;
        let relationship = required_text(edge, "relationship", &path)?;
        if !["CONSUMED_BY", "PRODUCED_BY"].contains(&relationship) {
            return Err(format!("{path}.relationship unsupported: {relationship}"));
        }
        let key = format!("{from}|{to}|{relationship}");
        if !edge_keys.insert(key.clone()) {
            return Err(format!("{path}: duplicate lineage edge"));
        }
        if !expected_edges.contains(&key) {
            return Err(format!("{path}: edge references unknown or directionally invalid lineage"));
        }
    }
    if edge_keys != expected_edges {
        return Err("lineage_edges do not connect every dataset to the declared analysis".to_owned());
    }

    let uncertainty = root
        .get("uncertainty")
        .ok_or_else(|| "uncertainty is required".to_owned())?;
    let uncertainty_status = required_text(uncertainty, "status", "uncertainty")?;
    if !["REPRESENTED", "NOT_REPRESENTED", "UNKNOWN"].contains(&uncertainty_status) {
        return Err(format!("unsupported uncertainty.status: {uncertainty_status}"));
    }
    required_text(uncertainty, "representation", "uncertainty")?;
    let uncertainty_method_known = uncertainty
        .get("method_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .is_some();
    let uncertainty_calibration_known = uncertainty
        .get("calibration_reference")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .is_some();
    if uncertainty_status == "REPRESENTED"
        && !(uncertainty_method_known && uncertainty_calibration_known)
    {
        return Err(
            "represented uncertainty requires method_id and calibration_reference".to_owned(),
        );
    }
    required_text(&root, "claim_ceiling", "root")?;

    let disposition = if evidence_class == "synthetic" || execution_status == "NOT_EXECUTED" {
        "ABSTAIN_SYNTHETIC_NOT_EXECUTED"
    } else if execution_status != "COMPLETED" {
        "ABSTAIN_ANALYSIS_NOT_COMPLETED"
    } else if !version_known
        || !run_id_known
        || !all_revisions_known
        || !all_rights_cleared
        || !all_dataset_crs_known
        || !scope_crs_known
        || uncertainty_status != "REPRESENTED"
    {
        "ABSTAIN_INCOMPLETE_EXTERNAL_EVIDENCE"
    } else {
        "ENVELOPE_INTEGRITY_VERIFIED_PHYSICAL_VALIDITY_UNASSESSED"
    };

    let canonical = serde_json::to_vec(&root)
        .map_err(|error| format!("envelope canonicalization failed: {error}"))?;
    Ok(GateReport {
        schema_version: SCHEMA,
        evidence_class: evidence_class.to_owned(),
        provider_id: provider_id.to_owned(),
        analysis_id: analysis_id.to_owned(),
        execution_status: execution_status.to_owned(),
        dataset_count: datasets.len(),
        input_dataset_count: input_ids.len(),
        output_dataset_count: output_ids.len(),
        artifact_receipts,
        envelope_sha256: format!("{:x}", Sha256::digest(canonical)),
        disposition,
        structural_checks_passed: true,
        claim_ceiling: "envelope and artifact integrity only; no independent model validation or physical truth",
    })
}

fn run() -> Result<GateReport, String> {
    validate_envelope(FIXTURE, INPUT_BYTES, OUTPUT_BYTES)
}

fn main() {
    match run() {
        Ok(report) => println!(
            "{}",
            serde_json::to_string_pretty(&report).expect("report serialization")
        ),
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("embedded fixture JSON")
    }

    #[test]
    fn synthetic_envelope_and_both_artifacts_validate_but_abstain_from_model_claim() {
        let report = run().expect("valid synthetic envelope");
        assert!(report.structural_checks_passed);
        assert_eq!(report.dataset_count, 2);
        assert_eq!(report.input_dataset_count, 1);
        assert_eq!(report.output_dataset_count, 1);
        assert_eq!(report.artifact_receipts.len(), 2);
        assert_eq!(report.disposition, "ABSTAIN_SYNTHETIC_NOT_EXECUTED");
        assert!(report.artifact_receipts.iter().all(|item| item.integrity_verified));
    }

    #[test]
    fn artifact_byte_tampering_is_rejected() {
        let tampered = b"{\"fixture_id\":\"tampered\"}\n";
        let error = validate_envelope(FIXTURE, tampered, OUTPUT_BYTES).expect_err("digest mismatch");
        assert!(error.contains("byte length mismatch") || error.contains("SHA-256 mismatch"));
    }

    #[test]
    fn duplicate_dataset_ids_are_rejected() {
        let mut root = fixture_value();
        let duplicate = root["datasets"][0]["dataset_id"].clone();
        root["datasets"][1]["dataset_id"] = duplicate;
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("duplicate dataset");
        assert!(error.contains("duplicate dataset_id"));
    }

    #[test]
    fn incomplete_lineage_is_rejected() {
        let mut root = fixture_value();
        root["lineage_edges"].as_array_mut().expect("edges").pop();
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("incomplete lineage");
        assert!(error.contains("lineage_edges do not connect"));
    }

    #[test]
    fn lineage_edge_cannot_reverse_input_output_direction() {
        let mut root = fixture_value();
        root["lineage_edges"][0]["relationship"] = Value::String("PRODUCED_BY".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("reversed edge");
        assert!(error.contains("directionally invalid"));
    }

    #[test]
    fn external_run_without_run_identity_is_rejected() {
        let mut root = fixture_value();
        root["evidence_class"] = Value::String("external_observed".into());
        root["source"]["provider_kind"] = Value::String("EXTERNAL_MODEL".into());
        root["analysis"]["execution_status"] = Value::String("COMPLETED".into());
        root["analysis"]["completed_at"] = Value::String("2026-11-16T00:00:00+02:00".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("missing run identity");
        assert!(error.contains("completed analysis requires a non-empty analysis.run_id"));
    }

    #[test]
    fn external_run_with_unknown_rights_abstains() {
        let mut root = fixture_value();
        root["evidence_class"] = Value::String("external_observed".into());
        root["source"]["provider_kind"] = Value::String("EXTERNAL_MODEL".into());
        root["analysis"]["execution_status"] = Value::String("COMPLETED".into());
        root["analysis"]["run_id"] = Value::String("provider-run-001".into());
        root["analysis"]["completed_at"] = Value::String("2026-11-16T00:00:00+02:00".into());
        let report = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect("unknown rights should not become an error or a pass");
        assert_eq!(report.disposition, "ABSTAIN_INCOMPLETE_EXTERNAL_EVIDENCE");
    }

    #[test]
    fn unknown_dataset_revision_is_preserved_but_not_admitted_for_external_comparison() {
        let mut root = fixture_value();
        root["evidence_class"] = Value::String("external_observed".into());
        root["source"]["provider_kind"] = Value::String("EXTERNAL_MODEL".into());
        root["source"]["software_version"] = json!({"status":"KNOWN","value":"1.22"});
        root["analysis"]["execution_status"] = Value::String("COMPLETED".into());
        root["analysis"]["run_id"] = Value::String("provider-run-001".into());
        root["analysis"]["completed_at"] = Value::String("2026-11-16T00:00:00+02:00".into());
        let report = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect("unknown revision retained explicitly");
        assert_eq!(report.disposition, "ABSTAIN_INCOMPLETE_EXTERNAL_EVIDENCE");
    }

    #[test]
    fn unknown_artifact_path_cannot_escape_fixture_allow_list() {
        let mut root = fixture_value();
        root["datasets"][0]["artifacts"][0]["relative_path"] =
            Value::String("../../../../secret.txt".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("path traversal");
        assert!(error.contains("path outside frozen fixture allow-list"));
    }

    #[test]
    fn represented_uncertainty_requires_method_and_calibration() {
        let mut root = fixture_value();
        root["uncertainty"]["status"] = Value::String("REPRESENTED".into());
        root["uncertainty"]["method_id"] = Value::String("method-001".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("missing calibration");
        assert!(error.contains("calibration_reference"));
    }
}
