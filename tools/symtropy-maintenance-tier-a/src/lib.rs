#![forbid(unsafe_code)]

//! Tier-A single-machine maintenance reference.
//!
//! This crate deliberately evaluates a supplied, absolute-time event schedule.
//! It does not generate failures, model latent degradation, observe sensors,
//! queue maintenance crews, or simulate a production line. Its role is to make
//! state transitions and time accounting independently hand-checkable.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineState {
    Operating,
    Repairing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MachineEventKind {
    Failure,
    RepairComplete,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MachineEvent {
    pub id: String,
    pub at_hours: f64,
    pub kind: MachineEventKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MachineSchedule {
    pub id: String,
    pub horizon_hours: f64,
    pub production_rate_per_hour: f64,
    /// Events are absolute calendar times and must be strictly increasing.
    ///
    /// The initial state is Operating. Events must alternate Failure and
    /// RepairComplete. An unfinished repair at the horizon is right-censored.
    pub events: Vec<MachineEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateInterval {
    pub start_hours: f64,
    pub end_hours: f64,
    pub state: MachineState,
}

impl StateInterval {
    pub fn duration_hours(&self) -> f64 {
        self.end_hours - self.start_hours
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventTransition {
    pub event_id: String,
    pub at_hours: f64,
    pub kind: MachineEventKind,
    pub state_before: MachineState,
    pub state_after: MachineState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MachineAssessment {
    pub schedule_id: String,
    pub horizon_hours: f64,
    pub production_rate_per_hour: f64,
    pub productive_time_hours: f64,
    pub repair_downtime_hours: f64,
    pub availability: f64,
    pub good_output_units: f64,
    pub failure_count: usize,
    pub completed_repair_count: usize,
    pub final_state: MachineState,
    pub repair_right_censored: bool,
    /// Elapsed time in the current repair at the horizon, when right-censored.
    /// Remaining repair duration is unknown unless separately supplied by a
    /// different model; this Tier-A trace does not invent it.
    pub current_repair_elapsed_hours: Option<f64>,
    pub intervals: Vec<StateInterval>,
    pub transitions: Vec<EventTransition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationError {
    EmptyScheduleId,
    InvalidHorizon,
    InvalidProductionRate,
    EmptyEventId,
    DuplicateEventId,
    NonFiniteEventTime,
    EventOutsideHorizon,
    EventTimeOutOfOrder,
    DuplicateEventTime,
    FailureWhileRepairing,
    RepairCompleteWhileOperating,
    NonFiniteTimeAccounting,
    NonFiniteAvailability,
    NonFiniteProduction,
}

pub fn assess(schedule: &MachineSchedule) -> Result<MachineAssessment, ValidationError> {
    validate(schedule)?;

    let mut state = MachineState::Operating;
    let mut cursor = 0.0_f64;
    let mut productive_time = 0.0_f64;
    let mut repair_downtime = 0.0_f64;
    let mut last_failure_at = None;
    let mut failure_count = 0_usize;
    let mut repair_count = 0_usize;
    let mut intervals = Vec::with_capacity(schedule.events.len() + 1);
    let mut transitions = Vec::with_capacity(schedule.events.len());

    for event in &schedule.events {
        if event.at_hours > cursor {
            let interval = StateInterval {
                start_hours: cursor,
                end_hours: event.at_hours,
                state,
            };
            match state {
                MachineState::Operating => productive_time += interval.duration_hours(),
                MachineState::Repairing => repair_downtime += interval.duration_hours(),
            }
            if !productive_time.is_finite() || !repair_downtime.is_finite() {
                return Err(ValidationError::NonFiniteTimeAccounting);
            }
            intervals.push(interval);
        }

        let before = state;
        match event.kind {
            MachineEventKind::Failure => {
                if state != MachineState::Operating {
                    return Err(ValidationError::FailureWhileRepairing);
                }
                state = MachineState::Repairing;
                failure_count += 1;
                last_failure_at = Some(event.at_hours);
            }
            MachineEventKind::RepairComplete => {
                if state != MachineState::Repairing {
                    return Err(ValidationError::RepairCompleteWhileOperating);
                }
                state = MachineState::Operating;
                repair_count += 1;
                last_failure_at = None;
            }
        }

        transitions.push(EventTransition {
            event_id: event.id.clone(),
            at_hours: event.at_hours,
            kind: event.kind,
            state_before: before,
            state_after: state,
        });
        cursor = event.at_hours;
    }

    if schedule.horizon_hours > cursor {
        let interval = StateInterval {
            start_hours: cursor,
            end_hours: schedule.horizon_hours,
            state,
        };
        match state {
            MachineState::Operating => productive_time += interval.duration_hours(),
            MachineState::Repairing => repair_downtime += interval.duration_hours(),
        }
        if !productive_time.is_finite() || !repair_downtime.is_finite() {
            return Err(ValidationError::NonFiniteTimeAccounting);
        }
        intervals.push(interval);
    }

    let availability = productive_time / schedule.horizon_hours;
    if !availability.is_finite() {
        return Err(ValidationError::NonFiniteAvailability);
    }

    let good_output_units = productive_time * schedule.production_rate_per_hour;
    if !good_output_units.is_finite() {
        return Err(ValidationError::NonFiniteProduction);
    }

    let repair_right_censored = state == MachineState::Repairing;
    let current_repair_elapsed_hours = if repair_right_censored {
        last_failure_at.map(|failure_at| schedule.horizon_hours - failure_at)
    } else {
        None
    };

    Ok(MachineAssessment {
        schedule_id: schedule.id.clone(),
        horizon_hours: schedule.horizon_hours,
        production_rate_per_hour: schedule.production_rate_per_hour,
        productive_time_hours: productive_time,
        repair_downtime_hours: repair_downtime,
        availability,
        good_output_units,
        failure_count,
        completed_repair_count: repair_count,
        final_state: state,
        repair_right_censored,
        current_repair_elapsed_hours,
        intervals,
        transitions,
    })
}

pub fn validate(schedule: &MachineSchedule) -> Result<(), ValidationError> {
    if schedule.id.trim().is_empty() {
        return Err(ValidationError::EmptyScheduleId);
    }
    if !schedule.horizon_hours.is_finite() || schedule.horizon_hours <= 0.0 {
        return Err(ValidationError::InvalidHorizon);
    }
    if !schedule.production_rate_per_hour.is_finite()
        || schedule.production_rate_per_hour < 0.0
    {
        return Err(ValidationError::InvalidProductionRate);
    }

    let mut event_ids = BTreeSet::new();
    let mut previous_time = None;
    let mut expected = MachineEventKind::Failure;

    for event in &schedule.events {
        if event.id.trim().is_empty() {
            return Err(ValidationError::EmptyEventId);
        }
        if !event_ids.insert(event.id.as_str()) {
            return Err(ValidationError::DuplicateEventId);
        }
        if !event.at_hours.is_finite() {
            return Err(ValidationError::NonFiniteEventTime);
        }
        if event.at_hours < 0.0 || event.at_hours > schedule.horizon_hours {
            return Err(ValidationError::EventOutsideHorizon);
        }

        if let Some(previous) = previous_time {
            if event.at_hours < previous {
                return Err(ValidationError::EventTimeOutOfOrder);
            }
            if event.at_hours == previous {
                return Err(ValidationError::DuplicateEventTime);
            }
        }

        if event.kind != expected {
            return Err(match event.kind {
                MachineEventKind::Failure => ValidationError::FailureWhileRepairing,
                MachineEventKind::RepairComplete => {
                    ValidationError::RepairCompleteWhileOperating
                }
            });
        }
        expected = match expected {
            MachineEventKind::Failure => MachineEventKind::RepairComplete,
            MachineEventKind::RepairComplete => MachineEventKind::Failure,
        };
        previous_time = Some(event.at_hours);
    }

    Ok(())
}

/// Steady-state availability for an alternating renewal process with an
/// exponential operating-time-to-failure distribution and deterministic
/// repair duration. Inputs are model parameters, not empirical estimates.
pub fn renewal_reward_availability(
    mean_operating_time_between_failures_hours: f64,
    deterministic_repair_time_hours: f64,
) -> Result<f64, ValidationError> {
    let mtbf = mean_operating_time_between_failures_hours;
    let mttr = deterministic_repair_time_hours;
    if !mtbf.is_finite() || mtbf <= 0.0 || !mttr.is_finite() || mttr <= 0.0 {
        return Err(ValidationError::InvalidHorizon);
    }
    let denominator = mtbf + mttr;
    if !denominator.is_finite() {
        return Err(ValidationError::NonFiniteAvailability);
    }
    let availability = mtbf / denominator;
    if !availability.is_finite() {
        return Err(ValidationError::NonFiniteAvailability);
    }
    Ok(availability)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, at_hours: f64, kind: MachineEventKind) -> MachineEvent {
        MachineEvent {
            id: id.to_owned(),
            at_hours,
            kind,
        }
    }

    fn schedule(
        horizon_hours: f64,
        production_rate_per_hour: f64,
        events: Vec<MachineEvent>,
    ) -> MachineSchedule {
        MachineSchedule {
            id: "known-answer".to_owned(),
            horizon_hours,
            production_rate_per_hour,
            events,
        }
    }

    #[test]
    fn no_events_is_fully_productive() {
        let result = assess(&schedule(10.0, 4.0, vec![])).unwrap();
        assert_eq!(result.productive_time_hours, 10.0);
        assert_eq!(result.repair_downtime_hours, 0.0);
        assert_eq!(result.availability, 1.0);
        assert_eq!(result.good_output_units, 40.0);
        assert_eq!(result.failure_count, 0);
        assert_eq!(result.completed_repair_count, 0);
        assert_eq!(result.final_state, MachineState::Operating);
    }

    #[test]
    fn one_failure_and_repair_has_exact_time_accounting() {
        let events = vec![
            event("failure-1", 2.0, MachineEventKind::Failure),
            event("repair-1", 5.0, MachineEventKind::RepairComplete),
        ];
        let result = assess(&schedule(8.0, 3.0, events)).unwrap();
        assert_eq!(result.productive_time_hours, 5.0);
        assert_eq!(result.repair_downtime_hours, 3.0);
        assert_eq!(result.availability, 5.0 / 8.0);
        assert_eq!(result.good_output_units, 15.0);
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.completed_repair_count, 1);
        assert_eq!(result.final_state, MachineState::Operating);
        assert!(!result.repair_right_censored);
    }

    #[test]
    fn failure_exactly_at_horizon_is_counted_without_downtime() {
        let events = vec![event("failure-at-horizon", 5.0, MachineEventKind::Failure)];
        let result = assess(&schedule(5.0, 2.0, events)).unwrap();
        assert_eq!(result.productive_time_hours, 5.0);
        assert_eq!(result.repair_downtime_hours, 0.0);
        assert_eq!(result.availability, 1.0);
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.final_state, MachineState::Repairing);
        assert!(result.repair_right_censored);
        assert_eq!(result.current_repair_elapsed_hours, Some(0.0));
    }

    #[test]
    fn repair_completion_exactly_at_horizon_is_completed() {
        let events = vec![
            event("failure", 2.0, MachineEventKind::Failure),
            event("repair-complete", 8.0, MachineEventKind::RepairComplete),
        ];
        let result = assess(&schedule(8.0, 1.0, events)).unwrap();
        assert_eq!(result.productive_time_hours, 2.0);
        assert_eq!(result.repair_downtime_hours, 6.0);
        assert_eq!(result.completed_repair_count, 1);
        assert_eq!(result.final_state, MachineState::Operating);
        assert!(!result.repair_right_censored);
    }

    #[test]
    fn horizon_during_repair_reports_right_censoring() {
        let events = vec![event("failure", 2.0, MachineEventKind::Failure)];
        let result = assess(&schedule(5.0, 1.0, events)).unwrap();
        assert_eq!(result.productive_time_hours, 2.0);
        assert_eq!(result.repair_downtime_hours, 3.0);
        assert!(result.repair_right_censored);
        assert_eq!(result.current_repair_elapsed_hours, Some(3.0));
    }

    #[test]
    fn multiple_cycles_preserve_intervals_and_totals() {
        let events = vec![
            event("failure-1", 2.0, MachineEventKind::Failure),
            event("repair-1", 4.0, MachineEventKind::RepairComplete),
            event("failure-2", 6.0, MachineEventKind::Failure),
        ];
        let result = assess(&schedule(8.0, 1.0, events)).unwrap();
        assert_eq!(result.productive_time_hours, 4.0);
        assert_eq!(result.repair_downtime_hours, 4.0);
        assert_eq!(result.failure_count, 2);
        assert_eq!(result.completed_repair_count, 1);
        assert_eq!(result.intervals.len(), 4);
        assert_eq!(result.final_state, MachineState::Repairing);
        assert_eq!(result.current_repair_elapsed_hours, Some(2.0));
    }

    #[test]
    fn zero_production_rate_is_valid_and_produces_zero_units() {
        let result = assess(&schedule(3.0, 0.0, vec![])).unwrap();
        assert_eq!(result.good_output_units, 0.0);
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let events = vec![
            event("same", 1.0, MachineEventKind::Failure),
            event("same", 2.0, MachineEventKind::RepairComplete),
        ];
        assert_eq!(
            assess(&schedule(3.0, 1.0, events)),
            Err(ValidationError::DuplicateEventId)
        );
    }

    #[test]
    fn duplicate_or_reversed_event_times_are_rejected() {
        let duplicate = vec![
            event("failure", 1.0, MachineEventKind::Failure),
            event("repair", 1.0, MachineEventKind::RepairComplete),
        ];
        assert_eq!(
            assess(&schedule(3.0, 1.0, duplicate)),
            Err(ValidationError::DuplicateEventTime)
        );

        let reversed = vec![
            event("failure", 2.0, MachineEventKind::Failure),
            event("repair", 1.0, MachineEventKind::RepairComplete),
        ];
        assert_eq!(
            assess(&schedule(3.0, 1.0, reversed)),
            Err(ValidationError::EventTimeOutOfOrder)
        );
    }

    #[test]
    fn invalid_state_transitions_are_rejected() {
        let repair_first = vec![event("repair", 1.0, MachineEventKind::RepairComplete)];
        assert_eq!(
            assess(&schedule(3.0, 1.0, repair_first)),
            Err(ValidationError::RepairCompleteWhileOperating)
        );

        let duplicate_failure = vec![
            event("failure-1", 1.0, MachineEventKind::Failure),
            event("failure-2", 2.0, MachineEventKind::Failure),
        ];
        assert_eq!(
            assess(&schedule(3.0, 1.0, duplicate_failure)),
            Err(ValidationError::FailureWhileRepairing)
        );
    }

    #[test]
    fn schedule_parameters_and_event_bounds_are_validated() {
        assert_eq!(
            assess(&schedule(0.0, 1.0, vec![])),
            Err(ValidationError::InvalidHorizon)
        );
        assert_eq!(
            assess(&schedule(3.0, f64::NAN, vec![])),
            Err(ValidationError::InvalidProductionRate)
        );
        assert_eq!(
            assess(&schedule(3.0, 1.0, vec![event("outside", 4.0, MachineEventKind::Failure)])),
            Err(ValidationError::EventOutsideHorizon)
        );
        assert_eq!(
            assess(&schedule(
                3.0,
                1.0,
                vec![event("nan", f64::NAN, MachineEventKind::Failure)]
            )),
            Err(ValidationError::NonFiniteEventTime)
        );
    }

    #[test]
    fn renewal_reward_formula_matches_hand_calculation() {
        let expected = 100.0 / 104.0;
        assert_eq!(renewal_reward_availability(100.0, 4.0).unwrap(), expected);
    }

    #[test]
    fn same_schedule_replays_to_identical_assessment() {
        let source = schedule(
            8.0,
            2.0,
            vec![
                event("failure-1", 2.0, MachineEventKind::Failure),
                event("repair-1", 4.0, MachineEventKind::RepairComplete),
                event("failure-2", 7.0, MachineEventKind::Failure),
            ],
        );
        assert_eq!(assess(&source).unwrap(), assess(&source).unwrap());
    }

    #[test]
    fn time_intervals_reconcile_to_the_horizon() {
        let source = schedule(
            9.0,
            2.0,
            vec![
                event("failure-1", 2.0, MachineEventKind::Failure),
                event("repair-1", 4.0, MachineEventKind::RepairComplete),
                event("failure-2", 7.0, MachineEventKind::Failure),
            ],
        );
        let result = assess(&source).unwrap();
        let interval_total: f64 = result.intervals.iter().map(StateInterval::duration_hours).sum();
        assert_eq!(interval_total, source.horizon_hours);
        assert_eq!(
            result.productive_time_hours + result.repair_downtime_hours,
            source.horizon_hours
        );
    }
}
