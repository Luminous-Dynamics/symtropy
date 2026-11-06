// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_counterfactual_baseline::{
    BaselineFold, BaselineIdentity, BaselineQualificationContract, DispatchEventPhase,
    DispatchEventPoint, EventRecoveryEvidence, NoDispatchObservation,
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
        500, // 5% lower residual tail
        19,  // minimum for the requested lower tail
        10, 200,   // maximum absolute normalized bias: 2%
        500,   // maximum normalized MAE: 5%
        9_000, // 90% observed lower-bound coverage
        100,   // maximum false-positive rate: 1%
        10.0, 9_500,
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

fn recovery_evidence(
    start_ns: u64,
    dispatch_end_ns: u64,
    observed_through_ns: u64,
    complete: bool,
) -> EventRecoveryEvidence {
    EventRecoveryEvidence::new(
        start_ns,
        dispatch_end_ns,
        observed_through_ns,
        "recovery-reconciliation-1",
        "sha256:recovery-receipt",
        "workload-reconciliation-receipt-1",
        "physical-state-reconciliation-receipt-1",
        complete,
        complete,
    )
    .expect("valid recovery evidence")
}

fn qualified_report() -> symtropy_counterfactual_baseline::BaselineQualificationReport {
    qualified_report_with_shape(1, 1_000_000_000)
}

fn qualified_report_with_shape(
    interval_count: usize,
    interval_duration_ns: u64,
) -> symtropy_counterfactual_baseline::BaselineQualificationReport {
    let mut rows = Vec::new();
    for fold in [BaselineFold::Calibration, BaselineFold::Evaluation] {
        let group_count = match fold {
            BaselineFold::Calibration => 40,
            BaselineFold::Evaluation => 20,
        };
        for group_index in 0..group_count {
            let group_id = match fold {
                BaselineFold::Calibration => format!("cal-group-{group_index}"),
                BaselineFold::Evaluation => format!("eval-group-{group_index}"),
            };
            let sample_base = match fold {
                BaselineFold::Calibration => 0,
                BaselineFold::Evaluation => 10_000,
            } + group_index * interval_count;
            let group_start_ns = (group_index * 100
                + if fold == BaselineFold::Evaluation {
                    10_000
                } else {
                    0
                }) as u64
                * interval_duration_ns
                * interval_count as u64;
            for interval_index in 0..interval_count {
                let sample_id = sample_base + interval_index;
                let start_ns = group_start_ns + interval_index as u64 * interval_duration_ns;
                let source_digest = match fold {
                    BaselineFold::Calibration => "sha256:calibration",
                    BaselineFold::Evaluation => "sha256:evaluation",
                };
                rows.push(
                    NoDispatchObservation::new(
                        format!("trajectory-sample-{sample_id}"),
                        &group_id,
                        "normal",
                        source_digest,
                        0,
                        start_ns,
                        interval_duration_ns,
                        100.0,
                        100.0,
                        0.0,
                        fold,
                        false,
                    )
                    .expect("valid trajectory observation"),
                );
            }
        }
    }
    qualify_counterfactual_baseline(&identity(), &contract(), &rows).expect("baseline qualifies")
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
            if i == 0 {
                "leaked-group"
            } else {
                &format!("cal-{i}")
            },
            BaselineFold::Calibration,
            100.0,
            100.0,
        ));
    }
    for i in 0..20 {
        rows.push(observation(
            100 + i,
            if i == 0 {
                "leaked-group"
            } else {
                &format!("eval-{i}")
            },
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
    assert_eq!(
        report
            .regime("normal")
            .unwrap()
            .lower_bound_coverage_basis_points,
        0.0
    );
}

#[test]
fn split_conformal_tail_requires_enough_calibration_samples() {
    let invalid =
        BaselineQualificationContract::new(500, 18, 10, 200, 500, 9_000, 100, 10.0, 9_500)
            .unwrap_err();

    assert!(matches!(
        invalid,
        symtropy_counterfactual_baseline::BaselineQualificationError::InsufficientCalibrationForTail { .. }
    ));
}

#[test]
fn baseline_and_meter_uncertainty_are_both_subtracted_from_delivery() {
    let baseline = qualified_report_with_shape(2, 3_600_000_000_000);
    let points = [
        DispatchEventPoint::new(
            "event-1",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            0,
            3_600_000_000_000,
            100.0,
            85.0,
            2.0,
            "pcc-receipt-1",
            "sha256:recovery-receipt",
        )
        .expect("valid dispatch point"),
        DispatchEventPoint::new(
            "recovery-1",
            DispatchEventPhase::Recovery,
            "normal",
            0,
            3_600_000_000_000,
            3_600_000_000_000,
            100.0,
            100.0,
            0.0,
            "pcc-recovery-receipt-1",
            "sha256:recovery-receipt",
        )
        .expect("valid recovery point"),
    ];
    let evidence = recovery_evidence(0, 3_600_000_000_000, 7_200_000_000_000, true);
    let report = evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points)
        .expect("event report");

    assert_eq!(report.intervals[0].baseline_lower_power_bound_kw, 100.0);
    assert_eq!(
        report.intervals[0].measured_dispatch_upper_power_bound_kw,
        87.0
    );
    assert_eq!(report.intervals[0].reduction_point_estimate_kw, 15.0);
    assert_eq!(
        report.intervals[0].conservative_reduction_lower_bound_kw,
        13.0
    );
    assert_eq!(report.net_reduction_energy_kwh, 15.0);
    assert!(report.event_qualified);
}

#[test]
fn rebound_is_preserved_and_reduces_net_event_energy() {
    let baseline = qualified_report_with_shape(2, 3_600_000_000_000);
    let points = [
        DispatchEventPoint::new(
            "event-1",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            0,
            3_600_000_000_000,
            100.0,
            80.0,
            0.0,
            "pcc-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "recovery-1",
            DispatchEventPhase::Recovery,
            "normal",
            0,
            3_600_000_000_000,
            3_600_000_000_000,
            100.0,
            110.0,
            0.0,
            "pcc-2",
            "sha256:recovery-receipt",
        )
        .unwrap(),
    ];
    let evidence = recovery_evidence(0, 3_600_000_000_000, 7_200_000_000_000, true);
    let report = evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points)
        .expect("event report");

    assert_eq!(report.gross_reduction_energy_kwh, 20.0);
    assert_eq!(report.gross_rebound_energy_kwh, 10.0);
    assert_eq!(report.net_reduction_energy_kwh, 10.0);
    assert_eq!(
        report.intervals[1].conservative_reduction_lower_bound_kw,
        -10.0
    );
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
    let report = qualify_counterfactual_baseline(&identity(), &contract(), &rows).expect("report");
    let point = DispatchEventPoint::new(
        "event-1",
        DispatchEventPhase::Dispatch,
        "normal",
        0,
        0,
        1_000,
        110.0,
        80.0,
        0.0,
        "pcc-1",
        "sha256:recovery-receipt",
    )
    .unwrap();
    let evidence = recovery_evidence(0, 1_000, 2_000, true);
    let error =
        evaluate_baseline_bounded_event(&report, &identity(), &evidence, &[point]).unwrap_err();

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
        DispatchEventPhase::Dispatch,
        "unknown-regime",
        0,
        0,
        1_000,
        100.0,
        90.0,
        0.0,
        "pcc-1",
        "sha256:recovery-receipt",
    )
    .unwrap();
    let evidence = recovery_evidence(0, 1_000, 2_000, true);
    let error =
        evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &[point]).unwrap_err();
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
        DispatchEventPhase::Dispatch,
        "normal",
        0,
        0,
        1_000,
        100.0,
        90.0,
        0.0,
        "pcc-2",
        "sha256:recovery-receipt",
    )
    .unwrap();
    let evidence = recovery_evidence(0, 1_000, 2_000, true);
    assert!(matches!(
        evaluate_baseline_bounded_event(&baseline, &other, &evidence, &[point]),
        Err(symtropy_counterfactual_baseline::BaselineQualificationError::BaselineIdentityMismatch)
    ));
}

#[test]
fn non_contiguous_recovery_horizon_is_rejected() {
    let baseline = qualified_report();
    let points = [
        DispatchEventPoint::new(
            "event-1",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            0,
            1_000,
            100.0,
            80.0,
            0.0,
            "pcc-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "recovery-1",
            DispatchEventPhase::Recovery,
            "normal",
            0,
            2_000,
            1_000,
            100.0,
            110.0,
            0.0,
            "pcc-2",
            "sha256:recovery-receipt",
        )
        .unwrap(),
    ];
    let evidence = recovery_evidence(0, 1_000, 3_000, true);
    let error =
        evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points).unwrap_err();

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
    assert_eq!(
        report.regime("normal").unwrap().lower_residual_quantile_kw,
        0.0
    );
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
        rows.push(
            NoDispatchObservation::new(
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
            )
            .unwrap(),
        );
    }
    let error = qualify_counterfactual_baseline(&identity(), &contract(), &rows).unwrap_err();
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
    rows.push(
        NoDispatchObservation::new(
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
        )
        .unwrap(),
    );
    for i in 1..20 {
        rows.push(observation(
            100 + i,
            &format!("eval-{i}"),
            BaselineFold::Evaluation,
            100.0,
            100.0,
        ));
    }
    let error = qualify_counterfactual_baseline(&identity(), &contract(), &rows).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::SourceDigestMismatch { .. }
    ));
}

#[test]
fn event_response_is_not_qualified_until_work_and_resource_recovery_are_reconciled() {
    let baseline = qualified_report_with_shape(2, 3_600_000_000_000);
    let points = [
        DispatchEventPoint::new(
            "event-1",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            0,
            3_600_000_000_000,
            100.0,
            80.0,
            0.0,
            "pcc-event-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "recovery-1",
            DispatchEventPhase::Recovery,
            "normal",
            0,
            3_600_000_000_000,
            3_600_000_000_000,
            100.0,
            100.0,
            0.0,
            "pcc-recovery-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
    ];
    let evidence = recovery_evidence(0, 3_600_000_000_000, 7_200_000_000_000, false);
    let report = evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points)
        .expect("event report with incomplete reconciliation");
    assert!(report.event_and_recovery_included_in_supplied_horizon);
    assert!(!report.workload_obligations_reconciled);
    assert!(!report.physical_resource_states_reconciled);
    assert!(!report.event_qualified);
    assert_eq!(report.dispatch_interval_count, 1);
    assert_eq!(report.recovery_interval_count, 1);
    assert_eq!(report.qualifying_interval_count, 1);
    assert!((report.net_reduction_energy_kwh - 20.0).abs() < 1e-12);
}

#[test]
fn event_cannot_qualify_without_at_least_one_recovery_interval() {
    let baseline = qualified_report();
    let points = [DispatchEventPoint::new(
        "dispatch-1",
        DispatchEventPhase::Dispatch,
        "normal",
        0,
        0,
        1_000,
        100.0,
        80.0,
        0.0,
        "pcc-dispatch-1",
        "sha256:recovery-receipt",
    )
    .unwrap()];
    let evidence = recovery_evidence(0, 1_000, 2_000, true);
    let error =
        evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::MissingRecoveryIntervals
    ));
}

#[test]
fn interval_may_not_straddle_the_dispatch_end_boundary() {
    let baseline = qualified_report();
    let points = [
        DispatchEventPoint::new(
            "dispatch-straddles",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            0,
            2_000,
            100.0,
            80.0,
            0.0,
            "pcc-straddles",
            "sha256:recovery-receipt",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "recovery-1",
            DispatchEventPhase::Recovery,
            "normal",
            0,
            2_000,
            1_000,
            100.0,
            100.0,
            0.0,
            "pcc-recovery-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
    ];
    let evidence = recovery_evidence(0, 1_000, 3_000, true);
    let error =
        evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::EventIntervalStraddlesDispatchEnd { .. }
    ));
}

#[test]
fn observed_trace_must_reach_the_pinned_recovery_horizon_end() {
    let baseline = qualified_report();
    let points = [
        DispatchEventPoint::new(
            "dispatch-1",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            0,
            1_000,
            100.0,
            80.0,
            0.0,
            "pcc-dispatch-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "recovery-1",
            DispatchEventPhase::Recovery,
            "normal",
            0,
            1_000,
            1_000,
            100.0,
            100.0,
            0.0,
            "pcc-recovery-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
    ];
    let evidence = recovery_evidence(0, 1_000, 3_000, true);
    let error =
        evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::EventHorizonEndMismatch { .. }
    ));
}

#[test]
fn dispatch_intervals_cannot_resume_after_recovery_has_started() {
    let baseline = qualified_report();
    let points = [
        DispatchEventPoint::new(
            "dispatch-1",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            0,
            1_000,
            100.0,
            80.0,
            0.0,
            "pcc-dispatch-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "recovery-1",
            DispatchEventPhase::Recovery,
            "normal",
            0,
            1_000,
            1_000,
            100.0,
            100.0,
            0.0,
            "pcc-recovery-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "dispatch-2",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            2_000,
            1_000,
            100.0,
            80.0,
            0.0,
            "pcc-dispatch-2",
            "sha256:recovery-receipt",
        )
        .unwrap(),
    ];
    let evidence = recovery_evidence(0, 1_000, 3_000, true);
    let error =
        evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::DispatchRecoveryPhaseOrderViolation { .. }
    ));
}

#[test]
fn baseline_calibration_group_must_be_contiguous() {
    let mut rows = Vec::new();
    for i in 0..40 {
        let group = format!("cal-{i}");
        rows.push(
            NoDispatchObservation::new(
                format!("cal-{i}-a"),
                &group,
                "normal",
                "sha256:calibration",
                0,
                i as u64 * 10_000,
                1_000,
                100.0,
                100.0,
                0.0,
                BaselineFold::Calibration,
                false,
            )
            .unwrap(),
        );
        rows.push(
            NoDispatchObservation::new(
                format!("cal-{i}-b"),
                &group,
                "normal",
                "sha256:calibration",
                0,
                i as u64 * 10_000 + 2_000,
                1_000,
                100.0,
                100.0,
                0.0,
                BaselineFold::Calibration,
                false,
            )
            .unwrap(),
        );
    }
    for i in 0..20 {
        rows.push(observation(
            100 + i,
            &format!("eval-{i}"),
            BaselineFold::Evaluation,
            100.0,
            100.0,
        ));
    }
    let error = qualify_counterfactual_baseline(&identity(), &contract(), &rows).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::NonContiguousBaselineTrajectory { .. }
    ));
}

#[test]
fn calibration_and_evaluation_groups_must_share_exact_horizon_shape() {
    let mut rows = Vec::new();
    for i in 0..40 {
        let group = format!("cal-{i}");
        rows.push(
            NoDispatchObservation::new(
                format!("cal-{i}-a"),
                &group,
                "normal",
                "sha256:calibration",
                0,
                i as u64 * 10_000,
                1_000,
                100.0,
                100.0,
                0.0,
                BaselineFold::Calibration,
                false,
            )
            .unwrap(),
        );
        if i != 0 {
            rows.push(
                NoDispatchObservation::new(
                    format!("cal-{i}-b"),
                    &group,
                    "normal",
                    "sha256:calibration",
                    0,
                    i as u64 * 10_000 + 1_000,
                    1_000,
                    100.0,
                    100.0,
                    0.0,
                    BaselineFold::Calibration,
                    false,
                )
                .unwrap(),
            );
        }
    }
    for i in 0..20 {
        rows.push(observation(
            100 + i,
            &format!("eval-{i}"),
            BaselineFold::Evaluation,
            100.0,
            100.0,
        ));
    }
    let error = qualify_counterfactual_baseline(&identity(), &contract(), &rows).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::BaselineTrajectoryShapeMismatch { .. }
    ));
}

#[test]
fn event_plus_recovery_must_match_qualified_baseline_trajectory_shape() {
    let baseline = qualified_report();
    let points = [
        DispatchEventPoint::new(
            "dispatch-1",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            0,
            1_000,
            100.0,
            80.0,
            0.0,
            "pcc-dispatch-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "recovery-1",
            DispatchEventPhase::Recovery,
            "normal",
            0,
            1_000,
            1_000,
            100.0,
            100.0,
            0.0,
            "pcc-recovery-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
    ];
    let evidence = recovery_evidence(0, 1_000, 2_000, true);
    let error =
        evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::EventTrajectoryShapeMismatch { .. }
    ));
}

#[test]
fn every_event_point_must_share_the_pinned_recovery_trace_digest() {
    let baseline = qualified_report_with_shape(2, 3_600_000_000_000);
    let points = [
        DispatchEventPoint::new(
            "dispatch-1",
            DispatchEventPhase::Dispatch,
            "normal",
            0,
            0,
            3_600_000_000_000,
            100.0,
            80.0,
            0.0,
            "pcc-dispatch-1",
            "sha256:wrong-source",
        )
        .unwrap(),
        DispatchEventPoint::new(
            "recovery-1",
            DispatchEventPhase::Recovery,
            "normal",
            0,
            3_600_000_000_000,
            3_600_000_000_000,
            100.0,
            100.0,
            0.0,
            "pcc-recovery-1",
            "sha256:recovery-receipt",
        )
        .unwrap(),
    ];
    let evidence = recovery_evidence(0, 3_600_000_000_000, 7_200_000_000_000, true);
    let error =
        evaluate_baseline_bounded_event(&baseline, &identity(), &evidence, &points).unwrap_err();
    assert!(matches!(
        error,
        symtropy_counterfactual_baseline::BaselineQualificationError::EventPointSourceDigestMismatch { .. }
    ));
}
