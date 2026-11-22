// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Event-sourced benchmark executor for multi-round active audiovisual sensing.
//!
//! This crate is a deterministic research harness. It does not control physical
//! sensors, provide real-time scheduling, transform clocks, or implement a
//! production action queue. Evaluator-private action outcomes become
//! candidate-visible only when the executor emits their arrival event.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Current transcript and action-protocol schema version.
pub const SCHEMA_VERSION_V1: u32 = 1;

/// SHA-256 genesis marker for an empty transcript.
pub const GENESIS_DIGEST_V1: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// Deterministic executor validation error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutorError {
    message: String,
}

impl ExecutorError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for ExecutorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ExecutorError {}

/// Exact integer-valued acquisition resource accounting.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceCostV1 {
    /// Raw audio bytes.
    pub audio_bytes: u64,
    /// Video frames.
    pub video_frames: u64,
    /// Energy in millijoules. Scenario units until backed by hardware metering.
    pub energy_millijoules: u64,
}

impl ResourceCostV1 {
    fn fits_within(self, limit: Self) -> bool {
        self.audio_bytes <= limit.audio_bytes
            && self.video_frames <= limit.video_frames
            && self.energy_millijoules <= limit.energy_millijoules
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            audio_bytes: self.audio_bytes.checked_add(other.audio_bytes)?,
            video_frames: self.video_frames.checked_add(other.video_frames)?,
            energy_millijoules: self
                .energy_millijoules
                .checked_add(other.energy_millijoules)?,
        })
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        Some(Self {
            audio_bytes: self.audio_bytes.checked_sub(other.audio_bytes)?,
            video_frames: self.video_frames.checked_sub(other.video_frames)?,
            energy_millijoules: self
                .energy_millijoules
                .checked_sub(other.energy_millijoules)?,
        })
    }
}

/// Modality carried by one delivered observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModalityV1 {
    /// Audio observation.
    Audio,
    /// Video observation.
    Video,
}

/// One candidate-visible observation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationV1 {
    /// Immutable observation identity.
    pub observation_ref: String,
    /// Source modality.
    pub modality: ModalityV1,
    /// Logical tick at which this observation became available.
    pub available_tick: u64,
    /// Human-readable or model-readable observation payload for this fixture.
    pub summary: String,
    /// Shared root dependency identity; equal groups are not independent corroboration.
    pub source_dependency_group: Option<String>,
    /// Whether the observation arrived by the decision deadline.
    pub eligible_for_decision: bool,
}

/// Public action that can be requested by the candidate.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSpecV1 {
    /// Stable action identifier.
    pub action_id: String,
    /// Source modality produced by this action.
    pub modality: ModalityV1,
    /// Logical ticks from accepted request to its fixture outcome.
    pub latency_ticks: u64,
    /// Maximum resource reservation before the outcome is known.
    pub reserve_cost: ResourceCostV1,
    /// Resource cost consumed by an explicit cancellation.
    pub cancel_cost: ResourceCostV1,
    /// Maximum accepted requests for this action in the episode.
    pub max_uses: u32,
    /// Whether a pending action may be cancelled.
    pub cancellable: bool,
}

/// Public initial evidence and action space for an episode.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeConfigV1 {
    /// Stable episode identifier.
    pub episode_id: String,
    /// Logical start tick.
    pub query_tick: u64,
    /// Last tick at which a decision can be on-time.
    pub deadline_tick: u64,
    /// Initial evidence already available to the candidate.
    pub initial_observations: Vec<ObservationV1>,
    /// Public actions and their reservation costs.
    pub action_catalogue: Vec<ActionSpecV1>,
    /// Total per-episode acquisition budget.
    pub budget: ResourceCostV1,
    /// Maximum number of simultaneously pending requests. V1 supports one.
    pub max_pending_actions: u32,
}

/// Hidden evaluator-side action result. It is never included in CandidateView.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionOutcomeFixtureV1 {
    /// Action identifier this fixture resolves.
    pub action_id: String,
    /// Actual outcome before deadline qualification is applied.
    pub status: FixtureOutcomeStatusV1,
    /// Observation identity, required only for delivered outcomes.
    pub observation_ref: Option<String>,
    /// Delivered observation content, required only for delivered outcomes.
    pub observation_summary: Option<String>,
    /// Source dependency group for delivered evidence.
    pub source_dependency_group: Option<String>,
    /// Diagnostic detail for partial, failed or unavailable outcomes.
    pub detail: Option<String>,
    /// Actual consumed resources after completion or failure.
    pub consumed_cost: ResourceCostV1,
}

/// Evaluator fixture outcome before deadline qualification.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureOutcomeStatusV1 {
    /// Requested observation was acquired.
    Delivered,
    /// Some operation completed, but no admissible observation was produced.
    Partial,
    /// Requested evidence was unavailable.
    Unavailable,
    /// Requested action failed.
    Failed,
}

/// Candidate-visible status for a resolved action.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibleOutcomeStatusV1 {
    /// Delivered before or at the decision deadline.
    Delivered,
    /// Result arrived after the decision deadline.
    Late,
    /// Partial operation, with no observation identity.
    Partial,
    /// Requested evidence was unavailable.
    Unavailable,
    /// Action failed.
    Failed,
    /// Candidate cancelled the pending request.
    Cancelled,
}

/// One candidate decision for the current transcript state.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CandidateActionV1 {
    /// Request one action; receive its outcome before replanning.
    Request {
        /// Unique idempotency key.
        request_id: String,
        /// Public action catalogue identity.
        action_id: String,
        /// Candidate finite EIG estimate, in nats.
        estimated_eig_nats: f64,
        /// Candidate finite EVSI estimate, in decision-value units.
        estimated_evsi: f64,
    },
    /// Provide an answer supported by currently visible evidence.
    Answer {
        /// Final answer text.
        answer: String,
        /// Exact observation identities used for the answer.
        cited_observation_refs: Vec<String>,
    },
    /// Explicitly abstain from answering.
    Abstain {
        /// Reason for abstention.
        reason: String,
        /// Exact observation identities considered.
        cited_observation_refs: Vec<String>,
    },
    /// Stop without an answer when no further acquisition is useful.
    StopWithoutAnswer {
        /// Reason for stopping.
        reason: String,
    },
    /// Cancel the currently pending action.
    CancelPending {
        /// Request identity returned by the executor on acceptance.
        request_id: String,
    },
}

/// Candidate turn bound to an exact prior transcript digest.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateTurnV1 {
    /// Monotonically increasing candidate turn number.
    pub turn_id: u64,
    /// Digest of the latest candidate-visible event transcript.
    pub transcript_head_sha256: String,
    /// Exactly one action for this turn.
    pub action: CandidateActionV1,
}

/// Event payload in the candidate-visible append-only transcript.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "event_type", content = "payload", rename_all = "snake_case")]
pub enum TranscriptPayloadV1 {
    /// Initial public episode state.
    EpisodeStarted {
        /// Episode identity.
        episode_id: String,
        /// Decision query tick.
        query_tick: u64,
        /// Decision deadline.
        deadline_tick: u64,
        /// Initial visible observations.
        initial_observations: Vec<ObservationV1>,
        /// Public action catalogue.
        action_catalogue: Vec<ActionSpecV1>,
        /// Total budget.
        budget: ResourceCostV1,
    },
    /// Candidate's own action, recorded for deterministic replay.
    CandidateTurnSubmitted {
        /// Candidate turn sequence.
        turn_id: u64,
        /// Candidate action.
        action: CandidateActionV1,
    },
    /// Request accepted and queued; no outcome data is disclosed.
    ActionAccepted {
        /// Request identity.
        request_id: String,
        /// Requested action identity.
        action_id: String,
        /// Logical request tick.
        request_tick: u64,
        /// Scheduled result tick.
        expected_arrival_tick: u64,
        /// Maximum reserved resources.
        reserved_cost: ResourceCostV1,
    },
    /// Outcome becomes visible only when the executor advances to its arrival tick.
    ActionOutcome {
        /// Request identity.
        request_id: String,
        /// Requested action identity.
        action_id: String,
        /// Candidate-visible result status.
        status: VisibleOutcomeStatusV1,
        /// Delivered observation, if any. Late observations retain identity but are ineligible.
        observation: Option<ObservationV1>,
        /// Diagnostic outcome detail.
        detail: Option<String>,
        /// Actual consumed resources.
        consumed_cost: ResourceCostV1,
    },
    /// Candidate provided a final answer.
    AnswerSubmitted {
        /// Final answer.
        answer: String,
        /// Cited observations.
        cited_observation_refs: Vec<String>,
    },
    /// Candidate abstained.
    AbstentionSubmitted {
        /// Abstention reason.
        reason: String,
        /// Cited observations.
        cited_observation_refs: Vec<String>,
    },
    /// Candidate stopped without an answer.
    StoppedWithoutAnswer {
        /// Stop reason.
        reason: String,
    },
}

/// One hash-chained event in the candidate-visible transcript.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptEventV1 {
    /// Zero-based event sequence.
    pub sequence: u64,
    /// Logical event tick.
    pub tick: u64,
    /// Previous event digest, or genesis marker for the first event.
    pub previous_digest_sha256: String,
    /// Event contents.
    pub payload: TranscriptPayloadV1,
    /// SHA-256 of the previous digest and exact serialized event body.
    pub digest_sha256: String,
}

/// Candidate-visible state. Evaluator fixtures and future outcomes are absent.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateViewV1 {
    /// Episode identifier.
    pub episode_id: String,
    /// Next required candidate turn.
    pub next_turn_id: u64,
    /// Current logical tick.
    pub current_tick: u64,
    /// Decision deadline.
    pub deadline_tick: u64,
    /// Public action catalogue.
    pub action_catalogue: Vec<ActionSpecV1>,
    /// Candidate-visible transcript.
    pub visible_events: Vec<TranscriptEventV1>,
    /// Currently visible observations, including late observations marked ineligible.
    pub visible_observations: Vec<ObservationV1>,
    /// Remaining budget after subtracting spent and pending reservations.
    pub remaining_budget: ResourceCostV1,
    /// Pending request identity, if one is in flight.
    pub pending_request_id: Option<String>,
    /// Digest to bind the next candidate turn to.
    pub transcript_head_sha256: String,
}

/// Completion state of the benchmark episode.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeStatusV1 {
    /// Candidate may submit another turn.
    Active,
    /// Candidate answered.
    Answered,
    /// Candidate abstained.
    Abstained,
    /// Candidate stopped without an answer.
    Stopped,
}

struct PendingAction {
    request_id: String,
    action_id: String,
    arrival_tick: u64,
    reserve_cost: ResourceCostV1,
}

/// Deterministic event-sourced multi-round benchmark executor.
pub struct ActiveSensingExecutorV1 {
    config: EpisodeConfigV1,
    fixtures: BTreeMap<String, ActionOutcomeFixtureV1>,
    events: Vec<TranscriptEventV1>,
    observations: BTreeMap<String, ObservationV1>,
    status: EpisodeStatusV1,
    current_tick: u64,
    next_turn_id: u64,
    pending: Option<PendingAction>,
    seen_request_ids: BTreeSet<String>,
    action_uses: BTreeMap<String, u32>,
    spent: ResourceCostV1,
    reserved: ResourceCostV1,
}

impl ActiveSensingExecutorV1 {
    /// Create an executor. Fixtures remain evaluator-private and are never returned.
    pub fn new(
        config: EpisodeConfigV1,
        fixtures: Vec<ActionOutcomeFixtureV1>,
    ) -> Result<Self, ExecutorError> {
        let mut config = config;
        validate_config(&config)?;
        for observation in &mut config.initial_observations {
            observation.eligible_for_decision = observation.available_tick <= config.deadline_tick;
        }
        let mut fixture_map = BTreeMap::new();
        for fixture in fixtures {
            if fixture_map
                .insert(fixture.action_id.clone(), fixture)
                .is_some()
            {
                return Err(ExecutorError::new("duplicate evaluator action fixture"));
            }
        }
        let action_ids: BTreeSet<String> = config
            .action_catalogue
            .iter()
            .map(|action| action.action_id.clone())
            .collect();
        if fixture_map.keys().cloned().collect::<BTreeSet<_>>() != action_ids {
            return Err(ExecutorError::new(
                "action catalogue and evaluator fixture identities must match exactly",
            ));
        }
        let mut fixture_observation_refs = BTreeSet::new();
        for observation in &config.initial_observations {
            fixture_observation_refs.insert(observation.observation_ref.clone());
        }
        for action in &config.action_catalogue {
            let fixture = fixture_map
                .get(&action.action_id)
                .ok_or_else(|| ExecutorError::new("missing action fixture"))?;
            validate_fixture(action, fixture)?;
            if let Some(reference) = &fixture.observation_ref {
                if !fixture_observation_refs.insert(reference.clone()) {
                    return Err(ExecutorError::new(
                        "action fixtures reuse an initial or another action observation identity",
                    ));
                }
            }
        }

        let current_tick = config.query_tick;
        let mut observations = BTreeMap::new();
        for observation in &config.initial_observations {
            let initial = observation.clone();
            if observations
                .insert(initial.observation_ref.clone(), initial)
                .is_some()
            {
                return Err(ExecutorError::new("duplicate initial observation identity"));
            }
        }

        let mut executor = Self {
            config,
            fixtures: fixture_map,
            events: Vec::new(),
            observations,
            status: EpisodeStatusV1::Active,
            current_tick,
            next_turn_id: 1,
            pending: None,
            seen_request_ids: BTreeSet::new(),
            action_uses: BTreeMap::new(),
            spent: ResourceCostV1::default(),
            reserved: ResourceCostV1::default(),
        };
        executor.append_event(TranscriptPayloadV1::EpisodeStarted {
            episode_id: executor.config.episode_id.clone(),
            query_tick: executor.config.query_tick,
            deadline_tick: executor.config.deadline_tick,
            initial_observations: executor.config.initial_observations.clone(),
            action_catalogue: executor.config.action_catalogue.clone(),
            budget: executor.config.budget,
        })?;
        Ok(executor)
    }

    /// Return current candidate-visible state. Future outcome fixtures are never exposed.
    pub fn candidate_view(&self) -> Result<CandidateViewV1, ExecutorError> {
        let committed = self
            .spent
            .checked_add(self.reserved)
            .ok_or_else(|| ExecutorError::new("resource accounting overflow"))?;
        let remaining_budget = self
            .config
            .budget
            .checked_sub(committed)
            .ok_or_else(|| ExecutorError::new("resource reservation exceeds episode budget"))?;
        let visible_observations = self
            .observations
            .values()
            .cloned()
            .map(|mut observation| {
                observation.eligible_for_decision = observation.available_tick
                    <= self.config.deadline_tick
                    && observation.available_tick <= self.current_tick;
                observation
            })
            .collect();
        Ok(CandidateViewV1 {
            episode_id: self.config.episode_id.clone(),
            next_turn_id: self.next_turn_id,
            current_tick: self.current_tick,
            deadline_tick: self.config.deadline_tick,
            action_catalogue: self.config.action_catalogue.clone(),
            visible_events: self.events.clone(),
            visible_observations,
            remaining_budget,
            pending_request_id: self
                .pending
                .as_ref()
                .map(|pending| pending.request_id.clone()),
            transcript_head_sha256: self.head_digest(),
        })
    }

    /// Read current episode status.
    pub fn status(&self) -> &EpisodeStatusV1 {
        &self.status
    }

    /// Read candidate-visible append-only transcript.
    pub fn transcript(&self) -> &[TranscriptEventV1] {
        &self.events
    }

    /// Submit one action bound to the exact current transcript head.
    pub fn submit_turn(&mut self, turn: CandidateTurnV1) -> Result<(), ExecutorError> {
        if self.status != EpisodeStatusV1::Active {
            return Err(ExecutorError::new("episode is not active"));
        }
        if turn.turn_id != self.next_turn_id {
            return Err(ExecutorError::new(
                "candidate turn id is stale or out of order",
            ));
        }
        if turn.transcript_head_sha256 != self.head_digest() {
            return Err(ExecutorError::new(
                "candidate turn is not bound to the exact current transcript head",
            ));
        }

        match &turn.action {
            CandidateActionV1::Request {
                request_id,
                action_id,
                estimated_eig_nats,
                estimated_evsi,
            } => {
                if self.pending.is_some() {
                    return Err(ExecutorError::new(
                        "candidate must receive or cancel pending outcome before requesting again",
                    ));
                }
                if self.current_tick > self.config.deadline_tick {
                    return Err(ExecutorError::new(
                        "cannot request evidence after the deadline",
                    ));
                }
                if request_id.trim().is_empty() || self.seen_request_ids.contains(request_id) {
                    return Err(ExecutorError::new(
                        "request id is empty or has already been used",
                    ));
                }
                if !estimated_eig_nats.is_finite()
                    || *estimated_eig_nats < 0.0
                    || !estimated_evsi.is_finite()
                {
                    return Err(ExecutorError::new("candidate EIG/EVSI estimate is invalid"));
                }
                let action = self.action_spec(action_id)?.clone();
                let uses = self.action_uses.get(action_id).copied().unwrap_or(0);
                if uses >= action.max_uses {
                    return Err(ExecutorError::new("action max-use limit exceeded"));
                }
                if !action.reserve_cost.fits_within(self.remaining_budget()?) {
                    return Err(ExecutorError::new(
                        "action reservation exceeds remaining budget",
                    ));
                }
                let arrival_tick = self
                    .current_tick
                    .checked_add(action.latency_ticks)
                    .ok_or_else(|| ExecutorError::new("action arrival tick overflow"))?;
                self.append_event(TranscriptPayloadV1::CandidateTurnSubmitted {
                    turn_id: turn.turn_id,
                    action: turn.action.clone(),
                })?;
                self.reserved = self
                    .reserved
                    .checked_add(action.reserve_cost)
                    .ok_or_else(|| ExecutorError::new("resource reservation overflow"))?;
                self.seen_request_ids.insert(request_id.clone());
                self.action_uses.insert(action_id.clone(), uses + 1);
                self.pending = Some(PendingAction {
                    request_id: request_id.clone(),
                    action_id: action_id.clone(),
                    arrival_tick,
                    reserve_cost: action.reserve_cost,
                });
                self.append_event(TranscriptPayloadV1::ActionAccepted {
                    request_id: request_id.clone(),
                    action_id: action_id.clone(),
                    request_tick: self.current_tick,
                    expected_arrival_tick: arrival_tick,
                    reserved_cost: action.reserve_cost,
                })?;
            }
            CandidateActionV1::CancelPending { request_id } => {
                let pending = self
                    .pending
                    .as_ref()
                    .ok_or_else(|| ExecutorError::new("there is no pending action to cancel"))?;
                if pending.request_id != *request_id {
                    return Err(ExecutorError::new(
                        "cancellation request id does not match pending action",
                    ));
                }
                let pending = self.pending.clone().ok_or_else(|| {
                    ExecutorError::new("pending request disappeared during cancellation")
                })?;
                let action = self.action_spec(&pending.action_id)?.clone();
                if !action.cancellable {
                    return Err(ExecutorError::new("pending action is not cancellable"));
                }
                if !action.cancel_cost.fits_within(pending.reserve_cost) {
                    return Err(ExecutorError::new(
                        "cancellation consumption exceeds reservation",
                    ));
                }
                self.append_event(TranscriptPayloadV1::CandidateTurnSubmitted {
                    turn_id: turn.turn_id,
                    action: turn.action.clone(),
                })?;
                self.settle_cost(pending.reserve_cost, action.cancel_cost)?;
                self.pending = None;
                self.append_event(TranscriptPayloadV1::ActionOutcome {
                    request_id: pending.request_id,
                    action_id: pending.action_id,
                    status: VisibleOutcomeStatusV1::Cancelled,
                    observation: None,
                    detail: Some("cancelled by candidate before fixture delivery".to_string()),
                    consumed_cost: action.cancel_cost,
                })?;
            }
            CandidateActionV1::Answer {
                answer,
                cited_observation_refs,
            } => {
                self.ensure_no_pending()?;
                self.validate_citations(cited_observation_refs)?;
                if self.current_tick > self.config.deadline_tick {
                    return Err(ExecutorError::new(
                        "answer was submitted after the deadline",
                    ));
                }
                self.append_event(TranscriptPayloadV1::CandidateTurnSubmitted {
                    turn_id: turn.turn_id,
                    action: turn.action.clone(),
                })?;
                self.append_event(TranscriptPayloadV1::AnswerSubmitted {
                    answer: answer.clone(),
                    cited_observation_refs: cited_observation_refs.clone(),
                })?;
                self.status = EpisodeStatusV1::Answered;
            }
            CandidateActionV1::Abstain {
                reason,
                cited_observation_refs,
            } => {
                self.ensure_no_pending()?;
                self.validate_citations(cited_observation_refs)?;
                if self.current_tick > self.config.deadline_tick {
                    return Err(ExecutorError::new(
                        "abstention was submitted after the deadline",
                    ));
                }
                self.append_event(TranscriptPayloadV1::CandidateTurnSubmitted {
                    turn_id: turn.turn_id,
                    action: turn.action.clone(),
                })?;
                self.append_event(TranscriptPayloadV1::AbstentionSubmitted {
                    reason: reason.clone(),
                    cited_observation_refs: cited_observation_refs.clone(),
                })?;
                self.status = EpisodeStatusV1::Abstained;
            }
            CandidateActionV1::StopWithoutAnswer { reason } => {
                self.ensure_no_pending()?;
                if self.current_tick > self.config.deadline_tick {
                    return Err(ExecutorError::new("stop was submitted after the deadline"));
                }
                self.append_event(TranscriptPayloadV1::CandidateTurnSubmitted {
                    turn_id: turn.turn_id,
                    action: turn.action.clone(),
                })?;
                self.append_event(TranscriptPayloadV1::StoppedWithoutAnswer {
                    reason: reason.clone(),
                })?;
                self.status = EpisodeStatusV1::Stopped;
            }
        }
        self.next_turn_id = self
            .next_turn_id
            .checked_add(1)
            .ok_or_else(|| ExecutorError::new("candidate turn sequence overflow"))?;
        Ok(())
    }

    /// Advance to the next pending outcome and expose it only at its arrival tick.
    pub fn advance_to_next_outcome(&mut self) -> Result<CandidateViewV1, ExecutorError> {
        if self.status != EpisodeStatusV1::Active {
            return Err(ExecutorError::new("episode is not active"));
        }
        let pending = self
            .pending
            .clone()
            .ok_or_else(|| ExecutorError::new("no pending outcome is scheduled"))?;
        let fixture = self
            .fixtures
            .get(&pending.action_id)
            .ok_or_else(|| ExecutorError::new("missing evaluator action outcome"))?
            .clone();
        let action = self.action_spec(&pending.action_id)?.clone();
        if !fixture.consumed_cost.fits_within(pending.reserve_cost) {
            return Err(ExecutorError::new(
                "actual action consumption exceeds the accepted reservation",
            ));
        }
        self.current_tick = pending.arrival_tick;
        self.settle_cost(pending.reserve_cost, fixture.consumed_cost)?;
        let visible_status = match fixture.status {
            FixtureOutcomeStatusV1::Delivered if self.current_tick > self.config.deadline_tick => {
                VisibleOutcomeStatusV1::Late
            }
            FixtureOutcomeStatusV1::Delivered => VisibleOutcomeStatusV1::Delivered,
            FixtureOutcomeStatusV1::Partial => VisibleOutcomeStatusV1::Partial,
            FixtureOutcomeStatusV1::Unavailable => VisibleOutcomeStatusV1::Unavailable,
            FixtureOutcomeStatusV1::Failed => VisibleOutcomeStatusV1::Failed,
        };
        let observation = match (
            fixture.observation_ref,
            fixture.observation_summary,
            fixture.source_dependency_group,
        ) {
            (Some(observation_ref), Some(summary), source_dependency_group) => {
                let observation = ObservationV1 {
                    observation_ref,
                    modality: action.modality,
                    available_tick: self.current_tick,
                    summary,
                    source_dependency_group,
                    eligible_for_decision: self.current_tick <= self.config.deadline_tick,
                };
                if self.observations.contains_key(&observation.observation_ref) {
                    return Err(ExecutorError::new(
                        "action outcome reuses an existing observation identity",
                    ));
                }
                self.observations
                    .insert(observation.observation_ref.clone(), observation.clone());
                Some(observation)
            }
            (None, None, None) => None,
            _ => {
                return Err(ExecutorError::new(
                    "action fixture observation ref, summary and dependency must be complete",
                ));
            }
        };
        if !matches!(
            visible_status,
            VisibleOutcomeStatusV1::Delivered | VisibleOutcomeStatusV1::Late
        ) && observation.is_some()
        {
            return Err(ExecutorError::new(
                "partial, unavailable or failed outcome cannot mint an observation",
            ));
        }
        self.pending = None;
        self.append_event(TranscriptPayloadV1::ActionOutcome {
            request_id: pending.request_id,
            action_id: pending.action_id,
            status: visible_status,
            observation,
            detail: fixture.detail,
            consumed_cost: fixture.consumed_cost,
        })?;
        self.candidate_view()
    }

    fn action_spec(&self, action_id: &str) -> Result<&ActionSpecV1, ExecutorError> {
        self.config
            .action_catalogue
            .iter()
            .find(|action| action.action_id == action_id)
            .ok_or_else(|| ExecutorError::new("unknown action id"))
    }

    fn ensure_no_pending(&self) -> Result<(), ExecutorError> {
        if self.pending.is_some() {
            Err(ExecutorError::new(
                "candidate must receive or cancel pending action before finalizing",
            ))
        } else {
            Ok(())
        }
    }

    fn validate_citations(&self, citations: &[String]) -> Result<(), ExecutorError> {
        let mut unique = BTreeSet::new();
        for reference in citations {
            if !unique.insert(reference) {
                return Err(ExecutorError::new("duplicate observation citation"));
            }
            let observation = self
                .observations
                .get(reference)
                .ok_or_else(|| ExecutorError::new("citation is not candidate-visible"))?;
            if observation.available_tick > self.current_tick
                || observation.available_tick > self.config.deadline_tick
            {
                return Err(ExecutorError::new(
                    "citation is future evidence or arrived after the decision deadline",
                ));
            }
        }
        Ok(())
    }

    fn remaining_budget(&self) -> Result<ResourceCostV1, ExecutorError> {
        self.config
            .budget
            .checked_sub(self.spent)
            .and_then(|available| available.checked_sub(self.reserved))
            .ok_or_else(|| ExecutorError::new("resource accounting exceeds episode budget"))
    }

    fn settle_cost(
        &mut self,
        reservation: ResourceCostV1,
        consumed: ResourceCostV1,
    ) -> Result<(), ExecutorError> {
        if !consumed.fits_within(reservation) {
            return Err(ExecutorError::new(
                "consumed resource cost exceeds reservation",
            ));
        }
        self.reserved = self
            .reserved
            .checked_sub(reservation)
            .ok_or_else(|| ExecutorError::new("resource reservation underflow"))?;
        self.spent = self
            .spent
            .checked_add(consumed)
            .ok_or_else(|| ExecutorError::new("spent resource accounting overflow"))?;
        Ok(())
    }

    fn head_digest(&self) -> String {
        self.events
            .last()
            .map(|event| event.digest_sha256.clone())
            .unwrap_or_else(|| GENESIS_DIGEST_V1.to_string())
    }

    fn append_event(&mut self, payload: TranscriptPayloadV1) -> Result<(), ExecutorError> {
        let sequence = self.events.len() as u64;
        let previous_digest_sha256 = self.head_digest();
        let encoded = serde_json::to_vec(&(
            sequence,
            self.current_tick,
            &previous_digest_sha256,
            &payload,
        ))
        .map_err(|error| ExecutorError::new(format!("transcript serialization failed: {error}")))?;
        let digest_sha256 = hex_sha256(&encoded);
        self.events.push(TranscriptEventV1 {
            sequence,
            tick: self.current_tick,
            previous_digest_sha256,
            payload,
            digest_sha256,
        });
        Ok(())
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn validate_config(config: &EpisodeConfigV1) -> Result<(), ExecutorError> {
    if config.episode_id.trim().is_empty() || config.deadline_tick < config.query_tick {
        return Err(ExecutorError::new(
            "invalid episode identity or query deadline",
        ));
    }
    if config.max_pending_actions != 1 {
        return Err(ExecutorError::new(
            "v1 executor currently supports exactly one pending action",
        ));
    }
    let mut action_ids = BTreeSet::new();
    for action in &config.action_catalogue {
        if action.action_id.trim().is_empty()
            || action.max_uses != 1
            || !action_ids.insert(action.action_id.clone())
            || !action.cancel_cost.fits_within(action.reserve_cost)
        {
            return Err(ExecutorError::new(
                "invalid or duplicate action catalogue entry",
            ));
        }
    }
    let mut observation_ids = BTreeSet::new();
    for observation in &config.initial_observations {
        if observation.observation_ref.trim().is_empty()
            || observation.summary.trim().is_empty()
            || observation.available_tick > config.query_tick
            || !observation_ids.insert(observation.observation_ref.clone())
        {
            return Err(ExecutorError::new("invalid initial observation metadata"));
        }
    }
    Ok(())
}

fn validate_fixture(
    action: &ActionSpecV1,
    fixture: &ActionOutcomeFixtureV1,
) -> Result<(), ExecutorError> {
    if !fixture.consumed_cost.fits_within(action.reserve_cost) {
        return Err(ExecutorError::new(
            "fixture consumption exceeds the action reservation",
        ));
    }
    match fixture.status {
        FixtureOutcomeStatusV1::Delivered => {
            if fixture
                .observation_ref
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
                || fixture
                    .observation_summary
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty()
            {
                return Err(ExecutorError::new(
                    "delivered action fixture requires an observation identity and summary",
                ));
            }
        }
        FixtureOutcomeStatusV1::Partial
        | FixtureOutcomeStatusV1::Unavailable
        | FixtureOutcomeStatusV1::Failed => {
            if fixture.observation_ref.is_some()
                || fixture.observation_summary.is_some()
                || fixture.source_dependency_group.is_some()
            {
                return Err(ExecutorError::new(
                    "non-delivered action fixture cannot contain observation identity",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cost(audio: u64, frames: u64, energy: u64) -> ResourceCostV1 {
        ResourceCostV1 {
            audio_bytes: audio,
            video_frames: frames,
            energy_millijoules: energy,
        }
    }

    fn action(
        action_id: &str,
        modality: ModalityV1,
        latency: u64,
        reserve: ResourceCostV1,
    ) -> ActionSpecV1 {
        ActionSpecV1 {
            action_id: action_id.to_string(),
            modality,
            latency_ticks: latency,
            reserve_cost: reserve,
            cancel_cost: ResourceCostV1::default(),
            max_uses: 1,
            cancellable: true,
        }
    }

    fn config() -> EpisodeConfigV1 {
        EpisodeConfigV1 {
            episode_id: "multiround-example".to_string(),
            query_tick: 10,
            deadline_tick: 40,
            initial_observations: vec![ObservationV1 {
                observation_ref: "obs-initial-camera".to_string(),
                modality: ModalityV1::Video,
                available_tick: 10,
                summary: "A machine is visible".to_string(),
                source_dependency_group: Some("root-machine".to_string()),
                eligible_for_decision: true,
            }],
            action_catalogue: vec![
                action(
                    "probe-machine-state",
                    ModalityV1::Audio,
                    2,
                    cost(2048, 0, 200),
                ),
                action(
                    "confirm-alarm-source",
                    ModalityV1::Video,
                    1,
                    cost(0, 1, 300),
                ),
            ],
            budget: cost(4096, 2, 1000),
            max_pending_actions: 1,
        }
    }

    fn fixtures() -> Vec<ActionOutcomeFixtureV1> {
        vec![
            ActionOutcomeFixtureV1 {
                action_id: "probe-machine-state".to_string(),
                status: FixtureOutcomeStatusV1::Delivered,
                observation_ref: Some("obs-machine-state".to_string()),
                observation_summary: Some(
                    "Non-speech acoustic pattern indicates an alarm".to_string(),
                ),
                source_dependency_group: Some("root-machine".to_string()),
                detail: None,
                consumed_cost: cost(1024, 0, 100),
            },
            ActionOutcomeFixtureV1 {
                action_id: "confirm-alarm-source".to_string(),
                status: FixtureOutcomeStatusV1::Delivered,
                observation_ref: Some("obs-alarm-source".to_string()),
                observation_summary: Some(
                    "A nearby maintenance cart is emitting the alarm".to_string(),
                ),
                source_dependency_group: Some("root-cart-alarm".to_string()),
                detail: None,
                consumed_cost: cost(0, 1, 150),
            },
        ]
    }

    fn turn(view: &CandidateViewV1, action: CandidateActionV1) -> CandidateTurnV1 {
        CandidateTurnV1 {
            turn_id: view.next_turn_id,
            transcript_head_sha256: view.transcript_head_sha256.clone(),
            action,
        }
    }

    #[test]
    fn replans_after_each_delivered_observation_without_future_leakage() {
        let mut executor =
            ActiveSensingExecutorV1::new(config(), fixtures()).expect("valid episode");
        let first_view = executor.candidate_view().expect("initial view");
        assert_eq!(first_view.visible_events.len(), 1);
        assert!(
            !first_view
                .visible_observations
                .iter()
                .any(|observation| observation.observation_ref == "obs-machine-state")
        );

        executor
            .submit_turn(turn(
                &first_view,
                CandidateActionV1::Request {
                    request_id: "request-1".to_string(),
                    action_id: "probe-machine-state".to_string(),
                    estimated_eig_nats: 0.2,
                    estimated_evsi: 4.0,
                },
            ))
            .expect("first request accepted");
        let accepted_view = executor.candidate_view().expect("accepted view");
        assert_eq!(
            accepted_view.pending_request_id.as_deref(),
            Some("request-1")
        );
        assert!(
            !accepted_view
                .visible_observations
                .iter()
                .any(|observation| observation.observation_ref == "obs-machine-state")
        );
        assert!(
            accepted_view
                .visible_events
                .iter()
                .any(|event| matches!(&event.payload, &TranscriptPayloadV1::ActionAccepted { .. }))
        );

        let after_first_delivery = executor
            .advance_to_next_outcome()
            .expect("first result delivered");
        assert!(
            after_first_delivery
                .visible_observations
                .iter()
                .any(
                    |observation| observation.observation_ref == "obs-machine-state"
                        && observation.eligible_for_decision
                        && observation.summary.contains("indicates an alarm")
                )
        );
        executor
            .submit_turn(turn(
                &after_first_delivery,
                CandidateActionV1::Request {
                    request_id: "request-2".to_string(),
                    action_id: "confirm-alarm-source".to_string(),
                    estimated_eig_nats: 0.1,
                    estimated_evsi: 2.0,
                },
            ))
            .expect("second request is a new turn after first delivery");

        let after_second_delivery = executor
            .advance_to_next_outcome()
            .expect("second result delivered");
        assert!(
            after_second_delivery
                .visible_observations
                .iter()
                .any(
                    |observation| observation.observation_ref == "obs-alarm-source"
                        && observation.eligible_for_decision
                )
        );
        executor
            .submit_turn(turn(
                &after_second_delivery,
                CandidateActionV1::Answer {
                    answer: "The machine is alarming and the maintenance cart is the source"
                        .to_string(),
                    cited_observation_refs: vec![
                        "obs-initial-camera".to_string(),
                        "obs-machine-state".to_string(),
                        "obs-alarm-source".to_string(),
                    ],
                },
            ))
            .expect("final answer accepted");
        assert_eq!(*executor.status(), EpisodeStatusV1::Answered);
        assert_eq!(
            executor.transcript().last().map(|event| event.tick),
            Some(13)
        );
    }

    #[test]
    fn v0_rejects_actions_with_multiple_fixture_uses() {
        let mut invalid_config = config();
        invalid_config.action_catalogue[0].max_uses = 2;
        assert!(ActiveSensingExecutorV1::new(invalid_config, fixtures()).is_err());
    }

    #[test]
    fn stale_transcript_digest_is_rejected_without_mutation() {
        let mut executor =
            ActiveSensingExecutorV1::new(config(), fixtures()).expect("valid episode");
        let mut candidate_turn = turn(
            &executor.candidate_view().expect("view"),
            CandidateActionV1::StopWithoutAnswer {
                reason: "test".to_string(),
            },
        );
        candidate_turn.transcript_head_sha256 = "f".repeat(64);
        let before = executor.transcript().len();
        assert!(executor.submit_turn(candidate_turn).is_err());
        assert_eq!(executor.transcript().len(), before);
        assert_eq!(*executor.status(), EpisodeStatusV1::Active);
    }

    #[test]
    fn late_observation_is_visible_but_cannot_support_a_decision() {
        let mut late_config = config();
        late_config.deadline_tick = 11;
        late_config.action_catalogue[0].latency_ticks = 4;
        let mut executor =
            ActiveSensingExecutorV1::new(late_config, fixtures()).expect("valid late episode");
        let view = executor.candidate_view().expect("initial view");
        executor
            .submit_turn(turn(
                &view,
                CandidateActionV1::Request {
                    request_id: "late-request".to_string(),
                    action_id: "probe-machine-state".to_string(),
                    estimated_eig_nats: 1.2,
                    estimated_evsi: 0.0,
                },
            ))
            .expect("request accepted");
        let after = executor
            .advance_to_next_outcome()
            .expect("late outcome delivered");
        let late = after
            .visible_observations
            .iter()
            .find(|observation| observation.observation_ref == "obs-machine-state")
            .expect("late observation is visible");
        assert!(!late.eligible_for_decision);
        let answer = turn(
            &after,
            CandidateActionV1::Answer {
                answer: "alarm".to_string(),
                cited_observation_refs: vec!["obs-machine-state".to_string()],
            },
        );
        assert!(executor.submit_turn(answer).is_err());
    }

    #[test]
    fn request_reserves_budget_until_outcome_settles() {
        let mut executor =
            ActiveSensingExecutorV1::new(config(), fixtures()).expect("valid episode");
        let view = executor.candidate_view().expect("initial view");
        executor
            .submit_turn(turn(
                &view,
                CandidateActionV1::Request {
                    request_id: "budget-request".to_string(),
                    action_id: "probe-machine-state".to_string(),
                    estimated_eig_nats: 0.2,
                    estimated_evsi: 1.0,
                },
            ))
            .expect("request accepted");
        let reserved = executor.candidate_view().expect("reserved view");
        assert_eq!(reserved.remaining_budget.audio_bytes, 2048);
        let settled = executor
            .advance_to_next_outcome()
            .expect("outcome delivered");
        assert_eq!(settled.remaining_budget.audio_bytes, 3072);
        assert_eq!(settled.remaining_budget.energy_millijoules, 900);
    }

    #[test]
    fn answer_is_rejected_while_an_action_is_pending() {
        let mut executor =
            ActiveSensingExecutorV1::new(config(), fixtures()).expect("valid episode");
        let view = executor.candidate_view().expect("initial view");
        executor
            .submit_turn(turn(
                &view,
                CandidateActionV1::Request {
                    request_id: "pending-request".to_string(),
                    action_id: "probe-machine-state".to_string(),
                    estimated_eig_nats: 0.1,
                    estimated_evsi: 1.0,
                },
            ))
            .expect("request accepted");
        let accepted = executor.candidate_view().expect("accepted view");
        let answer = turn(
            &accepted,
            CandidateActionV1::Answer {
                answer: "guess".to_string(),
                cited_observation_refs: vec!["obs-initial-camera".to_string()],
            },
        );
        assert!(executor.submit_turn(answer).is_err());
        assert_eq!(
            executor
                .candidate_view()
                .expect("still active")
                .pending_request_id
                .as_deref(),
            Some("pending-request")
        );
    }

    #[test]
    fn partial_outcome_does_not_create_an_observation() {
        let mut partial_fixtures = fixtures();
        partial_fixtures[0].status = FixtureOutcomeStatusV1::Partial;
        partial_fixtures[0].observation_ref = None;
        partial_fixtures[0].observation_summary = None;
        partial_fixtures[0].source_dependency_group = None;
        let mut executor = ActiveSensingExecutorV1::new(config(), partial_fixtures)
            .expect("partial fixture is valid");
        let view = executor.candidate_view().expect("initial view");
        executor
            .submit_turn(turn(
                &view,
                CandidateActionV1::Request {
                    request_id: "partial-request".to_string(),
                    action_id: "probe-machine-state".to_string(),
                    estimated_eig_nats: 0.1,
                    estimated_evsi: 1.0,
                },
            ))
            .expect("request accepted");
        let result = executor
            .advance_to_next_outcome()
            .expect("partial result delivered");
        assert!(
            !result
                .visible_observations
                .iter()
                .any(|observation| observation.observation_ref == "obs-machine-state")
        );
        assert!(result.visible_events.iter().any(|event| matches!(
            &event.payload,
            &TranscriptPayloadV1::ActionOutcome {
                status: VisibleOutcomeStatusV1::Partial,
                observation: None,
                ..
            }
        )));
    }

    #[test]
    fn transcript_replay_hash_is_deterministic_for_identical_interactions() {
        fn run_once() -> Vec<String> {
            let mut executor =
                ActiveSensingExecutorV1::new(config(), fixtures()).expect("valid episode");
            let view = executor.candidate_view().expect("initial view");
            executor
                .submit_turn(turn(
                    &view,
                    CandidateActionV1::Request {
                        request_id: "same-request".to_string(),
                        action_id: "probe-machine-state".to_string(),
                        estimated_eig_nats: 0.1,
                        estimated_evsi: 2.0,
                    },
                ))
                .expect("accepted");
            let after = executor.advance_to_next_outcome().expect("delivered");
            executor
                .submit_turn(turn(
                    &after,
                    CandidateActionV1::Abstain {
                        reason: "test abstention".to_string(),
                        cited_observation_refs: vec!["obs-machine-state".to_string()],
                    },
                ))
                .expect("abstained");
            executor
                .transcript()
                .iter()
                .map(|event| event.digest_sha256.clone())
                .collect()
        }
        assert_eq!(run_once(), run_once());
    }

    #[test]
    fn duplicate_request_identity_is_rejected_across_turns() {
        let mut executor =
            ActiveSensingExecutorV1::new(config(), fixtures()).expect("valid episode");
        let view = executor.candidate_view().expect("initial view");
        executor
            .submit_turn(turn(
                &view,
                CandidateActionV1::Request {
                    request_id: "reused-request".to_string(),
                    action_id: "probe-machine-state".to_string(),
                    estimated_eig_nats: 0.1,
                    estimated_evsi: 1.0,
                },
            ))
            .expect("first request");
        let after = executor.advance_to_next_outcome().expect("outcome");
        let count = executor.transcript().len();
        let duplicate = turn(
            &after,
            CandidateActionV1::Request {
                request_id: "reused-request".to_string(),
                action_id: "confirm-alarm-source".to_string(),
                estimated_eig_nats: 0.2,
                estimated_evsi: 2.0,
            },
        );
        assert!(executor.submit_turn(duplicate).is_err());
        assert_eq!(executor.transcript().len(), count);
    }

    #[test]
    fn cancellation_settles_reserved_budget_and_mints_no_observation() {
        let mut executor =
            ActiveSensingExecutorV1::new(config(), fixtures()).expect("valid episode");
        let view = executor.candidate_view().expect("initial view");
        executor
            .submit_turn(turn(
                &view,
                CandidateActionV1::Request {
                    request_id: "cancel-me".to_string(),
                    action_id: "probe-machine-state".to_string(),
                    estimated_eig_nats: 0.3,
                    estimated_evsi: 0.0,
                },
            ))
            .expect("request accepted");
        let pending = executor.candidate_view().expect("pending");
        executor
            .submit_turn(turn(
                &pending,
                CandidateActionV1::CancelPending {
                    request_id: "cancel-me".to_string(),
                },
            ))
            .expect("cancel accepted");
        let cancelled = executor.candidate_view().expect("cancelled view");
        assert_eq!(cancelled.pending_request_id, None);
        assert_eq!(cancelled.remaining_budget.audio_bytes, 4096);
        assert!(
            !cancelled
                .visible_observations
                .iter()
                .any(|observation| observation.observation_ref == "obs-machine-state")
        );
        assert!(cancelled.visible_events.iter().any(|event| matches!(
            &event.payload,
            &TranscriptPayloadV1::ActionOutcome {
                status: VisibleOutcomeStatusV1::Cancelled,
                ..
            }
        )));
    }
}
