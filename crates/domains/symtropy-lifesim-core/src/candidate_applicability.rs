// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Applicability annotation for bounded information-transition candidates.
//!
//! Candidate discovery answers "which structural paths exist inside the admitted
//! search envelope?" Applicability policy answers "which of those path edges are
//! universal versus state-conditioned?" This module joins those read-only facts
//! without filtering or selecting a winner.
//!
//! A returned `TransitionDomainKey` remains an unresolved authority requirement.
//! It is never evidence that the current canonical state satisfies the domain.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use crate::information::{RepresentationKey};
use crate::information_registry::InformationPolicyRegistryKey;
use crate::information_transition::{InformationTransitionKey, InformationTransitionRegistryKey};
use crate::transition_applicability::{
    TransitionApplicability, TransitionApplicabilityPolicy, TransitionApplicabilityPolicyKey,
    TransitionDomainKey,
};
use crate::transition_candidates::{
    TransitionCandidatePath, TransitionCandidateSearchLimits, TransitionCandidateSet,
};

/// One discovered structural candidate annotated with the unresolved state-domain
/// requirements imposed by one exact semantic applicability-policy generation.
///
/// This type is not commit-ready authority and is not a selection result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicableTransitionCandidate {
    applicability_policy_key: TransitionApplicabilityPolicyKey,
    candidate: TransitionCandidatePath,
    required_domains: BTreeSet<TransitionDomainKey>,
}

impl ApplicableTransitionCandidate {
    pub const fn applicability_policy_key(&self) -> TransitionApplicabilityPolicyKey {
        self.applicability_policy_key
    }

    pub const fn transition_registry_key(&self) -> InformationTransitionRegistryKey {
        self.candidate.transition_registry_key()
    }

    pub const fn policy_registry_key(&self) -> InformationPolicyRegistryKey {
        self.candidate.policy_registry_key()
    }

    pub const fn source(&self) -> RepresentationKey {
        self.candidate.source()
    }

    pub const fn destination(&self) -> RepresentationKey {
        self.candidate.destination()
    }

    pub fn candidate(&self) -> &TransitionCandidatePath {
        &self.candidate
    }

    pub fn transitions(&self) -> &[InformationTransitionKey] {
        self.candidate.transitions()
    }

    pub fn required_domains(&self) -> &BTreeSet<TransitionDomainKey> {
        &self.required_domains
    }

    /// True only when every edge on this candidate is explicitly Universal.
    /// This does not resolve R0/R1/R2/R3/R4, closure, spatiotemporal, or other
    /// authority requirements carried by the candidate.
    pub fn is_unconditionally_applicable(&self) -> bool {
        self.required_domains.is_empty()
    }
}

/// A complete bounded candidate set after applicability annotation.
///
/// Candidate ordering exactly preserves discovery ordering for replay/inspection.
/// It is explicitly not a canonical ranking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicableTransitionCandidateSet {
    applicability_policy_key: TransitionApplicabilityPolicyKey,
    transition_registry_key: InformationTransitionRegistryKey,
    policy_registry_key: InformationPolicyRegistryKey,
    source: RepresentationKey,
    destination: RepresentationKey,
    search_limits: TransitionCandidateSearchLimits,
    candidates: Vec<ApplicableTransitionCandidate>,
}

impl ApplicableTransitionCandidateSet {
    pub const fn applicability_policy_key(&self) -> TransitionApplicabilityPolicyKey {
        self.applicability_policy_key
    }

    pub const fn transition_registry_key(&self) -> InformationTransitionRegistryKey {
        self.transition_registry_key
    }

    pub const fn policy_registry_key(&self) -> InformationPolicyRegistryKey {
        self.policy_registry_key
    }

    pub const fn source(&self) -> RepresentationKey {
        self.source
    }

    pub const fn destination(&self) -> RepresentationKey {
        self.destination
    }

    pub const fn search_limits(&self) -> TransitionCandidateSearchLimits {
        self.search_limits
    }

    pub fn candidates(&self) -> &[ApplicableTransitionCandidate] {
        &self.candidates
    }
}

impl TransitionApplicabilityPolicy {
    /// Annotate one discovered candidate with every unresolved domain descriptor
    /// required by its exact ordered transition sequence.
    pub fn annotate_candidate(
        &self,
        candidate: &TransitionCandidatePath,
    ) -> Result<ApplicableTransitionCandidate, CandidateApplicabilityError> {
        if candidate.transition_registry_key() != self.transition_registry_key() {
            return Err(CandidateApplicabilityError::TransitionRegistryKeyMismatch {
                expected: self.transition_registry_key(),
                actual: candidate.transition_registry_key(),
            });
        }

        let mut required_domains = BTreeSet::new();
        for transition in candidate.transitions() {
            let applicability = self.applicability(*transition).ok_or(
                CandidateApplicabilityError::MissingApplicability {
                    transition: *transition,
                },
            )?;
            if let TransitionApplicability::RegisteredDomain(domain) = applicability {
                required_domains.insert(domain);
            }
        }

        Ok(ApplicableTransitionCandidate {
            applicability_policy_key: self.key(),
            candidate: candidate.clone(),
            required_domains,
        })
    }

    /// Annotate every candidate in a complete bounded discovery set.
    ///
    /// This preserves discovery order exactly and never filters candidates whose
    /// domains remain unresolved. Filtering belongs to the later authority/evidence
    /// layer after #307 proofs are evaluated.
    pub fn annotate_candidate_set(
        &self,
        set: &TransitionCandidateSet,
    ) -> Result<ApplicableTransitionCandidateSet, CandidateApplicabilityError> {
        if set.transition_registry_key() != self.transition_registry_key() {
            return Err(CandidateApplicabilityError::TransitionRegistryKeyMismatch {
                expected: self.transition_registry_key(),
                actual: set.transition_registry_key(),
            });
        }

        let candidates = set
            .candidates()
            .iter()
            .map(|candidate| self.annotate_candidate(candidate))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ApplicableTransitionCandidateSet {
            applicability_policy_key: self.key(),
            transition_registry_key: set.transition_registry_key(),
            policy_registry_key: set.policy_registry_key(),
            source: set.source(),
            destination: set.destination(),
            search_limits: set.limits(),
            candidates,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateApplicabilityError {
    TransitionRegistryKeyMismatch {
        expected: InformationTransitionRegistryKey,
        actual: InformationTransitionRegistryKey,
    },
    MissingApplicability {
        transition: InformationTransitionKey,
    },
}

impl fmt::Display for CandidateApplicabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TransitionRegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "applicability policy is bound to transition registry {expected:?}, candidate uses {actual:?}"
            ),
            Self::MissingApplicability { transition } => write!(
                formatter,
                "candidate transition {transition:?} has no applicability declaration"
            ),
        }
    }
}

impl Error for CandidateApplicabilityError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
        RepresentationCapabilities,
    };
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionRegistry,
        InformationTransitionRegistryBuilder,
    };
    use crate::transition_applicability::{
        TransitionApplicabilityPolicyBuilder, TransitionApplicabilityPolicyKey,
    };

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(900, 1);
    const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(901, 1);
    const APP: TransitionApplicabilityPolicyKey = TransitionApplicabilityPolicyKey::new(902, 1);

    const A: RepresentationKey = RepresentationKey::new(910, 1);
    const B: RepresentationKey = RepresentationKey::new(911, 1);
    const C: RepresentationKey = RepresentationKey::new(912, 1);
    const D: RepresentationKey = RepresentationKey::new(913, 1);

    const AB: InformationTransitionKey = InformationTransitionKey::new(920, 1);
    const BD: InformationTransitionKey = InformationTransitionKey::new(921, 1);
    const AC: InformationTransitionKey = InformationTransitionKey::new(922, 1);
    const CD: InformationTransitionKey = InformationTransitionKey::new(923, 1);

    const DOMAIN_AB: TransitionDomainKey = TransitionDomainKey::new(930, 1);
    const DOMAIN_CD: TransitionDomainKey = TransitionDomainKey::new(931, 1);

    fn information_policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        for representation in [A, B, C, D] {
            builder
                .register_representation(RepresentationCapabilities::new(
                    representation,
                    EcologicalAuthorityLevel::Coarse,
                    [(
                        EcologicalInformation::Headcount,
                        CapabilityEvidence::Exact,
                    )],
                ))
                .unwrap();
        }
        builder.seal()
    }

    fn edge(
        key: InformationTransitionKey,
        source: RepresentationKey,
        destination: RepresentationKey,
    ) -> InformationTransitionDefinition {
        InformationTransitionDefinition::new(key, source, destination, [], []).unwrap()
    }

    fn transition_registry(
        policy: &InformationPolicyRegistry,
    ) -> InformationTransitionRegistry {
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
        for transition in [
            edge(CD, C, D),
            edge(BD, B, D),
            edge(AC, A, C),
            edge(AB, A, B),
        ] {
            builder.register_transition(transition).unwrap();
        }
        builder.seal(policy).unwrap()
    }

    fn applicability_policy(
        transitions: &InformationTransitionRegistry,
    ) -> TransitionApplicabilityPolicy {
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APP);
        builder
            .register(AB, TransitionApplicability::RegisteredDomain(DOMAIN_AB))
            .unwrap();
        builder.register(BD, TransitionApplicability::Universal).unwrap();
        builder.register(AC, TransitionApplicability::Universal).unwrap();
        builder
            .register(CD, TransitionApplicability::RegisteredDomain(DOMAIN_CD))
            .unwrap();
        builder.seal(transitions).unwrap()
    }

    #[test]
    fn every_discovered_candidate_gets_its_own_domain_requirements() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let applicability = applicability_policy(&transitions);
        let discovered = transitions
            .discover_candidates(
                &information,
                A,
                D,
                TransitionCandidateSearchLimits::new(3, 8).unwrap(),
            )
            .unwrap();

        let annotated = applicability.annotate_candidate_set(&discovered).unwrap();

        assert_eq!(annotated.candidates().len(), 2);
        assert_eq!(annotated.candidates()[0].transitions(), &[AB, BD]);
        assert_eq!(
            annotated.candidates()[0].required_domains(),
            &BTreeSet::from([DOMAIN_AB])
        );
        assert_eq!(annotated.candidates()[1].transitions(), &[AC, CD]);
        assert_eq!(
            annotated.candidates()[1].required_domains(),
            &BTreeSet::from([DOMAIN_CD])
        );
    }

    #[test]
    fn annotation_preserves_discovery_order_but_does_not_select_a_winner() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let applicability = applicability_policy(&transitions);
        let discovered = transitions
            .discover_candidates(
                &information,
                A,
                D,
                TransitionCandidateSearchLimits::new(3, 8).unwrap(),
            )
            .unwrap();
        let before = discovered
            .candidates()
            .iter()
            .map(|candidate| candidate.transitions().to_vec())
            .collect::<Vec<_>>();

        let annotated = applicability.annotate_candidate_set(&discovered).unwrap();
        let after = annotated
            .candidates()
            .iter()
            .map(|candidate| candidate.transitions().to_vec())
            .collect::<Vec<_>>();

        assert_eq!(before, after);
        assert_eq!(annotated.candidates().len(), 2);
    }

    #[test]
    fn domain_descriptor_remains_unresolved_after_annotation() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let applicability = applicability_policy(&transitions);
        let discovered = transitions
            .discover_candidates(
                &information,
                A,
                D,
                TransitionCandidateSearchLimits::new(3, 8).unwrap(),
            )
            .unwrap();
        let annotated = applicability.annotate_candidate_set(&discovered).unwrap();

        assert!(!annotated.candidates()[0].is_unconditionally_applicable());
        assert!(!annotated.candidates()[1].is_unconditionally_applicable());
    }

    #[test]
    fn identity_candidate_is_unconditionally_applicable() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let applicability = applicability_policy(&transitions);
        let discovered = transitions
            .discover_candidates(
                &information,
                A,
                A,
                TransitionCandidateSearchLimits::new(3, 8).unwrap(),
            )
            .unwrap();
        let annotated = applicability.annotate_candidate_set(&discovered).unwrap();

        assert_eq!(annotated.candidates().len(), 1);
        assert!(annotated.candidates()[0].is_unconditionally_applicable());
    }

    #[test]
    fn candidate_from_different_transition_registry_fails_closed() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let applicability = applicability_policy(&transitions);

        let mut other_builder = InformationTransitionRegistryBuilder::new(
            InformationTransitionRegistryKey::new(999, 1),
        );
        other_builder.register_transition(edge(AB, A, B)).unwrap();
        let other = other_builder.seal(&information).unwrap();
        let discovered = other
            .discover_candidates(
                &information,
                A,
                B,
                TransitionCandidateSearchLimits::new(2, 4).unwrap(),
            )
            .unwrap();

        assert!(matches!(
            applicability.annotate_candidate_set(&discovered),
            Err(CandidateApplicabilityError::TransitionRegistryKeyMismatch { .. })
        ));
    }
}
