// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Qualified R3 measurement/assimilation authority.
//!
//! A [`MeasurementAuthorityKey`] is only a semantic descriptor. Exact canonical
//! information enters through R3 only when a sealed qualification record and a
//! concrete current observation both cover the exact candidate claims. A
//! qualified measurement profile is not evidence that an observation happened.

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
    CapabilityEvidence, EcologicalInformation, EvidenceLineageToken, RepresentationKey,
};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::InformationRegistryError;
use crate::information_transition::{
    InformationTransitionKey, MeasurementAuthorityKey, PromotionProvenance,
    TransitionCapabilityClaim,
};
use crate::population::PopulationState;

use super::transition_domain::{
    TransitionDomainAuthorityError, TransitionDomainEvaluationSubject,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasurementRegistryKey {
    id: u128,
    version: u32,
}

impl MeasurementRegistryKey {
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
pub struct MeasurementQualificationProfileKey {
    id: u128,
    version: u32,
}

impl MeasurementQualificationProfileKey {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasurementImplementationFingerprint(Vec<u8>);

impl MeasurementImplementationFingerprint {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, MeasurementAuthorityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(MeasurementAuthorityError::EmptyImplementationFingerprint);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasurementCalibrationFingerprint(Vec<u8>);

impl MeasurementCalibrationFingerprint {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, MeasurementAuthorityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(MeasurementAuthorityError::EmptyCalibrationFingerprint);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasurementObservationManifest(Vec<u8>);

impl MeasurementObservationManifest {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, MeasurementAuthorityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(MeasurementAuthorityError::EmptyObservationManifest);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MeasurementObservationRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MeasurementQualificationStatus {
    Qualified,
    Revoked,
    Superseded,
}

/// Standing authority describing what one measurement profile is qualified to
/// introduce as exact canonical information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementQualification {
    authority: MeasurementAuthorityKey,
    source: RepresentationKey,
    destination: RepresentationKey,
    claims: BTreeSet<TransitionCapabilityClaim>,
    implementation: MeasurementImplementationFingerprint,
    profile: MeasurementQualificationProfileKey,
    qualification_evidence: EvidenceLineageToken,
    calibration: Option<MeasurementCalibrationFingerprint>,
    status: MeasurementQualificationStatus,
}

impl MeasurementQualification {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        authority: MeasurementAuthorityKey,
        source: RepresentationKey,
        destination: RepresentationKey,
        claims: impl IntoIterator<Item = TransitionCapabilityClaim>,
        implementation: MeasurementImplementationFingerprint,
        profile: MeasurementQualificationProfileKey,
        qualification_evidence: EvidenceLineageToken,
        calibration: Option<MeasurementCalibrationFingerprint>,
        status: MeasurementQualificationStatus,
    ) -> Result<Self, MeasurementAuthorityError> {
        let claims = claims.into_iter().collect::<BTreeSet<_>>();
        validate_exact_claim_set(authority, &claims)?;
        Ok(Self {
            authority,
            source,
            destination,
            claims,
            implementation,
            profile,
            qualification_evidence,
            calibration,
            status,
        })
    }

    pub const fn authority(&self) -> MeasurementAuthorityKey {
        self.authority
    }

    pub const fn source(&self) -> RepresentationKey {
        self.source
    }

    pub const fn destination(&self) -> RepresentationKey {
        self.destination
    }

    pub fn claims(&self) -> &BTreeSet<TransitionCapabilityClaim> {
        &self.claims
    }

    pub const fn implementation(&self) -> &MeasurementImplementationFingerprint {
        &self.implementation
    }

    pub const fn profile(&self) -> MeasurementQualificationProfileKey {
        self.profile
    }

    pub const fn qualification_evidence(&self) -> EvidenceLineageToken {
        self.qualification_evidence
    }

    pub const fn calibration(&self) -> Option<&MeasurementCalibrationFingerprint> {
        self.calibration.as_ref()
    }

    pub const fn status(&self) -> MeasurementQualificationStatus {
        self.status
    }
}

/// One concrete measurement/assimilation event for one exact source state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementObservation {
    authority: MeasurementAuthorityKey,
    subject: TransitionDomainEvaluationSubject,
    claims: BTreeSet<TransitionCapabilityClaim>,
    content_manifest: MeasurementObservationManifest,
    evidence_lineage: EvidenceLineageToken,
    revision: MeasurementObservationRevision,
}

impl MeasurementObservation {
    pub fn new(
        authority: MeasurementAuthorityKey,
        subject: TransitionDomainEvaluationSubject,
        claims: impl IntoIterator<Item = TransitionCapabilityClaim>,
        content_manifest: MeasurementObservationManifest,
        evidence_lineage: EvidenceLineageToken,
        revision: MeasurementObservationRevision,
    ) -> Result<Self, MeasurementAuthorityError> {
        let claims = claims.into_iter().collect::<BTreeSet<_>>();
        validate_exact_claim_set(authority, &claims)?;
        Ok(Self {
            authority,
            subject,
            claims,
            content_manifest,
            evidence_lineage,
            revision,
        })
    }

    pub const fn authority(&self) -> MeasurementAuthorityKey {
        self.authority
    }

    pub const fn subject(&self) -> &TransitionDomainEvaluationSubject {
        &self.subject
    }

    pub fn claims(&self) -> &BTreeSet<TransitionCapabilityClaim> {
        &self.claims
    }

    pub const fn content_manifest(&self) -> &MeasurementObservationManifest {
        &self.content_manifest
    }

    pub const fn evidence_lineage(&self) -> EvidenceLineageToken {
        self.evidence_lineage
    }

    pub const fn revision(&self) -> MeasurementObservationRevision {
        self.revision
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementAuthorityRegistryBuilder {
    key: MeasurementRegistryKey,
    qualifications: BTreeMap<MeasurementAuthorityKey, MeasurementQualification>,
    observations: BTreeMap<MeasurementAuthorityKey, MeasurementObservation>,
}

impl MeasurementAuthorityRegistryBuilder {
    pub const fn new(key: MeasurementRegistryKey) -> Self {
        Self {
            key,
            qualifications: BTreeMap::new(),
            observations: BTreeMap::new(),
        }
    }

    pub fn register_qualification(
        &mut self,
        qualification: MeasurementQualification,
    ) -> Result<(), MeasurementAuthorityError> {
        insert_unique(
            &mut self.qualifications,
            qualification.authority(),
            qualification,
            MeasurementAuthorityError::ConflictingQualification,
        )
    }

    pub fn register_observation(
        &mut self,
        observation: MeasurementObservation,
    ) -> Result<(), MeasurementAuthorityError> {
        insert_unique(
            &mut self.observations,
            observation.authority(),
            observation,
            MeasurementAuthorityError::ConflictingObservation,
        )
    }

    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<MeasurementAuthorityRegistry, MeasurementAuthorityError> {
        for qualification in self.qualifications.values() {
            validate_qualification_against_policy(policy, qualification)?;
        }
        for observation in self.observations.values() {
            let qualification = self
                .qualifications
                .get(&observation.authority())
                .ok_or(MeasurementAuthorityError::ObservationWithoutQualification {
                    authority: observation.authority(),
                })?;
            if observation.subject().source_representation() != qualification.source() {
                return Err(MeasurementAuthorityError::ObservationSourceMismatch {
                    authority: observation.authority(),
                });
            }
            if !observation.claims().is_subset(qualification.claims()) {
                return Err(MeasurementAuthorityError::ObservationClaimsExceedQualification {
                    authority: observation.authority(),
                });
            }
        }

        let authority = MeasurementAuthorityStamp {
            key: self.key,
            policy_authority: policy.authority_stamp().clone(),
            qualifications: self.qualifications.clone(),
            observations: self.observations.clone(),
        };
        Ok(MeasurementAuthorityRegistry {
            qualifications: self.qualifications,
            observations: self.observations,
            authority,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementAuthorityStamp {
    key: MeasurementRegistryKey,
    policy_authority: InformationPolicyAuthorityStamp,
    qualifications: BTreeMap<MeasurementAuthorityKey, MeasurementQualification>,
    observations: BTreeMap<MeasurementAuthorityKey, MeasurementObservation>,
}

impl MeasurementAuthorityStamp {
    pub const fn key(&self) -> MeasurementRegistryKey {
        self.key
    }

    pub const fn policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.policy_authority
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementAuthorityRegistry {
    qualifications: BTreeMap<MeasurementAuthorityKey, MeasurementQualification>,
    observations: BTreeMap<MeasurementAuthorityKey, MeasurementObservation>,
    authority: MeasurementAuthorityStamp,
}

impl MeasurementAuthorityRegistry {
    pub const fn authority_stamp(&self) -> &MeasurementAuthorityStamp {
        &self.authority
    }

    fn validate_policy(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), MeasurementAuthorityError> {
        self.authority
            .policy_authority
            .validate_registry(policy.registry())
            .map_err(MeasurementAuthorityError::PolicyIdentity)
    }

    fn resolve(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        authority: MeasurementAuthorityKey,
        candidate_transitions: &[InformationTransitionKey],
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        subject: &TransitionDomainEvaluationSubject,
    ) -> Result<ResolvedMeasurementAuthority, MeasurementAuthorityError> {
        self.validate_policy(policy)?;
        let qualification = self
            .qualifications
            .get(&authority)
            .ok_or(MeasurementAuthorityError::UnknownMeasurementAuthority { authority })?;
        match qualification.status() {
            MeasurementQualificationStatus::Qualified => {}
            status => {
                return Err(MeasurementAuthorityError::MeasurementNotQualified {
                    authority,
                    status,
                });
            }
        }

        let observation = self
            .observations
            .get(&authority)
            .ok_or(MeasurementAuthorityError::MissingObservation { authority })?;
        if observation.subject() != subject {
            return Err(MeasurementAuthorityError::ObservationSubjectMismatch { authority });
        }

        let required_claims = collect_candidate_r3_claims(
            applicability,
            candidate_transitions,
            authority,
            qualification,
            subject,
        )?;
        if !required_claims.is_subset(qualification.claims()) {
            return Err(MeasurementAuthorityError::RequiredClaimsExceedQualification {
                authority,
            });
        }
        if !required_claims.is_subset(observation.claims()) {
            return Err(MeasurementAuthorityError::RequiredClaimsMissingFromObservation {
                authority,
            });
        }

        Ok(ResolvedMeasurementAuthority {
            registry_authority: self.authority.clone(),
            qualification: qualification.clone(),
            observation: observation.clone(),
            required_claims,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedMeasurementAuthority {
    registry_authority: MeasurementAuthorityStamp,
    qualification: MeasurementQualification,
    observation: MeasurementObservation,
    required_claims: BTreeSet<TransitionCapabilityClaim>,
}

impl ResolvedMeasurementAuthority {
    pub const fn authority(&self) -> MeasurementAuthorityKey {
        self.qualification.authority()
    }

    pub const fn qualification(&self) -> &MeasurementQualification {
        &self.qualification
    }

    pub const fn observation(&self) -> &MeasurementObservation {
        &self.observation
    }

    pub fn required_claims(&self) -> &BTreeSet<TransitionCapabilityClaim> {
        &self.required_claims
    }

    pub const fn registry_authority(&self) -> &MeasurementAuthorityStamp {
        &self.registry_authority
    }
}

/// Candidate after resolving only R3 measurement obligations. This is not an
/// admissibility certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeasurementResolvedCandidateRequest {
    candidate_index: usize,
    applicability_authority: TransitionApplicabilityAuthorityStamp,
    measurement_authority: MeasurementAuthorityStamp,
    identity: CandidatePathIdentity,
    subject: TransitionDomainEvaluationSubject,
    resolved: BTreeMap<MeasurementAuthorityKey, ResolvedMeasurementAuthority>,
    remaining_obligations: BTreeSet<CandidateEvidenceObligation>,
    discarded_information: BTreeSet<EcologicalInformation>,
}

impl MeasurementResolvedCandidateRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        requests: &CandidateCertificationRequestSet,
        candidate_index: usize,
        registry: &MeasurementAuthorityRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        subject: &TransitionDomainEvaluationSubject,
        population: &PopulationState,
    ) -> Result<Self, MeasurementAuthorityError> {
        requests
            .validate_current(applicability)
            .map_err(MeasurementAuthorityError::CandidateRequest)?;
        registry.validate_policy(policy)?;
        if requests.information_policy_authority() != registry.authority_stamp().policy_authority() {
            return Err(MeasurementAuthorityError::PolicyAuthorityMismatch);
        }

        let request = requests
            .requests()
            .get(candidate_index)
            .ok_or(MeasurementAuthorityError::CandidateIndexOutOfRange {
                index: candidate_index,
                len: requests.requests().len(),
            })?;
        if request.identity().source() != subject.source_representation() {
            return Err(MeasurementAuthorityError::SourceRepresentationMismatch);
        }
        subject
            .validate_population(population)
            .map_err(MeasurementAuthorityError::SourceState)?;

        let mut resolved = BTreeMap::new();
        let mut remaining_obligations = BTreeSet::new();
        for obligation in request.obligations().iter().copied() {
            match obligation {
                CandidateEvidenceObligation::MeasurementAuthority(authority) => {
                    resolved.insert(
                        authority,
                        registry.resolve(
                            policy,
                            authority,
                            request.identity().transitions(),
                            applicability,
                            subject,
                        )?,
                    );
                }
                other => {
                    remaining_obligations.insert(other);
                }
            }
        }

        Ok(Self {
            candidate_index,
            applicability_authority: requests.applicability_authority().clone(),
            measurement_authority: registry.authority_stamp().clone(),
            identity: request.identity().clone(),
            subject: subject.clone(),
            resolved,
            remaining_obligations,
            discarded_information: request.discarded_information().clone(),
        })
    }

    pub fn resolved(&self) -> &BTreeMap<MeasurementAuthorityKey, ResolvedMeasurementAuthority> {
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
        registry: &MeasurementAuthorityRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        subject: &TransitionDomainEvaluationSubject,
        population: &PopulationState,
    ) -> Result<(), MeasurementAuthorityError> {
        if requests.applicability_authority() != &self.applicability_authority {
            return Err(MeasurementAuthorityError::ApplicabilityAuthorityChanged);
        }
        if registry.authority_stamp() != &self.measurement_authority {
            return Err(MeasurementAuthorityError::MeasurementCorpusChanged);
        }
        if subject != &self.subject {
            return Err(MeasurementAuthorityError::SubjectChanged);
        }
        let current = Self::resolve(
            requests,
            self.candidate_index,
            registry,
            policy,
            applicability,
            subject,
            population,
        )?;
        if current != *self {
            return Err(MeasurementAuthorityError::ResolutionChanged);
        }
        Ok(())
    }
}

fn collect_candidate_r3_claims(
    applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    candidate_transitions: &[InformationTransitionKey],
    authority: MeasurementAuthorityKey,
    qualification: &MeasurementQualification,
    subject: &TransitionDomainEvaluationSubject,
) -> Result<BTreeSet<TransitionCapabilityClaim>, MeasurementAuthorityError> {
    let registry = applicability.transitions().registry();
    let mut claims = BTreeSet::new();
    for transition_key in candidate_transitions {
        let transition = registry
            .transitions()
            .find_map(|(key, transition)| (key == transition_key).then_some(transition))
            .ok_or(MeasurementAuthorityError::TransitionMissing {
                transition: *transition_key,
            })?;
        let uses_authority = transition.introductions().iter().any(|(_, provenance)| {
            matches!(
                provenance,
                PromotionProvenance::MeasurementAssimilation { authority: found } if *found == authority
            )
        });
        if !uses_authority {
            continue;
        }
        if transition.source() != qualification.source()
            || transition.destination() != qualification.destination()
            || transition.source() != subject.source_representation()
        {
            return Err(MeasurementAuthorityError::TransitionShapeMismatch { authority });
        }
        for (claim, provenance) in transition.introductions() {
            if matches!(
                provenance,
                PromotionProvenance::MeasurementAssimilation { authority: found } if *found == authority
            ) {
                claims.insert(*claim);
            }
        }
    }
    if claims.is_empty() {
        return Err(MeasurementAuthorityError::NoClaimsForAuthority { authority });
    }
    Ok(claims)
}

fn validate_qualification_against_policy(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
    qualification: &MeasurementQualification,
) -> Result<(), MeasurementAuthorityError> {
    policy
        .registry()
        .resolve_representation(qualification.source())
        .map_err(|source| MeasurementAuthorityError::PolicyRegistry { source })?;
    let destination = policy
        .registry()
        .resolve_representation(qualification.destination())
        .map_err(|source| MeasurementAuthorityError::PolicyRegistry { source })?;
    for claim in qualification.claims() {
        let covered = destination.capabilities().claims().iter().any(|(available, evidence)| {
            available.covers(claim.information()) && evidence.contains(&CapabilityEvidence::Exact)
        });
        if !covered {
            return Err(MeasurementAuthorityError::DestinationMissingClaim {
                authority: qualification.authority(),
                claim: *claim,
            });
        }
    }
    Ok(())
}

fn validate_exact_claim_set(
    authority: MeasurementAuthorityKey,
    claims: &BTreeSet<TransitionCapabilityClaim>,
) -> Result<(), MeasurementAuthorityError> {
    if claims.is_empty() {
        return Err(MeasurementAuthorityError::EmptyClaimSet { authority });
    }
    if let Some(claim) = claims
        .iter()
        .find(|claim| claim.evidence() != CapabilityEvidence::Exact)
        .copied()
    {
        return Err(MeasurementAuthorityError::NonExactClaim { authority, claim });
    }
    Ok(())
}

fn insert_unique<K: Ord + Copy, V: PartialEq>(
    map: &mut BTreeMap<K, V>,
    key: K,
    value: V,
    conflict: fn(K) -> MeasurementAuthorityError,
) -> Result<(), MeasurementAuthorityError> {
    use std::collections::btree_map::Entry;
    match map.entry(key) {
        Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
        Entry::Occupied(entry) if entry.get() == &value => Ok(()),
        Entry::Occupied(_) => Err(conflict(key)),
    }
}

#[derive(Debug)]
pub enum MeasurementAuthorityError {
    EmptyImplementationFingerprint,
    EmptyCalibrationFingerprint,
    EmptyObservationManifest,
    EmptyClaimSet { authority: MeasurementAuthorityKey },
    NonExactClaim {
        authority: MeasurementAuthorityKey,
        claim: TransitionCapabilityClaim,
    },
    ConflictingQualification(MeasurementAuthorityKey),
    ConflictingObservation(MeasurementAuthorityKey),
    PolicyRegistry { source: InformationRegistryError },
    PolicyIdentity(InformationPolicyIdentityError),
    DestinationMissingClaim {
        authority: MeasurementAuthorityKey,
        claim: TransitionCapabilityClaim,
    },
    ObservationWithoutQualification { authority: MeasurementAuthorityKey },
    ObservationSourceMismatch { authority: MeasurementAuthorityKey },
    ObservationClaimsExceedQualification { authority: MeasurementAuthorityKey },
    UnknownMeasurementAuthority { authority: MeasurementAuthorityKey },
    MeasurementNotQualified {
        authority: MeasurementAuthorityKey,
        status: MeasurementQualificationStatus,
    },
    MissingObservation { authority: MeasurementAuthorityKey },
    ObservationSubjectMismatch { authority: MeasurementAuthorityKey },
    RequiredClaimsExceedQualification { authority: MeasurementAuthorityKey },
    RequiredClaimsMissingFromObservation { authority: MeasurementAuthorityKey },
    CandidateRequest(CandidateCertificationRequestError),
    PolicyAuthorityMismatch,
    CandidateIndexOutOfRange { index: usize, len: usize },
    SourceRepresentationMismatch,
    SourceState(TransitionDomainAuthorityError),
    TransitionMissing { transition: InformationTransitionKey },
    TransitionShapeMismatch { authority: MeasurementAuthorityKey },
    NoClaimsForAuthority { authority: MeasurementAuthorityKey },
    ApplicabilityAuthorityChanged,
    MeasurementCorpusChanged,
    SubjectChanged,
    ResolutionChanged,
}

impl fmt::Display for MeasurementAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for MeasurementAuthorityError {
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
    use crate::information::{EcologicalAuthorityLevel, RepresentationCapabilities};
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionRegistry,
        InformationTransitionRegistryBuilder, InformationTransitionRegistryKey, LosslessTransformKey,
    };
    use crate::living_world_authority::transition_domain::{
        TransitionDomainAuthorityScope, TransitionDomainSnapshotId, TransitionDomainSourceRevision,
    };
    use crate::population::{
        CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand,
    };
    use crate::transition_applicability::{
        TransitionApplicability, TransitionApplicabilityPolicy, TransitionApplicabilityPolicyBuilder,
        TransitionApplicabilityPolicyKey,
    };
    use crate::transition_candidates::TransitionCandidateSearchLimits;
    use crate::transition_policy_manifest::ManifestBoundInformationTransitionRegistry;

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(3_000, 1);
    const SOURCE: RepresentationKey = RepresentationKey::new(3_010, 1);
    const TARGET: RepresentationKey = RepresentationKey::new(3_011, 1);
    const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(3_020, 1);
    const EDGE: InformationTransitionKey = InformationTransitionKey::new(3_021, 1);
    const APP: TransitionApplicabilityPolicyKey = TransitionApplicabilityPolicyKey::new(3_030, 1);
    const MEASUREMENT: MeasurementAuthorityKey = MeasurementAuthorityKey(3_040, 1);
    const TRANSFORM: LosslessTransformKey = LosslessTransformKey(3_041, 1);
    const REGISTRY: MeasurementRegistryKey = MeasurementRegistryKey::new(3_050, 1);
    const PROFILE: MeasurementQualificationProfileKey =
        MeasurementQualificationProfileKey::new(3_051, 1);
    const SCOPE: TransitionDomainAuthorityScope = TransitionDomainAuthorityScope::new(3_060, 1);
    const SNAPSHOT: TransitionDomainSnapshotId = TransitionDomainSnapshotId(3_070);

    fn exact_claim(info: EcologicalInformation) -> TransitionCapabilityClaim {
        TransitionCapabilityClaim::new(info, CapabilityEvidence::Exact)
    }

    fn capabilities(
        key: RepresentationKey,
        infos: impl IntoIterator<Item = EcologicalInformation>,
    ) -> RepresentationCapabilities {
        RepresentationCapabilities::new(
            key,
            EcologicalAuthorityLevel::Coarse,
            infos.into_iter().map(|info| (info, CapabilityEvidence::Exact)),
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
        builder.seal()
    }

    fn transitions(
        policy: &InformationPolicyRegistry,
        both_measured: bool,
    ) -> InformationTransitionRegistry {
        let disease = if both_measured {
            PromotionProvenance::MeasurementAssimilation {
                authority: MEASUREMENT,
            }
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
                    PromotionProvenance::MeasurementAssimilation {
                        authority: MEASUREMENT,
                    },
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

    fn qualification(
        claims: impl IntoIterator<Item = TransitionCapabilityClaim>,
        status: MeasurementQualificationStatus,
        implementation: &[u8],
    ) -> MeasurementQualification {
        MeasurementQualification::new(
            MEASUREMENT,
            SOURCE,
            TARGET,
            claims,
            MeasurementImplementationFingerprint::new(implementation.to_vec()).unwrap(),
            PROFILE,
            EvidenceLineageToken(3_080),
            Some(MeasurementCalibrationFingerprint::new(b"calibration-a".to_vec()).unwrap()),
            status,
        )
        .unwrap()
    }

    fn observation(
        subject: &TransitionDomainEvaluationSubject,
        claims: impl IntoIterator<Item = TransitionCapabilityClaim>,
        manifest: &[u8],
    ) -> MeasurementObservation {
        MeasurementObservation::new(
            MEASUREMENT,
            subject.clone(),
            claims,
            MeasurementObservationManifest::new(manifest.to_vec()).unwrap(),
            EvidenceLineageToken(3_081),
            MeasurementObservationRevision(7),
        )
        .unwrap()
    }

    fn registry(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        qualification: MeasurementQualification,
        observation: Option<MeasurementObservation>,
    ) -> MeasurementAuthorityRegistry {
        let mut builder = MeasurementAuthorityRegistryBuilder::new(REGISTRY);
        builder.register_qualification(qualification).unwrap();
        if let Some(observation) = observation {
            builder.register_observation(observation).unwrap();
        }
        builder.seal(policy).unwrap()
    }

    #[test]
    fn qualified_observation_resolves_r3_while_r1_and_loss_remain() {
        let policy = policy();
        let transitions = transitions(&policy, false);
        let applicability = applicability(&transitions);
        let (policy_exact, applicability_exact, requests) =
            exact_context(&policy, &transitions, &applicability);
        let population = population();
        let subject = subject(&population);
        let registry = registry(
            &policy_exact,
            qualification(
                [exact_claim(EcologicalInformation::SpatialStructure)],
                MeasurementQualificationStatus::Qualified,
                b"instrument-a",
            ),
            Some(observation(
                &subject,
                [exact_claim(EcologicalInformation::SpatialStructure)],
                b"observation-a",
            )),
        );

        let resolved = MeasurementResolvedCandidateRequest::resolve(
            &requests,
            0,
            &registry,
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
    fn qualified_profile_without_observation_rejects() {
        let policy = policy();
        let transitions = transitions(&policy, false);
        let applicability = applicability(&transitions);
        let (policy_exact, applicability_exact, requests) =
            exact_context(&policy, &transitions, &applicability);
        let population = population();
        let subject = subject(&population);
        let registry = registry(
            &policy_exact,
            qualification(
                [exact_claim(EcologicalInformation::SpatialStructure)],
                MeasurementQualificationStatus::Qualified,
                b"instrument-a",
            ),
            None,
        );

        assert!(matches!(
            MeasurementResolvedCandidateRequest::resolve(
                &requests,
                0,
                &registry,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            ),
            Err(MeasurementAuthorityError::MissingObservation { .. })
        ));
    }

    #[test]
    fn observation_missing_one_of_two_r3_claims_rejects() {
        let policy = policy();
        let transitions = transitions(&policy, true);
        let applicability = applicability(&transitions);
        let (policy_exact, applicability_exact, requests) =
            exact_context(&policy, &transitions, &applicability);
        let population = population();
        let subject = subject(&population);
        let registry = registry(
            &policy_exact,
            qualification(
                [
                    exact_claim(EcologicalInformation::SpatialStructure),
                    exact_claim(EcologicalInformation::DiseaseState),
                ],
                MeasurementQualificationStatus::Qualified,
                b"instrument-a",
            ),
            Some(observation(
                &subject,
                [exact_claim(EcologicalInformation::SpatialStructure)],
                b"observation-a",
            )),
        );

        assert!(matches!(
            MeasurementResolvedCandidateRequest::resolve(
                &requests,
                0,
                &registry,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            ),
            Err(MeasurementAuthorityError::RequiredClaimsMissingFromObservation { .. })
        ));
    }

    #[test]
    fn changed_observation_content_stales_old_resolution() {
        let policy = policy();
        let transitions = transitions(&policy, false);
        let applicability = applicability(&transitions);
        let (policy_exact, applicability_exact, requests) =
            exact_context(&policy, &transitions, &applicability);
        let population = population();
        let subject = subject(&population);
        let q = qualification(
            [exact_claim(EcologicalInformation::SpatialStructure)],
            MeasurementQualificationStatus::Qualified,
            b"instrument-a",
        );
        let registry_a = registry(
            &policy_exact,
            q.clone(),
            Some(observation(
                &subject,
                [exact_claim(EcologicalInformation::SpatialStructure)],
                b"observation-a",
            )),
        );
        let registry_b = registry(
            &policy_exact,
            q,
            Some(observation(
                &subject,
                [exact_claim(EcologicalInformation::SpatialStructure)],
                b"observation-b",
            )),
        );
        let resolved = MeasurementResolvedCandidateRequest::resolve(
            &requests,
            0,
            &registry_a,
            &policy_exact,
            &applicability_exact,
            &subject,
            &population,
        )
        .unwrap();

        assert!(matches!(
            resolved.validate_current(
                &requests,
                &registry_b,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            ),
            Err(MeasurementAuthorityError::MeasurementCorpusChanged)
        ));
    }
}
