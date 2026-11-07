// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_large_load_disturbance::{
    ActivePowerRecoveryRequirement, ConnectionRequirement, DisturbanceError,
    DisturbanceEventIdentity, DisturbanceRequirementSet, DisturbanceTraceContract,
    PccDisturbanceSample, ReactiveCurrentRequirement, RideThroughSegment,
    evaluate_disturbance_conformance,
};

fn requirement_with_rule(
    rule_id: &str,
    digest: &str,
    first_zone_connection: ConnectionRequirement,
) -> DisturbanceRequirementSet {
    DisturbanceRequirementSet::new(
        rule_id,
        "2026.1",
        "operator-region-a",
        digest,
        "pcc-1",
        50.0,
        4_000_000_000,
        vec![
            RideThroughSegment::new(
                0,
                2_000_000_000,
                0.1,
                1.2,
                47.0,
                52.0,
                Some(2.0),
                first_zone_connection,
            )
            .unwrap(),
            RideThroughSegment::new(
                2_000_000_000,
                4_000_000_000,
                0.9,
                1.1,
                49.0,
                51.0,
                Some(1.0),
                ConnectionRequirement::MustRemainConnected,
            )
            .unwrap(),
        ],
        ActivePowerRecoveryRequirement::new(
            0.9,
            49.0,
            51.0,
            500_000_000,
            500_000_000,
            0.9,
            0.1,
            20.0,
        )
        .unwrap(),
    )
    .unwrap()
}

fn requirement() -> DisturbanceRequirementSet {
    requirement_with_rule(
        "operator-rule-1",
        "sha256:requirement",
        ConnectionRequirement::MustRemainConnected,
    )
}

fn event_for(rule_id: &str, digest: &str) -> DisturbanceEventIdentity {
    DisturbanceEventIdentity::new(
        "event-1",
        rule_id,
        "2026.1",
        digest,
        "pcc-1",
        "sha256:waveform",
        "sha256:meter-chain",
        "clock-model-1",
        "sha256:pre-event-reference",
        10_000_000_000,
        100.0,
        1_000,
        10_000_000,
        0.4,
    )
    .unwrap()
}

fn event() -> DisturbanceEventIdentity {
    event_for("operator-rule-1", "sha256:requirement")
}

fn contract() -> DisturbanceTraceContract {
    DisturbanceTraceContract::new(10_000, 1_000_000_000, 20_000_000, 0.4, 3).unwrap()
}

fn sample(
    elapsed_ns: u64,
    voltage: f64,
    frequency: f64,
    active_power: f64,
    connected: bool,
) -> PccDisturbanceSample {
    PccDisturbanceSample::new(
        10_000_000_000 + elapsed_ns,
        voltage,
        frequency,
        active_power,
        Some(0.0),
        Some(1.0),
        connected,
    )
    .unwrap()
}

fn passing_trace() -> Vec<PccDisturbanceSample> {
    vec![
        sample(0, 0.85, 49.5, 70.0, true),
        sample(1_000_000_000, 0.88, 49.7, 75.0, true),
        sample(2_000_000_000, 0.95, 50.0, 85.0, true),
        sample(2_500_000_000, 0.96, 50.0, 90.0, true),
        sample(3_000_000_000, 0.98, 50.0, 95.0, true),
        sample(3_999_999_999, 0.98, 50.0, 96.0, true),
    ]
}

#[test]
fn operator_envelope_and_power_recovery_known_truth_pass() {
    let report =
        evaluate_disturbance_conformance(&event(), &requirement(), &contract(), &passing_trace())
            .unwrap();

    assert!(report.ride_through_compliant);
    assert!(report.connection_continuity_compliant);
    assert!(report.recovery_condition_observed);
    assert!(report.recovery_compliant);
    assert!(report.qualified);
    assert!(!report.intentional_demand_response_credit);
    assert_eq!(report.jurisdiction, "operator-region-a");
    assert_eq!(report.nominal_frequency_hz, 50.0);
}

#[test]
fn unexpected_trip_inside_must_remain_connected_zone_fails() {
    let mut trace = passing_trace();
    trace[1] = sample(1_000_000_000, 0.88, 49.7, 0.0, false);
    let report =
        evaluate_disturbance_conformance(&event(), &requirement(), &contract(), &trace).unwrap();

    assert!(report.unexpected_trip_detected);
    assert!(!report.connection_continuity_compliant);
    assert!(!report.qualified);
    assert!(!report.intentional_demand_response_credit);
}

#[test]
fn operator_permitted_momentary_cessation_is_duration_bounded() {
    let requirement = requirement_with_rule(
        "operator-rule-2",
        "sha256:requirement-2",
        ConnectionRequirement::MomentaryCessationAllowed {
            maximum_disconnected_duration_ns: 500_000_000,
        },
    );
    let event = event_for("operator-rule-2", "sha256:requirement-2");
    let trace_contract =
        DisturbanceTraceContract::new(10_000, 1_000_000_000, 20_000_000, 0.4, 3).unwrap();
    let trace = vec![
        sample(0, 0.2, 49.5, 60.0, false),
        sample(250_000_000, 0.4, 49.5, 70.0, true),
        sample(1_000_000_000, 0.9, 50.0, 90.0, true),
        sample(2_000_000_000, 0.95, 50.0, 95.0, true),
        sample(2_500_000_000, 0.96, 50.0, 96.0, true),
        sample(3_000_000_000, 0.98, 50.0, 96.0, true),
        sample(3_999_999_999, 0.98, 50.0, 96.0, true),
    ];
    let report =
        evaluate_disturbance_conformance(&event, &requirement, &trace_contract, &trace).unwrap();

    assert!(!report.unexpected_trip_detected);
    assert!(report.connection_continuity_compliant);
    assert_eq!(report.total_disconnected_duration_ns, 250_000_000);
}

#[test]
fn momentary_cessation_over_operator_duration_limit_fails() {
    let requirement = requirement_with_rule(
        "operator-rule-3",
        "sha256:requirement-3",
        ConnectionRequirement::MomentaryCessationAllowed {
            maximum_disconnected_duration_ns: 100_000_000,
        },
    );
    let event = event_for("operator-rule-3", "sha256:requirement-3");
    let trace_contract =
        DisturbanceTraceContract::new(10_000, 1_000_000_000, 20_000_000, 0.4, 3).unwrap();
    let trace = vec![
        sample(0, 0.2, 49.5, 60.0, false),
        sample(250_000_000, 0.4, 49.5, 70.0, true),
        sample(1_000_000_000, 0.9, 50.0, 90.0, true),
        sample(2_000_000_000, 0.95, 50.0, 95.0, true),
        sample(2_500_000_000, 0.96, 50.0, 96.0, true),
        sample(3_000_000_000, 0.98, 50.0, 96.0, true),
        sample(3_999_999_999, 0.98, 50.0, 96.0, true),
    ];
    let report =
        evaluate_disturbance_conformance(&event, &requirement, &trace_contract, &trace).unwrap();

    assert!(!report.connection_continuity_compliant);
    assert!(report.unexpected_trip_detected);
    assert!(!report.qualified);
}

#[test]
fn voltage_frequency_boundary_violation_fails_without_panicking() {
    let mut trace = passing_trace();
    trace[0] = sample(0, 0.05, 49.5, 70.0, true);
    let report =
        evaluate_disturbance_conformance(&event(), &requirement(), &contract(), &trace).unwrap();

    assert!(!report.ride_through_compliant);
    assert!(!report.qualified);
}

#[test]
fn rocof_boundary_is_checked_using_measured_timebase() {
    let trace = vec![
        sample(0, 0.85, 50.0, 70.0, true),
        sample(1_000_000_000, 0.88, 47.0, 75.0, true),
        sample(2_000_000_000, 0.95, 50.0, 85.0, true),
        sample(2_500_000_000, 0.96, 50.0, 90.0, true),
        sample(3_000_000_000, 0.98, 50.0, 95.0, true),
        sample(3_999_999_999, 0.98, 50.0, 96.0, true),
    ];
    let report =
        evaluate_disturbance_conformance(&event(), &requirement(), &contract(), &trace).unwrap();

    assert!(!report.ride_through_compliant);
    assert!(report.maximum_absolute_rocof_hz_per_s >= 3.0);
}

#[test]
fn excessive_sample_gap_abstains() {
    let trace = vec![
        sample(0, 0.85, 50.0, 70.0, true),
        sample(1_000_000_000, 0.88, 49.7, 75.0, true),
        sample(2_000_000_000, 0.95, 50.0, 85.0, true),
        sample(3_000_000_000, 0.98, 50.0, 95.0, true),
        sample(3_999_999_999, 0.98, 50.0, 96.0, true),
    ];
    let strict_cadence =
        DisturbanceTraceContract::new(10_000, 500_000_000, 20_000_000, 0.4, 3).unwrap();
    let error = evaluate_disturbance_conformance(&event(), &requirement(), &strict_cadence, &trace)
        .unwrap_err();

    assert!(matches!(
        error,
        DisturbanceError::SampleGapExceedsContract { .. }
    ));
}

#[test]
fn incomplete_observation_horizon_abstains_even_when_samples_are_dense() {
    let trace = vec![
        sample(0, 0.85, 50.0, 70.0, true),
        sample(100_000_000, 0.86, 50.0, 71.0, true),
        sample(200_000_000, 0.87, 50.0, 72.0, true),
    ];
    let error = evaluate_disturbance_conformance(&event(), &requirement(), &contract(), &trace)
        .unwrap_err();

    assert!(matches!(
        error,
        DisturbanceError::ObservationHorizonIncomplete { .. }
    ));
}

#[test]
fn sample_at_exclusive_requirement_horizon_is_rejected() {
    let mut trace = passing_trace();
    trace[5] = sample(4_000_000_000, 0.98, 50.0, 96.0, true);
    let error = evaluate_disturbance_conformance(&event(), &requirement(), &contract(), &trace)
        .unwrap_err();

    assert!(matches!(
        error,
        DisturbanceError::SampleOutsideRequirementHorizon
    ));
}

#[test]
fn stale_requirement_or_wrong_pcc_abstains() {
    let stale_event = event_for("operator-rule-1", "sha256:old-requirement");
    assert!(matches!(
        evaluate_disturbance_conformance(
            &stale_event,
            &requirement(),
            &contract(),
            &passing_trace()
        ),
        Err(DisturbanceError::RequirementIdentityMismatch)
    ));

    let wrong_pcc = DisturbanceEventIdentity::new(
        "event-1",
        "operator-rule-1",
        "2026.1",
        "sha256:requirement",
        "wrong-pcc",
        "sha256:waveform",
        "sha256:meter-chain",
        "clock-model-1",
        "sha256:pre-event-reference",
        10_000_000_000,
        100.0,
        1_000,
        10_000_000,
        1_000.0,
    )
    .unwrap();
    assert!(matches!(
        evaluate_disturbance_conformance(&wrong_pcc, &requirement(), &contract(), &passing_trace()),
        Err(DisturbanceError::PccIdentityMismatch)
    ));
}

#[test]
fn measurement_bandwidth_rms_window_and_clock_uncertainty_are_enforced() {
    let long_rms = DisturbanceEventIdentity::new(
        "event-1",
        "operator-rule-1",
        "2026.1",
        "sha256:requirement",
        "pcc-1",
        "sha256:waveform",
        "sha256:meter-chain",
        "clock-model-1",
        "sha256:pre-event",
        10_000_000_000,
        100.0,
        1_000,
        30_000_000,
        1_000.0,
    )
    .unwrap();
    assert!(matches!(
        evaluate_disturbance_conformance(&long_rms, &requirement(), &contract(), &passing_trace()),
        Err(DisturbanceError::RmsWindowExceedsContract { .. })
    ));

    let low_bandwidth = DisturbanceEventIdentity::new(
        "event-1",
        "operator-rule-1",
        "2026.1",
        "sha256:requirement",
        "pcc-1",
        "sha256:waveform",
        "sha256:meter-chain",
        "clock-model-1",
        "sha256:pre-event",
        10_000_000_000,
        100.0,
        1_000,
        10_000_000,
        0.1,
    )
    .unwrap();
    assert!(matches!(
        evaluate_disturbance_conformance(
            &low_bandwidth,
            &requirement(),
            &contract(),
            &passing_trace()
        ),
        Err(DisturbanceError::MeasurementBandwidthBelowContract { .. })
    ));

    let uncertain_clock = DisturbanceEventIdentity::new(
        "event-1",
        "operator-rule-1",
        "2026.1",
        "sha256:requirement",
        "pcc-1",
        "sha256:waveform",
        "sha256:meter-chain",
        "clock-model-1",
        "sha256:pre-event",
        10_000_000_000,
        100.0,
        100_000,
        10_000_000,
        1_000.0,
    )
    .unwrap();
    assert!(matches!(
        evaluate_disturbance_conformance(
            &uncertain_clock,
            &requirement(),
            &contract(),
            &passing_trace()
        ),
        Err(DisturbanceError::ClockUncertaintyExceedsContract { .. })
    ));
}

#[test]
fn recovery_ramp_and_overshoot_must_both_conform() {
    let trace = vec![
        sample(0, 0.85, 49.5, 70.0, true),
        sample(1_000_000_000, 0.88, 49.7, 75.0, true),
        sample(2_000_000_000, 0.95, 50.0, 85.0, true),
        sample(2_500_000_000, 0.96, 50.0, 100.0, true),
        sample(3_000_000_000, 0.98, 50.0, 120.0, true),
        sample(3_999_999_999, 0.98, 50.0, 110.0, true),
    ];
    let report =
        evaluate_disturbance_conformance(&event(), &requirement(), &contract(), &trace).unwrap();

    assert!(!report.recovery_compliant);
    assert!(!report.qualified);
}

#[test]
fn no_voltage_recovery_means_recovery_not_qualified() {
    let trace = vec![
        sample(0, 0.7, 49.5, 70.0, true),
        sample(1_000_000_000, 0.75, 49.7, 75.0, true),
        sample(2_000_000_000, 0.8, 50.0, 85.0, true),
        sample(2_500_000_000, 0.8, 50.0, 90.0, true),
        sample(3_000_000_000, 0.8, 50.0, 95.0, true),
        sample(3_999_999_999, 0.8, 50.0, 96.0, true),
    ];
    let report =
        evaluate_disturbance_conformance(&event(), &requirement(), &contract(), &trace).unwrap();

    assert!(report.ride_through_compliant);
    assert!(!report.recovery_condition_observed);
    assert!(!report.recovery_compliant);
    assert!(!report.qualified);
}

#[test]
fn a_load_trip_is_never_issued_intentional_dispatch_credit() {
    let mut trace = passing_trace();
    trace[1] = sample(1_000_000_000, 0.88, 49.7, 0.0, false);
    let report =
        evaluate_disturbance_conformance(&event(), &requirement(), &contract(), &trace).unwrap();

    assert!(report.unexpected_trip_detected);
    assert!(!report.intentional_demand_response_credit);
}

#[test]
fn recovery_ramp_across_stability_boundary_is_not_hidden() {
    let trace = vec![
        sample(0, 0.85, 49.5, 70.0, true),
        sample(1_000_000_000, 0.88, 49.7, 75.0, true),
        sample(2_000_000_000, 0.95, 50.0, 80.0, true),
        sample(2_500_000_000, 0.96, 50.0, 80.0, true),
        sample(2_900_000_000, 0.98, 50.0, 80.0, true),
        sample(3_100_000_000, 0.98, 50.0, 100.0, true),
        sample(3_999_999_999, 0.98, 50.0, 100.0, true),
    ];
    let policy = DisturbanceTraceContract::new(10_000, 1_000_000_000, 20_000_000, 0.4, 3).unwrap();
    let report =
        evaluate_disturbance_conformance(&event(), &requirement(), &policy, &trace).unwrap();
    assert!(report.recovery_condition_observed);
    assert!(!report.recovery_compliant);
    assert!(report.maximum_absolute_recovery_ramp_mw_per_s.unwrap() >= 100.0);
}

#[test]
fn sample_cadence_cannot_claim_unobservable_bandwidth() {
    let error =
        DisturbanceTraceContract::new(10_000, 1_000_000_000, 20_000_000, 1.0, 3).unwrap_err();
    assert!(matches!(
        error,
        DisturbanceError::SamplingCadenceCannotResolveBandwidth { .. }
    ));
}

#[test]
fn operator_reactive_power_and_current_envelope_is_enforced() {
    let requirement = requirement().with_reactive_current_requirement(
        ReactiveCurrentRequirement::new(-5.0, 5.0, 1.2).unwrap(),
    );
    let mut trace = passing_trace();
    trace[3] = PccDisturbanceSample::new(
        10_000_000_000 + 2_500_000_000,
        0.96,
        50.0,
        90.0,
        Some(6.0),
        Some(1.3),
        true,
    ).unwrap();
    let report = evaluate_disturbance_conformance(&event(), &requirement, &contract(), &trace)
        .unwrap();
    assert!(report.reactive_current_assessed);
    assert!(!report.reactive_current_compliant);
    assert!(!report.qualified);
    assert_eq!(report.maximum_reactive_power_mvar, Some(6.0));
    assert_eq!(report.maximum_current_pu, Some(1.3));
}

#[test]
fn required_reactive_current_measurements_missing_causes_abstention() {
    let requirement = requirement().with_reactive_current_requirement(
        ReactiveCurrentRequirement::new(-5.0, 5.0, 1.2).unwrap(),
    );
    let mut trace = passing_trace();
    trace[3] = PccDisturbanceSample::new(
        10_000_000_000 + 2_500_000_000,
        0.96,
        50.0,
        90.0,
        None,
        Some(1.0),
        true,
    ).unwrap();
    let error = evaluate_disturbance_conformance(&event(), &requirement, &contract(), &trace)
        .unwrap_err();
    assert!(matches!(error, DisturbanceError::MissingReactivePowerSample { .. }));
}

#[test]
fn initial_measurement_gap_from_event_start_abstains() {
    let trace = vec![
        sample(100_000_000, 0.85, 50.0, 70.0, true),
        sample(150_000_000, 0.86, 50.0, 71.0, true),
        sample(200_000_000, 0.87, 50.0, 72.0, true),
    ];
    let policy = DisturbanceTraceContract::new(10_000, 50_000_000, 20_000_000, 0.4, 3).unwrap();
    let error =
        evaluate_disturbance_conformance(&event(), &requirement(), &policy, &trace).unwrap_err();
    assert!(matches!(
        error,
        DisturbanceError::InitialSampleGapExceedsContract { .. }
    ));
}
