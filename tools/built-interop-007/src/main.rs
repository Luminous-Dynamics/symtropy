// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Paired source→target preservation gate built from typed IFC/IDS observations.
//!
//! This compares stable object/property observations, never STEP text or row
//! numbers. IDS conformance and export/import preservation remain separate claims.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

const REPORT_SCHEMA_VERSION: &str = "1.0.0";
const BENCHMARK_ID: &str = "BUILT-INTEROP-007";
const ADAPTER_BENCHMARK_ID: &str = "BUILT-INTEROP-002-ifc-ids-v0.1";
const ADAPTER_SCHEMA_VERSION: &str = "0.1.0";
const SUPPORTED_PROFILE: &str = "IFC4/IDS1.0/IFCWALL.Name/simpleValue-v0.1";
const IDS_DECLARED_EVIDENCE: &str = "Declared:IDS1.0";
const POLICY_ID: &str = "BUILT-INTEROP-007/v0.1;match=valid-GlobalId+requirement-id+property-key;profile=IFC4-IDS1.0-IFCWALL.Name;changed-lost-dropped=FAIL;unstable-incomplete-unsupported=UNRESOLVED;additions=record-only;semantic-digest=verified";
const PAYLOAD_POINTER: &str = "report-json-pointer:/evidence_payload";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum AdapterDecision {
    SubsetConformant,
    Blocked,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum AdapterPropertyOutcome {
    Preserved,
    Changed,
    Missing,
    NotEvaluated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdapterObservation {
    requirement_id: String,
    source_object_id: String,
    source_step_id: String,
    source_entity_type: String,
    source_attribute_index: usize,
    source_attribute_path: String,
    property_key: String,
    expected_value: String,
    observed_value: Option<String>,
    outcome: AdapterPropertyOutcome,
    diagnostic_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AdapterReport {
    schema_version: String,
    benchmark_id: String,
    supported_profile: String,
    decision: AdapterDecision,
    ifc_schema_token: Option<String>,
    ids_version_evidence: String,
    ifc_input_sha256: String,
    ids_input_sha256: String,
    parsed_entity_count: usize,
    applicable_wall_count: Option<usize>,
    observations: Vec<AdapterObservation>,
    diagnostics: Vec<String>,
    semantic_result_sha256: String,
}

#[derive(Serialize)]
struct AdapterSemanticDigest<'a> {
    supported_profile: &'a str,
    decision: AdapterDecision,
    ifc_schema_token: &'a Option<String>,
    ids_version_evidence: &'a str,
    parsed_entity_count: usize,
    applicable_wall_count: Option<usize>,
    observations: &'a [AdapterObservation],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum GateStatus {
    Pass,
    Fail,
    Unresolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolIdentity {
    name: String,
    version: String,
    config_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceKind {
    ValidatorReport,
    ToolOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    evidence_id: String,
    kind: EvidenceKind,
    sha256: String,
    locator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Gate {
    gate_id: String,
    mandatory: bool,
    status: GateStatus,
    claim_scope: String,
    limitations: Vec<String>,
    tool: ToolIdentity,
    evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeltaCounts {
    compared: usize,
    preserved: usize,
    changed: usize,
    lost: usize,
    dropped: usize,
    added: usize,
    unresolved: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct DeltaFinding {
    code: String,
    object_id: String,
    source_value: Option<String>,
    target_value: Option<String>,
    detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvidencePayload {
    policy_id: String,
    source_ifc_sha256: String,
    source_ifc_byte_length: usize,
    target_ifc_sha256: String,
    target_ifc_byte_length: usize,
    requirements_ids_sha256: String,
    source_adapter_report_sha256: String,
    target_adapter_report_sha256: String,
    supported_profile: String,
    profile_confirmed: bool,
    comparison_coverage_complete: bool,
    counts: DeltaCounts,
    findings: Vec<DeltaFinding>,
    decision_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Output {
    schema_version: String,
    benchmark_id: String,
    gate: Gate,
    evidence_payload: EvidencePayload,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn is_lower_hex_of_length(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_global_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 22
        && matches!(bytes[0], b'0'..=b'3')
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'$'))
}

fn verify_adapter_report(
    report: &AdapterReport,
    model_bytes: &[u8],
    ids_bytes: &[u8],
    label: &str,
) -> Result<(), String> {
    if report.schema_version != ADAPTER_SCHEMA_VERSION
        || report.benchmark_id != ADAPTER_BENCHMARK_ID
    {
        return Err(format!("UNSUPPORTED_ADAPTER_REPORT:{label}"));
    }
    let model_sha = sha256_hex(model_bytes);
    let ids_sha = sha256_hex(ids_bytes);
    if report.ifc_input_sha256 != model_sha {
        return Err(format!("ADAPTER_MODEL_DIGEST_MISMATCH:{label}"));
    }
    if report.ids_input_sha256 != ids_sha {
        return Err(format!("ADAPTER_IDS_DIGEST_MISMATCH:{label}"));
    }
    if !is_lower_hex_of_length(&report.semantic_result_sha256, 64) {
        return Err(format!("ADAPTER_SEMANTIC_DIGEST_INVALID:{label}"));
    }

    let semantic = AdapterSemanticDigest {
        supported_profile: &report.supported_profile,
        decision: report.decision,
        ifc_schema_token: &report.ifc_schema_token,
        ids_version_evidence: &report.ids_version_evidence,
        parsed_entity_count: report.parsed_entity_count,
        applicable_wall_count: report.applicable_wall_count,
        observations: &report.observations,
        diagnostics: &report.diagnostics,
    };
    let semantic_bytes = serde_json::to_vec(&semantic)
        .map_err(|error| format!("ADAPTER_SEMANTIC_SERIALIZATION_FAILED:{label}:{error}"))?;
    if sha256_hex(&semantic_bytes) != report.semantic_result_sha256 {
        return Err(format!("ADAPTER_SEMANTIC_DIGEST_MISMATCH:{label}"));
    }
    Ok(())
}

fn observations_coverage_complete(report: &AdapterReport) -> bool {
    report.decision != AdapterDecision::Unresolved
        && report
            .applicable_wall_count
            .is_some_and(|count| count > 0 && count == report.observations.len())
}

fn index_stable_observations(
    report: &AdapterReport,
    side: &str,
    counts: &mut DeltaCounts,
    findings: &mut Vec<DeltaFinding>,
) -> (BTreeMap<String, AdapterObservation>, bool) {
    let mut id_counts = BTreeMap::<String, usize>::new();
    for observation in &report.observations {
        *id_counts
            .entry(observation.source_object_id.clone())
            .or_insert(0) += 1;
    }

    let mut indexed = BTreeMap::new();
    let mut identity_complete = true;
    for observation in &report.observations {
        let id = observation.source_object_id.as_str();
        if !valid_global_id(id) {
            identity_complete = false;
            counts.unresolved += 1;
            findings.push(DeltaFinding {
                code: "UNSTABLE_OR_INVALID_OBJECT_ID".into(),
                object_id: id.into(),
                source_value: observation.observed_value.clone(),
                target_value: None,
                detail: format!("{side} observation cannot be matched by a valid IFC GlobalId"),
            });
            continue;
        }
        if id_counts.get(id).copied().unwrap_or(0) != 1 {
            identity_complete = false;
            counts.unresolved += 1;
            findings.push(DeltaFinding {
                code: "DUPLICATE_GLOBAL_ID".into(),
                object_id: id.into(),
                source_value: observation.observed_value.clone(),
                target_value: None,
                detail: format!("{side} report contains a non-unique GlobalId"),
            });
            continue;
        }
        if observation.outcome == AdapterPropertyOutcome::NotEvaluated {
            identity_complete = false;
            counts.unresolved += 1;
            findings.push(DeltaFinding {
                code: "OBSERVATION_NOT_EVALUATED".into(),
                object_id: id.into(),
                source_value: observation.observed_value.clone(),
                target_value: None,
                detail: format!("{side} producer could not evaluate the supported property"),
            });
            continue;
        }
        if observation.source_entity_type != "IFCWALL"
            || observation.source_attribute_index != 2
            || observation.source_attribute_path != "IFCWALL.Name"
            || observation.property_key != "Name"
            || observation.requirement_id.trim().is_empty()
        {
            identity_complete = false;
            counts.unresolved += 1;
            findings.push(DeltaFinding {
                code: "OBSERVATION_OUTSIDE_SUPPORTED_PROFILE".into(),
                object_id: id.into(),
                source_value: observation.observed_value.clone(),
                target_value: None,
                detail: format!(
                    "{side} observation does not match the pinned IFCWALL.Name profile"
                ),
            });
            continue;
        }
        indexed.insert(id.to_owned(), observation.clone());
    }
    (indexed, identity_complete)
}

fn compare_reports(
    source: &AdapterReport,
    target: &AdapterReport,
) -> (
    GateStatus,
    DeltaCounts,
    Vec<DeltaFinding>,
    bool,
    bool,
    String,
) {
    let mut counts = DeltaCounts::default();
    let mut findings = Vec::new();
    let profile_ready = source.supported_profile == SUPPORTED_PROFILE
        && target.supported_profile == SUPPORTED_PROFILE
        && source.ifc_schema_token.as_deref() == Some("IFC4")
        && target.ifc_schema_token.as_deref() == Some("IFC4")
        && source.ids_version_evidence == IDS_DECLARED_EVIDENCE
        && target.ids_version_evidence == IDS_DECLARED_EVIDENCE;
    let source_coverage = observations_coverage_complete(source);
    let target_coverage = observations_coverage_complete(target);
    if !profile_ready || !source_coverage || !target_coverage {
        findings.push(DeltaFinding {
            code: "PROFILE_OR_COVERAGE_UNRESOLVED".into(),
            object_id: String::new(),
            source_value: None,
            target_value: None,
            detail: "Both reports must explicitly confirm IFC4 / IDS 1.0 and provide a complete supported observation set".into(),
        });
        counts.unresolved += 1;
    }

    let (source_index, source_identity_complete) =
        index_stable_observations(source, "source", &mut counts, &mut findings);
    let (target_index, target_identity_complete) =
        index_stable_observations(target, "target", &mut counts, &mut findings);
    let comparison_coverage_complete = profile_ready
        && source_coverage
        && target_coverage
        && source_identity_complete
        && target_identity_complete;

    for (object_id, source_observation) in &source_index {
        let Some(target_observation) = target_index.get(object_id) else {
            if target_coverage && target_identity_complete {
                counts.dropped += 1;
                findings.push(DeltaFinding {
                    code: "SOURCE_OBJECT_DROPPED".into(),
                    object_id: object_id.clone(),
                    source_value: source_observation.observed_value.clone(),
                    target_value: None,
                    detail: "A source IFCWALL with a stable GlobalId has no target observation"
                        .into(),
                });
            } else {
                counts.unresolved += 1;
                findings.push(DeltaFinding {
                    code: "UNMATCHED_SOURCE_OBJECT_UNRESOLVED".into(),
                    object_id: object_id.clone(),
                    source_value: source_observation.observed_value.clone(),
                    target_value: None,
                    detail: "The target observation set is incomplete or contains unmatchable identities".into(),
                });
            }
            continue;
        };

        if source_observation.requirement_id != target_observation.requirement_id
            || source_observation.property_key != target_observation.property_key
            || source_observation.expected_value != target_observation.expected_value
        {
            counts.unresolved += 1;
            findings.push(DeltaFinding {
                code: "REQUIREMENT_OBSERVATION_MISMATCH".into(),
                object_id: object_id.clone(),
                source_value: source_observation.observed_value.clone(),
                target_value: target_observation.observed_value.clone(),
                detail: "Same IDS bytes yielded different typed requirement identities or expected values".into(),
            });
            continue;
        }

        counts.compared += 1;
        match (
            source_observation.observed_value.as_deref(),
            target_observation.observed_value.as_deref(),
        ) {
            (Some(source_value), Some(target_value)) if source_value == target_value => {
                counts.preserved += 1;
            }
            (Some(source_value), Some(target_value)) => {
                counts.changed += 1;
                findings.push(DeltaFinding {
                    code: "REQUIRED_VALUE_CHANGED".into(),
                    object_id: object_id.clone(),
                    source_value: Some(source_value.into()),
                    target_value: Some(target_value.into()),
                    detail: "The IFCWALL.Name literal differs between source and target".into(),
                });
            }
            (Some(source_value), None) => {
                counts.lost += 1;
                findings.push(DeltaFinding {
                    code: "REQUIRED_VALUE_LOST".into(),
                    object_id: object_id.clone(),
                    source_value: Some(source_value.into()),
                    target_value: None,
                    detail: "A populated source IFCWALL.Name is absent in the target".into(),
                });
            }
            (None, Some(target_value)) => {
                counts.added += 1;
                findings.push(DeltaFinding {
                    code: "VALUE_ADDED_OR_RESTORED".into(),
                    object_id: object_id.clone(),
                    source_value: None,
                    target_value: Some(target_value.into()),
                    detail: "The target contains a Name value that was absent in the source".into(),
                });
            }
            (None, None) => {
                // Equal absence is preservation of the observed state, not IDS
                // conformance. The separate IDS gate owns that requirement.
                counts.preserved += 1;
            }
        }
    }

    for (object_id, target_observation) in &target_index {
        if source_index.contains_key(object_id) {
            continue;
        }
        if source_coverage && source_identity_complete {
            counts.added += 1;
            findings.push(DeltaFinding {
                code: "TARGET_OBJECT_ADDED".into(),
                object_id: object_id.clone(),
                source_value: None,
                target_value: target_observation.observed_value.clone(),
                detail: "Target IFCWALL has no source counterpart; addition alone is not a preservation failure".into(),
            });
        } else {
            counts.unresolved += 1;
            findings.push(DeltaFinding {
                code: "UNMATCHED_TARGET_OBJECT_UNRESOLVED".into(),
                object_id: object_id.clone(),
                source_value: None,
                target_value: target_observation.observed_value.clone(),
                detail:
                    "The source observation set is incomplete or contains unmatchable identities"
                        .into(),
            });
        }
    }

    findings.sort_by(|a, b| {
        a.object_id
            .cmp(&b.object_id)
            .then_with(|| a.code.cmp(&b.code))
            .then_with(|| a.detail.cmp(&b.detail))
    });

    let has_definite_loss = counts.changed > 0 || counts.lost > 0 || counts.dropped > 0;
    let status = if profile_ready && has_definite_loss {
        GateStatus::Fail
    } else if comparison_coverage_complete && counts.compared > 0 && counts.unresolved == 0 {
        GateStatus::Pass
    } else {
        GateStatus::Unresolved
    };
    let reason = match status {
        GateStatus::Fail => "DEFINITE_SOURCE_TO_TARGET_LOSS",
        GateStatus::Pass => "COMPLETE_SUPPORTED_DELTA_WITH_NO_LOSS",
        GateStatus::Unresolved => "PROFILE_IDENTITY_OR_COVERAGE_UNRESOLVED",
    };
    (
        status,
        counts,
        findings,
        profile_ready,
        comparison_coverage_complete,
        reason.into(),
    )
}

fn evaluate(
    source_ifc: &[u8],
    target_ifc: &[u8],
    ids_bytes: &[u8],
    source_report_bytes: &[u8],
    target_report_bytes: &[u8],
) -> Result<Output, String> {
    let source_report: AdapterReport = serde_json::from_slice(source_report_bytes)
        .map_err(|error| format!("SOURCE_ADAPTER_REPORT_INVALID:{error}"))?;
    let target_report: AdapterReport = serde_json::from_slice(target_report_bytes)
        .map_err(|error| format!("TARGET_ADAPTER_REPORT_INVALID:{error}"))?;
    verify_adapter_report(&source_report, source_ifc, ids_bytes, "source")?;
    verify_adapter_report(&target_report, target_ifc, ids_bytes, "target")?;

    let source_sha256 = sha256_hex(source_ifc);
    let target_sha256 = sha256_hex(target_ifc);
    let ids_sha256 = sha256_hex(ids_bytes);
    let source_report_sha256 = sha256_hex(source_report_bytes);
    let target_report_sha256 = sha256_hex(target_report_bytes);
    let (status, counts, findings, profile_confirmed, coverage_complete, reason) =
        compare_reports(&source_report, &target_report);

    let evidence_payload = EvidencePayload {
        policy_id: POLICY_ID.into(),
        source_ifc_sha256: source_sha256,
        source_ifc_byte_length: source_ifc.len(),
        target_ifc_sha256: target_sha256,
        target_ifc_byte_length: target_ifc.len(),
        requirements_ids_sha256: ids_sha256,
        source_adapter_report_sha256: source_report_sha256,
        target_adapter_report_sha256: target_report_sha256,
        supported_profile: SUPPORTED_PROFILE.into(),
        profile_confirmed,
        comparison_coverage_complete: coverage_complete,
        counts,
        findings,
        decision_reason: reason,
    };
    let payload_bytes = serde_json::to_vec(&evidence_payload)
        .map_err(|error| format!("EVIDENCE_PAYLOAD_SERIALIZATION_FAILED:{error}"))?;
    let payload_sha256 = sha256_hex(&payload_bytes);
    let gate = Gate {
        gate_id: "exchange_semantic_preservation".into(),
        mandatory: true,
        status,
        claim_scope: "Source-to-target preservation of stable IFCWALL.Name observations under the pinned IFC4/IDS 1.0 subset only".into(),
        limitations: vec![
            "This gate compares IFCWALL.Name observations only; it does not compare geometry, relationships, quantities, units, tolerances, material layers or arbitrary properties.".into(),
            "A PASS is not IDS model conformance. The separate ids_model_conformance gate remains authoritative for requirement satisfaction.".into(),
            "Stable matching requires a unique syntactically valid IFC GlobalId. Missing, malformed or duplicate identities prevent a clean preservation claim.".into(),
            "No building-code, structural safety, geometry equivalence, manufacturing release or physical qualification is implied.".into(),
        ],
        tool: ToolIdentity {
            name: "built-interop-semantic-delta".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            config_sha256: sha256_hex(POLICY_ID.as_bytes()),
        },
        evidence: vec![Evidence {
            evidence_id: format!("built-interop-007-{payload_sha256}"),
            kind: EvidenceKind::ValidatorReport,
            sha256: payload_sha256,
            locator: PAYLOAD_POINTER.into(),
        }],
    };
    Ok(Output {
        schema_version: REPORT_SCHEMA_VERSION.into(),
        benchmark_id: BENCHMARK_ID.into(),
        gate,
        evidence_payload,
    })
}

#[derive(Debug)]
struct CliInputs {
    source_ifc: PathBuf,
    target_ifc: PathBuf,
    ids: PathBuf,
    source_report: PathBuf,
    target_report: PathBuf,
    output: Option<PathBuf>,
}

fn parse_args() -> Result<Option<CliInputs>, String> {
    let mut raw = env::args().skip(1);
    let first = raw.next();
    if matches!(first.as_deref(), Some("--help" | "-h")) {
        return Ok(None);
    }
    let mut args = first.into_iter().chain(raw);
    let mut options = BTreeMap::<String, String>::new();
    while let Some(key) = args.next() {
        match key.as_str() {
            "--source-ifc" | "--target-ifc" | "--ids" | "--source-report" | "--target-report"
            | "--output" => {
                let value = args.next().ok_or_else(|| format!("MISSING_VALUE:{key}"))?;
                if options.insert(key.clone(), value).is_some() {
                    return Err(format!("DUPLICATE_ARGUMENT:{key}"));
                }
            }
            _ => return Err(format!("UNKNOWN_ARGUMENT:{key}")),
        }
    }
    let take = |key: &str| -> Result<String, String> {
        options
            .get(key)
            .cloned()
            .ok_or_else(|| format!("MISSING_REQUIRED_ARGUMENT:{key}"))
    };
    Ok(Some(CliInputs {
        source_ifc: PathBuf::from(take("--source-ifc")?),
        target_ifc: PathBuf::from(take("--target-ifc")?),
        ids: PathBuf::from(take("--ids")?),
        source_report: PathBuf::from(take("--source-report")?),
        target_report: PathBuf::from(take("--target-report")?),
        output: options.get("--output").map(PathBuf::from),
    }))
}

fn usage() -> &'static str {
    "Usage: built-interop-semantic-delta --source-ifc PATH --target-ifc PATH --ids PATH --source-report PATH --target-report PATH [--output PATH]\nConsumes the exact BUILT-INTEROP-002 typed report for source and target. Exit codes: 0=PASS, 1=FAIL, 2=UNRESOLVED, 64=invalid input/evidence."
}

fn run() -> Result<i32, String> {
    let Some(args) = parse_args()? else {
        println!("{}", usage());
        return Ok(0);
    };
    let source_ifc = fs::read(&args.source_ifc).map_err(|error| {
        format!(
            "CANNOT_READ_SOURCE_IFC:{}:{error}",
            args.source_ifc.display()
        )
    })?;
    let target_ifc = fs::read(&args.target_ifc).map_err(|error| {
        format!(
            "CANNOT_READ_TARGET_IFC:{}:{error}",
            args.target_ifc.display()
        )
    })?;
    let ids = fs::read(&args.ids)
        .map_err(|error| format!("CANNOT_READ_IDS:{}:{error}", args.ids.display()))?;
    let source_report = fs::read(&args.source_report).map_err(|error| {
        format!(
            "CANNOT_READ_SOURCE_REPORT:{}:{error}",
            args.source_report.display()
        )
    })?;
    let target_report = fs::read(&args.target_report).map_err(|error| {
        format!(
            "CANNOT_READ_TARGET_REPORT:{}:{error}",
            args.target_report.display()
        )
    })?;

    let output = evaluate(
        &source_ifc,
        &target_ifc,
        &ids,
        &source_report,
        &target_report,
    )?;
    let json = serde_json::to_vec_pretty(&output)
        .map_err(|error| format!("OUTPUT_SERIALIZATION_FAILED:{error}"))?;
    if let Some(path) = &args.output {
        fs::write(path, &json)
            .map_err(|error| format!("CANNOT_WRITE_OUTPUT:{}:{error}", path.display()))?;
    }
    println!("{}", String::from_utf8_lossy(&json));
    Ok(match output.gate.status {
        GateStatus::Pass => 0,
        GateStatus::Fail => 1,
        GateStatus::Unresolved => 2,
    })
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            process::exit(64);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDS_BYTES: &[u8] = b"synthetic IDS bytes";

    fn observation(
        id: &str,
        value: Option<&str>,
        outcome: AdapterPropertyOutcome,
        step_id: &str,
    ) -> AdapterObservation {
        AdapterObservation {
            requirement_id: "walls::attribute::Name".into(),
            source_object_id: id.into(),
            source_step_id: step_id.into(),
            source_entity_type: "IFCWALL".into(),
            source_attribute_index: 2,
            source_attribute_path: "IFCWALL.Name".into(),
            property_key: "Name".into(),
            expected_value: "Panel 001".into(),
            observed_value: value.map(str::to_string),
            outcome,
            diagnostic_code: match outcome {
                AdapterPropertyOutcome::Preserved => None,
                AdapterPropertyOutcome::Changed => Some("CHANGED_TARGET_PROPERTY".into()),
                AdapterPropertyOutcome::Missing => Some("MISSING_TARGET_PROPERTY".into()),
                AdapterPropertyOutcome::NotEvaluated => Some("GLOBAL_ID_INVALID".into()),
            },
        }
    }

    fn report(
        model: &[u8],
        observations: Vec<AdapterObservation>,
        decision: AdapterDecision,
        schema: Option<&str>,
        ids_bytes: &[u8],
    ) -> AdapterReport {
        let mut observations = observations;
        observations.sort_by(|a, b| {
            a.source_object_id
                .cmp(&b.source_object_id)
                .then_with(|| a.source_step_id.cmp(&b.source_step_id))
        });
        let mut diagnostics = observations
            .iter()
            .filter_map(|observation| observation.diagnostic_code.clone())
            .collect::<Vec<_>>();
        diagnostics.sort();
        diagnostics.dedup();
        let mut report = AdapterReport {
            schema_version: ADAPTER_SCHEMA_VERSION.into(),
            benchmark_id: ADAPTER_BENCHMARK_ID.into(),
            supported_profile: SUPPORTED_PROFILE.into(),
            decision,
            ifc_schema_token: schema.map(str::to_string),
            ids_version_evidence: IDS_DECLARED_EVIDENCE.into(),
            ifc_input_sha256: sha256_hex(model),
            ids_input_sha256: sha256_hex(ids_bytes),
            parsed_entity_count: observations.len(),
            applicable_wall_count: (!observations.is_empty()).then_some(observations.len()),
            observations,
            diagnostics,
            semantic_result_sha256: "0".repeat(64),
        };
        let semantic = AdapterSemanticDigest {
            supported_profile: &report.supported_profile,
            decision: report.decision,
            ifc_schema_token: &report.ifc_schema_token,
            ids_version_evidence: &report.ids_version_evidence,
            parsed_entity_count: report.parsed_entity_count,
            applicable_wall_count: report.applicable_wall_count,
            observations: &report.observations,
            diagnostics: &report.diagnostics,
        };
        report.semantic_result_sha256 = sha256_hex(&serde_json::to_vec(&semantic).unwrap());
        report
    }

    fn run_pair(
        source_obs: Vec<AdapterObservation>,
        target_obs: Vec<AdapterObservation>,
    ) -> Output {
        let source = b"source IFC";
        let target = b"target IFC";
        let source_decision = if source_obs
            .iter()
            .any(|observation| observation.outcome != AdapterPropertyOutcome::Preserved)
        {
            AdapterDecision::Blocked
        } else {
            AdapterDecision::SubsetConformant
        };
        let target_decision = if target_obs
            .iter()
            .any(|observation| observation.outcome != AdapterPropertyOutcome::Preserved)
        {
            AdapterDecision::Blocked
        } else {
            AdapterDecision::SubsetConformant
        };
        let source_report = report(source, source_obs, source_decision, Some("IFC4"), IDS_BYTES);
        let target_report = report(target, target_obs, target_decision, Some("IFC4"), IDS_BYTES);
        evaluate(
            source,
            target,
            IDS_BYTES,
            &serde_json::to_vec(&source_report).unwrap(),
            &serde_json::to_vec(&target_report).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn identical_stable_property_is_passed() {
        let output = run_pair(
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#7",
            )],
        );
        assert_eq!(output.gate.status, GateStatus::Pass);
        assert!(output.evidence_payload.profile_confirmed);
        assert_eq!(output.evidence_payload.counts.compared, 1);
        assert_eq!(output.evidence_payload.counts.preserved, 1);
    }

    #[test]
    fn changed_required_value_is_a_preservation_failure() {
        let output = run_pair(
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            vec![observation(
                "0000000000000000000000",
                Some("Panel 002"),
                AdapterPropertyOutcome::Changed,
                "#8",
            )],
        );
        assert_eq!(output.gate.status, GateStatus::Fail);
        assert_eq!(output.evidence_payload.counts.changed, 1);
        assert_eq!(
            output.evidence_payload.findings[0].code,
            "REQUIRED_VALUE_CHANGED"
        );
    }

    #[test]
    fn populated_source_value_missing_from_target_is_a_loss() {
        let output = run_pair(
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            vec![observation(
                "0000000000000000000000",
                None,
                AdapterPropertyOutcome::Missing,
                "#4",
            )],
        );
        assert_eq!(output.gate.status, GateStatus::Fail);
        assert_eq!(output.evidence_payload.counts.lost, 1);
    }

    #[test]
    fn source_object_without_a_target_counterpart_is_dropped() {
        let output = run_pair(
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            vec![observation(
                "0000000000000000000001",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#2",
            )],
        );
        assert_eq!(output.gate.status, GateStatus::Fail);
        assert_eq!(output.evidence_payload.counts.dropped, 1);
        assert_eq!(output.evidence_payload.counts.added, 1);
    }

    #[test]
    fn new_target_object_is_recorded_without_failing_preservation() {
        let output = run_pair(
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            vec![
                observation(
                    "0000000000000000000000",
                    Some("Panel 001"),
                    AdapterPropertyOutcome::Preserved,
                    "#2",
                ),
                observation(
                    "0000000000000000000001",
                    Some("Panel 001"),
                    AdapterPropertyOutcome::Preserved,
                    "#3",
                ),
            ],
        );
        assert_eq!(output.gate.status, GateStatus::Pass);
        assert_eq!(output.evidence_payload.counts.preserved, 1);
        assert_eq!(output.evidence_payload.counts.added, 1);
    }

    #[test]
    fn equal_absence_does_not_confuse_preservation_with_ids_conformance() {
        let output = run_pair(
            vec![observation(
                "0000000000000000000000",
                None,
                AdapterPropertyOutcome::Missing,
                "#1",
            )],
            vec![observation(
                "0000000000000000000000",
                None,
                AdapterPropertyOutcome::Missing,
                "#3",
            )],
        );
        assert_eq!(output.gate.status, GateStatus::Pass);
        assert_eq!(output.evidence_payload.counts.preserved, 1);
    }

    #[test]
    fn invalid_identity_cannot_produce_a_false_pass_or_false_drop() {
        let source = b"source IFC";
        let target = b"target IFC";
        let source_report = report(
            source,
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            AdapterDecision::SubsetConformant,
            Some("IFC4"),
            IDS_BYTES,
        );
        let target_report = report(
            target,
            vec![observation(
                "STEP:#1",
                Some("Panel 001"),
                AdapterPropertyOutcome::NotEvaluated,
                "#1",
            )],
            AdapterDecision::Blocked,
            Some("IFC4"),
            IDS_BYTES,
        );
        let output = evaluate(
            source,
            target,
            IDS_BYTES,
            &serde_json::to_vec(&source_report).unwrap(),
            &serde_json::to_vec(&target_report).unwrap(),
        )
        .unwrap();
        assert_eq!(output.gate.status, GateStatus::Unresolved);
        assert_eq!(output.evidence_payload.counts.dropped, 0);
        assert!(output.evidence_payload.counts.unresolved > 0);
    }

    #[test]
    fn duplicate_global_ids_are_unresolved_not_silently_collapsed() {
        let output = run_pair(
            vec![
                observation(
                    "0000000000000000000000",
                    Some("Panel 001"),
                    AdapterPropertyOutcome::NotEvaluated,
                    "#1",
                ),
                observation(
                    "0000000000000000000000",
                    Some("Panel 002"),
                    AdapterPropertyOutcome::NotEvaluated,
                    "#2",
                ),
            ],
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#7",
            )],
        );
        assert_eq!(output.gate.status, GateStatus::Unresolved);
        assert!(
            output
                .evidence_payload
                .findings
                .iter()
                .any(|f| f.code == "DUPLICATE_GLOBAL_ID")
        );
    }

    #[test]
    fn wrong_artifact_and_ids_digests_are_rejected() {
        let source = b"source IFC";
        let target = b"target IFC";
        let source_report = report(
            b"not source IFC",
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            AdapterDecision::SubsetConformant,
            Some("IFC4"),
            IDS_BYTES,
        );
        let target_report = report(
            target,
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#2",
            )],
            AdapterDecision::SubsetConformant,
            Some("IFC4"),
            IDS_BYTES,
        );
        assert!(
            evaluate(
                source,
                target,
                IDS_BYTES,
                &serde_json::to_vec(&source_report).unwrap(),
                &serde_json::to_vec(&target_report).unwrap()
            )
            .unwrap_err()
            .starts_with("ADAPTER_MODEL_DIGEST_MISMATCH")
        );

        let bad_ids = b"different IDS bytes";
        let source_report = report(
            source,
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            AdapterDecision::SubsetConformant,
            Some("IFC4"),
            bad_ids,
        );
        assert!(
            evaluate(
                source,
                target,
                IDS_BYTES,
                &serde_json::to_vec(&source_report).unwrap(),
                &serde_json::to_vec(&target_report).unwrap()
            )
            .unwrap_err()
            .starts_with("ADAPTER_IDS_DIGEST_MISMATCH")
        );
    }

    #[test]
    fn malformed_or_tampered_semantic_digest_is_rejected() {
        let source = b"source IFC";
        let target = b"target IFC";
        let mut source_report = report(
            source,
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            AdapterDecision::SubsetConformant,
            Some("IFC4"),
            IDS_BYTES,
        );
        source_report.observations[0].observed_value = Some("altered".into());
        let target_report = report(
            target,
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#2",
            )],
            AdapterDecision::SubsetConformant,
            Some("IFC4"),
            IDS_BYTES,
        );
        assert!(
            evaluate(
                source,
                target,
                IDS_BYTES,
                &serde_json::to_vec(&source_report).unwrap(),
                &serde_json::to_vec(&target_report).unwrap()
            )
            .unwrap_err()
            .starts_with("ADAPTER_SEMANTIC_DIGEST_MISMATCH")
        );
    }

    #[test]
    fn permutation_of_step_ids_does_not_change_semantic_delta_findings() {
        let first = run_pair(
            vec![
                observation(
                    "0000000000000000000000",
                    Some("Panel 001"),
                    AdapterPropertyOutcome::Preserved,
                    "#1",
                ),
                observation(
                    "0000000000000000000001",
                    Some("Panel 003"),
                    AdapterPropertyOutcome::Preserved,
                    "#2",
                ),
            ],
            vec![
                observation(
                    "0000000000000000000000",
                    Some("Panel 002"),
                    AdapterPropertyOutcome::Changed,
                    "#8",
                ),
                observation(
                    "0000000000000000000001",
                    Some("Panel 003"),
                    AdapterPropertyOutcome::Preserved,
                    "#7",
                ),
            ],
        );
        let second = run_pair(
            vec![
                observation(
                    "0000000000000000000001",
                    Some("Panel 003"),
                    AdapterPropertyOutcome::Preserved,
                    "#1",
                ),
                observation(
                    "0000000000000000000000",
                    Some("Panel 001"),
                    AdapterPropertyOutcome::Preserved,
                    "#2",
                ),
            ],
            vec![
                observation(
                    "0000000000000000000001",
                    Some("Panel 003"),
                    AdapterPropertyOutcome::Preserved,
                    "#8",
                ),
                observation(
                    "0000000000000000000000",
                    Some("Panel 002"),
                    AdapterPropertyOutcome::Changed,
                    "#7",
                ),
            ],
        );
        assert_eq!(first.gate.status, second.gate.status);
        assert_eq!(
            first.evidence_payload.counts.compared,
            second.evidence_payload.counts.compared
        );
        assert_eq!(
            first.evidence_payload.counts.changed,
            second.evidence_payload.counts.changed
        );
        assert_eq!(
            first.evidence_payload.findings,
            second.evidence_payload.findings
        );
    }

    #[test]
    fn unconfirmed_ifc_schema_is_unresolved() {
        let source = b"source IFC";
        let target = b"target IFC";
        let source_report = report(
            source,
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            AdapterDecision::SubsetConformant,
            Some("IFC2X3"),
            IDS_BYTES,
        );
        let target_report = report(
            target,
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#2",
            )],
            AdapterDecision::SubsetConformant,
            Some("IFC4"),
            IDS_BYTES,
        );
        let output = evaluate(
            source,
            target,
            IDS_BYTES,
            &serde_json::to_vec(&source_report).unwrap(),
            &serde_json::to_vec(&target_report).unwrap(),
        )
        .unwrap();
        assert_eq!(output.gate.status, GateStatus::Unresolved);
        assert!(!output.evidence_payload.profile_confirmed);
        assert!(!output.evidence_payload.comparison_coverage_complete);
    }

    #[test]
    fn output_embeds_hashed_payload_in_standard_gate_shape() {
        let output = run_pair(
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#1",
            )],
            vec![observation(
                "0000000000000000000000",
                Some("Panel 001"),
                AdapterPropertyOutcome::Preserved,
                "#2",
            )],
        );
        let payload_bytes = serde_json::to_vec(&output.evidence_payload).unwrap();
        assert_eq!(output.gate.evidence[0].sha256, sha256_hex(&payload_bytes));
        assert_eq!(output.gate.evidence[0].locator, PAYLOAD_POINTER);
        assert_eq!(output.gate.gate_id, "exchange_semantic_preservation");
    }
}
