// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Owner-reviewed dispositions for assumptions and exclusions in a claim profile.
//!
//! These entries are advisory defeater records, not mandatory assurance gates.
//! The evaluator binds every declared assumption/exclusion to an explicit
//! review disposition and preserves OPEN items. It never upgrades an open
//! defeater, changes the claim gate, or grants engineering/physical authority.

use super::claim_obligation_profile_v0_1::{
    derive_claim_obligation_profile_v0_1, DerivedClaimObligationProfileV0_1,
    RegisteredClaimRequirementProfileV0_1,
};
use super::claim_specification_v0_1::ClaimSpecificationV0_1;
use super::decision_context_v0_1::DecisionContextV0_1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CLAIM_DEFEATER_RESOLUTION_SCHEMA_V0_1: &str =
    "AeroClaimDefeaterResolutionV0_1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum DefeaterResolutionReceiptStatusV0_1 {
    #[serde(rename = "VERIFIED")]
    Verified,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "FAILED")]
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum DefeaterResolutionStateV0_1 {
    #[serde(rename = "OPEN")]
    Open,
    #[serde(rename = "RESOLVED_WITH_EVIDENCE")]
    ResolvedWithEvidence,
    #[serde(rename = "ACCEPTED_AS_LIMITATION")]
    AcceptedAsLimitation,
    #[serde(rename = "NOT_APPLICABLE_WITH_JUSTIFICATION")]
    NotApplicableWithJustification,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DefeaterKindV0_1 {
    #[serde(rename = "ASSUMPTION")]
    Assumption,
    #[serde(rename = "EXCLUSION")]
    Exclusion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DefeaterBindingDispositionV0_1 {
    #[serde(rename = "ALL_REVIEWED")]
    AllReviewed,
    #[serde(rename = "OPEN_ITEMS_REMAIN")]
    OpenItemsRemain,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "REJECTED")]
    Rejected,
    #[serde(rename = "INVALID_INPUT")]
    InvalidInput,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimDefeaterResolutionReceiptV0_1 {
    pub verification_status: DefeaterResolutionReceiptStatusV0_1,
    pub reviewer_namespace: String,
    pub authority_holder_namespace: String,
    pub authority_role_ref: String,
    pub claim_id: String,
    pub configuration_ref: String,
    pub decision_context_sha256: String,
    pub claim_specification_semantic_sha256: String,
    pub obligation_profile_sha256: String,
    pub obligation_manifest_sha256: String,
    pub obligation_id: String,
    pub requirement_source_ref: String,
    pub defeater_kind: DefeaterKindV0_1,
    pub statement_sha256: String,
    pub state: DefeaterResolutionStateV0_1,
    pub supporting_evidence_refs: Vec<String>,
    pub supporting_evidence_manifest_sha256: String,
    pub rationale_sha256: Option<String>,
    pub receiver_first_seen_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimDefeaterResolutionInputV0_1<'a> {
    pub schema: String,
    pub requirement_profile: RegisteredClaimRequirementProfileV0_1,
    pub claim_specification: &'a ClaimSpecificationV0_1,
    pub decision_context: &'a DecisionContextV0_1,
    pub receipts: &'a [ClaimDefeaterResolutionReceiptV0_1],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoundClaimDefeaterV0_1 {
    pub obligation_id: String,
    pub kind: DefeaterKindV0_1,
    pub statement_sha256: String,
    pub state: Option<DefeaterResolutionStateV0_1>,
    pub receipt_sha256: Option<String>,
    pub supporting_evidence_refs: Vec<String>,
    pub rationale_sha256: Option<String>,
    pub reviewed_by_namespace: Option<String>,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClaimDefeaterResolutionAssessmentV0_1 {
    pub disposition: DefeaterBindingDispositionV0_1,
    pub claim_id: String,
    pub configuration_ref: String,
    pub decision_context_sha256: String,
    pub claim_specification_semantic_sha256: String,
    pub obligation_profile_sha256: String,
    pub obligation_manifest_sha256: String,
    pub defeaters: Vec<BoundClaimDefeaterV0_1>,
    pub open_obligation_ids: Vec<String>,
    /// Defeaters are advisory/non-compensatory annotations under the registered
    /// v0.1 profile. This adapter cannot alter their severity or the ClaimGate.
    pub advisory_nonblocking: bool,
    pub complete_claim_assurance_state: bool,
    pub assurance_state_mutated: bool,
    pub engineering_acceptance_granted: bool,
    pub physical_work_authorized: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Serialize)]
struct SupportingEvidenceManifestV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    evidence_refs: Vec<&'a str>,
}

#[derive(Serialize)]
struct DefeaterResolutionManifestEntryV0_1<'a> {
    obligation_id: &'a str,
    requirement_source_ref: &'a str,
    kind: DefeaterKindV0_1,
    statement_sha256: &'a str,
    state: Option<DefeaterResolutionStateV0_1>,
    reviewer_namespace: Option<&'a str>,
    supporting_evidence_manifest_sha256: &'a str,
    rationale_sha256: Option<&'a str>,
    receipt_sha256: Option<&'a str>,
}

#[derive(Serialize)]
struct DefeaterResolutionManifestV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    claim_id: &'a str,
    decision_context_sha256: &'a str,
    claim_specification_semantic_sha256: &'a str,
    obligation_profile_sha256: &'a str,
    obligation_manifest_sha256: &'a str,
    entries: Vec<DefeaterResolutionManifestEntryV0_1<'a>>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn hash_serializable<T: Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "CLAIM_DEFEATER_CANONICAL_SERIALIZATION_FAILED".to_owned())
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

fn defeater_specs(
    spec: &ClaimSpecificationV0_1,
    profile: &DerivedClaimObligationProfileV0_1,
) -> Vec<(String, String, DefeaterKindV0_1, String)> {
    let mut result = Vec::new();
    for statement in &spec.assumptions {
        let id = format!("assumption-defeater:{statement}");
        if let Some(obligation) = profile.obligations.iter().find(|item| item.obligation_id == id) {
            result.push((
                id,
                obligation.requirement_source_ref.clone(),
                DefeaterKindV0_1::Assumption,
                sha256_hex(statement.as_bytes()),
            ));
        }
    }
    for statement in &spec.exclusions {
        let id = format!("exclusion-defeater:{statement}");
        if let Some(obligation) = profile.obligations.iter().find(|item| item.obligation_id == id) {
            result.push((
                id,
                obligation.requirement_source_ref.clone(),
                DefeaterKindV0_1::Exclusion,
                sha256_hex(statement.as_bytes()),
            ));
        }
    }
    result.sort_by(|left, right| left.0.cmp(&right.0));
    result
}

pub fn compute_defeater_supporting_evidence_manifest_sha256_v0_1(
    evidence_refs: &[String],
) -> Result<String, String> {
    let mut ordered = evidence_refs.iter().map(String::as_str).collect::<Vec<_>>();
    ordered.sort_unstable();
    if ordered.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err("CLAIM_DEFEATER_SUPPORTING_EVIDENCE_DUPLICATE".to_owned());
    }
    hash_serializable(&SupportingEvidenceManifestV0_1 {
        schema: "AeroDefeaterSupportingEvidenceManifestV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        evidence_refs: ordered,
    })
}

pub fn compute_claim_defeater_resolution_manifest_sha256_v0_1(
    claim_id: &str,
    context_sha: &str,
    semantic_sha: &str,
    profile: &DerivedClaimObligationProfileV0_1,
    entries: &[ClaimDefeaterResolutionReceiptV0_1],
) -> Result<String, String> {
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));
    let manifest_entries = ordered
        .into_iter()
        .map(|entry| DefeaterResolutionManifestEntryV0_1 {
            obligation_id: &entry.obligation_id,
            requirement_source_ref: &entry.requirement_source_ref,
            kind: entry.defeater_kind,
            statement_sha256: &entry.statement_sha256,
            state: Some(entry.state),
            reviewer_namespace: Some(&entry.reviewer_namespace),
            supporting_evidence_manifest_sha256: &entry.supporting_evidence_manifest_sha256,
            rationale_sha256: entry.rationale_sha256.as_deref(),
            receipt_sha256: Some(&entry.receipt_sha256),
        })
        .collect();
    hash_serializable(&DefeaterResolutionManifestV0_1 {
        schema: "AeroClaimDefeaterResolutionManifestV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        claim_id,
        decision_context_sha256: context_sha,
        claim_specification_semantic_sha256: semantic_sha,
        obligation_profile_sha256: &profile.profile_sha256,
        obligation_manifest_sha256: &profile.obligation_manifest_sha256,
        entries: manifest_entries,
    })
}

fn invalid_assessment(
    input: &ClaimDefeaterResolutionInputV0_1<'_>,
    reason: &str,
) -> ClaimDefeaterResolutionAssessmentV0_1 {
    ClaimDefeaterResolutionAssessmentV0_1 {
        disposition: DefeaterBindingDispositionV0_1::InvalidInput,
        claim_id: input.claim_specification.claim_id.clone(),
        configuration_ref: input.claim_specification.configuration_ref.clone(),
        decision_context_sha256: String::new(),
        claim_specification_semantic_sha256: String::new(),
        obligation_profile_sha256: String::new(),
        obligation_manifest_sha256: String::new(),
        defeaters: Vec::new(),
        open_obligation_ids: Vec::new(),
        advisory_nonblocking: true,
        complete_claim_assurance_state: false,
        assurance_state_mutated: false,
        engineering_acceptance_granted: false,
        physical_work_authorized: false,
        reason_codes: vec![reason.to_owned()],
    }
}

/// Resolve the claim's exact declared assumption/exclusion defeater universe.
/// Every derived defeater must have one owner-reviewed receipt. OPEN is a
/// preserved state and does not rewrite the registered nonblocking severity.
pub fn bind_claim_defeater_resolutions_v0_1(
    input: ClaimDefeaterResolutionInputV0_1<'_>,
) -> ClaimDefeaterResolutionAssessmentV0_1 {
    let spec = input.claim_specification;
    let context = input.decision_context;
    if input.schema != CLAIM_DEFEATER_RESOLUTION_SCHEMA_V0_1
        || input.requirement_profile
            != RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1
        || context.claim_id != spec.claim_id
        || context.subject_ref != spec.subject_ref
        || context.configuration_ref != spec.configuration_ref
        || context.operating_envelope_ref != spec.operating_envelope_ref
    {
        return invalid_assessment(&input, "CLAIM_DEFEATER_INPUT_SCOPE_OR_PROFILE_INVALID");
    }
    let semantic_sha = match spec.compute_semantic_sha256() {
        Ok(value) => value,
        Err(_) => return invalid_assessment(&input, "CLAIM_DEFEATER_CLAIM_SPECIFICATION_INVALID"),
    };
    let context_sha = match context.compute_sha256() {
        Ok(value) => value,
        Err(_) => return invalid_assessment(&input, "CLAIM_DEFEATER_DECISION_CONTEXT_INVALID"),
    };
    let profile = derive_claim_obligation_profile_v0_1(spec, input.requirement_profile);
    let Some(profile) = profile.profile else {
        return invalid_assessment(&input, "CLAIM_DEFEATER_OBLIGATION_PROFILE_NOT_DERIVED");
    };
    let expected = defeater_specs(spec, &profile);
    let mut errors = BTreeSet::new();
    let mut receipt_map = BTreeMap::new();
    for receipt in input.receipts {
        if receipt_map.insert(receipt.obligation_id.as_str(), receipt).is_some() {
            errors.insert("CLAIM_DEFEATER_DUPLICATE_RESOLUTION_RECEIPT".to_owned());
        }
    }
    let expected_ids = expected.iter().map(|item| item.0.as_str()).collect::<BTreeSet<_>>();
    let supplied_ids = receipt_map.keys().copied().collect::<BTreeSet<_>>();
    if supplied_ids.difference(&expected_ids).next().is_some() {
        errors.insert("CLAIM_DEFEATER_RECEIPT_FOR_UNDECLARED_OBLIGATION".to_owned());
    }

    let mut output = Vec::new();
    let mut open_ids = Vec::new();
    let mut has_unverified = false;
    let mut has_rejected = !errors.is_empty();

    for (obligation_id, source_ref, kind, statement_sha) in &expected {
        let Some(receipt) = receipt_map.get(obligation_id.as_str()).copied() else {
            open_ids.push(obligation_id.clone());
            output.push(BoundClaimDefeaterV0_1 {
                obligation_id: obligation_id.clone(),
                kind: *kind,
                statement_sha256: statement_sha.clone(),
                state: None,
                receipt_sha256: None,
                supporting_evidence_refs: Vec::new(),
                rationale_sha256: None,
                reviewed_by_namespace: None,
                reason_codes: vec!["CLAIM_DEFEATER_REVIEW_RECEIPT_MISSING".to_owned()],
            });
            continue;
        };

        let expected_support_sha =
            compute_defeater_supporting_evidence_manifest_sha256_v0_1(
                &receipt.supporting_evidence_refs,
            );
        let mut reasons = BTreeSet::new();
        if !valid_identifier(&receipt.reviewer_namespace)
            || !valid_identifier(&receipt.authority_holder_namespace)
            || !valid_identifier(&receipt.authority_role_ref)
            || !valid_sha256(&receipt.receipt_sha256)
            || receipt.claim_id != spec.claim_id
            || receipt.configuration_ref != spec.configuration_ref
            || receipt.decision_context_sha256 != context_sha
            || receipt.claim_specification_semantic_sha256 != semantic_sha
            || receipt.obligation_profile_sha256 != profile.profile_sha256
            || receipt.obligation_manifest_sha256 != profile.obligation_manifest_sha256
            || receipt.obligation_id != *obligation_id
            || receipt.requirement_source_ref != *source_ref
            || receipt.defeater_kind != *kind
            || receipt.statement_sha256 != *statement_sha
            || receipt.authority_role_ref != spec.acceptance_authority_role_ref
            || expected_support_sha.as_deref()
                != Ok(receipt.supporting_evidence_manifest_sha256.as_str())
        {
            reasons.insert("CLAIM_DEFEATER_RECEIPT_SCOPE_OR_COMMITMENT_MISMATCH".to_owned());
        }
        if receipt.supporting_evidence_refs.iter().any(|reference| !valid_identifier(reference)) {
            reasons.insert("CLAIM_DEFEATER_SUPPORTING_EVIDENCE_REFERENCE_INVALID".to_owned());
        }
        if let Some(rationale) = &receipt.rationale_sha256 {
            if !valid_sha256(rationale) {
                reasons.insert("CLAIM_DEFEATER_RATIONALE_DIGEST_INVALID".to_owned());
            }
        }
        match receipt.state {
            DefeaterResolutionStateV0_1::Open => {}
            DefeaterResolutionStateV0_1::ResolvedWithEvidence => {
                if receipt.supporting_evidence_refs.is_empty() {
                    reasons.insert("CLAIM_DEFEATER_RESOLVED_WITHOUT_SUPPORTING_EVIDENCE".to_owned());
                }
            }
            DefeaterResolutionStateV0_1::AcceptedAsLimitation
            | DefeaterResolutionStateV0_1::NotApplicableWithJustification => {
                if receipt.rationale_sha256.is_none() {
                    reasons.insert("CLAIM_DEFEATER_JUSTIFICATION_MISSING".to_owned());
                }
            }
        }
        if receipt.receiver_first_seen_at_unix_ms > context.as_known_at_unix_ms
            || !effective_at(
                receipt.effective_from_unix_ms,
                receipt.effective_until_unix_ms,
                context.as_known_at_unix_ms,
            )
        {
            reasons.insert("CLAIM_DEFEATER_RECEIPT_NOT_KNOWN_OR_EFFECTIVE_AT_CUTOFF".to_owned());
        }
        match receipt.verification_status {
            DefeaterResolutionReceiptStatusV0_1::Verified => {}
            DefeaterResolutionReceiptStatusV0_1::Unverified => {
                reasons.insert("CLAIM_DEFEATER_RECEIPT_UNVERIFIED".to_owned());
            }
            DefeaterResolutionReceiptStatusV0_1::Failed => {
                reasons.insert("CLAIM_DEFEATER_RECEIPT_VERIFICATION_FAILED".to_owned());
            }
        }

        let rejected = reasons.iter().any(|reason| {
            reason.contains("MISMATCH")
                || reason.contains("INVALID")
                || reason.contains("WITHOUT_SUPPORTING")
                || reason.contains("JUSTIFICATION_MISSING")
                || reason.contains("VERIFICATION_FAILED")
        });
        let unverified = reasons.iter().any(|reason| {
            reason.contains("UNVERIFIED")
                || reason.contains("NOT_KNOWN_OR_EFFECTIVE")
        });
        if rejected {
            has_rejected = true;
        }
        if unverified {
            has_unverified = true;
        }
        if receipt.state == DefeaterResolutionStateV0_1::Open || rejected || unverified {
            open_ids.push(obligation_id.clone());
        }
        output.push(BoundClaimDefeaterV0_1 {
            obligation_id: obligation_id.clone(),
            kind: *kind,
            statement_sha256: statement_sha.clone(),
            state: Some(receipt.state),
            receipt_sha256: Some(receipt.receipt_sha256.clone()),
            supporting_evidence_refs: receipt.supporting_evidence_refs.clone(),
            rationale_sha256: receipt.rationale_sha256.clone(),
            reviewed_by_namespace: Some(receipt.reviewer_namespace.clone()),
            reason_codes: reasons.into_iter().collect(),
        });
    }
    output.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));
    open_ids.sort();
    let disposition = if has_rejected {
        DefeaterBindingDispositionV0_1::Rejected
    } else if has_unverified {
        DefeaterBindingDispositionV0_1::Unverified
    } else if !open_ids.is_empty() {
        DefeaterBindingDispositionV0_1::OpenItemsRemain
    } else {
        DefeaterBindingDispositionV0_1::AllReviewed
    };
    let reason_codes = if open_ids.is_empty() {
        Vec::new()
    } else {
        vec!["CLAIM_DEFEATER_ITEMS_REMAIN_OPEN_OR_UNVERIFIED".to_owned()]
    };
    ClaimDefeaterResolutionAssessmentV0_1 {
        disposition,
        claim_id: spec.claim_id.clone(),
        configuration_ref: spec.configuration_ref.clone(),
        decision_context_sha256: context_sha,
        claim_specification_semantic_sha256: semantic_sha,
        obligation_profile_sha256: profile.profile_sha256,
        obligation_manifest_sha256: profile.obligation_manifest_sha256,
        defeaters: output,
        open_obligation_ids: open_ids,
        advisory_nonblocking: true,
        complete_claim_assurance_state: false,
        assurance_state_mutated: false,
        engineering_acceptance_granted: false,
        physical_work_authorized: false,
        reason_codes,
    }
}
