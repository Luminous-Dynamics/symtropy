// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Authority binding for the scalar observations consumed by Q2 shadow validation.
//!
//! `shadow_validation` deliberately performs deterministic metric arithmetic over a
//! sealed scalar trace. That proves agreement of the submitted numbers, but not by
//! itself that those numbers are observations of the typed closure observable.
//! This layer closes that evidence gap without turning `lifesim-core` into a
//! universal telemetry system.
//!
//! A shadow observation is anchor-grade only when it resolves through an immutable
//! qualified extractor corpus that binds the exact closure observable, source
//! representation, scalar unit, implementation/profile, execution environment and
//! source-state identity. The strengthened certificate then proves that every raw
//! coarse/reference sample in the Q2 trace has one matching resolved observation.
//! No ecological state is mutated here.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::{EvidenceLineageToken, RepresentationKey};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::InformationRegistryError;
use crate::population::PopulationState;

use super::closure_usage_authority::ClosureUsagePolicyRegistry;
use super::shadow_validation::{
    ShadowFixedScalar, ShadowScalarUnitKey, ShadowValidationAuthorityError,
    ShadowValidationCertificate, ShadowValidationRegistry,
};
use super::spatiotemporal_information::{CanonicalTick, SpatiotemporalPolicyRegistry};
use super::transition_domain::{
    TransitionDomainAuthorityScope, TransitionDomainSnapshotId, TransitionDomainSourceRevision,
};
use super::typed_closure_process_acceptance::TypedClosureProcessAcceptanceRegistry;
use super::typed_closure_qualification::{
    ClosureObservableKey, TypedClosureQualificationRegistry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowObservableRegistryKey {
    id: u128,
    version: u32,
}
impl ShadowObservableRegistryKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowObservableExtractorKey {
    id: u128,
    version: u32,
}
impl ShadowObservableExtractorKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowObservableQualificationProfileKey {
    id: u128,
    version: u32,
}
impl ShadowObservableQualificationProfileKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowObservationKey {
    id: u128,
    version: u32,
}
impl ShadowObservationKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowObservationRevision(pub u64);

macro_rules! opaque_bytes {
    ($name:ident, $error:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Vec<u8>);
        impl $name {
            pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, ShadowObservableAuthorityError> {
                let bytes = bytes.into();
                if bytes.is_empty() {
                    return Err(ShadowObservableAuthorityError::$error);
                }
                Ok(Self(bytes))
            }
            pub fn as_bytes(&self) -> &[u8] { &self.0 }
        }
    };
}

opaque_bytes!(ShadowObservableImplementationFingerprint, EmptyImplementationFingerprint);
opaque_bytes!(ShadowObservableExecutionCapsuleFingerprint, EmptyExecutionCapsule);
opaque_bytes!(ShadowObservationContentManifest, EmptyObservationContentManifest);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShadowObservableQualificationStatus {
    Qualified,
    Revoked,
    Superseded,
}

/// Standing qualification for one exact scalar extractor.
///
/// Construction is evidence/bootstrap ingestion. Runtime authority comes only from
/// resolving through the sealed registry generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowObservableExtractorQualification {
    extractor: ShadowObservableExtractorKey,
    observable: ClosureObservableKey,
    representation: RepresentationKey,
    unit: ShadowScalarUnitKey,
    implementation: ShadowObservableImplementationFingerprint,
    profile: ShadowObservableQualificationProfileKey,
    qualification_evidence: EvidenceLineageToken,
    execution_capsule: Option<ShadowObservableExecutionCapsuleFingerprint>,
    status: ShadowObservableQualificationStatus,
}
impl ShadowObservableExtractorQualification {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        extractor: ShadowObservableExtractorKey,
        observable: ClosureObservableKey,
        representation: RepresentationKey,
        unit: ShadowScalarUnitKey,
        implementation: ShadowObservableImplementationFingerprint,
        profile: ShadowObservableQualificationProfileKey,
        qualification_evidence: EvidenceLineageToken,
        execution_capsule: Option<ShadowObservableExecutionCapsuleFingerprint>,
        status: ShadowObservableQualificationStatus,
    ) -> Self {
        Self {
            extractor,
            observable,
            representation,
            unit,
            implementation,
            profile,
            qualification_evidence,
            execution_capsule,
            status,
        }
    }
    pub const fn extractor(&self) -> ShadowObservableExtractorKey { self.extractor }
    pub const fn observable(&self) -> ClosureObservableKey { self.observable }
    pub const fn representation(&self) -> RepresentationKey { self.representation }
    pub const fn unit(&self) -> ShadowScalarUnitKey { self.unit }
    pub const fn implementation(&self) -> &ShadowObservableImplementationFingerprint { &self.implementation }
    pub const fn profile(&self) -> ShadowObservableQualificationProfileKey { self.profile }
    pub const fn qualification_evidence(&self) -> EvidenceLineageToken { self.qualification_evidence }
    pub const fn execution_capsule(&self) -> Option<&ShadowObservableExecutionCapsuleFingerprint> { self.execution_capsule.as_ref() }
    pub const fn status(&self) -> ShadowObservableQualificationStatus { self.status }
}

/// Exact identity of the source state from which one observable value was extracted.
///
/// Scope/snapshot/revision are structural state identity. `content_manifest` is the
/// producer-owned exact content identity for the representation state itself. The
/// manifest is opaque here; this module does not invent a universal state encoder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowObservationSourceIdentity {
    representation: RepresentationKey,
    scope: TransitionDomainAuthorityScope,
    snapshot: TransitionDomainSnapshotId,
    source_revision: TransitionDomainSourceRevision,
    content_manifest: ShadowObservationContentManifest,
}
impl ShadowObservationSourceIdentity {
    pub fn new(
        representation: RepresentationKey,
        scope: TransitionDomainAuthorityScope,
        snapshot: TransitionDomainSnapshotId,
        source_revision: TransitionDomainSourceRevision,
        content_manifest: ShadowObservationContentManifest,
    ) -> Self {
        Self { representation, scope, snapshot, source_revision, content_manifest }
    }
    pub const fn representation(&self) -> RepresentationKey { self.representation }
    pub const fn scope(&self) -> TransitionDomainAuthorityScope { self.scope }
    pub const fn snapshot(&self) -> TransitionDomainSnapshotId { self.snapshot }
    pub const fn source_revision(&self) -> TransitionDomainSourceRevision { self.source_revision }
    pub const fn content_manifest(&self) -> &ShadowObservationContentManifest { &self.content_manifest }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShadowObservationExecutionStatus {
    Completed,
    Failed,
    Cancelled,
}

/// Immutable execution record for one extractor invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowObservationRecord {
    key: ShadowObservationKey,
    revision: ShadowObservationRevision,
    extractor: ShadowObservableExtractorKey,
    tick: CanonicalTick,
    source: ShadowObservationSourceIdentity,
    status: ShadowObservationExecutionStatus,
    value: Option<ShadowFixedScalar>,
    evidence_lineage: EvidenceLineageToken,
}
impl ShadowObservationRecord {
    pub fn completed(
        key: ShadowObservationKey,
        revision: ShadowObservationRevision,
        extractor: ShadowObservableExtractorKey,
        tick: CanonicalTick,
        source: ShadowObservationSourceIdentity,
        value: ShadowFixedScalar,
        evidence_lineage: EvidenceLineageToken,
    ) -> Self {
        Self {
            key,
            revision,
            extractor,
            tick,
            source,
            status: ShadowObservationExecutionStatus::Completed,
            value: Some(value),
            evidence_lineage,
        }
    }

    pub fn incomplete(
        key: ShadowObservationKey,
        revision: ShadowObservationRevision,
        extractor: ShadowObservableExtractorKey,
        tick: CanonicalTick,
        source: ShadowObservationSourceIdentity,
        status: ShadowObservationExecutionStatus,
        evidence_lineage: EvidenceLineageToken,
    ) -> Result<Self, ShadowObservableAuthorityError> {
        if status == ShadowObservationExecutionStatus::Completed {
            return Err(ShadowObservableAuthorityError::CompletedObservationRequiresValue);
        }
        Ok(Self {
            key,
            revision,
            extractor,
            tick,
            source,
            status,
            value: None,
            evidence_lineage,
        })
    }

    pub const fn key(&self) -> ShadowObservationKey { self.key }
    pub const fn revision(&self) -> ShadowObservationRevision { self.revision }
    pub const fn extractor(&self) -> ShadowObservableExtractorKey { self.extractor }
    pub const fn tick(&self) -> CanonicalTick { self.tick }
    pub const fn source(&self) -> &ShadowObservationSourceIdentity { &self.source }
    pub const fn status(&self) -> ShadowObservationExecutionStatus { self.status }
    pub const fn value(&self) -> Option<ShadowFixedScalar> { self.value }
    pub const fn evidence_lineage(&self) -> EvidenceLineageToken { self.evidence_lineage }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowObservableAuthorityRegistryBuilder {
    key: ShadowObservableRegistryKey,
    qualifications: BTreeMap<ShadowObservableExtractorKey, ShadowObservableExtractorQualification>,
    observations: BTreeMap<ShadowObservationKey, ShadowObservationRecord>,
}
impl ShadowObservableAuthorityRegistryBuilder {
    pub const fn new(key: ShadowObservableRegistryKey) -> Self {
        Self { key, qualifications: BTreeMap::new(), observations: BTreeMap::new() }
    }

    pub fn register_qualification(
        &mut self,
        qualification: ShadowObservableExtractorQualification,
    ) -> Result<(), ShadowObservableAuthorityError> {
        use std::collections::btree_map::Entry;
        match self.qualifications.entry(qualification.extractor()) {
            Entry::Vacant(entry) => { entry.insert(qualification); Ok(()) }
            Entry::Occupied(entry) if entry.get() == &qualification => Ok(()),
            Entry::Occupied(entry) => Err(ShadowObservableAuthorityError::ConflictingExtractorRegistration {
                extractor: *entry.key(),
            }),
        }
    }

    pub fn register_observation(
        &mut self,
        observation: ShadowObservationRecord,
    ) -> Result<(), ShadowObservableAuthorityError> {
        use std::collections::btree_map::Entry;
        match self.observations.entry(observation.key()) {
            Entry::Vacant(entry) => { entry.insert(observation); Ok(()) }
            Entry::Occupied(entry) if entry.get() == &observation => Ok(()),
            Entry::Occupied(entry) => Err(ShadowObservableAuthorityError::ConflictingObservationRegistration {
                observation: *entry.key(),
            }),
        }
    }

    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<ShadowObservableAuthorityRegistry, ShadowObservableAuthorityError> {
        policy.authority_stamp().validate_registry(policy.registry())
            .map_err(ShadowObservableAuthorityError::PolicyIdentity)?;

        for qualification in self.qualifications.values() {
            policy.registry().resolve_representation(qualification.representation())
                .map_err(ShadowObservableAuthorityError::InformationRegistry)?;
        }
        for observation in self.observations.values() {
            let qualification = self.qualifications.get(&observation.extractor())
                .ok_or(ShadowObservableAuthorityError::ObservationWithoutExtractor {
                    observation: observation.key(),
                    extractor: observation.extractor(),
                })?;
            if observation.source().representation() != qualification.representation() {
                return Err(ShadowObservableAuthorityError::ObservationRepresentationMismatch {
                    observation: observation.key(),
                    expected: qualification.representation(),
                    actual: observation.source().representation(),
                });
            }
        }

        let authority = ShadowObservableAuthorityStamp {
            key: self.key,
            information_policy_authority: policy.authority_stamp().clone(),
            qualifications: self.qualifications.clone(),
            observations: self.observations.clone(),
        };
        Ok(ShadowObservableAuthorityRegistry {
            authority,
            qualifications: self.qualifications,
            observations: self.observations,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowObservableAuthorityStamp {
    key: ShadowObservableRegistryKey,
    information_policy_authority: InformationPolicyAuthorityStamp,
    qualifications: BTreeMap<ShadowObservableExtractorKey, ShadowObservableExtractorQualification>,
    observations: BTreeMap<ShadowObservationKey, ShadowObservationRecord>,
}
impl ShadowObservableAuthorityStamp {
    pub const fn key(&self) -> ShadowObservableRegistryKey { self.key }
    pub const fn information_policy_authority(&self) -> &InformationPolicyAuthorityStamp { &self.information_policy_authority }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowObservableAuthorityRegistry {
    authority: ShadowObservableAuthorityStamp,
    qualifications: BTreeMap<ShadowObservableExtractorKey, ShadowObservableExtractorQualification>,
    observations: BTreeMap<ShadowObservationKey, ShadowObservationRecord>,
}
impl ShadowObservableAuthorityRegistry {
    pub const fn authority_stamp(&self) -> &ShadowObservableAuthorityStamp { &self.authority }

    pub fn validate_current(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), ShadowObservableAuthorityError> {
        self.authority.information_policy_authority.validate_registry(policy.registry())
            .map_err(ShadowObservableAuthorityError::PolicyIdentity)?;
        if policy.authority_stamp() != &self.authority.information_policy_authority {
            return Err(ShadowObservableAuthorityError::PolicyAuthorityChanged);
        }
        Ok(())
    }

    pub fn resolve_observation(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        key: ShadowObservationKey,
    ) -> Result<ResolvedShadowObservation, ShadowObservableAuthorityError> {
        self.validate_current(policy)?;
        let observation = self.observations.get(&key)
            .ok_or(ShadowObservableAuthorityError::UnknownObservation { observation: key })?;
        if observation.status() != ShadowObservationExecutionStatus::Completed {
            return Err(ShadowObservableAuthorityError::ObservationNotCompleted {
                observation: key,
                status: observation.status(),
            });
        }
        let qualification = self.qualifications.get(&observation.extractor())
            .ok_or(ShadowObservableAuthorityError::UnknownExtractor {
                extractor: observation.extractor(),
            })?;
        if qualification.status() != ShadowObservableQualificationStatus::Qualified {
            return Err(ShadowObservableAuthorityError::ExtractorNotQualified {
                extractor: qualification.extractor(),
                status: qualification.status(),
            });
        }
        if observation.source().representation() != qualification.representation() {
            return Err(ShadowObservableAuthorityError::ObservationRepresentationMismatch {
                observation: key,
                expected: qualification.representation(),
                actual: observation.source().representation(),
            });
        }
        Ok(ResolvedShadowObservation {
            registry_authority: self.authority.clone(),
            qualification: qualification.clone(),
            observation: observation.clone(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn certify_shadow_trace(
        &self,
        shadow_certificate: &ShadowValidationCertificate,
        pairs: impl IntoIterator<Item = ShadowObservationPair>,
        shadow_registry: &ShadowValidationRegistry,
        usage: &ClosureUsagePolicyRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        current_coarse_population: &PopulationState,
    ) -> Result<ObservableBoundShadowValidationCertificate, ShadowObservableAuthorityError> {
        self.validate_current(policy)?;
        shadow_certificate.validate_current(
            shadow_registry,
            usage,
            policy,
            closures,
            acceptances,
            spatiotemporal,
            current_coarse_population,
        ).map_err(ShadowObservableAuthorityError::ShadowValidation)?;

        let qualification = shadow_certificate.resolved_use().qualification().qualification();
        let observable = qualification.observable();
        let unit = shadow_certificate.profile().scalar_unit();
        let coarse_representation = shadow_certificate.resolved_use().representation();
        let reference_representation = shadow_certificate.profile().reference_representation();
        let raw_trace = shadow_certificate.evidence().trace()
            .ok_or(ShadowObservableAuthorityError::ShadowTraceMissing)?;
        if raw_trace.unit() != unit {
            return Err(ShadowObservableAuthorityError::ShadowTraceUnitChanged {
                expected: unit,
                actual: raw_trace.unit(),
            });
        }

        let mut requested = BTreeMap::new();
        for pair in pairs {
            if requested.insert(pair.tick(), pair).is_some() {
                return Err(ShadowObservableAuthorityError::DuplicateBindingTick { tick: pair.tick() });
            }
        }
        if requested.len() != raw_trace.len() {
            return Err(ShadowObservableAuthorityError::BindingCardinalityMismatch {
                expected: raw_trace.len(),
                actual: requested.len(),
            });
        }

        let mut bindings = BTreeMap::new();
        for raw in raw_trace.samples() {
            let pair = requested.remove(&raw.tick())
                .ok_or(ShadowObservableAuthorityError::MissingBindingTick { tick: raw.tick() })?;
            let coarse = self.resolve_observation(policy, pair.coarse())?;
            let reference = self.resolve_observation(policy, pair.reference())?;
            validate_resolved_observation(
                &coarse,
                observable,
                unit,
                coarse_representation,
                raw.tick(),
                raw.coarse(),
                ShadowObservationLane::Coarse,
            )?;
            validate_resolved_observation(
                &reference,
                observable,
                unit,
                reference_representation,
                raw.tick(),
                raw.reference(),
                ShadowObservationLane::Reference,
            )?;
            bindings.insert(raw.tick(), ShadowObservationBinding { coarse, reference });
        }
        if let Some(tick) = requested.keys().next().copied() {
            return Err(ShadowObservableAuthorityError::UnexpectedBindingTick { tick });
        }

        Ok(ObservableBoundShadowValidationCertificate {
            shadow: shadow_certificate.clone(),
            observable_authority: self.authority.clone(),
            observable,
            unit,
            bindings,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedShadowObservation {
    registry_authority: ShadowObservableAuthorityStamp,
    qualification: ShadowObservableExtractorQualification,
    observation: ShadowObservationRecord,
}
impl ResolvedShadowObservation {
    pub const fn registry_authority(&self) -> &ShadowObservableAuthorityStamp { &self.registry_authority }
    pub const fn qualification(&self) -> &ShadowObservableExtractorQualification { &self.qualification }
    pub const fn observation(&self) -> &ShadowObservationRecord { &self.observation }

    pub fn validate_current(
        &self,
        registry: &ShadowObservableAuthorityRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), ShadowObservableAuthorityError> {
        if registry.authority_stamp() != &self.registry_authority {
            return Err(ShadowObservableAuthorityError::ObservableAuthorityChanged);
        }
        let current = registry.resolve_observation(policy, self.observation.key())?;
        if current != *self {
            return Err(ShadowObservableAuthorityError::ResolvedObservationStale {
                observation: self.observation.key(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowObservationPair {
    tick: CanonicalTick,
    coarse: ShadowObservationKey,
    reference: ShadowObservationKey,
}
impl ShadowObservationPair {
    pub const fn new(
        tick: CanonicalTick,
        coarse: ShadowObservationKey,
        reference: ShadowObservationKey,
    ) -> Self {
        Self { tick, coarse, reference }
    }
    pub const fn tick(self) -> CanonicalTick { self.tick }
    pub const fn coarse(self) -> ShadowObservationKey { self.coarse }
    pub const fn reference(self) -> ShadowObservationKey { self.reference }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowObservationBinding {
    coarse: ResolvedShadowObservation,
    reference: ResolvedShadowObservation,
}
impl ShadowObservationBinding {
    pub const fn coarse(&self) -> &ResolvedShadowObservation { &self.coarse }
    pub const fn reference(&self) -> &ResolvedShadowObservation { &self.reference }
}

/// Anchor-grade Q2 evidence precursor.
///
/// This is intentionally stronger than `ShadowValidationCertificate`: it proves
/// that every scalar used by the deterministic Q2 metric resolves to the exact
/// typed observable through the current qualified extractor corpus. #392's
/// private-anchor bridge should consume this type (plus legitimate common-start
/// provenance), never a bare raw-scalar shadow certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservableBoundShadowValidationCertificate {
    shadow: ShadowValidationCertificate,
    observable_authority: ShadowObservableAuthorityStamp,
    observable: ClosureObservableKey,
    unit: ShadowScalarUnitKey,
    bindings: BTreeMap<CanonicalTick, ShadowObservationBinding>,
}
impl ObservableBoundShadowValidationCertificate {
    pub const fn shadow(&self) -> &ShadowValidationCertificate { &self.shadow }
    pub const fn observable_authority(&self) -> &ShadowObservableAuthorityStamp { &self.observable_authority }
    pub const fn observable(&self) -> ClosureObservableKey { self.observable }
    pub const fn unit(&self) -> ShadowScalarUnitKey { self.unit }
    pub fn bindings(&self) -> impl Iterator<Item = (&CanonicalTick, &ShadowObservationBinding)> {
        self.bindings.iter()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        registry: &ShadowObservableAuthorityRegistry,
        shadow_registry: &ShadowValidationRegistry,
        usage: &ClosureUsagePolicyRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        current_coarse_population: &PopulationState,
    ) -> Result<(), ShadowObservableAuthorityError> {
        if registry.authority_stamp() != &self.observable_authority {
            return Err(ShadowObservableAuthorityError::ObservableAuthorityChanged);
        }
        let pairs = self.bindings.iter().map(|(tick, binding)| {
            ShadowObservationPair::new(
                *tick,
                binding.coarse().observation().key(),
                binding.reference().observation().key(),
            )
        }).collect::<Vec<_>>();
        let current = registry.certify_shadow_trace(
            &self.shadow,
            pairs,
            shadow_registry,
            usage,
            policy,
            closures,
            acceptances,
            spatiotemporal,
            current_coarse_population,
        )?;
        if current != *self {
            return Err(ShadowObservableAuthorityError::ObservableBoundCertificateStale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowObservationLane {
    Coarse,
    Reference,
}

fn validate_resolved_observation(
    resolved: &ResolvedShadowObservation,
    observable: ClosureObservableKey,
    unit: ShadowScalarUnitKey,
    representation: RepresentationKey,
    tick: CanonicalTick,
    value: ShadowFixedScalar,
    lane: ShadowObservationLane,
) -> Result<(), ShadowObservableAuthorityError> {
    let qualification = resolved.qualification();
    let observation = resolved.observation();
    if qualification.observable() != observable {
        return Err(ShadowObservableAuthorityError::ObservableMismatch {
            lane,
            expected: observable,
            actual: qualification.observable(),
        });
    }
    if qualification.unit() != unit {
        return Err(ShadowObservableAuthorityError::UnitMismatch {
            lane,
            expected: unit,
            actual: qualification.unit(),
        });
    }
    if qualification.representation() != representation
        || observation.source().representation() != representation
    {
        return Err(ShadowObservableAuthorityError::LaneRepresentationMismatch {
            lane,
            expected: representation,
            actual: observation.source().representation(),
        });
    }
    if observation.tick() != tick {
        return Err(ShadowObservableAuthorityError::ObservationTickMismatch {
            lane,
            expected: tick,
            actual: observation.tick(),
        });
    }
    let actual = observation.value()
        .ok_or(ShadowObservableAuthorityError::CompletedObservationRequiresValue)?;
    if actual != value {
        return Err(ShadowObservableAuthorityError::ObservationValueMismatch {
            lane,
            tick,
            expected: value,
            actual,
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShadowObservableAuthorityError {
    EmptyImplementationFingerprint,
    EmptyExecutionCapsule,
    EmptyObservationContentManifest,
    CompletedObservationRequiresValue,
    ConflictingExtractorRegistration { extractor: ShadowObservableExtractorKey },
    ConflictingObservationRegistration { observation: ShadowObservationKey },
    ObservationWithoutExtractor { observation: ShadowObservationKey, extractor: ShadowObservableExtractorKey },
    ObservationRepresentationMismatch { observation: ShadowObservationKey, expected: RepresentationKey, actual: RepresentationKey },
    PolicyIdentity(InformationPolicyIdentityError),
    PolicyAuthorityChanged,
    InformationRegistry(InformationRegistryError),
    UnknownObservation { observation: ShadowObservationKey },
    UnknownExtractor { extractor: ShadowObservableExtractorKey },
    ObservationNotCompleted { observation: ShadowObservationKey, status: ShadowObservationExecutionStatus },
    ExtractorNotQualified { extractor: ShadowObservableExtractorKey, status: ShadowObservableQualificationStatus },
    ShadowValidation(ShadowValidationAuthorityError),
    ShadowTraceMissing,
    ShadowTraceUnitChanged { expected: ShadowScalarUnitKey, actual: ShadowScalarUnitKey },
    DuplicateBindingTick { tick: CanonicalTick },
    BindingCardinalityMismatch { expected: usize, actual: usize },
    MissingBindingTick { tick: CanonicalTick },
    UnexpectedBindingTick { tick: CanonicalTick },
    ObservableMismatch { lane: ShadowObservationLane, expected: ClosureObservableKey, actual: ClosureObservableKey },
    UnitMismatch { lane: ShadowObservationLane, expected: ShadowScalarUnitKey, actual: ShadowScalarUnitKey },
    LaneRepresentationMismatch { lane: ShadowObservationLane, expected: RepresentationKey, actual: RepresentationKey },
    ObservationTickMismatch { lane: ShadowObservationLane, expected: CanonicalTick, actual: CanonicalTick },
    ObservationValueMismatch { lane: ShadowObservationLane, tick: CanonicalTick, expected: ShadowFixedScalar, actual: ShadowFixedScalar },
    ObservableAuthorityChanged,
    ResolvedObservationStale { observation: ShadowObservationKey },
    ObservableBoundCertificateStale,
}

impl fmt::Display for ShadowObservableAuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyImplementationFingerprint => write!(f, "shadow observable implementation fingerprint is empty"),
            Self::EmptyExecutionCapsule => write!(f, "shadow observable execution capsule is empty"),
            Self::EmptyObservationContentManifest => write!(f, "shadow observation source-content manifest is empty"),
            Self::CompletedObservationRequiresValue => write!(f, "completed shadow observation requires one scalar value"),
            Self::ConflictingExtractorRegistration { extractor } => write!(f, "conflicting shadow observable extractor {}@{}", extractor.id(), extractor.version()),
            Self::ConflictingObservationRegistration { observation } => write!(f, "conflicting shadow observation {}@{}", observation.id(), observation.version()),
            Self::ObservationWithoutExtractor { observation, extractor } => write!(f, "shadow observation {}@{} references unknown extractor {}@{}", observation.id(), observation.version(), extractor.id(), extractor.version()),
            Self::ObservationRepresentationMismatch { observation, expected, actual } => write!(f, "shadow observation {}@{} source representation {actual:?} differs from qualified {expected:?}", observation.id(), observation.version()),
            Self::PolicyIdentity(error) => write!(f, "information-policy identity error: {error}"),
            Self::PolicyAuthorityChanged => write!(f, "exact information-policy authority changed for shadow observable corpus"),
            Self::InformationRegistry(error) => write!(f, "information registry error: {error}"),
            Self::UnknownObservation { observation } => write!(f, "unknown shadow observation {}@{}", observation.id(), observation.version()),
            Self::UnknownExtractor { extractor } => write!(f, "unknown shadow observable extractor {}@{}", extractor.id(), extractor.version()),
            Self::ObservationNotCompleted { observation, status } => write!(f, "shadow observation {}@{} is {status:?}, not Completed", observation.id(), observation.version()),
            Self::ExtractorNotQualified { extractor, status } => write!(f, "shadow observable extractor {}@{} is {status:?}, not Qualified", extractor.id(), extractor.version()),
            Self::ShadowValidation(error) => write!(f, "shadow validation error: {error}"),
            Self::ShadowTraceMissing => write!(f, "Q2 shadow certificate has no comparison trace"),
            Self::ShadowTraceUnitChanged { expected, actual } => write!(f, "Q2 shadow trace unit {}@{} differs from certificate profile unit {}@{}", actual.id(), actual.version(), expected.id(), expected.version()),
            Self::DuplicateBindingTick { tick } => write!(f, "duplicate observable binding at canonical tick {}", tick.0),
            Self::BindingCardinalityMismatch { expected, actual } => write!(f, "observable binding count {actual} differs from Q2 trace count {expected}"),
            Self::MissingBindingTick { tick } => write!(f, "missing observable binding for Q2 tick {}", tick.0),
            Self::UnexpectedBindingTick { tick } => write!(f, "observable binding contains extra tick {}", tick.0),
            Self::ObservableMismatch { lane, expected, actual } => write!(f, "{lane:?} extractor observable {}@{} differs from typed closure observable {}@{}", actual.id(), actual.version(), expected.id(), expected.version()),
            Self::UnitMismatch { lane, expected, actual } => write!(f, "{lane:?} extractor unit {}@{} differs from Q2 unit {}@{}", actual.id(), actual.version(), expected.id(), expected.version()),
            Self::LaneRepresentationMismatch { lane, expected, actual } => write!(f, "{lane:?} observation representation {actual:?} differs from required {expected:?}"),
            Self::ObservationTickMismatch { lane, expected, actual } => write!(f, "{lane:?} observation tick {} differs from Q2 tick {}", actual.0, expected.0),
            Self::ObservationValueMismatch { lane, tick, expected, actual } => write!(f, "{lane:?} observation value {} differs from Q2 value {} at tick {}", actual.0, expected.0, tick.0),
            Self::ObservableAuthorityChanged => write!(f, "exact shadow observable authority changed"),
            Self::ResolvedObservationStale { observation } => write!(f, "resolved shadow observation {}@{} is stale", observation.id(), observation.version()),
            Self::ObservableBoundCertificateStale => write!(f, "observable-bound shadow validation certificate is stale"),
        }
    }
}

impl Error for ShadowObservableAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyIdentity(error) => Some(error),
            Self::InformationRegistry(error) => Some(error),
            Self::ShadowValidation(error) => Some(error),
            _ => None,
        }
    }
}

/// Exact duplicate detection helper for observation bindings assembled by adapters.
pub fn validate_unique_observation_keys(
    pairs: impl IntoIterator<Item = ShadowObservationPair>,
) -> Result<(), ShadowObservableAuthorityError> {
    let mut keys = BTreeSet::new();
    for pair in pairs {
        for key in [pair.coarse(), pair.reference()] {
            if !keys.insert(key) {
                return Err(ShadowObservableAuthorityError::ConflictingObservationRegistration {
                    observation: key,
                });
            }
        }
    }
    Ok(())
}
