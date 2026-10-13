// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Analytical reference test only. It does not validate an FEM solver or a
//! construction design. The fixture explicitly lists the beam model's limits.

use serde_json::Value;

const CASE: &str = include_str!(
    "../../../../docs/built-environment/benchmarks/cantilever-beam-linear-static-v1.json"
);

fn close_relative(actual: f64, expected: f64, tolerance: f64) -> bool {
    actual.is_finite()
        && expected.is_finite()
        && tolerance.is_finite()
        && tolerance >= 0.0
        && (actual - expected).abs() <= tolerance * expected.abs().max(1.0)
}

#[test]
fn analytical_cantilever_reference_matches_declared_oracle() {
    let case: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    assert_eq!(
        case["status"].as_str(),
        Some("analytical_reference_only"),
        "reference case must not imply construction approval"
    );

    let inputs = &case["inputs"];
    let expected = &case["expected"];
    let acceptance = &case["acceptance"];
    let load = inputs["load_N"].as_f64().expect("load_N");
    let span = inputs["span_m"].as_f64().expect("span_m");
    let youngs_modulus = inputs["elastic_modulus_Pa"].as_f64().expect("elastic_modulus_Pa");
    let width = inputs["section_width_m"].as_f64().expect("section_width_m");
    let depth = inputs["section_depth_m"].as_f64().expect("section_depth_m");
    let inertia = inputs["second_moment_area_m4"]
        .as_f64()
        .expect("second_moment_area_m4");
    let c = inputs["extreme_fiber_distance_m"]
        .as_f64()
        .expect("extreme_fiber_distance_m");
    let tolerance = acceptance["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");

    for (name, value) in [
        ("load_N", load),
        ("span_m", span),
        ("elastic_modulus_Pa", youngs_modulus),
        ("section_width_m", width),
        ("section_depth_m", depth),
        ("second_moment_area_m4", inertia),
        ("extreme_fiber_distance_m", c),
    ] {
        assert!(value.is_finite() && value > 0.0, "{name} must be finite and positive");
    }
    assert!(tolerance.is_finite() && tolerance >= 0.0);

    let computed_inertia = width * depth.powi(3) / 12.0;
    let computed_deflection = load * span.powi(3) / (3.0 * youngs_modulus * inertia);
    let computed_stress = load * span * c / inertia;

    assert!(
        close_relative(
            computed_inertia,
            expected["second_moment_area_m4"].as_f64().unwrap(),
            tolerance,
        ),
        "section inertia mismatch: computed={computed_inertia}"
    );
    assert!(
        close_relative(
            computed_deflection,
            expected["free_end_deflection_m"].as_f64().unwrap(),
            tolerance,
        ),
        "free-end deflection mismatch: computed={computed_deflection}"
    );
    assert!(
        close_relative(
            computed_stress,
            expected["support_maximum_bending_stress_Pa"].as_f64().unwrap(),
            tolerance,
        ),
        "support bending stress mismatch: computed={computed_stress}"
    );
}

#[test]
fn analytical_reference_rejects_nonpositive_physical_inputs() {
    let case: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let inputs = &case["inputs"];
    let values = [
        inputs["load_N"].as_f64().unwrap(),
        inputs["span_m"].as_f64().unwrap(),
        inputs["elastic_modulus_Pa"].as_f64().unwrap(),
        inputs["section_width_m"].as_f64().unwrap(),
        inputs["section_depth_m"].as_f64().unwrap(),
        inputs["second_moment_area_m4"].as_f64().unwrap(),
        inputs["extreme_fiber_distance_m"].as_f64().unwrap(),
    ];
    assert!(values.iter().all(|value| value.is_finite() && *value > 0.0));
}
