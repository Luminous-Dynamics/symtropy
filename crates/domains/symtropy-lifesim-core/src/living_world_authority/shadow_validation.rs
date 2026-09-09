// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Q2 shadow-validation evidence for closure-horizon renewal.
//!
//! This module deliberately stops one capability boundary short of constructing
//! [`ClosureValidationAnchor`]. It recomputes a sealed shadow comparison against
//! the exact typed closure/process contract and emits an opaque
//! [`ShadowAnchorClaim`]. A narrow child bridge inside `closure_usage_authority`
//! can later convert that claim into the parent's private anchor type without
//! making anchor construction public.
//!
//! V0 is conservative by design: evidence is sealed, the current coarse endpoint
//! must match the compared endpoint, the reference representation must carry
//! Exact evidence for the process information, fixed-point relative-error metrics
//! are recomputed deterministically, unsupported metric families fail closed, and
//! validation never mutates canonical ecology.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::{
    CapabilityEvidence, ErrorPpm, EvidenceLineageToken, PPM_SCALE, RepresentationKey,
};
use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
use crate::population::PopulationState;
use crate::population_manifest::{
    ManifestBoundPopulationState, PopulationStateIdentityError, PopulationStateManifest,
};

use super::closure_usage_authority::{
    ClosureUsageAuthorityError, ClosureUsagePolicyAuthorityStamp, ClosureUsagePolicyKey,
    ClosureUsagePolicyRegistry, ClosureValidationAnchorId, ClosureValidationAnchorRevision,
};
use super::spatiotemporal_information::{
    CanonicalTick, SpatiotemporalFailure, SpatiotemporalPolicyError,
    SpatiotemporalPolicyRegistry, SpatiotemporalStateContext,
};
use super::typed_closure_process_acceptance::{
    ResolvedTypedClosureUse, TypedClosureProcessAcceptanceError,
    TypedClosureProcessAcceptanceRegistry,
};
use super::typed_closure_qualification::{
    ClosureAggregationSemantics, ClosureErrorMetric, RelativeZeroReferencePolicy,
    TypedClosureQualification, TypedClosureQualificationError, TypedClosureQualificationRegistry,
};

const MAX_SHADOW_SAMPLES: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowValidationRegistryKey {
    id: u128,
    version: u32,
}
impl ShadowValidationRegistryKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowValidationProfileKey {
    id: u128,
    version: u32,
}
impl ShadowValidationProfileKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowEvidenceKey {
    id: u128,
    version: u32,
}
impl ShadowEvidenceKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowEvidenceRevision(pub u64);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowMetricEvaluatorVersion(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowScalarUnitKey {
    id: u128,
    version: u32,
}
impl ShadowScalarUnitKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

macro_rules! opaque_bytes {
    ($name:ident, $error:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Vec<u8>);
        impl $name {
            pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, ShadowValidationAuthorityError> {
                let bytes = bytes.into();
                if bytes.is_empty() {
                    return Err(ShadowValidationAuthorityError::$error);
                }
                Ok(Self(bytes))
            }
            pub fn as_bytes(&self) -> &[u8] { &self.0 }
        }
    };
}

opaque_bytes!(ShadowImplementationFingerprint, EmptyImplementationFingerprint);
opaque_bytes!(ShadowExecutionCapsuleFingerprint, EmptyExecutionCapsule);
opaque_bytes!(ShadowScenarioFingerprint, EmptyScenarioFingerprint);
opaque_bytes!(ShadowExecutionManifest, EmptyExecutionManifest);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShadowEvidenceTier {
    Q2Differential,
    Q3CausalDifferential,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShadowExecutionStatus {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowValidationProfile {
    key: ShadowValidationProfileKey,
    usage_policy: ClosureUsagePolicyKey,
    reference_representation: RepresentationKey,
    scalar_unit: ShadowScalarUnitKey,
    metric_evaluator_version: ShadowMetricEvaluatorVersion,
    coarse_implementation: ShadowImplementationFingerprint,
    reference_implementation: ShadowImplementationFingerprint,
    execution_capsule: Option<ShadowExecutionCapsuleFingerprint>,
}
impl ShadowValidationProfile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: ShadowValidationProfileKey,
        usage_policy: ClosureUsagePolicyKey,
        reference_representation: RepresentationKey,
        scalar_unit: ShadowScalarUnitKey,
        metric_evaluator_version: ShadowMetricEvaluatorVersion,
        coarse_implementation: ShadowImplementationFingerprint,
        reference_implementation: ShadowImplementationFingerprint,
        execution_capsule: Option<ShadowExecutionCapsuleFingerprint>,
    ) -> Self {
        Self {
            key,
            usage_policy,
            reference_representation,
            scalar_unit,
            metric_evaluator_version,
            coarse_implementation,
            reference_implementation,
            execution_capsule,
        }
    }
    pub const fn key(&self) -> ShadowValidationProfileKey { self.key }
    pub const fn usage_policy(&self) -> ClosureUsagePolicyKey { self.usage_policy }
    pub const fn reference_representation(&self) -> RepresentationKey { self.reference_representation }
    pub const fn scalar_unit(&self) -> ShadowScalarUnitKey { self.scalar_unit }
    pub const fn metric_evaluator_version(&self) -> ShadowMetricEvaluatorVersion { self.metric_evaluator_version }
    pub const fn coarse_implementation(&self) -> &ShadowImplementationFingerprint { &self.coarse_implementation }
    pub const fn reference_implementation(&self) -> &ShadowImplementationFingerprint { &self.reference_implementation }
    pub const fn execution_capsule(&self) -> Option<&ShadowExecutionCapsuleFingerprint> { self.execution_capsule.as_ref() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowValidationWindow {
    start_exclusive: CanonicalTick,
    end_inclusive: CanonicalTick,
}
impl ShadowValidationWindow {
    pub fn new(start_exclusive: CanonicalTick, end_inclusive: CanonicalTick)
        -> Result<Self, ShadowValidationAuthorityError>
    {
        if end_inclusive.0 <= start_exclusive.0 {
            return Err(ShadowValidationAuthorityError::InvalidValidationWindow {
                start_exclusive,
                end_inclusive,
            });
        }
        Ok(Self { start_exclusive, end_inclusive })
    }
    pub const fn start_exclusive(self) -> CanonicalTick { self.start_exclusive }
    pub const fn end_inclusive(self) -> CanonicalTick { self.end_inclusive }
    pub const fn duration_ticks(self) -> u64 { self.end_inclusive.0 - self.start_exclusive.0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowFixedScalar(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowScalarSample {
    tick: CanonicalTick,
    coarse: ShadowFixedScalar,
    reference: ShadowFixedScalar,
}
impl ShadowScalarSample {
    pub const fn new(tick: CanonicalTick, coarse: ShadowFixedScalar, reference: ShadowFixedScalar) -> Self {
        Self { tick, coarse, reference }
    }
    pub const fn tick(self) -> CanonicalTick { self.tick }
    pub const fn coarse(self) -> ShadowFixedScalar { self.coarse }
    pub const fn reference(self) -> ShadowFixedScalar { self.reference }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowScalarTrace {
    unit: ShadowScalarUnitKey,
    samples: BTreeMap<CanonicalTick, ShadowScalarSample>,
}
impl ShadowScalarTrace {
    pub fn new(
        unit: ShadowScalarUnitKey,
        samples: impl IntoIterator<Item = ShadowScalarSample>,
    ) -> Result<Self, ShadowValidationAuthorityError> {
        let mut canonical = BTreeMap::new();
        for sample in samples {
            if canonical.len() >= MAX_SHADOW_SAMPLES {
                return Err(ShadowValidationAuthorityError::TooManyShadowSamples { maximum: MAX_SHADOW_SAMPLES });
            }
            if canonical.insert(sample.tick(), sample).is_some() {
                return Err(ShadowValidationAuthorityError::DuplicateSampleTick { tick: sample.tick() });
            }
        }
        if canonical.is_empty() {
            return Err(ShadowValidationAuthorityError::EmptyShadowTrace);
        }
        Ok(Self { unit, samples: canonical })
    }
    pub const fn unit(&self) -> ShadowScalarUnitKey { self.unit }
    pub fn samples(&self) -> impl Iterator<Item = ShadowScalarSample> + '_ { self.samples.values().copied() }
    pub fn len(&self) -> usize { self.samples.len() }
    pub fn is_empty(&self) -> bool { self.samples.is_empty() }
    fn sample(&self, tick: CanonicalTick) -> Option<ShadowScalarSample> { self.samples.get(&tick).copied() }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowValidationEvidenceRecord {
    key: ShadowEvidenceKey,
    revision: ShadowEvidenceRevision,
    anchor_id: ClosureValidationAnchorId,
    anchor_revision: ClosureValidationAnchorRevision,
    profile: ShadowValidationProfileKey,
    tier: ShadowEvidenceTier,
    status: ShadowExecutionStatus,
    window: ShadowValidationWindow,
    start_population_manifest: PopulationStateManifest,
    coarse_endpoint_manifest: PopulationStateManifest,
    coarse_state_context: SpatiotemporalStateContext,
    reference_state_context: SpatiotemporalStateContext,
    scenario: ShadowScenarioFingerprint,
    execution_manifest: ShadowExecutionManifest,
    evidence_lineage: EvidenceLineageToken,
    trace: Option<ShadowScalarTrace>,
}
impl ShadowValidationEvidenceRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn completed(
        key: ShadowEvidenceKey,
        revision: ShadowEvidenceRevision,
        anchor_id: ClosureValidationAnchorId,
        anchor_revision: ClosureValidationAnchorRevision,
        profile: ShadowValidationProfileKey,
        tier: ShadowEvidenceTier,
        window: ShadowValidationWindow,
        start_population: &PopulationState,
        coarse_endpoint: &PopulationState,
        coarse_state_context: SpatiotemporalStateContext,
        reference_state_context: SpatiotemporalStateContext,
        scenario: ShadowScenarioFingerprint,
        execution_manifest: ShadowExecutionManifest,
        evidence_lineage: EvidenceLineageToken,
        trace: ShadowScalarTrace,
    ) -> Result<Self, ShadowValidationAuthorityError> {
        Ok(Self {
            key,
            revision,
            anchor_id,
            anchor_revision,
            profile,
            tier,
            status: ShadowExecutionStatus::Completed,
            window,
            start_population_manifest: population_manifest(start_population)?,
            coarse_endpoint_manifest: population_manifest(coarse_endpoint)?,
            coarse_state_context,
            reference_state_context,
            scenario,
            execution_manifest,
            evidence_lineage,
            trace: Some(trace),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn incomplete(
        key: ShadowEvidenceKey,
        revision: ShadowEvidenceRevision,
        anchor_id: ClosureValidationAnchorId,
        anchor_revision: ClosureValidationAnchorRevision,
        profile: ShadowValidationProfileKey,
        tier: ShadowEvidenceTier,
        status: ShadowExecutionStatus,
        window: ShadowValidationWindow,
        start_population: &PopulationState,
        coarse_endpoint: &PopulationState,
        coarse_state_context: SpatiotemporalStateContext,
        reference_state_context: SpatiotemporalStateContext,
        scenario: ShadowScenarioFingerprint,
        execution_manifest: ShadowExecutionManifest,
        evidence_lineage: EvidenceLineageToken,
    ) -> Result<Self, ShadowValidationAuthorityError> {
        if status == ShadowExecutionStatus::Completed {
            return Err(ShadowValidationAuthorityError::CompletedEvidenceRequiresTrace);
        }
        Ok(Self {
            key,
            revision,
            anchor_id,
            anchor_revision,
            profile,
            tier,
            status,
            window,
            start_population_manifest: population_manifest(start_population)?,
            coarse_endpoint_manifest: population_manifest(coarse_endpoint)?,
            coarse_state_context,
            reference_state_context,
            scenario,
            execution_manifest,
            evidence_lineage,
            trace: None,
        })
    }

    pub const fn key(&self) -> ShadowEvidenceKey { self.key }
    pub const fn revision(&self) -> ShadowEvidenceRevision { self.revision }
    pub const fn anchor_id(&self) -> ClosureValidationAnchorId { self.anchor_id }
    pub const fn anchor_revision(&self) -> ClosureValidationAnchorRevision { self.anchor_revision }
    pub const fn profile(&self) -> ShadowValidationProfileKey { self.profile }
    pub const fn tier(&self) -> ShadowEvidenceTier { self.tier }
    pub const fn status(&self) -> ShadowExecutionStatus { self.status }
    pub const fn window(&self) -> ShadowValidationWindow { self.window }
    pub const fn start_population_manifest(&self) -> &PopulationStateManifest { &self.start_population_manifest }
    pub const fn coarse_endpoint_manifest(&self) -> &PopulationStateManifest { &self.coarse_endpoint_manifest }
    pub const fn coarse_state_context(&self) -> &SpatiotemporalStateContext { &self.coarse_state_context }
    pub const fn reference_state_context(&self) -> &SpatiotemporalStateContext { &self.reference_state_context }
    pub const fn scenario(&self) -> &ShadowScenarioFingerprint { &self.scenario }
    pub const fn execution_manifest(&self) -> &ShadowExecutionManifest { &self.execution_manifest }
    pub const fn evidence_lineage(&self) -> EvidenceLineageToken { self.evidence_lineage }
    pub const fn trace(&self) -> Option<&ShadowScalarTrace> { self.trace.as_ref() }
}

fn population_manifest(population: &PopulationState)
    -> Result<PopulationStateManifest, ShadowValidationAuthorityError>
{
    ManifestBoundPopulationState::new(population)
        .map(|bound| bound.manifest().clone())
        .map_err(ShadowValidationAuthorityError::PopulationIdentity)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowValidationRegistryBuilder {
    key: ShadowValidationRegistryKey,
    profiles: BTreeMap<ShadowValidationProfileKey, ShadowValidationProfile>,
    evidence: BTreeMap<ShadowEvidenceKey, ShadowValidationEvidenceRecord>,
}
impl ShadowValidationRegistryBuilder {
    pub const fn new(key: ShadowValidationRegistryKey) -> Self {
        Self { key, profiles: BTreeMap::new(), evidence: BTreeMap::new() }
    }
    pub fn register_profile(&mut self, profile: ShadowValidationProfile)
        -> Result<(), ShadowValidationAuthorityError>
    {
        use std::collections::btree_map::Entry;
        match self.profiles.entry(profile.key()) {
            Entry::Vacant(entry) => { entry.insert(profile); Ok(()) }
            Entry::Occupied(entry) if entry.get() == &profile => Ok(()),
            Entry::Occupied(entry) => Err(ShadowValidationAuthorityError::ConflictingProfileRegistration { profile: *entry.key() }),
        }
    }
    pub fn register_evidence(&mut self, evidence: ShadowValidationEvidenceRecord)
        -> Result<(), ShadowValidationAuthorityError>
    {
        use std::collections::btree_map::Entry;
        match self.evidence.entry(evidence.key()) {
            Entry::Vacant(entry) => { entry.insert(evidence); Ok(()) }
            Entry::Occupied(entry) if entry.get() == &evidence => Ok(()),
            Entry::Occupied(entry) => Err(ShadowValidationAuthorityError::ConflictingEvidenceRegistration { evidence: *entry.key() }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        self,
        usage: &ClosureUsagePolicyRegistry,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<ShadowValidationRegistry, ShadowValidationAuthorityError> {
        usage.validate_current(information_policy, closures, acceptances, spatiotemporal)
            .map_err(ShadowValidationAuthorityError::ClosureUsage)?;
        for profile in self.profiles.values() {
            validate_profile(profile, usage, information_policy)?;
        }
        let mut anchors = BTreeSet::new();
        for evidence in self.evidence.values() {
            let profile = self.profiles.get(&evidence.profile())
                .ok_or(ShadowValidationAuthorityError::UnknownProfile { profile: evidence.profile() })?;
            validate_evidence_structure(evidence, profile, usage)?;
            let identity = (evidence.anchor_id(), evidence.anchor_revision());
            if !anchors.insert(identity) {
                return Err(ShadowValidationAuthorityError::DuplicateAnchorIdentity {
                    anchor: evidence.anchor_id(), revision: evidence.anchor_revision(),
                });
            }
        }
        let authority = ShadowValidationAuthorityStamp {
            key: self.key,
            usage_policy_authority: usage.authority_stamp().clone(),
            profiles: self.profiles.clone(),
            evidence: self.evidence.clone(),
        };
        Ok(ShadowValidationRegistry { authority, profiles: self.profiles, evidence: self.evidence })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowValidationAuthorityStamp {
    key: ShadowValidationRegistryKey,
    usage_policy_authority: ClosureUsagePolicyAuthorityStamp,
    profiles: BTreeMap<ShadowValidationProfileKey, ShadowValidationProfile>,
    evidence: BTreeMap<ShadowEvidenceKey, ShadowValidationEvidenceRecord>,
}
impl ShadowValidationAuthorityStamp {
    pub const fn key(&self) -> ShadowValidationRegistryKey { self.key }
    pub const fn usage_policy_authority(&self) -> &ClosureUsagePolicyAuthorityStamp { &self.usage_policy_authority }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowValidationRegistry {
    authority: ShadowValidationAuthorityStamp,
    profiles: BTreeMap<ShadowValidationProfileKey, ShadowValidationProfile>,
    evidence: BTreeMap<ShadowEvidenceKey, ShadowValidationEvidenceRecord>,
}
impl ShadowValidationRegistry {
    pub const fn authority_stamp(&self) -> &ShadowValidationAuthorityStamp { &self.authority }
    pub fn profile(&self, key: ShadowValidationProfileKey) -> Option<&ShadowValidationProfile> { self.profiles.get(&key) }
    pub fn evidence(&self, key: ShadowEvidenceKey) -> Option<&ShadowValidationEvidenceRecord> { self.evidence.get(&key) }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        usage: &ClosureUsagePolicyRegistry,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<(), ShadowValidationAuthorityError> {
        usage.validate_current(information_policy, closures, acceptances, spatiotemporal)
            .map_err(ShadowValidationAuthorityError::ClosureUsage)?;
        if usage.authority_stamp() != &self.authority.usage_policy_authority {
            return Err(ShadowValidationAuthorityError::UsagePolicyAuthorityChanged);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn certify(
        &self,
        evidence_key: ShadowEvidenceKey,
        usage: &ClosureUsagePolicyRegistry,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        current_coarse_population: &PopulationState,
    ) -> Result<ShadowValidationCertificate, ShadowValidationAuthorityError> {
        self.validate_current(usage, information_policy, closures, acceptances, spatiotemporal)?;
        let evidence = self.evidence.get(&evidence_key)
            .ok_or(ShadowValidationAuthorityError::UnknownEvidence { evidence: evidence_key })?;
        if evidence.status() != ShadowExecutionStatus::Completed {
            return Err(ShadowValidationAuthorityError::ExecutionNotCompleted {
                evidence: evidence_key, status: evidence.status(),
            });
        }
        if population_manifest(current_coarse_population)? != *evidence.coarse_endpoint_manifest() {
            return Err(ShadowValidationAuthorityError::CanonicalEndpointChanged { evidence: evidence_key });
        }
        let profile = self.profiles.get(&evidence.profile())
            .ok_or(ShadowValidationAuthorityError::UnknownProfile { profile: evidence.profile() })?;
        validate_evidence_structure(evidence, profile, usage)?;
        let definition = usage.definition(profile.usage_policy())
            .ok_or(ShadowValidationAuthorityError::UnknownUsagePolicy { policy: profile.usage_policy() })?;

        let resolved_use = acceptances.match_use(
            information_policy,
            closures,
            spatiotemporal,
            definition.process(),
            definition.information(),
            definition.lineage(),
            definition.representation(),
            evidence.window().end_inclusive(),
            evidence.coarse_state_context(),
        ).map_err(ShadowValidationAuthorityError::ClosureAcceptance)?;

        validate_reference_authority(
            profile,
            evidence,
            definition.process(),
            information_policy,
            spatiotemporal,
        )?;

        let qualification = resolved_use.qualification().qualification();
        if evidence.window().duration_ticks() != qualification.horizon().get() {
            return Err(ShadowValidationAuthorityError::ValidationHorizonMismatch {
                evidence: evidence_key,
                actual: evidence.window().duration_ticks(),
                required: qualification.horizon().get(),
            });
        }
        let trace = evidence.trace().ok_or(ShadowValidationAuthorityError::CompletedEvidenceRequiresTrace)?;
        let evaluation = evaluate_trace(qualification, evidence.window(), trace)?;
        let accepted = stricter_bound(
            qualification.max_error_ppm(),
            resolved_use.acceptance().maximum_error_ppm(),
        );
        let report = ShadowValidationReport {
            evidence: evidence.key(),
            revision: evidence.revision(),
            usage_policy: profile.usage_policy(),
            reference_representation: profile.reference_representation(),
            metric: qualification.metric(),
            aggregation: qualification.aggregation(),
            window: evidence.window(),
            observed_ppm: evaluation.observed_ppm,
            accepted_ppm: accepted,
            samples: evaluation.samples,
        };
        if !report.is_satisfied() {
            return Err(ShadowValidationAuthorityError::ValidationBoundExceeded(report));
        }
        let anchor_claim = ShadowAnchorClaim {
            id: evidence.anchor_id(),
            revision: evidence.anchor_revision(),
            tick: evidence.window().end_inclusive(),
            usage_policy_authority: usage.authority_stamp().clone(),
            usage_policy: profile.usage_policy(),
            shadow_authority: self.authority.clone(),
            evidence: evidence.key(),
            evidence_revision: evidence.revision(),
        };
        Ok(ShadowValidationCertificate {
            authority: self.authority.clone(),
            evidence: evidence.clone(),
            profile: profile.clone(),
            resolved_use,
            report,
            anchor_claim,
        })
    }
}

fn validate_profile(
    profile: &ShadowValidationProfile,
    usage: &ClosureUsagePolicyRegistry,
    information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
) -> Result<(), ShadowValidationAuthorityError> {
    let definition = usage.definition(profile.usage_policy())
        .ok_or(ShadowValidationAuthorityError::UnknownUsagePolicy { policy: profile.usage_policy() })?;
    let reference = information_policy.registry().resolve_representation(profile.reference_representation())
        .map_err(ShadowValidationAuthorityError::InformationRegistry)?;
    let exact = reference.capabilities().claims().iter().any(|(information, evidence)| {
        information.covers(definition.information()) && evidence.contains(&CapabilityEvidence::Exact)
    });
    if !exact {
        return Err(ShadowValidationAuthorityError::ReferenceRepresentationNotExact {
            representation: profile.reference_representation(), information: definition.information(),
        });
    }
    Ok(())
}

fn validate_evidence_structure(
    evidence: &ShadowValidationEvidenceRecord,
    profile: &ShadowValidationProfile,
    usage: &ClosureUsagePolicyRegistry,
) -> Result<(), ShadowValidationAuthorityError> {
    if evidence.profile() != profile.key() {
        return Err(ShadowValidationAuthorityError::EvidenceProfileMismatch);
    }
    let definition = usage.definition(profile.usage_policy())
        .ok_or(ShadowValidationAuthorityError::UnknownUsagePolicy { policy: profile.usage_policy() })?;
    if evidence.coarse_state_context().representation() != definition.representation() {
        return Err(ShadowValidationAuthorityError::CoarseStateRepresentationMismatch {
            expected: definition.representation(), actual: evidence.coarse_state_context().representation(),
        });
    }
    if evidence.reference_state_context().representation() != profile.reference_representation() {
        return Err(ShadowValidationAuthorityError::ReferenceStateRepresentationMismatch {
            expected: profile.reference_representation(), actual: evidence.reference_state_context().representation(),
        });
    }
    if evidence.status() == ShadowExecutionStatus::Completed {
        let trace = evidence.trace().ok_or(ShadowValidationAuthorityError::CompletedEvidenceRequiresTrace)?;
        if trace.unit() != profile.scalar_unit() {
            return Err(ShadowValidationAuthorityError::TraceUnitMismatch {
                expected: profile.scalar_unit(), actual: trace.unit(),
            });
        }
    }
    Ok(())
}

fn validate_reference_authority(
    profile: &ShadowValidationProfile,
    evidence: &ShadowValidationEvidenceRecord,
    process: crate::information::ProcessKey,
    information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
    spatiotemporal: &SpatiotemporalPolicyRegistry,
) -> Result<(), ShadowValidationAuthorityError> {
    let information_report = information_policy.registry()
        .evaluate_registered([process], profile.reference_representation())
        .map_err(ShadowValidationAuthorityError::InformationRegistry)?;
    if !information_report.is_sufficient() {
        return Err(ShadowValidationAuthorityError::ReferenceInformationInsufficient);
    }
    let report = spatiotemporal.evaluate(
        information_policy,
        [process],
        profile.reference_representation(),
        evidence.window().end_inclusive(),
        evidence.reference_state_context(),
    ).map_err(ShadowValidationAuthorityError::SpatiotemporalPolicy)?;
    if !report.is_sufficient() {
        return Err(ShadowValidationAuthorityError::ReferenceSpatiotemporalInsufficient {
            failures: report.failures().to_vec(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowObservedPpm(pub u128);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowValidationReport {
    evidence: ShadowEvidenceKey,
    revision: ShadowEvidenceRevision,
    usage_policy: ClosureUsagePolicyKey,
    reference_representation: RepresentationKey,
    metric: ClosureErrorMetric,
    aggregation: ClosureAggregationSemantics,
    window: ShadowValidationWindow,
    observed_ppm: ShadowObservedPpm,
    accepted_ppm: ErrorPpm,
    samples: usize,
}
impl ShadowValidationReport {
    pub const fn evidence(&self) -> ShadowEvidenceKey { self.evidence }
    pub const fn revision(&self) -> ShadowEvidenceRevision { self.revision }
    pub const fn usage_policy(&self) -> ClosureUsagePolicyKey { self.usage_policy }
    pub const fn reference_representation(&self) -> RepresentationKey { self.reference_representation }
    pub const fn metric(&self) -> ClosureErrorMetric { self.metric }
    pub const fn aggregation(&self) -> ClosureAggregationSemantics { self.aggregation }
    pub const fn window(&self) -> ShadowValidationWindow { self.window }
    pub const fn observed_ppm(&self) -> ShadowObservedPpm { self.observed_ppm }
    pub const fn accepted_ppm(&self) -> ErrorPpm { self.accepted_ppm }
    pub const fn samples(&self) -> usize { self.samples }
    pub fn is_satisfied(&self) -> bool { self.observed_ppm.0 <= u128::from(self.accepted_ppm.get()) }
}

/// Opaque anchor precursor. No public constructor exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowAnchorClaim {
    id: ClosureValidationAnchorId,
    revision: ClosureValidationAnchorRevision,
    tick: CanonicalTick,
    usage_policy_authority: ClosureUsagePolicyAuthorityStamp,
    usage_policy: ClosureUsagePolicyKey,
    shadow_authority: ShadowValidationAuthorityStamp,
    evidence: ShadowEvidenceKey,
    evidence_revision: ShadowEvidenceRevision,
}
impl ShadowAnchorClaim {
    pub const fn id(&self) -> ClosureValidationAnchorId { self.id }
    pub const fn revision(&self) -> ClosureValidationAnchorRevision { self.revision }
    pub const fn tick(&self) -> CanonicalTick { self.tick }
    pub const fn usage_policy_authority(&self) -> &ClosureUsagePolicyAuthorityStamp { &self.usage_policy_authority }
    pub const fn usage_policy(&self) -> ClosureUsagePolicyKey { self.usage_policy }
    pub const fn shadow_authority(&self) -> &ShadowValidationAuthorityStamp { &self.shadow_authority }
    pub const fn evidence(&self) -> ShadowEvidenceKey { self.evidence }
    pub const fn evidence_revision(&self) -> ShadowEvidenceRevision { self.evidence_revision }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowValidationCertificate {
    authority: ShadowValidationAuthorityStamp,
    evidence: ShadowValidationEvidenceRecord,
    profile: ShadowValidationProfile,
    resolved_use: ResolvedTypedClosureUse,
    report: ShadowValidationReport,
    anchor_claim: ShadowAnchorClaim,
}
impl ShadowValidationCertificate {
    pub const fn authority_stamp(&self) -> &ShadowValidationAuthorityStamp { &self.authority }
    pub const fn evidence(&self) -> &ShadowValidationEvidenceRecord { &self.evidence }
    pub const fn profile(&self) -> &ShadowValidationProfile { &self.profile }
    pub const fn resolved_use(&self) -> &ResolvedTypedClosureUse { &self.resolved_use }
    pub const fn report(&self) -> &ShadowValidationReport { &self.report }
    pub const fn anchor_claim(&self) -> &ShadowAnchorClaim { &self.anchor_claim }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        registry: &ShadowValidationRegistry,
        usage: &ClosureUsagePolicyRegistry,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        current_coarse_population: &PopulationState,
    ) -> Result<(), ShadowValidationAuthorityError> {
        if registry.authority_stamp() != &self.authority {
            return Err(ShadowValidationAuthorityError::ShadowAuthorityChanged);
        }
        let current = registry.certify(
            self.evidence.key(), usage, information_policy, closures, acceptances,
            spatiotemporal, current_coarse_population,
        )?;
        if current != *self {
            return Err(ShadowValidationAuthorityError::CertificateStale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MetricEvaluation { observed_ppm: ShadowObservedPpm, samples: usize }

fn evaluate_trace(
    qualification: &TypedClosureQualification,
    window: ShadowValidationWindow,
    trace: &ShadowScalarTrace,
) -> Result<MetricEvaluation, ShadowValidationAuthorityError> {
    match (qualification.metric(), qualification.aggregation()) {
        (ClosureErrorMetric::MeanRelativePpm { zero_reference }, ClosureAggregationSemantics::MeanOverHorizon) => {
            validate_full_tick_coverage(window, trace)?;
            let mut total = 0u128;
            for sample in trace.samples() {
                total = total.checked_add(relative_ppm_ceil(sample, zero_reference)?)
                    .ok_or(ShadowValidationAuthorityError::MetricArithmeticOverflow)?;
            }
            Ok(MetricEvaluation {
                observed_ppm: ShadowObservedPpm(ceil_div(total, trace.len() as u128)?),
                samples: trace.len(),
            })
        }
        (ClosureErrorMetric::MaximumRelativePpm { zero_reference }, ClosureAggregationSemantics::MaximumOverHorizon | ClosureAggregationSemantics::PerStep) => {
            validate_full_tick_coverage(window, trace)?;
            let mut maximum = 0u128;
            for sample in trace.samples() {
                maximum = maximum.max(relative_ppm_ceil(sample, zero_reference)?);
            }
            Ok(MetricEvaluation { observed_ppm: ShadowObservedPpm(maximum), samples: trace.len() })
        }
        (ClosureErrorMetric::MeanRelativePpm { zero_reference } | ClosureErrorMetric::MaximumRelativePpm { zero_reference }, ClosureAggregationSemantics::TerminalState) => {
            if trace.len() != 1 {
                return Err(ShadowValidationAuthorityError::TerminalTraceMustHaveOneSample { actual: trace.len() });
            }
            let sample = trace.sample(window.end_inclusive())
                .ok_or(ShadowValidationAuthorityError::TerminalSampleMissing { tick: window.end_inclusive() })?;
            Ok(MetricEvaluation { observed_ppm: ShadowObservedPpm(relative_ppm_ceil(sample, zero_reference)?), samples: 1 })
        }
        (metric, aggregation) => Err(ShadowValidationAuthorityError::UnsupportedMetricContract { metric, aggregation }),
    }
}

fn validate_full_tick_coverage(window: ShadowValidationWindow, trace: &ShadowScalarTrace)
    -> Result<(), ShadowValidationAuthorityError>
{
    let expected = usize::try_from(window.duration_ticks())
        .map_err(|_| ShadowValidationAuthorityError::ValidationWindowTooLarge { ticks: window.duration_ticks() })?;
    if expected > MAX_SHADOW_SAMPLES {
        return Err(ShadowValidationAuthorityError::TooManyShadowSamples { maximum: MAX_SHADOW_SAMPLES });
    }
    if trace.len() != expected {
        return Err(ShadowValidationAuthorityError::IncompleteTraceCoverage { expected, actual: trace.len() });
    }
    let mut tick = window.start_exclusive().0.checked_add(1)
        .ok_or(ShadowValidationAuthorityError::TickOverflow)?;
    for sample in trace.samples() {
        if sample.tick().0 != tick {
            return Err(ShadowValidationAuthorityError::TraceCoverageGap {
                expected: CanonicalTick(tick), actual: sample.tick(),
            });
        }
        tick = tick.checked_add(1).ok_or(ShadowValidationAuthorityError::TickOverflow)?;
    }
    Ok(())
}

fn relative_ppm_ceil(sample: ShadowScalarSample, zero_reference: RelativeZeroReferencePolicy)
    -> Result<u128, ShadowValidationAuthorityError>
{
    let coarse = i128::from(sample.coarse().0);
    let reference = i128::from(sample.reference().0);
    let denominator = reference.unsigned_abs();
    if denominator == 0 {
        return Err(ShadowValidationAuthorityError::ZeroReferenceValue { tick: sample.tick(), policy: zero_reference });
    }
    let numerator = (coarse - reference).unsigned_abs()
        .checked_mul(u128::from(PPM_SCALE))
        .ok_or(ShadowValidationAuthorityError::MetricArithmeticOverflow)?;
    ceil_div(numerator, denominator)
}

fn ceil_div(numerator: u128, denominator: u128) -> Result<u128, ShadowValidationAuthorityError> {
    if denominator == 0 { return Err(ShadowValidationAuthorityError::MetricArithmeticOverflow); }
    let q = numerator / denominator;
    if numerator % denominator == 0 { Ok(q) } else { q.checked_add(1).ok_or(ShadowValidationAuthorityError::MetricArithmeticOverflow) }
}

fn stricter_bound(left: ErrorPpm, right: ErrorPpm) -> ErrorPpm {
    if left <= right { left } else { right }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShadowValidationAuthorityError {
    EmptyImplementationFingerprint,
    EmptyExecutionCapsule,
    EmptyScenarioFingerprint,
    EmptyExecutionManifest,
    InvalidValidationWindow { start_exclusive: CanonicalTick, end_inclusive: CanonicalTick },
    EmptyShadowTrace,
    TooManyShadowSamples { maximum: usize },
    DuplicateSampleTick { tick: CanonicalTick },
    CompletedEvidenceRequiresTrace,
    PopulationIdentity(PopulationStateIdentityError),
    ConflictingProfileRegistration { profile: ShadowValidationProfileKey },
    ConflictingEvidenceRegistration { evidence: ShadowEvidenceKey },
    UnknownProfile { profile: ShadowValidationProfileKey },
    UnknownEvidence { evidence: ShadowEvidenceKey },
    UnknownUsagePolicy { policy: ClosureUsagePolicyKey },
    DuplicateAnchorIdentity { anchor: ClosureValidationAnchorId, revision: ClosureValidationAnchorRevision },
    ClosureUsage(ClosureUsageAuthorityError),
    UsagePolicyAuthorityChanged,
    InformationRegistry(crate::information_registry::InformationRegistryError),
    ReferenceRepresentationNotExact { representation: RepresentationKey, information: crate::information::EcologicalInformation },
    EvidenceProfileMismatch,
    CoarseStateRepresentationMismatch { expected: RepresentationKey, actual: RepresentationKey },
    ReferenceStateRepresentationMismatch { expected: RepresentationKey, actual: RepresentationKey },
    TraceUnitMismatch { expected: ShadowScalarUnitKey, actual: ShadowScalarUnitKey },
    ExecutionNotCompleted { evidence: ShadowEvidenceKey, status: ShadowExecutionStatus },
    CanonicalEndpointChanged { evidence: ShadowEvidenceKey },
    ClosureAcceptance(TypedClosureProcessAcceptanceError),
    ClosureQualification(TypedClosureQualificationError),
    SpatiotemporalPolicy(SpatiotemporalPolicyError),
    ReferenceInformationInsufficient,
    ReferenceSpatiotemporalInsufficient { failures: Vec<SpatiotemporalFailure> },
    ValidationHorizonMismatch { evidence: ShadowEvidenceKey, actual: u64, required: u64 },
    UnsupportedMetricContract { metric: ClosureErrorMetric, aggregation: ClosureAggregationSemantics },
    ValidationWindowTooLarge { ticks: u64 },
    IncompleteTraceCoverage { expected: usize, actual: usize },
    TraceCoverageGap { expected: CanonicalTick, actual: CanonicalTick },
    TerminalTraceMustHaveOneSample { actual: usize },
    TerminalSampleMissing { tick: CanonicalTick },
    ZeroReferenceValue { tick: CanonicalTick, policy: RelativeZeroReferencePolicy },
    MetricArithmeticOverflow,
    TickOverflow,
    ValidationBoundExceeded(ShadowValidationReport),
    ShadowAuthorityChanged,
    CertificateStale,
}

impl fmt::Display for ShadowValidationAuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyImplementationFingerprint => write!(f, "shadow implementation fingerprint is empty"),
            Self::EmptyExecutionCapsule => write!(f, "shadow execution capsule is empty"),
            Self::EmptyScenarioFingerprint => write!(f, "shadow scenario fingerprint is empty"),
            Self::EmptyExecutionManifest => write!(f, "shadow execution manifest is empty"),
            Self::InvalidValidationWindow { start_exclusive, end_inclusive } => write!(f, "invalid shadow validation window ({}, {}]", start_exclusive.0, end_inclusive.0),
            Self::EmptyShadowTrace => write!(f, "shadow validation trace is empty"),
            Self::TooManyShadowSamples { maximum } => write!(f, "shadow trace exceeds {maximum} samples"),
            Self::DuplicateSampleTick { tick } => write!(f, "duplicate shadow sample at tick {}", tick.0),
            Self::CompletedEvidenceRequiresTrace => write!(f, "completed shadow evidence requires a comparison trace"),
            Self::PopulationIdentity(error) => write!(f, "population identity error: {error}"),
            Self::ConflictingProfileRegistration { profile } => write!(f, "conflicting shadow profile {}@{}", profile.id(), profile.version()),
            Self::ConflictingEvidenceRegistration { evidence } => write!(f, "conflicting shadow evidence {}@{}", evidence.id(), evidence.version()),
            Self::UnknownProfile { profile } => write!(f, "unknown shadow profile {}@{}", profile.id(), profile.version()),
            Self::UnknownEvidence { evidence } => write!(f, "unknown shadow evidence {}@{}", evidence.id(), evidence.version()),
            Self::UnknownUsagePolicy { policy } => write!(f, "unknown closure usage policy {}@{}", policy.id(), policy.version()),
            Self::DuplicateAnchorIdentity { anchor, revision } => write!(f, "duplicate shadow anchor identity {} revision {}", anchor.0, revision.0),
            Self::ClosureUsage(error) => write!(f, "closure usage error: {error}"),
            Self::UsagePolicyAuthorityChanged => write!(f, "exact closure usage policy authority changed"),
            Self::InformationRegistry(error) => write!(f, "information registry error: {error}"),
            Self::ReferenceRepresentationNotExact { representation, information } => write!(f, "reference representation {representation:?} lacks Exact evidence for {information:?}"),
            Self::EvidenceProfileMismatch => write!(f, "shadow evidence/profile mismatch"),
            Self::CoarseStateRepresentationMismatch { expected, actual } => write!(f, "coarse state context is {actual:?}, expected {expected:?}"),
            Self::ReferenceStateRepresentationMismatch { expected, actual } => write!(f, "reference state context is {actual:?}, expected {expected:?}"),
            Self::TraceUnitMismatch { expected, actual } => write!(f, "shadow trace unit {}@{} differs from profile unit {}@{}", actual.id(), actual.version(), expected.id(), expected.version()),
            Self::ExecutionNotCompleted { evidence, status } => write!(f, "shadow evidence {}@{} is {status:?}, not Completed", evidence.id(), evidence.version()),
            Self::CanonicalEndpointChanged { evidence } => write!(f, "canonical coarse endpoint changed since shadow evidence {}@{}", evidence.id(), evidence.version()),
            Self::ClosureAcceptance(error) => write!(f, "typed closure acceptance error: {error}"),
            Self::ClosureQualification(error) => write!(f, "typed closure qualification error: {error}"),
            Self::SpatiotemporalPolicy(error) => write!(f, "spatiotemporal policy error: {error}"),
            Self::ReferenceInformationInsufficient => write!(f, "reference representation is process-information insufficient"),
            Self::ReferenceSpatiotemporalInsufficient { failures } => write!(f, "reference representation has {} spatiotemporal failure(s)", failures.len()),
            Self::ValidationHorizonMismatch { evidence, actual, required } => write!(f, "shadow evidence {}@{} covers {actual} ticks, required {required}", evidence.id(), evidence.version()),
            Self::UnsupportedMetricContract { metric, aggregation } => write!(f, "shadow V0 does not implement {metric:?}/{aggregation:?}"),
            Self::ValidationWindowTooLarge { ticks } => write!(f, "shadow validation window {ticks} is too large for this host"),
            Self::IncompleteTraceCoverage { expected, actual } => write!(f, "shadow trace has {actual} samples, expected {expected}"),
            Self::TraceCoverageGap { expected, actual } => write!(f, "shadow trace expected tick {}, found {}", expected.0, actual.0),
            Self::TerminalTraceMustHaveOneSample { actual } => write!(f, "terminal-state shadow trace needs one sample, found {actual}"),
            Self::TerminalSampleMissing { tick } => write!(f, "terminal shadow sample missing at tick {}", tick.0),
            Self::ZeroReferenceValue { tick, policy } => write!(f, "relative shadow metric encountered zero reference at tick {} under {policy:?}", tick.0),
            Self::MetricArithmeticOverflow => write!(f, "shadow metric fixed-point arithmetic overflow"),
            Self::TickOverflow => write!(f, "canonical tick overflow during shadow validation"),
            Self::ValidationBoundExceeded(report) => write!(f, "shadow validation observed {} ppm, exceeding {} ppm", report.observed_ppm().0, report.accepted_ppm().get()),
            Self::ShadowAuthorityChanged => write!(f, "exact shadow validation authority changed"),
            Self::CertificateStale => write!(f, "shadow validation certificate is stale"),
        }
    }
}

impl Error for ShadowValidationAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PopulationIdentity(error) => Some(error),
            Self::ClosureUsage(error) => Some(error),
            Self::InformationRegistry(error) => Some(error),
            Self::ClosureAcceptance(error) => Some(error),
            Self::ClosureQualification(error) => Some(error),
            Self::SpatiotemporalPolicy(error) => Some(error),
            _ => None,
        }
    }
}
