use maritime_capability_to_service::conformal::{
    CalibrationContract, CalibrationObservation, CoverageApplicability, DecisionGate,
    assess_deadline_feasibility, build_interval, calibrate_absolute_residuals, validate_interval,
};

fn contract(applicability: CoverageApplicability) -> CalibrationContract {
    CalibrationContract {
        model_fit_cutoff: 10,
        calibration_window_start: 11,
        calibration_window_end: 40,
        decision_cutoff: 50,
        alpha_ppm: 100_000,
        model_sha256: "a".repeat(64),
        feature_schema_sha256: "b".repeat(64),
        calibration_data_sha256: "c".repeat(64),
        applicability,
    }
}

fn observations(n: usize) -> Vec<CalibrationObservation> {
    (0..n)
        .map(|index| CalibrationObservation {
            observation_id: format!("VESSEL-CALL-{index:03}"),
            prediction_issued_at: 12 + (index as u64 % 20),
            label_available_at: 35,
            predicted_value: 100,
            observed_value: 101 + index as u64,
        })
        .collect()
}

#[test]
fn public_api_computes_finite_sample_conformal_interval_and_deadline_gate() {
    let report = calibrate_absolute_residuals(
        &contract(CoverageApplicability::ExchangeabilitySupported),
        &observations(9),
    )
    .expect("public calibration API");
    let interval = build_interval(&report, 100)
        .expect("public interval builder")
        .expect("finite interval");

    assert_eq!(report.quantile_rank_1based, 9);
    assert_eq!(interval.lower, 91);
    assert_eq!(interval.upper, 109);
    assert_eq!(
        assess_deadline_feasibility(&interval, 10, 119),
        DecisionGate::Feasible
    );
    assert_eq!(
        assess_deadline_feasibility(&interval, 10, 118),
        DecisionGate::Infeasible
    );
}

#[test]
fn public_interval_validator_checks_rank_provenance_chronology_and_endpoints() {
    let report = calibrate_absolute_residuals(
        &contract(CoverageApplicability::ExchangeabilitySupported),
        &observations(9),
    )
    .expect("calibration");
    let mut interval = build_interval(&report, 100)
        .expect("interval")
        .expect("finite");
    assert!(validate_interval(&interval).is_ok());

    interval.quantile_rank_1based = 8;
    assert!(validate_interval(&interval).unwrap_err().contains("exact finite-sample"));

    let mut interval = build_interval(&report, 100)
        .expect("interval")
        .expect("finite");
    interval.calibration_window_end = interval.decision_cutoff + 1;
    assert!(validate_interval(&interval).unwrap_err().contains("provenance or chronology"));

    let mut interval = build_interval(&report, 100)
        .expect("interval")
        .expect("finite");
    interval.feature_schema_sha256 = "not-a-digest".to_owned();
    assert!(validate_interval(&interval).unwrap_err().contains("provenance or chronology"));

    let mut interval = build_interval(&report, 100)
        .expect("interval")
        .expect("finite");
    interval.upper += 1;
    assert!(validate_interval(&interval).unwrap_err().contains("endpoints"));
}

#[test]
fn public_api_keeps_small_sample_unbounded_and_shifted_coverage_unclaimed() {
    let small = calibrate_absolute_residuals(
        &contract(CoverageApplicability::ExchangeabilitySupported),
        &observations(8),
    )
    .expect("small-sample calibration");
    assert!(
        build_interval(&small, 100)
            .expect("interval result")
            .is_none()
    );

    let shifted = calibrate_absolute_residuals(
        &contract(CoverageApplicability::KnownDistributionShift),
        &observations(9),
    )
    .expect("shifted regime calibration arithmetic");
    assert_eq!(shifted.nominal_coverage_claim_ppm, None);
    let interval = build_interval(&shifted, 100)
        .expect("interval result")
        .expect("finite empirical interval");
    assert_eq!(
        assess_deadline_feasibility(&interval, 10, 200),
        DecisionGate::Indeterminate
    );
}
