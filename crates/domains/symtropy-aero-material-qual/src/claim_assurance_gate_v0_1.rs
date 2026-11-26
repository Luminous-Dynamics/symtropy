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

use super::effectivity_interval_v0_1::EffectivityRelation;
use super::observer_coverage_v0_1::{CoverageAssessment, CoverageStatus};
use super::vector_watermark_v0_1::{CutAssessment, CutStatus};
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
    /// Upstream evidence admission state.
    pub state: EvidenceAdmissionState,
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
        EffectivityRelation::DefinitelyEffective => {}
        EffectivityRelation::DefinitelyNotYetEffective => not_yet_effective = true,
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

    ClaimAssuranceAssessment {
        claim_id: input.claim_id.to_owned(),
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
        ClaimGateInput, ClaimGateStatus, EvidenceAdmissionState, RequiredEvidenceAdmission,
        evaluate_claim_gate,
    };
    use crate::effectivity_interval_v0_1::EffectivityRelation;
    use crate::observer_coverage_v0_1::{
        CoverageAssessment, CoverageStatus, ReplayConsequence,
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

    fn evidence(state: EvidenceAdmissionState, evidence_ref: &str) -> RequiredEvidenceAdmission {
        RequiredEvidenceAdmission {
            evidence_ref: evidence_ref.to_owned(),
            state,
        }
    }

    fn evaluate(
        coverage_status: CoverageStatus,
        cut_status: CutStatus,
        effectivity: EffectivityRelation,
        evidence: &[RequiredEvidenceAdmission],
    ) -> super::ClaimAssuranceAssessment {
        let coverage = coverage(coverage_status);
        let cut = cut(cut_status);
        evaluate_claim_gate(ClaimGateInput {
            claim_id: "claim-wing-panel-001",
            coverage: &coverage,
            causal_cut: &cut,
            effectivity,
            required_evidence: evidence,
        })
    }

    fn all_admitted() -> Vec<RequiredEvidenceAdmission> {
        vec![
            evidence(EvidenceAdmissionState::Admitted, "receipt:inspection-01"),
            evidence(EvidenceAdmissionState::Admitted, "receipt:calibration-02"),
        ]
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
            (CoverageStatus::QualifiedCompleteForScope, CutStatus::Unverified),
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
