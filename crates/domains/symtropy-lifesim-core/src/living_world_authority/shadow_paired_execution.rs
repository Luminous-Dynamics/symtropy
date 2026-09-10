// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Paired coarse/reference execution authority for Q2 shadow validation.
//!
//! #563 authenticates the richer reference execution lineage. This module closes
//! the symmetric coarse-lane gap and only produces a paired certificate when both
//! execution histories resolve to the same exact Q2 experiment and T0 source
//! context. It remains evidence-only: no ecological state or closure anchor is
//! mutated here.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::applicability_policy_manifest::ManifestBoundTransitionApplicabilityPolicy;
use crate::candidate_evidence_obligations::CandidateCertificationRequestSet;
use crate::information::{EvidenceLineageToken, RepresentationKey};
use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
use crate::population::PopulationState;

use super::closure_usage_authority::{
    ClosureUsagePolicyAuthorityStamp, ClosureUsagePolicyRegistry,
};
use super::retained_authority::RetainedAuthorityRegistry;
use super::shadow_execution_lineage::{
    ShadowExecutionLineageError, ShadowReferenceExecutionAuthorityStamp,
    ShadowReferenceExecutionCertificate, ShadowReferenceExecutionRegistry,
};
use super::shadow_observable_authority::{
    ShadowObservableAuthorityRegistry, ShadowObservableAuthorityStamp, ShadowObservationKey,
    ShadowObservationRevision, ShadowObservationSourceIdentity,
};
use super::shadow_validation::{
    ShadowEvidenceKey, ShadowEvidenceRevision, ShadowExecutionCapsuleFingerprint,
    ShadowExecutionManifest, ShadowImplementationFingerprint, ShadowScenarioFingerprint,
    ShadowValidationAuthorityStamp, ShadowValidationRegistry, ShadowValidationWindow,
};
use super::spatiotemporal_information::{CanonicalTick, SpatiotemporalPolicyRegistry};
use super::transition_domain::TransitionDomainEvaluationSubject;
use super::typed_closure_process_acceptance::TypedClosureProcessAcceptanceRegistry;
use super::typed_closure_qualification::TypedClosureQualificationRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowCoarseRunnerRegistryKey {
    id: u128,
    version: u32,
}
impl ShadowCoarseRunnerRegistryKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowCoarseRunnerKey {
    id: u128,
    version: u32,
}
impl ShadowCoarseRunnerKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowCoarseRunnerProfileVersion(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShadowCoarseRunnerStatus {
    Qualified,
    Revoked,
    Superseded,
}

/// Bootstrap/evidence qualification for one deterministic coarse shadow runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowCoarseRunnerQualification {
    runner: ShadowCoarseRunnerKey,
    profile: ShadowCoarseRunnerProfileVersion,
    representation: RepresentationKey,
    implementation: ShadowImplementationFingerprint,
    execution_capsule: Option<ShadowExecutionCapsuleFingerprint>,
    qualification_evidence: EvidenceLineageToken,
    status: ShadowCoarseRunnerStatus,
}
impl ShadowCoarseRunnerQualification {
    pub fn new(
        runner: ShadowCoarseRunnerKey,
        profile: ShadowCoarseRunnerProfileVersion,
        representation: RepresentationKey,
        implementation: ShadowImplementationFingerprint,
        execution_capsule: Option<ShadowExecutionCapsuleFingerprint>,
        qualification_evidence: EvidenceLineageToken,
        status: ShadowCoarseRunnerStatus,
    ) -> Self {
        Self {
            runner,
            profile,
            representation,
            implementation,
            execution_capsule,
            qualification_evidence,
            status,
        }
    }
    pub const fn runner(&self) -> ShadowCoarseRunnerKey { self.runner }
    pub const fn profile(&self) -> ShadowCoarseRunnerProfileVersion { self.profile }
    pub const fn representation(&self) -> RepresentationKey { self.representation }
    pub const fn implementation(&self) -> &ShadowImplementationFingerprint { &self.implementation }
    pub const fn execution_capsule(&self) -> Option<&ShadowExecutionCapsuleFingerprint> { self.execution_capsule.as_ref() }
    pub const fn qualification_evidence(&self) -> EvidenceLineageToken { self.qualification_evidence }
    pub const fn status(&self) -> ShadowCoarseRunnerStatus { self.status }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowCoarseRunId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowCoarseRunRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShadowCoarseRunStatus {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowCoarseRunObservedState {
    observation: ShadowObservationKey,
    observation_revision: ShadowObservationRevision,
    source: ShadowObservationSourceIdentity,
}
impl ShadowCoarseRunObservedState {
    pub fn new(
        observation: ShadowObservationKey,
        observation_revision: ShadowObservationRevision,
        source: ShadowObservationSourceIdentity,
    ) -> Self {
        Self { observation, observation_revision, source }
    }
    pub const fn observation(&self) -> ShadowObservationKey { self.observation }
    pub const fn observation_revision(&self) -> ShadowObservationRevision { self.observation_revision }
    pub const fn source(&self) -> &ShadowObservationSourceIdentity { &self.source }
}

/// Immutable evidence for one coarse shadow execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowCoarseRunRecord {
    id: ShadowCoarseRunId,
    revision: ShadowCoarseRunRevision,
    runner: ShadowCoarseRunnerKey,
    shadow_evidence: ShadowEvidenceKey,
    shadow_evidence_revision: ShadowEvidenceRevision,
    window: ShadowValidationWindow,
    scenario: ShadowScenarioFingerprint,
    execution_manifest: ShadowExecutionManifest,
    start: TransitionDomainEvaluationSubject,
    observed_states: BTreeMap<CanonicalTick, ShadowCoarseRunObservedState>,
    evidence_lineage: EvidenceLineageToken,
    status: ShadowCoarseRunStatus,
}
impl ShadowCoarseRunRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ShadowCoarseRunId,
        revision: ShadowCoarseRunRevision,
        runner: ShadowCoarseRunnerKey,
        shadow_evidence: ShadowEvidenceKey,
        shadow_evidence_revision: ShadowEvidenceRevision,
        window: ShadowValidationWindow,
        scenario: ShadowScenarioFingerprint,
        execution_manifest: ShadowExecutionManifest,
        start: TransitionDomainEvaluationSubject,
        observed_states: impl IntoIterator<Item = (CanonicalTick, ShadowCoarseRunObservedState)>,
        evidence_lineage: EvidenceLineageToken,
        status: ShadowCoarseRunStatus,
    ) -> Result<Self, PairedShadowExecutionError> {
        let mut canonical = BTreeMap::new();
        for (tick, state) in observed_states {
            if tick.0 <= window.start_exclusive().0 || tick.0 > window.end_inclusive().0 {
                return Err(PairedShadowExecutionError::ObservedTickOutsideWindow { tick, window });
            }
            if canonical.insert(tick, state).is_some() {
                return Err(PairedShadowExecutionError::DuplicateObservedTick { tick });
            }
        }
        if status == ShadowCoarseRunStatus::Completed && canonical.is_empty() {
            return Err(PairedShadowExecutionError::CompletedRunHasNoObservedStates { run: id });
        }
        Ok(Self {
            id,
            revision,
            runner,
            shadow_evidence,
            shadow_evidence_revision,
            window,
            scenario,
            execution_manifest,
            start,
            observed_states: canonical,
            evidence_lineage,
            status,
        })
    }
    pub const fn id(&self) -> ShadowCoarseRunId { self.id }
    pub const fn revision(&self) -> ShadowCoarseRunRevision { self.revision }
    pub const fn runner(&self) -> ShadowCoarseRunnerKey { self.runner }
    pub const fn shadow_evidence(&self) -> ShadowEvidenceKey { self.shadow_evidence }
    pub const fn shadow_evidence_revision(&self) -> ShadowEvidenceRevision { self.shadow_evidence_revision }
    pub const fn window(&self) -> ShadowValidationWindow { self.window }
    pub const fn scenario(&self) -> &ShadowScenarioFingerprint { &self.scenario }
    pub const fn execution_manifest(&self) -> &ShadowExecutionManifest { &self.execution_manifest }
    pub const fn start(&self) -> &TransitionDomainEvaluationSubject { &self.start }
    pub fn observed_states(&self) -> &BTreeMap<CanonicalTick, ShadowCoarseRunObservedState> { &self.observed_states }
    pub const fn evidence_lineage(&self) -> EvidenceLineageToken { self.evidence_lineage }
    pub const fn status(&self) -> ShadowCoarseRunStatus { self.status }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedShadowExecutionAuthorityStamp {
    key: ShadowCoarseRunnerRegistryKey,
    shadow_authority: ShadowValidationAuthorityStamp,
    observable_authority: ShadowObservableAuthorityStamp,
    usage_authority: ClosureUsagePolicyAuthorityStamp,
    reference_execution_authority: ShadowReferenceExecutionAuthorityStamp,
    runners: BTreeMap<ShadowCoarseRunnerKey, ShadowCoarseRunnerQualification>,
    runs: BTreeMap<ShadowCoarseRunId, ShadowCoarseRunRecord>,
}
impl PairedShadowExecutionAuthorityStamp {
    pub const fn key(&self) -> ShadowCoarseRunnerRegistryKey { self.key }
    pub const fn shadow_authority(&self) -> &ShadowValidationAuthorityStamp { &self.shadow_authority }
    pub const fn observable_authority(&self) -> &ShadowObservableAuthorityStamp { &self.observable_authority }
    pub const fn usage_authority(&self) -> &ClosureUsagePolicyAuthorityStamp { &self.usage_authority }
    pub const fn reference_execution_authority(&self) -> &ShadowReferenceExecutionAuthorityStamp { &self.reference_execution_authority }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedShadowExecutionRegistryBuilder {
    key: ShadowCoarseRunnerRegistryKey,
    runners: BTreeMap<ShadowCoarseRunnerKey, ShadowCoarseRunnerQualification>,
    runs: BTreeMap<ShadowCoarseRunId, ShadowCoarseRunRecord>,
}
impl PairedShadowExecutionRegistryBuilder {
    pub const fn new(key: ShadowCoarseRunnerRegistryKey) -> Self {
        Self { key, runners: BTreeMap::new(), runs: BTreeMap::new() }
    }

    pub fn register_runner(
        &mut self,
        qualification: ShadowCoarseRunnerQualification,
    ) -> Result<(), PairedShadowExecutionError> {
        use std::collections::btree_map::Entry;
        match self.runners.entry(qualification.runner()) {
            Entry::Vacant(entry) => { entry.insert(qualification); Ok(()) }
            Entry::Occupied(entry) if entry.get() == &qualification => Ok(()),
            Entry::Occupied(entry) => Err(PairedShadowExecutionError::ConflictingRunnerRegistration {
                runner: *entry.key(),
            }),
        }
    }

    pub fn register_run(&mut self, run: ShadowCoarseRunRecord) -> Result<(), PairedShadowExecutionError> {
        use std::collections::btree_map::Entry;
        match self.runs.entry(run.id()) {
            Entry::Vacant(entry) => { entry.insert(run); Ok(()) }
            Entry::Occupied(entry) if entry.get() == &run => Ok(()),
            Entry::Occupied(entry) => Err(PairedShadowExecutionError::ConflictingRunRegistration {
                run: *entry.key(),
            }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        self,
        shadow: &ShadowValidationRegistry,
        observable: &ShadowObservableAuthorityRegistry,
        usage: &ClosureUsagePolicyRegistry,
        reference_execution: &ShadowReferenceExecutionRegistry,
    ) -> Result<PairedShadowExecutionRegistry, PairedShadowExecutionError> {
        let mut evidence_bindings = BTreeSet::new();
        for run in self.runs.values() {
            let runner = self.runners.get(&run.runner())
                .ok_or(PairedShadowExecutionError::RunWithoutRunner {
                    run: run.id(), runner: run.runner(),
                })?;
            let evidence = shadow.evidence(run.shadow_evidence())
                .ok_or(PairedShadowExecutionError::UnknownShadowEvidence {
                    evidence: run.shadow_evidence(),
                })?;
            if evidence.revision() != run.shadow_evidence_revision() {
                return Err(PairedShadowExecutionError::ShadowEvidenceRevisionMismatch {
                    evidence: run.shadow_evidence(),
                    expected: evidence.revision(),
                    actual: run.shadow_evidence_revision(),
                });
            }
            let profile = shadow.profile(evidence.profile())
                .ok_or(PairedShadowExecutionError::UnknownShadowProfile)?;
            let usage_definition = usage.definition(profile.usage_policy())
                .ok_or(PairedShadowExecutionError::UnknownUsagePolicy)?;
            if runner.representation() != usage_definition.representation() {
                return Err(PairedShadowExecutionError::RunnerRepresentationMismatch {
                    expected: usage_definition.representation(),
                    actual: runner.representation(),
                });
            }
            if runner.implementation() != profile.coarse_implementation() {
                return Err(PairedShadowExecutionError::RunnerImplementationMismatch);
            }
            if runner.execution_capsule() != profile.execution_capsule() {
                return Err(PairedShadowExecutionError::RunnerExecutionCapsuleMismatch);
            }
            if run.window() != evidence.window() {
                return Err(PairedShadowExecutionError::WindowMismatch {
                    expected: evidence.window(), actual: run.window(),
                });
            }
            if run.scenario() != evidence.scenario() {
                return Err(PairedShadowExecutionError::ScenarioMismatch);
            }
            if run.execution_manifest() != evidence.execution_manifest() {
                return Err(PairedShadowExecutionError::ExecutionManifestMismatch);
            }
            let pair = (run.shadow_evidence(), run.shadow_evidence_revision());
            if !evidence_bindings.insert(pair) {
                return Err(PairedShadowExecutionError::DuplicateShadowEvidenceRunBinding {
                    evidence: run.shadow_evidence(),
                    revision: run.shadow_evidence_revision(),
                });
            }
        }
        let authority = PairedShadowExecutionAuthorityStamp {
            key: self.key,
            shadow_authority: shadow.authority_stamp().clone(),
            observable_authority: observable.authority_stamp().clone(),
            usage_authority: usage.authority_stamp().clone(),
            reference_execution_authority: reference_execution.authority_stamp().clone(),
            runners: self.runners.clone(),
            runs: self.runs.clone(),
        };
        Ok(PairedShadowExecutionRegistry { authority, runners: self.runners, runs: self.runs })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedShadowExecutionRegistry {
    authority: PairedShadowExecutionAuthorityStamp,
    runners: BTreeMap<ShadowCoarseRunnerKey, ShadowCoarseRunnerQualification>,
    runs: BTreeMap<ShadowCoarseRunId, ShadowCoarseRunRecord>,
}
impl PairedShadowExecutionRegistry {
    pub const fn authority_stamp(&self) -> &PairedShadowExecutionAuthorityStamp { &self.authority }

    #[allow(clippy::too_many_arguments)]
    pub fn certify_pair(
        &self,
        coarse_run_id: ShadowCoarseRunId,
        reference: &ShadowReferenceExecutionCertificate,
        reference_registry: &ShadowReferenceExecutionRegistry,
        requests: &CandidateCertificationRequestSet,
        retained: &RetainedAuthorityRegistry,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        observable_registry: &ShadowObservableAuthorityRegistry,
        shadow_registry: &ShadowValidationRegistry,
        usage: &ClosureUsagePolicyRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        start_population: &PopulationState,
        current_coarse_population: &PopulationState,
    ) -> Result<PairedShadowExecutionCertificate, PairedShadowExecutionError> {
        if shadow_registry.authority_stamp() != self.authority.shadow_authority() {
            return Err(PairedShadowExecutionError::ShadowAuthorityChanged);
        }
        if observable_registry.authority_stamp() != self.authority.observable_authority() {
            return Err(PairedShadowExecutionError::ObservableAuthorityChanged);
        }
        if usage.authority_stamp() != self.authority.usage_authority() {
            return Err(PairedShadowExecutionError::UsageAuthorityChanged);
        }
        if reference_registry.authority_stamp() != self.authority.reference_execution_authority() {
            return Err(PairedShadowExecutionError::ReferenceExecutionAuthorityChanged);
        }

        reference.validate_current(
            reference_registry,
            requests,
            retained,
            applicability,
            observable_registry,
            shadow_registry,
            usage,
            policy,
            closures,
            acceptances,
            spatiotemporal,
            start_population,
            current_coarse_population,
        ).map_err(PairedShadowExecutionError::ReferenceExecution)?;

        let run = self.runs.get(&coarse_run_id)
            .ok_or(PairedShadowExecutionError::UnknownRun { run: coarse_run_id })?;
        if run.status() != ShadowCoarseRunStatus::Completed {
            return Err(PairedShadowExecutionError::RunNotCompleted {
                run: coarse_run_id, status: run.status(),
            });
        }
        let runner = self.runners.get(&run.runner())
            .ok_or(PairedShadowExecutionError::UnknownRunner { runner: run.runner() })?;
        if runner.status() != ShadowCoarseRunnerStatus::Qualified {
            return Err(PairedShadowExecutionError::RunnerNotQualified {
                runner: runner.runner(), status: runner.status(),
            });
        }

        let retained_start = reference.retained_start();
        let bound = retained_start.observable_bound_shadow();
        let shadow = bound.shadow();
        if run.shadow_evidence() != shadow.evidence().key()
            || run.shadow_evidence_revision() != shadow.evidence().revision()
        {
            return Err(PairedShadowExecutionError::ShadowEvidenceMismatch);
        }
        if run.window() != shadow.evidence().window()
            || run.window() != reference.run().window()
        {
            return Err(PairedShadowExecutionError::PairedWindowMismatch);
        }
        if run.scenario() != shadow.evidence().scenario()
            || run.scenario() != reference.run().scenario()
        {
            return Err(PairedShadowExecutionError::PairedScenarioMismatch);
        }
        if run.execution_manifest() != shadow.evidence().execution_manifest()
            || run.execution_manifest() != reference.run().execution_manifest()
        {
            return Err(PairedShadowExecutionError::PairedExecutionManifestMismatch);
        }
        if run.start() != retained_start.source_subject() {
            return Err(PairedShadowExecutionError::CoarseStartMismatch);
        }
        if run.start().population_manifest() != shadow.evidence().start_population_manifest() {
            return Err(PairedShadowExecutionError::CoarseStartPopulationMismatch);
        }
        if runner.representation() != retained_start.coarse_representation() {
            return Err(PairedShadowExecutionError::RunnerRepresentationMismatch {
                expected: retained_start.coarse_representation(),
                actual: runner.representation(),
            });
        }
        if runner.implementation() != shadow.profile().coarse_implementation() {
            return Err(PairedShadowExecutionError::RunnerImplementationMismatch);
        }
        if runner.execution_capsule() != shadow.profile().execution_capsule() {
            return Err(PairedShadowExecutionError::RunnerExecutionCapsuleMismatch);
        }

        let expected_count = bound.bindings().count();
        if run.observed_states().len() != expected_count {
            return Err(PairedShadowExecutionError::ObservedStateCardinalityMismatch {
                expected: expected_count,
                actual: run.observed_states().len(),
            });
        }
        for (tick, binding) in bound.bindings() {
            let coarse = binding.coarse().observation();
            let state = run.observed_states().get(tick)
                .ok_or(PairedShadowExecutionError::MissingObservedTick { tick: *tick })?;
            if state.observation() != coarse.key()
                || state.observation_revision() != coarse.revision()
                || state.source() != coarse.source()
            {
                return Err(PairedShadowExecutionError::ObservedStateMismatch { tick: *tick });
            }
            if state.source().representation() != retained_start.coarse_representation() {
                return Err(PairedShadowExecutionError::ObservedRepresentationMismatch {
                    tick: *tick,
                    expected: retained_start.coarse_representation(),
                    actual: state.source().representation(),
                });
            }
        }

        Ok(PairedShadowExecutionCertificate {
            authority: self.authority.clone(),
            coarse_run: run.clone(),
            coarse_runner: runner.clone(),
            reference: reference.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairedShadowExecutionCertificate {
    authority: PairedShadowExecutionAuthorityStamp,
    coarse_run: ShadowCoarseRunRecord,
    coarse_runner: ShadowCoarseRunnerQualification,
    reference: ShadowReferenceExecutionCertificate,
}
impl PairedShadowExecutionCertificate {
    pub const fn authority_stamp(&self) -> &PairedShadowExecutionAuthorityStamp { &self.authority }
    pub const fn coarse_run(&self) -> &ShadowCoarseRunRecord { &self.coarse_run }
    pub const fn coarse_runner(&self) -> &ShadowCoarseRunnerQualification { &self.coarse_runner }
    pub const fn reference(&self) -> &ShadowReferenceExecutionCertificate { &self.reference }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        registry: &PairedShadowExecutionRegistry,
        reference_registry: &ShadowReferenceExecutionRegistry,
        requests: &CandidateCertificationRequestSet,
        retained: &RetainedAuthorityRegistry,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        observable_registry: &ShadowObservableAuthorityRegistry,
        shadow_registry: &ShadowValidationRegistry,
        usage: &ClosureUsagePolicyRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        start_population: &PopulationState,
        current_coarse_population: &PopulationState,
    ) -> Result<(), PairedShadowExecutionError> {
        if registry.authority_stamp() != &self.authority {
            return Err(PairedShadowExecutionError::PairedAuthorityChanged);
        }
        let current = registry.certify_pair(
            self.coarse_run.id(),
            &self.reference,
            reference_registry,
            requests,
            retained,
            applicability,
            observable_registry,
            shadow_registry,
            usage,
            policy,
            closures,
            acceptances,
            spatiotemporal,
            start_population,
            current_coarse_population,
        )?;
        if current != *self {
            return Err(PairedShadowExecutionError::CertificateStale);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum PairedShadowExecutionError {
    ObservedTickOutsideWindow { tick: CanonicalTick, window: ShadowValidationWindow },
    DuplicateObservedTick { tick: CanonicalTick },
    CompletedRunHasNoObservedStates { run: ShadowCoarseRunId },
    ConflictingRunnerRegistration { runner: ShadowCoarseRunnerKey },
    ConflictingRunRegistration { run: ShadowCoarseRunId },
    RunWithoutRunner { run: ShadowCoarseRunId, runner: ShadowCoarseRunnerKey },
    UnknownShadowEvidence { evidence: ShadowEvidenceKey },
    ShadowEvidenceRevisionMismatch { evidence: ShadowEvidenceKey, expected: ShadowEvidenceRevision, actual: ShadowEvidenceRevision },
    UnknownShadowProfile,
    UnknownUsagePolicy,
    DuplicateShadowEvidenceRunBinding { evidence: ShadowEvidenceKey, revision: ShadowEvidenceRevision },
    UnknownRun { run: ShadowCoarseRunId },
    UnknownRunner { runner: ShadowCoarseRunnerKey },
    RunNotCompleted { run: ShadowCoarseRunId, status: ShadowCoarseRunStatus },
    RunnerNotQualified { runner: ShadowCoarseRunnerKey, status: ShadowCoarseRunnerStatus },
    RunnerRepresentationMismatch { expected: RepresentationKey, actual: RepresentationKey },
    RunnerImplementationMismatch,
    RunnerExecutionCapsuleMismatch,
    WindowMismatch { expected: ShadowValidationWindow, actual: ShadowValidationWindow },
    ScenarioMismatch,
    ExecutionManifestMismatch,
    ShadowAuthorityChanged,
    ObservableAuthorityChanged,
    UsageAuthorityChanged,
    ReferenceExecutionAuthorityChanged,
    ReferenceExecution(ShadowExecutionLineageError),
    ShadowEvidenceMismatch,
    PairedWindowMismatch,
    PairedScenarioMismatch,
    PairedExecutionManifestMismatch,
    CoarseStartMismatch,
    CoarseStartPopulationMismatch,
    ObservedStateCardinalityMismatch { expected: usize, actual: usize },
    MissingObservedTick { tick: CanonicalTick },
    ObservedStateMismatch { tick: CanonicalTick },
    ObservedRepresentationMismatch { tick: CanonicalTick, expected: RepresentationKey, actual: RepresentationKey },
    PairedAuthorityChanged,
    CertificateStale,
}

impl fmt::Display for PairedShadowExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObservedTickOutsideWindow { tick, window } => write!(f, "coarse shadow state tick {} lies outside ({}, {}]", tick.0, window.start_exclusive().0, window.end_inclusive().0),
            Self::DuplicateObservedTick { tick } => write!(f, "duplicate coarse shadow state at tick {}", tick.0),
            Self::CompletedRunHasNoObservedStates { run } => write!(f, "completed coarse shadow run {} has no observed states", run.0),
            Self::ConflictingRunnerRegistration { runner } => write!(f, "conflicting coarse shadow runner {}@{}", runner.id(), runner.version()),
            Self::ConflictingRunRegistration { run } => write!(f, "conflicting coarse shadow run {}", run.0),
            Self::RunWithoutRunner { run, runner } => write!(f, "coarse shadow run {} references unknown runner {}@{}", run.0, runner.id(), runner.version()),
            Self::UnknownShadowEvidence { evidence } => write!(f, "coarse shadow run references unknown Q2 evidence {}@{}", evidence.id(), evidence.version()),
            Self::ShadowEvidenceRevisionMismatch { evidence, expected, actual } => write!(f, "coarse shadow run references Q2 evidence {}@{} revision {}, current record is revision {}", evidence.id(), evidence.version(), actual.0, expected.0),
            Self::UnknownShadowProfile => write!(f, "Q2 evidence references unknown shadow profile"),
            Self::UnknownUsagePolicy => write!(f, "Q2 profile references unknown closure usage policy"),
            Self::DuplicateShadowEvidenceRunBinding { evidence, revision } => write!(f, "Q2 evidence {}@{} revision {} is bound to multiple coarse runs", evidence.id(), evidence.version(), revision.0),
            Self::UnknownRun { run } => write!(f, "unknown coarse shadow run {}", run.0),
            Self::UnknownRunner { runner } => write!(f, "unknown coarse shadow runner {}@{}", runner.id(), runner.version()),
            Self::RunNotCompleted { run, status } => write!(f, "coarse shadow run {} is {status:?}, not Completed", run.0),
            Self::RunnerNotQualified { runner, status } => write!(f, "coarse shadow runner {}@{} is {status:?}, not Qualified", runner.id(), runner.version()),
            Self::RunnerRepresentationMismatch { expected, actual } => write!(f, "coarse runner representation {actual:?} differs from required {expected:?}"),
            Self::RunnerImplementationMismatch => write!(f, "coarse runner implementation differs from Q2 coarse implementation"),
            Self::RunnerExecutionCapsuleMismatch => write!(f, "coarse runner execution capsule differs from Q2 profile"),
            Self::WindowMismatch { expected, actual } => write!(f, "coarse run window ({}, {}] differs from Q2 ({}, {}]", actual.start_exclusive().0, actual.end_inclusive().0, expected.start_exclusive().0, expected.end_inclusive().0),
            Self::ScenarioMismatch => write!(f, "coarse run scenario differs from Q2 evidence"),
            Self::ExecutionManifestMismatch => write!(f, "coarse run execution manifest differs from Q2 evidence"),
            Self::ShadowAuthorityChanged => write!(f, "exact Q2 shadow authority changed after paired registry sealing"),
            Self::ObservableAuthorityChanged => write!(f, "exact observable authority changed after paired registry sealing"),
            Self::UsageAuthorityChanged => write!(f, "exact closure usage authority changed after paired registry sealing"),
            Self::ReferenceExecutionAuthorityChanged => write!(f, "exact reference execution authority changed after paired registry sealing"),
            Self::ReferenceExecution(error) => write!(f, "reference execution authority: {error}"),
            Self::ShadowEvidenceMismatch => write!(f, "coarse and reference execution certificates name different Q2 evidence"),
            Self::PairedWindowMismatch => write!(f, "coarse/reference runs do not share the exact Q2 validation window"),
            Self::PairedScenarioMismatch => write!(f, "coarse/reference runs do not share the exact Q2 scenario"),
            Self::PairedExecutionManifestMismatch => write!(f, "coarse/reference runs do not share the exact Q2 execution manifest"),
            Self::CoarseStartMismatch => write!(f, "coarse run was not initialized from the exact #553 T0 source subject"),
            Self::CoarseStartPopulationMismatch => write!(f, "coarse run T0 population differs from #393 start population"),
            Self::ObservedStateCardinalityMismatch { expected, actual } => write!(f, "coarse run binds {actual} sampled states, Q2 requires {expected}"),
            Self::MissingObservedTick { tick } => write!(f, "coarse run lacks state at required Q2 tick {}", tick.0),
            Self::ObservedStateMismatch { tick } => write!(f, "coarse run state differs from #406 coarse observation at tick {}", tick.0),
            Self::ObservedRepresentationMismatch { tick, expected, actual } => write!(f, "coarse run state at tick {} uses {actual:?}, expected {expected:?}", tick.0),
            Self::PairedAuthorityChanged => write!(f, "exact paired shadow execution authority changed"),
            Self::CertificateStale => write!(f, "paired shadow execution certificate is stale"),
        }
    }
}

impl Error for PairedShadowExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReferenceExecution(error) => Some(error),
            _ => None,
        }
    }
}
