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
}

impl core::fmt::Display for HorizonInputError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ZeroControllerPeriod => write!(f, "controller period must be non-zero"),
            Self::EmptySourceIdentity => write!(f, "qualified horizon source identity is empty"),
            Self::EmptyTimebaseIdentity => write!(f, "qualified horizon timebase identity is empty"),
            Self::EmptyUnknownReason => write!(f, "unknown horizon reason is empty"),
            Self::EmptyLatencyProfileIdentity => write!(f, "p99 latency profile identity is empty"),
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
    /// The safe duration does not contain one complete control cycle.
    NoCompleteActionCycle,
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
    pub disposition: HorizonDisposition,
    pub bounds: [HorizonBoundReceipt<'a>; ACTION_HORIZON_CONSTRAINT_COUNT],
    pub common_timebase_id: Option<&'a str>,
    pub safe_horizon_ns: Option<u64>,
    /// Active-minimum constraints in canonical order. Slots after count are None.
    pub active_constraints: [Option<HorizonConstraint>; ACTION_HORIZON_CONSTRAINT_COUNT],
    pub active_constraint_count: usize,
    pub controller_period_ns: u64,
    pub p99_total_latency_ns: u64,
    pub latency_profile_id: &'a str,
    /// ceil(p99 / controller period) for queued mode; None for split-timescale.
    pub k_min_queue_cycles: Option<u64>,
    /// floor(safe horizon / controller period) when all bounds qualify.
    pub k_max_safe_cycles: Option<u64>,
    /// Longest admissible whole-cycle prefix, when the mode is feasible.
    pub selected_cycles: Option<u64>,
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

    let mut unknown_count = 0usize;
    let mut common_timebase: Option<&'a str> = None;
    let mut timebase_mismatch = false;
    let mut constraint_mismatch = false;
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
            selection.disposition = HorizonDisposition::AdmissibleQueuedOpenLoop;
            selection.selected_cycles = Some(k_max_safe_cycles);
        }
        ActionExecutionMode::SplitTimescaleLocalExpert => {
            selection.disposition = HorizonDisposition::AdmissibleSplitTimescale;
            selection.selected_cycles = Some(k_max_safe_cycles);
        }
    }

    Ok(selection)
}

impl ActionHorizonSelection<'_> {
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
