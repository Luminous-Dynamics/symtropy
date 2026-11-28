// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-bound claim authority, observer coverage and temporal-knowledge obligations.
//!
//! This adapter re-runs the claim-specification/evidence-class source binding,
//! then binds three additional claim-level propositions to owner-approved
//! receipts: acceptance-role authority, observer coverage for the exact source
//! and acquisition scope, and a complete receiver-time snapshot. It never
//! replaces ClaimGate, authenticates signatures, infers physical truth or
//! grants engineering/physical-work authority.

use super::assurance_disposition_v0_1::ObligationKindV0_1;
use super::claim_assurance_gate_v0_1::AssessmentBinding;
use super::claim_obligation_evidence_binding_v0_1::{
    bind_claim_obligation_evidence_v0_1,
    compute_owner_evidence_source_receipt_manifest_sha256_v0_1,
    ClaimObligationEvidenceBindingDispositionV0_1,
    ClaimObligationEvidenceBindingInputV0_1,
    EvidenceBindingReceiptStatusV0_1,
};
use super::claim_obligation_profile_v0_1::{
    derive_claim_obligation_profile_v0_1, DerivedClaimObligationProfileV0_1,
    RegisteredClaimRequirementProfileV0_1,
};
use super::decision_context_v0_1::DecisionContextV0_1;
use super::observer_coverage::{
    CoverageAssessment, CoverageStatus, ReplayConsequence,
};
use super::owner_evidence_cut_v0_1::{
    OwnerEvidenceCutV0_1, OwnerObserverCoverageStatus,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CLAIM_AUTHORITY_OBSERVER_TEMPORAL_SCHEMA_V0_1: &str =
    "AeroClaimAuthorityObserverTemporalBindingV0_1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClaimSourceObligationDispositionV0_1 {
    #[serde(rename = "BOUND")]
    Bound,
    #[serde(rename = "INCOMPLETE")]
    Incomplete,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "OUT_OF_SCOPE")]
    OutOfScope,
    #[serde(rename = "REJECTED")]
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimAcceptanceAuthorityReceiptV0_1 {
    pub verification_status: EvidenceBindingReceiptStatusV0_1,
    pub owner_namespace: String,
    pub authority_holder_namespace: String,
    pub authority_role_ref: String,
    pub claim_id: String,
    pub subject_ref: String,
    pub configuration_ref: String,
    pub decision_context_sha256: String,
    pub trust_policy_sha256: String,
    pub source_scope_manifest_sha256: String,
    /// Receiver first possessed this authority assertion.
    pub receiver_first_seen_at_unix_ms: u64,
    /// Half-open authority effectivity interval.
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
    pub receipt_sha256: String,
}

/// Owner/BASALT receipt for observer coverage of one exact claim scope.
/// The embedded CoverageAssessment remains the independently produced typed
/// evaluation; this receipt binds its report digest, scope and chronology.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimObserverCoverageReceiptV0_1 {
    pub verification_status: EvidenceBindingReceiptStatusV0_1,
    pub owner_namespace: String,
    pub claim_id: String,
    pub subject_ref: String,
    pub configuration_ref: String,
    pub decision_context_sha256: String,
    pub source_scope_manifest_sha256: String,
    pub acquisition_time_earliest_unix_ms: u64,
    pub acquisition_time_latest_unix_ms: u64,
    pub observer_coverage_profile_id: String,
    pub owner_observer_coverage_status: OwnerObserverCoverageStatus,
    pub owner_observer_coverage_receipt_sha256: String,
    pub coverage_report_sha256: String,
    pub coverage_status: CoverageStatus,
    pub replay_consequence: ReplayConsequence,
    pub source_roster_manifest_sha256: String,
    pub required_capture_obligation_manifest_sha256: String,
    pub critical_channel_manifest_sha256: String,
    pub observer_watermark_unix_ms: u64,
    pub receiver_first_seen_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
    pub receipt_sha256: String,
}

/// Historical knowledge snapshot for the exact owner evidence/receipt cut.
/// The complete per-record source-receipt manifest is independently recomputed
/// from the original evidence-binding input before this receipt is consumed.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimTemporalKnowledgeReceiptV0_1 {
    pub verification_status: EvidenceBindingReceiptStatusV0_1,
    pub owner_namespace: String,
    pub claim_id: String,
    pub configuration_ref: String,
    pub decision_context_sha256: String,
    pub trust_policy_sha256: String,
    pub owner_evidence_cut_sha256: String,
    pub source_receipt_manifest_sha256: String,
    pub receiver_receipt_manifest_sha256: String,
    pub receiver_watermark_unix_ms: u64,
    pub as_known_at_unix_ms: u64,
    pub receiver_first_seen_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimAuthorityObserverTemporalBindingInputV0_1<'a> {
    pub schema: String,
    pub requirement_profile: RegisteredClaimRequirementProfileV0_1,
    pub evidence_binding_input: &'a ClaimObligationEvidenceBindingInputV0_1<'a>,
    pub observer_assessment_binding: &'a AssessmentBinding,
    pub observer_coverage: &'a CoverageAssessment,
    pub authority_receipt: Option<&'a ClaimAcceptanceAuthorityReceiptV0_1>,
    pub observer_receipt: Option<&'a ClaimObserverCoverageReceiptV0_1>,
    pub temporal_receipt: Option<&'a ClaimTemporalKnowledgeReceiptV0_1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoundClaimSourceObligationV0_1 {
    pub obligation_id: String,
    pub kind: String,
    pub disposition: ClaimSourceObligationDispositionV0_1,
    pub source_receipt_sha256: String,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClaimAuthorityObserverTemporalBindingAssessmentV0_1 {
    pub disposition: ClaimSourceObligationDispositionV0_1,
    pub claim_id: String,
    pub configuration_ref: String,
    pub as_known_at_unix_ms: u64,
    pub decision_context_sha256: String,
    pub claim_specification_semantic_sha256: String,
    pub obligation_profile_sha256: String,
    pub obligation_manifest_sha256: String,
    pub owner_evidence_cut_sha256: String,
    pub source_receipt_manifest_sha256: String,
    pub obligations: Vec<BoundClaimSourceObligationV0_1>,
    pub unassessed_obligation_ids: Vec<String>,
    pub complete_claim_assurance_state: bool,
    pub assurance_state_mutated: bool,
    pub physical_truth_established: bool,
    pub engineering_acceptance_granted: bool,
    pub physical_work_authorized: bool,
    pub reason_codes: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value == value.trim()
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn effective_at(from: u64, until: u64, cutoff: u64) -> bool {
    from < until && from <= cutoff && cutoff < until
}

fn obligation<'a>(
    profile: &'a DerivedClaimObligationProfileV0_1,
    id: &str,
) -> Option<&'a super::claim_obligation_profile_v0_1::DerivedClaimObligationV0_1> {
    profile.obligations.iter().find(|item| item.obligation_id == id)
}

fn disposition_for_receipt(
    status: EvidenceBindingReceiptStatusV0_1,
) -> ClaimSourceObligationDispositionV0_1 {
    match status {
        EvidenceBindingReceiptStatusV0_1::Verified => ClaimSourceObligationDispositionV0_1::Bound,
        EvidenceBindingReceiptStatusV0_1::Unverified => ClaimSourceObligationDispositionV0_1::Unverified,
        EvidenceBindingReceiptStatusV0_1::Failed => ClaimSourceObligationDispositionV0_1::Rejected,
    }
}

fn make_obligation(
    id: &str,
    kind: &str,
    disposition: ClaimSourceObligationDispositionV0_1,
    receipt_sha256: String,
    reason: Option<String>,
) -> BoundClaimSourceObligationV0_1 {
    BoundClaimSourceObligationV0_1 {
        obligation_id: id.to_owned(),
        kind: kind.to_owned(),
        disposition,
        source_receipt_sha256: receipt_sha256,
        reason_codes: reason.into_iter().collect(),
    }
}

fn validate_authority_receipt(
    receipt: Option<&ClaimAcceptanceAuthorityReceiptV0_1>,
    owner_cut: &OwnerEvidenceCutV0_1,
    spec: &super::claim_specification_v0_1::ClaimSpecificationV0_1,
    context: &DecisionContextV0_1,
    context_sha: &str,
    cutoff: u64,
) -> (ClaimSourceObligationDispositionV0_1, String, Option<String>) {
    let Some(receipt) = receipt else {
        return (
            ClaimSourceObligationDispositionV0_1::Incomplete,
            String::new(),
            Some("CLAIM_ACCEPTANCE_AUTHORITY_RECEIPT_MISSING".to_owned()),
        );
    };
    let status_disposition = disposition_for_receipt(receipt.verification_status);
    let mut reason = None;
    if !valid_identifier(&receipt.owner_namespace)
        || !valid_identifier(&receipt.authority_holder_namespace)
        || !valid_identifier(&receipt.authority_role_ref)
        || !valid_sha256(&receipt.receipt_sha256)
        || receipt.owner_namespace != owner_cut.owner_namespace
        || receipt.authority_role_ref != spec.acceptance_authority_role_ref
        || receipt.claim_id != spec.claim_id
        || receipt.subject_ref != spec.subject_ref
        || receipt.configuration_ref != spec.configuration_ref
        || receipt.decision_context_sha256 != context_sha
        || receipt.trust_policy_sha256 != context.trust_policy_sha256
        || receipt.source_scope_manifest_sha256 != context.source_scope_manifest_sha256
    {
        reason = Some("CLAIM_ACCEPTANCE_AUTHORITY_SCOPE_OR_COMMITMENT_MISMATCH".to_owned());
    } else if receipt.receiver_first_seen_at_unix_ms > cutoff {
        reason = Some("CLAIM_ACCEPTANCE_AUTHORITY_NOT_KNOWN_AT_CUTOFF".to_owned());
    } else if !effective_at(receipt.effective_from_unix_ms, receipt.effective_until_unix_ms, cutoff) {
        reason = Some("CLAIM_ACCEPTANCE_AUTHORITY_NOT_EFFECTIVE_AT_CUTOFF".to_owned());
    }
    let disposition = if reason.as_deref().is_some_and(|r| r.contains("MISMATCH")) {
        ClaimSourceObligationDispositionV0_1::Rejected
    } else if reason.is_some() {
        ClaimSourceObligationDispositionV0_1::Unverified
    } else {
        status_disposition
    };
    (disposition, receipt.receipt_sha256.clone(), reason)
}

fn validate_observer_receipt(
    receipt: Option<&ClaimObserverCoverageReceiptV0_1>,
    owner_cut: &OwnerEvidenceCutV0_1,
    context: &DecisionContextV0_1,
    context_sha: &str,
    binding: &AssessmentBinding,
    assessment: &CoverageAssessment,
) -> (ClaimSourceObligationDispositionV0_1, String, Option<String>) {
    let Some(receipt) = receipt else {
        return (
            ClaimSourceObligationDispositionV0_1::Incomplete,
            String::new(),
            Some("CLAIM_OBSERVER_COVERAGE_RECEIPT_MISSING".to_owned()),
        );
    };
    let mut reason = None;
    if !valid_identifier(&receipt.owner_namespace)
        || !valid_identifier(&receipt.observer_coverage_profile_id)
        || !valid_sha256(&receipt.receipt_sha256)
        || !valid_sha256(&receipt.coverage_report_sha256)
        || !valid_sha256(&receipt.source_roster_manifest_sha256)
        || !valid_sha256(&receipt.required_capture_obligation_manifest_sha256)
        || !valid_sha256(&receipt.critical_channel_manifest_sha256)
        || receipt.owner_namespace != owner_cut.owner_namespace
        || receipt.claim_id != context.claim_id
        || receipt.subject_ref != context.subject_ref
        || receipt.configuration_ref != context.configuration_ref
        || receipt.decision_context_sha256 != context_sha
        || receipt.source_scope_manifest_sha256 != context.source_scope_manifest_sha256
        || receipt.acquisition_time_earliest_unix_ms != context.acquisition_time_earliest_unix_ms
        || receipt.acquisition_time_latest_unix_ms != context.acquisition_time_latest_unix_ms
        || receipt.observer_coverage_profile_id != owner_cut.observer_coverage_profile_id
        || receipt.owner_observer_coverage_status != owner_cut.observer_coverage_status
        || receipt.owner_observer_coverage_receipt_sha256 != owner_cut.observer_coverage_receipt_sha256
        || (assessment.status == CoverageStatus::QualifiedCompleteForScope
            && owner_cut.observer_coverage_status != OwnerObserverCoverageStatus::QualifiedCompleteForScope)
        || (assessment.status != CoverageStatus::QualifiedCompleteForScope
            && owner_cut.observer_coverage_status == OwnerObserverCoverageStatus::QualifiedCompleteForScope)
        || receipt.coverage_report_sha256 != binding.report_sha256
        || receipt.coverage_status != assessment.status
        || receipt.replay_consequence != assessment.replay_consequence
        || receipt.observer_watermark_unix_ms != owner_cut.observer_coverage_watermark_unix_ms
        || receipt.observer_watermark_unix_ms > owner_cut.receiver_receipt_watermark_unix_ms
    {
        reason = Some("CLAIM_OBSERVER_COVERAGE_SCOPE_OR_REPORT_MISMATCH".to_owned());
    } else if receipt.receiver_first_seen_at_unix_ms > context.as_known_at_unix_ms
        || receipt.observer_watermark_unix_ms > context.as_known_at_unix_ms
    {
        reason = Some("CLAIM_OBSERVER_COVERAGE_NOT_KNOWN_AT_CUTOFF".to_owned());
    } else if !effective_at(
        receipt.effective_from_unix_ms,
        receipt.effective_until_unix_ms,
        context.as_known_at_unix_ms,
    ) {
        reason = Some("CLAIM_OBSERVER_COVERAGE_NOT_EFFECTIVE_AT_CUTOFF".to_owned());
    }
    let disposition = if reason.as_deref().is_some_and(|r| r.contains("MISMATCH")) {
        ClaimSourceObligationDispositionV0_1::Rejected
    } else if reason.is_some() {
        ClaimSourceObligationDispositionV0_1::Unverified
    } else {
        match receipt.verification_status {
            EvidenceBindingReceiptStatusV0_1::Failed => {
                ClaimSourceObligationDispositionV0_1::Rejected
            }
            EvidenceBindingReceiptStatusV0_1::Unverified => {
                ClaimSourceObligationDispositionV0_1::Unverified
            }
            EvidenceBindingReceiptStatusV0_1::Verified => match assessment.status {
                CoverageStatus::QualifiedCompleteForScope => ClaimSourceObligationDispositionV0_1::Bound,
                CoverageStatus::Incomplete => ClaimSourceObligationDispositionV0_1::Incomplete,
                CoverageStatus::Unverified => ClaimSourceObligationDispositionV0_1::Unverified,
                CoverageStatus::Invalid => ClaimSourceObligationDispositionV0_1::Rejected,
                CoverageStatus::OutOfScope => ClaimSourceObligationDispositionV0_1::OutOfScope,
            },
        }
    };
    (disposition, receipt.receipt_sha256.clone(), reason)
}

fn validate_temporal_receipt(
    receipt: Option<&ClaimTemporalKnowledgeReceiptV0_1>,
    owner_cut: &OwnerEvidenceCutV0_1,
    context: &DecisionContextV0_1,
    context_sha: &str,
    owner_cut_sha: &str,
    source_receipt_manifest_sha: &str,
) -> (ClaimSourceObligationDispositionV0_1, String, Option<String>) {
    let Some(receipt) = receipt else {
        return (
            ClaimSourceObligationDispositionV0_1::Incomplete,
            String::new(),
            Some("CLAIM_TEMPORAL_KNOWLEDGE_RECEIPT_MISSING".to_owned()),
        );
    };
    let cutoff = context.as_known_at_unix_ms;
    let mut reason = None;
    if !valid_identifier(&receipt.owner_namespace)
        || !valid_sha256(&receipt.receipt_sha256)
        || receipt.owner_namespace != owner_cut.owner_namespace
        || receipt.claim_id != context.claim_id
        || receipt.configuration_ref != context.configuration_ref
        || receipt.decision_context_sha256 != context_sha
        || receipt.trust_policy_sha256 != context.trust_policy_sha256
        || receipt.owner_evidence_cut_sha256 != owner_cut_sha
        || receipt.source_receipt_manifest_sha256 != source_receipt_manifest_sha
        || receipt.receiver_receipt_manifest_sha256 != owner_cut.receiver_receipt_manifest_sha256
        || receipt.receiver_watermark_unix_ms != owner_cut.receiver_receipt_watermark_unix_ms
        || receipt.as_known_at_unix_ms != cutoff
    {
        reason = Some("CLAIM_TEMPORAL_KNOWLEDGE_SCOPE_OR_MANIFEST_MISMATCH".to_owned());
    } else if receipt.receiver_first_seen_at_unix_ms > cutoff
        || receipt.receiver_watermark_unix_ms > cutoff
    {
        reason = Some("CLAIM_TEMPORAL_KNOWLEDGE_NOT_KNOWN_AT_CUTOFF".to_owned());
    } else if !effective_at(receipt.effective_from_unix_ms, receipt.effective_until_unix_ms, cutoff) {
        reason = Some("CLAIM_TEMPORAL_KNOWLEDGE_NOT_EFFECTIVE_AT_CUTOFF".to_owned());
    }
    let disposition = if reason.as_deref().is_some_and(|r| r.contains("MISMATCH")) {
        ClaimSourceObligationDispositionV0_1::Rejected
    } else if reason.is_some() {
        ClaimSourceObligationDispositionV0_1::Unverified
    } else {
        disposition_for_receipt(receipt.verification_status)
    };
    (disposition, receipt.receipt_sha256.clone(), reason)
}

/// Bind the three remaining claim-level sources that can be reconstructed
/// directly from the specification, owner receipt cut and observer assessment:
/// acceptance-role authority, observer coverage, and historical receipt time.
pub fn bind_claim_authority_observer_temporal_v0_1(
    input: ClaimAuthorityObserverTemporalBindingInputV0_1<'_>,
) -> ClaimAuthorityObserverTemporalBindingAssessmentV0_1 {
    let evidence = bind_claim_obligation_evidence_v0_1(input.evidence_binding_input.clone());
    let spec = input.evidence_binding_input.claim_specification;
    let context = input.evidence_binding_input.decision_context;
    let owner_cut = input.evidence_binding_input.owner_evidence_cut;
    let mut reasons = BTreeSet::new();

    let semantic_sha = match spec.compute_semantic_sha256() {
        Ok(value) => value,
        Err(_) => {
            reasons.insert("CLAIM_SOURCE_SPECIFICATION_INVALID".to_owned());
            String::new()
        }
    };
    let context_sha = context.compute_sha256().unwrap_or_default();
    let profile = derive_claim_obligation_profile_v0_1(spec, input.requirement_profile).profile;
    let Some(profile) = profile else {
        return ClaimAuthorityObserverTemporalBindingAssessmentV0_1 {
            disposition: ClaimSourceObligationDispositionV0_1::Rejected,
            claim_id: spec.claim_id.clone(),
            configuration_ref: spec.configuration_ref.clone(),
            as_known_at_unix_ms: context.as_known_at_unix_ms,
            decision_context_sha256: context_sha,
            claim_specification_semantic_sha256: semantic_sha,
            obligation_profile_sha256: String::new(),
            obligation_manifest_sha256: String::new(),
            owner_evidence_cut_sha256: String::new(),
            source_receipt_manifest_sha256: String::new(),
            obligations: Vec::new(),
            unassessed_obligation_ids: Vec::new(),
            complete_claim_assurance_state: false,
            assurance_state_mutated: false,
            physical_truth_established: false,
            engineering_acceptance_granted: false,
            physical_work_authorized: false,
            reason_codes: vec!["CLAIM_SOURCE_OBLIGATION_PROFILE_NOT_DERIVED".to_owned()],
        };
    };
    if input.schema != CLAIM_AUTHORITY_OBSERVER_TEMPORAL_SCHEMA_V0_1 {
        reasons.insert("CLAIM_AUTHORITY_OBSERVER_TEMPORAL_SCHEMA_INVALID".to_owned());
    }
    if input.requirement_profile != input.evidence_binding_input.requirement_profile {
        reasons.insert("CLAIM_SOURCE_REQUIREMENT_PROFILE_MISMATCH".to_owned());
    }
    if evidence.disposition != ClaimObligationEvidenceBindingDispositionV0_1::Bound {
        reasons.insert("CLAIM_SOURCE_UPSTREAM_EVIDENCE_BINDING_NOT_BOUND".to_owned());
    }
    if evidence.claim_specification_semantic_sha256 != semantic_sha
        || evidence.decision_context_sha256 != context_sha
    {
        reasons.insert("CLAIM_SOURCE_UPSTREAM_BINDING_SCOPE_MISMATCH".to_owned());
    }

    let owner_cut_sha = owner_cut.compute_sha256().unwrap_or_default();
    let source_receipt_manifest_sha =
        compute_owner_evidence_source_receipt_manifest_sha256_v0_1(
            input.evidence_binding_input.source_receipts,
        )
        .unwrap_or_default();

    let upstream_claim_disposition = match evidence.disposition {
        ClaimObligationEvidenceBindingDispositionV0_1::Bound => ClaimSourceObligationDispositionV0_1::Bound,
        ClaimObligationEvidenceBindingDispositionV0_1::Incomplete => ClaimSourceObligationDispositionV0_1::Incomplete,
        ClaimObligationEvidenceBindingDispositionV0_1::Unverified => ClaimSourceObligationDispositionV0_1::Unverified,
        ClaimObligationEvidenceBindingDispositionV0_1::Rejected => ClaimSourceObligationDispositionV0_1::Rejected,
    };
    let upstream_claim_reason = (upstream_claim_disposition != ClaimSourceObligationDispositionV0_1::Bound)
        .then(|| "CLAIM_SPECIFICATION_OR_EVIDENCE_BINDING_NOT_BOUND".to_owned());
    let claim_identity = obligation(&profile, "claim-identity")
        .map(|item| make_obligation(
            &item.obligation_id,
            "CLAIM_IDENTITY",
            upstream_claim_disposition,
            semantic_sha.clone(),
            upstream_claim_reason.clone(),
        ));
    let semantic_validity = obligation(&profile, "semantic-validity")
        .map(|item| make_obligation(
            &item.obligation_id,
            "SEMANTIC_VALIDITY",
            upstream_claim_disposition,
            semantic_sha.clone(),
            upstream_claim_reason.clone(),
        ));

    let authority = validate_authority_receipt(
        input.authority_receipt,
        owner_cut,
        spec,
        context,
        &context_sha,
        context.as_known_at_unix_ms,
    );
    let observer = validate_observer_receipt(
        input.observer_receipt,
        owner_cut,
        context,
        &context_sha,
        input.observer_assessment_binding,
        input.observer_coverage,
    );
    let temporal = validate_temporal_receipt(
        input.temporal_receipt,
        owner_cut,
        context,
        &context_sha,
        &owner_cut_sha,
        &source_receipt_manifest_sha,
    );

    let mut bound_obligations = Vec::new();
    if let Some(item) = claim_identity { bound_obligations.push(item); }
    if let Some(item) = semantic_validity { bound_obligations.push(item); }
    if let Some(profile_obligation) = obligation(&profile, "producer-authority") {
        bound_obligations.push(make_obligation(
            &profile_obligation.obligation_id,
            "PRODUCER_AUTHORITY",
            authority.0,
            authority.1.clone(),
            authority.2.clone(),
        ));
    }
    if let Some(profile_obligation) = obligation(&profile, "dependency-observation") {
        bound_obligations.push(make_obligation(
            &profile_obligation.obligation_id,
            "DEPENDENCY_OBSERVATION",
            observer.0,
            observer.1.clone(),
            observer.2.clone(),
        ));
    }
    if let Some(profile_obligation) = obligation(&profile, "temporal-knowledge") {
        bound_obligations.push(make_obligation(
            &profile_obligation.obligation_id,
            "TEMPORAL_KNOWLEDGE",
            temporal.0,
            temporal.1.clone(),
            temporal.2.clone(),
        ));
    }
    bound_obligations.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));

    for item in &bound_obligations {
        if item.disposition != ClaimSourceObligationDispositionV0_1::Bound {
            reasons.extend(item.reason_codes.iter().cloned());
            if item.reason_codes.is_empty() {
                reasons.insert(format!("CLAIM_SOURCE_OBLIGATION_NOT_BOUND:{}", item.obligation_id));
            }
        }
    }
    if !evidence.reason_codes.is_empty() {
        reasons.extend(evidence.reason_codes.iter().cloned());
    }
    if owner_cut.receiver_receipt_watermark_unix_ms > context.as_known_at_unix_ms
        || owner_cut.observer_coverage_watermark_unix_ms > context.as_known_at_unix_ms
    {
        reasons.insert("CLAIM_SOURCE_OWNER_CUTOFF_AFTER_AS_KNOWN".to_owned());
    }

    let mut unassessed = profile.obligations.iter()
        .filter(|item| ![
            "claim-identity",
            "semantic-validity",
            "producer-authority",
            "dependency-observation",
            "temporal-knowledge",
        ].contains(&item.obligation_id.as_str()))
        .map(|item| item.obligation_id.clone())
        .collect::<Vec<_>>();
    unassessed.sort();
    if !unassessed.is_empty() {
        reasons.insert("OTHER_DERIVED_OBLIGATIONS_REMAIN_UNASSESSED".to_owned());
    }

    let has_rejected = bound_obligations.iter().any(|item| item.disposition == ClaimSourceObligationDispositionV0_1::Rejected)
        || reasons.iter().any(|reason| reason.contains("MISMATCH") || reason.contains("INVALID"));
    let has_out_of_scope = bound_obligations.iter().any(|item| item.disposition == ClaimSourceObligationDispositionV0_1::OutOfScope);
    let has_unverified = bound_obligations.iter().any(|item| item.disposition == ClaimSourceObligationDispositionV0_1::Unverified);
    let has_incomplete = bound_obligations.iter().any(|item| item.disposition == ClaimSourceObligationDispositionV0_1::Incomplete);
    let disposition = if has_rejected {
        ClaimSourceObligationDispositionV0_1::Rejected
    } else if has_out_of_scope {
        ClaimSourceObligationDispositionV0_1::OutOfScope
    } else if has_unverified {
        ClaimSourceObligationDispositionV0_1::Unverified
    } else if has_incomplete || !unassessed.is_empty() {
        ClaimSourceObligationDispositionV0_1::Incomplete
    } else {
        ClaimSourceObligationDispositionV0_1::Bound
    };

    ClaimAuthorityObserverTemporalBindingAssessmentV0_1 {
        disposition,
        claim_id: spec.claim_id.clone(),
        configuration_ref: spec.configuration_ref.clone(),
        as_known_at_unix_ms: context.as_known_at_unix_ms,
        decision_context_sha256: context_sha,
        claim_specification_semantic_sha256: semantic_sha,
        obligation_profile_sha256: profile.profile_sha256,
        obligation_manifest_sha256: profile.obligation_manifest_sha256,
        owner_evidence_cut_sha256: owner_cut_sha,
        source_receipt_manifest_sha256: source_receipt_manifest_sha,
        obligations: bound_obligations,
        unassessed_obligation_ids: unassessed,
        complete_claim_assurance_state: false,
        assurance_state_mutated: false,
        physical_truth_established: false,
        engineering_acceptance_granted: false,
        physical_work_authorized: false,
        reason_codes: reasons.into_iter().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim_obligation_evidence_binding_v0_1::tests::build_case as build_evidence_case;

    const CUTOFF: u64 = 1_795_516_800_000;

    fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
    }

    struct Case {
        raw: Vec<u8>,
        spec: super::super::claim_specification_v0_1::ClaimSpecificationV0_1,
        context: DecisionContextV0_1,
        owner_cut: OwnerEvidenceCutV0_1,
        admissions: Vec<super::super::claim_assurance_gate_v0_1::RequiredEvidenceAdmission>,
        evidence_entries: Vec<super::super::claim_obligation_evidence_binding_v0_1::ClaimObligationEvidenceMappingEntryV0_1>,
        evidence_approval: super::super::claim_obligation_evidence_binding_v0_1::ClaimObligationEvidenceMappingReceiptV0_1,
        source_receipts: Vec<super::super::claim_obligation_evidence_binding_v0_1::OwnerEvidenceSourceReceiptV0_1>,
        authority: ClaimAcceptanceAuthorityReceiptV0_1,
        observer: ClaimObserverCoverageReceiptV0_1,
        temporal: ClaimTemporalKnowledgeReceiptV0_1,
        observer_binding: AssessmentBinding,
        coverage: CoverageAssessment,
    }

    fn build_case() -> Case {
        let (raw, spec, context, owner_cut, admissions, evidence_entries, evidence_approval, source_receipts) =
            build_evidence_case();
        let context_sha = context.compute_sha256().expect("context SHA");
        let owner_cut_sha = owner_cut.compute_sha256().expect("owner cut SHA");
        let source_manifest_sha = compute_owner_evidence_source_receipt_manifest_sha256_v0_1(&source_receipts)
            .expect("source receipt manifest");
        let coverage = CoverageAssessment {
            status: CoverageStatus::QualifiedCompleteForScope,
            replay_consequence: coverage.replay_consequence.clone(),
            reason_codes: vec![],
        };
        let observer_binding = AssessmentBinding {
            claim_id: spec.claim_id.clone(),
            subject_ref: spec.subject_ref.clone(),
            decision_context_sha256: context_sha.clone(),
            report_sha256: "9".repeat(64),
            evaluator_profile: "observer-coverage-v0.1".to_owned(),
        };
        let authority = ClaimAcceptanceAuthorityReceiptV0_1 {
            verification_status: EvidenceBindingReceiptStatusV0_1::Verified,
            owner_namespace: owner_cut.owner_namespace.clone(),
            authority_holder_namespace: "authority:independent-structures-reviewer".to_owned(),
            authority_role_ref: spec.acceptance_authority_role_ref.clone(),
            claim_id: spec.claim_id.clone(),
            subject_ref: spec.subject_ref.clone(),
            configuration_ref: spec.configuration_ref.clone(),
            decision_context_sha256: context_sha.clone(),
            trust_policy_sha256: context.trust_policy_sha256.clone(),
            source_scope_manifest_sha256: context.source_scope_manifest_sha256.clone(),
            receiver_first_seen_at_unix_ms: CUTOFF - 100,
            effective_from_unix_ms: CUTOFF - 200,
            effective_until_unix_ms: CUTOFF + 200,
            receipt_sha256: "a".repeat(64),
        };
        let observer = ClaimObserverCoverageReceiptV0_1 {
            verification_status: EvidenceBindingReceiptStatusV0_1::Verified,
            owner_namespace: owner_cut.owner_namespace.clone(),
            claim_id: spec.claim_id.clone(),
            subject_ref: spec.subject_ref.clone(),
            configuration_ref: spec.configuration_ref.clone(),
            decision_context_sha256: context_sha.clone(),
            source_scope_manifest_sha256: context.source_scope_manifest_sha256.clone(),
            acquisition_time_earliest_unix_ms: context.acquisition_time_earliest_unix_ms,
            acquisition_time_latest_unix_ms: context.acquisition_time_latest_unix_ms,
            observer_coverage_profile_id: owner_cut.observer_coverage_profile_id.clone(),
            owner_observer_coverage_status: owner_cut.observer_coverage_status.clone(),
            owner_observer_coverage_receipt_sha256: owner_cut.observer_coverage_receipt_sha256.clone(),
            coverage_report_sha256: observer_binding.report_sha256.clone(),
            coverage_status: coverage.status.clone(),
            replay_consequence: ReplayConsequence::AllowScopedAbsenceInference,
            source_roster_manifest_sha256: "b".repeat(64),
            required_capture_obligation_manifest_sha256: "c".repeat(64),
            critical_channel_manifest_sha256: "d".repeat(64),
            observer_watermark_unix_ms: owner_cut.observer_coverage_watermark_unix_ms,
            receiver_first_seen_at_unix_ms: CUTOFF - 100,
            effective_from_unix_ms: CUTOFF - 200,
            effective_until_unix_ms: CUTOFF + 200,
            receipt_sha256: "e".repeat(64),
        };
        let temporal = ClaimTemporalKnowledgeReceiptV0_1 {
            verification_status: EvidenceBindingReceiptStatusV0_1::Verified,
            owner_namespace: owner_cut.owner_namespace.clone(),
            claim_id: spec.claim_id.clone(),
            configuration_ref: spec.configuration_ref.clone(),
            decision_context_sha256: context_sha,
            trust_policy_sha256: context.trust_policy_sha256.clone(),
            owner_evidence_cut_sha256: owner_cut_sha,
            source_receipt_manifest_sha256: source_manifest_sha,
            receiver_receipt_manifest_sha256: owner_cut.receiver_receipt_manifest_sha256.clone(),
            receiver_watermark_unix_ms: owner_cut.receiver_receipt_watermark_unix_ms,
            as_known_at_unix_ms: CUTOFF,
            receiver_first_seen_at_unix_ms: CUTOFF - 100,
            effective_from_unix_ms: CUTOFF - 200,
            effective_until_unix_ms: CUTOFF + 200,
            receipt_sha256: "f".repeat(64),
        };
        Case {
            raw,
            spec,
            context,
            owner_cut,
            admissions,
            evidence_entries,
            evidence_approval,
            source_receipts,
            authority,
            observer,
            temporal,
            observer_binding,
            coverage,
        }
    }

    impl Case {
        fn evaluate(&self) -> ClaimAuthorityObserverTemporalBindingAssessmentV0_1 {
            let raw_sha = sha256(&self.raw);
            let evidence_input = ClaimObligationEvidenceBindingInputV0_1 {
                schema: super::super::claim_obligation_evidence_binding_v0_1::CLAIM_OBLIGATION_EVIDENCE_BINDING_SCHEMA_V0_1.to_owned(),
                raw_claim_specification_payload: &self.raw,
                expected_raw_claim_specification_sha256: &raw_sha,
                claim_specification: &self.spec,
                decision_context: &self.context,
                requirement_profile: RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
                owner_evidence_cut: &self.owner_cut,
                required_evidence_admissions: &self.admissions,
                mapping_entries: &self.evidence_entries,
                mapping_receipt: Some(&self.evidence_approval),
                source_receipts: &self.source_receipts,
            };
            bind_claim_authority_observer_temporal_v0_1(
                ClaimAuthorityObserverTemporalBindingInputV0_1 {
                    schema: CLAIM_AUTHORITY_OBSERVER_TEMPORAL_SCHEMA_V0_1.to_owned(),
                    requirement_profile: RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
                    evidence_binding_input: &evidence_input,
                    observer_assessment_binding: &self.observer_binding,
                    observer_coverage: &self.coverage,
                    authority_receipt: Some(&self.authority),
                    observer_receipt: Some(&self.observer),
                    temporal_receipt: Some(&self.temporal),
                },
            )
        }
    }

    #[test]
    fn binds_claim_identity_semantics_authority_observation_and_temporal_snapshot() {
        let case = build_case();
        let result = case.evaluate();
        assert_eq!(result.disposition, ClaimSourceObligationDispositionV0_1::Incomplete);
        assert_eq!(result.obligations.len(), 5);
        assert!(result.unassessed_obligation_ids.iter().any(|id| id == "model-applicability"));
        assert!(!result.complete_claim_assurance_state);
        assert!(!result.assurance_state_mutated);
        assert!(!result.physical_truth_established);
        assert!(!result.engineering_acceptance_granted);
        assert!(!result.physical_work_authorized);
    }

    #[test]
    fn late_authority_receipt_cannot_backfill_historical_role_authority() {
        let mut case = build_case();
        case.authority.receiver_first_seen_at_unix_ms = CUTOFF + 1;
        let result = case.evaluate();
        assert_eq!(result.disposition, ClaimSourceObligationDispositionV0_1::Unverified);
        assert!(result.reason_codes.contains(&"CLAIM_ACCEPTANCE_AUTHORITY_NOT_KNOWN_AT_CUTOFF".to_owned()));
    }

    #[test]
    fn authority_role_mismatch_is_rejected() {
        let mut case = build_case();
        case.authority.authority_role_ref = "role:unrelated".to_owned();
        let result = case.evaluate();
        assert_eq!(result.disposition, ClaimSourceObligationDispositionV0_1::Rejected);
    }

    #[test]
    fn out_of_scope_observer_receipt_remains_out_of_scope() {
        let mut case = build_case();
        case.coverage.status = CoverageStatus::OutOfScope;
        case.coverage.replay_consequence = ReplayConsequence::Abstain;
        case.observer.coverage_status = CoverageStatus::OutOfScope;
        case.observer.replay_consequence = ReplayConsequence::Abstain;
        let result = case.evaluate();
        assert_eq!(result.disposition, ClaimSourceObligationDispositionV0_1::OutOfScope);
    }

    #[test]
    fn observer_report_substitution_is_rejected() {
        let mut case = build_case();
        case.observer.coverage_report_sha256 = "0".repeat(64);
        let result = case.evaluate();
        assert_eq!(result.disposition, ClaimSourceObligationDispositionV0_1::Rejected);
    }

    #[test]
    fn observer_and_temporal_receipts_must_be_known_at_cutoff() {
        let mut case = build_case();
        case.temporal.receiver_first_seen_at_unix_ms = CUTOFF + 1;
        let result = case.evaluate();
        assert_eq!(result.disposition, ClaimSourceObligationDispositionV0_1::Unverified);
        assert!(result.reason_codes.contains(&"CLAIM_TEMPORAL_KNOWLEDGE_NOT_KNOWN_AT_CUTOFF".to_owned()));
    }

    #[test]
    fn late_observer_watermark_is_not_accepted() {
        let mut case = build_case();
        case.observer.observer_watermark_unix_ms = CUTOFF + 1;
        case.owner_cut.observer_coverage_watermark_unix_ms = CUTOFF + 1;
        case.temporal.receiver_watermark_unix_ms = CUTOFF + 1;
        case.owner_cut.receiver_receipt_watermark_unix_ms = CUTOFF + 1;
        let result = case.evaluate();
        assert_eq!(result.disposition, ClaimSourceObligationDispositionV0_1::Rejected);
    }

    #[test]
    fn missing_observer_receipt_never_infers_scoped_absence() {
        let mut case = build_case();
        let raw_sha = sha256(&case.raw);
        let evidence_input = ClaimObligationEvidenceBindingInputV0_1 {
            schema: super::super::claim_obligation_evidence_binding_v0_1::CLAIM_OBLIGATION_EVIDENCE_BINDING_SCHEMA_V0_1.to_owned(),
            raw_claim_specification_payload: &case.raw,
            expected_raw_claim_specification_sha256: &raw_sha,
            claim_specification: &case.spec,
            decision_context: &case.context,
            requirement_profile: RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
            owner_evidence_cut: &case.owner_cut,
            required_evidence_admissions: &case.admissions,
            mapping_entries: &case.evidence_entries,
            mapping_receipt: Some(&case.evidence_approval),
            source_receipts: &case.source_receipts,
        };
        let result = bind_claim_authority_observer_temporal_v0_1(
            ClaimAuthorityObserverTemporalBindingInputV0_1 {
                schema: CLAIM_AUTHORITY_OBSERVER_TEMPORAL_SCHEMA_V0_1.to_owned(),
                requirement_profile: RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
                evidence_binding_input: &evidence_input,
                observer_assessment_binding: &case.observer_binding,
                observer_coverage: &case.coverage,
                authority_receipt: Some(&case.authority),
                observer_receipt: None,
                temporal_receipt: Some(&case.temporal),
            },
        );
        assert_eq!(result.disposition, ClaimSourceObligationDispositionV0_1::Incomplete);
        assert!(result.reason_codes.contains(&"CLAIM_OBSERVER_COVERAGE_RECEIPT_MISSING".to_owned()));
    }
}
