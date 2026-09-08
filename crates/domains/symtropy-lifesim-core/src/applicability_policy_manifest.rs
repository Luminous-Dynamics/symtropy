// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact content identity for transition-applicability policy.
//!
//! The semantic applicability layer answers whether each transition edge is
//! `Universal` or requires one registered source-state domain. A semantic policy
//! key is not replay authority: the exact mapping and the exact transition graph
//! it governs must remain bound together.
//!
//! This module deliberately uses the complete canonical bytes as identity. A
//! higher layer may derive a compact cryptographic digest from these bytes using
//! a separately fixed and qualified digest profile.

use std::error::Error;
use std::fmt;

use crate::information::RepresentationKey;
use crate::information_transition::{InformationTransitionKey, InformationTransitionRegistryKey};
use crate::transition_policy_manifest::{
    InformationTransitionAuthorityStamp, ManifestBoundInformationTransitionRegistry,
    TransitionPolicyIdentityError,
};

// Reuse #306's exact semantic implementation byte-for-byte on the convergence
// branch. Re-exporting this nested module from crate root preserves the public
// semantic API while avoiding a conflict-only edit to the frozen parent heads.
#[path = "transition_applicability.rs"]
pub mod transition_applicability;

use transition_applicability::{
    ApplicableTransitionPlan, TransitionApplicability, TransitionApplicabilityError,
    TransitionApplicabilityPolicy, TransitionApplicabilityPolicyBuilder,
    TransitionApplicabilityPolicyKey, TransitionDomainKey,
};

/// Canonical encoding version for exact applicability-policy manifests.
pub const TRANSITION_APPLICABILITY_MANIFEST_VERSION: u32 = 1;

const MANIFEST_DOMAIN: &[u8] = b"SYMTROPY_TRANSITION_APPLICABILITY_MANIFEST\0";

/// Canonical exact byte identity for one applicability mapping under one exact
/// transition authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionApplicabilityManifest {
    bytes: Vec<u8>,
}

impl TransitionApplicabilityManifest {
    fn from_validated(
        policy: &TransitionApplicabilityPolicy,
        transitions: &ManifestBoundInformationTransitionRegistry<'_>,
    ) -> Result<Self, ApplicabilityPolicyIdentityError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MANIFEST_DOMAIN);
        push_u32(&mut bytes, TRANSITION_APPLICABILITY_MANIFEST_VERSION);
        encode_policy_key(&mut bytes, policy.key());

        let transition_bytes = transitions.authority_stamp().manifest().as_bytes();
        push_len(&mut bytes, transition_bytes.len());
        bytes.extend_from_slice(transition_bytes);

        let edges = transitions.registry().transitions().collect::<Vec<_>>();
        push_len(&mut bytes, edges.len());
        for (transition, _) in edges {
            encode_transition_key(&mut bytes, *transition);
            let applicability = policy.applicability(*transition).ok_or(
                ApplicabilityPolicyIdentityError::MissingApplicability {
                    transition: *transition,
                },
            )?;
            encode_applicability(&mut bytes, applicability);
        }

        Ok(Self { bytes })
    }

    pub const fn version(&self) -> u32 {
        TRANSITION_APPLICABILITY_MANIFEST_VERSION
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Exact applicability authority: semantic applicability key + exact transition
/// authority + exact canonical mapping manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionApplicabilityAuthorityStamp {
    key: TransitionApplicabilityPolicyKey,
    transition_authority: InformationTransitionAuthorityStamp,
    manifest: TransitionApplicabilityManifest,
}

impl TransitionApplicabilityAuthorityStamp {
    pub const fn key(&self) -> TransitionApplicabilityPolicyKey {
        self.key
    }

    pub const fn manifest_version(&self) -> u32 {
        TRANSITION_APPLICABILITY_MANIFEST_VERSION
    }

    pub const fn transition_authority(&self) -> &InformationTransitionAuthorityStamp {
        &self.transition_authority
    }

    pub const fn manifest(&self) -> &TransitionApplicabilityManifest {
        &self.manifest
    }

    /// Revalidate the semantic applicability mapping and exact transition corpus
    /// before a later authority boundary consumes prepared work.
    pub fn validate(
        &self,
        policy: &TransitionApplicabilityPolicy,
        transitions: &ManifestBoundInformationTransitionRegistry<'_>,
    ) -> Result<(), ApplicabilityPolicyIdentityError> {
        if policy.key() != self.key {
            return Err(ApplicabilityPolicyIdentityError::PolicyKeyMismatch {
                expected: self.key,
                actual: policy.key(),
            });
        }
        if transitions.authority_stamp() != &self.transition_authority {
            return Err(ApplicabilityPolicyIdentityError::TransitionAuthorityMismatch {
                key: self.transition_authority.key(),
            });
        }

        let current = ManifestBoundTransitionApplicabilityPolicy::new(policy, transitions)?;
        if current.authority != *self {
            return Err(ApplicabilityPolicyIdentityError::ManifestMismatch { key: self.key });
        }
        Ok(())
    }
}

/// Read-only exact-content view over one sealed #306 applicability policy.
#[derive(Debug, Clone)]
pub struct ManifestBoundTransitionApplicabilityPolicy<'a> {
    policy: &'a TransitionApplicabilityPolicy,
    transitions: ManifestBoundInformationTransitionRegistry<'a>,
    authority: TransitionApplicabilityAuthorityStamp,
}

impl<'a> ManifestBoundTransitionApplicabilityPolicy<'a> {
    /// Upgrade a semantic applicability policy into exact replay authority.
    ///
    /// The semantic mapping is reconstructed over the exact graph and sealed
    /// again before a stamp is produced. Absence never becomes Universal.
    pub fn new(
        policy: &'a TransitionApplicabilityPolicy,
        transitions: &ManifestBoundInformationTransitionRegistry<'a>,
    ) -> Result<Self, ApplicabilityPolicyIdentityError> {
        if policy.transition_registry_key() != transitions.authority_stamp().key() {
            return Err(ApplicabilityPolicyIdentityError::TransitionRegistryKeyMismatch {
                expected: policy.transition_registry_key(),
                actual: transitions.authority_stamp().key(),
            });
        }

        revalidate_policy(policy, transitions)?;
        let transition_authority = transitions.authority_stamp().clone();
        let manifest = TransitionApplicabilityManifest::from_validated(policy, transitions)?;
        let authority = TransitionApplicabilityAuthorityStamp {
            key: policy.key(),
            transition_authority,
            manifest,
        };

        Ok(Self {
            policy,
            transitions: transitions.clone(),
            authority,
        })
    }

    pub const fn policy(&self) -> &'a TransitionApplicabilityPolicy {
        self.policy
    }

    pub const fn transitions(&self) -> &ManifestBoundInformationTransitionRegistry<'a> {
        &self.transitions
    }

    pub const fn authority_stamp(&self) -> &TransitionApplicabilityAuthorityStamp {
        &self.authority
    }

    /// Produce a path carrying exact policy + transition + applicability authority.
    /// This remains structural/diagnostic path planning; canonical path selection
    /// belongs to the candidate/admissibility/selection stack.
    pub fn plan(
        &self,
        source: RepresentationKey,
        destination: RepresentationKey,
    ) -> Result<ManifestBoundApplicableTransitionPlan, ApplicabilityPolicyIdentityError> {
        let plan = self
            .policy
            .plan(
                self.transitions.registry(),
                self.transitions.policy().registry(),
                source,
                destination,
            )
            .map_err(ApplicabilityPolicyIdentityError::Applicability)?;

        Ok(ManifestBoundApplicableTransitionPlan {
            authority: self.authority.clone(),
            plan,
        })
    }
}

/// Applicability-annotated structural plan carrying exact applicability authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestBoundApplicableTransitionPlan {
    authority: TransitionApplicabilityAuthorityStamp,
    plan: ApplicableTransitionPlan,
}

impl ManifestBoundApplicableTransitionPlan {
    pub const fn authority_stamp(&self) -> &TransitionApplicabilityAuthorityStamp {
        &self.authority
    }

    pub const fn plan(&self) -> &ApplicableTransitionPlan {
        &self.plan
    }

    pub fn validate(
        &self,
        policy: &TransitionApplicabilityPolicy,
        transitions: &ManifestBoundInformationTransitionRegistry<'_>,
    ) -> Result<(), ApplicabilityPolicyIdentityError> {
        self.authority.validate(policy, transitions)
    }
}

fn revalidate_policy(
    policy: &TransitionApplicabilityPolicy,
    transitions: &ManifestBoundInformationTransitionRegistry<'_>,
) -> Result<(), ApplicabilityPolicyIdentityError> {
    // #328 already proved this exact graph remains valid against its exact
    // information-policy corpus. Rebuilding #306 here proves the applicability
    // mapping is total over that exact edge set.
    transitions
        .authority_stamp()
        .validate(transitions.registry(), transitions.policy())
        .map_err(ApplicabilityPolicyIdentityError::TransitionIdentity)?;

    let mut builder = TransitionApplicabilityPolicyBuilder::new(policy.key());
    for (transition, _) in transitions.registry().transitions() {
        let applicability = policy.applicability(*transition).ok_or(
            ApplicabilityPolicyIdentityError::MissingApplicability {
                transition: *transition,
            },
        )?;
        builder
            .register(*transition, applicability)
            .map_err(ApplicabilityPolicyIdentityError::Applicability)?;
    }
    builder
        .seal(transitions.registry())
        .map_err(ApplicabilityPolicyIdentityError::Applicability)?;
    Ok(())
}

fn encode_policy_key(bytes: &mut Vec<u8>, key: TransitionApplicabilityPolicyKey) {
    push_u128(bytes, key.id());
    push_u32(bytes, key.version());
}

fn encode_transition_key(bytes: &mut Vec<u8>, key: InformationTransitionKey) {
    push_u128(bytes, key.id());
    push_u32(bytes, key.version());
}

fn encode_domain_key(bytes: &mut Vec<u8>, key: TransitionDomainKey) {
    push_u128(bytes, key.id());
    push_u32(bytes, key.version());
}

fn encode_applicability(bytes: &mut Vec<u8>, applicability: TransitionApplicability) {
    match applicability {
        TransitionApplicability::Universal => push_u8(bytes, 0),
        TransitionApplicability::RegisteredDomain(domain) => {
            push_u8(bytes, 1);
            encode_domain_key(bytes, domain);
        }
    }
}

fn push_len(bytes: &mut Vec<u8>, len: usize) {
    let len = u64::try_from(len).expect("applicability manifest section length fits u64");
    bytes.extend_from_slice(&len.to_le_bytes());
}

fn push_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u128(bytes: &mut Vec<u8>, value: u128) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

#[derive(Debug)]
pub enum ApplicabilityPolicyIdentityError {
    Applicability(TransitionApplicabilityError),
    TransitionIdentity(TransitionPolicyIdentityError),
    TransitionRegistryKeyMismatch {
        expected: InformationTransitionRegistryKey,
        actual: InformationTransitionRegistryKey,
    },
    PolicyKeyMismatch {
        expected: TransitionApplicabilityPolicyKey,
        actual: TransitionApplicabilityPolicyKey,
    },
    TransitionAuthorityMismatch {
        key: InformationTransitionRegistryKey,
    },
    MissingApplicability {
        transition: InformationTransitionKey,
    },
    ManifestMismatch {
        key: TransitionApplicabilityPolicyKey,
    },
}

impl fmt::Display for ApplicabilityPolicyIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Applicability(error) => write!(formatter, "transition applicability error: {error}"),
            Self::TransitionIdentity(error) => {
                write!(formatter, "transition authority identity error: {error}")
            }
            Self::TransitionRegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "applicability policy is bound to transition registry {expected:?}, got {actual:?}"
            ),
            Self::PolicyKeyMismatch { expected, actual } => write!(
                formatter,
                "applicability policy key {actual:?} does not match prepared key {expected:?}"
            ),
            Self::TransitionAuthorityMismatch { key } => write!(
                formatter,
                "exact transition authority changed under semantic transition key {key:?}"
            ),
            Self::MissingApplicability { transition } => write!(
                formatter,
                "exact transition {transition:?} has no applicability declaration"
            ),
            Self::ManifestMismatch { key } => write!(
                formatter,
                "applicability corpus changed under semantic policy key {key:?}"
            ),
        }
    }
}

impl Error for ApplicabilityPolicyIdentityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Applicability(error) => Some(error),
            Self::TransitionIdentity(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
        RepresentationCapabilities, RepresentationKey,
    };
    use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionRegistry,
        InformationTransitionRegistryBuilder, RetainedAuthorityKey, TransitionCapabilityClaim,
    };

    const POLICY_KEY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(70, 3);
    const GRAPH_KEY: InformationTransitionRegistryKey =
        InformationTransitionRegistryKey::new(80, 2);
    const APPLICABILITY_KEY: TransitionApplicabilityPolicyKey =
        TransitionApplicabilityPolicyKey::new(90, 4);
    const EDGE: InformationTransitionKey = InformationTransitionKey::new(20, 1);
    const SOURCE: RepresentationKey = RepresentationKey::new(10, 1);
    const TARGET: RepresentationKey = RepresentationKey::new(11, 1);
    const DOMAIN_A: TransitionDomainKey = TransitionDomainKey::new(100, 1);
    const DOMAIN_B: TransitionDomainKey = TransitionDomainKey::new(101, 1);

    fn information_policy(extra_target_claim: bool) -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY_KEY);
        builder
            .register_representation(RepresentationCapabilities::new(
                SOURCE,
                EcologicalAuthorityLevel::Coarse,
                [(EcologicalInformation::Headcount, CapabilityEvidence::Exact)],
            ))
            .unwrap();

        let mut target_claims = vec![
            (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
            (
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::Exact,
            ),
        ];
        if extra_target_claim {
            target_claims.push((
                EcologicalInformation::ConditionDistribution,
                CapabilityEvidence::Exact,
            ));
        }
        builder
            .register_representation(RepresentationCapabilities::new(
                TARGET,
                EcologicalAuthorityLevel::Coarse,
                target_claims,
            ))
            .unwrap();
        builder.seal()
    }

    fn transition_graph(
        policy: &InformationPolicyRegistry,
        include_condition: bool,
    ) -> InformationTransitionRegistry {
        let mut introductions = vec![(
            TransitionCapabilityClaim::new(
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::Exact,
            ),
            crate::information_transition::PromotionProvenance::RetainedExact {
                authority: RetainedAuthorityKey(30, 1),
            },
        )];
        if include_condition {
            introductions.push((
                TransitionCapabilityClaim::new(
                    EcologicalInformation::ConditionDistribution,
                    CapabilityEvidence::Exact,
                ),
                crate::information_transition::PromotionProvenance::RetainedExact {
                    authority: RetainedAuthorityKey(31, 1),
                },
            ));
        }

        let edge = InformationTransitionDefinition::new(
            EDGE,
            SOURCE,
            TARGET,
            introductions,
            [],
        )
        .unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH_KEY);
        builder.register_transition(edge).unwrap();
        builder.seal(policy).unwrap()
    }

    fn exact_transitions<'a>(
        graph: &'a InformationTransitionRegistry,
        policy: &'a InformationPolicyRegistry,
    ) -> ManifestBoundInformationTransitionRegistry<'a> {
        let policy = ManifestBoundInformationPolicyRegistry::new(policy);
        ManifestBoundInformationTransitionRegistry::new(graph, &policy).unwrap()
    }

    fn applicability(
        graph: &InformationTransitionRegistry,
        mode: TransitionApplicability,
    ) -> TransitionApplicabilityPolicy {
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY_KEY);
        builder.register(EDGE, mode).unwrap();
        builder.seal(graph).unwrap()
    }

    #[test]
    fn identical_mapping_and_transition_authority_produce_identical_stamp() {
        let policy = information_policy(false);
        let graph = transition_graph(&policy, false);
        let exact = exact_transitions(&graph, &policy);
        let a = applicability(&graph, TransitionApplicability::Universal);
        let b = applicability(&graph, TransitionApplicability::Universal);

        let a = ManifestBoundTransitionApplicabilityPolicy::new(&a, &exact).unwrap();
        let b = ManifestBoundTransitionApplicabilityPolicy::new(&b, &exact).unwrap();
        assert_eq!(a.authority_stamp(), b.authority_stamp());
    }

    #[test]
    fn universal_vs_registered_domain_changes_exact_identity() {
        let policy = information_policy(false);
        let graph = transition_graph(&policy, false);
        let exact = exact_transitions(&graph, &policy);
        let universal = applicability(&graph, TransitionApplicability::Universal);
        let gated = applicability(
            &graph,
            TransitionApplicability::RegisteredDomain(DOMAIN_A),
        );

        let universal =
            ManifestBoundTransitionApplicabilityPolicy::new(&universal, &exact).unwrap();
        let gated = ManifestBoundTransitionApplicabilityPolicy::new(&gated, &exact).unwrap();
        assert_ne!(universal.authority_stamp(), gated.authority_stamp());
    }

    #[test]
    fn changing_only_domain_descriptor_changes_exact_identity() {
        let policy = information_policy(false);
        let graph = transition_graph(&policy, false);
        let exact = exact_transitions(&graph, &policy);
        let a = applicability(
            &graph,
            TransitionApplicability::RegisteredDomain(DOMAIN_A),
        );
        let b = applicability(
            &graph,
            TransitionApplicability::RegisteredDomain(DOMAIN_B),
        );

        let a = ManifestBoundTransitionApplicabilityPolicy::new(&a, &exact).unwrap();
        let b = ManifestBoundTransitionApplicabilityPolicy::new(&b, &exact).unwrap();
        assert_ne!(a.authority_stamp(), b.authority_stamp());
    }

    #[test]
    fn same_mapping_bound_to_changed_exact_graph_is_different_authority() {
        let policy_a = information_policy(false);
        let graph_a = transition_graph(&policy_a, false);
        let exact_a = exact_transitions(&graph_a, &policy_a);
        let mapping_a = applicability(&graph_a, TransitionApplicability::Universal);
        let stamp_a = ManifestBoundTransitionApplicabilityPolicy::new(&mapping_a, &exact_a)
            .unwrap()
            .authority_stamp()
            .clone();

        let policy_b = information_policy(true);
        let graph_b = transition_graph(&policy_b, true);
        let exact_b = exact_transitions(&graph_b, &policy_b);
        let mapping_b = applicability(&graph_b, TransitionApplicability::Universal);
        let stamp_b = ManifestBoundTransitionApplicabilityPolicy::new(&mapping_b, &exact_b)
            .unwrap()
            .authority_stamp()
            .clone();

        assert_eq!(stamp_a.key(), stamp_b.key());
        assert_ne!(stamp_a, stamp_b);
    }

    #[test]
    fn prepared_plan_rejects_same_key_mapping_drift() {
        let policy = information_policy(false);
        let graph = transition_graph(&policy, false);
        let exact = exact_transitions(&graph, &policy);
        let universal = applicability(&graph, TransitionApplicability::Universal);
        let gated = applicability(
            &graph,
            TransitionApplicability::RegisteredDomain(DOMAIN_A),
        );

        let exact_universal =
            ManifestBoundTransitionApplicabilityPolicy::new(&universal, &exact).unwrap();
        let plan = exact_universal.plan(SOURCE, TARGET).unwrap();

        assert!(matches!(
            plan.validate(&gated, &exact),
            Err(ApplicabilityPolicyIdentityError::ManifestMismatch { key })
                if key == APPLICABILITY_KEY
        ));
    }

    #[test]
    fn registered_domain_plan_preserves_unresolved_requirement() {
        let policy = information_policy(false);
        let graph = transition_graph(&policy, false);
        let exact = exact_transitions(&graph, &policy);
        let gated = applicability(
            &graph,
            TransitionApplicability::RegisteredDomain(DOMAIN_A),
        );
        let exact_gated = ManifestBoundTransitionApplicabilityPolicy::new(&gated, &exact).unwrap();
        let plan = exact_gated.plan(SOURCE, TARGET).unwrap();

        assert_eq!(plan.authority_stamp(), exact_gated.authority_stamp());
        assert_eq!(plan.plan().required_domains(), &std::collections::BTreeSet::from([DOMAIN_A]));
    }

    #[test]
    fn manifest_prefix_and_version_are_explicit() {
        let policy = information_policy(false);
        let graph = transition_graph(&policy, false);
        let exact = exact_transitions(&graph, &policy);
        let mapping = applicability(&graph, TransitionApplicability::Universal);
        let exact_mapping =
            ManifestBoundTransitionApplicabilityPolicy::new(&mapping, &exact).unwrap();

        let bytes = exact_mapping.authority_stamp().manifest().as_bytes();
        assert!(bytes.starts_with(MANIFEST_DOMAIN));
        let version_offset = MANIFEST_DOMAIN.len();
        assert_eq!(
            &bytes[version_offset..version_offset + 4],
            &TRANSITION_APPLICABILITY_MANIFEST_VERSION.to_le_bytes()
        );
    }
}
