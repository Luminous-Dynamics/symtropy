// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact content identity for ecological information-transition graphs.
//!
//! The V0 transition registry binds only a semantic registry key and semantic
//! information-policy key. This module upgrades that boundary by revalidating all
//! registered edges against one exact manifest-bound information-policy corpus,
//! then deriving a canonical byte manifest of the graph plus that exact policy
//! identity. The complete canonical bytes are the exact identity at this layer;
//! no ad-hoc short hash is invented here.

use std::error::Error;
use std::fmt;

use crate::conservation::ConservedQuantity;
use crate::information::{
    CapabilityEvidence, EcologicalInformation, QualifiedClosureEvidence, RepresentationKey,
};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_transition::{
    ConditionalDerivationKey, InformationTransitionDefinition, InformationTransitionError,
    InformationTransitionKey, InformationTransitionPlan, InformationTransitionRegistry,
    InformationTransitionRegistryBuilder, InformationTransitionRegistryKey, LosslessTransformKey,
    MeasurementAuthorityKey, PromotionProvenance, RetainedAuthorityKey, TransitionCapabilityClaim,
};

/// Canonical encoding version for exact transition-graph manifests.
pub const INFORMATION_TRANSITION_MANIFEST_VERSION: u32 = 1;

const MANIFEST_DOMAIN: &[u8] = b"SYMTROPY_INFORMATION_TRANSITION_MANIFEST\0";

/// Canonical exact byte identity for one transition graph under one exact policy
/// corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationTransitionManifest {
    bytes: Vec<u8>,
}

impl InformationTransitionManifest {
    fn from_validated(
        registry: &InformationTransitionRegistry,
        policy_authority: &InformationPolicyAuthorityStamp,
    ) -> Self {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MANIFEST_DOMAIN);
        push_u32(&mut bytes, INFORMATION_TRANSITION_MANIFEST_VERSION);
        encode_transition_registry_key(&mut bytes, registry.key());

        let policy_bytes = policy_authority.manifest().as_bytes();
        push_len(&mut bytes, policy_bytes.len());
        bytes.extend_from_slice(policy_bytes);

        let transitions = registry.transitions().collect::<Vec<_>>();
        push_len(&mut bytes, transitions.len());
        for (_, transition) in transitions {
            encode_transition(&mut bytes, transition);
        }

        Self { bytes }
    }

    pub const fn version(&self) -> u32 {
        INFORMATION_TRANSITION_MANIFEST_VERSION
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Exact transition authority: semantic graph key + exact policy corpus + exact
/// canonical graph manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationTransitionAuthorityStamp {
    key: InformationTransitionRegistryKey,
    policy_authority: InformationPolicyAuthorityStamp,
    manifest: InformationTransitionManifest,
}

impl InformationTransitionAuthorityStamp {
    pub const fn key(&self) -> InformationTransitionRegistryKey {
        self.key
    }

    pub const fn manifest_version(&self) -> u32 {
        INFORMATION_TRANSITION_MANIFEST_VERSION
    }

    pub const fn policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.policy_authority
    }

    pub const fn manifest(&self) -> &InformationTransitionManifest {
        &self.manifest
    }

    /// Revalidate exact graph and policy content before a later authority
    /// boundary consumes a prepared transition result.
    pub fn validate(
        &self,
        registry: &InformationTransitionRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), TransitionPolicyIdentityError> {
        if registry.key() != self.key {
            return Err(TransitionPolicyIdentityError::TransitionRegistryKeyMismatch {
                expected: self.key,
                actual: registry.key(),
            });
        }
        self.policy_authority
            .validate_registry(policy.registry())
            .map_err(TransitionPolicyIdentityError::PolicyIdentity)?;

        let current = ManifestBoundInformationTransitionRegistry::new(registry, policy)?;
        if current.authority != *self {
            return Err(TransitionPolicyIdentityError::ManifestMismatch { key: self.key });
        }
        Ok(())
    }
}

/// Read-only exact-content view over one sealed transition registry.
#[derive(Debug, Clone)]
pub struct ManifestBoundInformationTransitionRegistry<'a> {
    registry: &'a InformationTransitionRegistry,
    policy: ManifestBoundInformationPolicyRegistry<'a>,
    authority: InformationTransitionAuthorityStamp,
}

impl<'a> ManifestBoundInformationTransitionRegistry<'a> {
    /// Upgrade a V0 semantic transition registry into exact-content authority.
    ///
    /// Every edge is cloned through the public transition-definition surface and
    /// resealed against the supplied exact policy corpus. This prevents an old
    /// semantic-key-only graph from being stamped against a same-key policy corpus
    /// without proving that all of its edge semantics remain valid there.
    pub fn new(
        registry: &'a InformationTransitionRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'a>,
    ) -> Result<Self, TransitionPolicyIdentityError> {
        if registry.policy_registry_key() != policy.authority_stamp().key() {
            return Err(TransitionPolicyIdentityError::PolicyRegistryKeyMismatch {
                expected: registry.policy_registry_key(),
                actual: policy.authority_stamp().key(),
            });
        }

        revalidate_graph(registry, policy)?;
        let policy_owned = ManifestBoundInformationPolicyRegistry::new(policy.registry());
        let policy_authority = policy_owned.authority_stamp().clone();
        let manifest = InformationTransitionManifest::from_validated(registry, &policy_authority);
        let authority = InformationTransitionAuthorityStamp {
            key: registry.key(),
            policy_authority,
            manifest,
        };

        Ok(Self {
            registry,
            policy: policy_owned,
            authority,
        })
    }

    pub const fn registry(&self) -> &'a InformationTransitionRegistry {
        self.registry
    }

    pub const fn policy(&self) -> &ManifestBoundInformationPolicyRegistry<'a> {
        &self.policy
    }

    pub const fn authority_stamp(&self) -> &InformationTransitionAuthorityStamp {
        &self.authority
    }

    /// Delegate structural path planning only after exact graph/policy authority
    /// has been established, and bind the resulting plan to that exact authority.
    pub fn plan(
        &self,
        source: RepresentationKey,
        destination: RepresentationKey,
    ) -> Result<ManifestBoundInformationTransitionPlan, TransitionPolicyIdentityError> {
        let plan = self
            .registry
            .plan(self.policy.registry(), source, destination)
            .map_err(TransitionPolicyIdentityError::Transition)?;
        Ok(ManifestBoundInformationTransitionPlan {
            authority: self.authority.clone(),
            plan,
        })
    }
}

/// Structural transition plan carrying the exact graph + policy authority that
/// produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestBoundInformationTransitionPlan {
    authority: InformationTransitionAuthorityStamp,
    plan: InformationTransitionPlan,
}

impl ManifestBoundInformationTransitionPlan {
    pub const fn authority_stamp(&self) -> &InformationTransitionAuthorityStamp {
        &self.authority
    }

    pub const fn plan(&self) -> &InformationTransitionPlan {
        &self.plan
    }

    pub fn validate(
        &self,
        registry: &InformationTransitionRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), TransitionPolicyIdentityError> {
        self.authority.validate(registry, policy)
    }
}

#[derive(Debug)]
pub enum TransitionPolicyIdentityError {
    PolicyIdentity(InformationPolicyIdentityError),
    Transition(InformationTransitionError),
    PolicyRegistryKeyMismatch {
        expected: crate::information_registry::InformationPolicyRegistryKey,
        actual: crate::information_registry::InformationPolicyRegistryKey,
    },
    TransitionRegistryKeyMismatch {
        expected: InformationTransitionRegistryKey,
        actual: InformationTransitionRegistryKey,
    },
    ManifestMismatch {
        key: InformationTransitionRegistryKey,
    },
}

impl fmt::Display for TransitionPolicyIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PolicyIdentity(error) => write!(formatter, "information-policy identity error: {error}"),
            Self::Transition(error) => write!(formatter, "information-transition error: {error}"),
            Self::PolicyRegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "transition graph expects policy registry {}@{}, got {}@{}",
                expected.id(),
                expected.version(),
                actual.id(),
                actual.version()
            ),
            Self::TransitionRegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "transition registry key mismatch: expected {}@{}, got {}@{}",
                expected.id(),
                expected.version(),
                actual.id(),
                actual.version()
            ),
            Self::ManifestMismatch { key } => write!(
                formatter,
                "transition registry {}@{} has different exact manifest content",
                key.id(),
                key.version()
            ),
        }
    }
}

impl Error for TransitionPolicyIdentityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyIdentity(error) => Some(error),
            Self::Transition(error) => Some(error),
            _ => None,
        }
    }
}

fn revalidate_graph(
    registry: &InformationTransitionRegistry,
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
) -> Result<(), TransitionPolicyIdentityError> {
    let mut builder = InformationTransitionRegistryBuilder::new(registry.key());
    for (_, transition) in registry.transitions() {
        builder
            .register_transition(transition.clone())
            .map_err(TransitionPolicyIdentityError::Transition)?;
    }
    builder
        .seal(policy.registry())
        .map_err(TransitionPolicyIdentityError::Transition)?;
    Ok(())
}

fn push_len(bytes: &mut Vec<u8>, len: usize) {
    let len = u64::try_from(len).expect("transition manifest count exceeds u64");
    bytes.extend_from_slice(&len.to_le_bytes());
}

fn push_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u128(bytes: &mut Vec<u8>, value: u128) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn encode_transition_registry_key(bytes: &mut Vec<u8>, key: InformationTransitionRegistryKey) {
    push_u128(bytes, key.id());
    push_u32(bytes, key.version());
}

fn encode_transition_key(bytes: &mut Vec<u8>, key: InformationTransitionKey) {
    push_u128(bytes, key.id());
    push_u32(bytes, key.version());
}

fn encode_representation_key(bytes: &mut Vec<u8>, key: RepresentationKey) {
    push_u128(bytes, key.id());
    push_u32(bytes, key.version());
}

fn encode_conserved_quantity(bytes: &mut Vec<u8>, quantity: ConservedQuantity) {
    let tag = match quantity {
        ConservedQuantity::BiomassMass => 0,
        ConservedQuantity::CarbonMass => 1,
        ConservedQuantity::WaterMass => 2,
        ConservedQuantity::MineralNutrientMass => 3,
    };
    push_u8(bytes, tag);
}

fn encode_information(bytes: &mut Vec<u8>, information: EcologicalInformation) {
    match information {
        EcologicalInformation::Headcount => push_u8(bytes, 0),
        EcologicalInformation::ExactLivingBiomass => push_u8(bytes, 1),
        EcologicalInformation::AgeDistribution => push_u8(bytes, 2),
        EcologicalInformation::ConditionDistribution => push_u8(bytes, 3),
        EcologicalInformation::OccupancyDistribution => push_u8(bytes, 4),
        EcologicalInformation::JointPopulationStatistics(statistics) => {
            push_u8(bytes, 5);
            push_u16(bytes, statistics.bits());
        }
        EcologicalInformation::SpatialStructure => push_u8(bytes, 6),
        EcologicalInformation::DiseaseState => push_u8(bytes, 7),
        EcologicalInformation::GeneticSummary => push_u8(bytes, 8),
        EcologicalInformation::ContactStructure => push_u8(bytes, 9),
        EcologicalInformation::IndividualActiveState => push_u8(bytes, 10),
        EcologicalInformation::PersistentIdentity => push_u8(bytes, 11),
        EcologicalInformation::ExactConservationAccount(quantity) => {
            push_u8(bytes, 12);
            encode_conserved_quantity(bytes, quantity);
        }
    }
}

fn encode_qualified_closure(bytes: &mut Vec<u8>, closure: QualifiedClosureEvidence) {
    push_u32(bytes, closure.model_version().0);
    push_u128(bytes, closure.domain().0);
    push_u32(bytes, closure.error_ppm().get());
    push_u128(bytes, closure.evidence_lineage().0);
}

fn encode_capability_evidence(bytes: &mut Vec<u8>, evidence: CapabilityEvidence) {
    match evidence {
        CapabilityEvidence::Exact => push_u8(bytes, 0),
        CapabilityEvidence::QualifiedClosure(closure) => {
            push_u8(bytes, 1);
            encode_qualified_closure(bytes, closure);
        }
        CapabilityEvidence::MeasurementOnly => push_u8(bytes, 2),
    }
}

fn encode_claim(bytes: &mut Vec<u8>, claim: TransitionCapabilityClaim) {
    encode_information(bytes, claim.information());
    encode_capability_evidence(bytes, claim.evidence());
}

fn encode_retained_authority(bytes: &mut Vec<u8>, key: RetainedAuthorityKey) {
    push_u128(bytes, key.0);
    push_u32(bytes, key.1);
}

fn encode_lossless_transform(bytes: &mut Vec<u8>, key: LosslessTransformKey) {
    push_u128(bytes, key.0);
    push_u32(bytes, key.1);
}

fn encode_measurement_authority(bytes: &mut Vec<u8>, key: MeasurementAuthorityKey) {
    push_u128(bytes, key.0);
    push_u32(bytes, key.1);
}

fn encode_conditional_derivation(bytes: &mut Vec<u8>, key: ConditionalDerivationKey) {
    push_u128(bytes, key.0);
    push_u32(bytes, key.1);
}

fn encode_provenance(bytes: &mut Vec<u8>, provenance: PromotionProvenance) {
    match provenance {
        PromotionProvenance::RetainedExact { authority } => {
            push_u8(bytes, 0);
            encode_retained_authority(bytes, authority);
        }
        PromotionProvenance::LosslessDerivation { transform } => {
            push_u8(bytes, 1);
            encode_lossless_transform(bytes, transform);
        }
        PromotionProvenance::QualifiedClosure { evidence_lineage } => {
            push_u8(bytes, 2);
            push_u128(bytes, evidence_lineage.0);
        }
        PromotionProvenance::MeasurementAssimilation { authority } => {
            push_u8(bytes, 3);
            encode_measurement_authority(bytes, authority);
        }
        PromotionProvenance::ConditionalMicrostate { model } => {
            push_u8(bytes, 4);
            encode_conditional_derivation(bytes, model);
        }
    }
}

fn encode_transition(bytes: &mut Vec<u8>, transition: &InformationTransitionDefinition) {
    encode_transition_key(bytes, transition.key());
    encode_representation_key(bytes, transition.source());
    encode_representation_key(bytes, transition.destination());

    push_len(bytes, transition.introductions().len());
    for (claim, provenance) in transition.introductions() {
        encode_claim(bytes, *claim);
        encode_provenance(bytes, *provenance);
    }

    push_len(bytes, transition.discarded_information().len());
    for information in transition.discarded_information() {
        encode_information(bytes, *information);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        EcologicalAuthorityLevel, ProcessInformationProfile, ProcessInformationRequirement,
        RepresentationCapabilities,
    };
    use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };

    const POLICY_KEY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(2000, 1);
    const GRAPH_KEY: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(2001, 1);
    const A: RepresentationKey = RepresentationKey::new(2010, 1);
    const B: RepresentationKey = RepresentationKey::new(2011, 1);
    const PROCESS: crate::information::ProcessKey = crate::information::ProcessKey::new(2020, 1);
    const EDGE: InformationTransitionKey = InformationTransitionKey::new(2030, 1);
    const RETAINED: RetainedAuthorityKey = RetainedAuthorityKey(2040, 1);

    fn policy(require_b_condition: bool) -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY_KEY);
        builder
            .register_process(ProcessInformationProfile::new(
                PROCESS,
                EcologicalAuthorityLevel::Coarse,
                [ProcessInformationRequirement::exact(
                    EcologicalInformation::Headcount,
                )],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                A,
                EcologicalAuthorityLevel::Coarse,
                [(EcologicalInformation::Headcount, CapabilityEvidence::Exact)],
            ))
            .unwrap();

        let mut b_claims = vec![
            (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
            (
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::Exact,
            ),
        ];
        if require_b_condition {
            b_claims.push((
                EcologicalInformation::ConditionDistribution,
                CapabilityEvidence::Exact,
            ));
        }
        builder
            .register_representation(RepresentationCapabilities::new(
                B,
                EcologicalAuthorityLevel::Coarse,
                b_claims,
            ))
            .unwrap();
        builder.seal()
    }

    fn edge(include_condition: bool) -> InformationTransitionDefinition {
        let mut introductions = vec![(
            TransitionCapabilityClaim::new(
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::Exact,
            ),
            PromotionProvenance::RetainedExact {
                authority: RETAINED,
            },
        )];
        if include_condition {
            introductions.push((
                TransitionCapabilityClaim::new(
                    EcologicalInformation::ConditionDistribution,
                    CapabilityEvidence::Exact,
                ),
                PromotionProvenance::RetainedExact {
                    authority: RETAINED,
                },
            ));
        }
        InformationTransitionDefinition::new(EDGE, A, B, introductions, []).unwrap()
    }

    fn graph(
        policy: &InformationPolicyRegistry,
        include_condition: bool,
    ) -> InformationTransitionRegistry {
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH_KEY);
        builder
            .register_transition(edge(include_condition))
            .unwrap();
        builder.seal(policy).unwrap()
    }

    #[test]
    fn same_graph_and_policy_produce_same_exact_stamp() {
        let policy = policy(false);
        let graph = graph(&policy, false);
        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);

        let first = ManifestBoundInformationTransitionRegistry::new(&graph, &bound_policy).unwrap();
        let second = ManifestBoundInformationTransitionRegistry::new(&graph, &bound_policy).unwrap();
        assert_eq!(first.authority_stamp(), second.authority_stamp());
    }

    #[test]
    fn same_semantic_graph_key_with_changed_edge_changes_identity() {
        let first_policy = policy(false);
        let first_graph = graph(&first_policy, false);
        let first_bound_policy = ManifestBoundInformationPolicyRegistry::new(&first_policy);
        let first =
            ManifestBoundInformationTransitionRegistry::new(&first_graph, &first_bound_policy)
                .unwrap();

        let second_policy = policy(true);
        let second_graph = graph(&second_policy, true);
        let second_bound_policy = ManifestBoundInformationPolicyRegistry::new(&second_policy);
        let second =
            ManifestBoundInformationTransitionRegistry::new(&second_graph, &second_bound_policy)
                .unwrap();

        assert_ne!(first.authority_stamp(), second.authority_stamp());
    }

    #[test]
    fn same_graph_shape_bound_to_changed_policy_corpus_changes_identity() {
        let first_policy = policy(false);
        let first_graph = graph(&first_policy, false);
        let first_bound_policy = ManifestBoundInformationPolicyRegistry::new(&first_policy);
        let first =
            ManifestBoundInformationTransitionRegistry::new(&first_graph, &first_bound_policy)
                .unwrap();

        let mut changed_builder = InformationPolicyRegistryBuilder::new(POLICY_KEY);
        changed_builder
            .register_process(ProcessInformationProfile::new(
                PROCESS,
                EcologicalAuthorityLevel::Active,
                [ProcessInformationRequirement::exact(
                    EcologicalInformation::Headcount,
                )],
            ))
            .unwrap();
        changed_builder
            .register_representation(RepresentationCapabilities::new(
                A,
                EcologicalAuthorityLevel::Coarse,
                [(EcologicalInformation::Headcount, CapabilityEvidence::Exact)],
            ))
            .unwrap();
        changed_builder
            .register_representation(RepresentationCapabilities::new(
                B,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::AgeDistribution,
                        CapabilityEvidence::Exact,
                    ),
                ],
            ))
            .unwrap();
        let changed_policy = changed_builder.seal();
        let changed_bound_policy = ManifestBoundInformationPolicyRegistry::new(&changed_policy);
        let second =
            ManifestBoundInformationTransitionRegistry::new(&first_graph, &changed_bound_policy)
                .unwrap();

        assert_ne!(first.authority_stamp(), second.authority_stamp());
    }

    #[test]
    fn plan_binds_exact_graph_and_policy_authority() {
        let policy = policy(false);
        let graph = graph(&policy, false);
        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let bound_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &bound_policy).unwrap();
        let plan = bound_graph.plan(A, B).unwrap();

        assert_eq!(plan.authority_stamp(), bound_graph.authority_stamp());
        assert_eq!(plan.plan().transitions(), &[EDGE]);
        assert!(plan.validate(&graph, &bound_policy).is_ok());
    }

    #[test]
    fn same_key_changed_policy_invalidates_old_plan() {
        let original_policy = policy(false);
        let graph = graph(&original_policy, false);
        let original_bound_policy = ManifestBoundInformationPolicyRegistry::new(&original_policy);
        let bound_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &original_bound_policy)
                .unwrap();
        let plan = bound_graph.plan(A, B).unwrap();

        let mut changed_builder = InformationPolicyRegistryBuilder::new(POLICY_KEY);
        changed_builder
            .register_process(ProcessInformationProfile::new(
                PROCESS,
                EcologicalAuthorityLevel::Active,
                [ProcessInformationRequirement::exact(
                    EcologicalInformation::Headcount,
                )],
            ))
            .unwrap();
        changed_builder
            .register_representation(RepresentationCapabilities::new(
                A,
                EcologicalAuthorityLevel::Coarse,
                [(EcologicalInformation::Headcount, CapabilityEvidence::Exact)],
            ))
            .unwrap();
        changed_builder
            .register_representation(RepresentationCapabilities::new(
                B,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::AgeDistribution,
                        CapabilityEvidence::Exact,
                    ),
                ],
            ))
            .unwrap();
        let changed_policy = changed_builder.seal();
        let changed_bound_policy = ManifestBoundInformationPolicyRegistry::new(&changed_policy);

        assert!(matches!(
            plan.validate(&graph, &changed_bound_policy),
            Err(TransitionPolicyIdentityError::PolicyIdentity(
                InformationPolicyIdentityError::ManifestMismatch { key: POLICY_KEY }
            ))
        ));
    }

    #[test]
    fn changed_transition_graph_invalidates_old_plan() {
        let first_policy = policy(false);
        let first_graph = graph(&first_policy, false);
        let first_bound_policy = ManifestBoundInformationPolicyRegistry::new(&first_policy);
        let bound_graph =
            ManifestBoundInformationTransitionRegistry::new(&first_graph, &first_bound_policy)
                .unwrap();
        let plan = bound_graph.plan(A, B).unwrap();

        let second_policy = policy(true);
        let second_graph = graph(&second_policy, true);
        let second_bound_policy = ManifestBoundInformationPolicyRegistry::new(&second_policy);

        assert!(plan.validate(&second_graph, &second_bound_policy).is_err());
    }

    #[test]
    fn manifest_has_explicit_domain_and_version_prefix() {
        let policy = policy(false);
        let graph = graph(&policy, false);
        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let bound_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &bound_policy).unwrap();
        let manifest = bound_graph.authority_stamp().manifest();

        assert!(manifest.as_bytes().starts_with(MANIFEST_DOMAIN));
        let version_offset = MANIFEST_DOMAIN.len();
        assert_eq!(
            &manifest.as_bytes()[version_offset..version_offset + 4],
            &INFORMATION_TRANSITION_MANIFEST_VERSION.to_le_bytes()
        );
    }
}
