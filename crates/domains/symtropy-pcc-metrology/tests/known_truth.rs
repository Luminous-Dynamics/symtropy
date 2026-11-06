// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_pcc_metrology::{
    CalibrationCorrelations, CalibrationParameterUncertainty, ClockAlignmentEvidence,
    LinearPowerCalibration, MeasurementChainIdentity, MeterReading, PairedPowerInterval,
    PccMetrologyContract, ReadingQuality, evaluate_pcc_metrology,
};

fn identity() -> MeasurementChainIdentity {
    MeasurementChainIdentity::new(
        "site-a",
        "pcc-1",
        "meter-1",
        "sha256:meter-config",
        "cal-2026-01",
        "sha256:certificate",
    )
    .expect("valid identity")
}

fn calibration(gain: f64, offset_w: f64) -> LinearPowerCalibration {
    LinearPowerCalibration::new(
        "cal-2026-01",
        gain,
        offset_w,
        CalibrationParameterUncertainty::zero(),
        0,
        100_000,
        -10_000.0,
        10_000.0,
    )
    .expect("valid calibration")
}

fn contract(minimum_response_w: f64) -> PccMetrologyContract {
    PccMetrologyContract::new(2.0, minimum_response_w, 100, 0.10, 1.0).expect("valid contract")
}

fn clock() -> ClockAlignmentEvidence {
    ClockAlignmentEvidence::new(5, 5, "clock-model-1")
        .expect("valid clock")
        .with_maximum_power_slew_bounds(0.0, 0.0)
        .expect("valid slew bounds")
}

fn reading(
    start_ns: u64,
    duration_ns: u64,
    power_w: f64,
    uncertainty_w: f64,
    quality: ReadingQuality,
    evidence_id: &str,
) -> MeterReading {
    MeterReading::new(
        start_ns,
        duration_ns,
        power_w,
        uncertainty_w,
        quality,
        evidence_id,
    )
    .expect("valid reading")
}

fn pair(
    start_ns: u64,
    duration_ns: u64,
    baseline_w: f64,
    candidate_w: f64,
    baseline_u_w: f64,
    candidate_u_w: f64,
    correlation: f64,
) -> PairedPowerInterval {
    PairedPowerInterval::new(
        reading(
            start_ns,
            duration_ns,
            baseline_w,
            baseline_u_w,
            ReadingQuality::Qualified,
            "baseline-evidence",
        ),
        reading(
            start_ns,
            duration_ns,
            candidate_w,
            candidate_u_w,
            ReadingQuality::Qualified,
            "candidate-evidence",
        ),
        correlation,
    )
    .expect("valid paired interval")
}

#[test]
fn gain_offset_correction_is_exact_and_applied_before_pairing() {
    let report = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.02, 20.0),
        &contract(1.0),
        &clock(),
        &[pair(0, 1_000, 1_020.0, 920.0, 0.0, 0.0, 0.0)],
    )
    .expect("evaluation succeeds");

    assert!((report.intervals[0].baseline_corrected_power_w - 1_020.0).abs() < 1e-12);
    assert!((report.intervals[0].candidate_corrected_power_w - 918.0).abs() < 1e-12);
    assert!((report.intervals[0].measured_reduction_w - 102.0).abs() < 1e-12);
}

#[test]
fn shared_chain_covariance_is_used_but_never_assumed() {
    let correlated = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(98.0),
        &clock(),
        &[pair(0, 1_000, 1_000.0, 900.0, 2.0, 2.0, 0.9)],
    )
    .expect("correlated evaluation");
    let sample = &correlated.intervals[0];

    assert!((sample.reduction_standard_uncertainty_w - 0.894_427_190_999_915_9).abs() < 1e-12);
    assert!((sample.expanded_uncertainty_w - 1.788_854_381_999_831_7).abs() < 1e-12);
    assert!((sample.conservative_reduction_lower_bound_w - 98.211_145_618_000_17).abs() < 1e-12);
    assert!(sample.interval_qualified);

    let independent = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(98.0),
        &clock(),
        &[pair(0, 1_000, 1_000.0, 900.0, 2.0, 2.0, 0.0)],
    )
    .expect("independent evaluation");
    assert!(
        (independent.intervals[0].reduction_standard_uncertainty_w - 2.828_427_124_746_190_3).abs()
            < 1e-12
    );
    assert!(!independent.intervals[0].interval_qualified);
}

#[test]
fn lower_bound_crossing_the_contract_threshold_does_not_pass() {
    let report = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(5.0),
        &clock(),
        &[pair(0, 1_000, 100.0, 94.0, 0.0, 1.0, 0.0)],
    )
    .expect("evaluation succeeds");

    assert_eq!(report.intervals[0].measured_reduction_w, 6.0);
    assert_eq!(
        report.intervals[0].conservative_reduction_lower_bound_w,
        4.0
    );
    assert!(!report.intervals[0].response_threshold_pass);
    assert!(!report.metrology_qualified);
}

#[test]
fn signed_rebound_is_not_clamped_and_energy_is_integrated() {
    let intervals = [
        pair(0, 1_000_000_000, 100.0, 90.0, 0.0, 0.0, 0.0),
        pair(1_000_000_000, 2_000_000_000, 100.0, 105.0, 0.0, 0.0, 0.0),
    ];
    let report = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(1.0),
        &clock(),
        &intervals,
    )
    .expect("evaluation succeeds");

    assert!((report.gross_reduction_energy_wh - 10.0 / 3_600.0).abs() < 1e-12);
    assert!((report.gross_rebound_energy_wh - 10.0 / 3_600.0).abs() < 1e-12);
    assert!(report.net_reduction_energy_wh.abs() < 1e-12);
    assert!(report.sub_interval_transients_unverified);
}

#[test]
fn calibration_parameter_uncertainty_and_covariance_are_propagated() {
    let uncertainty = CalibrationParameterUncertainty::new(
        0.001,
        0.5,
        CalibrationCorrelations::new(0.1, 0.0, 0.0).expect("valid correlation"),
    )
    .expect("valid uncertainty");
    let cal = LinearPowerCalibration::new(
        "cal-2026-01",
        1.01,
        2.0,
        uncertainty,
        0,
        100_000,
        -10_000.0,
        10_000.0,
    )
    .expect("valid calibration");
    let report = evaluate_pcc_metrology(
        &identity(),
        &cal,
        &contract(1.0),
        &clock(),
        &[pair(0, 1_000, 1_000.0, 900.0, 2.0, 2.0, 0.0)],
    )
    .expect("evaluation succeeds");

    assert!(report.intervals[0].baseline_standard_uncertainty_w > 1.0);
    assert!(report.intervals[0].candidate_standard_uncertainty_w > 1.0);
}

#[test]
fn non_positive_semidefinite_calibration_correlation_is_rejected() {
    let error = CalibrationCorrelations::new(0.9, 0.9, -0.9).unwrap_err();
    assert!(matches!(
        error,
        symtropy_pcc_metrology::PccMetrologyError::NonPositiveSemidefiniteCalibrationCovariance { .. }
    ));
}

#[test]
fn excessive_clock_uncertainty_is_an_abstention() {
    let clock = ClockAlignmentEvidence::new(15, 10, "clock-model-1").expect("valid clock");
    let error = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(1.0),
        &clock,
        &[pair(0, 1_000, 100.0, 90.0, 0.0, 0.0, 0.0)],
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_pcc_metrology::PccMetrologyError::ClockUncertaintyExceedsContract { .. }
    ));
}

#[test]
fn expired_calibration_is_rejected_even_for_otherwise_good_readings() {
    let cal = LinearPowerCalibration::new(
        "cal-2026-01",
        1.0,
        0.0,
        CalibrationParameterUncertainty::zero(),
        0,
        500,
        -10_000.0,
        10_000.0,
    )
    .expect("valid calibration");
    let error = evaluate_pcc_metrology(
        &identity(),
        &cal,
        &contract(1.0),
        &clock(),
        &[pair(0, 1_000, 100.0, 90.0, 0.0, 0.0, 0.0)],
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_pcc_metrology::PccMetrologyError::CalibrationNotValid { .. }
    ));
}

#[test]
fn clipped_or_missing_reading_is_rejected_without_imputation() {
    let clipped = PairedPowerInterval::new(
        reading(
            0,
            1_000,
            100.0,
            0.0,
            ReadingQuality::Clipped,
            "baseline-clipped",
        ),
        reading(
            0,
            1_000,
            90.0,
            0.0,
            ReadingQuality::Qualified,
            "candidate-valid",
        ),
        0.0,
    )
    .expect("pair structure valid");
    let error = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(1.0),
        &clock(),
        &[clipped],
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_pcc_metrology::PccMetrologyError::ReadingIneligible {
            quality: ReadingQuality::Clipped,
            ..
        }
    ));
}

#[test]
fn interval_gaps_are_rejected_instead_of_interpolated() {
    let intervals = [
        pair(0, 1_000, 100.0, 90.0, 0.0, 0.0, 0.0),
        pair(2_000, 1_000, 100.0, 90.0, 0.0, 0.0, 0.0),
    ];
    let error = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(1.0),
        &clock(),
        &intervals,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_pcc_metrology::PccMetrologyError::NonContiguousIntervals { .. }
    ));
}

#[test]
fn mismatched_pair_boundaries_are_rejected() {
    let candidate = reading(1, 1_000, 90.0, 0.0, ReadingQuality::Qualified, "candidate");
    let baseline = reading(0, 1_000, 100.0, 0.0, ReadingQuality::Qualified, "baseline");
    let interval = PairedPowerInterval::new(baseline, candidate, 0.0).expect("pair accepted");
    let error = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(1.0),
        &clock(),
        &[interval],
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_pcc_metrology::PccMetrologyError::PairedTimestampMismatch { .. }
    ));
}

#[test]
fn calibration_operating_range_is_fail_closed() {
    let cal = LinearPowerCalibration::new(
        "cal-2026-01",
        1.0,
        0.0,
        CalibrationParameterUncertainty::zero(),
        0,
        100_000,
        0.0,
        100.0,
    )
    .expect("valid calibration");
    let error = evaluate_pcc_metrology(
        &identity(),
        &cal,
        &contract(1.0),
        &clock(),
        &[pair(0, 1_000, 150.0, 90.0, 0.0, 0.0, 0.0)],
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_pcc_metrology::PccMetrologyError::OutsideCalibrationOperatingRange { .. }
    ));
}

#[test]
fn clock_time_uncertainty_is_propagated_through_declared_power_slew() {
    let clock = ClockAlignmentEvidence::new(5, 5, "clock-model-1")
        .expect("valid clock")
        .with_maximum_power_slew_bounds(1.0e9, 1.0e9)
        .expect("valid slew bounds");
    let report = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(95.0),
        &clock,
        &[pair(0, 1_000_000_000, 1_000.0, 900.0, 0.0, 0.0, 0.0)],
    )
    .expect("evaluation succeeds");
    let interval = &report.intervals[0];
    assert_eq!(report.baseline_clock_uncertainty_ns, 5);
    assert_eq!(report.candidate_clock_uncertainty_ns, 5);
    assert_eq!(report.baseline_maximum_absolute_power_slew_w_per_s, 1.0e9);
    assert_eq!(report.candidate_maximum_absolute_power_slew_w_per_s, 1.0e9);
    assert_eq!(interval.measured_reduction_w, 100.0);
    assert!((interval.time_alignment_uncertainty_bound_w - 10.0).abs() < 1e-12);
    assert!((interval.expanded_uncertainty_w - 10.0).abs() < 1e-12);
    assert!((interval.conservative_reduction_lower_bound_w - 90.0).abs() < 1e-12);
    assert!(!interval.response_threshold_pass);
}

#[test]
fn nonzero_clock_uncertainty_without_slew_bounds_abstains() {
    let clock = ClockAlignmentEvidence::new(5, 5, "clock-model-1").expect("valid clock");
    let error = evaluate_pcc_metrology(
        &identity(),
        &calibration(1.0, 0.0),
        &contract(1.0),
        &clock,
        &[pair(0, 1_000, 100.0, 90.0, 0.0, 0.0, 0.0)],
    )
    .unwrap_err();
    assert!(matches!(
        error,
        symtropy_pcc_metrology::PccMetrologyError::MissingClockSlewBounds { .. }
    ));
}

#[test]
fn invalid_power_slew_bound_is_rejected() {
    let error = ClockAlignmentEvidence::new(5, 5, "clock-model-1")
        .expect("valid clock")
        .with_maximum_power_slew_bounds(-1.0, 0.0)
        .unwrap_err();
    assert!(matches!(
        error,
        symtropy_pcc_metrology::PccMetrologyError::InvalidQuantity { .. }
    ));
}
