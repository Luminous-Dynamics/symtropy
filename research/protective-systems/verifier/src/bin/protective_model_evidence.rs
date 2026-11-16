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
const SCHEMA_SPEC: &str =
    include_str!("../../../schemas/protective-model-evidence-envelope-v0.1.schema.json");
const INPUT_DATASET_METADATA_BYTES: &[u8] =
    include_bytes!("../../../fixtures/model-evidence-metadata/synthetic-input-dataset.json");
const INPUT_FILE_METADATA_BYTES: &[u8] =
    include_bytes!("../../../fixtures/model-evidence-metadata/synthetic-input-files.json");
const OUTPUT_DATASET_METADATA_BYTES: &[u8] =
    include_bytes!("../../../fixtures/model-evidence-metadata/synthetic-output-dataset.json");
const OUTPUT_FILE_METADATA_BYTES: &[u8] =
    include_bytes!("../../../fixtures/model-evidence-metadata/synthetic-output-files.json");
const INPUT_BYTES: &[u8] =
    include_bytes!("../../../fixtures/model-evidence-payloads/synthetic-input.json");
const OUTPUT_BYTES: &[u8] =
    include_bytes!("../../../fixtures/model-evidence-payloads/synthetic-output.json");
const SCHEMA: &str = "protective-model-evidence-envelope/0.1";

#[derive(Debug, Serialize)]
struct ArtifactReceipt {
    dataset_id: String,
    role: String,
    relative_path: String,
    provider_file_id: Option<String>,
    file_name: Option<String>,
    media_type: Option<String>,
    byte_length: usize,
    sha256: String,
    integrity_verified: bool,
}

#[derive(Debug, Serialize)]
struct GateReport {
    schema_version: &'static str,
    schema_spec_sha256: String,
    evidence_class: String,
    provider_id: String,
    analysis_id: String,
    execution_status: String,
    dataset_count: usize,
    input_dataset_count: usize,
    output_dataset_count: usize,
    dataset_metadata_snapshot_count: usize,
    file_metadata_snapshot_count: usize,
    data_payload_count: usize,
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

fn validate_dataset_payload_media_type(
    dataset: &Value,
    payload_media_type: &str,
) -> Result<(), String> {
    let declared = required_text(dataset, "data_type", "dataset")?;
    if declared != payload_media_type {
        return Err("dataset data_type does not match provider payload MIME type".to_owned());
    }
    Ok(())
}

fn validate_rights(
    value: &Value,
    path: &str,
    evidence_class: &str,
    intended_use: &str,
) -> Result<bool, String> {
    let status = required_text(value, "status", path)?;
    let license = value
        .get("license_id")
        .ok_or_else(|| format!("{path}.license_id is required"))?;
    let permitted_use = required_text(value, "permitted_use", path)?;
    match status {
        "SYNTHETIC_OPEN_FIXTURE"
            if evidence_class == "synthetic"
                && intended_use == "SYNTHETIC_TEST_ONLY"
                && permitted_use == "SYNTHETIC_TEST_ONLY"
                && license.is_null() => Ok(true),
        "LICENSED"
            if evidence_class == "external_observed"
                && permitted_use == intended_use
                && ["INTEROPERABILITY_EVALUATION", "RESEARCH_EVALUATION", "PUBLICATION_REVIEW"]
                    .contains(&intended_use)
                && license.as_str().map(str::trim).filter(|item| !item.is_empty()).is_some() => Ok(true),
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
    let schema_value: Value = serde_json::from_str(SCHEMA_SPEC)
        .map_err(|error| format!("embedded JSON Schema invalid: {error}"))?;
    if schema_value["$schema"] != "https://json-schema.org/draft/2020-12/schema"
        || schema_value["properties"]["schema_version"]["const"] != SCHEMA
    {
        return Err("embedded JSON Schema metadata does not match the executable schema".to_owned());
    }
    let schema_required = schema_value["required"]
        .as_array()
        .ok_or_else(|| "embedded JSON Schema required fields must be an array".to_owned())?;
    for key in [
        "schema_version",
        "evidence_class",
        "intended_use",
        "source",
        "analysis",
        "scope",
        "datasets",
        "lineage_edges",
        "uncertainty",
        "claim_ceiling",
    ] {
        if !schema_required.iter().any(|item| item.as_str() == Some(key)) {
            return Err(format!("embedded JSON Schema omits required field {key}"));
        }
    }

    let root: Value = serde_json::from_str(envelope_text)
        .map_err(|error| format!("envelope JSON invalid: {error}"))?;
    if required_text(&root, "schema_version", "root")? != SCHEMA {
        return Err(format!("schema_version must equal {SCHEMA}"));
    }

    let evidence_class = required_text(&root, "evidence_class", "root")?;
    if !["synthetic", "external_observed"].contains(&evidence_class) {
        return Err(format!("unsupported evidence_class: {evidence_class}"));
    }
    let intended_use = required_text(&root, "intended_use", "root")?;
    let allowed_uses = [
        "SYNTHETIC_TEST_ONLY",
        "INTEROPERABILITY_EVALUATION",
        "RESEARCH_EVALUATION",
        "PUBLICATION_REVIEW",
    ];
    if !allowed_uses.contains(&intended_use) {
        return Err(format!("unsupported intended_use: {intended_use}"));
    }
    if (evidence_class == "synthetic" && intended_use != "SYNTHETIC_TEST_ONLY")
        || (evidence_class == "external_observed" && intended_use == "SYNTHETIC_TEST_ONLY")
    {
        return Err("intended_use conflicts with evidence_class".to_owned());
    }
    let claim_ceiling = required_text(&root, "claim_ceiling", "root")?;
    let expected_claim_ceiling = if evidence_class == "synthetic" {
        "SYNTHETIC_ENVELOPE_AND_ARTIFACT_INTEGRITY_ONLY"
    } else {
        "EXTERNAL_MODEL_METADATA_AND_ARTIFACT_INTEGRITY_ONLY"
    };
    if claim_ceiling != expected_claim_ceiling {
        return Err("claim_ceiling exceeds or conflicts with the declared evidence class".to_owned());
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
    let documentation_uri_known = source
        .get("documentation_uri")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|item| {
            item.starts_with("https://")
                || item.starts_with("http://")
                || item.starts_with("urn:")
        })
        .is_some();

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
        if analysis["started_at"].is_null() || analysis["completed_at"].is_null() {
            return Err("completed analysis requires both analysis.started_at and analysis.completed_at".to_owned());
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
    let mut artifact_paths = BTreeSet::new();
    let mut all_revisions_known = true;
    let mut all_rights_metadata_present = true;
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
            intended_use,
        )?;
        all_rights_metadata_present &= rights_cleared;

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
        let mut roles = BTreeSet::new();
        let mut dataset_file_ids = BTreeSet::new();
        let mut endpoint_file_ids = BTreeSet::new();
        let mut payload_descriptor: Option<(String, String, String, u64)> = None;
        let mut native_file_descriptor: Option<(String, String, String, u64)> = None;
        let mut dataset_metadata_seen = false;
        let mut file_metadata_seen = false;
        let mut payload_seen = false;

        for (artifact_index, artifact) in artifacts.iter().enumerate() {
            let artifact_path = format!("{path}.artifacts[{artifact_index}]");
            let role = required_text(artifact, "role", &artifact_path)?;
            if !roles.insert(role.to_owned()) {
                return Err(format!("{artifact_path}: duplicate artifact role {role}"));
            }
            let relative_path = required_text(artifact, "relative_path", &artifact_path)?;
            let file_name = required_text(artifact, "file_name", &artifact_path)?;
            let media_type = required_text(artifact, "media_type", &artifact_path)?;
            let provider_file_id = match artifact.get("provider_file_id") {
                Some(Value::Null) => None,
                Some(Value::String(value)) if !value.trim().is_empty() => {
                    Some(value.trim().to_owned())
                }
                _ => return Err(format!("{artifact_path}.provider_file_id must be non-empty text or null")),
            };
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

            let (expected_path, expected_name, expected_media, expected_file_id, bytes) =
                match (direction, role) {
                    ("INPUT", "DATASET_METADATA") => (
                        "model-evidence-metadata/synthetic-input-dataset.json",
                        "synthetic-input-dataset.json",
                        "application/json",
                        None,
                        INPUT_DATASET_METADATA_BYTES,
                    ),
                    ("INPUT", "FILE_METADATA") => (
                        "model-evidence-metadata/synthetic-input-files.json",
                        "synthetic-input-files.json",
                        "application/json",
                        None,
                        INPUT_FILE_METADATA_BYTES,
                    ),
                    ("INPUT", "DATA_PAYLOAD") => (
                        "model-evidence-payloads/synthetic-input.json",
                        "synthetic-input.json",
                        "application/json",
                        Some("synthetic-file-water-input-001"),
                        input_bytes,
                    ),
                    ("OUTPUT", "DATASET_METADATA") => (
                        "model-evidence-metadata/synthetic-output-dataset.json",
                        "synthetic-output-dataset.json",
                        "application/json",
                        None,
                        OUTPUT_DATASET_METADATA_BYTES,
                    ),
                    ("OUTPUT", "FILE_METADATA") => (
                        "model-evidence-metadata/synthetic-output-files.json",
                        "synthetic-output-files.json",
                        "application/json",
                        None,
                        OUTPUT_FILE_METADATA_BYTES,
                    ),
                    ("OUTPUT", "DATA_PAYLOAD") => (
                        "model-evidence-payloads/synthetic-output.json",
                        "synthetic-output.json",
                        "application/json",
                        Some("synthetic-file-water-output-001"),
                        output_bytes,
                    ),
                    _ => return Err(format!("{artifact_path}: unsupported direction/role pair")),
                };
            if relative_path != expected_path || file_name != expected_name || media_type != expected_media {
                return Err(format!("{artifact_path}: artifact path/name/media type does not match its direction and role"));
            }
            if provider_file_id.as_deref() != expected_file_id {
                return Err(format!("{artifact_path}: provider file ID does not match dataset metadata role"));
            }
            if !artifact_paths.insert(relative_path.to_owned()) {
                return Err(format!("{artifact_path}: duplicate artifact path"));
            }
            if expected_length != bytes.len() as u64 {
                return Err(format!(
                    "{artifact_path}: byte length mismatch for {relative_path}; expected {expected_length}, received {}",
                    bytes.len()
                ));
            }
            let digest = format!("{:x}", Sha256::digest(bytes));
            if digest != expected_digest {
                return Err(format!("{artifact_path}: SHA-256 mismatch for {relative_path}"));
            }
            let parsed: Value = serde_json::from_slice(bytes)
                .map_err(|error| format!("{artifact_path}: artifact JSON invalid: {error}"))?;

            match role {
                "DATASET_METADATA" => {
                    if required_text(&parsed, "id", &artifact_path)? != id {
                        return Err(format!("{artifact_path}: provider dataset ID differs from envelope dataset_id"));
                    }
                    if required_text(&parsed, "dataType", &artifact_path)?
                        != required_text(dataset, "semantic_type", &path)?
                    {
                        return Err(format!("{artifact_path}: provider dataType differs from envelope semantic_type"));
                    }
                    let descriptors = parsed
                        .get("fileDescriptors")
                        .and_then(Value::as_array)
                        .filter(|items| !items.is_empty())
                        .ok_or_else(|| format!("{artifact_path}.fileDescriptors must be non-empty"))?;
                    for descriptor in descriptors {
                        let file_id = required_text(descriptor, "id", &artifact_path)?;
                        let filename = required_text(descriptor, "filename", &artifact_path)?;
                        let mime_type = required_text(descriptor, "mimeType", &artifact_path)?;
                        let size = required_u64(descriptor, "size", &artifact_path)?;
                        if !dataset_file_ids.insert(file_id.to_owned()) {
                            return Err(format!("{artifact_path}: duplicate provider file ID {file_id}"));
                        }
                        if native_file_descriptor.replace((
                            file_id.to_owned(),
                            filename.to_owned(),
                            mime_type.to_owned(),
                            size,
                        )).is_some() {
                            return Err(format!("{artifact_path}: E1 v0.1 fixture expects exactly one provider file descriptor"));
                        }
                    }
                    dataset_metadata_seen = true;
                }
                "FILE_METADATA" => {
                    let descriptors = parsed
                        .as_array()
                        .filter(|items| !items.is_empty())
                        .ok_or_else(|| format!("{artifact_path}: file metadata response must be a non-empty array"))?;
                    for descriptor in descriptors {
                        let file_id = required_text(descriptor, "id", &artifact_path)?;
                        if !endpoint_file_ids.insert(file_id.to_owned()) {
                            return Err(format!("{artifact_path}: duplicate file ID in provider file-metadata response"));
                        }
                    }
                    file_metadata_seen = true;
                }
                "DATA_PAYLOAD" => {
                    let file_id = provider_file_id.clone().expect("payload file ID validated above");
                    payload_descriptor = Some((
                        file_id,
                        file_name.to_owned(),
                        media_type.to_owned(),
                        expected_length,
                    ));
                    payload_seen = true;
                }
                _ => unreachable!("role was validated by direction/role match"),
            }

            artifact_receipts.push(ArtifactReceipt {
                dataset_id: id.to_owned(),
                role: role.to_owned(),
                relative_path: relative_path.to_owned(),
                provider_file_id,
                file_name: Some(file_name.to_owned()),
                media_type: Some(media_type.to_owned()),
                byte_length: bytes.len(),
                sha256: digest,
                integrity_verified: true,
            });
        }

        let expected_roles: BTreeSet<String> = [
            "DATASET_METADATA".to_owned(),
            "FILE_METADATA".to_owned(),
            "DATA_PAYLOAD".to_owned(),
        ]
        .into_iter()
        .collect();
        if roles != expected_roles
            || !dataset_metadata_seen
            || !file_metadata_seen
            || !payload_seen
        {
            return Err(format!("{path}: exactly one dataset metadata, file metadata and data payload artifact are required"));
        }
        if dataset_file_ids != endpoint_file_ids {
            return Err(format!("{path}: embedded fileDescriptors IDs do not match the separate file-metadata endpoint"));
        }
        let (payload_file_id, payload_file_name, payload_media_type, payload_size) =
            payload_descriptor.expect("payload role validated above");
        let (native_file_id, native_filename, native_media_type, native_size) =
            native_file_descriptor.expect("dataset metadata file descriptor validated above");
        if !dataset_file_ids.contains(&payload_file_id) {
            return Err(format!("{path}: data payload file ID is absent from provider file metadata"));
        }
        if payload_file_id != native_file_id
            || payload_file_name != native_filename
            || payload_media_type != native_media_type
            || payload_size != native_size
        {
            return Err(format!("{path}: data payload identity/filename/MIME/size differs from provider file descriptor"));
        }
        validate_dataset_payload_media_type(dataset, &payload_media_type)?;
    }
    if input_ids.is_empty() || output_ids.is_empty() {
        return Err("at least one INPUT and one OUTPUT dataset are required".to_owned());
    }
    if artifact_receipts.len() != 6 || artifact_paths.len() != 6 {
        return Err("E1 v0.1 frozen fixture requires three distinct provider/payload artifacts per dataset".to_owned());
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
    let disposition = if evidence_class == "synthetic" || execution_status == "NOT_EXECUTED" {
        "ABSTAIN_SYNTHETIC_NOT_EXECUTED"
    } else if execution_status != "COMPLETED" {
        "ABSTAIN_ANALYSIS_NOT_COMPLETED"
    } else if !version_known
        || !run_id_known
        || !all_revisions_known
        || !all_rights_metadata_present
        || !all_dataset_crs_known
        || !scope_crs_known
        || !documentation_uri_known
        || uncertainty_status != "REPRESENTED"
    {
        "ABSTAIN_INCOMPLETE_EXTERNAL_EVIDENCE"
    } else {
        "ENVELOPE_METADATA_COMPLETE_SOURCE_AND_RIGHTS_AUTHENTICITY_UNVERIFIED"
    };

    let canonical = serde_json::to_vec(&root)
        .map_err(|error| format!("envelope canonicalization failed: {error}"))?;
    Ok(GateReport {
        schema_version: SCHEMA,
        schema_spec_sha256: format!("{:x}", Sha256::digest(SCHEMA_SPEC.as_bytes())),
        evidence_class: evidence_class.to_owned(),
        provider_id: provider_id.to_owned(),
        analysis_id: analysis_id.to_owned(),
        execution_status: execution_status.to_owned(),
        dataset_count: datasets.len(),
        input_dataset_count: input_ids.len(),
        output_dataset_count: output_ids.len(),
        dataset_metadata_snapshot_count: datasets.len(),
        file_metadata_snapshot_count: datasets.len(),
        data_payload_count: datasets.len(),
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
        assert_eq!(report.artifact_receipts.len(), 6);
        assert_eq!(report.dataset_metadata_snapshot_count, 2);
        assert_eq!(report.file_metadata_snapshot_count, 2);
        assert_eq!(report.data_payload_count, 2);
        assert_eq!(report.disposition, "ABSTAIN_SYNTHETIC_NOT_EXECUTED");
        assert!(report.artifact_receipts.iter().all(|item| item.integrity_verified));
    }

    #[test]
    fn completed_external_metadata_never_authenticates_the_provider() {
        let mut root = fixture_value();
        root["evidence_class"] = Value::String("external_observed".into());
        root["intended_use"] = Value::String("INTEROPERABILITY_EVALUATION".into());
        root["claim_ceiling"] =
            Value::String("EXTERNAL_MODEL_METADATA_AND_ARTIFACT_INTEGRITY_ONLY".into());
        root["source"]["provider_kind"] = Value::String("EXTERNAL_MODEL".into());
        root["source"]["software_version"] =
            json!({"status":"KNOWN","value":"adapter-fixture-1"});
        root["analysis"]["execution_status"] = Value::String("COMPLETED".into());
        root["analysis"]["run_id"] = Value::String("synthetic-test-run-only".into());
        root["analysis"]["started_at"] = Value::String("2026-11-16T00:00:00+01:00".into());
        root["analysis"]["completed_at"] = Value::String("2026-11-16T00:00:00+02:00".into());
        root["scope"]["coordinate_reference"] =
            json!({"status":"KNOWN","value":"EPSG:4326"});
        root["uncertainty"] = json!({
            "status":"REPRESENTED",
            "representation":"test-only interval metadata",
            "method_id":"test-method",
            "calibration_reference":"test-calibration"
        });
        for dataset in root["datasets"].as_array_mut().expect("datasets") {
            dataset["revision"] = json!({"status":"KNOWN","value":"test-revision"});
            dataset["rights"] = json!({
                "status":"LICENSED",
                "license_id":"TEST-LICENSE-ONLY",
                "permitted_use":"INTEROPERABILITY_EVALUATION"
            });
            dataset["coordinate_reference"] =
                json!({"status":"KNOWN","value":"EPSG:4326"});
        }
        let report = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect("complete synthetic metadata envelope");
        assert_eq!(
            report.disposition,
            "ENVELOPE_METADATA_COMPLETE_SOURCE_AND_RIGHTS_AUTHENTICITY_UNVERIFIED"
        );
    }

    #[test]
    fn claim_ceiling_cannot_be_promoted_by_envelope_author() {
        let mut root = fixture_value();
        root["claim_ceiling"] =
            Value::String("PHYSICALLY_VALIDATED_AND_SAFE".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("unsupported claim");
        assert!(error.contains("claim_ceiling exceeds"));
    }

    #[test]
    fn intended_use_must_match_the_evidence_class() {
        let mut root = fixture_value();
        root["intended_use"] = Value::String("PUBLICATION_REVIEW".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("rights use mismatch");
        assert!(error.contains("intended_use conflicts"));
    }

    #[test]
    fn license_scope_mismatch_fails_closed() {
        let mut root = fixture_value();
        root["evidence_class"] = Value::String("external_observed".into());
        root["intended_use"] = Value::String("INTEROPERABILITY_EVALUATION".into());
        root["claim_ceiling"] =
            Value::String("EXTERNAL_MODEL_METADATA_AND_ARTIFACT_INTEGRITY_ONLY".into());
        root["source"]["provider_kind"] = Value::String("EXTERNAL_MODEL".into());
        for dataset in root["datasets"].as_array_mut().expect("datasets") {
            dataset["rights"] = json!({
                "status":"LICENSED",
                "license_id":"TEST-LICENSE-ONLY",
                "permitted_use":"RESEARCH_EVALUATION"
            });
        }
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("license purpose does not cover intended use");
        assert!(error.contains(".status is unsupported"));
    }

    #[test]
    fn artifact_byte_tampering_is_rejected() {
        let tampered = b"{\"fixture_id\":\"tampered\"}\n";
        let error = validate_envelope(FIXTURE, tampered, OUTPUT_BYTES).expect_err("digest mismatch");
        assert!(error.contains("byte length mismatch") || error.contains("SHA-256 mismatch"));
    }

    #[test]
    fn provider_dataset_id_must_match_native_metadata_snapshot() {
        let mut root = fixture_value();
        root["datasets"][0]["dataset_id"] = Value::String("wrong-provider-dataset-id".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("provider dataset ID mismatch");
        assert!(error.contains("provider dataset ID differs"));
    }

    #[test]
    fn payload_file_id_must_match_dataset_and_file_metadata_descriptors() {
        let mut root = fixture_value();
        root["datasets"][0]["artifacts"][2]["provider_file_id"] =
            Value::String("unregistered-provider-file".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("unregistered provider file ID");
        assert!(error.contains("provider file ID does not match"));
    }

    #[test]
    fn semantic_type_must_match_provider_native_data_type() {
        let mut root = fixture_value();
        root["datasets"][0]["semantic_type"] =
            Value::String("different:providerDataType".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("semantic type mismatch");
        assert!(error.contains("provider dataType differs"));
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
    fn input_dataset_cannot_claim_output_artifact() {
        let mut root = fixture_value();
        root["datasets"][0]["artifacts"][0]["relative_path"] =
            Value::String("model-evidence-payloads/synthetic-output.json".into());
        root["datasets"][0]["artifacts"][0]["byte_length"] = Value::from(272);
        root["datasets"][0]["artifacts"][0]["sha256"] =
            Value::String("ce1c4bf7409abfb7e6056cfad06552e694d8f147ca4a8e78242cecd4dcbe8cf6".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("input/output artifact mismatch");
        assert!(error.contains("does not match dataset direction"));
    }

    #[test]
    fn artifact_path_cannot_be_reused_across_datasets() {
        let mut root = fixture_value();
        root["datasets"][1]["artifacts"][0]["relative_path"] =
            Value::String("model-evidence-payloads/synthetic-input.json".into());
        root["datasets"][1]["artifacts"][0]["byte_length"] = Value::from(228);
        root["datasets"][1]["artifacts"][0]["sha256"] =
            Value::String("7dc7062f31a2cb3f39ba2b65dbd4c9b0d7f1839f4b48288583806d9e188f03bf".into());
        let error = validate_envelope(&root.to_string(), INPUT_BYTES, OUTPUT_BYTES)
            .expect_err("output/input artifact mismatch");
        assert!(error.contains("does not match dataset direction"));
    }

    #[test]
    fn incomplete_lineage_is_rejected() {
        let mut root = fixture_value();
        let _ = root["lineage_edges"].as_array_mut().expect("edges").pop();
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
        root["analysis"]["started_at"] = Value::String("2026-11-16T00:00:00+01:00".into());
        root["analysis"]["completed_at"] = Value::String("2026-11-16T00:00:00+02:00".into());
        root["intended_use"] = Value::String("INTEROPERABILITY_EVALUATION".into());
        root["claim_ceiling"] = Value::String("EXTERNAL_MODEL_METADATA_AND_ARTIFACT_INTEGRITY_ONLY".into());
        root["source"]["software_version"] = json!({"status":"KNOWN","value":"adapter-fixture-1"});
        root["scope"]["coordinate_reference"] = json!({"status":"KNOWN","value":"EPSG:4326"});
        root["uncertainty"] = json!({"status":"REPRESENTED","representation":"test-only interval metadata","method_id":"test-method","calibration_reference":"test-calibration"});
        for dataset in root["datasets"].as_array_mut().expect("datasets") {
            dataset["revision"] = json!({"status":"KNOWN","value":"dataset-revision-1"});
            dataset["coordinate_reference"] = json!({"status":"KNOWN","value":"EPSG:4326"});
            dataset["rights"]["status"] = Value::String("UNKNOWN".into());
            dataset["rights"]["license_id"] = Value::Null;
            dataset["rights"]["permitted_use"] = Value::String("NOT_ADMITTED".into());
        }
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
        root["analysis"]["started_at"] = Value::String("2026-11-16T00:00:00+01:00".into());
        root["analysis"]["completed_at"] = Value::String("2026-11-16T00:00:00+02:00".into());
        root["intended_use"] = Value::String("INTEROPERABILITY_EVALUATION".into());
        root["claim_ceiling"] = Value::String("EXTERNAL_MODEL_METADATA_AND_ARTIFACT_INTEGRITY_ONLY".into());
        root["scope"]["coordinate_reference"] = json!({"status":"KNOWN","value":"EPSG:4326"});
        root["uncertainty"] = json!({"status":"REPRESENTED","representation":"test-only interval metadata","method_id":"test-method","calibration_reference":"test-calibration"});
        for dataset in root["datasets"].as_array_mut().expect("datasets") {
            dataset["rights"] = json!({"status":"LICENSED","license_id":"TEST-LICENSE-ONLY","permitted_use":"INTEROPERABILITY_EVALUATION"});
            dataset["coordinate_reference"] = json!({"status":"KNOWN","value":"EPSG:4326"});
        }
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
        assert!(error.contains("artifact path does not match dataset direction"));
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
