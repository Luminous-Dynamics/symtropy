// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Noise-aware practical identifiability and measurement-design diagnostics.
//!
//! This bounded linear-Gaussian reference separates exact structural rank from the
//! information available under declared observation standard deviations and correlation.
//! It is not a field error model, Bayesian inversion engine, or ecological parameter fit.

use crate::flux_identifiability_kernel::exact_structural_rank_for_observation_rows;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

const MAX_VARIABLES: usize = 16;
const MAX_MEASUREMENT_OPTIONS: usize = 12;
const MAX_OBSERVATIONS: usize = 64;
const NUMERIC_TOLERANCE: f64 = 1e-12;

#[derive(Clone, Debug)]
struct Observation {
    id: String,
    row: Vec<i64>,
    standard_deviation: f64,
}

#[derive(Clone, Debug)]
struct MeasurementOption {
    observation: Observation,
    cost: u64,
}

#[derive(Clone, Debug)]
struct InformationMetrics {
    eigenvalues: Vec<f64>,
    minimum_eigenvalue: f64,
    maximum_eigenvalue: f64,
    condition_number: f64,
    log_determinant: f64,
    trace_inverse: f64,
    gls_covariance_diagonal: Vec<f64>,
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn number_at(value: &Value, key: &str) -> Result<f64, String> {
    let number = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key} must be a number"))?;
    if !number.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(number)
}

fn usize_at(value: &Value, key: &str) -> Result<usize, String> {
    let number = value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))?;
    usize::try_from(number).map_err(|_| format!("{key} exceeds platform usize"))
}

fn parse_row(value: &Value, key: &str, variable_count: usize) -> Result<Vec<i64>, String> {
    let entries = value.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an integer array"))?;
    if entries.len() != variable_count {
        return Err(format!("{key} has {} coefficients; expected {variable_count}", entries.len()));
    }
    entries.iter().enumerate().map(|(index, entry)| {
        let coefficient = entry.as_i64()
            .ok_or_else(|| format!("{key}[{index}] must be an integer"))?;
        if !(-1_000_000..=1_000_000).contains(&coefficient) {
            return Err(format!("{key}[{index}] exceeds coefficient bounds"));
        }
        Ok(coefficient)
    }).collect()
}

fn parse_observations(
    manifest: &Value,
    key: &str,
    variable_count: usize,
    seen: &mut BTreeSet<String>,
) -> Result<Vec<Observation>, String> {
    let entries = manifest.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array"))?;
    if entries.is_empty() {
        return Err(format!("{key} must not be empty"));
    }
    let mut parsed = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let id = string_at(entry, "observation_id")
            .map_err(|error| format!("{key}[{index}]: {error}"))?;
        if id.trim().is_empty() || !seen.insert(id.clone()) {
            return Err(format!("{key}[{index}] has an empty or duplicate observation ID"));
        }
        let standard_deviation = number_at(entry, "standard_deviation")?;
        if standard_deviation <= 0.0 {
            return Err(format!("{key}[{index}].standard_deviation must be positive; zero/unknown is not infinite precision"));
        }
        let row = parse_row(entry, "row", variable_count)?;
        parsed.push(Observation { id, row, standard_deviation });
    }
    Ok(parsed)
}

fn parse_options(
    manifest: &Value,
    variable_count: usize,
    seen: &mut BTreeSet<String>,
) -> Result<Vec<MeasurementOption>, String> {
    let entries = manifest.get("measurement_options").and_then(Value::as_array)
        .ok_or_else(|| "measurement_options must be an array".to_string())?;
    if entries.is_empty() || entries.len() > MAX_MEASUREMENT_OPTIONS {
        return Err(format!("measurement_options count must be in 1..={MAX_MEASUREMENT_OPTIONS}"));
    }
    let mut parsed = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let id = string_at(entry, "observation_id")?;
        if id.trim().is_empty() || !seen.insert(id.clone()) {
            return Err(format!("measurement option {index} has empty or duplicate identity"));
        }
        let standard_deviation = number_at(entry, "standard_deviation")?;
        if standard_deviation <= 0.0 {
            return Err(format!("measurement option {id} standard_deviation must be positive"));
        }
        let cost_value = entry.get("synthetic_effort_cost").and_then(Value::as_u64)
            .ok_or_else(|| format!("measurement option {id} synthetic_effort_cost must be a non-negative integer"))?;
        let row = parse_row(entry, "row", variable_count)?;
        parsed.push(MeasurementOption {
            observation: Observation { id, row, standard_deviation },
            cost: cost_value,
        });
    }
    Ok(parsed)
}

fn correlation_map(
    manifest: &Value,
    known_ids: &BTreeSet<String>,
) -> Result<BTreeMap<(String, String), f64>, String> {
    let entries = manifest.get("correlations").and_then(Value::as_array)
        .ok_or_else(|| "correlations must be an array".to_string())?;
    let mut correlations = BTreeMap::new();
    for (index, entry) in entries.iter().enumerate() {
        let left = string_at(entry, "left_observation_id")?;
        let right = string_at(entry, "right_observation_id")?;
        let rho = number_at(entry, "correlation")?;
        if left == right || !known_ids.contains(&left) || !known_ids.contains(&right) {
            return Err(format!("correlations[{index}] references invalid or identical observation IDs"));
        }
        if !(-1.0..1.0).contains(&rho) {
            return Err(format!("correlations[{index}].correlation must be strictly between -1 and 1"));
        }
        let key = if left < right { (left, right) } else { (right, left) };
        if correlations.insert(key, rho).is_some() {
            return Err(format!("correlations[{index}] duplicates an existing pair"));
        }
    }
    Ok(correlations)
}

fn covariance_matrix(
    observations: &[Observation],
    correlations: &BTreeMap<(String, String), f64>,
) -> Vec<Vec<f64>> {
    let count = observations.len();
    let mut covariance = vec![vec![0.0; count]; count];
    for row in 0..count {
        for column in 0..count {
            let correlation = if row == column {
                1.0
            } else {
                let left = &observations[row].id;
                let right = &observations[column].id;
                let key = if left < right {
                    (left.clone(), right.clone())
                } else {
                    (right.clone(), left.clone())
                };
                correlations.get(&key).copied().unwrap_or(0.0)
            };
            covariance[row][column] = observations[row].standard_deviation
                * observations[column].standard_deviation * correlation;
        }
    }
    covariance
}

fn cholesky_lower(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let count = matrix.len();
    if count == 0 || matrix.iter().any(|row| row.len() != count) {
        return Err("covariance matrix must be non-empty and square".to_string());
    }
    let mut lower = vec![vec![0.0; count]; count];
    for row in 0..count {
        for column in 0..=row {
            if (matrix[row][column] - matrix[column][row]).abs() > NUMERIC_TOLERANCE {
                return Err("covariance matrix must be symmetric".to_string());
            }
            let mut residual = matrix[row][column];
            for inner in 0..column {
                residual -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !residual.is_finite() || residual <= NUMERIC_TOLERANCE {
                    return Err("measurement covariance must be positive definite".to_string());
                }
                lower[row][column] = residual.sqrt();
            } else {
                lower[row][column] = residual / lower[column][column];
                if !lower[row][column].is_finite() {
                    return Err("covariance Cholesky factor is non-finite".to_string());
                }
            }
        }
    }
    Ok(lower)
}

fn whiten_design(
    lower: &[Vec<f64>],
    rows: &[Vec<i64>],
    variable_count: usize,
) -> Result<Vec<Vec<f64>>, String> {
    if lower.len() != rows.len() {
        return Err("covariance and observation row counts differ".to_string());
    }
    let count = rows.len();
    let mut whitened = vec![vec![0.0; variable_count]; count];
    for variable in 0..variable_count {
        for row in 0..count {
            let mut residual = rows[row][variable] as f64;
            for prior in 0..row {
                residual -= lower[row][prior] * whitened[prior][variable];
            }
            whitened[row][variable] = residual / lower[row][row];
            if !whitened[row][variable].is_finite() {
                return Err("whitened observation operator is non-finite".to_string());
            }
        }
    }
    Ok(whitened)
}

fn fisher_information(whitened: &[Vec<f64>], variables: usize) -> Vec<Vec<f64>> {
    let mut fisher = vec![vec![0.0; variables]; variables];
    for left in 0..variables {
        for right in 0..variables {
            fisher[left][right] = whitened.iter()
                .map(|row| row[left] * row[right])
                .sum();
        }
    }
    fisher
}

fn inverse(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let count = matrix.len();
    if count == 0 || matrix.iter().any(|row| row.len() != count) {
        return Err("matrix inverse requires a non-empty square matrix".to_string());
    }
    let mut augmented = vec![vec![0.0; 2 * count]; count];
    let mut scale: f64 = 0.0;
    for row in 0..count {
        for column in 0..count {
            augmented[row][column] = matrix[row][column];
            scale = scale.max(matrix[row][column].abs());
        }
        augmented[row][count + row] = 1.0;
    }
    let threshold = (scale * 1e-14).max(1e-15);
    for column in 0..count {
        let mut pivot_row = column;
        for row in (column + 1)..count {
            if augmented[row][column].abs() > augmented[pivot_row][column].abs() {
                pivot_row = row;
            }
        }
        if augmented[pivot_row][column].abs() <= threshold {
            return Err("matrix is numerically singular under the declared tolerance".to_string());
        }
        augmented.swap(column, pivot_row);
        let pivot = augmented[column][column];
        for entry in 0..(2 * count) {
            augmented[column][entry] /= pivot;
        }
        for row in 0..count {
            if row == column {
                continue;
            }
            let factor = augmented[row][column];
            for entry in 0..(2 * count) {
                augmented[row][entry] -= factor * augmented[column][entry];
            }
        }
    }
    let result: Vec<Vec<f64>> = augmented.iter()
        .map(|row| row[count..].to_vec())
        .collect();
    if result.iter().flatten().any(|value| !value.is_finite()) {
        return Err("matrix inverse produced a non-finite value".to_string());
    }
    Ok(result)
}

fn symmetric_eigenvalues(matrix: &[Vec<f64>]) -> Result<Vec<f64>, String> {
    let count = matrix.len();
    if count == 0 || matrix.iter().any(|row| row.len() != count) {
        return Err("eigenvalue calculation requires a non-empty square matrix".to_string());
    }
    let mut work = matrix.to_vec();
    let scale = work.iter().flatten().fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    let tolerance = (scale * 1e-13).max(1e-15);
    let maximum_iterations = 100usize.saturating_mul(count).saturating_mul(count);
    for _ in 0..maximum_iterations {
        let mut pivot_left = 0usize;
        let mut pivot_right = 0usize;
        let mut maximum_off_diagonal = 0.0_f64;
        for row in 0..count {
            for column in (row + 1)..count {
                if work[row][column].abs() > maximum_off_diagonal {
                    maximum_off_diagonal = work[row][column].abs();
                    pivot_left = row;
                    pivot_right = column;
                }
            }
        }
        if maximum_off_diagonal <= tolerance {
            break;
        }
        let left = pivot_left;
        let right = pivot_right;
        let angle = 0.5 * (2.0 * work[left][right])
            .atan2(work[right][right] - work[left][left]);
        let cosine = angle.cos();
        let sine = angle.sin();
        let app = work[left][left];
        let aqq = work[right][right];
        let apq = work[left][right];
        work[left][left] = cosine * cosine * app
            - 2.0 * sine * cosine * apq
            + sine * sine * aqq;
        work[right][right] = sine * sine * app
            + 2.0 * sine * cosine * apq
            + cosine * cosine * aqq;
        work[left][right] = 0.0;
        work[right][left] = 0.0;
        for index in 0..count {
            if index == left || index == right {
                continue;
            }
            let left_value = work[index][left];
            let right_value = work[index][right];
            work[index][left] = cosine * left_value - sine * right_value;
            work[left][index] = work[index][left];
            work[index][right] = sine * left_value + cosine * right_value;
            work[right][index] = work[index][right];
        }
    }

    let mut values: Vec<f64> = (0..count).map(|index| work[index][index]).collect();
    if values.iter().any(|value| !value.is_finite()) {
        return Err("Fisher information eigenvalues are non-finite".to_string());
    }
    values.sort_by(f64::total_cmp);
    Ok(values)
}

fn information_metrics(
    observations: &[Observation],
    rows: &[Vec<i64>],
    variable_count: usize,
    correlations: &BTreeMap<(String, String), f64>,
) -> Result<InformationMetrics, String> {
    let covariance = covariance_matrix(observations, correlations);
    let lower = cholesky_lower(&covariance)?;
    let whitened = whiten_design(&lower, rows, variable_count)?;
    let fisher = fisher_information(&whitened, variable_count);
    let eigenvalues = symmetric_eigenvalues(&fisher)?;
    let minimum_eigenvalue = eigenvalues[0];
    let maximum_eigenvalue = eigenvalues[variable_count - 1];
    if minimum_eigenvalue <= 0.0 || !minimum_eigenvalue.is_finite() {
        return Err("Fisher information is not positive definite for a claimed full-rank design".to_string());
    }
    let condition_number = maximum_eigenvalue / minimum_eigenvalue;
    let log_determinant = eigenvalues.iter().map(|value| value.ln()).sum::<f64>();
    let gls_covariance = inverse(&fisher)?;
    let trace_inverse = (0..variable_count).map(|index| gls_covariance[index][index]).sum::<f64>();
    if !condition_number.is_finite() || !log_determinant.is_finite() || !trace_inverse.is_finite() {
        return Err("practical-identifiability metrics are non-finite".to_string());
    }
    Ok(InformationMetrics {
        eigenvalues,
        minimum_eigenvalue,
        maximum_eigenvalue,
        condition_number,
        log_determinant,
        trace_inverse,
        gls_covariance_diagonal: (0..variable_count)
            .map(|index| gls_covariance[index][index])
            .collect(),
    })
}

fn almost_equal(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance * (1.0 + left.abs().max(right.abs()))
}

fn collect_optima(
    designs: &[Value],
    metric_name: &str,
    maximize: bool,
) -> Result<Vec<Vec<String>>, String> {
    let eligible: Vec<&Value> = designs.iter()
        .filter(|design| design["structurally_identifiable"].as_bool() == Some(true))
        .collect();
    if eligible.is_empty() {
        return Err("no structurally full-rank designs available for practical comparison".to_string());
    }
    let mut best = if maximize { f64::NEG_INFINITY } else { f64::INFINITY };
    for design in &eligible {
        let value = design[metric_name].as_f64()
            .ok_or_else(|| format!("full-rank design lacks metric {metric_name}"))?;
        if (maximize && value > best) || (!maximize && value < best) {
            best = value;
        }
    }
    let mut result = Vec::new();
    for design in eligible {
        let value = design[metric_name].as_f64().unwrap_or(f64::NAN);
        if almost_equal(value, best, 1e-9) {
            let ids = design["measurement_ids"].as_array()
                .ok_or_else(|| "measurement_ids must be an array".to_string())?
                .iter()
                .map(|id| id.as_str().map(ToOwned::to_owned)
                    .ok_or_else(|| "measurement ID must be a string".to_string()))
                .collect::<Result<Vec<_>, _>>()?;
            result.push(ids);
        }
    }
    result.sort();
    Ok(result)
}

fn evaluate_designs(
    base: &[Observation],
    options: &[MeasurementOption],
    correlations: &BTreeMap<(String, String), f64>,
    variable_count: usize,
) -> Result<Vec<Value>, String> {
    let subset_count = 1_u32.checked_shl(
        u32::try_from(options.len()).map_err(|_| "too many measurement options".to_string())?
    ).ok_or_else(|| "measurement subset count overflow".to_string())?;
    let capacity = usize::try_from(subset_count)
        .map_err(|_| "measurement subset count exceeds platform usize".to_string())?;
    let mut designs = Vec::with_capacity(capacity);

    for mask in 0..subset_count {
        let mut observations = base.to_vec();
        let mut matrix: Vec<Vec<i64>> = base.iter().map(|observation| observation.row.clone()).collect();
        let mut measurement_ids = Vec::new();
        let mut cost = 0_u64;
        for (index, option) in options.iter().enumerate() {
            if mask & (1_u32 << index) != 0 {
                observations.push(option.observation.clone());
                matrix.push(option.observation.row.clone());
                measurement_ids.push(option.observation.id.clone());
                cost = cost.checked_add(option.cost)
                    .ok_or_else(|| "measurement design cost overflow".to_string())?;
            }
        }
        let rank = exact_structural_rank_for_observation_rows(&matrix, variable_count)?;
        let structurally_identifiable = rank == variable_count;
        let metrics = if structurally_identifiable {
            Some(information_metrics(&observations, &matrix, variable_count, correlations)?)
        } else {
            None
        };
        let (minimum_eigenvalue, maximum_eigenvalue, condition_number, log_determinant, trace_inverse, covariance_diagonal, eigenvalues) =
            if let Some(metrics) = metrics {
                (
                    Some(metrics.minimum_eigenvalue),
                    Some(metrics.maximum_eigenvalue),
                    Some(metrics.condition_number),
                    Some(metrics.log_determinant),
                    Some(metrics.trace_inverse),
                    Some(metrics.gls_covariance_diagonal),
                    Some(metrics.eigenvalues),
                )
            } else {
                (None, None, None, None, None, None, None)
            };
        designs.push(json!({
            "measurement_ids": measurement_ids,
            "measurement_count": measurement_ids.len(),
            "synthetic_effort_cost": cost,
            "structural_rank": rank,
            "structural_nullity": variable_count - rank,
            "structurally_identifiable": structurally_identifiable,
            "minimum_fisher_eigenvalue": minimum_eigenvalue,
            "maximum_fisher_eigenvalue": maximum_eigenvalue,
            "fisher_condition_number": condition_number,
            "log_determinant_fisher": log_determinant,
            "trace_gls_covariance": trace_inverse,
            "gls_covariance_diagonal": covariance_diagonal,
            "fisher_eigenvalues": eigenvalues
        }));
    }
    Ok(designs)
}

/// Validate a noise-aware linear-Gaussian practical-identifiability corpus.
pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let schema_version = string_at(manifest, "schema_version")?;
    let benchmark_id = string_at(manifest, "benchmark_id")?;
    let corpus_id = string_at(manifest, "corpus_id")?;
    let claim_ceiling = string_at(manifest, "claim_ceiling")?;
    if schema_version != "0.1.0"
        || benchmark_id != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || corpus_id != "FOREST-FLUX-PRACTICAL-IDENTIFIABILITY-001"
        || !claim_ceiling.contains("does not establish practical identifiability in real forests")
    {
        return Err("practical-identifiability corpus identity or claim ceiling is invalid".to_string());
    }

    let variables = manifest.get("unknown_fluxes").and_then(Value::as_array)
        .ok_or_else(|| "unknown_fluxes must be an array".to_string())?;
    let variable_names: Vec<String> = variables.iter()
        .map(|value| value.as_str().map(ToOwned::to_owned)
            .ok_or_else(|| "unknown_fluxes entries must be strings".to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let variable_count = variable_names.len();
    if variable_count == 0 || variable_count > MAX_VARIABLES {
        return Err(format!("unknown flux count must be in 1..={MAX_VARIABLES}"));
    }
    let mut seen_ids = BTreeSet::new();
    if variable_names.iter().any(|name| name.trim().is_empty() || !seen_ids.insert(name.clone())) {
        return Err("unknown flux names must be non-empty and unique".to_string());
    }

    let base = parse_observations(manifest, "base_observations", variable_count, &mut BTreeSet::new())?;
    let base_ids: BTreeSet<String> = base.iter().map(|item| item.id.clone()).collect();
    let options = parse_options(manifest, variable_count, &mut base_ids.clone())?;
    let mut known_ids = base_ids.clone();
    known_ids.extend(options.iter().map(|option| option.observation.id.clone()));
    let correlations = correlation_map(manifest, &known_ids)?;
    let maximum_observations = base.len() + options.len();
    if maximum_observations > MAX_OBSERVATIONS {
        return Err(format!("observation count exceeds {MAX_OBSERVATIONS}"));
    }

    let designs = evaluate_designs(&base, &options, &correlations, variable_count)?;
    let baseline_rows: Vec<Vec<i64>> = base.iter().map(|item| item.row.clone()).collect();
    let baseline_rank = exact_structural_rank_for_observation_rows(&baseline_rows, variable_count)?;
    let full_rank_count = designs.iter()
        .filter(|design| design["structurally_identifiable"].as_bool() == Some(true))
        .count();

    let mut cost_optimum = u64::MAX;
    let mut cost_optima: Vec<Vec<String>> = Vec::new();
    for design in designs.iter().filter(|design| design["structurally_identifiable"] == true) {
        let cost = design["synthetic_effort_cost"].as_u64()
            .ok_or_else(|| "design cost is missing".to_string())?;
        if cost < cost_optimum {
            cost_optimum = cost;
            cost_optima.clear();
        }
        if cost == cost_optimum {
            let ids = design["measurement_ids"].as_array().unwrap().iter()
                .map(|id| id.as_str().unwrap().to_string()).collect();
            cost_optima.push(ids);
        }
    }
    let d_optima = collect_optima(&designs, "log_determinant_fisher", true)?;
    let a_optima = collect_optima(&designs, "trace_gls_covariance", false)?;
    let e_optima = collect_optima(&designs, "minimum_fisher_eigenvalue", true)?;

    let option_count_u64 = u64::try_from(options.len())
        .map_err(|_| "measurement option count exceeds u64".to_string())?;
    let full_design = designs.iter().find(|design| {
        design["measurement_count"].as_u64() == Some(option_count_u64)
    }).ok_or_else(|| "full measurement design is missing".to_string())?;
    let full_design_metrics = json!({
        "measurement_ids": full_design["measurement_ids"],
        "minimum_fisher_eigenvalue": full_design["minimum_fisher_eigenvalue"],
        "maximum_fisher_eigenvalue": full_design["maximum_fisher_eigenvalue"],
        "fisher_condition_number": full_design["fisher_condition_number"],
        "log_determinant_fisher": full_design["log_determinant_fisher"],
        "trace_gls_covariance": full_design["trace_gls_covariance"],
        "gls_covariance_diagonal": full_design["gls_covariance_diagonal"]
    });

    let expected = manifest.get("expected")
        .ok_or_else(|| "expected object is required".to_string())?;
    let expected_baseline_rank = usize_at(expected, "baseline_rank")?;
    let expected_baseline_nullity = usize_at(expected, "baseline_nullity")?;
    let expected_design_count = usize_at(expected, "design_count")?;
    let expected_full_rank_count = usize_at(expected, "full_rank_design_count")?;
    let expected_cost = expected.get("minimum_structural_cost").and_then(Value::as_u64)
        .ok_or_else(|| "expected.minimum_structural_cost must be an integer".to_string())?;
    let expected_cost_ids = expected.get("minimum_structural_cost_designs")
        .ok_or_else(|| "expected minimum-cost designs are required".to_string())?;
    let expected_d_ids = expected.get("d_optimal_designs")
        .ok_or_else(|| "expected d_optimal_designs are required".to_string())?;
    let expected_a_ids = expected.get("a_optimal_designs")
        .ok_or_else(|| "expected a_optimal_designs are required".to_string())?;
    let expected_e_ids = expected.get("e_optimal_designs")
        .ok_or_else(|| "expected e_optimal_designs are required".to_string())?;
    let expected_metrics = expected.get("all_measurements_metrics")
        .ok_or_else(|| "expected all_measurements_metrics is required".to_string())?;

    let actual_cost_ids = json!(cost_optima);
    let actual_d_ids = json!(d_optima);
    let actual_a_ids = json!(a_optima);
    let actual_e_ids = json!(e_optima);
    let all_metrics_match = [
        ("minimum_fisher_eigenvalue", "minimum_fisher_eigenvalue"),
        ("maximum_fisher_eigenvalue", "maximum_fisher_eigenvalue"),
        ("fisher_condition_number", "fisher_condition_number"),
        ("log_determinant_fisher", "log_determinant_fisher"),
        ("trace_gls_covariance", "trace_gls_covariance"),
    ].iter().all(|(actual_key, expected_key)| {
        let actual = full_design_metrics[*actual_key].as_f64().unwrap_or(f64::NAN);
        let target = expected_metrics[*expected_key].as_f64().unwrap_or(f64::NAN);
        almost_equal(actual, target, 1e-9)
    });

    let checks = vec![
        json!({
            "id": "BASE-STRUCTURAL-RANK-001",
            "passed": baseline_rank == expected_baseline_rank
                && variable_count - baseline_rank == expected_baseline_nullity,
            "actual_rank": baseline_rank,
            "expected_rank": expected_baseline_rank,
            "actual_nullity": variable_count - baseline_rank,
            "expected_nullity": expected_baseline_nullity
        }),
        json!({
            "id": "ALL-SUBSET-ENUMERATION-001",
            "passed": designs.len() == expected_design_count,
            "actual_design_count": designs.len(),
            "expected_design_count": expected_design_count
        }),
        json!({
            "id": "FULL-RANK-DESIGN-COUNT-001",
            "passed": full_rank_count == expected_full_rank_count,
            "actual_full_rank_design_count": full_rank_count,
            "expected_full_rank_design_count": expected_full_rank_count
        }),
        json!({
            "id": "MINIMUM-STRUCTURAL-COST-DESIGN-001",
            "passed": cost_optimum == expected_cost && actual_cost_ids == *expected_cost_ids,
            "actual_cost": cost_optimum,
            "actual_designs": actual_cost_ids,
            "expected_cost": expected_cost,
            "expected_designs": expected_cost_ids
        }),
        json!({
            "id": "D-OPTIMAL-DESIGN-001",
            "passed": actual_d_ids == *expected_d_ids,
            "actual_designs": actual_d_ids,
            "expected_designs": expected_d_ids
        }),
        json!({
            "id": "A-OPTIMAL-DESIGN-001",
            "passed": actual_a_ids == *expected_a_ids,
            "actual_designs": actual_a_ids,
            "expected_designs": expected_a_ids
        }),
        json!({
            "id": "E-OPTIMAL-DESIGN-001",
            "passed": actual_e_ids == *expected_e_ids,
            "actual_designs": actual_e_ids,
            "expected_designs": expected_e_ids
        }),
        json!({
            "id": "FULL-DESIGN-INFORMATION-ORACLE-001",
            "passed": all_metrics_match,
            "actual_metrics": full_design_metrics,
            "expected_metrics": expected_metrics
        })
    ];
    let checks_passed = checks.iter()
        .filter(|check| check["passed"].as_bool() == Some(true)).count();
    let passed = checks_passed == checks.len();

    Ok(json!({
        "receipt_version": "0.1.0",
        "validator": "symtropy-tropical-forest-qualifier",
        "benchmark_id": benchmark_id,
        "corpus_id": corpus_id,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { "PASS_SYNTHETIC_PRACTICAL_IDENTIFIABILITY" } else { "FAIL" },
        "checks_passed": checks_passed,
        "checks_total": checks.len(),
        "checks": checks,
        "unknown_fluxes": variable_names,
        "baseline_structural_rank": baseline_rank,
        "baseline_structural_nullity": variable_count - baseline_rank,
        "design_count": designs.len(),
        "full_rank_design_count": full_rank_count,
        "minimum_structural_cost": cost_optimum,
        "minimum_structural_cost_designs": cost_optima,
        "d_optimal_designs": d_optima,
        "a_optimal_designs": a_optima,
        "e_optimal_designs": e_optima,
        "all_measurements_metrics": full_design_metrics,
        "designs": designs,
        "observation_noise_model": "SYNTHETIC_LINEAR_GAUSSIAN_KNOWN_COVARIANCE",
        "parameter_prior": "NONE",
        "estimator_covariance": "GENERALIZED_LEAST_SQUARES_LINEAR_MODEL",
        "field_measurement_error": "NOT_ESTABLISHED",
        "practical_identifiability_in_real_forest": "NOT_ESTABLISHED",
        "external_data_ingested": false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../cases/forest-flux-practical-identifiability-synthetic-v0.1.json");

    fn manifest() -> Value {
        serde_json::from_str(FIXTURE).expect("valid practical-identifiability fixture")
    }

    #[test]
    fn stock_only_flux_observations_remain_structurally_non_identifiable() {
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("valid corpus");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_PRACTICAL_IDENTIFIABILITY");
        assert_eq!(receipt["baseline_structural_rank"], 2);
        assert_eq!(receipt["baseline_structural_nullity"], 4);
    }

    #[test]
    fn structural_cost_optimum_differs_from_information_optimum() {
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("valid corpus");
        assert_eq!(receipt["minimum_structural_cost"], 14);
        assert_eq!(receipt["minimum_structural_cost_designs"][0].as_array().unwrap().len(), 4);
        assert_eq!(receipt["d_optimal_designs"][0].as_array().unwrap().len(), 6);
        assert_eq!(receipt["a_optimal_designs"][0].as_array().unwrap().len(), 6);
        assert_eq!(receipt["e_optimal_designs"][0].as_array().unwrap().len(), 6);
    }

    #[test]
    fn covariance_correlations_change_information_metrics() {
        let mut value = manifest();
        value["correlations"][0]["correlation"] = json!(0.0);
        let changed = serde_json::to_vec(&value).expect("serialize independent covariance");
        let independent = validate_corpus(&changed, &value).expect("valid independent covariance");
        let original = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("original covariance");
        assert_ne!(
            independent["all_measurements_metrics"]["trace_gls_covariance"],
            original["all_measurements_metrics"]["trace_gls_covariance"]
        );
    }

    #[test]
    fn zero_standard_deviation_is_rejected_not_treated_as_infinite_precision() {
        let mut value = manifest();
        value["base_observations"][0]["standard_deviation"] = json!(0.0);
        assert!(validate_corpus(FIXTURE.as_bytes(), &value).is_err());
    }

    #[test]
    fn unit_correlation_is_rejected_at_the_covariance_input_boundary() {
        let mut value = manifest();
        value["correlations"][0]["correlation"] = json!(1.0);
        let raw = serde_json::to_vec(&value).expect("serialize singular correlation");
        let result = validate_corpus(&raw, &value);
        assert!(result.is_err());
    }

    #[test]
    fn mutated_d_optimal_design_is_rejected() {
        let mut value = manifest();
        value["expected"]["d_optimal_designs"][0][0] = json!("growth");
        let raw = serde_json::to_vec(&value).expect("serialize design mutation");
        let receipt = validate_corpus(&raw, &value).expect("valid changed oracle returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn duplicate_correlation_pair_is_rejected() {
        let mut value = manifest();
        value["correlations"][1] = value["correlations"][0].clone();
        let raw = serde_json::to_vec(&value).expect("serialize duplicated correlation");
        assert!(validate_corpus(&raw, &value).is_err());
    }

    #[test]
    fn covariance_cannot_reference_unknown_observation_id() {
        let mut value = manifest();
        value["correlations"][0]["left_observation_id"] = json!("unlisted_sensor");
        let raw = serde_json::to_vec(&value).expect("serialize unknown correlation");
        assert!(validate_corpus(&raw, &value).is_err());
    }

    #[test]
    fn gls_covariance_remains_finite_and_reports_conditioning() {
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("valid corpus");
        let metrics = &receipt["all_measurements_metrics"];
        assert!(metrics["fisher_condition_number"].as_f64().unwrap().is_finite());
        assert_eq!(metrics["gls_covariance_diagonal"].as_array().unwrap().len(), 6);
    }
}
