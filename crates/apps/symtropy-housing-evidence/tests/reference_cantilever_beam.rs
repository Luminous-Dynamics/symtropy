// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Analytical reference test only. It does not validate an FEM solver or a
//! construction design. The fixture explicitly lists the beam model's limits.

use serde_json::Value;

const CASE: &str = include_str!(
    "../../../../docs/built-environment/benchmarks/cantilever-beam-linear-static-v1.json"
);

#[derive(Debug, Clone, Copy)]
struct BeamResult {
    inertia_m4: f64,
    deflection_m: f64,
    support_stress_pa: f64,
}

fn close_relative(actual: f64, expected: f64, tolerance: f64) -> bool {
    actual.is_finite()
        && expected.is_finite()
        && tolerance.is_finite()
        && tolerance >= 0.0
        && (actual - expected).abs() <= tolerance * expected.abs().max(f64::MIN_POSITIVE)
}

fn evaluate_reference(case: &Value) -> Result<BeamResult, String> {
    let inputs = case
        .get("inputs")
        .and_then(Value::as_object)
        .ok_or_else(|| "inputs must be an object".to_string())?;

    let read_positive = |name: &str| -> Result<f64, String> {
        let value = inputs
            .get(name)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("{name} must be numeric"))?;
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name} must be finite and greater than zero"));
        }
        Ok(value)
    };

    let load = read_positive("load_N")?;
    let span = read_positive("span_m")?;
    let youngs_modulus = read_positive("elastic_modulus_Pa")?;
    let width = read_positive("section_width_m")?;
    let depth = read_positive("section_depth_m")?;
    let inertia = read_positive("second_moment_area_m4")?;
    let c = read_positive("extreme_fiber_distance_m")?;

    let computed_inertia = width * depth.powi(3) / 12.0;
    let computed_deflection = load * span.powi(3) / (3.0 * youngs_modulus * inertia);
    let computed_stress = load * span * c / inertia;

    for (name, value) in [
        ("computed_inertia", computed_inertia),
        ("computed_deflection", computed_deflection),
        ("computed_stress", computed_stress),
    ] {
        if !value.is_finite() {
            return Err(format!("{name} is not finite"));
        }
    }

    Ok(BeamResult {
        inertia_m4: computed_inertia,
        deflection_m: computed_deflection,
        support_stress_pa: computed_stress,
    })
}

#[test]
fn analytical_cantilever_reference_matches_declared_oracle() {
    let case: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    assert_eq!(
        case["status"].as_str(),
        Some("analytical_reference_only"),
        "reference case must not imply construction approval"
    );

    let tolerance = case["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    assert!(tolerance.is_finite() && tolerance >= 0.0);

    let computed = evaluate_reference(&case).expect("positive reference inputs are valid");
    let expected = &case["expected"];

    assert!(
        close_relative(
            computed.inertia_m4,
            expected["second_moment_area_m4"].as_f64().unwrap(),
            tolerance,
        ),
        "section inertia mismatch: computed={}",
        computed.inertia_m4
    );
    assert!(
        close_relative(
            computed.deflection_m,
            expected["free_end_deflection_m"].as_f64().unwrap(),
            tolerance,
        ),
        "free-end deflection mismatch: computed={}",
        computed.deflection_m
    );
    assert!(
        close_relative(
            computed.support_stress_pa,
            expected["support_maximum_bending_stress_Pa"]
                .as_f64()
                .unwrap(),
            tolerance,
        ),
        "support bending stress mismatch: computed={}",
        computed.support_stress_pa
    );
}

#[test]
fn analytical_reference_rejects_nonpositive_physical_inputs() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");

    for (name, bad_value) in [
        ("load_N", 0.0),
        ("span_m", -1.0),
        ("elastic_modulus_Pa", 0.0),
        ("section_width_m", -0.096),
        ("section_depth_m", 0.0),
        ("second_moment_area_m4", -0.000008),
        ("extreme_fiber_distance_m", 0.0),
    ] {
        let mut invalid = base.clone();
        invalid["inputs"][name] = serde_json::json!(bad_value);
        let result = evaluate_reference(&invalid);
        assert!(result.is_err(), "{name}={bad_value} should be rejected");
    }
}
