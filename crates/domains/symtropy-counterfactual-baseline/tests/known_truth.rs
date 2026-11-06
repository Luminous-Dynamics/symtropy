// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_counterfactual_baseline::{
    BaselineFold, BaselineIdentity,
    BaselineQualificationContract, DispatchEventPoint, NoDispatchObservation,
    evaluate_baseline_bounded_event, qualify_counterfactual_baseline,
};

fn identity() -> BaselineIdentity {
    BaselineIdentity::new(
        "site-a",
        "pcc-1",
        "baseline-model-v1",
        "sha256:feature-set",
        "sha256:training",
        "sha256:calibration",
        "sha256:evaluation",
    )
    .expect("valid identity")
}

fn contract() -> BaselineQualificationContract {
    BaselineQualificationContract::new(
        500,   // 5% lower residual tail
        19,    // minimum for the requested lower tail
        10,
        200,   // maximum absolute normalized bias: 2%
        500,   // maximum normalized MAE: 5%
        9_000, // 90% observed lower-bound coverage
        100,   // maximum false-positive rate: 1%
        10.0,
        9_500,
    )
    .expect("valid contract")
}

fn observation(
    sample: usize,
    group: &str,
    fold: BaselineFold,
    predicted: f64,
    observed: f64,
) -> NoDispatchObservation {
    let source_digest = match fold {
        BaselineFold::Calibration => "sha256:calibration",
        BaselineFold::Evaluation => "sha256:evaluation",
    };
    NoDispatchObservation::new(
        format!("sample-{sample}"),
        group,
        "normal",
        source_digest,
        0,
        sample as u64 * 1_000_000_000,
        1_000_000_000,
        predicted,
        observed,
        0.0,
        fold,
        false,
    )
    .expect("valid no-dispatch observation")
}

fn qualified_report() -> symtropy_counterfactual_baseline::BaselineQualificationReport {
    let mut rows = Vec::new();
    for i in 0..40 {
        rows.push(observation(
            i,
            &format!("cal-group-{i}"),
            BaselineFold::Calibration,
            100.0,
            100.0,
        ));
    }
    for i in 0..20 {
        rows.push(observation(
            100 + i,
            &format!("eval-group-{i}"),
            BaselineFold::Evaluation,
            100.0,
            100.0,
        ));
    }
    qualify_counterfactual_baseline(&identity(), &contract(), &rows)
        .expect("baseline qualifies")
}

#[test]
fn exact_no_dispatch_baseline_qualifies_on_disjoint_holdout_groups() {
    let report = qualified_report();
    assert!(report.is_qualified());
    assert!(report.calibration_evaluation_groups_disjoint());
    assert!(report.forecast_issue_time_checked());
    let regime = report.regime("normal").unwrap();
    assert_eq!(regime.lower_residual_quantile_kw, 0.0);
    assert_eq!(regime.lower_bound_coverage_basis_points, 10_000.0);
    assert_eq!(regime.false_positive_reduction_rate_basis_points, 0.0);
}

#[test]
fn future_forecast_feature_timestamp_is_rejected() {
    let error = NoDispatchObservation::new(
        "future",
        "group",
        "normal",
        "sha256:source",
        2_000,
        1_000,
        1_000,
        100.0,
        100.0,
        0.0,
        BaselineFold::Evaluation,
        false,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::FutureInformationLeakage { .. }
    ));
}

#[test]
fn dispatched_observations_cannot_be_used_as_no_dispatch_controls() {
    let error = NoDispatchObservation::new(
        "treated",
        "group",
        "normal",
        "sha256:source",
        0,
        1_000,
        1_000,
        100.0,
        90.0,
        0.0,
        BaselineFold::Calibration,
        true,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::DispatchedDataInBaselineFold { .. }
    ));
}

#[test]
fn same_independence_group_cannot_leak_across_calibration_and_evaluation() {
    let mut rows = Vec::new();
    for i in 0..40 {
        rows.push(observation(
            i,
            if i == 0 { "leaked-group" } else { &format!("cal-{i}") },
            BaselineFold::Calibration,
            100.0,
            100.0,
        ));
    }
    for i in 0..20 {
        rows.push(observation(
            100 + i,
            if i == 0 { "leaked-group" } else { &format!("eval-{i}") },
            BaselineFold::Evaluation,
            100.0,
            100.0,
        ));
    }
    let error = qualify_counterfactual_baseline(&identity(), &contract(), &rows).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::CalibrationEvaluationGroupOverlap { .. }
    ));
}

#[test]
fn positive_baseline_bias_is_detected_and_does_not_mint_false_reduction() {
    let mut rows = Vec::new();
    for i in 0..40 {
        rows.push(observation(
            i,
            &format!("cal-{i}"),
            BaselineFold::Calibration,
            110.0,
            100.0,
        ));
    }
    for i in 0..20 {
        rows.push(observation(
            100 + i,
            &format!("eval-{i}"),
            BaselineFold::Evaluation,
            110.0,
            100.0,
        ));
    }
    let report = qualify_counterfactual_baseline(&identity(), &contract(), &rows)
        .expect("qualification report produced");
    assert!(!report.is_qualified());
    let regime = report.regime("normal").unwrap();
    assert!(regime.normalized_bias_basis_points > 200.0);
    assert!(regime.false_positive_reduction_rate_basis_points <= 100.0);
}

#[test]
fn distribution_shift_fails_holdout_coverage() {
    let mut rows = Vec::new();
    for i in 0..40 {
        rows.push(observation(
            i,
            &format!("cal-{i}"),
            BaselineFold::Calibration,
            100.0,
            100.0,
        ));
    }
    for i in 0..20 {
        rows.push(observation(
            100 + i,
            &format!("eval-{i}"),
            BaselineFold::Evaluation,
            100.0,
            80.0,
        ));
    }
    let report = qualify_counterfactual_baseline(&identity(), &contract(), &rows)
        .expect("qualification report produced");
    assert!(!report.is_qualified());
    assert_eq!(report.regime("normal").unwrap().lower_bound_coverage_basis_points, 0.0);
}

#[test]
fn split_conformal_tail_requires_enough_calibration_samples() {
    let invalid = BaselineQualificationContract::new(
        500,
        18,
        10,
        200,
        500,
        9_000,
        100,
        10.0,
        9_500,
    )
    .unwrap_err();

    assert!(matches!(
        invalid,
        symtropy_counterfactual_baseline::BaselineQualificationError::InsufficientCalibrationForTail { .. }
    ));
}

#[test]
fn baseline_and_meter_uncertainty_are_both_subtracted_from_delivery() {
    let baseline = qualified_report();
    let point = DispatchEventPoint::new(
        "event-1",
        "normal",
        0,
        0,
        3_600_000_000_000,
        100.0,
        85.0,
        2.0,
        "pcc-receipt-1",
    )
    .expect("valid event point");
    let report = evaluate_baseline_bounded_event(&baseline, &identity(), &[point], true)
        .expect("event report");

    assert_eq!(report.intervals[0].baseline_lower_power_bound_kw, 100.0);
    assert_eq!(report.intervals[0].measured_dispatch_upper_power_bound_kw, 87.0);
    assert_eq!(report.intervals[0].reduction_point_estimate_kw, 15.0);
    assert_eq!(report.intervals[0].conservative_reduction_lower_bound_kw, 13.0);
    assert_eq!(report.net_reduction_energy_kwh, 15.0);
    assert!(report.event_qualified);
}

#[test]
fn rebound_is_preserved_and_reduces_net_event_energy() {
    let baseline = qualified_report();
    let points = [
        DispatchEventPoint::new(
            "event-1",
            "normal",
            0,
            0,
            3_600_000_000_000,
            100.0,
            80.0,
            0.0,
            "pcc-1",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "recovery-1",
            "normal",
            0,
            3_600_000_000_000,
            3_600_000_000_000,
            100.0,
            110.0,
            0.0,
            "pcc-2",
        )
        .unwrap(),
    ];
    let report = evaluate_baseline_bounded_event(&baseline, &identity(), &points, true)
        .expect("event report");

    assert_eq!(report.gross_reduction_energy_kwh, 20.0);
    assert_eq!(report.gross_rebound_energy_kwh, 10.0);
    assert_eq!(report.net_reduction_energy_kwh, 10.0);
    assert_eq!(report.intervals[1].conservative_reduction_lower_bound_kw, -10.0);
}

#[test]
fn unqualified_baseline_cannot_be_used_for_event_credit() {
    let mut rows = Vec::new();
    for i in 0..40 {
        rows.push(observation(
            i,
            &format!("cal-{i}"),
            BaselineFold::Calibration,
            110.0,
            100.0,
        ));
    }
    for i in 0..20 {
        rows.push(observation(
            100 + i,
            &format!("eval-{i}"),
            BaselineFold::Evaluation,
            110.0,
            100.0,
        ));
    }
    let report = qualify_counterfactual_baseline(&identity(), &contract(), &rows)
        .expect("report");
    let point = DispatchEventPoint::new(
        "event-1",
        "normal",
        0,
        0,
        1_000,
        110.0,
        80.0,
        0.0,
        "pcc-1",
    )
    .unwrap();
    let error = evaluate_baseline_bounded_event(&report, &identity(), &[point], true).unwrap_err();

    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::BaselineNotQualified
    ));
}

#[test]
fn event_identity_regime_and_chronology_are_bound() {
    let baseline = qualified_report();
    let point = DispatchEventPoint::new(
        "event-1",
        "unknown-regime",
        0,
        0,
        1_000,
        100.0,
        90.0,
        0.0,
        "pcc-1",
    )
    .unwrap();
    let error = evaluate_baseline_bounded_event(&baseline, &identity(), &[point]).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::UnknownOperatingRegime { .. }
    ));

    let other = BaselineIdentity::new(
        "site-b",
        "pcc-1",
        "baseline-model-v1",
        "sha256:feature-set",
        "sha256:training",
        "sha256:calibration",
        "sha256:evaluation",
    )
    .unwrap();
    let point = DispatchEventPoint::new(
        "event-2",
        "normal",
        0,
        0,
        1_000,
        100.0,
        90.0,
        0.0,
        "pcc-2",
    )
    .unwrap();
    assert!(matches!(
        evaluate_baseline_bounded_event(&baseline, &other, &[point], true),
        Err(symtropy_counterfactual_baseline::BaselineQualificationError::BaselineIdentityMismatch)
    ));
}

#[test]
fn non_contiguous_recovery_horizon_is_rejected() {
    let baseline = qualified_report();
    let points = [
        DispatchEventPoint::new("event-1", "normal", 0, 0, 1_000, 100.0, 80.0, 0.0, "pcc-1").unwrap(),
        DispatchEventPoint::new("recovery-1", "normal", 0, 2_000, 1_000, 100.0, 110.0, 0.0, "pcc-2").unwrap(),
    ];
    let error = evaluate_baseline_bounded_event(&baseline, &identity(), &points).unwrap_err();

    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::NonContiguousEventIntervals { .. }
    ));
}

#[test]
fn residual_quantile_weights_independent_blocks_not_autocorrelated_rows() {
    let mut rows = Vec::new();
    for i in 0..3 {
        rows.push(observation(
            i,
            "one-bad-session",
            BaselineFold::Calibration,
            100.0,
            80.0,
        ));
    }
    for i in 3..42 {
        rows.push(observation(
            i,
            &format!("cal-group-{i}"),
            BaselineFold::Calibration,
            100.0,
            100.0,
        ));
    }
    for i in 0..20 {
        rows.push(observation(
            100 + i,
            &format!("eval-group-{i}"),
            BaselineFold::Evaluation,
            100.0,
            100.0,
        ));
    }
    let report = qualify_counterfactual_baseline(&identity(), &contract(), &rows)
        .expect("baseline qualification report");
    assert_eq!(report.regime("normal").unwrap().calibration_group_count, 40);
    assert_eq!(report.regime("normal").unwrap().lower_residual_quantile_kw, 0.0);
}

#[test]
fn evaluation_regime_without_calibration_abstains() {
    let mut rows = Vec::new();
    for i in 0..40 {
        rows.push(observation(
            i,
            &format!("cal-{i}"),
            BaselineFold::Calibration,
            100.0,
            100.0,
        ));
    }
    for i in 0..20 {
        rows.push(NoDispatchObservation::new(
            format!("hot-eval-{i}"),
            format!("hot-group-{i}"),
            "heatwave",
            "sha256:evaluation",
            0,
            (100 + i) as u64 * 1_000_000_000,
            1_000_000_000,
            100.0,
            100.0,
            0.0,
            BaselineFold::Evaluation,
            false,
        ).unwrap());
    }
    let error = qualify_counterfactual_baseline(&identity(), &contract(), &rows)
        .unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::EvaluationRegimeMissingCalibration { .. }
    ));
}

#[test]
fn observation_source_digest_must_match_pinned_partition() {
    let mut rows = Vec::new();
    for i in 0..40 {
        rows.push(observation(
            i,
            &format!("cal-{i}"),
            BaselineFold::Calibration,
            100.0,
            100.0,
        ));
    }
    rows.push(NoDispatchObservation::new(
        "wrong-source",
        "eval-group",
        "normal",
        "sha256:wrong-evaluation-source",
        0,
        100_000_000_000,
        1_000_000_000,
        100.0,
        100.0,
        0.0,
        BaselineFold::Evaluation,
        false,
    ).unwrap());
    for i in 1..20 {
        rows.push(observation(
            100 + i,
            &format!("eval-{i}"),
            BaselineFold::Evaluation,
            100.0,
            100.0,
        ));
    }
    let error = qualify_counterfactual_baseline(&identity(), &contract(), &rows)
        .unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::SourceDigestMismatch { .. }
    ));
}
