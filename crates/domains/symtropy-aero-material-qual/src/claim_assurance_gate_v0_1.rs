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
use super::observer_coverage_v0_1::{CoverageAssessment, CoverageStatus};
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

/// Validate that the three typed component results refer to exactly the same
/// claim, physical subject and canonical decision context. Each supplied
/// report digest must match a fresh digest of its actual typed assessment.
fn validate_component_bindings(
    claim_id: &str,
    subject_ref: &str,
    expected_context_sha256: &str,
    bindings: &ClaimComponentBindings,
    coverage: &CoverageAssessment,
    causal_cut: &CutAssessment,
    effectivity: &EffectivityRelation,
    reasons: &mut BTreeSet<String>,
) -> Option<String> {
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
        input.claim_id,
        &input.decision_context.subject_ref,
        &decision_context_sha256,
        input.bindings,
        input.coverage,
        input.causal_cut,
        &input.effectivity,
        &mut reasons,
    ) {
        invalid = true;
    }

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
    ClaimAssuranceAssessment {
        claim_id: input.claim_id.to_owned(),
        decision_context_sha256,
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
    use crate::decision_context_v0_1::{DecisionContextV0_1, compute_required_evidence_manifest_sha256};
    use crate::effectivity_interval_v0_1::EffectivityRelation;
    use crate::observer_coverage_v0_1::{CoverageAssessment, CoverageStatus, ReplayConsequence};
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

    fn decision_context_for_refs(evidence_refs: &[String]) -> DecisionContextV0_1 {
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
            required_evidence_manifest_sha256: compute_required_evidence_manifest_sha256(evidence_refs)
                .expect("evidence manifest serializes"),
            risk_classification_sha256: "e".repeat(64),
            model_applicability_profile_sha256: "f".repeat(64),
        }
    }

    fn context_for_evidence(evidence: &[RequiredEvidenceAdmission]) -> DecisionContextV0_1 {
        let refs = evidence.iter().map(|item| item.evidence_ref.clone()).collect::<Vec<_>>();
        decision_context_for_refs(&refs)
    }

    fn context_digest(context: &DecisionContextV0_1) -> String {
        context.compute_sha256().expect("test decision context is valid")
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
        evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            decision_context: &decision_context,
            bindings: component_bindings,
            coverage: &coverage,
            causal_cut: &cut,
            effectivity,
            required_evidence: evidence,
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
        let component_bindings = bindings_for(&coverage_result, &cut_result, effectivity, &decision_context);
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
        assert_eq!(result.admitted_required_evidence_count, 2);
        assert_eq!(result.required_evidence_count, 2);
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
        let blank = [evidence(EvidenceAdmissionState::Admitted, " ")];
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &blank,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);

        let duplicate = [
            evidence(EvidenceAdmissionState::Admitted, "receipt:same"),
            evidence(EvidenceAdmissionState::Admitted, "receipt:same"),
        ];
        let result = evaluate(
            CoverageStatus::QualifiedCompleteForScope,
            CutStatus::Closed,
            EffectivityRelation::DefinitelyEffective,
            &duplicate,
        );
        assert_eq!(result.status, ClaimGateStatus::Rejected);
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
