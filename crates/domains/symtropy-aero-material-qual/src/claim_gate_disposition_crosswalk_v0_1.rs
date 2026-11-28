// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compatibility projection from the existing composite ClaimGate result.
//!
//! This module deliberately does not re-evaluate owner, model, observer,
//! chronology or evidence obligations. It is a presentation/compatibility
//! crosswalk only. The existing evaluate_claim_gate remains the source of the
//! disposition and reason codes until a separately reviewed source adapter
//! demonstrates parity across the complete corpus.

use super::assurance_disposition_v0_1::AssuranceDispositionV0_1;
use super::claim_assurance_gate_v0_1::{ClaimAssuranceAssessment, ClaimGateStatus};

pub const CLAIM_GATE_CROSSWALK_SCHEMA_V0_1: &str = "AeroClaimGateDispositionCrosswalkV0_1";

/// Lossless disposition/reason-code projection of the existing ClaimGate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimGateDispositionCrosswalkV0_1 {
    pub schema: String,
    pub claim_id: String,
    pub decision_context_sha256: String,
    pub owner_evidence_cut_sha256: String,
    pub owner_trust_policy_sha256: String,
    pub owner_observer_coverage_receipt_sha256: String,
    pub model_credibility_sha256: String,
    pub component_report_sha256: [String; 3],
    pub required_evidence_refs: Vec<String>,
    pub required_evidence_admitted_count: usize,
    pub required_evidence_total_count: usize,
    pub claim_gate_status: ClaimGateStatus,
    pub assurance_disposition: AssuranceDispositionV0_1,
    pub reason_codes: Vec<String>,
    pub authority_granted: bool,
    pub physical_qualification_status: String,
    pub absence_inference_granted: bool,
}

/// Deterministic one-to-one mapping for all six current ClaimGate dispositions.
///
/// The richer assurance evaluator may later add NOT_KNOWN_AT_SNAPSHOT as a
/// source-derived temporal state. The current ClaimGate does not expose that
/// status independently, so this crosswalk does not infer it.
pub fn map_claim_gate_status_v0_1(status: ClaimGateStatus) -> AssuranceDispositionV0_1 {
    match status {
        ClaimGateStatus::ReviewableForIndependentDecision => {
            AssuranceDispositionV0_1::ReviewableForIndependentDecision
        }
        ClaimGateStatus::NotEffectiveAtSubjectTime => {
            AssuranceDispositionV0_1::NotEffectiveAtSubjectTime
        }
        ClaimGateStatus::Incomplete => AssuranceDispositionV0_1::Incomplete,
        ClaimGateStatus::Unverified => AssuranceDispositionV0_1::Unverified,
        ClaimGateStatus::OutOfScope => AssuranceDispositionV0_1::OutOfScope,
        ClaimGateStatus::Rejected => AssuranceDispositionV0_1::Rejected,
    }
}

/// Preserve the exact existing ClaimGate result without calculating a new one.
pub fn crosswalk_claim_gate_assessment_v0_1(
    assessment: &ClaimAssuranceAssessment,
) -> ClaimGateDispositionCrosswalkV0_1 {
    ClaimGateDispositionCrosswalkV0_1 {
        schema: CLAIM_GATE_CROSSWALK_SCHEMA_V0_1.to_owned(),
        claim_id: assessment.claim_id.clone(),
        decision_context_sha256: assessment.decision_context_sha256.clone(),
        owner_evidence_cut_sha256: assessment.owner_evidence_cut_sha256.clone(),
        owner_trust_policy_sha256: assessment.owner_trust_policy_sha256.clone(),
        owner_observer_coverage_receipt_sha256: assessment
            .owner_observer_coverage_receipt_sha256
            .clone(),
        model_credibility_sha256: assessment.model_credibility_sha256.clone(),
        component_report_sha256: assessment.component_report_sha256.clone(),
        required_evidence_refs: assessment.required_evidence_refs.clone(),
        required_evidence_admitted_count: assessment.admitted_required_evidence_count,
        required_evidence_total_count: assessment.required_evidence_count,
        claim_gate_status: assessment.status,
        assurance_disposition: map_claim_gate_status_v0_1(assessment.status),
        reason_codes: assessment.reason_codes.clone(),
        authority_granted: false,
        physical_qualification_status: "OUT_OF_SCOPE_NOT_CLAIMED".to_owned(),
        absence_inference_granted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_claim_gate_status_has_an_exact_non_more_permissive_crosswalk() {
        let cases = [
            (
                ClaimGateStatus::ReviewableForIndependentDecision,
                AssuranceDispositionV0_1::ReviewableForIndependentDecision,
            ),
            (
                ClaimGateStatus::NotEffectiveAtSubjectTime,
                AssuranceDispositionV0_1::NotEffectiveAtSubjectTime,
            ),
            (
                ClaimGateStatus::Incomplete,
                AssuranceDispositionV0_1::Incomplete,
            ),
            (
                ClaimGateStatus::Unverified,
                AssuranceDispositionV0_1::Unverified,
            ),
            (
                ClaimGateStatus::OutOfScope,
                AssuranceDispositionV0_1::OutOfScope,
            ),
            (
                ClaimGateStatus::Rejected,
                AssuranceDispositionV0_1::Rejected,
            ),
        ];
        for (source, expected) in cases {
            assert_eq!(map_claim_gate_status_v0_1(source), expected);
        }
    }

    fn assessment(status: ClaimGateStatus) -> ClaimAssuranceAssessment {
        ClaimAssuranceAssessment {
            claim_id: "claim:synthetic-001".to_owned(),
            decision_context_sha256: "a".repeat(64),
            owner_evidence_cut_sha256: "b".repeat(64),
            owner_component_packet_id: "packet:synthetic".to_owned(),
            owner_component_packet_sha256: "c".repeat(64),
            owner_basalt_run_id: "basalt:synthetic".to_owned(),
            owner_basalt_report_version: 1,
            owner_basalt_report_sha256: "d".repeat(64),
            owner_basalt_final_state_sha256: "e".repeat(64),
            owner_basalt_confirmatory_eligible: false,
            owner_artifact_payloads_verified: false,
            owner_receiver_receipt_manifest_sha256: "f".repeat(64),
            owner_trust_policy_sha256: "1".repeat(64),
            owner_observer_coverage_profile_id: "coverage:synthetic".to_owned(),
            owner_observer_coverage_receipt_sha256: "2".repeat(64),
            owner_observer_coverage_status: "UNVERIFIED".to_owned(),
            owner_receiver_receipt_watermark_unix_ms: 10,
            model_credibility_evidence_ref: "model-evidence:synthetic".to_owned(),
            model_credibility_sha256: "3".repeat(64),
            model_ref: "model:synthetic".to_owned(),
            model_artifact_sha256: "4".repeat(64),
            solver_configuration_sha256: "5".repeat(64),
            model_applicability_profile_sha256: "6".repeat(64),
            model_risk_classification_sha256: "7".repeat(64),
            model_validation_package_sha256: "8".repeat(64),
            model_validation_protocol_sha256: "9".repeat(64),
            model_confirmatory_manifest_sha256: "a".repeat(64),
            model_independent_reference_manifest_sha256: "b".repeat(64),
            model_domain_coverage_status: "UNVERIFIED".to_owned(),
            model_split_integrity_status: "UNVERIFIED".to_owned(),
            component_report_sha256: ["c".repeat(64), "d".repeat(64), "e".repeat(64)],
            required_evidence_refs: vec!["evidence:synthetic".to_owned()],
            status,
            reason_codes: vec![
                "SYNTHETIC_BLOCKER_A".to_owned(),
                "SYNTHETIC_BLOCKER_B".to_owned(),
            ],
            admitted_required_evidence_count: 0,
            required_evidence_count: 1,
            authority_granted: false,
            physical_qualification_status: "OUT_OF_SCOPE_NOT_CLAIMED",
            absence_inference_granted: false,
        }
    }

    #[test]
    fn crosswalk_preserves_reasons_and_never_grants_authority() {
        for source in [
            ClaimGateStatus::ReviewableForIndependentDecision,
            ClaimGateStatus::NotEffectiveAtSubjectTime,
            ClaimGateStatus::Incomplete,
            ClaimGateStatus::Unverified,
            ClaimGateStatus::OutOfScope,
            ClaimGateStatus::Rejected,
        ] {
            let original = assessment(source);
            let result = crosswalk_claim_gate_assessment_v0_1(&original);
            assert_eq!(result.claim_gate_status, source);
            assert_eq!(
                result.assurance_disposition,
                map_claim_gate_status_v0_1(source)
            );
            assert_eq!(result.reason_codes, original.reason_codes);
            assert_eq!(
                result.decision_context_sha256,
                original.decision_context_sha256
            );
            assert_eq!(
                result.owner_evidence_cut_sha256,
                original.owner_evidence_cut_sha256
            );
            assert_eq!(
                result.owner_trust_policy_sha256,
                original.owner_trust_policy_sha256
            );
            assert_eq!(
                result.model_credibility_sha256,
                original.model_credibility_sha256
            );
            assert_eq!(
                result.component_report_sha256,
                original.component_report_sha256
            );
            assert!(!result.authority_granted);
            assert_eq!(
                result.physical_qualification_status,
                "OUT_OF_SCOPE_NOT_CLAIMED"
            );
            assert!(!result.absence_inference_granted);
        }
    }
}
