// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-bound model applicability, population-stratum and uncertainty obligations.
//!
//! This adapter consumes the exact evidence-class binding input and reruns that
//! pure adapter before it trusts a model-validation evidence record. It then
//! binds the exact ModelCredibilityCut to claim-spec strata, uncertainty method,
//! configuration, risk and operating-envelope context through an owner-approved
//! mapping receipt. It does not calculate VVUQ or establish physical truth.

use super::assurance_disposition_v0_1::ObligationKindV0_1;
use super::claim_obligation_evidence_binding_v0_1::{
    bind_claim_obligation_evidence_v0_1, ClaimObligationEvidenceBindingDispositionV0_1,
    ClaimObligationEvidenceBindingInputV0_1, EvidenceBindingEffectivityStateV0_1,
    EvidenceBindingIntegrityStateV0_1, EvidenceProducerAuthorityStateV0_1,
    EvidenceTemporalStateV0_1,
};
use super::claim_obligation_profile_v0_1::{
    derive_claim_obligation_profile_v0_1, DerivedClaimObligationProfileV0_1,
    RegisteredClaimRequirementProfileV0_1,
};
use super::claim_specification_v0_1::ClaimSpecificationV0_1;
use super::decision_context_v0_1::DecisionContextV0_1;
use super::model_credibility_cut_v0_1::{
    MetricStratumStatus, ModelCredibilityCutV0_1, ModelDomainCoverageStatus,
    ModelPackageStatus, SplitIntegrityStatus, UncertaintyComponentKind,
    UncertaintyEvidenceStatus,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const CLAIM_MODEL_OBLIGATION_BINDING_SCHEMA_V0_1: &str =
    "AeroClaimModelObligationBindingV0_1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelObligationReceiptStatusV0_1 {
    #[serde(rename = "VERIFIED")]
    Verified,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "FAILED")]
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ModelObligationDispositionV0_1 {
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
pub struct ModelMetricStratumMappingEntryV0_1 {
    /// Derived obligation ID: model-credibility:<claim-spec stratum ID>.
    pub obligation_id: String,
    pub requirement_source_ref: String,
    pub claim_stratum_id: String,
    /// Exact model-owner metric identifier.
    pub metric_id: String,
    /// Exact model-owner stratum reference; its mapping is owner-approved.
    pub model_stratum_ref: String,
    pub population_manifest_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelUncertaintyMappingEntryV0_1 {
    /// Derived obligation ID: model-uncertainty.
    pub obligation_id: String,
    pub requirement_source_ref: String,
    /// Exact method identifier in ClaimSpecificationV0_1.
    pub uncertainty_method_ref: String,
    pub component: UncertaintyComponentKind,
    /// Exact model-owner profile and report commitments.
    pub method_profile_sha256: String,
    pub report_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelApplicabilityMappingEntryV0_1 {
    /// Derived obligation ID: model-applicability.
    pub obligation_id: String,
    pub requirement_source_ref: String,
    pub model_ref: String,
    pub model_artifact_sha256: String,
    pub solver_configuration_sha256: String,
    pub operating_envelope_ref: String,
    pub applicability_profile_sha256: String,
    pub risk_classification_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimModelObligationMappingReceiptV0_1 {
    pub verification_status: ModelObligationReceiptStatusV0_1,
    pub owner_namespace: String,
    pub claim_id: String,
    pub decision_context_sha256: String,
    pub configuration_ref: String,
    pub claim_specification_semantic_sha256: String,
    pub obligation_profile_sha256: String,
    pub obligation_manifest_sha256: String,
    pub model_evidence_ref: String,
    pub model_evidence_record_sha256: String,
    pub model_credibility_cut_sha256: String,
    pub applicability_mapping_sha256: String,
    pub metric_strata_mapping_sha256: String,
    pub uncertainty_mapping_sha256: String,
    /// Exact uncertainty treatment and confidence from the claim specification.
    pub uncertainty_treatment: String,
    pub uncertainty_method_ref: String,
    pub uncertainty_confidence_level_ppm: Option<u32>,
    pub receipt_sha256: String,
    pub receiver_first_seen_at_unix_ms: u64,
    pub effective_from_unix_ms: u64,
    pub effective_until_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimModelObligationBindingInputV0_1<'a> {
    pub schema: String,
    pub requirement_profile: RegisteredClaimRequirementProfileV0_1,
    pub evidence_binding_input: &'a ClaimObligationEvidenceBindingInputV0_1<'a>,
    pub model_credibility_cut: &'a ModelCredibilityCutV0_1,
    pub applicability_mapping: &'a ModelApplicabilityMappingEntryV0_1,
    pub metric_strata_mappings: &'a [ModelMetricStratumMappingEntryV0_1],
    pub uncertainty_mappings: &'a [ModelUncertaintyMappingEntryV0_1],
    pub mapping_receipt: Option<&'a ClaimModelObligationMappingReceiptV0_1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoundMetricStratumV0_1 {
    pub obligation_id: String,
    pub claim_stratum_id: String,
    pub population_manifest_sha256: String,
    pub metric_id: String,
    pub model_stratum_ref: String,
    pub report_sha256: String,
    pub uncertainty_report_sha256: String,
    pub sample_count: u64,
    pub adverse_count: u64,
    pub status: String,
    pub disposition: ModelObligationDispositionV0_1,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoundModelUncertaintyV0_1 {
    pub obligation_id: String,
    pub method_ref: String,
    pub treatment: String,
    pub confidence_level_ppm: Option<u32>,
    pub component: String,
    pub method_profile_sha256: String,
    pub report_sha256: String,
    pub status: String,
    pub disposition: ModelObligationDispositionV0_1,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BoundModelApplicabilityV0_1 {
    pub obligation_id: String,
    pub model_ref: String,
    pub model_artifact_sha256: String,
    pub solver_configuration_sha256: String,
    pub operating_envelope_ref: String,
    pub applicability_profile_sha256: String,
    pub risk_classification_sha256: String,
    pub domain_coverage_status: String,
    pub disposition: ModelObligationDispositionV0_1,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClaimModelObligationBindingAssessmentV0_1 {
    pub disposition: ModelObligationDispositionV0_1,
    pub claim_id: String,
    pub configuration_ref: String,
    pub decision_context_sha256: String,
    pub claim_specification_semantic_sha256: String,
    pub obligation_profile_sha256: String,
    pub obligation_manifest_sha256: String,
    pub model_evidence_ref: String,
    pub model_evidence_record_sha256: String,
    pub model_credibility_cut_sha256: String,
    pub mapping_receipt_sha256: String,
    pub applicability: Option<BoundModelApplicabilityV0_1>,
    pub metric_strata: Vec<BoundMetricStratumV0_1>,
    pub uncertainty_components: Vec<BoundModelUncertaintyV0_1>,
    pub unassessed_obligation_ids: Vec<String>,
    pub complete_claim_assurance_state: bool,
    pub assurance_state_mutated: bool,
    pub physical_truth_established: bool,
    pub engineering_acceptance_granted: bool,
    pub physical_work_authorized: bool,
    pub reason_codes: Vec<String>,
}

#[derive(Serialize)]
struct ApplicabilityMappingEnvelopeV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    claim_id: &'a str,
    decision_context_sha256: &'a str,
    claim_specification_semantic_sha256: &'a str,
    obligation_profile_sha256: &'a str,
    obligation_manifest_sha256: &'a str,
    model_credibility_cut_sha256: &'a str,
    entry: &'a ModelApplicabilityMappingEntryV0_1,
}

#[derive(Serialize)]
struct MetricMappingManifestEnvelopeV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    claim_id: &'a str,
    decision_context_sha256: &'a str,
    claim_specification_semantic_sha256: &'a str,
    obligation_profile_sha256: &'a str,
    obligation_manifest_sha256: &'a str,
    model_credibility_cut_sha256: &'a str,
    entries: Vec<&'a ModelMetricStratumMappingEntryV0_1>,
}

#[derive(Serialize)]
struct UncertaintyMappingManifestEnvelopeV0_1<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    claim_id: &'a str,
    decision_context_sha256: &'a str,
    claim_specification_semantic_sha256: &'a str,
    obligation_profile_sha256: &'a str,
    obligation_manifest_sha256: &'a str,
    model_credibility_cut_sha256: &'a str,
    uncertainty_method_ref: &'a str,
    uncertainty_treatment: &'a str,
    confidence_level_ppm: Option<u32>,
    entries: Vec<&'a ModelUncertaintyMappingEntryV0_1>,
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

fn valid_interval(from: u64, until: u64) -> bool {
    from < until
}

fn interval_contains_instant(from: u64, until: u64, instant: u64) -> bool {
    valid_interval(from, until) && from <= instant && instant < until
}

fn find_obligation<'a>(
    profile: &'a DerivedClaimObligationProfileV0_1,
    obligation_id: &str,
) -> Option<&'a super::claim_obligation_profile_v0_1::DerivedClaimObligationV0_1> {
    profile
        .obligations
        .iter()
        .find(|item| item.obligation_id == obligation_id)
}

pub fn compute_model_applicability_mapping_sha256_v0_1(
    claim_id: &str,
    context_sha: &str,
    semantic_sha: &str,
    profile: &DerivedClaimObligationProfileV0_1,
    model_cut_sha: &str,
    entry: &ModelApplicabilityMappingEntryV0_1,
) -> Result<String, String> {
    let envelope = ApplicabilityMappingEnvelopeV0_1 {
        schema: "AeroModelApplicabilityMappingManifestV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        claim_id,
        decision_context_sha256: context_sha,
        claim_specification_semantic_sha256: semantic_sha,
        obligation_profile_sha256: &profile.profile_sha256,
        obligation_manifest_sha256: &profile.obligation_manifest_sha256,
        model_credibility_cut_sha256: model_cut_sha,
        entry,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "MODEL_APPLICABILITY_MAPPING_SERIALIZATION_FAILED".to_owned())
}

pub fn compute_model_metric_strata_mapping_sha256_v0_1(
    claim_id: &str,
    context_sha: &str,
    semantic_sha: &str,
    profile: &DerivedClaimObligationProfileV0_1,
    model_cut_sha: &str,
    entries: &[ModelMetricStratumMappingEntryV0_1],
) -> Result<String, String> {
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.metric_id
            .cmp(&right.metric_id)
            .then_with(|| left.model_stratum_ref.cmp(&right.model_stratum_ref))
    });
    let envelope = MetricMappingManifestEnvelopeV0_1 {
        schema: "AeroModelMetricStrataMappingManifestV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        claim_id,
        decision_context_sha256: context_sha,
        claim_specification_semantic_sha256: semantic_sha,
        obligation_profile_sha256: &profile.profile_sha256,
        obligation_manifest_sha256: &profile.obligation_manifest_sha256,
        model_credibility_cut_sha256: model_cut_sha,
        entries: ordered,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "MODEL_METRIC_STRATA_MAPPING_SERIALIZATION_FAILED".to_owned())
}

pub fn compute_model_uncertainty_mapping_sha256_v0_1(
    claim_id: &str,
    context_sha: &str,
    semantic_sha: &str,
    profile: &DerivedClaimObligationProfileV0_1,
    model_cut_sha: &str,
    uncertainty_method_ref: &str,
    uncertainty_treatment: &str,
    confidence_level_ppm: Option<u32>,
    entries: &[ModelUncertaintyMappingEntryV0_1],
) -> Result<String, String> {
    let mut ordered = entries.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|entry| entry.component);
    let envelope = UncertaintyMappingManifestEnvelopeV0_1 {
        schema: "AeroModelUncertaintyMappingManifestV0_1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        claim_id,
        decision_context_sha256: context_sha,
        claim_specification_semantic_sha256: semantic_sha,
        obligation_profile_sha256: &profile.profile_sha256,
        obligation_manifest_sha256: &profile.obligation_manifest_sha256,
        model_credibility_cut_sha256: model_cut_sha,
        uncertainty_method_ref,
        uncertainty_treatment,
        confidence_level_ppm,
        entries: ordered,
    };
    serde_json::to_vec(&envelope)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| "MODEL_UNCERTAINTY_MAPPING_SERIALIZATION_FAILED".to_owned())
}

fn validate_model_receipt(
    receipt: Option<&ClaimModelObligationMappingReceiptV0_1>,
    owner_namespace: &str,
    spec: &ClaimSpecificationV0_1,
    profile: &DerivedClaimObligationProfileV0_1,
    model_cut: &ModelCredibilityCutV0_1,
    model_cut_sha: &str,
    context_sha: &str,
    semantic_sha: &str,
    applicability_sha: &str,
    metric_sha: &str,
    uncertainty_sha: &str,
    cutoff: u64,
) -> BTreeSet<String> {
    let mut errors = BTreeSet::new();
    let Some(receipt) = receipt else {
        errors.insert("MODEL_OBLIGATION_MAPPING_RECEIPT_MISSING".to_owned());
        return errors;
    };
    match receipt.verification_status {
        ModelObligationReceiptStatusV0_1::Verified => {}
        ModelObligationReceiptStatusV0_1::Unverified => {
            errors.insert("MODEL_OBLIGATION_MAPPING_RECEIPT_NOT_UPSTREAM_VERIFIED".to_owned());
        }
        ModelObligationReceiptStatusV0_1::Failed => {
            errors.insert("MODEL_OBLIGATION_MAPPING_RECEIPT_VERIFICATION_FAILED".to_owned());
        }
    }
    if !valid_identifier(&receipt.owner_namespace)
        || !valid_sha256(&receipt.receipt_sha256)
        || receipt.owner_namespace != owner_namespace
        || receipt.claim_id != spec.claim_id
        || receipt.configuration_ref != spec.configuration_ref
        || receipt.decision_context_sha256 != context_sha
        || receipt.claim_specification_semantic_sha256 != semantic_sha
        || receipt.obligation_profile_sha256 != profile.profile_sha256
        || receipt.obligation_manifest_sha256 != profile.obligation_manifest_sha256
        || receipt.model_evidence_ref != model_cut.evidence_ref
        || receipt.model_evidence_record_sha256 != model_cut_sha
        || receipt.model_credibility_cut_sha256 != model_cut_sha
        || receipt.applicability_mapping_sha256 != applicability_sha
        || receipt.metric_strata_mapping_sha256 != metric_sha
        || receipt.uncertainty_mapping_sha256 != uncertainty_sha
        || receipt.uncertainty_treatment != format!("{:?}", spec.uncertainty.treatment)
        || receipt.uncertainty_method_ref != spec.uncertainty.method_ref
        || receipt.uncertainty_confidence_level_ppm != spec.uncertainty.confidence_level_ppm
    {
        errors.insert("MODEL_OBLIGATION_MAPPING_RECEIPT_SCOPE_OR_COMMITMENT_MISMATCH".to_owned());
    }
    if receipt.receiver_first_seen_at_unix_ms > cutoff {
        errors.insert("MODEL_OBLIGATION_MAPPING_RECEIPT_NOT_KNOWN_AT_CUTOFF".to_owned());
    }
    if !interval_contains_instant(
        receipt.effective_from_unix_ms,
        receipt.effective_until_unix_ms,
        cutoff,
    ) {
        errors.insert("MODEL_OBLIGATION_MAPPING_RECEIPT_NOT_EFFECTIVE_AT_CUTOFF".to_owned());
    }
    errors
}

fn classify_metric_status(status: MetricStratumStatus) -> (ModelObligationDispositionV0_1, &'static str) {
    match status {
        MetricStratumStatus::WithinDeclaredProfile => (ModelObligationDispositionV0_1::Bound, "METRIC_STRATUM_WITHIN_DECLARED_PROFILE"),
        MetricStratumStatus::OutsideDeclaredProfile => (ModelObligationDispositionV0_1::OutOfScope, "METRIC_STRATUM_OUTSIDE_DECLARED_PROFILE"),
        MetricStratumStatus::Indeterminate => (ModelObligationDispositionV0_1::Incomplete, "METRIC_STRATUM_INDETERMINATE"),
        MetricStratumStatus::Unverified => (ModelObligationDispositionV0_1::Unverified, "METRIC_STRATUM_UNVERIFIED"),
    }
}

fn classify_uncertainty_status(status: UncertaintyEvidenceStatus) -> (ModelObligationDispositionV0_1, &'static str) {
    match status {
        UncertaintyEvidenceStatus::Characterized => (ModelObligationDispositionV0_1::Bound, "UNCERTAINTY_COMPONENT_CHARACTERIZED"),
        UncertaintyEvidenceStatus::Uncharacterized => (ModelObligationDispositionV0_1::Incomplete, "UNCERTAINTY_COMPONENT_UNCHARACTERIZED"),
        UncertaintyEvidenceStatus::Unverified => (ModelObligationDispositionV0_1::Unverified, "UNCERTAINTY_COMPONENT_UNVERIFIED"),
    }
}

/// Bind one ModelCredibilityCut to the exact model-validation evidence record
/// already established by the source-bound evidence-class adapter.
pub fn bind_claim_model_obligations_v0_1(
    input: ClaimModelObligationBindingInputV0_1<'_>,
) -> ClaimModelObligationBindingAssessmentV0_1 {
    let spec = input.evidence_binding_input.claim_specification;
    let context = input.evidence_binding_input.decision_context;
    let model = input.model_credibility_cut;
    let reject = |reason: &str| ClaimModelObligationBindingAssessmentV0_1 {
        disposition: ModelObligationDispositionV0_1::Rejected,
        claim_id: spec.claim_id.clone(),
        configuration_ref: spec.configuration_ref.clone(),
        decision_context_sha256: String::new(),
        claim_specification_semantic_sha256: String::new(),
        obligation_profile_sha256: String::new(),
        obligation_manifest_sha256: String::new(),
        model_evidence_ref: model.evidence_ref.clone(),
        model_evidence_record_sha256: String::new(),
        model_credibility_cut_sha256: String::new(),
        mapping_receipt_sha256: input.mapping_receipt.map(|r| r.receipt_sha256.clone()).unwrap_or_default(),
        applicability: None,
        metric_strata: Vec::new(),
        uncertainty_components: Vec::new(),
        unassessed_obligation_ids: Vec::new(),
        complete_claim_assurance_state: false,
        assurance_state_mutated: false,
        physical_truth_established: false,
        engineering_acceptance_granted: false,
        physical_work_authorized: false,
        reason_codes: vec![reason.to_owned()],
    };

    if input.schema != CLAIM_MODEL_OBLIGATION_BINDING_SCHEMA_V0_1 {
        return reject("MODEL_OBLIGATION_BINDING_SCHEMA_INVALID");
    }

    let evidence_binding = bind_claim_obligation_evidence_v0_1(
        input.evidence_binding_input.clone(),
    );
    if evidence_binding.disposition != ClaimObligationEvidenceBindingDispositionV0_1::Bound {
        return reject("MODEL_OBLIGATION_UPSTREAM_EVIDENCE_BINDING_NOT_BOUND");
    }

    let semantic_sha = match spec.compute_semantic_sha256() {
        Ok(value) => value,
        Err(_) => return reject("MODEL_OBLIGATION_CLAIM_SPECIFICATION_INVALID"),
    };
    let context_sha = match context.compute_sha256() {
        Ok(value) => value,
        Err(_) => return reject("MODEL_OBLIGATION_DECISION_CONTEXT_INVALID"),
    };
    if evidence_binding.claim_specification_semantic_sha256 != semantic_sha
        || evidence_binding.decision_context_sha256 != context_sha
    {
        return reject("MODEL_OBLIGATION_UPSTREAM_BINDING_SCOPE_MISMATCH");
    }

    let profile_assessment = derive_claim_obligation_profile_v0_1(
        spec,
        input.requirement_profile,
    );
    let Some(profile) = profile_assessment.profile else {
        return reject("MODEL_OBLIGATION_PROFILE_NOT_DERIVED");
    };

    let model_cut_sha = match model.compute_sha256() {
        Ok(value) => value,
        Err(_) => return reject("MODEL_OBLIGATION_MODEL_CUT_INVALID"),
    };
    let mut reasons = BTreeSet::new();
    for (code, condition) in [
        ("MODEL_OBLIGATION_CLAIM_ID_MISMATCH", model.claim_id != spec.claim_id),
        ("MODEL_OBLIGATION_CONFIGURATION_MISMATCH", model.configuration_ref != context.configuration_ref),
        ("MODEL_OBLIGATION_OPERATING_ENVELOPE_MISMATCH", model.operating_envelope_ref != context.operating_envelope_ref),
        ("MODEL_OBLIGATION_APPLICABILITY_PROFILE_MISMATCH", model.applicability_profile_sha256 != context.model_applicability_profile_sha256),
        ("MODEL_OBLIGATION_RISK_CLASSIFICATION_MISMATCH", model.risk_classification_sha256 != context.risk_classification_sha256),
    ] {
        if condition { reasons.insert(code.to_owned()); }
    }

    let evidence_record = evidence_binding
        .evidence_class_obligations
        .iter()
        .find(|obligation| obligation.evidence_class == "evidence-class:model-validation")
        .and_then(|obligation| obligation.evidence_records.iter().find(|record| record.evidence_ref == model.evidence_ref));
    let Some(evidence_record) = evidence_record else {
        reasons.insert("MODEL_OBLIGATION_SOURCE_RECORD_NOT_BOUND_AS_MODEL_VALIDATION_EVIDENCE".to_owned());
        return reject("MODEL_OBLIGATION_SOURCE_RECORD_NOT_BOUND_AS_MODEL_VALIDATION_EVIDENCE");
    };
    if evidence_record.evidence_record_sha256 != model_cut_sha {
        reasons.insert("MODEL_OBLIGATION_MODEL_CUT_DIGEST_DIFFERS_FROM_SOURCE_RECORD".to_owned());
    }
    if evidence_record.admission_state != "ADMITTED"
        || evidence_record.integrity_state != EvidenceBindingIntegrityStateV0_1::Verified
        || evidence_record.producer_authority_state
            != EvidenceProducerAuthorityStateV0_1::Authorized
        || evidence_record.temporal_state != EvidenceTemporalStateV0_1::KnownAtCutoff
        || evidence_record.effectivity_state
            != EvidenceBindingEffectivityStateV0_1::CoversAcquisitionInterval
        || !evidence_record.reason_codes.is_empty()
    {
        reasons.insert("MODEL_OBLIGATION_SOURCE_RECORD_NOT_ADMITTED".to_owned());
    }

    let applicability_obligation = match find_obligation(&profile, "model-applicability") {
        Some(value) => value,
        None => return reject("MODEL_OBLIGATION_APPLICABILITY_REQUIREMENT_NOT_DERIVED"),
    };
    let applicability_entry = input.applicability_mapping;
    if applicability_entry.obligation_id != applicability_obligation.obligation_id
        || applicability_entry.requirement_source_ref != applicability_obligation.requirement_source_ref
        || applicability_entry.model_ref != model.model_ref
        || applicability_entry.model_artifact_sha256 != model.model_artifact_sha256
        || applicability_entry.solver_configuration_sha256 != model.solver_configuration_sha256
        || applicability_entry.operating_envelope_ref != spec.operating_envelope_ref
        || applicability_entry.applicability_profile_sha256 != context.model_applicability_profile_sha256
        || applicability_entry.risk_classification_sha256 != context.risk_classification_sha256
    {
        reasons.insert("MODEL_APPLICABILITY_MAPPING_SCOPE_MISMATCH".to_owned());
    }

    let metric_sha = compute_model_metric_strata_mapping_sha256_v0_1(
        &spec.claim_id, &context_sha, &semantic_sha, &profile, &model_cut_sha,
        input.metric_strata_mappings,
    ).unwrap_or_default();
    let uncertainty_sha = compute_model_uncertainty_mapping_sha256_v0_1(
        &spec.claim_id, &context_sha, &semantic_sha, &profile, &model_cut_sha,
        &spec.uncertainty.method_ref,
        match spec.uncertainty.treatment {
            super::claim_specification_v0_1::UncertaintyTreatmentV0_1::DeterministicUpperBound => "deterministic_upper_bound",
            super::claim_specification_v0_1::UncertaintyTreatmentV0_1::StatisticalUpperConfidenceBound => "statistical_upper_confidence_bound",
            super::claim_specification_v0_1::UncertaintyTreatmentV0_1::StatisticalLowerConfidenceBound => "statistical_lower_confidence_bound",
            super::claim_specification_v0_1::UncertaintyTreatmentV0_1::BayesianCredibleInterval => "bayesian_credible_interval",
            super::claim_specification_v0_1::UncertaintyTreatmentV0_1::NotSpecified => "not_specified",
        },
        spec.uncertainty.confidence_level_ppm,
        input.uncertainty_mappings,
    ).unwrap_or_default();

    let mut metric_entries = Vec::new();
    let mut seen_metric_pairs = BTreeSet::new();
    let required_model_pairs = model.required_metric_strata.iter()
        .map(|pair| (pair.metric_id.as_str(), pair.stratum_ref.as_str()))
        .collect::<BTreeSet<_>>();
    let mut claim_strata_covered = BTreeSet::new();
    let mut metric_dispositions = Vec::new();
    for entry in input.metric_strata_mappings {
        let Some(obligation) = find_obligation(&profile, &entry.obligation_id) else {
            reasons.insert("MODEL_METRIC_MAPPING_OBLIGATION_NOT_DERIVED".to_owned());
            continue;
        };
        if obligation.kind != ObligationKindV0_1::ModelCredibility
            || obligation.requirement_source_ref != entry.requirement_source_ref
            || !entry.obligation_id.starts_with("model-credibility:")
        {
            reasons.insert("MODEL_METRIC_MAPPING_REQUIREMENT_SOURCE_MISMATCH".to_owned());
            continue;
        }
        let Some(stratum) = spec.population.required_strata.iter().find(|s| s.stratum_id == entry.claim_stratum_id) else {
            reasons.insert("MODEL_METRIC_MAPPING_CLAIM_STRATUM_UNKNOWN".to_owned());
            continue;
        };
        let expected_scoped_ref = format!("{}|{}|{}", spec.population.population_ref, stratum.dimension_ref, stratum.level_ref);
        if entry.obligation_id != format!("model-credibility:{}", stratum.stratum_id)
            || entry.population_manifest_sha256 != spec.population.population_manifest_sha256
            || !required_model_pairs.contains(&(entry.metric_id.as_str(), entry.model_stratum_ref.as_str()))
            || !seen_metric_pairs.insert((entry.metric_id.as_str(), entry.model_stratum_ref.as_str()))
            || obligation.scoped_subject_ref != expected_scoped_ref
        {
            reasons.insert("MODEL_METRIC_MAPPING_SCOPE_OR_DUPLICATE_MISMATCH".to_owned());
        } else {
            claim_strata_covered.insert(stratum.stratum_id.as_str());
        }
        let report = model.metric_stratum_reports.iter().find(|r| r.metric_id == entry.metric_id && r.stratum_ref == entry.model_stratum_ref);
        match report {
            None => {
                reasons.insert("MODEL_METRIC_MAPPING_REPORT_MISSING".to_owned());
            }
            Some(report) => {
                if report.population_manifest_sha256 != spec.population.population_manifest_sha256 {
                    reasons.insert("MODEL_METRIC_REPORT_POPULATION_MANIFEST_MISMATCH".to_owned());
                }
                let (disposition, reason) = classify_metric_status(report.status);
                metric_dispositions.push(disposition);
                metric_entries.push(BoundMetricStratumV0_1 {
                    obligation_id: entry.obligation_id.clone(),
                    claim_stratum_id: entry.claim_stratum_id.clone(),
                    population_manifest_sha256: report.population_manifest_sha256.clone(),
                    metric_id: report.metric_id.clone(),
                    model_stratum_ref: report.stratum_ref.clone(),
                    report_sha256: report.report_sha256.clone(),
                    uncertainty_report_sha256: report.uncertainty_report_sha256.clone(),
                    sample_count: report.sample_count,
                    adverse_count: report.adverse_count,
                    status: format!("{:?}", report.status),
                    disposition,
                    reason_codes: vec![reason.to_owned()],
                });
            }
        }
    }
    if seen_metric_pairs != required_model_pairs
        || claim_strata_covered.len() != spec.population.required_strata.len()
    {
        reasons.insert("MODEL_METRIC_MAPPING_DERIVED_STRATA_SET_INCOMPLETE".to_owned());
    }

    let mut uncertainty_entries = Vec::new();
    let mut seen_uncertainty = BTreeSet::new();
    let required_uncertainty = model.required_uncertainty_components.iter().copied().collect::<BTreeSet<_>>();
    let uncertainty_obligation = match find_obligation(&profile, "model-uncertainty") {
        Some(value) => value,
        None => return reject("MODEL_OBLIGATION_UNCERTAINTY_REQUIREMENT_NOT_DERIVED"),
    };
    for entry in input.uncertainty_mappings {
        if entry.obligation_id != uncertainty_obligation.obligation_id
            || entry.requirement_source_ref != uncertainty_obligation.requirement_source_ref
            || entry.uncertainty_method_ref != spec.uncertainty.method_ref
            || !required_uncertainty.contains(&entry.component)
            || !seen_uncertainty.insert(entry.component)
        {
            reasons.insert("MODEL_UNCERTAINTY_MAPPING_SCOPE_OR_DUPLICATE_MISMATCH".to_owned());
            continue;
        }
        let component = model.uncertainty_components.iter().find(|item| item.component == entry.component);
        match component {
            None => {
                reasons.insert("MODEL_UNCERTAINTY_MAPPING_COMPONENT_MISSING".to_owned());
            }
            Some(component) => {
                if component.method_profile_sha256 != entry.method_profile_sha256
                    || component.report_sha256 != entry.report_sha256
                {
                    reasons.insert("MODEL_UNCERTAINTY_MAPPING_REPORT_MISMATCH".to_owned());
                }
                let (disposition, reason) = classify_uncertainty_status(component.status);
                uncertainty_entries.push(BoundModelUncertaintyV0_1 {
                    obligation_id: entry.obligation_id.clone(),
                    method_ref: spec.uncertainty.method_ref.clone(),
                    treatment: format!("{:?}", spec.uncertainty.treatment),
                    confidence_level_ppm: spec.uncertainty.confidence_level_ppm,
                    component: component.component.as_str().to_owned(),
                    method_profile_sha256: component.method_profile_sha256.clone(),
                    report_sha256: component.report_sha256.clone(),
                    status: format!("{:?}", component.status),
                    disposition,
                    reason_codes: vec![reason.to_owned()],
                });
            }
        }
    }
    if seen_uncertainty != required_uncertainty {
        reasons.insert("MODEL_UNCERTAINTY_MAPPING_REQUIRED_COMPONENT_SET_INCOMPLETE".to_owned());
    }

    let applicability_disposition = match model.domain_coverage_status {
        ModelDomainCoverageStatus::Covered => ModelObligationDispositionV0_1::Bound,
        ModelDomainCoverageStatus::NotCovered => ModelObligationDispositionV0_1::OutOfScope,
        ModelDomainCoverageStatus::Indeterminate => ModelObligationDispositionV0_1::Incomplete,
        ModelDomainCoverageStatus::Unverified => ModelObligationDispositionV0_1::Unverified,
    };
    let mut applicability_reasons = Vec::new();
    if applicability_disposition != ModelObligationDispositionV0_1::Bound {
        applicability_reasons.push(format!("MODEL_DOMAIN_COVERAGE_{}", model.domain_coverage_status.as_str().to_ascii_uppercase()));
    }
    let applicability_result = BoundModelApplicabilityV0_1 {
        obligation_id: applicability_obligation.obligation_id.clone(),
        model_ref: model.model_ref.clone(),
        model_artifact_sha256: model.model_artifact_sha256.clone(),
        solver_configuration_sha256: model.solver_configuration_sha256.clone(),
        operating_envelope_ref: model.operating_envelope_ref.clone(),
        applicability_profile_sha256: model.applicability_profile_sha256.clone(),
        risk_classification_sha256: model.risk_classification_sha256.clone(),
        domain_coverage_status: model.domain_coverage_status.as_str().to_owned(),
        disposition: applicability_disposition,
        reason_codes: applicability_reasons,
    };

    let mut reasons_from_model = model.reviewability_blockers();
    reasons_from_model.sort();
    reasons.extend(reasons_from_model);

    let applicability_sha = compute_model_applicability_mapping_sha256_v0_1(
        &spec.claim_id,
        &context_sha,
        &semantic_sha,
        &profile,
        &model_cut_sha,
        applicability_entry,
    )
    .unwrap_or_default();
    let receipt_errors = validate_model_receipt(
        input.mapping_receipt,
        evidence_record.owner_namespace.as_str(),
        spec,
        &profile,
        model,
        &model_cut_sha,
        &context_sha,
        &semantic_sha,
        &applicability_sha,
        &metric_sha,
        &uncertainty_sha,
        context.as_known_at_unix_ms,
    );
    reasons.extend(receipt_errors.iter().cloned());

    let mut unassessed = profile.obligations.iter().filter(|obligation| {
        !matches!(
            obligation.kind,
            ObligationKindV0_1::EvidenceIntegrity
                | ObligationKindV0_1::ModelCredibility
                | ObligationKindV0_1::ModelApplicability
        )
    }).map(|obligation| obligation.obligation_id.clone()).collect::<Vec<_>>();
    unassessed.sort();

    let mut dispositions = Vec::new();
    dispositions.push(applicability_disposition);
    dispositions.extend(metric_dispositions);
    dispositions.extend(uncertainty_entries.iter().map(|entry| entry.disposition));
    let has_rejecting_reason = reasons.iter().any(|reason| {
        reason.contains("MISMATCH")
            || reason.contains("INVALID")
            || reason.contains("DUPLICATE")
            || reason.contains("NOT_DERIVED")
            || reason.contains("FAILED")
            || reason.contains("REJECTED")
    });
    let has_unverified_reason = reasons.iter().any(|reason| {
        reason.contains("UNVERIFIED")
            || reason.contains("NOT_KNOWN")
            || reason.contains("UNKNOWN")
    });
    let overall = if has_rejecting_reason || dispositions.contains(&ModelObligationDispositionV0_1::Rejected) {
        ModelObligationDispositionV0_1::Rejected
    } else if dispositions.contains(&ModelObligationDispositionV0_1::OutOfScope) {
        ModelObligationDispositionV0_1::OutOfScope
    } else if has_unverified_reason || dispositions.contains(&ModelObligationDispositionV0_1::Unverified) {
        ModelObligationDispositionV0_1::Unverified
    } else if reasons.len() > 0 || dispositions.contains(&ModelObligationDispositionV0_1::Incomplete) {
        ModelObligationDispositionV0_1::Incomplete
    } else {
        ModelObligationDispositionV0_1::Bound
    };

    ClaimModelObligationBindingAssessmentV0_1 {
        disposition: overall,
        claim_id: spec.claim_id.clone(),
        configuration_ref: spec.configuration_ref.clone(),
        decision_context_sha256: context_sha,
        claim_specification_semantic_sha256: semantic_sha,
        obligation_profile_sha256: profile.profile_sha256,
        obligation_manifest_sha256: profile.obligation_manifest_sha256,
        model_evidence_ref: model.evidence_ref.clone(),
        model_evidence_record_sha256: evidence_record.evidence_record_sha256.clone(),
        model_credibility_cut_sha256: model_cut_sha,
        mapping_receipt_sha256: input.mapping_receipt.map(|r| r.receipt_sha256.clone()).unwrap_or_default(),
        applicability: Some(applicability_result),
        metric_strata: metric_entries,
        uncertainty_components: uncertainty_entries,
        unassessed_obligation_ids: unassessed,
        complete_claim_assurance_state: false,
        assurance_state_mutated: false,
        physical_truth_established: false,
        engineering_acceptance_granted: false,
        physical_work_authorized: false,
        reason_codes: reasons.into_iter().collect(),
    }
}
