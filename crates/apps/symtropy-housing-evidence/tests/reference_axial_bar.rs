// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Analytical reference tests only. They do not validate an FEM solver or a
//! construction design. See the fixture for the idealization and exclusions.

use serde_json::Value;

const CASE: &str = include_str!(
    "../../../../docs/built-environment/benchmarks/axial-bar-linear-static-v1.json"
);

#[derive(Debug, Clone, Copy)]
struct AxialResult {
    area_m2: f64,
    stress_pa: f64,
    extension_m: f64,
    reaction_n: f64,
}

fn close_relative(actual: f64, expected: f64, tolerance: f64) -> bool {
    actual.is_finite()
        && expected.is_finite()
        && tolerance.is_finite()
        && tolerance >= 0.0
        && (actual - expected).abs() <= tolerance * expected.abs().max(f64::MIN_POSITIVE)
}

fn evaluate_reference(case: &Value) -> Result<AxialResult, String> {
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
    let length = read_positive("length_m")?;
    let modulus = read_positive("elastic_modulus_Pa")?;
    let width = read_positive("section_width_m")?;
    let height = read_positive("section_height_m")?;
    let declared_area = read_positive("cross_section_area_m2")?;
    let tolerance = case["acceptance"]["relative_tolerance"]
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| "relative_tolerance must be finite and non-negative".to_string())?;

    // Calculate from primitive dimensions, not the duplicated area value.
    let area = width * height;
    if !area.is_finite() {
        return Err("geometry-derived cross-sectional area is not finite".to_string());
    }
    if !close_relative(area, declared_area, tolerance) {
        return Err(format!(
            "declared cross-sectional area {declared_area} disagrees with geometry-derived value {area}"
        ));
    }

    let stress = load / area;
    let extension = load * length / (modulus * area);
    let reaction = load;
    for (name, value) in [
        ("stress", stress),
        ("extension", extension),
        ("support reaction", reaction),
    ] {
        if !value.is_finite() {
            return Err(format!("computed {name} is not finite"));
        }
    }

    Ok(AxialResult {
        area_m2: area,
        stress_pa: stress,
        extension_m: extension,
        reaction_n: reaction,
    })
}

#[test]
fn analytical_axial_reference_matches_declared_oracle() {
    let case: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    assert_eq!(
        case["status"].as_str(),
        Some("analytical_reference_only"),
        "reference case must not imply construction approval"
    );
    let tolerance = case["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let computed = evaluate_reference(&case).expect("reference inputs are valid");
    let expected = &case["expected"];

    for (name, actual, expected_value) in [
        (
            "cross-section area",
            computed.area_m2,
            expected["cross_section_area_m2"].as_f64().unwrap(),
        ),
        (
            "axial stress",
            computed.stress_pa,
            expected["axial_stress_Pa"].as_f64().unwrap(),
        ),
        (
            "free-end extension",
            computed.extension_m,
            expected["free_end_extension_m"].as_f64().unwrap(),
        ),
        (
            "support reaction magnitude",
            computed.reaction_n,
            expected["support_reaction_magnitude_N"].as_f64().unwrap(),
        ),
    ] {
        assert!(
            close_relative(actual, expected_value, tolerance),
            "{name} mismatch: actual={actual}, expected={expected_value}"
        );
    }
}

#[test]
fn analytical_axial_reference_rejects_nonpositive_inputs() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    for (name, bad_value) in [
        ("load_N", 0.0),
        ("length_m", -3.0),
        ("elastic_modulus_Pa", 0.0),
        ("section_width_m", -0.05),
        ("section_height_m", 0.0),
        ("cross_section_area_m2", -0.005),
    ] {
        let mut invalid = base.clone();
        invalid["inputs"][name] = serde_json::json!(bad_value);
        assert!(
            evaluate_reference(&invalid).is_err(),
            "{name}={bad_value} should be rejected"
        );
    }
}

#[test]
fn analytical_axial_reference_rejects_inconsistent_area() {
    let mut case: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let area = case["inputs"]["cross_section_area_m2"]
        .as_f64()
        .expect("declared area");
    case["inputs"]["cross_section_area_m2"] = serde_json::json!(area * 1.1);

    assert!(
        evaluate_reference(&case).is_err(),
        "a declared area that contradicts section dimensions must block evaluation"
    );
}

#[test]
fn analytical_axial_reference_obeys_mechanical_scaling_invariants() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let reference = evaluate_reference(&base).expect("reference inputs are valid");

    let mut double_load = base.clone();
    double_load["inputs"]["load_N"] = serde_json::json!(24000.0);
    let load_result = evaluate_reference(&double_load).expect("doubled load is valid");
    assert!(close_relative(load_result.stress_pa, reference.stress_pa * 2.0, tolerance));
    assert!(close_relative(
        load_result.extension_m,
        reference.extension_m * 2.0,
        tolerance
    ));
    assert!(close_relative(load_result.reaction_n, reference.reaction_n * 2.0, tolerance));

    let mut double_length = base.clone();
    double_length["inputs"]["length_m"] = serde_json::json!(6.0);
    let length_result = evaluate_reference(&double_length).expect("doubled length is valid");
    assert!(close_relative(
        length_result.extension_m,
        reference.extension_m * 2.0,
        tolerance
    ));
    assert!(close_relative(length_result.stress_pa, reference.stress_pa, tolerance));

    let mut double_modulus = base.clone();
    double_modulus["inputs"]["elastic_modulus_Pa"] = serde_json::json!(400_000_000_000.0);
    let modulus_result = evaluate_reference(&double_modulus).expect("doubled modulus is valid");
    assert!(close_relative(
        modulus_result.extension_m,
        reference.extension_m / 2.0,
        tolerance
    ));
    assert!(close_relative(modulus_result.stress_pa, reference.stress_pa, tolerance));

    let mut double_width = base.clone();
    double_width["inputs"]["section_width_m"] = serde_json::json!(0.1);
    double_width["inputs"]["cross_section_area_m2"] = serde_json::json!(0.01);
    let width_result =
        evaluate_reference(&double_width).expect("consistent doubled width is valid");
    assert!(close_relative(width_result.area_m2, reference.area_m2 * 2.0, tolerance));
    assert!(close_relative(width_result.stress_pa, reference.stress_pa / 2.0, tolerance));
    assert!(close_relative(
        width_result.extension_m,
        reference.extension_m / 2.0,
        tolerance
    ));

    let mut double_height = base;
    double_height["inputs"]["section_height_m"] = serde_json::json!(0.2);
    double_height["inputs"]["cross_section_area_m2"] = serde_json::json!(0.01);
    let height_result =
        evaluate_reference(&double_height).expect("consistent doubled height is valid");
    assert!(close_relative(
        height_result.area_m2,
        reference.area_m2 * 2.0,
        tolerance
    ));
    assert!(close_relative(
        height_result.stress_pa,
        reference.stress_pa / 2.0,
        tolerance
    ));
    assert!(close_relative(
        height_result.extension_m,
        reference.extension_m / 2.0,
        tolerance
    ));
}


#[test]
fn analytical_axial_bar_survives_deterministic_scaling_sweep() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let reference = evaluate_reference(&base).expect("baseline case is valid");
    let inputs = &base["inputs"];
    let load = inputs["load_N"].as_f64().unwrap();
    let length = inputs["length_m"].as_f64().unwrap();
    let modulus = inputs["elastic_modulus_Pa"].as_f64().unwrap();
    let width = inputs["section_width_m"].as_f64().unwrap();
    let height = inputs["section_height_m"].as_f64().unwrap();
    let area = inputs["cross_section_area_m2"].as_f64().unwrap();
    let factors = [0.25_f64, 0.5, 0.75, 1.25, 2.0, 4.0];

    for factor in factors {
        let mut changed_load = base.clone();
        changed_load["inputs"]["load_N"] = serde_json::json!(load * factor);
        let result = evaluate_reference(&changed_load).expect("scaled load is valid");
        assert!(close_relative(result.stress_pa, reference.stress_pa * factor, tolerance));
        assert!(close_relative(
            result.extension_m,
            reference.extension_m * factor,
            tolerance
        ));
        assert!(close_relative(result.reaction_n, reference.reaction_n * factor, tolerance));

        let mut changed_length = base.clone();
        changed_length["inputs"]["length_m"] = serde_json::json!(length * factor);
        let result = evaluate_reference(&changed_length).expect("scaled length is valid");
        assert!(close_relative(
            result.extension_m,
            reference.extension_m * factor,
            tolerance
        ));
        assert!(close_relative(result.stress_pa, reference.stress_pa, tolerance));
        assert!(close_relative(result.reaction_n, reference.reaction_n, tolerance));

        let mut changed_modulus = base.clone();
        changed_modulus["inputs"]["elastic_modulus_Pa"] =
            serde_json::json!(modulus * factor);
        let result = evaluate_reference(&changed_modulus).expect("scaled modulus is valid");
        assert!(close_relative(
            result.extension_m,
            reference.extension_m / factor,
            tolerance
        ));
        assert!(close_relative(result.stress_pa, reference.stress_pa, tolerance));
        assert!(close_relative(result.reaction_n, reference.reaction_n, tolerance));

        let mut changed_width = base.clone();
        changed_width["inputs"]["section_width_m"] = serde_json::json!(width * factor);
        changed_width["inputs"]["cross_section_area_m2"] = serde_json::json!(area * factor);
        let result = evaluate_reference(&changed_width).expect("consistent scaled width is valid");
        assert!(close_relative(result.area_m2, reference.area_m2 * factor, tolerance));
        assert!(close_relative(result.stress_pa, reference.stress_pa / factor, tolerance));
        assert!(close_relative(
            result.extension_m,
            reference.extension_m / factor,
            tolerance
        ));
        assert!(close_relative(result.reaction_n, reference.reaction_n, tolerance));

        let mut changed_height = base.clone();
        changed_height["inputs"]["section_height_m"] = serde_json::json!(height * factor);
        changed_height["inputs"]["cross_section_area_m2"] = serde_json::json!(area * factor);
        let result =
            evaluate_reference(&changed_height).expect("consistent scaled height is valid");
        assert!(close_relative(result.area_m2, reference.area_m2 * factor, tolerance));
        assert!(close_relative(result.stress_pa, reference.stress_pa / factor, tolerance));
        assert!(close_relative(
            result.extension_m,
            reference.extension_m / factor,
            tolerance
        ));
        assert!(close_relative(result.reaction_n, reference.reaction_n, tolerance));
    }
}


#[test]
fn analytical_axial_bar_obeys_load_length_modulus_area_interactions() {
    let base: Value = serde_json::from_str(CASE).expect("reference case must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let reference = evaluate_reference(&base).expect("baseline case is valid");
    let inputs = &base["inputs"];
    let load = inputs["load_N"].as_f64().unwrap();
    let length = inputs["length_m"].as_f64().unwrap();
    let modulus = inputs["elastic_modulus_Pa"].as_f64().unwrap();
    let width = inputs["section_width_m"].as_f64().unwrap();
    let area = inputs["cross_section_area_m2"].as_f64().unwrap();

    for load_factor in [0.5_f64, 2.0] {
        for length_factor in [0.5_f64, 2.0] {
            for modulus_factor in [0.5_f64, 2.0] {
                for area_factor in [0.5_f64, 2.0] {
                    let mut changed = base.clone();
                    changed["inputs"]["load_N"] = serde_json::json!(load * load_factor);
                    changed["inputs"]["length_m"] =
                        serde_json::json!(length * length_factor);
                    changed["inputs"]["elastic_modulus_Pa"] =
                        serde_json::json!(modulus * modulus_factor);
                    changed["inputs"]["section_width_m"] =
                        serde_json::json!(width * area_factor);
                    changed["inputs"]["cross_section_area_m2"] =
                        serde_json::json!(area * area_factor);
                    let result = evaluate_reference(&changed)
                        .expect("combined load/length/modulus/area transform is valid");
                    assert!(close_relative(
                        result.stress_pa,
                        reference.stress_pa * load_factor / area_factor,
                        tolerance
                    ));
                    assert!(close_relative(
                        result.extension_m,
                        reference.extension_m * load_factor * length_factor
                            / (modulus_factor * area_factor),
                        tolerance
                    ));
                    assert!(close_relative(
                        result.reaction_n,
                        reference.reaction_n * load_factor,
                        tolerance
                    ));
                }
            }
        }
    }
}
