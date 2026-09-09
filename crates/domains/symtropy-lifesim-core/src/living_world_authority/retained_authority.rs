// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Resolution of R0 retained-exact candidate obligations.
//!
//! A `RetainedAuthorityKey` is only a semantic descriptor. Canonical R0 evidence
//! comes from one sealed retained-store registry whose exact policy authority,
//! retained claim set, scope/snapshot, store revision, content manifest and
//! status remain current. This module never reconstructs or samples information.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::applicability_policy_manifest::{
    ManifestBoundTransitionApplicabilityPolicy, TransitionApplicabilityAuthorityStamp,
};
use crate::candidate_evidence_obligations::{
    CandidateCertificationRequestError, CandidateCertificationRequestSet,
    CandidateEvidenceObligation, CandidatePathIdentity,
};
use crate::information::{
    CapabilityEvidence, EcologicalInformation, RepresentationCapabilities, RepresentationKey,
};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::InformationRegistryError;
use crate::information_transition::{
    InformationTransitionKey, PromotionProvenance, RetainedAuthorityKey, TransitionCapabilityClaim,
};
use crate::population::PopulationState;

use super::transition_domain::{
    TransitionDomainAuthorityError, TransitionDomainAuthorityScope, TransitionDomainEvaluationSubject,
    TransitionDomainSnapshotId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RetainedAuthorityRegistryKey {
    id: u128,
    version: u32,
}

impl RetainedAuthorityRegistryKey {
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
pub struct RetainedStoreRevision(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RetainedContentManifest(Vec<u8>);

impl RetainedContentManifest {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, RetainedAuthorityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(RetainedAuthorityError::EmptyContentManifest);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainedAuthorityStatus {
    Retained,
    Revoked,
    Superseded,
}

/// Complete canonical record for one retained exact sidecar/store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedAuthorityRecord {
    authority: RetainedAuthorityKey,
    store_representation: RepresentationKey,
    scope: TransitionDomainAuthorityScope,
    snapshot: TransitionDomainSnapshotId,
    revision: RetainedStoreRevision,
    claims: BTreeSet<TransitionCapabilityClaim>,
    content_manifest: RetainedContentManifest,
    status: RetainedAuthorityStatus,
}

impl RetainedAuthorityRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        authority: RetainedAuthorityKey,
        store_representation: RepresentationKey,
        scope: TransitionDomainAuthorityScope,
        snapshot: TransitionDomainSnapshotId,
        revision: RetainedStoreRevision,
        claims: impl IntoIterator<Item = TransitionCapabilityClaim>,
        content_manifest: RetainedContentManifest,
        status: RetainedAuthorityStatus,
    ) -> Result<Self, RetainedAuthorityError> {
        let claims = claims.into_iter().collect::<BTreeSet<_>>();
        if claims.is_empty() {
            return Err(RetainedAuthorityError::EmptyClaimSet { authority });
        }
        if let Some(claim) = claims
            .iter()
            .find(|claim| claim.evidence() != CapabilityEvidence::Exact)
            .copied()
        {
            return Err(RetainedAuthorityError::NonExactClaim { authority, claim });
        }
        Ok(Self {
            authority,
            store_representation,
            scope,
            snapshot,
            revision,
            claims,
            content_manifest,
            status,
        })
    }

    pub const fn authority(&self) -> RetainedAuthorityKey {
        self.authority
    }

    pub const fn store_representation(&self) -> RepresentationKey {
        self.store_representation
    }

    pub const fn scope(&self) -> TransitionDomainAuthorityScope {
        self.scope
    }

    pub const fn snapshot(&self) -> TransitionDomainSnapshotId {
        self.snapshot
    }

    pub const fn revision(&self) -> RetainedStoreRevision {
        self.revision
    }

    pub fn claims(&self) -> &BTreeSet<TransitionCapabilityClaim> {
        &self.claims
    }

    pub const fn content_manifest(&self) -> &RetainedContentManifest {
        &self.content_manifest
    }

    pub const fn status(&self) -> RetainedAuthorityStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedAuthorityRegistryBuilder {
    key: RetainedAuthorityRegistryKey,
    records: BTreeMap<RetainedAuthorityKey, RetainedAuthorityRecord>,
}

impl RetainedAuthorityRegistryBuilder {
    pub const fn new(key: RetainedAuthorityRegistryKey) -> Self {
        Self {
            key,
            records: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, record: RetainedAuthorityRecord) -> Result<(), RetainedAuthorityError> {
        use std::collections::btree_map::Entry;
        match self.records.entry(record.authority()) {
            Entry::Vacant(entry) => {
                entry.insert(record);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &record => Ok(()),
            Entry::Occupied(entry) => Err(RetainedAuthorityError::ConflictingRegistration {
                authority: *entry.key(),
            }),
        }
    }

    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<RetainedAuthorityRegistry, RetainedAuthorityError> {
        for record in self.records.values() {
            let registered = policy
                .registry()
                .resolve_representation(record.store_representation())
                .map_err(|source| RetainedAuthorityError::PolicyRegistry { source })?;
            for claim in record.claims() {
                if !covers_exact(registered.capabilities(), *claim) {
                    return Err(RetainedAuthorityError::StoreMissingClaim {
                        authority: record.authority(),
                        claim: *claim,
                    });
                }
            }
        }

        let authority = RetainedAuthorityStamp {
            key: self.key,
            policy_authority: policy.authority_stamp().clone(),
            records: self.records.clone(),
        };
        Ok(RetainedAuthorityRegistry {
            records: self.records,
            authority,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedAuthorityStamp {
    key: RetainedAuthorityRegistryKey,
    policy_authority: InformationPolicyAuthorityStamp,
    records: BTreeMap<RetainedAuthorityKey, RetainedAuthorityRecord>,
}

impl RetainedAuthorityStamp {
    pub const fn key(&self) -> RetainedAuthorityRegistryKey {
        self.key
    }

    pub const fn policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.policy_authority
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedAuthorityRegistry {
    records: BTreeMap<RetainedAuthorityKey, RetainedAuthorityRecord>,
    authority: RetainedAuthorityStamp,
}

impl RetainedAuthorityRegistry {
    pub const fn authority_stamp(&self) -> &RetainedAuthorityStamp {
        &self.authority
    }

    fn validate_policy(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), RetainedAuthorityError> {
        self.authority
            .policy_authority
            .validate_registry(policy.registry())
            .map_err(RetainedAuthorityError::PolicyIdentity)
    }

    fn resolve(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        authority: RetainedAuthorityKey,
        required_claims: BTreeSet<TransitionCapabilityClaim>,
        subject: &TransitionDomainEvaluationSubject,
    ) -> Result<ResolvedRetainedAuthority, RetainedAuthorityError> {
        self.validate_policy(policy)?;
        let record = self
            .records
            .get(&authority)
            .ok_or(RetainedAuthorityError::UnknownAuthority { authority })?;
        if record.status() != RetainedAuthorityStatus::Retained {
            return Err(RetainedAuthorityError::AuthorityNotCurrent {
                authority,
                status: record.status(),
            });
        }
        if record.scope() != subject.scope() || record.snapshot() != subject.snapshot() {
            return Err(RetainedAuthorityError::ContextMismatch { authority });
        }
        if !required_claims.is_subset(record.claims()) {
            return Err(RetainedAuthorityError::RequiredClaimsMissing {
                authority,
                required: required_claims,
                available: record.claims().clone(),
            });
        }
        Ok(ResolvedRetainedAuthority {
            registry_authority: self.authority.clone(),
            record: record.clone(),
            required_claims,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRetainedAuthority {
    registry_authority: RetainedAuthorityStamp,
    record: RetainedAuthorityRecord,
    required_claims: BTreeSet<TransitionCapabilityClaim>,
}

impl ResolvedRetainedAuthority {
    pub const fn authority(&self) -> RetainedAuthorityKey {
        self.record.authority()
    }

    pub const fn record(&self) -> &RetainedAuthorityRecord {
        &self.record
    }

    pub fn required_claims(&self) -> &BTreeSet<TransitionCapabilityClaim> {
        &self.required_claims
    }

    pub const fn registry_authority(&self) -> &RetainedAuthorityStamp {
        &self.registry_authority
    }
}

/// Candidate after resolving only R0 obligations. This is not `Admitted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedResolvedCandidateRequest {
    candidate_index: usize,
    applicability_authority: TransitionApplicabilityAuthorityStamp,
    retained_authority: RetainedAuthorityStamp,
    identity: CandidatePathIdentity,
    subject: TransitionDomainEvaluationSubject,
    resolved: BTreeMap<RetainedAuthorityKey, ResolvedRetainedAuthority>,
    remaining_obligations: BTreeSet<CandidateEvidenceObligation>,
    discarded_information: BTreeSet<EcologicalInformation>,
}

impl RetainedResolvedCandidateRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        requests: &CandidateCertificationRequestSet,
        candidate_index: usize,
        retained: &RetainedAuthorityRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        subject: &TransitionDomainEvaluationSubject,
        population: &PopulationState,
    ) -> Result<Self, RetainedAuthorityError> {
        requests
            .validate_current(applicability)
            .map_err(RetainedAuthorityError::CandidateRequest)?;
        retained.validate_policy(policy)?;
        if requests.information_policy_authority() != retained.authority_stamp().policy_authority() {
            return Err(RetainedAuthorityError::PolicyAuthorityMismatch);
        }

        let request = requests
            .requests()
            .get(candidate_index)
            .ok_or(RetainedAuthorityError::CandidateIndexOutOfRange {
                index: candidate_index,
                len: requests.requests().len(),
            })?;
        if request.identity().source() != subject.source_representation() {
            return Err(RetainedAuthorityError::SourceRepresentationMismatch);
        }
        subject
            .validate_population(population)
            .map_err(RetainedAuthorityError::SourceState)?;

        let mut resolved = BTreeMap::new();
        let mut remaining_obligations = BTreeSet::new();
        for obligation in request.obligations().iter().copied() {
            match obligation {
                CandidateEvidenceObligation::RetainedAuthority(authority) => {
                    let claims = collect_r0_claims(applicability, request.identity(), authority)?;
                    resolved.insert(authority, retained.resolve(policy, authority, claims, subject)?);
                }
                other => {
                    remaining_obligations.insert(other);
                }
            }
        }

        Ok(Self {
            candidate_index,
            applicability_authority: requests.applicability_authority().clone(),
            retained_authority: retained.authority_stamp().clone(),
            identity: request.identity().clone(),
            subject: subject.clone(),
            resolved,
            remaining_obligations,
            discarded_information: request.discarded_information().clone(),
        })
    }

    pub fn resolved(&self) -> &BTreeMap<RetainedAuthorityKey, ResolvedRetainedAuthority> {
        &self.resolved
    }

    pub fn remaining_obligations(&self) -> &BTreeSet<CandidateEvidenceObligation> {
        &self.remaining_obligations
    }

    pub fn discarded_information(&self) -> &BTreeSet<EcologicalInformation> {
        &self.discarded_information
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        requests: &CandidateCertificationRequestSet,
        retained: &RetainedAuthorityRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        subject: &TransitionDomainEvaluationSubject,
        population: &PopulationState,
    ) -> Result<(), RetainedAuthorityError> {
        if requests.applicability_authority() != &self.applicability_authority {
            return Err(RetainedAuthorityError::ApplicabilityAuthorityChanged);
        }
        if retained.authority_stamp() != &self.retained_authority {
            return Err(RetainedAuthorityError::RetainedCorpusChanged);
        }
        if subject != &self.subject {
            return Err(RetainedAuthorityError::SubjectChanged);
        }
        let current = Self::resolve(
            requests,
            self.candidate_index,
            retained,
            policy,
            applicability,
            subject,
            population,
        )?;
        if current != *self {
            return Err(RetainedAuthorityError::ResolutionChanged);
        }
        Ok(())
    }
}

fn collect_r0_claims(
    applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    identity: &CandidatePathIdentity,
    authority: RetainedAuthorityKey,
) -> Result<BTreeSet<TransitionCapabilityClaim>, RetainedAuthorityError> {
    let mut claims = BTreeSet::new();
    for transition_key in identity.transitions() {
        let transition = applicability
            .transitions()
            .registry()
            .transitions()
            .find_map(|(key, transition)| (key == transition_key).then_some(transition))
            .ok_or(RetainedAuthorityError::TransitionMissing {
                transition: *transition_key,
            })?;
        for (claim, provenance) in transition.introductions() {
            if matches!(
                provenance,
                PromotionProvenance::RetainedExact { authority: found } if *found == authority
            ) {
                claims.insert(*claim);
            }
        }
    }
    if claims.is_empty() {
        return Err(RetainedAuthorityError::NoClaimsForAuthority { authority });
    }
    Ok(claims)
}

fn covers_exact(
    capabilities: &RepresentationCapabilities,
    claim: TransitionCapabilityClaim,
) -> bool {
    capabilities.claims().iter().any(|(available, evidence)| {
        available.covers(claim.information()) && evidence.contains(&CapabilityEvidence::Exact)
    })
}

#[derive(Debug)]
pub enum RetainedAuthorityError {
    EmptyContentManifest,
    EmptyClaimSet { authority: RetainedAuthorityKey },
    NonExactClaim {
        authority: RetainedAuthorityKey,
        claim: TransitionCapabilityClaim,
    },
    ConflictingRegistration { authority: RetainedAuthorityKey },
    PolicyRegistry { source: InformationRegistryError },
    PolicyIdentity(InformationPolicyIdentityError),
    StoreMissingClaim {
        authority: RetainedAuthorityKey,
        claim: TransitionCapabilityClaim,
    },
    UnknownAuthority { authority: RetainedAuthorityKey },
    AuthorityNotCurrent {
        authority: RetainedAuthorityKey,
        status: RetainedAuthorityStatus,
    },
    ContextMismatch { authority: RetainedAuthorityKey },
    RequiredClaimsMissing {
        authority: RetainedAuthorityKey,
        required: BTreeSet<TransitionCapabilityClaim>,
        available: BTreeSet<TransitionCapabilityClaim>,
    },
    CandidateRequest(CandidateCertificationRequestError),
    PolicyAuthorityMismatch,
    CandidateIndexOutOfRange { index: usize, len: usize },
    SourceRepresentationMismatch,
    SourceState(TransitionDomainAuthorityError),
    TransitionMissing { transition: InformationTransitionKey },
    NoClaimsForAuthority { authority: RetainedAuthorityKey },
    ApplicabilityAuthorityChanged,
    RetainedCorpusChanged,
    SubjectChanged,
    ResolutionChanged,
}

impl fmt::Display for RetainedAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for RetainedAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyRegistry { source } => Some(source),
            Self::PolicyIdentity(source) => Some(source),
            Self::CandidateRequest(source) => Some(source),
            Self::SourceState(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::candidate_applicability::ApplicableTransitionCandidateSet;
    use crate::information::{EcologicalAuthorityLevel, EcologicalInformation};
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionRegistry,
        InformationTransitionRegistryBuilder, InformationTransitionRegistryKey, LosslessTransformKey,
    };
    use crate::living_world_authority::transition_domain::TransitionDomainSourceRevision;
    use crate::population::{
        CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand,
    };
    use crate::transition_applicability::{
        TransitionApplicability, TransitionApplicabilityPolicy, TransitionApplicabilityPolicyBuilder,
        TransitionApplicabilityPolicyKey,
    };
    use crate::transition_candidates::TransitionCandidateSearchLimits;
    use crate::transition_policy_manifest::ManifestBoundInformationTransitionRegistry;

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(2_000, 1);
    const SOURCE: RepresentationKey = RepresentationKey::new(2_010, 1);
    const TARGET: RepresentationKey = RepresentationKey::new(2_011, 1);
    const STORE: RepresentationKey = RepresentationKey::new(2_012, 1);
    const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(2_020, 1);
    const EDGE: InformationTransitionKey = InformationTransitionKey::new(2_021, 1);
    const APP: TransitionApplicabilityPolicyKey = TransitionApplicabilityPolicyKey::new(2_030, 1);
    const RETAINED: RetainedAuthorityKey = RetainedAuthorityKey(2_040, 1);
    const TRANSFORM: LosslessTransformKey = LosslessTransformKey(2_041, 1);
    const REGISTRY: RetainedAuthorityRegistryKey = RetainedAuthorityRegistryKey::new(2_050, 1);
    const SCOPE: TransitionDomainAuthorityScope = TransitionDomainAuthorityScope::new(2_060, 1);
    const SNAPSHOT: TransitionDomainSnapshotId = TransitionDomainSnapshotId(2_070);

    fn exact_claim(information: EcologicalInformation) -> TransitionCapabilityClaim {
        TransitionCapabilityClaim::new(information, CapabilityEvidence::Exact)
    }

    fn capabilities(
        key: RepresentationKey,
        information: impl IntoIterator<Item = EcologicalInformation>,
    ) -> RepresentationCapabilities {
        RepresentationCapabilities::new(
            key,
            EcologicalAuthorityLevel::Coarse,
            information
                .into_iter()
                .map(|info| (info, CapabilityEvidence::Exact)),
        )
    }

    fn policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_representation(capabilities(
                SOURCE,
                [EcologicalInformation::Headcount, EcologicalInformation::AgeDistribution],
            ))
            .unwrap();
        builder
            .register_representation(capabilities(
                TARGET,
                [
                    EcologicalInformation::Headcount,
                    EcologicalInformation::SpatialStructure,
                    EcologicalInformation::DiseaseState,
                ],
            ))
            .unwrap();
        builder
            .register_representation(capabilities(
                STORE,
                [
                    EcologicalInformation::SpatialStructure,
                    EcologicalInformation::AgeDistribution,
                ],
            ))
            .unwrap();
        builder.seal()
    }

    fn transitions(
        policy: &InformationPolicyRegistry,
        disease_from_r0: bool,
    ) -> InformationTransitionRegistry {
        let disease = if disease_from_r0 {
            PromotionProvenance::RetainedExact { authority: RETAINED }
        } else {
            PromotionProvenance::LosslessDerivation { transform: TRANSFORM }
        };
        let edge = InformationTransitionDefinition::new(
            EDGE,
            SOURCE,
            TARGET,
            [
                (
                    exact_claim(EcologicalInformation::SpatialStructure),
                    PromotionProvenance::RetainedExact { authority: RETAINED },
                ),
                (exact_claim(EcologicalInformation::DiseaseState), disease),
            ],
            [EcologicalInformation::AgeDistribution],
        )
        .unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
        builder.register_transition(edge).unwrap();
        builder.seal(policy).unwrap()
    }

    fn applicability(transitions: &InformationTransitionRegistry) -> TransitionApplicabilityPolicy {
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APP);
        builder.register(EDGE, TransitionApplicability::Universal).unwrap();
        builder.seal(transitions).unwrap()
    }

    fn population() -> PopulationState {
        fn dist<K: Ord>(pairs: impl IntoIterator<Item = (K, u64)>) -> CountDistribution<K> {
            CountDistribution::new(pairs.into_iter().collect::<BTreeMap<_, _>>()).unwrap()
        }
        PopulationState::new(
            10,
            1_000_000,
            dist([(PopulationAgeBand::Mature, 10)]),
            dist([(PopulationConditionBand::Stable, 10)]),
            dist([(PopulationCell::new(0, 0, 0), 10)]),
        )
        .unwrap()
    }

    fn subject(population: &PopulationState) -> TransitionDomainEvaluationSubject {
        TransitionDomainEvaluationSubject::from_population(
            SCOPE,
            SOURCE,
            SNAPSHOT,
            TransitionDomainSourceRevision(1),
            population,
        )
        .unwrap()
    }

    fn exact_context<'a>(
        policy: &'a InformationPolicyRegistry,
        transitions: &'a InformationTransitionRegistry,
        applicability: &'a TransitionApplicabilityPolicy,
    ) -> (
        ManifestBoundInformationPolicyRegistry<'a>,
        ManifestBoundTransitionApplicabilityPolicy<'a>,
        CandidateCertificationRequestSet,
    ) {
        let policy_exact = ManifestBoundInformationPolicyRegistry::new(policy);
        let transitions_exact =
            ManifestBoundInformationTransitionRegistry::new(transitions, &policy_exact).unwrap();
        let applicability_exact =
            ManifestBoundTransitionApplicabilityPolicy::new(applicability, &transitions_exact)
                .unwrap();
        let candidates = transitions_exact
            .registry()
            .discover_candidates(
                transitions_exact.policy().registry(),
                SOURCE,
                TARGET,
                TransitionCandidateSearchLimits::new(4, 8).unwrap(),
            )
            .unwrap();
        let annotated: ApplicableTransitionCandidateSet = applicability_exact
            .policy()
            .annotate_candidate_set(&candidates)
            .unwrap();
        let requests =
            CandidateCertificationRequestSet::prepare(&applicability_exact, &annotated).unwrap();
        (policy_exact, applicability_exact, requests)
    }

    fn retained_registry(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        claims: impl IntoIterator<Item = TransitionCapabilityClaim>,
        content: &[u8],
        status: RetainedAuthorityStatus,
    ) -> RetainedAuthorityRegistry {
        let record = RetainedAuthorityRecord::new(
            RETAINED,
            STORE,
            SCOPE,
            SNAPSHOT,
            RetainedStoreRevision(9),
            claims,
            RetainedContentManifest::new(content.to_vec()).unwrap(),
            status,
        )
        .unwrap();
        let mut builder = RetainedAuthorityRegistryBuilder::new(REGISTRY);
        builder.register(record).unwrap();
        builder.seal(policy).unwrap()
    }

    #[test]
    fn r0_resolves_exact_claim_and_preserves_r1_and_information_loss() {
        let policy = policy();
        let transitions = transitions(&policy, false);
        let applicability = applicability(&transitions);
        let (policy_exact, applicability_exact, requests) =
            exact_context(&policy, &transitions, &applicability);
        let retained = retained_registry(
            &policy_exact,
            [exact_claim(EcologicalInformation::SpatialStructure)],
            b"store-a",
            RetainedAuthorityStatus::Retained,
        );
        let population = population();
        let subject = subject(&population);

        let resolved = RetainedResolvedCandidateRequest::resolve(
            &requests,
            0,
            &retained,
            &policy_exact,
            &applicability_exact,
            &subject,
            &population,
        )
        .unwrap();

        assert_eq!(resolved.resolved().len(), 1);
        assert!(resolved
            .remaining_obligations()
            .contains(&CandidateEvidenceObligation::LosslessTransform(TRANSFORM)));
        assert!(resolved
            .discarded_information()
            .contains(&EcologicalInformation::AgeDistribution));
    }

    #[test]
    fn same_r0_key_cannot_cover_a_missing_second_claim() {
        let policy = policy();
        let transitions = transitions(&policy, true);
        let applicability = applicability(&transitions);
        let (policy_exact, applicability_exact, requests) =
            exact_context(&policy, &transitions, &applicability);
        let retained = retained_registry(
            &policy_exact,
            [exact_claim(EcologicalInformation::SpatialStructure)],
            b"store-a",
            RetainedAuthorityStatus::Retained,
        );
        let population = population();
        let subject = subject(&population);

        assert!(matches!(
            RetainedResolvedCandidateRequest::resolve(
                &requests,
                0,
                &retained,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            ),
            Err(RetainedAuthorityError::RequiredClaimsMissing { .. })
        ));
    }

    #[test]
    fn non_exact_claim_cannot_enter_retained_authority() {
        let claim = TransitionCapabilityClaim::new(
            EcologicalInformation::SpatialStructure,
            CapabilityEvidence::MeasurementOnly,
        );
        assert!(matches!(
            RetainedAuthorityRecord::new(
                RETAINED,
                STORE,
                SCOPE,
                SNAPSHOT,
                RetainedStoreRevision(1),
                [claim],
                RetainedContentManifest::new(b"store".to_vec()).unwrap(),
                RetainedAuthorityStatus::Retained,
            ),
            Err(RetainedAuthorityError::NonExactClaim { .. })
        ));
    }

    #[test]
    fn same_key_revision_with_changed_content_stales_old_resolution() {
        let policy = policy();
        let transitions = transitions(&policy, false);
        let applicability = applicability(&transitions);
        let (policy_exact, applicability_exact, requests) =
            exact_context(&policy, &transitions, &applicability);
        let retained_a = retained_registry(
            &policy_exact,
            [exact_claim(EcologicalInformation::SpatialStructure)],
            b"store-a",
            RetainedAuthorityStatus::Retained,
        );
        let retained_b = retained_registry(
            &policy_exact,
            [exact_claim(EcologicalInformation::SpatialStructure)],
            b"store-b",
            RetainedAuthorityStatus::Retained,
        );
        let population = population();
        let subject = subject(&population);
        let resolved = RetainedResolvedCandidateRequest::resolve(
            &requests,
            0,
            &retained_a,
            &policy_exact,
            &applicability_exact,
            &subject,
            &population,
        )
        .unwrap();

        assert!(matches!(
            resolved.validate_current(
                &requests,
                &retained_b,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            ),
            Err(RetainedAuthorityError::RetainedCorpusChanged)
        ));
    }
}
