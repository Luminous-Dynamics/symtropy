// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic, causality-preserving minimization of RRVR fault traces.
//!
//! A minimized trace is accepted only when the replay oracle returns the exact
//! original failure signature, including property, violation class and witness
//! event identity. Removing an event also removes all descendants that depend
//! on it. Keeping a subset always closes over its causal ancestors.
//!
//! This crate validates and minimizes supplied event schedules. It does not
//! execute ROS, a simulator, hardware, a TLA+ model or a native sink. The replay
//! oracle remains an independently versioned owner-supplied implementation.

use std::collections::BTreeSet;

pub const CONTRACT_VERSION: &str = "0.1";
pub const MAX_TRACE_EVENTS: usize = 256;
pub const MAX_ARTIFACTS: usize = 64;
pub const MAX_MINIMIZER_CALLS: usize = 100_000;

pub type Commitment = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceEventKind {
    ProcessStarted,
    AttemptReserved,
    DispatchStarted,
    NativeAccepted,
    CommandQueued,
    ProcessCrashed,
    ProcessRestarted,
    ControllerQueueFenced,
    DownstreamGraphFenced,
    OldCommandApplied,
    OldCommandNoEffect,
    OldCommandPartial,
    PersistenceCommitStarted,
    PersistenceCommitObserved,
    PersistenceCommitConfirmed,
    PersistenceNoCommitConfirmed,
    PersistenceCommitAborted,
    PersistenceResolved,
    PhysicalEffectObserved,
    PhysicalNoEffectObserved,
    PhysicalPartialObserved,
    PhysicalOutcomeUnknown,
    PhysicalOutcomeDisputed,
    MissionResolutionRecorded,
    RecoveryPermitIssued,
    FreshTicketCreated,
    NativeRecoveryAccepted,
    RecoveryCommandApplied,
    RecoveryCommandNoEffect,
    RecoveryCommandPartial,
    M1AuthorityRevoked,
    M2ApplicabilityRevoked,
    LiveBindingInvalidated,
    NaiveRetryAttempted,
    ObserverCommonCauseInjected,
    ObserverSampleCaptured,
    TelemetryCaptured,
    TraceFlushed,
    Other,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TlaActionEventMapping {
    pub action_name: &'static str,
    pub event_kind: TraceEventKind,
}

/// Manual, versioned mapping of every action in the pinned
/// RestartSafeEffectContinuity.tla Next relation. The workflow checks that each
/// action still exists in the model and that the declared count matches Next.
pub const TLA_ACTION_EVENT_MAPPING: [TlaActionEventMapping; 30] = [
    TlaActionEventMapping {
        action_name: "ReserveAttempt",
        event_kind: TraceEventKind::AttemptReserved,
    },
    TlaActionEventMapping {
        action_name: "MarkDispatchStarted",
        event_kind: TraceEventKind::DispatchStarted,
    },
    TlaActionEventMapping {
        action_name: "NativeAcceptOld",
        event_kind: TraceEventKind::NativeAccepted,
    },
    TlaActionEventMapping {
        action_name: "OldDeviceApplies",
        event_kind: TraceEventKind::OldCommandApplied,
    },
    TlaActionEventMapping {
        action_name: "OldDeviceDoesNotApply",
        event_kind: TraceEventKind::OldCommandNoEffect,
    },
    TlaActionEventMapping {
        action_name: "OldDevicePartiallyApplies",
        event_kind: TraceEventKind::OldCommandPartial,
    },
    TlaActionEventMapping {
        action_name: "BeginEffectRecordCommit",
        event_kind: TraceEventKind::PersistenceCommitStarted,
    },
    TlaActionEventMapping {
        action_name: "CommitEffectRecord",
        event_kind: TraceEventKind::PersistenceCommitObserved,
    },
    TlaActionEventMapping {
        action_name: "AbortEffectRecord",
        event_kind: TraceEventKind::PersistenceCommitAborted,
    },
    TlaActionEventMapping {
        action_name: "AcknowledgeStoreCommit",
        event_kind: TraceEventKind::PersistenceCommitConfirmed,
    },
    TlaActionEventMapping {
        action_name: "AcknowledgeStoreNoCommit",
        event_kind: TraceEventKind::PersistenceNoCommitConfirmed,
    },
    TlaActionEventMapping {
        action_name: "Crash",
        event_kind: TraceEventKind::ProcessCrashed,
    },
    TlaActionEventMapping {
        action_name: "Restart",
        event_kind: TraceEventKind::ProcessRestarted,
    },
    TlaActionEventMapping {
        action_name: "FenceControllerQueue",
        event_kind: TraceEventKind::ControllerQueueFenced,
    },
    TlaActionEventMapping {
        action_name: "FenceDownstreamExecution",
        event_kind: TraceEventKind::DownstreamGraphFenced,
    },
    TlaActionEventMapping {
        action_name: "ResolvePersistence",
        event_kind: TraceEventKind::PersistenceResolved,
    },
    TlaActionEventMapping {
        action_name: "ObserveOldEffect",
        event_kind: TraceEventKind::PhysicalEffectObserved,
    },
    TlaActionEventMapping {
        action_name: "ObserveUnknownEffect",
        event_kind: TraceEventKind::PhysicalOutcomeUnknown,
    },
    TlaActionEventMapping {
        action_name: "ObserveDisputedEffect",
        event_kind: TraceEventKind::PhysicalOutcomeDisputed,
    },
    TlaActionEventMapping {
        action_name: "ResolveMission",
        event_kind: TraceEventKind::MissionResolutionRecorded,
    },
    TlaActionEventMapping {
        action_name: "IssueRecoveryPermit",
        event_kind: TraceEventKind::RecoveryPermitIssued,
    },
    TlaActionEventMapping {
        action_name: "CreateFreshTicket",
        event_kind: TraceEventKind::FreshTicketCreated,
    },
    TlaActionEventMapping {
        action_name: "RevokeM1",
        event_kind: TraceEventKind::M1AuthorityRevoked,
    },
    TlaActionEventMapping {
        action_name: "RevokeM2",
        event_kind: TraceEventKind::M2ApplicabilityRevoked,
    },
    TlaActionEventMapping {
        action_name: "ChangeLiveBinding",
        event_kind: TraceEventKind::LiveBindingInvalidated,
    },
    TlaActionEventMapping {
        action_name: "NativeAcceptRecovery",
        event_kind: TraceEventKind::NativeRecoveryAccepted,
    },
    TlaActionEventMapping {
        action_name: "RecoveryDeviceApplies",
        event_kind: TraceEventKind::RecoveryCommandApplied,
    },
    TlaActionEventMapping {
        action_name: "RecoveryDeviceDoesNotApply",
        event_kind: TraceEventKind::RecoveryCommandNoEffect,
    },
    TlaActionEventMapping {
        action_name: "RecoveryDevicePartiallyApplies",
        event_kind: TraceEventKind::RecoveryCommandPartial,
    },
    TlaActionEventMapping {
        action_name: "NaiveRetry",
        event_kind: TraceEventKind::NaiveRetryAttempted,
    },
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockObservation {
    /// Source clock identity. Never compare values from different clock IDs.
    pub clock_id: String,
    /// Clock epoch; distinguishes monotonic-clock resets across boots.
    pub boot_epoch_id: String,
    /// Explicit source unit (for example `ns`, `ticks`, or a device-specific unit).
    pub unit: String,
    pub time_value: u64,
    /// Smallest resolution represented by this source clock reading.
    pub resolution: u64,
    /// Owner-supplied upper bound on timestamp uncertainty in the same unit.
    pub uncertainty: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub event_id: String,
    /// Strictly increasing deterministic serialization order. This is not
    /// itself a happens-before claim; causal order is expressed by predecessors.
    pub logical_sequence: u64,
    pub kind: TraceEventKind,
    pub actor_id: String,
    /// Every predecessor must appear earlier in the serialized trace.
    pub predecessor_event_ids: Vec<String>,
    /// Source-clock metadata is retained but never used to order incomparable clocks.
    pub clock_observation: Option<ClockObservation>,
    /// Raw immutable evidence/artifact identities produced or consumed here.
    pub raw_artifact_ids: Vec<String>,
    /// Commitment to the exact event input/parameters, computed externally.
    pub input_commitment: Commitment,
    /// Commitment to the exact event configuration/profile, computed externally.
    pub configuration_commitment: Commitment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FailureSignature {
    pub property_id: String,
    pub violation_class: String,
    /// Stable event identity, not an array index or wall-clock timestamp.
    pub witness_event_id: String,
    /// Commitment to the normalized violating state/observation at the witness.
    pub witness_state_commitment: Commitment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceArtifact {
    pub artifact_id: String,
    pub media_type: String,
    /// SHA-256 of raw immutable artifact bytes, computed by the artifact owner.
    pub raw_artifact_commitment: Commitment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceHeader {
    pub bundle_id: String,
    pub parent_bundle_id: Option<String>,
    pub format_version: String,
    pub system_id: String,
    pub implementation_revision: String,
    pub model_revision: String,
    pub fault_profile_revision: String,
    pub oracle_revision: String,
    pub replay_seed: u64,
    /// Commitment to the exact initial state and environment.
    pub initial_state_commitment: Commitment,
    pub target_failure: FailureSignature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceBundle {
    pub header: TraceHeader,
    pub events: Vec<TraceEvent>,
    pub artifacts: Vec<TraceArtifact>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceHoldReason {
    MissingHeaderIdentity,
    UnsupportedFormatVersion,
    ZeroInitialStateCommitment,
    MissingFailureSignature,
    MissingFailureWitness,
    EmptyTrace,
    TooManyEvents,
    TooManyArtifacts,
    InvalidEventIdentity,
    DuplicateEventIdentity,
    InvalidLogicalSequence,
    InvalidEventCommitment,
    MissingPredecessor,
    ForwardDependency,
    DuplicatePredecessor,
    InvalidArtifact,
    DuplicateArtifact,
    DuplicateEventArtifactReference,
    UnknownEventArtifactReference,
    InvalidClockObservation,
    OracleRevisionMismatch,
    ReplayBudgetTooSmall,
    ReplayBudgetTooLarge,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceValidation {
    Valid,
    Hold(TraceHoldReason),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayOutcome {
    Pass,
    InvalidTrace,
    Failure(FailureSignature),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayDeterminism {
    Deterministic {
        runs: usize,
        outcome: ReplayOutcome,
    },
    Nondeterministic {
        runs: usize,
        first: ReplayOutcome,
        differing: ReplayOutcome,
    },
}

/// Oracle implementations must be deterministic for a fixed revision, seed,
/// environment and event sequence. Replay is observational only: the oracle
/// must not call a native actuator sink or perform a physical action.
pub trait ReplayOracle {
    fn revision(&self) -> &str;

    fn replay(&self, bundle: &TraceBundle) -> ReplayOutcome;
}

/// Re-run the exact bundle repeatedly to test replay determinism before
/// publishing a benchmark result. This checks equality of returned verdicts;
/// it does not prove deterministic physical or simulator execution in general.
pub fn check_replay_determinism<O: ReplayOracle>(
    bundle: &TraceBundle,
    oracle: &O,
    repetitions: usize,
) -> Result<ReplayDeterminism, MinimizationError> {
    if let TraceValidation::Hold(reason) = validate_trace_bundle(bundle) {
        return Err(MinimizationError::InvalidBundle(reason));
    }
    if oracle.revision() != bundle.header.oracle_revision {
        return Err(MinimizationError::OracleRevisionMismatch);
    }
    if repetitions == 0 {
        return Err(MinimizationError::InvalidBundle(
            TraceHoldReason::ReplayBudgetTooSmall,
        ));
    }

    let first = oracle.replay(bundle);
    for run in 2..=repetitions {
        let next = oracle.replay(bundle);
        if next != first {
            return Ok(ReplayDeterminism::Nondeterministic {
                runs: run,
                first,
                differing: next,
            });
        }
    }
    Ok(ReplayDeterminism::Deterministic {
        runs: repetitions,
        outcome: first,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinimizationError {
    InvalidBundle(TraceHoldReason),
    BaselineDoesNotReproduce,
    OracleRevisionMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimizationReport {
    pub original_event_count: usize,
    pub minimized_event_count: usize,
    pub removed_event_count: usize,
    pub oracle_calls: usize,
    /// True only after every single-event causal removal was checked and
    /// rejected as a preservation of the exact target failure. Structurally
    /// invalid removals are rejected before invoking the replay oracle.
    pub one_minimal: bool,
    /// True when the oracle-call budget prevented completing minimization or
    /// its final one-minimality check.
    pub budget_exhausted: bool,
    pub preserved_failure: FailureSignature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MinimizationResult {
    pub bundle: TraceBundle,
    pub report: MinimizationReport,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn nonzero(value: &Commitment) -> bool {
    value.iter().any(|byte| *byte != 0)
}

fn event_index(events: &[TraceEvent], event_id: &str) -> Option<usize> {
    events.iter().position(|event| event.event_id == event_id)
}

fn validate_events(
    events: &[TraceEvent],
    target_failure: &FailureSignature,
) -> Result<(), TraceHoldReason> {
    if events.is_empty() {
        return Err(TraceHoldReason::EmptyTrace);
    }
    if events.len() > MAX_TRACE_EVENTS {
        return Err(TraceHoldReason::TooManyEvents);
    }

    let mut seen = BTreeSet::new();
    let mut prior_sequence = None;
    for (event_position, event) in events.iter().enumerate() {
        if !nonempty(&event.event_id) || !nonempty(&event.actor_id) {
            return Err(TraceHoldReason::InvalidEventIdentity);
        }
        if !seen.insert(event.event_id.as_str()) {
            return Err(TraceHoldReason::DuplicateEventIdentity);
        }
        if prior_sequence.is_some_and(|sequence| event.logical_sequence <= sequence) {
            return Err(TraceHoldReason::InvalidLogicalSequence);
        }
        prior_sequence = Some(event.logical_sequence);

        if !nonzero(&event.input_commitment) || !nonzero(&event.configuration_commitment) {
            return Err(TraceHoldReason::InvalidEventCommitment);
        }

        let mut predecessors = BTreeSet::new();
        for predecessor_id in &event.predecessor_event_ids {
            if !predecessors.insert(predecessor_id.as_str()) {
                return Err(TraceHoldReason::DuplicatePredecessor);
            }
            let Some(predecessor_index) = event_index(events, predecessor_id) else {
                return Err(TraceHoldReason::MissingPredecessor);
            };
            if predecessor_index >= event_position {
                return Err(TraceHoldReason::ForwardDependency);
            }
        }
    }

    if !nonempty(&target_failure.property_id)
        || !nonempty(&target_failure.violation_class)
        || !nonempty(&target_failure.witness_event_id)
        || !nonzero(&target_failure.witness_state_commitment)
    {
        return Err(TraceHoldReason::MissingFailureSignature);
    }
    if !seen.contains(target_failure.witness_event_id.as_str()) {
        return Err(TraceHoldReason::MissingFailureWitness);
    }

    Ok(())
}

pub fn validate_trace_bundle(bundle: &TraceBundle) -> TraceValidation {
    let header = &bundle.header;
    if !nonempty(&header.bundle_id)
        || !nonempty(&header.system_id)
        || !nonempty(&header.implementation_revision)
        || !nonempty(&header.model_revision)
        || !nonempty(&header.fault_profile_revision)
        || !nonempty(&header.oracle_revision)
        || header
            .parent_bundle_id
            .as_ref()
            .is_some_and(|id| !nonempty(id))
    {
        return TraceValidation::Hold(TraceHoldReason::MissingHeaderIdentity);
    }
    if header.format_version != CONTRACT_VERSION {
        return TraceValidation::Hold(TraceHoldReason::UnsupportedFormatVersion);
    }
    if !nonzero(&header.initial_state_commitment) {
        return TraceValidation::Hold(TraceHoldReason::ZeroInitialStateCommitment);
    }
    if bundle.artifacts.len() > MAX_ARTIFACTS {
        return TraceValidation::Hold(TraceHoldReason::TooManyArtifacts);
    }
    let mut artifact_ids = BTreeSet::new();
    for artifact in &bundle.artifacts {
        if !nonempty(&artifact.artifact_id)
            || !nonempty(&artifact.media_type)
            || !nonzero(&artifact.raw_artifact_commitment)
        {
            return TraceValidation::Hold(TraceHoldReason::InvalidArtifact);
        }
        if !artifact_ids.insert(artifact.artifact_id.as_str()) {
            return TraceValidation::Hold(TraceHoldReason::DuplicateArtifact);
        }
    }
    for event in &bundle.events {
        if event.clock_observation.as_ref().is_some_and(|clock| {
            !nonempty(&clock.clock_id)
                || !nonempty(&clock.boot_epoch_id)
                || !nonempty(&clock.unit)
                || clock.resolution == 0
        }) {
            return TraceValidation::Hold(TraceHoldReason::InvalidClockObservation);
        }

        let mut event_artifacts = BTreeSet::new();
        for artifact_id in &event.raw_artifact_ids {
            if !event_artifacts.insert(artifact_id.as_str()) {
                return TraceValidation::Hold(TraceHoldReason::DuplicateEventArtifactReference);
            }
            if !artifact_ids.contains(artifact_id.as_str()) {
                return TraceValidation::Hold(TraceHoldReason::UnknownEventArtifactReference);
            }
        }
    }
    match validate_events(&bundle.events, &header.target_failure) {
        Ok(()) => TraceValidation::Valid,
        Err(reason) => TraceValidation::Hold(reason),
    }
}

fn remove_chunk_with_descendants(
    events: &[TraceEvent],
    initial_removed_ids: &BTreeSet<String>,
) -> Vec<TraceEvent> {
    let mut removed = initial_removed_ids.clone();
    loop {
        let mut changed = false;
        for event in events {
            if removed.contains(&event.event_id) {
                continue;
            }
            if event
                .predecessor_event_ids
                .iter()
                .any(|predecessor| removed.contains(predecessor))
            {
                removed.insert(event.event_id.clone());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    events
        .iter()
        .filter(|event| !removed.contains(&event.event_id))
        .cloned()
        .collect()
}

fn keep_chunk_with_ancestors(
    events: &[TraceEvent],
    initial_keep_ids: &BTreeSet<String>,
) -> Vec<TraceEvent> {
    let mut keep = initial_keep_ids.clone();
    loop {
        let mut changed = false;
        for event in events {
            if keep.contains(&event.event_id) {
                for predecessor in &event.predecessor_event_ids {
                    if keep.insert(predecessor.clone()) {
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    events
        .iter()
        .filter(|event| keep.contains(&event.event_id))
        .cloned()
        .collect()
}

fn chunks(length: usize, count: usize) -> Vec<Vec<usize>> {
    let count = count.max(1).min(length);
    (0..count)
        .map(|index| {
            let start = index * length / count;
            let end = (index + 1) * length / count;
            (start..end).collect()
        })
        .collect()
}

fn preserves_target<O: ReplayOracle>(
    candidate: &[TraceEvent],
    target: &FailureSignature,
    original_bundle: &TraceBundle,
    oracle: &O,
    calls: &mut usize,
    budget: usize,
) -> Option<bool> {
    if *calls >= budget {
        return None;
    }
    if validate_events(candidate, target).is_err() {
        return Some(false);
    }
    let mut candidate_bundle = original_bundle.clone();
    candidate_bundle.events = candidate.to_vec();
    *calls += 1;
    Some(matches!(
        oracle.replay(&candidate_bundle),
        ReplayOutcome::Failure(ref signature) if signature == target
    ))
}

/// Minimize a trace while preserving its exact target failure signature.
///
/// Removal uses causal descendant closure: if a prerequisite is removed, every
/// descendant requiring it is also removed. Inclusion uses ancestor closure:
/// a candidate subset automatically includes every predecessor it requires.
///
/// The algorithm reports one-minimality only after explicitly testing every
/// single-event causal removal. Budget exhaustion never becomes a PASS.
pub fn minimize_trace<O: ReplayOracle>(
    bundle: &TraceBundle,
    oracle: &O,
    max_oracle_calls: usize,
) -> Result<MinimizationResult, MinimizationError> {
    if let TraceValidation::Hold(reason) = validate_trace_bundle(bundle) {
        return Err(MinimizationError::InvalidBundle(reason));
    }
    if oracle.revision() != bundle.header.oracle_revision {
        return Err(MinimizationError::OracleRevisionMismatch);
    }
    if max_oracle_calls == 0 {
        return Err(MinimizationError::InvalidBundle(
            TraceHoldReason::ReplayBudgetTooSmall,
        ));
    }
    if max_oracle_calls > MAX_MINIMIZER_CALLS {
        return Err(MinimizationError::InvalidBundle(
            TraceHoldReason::ReplayBudgetTooLarge,
        ));
    }

    let mut calls = 1_usize;
    if !matches!(
        oracle.replay(bundle),
        ReplayOutcome::Failure(ref signature) if signature == &bundle.header.target_failure
    ) {
        return Err(MinimizationError::BaselineDoesNotReproduce);
    }
    if calls > max_oracle_calls {
        return Err(MinimizationError::InvalidBundle(
            TraceHoldReason::ReplayBudgetTooSmall,
        ));
    }

    let target = bundle.header.target_failure.clone();
    let mut current = bundle.events.clone();
    let mut granularity = 2_usize;
    let mut budget_exhausted = false;

    loop {
        if current.len() < 2 {
            break;
        }
        let current_chunks = chunks(current.len(), granularity.min(current.len()));
        let mut reduced = false;

        for chunk in &current_chunks {
            let remove_ids: BTreeSet<String> = chunk
                .iter()
                .map(|index| current[*index].event_id.clone())
                .collect();
            let candidate = remove_chunk_with_descendants(&current, &remove_ids);
            if candidate.len() == current.len() {
                continue;
            }
            match preserves_target(
                &candidate,
                &target,
                bundle,
                oracle,
                &mut calls,
                max_oracle_calls,
            ) {
                Some(true) => {
                    current = candidate;
                    granularity = granularity.saturating_sub(1).max(2);
                    reduced = true;
                    break;
                }
                Some(false) => {}
                None => {
                    budget_exhausted = true;
                    break;
                }
            }
        }
        if budget_exhausted {
            break;
        }
        if reduced {
            continue;
        }

        // Complements (ancestor-closed) can retain a smaller causal core when
        // deleting a contiguous partition would cut all prerequisites.
        for chunk in &current_chunks {
            let keep_ids: BTreeSet<String> = chunk
                .iter()
                .map(|index| current[*index].event_id.clone())
                .collect();
            let candidate = keep_chunk_with_ancestors(&current, &keep_ids);
            if candidate.len() == current.len() || candidate.is_empty() {
                continue;
            }
            match preserves_target(
                &candidate,
                &target,
                bundle,
                oracle,
                &mut calls,
                max_oracle_calls,
            ) {
                Some(true) => {
                    current = candidate;
                    granularity = granularity.saturating_sub(1).max(2);
                    reduced = true;
                    break;
                }
                Some(false) => {}
                None => {
                    budget_exhausted = true;
                    break;
                }
            }
        }
        if budget_exhausted {
            break;
        }
        if reduced {
            continue;
        }
        if granularity >= current.len() {
            break;
        }
        granularity = (granularity.saturating_mul(2)).min(current.len());
    }

    // A strict one-minimality certificate requires replay of every single-event
    // causal deletion. Do not infer it merely because the partition loop ended.
    let mut one_minimal = !budget_exhausted;
    if one_minimal {
        for event in &current {
            let remove_ids = BTreeSet::from([event.event_id.clone()]);
            let candidate = remove_chunk_with_descendants(&current, &remove_ids);
            if candidate.len() == current.len() {
                continue;
            }
            match preserves_target(
                &candidate,
                &target,
                bundle,
                oracle,
                &mut calls,
                max_oracle_calls,
            ) {
                Some(true) => {
                    one_minimal = false;
                    break;
                }
                Some(false) => {}
                None => {
                    one_minimal = false;
                    budget_exhausted = true;
                    break;
                }
            }
        }
    }

    let mut minimized = bundle.clone();
    minimized.header.parent_bundle_id = Some(bundle.header.bundle_id.clone());
    minimized.header.bundle_id = format!("{}-min-v0.1", bundle.header.bundle_id);
    minimized.events = current;

    if validate_trace_bundle(&minimized) != TraceValidation::Valid {
        return Err(MinimizationError::InvalidBundle(
            TraceHoldReason::MissingFailureWitness,
        ));
    }

    Ok(MinimizationResult {
        report: MinimizationReport {
            original_event_count: bundle.events.len(),
            minimized_event_count: minimized.events.len(),
            removed_event_count: bundle.events.len() - minimized.events.len(),
            oracle_calls: calls,
            one_minimal,
            budget_exhausted,
            preserved_failure: target,
        },
        bundle: minimized,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_failure() -> FailureSignature {
        FailureSignature {
            property_id: "DownstreamFenceClosesPendingCommand".to_owned(),
            violation_class: "old_command_executes_after_fence".to_owned(),
            witness_event_id: "late-old-apply".to_owned(),
            witness_state_commitment: commitment(5),
        }
    }

    struct LateExecutionOracle;

    impl ReplayOracle for LateExecutionOracle {
        fn revision(&self) -> &str {
            "late-execution-oracle-v1"
        }

        fn replay(&self, bundle: &TraceBundle) -> ReplayOutcome {
            let events = &bundle.events;
            let fence = events
                .iter()
                .position(|event| event.kind == TraceEventKind::DownstreamGraphFenced);
            let late_apply = events
                .iter()
                .find(|event| event.kind == TraceEventKind::OldCommandApplied);
            match (fence, late_apply) {
                (Some(fence_index), Some(apply))
                    if events
                        .iter()
                        .position(|event| event.event_id == apply.event_id)
                        .is_some_and(|apply_index| apply_index > fence_index) =>
                {
                    ReplayOutcome::Failure(FailureSignature {
                        property_id: target_failure().property_id,
                        violation_class: target_failure().violation_class,
                        witness_event_id: apply.event_id.clone(),
                        witness_state_commitment: target_failure().witness_state_commitment,
                    })
                }
                _ => ReplayOutcome::Pass,
            }
        }
    }

    struct WrongWitnessStateOracle;

    impl ReplayOracle for WrongWitnessStateOracle {
        fn revision(&self) -> &str {
            "late-execution-oracle-v1"
        }

        fn replay(&self, _bundle: &TraceBundle) -> ReplayOutcome {
            let mut signature = target_failure();
            signature.witness_state_commitment = commitment(77);
            ReplayOutcome::Failure(signature)
        }
    }

    struct PassingOracle;

    impl ReplayOracle for PassingOracle {
        fn revision(&self) -> &str {
            "late-execution-oracle-v1"
        }

        fn replay(&self, _bundle: &TraceBundle) -> ReplayOutcome {
            ReplayOutcome::Pass
        }
    }

    struct WrongSignatureSameRevisionOracle;

    impl ReplayOracle for WrongSignatureSameRevisionOracle {
        fn revision(&self) -> &str {
            "late-execution-oracle-v1"
        }

        fn replay(&self, _bundle: &TraceBundle) -> ReplayOutcome {
            ReplayOutcome::Failure(FailureSignature {
                property_id: "DifferentProperty".to_owned(),
                violation_class: "different_violation".to_owned(),
                witness_event_id: "late-old-apply".to_owned(),
                witness_state_commitment: commitment(99),
            })
        }
    }

    fn commitment(value: u8) -> Commitment {
        [value; 32]
    }

    fn event(id: &str, sequence: u64, kind: TraceEventKind, predecessors: &[&str]) -> TraceEvent {
        TraceEvent {
            event_id: id.to_owned(),
            logical_sequence: sequence,
            kind,
            actor_id: "robot-runtime".to_owned(),
            predecessor_event_ids: predecessors
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            clock_observation: None,
            raw_artifact_ids: Vec::new(),
            input_commitment: commitment(1),
            configuration_commitment: commitment(2),
        }
    }

    fn bundle() -> TraceBundle {
        TraceBundle {
            header: TraceHeader {
                bundle_id: "rrvr-trace-0001".to_owned(),
                parent_bundle_id: None,
                format_version: "0.1".to_owned(),
                system_id: "robot-stack-a".to_owned(),
                implementation_revision: "symtropy-head-a".to_owned(),
                model_revision: "restart-model-v0.1".to_owned(),
                fault_profile_revision: "fault-profile-v0.1".to_owned(),
                oracle_revision: "late-execution-oracle-v1".to_owned(),
                replay_seed: 42,
                initial_state_commitment: commitment(3),
                target_failure: target_failure(),
            },
            events: vec![
                {
                    let mut item = event("boot", 1, TraceEventKind::ProcessStarted, &[]);
                    item.clock_observation = Some(ClockObservation {
                        clock_id: "controller-monotonic".to_owned(),
                        boot_epoch_id: "boot-9".to_owned(),
                        unit: "ns".to_owned(),
                        time_value: 1_000,
                        resolution: 1,
                        uncertainty: 5,
                    });
                    item
                },
                event("old-command", 2, TraceEventKind::CommandQueued, &["boot"]),
                event(
                    "controller-fence",
                    3,
                    TraceEventKind::ControllerQueueFenced,
                    &["boot"],
                ),
                event(
                    "downstream-fence",
                    4,
                    TraceEventKind::DownstreamGraphFenced,
                    &["controller-fence"],
                ),
                {
                    let mut item = event(
                        "late-old-apply",
                        5,
                        TraceEventKind::OldCommandApplied,
                        &["old-command", "downstream-fence"],
                    );
                    item.raw_artifact_ids = vec!["raw-event-stream".to_owned()];
                    item
                },
                event("telemetry", 6, TraceEventKind::TelemetryCaptured, &["boot"]),
                event(
                    "trace-flush",
                    7,
                    TraceEventKind::TraceFlushed,
                    &["telemetry"],
                ),
                event("metrics", 8, TraceEventKind::Other, &["boot"]),
            ],
            artifacts: vec![TraceArtifact {
                artifact_id: "raw-event-stream".to_owned(),
                media_type: "application/octet-stream".to_owned(),
                raw_artifact_commitment: commitment(4),
            }],
        }
    }

    #[test]
    fn every_pinned_tla_next_action_has_one_trace_event_mapping() {
        const EXPECTED_ACTIONS: [&str; 30] = [
            "ReserveAttempt",
            "MarkDispatchStarted",
            "NativeAcceptOld",
            "OldDeviceApplies",
            "OldDeviceDoesNotApply",
            "OldDevicePartiallyApplies",
            "BeginEffectRecordCommit",
            "CommitEffectRecord",
            "AbortEffectRecord",
            "AcknowledgeStoreCommit",
            "AcknowledgeStoreNoCommit",
            "Crash",
            "Restart",
            "FenceControllerQueue",
            "FenceDownstreamExecution",
            "ResolvePersistence",
            "ObserveOldEffect",
            "ObserveUnknownEffect",
            "ObserveDisputedEffect",
            "ResolveMission",
            "IssueRecoveryPermit",
            "CreateFreshTicket",
            "RevokeM1",
            "RevokeM2",
            "ChangeLiveBinding",
            "NativeAcceptRecovery",
            "RecoveryDeviceApplies",
            "RecoveryDeviceDoesNotApply",
            "RecoveryDevicePartiallyApplies",
            "NaiveRetry",
        ];
        let mapped: BTreeSet<&str> = TLA_ACTION_EVENT_MAPPING
            .iter()
            .map(|mapping| mapping.action_name)
            .collect();
        let expected: BTreeSet<&str> = EXPECTED_ACTIONS.into_iter().collect();
        assert_eq!(mapped, expected);
        assert_eq!(TLA_ACTION_EVENT_MAPPING.len(), EXPECTED_ACTIONS.len());
    }

    #[test]
    fn exact_target_failure_is_preserved_while_irrelevant_events_are_removed() {
        let result = minimize_trace(&bundle(), &LateExecutionOracle, 500).expect("minimize");
        assert_eq!(result.report.preserved_failure, target_failure());
        assert_eq!(result.report.minimized_event_count, 5);
        assert_eq!(result.report.removed_event_count, 3);
        assert!(result.report.one_minimal);
        assert!(!result.report.budget_exhausted);
        assert_eq!(
            result
                .bundle
                .events
                .last()
                .expect("witness retained")
                .event_id,
            "late-old-apply"
        );
    }

    #[test]
    fn removing_a_prerequisite_also_removes_its_dependent_descendants() {
        let events = bundle().events;
        let remove = BTreeSet::from(["downstream-fence".to_owned()]);
        let candidate = remove_chunk_with_descendants(&events, &remove);
        assert!(
            !candidate
                .iter()
                .any(|event| event.event_id == "downstream-fence")
        );
        assert!(
            !candidate
                .iter()
                .any(|event| event.event_id == "late-old-apply")
        );
        assert!(
            candidate
                .iter()
                .any(|event| event.event_id == "old-command")
        );
    }

    #[test]
    fn retained_subset_is_closed_over_all_causal_ancestors() {
        let events = bundle().events;
        let keep = BTreeSet::from(["late-old-apply".to_owned()]);
        let candidate = keep_chunk_with_ancestors(&events, &keep);
        let ids: BTreeSet<&str> = candidate
            .iter()
            .map(|event| event.event_id.as_str())
            .collect();
        assert_eq!(
            ids,
            BTreeSet::from([
                "boot",
                "old-command",
                "controller-fence",
                "downstream-fence",
                "late-old-apply"
            ])
        );
    }

    #[test]
    fn a_different_failure_signature_is_not_accepted_as_preservation() {
        assert_eq!(
            minimize_trace(&bundle(), &WrongSignatureSameRevisionOracle, 500),
            Err(MinimizationError::BaselineDoesNotReproduce)
        );
    }

    #[test]
    fn changed_witness_state_commitment_is_not_the_same_failure() {
        assert_eq!(
            minimize_trace(&bundle(), &WrongWitnessStateOracle, 500),
            Err(MinimizationError::BaselineDoesNotReproduce)
        );
    }

    #[test]
    fn replay_oracle_revision_must_match_the_bundle() {
        let mut input = bundle();
        input.header.oracle_revision = "different-revision".to_owned();
        assert_eq!(
            minimize_trace(&input, &LateExecutionOracle, 500),
            Err(MinimizationError::OracleRevisionMismatch)
        );
    }

    #[test]
    fn a_baseline_that_no_longer_reproduces_is_not_minimized() {
        let mut input = bundle();
        input.header.oracle_revision = PassingOracle.revision().to_owned();
        assert_eq!(
            minimize_trace(&input, &PassingOracle, 500),
            Err(MinimizationError::BaselineDoesNotReproduce)
        );
    }

    #[test]
    fn budget_exhaustion_never_claims_one_minimality() {
        let result = minimize_trace(&bundle(), &LateExecutionOracle, 1).expect("baseline replay");
        assert!(!result.report.one_minimal);
        assert!(result.report.budget_exhausted);
        assert_eq!(result.report.minimized_event_count, 8);
    }

    #[test]
    fn zero_budget_is_rejected() {
        assert_eq!(
            minimize_trace(&bundle(), &LateExecutionOracle, 0),
            Err(MinimizationError::InvalidBundle(
                TraceHoldReason::ReplayBudgetTooSmall
            ))
        );
    }

    #[test]
    fn duplicate_events_and_predecessors_hold() {
        let mut input = bundle();
        input.events[1].event_id = "boot".to_owned();
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::DuplicateEventIdentity)
        );

        let mut input = bundle();
        input.events[1].predecessor_event_ids = vec!["boot".to_owned(), "boot".to_owned()];
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::DuplicatePredecessor)
        );
    }

    #[test]
    fn missing_and_forward_predecessors_hold() {
        let mut input = bundle();
        input.events[1].predecessor_event_ids = vec!["missing".to_owned()];
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::MissingPredecessor)
        );

        let mut input = bundle();
        input.events[0].predecessor_event_ids = vec!["old-command".to_owned()];
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::ForwardDependency)
        );
    }

    #[test]
    fn missing_witness_and_zero_commitments_hold() {
        let mut input = bundle();
        input.header.target_failure.witness_event_id = "missing-witness".to_owned();
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::MissingFailureWitness)
        );

        let mut input = bundle();
        input.header.target_failure.witness_state_commitment = commitment(0);
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::MissingFailureSignature)
        );

        let mut input = bundle();
        input.events[0].input_commitment = commitment(0);
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::InvalidEventCommitment)
        );

        let mut input = bundle();
        input.artifacts[0].raw_artifact_commitment = commitment(0);
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::InvalidArtifact)
        );
    }

    #[test]
    fn non_increasing_logical_sequences_hold() {
        let mut input = bundle();
        input.events[1].logical_sequence = 1;
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::InvalidLogicalSequence)
        );
    }

    #[test]
    fn trace_event_and_artifact_inventory_bounds_hold() {
        let mut input = bundle();
        let template = input.events[0].clone();
        for index in 0..MAX_TRACE_EVENTS {
            let mut item = template.clone();
            item.event_id = format!("extra-{index}");
            item.logical_sequence = 100 + index as u64;
            item.predecessor_event_ids.clear();
            input.events.push(item);
        }
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::TooManyEvents)
        );

        let mut input = bundle();
        for index in 0..=MAX_ARTIFACTS {
            input.artifacts.push(TraceArtifact {
                artifact_id: format!("artifact-{index}"),
                media_type: "application/octet-stream".to_owned(),
                raw_artifact_commitment: commitment(9),
            });
        }
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::TooManyArtifacts)
        );
    }

    #[test]
    fn parent_bundle_identity_and_trace_format_are_validated() {
        let mut input = bundle();
        input.header.parent_bundle_id = Some(" ".to_owned());
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::MissingHeaderIdentity)
        );

        let mut input = bundle();
        input.header.format_version = "9.9".to_owned();
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::UnsupportedFormatVersion)
        );
    }

    #[test]
    fn clock_metadata_is_retained_but_not_used_to_assert_cross_clock_order() {
        let mut input = bundle();
        input.events[0].clock_observation = Some(ClockObservation {
            clock_id: "clock-a".to_owned(),
            boot_epoch_id: "boot-a".to_owned(),
            unit: "ticks".to_owned(),
            time_value: 900,
            resolution: 1,
            uncertainty: 3,
        });
        input.events[1].clock_observation = Some(ClockObservation {
            clock_id: "clock-b".to_owned(),
            boot_epoch_id: "boot-b".to_owned(),
            unit: "ns".to_owned(),
            time_value: 5,
            resolution: 1,
            uncertainty: 0,
        });
        assert_eq!(validate_trace_bundle(&input), TraceValidation::Valid);

        let mut invalid = bundle();
        invalid.events[0].clock_observation = Some(ClockObservation {
            clock_id: "clock-a".to_owned(),
            boot_epoch_id: "boot-a".to_owned(),
            unit: "ns".to_owned(),
            time_value: 1,
            resolution: 0,
            uncertainty: 0,
        });
        assert_eq!(
            validate_trace_bundle(&invalid),
            TraceValidation::Hold(TraceHoldReason::InvalidClockObservation)
        );
    }

    #[test]
    fn event_raw_artifact_references_must_resolve_and_be_unique() {
        let mut input = bundle();
        input.events[4].raw_artifact_ids = vec!["missing-artifact".to_owned()];
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::UnknownEventArtifactReference)
        );

        let mut input = bundle();
        input.events[4].raw_artifact_ids =
            vec!["raw-event-stream".to_owned(), "raw-event-stream".to_owned()];
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::DuplicateEventArtifactReference)
        );
    }

    #[test]
    fn artifact_identity_must_be_unique() {
        let mut input = bundle();
        input.artifacts.push(input.artifacts[0].clone());
        assert_eq!(
            validate_trace_bundle(&input),
            TraceValidation::Hold(TraceHoldReason::DuplicateArtifact)
        );
    }

    #[test]
    fn minimized_bundle_retains_parent_and_target_identity() {
        let original = bundle();
        let result = minimize_trace(&original, &LateExecutionOracle, 500).expect("minimize");
        assert_eq!(
            result.bundle.header.parent_bundle_id.as_deref(),
            Some(original.header.bundle_id.as_str())
        );
        assert_eq!(
            result.bundle.header.target_failure,
            original.header.target_failure
        );
        assert_eq!(
            result.bundle.header.initial_state_commitment,
            original.header.initial_state_commitment
        );
    }

    struct AlternatingOracle {
        calls: std::sync::atomic::AtomicUsize,
    }

    impl ReplayOracle for AlternatingOracle {
        fn revision(&self) -> &str {
            "alternating-oracle-v1"
        }

        fn replay(&self, _bundle: &TraceBundle) -> ReplayOutcome {
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if call.is_multiple_of(2) {
                ReplayOutcome::Pass
            } else {
                ReplayOutcome::InvalidTrace
            }
        }
    }

    #[test]
    fn deterministic_replay_check_detects_a_nondeterministic_oracle() {
        let mut input = bundle();
        input.header.oracle_revision = "alternating-oracle-v1".to_owned();
        let oracle = AlternatingOracle {
            calls: std::sync::atomic::AtomicUsize::new(0),
        };
        assert_eq!(
            check_replay_determinism(&input, &oracle, 5),
            Ok(ReplayDeterminism::Nondeterministic {
                runs: 2,
                first: ReplayOutcome::Pass,
                differing: ReplayOutcome::InvalidTrace
            })
        );
    }

    #[test]
    fn deterministic_replay_check_confirms_fixed_oracle_verdicts() {
        let input = bundle();
        assert_eq!(
            check_replay_determinism(&input, &LateExecutionOracle, 3),
            Ok(ReplayDeterminism::Deterministic {
                runs: 3,
                outcome: ReplayOutcome::Failure(target_failure())
            })
        );
    }

    #[test]
    fn repeated_minimization_is_deterministic_for_fixed_oracle_seed_and_budget() {
        let original = bundle();
        let first = minimize_trace(&original, &LateExecutionOracle, 500).expect("first");
        let second = minimize_trace(&original, &LateExecutionOracle, 500).expect("second");
        assert_eq!(first, second);
    }
}
