// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Authority-bound, state-specific proofs for transition applicability domains.
//!
//! [`crate::transition_applicability::TransitionDomainKey`] is only a semantic
//! descriptor. This module adds the higher authority layer that qualifies a
//! deterministic evaluator and proves that one exact canonical population state
//! lies inside that domain. Callers never submit `domain_satisfied: bool`, and a
//! proof cannot be constructed directly.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::applicability_policy_manifest::{
    ApplicabilityPolicyIdentityError, ManifestBoundTransitionApplicabilityPolicy,
    TransitionApplicabilityAuthorityStamp,
};
use crate::information::{EvidenceLineageToken, RepresentationKey};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::InformationRegistryError;
use crate::population::PopulationState;
use crate::population_manifest::{
    ManifestBoundPopulationState, PopulationStateIdentityError, PopulationStateManifest,
};
use crate::transition_applicability::{TransitionApplicability, TransitionDomainKey};

/// Semantic identity/version of one sealed domain-authority registry generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionDomainAuthorityRegistryKey {
    id: u128,
    version: u32,
}

impl TransitionDomainAuthorityRegistryKey {
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

/// Canonical authority scope bound into one per-state domain proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionDomainAuthorityScope {
    id: u128,
    version: u32,
}

impl TransitionDomainAuthorityScope {
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

/// Canonical snapshot identity for a domain evaluation subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionDomainSnapshotId(pub u128);

/// Canonical source revision/generation for a domain evaluation subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionDomainSourceRevision(pub u64);

/// Version of the deterministic predicate/profile admitted by qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionDomainPredicateProfileVersion(pub u32);

/// V0 built-in predicates. Arbitrary scripts are intentionally not admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransitionDomainPredicate {
    /// Exact age x condition covariance is uniquely determined because the
    /// canonical population occupies exactly one age band and one condition
    /// band. Empty populations and any multi-bin marginal fail the theorem.
    UniqueAgeConditionFromSingletonMarginals,
}

/// Opaque exact evaluator implementation/artifact identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionDomainEvaluatorFingerprint(Vec<u8>);

impl TransitionDomainEvaluatorFingerprint {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, TransitionDomainAuthorityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(TransitionDomainAuthorityError::EmptyEvaluatorFingerprint);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Optional execution/toolchain capsule identity for evaluator qualification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionDomainExecutionCapsuleFingerprint(Vec<u8>);

impl TransitionDomainExecutionCapsuleFingerprint {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, TransitionDomainAuthorityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(TransitionDomainAuthorityError::EmptyExecutionCapsuleFingerprint);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Settled authority status of one qualified domain evaluator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransitionDomainAuthorityStatus {
    Qualified,
    Revoked,
    Superseded,
}

/// Qualification record for one semantic domain descriptor.
///
/// Constructing this record is bootstrap/evidence ingestion, not a proof that an
/// arbitrary evaluator is correct. Canonical runtime authority comes from the
/// reviewed sealed registry generation that it owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDomainQualification {
    domain: TransitionDomainKey,
    source_representation: RepresentationKey,
    predicate: TransitionDomainPredicate,
    predicate_profile_version: TransitionDomainPredicateProfileVersion,
    evaluator_fingerprint: TransitionDomainEvaluatorFingerprint,
    qualification_evidence: EvidenceLineageToken,
    execution_capsule: Option<TransitionDomainExecutionCapsuleFingerprint>,
    status: TransitionDomainAuthorityStatus,
}

impl TransitionDomainQualification {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        domain: TransitionDomainKey,
        source_representation: RepresentationKey,
        predicate: TransitionDomainPredicate,
        predicate_profile_version: TransitionDomainPredicateProfileVersion,
        evaluator_fingerprint: TransitionDomainEvaluatorFingerprint,
        qualification_evidence: EvidenceLineageToken,
        execution_capsule: Option<TransitionDomainExecutionCapsuleFingerprint>,
        status: TransitionDomainAuthorityStatus,
    ) -> Self {
        Self {
            domain,
            source_representation,
            predicate,
            predicate_profile_version,
            evaluator_fingerprint,
            qualification_evidence,
            execution_capsule,
            status,
        }
    }

    pub const fn domain(&self) -> TransitionDomainKey {
        self.domain
    }

    pub const fn source_representation(&self) -> RepresentationKey {
        self.source_representation
    }

    pub const fn predicate(&self) -> TransitionDomainPredicate {
        self.predicate
    }

    pub const fn predicate_profile_version(&self) -> TransitionDomainPredicateProfileVersion {
        self.predicate_profile_version
    }

    pub const fn evaluator_fingerprint(&self) -> &TransitionDomainEvaluatorFingerprint {
        &self.evaluator_fingerprint
    }

    pub const fn qualification_evidence(&self) -> EvidenceLineageToken {
        self.qualification_evidence
    }

    pub const fn execution_capsule(&self) -> Option<&TransitionDomainExecutionCapsuleFingerprint> {
        self.execution_capsule.as_ref()
    }

    pub const fn status(&self) -> TransitionDomainAuthorityStatus {
        self.status
    }
}

/// Bootstrap-only registry builder. Conflicting reuse of one domain key fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDomainAuthorityRegistryBuilder {
    key: TransitionDomainAuthorityRegistryKey,
    qualifications: BTreeMap<TransitionDomainKey, TransitionDomainQualification>,
}

impl TransitionDomainAuthorityRegistryBuilder {
    pub const fn new(key: TransitionDomainAuthorityRegistryKey) -> Self {
        Self {
            key,
            qualifications: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        qualification: TransitionDomainQualification,
    ) -> Result<(), TransitionDomainAuthorityError> {
        use std::collections::btree_map::Entry;

        let domain = qualification.domain();
        match self.qualifications.entry(domain) {
            Entry::Vacant(entry) => {
                entry.insert(qualification);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &qualification => Ok(()),
            Entry::Occupied(_) => {
                Err(TransitionDomainAuthorityError::ConflictingDomainRegistration { domain })
            }
        }
    }

    /// Seal qualifications against one exact information-policy corpus.
    ///
    /// This proves that every admitted source representation descriptor exists in
    /// that exact corpus. Same semantic policy key with different content does
    /// not share this registry authority.
    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<TransitionDomainAuthorityRegistry, TransitionDomainAuthorityError> {
        for qualification in self.qualifications.values() {
            policy
                .registry()
                .resolve_representation(qualification.source_representation())
                .map_err(|source| TransitionDomainAuthorityError::PolicyRegistry { source })?;
        }

        let policy_authority = policy.authority_stamp().clone();
        let authority = TransitionDomainAuthorityStamp {
            key: self.key,
            policy_authority,
            qualifications: self.qualifications.clone(),
        };

        Ok(TransitionDomainAuthorityRegistry {
            qualifications: self.qualifications,
            authority,
        })
    }
}

/// Exact in-process authority identity for one sealed domain evaluator corpus.
///
/// V0 deliberately uses the complete structured qualification map rather than a
/// weak short hash. A future portable manifest may encode this structure without
/// changing the authority theorem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDomainAuthorityStamp {
    key: TransitionDomainAuthorityRegistryKey,
    policy_authority: InformationPolicyAuthorityStamp,
    qualifications: BTreeMap<TransitionDomainKey, TransitionDomainQualification>,
}

impl TransitionDomainAuthorityStamp {
    pub const fn key(&self) -> TransitionDomainAuthorityRegistryKey {
        self.key
    }

    pub const fn policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.policy_authority
    }

    pub fn qualification(
        &self,
        domain: TransitionDomainKey,
    ) -> Option<&TransitionDomainQualification> {
        self.qualifications.get(&domain)
    }
}

/// Immutable canonical runtime authority for transition-domain evaluators.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDomainAuthorityRegistry {
    qualifications: BTreeMap<TransitionDomainKey, TransitionDomainQualification>,
    authority: TransitionDomainAuthorityStamp,
}

impl TransitionDomainAuthorityRegistry {
    pub const fn authority_stamp(&self) -> &TransitionDomainAuthorityStamp {
        &self.authority
    }

    pub fn qualification(
        &self,
        domain: TransitionDomainKey,
    ) -> Option<&TransitionDomainQualification> {
        self.qualifications.get(&domain)
    }

    fn validate_policy(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), TransitionDomainAuthorityError> {
        self.authority
            .policy_authority
            .validate_registry(policy.registry())
            .map_err(TransitionDomainAuthorityError::PolicyIdentity)
    }

    /// Prove that one exact canonical population state satisfies a required
    /// transition domain.
    ///
    /// The exact applicability corpus is validated first. The domain must be
    /// required by at least one edge whose source representation equals the
    /// evaluation subject. Only a currently `Qualified` evaluator may run.
    pub fn evaluate_population_domain(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        domain: TransitionDomainKey,
        subject: &TransitionDomainEvaluationSubject,
        population: &PopulationState,
    ) -> Result<TransitionDomainProof, TransitionDomainAuthorityError> {
        self.validate_policy(policy)?;
        validate_applicability(applicability, &self.authority.policy_authority)?;

        let qualification = self
            .qualifications
            .get(&domain)
            .ok_or(TransitionDomainAuthorityError::UnknownDomain { domain })?;
        if qualification.status() != TransitionDomainAuthorityStatus::Qualified {
            return Err(TransitionDomainAuthorityError::DomainAuthorityNotQualified {
                domain,
                status: qualification.status(),
            });
        }
        if qualification.source_representation() != subject.source_representation() {
            return Err(TransitionDomainAuthorityError::SourceRepresentationMismatch {
                expected: qualification.source_representation(),
                actual: subject.source_representation(),
            });
        }
        if !applicability_requires_domain_for_source(
            applicability,
            domain,
            subject.source_representation(),
        ) {
            return Err(TransitionDomainAuthorityError::DomainNotRequiredByApplicability {
                domain,
                source: subject.source_representation(),
            });
        }

        subject.validate_population(population)?;
        if !evaluate_predicate(qualification.predicate(), population) {
            return Err(TransitionDomainAuthorityError::DomainNotSatisfied { domain });
        }

        Ok(TransitionDomainProof {
            domain_authority: self.authority.clone(),
            applicability_authority: applicability.authority_stamp().clone(),
            qualification: qualification.clone(),
            subject: subject.clone(),
        })
    }
}

/// Exact source state evaluated by a qualified transition-domain authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDomainEvaluationSubject {
    scope: TransitionDomainAuthorityScope,
    source_representation: RepresentationKey,
    snapshot: TransitionDomainSnapshotId,
    source_revision: TransitionDomainSourceRevision,
    population_manifest: PopulationStateManifest,
}

impl TransitionDomainEvaluationSubject {
    pub fn from_population(
        scope: TransitionDomainAuthorityScope,
        source_representation: RepresentationKey,
        snapshot: TransitionDomainSnapshotId,
        source_revision: TransitionDomainSourceRevision,
        population: &PopulationState,
    ) -> Result<Self, TransitionDomainAuthorityError> {
        let bound = ManifestBoundPopulationState::new(population)
            .map_err(TransitionDomainAuthorityError::PopulationIdentity)?;
        Ok(Self {
            scope,
            source_representation,
            snapshot,
            source_revision,
            population_manifest: bound.manifest().clone(),
        })
    }

    pub const fn scope(&self) -> TransitionDomainAuthorityScope {
        self.scope
    }

    pub const fn source_representation(&self) -> RepresentationKey {
        self.source_representation
    }

    pub const fn snapshot(&self) -> TransitionDomainSnapshotId {
        self.snapshot
    }

    pub const fn source_revision(&self) -> TransitionDomainSourceRevision {
        self.source_revision
    }

    pub const fn population_manifest(&self) -> &PopulationStateManifest {
        &self.population_manifest
    }

    pub fn validate_population(
        &self,
        population: &PopulationState,
    ) -> Result<(), TransitionDomainAuthorityError> {
        let current = ManifestBoundPopulationState::new(population)
            .map_err(TransitionDomainAuthorityError::PopulationIdentity)?;
        if current.manifest() != &self.population_manifest {
            return Err(TransitionDomainAuthorityError::PopulationManifestMismatch);
        }
        Ok(())
    }
}

/// Satisfied proof minted only by [`TransitionDomainAuthorityRegistry`].
///
/// There is deliberately no public constructor and no boolean result field. An
/// unsatisfied domain produces `DomainNotSatisfied` and therefore no canonical
/// proof object at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDomainProof {
    domain_authority: TransitionDomainAuthorityStamp,
    applicability_authority: TransitionApplicabilityAuthorityStamp,
    qualification: TransitionDomainQualification,
    subject: TransitionDomainEvaluationSubject,
}

impl TransitionDomainProof {
    pub const fn domain(&self) -> TransitionDomainKey {
        self.qualification.domain()
    }

    pub const fn domain_authority(&self) -> &TransitionDomainAuthorityStamp {
        &self.domain_authority
    }

    pub const fn applicability_authority(&self) -> &TransitionApplicabilityAuthorityStamp {
        &self.applicability_authority
    }

    pub const fn qualification(&self) -> &TransitionDomainQualification {
        &self.qualification
    }

    pub const fn subject(&self) -> &TransitionDomainEvaluationSubject {
        &self.subject
    }

    /// Revalidate every authority/state identity before a prepared transition
    /// consumes this proof.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        registry: &TransitionDomainAuthorityRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        current_subject: &TransitionDomainEvaluationSubject,
        current_population: &PopulationState,
    ) -> Result<(), TransitionDomainAuthorityError> {
        registry.validate_policy(policy)?;
        if registry.authority_stamp() != &self.domain_authority {
            return Err(TransitionDomainAuthorityError::DomainAuthorityChanged);
        }

        validate_applicability(applicability, &self.domain_authority.policy_authority)?;
        if applicability.authority_stamp() != &self.applicability_authority {
            return Err(TransitionDomainAuthorityError::ApplicabilityAuthorityChanged);
        }

        let current_qualification = registry
            .qualification(self.domain())
            .ok_or(TransitionDomainAuthorityError::UnknownDomain {
                domain: self.domain(),
            })?;
        if current_qualification != &self.qualification {
            return Err(TransitionDomainAuthorityError::DomainQualificationChanged {
                domain: self.domain(),
            });
        }
        if current_qualification.status() != TransitionDomainAuthorityStatus::Qualified {
            return Err(TransitionDomainAuthorityError::DomainAuthorityNotQualified {
                domain: self.domain(),
                status: current_qualification.status(),
            });
        }

        if current_subject != &self.subject {
            return Err(TransitionDomainAuthorityError::EvaluationSubjectChanged);
        }
        current_subject.validate_population(current_population)?;

        if !applicability_requires_domain_for_source(
            applicability,
            self.domain(),
            current_subject.source_representation(),
        ) {
            return Err(TransitionDomainAuthorityError::DomainNotRequiredByApplicability {
                domain: self.domain(),
                source: current_subject.source_representation(),
            });
        }
        if !evaluate_predicate(current_qualification.predicate(), current_population) {
            return Err(TransitionDomainAuthorityError::DomainNotSatisfied {
                domain: self.domain(),
            });
        }
        Ok(())
    }
}

fn evaluate_predicate(predicate: TransitionDomainPredicate, population: &PopulationState) -> bool {
    match predicate {
        TransitionDomainPredicate::UniqueAgeConditionFromSingletonMarginals => {
            population.age_distribution().bins().count() == 1
                && population.condition_distribution().bins().count() == 1
        }
    }
}

fn validate_applicability(
    applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    expected_policy: &InformationPolicyAuthorityStamp,
) -> Result<(), TransitionDomainAuthorityError> {
    applicability
        .authority_stamp()
        .validate(applicability.policy(), applicability.transitions())
        .map_err(TransitionDomainAuthorityError::ApplicabilityIdentity)?;

    if applicability
        .authority_stamp()
        .transition_authority()
        .policy_authority()
        != expected_policy
    {
        return Err(TransitionDomainAuthorityError::ApplicabilityPolicyAuthorityMismatch);
    }
    Ok(())
}

fn applicability_requires_domain_for_source(
    applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    domain: TransitionDomainKey,
    source: RepresentationKey,
) -> bool {
    applicability
        .transitions()
        .registry()
        .transitions()
        .any(|(transition_key, transition)| {
            transition.source() == source
                && applicability.policy().applicability(*transition_key)
                    == Some(TransitionApplicability::RegisteredDomain(domain))
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionDomainAuthorityError {
    EmptyEvaluatorFingerprint,
    EmptyExecutionCapsuleFingerprint,
    ConflictingDomainRegistration {
        domain: TransitionDomainKey,
    },
    PolicyRegistry {
        source: InformationRegistryError,
    },
    PolicyIdentity(InformationPolicyIdentityError),
    ApplicabilityIdentity(ApplicabilityPolicyIdentityError),
    ApplicabilityPolicyAuthorityMismatch,
    UnknownDomain {
        domain: TransitionDomainKey,
    },
    DomainAuthorityNotQualified {
        domain: TransitionDomainKey,
        status: TransitionDomainAuthorityStatus,
    },
    SourceRepresentationMismatch {
        expected: RepresentationKey,
        actual: RepresentationKey,
    },
    DomainNotRequiredByApplicability {
        domain: TransitionDomainKey,
        source: RepresentationKey,
    },
    PopulationIdentity(PopulationStateIdentityError),
    PopulationManifestMismatch,
    DomainNotSatisfied {
        domain: TransitionDomainKey,
    },
    DomainAuthorityChanged,
    ApplicabilityAuthorityChanged,
    DomainQualificationChanged {
        domain: TransitionDomainKey,
    },
    EvaluationSubjectChanged,
}

impl fmt::Display for TransitionDomainAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEvaluatorFingerprint => {
                write!(formatter, "transition-domain evaluator fingerprint must not be empty")
            }
            Self::EmptyExecutionCapsuleFingerprint => write!(
                formatter,
                "transition-domain execution capsule fingerprint must not be empty"
            ),
            Self::ConflictingDomainRegistration { domain } => write!(
                formatter,
                "conflicting transition-domain qualification for {domain:?}"
            ),
            Self::PolicyRegistry { source } => {
                write!(formatter, "transition-domain policy registry error: {source}")
            }
            Self::PolicyIdentity(source) => {
                write!(formatter, "transition-domain policy identity error: {source}")
            }
            Self::ApplicabilityIdentity(source) => write!(
                formatter,
                "transition-domain applicability identity error: {source}"
            ),
            Self::ApplicabilityPolicyAuthorityMismatch => write!(
                formatter,
                "applicability policy is not bound to the exact information-policy authority used by the domain registry"
            ),
            Self::UnknownDomain { domain } => {
                write!(formatter, "unknown transition-domain descriptor {domain:?}")
            }
            Self::DomainAuthorityNotQualified { domain, status } => write!(
                formatter,
                "transition-domain authority {domain:?} is not qualified: {status:?}"
            ),
            Self::SourceRepresentationMismatch { expected, actual } => write!(
                formatter,
                "transition-domain source representation mismatch: expected {expected:?}, got {actual:?}"
            ),
            Self::DomainNotRequiredByApplicability { domain, source } => write!(
                formatter,
                "exact applicability corpus does not require domain {domain:?} from source representation {source:?}"
            ),
            Self::PopulationIdentity(source) => {
                write!(formatter, "transition-domain population identity error: {source}")
            }
            Self::PopulationManifestMismatch => write!(
                formatter,
                "transition-domain source population content changed from the evaluated manifest"
            ),
            Self::DomainNotSatisfied { domain } => write!(
                formatter,
                "canonical source state does not satisfy transition domain {domain:?}"
            ),
            Self::DomainAuthorityChanged => {
                write!(formatter, "transition-domain authority corpus changed after evaluation")
            }
            Self::ApplicabilityAuthorityChanged => write!(
                formatter,
                "transition applicability authority changed after domain evaluation"
            ),
            Self::DomainQualificationChanged { domain } => write!(
                formatter,
                "transition-domain qualification {domain:?} changed after evaluation"
            ),
            Self::EvaluationSubjectChanged => write!(
                formatter,
                "transition-domain evaluation subject scope/snapshot/revision/state changed"
            ),
        }
    }
}

impl Error for TransitionDomainAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyRegistry { source } => Some(source),
            Self::PolicyIdentity(source) => Some(source),
            Self::ApplicabilityIdentity(source) => Some(source),
            Self::PopulationIdentity(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::applicability_policy_manifest::ManifestBoundTransitionApplicabilityPolicy;
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
        RepresentationCapabilities,
    };
    use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionKey,
        InformationTransitionRegistry, InformationTransitionRegistryBuilder,
        InformationTransitionRegistryKey,
    };
    use crate::population::{
        CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand,
    };
    use crate::transition_applicability::{
        TransitionApplicabilityPolicy, TransitionApplicabilityPolicyBuilder,
        TransitionApplicabilityPolicyKey,
    };
    use crate::transition_policy_manifest::ManifestBoundInformationTransitionRegistry;

    const POLICY_KEY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(1, 1);
    const SOURCE: RepresentationKey = RepresentationKey::new(10, 1);
    const DESTINATION: RepresentationKey = RepresentationKey::new(11, 1);
    const TRANSITION_REGISTRY: InformationTransitionRegistryKey =
        InformationTransitionRegistryKey::new(20, 1);
    const TRANSITION: InformationTransitionKey = InformationTransitionKey::new(21, 1);
    const APPLICABILITY_POLICY: TransitionApplicabilityPolicyKey =
        TransitionApplicabilityPolicyKey::new(30, 1);
    const DOMAIN: TransitionDomainKey = TransitionDomainKey::new(31, 1);
    const DOMAIN_REGISTRY: TransitionDomainAuthorityRegistryKey =
        TransitionDomainAuthorityRegistryKey::new(40, 1);
    const SCOPE: TransitionDomainAuthorityScope = TransitionDomainAuthorityScope::new(50, 1);
    const SNAPSHOT: TransitionDomainSnapshotId = TransitionDomainSnapshotId(60);
    const REVISION: TransitionDomainSourceRevision = TransitionDomainSourceRevision(7);

    fn capabilities(key: RepresentationKey) -> RepresentationCapabilities {
        RepresentationCapabilities::new(
            key,
            EcologicalAuthorityLevel::Coarse,
            [
                (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
                (
                    EcologicalInformation::AgeDistribution,
                    CapabilityEvidence::Exact,
                ),
                (
                    EcologicalInformation::ConditionDistribution,
                    CapabilityEvidence::Exact,
                ),
            ],
        )
    }

    fn policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY_KEY);
        builder.register_representation(capabilities(SOURCE)).unwrap();
        builder
            .register_representation(capabilities(DESTINATION))
            .unwrap();
        builder.seal()
    }

    fn transitions(policy: &InformationPolicyRegistry) -> InformationTransitionRegistry {
        let definition =
            InformationTransitionDefinition::new(TRANSITION, SOURCE, DESTINATION, [], []).unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITION_REGISTRY);
        builder.register_transition(definition).unwrap();
        builder.seal(policy).unwrap()
    }

    fn applicability(
        transitions: &InformationTransitionRegistry,
        mode: TransitionApplicability,
    ) -> TransitionApplicabilityPolicy {
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY_POLICY);
        builder.register(TRANSITION, mode).unwrap();
        builder.seal(transitions).unwrap()
    }

    fn qualification(
        fingerprint: &[u8],
        status: TransitionDomainAuthorityStatus,
    ) -> TransitionDomainQualification {
        TransitionDomainQualification::new(
            DOMAIN,
            SOURCE,
            TransitionDomainPredicate::UniqueAgeConditionFromSingletonMarginals,
            TransitionDomainPredicateProfileVersion(1),
            TransitionDomainEvaluatorFingerprint::new(fingerprint.to_vec()).unwrap(),
            EvidenceLineageToken(900),
            None,
            status,
        )
    }

    fn domain_registry(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        record: TransitionDomainQualification,
    ) -> TransitionDomainAuthorityRegistry {
        let mut builder = TransitionDomainAuthorityRegistryBuilder::new(DOMAIN_REGISTRY);
        builder.register(record).unwrap();
        builder.seal(policy).unwrap()
    }

    fn distribution<K: Ord>(pairs: impl IntoIterator<Item = (K, u64)>) -> CountDistribution<K> {
        CountDistribution::new(pairs.into_iter().collect::<BTreeMap<_, _>>()).unwrap()
    }

    fn singleton_population() -> PopulationState {
        PopulationState::new(
            10,
            2_500_000,
            distribution([(PopulationAgeBand::Mature, 10)]),
            distribution([(PopulationConditionBand::Stable, 10)]),
            distribution([(PopulationCell::new(0, 0, 0), 10)]),
        )
        .unwrap()
    }

    fn multibin_population() -> PopulationState {
        PopulationState::new(
            10,
            2_500_000,
            distribution([
                (PopulationAgeBand::Juvenile, 5),
                (PopulationAgeBand::Mature, 5),
            ]),
            distribution([
                (PopulationConditionBand::Stable, 5),
                (PopulationConditionBand::Stressed, 5),
            ]),
            distribution([(PopulationCell::new(0, 0, 0), 10)]),
        )
        .unwrap()
    }

    fn subject(population: &PopulationState) -> TransitionDomainEvaluationSubject {
        TransitionDomainEvaluationSubject::from_population(
            SCOPE, SOURCE, SNAPSHOT, REVISION, population,
        )
        .unwrap()
    }

    fn exact_context<'a>(
        policy: &'a InformationPolicyRegistry,
        transitions: &'a InformationTransitionRegistry,
        applicability: &'a TransitionApplicabilityPolicy,
    ) -> (
        ManifestBoundInformationPolicyRegistry<'a>,
        ManifestBoundInformationTransitionRegistry<'a>,
        ManifestBoundTransitionApplicabilityPolicy<'a>,
    ) {
        let policy_exact = ManifestBoundInformationPolicyRegistry::new(policy);
        let transitions_exact =
            ManifestBoundInformationTransitionRegistry::new(transitions, &policy_exact).unwrap();
        let applicability_exact =
            ManifestBoundTransitionApplicabilityPolicy::new(applicability, &transitions_exact)
                .unwrap();
        (policy_exact, transitions_exact, applicability_exact)
    }

    #[test]
    fn qualified_singleton_theorem_mints_revision_scoped_satisfied_proof() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact) =
            exact_context(&policy, &transitions, &applicability);
        let registry = domain_registry(
            &policy_exact,
            qualification(b"domain-evaluator-a", TransitionDomainAuthorityStatus::Qualified),
        );
        let population = singleton_population();
        let subject = subject(&population);

        let proof = registry
            .evaluate_population_domain(
                &policy_exact,
                &applicability_exact,
                DOMAIN,
                &subject,
                &population,
            )
            .unwrap();

        assert_eq!(proof.domain(), DOMAIN);
        assert_eq!(proof.subject(), &subject);
        assert_eq!(
            proof.qualification().predicate(),
            TransitionDomainPredicate::UniqueAgeConditionFromSingletonMarginals
        );
        proof
            .validate_current(
                &registry,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            )
            .unwrap();
    }

    #[test]
    fn generic_multibin_population_does_not_mint_proof() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact) =
            exact_context(&policy, &transitions, &applicability);
        let registry = domain_registry(
            &policy_exact,
            qualification(b"domain-evaluator-a", TransitionDomainAuthorityStatus::Qualified),
        );
        let population = multibin_population();
        let subject = subject(&population);

        assert_eq!(
            registry.evaluate_population_domain(
                &policy_exact,
                &applicability_exact,
                DOMAIN,
                &subject,
                &population,
            ),
            Err(TransitionDomainAuthorityError::DomainNotSatisfied { domain: DOMAIN })
        );
    }

    #[test]
    fn same_revision_but_different_exact_population_manifest_stales_proof() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact) =
            exact_context(&policy, &transitions, &applicability);
        let registry = domain_registry(
            &policy_exact,
            qualification(b"domain-evaluator-a", TransitionDomainAuthorityStatus::Qualified),
        );
        let original = singleton_population();
        let original_subject = subject(&original);
        let proof = registry
            .evaluate_population_domain(
                &policy_exact,
                &applicability_exact,
                DOMAIN,
                &original_subject,
                &original,
            )
            .unwrap();

        let changed = PopulationState::new(
            10,
            2_500_001,
            distribution([(PopulationAgeBand::Mature, 10)]),
            distribution([(PopulationConditionBand::Stable, 10)]),
            distribution([(PopulationCell::new(0, 0, 0), 10)]),
        )
        .unwrap();
        let changed_subject = subject(&changed);

        assert_eq!(
            proof.validate_current(
                &registry,
                &policy_exact,
                &applicability_exact,
                &changed_subject,
                &changed,
            ),
            Err(TransitionDomainAuthorityError::EvaluationSubjectChanged)
        );
    }

    #[test]
    fn revision_change_stales_proof_even_when_population_bytes_match() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact) =
            exact_context(&policy, &transitions, &applicability);
        let registry = domain_registry(
            &policy_exact,
            qualification(b"domain-evaluator-a", TransitionDomainAuthorityStatus::Qualified),
        );
        let population = singleton_population();
        let original_subject = subject(&population);
        let proof = registry
            .evaluate_population_domain(
                &policy_exact,
                &applicability_exact,
                DOMAIN,
                &original_subject,
                &population,
            )
            .unwrap();
        let newer_subject = TransitionDomainEvaluationSubject::from_population(
            SCOPE,
            SOURCE,
            SNAPSHOT,
            TransitionDomainSourceRevision(REVISION.0 + 1),
            &population,
        )
        .unwrap();

        assert_eq!(
            proof.validate_current(
                &registry,
                &policy_exact,
                &applicability_exact,
                &newer_subject,
                &population,
            ),
            Err(TransitionDomainAuthorityError::EvaluationSubjectChanged)
        );
    }

    #[test]
    fn same_domain_key_with_changed_evaluator_fingerprint_stales_old_proof() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact) =
            exact_context(&policy, &transitions, &applicability);
        let registry_a = domain_registry(
            &policy_exact,
            qualification(b"domain-evaluator-a", TransitionDomainAuthorityStatus::Qualified),
        );
        let registry_b = domain_registry(
            &policy_exact,
            qualification(b"domain-evaluator-b", TransitionDomainAuthorityStatus::Qualified),
        );
        let population = singleton_population();
        let subject = subject(&population);
        let proof = registry_a
            .evaluate_population_domain(
                &policy_exact,
                &applicability_exact,
                DOMAIN,
                &subject,
                &population,
            )
            .unwrap();

        assert_eq!(
            proof.validate_current(
                &registry_b,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            ),
            Err(TransitionDomainAuthorityError::DomainAuthorityChanged)
        );
    }

    #[test]
    fn revoked_or_superseded_domain_authority_cannot_mint_proof() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact) =
            exact_context(&policy, &transitions, &applicability);
        let population = singleton_population();
        let subject = subject(&population);

        for status in [
            TransitionDomainAuthorityStatus::Revoked,
            TransitionDomainAuthorityStatus::Superseded,
        ] {
            let registry = domain_registry(
                &policy_exact,
                qualification(b"domain-evaluator-a", status),
            );
            assert_eq!(
                registry.evaluate_population_domain(
                    &policy_exact,
                    &applicability_exact,
                    DOMAIN,
                    &subject,
                    &population,
                ),
                Err(TransitionDomainAuthorityError::DomainAuthorityNotQualified {
                    domain: DOMAIN,
                    status,
                })
            );
        }
    }

    #[test]
    fn universal_applicability_does_not_authorize_unrequested_domain_proof() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(&transitions, TransitionApplicability::Universal);
        let (policy_exact, _transitions_exact, applicability_exact) =
            exact_context(&policy, &transitions, &applicability);
        let registry = domain_registry(
            &policy_exact,
            qualification(b"domain-evaluator-a", TransitionDomainAuthorityStatus::Qualified),
        );
        let population = singleton_population();
        let subject = subject(&population);

        assert_eq!(
            registry.evaluate_population_domain(
                &policy_exact,
                &applicability_exact,
                DOMAIN,
                &subject,
                &population,
            ),
            Err(TransitionDomainAuthorityError::DomainNotRequiredByApplicability {
                domain: DOMAIN,
                source: SOURCE,
            })
        );
    }

    #[test]
    fn changed_exact_applicability_corpus_stales_old_proof() {
        let policy = policy();
        let transitions = transitions(&policy);
        let domain_applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let universal_applicability = applicability(&transitions, TransitionApplicability::Universal);
        let (policy_exact, _domain_transitions, domain_exact) =
            exact_context(&policy, &transitions, &domain_applicability);
        let (_policy_exact_again, _universal_transitions, universal_exact) =
            exact_context(&policy, &transitions, &universal_applicability);
        let registry = domain_registry(
            &policy_exact,
            qualification(b"domain-evaluator-a", TransitionDomainAuthorityStatus::Qualified),
        );
        let population = singleton_population();
        let subject = subject(&population);
        let proof = registry
            .evaluate_population_domain(
                &policy_exact,
                &domain_exact,
                DOMAIN,
                &subject,
                &population,
            )
            .unwrap();

        assert_eq!(
            proof.validate_current(
                &registry,
                &policy_exact,
                &universal_exact,
                &subject,
                &population,
            ),
            Err(TransitionDomainAuthorityError::ApplicabilityAuthorityChanged)
        );
    }
}
