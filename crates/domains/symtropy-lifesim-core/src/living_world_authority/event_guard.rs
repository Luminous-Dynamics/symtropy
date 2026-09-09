// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Canonical threshold-event guards with hysteresis.
//!
//! Periodic cadence alone can step across a future-bearing regime boundary.
//! This module adds registry-owned two-sample crossing semantics that report a
//! canonical tick bracket without pretending to reconstruct the exact continuous
//! crossing time. Triggered guards latch until an explicit hysteresis release
//! threshold is crossed, preventing numerical chatter from becoming history.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::information::ProcessKey;
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::InformationRegistryError;

use super::spatiotemporal_information::CanonicalTick;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventGuardRegistryKey {
    id: u128,
    version: u32,
}

impl EventGuardRegistryKey {
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
pub struct EventGuardKey {
    id: u128,
    version: u32,
}

impl EventGuardKey {
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

/// Opaque/versioned identity for the scalar observed by one or more guards.
///
/// A process-specific scalar such as shear margin or ignition index should not be
/// mislabeled as a generic [`crate::information::EcologicalInformation`] item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuardObservableKey {
    id: u128,
    version: u32,
}

impl GuardObservableKey {
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

/// Finite canonical scalar encoded by its normalized IEEE-754 bit pattern.
///
/// Storing bits gives exact `Eq`/ordering identity for policy and persisted guard
/// samples while still allowing ordinary finite `f64` comparisons during guard
/// evaluation. Negative zero is normalized to positive zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuardScalar(u64);

impl GuardScalar {
    pub fn new(value: f64) -> Result<Self, EventGuardError> {
        if !value.is_finite() {
            return Err(EventGuardError::NonFiniteScalar(value));
        }
        let normalized = if value == 0.0 { 0.0 } else { value };
        Ok(Self(normalized.to_bits()))
    }

    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuardDirection {
    Rising,
    Falling,
}

/// One guard's immutable canonical policy.
///
/// V0 requires a strict hysteresis band: rising guards require release < trigger;
/// falling guards require release > trigger. A zero-width band is deliberately
/// rejected instead of permitting a chatter-prone shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventGuardDefinition {
    key: EventGuardKey,
    process: ProcessKey,
    observable: GuardObservableKey,
    direction: GuardDirection,
    trigger: GuardScalar,
    release: GuardScalar,
}

impl EventGuardDefinition {
    pub fn new(
        key: EventGuardKey,
        process: ProcessKey,
        observable: GuardObservableKey,
        direction: GuardDirection,
        trigger: GuardScalar,
        release: GuardScalar,
    ) -> Result<Self, EventGuardError> {
        let valid = match direction {
            GuardDirection::Rising => release.get() < trigger.get(),
            GuardDirection::Falling => release.get() > trigger.get(),
        };
        if !valid {
            return Err(EventGuardError::InvalidHysteresis {
                guard: key,
                direction,
                trigger,
                release,
            });
        }
        Ok(Self {
            key,
            process,
            observable,
            direction,
            trigger,
            release,
        })
    }

    pub const fn key(self) -> EventGuardKey {
        self.key
    }

    pub const fn process(self) -> ProcessKey {
        self.process
    }

    pub const fn observable(self) -> GuardObservableKey {
        self.observable
    }

    pub const fn direction(self) -> GuardDirection {
        self.direction
    }

    pub const fn trigger(self) -> GuardScalar {
        self.trigger
    }

    pub const fn release(self) -> GuardScalar {
        self.release
    }

    fn active_at(self, value: GuardScalar) -> bool {
        match self.direction {
            GuardDirection::Rising => value.get() >= self.trigger.get(),
            GuardDirection::Falling => value.get() <= self.trigger.get(),
        }
    }

    fn trigger_crossed(self, previous: GuardScalar, current: GuardScalar) -> bool {
        match self.direction {
            GuardDirection::Rising => {
                previous.get() < self.trigger.get() && current.get() >= self.trigger.get()
            }
            GuardDirection::Falling => {
                previous.get() > self.trigger.get() && current.get() <= self.trigger.get()
            }
        }
    }

    fn release_reached(self, current: GuardScalar) -> bool {
        match self.direction {
            GuardDirection::Rising => current.get() <= self.release.get(),
            GuardDirection::Falling => current.get() >= self.release.get(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuardSample {
    tick: CanonicalTick,
    value: GuardScalar,
}

impl GuardSample {
    pub const fn new(tick: CanonicalTick, value: GuardScalar) -> Self {
        Self { tick, value }
    }

    pub fn from_value(tick: CanonicalTick, value: f64) -> Result<Self, EventGuardError> {
        Ok(Self::new(tick, GuardScalar::new(value)?))
    }

    pub const fn tick(self) -> CanonicalTick {
        self.tick
    }

    pub const fn value(self) -> GuardScalar {
        self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuardLatchState {
    Armed,
    Latched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuardTickBracket {
    start_exclusive: CanonicalTick,
    end_inclusive: CanonicalTick,
}

impl GuardTickBracket {
    pub const fn start_exclusive(self) -> CanonicalTick {
        self.start_exclusive
    }

    pub const fn end_inclusive(self) -> CanonicalTick {
        self.end_inclusive
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GuardOutcome {
    NoChange,
    Triggered { bracket: GuardTickBracket },
    Rearmed { bracket: GuardTickBracket },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventGuardAuthorityStamp {
    key: EventGuardRegistryKey,
    information_policy_authority: InformationPolicyAuthorityStamp,
    definitions: BTreeMap<EventGuardKey, EventGuardDefinition>,
}

impl EventGuardAuthorityStamp {
    pub const fn key(&self) -> EventGuardRegistryKey {
        self.key
    }

    pub const fn information_policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.information_policy_authority
    }

    pub fn definitions(&self) -> &BTreeMap<EventGuardKey, EventGuardDefinition> {
        &self.definitions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventGuardRegistryBuilder {
    key: EventGuardRegistryKey,
    definitions: BTreeMap<EventGuardKey, EventGuardDefinition>,
}

impl EventGuardRegistryBuilder {
    pub const fn new(key: EventGuardRegistryKey) -> Self {
        Self {
            key,
            definitions: BTreeMap::new(),
        }
    }

    pub fn register_guard(
        &mut self,
        definition: EventGuardDefinition,
    ) -> Result<(), EventGuardError> {
        let key = definition.key();
        if let Some(existing) = self.definitions.get(&key) {
            if existing == &definition {
                return Ok(());
            }
            return Err(EventGuardError::ConflictingGuardRegistration { guard: key });
        }
        self.definitions.insert(key, definition);
        Ok(())
    }

    /// Seal the guard corpus only after every guard's owning process resolves
    /// through the exact registered process-information policy.
    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<EventGuardRegistry, EventGuardError> {
        policy
            .authority_stamp()
            .validate_registry(policy.registry())
            .map_err(EventGuardError::PolicyIdentity)?;
        for definition in self.definitions.values().copied() {
            policy
                .registry()
                .resolve_process(definition.process())
                .map_err(EventGuardError::InformationRegistry)?;
        }

        let authority = EventGuardAuthorityStamp {
            key: self.key,
            information_policy_authority: policy.authority_stamp().clone(),
            definitions: self.definitions.clone(),
        };
        Ok(EventGuardRegistry {
            authority,
            definitions: self.definitions,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventGuardRegistry {
    authority: EventGuardAuthorityStamp,
    definitions: BTreeMap<EventGuardKey, EventGuardDefinition>,
}

impl EventGuardRegistry {
    pub const fn authority_stamp(&self) -> &EventGuardAuthorityStamp {
        &self.authority
    }

    pub fn definition(&self, guard: EventGuardKey) -> Option<EventGuardDefinition> {
        self.definitions.get(&guard).copied()
    }

    pub fn validate_current(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), EventGuardError> {
        self.authority
            .information_policy_authority
            .validate_registry(policy.registry())
            .map_err(EventGuardError::PolicyIdentity)?;
        if policy.authority_stamp() != &self.authority.information_policy_authority {
            return Err(EventGuardError::PolicyAuthorityChanged);
        }
        Ok(())
    }

    /// Initialize runtime latch state from a current canonical sample without
    /// fabricating a historical crossing. A world loaded already beyond the
    /// trigger starts latched; otherwise it starts armed.
    pub fn initialize_guard(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        guard: EventGuardKey,
        sample: GuardSample,
    ) -> Result<GuardRuntimeState, EventGuardError> {
        self.validate_current(policy)?;
        let definition = self
            .definition(guard)
            .ok_or(EventGuardError::UnknownGuard { guard })?;
        let latch = if definition.active_at(sample.value()) {
            GuardLatchState::Latched
        } else {
            GuardLatchState::Armed
        };
        Ok(GuardRuntimeState {
            authority: self.authority.clone(),
            guard,
            latch,
            last_sample: sample,
        })
    }

    /// Advance one guard from its authority-bound previous runtime state.
    ///
    /// The caller supplies only the new canonical sample. It cannot inject an
    /// arbitrary previous sample or armed boolean to manufacture repeated events.
    pub fn advance_guard(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        state: &GuardRuntimeState,
        current: GuardSample,
    ) -> Result<GuardEvaluation, EventGuardError> {
        self.validate_current(policy)?;
        if state.authority != self.authority {
            return Err(EventGuardError::GuardAuthorityChanged);
        }
        let definition = self
            .definition(state.guard)
            .ok_or(EventGuardError::UnknownGuard { guard: state.guard })?;
        if current.tick().0 <= state.last_sample.tick().0 {
            return Err(EventGuardError::NonMonotonicSample {
                previous: state.last_sample.tick(),
                current: current.tick(),
            });
        }

        let bracket = GuardTickBracket {
            start_exclusive: state.last_sample.tick(),
            end_inclusive: current.tick(),
        };

        let (latch, outcome) = match state.latch {
            GuardLatchState::Armed
                if definition.trigger_crossed(state.last_sample.value(), current.value()) =>
            {
                (GuardLatchState::Latched, GuardOutcome::Triggered { bracket })
            }
            GuardLatchState::Latched if definition.release_reached(current.value()) => {
                (GuardLatchState::Armed, GuardOutcome::Rearmed { bracket })
            }
            latch => (latch, GuardOutcome::NoChange),
        };

        Ok(GuardEvaluation {
            outcome,
            state: GuardRuntimeState {
                authority: self.authority.clone(),
                guard: state.guard,
                latch,
                last_sample: current,
            },
        })
    }
}

/// Runtime guard state is authority-bound and has no public constructor.
///
/// Higher orchestration may persist this object later, but ordinary callers cannot
/// forge `Armed` after a trigger to cause duplicate canonical events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardRuntimeState {
    authority: EventGuardAuthorityStamp,
    guard: EventGuardKey,
    latch: GuardLatchState,
    last_sample: GuardSample,
}

impl GuardRuntimeState {
    pub const fn authority_stamp(&self) -> &EventGuardAuthorityStamp {
        &self.authority
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
pub struct GuardEvaluation {
    outcome: GuardOutcome,
    state: GuardRuntimeState,
}

impl GuardEvaluation {
    pub const fn outcome(&self) -> GuardOutcome {
        self.outcome
    }

    pub const fn state(&self) -> &GuardRuntimeState {
        &self.state
    }

    pub fn into_state(self) -> GuardRuntimeState {
        self.state
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EventGuardError {
    NonFiniteScalar(f64),
    InvalidHysteresis {
        guard: EventGuardKey,
        direction: GuardDirection,
        trigger: GuardScalar,
        release: GuardScalar,
    },
    ConflictingGuardRegistration {
        guard: EventGuardKey,
    },
    InformationRegistry(InformationRegistryError),
    PolicyIdentity(InformationPolicyIdentityError),
    PolicyAuthorityChanged,
    GuardAuthorityChanged,
    UnknownGuard {
        guard: EventGuardKey,
    },
    NonMonotonicSample {
        previous: CanonicalTick,
        current: CanonicalTick,
    },
}

impl fmt::Display for EventGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteScalar(value) => {
                write!(formatter, "event-guard scalar must be finite, got {value}")
            }
            Self::InvalidHysteresis {
                guard,
                direction,
                trigger,
                release,
            } => write!(
                formatter,
                "guard {}@{} has invalid {direction:?} hysteresis: trigger {}, release {}",
                guard.id(),
                guard.version(),
                trigger.get(),
                release.get()
            ),
            Self::ConflictingGuardRegistration { guard } => write!(
                formatter,
                "guard {}@{} was registered with conflicting semantics",
                guard.id(),
                guard.version()
            ),
            Self::InformationRegistry(error) => {
                write!(formatter, "information registry error: {error}")
            }
            Self::PolicyIdentity(error) => {
                write!(formatter, "information-policy identity error: {error}")
            }
            Self::PolicyAuthorityChanged => {
                write!(formatter, "exact information-policy authority changed")
            }
            Self::GuardAuthorityChanged => write!(
                formatter,
                "runtime guard state was produced under different guard authority"
            ),
            Self::UnknownGuard { guard } => write!(
                formatter,
                "unknown event guard {}@{}",
                guard.id(),
                guard.version()
            ),
            Self::NonMonotonicSample { previous, current } => write!(
                formatter,
                "event-guard samples must advance canonical time: previous {}, current {}",
                previous.0,
                current.0
            ),
        }
    }
}

impl Error for EventGuardError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InformationRegistry(error) => Some(error),
            Self::PolicyIdentity(error) => Some(error),
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

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(9_000, 1);
    const PROCESS: ProcessKey = ProcessKey::new(9_010, 1);
    const UNKNOWN_PROCESS: ProcessKey = ProcessKey::new(9_011, 1);
    const REPRESENTATION: RepresentationKey = RepresentationKey::new(9_020, 1);
    const REGISTRY: EventGuardRegistryKey = EventGuardRegistryKey::new(9_030, 1);
    const GUARD: EventGuardKey = EventGuardKey::new(9_040, 1);
    const GUARD_2: EventGuardKey = EventGuardKey::new(9_041, 1);
    const OBSERVABLE: GuardObservableKey = GuardObservableKey::new(9_050, 1);

    fn policy(extra_process: bool) -> InformationPolicyRegistry {
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
        if extra_process {
            builder
                .register_process(ProcessInformationProfile::new(
                    UNKNOWN_PROCESS,
                    EcologicalAuthorityLevel::Coarse,
                    [ProcessInformationRequirement::exact(
                        EcologicalInformation::Headcount,
                    )],
                ))
                .unwrap();
        }
        builder
            .register_representation(RepresentationCapabilities::new(
                REPRESENTATION,
                EcologicalAuthorityLevel::Coarse,
                [(EcologicalInformation::Headcount, CapabilityEvidence::Exact)],
            ))
            .unwrap();
        builder.seal()
    }

    fn rising(guard: EventGuardKey, process: ProcessKey) -> EventGuardDefinition {
        EventGuardDefinition::new(
            guard,
            process,
            OBSERVABLE,
            GuardDirection::Rising,
            GuardScalar::new(10.0).unwrap(),
            GuardScalar::new(8.0).unwrap(),
        )
        .unwrap()
    }

    fn falling(guard: EventGuardKey, process: ProcessKey) -> EventGuardDefinition {
        EventGuardDefinition::new(
            guard,
            process,
            OBSERVABLE,
            GuardDirection::Falling,
            GuardScalar::new(2.0).unwrap(),
            GuardScalar::new(4.0).unwrap(),
        )
        .unwrap()
    }

    fn registry(
        policy: &InformationPolicyRegistry,
        definitions: impl IntoIterator<Item = EventGuardDefinition>,
    ) -> EventGuardRegistry {
        let exact = ManifestBoundInformationPolicyRegistry::new(policy);
        let mut builder = EventGuardRegistryBuilder::new(REGISTRY);
        for definition in definitions {
            builder.register_guard(definition).unwrap();
        }
        builder.seal(&exact).unwrap()
    }

    fn sample(tick: u64, value: f64) -> GuardSample {
        GuardSample::from_value(CanonicalTick(tick), value).unwrap()
    }

    #[test]
    fn rising_crossing_triggers_once_and_latches() {
        let raw = policy(false);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let registry = registry(&raw, [rising(GUARD, PROCESS)]);
        let state = registry.initialize_guard(&exact, GUARD, sample(10, 9.0)).unwrap();
        let first = registry
            .advance_guard(&exact, &state, sample(20, 11.0))
            .unwrap();
        assert!(matches!(first.outcome(), GuardOutcome::Triggered { .. }));
        assert_eq!(first.state().latch(), GuardLatchState::Latched);

        let second = registry
            .advance_guard(&exact, first.state(), sample(30, 12.0))
            .unwrap();
        assert_eq!(second.outcome(), GuardOutcome::NoChange);
        assert_eq!(second.state().latch(), GuardLatchState::Latched);
    }

    #[test]
    fn hysteresis_band_does_not_rearm_until_release() {
        let raw = policy(false);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let registry = registry(&raw, [rising(GUARD, PROCESS)]);
        let state = registry.initialize_guard(&exact, GUARD, sample(0, 9.0)).unwrap();
        let triggered = registry
            .advance_guard(&exact, &state, sample(1, 10.5))
            .unwrap();
        let in_band = registry
            .advance_guard(&exact, triggered.state(), sample(2, 9.0))
            .unwrap();
        assert_eq!(in_band.outcome(), GuardOutcome::NoChange);
        assert_eq!(in_band.state().latch(), GuardLatchState::Latched);

        let rearmed = registry
            .advance_guard(&exact, in_band.state(), sample(3, 8.0))
            .unwrap();
        assert!(matches!(rearmed.outcome(), GuardOutcome::Rearmed { .. }));
        assert_eq!(rearmed.state().latch(), GuardLatchState::Armed);

        let triggered_again = registry
            .advance_guard(&exact, rearmed.state(), sample(4, 10.1))
            .unwrap();
        assert!(matches!(
            triggered_again.outcome(),
            GuardOutcome::Triggered { .. }
        ));
    }

    #[test]
    fn falling_guard_is_symmetric() {
        let raw = policy(false);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let registry = registry(&raw, [falling(GUARD, PROCESS)]);
        let state = registry.initialize_guard(&exact, GUARD, sample(0, 3.0)).unwrap();
        let triggered = registry
            .advance_guard(&exact, &state, sample(5, 1.0))
            .unwrap();
        assert!(matches!(triggered.outcome(), GuardOutcome::Triggered { .. }));
        let still_latched = registry
            .advance_guard(&exact, triggered.state(), sample(6, 3.0))
            .unwrap();
        assert_eq!(still_latched.outcome(), GuardOutcome::NoChange);
        let rearmed = registry
            .advance_guard(&exact, still_latched.state(), sample(7, 4.0))
            .unwrap();
        assert!(matches!(rearmed.outcome(), GuardOutcome::Rearmed { .. }));
    }

    #[test]
    fn initialization_beyond_trigger_does_not_fabricate_event() {
        let raw = policy(false);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let registry = registry(&raw, [rising(GUARD, PROCESS)]);
        let state = registry.initialize_guard(&exact, GUARD, sample(100, 12.0)).unwrap();
        assert_eq!(state.latch(), GuardLatchState::Latched);
        let next = registry
            .advance_guard(&exact, &state, sample(101, 13.0))
            .unwrap();
        assert_eq!(next.outcome(), GuardOutcome::NoChange);
    }

    #[test]
    fn trigger_result_carries_canonical_crossing_bracket() {
        let raw = policy(false);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let registry = registry(&raw, [rising(GUARD, PROCESS)]);
        let state = registry.initialize_guard(&exact, GUARD, sample(40, 1.0)).unwrap();
        let result = registry
            .advance_guard(&exact, &state, sample(55, 20.0))
            .unwrap();
        let GuardOutcome::Triggered { bracket } = result.outcome() else {
            panic!("expected trigger");
        };
        assert_eq!(bracket.start_exclusive(), CanonicalTick(40));
        assert_eq!(bracket.end_inclusive(), CanonicalTick(55));
    }

    #[test]
    fn invalid_hysteresis_and_non_finite_values_reject() {
        assert!(matches!(
            GuardScalar::new(f64::NAN),
            Err(EventGuardError::NonFiniteScalar(_))
        ));
        assert!(matches!(
            EventGuardDefinition::new(
                GUARD,
                PROCESS,
                OBSERVABLE,
                GuardDirection::Rising,
                GuardScalar::new(10.0).unwrap(),
                GuardScalar::new(10.0).unwrap(),
            ),
            Err(EventGuardError::InvalidHysteresis { .. })
        ));
    }

    #[test]
    fn non_monotonic_or_equal_tick_rejects() {
        let raw = policy(false);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let registry = registry(&raw, [rising(GUARD, PROCESS)]);
        let state = registry.initialize_guard(&exact, GUARD, sample(10, 9.0)).unwrap();
        assert!(matches!(
            registry.advance_guard(&exact, &state, sample(10, 11.0)),
            Err(EventGuardError::NonMonotonicSample { .. })
        ));
        assert!(matches!(
            registry.advance_guard(&exact, &state, sample(9, 11.0)),
            Err(EventGuardError::NonMonotonicSample { .. })
        ));
    }

    #[test]
    fn unknown_process_cannot_enter_guard_authority() {
        let raw = policy(false);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let mut builder = EventGuardRegistryBuilder::new(REGISTRY);
        builder
            .register_guard(rising(GUARD, UNKNOWN_PROCESS))
            .unwrap();
        assert!(matches!(
            builder.seal(&exact),
            Err(EventGuardError::InformationRegistry(_))
        ));
    }

    #[test]
    fn changed_exact_process_policy_stales_guard_registry_and_state() {
        let raw = policy(false);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let registry = registry(&raw, [rising(GUARD, PROCESS)]);
        let state = registry.initialize_guard(&exact, GUARD, sample(0, 9.0)).unwrap();

        let changed = policy(true);
        let changed_exact = ManifestBoundInformationPolicyRegistry::new(&changed);
        assert!(registry.validate_current(&changed_exact).is_err());
        assert!(registry
            .advance_guard(&changed_exact, &state, sample(1, 11.0))
            .is_err());
    }

    #[test]
    fn definition_order_is_canonical() {
        let raw = policy(false);
        let exact = ManifestBoundInformationPolicyRegistry::new(&raw);
        let first = {
            let mut builder = EventGuardRegistryBuilder::new(REGISTRY);
            builder.register_guard(rising(GUARD, PROCESS)).unwrap();
            builder.register_guard(falling(GUARD_2, PROCESS)).unwrap();
            builder.seal(&exact).unwrap()
        };
        let second = {
            let mut builder = EventGuardRegistryBuilder::new(REGISTRY);
            builder.register_guard(falling(GUARD_2, PROCESS)).unwrap();
            builder.register_guard(rising(GUARD, PROCESS)).unwrap();
            builder.seal(&exact).unwrap()
        };
        assert_eq!(first.authority_stamp(), second.authority_stamp());
    }
}
