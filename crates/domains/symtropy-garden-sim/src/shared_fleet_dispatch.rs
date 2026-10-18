// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-gated, deterministic dispatch baseline for shared agricultural robots.
//!
//! This is a schedule-decision benchmark, not a road router, MAPF solver, or
//! authorization to dispatch physical robots. Missing/unvalidated travel data
//! remains unknown; geodesic distance is never converted into travel minutes.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SHARED_FLEET_DISPATCH_SCHEMA_VERSION_V1: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchRobotReadinessV1 {
    Ready,
    Maintenance,
    PartsUnavailable,
    OperatorUnavailable,
    Retired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchTravelMetricV1 {
    /// Great-circle distance only. It is never a schedulable travel-time value.
    GeodesicDistance,
    /// A duration produced by a versioned route graph or friction-surface model.
    ModeledRouteTime,
    /// A duration observed on a recorded historical trip. This is evidence,
    /// not a forecast for a future dispatch.
    ObservedTripTime,
    /// Explicitly unknown metric.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchTravelStateV1 {
    Available,
    NoRoute,
    OutsideCoverage,
    Unknown,
    Stale,
    Conflicting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchTravelCoverageV1 {
    NotApplicable,
    Validated,
    PartiallyValidated,
    Unvalidated,
    OutsideCoverage,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchTravelModeV1 {
    StraightLine,
    Walking,
    Bicycle,
    Motorcycle,
    MotorVehicle,
    SelfPropelledRobot,
    Multimodal,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetRequestV1 {
    pub request_id: String,
    pub location_ref: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub season_ref: String,
    pub task_code: String,
    pub requested_at_minute: u64,
    pub useful_window_deadline_minute: u64,
    /// None means task duration is not evidenced; it is never interpreted as 0.
    pub task_duration_minutes: Option<u64>,
    pub required_service_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetRobotV1 {
    pub robot_id: String,
    pub start_location_ref: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub travel_mode: DispatchTravelModeV1,
    pub readiness: DispatchRobotReadinessV1,
    pub available_from_minute: u64,
    pub available_until_minute: u64,
    pub maximum_service_units_per_job: u64,
    /// Explicit task capability set; duplicate/empty entries are rejected.
    pub supported_task_codes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetTravelLegV1 {
    pub robot_id: String,
    pub from_location_ref: String,
    pub to_location_ref: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub season_ref: String,
    pub metric: DispatchTravelMetricV1,
    pub state: DispatchTravelStateV1,
    pub coverage: DispatchTravelCoverageV1,
    pub travel_minutes: Option<u64>,
    pub travel_mode: DispatchTravelModeV1,
    pub method_id: Option<String>,
    pub method_version: Option<String>,
    /// Reference to the canonical route/trip evidence in the upstream spatial system.
    /// A reference or digest shape is not authentication.
    pub evidence_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetDispatchInputV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub season_ref: String,
    pub requests: Vec<SharedFleetRequestV1>,
    pub robots: Vec<SharedFleetRobotV1>,
    pub travel_legs: Vec<SharedFleetTravelLegV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedFleetDispatchDispositionV1 {
    ScheduledWithinUsefulWindow,
    ScheduledLate,
    UnknownTravelEvidence,
    UnknownTaskDuration,
    NoCompatibleRobot,
    AllCompatibleRobotsUnavailable,
    NoFeasibleRoute,
    CapacityUnavailable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetDispatchAssignmentV1 {
    pub request_id: String,
    pub disposition: SharedFleetDispatchDispositionV1,
    pub robot_id: Option<String>,
    /// A forecast schedule, not proof that a booking or physical task occurred.
    pub departure_minute: Option<u64>,
    pub service_start_minute: Option<u64>,
    pub forecast_completion_minute: Option<u64>,
    pub travel_minutes: Option<u64>,
    /// Only explicitly modeled route-time results can be scheduled; the metric
    /// type is retained for every scheduled travel leg.
    pub travel_metric: Option<DispatchTravelMetricV1>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetDispatchReportV1 {
    pub schema_version: u16,
    pub scenario_id: String,
    pub scope_ref: String,
    pub period_ref: String,
    pub season_ref: String,
    pub request_count: usize,
    pub scheduled_count: usize,
    pub scheduled_within_window_count: usize,
    pub scheduled_late_count: usize,
    pub unknown_travel_evidence_count: usize,
    pub unknown_task_duration_count: usize,
    pub no_compatible_robot_count: usize,
    pub all_compatible_robots_unavailable_count: usize,
    pub no_feasible_route_count: usize,
    pub capacity_unavailable_count: usize,
    /// Never combines modeled-route and observed-trip minutes.
    pub total_scheduled_modeled_route_minutes: u64,
    /// Results are sorted by request_id, independent of input row order.
    pub assignments: Vec<SharedFleetDispatchAssignmentV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedFleetDispatchBlockerCodeV1 {
    UnsupportedSchema,
    EmptyIdentity,
    EmptyRequests,
    EmptyRobots,
    DuplicateRequestId,
    DuplicateRobotId,
    DuplicateTravelLeg,
    UnknownRobotReference,
    UnknownLocationReference,
    ScopeMismatch,
    PeriodMismatch,
    SeasonMismatch,
    InvalidRequestWindow,
    InvalidServiceQuantity,
    InvalidTaskDuration,
    InvalidRobotAvailability,
    InvalidRobotCapacity,
    InvalidRobotCapabilities,
    InvalidTravelMetricMode,
    InvalidTravelCoverage,
    InvalidTravelStateValue,
    MissingTravelMethod,
    MissingTravelEvidence,
    InvalidUnknownTravelRecord,
    ArithmeticOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SharedFleetDispatchBlockerV1 {
    pub code: SharedFleetDispatchBlockerCodeV1,
    pub record_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RouteKey {
    robot_id: String,
    from_location_ref: String,
    to_location_ref: String,
}

impl Ord for RouteKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (
            &self.robot_id,
            &self.from_location_ref,
            &self.to_location_ref,
        )
            .cmp(&(
                &other.robot_id,
                &other.from_location_ref,
                &other.to_location_ref,
            ))
    }
}

impl PartialOrd for RouteKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
struct RobotScheduleState<'a> {
    robot: &'a SharedFleetRobotV1,
    available_at_minute: u64,
    current_location_ref: String,
}

#[derive(Clone, Debug)]
struct ScheduleCandidate {
    robot_id: String,
    departure_minute: u64,
    service_start_minute: u64,
    completion_minute: u64,
    travel_minutes: u64,
    travel_metric: DispatchTravelMetricV1,
    within_useful_window: bool,
}

/// Validate the full scenario, then produce a deterministic earliest-deadline
/// first schedule. This greedy baseline is not globally optimal and is not a
/// multi-agent path-finding/corridor-conflict solver.
pub fn plan_shared_fleet_dispatch_v1(
    input: &SharedFleetDispatchInputV1,
) -> Result<SharedFleetDispatchReportV1, Vec<SharedFleetDispatchBlockerV1>> {
    let mut blockers = validate_dispatch_input(input);
    if !blockers.is_empty() {
        blockers.sort_by_key(|blocker| (blocker.code, blocker.record_ref.clone()));
        blockers.dedup();
        return Err(blockers);
    }

    let mut routes: BTreeMap<RouteKey, &SharedFleetTravelLegV1> = BTreeMap::new();
    for leg in &input.travel_legs {
        routes.insert(
            RouteKey {
                robot_id: leg.robot_id.clone(),
                from_location_ref: leg.from_location_ref.clone(),
                to_location_ref: leg.to_location_ref.clone(),
            },
            leg,
        );
    }

    let mut robot_states: BTreeMap<String, RobotScheduleState<'_>> = input
        .robots
        .iter()
        .map(|robot| {
            (
                robot.robot_id.clone(),
                RobotScheduleState {
                    robot,
                    available_at_minute: robot.available_from_minute,
                    current_location_ref: robot.start_location_ref.clone(),
                },
            )
        })
        .collect();

    let mut schedule_order: Vec<&SharedFleetRequestV1> = input.requests.iter().collect();
    schedule_order.sort_by_key(|request| (
        request.useful_window_deadline_minute,
        request.requested_at_minute,
        request.request_id.clone(),
    ));

    let mut assignments = Vec::with_capacity(input.requests.len());
    let mut total_modeled_route_minutes = 0_u64;

    for request in schedule_order {
        let compatible_ids: Vec<String> = robot_states
            .iter()
            .filter(|(_, state)| {
                state.robot.maximum_service_units_per_job >= request.required_service_units
                    && state
                        .robot
                        .supported_task_codes
                        .contains(&request.task_code)
            })
            .map(|(id, _)| id.clone())
            .collect();

        if compatible_ids.is_empty() {
            assignments.push(unassigned(
                request,
                SharedFleetDispatchDispositionV1::NoCompatibleRobot,
            ));
            continue;
        }

        let ready_ids: Vec<String> = compatible_ids
            .iter()
            .filter(|id| {
                robot_states
                    .get(*id)
                    .is_some_and(|state| state.robot.readiness == DispatchRobotReadinessV1::Ready)
            })
            .cloned()
            .collect();

        if ready_ids.is_empty() {
            assignments.push(unassigned(
                request,
                SharedFleetDispatchDispositionV1::AllCompatibleRobotsUnavailable,
            ));
            continue;
        }

        let Some(task_minutes) = request.task_duration_minutes else {
            assignments.push(unassigned(
                request,
                SharedFleetDispatchDispositionV1::UnknownTaskDuration,
            ));
            continue;
        };

        let mut candidates = Vec::new();
        let mut saw_unknown_route = false;
        let mut saw_confirmed_no_route = false;
        let mut saw_route_that_did_not_fit = false;

        for robot_id in &ready_ids {
            let state = robot_states
                .get(robot_id)
                .expect("ready robot ID was read from the same map");
            let key = RouteKey {
                robot_id: robot_id.clone(),
                from_location_ref: state.current_location_ref.clone(),
                to_location_ref: request.location_ref.clone(),
            };
            let Some(leg) = routes.get(&key) else {
                saw_unknown_route = true;
                continue;
            };

            match schedulable_travel_minutes(leg) {
                Some((travel_minutes, metric)) => {
                    let departure_minute =
                        state.available_at_minute.max(request.requested_at_minute);
                    let Some(service_start_minute) =
                        departure_minute.checked_add(travel_minutes)
                    else {
                        return Err(vec![blocker(
                            SharedFleetDispatchBlockerCodeV1::ArithmeticOverflow,
                            &request.request_id,
                        )]);
                    };
                    let Some(completion_minute) = service_start_minute.checked_add(task_minutes)
                    else {
                        return Err(vec![blocker(
                            SharedFleetDispatchBlockerCodeV1::ArithmeticOverflow,
                            &request.request_id,
                        )]);
                    };

                    if completion_minute > state.robot.available_until_minute {
                        saw_route_that_did_not_fit = true;
                        continue;
                    }
                    candidates.push(ScheduleCandidate {
                        robot_id: robot_id.clone(),
                        departure_minute,
                        service_start_minute,
                        completion_minute,
                        travel_minutes,
                        travel_metric: metric,
                        within_useful_window: completion_minute
                            <= request.useful_window_deadline_minute,
                    });
                }
                None => match leg.state {
                    DispatchTravelStateV1::NoRoute => saw_confirmed_no_route = true,
                    DispatchTravelStateV1::OutsideCoverage
                    | DispatchTravelStateV1::Unknown
                    | DispatchTravelStateV1::Stale
                    | DispatchTravelStateV1::Conflicting => saw_unknown_route = true,
                    DispatchTravelStateV1::Available => saw_unknown_route = true,
                },
            }
        }

        candidates.sort_by_key(|candidate| (
            !candidate.within_useful_window,
            candidate.completion_minute,
            candidate.travel_minutes,
            candidate.robot_id.clone(),
        ));

        let Some(candidate) = candidates.into_iter().next() else {
            let disposition = if saw_unknown_route {
                SharedFleetDispatchDispositionV1::UnknownTravelEvidence
            } else if saw_route_that_did_not_fit {
                SharedFleetDispatchDispositionV1::CapacityUnavailable
            } else if saw_confirmed_no_route {
                SharedFleetDispatchDispositionV1::NoFeasibleRoute
            } else {
                SharedFleetDispatchDispositionV1::UnknownTravelEvidence
            };
            assignments.push(unassigned(request, disposition));
            continue;
        };

        match candidate.travel_metric {
            DispatchTravelMetricV1::ModeledRouteTime => {
                total_modeled_route_minutes = total_modeled_route_minutes
                    .checked_add(candidate.travel_minutes)
                    .ok_or_else(|| {
                        vec![blocker(
                            SharedFleetDispatchBlockerCodeV1::ArithmeticOverflow,
                            &request.request_id,
                        )]
                    })?;
            }
            DispatchTravelMetricV1::ObservedTripTime
            | DispatchTravelMetricV1::GeodesicDistance
            | DispatchTravelMetricV1::Unknown => {
                // schedulable_travel_minutes excludes these metrics.
                return Err(vec![blocker(
                    SharedFleetDispatchBlockerCodeV1::InvalidTravelMetricMode,
                    &request.request_id,
                )]);
            }
        }

        let disposition = if candidate.within_useful_window {
            SharedFleetDispatchDispositionV1::ScheduledWithinUsefulWindow
        } else {
            SharedFleetDispatchDispositionV1::ScheduledLate
        };

        let state = robot_states
            .get_mut(&candidate.robot_id)
            .expect("a selected candidate must reference a known robot");
        state.available_at_minute = candidate.completion_minute;
        state.current_location_ref = request.location_ref.clone();

        assignments.push(SharedFleetDispatchAssignmentV1 {
            request_id: request.request_id.clone(),
            disposition,
            robot_id: Some(candidate.robot_id),
            departure_minute: Some(candidate.departure_minute),
            service_start_minute: Some(candidate.service_start_minute),
            forecast_completion_minute: Some(candidate.completion_minute),
            travel_minutes: Some(candidate.travel_minutes),
            travel_metric: Some(candidate.travel_metric),
        });
    }

    assignments.sort_by_key(|assignment| assignment.request_id.clone());

    let scheduled_within_window_count = count_disposition(
        &assignments,
        SharedFleetDispatchDispositionV1::ScheduledWithinUsefulWindow,
    );
    let scheduled_late_count = count_disposition(
        &assignments,
        SharedFleetDispatchDispositionV1::ScheduledLate,
    );

    Ok(SharedFleetDispatchReportV1 {
        schema_version: SHARED_FLEET_DISPATCH_SCHEMA_VERSION_V1,
        scenario_id: input.scenario_id.clone(),
        scope_ref: input.scope_ref.clone(),
        period_ref: input.period_ref.clone(),
        season_ref: input.season_ref.clone(),
        request_count: assignments.len(),
        scheduled_count: scheduled_within_window_count + scheduled_late_count,
        scheduled_within_window_count,
        scheduled_late_count,
        unknown_travel_evidence_count: count_disposition(
            &assignments,
            SharedFleetDispatchDispositionV1::UnknownTravelEvidence,
        ),
        unknown_task_duration_count: count_disposition(
            &assignments,
            SharedFleetDispatchDispositionV1::UnknownTaskDuration,
        ),
        no_compatible_robot_count: count_disposition(
            &assignments,
            SharedFleetDispatchDispositionV1::NoCompatibleRobot,
        ),
        all_compatible_robots_unavailable_count: count_disposition(
            &assignments,
            SharedFleetDispatchDispositionV1::AllCompatibleRobotsUnavailable,
        ),
        no_feasible_route_count: count_disposition(
            &assignments,
            SharedFleetDispatchDispositionV1::NoFeasibleRoute,
        ),
        capacity_unavailable_count: count_disposition(
            &assignments,
            SharedFleetDispatchDispositionV1::CapacityUnavailable,
        ),
        total_scheduled_modeled_route_minutes: total_modeled_route_minutes,
        assignments,
    })
}

fn count_disposition(
    assignments: &[SharedFleetDispatchAssignmentV1],
    disposition: SharedFleetDispatchDispositionV1,
) -> usize {
    assignments
        .iter()
        .filter(|assignment| assignment.disposition == disposition)
        .count()
}

fn unassigned(
    request: &SharedFleetRequestV1,
    disposition: SharedFleetDispatchDispositionV1,
) -> SharedFleetDispatchAssignmentV1 {
    SharedFleetDispatchAssignmentV1 {
        request_id: request.request_id.clone(),
        disposition,
        robot_id: None,
        departure_minute: None,
        service_start_minute: None,
        forecast_completion_minute: None,
        travel_minutes: None,
        travel_metric: None,
    }
}

fn schedulable_travel_minutes(
    leg: &SharedFleetTravelLegV1,
) -> Option<(u64, DispatchTravelMetricV1)> {
    if leg.state != DispatchTravelStateV1::Available
        || !matches!(
            leg.coverage,
            DispatchTravelCoverageV1::Validated | DispatchTravelCoverageV1::PartiallyValidated
        )
    {
        return None;
    }

    match (leg.metric, leg.travel_minutes) {
        (DispatchTravelMetricV1::ModeledRouteTime, Some(minutes)) => {
            Some((minutes, leg.metric))
        }
        _ => None,
    }
}

fn validate_dispatch_input(
    input: &SharedFleetDispatchInputV1,
) -> Vec<SharedFleetDispatchBlockerV1> {
    let mut blockers = Vec::new();
    let mut request_ids = BTreeSet::new();
    let mut robot_ids = BTreeSet::new();
    let mut route_keys = BTreeSet::new();
    let mut locations = BTreeSet::new();

    if input.schema_version != SHARED_FLEET_DISPATCH_SCHEMA_VERSION_V1 {
        blockers.push(blocker(
            SharedFleetDispatchBlockerCodeV1::UnsupportedSchema,
            &input.scenario_id,
        ));
    }
    for (field, value) in [
        ("scenario_id", input.scenario_id.as_str()),
        ("scope_ref", input.scope_ref.as_str()),
        ("period_ref", input.period_ref.as_str()),
        ("season_ref", input.season_ref.as_str()),
    ] {
        if value.trim().is_empty() {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::EmptyIdentity,
                field,
            ));
        }
    }
    if input.requests.is_empty() {
        blockers.push(blocker(
            SharedFleetDispatchBlockerCodeV1::EmptyRequests,
            &input.scenario_id,
        ));
    }
    if input.robots.is_empty() {
        blockers.push(blocker(
            SharedFleetDispatchBlockerCodeV1::EmptyRobots,
            &input.scenario_id,
        ));
    }

    for request in &input.requests {
        locations.insert(request.location_ref.as_str());
        if request.request_id.trim().is_empty()
            || request.location_ref.trim().is_empty()
            || request.task_code.trim().is_empty()
        {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::EmptyIdentity,
                &request.request_id,
            ));
        }
        if !request_ids.insert(request.request_id.as_str()) {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::DuplicateRequestId,
                &request.request_id,
            ));
        }
        if request.scope_ref != input.scope_ref {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::ScopeMismatch,
                &request.request_id,
            ));
        }
        if request.period_ref != input.period_ref {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::PeriodMismatch,
                &request.request_id,
            ));
        }
        if request.season_ref != input.season_ref {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::SeasonMismatch,
                &request.request_id,
            ));
        }
        if request.useful_window_deadline_minute < request.requested_at_minute {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidRequestWindow,
                &request.request_id,
            ));
        }
        if request.required_service_units == 0 {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidServiceQuantity,
                &request.request_id,
            ));
        }
        if request.task_duration_minutes == Some(0) {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidTaskDuration,
                &request.request_id,
            ));
        }
    }

    for robot in &input.robots {
        locations.insert(robot.start_location_ref.as_str());
        if robot.robot_id.trim().is_empty() || robot.start_location_ref.trim().is_empty() {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::EmptyIdentity,
                &robot.robot_id,
            ));
        }
        if !robot_ids.insert(robot.robot_id.as_str()) {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::DuplicateRobotId,
                &robot.robot_id,
            ));
        }
        if robot.scope_ref != input.scope_ref {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::ScopeMismatch,
                &robot.robot_id,
            ));
        }
        if robot.period_ref != input.period_ref {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::PeriodMismatch,
                &robot.robot_id,
            ));
        }
        if robot.available_until_minute <= robot.available_from_minute {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidRobotAvailability,
                &robot.robot_id,
            ));
        }
        if robot.maximum_service_units_per_job == 0 {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidRobotCapacity,
                &robot.robot_id,
            ));
        }
        let mut task_codes = BTreeSet::new();
        if robot.supported_task_codes.is_empty()
            || robot
                .supported_task_codes
                .iter()
                .any(|task| task.trim().is_empty() || !task_codes.insert(task.as_str()))
        {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidRobotCapabilities,
                &robot.robot_id,
            ));
        }
    }

    for leg in &input.travel_legs {
        if !robot_ids.contains(leg.robot_id.as_str()) {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::UnknownRobotReference,
                &leg.robot_id,
            ));
        }
        if !locations.contains(leg.from_location_ref.as_str())
            || !locations.contains(leg.to_location_ref.as_str())
        {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::UnknownLocationReference,
                &format!("{}:{}->{}", leg.robot_id, leg.from_location_ref, leg.to_location_ref),
            ));
        }
        if leg.scope_ref != input.scope_ref {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::ScopeMismatch,
                &leg.robot_id,
            ));
        }
        if leg.period_ref != input.period_ref {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::PeriodMismatch,
                &leg.robot_id,
            ));
        }
        if leg.season_ref != input.season_ref {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::SeasonMismatch,
                &leg.robot_id,
            ));
        }
        let key = RouteKey {
            robot_id: leg.robot_id.clone(),
            from_location_ref: leg.from_location_ref.clone(),
            to_location_ref: leg.to_location_ref.clone(),
        };
        if !route_keys.insert(key) {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::DuplicateTravelLeg,
                &format!(
                    "{}:{}->{}",
                    leg.robot_id, leg.from_location_ref, leg.to_location_ref
                ),
            ));
        }

        let robot_mode = input
            .robots
            .iter()
            .find(|robot| robot.robot_id == leg.robot_id)
            .map(|robot| robot.travel_mode)
            .unwrap_or(DispatchTravelModeV1::Unknown);
        if leg.metric == DispatchTravelMetricV1::GeodesicDistance {
            if leg.travel_mode != DispatchTravelModeV1::StraightLine
                || leg.coverage != DispatchTravelCoverageV1::NotApplicable
                || leg.travel_minutes.is_some()
                || matches!(
                    leg.state,
                    DispatchTravelStateV1::NoRoute | DispatchTravelStateV1::OutsideCoverage
                )
            {
                blockers.push(blocker(
                    SharedFleetDispatchBlockerCodeV1::InvalidTravelMetricMode,
                    &leg.robot_id,
                ));
            }
        } else if leg.metric == DispatchTravelMetricV1::Unknown {
            if leg.state != DispatchTravelStateV1::Unknown
                || leg.travel_mode != DispatchTravelModeV1::Unknown
                || leg.coverage != DispatchTravelCoverageV1::Unknown
                || leg.travel_minutes.is_some()
                || leg.method_id.is_some()
                || leg.method_version.is_some()
                || leg.evidence_ref.is_some()
            {
                blockers.push(blocker(
                    SharedFleetDispatchBlockerCodeV1::InvalidUnknownTravelRecord,
                    &leg.robot_id,
                ));
            }
        } else if leg.travel_mode == DispatchTravelModeV1::StraightLine
            || leg.travel_mode == DispatchTravelModeV1::Unknown
            || leg.travel_mode != robot_mode
            || leg.coverage == DispatchTravelCoverageV1::NotApplicable
        {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidTravelMetricMode,
                &leg.robot_id,
            ));
        }

        if (leg.state == DispatchTravelStateV1::OutsideCoverage)
            != (leg.coverage == DispatchTravelCoverageV1::OutsideCoverage)
        {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidTravelCoverage,
                &leg.robot_id,
            ));
        }
        if leg.state == DispatchTravelStateV1::NoRoute
            && !matches!(
                leg.coverage,
                DispatchTravelCoverageV1::Validated | DispatchTravelCoverageV1::PartiallyValidated
            )
        {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidTravelCoverage,
                &leg.robot_id,
            ));
        }
        if leg.state == DispatchTravelStateV1::Available {
            if leg.metric == DispatchTravelMetricV1::Unknown {
                blockers.push(blocker(
                    SharedFleetDispatchBlockerCodeV1::InvalidUnknownTravelRecord,
                    &leg.robot_id,
                ));
            } else if leg.metric != DispatchTravelMetricV1::GeodesicDistance {
                if leg.travel_minutes.is_none()
                    || matches!(
                        leg.coverage,
                        DispatchTravelCoverageV1::OutsideCoverage
                            | DispatchTravelCoverageV1::Unknown
                            | DispatchTravelCoverageV1::NotApplicable
                    )
                {
                    blockers.push(blocker(
                        SharedFleetDispatchBlockerCodeV1::InvalidTravelStateValue,
                        &leg.robot_id,
                    ));
                }
            }
        } else if leg.travel_minutes.is_some() {
            blockers.push(blocker(
                SharedFleetDispatchBlockerCodeV1::InvalidTravelStateValue,
                &leg.robot_id,
            ));
        }

        if leg.state != DispatchTravelStateV1::Unknown
            && leg.metric != DispatchTravelMetricV1::Unknown
        {
            for value in [leg.method_id.as_deref(), leg.method_version.as_deref()] {
                if value.is_none_or(|s| s.trim().is_empty()) {
                    blockers.push(blocker(
                        SharedFleetDispatchBlockerCodeV1::MissingTravelMethod,
                        &leg.robot_id,
                    ));
                    break;
                }
            }
            if leg
                .evidence_ref
                .as_deref()
                .is_none_or(|reference| reference.trim().is_empty())
            {
                blockers.push(blocker(
                    SharedFleetDispatchBlockerCodeV1::MissingTravelEvidence,
                    &leg.robot_id,
                ));
            }
        }
    }

    blockers
}

fn blocker(
    code: SharedFleetDispatchBlockerCodeV1,
    record_ref: &str,
) -> SharedFleetDispatchBlockerV1 {
    SharedFleetDispatchBlockerV1 {
        code,
        record_ref: record_ref.to_string(),
    }
}
