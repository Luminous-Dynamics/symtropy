// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-bound evidence-class obligations for AERO v0.1.
//!
//! This is a deliberately narrow adapter increment. It binds every mandatory
//! evidence-class obligation derived from a typed claim specification to
//! owner-native evidence records, per-record source receipts, an exact
//! receiver-first-seen time and an owner-approved mapping manifest.
//!
//! It does NOT populate model-credibility, model-applicability, uncertainty,
//! observer, defeater, temporal-policy or independent-replay obligations.
//! Those remain explicitly unassessed, and this module can never declare the
//! complete claim assurance state or grant engineering/physical authority.

use super::claim_specification_binding_v0_1::{
    bind_claim_specification_v0_1, ClaimSpecificationBindingDispositionV0_1,
    ClaimSpecificationBindingInputV0_1,
};
use super::claim_obligation_profile_v0_1::{
    derive_claim_obligation_profile_v0_1, DerivedClaimObligationProfileV0_1,
    RegisteredClaimRequirementProfileV0_1,
};
use super::claim_specification_v0_1::ClaimSpecificationV0_1;
use super::decision_context_v0_1::{
    compute_required_evidence_manifest_sha256, DecisionContextV0_1,
};
use super::owner_evidence_cut_v0_1::{
    OwnerDependencyCoverage, OwnerEvidenceCutV0_1, OwnerEvidenceFreshness,
    OwnerEvidenceLifecycle,
};
use super::claim_assurance_gate_v0_1::{
    EvidenceAdmissionState, RequiredEvidenceAdmission,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CLAIM_OBLIGATION_EVIDENCE_BINDING_SCHEMA_V0_1: &str =
    "AeroClaimObligationEvidenceBindingV0_1";
pub const CLAIM_OBLIGATION_EVIDENCE_MAPPING_SCHEMA_V0_1: &str =
    "AeroClaimObligationEvidenceMappingV0_1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EvidenceBindingReceiptStatusV0_1 {
    #[serde(rename = "VERIFIED")]
    Verified,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "FAILED")]
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EvidenceBindingIntegrityStateV0_1 {
    #[serde(rename = "VERIFIED")]
    Verified,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "FAILED")]
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EvidenceProducerAuthorityStateV0_1 {
    #[serde(rename = "AUTHORIZED")]
    Authorized,
    #[serde(rename = "DENIED")]
    Denied,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EvidenceTemporalStateV0_1 {
    #[serde(rename = "KNOWN_AT_CUTOFF")]
    KnownAtCutoff,
    #[serde(rename = "NOT_KNOWN_AT_CUTOFF")]
    NotKnownAtCutoff,
    #[serde(rename = "CONFLICT")]
    Conflict,
    #[serde(rename = "INVALID")]
    Invalid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EvidenceEffectivityStateV0_1 {
    #[serde(rename = "COVERS_ACQUISITION_INTERVAL")]
    CoversAcquisitionInterval,
    #[serde(rename = "DOES_NOT_COVER_ACQUISITION_INTERVAL")]
    DoesNotCoverAcquisitionInterval,
    #[serde(rename = "INVALID_INTERVAL")]
    InvalidInterval,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClaimObligationEvidenceBindingDispositionV0_1 {
    #[serde(rename = "BOUND")]
    Bound,
    #[serde(rename = "INCOMPLETE")]
    Incomplete,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "REJECTED")]
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimObligationEvidenceMappingEntryV0_1 {
    /// Exact derived obligation ID, e.g. evidence-integrity:<class>.
    pub obligation_id: String,
    /// Exact requirement source reference from the derived profile.
    pub requirement_source_ref: String,
    /// Exact class ID from ClaimSpecificationV0_1.required_evidence_classes.
    pub evidence_class: String,
    /// One or more unique owner-native evidence references for this class.
    pub evidence_refs: Vec<String>,
}

/// Owner approval of the complete mapping from claim-spec evidence classes
/// to owner-native records. VERIFIED is an upstream disposition; this module
/// checks its scope and chronology but does not authenticate signatures.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimObligationEvidenceMappingReceiptV0_1 {
    pub verification_status: EvidenceBindingReceiptStatusV0_1,
    pub owner_namespace: String,
    pub claim_id: String,
    pub decision_context_sha256: String,
    pub configuration_ref: String,
    pub claim_specification_semantic_sha256: String,
    pub obligation_profile_sha256: String,
    pub obligation_manifest_sha256: String,
    pub required_evidence_manifest_sha256: String,
    pub owner_evidence_cut_sha256: String,
    pub trust_policy_sha256: String,
    pub mapping_manifest_sha256: String,
    pub receipt_sha256: String,
    pub receiver_first_seen_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
}

/// Per-record receipt. It supplies the chronology and authority information
/// missing from the existing owner-cut-wide receiver watermark.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerEvidenceSourceReceiptV0_1 {
    pub verification_status: EvidenceBindingReceiptStatusV0_1,
    /// Owner of the source record/receipt.
    pub owner_namespace: String,
    /// Producer identity whose authority is evaluated for this claim.
    pub producer_namespace: String,
    pub claim_id: String,
    pub decision_context_sha256: String,
    pub subject_ref: String,
    pub configuration_ref: String,
    pub evidence_ref: String,
    pub evidence_record_sha256: String,
    pub trust_policy_sha256: String,
    pub integrity_state: EvidenceBindingIntegrityStateV0_1,
    pub producer_authority_state: EvidenceProducerAuthorityStateV0_1,
    /// Receiver first possessed this exact receipt.
    pub receiver_first_seen_at_unix_ms: u64,
    /// The evidence itself must cover the entire physical acquisition interval.
    pub evidence_effective_from_unix_ms: u64,
    pub evidence_effective_until_unix_ms: u64,
    /// Producer authority must be effective at the as-known decision cutoff.
    pub authority_effective_from_unix_ms: u64,
    pub authority_effective_until_unix_ms: u64,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimObligationEvidenceBindingInputV0_1<'a> {
    pub schema: String,
    pub raw_claim_specification_payload: &'a [u8],
    pub expected_raw_claim_specification_sha256: &'a str,
    pub claim_specification: &'a ClaimSpecificationV0_1,
    pub decision_context: &'a DecisionContextV0_1,
    pub requirement_profile: RegisteredClaimRequirementProfileV0_1,
    pub owner_evidence_cut: &'a OwnerEvidenceCutV0_1,
    pub required_evidence_admissions: &'a [RequiredEvidenceAdmission],
    pub mapping_entries: &'a [ClaimObligationEvidenceMappingEntryV0_1],
    pub mapping_receipt: Option<&'a ClaimObligationEvidenceMappingReceiptV0_1>,
    pub source_receipts: &'a [OwnerEvidenceSourceReceiptV0_1],
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoundEvidenceRecordV0_1 {
    pub evidence_ref: String,
    pub evidence_record_sha256: String,
    pub owner_namespace: String,
    pub producer_namespace: String,
    pub source_receipt_sha256: String,
    pub admission_state: String,
    pub integrity_state: EvidenceBindingIntegrityStateV0_1,
    pub producer_authority_state: EvidenceProducerAuthorityStateV0_1,
    pub temporal_state: EvidenceTemporalStateV0_1,
    pub effectivity_state: EvidenceEffectivityStateV0_1,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoundEvidenceClassObligationV0_1 {
    pub obligation_id: String,
    pub requirement_source_ref: String,
    pub evidence_class: String,
    pub evidence_records: Vec<BoundEvidenceRecordV0_1>,
    pub disposition: ClaimObligationEvidenceBindingDispositionV0_1,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClaimObligationEvidenceBindingAssessmentV0_1 {
    pub disposition: ClaimObligationEvidenceBindingDispositionV0_1,
    pub claim_id: String,
    pub configuration_ref: String,
    pub as_known_at_unix_ms: u64,
    pub raw_claim_specification_sha256: String,
    pub claim_specification_semantic_sha256: String,
    pub decision_context_sha256: String,
    pub obligation_profile_sha256: String,
    pub obligation_manifest_sha256: String,
    pub required_evidence_manifest_sha256: String,
    pub owner_evidence_cut_sha256: String,
    pub mapping_manifest_sha256: String,
    pub mapping_receipt_sha256: String,
    pub evidence_class_obligations: Vec<BoundEvidenceClassObligationV0_1>,
    /// These obligations are intentionally not evaluated by this first adapter.
    pub unassessed_obligation_ids: Vec<String>,
    /// This module never claims a complete assurance state.
    pub complete_claim_assurance_state: bool,
    pub assurance_state_mutated: bool,
    pub physical_truth_established: bool,
    pub engineering_acceptance_granted: bool,
    pub physical_work_authorized: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Serialize)]
struct EvidenceMappingManifestEnvelopeV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    claim_id: &'a str,
    decision_context_sha256: &'a str,
    claim_specification_semantic_sha256: &'a str,
    obligation_profile_sha256: &'a str,
    obligation_manifest_sha256: &'a str,
    required_evidence_manifest_sha256: &'a str,
    owner_evidence_cut_sha256: &'a str,
    trust_policy_sha256: &'a str,
    entries: Vec<EvidenceMappingEntryEnvelopeV0_1<'a>>,
}

#[derive(Serialize)]
struct EvidenceMappingEntryEnvelopeV0_1<'a> {
    obligation_id: &'a str,
    requirement_source_ref: &'a str,
    evidence_class: &'a str,
    evidence_refs: Vec<&'a str>,
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
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_half_open_interval(from: u64, until: u64) -> bool {
    from < until
}

fn interval_contains(outer_from: u64, outer_until: u64, inner_from: u64, inner_until: u64) -> bool {
    valid_half_open_interval(outer_from, outer_until)
        && inner_from <= inner_until
        && outer_from <= inner_from
        && inner_until < outer_until
}

fn interval_contains_instant(from: u64, until: u64, instant: u64) -> bool {
    valid_half_open_interval(from, until) && from <= instant && instant < until
}

fn assessment_shell(
    input: &ClaimObligationEvidenceBindingInputV0_1<'_>,
    disposition: ClaimObligationEvidenceBindingDispositionV0_1,
    raw_sha: String,
    semantic_sha: String,
    context_sha: String,
    profile: Option<&DerivedClaimObligationProfileV0_1>,
    owner_cut_sha: String,
    mapping_sha: String,
    mapping_receipt_sha: String,
    evidence_class_obligations: Vec<BoundEvidenceClassObligationV0_1>,
    unassessed_obligation_ids: Vec<String>,
    reasons: BTreeSet<String>,
) -> ClaimObligationEvidenceBindingAssessmentV0_1 {
    ClaimObligationEvidenceBindingAssessmentV0_1 {
        disposition,
        claim_id: input.claim_specification.claim_id.clone(),
        configuration_ref: input.claim_specification.configuration_ref.clone(),
        as_known_at_unix_ms: input.decision_context.as_known_at_unix_ms,
        raw_claim_specification_sha256: raw_sha,
        claim_specification_semantic_sha256: semantic_sha,
        decision_context_sha256: context_sha,
        obligation_profile_sha256: profile.map(|item| item.profile_sha256.clone()).unwrap_or_default(),
        obligation_manifest_sha256: profile.map(|item| item.obligation_manifest_sha256.clone()).unwrap_or_default(),
        required_evidence_manifest_sha256: input.decision_context.required_evidence_manifest_sha256.clone(),
        owner_evidence_cut_sha256: owner_cut_sha,
        mapping_manifest_sha256: mapping_sha,
        mapping_receipt_sha256: mapping_receipt_sha,
        evidence_class_obligations,
        unassessed_obligation_ids,
        complete_claim_assurance_state: false,
        assurance_state_mutated: false,
        physical_truth_established: false,
        engineering_acceptance_granted: false,
        physical_work_authorized: false,
        reason_codes: reasons.into_iter().collect(),
    }
}

/// Compute the owner-approved mapping manifest. Order of entries and references
/// is normalized; mapping semantics and every reference remain in the digest.
pub fn compute_evidence_mapping_manifest_sha256_v0_1(
    claim_id: &str,
    decision_context_sha256: &str,
    claim_specification_semantic_sha256: &str,
    profile: &DerivedClaimObligationProfileV0_1,
    required_evidence_manifest_sha256: &str,
    owner_evidence_cut_sha256: &str,
    trust_policy_sha256: &str,
    entries: &[ClaimObligationEvidenceMappingEntryV0_1],
) -> Result<String, String> {
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));
    let envelope_entries = ordered
        .into_iter()
        .map(|entry| {
            let mut refs = entry.evidence_refs.iter().map(String::as_str).collect::<Vec<_>>();
            refs.sort_unstable();
            EvidenceMappingEntryEnvelopeV0_1 {
                obligation_id: &entry.obligation_id,
                requirement_source_ref: &entry.requirement_source_ref,
                evidence_class: &entry.evidence_class,
                evidence_refs: refs,
            }
        })
        .collect();
    let envelope = EvidenceMappingManifestEnvelopeV0_1 {
        schema: "AeroClaimObligationEvidenceMappingManifestV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        claim_id,
        decision_context_sha256,
        claim_specification_semantic_sha256,
        obligation_profile_sha256: &profile.profile_sha256,
        obligation_manifest_sha256: &profile.obligation_manifest_sha256,
        required_evidence_manifest_sha256,
        owner_evidence_cut_sha256,
        trust_policy_sha256,
        entries: envelope_entries,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "CLAIM_OBLIGATION_EVIDENCE_MAPPING_SERIALIZATION_FAILED".to_owned())
}

fn invalid_shell(
    input: &ClaimObligationEvidenceBindingInputV0_1<'_>,
    reasons: BTreeSet<String>,
) -> ClaimObligationEvidenceBindingAssessmentV0_1 {
    assessment_shell(
        input,
        ClaimObligationEvidenceBindingDispositionV0_1::Rejected,
        sha256_hex(input.raw_claim_specification_payload),
        String::new(),
        String::new(),
        None,
        String::new(),
        String::new(),
        String::new(),
        Vec::new(),
        Vec::new(),
        reasons,
    )
}

fn evidence_class_obligations<'a>(
    profile: &'a DerivedClaimObligationProfileV0_1,
) -> Vec<&'a super::claim_obligation_profile_v0_1::DerivedClaimObligationV0_1> {
    profile
        .obligations
        .iter()
        .filter(|obligation| {
            obligation.kind == super::assurance_disposition_v0_1::ObligationKindV0_1::EvidenceIntegrity
        })
        .collect()
}

fn validate_mapping_entries(
    profile: &DerivedClaimObligationProfileV0_1,
    entries: &[ClaimObligationEvidenceMappingEntryV0_1],
) -> BTreeSet<String> {
    let mut errors = BTreeSet::new();
    let expected = evidence_class_obligations(profile);
    let expected_by_id = expected
        .iter()
        .map(|obligation| (obligation.obligation_id.as_str(), *obligation))
        .collect::<BTreeMap<_, _>>();
    let mut seen_obligations = BTreeSet::new();
    let mut seen_evidence_refs = BTreeSet::new();
    for entry in entries {
        if !valid_identifier(&entry.obligation_id)
            || !valid_identifier(&entry.evidence_class)
            || !seen_obligations.insert(entry.obligation_id.as_str())
        {
            errors.insert("EVIDENCE_MAPPING_OBLIGATION_ID_INVALID_OR_DUPLICATE".to_owned());
            continue;
        }
        let Some(obligation) = expected_by_id.get(entry.obligation_id.as_str()) else {
            errors.insert(format!(
                "EVIDENCE_MAPPING_OBLIGATION_NOT_DERIVED:{}",
                entry.obligation_id
            ));
            continue;
        };
        let expected_class = entry
            .obligation_id
            .strip_prefix("evidence-integrity:")
            .unwrap_or_default();
        let expected_source = format!("claim-spec:/required_evidence_classes/{expected_class}");
        if expected_class.is_empty()
            || entry.evidence_class != expected_class
            || entry.requirement_source_ref != obligation.requirement_source_ref
            || entry.requirement_source_ref != expected_source
            || !obligation.mandatory
            || !obligation.hard_gate
        {
            errors.insert(format!(
                "EVIDENCE_MAPPING_REQUIREMENT_SOURCE_OR_CLASS_MISMATCH:{}",
                entry.obligation_id
            ));
        }
        if entry.evidence_refs.is_empty() {
            errors.insert(format!(
                "EVIDENCE_MAPPING_HAS_NO_EVIDENCE_REFS:{}",
                entry.obligation_id
            ));
        }
        let mut local = BTreeSet::new();
        for evidence_ref in &entry.evidence_refs {
            if !valid_identifier(evidence_ref)
                || !local.insert(evidence_ref.as_str())
                || !seen_evidence_refs.insert(evidence_ref.as_str())
            {
                errors.insert(format!(
                    "EVIDENCE_MAPPING_REFERENCE_INVALID_DUPLICATE_OR_REUSED:{}",
                    entry.obligation_id
                ));
            }
        }
    }
    if seen_obligations.len() != expected.len()
        || expected_by_id
            .keys()
            .any(|obligation_id| !seen_obligations.contains(obligation_id))
    {
        errors.insert("EVIDENCE_MAPPING_DERIVED_OBLIGATION_SET_INCOMPLETE".to_owned());
    }
    errors
}

fn validate_mapping_receipt(
    receipt: Option<&ClaimObligationEvidenceMappingReceiptV0_1>,
    owner_cut: &OwnerEvidenceCutV0_1,
    claim_id: &str,
    configuration_ref: &str,
    context_sha: &str,
    semantic_sha: &str,
    profile: &DerivedClaimObligationProfileV0_1,
    required_manifest_sha: &str,
    owner_cut_sha: &str,
    mapping_sha: &str,
    cutoff: u64,
) -> BTreeSet<String> {
    let mut errors = BTreeSet::new();
    let Some(receipt) = receipt else {
        errors.insert("EVIDENCE_MAPPING_APPROVAL_RECEIPT_MISSING".to_owned());
        return errors;
    };
    match receipt.verification_status {
        EvidenceBindingReceiptStatusV0_1::Verified => {}
        EvidenceBindingReceiptStatusV0_1::Unverified => {
            errors.insert("EVIDENCE_MAPPING_APPROVAL_NOT_UPSTREAM_VERIFIED".to_owned());
        }
        EvidenceBindingReceiptStatusV0_1::Failed => {
            errors.insert("EVIDENCE_MAPPING_APPROVAL_VERIFICATION_FAILED".to_owned());
        }
    }
    if !valid_identifier(&receipt.owner_namespace)
        || !valid_sha256(&receipt.receipt_sha256)
        || receipt.owner_namespace != owner_cut.owner_namespace
        || receipt.claim_id != claim_id
        || receipt.configuration_ref != configuration_ref
        || receipt.decision_context_sha256 != context_sha
        || receipt.claim_specification_semantic_sha256 != semantic_sha
        || receipt.obligation_profile_sha256 != profile.profile_sha256
        || receipt.obligation_manifest_sha256 != profile.obligation_manifest_sha256
        || receipt.required_evidence_manifest_sha256 != required_manifest_sha
        || receipt.owner_evidence_cut_sha256 != owner_cut_sha
        || receipt.trust_policy_sha256 != owner_cut.trust_policy_sha256
        || receipt.mapping_manifest_sha256 != mapping_sha
    {
        errors.insert("EVIDENCE_MAPPING_APPROVAL_SCOPE_OR_COMMITMENT_MISMATCH".to_owned());
    }
    if receipt.receiver_first_seen_at_unix_ms > cutoff {
        errors.insert("EVIDENCE_MAPPING_APPROVAL_NOT_KNOWN_AT_CUTOFF".to_owned());
    }
    if !interval_contains_instant(
        receipt.effective_from_unix_ms,
        receipt.effective_until_unix_ms,
        cutoff,
    ) {
        errors.insert("EVIDENCE_MAPPING_APPROVAL_NOT_EFFECTIVE_AT_CUTOFF".to_owned());
    }
    errors
}

fn evaluate_source_record(
    evidence_ref: &str,
    admission: Option<&RequiredEvidenceAdmission>,
    receipt: Option<&OwnerEvidenceSourceReceiptV0_1>,
    owner_cut: &OwnerEvidenceCutV0_1,
    decision_context: &DecisionContextV0_1,
) -> BoundEvidenceRecordV0_1 {
    let mut reasons = BTreeSet::new();
    let record = owner_cut.find_evidence(evidence_ref);
    let mut integrity = EvidenceBindingIntegrityStateV0_1::Unverified;
    let mut authority = EvidenceProducerAuthorityStateV0_1::Unknown;
    let mut temporal = EvidenceTemporalStateV0_1::Invalid;
    let mut effectivity = EvidenceEffectivityStateV0_1::InvalidInterval;
    let mut admission_state = "MISSING".to_owned();
    let mut source_owner = String::new();
    let mut producer = String::new();
    let mut record_sha = String::new();
    let mut receipt_sha = String::new();

    match admission {
        None => {
            reasons.insert("REQUIRED_EVIDENCE_ADMISSION_MISSING".to_owned());
        }
        Some(admission) => {
            admission_state = admission.state.as_str().to_owned();
            if admission.evidence_ref != evidence_ref
                || admission.decision_context_sha256 != decision_context.compute_sha256().unwrap_or_default()
            {
                reasons.insert("REQUIRED_EVIDENCE_ADMISSION_SCOPE_MISMATCH".to_owned());
            }
            match admission.state {
                EvidenceAdmissionState::Admitted => {}
                EvidenceAdmissionState::Unverified => {
                    reasons.insert("REQUIRED_EVIDENCE_ADMISSION_UNVERIFIED".to_owned());
                }
                EvidenceAdmissionState::Rejected => {
                    reasons.insert("REQUIRED_EVIDENCE_ADMISSION_REJECTED".to_owned());
                }
                EvidenceAdmissionState::NotReceived => {
                    reasons.insert("REQUIRED_EVIDENCE_ADMISSION_NOT_RECEIVED".to_owned());
                }
            }
        }
    }

    match record {
        None => {
            reasons.insert("OWNER_EVIDENCE_RECORD_MISSING_FROM_CUT".to_owned());
        }
        Some(record) => {
            record_sha = record.record_sha256.clone();
            if record.subject_ref != decision_context.subject_ref
                || record.configuration_ref != decision_context.configuration_ref
            {
                reasons.insert("OWNER_EVIDENCE_RECORD_SUBJECT_OR_CONFIGURATION_MISMATCH".to_owned());
            }
            if record.lifecycle != OwnerEvidenceLifecycle::Active {
                reasons.insert("OWNER_EVIDENCE_RECORD_NOT_ACTIVE".to_owned());
            }
            if record.freshness != OwnerEvidenceFreshness::Fresh {
                reasons.insert("OWNER_EVIDENCE_RECORD_STALE".to_owned());
            }
            if record.dependency_coverage != OwnerDependencyCoverage::Complete
                || !record.unresolved_inputs.is_empty()
                || record.requires_reverification
                || !record.eligible_under_declared_coverage
            {
                reasons.insert("OWNER_EVIDENCE_RECORD_DEPENDENCY_OR_REVERIFICATION_BLOCKER".to_owned());
            }
        }
    }

    match receipt {
        None => {
            reasons.insert("OWNER_EVIDENCE_SOURCE_RECEIPT_MISSING".to_owned());
        }
        Some(receipt) => {
            source_owner = receipt.owner_namespace.clone();
            producer = receipt.producer_namespace.clone();
            receipt_sha = receipt.receipt_sha256.clone();
            integrity = receipt.integrity_state;
            authority = receipt.producer_authority_state;
            if !valid_identifier(&receipt.owner_namespace)
                || !valid_identifier(&receipt.producer_namespace)
                || !valid_sha256(&receipt.receipt_sha256)
                || !valid_sha256(&receipt.evidence_record_sha256)
            {
                reasons.insert("OWNER_EVIDENCE_SOURCE_RECEIPT_INVALID".to_owned());
            }
            match receipt.verification_status {
                EvidenceBindingReceiptStatusV0_1::Verified => {}
                EvidenceBindingReceiptStatusV0_1::Unverified => {
                    reasons.insert("OWNER_EVIDENCE_SOURCE_RECEIPT_NOT_UPSTREAM_VERIFIED".to_owned());
                }
                EvidenceBindingReceiptStatusV0_1::Failed => {
                    reasons.insert("OWNER_EVIDENCE_SOURCE_RECEIPT_VERIFICATION_FAILED".to_owned());
                }
            }
            if receipt.owner_namespace != owner_cut.owner_namespace
                || receipt.claim_id != decision_context.claim_id
                || receipt.decision_context_sha256 != decision_context.compute_sha256().unwrap_or_default()
                || receipt.subject_ref != decision_context.subject_ref
                || receipt.configuration_ref != decision_context.configuration_ref
                || receipt.evidence_ref != evidence_ref
                || receipt.trust_policy_sha256 != owner_cut.trust_policy_sha256
            {
                reasons.insert("OWNER_EVIDENCE_SOURCE_RECEIPT_SCOPE_MISMATCH".to_owned());
            }
            if let Some(record) = record {
                if receipt.evidence_record_sha256 != record.record_sha256 {
                    reasons.insert("OWNER_EVIDENCE_SOURCE_RECEIPT_RECORD_DIGEST_MISMATCH".to_owned());
                }
            }
            temporal = if receipt.receiver_first_seen_at_unix_ms <= decision_context.as_known_at_unix_ms {
                EvidenceTemporalStateV0_1::KnownAtCutoff
            } else {
                EvidenceTemporalStateV0_1::NotKnownAtCutoff
            };
            if temporal != EvidenceTemporalStateV0_1::KnownAtCutoff {
                reasons.insert("OWNER_EVIDENCE_NOT_KNOWN_AT_CUTOFF".to_owned());
            }
            if receipt.authority_effective_from_unix_ms >= receipt.authority_effective_until_unix_ms {
                reasons.insert("OWNER_EVIDENCE_AUTHORITY_INTERVAL_INVALID".to_owned());
            } else if !interval_contains_instant(
                receipt.authority_effective_from_unix_ms,
                receipt.authority_effective_until_unix_ms,
                decision_context.as_known_at_unix_ms,
            ) {
                authority = EvidenceProducerAuthorityStateV0_1::Denied;
                reasons.insert("OWNER_EVIDENCE_PRODUCER_AUTHORITY_NOT_EFFECTIVE_AT_CUTOFF".to_owned());
            }
            effectivity = if interval_contains(
                receipt.evidence_effective_from_unix_ms,
                receipt.evidence_effective_until_unix_ms,
                decision_context.acquisition_time_earliest_unix_ms,
                decision_context.acquisition_time_latest_unix_ms,
            ) {
                EvidenceEffectivityStateV0_1::CoversAcquisitionInterval
            } else if valid_half_open_interval(
                receipt.evidence_effective_from_unix_ms,
                receipt.evidence_effective_until_unix_ms,
            ) {
                EvidenceEffectivityStateV0_1::DoesNotCoverAcquisitionInterval
            } else {
                EvidenceEffectivityStateV0_1::InvalidInterval
            };
            match effectivity {
                EvidenceEffectivityStateV0_1::CoversAcquisitionInterval => {}
                EvidenceEffectivityStateV0_1::DoesNotCoverAcquisitionInterval => {
                    reasons.insert("OWNER_EVIDENCE_DOES_NOT_COVER_ACQUISITION_INTERVAL".to_owned());
                }
                EvidenceEffectivityStateV0_1::InvalidInterval => {
                    reasons.insert("OWNER_EVIDENCE_EFFECTIVITY_INTERVAL_INVALID".to_owned());
                }
            }
        }
    }
    if integrity == EvidenceBindingIntegrityStateV0_1::Failed {
        reasons.insert("OWNER_EVIDENCE_INTEGRITY_FAILED".to_owned());
    } else if integrity == EvidenceBindingIntegrityStateV0_1::Unverified {
        reasons.insert("OWNER_EVIDENCE_INTEGRITY_UNVERIFIED".to_owned());
    }
    match authority {
        EvidenceProducerAuthorityStateV0_1::Authorized => {}
        EvidenceProducerAuthorityStateV0_1::Denied => {
            reasons.insert("OWNER_EVIDENCE_PRODUCER_AUTHORITY_DENIED".to_owned());
        }
        EvidenceProducerAuthorityStateV0_1::Unknown => {
            reasons.insert("OWNER_EVIDENCE_PRODUCER_AUTHORITY_UNKNOWN".to_owned());
        }
    }

    BoundEvidenceRecordV0_1 {
        evidence_ref: evidence_ref.to_owned(),
        evidence_record_sha256: record_sha,
        owner_namespace: source_owner,
        producer_namespace: producer,
        source_receipt_sha256: receipt_sha,
        admission_state,
        integrity_state: integrity,
        producer_authority_state: authority,
        temporal_state: temporal,
        effectivity_state: effectivity,
        reason_codes: reasons.into_iter().collect(),
    }
}

fn disposition_for_reasons(
    records: &[BoundEvidenceRecordV0_1],
    mapping_errors: &BTreeSet<String>,
) -> ClaimObligationEvidenceBindingDispositionV0_1 {
    if !mapping_errors.is_empty() {
        if mapping_errors.iter().any(|reason| {
            reason.contains("REJECTED")
                || reason.contains("MISMATCH")
                || reason.contains("INVALID")
                || reason.contains("DUPLICATE")
                || reason.contains("NOT_DERIVED")
        }) {
            return ClaimObligationEvidenceBindingDispositionV0_1::Rejected;
        }
        if mapping_errors.iter().any(|reason| reason.contains("NOT_UPSTREAM_VERIFIED")) {
            return ClaimObligationEvidenceBindingDispositionV0_1::Unverified;
        }
        return ClaimObligationEvidenceBindingDispositionV0_1::Incomplete;
    }
    if records.iter().any(|record| {
        record.reason_codes.iter().any(|reason| {
            reason.contains("REJECTED")
                || reason.contains("FAILED")
                || reason.contains("MISMATCH")
                || reason.contains("NOT_ACTIVE")
                || reason.contains("STALE")
                || reason.contains("AUTHORITY_DENIED")
                || reason.contains("INVALID")
                || reason.contains("DUPLICATE")
        })
    }) {
        return ClaimObligationEvidenceBindingDispositionV0_1::Rejected;
    }
    if records.iter().any(|record| {
        record.reason_codes.iter().any(|reason| {
            reason.contains("UNVERIFIED")
                || reason.contains("NOT_KNOWN")
                || reason.contains("AUTHORITY_UNKNOWN")
        })
    }) {
        return ClaimObligationEvidenceBindingDispositionV0_1::Unverified;
    }
    if records.iter().any(|record| !record.reason_codes.is_empty()) {
        return ClaimObligationEvidenceBindingDispositionV0_1::Incomplete;
    }
    ClaimObligationEvidenceBindingDispositionV0_1::Bound
}

/// Bind source-owned evidence records to every derived evidence-class
/// obligation. Other obligations remain explicitly unassessed.
pub fn bind_claim_obligation_evidence_v0_1(
    input: ClaimObligationEvidenceBindingInputV0_1<'_>,
) -> ClaimObligationEvidenceBindingAssessmentV0_1 {
    if input.schema != CLAIM_OBLIGATION_EVIDENCE_BINDING_SCHEMA_V0_1 {
        return invalid_shell(
            &input,
            BTreeSet::from(["EVIDENCE_BINDING_SCHEMA_INVALID".to_owned()]),
        );
    }
    let raw_sha = sha256_hex(input.raw_claim_specification_payload);
    let binding = bind_claim_specification_v0_1(ClaimSpecificationBindingInputV0_1 {
        raw_payload: input.raw_claim_specification_payload,
        expected_raw_payload_sha256: input.expected_raw_claim_specification_sha256,
        specification: input.claim_specification,
        decision_context: input.decision_context,
    });
    if binding.disposition != ClaimSpecificationBindingDispositionV0_1::Bound {
        return invalid_shell(
            &input,
            binding.reason_codes.into_iter().collect(),
        );
    }
    let semantic_sha = binding.semantic_sha256.clone().unwrap_or_default();
    let context_sha = binding.decision_context_sha256.clone().unwrap_or_default();

    let profile_assessment = derive_claim_obligation_profile_v0_1(
        input.claim_specification,
        input.requirement_profile,
    );
    let Some(profile) = profile_assessment.profile else {
        return assessment_shell(
            &input,
            ClaimObligationEvidenceBindingDispositionV0_1::Incomplete,
            raw_sha,
            semantic_sha,
            context_sha,
            None,
            String::new(),
            String::new(),
            String::new(),
            Vec::new(),
            Vec::new(),
            profile_assessment.reason_codes.into_iter().collect(),
        );
    };

    let mut reasons = BTreeSet::new();
    for error in input.decision_context.validation_errors() {
        reasons.insert(error);
    }
    if input.decision_context.compute_sha256().as_deref() != Ok(context_sha.as_str()) {
        reasons.insert("EVIDENCE_BINDING_DECISION_CONTEXT_COMMITMENT_MISMATCH".to_owned());
    }
    if input.decision_context.claim_id != input.claim_specification.claim_id
        || input.decision_context.subject_ref != input.claim_specification.subject_ref
        || input.decision_context.configuration_ref != input.claim_specification.configuration_ref
        || input.decision_context.operating_envelope_ref
            != input.claim_specification.operating_envelope_ref
    {
        reasons.insert("EVIDENCE_BINDING_CLAIM_CONTEXT_SCOPE_MISMATCH".to_owned());
    }

    let required_refs = input
        .required_evidence_admissions
        .iter()
        .map(|entry| entry.evidence_ref.clone())
        .collect::<Vec<_>>();
    let required_manifest = compute_required_evidence_manifest_sha256(&required_refs)
        .unwrap_or_default();
    if required_manifest != input.decision_context.required_evidence_manifest_sha256 {
        reasons.insert("EVIDENCE_BINDING_REQUIRED_EVIDENCE_MANIFEST_MISMATCH".to_owned());
    }
    let admission_ids = required_refs.iter().collect::<BTreeSet<_>>();
    if admission_ids.len() != required_refs.len() {
        reasons.insert("EVIDENCE_BINDING_REQUIRED_EVIDENCE_DUPLICATE".to_owned());
    }

    let owner_cut_sha = match input.owner_evidence_cut.compute_sha256() {
        Ok(value) => value,
        Err(errors) => {
            reasons.extend(errors);
            String::new()
        }
    };
    if owner_cut_sha != input.decision_context.evidence_knowledge_cut_sha256 {
        reasons.insert("EVIDENCE_BINDING_OWNER_CUT_COMMITMENT_MISMATCH".to_owned());
    }
    if input.owner_evidence_cut.trust_policy_sha256 != input.decision_context.trust_policy_sha256 {
        reasons.insert("EVIDENCE_BINDING_TRUST_POLICY_MISMATCH".to_owned());
    }
    if input.owner_evidence_cut.receiver_receipt_watermark_unix_ms
        > input.decision_context.as_known_at_unix_ms
    {
        reasons.insert("EVIDENCE_BINDING_OWNER_WATERMARK_AFTER_AS_KNOWN".to_owned());
    }
    if input.owner_evidence_cut.physical_unit_ref != input.decision_context.subject_ref
        || input.owner_evidence_cut.configuration_ref != input.decision_context.configuration_ref
    {
        reasons.insert("EVIDENCE_BINDING_OWNER_CUT_SUBJECT_OR_CONFIGURATION_MISMATCH".to_owned());
    }

    let expected_evidence_obligations = evidence_class_obligations(&profile);
    let mapping_errors = validate_mapping_entries(&profile, input.mapping_entries);
    reasons.extend(mapping_errors.iter().cloned());

    let mapping_sha = match compute_evidence_mapping_manifest_sha256_v0_1(
        &input.claim_specification.claim_id,
        &context_sha,
        &semantic_sha,
        &profile,
        &required_manifest,
        &owner_cut_sha,
        &input.decision_context.trust_policy_sha256,
        input.mapping_entries,
    ) {
        Ok(value) => value,
        Err(error) => {
            reasons.insert(error);
            String::new()
        }
    };
    let receipt_errors = validate_mapping_receipt(
        input.mapping_receipt,
        input.owner_evidence_cut,
        &input.claim_specification.claim_id,
        &input.claim_specification.configuration_ref,
        &context_sha,
        &semantic_sha,
        &profile,
        &required_manifest,
        &owner_cut_sha,
        &mapping_sha,
        input.decision_context.as_known_at_unix_ms,
    );
    reasons.extend(receipt_errors.iter().cloned());

    let mut admissions = BTreeMap::new();
    for admission in input.required_evidence_admissions {
        if admissions.insert(admission.evidence_ref.as_str(), admission).is_some() {
            reasons.insert("EVIDENCE_BINDING_REQUIRED_EVIDENCE_DUPLICATE".to_owned());
        }
    }
    let mut source_receipts = BTreeMap::new();
    for receipt in input.source_receipts {
        if source_receipts.insert(receipt.evidence_ref.as_str(), receipt).is_some() {
            reasons.insert("EVIDENCE_BINDING_SOURCE_RECEIPT_DUPLICATE".to_owned());
        }
    }

    let mut mapped_refs = BTreeSet::new();
    let mut output_entries = Vec::new();
    let mut any_record_blocker = false;
    for obligation in expected_evidence_obligations {
        let Some(entry) = input
            .mapping_entries
            .iter()
            .find(|entry| entry.obligation_id == obligation.obligation_id)
        else {
            continue;
        };
        let mut records = Vec::new();
        let mut ordered_evidence_refs = entry.evidence_refs.iter().collect::<Vec<_>>();
        ordered_evidence_refs.sort_unstable();
        for evidence_ref in ordered_evidence_refs {
            mapped_refs.insert(evidence_ref.as_str());
            let record = evaluate_source_record(
                evidence_ref,
                admissions.get(evidence_ref.as_str()).copied(),
                source_receipts.get(evidence_ref.as_str()).copied(),
                input.owner_evidence_cut,
                input.decision_context,
            );
            if !record.reason_codes.is_empty() {
                any_record_blocker = true;
            }
            records.push(record);
        }
        let entry_disposition = disposition_for_reasons(&records, &mapping_errors);
        let entry_reasons = records
            .iter()
            .flat_map(|record| record.reason_codes.iter().cloned())
            .collect::<BTreeSet<_>>();
        output_entries.push(BoundEvidenceClassObligationV0_1 {
            obligation_id: entry.obligation_id.clone(),
            requirement_source_ref: entry.requirement_source_ref.clone(),
            evidence_class: entry.evidence_class.clone(),
            evidence_records: records,
            disposition: entry_disposition,
            reason_codes: entry_reasons.into_iter().collect(),
        });
    }
    if mapped_refs.len() != admission_ids.len()
        || admission_ids.iter().any(|item| !mapped_refs.contains(item.as_str()))
    {
        reasons.insert("EVIDENCE_BINDING_MAPPING_DOES_NOT_COVER_REQUIRED_EVIDENCE_SET".to_owned());
    }

    let source_receipt_ids = source_receipts.keys().copied().collect::<BTreeSet<_>>();
    if source_receipt_ids != mapped_refs {
        reasons.insert("EVIDENCE_BINDING_SOURCE_RECEIPT_SET_MISMATCH".to_owned());
    }
    if !mapping_errors.is_empty() {
        reasons.insert("EVIDENCE_BINDING_MAPPING_CONTRACT_INVALID".to_owned());
    }
    if !receipt_errors.is_empty() {
        reasons.insert("EVIDENCE_BINDING_MAPPING_APPROVAL_INVALID".to_owned());
    }

    let mut unassessed = profile
        .obligations
        .iter()
        .filter(|obligation| obligation.kind != super::assurance_disposition_v0_1::ObligationKindV0_1::EvidenceIntegrity)
        .map(|obligation| obligation.obligation_id.clone())
        .collect::<Vec<_>>();
    unassessed.sort();

    if !unassessed.is_empty() {
        reasons.insert("NON_EVIDENCE_CLASS_OBLIGATIONS_REMAIN_UNASSESSED".to_owned());
    }

    let entry_dispositions = output_entries
        .iter()
        .map(|entry| entry.disposition)
        .collect::<Vec<_>>();
    let hard_errors = reasons.iter().any(|reason| {
        reason.contains("MISMATCH")
            || reason.contains("INVALID")
            || reason.contains("DUPLICATE")
            || reason.contains("NOT_DERIVED")
            || reason.contains("REJECTED")
            || reason.contains("VERIFICATION_FAILED")
    });
    let unverified = reasons.iter().any(|reason| {
        reason.contains("NOT_UPSTREAM_VERIFIED")
            || reason.contains("NOT_KNOWN")
            || reason.contains("UNVERIFIED")
    });
    let other_incomplete = reasons
        .iter()
        .any(|reason| reason != "NON_EVIDENCE_CLASS_OBLIGATIONS_REMAIN_UNASSESSED");
    let disposition = if hard_errors
        || entry_dispositions.contains(&ClaimObligationEvidenceBindingDispositionV0_1::Rejected)
    {
        ClaimObligationEvidenceBindingDispositionV0_1::Rejected
    } else if unverified
        || entry_dispositions.contains(&ClaimObligationEvidenceBindingDispositionV0_1::Unverified)
    {
        ClaimObligationEvidenceBindingDispositionV0_1::Unverified
    } else if other_incomplete
        || any_record_blocker
        || entry_dispositions.contains(&ClaimObligationEvidenceBindingDispositionV0_1::Incomplete)
    {
        ClaimObligationEvidenceBindingDispositionV0_1::Incomplete
    } else {
        ClaimObligationEvidenceBindingDispositionV0_1::Bound
    };

    let mapping_receipt_sha = input
        .mapping_receipt
        .map(|receipt| receipt.receipt_sha256.clone())
        .unwrap_or_default();
    let mut output_entries = output_entries;
    output_entries.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));
    assessment_shell(
        &input,
        disposition,
        raw_sha,
        semantic_sha,
        context_sha,
        Some(&profile),
        owner_cut_sha,
        mapping_sha,
        mapping_receipt_sha,
        output_entries,
        unassessed,
        reasons,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim_assurance_gate_v0_1::{
        EvidenceAdmissionState, RequiredEvidenceAdmission,
    };
    use crate::decision_context_v0_1::DecisionContextV0_1;
    use crate::owner_evidence_cut_v0_1::{
        OwnerDependencyCoverage, OwnerEvidenceCutV0_1, OwnerEvidenceFreshness,
        OwnerEvidenceLifecycle, OwnerEvidenceRecordV0_1, OwnerObserverCoverageStatus,
    };
    use serde_json::Value;

    const SPECIFICATION_FIXTURE: &str = include_str!(
        "../../../../docs/research/aviation/examples/aero_claim_specification_v0_1.json"
    );

    const CUTOFF: u64 = 1_795_516_800_000;

    fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join("")
    }

    fn specification() -> ClaimSpecificationV0_1 {
        serde_json::from_str(SPECIFICATION_FIXTURE).expect("claim specification fixture parses")
    }

    fn build_case() -> (
        Vec<u8>,
        ClaimSpecificationV0_1,
        DecisionContextV0_1,
        OwnerEvidenceCutV0_1,
        Vec<RequiredEvidenceAdmission>,
        Vec<ClaimObligationEvidenceMappingEntryV0_1>,
        ClaimObligationEvidenceMappingReceiptV0_1,
        Vec<OwnerEvidenceSourceReceiptV0_1>,
    ) {
        let spec = specification();
        let raw = SPECIFICATION_FIXTURE.as_bytes().to_vec();
        let semantic_sha = spec.compute_semantic_sha256().expect("semantic digest");
        let required_refs = spec
            .required_evidence_classes
            .iter()
            .map(|class| format!("evidence:{}", class.replace(':', "-")))
            .collect::<Vec<_>>();
        let required_manifest = compute_required_evidence_manifest_sha256(&required_refs)
            .expect("required evidence manifest");
        let owner_cut = OwnerEvidenceCutV0_1 {
            schema: "AeroOwnerEvidenceCutV0_1".to_owned(),
            owner_namespace: "owner:materials-authority".to_owned(),
            component_packet_id: "packet:wing-r17".to_owned(),
            physical_unit_ref: spec.subject_ref.clone(),
            configuration_ref: spec.configuration_ref.clone(),
            component_packet_sha256: "a".repeat(64),
            component_packet_byte_length: 8192,
            artifact_payloads_verified: true,
            evidence_records: required_refs
                .iter()
                .map(|reference| OwnerEvidenceRecordV0_1 {
                    evidence_ref: reference.clone(),
                    record_sha256: sha256(reference.as_bytes()),
                    subject_ref: spec.subject_ref.clone(),
                    configuration_ref: spec.configuration_ref.clone(),
                    lifecycle: OwnerEvidenceLifecycle::Active,
                    predecessor_ref: None,
                    freshness: OwnerEvidenceFreshness::Fresh,
                    dependency_coverage: OwnerDependencyCoverage::Complete,
                    unresolved_inputs: Vec::new(),
                    requires_reverification: false,
                    eligible_under_declared_coverage: true,
                })
                .collect(),
            basalt_run_id: "basalt:wing-r17".to_owned(),
            basalt_report_version: 1,
            basalt_timeline_input_sha256: "b".repeat(64),
            basalt_final_state_sha256: "c".repeat(64),
            basalt_report_sha256: "d".repeat(64),
            basalt_confirmatory_eligible: true,
            receiver_receipt_manifest_sha256: "e".repeat(64),
            receiver_receipt_watermark_unix_ms: CUTOFF - 100,
            trust_policy_sha256: "f".repeat(64),
            observer_coverage_profile_id: "observer:synthetic".to_owned(),
            observer_coverage_receipt_sha256: "1".repeat(64),
            observer_coverage_status: OwnerObserverCoverageStatus::QualifiedCompleteForScope,
            observer_coverage_watermark_unix_ms: CUTOFF - 150,
        };
        let owner_cut_sha = owner_cut.compute_sha256().expect("owner cut digest");
        let context = DecisionContextV0_1 {
            claim_id: spec.claim_id.clone(),
            subject_ref: spec.subject_ref.clone(),
            configuration_ref: spec.configuration_ref.clone(),
            operating_envelope_ref: spec.operating_envelope_ref.clone(),
            acquisition_time_earliest_unix_ms: spec.validity.effective_from_unix_ms,
            acquisition_time_latest_unix_ms: spec.validity.effective_from_unix_ms + 10,
            as_known_at_unix_ms: CUTOFF,
            source_scope_manifest_sha256: "2".repeat(64),
            evidence_knowledge_cut_sha256: owner_cut_sha,
            trust_policy_sha256: owner_cut.trust_policy_sha256.clone(),
            claim_specification_sha256: semantic_sha.clone(),
            required_evidence_manifest_sha256: required_manifest.clone(),
            risk_classification_sha256: "3".repeat(64),
            model_applicability_profile_sha256: "4".repeat(64),
        };
        let context_sha = context.compute_sha256().expect("context digest");
        let profile = derive_claim_obligation_profile_v0_1(
            &spec,
            RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
        )
        .profile
        .expect("obligation profile");
        let entries = spec
            .required_evidence_classes
            .iter()
            .zip(required_refs.iter())
            .map(|(class, evidence_ref)| ClaimObligationEvidenceMappingEntryV0_1 {
                obligation_id: format!("evidence-integrity:{class}"),
                requirement_source_ref: format!("claim-spec:/required_evidence_classes/{class}"),
                evidence_class: class.clone(),
                evidence_refs: vec![evidence_ref.clone()],
            })
            .collect::<Vec<_>>();
        let mapping_sha = compute_evidence_mapping_manifest_sha256_v0_1(
            &spec.claim_id,
            &context_sha,
            &semantic_sha,
            &profile,
            &required_manifest,
            &owner_cut_sha,
            &owner_cut.trust_policy_sha256,
            &entries,
        )
        .expect("mapping manifest");
        let mapping_receipt = ClaimObligationEvidenceMappingReceiptV0_1 {
            verification_status: EvidenceBindingReceiptStatusV0_1::Verified,
            owner_namespace: owner_cut.owner_namespace.clone(),
            claim_id: spec.claim_id.clone(),
            decision_context_sha256: context_sha.clone(),
            configuration_ref: spec.configuration_ref.clone(),
            claim_specification_semantic_sha256: semantic_sha,
            obligation_profile_sha256: profile.profile_sha256,
            obligation_manifest_sha256: profile.obligation_manifest_sha256,
            required_evidence_manifest_sha256: required_manifest,
            owner_evidence_cut_sha256: owner_cut_sha,
            trust_policy_sha256: owner_cut.trust_policy_sha256.clone(),
            mapping_manifest_sha256: mapping_sha,
            receipt_sha256: "5".repeat(64),
            receiver_first_seen_at_unix_ms: CUTOFF - 200,
            effective_from_unix_ms: CUTOFF - 300,
            effective_until_unix_ms: CUTOFF + 300,
        };
        let admissions = required_refs
            .iter()
            .map(|reference| RequiredEvidenceAdmission {
                evidence_ref: reference.clone(),
                decision_context_sha256: context_sha.clone(),
                state: EvidenceAdmissionState::Admitted,
            })
            .collect();
        let source_receipts = required_refs
            .iter()
            .map(|reference| {
                let record = owner_cut.find_evidence(reference).unwrap();
                OwnerEvidenceSourceReceiptV0_1 {
                    verification_status: EvidenceBindingReceiptStatusV0_1::Verified,
                    owner_namespace: owner_cut.owner_namespace.clone(),
                    producer_namespace: "producer:materials-lab".to_owned(),
                    claim_id: spec.claim_id.clone(),
                    decision_context_sha256: context_sha.clone(),
                    subject_ref: spec.subject_ref.clone(),
                    configuration_ref: spec.configuration_ref.clone(),
                    evidence_ref: reference.clone(),
                    evidence_record_sha256: record.record_sha256.clone(),
                    trust_policy_sha256: owner_cut.trust_policy_sha256.clone(),
                    integrity_state: EvidenceBindingIntegrityStateV0_1::Verified,
                    producer_authority_state: EvidenceProducerAuthorityStateV0_1::Authorized,
                    receiver_first_seen_at_unix_ms: CUTOFF - 50,
                    evidence_effective_from_unix_ms: spec.validity.effective_from_unix_ms,
                    evidence_effective_until_unix_ms: spec.validity.effective_until_unix_ms,
                    authority_effective_from_unix_ms: CUTOFF - 1000,
                    authority_effective_until_unix_ms: CUTOFF + 1000,
                    receipt_sha256: "6".repeat(64),
                }
            })
            .collect();
        (
            raw,
            spec,
            context,
            owner_cut,
            admissions,
            entries,
            mapping_receipt,
            source_receipts,
        )
    }

    fn run(
        raw: &[u8],
        spec: &ClaimSpecificationV0_1,
        context: &DecisionContextV0_1,
        owner_cut: &OwnerEvidenceCutV0_1,
        admissions: &[RequiredEvidenceAdmission],
        entries: &[ClaimObligationEvidenceMappingEntryV0_1],
        receipt: Option<&ClaimObligationEvidenceMappingReceiptV0_1>,
        source_receipts: &[OwnerEvidenceSourceReceiptV0_1],
    ) -> ClaimObligationEvidenceBindingAssessmentV0_1 {
        let raw_sha = sha256(raw);
        bind_claim_obligation_evidence_v0_1(ClaimObligationEvidenceBindingInputV0_1 {
            schema: CLAIM_OBLIGATION_EVIDENCE_BINDING_SCHEMA_V0_1.to_owned(),
            raw_claim_specification_payload: raw,
            expected_raw_claim_specification_sha256: &raw_sha,
            claim_specification: spec,
            decision_context: context,
            requirement_profile: RegisteredClaimRequirementProfileV0_1::AeroScalarEngineeringClaimV0_1,
            owner_evidence_cut: owner_cut,
            required_evidence_admissions: admissions,
            mapping_entries: entries,
            mapping_receipt: receipt,
            source_receipts,
        })
    }

    #[test]
    fn binds_every_required_evidence_class_to_known_authorized_source_records() {
        let (raw, spec, context, owner_cut, admissions, entries, receipt, source_receipts) =
            build_case();
        let result = run(
            &raw,
            &spec,
            &context,
            &owner_cut,
            &admissions,
            &entries,
            Some(&receipt),
            &source_receipts,
        );
        assert_eq!(result.disposition, ClaimObligationEvidenceBindingDispositionV0_1::Bound);
        assert_eq!(result.evidence_class_obligations.len(), spec.required_evidence_classes.len());
        assert!(!result.unassessed_obligation_ids.is_empty());
        assert!(!result.complete_claim_assurance_state);
        assert!(result.reason_codes.contains(
            &"NON_EVIDENCE_CLASS_OBLIGATIONS_REMAIN_UNASSESSED".to_owned()
        ));
        assert!(!result.assurance_state_mutated);
        assert!(!result.physical_truth_established);
        assert!(!result.engineering_acceptance_granted);
        assert!(!result.physical_work_authorized);
    }

    #[test]
    fn missing_one_derived_evidence_class_mapping_fails_closed() {
        let (raw, spec, context, owner_cut, admissions, mut entries, receipt, source_receipts) =
            build_case();
        entries.pop();
        let result = run(
            &raw,
            &spec,
            &context,
            &owner_cut,
            &admissions,
            &entries,
            Some(&receipt),
            &source_receipts,
        );
        assert_ne!(result.disposition, ClaimObligationEvidenceBindingDispositionV0_1::Bound);
        assert!(result.reason_codes.iter().any(|reason| {
            reason == "EVIDENCE_MAPPING_DERIVED_OBLIGATION_SET_INCOMPLETE"
        }));
    }

    #[test]
    fn late_per_record_source_receipt_is_not_known_at_cutoff() {
        let (raw, spec, context, owner_cut, admissions, entries, receipt, mut source_receipts) =
            build_case();
        source_receipts[0].receiver_first_seen_at_unix_ms = CUTOFF + 1;
        let result = run(
            &raw,
            &spec,
            &context,
            &owner_cut,
            &admissions,
            &entries,
            Some(&receipt),
            &source_receipts,
        );
        assert_eq!(result.disposition, ClaimObligationEvidenceBindingDispositionV0_1::Unverified);
        assert!(result.evidence_class_obligations.iter().any(|entry| {
            entry.evidence_records.iter().any(|record| {
                record.temporal_state == EvidenceTemporalStateV0_1::NotKnownAtCutoff
            })
        }));
    }

    #[test]
    fn producer_authority_denial_rejects_the_evidence_binding() {
        let (raw, spec, context, owner_cut, admissions, entries, receipt, mut source_receipts) =
            build_case();
        source_receipts[0].producer_authority_state = EvidenceProducerAuthorityStateV0_1::Denied;
        let result = run(
            &raw,
            &spec,
            &context,
            &owner_cut,
            &admissions,
            &entries,
            Some(&receipt),
            &source_receipts,
        );
        assert_eq!(result.disposition, ClaimObligationEvidenceBindingDispositionV0_1::Rejected);
    }

    #[test]
    fn mapping_manifest_mutation_requires_new_owner_approval() {
        let (raw, spec, context, owner_cut, admissions, entries, mut receipt, source_receipts) =
            build_case();
        receipt.mapping_manifest_sha256 = "0".repeat(64);
        let result = run(
            &raw,
            &spec,
            &context,
            &owner_cut,
            &admissions,
            &entries,
            Some(&receipt),
            &source_receipts,
        );
        assert_eq!(result.disposition, ClaimObligationEvidenceBindingDispositionV0_1::Rejected);
    }

    #[test]
    fn source_record_digest_substitution_is_rejected() {
        let (raw, spec, context, owner_cut, admissions, entries, receipt, mut source_receipts) =
            build_case();
        source_receipts[0].evidence_record_sha256 = "9".repeat(64);
        let result = run(
            &raw,
            &spec,
            &context,
            &owner_cut,
            &admissions,
            &entries,
            Some(&receipt),
            &source_receipts,
        );
        assert_eq!(result.disposition, ClaimObligationEvidenceBindingDispositionV0_1::Rejected);
    }

    #[test]
    fn source_order_permutation_preserves_result() {
        let (raw, spec, context, owner_cut, admissions, entries, receipt, source_receipts) =
            build_case();
        let baseline = run(
            &raw,
            &spec,
            &context,
            &owner_cut,
            &admissions,
            &entries,
            Some(&receipt),
            &source_receipts,
        );
        let mut permuted = source_receipts.clone();
        permuted.reverse();
        let result = run(
            &raw,
            &spec,
            &context,
            &owner_cut,
            &admissions,
            &entries,
            Some(&receipt),
            &permuted,
        );
        assert_eq!(baseline, result);
    }
}
