// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conservative stored-energy reconstruction for linear reciprocal WPT storage models.

use serde_json::{Value, json};

const MAX_DIMENSION: usize = 32;
const EPS: f64 = f64::EPSILON;

struct Matrix {
    center: Vec<Vec<f64>>,
    uncertainty: Vec<Vec<f64>>,
    reciprocity_adjustment: f64,
}

#[derive(Debug, Clone, Copy)]
struct EigenInterval {
    minimum_lower: f64,
    minimum_upper: f64,
    maximum_lower: f64,
    maximum_upper: f64,
    residual_frobenius: f64,
    uncertainty_frobenius: f64,
    numeric_guard: f64,
    converged: bool,
}

#[derive(Clone, Copy)]
struct StorageSpec<'a> {
    matrix_key: &'a str,
    uncertainty_key: &'a str,
    state_key: &'a str,
    state_uncertainty_key: &'a str,
    matrix_unit: &'a str,
    state_unit: &'a str,
    path: &'a str,
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing {path}.{key}"))
}

fn string_field(value: &Value, key: &str, path: &str) -> Result<String, String> {
    let result = field(value, key, path)?
        .as_str()
        .ok_or_else(|| format!("{path}.{key} must be a string"))?;
    if result.trim().is_empty() {
        return Err(format!("{path}.{key} must not be empty"));
    }
    Ok(result.to_owned())
}

fn number(value: &Value, path: &str) -> Result<f64, String> {
    let x = value
        .as_f64()
        .ok_or_else(|| format!("{path} must be a number"))?;
    if !x.is_finite() {
        return Err(format!("{path} must be finite"));
    }
    Ok(x)
}

fn vector(value: &Value, path: &str, n: usize, nonnegative: bool) -> Result<Vec<f64>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{path} must be an array"))?;
    if values.len() != n {
        return Err(format!("{path} length must equal {n}"));
    }
    let mut result = Vec::with_capacity(n);
    for (i, raw) in values.iter().enumerate() {
        let x = number(raw, &format!("{path}[{i}]"))?;
        if nonnegative && x < 0.0 {
            return Err(format!("{path}[{i}] must be nonnegative"));
        }
        result.push(x);
    }
    Ok(result)
}

fn square_matrix(value: &Value, path: &str, nonnegative: bool) -> Result<Vec<Vec<f64>>, String> {
    let rows = value
        .as_array()
        .ok_or_else(|| format!("{path} must be an array of arrays"))?;
    let n = rows.len();
    if n == 0 || n > MAX_DIMENSION {
        return Err(format!("{path} dimension must be in 1..={MAX_DIMENSION}"));
    }
    let mut result = Vec::with_capacity(n);
    for (i, raw_row) in rows.iter().enumerate() {
        let row = raw_row
            .as_array()
            .ok_or_else(|| format!("{path}[{i}] must be an array"))?;
        if row.len() != n {
            return Err(format!("{path} must be square"));
        }
        let mut parsed = Vec::with_capacity(n);
        for (j, raw) in row.iter().enumerate() {
            let x = number(raw, &format!("{path}[{i}][{j}]"))?;
            if nonnegative && x < 0.0 {
                return Err(format!("{path}[{i}][{j}] must be nonnegative"));
            }
            parsed.push(x);
        }
        result.push(parsed);
    }
    Ok(result)
}

fn parse_matrix(
    component: &Value,
    center_key: &str,
    uncertainty_key: &str,
    unit: &str,
    path: &str,
) -> Result<Matrix, String> {
    let actual_unit = string_field(component, "matrix_unit", path)?;
    if actual_unit != unit {
        return Err(format!("{path}.matrix_unit must be {unit}"));
    }
    let matrix_basis = string_field(component, "matrix_basis_sha256", path)?;
    let state_basis = string_field(component, "state_basis_sha256", path)?;
    let matrix_receipt = string_field(component, "matrix_receipt_sha256", path)?;
    let state_receipt = string_field(component, "state_receipt_sha256", path)?;
    let calibration_receipt = string_field(component, "calibration_receipt_sha256", path)?;
    for (name, digest) in [
        ("matrix_basis_sha256", matrix_basis.as_str()),
        ("state_basis_sha256", state_basis.as_str()),
        ("matrix_receipt_sha256", matrix_receipt.as_str()),
        ("state_receipt_sha256", state_receipt.as_str()),
        ("calibration_receipt_sha256", calibration_receipt.as_str()),
    ] {
        if digest.len() != 64
            || !digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(format!(
                "{path}.{name} must be 64 lowercase hexadecimal characters"
            ));
        }
    }
    if matrix_basis != state_basis {
        return Err(format!(
            "{path} matrix and state coordinate-basis digests differ"
        ));
    }
    let center = square_matrix(
        field(component, center_key, path)?,
        &format!("{path}.{center_key}"),
        false,
    )?;
    let uncertainty = square_matrix(
        field(component, uncertainty_key, path)?,
        &format!("{path}.{uncertainty_key}"),
        true,
    )?;
    let n = center.len();
    if uncertainty.len() != n {
        return Err(format!("{path} matrix and uncertainty dimensions differ"));
    }

    let mut symmetric = center.clone();
    let mut symmetric_uncertainty = uncertainty.clone();
    let mut adjustment = 0.0_f64;
    for i in 0..n {
        for j in (i + 1)..n {
            let lo_a = center[i][j] - uncertainty[i][j];
            let hi_a = center[i][j] + uncertainty[i][j];
            let lo_b = center[j][i] - uncertainty[j][i];
            let hi_b = center[j][i] + uncertainty[j][i];
            if ![lo_a, hi_a, lo_b, hi_b].into_iter().all(f64::is_finite) {
                return Err(format!("{path} reciprocal interval overflow at ({i},{j})"));
            }
            let lo = lo_a.max(lo_b);
            let hi = hi_a.min(hi_b);
            if lo > hi {
                return Err(format!(
                    "{path} reciprocity intervals do not overlap at ({i},{j})"
                ));
            }
            let midpoint = lo * 0.5 + hi * 0.5;
            let radius = hi * 0.5 - lo * 0.5;
            adjustment = adjustment
                .max((midpoint - center[i][j]).abs())
                .max((midpoint - center[j][i]).abs());
            symmetric[i][j] = midpoint;
            symmetric[j][i] = midpoint;
            symmetric_uncertainty[i][j] = radius;
            symmetric_uncertainty[j][i] = radius;
        }
    }
    Ok(Matrix {
        center: symmetric,
        uncertainty: symmetric_uncertainty,
        reciprocity_adjustment: adjustment,
    })
}

fn jacobi(mut a: Vec<Vec<f64>>) -> Result<(f64, f64, f64, bool), String> {
    let n = a.len();
    let mut scale = 0.0_f64;
    for x in a.iter().flatten() {
        scale = scale.max(x.abs());
    }
    if scale == 0.0 {
        return Ok((0.0, 0.0, 0.0, true));
    }
    for row in &mut a {
        for x in row {
            *x /= scale;
        }
    }

    let threshold = 16.0 * EPS;
    let mut converged = false;
    for _ in 0..100 {
        let largest = (0..n)
            .flat_map(|p| ((p + 1)..n).map(move |q| (p, q)))
            .map(|(p, q)| a[p][q].abs())
            .fold(0.0_f64, f64::max);
        if largest <= threshold {
            converged = true;
            break;
        }
        for p in 0..n {
            for q in (p + 1)..n {
                let apq = a[p][q];
                if apq.abs() <= threshold {
                    continue;
                }
                let app = a[p][p];
                let aqq = a[q][q];
                let tau = (aqq - app) / (2.0 * apq);
                let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
                let t = sign / (tau.abs() + tau.hypot(1.0));
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = t * c;
                a[p][p] = app - t * apq;
                a[q][q] = aqq + t * apq;
                a[p][q] = 0.0;
                a[q][p] = 0.0;
                // Compute paired row/column updates from the same pre-rotation values.
                let updates: Vec<Option<(f64, f64)>> = a
                    .iter()
                    .enumerate()
                    .map(|(k, row)| {
                        if k == p || k == q {
                            None
                        } else {
                            let akp = row[p];
                            let akq = row[q];
                            Some((c * akp - s * akq, s * akp + c * akq))
                        }
                    })
                    .collect();

                for (row, update) in a.iter_mut().zip(&updates) {
                    if let Some((new_kp, new_kq)) = update {
                        row[p] = *new_kp;
                        row[q] = *new_kq;
                    }
                }

                let (before_q, from_q) = a.split_at_mut(q);
                let row_p = &mut before_q[p];
                let row_q = &mut from_q[0];
                for ((p_value, q_value), update) in
                    row_p.iter_mut().zip(row_q.iter_mut()).zip(&updates)
                {
                    if let Some((new_kp, new_kq)) = update {
                        *p_value = *new_kp;
                        *q_value = *new_kq;
                    }
                }
            }
        }
    }

    let mut min_diag = f64::INFINITY;
    let mut max_diag = f64::NEG_INFINITY;
    let mut residual_sq = 0.0_f64;
    for (i, row) in a.iter().enumerate() {
        min_diag = min_diag.min(row[i]);
        max_diag = max_diag.max(row[i]);
        for value in row.iter().skip(i + 1) {
            residual_sq += 2.0 * value * value;
        }
    }
    let residual = residual_sq.sqrt();
    if ![min_diag, max_diag, residual]
        .into_iter()
        .all(f64::is_finite)
    {
        return Err("Jacobi eigensolver returned non-finite values".into());
    }
    converged |= residual <= threshold * (n as f64).sqrt();
    Ok((
        min_diag * scale,
        max_diag * scale,
        residual * scale,
        converged,
    ))
}

fn eigen_interval(matrix: &Matrix) -> Result<EigenInterval, String> {
    let mut scale = 0.0_f64;
    for x in matrix
        .center
        .iter()
        .flatten()
        .chain(matrix.uncertainty.iter().flatten())
    {
        scale = scale.max(x.abs());
    }
    if scale == 0.0 {
        return Ok(EigenInterval {
            minimum_lower: 0.0,
            minimum_upper: 0.0,
            maximum_lower: 0.0,
            maximum_upper: 0.0,
            residual_frobenius: 0.0,
            uncertainty_frobenius: 0.0,
            numeric_guard: 0.0,
            converged: true,
        });
    }

    let exact_diagonal = matrix.center.iter().enumerate().all(|(i, row)| {
        row.iter()
            .enumerate()
            .all(|(j, x)| i == j || (*x == 0.0 && matrix.uncertainty[i][j] == 0.0))
    }) && matrix.uncertainty.iter().flatten().all(|x| *x == 0.0);

    if exact_diagonal {
        let diag: Vec<f64> = matrix
            .center
            .iter()
            .enumerate()
            .map(|(i, row)| row[i])
            .collect();
        let min = diag.iter().copied().fold(f64::INFINITY, f64::min);
        let max = diag.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        return Ok(EigenInterval {
            minimum_lower: min,
            minimum_upper: min,
            maximum_lower: max,
            maximum_upper: max,
            residual_frobenius: 0.0,
            uncertainty_frobenius: 0.0,
            numeric_guard: 0.0,
            converged: true,
        });
    }

    let (min_eig, max_eig, residual, converged) = jacobi(matrix.center.clone())?;
    let uncertainty_norm = matrix
        .uncertainty
        .iter()
        .flatten()
        .map(|x| (x / scale).powi(2))
        .sum::<f64>()
        .sqrt()
        * scale;
    let guard = 128.0 * EPS * matrix.center.len() as f64 * scale;
    let radius = residual + uncertainty_norm + guard;
    let values = [
        min_eig - radius,
        min_eig + radius,
        max_eig - radius,
        max_eig + radius,
        uncertainty_norm,
        guard,
    ];
    if !values.into_iter().all(f64::is_finite) {
        return Err("eigenvalue interval overflow".into());
    }
    Ok(EigenInterval {
        minimum_lower: min_eig - radius,
        minimum_upper: min_eig + radius,
        maximum_lower: max_eig - radius,
        maximum_upper: max_eig + radius,
        residual_frobenius: residual,
        uncertainty_frobenius: uncertainty_norm,
        numeric_guard: guard,
        converged,
    })
}

fn norm_bounds(values: &[f64], uncertainty: &[f64]) -> Result<(f64, f64), String> {
    let mut min_sq = 0.0;
    let mut max_sq = 0.0;
    for (x, u) in values.iter().zip(uncertainty) {
        let lo = x - u;
        let hi = x + u;
        if !lo.is_finite() || !hi.is_finite() {
            return Err("state interval overflow".into());
        }
        let min_abs = if lo <= 0.0 && hi >= 0.0 {
            0.0
        } else {
            lo.abs().min(hi.abs())
        };
        let max_abs = lo.abs().max(hi.abs());
        min_sq += min_abs * min_abs;
        max_sq += max_abs * max_abs;
    }
    if !min_sq.is_finite() || !max_sq.is_finite() {
        return Err("state norm overflow".into());
    }
    Ok((min_sq.sqrt(), max_sq.sqrt()))
}

fn energy(component: &Value, spec: StorageSpec<'_>) -> Result<Value, String> {
    let StorageSpec {
        matrix_key,
        uncertainty_key,
        state_key,
        state_uncertainty_key,
        matrix_unit,
        state_unit,
        path,
    } = spec;
    let matrix = parse_matrix(component, matrix_key, uncertainty_key, matrix_unit, path)?;
    let n = matrix.center.len();
    let actual_state_unit = string_field(component, "state_unit", path)?;
    if actual_state_unit != state_unit {
        return Err(format!("{path}.state_unit must be {state_unit}"));
    }
    let values = vector(
        field(component, state_key, path)?,
        &format!("{path}.{state_key}"),
        n,
        false,
    )?;
    let state_uncertainty = vector(
        field(component, state_uncertainty_key, path)?,
        &format!("{path}.{state_uncertainty_key}"),
        n,
        true,
    )?;
    let matrix_basis = string_field(component, "matrix_basis_sha256", path)?;
    let state_basis = string_field(component, "state_basis_sha256", path)?;
    let matrix_receipt = string_field(component, "matrix_receipt_sha256", path)?;
    let state_receipt = string_field(component, "state_receipt_sha256", path)?;
    let calibration_receipt = string_field(component, "calibration_receipt_sha256", path)?;
    if matrix_basis != state_basis {
        return Err(format!(
            "{path} matrix and state coordinate-basis digests differ"
        ));
    }
    let eigen = eigen_interval(&matrix)?;
    let status = if !eigen.converged {
        "STORAGE_MATRIX_UNRESOLVED"
    } else if eigen.minimum_upper < 0.0 {
        "STORAGE_MATRIX_REJECTED"
    } else if eigen.minimum_lower < 0.0 {
        "STORAGE_MATRIX_UNRESOLVED"
    } else {
        "STORAGE_MATRIX_CERTIFIED"
    };

    let mut quadratic = 0.0;
    for i in 0..n {
        for j in 0..n {
            quadratic += values[i] * matrix.center[i][j] * values[j];
        }
    }
    let nominal = 0.5 * quadratic;
    if !nominal.is_finite() {
        return Err(format!("{path} nominal energy overflow"));
    }
    let (norm_lower, norm_upper) = norm_bounds(&values, &state_uncertainty)?;
    let lower = 0.5 * eigen.minimum_lower.max(0.0) * norm_lower * norm_lower;
    let upper = 0.5 * eigen.maximum_upper.max(0.0) * norm_upper * norm_upper;
    if !lower.is_finite() || !upper.is_finite() {
        return Err(format!("{path} conservative energy interval overflow"));
    }

    Ok(json!({
        "status": status,
        "matrix_unit": matrix_unit,
        "state_unit": state_unit,
        "matrix_basis_sha256": matrix_basis,
        "state_basis_sha256": state_basis,
        "matrix_receipt_sha256": matrix_receipt,
        "state_receipt_sha256": state_receipt,
        "calibration_receipt_sha256": calibration_receipt,
        "nominal_energy_j": nominal,
        "energy_interval_j": { "lower": lower, "upper": upper },
        "minimum_eigenvalue_interval": { "lower": eigen.minimum_lower, "upper": eigen.minimum_upper },
        "maximum_eigenvalue_interval": { "lower": eigen.maximum_lower, "upper": eigen.maximum_upper },
        "state_norm_interval": { "lower": norm_lower, "upper": norm_upper },
        "off_diagonal_residual_frobenius": eigen.residual_frobenius,
        "matrix_uncertainty_frobenius": eigen.uncertainty_frobenius,
        "floating_point_guard": eigen.numeric_guard,
        "reciprocity_max_interval_adjustment": matrix.reciprocity_adjustment,
        "eigensolver_converged": eigen.converged
    }))
}

pub fn evaluate(record: &Value) -> Value {
    let record_id = record
        .get("record_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    match evaluate_checked(record, &record_id) {
        Ok(report) => report,
        Err(message) => {
            json!({ "record_id": record_id, "disposition": "INVALID_INPUT", "message": message })
        }
    }
}

fn evaluate_checked(record: &Value, record_id: &str) -> Result<Value, String> {
    if string_field(record, "record_id", "record")? != record_id {
        return Err("record_id must be non-empty".into());
    }
    let version = string_field(record, "schema_version", "record")?;
    if version != "1.0.0" {
        return Err(format!("unsupported schema_version {version}"));
    }
    let mode_id = string_field(record, "mode_id", "record")?;
    let digest = string_field(record, "topology_sha256", "record")?;
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("topology_sha256 must be 64 lowercase hexadecimal characters".into());
    }
    if string_field(record, "model_class", "record")? != "LINEAR_RECIPROCAL_QUASI_STATIC" {
        return Err("v1 accepts only LINEAR_RECIPROCAL_QUASI_STATIC".into());
    }

    let magnetic = energy(
        field(record, "inductance", "record")?,
        StorageSpec {
            matrix_key: "matrix_h",
            uncertainty_key: "abs_uncertainty_h",
            state_key: "current_a",
            state_uncertainty_key: "current_abs_uncertainty_a",
            matrix_unit: "H",
            state_unit: "A",
            path: "inductance",
        },
    )?;
    let electric = energy(
        field(record, "capacitance", "record")?,
        StorageSpec {
            matrix_key: "matrix_f",
            uncertainty_key: "abs_uncertainty_f",
            state_key: "voltage_v",
            state_uncertainty_key: "voltage_abs_uncertainty_v",
            matrix_unit: "F",
            state_unit: "V",
            path: "capacitance",
        },
    )?;
    let m_status = magnetic["status"]
        .as_str()
        .unwrap_or("STORAGE_MATRIX_UNRESOLVED");
    let e_status = electric["status"]
        .as_str()
        .unwrap_or("STORAGE_MATRIX_UNRESOLVED");

    let (disposition, message) = if m_status == "STORAGE_MATRIX_REJECTED"
        || e_status == "STORAGE_MATRIX_REJECTED"
    {
        (
            "STORAGE_MODEL_REJECTED",
            "at least one matrix is provably non-PSD under its uncertainty bounds",
        )
    } else if m_status != "STORAGE_MATRIX_CERTIFIED" || e_status != "STORAGE_MATRIX_CERTIFIED" {
        (
            "STORAGE_MODEL_UNRESOLVED",
            "at least one storage matrix lacks a robust PSD certificate",
        )
    } else {
        (
            "STORED_ENERGY_DERIVED",
            "magnetic and electric storage energies are derived from qualified matrices and bounded states",
        )
    };

    let magnetic_nominal = magnetic["nominal_energy_j"].as_f64().unwrap_or(f64::NAN);
    let electric_nominal = electric["nominal_energy_j"].as_f64().unwrap_or(f64::NAN);
    let magnetic_low = magnetic["energy_interval_j"]["lower"]
        .as_f64()
        .unwrap_or(f64::NAN);
    let electric_low = electric["energy_interval_j"]["lower"]
        .as_f64()
        .unwrap_or(f64::NAN);
    let magnetic_high = magnetic["energy_interval_j"]["upper"]
        .as_f64()
        .unwrap_or(f64::NAN);
    let electric_high = electric["energy_interval_j"]["upper"]
        .as_f64()
        .unwrap_or(f64::NAN);
    let total_nominal = magnetic_nominal + electric_nominal;
    let total_lower = magnetic_low + electric_low;
    let total_upper = magnetic_high + electric_high;
    if ![total_nominal, total_lower, total_upper]
        .into_iter()
        .all(f64::is_finite)
    {
        return Err("combined stored-energy interval overflow".into());
    }

    Ok(json!({
        "record_id": record_id,
        "mode_id": mode_id,
        "topology_sha256": digest,
        "model_class": "LINEAR_RECIPROCAL_QUASI_STATIC",
        "disposition": disposition,
        "message": message,
        "magnetic_storage": magnetic,
        "electric_storage": electric,
        "total_stored_energy_j": { "nominal": total_nominal, "lower": total_lower, "upper": total_upper },
        "claim_ceiling": "storage matrices and supplied state intervals only; not a complete network passivity or hardware safety certificate"
    }))
}

#[cfg(test)]
mod tests {
    use super::evaluate;
    use serde_json::{Value, json};

    const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    // This test fixture mirrors four independent physical value/uncertainty pairs.
    #[expect(
        clippy::too_many_arguments,
        reason = "keeps the fixture's physical input pairs explicit"
    )]
    fn record(
        id: &str,
        l: Value,
        lu: Value,
        i: Value,
        iu: Value,
        c: Value,
        cu: Value,
        v: Value,
        vu: Value,
    ) -> Value {
        json!({
            "schema_version":"1.0.0", "record_id":id, "mode_id":"LCC_S", "topology_sha256":SHA,
            "model_class":"LINEAR_RECIPROCAL_QUASI_STATIC",
            "inductance":{"matrix_unit":"H","matrix_h":l,"abs_uncertainty_h":lu,"state_unit":"A","current_a":i,"current_abs_uncertainty_a":iu,"matrix_basis_sha256":SHA,"state_basis_sha256":SHA,"matrix_receipt_sha256":SHA,"state_receipt_sha256":SHA,"calibration_receipt_sha256":SHA},
            "capacitance":{"matrix_unit":"F","matrix_f":c,"abs_uncertainty_f":cu,"state_unit":"V","voltage_v":v,"voltage_abs_uncertainty_v":vu,"matrix_basis_sha256":SHA,"state_basis_sha256":SHA,"matrix_receipt_sha256":SHA,"state_receipt_sha256":SHA,"calibration_receipt_sha256":SHA}
        })
    }

    fn basic(id: &str) -> Value {
        record(
            id,
            json!([[2e-6, 0.5e-6], [0.5e-6, 1e-6]]),
            json!([[1e-12, 1e-12], [1e-12, 1e-12]]),
            json!([0.4, 0.2]),
            json!([0.001, 0.001]),
            json!([[1e-6, 0.2e-6], [0.2e-6, 0.8e-6]]),
            json!([[1e-12, 1e-12], [1e-12, 1e-12]]),
            json!([100.0, 50.0]),
            json!([0.1, 0.1]),
        )
    }

    fn disposition(value: &Value) -> String {
        evaluate(value)["disposition"].as_str().unwrap().to_owned()
    }

    #[test]
    fn rejects_coordinate_basis_mismatch_and_missing_receipts() {
        let mut mismatch = basic("basis-mismatch");
        mismatch["inductance"]["state_basis_sha256"] = json!("9".repeat(64));
        assert_eq!(disposition(&mismatch), "INVALID_INPUT");

        let mut missing = basic("missing-receipt");
        missing["capacitance"]
            .as_object_mut()
            .unwrap()
            .remove("calibration_receipt_sha256");
        assert_eq!(disposition(&missing), "INVALID_INPUT");
    }

    #[test]
    fn derives_analytic_magnetic_and_electric_energy() {
        let output = evaluate(&basic("analytic"));
        assert_eq!(output["disposition"], "STORED_ENERGY_DERIVED");
        let magnetic = output["magnetic_storage"]["nominal_energy_j"]
            .as_f64()
            .unwrap();
        let electric = output["electric_storage"]["nominal_energy_j"]
            .as_f64()
            .unwrap();
        assert!((magnetic - 0.22e-6).abs() < 1e-18);
        assert!((electric - 0.007).abs() < 1e-14);
    }

    #[test]
    fn certifies_exact_singular_diagonal_psd_matrix() {
        let x = record(
            "singular",
            json!([[1e-6, 0.0], [0.0, 0.0]]),
            json!([[0.0, 0.0], [0.0, 0.0]]),
            json!([1.0, 0.5]),
            json!([0.0, 0.0]),
            json!([[1e-6]]),
            json!([[0.0]]),
            json!([10.0]),
            json!([0.0]),
        );
        assert_eq!(disposition(&x), "STORED_ENERGY_DERIVED");
    }

    #[test]
    fn rejects_provably_indefinite_matrix() {
        let mut x = basic("indefinite");
        x["inductance"]["matrix_h"] = json!([[1e-6, 2e-6], [2e-6, 1e-6]]);
        x["inductance"]["abs_uncertainty_h"] = json!([[0.0, 0.0], [0.0, 0.0]]);
        assert_eq!(disposition(&x), "STORAGE_MODEL_REJECTED");
    }

    #[test]
    fn abstains_at_psd_boundary_with_overlapping_uncertainty() {
        let mut x = basic("boundary");
        x["inductance"]["matrix_h"] = json!([[1e-6, 0.999999e-6], [0.999999e-6, 1e-6]]);
        x["inductance"]["abs_uncertainty_h"] = json!([[0.0, 2e-9], [2e-9, 0.0]]);
        assert_eq!(disposition(&x), "STORAGE_MODEL_UNRESOLVED");
    }

    #[test]
    fn reconciles_reciprocity_only_when_intervals_overlap() {
        let mut x = basic("reciprocal-overlap");
        x["inductance"]["matrix_h"] = json!([[2e-6, 0.5e-6], [0.5001e-6, 1e-6]]);
        x["inductance"]["abs_uncertainty_h"] =
            json!([[1e-12, 1e-10], [1e-10, 1e-12]]);
        assert_eq!(disposition(&x), "STORED_ENERGY_DERIVED");
        assert!(
            evaluate(&x)["magnetic_storage"]["reciprocity_max_interval_adjustment"]
                .as_f64()
                .unwrap()
                > 0.0
        );
    }

    #[test]
    fn rejects_nonoverlapping_reciprocity_intervals() {
        let mut x = basic("reciprocal-conflict");
        x["inductance"]["matrix_h"] = json!([[2e-6, 0.4e-6], [0.6e-6, 1e-6]]);
        x["inductance"]["abs_uncertainty_h"] = json!([[0.0, 1e-9], [1e-9, 0.0]]);
        assert_eq!(disposition(&x), "INVALID_INPUT");
    }

    #[test]
    fn rejects_state_dimension_mismatch() {
        let mut x = basic("dimension");
        x["inductance"]["current_a"] = json!([0.2]);
        assert_eq!(disposition(&x), "INVALID_INPUT");
    }

    #[test]
    fn rejects_unit_substitution_and_unqualified_models() {
        let mut unit = basic("unit");
        unit["capacitance"]["state_unit"] = json!("mV");
        assert_eq!(disposition(&unit), "INVALID_INPUT");
        let mut nonlinear = basic("nonlinear");
        nonlinear["model_class"] = json!("NONLINEAR_HYSTERETIC");
        assert_eq!(disposition(&nonlinear), "INVALID_INPUT");
    }

    #[test]
    fn seeded_gram_matrices_are_psd_for_dimensions_two_to_sixteen() {
        let mut seed = 0x9e3779b97f4a7c15_u64;
        for n in 2..=16 {
            let mut a = vec![vec![0.0_f64; n]; n];
            for row in &mut a {
                for value in row {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    *value = ((seed >> 11) as f64 / ((1_u64 << 53) as f64)) - 0.5;
                }
            }
            let mut gram = vec![vec![0.0_f64; n]; n];
            for i in 0..n {
                for j in 0..n {
                    gram[i][j] = (0..n).map(|k| a[k][i] * a[k][j]).sum::<f64>() * 1e-6
                        + if i == j { 1e-6 } else { 0.0 };
                }
            }
            let state: Vec<f64> = (0..n).map(|i| (i as f64 + 1.0) * 0.01).collect();
            let zeros = vec![vec![0.0; n]; n];
            let x = record(
                &format!("gram-{n}"),
                json!(gram),
                json!(zeros),
                json!(state),
                json!(vec![0.0; n]),
                json!([[1e-6]]),
                json!([[0.0]]),
                json!([1.0]),
                json!([0.0]),
            );
            assert_eq!(disposition(&x), "STORED_ENERGY_DERIVED", "dimension {n}");
        }
    }

    #[test]
    fn rejects_non_psd_capacitance_matrix() {
        let mut value = basic("indefinite-capacitance");
        value["capacitance"]["matrix_f"] = json!([[1e-6, 2e-6], [2e-6, 1e-6]]);
        value["capacitance"]["abs_uncertainty_f"] = json!([[0.0, 0.0], [0.0, 0.0]]);
        assert_eq!(disposition(&value), "STORAGE_MODEL_REJECTED");
    }

    #[test]
    fn conservative_energy_interval_contains_seeded_perturbations() {
        let output = evaluate(&basic("perturbation-coverage"));
        let lower = output["total_stored_energy_j"]["lower"].as_f64().unwrap();
        let upper = output["total_stored_energy_j"]["upper"].as_f64().unwrap();
        let mut seed = 0xd1b54a32d192ed03_u64;
        let mut unit = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed >> 11) as f64 / ((1_u64 << 53) as f64)
        };

        for _ in 0..1000 {
            let l00 = 2.0e-6 + (unit() - 0.5) * 2.0e-12;
            let l11 = 1.0e-6 + (unit() - 0.5) * 2.0e-12;
            let l01 = 0.5e-6 + (unit() - 0.5) * 2.0e-12;
            let i0 = 0.4 + (unit() - 0.5) * 0.002;
            let i1 = 0.2 + (unit() - 0.5) * 0.002;
            let magnetic = 0.5 * (l00 * i0 * i0 + 2.0 * l01 * i0 * i1 + l11 * i1 * i1);

            let c00 = 1.0e-6 + (unit() - 0.5) * 2.0e-12;
            let c11 = 0.8e-6 + (unit() - 0.5) * 2.0e-12;
            let c01 = 0.2e-6 + (unit() - 0.5) * 2.0e-12;
            let v0 = 100.0 + (unit() - 0.5) * 0.2;
            let v1 = 50.0 + (unit() - 0.5) * 0.2;
            let electric = 0.5 * (c00 * v0 * v0 + 2.0 * c01 * v0 * v1 + c11 * v1 * v1);
            let total = magnetic + electric;
            assert!(total >= lower, "sampled energy {total:e} below {lower:e}");
            assert!(total <= upper, "sampled energy {total:e} above {upper:e}");
        }
    }

    #[test]
    fn energy_interval_contains_nominal_value() {
        let output = evaluate(&basic("interval"));
        let nominal = output["total_stored_energy_j"]["nominal"].as_f64().unwrap();
        let lower = output["total_stored_energy_j"]["lower"].as_f64().unwrap();
        let upper = output["total_stored_energy_j"]["upper"].as_f64().unwrap();
        assert!(lower <= nominal && nominal <= upper);
    }

    #[test]
    fn handles_small_and_large_scales() {
        let tiny = record(
            "tiny",
            json!([[1e-308]]),
            json!([[0.0]]),
            json!([1e100]),
            json!([0.0]),
            json!([[1e-308]]),
            json!([[0.0]]),
            json!([1e100]),
            json!([0.0]),
        );
        assert_eq!(disposition(&tiny), "STORED_ENERGY_DERIVED");
        let large = record(
            "large",
            json!([[1e308]]),
            json!([[0.0]]),
            json!([1e-100]),
            json!([0.0]),
            json!([[1e308]]),
            json!([[0.0]]),
            json!([1e-100]),
            json!([0.0]),
        );
        assert_eq!(disposition(&large), "STORED_ENERGY_DERIVED");
    }

    #[test]
    fn supports_different_current_and_voltage_dimensions() {
        let x = record(
            "different-dimensions",
            json!([[2e-6, 0.1e-6], [0.1e-6, 1e-6]]),
            json!([[0.0, 0.0], [0.0, 0.0]]),
            json!([0.2, 0.4]),
            json!([0.0, 0.0]),
            json!([[1e-6]]),
            json!([[0.0]]),
            json!([12.0]),
            json!([0.0]),
        );
        assert_eq!(disposition(&x), "STORED_ENERGY_DERIVED");
    }
}
