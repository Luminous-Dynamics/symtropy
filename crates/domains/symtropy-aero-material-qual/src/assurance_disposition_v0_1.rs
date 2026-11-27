// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Non-compensatory AERO assurance disposition projection v0.1.
//!
//! This module projects an already assembled, typed obligation vector into a
//! review-routing disposition. It does not authenticate evidence, determine
//! physical truth, validate model credibility, or grant any form of approval.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const ASSURANCE_STATE_VECTOR_SCHEMA_V0_1: &str = "AeroAssuranceStateVectorV0_1";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ReplayModeV0_1 {
    #[serde(rename = "HISTORICAL_AS_KNOWN")]
    HistoricalAsKnown,
    #[serde(rename = "CURRENT_POLICY_COUNTERFACTUAL")]
    CurrentPolicyCounterfactual,
    #[serde(rename = "CURRENT_AS_KNOWN")]
    CurrentAsKnown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SubjectTimeEffectivityV0_1 {
    #[serde(rename = "DEFINITELY_EFFECTIVE")]
    DefinitelyEffective,
    #[serde(rename = "DEFINITELY_NOT_YET_EFFECTIVE")]
    DefinitelyNotYetEffective,
    #[serde(rename = "BOUNDARY_AMBIGUOUS")]
    BoundaryAmbiguous,
    #[serde(rename = "INVALID_INTERVAL")]
    InvalidInterval,
}

impl SubjectTimeEffectivityV0_1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DefinitelyEffective => "DEFINITELY_EFFECTIVE",
            Self::DefinitelyNotYetEffective => "DEFINITELY_NOT_YET_EFFECTIVE",
            Self::BoundaryAmbiguous => "BOUNDARY_AMBIGUOUS",
            Self::InvalidInterval => "INVALID_INTERVAL",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PropositionEvidenceStateV0_1 {
    #[serde(rename = "NEITHER")]
    Neither,
    #[serde(rename = "SUPPORTED_ONLY")]
    SupportedOnly,
    #[serde(rename = "REFUTED_ONLY")]
    RefutedOnly,
    #[serde(rename = "BOTH")]
    Both,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ApplicabilityStateV0_1 {
    #[serde(rename = "IN_SCOPE")]
    InScope,
    #[serde(rename = "OUT_OF_SCOPE")]
    OutOfScope,
    #[serde(rename = "UNKNOWN")]
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AuthorityStateV0_1 {
    #[serde(rename = "AUTHORIZED")]
    Authorized,
    #[serde(rename = "DENIED")]
    Denied,
    #[serde(rename = "UNKNOWN")]
    Unknown,
    #[serde(rename = "NOT_REQUIRED")]
    NotRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TemporalKnowledgeStateV0_1 {
    #[serde(rename = "KNOWN_AT_CUTOFF")]
    KnownAtCutoff,
    #[serde(rename = "NOT_KNOWN_AT_CUTOFF")]
    NotKnownAtCutoff,
    #[serde(rename = "TEMPORAL_CONFLICT")]
    TemporalConflict,
    #[serde(rename = "NOT_TEMPORAL")]
    NotTemporal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ObserverCoverageStateV0_1 {
    #[serde(rename = "COMPLETE")]
    Complete,
    #[serde(rename = "INCOMPLETE")]
    Incomplete,
    #[serde(rename = "UNKNOWN")]
    Unknown,
    #[serde(rename = "NOT_APPLICABLE")]
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum VerificationStateV0_1 {
    #[serde(rename = "VERIFIED")]
    Verified,
    #[serde(rename = "FAILED")]
    Failed,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "NOT_RUN")]
    NotRun,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ObligationKindV0_1 {
    #[serde(rename = "CLAIM_IDENTITY")]
    ClaimIdentity,
    #[serde(rename = "EVIDENCE_INTEGRITY")]
    EvidenceIntegrity,
    #[serde(rename = "PRODUCER_AUTHORITY")]
    ProducerAuthority,
    #[serde(rename = "SEMANTIC_VALIDITY")]
    SemanticValidity,
    #[serde(rename = "MODEL_APPLICABILITY")]
    ModelApplicability,
    #[serde(rename = "MODEL_CREDIBILITY")]
    ModelCredibility,
    #[serde(rename = "TEMPORAL_KNOWLEDGE")]
    TemporalKnowledge,
    #[serde(rename = "DEPENDENCY_OBSERVATION")]
    DependencyObservation,
    #[serde(rename = "DEFEATER_RESOLUTION")]
    DefeaterResolution,
    #[serde(rename = "INDEPENDENT_REPLAY")]
    IndependentReplay,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimContextV0_1 {
    pub claim_id: String,
    pub physical_subject_ref: String,
    pub configuration_ref: String,
    pub as_known_cutoff: String,
    pub trust_policy_snapshot_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceObligationV0_1 {
    pub obligation_id: String,
    pub kind: ObligationKindV0_1,
    pub mandatory: bool,
    pub hard_gate: bool,
    pub proposition_evidence: PropositionEvidenceStateV0_1,
    pub applicability: ApplicabilityStateV0_1,
    pub authority: AuthorityStateV0_1,
    pub temporal_knowledge: TemporalKnowledgeStateV0_1,
    pub observer_coverage: ObserverCoverageStateV0_1,
    pub verification: VerificationStateV0_1,
    pub evidence_refs: Vec<String>,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DefeaterStatusV0_1 {
    #[serde(rename = "CANDIDATE")]
    Candidate,
    #[serde(rename = "UNRESOLVED")]
    Unresolved,
    #[serde(rename = "CONFIRMED")]
    Confirmed,
    #[serde(rename = "REFUTED")]
    Refuted,
    #[serde(rename = "DISMISSED")]
    Dismissed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DefeaterMaterialityV0_1 {
    #[serde(rename = "BLOCKING")]
    Blocking,
    #[serde(rename = "ADVISORY")]
    Advisory,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefeaterV0_1 {
    pub defeater_id: String,
    pub status: DefeaterStatusV0_1,
    pub materiality: DefeaterMaterialityV0_1,
    pub evidence_refs: Vec<String>,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssuranceStateCaseV0_1 {
    pub case_id: String,
    pub description: String,
    pub replay_mode: ReplayModeV0_1,
    pub subject_time_effectivity: SubjectTimeEffectivityV0_1,
    pub claim_context: ClaimContextV0_1,
    pub obligations: Vec<AssuranceObligationV0_1>,
    pub defeaters: Vec<DefeaterV0_1>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AssuranceDispositionV0_1 {
    #[serde(rename = "REJECTED")]
    Rejected,
    #[serde(rename = "OUT_OF_SCOPE")]
    OutOfScope,
    #[serde(rename = "NOT_KNOWN_AT_SNAPSHOT")]
    NotKnownAtSnapshot,
    #[serde(rename = "NOT_EFFECTIVE_AT_SUBJECT_TIME")]
    NotEffectiveAtSubjectTime,
    #[serde(rename = "UNVERIFIED")]
    Unverified,
    #[serde(rename = "INCOMPLETE")]
    Incomplete,
    #[serde(rename = "REVIEWABLE_FOR_INDEPENDENT_DECISION")]
    ReviewableForIndependentDecision,
}

impl AssuranceDispositionV0_1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rejected => "REJECTED",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::NotKnownAtSnapshot => "NOT_KNOWN_AT_SNAPSHOT",
            Self::NotEffectiveAtSubjectTime => "NOT_EFFECTIVE_AT_SUBJECT_TIME",
            Self::Unverified => "UNVERIFIED",
            Self::Incomplete => "INCOMPLETE",
            Self::ReviewableForIndependentDecision => "REVIEWABLE_FOR_INDEPENDENT_DECISION",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BlockingObligationV0_1 {
    pub obligation_id: String,
    pub reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssuranceDispositionAssessmentV0_1 {
    pub case_id: String,
    pub disposition: AssuranceDispositionV0_1,
    pub reason_codes: Vec<String>,
    pub blocking_obligations: Vec<BlockingObligationV0_1>,
    pub blocking_defeater_ids: Vec<String>,
    pub subject_time_effectivity: String,
    pub authority_granted: bool,
    pub physical_qualification_status: String,
    pub certification_status: String,
}

fn nonempty_canonical(value: &str) -> bool {
    !value.trim().is_empty() && value == value.trim()
}

fn valid_sha256_reference(value: &str) -> bool {
    let Some(digest) = value.strip_prefix("sha256:") else {
        return false;
    };
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Deterministically project a typed obligation vector into a review disposition.
///
/// All component reason codes are preserved and sorted. A failed mandatory
/// hard-gate verifier is distinct from an unrun or untrusted verifier. A BOTH
/// evidence state is a conflict and is never silently collapsed into support.
pub fn evaluate_assurance_state_case_v0_1(
    case: &AssuranceStateCaseV0_1,
) -> AssuranceDispositionAssessmentV0_1 {
    let mut reasons = BTreeSet::new();
    let mut blocking_obligations = Vec::new();
    let mut blocking_defeaters = BTreeSet::new();

    let mut rejected = false;
    let mut out_of_scope = false;
    let mut not_known = false;
    let mut not_effective_at_subject_time = false;
    let mut unverified = false;
    let mut incomplete = false;

    match case.subject_time_effectivity {
        SubjectTimeEffectivityV0_1::DefinitelyEffective => {}
        SubjectTimeEffectivityV0_1::DefinitelyNotYetEffective => {
            not_effective_at_subject_time = true;
            reasons.insert("EVENT_EFFECTIVE_AFTER_EVERY_POSSIBLE_ACQUISITION".to_owned());
        }
        SubjectTimeEffectivityV0_1::BoundaryAmbiguous => {
            incomplete = true;
            reasons.insert("EVENT_AND_ACQUISITION_TIME_BOUNDS_OVERLAP".to_owned());
        }
        SubjectTimeEffectivityV0_1::InvalidInterval => {
            rejected = true;
            reasons.insert("TEMPORAL_INTERVAL_START_AFTER_END".to_owned());
        }
    }

    if !nonempty_canonical(&case.case_id)
        || !nonempty_canonical(&case.claim_context.claim_id)
        || !nonempty_canonical(&case.claim_context.physical_subject_ref)
        || !nonempty_canonical(&case.claim_context.configuration_ref)
        || !nonempty_canonical(&case.claim_context.as_known_cutoff)
        || !valid_sha256_reference(&case.claim_context.trust_policy_snapshot_digest)
        || case.obligations.is_empty()
        || !case.obligations.iter().any(|item| item.mandatory)
    {
        incomplete = true;
        reasons.insert("ASSURANCE_STATE_VECTOR_INPUT_INCOMPLETE".to_owned());
    }

    let intrinsic_hard_gates = [
        ObligationKindV0_1::ClaimIdentity,
        ObligationKindV0_1::EvidenceIntegrity,
        ObligationKindV0_1::ProducerAuthority,
    ];
    for required_kind in intrinsic_hard_gates {
        let matching = case
            .obligations
            .iter()
            .filter(|obligation| obligation.kind == required_kind)
            .collect::<Vec<_>>();
        if matching.is_empty()
            || matching
                .iter()
                .any(|obligation| !obligation.mandatory || !obligation.hard_gate)
        {
            unverified = true;
            reasons.insert("INTRINSIC_HARD_GATE_POLICY_MISSING_OR_DOWNGRADED".to_owned());
        }
    }

    let mut obligation_ids = BTreeSet::new();
    for obligation in &case.obligations {
        let mut local_reasons = BTreeSet::new();
        local_reasons.extend(obligation.reason_codes.iter().cloned());

        if !nonempty_canonical(&obligation.obligation_id)
            || !obligation_ids.insert(obligation.obligation_id.as_str())
        {
            incomplete = true;
            reasons.insert("ASSURANCE_OBLIGATION_ID_INVALID_OR_DUPLICATE".to_owned());
            if obligation.mandatory {
                blocking_obligations.push(BlockingObligationV0_1 {
                    obligation_id: obligation.obligation_id.clone(),
                    reason_codes: vec!["ASSURANCE_OBLIGATION_ID_INVALID_OR_DUPLICATE".to_owned()],
                });
            }
            continue;
        }

        let mut blocks = false;
        if obligation.mandatory {
            let mut evidence_refs_seen = BTreeSet::new();
            let evidence_refs_valid = obligation.evidence_refs.iter().all(|reference| {
                nonempty_canonical(reference) && evidence_refs_seen.insert(reference.as_str())
            });
            let has_evidence_reference =
                evidence_refs_valid && !obligation.evidence_refs.is_empty();
            if !evidence_refs_valid
                || (obligation.proposition_evidence != PropositionEvidenceStateV0_1::Neither
                    && !has_evidence_reference)
            {
                incomplete = true;
                blocks = true;
                local_reasons.insert("MANDATORY_EVIDENCE_REFERENCE_MISSING_OR_INVALID".to_owned());
            }

            if obligation.kind == ObligationKindV0_1::ProducerAuthority
                && obligation.authority == AuthorityStateV0_1::NotRequired
            {
                incomplete = true;
                blocks = true;
                local_reasons.insert("MANDATORY_PRODUCER_AUTHORITY_CANNOT_BE_WAIVED".to_owned());
            }

            // A verified negative result on a hard gate is a known failure.
            if obligation.hard_gate
                && ((obligation.proposition_evidence == PropositionEvidenceStateV0_1::RefutedOnly
                    && obligation.verification == VerificationStateV0_1::Verified
                    || obligation.verification == VerificationStateV0_1::Failed)
                    && has_evidence_reference)
            {
                rejected = true;
                blocks = true;
                local_reasons.insert("MANDATORY_HARD_GATE_FAILED".to_owned());
            }

            if obligation.hard_gate
                && obligation.authority == AuthorityStateV0_1::Denied
                && obligation.verification == VerificationStateV0_1::Verified
            {
                rejected = true;
                blocks = true;
                local_reasons.insert("REQUIRED_AUTHORITY_EXPLICITLY_DENIED".to_owned());
            }

            if obligation.applicability == ApplicabilityStateV0_1::OutOfScope
                && obligation.verification == VerificationStateV0_1::Verified
            {
                out_of_scope = true;
                blocks = true;
                local_reasons.insert("MANDATORY_OBLIGATION_OUT_OF_SCOPE".to_owned());
            }

            if obligation.temporal_knowledge == TemporalKnowledgeStateV0_1::NotKnownAtCutoff {
                not_known = true;
                blocks = true;
                local_reasons.insert("EVIDENCE_NOT_KNOWN_AT_SNAPSHOT".to_owned());
            }

            if obligation.authority == AuthorityStateV0_1::Unknown
                || obligation.verification == VerificationStateV0_1::Unverified
                || (obligation.verification == VerificationStateV0_1::NotRun
                    && !obligation.evidence_refs.is_empty())
            {
                unverified = true;
                blocks = true;
                local_reasons.insert("MANDATORY_TRUST_OR_VERIFICATION_UNVERIFIED".to_owned());
            }

            if obligation.verification == VerificationStateV0_1::NotRun
                && obligation.evidence_refs.is_empty()
            {
                incomplete = true;
                blocks = true;
                local_reasons.insert("MANDATORY_EVIDENCE_NOT_RECEIVED".to_owned());
            }

            if obligation.verification == VerificationStateV0_1::Failed && !obligation.hard_gate {
                incomplete = true;
                blocks = true;
                local_reasons.insert("MANDATORY_NON_HARD_VERIFICATION_FAILED".to_owned());
            }

            if obligation.applicability == ApplicabilityStateV0_1::Unknown
                || obligation.temporal_knowledge == TemporalKnowledgeStateV0_1::TemporalConflict
                || obligation.proposition_evidence != PropositionEvidenceStateV0_1::SupportedOnly
                || obligation.observer_coverage == ObserverCoverageStateV0_1::Incomplete
                || obligation.observer_coverage == ObserverCoverageStateV0_1::Unknown
            {
                incomplete = true;
                blocks = true;
                if obligation.proposition_evidence == PropositionEvidenceStateV0_1::Both {
                    local_reasons.insert("MANDATORY_EVIDENCE_CONFLICT".to_owned());
                } else if obligation.proposition_evidence == PropositionEvidenceStateV0_1::Neither {
                    local_reasons.insert("MANDATORY_EVIDENCE_NOT_ESTABLISHED".to_owned());
                }
                if obligation.temporal_knowledge == TemporalKnowledgeStateV0_1::TemporalConflict {
                    local_reasons.insert("MANDATORY_TEMPORAL_CONFLICT".to_owned());
                }
                if matches!(
                    obligation.observer_coverage,
                    ObserverCoverageStateV0_1::Incomplete | ObserverCoverageStateV0_1::Unknown
                ) {
                    local_reasons.insert("DEPENDENCY_OBSERVER_COVERAGE_NOT_COMPLETE".to_owned());
                }
                if obligation.applicability == ApplicabilityStateV0_1::Unknown {
                    local_reasons.insert("MANDATORY_APPLICABILITY_UNKNOWN".to_owned());
                }
            }

            if obligation.authority == AuthorityStateV0_1::Denied
                && !(obligation.hard_gate
                    && obligation.verification == VerificationStateV0_1::Verified)
            {
                incomplete = true;
                blocks = true;
                local_reasons.insert("MANDATORY_AUTHORITY_DENIED_OR_UNRESOLVED".to_owned());
            }

            if obligation.applicability == ApplicabilityStateV0_1::OutOfScope
                && obligation.verification != VerificationStateV0_1::Verified
            {
                unverified = true;
                blocks = true;
                local_reasons.insert("OUT_OF_SCOPE_ASSERTION_NOT_VERIFIED".to_owned());
            }

            if blocks {
                blocking_obligations.push(BlockingObligationV0_1 {
                    obligation_id: obligation.obligation_id.clone(),
                    reason_codes: local_reasons.iter().cloned().collect(),
                });
            }
        }

        reasons.extend(local_reasons);
    }

    for defeater in &case.defeaters {
        reasons.extend(defeater.reason_codes.iter().cloned());
        if defeater.materiality == DefeaterMaterialityV0_1::Blocking
            && matches!(
                defeater.status,
                DefeaterStatusV0_1::Unresolved | DefeaterStatusV0_1::Confirmed
            )
        {
            incomplete = true;
            blocking_defeaters.insert(defeater.defeater_id.clone());
            reasons.insert("MATERIAL_DEFEATER_REMAINS_BLOCKING".to_owned());
        }
    }

    blocking_obligations.sort_by(|left, right| left.obligation_id.cmp(&right.obligation_id));

    let disposition = if rejected {
        AssuranceDispositionV0_1::Rejected
    } else if out_of_scope {
        AssuranceDispositionV0_1::OutOfScope
    } else if not_known {
        AssuranceDispositionV0_1::NotKnownAtSnapshot
    } else if unverified {
        AssuranceDispositionV0_1::Unverified
    } else if incomplete {
        AssuranceDispositionV0_1::Incomplete
    } else if not_effective_at_subject_time {
        AssuranceDispositionV0_1::NotEffectiveAtSubjectTime
    } else {
        AssuranceDispositionV0_1::ReviewableForIndependentDecision
    };

    AssuranceDispositionAssessmentV0_1 {
        case_id: case.case_id.clone(),
        disposition,
        reason_codes: reasons.into_iter().collect(),
        blocking_obligations,
        blocking_defeater_ids: blocking_defeaters.into_iter().collect(),
        subject_time_effectivity: case.subject_time_effectivity.as_str().to_owned(),
        authority_granted: false,
        physical_qualification_status: "OUT_OF_SCOPE_NOT_CLAIMED".to_owned(),
        certification_status: "NOT_CLAIMED".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApplicabilityStateV0_1 as Applicability, AssuranceDispositionV0_1 as Disposition,
        AssuranceObligationV0_1 as Obligation, AssuranceStateCaseV0_1 as Case,
        SubjectTimeEffectivityV0_1 as SubjectEffectivity,
        AuthorityStateV0_1 as Authority, ClaimContextV0_1 as Context,
        DefeaterMaterialityV0_1 as Materiality, DefeaterStatusV0_1 as DefeaterStatus,
        DefeaterV0_1 as Defeater, ObligationKindV0_1 as Kind,
        ObserverCoverageStateV0_1 as Observer, PropositionEvidenceStateV0_1 as Evidence,
        ReplayModeV0_1 as Replay, TemporalKnowledgeStateV0_1 as Temporal,
        VerificationStateV0_1 as Verification, evaluate_assurance_state_case_v0_1,
    };

    fn obligation(id: &str, kind: Kind) -> Obligation {
        Obligation {
            obligation_id: id.to_owned(),
            kind,
            mandatory: true,
            hard_gate: matches!(
                kind,
                Kind::ClaimIdentity | Kind::EvidenceIntegrity | Kind::ProducerAuthority
            ),
            proposition_evidence: Evidence::SupportedOnly,
            applicability: Applicability::InScope,
            authority: if kind == Kind::ProducerAuthority {
                Authority::Authorized
            } else {
                Authority::NotRequired
            },
            temporal_knowledge: if kind == Kind::TemporalKnowledge {
                Temporal::KnownAtCutoff
            } else {
                Temporal::NotTemporal
            },
            observer_coverage: if kind == Kind::DependencyObservation {
                Observer::Complete
            } else {
                Observer::NotApplicable
            },
            verification: Verification::Verified,
            evidence_refs: vec![
                "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_owned(),
            ],
            reason_codes: Vec::new(),
        }
    }

    fn positive_case() -> Case {
        Case {
            case_id: "ASV-TEST-01".to_owned(),
            description: "All mandatory obligations pass.".to_owned(),
            replay_mode: Replay::HistoricalAsKnown,
            subject_time_effectivity: super::SubjectTimeEffectivityV0_1::DefinitelyEffective,
            claim_context: Context {
                claim_id: "CLAIM-001".to_owned(),
                physical_subject_ref: "ARTICLE-001".to_owned(),
                configuration_ref: "CONFIG-A".to_owned(),
                as_known_cutoff: "2026-11-01T00:00:00Z".to_owned(),
                trust_policy_snapshot_digest:
                    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                        .to_owned(),
            },
            obligations: vec![
                obligation("identity", Kind::ClaimIdentity),
                obligation("producer", Kind::ProducerAuthority),
                obligation("semantic", Kind::SemanticValidity),
                obligation("temporal", Kind::TemporalKnowledge),
                obligation("observer", Kind::DependencyObservation),
            ],
            defeaters: Vec::new(),
        }
    }

    fn disposition(case: &Case) -> Disposition {
        evaluate_assurance_state_case_v0_1(case).disposition
    }

    #[test]
    fn positive_control_is_reviewable_but_never_grants_authority() {
        let result = evaluate_assurance_state_case_v0_1(&positive_case());
        assert_eq!(
            result.disposition,
            Disposition::ReviewableForIndependentDecision
        );
        assert!(!result.authority_granted);
        assert_eq!(
            result.physical_qualification_status,
            "OUT_OF_SCOPE_NOT_CLAIMED"
        );
        assert_eq!(result.certification_status, "NOT_CLAIMED");
    }

    #[test]
    fn both_preserves_conflict_and_blocks_reviewability() {
        let mut case = positive_case();
        case.obligations[2].proposition_evidence = Evidence::Both;
        case.obligations[2]
            .reason_codes
            .push("EVIDENCE_CONFLICT".to_owned());
        let result = evaluate_assurance_state_case_v0_1(&case);
        assert_eq!(result.disposition, Disposition::Incomplete);
        assert!(
            result
                .reason_codes
                .iter()
                .any(|code| code == "EVIDENCE_CONFLICT")
        );
        assert!(
            result
                .reason_codes
                .iter()
                .any(|code| code == "MANDATORY_EVIDENCE_CONFLICT")
        );
    }

    #[test]
    fn neither_does_not_become_false_or_true() {
        let mut case = positive_case();
        case.obligations[2].proposition_evidence = Evidence::Neither;
        assert_eq!(disposition(&case), Disposition::Incomplete);
    }

    #[test]
    fn late_receiver_knowledge_is_not_historical_knowledge() {
        let mut case = positive_case();
        case.obligations[3].temporal_knowledge = Temporal::NotKnownAtCutoff;
        assert_eq!(disposition(&case), Disposition::NotKnownAtSnapshot);
    }

    #[test]
    fn unknown_observer_coverage_never_means_empty_impact() {
        let mut case = positive_case();
        case.obligations[4].observer_coverage = Observer::Unknown;
        assert_eq!(disposition(&case), Disposition::Incomplete);
    }

    #[test]
    fn verified_hard_gate_counterevidence_rejects() {
        let mut case = positive_case();
        case.obligations[0].proposition_evidence = Evidence::RefutedOnly;
        assert_eq!(disposition(&case), Disposition::Rejected);
    }

    #[test]
    fn affirmatively_out_of_scope_claim_is_not_reviewable() {
        let mut case = positive_case();
        case.obligations[2].applicability = Applicability::OutOfScope;
        assert_eq!(disposition(&case), Disposition::OutOfScope);
    }

    #[test]
    fn unknown_authority_is_unverified_not_rejected() {
        let mut case = positive_case();
        case.obligations[1].authority = Authority::Unknown;
        assert_eq!(disposition(&case), Disposition::Unverified);
    }

    #[test]
    fn verified_authority_denial_is_a_hard_gate_rejection() {
        let mut case = positive_case();
        case.obligations[1].authority = Authority::Denied;
        assert_eq!(disposition(&case), Disposition::Rejected);
    }

    #[test]
    fn candidate_defeater_does_not_become_a_finding() {
        let mut case = positive_case();
        case.defeaters.push(Defeater {
            defeater_id: "candidate-llm-01".to_owned(),
            status: DefeaterStatus::Candidate,
            materiality: Materiality::Blocking,
            evidence_refs: Vec::new(),
            reason_codes: vec!["GENERATED_DEFEATER_HYPOTHESIS".to_owned()],
        });
        let result = evaluate_assurance_state_case_v0_1(&case);
        assert_eq!(result.disposition, Disposition::ReviewableForIndependentDecision);
        assert!(
            result
                .reason_codes
                .iter()
                .any(|code| code == "GENERATED_DEFEATER_HYPOTHESIS")
        );
        assert!(result.blocking_defeater_ids.is_empty());
    }

    #[test]
    fn unresolved_material_defeater_blocks_reviewability() {
        let mut case = positive_case();
        case.defeaters.push(Defeater {
            defeater_id: "defeater-01".to_owned(),
            status: DefeaterStatus::Unresolved,
            materiality: Materiality::Blocking,
            evidence_refs: vec![
                "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                    .to_owned(),
            ],
            reason_codes: vec!["MATERIAL_DEFEATER_UNRESOLVED".to_owned()],
        });
        let result = evaluate_assurance_state_case_v0_1(&case);
        assert_eq!(result.disposition, Disposition::Incomplete);
        assert_eq!(result.blocking_defeater_ids, vec!["defeater-01".to_owned()]);
    }

    #[test]
    fn optional_nonpassing_obligation_is_reported_but_does_not_block() {
        let mut case = positive_case();
        let mut optional = obligation("optional", Kind::IndependentReplay);
        optional.mandatory = false;
        optional.proposition_evidence = Evidence::Neither;
        optional
            .reason_codes
            .push("OPTIONAL_REPLAY_NOT_RUN".to_owned());
        case.obligations.push(optional);
        let result = evaluate_assurance_state_case_v0_1(&case);
        assert_eq!(result.disposition, Disposition::ReviewableForIndependentDecision);
        assert!(
            result
                .reason_codes
                .iter()
                .any(|code| code == "OPTIONAL_REPLAY_NOT_RUN")
        );
        assert!(
            !result
                .blocking_obligations
                .iter()
                .any(|item| item.obligation_id == "optional")
        );
    }

    #[test]
    fn projection_precedence_is_deterministic_and_retains_all_reason_codes() {
        let mut case = positive_case();
        case.obligations[0].proposition_evidence = Evidence::RefutedOnly;
        case.obligations[2].applicability = Applicability::OutOfScope;
        case.obligations[3].temporal_knowledge = Temporal::NotKnownAtCutoff;
        case.obligations[4].observer_coverage = Observer::Unknown;
        let result = evaluate_assurance_state_case_v0_1(&case);
        assert_eq!(result.disposition, Disposition::Rejected);
        assert!(
            result
                .reason_codes
                .iter()
                .any(|code| code == "MANDATORY_OBLIGATION_OUT_OF_SCOPE")
        );
        assert!(
            result
                .reason_codes
                .iter()
                .any(|code| code == "EVIDENCE_NOT_KNOWN_AT_SNAPSHOT")
        );
        assert!(
            result
                .reason_codes
                .iter()
                .any(|code| code == "DEPENDENCY_OBSERVER_COVERAGE_NOT_COMPLETE")
        );
    }

    #[test]
    fn failed_hard_gate_verifier_is_rejected_but_unrun_is_not() {
        let mut case = positive_case();
        case.obligations[0].verification = Verification::Failed;
        assert_eq!(disposition(&case), Disposition::Rejected);
        case.obligations[0].verification = Verification::NotRun;
        assert_eq!(disposition(&case), Disposition::Unverified);
    }

    #[test]
    fn empty_mandatory_obligation_set_fails_closed() {
        let mut case = positive_case();
        for item in &mut case.obligations {
            item.mandatory = false;
        }
        assert_eq!(disposition(&case), Disposition::Incomplete);
    }

    #[test]
    fn supported_proposition_without_evidence_reference_is_incomplete() {
        let mut case = positive_case();
        case.obligations[2].evidence_refs.clear();
        assert_eq!(disposition(&case), Disposition::Incomplete);
    }

    #[test]
    fn mandatory_producer_authority_cannot_be_waived() {
        let mut case = positive_case();
        case.obligations[1].authority = Authority::NotRequired;
        assert_eq!(disposition(&case), Disposition::Incomplete);
    }

    #[test]
    fn definitely_not_yet_effective_is_distinct_from_late_receiver_knowledge() {
        let mut case = positive_case();
        case.subject_time_effectivity = SubjectEffectivity::DefinitelyNotYetEffective;
        assert_eq!(
            disposition(&case),
            Disposition::NotEffectiveAtSubjectTime
        );
        case.obligations[3].temporal_knowledge = Temporal::NotKnownAtCutoff;
        assert_eq!(disposition(&case), Disposition::NotKnownAtSnapshot);
    }

    #[test]
    fn ambiguous_subject_effectivity_abstains() {
        let mut case = positive_case();
        case.subject_time_effectivity = SubjectEffectivity::BoundaryAmbiguous;
        assert_eq!(disposition(&case), Disposition::Incomplete);
    }

    #[test]
    fn malformed_subject_effectivity_interval_rejects() {
        let mut case = positive_case();
        case.subject_time_effectivity = SubjectEffectivity::InvalidInterval;
        assert_eq!(disposition(&case), Disposition::Rejected);
    }

    #[test]
    fn intrinsic_identity_gate_cannot_be_downgraded_to_optional() {
        let mut case = positive_case();
        let identity = case
            .obligations
            .iter_mut()
            .find(|item| item.kind == Kind::ClaimIdentity)
            .expect("identity obligation");
        identity.mandatory = false;
        identity.hard_gate = false;
        let result = evaluate_assurance_state_case_v0_1(&case);
        assert_eq!(result.disposition, Disposition::Unverified);
        assert!(result.reason_codes.iter().any(|code| {
            code == "INTRINSIC_HARD_GATE_POLICY_MISSING_OR_DOWNGRADED"
        }));
    }

    #[test]
    fn omission_of_intrinsic_producer_authority_gate_fails_closed() {
        let mut case = positive_case();
        case.obligations
            .retain(|item| item.kind != Kind::ProducerAuthority);
        assert_eq!(disposition(&case), Disposition::Unverified);
    }

    #[test]
    fn invalid_trust_policy_digest_fails_closed() {
        let mut case = positive_case();
        case.claim_context.trust_policy_snapshot_digest = "not-a-digest".to_owned();
        assert_eq!(disposition(&case), Disposition::Incomplete);
    }

    #[test]
    fn unsupported_hard_gate_without_evidence_reference_is_not_rejected() {
        let mut case = positive_case();
        case.obligations[0].proposition_evidence = Evidence::RefutedOnly;
        case.obligations[0].evidence_refs.clear();
        assert_eq!(disposition(&case), Disposition::Incomplete);
    }

    #[test]
    fn unrun_verification_with_absent_evidence_is_incomplete() {
        let mut case = positive_case();
        let item = &mut case.obligations[2];
        item.proposition_evidence = Evidence::Neither;
        item.verification = Verification::NotRun;
        item.evidence_refs.clear();
        assert_eq!(disposition(&case), Disposition::Incomplete);
    }

    #[test]
    fn unrun_verification_with_present_but_unverified_evidence_is_unverified() {
        let mut case = positive_case();
        let item = &mut case.obligations[2];
        item.proposition_evidence = Evidence::Neither;
        item.verification = Verification::NotRun;
        assert_eq!(disposition(&case), Disposition::Unverified);
    }

    #[test]
    fn published_synthetic_vectors_deserialize_and_match_the_proposed_projection() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/research/aviation/examples/aero_assurance_state_vectors_v0_1.json"
        ))
        .expect("published state-vector fixture is valid JSON");
        assert_eq!(
            fixture["schema"].as_str(),
            Some("AeroAssuranceStateVectorV0_1")
        );
        let cases = fixture["cases"]
            .as_array()
            .expect("state-vector fixture contains cases");
        assert_eq!(cases.len(), 10);

        for value in cases {
            let mut value = value.clone();
            let expected_disposition = value
                .as_object_mut()
                .expect("case is an object")
                .remove("expected_disposition")
                .and_then(|value| value.as_str().map(str::to_owned))
                .expect("expected disposition is present");
            let expected_blockers = value
                .as_object_mut()
                .expect("case is an object")
                .remove("expected_blocking_obligation_ids")
                .and_then(|value| value.as_array().cloned())
                .expect("expected blockers are present")
                .into_iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect::<Vec<_>>();
            let expected_reasons = value
                .as_object_mut()
                .expect("case is an object")
                .remove("expected_reason_codes")
                .and_then(|value| value.as_array().cloned())
                .expect("expected reasons are present")
                .into_iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect::<Vec<_>>();
            let case: Case =
                serde_json::from_value(value).expect("synthetic vector matches the Rust contract");
            let assessment = evaluate_assurance_state_case_v0_1(&case);

            assert_eq!(
                assessment.disposition.as_str(),
                expected_disposition,
                "case {}",
                case.case_id
            );
            let actual_blockers = assessment
                .blocking_obligations
                .iter()
                .map(|item| item.obligation_id.clone())
                .collect::<Vec<_>>();
            assert_eq!(actual_blockers, expected_blockers, "case {}", case.case_id);
            for expected_reason in expected_reasons {
                assert!(
                    assessment.reason_codes.contains(&expected_reason),
                    "case {} missing reason {}",
                    case.case_id,
                    expected_reason
                );
            }
        }
    }
}
