// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration-bound mobility action and physical-outcome evidence contract.
//!
//! This crate checks action applicability against a live platform context and
//! independently validates physical-effect evidence. It does not issue
//! commands, authenticate identities, compute commitments, estimate position,
//! prove sensor independence, or provide runtime-assurance authority.

use std::fmt;

/// Opaque digest supplied by an upstream canonicalization/cryptographic owner.
///
/// This type carries a commitment; it does not calculate or authenticate it.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Digest(pub [u8; 32]);

/// Exact physical platform identity and engineering configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformBinding {
    pub platform_id: String,
    pub configuration_digest: Digest,
}

/// Operational design domain and operating-mode revisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationBinding {
    pub odd_id: String,
    pub odd_revision: Digest,
    pub mode_id: String,
    pub mode_revision: Digest,
}

/// Delegated authority reference, epoch and required scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorityBinding {
    pub authority_id: String,
    pub epoch: u64,
    pub required_scope: String,
}

/// Monotonic clock identity and boot epoch.
///
/// Ticks from different clock or boot epochs are never compared.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockBinding {
    pub clock_id: String,
    pub boot_epoch_id: String,
}

/// Navigation integrity state. These labels are not a substitute for the
/// profile-specific semantics and evidence owned by Mycelix Position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavigationService {
    Qualified,
    Degraded,
    Unqualified,
    Unknown,
}

impl NavigationService {
    fn rank(self) -> Option<u8> {
        match self {
            Self::Qualified => Some(2),
            Self::Degraded => Some(1),
            Self::Unqualified | Self::Unknown => None,
        }
    }

    fn satisfies(self, required: Self) -> bool {
        match (self.rank(), required.rank()) {
            (Some(actual), Some(minimum)) => actual >= minimum,
            _ => false,
        }
    }
}

/// Navigation prerequisites bound into an action ticket.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationRequirement {
    pub profile_digest: Digest,
    pub required_service: NavigationService,
    pub spatial_frame_id: String,
    pub source_id: String,
    pub minimum_source_sequence: u64,
    pub maximum_age_ticks: u64,
}

/// Navigation acquisition time qualified into a comparison clock domain by
/// the upstream time/correlation owner (Mycelix Position #435).
///
/// The normalized reading is comparable with the live platform clock only
/// after the upstream owner has established this mapping. A raw source
/// timestamp, local receipt time, or unqualified device tick is not enough.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NavigationTimeEvidence {
    Qualified {
        source_clock: ClockBinding,
        comparison_clock: ClockBinding,
        normalized_observed_at_ticks: Option<u64>,
        /// Required when source and comparison clock/boot epochs differ.
        normalization_commitment: Option<Digest>,
    },
    /// A source timestamp exists but has no qualified time basis/correlation,
    /// or its semantics do not permit a safe comparison.
    Unqualified { source_clock: Option<ClockBinding> },
    /// The provider supplied no usable source-time qualification.
    Unknown,
}

/// An immutable proposal commitment intended for a single bounded dispatch.
///
/// The caller computes and authenticates ticket_commitment outside this crate.
/// The ticket is an applicability reference, not an actuator command.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTicket {
    pub ticket_commitment: Digest,
    pub mission_id: String,
    pub mission_generation: u64,
    /// M1 mission subgoal identity; must equal the active supervisor ticket.
    pub subgoal_id: String,
    pub action_id: String,
    /// Monotonic M1 attempt number; retries always receive a new attempt.
    pub attempt: u32,
    pub action_digest: Digest,
    /// Exact M1 MissionPlan predicate identifier for this action's expected effect.
    pub expected_effect_predicate_id: String,
    pub platform: PlatformBinding,
    pub operation: OperationBinding,
    pub authority: AuthorityBinding,
    pub clock: ClockBinding,
    pub issued_at_ticks: u64,
    pub expires_at_ticks: u64,
    pub navigation: NavigationRequirement,
    pub expected_observation_frame_id: String,
    pub expected_calibration_id: String,
    pub expected_effect_digest: Digest,
    /// Digest of the approved observer-vs-executor failure-domain policy.
    pub observer_independence_policy_digest: Digest,
}

/// Latest locally available platform/authority/navigation context.
///
/// This snapshot must be obtained at the dispatch boundary, not copied from
/// the planning-time ticket. Source authenticity and snapshot provenance are
/// established by the caller and upstream evidence owners.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiveContext {
    pub mission_id: String,
    pub mission_generation: u64,
    /// Active M1 supervisor ticket fields observed at the commit boundary.
    pub active_subgoal_id: String,
    pub active_action_id: String,
    pub active_attempt: u32,
    pub active_effect_predicate_id: String,
    pub platform: PlatformBinding,
    pub operation: OperationBinding,
    pub authority_id: String,
    pub authority_epoch: u64,
    pub authority_revoked: bool,
    pub authority_scopes: Vec<String>,
    pub clock: ClockBinding,
    pub now_ticks: u64,
    pub navigation_profile_digest: Digest,
    pub navigation_service: NavigationService,
    pub navigation_frame_id: String,
    pub navigation_source_id: String,
    pub navigation_source_sequence: u64,
    pub navigation_time: NavigationTimeEvidence,
}

/// The reason why a ticket cannot be admitted for dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    EmptyRequiredReference,
    InvalidTicketValidity,
    TicketNotYetValid,
    TicketExpired,
    MissionMismatch,
    MissionGenerationMismatch,
    SubgoalMismatch,
    ActionMismatch,
    ActionAttemptMismatch,
    EffectPredicateMismatch,
    PlatformMismatch,
    ConfigurationMismatch,
    OddMismatch,
    OddRevisionMismatch,
    ModeMismatch,
    ModeRevisionMismatch,
    AuthorityMismatch,
    AuthorityEpochMismatch,
    AuthorityRevoked,
    AuthorityScopeMissing,
    ClockMismatch,
    BootEpochMismatch,
    NavigationProfileMismatch,
    NavigationServiceInsufficient,
    NavigationFrameMismatch,
    NavigationSourceMismatch,
    NavigationSequenceStale,
    NavigationTimeUnknown,
    NavigationTimeUnqualified,
    NavigationTimeMissing,
    NavigationTimeComparisonClockMismatch,
    NavigationTimeCorrelationMissing,
    NavigationObservationFromFuture,
    NavigationObservationStale,
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AdmissionError {}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn has_duplicates(values: &[String]) -> bool {
    values
        .iter()
        .enumerate()
        .any(|(index, value)| values[..index].contains(value))
}

/// Recheck ticket applicability against current context immediately before
/// dispatch. Success means only "still applicable to this context".
///
/// The domain-specific runtime-assurance layer must independently evaluate
/// the proposed action, control envelope, plant state and local interlocks.
pub fn validate_ticket_applicability(
    ticket: &ActionTicket,
    live: &LiveContext,
) -> Result<(), AdmissionError> {
    let required_references = [
        ticket.mission_id.as_str(),
        ticket.subgoal_id.as_str(),
        ticket.action_id.as_str(),
        ticket.expected_effect_predicate_id.as_str(),
        ticket.platform.platform_id.as_str(),
        ticket.operation.odd_id.as_str(),
        ticket.operation.mode_id.as_str(),
        ticket.authority.authority_id.as_str(),
        ticket.authority.required_scope.as_str(),
        ticket.clock.clock_id.as_str(),
        ticket.clock.boot_epoch_id.as_str(),
        ticket.navigation.spatial_frame_id.as_str(),
        ticket.navigation.source_id.as_str(),
        ticket.expected_observation_frame_id.as_str(),
        ticket.expected_calibration_id.as_str(),
        live.mission_id.as_str(),
        live.active_subgoal_id.as_str(),
        live.active_action_id.as_str(),
        live.active_effect_predicate_id.as_str(),
        live.platform.platform_id.as_str(),
        live.operation.odd_id.as_str(),
        live.operation.mode_id.as_str(),
        live.authority_id.as_str(),
        live.clock.clock_id.as_str(),
        live.clock.boot_epoch_id.as_str(),
        live.navigation_frame_id.as_str(),
        live.navigation_source_id.as_str(),
    ];
    if required_references.iter().any(|value| !is_nonempty(value)) {
        return Err(AdmissionError::EmptyRequiredReference);
    }

    if ticket.issued_at_ticks >= ticket.expires_at_ticks
        || ticket.navigation.maximum_age_ticks == 0
        || ticket.navigation.required_service.rank().is_none()
    {
        return Err(AdmissionError::InvalidTicketValidity);
    }
    if ticket.clock.clock_id != live.clock.clock_id {
        return Err(AdmissionError::ClockMismatch);
    }
    if ticket.clock.boot_epoch_id != live.clock.boot_epoch_id {
        return Err(AdmissionError::BootEpochMismatch);
    }
    if ticket.issued_at_ticks > live.now_ticks {
        return Err(AdmissionError::TicketNotYetValid);
    }
    if live.now_ticks >= ticket.expires_at_ticks {
        return Err(AdmissionError::TicketExpired);
    }

    if ticket.mission_id != live.mission_id {
        return Err(AdmissionError::MissionMismatch);
    }
    if ticket.mission_generation != live.mission_generation {
        return Err(AdmissionError::MissionGenerationMismatch);
    }
    if ticket.subgoal_id != live.active_subgoal_id {
        return Err(AdmissionError::SubgoalMismatch);
    }
    if ticket.action_id != live.active_action_id {
        return Err(AdmissionError::ActionMismatch);
    }
    if ticket.attempt != live.active_attempt {
        return Err(AdmissionError::ActionAttemptMismatch);
    }
    if ticket.expected_effect_predicate_id != live.active_effect_predicate_id {
        return Err(AdmissionError::EffectPredicateMismatch);
    }
    if ticket.platform.platform_id != live.platform.platform_id {
        return Err(AdmissionError::PlatformMismatch);
    }
    if ticket.platform.configuration_digest != live.platform.configuration_digest {
        return Err(AdmissionError::ConfigurationMismatch);
    }
    if ticket.operation.odd_id != live.operation.odd_id {
        return Err(AdmissionError::OddMismatch);
    }
    if ticket.operation.odd_revision != live.operation.odd_revision {
        return Err(AdmissionError::OddRevisionMismatch);
    }
    if ticket.operation.mode_id != live.operation.mode_id {
        return Err(AdmissionError::ModeMismatch);
    }
    if ticket.operation.mode_revision != live.operation.mode_revision {
        return Err(AdmissionError::ModeRevisionMismatch);
    }
    if ticket.authority.authority_id != live.authority_id {
        return Err(AdmissionError::AuthorityMismatch);
    }
    if ticket.authority.epoch != live.authority_epoch {
        return Err(AdmissionError::AuthorityEpochMismatch);
    }
    if live.authority_revoked {
        return Err(AdmissionError::AuthorityRevoked);
    }
    if !live
        .authority_scopes
        .iter()
        .any(|scope| scope == &ticket.authority.required_scope)
    {
        return Err(AdmissionError::AuthorityScopeMissing);
    }

    if ticket.navigation.profile_digest != live.navigation_profile_digest {
        return Err(AdmissionError::NavigationProfileMismatch);
    }
    if !live
        .navigation_service
        .satisfies(ticket.navigation.required_service)
    {
        return Err(AdmissionError::NavigationServiceInsufficient);
    }
    if ticket.navigation.spatial_frame_id != live.navigation_frame_id {
        return Err(AdmissionError::NavigationFrameMismatch);
    }
    if ticket.navigation.source_id != live.navigation_source_id {
        return Err(AdmissionError::NavigationSourceMismatch);
    }
    if live.navigation_source_sequence < ticket.navigation.minimum_source_sequence {
        return Err(AdmissionError::NavigationSequenceStale);
    }

    // Never compare an unqualified provider timestamp (or a local receipt
    // tick) with the platform monotonic clock. Mycelix Position #435 must
    // establish a qualified mapping into this exact clock/boot epoch first.
    match &live.navigation_time {
        NavigationTimeEvidence::Unknown => {
            return Err(AdmissionError::NavigationTimeUnknown);
        }
        NavigationTimeEvidence::Unqualified { .. } => {
            return Err(AdmissionError::NavigationTimeUnqualified);
        }
        NavigationTimeEvidence::Qualified {
            source_clock,
            comparison_clock,
            normalized_observed_at_ticks,
            normalization_commitment,
        } => {
            if !is_nonempty(&source_clock.clock_id)
                || !is_nonempty(&source_clock.boot_epoch_id)
                || !is_nonempty(&comparison_clock.clock_id)
                || !is_nonempty(&comparison_clock.boot_epoch_id)
            {
                return Err(AdmissionError::EmptyRequiredReference);
            }
            if comparison_clock != &live.clock {
                return Err(AdmissionError::NavigationTimeComparisonClockMismatch);
            }
            if source_clock != comparison_clock && normalization_commitment.is_none() {
                return Err(AdmissionError::NavigationTimeCorrelationMissing);
            }
            let observed_at =
                normalized_observed_at_ticks.ok_or(AdmissionError::NavigationTimeMissing)?;
            if observed_at > live.now_ticks {
                return Err(AdmissionError::NavigationObservationFromFuture);
            }
            if live.now_ticks - observed_at > ticket.navigation.maximum_age_ticks {
                return Err(AdmissionError::NavigationObservationStale);
            }
        }
    }

    Ok(())
}

/// Role asserted by an upstream-authenticated observer identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObserverRole {
    AgentSelfReport,
    IndependentSensor,
    IndependentEvaluator,
}

impl ObserverRole {
    fn is_independent(self) -> bool {
        matches!(self, Self::IndependentSensor | Self::IndependentEvaluator)
    }
}

/// Independently observed physical outcome. This is not a transport receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalOutcomeEvidence {
    pub ticket_commitment: Digest,
    /// Canonical digest of the complete physical-outcome receipt payload.
    /// The caller/evidence owner computes and authenticates it; this crate does not.
    pub content_digest: Digest,
    pub event_id: String,
    pub source_id: String,
    pub executing_controller_id: String,
    /// Canonical dependency domains declared for the observing source.
    /// These must come from an authenticated upstream inventory.
    pub observer_failure_domains: Vec<String>,
    pub observer_role: ObserverRole,
    pub source_sequence: u64,
    pub observed_at_ticks: u64,
    pub clock: ClockBinding,
    pub platform_configuration_digest: Digest,
    pub observation_frame_id: String,
    pub calibration_id: String,
    pub effect_predicate_digest: Digest,
    pub confidence_basis_points: u16,
    pub outcome: PhysicalOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PhysicalOutcome {
    Satisfied,
    NoEffect,
    PartialEffect,
    Unknown,
}

/// Limits for accepting a physical-outcome receipt.
///
/// The last source sequence must be tracked by the caller/M1 event ledger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutcomePolicy {
    /// Clock/boot epoch that produced now_ticks. It must match the ticket and
    /// physical-outcome receipt before any freshness comparison is performed.
    pub clock: ClockBinding,
    pub now_ticks: u64,
    pub maximum_age_ticks: u64,
    pub sequence_source_id: String,
    pub last_source_sequence: u64,
    pub minimum_confidence_basis_points: u16,
    /// Dependency domains of the executor/platform path for this action.
    pub execution_failure_domains: Vec<String>,
    /// Approved observer dependency domains under the ticket-bound policy.
    pub approved_observer_failure_domains: Vec<String>,
    pub observer_independence_policy_digest: Digest,
}

/// Why an outcome receipt cannot establish the expected physical effect.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutcomeError {
    EmptyReference,
    TicketCommitmentMismatch,
    SelfReportNotAccepted,
    ObserverIsExecutingController,
    StaleSourceSequence,
    SequenceSourceMismatch,
    ClockMismatch,
    BootEpochMismatch,
    PolicyClockMismatch,
    OutcomePredatesAction,
    OutcomeFromFuture,
    OutcomeStale,
    ConfigurationMismatch,
    ObservationFrameMismatch,
    CalibrationMismatch,
    PredicateMismatch,
    InvalidConfidence,
    ConfidenceInsufficient,
    IndependencePolicyMismatch,
    FailureDomainPolicyInvalid,
    ObserverFailureDomainUnapproved,
    CommonCauseFailureDomainOverlap,
}

impl fmt::Display for OutcomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for OutcomeError {}

/// Disposition to pass to the M1 mission supervisor after receipt validation.
///
/// Accepted means only that the stated independent observation matched the
/// declared predicate. Domain policy decides whether that predicate is
/// sufficient for a real mission subgoal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutcomeDisposition {
    Accepted,
    RecoveryRequired,
    ReconciliationRequired,
}

/// Validate an independent physical outcome against its action ticket.
///
/// Authentication, canonical digest calculation, source independence,
/// calibration validity and observer qualification must be established by
/// upstream owners. This function checks their declared references and
/// constraints; it cannot prove those facts itself.
pub fn validate_physical_outcome(
    ticket: &ActionTicket,
    evidence: &PhysicalOutcomeEvidence,
    policy: OutcomePolicy,
) -> Result<OutcomeDisposition, OutcomeError> {
    let required_references = [
        evidence.event_id.as_str(),
        evidence.source_id.as_str(),
        evidence.executing_controller_id.as_str(),
        evidence.clock.clock_id.as_str(),
        evidence.clock.boot_epoch_id.as_str(),
        evidence.observation_frame_id.as_str(),
        evidence.calibration_id.as_str(),
    ];
    if required_references.iter().any(|value| !is_nonempty(value)) {
        return Err(OutcomeError::EmptyReference);
    }
    if evidence.ticket_commitment != ticket.ticket_commitment {
        return Err(OutcomeError::TicketCommitmentMismatch);
    }
    if policy.observer_independence_policy_digest
        != ticket.observer_independence_policy_digest
    {
        return Err(OutcomeError::IndependencePolicyMismatch);
    }
    if policy.execution_failure_domains.is_empty()
        || policy.approved_observer_failure_domains.is_empty()
        || evidence.observer_failure_domains.is_empty()
        || policy
            .execution_failure_domains
            .iter()
            .chain(policy.approved_observer_failure_domains.iter())
            .chain(evidence.observer_failure_domains.iter())
            .any(|domain| !is_nonempty(domain))
        || has_duplicates(&policy.execution_failure_domains)
        || has_duplicates(&policy.approved_observer_failure_domains)
        || has_duplicates(&evidence.observer_failure_domains)
    {
        return Err(OutcomeError::FailureDomainPolicyInvalid);
    }
    if evidence
        .observer_failure_domains
        .iter()
        .any(|domain| !policy.approved_observer_failure_domains.contains(domain))
    {
        return Err(OutcomeError::ObserverFailureDomainUnapproved);
    }
    if evidence
        .observer_failure_domains
        .iter()
        .any(|domain| policy.execution_failure_domains.contains(domain))
    {
        return Err(OutcomeError::CommonCauseFailureDomainOverlap);
    }
    if !evidence.observer_role.is_independent() {
        return Err(OutcomeError::SelfReportNotAccepted);
    }
    if evidence.source_id == evidence.executing_controller_id {
        return Err(OutcomeError::ObserverIsExecutingController);
    }
    if evidence.source_id != policy.sequence_source_id {
        return Err(OutcomeError::SequenceSourceMismatch);
    }
    if evidence.source_sequence <= policy.last_source_sequence {
        return Err(OutcomeError::StaleSourceSequence);
    }
    if policy.clock != ticket.clock {
        return Err(OutcomeError::PolicyClockMismatch);
    }
    if ticket.clock.clock_id != evidence.clock.clock_id {
        return Err(OutcomeError::ClockMismatch);
    }
    if ticket.clock.boot_epoch_id != evidence.clock.boot_epoch_id {
        return Err(OutcomeError::BootEpochMismatch);
    }
    if evidence.observed_at_ticks < ticket.issued_at_ticks {
        return Err(OutcomeError::OutcomePredatesAction);
    }
    if evidence.observed_at_ticks > policy.now_ticks {
        return Err(OutcomeError::OutcomeFromFuture);
    }
    if policy.maximum_age_ticks == 0
        || policy.now_ticks - evidence.observed_at_ticks > policy.maximum_age_ticks
    {
        return Err(OutcomeError::OutcomeStale);
    }
    if evidence.platform_configuration_digest != ticket.platform.configuration_digest {
        return Err(OutcomeError::ConfigurationMismatch);
    }
    if evidence.observation_frame_id != ticket.expected_observation_frame_id {
        return Err(OutcomeError::ObservationFrameMismatch);
    }
    if evidence.calibration_id != ticket.expected_calibration_id {
        return Err(OutcomeError::CalibrationMismatch);
    }
    if evidence.effect_predicate_digest != ticket.expected_effect_digest {
        return Err(OutcomeError::PredicateMismatch);
    }
    if evidence.confidence_basis_points > 10_000 || policy.minimum_confidence_basis_points > 10_000
    {
        return Err(OutcomeError::InvalidConfidence);
    }
    if evidence.confidence_basis_points < policy.minimum_confidence_basis_points {
        return Err(OutcomeError::ConfidenceInsufficient);
    }

    let disposition = match evidence.outcome {
        PhysicalOutcome::Satisfied => OutcomeDisposition::Accepted,
        PhysicalOutcome::NoEffect => OutcomeDisposition::RecoveryRequired,
        PhysicalOutcome::PartialEffect | PhysicalOutcome::Unknown => {
            OutcomeDisposition::ReconciliationRequired
        }
    };
    Ok(disposition)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest([byte; 32])
    }

    fn ticket() -> ActionTicket {
        ActionTicket {
            ticket_commitment: digest(1),
            mission_id: "mission-1".into(),
            mission_generation: 7,
            subgoal_id: "subgoal-1".into(),
            action_id: "action-1".into(),
            attempt: 1,
            action_digest: digest(2),
            expected_effect_predicate_id: "predicate.reached-waypoint".into(),
            platform: PlatformBinding {
                platform_id: "vehicle-1".into(),
                configuration_digest: digest(3),
            },
            operation: OperationBinding {
                odd_id: "odd-coastal-day".into(),
                odd_revision: digest(4),
                mode_id: "autonomous-supervised".into(),
                mode_revision: digest(5),
            },
            authority: AuthorityBinding {
                authority_id: "operator-delegation-1".into(),
                epoch: 9,
                required_scope: "mission.navigate".into(),
            },
            clock: ClockBinding {
                clock_id: "monotonic-clock-1".into(),
                boot_epoch_id: "boot-epoch-4".into(),
            },
            issued_at_ticks: 1_000,
            expires_at_ticks: 2_000,
            navigation: NavigationRequirement {
                profile_digest: digest(6),
                required_service: NavigationService::Degraded,
                spatial_frame_id: "frame.local-enu-1".into(),
                source_id: "navigation-sensor-1".into(),
                minimum_source_sequence: 20,
                maximum_age_ticks: 100,
            },
            expected_observation_frame_id: "frame.local-enu-1".into(),
            expected_calibration_id: "calibration-camera-1".into(),
            expected_effect_digest: digest(7),
            observer_independence_policy_digest: digest(40),
        }
    }

    fn live_context() -> LiveContext {
        let action = ticket();
        LiveContext {
            mission_id: action.mission_id.clone(),
            mission_generation: action.mission_generation,
            active_subgoal_id: action.subgoal_id.clone(),
            active_action_id: action.action_id.clone(),
            active_attempt: action.attempt,
            active_effect_predicate_id: action.expected_effect_predicate_id.clone(),
            platform: action.platform.clone(),
            operation: action.operation.clone(),
            authority_id: action.authority.authority_id.clone(),
            authority_epoch: action.authority.epoch,
            authority_revoked: false,
            authority_scopes: vec!["mission.navigate".into()],
            clock: action.clock.clone(),
            now_ticks: 1_100,
            navigation_profile_digest: action.navigation.profile_digest,
            navigation_service: NavigationService::Qualified,
            navigation_frame_id: action.navigation.spatial_frame_id.clone(),
            navigation_source_id: action.navigation.source_id.clone(),
            navigation_source_sequence: 22,
            navigation_time: NavigationTimeEvidence::Qualified {
                source_clock: action.clock.clone(),
                comparison_clock: action.clock.clone(),
                normalized_observed_at_ticks: Some(1_090),
                normalization_commitment: None,
            },
        }
    }

    fn physical_evidence() -> PhysicalOutcomeEvidence {
        let action = ticket();
        PhysicalOutcomeEvidence {
            ticket_commitment: action.ticket_commitment,
            content_digest: digest(8),
            event_id: "event-1".into(),
            source_id: "independent-camera-1".into(),
            executing_controller_id: "vehicle-controller-1".into(),
            observer_failure_domains: vec![
                "camera-sensor".into(),
                "camera-processor".into(),
                "observer-power".into(),
            ],
            observer_role: ObserverRole::IndependentSensor,
            source_sequence: 31,
            observed_at_ticks: 1_150,
            clock: action.clock.clone(),
            platform_configuration_digest: action.platform.configuration_digest,
            observation_frame_id: action.expected_observation_frame_id.clone(),
            calibration_id: action.expected_calibration_id.clone(),
            effect_predicate_digest: action.expected_effect_digest,
            confidence_basis_points: 9_900,
            outcome: PhysicalOutcome::Satisfied,
        }
    }

    fn outcome_policy() -> OutcomePolicy {
        OutcomePolicy {
            clock: ticket().clock.clone(),
            now_ticks: 1_160,
            maximum_age_ticks: 100,
            sequence_source_id: "independent-camera-1".into(),
            last_source_sequence: 30,
            minimum_confidence_basis_points: 9_000,
            execution_failure_domains: vec![
                "vehicle-controller".into(),
                "vehicle-bus".into(),
                "vehicle-power".into(),
            ],
            approved_observer_failure_domains: vec![
                "camera-sensor".into(),
                "camera-processor".into(),
                "observer-power".into(),
            ],
            observer_independence_policy_digest: digest(40),
        }
    }

    #[test]
    fn valid_current_ticket_is_applicable() {
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live_context()),
            Ok(())
        );
    }

    #[test]
    fn exact_expiry_boundary_is_rejected() {
        let mut live = live_context();
        live.now_ticks = ticket().expires_at_ticks;
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::TicketExpired)
        );
    }

    #[test]
    fn configuration_swap_invalidates_ticket() {
        let mut live = live_context();
        live.platform.configuration_digest = digest(99);
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::ConfigurationMismatch)
        );
    }

    #[test]
    fn mission_generation_change_invalidates_ticket() {
        let mut live = live_context();
        live.mission_generation += 1;
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::MissionGenerationMismatch)
        );
    }

    #[test]
    fn odd_and_mode_revision_changes_invalidate_ticket() {
        let mut live = live_context();
        live.operation.odd_revision = digest(90);
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::OddRevisionMismatch)
        );

        let mut live = live_context();
        live.operation.mode_revision = digest(91);
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::ModeRevisionMismatch)
        );
    }

    #[test]
    fn revoked_or_re_issued_authority_invalidates_ticket() {
        let mut live = live_context();
        live.authority_revoked = true;
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::AuthorityRevoked)
        );

        let mut live = live_context();
        live.authority_epoch += 1;
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::AuthorityEpochMismatch)
        );
    }

    #[test]
    fn authority_scope_must_match() {
        let mut live = live_context();
        live.authority_scopes.clear();
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::AuthorityScopeMissing)
        );
    }

    #[test]
    fn clock_and_boot_epoch_must_match() {
        let mut live = live_context();
        live.clock.clock_id = "different-clock".into();
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::ClockMismatch)
        );

        let mut live = live_context();
        live.clock.boot_epoch_id = "restarted-device".into();
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::BootEpochMismatch)
        );
    }

    #[test]
    fn navigation_degradation_frame_and_profile_are_checked() {
        let mut live = live_context();
        live.navigation_service = NavigationService::Unqualified;
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationServiceInsufficient)
        );

        let mut live = live_context();
        live.navigation_frame_id = "wrong-frame".into();
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationFrameMismatch)
        );

        let mut live = live_context();
        live.navigation_profile_digest = digest(88);
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationProfileMismatch)
        );
    }

    #[test]
    fn navigation_freshness_sequence_and_future_time_are_checked() {
        let mut live = live_context();
        live.navigation_time = NavigationTimeEvidence::Qualified {
            source_clock: ticket().clock.clone(),
            comparison_clock: ticket().clock.clone(),
            normalized_observed_at_ticks: Some(900),
            normalization_commitment: None,
        };
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationObservationStale)
        );

        let mut live = live_context();
        live.navigation_source_id = "different-navigation-source".into();
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationSourceMismatch)
        );

        let mut live = live_context();
        live.navigation_source_sequence = 19;
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationSequenceStale)
        );

        let mut live = live_context();
        live.navigation_time = NavigationTimeEvidence::Qualified {
            source_clock: ticket().clock.clone(),
            comparison_clock: ticket().clock.clone(),
            normalized_observed_at_ticks: Some(live.now_ticks + 1),
            normalization_commitment: None,
        };
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationObservationFromFuture)
        );
    }

    #[test]
    fn navigation_freshness_requires_upstream_qualified_time_and_clock_correlation() {
        let mut live = live_context();
        live.navigation_time = NavigationTimeEvidence::Unqualified {
            source_clock: Some(ticket().clock.clone()),
        };
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationTimeUnqualified)
        );

        let mut live = live_context();
        live.navigation_time = NavigationTimeEvidence::Unknown;
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationTimeUnknown)
        );

        let mut live = live_context();
        live.navigation_time = NavigationTimeEvidence::Qualified {
            source_clock: ticket().clock.clone(),
            comparison_clock: ticket().clock.clone(),
            normalized_observed_at_ticks: None,
            normalization_commitment: None,
        };
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationTimeMissing)
        );

        let mut live = live_context();
        let other_clock = ClockBinding {
            clock_id: "phone-clock".into(),
            boot_epoch_id: "phone-boot-1".into(),
        };
        live.navigation_time = NavigationTimeEvidence::Qualified {
            source_clock: other_clock.clone(),
            comparison_clock: other_clock,
            normalized_observed_at_ticks: Some(1_090),
            normalization_commitment: Some(digest(44)),
        };
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationTimeComparisonClockMismatch)
        );

        let mut live = live_context();
        let source_clock = ClockBinding {
            clock_id: "sensor-clock".into(),
            boot_epoch_id: "sensor-boot-3".into(),
        };
        live.navigation_time = NavigationTimeEvidence::Qualified {
            source_clock,
            comparison_clock: ticket().clock.clone(),
            normalized_observed_at_ticks: Some(1_090),
            normalization_commitment: None,
        };
        assert_eq!(
            validate_ticket_applicability(&ticket(), &live),
            Err(AdmissionError::NavigationTimeCorrelationMissing)
        );

        let mut live = live_context();
        let source_clock = ClockBinding {
            clock_id: "sensor-clock".into(),
            boot_epoch_id: "sensor-boot-3".into(),
        };
        live.navigation_time = NavigationTimeEvidence::Qualified {
            source_clock,
            comparison_clock: ticket().clock.clone(),
            normalized_observed_at_ticks: Some(1_090),
            normalization_commitment: Some(digest(44)),
        };
        assert_eq!(validate_ticket_applicability(&ticket(), &live), Ok(()));
    }

    #[test]
    fn satisfied_effect_requires_matching_independent_evidence() {
        assert_eq!(
            validate_physical_outcome(&ticket(), &physical_evidence(), outcome_policy()),
            Ok(OutcomeDisposition::Accepted)
        );
    }

    #[test]
    fn self_report_and_controller_observation_are_not_independent() {
        let mut evidence = physical_evidence();
        evidence.observer_role = ObserverRole::AgentSelfReport;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::SelfReportNotAccepted)
        );

        let mut evidence = physical_evidence();
        evidence.source_id = evidence.executing_controller_id.clone();
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::ObserverIsExecutingController)
        );
    }

    #[test]
    fn physical_evidence_must_match_ticket_configuration_frame_calibration_and_predicate() {
        let mut evidence = physical_evidence();
        evidence.platform_configuration_digest = digest(80);
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::ConfigurationMismatch)
        );

        let mut evidence = physical_evidence();
        evidence.observation_frame_id = "wrong-frame".into();
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::ObservationFrameMismatch)
        );

        let mut evidence = physical_evidence();
        evidence.calibration_id = "wrong-calibration".into();
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::CalibrationMismatch)
        );

        let mut evidence = physical_evidence();
        evidence.effect_predicate_digest = digest(81);
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::PredicateMismatch)
        );
    }

    #[test]
    fn physical_evidence_must_be_fresh_ordered_and_in_same_time_domain() {
        let mut evidence = physical_evidence();
        evidence.source_id = "other-camera".into();
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::SequenceSourceMismatch)
        );

        let mut evidence = physical_evidence();
        evidence.source_sequence = 30;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::StaleSourceSequence)
        );

        let mut evidence = physical_evidence();
        evidence.observed_at_ticks = 1_000;
        let mut policy = outcome_policy();
        policy.now_ticks = 1_200;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, policy),
            Err(OutcomeError::OutcomeStale)
        );

        let mut evidence = physical_evidence();
        evidence.observed_at_ticks = 1_170;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::OutcomeFromFuture)
        );

        let mut evidence = physical_evidence();
        evidence.clock.boot_epoch_id = "new-boot".into();
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::BootEpochMismatch)
        );
    }

    #[test]
    fn outcome_freshness_policy_must_use_ticket_clock_and_boot_epoch() {
        let mut policy = outcome_policy();
        policy.clock.clock_id = "different-clock".into();
        assert_eq!(
            validate_physical_outcome(&ticket(), &physical_evidence(), policy),
            Err(OutcomeError::PolicyClockMismatch)
        );

        let mut policy = outcome_policy();
        policy.clock.boot_epoch_id = "different-boot".into();
        assert_eq!(
            validate_physical_outcome(&ticket(), &physical_evidence(), policy),
            Err(OutcomeError::PolicyClockMismatch)
        );
    }


    #[test]
    fn observer_independence_requires_nonempty_declared_failure_domains() {
        let mut evidence = physical_evidence();
        evidence.observer_failure_domains.clear();
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::FailureDomainPolicyInvalid)
        );

        let mut policy = outcome_policy();
        policy.execution_failure_domains.clear();
        assert_eq!(
            validate_physical_outcome(&ticket(), &physical_evidence(), policy),
            Err(OutcomeError::FailureDomainPolicyInvalid)
        );
    }

    #[test]
    fn common_cause_failure_domain_overlap_rejects_independent_observer() {
        let mut evidence = physical_evidence();
        evidence.observer_failure_domains.push("vehicle-power".into());
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::CommonCauseFailureDomainOverlap)
        );
    }

    #[test]
    fn observer_failure_domains_must_be_approved_by_ticket_bound_policy() {
        let mut evidence = physical_evidence();
        evidence.observer_failure_domains.push("unregistered-clock".into());
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::ObserverFailureDomainUnapproved)
        );
    }

    #[test]
    fn independence_policy_version_is_bound_to_action_ticket() {
        let mut policy = outcome_policy();
        policy.observer_independence_policy_digest = digest(41);
        assert_eq!(
            validate_physical_outcome(&ticket(), &physical_evidence(), policy),
            Err(OutcomeError::IndependencePolicyMismatch)
        );
    }

    #[test]
    fn duplicate_failure_domain_labels_are_rejected() {
        let mut evidence = physical_evidence();
        evidence.observer_failure_domains.push("camera-sensor".into());
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::FailureDomainPolicyInvalid)
        );
    }

    #[test]
    fn ambiguous_or_low_confidence_effect_never_completes() {
        let mut evidence = physical_evidence();
        evidence.confidence_basis_points = 8_999;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::ConfidenceInsufficient)
        );

        let mut evidence = physical_evidence();
        evidence.outcome = PhysicalOutcome::PartialEffect;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Ok(OutcomeDisposition::ReconciliationRequired)
        );

        let mut evidence = physical_evidence();
        evidence.outcome = PhysicalOutcome::Unknown;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Ok(OutcomeDisposition::ReconciliationRequired)
        );
    }

    #[test]
    fn confirmed_no_effect_requires_recovery_not_blind_retry() {
        let mut evidence = physical_evidence();
        evidence.outcome = PhysicalOutcome::NoEffect;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Ok(OutcomeDisposition::RecoveryRequired)
        );
    }

    #[test]
    fn unrelated_ticket_and_invalid_confidence_are_rejected() {
        let mut evidence = physical_evidence();
        evidence.ticket_commitment = digest(200);
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::TicketCommitmentMismatch)
        );

        let mut evidence = physical_evidence();
        evidence.confidence_basis_points = 10_001;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::InvalidConfidence)
        );

        let mut policy = outcome_policy();
        policy.minimum_confidence_basis_points = 10_001;
        assert_eq!(
            validate_physical_outcome(&ticket(), &physical_evidence(), policy),
            Err(OutcomeError::InvalidConfidence)
        );
    }

    #[test]
    fn source_and_observation_time_must_be_present_and_not_precede_action() {
        let mut evidence = physical_evidence();
        evidence.source_id.clear();
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::EmptyReference)
        );

        let mut evidence = physical_evidence();
        evidence.observed_at_ticks = 999;
        assert_eq!(
            validate_physical_outcome(&ticket(), &evidence, outcome_policy()),
            Err(OutcomeError::OutcomePredatesAction)
        );
    }

    #[test]
    fn degraded_navigation_is_accepted_only_when_ticket_profile_allows_it() {
        let mut action = ticket();
        action.navigation.required_service = NavigationService::Qualified;
        let mut live = live_context();
        live.navigation_service = NavigationService::Degraded;
        assert_eq!(
            validate_ticket_applicability(&action, &live),
            Err(AdmissionError::NavigationServiceInsufficient)
        );

        let mut action = ticket();
        action.navigation.required_service = NavigationService::Degraded;
        let mut live = live_context();
        live.navigation_service = NavigationService::Qualified;
        assert_eq!(validate_ticket_applicability(&action, &live), Ok(()));
    }
}
