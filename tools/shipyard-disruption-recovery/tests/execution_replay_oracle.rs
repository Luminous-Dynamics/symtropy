//! Independent generated oracle for execution-versus-plan replay.
//!
//! The reference computes visibility and lateness from event maps using plain arithmetic, without
//! calling the production replay evaluator's internal helpers.

use std::collections::BTreeMap;

use shipyard_disruption_recovery::execution::{
    ExecutionEvent, ExecutionEventKind, ExecutionReplayCutoff, PlannedOperation,
    replay_execution,
};

const CASES: usize = 512;

fn next_u32(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn pick(state: &mut u32, bound: u32) -> u32 {
    next_u32(state) % bound
}

fn event(
    event_id: String,
    operation_id: &str,
    kind: ExecutionEventKind,
    occurred_tick: u64,
    recorded_tick: u64,
) -> ExecutionEvent {
    ExecutionEvent {
        event_id,
        operation_id: operation_id.to_owned(),
        source_id: "synthetic-source".to_owned(),
        evidence_ref: "synthetic://frozen-case".to_owned(),
        kind,
        occurred_tick,
        recorded_tick,
    }
}

#[test]
fn generated_execution_replay_matches_independent_as_of_oracle() {
    let mut state = 0x0e71_2026_u32;
    let mut cases = 0usize;
    let mut total_planned_operations = 0usize;
    let mut observed_start_events = 0usize;
    let mut observed_completion_events = 0usize;
    let mut fully_observed_operations = 0usize;
    let mut missing_start_events = 0usize;
    let mut missing_completion_events = 0usize;
    let mut not_due_start_operations = 0usize;
    let mut not_due_completion_operations = 0usize;
    let mut late_start_operations = 0usize;
    let mut late_completion_operations = 0usize;
    let mut total_start_lateness = 0u128;
    let mut total_completion_lateness = 0u128;
    let mut total_absolute_start_deviation = 0u128;
    let mut total_absolute_completion_deviation = 0u128;
    let mut total_duration_overrun = 0u128;
    let mut total_recording_latency = 0u128;
    let mut visible_events = 0usize;
    let mut held_by_recorded_cutoff = 0usize;
    let mut held_by_occurrence_cutoff = 0usize;
    let mut late_completion_cases = 0usize;

    for _case in 0..CASES {
        let mut plan = Vec::new();
        let mut events = Vec::new();

        for index in 0..3 {
            let operation_id = format!("op-{index}");
            let planned_start = 4 + u64::from(pick(&mut state, 12));
            let planned_duration = 1 + u64::from(pick(&mut state, 6));
            let planned_end = planned_start + planned_duration;
            plan.push(PlannedOperation {
                operation_id: operation_id.clone(),
                planned_start_tick: planned_start,
                planned_end_tick: planned_end,
            });

            let signed_start_shift = i64::from(pick(&mut state, 7)) - 2;
            let actual_start: u64 = (i64::try_from(planned_start).unwrap() + signed_start_shift)
                .try_into()
                .unwrap();
            let signed_duration_shift = i64::from(pick(&mut state, 5)) - 1;
            let actual_duration =
                (i64::try_from(planned_duration).unwrap() + signed_duration_shift).max(1) as u64;
            let actual_completion = actual_start + actual_duration;
            let start_recorded = actual_start + u64::from(pick(&mut state, 4));
            let completion_recorded = actual_completion + u64::from(pick(&mut state, 4));

            if pick(&mut state, 5) != 0 {
                events.push(event(
                    format!("start-{index}"),
                    &operation_id,
                    ExecutionEventKind::Started,
                    actual_start,
                    start_recorded,
                ));
            }
            if pick(&mut state, 4) != 0 {
                events.push(event(
                    format!("complete-{index}"),
                    &operation_id,
                    ExecutionEventKind::Completed,
                    actual_completion,
                    completion_recorded,
                ));
            }
        }

        let as_of_recorded_tick = u64::from(pick(&mut state, 34));
        let as_of_occurred_tick = u64::from(pick(&mut state, 34));
        if !events.is_empty() && pick(&mut state, 2) == 0 {
            events.reverse();
        }
        let report = replay_execution(
            &plan,
            &events,
            ExecutionReplayCutoff {
                as_of_occurred_tick,
                as_of_recorded_tick,
            },
        );
        assert!(
            report.evidence_valid,
            "case produced invalid evidence: {report:?}"
        );

        let mut starts: BTreeMap<&str, u64> = BTreeMap::new();
        let mut completions: BTreeMap<&str, u64> = BTreeMap::new();
        let mut recording_latency = 0u128;
        for item in &events {
            if item.recorded_tick > as_of_recorded_tick
                || item.occurred_tick > as_of_occurred_tick
            {
                continue;
            }
            recording_latency += u128::from(item.recorded_tick - item.occurred_tick);
            match item.kind {
                ExecutionEventKind::Started => {
                    starts.insert(item.operation_id.as_str(), item.occurred_tick);
                }
                ExecutionEventKind::Completed => {
                    completions.insert(item.operation_id.as_str(), item.occurred_tick);
                }
            }
        }

        let expected_missing_starts = plan
            .iter()
            .filter(|operation| {
                operation.planned_start_tick <= as_of_occurred_tick
                    && !starts.contains_key(operation.operation_id.as_str())
            })
            .count();
        let expected_missing_completions = plan
            .iter()
            .filter(|operation| {
                operation.planned_end_tick <= as_of_occurred_tick
                    && !completions.contains_key(operation.operation_id.as_str())
            })
            .count();
        let expected_not_yet_due_to_start = plan
            .iter()
            .filter(|operation| {
                operation.planned_start_tick > as_of_occurred_tick
                    && !starts.contains_key(operation.operation_id.as_str())
            })
            .count();
        let expected_not_yet_due_to_complete = plan
            .iter()
            .filter(|operation| {
                operation.planned_end_tick > as_of_occurred_tick
                    && !completions.contains_key(operation.operation_id.as_str())
            })
            .count();
        let expected_fully_observed = plan
            .iter()
            .filter(|operation| {
                starts.contains_key(operation.operation_id.as_str())
                    && completions.contains_key(operation.operation_id.as_str())
            })
            .count();
        let expected_late_start_operations = plan
            .iter()
            .filter(|operation| {
                starts
                    .get(operation.operation_id.as_str())
                    .is_some_and(|actual| *actual > operation.planned_start_tick)
            })
            .count();
        let expected_late_completion_operations = plan
            .iter()
            .filter(|operation| {
                completions
                    .get(operation.operation_id.as_str())
                    .is_some_and(|actual| *actual > operation.planned_end_tick)
            })
            .count();
        let expected_total_start_lateness = plan
            .iter()
            .filter_map(|operation| {
                starts
                    .get(operation.operation_id.as_str())
                    .map(|actual| actual.saturating_sub(operation.planned_start_tick))
            })
            .map(u128::from)
            .sum::<u128>();
        let expected_total_completion_lateness = plan
            .iter()
            .filter_map(|operation| {
                completions
                    .get(operation.operation_id.as_str())
                    .map(|actual| actual.saturating_sub(operation.planned_end_tick))
            })
            .map(u128::from)
            .sum::<u128>();
        let expected_total_absolute_start_deviation = plan
            .iter()
            .filter_map(|operation| {
                starts.get(operation.operation_id.as_str()).map(|actual| {
                    (i128::from(*actual) - i128::from(operation.planned_start_tick)).unsigned_abs()
                })
            })
            .sum::<u128>();
        let expected_total_absolute_completion_deviation = plan
            .iter()
            .filter_map(|operation| {
                completions
                    .get(operation.operation_id.as_str())
                    .map(|actual| {
                        (i128::from(*actual) - i128::from(operation.planned_end_tick))
                            .unsigned_abs()
                    })
            })
            .sum::<u128>();
        let expected_duration_overrun = plan
            .iter()
            .filter_map(|operation| {
                let start = starts.get(operation.operation_id.as_str())?;
                let completion = completions.get(operation.operation_id.as_str())?;
                Some(
                    completion
                        .saturating_sub(*start)
                        .saturating_sub(operation.planned_end_tick - operation.planned_start_tick),
                )
            })
            .map(u128::from)
            .sum::<u128>();
        let visible_count = events
            .iter()
            .filter(|item| {
                item.recorded_tick <= as_of_recorded_tick
                    && item.occurred_tick <= as_of_occurred_tick
            })
            .count();
        let held_back_by_recorded_count = events
            .iter()
            .filter(|item| item.recorded_tick > as_of_recorded_tick)
            .count();
        let held_back_by_occurrence_count = events
            .iter()
            .filter(|item| {
                item.recorded_tick <= as_of_recorded_tick
                    && item.occurred_tick > as_of_occurred_tick
            })
            .count();

        assert_eq!(report.metrics.planned_operations, plan.len());
        assert_eq!(report.metrics.operations_started, starts.len());
        assert_eq!(report.metrics.operations_completed, completions.len());
        assert_eq!(
            report.metrics.operations_fully_observed,
            expected_fully_observed
        );
        assert_eq!(report.metrics.missing_start_events, expected_missing_starts);
        assert_eq!(
            report.metrics.late_start_operations,
            expected_late_start_operations
        );
        assert_eq!(
            report.metrics.late_completion_operations,
            expected_late_completion_operations
        );
        assert_eq!(
            report.metrics.missing_completion_events,
            expected_missing_completions
        );
        assert_eq!(
            report.metrics.operations_not_yet_due_to_start,
            expected_not_yet_due_to_start
        );
        assert_eq!(
            report.metrics.operations_not_yet_due_to_complete,
            expected_not_yet_due_to_complete
        );
        assert_eq!(
            report.metrics.total_start_lateness_ticks,
            expected_total_start_lateness
        );
        assert_eq!(
            report.metrics.total_completion_lateness_ticks,
            expected_total_completion_lateness
        );
        assert_eq!(
            report.metrics.total_absolute_start_deviation_ticks,
            expected_total_absolute_start_deviation
        );
        assert_eq!(
            report.metrics.total_absolute_completion_deviation_ticks,
            expected_total_absolute_completion_deviation
        );
        assert_eq!(
            report.metrics.total_duration_overrun_ticks,
            expected_duration_overrun
        );
        assert_eq!(
            report.metrics.total_event_recording_latency_ticks,
            recording_latency
        );
        assert_eq!(report.metrics.events_considered, visible_count);
        assert_eq!(
            report.metrics.events_held_back_by_recorded_time,
            held_back_by_recorded_count
        );
        assert_eq!(
            report.metrics.events_held_back_by_occurrence_time,
            held_back_by_occurrence_count
        );
        for assessment in &report.operations {
            let planned = plan
                .iter()
                .find(|operation| operation.operation_id == assessment.operation_id)
                .expect("assessment should refer to the frozen plan");
            let start_due = planned.planned_start_tick <= as_of_occurred_tick;
            let completion_due = planned.planned_end_tick <= as_of_occurred_tick;
            assert_eq!(assessment.start_milestone_due, start_due);
            assert_eq!(assessment.completion_milestone_due, completion_due);
            assert_eq!(
                assessment.start_observation_missing,
                start_due && !starts.contains_key(assessment.operation_id.as_str())
            );
            assert_eq!(
                assessment.completion_observation_missing,
                completion_due && !completions.contains_key(assessment.operation_id.as_str())
            );
        }

        cases += 1;
        total_planned_operations += plan.len();
        observed_start_events += starts.len();
        observed_completion_events += completions.len();
        fully_observed_operations += expected_fully_observed;
        missing_start_events += expected_missing_starts;
        missing_completion_events += expected_missing_completions;
        not_due_start_operations += expected_not_yet_due_to_start;
        not_due_completion_operations += expected_not_yet_due_to_complete;
        late_start_operations += expected_late_start_operations;
        late_completion_operations += expected_late_completion_operations;
        total_start_lateness += expected_total_start_lateness;
        total_completion_lateness += expected_total_completion_lateness;
        total_absolute_start_deviation += expected_total_absolute_start_deviation;
        total_absolute_completion_deviation += expected_total_absolute_completion_deviation;
        total_duration_overrun += expected_duration_overrun;
        total_recording_latency += recording_latency;
        visible_events += visible_count;
        held_by_recorded_cutoff += held_back_by_recorded_count;
        held_by_occurrence_cutoff += held_back_by_occurrence_count;
        if expected_total_completion_lateness > 0 {
            late_completion_cases += 1;
        }
    }

    assert_eq!(cases, CASES);
    assert_eq!(total_planned_operations, 1_536);
    assert_eq!(observed_start_events, 546);
    assert_eq!(observed_completion_events, 320);
    assert_eq!(fully_observed_operations, 261);
    assert_eq!(missing_start_events, 504);
    assert_eq!(missing_completion_events, 580);
    assert_eq!(not_due_start_operations, 486);
    assert_eq!(not_due_completion_operations, 636);
    assert_eq!(late_start_operations, 277);
    assert_eq!(late_completion_operations, 206);
    assert_eq!(total_start_lateness, 699);
    assert_eq!(total_completion_lateness, 553);
    assert_eq!(total_absolute_start_deviation, 962);
    assert_eq!(total_absolute_completion_deviation, 663);
    assert_eq!(total_duration_overrun, 268);
    assert_eq!(total_recording_latency, 1_266);
    assert_eq!(visible_events, 866);
    assert_eq!(held_by_recorded_cutoff, 1_045);
    assert_eq!(held_by_occurrence_cutoff, 464);
    assert_eq!(late_completion_cases, 140);
}
