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
    PersistenceCommitAborted,
    PersistenceResolved,
    PhysicalEffectObserved,
    PhysicalNoEffectObserved,
    PhysicalOutcomeUnknown,
    MissionResolutionRecorded,
    RecoveryPermitIssued,
    FreshTicketCreated,
    NativeRecoveryAccepted,
    RecoveryCommandApplied,
    RecoveryCommandNoEffect,
    NaiveRetryAttempted,
    ObserverCommonCauseInjected,
    ObserverSampleCaptured,
    TelemetryCaptured,
    TraceFlushed,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub event_id: String,
    /// Strictly increasing logical order in this serialized trace.
    pub logical_sequence: u64,
    pub kind: TraceEventKind,
    pub actor_id: String,
    /// Every predecessor must appear earlier in the serialized trace.
    pub predecessor_event_ids: Vec<String>,
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

    fn replay(&self, seed: u64, events: &[TraceEvent]) -> ReplayOutcome;
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

    let first = oracle.replay(bundle.header.replay_seed, &bundle.events);
    for run in 2..=repetitions {
        let next = oracle.replay(bundle.header.replay_seed, &bundle.events);
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
        || header.parent_bundle_id.as_ref().is_some_and(|id| !nonempty(id))
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

fn keep_chunk_with_ancestors(events: &[TraceEvent], initial_keep_ids: &BTreeSet<String>) -> Vec<TraceEvent> {
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
    header: &TraceHeader,
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
    *calls += 1;
    Some(matches!(
        oracle.replay(header.replay_seed, candidate),
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
        oracle.replay(bundle.header.replay_seed, &bundle.events),
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
            match preserves_target(&candidate, &target, &bundle.header, oracle, &mut calls, max_oracle_calls) {
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
            match preserves_target(&candidate, &target, &bundle.header, oracle, &mut calls, max_oracle_calls) {
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
            match preserves_target(&candidate, &target, &bundle.header, oracle, &mut calls, max_oracle_calls) {
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
        return Err(MinimizationError::InvalidBundle(TraceHoldReason::MissingFailureWitness));
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
        }
    }

    struct LateExecutionOracle;

    impl ReplayOracle for LateExecutionOracle {
        fn revision(&self) -> &str {
            "late-execution-oracle-v1"
        }

        fn replay(&self, _seed: u64, events: &[TraceEvent]) -> ReplayOutcome {
            let fence = events
                .iter()
                .position(|event| event.kind == TraceEventKind::DownstreamGraphFenced);
            let late_apply = events
                .iter()
                .find(|event| event.kind == TraceEventKind::OldCommandApplied);
            match (fence, late_apply) {
                (Some(fence_index), Some(apply))
                    if events.iter().position(|event| event.event_id == apply.event_id)
                        .is_some_and(|apply_index| apply_index > fence_index) =>
                {
                    ReplayOutcome::Failure(FailureSignature {
                        property_id: target_failure().property_id,
                        violation_class: target_failure().violation_class,
                        witness_event_id: apply.event_id.clone(),
                    })
                }
                _ => ReplayOutcome::Pass,
            }
        }
    }

    struct WrongSignatureSameRevisionOracle;

    impl ReplayOracle for WrongSignatureSameRevisionOracle {
        fn revision(&self) -> &str {
            "late-execution-oracle-v1"
        }

        fn replay(&self, _seed: u64, _events: &[TraceEvent]) -> ReplayOutcome {
            ReplayOutcome::Failure(FailureSignature {
                property_id: "DifferentProperty".to_owned(),
                violation_class: "different_violation".to_owned(),
                witness_event_id: "late-old-apply".to_owned(),
            })
        }
    }

    fn commitment(value: u8) -> Commitment {
        [value; 32]
    }

    fn event(
        id: &str,
        sequence: u64,
        kind: TraceEventKind,
        predecessors: &[&str],
    ) -> TraceEvent {
        TraceEvent {
            event_id: id.to_owned(),
            logical_sequence: sequence,
            kind,
            actor_id: "robot-runtime".to_owned(),
            predecessor_event_ids: predecessors.iter().map(|value| (*value).to_owned()).collect(),
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
                event("boot", 1, TraceEventKind::ProcessStarted, &[]),
                event("old-command", 2, TraceEventKind::CommandQueued, &["boot"]),
                event("controller-fence", 3, TraceEventKind::ControllerQueueFenced, &["boot"]),
                event("downstream-fence", 4, TraceEventKind::DownstreamGraphFenced, &["controller-fence"]),
                event("late-old-apply", 5, TraceEventKind::OldCommandApplied, &["old-command", "downstream-fence"]),
                event("telemetry", 6, TraceEventKind::TelemetryCaptured, &["boot"]),
                event("trace-flush", 7, TraceEventKind::TraceFlushed, &["telemetry"]),
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
    fn exact_target_failure_is_preserved_while_irrelevant_events_are_removed() {
        let result = minimize_trace(&bundle(), &LateExecutionOracle, 500).expect("minimize");
        assert_eq!(result.report.preserved_failure, target_failure());
        assert_eq!(result.report.minimized_event_count, 5);
        assert_eq!(result.report.removed_event_count, 3);
        assert!(result.report.one_minimal);
        assert!(!result.report.budget_exhausted);
        assert_eq!(
            result.bundle.events.last().expect("witness retained").event_id,
            "late-old-apply"
        );
    }

    #[test]
    fn removing_a_prerequisite_also_removes_its_dependent_descendants() {
        let events = bundle().events;
        let remove = BTreeSet::from(["downstream-fence".to_owned()]);
        let candidate = remove_chunk_with_descendants(&events, &remove);
        assert!(!candidate.iter().any(|event| event.event_id == "downstream-fence"));
        assert!(!candidate.iter().any(|event| event.event_id == "late-old-apply"));
        assert!(candidate.iter().any(|event| event.event_id == "old-command"));
    }

    #[test]
    fn retained_subset_is_closed_over_all_causal_ancestors() {
        let events = bundle().events;
        let keep = BTreeSet::from(["late-old-apply".to_owned()]);
        let candidate = keep_chunk_with_ancestors(&events, &keep);
        let ids: BTreeSet<&str> = candidate.iter().map(|event| event.event_id.as_str()).collect();
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
        input.header.oracle_revision = DifferentFailureOracle.revision().to_owned();
        assert_eq!(
            minimize_trace(&input, &DifferentFailureOracle, 500),
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
        assert_eq!(result.bundle.header.target_failure, original.header.target_failure);
        assert_eq!(result.bundle.header.initial_state_commitment, original.header.initial_state_commitment);
    }

    struct AlternatingOracle {
        calls: std::sync::atomic::AtomicUsize,
    }

    impl ReplayOracle for AlternatingOracle {
        fn revision(&self) -> &str {
            "alternating-oracle-v1"
        }

        fn replay(&self, _seed: u64, _events: &[TraceEvent]) -> ReplayOutcome {
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if call % 2 == 0 {
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
