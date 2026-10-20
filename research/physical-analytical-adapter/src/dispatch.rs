// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-first dispatch receipts for physical analytical requests.
//!
//! Requests are matched against the pinned capability manifest before the
//! mechanics adapter runs. This module never launches external processes.

use crate::capabilities::{SelectionStatus, resolve_request};
use crate::evaluate_request_bytes;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub type Check<T> = Result<T, String>;

const REQUEST_SCHEMA_V01: &str = "physical-analytical-request-v0.1";
const RECEIPT_SCHEMA: &str = "physical-solver-dispatch-receipt-v0.1";
const REQUEST_KEYS_V01: &[&str] = &[
    "schema_version",
    "request_id",
    "source_revision",
    "case_id",
    "assumption_profile",
    "requested_physics",
    "units",
    "inputs",
];
const UNIT_KEYS: &[&str] = &[
    "length",
    "force",
    "area",
    "stress",
    "second_moment",
    "distributed_load",
];
const KNOWN_PHYSICS: &[&str] = &[
    "static_linear_elastic",
    "dynamic_response",
    "fracture",
    "rigid_contact",
    "granular_contact",
    "thermal_coupling",
];

#[derive(Clone, Debug)]
struct DispatchRequest {
    schema_version: String,
    request_id: String,
    source_revision: String,
    case_id: String,
    assumption_profile: String,
    requested_physics: Vec<String>,
    units: Map<String, Value>,
}

pub fn dispatch_request_bytes(
    bytes: &[u8],
    dispatcher_revision: &str,
    adapter_revision: &str,
) -> Check<Value> {
    validate_revision(dispatcher_revision, "dispatcher revision")?;
    validate_revision(adapter_revision, "adapter revision")?;
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("dispatch request JSON is invalid: {error}"))?;
    let request = parse_dispatch_request(&value)?;
    let request_digest = sha256_hex(bytes);

    // The resolver is deliberately called before the numerical adapter. If no
    // compatible backend exists, return an abstention receipt with no partial
    // numerical output and without evaluating a weaker fallback model.
    let resolution = resolve_request(
        &request.case_id,
        &request.assumption_profile,
        &request.requested_physics,
        &request.units,
    )?;
    if resolution.status == SelectionStatus::Unresolved {
        return Ok(json!({
            "schema_version": RECEIPT_SCHEMA,
            "request_id": request.request_id,
            "request_schema_version": request.schema_version,
            "request_sha256": request_digest,
            "source_revision": request.source_revision,
            "dispatcher_revision": dispatcher_revision,
            "adapter_revision": adapter_revision,
            "manifest_sha256": resolution.manifest_sha256,
            "case_id": request.case_id,
            "requested_physics": request.requested_physics,
            "physics_omitted": resolution.physics_omitted,
            "selection_status": "UNRESOLVED",
            "evaluation_status": "NOT_EVALUATED",
            "selected_backend": null,
            "model_evaluation_attempted": false,
            "external_solver_invoked": false,
            "outputs": null,
            "adapter_receipt": null,
            "diagnostics": resolution.diagnostics,
            "claim_ceiling": "capability screening only; no model evaluation, external solver execution, measured physical validation, or structural qualification"
        }));
    }

    let adapter_receipt = evaluate_request_bytes(bytes, adapter_revision)?;
    let adapter_status = required_string(&adapter_receipt, "result_status", "adapter receipt")?;
    let output_contract_matches = adapter_status != "PASS"
        || output_keys_match(
            adapter_receipt.get("outputs").unwrap_or(&Value::Null),
            &resolution.outputs,
        );
    let evaluation_status = if adapter_status == "PASS" && output_contract_matches {
        "PASS"
    } else {
        "UNRESOLVED"
    };
    let outputs = if evaluation_status == "PASS" {
        adapter_receipt
            .get("outputs")
            .cloned()
            .unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    let selected_backend = resolution
        .selected_backend
        .as_ref()
        .map(backend_summary)
        .unwrap_or(Value::Null);
    let mut diagnostics = adapter_receipt
        .get("diagnostics")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if !output_contract_matches {
        diagnostics.push(
            "analytical adapter outputs do not match the selected capability manifest output set"
                .to_owned(),
        );
    }

    Ok(json!({
        "schema_version": RECEIPT_SCHEMA,
        "request_id": request.request_id,
        "request_schema_version": request.schema_version,
        "request_sha256": adapter_receipt["request_sha256"],
        "source_revision": request.source_revision,
        "dispatcher_revision": dispatcher_revision,
        "adapter_revision": adapter_receipt["adapter_revision"],
        "manifest_sha256": resolution.manifest_sha256,
        "case_id": request.case_id,
        "requested_physics": request.requested_physics,
        "physics_omitted": [],
        "selection_status": "SELECTED",
        "evaluation_status": evaluation_status,
        "selected_backend": selected_backend,
        "model_evaluation_attempted": true,
        "external_solver_invoked": false,
        "outputs": outputs,
        "adapter_receipt": adapter_receipt,
        "diagnostics": diagnostics,
        "claim_ceiling": "capability dispatch plus the nested named closed-form analytical result only; no external solver execution, measured physical validation, or structural qualification"
    }))
}

fn parse_dispatch_request(value: &Value) -> Check<DispatchRequest> {
    let schema_version = required_string(value, "schema_version", "request")?;
    if schema_version != REQUEST_SCHEMA_V01 {
        return Err(format!("request.schema_version is unsupported: {schema_version}"));
    }
    exact_keys_with_optional(value, REQUEST_KEYS_V01, &["input_bounds"], "request")?;
    let request_id = required_string(value, "request_id", "request")?;
    validate_token(&request_id, "request ID")?;
    let source_revision = required_string(value, "source_revision", "request")?;
    validate_revision(&source_revision, "source revision")?;
    let case_id = required_string(value, "case_id", "request")?;
    validate_token(&case_id, "case ID")?;
    let assumption_profile = required_string(value, "assumption_profile", "request")?;
    validate_token(&assumption_profile, "assumption profile")?;
    let requested_physics = required_string_array(value, "requested_physics", "request")?;
    if requested_physics.is_empty() {
        return Err("request.requested_physics must not be empty".to_owned());
    }
    let mut seen = BTreeSet::new();
    for physics in &requested_physics {
        if !KNOWN_PHYSICS.contains(&physics.as_str()) {
            return Err(format!(
                "request.requested_physics has unsupported value '{physics}'"
            ));
        }
        if !seen.insert(physics.as_str()) {
            return Err(format!(
                "request.requested_physics contains duplicate value '{physics}'"
            ));
        }
    }
    let inputs = value
        .get("inputs")
        .and_then(Value::as_object)
        .ok_or_else(|| "request.inputs must be an object".to_owned())?
        .clone();
    validate_case_inputs(&case_id, &inputs)?;
    if let Some(input_bounds) = value.get("input_bounds") {
        validate_input_bounds(&case_id, &inputs, input_bounds)?;
    }
    let units = value
        .get("units")
        .and_then(Value::as_object)
        .ok_or_else(|| "request.units must be an object".to_owned())?
        .clone();
    exact_map_keys(&units, UNIT_KEYS, "request.units")?;
    for key in UNIT_KEYS {
        let actual = units
            .get(*key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("request.units.{key} must be a string"))?;
        if actual.trim().is_empty() {
            return Err(format!("request.units.{key} must not be empty"));
        }
    }
    Ok(DispatchRequest {
        schema_version,
        request_id,
        source_revision,
        case_id,
        assumption_profile,
        requested_physics,
        units,
    })
}

fn validate_input_bounds(case_id: &str, inputs: &Map<String, Value>, contract: &Value) -> Check<()> {
    exact_keys(contract, &["schema_version", "inputs"], "request.input_bounds")?;
    let schema_version = required_string(contract, "schema_version", "request.input_bounds")?;
    if schema_version != "physical-input-bounds-v0.1" {
        return Err("request.input_bounds.schema_version must be 'physical-input-bounds-v0.1'".to_owned());
    }
    let bound_map = contract
        .get("inputs")
        .and_then(Value::as_object)
        .ok_or_else(|| "request.input_bounds.inputs must be an object".to_owned())?;
    let expected: &[&str] = match case_id {
        "axial_bar" => &["force_n", "length_m", "area_m2", "youngs_modulus_pa"],
        "cantilever_tip_load" => &[
            "force_n", "length_m", "youngs_modulus_pa", "second_moment_area_m4",
            "outer_fiber_distance_m",
        ],
        "simply_supported_uniform_load" => &[
            "load_n_per_m", "length_m", "youngs_modulus_pa", "second_moment_area_m4",
            "outer_fiber_distance_m",
        ],
        _ => return Err(format!("cannot validate bounds for unsupported case '{case_id}'")),
    };
    exact_map_keys(bound_map, expected, "request.input_bounds.inputs")?;
    const BOUND_KEYS: &[&str] = &["lower", "upper", "unit", "basis", "source_ref"];
    for key in expected {
        let at = format!("request.input_bounds.inputs.{key}");
        let bound = bound_map
            .get(*key)
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{at} must be an object"))?;
        exact_map_keys(bound, BOUND_KEYS, &at)?;
        let lower = finite_number(bound, "lower", &at)?;
        let upper = finite_number(bound, "upper", &at)?;
        if lower > upper {
            return Err(format!("{at}.lower must not exceed upper"));
        }
        let nominal = inputs
            .get(*key)
            .and_then(Value::as_f64)
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("request.inputs.{key} must be finite"))?;
        if nominal < lower || nominal > upper {
            return Err(format!("{at} must contain its nominal input"));
        }
        let (expected_unit, require_positive) = match *key {
            "force_n" => ("N", false),
            "length_m" => ("m", true),
            "area_m2" => ("m^2", true),
            "youngs_modulus_pa" => ("Pa", true),
            "second_moment_area_m4" => ("m^4", true),
            "outer_fiber_distance_m" => ("m", true),
            "load_n_per_m" => ("N/m", false),
            _ => return Err(format!("{at} names an unrecognized input")),
        };
        let unit = required_string_map(bound, "unit", &at)?;
        if unit != expected_unit {
            return Err(format!("{at}.unit must be '{expected_unit}'"));
        }
        if require_positive && lower <= 0.0 {
            return Err(format!("{at}.lower must be strictly positive"));
        }
        let basis = required_string_map(bound, "basis", &at)?;
        if ![
            "instrumented_measurement",
            "datasheet_tolerance",
            "engineering_estimate",
            "scenario_bound",
        ].contains(&basis.as_str())
        {
            return Err(format!("{at}.basis is unsupported"));
        }
        let source_ref = required_string_map(bound, "source_ref", &at)?;
        if source_ref.trim().is_empty()
            || source_ref.len() > 2048
            || source_ref.chars().any(char::is_control)
        {
            return Err(format!("{at}.source_ref must be a valid non-empty locator"));
        }
    }
    Ok(())
}

fn finite_number(value: &Map<String, Value>, key: &str, at: &str) -> Check<f64> {
    value.get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{at}.{key} must be finite numeric"))
}

fn required_string_map(value: &Map<String, Value>, key: &str, at: &str) -> Check<String> {
    value.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn validate_case_inputs(case_id: &str, inputs: &Map<String, Value>) -> Check<()> {
    let expected: &[&str] = match case_id {
        "axial_bar" => &["force_n", "length_m", "area_m2", "youngs_modulus_pa"],
        "cantilever_tip_load" => &[
            "force_n",
            "length_m",
            "youngs_modulus_pa",
            "second_moment_area_m4",
            "outer_fiber_distance_m",
        ],
        "simply_supported_uniform_load" => &[
            "load_n_per_m",
            "length_m",
            "youngs_modulus_pa",
            "second_moment_area_m4",
            "outer_fiber_distance_m",
        ],
        // Unknown cases are not evaluated, but their request still receives a
        // structured unresolved dispatch receipt.
        _ => return Ok(()),
    };
    exact_map_keys(inputs, expected, "request.inputs")?;
    for key in expected {
        let number = inputs
            .get(*key)
            .and_then(Value::as_f64)
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("request.inputs.{key} must be finite"))?;
        if matches!(
            *key,
            "length_m"
                | "area_m2"
                | "youngs_modulus_pa"
                | "second_moment_area_m4"
                | "outer_fiber_distance_m"
        ) && number <= 0.0
        {
            return Err(format!("request.inputs.{key} must be positive"));
        }
    }
    Ok(())
}

fn output_keys_match(outputs: &Value, expected: &[String]) -> bool {
    let Some(actual) = outputs.as_object() else {
        return false;
    };
    let actual: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().map(String::as_str).collect();
    actual == expected
}

fn backend_summary(backend: &Value) -> Value {
    json!({
        "backend_id": backend["backend_id"],
        "backend_version": backend["backend_version"],
        "implementation_kind": backend["implementation_kind"],
        "qualification_tier": backend["qualification_tier"],
        "real_world_validation_status": backend["real_world_validation_status"],
        "external_solver_invoked": backend["external_solver_invoked"],
        "claim_ceiling": backend["claim_ceiling"]
    })
}

fn exact_keys_with_optional(
    value: &Value,
    required: &[&str],
    optional: &[&str],
    at: &str,
) -> Check<()> {
    let actual = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    let required_set: BTreeSet<&str> = required.iter().copied().collect();
    let mut allowed: BTreeSet<&str> = required_set.clone();
    allowed.extend(optional.iter().copied());
    let actual_set: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    let missing: Vec<_> = required_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&allowed).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
    }
    Ok(())
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    exact_map_keys(object, expected, at)
}

fn exact_map_keys(actual: &Map<String, Value>, expected: &[&str], at: &str) -> Check<()> {
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{at}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}

fn required_string(value: &Value, key: &str, at: &str) -> Check<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn required_string_array(value: &Value, key: &str, at: &str) -> Check<Vec<String>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))?
        .iter()
        .enumerate()
        .map(|(index, element)| {
            element
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{at}.{key}[{index}] must be a string"))
        })
        .collect()
}

fn validate_token(value: &str, label: &str) -> Check<()> {
    if value.is_empty()
        || value.len() > 128
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
        })
    {
        return Err(format!("invalid {label}"));
    }
    Ok(())
}

fn validate_revision(revision: &str, label: &str) -> Check<()> {
    if revision.len() != 40
        || !revision
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(format!(
            "{label} must be a 40-character lowercase hexadecimal Git SHA-1"
        ));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{fs, path::PathBuf};

    const REV: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn fixture(name: &str) -> Vec<u8> {
        fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/physical-transformation/analytical-fixtures")
                .join(name),
        )
        .expect("analytical fixture exists")
    }

    #[test]
    fn all_supported_case_families_resolve_and_preserve_formula_outputs() {
        for (name, output_key) in [
            ("analytical-bar-v0.1.json", "axial_stress_pa"),
            ("cantilever-tip-load-v0.1.json", "tip_deflection_m"),
            (
                "simply-supported-uniform-load-v0.1.json",
                "midspan_deflection_m",
            ),
        ] {
            let receipt =
                dispatch_request_bytes(&fixture(name), REV, REV).expect("supported case dispatch");
            assert_eq!(receipt["selection_status"], "SELECTED");
            assert_eq!(receipt["evaluation_status"], "PASS");
            assert_eq!(
                receipt["selected_backend"]["backend_id"],
                "symtropy-analytical-reference"
            );
            assert!(receipt["outputs"][output_key].is_number());
            assert_eq!(receipt["external_solver_invoked"], false);
            assert_eq!(receipt["model_evaluation_attempted"], true);
            assert_eq!(receipt["manifest_sha256"].as_str().unwrap().len(), 64);
            assert!(receipt["adapter_receipt"].is_object());
        }
    }

    #[test]
    fn unsupported_physics_yields_no_backend_and_no_formula_evaluation() {
        let receipt = dispatch_request_bytes(&fixture("unsupported-fracture-v0.1.json"), REV, REV)
            .expect("unsupported physics is a dispatch abstention");
        assert_eq!(receipt["selection_status"], "UNRESOLVED");
        assert_eq!(receipt["evaluation_status"], "NOT_EVALUATED");
        assert_eq!(receipt["selected_backend"], Value::Null);
        assert_eq!(receipt["outputs"], Value::Null);
        assert_eq!(receipt["adapter_receipt"], Value::Null);
        assert_eq!(receipt["model_evaluation_attempted"], false);
        assert_eq!(receipt["external_solver_invoked"], false);
        assert!(
            receipt["physics_omitted"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| value == "fracture")
        );
    }

    #[test]
    fn unknown_case_id_returns_structured_unresolved_receipt() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["case_id"] = json!("dynamic_collapse");
        request["assumption_profile"] = json!("dynamic_collapse");
        request["inputs"] = json!({});
        let bytes = serde_json::to_vec(&request).expect("serialize unknown case request");
        let receipt = dispatch_request_bytes(&bytes, REV, REV)
            .expect("unknown validly-shaped case becomes a dispatch abstention");
        assert_eq!(receipt["selection_status"], "UNRESOLVED");
        assert_eq!(receipt["evaluation_status"], "NOT_EVALUATED");
        assert_eq!(receipt["selected_backend"], Value::Null);
        assert_eq!(receipt["outputs"], Value::Null);
        assert_eq!(receipt["adapter_receipt"], Value::Null);
        assert_eq!(receipt["model_evaluation_attempted"], false);
        assert!(
            receipt["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| { value.as_str().unwrap().contains("no registered backend") })
        );
    }

    #[test]
    fn selected_backend_with_domain_failure_does_not_become_pass() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["inputs"]["force_n"] = json!(1.0e308);
        request["inputs"]["length_m"] = json!(1.0e308);
        let bytes = serde_json::to_vec(&request).expect("serialize finite request");
        let receipt = dispatch_request_bytes(&bytes, REV, REV)
            .expect("domain failure is represented in the nested receipt");
        assert_eq!(receipt["selection_status"], "SELECTED");
        assert_eq!(receipt["evaluation_status"], "UNRESOLVED");
        assert_eq!(receipt["outputs"], Value::Null);
        assert_eq!(receipt["adapter_receipt"]["result_status"], "UNRESOLVED");
        assert_eq!(receipt["model_evaluation_attempted"], true);
        assert!(
            receipt["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|value| { value.as_str().unwrap().contains("could not be completed") })
        );
    }

    #[test]
    fn output_contract_mismatch_cannot_be_reported_as_pass() {
        let expected = vec!["axial_stress_pa".to_owned(), "extension_m".to_owned()];
        let outputs = json!({
            "axial_stress_pa": 100_000.0,
            "extension_m": 1.0e-6,
            "unexpected_output": 7.0
        });
        assert!(!output_keys_match(&outputs, &expected));
        assert!(!output_keys_match(&Value::Null, &expected));
    }

    #[test]
    fn bounded_v01_request_dispatches_with_current_provenance_enclosures() {
        let receipt = dispatch_request_bytes(
            &fixture("analytical-bar-bounds-v0.1.json"),
            REV,
            REV,
        ).expect("bounded v0.1 request should dispatch");
        assert_eq!(receipt["request_schema_version"], REQUEST_SCHEMA_V01);
        assert_eq!(receipt["selection_status"], "SELECTED");
        assert_eq!(receipt["evaluation_status"], "PASS");
        assert_eq!(
            receipt["adapter_receipt"]["schema_version"],
            "physical-analytical-receipt-v0.1"
        );
        assert_eq!(
            receipt["outputs"]["axial_stress_pa"],
            receipt["adapter_receipt"]["output_enclosures"]["axial_stress_pa"]["nominal"]
        );
        assert_eq!(
            receipt["adapter_receipt"]["output_enclosures"]["axial_stress_pa"]["unit"],
            "Pa"
        );
    }

    #[test]
    fn invalid_v01_bounds_fail_before_dispatch() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-bounds-v0.1.json")).unwrap();
        request["input_bounds"]["inputs"]["force_n"]["lower"] = json!(1_001.0);
        let error = dispatch_request_bytes(&request.to_string().into_bytes(), REV, REV)
            .unwrap_err();
        assert!(error.contains("must contain its nominal input"));
    }

    #[test]
    fn unsupported_physics_with_bounds_abstains_before_formula_evaluation() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-bounds-v0.1.json")).unwrap();
        request["requested_physics"] = json!(["static_linear_elastic", "fracture"]);
        let receipt = dispatch_request_bytes(&request.to_string().into_bytes(), REV, REV)
            .expect("unsupported physics is a structured abstention");
        assert_eq!(receipt["selection_status"], "UNRESOLVED");
        assert_eq!(receipt["evaluation_status"], "NOT_EVALUATED");
        assert_eq!(receipt["adapter_receipt"], Value::Null);
        assert_eq!(receipt["outputs"], Value::Null);
        assert_eq!(receipt["model_evaluation_attempted"], false);
    }

    #[test]
    fn dispatch_receipt_is_deterministic_for_fixed_inputs_and_revisions() {
        let bytes = fixture("analytical-bar-v0.1.json");
        let first = dispatch_request_bytes(&bytes, REV, REV).expect("first dispatch");
        let second = dispatch_request_bytes(&bytes, REV, REV).expect("repeat dispatch");
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first"),
            serde_json::to_vec(&second).expect("serialize second")
        );
        assert_eq!(first["request_sha256"], second["request_sha256"]);
        assert_eq!(first["manifest_sha256"], second["manifest_sha256"]);
    }

    #[test]
    fn invalid_dispatcher_revision_fails_closed() {
        assert!(
            dispatch_request_bytes(&fixture("analytical-bar-v0.1.json"), "not-a-sha", REV)
                .unwrap_err()
                .contains("dispatcher revision")
        );
    }

    #[test]
    fn incompatible_units_return_abstention_and_malformed_contracts_are_rejected() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["units"]["length"] = json!("ft");
        let receipt = dispatch_request_bytes(request.to_string().as_bytes(), REV, REV)
            .expect("well-formed but incompatible units yield an abstention receipt");
        assert_eq!(receipt["selection_status"], "UNRESOLVED");
        assert_eq!(receipt["evaluation_status"], "NOT_EVALUATED");
        assert_eq!(receipt["selected_backend"], Value::Null);
        assert_eq!(receipt["outputs"], Value::Null);
        assert_eq!(receipt["model_evaluation_attempted"], false);

        let mut malformed_units: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        malformed_units["units"]["length"] = json!(17);
        assert!(dispatch_request_bytes(malformed_units.to_string().as_bytes(), REV, REV).is_err());

        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["unreviewed_override"] = json!(true);
        assert!(dispatch_request_bytes(request.to_string().as_bytes(), REV, REV).is_err());
    }
}
