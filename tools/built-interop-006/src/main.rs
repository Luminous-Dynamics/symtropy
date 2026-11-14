// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed, fail-closed assembly of eight-gate built-environment evidence reports.
//! Producer JSON is deserialized into closed Rust structs; logs are never scraped.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

const REPORT_SCHEMA_VERSION: &str = "1.0.0";
const REPORT_BENCHMARK_ID: &str = "BUILT-INTEROP-004";
const IDS_STANDARD_VERSION: &str = "1.0";
const IFC_SCHEMA_PROFILE_ID: &str = "DECLARED-IFC-SCHEMA-SUPPORTED-RULES-v0.1";
const IFC_SCHEMA_POLICY_ID: &str = "BUILT-INTEROP-005/v0.1;step=ifc-step-0.5.0-strict;model=ifc-model-0.3.1;schema=ifc-schema-0.3.2-ifc4-only;validator=ifc-validate-0.6.0;schema-selection=declared-only;errors=FAIL;evaluation-errors-unsupported-truncated=UNRESOLVED;findings=stable-sorted;lock-sha256=cdf5e081cacba012ed1e52246b399f1062934b0e71eeeb5f6743f4f81a279c7c";
const IFC_SCHEMA_TOOL_VERSION: &str =
    "ifc-validate=0.6.0;ifc-schema=0.3.2;ifc-model=0.3.1;ifc-step=0.5.0";
const DELTA_PROFILE_ID: &str = "IFC4/IDS1.0/IFCWALL.Name/simpleValue-v0.1";
const DELTA_POLICY_ID: &str = "BUILT-INTEROP-007/v0.1;match=valid-GlobalId+requirement-id+property-key;profile=IFC4-IDS1.0-IFCWALL.Name;changed-lost-dropped=FAIL;unstable-incomplete-unsupported=UNRESOLVED;additions=record-only;semantic-digest=verified";
const DELTA_TOOL_VERSION: &str = "0.1.0";
const PAYLOAD_POINTER: &str = "report-json-pointer:/evidence_payload";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum GateId {
    ArtifactIntegrity,
    StepSyntax,
    IfcSchemaConformance,
    IdsXsdValidity,
    IdsSemanticValidity,
    IdsModelConformance,
    ExchangeSemanticPreservation,
    EngineeringReleaseQualification,
}

impl GateId {
    const ALL: [Self; 8] = [
        Self::ArtifactIntegrity,
        Self::StepSyntax,
        Self::IfcSchemaConformance,
        Self::IdsXsdValidity,
        Self::IdsSemanticValidity,
        Self::IdsModelConformance,
        Self::ExchangeSemanticPreservation,
        Self::EngineeringReleaseQualification,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::ArtifactIntegrity => "artifact_integrity",
            Self::StepSyntax => "step_syntax",
            Self::IfcSchemaConformance => "ifc_schema_conformance",
            Self::IdsXsdValidity => "ids_xsd_validity",
            Self::IdsSemanticValidity => "ids_semantic_validity",
            Self::IdsModelConformance => "ids_model_conformance",
            Self::ExchangeSemanticPreservation => "exchange_semantic_preservation",
            Self::EngineeringReleaseQualification => "engineering_release_qualification",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum GateStatus {
    Pass,
    Fail,
    Unresolved,
    NotRun,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum OverallDecision {
    Qualified,
    Blocked,
    Unqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ArtifactRole {
    SourceIfc,
    RequirementsIds,
    TargetIfc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceKind {
    ToolOutput,
    ValidatorReport,
    MutationTest,
    ExternalAttestation,
    ArtifactDigest,
    ReviewRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolIdentity {
    name: String,
    version: String,
    config_sha256: String,
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
    gate_id: GateId,
    mandatory: bool,
    status: GateStatus,
    claim_scope: String,
    limitations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool: Option<ToolIdentity>,
    evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    role: ArtifactRole,
    sha256: String,
    byte_length: usize,
    media_type: String,
    locator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationProfile {
    ifc_schema: String,
    ids_version: String,
    supported_subset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evaluator {
    name: String,
    version: String,
    source_commit_sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Report {
    schema_version: String,
    benchmark_id: String,
    run_id: String,
    reported_at: String,
    evaluation_profile: EvaluationProfile,
    evaluator: Evaluator,
    artifacts: Vec<Artifact>,
    gates: Vec<Gate>,
    overall_decision: OverallDecision,
    scope_statement: String,
    limitations: Vec<String>,
}

#[derive(Debug)]
struct ProducerGate {
    gate: Gate,
    path: PathBuf,
    evidence_payload: Option<Value>,
}

const BUILT_INTEROP_002_ID: &str = "BUILT-INTEROP-002-ifc-ids-v0.1";
const BUILT_INTEROP_002_PROFILE: &str = "IFC4/IDS1.0/IFCWALL.Name/simpleValue-v0.1";
const BUILT_INTEROP_002_MAPPING_POLICY: &str = "BUILT-INTEROP-006/v0.2;adapter=BUILT-INTEROP-002-ifc-ids-v0.1;binding=target-ifc+requirements-ids-digest;pass=SubsetConformant-all-Preserved;fail=Blocked-with-Changed-or-Missing;otherwise=UNRESOLVED";

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
struct BuiltInterop002Report {
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

#[derive(Debug)]
struct CliArgs {
    source_ifc: PathBuf,
    requirements_ids: PathBuf,
    target_ifc: PathBuf,
    ifc_schema: String,
    supported_subset: String,
    run_id: String,
    reported_at: String,
    source_commit_sha: String,
    evaluator_name: String,
    gate_json: Vec<PathBuf>,
    output: Option<PathBuf>,
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

fn is_run_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-'))
}

/// Validate the RFC-3339 report timestamp without bringing in a date runtime.
fn is_rfc3339(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() < 20
        || !b[0..4].iter().all(u8::is_ascii_digit)
        || b[4] != b'-'
        || !b[5..7].iter().all(u8::is_ascii_digit)
        || b[7] != b'-'
        || !b[8..10].iter().all(u8::is_ascii_digit)
        || b[10] != b'T'
        || !b[11..13].iter().all(u8::is_ascii_digit)
        || b[13] != b':'
        || !b[14..16].iter().all(u8::is_ascii_digit)
        || b[16] != b':'
        || !b[17..19].iter().all(u8::is_ascii_digit)
    {
        return false;
    }

    let number =
        |start: usize, end: usize| -> u32 { value[start..end].parse().unwrap_or(u32::MAX) };
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let hour = number(11, 13);
    let minute = number(14, 16);
    let second = number(17, 19);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    if day > days_in_month {
        return false;
    }

    let mut rest = &value[19..];
    if let Some(after_dot) = rest.strip_prefix('.') {
        let fraction_len = after_dot.bytes().take_while(u8::is_ascii_digit).count();
        if fraction_len == 0 {
            return false;
        }
        rest = &after_dot[fraction_len..];
    }
    if rest == "Z" {
        return true;
    }
    if rest.len() != 6
        || !matches!(rest.as_bytes()[0], b'+' | b'-')
        || rest.as_bytes()[3] != b':'
        || !rest.as_bytes()[1..3].iter().all(u8::is_ascii_digit)
        || !rest.as_bytes()[4..6].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let offset_hour = rest[1..3].parse::<u32>().unwrap_or(u32::MAX);
    let offset_minute = rest[4..6].parse::<u32>().unwrap_or(u32::MAX);
    offset_hour <= 23 && offset_minute <= 59
}

fn validate_gate(gate: &Gate) -> Result<(), String> {
    if !gate.mandatory {
        return Err(format!("GATE_NOT_MANDATORY:{}", gate.gate_id.as_str()));
    }
    if gate.claim_scope.trim().is_empty()
        || gate.limitations.is_empty()
        || gate.limitations.iter().any(|text| text.trim().is_empty())
    {
        return Err(format!(
            "GATE_SCOPE_OR_LIMITATIONS_INVALID:{}",
            gate.gate_id.as_str()
        ));
    }
    if let Some(tool) = &gate.tool
        && (tool.name.trim().is_empty()
            || tool.version.trim().is_empty()
            || !is_lower_hex_of_length(&tool.config_sha256, 64))
    {
        return Err(format!(
            "GATE_TOOL_IDENTITY_INVALID:{}",
            gate.gate_id.as_str()
        ));
    }
    let mut evidence_ids = BTreeSet::new();
    for evidence in &gate.evidence {
        if evidence.evidence_id.trim().is_empty()
            || evidence.locator.trim().is_empty()
            || !is_lower_hex_of_length(&evidence.sha256, 64)
        {
            return Err(format!("GATE_EVIDENCE_INVALID:{}", gate.gate_id.as_str()));
        }
        if !evidence_ids.insert(evidence.evidence_id.as_str()) {
            return Err(format!("DUPLICATE_EVIDENCE_ID:{}", evidence.evidence_id));
        }
    }
    match gate.status {
        GateStatus::Pass | GateStatus::Fail => {
            if gate.tool.is_none() || gate.evidence.is_empty() {
                return Err(format!(
                    "EXECUTED_GATE_REQUIRES_TOOL_AND_EVIDENCE:{}",
                    gate.gate_id.as_str()
                ));
            }
        }
        GateStatus::NotRun => {
            if gate.tool.is_some() || !gate.evidence.is_empty() {
                return Err(format!(
                    "NOT_RUN_GATE_HAS_EXECUTION_CLAIM:{}",
                    gate.gate_id.as_str()
                ));
            }
        }
        GateStatus::Unresolved => {}
    }
    Ok(())
}

fn not_run_gate(gate_id: GateId) -> Gate {
    Gate {
        gate_id,
        mandatory: true,
        status: GateStatus::NotRun,
        claim_scope: format!("{} was not evaluated in this run.", gate_id.as_str()),
        limitations: vec![
            "No producer supplied a result for this gate.".into(),
            "NOT_RUN is not evidence of conformance and cannot qualify the exchange.".into(),
        ],
        tool: None,
        evidence: Vec::new(),
    }
}

fn artifact(role: ArtifactRole, locator: &Path, media_type: &str, bytes: &[u8]) -> Artifact {
    Artifact {
        role,
        sha256: sha256_hex(bytes),
        byte_length: bytes.len(),
        media_type: media_type.into(),
        locator: locator.display().to_string(),
    }
}

fn overall_decision(gates: &[Gate]) -> OverallDecision {
    if gates.iter().any(|gate| gate.status == GateStatus::Fail) {
        OverallDecision::Blocked
    } else if gates.len() == GateId::ALL.len()
        && gates.iter().all(|gate| gate.status == GateStatus::Pass)
    {
        OverallDecision::Qualified
    } else {
        OverallDecision::Unqualified
    }
}

fn assemble_report(
    args: &CliArgs,
    source_bytes: &[u8],
    ids_bytes: &[u8],
    target_bytes: &[u8],
    producer_gates: Vec<Gate>,
) -> Result<Report, String> {
    if !is_run_id(&args.run_id) {
        return Err("INVALID_RUN_ID".into());
    }
    if !is_rfc3339(&args.reported_at) {
        return Err("INVALID_RFC3339_TIMESTAMP".into());
    }
    if !is_lower_hex_of_length(&args.source_commit_sha, 40) {
        return Err("INVALID_EVALUATOR_SOURCE_COMMIT_SHA".into());
    }
    if args.ifc_schema.trim().is_empty()
        || args.supported_subset.trim().is_empty()
        || args.evaluator_name.trim().is_empty()
    {
        return Err("EVALUATION_PROFILE_OR_EVALUATOR_REQUIRED".into());
    }

    let mut by_id = BTreeMap::new();
    for gate in producer_gates {
        validate_gate(&gate)?;
        if by_id.insert(gate.gate_id, gate).is_some() {
            return Err("DUPLICATE_GATE_ID".into());
        }
    }

    let mut gates = Vec::with_capacity(GateId::ALL.len());
    let mut all_evidence_ids = BTreeSet::new();
    for gate_id in GateId::ALL {
        let mut gate = by_id
            .remove(&gate_id)
            .unwrap_or_else(|| not_run_gate(gate_id));
        gate.evidence
            .sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
        for evidence in &gate.evidence {
            if !all_evidence_ids.insert(evidence.evidence_id.clone()) {
                return Err(format!(
                    "DUPLICATE_EVIDENCE_ID_ACROSS_GATES:{}",
                    evidence.evidence_id
                ));
            }
        }
        validate_gate(&gate)?;
        gates.push(gate);
    }
    if !by_id.is_empty() {
        return Err("UNKNOWN_GATE_ID".into());
    }

    let artifacts = vec![
        artifact(
            ArtifactRole::SourceIfc,
            &args.source_ifc,
            "model/ifc",
            source_bytes,
        ),
        artifact(
            ArtifactRole::RequirementsIds,
            &args.requirements_ids,
            "application/ids+xml",
            ids_bytes,
        ),
        artifact(
            ArtifactRole::TargetIfc,
            &args.target_ifc,
            "model/ifc",
            target_bytes,
        ),
    ];
    let decision = overall_decision(&gates);
    let report = Report {
        schema_version: REPORT_SCHEMA_VERSION.into(),
        benchmark_id: REPORT_BENCHMARK_ID.into(),
        run_id: args.run_id.clone(),
        reported_at: args.reported_at.clone(),
        evaluation_profile: EvaluationProfile {
            ifc_schema: args.ifc_schema.clone(),
            ids_version: IDS_STANDARD_VERSION.into(),
            supported_subset: args.supported_subset.clone(),
        },
        evaluator: Evaluator {
            name: args.evaluator_name.clone(),
            version: env!("CARGO_PKG_VERSION").into(),
            source_commit_sha: args.source_commit_sha.clone(),
        },
        artifacts,
        gates,
        overall_decision: decision,
        scope_statement: "Assembled evidence report for the declared evaluation profile; no missing gate is treated as a pass.".into(),
        limitations: vec![
            "Artifact SHA-256 values identify the bytes read by this process; they do not prove an external expected digest or chain of custody.".into(),
            "Producer gate evidence is structurally checked but its payload, signer and underlying tool soundness are not authenticated by this assembler.".into(),
            "QUALIFIED is a report-contract decision only; engineering-release claims remain scoped to the evidence in the required gate.".into(),
        ],
    };
    validate_report(&report)?;
    Ok(report)
}

fn validate_report(report: &Report) -> Result<(), String> {
    if report.schema_version != REPORT_SCHEMA_VERSION
        || report.benchmark_id != REPORT_BENCHMARK_ID
        || !is_run_id(&report.run_id)
        || !is_rfc3339(&report.reported_at)
        || report.evaluation_profile.ifc_schema.trim().is_empty()
        || report.evaluation_profile.ids_version != IDS_STANDARD_VERSION
        || report.evaluation_profile.supported_subset.trim().is_empty()
        || report.evaluator.name.trim().is_empty()
        || report.evaluator.version.trim().is_empty()
        || !is_lower_hex_of_length(&report.evaluator.source_commit_sha, 40)
        || report.scope_statement.trim().is_empty()
        || report.limitations.is_empty()
        || report.limitations.iter().any(|text| text.trim().is_empty())
    {
        return Err("REPORT_ENVELOPE_INVALID".into());
    }

    let expected_roles = [
        ArtifactRole::SourceIfc,
        ArtifactRole::RequirementsIds,
        ArtifactRole::TargetIfc,
    ];
    if report.artifacts.len() != expected_roles.len()
        || report
            .artifacts
            .iter()
            .map(|a| a.role)
            .collect::<Vec<_>>()
            .as_slice()
            != expected_roles.as_slice()
        || report.artifacts.iter().any(|a| {
            !is_lower_hex_of_length(&a.sha256, 64)
                || a.media_type.trim().is_empty()
                || a.locator.trim().is_empty()
        })
    {
        return Err("ARTIFACT_SET_INVALID".into());
    }

    if report.gates.len() != GateId::ALL.len()
        || report
            .gates
            .iter()
            .map(|gate| gate.gate_id)
            .collect::<Vec<_>>()
            .as_slice()
            != GateId::ALL.as_slice()
    {
        return Err("GATE_SET_INVALID".into());
    }
    let mut evidence_ids = BTreeSet::new();
    for gate in &report.gates {
        validate_gate(gate)?;
        for evidence in &gate.evidence {
            if !evidence_ids.insert(evidence.evidence_id.as_str()) {
                return Err(format!("DUPLICATE_EVIDENCE_ID:{}", evidence.evidence_id));
            }
        }
    }
    if report.overall_decision != overall_decision(&report.gates) {
        return Err("OVERALL_DECISION_MISMATCH".into());
    }
    Ok(())
}

fn adapt_built_interop_002_report(
    report: &BuiltInterop002Report,
    raw_report_bytes: &[u8],
    report_path: &Path,
    target_ifc_bytes: &[u8],
    requirements_ids_bytes: &[u8],
) -> Result<Gate, String> {
    if report.schema_version != "0.1.0"
        || report.benchmark_id != BUILT_INTEROP_002_ID
        || report.supported_profile != BUILT_INTEROP_002_PROFILE
    {
        return Err("BUILT_INTEROP_002_PROFILE_MISMATCH".into());
    }

    let target_sha256 = sha256_hex(target_ifc_bytes);
    let ids_sha256 = sha256_hex(requirements_ids_bytes);
    if report.ifc_input_sha256 != target_sha256 || report.ids_input_sha256 != ids_sha256 {
        return Err("BUILT_INTEROP_002_ARTIFACT_DIGEST_MISMATCH".into());
    }
    if !is_lower_hex_of_length(&report.semantic_result_sha256, 64) {
        return Err("BUILT_INTEROP_002_SEMANTIC_DIGEST_INVALID".into());
    }
    let semantic_digest = AdapterSemanticDigest {
        supported_profile: &report.supported_profile,
        decision: report.decision,
        ifc_schema_token: &report.ifc_schema_token,
        ids_version_evidence: &report.ids_version_evidence,
        parsed_entity_count: report.parsed_entity_count,
        applicable_wall_count: report.applicable_wall_count,
        observations: &report.observations,
        diagnostics: &report.diagnostics,
    };
    let semantic_bytes = serde_json::to_vec(&semantic_digest)
        .map_err(|error| format!("BUILT_INTEROP_002_SEMANTIC_SERIALIZATION_FAILED:{error}"))?;
    if sha256_hex(&semantic_bytes) != report.semantic_result_sha256 {
        return Err("BUILT_INTEROP_002_SEMANTIC_DIGEST_MISMATCH".into());
    }

    let profile_confirmed = report.ifc_schema_token.as_deref() == Some("IFC4")
        && report.ids_version_evidence == "Declared:IDS1.0";
    let property_failure = report.observations.iter().any(|observation| {
        matches!(
            observation.outcome,
            AdapterPropertyOutcome::Changed | AdapterPropertyOutcome::Missing
        )
    });
    let status = if !profile_confirmed {
        GateStatus::Unresolved
    } else {
        match report.decision {
            AdapterDecision::SubsetConformant => {
                let complete_observation_set = report
                    .applicable_wall_count
                    .is_some_and(|count| count > 0 && count == report.observations.len());
                let every_observation_preserved = report
                    .observations
                    .iter()
                    .all(|observation| observation.outcome == AdapterPropertyOutcome::Preserved);
                if complete_observation_set
                    && every_observation_preserved
                    && report.diagnostics.is_empty()
                {
                    GateStatus::Pass
                } else {
                    GateStatus::Unresolved
                }
            }
            AdapterDecision::Blocked if property_failure => GateStatus::Fail,
            AdapterDecision::Blocked | AdapterDecision::Unresolved => GateStatus::Unresolved,
        }
    };

    let report_sha256 = sha256_hex(raw_report_bytes);
    let gate = Gate {
        gate_id: GateId::IdsModelConformance,
        mandatory: true,
        status,
        claim_scope: report.supported_profile.clone(),
        limitations: vec![
            "This mapping implements only IFC4 / IDS 1.0 / required unbounded IFCWALL applicability / literal IFCWALL.Name.".into(),
            "A Blocked adapter result is FAIL only when a typed observation proves a required Name is changed or missing; identity/profile/unsupported blockers remain UNRESOLVED.".into(),
            "The assembler hashes the exact producer report bytes; it does not independently recompute the adapter semantic_result_sha256 or prove complete IDS conformance.".into(),
        ],
        tool: Some(ToolIdentity {
            name: "built-interop-ifc-ids".into(),
            version: report.schema_version.clone(),
            config_sha256: sha256_hex(BUILT_INTEROP_002_MAPPING_POLICY.as_bytes()),
        }),
        evidence: vec![Evidence {
            evidence_id: format!("built-interop-002-report-{report_sha256}"),
            kind: EvidenceKind::ValidatorReport,
            sha256: report_sha256,
            locator: report_path.display().to_string(),
        }],
    };
    validate_gate(&gate)?;
    Ok(gate)
}

fn parse_gate_file(
    path: &Path,
    target_ifc_bytes: &[u8],
    requirements_ids_bytes: &[u8],
) -> Result<ProducerGate, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("CANNOT_READ_GATE_JSON:{}:{error}", path.display()))?;
    let json: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("INVALID_GATE_JSON:{}:{error}", path.display()))?;

    // This is an explicit typed converter for one pinned producer schema,
    // not heuristic parsing of diagnostics or console output.
    if json.get("benchmark_id").and_then(Value::as_str) == Some(BUILT_INTEROP_002_ID)
        && json.get("gate").is_none()
    {
        let adapter_report: BuiltInterop002Report =
            serde_json::from_value(json).map_err(|error| {
                format!(
                    "INVALID_BUILT_INTEROP_002_REPORT:{}:{error}",
                    path.display()
                )
            })?;
        let gate = adapt_built_interop_002_report(
            &adapter_report,
            &bytes,
            path,
            target_ifc_bytes,
            requirements_ids_bytes,
        )?;
        return Ok(ProducerGate {
            gate,
            path: path.to_path_buf(),
            evidence_payload: None,
        });
    }

    let is_envelope = json.get("gate").is_some();
    let gate_value = json.get("gate").unwrap_or(&json).clone();
    let evidence_payload = if is_envelope {
        json.get("evidence_payload").cloned()
    } else {
        None
    };
    let mut gate: Gate = serde_json::from_value(gate_value)
        .map_err(|error| format!("INVALID_TYPED_GATE:{}:{error}", path.display()))?;
    validate_gate(&gate)?;
    gate.evidence
        .sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
    Ok(ProducerGate {
        gate,
        path: path.to_path_buf(),
        evidence_payload,
    })
}

/// Verify an embedded producer payload against the evidence digest and, for
/// BUILT-INTEROP-005, against the exact target IFC bytes and declared profile.
fn verify_embedded_payload(
    input: &ProducerGate,
    source_ifc_bytes: &[u8],
    target_ifc_bytes: &[u8],
    requirements_ids_bytes: &[u8],
    declared_schema: &str,
) -> Result<(), String> {
    let points_to_embedded_payload = input
        .gate
        .evidence
        .iter()
        .any(|evidence| evidence.locator == PAYLOAD_POINTER);
    let Some(payload) = &input.evidence_payload else {
        if points_to_embedded_payload {
            return Err("EVIDENCE_PAYLOAD_MISSING".into());
        }
        return match input.gate.gate_id {
            GateId::IfcSchemaConformance => Err("IFC_SCHEMA_EVIDENCE_PAYLOAD_REQUIRED".into()),
            GateId::ExchangeSemanticPreservation => Err("DELTA_EVIDENCE_PAYLOAD_REQUIRED".into()),
            _ => Ok(()),
        };
    };

    let pointer_evidence = input
        .gate
        .evidence
        .iter()
        .filter(|evidence| evidence.locator == PAYLOAD_POINTER)
        .collect::<Vec<_>>();
    if pointer_evidence.is_empty() {
        return Err("EVIDENCE_PAYLOAD_UNREFERENCED".into());
    }
    let payload_bytes = serde_json::to_vec(payload)
        .map_err(|error| format!("EVIDENCE_PAYLOAD_SERIALIZATION_FAILED:{error}"))?;
    let payload_sha256 = sha256_hex(&payload_bytes);
    if pointer_evidence
        .iter()
        .any(|evidence| evidence.sha256 != payload_sha256)
    {
        return Err("EVIDENCE_POINTER_DIGEST_MISMATCH".into());
    }

    if input.gate.gate_id == GateId::IfcSchemaConformance {
        if payload["profile_id"] != IFC_SCHEMA_PROFILE_ID {
            return Err("IFC_SCHEMA_PROFILE_ID_MISMATCH".into());
        }
        let target_sha256 = sha256_hex(target_ifc_bytes);
        if payload["input_sha256"].as_str() != Some(target_sha256.as_str())
            || payload["input_byte_length"].as_u64() != u64::try_from(target_ifc_bytes.len()).ok()
        {
            return Err("IFC_SCHEMA_EVIDENCE_ARTIFACT_MISMATCH".into());
        }
        match payload.get("declared_schema_token") {
            Some(Value::String(token)) if token == declared_schema => {}
            Some(Value::Null) if declared_schema == "UNDECLARED" => {}
            _ => return Err("IFC_SCHEMA_EVIDENCE_PROFILE_MISMATCH".into()),
        }

        let expected_config_sha256 = sha256_hex(IFC_SCHEMA_POLICY_ID.as_bytes());
        match input.gate.tool.as_ref() {
            Some(tool)
                if tool.name == "ifc-validate+ifc-schema"
                    && tool.version == IFC_SCHEMA_TOOL_VERSION
                    && tool.config_sha256 == expected_config_sha256 => {}
            _ => return Err("IFC_SCHEMA_TOOL_PROVENANCE_MISMATCH".into()),
        }
    } else if input.gate.gate_id == GateId::ExchangeSemanticPreservation {
        if payload["policy_id"] != DELTA_POLICY_ID {
            return Err("DELTA_POLICY_ID_MISMATCH".into());
        }
        if payload["source_ifc_sha256"].as_str() != Some(sha256_hex(source_ifc_bytes).as_str())
            || payload["source_ifc_byte_length"].as_u64()
                != u64::try_from(source_ifc_bytes.len()).ok()
        {
            return Err("DELTA_SOURCE_ARTIFACT_MISMATCH".into());
        }
        if payload["target_ifc_sha256"].as_str() != Some(sha256_hex(target_ifc_bytes).as_str())
            || payload["target_ifc_byte_length"].as_u64()
                != u64::try_from(target_ifc_bytes.len()).ok()
        {
            return Err("DELTA_TARGET_ARTIFACT_MISMATCH".into());
        }
        if payload["requirements_ids_sha256"].as_str()
            != Some(sha256_hex(requirements_ids_bytes).as_str())
        {
            return Err("DELTA_REQUIREMENTS_ARTIFACT_MISMATCH".into());
        }
        if payload["supported_profile"] != DELTA_PROFILE_ID {
            return Err("DELTA_SUPPORTED_PROFILE_MISMATCH".into());
        }
        let counts = payload
            .get("counts")
            .and_then(Value::as_object)
            .ok_or_else(|| "DELTA_COUNTS_INVALID".to_string())?;
        let expected_count_keys = [
            "compared",
            "preserved",
            "changed",
            "lost",
            "dropped",
            "added",
            "unresolved",
        ];
        if counts.len() != expected_count_keys.len()
            || expected_count_keys
                .iter()
                .any(|key| counts.get(*key).and_then(Value::as_u64).is_none())
        {
            return Err("DELTA_COUNTS_INVALID".into());
        }
        let compared = counts["compared"].as_u64().unwrap_or(0);
        let changed = counts["changed"].as_u64().unwrap_or(0);
        let lost = counts["lost"].as_u64().unwrap_or(0);
        let dropped = counts["dropped"].as_u64().unwrap_or(0);
        let unresolved = counts["unresolved"].as_u64().unwrap_or(0);
        let profile_confirmed = payload
            .get("profile_confirmed")
            .and_then(Value::as_bool)
            .ok_or_else(|| "DELTA_PROFILE_CONFIRMATION_MISSING".to_string())?;
        let coverage_complete = payload
            .get("comparison_coverage_complete")
            .and_then(Value::as_bool)
            .ok_or_else(|| "DELTA_COVERAGE_FIELD_MISSING".to_string())?;
        let definite_loss = changed > 0 || lost > 0 || dropped > 0;
        let reason = payload["decision_reason"].as_str().unwrap_or("");
        let state_consistent = match input.gate.status {
            GateStatus::Pass => {
                profile_confirmed
                    && coverage_complete
                    && compared > 0
                    && unresolved == 0
                    && !definite_loss
                    && reason == "COMPLETE_SUPPORTED_DELTA_WITH_NO_LOSS"
            }
            GateStatus::Fail => {
                profile_confirmed && definite_loss && reason == "DEFINITE_SOURCE_TO_TARGET_LOSS"
            }
            GateStatus::Unresolved => {
                reason == "PROFILE_IDENTITY_OR_COVERAGE_UNRESOLVED"
                    && (!definite_loss || !profile_confirmed)
            }
            GateStatus::NotRun => false,
        };
        if !state_consistent {
            return Err("DELTA_STATUS_PAYLOAD_MISMATCH".into());
        }
        let expected_config_sha256 = sha256_hex(DELTA_POLICY_ID.as_bytes());
        match input.gate.tool.as_ref() {
            Some(tool)
                if tool.name == "built-interop-semantic-delta"
                    && tool.version == DELTA_TOOL_VERSION
                    && tool.config_sha256 == expected_config_sha256 => {}
            _ => return Err("DELTA_TOOL_PROVENANCE_MISMATCH".into()),
        }
    }
    Ok(())
}

/// Replace a producer-local JSON pointer with a file-qualified pointer so the
/// assembled report can locate the original embedded payload.
fn resolve_payload_locator(input: &mut ProducerGate) -> Result<(), String> {
    if input.evidence_payload.is_none()
        || !input
            .gate
            .evidence
            .iter()
            .any(|evidence| evidence.locator == PAYLOAD_POINTER)
    {
        return Ok(());
    }
    let absolute = fs::canonicalize(&input.path).map_err(|error| {
        format!(
            "CANNOT_CANONICALIZE_GATE_SOURCE:{}:{error}",
            input.path.display()
        )
    })?;
    let raw = absolute.to_string_lossy().replace('\\', "/");
    let mut encoded = String::new();
    for byte in raw.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b':' | b'-' | b'.' | b'_' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    let file_uri = if encoded.starts_with('/') {
        format!("file://{encoded}")
    } else {
        format!("file:///{encoded}")
    };
    for evidence in &mut input.gate.evidence {
        if evidence.locator == PAYLOAD_POINTER {
            evidence.locator = format!("{file_uri}#/evidence_payload");
        }
    }
    Ok(())
}

fn parse_args() -> Result<Option<CliArgs>, String> {
    let mut raw_args = env::args().skip(1);
    let first = raw_args.next();
    if matches!(first.as_deref(), Some("--help" | "-h")) {
        return Ok(None);
    }
    let mut args = first.into_iter().chain(raw_args);
    let mut singleton = BTreeMap::<String, String>::new();
    let mut gate_json = Vec::new();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--gate-json" => {
                let value = args.next().ok_or("MISSING_VALUE:--gate-json")?;
                gate_json.push(PathBuf::from(value));
            }
            "--source-ifc"
            | "--ids"
            | "--target-ifc"
            | "--ifc-schema"
            | "--supported-subset"
            | "--run-id"
            | "--reported-at"
            | "--source-commit-sha"
            | "--evaluator-name"
            | "--output" => {
                let value = args.next().ok_or_else(|| format!("MISSING_VALUE:{flag}"))?;
                if singleton.insert(flag.clone(), value).is_some() {
                    return Err(format!("DUPLICATE_ARGUMENT:{flag}"));
                }
            }
            _ => return Err(format!("UNKNOWN_ARGUMENT:{flag}")),
        }
    }
    let take = |key: &str| -> Result<String, String> {
        singleton
            .get(key)
            .cloned()
            .ok_or_else(|| format!("MISSING_REQUIRED_ARGUMENT:{key}"))
    };
    Ok(Some(CliArgs {
        source_ifc: PathBuf::from(take("--source-ifc")?),
        requirements_ids: PathBuf::from(take("--ids")?),
        target_ifc: PathBuf::from(take("--target-ifc")?),
        ifc_schema: take("--ifc-schema")?,
        supported_subset: take("--supported-subset")?,
        run_id: take("--run-id")?,
        reported_at: take("--reported-at")?,
        source_commit_sha: take("--source-commit-sha")?,
        evaluator_name: take("--evaluator-name")?,
        gate_json,
        output: singleton.get("--output").map(PathBuf::from),
    }))
}

fn usage() -> &'static str {
    "Usage: built-interop-report-assembler --source-ifc PATH --ids PATH --target-ifc PATH --ifc-schema TOKEN --supported-subset PROFILE --run-id ID --reported-at RFC3339 --source-commit-sha HEX40 --evaluator-name NAME [--gate-json PATH ...] [--output PATH]\nMissing gates are emitted as NOT_RUN. Each --gate-json accepts a typed gate object or an envelope containing a gate object."
}

fn run() -> Result<i32, String> {
    let Some(args) = parse_args()? else {
        println!("{}", usage());
        return Ok(0);
    };
    let source_bytes = fs::read(&args.source_ifc).map_err(|error| {
        format!(
            "CANNOT_READ_SOURCE_IFC:{}:{error}",
            args.source_ifc.display()
        )
    })?;
    let ids_bytes = fs::read(&args.requirements_ids).map_err(|error| {
        format!(
            "CANNOT_READ_IDS:{}:{error}",
            args.requirements_ids.display()
        )
    })?;
    let target_bytes = fs::read(&args.target_ifc).map_err(|error| {
        format!(
            "CANNOT_READ_TARGET_IFC:{}:{error}",
            args.target_ifc.display()
        )
    })?;
    let mut producer_gates = Vec::new();
    for gate_path in &args.gate_json {
        let mut input = parse_gate_file(gate_path, &target_bytes, &ids_bytes)?;
        verify_embedded_payload(
            &input,
            &source_bytes,
            &target_bytes,
            &ids_bytes,
            &args.ifc_schema,
        )?;
        resolve_payload_locator(&mut input)?;
        producer_gates.push(input.gate);
    }

    let report = assemble_report(
        &args,
        &source_bytes,
        &ids_bytes,
        &target_bytes,
        producer_gates,
    )?;
    let json = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("REPORT_SERIALIZATION_FAILED:{error}"))?;
    if let Some(output) = &args.output {
        fs::write(output, &json)
            .map_err(|error| format!("CANNOT_WRITE_OUTPUT:{}:{error}", output.display()))?;
    }
    println!("{}", String::from_utf8_lossy(&json));
    Ok(match report.overall_decision {
        OverallDecision::Qualified => 0,
        OverallDecision::Blocked => 1,
        OverallDecision::Unqualified => 2,
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

    fn fixture_args() -> CliArgs {
        CliArgs {
            source_ifc: PathBuf::from("source.ifc"),
            requirements_ids: PathBuf::from("requirements.ids"),
            target_ifc: PathBuf::from("target.ifc"),
            ifc_schema: "IFC4".into(),
            supported_subset: "synthetic-only-v0.1".into(),
            run_id: "contract-test:0001".into(),
            reported_at: "2026-10-18T20:00:00Z".into(),
            source_commit_sha: "a".repeat(40),
            evaluator_name: "built-interop-contract-test".into(),
            gate_json: Vec::new(),
            output: None,
        }
    }

    fn producer_gate(gate_id: GateId, status: GateStatus, evidence_id: &str) -> Gate {
        let executed = matches!(status, GateStatus::Pass | GateStatus::Fail);
        Gate {
            gate_id,
            mandatory: true,
            status,
            claim_scope: "synthetic gate contract test only".into(),
            limitations: vec!["No real IFC or engineering claim.".into()],
            tool: executed.then(|| ToolIdentity {
                name: "synthetic-producer".into(),
                version: "1.0.0".into(),
                config_sha256: "b".repeat(64),
            }),
            evidence: if executed {
                vec![Evidence {
                    evidence_id: evidence_id.into(),
                    kind: EvidenceKind::MutationTest,
                    sha256: "c".repeat(64),
                    locator: format!("fixture://evidence/{evidence_id}"),
                }]
            } else {
                Vec::new()
            },
        }
    }

    fn adapter_observation(outcome: AdapterPropertyOutcome) -> AdapterObservation {
        AdapterObservation {
            requirement_id: "walls::attribute::Name".into(),
            source_object_id: "0000000000000000000000".into(),
            source_step_id: "#1".into(),
            source_entity_type: "IFCWALL".into(),
            source_attribute_index: 2,
            source_attribute_path: "IFCWALL.Name".into(),
            property_key: "Name".into(),
            expected_value: "Panel 001".into(),
            observed_value: (outcome == AdapterPropertyOutcome::Preserved)
                .then(|| "Panel 001".into()),
            outcome,
            diagnostic_code: match outcome {
                AdapterPropertyOutcome::Preserved => None,
                AdapterPropertyOutcome::Changed => Some("CHANGED_TARGET_PROPERTY".into()),
                AdapterPropertyOutcome::Missing => Some("MISSING_TARGET_PROPERTY".into()),
                AdapterPropertyOutcome::NotEvaluated => Some("GLOBAL_ID_INVALID".into()),
            },
        }
    }

    fn built_interop_002_report(
        decision: AdapterDecision,
        outcome: AdapterPropertyOutcome,
        target_bytes: &[u8],
        ids_bytes: &[u8],
    ) -> BuiltInterop002Report {
        let diagnostic_code = match outcome {
            AdapterPropertyOutcome::Preserved => None,
            AdapterPropertyOutcome::Changed => Some("CHANGED_TARGET_PROPERTY"),
            AdapterPropertyOutcome::Missing => Some("MISSING_TARGET_PROPERTY"),
            AdapterPropertyOutcome::NotEvaluated => Some("GLOBAL_ID_INVALID"),
        };
        let mut report = BuiltInterop002Report {
            schema_version: "0.1.0".into(),
            benchmark_id: BUILT_INTEROP_002_ID.into(),
            supported_profile: BUILT_INTEROP_002_PROFILE.into(),
            decision,
            ifc_schema_token: Some("IFC4".into()),
            ids_version_evidence: "Declared:IDS1.0".into(),
            ifc_input_sha256: sha256_hex(target_bytes),
            ids_input_sha256: sha256_hex(ids_bytes),
            parsed_entity_count: 1,
            applicable_wall_count: Some(1),
            observations: vec![adapter_observation(outcome)],
            diagnostics: diagnostic_code.into_iter().map(str::to_string).collect(),
            semantic_result_sha256: "0".repeat(64),
        };
        let semantic_digest = AdapterSemanticDigest {
            supported_profile: &report.supported_profile,
            decision: report.decision,
            ifc_schema_token: &report.ifc_schema_token,
            ids_version_evidence: &report.ids_version_evidence,
            parsed_entity_count: report.parsed_entity_count,
            applicable_wall_count: report.applicable_wall_count,
            observations: &report.observations,
            diagnostics: &report.diagnostics,
        };
        report.semantic_result_sha256 = sha256_hex(&serde_json::to_vec(&semantic_digest).unwrap());
        report
    }

    fn gate_from_adapter(
        report: &BuiltInterop002Report,
        target_bytes: &[u8],
        ids_bytes: &[u8],
    ) -> Result<Gate, String> {
        let raw = serde_json::to_vec(report).unwrap();
        adapt_built_interop_002_report(
            report,
            &raw,
            Path::new("adapter-report.json"),
            target_bytes,
            ids_bytes,
        )
    }

    struct DeltaTestConfig {
        status: GateStatus,
        profile_confirmed: bool,
        coverage_complete: bool,
        compared: u64,
        preserved: u64,
        changed: u64,
        lost: u64,
        dropped: u64,
        unresolved: u64,
        reason: &'static str,
    }

    fn delta_gate_producer(
        source_bytes: &[u8],
        target_bytes: &[u8],
        ids_bytes: &[u8],
        config: DeltaTestConfig,
    ) -> ProducerGate {
        let payload = serde_json::json!({
            "policy_id": DELTA_POLICY_ID,
            "source_ifc_sha256": sha256_hex(source_bytes),
            "source_ifc_byte_length": source_bytes.len(),
            "target_ifc_sha256": sha256_hex(target_bytes),
            "target_ifc_byte_length": target_bytes.len(),
            "requirements_ids_sha256": sha256_hex(ids_bytes),
            "source_adapter_report_sha256": "a".repeat(64),
            "target_adapter_report_sha256": "b".repeat(64),
            "supported_profile": DELTA_PROFILE_ID,
            "profile_confirmed": config.profile_confirmed,
            "comparison_coverage_complete": config.coverage_complete,
            "counts": {
                "compared": config.compared,
                "preserved": config.preserved,
                "changed": config.changed,
                "lost": config.lost,
                "dropped": config.dropped,
                "added": 0,
                "unresolved": config.unresolved
            },
            "findings": [],
            "decision_reason": config.reason
        });
        let payload_sha256 = sha256_hex(&serde_json::to_vec(&payload).unwrap());
        ProducerGate {
            gate: Gate {
                gate_id: GateId::ExchangeSemanticPreservation,
                mandatory: true,
                status: config.status,
                claim_scope: "synthetic paired preservation test only".into(),
                limitations: vec!["Not real export/import evidence.".into()],
                tool: Some(ToolIdentity {
                    name: "built-interop-semantic-delta".into(),
                    version: DELTA_TOOL_VERSION.into(),
                    config_sha256: sha256_hex(DELTA_POLICY_ID.as_bytes()),
                }),
                evidence: vec![Evidence {
                    evidence_id: "delta-test-evidence".into(),
                    kind: EvidenceKind::ValidatorReport,
                    sha256: payload_sha256,
                    locator: PAYLOAD_POINTER.into(),
                }],
            },
            path: PathBuf::from("semantic-delta.json"),
            evidence_payload: Some(payload),
        }
    }

    fn schema_gate_producer(target_bytes: &[u8]) -> ProducerGate {
        let payload = serde_json::json!({
            "profile_id": IFC_SCHEMA_PROFILE_ID,
            "input_sha256": sha256_hex(target_bytes),
            "input_byte_length": target_bytes.len(),
            "declared_schema_token": "IFC4",
            "decision_reason": {"code": "VALIDATION_COMPLETED"},
            "summary": {
                "errors": 0,
                "evaluation_errors": 0,
                "warnings": 0,
                "unsupported": 0,
                "truncated": false,
                "validator_is_conformant": true
            },
            "findings": []
        });
        let payload_hash = sha256_hex(&serde_json::to_vec(&payload).unwrap());
        ProducerGate {
            gate: Gate {
                gate_id: GateId::IfcSchemaConformance,
                mandatory: true,
                status: GateStatus::Pass,
                claim_scope: "synthetic schema-gate contract fixture".into(),
                limitations: vec!["Synthetic report only.".into()],
                tool: Some(ToolIdentity {
                    name: "ifc-validate+ifc-schema".into(),
                    version: IFC_SCHEMA_TOOL_VERSION.into(),
                    config_sha256: sha256_hex(IFC_SCHEMA_POLICY_ID.as_bytes()),
                }),
                evidence: vec![Evidence {
                    evidence_id: "schema-test-evidence".into(),
                    kind: EvidenceKind::ValidatorReport,
                    sha256: payload_hash,
                    locator: PAYLOAD_POINTER.into(),
                }],
            },
            path: PathBuf::from("schema-gate.json"),
            evidence_payload: Some(payload),
        }
    }

    fn all_pass_gates() -> Vec<Gate> {
        GateId::ALL
            .iter()
            .enumerate()
            .map(|(index, id)| producer_gate(*id, GateStatus::Pass, &format!("evidence-{index}")))
            .collect()
    }

    fn assemble(gates: Vec<Gate>) -> Result<Report, String> {
        assemble_report(
            &fixture_args(),
            b"source model bytes",
            b"ids requirements bytes",
            b"target model bytes",
            gates,
        )
    }

    #[test]
    fn missing_gates_remain_not_run_and_cannot_qualify() {
        let report = assemble(Vec::new()).unwrap();
        assert_eq!(report.gates.len(), 8);
        assert!(
            report
                .gates
                .iter()
                .all(|gate| gate.status == GateStatus::NotRun)
        );
        assert_eq!(report.overall_decision, OverallDecision::Unqualified);
    }

    #[test]
    fn all_eight_passes_are_required_for_qualified() {
        let report = assemble(all_pass_gates()).unwrap();
        assert_eq!(report.overall_decision, OverallDecision::Qualified);
        assert_eq!(report.gates.len(), 8);
    }

    #[test]
    fn any_fail_blocks_even_when_the_other_gates_pass() {
        let mut gates = all_pass_gates();
        gates[3] = producer_gate(GateId::IdsXsdValidity, GateStatus::Fail, "evidence-fail");
        let report = assemble(gates).unwrap();
        assert_eq!(report.overall_decision, OverallDecision::Blocked);
    }

    #[test]
    fn unresolved_and_not_run_never_qualify() {
        let mut gates = all_pass_gates();
        gates[2] = producer_gate(
            GateId::IfcSchemaConformance,
            GateStatus::Unresolved,
            "unused",
        );
        let report = assemble(gates).unwrap();
        assert_eq!(report.overall_decision, OverallDecision::Unqualified);
    }

    #[test]
    fn duplicate_gate_identity_is_rejected() {
        let a = producer_gate(GateId::StepSyntax, GateStatus::Pass, "evidence-a");
        let b = producer_gate(GateId::StepSyntax, GateStatus::Pass, "evidence-b");
        assert_eq!(assemble(vec![a, b]).unwrap_err(), "DUPLICATE_GATE_ID");
    }

    #[test]
    fn pass_without_tool_or_evidence_is_rejected() {
        let mut gate = producer_gate(GateId::StepSyntax, GateStatus::Pass, "evidence");
        gate.tool = None;
        assert!(
            validate_gate(&gate)
                .unwrap_err()
                .starts_with("EXECUTED_GATE_REQUIRES_TOOL")
        );
        gate.tool = Some(ToolIdentity {
            name: "producer".into(),
            version: "1".into(),
            config_sha256: "b".repeat(64),
        });
        gate.evidence.clear();
        assert!(
            validate_gate(&gate)
                .unwrap_err()
                .starts_with("EXECUTED_GATE_REQUIRES_TOOL")
        );
    }

    #[test]
    fn not_run_with_tool_or_evidence_is_rejected() {
        let mut gate = producer_gate(GateId::StepSyntax, GateStatus::NotRun, "unused");
        gate.tool = Some(ToolIdentity {
            name: "producer".into(),
            version: "1".into(),
            config_sha256: "b".repeat(64),
        });
        assert!(
            validate_gate(&gate)
                .unwrap_err()
                .starts_with("NOT_RUN_GATE_HAS_EXECUTION_CLAIM")
        );
    }

    #[test]
    fn unknown_fields_and_malformed_hashes_are_rejected() {
        let raw = serde_json::json!({
            "gate_id":"step_syntax","mandatory":true,"status":"PASS","claim_scope":"test","limitations":["test"],
            "tool":{"name":"tool","version":"1","config_sha256":"b"},
            "evidence":[{"evidence_id":"e1","kind":"mutation_test","sha256":"c","locator":"fixture://e1"}],
            "surprise":true
        });
        assert!(serde_json::from_value::<Gate>(raw).is_err());
        let mut gate = producer_gate(GateId::StepSyntax, GateStatus::Pass, "evidence");
        gate.evidence[0].sha256 = "not-a-sha".into();
        assert!(
            validate_gate(&gate)
                .unwrap_err()
                .starts_with("GATE_EVIDENCE_INVALID")
        );
    }

    #[test]
    fn duplicate_evidence_id_across_gates_is_rejected() {
        let a = producer_gate(GateId::StepSyntax, GateStatus::Pass, "same-id");
        let b = producer_gate(GateId::IdsXsdValidity, GateStatus::Pass, "same-id");
        assert!(
            assemble(vec![a, b])
                .unwrap_err()
                .starts_with("DUPLICATE_EVIDENCE_ID_ACROSS_GATES")
        );
    }

    #[test]
    fn artifact_hashes_are_for_exact_bytes_and_change_when_bytes_change() {
        let first = artifact(
            ArtifactRole::SourceIfc,
            Path::new("source.ifc"),
            "model/ifc",
            b"A",
        );
        let second = artifact(
            ArtifactRole::SourceIfc,
            Path::new("source.ifc"),
            "model/ifc",
            b"B",
        );
        assert_ne!(first.sha256, second.sha256);
        assert_eq!(first.sha256, sha256_hex(b"A"));
        assert_eq!(first.byte_length, 1);
    }

    #[test]
    fn output_is_normalized_independent_of_gate_input_order() {
        let gates = all_pass_gates();
        let forward = assemble(gates.clone()).unwrap();
        let reverse = assemble(gates.into_iter().rev().collect()).unwrap();
        assert_eq!(
            serde_json::to_vec(&forward).unwrap(),
            serde_json::to_vec(&reverse).unwrap()
        );
    }

    #[test]
    fn timestamp_run_id_and_commit_provenance_fail_closed() {
        let mut args = fixture_args();
        args.reported_at = "2026-99-99T25:61:99Z".into();
        assert!(assemble_report(&args, b"s", b"i", b"t", Vec::new()).is_err());
        args = fixture_args();
        args.run_id = "bad/run/id".into();
        assert!(assemble_report(&args, b"s", b"i", b"t", Vec::new()).is_err());
        args = fixture_args();
        args.source_commit_sha = "g".repeat(40);
        assert!(assemble_report(&args, b"s", b"i", b"t", Vec::new()).is_err());
    }

    #[test]
    fn pinned_json_schema_matches_rust_gate_contract() {
        let schema: Value =
            serde_json::from_str(include_str!("../assembled-report-v1.schema.json")).unwrap();
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(schema["properties"]["schema_version"]["const"], "1.0.0");
        assert_eq!(
            schema["properties"]["benchmark_id"]["const"],
            "BUILT-INTEROP-004"
        );
        assert_eq!(
            schema["properties"]["evaluation_profile"]["properties"]["ids_version"]["const"],
            "1.0"
        );

        let schema_ids = schema["properties"]["gates"]["items"]["properties"]["gate_id"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();
        let rust_ids = GateId::ALL.iter().map(|id| id.as_str()).collect::<Vec<_>>();
        assert_eq!(schema_ids, rust_ids);

        let schema_statuses =
            schema["properties"]["gates"]["items"]["properties"]["status"]["enum"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value.as_str().unwrap())
                .collect::<BTreeSet<_>>();
        let expected_statuses = ["FAIL", "NOT_RUN", "PASS", "UNRESOLVED"]
            .into_iter()
            .collect::<BTreeSet<_>>();
        assert_eq!(schema_statuses, expected_statuses);

        let not_run_rule = schema["properties"]["gates"]["items"]["allOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|rule| rule["if"]["properties"]["status"]["const"] == "NOT_RUN")
            .unwrap();
        assert_eq!(
            not_run_rule["then"]["properties"]["evidence"]["maxItems"],
            0
        );
        assert!(
            not_run_rule["then"]["not"]["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "tool")
        );
    }

    #[test]
    fn embedded_schema_payload_is_bound_to_exact_target_bytes_and_profile() {
        let target = b"target model bytes";
        let producer = schema_gate_producer(target);
        assert!(
            verify_embedded_payload(
                &producer,
                b"source bytes",
                target,
                b"ids requirements",
                "IFC4"
            )
            .is_ok()
        );
    }

    #[test]
    fn embedded_schema_payload_rejects_a_different_target_artifact() {
        let target = b"target model bytes";
        let producer = schema_gate_producer(target);
        assert_eq!(
            verify_embedded_payload(
                &producer,
                b"source bytes",
                b"other target",
                b"ids requirements",
                "IFC4"
            )
            .unwrap_err(),
            "IFC_SCHEMA_EVIDENCE_ARTIFACT_MISMATCH"
        );
    }

    #[test]
    fn embedded_payload_without_a_pointer_is_not_silently_accepted() {
        let target = b"target model bytes";
        let mut producer = schema_gate_producer(target);
        producer.gate.evidence[0].locator = "schema-gate.json".into();
        assert_eq!(
            verify_embedded_payload(
                &producer,
                b"source bytes",
                target,
                b"ids requirements",
                "IFC4"
            )
            .unwrap_err(),
            "EVIDENCE_PAYLOAD_UNREFERENCED"
        );
    }

    #[test]
    fn schema_pointer_without_embedded_payload_is_rejected() {
        let target = b"target model bytes";
        let mut producer = schema_gate_producer(target);
        producer.evidence_payload = None;
        assert_eq!(
            verify_embedded_payload(
                &producer,
                b"source bytes",
                target,
                b"ids requirements",
                "IFC4"
            )
            .unwrap_err(),
            "EVIDENCE_PAYLOAD_MISSING"
        );
    }

    #[test]
    fn critical_gates_cannot_bypass_payload_verification_with_bare_gate_objects() {
        let source = b"source bytes";
        let target = b"target bytes";
        let ids = b"ids bytes";

        let mut schema_gate = schema_gate_producer(target);
        schema_gate.evidence_payload = None;
        schema_gate.gate.evidence[0].locator = "schema-gate.json".into();
        assert_eq!(
            verify_embedded_payload(&schema_gate, source, target, ids, "IFC4").unwrap_err(),
            "IFC_SCHEMA_EVIDENCE_PAYLOAD_REQUIRED"
        );

        let mut delta_gate = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Pass,
                profile_confirmed: true,
                coverage_complete: true,
                compared: 1,
                preserved: 1,
                changed: 0,
                lost: 0,
                dropped: 0,
                unresolved: 0,
                reason: "COMPLETE_SUPPORTED_DELTA_WITH_NO_LOSS",
            },
        );
        delta_gate.evidence_payload = None;
        delta_gate.gate.evidence[0].locator = "delta-gate.json".into();
        assert_eq!(
            verify_embedded_payload(&delta_gate, source, target, ids, "IFC4").unwrap_err(),
            "DELTA_EVIDENCE_PAYLOAD_REQUIRED"
        );
    }

    #[test]
    fn embedded_payload_digest_and_declared_schema_are_verified() {
        let target = b"target model bytes";
        let mut producer = schema_gate_producer(target);
        producer.evidence_payload.as_mut().unwrap()["findings"] =
            serde_json::json!([{"tampered": true}]);
        assert_eq!(
            verify_embedded_payload(
                &producer,
                b"source bytes",
                target,
                b"ids requirements",
                "IFC4"
            )
            .unwrap_err(),
            "EVIDENCE_POINTER_DIGEST_MISMATCH"
        );

        let producer = schema_gate_producer(target);
        assert_eq!(
            verify_embedded_payload(
                &producer,
                b"source bytes",
                target,
                b"ids requirements",
                "IFC2X3"
            )
            .unwrap_err(),
            "IFC_SCHEMA_EVIDENCE_PROFILE_MISMATCH"
        );
    }

    #[test]
    fn schema_gate_requires_pinned_tool_configuration_provenance() {
        let target = b"target model bytes";
        let mut producer = schema_gate_producer(target);
        producer.gate.tool.as_mut().unwrap().config_sha256 = "0".repeat(64);
        assert_eq!(
            verify_embedded_payload(
                &producer,
                b"source bytes",
                target,
                b"ids requirements",
                "IFC4"
            )
            .unwrap_err(),
            "IFC_SCHEMA_TOOL_PROVENANCE_MISMATCH"
        );
    }

    #[test]
    fn paired_delta_payload_binds_source_target_ids_and_policy() {
        let source = b"source exact bytes";
        let target = b"target exact bytes";
        let ids = b"requirements exact bytes";
        let producer = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Pass,
                profile_confirmed: true,
                coverage_complete: true,
                compared: 1,
                preserved: 1,
                changed: 0,
                lost: 0,
                dropped: 0,
                unresolved: 0,
                reason: "COMPLETE_SUPPORTED_DELTA_WITH_NO_LOSS",
            },
        );
        assert!(verify_embedded_payload(&producer, source, target, ids, "IFC4").is_ok());

        assert_eq!(
            verify_embedded_payload(&producer, b"other source", target, ids, "IFC4").unwrap_err(),
            "DELTA_SOURCE_ARTIFACT_MISMATCH"
        );
        assert_eq!(
            verify_embedded_payload(&producer, source, b"other target", ids, "IFC4").unwrap_err(),
            "DELTA_TARGET_ARTIFACT_MISMATCH"
        );
        assert_eq!(
            verify_embedded_payload(&producer, source, target, b"other IDS", "IFC4").unwrap_err(),
            "DELTA_REQUIREMENTS_ARTIFACT_MISMATCH"
        );
    }

    #[test]
    fn paired_delta_status_must_agree_with_counts_coverage_and_reason() {
        let source = b"source";
        let target = b"target";
        let ids = b"ids";
        let forged_pass = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Pass,
                profile_confirmed: true,
                coverage_complete: true,
                compared: 1,
                preserved: 0,
                changed: 1,
                lost: 0,
                dropped: 0,
                unresolved: 0,
                reason: "COMPLETE_SUPPORTED_DELTA_WITH_NO_LOSS",
            },
        );
        assert_eq!(
            verify_embedded_payload(&forged_pass, source, target, ids, "IFC4").unwrap_err(),
            "DELTA_STATUS_PAYLOAD_MISMATCH"
        );

        let valid_fail = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Fail,
                profile_confirmed: true,
                coverage_complete: false,
                compared: 1,
                preserved: 0,
                changed: 1,
                lost: 0,
                dropped: 0,
                unresolved: 1,
                reason: "DEFINITE_SOURCE_TO_TARGET_LOSS",
            },
        );
        assert!(verify_embedded_payload(&valid_fail, source, target, ids, "IFC4").is_ok());

        let invalid_fail = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Fail,
                profile_confirmed: true,
                coverage_complete: true,
                compared: 1,
                preserved: 1,
                changed: 0,
                lost: 0,
                dropped: 0,
                unresolved: 0,
                reason: "DEFINITE_SOURCE_TO_TARGET_LOSS",
            },
        );
        assert_eq!(
            verify_embedded_payload(&invalid_fail, source, target, ids, "IFC4").unwrap_err(),
            "DELTA_STATUS_PAYLOAD_MISMATCH"
        );
    }

    #[test]
    fn unconfirmed_profile_with_apparent_loss_remains_unresolved() {
        let source = b"source";
        let target = b"target";
        let ids = b"ids";
        let producer = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Unresolved,
                profile_confirmed: false,
                coverage_complete: false,
                compared: 1,
                preserved: 0,
                changed: 1,
                lost: 0,
                dropped: 0,
                unresolved: 1,
                reason: "PROFILE_IDENTITY_OR_COVERAGE_UNRESOLVED",
            },
        );
        assert!(verify_embedded_payload(&producer, source, target, ids, "IFC4").is_ok());

        let falsely_blocked = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Fail,
                profile_confirmed: false,
                coverage_complete: false,
                compared: 1,
                preserved: 0,
                changed: 1,
                lost: 0,
                dropped: 0,
                unresolved: 1,
                reason: "DEFINITE_SOURCE_TO_TARGET_LOSS",
            },
        );
        assert_eq!(
            verify_embedded_payload(&falsely_blocked, source, target, ids, "IFC4").unwrap_err(),
            "DELTA_STATUS_PAYLOAD_MISMATCH"
        );
    }

    #[test]
    fn paired_delta_requires_pinned_tool_and_configuration() {
        let source = b"source";
        let target = b"target";
        let ids = b"ids";
        let mut producer = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Pass,
                profile_confirmed: true,
                coverage_complete: true,
                compared: 1,
                preserved: 1,
                changed: 0,
                lost: 0,
                dropped: 0,
                unresolved: 0,
                reason: "COMPLETE_SUPPORTED_DELTA_WITH_NO_LOSS",
            },
        );
        producer.gate.tool.as_mut().unwrap().config_sha256 = "0".repeat(64);
        assert_eq!(
            verify_embedded_payload(&producer, source, target, ids, "IFC4").unwrap_err(),
            "DELTA_TOOL_PROVENANCE_MISMATCH"
        );
    }

    #[test]
    fn typed_ids_adapter_report_maps_subset_conformance_to_pass_and_hashes_exact_report() {
        let target = b"the exact target IFC bytes";
        let ids = b"the exact IDS requirement bytes";
        let report = built_interop_002_report(
            AdapterDecision::SubsetConformant,
            AdapterPropertyOutcome::Preserved,
            target,
            ids,
        );
        let raw = serde_json::to_vec(&report).unwrap();
        let gate = adapt_built_interop_002_report(
            &report,
            &raw,
            Path::new("adapter-report.json"),
            target,
            ids,
        )
        .unwrap();

        assert_eq!(gate.gate_id, GateId::IdsModelConformance);
        assert_eq!(gate.status, GateStatus::Pass);
        assert_eq!(gate.evidence[0].sha256, sha256_hex(&raw));
        assert_eq!(
            gate.evidence[0].evidence_id,
            format!("built-interop-002-report-{}", sha256_hex(&raw))
        );
    }

    #[test]
    fn typed_ids_adapter_maps_changed_or_missing_required_name_to_fail() {
        let target = b"target IFC";
        let ids = b"IDS file";
        for outcome in [
            AdapterPropertyOutcome::Changed,
            AdapterPropertyOutcome::Missing,
        ] {
            let report = built_interop_002_report(AdapterDecision::Blocked, outcome, target, ids);
            let gate = gate_from_adapter(&report, target, ids).unwrap();
            assert_eq!(gate.gate_id, GateId::IdsModelConformance);
            assert_eq!(gate.status, GateStatus::Fail, "outcome={outcome:?}");
        }
    }

    #[test]
    fn typed_ids_identity_or_unsupported_blocker_stays_unresolved_not_fail() {
        let target = b"target IFC";
        let ids = b"IDS file";
        let report = built_interop_002_report(
            AdapterDecision::Blocked,
            AdapterPropertyOutcome::NotEvaluated,
            target,
            ids,
        );
        let gate = gate_from_adapter(&report, target, ids).unwrap();
        assert_eq!(gate.status, GateStatus::Unresolved);
    }

    #[test]
    fn typed_ids_adapter_recomputes_and_verifies_semantic_result_digest() {
        let target = b"target IFC";
        let ids = b"IDS file";
        let mut report = built_interop_002_report(
            AdapterDecision::SubsetConformant,
            AdapterPropertyOutcome::Preserved,
            target,
            ids,
        );
        report.observations[0].observed_value = Some("tampered".into());
        let raw = serde_json::to_vec(&report).unwrap();
        assert_eq!(
            adapt_built_interop_002_report(
                &report,
                &raw,
                Path::new("adapter-report.json"),
                target,
                ids,
            )
            .unwrap_err(),
            "BUILT_INTEROP_002_SEMANTIC_DIGEST_MISMATCH"
        );
    }

    #[test]
    fn typed_ids_adapter_requires_target_and_ids_byte_digests_to_match() {
        let target = b"target IFC";
        let ids = b"IDS file";
        let report = built_interop_002_report(
            AdapterDecision::SubsetConformant,
            AdapterPropertyOutcome::Preserved,
            target,
            ids,
        );
        assert!(
            adapt_built_interop_002_report(
                &report,
                &serde_json::to_vec(&report).unwrap(),
                Path::new("adapter-report.json"),
                b"different IFC",
                ids,
            )
            .unwrap_err()
            .starts_with("BUILT_INTEROP_002_ARTIFACT_DIGEST_MISMATCH")
        );
    }

    #[test]
    fn typed_ids_report_with_wrong_schema_profile_cannot_pass() {
        let target = b"target IFC";
        let ids = b"IDS file";
        let mut report = built_interop_002_report(
            AdapterDecision::SubsetConformant,
            AdapterPropertyOutcome::Preserved,
            target,
            ids,
        );
        report.ifc_schema_token = Some("IFC2X3".into());
        let semantic_digest = AdapterSemanticDigest {
            supported_profile: &report.supported_profile,
            decision: report.decision,
            ifc_schema_token: &report.ifc_schema_token,
            ids_version_evidence: &report.ids_version_evidence,
            parsed_entity_count: report.parsed_entity_count,
            applicable_wall_count: report.applicable_wall_count,
            observations: &report.observations,
            diagnostics: &report.diagnostics,
        };
        report.semantic_result_sha256 = sha256_hex(&serde_json::to_vec(&semantic_digest).unwrap());
        let gate = gate_from_adapter(&report, target, ids).unwrap();
        assert_eq!(gate.status, GateStatus::Unresolved);
    }

    #[test]
    fn schema_ids_and_preservation_producers_compose_into_one_unqualified_report() {
        let source = b"source exchange model bytes";
        let target = b"target exchange model bytes";
        let ids = b"exact IDS requirements bytes";

        let schema_producer = schema_gate_producer(target);
        verify_embedded_payload(&schema_producer, source, target, ids, "IFC4").unwrap();

        let delta_producer = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Pass,
                profile_confirmed: true,
                coverage_complete: true,
                compared: 1,
                preserved: 1,
                changed: 0,
                lost: 0,
                dropped: 0,
                unresolved: 0,
                reason: "COMPLETE_SUPPORTED_DELTA_WITH_NO_LOSS",
            },
        );
        verify_embedded_payload(&delta_producer, source, target, ids, "IFC4").unwrap();

        let adapter_report = built_interop_002_report(
            AdapterDecision::SubsetConformant,
            AdapterPropertyOutcome::Preserved,
            target,
            ids,
        );
        let adapter_bytes = serde_json::to_vec(&adapter_report).unwrap();
        let ids_gate = adapt_built_interop_002_report(
            &adapter_report,
            &adapter_bytes,
            Path::new("adapter-report.json"),
            target,
            ids,
        )
        .unwrap();

        let supplied = vec![schema_producer.gate, delta_producer.gate, ids_gate];
        let report = assemble_report(&fixture_args(), source, ids, target, supplied).unwrap();

        assert_eq!(report.overall_decision, OverallDecision::Unqualified);
        assert_eq!(report.artifacts[0].sha256, sha256_hex(source));
        assert_eq!(report.artifacts[1].sha256, sha256_hex(ids));
        assert_eq!(report.artifacts[2].sha256, sha256_hex(target));
        assert_eq!(
            report
                .gates
                .iter()
                .find(|gate| gate.gate_id == GateId::IfcSchemaConformance)
                .unwrap()
                .status,
            GateStatus::Pass
        );
        assert_eq!(
            report
                .gates
                .iter()
                .find(|gate| gate.gate_id == GateId::IdsModelConformance)
                .unwrap()
                .status,
            GateStatus::Pass
        );
        assert_eq!(
            report
                .gates
                .iter()
                .find(|gate| gate.gate_id == GateId::ExchangeSemanticPreservation)
                .unwrap()
                .status,
            GateStatus::Pass
        );
        assert_eq!(
            report
                .gates
                .iter()
                .filter(|gate| gate.status == GateStatus::NotRun)
                .count(),
            5
        );
    }

    #[test]
    fn one_confirmed_preservation_failure_blocks_composed_report() {
        let source = b"source exchange model bytes";
        let target = b"target exchange model bytes";
        let ids = b"exact IDS requirements bytes";
        let delta_producer = delta_gate_producer(
            source,
            target,
            ids,
            DeltaTestConfig {
                status: GateStatus::Fail,
                profile_confirmed: true,
                coverage_complete: false,
                compared: 1,
                preserved: 0,
                changed: 1,
                lost: 0,
                dropped: 0,
                unresolved: 1,
                reason: "DEFINITE_SOURCE_TO_TARGET_LOSS",
            },
        );
        verify_embedded_payload(&delta_producer, source, target, ids, "IFC4").unwrap();

        let report = assemble_report(
            &fixture_args(),
            source,
            ids,
            target,
            vec![delta_producer.gate],
        )
        .unwrap();
        assert_eq!(report.overall_decision, OverallDecision::Blocked);
        assert_eq!(
            report
                .gates
                .iter()
                .find(|gate| gate.gate_id == GateId::ExchangeSemanticPreservation)
                .unwrap()
                .status,
            GateStatus::Fail
        );
        assert_eq!(
            report
                .gates
                .iter()
                .filter(|gate| gate.status == GateStatus::NotRun)
                .count(),
            7
        );
    }

    #[test]
    fn report_matches_closed_contract_shape() {
        let report = assemble(Vec::new()).unwrap();
        let json = serde_json::to_value(report).unwrap();
        assert_eq!(json["schema_version"], "1.0.0");
        assert_eq!(json["benchmark_id"], "BUILT-INTEROP-004");
        assert_eq!(json["evaluation_profile"]["ids_version"], "1.0");
        assert_eq!(json["artifacts"].as_array().unwrap().len(), 3);
        assert_eq!(json["gates"].as_array().unwrap().len(), 8);
        assert_eq!(json["overall_decision"], "UNQUALIFIED");
    }
}
