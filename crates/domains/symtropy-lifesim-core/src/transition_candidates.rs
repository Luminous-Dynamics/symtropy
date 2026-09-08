// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic bounded discovery of information-transition candidates.
//!
//! `information_transition::InformationTransitionRegistry::plan` freezes one
//! structural shortest-path behavior. That remains a useful primitive, but raw
//! edge count must not become canonical ecological fidelity policy. This module
//! exposes the independent discovery step: enumerate every simple
//! source->destination path inside an explicit bounded search envelope, summarize
//! each path's authority/evidence consequences, and leave semantic selection to a
//! higher policy layer.
//!
//! If the configured envelope could hide an additional simple path, discovery
//! fails closed rather than returning an incomplete candidate set as though it
//! were exhaustive.

use std::collections::{BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

use crate::information::{EcologicalInformation, EvidenceLineageToken, RepresentationKey};
use crate::information_registry::{
    InformationPolicyRegistry, InformationPolicyRegistryKey, InformationRegistryError,
};
use crate::information_transition::{
    ConditionalDerivationKey, InformationTransitionKey, InformationTransitionRegistry,
    InformationTransitionRegistryKey, PromotionEvidenceRequirement, PromotionProvenance,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionCandidateSearchLimits {
    max_edges: usize,
    max_candidates: usize,
}

impl TransitionCandidateSearchLimits {
    pub fn new(max_edges: usize, max_candidates: usize) -> Result<Self, TransitionCandidateError> {
        if max_edges == 0 {
            return Err(TransitionCandidateError::ZeroMaxEdges);
        }
        if max_candidates == 0 {
            return Err(TransitionCandidateError::ZeroMaxCandidates);
        }
        Ok(Self {
            max_edges,
            max_candidates,
        })
    }

    pub const fn max_edges(self) -> usize {
        self.max_edges
    }

    pub const fn max_candidates(self) -> usize {
        self.max_candidates
    }
}

/// One structurally valid path plus the authority/evidence consequences implied
/// by its registered edges. This is a candidate, not a canonical selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionCandidatePath {
    transition_registry_key: InformationTransitionRegistryKey,
    policy_registry_key: InformationPolicyRegistryKey,
    source: RepresentationKey,
    destination: RepresentationKey,
    transitions: Vec<InformationTransitionKey>,
    external_requirements: BTreeSet<PromotionEvidenceRequirement>,
    closure_evidence: BTreeSet<EvidenceLineageToken>,
    conditional_derivations: BTreeSet<ConditionalDerivationKey>,
    discarded_information: BTreeSet<EcologicalInformation>,
}

impl TransitionCandidatePath {
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

    pub fn transitions(&self) -> &[InformationTransitionKey] {
        &self.transitions
    }

    pub fn edge_count(&self) -> usize {
        self.transitions.len()
    }

    pub fn external_requirements(&self) -> &BTreeSet<PromotionEvidenceRequirement> {
        &self.external_requirements
    }

    pub fn closure_evidence(&self) -> &BTreeSet<EvidenceLineageToken> {
        &self.closure_evidence
    }

    pub fn conditional_derivations(&self) -> &BTreeSet<ConditionalDerivationKey> {
        &self.conditional_derivations
    }

    pub fn discarded_information(&self) -> &BTreeSet<EcologicalInformation> {
        &self.discarded_information
    }

    /// Structural summary only. Even a candidate with no visible debt here may
    /// still require applicability/domain or later policy validation.
    pub fn preserves_information_without_external_or_derived_requirements(&self) -> bool {
        self.external_requirements.is_empty()
            && self.closure_evidence.is_empty()
            && self.conditional_derivations.is_empty()
            && self.discarded_information.is_empty()
    }
}

/// Complete candidate set under a search envelope that did not truncate any
/// additional simple path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionCandidateSet {
    transition_registry_key: InformationTransitionRegistryKey,
    policy_registry_key: InformationPolicyRegistryKey,
    source: RepresentationKey,
    destination: RepresentationKey,
    limits: TransitionCandidateSearchLimits,
    candidates: Vec<TransitionCandidatePath>,
}

impl TransitionCandidateSet {
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

    pub const fn limits(&self) -> TransitionCandidateSearchLimits {
        self.limits
    }

    pub fn candidates(&self) -> &[TransitionCandidatePath] {
        &self.candidates
    }
}

#[derive(Debug, Clone)]
struct SearchNode {
    current: RepresentationKey,
    transitions: Vec<InformationTransitionKey>,
    visited: BTreeSet<RepresentationKey>,
}

impl InformationTransitionRegistry {
    /// Discover every simple source->destination path admitted by `limits`.
    ///
    /// Candidate order is canonicalized by `(edge_count, transition-key sequence)`
    /// only for stable replay. That ordering is explicitly **not** semantic
    /// preference; canonical fidelity selection belongs to #272/#308.
    pub fn discover_candidates(
        &self,
        policy: &InformationPolicyRegistry,
        source: RepresentationKey,
        destination: RepresentationKey,
        limits: TransitionCandidateSearchLimits,
    ) -> Result<TransitionCandidateSet, TransitionCandidateError> {
        if policy.key() != self.policy_registry_key() {
            return Err(TransitionCandidateError::PolicyRegistryKeyMismatch {
                expected: self.policy_registry_key(),
                actual: policy.key(),
            });
        }

        policy
            .resolve_representation(source)
            .map_err(TransitionCandidateError::Policy)?;
        policy
            .resolve_representation(destination)
            .map_err(TransitionCandidateError::Policy)?;

        if source == destination {
            return Ok(TransitionCandidateSet {
                transition_registry_key: self.key(),
                policy_registry_key: self.policy_registry_key(),
                source,
                destination,
                limits,
                candidates: vec![self.summarize_candidate(source, destination, Vec::new())],
            });
        }

        let mut queue = VecDeque::from([SearchNode {
            current: source,
            transitions: Vec::new(),
            visited: BTreeSet::from([source]),
        }]);
        let mut candidates = Vec::new();

        while let Some(node) = queue.pop_front() {
            let outgoing = self
                .transitions()
                .filter(|(_, transition)| transition.source() == node.current)
                .filter(|(_, transition)| !node.visited.contains(&transition.destination()))
                .map(|(key, transition)| (*key, transition.destination()))
                .collect::<Vec<_>>();

            if node.transitions.len() >= limits.max_edges {
                if !outgoing.is_empty() {
                    return Err(TransitionCandidateError::SearchEdgeLimitExceeded {
                        max_edges: limits.max_edges,
                    });
                }
                continue;
            }

            for (transition, next) in outgoing {
                let mut path = node.transitions.clone();
                path.push(transition);

                if next == destination {
                    if candidates.len() >= limits.max_candidates {
                        return Err(TransitionCandidateError::SearchCandidateLimitExceeded {
                            max_candidates: limits.max_candidates,
                        });
                    }
                    candidates.push(self.summarize_candidate(source, destination, path));
                    continue;
                }

                let mut visited = node.visited.clone();
                visited.insert(next);
                queue.push_back(SearchNode {
                    current: next,
                    transitions: path,
                    visited,
                });
            }
        }

        if candidates.is_empty() {
            return Err(TransitionCandidateError::NoCandidatePath {
                source,
                destination,
            });
        }

        candidates.sort_by(|left, right| {
            left.edge_count()
                .cmp(&right.edge_count())
                .then_with(|| left.transitions.cmp(&right.transitions))
        });

        Ok(TransitionCandidateSet {
            transition_registry_key: self.key(),
            policy_registry_key: self.policy_registry_key(),
            source,
            destination,
            limits,
            candidates,
        })
    }

    fn summarize_candidate(
        &self,
        source: RepresentationKey,
        destination: RepresentationKey,
        transitions: Vec<InformationTransitionKey>,
    ) -> TransitionCandidatePath {
        let mut external_requirements = BTreeSet::new();
        let mut closure_evidence = BTreeSet::new();
        let mut conditional_derivations = BTreeSet::new();
        let mut discarded_information = BTreeSet::new();

        for key in &transitions {
            let transition = self
                .transitions()
                .find_map(|(registered_key, definition)| {
                    (*registered_key == *key).then_some(definition)
                })
                .expect("candidate transition remains registered");

            discarded_information.extend(transition.discarded_information().iter().copied());
            for provenance in transition.introductions().values().copied() {
                match provenance {
                    PromotionProvenance::RetainedExact { authority } => {
                        external_requirements
                            .insert(PromotionEvidenceRequirement::RetainedAuthority(authority));
                    }
                    PromotionProvenance::LosslessDerivation { transform } => {
                        external_requirements
                            .insert(PromotionEvidenceRequirement::LosslessTransform(transform));
                    }
                    PromotionProvenance::QualifiedClosure { evidence_lineage } => {
                        closure_evidence.insert(evidence_lineage);
                    }
                    PromotionProvenance::MeasurementAssimilation { authority } => {
                        external_requirements.insert(
                            PromotionEvidenceRequirement::MeasurementAuthority(authority),
                        );
                    }
                    PromotionProvenance::ConditionalMicrostate { model } => {
                        conditional_derivations.insert(model);
                    }
                }
            }
        }

        TransitionCandidatePath {
            transition_registry_key: self.key(),
            policy_registry_key: self.policy_registry_key(),
            source,
            destination,
            transitions,
            external_requirements,
            closure_evidence,
            conditional_derivations,
            discarded_information,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionCandidateError {
    Policy(InformationRegistryError),
    ZeroMaxEdges,
    ZeroMaxCandidates,
    PolicyRegistryKeyMismatch {
        expected: InformationPolicyRegistryKey,
        actual: InformationPolicyRegistryKey,
    },
    SearchEdgeLimitExceeded {
        max_edges: usize,
    },
    SearchCandidateLimitExceeded {
        max_candidates: usize,
    },
    NoCandidatePath {
        source: RepresentationKey,
        destination: RepresentationKey,
    },
}

impl fmt::Display for TransitionCandidateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(error) => write!(formatter, "information policy error: {error}"),
            Self::ZeroMaxEdges => {
                write!(formatter, "transition candidate search max_edges must be > 0")
            }
            Self::ZeroMaxCandidates => write!(
                formatter,
                "transition candidate search max_candidates must be > 0"
            ),
            Self::PolicyRegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "transition graph is bound to policy registry {expected:?}, got {actual:?}"
            ),
            Self::SearchEdgeLimitExceeded { max_edges } => write!(
                formatter,
                "candidate discovery reached max_edges={max_edges} while additional simple paths remained"
            ),
            Self::SearchCandidateLimitExceeded { max_candidates } => write!(
                formatter,
                "candidate discovery exceeded max_candidates={max_candidates}"
            ),
            Self::NoCandidatePath {
                source,
                destination,
            } => write!(
                formatter,
                "no transition candidate path from {source:?} to {destination:?}"
            ),
        }
    }
}

impl Error for TransitionCandidateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Policy(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
        RepresentationCapabilities,
    };
    use crate::information_registry::{
        InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionRegistryBuilder,
    };

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(80, 1);
    const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(81, 1);
    const A: RepresentationKey = RepresentationKey::new(100, 1);
    const B: RepresentationKey = RepresentationKey::new(101, 1);
    const C: RepresentationKey = RepresentationKey::new(102, 1);
    const D: RepresentationKey = RepresentationKey::new(103, 1);
    const AB: InformationTransitionKey = InformationTransitionKey::new(200, 1);
    const BD: InformationTransitionKey = InformationTransitionKey::new(201, 1);
    const AC: InformationTransitionKey = InformationTransitionKey::new(202, 1);
    const CD: InformationTransitionKey = InformationTransitionKey::new(203, 1);
    const BC: InformationTransitionKey = InformationTransitionKey::new(204, 1);

    fn policy() -> InformationPolicyRegistry {
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

    fn graph(policy: &InformationPolicyRegistry) -> InformationTransitionRegistry {
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

    #[test]
    fn discovers_multiple_candidates_without_selecting_one_as_canonical() {
        let policy = policy();
        let graph = graph(&policy);
        let limits = TransitionCandidateSearchLimits::new(3, 8).unwrap();
        let set = graph.discover_candidates(&policy, A, D, limits).unwrap();

        assert_eq!(set.candidates().len(), 2);
        assert_eq!(set.candidates()[0].transitions(), &[AB, BD]);
        assert_eq!(set.candidates()[1].transitions(), &[AC, CD]);
    }

    #[test]
    fn registration_order_does_not_change_candidate_order() {
        let policy = policy();
        let first = graph(&policy);

        let mut second_builder = InformationTransitionRegistryBuilder::new(GRAPH);
        for transition in [
            edge(AB, A, B),
            edge(AC, A, C),
            edge(BD, B, D),
            edge(CD, C, D),
        ] {
            second_builder.register_transition(transition).unwrap();
        }
        let second = second_builder.seal(&policy).unwrap();
        let limits = TransitionCandidateSearchLimits::new(3, 8).unwrap();

        assert_eq!(
            first.discover_candidates(&policy, A, D, limits).unwrap(),
            second.discover_candidates(&policy, A, D, limits).unwrap()
        );
    }

    #[test]
    fn candidate_limit_fails_closed_instead_of_returning_partial_search() {
        let policy = policy();
        let graph = graph(&policy);
        let limits = TransitionCandidateSearchLimits::new(3, 1).unwrap();

        assert_eq!(
            graph.discover_candidates(&policy, A, D, limits),
            Err(TransitionCandidateError::SearchCandidateLimitExceeded {
                max_candidates: 1
            })
        );
    }

    #[test]
    fn edge_limit_fails_closed_when_it_would_hide_a_longer_simple_path() {
        let policy = policy();
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
        for transition in [edge(AB, A, B), edge(BC, B, C), edge(CD, C, D)] {
            builder.register_transition(transition).unwrap();
        }
        let graph = builder.seal(&policy).unwrap();
        let limits = TransitionCandidateSearchLimits::new(2, 8).unwrap();

        assert_eq!(
            graph.discover_candidates(&policy, A, D, limits),
            Err(TransitionCandidateError::SearchEdgeLimitExceeded { max_edges: 2 })
        );
    }

    #[test]
    fn cycles_are_excluded_by_representation_identity() {
        let policy = policy();
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
        for transition in [
            edge(AB, A, B),
            edge(InformationTransitionKey::new(205, 1), B, A),
            edge(BD, B, D),
        ] {
            builder.register_transition(transition).unwrap();
        }
        let graph = builder.seal(&policy).unwrap();
        let limits = TransitionCandidateSearchLimits::new(4, 8).unwrap();
        let set = graph.discover_candidates(&policy, A, D, limits).unwrap();

        assert_eq!(set.candidates().len(), 1);
        assert_eq!(set.candidates()[0].transitions(), &[AB, BD]);
    }

    #[test]
    fn identity_has_one_zero_edge_candidate() {
        let policy = policy();
        let graph = graph(&policy);
        let limits = TransitionCandidateSearchLimits::new(3, 8).unwrap();
        let set = graph.discover_candidates(&policy, A, A, limits).unwrap();

        assert_eq!(set.candidates().len(), 1);
        assert_eq!(set.candidates()[0].edge_count(), 0);
        assert!(set.candidates()[0]
            .preserves_information_without_external_or_derived_requirements());
    }

    #[test]
    fn no_path_is_explicit_refusal() {
        let policy = policy();
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
        builder.register_transition(edge(AB, A, B)).unwrap();
        let graph = builder.seal(&policy).unwrap();
        let limits = TransitionCandidateSearchLimits::new(3, 8).unwrap();

        assert_eq!(
            graph.discover_candidates(&policy, A, D, limits),
            Err(TransitionCandidateError::NoCandidatePath {
                source: A,
                destination: D
            })
        );
    }
}
