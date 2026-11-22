// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Three-valued evidence verdicts for scoped mobility assurance claims.
//!
//! This is deliberately separate from physical outcome classification and
//! recovery/action authority. It never returns a permit, ticket, capability,
//! or dispatch decision.
//!
//! A subject's own supporting assertion cannot confirm a claim. A current,
//! scope-matching contradiction from a valid subject assertion or an external
//! observation can refute it. Confirmation requires at least one current,
//! scope-matching support observation from every independently declared
//! required failure domain.

pub const CONTRACT_VERSION: &str = "0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Commitment(pub [u8; 32]);

impl Commitment {
    fn is_nonzero(self) -> bool {
        self.0.iter().any(|byte| *byte != 0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceSourceKind {
    SubjectAssertion,
    IndependentObservation,
    HarnessObservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceRelation {
    Supports,
    Contradicts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceVerdict {
    Confirmed,
    Refuted,
    InsufficientEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClaimProfile {
    pub claim_id: String,
    /// Commitment to the exact claim scope: platform, profile, configuration,
    /// transition, fault envelope and validity period as applicable.
    pub scope_commitment: Commitment,
    pub subject_failure_domain_id: String,
    /// All listed domains must independently support the claim before it can
    /// be CONFIRMED. The list is declared by the evaluator profile.
    pub required_independent_domains: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceFact {
    pub claim_id: String,
    pub scope_commitment: Commitment,
    pub evidence_id: String,
    pub source_id: String,
    pub failure_domain_id: String,
    pub source_kind: EvidenceSourceKind,
    pub relation: EvidenceRelation,
    /// Owner/evaluator assessment that this evidence remains current at the
    /// evaluation boundary. This crate does not establish the truth of it.
    pub current: bool,
    /// Owner/evaluator assessment that the evidence's applicability matches
    /// the exact claim profile. Commitment equality is checked as well.
    pub scope_matches: bool,
    pub raw_evidence_commitment: Commitment,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FactValidity {
    Valid,
    Invalid,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn profile_is_valid(profile: &ClaimProfile) -> bool {
    if !nonempty(&profile.claim_id)
        || !nonempty(&profile.subject_failure_domain_id)
        || !profile.scope_commitment.is_nonzero()
        || profile.required_independent_domains.is_empty()
    {
        return false;
    }

    for (index, domain) in profile.required_independent_domains.iter().enumerate() {
        if !nonempty(domain)
            || domain == &profile.subject_failure_domain_id
            || profile.required_independent_domains[..index]
                .iter()
                .any(|prior| prior == domain)
        {
            return false;
        }
    }
    true
}

fn fact_validity(profile: &ClaimProfile, fact: &EvidenceFact) -> FactValidity {
    if !nonempty(&fact.claim_id)
        || !nonempty(&fact.evidence_id)
        || !nonempty(&fact.source_id)
        || !nonempty(&fact.failure_domain_id)
        || !fact.raw_evidence_commitment.is_nonzero()
        || fact.claim_id != profile.claim_id
        || fact.scope_commitment != profile.scope_commitment
        || !fact.current
        || !fact.scope_matches
    {
        return FactValidity::Invalid;
    }

    let source_domain_matches = match fact.source_kind {
        EvidenceSourceKind::SubjectAssertion => {
            fact.failure_domain_id == profile.subject_failure_domain_id
        }
        EvidenceSourceKind::IndependentObservation | EvidenceSourceKind::HarnessObservation => {
            fact.failure_domain_id != profile.subject_failure_domain_id
        }
    };

    if source_domain_matches {
        FactValidity::Valid
    } else {
        FactValidity::Invalid
    }
}

/// Derive a scoped three-valued evidence verdict.
///
/// Rules:
/// - A malformed profile or duplicate evidence identity is insufficient.
/// - A valid, current, scope-matching contradiction refutes the claim. This
///   includes a subject's own contradictory assertion ("can convict").
/// - Subject support never confirms a claim.
/// - Confirmation requires a valid support fact from every required
///   independent failure domain.
/// - Missing, stale, mismatched, untrusted-by-profile or incomplete evidence
///   yields InsufficientEvidence.
///
/// No EvidenceVerdict is an action authority or a physical safety certificate.
pub fn assess_evidence_claim(
    profile: &ClaimProfile,
    facts: &[EvidenceFact],
) -> EvidenceVerdict {
    if !profile_is_valid(profile) {
        return EvidenceVerdict::InsufficientEvidence;
    }

    for (index, fact) in facts.iter().enumerate() {
        if facts[..index]
            .iter()
            .any(|prior| prior.evidence_id == fact.evidence_id)
        {
            return EvidenceVerdict::InsufficientEvidence;
        }
    }

    for fact in facts {
        if fact_validity(profile, fact) == FactValidity::Valid
            && fact.relation == EvidenceRelation::Contradicts
        {
            return EvidenceVerdict::Refuted;
        }
    }

    let all_required_domains_support = profile.required_independent_domains.iter().all(|domain| {
        facts.iter().any(|fact| {
            fact_validity(profile, fact) == FactValidity::Valid
                && fact.failure_domain_id == *domain
                && matches!(
                    fact.source_kind,
                    EvidenceSourceKind::IndependentObservation
                        | EvidenceSourceKind::HarnessObservation
                )
                && fact.relation == EvidenceRelation::Supports
        })
    });

    if all_required_domains_support {
        EvidenceVerdict::Confirmed
    } else {
        EvidenceVerdict::InsufficientEvidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commitment(byte: u8) -> Commitment {
        Commitment([byte; 32])
    }

    fn profile() -> ClaimProfile {
        ClaimProfile {
            claim_id: "claim-downstream-quiescence".to_owned(),
            scope_commitment: commitment(1),
            subject_failure_domain_id: "robot-under-test".to_owned(),
            required_independent_domains: vec![
                "read-only-bus-observer".to_owned(),
                "independent-encoder-observer".to_owned(),
            ],
        }
    }

    fn fact(
        profile: &ClaimProfile,
        evidence_id: &str,
        domain: &str,
        source_kind: EvidenceSourceKind,
        relation: EvidenceRelation,
    ) -> EvidenceFact {
        EvidenceFact {
            claim_id: profile.claim_id.clone(),
            scope_commitment: profile.scope_commitment,
            evidence_id: evidence_id.to_owned(),
            source_id: format!("source-{evidence_id}"),
            failure_domain_id: domain.to_owned(),
            source_kind,
            relation,
            current: true,
            scope_matches: true,
            raw_evidence_commitment: commitment(2),
        }
    }

    fn complete_support(profile: &ClaimProfile) -> Vec<EvidenceFact> {
        vec![
            fact(
                profile,
                "evidence-bus",
                "read-only-bus-observer",
                EvidenceSourceKind::IndependentObservation,
                EvidenceRelation::Supports,
            ),
            fact(
                profile,
                "evidence-encoder",
                "independent-encoder-observer",
                EvidenceSourceKind::HarnessObservation,
                EvidenceRelation::Supports,
            ),
        ]
    }

    #[test]
    fn all_required_independent_domains_confirm_the_scoped_claim() {
        let profile = profile();
        assert_eq!(
            assess_evidence_claim(&profile, &complete_support(&profile)),
            EvidenceVerdict::Confirmed
        );
    }

    #[test]
    fn subject_support_alone_never_confirms() {
        let profile = profile();
        let facts = vec![fact(
            &profile,
            "subject-says-safe",
            "robot-under-test",
            EvidenceSourceKind::SubjectAssertion,
            EvidenceRelation::Supports,
        )];
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn subject_contradiction_can_refute_but_cannot_confirm() {
        let profile = profile();
        let facts = vec![fact(
            &profile,
            "subject-admits-violation",
            "robot-under-test",
            EvidenceSourceKind::SubjectAssertion,
            EvidenceRelation::Contradicts,
        )];
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::Refuted
        );
    }

    #[test]
    fn independent_contradiction_refutes_even_when_other_domains_are_missing() {
        let profile = profile();
        let facts = vec![fact(
            &profile,
            "old-command-observed",
            "read-only-bus-observer",
            EvidenceSourceKind::IndependentObservation,
            EvidenceRelation::Contradicts,
        )];
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::Refuted
        );
    }

    #[test]
    fn unexpected_independent_contradiction_can_refute() {
        let profile = profile();
        let facts = vec![fact(
            &profile,
            "unexpected-contradiction",
            "external-camera-observer",
            EvidenceSourceKind::IndependentObservation,
            EvidenceRelation::Contradicts,
        )];
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::Refuted
        );
    }

    #[test]
    fn one_missing_required_domain_is_insufficient() {
        let profile = profile();
        let facts = vec![fact(
            &profile,
            "evidence-bus",
            "read-only-bus-observer",
            EvidenceSourceKind::IndependentObservation,
            EvidenceRelation::Supports,
        )];
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn stale_contradiction_does_not_refute_a_current_scoped_claim() {
        let profile = profile();
        let mut stale = fact(
            &profile,
            "stale-contradiction",
            "read-only-bus-observer",
            EvidenceSourceKind::IndependentObservation,
            EvidenceRelation::Contradicts,
        );
        stale.current = false;
        assert_eq!(
            assess_evidence_claim(&profile, &[stale]),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn wrong_scope_contradiction_is_insufficient_not_refuted() {
        let profile = profile();
        let mut wrong_scope = fact(
            &profile,
            "wrong-scope-contradiction",
            "read-only-bus-observer",
            EvidenceSourceKind::IndependentObservation,
            EvidenceRelation::Contradicts,
        );
        wrong_scope.scope_commitment = commitment(3);
        assert_eq!(
            assess_evidence_claim(&profile, &[wrong_scope]),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn zero_raw_evidence_commitment_cannot_confirm_or_refute() {
        let profile = profile();
        let mut facts = complete_support(&profile);
        facts[0].raw_evidence_commitment = commitment(0);
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::InsufficientEvidence
        );

        let mut contradiction = fact(
            &profile,
            "no-bytes",
            "read-only-bus-observer",
            EvidenceSourceKind::IndependentObservation,
            EvidenceRelation::Contradicts,
        );
        contradiction.raw_evidence_commitment = commitment(0);
        assert_eq!(
            assess_evidence_claim(&profile, &[contradiction]),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn independent_source_cannot_claim_the_subject_failure_domain() {
        let profile = profile();
        let facts = vec![fact(
            &profile,
            "fake-independent",
            "robot-under-test",
            EvidenceSourceKind::IndependentObservation,
            EvidenceRelation::Supports,
        )];
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn subject_source_cannot_claim_an_independent_failure_domain() {
        let profile = profile();
        let facts = vec![fact(
            &profile,
            "mislabelled-subject",
            "read-only-bus-observer",
            EvidenceSourceKind::SubjectAssertion,
            EvidenceRelation::Supports,
        )];
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn repeated_evidence_identity_is_insufficient() {
        let profile = profile();
        let mut facts = complete_support(&profile);
        facts[1].evidence_id = facts[0].evidence_id.clone();
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn invalid_profiles_never_confirm() {
        let mut profile = profile();
        profile.required_independent_domains.clear();
        assert_eq!(
            assess_evidence_claim(&profile, &[]),
            EvidenceVerdict::InsufficientEvidence
        );

        let mut profile = profile();
        profile.required_independent_domains[1] =
            profile.required_independent_domains[0].clone();
        assert_eq!(
            assess_evidence_claim(&profile, &[]),
            EvidenceVerdict::InsufficientEvidence
        );

        let mut profile = profile();
        profile.scope_commitment = commitment(0);
        assert_eq!(
            assess_evidence_claim(&profile, &[]),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn evidence_for_another_claim_cannot_be_reused() {
        let profile = profile();
        let mut facts = complete_support(&profile);
        facts[0].claim_id = "different-claim".to_owned();
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn subject_support_does_not_replace_missing_independent_evidence() {
        let profile = profile();
        let mut facts = complete_support(&profile);
        facts.pop();
        facts.push(fact(
            &profile,
            "subject-support",
            "robot-under-test",
            EvidenceSourceKind::SubjectAssertion,
            EvidenceRelation::Supports,
        ));
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::InsufficientEvidence
        );
    }

    #[test]
    fn complete_external_support_remains_confirmed_with_subject_support_present() {
        let profile = profile();
        let mut facts = complete_support(&profile);
        facts.push(fact(
            &profile,
            "subject-support",
            "robot-under-test",
            EvidenceSourceKind::SubjectAssertion,
            EvidenceRelation::Supports,
        ));
        assert_eq!(
            assess_evidence_claim(&profile, &facts),
            EvidenceVerdict::Confirmed
        );
    }
}
