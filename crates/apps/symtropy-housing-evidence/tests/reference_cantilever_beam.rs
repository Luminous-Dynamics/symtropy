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
    use symtropy_housing_evidence::structural_reference::{
        cantilever_tip_load, Meters, Newtons, Pascals,
    };
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
    let declared_inertia = read_positive("second_moment_area_m4")?;
    let declared_extreme_fiber_distance = read_positive("extreme_fiber_distance_m")?;
    let tolerance = case["acceptance"]["relative_tolerance"]
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| "relative_tolerance must be finite and non-negative".to_string())?;

    let result = cantilever_tip_load(
        Newtons::try_from_si(load).map_err(|error| error.to_string())?,
        Meters::try_from_si(span).map_err(|error| error.to_string())?,
        Pascals::try_from_si(youngs_modulus).map_err(|error| error.to_string())?,
        Meters::try_from_si(width).map_err(|error| error.to_string())?,
        Meters::try_from_si(depth).map_err(|error| error.to_string())?,
    ).map_err(|error| error.to_string())?;
    if !close_relative(
        result.section.second_moment_area_m4.get(),
        declared_inertia,
        tolerance,
    ) {
        return Err(format!(
            "declared second moment of area {declared_inertia} disagrees with kernel value {}",
            result.section.second_moment_area_m4.get()
        ));
    }
    if !close_relative(
        result.section.extreme_fiber_distance_m.get(),
        declared_extreme_fiber_distance,
        tolerance,
    ) {
        return Err(format!(
            "declared extreme-fiber distance {declared_extreme_fiber_distance} disagrees with kernel value {}",
            result.section.extreme_fiber_distance_m.get()
        ));
    }

    Ok(BeamResult {
        inertia_m4: result.section.second_moment_area_m4.get(),
        deflection_m: result.maximum_deflection_m.get(),
        support_stress_pa: result.maximum_bending_stress_pa.get(),
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

#[test]
fn analytical_reference_rejects_inconsistent_declared_inertia() {
    let mut case: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let declared = case["inputs"]["second_moment_area_m4"]
        .as_f64()
        .expect("declared inertia");
    case["inputs"]["second_moment_area_m4"] = serde_json::json!(declared * 1.1);

    assert!(
        evaluate_reference(&case).is_err(),
        "a declared inertia that contradicts the section geometry must block evaluation"
    );
}

#[test]
fn analytical_reference_obeys_mechanical_scaling_invariants() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let reference = evaluate_reference(&base).expect("reference case is valid");

    let mut double_load = base.clone();
    double_load["inputs"]["load_N"] = serde_json::json!(2000.0);
    let load_result = evaluate_reference(&double_load).expect("positive doubled load is valid");
    assert!(close_relative(
        load_result.deflection_m,
        reference.deflection_m * 2.0,
        tolerance
    ));
    assert!(close_relative(
        load_result.support_stress_pa,
        reference.support_stress_pa * 2.0,
        tolerance
    ));

    let mut double_span = base.clone();
    double_span["inputs"]["span_m"] = serde_json::json!(4.0);
    let span_result = evaluate_reference(&double_span).expect("positive doubled span is valid");
    assert!(close_relative(
        span_result.deflection_m,
        reference.deflection_m * 8.0,
        tolerance
    ));
    assert!(close_relative(
        span_result.support_stress_pa,
        reference.support_stress_pa * 2.0,
        tolerance
    ));

    let mut double_modulus = base.clone();
    double_modulus["inputs"]["elastic_modulus_Pa"] = serde_json::json!(400_000_000_000.0);
    let modulus_result =
        evaluate_reference(&double_modulus).expect("positive doubled modulus is valid");
    assert!(close_relative(
        modulus_result.deflection_m,
        reference.deflection_m / 2.0,
        tolerance
    ));
    assert!(close_relative(
        modulus_result.support_stress_pa,
        reference.support_stress_pa,
        tolerance
    ));

    let mut double_width = base.clone();
    double_width["inputs"]["section_width_m"] = serde_json::json!(0.192);
    double_width["inputs"]["second_moment_area_m4"] = serde_json::json!(0.000016);
    let width_result =
        evaluate_reference(&double_width).expect("consistent doubled width is valid");
    assert!(close_relative(
        width_result.inertia_m4,
        reference.inertia_m4 * 2.0,
        tolerance
    ));
    assert!(close_relative(
        width_result.deflection_m,
        reference.deflection_m / 2.0,
        tolerance
    ));
    assert!(close_relative(
        width_result.support_stress_pa,
        reference.support_stress_pa / 2.0,
        tolerance
    ));

    let mut double_depth = base;
    double_depth["inputs"]["section_depth_m"] = serde_json::json!(0.2);
    double_depth["inputs"]["second_moment_area_m4"] = serde_json::json!(0.000064);
    double_depth["inputs"]["extreme_fiber_distance_m"] = serde_json::json!(0.1);
    let depth_result =
        evaluate_reference(&double_depth).expect("consistent doubled depth is valid");
    assert!(close_relative(
        depth_result.inertia_m4,
        reference.inertia_m4 * 8.0,
        tolerance
    ));
    assert!(close_relative(
        depth_result.deflection_m,
        reference.deflection_m / 8.0,
        tolerance
    ));
    assert!(close_relative(
        depth_result.support_stress_pa,
        reference.support_stress_pa / 4.0,
        tolerance
    ));
}


#[test]
fn analytical_reference_rejects_inconsistent_extreme_fiber_distance() {
    let mut case: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let declared = case["inputs"]["extreme_fiber_distance_m"]
        .as_f64()
        .expect("declared extreme-fiber distance");
    case["inputs"]["extreme_fiber_distance_m"] = serde_json::json!(declared * 1.1);

    assert!(
        evaluate_reference(&case).is_err(),
        "a declared extreme-fiber distance that contradicts the rectangular section geometry must block evaluation"
    );
}


#[test]
fn analytical_cantilever_survives_deterministic_scaling_sweep() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let reference = evaluate_reference(&base).expect("baseline case is valid");
    let factors = [0.25_f64, 0.5, 0.75, 1.25, 2.0, 4.0];

    let input = &base["inputs"];
    let load = input["load_N"].as_f64().unwrap();
    let span = input["span_m"].as_f64().unwrap();
    let modulus = input["elastic_modulus_Pa"].as_f64().unwrap();
    let width = input["section_width_m"].as_f64().unwrap();
    let depth = input["section_depth_m"].as_f64().unwrap();
    let inertia = input["second_moment_area_m4"].as_f64().unwrap();
    let fiber = input["extreme_fiber_distance_m"].as_f64().unwrap();

    for factor in factors {
        let mut changed_load = base.clone();
        changed_load["inputs"]["load_N"] = serde_json::json!(load * factor);
        let result = evaluate_reference(&changed_load).expect("scaled load is valid");
        assert!(close_relative(result.deflection_m, reference.deflection_m * factor, tolerance));
        assert!(close_relative(
            result.support_stress_pa,
            reference.support_stress_pa * factor,
            tolerance
        ));

        let mut changed_span = base.clone();
        changed_span["inputs"]["span_m"] = serde_json::json!(span * factor);
        let result = evaluate_reference(&changed_span).expect("scaled span is valid");
        assert!(close_relative(
            result.deflection_m,
            reference.deflection_m * factor.powi(3),
            tolerance
        ));
        assert!(close_relative(
            result.support_stress_pa,
            reference.support_stress_pa * factor,
            tolerance
        ));

        let mut changed_modulus = base.clone();
        changed_modulus["inputs"]["elastic_modulus_Pa"] =
            serde_json::json!(modulus * factor);
        let result = evaluate_reference(&changed_modulus).expect("scaled modulus is valid");
        assert!(close_relative(
            result.deflection_m,
            reference.deflection_m / factor,
            tolerance
        ));
        assert!(close_relative(
            result.support_stress_pa,
            reference.support_stress_pa,
            tolerance
        ));

        let mut changed_width = base.clone();
        changed_width["inputs"]["section_width_m"] = serde_json::json!(width * factor);
        changed_width["inputs"]["second_moment_area_m4"] =
            serde_json::json!(inertia * factor);
        let result = evaluate_reference(&changed_width).expect("consistent scaled width is valid");
        assert!(close_relative(result.inertia_m4, reference.inertia_m4 * factor, tolerance));
        assert!(close_relative(
            result.deflection_m,
            reference.deflection_m / factor,
            tolerance
        ));
        assert!(close_relative(
            result.support_stress_pa,
            reference.support_stress_pa / factor,
            tolerance
        ));

        let mut changed_depth = base.clone();
        changed_depth["inputs"]["section_depth_m"] = serde_json::json!(depth * factor);
        changed_depth["inputs"]["second_moment_area_m4"] =
            serde_json::json!(inertia * factor.powi(3));
        changed_depth["inputs"]["extreme_fiber_distance_m"] =
            serde_json::json!(fiber * factor);
        let result = evaluate_reference(&changed_depth).expect("consistent scaled depth is valid");
        assert!(close_relative(
            result.inertia_m4,
            reference.inertia_m4 * factor.powi(3),
            tolerance
        ));
        assert!(close_relative(
            result.deflection_m,
            reference.deflection_m / factor.powi(3),
            tolerance
        ));
        assert!(close_relative(
            result.support_stress_pa,
            reference.support_stress_pa / factor.powi(2),
            tolerance
        ));
    }
}


#[test]
fn analytical_cantilever_obeys_load_span_modulus_interactions() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let reference = evaluate_reference(&base).expect("baseline case is valid");
    let inputs = &base["inputs"];
    let load = inputs["load_N"].as_f64().unwrap();
    let span = inputs["span_m"].as_f64().unwrap();
    let modulus = inputs["elastic_modulus_Pa"].as_f64().unwrap();

    for load_factor in [0.5_f64, 2.0] {
        for span_factor in [0.5_f64, 2.0] {
            for modulus_factor in [0.5_f64, 2.0] {
                let mut changed = base.clone();
                changed["inputs"]["load_N"] = serde_json::json!(load * load_factor);
                changed["inputs"]["span_m"] = serde_json::json!(span * span_factor);
                changed["inputs"]["elastic_modulus_Pa"] =
                    serde_json::json!(modulus * modulus_factor);
                let result = evaluate_reference(&changed)
                    .expect("combined load/span/modulus transformation is valid");
                assert!(close_relative(
                    result.deflection_m,
                    reference.deflection_m * load_factor * span_factor.powi(3)
                        / modulus_factor,
                    tolerance
                ));
                assert!(close_relative(
                    result.support_stress_pa,
                    reference.support_stress_pa * load_factor * span_factor,
                    tolerance
                ));
                assert!(close_relative(
                    result.inertia_m4,
                    reference.inertia_m4,
                    tolerance
                ));
            }
        }
    }
}


#[test]
fn cantilever_reference_preserves_dimensionless_deflection_under_geometric_similarity() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let reference = evaluate_reference(&base).expect("baseline inputs are valid");
    let inputs = &base["inputs"];
    let load = inputs["load_N"].as_f64().unwrap();
    let span = inputs["span_m"].as_f64().unwrap();
    let width = inputs["section_width_m"].as_f64().unwrap();
    let depth = inputs["section_depth_m"].as_f64().unwrap();
    let inertia = inputs["second_moment_area_m4"].as_f64().unwrap();
    let fiber_distance = inputs["extreme_fiber_distance_m"].as_f64().unwrap();
    let reference_normalized_deflection = reference.deflection_m / span;

    // Geometric similarity scales every length by s, section inertia by s^4,
    // and point force by s^2 at constant E. Therefore delta/span and bending
    // stress are invariant. This is an analytical metamorphic test, not
    // physical validation of real beams outside the model assumptions.
    for scale in [0.5_f64, 0.75, 1.25, 2.0, 4.0] {
        let mut changed = base.clone();
        changed["inputs"]["load_N"] = serde_json::json!(load * scale.powi(2));
        changed["inputs"]["span_m"] = serde_json::json!(span * scale);
        changed["inputs"]["section_width_m"] = serde_json::json!(width * scale);
        changed["inputs"]["section_depth_m"] = serde_json::json!(depth * scale);
        changed["inputs"]["second_moment_area_m4"] =
            serde_json::json!(inertia * scale.powi(4));
        changed["inputs"]["extreme_fiber_distance_m"] =
            serde_json::json!(fiber_distance * scale);

        let result = evaluate_reference(&changed)
            .expect("similarity-scaled cantilever reference remains valid");
        assert!(close_relative(
            result.deflection_m / (span * scale),
            reference_normalized_deflection,
            tolerance
        ), "normalized tip deflection must be scale invariant at s={scale}");
        assert!(close_relative(result.support_stress_pa, reference.support_stress_pa, tolerance));
        assert!(close_relative(
            result.inertia_m4,
            reference.inertia_m4 * scale.powi(4),
            tolerance
        ));
    }
}

#[test]
fn cantilever_similarity_control_detects_missing_span_power_mutant() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let inputs = &base["inputs"];
    let load = inputs["load_N"].as_f64().unwrap();
    let span = inputs["span_m"].as_f64().unwrap();
    let modulus = inputs["elastic_modulus_Pa"].as_f64().unwrap();
    let inertia = inputs["second_moment_area_m4"].as_f64().unwrap();
    let scale = 2.0_f64;

    // Seeded mutant: delta = F * L^2 / (3 E I), omitting one span factor.
    // Under geometric similarity I scales as s^4 and force as s^2, so this
    // mutant incorrectly makes delta/span drift with the scale factor.
    let mutant_base = load * span.powi(2) / (3.0 * modulus * inertia);
    let mutant_scaled = (load * scale.powi(2) * (span * scale).powi(2))
        / (3.0 * modulus * (inertia * scale.powi(4)));
    assert!(
        !close_relative(
            mutant_scaled / (span * scale),
            mutant_base / span,
            tolerance
        ),
        "the missing-span-power cantilever mutant must violate normalized-deflection similarity"
    );
}
