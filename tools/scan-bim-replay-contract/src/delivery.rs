// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fail-closed audit of Scan-to-BIM IFC delivery receipts.
//!
//! This module validates receipt semantics and cross-artifact bindings. It does
//! not parse IFC, execute IDS checks, read evidence files, or validate geometry.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const REQUIRED_GATE_IDS: [GateId; 9] = [
    GateId::ArtifactIntegrity,
    GateId::StepParse,
    GateId::IfcSchema,
    GateId::IdsXsdValidity,
    GateId::IdsSemanticSupport,
    GateId::IfcIdsConformance,
    GateId::SemanticPreservation,
    GateId::GeometryTopology,
    GateId::EditRoundTrip,
];

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum GateId {
    ArtifactIntegrity,
    StepParse,
    IfcSchema,
    IdsXsdValidity,
    IdsSemanticSupport,
    IfcIdsConformance,
    SemanticPreservation,
    GeometryTopology,
    EditRoundTrip,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateStatus {
    Pass,
    Fail,
    Unresolved,
    NotRun,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DeliveryDecisionStatus {
    Qualified,
    Blocked,
    Unqualified,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SubjectRole {
    IfcOutput,
    ElementGraph,
    IdsRequirements,
    SourceModel,
    RoundTripOutput,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CorrespondenceStatus {
    Supported,
    Ambiguous,
    Unresolved,
    UnmatchedIfc,
    UnmatchedGraph,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeliveryReceipt {
    pub schema_version: String,
    pub delivery_id: String,
    pub reported_at: String,
    pub replay: ReplayBinding,
    pub scene: SceneBinding,
    pub ifc: Artifact,
    pub element_graph: Artifact,
    pub exchange_profile: ExchangeProfile,
    pub ids_requirements: Option<IdsRequirements>,
    pub crosswalk: Vec<CrosswalkEntry>,
    pub required_gate_ids: Vec<GateId>,
    pub gates: Vec<Gate>,
    pub decision: DeclaredDecision,
    pub claim_ceiling: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayBinding {
    pub run_id: String,
    pub receipt_sha256: String,
    pub candidate_source_commit: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneBinding {
    pub building_group_id: String,
    pub scene_id: String,
    pub source_model_revision: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub path: String,
    pub sha256: String,
    pub byte_length: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExchangeProfile {
    pub ifc_schema: String,
    pub exchange_view: String,
    pub length_unit: String,
    pub coordinate_frame_id: String,
    pub axis_order: String,
    pub handedness: String,
    pub placement_convention: String,
    pub crs: Option<String>,
    pub vertical_datum: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdsRequirements {
    pub sha256: String,
    pub standard_version: String,
    pub requirement_revision: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrosswalkEntry {
    pub ifc_global_id: String,
    pub element_graph_id: String,
    pub correspondence: CorrespondenceStatus,
    pub evidence_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    pub gate_id: GateId,
    pub status: GateStatus,
    pub replay_receipt_sha256: String,
    pub scene_id: String,
    pub subject_artifacts: Vec<GateSubjectArtifact>,
    pub scope: String,
    pub limitation: String,
    pub producer: Option<Producer>,
    pub evidence: Vec<EvidenceArtifact>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateSubjectArtifact {
    pub artifact_id: String,
    pub role: SubjectRole,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    pub tool: String,
    pub version: String,
    pub configuration_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceArtifact {
    pub artifact_id: String,
    pub sha256: String,
    pub media_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredDecision {
    pub status: DeliveryDecisionStatus,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DeliveryAuditReport {
    pub schema_version: String,
    pub delivery_id: String,
    pub scene_id: String,
    pub gate_count: usize,
    pub gate_status_counts: BTreeMap<String, usize>,
    pub derived_decision: DeliveryDecisionStatus,
    pub crosswalk_supported_count: usize,
    pub crosswalk_unresolved_count: usize,
    pub claim_ceiling: String,
}

/// Parse and validate a receipt from its exact JSON bytes.
///
/// Byte integrity of the referenced IFC/graph/evidence files remains an
/// independent gate. This function verifies only the receipt's declared
/// identities, structure and fail-closed decision semantics.
pub fn audit_delivery_receipt_bytes(bytes: &[u8]) -> Result<DeliveryAuditReport, String> {
    let receipt: DeliveryReceipt = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid delivery receipt: {error}"))?;
    audit_delivery_receipt(receipt)
}

/// Validate an already parsed delivery receipt.
pub fn audit_delivery_receipt(
    receipt: DeliveryReceipt,
) -> Result<DeliveryAuditReport, String> {
    if receipt.schema_version != "0.1" {
        return Err("unsupported delivery receipt schema_version".to_owned());
    }
    for (field, value) in [
        ("delivery_id", receipt.delivery_id.as_str()),
        ("reported_at", receipt.reported_at.as_str()),
        ("replay.run_id", receipt.replay.run_id.as_str()),
        ("scene.building_group_id", receipt.scene.building_group_id.as_str()),
        ("scene.scene_id", receipt.scene.scene_id.as_str()),
        ("scene.source_model_revision", receipt.scene.source_model_revision.as_str()),
        ("exchange_profile.ifc_schema", receipt.exchange_profile.ifc_schema.as_str()),
        ("exchange_profile.exchange_view", receipt.exchange_profile.exchange_view.as_str()),
        (
            "exchange_profile.coordinate_frame_id",
            receipt.exchange_profile.coordinate_frame_id.as_str(),
        ),
        (
            "exchange_profile.placement_convention",
            receipt.exchange_profile.placement_convention.as_str(),
        ),
        ("claim_ceiling", receipt.claim_ceiling.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{field} must not be empty"));
        }
    }

    validate_sha256("replay.receipt_sha256", &receipt.replay.receipt_sha256)?;
    validate_commit_sha("replay.candidate_source_commit", &receipt.replay.candidate_source_commit)?;
    validate_artifact("ifc", &receipt.ifc)?;
    validate_artifact("element_graph", &receipt.element_graph)?;
    validate_exchange_profile(&receipt.exchange_profile)?;

    if let Some(ids) = &receipt.ids_requirements {
        validate_sha256("ids_requirements.sha256", &ids.sha256)?;
        if ids.standard_version != "1.0" || ids.requirement_revision.trim().is_empty() {
            return Err("IDS requirements must declare IDS 1.0 and a non-empty revision".to_owned());
        }
    }

    if receipt.required_gate_ids.as_slice() != &REQUIRED_GATE_IDS[..] {
        return Err(
            "required_gate_ids must equal the nine canonical gate identities in order".to_owned(),
        );
    }
    if receipt.gates.len() != REQUIRED_GATE_IDS.len() {
        return Err(
            "gates must contain exactly one row for each of the nine required gates".to_owned(),
        );
    }

    let mut gate_by_id = BTreeMap::new();
    for gate in &receipt.gates {
        if gate_by_id.insert(gate.gate_id, gate).is_some() {
            return Err(format!("duplicate gate identity: {:?}", gate.gate_id));
        }
    }
    for required in REQUIRED_GATE_IDS {
        if !gate_by_id.contains_key(&required) {
            return Err(format!("missing required gate: {required:?}"));
        }
    }

    let mut status_counts = BTreeMap::new();
    for gate in &receipt.gates {
        validate_gate_binding(gate, &receipt)?;
        validate_gate_subjects(gate, &receipt)?;
        validate_gate_execution_shape(gate)?;
        *status_counts.entry(format!("{:?}", gate.status).to_ascii_uppercase()).or_insert(0) += 1;
    }

    let (supported_count, unresolved_count) = validate_crosswalk(&receipt.crosswalk)?;

    let has_failure = receipt.gates.iter().any(|gate| gate.status == GateStatus::Fail);
    let all_pass = receipt.gates.iter().all(|gate| gate.status == GateStatus::Pass);
    let crosswalk_complete = !receipt.crosswalk.is_empty()
        && receipt
            .crosswalk
            .iter()
            .all(|entry| entry.correspondence == CorrespondenceStatus::Supported);
    let ids_present = receipt.ids_requirements.is_some();

    let derived_decision = if has_failure {
        DeliveryDecisionStatus::Blocked
    } else if all_pass && crosswalk_complete && ids_present {
        DeliveryDecisionStatus::Qualified
    } else {
        DeliveryDecisionStatus::Unqualified
    };

    if !ids_present
        && receipt.gates.iter().any(|gate| {
            matches!(
                gate.gate_id,
                GateId::IdsXsdValidity | GateId::IdsSemanticSupport | GateId::IfcIdsConformance
            ) && gate.status == GateStatus::Pass
        })
    {
        return Err(
            "IDS validation gates cannot PASS without an IDS requirements artifact".to_owned(),
        );
    }

    if receipt.decision.status != derived_decision {
        return Err(format!(
            "declared decision {:?} does not match derived decision {:?}",
            receipt.decision.status, derived_decision
        ));
    }
    if receipt.decision.reason_codes.iter().any(|code| code.trim().is_empty()) {
        return Err("decision reason codes must not be empty strings".to_owned());
    }
    if derived_decision == DeliveryDecisionStatus::Qualified
        && !receipt.decision.reason_codes.is_empty()
    {
        return Err("QUALIFIED decision must not contain blocking reason codes".to_owned());
    }
    if derived_decision != DeliveryDecisionStatus::Qualified
        && receipt.decision.reason_codes.is_empty()
    {
        return Err("non-qualified decision must include at least one reason code".to_owned());
    }

    Ok(DeliveryAuditReport {
        schema_version: receipt.schema_version,
        delivery_id: receipt.delivery_id,
        scene_id: receipt.scene.scene_id,
        gate_count: receipt.gates.len(),
        gate_status_counts: status_counts,
        derived_decision,
        crosswalk_supported_count: supported_count,
        crosswalk_unresolved_count: unresolved_count,
        claim_ceiling: receipt.claim_ceiling,
    })
}

fn validate_sha256(field: &str, digest: &str) -> Result<(), String> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{field} must be 64 lowercase hexadecimal characters"));
    }
    Ok(())
}

fn validate_commit_sha(field: &str, sha: &str) -> Result<(), String> {
    if sha.len() != 40
        || !sha
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{field} must be a 40-character lowercase commit SHA"));
    }
    Ok(())
}

fn validate_artifact(field: &str, artifact: &Artifact) -> Result<(), String> {
    validate_sha256(&format!("{field}.sha256"), &artifact.sha256)?;
    if artifact.byte_length == 0 {
        return Err(format!("{field}.byte_length must be positive"));
    }
    if artifact.path.trim().is_empty()
        || artifact.path.starts_with('/')
        || artifact.path.contains('\\')
        || artifact.path.split('/').any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(format!("{field}.path must be a normalized relative path without traversal"));
    }
    Ok(())
}

fn validate_exchange_profile(profile: &ExchangeProfile) -> Result<(), String> {
    if !matches!(
        profile.length_unit.as_str(),
        "metre" | "millimetre" | "international_foot" | "us_survey_foot"
    ) {
        return Err("exchange_profile.length_unit is unsupported".to_owned());
    }
    if !matches!(
        profile.axis_order.as_str(),
        "XYZ" | "XZY" | "YXZ" | "YZX" | "ZXY" | "ZYX"
    ) {
        return Err("exchange_profile.axis_order is unsupported".to_owned());
    }
    if !matches!(profile.handedness.as_str(), "right" | "left") {
        return Err("exchange_profile.handedness is unsupported".to_owned());
    }
    Ok(())
}

fn validate_gate_binding(gate: &Gate, receipt: &DeliveryReceipt) -> Result<(), String> {
    validate_sha256("gate.replay_receipt_sha256", &gate.replay_receipt_sha256)?;
    if gate.replay_receipt_sha256 != receipt.replay.receipt_sha256 {
        return Err(format!("gate {:?} is bound to a different replay receipt", gate.gate_id));
    }
    if gate.scene_id != receipt.scene.scene_id {
        return Err(format!("gate {:?} is bound to a different scene", gate.gate_id));
    }
    if gate.scope.trim().is_empty() || gate.limitation.trim().is_empty() {
        return Err(format!("gate {:?} must declare scope and limitation", gate.gate_id));
    }
    Ok(())
}

fn validate_gate_subjects(gate: &Gate, receipt: &DeliveryReceipt) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    let mut roles = BTreeSet::new();
    for subject in &gate.subject_artifacts {
        if subject.artifact_id.trim().is_empty() || !ids.insert(subject.artifact_id.as_str()) {
            return Err(format!(
                "gate {:?} has empty or duplicate subject artifact IDs",
                gate.gate_id
            ));
        }
        if !roles.insert(subject.role) {
            return Err(format!("gate {:?} has duplicate subject artifact roles", gate.gate_id));
        }
        validate_sha256("gate.subject_artifacts.sha256", &subject.sha256)?;
        match subject.role {
            SubjectRole::IfcOutput if subject.sha256 != receipt.ifc.sha256 => {
                return Err(format!(
                    "gate {:?} IFC subject digest does not match receipt IFC",
                    gate.gate_id
                ));
            }
            SubjectRole::ElementGraph if subject.sha256 != receipt.element_graph.sha256 => {
                return Err(format!(
                    "gate {:?} element-graph subject digest does not match receipt graph",
                    gate.gate_id
                ));
            }
            SubjectRole::IdsRequirements => {
                let expected = receipt
                    .ids_requirements
                    .as_ref()
                    .ok_or("gate references IDS requirements absent from the receipt")?;
                if subject.sha256 != expected.sha256 {
                    return Err(format!(
                        "gate {:?} IDS subject digest does not match receipt requirements",
                        gate.gate_id
                    ));
                }
            }
            _ => {}
        }
    }

    if gate.status == GateStatus::NotRun
        || (gate.status == GateStatus::Unresolved && gate.producer.is_none())
    {
        return Ok(());
    }

    let required_roles: &[SubjectRole] = match gate.gate_id {
        GateId::ArtifactIntegrity => &[SubjectRole::IfcOutput, SubjectRole::ElementGraph],
        GateId::StepParse | GateId::IfcSchema => &[SubjectRole::IfcOutput],
        GateId::IdsXsdValidity | GateId::IdsSemanticSupport => &[SubjectRole::IdsRequirements],
        GateId::IfcIdsConformance => &[SubjectRole::IfcOutput, SubjectRole::IdsRequirements],
        GateId::SemanticPreservation | GateId::GeometryTopology => {
            &[SubjectRole::IfcOutput, SubjectRole::ElementGraph]
        }
        GateId::EditRoundTrip => &[SubjectRole::IfcOutput, SubjectRole::RoundTripOutput],
    };
    for required in required_roles {
        if !roles.contains(required) {
            return Err(format!(
                "gate {:?} is missing required subject role {required:?}",
                gate.gate_id
            ));
        }
    }
    Ok(())
}

fn validate_gate_execution_shape(gate: &Gate) -> Result<(), String> {
    match gate.status {
        GateStatus::NotRun => {
            if gate.producer.is_some()
                || !gate.evidence.is_empty()
                || !gate.subject_artifacts.is_empty()
            {
                return Err(format!(
                    "NOT_RUN gate {:?} must not claim producer, subjects or evidence",
                    gate.gate_id
                ));
            }
        }
        GateStatus::Pass | GateStatus::Fail => {
            let producer = gate.producer.as_ref().ok_or_else(|| {
                format!("evaluated gate {:?} requires a producer", gate.gate_id)
            })?;
            if producer.tool.trim().is_empty() || producer.version.trim().is_empty() {
                return Err(format!("gate {:?} producer identity must not be empty", gate.gate_id));
            }
            validate_sha256("gate.producer.configuration_sha256", &producer.configuration_sha256)?;
            if gate.evidence.is_empty() || gate.subject_artifacts.is_empty() {
                return Err(format!(
                    "evaluated gate {:?} requires subject artifacts and evidence",
                    gate.gate_id
                ));
            }
        }
        GateStatus::Unresolved => {
            if let Some(producer) = &gate.producer {
                if producer.tool.trim().is_empty() || producer.version.trim().is_empty() {
                    return Err(format!(
                        "gate {:?} producer identity must not be empty",
                        gate.gate_id
                    ));
                }
                validate_sha256(
                    "gate.producer.configuration_sha256",
                    &producer.configuration_sha256,
                )?;
                if gate.evidence.is_empty() || gate.subject_artifacts.is_empty() {
                    return Err(format!(
                        "UNRESOLVED gate {:?} with a producer requires subjects and evidence",
                        gate.gate_id
                    ));
                }
            } else if !gate.evidence.is_empty() || !gate.subject_artifacts.is_empty() {
                return Err(format!(
                    "UNRESOLVED gate {:?} without a producer cannot carry execution evidence",
                    gate.gate_id
                ));
            }
        }
    }

    let mut evidence_ids = BTreeSet::new();
    for evidence in &gate.evidence {
        if evidence.artifact_id.trim().is_empty()
            || evidence.media_type.trim().is_empty()
            || !evidence_ids.insert(evidence.artifact_id.as_str())
        {
            return Err(format!("gate {:?} has empty or duplicate evidence identity", gate.gate_id));
        }
        validate_sha256("gate.evidence.sha256", &evidence.sha256)?;
    }
    for diagnostic in &gate.diagnostics {
        if diagnostic.code.trim().is_empty()
            || diagnostic.message.trim().is_empty()
            || !matches!(diagnostic.severity.as_str(), "info" | "warning" | "error")
        {
            return Err(format!("gate {:?} contains an invalid diagnostic", gate.gate_id));
        }
    }
    Ok(())
}

fn validate_ifc_global_id(global_id: &str) -> Result<(), String> {
    const IFC_GUID_LENGTH: usize = 22;
    if global_id.len() != IFC_GUID_LENGTH
        || !matches!(global_id.as_bytes().first(), Some(b'0'..=b'3'))
        || !global_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == 36)
    {
        return Err(
            "22-character compressed IFC GUID alphabet required; leading range 0..=3"
                .to_owned(),
        );
    }
    Ok(())
}

fn validate_crosswalk(entries: &[CrosswalkEntry]) -> Result<(usize, usize), String> {
    if entries.is_empty() {
        return Ok((0, 0));
    }
    let mut supported_ifc_ids = BTreeSet::new();
    let mut supported_graph_ids = BTreeSet::new();
    let mut unresolved_ifc_ids = BTreeSet::new();
    let mut unresolved_graph_ids = BTreeSet::new();
    let mut supported_count = 0usize;
    let mut unresolved_count = 0usize;

    for entry in entries {
        if entry.ifc_global_id.trim().is_empty() || entry.element_graph_id.trim().is_empty() {
            return Err("crosswalk IDs must not be empty".to_owned());
        }
        validate_ifc_global_id(&entry.ifc_global_id)?;
        if let Some(digest) = &entry.evidence_sha256 {
            validate_sha256("crosswalk.evidence_sha256", digest)?;
        }
        match entry.correspondence {
            CorrespondenceStatus::Supported => {
                if !supported_ifc_ids.insert(entry.ifc_global_id.as_str()) {
                    return Err("supported crosswalk contains a duplicate IFC GlobalId".to_owned());
                }
                if !supported_graph_ids.insert(entry.element_graph_id.as_str()) {
                    return Err(
                        "supported crosswalk contains a duplicate element-graph ID".to_owned(),
                    );
                }
                if entry.evidence_sha256.is_none() {
                    return Err("supported crosswalk entry requires evidence_sha256".to_owned());
                }
                supported_count += 1;
            }
            CorrespondenceStatus::Ambiguous
            | CorrespondenceStatus::Unresolved
            | CorrespondenceStatus::UnmatchedIfc
            | CorrespondenceStatus::UnmatchedGraph => {
                unresolved_ifc_ids.insert(entry.ifc_global_id.as_str());
                unresolved_graph_ids.insert(entry.element_graph_id.as_str());
                unresolved_count += 1;
            }
        }
    }

    if supported_ifc_ids.iter().any(|id| unresolved_ifc_ids.contains(id)) {
        return Err(
            "supported crosswalk conflicts with an ambiguous or unresolved IFC GlobalId".to_owned(),
        );
    }
    if supported_graph_ids
        .iter()
        .any(|id| unresolved_graph_ids.contains(id))
    {
        return Err(
            "supported crosswalk conflicts with an ambiguous or unresolved element-graph ID"
                .to_owned(),
        );
    }
    Ok((supported_count, unresolved_count))
}

#[cfg(test)]
mod tests {
    use super::{
        DeliveryAuditReport, DeliveryDecisionStatus, audit_delivery_receipt_bytes,
        validate_ifc_global_id,
    };
    use serde_json::{Value, json};

    fn fixture_cases() -> Value {
        serde_json::from_str(include_str!(
            "../../../research/scan-bim-ifc-delivery-001/fixtures/known_answers_v0_1.json"
        ))
        .expect("known-answer fixture JSON")
    }

    fn case_report(case_id: &str) -> Value {
        fixture_cases()["cases"]
            .as_array()
            .expect("fixture cases")
            .iter()
            .find(|case| case["case_id"] == case_id)
            .unwrap_or_else(|| panic!("missing fixture case {case_id}"))["report"]
            .clone()
    }

    #[test]
    fn accepts_all_gates_not_run_as_unqualified() {
        let report = audit_delivery_receipt_bytes(
            serde_json::to_string(&case_report("all_gates_not_run"))
                .expect("serialize fixture")
                .as_bytes(),
        )
        .expect("NOT_RUN receipt is valid but unqualified");
        assert_eq!(report.derived_decision, DeliveryDecisionStatus::Unqualified);
        assert_eq!(report.gate_count, 9);
        assert_eq!(report.crosswalk_supported_count, 1);
    }

    #[test]
    fn rejects_forged_qualified_decision() {
        let value = case_report("forged_qualified_with_not_run");
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize fixture").as_bytes(),
        )
        .expect_err("NOT_RUN gates cannot yield QUALIFIED");
        assert!(error.contains("does not match derived decision"));
    }

    #[test]
    fn rejects_gate_subject_digest_mismatch() {
        let value = case_report("gate_subject_digest_mismatch");
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize fixture").as_bytes(),
        )
        .expect_err("gate subject must match exact IFC bytes");
        assert!(error.contains("IFC subject digest does not match"));
    }

    #[test]
    fn rejects_supported_crosswalk_collision() {
        let value = case_report("duplicate_ifc_global_id");
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize fixture").as_bytes(),
        )
        .expect_err("duplicate supported GlobalId must fail closed");
        assert!(error.contains("duplicate IFC GlobalId"));
    }

    #[test]
    fn rejects_missing_required_gate() {
        let value = case_report("missing_required_gate");
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize fixture").as_bytes(),
        )
        .expect_err("missing gates must fail closed");
        assert!(error.contains("exactly one row"));
    }

    fn all_pass_receipt() -> Value {
        let mut value = case_report("all_gates_not_run");
        let ids_digest = "a".repeat(64);
        value["ids_requirements"] = json!({
            "sha256": ids_digest.clone(),
            "standard_version": "1.0",
            "requirement_revision": "synthetic-ids-v0.1"
        });

        let ifc_digest = value["ifc"]["sha256"].as_str().expect("IFC digest").to_owned();
        let graph_digest = value["element_graph"]["sha256"]
            .as_str()
            .expect("graph digest")
            .to_owned();
        let gate_ids = [
            ("artifact_integrity", vec!["ifc_output", "element_graph"]),
            ("step_parse", vec!["ifc_output"]),
            ("ifc_schema", vec!["ifc_output"]),
            ("ids_xsd_validity", vec!["ids_requirements"]),
            ("ids_semantic_support", vec!["ids_requirements"]),
            ("ifc_ids_conformance", vec!["ifc_output", "ids_requirements"]),
            ("semantic_preservation", vec!["ifc_output", "element_graph"]),
            ("geometry_topology", vec!["ifc_output", "element_graph"]),
            ("edit_round_trip", vec!["ifc_output", "round_trip_output"]),
        ];
        for (index, (gate_id, roles)) in gate_ids.into_iter().enumerate() {
            let gate = &mut value["gates"][index];
            gate["gate_id"] = json!(gate_id);
            gate["status"] = json!("PASS");
            gate["producer"] = json!({
                "tool": "Synthetic known-answer validator",
                "version": "0.1",
                "configuration_sha256": "b".repeat(64)
            });
            gate["evidence"] = json!([{
                "artifact_id": format!("evidence-{gate_id}"),
                "sha256": "c".repeat(64),
                "media_type": "application/json"
            }]);
            let mut subjects = Vec::new();
            for (subject_index, role) in roles.into_iter().enumerate() {
                let digest = match role {
                    "ifc_output" => ifc_digest.clone(),
                    "element_graph" => graph_digest.clone(),
                    "ids_requirements" => ids_digest.clone(),
                    "round_trip_output" => "d".repeat(64),
                    _ => unreachable!("known fixture role"),
                };
                subjects.push(json!({
                    "artifact_id": format!("{gate_id}-subject-{subject_index}"),
                    "role": role,
                    "sha256": digest
                }));
            }
            gate["subject_artifacts"] = json!(subjects);
        }
        value["decision"] = json!({"status": "QUALIFIED", "reason_codes": []});
        value
    }

    #[test]
    fn qualifies_only_when_all_nine_gates_and_crosswalk_are_supported() {
        let value = all_pass_receipt();
        let report = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect("fully evidenced synthetic receipt qualifies at the contract layer");
        assert_eq!(report.derived_decision, DeliveryDecisionStatus::Qualified);
        assert_eq!(report.gate_status_counts.get("PASS"), Some(&9));
    }

    #[test]
    fn unresolved_crosswalk_prevents_qualification() {
        let mut value = all_pass_receipt();
        value["crosswalk"][0]["correspondence"] = json!("ambiguous");
        value["decision"] = json!({
            "status": "UNQUALIFIED",
            "reason_codes": ["CROSSWALK_AMBIGUOUS"]
        });
        let report = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect("ambiguous identity must not qualify");
        assert_eq!(report.derived_decision, DeliveryDecisionStatus::Unqualified);
        assert_eq!(report.crosswalk_unresolved_count, 1);
    }

    #[test]
    fn rejects_ids_pass_without_an_ids_requirements_artifact() {
        let mut value = all_pass_receipt();
        value["ids_requirements"] = Value::Null;
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect_err("IDS results cannot be grounded without IDS input");
        assert!(error.contains("gate references IDS requirements absent"));
    }

    #[test]
    fn accepts_buildingsmart_ifc_guid_example() {
        assert!(
            validate_ifc_global_id("3t3TDZl_D9NOIWB0BSjzJI").is_ok(),
            "the IFC 4.3.2.0 published compressed GUID example must pass lexical validation"
        );
    }

    #[test]
    fn rejects_non_alphabet_character_in_ifc_global_id() {
        let error = validate_ifc_global_id("3t3TDZl_D9NOIWB0BSjzJ!").expect_err(
            "characters outside the compressed IFC GUID alphabet must be rejected",
        );
        assert!(error.contains("22-character compressed IFC GUID alphabet"));
    }

    #[test]
    fn rejects_malformed_ifc_global_id() {
        let mut value = case_report("all_gates_not_run");
        value["crosswalk"][0]["ifc_global_id"] = json!("not-a-valid-ifc-guid");
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect_err("IFC GlobalId must use the compressed 22-character alphabet");
        assert!(error.contains("22-character compressed IFC GUID alphabet"));
    }

    #[test]
    fn rejects_ifc_global_id_with_invalid_leading_character() {
        let mut value = case_report("all_gates_not_run");
        value["crosswalk"][0]["ifc_global_id"] = json!("A123456789ABCDEFGHIJKL");
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect_err("the first compressed IFC GUID character must be in 0..=3");
        assert!(error.contains("leading range 0..=3"));
    }

    #[test]
    fn rejects_supported_mapping_that_conflicts_with_ambiguous_identity() {
        let mut value = all_pass_receipt();
        let ambiguous = json!({
            "ifc_global_id": "0123456789ABCDEFGHIJKL",
            "element_graph_id": "wall-ambiguous-alternative",
            "correspondence": "ambiguous",
            "evidence_sha256": "e".repeat(64)
        });
        value["crosswalk"].as_array_mut().expect("crosswalk array").push(ambiguous);
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect_err("one IFC GlobalId cannot be both supported and ambiguous");
        assert!(error.contains("conflicts with an ambiguous or unresolved IFC GlobalId"));
    }

    #[test]
    fn rejects_supported_mapping_that_conflicts_with_unresolved_graph_identity() {
        let mut value = all_pass_receipt();
        let unresolved = json!({
            "ifc_global_id": "1BCDEFGHIJKLMNOPQRSTUV",
            "element_graph_id": "wall-01",
            "correspondence": "unresolved",
            "evidence_sha256": "f".repeat(64)
        });
        value["crosswalk"].as_array_mut().expect("crosswalk array").push(unresolved);
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect_err("one graph element cannot be both supported and unresolved");
        assert!(error.contains("conflicts with an ambiguous or unresolved element-graph ID"));
    }

    #[test]
    fn rejects_path_traversal() {
        let mut value = case_report("all_gates_not_run");
        value["ifc"]["path"] = json!("candidate/../outside.ifc");
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect_err("relative traversal must fail");
        assert!(error.contains("normalized relative path"));
    }

    #[test]
    fn rejects_unknown_fields() {
        let mut value = case_report("all_gates_not_run");
        value["oracle_only_reference_geometry"] = json!("must never be candidate-visible");
        let error = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect_err("unknown fields fail closed");
        assert!(error.contains("unknown field"));
    }

    // Keep the report type referenced by this test module so serialization
    // remains compile-checked independently of the CLI.
    #[test]
    fn audit_report_is_serializable() {
        let value = all_pass_receipt();
        let report: DeliveryAuditReport = audit_delivery_receipt_bytes(
            serde_json::to_string(&value).expect("serialize").as_bytes(),
        )
        .expect("valid synthetic receipt");
        assert!(serde_json::to_string(&report).is_ok());
    }
}
