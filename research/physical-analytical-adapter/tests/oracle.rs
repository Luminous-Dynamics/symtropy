// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent known-answer checks for the analytical adapter's public receipt.

use physical_analytical_adapter::evaluate_request_bytes;
use serde_json::Value;
use std::{fs, path::PathBuf};

const ADAPTER_REVISION: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../schemas/physical-transformation/analytical-fixtures")
            .join(name),
    )
    .expect("fixture file exists")
}

fn output_number(receipt: &Value, name: &str) -> f64 {
    receipt["outputs"][name]
        .as_f64()
        .unwrap_or_else(|| panic!("receipt output {name} should be numeric"))
}

#[test]
fn axial_bar_receipt_matches_independent_closed_form_values() {
    let request = fixture("analytical-bar-v0.1.json");
    let receipt = evaluate_request_bytes(&request, ADAPTER_REVISION).expect("valid analytical case");
    assert_eq!(receipt["result_status"], "PASS");
    assert_eq!(receipt["case_id"], "axial_bar");
    assert!((output_number(&receipt, "axial_stress_pa") - 100_000.0).abs() <= 1.0e-8);
    assert!((output_number(&receipt, "axial_strain") - 5.0e-7).abs() <= 1.0e-18);
    assert!((output_number(&receipt, "extension_m") - 1.0e-6).abs() <= 1.0e-15);
}

#[test]
fn cantilever_receipt_matches_independent_closed_form_values() {
    let receipt = evaluate_request_bytes(
        &fixture("cantilever-tip-load-v0.1.json"),
        ADAPTER_REVISION,
    )
    .expect("valid cantilever case");
    assert_eq!(receipt["result_status"], "PASS");
    assert_eq!(receipt["case_id"], "cantilever_tip_load");
    assert!(
        (output_number(&receipt, "tip_deflection_m") - (1.0 / 6_000.0)).abs()
            <= 1.0e-15
    );
    assert!(
        (output_number(&receipt, "maximum_bending_stress_pa") - 2_500_000.0).abs()
            <= 1.0e-6
    );
}

#[test]
fn simply_supported_beam_receipt_matches_independent_closed_form_values() {
    let receipt = evaluate_request_bytes(
        &fixture("simply-supported-uniform-load-v0.1.json"),
        ADAPTER_REVISION,
    )
    .expect("valid simply-supported beam case");
    assert_eq!(receipt["result_status"], "PASS");
    assert_eq!(receipt["case_id"], "simply_supported_uniform_load");
    assert!(
        (output_number(&receipt, "midspan_deflection_m") - (1.0 / 480.0)).abs()
            <= 1.0e-15
    );
    assert!(
        (output_number(&receipt, "maximum_bending_stress_pa") - 25_000_000.0).abs()
            <= 1.0e-6
    );
}

#[test]
fn unsupported_fracture_receipt_is_unresolved_and_does_not_fallback() {
    let request = fixture("unsupported-fracture-v0.1.json");
    let receipt = evaluate_request_bytes(&request, ADAPTER_REVISION)
        .expect("well-formed unsupported case produces a receipt");
    assert_eq!(receipt["result_status"], "UNRESOLVED");
    assert_eq!(receipt["outputs"], Value::Null);
    assert_eq!(receipt["physics_omitted"][0], "fracture");
    assert_eq!(receipt["request_sha256"].as_str().unwrap().len(), 64);
}

#[test]
fn identical_request_and_revision_produce_identical_receipt_bytes() {
    let request = fixture("analytical-bar-v0.1.json");
    let first = evaluate_request_bytes(&request, ADAPTER_REVISION).expect("first receipt");
    let second = evaluate_request_bytes(&request, ADAPTER_REVISION).expect("second receipt");
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
}
