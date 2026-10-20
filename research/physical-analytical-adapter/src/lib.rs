// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executes only three qualified closed-form reference cases.
//!
//! PASS means that the named formula evaluated under its declared assumptions.
//! It is not a finding about the safety of a real structure.

use physical_transformation_reference::mechanics::{
    axial_bar, cantilever_tip_load, simply_supported_uniform_load,
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub type Check<T> = Result<T, String>;

const REQUEST_SCHEMA: &str = "physical-analytical-request-v0.1";
const RECEIPT_SCHEMA: &str = "physical-analytical-receipt-v0.1";
const KNOWN_PHYSICS: &[&str] = &[
    "static_linear_elastic",
    "dynamic_response",
    "fracture",
    "rigid_contact",
    "granular_contact",
    "thermal_coupling",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaseKind {
    AxialBar,
    CantileverTipLoad,
    SimplySupportedUniformLoad,
}

impl CaseKind {
    fn parse(id: &str) -> Check<Self> {
        match id {
            "axial_bar" => Ok(Self::AxialBar),
            "cantilever_tip_load" => Ok(Self::CantileverTipLoad),
            "simply_supported_uniform_load" => Ok(Self::SimplySupportedUniformLoad),
            _ => Err(format!("unsupported analytical case '{id}'")),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::AxialBar => "axial_bar",
            Self::CantileverTipLoad => "cantilever_tip_load",
            Self::SimplySupportedUniformLoad => "simply_supported_uniform_load",
        }
    }

    fn profile(self) -> &'static str {
        match self {
            Self::AxialBar => "uniform_linear_elastic_axial_bar",
            Self::CantileverTipLoad => "small_deflection_euler_bernoulli_cantilever_tip_load",
            Self::SimplySupportedUniformLoad => {
                "small_deflection_euler_bernoulli_simply_supported_uniform_load"
            }
        }
    }

    fn input_keys(self) -> &'static [&'static str] {
        match self {
            Self::AxialBar => &["force_n", "length_m", "area_m2", "youngs_modulus_pa"],
            Self::CantileverTipLoad => &[
                "force_n",
                "length_m",
                "youngs_modulus_pa",
                "second_moment_area_m4",
                "outer_fiber_distance_m",
            ],
            Self::SimplySupportedUniformLoad => &[
                "load_n_per_m",
                "length_m",
                "youngs_modulus_pa",
                "second_moment_area_m4",
                "outer_fiber_distance_m",
            ],
        }
    }

    fn units(self) -> Value {
        match self {
            Self::AxialBar => json!({
                "axial_stress_pa": "Pa",
                "axial_strain": "dimensionless",
                "extension_m": "m"
            }),
            Self::CantileverTipLoad => json!({
                "tip_deflection_m": "m",
                "maximum_bending_stress_pa": "Pa"
            }),
            Self::SimplySupportedUniformLoad => json!({
                "midspan_deflection_m": "m",
                "maximum_bending_stress_pa": "Pa"
            }),
        }
    }

    fn assumptions(self) -> &'static str {
        match self {
            Self::AxialBar => "Uniform axial bar, constant cross-sectional area, static linear elasticity; tension is positive and compression negative; small strain.",
            Self::CantileverTipLoad => "Static, small-deflection, linear-elastic Euler-Bernoulli cantilever with constant section and a point load at the free tip. Force sign determines deflection direction; maximum bending stress is reported as a non-negative magnitude.",
            Self::SimplySupportedUniformLoad => "Static, small-deflection, linear-elastic Euler-Bernoulli simply supported prismatic beam with constant section and uniform load across the full span. Load sign determines deflection direction; maximum bending stress is reported as a non-negative magnitude.",
        }
    }
}

#[derive(Clone, Debug)]
struct ParsedRequest {
    request_id: String,
    source_revision: String,
    case: CaseKind,
    requested_physics: Vec<String>,
    physics_omitted: Vec<String>,
    inputs: Map<String, Value>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut result, "{byte:02x}");
    }
    result
}

/// Evaluate the request or return a typed UNRESOLVED receipt for unsupported
/// physics or numerical-domain failures. Malformed request contracts are errors.
pub fn evaluate_request_bytes(bytes: &[u8], adapter_revision: &str) -> Check<Value> {
    validate_hex(adapter_revision, 40, "adapter revision")?;
    let value: Value =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid request JSON: {error}"))?;
    let request = parse_request(&value)?;

    let input_digest = sha256_hex(bytes);
    if !request.physics_omitted.is_empty() {
        let diagnostics = request
            .physics_omitted
            .iter()
            .map(|physics| {
                format!(
                    "requested physics '{physics}' is outside the analytical adapter capability"
                )
            })
            .collect();
        return Ok(receipt(
            &request,
            adapter_revision,
            &input_digest,
            "UNRESOLVED",
            Value::Null,
            diagnostics,
        ));
    }

    match evaluate_case(request.case, &request.inputs) {
        Ok(outputs) => Ok(receipt(
            &request,
            adapter_revision,
            &input_digest,
            "PASS",
            outputs,
            Vec::new(),
        )),
        Err(error) => Ok(receipt(
            &request,
            adapter_revision,
            &input_digest,
            "UNRESOLVED",
            Value::Null,
            vec![format!("analytical evaluation could not be completed: {error}")],
        )),
    }
}

fn receipt(
    request: &ParsedRequest,
    adapter_revision: &str,
    request_digest: &str,
    status: &str,
    outputs: Value,
    diagnostics: Vec<String>,
) -> Value {
    json!({
        "schema_version": RECEIPT_SCHEMA,
        "request_id": request.request_id,
        "request_sha256": request_digest,
        "source_revision": request.source_revision,
        "adapter_revision": adapter_revision,
        "case_id": request.case.id(),
        "evidence_class": "analytical_reference",
        "result_status": status,
        "computation_method": "closed_form_analytical_reference",
        "requested_physics": request.requested_physics,
        "physics_omitted": request.physics_omitted,
        "assumption_profile": request.case.profile(),
        "assumptions": request.case.assumptions(),
        "outputs": outputs,
        "output_units": request.case.units(),
        "reference_oracle_tolerances": {
            "relative_tolerance": 1.0e-12,
            "absolute_tolerance": 1.0e-15,
            "scope": "applied to frozen CI known-answer fixtures; not a comparison against real specimen data"
        },
        "diagnostics": diagnostics,
        "claim_ceiling": "named closed-form analytical case only; not physical validation, structural certification, or real collapse prediction"
    })
}

fn evaluate_case(case: CaseKind, inputs: &Map<String, Value>) -> Check<Value> {
    let number = |key: &str| -> Check<f64> {
        inputs
            .get(key)
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("inputs.{key} must be finite"))
    };
    match case {
        CaseKind::AxialBar => {
            let result = axial_bar(
                number("force_n")?,
                number("length_m")?,
                number("area_m2")?,
                number("youngs_modulus_pa")?,
            )?;
            Ok(json!({
                "axial_stress_pa": result.axial_stress_pa,
                "axial_strain": result.axial_strain,
                "extension_m": result.extension_m
            }))
        }
        CaseKind::CantileverTipLoad => {
            let result = cantilever_tip_load(
                number("force_n")?,
                number("length_m")?,
                number("youngs_modulus_pa")?,
                number("second_moment_area_m4")?,
                number("outer_fiber_distance_m")?,
            )?;
            Ok(json!({
                "tip_deflection_m": result.tip_deflection_m,
                "maximum_bending_stress_pa": result.maximum_bending_stress_pa
            }))
        }
        CaseKind::SimplySupportedUniformLoad => {
            let result = simply_supported_uniform_load(
                number("load_n_per_m")?,
                number("length_m")?,
                number("youngs_modulus_pa")?,
                number("second_moment_area_m4")?,
                number("outer_fiber_distance_m")?,
            )?;
            Ok(json!({
                "midspan_deflection_m": result.midspan_deflection_m,
                "maximum_bending_stress_pa": result.maximum_bending_stress_pa
            }))
        }
    }
}

fn parse_request(value: &Value) -> Check<ParsedRequest> {
    exact_keys(
        value,
        &[
            "schema_version",
            "request_id",
            "source_revision",
            "case_id",
            "assumption_profile",
            "requested_physics",
            "units",
            "inputs",
        ],
        "request",
    )?;
    expect_string(value, "schema_version", REQUEST_SCHEMA, "request")?;
    let request_id = string(value, "request_id", "request")?;
    token(&request_id, "request ID")?;
    let source_revision = string(value, "source_revision", "request")?;
    validate_hex(&source_revision, 40, "source revision")?;

    let case_id = string(value, "case_id", "request")?;
    let case = CaseKind::parse(&case_id)?;
    expect_string(value, "assumption_profile", case.profile(), "request")?;

    let requested_physics = strings(value, "requested_physics", "request")?;
    if requested_physics.is_empty() {
        return Err("request.requested_physics must not be empty".to_owned());
    }
    let known: BTreeSet<&str> = KNOWN_PHYSICS.iter().copied().collect();
    let mut seen = BTreeSet::new();
    for item in &requested_physics {
        if !known.contains(item.as_str()) {
            return Err(format!("unknown requested physics '{item}'"));
        }
        if !seen.insert(item.as_str()) {
            return Err(format!("duplicate requested physics '{item}'"));
        }
    }
    let physics_omitted = requested_physics
        .iter()
        .filter(|physics| physics.as_str() != "static_linear_elastic")
        .cloned()
        .collect();

    let units = object(value, "units", "request")?;
    exact_keys_map(
        units,
        &[
            "length",
            "force",
            "area",
            "stress",
            "second_moment",
            "distributed_load",
        ],
        "request.units",
    )?;
    for (key, expected) in [
        ("length", "m"),
        ("force", "N"),
        ("area", "m^2"),
        ("stress", "Pa"),
        ("second_moment", "m^4"),
        ("distributed_load", "N/m"),
    ] {
        if map_string(units, key, "request.units")? != expected {
            return Err(format!("request.units.{key} must be '{expected}'"));
        }
    }

    let inputs = object(value, "inputs", "request")?;
    exact_keys_map(inputs, case.input_keys(), "request.inputs")?;
    for key in case.input_keys() {
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

    Ok(ParsedRequest {
        request_id,
        source_revision,
        case,
        requested_physics,
        physics_omitted,
        inputs: inputs.to_owned(),
    })
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    exact_keys_map(object, expected, at)
}

fn exact_keys_map(
    actual: &Map<String, Value>,
    expected: &[&str],
    at: &str,
) -> Check<()> {
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
    }
    Ok(())
}

fn object<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Map<String, Value>> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{at}.{key} must be an object"))
}

fn string(value: &Value, key: &str, at: &str) -> Check<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn map_string(value: &Map<String, Value>, key: &str, at: &str) -> Check<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn strings(value: &Value, key: &str, at: &str) -> Check<Vec<String>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))?
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{at}.{key}[{index}] must be a string"))
        })
        .collect()
}

fn expect_string(value: &Value, key: &str, expected: &str, at: &str) -> Check<()> {
    let actual = string(value, key, at)?;
    if actual != expected {
        return Err(format!("{at}.{key} must be '{expected}', got '{actual}'"));
    }
    Ok(())
}

fn token(value: &str, label: &str) -> Check<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
    {
        return Err(format!("invalid {label}"));
    }
    Ok(())
}

fn validate_hex(value: &str, length: usize, label: &str) -> Check<()> {
    if value.len() != length
        || !value
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(format!("{label} must be {length} lowercase hexadecimal characters"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    const REV: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn fixture(name: &str) -> Vec<u8> {
        fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/physical-transformation/analytical-fixtures")
                .join(name),
        )
        .expect("analytical request fixture exists")
    }

    #[test]
    fn sha256_matches_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn all_three_known_answer_cases_evaluate_with_units() {
        for name in [
            "analytical-bar-v0.1.json",
            "cantilever-tip-load-v0.1.json",
            "simply-supported-uniform-load-v0.1.json",
        ] {
            let bytes = fixture(name);
            let receipt =
                evaluate_request_bytes(&bytes, REV).expect("supported analytical request");
            assert_eq!(receipt["result_status"], "PASS");
            assert_eq!(receipt["evidence_class"], "analytical_reference");
            assert!(receipt["outputs"].is_object());
            assert!(receipt["output_units"].is_object());
            assert_eq!(receipt["request_sha256"], sha256_hex(&bytes));
        }
    }

    #[test]
    fn unsupported_physics_produces_unresolved_receipt_not_weak_fallback() {
        let receipt = evaluate_request_bytes(&fixture("unsupported-fracture-v0.1.json"), REV)
            .expect("well-formed unsupported request yields a receipt");
        assert_eq!(receipt["result_status"], "UNRESOLVED");
        assert_eq!(receipt["outputs"], Value::Null);
        assert!(
            receipt["physics_omitted"]
                .as_array()
                .unwrap()
                .iter()
                .any(|v| v == "fracture")
        );
    }

    #[test]
    fn input_mutations_fail_closed() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["units"]["length"] = json!("ft");
        assert!(evaluate_request_bytes(request.to_string().as_bytes(), REV).is_err());

        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["assumption_profile"] = json!("generic_collapse");
        assert!(evaluate_request_bytes(request.to_string().as_bytes(), REV).is_err());

        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["inputs"]["area_m2"] = json!(-1);
        assert!(evaluate_request_bytes(request.to_string().as_bytes(), REV).is_err());

        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["unreviewed_override"] = json!(true);
        assert!(evaluate_request_bytes(request.to_string().as_bytes(), REV).is_err());
    }

    #[test]
    fn formula_overflow_returns_unresolved_receipt() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-v0.1.json")).unwrap();
        request["inputs"]["force_n"] = json!(1.0e308);
        request["inputs"]["length_m"] = json!(1.0e308);
        let receipt = evaluate_request_bytes(&request.to_string().into_bytes(), REV)
            .expect("well-shaped but out-of-domain finite values produce unresolved receipt");
        assert_eq!(receipt["result_status"], "UNRESOLVED");
        assert_eq!(receipt["outputs"], Value::Null);
    }
}
