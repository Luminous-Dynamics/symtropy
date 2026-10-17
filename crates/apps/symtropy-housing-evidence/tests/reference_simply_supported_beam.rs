// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Analytical reference checks for simple-span beam load cases.
//! They do not validate an FEM solver or a construction design.

use serde_json::Value;

const CASE: &str = include_str!(
    "../../../../docs/built-environment/benchmarks/simply-supported-beam-linear-static-v1.json"
);

#[derive(Debug, Clone, Copy)]
struct BeamResult {
    deflection_m: f64,
    maximum_moment_nm: f64,
    maximum_stress_pa: f64,
    reaction_magnitude_n: f64,
}

fn close_relative(actual: f64, expected: f64, tolerance: f64) -> bool {
    actual.is_finite()
        && expected.is_finite()
        && tolerance.is_finite()
        && tolerance >= 0.0
        && (actual - expected).abs() <= tolerance * expected.abs().max(f64::MIN_POSITIVE)
}

fn positive(object: &Value, name: &str) -> Result<f64, String> {
    let value = object
        .get(name)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{name} must be numeric"))?;
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{name} must be finite and greater than zero"));
    }
    Ok(value)
}

fn evaluate_reference(case: &Value, load_case: &Value) -> Result<BeamResult, String> {
    use symtropy_housing_evidence::structural_reference::{
        simply_supported_beam, Meters, Newtons, NewtonsPerMeter, Pascals,
        SimplySupportedLoadCase,
    };
    let inputs = case
        .get("inputs")
        .ok_or_else(|| "inputs must be an object".to_string())?;
    let span = positive(inputs, "span_m")?;
    let modulus = positive(inputs, "elastic_modulus_Pa")?;
    let width = positive(inputs, "section_width_m")?;
    let depth = positive(inputs, "section_depth_m")?;
    let declared_inertia = positive(inputs, "second_moment_area_m4")?;
    let declared_extreme_fiber_distance = positive(inputs, "extreme_fiber_distance_m")?;
    let tolerance = case["acceptance"]["relative_tolerance"]
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| "relative_tolerance must be finite and non-negative".to_string())?;

    let load_case = match load_case.get("load_type").and_then(Value::as_str) {
        Some("point_load_midspan") => SimplySupportedLoadCase::MidpointPointLoad {
            force_n: Newtons::try_from_si(positive(load_case, "load_N")?)
                .map_err(|error| error.to_string())?,
        },
        Some("uniform_distributed_load") => SimplySupportedLoadCase::UniformDistributedLoad {
            load_n_per_m: NewtonsPerMeter::try_from_si(positive(load_case, "load_N_per_m")?)
                .map_err(|error| error.to_string())?,
        },
        Some(other) => return Err(format!("unsupported load_type {other:?}")),
        None => return Err("load_type must be a string".to_string()),
    };

    let result = simply_supported_beam(
        Meters::try_from_si(span).map_err(|error| error.to_string())?,
        Pascals::try_from_si(modulus).map_err(|error| error.to_string())?,
        Meters::try_from_si(width).map_err(|error| error.to_string())?,
        Meters::try_from_si(depth).map_err(|error| error.to_string())?,
        load_case,
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
        deflection_m: result.maximum_deflection_m.get(),
        maximum_moment_nm: result.maximum_bending_moment_nm.get(),
        maximum_stress_pa: result.maximum_bending_stress_pa.get(),
        reaction_magnitude_n: result.support_reaction_magnitude_n.get(),
    })
}

#[test]
fn analytical_simply_supported_beam_matches_both_load_oracles() {
    let case: Value = serde_json::from_str(CASE).expect("reference fixture must be valid JSON");
    assert_eq!(
        case["status"].as_str(),
        Some("analytical_reference_only"),
        "reference fixture must not imply construction approval"
    );
    let tolerance = case["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let cases = case["load_cases"].as_array().expect("load_cases array");
    assert_eq!(cases.len(), 2, "both predeclared load cases are required");

    for load_case in cases {
        let id = load_case["case_id"].as_str().expect("case_id");
        let actual = evaluate_reference(&case, load_case)
            .unwrap_or_else(|error| panic!("{id} should evaluate: {error}"));
        assert_matches_expected(actual, &load_case["expected"], tolerance);
    }
}

#[test]
fn analytical_simply_supported_beam_rejects_invalid_geometry_and_loads() {
    let base: Value = serde_json::from_str(CASE).expect("reference fixture must be valid JSON");
    for (name, bad_value) in [
        ("span_m", 0.0),
        ("elastic_modulus_Pa", -1.0),
        ("section_width_m", 0.0),
        ("section_depth_m", -0.2),
        ("second_moment_area_m4", 0.0),
        ("extreme_fiber_distance_m", -0.1),
    ] {
        let mut invalid = base.clone();
        invalid["inputs"][name] = serde_json::json!(bad_value);
        assert!(
            evaluate_reference(&invalid, &invalid["load_cases"][0]).is_err(),
            "{name}={bad_value} should be rejected"
        );
    }

    let mut bad_point = base.clone();
    bad_point["load_cases"][0]["load_N"] = serde_json::json!(0.0);
    assert!(evaluate_reference(&bad_point, &bad_point["load_cases"][0]).is_err());

    let mut bad_uniform = base.clone();
    bad_uniform["load_cases"][1]["load_N_per_m"] = serde_json::json!(-1.0);
    assert!(evaluate_reference(&bad_uniform, &bad_uniform["load_cases"][1]).is_err());

    let mut inconsistent = base.clone();
    inconsistent["inputs"]["second_moment_area_m4"] = serde_json::json!(0.00007);
    assert!(evaluate_reference(&inconsistent, &inconsistent["load_cases"][0]).is_err());
}

#[test]
fn analytical_simply_supported_beam_obeys_load_and_span_scaling() {
    let base: Value = serde_json::from_str(CASE).expect("reference fixture must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");

    let point_reference =
        evaluate_reference(&base, &base["load_cases"][0]).expect("point baseline");
    let uniform_reference =
        evaluate_reference(&base, &base["load_cases"][1]).expect("uniform baseline");

    let mut doubled_load = base.clone();
    doubled_load["load_cases"][0]["load_N"] = serde_json::json!(10000.0);
    let point_double = evaluate_reference(&doubled_load, &doubled_load["load_cases"][0])
        .expect("doubled point load");
    assert!(close_relative(
        point_double.deflection_m,
        point_reference.deflection_m * 2.0,
        tolerance
    ));
    assert!(close_relative(
        point_double.maximum_stress_pa,
        point_reference.maximum_stress_pa * 2.0,
        tolerance
    ));
    assert!(close_relative(
        point_double.reaction_magnitude_n,
        point_reference.reaction_magnitude_n * 2.0,
        tolerance
    ));

    let mut doubled_uniform = base.clone();
    doubled_uniform["load_cases"][1]["load_N_per_m"] = serde_json::json!(2000.0);
    let uniform_double =
        evaluate_reference(&doubled_uniform, &doubled_uniform["load_cases"][1])
            .expect("doubled uniform load");
    assert!(close_relative(
        uniform_double.deflection_m,
        uniform_reference.deflection_m * 2.0,
        tolerance
    ));
    assert!(close_relative(
        uniform_double.maximum_stress_pa,
        uniform_reference.maximum_stress_pa * 2.0,
        tolerance
    ));
    assert!(close_relative(
        uniform_double.reaction_magnitude_n,
        uniform_reference.reaction_magnitude_n * 2.0,
        tolerance
    ));

    let mut doubled_span = base.clone();
    doubled_span["inputs"]["span_m"] = serde_json::json!(8.0);
    let point_span = evaluate_reference(&doubled_span, &doubled_span["load_cases"][0])
        .expect("point load with doubled span");
    assert!(close_relative(point_span.deflection_m, point_reference.deflection_m * 8.0, tolerance));
    assert!(close_relative(
        point_span.maximum_moment_nm,
        point_reference.maximum_moment_nm * 2.0,
        tolerance
    ));
    assert!(close_relative(
        point_span.reaction_magnitude_n,
        point_reference.reaction_magnitude_n,
        tolerance
    ));

    let uniform_span = evaluate_reference(&doubled_span, &doubled_span["load_cases"][1])
        .expect("uniform load with doubled span");
    assert!(close_relative(
        uniform_span.deflection_m,
        uniform_reference.deflection_m * 16.0,
        tolerance
    ));
    assert!(close_relative(
        uniform_span.maximum_moment_nm,
        uniform_reference.maximum_moment_nm * 4.0,
        tolerance
    ));
    assert!(close_relative(
        uniform_span.reaction_magnitude_n,
        uniform_reference.reaction_magnitude_n * 2.0,
        tolerance
    ));
}


#[test]
fn analytical_simply_supported_beam_survives_deterministic_scaling_sweep() {
    let base: Value = serde_json::from_str(CASE).expect("reference fixture must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let inputs = &base["inputs"];
    let span = inputs["span_m"].as_f64().unwrap();
    let modulus = inputs["elastic_modulus_Pa"].as_f64().unwrap();
    let width = inputs["section_width_m"].as_f64().unwrap();
    let depth = inputs["section_depth_m"].as_f64().unwrap();
    let inertia = inputs["second_moment_area_m4"].as_f64().unwrap();
    let fiber = inputs["extreme_fiber_distance_m"].as_f64().unwrap();
    let factors = [0.25_f64, 0.5, 0.75, 1.25, 2.0, 4.0];

    for (case_index, base_case) in base["load_cases"]
        .as_array()
        .expect("load_cases array")
        .iter()
        .enumerate()
    {
        let reference =
            evaluate_reference(&base, base_case).expect("baseline load case is valid");
        for factor in factors {
            let mut changed_load = base.clone();
            match base_case["load_type"].as_str() {
                Some("point_load_midspan") => {
                    let load = base_case["load_N"].as_f64().unwrap();
                    changed_load["load_cases"][case_index]["load_N"] =
                        serde_json::json!(load * factor);
                }
                Some("uniform_distributed_load") => {
                    let load = base_case["load_N_per_m"].as_f64().unwrap();
                    changed_load["load_cases"][case_index]["load_N_per_m"] =
                        serde_json::json!(load * factor);
                }
                other => panic!("unsupported load type {other:?}"),
            }
            let result = evaluate_reference(
                &changed_load,
                &changed_load["load_cases"][case_index],
            )
            .expect("scaled load is valid");
            assert!(close_relative(
                result.deflection_m,
                reference.deflection_m * factor,
                tolerance
            ));
            assert!(close_relative(
                result.maximum_moment_nm,
                reference.maximum_moment_nm * factor,
                tolerance
            ));
            assert!(close_relative(
                result.maximum_stress_pa,
                reference.maximum_stress_pa * factor,
                tolerance
            ));
            assert!(close_relative(
                result.reaction_magnitude_n,
                reference.reaction_magnitude_n * factor,
                tolerance
            ));

            let mut changed_span = base.clone();
            changed_span["inputs"]["span_m"] = serde_json::json!(span * factor);
            let result = evaluate_reference(
                &changed_span,
                &changed_span["load_cases"][case_index],
            )
            .expect("scaled span is valid");
            let (deflection_power, moment_power, reaction_power) =
                match base_case["load_type"].as_str() {
                    Some("point_load_midspan") => (3, 1, 0),
                    Some("uniform_distributed_load") => (4, 2, 1),
                    other => panic!("unsupported load type {other:?}"),
                };
            assert!(close_relative(
                result.deflection_m,
                reference.deflection_m * factor.powi(deflection_power),
                tolerance
            ));
            assert!(close_relative(
                result.maximum_moment_nm,
                reference.maximum_moment_nm * factor.powi(moment_power),
                tolerance
            ));
            assert!(close_relative(
                result.reaction_magnitude_n,
                reference.reaction_magnitude_n * factor.powi(reaction_power),
                tolerance
            ));

            let mut changed_modulus = base.clone();
            changed_modulus["inputs"]["elastic_modulus_Pa"] =
                serde_json::json!(modulus * factor);
            let result = evaluate_reference(
                &changed_modulus,
                &changed_modulus["load_cases"][case_index],
            )
            .expect("scaled modulus is valid");
            assert!(close_relative(
                result.deflection_m,
                reference.deflection_m / factor,
                tolerance
            ));
            assert!(close_relative(
                result.maximum_moment_nm,
                reference.maximum_moment_nm,
                tolerance
            ));
            assert!(close_relative(
                result.maximum_stress_pa,
                reference.maximum_stress_pa,
                tolerance
            ));
            assert!(close_relative(
                result.reaction_magnitude_n,
                reference.reaction_magnitude_n,
                tolerance
            ));

            let mut changed_width = base.clone();
            changed_width["inputs"]["section_width_m"] =
                serde_json::json!(width * factor);
            changed_width["inputs"]["second_moment_area_m4"] =
                serde_json::json!(inertia * factor);
            let result = evaluate_reference(
                &changed_width,
                &changed_width["load_cases"][case_index],
            )
            .expect("consistent scaled width is valid");
            assert!(close_relative(
                result.deflection_m,
                reference.deflection_m / factor,
                tolerance
            ));
            assert!(close_relative(
                result.maximum_stress_pa,
                reference.maximum_stress_pa / factor,
                tolerance
            ));
            assert!(close_relative(
                result.maximum_moment_nm,
                reference.maximum_moment_nm,
                tolerance
            ));
            assert!(close_relative(
                result.reaction_magnitude_n,
                reference.reaction_magnitude_n,
                tolerance
            ));

            let mut changed_depth = base.clone();
            changed_depth["inputs"]["section_depth_m"] =
                serde_json::json!(depth * factor);
            changed_depth["inputs"]["second_moment_area_m4"] =
                serde_json::json!(inertia * factor.powi(3));
            changed_depth["inputs"]["extreme_fiber_distance_m"] =
                serde_json::json!(fiber * factor);
            let result = evaluate_reference(
                &changed_depth,
                &changed_depth["load_cases"][case_index],
            )
            .expect("consistent scaled depth is valid");
            assert!(close_relative(
                result.deflection_m,
                reference.deflection_m / factor.powi(3),
                tolerance
            ));
            assert!(close_relative(
                result.maximum_stress_pa,
                reference.maximum_stress_pa / factor.powi(2),
                tolerance
            ));
            assert!(close_relative(
                result.maximum_moment_nm,
                reference.maximum_moment_nm,
                tolerance
            ));
            assert!(close_relative(
                result.reaction_magnitude_n,
                reference.reaction_magnitude_n,
                tolerance
            ));
        }
    }
}


#[test]
fn analytical_simply_supported_beam_obeys_load_span_modulus_interactions() {
    let base: Value = serde_json::from_str(CASE).expect("reference fixture must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let span = base["inputs"]["span_m"].as_f64().unwrap();
    let modulus = base["inputs"]["elastic_modulus_Pa"].as_f64().unwrap();

    for (case_index, base_case) in base["load_cases"]
        .as_array()
        .expect("load_cases array")
        .iter()
        .enumerate()
    {
        let reference =
            evaluate_reference(&base, base_case).expect("baseline load case is valid");
        let is_point_load = base_case["load_type"].as_str() == Some("point_load_midspan");
        let (deflection_span_power, moment_span_power, reaction_span_power) =
            if is_point_load { (3, 1, 0) } else { (4, 2, 1) };

        for load_factor in [0.5_f64, 2.0] {
            for span_factor in [0.5_f64, 2.0] {
                for modulus_factor in [0.5_f64, 2.0] {
                    let mut changed = base.clone();
                    if is_point_load {
                        let load = base_case["load_N"].as_f64().unwrap();
                        changed["load_cases"][case_index]["load_N"] =
                            serde_json::json!(load * load_factor);
                    } else {
                        let load = base_case["load_N_per_m"].as_f64().unwrap();
                        changed["load_cases"][case_index]["load_N_per_m"] =
                            serde_json::json!(load * load_factor);
                    }
                    changed["inputs"]["span_m"] = serde_json::json!(span * span_factor);
                    changed["inputs"]["elastic_modulus_Pa"] =
                        serde_json::json!(modulus * modulus_factor);

                    let result = evaluate_reference(
                        &changed,
                        &changed["load_cases"][case_index],
                    )
                    .expect("combined load/span/modulus transformation is valid");
                    assert!(close_relative(
                        result.deflection_m,
                        reference.deflection_m * load_factor
                            * span_factor.powi(deflection_span_power)
                            / modulus_factor,
                        tolerance
                    ));
                    assert!(close_relative(
                        result.maximum_moment_nm,
                        reference.maximum_moment_nm * load_factor
                            * span_factor.powi(moment_span_power),
                        tolerance
                    ));
                    assert!(close_relative(
                        result.maximum_stress_pa,
                        reference.maximum_stress_pa * load_factor
                            * span_factor.powi(moment_span_power),
                        tolerance
                    ));
                    assert!(close_relative(
                        result.reaction_magnitude_n,
                        reference.reaction_magnitude_n * load_factor
                            * span_factor.powi(reaction_span_power),
                        tolerance
                    ));
                }
            }
        }
    }
}


#[test]
fn simply_supported_reference_preserves_dimensionless_deflection_under_geometric_similarity() {
    let base: Value = serde_json::from_str(CASE).expect("reference fixture must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let base_inputs = &base["inputs"];
    let span = base_inputs["span_m"].as_f64().unwrap();
    let width = base_inputs["section_width_m"].as_f64().unwrap();
    let depth = base_inputs["section_depth_m"].as_f64().unwrap();
    let inertia = base_inputs["second_moment_area_m4"].as_f64().unwrap();
    let fiber_distance = base_inputs["extreme_fiber_distance_m"].as_f64().unwrap();
    let load_cases = base["load_cases"].as_array().expect("load_cases array");
    assert_eq!(load_cases.len(), 2, "both load families must be covered");

    // Under geometric similarity, span/depth/width scale by s, I by s^4,
    // point force by s^2 and distributed load intensity by s. For either
    // load family, normalized deflection and bending stress remain invariant;
    // moments scale as s^3 and reactions as s^2.
    for scale in [0.5_f64, 0.75, 1.25, 2.0, 4.0] {
        for case_index in 0..load_cases.len() {
            let baseline_case = &base["load_cases"][case_index];
            let reference = evaluate_reference(&base, baseline_case)
                .expect("baseline simple-span case is valid");
            let mut changed = base.clone();
            changed["inputs"]["span_m"] = serde_json::json!(span * scale);
            changed["inputs"]["section_width_m"] = serde_json::json!(width * scale);
            changed["inputs"]["section_depth_m"] = serde_json::json!(depth * scale);
            changed["inputs"]["second_moment_area_m4"] =
                serde_json::json!(inertia * scale.powi(4));
            changed["inputs"]["extreme_fiber_distance_m"] =
                serde_json::json!(fiber_distance * scale);

            if case_index == 0 {
                let point_load = baseline_case["load_N"].as_f64().unwrap();
                changed["load_cases"][case_index]["load_N"] =
                    serde_json::json!(point_load * scale.powi(2));
            } else {
                let uniform_load = baseline_case["load_N_per_m"].as_f64().unwrap();
                changed["load_cases"][case_index]["load_N_per_m"] =
                    serde_json::json!(uniform_load * scale);
            }

            let result = evaluate_reference(&changed, &changed["load_cases"][case_index])
                .expect("similarity-scaled simple-span case is valid");
            assert!(close_relative(
                result.deflection_m / (span * scale),
                reference.deflection_m / span,
                tolerance
            ), "normalized deflection must be scale invariant for case={case_index}, s={scale}");
            assert!(close_relative(
                result.maximum_moment_nm,
                reference.maximum_moment_nm * scale.powi(3),
                tolerance
            ));
            assert!(close_relative(result.maximum_stress_pa, reference.maximum_stress_pa, tolerance));
            assert!(close_relative(
                result.reaction_magnitude_n,
                reference.reaction_magnitude_n * scale.powi(2),
                tolerance
            ));
        }
    }
}

#[test]
fn simply_supported_similarity_control_detects_missing_span_power_mutants() {
    let base: Value = serde_json::from_str(CASE).expect("reference fixture must be valid JSON");
    let tolerance = base["acceptance"]["relative_tolerance"]
        .as_f64()
        .expect("relative_tolerance");
    let inputs = &base["inputs"];
    let span = inputs["span_m"].as_f64().unwrap();
    let modulus = inputs["elastic_modulus_Pa"].as_f64().unwrap();
    let inertia = inputs["second_moment_area_m4"].as_f64().unwrap();
    let scale = 2.0_f64;
    let cases = base["load_cases"].as_array().expect("load_cases array");
    assert_eq!(cases.len(), 2, "both simple-span load families are required");

    for (case_index, load_case) in cases.iter().enumerate() {
        let (mutant_base, mutant_scaled) = if case_index == 0 {
            let load = load_case["load_N"].as_f64().unwrap();
            // Seeded point-load mutant: delta = F * L^2 / (48 E I).
            (
                load * span.powi(2) / (48.0 * modulus * inertia),
                (load * scale.powi(2) * (span * scale).powi(2))
                    / (48.0 * modulus * (inertia * scale.powi(4))),
            )
        } else {
            let load_per_m = load_case["load_N_per_m"].as_f64().unwrap();
            // Seeded uniform-load mutant: delta = 5 q * L^3 / (384 E I).
            (
                5.0 * load_per_m * span.powi(3) / (384.0 * modulus * inertia),
                5.0 * (load_per_m * scale) * (span * scale).powi(3)
                    / (384.0 * modulus * (inertia * scale.powi(4))),
            )
        };
        assert!(
            !close_relative(
                mutant_scaled / (span * scale),
                mutant_base / span,
                tolerance
            ),
            "missing-span-power mutant must be detected for load case {case_index}"
        );
    }
}
