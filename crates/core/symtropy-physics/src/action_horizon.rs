// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Evidence-bounded action-horizon selection.
//!
//! This module composes already-qualified horizon lower bounds. It does not
//! estimate sensor freshness, validate certificates, predict contact/thermal
//! dynamics, project actions, schedule inference, or authorize actuation.
//! Every qualified duration must already use the same declared application
//! time basis. An unknown bound is never treated as infinity.

/// Number of independently qualified horizon constraints in this contract.
pub const ACTION_HORIZON_CONSTRAINT_COUNT: usize = 8;

/// The evidence source that limits how long a committed action prefix remains
/// admissible. The canonical order is also the deterministic tie-break order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HorizonConstraint {
    ObservationFreshness,
    ChunkExpiry,
    ActionDeadline,
    StateCertificate,
    ContactModelValidity,
    ActuatorEnvelope,
    ThermalDerating,
    RecoveryReserve,
}

impl HorizonConstraint {
    pub const ALL: [Self; ACTION_HORIZON_CONSTRAINT_COUNT] = [
        Self::ObservationFreshness,
        Self::ChunkExpiry,
        Self::ActionDeadline,
        Self::StateCertificate,
        Self::ContactModelValidity,
        Self::ActuatorEnvelope,
        Self::ThermalDerating,
        Self::RecoveryReserve,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ObservationFreshness => "observation_freshness",
            Self::ChunkExpiry => "chunk_expiry",
            Self::ActionDeadline => "action_deadline",
            Self::StateCertificate => "state_certificate",
            Self::ContactModelValidity => "contact_model_validity",
            Self::ActuatorEnvelope => "actuator_envelope",
            Self::ThermalDerating => "thermal_derating",
            Self::RecoveryReserve => "recovery_reserve",
        }
    }
}

/// A single upstream horizon lower bound.
///
/// remaining_ns is the conservative lower bound on time-to-invalidity,
/// already adjusted for source uncertainty and mapped into the common
/// application-time basis. This module does not repair missing clock mappings
/// or subtract timestamp uncertainty a second time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HorizonEvidence {
    Qualified {
        constraint: HorizonConstraint,
        remaining_ns: u64,
        source_id: String,
        timebase_id: String,
    },
    Unknown {
        constraint: HorizonConstraint,
        reason: String,
    },
}

impl HorizonEvidence {
    /// Construct a qualified, identity-bound horizon.
    pub fn qualified(
        constraint: HorizonConstraint,
        remaining_ns: u64,
        source_id: impl Into<String>,
        timebase_id: impl Into<String>,
    ) -> Result<Self, HorizonInputError> {
        let source_id = source_id.into();
        let timebase_id = timebase_id.into();
        if source_id.trim().is_empty() {
            return Err(HorizonInputError::EmptySourceIdentity);
        }
        if timebase_id.trim().is_empty() {
            return Err(HorizonInputError::EmptyTimebaseIdentity);
        }
        Ok(Self::Qualified {
            constraint,
            remaining_ns,
            source_id,
            timebase_id,
        })
    }

    /// Construct an explicitly unknown bound. Unknown must remain visible;
    /// it is not a large duration and cannot be omitted.
    pub fn unknown(
        constraint: HorizonConstraint,
        reason: impl Into<String>,
    ) -> Result<Self, HorizonInputError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(HorizonInputError::EmptyUnknownReason);
        }
        Ok(Self::Unknown { constraint, reason })
    }
}

/// Execution architecture changes the meaning of inference latency.
///
/// Queued open-loop execution needs enough already-committed actions to cover
/// measured p99 deliberation/verification latency. A split-timescale
/// architecture has a continuously running local action expert, so slow
/// world-context inference is advisory and does not impose that queue minimum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionExecutionMode {
    QueuedOpenLoop,
    SplitTimescaleLocalExpert,
}

/// The eight evidence entries must correspond to HorizonConstraint::ALL in
/// that exact order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionHorizonRequest {
    pub bounds: [HorizonEvidence; ACTION_HORIZON_CONSTRAINT_COUNT],
    /// Application-monotonic instant at which every remaining horizon was evaluated.
    pub evaluation_time_ns: u64,
    /// Requested policy action-prefix length in low-level control cycles.
    /// The selector must accept or reject this exact K; it never clamps it.
    pub requested_cycles: u64,
    /// Frozen low-level controller period in nanoseconds.
    pub controller_period_ns: u64,
    /// Measured p99 observation-cutoff-to-controller-admission latency.
    pub p99_total_latency_ns: u64,
    /// Identity for the measured hardware/network/contention latency stratum.
    pub latency_profile_id: String,
    pub execution_mode: ActionExecutionMode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HorizonInputError {
    ZeroControllerPeriod,
    EmptySourceIdentity,
    EmptyTimebaseIdentity,
    EmptyUnknownReason,
    EmptyLatencyProfileIdentity,
    ZeroRequestedCycles,
}

impl core::fmt::Display for HorizonInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroControllerPeriod => write!(f, "controller period must be non-zero"),
            Self::EmptySourceIdentity => write!(f, "qualified horizon source identity is empty"),
            Self::EmptyTimebaseIdentity => write!(f, "qualified horizon timebase identity is empty"),
            Self::EmptyUnknownReason => write!(f, "unknown horizon reason is empty"),
            Self::EmptyLatencyProfileIdentity => write!(f, "p99 latency profile identity is empty"),
            Self::ZeroRequestedCycles => write!(f, "requested action prefix must contain at least one control cycle"),
        }
    }
}

impl std::error::Error for HorizonInputError {}

/// Auditable copy of one input bound. Included in every result so the active
/// minimum can be traced to its owner and evidence source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HorizonBoundReceipt<'a> {
    /// Canonical constraint expected for this array slot.
    pub constraint: HorizonConstraint,
    /// Constraint declared by the input evidence item.
    pub reported_constraint: HorizonConstraint,
    pub remaining_ns: Option<u64>,
    pub source_id: Option<&'a str>,
    pub timebase_id: Option<&'a str>,
    pub unknown_reason: Option<&'a str>,
}

/// Why the requested horizon was or was not admissible.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HorizonDisposition {
    /// At least one required upstream bound is unknown.
    UnqualifiedEvidence,
    /// All bounds are known but are not mapped to one common timebase.
    IncompatibleTimebases,
    /// An evidence item declared a different constraint than its canonical slot.
    ConstraintOrderMismatch,
    /// A public enum variant bypassed the validating constructor with a blank identity.
    InvalidEvidenceIdentity,
    /// The safe duration does not contain one complete control cycle.
    NoCompleteActionCycle,
    /// The policy's requested prefix exceeds the qualified safe maximum.
    RequestedHorizonExceedsBound,
    /// The queued prefix is shorter than the minimum required to cover p99 latency.
    RequestedHorizonBelowQueueCoverage,
    /// Required open-loop queue coverage exceeds the safe maximum.
    QueueCoverageInfeasible,
    /// Longest whole-cycle prefix fits every qualified horizon and covers p99.
    AdmissibleQueuedOpenLoop,
    /// Longest whole-cycle prefix is safe while a local expert remains active.
    AdmissibleSplitTimescale,
}

/// Result of applying the horizon-composition contract.
///
/// selected_cycles is populated only for an admissible result. For queued
/// execution it is the longest chunk in [K_min, K_max]; for split-timescale
/// it is K_max. The caller still needs its independent safety authority.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionHorizonSelection<'a> {
    // Private fields prevent callers from fabricating or mutating an admissible
    // selector result before it is bound into an ActionCommitment.
    disposition: HorizonDisposition,
    evaluated_at_ns: u64,
    bounds: [HorizonBoundReceipt<'a>; ACTION_HORIZON_CONSTRAINT_COUNT],
    common_timebase_id: Option<&'a str>,
    safe_horizon_ns: Option<u64>,
    /// Active-minimum constraints in canonical order. Slots after count are None.
    active_constraints: [Option<HorizonConstraint>; ACTION_HORIZON_CONSTRAINT_COUNT],
    active_constraint_count: usize,
    controller_period_ns: u64,
    p99_total_latency_ns: u64,
    latency_profile_id: &'a str,
    k_min_queue_cycles: Option<u64>,
    k_max_safe_cycles: Option<u64>,
    selected_cycles: Option<u64>,
}

/// Compose the eight qualified bounds and return the longest admissible
/// whole-cycle action prefix.
///
/// The selector fails closed: Unknown bounds, incompatible timebases,
/// horizons shorter than one controller period, and infeasible queued
/// coverage all prevent a selected action chunk. This function does not
/// dispatch, schedule, or authorize actions.
pub fn select_action_horizon<'a>(
    request: &'a ActionHorizonRequest,
) -> Result<ActionHorizonSelection<'a>, HorizonInputError> {
    if request.controller_period_ns == 0 {
        return Err(HorizonInputError::ZeroControllerPeriod);
    }
    if request.latency_profile_id.trim().is_empty() {
        return Err(HorizonInputError::EmptyLatencyProfileIdentity);
    }
    if request.requested_cycles == 0 {
        return Err(HorizonInputError::ZeroRequestedCycles);
    }

    let mut unknown_count = 0usize;
    let mut common_timebase: Option<&'a str> = None;
    let mut timebase_mismatch = false;
    let mut constraint_mismatch = false;
    let mut invalid_evidence_identity = false;
    let mut safe_horizon_ns = u64::MAX;

    let receipts = core::array::from_fn(|index| {
        let constraint = HorizonConstraint::ALL[index];
        match &request.bounds[index] {
            HorizonEvidence::Qualified {
                constraint: reported_constraint,
                remaining_ns,
                source_id,
                timebase_id,
            } => {
                if source_id.trim().is_empty() || timebase_id.trim().is_empty() {
                    invalid_evidence_identity = true;
                }
                if *reported_constraint != constraint {
                    constraint_mismatch = true;
                }
                match common_timebase {
                    None => common_timebase = Some(timebase_id.as_str()),
                    Some(existing) if existing != timebase_id => timebase_mismatch = true,
                    Some(_) => {}
                }
                safe_horizon_ns = safe_horizon_ns.min(*remaining_ns);
                HorizonBoundReceipt {
                    constraint,
                    reported_constraint: *reported_constraint,
                    remaining_ns: Some(*remaining_ns),
                    source_id: Some(source_id.as_str()),
                    timebase_id: Some(timebase_id.as_str()),
                    unknown_reason: None,
                }
            }
            HorizonEvidence::Unknown { constraint: reported_constraint, reason } => {
                if reason.trim().is_empty() {
                    invalid_evidence_identity = true;
                }
                if *reported_constraint != constraint {
                    constraint_mismatch = true;
                }
                unknown_count += 1;
                HorizonBoundReceipt {
                    constraint,
                    reported_constraint: *reported_constraint,
                    remaining_ns: None,
                    source_id: None,
                    timebase_id: None,
                    unknown_reason: Some(reason.as_str()),
                }
            }
        }
    });

    let k_min_queue_cycles = match request.execution_mode {
        ActionExecutionMode::QueuedOpenLoop => Some(div_ceil(
            request.p99_total_latency_ns,
            request.controller_period_ns,
        )),
        ActionExecutionMode::SplitTimescaleLocalExpert => None,
    };

    let mut selection = ActionHorizonSelection {
        disposition: HorizonDisposition::UnqualifiedEvidence,
        evaluated_at_ns: request.evaluation_time_ns,
        bounds: receipts,
        common_timebase_id: common_timebase,
        safe_horizon_ns: None,
        active_constraints: [None; ACTION_HORIZON_CONSTRAINT_COUNT],
        active_constraint_count: 0,
        controller_period_ns: request.controller_period_ns,
        p99_total_latency_ns: request.p99_total_latency_ns,
        latency_profile_id: request.latency_profile_id.as_str(),
        k_min_queue_cycles,
        k_max_safe_cycles: None,
        selected_cycles: None,
    };

    if invalid_evidence_identity {
        selection.disposition = HorizonDisposition::InvalidEvidenceIdentity;
        return Ok(selection);
    }
    if unknown_count > 0 {
        return Ok(selection);
    }
    if constraint_mismatch {
        selection.disposition = HorizonDisposition::ConstraintOrderMismatch;
        return Ok(selection);
    }
    if timebase_mismatch {
        selection.disposition = HorizonDisposition::IncompatibleTimebases;
        return Ok(selection);
    }

    selection.safe_horizon_ns = Some(safe_horizon_ns);
    for (index, evidence) in request.bounds.iter().enumerate() {
        if let HorizonEvidence::Qualified { remaining_ns, .. } = evidence {
            if *remaining_ns == safe_horizon_ns {
                selection.active_constraints[selection.active_constraint_count] =
                    Some(HorizonConstraint::ALL[index]);
                selection.active_constraint_count += 1;
            }
        }
    }

    let k_max_safe_cycles = safe_horizon_ns / request.controller_period_ns;
    selection.k_max_safe_cycles = Some(k_max_safe_cycles);

    if k_max_safe_cycles == 0 {
        selection.disposition = HorizonDisposition::NoCompleteActionCycle;
        return Ok(selection);
    }

    match request.execution_mode {
        ActionExecutionMode::QueuedOpenLoop => {
            let k_min = selection.k_min_queue_cycles.unwrap_or(0);
            if k_min > k_max_safe_cycles {
                selection.disposition = HorizonDisposition::QueueCoverageInfeasible;
                return Ok(selection);
            }
            if request.requested_cycles > k_max_safe_cycles {
                selection.disposition = HorizonDisposition::RequestedHorizonExceedsBound;
                return Ok(selection);
            }
            if request.requested_cycles < k_min {
                selection.disposition = HorizonDisposition::RequestedHorizonBelowQueueCoverage;
                return Ok(selection);
            }
            selection.disposition = HorizonDisposition::AdmissibleQueuedOpenLoop;
            selection.selected_cycles = Some(request.requested_cycles);
        }
        ActionExecutionMode::SplitTimescaleLocalExpert => {
            if request.requested_cycles > k_max_safe_cycles {
                selection.disposition = HorizonDisposition::RequestedHorizonExceedsBound;
                return Ok(selection);
            }
            selection.disposition = HorizonDisposition::AdmissibleSplitTimescale;
            selection.selected_cycles = Some(request.requested_cycles);
        }
    }

    Ok(selection)
}

impl ActionHorizonSelection<'_> {
    pub const fn disposition(&self) -> HorizonDisposition {
        self.disposition
    }

    pub const fn evaluated_at_ns(&self) -> u64 {
        self.evaluated_at_ns
    }

    pub fn bounds(&self) -> &[HorizonBoundReceipt<'_>; ACTION_HORIZON_CONSTRAINT_COUNT] {
        &self.bounds
    }

    pub const fn common_timebase_id(&self) -> Option<&str> {
        self.common_timebase_id
    }

    pub const fn safe_horizon_ns(&self) -> Option<u64> {
        self.safe_horizon_ns
    }

    pub const fn controller_period_ns(&self) -> u64 {
        self.controller_period_ns
    }

    pub const fn p99_total_latency_ns(&self) -> u64 {
        self.p99_total_latency_ns
    }

    pub const fn latency_profile_id(&self) -> &str {
        self.latency_profile_id
    }

    pub const fn k_min_queue_cycles(&self) -> Option<u64> {
        self.k_min_queue_cycles
    }

    pub const fn k_max_safe_cycles(&self) -> Option<u64> {
        self.k_max_safe_cycles
    }

    pub const fn selected_cycles(&self) -> Option<u64> {
        self.selected_cycles
    }

    pub const fn active_constraint_count(&self) -> usize {
        self.active_constraint_count
    }

    /// Iterate over every constraint tied for the active minimum.
    pub fn active_constraints_iter(&self) -> impl Iterator<Item = HorizonConstraint> + '_ {
        self.active_constraints
            .iter()
            .take(self.active_constraint_count)
            .filter_map(|constraint| *constraint)
    }
}


#[inline]
const fn div_ceil(numerator: u64, denominator: u64) -> u64 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    quotient + if remainder == 0 { 0 } else { 1 }
}

/// Errors returned when an admissible selector result is bound to a proposal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionCommitmentError {
    SelectionNotAdmissible,
    MissingTimebase,
    MissingSelectedCycles,
    DurationOverflow,
    EvaluationTimeMismatch,
    ExpiryOverflow,
    ExceedsSafeHorizon,
    ZeroSequenceNumber,
}

impl core::fmt::Display for ActionCommitmentError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SelectionNotAdmissible => write!(f, "action horizon selection is not admissible"),
            Self::MissingTimebase => write!(f, "admissible selection has no common timebase"),
            Self::MissingSelectedCycles => write!(f, "admissible selection has no selected cycles"),
            Self::DurationOverflow => write!(f, "selected action duration overflowed nanoseconds"),
            Self::EvaluationTimeMismatch => write!(f, "commit time differs from horizon evaluation time"),
            Self::ExpiryOverflow => write!(f, "action commitment expiry overflowed nanoseconds"),
            Self::ExceedsSafeHorizon => write!(f, "selected duration exceeds the safe horizon"),
            Self::ZeroSequenceNumber => write!(f, "action commitment sequence number must be non-zero"),
        }
    }
}

impl std::error::Error for ActionCommitmentError {}

/// Why an existing action commitment must be rejected before dispatch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ActionCommitmentInvalidity {
    NotYetValid,
    Expired,
    TimebaseMismatch,
    ProposalDigestMismatch,
    ConfigurationDigestMismatch,
    SequenceReplay,
}

/// Evidence-carrying permission window for one exact proposed action prefix.
///
/// This is a receipt, not an actuator command or safety certificate. The
/// caller supplies SHA-256 digests of the exact canonical proposal and robot
/// configuration. The local execution authority must still independently
/// validate the command and atomically consume/replay-protect its sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionCommitment<'a> {
    // Private immutable fields prevent a caller from extending expiry, swapping
    // the proposal/configuration identity, or altering the selected horizon.
    proposal_digest: [u8; 32],
    configuration_digest: [u8; 32],
    sequence_number: u64,
    selection: ActionHorizonSelection<'a>,
    timebase_id: &'a str,
    committed_at_ns: u64,
    expires_at_ns: u64,
    selected_cycles: u64,
}

/// Bind an admissible horizon selection to an exact proposal and configuration.
///
/// The selector result is consumed so a caller cannot accidentally create a
/// receipt from one decision and later mutate the evidence object it came from.
/// Call this at the same application-time instant used to evaluate the input
/// horizons. It does not sign, dispatch or authorize the proposal.
pub fn create_action_commitment<'a>(
    selection: ActionHorizonSelection<'a>,
    proposal_digest: [u8; 32],
    configuration_digest: [u8; 32],
    sequence_number: u64,
    committed_at_ns: u64,
) -> Result<ActionCommitment<'a>, ActionCommitmentError> {
    if sequence_number == 0 {
        return Err(ActionCommitmentError::ZeroSequenceNumber);
    }
    if !matches!(
        selection.disposition,
        HorizonDisposition::AdmissibleQueuedOpenLoop
            | HorizonDisposition::AdmissibleSplitTimescale
    ) {
        return Err(ActionCommitmentError::SelectionNotAdmissible);
    }
    if committed_at_ns != selection.evaluated_at_ns {
        return Err(ActionCommitmentError::EvaluationTimeMismatch);
    }
    let timebase_id = selection
        .common_timebase_id
        .ok_or(ActionCommitmentError::MissingTimebase)?;
    let selected_cycles = selection
        .selected_cycles
        .ok_or(ActionCommitmentError::MissingSelectedCycles)?;
    let selected_duration_ns = selected_cycles
        .checked_mul(selection.controller_period_ns)
        .ok_or(ActionCommitmentError::DurationOverflow)?;
    let safe_horizon_ns = selection
        .safe_horizon_ns
        .ok_or(ActionCommitmentError::SelectionNotAdmissible)?;
    if selected_duration_ns > safe_horizon_ns {
        return Err(ActionCommitmentError::ExceedsSafeHorizon);
    }
    let expires_at_ns = committed_at_ns
        .checked_add(selected_duration_ns)
        .ok_or(ActionCommitmentError::ExpiryOverflow)?;

    Ok(ActionCommitment {
        proposal_digest,
        configuration_digest,
        sequence_number,
        selection,
        timebase_id,
        committed_at_ns,
        expires_at_ns,
        selected_cycles,
    })
}

impl ActionCommitment<'_> {
    /// Check whether this receipt still matches the exact proposed action,
    /// robot configuration and application-time basis at the instant of use.
    ///
    /// The caller must separately enforce sequence-number anti-replay using
    /// durable local state and obtain fresh independent actuator authority.
    pub fn validate(
        &self,
        now_ns: u64,
        current_timebase_id: &str,
        proposal_digest: &[u8; 32],
        configuration_digest: &[u8; 32],
        last_accepted_sequence: u64,
    ) -> Result<(), ActionCommitmentInvalidity> {
        if now_ns < self.committed_at_ns {
            return Err(ActionCommitmentInvalidity::NotYetValid);
        }
        if now_ns >= self.expires_at_ns {
            return Err(ActionCommitmentInvalidity::Expired);
        }
        if current_timebase_id != self.timebase_id {
            return Err(ActionCommitmentInvalidity::TimebaseMismatch);
        }
        if proposal_digest != &self.proposal_digest {
            return Err(ActionCommitmentInvalidity::ProposalDigestMismatch);
        }
        if configuration_digest != &self.configuration_digest {
            return Err(ActionCommitmentInvalidity::ConfigurationDigestMismatch);
        }
        if self.sequence_number <= last_accepted_sequence {
            return Err(ActionCommitmentInvalidity::SequenceReplay);
        }
        Ok(())
    }

    pub const fn proposal_digest(&self) -> &[u8; 32] {
        &self.proposal_digest
    }

    pub const fn configuration_digest(&self) -> &[u8; 32] {
        &self.configuration_digest
    }

    pub const fn sequence_number(&self) -> u64 {
        self.sequence_number
    }

    pub const fn selection(&self) -> &ActionHorizonSelection<'_> {
        &self.selection
    }

    pub const fn timebase_id(&self) -> &str {
        self.timebase_id
    }

    pub const fn committed_at_ns(&self) -> u64 {
        self.committed_at_ns
    }

    pub const fn expires_at_ns(&self) -> u64 {
        self.expires_at_ns
    }

    pub const fn selected_cycles(&self) -> u64 {
        self.selected_cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qualified_bounds(
        values_ns: [u64; ACTION_HORIZON_CONSTRAINT_COUNT],
    ) -> [HorizonEvidence; ACTION_HORIZON_CONSTRAINT_COUNT] {
        core::array::from_fn(|index| {
            HorizonEvidence::qualified(
                values_ns[index],
                HorizonConstraint::ALL[index].as_str(),
                "application-monotonic-epoch-7",
            )
            .unwrap()
        })
    }

    fn request(
        values_ns: [u64; ACTION_HORIZON_CONSTRAINT_COUNT],
        execution_mode: ActionExecutionMode,
        period_ns: u64,
        p99_latency_ns: u64,
    ) -> ActionHorizonRequest {
        ActionHorizonRequest {
            bounds: qualified_bounds(values_ns),
            evaluation_time_ns: 100,
            requested_cycles: if period_ns == 0 { 1 } else { values_ns.iter().copied().min().unwrap_or(1) / period_ns },
            controller_period_ns: period_ns,
            p99_total_latency_ns: p99_latency_ns,
            latency_profile_id: "p99-profile/test-target-a".to_owned(),
            execution_mode,
        }
    }

    #[test]
    fn exact_known_answer_uses_minimum_and_integer_floor() {
        let result = select_action_horizon(&request(
            [120, 80, 70, 60, 90, 50, 100, 45],
            ActionExecutionMode::QueuedOpenLoop,
            10,
            25,
        ))
        .unwrap();

        assert_eq!(result.safe_horizon_ns, Some(45));
        assert_eq!(
            result.active_constraints_iter().collect::<Vec<_>>(),
            vec![HorizonConstraint::RecoveryReserve]
        );
        assert_eq!(result.k_max_safe_cycles, Some(4));
        assert_eq!(result.k_min_queue_cycles, Some(3));
        assert_eq!(result.selected_cycles, Some(4));
        assert_eq!(result.disposition, HorizonDisposition::AdmissibleQueuedOpenLoop);
    }

    #[test]
    fn all_constraints_tied_for_minimum_are_preserved() {
        let result = select_action_horizon(&request(
            [40; ACTION_HORIZON_CONSTRAINT_COUNT],
            ActionExecutionMode::SplitTimescaleLocalExpert,
            10,
            10_000,
        ))
        .unwrap();

        assert_eq!(
            result.active_constraints_iter().collect::<Vec<_>>(),
            HorizonConstraint::ALL.to_vec()
        );
        assert_eq!(result.selected_cycles, Some(4));
        assert_eq!(result.k_min_queue_cycles, None);
        assert_eq!(result.disposition, HorizonDisposition::AdmissibleSplitTimescale);
    }

    #[test]
    fn unknown_bound_is_not_dropped_or_treated_as_infinity() {
        let mut bounds = qualified_bounds([100; ACTION_HORIZON_CONSTRAINT_COUNT]);
        bounds[6] = HorizonEvidence::unknown(HorizonConstraint::ThermalDerating, "motor thermal state has no qualified estimate").unwrap();
        let request = ActionHorizonRequest {
            bounds,
            evaluation_time_ns: 100,
            requested_cycles: 2,
            controller_period_ns: 10,
            p99_total_latency_ns: 20,
            latency_profile_id: "p99-profile/test-target-a".to_owned(),
            execution_mode: ActionExecutionMode::QueuedOpenLoop,
        };

        let result = select_action_horizon(&request).unwrap();
        assert_eq!(result.disposition, HorizonDisposition::UnqualifiedEvidence);
        assert_eq!(result.safe_horizon_ns, None);
        assert_eq!(result.k_max_safe_cycles, None);
        assert_eq!(result.selected_cycles, None);
        assert_eq!(result.bounds[6].unknown_reason.as_deref(), Some("motor thermal state has no qualified estimate"));
    }

    #[test]
    fn every_required_constraint_is_mandatory() {
        for index in 0..ACTION_HORIZON_CONSTRAINT_COUNT {
            let mut bounds = qualified_bounds([u64::MAX; ACTION_HORIZON_CONSTRAINT_COUNT]);
            bounds[index] = HorizonEvidence::unknown(HorizonConstraint::ALL[index], "not supplied").unwrap();
            let result = select_action_horizon(&ActionHorizonRequest {
                bounds,
                evaluation_time_ns: 100,
                requested_cycles: 1,
                controller_period_ns: 1,
                p99_total_latency_ns: 1,
                latency_profile_id: "p99-profile/test-target-a".to_owned(),
                execution_mode: ActionExecutionMode::QueuedOpenLoop,
            }).unwrap();
            assert_eq!(result.disposition, HorizonDisposition::UnqualifiedEvidence);
            assert_eq!(result.selected_cycles, None);
        }
    }

    #[test]
    fn direct_enum_construction_cannot_bypass_identity_validation() {
        let mut bounds = qualified_bounds([100; ACTION_HORIZON_CONSTRAINT_COUNT]);
        bounds[0] = HorizonEvidence::Qualified {
            constraint: HorizonConstraint::ObservationFreshness,
            remaining_ns: 100,
            source_id: " ".to_owned(),
            timebase_id: "application-monotonic-epoch-7".to_owned(),
        };
        let result = select_action_horizon(&ActionHorizonRequest {
            bounds,
            evaluation_time_ns: 100,
            requested_cycles: 2,
            controller_period_ns: 10,
            p99_total_latency_ns: 20,
            latency_profile_id: "p99-profile/test-target-a".to_owned(),
            execution_mode: ActionExecutionMode::QueuedOpenLoop,
        })
        .unwrap();

        assert_eq!(result.disposition, HorizonDisposition::InvalidEvidenceIdentity);
        assert_eq!(result.selected_cycles, None);
    }

    #[test]
    fn misordered_constraint_identity_rejects_composition() {
        let mut bounds = qualified_bounds([100; ACTION_HORIZON_CONSTRAINT_COUNT]);
        bounds[6] = HorizonEvidence::qualified(
            HorizonConstraint::ObservationFreshness,
            50,
            "wrongly-labelled-source",
            "application-monotonic-epoch-7",
        )
        .unwrap();
        let result = select_action_horizon(&ActionHorizonRequest {
            bounds,
            evaluation_time_ns: 100,
            requested_cycles: 2,
            controller_period_ns: 10,
            p99_total_latency_ns: 20,
            latency_profile_id: "p99-profile/test-target-a".to_owned(),
            execution_mode: ActionExecutionMode::QueuedOpenLoop,
        })
        .unwrap();

        assert_eq!(result.disposition, HorizonDisposition::ConstraintOrderMismatch);
        assert_eq!(result.safe_horizon_ns, None);
        assert_eq!(result.selected_cycles, None);
        assert_eq!(
            result.bounds[6].reported_constraint,
            HorizonConstraint::ObservationFreshness
        );
    }

    #[test]
    fn mismatched_timebases_reject_composition() {
        let mut bounds = qualified_bounds([100; ACTION_HORIZON_CONSTRAINT_COUNT]);
        bounds[3] = HorizonEvidence::qualified(
            HorizonConstraint::StateCertificate,
            50,
            "certificate-42",
            "another-monotonic-epoch",
        ).unwrap();
        let result = select_action_horizon(&ActionHorizonRequest {
            bounds,
            evaluation_time_ns: 100,
            requested_cycles: 2,
            controller_period_ns: 10,
            p99_total_latency_ns: 20,
            execution_mode: ActionExecutionMode::QueuedOpenLoop,
        }).unwrap();

        assert_eq!(result.disposition, HorizonDisposition::IncompatibleTimebases);
        assert_eq!(result.safe_horizon_ns, None);
        assert_eq!(result.selected_cycles, None);
    }

    #[test]
    fn queue_coverage_infeasible_does_not_extend_safe_horizon() {
        let result = select_action_horizon(&request(
            [100, 90, 80, 70, 60, 50, 40, 50],
            ActionExecutionMode::QueuedOpenLoop,
            10,
            51,
        ))
        .unwrap();

        assert_eq!(result.safe_horizon_ns, Some(40));
        assert_eq!(result.k_max_safe_cycles, Some(4));
        assert_eq!(result.k_min_queue_cycles, Some(6));
        assert_eq!(result.disposition, HorizonDisposition::QueueCoverageInfeasible);
        assert_eq!(result.selected_cycles, None);
    }

    #[test]
    fn split_timescale_local_expert_is_not_forced_to_cover_remote_latency() {
        let result = select_action_horizon(&request(
            [100, 90, 80, 70, 60, 50, 40, 50],
            ActionExecutionMode::SplitTimescaleLocalExpert,
            10,
            1_000_000,
        ))
        .unwrap();

        assert_eq!(result.k_min_queue_cycles, None);
        assert_eq!(result.k_max_safe_cycles, Some(4));
        assert_eq!(result.selected_cycles, Some(4));
        assert_eq!(result.disposition, HorizonDisposition::AdmissibleSplitTimescale);
    }

    #[test]
    fn requested_prefix_is_preserved_when_inside_safe_interval() {
        let mut request = request(
            [300, 500, 500, 400, 350, 320, 450, 600],
            ActionExecutionMode::QueuedOpenLoop,
            50,
            200,
        );
        request.requested_cycles = 5;
        let result = select_action_horizon(&request).unwrap();
        assert_eq!(result.k_min_queue_cycles, Some(4));
        assert_eq!(result.k_max_safe_cycles, Some(6));
        assert_eq!(result.selected_cycles, Some(5));
        assert_eq!(result.active_constraints_iter().next(), Some(HorizonConstraint::ObservationFreshness));
        assert_eq!(result.disposition, HorizonDisposition::AdmissibleQueuedOpenLoop);
    }

    #[test]
    fn requested_prefix_above_safe_maximum_is_rejected_not_clamped() {
        let mut request = request(
            [300, 100, 500, 400, 350, 320, 450, 600],
            ActionExecutionMode::QueuedOpenLoop,
            25,
            50,
        );
        request.requested_cycles = 5;
        let result = select_action_horizon(&request).unwrap();
        assert_eq!(result.k_max_safe_cycles, Some(4));
        assert_eq!(result.selected_cycles, None);
        assert_eq!(result.disposition, HorizonDisposition::RequestedHorizonExceedsBound);
    }

    #[test]
    fn requested_queued_prefix_below_latency_coverage_is_rejected() {
        let mut request = request(
            [300, 500, 500, 400, 350, 320, 450, 600],
            ActionExecutionMode::QueuedOpenLoop,
            50,
            200,
        );
        request.requested_cycles = 3;
        let result = select_action_horizon(&request).unwrap();
        assert_eq!(result.k_min_queue_cycles, Some(4));
        assert_eq!(result.k_max_safe_cycles, Some(6));
        assert_eq!(result.selected_cycles, None);
        assert_eq!(result.disposition, HorizonDisposition::RequestedHorizonBelowQueueCoverage);
    }

    #[test]
    fn queued_mode_rejects_when_latency_minimum_exceeds_safe_maximum() {
        let mut request = request(
            [300, 500, 500, 400, 350, 320, 150, 600],
            ActionExecutionMode::QueuedOpenLoop,
            50,
            250,
        );
        request.requested_cycles = 4;
        let result = select_action_horizon(&request).unwrap();
        assert_eq!(result.k_min_queue_cycles, Some(5));
        assert_eq!(result.k_max_safe_cycles, Some(3));
        assert_eq!(result.selected_cycles, None);
        assert_eq!(result.disposition, HorizonDisposition::QueueCoverageInfeasible);
    }

    #[test]
    fn exact_latency_multiple_has_no_off_by_one() {
        let result = select_action_horizon(&request(
            [100; ACTION_HORIZON_CONSTRAINT_COUNT],
            ActionExecutionMode::QueuedOpenLoop,
            10,
            30,
        ))
        .unwrap();

        assert_eq!(result.k_min_queue_cycles, Some(3));
        assert_eq!(result.disposition, HorizonDisposition::AdmissibleQueuedOpenLoop);
    }

    #[test]
    fn fractional_latency_cycle_rounds_up() {
        let result = select_action_horizon(&request(
            [100; ACTION_HORIZON_CONSTRAINT_COUNT],
            ActionExecutionMode::QueuedOpenLoop,
            10,
            31,
        ))
        .unwrap();

        assert_eq!(result.k_min_queue_cycles, Some(4));
    }

    #[test]
    fn zero_safe_horizon_rejects_a_complete_control_cycle() {
        let result = select_action_horizon(&request(
            [100, 100, 100, 100, 100, 100, 100, 0],
            ActionExecutionMode::SplitTimescaleLocalExpert,
            10,
            1,
        ))
        .unwrap();

        assert_eq!(result.safe_horizon_ns, Some(0));
        assert_eq!(result.k_max_safe_cycles, Some(0));
        assert_eq!(result.disposition, HorizonDisposition::NoCompleteActionCycle);
        assert_eq!(result.selected_cycles, None);
    }

    #[test]
    fn zero_controller_period_is_rejected() {
        let result = select_action_horizon(&request(
            [100; ACTION_HORIZON_CONSTRAINT_COUNT],
            ActionExecutionMode::QueuedOpenLoop,
            0,
            10,
        ));
        assert_eq!(result, Err(HorizonInputError::ZeroControllerPeriod));
    }

    #[test]
    fn empty_latency_profile_identity_is_rejected() {
        let mut request = request(
            [100; ACTION_HORIZON_CONSTRAINT_COUNT],
            ActionExecutionMode::QueuedOpenLoop,
            10,
            20,
        );
        request.latency_profile_id = " ".to_owned();
        assert_eq!(
            select_action_horizon(&request),
            Err(HorizonInputError::EmptyLatencyProfileIdentity)
        );
    }

    #[test]
    fn empty_evidence_identities_are_rejected() {
        assert_eq!(
            HorizonEvidence::qualified(HorizonConstraint::ObservationFreshness, 10, " ", "timebase"),
            Err(HorizonInputError::EmptySourceIdentity)
        );
        assert_eq!(
            HorizonEvidence::qualified(HorizonConstraint::ObservationFreshness, 10, "source", " "),
            Err(HorizonInputError::EmptyTimebaseIdentity)
        );
        assert_eq!(
            HorizonEvidence::unknown(HorizonConstraint::ObservationFreshness, " "),
            Err(HorizonInputError::EmptyUnknownReason)
        );
    }

    #[test]
    fn receipt_order_is_stable_and_follows_the_contract() {
        let result = select_action_horizon(&request(
            [8, 7, 6, 5, 4, 3, 2, 1],
            ActionExecutionMode::SplitTimescaleLocalExpert,
            1,
            0,
        ))
        .unwrap();
        let receipt_constraints: Vec<_> = result.bounds.iter().map(|r| r.constraint).collect();
        assert_eq!(receipt_constraints, HorizonConstraint::ALL);
        assert_eq!(
            result.active_constraints_iter().collect::<Vec<_>>(),
            vec![HorizonConstraint::RecoveryReserve]
        );
    }
}
\n#[cfg(test)]
mod action_commitment_tests {
    use super::*;

    fn valid_selection() -> ActionHorizonSelection<'static> {
        let bounds = core::array::from_fn(|index| HorizonBoundReceipt {
            constraint: HorizonConstraint::ALL[index],
            reported_constraint: HorizonConstraint::ALL[index],
            remaining_ns: Some(100),
            source_id: Some("qualified-source"),
            timebase_id: Some("application-epoch-1"),
            unknown_reason: None,
        });
        ActionHorizonSelection {
            disposition: HorizonDisposition::AdmissibleQueuedOpenLoop,
            evaluated_at_ns: 1_000,
            bounds,
            common_timebase_id: Some("application-epoch-1"),
            safe_horizon_ns: Some(95),
            active_constraints: [Some(HorizonConstraint::RecoveryReserve), None, None, None, None, None, None, None],
            active_constraint_count: 1,
            controller_period_ns: 10,
            p99_total_latency_ns: 20,
            latency_profile_id: "latency-profile-v1",
            k_min_queue_cycles: Some(2),
            k_max_safe_cycles: Some(9),
            selected_cycles: Some(9),
        }
    }

    fn commitment() -> ActionCommitment<'static> {
        create_action_commitment(
            valid_selection(),
            [0x11; 32],
            [0x22; 32],
            7,
            1_000,
        )
        .unwrap()
    }

    #[test]
    fn commitment_binds_proposal_configuration_timebase_and_expiry() {
        let receipt = commitment();
        assert_eq!(receipt.selected_cycles(), 9);
        assert_eq!(receipt.expires_at_ns(), 1_090);
        assert_eq!(
            receipt.validate(1_089, "application-epoch-1", &[0x11; 32], &[0x22; 32], 6),
            Ok(())
        );
    }

    #[test]
    fn commitment_expires_at_exclusive_boundary() {
        let receipt = commitment();
        assert_eq!(
            receipt.validate(1_090, "application-epoch-1", &[0x11; 32], &[0x22; 32], 6),
            Err(ActionCommitmentInvalidity::Expired)
        );
    }

    #[test]
    fn commitment_rejects_precommit_time() {
        let receipt = commitment();
        assert_eq!(
            receipt.validate(999, "application-epoch-1", &[0x11; 32], &[0x22; 32], 6),
            Err(ActionCommitmentInvalidity::NotYetValid)
        );
    }

    #[test]
    fn commitment_rejects_wrong_timebase() {
        let receipt = commitment();
        assert_eq!(
            receipt.validate(1_010, "other-epoch", &[0x11; 32], &[0x22; 32], 6),
            Err(ActionCommitmentInvalidity::TimebaseMismatch)
        );
    }

    #[test]
    fn commitment_rejects_changed_proposal_digest() {
        let receipt = commitment();
        assert_eq!(
            receipt.validate(1_010, "application-epoch-1", &[0x33; 32], &[0x22; 32], 6),
            Err(ActionCommitmentInvalidity::ProposalDigestMismatch)
        );
    }

    #[test]
    fn commitment_rejects_changed_robot_configuration_digest() {
        let receipt = commitment();
        assert_eq!(
            receipt.validate(1_010, "application-epoch-1", &[0x11; 32], &[0x44; 32], 6),
            Err(ActionCommitmentInvalidity::ConfigurationDigestMismatch)
        );
    }

    #[test]
    fn commitment_rejects_reused_or_out_of_order_sequence() {
        let receipt = commitment();
        assert_eq!(
            receipt.validate(1_010, "application-epoch-1", &[0x11; 32], &[0x22; 32], 7),
            Err(ActionCommitmentInvalidity::SequenceReplay)
        );
    }

    #[test]
    fn zero_sequence_number_cannot_be_committed() {
        assert_eq!(
            create_action_commitment(valid_selection(), [0; 32], [0; 32], 0, 0),
            Err(ActionCommitmentError::ZeroSequenceNumber)
        );
    }

    #[test]
    fn commitment_cannot_be_created_from_unknown_or_infeasible_selection() {
        let mut selection = valid_selection();
        selection.disposition = HorizonDisposition::QueueCoverageInfeasible;
        assert_eq!(
            create_action_commitment(selection, [0; 32], [0; 32], 1, 0),
            Err(ActionCommitmentError::SelectionNotAdmissible)
        );
    }

    #[test]
    fn commitment_cannot_reuse_a_selection_after_evidence_evaluation() {
        assert_eq!(
            create_action_commitment(
                valid_selection(),
                [0; 32],
                [0; 32],
                1,
                1_001,
            ),
            Err(ActionCommitmentError::EvaluationTimeMismatch)
        );
    }

    #[test]
    fn commitment_rejects_expiry_overflow() {
        let mut selection = valid_selection();
        selection.evaluated_at_ns = u64::MAX - 50;
        assert_eq!(
            create_action_commitment(selection, [0; 32], [0; 32], 1, u64::MAX - 50),
            Err(ActionCommitmentError::ExpiryOverflow)
        );
    }

    #[test]
    fn commitment_rejects_duration_over_safe_horizon() {
        let mut selection = valid_selection();
        selection.selected_cycles = Some(10);
        assert_eq!(
            create_action_commitment(selection, [0; 32], [0; 32], 1, 0),
            Err(ActionCommitmentError::ExceedsSafeHorizon)
        );
    }
}

