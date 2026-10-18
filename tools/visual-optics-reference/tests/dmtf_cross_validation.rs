// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_visual_optics_reference::dmtf::{
    DmtfComparisonConfig, DmtfError, DmtfSample, compare_dmtf_methods,
};

fn sample(frequency: f64, angle: f64, depth: f64, a: f64, b: f64, uncertainty: f64) -> DmtfSample {
    DmtfSample {
        spatial_frequency_cycles_per_degree: frequency,
        view_angle_degrees: angle,
        depth_mm: depth,
        method_a_mtf: a,
        method_b_mtf: b,
        combined_uncertainty_abs: uncertainty,
    }
}

fn config() -> DmtfComparisonConfig {
    DmtfComparisonConfig {
        minimum_reference_mtf: 0.1,
        max_normalized_rms_percent: 2.0,
        minimum_points: 2,
    }
}

#[test]
fn independent_methods_agree_within_registered_gate_and_uncertainty() {
    let data = [
        sample(1.0, 0.0, 0.0, 0.80, 0.792, 0.01),
        sample(2.0, 0.0, 0.0, 0.60, 0.594, 0.01),
        sample(1.0, 20.0, 20.0, 0.50, 0.496, 0.01),
        sample(2.0, 20.0, 20.0, 0.30, 0.297, 0.01),
    ];
    let report = compare_dmtf_methods(&data, config()).expect("valid paired methods");

    assert!(report.passed);
    assert_eq!(report.points_evaluated, 4);
    assert_eq!(report.points_below_reference_floor, 0);
    assert_eq!(report.points_within_uncertainty, 4);
    assert_eq!(report.points_outside_uncertainty, 0);
    assert!(report.normalized_rms_error_percent <= 2.0);
}

#[test]
fn low_rms_does_not_pass_when_a_point_exceeds_combined_uncertainty() {
    let data = [
        sample(1.0, 0.0, 0.0, 0.80, 0.799, 0.01),
        sample(2.0, 0.0, 0.0, 0.60, 0.599, 0.01),
        sample(1.0, 20.0, 20.0, 0.50, 0.49, 0.001),
    ];
    let report = compare_dmtf_methods(&data, config()).expect("valid paired methods");

    assert!(report.normalized_rms_error_percent <= 2.0);
    assert_eq!(report.points_outside_uncertainty, 1);
    assert!(!report.passed);
}

#[test]
fn low_reference_points_are_counted_and_excluded_under_the_frozen_floor() {
    let data = [
        sample(1.0, 0.0, 0.0, 0.80, 0.798, 0.01),
        sample(2.0, 0.0, 0.0, 0.60, 0.598, 0.01),
        sample(3.0, 0.0, 0.0, 0.05, 0.90, 1.0),
    ];
    let report = compare_dmtf_methods(&data, config()).expect("two points meet the floor");

    assert_eq!(report.points_evaluated, 2);
    assert_eq!(report.points_below_reference_floor, 1);
    assert!(report.passed);
}

#[test]
fn malformed_values_and_duplicate_grid_points_fail_closed() {
    assert_eq!(
        compare_dmtf_methods(&[], config()),
        Err(DmtfError::EmptySamples)
    );
    assert_eq!(
        compare_dmtf_methods(&[sample(1.0, 0.0, 0.0, 0.8, 0.8, 0.01)], config()),
        Err(DmtfError::InsufficientInBandPoints)
    );
    assert_eq!(
        compare_dmtf_methods(
            &[sample(1.0, 0.0, 0.0, 0.8, 0.8, 0.01), sample(1.0, 0.0, 0.0, 0.8, 0.8, 0.01)],
            config(),
        ),
        Err(DmtfError::DuplicateGridPoint)
    );
    assert_eq!(
        compare_dmtf_methods(
            &[sample(1.0, 0.0, 0.0, f64::NAN, 0.8, 0.01), sample(2.0, 0.0, 0.0, 0.6, 0.6, 0.01)],
            config(),
        ),
        Err(DmtfError::InvalidMtf)
    );
    assert_eq!(
        compare_dmtf_methods(
            &[sample(1.0, 0.0, 0.0, 0.8, 0.8, -0.01), sample(2.0, 0.0, 0.0, 0.6, 0.6, 0.01)],
            config(),
        ),
        Err(DmtfError::InvalidUncertainty)
    );
}
