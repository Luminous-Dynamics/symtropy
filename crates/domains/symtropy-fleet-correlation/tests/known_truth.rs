// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_fleet_correlation::{
    FleetCorrelationContract, FleetSiteSeries, evaluate_fleet_correlation,
};

fn site(site_id: &str, samples_kw: &[f64], clock_uncertainty_ns: u64) -> FleetSiteSeries {
    FleetSiteSeries::new(
        site_id,
        "region-a",
        "pcc",
        format!("sha256:{site_id}"),
        "alignment-v1",
        0,
        1_000_000_000,
        clock_uncertainty_ns,
        samples_kw.to_vec(),
    )
    .expect("valid site series")
}

fn contract(max_lag: usize, minimum_overlap: usize, threshold: f64) -> FleetCorrelationContract {
    FleetCorrelationContract::new(100, max_lag, minimum_overlap, threshold, 9_500)
        .expect("valid contract")
}

#[test]
fn independent_sites_match_independent_variance_identity() {
    let report = evaluate_fleet_correlation(
        &[
            site("a", &[-3.0, -1.0, 1.0, 3.0], 5),
            site("b", &[-1.0, 3.0, -3.0, 1.0], 5),
        ],
        &contract(0, 4, 2.0),
    )
    .expect("evaluation succeeds");

    assert!((report.portfolio_sample_variance_kw2 - 40.0 / 3.0).abs() < 1e-12);
    assert!((report.independent_site_variance_sum_kw2 - 40.0 / 3.0).abs() < 1e-12);
    assert!(report.covariance_excess_variance_kw2.abs() < 1e-12);
    assert!((report.variance_ratio_vs_independence.unwrap() - 1.0).abs() < 1e-12);
    assert!((report.variance_identity_residual_kw2).abs() < 1e-12);
}

#[test]
fn perfectly_correlated_sites_double_variance_vs_independence() {
    let report = evaluate_fleet_correlation(
        &[
            site("a", &[-3.0, -1.0, 1.0, 3.0], 5),
            site("b", &[-3.0, -1.0, 1.0, 3.0], 5),
        ],
        &contract(0, 4, 2.0),
    )
    .expect("evaluation succeeds");

    assert!((report.variance_ratio_vs_independence.unwrap() - 2.0).abs() < 1e-12);
    assert!(
        (report.pair_dependence[0]
            .contemporaneous_correlation
            .unwrap()
            - 1.0)
            .abs()
            < 1e-12
    );
}

#[test]
fn anti_correlated_sites_cancel_portfolio_variance() {
    let report = evaluate_fleet_correlation(
        &[
            site("a", &[-3.0, -1.0, 1.0, 3.0], 5),
            site("b", &[3.0, 1.0, -1.0, -3.0], 5),
        ],
        &contract(0, 4, 2.0),
    )
    .expect("evaluation succeeds");

    assert!(report.portfolio_sample_variance_kw2.abs() < 1e-12);
    assert_eq!(report.variance_ratio_vs_independence, Some(0.0));
    assert!(
        (report.pair_dependence[0]
            .contemporaneous_correlation
            .unwrap()
            + 1.0)
            .abs()
            < 1e-12
    );
}

#[test]
fn lagged_common_mode_is_visible_even_when_contemporaneous_correlation_is_weaker() {
    let report = evaluate_fleet_correlation(
        &[
            site("a", &[0.0, 1.0, 0.0, 2.0, 0.0, 3.0, 0.0, 4.0], 5),
            site("b", &[0.0, 0.0, 1.0, 0.0, 2.0, 0.0, 3.0, 0.0], 5),
        ],
        &contract(1, 7, 5.0),
    )
    .expect("evaluation succeeds");
    let pair = &report.pair_dependence[0];

    assert!((pair.strongest_positive_correlation.unwrap() - 1.0).abs() < 1e-12);
    assert_eq!(pair.strongest_positive_lag_intervals, Some(1));
    assert!(pair.contemporaneous_correlation.unwrap() < 1.0);
}

#[test]
fn correlated_peak_exceedance_and_portfolio_ramp_are_reported() {
    let report = evaluate_fleet_correlation(
        &[
            site("a", &[0.0, 10.0, 0.0, 10.0], 5),
            site("b", &[0.0, 10.0, 0.0, 10.0], 5),
        ],
        &contract(0, 4, 15.0),
    )
    .expect("evaluation succeeds");

    assert_eq!(report.portfolio_peak_power_kw, 20.0);
    assert_eq!(report.threshold_exceedance_fraction, 0.5);
    assert_eq!(report.maximum_absolute_portfolio_ramp_kw_per_second, 20.0);
    assert!(report.sub_interval_transients_unverified);
}

#[test]
fn excessive_pairwise_clock_uncertainty_is_rejected() {
    let error = evaluate_fleet_correlation(
        &[
            site("a", &[1.0, 2.0, 3.0], 60),
            site("b", &[1.0, 2.0, 3.0], 50),
        ],
        &contract(0, 3, 10.0),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_fleet_correlation::FleetCorrelationError::ClockUncertaintyExceedsContract {
            observed_ns: 110,
            maximum_ns: 100,
        }
    ));
}

#[test]
fn mismatched_start_or_interval_is_rejected() {
    let shifted = FleetSiteSeries::new(
        "b",
        "region-a",
        "pcc",
        "sha256:b",
        "alignment-v1",
        1,
        1_000_000_000,
        5,
        vec![1.0, 2.0, 3.0],
    )
    .unwrap();
    let error = evaluate_fleet_correlation(
        &[site("a", &[1.0, 2.0, 3.0], 5), shifted],
        &contract(0, 3, 10.0),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_fleet_correlation::FleetCorrelationError::TimestampAxisMismatch { .. }
    ));
}

#[test]
fn duplicate_site_identity_is_rejected() {
    let error = evaluate_fleet_correlation(
        &[
            site("duplicate", &[1.0, 2.0, 3.0], 5),
            site("duplicate", &[2.0, 3.0, 4.0], 5),
        ],
        &contract(0, 3, 10.0),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_fleet_correlation::FleetCorrelationError::DuplicateSiteId { .. }
    ));
}

#[test]
fn constant_series_returns_no_undefined_correlation() {
    let report = evaluate_fleet_correlation(
        &[
            site("a", &[10.0, 10.0, 10.0], 5),
            site("b", &[20.0, 20.0, 20.0], 5),
        ],
        &contract(0, 3, 40.0),
    )
    .expect("evaluation succeeds");

    assert_eq!(report.pair_dependence[0].contemporaneous_correlation, None);
    assert_eq!(report.variance_ratio_vs_independence, None);
    assert_eq!(report.maximum_absolute_portfolio_ramp_kw_per_second, 0.0);
}

#[test]
fn minimum_overlap_is_enforced_for_lagged_analysis() {
    let contract =
        FleetCorrelationContract::new(100, 3, 4, 10.0, 9_500).expect("contract policy is valid");
    let error = evaluate_fleet_correlation(
        &[
            site("a", &[1.0, 2.0, 3.0], 5),
            site("b", &[2.0, 3.0, 4.0], 5),
        ],
        &contract,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        symtropy_fleet_correlation::FleetCorrelationError::NoLagHasMinimumOverlap { .. }
    ));
}

#[test]
fn alignment_model_mismatch_is_rejected() {
    let other = FleetSiteSeries::new(
        "b",
        "region-a",
        "pcc",
        "sha256:b",
        "different-alignment-model",
        0,
        1_000_000_000,
        5,
        vec![1.0, 2.0, 3.0],
    )
    .unwrap();
    let error = evaluate_fleet_correlation(
        &[site("a", &[1.0, 2.0, 3.0], 5), other],
        &contract(0, 3, 10.0),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        symtropy_fleet_correlation::FleetCorrelationError::AlignmentModelMismatch { .. }
    ));
}
