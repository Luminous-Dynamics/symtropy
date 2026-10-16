// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independently compares a benchmark run's parsed outputs against an externally
//! pinned analytical reference. It does not authenticate that the declared
//! evaluator actually executed or establish real-world engineering adequacy.

use jsonschema::draft202012;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

const RECORD_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/benchmark-result.schema.json");
const OUTPUT_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/benchmark-result-output.schema.json");
const REPORT_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/benchmark-result-report.schema.json");

#[derive(Debug, Serialize)]
pub struct QuantityComparison {
    pub quantity: String,
    pub actual_value: Option<f64>,
    pub actual_unit: Option<String>,
    pub expected_value: f64,
    pub expected_unit: String,
    pub absolute_error: Option<f64>,
    pub allowed_error: f64,
    pub within_tolerance: bool,
}

#[derive(Debug, Serialize)]
pub struct BenchmarkResultReport {
    pub validator_version: String,
    pub benchmark_id: Option<String>,
    pub case_id: Option<String>,
    pub validator_binary_sha256: Option<String>,
    pub checker_pin_matches: bool,
    pub record_schema_sha256: String,
    pub output_schema_sha256: String,
    pub report_schema_sha256: String,
    pub benchmark_sha256_actual: String,
    pub benchmark_pin_matches: bool,
    pub result_record_sha256: String,
    pub record_schema_valid: bool,
    pub record_benchmark_sha256_matches: bool,
    pub benchmark_identity_matches: bool,
    pub execution_completed: bool,
    pub raw_output_sha256_actual: Option<String>,
    pub raw_output_digest_matches: bool,
    pub output_schema_valid: bool,
    pub reference_case_resolved: bool,
    pub outputs_match_reference: bool,
    pub quantity_comparisons: Vec<QuantityComparison>,
    pub declared_status: Option<String>,
    pub declared_status_consistent: Option<bool>,
    pub independent_result_status: String,
    pub issues: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Clone, Debug)]
struct ExpectedOutput {
    value: f64,
    unit: &'static str,
}

#[derive(Clone, Debug)]
struct ObservedOutput {
    value: f64,
    unit: String,
}

pub fn validate_paths(
    benchmark_path: &Path,
    record_path: &Path,
    expected_benchmark_sha256: &str,
    expected_checker_sha256: &str,
    artifact_root: &Path,
) -> Result<BenchmarkResultReport, String> {
    if !is_sha256_hex(expected_benchmark_sha256) {
        return Err(
            "expected benchmark SHA-256 must be 64 lowercase hexadecimal characters".to_string(),
        );
    }
    if !is_sha256_hex(expected_checker_sha256) {
        return Err(
            "expected checker SHA-256 must be 64 lowercase hexadecimal characters".to_string(),
        );
    }

    let validator_binary_sha256 = std::env::current_exe()
        .ok()
        .and_then(|path| fs::read(path).ok())
        .map(|bytes| sha256_hex(&bytes));
    let checker_pin_matches =
        validator_binary_sha256.as_deref() == Some(expected_checker_sha256);
    let record_schema_sha256 = sha256_hex(RECORD_SCHEMA.as_bytes());
    let output_schema_sha256 = sha256_hex(OUTPUT_SCHEMA.as_bytes());
    let report_schema_sha256 = sha256_hex(REPORT_SCHEMA.as_bytes());

    let benchmark_bytes = fs::read(benchmark_path)
        .map_err(|error| format!("cannot read {}: {error}", benchmark_path.display()))?;
    let benchmark_sha256_actual = sha256_hex(&benchmark_bytes);
    let benchmark_pin_matches = benchmark_sha256_actual == expected_benchmark_sha256;
    let benchmark: Value = serde_json::from_slice(&benchmark_bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", benchmark_path.display()))?;

    let record_bytes = fs::read(record_path)
        .map_err(|error| format!("cannot read {}: {error}", record_path.display()))?;
    let result_record_sha256 = sha256_hex(&record_bytes);
    let record: Value = serde_json::from_slice(&record_bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", record_path.display()))?;

    let mut issues = Vec::new();
    if validator_binary_sha256.is_none() {
        issues.push("cannot determine the running checker executable SHA-256".to_string());
    } else if !checker_pin_matches {
        issues.push(format!(
            "checker executable does not match out-of-band SHA-256 pin (actual {})",
            validator_binary_sha256.as_deref().unwrap_or_default()
        ));
    }
    if !benchmark_pin_matches {
        issues.push(format!(
            "benchmark bytes do not match the out-of-band SHA-256 pin (actual {benchmark_sha256_actual})"
        ));
    }

    let record_schema_errors =
        validate_against_schema(RECORD_SCHEMA, &record, "benchmark result record")?;
    let record_schema_valid = record_schema_errors.is_empty();
    issues.extend(record_schema_errors);

    let record_benchmark_sha256_matches = record
        .get("benchmark_sha256")
        .and_then(Value::as_str)
        == Some(benchmark_sha256_actual.as_str());
    if !record_benchmark_sha256_matches {
        issues.push(
            "result record benchmark_sha256 does not match the exact benchmark bytes".to_string(),
        );
    }

    let benchmark_id = benchmark.get("benchmark_id").and_then(Value::as_str);
    let record_benchmark_id = record.get("benchmark_id").and_then(Value::as_str);
    let case_id = record.get("case_id").and_then(Value::as_str);
    let benchmark_identity_matches = benchmark_id.is_some()
        && benchmark_id == record_benchmark_id
        && record_benchmark_id.is_some()
        && case_id.is_some();
    if !benchmark_identity_matches {
        issues.push("benchmark_id or case_id does not match the selected benchmark".to_string());
    }

    let execution_kind = record["execution"]["execution_kind"].as_str();
    let execution_completed = record.get("execution").is_some_and(|execution| {
        execution.get("exit_code").and_then(Value::as_i64) == Some(0)
            && execution.get("termination_status").and_then(Value::as_str) == Some("COMPLETED")
            && execution.get("execution_kind").and_then(Value::as_str)
                == Some("ANALYTICAL_REFERENCE")
            && execution.get("convergence_status").and_then(Value::as_str)
                == Some("NOT_APPLICABLE")
    });
    if !execution_completed {
        issues.push(
            if execution_kind == Some("NUMERICAL_SOLVER") {
                "numerical solver results are unsupported until solver-specific acceptance profiles and authenticated execution are implemented".to_string()
            } else {
                "execution did not complete successfully or analytical-reference convergence must be NOT_APPLICABLE".to_string()
            },
        );
    }

    let mut raw_output_sha256_actual = None;
    let mut raw_output_digest_matches = false;
    let mut output_schema_valid = false;
    let mut outputs_match_reference = false;
    let mut quantity_comparisons = Vec::new();
    let mut reference_case_resolved = false;

    let raw_artifact = record.get("raw_output_artifact");
    let relative_path = raw_artifact
        .and_then(|artifact| artifact.get("relative_path"))
        .and_then(Value::as_str);
    let expected_output_sha256 = raw_artifact
        .and_then(|artifact| artifact.get("sha256"))
        .and_then(Value::as_str);

    let raw_output_path = match relative_path {
        Some(path) => match safe_artifact_path(artifact_root, path) {
            Ok(path) => Some(path),
            Err(error) => {
                issues.push(format!("raw output artifact path is unsafe: {error}"));
                None
            }
        },
        None => {
            issues.push("result record is missing raw output artifact path".to_string());
            None
        }
    };

    let mut output_value: Option<Value> = None;
    if let Some(path) = raw_output_path {
        match fs::read(&path) {
            Ok(bytes) => {
                let actual_digest = sha256_hex(&bytes);
                raw_output_sha256_actual = Some(actual_digest.clone());
                raw_output_digest_matches = expected_output_sha256 == Some(actual_digest.as_str());
                if !raw_output_digest_matches {
                    issues.push(format!(
                        "raw output artifact SHA-256 mismatch (actual {actual_digest})"
                    ));
                }
                match serde_json::from_slice::<Value>(&bytes) {
                    Ok(value) => {
                        let schema_errors =
                            validate_against_schema(OUTPUT_SCHEMA, &value, "benchmark raw output")?;
                        output_schema_valid = schema_errors.is_empty();
                        issues.extend(schema_errors);
                        output_value = Some(value);
                    }
                    Err(error) => issues.push(format!("raw output artifact is not valid JSON: {error}")),
                }
            }
            Err(error) => issues.push(format!("cannot read raw output artifact: {error}")),
        }
    }

    let declared_status = record
        .get("declared_status")
        .and_then(Value::as_str)
        .map(str::to_owned);

    let mut output_identity_matches = false;
    if let (Some(output), Some(expected_id), Some(expected_case_id)) =
        (output_value.as_ref(), benchmark_id, case_id)
    {
        output_identity_matches = output.get("benchmark_id").and_then(Value::as_str)
            == Some(expected_id)
            && output.get("case_id").and_then(Value::as_str) == Some(expected_case_id);
        if !output_identity_matches {
            issues.push("raw output benchmark_id/case_id does not match the pinned reference".to_string());
        }

        match expected_outputs(&benchmark, expected_id, expected_case_id) {
            Ok((expected, tolerance)) => {
                reference_case_resolved = true;
                let (matches, comparisons) = compare_outputs(
                    output.get("outputs"),
                    &expected,
                    tolerance,
                    &mut issues,
                );
                outputs_match_reference = matches;
                quantity_comparisons = comparisons;
            }
            Err(error) => issues.push(error),
        }
    }

    let blocked = validator_binary_sha256.is_none()
        || !checker_pin_matches
        || !benchmark_pin_matches
        || !record_schema_valid
        || !record_benchmark_sha256_matches
        || !benchmark_identity_matches
        || !execution_completed
        || !raw_output_digest_matches
        || !output_schema_valid
        || !output_identity_matches
        || !reference_case_resolved;
    let independent_result_status = if blocked {
        "BLOCKED"
    } else if outputs_match_reference {
        "REFERENCE_MATCH"
    } else {
        "REFERENCE_MISMATCH"
    };
    let declared_status_consistent = if blocked {
        None
    } else {
        match (declared_status.as_deref(), outputs_match_reference) {
            (Some("CLAIMED_PASS"), true) => Some(true),
            (Some("CLAIMED_FAIL"), false) => Some(true),
            (Some("CLAIMED_PASS" | "CLAIMED_FAIL"), _) => Some(false),
            _ => None,
        }
    };
    if declared_status_consistent == Some(false) {
        issues.push(
            "producer-declared gate outcome contradicts the independently checked reference result"
                .to_string(),
        );
    }

    let report = BenchmarkResultReport {
        validator_version: env!("CARGO_PKG_VERSION").to_string(),
        benchmark_id: benchmark_id.map(str::to_owned),
        case_id: case_id.map(str::to_owned),
        validator_binary_sha256,
        checker_pin_matches,
        record_schema_sha256,
        output_schema_sha256,
        report_schema_sha256,
        benchmark_sha256_actual,
        benchmark_pin_matches,
        result_record_sha256,
        record_schema_valid,
        record_benchmark_sha256_matches,
        benchmark_identity_matches,
        execution_completed,
        raw_output_sha256_actual,
        raw_output_digest_matches,
        output_schema_valid,
        reference_case_resolved,
        outputs_match_reference,
        quantity_comparisons,
        declared_status,
        declared_status_consistent,
        independent_result_status: independent_result_status.to_string(),
        issues,
        limitations: vec![
            "This command compares parsed numeric outputs to a pinned analytical reference; it does not independently execute the producer's solver.".to_string(),
            "Evaluator identity, exit status and convergence state are producer-declared metadata, not authenticated execution attestations.".to_string(),
            "A reference match verifies only this benchmark case and its listed output quantities; it does not validate a finite-element solver generally, a real structure, legal compliance or building safety.".to_string(),
            "The benchmark's out-of-band digest and acceptance criteria must be approved by a trusted independent authority.".to_string(),
        ],
    };
    let report_value = serde_json::to_value(&report)
        .map_err(|error| format!("cannot serialize benchmark result report: {error}"))?;
    let report_schema_errors =
        validate_against_schema(REPORT_SCHEMA, &report_value, "benchmark result report")?;
    if !report_schema_errors.is_empty() {
        return Err(format!(
            "constructed benchmark result report violates its schema: {}",
            report_schema_errors.join("; ")
        ));
    }
    Ok(report)
}

fn require_derived_input(
    inputs: &Value,
    name: &str,
    computed: f64,
    tolerance: f64,
) -> Result<(), String> {
    if !computed.is_finite() || computed <= 0.0 {
        return Err(format!(
            "equation-derived benchmark input {name:?} is not finite and positive"
        ));
    }
    let declared = inputs
        .get(name)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| format!("derived benchmark input {name:?} must be finite and positive"))?;
    let allowed_error = tolerance * computed.abs().max(f64::MIN_POSITIVE);
    let actual_error = (declared - computed).abs();
    if actual_error > allowed_error {
        return Err(format!(
            "derived benchmark input {name:?} contradicts primitive geometry: declared {declared}, equation-derived {computed}, allowed error {allowed_error}"
        ));
    }
    Ok(())
}

fn expected_outputs(
    benchmark: &Value,
    benchmark_id: &str,
    case_id: &str,
) -> Result<(BTreeMap<String, ExpectedOutput>, f64), String> {
    let supported = matches!(
        benchmark_id,
        "structural/cantilever-beam-linear-static/v1"
            | "structural/axial-bar-linear-static/v1"
            | "structural/simply-supported-beam-linear-static/v1"
    );
    if !supported {
        return Err(format!("unsupported analytical benchmark {benchmark_id:?}"));
    }
    if benchmark.get("status").and_then(Value::as_str) != Some("analytical_reference_only") {
        return Err(
            "supported benchmark fixture must explicitly declare analytical_reference_only"
                .to_string(),
        );
    }

    // These three fixtures use elementary closed-form equations. A loose
    // tolerance must not be allowed to turn a poor match into an accepted one.
    // Numerical/FEM profiles require their own separately reviewed policy.
    let tolerance = benchmark["acceptance"]["relative_tolerance"]
        .as_f64()
        .filter(|value| value.is_finite() && *value >= 0.0 && *value <= 1e-9)
        .ok_or_else(|| {
            "analytical benchmark relative_tolerance must be finite, non-negative, and no greater than 1e-9".to_string()
        })?;
    let inputs = benchmark
        .get("inputs")
        .ok_or_else(|| "benchmark inputs must be an object".to_string())?;
    let read_positive = |object: &Value, key: &str| -> Result<f64, String> {
        let value = object
            .get(key)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("benchmark input {key:?} must be numeric"))?;
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("benchmark input {key:?} must be finite and positive"));
        }
        Ok(value)
    };

    let (calculated, declared_expected): (BTreeMap<String, f64>, &Value) = match benchmark_id {
        "structural/cantilever-beam-linear-static/v1" => {
            if case_id != "default" {
                return Err(format!(
                    "single-case benchmark requires case_id \"default\", not {case_id:?}"
                ));
            }
            let load = read_positive(inputs, "load_N")?;
            let span = read_positive(inputs, "span_m")?;
            let modulus = read_positive(inputs, "elastic_modulus_Pa")?;
            let width = read_positive(inputs, "section_width_m")?;
            let depth = read_positive(inputs, "section_depth_m")?;
            let inertia = width * depth.powi(3) / 12.0;
            let fiber = depth / 2.0;
            require_derived_input(inputs, "second_moment_area_m4", inertia, tolerance)?;
            require_derived_input(inputs, "extreme_fiber_distance_m", fiber, tolerance)?;
            let values = BTreeMap::from([
                ("second_moment_area_m4".to_string(), inertia),
                (
                    "free_end_deflection_m".to_string(),
                    load * span.powi(3) / (3.0 * modulus * inertia),
                ),
                (
                    "support_maximum_bending_stress_Pa".to_string(),
                    load * span * fiber / inertia,
                ),
            ]);
            let expected = benchmark
                .get("expected")
                .ok_or_else(|| "cantilever benchmark expected object is missing".to_string())?;
            (values, expected)
        }
        "structural/axial-bar-linear-static/v1" => {
            if case_id != "default" {
                return Err(format!(
                    "single-case benchmark requires case_id \"default\", not {case_id:?}"
                ));
            }
            let load = read_positive(inputs, "load_N")?;
            let length = read_positive(inputs, "length_m")?;
            let modulus = read_positive(inputs, "elastic_modulus_Pa")?;
            let width = read_positive(inputs, "section_width_m")?;
            let height = read_positive(inputs, "section_height_m")?;
            let area = width * height;
            require_derived_input(inputs, "cross_section_area_m2", area, tolerance)?;
            let values = BTreeMap::from([
                ("cross_section_area_m2".to_string(), area),
                ("axial_stress_Pa".to_string(), load / area),
                (
                    "free_end_extension_m".to_string(),
                    load * length / (modulus * area),
                ),
                ("support_reaction_magnitude_N".to_string(), load),
            ]);
            let expected = benchmark
                .get("expected")
                .ok_or_else(|| "axial-bar benchmark expected object is missing".to_string())?;
            (values, expected)
        }
        "structural/simply-supported-beam-linear-static/v1" => {
            let load_cases = benchmark
                .get("load_cases")
                .and_then(Value::as_array)
                .ok_or_else(|| "simply-supported benchmark load_cases must be an array".to_string())?;
            let mut matching = load_cases.iter().filter(|item| {
                item.get("case_id").and_then(Value::as_str) == Some(case_id)
            });
            let selected = matching
                .next()
                .ok_or_else(|| format!("benchmark has no load case {case_id:?}"))?;
            if matching.next().is_some() {
                return Err(format!("benchmark duplicates load case ID {case_id:?}"));
            }
            let span = read_positive(inputs, "span_m")?;
            let modulus = read_positive(inputs, "elastic_modulus_Pa")?;
            let width = read_positive(inputs, "section_width_m")?;
            let depth = read_positive(inputs, "section_depth_m")?;
            let inertia = width * depth.powi(3) / 12.0;
            let fiber = depth / 2.0;
            require_derived_input(inputs, "second_moment_area_m4", inertia, tolerance)?;
            require_derived_input(inputs, "extreme_fiber_distance_m", fiber, tolerance)?;
            let (deflection, moment, reaction) =
                match selected.get("load_type").and_then(Value::as_str) {
                    Some("point_load_midspan") => {
                        let load = read_positive(selected, "load_N")?;
                        (
                            load * span.powi(3) / (48.0 * modulus * inertia),
                            load * span / 4.0,
                            load / 2.0,
                        )
                    }
                    Some("uniform_distributed_load") => {
                        let load_per_m = read_positive(selected, "load_N_per_m")?;
                        (
                            5.0 * load_per_m * span.powi(4) / (384.0 * modulus * inertia),
                            load_per_m * span.powi(2) / 8.0,
                            load_per_m * span / 2.0,
                        )
                    }
                    Some(other) => return Err(format!("unsupported load_type {other:?}")),
                    None => return Err("load case load_type must be a string".to_string()),
                };
            let values = BTreeMap::from([
                ("maximum_deflection_m".to_string(), deflection),
                ("maximum_bending_moment_Nm".to_string(), moment),
                (
                    "maximum_bending_stress_Pa".to_string(),
                    moment * fiber / inertia,
                ),
                ("support_reaction_magnitude_N".to_string(), reaction),
            ]);
            let expected = selected
                .get("expected")
                .ok_or_else(|| format!("load case {case_id:?} has no expected object"))?;
            (values, expected)
        }
        _ => unreachable!("supported benchmark checked above"),
    };

    if calculated.is_empty()
        || calculated
            .values()
            .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(
            "derived benchmark reference outputs must be finite, positive, and non-empty"
                .to_string(),
        );
    }
    let declared = declared_expected
        .as_object()
        .ok_or_else(|| "benchmark expected outputs must be an object".to_string())?;
    if declared.len() != calculated.len() {
        return Err(format!(
            "benchmark oracle declares {} outputs but equations derive {}",
            declared.len(),
            calculated.len()
        ));
    }

    let mut expected = BTreeMap::new();
    for (quantity, calculated_value) in calculated {
        let unit = unit_for_quantity(&quantity)
            .ok_or_else(|| format!("unsupported expected quantity {quantity:?}"))?;
        let declared_value = declared
            .get(&quantity)
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("benchmark oracle omits finite numeric value for {quantity:?}"))?;
        let allowed_error = tolerance * calculated_value.abs().max(f64::MIN_POSITIVE);
        let actual_error = (declared_value - calculated_value).abs();
        if actual_error > allowed_error {
            return Err(format!(
                "benchmark oracle drift for {quantity:?}: declared {declared_value}, equation-derived {calculated_value}, allowed error {allowed_error}"
            ));
        }
        expected.insert(
            quantity,
            ExpectedOutput {
                value: calculated_value,
                unit,
            },
        );
    }
    if declared.keys().any(|quantity| !expected.contains_key(quantity)) {
        return Err("benchmark oracle declares an unexpected output quantity".to_string());
    }
    Ok((expected, tolerance))
}

fn compare_outputs(
    outputs: Option<&Value>,
    expected: &BTreeMap<String, ExpectedOutput>,
    tolerance: f64,
    issues: &mut Vec<String>,
) -> (bool, Vec<QuantityComparison>) {
    let Some(outputs) = outputs.and_then(Value::as_array) else {
        issues.push("raw output outputs field must be an array".to_string());
        return (false, Vec::new());
    };

    let mut observed = BTreeMap::<String, ObservedOutput>::new();
    let mut valid = true;
    for output in outputs {
        let Some(quantity) = output.get("quantity").and_then(Value::as_str) else {
            valid = false;
            issues.push("raw output item is missing quantity".to_string());
            continue;
        };
        let Some(value) = output
            .get("value")
            .and_then(Value::as_f64)
            .filter(|value| value.is_finite())
        else {
            valid = false;
            issues.push(format!("raw output {quantity:?} is not finite numeric"));
            continue;
        };
        let Some(unit) = output.get("unit").and_then(Value::as_str) else {
            valid = false;
            issues.push(format!("raw output {quantity:?} is missing unit"));
            continue;
        };
        if !expected.contains_key(quantity) {
            valid = false;
            issues.push(format!("raw output contains unexpected quantity {quantity:?}"));
            continue;
        }
        if observed
            .insert(
                quantity.to_string(),
                ObservedOutput {
                    value,
                    unit: unit.to_string(),
                },
            )
            .is_some()
        {
            valid = false;
            issues.push(format!("raw output duplicates quantity {quantity:?}"));
        }
    }

    let mut comparisons = Vec::with_capacity(expected.len());
    for (quantity, expected_output) in expected {
        let allowed_error = tolerance * expected_output.value.abs().max(f64::MIN_POSITIVE);
        let Some(actual) = observed.get(quantity) else {
            valid = false;
            issues.push(format!("raw output omits required quantity {quantity:?}"));
            comparisons.push(QuantityComparison {
                quantity: quantity.clone(),
                actual_value: None,
                actual_unit: None,
                expected_value: expected_output.value,
                expected_unit: expected_output.unit.to_string(),
                absolute_error: None,
                allowed_error,
                within_tolerance: false,
            });
            continue;
        };

        let unit_matches = actual.unit == expected_output.unit;
        if !unit_matches {
            valid = false;
            issues.push(format!(
                "raw output quantity {quantity:?} has unit {:?}, expected {:?}",
                actual.unit, expected_output.unit
            ));
        }
        let absolute_error = (actual.value - expected_output.value).abs();
        let numerical_match = absolute_error <= allowed_error;
        if !numerical_match {
            valid = false;
            issues.push(format!(
                "reference mismatch for {quantity:?}: actual {}, expected {}, absolute error {}, allowed {}",
                actual.value, expected_output.value, absolute_error, allowed_error
            ));
        }
        comparisons.push(QuantityComparison {
            quantity: quantity.clone(),
            actual_value: Some(actual.value),
            actual_unit: Some(actual.unit.clone()),
            expected_value: expected_output.value,
            expected_unit: expected_output.unit.to_string(),
            absolute_error: Some(absolute_error),
            allowed_error,
            within_tolerance: unit_matches && numerical_match,
        });
    }
    (valid && observed.len() == expected.len(), comparisons)
}

fn unit_for_quantity(quantity: &str) -> Option<&'static str> {
    match quantity {
        "second_moment_area_m4" => Some("m^4"),
        "free_end_deflection_m" | "free_end_extension_m" | "maximum_deflection_m" => Some("m"),
        "support_maximum_bending_stress_Pa"
        | "axial_stress_Pa"
        | "maximum_bending_stress_Pa" => Some("Pa"),
        "cross_section_area_m2" => Some("m^2"),
        "support_reaction_magnitude_N" => Some("N"),
        "maximum_bending_moment_Nm" => Some("N*m"),
        _ => None,
    }
}

fn safe_artifact_path(root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("path must contain only normal relative components".to_string());
    }
    let base = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve artifact root: {error}"))?;
    let resolved = base
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("cannot resolve artifact: {error}"))?;
    if !resolved.starts_with(&base) {
        return Err("artifact resolves outside declared root".to_string());
    }
    Ok(resolved)
}

fn validate_against_schema(
    schema_text: &str,
    value: &Value,
    label: &str,
) -> Result<Vec<String>, String> {
    let schema: Value = serde_json::from_str(schema_text)
        .map_err(|error| format!("embedded {label} schema is invalid JSON: {error}"))?;
    if !all_refs_are_local(&schema) {
        return Err(format!("{label} schema contains an external reference"));
    }
    draft202012::meta::validate(&schema)
        .map_err(|error| format!("{label} schema fails Draft 2020-12 meta-validation: {error}"))?;
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("cannot compile {label} schema: {error}"))?;
    Ok(validator
        .iter_errors(value)
        .map(|error| format!("{label} schema validation: {error}"))
        .collect())
}

fn all_refs_are_local(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().all(|(key, child)| {
            (key != "$ref"
                || child
                    .as_str()
                    .is_some_and(|reference| reference.starts_with("#/")))
                && all_refs_are_local(child)
        }),
        Value::Array(values) => values.iter().all(all_refs_are_local),
        _ => true,
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    const AXIAL_FIXTURE: &str = include_str!(
        "../../../../docs/built-environment/benchmarks/axial-bar-linear-static-v1.json"
    );
    const CANTILEVER_FIXTURE: &str = include_str!(
        "../../../../docs/built-environment/benchmarks/cantilever-beam-linear-static-v1.json"
    );
    const SIMPLE_FIXTURE: &str = include_str!(
        "../../../../docs/built-environment/benchmarks/simply-supported-beam-linear-static-v1.json"
    );
    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "symtropy-benchmark-result-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("create temp directory");
        path
    }

    fn checker_pin() -> String {
        let executable = std::env::current_exe().expect("current test executable path");
        let bytes = fs::read(executable).expect("read current test executable");
        sha256_hex(&bytes)
    }

    fn outputs_from_fixture(fixture: &str, case_id: &str) -> Value {
        let benchmark: Value = serde_json::from_str(fixture).expect("valid benchmark fixture");
        let benchmark_id = benchmark["benchmark_id"]
            .as_str()
            .expect("benchmark_id in fixture");
        let inputs = &benchmark["inputs"];
        let number = |object: &Value, key: &str| -> f64 {
            object[key].as_f64().unwrap_or_else(|| panic!("{key} is numeric"))
        };

        // Recompute from primitive inputs instead of copying expected values.
        // These controls can now detect an incorrect or stale numerical oracle,
        // not just exercise the report and hashing plumbing.
        let calculated: Vec<(String, f64)> = match benchmark_id {
            "structural/cantilever-beam-linear-static/v1" => {
                assert_eq!(case_id, "default");
                let load = number(inputs, "load_N");
                let span = number(inputs, "span_m");
                let modulus = number(inputs, "elastic_modulus_Pa");
                let width = number(inputs, "section_width_m");
                let depth = number(inputs, "section_depth_m");
                let inertia = width * depth.powi(3) / 12.0;
                let fiber = depth / 2.0;
                vec![
                    ("second_moment_area_m4".to_string(), inertia),
                    (
                        "free_end_deflection_m".to_string(),
                        load * span.powi(3) / (3.0 * modulus * inertia),
                    ),
                    (
                        "support_maximum_bending_stress_Pa".to_string(),
                        load * span * fiber / inertia,
                    ),
                ]
            }
            "structural/axial-bar-linear-static/v1" => {
                assert_eq!(case_id, "default");
                let load = number(inputs, "load_N");
                let length = number(inputs, "length_m");
                let modulus = number(inputs, "elastic_modulus_Pa");
                let area =
                    number(inputs, "section_width_m") * number(inputs, "section_height_m");
                vec![
                    ("cross_section_area_m2".to_string(), area),
                    ("axial_stress_Pa".to_string(), load / area),
                    (
                        "free_end_extension_m".to_string(),
                        load * length / (modulus * area),
                    ),
                    ("support_reaction_magnitude_N".to_string(), load),
                ]
            }
            "structural/simply-supported-beam-linear-static/v1" => {
                let cases = benchmark["load_cases"].as_array().expect("load_cases array");
                let mut matching = cases
                    .iter()
                    .filter(|item| item["case_id"].as_str() == Some(case_id));
                let load_case = matching.next().expect("known load case in fixture");
                assert!(matching.next().is_none(), "load_case ID must be unique");
                let span = number(inputs, "span_m");
                let modulus = number(inputs, "elastic_modulus_Pa");
                let inertia = number(inputs, "section_width_m")
                    * number(inputs, "section_depth_m").powi(3)
                    / 12.0;
                let fiber = number(inputs, "section_depth_m") / 2.0;
                let (deflection, moment, reaction) =
                    match load_case["load_type"].as_str().expect("load type") {
                        "point_load_midspan" => {
                            let load = number(load_case, "load_N");
                            (
                                load * span.powi(3) / (48.0 * modulus * inertia),
                                load * span / 4.0,
                                load / 2.0,
                            )
                        }
                        "uniform_distributed_load" => {
                            let load_per_m = number(load_case, "load_N_per_m");
                            (
                                5.0 * load_per_m * span.powi(4)
                                    / (384.0 * modulus * inertia),
                                load_per_m * span.powi(2) / 8.0,
                                load_per_m * span / 2.0,
                            )
                        }
                        other => panic!("unsupported load type {other}"),
                    };
                vec![
                    ("maximum_deflection_m".to_string(), deflection),
                    ("maximum_bending_moment_Nm".to_string(), moment),
                    (
                        "maximum_bending_stress_Pa".to_string(),
                        moment * fiber / inertia,
                    ),
                    ("support_reaction_magnitude_N".to_string(), reaction),
                ]
            }
            other => panic!("unsupported test fixture {other}"),
        };

        let outputs: Vec<Value> = calculated
            .into_iter()
            .map(|(quantity, value)| {
                let unit = unit_for_quantity(&quantity).expect("known output unit");
                json!({ "quantity": quantity, "value": value, "unit": unit })
            })
            .collect();

        json!({
            "schema_version": "0.1.0",
            "benchmark_id": benchmark_id,
            "case_id": case_id,
            "outputs": outputs
        })
    }

    fn setup_case(
        outputs: Value,
        declared_status: &str,
        override_output_digest: Option<&str>,
    ) -> (PathBuf, PathBuf, String) {
        setup_case_for_fixture(
            AXIAL_FIXTURE,
            "default",
            outputs,
            declared_status,
            override_output_digest,
        )
    }

    fn setup_case_for_fixture(
        fixture: &str,
        case_id: &str,
        mut outputs: Value,
        declared_status: &str,
        override_output_digest: Option<&str>,
    ) -> (PathBuf, PathBuf, String) {
        let root = temp_dir();
        let benchmark_path = root.join("benchmark.json");
        fs::write(&benchmark_path, fixture.as_bytes()).expect("write benchmark fixture");
        let benchmark_sha = sha256_hex(fixture.as_bytes());
        let benchmark: Value = serde_json::from_str(fixture).expect("parse benchmark fixture");
        let benchmark_id = benchmark["benchmark_id"].as_str().expect("benchmark_id");
        outputs["benchmark_id"] = json!(benchmark_id);
        outputs["case_id"] = json!(case_id);

        let outputs_bytes = serde_json::to_vec_pretty(&outputs).expect("serialize raw outputs");
        let outputs_path = root.join("raw-outputs.json");
        fs::write(&outputs_path, &outputs_bytes).expect("write raw outputs");
        let raw_digest = override_output_digest
            .map(str::to_owned)
            .unwrap_or_else(|| sha256_hex(&outputs_bytes));

        let record = json!({
            "schema_version": "0.1.0",
            "benchmark_id": benchmark_id,
            "case_id": case_id,
            "benchmark_sha256": benchmark_sha,
            "declared_status": declared_status,
            "evaluator": {
                "id": "synthetic-test-evaluator",
                "version": "0.1.0",
                "executable_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            },
            "execution": {
                "exit_code": 0,
                "termination_status": "COMPLETED",
                "execution_kind": "ANALYTICAL_REFERENCE",
                "convergence_status": "NOT_APPLICABLE"
            },
            "raw_output_artifact": {
                "relative_path": "raw-outputs.json",
                "sha256": raw_digest
            }
        });
        let record_path = root.join("result-record.json");
        fs::write(
            &record_path,
            serde_json::to_vec_pretty(&record).expect("serialize result record"),
        )
        .expect("write result record");
        (root, record_path, benchmark_sha)
    }

    #[test]
    fn checker_matches_cantilever_and_both_simple_span_reference_cases() {
        for (fixture, case_id) in [
            (CANTILEVER_FIXTURE, "default"),
            (SIMPLE_FIXTURE, "midpoint-point-load-v1"),
            (SIMPLE_FIXTURE, "uniform-distributed-load-v1"),
        ] {
            let outputs = outputs_from_fixture(fixture, case_id);
            let (root, record_path, pin) =
                setup_case_for_fixture(fixture, case_id, outputs, "CLAIMED_PASS", None);
            let report = validate_paths(
                &root.join("benchmark.json"),
                &record_path,
                &pin,
                &checker_pin(),
                &root,
            )
            .expect("validation should produce a report");
            assert_eq!(
                report.independent_result_status,
                "REFERENCE_MATCH",
                "reference check failed for case {case_id}"
            );
            assert_eq!(report.declared_status_consistent, Some(true));
            fs::remove_dir_all(root).expect("remove temp directory");
        }
    }

    #[test]
    fn fixture_oracle_drift_is_detected_against_recomputed_outputs() {
        let mut benchmark: Value =
            serde_json::from_str(AXIAL_FIXTURE).expect("valid analytical fixture");
        benchmark["expected"]["axial_stress_Pa"] = json!(2500000.0);
        let altered_fixture =
            serde_json::to_string_pretty(&benchmark).expect("serialize altered fixture");
        let outputs = outputs_from_fixture(&altered_fixture, "default");
        let (root, record_path, pin) = setup_case_for_fixture(
            &altered_fixture,
            "default",
            outputs,
            "CLAIMED_PASS",
            None,
        );
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert_eq!(report.declared_status_consistent, None);
        assert!(report.issues.iter().any(|issue| issue.contains("benchmark oracle drift")));
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn overflowed_derived_geometry_is_blocked() {
        let mut benchmark: Value =
            serde_json::from_str(AXIAL_FIXTURE).expect("valid analytical fixture");
        benchmark["inputs"]["section_width_m"] = json!(1.0e308);
        benchmark["inputs"]["section_height_m"] = json!(1.0e100);
        let altered_fixture =
            serde_json::to_string_pretty(&benchmark).expect("serialize altered fixture");
        let outputs = outputs_from_fixture(AXIAL_FIXTURE, "default");
        let (root, record_path, pin) = setup_case_for_fixture(
            &altered_fixture,
            "default",
            outputs,
            "CLAIMED_PASS",
            None,
        );
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert!(report.issues.iter().any(|issue| {
            issue.contains("equation-derived benchmark input")
                && issue.contains("cross_section_area_m2")
        }));
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn overly_permissive_reference_tolerance_is_blocked() {
        let mut benchmark: Value =
            serde_json::from_str(AXIAL_FIXTURE).expect("valid analytical fixture");
        benchmark["acceptance"]["relative_tolerance"] = json!(0.01);
        let altered_fixture =
            serde_json::to_string_pretty(&benchmark).expect("serialize altered fixture");
        let outputs = outputs_from_fixture(&altered_fixture, "default");
        let (root, record_path, pin) = setup_case_for_fixture(
            &altered_fixture,
            "default",
            outputs,
            "CLAIMED_PASS",
            None,
        );
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert!(report.issues.iter().any(|issue| {
            issue.contains("relative_tolerance")
                && issue.contains("no greater than 1e-9")
        }));
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn contradictory_derived_input_is_blocked_even_if_outputs_follow_geometry() {
        let mut benchmark: Value =
            serde_json::from_str(AXIAL_FIXTURE).expect("valid analytical fixture");
        benchmark["inputs"]["cross_section_area_m2"] = json!(0.006);
        let altered_fixture =
            serde_json::to_string_pretty(&benchmark).expect("serialize altered fixture");
        let outputs = outputs_from_fixture(&altered_fixture, "default");
        let (root, record_path, pin) = setup_case_for_fixture(
            &altered_fixture,
            "default",
            outputs,
            "CLAIMED_PASS",
            None,
        );
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert!(report.issues.iter().any(|issue| {
            issue.contains("derived benchmark input")
                && issue.contains("cross_section_area_m2")
        }));
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn independent_checker_matches_valid_reference_outputs() {
        let (root, record_path, pin) = setup_case(outputs_from_fixture(AXIAL_FIXTURE, "default"), "CLAIMED_PASS", None);
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "REFERENCE_MATCH");
        assert_eq!(report.declared_status_consistent, Some(true));
        assert!(report.outputs_match_reference);
        assert_eq!(
            report.benchmark_id.as_deref(),
            Some("structural/axial-bar-linear-static/v1")
        );
        assert_eq!(report.case_id.as_deref(), Some("default"));
        assert_eq!(report.quantity_comparisons.len(), 4);
        assert!(report.quantity_comparisons.iter().all(|item| item.within_tolerance));
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn false_claimed_pass_is_rejected_even_when_output_digest_is_valid() {
        let mut outputs = outputs_from_fixture(AXIAL_FIXTURE, "default");
        let stress = outputs["outputs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item.get("quantity").and_then(Value::as_str) == Some("axial_stress_Pa"))
            .expect("axial stress output");
        stress["value"] = json!(3000000.0);
        let (root, record_path, pin) = setup_case(outputs, "CLAIMED_PASS", None);
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "REFERENCE_MISMATCH");
        assert_eq!(report.declared_status_consistent, Some(false));
        assert!(!report.outputs_match_reference);
        assert!(report.raw_output_digest_matches);
        let stress = report
            .quantity_comparisons
            .iter()
            .find(|item| item.quantity == "axial_stress_Pa")
            .expect("stress comparison must be auditable");
        assert_eq!(stress.actual_value, Some(3_000_000.0));
        assert_eq!(stress.expected_value, 2_400_000.0);
        assert_eq!(stress.absolute_error, Some(600_000.0));
        assert!(!stress.within_tolerance);
        assert!(stress.allowed_error < 1e-4);
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn benchmark_pin_mismatch_blocks_result_even_if_outputs_match() {
        let (root, record_path, _) = setup_case(correct_outputs(), "CLAIMED_PASS", None);
        let wrong_pin = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            wrong_pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert!(!report.benchmark_pin_matches);
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn numerical_solver_without_convergence_is_blocked_even_when_values_match() {
        let (root, record_path, pin) = setup_case(correct_outputs(), "CLAIMED_PASS", None);
        let record_bytes = fs::read(&record_path).expect("read record");
        let mut record: Value =
            serde_json::from_slice(&record_bytes).expect("parse result record");
        record["execution"]["execution_kind"] = json!("NUMERICAL_SOLVER");
        record["execution"]["convergence_status"] = json!("NOT_CONVERGED");
        fs::write(
            &record_path,
            serde_json::to_vec_pretty(&record).expect("serialize changed record"),
        )
        .expect("write changed record");

        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert!(!report.execution_completed);
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn converged_numerical_solver_is_blocked_without_solver_specific_profile() {
        let outputs = outputs_from_fixture(AXIAL_FIXTURE, "default");
        let (root, record_path, pin) = setup_case(outputs, "CLAIMED_PASS", None);
        let record_bytes = fs::read(&record_path).expect("read result record");
        let mut record: Value =
            serde_json::from_slice(&record_bytes).expect("parse result record");
        record["execution"]["execution_kind"] = json!("NUMERICAL_SOLVER");
        record["execution"]["convergence_status"] = json!("CONVERGED");
        fs::write(
            &record_path,
            serde_json::to_vec_pretty(&record).expect("serialize result record"),
        )
        .expect("write changed result record");

        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert!(!report.execution_completed);
        assert!(report.issues.iter().any(|issue| issue.contains("solver-specific acceptance profiles")));
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn checker_pin_mismatch_blocks_even_correct_reference_outputs() {
        let (root, record_path, pin) = setup_case(correct_outputs(), "CLAIMED_PASS", None);
        let wrong_checker_pin =
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            wrong_checker_pin,
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert!(!report.checker_pin_matches);
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn output_path_traversal_is_blocked() {
        let outputs = outputs_from_fixture(AXIAL_FIXTURE, "default");
        let (root, record_path, pin) = setup_case(outputs, "CLAIMED_PASS", None);
        let record_bytes = fs::read(&record_path).expect("read result record");
        let mut record: Value =
            serde_json::from_slice(&record_bytes).expect("parse result record");
        record["raw_output_artifact"]["relative_path"] = json!("../outside.json");
        fs::write(
            &record_path,
            serde_json::to_vec_pretty(&record).expect("serialize result record"),
        )
        .expect("write changed result record");

        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert!(report.issues.iter().any(|issue| issue.contains("path is unsafe")));
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn raw_output_byte_mutation_is_detected() {
        let (root, record_path, pin) = setup_case(correct_outputs(), "CLAIMED_PASS", None);
        fs::write(root.join("raw-outputs.json"), b"{\"tampered\":true}")
            .expect("mutate raw output");
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "BLOCKED");
        assert!(!report.raw_output_digest_matches);
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn output_with_incompatible_unit_cannot_pass_reference() {
        let mut outputs = outputs_from_fixture(AXIAL_FIXTURE, "default");
        let extension = outputs["outputs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|item| item.get("quantity").and_then(Value::as_str) == Some("free_end_extension_m"))
            .expect("free-end extension output");
        extension["unit"] = json!("mm");

        let (root, record_path, pin) = setup_case(outputs, "CLAIMED_PASS", None);
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "REFERENCE_MISMATCH");
        assert!(!report.outputs_match_reference);
        assert!(report.issues.iter().any(|issue| issue.contains("has unit")));
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn missing_required_output_cannot_pass_reference() {
        let mut outputs = outputs_from_fixture(AXIAL_FIXTURE, "default");
        outputs["outputs"].as_array_mut().unwrap().retain(|item| {
            item.get("quantity").and_then(Value::as_str) != Some("support_reaction_magnitude_N")
        });

        let (root, record_path, pin) = setup_case(outputs, "CLAIMED_PASS", None);
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "REFERENCE_MISMATCH");
        assert!(!report.outputs_match_reference);
        assert!(report.issues.iter().any(|issue| issue.contains("omits required quantity")));
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn duplicate_output_quantity_cannot_satisfy_reference() {
        let mut outputs = outputs_from_fixture(AXIAL_FIXTURE, "default");
        let duplicate = outputs["outputs"][0].clone();
        outputs["outputs"].as_array_mut().unwrap().push(duplicate);
        let (root, record_path, pin) = setup_case(outputs, "CLAIMED_PASS", None);
        let report = validate_paths(
            &root.join("benchmark.json"),
            &record_path,
            &pin,
            &checker_pin(),
            &root,
        )
        .expect("validation should produce a report");
        assert_eq!(report.independent_result_status, "REFERENCE_MISMATCH");
        assert!(!report.outputs_match_reference);
        fs::remove_dir_all(root).expect("remove temp directory");
    }
}
