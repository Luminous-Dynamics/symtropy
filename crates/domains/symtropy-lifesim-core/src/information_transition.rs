// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Information-preserving reachability for ecological fidelity transitions.
//!
//! A richer representation being sufficient for a process does not imply that
//! the current canonical state can legitimately become that representation.
//! This module keeps those questions separate. Transition policy is sealed and
//! registry-owned; callers may request a source/target pair, but cannot inject
//! an ad hoc edge that manufactures forgotten exact information.

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

/// Identity/version of one sealed information-transition policy graph.
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

/// Stable identity/version of one semantic transition edge.
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

/// Stable identity/version of retained latent exact authority used by R0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RetainedAuthorityKey {
    id: u128,
    version: u32,
}

impl RetainedAuthorityKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }
}

/// Stable identity/version of a deterministic lossless transform used by R1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LosslessTransformKey {
    id: u128,
    version: u32,
}

impl LosslessTransformKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }
}

/// Stable identity/version of qualified measurement authority used by R3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasurementAuthorityKey {
    id: u128,
    version: u32,
}

impl MeasurementAuthorityKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }
}

/// Stable identity/version of a conditional microstate model used by R4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConditionalDerivationKey {
    id: u128,
    version: u32,
}

impl ConditionalDerivationKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }
}

/// The five provenance classes frozen by the Living World authority contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromotionProvenanceClass {
    /// R0: exact information already existed canonically and is being revealed.
    RetainedExact,
    /// R1: exact information follows uniquely from a registered lossless transform.
    LosslessDerivation,
    /// R2: approximate information is reconstructed under qualified closure evidence.
    QualifiedClosure,
    /// R3: new exact information enters through qualified measurement/assimilation.
    MeasurementAssimilation,
    /// R4: conditional D-state fills unresolved degrees of freedom non-authoritatively.
    ConditionalMicrostate,
}

/// Provenance for one capability that appears at a transition destination.
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

/// One concrete capability claim at a transition destination.
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

/// One registered semantic representation transition.
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

    pub fn introductions(
        &self,
    ) -> &BTreeMap<TransitionCapabilityClaim, PromotionProvenance> {
        &self.introductions
    }

    pub fn discarded_information(&self) -> &BTreeSet<EcologicalInformation> {
        &self.discarded_information
    }
}

/// Bootstrap-only builder for one transition-policy generation.
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

    /// Validate every transition against one sealed information-policy registry
    /// and seal the immutable reachability graph.
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

/// Immutable transition graph bound to one information-policy registry generation.
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

    /// Find the shortest transition path. Equal-length alternatives are chosen
    /// deterministically by transition-key ordering, never by thread or map order.
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
            return Ok(InformationTransitionPlan {
                transition_registry_key: self.key,
                policy_registry_key: self.policy_registry_key,
                source,
                destination,
                transitions: Vec::new(),
                external_requirements: BTreeSet::new(),
                closure_evidence: BTreeSet::new(),
                conditional_derivations: BTreeSet::new(),
            });
        }

        let mut queue = VecDeque::from([source]);
        let mut visited = BTreeSet::from([source]);
        let mut predecessor: BTreeMap<
            RepresentationKey,
            (RepresentationKey, InformationTransitionKey),
        > = BTreeMap::new();

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

        for key in &reversed {
            let transition = self
                .transitions
                .get(key)
                .expect("planned transition remains registered");
            for provenance in transition.introductions.values().copied() {
                match provenance {
                    PromotionProvenance::RetainedExact { authority } => {
                        external_requirements
                            .insert(PromotionEvidenceRequirement::RetainedAuthority(authority));
                    }
                    PromotionProvenance::LosslessDerivation { .. } => {}
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
        }
    }
}

/// External exact-authority prerequisites that a low-level structural plan cannot
/// authenticate by itself. Higher orchestration must resolve these before commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PromotionEvidenceRequirement {
    RetainedAuthority(RetainedAuthorityKey),
    MeasurementAuthority(MeasurementAuthorityKey),
}

/// Deterministic, read-only transition plan. This proves policy reachability; it
/// does not mutate ecology and does not authenticate opaque external authorities.
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

    /// True only for a path whose newly introduced authority is losslessly
    /// derivable from current canonical state without external retained/measurement
    /// evidence, closure approximation, or conditional D-state.
    pub fn is_self_contained_exact(&self) -> bool {
        self.external_requirements.is_empty()
            && self.closure_evidence.is_empty()
            && self.conditional_derivations.is_empty()
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
            let already_available = source_covers_claim(source, claim);
            match (already_available, transition.introductions.get(&claim).copied()) {
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

fn source_covers_claim(
    source: &RepresentationCapabilities,
    target: TransitionCapabilityClaim,
) -> bool {
    source.claims().iter().any(|(available_information, evidence_set)| {
        available_information.covers(target.information)
            && evidence_set
                .iter()
                .copied()
                .any(|available_evidence| evidence_preserves_claim(available_evidence, target.evidence))
    })
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

fn evidence_preserves_claim(available: CapabilityEvidence, target: CapabilityEvidence) -> bool {
    match (available, target) {
        (CapabilityEvidence::Exact, CapabilityEvidence::Exact) => true,
        (CapabilityEvidence::Exact, CapabilityEvidence::QualifiedClosure(_)) => true,
        (CapabilityEvidence::Exact, CapabilityEvidence::MeasurementOnly) => true,
        (
            CapabilityEvidence::QualifiedClosure(available),
            CapabilityEvidence::QualifiedClosure(target),
        ) => available == target,
        (CapabilityEvidence::QualifiedClosure(_), CapabilityEvidence::MeasurementOnly) => true,
        (CapabilityEvidence::MeasurementOnly, CapabilityEvidence::MeasurementOnly) => true,
        _ => false,
    }
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
            let preserved = source_evidence.iter().copied().all(|evidence| {
                destination.claims().iter().any(|(destination_information, destination_evidence)| {
                    destination_information.covers(*information)
                        && destination_evidence
                            .iter()
                            .copied()
                            .any(|target| evidence_preserves_claim(evidence, target))
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
    const MEASUREMENT_ONLY: RepresentationKey = RepresentationKey::new(103, 1);
    const INTERMEDIATE: RepresentationKey = RepresentationKey::new(104, 1);

    const RETAINED: RetainedAuthorityKey = RetainedAuthorityKey::new(200, 1);
    const LOSSLESS: LosslessTransformKey = LosslessTransformKey::new(201, 1);
    const MEASUREMENT: MeasurementAuthorityKey = MeasurementAuthorityKey::new(202, 1);
    const CONDITIONAL: ConditionalDerivationKey = ConditionalDerivationKey::new(203, 1);

    const DIRECT: InformationTransitionKey = InformationTransitionKey::new(300, 1);
    const FIRST: InformationTransitionKey = InformationTransitionKey::new(301, 1);
    const SECOND: InformationTransitionKey = InformationTransitionKey::new(302, 1);
    const ALTERNATE_FIRST: InformationTransitionKey = InformationTransitionKey::new(399, 1);

    const CLOSURE_LINEAGE: EvidenceLineageToken = EvidenceLineageToken(400);

    fn joint_information() -> EcologicalInformation {
        EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE.union(PopulationStatisticSet::CONDITION),
        )
    }

    fn closure_evidence() -> QualifiedClosureEvidence {
        QualifiedClosureEvidence::new(
            ClosureModelVersion(1),
            ClosureDomainToken(9),
            ErrorPpm::new(10_000).unwrap(),
            CLOSURE_LINEAGE,
        )
    }

    fn policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_closure_evidence(RegisteredClosureEvidence::new(
                closure_evidence(),
                ClosureEvidenceStatus::Qualified,
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                MARGINALS,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::AgeDistribution, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::ConditionDistribution,
                        CapabilityEvidence::Exact,
                    ),
                ],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                EXACT_STRATA,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::AgeDistribution, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::ConditionDistribution,
                        CapabilityEvidence::Exact,
                    ),
                    (joint_information(), CapabilityEvidence::Exact),
                ],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                CLOSURE_STRATA,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::AgeDistribution, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::ConditionDistribution,
                        CapabilityEvidence::Exact,
                    ),
                    (
                        joint_information(),
                        CapabilityEvidence::QualifiedClosure(closure_evidence()),
                    ),
                ],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                MEASUREMENT_ONLY,
                EcologicalAuthorityLevel::Coarse,
                [(joint_information(), CapabilityEvidence::MeasurementOnly)],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                INTERMEDIATE,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::AgeDistribution, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::ConditionDistribution,
                        CapabilityEvidence::Exact,
                    ),
                ],
            ))
            .unwrap();
        builder.seal()
    }

    fn transition(
        key: InformationTransitionKey,
        source: RepresentationKey,
        destination: RepresentationKey,
        provenance: PromotionProvenance,
    ) -> InformationTransitionDefinition {
        InformationTransitionDefinition::new(
            key,
            source,
            destination,
            [(TransitionCapabilityClaim::new(
                joint_information(),
                policy()
                    .resolve_representation(destination)
                    .unwrap()
                    .capabilities()
                    .claims()
                    .get(&joint_information())
                    .unwrap()
                    .iter()
                    .copied()
                    .next()
                    .unwrap(),
            ), provenance)],
            [],
        )
        .unwrap()
    }

    #[test]
    fn exact_joint_information_cannot_be_created_by_closure_reconstruction() {
        let policy = policy();
        let transition = transition(
            DIRECT,
            MARGINALS,
            EXACT_STRATA,
            PromotionProvenance::QualifiedClosure {
                evidence_lineage: CLOSURE_LINEAGE,
            },
        );
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(transition).unwrap();

        assert!(matches!(
            builder.seal(&policy),
            Err(InformationTransitionError::InvalidProvenanceForClaim {
                provenance: PromotionProvenanceClass::QualifiedClosure,
                ..
            })
        ));
    }

    #[test]
    fn exact_joint_information_cannot_be_created_by_conditional_microstate() {
        let policy = policy();
        let transition = transition(
            DIRECT,
            MARGINALS,
            EXACT_STRATA,
            PromotionProvenance::ConditionalMicrostate { model: CONDITIONAL },
        );
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(transition).unwrap();

        assert!(matches!(
            builder.seal(&policy),
            Err(InformationTransitionError::InvalidProvenanceForClaim {
                provenance: PromotionProvenanceClass::ConditionalMicrostate,
                ..
            })
        ));
    }

    #[test]
    fn retained_exact_information_is_reachable_but_requires_external_authority() {
        let policy = policy();
        let transition = transition(
            DIRECT,
            MARGINALS,
            EXACT_STRATA,
            PromotionProvenance::RetainedExact { authority: RETAINED },
        );
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(transition).unwrap();
        let registry = builder.seal(&policy).unwrap();
        let plan = registry.plan(&policy, MARGINALS, EXACT_STRATA).unwrap();

        assert_eq!(
            plan.external_requirements(),
            &BTreeSet::from([PromotionEvidenceRequirement::RetainedAuthority(RETAINED)])
        );
        assert!(!plan.is_self_contained_exact());
    }

    #[test]
    fn lossless_derivation_is_self_contained_exact_reachability() {
        let policy = policy();
        let transition = transition(
            DIRECT,
            MARGINALS,
            EXACT_STRATA,
            PromotionProvenance::LosslessDerivation { transform: LOSSLESS },
        );
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(transition).unwrap();
        let registry = builder.seal(&policy).unwrap();
        let plan = registry.plan(&policy, MARGINALS, EXACT_STRATA).unwrap();

        assert!(plan.is_self_contained_exact());
    }

    #[test]
    fn measurement_assimilation_requires_external_measurement_authority() {
        let policy = policy();
        let transition = transition(
            DIRECT,
            MARGINALS,
            EXACT_STRATA,
            PromotionProvenance::MeasurementAssimilation {
                authority: MEASUREMENT,
            },
        );
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(transition).unwrap();
        let registry = builder.seal(&policy).unwrap();
        let plan = registry.plan(&policy, MARGINALS, EXACT_STRATA).unwrap();

        assert_eq!(
            plan.external_requirements(),
            &BTreeSet::from([PromotionEvidenceRequirement::MeasurementAuthority(
                MEASUREMENT
            )])
        );
        assert!(!plan.is_self_contained_exact());
    }

    #[test]
    fn qualified_reconstruction_remains_closure_evidence() {
        let policy = policy();
        let transition = transition(
            DIRECT,
            MARGINALS,
            CLOSURE_STRATA,
            PromotionProvenance::QualifiedClosure {
                evidence_lineage: CLOSURE_LINEAGE,
            },
        );
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(transition).unwrap();
        let registry = builder.seal(&policy).unwrap();
        let plan = registry.plan(&policy, MARGINALS, CLOSURE_STRATA).unwrap();

        assert_eq!(plan.closure_evidence(), &BTreeSet::from([CLOSURE_LINEAGE]));
        assert!(!plan.is_self_contained_exact());
    }

    #[test]
    fn conditional_microstate_can_only_land_in_measurement_only_capability() {
        let policy = policy();
        let transition = transition(
            DIRECT,
            MARGINALS,
            MEASUREMENT_ONLY,
            PromotionProvenance::ConditionalMicrostate { model: CONDITIONAL },
        );
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(transition).unwrap();
        let registry = builder.seal(&policy).unwrap();
        let plan = registry.plan(&policy, MARGINALS, MEASUREMENT_ONLY).unwrap();

        assert_eq!(
            plan.conditional_derivations(),
            &BTreeSet::from([CONDITIONAL])
        );
        assert!(!plan.is_self_contained_exact());
    }

    #[test]
    fn collapse_must_declare_lost_exact_covariance() {
        let policy = policy();
        let transition = InformationTransitionDefinition::new(
            DIRECT,
            EXACT_STRATA,
            MARGINALS,
            [],
            [],
        )
        .unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(transition).unwrap();

        assert!(matches!(
            builder.seal(&policy),
            Err(InformationTransitionError::MissingDeclaredDiscard {
                information,
                ..
            }) if information == joint_information()
        ));
    }

    #[test]
    fn declared_collapse_loss_does_not_make_exact_restoration_reachable() {
        let policy = policy();
        let collapse = InformationTransitionDefinition::new(
            DIRECT,
            EXACT_STRATA,
            MARGINALS,
            [],
            [joint_information()],
        )
        .unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        builder.register_transition(collapse).unwrap();
        let registry = builder.seal(&policy).unwrap();

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
            SECOND,
            INTERMEDIATE,
            EXACT_STRATA,
            PromotionProvenance::LosslessDerivation { transform: LOSSLESS },
        );
        let alternate_first = InformationTransitionDefinition::new(
            ALTERNATE_FIRST,
            MARGINALS,
            CLOSURE_STRATA,
            [(TransitionCapabilityClaim::new(
                joint_information(),
                CapabilityEvidence::QualifiedClosure(closure_evidence()),
            ), PromotionProvenance::QualifiedClosure {
                evidence_lineage: CLOSURE_LINEAGE,
            })],
            [],
        )
        .unwrap();
        let alternate_second = InformationTransitionDefinition::new(
            InformationTransitionKey::new(400, 1),
            CLOSURE_STRATA,
            EXACT_STRATA,
            [(TransitionCapabilityClaim::new(
                joint_information(),
                CapabilityEvidence::Exact,
            ), PromotionProvenance::MeasurementAssimilation {
                authority: MEASUREMENT,
            })],
            [],
        )
        .unwrap();

        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITIONS);
        for transition in [alternate_second, alternate_first, second, first] {
            builder.register_transition(transition).unwrap();
        }
        let registry = builder.seal(&policy).unwrap();
        let plan = registry.plan(&policy, MARGINALS, EXACT_STRATA).unwrap();

        assert_eq!(plan.transitions(), &[FIRST, SECOND]);
    }
}
