// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic, synthetic numeric benchmark for the full optical-range to
//! route-coordinate transform. The oracle formulas below are independent of
//! the crate's route interpolation implementation.

use suf_fibre_calibration::{
    AxisOrder, CalibrationAnchor, CalibrationBinding, CalibrationModel, CoordinateFrame,
    CoordinateKind, CoordinateUnit, FibreDirection, FibreObservation, FibreTopology,
    PiecewiseLinearMap, RouteGeometry, RouteVertex, UncertaintyBudget,
};

const DIGEST: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn identity_map(anchors: &[(f64, f64)]) -> PiecewiseLinearMap {
    PiecewiseLinearMap::new(
        anchors
            .iter()
            .map(|(input_m, output_m)| CalibrationAnchor {
                input_m: *input_m,
                output_m: *output_m,
            })
            .collect(),
    )
    .expect("benchmark anchors are valid")
}

fn benchmark_model() -> CalibrationModel {
    let binding = CalibrationBinding {
        fibre_id: "utility-owner:fibre:synthetic-01".to_owned(),
        fibre_revision: "fibre-r1".to_owned(),
        route_revision: "route-r1".to_owned(),
        interrogator_config_revision: "interrogator-r1".to_owned(),
        calibration_revision: "calibration-r1".to_owned(),
        method_revision: "piecewise-linear-benchmark-v1".to_owned(),
        calibration_artifact_sha256: DIGEST.to_owned(),
        route_geometry_sha256: DIGEST.to_owned(),
        endpoint_a_id: "endpoint-a".to_owned(),
        endpoint_b_id: "endpoint-b".to_owned(),
        direction: FibreDirection::AToB,
        valid_from_unix_ms: 1_000,
        valid_until_unix_ms: 9_999_999,
    };
    let route = RouteGeometry::new(
        "route-r1".to_owned(),
        "endpoint-a".to_owned(),
        "endpoint-b".to_owned(),
        CoordinateFrame {
            crs_id: "urn:ogc:def:crs:EPSG::32735".to_owned(),
            kind: CoordinateKind::ProjectedMetric,
            axis_order: AxisOrder::EastingNorthing,
            unit: CoordinateUnit::Metre,
        },
        vec![
            RouteVertex {
                station_m: 0.0,
                easting_m: 10_000.0,
                northing_m: 20_000.0,
            },
            RouteVertex {
                station_m: 300.0,
                easting_m: 10_300.0,
                northing_m: 20_000.0,
            },
            RouteVertex {
                station_m: 700.0,
                easting_m: 10_300.0,
                northing_m: 20_400.0,
            },
            RouteVertex {
                station_m: 1_000.0,
                easting_m: 10_600.0,
                northing_m: 20_400.0,
            },
        ],
    )
    .expect("route vertices are valid");

    CalibrationModel::new(
        binding,
        FibreTopology::SingleSpan,
        identity_map(&[(0.0, 5.0), (400.0, 405.0), (1_000.0, 1_005.0)]),
        identity_map(&[(5.0, 0.0), (405.0, 400.0), (1_005.0, 1_000.0)]),
        route,
        UncertaintyBudget {
            // These are already horizontal-equivalent upper bounds for this
            // synthetic profile, not raw optical/fibre-axis uncertainties.
            optical_measurement_bound_m: 0.01,
            optical_to_fibre_bound_m: 0.02,
            fibre_to_route_bound_m: 0.03,
            route_geometry_bound_m: 0.04,
        },
    )
    .expect("synthetic calibration model is valid")
}

fn observation(optical_range_m: f64) -> FibreObservation {
    FibreObservation {
        observation_id: format!("synthetic-obs-{optical_range_m:.6}"),
        fibre_id: "utility-owner:fibre:synthetic-01".to_owned(),
        fibre_revision: "fibre-r1".to_owned(),
        route_revision: "route-r1".to_owned(),
        interrogator_config_revision: "interrogator-r1".to_owned(),
        direction: FibreDirection::AToB,
        endpoint_from_id: "endpoint-a".to_owned(),
        endpoint_to_id: "endpoint-b".to_owned(),
        phenomenon_time_unix_ms: 5_000,
        optical_range_m,
        source_sha256: DIGEST.to_owned(),
    }
}

/// Independent oracle: optical range x maps to route station x for the
/// piecewise calibration used in benchmark_model().
fn route_oracle(station_m: f64) -> (f64, f64) {
    if station_m <= 300.0 {
        (10_000.0 + station_m, 20_000.0)
    } else if station_m <= 700.0 {
        (10_300.0, 20_000.0 + (station_m - 300.0))
    } else {
        (10_300.0 + (station_m - 700.0), 20_400.0)
    }
}

#[test]
fn seeded_ten_thousand_point_route_transform_matches_independent_oracle() {
    let model = benchmark_model();
    let mut state = 0x5eed_u64;
    let mut max_easting_error = 0.0_f64;
    let mut max_northing_error = 0.0_f64;
    let mut max_chainage_error = 0.0_f64;
    let mut max_route_station_error = 0.0_f64;

    for index in 0..10_000_u64 {
        // Fixed LCG for reproducible coverage; no external RNG dependency.
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let fraction = (state >> 11) as f64 / ((1_u64 << 53) as f64);
        let optical = fraction * 1_000.0;
        let result = model
            .locate(&observation(optical))
            .expect("in-domain sample");
        let (expected_easting, expected_northing) = route_oracle(optical);

        max_easting_error = max_easting_error.max((result.easting_m - expected_easting).abs());
        max_northing_error = max_northing_error.max((result.northing_m - expected_northing).abs());
        max_chainage_error =
            max_chainage_error.max((result.physical_fibre_chainage_m - (optical + 5.0)).abs());
        max_route_station_error =
            max_route_station_error.max((result.route_station_m - optical).abs());

        assert_eq!(
            result.observation_id,
            format!("synthetic-obs-{optical:.6}"),
            "sample {index}"
        );
        assert!((result.horizontal_error_bound_m - 0.10).abs() < 1e-12);
    }

    assert!(
        max_easting_error < 1e-8,
        "max easting error {max_easting_error}"
    );
    assert!(
        max_northing_error < 1e-8,
        "max northing error {max_northing_error}"
    );
    assert!(
        max_chainage_error < 1e-8,
        "max chainage error {max_chainage_error}"
    );
    assert!(
        max_route_station_error < 1e-8,
        "max station error {max_route_station_error}"
    );
}

#[test]
fn exact_polyline_vertices_and_boundaries_are_stable() {
    let model = benchmark_model();
    for station in [0.0, 300.0, 700.0, 1_000.0] {
        let result = model
            .locate(&observation(station))
            .expect("boundary should be supported");
        let (expected_easting, expected_northing) = route_oracle(station);
        assert!((result.easting_m - expected_easting).abs() < 1e-9);
        assert!((result.northing_m - expected_northing).abs() < 1e-9);
    }
}

#[test]
fn seeded_monotone_input_produces_monotone_chainage_and_route_station() {
    let model = benchmark_model();
    let mut previous_chainage = f64::NEG_INFINITY;
    let mut previous_station = f64::NEG_INFINITY;
    for step in 0..=10_000 {
        let optical = step as f64 / 10.0;
        let result = model
            .locate(&observation(optical))
            .expect("in-domain sample");
        assert!(result.physical_fibre_chainage_m > previous_chainage);
        assert!(result.route_station_m > previous_station);
        previous_chainage = result.physical_fibre_chainage_m;
        previous_station = result.route_station_m;
    }
}
