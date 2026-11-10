//! Deterministic mission-reserve state carryover for sequential locomotion faults.
//!
//! This reference replays caller-supplied, identity-bound event evidence. It
//! does not estimate physical state, generate robot dynamics, compute
//! viability, authenticate receipts, or certify mission safety.

use std::collections::BTreeSet;

pub const MAX_EVENTS: usize = 1_024;
pub const MAX_AUTHORITY_PPM: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceDisposition {
    Qualified,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityRequalification {
    pub receipt_ref: String,
    pub context_digest: String,
    pub valid_through_tick: u64,
    /// Independently measured post-requalification authority, not a requested
    /// reset or controller-provided nominal value.
    pub measured_authority_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TopologyUpdate {
    /// The upstream topology authority explicitly confirms that this event
    /// does not change the current independent recovery-route count.
    Unchanged {
        receipt_ref: String,
        valid_through_tick: u64,
    },
    /// A fresh upstream topology receipt reports the post-event count.
    Recomputed {
        receipt_ref: String,
        context_digest: String,
        valid_through_tick: u64,
        independent_routes: u32,
    },
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionState {
    pub context_digest: String,
    pub current_tick: u64,
    pub soc_mwh: u64,
    /// Reserved diagnostic/recovery budget; it is separate from battery SOC.
    pub recovery_budget_mwh: u64,
    /// Remaining qualified thermal headroom, not temperature itself.
    pub thermal_headroom_mj: u64,
    pub thermal_capacity_mj: u64,
    /// Measured/qualified actuator authority on a 0..=1,000,000 ppm scale.
    pub actuator_authority_ppm: u32,
    pub terminal_safe_stop_required_energy_mwh: u64,
    pub independent_recovery_routes: u32,
    pub minimum_independent_recovery_routes: u32,
    pub topology_receipt_ref: String,
    pub topology_valid_through_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionEvent {
    pub event_id: String,
    pub context_digest: String,
    pub event_tick: u64,
    pub valid_through_tick: u64,
    pub evidence: EvidenceDisposition,
    /// Energy drawn from the battery by this event/action.
    pub battery_energy_cost_mwh: u64,
    /// Separate debit against the reserved diagnostic/recovery budget.
    pub recovery_budget_cost_mwh: u64,
    /// Reduction in qualified thermal headroom.
    pub thermal_headroom_loss_mj: u64,
    /// Qualified cooling gained during this event/interval.
    pub thermal_cooling_gain_mj: u64,
    /// Authority needed to perform the event's action, checked before effects.
    pub minimum_authority_required_ppm: u32,
    /// Fault-induced authority loss, in absolute ppm points.
    pub actuator_authority_loss_ppm: u32,
    /// Optional independently measured authority after repair/requalification.
    pub measured_authority_after_event: Option<AuthorityRequalification>,
    /// Every event must explicitly preserve or refresh upstream topology.
    pub topology_update: TopologyUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissionStatus {
    Complete,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    InitialTopologyEvidenceStale,
    InitialTerminalReserveInsufficient,
    InitialTopologyReserveInsufficient,
    EventEvidenceUnknown,
    EventEvidenceStale,
    InsufficientActuatorAuthority,
    InsufficientBatteryEnergy,
    InsufficientRecoveryBudget,
    ThermalHeadroomExhausted,
    TerminalSafeStopReserveInsufficient,
    TopologyReserveInsufficient,
    RequalificationEvidenceUnknown,
    TopologyEvidenceUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionSnapshot {
    pub event_id: String,
    pub tick: u64,
    pub state: MissionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionReport {
    pub status: MissionStatus,
    pub stop_reason: Option<StopReason>,
    pub stopped_event_id: Option<String>,
    /// State after the last fully or physically applied event. An Unknown
    /// evidence stop never invents an update for the unresolved evidence.
    pub final_state: MissionState,
    pub snapshots: Vec<MissionSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MissionError {
    EmptyContextDigest,
    EmptyInitialTopologyReceipt,
    InvalidInitialThermalEnvelope,
    InvalidInitialAuthority,
    TooManyEvents { actual: usize, limit: usize },
    EmptyEventId,
    DuplicateEventId(String),
    EventContextMismatch(String),
    NonMonotonicEventTick { event_id: String, previous: u64, next: u64 },
    InvalidEventAuthority { event_id: String },
    UnexpectedRequalificationReceipt(String),
    InvalidRequalificationAuthority(String),
    ThermalCapacityExceeded(String),
    InvalidTopologyReceipt(String),
}

fn stopped(
    status: MissionStatus,
    reason: StopReason,
    event_id: Option<String>,
    state: &MissionState,
    snapshots: Vec<MissionSnapshot>,
) -> MissionReport {
    MissionReport {
        status,
        stop_reason: Some(reason),
        stopped_event_id: event_id,
        final_state: state.clone(),
        snapshots,
    }
}

fn validate_initial(state: &MissionState) -> Result<(), MissionError> {
    if state.context_digest.trim().is_empty() {
        return Err(MissionError::EmptyContextDigest);
    }
    if state.topology_receipt_ref.trim().is_empty() {
        return Err(MissionError::EmptyInitialTopologyReceipt);
    }
    if state.thermal_headroom_mj > state.thermal_capacity_mj {
        return Err(MissionError::InvalidInitialThermalEnvelope);
    }
    if state.actuator_authority_ppm > MAX_AUTHORITY_PPM {
        return Err(MissionError::InvalidInitialAuthority);
    }
    Ok(())
}

/// Replay an ordered mission sequence while carrying physical and assurance
/// reserve forward. The event order is semantically significant and is never
/// sorted. Each event is atomic for bookkeeping: a rejected action is not
/// debited, while a completed action can still leave insufficient terminal
/// safe-stop reserve.
pub fn replay(
    initial: &MissionState,
    events: &[MissionEvent],
) -> Result<MissionReport, MissionError> {
    validate_initial(initial)?;
    if events.len() > MAX_EVENTS {
        return Err(MissionError::TooManyEvents {
            actual: events.len(),
            limit: MAX_EVENTS,
        });
    }

    let mut state = initial.clone();
    let mut snapshots = Vec::new();
    let mut seen_event_ids = BTreeSet::new();

    if state.topology_valid_through_tick < state.current_tick {
        return Ok(stopped(
            MissionStatus::Unknown,
            StopReason::InitialTopologyEvidenceStale,
            None,
            &state,
            snapshots,
        ));
    }
    if state.soc_mwh < state.terminal_safe_stop_required_energy_mwh {
        return Ok(stopped(
            MissionStatus::Rejected,
            StopReason::InitialTerminalReserveInsufficient,
            None,
            &state,
            snapshots,
        ));
    }
    if state.independent_recovery_routes < state.minimum_independent_recovery_routes {
        return Ok(stopped(
            MissionStatus::Rejected,
            StopReason::InitialTopologyReserveInsufficient,
            None,
            &state,
            snapshots,
        ));
    }

    for event in events {
        if event.event_id.trim().is_empty() {
            return Err(MissionError::EmptyEventId);
        }
        if !seen_event_ids.insert(event.event_id.clone()) {
            return Err(MissionError::DuplicateEventId(event.event_id.clone()));
        }
        if event.context_digest != state.context_digest {
            return Err(MissionError::EventContextMismatch(event.event_id.clone()));
        }
        if event.event_tick <= state.current_tick {
            return Err(MissionError::NonMonotonicEventTick {
                event_id: event.event_id.clone(),
                previous: state.current_tick,
                next: event.event_tick,
            });
        }
        if event.minimum_authority_required_ppm > MAX_AUTHORITY_PPM
            || event.actuator_authority_loss_ppm > MAX_AUTHORITY_PPM
        {
            return Err(MissionError::InvalidEventAuthority {
                event_id: event.event_id.clone(),
            });
        }

        if event.evidence == EvidenceDisposition::Unknown {
            return Ok(stopped(
                MissionStatus::Unknown,
                StopReason::EventEvidenceUnknown,
                Some(event.event_id.clone()),
                &state,
                snapshots,
            ));
        }
        if event.valid_through_tick < event.event_tick {
            return Ok(stopped(
                MissionStatus::Unknown,
                StopReason::EventEvidenceStale,
                Some(event.event_id.clone()),
                &state,
                snapshots,
            ));
        }
        if state.actuator_authority_ppm < event.minimum_authority_required_ppm {
            return Ok(stopped(
                MissionStatus::Rejected,
                StopReason::InsufficientActuatorAuthority,
                Some(event.event_id.clone()),
                &state,
                snapshots,
            ));
        }
        if event.battery_energy_cost_mwh > state.soc_mwh {
            return Ok(stopped(
                MissionStatus::Rejected,
                StopReason::InsufficientBatteryEnergy,
                Some(event.event_id.clone()),
                &state,
                snapshots,
            ));
        }
        if event.recovery_budget_cost_mwh > state.recovery_budget_mwh {
            return Ok(stopped(
                MissionStatus::Rejected,
                StopReason::InsufficientRecoveryBudget,
                Some(event.event_id.clone()),
                &state,
                snapshots,
            ));
        }
        if event.thermal_headroom_loss_mj > state.thermal_headroom_mj {
            return Ok(stopped(
                MissionStatus::Rejected,
                StopReason::ThermalHeadroomExhausted,
                Some(event.event_id.clone()),
                &state,
                snapshots,
            ));
        }

        // Build a candidate state and commit it only after all exact arithmetic
        // and receipt checks for this event have passed.
        let mut next = state.clone();
        next.soc_mwh -= event.battery_energy_cost_mwh;
        next.recovery_budget_mwh -= event.recovery_budget_cost_mwh;
        next.thermal_headroom_mj -= event.thermal_headroom_loss_mj;
        next.thermal_headroom_mj = next
            .thermal_headroom_mj
            .checked_add(event.thermal_cooling_gain_mj)
            .ok_or_else(|| MissionError::ThermalCapacityExceeded(event.event_id.clone()))?;
        if next.thermal_headroom_mj > next.thermal_capacity_mj {
            return Err(MissionError::ThermalCapacityExceeded(
                event.event_id.clone(),
            ));
        }
        next.actuator_authority_ppm = next
            .actuator_authority_ppm
            .saturating_sub(event.actuator_authority_loss_ppm);
        next.current_tick = event.event_tick;

        if let Some(receipt) = &event.measured_authority_after_event {
            if receipt.receipt_ref.trim().is_empty()
                || receipt.context_digest != state.context_digest
                || receipt.valid_through_tick < event.event_tick
            {
                return Ok(stopped(
                    MissionStatus::Unknown,
                    StopReason::RequalificationEvidenceUnknown,
                    Some(event.event_id.clone()),
                    &next,
                    snapshots,
                ));
            }
            if receipt.measured_authority_ppm > MAX_AUTHORITY_PPM {
                return Err(MissionError::InvalidRequalificationAuthority(
                    event.event_id.clone(),
                ));
            }
            next.actuator_authority_ppm = receipt.measured_authority_ppm;
        }

        match &event.topology_update {
            TopologyUpdate::Unchanged {
                receipt_ref,
                valid_through_tick,
            } => {
                if receipt_ref.trim().is_empty() || *valid_through_tick < event.event_tick {
                    return Ok(stopped(
                        MissionStatus::Unknown,
                        StopReason::TopologyEvidenceUnknown,
                        Some(event.event_id.clone()),
                        &next,
                        snapshots,
                    ));
                }
                next.topology_receipt_ref = receipt_ref.clone();
                next.topology_valid_through_tick = *valid_through_tick;
            }
            TopologyUpdate::Recomputed {
                receipt_ref,
                context_digest,
                valid_through_tick,
                independent_routes,
            } => {
                if receipt_ref.trim().is_empty()
                    || context_digest != &state.context_digest
                    || *valid_through_tick < event.event_tick
                {
                    return Ok(stopped(
                        MissionStatus::Unknown,
                        StopReason::TopologyEvidenceUnknown,
                        Some(event.event_id.clone()),
                        &next,
                        snapshots,
                    ));
                }
                next.topology_receipt_ref = receipt_ref.clone();
                next.topology_valid_through_tick = *valid_through_tick;
                next.independent_recovery_routes = *independent_routes;
            }
            TopologyUpdate::Unknown => {
                return Ok(stopped(
                    MissionStatus::Unknown,
                    StopReason::TopologyEvidenceUnknown,
                    Some(event.event_id.clone()),
                    &next,
                    snapshots,
                ));
            }
        }

        if next.soc_mwh < next.terminal_safe_stop_required_energy_mwh {
            return Ok(stopped(
                MissionStatus::Rejected,
                StopReason::TerminalSafeStopReserveInsufficient,
                Some(event.event_id.clone()),
                &next,
                snapshots,
            ));
        }
        if next.independent_recovery_routes < next.minimum_independent_recovery_routes {
            return Ok(stopped(
                MissionStatus::Rejected,
                StopReason::TopologyReserveInsufficient,
                Some(event.event_id.clone()),
                &next,
                snapshots,
            ));
        }

        snapshots.push(MissionSnapshot {
            event_id: event.event_id.clone(),
            tick: event.event_tick,
            state: next.clone(),
        });
        state = next;
    }

    Ok(MissionReport {
        status: MissionStatus::Complete,
        stop_reason: None,
        stopped_event_id: None,
        final_state: state,
        snapshots,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initial() -> MissionState {
        MissionState {
            context_digest: "mission-context-1".to_owned(),
            current_tick: 0,
            soc_mwh: 1_000,
            recovery_budget_mwh: 300,
            thermal_headroom_mj: 500,
            thermal_capacity_mj: 500,
            actuator_authority_ppm: MAX_AUTHORITY_PPM,
            terminal_safe_stop_required_energy_mwh: 100,
            independent_recovery_routes: 2,
            minimum_independent_recovery_routes: 1,
            topology_receipt_ref: "topology:0".to_owned(),
            topology_valid_through_tick: 1_000,
        }
    }

    fn event(id: &str, tick: u64) -> MissionEvent {
        MissionEvent {
            event_id: id.to_owned(),
            context_digest: "mission-context-1".to_owned(),
            event_tick: tick,
            valid_through_tick: 1_000,
            evidence: EvidenceDisposition::Qualified,
            battery_energy_cost_mwh: 0,
            recovery_budget_cost_mwh: 0,
            thermal_headroom_loss_mj: 0,
            thermal_cooling_gain_mj: 0,
            minimum_authority_required_ppm: 0,
            actuator_authority_loss_ppm: 0,
            measured_authority_after_event: None,
            topology_update: TopologyUpdate::Unchanged {
                receipt_ref: "topology:unchanged".to_owned(),
                valid_through_tick: 1_000,
            },
        }
    }

    #[test]
    fn repeated_recoveries_cannot_reset_reserved_budget() {
        let mut first = event("derate", 1);
        first.recovery_budget_cost_mwh = 120;
        first.actuator_authority_loss_ppm = 300_000;
        first.minimum_authority_required_ppm = 600_000;

        let mut second = event("slip-recovery", 2);
        second.recovery_budget_cost_mwh = 210;
        second.minimum_authority_required_ppm = 600_000;

        let report = replay(&initial(), &[first, second]).unwrap();
        assert_eq!(report.status, MissionStatus::Rejected);
        assert_eq!(
            report.stop_reason,
            Some(StopReason::InsufficientRecoveryBudget)
        );
        assert_eq!(report.final_state.recovery_budget_mwh, 180);
        assert_eq!(report.final_state.actuator_authority_ppm, 700_000);
        assert_eq!(report.snapshots.len(), 1);
    }

    #[test]
    fn event_order_changes_authority_admissibility() {
        let mut state = initial();
        state.soc_mwh = 1_500;
        state.recovery_budget_mwh = 500;
        state.thermal_headroom_mj = 600;
        state.thermal_capacity_mj = 600;

        let mut high_load = event("high-load-turn", 1);
        high_load.battery_energy_cost_mwh = 100;
        high_load.thermal_headroom_loss_mj = 350;
        high_load.minimum_authority_required_ppm = 900_000;

        let mut derate = event("torque-derate", 2);
        derate.battery_energy_cost_mwh = 100;
        derate.thermal_headroom_loss_mj = 180;
        derate.actuator_authority_loss_ppm = 250_000;
        derate.minimum_authority_required_ppm = 600_000;

        let forward = replay(&state, &[high_load.clone(), derate.clone()]).unwrap();
        assert_eq!(forward.status, MissionStatus::Complete);
        assert_eq!(forward.final_state.actuator_authority_ppm, 750_000);
        assert_eq!(forward.final_state.thermal_headroom_mj, 70);

        let reversed = replay(&state, &[derate, high_load]).unwrap();
        assert_eq!(reversed.status, MissionStatus::Rejected);
        assert_eq!(
            reversed.stop_reason,
            Some(StopReason::InsufficientActuatorAuthority)
        );
        assert_eq!(reversed.final_state.actuator_authority_ppm, 750_000);
    }

    #[test]
    fn unknown_second_event_preserves_last_qualified_state() {
        let mut first = event("qualified-first", 1);
        first.thermal_headroom_loss_mj = 100;
        let mut second = event("unknown-second", 2);
        second.evidence = EvidenceDisposition::Unknown;
        let report = replay(&initial(), &[first, second]).unwrap();
        assert_eq!(report.status, MissionStatus::Unknown);
        assert_eq!(report.stop_reason, Some(StopReason::EventEvidenceUnknown));
        assert_eq!(report.final_state.thermal_headroom_mj, 400);
        assert_eq!(report.final_state.current_tick, 1);
        assert_eq!(report.snapshots.len(), 1);
    }

    #[test]
    fn cooling_does_not_restore_actuator_authority() {
        let mut derate = event("derate", 1);
        derate.actuator_authority_loss_ppm = 300_000;
        let mut cooldown = event("cooldown", 2);
        cooldown.thermal_cooling_gain_mj = 100;
        let report = replay(&initial(), &[derate, cooldown]).unwrap();
        assert_eq!(report.status, MissionStatus::Complete);
        assert_eq!(report.final_state.thermal_headroom_mj, 500);
        assert_eq!(report.final_state.actuator_authority_ppm, 700_000);
    }

    #[test]
    fn actuator_authority_cannot_be_restored_without_a_qualified_receipt() {
        let mut repair = event("unqualified-repair", 1);
        repair.actuator_authority_loss_ppm = 0;
        repair.measured_authority_after_event = Some(AuthorityRequalification {
            receipt_ref: String::new(),
            context_digest: "mission-context-1".to_owned(),
            valid_through_tick: 1_000,
            measured_authority_ppm: MAX_AUTHORITY_PPM,
        });
        let report = replay(&initial(), &[repair]).unwrap();
        assert_eq!(report.status, MissionStatus::Unknown);
        assert_eq!(
            report.stop_reason,
            Some(StopReason::RequalificationEvidenceUnknown)
        );
        assert_eq!(report.final_state.actuator_authority_ppm, MAX_AUTHORITY_PPM);
    }

    #[test]
    fn fresh_requalification_can_restore_measured_authority() {
        let mut repair = event("qualified-repair", 1);
        repair.actuator_authority_loss_ppm = 300_000;
        repair.measured_authority_after_event = Some(AuthorityRequalification {
            receipt_ref: "actuator-requalification:1".to_owned(),
            context_digest: "mission-context-1".to_owned(),
            valid_through_tick: 1_000,
            measured_authority_ppm: 850_000,
        });
        let report = replay(&initial(), &[repair]).unwrap();
        assert_eq!(report.status, MissionStatus::Complete);
        assert_eq!(report.final_state.actuator_authority_ppm, 850_000);
    }

    #[test]
    fn stale_topology_receipt_makes_sequence_unknown() {
        let mut changed = event("fault", 1);
        changed.topology_update = TopologyUpdate::Recomputed {
            receipt_ref: "topology:stale".to_owned(),
            context_digest: "mission-context-1".to_owned(),
            valid_through_tick: 0,
            independent_routes: 2,
        };
        let report = replay(&initial(), &[changed]).unwrap();
        assert_eq!(report.status, MissionStatus::Unknown);
        assert_eq!(report.stop_reason, Some(StopReason::TopologyEvidenceUnknown));
        assert_eq!(report.final_state.current_tick, 1);
    }

    #[test]
    fn lost_topology_reserve_rejects_mission_continuation() {
        let mut state = initial();
        state.minimum_independent_recovery_routes = 2;
        let mut fault = event("fault", 1);
        fault.topology_update = TopologyUpdate::Recomputed {
            receipt_ref: "topology:after-fault".to_owned(),
            context_digest: "mission-context-1".to_owned(),
            valid_through_tick: 1_000,
            independent_routes: 1,
        };
        let report = replay(&state, &[fault]).unwrap();
        assert_eq!(report.status, MissionStatus::Rejected);
        assert_eq!(
            report.stop_reason,
            Some(StopReason::TopologyReserveInsufficient)
        );
        assert_eq!(report.final_state.independent_recovery_routes, 1);
    }

    #[test]
    fn task_completion_does_not_override_terminal_safe_stop_energy() {
        let mut state = initial();
        state.soc_mwh = 700;
        state.terminal_safe_stop_required_energy_mwh = 180;
        let mut completion = event("complete-task", 1);
        completion.battery_energy_cost_mwh = 550;
        let report = replay(&state, &[completion]).unwrap();
        assert_eq!(report.status, MissionStatus::Rejected);
        assert_eq!(
            report.stop_reason,
            Some(StopReason::TerminalSafeStopReserveInsufficient)
        );
        assert_eq!(report.final_state.soc_mwh, 150);
    }

    #[test]
    fn duplicate_event_identity_is_invalid() {
        let a = event("duplicate", 1);
        let mut b = event("duplicate", 2);
        b.event_tick = 2;
        let result = replay(&initial(), &[a, b]);
        assert_eq!(
            result,
            Err(MissionError::DuplicateEventId("duplicate".to_owned()))
        );
    }

    #[test]
    fn event_order_is_not_silently_sorted() {
        let later = event("later", 2);
        let earlier = event("earlier", 1);
        let result = replay(&initial(), &[later, earlier]);
        assert!(matches!(
            result,
            Err(MissionError::NonMonotonicEventTick { .. })
        ));
    }

    #[test]
    fn topology_unknown_is_not_assumed_unchanged() {
        let mut event = event("topology-unknown", 1);
        event.topology_update = TopologyUpdate::Unknown;
        let report = replay(&initial(), &[event]).unwrap();
        assert_eq!(report.status, MissionStatus::Unknown);
        assert_eq!(report.stop_reason, Some(StopReason::TopologyEvidenceUnknown));
    }

    #[test]
    fn event_context_mismatch_is_invalid() {
        let mut event = event("wrong-context", 1);
        event.context_digest = "other-context".to_owned();
        let result = replay(&initial(), &[event]);
        assert_eq!(
            result,
            Err(MissionError::EventContextMismatch(
                "wrong-context".to_owned()
            ))
        );
    }

    #[test]
    fn cooling_cannot_exceed_declared_thermal_capacity() {
        let mut event = event("overcool", 1);
        event.thermal_cooling_gain_mj = 1;
        let result = replay(&initial(), &[event]);
        assert_eq!(
            result,
            Err(MissionError::ThermalCapacityExceeded("overcool".to_owned()))
        );
    }
}
