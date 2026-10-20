// SPDX-License-Identifier: AGPL-3.0-or-later
//! Executes only three qualified closed-form reference cases.
//!
//! PASS means that the named formula evaluated under its declared assumptions.
//! It is not a finding about the safety of a real structure.

pub mod uncertainty;

use physical_transformation_reference::mechanics::{
    axial_bar, cantilever_tip_load, simply_supported_uniform_load,
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub type Check<T> = Result<T, String>;

const REQUEST_SCHEMA: &str = "physical-analytical-request-v0.1";
const REQUEST_SCHEMA_V2: &str = "physical-analytical-request-v0.2";
const RECEIPT_SCHEMA: &str = "physical-analytical-receipt-v0.1";
const RECEIPT_SCHEMA_V2: &str = "physical-analytical-receipt-v0.2";
const REQUEST_KEYS: &[&str] = &[
    "schema_version",
    "request_id",
    "source_revision",
    "case_id",
    "assumption_profile",
    "requested_physics",
    "units",
    "inputs",
];
const REQUEST_KEYS_V2: &[&str] = &[
    "schema_version",
    "request_id",
    "source_revision",
    "case_id",
    "assumption_profile",
    "requested_physics",
    "units",
    "inputs",
    "input_bounds",
];
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
            Self::AxialBar => {
                "Uniform axial bar, constant cross-sectional area, static linear elasticity; tension is positive and compression negative; small strain."
            }
            Self::CantileverTipLoad => {
                "Static, small-deflection, linear-elastic Euler-Bernoulli cantilever with constant section and a point load at the free tip. Force sign determines deflection direction; maximum bending stress is reported as a non-negative magnitude."
            }
            Self::SimplySupportedUniformLoad => {
                "Static, small-deflection, linear-elastic Euler-Bernoulli simply supported prismatic beam with constant section and uniform load across the full span. Load sign determines deflection direction; maximum bending stress is reported as a non-negative magnitude."
            }
        }
    }
}

#[derive(Clone, Debug)]
struct ParsedRequest {
    schema_version: String,
    request_id: String,
    source_revision: String,
    case: CaseKind,
    requested_physics: Vec<String>,
    physics_omitted: Vec<String>,
    inputs: Map<String, Value>,
    input_bounds: Option<BTreeMap<String, uncertainty::Interval>>,
    input_bounds_raw: Option<Value>,
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
            None,
            diagnostics,
        ));
    }

    match evaluate_case(request.case, &request.inputs) {
        Ok(outputs) => {
            let interval_result = match request.input_bounds.as_ref() {
                Some(bounds) => build_output_intervals(&request, &outputs, bounds).map(Some),
                None => Ok(None),
            };
            match interval_result {
                Ok(output_intervals) => Ok(receipt(
                    &request,
                    adapter_revision,
                    &input_digest,
                    "PASS",
                    outputs,
                    output_intervals,
                    Vec::new(),
                )),
                Err(error) => Ok(receipt(
                    &request,
                    adapter_revision,
                    &input_digest,
                    "UNRESOLVED",
                    Value::Null,
                    None,
                    vec![format!(
                        "bounded analytical evaluation could not be completed: {error}"
                    )],
                )),
            }
        }
        Err(error) => Ok(receipt(
            &request,
            adapter_revision,
            &input_digest,
            "UNRESOLVED",
            Value::Null,
            None,
            vec![format!(
                "analytical evaluation could not be completed: {error}"
            )],
        )),
    }
}

fn receipt(
    request: &ParsedRequest,
    adapter_revision: &str,
    request_digest: &str,
    status: &str,
    outputs: Value,
    output_intervals: Option<Value>,
    diagnostics: Vec<String>,
) -> Value {
    let schema_version = if request.schema_version == REQUEST_SCHEMA_V2 {
        RECEIPT_SCHEMA_V2
    } else {
        RECEIPT_SCHEMA
    };
    let mut result = json!({
        "schema_version": schema_version,
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
    });
    if request.schema_version == REQUEST_SCHEMA_V2 {
        result["input_bounds"] = request.input_bounds_raw.clone().unwrap_or(Value::Null);
        result["output_intervals"] = output_intervals.unwrap_or(Value::Null);
        result["interval_semantics"] = json!(
            "deterministic output enclosure conditional on declared input bounds; not a probability distribution or confidence interval"
        );
    }
    result
}

fn build_output_intervals(
    request: &ParsedRequest,
    outputs: &Value,
    bounds: &BTreeMap<String, uncertainty::Interval>,
) -> Check<Value> {
    let nominal_inputs = request
        .inputs
        .iter()
        .map(|(key, value)| {
            let number = value
                .as_f64()
                .filter(|number| number.is_finite())
                .ok_or_else(|| format!("request.inputs.{key} must be finite"))?;
            Ok((key.clone(), number))
        })
        .collect::<Check<BTreeMap<String, f64>>>()?;
    let intervals = uncertainty::propagate(request.case.id(), &nominal_inputs, bounds)?;
    let output_units = request.case.units();
    let mut result = Map::new();
    for (key, interval) in intervals {
        let nominal = outputs
            .get(&key)
            .and_then(Value::as_f64)
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("nominal output '{key}' is missing or non-finite"))?;
        if !interval.contains(nominal) || interval.lower > interval.upper {
            return Err(format!(
                "output interval '{key}' does not enclose its nominal value"
            ));
        }
        let unit = output_units
            .get(&key)
            .and_then(Value::as_str)
            .ok_or_else(|| format!("unit for analytical output '{key}' is missing"))?;
        result.insert(
            key,
            json!({
                "lower": interval.lower,
                "nominal": nominal,
                "upper": interval.upper,
                "unit": unit
            }),
        );
    }
    Ok(Value::Object(result))
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
    let schema_version = string(value, "schema_version", "request")?;
    let expected_keys = match schema_version.as_str() {
        REQUEST_SCHEMA => REQUEST_KEYS,
        REQUEST_SCHEMA_V2 if value.get("input_bounds").is_some() => REQUEST_KEYS_V2,
        REQUEST_SCHEMA_V2 => REQUEST_KEYS,
        _ => {
            return Err(format!(
                "unsupported request schema version '{schema_version}'"
            ));
        }
    };
    exact_keys(value, expected_keys, "request")?;
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

    let (input_bounds, input_bounds_raw) = if let Some(raw_bounds) = value.get("input_bounds") {
        if schema_version != REQUEST_SCHEMA_V2 {
            return Err("input_bounds requires physical-analytical-request-v0.2".to_owned());
        }
        let bounds = parse_input_bounds(raw_bounds, case, inputs)?;
        (Some(bounds), Some(raw_bounds.clone()))
    } else {
        (None, None)
    };

    Ok(ParsedRequest {
        schema_version,
        request_id,
        source_revision,
        case,
        requested_physics,
        physics_omitted,
        inputs: inputs.to_owned(),
        input_bounds,
        input_bounds_raw,
    })
}

fn parse_input_bounds(
    raw_bounds: &Value,
    case: CaseKind,
    inputs: &Map<String, Value>,
) -> Check<BTreeMap<String, uncertainty::Interval>> {
    let bounds = raw_bounds
        .as_object()
        .ok_or_else(|| "request.input_bounds must be an object".to_owned())?;
    exact_keys_map(bounds, case.input_keys(), "request.input_bounds")?;
    let mut result = BTreeMap::new();
    for key in case.input_keys() {
        let at = format!("request.input_bounds.{key}");
        let entry = bounds
            .get(*key)
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{at} must be an object"))?;
        exact_keys_map(
            entry,
            &["lower", "upper", "unit", "evidence_class", "source_ref"],
            &at,
        )?;
        let lower = entry
            .get("lower")
            .and_then(Value::as_f64)
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("{at}.lower must be finite"))?;
        let upper = entry
            .get("upper")
            .and_then(Value::as_f64)
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("{at}.upper must be finite"))?;
        let interval =
            uncertainty::Interval::new(lower, upper).map_err(|error| format!("{at}: {error}"))?;
        let nominal = inputs
            .get(*key)
            .and_then(Value::as_f64)
            .filter(|number| number.is_finite())
            .ok_or_else(|| format!("request.inputs.{key} must be finite"))?;
        if !interval.contains(nominal) {
            return Err(format!(
                "nominal input '{key}' lies outside request.input_bounds"
            ));
        }
        let unit = map_string(entry, "unit", &at)?;
        if unit != expected_input_unit(key) {
            return Err(format!("{at}.unit must be '{}'", expected_input_unit(key)));
        }
        let evidence = map_string(entry, "evidence_class", &at)?;
        if !["specified", "measured", "estimated", "synthetic", "unknown"]
            .contains(&evidence.as_str())
        {
            return Err(format!("{at}.evidence_class is unsupported"));
        }
        let source_ref = entry
            .get("source_ref")
            .ok_or_else(|| format!("{at}.source_ref is missing"))?;
        if !(source_ref.is_null() || source_ref.as_str().is_some()) {
            return Err(format!("{at}.source_ref must be a string or null"));
        }
        if let Some(source_ref) = source_ref.as_str()
            && source_ref.trim().is_empty()
        {
            return Err(format!("{at}.source_ref must not be empty"));
        }
        if evidence == "measured" && source_ref.as_str().is_none() {
            return Err(format!("{at}: measured bounds require a source_ref"));
        }
        result.insert((*key).to_owned(), interval);
    }
    Ok(result)
}

fn expected_input_unit(key: &str) -> &'static str {
    match key {
        "force_n" => "N",
        "load_n_per_m" => "N/m",
        "length_m" | "outer_fiber_distance_m" => "m",
        "area_m2" => "m^2",
        "youngs_modulus_pa" => "Pa",
        "second_moment_area_m4" => "m^4",
        _ => "unknown",
    }
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    exact_keys_map(object, expected, at)
}

fn exact_keys_map(actual: &Map<String, Value>, expected: &[&str], at: &str) -> Check<()> {
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
        return Err(format!(
            "{label} must be {length} lowercase hexadecimal characters"
        ));
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
    fn bounded_v0_2_request_emits_ordered_output_enclosures() {
        let bytes = fixture("analytical-bar-bounds-v0.2.json");
        let receipt = evaluate_request_bytes(&bytes, REV).expect("valid bounded request");
        assert_eq!(receipt["schema_version"], RECEIPT_SCHEMA_V2);
        assert_eq!(receipt["result_status"], "PASS");
        assert_eq!(
            receipt["input_bounds"]["force_n"]["evidence_class"],
            "synthetic"
        );
        assert!(
            receipt["interval_semantics"]
                .as_str()
                .unwrap()
                .contains("not a probability distribution")
        );
        let nominal = receipt["outputs"]["axial_stress_pa"]
            .as_f64()
            .expect("nominal stress");
        let range = &receipt["output_intervals"]["axial_stress_pa"];
        assert!(range["lower"].as_f64().unwrap() < nominal);
        assert_eq!(range["nominal"].as_f64(), Some(nominal));
        assert!(range["upper"].as_f64().unwrap() > nominal);
        assert_eq!(range["unit"], "Pa");
        for (name, value) in receipt["outputs"].as_object().unwrap() {
            let range = &receipt["output_intervals"][name];
            assert!(range["lower"].as_f64().unwrap() <= value.as_f64().unwrap());
            assert!(value.as_f64().unwrap() <= range["upper"].as_f64().unwrap());
        }
    }


    #[test]
    fn bounded_simply_supported_request_encloses_deflection_and_stress() {
        let receipt = evaluate_request_bytes(
            &fixture("simply-supported-bounds-v0.2.json"),
            REV,
        )
        .expect("bounded simply supported request");
        assert_eq!(receipt["schema_version"], RECEIPT_SCHEMA_V2);
        assert_eq!(receipt["result_status"], "PASS");
        for key in ["midspan_deflection_m", "maximum_bending_stress_pa"] {
            let nominal = receipt["outputs"][key].as_f64().expect("nominal output");
            let range = &receipt["output_intervals"][key];
            assert!(range["lower"].as_f64().unwrap() <= nominal);
            assert_eq!(range["nominal"].as_f64(), Some(nominal));
            assert!(nominal <= range["upper"].as_f64().unwrap());
        }
        assert_eq!(receipt["output_intervals"]["midspan_deflection_m"]["unit"], "m");
        assert_eq!(receipt["output_intervals"]["maximum_bending_stress_pa"]["unit"], "Pa");
    }

    #[test]
    fn point_only_v0_1_request_keeps_legacy_receipt_shape() {
        let receipt = evaluate_request_bytes(&fixture("analytical-bar-v0.1.json"), REV)
            .expect("existing point-only request");
        assert_eq!(receipt["schema_version"], RECEIPT_SCHEMA);
        assert_eq!(receipt["result_status"], "PASS");
        assert!(receipt.get("output_intervals").is_none());
        assert!(receipt.get("input_bounds").is_none());
    }

    #[test]
    fn point_only_v0_2_request_does_not_invent_input_ranges() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-bounds-v0.2.json")).unwrap();
        request.as_object_mut().unwrap().remove("input_bounds");
        let receipt = evaluate_request_bytes(request.to_string().as_bytes(), REV)
            .expect("v0.2 point-only request");
        assert_eq!(receipt["schema_version"], RECEIPT_SCHEMA_V2);
        assert_eq!(receipt["result_status"], "PASS");
        assert_eq!(receipt["input_bounds"], Value::Null);
        assert_eq!(receipt["output_intervals"], Value::Null);
    }

    #[test]
    fn v0_2_bounds_reject_nominal_mismatch_units_and_unknown_keys() {
        let fixture = fixture("analytical-bar-bounds-v0.2.json");
        let mut request: Value = serde_json::from_slice(&fixture).unwrap();
        request["inputs"]["force_n"] = json!(1_200.0);
        assert!(
            evaluate_request_bytes(request.to_string().as_bytes(), REV)
                .unwrap_err()
                .contains("lies outside request.input_bounds")
        );

        let mut request: Value = serde_json::from_slice(&fixture).unwrap();
        request["input_bounds"]["area_m2"]["unit"] = json!("m");
        assert!(
            evaluate_request_bytes(request.to_string().as_bytes(), REV)
                .unwrap_err()
                .contains("input_bounds.area_m2.unit")
        );

        let mut request: Value = serde_json::from_slice(&fixture).unwrap();
        request["input_bounds"]["area_m2"]["lower"] = json!(0.012);
        assert!(evaluate_request_bytes(request.to_string().as_bytes(), REV).is_err());

        let mut request: Value = serde_json::from_slice(&fixture).unwrap();
        request["input_bounds"]["extra_parameter"] = json!({
            "lower":0.0,"upper":1.0,"unit":"m","evidence_class":"synthetic","source_ref":"fixture-only"
        });
        assert!(evaluate_request_bytes(request.to_string().as_bytes(), REV).is_err());
    }

    #[test]
    fn zero_crossing_load_yields_zero_lower_bound_for_absolute_stress() {
        let receipt =
            evaluate_request_bytes(&fixture("cantilever-zero-crossing-bounds-v0.2.json"), REV)
                .expect("zero-crossing bounded cantilever request");
        assert_eq!(receipt["result_status"], "PASS");
        assert_eq!(receipt["outputs"]["maximum_bending_stress_pa"], 0.0);
        assert_eq!(
            receipt["output_intervals"]["maximum_bending_stress_pa"]["lower"],
            0.0
        );
        assert!(
            receipt["output_intervals"]["maximum_bending_stress_pa"]["upper"]
                .as_f64()
                .unwrap()
                > 0.0
        );
    }

    #[test]
    fn interval_overflow_returns_unresolved_v0_2_receipt_without_outputs() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-bounds-v0.2.json")).unwrap();
        request["input_bounds"]["force_n"]["upper"] = json!(1.0e308);
        request["input_bounds"]["length_m"]["upper"] = json!(1.0e308);
        request["input_bounds"]["area_m2"]["lower"] = json!(0.009);
        request["input_bounds"]["youngs_modulus_pa"]["lower"] = json!(1.0);
        request["inputs"]["youngs_modulus_pa"] = json!(200.0e9);
        let receipt = evaluate_request_bytes(request.to_string().as_bytes(), REV)
            .expect("a well-formed request with overflowing bounds yields an unresolved receipt");
        assert_eq!(receipt["result_status"], "UNRESOLVED");
        assert_eq!(receipt["outputs"], Value::Null);
        assert_eq!(receipt["output_intervals"], Value::Null);
        assert!(receipt["diagnostics"].as_array().unwrap().iter().any(|v| {
            v.as_str()
                .unwrap()
                .contains("bounded analytical evaluation")
        }));
    }

    #[test]
    fn bounded_request_with_unsupported_physics_is_unresolved_without_partial_ranges() {
        let mut request: Value =
            serde_json::from_slice(&fixture("analytical-bar-bounds-v0.2.json")).unwrap();
        request["requested_physics"] = json!(["static_linear_elastic", "fracture"]);
        let receipt = evaluate_request_bytes(request.to_string().as_bytes(), REV)
            .expect("well-formed unsupported request yields receipt");
        assert_eq!(receipt["schema_version"], RECEIPT_SCHEMA_V2);
        assert_eq!(receipt["result_status"], "UNRESOLVED");
        assert_eq!(receipt["outputs"], Value::Null);
        assert_eq!(receipt["output_intervals"], Value::Null);
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
