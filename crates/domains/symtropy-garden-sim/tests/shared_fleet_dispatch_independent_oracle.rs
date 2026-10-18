// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent synthetic oracle cases for evidence-gated shared-fleet dispatch.

use symtropy_garden_sim::shared_fleet_dispatch::{
    DispatchRobotReadinessV1, DispatchTravelCoverageV1, DispatchTravelLegV1,
    DispatchTravelMetricV1, DispatchTravelModeV1, DispatchTravelStateV1,
    SHARED_FLEET_DISPATCH_SCHEMA_VERSION_V1, SharedFleetDispatchAssignmentV1,
    SharedFleetDispatchBlockerCodeV1, SharedFleetDispatchDispositionV1,
    SharedFleetDispatchInputV1, SharedFleetDispatchReportV1, SharedFleetRequestV1,
    SharedFleetRobotV1, plan_shared_fleet_dispatch_v1,
};

fn request(id: &str, location: &str, deadline: u64, duration: Option<u64>) -> SharedFleetRequestV1 {
    SharedFleetRequestV1 {
        request_id: id.into(),
        location_ref: location.into(),
        scope_ref: "scope:synthetic-v1".into(),
        period_ref: "period:synthetic-v1".into(),
        season_ref: "season:synthetic-dry-v1".into(),
        task_code: "weeding".into(),
        requested_at_minute: 0,
        useful_window_deadline_minute: deadline,
        task_duration_minutes: duration,
        required_service_units: 1_000,
    }
}

fn robot(id: &str, start: &str, until: u64, readiness: DispatchRobotReadinessV1) -> SharedFleetRobotV1 {
    SharedFleetRobotV1 {
        robot_id: id.into(),
        start_location_ref: start.into(),
        scope_ref: "scope:synthetic-v1".into(),
        period_ref: "period:synthetic-v1".into(),
        travel_mode: DispatchTravelModeV1::SelfPropelledRobot,
        readiness,
        available_from_minute: 0,
        available_until_minute: until,
        maximum_service_units_per_job: 2_000,
        supported_task_codes: vec!["weeding".into()],
    }
}

fn route(
    robot_id: &str,
    from: &str,
    to: &str,
    minutes: u64,
) -> DispatchTravelLegV1 {
    DispatchTravelLegV1 {
        robot_id: robot_id.into(),
        from_location_ref: from.into(),
        to_location_ref: to.into(),
        scope_ref: "scope:synthetic-v1".into(),
        period_ref: "period:synthetic-v1".into(),
        season_ref: "season:synthetic-dry-v1".into(),
        metric: DispatchTravelMetricV1::ModeledRouteTime,
        state: DispatchTravelStateV1::Available,
        coverage: DispatchTravelCoverageV1::Validated,
        travel_minutes: Some(minutes),
        travel_mode: DispatchTravelModeV1::SelfPropelledRobot,
        method_id: Some("synthetic-route-model".into()),
        method_version: Some("fixture-v1".into()),
        evidence_ref: Some(format!("evidence:synthetic:{robot_id}:{from}:{to}")),
    }
}

fn no_route(robot_id: &str, from: &str, to: &str) -> DispatchTravelLegV1 {
    let mut leg = route(robot_id, from, to, 0);
    leg.state = DispatchTravelStateV1::NoRoute;
    leg.coverage = DispatchTravelCoverageV1::Validated;
    leg.travel_minutes = None;
    leg
}

fn input(
    requests: Vec<SharedFleetRequestV1>,
    robots: Vec<SharedFleetRobotV1>,
    travel_legs: Vec<DispatchTravelLegV1>,
) -> SharedFleetDispatchInputV1 {
    SharedFleetDispatchInputV1 {
        schema_version: SHARED_FLEET_DISPATCH_SCHEMA_VERSION_V1,
        scenario_id: "shared-fleet-synthetic-oracle-v1".into(),
        scope_ref: "scope:synthetic-v1".into(),
        period_ref: "period:synthetic-v1".into(),
        season_ref: "season:synthetic-dry-v1".into(),
        requests,
        robots,
        travel_legs,
    }
}

fn base_input() -> SharedFleetDispatchInputV1 {
    input(
        vec![
            request("request-a", "site-a", 20, Some(10)),
            request("request-b", "site-b", 13, Some(5)),
        ],
        vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
        vec![
            route("robot-1", "depot", "site-a", 5),
            route("robot-1", "depot", "site-b", 2),
            route("robot-1", "site-b", "site-a", 4),
            route("robot-1", "site-a", "site-b", 3),
        ],
    )
}

fn assignment(
    report: &SharedFleetDispatchReportV1,
    request_id: &str,
) -> &SharedFleetDispatchAssignmentV1 {
    report
        .assignments
        .iter()
        .find(|assignment| assignment.request_id == request_id)
        .expect("every request has exactly one disposition")
}

fn assert_blocked(input: &SharedFleetDispatchInputV1, expected: SharedFleetDispatchBlockerCodeV1) {
    let error = plan_shared_fleet_dispatch_v1(input).unwrap_err();
    assert!(
        error.iter().any(|blocker| blocker.code == expected),
        "expected blocker {expected:?}, got {:?}",
        error.iter().map(|blocker| blocker.code).collect::<Vec<_>>()
    );
}

#[test]
fn deadline_first_schedule_counts_on_time_and_late_separately() {
    let report = plan_shared_fleet_dispatch_v1(&base_input()).unwrap();
    assert_eq!(report.request_count, 2);
    assert_eq!(report.scheduled_count, 2);
    assert_eq!(report.scheduled_within_window_count, 1);
    assert_eq!(report.scheduled_late_count, 1);
    assert_eq!(report.total_scheduled_modeled_route_minutes, 6);

    let earliest_deadline = assignment(&report, "request-b");
    assert_eq!(
        earliest_deadline.disposition,
        SharedFleetDispatchDispositionV1::ScheduledWithinUsefulWindow
    );
    assert_eq!(earliest_deadline.travel_minutes, Some(2));
    assert_eq!(earliest_deadline.service_start_minute, Some(2));
    assert_eq!(earliest_deadline.forecast_completion_minute, Some(7));

    let later_request = assignment(&report, "request-a");
    assert_eq!(
        later_request.disposition,
        SharedFleetDispatchDispositionV1::ScheduledLate
    );
    assert_eq!(later_request.travel_minutes, Some(4));
    assert_eq!(later_request.forecast_completion_minute, Some(21));
}

#[test]
fn input_row_permutation_preserves_deterministic_report() {
    let original = base_input();
    let original_report = plan_shared_fleet_dispatch_v1(&original).unwrap();

    let mut permuted = original;
    permuted.requests.reverse();
    permuted.robots.reverse();
    permuted.travel_legs.reverse();

    assert_eq!(
        plan_shared_fleet_dispatch_v1(&permuted).unwrap(),
        original_report
    );
}

#[test]
fn geodesic_distance_cannot_be_relabelled_as_route_time() {
    let mut geodesic = route("robot-1", "depot", "site-a", 0);
    geodesic.metric = DispatchTravelMetricV1::GeodesicDistance;
    geodesic.travel_mode = DispatchTravelModeV1::StraightLine;
    geodesic.coverage = DispatchTravelCoverageV1::NotApplicable;
    geodesic.travel_minutes = None;

    let report = plan_shared_fleet_dispatch_v1(&input(
        vec![request("request-a", "site-a", 20, Some(5))],
        vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
        vec![geodesic],
    ))
    .unwrap();

    assert_eq!(
        assignment(&report, "request-a").disposition,
        SharedFleetDispatchDispositionV1::UnknownTravelEvidence
    );
    assert_eq!(report.scheduled_count, 0);
}

#[test]
fn missing_route_leg_is_unknown_not_zero_distance() {
    let report = plan_shared_fleet_dispatch_v1(&input(
        vec![request("request-a", "site-a", 20, Some(5))],
        vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
        vec![],
    ))
    .unwrap();

    assert_eq!(
        assignment(&report, "request-a").disposition,
        SharedFleetDispatchDispositionV1::UnknownTravelEvidence
    );
    assert_eq!(assignment(&report, "request-a").travel_minutes, None);
    assert_eq!(report.scheduled_count, 0);
}

#[test]
fn no_route_claim_requires_validated_coverage_and_source_evidence() {
    let confirmed = no_route("robot-1", "depot", "site-a");
    let report = plan_shared_fleet_dispatch_v1(&input(
        vec![request("request-a", "site-a", 20, Some(5))],
        vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
        vec![confirmed],
    ))
    .unwrap();
    assert_eq!(
        assignment(&report, "request-a").disposition,
        SharedFleetDispatchDispositionV1::NoFeasibleRoute
    );

    let mut unsupported = no_route("robot-1", "depot", "site-a");
    unsupported.coverage = DispatchTravelCoverageV1::Unknown;
    assert_blocked(
        &input(
            vec![request("request-a", "site-a", 20, Some(5))],
            vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
            vec![unsupported],
        ),
        SharedFleetDispatchBlockerCodeV1::InvalidTravelCoverage,
    );

    let mut no_evidence = no_route("robot-1", "depot", "site-a");
    no_evidence.evidence_ref = None;
    assert_blocked(
        &input(
            vec![request("request-a", "site-a", 20, Some(5))],
            vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
            vec![no_evidence],
        ),
        SharedFleetDispatchBlockerCodeV1::MissingTravelEvidence,
    );

    let mut observation_only = no_route("robot-1", "depot", "site-a");
    observation_only.metric = DispatchTravelMetricV1::ObservedTripTime;
    assert_blocked(
        &input(
            vec![request("request-a", "site-a", 20, Some(5))],
            vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
            vec![observation_only],
        ),
        SharedFleetDispatchBlockerCodeV1::InvalidTravelStateValue,
    );
}

#[test]
fn unvalidated_route_does_not_imply_feasible_dispatch() {
    let mut unvalidated = route("robot-1", "depot", "site-a", 5);
    unvalidated.coverage = DispatchTravelCoverageV1::Unvalidated;

    let report = plan_shared_fleet_dispatch_v1(&input(
        vec![request("request-a", "site-a", 20, Some(5))],
        vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
        vec![unvalidated],
    ))
    .unwrap();

    assert_eq!(
        assignment(&report, "request-a").disposition,
        SharedFleetDispatchDispositionV1::UnknownTravelEvidence
    );
    assert_eq!(report.scheduled_count, 0);
}

#[test]
fn unknown_task_duration_is_not_zero() {
    let report = plan_shared_fleet_dispatch_v1(&input(
        vec![request("request-a", "site-a", 20, None)],
        vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
        vec![route("robot-1", "depot", "site-a", 1)],
    ))
    .unwrap();

    assert_eq!(
        assignment(&report, "request-a").disposition,
        SharedFleetDispatchDispositionV1::UnknownTaskDuration
    );
    assert_eq!(assignment(&report, "request-a").forecast_completion_minute, None);
}

#[test]
fn capacity_limit_yields_explicit_unavailable_outcome() {
    let report = plan_shared_fleet_dispatch_v1(&input(
        vec![request("request-a", "site-a", 20, Some(5))],
        vec![robot("robot-1", "depot", 5, DispatchRobotReadinessV1::Ready)],
        vec![route("robot-1", "depot", "site-a", 2)],
    ))
    .unwrap();

    assert_eq!(
        assignment(&report, "request-a").disposition,
        SharedFleetDispatchDispositionV1::CapacityUnavailable
    );
    assert_eq!(report.scheduled_count, 0);
}

#[test]
fn maintenance_robot_is_never_assigned() {
    let report = plan_shared_fleet_dispatch_v1(&input(
        vec![request("request-a", "site-a", 20, Some(5))],
        vec![robot(
            "robot-1",
            "depot",
            100,
            DispatchRobotReadinessV1::PartsUnavailable,
        )],
        vec![route("robot-1", "depot", "site-a", 1)],
    ))
    .unwrap();

    assert_eq!(
        assignment(&report, "request-a").disposition,
        SharedFleetDispatchDispositionV1::AllCompatibleRobotsUnavailable
    );
    assert_eq!(assignment(&report, "request-a").robot_id, None);
}

#[test]
fn no_compatible_robot_is_counted_explicitly() {
    let mut robot = robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready);
    robot.supported_task_codes = vec!["spraying".into()];

    let report = plan_shared_fleet_dispatch_v1(&input(
        vec![request("request-a", "site-a", 20, Some(5))],
        vec![robot],
        vec![],
    ))
    .unwrap();

    assert_eq!(
        assignment(&report, "request-a").disposition,
        SharedFleetDispatchDispositionV1::NoCompatibleRobot
    );
    assert_eq!(report.no_compatible_robot_count, 1);
}

#[test]
fn observed_trip_time_is_not_used_as_future_forecast() {
    let mut observed = route("robot-1", "site-a", "site-b", 4);
    observed.metric = DispatchTravelMetricV1::ObservedTripTime;
    observed.method_id = Some("synthetic-trip-log".into());
    observed.method_version = Some("trip-protocol-v1".into());
    observed.evidence_ref = Some("evidence:synthetic:observed-trip".into());

    let report = plan_shared_fleet_dispatch_v1(&input(
        vec![
            request("request-a", "site-a", 20, Some(1)),
            request("request-b", "site-b", 30, Some(1)),
        ],
        vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
        vec![route("robot-1", "depot", "site-a", 3), observed],
    ))
    .unwrap();

    assert_eq!(report.scheduled_count, 1);
    assert_eq!(report.total_scheduled_modeled_route_minutes, 3);
    assert_eq!(
        assignment(&report, "request-a").travel_metric,
        Some(DispatchTravelMetricV1::ModeledRouteTime)
    );
    assert_eq!(
        assignment(&report, "request-b").disposition,
        SharedFleetDispatchDispositionV1::UnknownTravelEvidence
    );
    assert_eq!(assignment(&report, "request-b").travel_minutes, None);
}

#[test]
fn duplicate_route_key_blocks_without_partial_report() {
    let leg = route("robot-1", "depot", "site-a", 5);
    assert_blocked(
        &input(
            vec![request("request-a", "site-a", 20, Some(5))],
            vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
            vec![leg.clone(), leg],
        ),
        SharedFleetDispatchBlockerCodeV1::DuplicateTravelLeg,
    );
}

#[test]
fn cross_season_route_evidence_blocks() {
    let mut wrong_season = route("robot-1", "depot", "site-a", 5);
    wrong_season.season_ref = "season:synthetic-wet-v1".into();
    assert_blocked(
        &input(
            vec![request("request-a", "site-a", 20, Some(5))],
            vec![robot("robot-1", "depot", 100, DispatchRobotReadinessV1::Ready)],
            vec![wrong_season],
        ),
        SharedFleetDispatchBlockerCodeV1::SeasonMismatch,
    );
}

#[test]
fn time_overflow_blocks_without_saturated_or_wrapped_schedule() {
    let mut r = request("request-a", "site-a", u64::MAX, Some(1));
    r.requested_at_minute = u64::MAX - 4;
    let report = input(
        vec![r],
        vec![robot(
            "robot-1",
            "depot",
            u64::MAX,
            DispatchRobotReadinessV1::Ready,
        )],
        vec![route("robot-1", "depot", "site-a", 4)],
    );
    assert_blocked(&report, SharedFleetDispatchBlockerCodeV1::ArithmeticOverflow);
}
