// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Research-only AERO evidence-gap packet adapter.
//!
//! The packet preserves AERO findings for the existing Symthaea evidence-action
//! selector. It does not select actions, calculate EVSI, infer probabilities,
//! authenticate owner receipts, execute tests, or grant physical authority.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const PACKET: &str =
    include_str!("../../../../docs/research/aviation/examples/aero_evidence_gap_packet_v0_1.json");

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionContextV1 {
    pub profile_ref: String,
    pub article_subject_ref: String,
    pub configuration_id: String,
    pub icd_revision: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AssessmentV1 {
    pub assessment_ref: String,
    pub report_digest_sha256: String,
    pub replay_coordinate_digest_sha256: String,
    pub disposition: String,
    pub verifier_profile_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRequirementV1 {
    pub requirement_id: String,
    pub semantic_type: String,
    pub owner_ref: String,
    pub output_contract_ref: String,
    pub mandatory: bool,
    pub independent_evidence_required: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClaimGapV1 {
    pub claim_id: String,
    pub disposition: String,
    pub reason_codes: Vec<String>,
    pub criticality: String,
    pub evidence_requirements: Vec<EvidenceRequirementV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceEvidenceV1 {
    pub id: String,
    pub kind: String,
    pub state: String,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeImpactV1 {
    pub source_owner: String,
    pub receipt_ref: String,
    pub event_frontier_ref: String,
    pub completeness: String,
    pub required_requalification_claims: Vec<String>,
    pub unaffected_claims: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceGapPacketV1 {
    pub schema: String,
    pub record_kind: String,
    pub packet_id: String,
    pub producer_profile: String,
    pub assessment: AssessmentV1,
    pub decision_context: DecisionContextV1,
    pub claims: Vec<ClaimGapV1>,
    pub source_evidence: Vec<SourceEvidenceV1>,
    pub evidence_dependency_groups: BTreeMap<String, Vec<String>>,
    pub change_impact: Option<ChangeImpactV1>,
    pub claim_ceiling: String,
    pub no_actuation: bool,
    pub authority_granted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SymthaeaEvidenceGapProjectionV1 {
    pub decision_context: DecisionContextV1,
    pub target_claim_ids: Vec<String>,
    pub current_evidence_refs: Vec<String>,
    pub unverified_evidence_refs: Vec<String>,
    pub rejected_evidence_refs: Vec<String>,
    pub not_received_evidence_refs: Vec<String>,
    pub current_evidence_dependency_groups: BTreeMap<String, Vec<String>>,
    pub required_obligations: Vec<String>,
    pub change_impact: Option<ChangeImpactV1>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty() && value == value.trim()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn allowed_disposition(value: &str) -> bool {
    matches!(
        value,
        "ACCEPTED_FOR_BINDING_ONLY"
            | "BLOCK"
            | "ABSTAIN"
            | "TRUST_UNRESOLVED"
            | "INCOMPLETE"
            | "OUT_OF_SCOPE"
            | "REJECT"
    )
}

fn valid_reason_code(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

impl EvidenceGapPacketV1 {
    /// Validate packet shape and semantic bindings before projecting it to the
    /// existing Symthaea selector interface.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.schema != "AeroEvidenceGapPacketV1"
            || self.record_kind != "EVIDENCE_GAP_PACKET"
            || self.claim_ceiling != "evidence_gap_packet_only"
            || !self.no_actuation
            || self.authority_granted
        {
            // Kept as one explicit authority/profile gate; this packet is
            // evidence-gap data only and can never grant execution authority.
            if self.schema != "AeroEvidenceGapPacketV1" {
                errors.push("PACKET_SCHEMA_INVALID".to_owned());
            }
            if self.record_kind != "EVIDENCE_GAP_PACKET" {
                errors.push("PACKET_RECORD_KIND_INVALID".to_owned());
            }
            if self.claim_ceiling != "evidence_gap_packet_only" {
                errors.push("PACKET_CLAIM_CEILING_INVALID".to_owned());
            }
            if !self.no_actuation {
                errors.push("PACKET_NO_ACTUATION_REQUIRED".to_owned());
            }
            if self.authority_granted {
                errors.push("PACKET_AUTHORITY_MUST_REMAIN_FALSE".to_owned());
            }
        }

        for (field, value) in [
            ("packet_id", self.packet_id.as_str()),
            ("producer_profile", self.producer_profile.as_str()),
            ("assessment_ref", self.assessment.assessment_ref.as_str()),
            (
                "verifier_profile_id",
                self.assessment.verifier_profile_id.as_str(),
            ),
            ("profile_ref", self.decision_context.profile_ref.as_str()),
            (
                "article_subject_ref",
                self.decision_context.article_subject_ref.as_str(),
            ),
            (
                "configuration_id",
                self.decision_context.configuration_id.as_str(),
            ),
            ("icd_revision", self.decision_context.icd_revision.as_str()),
        ] {
            if !nonempty(value) {
                errors.push(format!("IDENTITY_MISSING_OR_NONCANONICAL_{field}"));
            }
        }

        if !valid_sha256(&self.assessment.report_digest_sha256) {
            errors.push("ASSESSMENT_REPORT_DIGEST_INVALID".to_owned());
        }
        if !valid_sha256(&self.assessment.replay_coordinate_digest_sha256) {
            errors.push("REPLAY_COORDINATE_DIGEST_INVALID".to_owned());
        }
        if !allowed_disposition(&self.assessment.disposition) {
            errors.push("ASSESSMENT_DISPOSITION_UNKNOWN".to_owned());
        }
        if self.claims.is_empty() {
            errors.push("CLAIM_GAP_LIST_EMPTY".to_owned());
        }

        let mut claim_ids = BTreeSet::new();
        let mut requirement_ids = BTreeSet::new();
        for claim in &self.claims {
            if !nonempty(&claim.claim_id) {
                errors.push("CLAIM_ID_MISSING_OR_NONCANONICAL".to_owned());
            } else if !claim_ids.insert(claim.claim_id.as_str()) {
                errors.push("DUPLICATE_CLAIM_ID".to_owned());
            }
            if !allowed_disposition(&claim.disposition) {
                errors.push("CLAIM_DISPOSITION_UNKNOWN".to_owned());
            }
            if !matches!(
                claim.criticality.as_str(),
                "CRITICAL" | "NON_CRITICAL" | "UNKNOWN"
            ) {
                errors.push("CLAIM_CRITICALITY_UNKNOWN".to_owned());
            }

            let mut local_reasons = BTreeSet::new();
            for reason in &claim.reason_codes {
                if !valid_reason_code(reason) {
                    errors.push("CLAIM_REASON_CODE_INVALID".to_owned());
                }
                if !local_reasons.insert(reason.as_str()) {
                    errors.push("DUPLICATE_CLAIM_REASON_CODE".to_owned());
                }
            }

            for requirement in &claim.evidence_requirements {
                if !nonempty(&requirement.requirement_id)
                    || !nonempty(&requirement.semantic_type)
                    || !nonempty(&requirement.owner_ref)
                    || !nonempty(&requirement.output_contract_ref)
                {
                    errors.push("EVIDENCE_REQUIREMENT_IDENTITY_MISSING".to_owned());
                }
                if !requirement_ids.insert(requirement.requirement_id.as_str()) {
                    errors.push("DUPLICATE_EVIDENCE_REQUIREMENT_ID".to_owned());
                }
            }
        }

        let mut source_ids = BTreeSet::new();
        let mut admitted_source_ids = BTreeSet::new();
        for source in &self.source_evidence {
            if !nonempty(&source.id) || !nonempty(&source.kind) {
                errors.push("SOURCE_EVIDENCE_IDENTITY_MISSING".to_owned());
            }
            if !source_ids.insert(source.id.as_str()) {
                errors.push("DUPLICATE_SOURCE_EVIDENCE_ID".to_owned());
            }
            if source
                .sha256
                .as_deref()
                .is_some_and(|digest| !valid_sha256(digest))
            {
                errors.push("SOURCE_EVIDENCE_DIGEST_INVALID".to_owned());
            }
            match source.state.as_str() {
                "ADMITTED" => {
                    if source
                        .sha256
                        .as_deref()
                        .is_none_or(|digest| !valid_sha256(digest))
                    {
                        errors.push("ADMITTED_SOURCE_EVIDENCE_REQUIRES_DIGEST".to_owned());
                    }
                    admitted_source_ids.insert(source.id.as_str());
                }
                "UNVERIFIED" | "REJECTED" | "NOT_RECEIVED" => {}
                _ => errors.push("SOURCE_EVIDENCE_STATE_UNKNOWN".to_owned()),
            }
        }

        for (group_id, members) in &self.evidence_dependency_groups {
            if !nonempty(group_id) || members.is_empty() {
                errors.push("EVIDENCE_DEPENDENCY_GROUP_INVALID".to_owned());
                continue;
            }
            let mut group_members = BTreeSet::new();
            for member in members {
                if !nonempty(member) || !source_ids.contains(member.as_str()) {
                    errors.push("EVIDENCE_DEPENDENCY_MEMBER_UNRESOLVED".to_owned());
                } else if !admitted_source_ids.contains(member.as_str()) {
                    errors.push("EVIDENCE_DEPENDENCY_MEMBER_NOT_ADMITTED".to_owned());
                }
                if !group_members.insert(member.as_str()) {
                    errors.push("EVIDENCE_DEPENDENCY_MEMBER_DUPLICATED".to_owned());
                }
            }
        }

        if let Some(change) = &self.change_impact {
            if change.source_owner != "mycelix:6219"
                || !nonempty(&change.receipt_ref)
                || !nonempty(&change.event_frontier_ref)
                || !matches!(change.completeness.as_str(), "COMPLETE" | "INCOMPLETE")
            {
                errors.push("CHANGE_IMPACT_RECEIPT_INVALID".to_owned());
            }
            let required = change
                .required_requalification_claims
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            let unaffected = change
                .unaffected_claims
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if required.len() != change.required_requalification_claims.len()
                || unaffected.len() != change.unaffected_claims.len()
                || required
                    .iter()
                    .chain(unaffected.iter())
                    .any(|id| !nonempty(id))
                || !required.is_disjoint(&unaffected)
            {
                errors.push("CHANGE_IMPACT_CLAIM_SETS_INVALID".to_owned());
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.into_iter().collect::<BTreeSet<_>>().into_iter().collect())
        }
    }

    /// Losslessly project AERO's exact context, unresolved claims, source
    /// references, dependency groups and owner change frontier. This is not an
    /// action proposal or execution command.
    pub fn selector_projection(&self) -> Result<SymthaeaEvidenceGapProjectionV1, Vec<String>> {
        self.validate()?;
        let target_claim_ids = self
            .claims
            .iter()
            .map(|claim| claim.claim_id.clone())
            .collect();
        let current_evidence_refs = self
            .source_evidence
            .iter()
            .filter(|source| source.state == "ADMITTED")
            .map(|source| source.id.clone())
            .collect();
        let unverified_evidence_refs = self
            .source_evidence
            .iter()
            .filter(|source| source.state == "UNVERIFIED")
            .map(|source| source.id.clone())
            .collect();
        let rejected_evidence_refs = self
            .source_evidence
            .iter()
            .filter(|source| source.state == "REJECTED")
            .map(|source| source.id.clone())
            .collect();
        let not_received_evidence_refs = self
            .source_evidence
            .iter()
            .filter(|source| source.state == "NOT_RECEIVED")
            .map(|source| source.id.clone())
            .collect();
        let mut required_obligations = self
            .claims
            .iter()
            .flat_map(|claim| claim.evidence_requirements.iter())
            .map(|requirement| requirement.requirement_id.clone())
            .collect::<Vec<_>>();
        required_obligations.sort();
        required_obligations.dedup();

        Ok(SymthaeaEvidenceGapProjectionV1 {
            decision_context: self.decision_context.clone(),
            target_claim_ids,
            current_evidence_refs,
            unverified_evidence_refs,
            rejected_evidence_refs,
            not_received_evidence_refs,
            current_evidence_dependency_groups: self.evidence_dependency_groups.clone(),
            required_obligations,
            change_impact: self.change_impact.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{EvidenceGapPacketV1, PACKET};
    use serde_json::{Value, from_str, from_value};

    fn packet() -> EvidenceGapPacketV1 {
        from_str(PACKET).expect("synthetic AERO evidence-gap packet parses")
    }

    #[test]
    fn checked_in_packet_projects_without_losing_context_or_dependency_groups() {
        let packet = packet();
        let projection = packet
            .selector_projection()
            .expect("packet validates and projects");
        assert_eq!(projection.decision_context, packet.decision_context);
        assert_eq!(projection.target_claim_ids.len(), 2);
        assert_eq!(projection.current_evidence_refs.len(), 2);
        assert_eq!(projection.unverified_evidence_refs.len(), 1);
        assert!(projection.rejected_evidence_refs.is_empty());
        assert!(projection.not_received_evidence_refs.is_empty());
        assert_eq!(projection.current_evidence_dependency_groups.len(), 1);
        assert_eq!(projection.required_obligations.len(), 3);
        assert_eq!(
            projection.current_evidence_dependency_groups,
            packet.evidence_dependency_groups
        );
    }

    #[test]
    fn duplicate_claim_ids_cannot_be_projected() {
        let mut value: Value = serde_json::from_str(PACKET).expect("packet JSON");
        let duplicate_id = value["claims"][0]["claim_id"].clone();
        value["claims"][1]["claim_id"] = duplicate_id;
        let packet: EvidenceGapPacketV1 = from_value(value).expect("packet shape");
        assert!(packet.validate().is_err());
    }

    #[test]
    fn dependency_groups_must_reference_admitted_source_evidence() {
        let mut value: Value = serde_json::from_str(PACKET).expect("packet JSON");
        value["evidence_dependency_groups"]["SYNTHETIC::UNKNOWN-GROUP"] =
            serde_json::json!(["SYNTHETIC::NOT-IN-SOURCE-EVIDENCE"]);
        let packet: EvidenceGapPacketV1 = from_value(value).expect("packet shape");
        assert!(packet.validate().is_err());
    }

    #[test]
    fn unverified_evidence_is_not_projected_as_current_evidence() {
        let projection = packet()
            .selector_projection()
            .expect("packet validates and projects");
        assert!(
            !projection
                .current_evidence_refs
                .iter()
                .any(|reference| reference.contains("UNVERIFIED"))
        );
        assert!(
            projection
                .unverified_evidence_refs
                .iter()
                .any(|reference| reference.contains("UNVERIFIED"))
        );
    }

    #[test]
    fn dependency_groups_cannot_treat_unverified_sources_as_admitted() {
        let mut value: Value = serde_json::from_str(PACKET).expect("packet JSON");
        value["evidence_dependency_groups"]["SYNTHETIC::UNVERIFIED-GROUP"] =
            serde_json::json!(["SYNTHETIC::OBSERVER-COVERAGE-RECEIPT-UNVERIFIED"]);
        let packet: EvidenceGapPacketV1 = from_value(value).expect("packet shape");
        assert!(packet.validate().is_err());
    }

    #[test]
    fn packet_cannot_grant_actuation_or_execution_authority() {
        let mut value: Value = serde_json::from_str(PACKET).expect("packet JSON");
        value["authority_granted"] = Value::Bool(true);
        let packet: EvidenceGapPacketV1 = from_value(value).expect("packet shape");
        assert!(packet.validate().is_err());
    }

    #[test]
    fn incomplete_change_impact_is_preserved_not_promoted() {
        let mut packet = packet();
        packet.change_impact = Some(super::ChangeImpactV1 {
            source_owner: "mycelix:6219".to_owned(),
            receipt_ref: "SYNTHETIC::CHANGE-RECEIPT".to_owned(),
            event_frontier_ref: "SYNTHETIC::EVENT-FRONTIER".to_owned(),
            completeness: "INCOMPLETE".to_owned(),
            required_requalification_claims: vec!["claim-A".to_owned()],
            unaffected_claims: vec!["claim-B".to_owned()],
        });
        let projection = packet
            .selector_projection()
            .expect("incomplete frontier is valid evidence-gap input");
        assert_eq!(
            projection.change_impact.expect("change impact").completeness,
            "INCOMPLETE"
        );
    }
}
