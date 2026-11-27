// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compositional, non-compensatory claim-readiness gate for AERO v0.1.
//!
//! This module composes already-produced observer-coverage, causal-cut,
//! effectivity and required-evidence assessments. It does not authenticate
//! owner receipts, discover omitted sources, validate a physical model,
//! estimate risk, select actions, authorize work, qualify a structure, or
//! establish airworthiness. Its strongest result is reviewability by an
//! independent decision-maker under the supplied evidence contract.

use super::decision_context_v0_1::{
    DecisionContextV0_1, compute_required_evidence_manifest_sha256,
};
use super::effectivity_interval_v0_1::EffectivityRelation;
use super::model_credibility_cut_v0_1::{
    ModelCredibilityCutV0_1, ModelDomainCoverageStatus, ModelPackageStatus, SplitIntegrityStatus,
};
use super::observer_coverage_v0_1::{CoverageAssessment, CoverageStatus};
use super::owner_evidence_cut_v0_1::{
    OwnerDependencyCoverage, OwnerEvidenceCutV0_1, OwnerEvidenceFreshness, OwnerEvidenceLifecycle,
    OwnerObserverCoverageStatus,
};
use super::vector_watermark_v0_1::{CutAssessment, CutStatus};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Owner-supplied admission disposition for one required evidence reference.
///
/// ADMITTED is inherited from the upstream owner/evidence workflow. This gate
/// does not independently authenticate the receipt, signature, source bytes,
/// or the owner's authority to make that admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceAdmissionState {
    /// Upstream owner workflow marks the exact evidence reference admitted.
    Admitted,
    /// Evidence exists but its admission or trust remains unresolved.
    Unverified,
    /// Upstream owner workflow has rejected the evidence.
    Rejected,
    /// Required evidence has not been received.
    NotReceived,
}

impl EvidenceAdmissionState {
    /// Stable machine-readable disposition.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "ADMITTED",
            Self::Unverified => "UNVERIFIED",
            Self::Rejected => "REJECTED",
            Self::NotReceived => "NOT_RECEIVED",
        }
    }
}

/// Admission state for one mandatory evidence reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequiredEvidenceAdmission {
    /// Stable owner-native evidence reference; not a display-only label.
    pub evidence_ref: String,
    /// Decision-context digest this evidence reference is asserted to support.
    pub decision_context_sha256: String,
    /// Upstream evidence admission state.
    pub state: EvidenceAdmissionState,
}

/// Immutable identity binding for one independently evaluated component report.
///
/// The report digest is recomputed over the typed assessment and this exact
/// envelope. It is not an external signature or owner-receipt verification.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssessmentBinding {
    /// Claim identity the component report evaluates.
    pub claim_id: String,
    /// Exact physical article/subject reference.
    pub subject_ref: String,
    /// Digest of the canonical decision context shared by all components.
    pub decision_context_sha256: String,
    /// Digest of the exact component report.
    pub report_sha256: String,
    /// Exact evaluator profile identifier/version.
    pub evaluator_profile: String,
}

/// Three component reports that must be bound to one claim and decision context.
pub struct ClaimComponentBindings {
    /// Observer coverage assessment binding.
    pub coverage: AssessmentBinding,
    /// Causal-cut assessment binding.
    pub causal_cut: AssessmentBinding,
    /// Effectivity assessment binding.
    pub effectivity: AssessmentBinding,
}

/// Non-compensatory disposition for one engineering claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimGateStatus {
    /// Supplied obligations are satisfied for independent review only.
    ReviewableForIndependentDecision,
    /// Event evidence is not effective at the physical acquisition time.
    NotEffectiveAtSubjectTime,
    /// A known coverage or evidence gap prevents the requested inference.
    Incomplete,
    /// A required trust, identity, observer, or source fact is unresolved.
    Unverified,
    /// The requested claim is outside the qualified coverage scope.
    OutOfScope,
    /// A contradictory or malformed integrity assertion was found.
    Rejected,
}

impl ClaimGateStatus {
    /// Stable machine-readable disposition.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReviewableForIndependentDecision => "REVIEWABLE_FOR_INDEPENDENT_DECISION",
            Self::NotEffectiveAtSubjectTime => "NOT_EFFECTIVE_AT_SUBJECT_TIME",
            Self::Incomplete => "INCOMPLETE",
            Self::Unverified => "UNVERIFIED",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::Rejected => "REJECTED",
        }
    }
}

/// Exact inputs for a claim-level gate evaluation.
pub struct ClaimGateInput<'a> {
    /// Stable claim identity.
    pub claim_id: &'a str,
    /// Typed decision context; the gate derives its own commitment from these fields.
    pub decision_context: &'a DecisionContextV0_1,
    /// Exact owner-native Mycelix/AeroCommons and BASALT evidence cut.
    pub owner_evidence_cut: &'a OwnerEvidenceCutV0_1,
    /// Exact AeroWeft/model-credibility cut for this claim and domain.
    pub model_credibility_cut: &'a ModelCredibilityCutV0_1,
    /// Exact identity/context/report bindings for all three component reports.
    pub bindings: &'a ClaimComponentBindings,
    /// Result of the separately-owned observer-coverage evaluator.
    pub coverage: &'a CoverageAssessment,
    /// Result of the declared source/partition causal-cut evaluator.
    pub causal_cut: &'a CutAssessment,
    /// Result of comparing event-effective and physical-acquisition intervals.
    pub effectivity: EffectivityRelation,
    /// Every mandatory evidence reference required by this claim.
    pub required_evidence: &'a [RequiredEvidenceAdmission],
}

/// Compositional result. A positive result is reviewability, never approval.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimAssuranceAssessment {
    /// Stable claim identity.
    pub claim_id: String,
    /// Shared digest of the exact physical subject and decision context.
    pub decision_context_sha256: String,
    /// Digest of the exact typed owner-native evidence cut.
    pub owner_evidence_cut_sha256: String,
    /// Mycelix/AeroCommons packet identity and digest.
    pub owner_component_packet_id: String,
    pub owner_component_packet_sha256: String,
    /// BASALT run identity, report version and exact audit report/final-state digests.
    pub owner_basalt_run_id: String,
    pub owner_basalt_report_version: u32,
    pub owner_basalt_report_sha256: String,
    pub owner_basalt_final_state_sha256: String,
    pub owner_basalt_confirmatory_eligible: bool,
    /// Owner-declared artifact byte-verification disposition.
    pub owner_artifact_payloads_verified: bool,
    /// Exact owner receipt-manifest and pinned trust-policy commitments.
    pub owner_receiver_receipt_manifest_sha256: String,
    pub owner_trust_policy_sha256: String,
    /// Exact observer coverage receipt identity and owner disposition.
    pub owner_observer_coverage_profile_id: String,
    pub owner_observer_coverage_receipt_sha256: String,
    pub owner_observer_coverage_status: String,
    /// Selected receiver-evidence cutoff.
    pub owner_receiver_receipt_watermark_unix_ms: u64,
    /// Exact model-credibility evidence reference and typed report commitment.
    pub model_credibility_evidence_ref: String,
    pub model_credibility_sha256: String,
    pub model_ref: String,
    pub model_artifact_sha256: String,
    pub solver_configuration_sha256: String,
    pub model_applicability_profile_sha256: String,
    pub model_risk_classification_sha256: String,
    pub model_validation_package_sha256: String,
    pub model_validation_protocol_sha256: String,
    pub model_confirmatory_manifest_sha256: String,
    pub model_independent_reference_manifest_sha256: String,
    pub model_domain_coverage_status: String,
    pub model_split_integrity_status: String,
    /// Component report digests in coverage, causal-cut, effectivity order.
    pub component_report_sha256: [String; 3],
    /// Sorted mandatory evidence references included in this assessment.
    pub required_evidence_refs: Vec<String>,
    /// Non-compensatory claim disposition.
    pub status: ClaimGateStatus,
    /// Stable, sorted, unique reason codes from all supplied components.
    pub reason_codes: Vec<String>,
    /// Count of mandatory evidence references marked ADMITTED upstream.
    pub admitted_required_evidence_count: usize,
    /// Count of mandatory evidence references in the input.
    pub required_evidence_count: usize,
    /// This profile never grants execution or operational authority.
    pub authority_granted: bool,
    /// This profile never claims physical qualification.
    pub physical_qualification_status: &'static str,
    /// This profile never infers universal absence from this composition.
    pub absence_inference_granted: bool,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[derive(Serialize)]
struct BoundComponentReport<'a, T: Serialize> {
    schema: &'static str,
    component_kind: &'static str,
    claim_id: &'a str,
    subject_ref: &'a str,
    decision_context_sha256: &'a str,
    evaluator_profile: &'a str,
    assessment: &'a T,
}

/// Compute the deterministic digest of one typed assessment and its exact
/// claim/subject/context envelope.
///
/// Struct field order is fixed by this versioned Rust type; assessment
/// collections are already canonically ordered by their evaluators. This is a
/// versioned Rust/Serde JSON profile, not a general RFC 8785 implementation.
pub fn compute_component_report_sha256<T: Serialize>(
    component_kind: &'static str,
    binding: &AssessmentBinding,
    assessment: &T,
) -> Result<String, String> {
    let envelope = BoundComponentReport {
        schema: "AeroClaimComponentReportV1",
        component_kind,
        claim_id: &binding.claim_id,
        subject_ref: &binding.subject_ref,
        decision_context_sha256: &binding.decision_context_sha256,
        evaluator_profile: &binding.evaluator_profile,
        assessment,
    };
    let bytes = serde_json::to_vec(&envelope)
        .map_err(|_| "COMPONENT_REPORT_SERIALIZATION_FAILED".to_owned())?;
    let digest = Sha256::digest(bytes);
    let digest_hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("");
    Ok(digest_hex)
}

struct ComponentBindingExpectation<'a> {
    claim_id: &'a str,
    subject_ref: &'a str,
    expected_context_sha256: &'a str,
    coverage: &'a CoverageAssessment,
    causal_cut: &'a CutAssessment,
    effectivity: &'a EffectivityRelation,
}

/// Validate that the three typed component results refer to exactly the same
/// claim, physical subject and canonical decision context. Each supplied
/// report digest must match a fresh digest of its actual typed assessment.
fn validate_component_bindings(
    expected: ComponentBindingExpectation<'_>,
    bindings: &ClaimComponentBindings,
    reasons: &mut BTreeSet<String>,
) -> bool {
    let ComponentBindingExpectation {
        claim_id,
        subject_ref,
        expected_context_sha256,
        coverage,
        causal_cut,
        effectivity,
    } = expected;
    let components = [
        ("COVERAGE", "OBSERVER_COVERAGE", &bindings.coverage),
        ("CAUSAL_CUT", "CAUSAL_CUT", &bindings.causal_cut),
        ("EFFECTIVITY", "EFFECTIVITY", &bindings.effectivity),
    ];
    let mut report_digests = BTreeSet::new();
    let mut valid = true;

    for (kind, component_kind, binding) in components {
        if binding.claim_id != claim_id {
            valid = false;
            reasons.insert(format!("{kind}_CLAIM_ID_MISMATCH"));
        }
        if binding.subject_ref.trim().is_empty()
            || binding.subject_ref != binding.subject_ref.trim()
        {
            valid = false;
            reasons.insert(format!("{kind}_SUBJECT_REF_INVALID"));
        }
        if binding.subject_ref != subject_ref {
            valid = false;
            reasons.insert("COMPONENT_SUBJECT_BINDING_MISMATCH".to_owned());
        }
        if !valid_sha256(&binding.decision_context_sha256) {
            valid = false;
            reasons.insert(format!("{kind}_DECISION_CONTEXT_DIGEST_INVALID"));
        }
        if binding.decision_context_sha256 != expected_context_sha256 {
            valid = false;
            reasons.insert("COMPONENT_DECISION_CONTEXT_MISMATCH".to_owned());
        }
        if !valid_sha256(&binding.report_sha256) {
            valid = false;
            reasons.insert(format!("{kind}_REPORT_DIGEST_INVALID"));
        }
        let recomputed_digest = match kind {
            "COVERAGE" => compute_component_report_sha256(component_kind, binding, coverage),
            "CAUSAL_CUT" => compute_component_report_sha256(component_kind, binding, causal_cut),
            "EFFECTIVITY" => compute_component_report_sha256(component_kind, binding, effectivity),
            _ => unreachable!("closed component set"),
        };
        match recomputed_digest {
            Ok(expected) if expected == binding.report_sha256 => {}
            Ok(_) => {
                valid = false;
                reasons.insert(format!("{kind}_REPORT_DIGEST_MISMATCH"));
            }
            Err(reason) => {
                valid = false;
                reasons.insert(reason);
            }
        }
        if !report_digests.insert(binding.report_sha256.as_str()) {
            valid = false;
            reasons.insert("COMPONENT_REPORT_DIGEST_REUSED".to_owned());
        }
        if binding.evaluator_profile.trim().is_empty()
            || binding.evaluator_profile != binding.evaluator_profile.trim()
        {
            valid = false;
            reasons.insert(format!("{kind}_EVALUATOR_PROFILE_INVALID"));
        }
    }

    valid
}

/// Validate the typed owner-native evidence cut and propagate its blockers.
///
/// The cut is a commitment to upstream declarations, not authentication of
/// the underlying Mycelix/BASALT bytes or signatures.
fn validate_owner_evidence_cut(
    input: &ClaimGateInput<'_>,
    expected_context_sha256: &str,
    reasons: &mut BTreeSet<String>,
    unverified: &mut bool,
    incomplete: &mut bool,
    out_of_scope: &mut bool,
    invalid: &mut bool,
) {
    let cut = input.owner_evidence_cut;
    match cut.compute_sha256() {
        Ok(actual) if actual == input.decision_context.evidence_knowledge_cut_sha256 => {}
        Ok(_) => {
            *invalid = true;
            reasons.insert("OWNER_EVIDENCE_CUT_DIGEST_MISMATCH".to_owned());
        }
        Err(errors) => {
            *invalid = true;
            reasons.extend(errors);
        }
    }

    if cut.trust_policy_sha256 != input.decision_context.trust_policy_sha256 {
        *invalid = true;
        reasons.insert("OWNER_CUT_TRUST_POLICY_MISMATCH".to_owned());
    }
    if cut.physical_unit_ref != input.decision_context.subject_ref {
        *invalid = true;
        reasons.insert("OWNER_CUT_PHYSICAL_UNIT_MISMATCH".to_owned());
    }
    if cut.configuration_ref != input.decision_context.configuration_ref {
        *invalid = true;
        reasons.insert("OWNER_CUT_CONFIGURATION_MISMATCH".to_owned());
    }
    if cut.receiver_receipt_watermark_unix_ms > input.decision_context.as_known_at_unix_ms {
        *invalid = true;
        reasons.insert("OWNER_CUT_RECEIPT_WATERMARK_AFTER_CONTEXT_SNAPSHOT".to_owned());
    }
    if cut.observer_coverage_watermark_unix_ms > cut.receiver_receipt_watermark_unix_ms {
        *invalid = true;
        reasons.insert("OWNER_CUT_OBSERVER_WATERMARK_AFTER_RECEIPT_CUT".to_owned());
    }

    if !cut.artifact_payloads_verified {
        *unverified = true;
        reasons.insert("MYCELIX_ARTIFACT_PAYLOADS_NOT_VERIFIED".to_owned());
    }
    if !cut.basalt_confirmatory_eligible {
        *unverified = true;
        reasons.insert("BASALT_CONFIRMATORY_ELIGIBILITY_NOT_ESTABLISHED".to_owned());
    }
    match cut.observer_coverage_status {
        OwnerObserverCoverageStatus::QualifiedCompleteForScope => {}
        OwnerObserverCoverageStatus::Incomplete => {
            *incomplete = true;
            reasons.insert("BASALT_OBSERVER_COVERAGE_INCOMPLETE".to_owned());
        }
        OwnerObserverCoverageStatus::Unverified => {
            *unverified = true;
            reasons.insert("BASALT_OBSERVER_COVERAGE_UNVERIFIED".to_owned());
        }
        OwnerObserverCoverageStatus::OutOfScope => {
            *out_of_scope = true;
            reasons.insert("BASALT_OBSERVER_COVERAGE_OUT_OF_SCOPE".to_owned());
        }
    }

    for required in input.required_evidence {
        let Some(record) = cut.find_evidence(&required.evidence_ref) else {
            *incomplete = true;
            reasons.insert(format!(
                "OWNER_EVIDENCE_REFERENCE_NOT_IN_CUT:{}",
                required.evidence_ref
            ));
            continue;
        };
        if record.lifecycle != OwnerEvidenceLifecycle::Active {
            *unverified = true;
            reasons.insert(format!(
                "AEROCOMMONS_EVIDENCE_LIFECYCLE_NOT_ACTIVE:{}",
                required.evidence_ref
            ));
        }
        match record.freshness {
            OwnerEvidenceFreshness::Fresh => {}
            OwnerEvidenceFreshness::StaleDirect => {
                *incomplete = true;
                reasons.insert(format!(
                    "BASALT_EVIDENCE_STALE_DIRECT:{}",
                    required.evidence_ref
                ));
            }
            OwnerEvidenceFreshness::StaleTransitive => {
                *incomplete = true;
                reasons.insert(format!(
                    "BASALT_EVIDENCE_STALE_TRANSITIVE:{}",
                    required.evidence_ref
                ));
            }
        }
        if record.dependency_coverage == OwnerDependencyCoverage::Incomplete {
            *incomplete = true;
            reasons.insert(format!(
                "BASALT_DEPENDENCY_COVERAGE_INCOMPLETE:{}",
                required.evidence_ref
            ));
        }
        if record.requires_reverification {
            *incomplete = true;
            reasons.insert(format!(
                "BASALT_EVIDENCE_REVERIFICATION_REQUIRED:{}",
                required.evidence_ref
            ));
        }
        if !record.eligible_under_declared_coverage {
            *incomplete = true;
            reasons.insert(format!(
                "BASALT_EVIDENCE_NOT_ELIGIBLE_UNDER_DECLARED_COVERAGE:{}",
                required.evidence_ref
            ));
        }
    }

    // The context's evidence-knowledge-cut digest is a derived commitment; the
    // separate typed context digest must also bind the evidence cut itself.
    if !valid_sha256(expected_context_sha256) {
        *invalid = true;
        reasons.insert("DECISION_CONTEXT_DIGEST_INVALID_FOR_OWNER_CUT".to_owned());
    }
}

/// Validate AeroWeft's typed model-credibility cut against the exact claim context
/// and owner evidence cut. This composes model-owner outputs; it does not
/// calculate VVUQ or independently authenticate the referenced reports.
fn validate_model_credibility_cut(
    input: &ClaimGateInput<'_>,
    reasons: &mut BTreeSet<String>,
    unverified: &mut bool,
    incomplete: &mut bool,
    out_of_scope: &mut bool,
    invalid: &mut bool,
) {
    let model = input.model_credibility_cut;
    match model.compute_sha256() {
        Ok(digest) => {
            match input.owner_evidence_cut.find_evidence(&model.evidence_ref) {
                Some(record) => {
                    if record.record_sha256 != digest {
                        *invalid = true;
                        reasons.insert("MODEL_CREDIBILITY_OWNER_RECORD_DIGEST_MISMATCH".to_owned());
                    }
                    if record.subject_ref != input.decision_context.subject_ref
                        || record.configuration_ref != model.configuration_ref
                    {
                        *invalid = true;
                        reasons.insert(
                            "MODEL_CREDIBILITY_OWNER_RECORD_MODEL_IDENTITY_MISMATCH".to_owned(),
                        );
                    }
                }
                None => {
                    *incomplete = true;
                    reasons.insert("MODEL_CREDIBILITY_OWNER_RECORD_MISSING".to_owned());
                }
            }
            if !input
                .required_evidence
                .iter()
                .any(|item| item.evidence_ref == model.evidence_ref)
            {
                *incomplete = true;
                reasons.insert("MODEL_CREDIBILITY_REF_NOT_IN_REQUIRED_EVIDENCE".to_owned());
            }
        }
        Err(errors) => {
            *invalid = true;
            reasons.extend(errors);
        }
    }

    for (reason, condition) in [
        (
            "MODEL_CREDIBILITY_CLAIM_ID_MISMATCH",
            model.claim_id != input.decision_context.claim_id,
        ),
        (
            "MODEL_CREDIBILITY_CONFIGURATION_MISMATCH",
            model.configuration_ref != input.decision_context.configuration_ref,
        ),
        (
            "MODEL_CREDIBILITY_OPERATING_ENVELOPE_MISMATCH",
            model.operating_envelope_ref != input.decision_context.operating_envelope_ref,
        ),
        (
            "MODEL_CREDIBILITY_APPLICABILITY_PROFILE_MISMATCH",
            model.applicability_profile_sha256
                != input.decision_context.model_applicability_profile_sha256,
        ),
        (
            "MODEL_CREDIBILITY_RISK_CLASSIFICATION_MISMATCH",
            model.risk_classification_sha256 != input.decision_context.risk_classification_sha256,
        ),
    ] {
        if condition {
            *invalid = true;
            reasons.insert(reason.to_owned());
        }
    }

    match model.package_status {
        ModelPackageStatus::StructurallyComplete => {}
        ModelPackageStatus::Incomplete => {
            *incomplete = true;
            reasons.insert("MODEL_VALIDATION_PACKAGE_INCOMPLETE".to_owned());
        }
        ModelPackageStatus::Rejected => {
            *invalid = true;
            reasons.insert("MODEL_VALIDATION_PACKAGE_REJECTED".to_owned());
        }
    }
    match model.split_integrity_status {
        SplitIntegrityStatus::DisjointByIndependentUnit => {}
        SplitIntegrityStatus::LeakageDetected => {
            *invalid = true;
            reasons.insert("MODEL_CONFIRMATORY_SPLIT_LEAKAGE".to_owned());
        }
        SplitIntegrityStatus::Unverified => {
            *unverified = true;
            reasons.insert("MODEL_CONFIRMATORY_SPLIT_UNVERIFIED".to_owned());
        }
    }
    match model.domain_coverage_status {
        ModelDomainCoverageStatus::Covered => {}
        ModelDomainCoverageStatus::NotCovered => {
            *out_of_scope = true;
            reasons.insert("MODEL_CLAIM_OUTSIDE_VALIDATED_DOMAIN".to_owned());
        }
        ModelDomainCoverageStatus::Indeterminate => {
            *incomplete = true;
            reasons.insert("MODEL_DOMAIN_COVERAGE_INDETERMINATE".to_owned());
        }
        ModelDomainCoverageStatus::Unverified => {
            *unverified = true;
            reasons.insert("MODEL_DOMAIN_COVERAGE_UNVERIFIED".to_owned());
        }
    }

    for blocker in model.reviewability_blockers() {
        if blocker == "MODEL_CLAIM_OUTSIDE_VALIDATED_DOMAIN"
            || blocker.starts_with("MODEL_METRIC_OUTSIDE_DECLARED_PROFILE:")
        {
            *out_of_scope = true;
        } else if blocker == "MODEL_VALIDATION_PACKAGE_REJECTED"
            || blocker == "MODEL_CONFIRMATORY_SPLIT_LEAKAGE"
        {
            *invalid = true;
        } else if blocker.starts_with("MODEL_METRIC_STRATUM_INDETERMINATE:")
            || blocker.starts_with("MODEL_METRIC_STRATUM_REPORT_MISSING:")
            || blocker == "MODEL_VALIDATION_PACKAGE_INCOMPLETE"
        {
            *incomplete = true;
        } else {
            *unverified = true;
        }
        reasons.insert(blocker);
    }
}

fn component_reasons(prefix: &str, reasons: &[String], target: &mut BTreeSet<String>) {
    for reason in reasons {
        target.insert(format!("{prefix}_{reason}"));
    }
}

/// Evaluate one claim with strict, non-compensatory precedence.
///
/// Integrity contradictions reject. Scope limits are preserved. Unknown
/// trust/identity facts remain UNVERIFIED. Known missing coverage or evidence
/// abstains. An event definitely after the acquisition is not applied to that
/// subject-time claim. Only fully satisfied supplied obligations permit
/// independent review; no numeric score or majority vote can compensate for
/// one failed mandatory condition.
pub fn evaluate_claim_gate(input: ClaimGateInput<'_>) -> ClaimAssuranceAssessment {
    let mut reasons = BTreeSet::new();
    let mut invalid = false;
    let mut out_of_scope = false;
    let mut unverified = false;
    let mut incomplete = false;
    let mut not_yet_effective = false;

    if input.claim_id.trim().is_empty() || input.claim_id != input.claim_id.trim() {
        invalid = true;
        reasons.insert("CLAIM_ID_MISSING_OR_NONCANONICAL".to_owned());
    }

    let decision_context_sha256 = match input.decision_context.compute_sha256() {
        Ok(digest) => digest,
        Err(context_errors) => {
            invalid = true;
            reasons.extend(context_errors);
            String::new()
        }
    };
    if input.decision_context.claim_id != input.claim_id {
        invalid = true;
        reasons.insert("DECISION_CONTEXT_CLAIM_ID_MISMATCH".to_owned());
    }
    if !validate_component_bindings(
        ComponentBindingExpectation {
            claim_id: input.claim_id,
            subject_ref: &input.decision_context.subject_ref,
            expected_context_sha256: &decision_context_sha256,
            coverage: input.coverage,
            causal_cut: input.causal_cut,
            effectivity: &input.effectivity,
        },
        input.bindings,
        &mut reasons,
    ) {
        invalid = true;
    }

    validate_owner_evidence_cut(
        &input,
        &decision_context_sha256,
        &mut reasons,
        &mut unverified,
        &mut incomplete,
        &mut out_of_scope,
        &mut invalid,
    );
    validate_model_credibility_cut(
        &input,
        &mut reasons,
        &mut unverified,
        &mut incomplete,
        &mut out_of_scope,
        &mut invalid,
    );

    let evidence_refs_for_manifest = input
        .required_evidence
        .iter()
        .map(|evidence| evidence.evidence_ref.clone())
        .collect::<Vec<_>>();
    match compute_required_evidence_manifest_sha256(&evidence_refs_for_manifest) {
        Ok(actual_manifest) => {
            if actual_manifest != input.decision_context.required_evidence_manifest_sha256 {
                invalid = true;
                reasons.insert("REQUIRED_EVIDENCE_MANIFEST_MISMATCH".to_owned());
            }
        }
        Err(reason) => {
            invalid = true;
            reasons.insert(reason);
        }
    }

    component_reasons("COVERAGE", &input.coverage.reason_codes, &mut reasons);
    component_reasons("CAUSAL_CUT", &input.causal_cut.reason_codes, &mut reasons);

    match input.coverage.status {
        CoverageStatus::QualifiedCompleteForScope => {}
        CoverageStatus::Incomplete => incomplete = true,
        CoverageStatus::Unverified => unverified = true,
        CoverageStatus::Invalid => invalid = true,
        CoverageStatus::OutOfScope => out_of_scope = true,
    }

    match input.causal_cut.status {
        CutStatus::Closed => {}
        CutStatus::Incomplete => incomplete = true,
        CutStatus::Unverified => unverified = true,
        CutStatus::Invalid => invalid = true,
    }

    match input.effectivity {
        EffectivityRelation::DefinitelyEffective => {
            reasons.insert("EFFECTIVE_FOR_ALL_ACQUISITION_TIMES".to_owned());
        }
        EffectivityRelation::DefinitelyNotYetEffective => {
            not_yet_effective = true;
            reasons.insert("EVENT_NOT_YET_EFFECTIVE_AT_ACQUISITION".to_owned());
        }
        EffectivityRelation::BoundaryAmbiguous => {
            incomplete = true;
            reasons.insert("EFFECTIVITY_BOUNDARY_AMBIGUOUS".to_owned());
        }
        EffectivityRelation::InvalidInterval => {
            invalid = true;
            reasons.insert("EFFECTIVITY_INTERVAL_INVALID".to_owned());
        }
    }

    let mut seen_refs = BTreeSet::new();
    let mut admitted_count = 0usize;
    for evidence in input.required_evidence {
        if evidence.evidence_ref.trim().is_empty()
            || evidence.evidence_ref != evidence.evidence_ref.trim()
        {
            invalid = true;
            reasons.insert("REQUIRED_EVIDENCE_REF_INVALID".to_owned());
            continue;
        }
        if !seen_refs.insert(evidence.evidence_ref.as_str()) {
            invalid = true;
            reasons.insert("DUPLICATE_REQUIRED_EVIDENCE_REF".to_owned());
            continue;
        }
        if !valid_sha256(&evidence.decision_context_sha256) {
            invalid = true;
            reasons.insert("REQUIRED_EVIDENCE_CONTEXT_DIGEST_INVALID".to_owned());
        } else if evidence.decision_context_sha256 != decision_context_sha256 {
            invalid = true;
            reasons.insert("REQUIRED_EVIDENCE_CONTEXT_MISMATCH".to_owned());
        }

        match evidence.state {
            EvidenceAdmissionState::Admitted => admitted_count += 1,
            EvidenceAdmissionState::Unverified => {
                unverified = true;
                reasons.insert("REQUIRED_EVIDENCE_UNVERIFIED".to_owned());
            }
            EvidenceAdmissionState::Rejected => {
                invalid = true;
                reasons.insert("REQUIRED_EVIDENCE_REJECTED".to_owned());
            }
            EvidenceAdmissionState::NotReceived => {
                incomplete = true;
                reasons.insert("REQUIRED_EVIDENCE_NOT_RECEIVED".to_owned());
            }
        }
    }

    if input.required_evidence.is_empty() {
        unverified = true;
        reasons.insert("REQUIRED_EVIDENCE_SET_EMPTY".to_owned());
    }

    let owner_evidence_cut_sha256 = input
        .owner_evidence_cut
        .compute_sha256()
        .unwrap_or_default();

    let status = if invalid {
        ClaimGateStatus::Rejected
    } else if out_of_scope {
        ClaimGateStatus::OutOfScope
    } else if unverified {
        ClaimGateStatus::Unverified
    } else if incomplete {
        ClaimGateStatus::Incomplete
    } else if not_yet_effective {
        ClaimGateStatus::NotEffectiveAtSubjectTime
    } else {
        reasons.insert("SUPPLIED_OBLIGATIONS_SATISFIED_FOR_REVIEW_ONLY".to_owned());
        ClaimGateStatus::ReviewableForIndependentDecision
    };

    let required_evidence_refs = seen_refs.into_iter().map(str::to_owned).collect();
    let model_credibility_sha256 = input
        .model_credibility_cut
        .compute_sha256()
        .unwrap_or_default();
    ClaimAssuranceAssessment {
        claim_id: input.claim_id.to_owned(),
        decision_context_sha256,
        model_credibility_evidence_ref: input.model_credibility_cut.evidence_ref.clone(),
        model_credibility_sha256,
        model_ref: input.model_credibility_cut.model_ref.clone(),
        model_artifact_sha256: input.model_credibility_cut.model_artifact_sha256.clone(),
        solver_configuration_sha256: input
            .model_credibility_cut
            .solver_configuration_sha256
            .clone(),
        model_applicability_profile_sha256: input
            .model_credibility_cut
            .applicability_profile_sha256
            .clone(),
        model_risk_classification_sha256: input
            .model_credibility_cut
            .risk_classification_sha256
            .clone(),
        model_validation_package_sha256: input
            .model_credibility_cut
            .validation_package_sha256
            .clone(),
        model_validation_protocol_sha256: input
            .model_credibility_cut
            .validation_protocol_sha256
            .clone(),
        model_confirmatory_manifest_sha256: input
            .model_credibility_cut
            .confirmatory_manifest_sha256
            .clone(),
        model_independent_reference_manifest_sha256: input
            .model_credibility_cut
            .independent_reference_manifest_sha256
            .clone(),
        model_domain_coverage_status: input
            .model_credibility_cut
            .domain_coverage_status
            .as_str()
            .to_owned(),
        model_split_integrity_status: input
            .model_credibility_cut
            .split_integrity_status
            .as_str()
            .to_owned(),
        owner_evidence_cut_sha256,
        owner_component_packet_id: input.owner_evidence_cut.component_packet_id.clone(),
        owner_component_packet_sha256: input.owner_evidence_cut.component_packet_sha256.clone(),
        owner_basalt_run_id: input.owner_evidence_cut.basalt_run_id.clone(),
        owner_basalt_report_version: input.owner_evidence_cut.basalt_report_version,
        owner_basalt_report_sha256: input.owner_evidence_cut.basalt_report_sha256.clone(),
        owner_basalt_final_state_sha256: input.owner_evidence_cut.basalt_final_state_sha256.clone(),
        owner_basalt_confirmatory_eligible: input.owner_evidence_cut.basalt_confirmatory_eligible,
        owner_artifact_payloads_verified: input.owner_evidence_cut.artifact_payloads_verified,
        owner_receiver_receipt_manifest_sha256: input
            .owner_evidence_cut
            .receiver_receipt_manifest_sha256
            .clone(),
        owner_trust_policy_sha256: input.owner_evidence_cut.trust_policy_sha256.clone(),
        owner_observer_coverage_profile_id: input
            .owner_evidence_cut
            .observer_coverage_profile_id
            .clone(),
        owner_observer_coverage_receipt_sha256: input
            .owner_evidence_cut
            .observer_coverage_receipt_sha256
            .clone(),
        owner_observer_coverage_status: input
            .owner_evidence_cut
            .observer_coverage_status
            .as_str()
            .to_owned(),
        owner_receiver_receipt_watermark_unix_ms: input
            .owner_evidence_cut
            .receiver_receipt_watermark_unix_ms,
        component_report_sha256: [
            input.bindings.coverage.report_sha256.clone(),
            input.bindings.causal_cut.report_sha256.clone(),
            input.bindings.effectivity.report_sha256.clone(),
        ],
        required_evidence_refs,
        status,
        reason_codes: reasons.into_iter().collect(),
        admitted_required_evidence_count: admitted_count,
        required_evidence_count: input.required_evidence.len(),
        authority_granted: false,
        physical_qualification_status: "OUT_OF_SCOPE_NOT_CLAIMED",
        absence_inference_granted: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AssessmentBinding, ClaimComponentBindings, ClaimGateInput, ClaimGateStatus,
        EvidenceAdmissionState, RequiredEvidenceAdmission, compute_component_report_sha256,
        evaluate_claim_gate,
    };
    use crate::decision_context_v0_1::DecisionContextV0_1;
    use crate::decision_context_v0_1::compute_required_evidence_manifest_sha256;
    use crate::effectivity_interval_v0_1::EffectivityRelation;
    use crate::model_credibility_cut_v0_1::{
        ModelCredibilityCutV0_1, ModelDomainCoverageStatus, SplitIntegrityStatus,
    };
    use crate::observer_coverage_v0_1::{CoverageAssessment, CoverageStatus, ReplayConsequence};
    use crate::owner_evidence_cut_v0_1::{
        OwnerDependencyCoverage, OwnerEvidenceCutV0_1, OwnerEvidenceFreshness,
        OwnerEvidenceLifecycle, OwnerEvidenceRecordV0_1, OwnerObserverCoverageStatus,
    };
    use crate::vector_watermark_v0_1::{CutAssessment, CutStatus};

    fn coverage(status: CoverageStatus) -> CoverageAssessment {
        let (replay_consequence, reason_codes) = match status {
            CoverageStatus::QualifiedCompleteForScope => {
                (ReplayConsequence::AllowScopedAbsenceInference, vec![])
            }
            CoverageStatus::Incomplete => (
                ReplayConsequence::Abstain,
                vec!["SOURCE_ROSTER_NOT_RECONCILED".to_owned()],
            ),
            CoverageStatus::Unverified => (
                ReplayConsequence::TrustUnresolved,
                vec!["OBSERVER_NOT_INDEPENDENTLY_QUALIFIED".to_owned()],
            ),
            CoverageStatus::Invalid => (
                ReplayConsequence::Reject,
                vec!["CHECKPOINT_INTEGRITY_INVALID".to_owned()],
            ),
            CoverageStatus::OutOfScope => (
                ReplayConsequence::Abstain,
                vec!["CLAIM_OUTSIDE_COVERAGE_SCOPE".to_owned()],
            ),
        };
        CoverageAssessment {
            status,
            replay_consequence,
            reason_codes,
        }
    }

    fn cut(status: CutStatus) -> CutAssessment {
        let reason_codes = match status {
            CutStatus::Closed => vec!["CAUSAL_CUT_CLOSED".to_owned()],
            CutStatus::Incomplete => vec!["CAUSAL_PREDECESSOR_MISSING".to_owned()],
            CutStatus::Unverified => vec!["EVIDENCE_WATERMARK_VECTOR_EMPTY".to_owned()],
            CutStatus::Invalid => vec!["CAUSAL_DEPENDENCY_CYCLE".to_owned()],
        };
        CutAssessment {
            status,
            reason_codes,
            watermark_count: 2,
            event_count: 3,
            dependency_count: 2,
        }
    }

    fn owner_record(evidence_ref: &str) -> OwnerEvidenceRecordV0_1 {
        OwnerEvidenceRecordV0_1 {
            evidence_ref: evidence_ref.to_owned(),
            record_sha256: "1".repeat(64),
            subject_ref: "instrument:caliper-01".to_owned(),
            configuration_ref: "configuration:wing-r17".to_owned(),
            lifecycle: OwnerEvidenceLifecycle::Active,
            predecessor_ref: None,
            freshness: OwnerEvidenceFreshness::Fresh,
            dependency_coverage: OwnerDependencyCoverage::Complete,
            unresolved_inputs: vec![],
            requires_reverification: false,
            eligible_under_declared_coverage: true,
        }
    }

    fn context_template(evidence_refs: &[String]) -> DecisionContextV0_1 {
        DecisionContextV0_1 {
            claim_id: "claim-wing-panel-001".to_owned(),
            subject_ref: "part:wing-panel-001".to_owned(),
            configuration_ref: "configuration:wing-r17".to_owned(),
            operating_envelope_ref: "envelope:transport-normal".to_owned(),
            acquisition_time_earliest_unix_ms: 1_795_514_399_900,
            acquisition_time_latest_unix_ms: 1_795_514_400_100,
            as_known_at_unix_ms: 1_795_516_800_000,
            source_scope_manifest_sha256: "a".repeat(64),
            evidence_knowledge_cut_sha256: "b".repeat(64),
            trust_policy_sha256: "c".repeat(64),
            claim_specification_sha256: "d".repeat(64),
            required_evidence_manifest_sha256: compute_required_evidence_manifest_sha256(
                evidence_refs,
            )
            .expect("evidence manifest serializes"),
            risk_classification_sha256: "e".repeat(64),
            model_applicability_profile_sha256: "f".repeat(64),
        }
    }

    fn model_cut_for_context(context: &DecisionContextV0_1) -> ModelCredibilityCutV0_1 {
        let mut model = crate::model_credibility_cut_v0_1::tests::fixture();
        model.claim_id = context.claim_id.clone();
        model.configuration_ref = context.configuration_ref.clone();
        model.operating_envelope_ref = context.operating_envelope_ref.clone();
        model.applicability_profile_sha256 = context.model_applicability_profile_sha256.clone();
        model.risk_classification_sha256 = context.risk_classification_sha256.clone();
        model
    }

    fn owner_cut_for_refs(
        evidence_refs: &[String],
        trust_policy_sha256: &str,
    ) -> OwnerEvidenceCutV0_1 {
        let model = model_cut_for_context(&context_template(evidence_refs));
        let mut refs = evidence_refs
            .iter()
            .filter(|reference| {
                !reference.trim().is_empty() && reference.as_str() == reference.trim()
            })
            .cloned()
            .collect::<Vec<_>>();
        refs.sort();
        refs.dedup();
        if !refs.iter().any(|item| item == &model.evidence_ref) {
            refs.push(model.evidence_ref.clone());
        }
        let mut cut = OwnerEvidenceCutV0_1 {
            schema: "AeroOwnerEvidenceCutV0_1".to_owned(),
            owner_namespace: "mycelix:aerocommons".to_owned(),
            component_packet_id: "packet:coupon-001-r1".to_owned(),
            physical_unit_ref: "part:wing-panel-001".to_owned(),
            configuration_ref: "configuration:wing-r17".to_owned(),
            component_packet_sha256: "2".repeat(64),
            component_packet_byte_length: 4096,
            artifact_payloads_verified: true,
            evidence_records: refs
                .iter()
                .map(|reference| owner_record(reference))
                .collect(),
            basalt_run_id: "basalt:run-001".to_owned(),
            basalt_report_version: 1,
            basalt_timeline_input_sha256: "3".repeat(64),
            basalt_final_state_sha256: "4".repeat(64),
            basalt_report_sha256: "5".repeat(64),
            basalt_confirmatory_eligible: true,
            receiver_receipt_manifest_sha256: "6".repeat(64),
            receiver_receipt_watermark_unix_ms: 1_795_516_700_000,
            trust_policy_sha256: trust_policy_sha256.to_owned(),
            observer_coverage_profile_id: "basalt-coverage-v0.1".to_owned(),
            observer_coverage_receipt_sha256: "7".repeat(64),
            observer_coverage_status: OwnerObserverCoverageStatus::QualifiedCompleteForScope,
            observer_coverage_watermark_unix_ms: 1_795_516_600_000,
        };
        let model_record = cut
            .evidence_records
            .iter_mut()
            .find(|record| record.evidence_ref == model.evidence_ref)
            .expect("model evidence reference is included in owner cut");
        model_record.record_sha256 = model.compute_sha256().expect("model cut serializes");
        cut
    }

    fn decision_context_for_refs(evidence_refs: &[String]) -> DecisionContextV0_1 {
        let owner_cut = owner_cut_for_refs(evidence_refs, &"c".repeat(64));
        let mut context = context_template(evidence_refs);
        context.evidence_knowledge_cut_sha256 = owner_cut
            .compute_sha256()
            .expect("owner evidence cut serializes");
        context
    }

    fn owner_cut_for_evidence(evidence: &[RequiredEvidenceAdmission]) -> OwnerEvidenceCutV0_1 {
        let refs = evidence
            .iter()
            .map(|item| item.evidence_ref.clone())
            .collect::<Vec<_>>();
        owner_cut_for_refs(&refs, &"c".repeat(64))
    }

    fn context_for_evidence(evidence: &[RequiredEvidenceAdmission]) -> DecisionContextV0_1 {
        let refs = evidence
            .iter()
            .map(|item| item.evidence_ref.clone())
            .collect::<Vec<_>>();
        decision_context_for_refs(&refs)
    }

    fn context_digest(context: &DecisionContextV0_1) -> String {
        context
            .compute_sha256()
            .expect("test decision context is valid")
    }

    fn evidence(state: EvidenceAdmissionState, evidence_ref: &str) -> RequiredEvidenceAdmission {
        let context = decision_context_for_refs(&[evidence_ref.to_owned()]);
        RequiredEvidenceAdmission {
            evidence_ref: evidence_ref.to_owned(),
            decision_context_sha256: context_digest(&context),
            state,
        }
    }

    fn component_binding(profile: &str, context: &DecisionContextV0_1) -> AssessmentBinding {
        AssessmentBinding {
            claim_id: context.claim_id.clone(),
            subject_ref: context.subject_ref.clone(),
            decision_context_sha256: context_digest(context),
            report_sha256: String::new(),
            evaluator_profile: profile.to_owned(),
        }
    }

    fn bindings_for(
        coverage: &CoverageAssessment,
        causal_cut: &CutAssessment,
        effectivity: EffectivityRelation,
        context: &DecisionContextV0_1,
    ) -> ClaimComponentBindings {
        let mut result = ClaimComponentBindings {
            coverage: component_binding("basalt-coverage-v0.1", context),
            causal_cut: component_binding("aero-causal-cut-v0.1", context),
            effectivity: component_binding("aero-effectivity-v0.1", context),
        };
        result.coverage.report_sha256 =
            compute_component_report_sha256("OBSERVER_COVERAGE", &result.coverage, coverage)
                .expect("coverage component serializes");
        result.causal_cut.report_sha256 =
            compute_component_report_sha256("CAUSAL_CUT", &result.causal_cut, causal_cut)
                .expect("causal-cut component serializes");
        result.effectivity.report_sha256 =
            compute_component_report_sha256("EFFECTIVITY", &result.effectivity, &effectivity)
                .expect("effectivity component serializes");
        result
    }

    fn positive_bindings() -> ClaimComponentBindings {
        let evidence = all_admitted();
        let context = context_for_evidence(&evidence);
        let coverage = coverage(CoverageStatus::QualifiedCompleteForScope);
        let causal_cut = cut(CutStatus::Closed);
        bindings_for(
            &coverage,
            &causal_cut,
            EffectivityRelation::DefinitelyEffective,
            &context,
        )
    }

    fn evaluate_with_bindings(
        coverage_status: CoverageStatus,
        cut_status: CutStatus,
        effectivity: EffectivityRelation,
        evidence: &[RequiredEvidenceAdmission],
        component_bindings: &ClaimComponentBindings,
    ) -> super::ClaimAssuranceAssessment {
        let coverage = coverage(coverage_status);
        let cut = cut(cut_status);
        let decision_context = context_for_evidence(evidence);
        let owner_evidence_cut = owner_cut_for_evidence(evidence);
        let model_credibility_cut = model_cut_for_context(&decision_context);
        evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            decision_context: &decision_context,
            owner_evidence_cut: &owner_evidence_cut,
            model_credibility_cut: &model_credibility_cut,
            bindings: component_bindings,
            coverage: &coverage,
            causal_cut: &cut,
            effectivity,
            required_evidence: evidence,
        })
    }

    fn evaluate_with_owner_cut(
        coverage_status: CoverageStatus,
        cut_status: CutStatus,
        effectivity: EffectivityRelation,
        evidence: &[RequiredEvidenceAdmission],
        owner_cut: &OwnerEvidenceCutV0_1,
    ) -> super::ClaimAssuranceAssessment {
        let mut decision_context = context_for_evidence(evidence);
        decision_context.evidence_knowledge_cut_sha256 = owner_cut
            .compute_sha256()
            .expect("owner evidence cut serializes");
        let digest = context_digest(&decision_context);
        let mut bound_evidence = evidence.to_vec();
        for item in &mut bound_evidence {
            item.decision_context_sha256 = digest.clone();
        }
        let coverage_result = coverage(coverage_status);
        let cut_result = cut(cut_status);
        let component_bindings = bindings_for(
            &coverage_result,
            &cut_result,
            effectivity,
            &decision_context,
        );
        let model_credibility_cut = model_cut_for_context(&decision_context);
        evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            decision_context: &decision_context,
            owner_evidence_cut: owner_cut,
            model_credibility_cut: &model_credibility_cut,
            bindings: &component_bindings,
            coverage: &coverage_result,
            causal_cut: &cut_result,
            effectivity,
            required_evidence: &bound_evidence,
        })
    }

    fn evaluate(
        coverage_status: CoverageStatus,
        cut_status: CutStatus,
        effectivity: EffectivityRelation,
        evidence: &[RequiredEvidenceAdmission],
    ) -> super::ClaimAssuranceAssessment {
        let coverage_result = coverage(coverage_status);
        let cut_result = cut(cut_status);
        let decision_context = context_for_evidence(evidence);
        let component_bindings = bindings_for(
            &coverage_result,
            &cut_result,
            effectivity,
            &decision_context,
        );
        evaluate_with_bindings(
            coverage_status,
            cut_status,
            effectivity,
            evidence,
            &component_bindings,
        )
    }

    fn all_admitted() -> Vec<RequiredEvidenceAdmission> {
        let mut result = vec![
            evidence(EvidenceAdmissionState::Admitted, "receipt:inspection-01"),
            evidence(EvidenceAdmissionState::Admitted, "receipt:calibration-02"),
            evidence(
                EvidenceAdmissionState::Admitted,
                "aeroweift:validation:wing-panel-001",
            ),
        ];
        let context = context_for_evidence(&result);
        let digest = context_digest(&context);
        for item in &mut result {
            item.decision_context_sha256 = digest.clone();
        }
        result
    }

    fn reviewable(result: &super::ClaimAssuranceAssessment) {
        assert_eq!(
            result.status,
            ClaimGateStatus::ReviewableForIndependentDecision
        );
        assert!(!result.authority_granted);
        assert_eq!(
            result.physical_qualification_status,
            "OUT_OF_SCOPE_NOT_CLAIMED"
        );
        assert!(!result.absence_inference_granted);
    }

    #[test]
    fn positive_control_is_reviewable_only_when_every_supplied_gate_is_closed() {
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
        );
        reviewable(&result);
        assert_eq!(result.admitted_required_evidence_count, 3);
        assert_eq!(result.required_evidence_count, 3);
    }

    #[test]
    fn invalid_coverage_rejects_even_when_every_other_input_is_positive() {
        let result = evaluate(
            CoverageStatus::Invalid,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
    }

    #[test]
    fn invalid_causal_cut_rejects_even_when_every_other_input_is_positive() {
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Invalid,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
    }

    #[test]
    fn invalid_time_interval_rejects() {
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::InvalidInterval,
            &all_admitted(),
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
    }

    #[test]
    fn rejected_required_evidence_dominates_positive_components() {
        let evidence = [evidence(EvidenceAdmissionState::Rejected, "receipt:bad")];
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
    }

    #[test]
    fn scope_exclusion_is_not_rescued_by_positive_evidence() {
        let result = evaluate(
            CoverageStatus::OutOfScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
        );
        assert_eq!(result.status, ClaimGateStatus::OutOfScope);
    }

    #[test]
    fn incomplete_observer_coverage_abstains() {
        let result = evaluate(
            CoverageStatus::Incomplete,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
        );
        assert_eq!(result.status, ClaimGateStatus::Incomplete);
    }

    #[test]
    fn incomplete_causal_cut_abstains() {
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Incomplete,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
        );
        assert_eq!(result.status, ClaimGateStatus::Incomplete);
    }

    #[test]
    fn unverified_observer_or_cut_cannot_be_promoted_to_reviewable() {
        for (coverage_status, cut_status) in [
            (CoverageStatus::Unverified, CutStatus::Closed),
            (
                CoverageStatus::QualifiedCompleteForScope,
                CutStatus::Unverified,
            ),
        ] {
            let result = evaluate(
                coverage_status,
                cut_status,
                EffectivityRelation::DefinitelyEffective,
                &all_admitted(),
            );
            assert_eq!(result.status, ClaimGateStatus::Unverified);
        }
    }

    #[test]
    fn ambiguous_effectivity_abstains_instead_of_selecting_a_favourable_endpoint() {
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::BoundaryAmbiguous,
            &all_admitted(),
        );
        assert_eq!(result.status, ClaimGateStatus::Incomplete);
        assert!(
            result
                .reason_codes
                .contains(&"EFFECTIVITY_BOUNDARY_AMBIGUOUS".to_owned())
        );
    }

    #[test]
    fn event_definitely_after_acquisition_is_not_applied_to_that_subject_time() {
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyNotYetEffective,
            &all_admitted(),
        );
        assert_eq!(result.status, ClaimGateStatus::NotEffectiveAtSubjectTime);
    }

    #[test]
    fn missing_or_unverified_required_evidence_never_counts_as_admitted() {
        for state in [
            EvidenceAdmissionState::NotReceived,
            EvidenceAdmissionState::Unverified,
        ] {
            let evidence = [evidence(state, "receipt:required")];
            let result = evaluate(
                CoverageStatus::QualifiedCompleteForScope,
                CutStatus::Closed,
                EffectivityRelation::DefinitelyEffective,
                &evidence,
            );
            assert_ne!(
                result.status,
                ClaimGateStatus::ReviewableForIndependentDecision
            );
            assert_eq!(result.admitted_required_evidence_count, 0);
        }
    }

    #[test]
    fn empty_required_evidence_set_is_unverified_not_vacuously_true() {
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &[],
        );
        assert_eq!(result.status, ClaimGateStatus::Unverified);
        assert_eq!(result.admitted_required_evidence_count, 0);
    }

    #[test]
    fn malformed_or_duplicate_evidence_identity_rejects() {
        // Keep the owner cut and decision context valid, then attack the
        // required-evidence identities at the composition boundary. Building a
        // context from malformed refs would correctly fail before the gate runs.
        let valid = all_admitted();
        let context = context_for_evidence(&valid);
        let owner_cut = owner_cut_for_evidence(&valid);
        let model_cut = model_cut_for_context(&context);
        let coverage = coverage(CoverageStatus::QualifiedCompleteForScope);
        let causal_cut = cut(CutStatus::Closed);
        let bindings = bindings_for(
            &coverage,
            &causal_cut,
            EffectivityRelation::DefinitelyEffective,
            &context,
        );
        let evaluate_attacked = |required_evidence: &[RequiredEvidenceAdmission]| {
            evaluate_claim_gate(ClaimGateInput {
                claim_id: "claim-wing-panel-001",
                decision_context: &context,
                owner_evidence_cut: &owner_cut,
                model_credibility_cut: &model_cut,
                bindings: &bindings,
                coverage: &coverage,
                causal_cut: &causal_cut,
                effectivity: EffectivityRelation::DefinitelyEffective,
                required_evidence,
            })
        };

        let mut blank = valid.clone();
        blank[0].evidence_ref = " ".to_owned();
        let result = evaluate_attacked(&blank);
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"REQUIRED_EVIDENCE_REF_INVALID".to_owned())
        );

        let mut duplicate = valid;
        duplicate[1].evidence_ref = duplicate[0].evidence_ref.clone();
        let result = evaluate_attacked(&duplicate);
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"DUPLICATE_REQUIRED_EVIDENCE_REF".to_owned())
        );
    }

    #[test]
    fn evidence_order_permutation_preserves_exact_result() {
        let first = all_admitted();
        let mut reversed = first.clone();
        reversed.reverse();
        let left = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &first,
        );
        let right = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &reversed,
        );
        assert_eq!(left, right);
    }

    #[test]
    fn component_claim_identity_mismatch_rejects_composition() {
        let mut component_bindings = positive_bindings();
        component_bindings.causal_cut.claim_id = "claim-different-wing".to_owned();
        let result = evaluate_with_bindings(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
            &component_bindings,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"CAUSAL_CUT_CLAIM_ID_MISMATCH".to_owned())
        );
    }

    #[test]
    fn component_subject_mismatch_rejects_composition() {
        let mut component_bindings = positive_bindings();
        component_bindings.effectivity.subject_ref = "part:other-wing".to_owned();
        let result = evaluate_with_bindings(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
            &component_bindings,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"COMPONENT_SUBJECT_BINDING_MISMATCH".to_owned())
        );
    }

    #[test]
    fn component_decision_context_mismatch_rejects_composition() {
        let mut component_bindings = positive_bindings();
        component_bindings.coverage.decision_context_sha256 = "e".to_string().repeat(64);
        let result = evaluate_with_bindings(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
            &component_bindings,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"COMPONENT_DECISION_CONTEXT_MISMATCH".to_owned())
        );
    }

    #[test]
    fn malformed_or_reused_component_report_digest_rejects_composition() {
        let mut malformed = positive_bindings();
        malformed.coverage.report_sha256 = "not-a-sha256".to_owned();
        let result = evaluate_with_bindings(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
            &malformed,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"COVERAGE_REPORT_DIGEST_INVALID".to_owned())
        );

        let mut reused = positive_bindings();
        reused.effectivity.report_sha256 = reused.coverage.report_sha256.clone();
        let result = evaluate_with_bindings(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
            &reused,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"COMPONENT_REPORT_DIGEST_REUSED".to_owned())
        );
    }

    #[test]
    fn mandatory_evidence_must_bind_to_the_same_decision_context() {
        let mut wrong_context = all_admitted();
        wrong_context[0].decision_context_sha256 = "f".to_string().repeat(64);
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &wrong_context,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"REQUIRED_EVIDENCE_CONTEXT_MISMATCH".to_owned())
        );
    }

    #[test]
    fn required_evidence_manifest_must_match_the_exact_input_reference_set() {
        let mut evidence = all_admitted();
        let mut context = context_for_evidence(&evidence);
        context.required_evidence_manifest_sha256 = "9".repeat(64);
        let digest = context_digest(&context);
        for item in &mut evidence {
            item.decision_context_sha256 = digest.clone();
        }
        let coverage_result = coverage(CoverageStatus::QualifiedCompleteForScope);
        let cut_result = cut(CutStatus::Closed);
        let bindings = bindings_for(
            &coverage_result,
            &cut_result,
            EffectivityRelation::DefinitelyEffective,
            &context,
        );
        let result = evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            decision_context: &context,
            owner_evidence_cut: &owner_cut_for_evidence(&evidence),
            model_credibility_cut: &model_cut_for_context(&context),
            bindings: &bindings,
            coverage: &coverage_result,
            causal_cut: &cut_result,
            effectivity: EffectivityRelation::DefinitelyEffective,
            required_evidence: &evidence,
        });
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"REQUIRED_EVIDENCE_MANIFEST_MISMATCH".to_owned())
        );
    }

    #[test]
    fn context_change_invalidates_previously_bound_component_reports() {
        let evidence = all_admitted();
        let mut context = context_for_evidence(&evidence);
        context.trust_policy_sha256 = "9".repeat(64);
        let bindings = positive_bindings();
        let coverage_result = coverage(CoverageStatus::QualifiedCompleteForScope);
        let cut_result = cut(CutStatus::Closed);
        let result = evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            decision_context: &context,
            owner_evidence_cut: &owner_cut_for_evidence(&evidence),
            model_credibility_cut: &model_cut_for_context(&context),
            bindings: &bindings,
            coverage: &coverage_result,
            causal_cut: &cut_result,
            effectivity: EffectivityRelation::DefinitelyEffective,
            required_evidence: &evidence,
        });
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"COMPONENT_DECISION_CONTEXT_MISMATCH".to_owned())
        );
    }

    #[test]
    fn component_digest_binds_the_actual_typed_assessment_payload() {
        let component_bindings = positive_bindings();
        let result = evaluate_with_bindings(
            CoverageStatus::Incomplete,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &all_admitted(),
            &component_bindings,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"COVERAGE_REPORT_DIGEST_MISMATCH".to_owned())
        );
    }

    #[test]
    fn definitely_late_event_has_explicit_non_application_reason() {
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyNotYetEffective,
            &all_admitted(),
        );
        assert_eq!(result.status, ClaimGateStatus::NotEffectiveAtSubjectTime);
        assert!(
            result
                .reason_codes
                .contains(&"EVENT_NOT_YET_EFFECTIVE_AT_ACQUISITION".to_owned())
        );
    }

    #[test]
    fn owner_cut_without_confirmatory_eligibility_cannot_be_reviewable() {
        let evidence = all_admitted();
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        owner_cut.basalt_confirmatory_eligible = false;
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &owner_cut,
        );
        assert_eq!(result.status, ClaimGateStatus::Unverified);
        assert!(
            result
                .reason_codes
                .contains(&"BASALT_CONFIRMATORY_ELIGIBILITY_NOT_ESTABLISHED".to_owned())
        );
    }

    #[test]
    fn unverified_owner_artifact_payloads_cannot_be_promoted() {
        let evidence = all_admitted();
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        owner_cut.artifact_payloads_verified = false;
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &owner_cut,
        );
        assert_eq!(result.status, ClaimGateStatus::Unverified);
        assert!(
            result
                .reason_codes
                .contains(&"MYCELIX_ARTIFACT_PAYLOADS_NOT_VERIFIED".to_owned())
        );
    }

    #[test]
    fn stale_or_incomplete_required_owner_evidence_blocks_reviewability() {
        let evidence = all_admitted();
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        let stale_record = owner_cut
            .evidence_records
            .iter_mut()
            .find(|record| record.evidence_ref == "receipt:inspection-01")
            .expect("required inspection receipt is present");
        stale_record.freshness = OwnerEvidenceFreshness::StaleTransitive;
        stale_record.dependency_coverage = OwnerDependencyCoverage::Incomplete;
        stale_record.unresolved_inputs = vec!["toolchain:rust".to_owned()];
        stale_record.requires_reverification = true;
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &owner_cut,
        );
        assert_eq!(result.status, ClaimGateStatus::Incomplete);
        assert!(
            result
                .reason_codes
                .contains(&"BASALT_EVIDENCE_STALE_TRANSITIVE:receipt:inspection-01".to_owned())
        );
        assert!(
            result.reason_codes.contains(
                &"BASALT_DEPENDENCY_COVERAGE_INCOMPLETE:receipt:inspection-01".to_owned()
            )
        );
        assert!(
            result.reason_codes.contains(
                &"BASALT_EVIDENCE_REVERIFICATION_REQUIRED:receipt:inspection-01".to_owned()
            )
        );
    }

    #[test]
    fn non_active_owner_lifecycle_cannot_be_promoted_to_reviewable() {
        let evidence = all_admitted();
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        let disputed_record = owner_cut
            .evidence_records
            .iter_mut()
            .find(|record| record.evidence_ref == "receipt:inspection-01")
            .expect("required inspection receipt is present");
        disputed_record.lifecycle = OwnerEvidenceLifecycle::Disputed;
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &owner_cut,
        );
        assert_eq!(result.status, ClaimGateStatus::Unverified);
        assert!(result.reason_codes.contains(
            &"AEROCOMMONS_EVIDENCE_LIFECYCLE_NOT_ACTIVE:receipt:inspection-01".to_owned()
        ));
    }

    #[test]
    fn owner_cut_must_include_every_required_evidence_reference() {
        let evidence = all_admitted();
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        owner_cut
            .evidence_records
            .retain(|record| record.evidence_ref != "receipt:inspection-01");
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &owner_cut,
        );
        assert_eq!(result.status, ClaimGateStatus::Incomplete);
        assert!(
            result
                .reason_codes
                .contains(&"OWNER_EVIDENCE_REFERENCE_NOT_IN_CUT:receipt:inspection-01".to_owned())
        );
    }

    #[test]
    fn owner_packet_unit_and_configuration_must_match_decision_context() {
        let evidence = all_admitted();
        let mut wrong_subject = owner_cut_for_evidence(&evidence);
        wrong_subject.physical_unit_ref = "part:other-wing".to_owned();
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &wrong_subject,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"OWNER_CUT_PHYSICAL_UNIT_MISMATCH".to_owned())
        );

        let mut wrong_configuration = owner_cut_for_evidence(&evidence);
        wrong_configuration.configuration_ref = "configuration:wing-r18".to_owned();
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &wrong_configuration,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"OWNER_CUT_CONFIGURATION_MISMATCH".to_owned())
        );
    }

    #[test]
    fn owner_trust_policy_must_match_the_context() {
        let evidence = all_admitted();
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        owner_cut.trust_policy_sha256 = "9".repeat(64);
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &owner_cut,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"OWNER_CUT_TRUST_POLICY_MISMATCH".to_owned())
        );
    }

    #[test]
    fn owner_observer_coverage_out_of_scope_cannot_be_reviewable() {
        let evidence = all_admitted();
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        owner_cut.observer_coverage_status = OwnerObserverCoverageStatus::OutOfScope;
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &owner_cut,
        );
        assert_eq!(result.status, ClaimGateStatus::OutOfScope);
        assert!(
            result
                .reason_codes
                .contains(&"BASALT_OBSERVER_COVERAGE_OUT_OF_SCOPE".to_owned())
        );
    }

    #[test]
    fn owner_receipt_watermark_after_context_snapshot_is_rejected() {
        let evidence = all_admitted();
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        owner_cut.receiver_receipt_watermark_unix_ms = 1_795_516_900_000;
        let result = evaluate_with_owner_cut(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &evidence,
            &owner_cut,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"OWNER_CUT_RECEIPT_WATERMARK_AFTER_CONTEXT_SNAPSHOT".to_owned())
        );
    }

    #[test]
    fn model_cut_with_confirmatory_split_leakage_rejects_claim_review() {
        let evidence = all_admitted();
        let mut context = context_for_evidence(&evidence);
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        let mut model_cut = model_cut_for_context(&context);
        model_cut.split_integrity_status = SplitIntegrityStatus::LeakageDetected;
        let model_record = owner_cut
            .evidence_records
            .iter_mut()
            .find(|record| record.evidence_ref == model_cut.evidence_ref)
            .unwrap();
        model_record.record_sha256 = model_cut.compute_sha256().unwrap();
        context.evidence_knowledge_cut_sha256 = owner_cut.compute_sha256().unwrap();
        let digest = context_digest(&context);
        let mut bound_evidence = evidence.clone();
        for item in &mut bound_evidence {
            item.decision_context_sha256 = digest.clone();
        }
        let coverage_result = coverage(CoverageStatus::QualifiedCompleteForScope);
        let cut_result = cut(CutStatus::Closed);
        let bindings = bindings_for(
            &coverage_result,
            &cut_result,
            EffectivityRelation::DefinitelyEffective,
            &context,
        );
        let result = evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            decision_context: &context,
            owner_evidence_cut: &owner_cut,
            model_credibility_cut: &model_cut,
            bindings: &bindings,
            coverage: &coverage_result,
            causal_cut: &cut_result,
            effectivity: EffectivityRelation::DefinitelyEffective,
            required_evidence: &bound_evidence,
        });
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"MODEL_CONFIRMATORY_SPLIT_LEAKAGE".to_owned())
        );
    }

    #[test]
    fn model_cut_out_of_domain_propagates_out_of_scope() {
        let evidence = all_admitted();
        let mut context = context_for_evidence(&evidence);
        let mut model_cut = model_cut_for_context(&context);
        model_cut.domain_coverage_status = ModelDomainCoverageStatus::NotCovered;
        let coverage_result = coverage(CoverageStatus::QualifiedCompleteForScope);
        let cut_result = cut(CutStatus::Closed);
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        let record = owner_cut
            .evidence_records
            .iter_mut()
            .find(|record| record.evidence_ref == model_cut.evidence_ref)
            .unwrap();
        record.record_sha256 = model_cut.compute_sha256().unwrap();
        context.evidence_knowledge_cut_sha256 = owner_cut.compute_sha256().unwrap();
        let digest = context_digest(&context);
        let mut bound_evidence = evidence.clone();
        for item in &mut bound_evidence {
            item.decision_context_sha256 = digest.clone();
        }
        let bindings = bindings_for(
            &coverage_result,
            &cut_result,
            EffectivityRelation::DefinitelyEffective,
            &context,
        );
        let result = evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            decision_context: &context,
            owner_evidence_cut: &owner_cut,
            model_credibility_cut: &model_cut,
            bindings: &bindings,
            coverage: &coverage_result,
            causal_cut: &cut_result,
            effectivity: EffectivityRelation::DefinitelyEffective,
            required_evidence: &bound_evidence,
        });
        assert_eq!(result.status, ClaimGateStatus::OutOfScope);
        assert!(
            result
                .reason_codes
                .contains(&"MODEL_CLAIM_OUTSIDE_VALIDATED_DOMAIN".to_owned())
        );
    }

    #[test]
    fn model_owner_record_subject_must_match_exact_model_identity() {
        let evidence = all_admitted();
        let mut owner_cut = owner_cut_for_evidence(&evidence);
        let model_cut = model_cut_for_context(&context_for_evidence(&evidence));
        let record = owner_cut
            .evidence_records
            .iter_mut()
            .find(|item| item.evidence_ref == model_cut.evidence_ref)
            .unwrap();
        record.subject_ref = "instrument:unrelated-caliper".to_owned();
        let mut context = context_for_evidence(&evidence);
        context.evidence_knowledge_cut_sha256 = owner_cut.compute_sha256().unwrap();
        let digest = context_digest(&context);
        let mut bound_evidence = evidence.clone();
        for item in &mut bound_evidence {
            item.decision_context_sha256 = digest.clone();
        }
        let coverage_result = coverage(CoverageStatus::QualifiedCompleteForScope);
        let cut_result = cut(CutStatus::Closed);
        let bindings = bindings_for(
            &coverage_result,
            &cut_result,
            EffectivityRelation::DefinitelyEffective,
            &context,
        );
        let result = evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            decision_context: &context,
            owner_evidence_cut: &owner_cut,
            model_credibility_cut: &model_cut,
            bindings: &bindings,
            coverage: &coverage_result,
            causal_cut: &cut_result,
            effectivity: EffectivityRelation::DefinitelyEffective,
            required_evidence: &bound_evidence,
        });
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"MODEL_CREDIBILITY_OWNER_RECORD_MODEL_IDENTITY_MISMATCH".to_owned())
        );
    }

    #[test]
    fn model_cut_risk_or_configuration_substitution_rejects_claim_binding() {
        let evidence = all_admitted();
        let context = context_for_evidence(&evidence);
        let mut model_cut = model_cut_for_context(&context);
        model_cut.risk_classification_sha256 = "9".repeat(64);
        let owner_cut = owner_cut_for_evidence(&evidence);
        let coverage_result = coverage(CoverageStatus::QualifiedCompleteForScope);
        let cut_result = cut(CutStatus::Closed);
        let bindings = bindings_for(
            &coverage_result,
            &cut_result,
            EffectivityRelation::DefinitelyEffective,
            &context,
        );
        let result = evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            decision_context: &context,
            owner_evidence_cut: &owner_cut,
            model_credibility_cut: &model_cut,
            bindings: &bindings,
            coverage: &coverage_result,
            causal_cut: &cut_result,
            effectivity: EffectivityRelation::DefinitelyEffective,
            required_evidence: &evidence,
        });
        assert_eq!(result.status, ClaimGateStatus::Rejected);
        assert!(
            result
                .reason_codes
                .contains(&"MODEL_CREDIBILITY_RISK_CLASSIFICATION_MISMATCH".to_owned())
        );
    }

    #[test]
    fn any_single_gate_degradation_removes_the_reviewable_disposition() {
        let positive = all_admitted();
        let degraded_coverage = [
            CoverageStatus::Incomplete,
            CoverageStatus::Unverified,
            CoverageStatus::Invalid,
            CoverageStatus::OutOfScope,
        ];
        for status in degraded_coverage {
            assert_ne!(
                evaluate(
                    status,
                    CutStatus::Closed,
                    EffectivityRelation::DefinitelyEffective,
                    &positive
                )
                .status,
                ClaimGateStatus::ReviewableForIndependentDecision
            );
        }

        let degraded_cut = [
            CutStatus::Incomplete,
            CutStatus::Unverified,
            CutStatus::Invalid,
        ];
        for status in degraded_cut {
            assert_ne!(
                evaluate(
                    CoverageStatus::QualifiedCompleteForScope,
                    status,
                    EffectivityRelation::DefinitelyEffective,
                    &positive
                )
                .status,
                ClaimGateStatus::ReviewableForIndependentDecision
            );
        }
    }
}
