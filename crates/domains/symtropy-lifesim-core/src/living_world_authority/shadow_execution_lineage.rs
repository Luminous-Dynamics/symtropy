// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Execution-lineage authority for Q2 shadow reference trajectories.
//!
//! #393 proves typed comparison arithmetic, #406 binds every compared scalar to
//! qualified observable extraction, and #553 proves one narrow R0 exact richer
//! state was available at T0. None of those facts alone proves that the reference
//! observations actually descend from that exact start through one execution.
//!
//! This module adds that independent proof. It remains read-only evidence
//! authority: it does not mutate ecology, correct the coarse trajectory, or mint
//! a closure validation anchor.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::applicability_policy_manifest::ManifestBoundTransitionApplicabilityPolicy;
use crate::candidate_evidence_obligations::CandidateCertificationRequestSet;
use crate::information::{EvidenceLineageToken, RepresentationKey};
use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
use crate::population::PopulationState;

use super::closure_usage_authority::ClosureUsagePolicyRegistry;
use super::retained_authority::{RetainedAuthorityRegistry, RetainedStoreRevision};
use super::shadow_common_start::{RetainedShadowStartCertificate, RetainedShadowStartError};
use super::shadow_observable_authority::{
    ShadowObservableAuthorityRegistry, ShadowObservableAuthorityError, ShadowObservationKey,
    ShadowObservationRevision, ShadowObservationSourceIdentity,
};
use super::shadow_validation::{
    ShadowEvidenceKey, ShadowEvidenceRevision, ShadowExecutionCapsuleFingerprint,
    ShadowExecutionManifest, ShadowImplementationFingerprint, ShadowScenarioFingerprint,
    ShadowValidationRegistry, ShadowValidationWindow,
};
use super::spatiotemporal_information::{CanonicalTick, SpatiotemporalPolicyRegistry};
use super::typed_closure_process_acceptance::TypedClosureProcessAcceptanceRegistry;
use super::typed_closure_qualification::TypedClosureQualificationRegistry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowReferenceRunnerRegistryKey {
    id: u128,
    version: u32,
}
impl ShadowReferenceRunnerRegistryKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowReferenceRunnerKey {
    id: u128,
    version: u32,
}
impl ShadowReferenceRunnerKey {
    pub const fn new(id: u128, version: u32) -> Self { Self { id, version } }
    pub const fn id(self) -> u128 { self.id }
    pub const fn version(self) -> u32 { self.version }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowReferenceRunnerProfileVersion(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShadowReferenceRunnerStatus {
    Qualified,
    Revoked,
    Superseded,
}

/// Qualification for one deterministic reference runner implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowReferenceRunnerQualification {
    runner: ShadowReferenceRunnerKey,
    profile: ShadowReferenceRunnerProfileVersion,
    reference_representation: RepresentationKey,
    implementation: ShadowImplementationFingerprint,
    execution_capsule: Option<ShadowExecutionCapsuleFingerprint>,
    qualification_evidence: EvidenceLineageToken,
    status: ShadowReferenceRunnerStatus,
}
impl ShadowReferenceRunnerQualification {
    pub fn new(
        runner: ShadowReferenceRunnerKey,
        profile: ShadowReferenceRunnerProfileVersion,
        reference_representation: RepresentationKey,
        implementation: ShadowImplementationFingerprint,
        execution_capsule: Option<ShadowExecutionCapsuleFingerprint>,
        qualification_evidence: EvidenceLineageToken,
        status: ShadowReferenceRunnerStatus,
    ) -> Self {
        Self {
            runner,
            profile,
            reference_representation,
            implementation,
            execution_capsule,
            qualification_evidence,
            status,
        }
    }
    pub const fn runner(&self) -> ShadowReferenceRunnerKey { self.runner }
    pub const fn profile(&self) -> ShadowReferenceRunnerProfileVersion { self.profile }
    pub const fn reference_representation(&self) -> RepresentationKey { self.reference_representation }
    pub const fn implementation(&self) -> &ShadowImplementationFingerprint { &self.implementation }
    pub const fn execution_capsule(&self) -> Option<&ShadowExecutionCapsuleFingerprint> { self.execution_capsule.as_ref() }
    pub const fn qualification_evidence(&self) -> EvidenceLineageToken { self.qualification_evidence }
    pub const fn status(&self) -> ShadowReferenceRunnerStatus { self.status }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowReferenceRunId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowReferenceRunRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShadowReferenceRunStatus {
    Completed,
    Failed,
    Cancelled,
}

/// Runner-owned statement identifying the exact retained state used to initialize
/// the reference lane. Certification checks this statement against #553 rather
/// than accepting representation capability as initialization proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowReferenceRetainedStartBinding {
    authority: crate::information_transition::RetainedAuthorityKey,
    retained_revision: RetainedStoreRevision,
    retained_content: super::retained_authority::RetainedContentManifest,
}
impl ShadowReferenceRetainedStartBinding {
    pub fn new(
        authority: crate::information_transition::RetainedAuthorityKey,
        retained_revision: RetainedStoreRevision,
        retained_content: super::retained_authority::RetainedContentManifest,
    ) -> Self {
        Self { authority, retained_revision, retained_content }
    }
    pub const fn authority(&self) -> crate::information_transition::RetainedAuthorityKey { self.authority }
    pub const fn retained_revision(&self) -> RetainedStoreRevision { self.retained_revision }
    pub const fn retained_content(&self) -> &super::retained_authority::RetainedContentManifest { &self.retained_content }
}

/// One reference sample-state identity claimed by the execution lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowReferenceRunObservedState {
    observation: ShadowObservationKey,
    observation_revision: ShadowObservationRevision,
    source: ShadowObservationSourceIdentity,
}
impl ShadowReferenceRunObservedState {
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

/// Immutable execution evidence for one uninterrupted reference run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowReferenceRunRecord {
    id: ShadowReferenceRunId,
    revision: ShadowReferenceRunRevision,
    runner: ShadowReferenceRunnerKey,
    shadow_evidence: ShadowEvidenceKey,
    shadow_evidence_revision: ShadowEvidenceRevision,
    window: ShadowValidationWindow,
    scenario: ShadowScenarioFingerprint,
    execution_manifest: ShadowExecutionManifest,
    start: ShadowReferenceRetainedStartBinding,
    observed_states: BTreeMap<CanonicalTick, ShadowReferenceRunObservedState>,
    evidence_lineage: EvidenceLineageToken,
    status: ShadowReferenceRunStatus,
}
impl ShadowReferenceRunRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ShadowReferenceRunId,
        revision: ShadowReferenceRunRevision,
        runner: ShadowReferenceRunnerKey,
        shadow_evidence: ShadowEvidenceKey,
        shadow_evidence_revision: ShadowEvidenceRevision,
        window: ShadowValidationWindow,
        scenario: ShadowScenarioFingerprint,
        execution_manifest: ShadowExecutionManifest,
        start: ShadowReferenceRetainedStartBinding,
        observed_states: impl IntoIterator<Item = (CanonicalTick, ShadowReferenceRunObservedState)>,
        evidence_lineage: EvidenceLineageToken,
        status: ShadowReferenceRunStatus,
    ) -> Result<Self, ShadowExecutionLineageError> {
        let mut canonical = BTreeMap::new();
        for (tick, state) in observed_states {
            if tick != state.source().source_revision_tick_hint().unwrap_or(tick) {
                // Source revisions are not time and must not be interpreted as time.
                // This branch is intentionally unreachable with the current source
                // identity API; exact tick binding is instead carried by the map key
                // and validated against the #406 observation below.
            }
            if canonical.insert(tick, state).is_some() {
                return Err(ShadowExecutionLineageError::DuplicateObservedTick { tick });
            }
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
    pub const fn id(&self) -> ShadowReferenceRunId { self.id }
    pub const fn revision(&self) -> ShadowReferenceRunRevision { self.revision }
    pub const fn runner(&self) -> ShadowReferenceRunnerKey { self.runner }
    pub const fn shadow_evidence(&self) -> ShadowEvidenceKey { self.shadow_evidence }
    pub const fn shadow_evidence_revision(&self) -> ShadowEvidenceRevision { self.shadow_evidence_revision }
    pub const fn window(&self) -> ShadowValidationWindow { self.window }
    pub const fn scenario(&self) -> &ShadowScenarioFingerprint { &self.scenario }
    pub const fn execution_manifest(&self) -> &ShadowExecutionManifest { &self.execution_manifest }
    pub const fn start(&self) -> &ShadowReferenceRetainedStartBinding { &self.start }
    pub fn observed_states(&self) -> &BTreeMap<CanonicalTick, ShadowReferenceRunObservedState> { &self.observed_states }
    pub const fn evidence_lineage(&self) -> EvidenceLineageToken { self.evidence_lineage }
    pub const fn status(&self) -> ShadowReferenceRunStatus { self.status }
}

/// Exact in-process authority identity for one immutable runner/run corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowReferenceExecutionAuthorityStamp {
    key: ShadowReferenceRunnerRegistryKey,
    runners: BTreeMap<ShadowReferenceRunnerKey, ShadowReferenceRunnerQualification>,
    runs: BTreeMap<ShadowReferenceRunId, ShadowReferenceRunRecord>,
}
impl ShadowReferenceExecutionAuthorityStamp {
    pub const fn key(&self) -> ShadowReferenceRunnerRegistryKey { self.key }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowReferenceExecutionRegistryBuilder {
    key: ShadowReferenceRunnerRegistryKey,
    runners: BTreeMap<ShadowReferenceRunnerKey, ShadowReferenceRunnerQualification>,
    runs: BTreeMap<ShadowReferenceRunId, ShadowReferenceRunRecord>,
}
impl ShadowReferenceExecutionRegistryBuilder {
    pub const fn new(key: ShadowReferenceRunnerRegistryKey) -> Self {
        Self { key, runners: BTreeMap::new(), runs: BTreeMap::new() }
    }

    pub fn register_runner(
        &mut self,
        qualification: ShadowReferenceRunnerQualification,
    ) -> Result<(), ShadowExecutionLineageError> {
        use std::collections::btree_map::Entry;
        match self.runners.entry(qualification.runner()) {
            Entry::Vacant(entry) => { entry.insert(qualification); Ok(()) }
            Entry::Occupied(entry) if entry.get() == &qualification => Ok(()),
            Entry::Occupied(entry) => Err(ShadowExecutionLineageError::ConflictingRunnerRegistration {
                runner: *entry.key(),
            }),
        }
    }

    pub fn register_run(&mut self, run: ShadowReferenceRunRecord) -> Result<(), ShadowExecutionLineageError> {
        use std::collections::btree_map::Entry;
        match self.runs.entry(run.id()) {
            Entry::Vacant(entry) => { entry.insert(run); Ok(()) }
            Entry::Occupied(entry) if entry.get() == &run => Ok(()),
            Entry::Occupied(entry) => Err(ShadowExecutionLineageError::ConflictingRunRegistration {
                run: *entry.key(),
            }),
        }
    }

    pub fn seal(self) -> Result<ShadowReferenceExecutionRegistry, ShadowExecutionLineageError> {
        let mut evidence_bindings = BTreeSet::new();
        for run in self.runs.values() {
            if !self.runners.contains_key(&run.runner()) {
                return Err(ShadowExecutionLineageError::RunWithoutQualifiedRunner { run: run.id(), runner: run.runner() });
            }
            let pair = (run.shadow_evidence(), run.shadow_evidence_revision());
            if !evidence_bindings.insert(pair) {
                return Err(ShadowExecutionLineageError::DuplicateShadowEvidenceRunBinding {
                    evidence: run.shadow_evidence(),
                    revision: run.shadow_evidence_revision(),
                });
            }
        }
        let authority = ShadowReferenceExecutionAuthorityStamp {
            key: self.key,
            runners: self.runners.clone(),
            runs: self.runs.clone(),
        };
        Ok(ShadowReferenceExecutionRegistry { authority, runners: self.runners, runs: self.runs })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowReferenceExecutionRegistry {
    authority: ShadowReferenceExecutionAuthorityStamp,
    runners: BTreeMap<ShadowReferenceRunnerKey, ShadowReferenceRunnerQualification>,
    runs: BTreeMap<ShadowReferenceRunId, ShadowReferenceRunRecord>,
}
impl ShadowReferenceExecutionRegistry {
    pub const fn authority_stamp(&self) -> &ShadowReferenceExecutionAuthorityStamp { &self.authority }

    #[allow(clippy::too_many_arguments)]
    pub fn certify_retained_run(
        &self,
        run_id: ShadowReferenceRunId,
        retained_start: &RetainedShadowStartCertificate,
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
    ) -> Result<ShadowReferenceExecutionCertificate, ShadowExecutionLineageError> {
        retained_start.validate_current(
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
        ).map_err(ShadowExecutionLineageError::RetainedStart)?;

        let run = self.runs.get(&run_id)
            .ok_or(ShadowExecutionLineageError::UnknownRun { run: run_id })?;
        if run.status() != ShadowReferenceRunStatus::Completed {
            return Err(ShadowExecutionLineageError::RunNotCompleted { run: run_id, status: run.status() });
        }
        let runner = self.runners.get(&run.runner())
            .ok_or(ShadowExecutionLineageError::UnknownRunner { runner: run.runner() })?;
        if runner.status() != ShadowReferenceRunnerStatus::Qualified {
            return Err(ShadowExecutionLineageError::RunnerNotQualified { runner: runner.runner(), status: runner.status() });
        }

        let bound = retained_start.observable_bound_shadow();
        let shadow = bound.shadow();
        if run.shadow_evidence() != shadow.evidence().key()
            || run.shadow_evidence_revision() != shadow.evidence().revision()
        {
            return Err(ShadowExecutionLineageError::ShadowEvidenceMismatch);
        }
        if run.window() != shadow.evidence().window() {
            return Err(ShadowExecutionLineageError::WindowMismatch {
                expected: shadow.evidence().window(), actual: run.window(),
            });
        }
        if run.scenario() != shadow.evidence().scenario() {
            return Err(ShadowExecutionLineageError::ScenarioMismatch);
        }
        if run.execution_manifest() != shadow.evidence().execution_manifest() {
            return Err(ShadowExecutionLineageError::ExecutionManifestMismatch);
        }
        if runner.reference_representation() != retained_start.reference_representation() {
            return Err(ShadowExecutionLineageError::RunnerRepresentationMismatch {
                expected: retained_start.reference_representation(), actual: runner.reference_representation(),
            });
        }
        if runner.implementation() != shadow.profile().reference_implementation() {
            return Err(ShadowExecutionLineageError::RunnerImplementationMismatch);
        }
        if runner.execution_capsule() != shadow.profile().execution_capsule() {
            return Err(ShadowExecutionLineageError::RunnerExecutionCapsuleMismatch);
        }

        let exact_start = retained_start.retained_start().record();
        if run.start().authority() != exact_start.authority()
            || run.start().retained_revision() != exact_start.revision()
            || run.start().retained_content() != exact_start.content_manifest()
        {
            return Err(ShadowExecutionLineageError::RetainedStartBindingMismatch);
        }

        if run.observed_states().len() != bound.bindings().count() {
            return Err(ShadowExecutionLineageError::ObservedStateCardinalityMismatch {
                expected: bound.bindings().count(), actual: run.observed_states().len(),
            });
        }
        for (tick, binding) in bound.bindings() {
            let reference = binding.reference().observation();
            let state = run.observed_states().get(tick)
                .ok_or(ShadowExecutionLineageError::MissingObservedTick { tick: *tick })?;
            if state.observation() != reference.key()
                || state.observation_revision() != reference.revision()
                || state.source() != reference.source()
            {
                return Err(ShadowExecutionLineageError::ObservedStateMismatch { tick: *tick });
            }
            if state.source().representation() != retained_start.reference_representation() {
                return Err(ShadowExecutionLineageError::ObservedRepresentationMismatch {
                    tick: *tick,
                    expected: retained_start.reference_representation(),
                    actual: state.source().representation(),
                });
            }
        }

        Ok(ShadowReferenceExecutionCertificate {
            authority: self.authority.clone(),
            run: run.clone(),
            runner: runner.clone(),
            retained_start: retained_start.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowReferenceExecutionCertificate {
    authority: ShadowReferenceExecutionAuthorityStamp,
    run: ShadowReferenceRunRecord,
    runner: ShadowReferenceRunnerQualification,
    retained_start: RetainedShadowStartCertificate,
}
impl ShadowReferenceExecutionCertificate {
    pub const fn authority_stamp(&self) -> &ShadowReferenceExecutionAuthorityStamp { &self.authority }
    pub const fn run(&self) -> &ShadowReferenceRunRecord { &self.run }
    pub const fn runner(&self) -> &ShadowReferenceRunnerQualification { &self.runner }
    pub const fn retained_start(&self) -> &RetainedShadowStartCertificate { &self.retained_start }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        registry: &ShadowReferenceExecutionRegistry,
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
    ) -> Result<(), ShadowExecutionLineageError> {
        if registry.authority_stamp() != &self.authority {
            return Err(ShadowExecutionLineageError::ExecutionAuthorityChanged);
        }
        let current = registry.certify_retained_run(
            self.run.id(),
            &self.retained_start,
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
            return Err(ShadowExecutionLineageError::CertificateStale);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ShadowExecutionLineageError {
    DuplicateObservedTick { tick: CanonicalTick },
    ConflictingRunnerRegistration { runner: ShadowReferenceRunnerKey },
    ConflictingRunRegistration { run: ShadowReferenceRunId },
    RunWithoutQualifiedRunner { run: ShadowReferenceRunId, runner: ShadowReferenceRunnerKey },
    DuplicateShadowEvidenceRunBinding { evidence: ShadowEvidenceKey, revision: ShadowEvidenceRevision },
    RetainedStart(RetainedShadowStartError),
    Observable(ShadowObservableAuthorityError),
    UnknownRun { run: ShadowReferenceRunId },
    UnknownRunner { runner: ShadowReferenceRunnerKey },
    RunNotCompleted { run: ShadowReferenceRunId, status: ShadowReferenceRunStatus },
    RunnerNotQualified { runner: ShadowReferenceRunnerKey, status: ShadowReferenceRunnerStatus },
    ShadowEvidenceMismatch,
    WindowMismatch { expected: ShadowValidationWindow, actual: ShadowValidationWindow },
    ScenarioMismatch,
    ExecutionManifestMismatch,
    RunnerRepresentationMismatch { expected: RepresentationKey, actual: RepresentationKey },
    RunnerImplementationMismatch,
    RunnerExecutionCapsuleMismatch,
    RetainedStartBindingMismatch,
    ObservedStateCardinalityMismatch { expected: usize, actual: usize },
    MissingObservedTick { tick: CanonicalTick },
    ObservedStateMismatch { tick: CanonicalTick },
    ObservedRepresentationMismatch { tick: CanonicalTick, expected: RepresentationKey, actual: RepresentationKey },
    ExecutionAuthorityChanged,
    CertificateStale,
}

impl fmt::Display for ShadowExecutionLineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateObservedTick { tick } => write!(f, "duplicate shadow run state at tick {}", tick.0),
            Self::ConflictingRunnerRegistration { runner } => write!(f, "conflicting shadow runner {}@{}", runner.id(), runner.version()),
            Self::ConflictingRunRegistration { run } => write!(f, "conflicting shadow run id {}", run.0),
            Self::RunWithoutQualifiedRunner { run, runner } => write!(f, "shadow run {} references unknown runner {}@{}", run.0, runner.id(), runner.version()),
            Self::DuplicateShadowEvidenceRunBinding { evidence, revision } => write!(f, "shadow evidence {}@{} revision {} is bound to multiple runs", evidence.id(), evidence.version(), revision.0),
            Self::RetainedStart(error) => write!(f, "retained common-start authority: {error}"),
            Self::Observable(error) => write!(f, "observable authority: {error}"),
            Self::UnknownRun { run } => write!(f, "unknown shadow reference run {}", run.0),
            Self::UnknownRunner { runner } => write!(f, "unknown shadow reference runner {}@{}", runner.id(), runner.version()),
            Self::RunNotCompleted { run, status } => write!(f, "shadow reference run {} is {status:?}, not Completed", run.0),
            Self::RunnerNotQualified { runner, status } => write!(f, "shadow reference runner {}@{} is {status:?}, not Qualified", runner.id(), runner.version()),
            Self::ShadowEvidenceMismatch => write!(f, "shadow execution lineage references different Q2 evidence"),
            Self::WindowMismatch { expected, actual } => write!(f, "shadow execution window ({}, {}] differs from Q2 ({}, {}]", actual.start_exclusive().0, actual.end_inclusive().0, expected.start_exclusive().0, expected.end_inclusive().0),
            Self::ScenarioMismatch => write!(f, "shadow execution scenario differs from Q2 evidence"),
            Self::ExecutionManifestMismatch => write!(f, "shadow execution manifest differs from Q2 evidence"),
            Self::RunnerRepresentationMismatch { expected, actual } => write!(f, "runner reference representation {actual:?} differs from required {expected:?}"),
            Self::RunnerImplementationMismatch => write!(f, "runner implementation differs from Q2 reference implementation"),
            Self::RunnerExecutionCapsuleMismatch => write!(f, "runner execution capsule differs from Q2 profile"),
            Self::RetainedStartBindingMismatch => write!(f, "run initialization does not bind the exact retained T0 state from #553"),
            Self::ObservedStateCardinalityMismatch { expected, actual } => write!(f, "run binds {actual} sampled states, Q2 requires {expected}"),
            Self::MissingObservedTick { tick } => write!(f, "run lacks reference state at required Q2 tick {}", tick.0),
            Self::ObservedStateMismatch { tick } => write!(f, "run state identity differs from #406 reference observation at tick {}", tick.0),
            Self::ObservedRepresentationMismatch { tick, expected, actual } => write!(f, "run state at tick {} uses {actual:?}, expected reference {expected:?}", tick.0),
            Self::ExecutionAuthorityChanged => write!(f, "exact shadow execution authority changed"),
            Self::CertificateStale => write!(f, "shadow execution lineage certificate is stale"),
        }
    }
}

impl Error for ShadowExecutionLineageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RetainedStart(error) => Some(error),
            Self::Observable(error) => Some(error),
            _ => None,
        }
    }
}

// Keep source revisions semantically distinct from canonical time. This private
// trait intentionally provides no conversion and always returns None.
trait SourceRevisionIsNotTime {
    fn source_revision_tick_hint(&self) -> Option<CanonicalTick>;
}
impl SourceRevisionIsNotTime for ShadowObservationSourceIdentity {
    fn source_revision_tick_hint(&self) -> Option<CanonicalTick> { None }
}
