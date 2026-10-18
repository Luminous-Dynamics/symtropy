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

    let number = |start: usize, end: usize| -> u32 {
        value[start..end].parse().unwrap_or(u32::MAX)
    };
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
        return Err(format!("GATE_SCOPE_OR_LIMITATIONS_INVALID:{}", gate.gate_id.as_str()));
    }
    if let Some(tool) = &gate.tool {
        if tool.name.trim().is_empty()
            || tool.version.trim().is_empty()
            || !is_lower_hex_of_length(&tool.config_sha256, 64)
        {
            return Err(format!("GATE_TOOL_IDENTITY_INVALID:{}", gate.gate_id.as_str()));
        }
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
                return Err(format!("EXECUTED_GATE_REQUIRES_TOOL_AND_EVIDENCE:{}", gate.gate_id.as_str()));
            }
        }
        GateStatus::NotRun => {
            if gate.tool.is_some() || !gate.evidence.is_empty() {
                return Err(format!("NOT_RUN_GATE_HAS_EXECUTION_CLAIM:{}", gate.gate_id.as_str()));
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
    if args.ifc_schema.trim().is_empty() || args.supported_subset.trim().is_empty() || args.evaluator_name.trim().is_empty() {
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
        let mut gate = by_id.remove(&gate_id).unwrap_or_else(|| not_run_gate(gate_id));
        gate.evidence.sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
        for evidence in &gate.evidence {
            if !all_evidence_ids.insert(evidence.evidence_id.clone()) {
                return Err(format!("DUPLICATE_EVIDENCE_ID_ACROSS_GATES:{}", evidence.evidence_id));
            }
        }
        validate_gate(&gate)?;
        gates.push(gate);
    }
    if !by_id.is_empty() {
        return Err("UNKNOWN_GATE_ID".into());
    }

    let artifacts = vec![
        artifact(ArtifactRole::SourceIfc, &args.source_ifc, "model/ifc", source_bytes),
        artifact(ArtifactRole::RequirementsIds, &args.requirements_ids, "application/ids+xml", ids_bytes),
        artifact(ArtifactRole::TargetIfc, &args.target_ifc, "model/ifc", target_bytes),
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
        || report.artifacts.iter().map(|a| a.role).collect::<Vec<_>>().as_slice() != expected_roles.as_slice()
        || report.artifacts.iter().any(|a| {
            !is_lower_hex_of_length(&a.sha256, 64) || a.media_type.trim().is_empty() || a.locator.trim().is_empty()
        })
    {
        return Err("ARTIFACT_SET_INVALID".into());
    }

    if report.gates.len() != GateId::ALL.len()
        || report.gates.iter().map(|gate| gate.gate_id).collect::<Vec<_>>().as_slice() != GateId::ALL.as_slice()
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

fn parse_gate_file(path: &Path) -> Result<Gate, String> {
    let bytes = fs::read(path).map_err(|error| format!("CANNOT_READ_GATE_JSON:{}:{error}", path.display()))?;
    let json: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("INVALID_GATE_JSON:{}:{error}", path.display()))?;
    let gate_value = json.get("gate").unwrap_or(&json).clone();
    let mut gate: Gate = serde_json::from_value(gate_value)
        .map_err(|error| format!("INVALID_TYPED_GATE:{}:{error}", path.display()))?;
    validate_gate(&gate)?;
    gate.evidence.sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
    Ok(gate)
}

fn parse_args() -> Result<Option<CliArgs>, String> {
    let mut args = env::args().skip(1);
    if matches!(args.clone().next().as_deref(), Some("--help" | "-h")) {
        return Ok(None);
    }
    let mut singleton = BTreeMap::<String, String>::new();
    let mut gate_json = Vec::new();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--gate-json" => {
                let value = args.next().ok_or("MISSING_VALUE:--gate-json")?;
                gate_json.push(PathBuf::from(value));
            }
            "--source-ifc" | "--ids" | "--target-ifc" | "--ifc-schema"
            | "--supported-subset" | "--run-id" | "--reported-at"
            | "--source-commit-sha" | "--evaluator-name" | "--output" => {
                let value = args.next().ok_or_else(|| format!("MISSING_VALUE:{flag}"))?;
                if singleton.insert(flag.clone(), value).is_some() {
                    return Err(format!("DUPLICATE_ARGUMENT:{flag}"));
                }
            }
            _ => return Err(format!("UNKNOWN_ARGUMENT:{flag}")),
        }
    }
    let take = |key: &str| -> Result<String, String> {
        singleton.get(key).cloned().ok_or_else(|| format!("MISSING_REQUIRED_ARGUMENT:{key}"))
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
    let source_bytes = fs::read(&args.source_ifc)
        .map_err(|error| format!("CANNOT_READ_SOURCE_IFC:{}:{error}", args.source_ifc.display()))?;
    let ids_bytes = fs::read(&args.requirements_ids)
        .map_err(|error| format!("CANNOT_READ_IDS:{}:{error}", args.requirements_ids.display()))?;
    let target_bytes = fs::read(&args.target_ifc)
        .map_err(|error| format!("CANNOT_READ_TARGET_IFC:{}:{error}", args.target_ifc.display()))?;
    let mut producer_gates = Vec::new();
    for gate_path in &args.gate_json {
        producer_gates.push(parse_gate_file(gate_path)?);
    }

    let report = assemble_report(&args, &source_bytes, &ids_bytes, &target_bytes, producer_gates)?;
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

    fn all_pass_gates() -> Vec<Gate> {
        GateId::ALL.iter().enumerate()
            .map(|(index, id)| producer_gate(*id, GateStatus::Pass, &format!("evidence-{index}")))
            .collect()
    }

    fn assemble(gates: Vec<Gate>) -> Result<Report, String> {
        assemble_report(&fixture_args(), b"source model bytes", b"ids requirements bytes", b"target model bytes", gates)
    }

    #[test]
    fn missing_gates_remain_not_run_and_cannot_qualify() {
        let report = assemble(Vec::new()).unwrap();
        assert_eq!(report.gates.len(), 8);
        assert!(report.gates.iter().all(|gate| gate.status == GateStatus::NotRun));
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
        gates[2] = producer_gate(GateId::IfcSchemaConformance, GateStatus::Unresolved, "unused");
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
        assert!(validate_gate(&gate).unwrap_err().starts_with("EXECUTED_GATE_REQUIRES_TOOL"));
        gate.tool = Some(ToolIdentity {
            name: "producer".into(),
            version: "1".into(),
            config_sha256: "b".repeat(64),
        });
        gate.evidence.clear();
        assert!(validate_gate(&gate).unwrap_err().starts_with("EXECUTED_GATE_REQUIRES_TOOL"));
    }

    #[test]
    fn not_run_with_tool_or_evidence_is_rejected() {
        let mut gate = producer_gate(GateId::StepSyntax, GateStatus::NotRun, "unused");
        gate.tool = Some(ToolIdentity { name: "producer".into(), version: "1".into(), config_sha256: "b".repeat(64) });
        assert!(validate_gate(&gate).unwrap_err().starts_with("NOT_RUN_GATE_HAS_EXECUTION_CLAIM"));
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
        assert!(validate_gate(&gate).unwrap_err().starts_with("GATE_EVIDENCE_INVALID"));
    }

    #[test]
    fn duplicate_evidence_id_across_gates_is_rejected() {
        let a = producer_gate(GateId::StepSyntax, GateStatus::Pass, "same-id");
        let b = producer_gate(GateId::IdsXsdValidity, GateStatus::Pass, "same-id");
        assert!(assemble(vec![a, b]).unwrap_err().starts_with("DUPLICATE_EVIDENCE_ID_ACROSS_GATES"));
    }

    #[test]
    fn artifact_hashes_are_for_exact_bytes_and_change_when_bytes_change() {
        let first = artifact(ArtifactRole::SourceIfc, Path::new("source.ifc"), "model/ifc", b"A");
        let second = artifact(ArtifactRole::SourceIfc, Path::new("source.ifc"), "model/ifc", b"B");
        assert_ne!(first.sha256, second.sha256);
        assert_eq!(first.sha256, sha256_hex(b"A"));
        assert_eq!(first.byte_length, 1);
    }

    #[test]
    fn output_is_normalized_independent_of_gate_input_order() {
        let gates = all_pass_gates();
        let forward = assemble(gates.clone()).unwrap();
        let reverse = assemble(gates.into_iter().rev().collect()).unwrap();
        assert_eq!(serde_json::to_vec(&forward).unwrap(), serde_json::to_vec(&reverse).unwrap());
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
