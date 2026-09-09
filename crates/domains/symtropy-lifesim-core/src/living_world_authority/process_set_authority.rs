// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Generation-stamped authority for enabled ecological process sets.
//!
//! A process must never become schedulable before the information/authority it
//! requires exists. This first #271 tranche can atomically commit a process-set
//! change only when the current representation already satisfies the complete
//! target set under exact information-policy and spatiotemporal authority.
//! Otherwise it returns `PromotionRequired` with zero process-set mutation.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::{ProcessKey, RepresentationKey};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::{InformationRegistryError, RegisteredSufficiencyReport};

use super::spatiotemporal_information::{
    CanonicalTick, SpatiotemporalPolicyAuthorityStamp, SpatiotemporalPolicyError,
    SpatiotemporalPolicyRegistry, SpatiotemporalStateContext, SpatiotemporalSufficiencyReport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessSetAuthorityScope {
    id: u128,
    version: u32,
}

impl ProcessSetAuthorityScope {
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
pub struct ProcessSetGeneration(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessSetChangeRequestId(pub u128);

/// Canonical enable/disable request. Input ordering is erased by ordered sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSetChangeRequest {
    id: ProcessSetChangeRequestId,
    enable: BTreeSet<ProcessKey>,
    disable: BTreeSet<ProcessKey>,
}

impl ProcessSetChangeRequest {
    pub fn new(
        id: ProcessSetChangeRequestId,
        enable: impl IntoIterator<Item = ProcessKey>,
        disable: impl IntoIterator<Item = ProcessKey>,
    ) -> Result<Self, ProcessSetAuthorityError> {
        let enable = enable.into_iter().collect::<BTreeSet<_>>();
        let disable = disable.into_iter().collect::<BTreeSet<_>>();
        if let Some(process) = enable.intersection(&disable).next().copied() {
            return Err(ProcessSetAuthorityError::ConflictingEnableDisable { process });
        }
        if enable.is_empty() && disable.is_empty() {
            return Err(ProcessSetAuthorityError::EmptyChangeRequest);
        }
        Ok(Self {
            id,
            enable,
            disable,
        })
    }

    pub const fn id(&self) -> ProcessSetChangeRequestId {
        self.id
    }

    pub fn enable(&self) -> &BTreeSet<ProcessKey> {
        &self.enable
    }

    pub fn disable(&self) -> &BTreeSet<ProcessKey> {
        &self.disable
    }

    fn apply_to(&self, current: &BTreeSet<ProcessKey>) -> BTreeSet<ProcessKey> {
        let mut target = current.clone();
        for process in &self.disable {
            target.remove(process);
        }
        target.extend(self.enable.iter().copied());
        target
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSetAuthorityStamp {
    scope: ProcessSetAuthorityScope,
    generation: ProcessSetGeneration,
    information_policy_authority: InformationPolicyAuthorityStamp,
    spatiotemporal_authority: SpatiotemporalPolicyAuthorityStamp,
    enabled: BTreeSet<ProcessKey>,
}

impl ProcessSetAuthorityStamp {
    pub const fn scope(&self) -> ProcessSetAuthorityScope {
        self.scope
    }

    pub const fn generation(&self) -> ProcessSetGeneration {
        self.generation
    }

    pub const fn information_policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.information_policy_authority
    }

    pub const fn spatiotemporal_authority(&self) -> &SpatiotemporalPolicyAuthorityStamp {
        &self.spatiotemporal_authority
    }

    pub fn enabled(&self) -> &BTreeSet<ProcessKey> {
        &self.enabled
    }

    pub fn is_enabled(&self, process: ProcessKey) -> bool {
        self.enabled.contains(&process)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommittedProcessSetRequest {
    request: ProcessSetChangeRequest,
    target: BTreeSet<ProcessKey>,
    resulting_authority: ProcessSetAuthorityStamp,
}

/// Canonical runtime owner of the enabled authoritative process set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeProcessSet {
    authority: ProcessSetAuthorityStamp,
    committed_requests: BTreeMap<ProcessSetChangeRequestId, CommittedProcessSetRequest>,
}

impl AuthoritativeProcessSet {
    /// V0 bootstraps an empty process set. Loading a non-empty persisted process
    /// set should later require its own exact persistence/replay authority path.
    pub fn bootstrap_empty(
        scope: ProcessSetAuthorityScope,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<Self, ProcessSetAuthorityError> {
        policy
            .authority_stamp()
            .validate_registry(policy.registry())
            .map_err(ProcessSetAuthorityError::PolicyIdentity)?;
        spatiotemporal
            .validate_current(policy)
            .map_err(ProcessSetAuthorityError::SpatiotemporalPolicy)?;

        Ok(Self {
            authority: ProcessSetAuthorityStamp {
                scope,
                generation: ProcessSetGeneration(0),
                information_policy_authority: policy.authority_stamp().clone(),
                spatiotemporal_authority: spatiotemporal.authority_stamp().clone(),
                enabled: BTreeSet::new(),
            },
            committed_requests: BTreeMap::new(),
        })
    }

    pub const fn authority_stamp(&self) -> &ProcessSetAuthorityStamp {
        &self.authority
    }

    pub fn is_enabled(&self, process: ProcessKey) -> bool {
        self.authority.is_enabled(process)
    }

    fn validate_authority_context(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<(), ProcessSetAuthorityError> {
        self.authority
            .information_policy_authority
            .validate_registry(policy.registry())
            .map_err(ProcessSetAuthorityError::PolicyIdentity)?;
        if policy.authority_stamp() != &self.authority.information_policy_authority {
            return Err(ProcessSetAuthorityError::InformationPolicyAuthorityChanged);
        }
        spatiotemporal
            .validate_current(policy)
            .map_err(ProcessSetAuthorityError::SpatiotemporalPolicy)?;
        if spatiotemporal.authority_stamp() != &self.authority.spatiotemporal_authority {
            return Err(ProcessSetAuthorityError::SpatiotemporalAuthorityChanged);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &self,
        request: &ProcessSetChangeRequest,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        representation: RepresentationKey,
        current_tick: CanonicalTick,
        state: &SpatiotemporalStateContext,
    ) -> Result<ProcessSetPreparationOutcome, ProcessSetAuthorityError> {
        self.validate_authority_context(policy, spatiotemporal)?;

        if let Some(committed) = self.committed_requests.get(&request.id()) {
            if &committed.request != request {
                return Err(ProcessSetAuthorityError::ConflictingRequestIdReuse {
                    request: request.id(),
                });
            }
            return Ok(ProcessSetPreparationOutcome::AlreadyCommitted(
                committed.resulting_authority.clone(),
            ));
        }

        let target = request.apply_to(self.authority.enabled());
        for process in &target {
            policy
                .registry()
                .resolve_process(*process)
                .map_err(ProcessSetAuthorityError::InformationRegistry)?;
        }

        if target == *self.authority.enabled() {
            return Ok(ProcessSetPreparationOutcome::NoChange(
                self.authority.clone(),
            ));
        }

        let information_report = policy
            .registry()
            .evaluate_registered(target.iter().copied(), representation)
            .map_err(ProcessSetAuthorityError::InformationRegistry)?;
        let spatiotemporal_report = spatiotemporal
            .evaluate(
                policy,
                target.iter().copied(),
                representation,
                current_tick,
                state,
            )
            .map_err(ProcessSetAuthorityError::SpatiotemporalPolicy)?;

        if !information_report.is_sufficient() || !spatiotemporal_report.is_sufficient() {
            return Ok(ProcessSetPreparationOutcome::PromotionRequired(
                PromotionRequiredProcessSetChange {
                    request: request.clone(),
                    source_authority: self.authority.clone(),
                    target,
                    representation,
                    current_tick,
                    state: state.clone(),
                    information_report,
                    spatiotemporal_report,
                },
            ));
        }

        Ok(ProcessSetPreparationOutcome::Ready(
            PreparedProcessSetChange {
                request: request.clone(),
                source_authority: self.authority.clone(),
                target,
                representation,
                current_tick,
                state: state.clone(),
                information_report,
                spatiotemporal_report,
            },
        ))
    }

    /// Commit a process-set change only when the current representation already
    /// remains sufficient for the complete target set. `PromotionRequired` has no
    /// corresponding commit API in this tranche.
    pub fn commit_ready(
        &mut self,
        prepared: &PreparedProcessSetChange,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<ProcessSetAuthorityStamp, ProcessSetAuthorityError> {
        if let Some(committed) = self.committed_requests.get(&prepared.request.id()) {
            if committed.request == prepared.request && committed.target == prepared.target {
                return Ok(committed.resulting_authority.clone());
            }
            return Err(ProcessSetAuthorityError::ConflictingRequestIdReuse {
                request: prepared.request.id(),
            });
        }

        if self.authority != prepared.source_authority {
            return Err(ProcessSetAuthorityError::PreparedSourceAuthorityStale);
        }
        self.validate_authority_context(policy, spatiotemporal)?;

        let current = self.prepare(
            &prepared.request,
            policy,
            spatiotemporal,
            prepared.representation,
            prepared.current_tick,
            &prepared.state,
        )?;
        let ProcessSetPreparationOutcome::Ready(current) = current else {
            return Err(ProcessSetAuthorityError::PreparedChangeNoLongerReady);
        };
        if current != *prepared {
            return Err(ProcessSetAuthorityError::PreparedChangeStale);
        }

        let next_generation = self
            .authority
            .generation
            .0
            .checked_add(1)
            .ok_or(ProcessSetAuthorityError::ProcessSetGenerationOverflow)?;
        let resulting_authority = ProcessSetAuthorityStamp {
            scope: self.authority.scope,
            generation: ProcessSetGeneration(next_generation),
            information_policy_authority: self.authority.information_policy_authority.clone(),
            spatiotemporal_authority: self.authority.spatiotemporal_authority.clone(),
            enabled: prepared.target.clone(),
        };

        self.authority = resulting_authority.clone();
        self.committed_requests.insert(
            prepared.request.id(),
            CommittedProcessSetRequest {
                request: prepared.request.clone(),
                target: prepared.target.clone(),
                resulting_authority: resulting_authority.clone(),
            },
        );
        Ok(resulting_authority)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessSetPreparationOutcome {
    AlreadyCommitted(ProcessSetAuthorityStamp),
    NoChange(ProcessSetAuthorityStamp),
    Ready(PreparedProcessSetChange),
    PromotionRequired(PromotionRequiredProcessSetChange),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedProcessSetChange {
    request: ProcessSetChangeRequest,
    source_authority: ProcessSetAuthorityStamp,
    target: BTreeSet<ProcessKey>,
    representation: RepresentationKey,
    current_tick: CanonicalTick,
    state: SpatiotemporalStateContext,
    information_report: RegisteredSufficiencyReport,
    spatiotemporal_report: SpatiotemporalSufficiencyReport,
}

impl PreparedProcessSetChange {
    pub const fn request(&self) -> &ProcessSetChangeRequest {
        &self.request
    }

    pub const fn source_authority(&self) -> &ProcessSetAuthorityStamp {
        &self.source_authority
    }

    pub fn target(&self) -> &BTreeSet<ProcessKey> {
        &self.target
    }

    pub const fn representation(&self) -> RepresentationKey {
        self.representation
    }

    pub const fn information_report(&self) -> &RegisteredSufficiencyReport {
        &self.information_report
    }

    pub const fn spatiotemporal_report(&self) -> &SpatiotemporalSufficiencyReport {
        &self.spatiotemporal_report
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionRequiredProcessSetChange {
    request: ProcessSetChangeRequest,
    source_authority: ProcessSetAuthorityStamp,
    target: BTreeSet<ProcessKey>,
    representation: RepresentationKey,
    current_tick: CanonicalTick,
    state: SpatiotemporalStateContext,
    information_report: RegisteredSufficiencyReport,
    spatiotemporal_report: SpatiotemporalSufficiencyReport,
}

impl PromotionRequiredProcessSetChange {
    pub const fn request(&self) -> &ProcessSetChangeRequest {
        &self.request
    }

    pub const fn source_authority(&self) -> &ProcessSetAuthorityStamp {
        &self.source_authority
    }

    pub fn target(&self) -> &BTreeSet<ProcessKey> {
        &self.target
    }

    pub const fn representation(&self) -> RepresentationKey {
        self.representation
    }

    pub const fn information_report(&self) -> &RegisteredSufficiencyReport {
        &self.information_report
    }

    pub const fn spatiotemporal_report(&self) -> &SpatiotemporalSufficiencyReport {
        &self.spatiotemporal_report
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessSetAuthorityError {
    EmptyChangeRequest,
    ConflictingEnableDisable {
        process: ProcessKey,
    },
    ConflictingRequestIdReuse {
        request: ProcessSetChangeRequestId,
    },
    PolicyIdentity(InformationPolicyIdentityError),
    InformationPolicyAuthorityChanged,
    InformationRegistry(InformationRegistryError),
    SpatiotemporalPolicy(SpatiotemporalPolicyError),
    SpatiotemporalAuthorityChanged,
    PreparedSourceAuthorityStale,
    PreparedChangeNoLongerReady,
    PreparedChangeStale,
    ProcessSetGenerationOverflow,
}

impl fmt::Display for ProcessSetAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyChangeRequest => write!(formatter, "process-set change request is empty"),
            Self::ConflictingEnableDisable { process } => write!(
                formatter,
                "process {process:?} appears in both enable and disable sets"
            ),
            Self::ConflictingRequestIdReuse { request } => write!(
                formatter,
                "process-set request id {} was reused with conflicting semantics",
                request.0
            ),
            Self::PolicyIdentity(error) => {
                write!(formatter, "information-policy identity error: {error}")
            }
            Self::InformationPolicyAuthorityChanged => {
                write!(formatter, "exact information-policy authority changed")
            }
            Self::InformationRegistry(error) => {
                write!(formatter, "information registry error: {error}")
            }
            Self::SpatiotemporalPolicy(error) => {
                write!(formatter, "spatiotemporal policy error: {error}")
            }
            Self::SpatiotemporalAuthorityChanged => {
                write!(formatter, "exact spatiotemporal authority changed")
            }
            Self::PreparedSourceAuthorityStale => write!(
                formatter,
                "prepared process-set change was based on a stale source generation"
            ),
            Self::PreparedChangeNoLongerReady => write!(
                formatter,
                "prepared process-set change no longer passes no-promotion readiness"
            ),
            Self::PreparedChangeStale => {
                write!(formatter, "prepared process-set change differs from current recomputation")
            }
            Self::ProcessSetGenerationOverflow => {
                write!(formatter, "process-set generation overflow")
            }
        }
    }
}

impl Error for ProcessSetAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyIdentity(error) => Some(error),
            Self::InformationRegistry(error) => Some(error),
            Self::SpatiotemporalPolicy(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
        ProcessInformationProfile, ProcessInformationRequirement, RepresentationCapabilities,
    };
    use crate::information_registry::{InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey};
    use crate::living_world_authority::spatiotemporal_information::{
        IntegrationSemantics, MaximumStateAgeTicks, MaximumUpdateIntervalTicks,
        ProcessSpatiotemporalRequirement, RepresentationSpatiotemporalCapability,
        SpatialResolutionUnits, SpatiotemporalPolicyRegistryBuilder,
        SpatiotemporalPolicyRegistryKey,
    };

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(7_000, 1);
    const PROCESS_A: ProcessKey = ProcessKey::new(7_010, 1);
    const PROCESS_B: ProcessKey = ProcessKey::new(7_011, 1);
    const PROCESS_C: ProcessKey = ProcessKey::new(7_012, 1);
    const REPRESENTATION: RepresentationKey = RepresentationKey::new(7_020, 1);
    const ST_POLICY: SpatiotemporalPolicyRegistryKey = SpatiotemporalPolicyRegistryKey::new(7_030, 1);
    const SCOPE: ProcessSetAuthorityScope = ProcessSetAuthorityScope::new(7_040, 1);

    fn policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        for process in [PROCESS_A, PROCESS_C] {
            builder
                .register_process(ProcessInformationProfile::new(
                    process,
                    EcologicalAuthorityLevel::Coarse,
                    [ProcessInformationRequirement::exact(
                        EcologicalInformation::Headcount,
                    )],
                ))
                .unwrap();
        }
        builder
            .register_process(ProcessInformationProfile::new(
                PROCESS_B,
                EcologicalAuthorityLevel::Coarse,
                [ProcessInformationRequirement::exact(
                    EcologicalInformation::AgeDistribution,
                )],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                REPRESENTATION,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::Headcount,
                    CapabilityEvidence::Exact,
                )],
            ))
            .unwrap();
        builder.seal()
    }

    fn spatiotemporal(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> SpatiotemporalPolicyRegistry {
        let mut builder = SpatiotemporalPolicyRegistryBuilder::new(ST_POLICY);
        for process in [PROCESS_A, PROCESS_C] {
            builder
                .register_process_requirement(
                    ProcessSpatiotemporalRequirement::new(
                        process,
                        EcologicalInformation::Headcount,
                        SpatialResolutionUnits::new(5).unwrap(),
                        MaximumStateAgeTicks(0),
                        MaximumUpdateIntervalTicks::new(1).unwrap(),
                        IntegrationSemantics::Instantaneous,
                        None,
                        None,
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        builder
            .register_process_requirement(
                ProcessSpatiotemporalRequirement::new(
                    PROCESS_B,
                    EcologicalInformation::AgeDistribution,
                    SpatialResolutionUnits::new(5).unwrap(),
                    MaximumStateAgeTicks(0),
                    MaximumUpdateIntervalTicks::new(1).unwrap(),
                    IntegrationSemantics::Instantaneous,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        builder
            .register_representation_capability(
                RepresentationSpatiotemporalCapability::new(
                    REPRESENTATION,
                    EcologicalInformation::Headcount,
                    SpatialResolutionUnits::new(5).unwrap(),
                    MaximumUpdateIntervalTicks::new(1).unwrap(),
                    IntegrationSemantics::Instantaneous,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        builder.seal(policy).unwrap()
    }

    fn state() -> SpatiotemporalStateContext {
        SpatiotemporalStateContext::from_records(
            REPRESENTATION,
            [(
                EcologicalInformation::Headcount,
                CanonicalTick(10),
            )],
        )
        .unwrap()
    }

    fn context() -> (
        InformationPolicyRegistry,
        SpatiotemporalPolicyRegistry,
    ) {
        let raw = policy();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let st = spatiotemporal(&exact);
        (raw, st)
    }

    #[test]
    fn supported_process_is_not_enabled_until_ready_plan_commits() {
        let (raw, st) = context();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let mut processes = AuthoritativeProcessSet::bootstrap_empty(SCOPE, &exact, &st).unwrap();
        let request = ProcessSetChangeRequest::new(
            ProcessSetChangeRequestId(1),
            [PROCESS_A],
            [],
        )
        .unwrap();

        let ProcessSetPreparationOutcome::Ready(plan) = processes
            .prepare(
                &request,
                &exact,
                &st,
                REPRESENTATION,
                CanonicalTick(10),
                &state(),
            )
            .unwrap()
        else {
            panic!("supported process should prepare without promotion");
        };
        assert!(!processes.is_enabled(PROCESS_A));
        let committed = processes.commit_ready(&plan, &exact, &st).unwrap();
        assert!(committed.is_enabled(PROCESS_A));
        assert_eq!(committed.generation(), ProcessSetGeneration(1));
    }

    #[test]
    fn insufficient_process_returns_promotion_required_without_mutation() {
        let (raw, st) = context();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let processes = AuthoritativeProcessSet::bootstrap_empty(SCOPE, &exact, &st).unwrap();
        let before = processes.authority_stamp().clone();
        let request = ProcessSetChangeRequest::new(
            ProcessSetChangeRequestId(2),
            [PROCESS_B],
            [],
        )
        .unwrap();

        assert!(matches!(
            processes
                .prepare(
                    &request,
                    &exact,
                    &st,
                    REPRESENTATION,
                    CanonicalTick(10),
                    &state(),
                )
                .unwrap(),
            ProcessSetPreparationOutcome::PromotionRequired(_)
        ));
        assert_eq!(processes.authority_stamp(), &before);
        assert!(!processes.is_enabled(PROCESS_B));
    }

    #[test]
    fn acknowledgement_loss_retry_does_not_increment_generation_twice() {
        let (raw, st) = context();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let mut processes = AuthoritativeProcessSet::bootstrap_empty(SCOPE, &exact, &st).unwrap();
        let request = ProcessSetChangeRequest::new(
            ProcessSetChangeRequestId(3),
            [PROCESS_A],
            [],
        )
        .unwrap();
        let ProcessSetPreparationOutcome::Ready(plan) = processes
            .prepare(
                &request,
                &exact,
                &st,
                REPRESENTATION,
                CanonicalTick(10),
                &state(),
            )
            .unwrap()
        else {
            panic!("expected ready plan");
        };
        let first = processes.commit_ready(&plan, &exact, &st).unwrap();
        let retry = processes.commit_ready(&plan, &exact, &st).unwrap();
        assert_eq!(first, retry);
        assert_eq!(retry.generation(), ProcessSetGeneration(1));
    }

    #[test]
    fn old_plan_rejects_after_another_process_generation_commits() {
        let (raw, st) = context();
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let mut processes = AuthoritativeProcessSet::bootstrap_empty(SCOPE, &exact, &st).unwrap();

        let request_a = ProcessSetChangeRequest::new(
            ProcessSetChangeRequestId(4),
            [PROCESS_A],
            [],
        )
        .unwrap();
        let request_c = ProcessSetChangeRequest::new(
            ProcessSetChangeRequestId(5),
            [PROCESS_C],
            [],
        )
        .unwrap();
        let ProcessSetPreparationOutcome::Ready(plan_a) = processes
            .prepare(
                &request_a,
                &exact,
                &st,
                REPRESENTATION,
                CanonicalTick(10),
                &state(),
            )
            .unwrap()
        else {
            panic!("expected A ready");
        };
        let ProcessSetPreparationOutcome::Ready(plan_c) = processes
            .prepare(
                &request_c,
                &exact,
                &st,
                REPRESENTATION,
                CanonicalTick(10),
                &state(),
            )
            .unwrap()
        else {
            panic!("expected C ready");
        };
        processes.commit_ready(&plan_c, &exact, &st).unwrap();
        assert!(matches!(
            processes.commit_ready(&plan_a, &exact, &st),
            Err(ProcessSetAuthorityError::PreparedSourceAuthorityStale)
        ));
    }

    #[test]
    fn request_input_order_is_canonicalized() {
        let left = ProcessSetChangeRequest::new(
            ProcessSetChangeRequestId(6),
            [PROCESS_A, PROCESS_C],
            [],
        )
        .unwrap();
        let right = ProcessSetChangeRequest::new(
            ProcessSetChangeRequestId(6),
            [PROCESS_C, PROCESS_A],
            [],
        )
        .unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn contradictory_request_rejects() {
        assert!(matches!(
            ProcessSetChangeRequest::new(
                ProcessSetChangeRequestId(7),
                [PROCESS_A],
                [PROCESS_A],
            ),
            Err(ProcessSetAuthorityError::ConflictingEnableDisable { .. })
        ));
    }
}
