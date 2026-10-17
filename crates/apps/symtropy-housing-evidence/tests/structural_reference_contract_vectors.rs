// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Cross-checks the shared structural-reference API against stable contract
//! vectors also intended to anchor the candidate Lean mathematical contracts.
//! This is a finite conformance corpus, not a Rust/Lean refinement proof.

use jsonschema::draft202012;
use serde_json::Value;
use std::collections::BTreeSet;

use symtropy_housing_evidence::structural_reference::{
    axial_bar, cantilever_tip_load, rectangular_section_properties, simply_supported_beam,
    FourthMomentOfArea, Meters, NewtonMeters, Newtons, NewtonsPerMeter, Pascals,
    SimplySupportedLoadCase, SquareMeters,
};

const CORPUS: &str = include_str!(
    "../../../../docs/built-environment/benchmarks/structural-reference-contract-vectors-v1.json"
);
const SCHEMA: &str = include_str!(
    "../../../../docs/built-environment/benchmarks/structural-reference-contract-vectors.schema.json"
);

fn number(object: &Value, name: &str) -> f64 {
    object
        .get(name)
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("expected finite numeric field {name}"))
}

fn meters(value: f64) -> Meters {
    Meters::try_from_si(value).expect("positive finite SI length")
}

fn newtons(value: f64) -> Newtons {
    Newtons::try_from_si(value).expect("positive finite SI force")
}

fn pascals(value: f64) -> Pascals {
    Pascals::try_from_si(value).expect("positive finite SI pressure")
}

fn newtons_per_meter(value: f64) -> NewtonsPerMeter {
    NewtonsPerMeter::try_from_si(value).expect("positive finite SI line load")
}

fn assert_vector_value(vector_id: &str, expected: &Value, key: &str, actual: f64, tolerance: f64) {
    let expected_value = number(expected, key);
    assert!(
        actual.is_finite() && expected_value.is_finite(),
        "{vector_id}: {key} must be finite (actual={actual}, expected={expected_value})"
    );
    assert!(
        (actual - expected_value).abs()
            <= tolerance * expected_value.abs().max(f64::MIN_POSITIVE),
        "{}: {} mismatch (actual={:.17e}, expected={:.17e}, tolerance={:.3e})",
        vector_id,
        key,
        actual,
        expected_value,
        tolerance
    );
}

#[test]
fn shared_contract_vector_corpus_matches_typed_rust_reference_api() {
    let corpus: Value =
        serde_json::from_str(CORPUS).expect("contract vector corpus must be valid JSON");
    let schema: Value =
        serde_json::from_str(SCHEMA).expect("contract vector schema must be valid JSON");
    draft202012::meta::validate(&schema).expect("contract vector schema must be Draft 2020-12");
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .expect("contract vector schema must compile");
    let schema_errors: Vec<String> = validator
        .iter_errors(&corpus)
        .map(|error| error.to_string())
        .collect();
    assert!(
        schema_errors.is_empty(),
        "contract vector corpus violates its schema: {schema_errors:#?}"
    );
    assert_eq!(
        corpus["corpus_id"].as_str(),
        Some("symtropy.structural-reference-contract-vectors.v1")
    );
    assert_eq!(
        corpus["status"].as_str(),
        Some("ANALYTICAL_REFERENCE_CONFORMANCE_ONLY"),
        "the corpus must not imply solver validation or construction approval"
    );
    assert_eq!(number(&corpus["numeric_policy"], "relative_tolerance"), 1.0e-12);

    let vectors = corpus["vectors"]
        .as_array()
        .expect("vectors must be an array");
    assert_eq!(vectors.len(), 5, "all five mathematical contract families must be represented");

    let mut vector_ids = BTreeSet::new();
    let mut contract_ids = BTreeSet::new();

    for vector in vectors {
        let vector_id = vector["vector_id"]
            .as_str()
            .expect("vector_id must be a string");
        assert!(
            vector_ids.insert(vector_id.to_string()),
            "vector_id must be unique: {vector_id}"
        );
        let contract_id = vector["contract_id"]
            .as_str()
            .expect("contract_id must be a string");
        contract_ids.insert(contract_id.to_string());

        let inputs = &vector["inputs"];
        let expected = &vector["expected"];
        let tolerance = number(&corpus["numeric_policy"], "relative_tolerance");

        match contract_id {
            "geometry.rectangular_section.second_moment.v1" => {
                let result = rectangular_section_properties(
                    meters(number(inputs, "width_m")),
                    meters(number(inputs, "depth_m")),
                )
                .expect("positive rectangular-section vector");
                assert_vector_value(
                    vector_id,
                    expected,
                    "area_m2",
                    result.area_m2.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "second_moment_area_m4",
                    result.second_moment_area_m4.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "extreme_fiber_distance_m",
                    result.extreme_fiber_distance_m.get(),
                    tolerance,
                );
                let _: SquareMeters = result.area_m2;
                let _: FourthMomentOfArea = result.second_moment_area_m4;
            }
            "mechanics.axial_bar.extension.v1" => {
                let result = axial_bar(
                    newtons(number(inputs, "load_N")),
                    meters(number(inputs, "length_m")),
                    pascals(number(inputs, "elastic_modulus_Pa")),
                    meters(number(inputs, "width_m")),
                    meters(number(inputs, "depth_m")),
                )
                .expect("positive axial-bar vector");
                assert_vector_value(
                    vector_id,
                    expected,
                    "area_m2",
                    result.section.area_m2.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "axial_stress_Pa",
                    result.axial_stress_pa.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "free_end_extension_m",
                    result.free_end_extension_m.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "support_reaction_magnitude_N",
                    result.support_reaction_magnitude_n.get(),
                    tolerance,
                );
            }
            "mechanics.cantilever.tip_deflection.v1" => {
                let result = cantilever_tip_load(
                    newtons(number(inputs, "load_N")),
                    meters(number(inputs, "span_m")),
                    pascals(number(inputs, "elastic_modulus_Pa")),
                    meters(number(inputs, "width_m")),
                    meters(number(inputs, "depth_m")),
                )
                .expect("positive cantilever vector");
                assert_vector_value(
                    vector_id,
                    expected,
                    "area_m2",
                    result.section.area_m2.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "second_moment_area_m4",
                    result.section.second_moment_area_m4.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "extreme_fiber_distance_m",
                    result.section.extreme_fiber_distance_m.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "maximum_deflection_m",
                    result.maximum_deflection_m.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "maximum_bending_moment_Nm",
                    result.maximum_bending_moment_nm.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "maximum_bending_stress_Pa",
                    result.maximum_bending_stress_pa.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "support_reaction_magnitude_N",
                    result.support_reaction_magnitude_n.get(),
                    tolerance,
                );
            }
            "mechanics.simply_supported.midpoint_point_load.v1"
            | "mechanics.simply_supported.uniform_load.v1" => {
                let load_case = &vector["load_case"];
                let load_kind = load_case["kind"]
                    .as_str()
                    .expect("load_case.kind must be a string");
                let typed_load_case = match (contract_id, load_kind) {
                    (
                        "mechanics.simply_supported.midpoint_point_load.v1",
                        "midpoint_point_load",
                    ) => SimplySupportedLoadCase::MidpointPointLoad {
                        force_n: newtons(number(load_case, "force_N")),
                    },
                    (
                        "mechanics.simply_supported.uniform_load.v1",
                        "uniform_distributed_load",
                    ) => SimplySupportedLoadCase::UniformDistributedLoad {
                        load_n_per_m: newtons_per_meter(number(load_case, "load_N_per_m")),
                    },
                    _ => panic!("{vector_id}: contract_id and load_case.kind disagree"),
                };
                let result = simply_supported_beam(
                    meters(number(inputs, "span_m")),
                    pascals(number(inputs, "elastic_modulus_Pa")),
                    meters(number(inputs, "width_m")),
                    meters(number(inputs, "depth_m")),
                    typed_load_case,
                )
                .expect("positive simply-supported beam vector");
                assert_vector_value(
                    vector_id,
                    expected,
                    "area_m2",
                    result.section.area_m2.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "second_moment_area_m4",
                    result.section.second_moment_area_m4.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "extreme_fiber_distance_m",
                    result.section.extreme_fiber_distance_m.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "maximum_deflection_m",
                    result.maximum_deflection_m.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "maximum_bending_moment_Nm",
                    result.maximum_bending_moment_nm.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "maximum_bending_stress_Pa",
                    result.maximum_bending_stress_pa.get(),
                    tolerance,
                );
                assert_vector_value(
                    vector_id,
                    expected,
                    "support_reaction_magnitude_N",
                    result.support_reaction_magnitude_n.get(),
                    tolerance,
                );
                let _: NewtonMeters = result.maximum_bending_moment_nm;
            }
            unknown => panic!("{vector_id}: unknown contract_id {unknown}"),
        }
    }

    assert_eq!(
        contract_ids.len(),
        5,
        "each unique mathematical contract family must occur in the corpus"
    );
}
