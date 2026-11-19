#![forbid(unsafe_code)]

//! Bounded verifier for the clock and communication-step portion of an FMI
//! co-simulation trace. It is not an FMI importer, solver, or energy-balance
//! evaluator.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceError {
    EmptyIdentity(&'static str),
    EmptyFederates,
    EmptyAttempts,
    EmptyBarriers,
    DuplicateFederateId,
    DuplicateAttemptId,
    DuplicateBarrierId,
    UnknownFederate,
    ModelDigestMismatch,
    ClockBindingMismatch,
    InvalidClockScale,
    ClockMappingOverflow,
    ClockMappingNotIntegral,
    ClockMappingMismatch,
    InvalidStepInterval,
    StepOutcomeMismatch,
    EarlyReturnNotAllowed,
    ZeroProgressEarlyReturnUnsupported,
    EventModeNotEnabled,
    EventModeTraceUnsupported,
    TerminationTraceUnsupported,
    RejectedStepAdvanced,
    RejectedStateChanged,
    MissingFederateSteps,
    SequenceGap,
    RetryLinkMismatch,
    RetryStartMismatch,
    RetryTargetWidened,
    AttemptAfterCommit,
    UnresolvedRejectedStep,
    AttemptChainDiscontinuity,
    InitialClockMismatch,
    BarrierParticipantCount,
    DuplicateBarrierParticipant,
    AttemptOrderRegression,
    UnknownBarrierAttempt,
    BarrierAttemptNotTerminal,
    BarrierFederateMismatch,
    BarrierTimeMismatch,
    BarrierStartMismatch,
    BarrierSequenceMismatch,
    BarrierNotMonotonic,
}

impl TraceError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyIdentity(_) => "EMPTY_IDENTITY",
            Self::EmptyFederates => "EMPTY_FEDERATES",
            Self::EmptyAttempts => "EMPTY_ATTEMPTS",
            Self::EmptyBarriers => "EMPTY_BARRIERS",
            Self::DuplicateFederateId => "DUPLICATE_FEDERATE_ID",
            Self::DuplicateAttemptId => "DUPLICATE_ATTEMPT_ID",
            Self::DuplicateBarrierId => "DUPLICATE_BARRIER_ID",
            Self::UnknownFederate => "UNKNOWN_FEDERATE",
            Self::ModelDigestMismatch => "MODEL_DIGEST_MISMATCH",
            Self::ClockBindingMismatch => "CLOCK_BINDING_MISMATCH",
            Self::InvalidClockScale => "INVALID_CLOCK_SCALE",
            Self::ClockMappingOverflow => "CLOCK_MAPPING_OVERFLOW",
            Self::ClockMappingNotIntegral => "CLOCK_MAPPING_NOT_INTEGRAL",
            Self::ClockMappingMismatch => "CLOCK_MAPPING_MISMATCH",
            Self::InvalidStepInterval => "INVALID_STEP_INTERVAL",
            Self::StepOutcomeMismatch => "STEP_OUTCOME_MISMATCH",
            Self::EarlyReturnNotAllowed => "EARLY_RETURN_NOT_ALLOWED",
            Self::ZeroProgressEarlyReturnUnsupported => "ZERO_PROGRESS_EARLY_RETURN_UNSUPPORTED",
            Self::EventModeNotEnabled => "EVENT_MODE_NOT_ENABLED",
            Self::EventModeTraceUnsupported => "EVENT_MODE_TRACE_UNSUPPORTED",
            Self::TerminationTraceUnsupported => "TERMINATION_TRACE_UNSUPPORTED",
            Self::RejectedStepAdvanced => "REJECTED_STEP_ADVANCED",
            Self::RejectedStateChanged => "REJECTED_STATE_CHANGED",
            Self::MissingFederateSteps => "MISSING_FEDERATE_STEPS",
            Self::SequenceGap => "SEQUENCE_GAP",
            Self::RetryLinkMismatch => "RETRY_LINK_MISMATCH",
            Self::RetryStartMismatch => "RETRY_START_MISMATCH",
            Self::RetryTargetWidened => "RETRY_TARGET_WIDENED",
            Self::AttemptAfterCommit => "ATTEMPT_AFTER_COMMIT",
            Self::UnresolvedRejectedStep => "UNRESOLVED_REJECTED_STEP",
            Self::AttemptChainDiscontinuity => "ATTEMPT_CHAIN_DISCONTINUITY",
            Self::InitialClockMismatch => "INITIAL_CLOCK_MISMATCH",
            Self::BarrierParticipantCount => "BARRIER_PARTICIPANT_COUNT",
            Self::DuplicateBarrierParticipant => "DUPLICATE_BARRIER_PARTICIPANT",
            Self::AttemptOrderRegression => "ATTEMPT_ORDER_REGRESSION",
            Self::UnknownBarrierAttempt => "UNKNOWN_BARRIER_ATTEMPT",
            Self::BarrierAttemptNotTerminal => "BARRIER_ATTEMPT_NOT_TERMINAL",
            Self::BarrierFederateMismatch => "BARRIER_FEDERATE_MISMATCH",
            Self::BarrierTimeMismatch => "BARRIER_TIME_MISMATCH",
            Self::BarrierStartMismatch => "BARRIER_START_MISMATCH",
            Self::BarrierSequenceMismatch => "BARRIER_SEQUENCE_MISMATCH",
            Self::BarrierNotMonotonic => "BARRIER_NOT_MONOTONIC",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockBinding {
    pub binding_id: String,
    pub model_clock_id: String,
    pub mapping_digest: String,
    pub local_epoch_tick: i64,
    pub master_epoch_ns: i64,
    pub nanoseconds_per_tick_numerator: i64,
    pub nanoseconds_per_tick_denominator: i64,
}

impl ClockBinding {
    /// Map a model-local integer tick to the master nanosecond clock.
    ///
    /// The rational conversion must be exact; this function never rounds.
    pub fn to_master_ns(&self, local_tick: i64) -> Result<i64, TraceError> {
        if self.nanoseconds_per_tick_numerator <= 0
            || self.nanoseconds_per_tick_denominator <= 0
        {
            return Err(TraceError::InvalidClockScale);
        }

        let delta = i128::from(local_tick) - i128::from(self.local_epoch_tick);
        let scaled = delta
            .checked_mul(i128::from(self.nanoseconds_per_tick_numerator))
            .ok_or(TraceError::ClockMappingOverflow)?;
        let denominator = i128::from(self.nanoseconds_per_tick_denominator);

        if scaled % denominator != 0 {
            return Err(TraceError::ClockMappingNotIntegral);
        }

        let master = i128::from(self.master_epoch_ns)
            .checked_add(scaled / denominator)
            .ok_or(TraceError::ClockMappingOverflow)?;

        i64::try_from(master).map_err(|_| TraceError::ClockMappingOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FederateProfile {
    pub federate_id: String,
    pub model_digest: String,
    pub clock: ClockBinding,
    pub initial_local_tick: i64,
    /// Importer-side opt-in supplied to FMI 3 `fmi3InstantiateCoSimulation`.
    pub early_return_allowed: bool,
    /// Importer/FMU event-mode agreement established at instantiation.
    pub event_mode_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Accepted,
    EarlyReturn,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepAttempt {
    pub attempt_id: String,
    pub federate_id: String,
    pub model_digest: String,
    pub clock_binding_id: String,
    pub clock_mapping_digest: String,
    /// Zero-based communication-step sequence within this run.
    pub sequence: u64,
    /// Retry must name the immediately preceding rejected attempt.
    pub retry_of: Option<String>,
    pub local_start_tick: i64,
    pub local_requested_end_tick: i64,
    /// Actual FMI `lastSuccessfulTime` for Accepted/EarlyReturn; canonical start
    /// marker for Rejected (never copied from undefined `fmi3Discard` outputs).
    pub local_returned_end_tick: i64,
    /// Exact mapped times, retained for audit and checked against the binding.
    /// For Rejected, returned-end is the normalized no-advance marker at start.
    pub master_start_ns: i64,
    pub master_requested_end_ns: i64,
    pub master_returned_end_ns: i64,
    /// Caller-provided state snapshot identities; required to prove no mutation on discard.
    pub state_before_digest: String,
    pub state_after_digest: String,
    /// FMI 3 `eventHandlingNeeded` output from `fmi3DoStep`.
    pub event_handling_needed: bool,
    /// FMI 3 `terminateSimulation` output from `fmi3DoStep`.
    pub terminate_simulation: bool,
    pub outcome: StepOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitBarrier {
    pub barrier_id: String,
    pub sequence: u64,
    pub master_time_ns: i64,
    /// Exactly one terminal attempt from each required federate.
    pub participant_attempt_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTrace {
    pub run_id: String,
    pub federates: Vec<FederateProfile>,
    /// Attempts are kept in delivery/record order; validation never sorts them.
    pub attempts: Vec<StepAttempt>,
    pub barriers: Vec<CommitBarrier>,
}

/// Validate a bounded common-barrier co-simulation trace.
///
/// This v0.1 contract requires one common barrier per communication sequence.
/// A federate may reject and retry a step, or return early; every required
/// federate must then terminate at the same master-clock barrier before the
/// next sequence begins.
pub fn validate_trace(trace: &RunTrace) -> Result<(), TraceError> {
    if trace.run_id.trim().is_empty() {
        return Err(TraceError::EmptyIdentity("run_id"));
    }
    if trace.federates.is_empty() {
        return Err(TraceError::EmptyFederates);
    }
    if trace.attempts.is_empty() {
        return Err(TraceError::EmptyAttempts);
    }
    if trace.barriers.is_empty() {
        return Err(TraceError::EmptyBarriers);
    }

    let mut profiles = BTreeMap::<String, &FederateProfile>::new();
    let mut common_initial_time = None;

    for profile in &trace.federates {
        if profile.federate_id.trim().is_empty()
            || profile.model_digest.trim().is_empty()
            || profile.clock.binding_id.trim().is_empty()
            || profile.clock.model_clock_id.trim().is_empty()
            || profile.clock.mapping_digest.trim().is_empty()
        {
            return Err(TraceError::EmptyIdentity("federate_or_clock"));
        }
        if profiles
            .insert(profile.federate_id.clone(), profile)
            .is_some()
        {
            return Err(TraceError::DuplicateFederateId);
        }

        let initial_time = profile
            .clock
            .to_master_ns(profile.initial_local_tick)?;
        match common_initial_time {
            Some(expected) if expected != initial_time => {
                return Err(TraceError::InitialClockMismatch);
            }
            None => common_initial_time = Some(initial_time),
            _ => {}
        }
    }

    let initial_time = common_initial_time.ok_or(TraceError::EmptyFederates)?;
    let mut attempts_by_sequence =
        BTreeMap::<(String, u64), Vec<&StepAttempt>>::new();
    let mut last_sequence_by_federate = BTreeMap::<String, u64>::new();
    let mut attempt_lookup = BTreeMap::<String, &StepAttempt>::new();

    for attempt in &trace.attempts {
        if attempt.attempt_id.trim().is_empty()
            || attempt.federate_id.trim().is_empty()
            || attempt.model_digest.trim().is_empty()
            || attempt.clock_binding_id.trim().is_empty()
            || attempt.clock_mapping_digest.trim().is_empty()
            || attempt.state_before_digest.trim().is_empty()
            || attempt.state_after_digest.trim().is_empty()
        {
            return Err(TraceError::EmptyIdentity("step_attempt"));
        }
        if attempt_lookup
            .insert(attempt.attempt_id.clone(), attempt)
            .is_some()
        {
            return Err(TraceError::DuplicateAttemptId);
        }

        let profile = profiles
            .get(&attempt.federate_id)
            .ok_or(TraceError::UnknownFederate)?;

        if attempt.model_digest != profile.model_digest {
            return Err(TraceError::ModelDigestMismatch);
        }
        if attempt.clock_binding_id != profile.clock.binding_id
            || attempt.clock_mapping_digest != profile.clock.mapping_digest
        {
            return Err(TraceError::ClockBindingMismatch);
        }
        if attempt.local_start_tick >= attempt.local_requested_end_tick {
            return Err(TraceError::InvalidStepInterval);
        }

        let mapped_start = profile.clock.to_master_ns(attempt.local_start_tick)?;
        let mapped_requested = profile
            .clock
            .to_master_ns(attempt.local_requested_end_tick)?;
        let mapped_returned = profile
            .clock
            .to_master_ns(attempt.local_returned_end_tick)?;

        if mapped_start != attempt.master_start_ns
            || mapped_requested != attempt.master_requested_end_ns
            || mapped_returned != attempt.master_returned_end_ns
        {
            return Err(TraceError::ClockMappingMismatch);
        }
        if mapped_start >= mapped_requested {
            return Err(TraceError::InvalidStepInterval);
        }

        if attempt.event_handling_needed {
            if !profile.event_mode_enabled {
                return Err(TraceError::EventModeNotEnabled);
            }
            // v0.1 has no Event Mode / UpdateDiscreteStates round-trip records.
            return Err(TraceError::EventModeTraceUnsupported);
        }
        if attempt.terminate_simulation {
            // Termination changes the required federation membership and is
            // deliberately not inferred from a time-step record alone.
            return Err(TraceError::TerminationTraceUnsupported);
        }

        match attempt.outcome {
            StepOutcome::Accepted => {
                // FMI 3.0.2 permits lastSuccessfulTime to differ from the
                // requested endpoint even when earlyReturn is false. The
                // actual returned time becomes this trace's commit time.
                if attempt.local_returned_end_tick <= attempt.local_start_tick
                    || attempt.local_returned_end_tick > attempt.local_requested_end_tick
                    || mapped_returned <= mapped_start
                    || mapped_returned > mapped_requested
                {
                    return Err(TraceError::StepOutcomeMismatch);
                }
            }
            StepOutcome::EarlyReturn => {
                if !profile.early_return_allowed {
                    return Err(TraceError::EarlyReturnNotAllowed);
                }
                // FMI permits an early return at the current communication
                // point. That is a zero-progress event/coordination round;
                // v0.1 does not model those rounds and reports them as
                // unsupported, not as an invalid FMI trace.
                if attempt.local_returned_end_tick == attempt.local_start_tick
                    || mapped_returned == mapped_start
                {
                    return Err(TraceError::ZeroProgressEarlyReturnUnsupported);
                }
                if attempt.local_returned_end_tick < attempt.local_start_tick
                    || attempt.local_returned_end_tick >= attempt.local_requested_end_tick
                    || mapped_returned < mapped_start
                    || mapped_returned >= mapped_requested
                {
                    return Err(TraceError::StepOutcomeMismatch);
                }
            }
            StepOutcome::Rejected => {
                // A normalized no-advance/discard disposition. The caller
                // must supply equal before/after state identities; the
                // rejected lastSuccessfulTime is not committed.
                if attempt.local_returned_end_tick != attempt.local_start_tick
                    || mapped_returned != mapped_start
                {
                    return Err(TraceError::RejectedStepAdvanced);
                }
                if attempt.state_before_digest != attempt.state_after_digest {
                    return Err(TraceError::RejectedStateChanged);
                }
            }
        }

        if let Some(previous_sequence) =
            last_sequence_by_federate.get(&attempt.federate_id)
        {
            if attempt.sequence < *previous_sequence {
                return Err(TraceError::AttemptOrderRegression);
            }
        }
        last_sequence_by_federate
            .insert(attempt.federate_id.clone(), attempt.sequence);

        attempts_by_sequence
            .entry((attempt.federate_id.clone(), attempt.sequence))
            .or_default()
            .push(attempt);
    }

    let barrier_count = trace.barriers.len();
    let mut terminal_attempts = BTreeMap::<(String, u64), &StepAttempt>::new();

    for profile in &trace.federates {
        let mut groups = BTreeMap::<u64, Vec<&StepAttempt>>::new();
        for ((federate_id, sequence), attempts) in &attempts_by_sequence {
            if federate_id == &profile.federate_id {
                groups.insert(*sequence, attempts.clone());
            }
        }

        if groups.len() != barrier_count {
            return Err(TraceError::MissingFederateSteps);
        }

        let mut expected_local_start = profile.initial_local_tick;
        for expected_sequence in 0..barrier_count {
            let sequence = u64::try_from(expected_sequence)
                .map_err(|_| TraceError::SequenceGap)?;
            let attempts = groups
                .get(&sequence)
                .ok_or(TraceError::SequenceGap)?;

            if attempts.is_empty() {
                return Err(TraceError::MissingFederateSteps);
            }

            for (index, attempt) in attempts.iter().enumerate() {
                if attempt.sequence != sequence {
                    return Err(TraceError::SequenceGap);
                }
                if attempt.local_start_tick != expected_local_start {
                    return Err(if index == 0 {
                        TraceError::AttemptChainDiscontinuity
                    } else {
                        TraceError::RetryStartMismatch
                    });
                }

                if index == 0 {
                    if attempt.retry_of.is_some() {
                        return Err(TraceError::RetryLinkMismatch);
                    }
                } else {
                    let previous = attempts[index - 1];
                    if previous.outcome != StepOutcome::Rejected {
                        return Err(TraceError::AttemptAfterCommit);
                    }
                    if attempt.retry_of.as_deref() != Some(previous.attempt_id.as_str()) {
                        return Err(TraceError::RetryLinkMismatch);
                    }
                    if attempt.local_start_tick != previous.local_start_tick
                        || attempt.master_start_ns != previous.master_start_ns
                    {
                        return Err(TraceError::RetryStartMismatch);
                    }
                    if attempt.local_requested_end_tick
                        > previous.local_requested_end_tick
                    {
                        return Err(TraceError::RetryTargetWidened);
                    }
                }

                if index + 1 < attempts.len()
                    && attempt.outcome != StepOutcome::Rejected
                {
                    return Err(TraceError::AttemptAfterCommit);
                }
            }

            let terminal = *attempts
                .last()
                .ok_or(TraceError::MissingFederateSteps)?;
            if terminal.outcome == StepOutcome::Rejected {
                return Err(TraceError::UnresolvedRejectedStep);
            }

            let mapped_start = profile.clock.to_master_ns(expected_local_start)?;
            if terminal.master_start_ns != mapped_start {
                return Err(TraceError::AttemptChainDiscontinuity);
            }

            expected_local_start = terminal.local_returned_end_tick;
            terminal_attempts.insert(
                (profile.federate_id.clone(), sequence),
                terminal,
            );
        }
    }

    let mut barrier_ids = BTreeSet::new();
    let mut previous_barrier_time = initial_time;

    for (index, barrier) in trace.barriers.iter().enumerate() {
        if barrier.barrier_id.trim().is_empty() {
            return Err(TraceError::EmptyIdentity("barrier_id"));
        }
        if !barrier_ids.insert(barrier.barrier_id.clone()) {
            return Err(TraceError::DuplicateBarrierId);
        }
        if barrier.master_time_ns <= previous_barrier_time {
            return Err(TraceError::BarrierNotMonotonic);
        }
        if barrier.participant_attempt_ids.len() != profiles.len() {
            return Err(TraceError::BarrierParticipantCount);
        }

        let expected_sequence =
            u64::try_from(index).map_err(|_| TraceError::SequenceGap)?;
        if barrier.sequence != expected_sequence {
            return Err(TraceError::BarrierSequenceMismatch);
        }

        let mut participant_ids = BTreeSet::new();
        let mut participant_federates = BTreeSet::new();

        for attempt_id in &barrier.participant_attempt_ids {
            if !participant_ids.insert(attempt_id) {
                return Err(TraceError::DuplicateBarrierParticipant);
            }
            let attempt = attempt_lookup
                .get(attempt_id)
                .copied()
                .ok_or(TraceError::UnknownBarrierAttempt)?;

            if attempt.sequence != barrier.sequence
                || attempt.outcome == StepOutcome::Rejected
            {
                return Err(TraceError::BarrierAttemptNotTerminal);
            }
            let terminal = terminal_attempts
                .get(&(attempt.federate_id.clone(), barrier.sequence))
                .ok_or(TraceError::BarrierAttemptNotTerminal)?;
            if terminal.attempt_id != attempt.attempt_id {
                return Err(TraceError::BarrierAttemptNotTerminal);
            }
            if !participant_federates.insert(attempt.federate_id.clone()) {
                return Err(TraceError::BarrierFederateMismatch);
            }
            if attempt.master_start_ns != previous_barrier_time {
                return Err(TraceError::BarrierStartMismatch);
            }
            if attempt.master_returned_end_ns != barrier.master_time_ns {
                return Err(TraceError::BarrierTimeMismatch);
            }
        }

        if participant_federates.len() != profiles.len() {
            return Err(TraceError::BarrierParticipantCount);
        }

        previous_barrier_time = barrier.master_time_ns;
    }

    Ok(())
}
