// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fail-closed audit of evidence continuity across Scan-to-BIM representation changes.
//!
//! This module audits declared lineage and uncertainty-transfer semantics. It
//! does not authenticate external evidence, recompute survey covariance, parse
//! IFC, validate geometry, or prove that a declared checkpoint is independent.

use crate::delivery::{ReplayBinding, SceneBinding};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RepresentationKind {
    RawObservations,
    RegisteredPointCloud,
    SurfaceMesh,
    SpatialGraph,
    ParametricModel,
    IfcModel,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRole {
    RawMeasurement,
    SurveySolution,
    IndependentCheckpoint,
    Transform,
    RegisteredObservation,
    GeometryOutput,
    PriorState,
    Calibration,
    Other,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ClaimOrigin {
    Measured,
    Inferred,
    Assumed,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CorrespondenceKind {
    OneToOne,
    Split,
    Merge,
    Replacement,
    Unresolved,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyStatus {
    Known,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UncertaintyUpdateBasis {
    RepresentationOnly,
    EvidenceConditioned,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepresentationContinuityReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub replay: ReplayBinding,
    pub scene: SceneBinding,
    pub transitions: Vec<RepresentationTransition>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepresentationTransition {
    pub transition_id: String,
    pub from_representation: RepresentationKind,
    pub to_representation: RepresentationKind,
    pub source_artifacts: Vec<ArtifactReference>,
    pub output_artifacts: Vec<ArtifactReference>,
    pub tool: ToolBinding,
    pub correspondences: Vec<EntityCorrespondence>,
    pub claims: Vec<ClaimLineage>,
    pub uncertainty: UncertaintyTransition,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReference {
    pub artifact_id: String,
    pub sha256: String,
    pub role: EvidenceRole,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolBinding {
    pub name: String,
    pub version: String,
    pub configuration_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntityCorrespondence {
    pub event_id: String,
    pub kind: CorrespondenceKind,
    pub source_entity_ids: Vec<String>,
    pub target_entity_ids: Vec<String>,
    /// Reference to the authoritative identity/correspondence receipt (e.g. #3331).
    pub identity_receipt_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimLineage {
    pub claim_id: String,
    pub target_entity_id: String,
    pub property_id: String,
    pub origin: ClaimOrigin,
    pub source_evidence: Vec<ArtifactReference>,
    pub value_sha256: Option<String>,
    pub derivation_method: Option<String>,
    pub assumptions: Vec<String>,
    pub unknown_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyBound {
    pub status: UncertaintyStatus,
    /// P0 intentionally supports only a conservative absolute geometric bound.
    /// Other uncertainty representations must remain unknown until an adapter
    /// with the appropriate mathematical semantics is implemented.
    pub metric_id: Option<String>,
    pub absolute_bound_m: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UncertaintyTransition {
    pub before: UncertaintyBound,
    pub after: UncertaintyBound,
    /// Deterministic representation/approximation error bound in metres.
    pub approximation_error_bound_m: f64,
    pub update_basis: UncertaintyUpdateBasis,
    /// New evidence used to condition/update the uncertainty estimate.
    pub conditioning_evidence: Vec<ArtifactReference>,
    pub estimator_id: Option<String>,
    pub calibration_profile_id: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct RepresentationContinuityReport {
    pub schema_version: String,
    pub receipt_id: String,
    pub scene_id: String,
    pub transition_count: usize,
    pub claim_count: usize,
    pub measured_claim_count: usize,
    pub inferred_claim_count: usize,
    pub assumed_claim_count: usize,
    pub unknown_claim_count: usize,
    pub correspondence_event_count: usize,
    pub unresolved_correspondence_count: usize,
    pub continuity_state: String,
}

/// Parse and audit the exact receipt bytes. This checks declared receipt
/// semantics only; it does not open or authenticate referenced artifact files.
pub fn audit_representation_continuity_bytes(
    bytes: &[u8],
) -> Result<RepresentationContinuityReport, String> {
    let receipt: RepresentationContinuityReceipt = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid representation continuity receipt: {error}"))?;
    audit_representation_continuity(receipt)
}

pub fn audit_representation_continuity(
    receipt: RepresentationContinuityReceipt,
) -> Result<RepresentationContinuityReport, String> {
    if receipt.schema_version != "0.1" {
        return Err("unsupported representation continuity schema_version".to_owned());
    }
    for (field, value) in [
        ("receipt_id", receipt.receipt_id.as_str()),
        ("replay.run_id", receipt.replay.run_id.as_str()),
        ("scene.building_group_id", receipt.scene.building_group_id.as_str()),
        ("scene.scene_id", receipt.scene.scene_id.as_str()),
        (
            "scene.source_model_revision",
            receipt.scene.source_model_revision.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(format!("{field} must not be empty"));
        }
    }
    validate_sha256("replay.receipt_sha256", &receipt.replay.receipt_sha256)?;
    if receipt.replay.candidate_source_commit.len() != 40
        || !receipt
            .replay
            .candidate_source_commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("replay.candidate_source_commit must be a 40-character lowercase SHA".to_owned());
    }
    if receipt.transitions.is_empty() {
        return Err("continuity receipt must contain at least one transition".to_owned());
    }

    let mut transition_ids = BTreeSet::new();
    let mut totals = BTreeMap::<ClaimOrigin, usize>::new();
    let mut transition_count = 0usize;
    let mut claim_count = 0usize;
    let mut correspondence_event_count = 0usize;
    let mut unresolved_correspondence_count = 0usize;

    for transition in &receipt.transitions {
        transition_count += 1;
        if transition.transition_id.trim().is_empty()
            || !transition_ids.insert(transition.transition_id.as_str())
        {
            return Err("transition IDs must be non-empty and unique".to_owned());
        }
        if transition.from_representation == transition.to_representation {
            return Err(format!(
                "transition {} must change representation kind",
                transition.transition_id
            ));
        }
        validate_tool(&transition.tool)?;
        validate_artifact_set(
            &format!("transition {} source", transition.transition_id),
            &transition.source_artifacts,
        )?;
        validate_artifact_set(
            &format!("transition {} output", transition.transition_id),
            &transition.output_artifacts,
        )?;
        if transition.source_artifacts.is_empty() || transition.output_artifacts.is_empty() {
            return Err(format!(
                "transition {} requires source and output artifacts",
                transition.transition_id
            ));
        }
        let source_artifacts: BTreeMap<&str, (&str, EvidenceRole)> = transition
            .source_artifacts
            .iter()
            .map(|artifact| {
                (
                    artifact.artifact_id.as_str(),
                    (artifact.sha256.as_str(), artifact.role),
                )
            })
            .collect();
        validate_uncertainty(&transition.uncertainty, &source_artifacts)?;

        let mut source_entity_ids = BTreeSet::new();
        let mut target_entity_ids = BTreeSet::new();
        let mut event_ids = BTreeSet::new();
        for correspondence in &transition.correspondences {
            correspondence_event_count += 1;
            if correspondence.event_id.trim().is_empty()
                || !event_ids.insert(correspondence.event_id.as_str())
            {
                return Err(format!(
                    "transition {} correspondence event IDs must be non-empty and unique",
                    transition.transition_id
                ));
            }
            validate_sha256(
                "correspondence.identity_receipt_sha256",
                &correspondence.identity_receipt_sha256,
            )?;
            validate_entity_ids(
                "correspondence.source_entity_ids",
                &correspondence.source_entity_ids,
            )?;
            validate_entity_ids(
                "correspondence.target_entity_ids",
                &correspondence.target_entity_ids,
            )?;
            if correspondence.source_entity_ids.is_empty()
                || correspondence.target_entity_ids.is_empty()
            {
                return Err(format!(
                    "correspondence {} must retain both source and target entity identities",
                    correspondence.event_id
                ));
            }
            validate_correspondence_shape(correspondence)?;
            if correspondence.kind == CorrespondenceKind::Unresolved {
                unresolved_correspondence_count += 1;
            }
            for source_id in &correspondence.source_entity_ids {
                if !source_entity_ids.insert(source_id.as_str()) {
                    return Err(format!(
                        "source entity {source_id} appears in multiple correspondence events"
                    ));
                }
            }
            for target_id in &correspondence.target_entity_ids {
                if !target_entity_ids.insert(target_id.as_str()) {
                    return Err(format!(
                        "target entity {target_id} appears in multiple correspondence events"
                    ));
                }
            }
        }
        if transition.correspondences.is_empty() {
            return Err(format!(
                "transition {} must declare entity correspondence or an explicit unresolved mapping",
                transition.transition_id
            ));
        }

        let mut claim_ids = BTreeSet::new();
        for claim in &transition.claims {
            claim_count += 1;
            if claim.claim_id.trim().is_empty()
                || !claim_ids.insert(claim.claim_id.as_str())
                || claim.target_entity_id.trim().is_empty()
                || claim.property_id.trim().is_empty()
            {
                return Err(format!(
                    "transition {} contains an empty or duplicate claim identity/property",
                    transition.transition_id
                ));
            }
            if !target_entity_ids.contains(claim.target_entity_id.as_str()) {
                return Err(format!(
                    "claim {} target entity is absent from the transition correspondence",
                    claim.claim_id
                ));
            }
            validate_claim(claim, &source_artifacts)?;
            *totals.entry(claim.origin).or_insert(0) += 1;
        }
    }

    if claim_count == 0 {
        return Err("continuity receipt must contain at least one claim lineage row".to_owned());
    }

    Ok(RepresentationContinuityReport {
        schema_version: "0.1".to_owned(),
        receipt_id: receipt.receipt_id,
        scene_id: receipt.scene.scene_id,
        transition_count,
        claim_count,
        measured_claim_count: totals.get(&ClaimOrigin::Measured).copied().unwrap_or(0),
        inferred_claim_count: totals.get(&ClaimOrigin::Inferred).copied().unwrap_or(0),
        assumed_claim_count: totals.get(&ClaimOrigin::Assumed).copied().unwrap_or(0),
        unknown_claim_count: totals.get(&ClaimOrigin::Unknown).copied().unwrap_or(0),
        correspondence_event_count,
        unresolved_correspondence_count,
        continuity_state: if unresolved_correspondence_count == 0 {
            "continuity_preserved".to_owned()
        } else {
            "continuity_unresolved".to_owned()
        },
    })
}

fn validate_sha256(field: &str, value: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{field} must be 64-character lowercase SHA-256"));
    }
    Ok(())
}

fn validate_artifact_set(
    label: &str,
    artifacts: &[ArtifactReference],
) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for artifact in artifacts {
        if artifact.artifact_id.trim().is_empty() || !ids.insert(artifact.artifact_id.as_str()) {
            return Err(format!("{label} artifact IDs must be non-empty and unique"));
        }
        validate_sha256(&format!("{label}.sha256"), &artifact.sha256)?;
    }
    Ok(())
}

fn validate_entity_ids(label: &str, ids: &[String]) -> Result<(), String> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if id.trim().is_empty() || !unique.insert(id.as_str()) {
            return Err(format!("{label} must contain unique non-empty IDs"));
        }
    }
    Ok(())
}

fn validate_tool(tool: &ToolBinding) -> Result<(), String> {
    if tool.name.trim().is_empty() || tool.version.trim().is_empty() {
        return Err("transition tool name and version must not be empty".to_owned());
    }
    validate_sha256("tool.configuration_sha256", &tool.configuration_sha256)
}

fn validate_correspondence_shape(event: &EntityCorrespondence) -> Result<(), String> {
    let source_count = event.source_entity_ids.len();
    let target_count = event.target_entity_ids.len();
    let valid = match event.kind {
        CorrespondenceKind::OneToOne => source_count == 1 && target_count == 1,
        CorrespondenceKind::Split => source_count == 1 && target_count >= 2,
        CorrespondenceKind::Merge => source_count >= 2 && target_count == 1,
        CorrespondenceKind::Replacement => {
            source_count == 1
                && target_count == 1
                && event.source_entity_ids[0] != event.target_entity_ids[0]
        }
        CorrespondenceKind::Unresolved => source_count >= 1 && target_count >= 1,
    };
    if !valid {
        return Err(format!(
            "correspondence event {} has invalid cardinality for {:?}",
            event.event_id, event.kind
        ));
    }
    Ok(())
}

fn validate_claim(
    claim: &ClaimLineage,
    source_artifacts: &BTreeMap<&str, (&str, EvidenceRole)>,
) -> Result<(), String> {
    let mut evidence_ids = BTreeSet::new();
    for evidence in &claim.source_evidence {
        if !evidence_ids.insert(evidence.artifact_id.as_str()) {
            return Err(format!(
                "claim {} contains duplicate source evidence IDs",
                claim.claim_id
            ));
        }
        validate_sha256("claim.source_evidence.sha256", &evidence.sha256)?;
        let (expected_sha, expected_role) = source_artifacts
            .get(evidence.artifact_id.as_str())
            .copied()
            .ok_or_else(|| {
                format!(
                    "claim {} references source evidence absent from the transition source artifacts",
                    claim.claim_id
                )
            })?;
        if expected_sha != evidence.sha256 || expected_role != evidence.role {
            return Err(format!(
                "claim {} source evidence does not match the transition source artifact binding",
                claim.claim_id
            ));
        }
    }

    match claim.origin {
        ClaimOrigin::Measured => {
            let has_direct_measurement = claim.source_evidence.iter().any(|evidence| {
                matches!(
                    evidence.role,
                    EvidenceRole::RawMeasurement
                        | EvidenceRole::SurveySolution
                        | EvidenceRole::IndependentCheckpoint
                )
            });
            if !has_direct_measurement {
                return Err(format!(
                    "measured claim {} requires directly traceable measurement or qualified survey evidence",
                    claim.claim_id
                ));
            }
            if claim.value_sha256.is_none() {
                return Err(format!("measured claim {} requires value_sha256", claim.claim_id));
            }
            if claim.unknown_reason.is_some() {
                return Err(format!(
                    "measured claim {} cannot also declare unknown_reason",
                    claim.claim_id
                ));
            }
        }
        ClaimOrigin::Inferred => {
            if claim.derivation_method.as_deref().is_none_or(str::is_empty)
                || claim.source_evidence.is_empty()
                || claim.value_sha256.is_none()
            {
                return Err(format!(
                    "inferred claim {} requires a derivation method and value digest",
                    claim.claim_id
                ));
            }
        }
        ClaimOrigin::Assumed => {
            if claim.assumptions.is_empty()
                || claim.assumptions.iter().any(|value| value.trim().is_empty())
                || claim.value_sha256.is_none()
            {
                return Err(format!(
                    "assumed claim {} requires explicit assumptions and a value digest",
                    claim.claim_id
                ));
            }
        }
        ClaimOrigin::Unknown => {
            if claim.unknown_reason.as_deref().is_none_or(str::is_empty)
                || claim.value_sha256.is_some()
            {
                return Err(format!(
                    "unknown claim {} requires a reason and must not declare a value digest",
                    claim.claim_id
                ));
            }
        }
    }
    if let Some(value_sha256) = &claim.value_sha256 {
        validate_sha256("claim.value_sha256", value_sha256)?;
    }
    if let Some(method) = &claim.derivation_method {
        if method.trim().is_empty() {
            return Err(format!("claim {} derivation_method must not be empty", claim.claim_id));
        }
    }
    Ok(())
}

fn validate_uncertainty(
    transition: &UncertaintyTransition,
    source_artifacts: &BTreeMap<&str, (&str, EvidenceRole)>,
) -> Result<(), String> {
    if !transition.approximation_error_bound_m.is_finite()
        || transition.approximation_error_bound_m < 0.0
    {
        return Err("approximation_error_bound_m must be finite and non-negative".to_owned());
    }
    validate_uncertainty_state("before", &transition.before)?;
    validate_uncertainty_state("after", &transition.after)?;

    match transition.update_basis {
        UncertaintyUpdateBasis::RepresentationOnly => {
            if !transition.conditioning_evidence.is_empty()
                || transition.estimator_id.is_some()
                || transition.calibration_profile_id.is_some()
            {
                return Err(
                    "representation_only uncertainty update cannot declare conditioning evidence, estimator or calibration profile"
                        .to_owned(),
                );
            }
            match (transition.before.status, transition.after.status) {
                (UncertaintyStatus::Unknown, UncertaintyStatus::Known) => {
                    return Err(
                        "representation-only transition cannot turn unknown uncertainty into known"
                            .to_owned(),
                    );
                }
                (UncertaintyStatus::Known, UncertaintyStatus::Known) => {
                    if transition.before.metric_id != transition.after.metric_id {
                        return Err(
                            "representation-only transition cannot change uncertainty metric"
                                .to_owned(),
                        );
                    }
                    let before = transition.before.absolute_bound_m.unwrap_or(0.0);
                    let after = transition.after.absolute_bound_m.unwrap_or(0.0);
                    let required = before + transition.approximation_error_bound_m;
                    let tolerance = required.abs().max(1.0) * 1.0e-12;
                    if !required.is_finite() || after + tolerance < required {
                        return Err(
                            "representation-only transition unjustifiably shrinks the conservative uncertainty bound"
                                .to_owned(),
                        );
                    }
                }
                _ => {}
            }
        }
        UncertaintyUpdateBasis::EvidenceConditioned => {
            if transition.conditioning_evidence.is_empty()
                || transition.estimator_id.as_deref().is_none_or(str::is_empty)
                || transition
                    .calibration_profile_id
                    .as_deref()
                    .is_none_or(str::is_empty)
            {
                return Err(
                    "evidence-conditioned update requires new evidence, estimator_id and calibration_profile_id"
                        .to_owned(),
                );
            }
            let mut ids = BTreeSet::new();
            let mut digests = BTreeSet::new();
            for evidence in &transition.conditioning_evidence {
                if evidence.role != EvidenceRole::IndependentCheckpoint {
                    return Err(
                        "conditioning evidence must be declared as independent_checkpoint"
                            .to_owned(),
                    );
                }
                if evidence.artifact_id.trim().is_empty()
                    || !ids.insert(evidence.artifact_id.as_str())
                    || !digests.insert(evidence.sha256.as_str())
                {
                    return Err(
                        "conditioning evidence IDs and digests must be unique and non-empty"
                            .to_owned(),
                    );
                }
                validate_sha256("conditioning_evidence.sha256", &evidence.sha256)?;
                if source_artifacts.contains_key(evidence.artifact_id.as_str())
                    || source_artifacts
                        .values()
                        .any(|(source_sha256, _)| *source_sha256 == evidence.sha256)
                {
                    return Err(
                        "conditioning evidence must be additional to the source representation artifacts"
                            .to_owned(),
                    );
                }
            }
        }
    }
    Ok(())
}

fn validate_uncertainty_state(label: &str, state: &UncertaintyBound) -> Result<(), String> {
    match state.status {
        UncertaintyStatus::Known => {
            if state.metric_id.as_deref() != Some("conservative_absolute_bound_m") {
                return Err(format!(
                    "uncertainty.{label} known state must use conservative_absolute_bound_m"
                ));
            }
            let bound = state
                .absolute_bound_m
                .ok_or_else(|| format!("uncertainty.{label} known state requires absolute_bound_m"))?;
            if !bound.is_finite() || bound < 0.0 {
                return Err(format!(
                    "uncertainty.{label}.absolute_bound_m must be finite and non-negative"
                ));
            }
        }
        UncertaintyStatus::Unknown => {
            if state.metric_id.is_some() || state.absolute_bound_m.is_some() {
                return Err(format!(
                    "uncertainty.{label} unknown state must not declare a metric or numeric bound"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{RepresentationContinuityReport, audit_representation_continuity_bytes};
    use serde_json::{Value, json};

    fn fixture_value() -> Value {
        serde_json::from_str(include_str!(
            "../../../research/scan-bim-representation-continuity-001/fixtures/known_answers_v0_1.json"
        ))
        .expect("continuity fixture JSON")
    }

    fn valid_receipt() -> Value {
        fixture_value()["canonical_receipt"].clone()
    }

    fn audit(value: &Value) -> Result<RepresentationContinuityReport, String> {
        let bytes = serde_json::to_vec(value).expect("serialize continuity test receipt");
        audit_representation_continuity_bytes(&bytes)
    }

    #[test]
    fn accepts_lossless_measured_claim_with_complete_source_lineage() {
        let report = audit(&valid_receipt()).expect("valid measured lineage");
        assert_eq!(report.transition_count, 1);
        assert_eq!(report.claim_count, 1);
        assert_eq!(report.measured_claim_count, 1);
        assert_eq!(report.continuity_state, "continuity_preserved");
    }

    #[test]
    fn rejects_measured_claim_supported_only_by_derived_geometry() {
        let mut value = valid_receipt();
        value["transitions"][0]["claims"][0]["source_evidence"] = json!([{
            "artifact_id": "TRANSFORM-001",
            "sha256": "b".repeat(64),
            "role": "transform"
        }]);
        let error = audit(&value).expect_err("a transform is not a direct measurement");
        assert!(error.contains("requires directly traceable measurement"));
    }

    #[test]
    fn rejects_unjustified_uncertainty_shrink_during_representation_change() {
        let mut value = valid_receipt();
        value["transitions"][0]["uncertainty"]["after"]["absolute_bound_m"] = json!(0.001);
        let error = audit(&value).expect_err("representation-only export cannot improve certainty");
        assert!(error.contains("unjustifiably shrinks"));
    }

    #[test]
    fn accepts_conservative_uncertainty_growth_from_approximation_error() {
        let mut value = valid_receipt();
        value["transitions"][0]["uncertainty"]["after"]["absolute_bound_m"] = json!(0.003);
        let report = audit(&value).expect("conservative output bound");
        assert_eq!(report.continuity_state, "continuity_preserved");
    }

    #[test]
    fn rejects_unknown_to_known_uncertainty_without_new_evidence() {
        let mut value = valid_receipt();
        value["transitions"][0]["uncertainty"]["before"] = json!({
            "status": "unknown",
            "metric_id": null,
            "absolute_bound_m": null
        });
        let error = audit(&value).expect_err("representation change cannot create certainty");
        assert!(error.contains("cannot turn unknown uncertainty into known"));
    }

    #[test]
    fn accepts_explicit_evidence_conditioned_uncertainty_update() {
        let mut value = valid_receipt();
        value["transitions"][0]["uncertainty"]["after"]["absolute_bound_m"] = json!(0.001);
        value["transitions"][0]["uncertainty"]["update_basis"] = json!("evidence_conditioned");
        value["transitions"][0]["uncertainty"]["conditioning_evidence"] = json!([{
            "artifact_id": "CHECKPOINT-001",
            "sha256": "f".repeat(64),
            "role": "independent_checkpoint"
        }]);
        value["transitions"][0]["uncertainty"]["estimator_id"] = json!("weighted-plane-update-v1");
        value["transitions"][0]["uncertainty"]["calibration_profile_id"] =
            json!("held-out-metrology-v1");
        let report = audit(&value).expect("evidence-conditioned update is structurally declared");
        assert_eq!(report.measured_claim_count, 1);
    }

    #[test]
    fn rejects_conditioning_evidence_reused_from_source_representation() {
        let mut value = valid_receipt();
        value["transitions"][0]["uncertainty"]["update_basis"] = json!("evidence_conditioned");
        value["transitions"][0]["uncertainty"]["conditioning_evidence"] = json!([{
            "artifact_id": "SCAN-001",
            "sha256": "a".repeat(64),
            "role": "independent_checkpoint"
        }]);
        value["transitions"][0]["uncertainty"]["estimator_id"] = json!("update-v1");
        value["transitions"][0]["uncertainty"]["calibration_profile_id"] = json!("cal-v1");
        let error = audit(&value).expect_err("source evidence cannot be reused as new independent evidence");
        assert!(error.contains("must be additional to the source representation"));
    }

    #[test]
    fn rejects_inferred_claim_without_evidence_or_derivation_assumptions() {
        let mut value = valid_receipt();
        value["transitions"][0]["claims"][0]["origin"] = json!("inferred");
        value["transitions"][0]["claims"][0]["source_evidence"] = json!([]);
        value["transitions"][0]["claims"][0]["derivation_method"] = json!("plane-fit-v1");
        let error = audit(&value).expect_err("inference must retain source support");
        assert!(error.contains("requires a derivation method and value digest"));
    }

    #[test]
    fn rejects_conditioned_update_without_independent_checkpoint() {
        let mut value = valid_receipt();
        value["transitions"][0]["uncertainty"]["update_basis"] = json!("evidence_conditioned");
        value["transitions"][0]["uncertainty"]["conditioning_evidence"] = json!([{
            "artifact_id": "REGISTERED-MESH-001",
            "sha256": "f".repeat(64),
            "role": "geometry_output"
        }]);
        value["transitions"][0]["uncertainty"]["estimator_id"] = json!("update-v1");
        value["transitions"][0]["uncertainty"]["calibration_profile_id"] = json!("cal-v1");
        let error = audit(&value).expect_err("derived geometry cannot be new independent evidence");
        assert!(error.contains("independent_checkpoint"));
    }

    #[test]
    fn rejects_inferred_to_measured_promotion_without_direct_measurement() {
        let mut value = valid_receipt();
        value["transitions"][0]["source_artifacts"][0]["role"] = json!("geometry_output");
        value["transitions"][0]["claims"][0]["source_evidence"][0]["role"] =
            json!("geometry_output");
        let error = audit(&value).expect_err("fitted geometry is not a raw measured observation");
        assert!(error.contains("requires directly traceable measurement"));
    }

    #[test]
    fn rejects_split_correspondence_with_only_one_target() {
        let mut value = valid_receipt();
        value["transitions"][0]["correspondences"][0]["kind"] = json!("split");
        let error = audit(&value).expect_err("split must produce at least two target entities");
        assert!(error.contains("invalid cardinality"));
    }

    #[test]
    fn reports_unresolved_correspondence_without_promoting_it_to_preserved() {
        let mut value = valid_receipt();
        value["transitions"][0]["correspondences"][0]["kind"] = json!("unresolved");
        let report = audit(&value).expect("unresolved identity is reported, not guessed");
        assert_eq!(report.unresolved_correspondence_count, 1);
        assert_eq!(report.continuity_state, "continuity_unresolved");
    }

    #[test]
    fn rejects_orphaned_claim_evidence() {
        let mut value = valid_receipt();
        value["transitions"][0]["claims"][0]["source_evidence"][0]["sha256"] =
            json!("9".repeat(64));
        let error = audit(&value).expect_err("claim evidence must bind exact source bytes");
        assert!(error.contains("does not match the transition source artifact"));
    }

    #[test]
    fn rejects_duplicate_target_identity_across_correspondence_events() {
        let mut value = valid_receipt();
        let mut second = value["transitions"][0]["correspondences"][0].clone();
        second["event_id"] = json!("CORR-002");
        second["source_entity_ids"] = json!(["WALL-RAW-002"]);
        value["transitions"][0]["correspondences"]
            .as_array_mut()
            .expect("correspondence array")
            .push(second);
        let error = audit(&value).expect_err("target identity must not be duplicated");
        assert!(error.contains("target entity"));
    }

    #[test]
    fn rejects_unknown_claim_that_contains_numeric_value() {
        let mut value = valid_receipt();
        value["transitions"][0]["claims"][0]["origin"] = json!("unknown");
        value["transitions"][0]["claims"][0]["unknown_reason"] = json!("no direct support");
        let error = audit(&value).expect_err("unknown claims cannot carry a value digest");
        assert!(error.contains("must not declare a value digest"));
    }

    #[test]
    fn rejects_unknown_fields() {
        let mut value = valid_receipt();
        value["transitions"][0]["unverified_confidence"] = json!(0.99);
        let error = audit(&value).expect_err("unknown continuity fields must fail closed");
        assert!(error.contains("unknown field"));
    }
}
