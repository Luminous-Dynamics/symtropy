// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Linearized authority for canonical event-guard runtime state.
//!
//! [`super::event_guard`] intentionally provides a read-only deterministic guard
//! evaluator. This layer owns the future-bearing latch/sample state, stamps every
//! accepted advance with a non-reused revision, and mints a stable event receipt
//! only when a trigger commits. Retry after acknowledgement loss is idempotent;
//! stale/forked prepared advances cannot commit twice.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::information::ProcessKey;
use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;

use super::event_guard::{
    EventGuardAuthorityStamp, EventGuardError, EventGuardKey, EventGuardRegistry,
    GuardEvaluation, GuardLatchState, GuardObservableKey, GuardOutcome, GuardRuntimeState,
    GuardSample, GuardTickBracket,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuardStateAuthorityScope {
    id: u128,
    version: u32,
}

impl GuardStateAuthorityScope {
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
pub struct GuardStateRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuardAdvanceRequestId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalGuardEventId {
    scope: GuardStateAuthorityScope,
    revision: GuardStateRevision,
}

impl CanonicalGuardEventId {
    pub const fn scope(self) -> GuardStateAuthorityScope {
        self.scope
    }

    pub const fn revision(self) -> GuardStateRevision {
        self.revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuardAdvanceRequest {
    id: GuardAdvanceRequestId,
    sample: GuardSample,
}

impl GuardAdvanceRequest {
    pub const fn new(id: GuardAdvanceRequestId, sample: GuardSample) -> Self {
        Self { id, sample }
    }

    pub const fn id(self) -> GuardAdvanceRequestId {
        self.id
    }

    pub const fn sample(self) -> GuardSample {
        self.sample
    }
}

/// Exact authority identity for the current canonical guard state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardStateAuthorityStamp {
    scope: GuardStateAuthorityScope,
    revision: GuardStateRevision,
    guard_authority: EventGuardAuthorityStamp,
    guard: EventGuardKey,
    latch: GuardLatchState,
    last_sample: GuardSample,
}

impl GuardStateAuthorityStamp {
    pub const fn scope(&self) -> GuardStateAuthorityScope {
        self.scope
    }

    pub const fn revision(&self) -> GuardStateRevision {
        self.revision
    }

    pub const fn guard_authority(&self) -> &EventGuardAuthorityStamp {
        &self.guard_authority
    }

    pub const fn guard(&self) -> EventGuardKey {
        self.guard
    }

    pub const fn latch(&self) -> GuardLatchState {
        self.latch
    }

    pub const fn last_sample(&self) -> GuardSample {
        self.last_sample
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalGuardEventReceipt {
    id: CanonicalGuardEventId,
    request: GuardAdvanceRequestId,
    source_revision: GuardStateRevision,
    resulting_revision: GuardStateRevision,
    guard_authority: EventGuardAuthorityStamp,
    guard: EventGuardKey,
    process: ProcessKey,
    observable: GuardObservableKey,
    bracket: GuardTickBracket,
}

impl CanonicalGuardEventReceipt {
    pub const fn id(&self) -> CanonicalGuardEventId {
        self.id
    }

    pub const fn request(&self) -> GuardAdvanceRequestId {
        self.request
    }

    pub const fn source_revision(&self) -> GuardStateRevision {
        self.source_revision
    }

    pub const fn resulting_revision(&self) -> GuardStateRevision {
        self.resulting_revision
    }

    pub const fn guard_authority(&self) -> &EventGuardAuthorityStamp {
        &self.guard_authority
    }

    pub const fn guard(&self) -> EventGuardKey {
        self.guard
    }

    pub const fn process(&self) -> ProcessKey {
        self.process
    }

    pub const fn observable(&self) -> GuardObservableKey {
        self.observable
    }

    pub const fn bracket(&self) -> GuardTickBracket {
        self.bracket
    }
}

/// Result of one committed guard advance. Every accepted sample advances the
/// state revision because the last canonical sample is itself future-bearing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardAdvanceCommit {
    authority: GuardStateAuthorityStamp,
    outcome: GuardOutcome,
    event: Option<CanonicalGuardEventReceipt>,
}

impl GuardAdvanceCommit {
    pub const fn authority_stamp(&self) -> &GuardStateAuthorityStamp {
        &self.authority
    }

    pub const fn outcome(&self) -> GuardOutcome {
        self.outcome
    }

    pub const fn event(&self) -> Option<&CanonicalGuardEventReceipt> {
        self.event.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommittedGuardAdvance {
    request: GuardAdvanceRequest,
    result: GuardAdvanceCommit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedGuardAdvance {
    request: GuardAdvanceRequest,
    source_authority: GuardStateAuthorityStamp,
    evaluation: GuardEvaluation,
}

impl PreparedGuardAdvance {
    pub const fn request(&self) -> GuardAdvanceRequest {
        self.request
    }

    pub const fn source_authority(&self) -> &GuardStateAuthorityStamp {
        &self.source_authority
    }

    pub const fn evaluation(&self) -> &GuardEvaluation {
        &self.evaluation
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardAdvancePreparationOutcome {
    AlreadyCommitted(GuardAdvanceCommit),
    Ready(PreparedGuardAdvance),
}

/// Canonical owner of one guard's future-bearing runtime state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeGuardState {
    authority: GuardStateAuthorityStamp,
    runtime_state: GuardRuntimeState,
    committed_requests: BTreeMap<GuardAdvanceRequestId, CommittedGuardAdvance>,
}

impl AuthoritativeGuardState {
    /// Bootstrap one guard at revision zero without fabricating a trigger. The
    /// underlying guard registry decides whether the initial sample is armed or
    /// already latched.
    pub fn bootstrap(
        scope: GuardStateAuthorityScope,
        guards: &EventGuardRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        guard: EventGuardKey,
        sample: GuardSample,
    ) -> Result<Self, GuardStateAuthorityError> {
        let runtime_state = guards
            .initialize_guard(policy, guard, sample)
            .map_err(GuardStateAuthorityError::EventGuard)?;
        let authority = stamp_from_runtime(scope, GuardStateRevision(0), &runtime_state);
        Ok(Self {
            authority,
            runtime_state,
            committed_requests: BTreeMap::new(),
        })
    }

    pub const fn authority_stamp(&self) -> &GuardStateAuthorityStamp {
        &self.authority
    }

    pub const fn runtime_state(&self) -> &GuardRuntimeState {
        &self.runtime_state
    }

    fn validate_context(
        &self,
        guards: &EventGuardRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), GuardStateAuthorityError> {
        guards
            .validate_current(policy)
            .map_err(GuardStateAuthorityError::EventGuard)?;
        if guards.authority_stamp() != self.authority.guard_authority() {
            return Err(GuardStateAuthorityError::GuardAuthorityChanged);
        }
        if self.runtime_state.authority_stamp() != self.authority.guard_authority()
            || self.runtime_state.guard() != self.authority.guard()
            || self.runtime_state.latch() != self.authority.latch()
            || self.runtime_state.last_sample() != self.authority.last_sample()
        {
            return Err(GuardStateAuthorityError::RuntimeStateAuthorityMismatch);
        }
        Ok(())
    }

    pub fn prepare(
        &self,
        request: GuardAdvanceRequest,
        guards: &EventGuardRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<GuardAdvancePreparationOutcome, GuardStateAuthorityError> {
        self.validate_context(guards, policy)?;

        if let Some(committed) = self.committed_requests.get(&request.id()) {
            if committed.request != request {
                return Err(GuardStateAuthorityError::ConflictingRequestIdReuse {
                    request: request.id(),
                });
            }
            return Ok(GuardAdvancePreparationOutcome::AlreadyCommitted(
                committed.result.clone(),
            ));
        }

        let evaluation = guards
            .advance_guard(policy, &self.runtime_state, request.sample())
            .map_err(GuardStateAuthorityError::EventGuard)?;
        Ok(GuardAdvancePreparationOutcome::Ready(PreparedGuardAdvance {
            request,
            source_authority: self.authority.clone(),
            evaluation,
        }))
    }

    /// Commit one prepared canonical sample at a single owner-local linearization
    /// point. Every accepted sample gets a fresh revision; only a trigger gets an
    /// event receipt.
    pub fn commit(
        &mut self,
        prepared: &PreparedGuardAdvance,
        guards: &EventGuardRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<GuardAdvanceCommit, GuardStateAuthorityError> {
        if let Some(committed) = self.committed_requests.get(&prepared.request.id()) {
            if committed.request == prepared.request {
                return Ok(committed.result.clone());
            }
            return Err(GuardStateAuthorityError::ConflictingRequestIdReuse {
                request: prepared.request.id(),
            });
        }

        if self.authority != prepared.source_authority {
            return Err(GuardStateAuthorityError::PreparedSourceRevisionStale);
        }
        self.validate_context(guards, policy)?;

        let current = self.prepare(prepared.request, guards, policy)?;
        let GuardAdvancePreparationOutcome::Ready(current) = current else {
            return Err(GuardStateAuthorityError::PreparedAdvanceNoLongerReady);
        };
        if current != *prepared {
            return Err(GuardStateAuthorityError::PreparedAdvanceStale);
        }

        let next_revision = GuardStateRevision(
            self.authority
                .revision()
                .0
                .checked_add(1)
                .ok_or(GuardStateAuthorityError::GuardStateRevisionOverflow)?,
        );
        let next_runtime = prepared.evaluation.state().clone();
        let next_authority = stamp_from_runtime(self.authority.scope(), next_revision, &next_runtime);
        let outcome = prepared.evaluation.outcome();

        let event = match outcome {
            GuardOutcome::Triggered { bracket } => {
                let definition = guards
                    .definition(next_authority.guard())
                    .ok_or(GuardStateAuthorityError::UnknownGuardAfterPrepare {
                        guard: next_authority.guard(),
                    })?;
                Some(CanonicalGuardEventReceipt {
                    id: CanonicalGuardEventId {
                        scope: next_authority.scope(),
                        revision: next_revision,
                    },
                    request: prepared.request.id(),
                    source_revision: prepared.source_authority.revision(),
                    resulting_revision: next_revision,
                    guard_authority: next_authority.guard_authority().clone(),
                    guard: next_authority.guard(),
                    process: definition.process(),
                    observable: definition.observable(),
                    bracket,
                })
            }
            GuardOutcome::NoChange | GuardOutcome::Rearmed { .. } => None,
        };

        let result = GuardAdvanceCommit {
            authority: next_authority.clone(),
            outcome,
            event,
        };
        self.runtime_state = next_runtime;
        self.authority = next_authority;
        self.committed_requests.insert(
            prepared.request.id(),
            CommittedGuardAdvance {
                request: prepared.request,
                result: result.clone(),
            },
        );
        Ok(result)
    }
}

fn stamp_from_runtime(
    scope: GuardStateAuthorityScope,
    revision: GuardStateRevision,
    state: &GuardRuntimeState,
) -> GuardStateAuthorityStamp {
    GuardStateAuthorityStamp {
        scope,
        revision,
        guard_authority: state.authority_stamp().clone(),
        guard: state.guard(),
        latch: state.latch(),
        last_sample: state.last_sample(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GuardStateAuthorityError {
    EventGuard(EventGuardError),
    GuardAuthorityChanged,
    RuntimeStateAuthorityMismatch,
    ConflictingRequestIdReuse {
        request: GuardAdvanceRequestId,
    },
    PreparedSourceRevisionStale,
    PreparedAdvanceNoLongerReady,
    PreparedAdvanceStale,
    GuardStateRevisionOverflow,
    UnknownGuardAfterPrepare {
        guard: EventGuardKey,
    },
}

impl fmt::Display for GuardStateAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EventGuard(error) => write!(formatter, "event-guard error: {error}"),
            Self::GuardAuthorityChanged => write!(
                formatter,
                "exact event-guard authority changed for authoritative guard state"
            ),
            Self::RuntimeStateAuthorityMismatch => write!(
                formatter,
                "runtime guard state does not match its canonical authority stamp"
            ),
            Self::ConflictingRequestIdReuse { request } => write!(
                formatter,
                "guard advance request id {} was reused with conflicting semantics",
                request.0
            ),
            Self::PreparedSourceRevisionStale => write!(
                formatter,
                "prepared guard advance was based on a stale guard-state revision"
            ),
            Self::PreparedAdvanceNoLongerReady => write!(
                formatter,
                "prepared guard advance no longer resolves as a new ready advance"
            ),
            Self::PreparedAdvanceStale => write!(
                formatter,
                "prepared guard advance differs from current deterministic recomputation"
            ),
            Self::GuardStateRevisionOverflow => {
                write!(formatter, "guard-state revision overflow")
            }
            Self::UnknownGuardAfterPrepare { guard } => write!(
                formatter,
                "guard {}@{} disappeared after prepare",
                guard.id(),
                guard.version()
            ),
        }
    }
}

impl Error for GuardStateAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::EventGuard(error) => Some(error),
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
        RepresentationKey,
    };
    use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::living_world_authority::event_guard::{
        EventGuardDefinition, EventGuardRegistryBuilder, EventGuardRegistryKey, GuardDirection,
        GuardObservableKey, GuardScalar,
    };
    use crate::living_world_authority::spatiotemporal_information::CanonicalTick;

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(10_000, 1);
    const PROCESS: ProcessKey = ProcessKey::new(10_010, 1);
    const REPRESENTATION: RepresentationKey = RepresentationKey::new(10_020, 1);
    const GUARD_REGISTRY: EventGuardRegistryKey = EventGuardRegistryKey::new(10_030, 1);
    const GUARD: EventGuardKey = EventGuardKey::new(10_040, 1);
    const OBSERVABLE: GuardObservableKey = GuardObservableKey::new(10_050, 1);
    const SCOPE: GuardStateAuthorityScope = GuardStateAuthorityScope::new(10_060, 1);

    fn policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_process(ProcessInformationProfile::new(
                PROCESS,
                EcologicalAuthorityLevel::Coarse,
                [ProcessInformationRequirement::exact(
                    EcologicalInformation::Headcount,
                )],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                REPRESENTATION,
                EcologicalAuthorityLevel::Coarse,
                [(EcologicalInformation::Headcount, CapabilityEvidence::Exact)],
            ))
            .unwrap();
        builder.seal()
    }

    fn guards(policy: &InformationPolicyRegistry) -> EventGuardRegistry {
        let exact = ManifestBoundInformationPolicyRegistry::new(policy);
        let mut builder = EventGuardRegistryBuilder::new(GUARD_REGISTRY);
        builder
            .register_guard(
                EventGuardDefinition::new(
                    GUARD,
                    PROCESS,
                    OBSERVABLE,
                    GuardDirection::Rising,
                    GuardScalar::new(10.0).unwrap(),
                    GuardScalar::new(8.0).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
        builder.seal(&exact).unwrap()
    }

    fn sample(tick: u64, value: f64) -> GuardSample {
        GuardSample::from_value(CanonicalTick(tick), value).unwrap()
    }

    fn owner<'a>(
        raw: &'a InformationPolicyRegistry,
        guards: &EventGuardRegistry,
    ) -> AuthoritativeGuardState {
        let exact = ManifestBoundInformationPolicyRegistry::new(raw);
        AuthoritativeGuardState::bootstrap(SCOPE, guards, &exact, GUARD, sample(0, 9.0)).unwrap()
    }

    #[test]
    fn one_trigger_commits_one_revision_and_event_receipt() {
        let raw = policy();
        let guards = guards(&raw);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let mut owner = owner(&raw, &guards);
        let request = GuardAdvanceRequest::new(GuardAdvanceRequestId(1), sample(10, 11.0));
        let GuardAdvancePreparationOutcome::Ready(plan) =
            owner.prepare(request, &guards, &exact).unwrap()
        else {
            panic!("expected ready guard advance");
        };

        let result = owner.commit(&plan, &guards, &exact).unwrap();
        assert_eq!(result.authority_stamp().revision(), GuardStateRevision(1));
        assert!(matches!(result.outcome(), GuardOutcome::Triggered { .. }));
        let receipt = result.event().expect("trigger must mint an event receipt");
        assert_eq!(receipt.source_revision(), GuardStateRevision(0));
        assert_eq!(receipt.resulting_revision(), GuardStateRevision(1));
        assert_eq!(receipt.process(), PROCESS);
        assert_eq!(receipt.observable(), OBSERVABLE);
    }

    #[test]
    fn acknowledgement_loss_retry_returns_same_receipt_without_new_revision() {
        let raw = policy();
        let guards = guards(&raw);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let mut owner = owner(&raw, &guards);
        let request = GuardAdvanceRequest::new(GuardAdvanceRequestId(2), sample(10, 11.0));
        let GuardAdvancePreparationOutcome::Ready(plan) =
            owner.prepare(request, &guards, &exact).unwrap()
        else {
            panic!("expected ready guard advance");
        };
        let first = owner.commit(&plan, &guards, &exact).unwrap();
        let retry = owner.commit(&plan, &guards, &exact).unwrap();
        assert_eq!(first, retry);
        assert_eq!(owner.authority_stamp().revision(), GuardStateRevision(1));
    }

    #[test]
    fn competing_prepared_advance_from_old_revision_cannot_commit() {
        let raw = policy();
        let guards = guards(&raw);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let mut owner = owner(&raw, &guards);
        let request_a = GuardAdvanceRequest::new(GuardAdvanceRequestId(3), sample(10, 11.0));
        let request_b = GuardAdvanceRequest::new(GuardAdvanceRequestId(4), sample(10, 12.0));
        let GuardAdvancePreparationOutcome::Ready(plan_a) =
            owner.prepare(request_a, &guards, &exact).unwrap()
        else {
            panic!("expected A ready");
        };
        let GuardAdvancePreparationOutcome::Ready(plan_b) =
            owner.prepare(request_b, &guards, &exact).unwrap()
        else {
            panic!("expected B ready");
        };

        owner.commit(&plan_a, &guards, &exact).unwrap();
        assert!(matches!(
            owner.commit(&plan_b, &guards, &exact),
            Err(GuardStateAuthorityError::PreparedSourceRevisionStale)
        ));
        assert_eq!(owner.authority_stamp().revision(), GuardStateRevision(1));
    }

    #[test]
    fn conflicting_request_id_reuse_rejects() {
        let raw = policy();
        let guards = guards(&raw);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let mut owner = owner(&raw, &guards);
        let id = GuardAdvanceRequestId(5);
        let first_request = GuardAdvanceRequest::new(id, sample(10, 11.0));
        let GuardAdvancePreparationOutcome::Ready(plan) =
            owner.prepare(first_request, &guards, &exact).unwrap()
        else {
            panic!("expected ready");
        };
        owner.commit(&plan, &guards, &exact).unwrap();

        let conflict = GuardAdvanceRequest::new(id, sample(20, 12.0));
        assert!(matches!(
            owner.prepare(conflict, &guards, &exact),
            Err(GuardStateAuthorityError::ConflictingRequestIdReuse { .. })
        ));
    }

    #[test]
    fn rearm_is_revisioned_and_later_trigger_gets_new_event_identity() {
        let raw = policy();
        let guards = guards(&raw);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let mut owner = owner(&raw, &guards);

        let trigger = GuardAdvanceRequest::new(GuardAdvanceRequestId(6), sample(10, 11.0));
        let GuardAdvancePreparationOutcome::Ready(trigger_plan) =
            owner.prepare(trigger, &guards, &exact).unwrap()
        else {
            panic!("expected trigger plan");
        };
        let first = owner.commit(&trigger_plan, &guards, &exact).unwrap();
        let first_id = first.event().unwrap().id();

        let rearm = GuardAdvanceRequest::new(GuardAdvanceRequestId(7), sample(20, 8.0));
        let GuardAdvancePreparationOutcome::Ready(rearm_plan) =
            owner.prepare(rearm, &guards, &exact).unwrap()
        else {
            panic!("expected rearm plan");
        };
        let rearmed = owner.commit(&rearm_plan, &guards, &exact).unwrap();
        assert!(matches!(rearmed.outcome(), GuardOutcome::Rearmed { .. }));
        assert!(rearmed.event().is_none());
        assert_eq!(owner.authority_stamp().revision(), GuardStateRevision(2));

        let second_trigger =
            GuardAdvanceRequest::new(GuardAdvanceRequestId(8), sample(30, 12.0));
        let GuardAdvancePreparationOutcome::Ready(second_plan) =
            owner.prepare(second_trigger, &guards, &exact).unwrap()
        else {
            panic!("expected second trigger plan");
        };
        let second = owner.commit(&second_plan, &guards, &exact).unwrap();
        let second_id = second.event().unwrap().id();
        assert_ne!(first_id, second_id);
        assert_eq!(second_id.revision(), GuardStateRevision(3));
    }

    #[test]
    fn non_monotonic_sample_fails_without_revision_mutation() {
        let raw = policy();
        let guards = guards(&raw);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let owner = owner(&raw, &guards);
        let before = owner.authority_stamp().clone();
        let request = GuardAdvanceRequest::new(GuardAdvanceRequestId(9), sample(0, 11.0));
        assert!(matches!(
            owner.prepare(request, &guards, &exact),
            Err(GuardStateAuthorityError::EventGuard(
                EventGuardError::NonMonotonicSample { .. }
            ))
        ));
        assert_eq!(owner.authority_stamp(), &before);
    }
}
