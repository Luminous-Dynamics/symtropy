// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Information-preserving reachability for ecological fidelity transitions.
//!
//! A richer representation being sufficient for a process does not imply that
//! the current canonical state can legitimately become that representation.
//! Transition policy is therefore sealed and registry-owned: callers may request
//! a source/target pair, but cannot inject an ad hoc edge that manufactures
//! forgotten exact information.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::error::Error;
use std::fmt;

use crate::information::{
    CapabilityEvidence, EcologicalInformation, EvidenceLineageToken, RepresentationCapabilities,
    RepresentationKey,
};
use crate::information_registry::{
    InformationPolicyRegistry, InformationPolicyRegistryKey, InformationRegistryError,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InformationTransitionRegistryKey {
    id: u128,
    version: u32,
}

impl InformationTransitionRegistryKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InformationTransitionKey {
    id: u128,
    version: u32,
}

impl InformationTransitionKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RetainedAuthorityKey(pub u128, pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LosslessTransformKey(pub u128, pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasurementAuthorityKey(pub u128, pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConditionalDerivationKey(pub u128, pub u32);

/// Provenance classes for information that becomes newly available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromotionProvenanceClass {
    /// R0: exact information already existed canonically and is being revealed.
    RetainedExact,
    /// R1: exact information follows uniquely from a registered lossless transform.
    LosslessDerivation,
    /// R2: approximate information is reconstructed under qualified closure evidence.
    QualifiedClosure,
    /// R3: exact information enters through qualified measurement/assimilation.
    MeasurementAssimilation,
    /// R4: conditional D-state fills unresolved degrees of freedom non-authoritatively.
    ConditionalMicrostate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromotionProvenance {
    RetainedExact { authority: RetainedAuthorityKey },
    LosslessDerivation { transform: LosslessTransformKey },
    QualifiedClosure { evidence_lineage: EvidenceLineageToken },
    MeasurementAssimilation { authority: MeasurementAuthorityKey },
    ConditionalMicrostate { model: ConditionalDerivationKey },
}

impl PromotionProvenance {
    pub const fn class(self) -> PromotionProvenanceClass {
        match self {
            Self::RetainedExact { .. } => PromotionProvenanceClass::RetainedExact,
            Self::LosslessDerivation { .. } => PromotionProvenanceClass::LosslessDerivation,
            Self::QualifiedClosure { .. } => PromotionProvenanceClass::QualifiedClosure,
            Self::MeasurementAssimilation { .. } => {
                PromotionProvenanceClass::MeasurementAssimilation
            }
            Self::ConditionalMicrostate { .. } => {
                PromotionProvenanceClass::ConditionalMicrostate
            }
        }
    }
}

/// One concrete destination capability that requires provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionCapabilityClaim {
    information: EcologicalInformation,
    evidence: CapabilityEvidence,
}

impl TransitionCapabilityClaim {
    pub const fn new(information: EcologicalInformation, evidence: CapabilityEvidence) -> Self {
        Self {
            information,
            evidence,
        }
    }

    pub const fn information(self) -> EcologicalInformation {
        self.information
    }

    pub const fn evidence(self) -> CapabilityEvidence {
        self.evidence
    }
}

/// One semantic representation transition. Exact information that becomes
/// unrecoverable must be listed in `discarded_information`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationTransitionDefinition {
    key: InformationTransitionKey,
    source: RepresentationKey,
    destination: RepresentationKey,
    introductions: BTreeMap<TransitionCapabilityClaim, PromotionProvenance>,
    discarded_information: BTreeSet<EcologicalInformation>,
}

impl InformationTransitionDefinition {
    pub fn new(
        key: InformationTransitionKey,
        source: RepresentationKey,
        destination: RepresentationKey,
        introductions: impl IntoIterator<Item = (TransitionCapabilityClaim, PromotionProvenance)>,
        discarded_information: impl IntoIterator<Item = EcologicalInformation>,
    ) -> Result<Self, InformationTransitionError> {
        let mut introduction_map = BTreeMap::new();
        for (claim, provenance) in introductions {
            if introduction_map.insert(claim, provenance).is_some() {
                return Err(InformationTransitionError::DuplicateIntroductionClaim {
                    transition: key,
                    claim,
                });
            }
        }
        Ok(Self {
            key,
            source,
            destination,
            introductions: introduction_map,
            discarded_information: discarded_information.into_iter().collect(),
        })
    }

    pub const fn key(&self) -> InformationTransitionKey {
        self.key
    }

    pub const fn source(&self) -> RepresentationKey {
        self.source
    }

    pub const fn destination(&self) -> RepresentationKey {
        self.destination
    }

    pub fn introductions(&self) -> &BTreeMap<TransitionCapabilityClaim, PromotionProvenance> {
        &self.introductions
    }

    pub fn discarded_information(&self) -> &BTreeSet<EcologicalInformation> {
        &self.discarded_information
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationTransitionRegistryBuilder {
    key: InformationTransitionRegistryKey,
    transitions: BTreeMap<InformationTransitionKey, InformationTransitionDefinition>,
}

impl InformationTransitionRegistryBuilder {
    pub const fn new(key: InformationTransitionRegistryKey) -> Self {
        Self {
            key,
            transitions: BTreeMap::new(),
        }
    }

    pub fn register_transition(
        &mut self,
        transition: InformationTransitionDefinition,
    ) -> Result<(), InformationTransitionError> {
        let key = transition.key();
        if let Some(existing) = self.transitions.get(&key) {
            if existing == &transition {
                return Ok(());
            }
            return Err(InformationTransitionError::ConflictingTransitionRegistration { key });
        }
        self.transitions.insert(key, transition);
        Ok(())
    }

    /// Seal the graph only after every edge is valid against one exact policy
    /// registry generation.
    pub fn seal(
        self,
        policy: &InformationPolicyRegistry,
    ) -> Result<InformationTransitionRegistry, InformationTransitionError> {
        for transition in self.transitions.values() {
            validate_transition(policy, transition)?;
        }
        Ok(InformationTransitionRegistry {
            key: self.key,
            policy_registry_key: policy.key(),
            transitions: self.transitions,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationTransitionRegistry {
    key: InformationTransitionRegistryKey,
    policy_registry_key: InformationPolicyRegistryKey,
    transitions: BTreeMap<InformationTransitionKey, InformationTransitionDefinition>,
}

impl InformationTransitionRegistry {
    pub const fn key(&self) -> InformationTransitionRegistryKey {
        self.key
    }

    pub const fn policy_registry_key(&self) -> InformationPolicyRegistryKey {
        self.policy_registry_key
    }

    pub fn transitions(
        &self,
    ) -> impl Iterator<Item = (&InformationTransitionKey, &InformationTransitionDefinition)> {
        self.transitions.iter()
    }

    /// Find the shortest registered path. Equal-length alternatives are chosen
    /// deterministically by transition-key ordering.
    pub fn plan(
        &self,
        policy: &InformationPolicyRegistry,
        source: RepresentationKey,
        destination: RepresentationKey,
    ) -> Result<InformationTransitionPlan, InformationTransitionError> {
        if policy.key() != self.policy_registry_key {
            return Err(InformationTransitionError::PolicyRegistryKeyMismatch {
                expected: self.policy_registry_key,
                actual: policy.key(),
            });
        }
        policy
            .resolve_representation(source)
            .map_err(InformationTransitionError::Policy)?;
        policy
            .resolve_representation(destination)
            .map_err(InformationTransitionError::Policy)?;

        if source == destination {
            return Ok(self.build_plan(source, destination, &BTreeMap::new()));
        }

        let mut queue = VecDeque::from([source]);
        let mut visited = BTreeSet::from([source]);
        let mut predecessor = BTreeMap::new();

        while let Some(current) = queue.pop_front() {
            for (key, transition) in self
                .transitions
                .iter()
                .filter(|(_, transition)| transition.source == current)
            {
                let next = transition.destination;
                if !visited.insert(next) {
                    continue;
                }
                predecessor.insert(next, (current, *key));
                if next == destination {
                    return Ok(self.build_plan(source, destination, &predecessor));
                }
                queue.push_back(next);
            }
        }

        Err(InformationTransitionError::NoTransitionPath {
            source,
            destination,
        })
    }

    fn build_plan(
        &self,
        source: RepresentationKey,
        destination: RepresentationKey,
        predecessor: &BTreeMap<
            RepresentationKey,
            (RepresentationKey, InformationTransitionKey),
        >,
    ) -> InformationTransitionPlan {
        let mut reversed = Vec::new();
        let mut current = destination;
        while current != source {
            let (previous, transition) = predecessor
                .get(&current)
                .copied()
                .expect("reachable destination has a predecessor chain");
            reversed.push(transition);
            current = previous;
        }
        reversed.reverse();

        let mut external_requirements = BTreeSet::new();
        let mut closure_evidence = BTreeSet::new();
        let mut conditional_derivations = BTreeSet::new();
        let mut discarded_information = BTreeSet::new();
        for key in &reversed {
            let transition = self
                .transitions
                .get(key)
                .expect("planned transition remains registered");
            discarded_information.extend(transition.discarded_information.iter().copied());
            for provenance in transition.introductions.values().copied() {
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
                        external_requirements
                            .insert(PromotionEvidenceRequirement::MeasurementAuthority(authority));
                    }
                    PromotionProvenance::ConditionalMicrostate { model } => {
                        conditional_derivations.insert(model);
                    }
                }
            }
        }

        InformationTransitionPlan {
            transition_registry_key: self.key,
            policy_registry_key: self.policy_registry_key,
            source,
            destination,
            transitions: reversed,
            external_requirements,
            closure_evidence,
            conditional_derivations,
            discarded_information,
        }
    }
}

/// External authority/implementation prerequisites that this low-level structural
/// planner does not authenticate. Higher orchestration must resolve them before
/// canonical commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromotionEvidenceRequirement {
    RetainedAuthority(RetainedAuthorityKey),
    LosslessTransform(LosslessTransformKey),
    MeasurementAuthority(MeasurementAuthorityKey),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationTransitionPlan {
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

impl InformationTransitionPlan {
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

    /// True only when the path introduces no unresolved authority/transform
    /// prerequisite, no closure/D-state approximation, and no information loss.
    pub fn is_self_contained_exact(&self) -> bool {
        self.external_requirements.is_empty()
            && self.closure_evidence.is_empty()
            && self.conditional_derivations.is_empty()
            && self.discarded_information.is_empty()
    }
}

fn validate_transition(
    policy: &InformationPolicyRegistry,
    transition: &InformationTransitionDefinition,
) -> Result<(), InformationTransitionError> {
    if transition.source == transition.destination {
        return Err(InformationTransitionError::SelfTransition {
            transition: transition.key,
            representation: transition.source,
        });
    }

    let source = policy
        .resolve_representation(transition.source)
        .map_err(InformationTransitionError::Policy)?;
    let destination = policy
        .resolve_representation(transition.destination)
        .map_err(InformationTransitionError::Policy)?;
    let source = source.capabilities();
    let destination = destination.capabilities();

    for (information, evidence_set) in destination.claims() {
        for evidence in evidence_set.iter().copied() {
            let claim = TransitionCapabilityClaim::new(*information, evidence);
            match (
                source_covers_claim(source, claim),
                transition.introductions.get(&claim).copied(),
            ) {
                (true, Some(_)) => {
                    return Err(InformationTransitionError::RedundantIntroductionClaim {
                        transition: transition.key,
                        claim,
                    });
                }
                (true, None) => {}
                (false, None) => {
                    return Err(InformationTransitionError::MissingIntroductionProvenance {
                        transition: transition.key,
                        claim,
                    });
                }
                (false, Some(provenance)) => {
                    validate_provenance_for_claim(transition.key, claim, provenance)?;
                }
            }
        }
    }

    for claim in transition.introductions.keys().copied() {
        if !destination_has_claim(destination, claim) {
            return Err(InformationTransitionError::IntroductionNotInDestination {
                transition: transition.key,
                claim,
            });
        }
    }

    let actual_discards = lost_information(source, destination);
    for information in &actual_discards {
        if !transition.discarded_information.contains(information) {
            return Err(InformationTransitionError::MissingDeclaredDiscard {
                transition: transition.key,
                information: *information,
            });
        }
    }
    for information in &transition.discarded_information {
        if !actual_discards.contains(information) {
            return Err(InformationTransitionError::RedundantDeclaredDiscard {
                transition: transition.key,
                information: *information,
            });
        }
    }

    Ok(())
}

/// This relation answers whether the destination claim requires *new* authority.
/// Exact source evidence may be deliberately downgraded into closure/measurement
/// evidence without creating new information.
fn source_covers_claim(
    source: &RepresentationCapabilities,
    target: TransitionCapabilityClaim,
) -> bool {
    source.claims().iter().any(|(available_information, evidence_set)| {
        available_information.covers(target.information)
            && evidence_set
                .iter()
                .copied()
                .any(|available| available_can_supply_target(available, target.evidence))
    })
}

fn available_can_supply_target(available: CapabilityEvidence, target: CapabilityEvidence) -> bool {
    match (available, target) {
        (CapabilityEvidence::Exact, _) => true,
        (
            CapabilityEvidence::QualifiedClosure(available),
            CapabilityEvidence::QualifiedClosure(target),
        ) => available == target,
        (CapabilityEvidence::QualifiedClosure(_), CapabilityEvidence::MeasurementOnly) => true,
        (CapabilityEvidence::MeasurementOnly, CapabilityEvidence::MeasurementOnly) => true,
        _ => false,
    }
}

/// This stricter relation answers whether source evidence survives without loss.
/// Exact→closure is therefore a loss even though the closure can still be used.
fn source_evidence_is_retained(
    source: CapabilityEvidence,
    destination: CapabilityEvidence,
) -> bool {
    match (source, destination) {
        (CapabilityEvidence::Exact, CapabilityEvidence::Exact) => true,
        (CapabilityEvidence::QualifiedClosure(_), CapabilityEvidence::Exact) => true,
        (
            CapabilityEvidence::QualifiedClosure(source),
            CapabilityEvidence::QualifiedClosure(destination),
        ) => source == destination,
        (CapabilityEvidence::MeasurementOnly, _) => true,
        _ => false,
    }
}

fn destination_has_claim(
    destination: &RepresentationCapabilities,
    target: TransitionCapabilityClaim,
) -> bool {
    destination
        .claims()
        .get(&target.information)
        .is_some_and(|evidence| evidence.contains(&target.evidence))
}

fn validate_provenance_for_claim(
    transition: InformationTransitionKey,
    claim: TransitionCapabilityClaim,
    provenance: PromotionProvenance,
) -> Result<(), InformationTransitionError> {
    let valid = match (claim.evidence, provenance) {
        (
            CapabilityEvidence::Exact,
            PromotionProvenance::RetainedExact { .. }
            | PromotionProvenance::LosslessDerivation { .. }
            | PromotionProvenance::MeasurementAssimilation { .. },
        ) => true,
        (
            CapabilityEvidence::QualifiedClosure(closure),
            PromotionProvenance::QualifiedClosure { evidence_lineage },
        ) => closure.evidence_lineage() == evidence_lineage,
        (
            CapabilityEvidence::MeasurementOnly,
            PromotionProvenance::MeasurementAssimilation { .. }
            | PromotionProvenance::ConditionalMicrostate { .. },
        ) => true,
        _ => false,
    };

    if valid {
        Ok(())
    } else {
        Err(InformationTransitionError::InvalidProvenanceForClaim {
            transition,
            claim,
            provenance: provenance.class(),
        })
    }
}

fn lost_information(
    source: &RepresentationCapabilities,
    destination: &RepresentationCapabilities,
) -> BTreeSet<EcologicalInformation> {
    source
        .claims()
        .iter()
        .filter_map(|(information, source_evidence)| {
            let preserved = source_evidence.iter().copied().all(|source_evidence| {
                destination
                    .claims()
                    .iter()
                    .any(|(destination_information, destination_evidence)| {
                        destination_information.covers(*information)
                            && destination_evidence.iter().copied().any(|destination_evidence| {
                                source_evidence_is_retained(source_evidence, destination_evidence)
                            })
                    })
            });
            (!preserved).then_some(*information)
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InformationTransitionError {
    Policy(InformationRegistryError),
    DuplicateIntroductionClaim {
        transition: InformationTransitionKey,
        claim: TransitionCapabilityClaim,
    },
    ConflictingTransitionRegistration {
        key: InformationTransitionKey,
    },
    SelfTransition {
        transition: InformationTransitionKey,
        representation: RepresentationKey,
    },
    MissingIntroductionProvenance {
        transition: InformationTransitionKey,
        claim: TransitionCapabilityClaim,
    },
    RedundantIntroductionClaim {
        transition: InformationTransitionKey,
        claim: TransitionCapabilityClaim,
    },
    IntroductionNotInDestination {
        transition: InformationTransitionKey,
        claim: TransitionCapabilityClaim,
    },
    InvalidProvenanceForClaim {
        transition: InformationTransitionKey,
        claim: TransitionCapabilityClaim,
        provenance: PromotionProvenanceClass,
    },
    MissingDeclaredDiscard {
        transition: InformationTransitionKey,
        information: EcologicalInformation,
    },
    RedundantDeclaredDiscard {
        transition: InformationTransitionKey,
        information: EcologicalInformation,
    },
    PolicyRegistryKeyMismatch {
        expected: InformationPolicyRegistryKey,
        actual: InformationPolicyRegistryKey,
    },
    NoTransitionPath {
        source: RepresentationKey,
        destination: RepresentationKey,
    },
}

impl fmt::Display for InformationTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(error) => write!(formatter, "information policy error: {error}"),
            Self::DuplicateIntroductionClaim { transition, claim } => write!(
                formatter,
                "transition {transition:?} repeats introduction claim {claim:?}"
            ),
            Self::ConflictingTransitionRegistration { key } => {
                write!(formatter, "conflicting information transition registration {key:?}")
            }
            Self::SelfTransition {
                transition,
                representation,
            } => write!(
                formatter,
                "transition {transition:?} is a self-transition on {representation:?}"
            ),
            Self::MissingIntroductionProvenance { transition, claim } => write!(
                formatter,
                "transition {transition:?} introduces {claim:?} without provenance"
            ),
            Self::RedundantIntroductionClaim { transition, claim } => write!(
                formatter,
                "transition {transition:?} declares provenance for already available {claim:?}"
            ),
            Self::IntroductionNotInDestination { transition, claim } => write!(
                formatter,
                "transition {transition:?} declares {claim:?}, but destination does not advertise it"
            ),
            Self::InvalidProvenanceForClaim {
                transition,
                claim,
                provenance,
            } => write!(
                formatter,
                "transition {transition:?} cannot authorize {claim:?} with {provenance:?} provenance"
            ),
            Self::MissingDeclaredDiscard {
                transition,
                information,
            } => write!(
                formatter,
                "transition {transition:?} loses {information:?} without declaring the discard"
            ),
            Self::RedundantDeclaredDiscard {
                transition,
                information,
            } => write!(
                formatter,
                "transition {transition:?} declares discard of {information:?} that is preserved"
            ),
            Self::PolicyRegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "transition graph is bound to policy registry {expected:?}, got {actual:?}"
            ),
            Self::NoTransitionPath {
                source,
                destination,
            } => write!(
                formatter,
                "no registered information transition path from {source:?} to {destination:?}"
            ),
        }
    }
}

impl Error for InformationTransitionError {
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
        ClosureDomainToken, ClosureModelVersion, EcologicalAuthorityLevel, ErrorPpm,
        PopulationStatisticSet, QualifiedClosureEvidence,
    };
    use crate::information_registry::{
        ClosureEvidenceStatus, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
        RegisteredClosureEvidence,
    };

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(50, 1);
    const TRANSITIONS: InformationTransitionRegistryKey =
        InformationTransitionRegistryKey::new(60, 1);
    const MARGINALS: RepresentationKey = RepresentationKey::new(100, 1);
    const EXACT_STRATA: RepresentationKey = RepresentationKey::new(101, 1);
    const CLOSURE_STRATA: RepresentationKey = RepresentationKey::new(102, 1);
    const DERIVED: RepresentationKey = RepresentationKey::new(103, 1);
    const INTERMEDIATE: RepresentationKey = RepresentationKey::new(104, 1);
    const RETAINED: RetainedAuthorityKey = RetainedAuthorityKey(200, 1);
    const LOSSLESS: LosslessTransformKey = LosslessTransformKey(201, 1);
    const MEASUREMENT: MeasurementAuthorityKey = MeasurementAuthorityKey(202, 1);
    const CONDITIONAL: ConditionalDerivationKey = ConditionalDerivationKey(203, 1);
    const DIRECT: InformationTransitionKey = InformationTransitionKey::new(300, 1);
    const FIRST: InformationTransitionKey = InformationTransitionKey::new(301, 1);
    const SECOND: InformationTransitionKey = InformationTransitionKey::new(302, 1);
    const ALTERNATE_FIRST: InformationTransitionKey = InformationTransitionKey::new(399, 1);
    const CLOSURE_LINEAGE: EvidenceLineageToken = EvidenceLineageToken(400);

    fn joint() -> EcologicalInformation {
        EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE.union(PopulationStatisticSet::CONDITION),
        )
    }

    fn closure() -> QualifiedClosureEvidence {
        QualifiedClosureEvidence::new(
            ClosureModelVersion(1),
            ClosureDomainToken(9),
            ErrorPpm::new(10_000).unwrap(),
            CLOSURE_LINEAGE,
        )
    }

    fn base_claims() -> [(EcologicalInformation, CapabilityEvidence); 2] {
        [
            (EcologicalInformation::AgeDistribution, CapabilityEvidence::Exact),
            (
                EcologicalInformation::ConditionDistribution,
                CapabilityEvidence::Exact,
            ),
        ]
    }

    fn policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_closure_evidence(RegisteredClosureEvidence::new(
                closure(),
                ClosureEvidenceStatus::Qualified,
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                MARGINALS,
                EcologicalAuthorityLevel::Coarse,
                base_claims(),
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                EXACT_STRATA,
                EcologicalAuthorityLevel::Coarse,
                base_claims()
                    .into_iter()
                    .chain([(joint(), CapabilityEvidence::Exact)]),
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                CLOSURE_STRATA,
                EcologicalAuthorityLevel::Coarse,
                base_claims().into_iter().chain([(
                    joint(),
                    CapabilityEvidence::QualifiedClosure(closure()),
                )]),
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                DERIVED,
                EcologicalAuthorityLevel::Coarse,
                base_claims()
                    .into_iter()
                    .chain([(joint(), CapabilityEvidence::MeasurementOnly)]),
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                INTERMEDIATE,
                EcologicalAuthorityLevel::Coarse,
                base_claims(),
            ))
            .unwrap();
        builder.seal()
    }

    fn introduction_for(
        policy: &InformationPolicyRegistry,
        destination: RepresentationKey,
        provenance: PromotionProvenance,
    ) -> (TransitionCapabilityClaim, PromotionProvenance) {
        let evidence = policy
            .resolve_representation(destination)
            .unwrap()
            .capabilities()
            .claims()
            .get(&joint())
            .unwrap()
            .iter()
            .copied()
            .next()
            .unwrap();
        (TransitionCapabilityClaim::new(joint(), evidence), provenance)
    }

    fn transition(
        policy: &InformationPolicyRegistry,
        key: InformationTransitionKey,
        source: RepresentationKey,
        destination: RepresentationKey,
        provenance: PromotionProvenance,
    ) -> InformationTransitionDefinition {
        InformationTransitionDefinition::new(
            key,
            source,
            destination,
            [introduction_for(policy, destination, provenance)],
            [],
        )
        .unwrap()
    }

    #[test]
    fn closure_or_conditional_state_cannot_create_exact_covariance() {
        let policy = policy();
        for provenance in [
            PromotionProvenance::QualifiedClosure {
                evidence_lineage: CLOSURE_LINEAGE,
            },
            PromotionProvenance::ConditionalMicrostate { model: CONDITIONAL },
        ] {
            let edge = transition(&policy, DIRECT, MARGINALS, EXACT_STRATA, provenance);
            let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
            builder.register_transition(edge).unwrap();
            assert!(matches!(
                builder.seal(&policy),
                Err(InformationTransitionError::InvalidProvenanceForClaim { .. })
            ));
        }
    }

    #[test]
    fn exact_paths_keep_external_provenance_requirements_explicit() {
        let policy = policy();
        for (provenance, expected) in [
            (
                PromotionProvenance::RetainedExact { authority: RETAINED },
                PromotionEvidenceRequirement::RetainedAuthority(RETAINED),
            ),
            (
                PromotionProvenance::LosslessDerivation { transform: LOSSLESS },
                PromotionEvidenceRequirement::LosslessTransform(LOSSLESS),
            ),
            (
                PromotionProvenance::MeasurementAssimilation {
                    authority: MEASUREMENT,
                },
                PromotionEvidenceRequirement::MeasurementAuthority(MEASUREMENT),
            ),
        ] {
            let edge = transition(&policy, DIRECT, MARGINALS, EXACT_STRATA, provenance);
            let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
            builder.register_transition(edge).unwrap();
            let registry = builder.seal(&policy).unwrap();
            let plan = registry.plan(&policy, MARGINALS, EXACT_STRATA).unwrap();
            assert_eq!(plan.external_requirements(), &BTreeSet::from([expected]));
            assert!(!plan.is_self_contained_exact());
        }
    }

    #[test]
    fn approximate_and_conditional_paths_never_become_exact_by_determinism() {
        let policy = policy();
        let closure_edge = transition(
            &policy,
            DIRECT,
            MARGINALS,
            CLOSURE_STRATA,
            PromotionProvenance::QualifiedClosure {
                evidence_lineage: CLOSURE_LINEAGE,
            },
        );
        let mut closure_builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        closure_builder.register_transition(closure_edge).unwrap();
        let closure_plan = closure_builder
            .seal(&policy)
            .unwrap()
            .plan(&policy, MARGINALS, CLOSURE_STRATA)
            .unwrap();
        assert_eq!(closure_plan.closure_evidence(), &BTreeSet::from([CLOSURE_LINEAGE]));
        assert!(!closure_plan.is_self_contained_exact());

        let derived_edge = transition(
            &policy,
            DIRECT,
            MARGINALS,
            DERIVED,
            PromotionProvenance::ConditionalMicrostate { model: CONDITIONAL },
        );
        let mut derived_builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        derived_builder.register_transition(derived_edge).unwrap();
        let derived_plan = derived_builder
            .seal(&policy)
            .unwrap()
            .plan(&policy, MARGINALS, DERIVED)
            .unwrap();
        assert_eq!(
            derived_plan.conditional_derivations(),
            &BTreeSet::from([CONDITIONAL])
        );
        assert!(!derived_plan.is_self_contained_exact());
    }

    #[test]
    fn collapse_must_declare_exact_information_loss() {
        let policy = policy();
        let edge = InformationTransitionDefinition::new(
            DIRECT,
            EXACT_STRATA,
            CLOSURE_STRATA,
            [],
            [],
        )
        .unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(edge).unwrap();
        assert!(matches!(
            builder.seal(&policy),
            Err(InformationTransitionError::MissingDeclaredDiscard {
                information,
                ..
            }) if information == joint()
        ));
    }

    #[test]
    fn declared_loss_is_visible_and_does_not_create_an_inverse_transition() {
        let policy = policy();
        let collapse = InformationTransitionDefinition::new(
            DIRECT,
            EXACT_STRATA,
            MARGINALS,
            [],
            [joint()],
        )
        .unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(collapse).unwrap();
        let registry = builder.seal(&policy).unwrap();
        let collapse_plan = registry.plan(&policy, EXACT_STRATA, MARGINALS).unwrap();
        assert_eq!(collapse_plan.discarded_information(), &BTreeSet::from([joint()]));
        assert!(!collapse_plan.is_self_contained_exact());
        assert!(matches!(
            registry.plan(&policy, MARGINALS, EXACT_STRATA),
            Err(InformationTransitionError::NoTransitionPath { .. })
        ));
    }

    #[test]
    fn deterministic_shortest_path_uses_transition_key_order() {
        let policy = policy();
        let first = InformationTransitionDefinition::new(
            FIRST,
            MARGINALS,
            INTERMEDIATE,
            [],
            [],
        )
        .unwrap();
        let second = transition(
            &policy,
            SECOND,
            INTERMEDIATE,
            EXACT_STRATA,
            PromotionProvenance::LosslessDerivation { transform: LOSSLESS },
        );
        let alternate_first = transition(
            &policy,
            ALTERNATE_FIRST,
            MARGINALS,
            CLOSURE_STRATA,
            PromotionProvenance::QualifiedClosure {
                evidence_lineage: CLOSURE_LINEAGE,
            },
        );
        let alternate_second = InformationTransitionDefinition::new(
            InformationTransitionKey::new(400, 1),
            CLOSURE_STRATA,
            EXACT_STRATA,
            [(
                TransitionCapabilityClaim::new(joint(), CapabilityEvidence::Exact),
                PromotionProvenance::MeasurementAssimilation {
                    authority: MEASUREMENT,
                },
            )],
            [],
        )
        .unwrap();

        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        for edge in [alternate_second, alternate_first, second, first] {
            builder.register_transition(edge).unwrap();
        }
        let registry = builder.seal(&policy).unwrap();
        let plan = registry.plan(&policy, MARGINALS, EXACT_STRATA).unwrap();
        assert_eq!(plan.transitions(), &[FIRST, SECOND]);
    }
}
