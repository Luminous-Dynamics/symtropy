// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::env;
use std::error::Error;
use std::fs;
use std::process;

const MAX_COILS: usize = 32;

#[derive(Debug)]
struct MatrixRecord {
    matrix_id: String,
    evidence_kind: String,
    matrix_kind: String,
    frequency_hz: f64,
    temperature_c: f64,
    dc_bias_currents_a: Vec<f64>,
    coil_ids: Vec<String>,
    polarity_reference: String,
    matrix_h: Vec<Vec<f64>>,
    bounds_h: Vec<Vec<f64>>,
    absolute_tolerance_h: f64,
    relative_tolerance: f64,
    expected_disposition: Option<String>,
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))?
        .as_str()
}

fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    path.iter()
        .try_fold(value, |current, key| current.get(*key))?
        .as_f64()
}

fn exact_keys(value: &Value, expected: &[&str], path: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{path}: expected object"))?;
    for key in expected {
        if !object.contains_key(*key) {
            return Err(format!("{path}.{key}: required field missing"));
        }
    }
    for key in object.keys() {
        if !expected.contains(&key.as_str()) {
            return Err(format!("{path}.{key}: unexpected field"));
        }
    }
    Ok(())
}

fn exact_keys_optional(
    value: &Value,
    required: &[&str],
    optional: &[&str],
    path: &str,
) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{path}: expected object"))?;
    for key in required {
        if !object.contains_key(*key) {
            return Err(format!("{path}.{key}: required field missing"));
        }
    }
    for key in object.keys() {
        if !required.contains(&key.as_str()) && !optional.contains(&key.as_str()) {
            return Err(format!("{path}.{key}: unexpected field"));
        }
    }
    Ok(())
}

fn is_absolute_http_uri(value: &str) -> bool {
    let Some((scheme, remainder)) = value.split_once("://") else {
        return false;
    };
    matches!(scheme, "http" | "https")
        && !remainder.is_empty()
        && !value.chars().any(char::is_whitespace)
}

fn number_array(value: &Value, path: &str) -> Result<Vec<f64>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{path}: expected array"))?;
    values
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let number = item
                .as_f64()
                .ok_or_else(|| format!("{path}[{index}]: expected finite number"))?;
            if !number.is_finite() {
                return Err(format!("{path}[{index}]: non-finite number"));
            }
            Ok(number)
        })
        .collect()
}

fn matrix(value: &Value, n: usize, path: &str, nonnegative: bool) -> Result<Vec<Vec<f64>>, String> {
    let rows = value
        .as_array()
        .ok_or_else(|| format!("{path}: expected matrix array"))?;
    if rows.len() != n {
        return Err(format!("{path}: expected {n} rows, got {}", rows.len()));
    }
    let mut result = Vec::with_capacity(n);
    for (row_index, row) in rows.iter().enumerate() {
        let values = number_array(row, &format!("{path}[{row_index}]"))?;
        if values.len() != n {
            return Err(format!(
                "{path}[{row_index}]: expected {n} columns, got {}",
                values.len()
            ));
        }
        if nonnegative && values.iter().any(|entry| *entry < 0.0) {
            return Err(format!("{path}[{row_index}]: bounds must be nonnegative"));
        }
        result.push(values);
    }
    Ok(result)
}

fn sha256_or_null(value: &Value, key: &str) -> Result<(), String> {
    match value.get(key) {
        Some(Value::Null) => Ok(()),
        Some(Value::String(digest))
            if digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()) =>
        {
            Ok(())
        }
        _ => Err(format!("{key}: expected null or 64 hexadecimal SHA-256 characters")),
    }
}

fn parse_record(value: &Value) -> Result<MatrixRecord, String> {
    exact_keys_optional(
        value,
        &[
            "schema_version",
            "matrix_id",
            "evidence_kind",
            "matrix_kind",
            "operating_point",
            "coil_ids",
            "polarity_reference",
            "matrix_h",
            "entrywise_absolute_bound_h",
            "uncertainty_semantics",
            "numerical_tolerances",
            "configuration_reference",
            "source_reference",
            "artifact_sha256",
        ],
        &["expected_disposition"],
        "$",
    )?;
    if string_at(value, &["schema_version"]) != Some("wpt-inductance-matrix-v1") {
        return Err("schema_version: expected wpt-inductance-matrix-v1".into());
    }
    let matrix_id = string_at(value, &["matrix_id"])
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| "matrix_id: expected non-empty string".to_string())?
        .to_string();
    let evidence_kind = string_at(value, &["evidence_kind"])
        .ok_or_else(|| "evidence_kind: required string missing".to_string())?
        .to_string();
    if !matches!(
        evidence_kind.as_str(),
        "SYNTHETIC_KNOWN_TRUTH" | "MEASURED" | "MODEL_DERIVED"
    ) {
        return Err(format!("evidence_kind: unsupported value {evidence_kind}"));
    }
    let matrix_kind = string_at(value, &["matrix_kind"])
        .ok_or_else(|| "matrix_kind: required string missing".to_string())?
        .to_string();
    if !matches!(
        matrix_kind.as_str(),
        "SMALL_SIGNAL" | "DIFFERENTIAL"
    ) {
        return Err(format!("matrix_kind: unsupported value {matrix_kind}"));
    }

    exact_keys(
        value.get("operating_point").ok_or("operating_point: missing")?,
        &["frequency_hz", "temperature_c", "dc_bias_currents_a"],
        "operating_point",
    )?;
    let frequency_hz = number_at(value, &["operating_point", "frequency_hz"])
        .filter(|number| number.is_finite() && *number > 0.0)
        .ok_or_else(|| "operating_point.frequency_hz: expected finite positive number".to_string())?;
    let temperature_c = number_at(value, &["operating_point", "temperature_c"])
        .filter(|number| number.is_finite())
        .ok_or_else(|| "operating_point.temperature_c: expected finite number".to_string())?;

    let coil_values = value
        .get("coil_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "coil_ids: expected array".to_string())?;
    if !(2..=MAX_COILS).contains(&coil_values.len()) {
        return Err(format!("coil_ids: count must be 2..={MAX_COILS}"));
    }
    let mut coil_ids = Vec::with_capacity(coil_values.len());
    for (index, item) in coil_values.iter().enumerate() {
        let id = item
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| format!("coil_ids[{index}]: expected non-empty string"))?;
        if coil_ids.iter().any(|existing| existing == id) {
            return Err(format!("coil_ids[{index}]: duplicate coil identifier {id}"));
        }
        coil_ids.push(id.to_string());
    }
    let n = coil_ids.len();

    let polarity_reference = string_at(value, &["polarity_reference"])
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| "polarity_reference: expected non-empty convention".to_string())?
        .to_string();
    let matrix_h = matrix(
        value.get("matrix_h").ok_or("matrix_h: missing")?,
        n,
        "matrix_h",
        false,
    )?;
    let bounds_h = matrix(
        value
            .get("entrywise_absolute_bound_h")
            .ok_or("entrywise_absolute_bound_h: missing")?,
        n,
        "entrywise_absolute_bound_h",
        true,
    )?;
    let dc_bias_currents_a = number_array(
        value
            .get("operating_point")
            .and_then(|point| point.get("dc_bias_currents_a"))
            .ok_or("operating_point.dc_bias_currents_a: missing")?,
        "operating_point.dc_bias_currents_a",
    )?;
    if dc_bias_currents_a.len() != n {
        return Err(format!(
            "operating_point.dc_bias_currents_a: expected {n} values"
        ));
    }
    if matrix_h
        .iter()
        .enumerate()
        .any(|(index, row)| row[index] <= 0.0)
    {
        return Err("matrix_h: every self-inductance diagonal entry must be positive".into());
    }

    if string_at(value, &["uncertainty_semantics"])
        != Some("DETERMINISTIC_ABSOLUTE_BOUND")
    {
        return Err(
            "uncertainty_semantics: only DETERMINISTIC_ABSOLUTE_BOUND is supported".into(),
        );
    }
    exact_keys(
        value
            .get("numerical_tolerances")
            .ok_or("numerical_tolerances: missing")?,
        &["absolute_h", "relative"],
        "numerical_tolerances",
    )?;
    let absolute_tolerance_h = number_at(value, &["numerical_tolerances", "absolute_h"])
        .filter(|number| number.is_finite() && *number >= 0.0)
        .ok_or_else(|| "numerical_tolerances.absolute_h: expected finite nonnegative number".to_string())?;
    let relative_tolerance = number_at(value, &["numerical_tolerances", "relative"])
        .filter(|number| number.is_finite() && *number >= 0.0)
        .ok_or_else(|| "numerical_tolerances.relative: expected finite nonnegative number".to_string())?;

    if string_at(value, &["configuration_reference"])
        .is_none_or(|text| text.trim().is_empty())
    {
        return Err("configuration_reference: expected non-empty reference".into());
    }
    let source_reference = value.get("source_reference");
    if !matches!(source_reference, Some(Value::String(_)) | Some(Value::Null)) {
        return Err("source_reference: expected URI string or null".into());
    }
    if let Some(uri) = source_reference.and_then(Value::as_str) {
        if !is_absolute_http_uri(uri) {
            return Err("source_reference: expected absolute HTTP(S) URI".into());
        }
    }
    if evidence_kind != "SYNTHETIC_KNOWN_TRUTH" {
        if !source_reference.and_then(Value::as_str).is_some_and(|text| !text.trim().is_empty()) {
            return Err("source_reference: measured/model-derived evidence requires source URI".into());
        }
        if value.get("artifact_sha256").and_then(Value::as_str).is_none() {
            return Err("artifact_sha256: measured/model-derived evidence requires artifact digest".into());
        }
    }
    sha256_or_null(value, "artifact_sha256")?;

    let expected_disposition = value
        .get("expected_disposition")
        .and_then(Value::as_str)
        .map(str::to_string);
    if expected_disposition.as_deref().is_some_and(|expected| {
        !matches!(
            expected,
            "PASSIVE_CERTIFIED"
                | "NONPASSIVE_REJECTED"
                | "NONRECIPROCAL_REJECTED"
                | "UNRESOLVED"
                | "INVALID_INPUT"
        )
    }) {
        return Err("expected_disposition: unsupported value".into());
    }

    Ok(MatrixRecord {
        matrix_id,
        evidence_kind,
        matrix_kind,
        frequency_hz,
        temperature_c,
        dc_bias_currents_a,
        coil_ids,
        polarity_reference,
        matrix_h,
        bounds_h,
        absolute_tolerance_h,
        relative_tolerance,
        expected_disposition,
    })
}

// Deterministic cyclic Jacobi diagonalization. Scaling the input first prevents
// small henry-valued matrices from being compared against an absolute epsilon.
fn smallest_symmetric_eigenvalue(matrix: &[Vec<f64>]) -> Result<f64, String> {
    let n = matrix.len();
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if !scale.is_finite() {
        return Err("eigensolver: non-finite matrix scale".into());
    }
    if scale == 0.0 {
        return Ok(0.0);
    }

    let mut values: Vec<Vec<f64>> = matrix
        .iter()
        .map(|row| row.iter().map(|value| value / scale).collect())
        .collect();
    let threshold = 16.0 * f64::EPSILON;
    let mut converged = false;

    for _sweep in 0..100 {
        let mut largest_off_diagonal = 0.0_f64;
        for p in 0..n {
            for q in (p + 1)..n {
                largest_off_diagonal = largest_off_diagonal.max(values[p][q].abs());
            }
        }
        if largest_off_diagonal <= threshold {
            converged = true;
            break;
        }

        for p in 0..n {
            for q in (p + 1)..n {
                let apq = values[p][q];
                if apq.abs() <= threshold {
                    continue;
                }
                let app = values[p][p];
                let aqq = values[q][q];
                let tau = (aqq - app) / (2.0 * apq);
                let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (tau.abs() + tau.hypot(1.0));
                let cosine = 1.0 / (1.0 + t * t).sqrt();
                let sine = t * cosine;

                values[p][p] = app - t * apq;
                values[q][q] = aqq + t * apq;
                values[p][q] = 0.0;
                values[q][p] = 0.0;

                for k in 0..n {
                    if k == p || k == q {
                        continue;
                    }
                    let akp = values[k][p];
                    let akq = values[k][q];
                    let new_kp = cosine * akp - sine * akq;
                    let new_kq = sine * akp + cosine * akq;
                    values[k][p] = new_kp;
                    values[p][k] = new_kp;
                    values[k][q] = new_kq;
                    values[q][k] = new_kq;
                }
            }
        }
    }

    if !converged {
        let mut largest_off_diagonal = 0.0_f64;
        for p in 0..n {
            for q in (p + 1)..n {
                largest_off_diagonal = largest_off_diagonal.max(values[p][q].abs());
            }
        }
        if largest_off_diagonal > 1.0e-12 {
            return Err(format!(
                "eigensolver: Jacobi method did not converge (off-diagonal residual {largest_off_diagonal:e})"
            ));
        }
    }

    let minimum = (0..n)
        .map(|index| values[index][index])
        .fold(f64::INFINITY, f64::min);
    let result = minimum * scale;
    if result.is_finite() {
        Ok(result)
    } else {
        Err("eigensolver: non-finite eigenvalue result".into())
    }
}

fn evaluate(record: &MatrixRecord) -> Result<Value, String> {
    let n = record.coil_ids.len();
    let max_diagonal = (0..n)
        .map(|index| record.matrix_h[index][index])
        .fold(0.0_f64, f64::max);
    let tolerance_h = record.absolute_tolerance_h + record.relative_tolerance * max_diagonal;
    if !tolerance_h.is_finite() {
        return Err("numerical tolerance overflow".into());
    }

    let mut reciprocal = vec![vec![0.0; n]; n];
    let mut symmetric_bounds = vec![vec![0.0; n]; n];
    let mut max_reciprocity_residual_h = 0.0_f64;
    let mut max_reciprocity_limit_h = 0.0_f64;
    let mut max_pairwise_abs_coupling = 0.0_f64;

    for i in 0..n {
        reciprocal[i][i] = record.matrix_h[i][i];
        symmetric_bounds[i][i] = record.bounds_h[i][i];
        for j in (i + 1)..n {
            let forward = record.matrix_h[i][j];
            let reverse = record.matrix_h[j][i];
            let mismatch = (forward - reverse).abs();
            let pair_limit = record.bounds_h[i][j] + record.bounds_h[j][i] + tolerance_h;
            max_reciprocity_residual_h = max_reciprocity_residual_h.max(mismatch);
            max_reciprocity_limit_h = max_reciprocity_limit_h.max(pair_limit);
            if mismatch > pair_limit {
                return Ok(json!({
                    "matrix_id": record.matrix_id,
                    "evidence_kind": record.evidence_kind,
                    "matrix_kind": record.matrix_kind,
                    "disposition": "NONRECIPROCAL_REJECTED",
                    "max_abs_reciprocity_residual_h": max_reciprocity_residual_h,
                    "max_reciprocity_limit_h": max_reciprocity_limit_h,
                    "coil_count": n,
                    "frequency_hz": record.frequency_hz,
                    "temperature_c": record.temperature_c,
                    "dc_bias_currents_a": record.dc_bias_currents_a,
                    "polarity_reference": record.polarity_reference,
                    "reasons": ["RECIPROCAL_INTERVALS_DO_NOT_OVERLAP"]
                }));
            }

            let midpoint = 0.5 * forward + 0.5 * reverse;
            let bound = 0.5
                * (record.bounds_h[i][j] + record.bounds_h[j][i] + mismatch)
                + tolerance_h;
            if !midpoint.is_finite() || !bound.is_finite() {
                return Err("reciprocity symmetrization overflow".into());
            }
            reciprocal[i][j] = midpoint;
            reciprocal[j][i] = midpoint;
            symmetric_bounds[i][j] = bound;
            symmetric_bounds[j][i] = bound;

            let denominator =
                (record.matrix_h[i][i] * record.matrix_h[j][j]).sqrt();
            let coupling = midpoint.abs() / denominator;
            if !coupling.is_finite() {
                return Err("pairwise coupling diagnostic overflow".into());
            }
            max_pairwise_abs_coupling = max_pairwise_abs_coupling.max(coupling);
        }
    }

    // ||Delta L||_2 <= ||Delta L||_F. hypot accumulation avoids overflow
    // from squaring very large but finite entrywise error bounds.
    let uncertainty_bound_h = symmetric_bounds
        .iter()
        .flatten()
        .fold(0.0_f64, |norm, value| norm.hypot(*value));
    if !uncertainty_bound_h.is_finite() {
        return Err("uncertainty Frobenius norm overflow".into());
    }

    let minimum_eigenvalue_h = smallest_symmetric_eigenvalue(&reciprocal)?;
    let lower_bound_h = minimum_eigenvalue_h - uncertainty_bound_h;
    let upper_bound_h = minimum_eigenvalue_h + uncertainty_bound_h;
    let disposition = if lower_bound_h >= -tolerance_h {
        "PASSIVE_CERTIFIED"
    } else if upper_bound_h < -tolerance_h {
        "NONPASSIVE_REJECTED"
    } else {
        "UNRESOLVED"
    };

    Ok(json!({
        "matrix_id": record.matrix_id,
        "evidence_kind": record.evidence_kind,
        "matrix_kind": record.matrix_kind,
        "disposition": disposition,
        "coil_count": n,
        "frequency_hz": record.frequency_hz,
        "temperature_c": record.temperature_c,
        "dc_bias_currents_a": record.dc_bias_currents_a,
        "max_abs_reciprocity_residual_h": max_reciprocity_residual_h,
        "max_reciprocity_limit_h": max_reciprocity_limit_h,
        "max_pairwise_abs_coupling": max_pairwise_abs_coupling,
        "midpoint_minimum_eigenvalue_h": minimum_eigenvalue_h,
        "eigenvalue_uncertainty_bound_h": uncertainty_bound_h,
        "minimum_eigenvalue_lower_bound_h": lower_bound_h,
        "minimum_eigenvalue_upper_bound_h": upper_bound_h,
        "numerical_tolerance_h": tolerance_h,
        "polarity_reference": record.polarity_reference,
        "reasons": []
    }))
}

fn evaluate_value(value: &Value) -> Value {
    match parse_record(value) {
        Ok(record) => match evaluate(&record) {
            Ok(mut result) => {
                if let Some(expected) = record.expected_disposition {
                    let actual = result
                        .get("disposition")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    result["expected_disposition"] = json!(expected);
                    result["expected_disposition_matches"] = json!(actual == expected);
                }
                result
            }
            Err(error) => json!({
                "matrix_id": string_at(value, &["matrix_id"]).unwrap_or("<unknown>"),
                "disposition": "INVALID_INPUT",
                "errors": [error]
            }),
        },
        Err(error) => json!({
            "matrix_id": string_at(value, &["matrix_id"]).unwrap_or("<unknown>"),
            "disposition": "INVALID_INPUT",
            "errors": [error]
        }),
    }
}

fn run(path: &str) -> Result<(), Box<dyn Error>> {
    let input = fs::read_to_string(path)?;
    let mut records = 0usize;
    let mut failures = 0usize;
    let mut matrix_ids = std::collections::BTreeSet::new();
    for (line_index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        records += 1;
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("{}:{}: JSON parse error: {error}", path, line_index + 1))?;
        let matrix_id = string_at(&value, &["matrix_id"]).unwrap_or("<unknown>");
        if !matrix_ids.insert(matrix_id.to_string()) {
            failures += 1;
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "matrix_id": matrix_id,
                    "disposition": "INVALID_INPUT",
                    "errors": ["duplicate matrix_id in JSONL corpus"]
                }))?
            );
            continue;
        }
        let result = evaluate_value(&value);
        if result
            .get("expected_disposition_matches")
            .and_then(Value::as_bool)
            == Some(false)
            || result.get("disposition").and_then(Value::as_str) == Some("INVALID_INPUT")
        {
            failures += 1;
        }
        println!("{}", serde_json::to_string(&result)?);
    }
    if records == 0 {
        return Err(format!("{path}: no JSONL records found").into());
    }
    if failures > 0 {
        return Err(format!("{failures} of {records} records failed validation or fixture expectation").into());
    }
    Ok(())
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: wpt-inductance-matrix-validator <matrix-records.jsonl>");
        process::exit(2);
    });
    if let Err(error) = run(&path) {
        eprintln!("wpt-inductance-matrix-validator: {error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(matrix: Value, bounds: Value, expected: &str) -> Value {
        json!({
            "schema_version": "wpt-inductance-matrix-v1",
            "matrix_id": "TEST-MATRIX",
            "evidence_kind": "SYNTHETIC_KNOWN_TRUTH",
            "matrix_kind": "SMALL_SIGNAL",
            "operating_point": {
                "frequency_hz": 85000.0,
                "temperature_c": 25.0,
                "dc_bias_currents_a": [0.0, 0.0, 0.0]
            },
            "coil_ids": ["TX", "RX1", "RX2"],
            "polarity_reference": "Fixed array order and positive current entering each marked dot terminal.",
            "matrix_h": matrix,
            "entrywise_absolute_bound_h": bounds,
            "uncertainty_semantics": "DETERMINISTIC_ABSOLUTE_BOUND",
            "numerical_tolerances": {"absolute_h": 1e-15, "relative": 1e-12},
            "configuration_reference": "synthetic:unit-test",
            "source_reference": null,
            "artifact_sha256": null,
            "expected_disposition": expected
        })
    }

    fn zero_bounds(n: usize) -> Value {
        json!((0..n).map(|_| vec![0.0; n]).collect::<Vec<_>>())
    }

    fn disposition(value: Value) -> String {
        evaluate_value(&value)["disposition"]
            .as_str()
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn accepts_physical_record_without_fixture_expected_disposition() {
        let value = json!({
            "schema_version": "wpt-inductance-matrix-v1",
            "matrix_id": "PHYSICAL-RECORD",
            "evidence_kind": "MEASURED",
            "matrix_kind": "DIFFERENTIAL",
            "operating_point": {"frequency_hz": 85000.0, "temperature_c": 40.0, "dc_bias_currents_a": [12.0, 8.0]},
            "coil_ids": ["TX", "RX"],
            "polarity_reference": "positive current enters each dot terminal",
            "matrix_h": [[10e-6, 2e-6], [2e-6, 8e-6]],
            "entrywise_absolute_bound_h": [[0.1e-6, 0.1e-6], [0.1e-6, 0.1e-6]],
            "uncertainty_semantics": "DETERMINISTIC_ABSOLUTE_BOUND",
            "numerical_tolerances": {"absolute_h": 1e-15, "relative": 1e-12},
            "configuration_reference": "test-rig:revision-3",
            "source_reference": "https://example.org/wpt-reference",
            "artifact_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        });
        assert_eq!(disposition(value), "PASSIVE_CERTIFIED");
    }

    #[test]
    fn rejects_secant_matrix_without_declared_linear_energy_model() {
        let mut value = fixture(
            json!([[1e-6, 0.0, 0.0], [0.0, 1e-6, 0.0], [0.0, 0.0, 1e-6]]),
            zero_bounds(3),
            "PASSIVE_CERTIFIED",
        );
        value["matrix_kind"] = json!("SECANT");
        assert_eq!(disposition(value), "INVALID_INPUT");
    }

    #[test]
    fn accepts_positive_definite_two_coil_matrix() {
        let value = json!({
            "schema_version": "wpt-inductance-matrix-v1",
            "matrix_id": "TWO-COIL",
            "evidence_kind": "SYNTHETIC_KNOWN_TRUTH",
            "matrix_kind": "SMALL_SIGNAL",
            "operating_point": {"frequency_hz": 85000.0, "temperature_c": 25.0, "dc_bias_currents_a": [0.0, 0.0]},
            "coil_ids": ["TX", "RX"],
            "polarity_reference": "positive current enters each dot",
            "matrix_h": [[10e-6, 2e-6], [2e-6, 8e-6]],
            "entrywise_absolute_bound_h": [[0.0, 0.0], [0.0, 0.0]],
            "uncertainty_semantics": "DETERMINISTIC_ABSOLUTE_BOUND",
            "numerical_tolerances": {"absolute_h": 1e-15, "relative": 1e-12},
            "configuration_reference": "synthetic:two-coil",
            "source_reference": null,
            "artifact_sha256": null
        });
        assert_eq!(disposition(value), "PASSIVE_CERTIFIED");
    }

    #[test]
    fn accepts_singular_positive_semidefinite_matrix() {
        let value = fixture(
            json!([[1e-6, 1e-6, 0.0], [1e-6, 1e-6, 0.0], [0.0, 0.0, 2e-6]]),
            zero_bounds(3),
            "PASSIVE_CERTIFIED",
        );
        assert_eq!(disposition(value), "PASSIVE_CERTIFIED");
    }

    #[test]
    fn rejects_globally_non_psd_matrix_even_when_every_pair_is_bounded() {
        let value = fixture(
            json!([
                [1e-6, 0.9e-6, 0.9e-6],
                [0.9e-6, 1e-6, -0.9e-6],
                [0.9e-6, -0.9e-6, 1e-6]
            ]),
            zero_bounds(3),
            "NONPASSIVE_REJECTED",
        );
        let result = evaluate_value(&value);
        assert_eq!(result["disposition"], "NONPASSIVE_REJECTED");
        assert!(result["max_pairwise_abs_coupling"].as_f64().unwrap() < 1.0);
        assert!(result["minimum_eigenvalue_upper_bound_h"].as_f64().unwrap() < 0.0);
    }

    #[test]
    fn rejects_reciprocity_violation_outside_bounds() {
        let value = json!({
            "schema_version": "wpt-inductance-matrix-v1",
            "matrix_id": "NONRECIPROCAL",
            "evidence_kind": "SYNTHETIC_KNOWN_TRUTH",
            "matrix_kind": "SMALL_SIGNAL",
            "operating_point": {"frequency_hz": 85000.0, "temperature_c": 25.0, "dc_bias_currents_a": [0.0, 0.0]},
            "coil_ids": ["TX", "RX"],
            "polarity_reference": "positive current enters each dot",
            "matrix_h": [[10e-6, 2e-6], [3e-6, 8e-6]],
            "entrywise_absolute_bound_h": [[0.0, 0.0], [0.0, 0.0]],
            "uncertainty_semantics": "DETERMINISTIC_ABSOLUTE_BOUND",
            "numerical_tolerances": {"absolute_h": 1e-12, "relative": 1e-12},
            "configuration_reference": "synthetic:nonreciprocal",
            "source_reference": null,
            "artifact_sha256": null
        });
        assert_eq!(disposition(value), "NONRECIPROCAL_REJECTED");
    }

    #[test]
    fn accepts_reciprocity_difference_contained_by_error_bounds() {
        let value = json!({
            "schema_version": "wpt-inductance-matrix-v1",
            "matrix_id": "RECIPROCAL-INTERVAL-OVERLAP",
            "evidence_kind": "SYNTHETIC_KNOWN_TRUTH",
            "matrix_kind": "SMALL_SIGNAL",
            "operating_point": {"frequency_hz": 85000.0, "temperature_c": 25.0, "dc_bias_currents_a": [0.0, 0.0]},
            "coil_ids": ["TX", "RX"],
            "polarity_reference": "positive current enters each dot",
            "matrix_h": [[10e-6, 2e-6], [2.1e-6, 8e-6]],
            "entrywise_absolute_bound_h": [[0.0, 0.0], [0.1e-6, 0.0]],
            "uncertainty_semantics": "DETERMINISTIC_ABSOLUTE_BOUND",
            "numerical_tolerances": {"absolute_h": 1e-15, "relative": 1e-12},
            "configuration_reference": "synthetic:reciprocity-overlap",
            "source_reference": null,
            "artifact_sha256": null
        });
        assert_eq!(disposition(value), "PASSIVE_CERTIFIED");
    }

    #[test]
    fn returns_unresolved_when_eigenvalue_interval_crosses_zero() {
        let value = json!({
            "schema_version": "wpt-inductance-matrix-v1",
            "matrix_id": "UNCERTAIN-PSD-BOUNDARY",
            "evidence_kind": "SYNTHETIC_KNOWN_TRUTH",
            "matrix_kind": "SMALL_SIGNAL",
            "operating_point": {"frequency_hz": 85000.0, "temperature_c": 25.0, "dc_bias_currents_a": [0.0, 0.0]},
            "coil_ids": ["TX", "RX"],
            "polarity_reference": "positive current enters each dot",
            "matrix_h": [[1e-6, 1e-6], [1e-6, 1e-6]],
            "entrywise_absolute_bound_h": [[0.05e-6, 0.05e-6], [0.05e-6, 0.05e-6]],
            "uncertainty_semantics": "DETERMINISTIC_ABSOLUTE_BOUND",
            "numerical_tolerances": {"absolute_h": 1e-15, "relative": 1e-12},
            "configuration_reference": "synthetic:uncertain-boundary",
            "source_reference": null,
            "artifact_sha256": null
        });
        assert_eq!(disposition(value), "UNRESOLVED");
    }

    #[test]
    fn rejects_nonpositive_self_inductance() {
        let value = json!({
            "schema_version": "wpt-inductance-matrix-v1",
            "matrix_id": "BAD-DIAGONAL",
            "evidence_kind": "SYNTHETIC_KNOWN_TRUTH",
            "matrix_kind": "SMALL_SIGNAL",
            "operating_point": {"frequency_hz": 85000.0, "temperature_c": 25.0, "dc_bias_currents_a": [0.0, 0.0]},
            "coil_ids": ["TX", "RX"],
            "polarity_reference": "positive current enters each dot",
            "matrix_h": [[1e-6, 0.0], [0.0, 0.0]],
            "entrywise_absolute_bound_h": [[0.0, 0.0], [0.0, 0.0]],
            "uncertainty_semantics": "DETERMINISTIC_ABSOLUTE_BOUND",
            "numerical_tolerances": {"absolute_h": 1e-15, "relative": 1e-12},
            "configuration_reference": "synthetic:bad-diagonal",
            "source_reference": null,
            "artifact_sha256": null
        });
        assert_eq!(disposition(value), "INVALID_INPUT");
    }

    #[test]
    fn rejects_duplicate_coil_identifiers() {
        let value = fixture(
            json!([[1e-6, 0.0, 0.0], [0.0, 1e-6, 0.0], [0.0, 0.0, 1e-6]]),
            zero_bounds(3),
            "PASSIVE_CERTIFIED",
        )
        .as_object()
        .map(|object| {
            let mut result = Value::Object(object.clone());
            result["coil_ids"] = json!(["TX", "RX1", "TX"]);
            result
        })
        .unwrap();
        assert_eq!(disposition(value), "INVALID_INPUT");
    }

    #[test]
    fn rejects_matrix_shape_mismatch() {
        let value = fixture(
            json!([[1e-6, 0.0], [0.0, 1e-6]]),
            zero_bounds(3),
            "PASSIVE_CERTIFIED",
        );
        assert_eq!(disposition(value), "INVALID_INPUT");
    }

    #[test]
    fn rejects_missing_artifact_digest_for_measured_input() {
        let mut value = fixture(
            json!([[1e-6, 0.0, 0.0], [0.0, 1e-6, 0.0], [0.0, 0.0, 1e-6]]),
            zero_bounds(3),
            "PASSIVE_CERTIFIED",
        );
        value["evidence_kind"] = json!("MEASURED");
        value["source_reference"] = json!("https://example.org/source");
        assert_eq!(disposition(value), "INVALID_INPUT");
    }

    #[test]
    fn keeps_pairwise_coupling_diagnostic_separate_from_global_decision() {
        let value = fixture(
            json!([
                [1e-6, 0.9e-6, 0.9e-6],
                [0.9e-6, 1e-6, -0.9e-6],
                [0.9e-6, -0.9e-6, 1e-6]
            ]),
            zero_bounds(3),
            "NONPASSIVE_REJECTED",
        );
        let result = evaluate_value(&value);
        assert!(result["max_pairwise_abs_coupling"].as_f64().unwrap() < 1.0);
        assert_eq!(result["disposition"], "NONPASSIVE_REJECTED");
    }
}
