//! Dependency-free semantic checks for blind scientific-world episode records.
//!
//! This crate validates internal consistency and selected information-flow invariants.
//! It does not authenticate producers, verify signatures, prove ledger completeness,
//! sandbox a policy, or validate the scientific adequacy of a world model.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

/// A typed observation with distinct phenomenon, acquisition, and availability times.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Observation {
    /// Stable observation identity within the episode.
    pub observation_id: String,
    /// Observation channel identity.
    pub channel_id: String,
    /// Model/event time at which the phenomenon occurred.
    pub event_time: u64,
    /// Instrument acquisition time, when separately available.
    pub acquisition_time: Option<u64>,
    /// Earliest time at which the policy could actually consume this observation.
    pub available_at: u64,
}

/// An action that the public packet declares admissible.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionSpec {
    /// Stable action identity.
    pub action_id: String,
    /// Frozen cost charged for attempting this action.
    pub cost_units: u64,
}

/// Public, policy-visible episode metadata after decoding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicEpisodePacket {
    /// Opaque episode ID; must not encode a seed, split, label, or expected answer.
    pub opaque_instance_id: String,
    /// SHA-256-shaped commitment to the public episode contract.
    pub contract_digest: String,
    /// Decision time for the initial public evidence prefix.
    pub initial_decision_time: u64,
    /// Declared observation channels.
    pub observation_channels: Vec<String>,
    /// Declared admissible actions and their frozen costs.
    pub actions: Vec<ActionSpec>,
    /// Initial observations exposed in the packet.
    pub initial_observations: Vec<Observation>,
}

/// A prospective decision's evidence references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidencePrefix {
    /// Decision time at which the policy selected its action.
    pub decision_time: u64,
    /// Exact observation IDs cited as visible inputs to that decision.
    pub observation_ids: Vec<String>,
}

/// Errors produced by the semantic contract checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContractError {
    /// Opaque episode identity is malformed or too short.
    InvalidOpaqueId,
    /// A required SHA-256-shaped digest is malformed.
    InvalidDigest,
    /// A required identifier is empty or malformed.
    InvalidIdentifier,
    /// Observation-channel identities are duplicated.
    DuplicateChannel,
    /// Action identities are duplicated.
    DuplicateAction,
    /// An episode declares no observation channels.
    EmptyObservationChannels,
    /// Observation identities are duplicated.
    DuplicateObservation,
    /// An observation refers to an undeclared channel.
    UnknownObservationChannel,
    /// Phenomenon, acquisition, and availability times are inconsistent.
    InvalidObservationChronology,
    /// An initial observation was not yet available at the packet's initial decision.
    InitialObservationNotYetAvailable,
    /// An evidence prefix contains duplicate observation references.
    DuplicateEvidenceReference,
    /// An evidence prefix references an unknown observation.
    UnknownEvidenceReference,
    /// An evidence prefix uses an observation before it was available.
    EvidenceUsedBeforeAvailable,
    /// An action attempt references an action absent from the public catalogue.
    UndeclaredAction,
    /// The attempt's cost differs from the frozen action cost.
    ActionCostMismatch,
    /// An action attempt has malformed status chronology.
    InvalidActionChronology,
    /// An attempt identity or ordinal is duplicated.
    DuplicateAttempt,
    /// The declared attempt count differs from the retained attempts.
    AttemptDenominatorMismatch,
    /// Attempt ordinals are not contiguous from zero.
    AttemptOrdinalGap,
    /// Request timestamps regress relative to the declared attempt order.
    PreviousAttemptTimeRegression,
    /// A logical effect was attempted again after an unknown outcome.
    RetryAfterUnknownOutcome,
    /// An in-progress attempt exists where a closed ledger was required.
    IncompleteAttempt,
    /// An arithmetic operation overflowed while totaling costs.
    CostOverflow,
    /// Intervention count or charged cost exceeds the frozen budget.
    BudgetExceeded,
    /// Counterfactual branches differ in one or more frozen pre-action inputs.
    CounterfactualContextMismatch,
    /// Counterfactual branches do not represent distinct candidate actions.
    SameCounterfactualAction,
    /// A paired random stream lacks a shared, explicit invariance profile.
    InvalidPairedRandomness,
    /// Independently sampled branches reused the same random-stream identity.
    ReusedIndependentRandomness,
    /// The two branches declare incompatible randomness protocols.
    RandomnessProtocolMismatch,
}

/// Validate the public packet's cross-field invariants.
pub fn validate_public_packet(packet: &PublicEpisodePacket) -> Result<(), ContractError> {
    if packet.opaque_instance_id.len() < 16
        || packet.opaque_instance_id.len() > 96
        || !packet
            .opaque_instance_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err(ContractError::InvalidOpaqueId);
    }
    validate_digest(&packet.contract_digest)?;

    if packet.observation_channels.is_empty() {
        return Err(ContractError::EmptyObservationChannels);
    }
    let mut channels = BTreeSet::new();
    for channel in &packet.observation_channels {
        if !valid_identifier(channel) {
            return Err(ContractError::InvalidIdentifier);
        }
        if !channels.insert(channel.clone()) {
            return Err(ContractError::DuplicateChannel);
        }
    }

    let mut actions = BTreeSet::new();
    for action in &packet.actions {
        if !valid_identifier(&action.action_id) {
            return Err(ContractError::InvalidIdentifier);
        }
        if !actions.insert(&action.action_id) {
            return Err(ContractError::DuplicateAction);
        }
    }

    let mut observations = BTreeSet::new();
    for observation in &packet.initial_observations {
        if !valid_identifier(&observation.observation_id)
            || !valid_identifier(&observation.channel_id)
        {
            return Err(ContractError::InvalidIdentifier);
        }
        if !observations.insert(&observation.observation_id) {
            return Err(ContractError::DuplicateObservation);
        }
        if !channels.contains(&observation.channel_id) {
            return Err(ContractError::UnknownObservationChannel);
        }
        validate_observation_chronology(observation)?;
        if observation.available_at > packet.initial_decision_time {
            return Err(ContractError::InitialObservationNotYetAvailable);
        }
    }
    Ok(())
}

/// Validate that event time does not follow acquisition or policy availability time.
pub fn validate_observation_chronology(
    observation: &Observation,
) -> Result<(), ContractError> {
    if let Some(acquisition_time) = observation.acquisition_time {
        if acquisition_time < observation.event_time
            || observation.available_at < acquisition_time
        {
            return Err(ContractError::InvalidObservationChronology);
        }
    } else if observation.available_at < observation.event_time {
        return Err(ContractError::InvalidObservationChronology);
    }
    Ok(())
}

/// Return true only when an observation was available by the decision cutoff.
pub fn observation_visible_at(observation: &Observation, decision_time: u64) -> bool {
    observation.available_at <= decision_time
}

/// Validate that every cited observation belongs to the visible evidence prefix.
pub fn validate_evidence_prefix(
    observations: &[Observation],
    prefix: &EvidencePrefix,
) -> Result<(), ContractError> {
    let mut by_id = BTreeMap::new();
    for observation in observations {
        validate_observation_chronology(observation)?;
        if by_id
            .insert(observation.observation_id.as_str(), observation)
            .is_some()
        {
            return Err(ContractError::DuplicateObservation);
        }
    }

    let mut references = BTreeSet::new();
    for observation_id in &prefix.observation_ids {
        if !references.insert(observation_id) {
            return Err(ContractError::DuplicateEvidenceReference);
        }
        let observation = by_id
            .get(observation_id.as_str())
            .ok_or(ContractError::UnknownEvidenceReference)?;
        if !observation_visible_at(observation, prefix.decision_time) {
            return Err(ContractError::EvidenceUsedBeforeAvailable);
        }
    }
    Ok(())
}

/// Current state of one attempted intervention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttemptStatus {
    /// Request recorded; no participant acceptance has been established.
    Requested,
    /// Participant accepted the request; execution is not yet resolved.
    Accepted,
    /// Participant rejected the request before execution.
    Rejected,
    /// Participant reports that the intervention was applied.
    Applied,
    /// Participant reports an execution failure.
    Failed,
    /// The attempt timed out without a known terminal effect.
    TimedOut,
    /// The effect outcome is unknown and must be reconciled before retry.
    OutcomeUnknown,
}

/// One append-only intervention-attempt record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionAttempt {
    /// Unique identity for this attempt.
    pub attempt_id: String,
    /// Contiguous zero-based ordering within this ledger.
    pub ordinal: u64,
    /// Logical operation/effect identity across retries.
    pub logical_effect_id: String,
    /// Action identity from the frozen public catalogue.
    pub action_id: String,
    /// Frozen charge for attempting the action, including failed attempts.
    pub cost_units: u64,
    /// Time at which the request was recorded.
    pub requested_at: u64,
    /// Participant acceptance time, when accepted.
    pub accepted_at: Option<u64>,
    /// Terminal disposition time, when resolved.
    pub resolved_at: Option<u64>,
    /// Latest observed attempt disposition.
    pub status: AttemptStatus,
}

/// Frozen resource limits for an episode's intervention ledger.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterventionBudget {
    /// Maximum number of intervention requests, including rejected/failed attempts.
    pub max_attempts: u64,
    /// Maximum total charged attempt cost.
    pub max_cost_units: u64,
}

/// Counts and cost derived from the retained action-attempt records.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AttemptSummary {
    /// Total attempts, including rejected, failed, timed-out, and unresolved requests.
    pub attempted: u64,
    /// Attempts whose disposition is Applied.
    pub applied: u64,
    /// Attempts whose disposition is Rejected.
    pub rejected: u64,
    /// Attempts whose disposition is Failed.
    pub failed: u64,
    /// Attempts whose disposition is TimedOut.
    pub timed_out: u64,
    /// Attempts whose outcome is unknown.
    pub outcome_unknown: u64,
    /// Attempts still requested or accepted but not terminal.
    pub outstanding: u64,
    /// Total cost charged across all attempts.
    pub cost_units: u64,
}

/// Validate an append-only attempt ledger and derive its metric denominators.
///
/// The declared count must come from the runner's independently maintained request
/// counter for completeness claims. Comparing it with the records detects truncation
/// only when that counter is itself trustworthy.
pub fn validate_attempt_ledger(
    actions: &[ActionSpec],
    attempts: &[ActionAttempt],
    declared_attempt_count: u64,
    budget: InterventionBudget,
    require_closed: bool,
) -> Result<AttemptSummary, ContractError> {
    if attempts.len() as u64 != declared_attempt_count {
        return Err(ContractError::AttemptDenominatorMismatch);
    }
    if declared_attempt_count > budget.max_attempts {
        return Err(ContractError::BudgetExceeded);
    }

    let mut action_costs = BTreeMap::new();
    for action in actions {
        if action_costs
            .insert(action.action_id.as_str(), action.cost_units)
            .is_some()
        {
            return Err(ContractError::DuplicateAction);
        }
    }

    let mut attempt_ids = BTreeSet::new();
    let mut ordinals = BTreeSet::new();
    let mut ordered: Vec<&ActionAttempt> = attempts.iter().collect();
    ordered.sort_by_key(|attempt| attempt.ordinal);

    let mut unknown_effects = BTreeSet::new();
    let mut previous_request_time = None;
    let mut summary = AttemptSummary {
        attempted: declared_attempt_count,
        ..AttemptSummary::default()
    };

    for (expected_ordinal, attempt) in ordered.iter().enumerate() {
        if !valid_identifier(&attempt.attempt_id)
            || !valid_identifier(&attempt.logical_effect_id)
        {
            return Err(ContractError::InvalidIdentifier);
        }
        if !attempt_ids.insert(&attempt.attempt_id) || !ordinals.insert(attempt.ordinal) {
            return Err(ContractError::DuplicateAttempt);
        }
        if attempt.ordinal != expected_ordinal as u64 {
            return Err(ContractError::AttemptOrdinalGap);
        }
        if previous_request_time.is_some_and(|previous| attempt.requested_at < previous) {
            return Err(ContractError::PreviousAttemptTimeRegression);
        }
        previous_request_time = Some(attempt.requested_at);

        let expected_cost = action_costs
            .get(attempt.action_id.as_str())
            .ok_or(ContractError::UndeclaredAction)?;
        if attempt.cost_units != *expected_cost {
            return Err(ContractError::ActionCostMismatch);
        }
        if unknown_effects.contains(&attempt.logical_effect_id) {
            return Err(ContractError::RetryAfterUnknownOutcome);
        }

        validate_attempt_chronology(attempt, require_closed)?;

        summary.cost_units = summary
            .cost_units
            .checked_add(attempt.cost_units)
            .ok_or(ContractError::CostOverflow)?;

        match attempt.status {
            AttemptStatus::Requested | AttemptStatus::Accepted => {
                summary.outstanding += 1;
            }
            AttemptStatus::Rejected => summary.rejected += 1,
            AttemptStatus::Applied => summary.applied += 1,
            AttemptStatus::Failed => summary.failed += 1,
            AttemptStatus::TimedOut => summary.timed_out += 1,
            AttemptStatus::OutcomeUnknown => {
                summary.outcome_unknown += 1;
                unknown_effects.insert(attempt.logical_effect_id.clone());
            }
        }
    }

    if summary.cost_units > budget.max_cost_units {
        return Err(ContractError::BudgetExceeded);
    }
    Ok(summary)
}

fn validate_attempt_chronology(
    attempt: &ActionAttempt,
    require_closed: bool,
) -> Result<(), ContractError> {
    let chronology_valid = match attempt.status {
        AttemptStatus::Requested => {
            attempt.accepted_at.is_none() && attempt.resolved_at.is_none()
        }
        AttemptStatus::Accepted => {
            attempt
                .accepted_at
                .is_some_and(|accepted| accepted >= attempt.requested_at)
                && attempt.resolved_at.is_none()
        }
        AttemptStatus::Rejected => {
            attempt.accepted_at.is_none()
                && attempt
                    .resolved_at
                    .is_some_and(|resolved| resolved >= attempt.requested_at)
        }
        AttemptStatus::Applied
        | AttemptStatus::Failed
        | AttemptStatus::TimedOut
        | AttemptStatus::OutcomeUnknown => match (attempt.accepted_at, attempt.resolved_at) {
            (Some(accepted), Some(resolved)) => {
                accepted >= attempt.requested_at && resolved >= accepted
            }
            _ => false,
        },
    };
    if !chronology_valid {
        return Err(ContractError::InvalidActionChronology);
    }
    if require_closed
        && matches!(
            attempt.status,
            AttemptStatus::Requested | AttemptStatus::Accepted
        )
    {
        return Err(ContractError::IncompleteAttempt);
    }
    Ok(())
}

/// Frozen pre-action inputs that must match across counterfactual branches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchContext {
    /// Opaque episode identity.
    pub episode_id: String,
    /// Exact world-model code/content digest.
    pub world_model_digest: String,
    /// Exact selected model/profile digest.
    pub model_profile_digest: String,
    /// Exact initial-state digest.
    pub initial_state_digest: String,
    /// Exact policy-visible observation-history digest.
    pub observation_history_digest: String,
    /// Exact policy/checkpoint digest.
    pub policy_digest: String,
    /// Exact recurrent-state digest.
    pub recurrent_state_digest: String,
    /// Exact candidate-action-set digest.
    pub candidate_action_set_digest: String,
    /// Exact resource-budget digest.
    pub budget_digest: String,
    /// Exact actor-policy digest for interacting agents.
    pub actor_policy_digest: String,
}

/// Randomness semantics used by one counterfactual branch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BranchRandomness {
    /// Deterministic branch with no stochastic stream.
    Deterministic,
    /// Same exogenous stream, permitted only under a qualified invariance profile.
    PairedExogenous {
        /// Identity of the paired exogenous stream.
        stream_digest: String,
        /// Digest of the owner-declared intervention-invariance profile.
        invariance_profile_digest: String,
    },
    /// Independently sampled stream for this branch.
    Independent {
        /// Unique identity of this branch's stochastic stream.
        stream_digest: String,
    },
}

/// One branch in a counterfactual action comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CounterfactualBranch {
    /// Shared frozen pre-action context.
    pub context: BranchContext,
    /// Candidate intervention applied in this branch.
    pub action_id: String,
    /// Branch randomness protocol and stream identity.
    pub randomness: BranchRandomness,
}

/// Validate that two branches share every frozen pre-action input.
pub fn validate_counterfactual_pair(
    left: &CounterfactualBranch,
    right: &CounterfactualBranch,
) -> Result<(), ContractError> {
    if left.context != right.context {
        return Err(ContractError::CounterfactualContextMismatch);
    }
    if !valid_identifier(&left.action_id)
        || !valid_identifier(&right.action_id)
        || left.action_id == right.action_id
    {
        return Err(ContractError::SameCounterfactualAction);
    }
    validate_branch_context(&left.context)?;

    match (&left.randomness, &right.randomness) {
        (BranchRandomness::Deterministic, BranchRandomness::Deterministic) => Ok(()),
        (
            BranchRandomness::PairedExogenous {
                stream_digest: left_stream,
                invariance_profile_digest: left_profile,
            },
            BranchRandomness::PairedExogenous {
                stream_digest: right_stream,
                invariance_profile_digest: right_profile,
            },
        ) if left_stream == right_stream && left_profile == right_profile => {
            validate_digest(left_stream)?;
            validate_digest(left_profile)
        }
        (
            BranchRandomness::Independent {
                stream_digest: left_stream,
            },
            BranchRandomness::Independent {
                stream_digest: right_stream,
            },
        ) if left_stream != right_stream => {
            validate_digest(left_stream)?;
            validate_digest(right_stream)
        }
        (BranchRandomness::PairedExogenous { .. }, BranchRandomness::PairedExogenous { .. }) => {
            Err(ContractError::InvalidPairedRandomness)
        }
        (BranchRandomness::Independent { .. }, BranchRandomness::Independent { .. }) => {
            Err(ContractError::ReusedIndependentRandomness)
        }
        _ => Err(ContractError::RandomnessProtocolMismatch),
    }
}

fn validate_branch_context(context: &BranchContext) -> Result<(), ContractError> {
    if !valid_identifier(&context.episode_id) {
        return Err(ContractError::InvalidIdentifier);
    }
    for digest in [
        context.world_model_digest.as_str(),
        context.model_profile_digest.as_str(),
        context.initial_state_digest.as_str(),
        context.observation_history_digest.as_str(),
        context.policy_digest.as_str(),
        context.recurrent_state_digest.as_str(),
        context.candidate_action_set_digest.as_str(),
        context.budget_digest.as_str(),
        context.actor_policy_digest.as_str(),
    ] {
        validate_digest(digest)?;
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), ContractError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ContractError::InvalidDigest);
    }
    Ok(())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:-".contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        std::iter::repeat_n(ch, 64).collect()
    }

    fn observation(id: &str, available_at: u64) -> Observation {
        Observation {
            observation_id: id.to_owned(),
            channel_id: "response.y".to_owned(),
            event_time: 1,
            acquisition_time: Some(2),
            available_at,
        }
    }

    fn action() -> ActionSpec {
        ActionSpec {
            action_id: "probe.input".to_owned(),
            cost_units: 3,
        }
    }

    fn attempt(status: AttemptStatus) -> ActionAttempt {
        let (accepted_at, resolved_at) = match status {
            AttemptStatus::Requested => (None, None),
            AttemptStatus::Accepted => (Some(2), None),
            AttemptStatus::Rejected => (None, Some(2)),
            AttemptStatus::Applied
            | AttemptStatus::Failed
            | AttemptStatus::TimedOut
            | AttemptStatus::OutcomeUnknown => (Some(2), Some(3)),
        };
        ActionAttempt {
            attempt_id: "attempt-0001".to_owned(),
            ordinal: 0,
            logical_effect_id: "effect-0001".to_owned(),
            action_id: "probe.input".to_owned(),
            cost_units: 3,
            requested_at: 1,
            accepted_at,
            resolved_at,
            status,
        }
    }

    fn branch_context() -> BranchContext {
        BranchContext {
            episode_id: "episode-00000001".to_owned(),
            world_model_digest: digest('a'),
            model_profile_digest: digest('b'),
            initial_state_digest: digest('c'),
            observation_history_digest: digest('d'),
            policy_digest: digest('e'),
            recurrent_state_digest: digest('f'),
            candidate_action_set_digest: digest('1'),
            budget_digest: digest('2'),
            actor_policy_digest: digest('3'),
        }
    }

    #[test]
    fn public_packet_accepts_well_formed_initial_prefix() {
        let packet = PublicEpisodePacket {
            opaque_instance_id: "A9k2mX7pQ4vD8sT1".to_owned(),
            contract_digest: digest('a'),
            initial_decision_time: 3,
            observation_channels: vec!["response.y".to_owned()],
            actions: vec![action()],
            initial_observations: vec![observation("obs-0001", 3)],
        };
        assert_eq!(validate_public_packet(&packet), Ok(()));
    }

    #[test]
    fn packet_rejects_malformed_short_ids_and_not_yet_available_observations() {
        let mut packet = PublicEpisodePacket {
            opaque_instance_id: "short".to_owned(),
            contract_digest: digest('a'),
            initial_decision_time: 3,
            observation_channels: vec!["response.y".to_owned()],
            actions: vec![action()],
            initial_observations: vec![observation("obs-0001", 3)],
        };
        assert_eq!(
            validate_public_packet(&packet),
            Err(ContractError::InvalidOpaqueId)
        );
        packet.opaque_instance_id = "A9k2mX7pQ4vD8sT1".to_owned();
        packet.initial_observations[0].available_at = 4;
        assert_eq!(
            validate_public_packet(&packet),
            Err(ContractError::InitialObservationNotYetAvailable)
        );
    }

    #[test]
    fn observation_chronology_rejects_availability_before_acquisition() {
        let mut item = observation("obs-0001", 3);
        item.available_at = 1;
        assert_eq!(
            validate_observation_chronology(&item),
            Err(ContractError::InvalidObservationChronology)
        );
    }

    #[test]
    fn late_observation_cannot_enter_earlier_evidence_prefix() {
        let observations = vec![observation("obs-0001", 8)];
        let prefix = EvidencePrefix {
            decision_time: 7,
            observation_ids: vec!["obs-0001".to_owned()],
        };
        assert_eq!(
            validate_evidence_prefix(&observations, &prefix),
            Err(ContractError::EvidenceUsedBeforeAvailable)
        );
    }

    #[test]
    fn duplicate_evidence_reference_is_rejected() {
        let observations = vec![observation("obs-0001", 3)];
        let prefix = EvidencePrefix {
            decision_time: 3,
            observation_ids: vec!["obs-0001".to_owned(), "obs-0001".to_owned()],
        };
        assert_eq!(
            validate_evidence_prefix(&observations, &prefix),
            Err(ContractError::DuplicateEvidenceReference)
        );
    }

    #[test]
    fn failed_attempt_remains_in_denominator_and_is_charged() {
        let summary = validate_attempt_ledger(
            &[action()],
            &[attempt(AttemptStatus::Failed)],
            1,
            InterventionBudget {
                max_attempts: 1,
                max_cost_units: 3,
            },
            true,
        )
        .expect("failed attempts remain recorded and charged");
        assert_eq!(summary.attempted, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.cost_units, 3);
    }

    #[test]
    fn attempt_denominator_mismatch_is_rejected() {
        assert_eq!(
            validate_attempt_ledger(
                &[action()],
                &[attempt(AttemptStatus::Applied)],
                2,
                InterventionBudget {
                    max_attempts: 2,
                    max_cost_units: 6,
                },
                true,
            ),
            Err(ContractError::AttemptDenominatorMismatch)
        );
    }

    #[test]
    fn unknown_outcome_blocks_retry_of_same_logical_effect() {
        let first = attempt(AttemptStatus::OutcomeUnknown);
        let mut second = attempt(AttemptStatus::Applied);
        second.attempt_id = "attempt-0002".to_owned();
        second.ordinal = 1;
        let error = validate_attempt_ledger(
            &[action()],
            &[first, second],
            2,
            InterventionBudget {
                max_attempts: 2,
                max_cost_units: 6,
            },
            true,
        );
        assert_eq!(error, Err(ContractError::RetryAfterUnknownOutcome));
    }

    #[test]
    fn budget_counts_rejected_and_failed_attempts() {
        let first = attempt(AttemptStatus::Rejected);
        let mut second = attempt(AttemptStatus::Failed);
        second.attempt_id = "attempt-0002".to_owned();
        second.ordinal = 1;
        second.logical_effect_id = "effect-0002".to_owned();
        assert_eq!(
            validate_attempt_ledger(
                &[action()],
                &[first, second],
                2,
                InterventionBudget {
                    max_attempts: 2,
                    max_cost_units: 5,
                },
                true,
            ),
            Err(ContractError::BudgetExceeded)
        );
    }

    #[test]
    fn counterfactual_rejects_stale_initial_state_or_policy_context() {
        let left = CounterfactualBranch {
            context: branch_context(),
            action_id: "probe.left".to_owned(),
            randomness: BranchRandomness::Deterministic,
        };
        let mut right = left.clone();
        right.action_id = "probe.right".to_owned();
        right.context.initial_state_digest = digest('9');
        assert_eq!(
            validate_counterfactual_pair(&left, &right),
            Err(ContractError::CounterfactualContextMismatch)
        );
    }

    #[test]
    fn paired_randomness_requires_identical_stream_and_invariance_profile() {
        let left = CounterfactualBranch {
            context: branch_context(),
            action_id: "probe.left".to_owned(),
            randomness: BranchRandomness::PairedExogenous {
                stream_digest: digest('4'),
                invariance_profile_digest: digest('5'),
            },
        };
        let mut right = left.clone();
        right.action_id = "probe.right".to_owned();
        right.randomness = BranchRandomness::PairedExogenous {
            stream_digest: digest('6'),
            invariance_profile_digest: digest('5'),
        };
        assert_eq!(
            validate_counterfactual_pair(&left, &right),
            Err(ContractError::InvalidPairedRandomness)
        );
    }

    #[test]
    fn independent_randomness_streams_must_be_distinct() {
        let left = CounterfactualBranch {
            context: branch_context(),
            action_id: "probe.left".to_owned(),
            randomness: BranchRandomness::Independent {
                stream_digest: digest('7'),
            },
        };
        let mut right = left.clone();
        right.action_id = "probe.right".to_owned();
        assert_eq!(
            validate_counterfactual_pair(&left, &right),
            Err(ContractError::ReusedIndependentRandomness)
        );
    }

    #[test]
    fn closed_ledger_rejects_an_unresolved_attempt() {
        assert_eq!(
            validate_attempt_ledger(
                &[action()],
                &[attempt(AttemptStatus::Accepted)],
                1,
                InterventionBudget {
                    max_attempts: 1,
                    max_cost_units: 3,
                },
                true,
            ),
            Err(ContractError::IncompleteAttempt)
        );
    }

    #[test]
    fn request_timestamps_must_follow_attempt_ordinals() {
        let mut first = attempt(AttemptStatus::Applied);
        first.requested_at = 3;
        first.accepted_at = Some(3);
        first.resolved_at = Some(4);
        let mut second = attempt(AttemptStatus::Applied);
        second.attempt_id = "attempt-0002".to_owned();
        second.ordinal = 1;
        second.logical_effect_id = "effect-0002".to_owned();
        second.requested_at = 2;
        second.accepted_at = Some(2);
        second.resolved_at = Some(3);
        assert_eq!(
            validate_attempt_ledger(
                &[action()],
                &[first, second],
                2,
                InterventionBudget {
                    max_attempts: 2,
                    max_cost_units: 6,
                },
                true,
            ),
            Err(ContractError::PreviousAttemptTimeRegression)
        );
    }

    #[test]
    fn valid_paired_exogenous_counterfactual_pair_passes() {
        let left = CounterfactualBranch {
            context: branch_context(),
            action_id: "probe.left".to_owned(),
            randomness: BranchRandomness::PairedExogenous {
                stream_digest: digest('4'),
                invariance_profile_digest: digest('5'),
            },
        };
        let mut right = left.clone();
        right.action_id = "probe.right".to_owned();
        assert_eq!(validate_counterfactual_pair(&left, &right), Ok(()));
    }

    #[test]
    fn malformed_branch_digest_is_rejected() {
        let mut context = branch_context();
        context.policy_digest = "not-a-digest".to_owned();
        let left = CounterfactualBranch {
            context: context.clone(),
            action_id: "probe.left".to_owned(),
            randomness: BranchRandomness::Deterministic,
        };
        let right = CounterfactualBranch {
            context,
            action_id: "probe.right".to_owned(),
            randomness: BranchRandomness::Deterministic,
        };
        assert_eq!(
            validate_counterfactual_pair(&left, &right),
            Err(ContractError::InvalidDigest)
        );
    }

    #[test]
    fn valid_deterministic_counterfactual_pair_passes() {
        let left = CounterfactualBranch {
            context: branch_context(),
            action_id: "probe.left".to_owned(),
            randomness: BranchRandomness::Deterministic,
        };
        let mut right = left.clone();
        right.action_id = "probe.right".to_owned();
        assert_eq!(validate_counterfactual_pair(&left, &right), Ok(()));
    }
}
