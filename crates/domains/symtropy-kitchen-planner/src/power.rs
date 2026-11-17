// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Power-envelope constraints for deterministic kitchen schedules.
//!
//! This module composes with the existing task/resource planner. It does not
//! infer household circuit capacity from appliance labels and it never controls
//! hardware. Power inputs are explicit piecewise-constant real-power profiles.
//! Missing task profiles fail closed. A schedule that exceeds the declared
//! envelope is not returned as feasible; the builder tries a conservative
//! serial baseline before reporting that its heuristics found no schedule.

use crate::{
    KitchenPlanV1, KitchenScheduleReceiptV1, build_schedule, build_serial_baseline,
    validate_schedule,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const POWER_CONSTRAINED_PLAN_SCHEMA_VERSION: u32 = 1;

/// Declared household real-power envelope. These values are inputs supplied by
/// a user, installer or independently measured site profile; they are not
/// inferred from a nominal appliance wattage.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PowerEnvelopeV1 {
    /// Maximum sustained aggregate real power in watts.
    pub continuous_real_power_limit_w: u64,
    /// Maximum aggregate short-duration real power in watts.
    pub surge_real_power_limit_w: u64,
    /// Longest contiguous interval above the continuous limit, in milliseconds.
    /// Set to zero to prohibit any excursion above the continuous limit.
    pub max_surge_duration_ms: u64,
    /// Declared background household load present during the scheduled meal.
    pub background_real_power_w: u64,
}

/// One piecewise-constant real-power segment relative to task start.
/// Segments must be ordered, contiguous, positive-duration and cover the entire
/// task duration. A zero-watt segment is explicit evidence that the task draws
/// no power during that interval.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PowerSegmentV1 {
    pub start_offset_ms: u64,
    pub end_offset_ms: u64,
    pub real_power_w: u64,
}

/// Provenance class for a task's power profile. "Measured" still requires an
/// external calibration / instrument review before being used as a qualified
/// physical claim; this enum only preserves the declared evidence class.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerProfileEvidenceClassV1 {
    Measured,
    Declared,
    Modelled,
    Simulated,
}

/// Full-duration power trace for one scheduled task.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TaskPowerProfileV1 {
    pub task_id: String,
    pub source_ref: String,
    pub evidence_class: PowerProfileEvidenceClassV1,
    pub segments: Vec<PowerSegmentV1>,
}

/// Complete input for power-aware scheduling. It composes with the existing
/// plan and does not modify KitchenPlanV1 or KitchenTaskV1.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PowerConstrainedPlanV1 {
    pub schema_version: u32,
    pub plan: KitchenPlanV1,
    pub envelope: PowerEnvelopeV1,
    /// Exactly one profile is required for every task, including manual tasks
    /// (represent these with an explicit zero-watt segment).
    pub task_profiles: Vec<TaskPowerProfileV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerConstraintViolationCode {
    UnsupportedSchemaVersion,
    InvalidEnvelope,
    MissingTaskProfile,
    DuplicateTaskProfile,
    UnknownTaskProfile,
    EmptyTaskProfileSource,
    EmptyTaskProfile,
    SegmentDoesNotStartAtZero,
    SegmentGap,
    SegmentOverlap,
    InvalidSegmentDuration,
    SegmentBeyondTaskDuration,
    SegmentDoesNotCoverTask,
    TimeOverflow,
    PowerArithmeticOverflow,
    SourcePlanInvalid,
    ScheduleInvalid,
    PeakPowerExceedsSurgeLimit,
    SurgeDurationExceedsLimit,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PowerConstraintViolation {
    pub code: PowerConstraintViolationCode,
    pub task_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PowerValidationReportV1 {
    pub schema_version: u32,
    pub plan_id: String,
    pub peak_real_power_w: Option<u64>,
    pub longest_surge_ms: u64,
    pub all_profiles_labelled_measured: bool,
    pub evidence_classes: Vec<PowerProfileEvidenceClassV1>,
    pub violations: Vec<PowerConstraintViolation>,
}

impl PowerValidationReportV1 {
    pub fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerScheduleStrategyV1 {
    ParallelEarliestGap,
    SerialFallback,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PowerScheduleReceiptV1 {
    pub schema_version: u32,
    pub schedule: KitchenScheduleReceiptV1,
    pub power_validation: PowerValidationReportV1,
    pub strategy: PowerScheduleStrategyV1,
}

/// A power-aware scheduling heuristic failure is not a proof of global
/// infeasibility. It means the existing parallel and serial heuristics did not
/// find a schedule satisfying the declared inputs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PowerScheduleErrorV1 {
    InvalidInput(Vec<PowerConstraintViolation>),
    NoScheduleFoundByHeuristics {
        parallel_schedule_error: Option<String>,
        parallel_power_violations: Vec<PowerConstraintViolation>,
        serial_schedule_error: Option<String>,
        serial_power_violations: Vec<PowerConstraintViolation>,
    },
}

fn violation(
    violations: &mut Vec<PowerConstraintViolation>,
    code: PowerConstraintViolationCode,
    task_id: Option<&str>,
    detail: impl Into<String>,
) {
    violations.push(PowerConstraintViolation {
        code,
        task_id: task_id.map(str::to_owned),
        detail: detail.into(),
    });
}

fn sort_violations(violations: &mut [PowerConstraintViolation]) {
    violations.sort_by(|a, b| {
        (
            a.code,
            a.task_id.as_deref().unwrap_or(""),
            a.detail.as_str(),
        )
            .cmp(&(
                b.code,
                b.task_id.as_deref().unwrap_or(""),
                b.detail.as_str(),
            ))
    });
}

fn validate_inputs(input: &PowerConstrainedPlanV1) -> Vec<PowerConstraintViolation> {
    let mut violations = Vec::new();
    if input.schema_version != POWER_CONSTRAINED_PLAN_SCHEMA_VERSION {
        violation(
            &mut violations,
            PowerConstraintViolationCode::UnsupportedSchemaVersion,
            None,
            format!(
                "unsupported power-plan schema version {}",
                input.schema_version
            ),
        );
    }

    let envelope = &input.envelope;
    if envelope.continuous_real_power_limit_w == 0
        || envelope.surge_real_power_limit_w < envelope.continuous_real_power_limit_w
        || envelope.background_real_power_w > envelope.surge_real_power_limit_w
    {
        violation(
            &mut violations,
            PowerConstraintViolationCode::InvalidEnvelope,
            None,
            "continuous limit must be positive, surge limit must be at least continuous limit, and background load must not exceed surge limit",
        );
    }

    let plan_report = crate::validate_plan(&input.plan);
    if !plan_report.is_valid() {
        violation(
            &mut violations,
            PowerConstraintViolationCode::SourcePlanInvalid,
            None,
            format!(
                "source plan has {} validation violation(s)",
                plan_report.violations.len()
            ),
        );
    }

    let task_durations: BTreeMap<&str, u64> = input
        .plan
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task.duration_ms))
        .collect();

    let mut seen = BTreeSet::new();
    for profile in &input.task_profiles {
        if !task_durations.contains_key(profile.task_id.as_str()) {
            violation(
                &mut violations,
                PowerConstraintViolationCode::UnknownTaskProfile,
                Some(&profile.task_id),
                "power profile references a task absent from the plan",
            );
            continue;
        }
        if !seen.insert(profile.task_id.as_str()) {
            violation(
                &mut violations,
                PowerConstraintViolationCode::DuplicateTaskProfile,
                Some(&profile.task_id),
                "more than one power profile exists for this task",
            );
        }
        if profile.source_ref.trim().is_empty() {
            violation(
                &mut violations,
                PowerConstraintViolationCode::EmptyTaskProfileSource,
                Some(&profile.task_id),
                "power profile source_ref must be non-empty",
            );
        }
        let Some(duration_ms) = task_durations.get(profile.task_id.as_str()).copied() else {
            continue;
        };
        if profile.segments.is_empty() {
            violation(
                &mut violations,
                PowerConstraintViolationCode::EmptyTaskProfile,
                Some(&profile.task_id),
                "power profile must explicitly cover the task duration",
            );
            continue;
        }

        let mut expected_start = 0_u64;
        for segment in &profile.segments {
            if segment.end_offset_ms <= segment.start_offset_ms {
                violation(
                    &mut violations,
                    PowerConstraintViolationCode::InvalidSegmentDuration,
                    Some(&profile.task_id),
                    "power segment end must be greater than its start",
                );
            }
            if segment.start_offset_ms < expected_start {
                violation(
                    &mut violations,
                    PowerConstraintViolationCode::SegmentOverlap,
                    Some(&profile.task_id),
                    format!(
                        "segment starts at {} before previous end {}",
                        segment.start_offset_ms, expected_start
                    ),
                );
            } else if segment.start_offset_ms > expected_start {
                violation(
                    &mut violations,
                    PowerConstraintViolationCode::SegmentGap,
                    Some(&profile.task_id),
                    format!(
                        "segment starts at {} after previous end {}",
                        segment.start_offset_ms, expected_start
                    ),
                );
            }
            if segment.end_offset_ms > duration_ms {
                violation(
                    &mut violations,
                    PowerConstraintViolationCode::SegmentBeyondTaskDuration,
                    Some(&profile.task_id),
                    format!(
                        "segment end {} exceeds task duration {}",
                        segment.end_offset_ms, duration_ms
                    ),
                );
            }
            expected_start = expected_start.max(segment.end_offset_ms);
        }
        if profile.segments[0].start_offset_ms != 0 {
            violation(
                &mut violations,
                PowerConstraintViolationCode::SegmentDoesNotStartAtZero,
                Some(&profile.task_id),
                "first power segment must begin at task offset zero",
            );
        }
        if expected_start != duration_ms {
            violation(
                &mut violations,
                PowerConstraintViolationCode::SegmentDoesNotCoverTask,
                Some(&profile.task_id),
                format!(
                    "profile ends at {} but task duration is {}",
                    expected_start, duration_ms
                ),
            );
        }
    }

    for task in &input.plan.tasks {
        if !seen.contains(task.task_id.as_str()) {
            violation(
                &mut violations,
                PowerConstraintViolationCode::MissingTaskProfile,
                Some(&task.task_id),
                "task has no power profile; use an explicit zero-watt segment for manual/no-load work",
            );
        }
    }

    sort_violations(&mut violations);
    violations
}

#[derive(Clone, Copy, Debug)]
struct PowerEvent {
    at_ms: u64,
    delta_w: i128,
}

/// Independently verify a schedule against task power traces and a declared
/// household envelope. Aggregates simultaneous real-power demand, includes
/// the declared background load, and preserves the longest contiguous period
/// above the continuous limit.
pub fn validate_power_envelope(
    input: &PowerConstrainedPlanV1,
    schedule: &KitchenScheduleReceiptV1,
) -> PowerValidationReportV1 {
    let mut violations = validate_inputs(input);
    let schedule_report = validate_schedule(&input.plan, schedule);
    if !schedule_report.is_valid() {
        violation(
            &mut violations,
            PowerConstraintViolationCode::ScheduleInvalid,
            None,
            format!(
                "schedule has {} independent validation violation(s)",
                schedule_report.violations.len()
            ),
        );
    }

    let mut evidence_classes: Vec<_> = input
        .task_profiles
        .iter()
        .map(|profile| profile.evidence_class)
        .collect();
    evidence_classes.sort();
    evidence_classes.dedup();
    let all_profiles_labelled_measured = !input.task_profiles.is_empty()
        && input
            .task_profiles
            .iter()
            .all(|profile| profile.evidence_class == PowerProfileEvidenceClassV1::Measured);

    let mut report = PowerValidationReportV1 {
        schema_version: 1,
        plan_id: input.plan.plan_id.clone(),
        peak_real_power_w: None,
        longest_surge_ms: 0,
        all_profiles_labelled_measured,
        evidence_classes,
        violations,
    };
    if !report.violations.is_empty() {
        return report;
    }

    let profiles: BTreeMap<&str, &TaskPowerProfileV1> = input
        .task_profiles
        .iter()
        .map(|profile| (profile.task_id.as_str(), profile))
        .collect();

    let mut events = Vec::new();
    let makespan_ms = schedule.makespan_ms;
    if makespan_ms > 0 {
        events.push(PowerEvent {
            at_ms: 0,
            delta_w: i128::from(input.envelope.background_real_power_w),
        });
        events.push(PowerEvent {
            at_ms: makespan_ms,
            delta_w: -i128::from(input.envelope.background_real_power_w),
        });
    }

    for scheduled in &schedule.tasks {
        let Some(profile) = profiles.get(scheduled.task_id.as_str()).copied() else {
            continue;
        };
        for segment in &profile.segments {
            let Some(start_ms) = scheduled.start_ms.checked_add(segment.start_offset_ms) else {
                violation(
                    &mut report.violations,
                    PowerConstraintViolationCode::TimeOverflow,
                    Some(&scheduled.task_id),
                    "absolute power-segment start overflows milliseconds",
                );
                continue;
            };
            let Some(end_ms) = scheduled.start_ms.checked_add(segment.end_offset_ms) else {
                violation(
                    &mut report.violations,
                    PowerConstraintViolationCode::TimeOverflow,
                    Some(&scheduled.task_id),
                    "absolute power-segment end overflows milliseconds",
                );
                continue;
            };
            if segment.real_power_w == 0 {
                continue;
            }
            events.push(PowerEvent {
                at_ms: start_ms,
                delta_w: i128::from(segment.real_power_w),
            });
            events.push(PowerEvent {
                at_ms: end_ms,
                delta_w: -i128::from(segment.real_power_w),
            });
        }
    }

    if !report.violations.is_empty() {
        sort_violations(&mut report.violations);
        return report;
    }

    events.sort_by_key(|event| event.at_ms);
    let mut current_power_w = 0_i128;
    let mut previous_time: Option<u64> = None;
    let mut surge_run_ms = 0_u64;
    let mut peak_exceeded_reported = false;
    let mut duration_exceeded_reported = false;
    let mut peak_power_w = 0_u64;

    let mut index = 0;
    while index < events.len() {
        let event_time = events[index].at_ms;
        if let Some(previous) = previous_time
            && event_time > previous
        {
            let interval_ms = event_time - previous;
            let nonnegative_power = current_power_w.max(0);
            let Ok(power_w) = u64::try_from(nonnegative_power) else {
                violation(
                    &mut report.violations,
                    PowerConstraintViolationCode::PowerArithmeticOverflow,
                    None,
                    "aggregate real power exceeds the u64 watt representation",
                );
                break;
            };
            peak_power_w = peak_power_w.max(power_w);

            if power_w > input.envelope.surge_real_power_limit_w && !peak_exceeded_reported {
                violation(
                    &mut report.violations,
                    PowerConstraintViolationCode::PeakPowerExceedsSurgeLimit,
                    None,
                    format!(
                        "aggregate real power {} W exceeds declared surge limit {} W",
                        power_w, input.envelope.surge_real_power_limit_w
                    ),
                );
                peak_exceeded_reported = true;
            }

            if power_w > input.envelope.continuous_real_power_limit_w {
                surge_run_ms = surge_run_ms.saturating_add(interval_ms);
                report.longest_surge_ms = report.longest_surge_ms.max(surge_run_ms);
                if surge_run_ms > input.envelope.max_surge_duration_ms
                    && !duration_exceeded_reported
                {
                    violation(
                        &mut report.violations,
                        PowerConstraintViolationCode::SurgeDurationExceedsLimit,
                        None,
                        format!(
                            "aggregate power stayed above continuous limit for {} ms; maximum permitted is {} ms",
                            surge_run_ms, input.envelope.max_surge_duration_ms
                        ),
                    );
                    duration_exceeded_reported = true;
                }
            } else {
                surge_run_ms = 0;
            }
        }

        let mut grouped_delta = 0_i128;
        while index < events.len() && events[index].at_ms == event_time {
            let Some(next_delta) = grouped_delta.checked_add(events[index].delta_w) else {
                violation(
                    &mut report.violations,
                    PowerConstraintViolationCode::PowerArithmeticOverflow,
                    None,
                    "simultaneous power-event arithmetic overflowed",
                );
                sort_violations(&mut report.violations);
                return report;
            };
            grouped_delta = next_delta;
            index += 1;
        }
        let Some(next_power) = current_power_w.checked_add(grouped_delta) else {
            violation(
                &mut report.violations,
                PowerConstraintViolationCode::PowerArithmeticOverflow,
                None,
                "aggregate power arithmetic overflowed",
            );
            sort_violations(&mut report.violations);
            return report;
        };
        current_power_w = next_power;
        previous_time = Some(event_time);
    }

    report.peak_real_power_w = Some(peak_power_w);
    sort_violations(&mut report.violations);
    report
}

/// Try the normal deterministic parallel schedule first. If its overlapping
/// loads exceed the declared envelope, try the existing serial topological
/// baseline. The serial fallback preserves all original dependency/deadline
/// constraints and is independently checked again for power.
pub fn build_power_constrained_schedule(
    input: &PowerConstrainedPlanV1,
) -> Result<PowerScheduleReceiptV1, PowerScheduleErrorV1> {
    let input_violations = validate_inputs(input);
    if !input_violations.is_empty() {
        return Err(PowerScheduleErrorV1::InvalidInput(input_violations));
    }

    let mut parallel_schedule_error = None;
    let mut parallel_power_violations = Vec::new();
    match build_schedule(&input.plan) {
        Ok(schedule) => {
            let report = validate_power_envelope(input, &schedule);
            if report.is_valid() {
                return Ok(PowerScheduleReceiptV1 {
                    schema_version: 1,
                    schedule,
                    power_validation: report,
                    strategy: PowerScheduleStrategyV1::ParallelEarliestGap,
                });
            }
            parallel_power_violations = report.violations;
        }
        Err(error) => parallel_schedule_error = Some(format!("{error:?}")),
    }

    let mut serial_schedule_error = None;
    let mut serial_power_violations = Vec::new();
    match build_serial_baseline(&input.plan) {
        Ok(schedule) => {
            let report = validate_power_envelope(input, &schedule);
            if report.is_valid() {
                return Ok(PowerScheduleReceiptV1 {
                    schema_version: 1,
                    schedule,
                    power_validation: report,
                    strategy: PowerScheduleStrategyV1::SerialFallback,
                });
            }
            serial_power_violations = report.violations;
        }
        Err(error) => serial_schedule_error = Some(format!("{error:?}")),
    }

    Err(PowerScheduleErrorV1::NoScheduleFoundByHeuristics {
        parallel_schedule_error,
        parallel_power_violations,
        serial_schedule_error,
        serial_power_violations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KITCHEN_PLAN_SCHEMA_VERSION, KitchenTaskV1};

    fn task(id: &str, duration_ms: u64, resource_id: &str) -> KitchenTaskV1 {
        KitchenTaskV1 {
            task_id: id.to_owned(),
            duration_ms,
            resource_ids: vec![resource_id.to_owned()],
            dependencies: vec![],
            earliest_start_ms: None,
            latest_finish_ms: None,
        }
    }

    fn profile(id: &str, duration_ms: u64, demand_w: u64) -> TaskPowerProfileV1 {
        TaskPowerProfileV1 {
            task_id: id.to_owned(),
            source_ref: format!("fixture:power:{id}"),
            evidence_class: PowerProfileEvidenceClassV1::Simulated,
            segments: vec![PowerSegmentV1 {
                start_offset_ms: 0,
                end_offset_ms: duration_ms,
                real_power_w: demand_w,
            }],
        }
    }

    fn input(
        tasks: Vec<KitchenTaskV1>,
        envelope: PowerEnvelopeV1,
        profiles: Vec<TaskPowerProfileV1>,
    ) -> PowerConstrainedPlanV1 {
        PowerConstrainedPlanV1 {
            schema_version: POWER_CONSTRAINED_PLAN_SCHEMA_VERSION,
            plan: KitchenPlanV1 {
                schema_version: KITCHEN_PLAN_SCHEMA_VERSION,
                plan_id: "power-test-meal-v1".to_owned(),
                tasks,
            },
            envelope,
            task_profiles: profiles,
        }
    }

    fn standard_envelope() -> PowerEnvelopeV1 {
        PowerEnvelopeV1 {
            continuous_real_power_limit_w: 1_000,
            surge_real_power_limit_w: 1_500,
            max_surge_duration_ms: 5,
            background_real_power_w: 0,
        }
    }

    #[test]
    fn parallel_overload_uses_serial_fallback() {
        let input = input(
            vec![
                task("processor", 10, "processor"),
                task("cooker", 10, "cooker"),
            ],
            standard_envelope(),
            vec![profile("processor", 10, 700), profile("cooker", 10, 600)],
        );
        let receipt = build_power_constrained_schedule(&input).unwrap();
        assert_eq!(receipt.strategy, PowerScheduleStrategyV1::SerialFallback);
        assert!(receipt.power_validation.is_valid());
        assert_eq!(receipt.power_validation.peak_real_power_w, Some(700));
        assert_eq!(receipt.schedule.makespan_ms, 20);
    }

    #[test]
    fn one_task_above_surge_limit_is_not_scheduled() {
        let input = input(
            vec![task("heater", 10, "heater")],
            standard_envelope(),
            vec![profile("heater", 10, 1_600)],
        );
        assert!(matches!(
            build_power_constrained_schedule(&input),
            Err(PowerScheduleErrorV1::NoScheduleFoundByHeuristics { .. })
        ));
    }

    #[test]
    fn missing_power_profile_fails_closed() {
        let input = input(
            vec![task("processor", 10, "processor")],
            standard_envelope(),
            vec![],
        );
        assert!(matches!(
            build_power_constrained_schedule(&input),
            Err(PowerScheduleErrorV1::InvalidInput(violations))
                if violations.iter().any(|v| v.code == PowerConstraintViolationCode::MissingTaskProfile)
        ));
    }

    #[test]
    fn background_household_load_is_included() {
        let mut envelope = standard_envelope();
        envelope.background_real_power_w = 400;
        envelope.surge_real_power_limit_w = 1_000;
        let input = input(
            vec![task("heater", 10, "heater")],
            envelope,
            vec![profile("heater", 10, 700)],
        );
        assert!(matches!(
            build_power_constrained_schedule(&input),
            Err(PowerScheduleErrorV1::NoScheduleFoundByHeuristics { .. })
        ));
    }

    #[test]
    fn short_declared_surge_is_measured_separately_from_continuous_load() {
        let input = input(
            vec![task("heater", 100, "heater")],
            standard_envelope(),
            vec![TaskPowerProfileV1 {
                task_id: "heater".to_owned(),
                source_ref: "fixture:startup-surge".to_owned(),
                evidence_class: PowerProfileEvidenceClassV1::Simulated,
                segments: vec![
                    PowerSegmentV1 {
                        start_offset_ms: 0,
                        end_offset_ms: 5,
                        real_power_w: 1_200,
                    },
                    PowerSegmentV1 {
                        start_offset_ms: 5,
                        end_offset_ms: 100,
                        real_power_w: 700,
                    },
                ],
            }],
        );
        let receipt = build_power_constrained_schedule(&input).unwrap();
        assert_eq!(
            receipt.strategy,
            PowerScheduleStrategyV1::ParallelEarliestGap
        );
        assert_eq!(receipt.power_validation.peak_real_power_w, Some(1_200));
        assert_eq!(receipt.power_validation.longest_surge_ms, 5);
    }

    #[test]
    fn profile_gap_is_rejected() {
        let input = input(
            vec![task("heater", 10, "heater")],
            standard_envelope(),
            vec![TaskPowerProfileV1 {
                task_id: "heater".to_owned(),
                source_ref: "fixture:gap".to_owned(),
                evidence_class: PowerProfileEvidenceClassV1::Simulated,
                segments: vec![
                    PowerSegmentV1 {
                        start_offset_ms: 0,
                        end_offset_ms: 4,
                        real_power_w: 500,
                    },
                    PowerSegmentV1 {
                        start_offset_ms: 5,
                        end_offset_ms: 10,
                        real_power_w: 500,
                    },
                ],
            }],
        );
        assert!(matches!(
            build_power_constrained_schedule(&input),
            Err(PowerScheduleErrorV1::InvalidInput(violations))
                if violations.iter().any(|v| v.code == PowerConstraintViolationCode::SegmentGap)
        ));
    }

    #[test]
    fn all_profile_classes_are_preserved_and_not_promoted() {
        let mut power_profile = profile("manual", 10, 0);
        power_profile.evidence_class = PowerProfileEvidenceClassV1::Declared;
        let input = input(
            vec![task("manual", 10, "counter")],
            standard_envelope(),
            vec![power_profile],
        );
        let schedule = build_schedule(&input.plan).unwrap();
        let report = validate_power_envelope(&input, &schedule);
        assert!(report.is_valid());
        assert!(!report.all_profiles_labelled_measured);
        assert_eq!(
            report.evidence_classes,
            vec![PowerProfileEvidenceClassV1::Declared]
        );
    }
}
