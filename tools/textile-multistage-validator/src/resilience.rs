#![forbid(unsafe_code)]

//! Deterministic disruption replay and replan qualification.
//!
//! This module never constructs a schedule. It applies declared disturbances
//! to cloned problem/utility inputs, protects work already started at the
//! observation tick, then independently validates a caller-supplied replan.

use std::collections::{BTreeMap, BTreeSet};

use crate::utilities::{
    UtilityEvidenceBasis, UtilityMetric, UtilityModel, UtilityReport, validate_utilities,
};
use crate::{Assignment, Interval, Problem, Schedule, SetupReservation, ValidationReport, validate};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Disruption {
    MachineUnavailable { machine_id: String, interval: Interval },
    OperatorUnavailable { operator_id: String, interval: Interval },
    MaterialAvailableAt { operation_id: String, available_at_tick: u64 },
    OperationDurationIncrease { operation_id: String, additional_ticks: u64 },
    DueDateTightened { operation_id: String, new_due_tick: u64 },
    UtilityCapacityDerated {
        metric: UtilityMetric,
        interval: Interval,
        /// None means capacity is unknown, not zero.
        capacity_units_per_tick: Option<u64>,
        /// Required when a numeric replacement capacity is supplied.
        evidence_basis: Option<UtilityEvidenceBasis>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisruptionScenario {
    pub scenario_id: String,
    /// Absolute tick at which the disruption is observed and replanning begins.
    pub observed_at_tick: u64,
    pub events: Vec<Disruption>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResilienceError {
    EmptyScenarioId,
    EmptyScenario,
    InvalidEventInterval,
    HistoricalEvent,
    DuplicateEventTarget(String),
    UnknownMachine(String),
    UnknownOperator(String),
    UnknownOperation(String),
    UtilityModelRequired,
    UtilityEnvelopeNotAffected(String),
    InvalidUtilityDerateEvidence,
    DisruptionHasNoEffect(String),
    OperationDurationOverflow(String),
    BaselineScheduleInfeasible,
    BaselineUtilityModelInfeasible,
    StartedOperationChanged(String),
    StartedSetupChanged(String),
    DisruptionInterruptsInProgressOperation(String),
    DisruptionInterruptsInProgressSetup(String),
    MetricOverflow,
}

impl std::fmt::Display for ResilienceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ResilienceError {}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlanChurn {
    pub baseline_operation_count: usize,
    pub replanned_operation_count: usize,
    pub changed_operation_count: usize,
    pub added_operation_count: usize,
    pub dropped_operation_count: usize,
    pub machine_reassignment_count: usize,
    pub operator_reassignment_count: usize,
    pub start_shift_count: usize,
    pub finish_shift_count: usize,
    pub total_absolute_start_shift_ticks: u64,
    pub total_absolute_finish_shift_ticks: u64,
    pub changed_setup_reservation_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResilienceReport {
    pub scenario_id: String,
    pub observed_at_tick: u64,
    pub baseline_feasible: bool,
    pub replan_feasible: bool,
    pub baseline_makespan_ticks: Option<u64>,
    pub replan_makespan_ticks: Option<u64>,
    /// Latest candidate finish minus the observation tick, floored at zero.
    pub post_event_completion_horizon_ticks: Option<u64>,
    pub total_tardiness_ticks: u64,
    pub maximum_tardiness_ticks: u64,
    pub churn: PlanChurn,
    pub schedule_validation: ValidationReport,
    pub utility_validation: Option<UtilityReport>,
}

fn event_key(event: &Disruption) -> String {
    match event {
        Disruption::MachineUnavailable { machine_id, .. } => format!("machine:{machine_id}"),
        Disruption::OperatorUnavailable { operator_id, .. } => format!("operator:{operator_id}"),
        Disruption::MaterialAvailableAt { operation_id, .. } => format!("material:{operation_id}"),
        Disruption::OperationDurationIncrease { operation_id, .. } => format!("duration:{operation_id}"),
        Disruption::DueDateTightened { operation_id, .. } => format!("due:{operation_id}"),
        Disruption::UtilityCapacityDerated { metric, .. } => format!("utility:{metric:?}"),
    }
}

fn valid_interval(interval: &Interval) -> bool {
    interval.start_tick < interval.end_tick
}

fn overlaps(left: &Interval, right: &Interval) -> bool {
    left.start_tick < right.end_tick && right.start_tick < left.end_tick
}

/// Historical portions are not rewritten: only the interval at/after the
/// observation tick is applied to the replanning problem.
fn future_interval(interval: &Interval, observed_at_tick: u64) -> Result<Interval, ResilienceError> {
    if !valid_interval(interval) {
        return Err(ResilienceError::InvalidEventInterval);
    }
    if interval.end_tick <= observed_at_tick {
        return Err(ResilienceError::HistoricalEvent);
    }
    Ok(Interval {
        start_tick: interval.start_tick.max(observed_at_tick),
        end_tick: interval.end_tick,
    })
}

fn subtract_interval(windows: &[Interval], outage: &Interval) -> Vec<Interval> {
    let mut remaining = Vec::new();
    for window in windows {
        if !valid_interval(window) || !overlaps(window, outage) {
            remaining.push(window.clone());
            continue;
        }
        if window.start_tick < outage.start_tick {
            remaining.push(Interval {
                start_tick: window.start_tick,
                end_tick: outage.start_tick.min(window.end_tick),
            });
        }
        if outage.end_tick < window.end_tick {
            remaining.push(Interval {
                start_tick: outage.end_tick.max(window.start_tick),
                end_tick: window.end_tick,
            });
        }
    }
    remaining.sort_by_key(|window| (window.start_tick, window.end_tick));
    remaining
}

fn assignment_map(schedule: &Schedule) -> BTreeMap<&str, &Assignment> {
    let mut result = BTreeMap::new();
    for assignment in &schedule.assignments {
        result.entry(assignment.operation_id.as_str()).or_insert(assignment);
    }
    result
}

type SetupKey = (String, Option<String>, String, String, String, u64, u64);

fn setup_key(reservation: &SetupReservation) -> SetupKey {
    (
        reservation.machine_id.clone(),
        reservation.predecessor_operation_id.clone(),
        reservation.next_operation_id.clone(),
        reservation.from_family.clone(),
        reservation.operator_id.clone(),
        reservation.start_tick,
        reservation.finish_tick,
    )
}

fn setup_set(schedule: &Schedule) -> BTreeSet<SetupKey> {
    schedule.setup_reservations.iter().map(setup_key).collect()
}

fn plan_churn(baseline: &Schedule, replan: &Schedule) -> Result<PlanChurn, ResilienceError> {
    let old = assignment_map(baseline);
    let new = assignment_map(replan);
    let mut ids = BTreeSet::new();
    ids.extend(old.keys().copied());
    ids.extend(new.keys().copied());

    let mut churn = PlanChurn {
        baseline_operation_count: baseline.assignments.len(),
        replanned_operation_count: replan.assignments.len(),
        ..PlanChurn::default()
    };

    for id in ids {
        match (old.get(id), new.get(id)) {
            (Some(before), Some(after)) => {
                let machine_changed = before.machine_id != after.machine_id;
                let operator_changed = before.operator_id != after.operator_id;
                let start_shift = before.start_tick.abs_diff(after.start_tick);
                let finish_shift = before.finish_tick.abs_diff(after.finish_tick);
                if machine_changed || operator_changed || start_shift > 0 || finish_shift > 0 {
                    churn.changed_operation_count += 1;
                }
                if machine_changed {
                    churn.machine_reassignment_count += 1;
                }
                if operator_changed {
                    churn.operator_reassignment_count += 1;
                }
                if start_shift > 0 {
                    churn.start_shift_count += 1;
                }
                if finish_shift > 0 {
                    churn.finish_shift_count += 1;
                }
                churn.total_absolute_start_shift_ticks = churn
                    .total_absolute_start_shift_ticks
                    .checked_add(start_shift)
                    .ok_or(ResilienceError::MetricOverflow)?;
                churn.total_absolute_finish_shift_ticks = churn
                    .total_absolute_finish_shift_ticks
                    .checked_add(finish_shift)
                    .ok_or(ResilienceError::MetricOverflow)?;
            }
            (Some(_), None) => {
                churn.changed_operation_count += 1;
                churn.dropped_operation_count += 1;
            }
            (None, Some(_)) => {
                churn.changed_operation_count += 1;
                churn.added_operation_count += 1;
            }
            (None, None) => {}
        }
    }

    let before = setup_set(baseline);
    let after = setup_set(replan);
    churn.changed_setup_reservation_count = before.symmetric_difference(&after).count();
    Ok(churn)
}

fn started_operation(
    baseline: &Schedule,
    operation_id: &str,
    observed_at_tick: u64,
) -> bool {
    baseline
        .assignments
        .iter()
        .find(|assignment| assignment.operation_id == operation_id)
        .is_some_and(|assignment| assignment.start_tick < observed_at_tick)
}

fn validate_started_work_unchanged(
    baseline: &Schedule,
    replan: &Schedule,
    observed_at_tick: u64,
) -> Result<(), ResilienceError> {
    let before = assignment_map(baseline);
    let after = assignment_map(replan);
    for (operation_id, assignment) in before {
        if assignment.start_tick < observed_at_tick
            && after.get(operation_id).copied() != Some(assignment)
        {
            return Err(ResilienceError::StartedOperationChanged(operation_id.to_owned()));
        }
    }

    let before_setup = setup_set(baseline);
    let after_setup = setup_set(replan);
    for reservation in &baseline.setup_reservations {
        if reservation.start_tick < observed_at_tick && !after_setup.contains(&setup_key(reservation)) {
            return Err(ResilienceError::StartedSetupChanged(
                reservation.next_operation_id.clone(),
            ));
        }
    }
    Ok(())
}

fn interrupting_operation(
    schedule: &Schedule,
    resource_id: &str,
    machine: bool,
    interval: &Interval,
    observed_at_tick: u64,
) -> Option<String> {
    schedule.assignments.iter().find_map(|assignment| {
        let resource_matches = if machine {
            assignment.machine_id == resource_id
        } else {
            assignment.operator_id == resource_id
        };
        (resource_matches
            && assignment.start_tick < observed_at_tick
            && assignment.finish_tick > observed_at_tick
            && overlaps(
                interval,
                &Interval {
                    start_tick: assignment.start_tick,
                    end_tick: assignment.finish_tick,
                },
            ))
        .then(|| assignment.operation_id.clone())
    })
}

fn interrupting_setup(
    schedule: &Schedule,
    resource_id: &str,
    machine: bool,
    interval: &Interval,
    observed_at_tick: u64,
) -> Option<String> {
    schedule.setup_reservations.iter().find_map(|reservation| {
        let resource_matches = if machine {
            reservation.machine_id == resource_id
        } else {
            reservation.operator_id == resource_id
        };
        (resource_matches
            && reservation.start_tick < observed_at_tick
            && reservation.finish_tick > observed_at_tick
            && overlaps(
                interval,
                &Interval {
                    start_tick: reservation.start_tick,
                    end_tick: reservation.finish_tick,
                },
            ))
        .then(|| reservation.next_operation_id.clone())
    })
}

fn apply_utility_derate(
    model: &mut UtilityModel,
    metric: UtilityMetric,
    interval: &Interval,
    capacity: Option<u64>,
    basis: Option<&UtilityEvidenceBasis>,
) -> Result<(), ResilienceError> {
    if capacity.is_some() != basis.is_some() {
        return Err(ResilienceError::InvalidUtilityDerateEvidence);
    }

    let original = std::mem::take(&mut model.envelopes);
    let mut updated = Vec::with_capacity(original.len() + 4);
    let mut affected = false;
    let mut changed = false;

    for envelope in original {
        if envelope.metric != metric || !overlaps(&envelope.interval, interval) {
            updated.push(envelope);
            continue;
        }

        affected = true;
        let overlap_start = envelope.interval.start_tick.max(interval.start_tick);
        let overlap_end = envelope.interval.end_tick.min(interval.end_tick);

        if envelope.interval.start_tick < overlap_start {
            updated.push(crate::utilities::UtilityEnvelope {
                metric: envelope.metric,
                interval: Interval {
                    start_tick: envelope.interval.start_tick,
                    end_tick: overlap_start,
                },
                capacity_units_per_tick: envelope.capacity_units_per_tick,
                capacity_basis: envelope.capacity_basis.clone(),
            });
        }

        let (new_capacity, new_basis) = match (envelope.capacity_units_per_tick, capacity) {
            (Some(old), Some(requested)) if requested < old => {
                changed = true;
                (Some(requested), basis.cloned())
            }
            (Some(_), None) => {
                changed = true;
                (None, None)
            }
            (None, _) => (None, None),
            (Some(old), Some(_)) => (Some(old), envelope.capacity_basis.clone()),
        };

        updated.push(crate::utilities::UtilityEnvelope {
            metric: envelope.metric,
            interval: Interval {
                start_tick: overlap_start,
                end_tick: overlap_end,
            },
            capacity_units_per_tick: new_capacity,
            capacity_basis: new_basis,
        });

        if overlap_end < envelope.interval.end_tick {
            updated.push(crate::utilities::UtilityEnvelope {
                metric: envelope.metric,
                interval: Interval {
                    start_tick: overlap_end,
                    end_tick: envelope.interval.end_tick,
                },
                capacity_units_per_tick: envelope.capacity_units_per_tick,
                capacity_basis: envelope.capacity_basis,
            });
        }
    }

    if !affected {
        return Err(ResilienceError::UtilityEnvelopeNotAffected(format!("{metric:?}")));
    }
    if !changed {
        return Err(ResilienceError::DisruptionHasNoEffect(format!("utility:{metric:?}")));
    }

    updated.sort_by_key(|envelope| {
        (
            envelope.metric,
            envelope.interval.start_tick,
            envelope.interval.end_tick,
        )
    });
    model.envelopes = updated;
    Ok(())
}

fn apply_scenario(
    problem: &Problem,
    utility_model: Option<&UtilityModel>,
    scenario: &DisruptionScenario,
    baseline: &Schedule,
) -> Result<(Problem, Option<UtilityModel>), ResilienceError> {
    if scenario.scenario_id.trim().is_empty() {
        return Err(ResilienceError::EmptyScenarioId);
    }
    if scenario.events.is_empty() {
        return Err(ResilienceError::EmptyScenario);
    }

    let mut changed = problem.clone();
    let mut changed_utilities = utility_model.cloned();
    let mut targets = BTreeSet::new();

    for event in &scenario.events {
        let key = event_key(event);
        if !targets.insert(key.clone()) {
            return Err(ResilienceError::DuplicateEventTarget(key));
        }

        match event {
            Disruption::MachineUnavailable { machine_id, interval } => {
                let future = future_interval(interval, scenario.observed_at_tick)?;
                if let Some(operation) = interrupting_operation(
                    baseline,
                    machine_id,
                    true,
                    &future,
                    scenario.observed_at_tick,
                ) {
                    return Err(ResilienceError::DisruptionInterruptsInProgressOperation(operation));
                }
                if let Some(operation) = interrupting_setup(
                    baseline,
                    machine_id,
                    true,
                    &future,
                    scenario.observed_at_tick,
                ) {
                    return Err(ResilienceError::DisruptionInterruptsInProgressSetup(operation));
                }
                let machine = changed
                    .machines
                    .iter_mut()
                    .find(|machine| machine.id == *machine_id)
                    .ok_or_else(|| ResilienceError::UnknownMachine(machine_id.clone()))?;
                let reduced = subtract_interval(&machine.availability, &future);
                if reduced == machine.availability {
                    return Err(ResilienceError::DisruptionHasNoEffect(key));
                }
                machine.availability = reduced;
            }
            Disruption::OperatorUnavailable { operator_id, interval } => {
                let future = future_interval(interval, scenario.observed_at_tick)?;
                if let Some(operation) = interrupting_operation(
                    baseline,
                    operator_id,
                    false,
                    &future,
                    scenario.observed_at_tick,
                ) {
                    return Err(ResilienceError::DisruptionInterruptsInProgressOperation(operation));
                }
                if let Some(operation) = interrupting_setup(
                    baseline,
                    operator_id,
                    false,
                    &future,
                    scenario.observed_at_tick,
                ) {
                    return Err(ResilienceError::DisruptionInterruptsInProgressSetup(operation));
                }
                let operator = changed
                    .operators
                    .iter_mut()
                    .find(|operator| operator.id == *operator_id)
                    .ok_or_else(|| ResilienceError::UnknownOperator(operator_id.clone()))?;
                let reduced = subtract_interval(&operator.availability, &future);
                if reduced == operator.availability {
                    return Err(ResilienceError::DisruptionHasNoEffect(key));
                }
                operator.availability = reduced;
            }
            Disruption::MaterialAvailableAt {
                operation_id,
                available_at_tick,
            } => {
                let operation = changed
                    .operations
                    .iter_mut()
                    .find(|operation| operation.id == *operation_id)
                    .ok_or_else(|| ResilienceError::UnknownOperation(operation_id.clone()))?;
                if started_operation(baseline, operation_id, scenario.observed_at_tick) {
                    return Err(ResilienceError::DisruptionInterruptsInProgressOperation(
                        operation_id.clone(),
                    ));
                }
                let new_release = operation.release_tick.max(*available_at_tick);
                if new_release == operation.release_tick {
                    return Err(ResilienceError::DisruptionHasNoEffect(key));
                }
                operation.release_tick = new_release;
            }
            Disruption::OperationDurationIncrease {
                operation_id,
                additional_ticks,
            } => {
                if *additional_ticks == 0 {
                    return Err(ResilienceError::DisruptionHasNoEffect(key));
                }
                let operation = changed
                    .operations
                    .iter_mut()
                    .find(|operation| operation.id == *operation_id)
                    .ok_or_else(|| ResilienceError::UnknownOperation(operation_id.clone()))?;
                if started_operation(baseline, operation_id, scenario.observed_at_tick) {
                    return Err(ResilienceError::DisruptionInterruptsInProgressOperation(
                        operation_id.clone(),
                    ));
                }
                operation.duration_ticks = operation
                    .duration_ticks
                    .checked_add(*additional_ticks)
                    .ok_or_else(|| ResilienceError::OperationDurationOverflow(operation_id.clone()))?;
            }
            Disruption::DueDateTightened {
                operation_id,
                new_due_tick,
            } => {
                let operation = changed
                    .operations
                    .iter_mut()
                    .find(|operation| operation.id == *operation_id)
                    .ok_or_else(|| ResilienceError::UnknownOperation(operation_id.clone()))?;
                if started_operation(baseline, operation_id, scenario.observed_at_tick) {
                    return Err(ResilienceError::DisruptionInterruptsInProgressOperation(
                        operation_id.clone(),
                    ));
                }
                if operation.due_tick.is_some_and(|old| *new_due_tick >= old) {
                    return Err(ResilienceError::DisruptionHasNoEffect(key));
                }
                operation.due_tick = Some(*new_due_tick);
            }
            Disruption::UtilityCapacityDerated {
                metric,
                interval,
                capacity_units_per_tick,
                evidence_basis,
            } => {
                let future = future_interval(interval, scenario.observed_at_tick)?;
                let model = changed_utilities
                    .as_mut()
                    .ok_or(ResilienceError::UtilityModelRequired)?;
                apply_utility_derate(
                    model,
                    *metric,
                    &future,
                    *capacity_units_per_tick,
                    evidence_basis.as_ref(),
                )?;
            }
        }
    }

    Ok((changed, changed_utilities))
}

fn tardiness(problem: &Problem, schedule: &Schedule) -> Result<(u64, u64), ResilienceError> {
    let mut total = 0_u64;
    let mut maximum = 0_u64;
    let due = problem
        .operations
        .iter()
        .map(|operation| (operation.id.as_str(), operation.due_tick))
        .collect::<BTreeMap<_, _>>();

    for assignment in &schedule.assignments {
        if let Some(Some(due_tick)) = due.get(assignment.operation_id.as_str()) {
            let late = assignment.finish_tick.saturating_sub(*due_tick);
            total = total.checked_add(late).ok_or(ResilienceError::MetricOverflow)?;
            maximum = maximum.max(late);
        }
    }
    Ok((total, maximum))
}

/// Replays a disturbance and independently checks the supplied replan.
///
/// A feasible baseline schedule/model is mandatory. Assignments and setup
/// reservations that started before the observation tick are immutable because
/// this v0.1 contract does not define abort/restart semantics.
pub fn evaluate_replan(
    problem: &Problem,
    baseline: &Schedule,
    replan: &Schedule,
    scenario: &DisruptionScenario,
    utility_model: Option<&UtilityModel>,
) -> Result<ResilienceReport, ResilienceError> {
    let baseline_schedule_report = validate(problem, baseline);
    if !baseline_schedule_report.valid {
        return Err(ResilienceError::BaselineScheduleInfeasible);
    }
    if let Some(model) = utility_model {
        if !validate_utilities(problem, baseline, model).valid {
            return Err(ResilienceError::BaselineUtilityModelInfeasible);
        }
    }

    let (disrupted_problem, disrupted_utility_model) =
        apply_scenario(problem, utility_model, scenario, baseline)?;
    validate_started_work_unchanged(baseline, replan, scenario.observed_at_tick)?;

    let schedule_validation = validate(&disrupted_problem, replan);
    let utility_validation = disrupted_utility_model
        .as_ref()
        .map(|model| validate_utilities(&disrupted_problem, replan, model));

    let churn = plan_churn(baseline, replan)?;
    let (total_tardiness_ticks, maximum_tardiness_ticks) =
        tardiness(&disrupted_problem, replan)?;
    let latest_finish = replan
        .assignments
        .iter()
        .map(|assignment| assignment.finish_tick)
        .max();
    let post_event_completion_horizon_ticks =
        latest_finish.map(|finish| finish.saturating_sub(scenario.observed_at_tick));

    let replan_feasible = schedule_validation.valid
        && utility_validation.as_ref().is_none_or(|report| report.valid);

    Ok(ResilienceReport {
        scenario_id: scenario.scenario_id.clone(),
        observed_at_tick: scenario.observed_at_tick,
        baseline_feasible: true,
        replan_feasible,
        baseline_makespan_ticks: baseline_schedule_report.makespan_ticks,
        replan_makespan_ticks: schedule_validation.makespan_ticks,
        post_event_completion_horizon_ticks,
        total_tardiness_ticks,
        maximum_tardiness_ticks,
        churn,
        schedule_validation,
        utility_validation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::two_job_four_stage_case;
    use crate::utilities::{
        UtilityConsumerKey, UtilityConsumerKind, UtilityConsumerProfile, UtilityEnvelope,
        UtilityEvidenceKind, UtilityRate,
    };

    fn basis(id: &str) -> UtilityEvidenceBasis {
        UtilityEvidenceBasis {
            kind: UtilityEvidenceKind::EngineeringEstimate,
            evidence_id: id.to_owned(),
            sha256: "a".repeat(64),
            method_ref: "synthetic-test-method-v1".to_owned(),
            calibration_id: None,
        }
    }

    fn full_utility_model() -> UtilityModel {
        let (problem, _) = two_job_four_stage_case();
        let profiles = problem
            .operations
            .iter()
            .map(|operation| UtilityConsumerProfile {
                consumer: UtilityConsumerKey {
                    kind: UtilityConsumerKind::Operation,
                    id: operation.id.clone(),
                },
                complete: true,
                demand_basis: basis(&format!("demand:{}", operation.id)),
                rates: vec![UtilityRate {
                    metric: UtilityMetric::ElectricalPowerW,
                    units_per_tick: Some(10),
                }],
            })
            .collect();
        UtilityModel {
            tick_unit: "minute".to_owned(),
            profiles,
            envelopes: vec![UtilityEnvelope {
                metric: UtilityMetric::ElectricalPowerW,
                interval: Interval {
                    start_tick: 0,
                    end_tick: 30,
                },
                capacity_units_per_tick: Some(20),
                capacity_basis: Some(basis("capacity:grid")),
            }],
        }
    }

    #[test]
    fn machine_outage_replan_uses_qualified_alternate_resources() {
        let (mut problem, baseline) = two_job_four_stage_case();
        let mut backup_machine = problem
            .machines
            .iter()
            .find(|machine| machine.id == "cut-line")
            .unwrap()
            .clone();
        backup_machine.id = "backup-cut-line".to_owned();
        problem.machines.push(backup_machine);
        let mut backup_operator = problem
            .operators
            .iter()
            .find(|operator| operator.id == "cutter")
            .unwrap()
            .clone();
        backup_operator.id = "backup-cutter".to_owned();
        problem.operators.push(backup_operator);

        let mut replan = baseline.clone();
        for assignment in &mut replan.assignments {
            if assignment.machine_id == "cut-line" {
                assignment.machine_id = "backup-cut-line".to_owned();
                assignment.operator_id = "backup-cutter".to_owned();
            }
        }

        let report = evaluate_replan(
            &problem,
            &baseline,
            &replan,
            &DisruptionScenario {
                scenario_id: "machine-outage-001".to_owned(),
                observed_at_tick: 0,
                events: vec![Disruption::MachineUnavailable {
                    machine_id: "cut-line".to_owned(),
                    interval: Interval { start_tick: 0, end_tick: 30 },
                }],
            },
            None,
        )
        .unwrap();

        assert!(report.baseline_feasible);
        assert!(report.replan_feasible, "{:#?}", report.schedule_validation.findings);
        assert_eq!(report.churn.changed_operation_count, 2);
        assert_eq!(report.churn.machine_reassignment_count, 2);
        assert_eq!(report.churn.operator_reassignment_count, 2);
        assert_eq!(report.post_event_completion_horizon_ticks, Some(15));
    }

    #[test]
    fn operator_absence_replan_uses_qualified_substitute() {
        let (mut problem, baseline) = two_job_four_stage_case();
        let mut substitute = problem
            .operators
            .iter()
            .find(|operator| operator.id == "cutter")
            .unwrap()
            .clone();
        substitute.id = "substitute-cutter".to_owned();
        problem.operators.push(substitute);

        let mut replan = baseline.clone();
        for assignment in &mut replan.assignments {
            if assignment.operator_id == "cutter" {
                assignment.operator_id = "substitute-cutter".to_owned();
            }
        }

        let report = evaluate_replan(
            &problem,
            &baseline,
            &replan,
            &DisruptionScenario {
                scenario_id: "operator-absence-001".to_owned(),
                observed_at_tick: 0,
                events: vec![Disruption::OperatorUnavailable {
                    operator_id: "cutter".to_owned(),
                    interval: Interval { start_tick: 0, end_tick: 30 },
                }],
            },
            None,
        )
        .unwrap();

        assert!(report.replan_feasible, "{:#?}", report.schedule_validation.findings);
        assert_eq!(report.churn.operator_reassignment_count, 2);
    }

    #[test]
    fn material_delay_requires_release_and_precedence_revalidation() {
        let (problem, baseline) = two_job_four_stage_case();
        let mut replan = baseline.clone();
        let scenario = DisruptionScenario {
            scenario_id: "material-delay-001".to_owned(),
            observed_at_tick: 0,
            events: vec![Disruption::MaterialAvailableAt {
                operation_id: "cut-a".to_owned(),
                available_at_tick: 5,
            }],
        };

        let report = evaluate_replan(&problem, &baseline, &replan, &scenario, None).unwrap();
        assert!(!report.replan_feasible);
        assert!(report.schedule_validation.findings.iter().any(|finding| {
            finding.code == "RELEASE_TIME_VIOLATION"
        }));

        for (operation_id, start, finish) in [
            ("cut-b", 3, 5),
            ("cut-a", 5, 8),
            ("sew-b", 5, 9),
            ("sew-a", 9, 14),
            ("finish-b", 9, 11),
            ("inspect-b", 11, 12),
            ("finish-a", 14, 16),
            ("inspect-a", 16, 17),
        ] {
            let assignment = replan
                .assignments
                .iter_mut()
                .find(|assignment| assignment.operation_id == operation_id)
                .unwrap();
            assignment.start_tick = start;
            assignment.finish_tick = finish;
        }

        let report = evaluate_replan(&problem, &baseline, &replan, &scenario, None).unwrap();
        assert!(report.replan_feasible, "{:#?}", report.schedule_validation.findings);
        assert_eq!(report.replan_makespan_ticks, Some(17));
        assert_eq!(report.churn.start_shift_count, 8);
    }

    #[test]
    fn increased_operation_duration_changes_hard_duration_and_downstream_precedence() {
        let (problem, baseline) = two_job_four_stage_case();
        let mut replan = baseline.clone();
        for (operation_id, start, finish) in [
            ("cut-a", 0, 4),
            ("sew-a", 4, 9),
            ("finish-a", 9, 11),
            ("inspect-a", 11, 12),
            ("cut-b", 4, 6),
            ("sew-b", 9, 13),
            ("finish-b", 13, 15),
            ("inspect-b", 15, 16),
        ] {
            let assignment = replan
                .assignments
                .iter_mut()
                .find(|assignment| assignment.operation_id == operation_id)
                .unwrap();
            assignment.start_tick = start;
            assignment.finish_tick = finish;
        }
        let report = evaluate_replan(
            &problem,
            &baseline,
            &replan,
            &DisruptionScenario {
                scenario_id: "longer-cutting-cycle-001".to_owned(),
                observed_at_tick: 0,
                events: vec![Disruption::OperationDurationIncrease {
                    operation_id: "cut-a".to_owned(),
                    additional_ticks: 1,
                }],
            },
            None,
        )
        .unwrap();
        assert!(report.replan_feasible, "{:#?}", report.schedule_validation.findings);
        assert_eq!(report.replan_makespan_ticks, Some(16));
    }

    #[test]
    fn tightened_due_date_is_a_hard_constraint() {
        let (problem, baseline) = two_job_four_stage_case();
        let report = evaluate_replan(
            &problem,
            &baseline,
            &baseline,
            &DisruptionScenario {
                scenario_id: "expedite-001".to_owned(),
                observed_at_tick: 0,
                events: vec![Disruption::DueDateTightened {
                    operation_id: "inspect-a".to_owned(),
                    new_due_tick: 10,
                }],
            },
            None,
        )
        .unwrap();
        assert!(!report.replan_feasible);
        assert_eq!(report.total_tardiness_ticks, 1);
        assert_eq!(report.maximum_tardiness_ticks, 1);
    }

    #[test]
    fn utility_derating_splits_only_the_affected_window_and_allows_deconfliction() {
        let (problem, baseline) = two_job_four_stage_case();
        let mut replan = baseline.clone();
        for (operation_id, start, finish) in [
            ("sew-b", 11, 15),
            ("finish-b", 15, 17),
            ("inspect-b", 17, 18),
        ] {
            let assignment = replan
                .assignments
                .iter_mut()
                .find(|assignment| assignment.operation_id == operation_id)
                .unwrap();
            assignment.start_tick = start;
            assignment.finish_tick = finish;
        }

        let utilities = full_utility_model();
        let report = evaluate_replan(
            &problem,
            &baseline,
            &replan,
            &DisruptionScenario {
                scenario_id: "grid-capacity-derate-001".to_owned(),
                observed_at_tick: 0,
                events: vec![Disruption::UtilityCapacityDerated {
                    metric: UtilityMetric::ElectricalPowerW,
                    interval: Interval { start_tick: 8, end_tick: 11 },
                    capacity_units_per_tick: Some(10),
                    evidence_basis: Some(basis("grid-derate-notice-001")),
                }],
            },
            Some(&utilities),
        )
        .unwrap();

        assert!(report.replan_feasible, "{:#?}", report.utility_validation);
        let utility = report.utility_validation.unwrap();
        assert!(utility.valid, "{:#?}", utility.findings);
        assert_eq!(report.post_event_completion_horizon_ticks, Some(18));
        assert_eq!(report.churn.start_shift_count, 3);
    }

    #[test]
    fn unknown_derated_capacity_stays_unknown_and_blocks_qualification() {
        let (problem, baseline) = two_job_four_stage_case();
        let report = evaluate_replan(
            &problem,
            &baseline,
            &baseline,
            &DisruptionScenario {
                scenario_id: "unknown-grid-capacity-001".to_owned(),
                observed_at_tick: 0,
                events: vec![Disruption::UtilityCapacityDerated {
                    metric: UtilityMetric::ElectricalPowerW,
                    interval: Interval { start_tick: 0, end_tick: 30 },
                    capacity_units_per_tick: None,
                    evidence_basis: None,
                }],
            },
            Some(&full_utility_model()),
        )
        .unwrap();
        assert!(!report.replan_feasible);
        assert!(
            report
                .utility_validation
                .unwrap()
                .findings
                .iter()
                .any(|finding| finding.code == "UTILITY_CAPACITY_UNKNOWN")
        );
    }

    #[test]
    fn baseline_must_be_feasible_before_replan_comparison() {
        let (problem, mut baseline) = two_job_four_stage_case();
        baseline.assignments[0].finish_tick += 1;
        let error = evaluate_replan(
            &problem,
            &baseline,
            &baseline,
            &DisruptionScenario {
                scenario_id: "invalid-baseline".to_owned(),
                observed_at_tick: 0,
                events: vec![Disruption::MachineUnavailable {
                    machine_id: "cut-line".to_owned(),
                    interval: Interval { start_tick: 0, end_tick: 5 },
                }],
            },
            None,
        )
        .unwrap_err();
        assert_eq!(error, ResilienceError::BaselineScheduleInfeasible);
    }

    #[test]
    fn started_work_cannot_be_rewritten_by_replanning() {
        let (problem, baseline) = two_job_four_stage_case();
        let mut replan = baseline.clone();
        let assignment = replan
            .assignments
            .iter_mut()
            .find(|assignment| assignment.operation_id == "sew-a")
            .unwrap();
        assignment.start_tick = 4;
        assignment.finish_tick = 9;

        let error = evaluate_replan(
            &problem,
            &baseline,
            &replan,
            &DisruptionScenario {
                scenario_id: "rewrite-started-work".to_owned(),
                observed_at_tick: 5,
                events: vec![Disruption::MaterialAvailableAt {
                    operation_id: "cut-b".to_owned(),
                    available_at_tick: 5,
                }],
            },
            None,
        )
        .unwrap_err();
        assert_eq!(
            error,
            ResilienceError::StartedOperationChanged("sew-a".to_owned())
        );
    }

    #[test]
    fn outage_interrupting_in_progress_operation_requires_incident_path() {
        let (problem, baseline) = two_job_four_stage_case();
        let error = evaluate_replan(
            &problem,
            &baseline,
            &baseline,
            &DisruptionScenario {
                scenario_id: "interrupt-running-cut".to_owned(),
                observed_at_tick: 5,
                events: vec![Disruption::MachineUnavailable {
                    machine_id: "cut-line".to_owned(),
                    interval: Interval { start_tick: 4, end_tick: 6 },
                }],
            },
            None,
        )
        .unwrap_err();
        assert_eq!(
            error,
            ResilienceError::DisruptionInterruptsInProgressOperation("cut-b".to_owned())
        );
    }

    #[test]
    fn duplicate_and_historical_events_are_rejected() {
        let (problem, baseline) = two_job_four_stage_case();
        let duplicate = evaluate_replan(
            &problem,
            &baseline,
            &baseline,
            &DisruptionScenario {
                scenario_id: "duplicate-outage".to_owned(),
                observed_at_tick: 0,
                events: vec![
                    Disruption::MachineUnavailable {
                        machine_id: "cut-line".to_owned(),
                        interval: Interval { start_tick: 0, end_tick: 2 },
                    },
                    Disruption::MachineUnavailable {
                        machine_id: "cut-line".to_owned(),
                        interval: Interval { start_tick: 4, end_tick: 6 },
                    },
                ],
            },
            None,
        )
        .unwrap_err();
        assert_eq!(
            duplicate,
            ResilienceError::DuplicateEventTarget("machine:cut-line".to_owned())
        );

        let historical = evaluate_replan(
            &problem,
            &baseline,
            &baseline,
            &DisruptionScenario {
                scenario_id: "historical-outage".to_owned(),
                observed_at_tick: 10,
                events: vec![Disruption::MachineUnavailable {
                    machine_id: "cut-line".to_owned(),
                    interval: Interval { start_tick: 1, end_tick: 5 },
                }],
            },
            None,
        )
        .unwrap_err();
        assert_eq!(historical, ResilienceError::HistoricalEvent);
    }
}
