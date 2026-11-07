// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Physics-independent deterministic actor policies for synthetic road scenarios.
//!
//! The policy boundary has no simulator, controller, wall clock, filesystem,
//! network, or ambient random-number generator. Policies can only react to a
//! validated, timestamped allow-list of actor-visible cues. This crate does not
//! model human behaviour and makes no vehicle-safety claims.

#![forbid(unsafe_code)]

use std::fmt;

/// Authoritative integer scenario time. Wall-clock time is never policy input.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScenarioTick(pub u64);

/// Stable scenario-level actor identity, independent of runtime body handles.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScenarioActorId(String);

impl ScenarioActorId {
    pub const MAX_LEN: usize = 96;

    /// Accepts a bounded ASCII identifier with an alphanumeric first byte.
    /// Runtime body handles must never be used as scenario identity.
    pub fn new(value: impl Into<String>) -> Result<Self, ActorIdError> {
        let value = value.into();
        if value.is_empty() {
            return Err(ActorIdError::Empty);
        }
        if value.len() > Self::MAX_LEN {
            return Err(ActorIdError::TooLong);
        }
        if !value.is_ascii() {
            return Err(ActorIdError::NonAscii);
        }
        let bytes = value.as_bytes();
        if !bytes[0].is_ascii_alphanumeric() {
            return Err(ActorIdError::InvalidStartCharacter);
        }
        if !bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-' | b'.' | b':'))
        {
            return Err(ActorIdError::InvalidCharacter);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorIdError {
    Empty,
    TooLong,
    NonAscii,
    InvalidStartCharacter,
    InvalidCharacter,
}

impl fmt::Display for ActorIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ActorIdError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorMode {
    Waiting,
    Crossing,
    Paused,
    Complete,
}

/// Actor-local policy state. The speed is a requested setpoint, not measured
/// body velocity; the physics adapter must rate-limit it under the scenario's
/// acceleration/deceleration profile when applying kinematic motion. The fields
/// are private so adapters use validated states rather than arbitrary values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorPolicyState {
    mode: ActorMode,
    target_forward_speed_mps: f64,
    last_decision_tick: Option<ScenarioTick>,
}

impl ActorPolicyState {
    pub const fn waiting() -> Self {
        Self {
            mode: ActorMode::Waiting,
            target_forward_speed_mps: 0.0,
            last_decision_tick: None,
        }
    }

    pub const fn mode(self) -> ActorMode {
        self.mode
    }
    pub const fn target_forward_speed_mps(self) -> f64 {
        self.target_forward_speed_mps
    }
    pub const fn last_decision_tick(self) -> Option<ScenarioTick> {
        self.last_decision_tick
    }

    fn new(mode: ActorMode, target_forward_speed_mps: f64) -> Self {
        Self {
            mode,
            target_forward_speed_mps,
            last_decision_tick: None,
        }
    }

    /// Restores a scenario-authored or replayed actor state after validating
    /// the policy speed setpoint against the declared profile bounds.
    pub fn from_scenario(
        mode: ActorMode,
        target_forward_speed_mps: f64,
        last_decision_tick: Option<ScenarioTick>,
        bounds: MotionBounds,
    ) -> Result<Self, PolicyError> {
        let mut state = Self::new(mode, target_forward_speed_mps);
        state.last_decision_tick = last_decision_tick;
        state.validate(bounds)?;
        Ok(state)
    }

    fn validate(self, bounds: MotionBounds) -> Result<(), PolicyError> {
        if !self.target_forward_speed_mps.is_finite()
            || self.target_forward_speed_mps < 0.0
            || self.target_forward_speed_mps > bounds.max_forward_speed_mps
        {
            return Err(PolicyError::InvalidActorState);
        }
        if matches!(
            self.mode,
            ActorMode::Waiting | ActorMode::Paused | ActorMode::Complete
        ) && self.target_forward_speed_mps != 0.0
        {
            return Err(PolicyError::InvalidActorState);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CueKind {
    EgoRangeMetres,
    EgoSpeedMetresPerSecond,
    EgoTimeToConflictSeconds,
    CrossingSignal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrossingSignal {
    Walk,
    DontWalk,
    Unknown,
}

/// Unit-bearing values prevent treating metres as seconds or vice versa.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CueValue {
    DistanceMetres(f64),
    SpeedMetresPerSecond(f64),
    TimeToConflictSeconds(f64),
    CrossingSignal(CrossingSignal),
}

impl CueValue {
    fn matches(self, kind: CueKind) -> bool {
        matches!(
            (kind, self),
            (CueKind::EgoRangeMetres, Self::DistanceMetres(_))
                | (
                    CueKind::EgoSpeedMetresPerSecond,
                    Self::SpeedMetresPerSecond(_)
                )
                | (
                    CueKind::EgoTimeToConflictSeconds,
                    Self::TimeToConflictSeconds(_)
                )
                | (CueKind::CrossingSignal, Self::CrossingSignal(_))
        )
    }

    fn validate(self) -> Result<(), ObservationError> {
        let number = match self {
            Self::DistanceMetres(value)
            | Self::SpeedMetresPerSecond(value)
            | Self::TimeToConflictSeconds(value) => value,
            Self::CrossingSignal(_) => return Ok(()),
        };
        if !number.is_finite() {
            return Err(ObservationError::NonFiniteCue);
        }
        if number < 0.0 {
            return Err(ObservationError::NegativeCue);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CueStatus {
    Observed,
    Stale,
    Missing,
    Occluded,
    Unsupported,
}

/// Provenance linking a policy cue to a stable source registry and event ID.
/// These identifiers provide traceability, not a claim that the source is true.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationProvenance {
    source_id: std::num::NonZeroU64,
    event_id: std::num::NonZeroU64,
}

impl ObservationProvenance {
    pub fn new(source_id: u64, event_id: u64) -> Option<Self> {
        Some(Self {
            source_id: std::num::NonZeroU64::new(source_id)?,
            event_id: std::num::NonZeroU64::new(event_id)?,
        })
    }

    pub const fn source_id(self) -> u64 {
        self.source_id.get()
    }

    pub const fn event_id(self) -> u64 {
        self.event_id.get()
    }
}

/// A cue's source observation time and actor-delivery time are distinct.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObservationCue {
    kind: CueKind,
    provenance: ObservationProvenance,
    status: CueStatus,
    value: Option<CueValue>,
    observed_tick: Option<ScenarioTick>,
    delivered_tick: ScenarioTick,
}

impl ObservationCue {
    pub fn observed(
        kind: CueKind,
        provenance: ObservationProvenance,
        value: CueValue,
        observed_tick: ScenarioTick,
        delivered_tick: ScenarioTick,
    ) -> Self {
        Self {
            kind,
            provenance,
            status: CueStatus::Observed,
            value: Some(value),
            observed_tick: Some(observed_tick),
            delivered_tick,
        }
    }

    pub fn stale(
        kind: CueKind,
        provenance: ObservationProvenance,
        last_value: CueValue,
        observed_tick: ScenarioTick,
        delivered_tick: ScenarioTick,
    ) -> Self {
        Self {
            kind,
            provenance,
            status: CueStatus::Stale,
            value: Some(last_value),
            observed_tick: Some(observed_tick),
            delivered_tick,
        }
    }

    pub const fn unavailable(
        kind: CueKind,
        provenance: ObservationProvenance,
        status: CueStatus,
        delivered_tick: ScenarioTick,
    ) -> Self {
        Self {
            kind,
            provenance,
            status,
            value: None,
            observed_tick: None,
            delivered_tick,
        }
    }

    pub const fn kind(self) -> CueKind {
        self.kind
    }
    pub const fn provenance(self) -> ObservationProvenance {
        self.provenance
    }
    pub const fn status(self) -> CueStatus {
        self.status
    }
    pub const fn value(self) -> Option<CueValue> {
        self.value
    }
    pub const fn observed_tick(self) -> Option<ScenarioTick> {
        self.observed_tick
    }
    pub const fn delivered_tick(self) -> ScenarioTick {
        self.delivered_tick
    }

    fn validate(self, decision_tick: ScenarioTick) -> Result<(), ObservationError> {
        if self.delivered_tick > decision_tick {
            return Err(ObservationError::UndeliveredCue);
        }
        match self.status {
            CueStatus::Observed | CueStatus::Stale => {
                let value = self.value.ok_or(ObservationError::MissingCueValue)?;
                let observed = self
                    .observed_tick
                    .ok_or(ObservationError::MissingObservationTime)?;
                if observed > self.delivered_tick {
                    return Err(ObservationError::ObservationAfterDelivery);
                }
                if !value.matches(self.kind) {
                    return Err(ObservationError::CueKindValueMismatch);
                }
                value.validate()?;
            }
            CueStatus::Missing | CueStatus::Occluded | CueStatus::Unsupported => {
                if self.value.is_some() || self.observed_tick.is_some() {
                    return Err(ObservationError::UnavailableCueHasValue);
                }
            }
        }
        Ok(())
    }
}

/// Closed, validated, allow-listed observation view for one actor and tick.
#[derive(Clone, Debug, PartialEq)]
pub struct AllowedActorObservation {
    tick: ScenarioTick,
    cues: Vec<ObservationCue>,
}

impl AllowedActorObservation {
    pub const MAX_CUES: usize = 16;

    pub fn new(
        tick: ScenarioTick,
        mut cues: Vec<ObservationCue>,
    ) -> Result<Self, ObservationError> {
        if cues.len() > Self::MAX_CUES {
            return Err(ObservationError::TooManyCues);
        }
        cues.sort_by_key(|cue| cue.kind);
        for cue in &cues {
            cue.validate(tick)?;
        }
        for pair in cues.windows(2) {
            if pair[0].kind == pair[1].kind {
                return Err(ObservationError::DuplicateCue(pair[0].kind));
            }
        }
        Ok(Self { tick, cues })
    }

    pub const fn tick(&self) -> ScenarioTick {
        self.tick
    }
    pub fn cues(&self) -> &[ObservationCue] {
        &self.cues
    }

    pub fn cue(&self, kind: CueKind) -> Option<ObservationCue> {
        self.cues
            .binary_search_by_key(&kind, |cue| cue.kind)
            .ok()
            .map(|index| self.cues[index])
    }

    /// TTC is usable only when explicitly observed and past delivery/reaction
    /// delay. Stale, missing, occluded and unsupported remain unavailable.
    fn observed_ttc_after_delay(&self, reaction_delay_ticks: u64) -> Option<f64> {
        let cue = self.cue(CueKind::EgoTimeToConflictSeconds)?;
        if cue.status != CueStatus::Observed {
            return None;
        }
        let elapsed = self.tick.0.checked_sub(cue.delivered_tick.0)?;
        if elapsed < reaction_delay_ticks {
            return None;
        }
        match cue.value {
            Some(CueValue::TimeToConflictSeconds(seconds)) => Some(seconds),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionBounds {
    max_forward_speed_mps: f64,
}

impl MotionBounds {
    pub fn new(max_forward_speed_mps: f64) -> Result<Self, PolicyError> {
        if !max_forward_speed_mps.is_finite() || max_forward_speed_mps <= 0.0 {
            return Err(PolicyError::InvalidConfiguration);
        }
        Ok(Self {
            max_forward_speed_mps,
        })
    }

    pub const fn max_forward_speed_mps(self) -> f64 {
        self.max_forward_speed_mps
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActorAction {
    Wait,
    EnterCrossing,
    Continue,
    Pause,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionReason {
    NoFreshTtc,
    EntryGapTooSmall,
    EntryGapAccepted,
    ObservedConflict,
    ConflictCleared,
    CommittedAfterEntry,
    AlreadyComplete,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorTransition {
    action: ActorAction,
    reason: TransitionReason,
    next_state: ActorPolicyState,
}

impl ActorTransition {
    const fn new(
        action: ActorAction,
        reason: TransitionReason,
        next_state: ActorPolicyState,
    ) -> Self {
        Self {
            action,
            reason,
            next_state,
        }
    }
    pub const fn action(self) -> ActorAction {
        self.action
    }
    pub const fn reason(self) -> TransitionReason {
        self.reason
    }
    pub const fn next_state(self) -> ActorPolicyState {
        self.next_state
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationError {
    TooManyCues,
    DuplicateCue(CueKind),
    UndeliveredCue,
    ObservationAfterDelivery,
    MissingCueValue,
    MissingObservationTime,
    UnavailableCueHasValue,
    CueKindValueMismatch,
    NonFiniteCue,
    NegativeCue,
}

impl fmt::Display for ObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ObservationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyError {
    InvalidConfiguration,
    InvalidActorState,
    NonMonotonicTick,
}
impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PolicyError {}

/// Pure policy API: only actor-local state and validated delivered cues enter.
pub trait ActorPolicy {
    fn profile_id(&self) -> &'static str;
    fn transition(
        &self,
        prior: ActorPolicyState,
        observation: &AllowedActorObservation,
    ) -> Result<ActorTransition, PolicyError>;
}

/// Cautious synthetic profile: wait for a sufficiently large entry gap and
/// pause a crossing actor when a fresh observed TTC is at/below the threshold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CautiousPausePolicy {
    bounds: MotionBounds,
    pause_ttc_seconds: f64,
    entry_ttc_seconds: f64,
    reaction_delay_ticks: u64,
}

impl CautiousPausePolicy {
    pub const PROFILE_ID: &'static str = "cautious_pause_v1";

    pub fn new(
        bounds: MotionBounds,
        pause_ttc_seconds: f64,
        entry_ttc_seconds: f64,
        reaction_delay_ticks: u64,
    ) -> Result<Self, PolicyError> {
        if !pause_ttc_seconds.is_finite()
            || !entry_ttc_seconds.is_finite()
            || pause_ttc_seconds < 0.0
            || entry_ttc_seconds < pause_ttc_seconds
        {
            return Err(PolicyError::InvalidConfiguration);
        }
        Ok(Self {
            bounds,
            pause_ttc_seconds,
            entry_ttc_seconds,
            reaction_delay_ticks,
        })
    }
}

impl ActorPolicy for CautiousPausePolicy {
    fn profile_id(&self) -> &'static str {
        Self::PROFILE_ID
    }

    fn transition(
        &self,
        prior: ActorPolicyState,
        observation: &AllowedActorObservation,
    ) -> Result<ActorTransition, PolicyError> {
        prior.validate(self.bounds)?;
        if prior
            .last_decision_tick
            .is_some_and(|last| observation.tick <= last)
        {
            return Err(PolicyError::NonMonotonicTick);
        }
        let ttc = observation.observed_ttc_after_delay(self.reaction_delay_ticks);
        let mut result = match prior.mode {
            ActorMode::Waiting => match ttc {
                Some(seconds) if seconds >= self.entry_ttc_seconds => ActorTransition::new(
                    ActorAction::EnterCrossing,
                    TransitionReason::EntryGapAccepted,
                    ActorPolicyState::new(ActorMode::Crossing, self.bounds.max_forward_speed_mps),
                ),
                Some(_) => ActorTransition::new(
                    ActorAction::Wait,
                    TransitionReason::EntryGapTooSmall,
                    ActorPolicyState::waiting(),
                ),
                None => ActorTransition::new(
                    ActorAction::Wait,
                    TransitionReason::NoFreshTtc,
                    ActorPolicyState::waiting(),
                ),
            },
            ActorMode::Crossing => match ttc {
                Some(seconds) if seconds <= self.pause_ttc_seconds => ActorTransition::new(
                    ActorAction::Pause,
                    TransitionReason::ObservedConflict,
                    ActorPolicyState::new(ActorMode::Paused, 0.0),
                ),
                Some(_) => ActorTransition::new(
                    ActorAction::Continue,
                    TransitionReason::ConflictCleared,
                    ActorPolicyState::new(ActorMode::Crossing, self.bounds.max_forward_speed_mps),
                ),
                None => {
                    ActorTransition::new(ActorAction::Continue, TransitionReason::NoFreshTtc, prior)
                }
            },
            ActorMode::Paused => match ttc {
                Some(seconds) if seconds > self.pause_ttc_seconds => ActorTransition::new(
                    ActorAction::Continue,
                    TransitionReason::ConflictCleared,
                    ActorPolicyState::new(ActorMode::Crossing, self.bounds.max_forward_speed_mps),
                ),
                Some(_) => ActorTransition::new(
                    ActorAction::Pause,
                    TransitionReason::ObservedConflict,
                    prior,
                ),
                None => {
                    ActorTransition::new(ActorAction::Pause, TransitionReason::NoFreshTtc, prior)
                }
            },
            ActorMode::Complete => ActorTransition::new(
                ActorAction::Complete,
                TransitionReason::AlreadyComplete,
                prior,
            ),
        };
        result.next_state.last_decision_tick = Some(observation.tick);
        Ok(result)
    }
}

/// Stylized baseline: wait for the declared entry gap, then continue after
/// entering. It is intentionally not a model of human behaviour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CommittedAfterEntryPolicy {
    bounds: MotionBounds,
    entry_ttc_seconds: f64,
    reaction_delay_ticks: u64,
}

impl CommittedAfterEntryPolicy {
    pub const PROFILE_ID: &'static str = "committed_after_entry_v1";

    pub fn new(
        bounds: MotionBounds,
        entry_ttc_seconds: f64,
        reaction_delay_ticks: u64,
    ) -> Result<Self, PolicyError> {
        if !entry_ttc_seconds.is_finite() || entry_ttc_seconds < 0.0 {
            return Err(PolicyError::InvalidConfiguration);
        }
        Ok(Self {
            bounds,
            entry_ttc_seconds,
            reaction_delay_ticks,
        })
    }
}

impl ActorPolicy for CommittedAfterEntryPolicy {
    fn profile_id(&self) -> &'static str {
        Self::PROFILE_ID
    }

    fn transition(
        &self,
        prior: ActorPolicyState,
        observation: &AllowedActorObservation,
    ) -> Result<ActorTransition, PolicyError> {
        prior.validate(self.bounds)?;
        if prior
            .last_decision_tick
            .is_some_and(|last| observation.tick <= last)
        {
            return Err(PolicyError::NonMonotonicTick);
        }
        let mut result = match prior.mode {
            ActorMode::Waiting => match observation
                .observed_ttc_after_delay(self.reaction_delay_ticks)
            {
                Some(seconds) if seconds > self.entry_ttc_seconds => ActorTransition::new(
                    ActorAction::EnterCrossing,
                    TransitionReason::EntryGapAccepted,
                    ActorPolicyState::new(ActorMode::Crossing, self.bounds.max_forward_speed_mps),
                ),
                Some(_) => ActorTransition::new(
                    ActorAction::Wait,
                    TransitionReason::EntryGapTooSmall,
                    ActorPolicyState::waiting(),
                ),
                None => ActorTransition::new(
                    ActorAction::Wait,
                    TransitionReason::NoFreshTtc,
                    ActorPolicyState::waiting(),
                ),
            },
            ActorMode::Crossing | ActorMode::Paused => ActorTransition::new(
                ActorAction::Continue,
                TransitionReason::CommittedAfterEntry,
                ActorPolicyState::new(ActorMode::Crossing, self.bounds.max_forward_speed_mps),
            ),
            ActorMode::Complete => ActorTransition::new(
                ActorAction::Complete,
                TransitionReason::AlreadyComplete,
                prior,
            ),
        };
        result.next_state.last_decision_tick = Some(observation.tick);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> MotionBounds {
        MotionBounds::new(1.5).unwrap()
    }
    fn cautious() -> CautiousPausePolicy {
        CautiousPausePolicy::new(bounds(), 2.5, 3.5, 1).unwrap()
    }
    fn committed() -> CommittedAfterEntryPolicy {
        CommittedAfterEntryPolicy::new(bounds(), 1.5, 1).unwrap()
    }
    fn provenance(event_id: u64) -> ObservationProvenance {
        ObservationProvenance::new(1, event_id).unwrap()
    }

    fn ttc_view(tick: u64, status: CueStatus, value: Option<f64>) -> AllowedActorObservation {
        let cue = match (status, value) {
            (CueStatus::Observed, Some(v)) => {
                let delivered = ScenarioTick(tick.saturating_sub(1));
                ObservationCue::observed(
                    CueKind::EgoTimeToConflictSeconds,
                    provenance(701),
                    CueValue::TimeToConflictSeconds(v),
                    delivered,
                    delivered,
                )
            }
            (CueStatus::Stale, Some(v)) => ObservationCue::stale(
                CueKind::EgoTimeToConflictSeconds,
                provenance(702),
                CueValue::TimeToConflictSeconds(v),
                ScenarioTick(tick.saturating_sub(2)),
                ScenarioTick(tick.saturating_sub(1)),
            ),
            (kind, None) => ObservationCue::unavailable(
                CueKind::EgoTimeToConflictSeconds,
                provenance(703),
                kind,
                ScenarioTick(tick),
            ),
            _ => panic!("invalid test fixture"),
        };
        AllowedActorObservation::new(ScenarioTick(tick), vec![cue]).unwrap()
    }

    #[test]
    fn observation_provenance_requires_nonzero_source_and_event_ids() {
        assert_eq!(ObservationProvenance::new(0, 1), None);
        assert_eq!(ObservationProvenance::new(1, 0), None);
        let source_event = provenance(7);
        let cue = ObservationCue::unavailable(
            CueKind::EgoTimeToConflictSeconds,
            source_event,
            CueStatus::Missing,
            ScenarioTick(8),
        );
        assert_eq!(cue.provenance().source_id(), 1);
        assert_eq!(cue.provenance().event_id(), 7);
    }

    #[test]
    fn scenario_actor_ids_are_stable_ascii_and_bounded() {
        let id = ScenarioActorId::new("pedestrian_001").unwrap();
        assert_eq!(id.as_str(), "pedestrian_001");
        assert_eq!(id, ScenarioActorId::new("pedestrian_001").unwrap());
        assert_eq!(ScenarioActorId::new(""), Err(ActorIdError::Empty));
        assert_eq!(
            ScenarioActorId::new("pédestrian"),
            Err(ActorIdError::NonAscii)
        );
        assert_eq!(
            ScenarioActorId::new("_ego"),
            Err(ActorIdError::InvalidStartCharacter)
        );
        assert_eq!(
            ScenarioActorId::new("ego/0"),
            Err(ActorIdError::InvalidCharacter)
        );
        assert_eq!(
            ScenarioActorId::new("a".repeat(ScenarioActorId::MAX_LEN + 1)),
            Err(ActorIdError::TooLong)
        );
    }

    #[test]
    fn cues_are_sorted_and_duplicate_kinds_rejected() {
        let view = AllowedActorObservation::new(
            ScenarioTick(2),
            vec![
                ObservationCue::observed(
                    CueKind::EgoSpeedMetresPerSecond,
                    provenance(704),
                    CueValue::SpeedMetresPerSecond(4.0),
                    ScenarioTick(2),
                    ScenarioTick(2),
                ),
                ObservationCue::observed(
                    CueKind::EgoRangeMetres,
                    provenance(705),
                    CueValue::DistanceMetres(8.0),
                    ScenarioTick(2),
                    ScenarioTick(2),
                ),
            ],
        )
        .unwrap();
        assert_eq!(view.cues()[0].kind(), CueKind::EgoRangeMetres);
        let duplicate = vec![
            ObservationCue::unavailable(
                CueKind::EgoRangeMetres,
                provenance(706),
                CueStatus::Missing,
                ScenarioTick(2),
            ),
            ObservationCue::unavailable(
                CueKind::EgoRangeMetres,
                provenance(707),
                CueStatus::Occluded,
                ScenarioTick(2),
            ),
        ];
        assert_eq!(
            AllowedActorObservation::new(ScenarioTick(2), duplicate),
            Err(ObservationError::DuplicateCue(CueKind::EgoRangeMetres))
        );
    }

    #[test]
    fn missing_occluded_and_unsupported_are_distinct_without_numeric_values() {
        for status in [
            CueStatus::Missing,
            CueStatus::Occluded,
            CueStatus::Unsupported,
        ] {
            let view = ttc_view(5, status, None);
            let cue = view.cue(CueKind::EgoTimeToConflictSeconds).unwrap();
            assert_eq!(cue.status(), status);
            assert_eq!(cue.value(), None);
        }
    }

    #[test]
    fn future_undelivered_and_observation_after_delivery_are_rejected() {
        let cue = ObservationCue::observed(
            CueKind::EgoRangeMetres,
            provenance(708),
            CueValue::DistanceMetres(8.0),
            ScenarioTick(4),
            ScenarioTick(6),
        );
        assert_eq!(
            AllowedActorObservation::new(ScenarioTick(5), vec![cue]),
            Err(ObservationError::UndeliveredCue)
        );
        let cue = ObservationCue::observed(
            CueKind::EgoRangeMetres,
            provenance(709),
            CueValue::DistanceMetres(8.0),
            ScenarioTick(6),
            ScenarioTick(5),
        );
        assert_eq!(
            AllowedActorObservation::new(ScenarioTick(6), vec![cue]),
            Err(ObservationError::ObservationAfterDelivery)
        );
    }

    #[test]
    fn non_finite_negative_and_unit_mismatched_cues_are_rejected() {
        for number in [f64::NAN, f64::INFINITY, -0.1] {
            let cue = ObservationCue::observed(
                CueKind::EgoRangeMetres,
                provenance(710),
                CueValue::DistanceMetres(number),
                ScenarioTick(1),
                ScenarioTick(1),
            );
            assert!(AllowedActorObservation::new(ScenarioTick(1), vec![cue]).is_err());
        }
        let cue = ObservationCue::observed(
            CueKind::EgoRangeMetres,
            provenance(711),
            CueValue::TimeToConflictSeconds(1.0),
            ScenarioTick(1),
            ScenarioTick(1),
        );
        assert_eq!(
            AllowedActorObservation::new(ScenarioTick(1), vec![cue]),
            Err(ObservationError::CueKindValueMismatch)
        );
    }

    #[test]
    fn reaction_delay_blocks_response_until_one_tick_after_delivery() {
        let policy = cautious();
        let cue = ObservationCue::observed(
            CueKind::EgoTimeToConflictSeconds,
            provenance(712),
            CueValue::TimeToConflictSeconds(8.0),
            ScenarioTick(4),
            ScenarioTick(5),
        );
        let early = AllowedActorObservation::new(ScenarioTick(5), vec![cue]).unwrap();
        assert_eq!(
            policy
                .transition(ActorPolicyState::waiting(), &early)
                .unwrap()
                .action(),
            ActorAction::Wait
        );
        let later = AllowedActorObservation::new(ScenarioTick(6), vec![cue]).unwrap();
        assert_eq!(
            policy
                .transition(ActorPolicyState::waiting(), &later)
                .unwrap()
                .action(),
            ActorAction::EnterCrossing
        );
    }

    #[test]
    fn stale_missing_occluded_and_unsupported_ttc_never_become_fresh() {
        let policy = cautious();
        for status in [
            CueStatus::Stale,
            CueStatus::Missing,
            CueStatus::Occluded,
            CueStatus::Unsupported,
        ] {
            let value = (status == CueStatus::Stale).then_some(8.0);
            let result = policy
                .transition(ActorPolicyState::waiting(), &ttc_view(10, status, value))
                .unwrap();
            assert_eq!(result.action(), ActorAction::Wait);
            assert_eq!(result.reason(), TransitionReason::NoFreshTtc);
        }
    }

    #[test]
    fn cautious_profile_waits_enters_pauses_then_resumes() {
        let policy = cautious();
        assert_eq!(
            policy
                .transition(
                    ActorPolicyState::waiting(),
                    &ttc_view(10, CueStatus::Observed, Some(3.4))
                )
                .unwrap()
                .action(),
            ActorAction::Wait
        );
        let entered = policy
            .transition(
                ActorPolicyState::waiting(),
                &ttc_view(12, CueStatus::Observed, Some(3.5)),
            )
            .unwrap();
        assert_eq!(entered.action(), ActorAction::EnterCrossing);
        let paused = policy
            .transition(
                entered.next_state(),
                &ttc_view(14, CueStatus::Observed, Some(2.5)),
            )
            .unwrap();
        assert_eq!(paused.action(), ActorAction::Pause);
        assert_eq!(paused.next_state().mode(), ActorMode::Paused);
        let resumed = policy
            .transition(
                paused.next_state(),
                &ttc_view(16, CueStatus::Observed, Some(2.6)),
            )
            .unwrap();
        assert_eq!(resumed.action(), ActorAction::Continue);
        assert_eq!(resumed.next_state().mode(), ActorMode::Crossing);
    }

    #[test]
    fn committed_profile_does_not_reconsider_after_entry() {
        let policy = committed();
        let entered = policy
            .transition(
                ActorPolicyState::waiting(),
                &ttc_view(2, CueStatus::Observed, Some(1.6)),
            )
            .unwrap();
        let continued = policy
            .transition(
                entered.next_state(),
                &ttc_view(3, CueStatus::Observed, Some(0.2)),
            )
            .unwrap();
        assert_eq!(continued.action(), ActorAction::Continue);
        assert_eq!(continued.reason(), TransitionReason::CommittedAfterEntry);
    }

    #[test]
    fn profiles_differ_on_the_same_crossing_observation() {
        let prior = ActorPolicyState::new(ActorMode::Crossing, 1.0);
        let view = ttc_view(20, CueStatus::Observed, Some(2.0));
        assert_eq!(
            cautious().transition(prior, &view).unwrap().action(),
            ActorAction::Pause
        );
        assert_eq!(
            committed().transition(prior, &view).unwrap().action(),
            ActorAction::Continue
        );
    }

    #[test]
    fn complete_is_terminal_and_invalid_bounds_or_states_are_rejected() {
        let view = ttc_view(20, CueStatus::Missing, None);
        let done = ActorPolicyState::new(ActorMode::Complete, 0.0);
        assert_eq!(
            cautious().transition(done, &view).unwrap().action(),
            ActorAction::Complete
        );
        assert_eq!(
            committed().transition(done, &view).unwrap().action(),
            ActorAction::Complete
        );
        assert_eq!(
            MotionBounds::new(0.0),
            Err(PolicyError::InvalidConfiguration)
        );
        assert_eq!(
            CautiousPausePolicy::new(bounds(), 4.0, 3.0, 1),
            Err(PolicyError::InvalidConfiguration)
        );
        assert_eq!(
            cautious().transition(ActorPolicyState::new(ActorMode::Waiting, 1.0), &view),
            Err(PolicyError::InvalidActorState)
        );
    }

    #[test]
    fn scenario_state_restoration_validates_mode_and_speed_setpoint() {
        let restored =
            ActorPolicyState::from_scenario(ActorMode::Crossing, 1.2, None, bounds()).unwrap();
        assert_eq!(restored.mode(), ActorMode::Crossing);
        assert_eq!(restored.target_forward_speed_mps(), 1.2);

        assert_eq!(
            ActorPolicyState::from_scenario(ActorMode::Waiting, 0.5, None, bounds()),
            Err(PolicyError::InvalidActorState)
        );
        assert_eq!(
            ActorPolicyState::from_scenario(ActorMode::Crossing, 2.0, None, bounds()),
            Err(PolicyError::InvalidActorState)
        );
    }

    #[test]
    fn policy_rejects_duplicate_and_backwards_decision_ticks() {
        let policy = cautious();
        let first = policy
            .transition(
                ActorPolicyState::waiting(),
                &ttc_view(10, CueStatus::Observed, Some(4.0)),
            )
            .unwrap();
        assert_eq!(
            first.next_state().last_decision_tick(),
            Some(ScenarioTick(10))
        );

        for tick in [10, 9] {
            assert_eq!(
                policy.transition(
                    first.next_state(),
                    &ttc_view(tick, CueStatus::Observed, Some(4.0)),
                ),
                Err(PolicyError::NonMonotonicTick)
            );
        }
    }

    #[test]
    fn restored_checkpoint_rejects_replaying_its_last_decision_tick() {
        let restored = ActorPolicyState::from_scenario(
            ActorMode::Crossing,
            1.0,
            Some(ScenarioTick(42)),
            bounds(),
        )
        .unwrap();
        assert_eq!(restored.last_decision_tick(), Some(ScenarioTick(42)));
        assert_eq!(
            cautious().transition(restored, &ttc_view(42, CueStatus::Observed, Some(4.0)),),
            Err(PolicyError::NonMonotonicTick)
        );
        assert!(
            cautious()
                .transition(restored, &ttc_view(43, CueStatus::Observed, Some(4.0)),)
                .is_ok()
        );
    }

    #[test]
    fn stable_profile_ids_match_the_conformance_manifest() {
        assert_eq!(cautious().profile_id(), "cautious_pause_v1");
        assert_eq!(committed().profile_id(), "committed_after_entry_v1");
    }
}
