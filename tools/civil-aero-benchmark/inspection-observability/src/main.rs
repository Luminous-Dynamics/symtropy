// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Tier-0 synthetic linear observability oracle.
// This is not a structural solver, SHM detector, or aircraft safety tool.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;

const FIXTURE: &[u8] = include_bytes!("../fixtures/known-answer-observability-v0.1.json");
const RANK_REL_TOL: f64 = 1.0e-12;
const EIGEN_REL_TOL: f64 = 1.0e-12;
const CHOLESKY_REL_TOL: f64 = 1.0e-14;

#[derive(Clone, Debug, Deserialize)]
struct Parameter {
    id: String,
    scale: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct Sensor {
    id: String,
    observation_row: Vec<f64>,
    variance: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct ModelInput {
    schema_version: String,
    model_id: String,
    model_scope: String,
    noise_model: String,
    parameters: Vec<Parameter>,
    sensors: Vec<Sensor>,
    select_count: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct SubsetMetrics {
    sensor_ids: Vec<String>,
    rank: usize,
    nullity: usize,
    fisher_information: Vec<Vec<f64>>,
    eigenvalues_ascending: Vec<f64>,
    minimum_eigenvalue: f64,
    log_determinant: Option<f64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Selection {
    sensor_ids: Vec<String>,
    rank: usize,
    nullity: usize,
    objective_value: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    model_id: String,
    model_scope: String,
    noise_model: String,
    source_sha256: String,
    parameter_ids: Vec<String>,
    parameter_scales: Vec<f64>,
    candidate_sensor_count: usize,
    select_count: usize,
    subset_count: usize,
    full_rank_subset_count: usize,
    d_optimal_selection: Option<Selection>,
    e_optimal_selection: Option<Selection>,
    subsets: Vec<SubsetMetrics>,
    rank_relative_tolerance: f64,
    eigen_relative_tolerance: f64,
    claim_ceiling: String,
}

fn sha256_hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

fn validate_input(input: &ModelInput) -> Result<(), String> {
    if input.schema_version != "aero-inspection-observability/0.1.0" {
        return Err("unsupported schema_version".to_owned());
    }
    if input.model_id.trim().is_empty() {
        return Err("model_id must be non-empty".to_owned());
    }
    if input.model_scope != "synthetic_local_linear_observation" {
        return Err("v0.1 only admits synthetic local linear observations".to_owned());
    }
    if input.noise_model != "independent_variance_per_sensor" {
        return Err("v0.1 only admits independent sensor variances".to_owned());
    }
    let parameter_count = input.parameters.len();
    if parameter_count == 0 || parameter_count > 8 {
        return Err("parameter count must be in 1..=8".to_owned());
    }
    if input.sensors.is_empty() || input.sensors.len() > 16 {
        return Err("candidate sensor count must be in 1..=16".to_owned());
    }
    if input.select_count == 0 || input.select_count > input.sensors.len() {
        return Err("select_count must be in 1..=candidate sensor count".to_owned());
    }

    let mut parameter_ids = BTreeSet::new();
    for parameter in &input.parameters {
        if parameter.id.trim().is_empty() || !parameter_ids.insert(parameter.id.as_str()) {
            return Err("parameter IDs must be non-empty and unique".to_owned());
        }
        if !parameter.scale.is_finite() || parameter.scale <= 0.0 {
            return Err(format!("parameter {} has invalid scale", parameter.id));
        }
    }

    let mut sensor_ids = BTreeSet::new();
    for sensor in &input.sensors {
        if sensor.id.trim().is_empty() || !sensor_ids.insert(sensor.id.as_str()) {
            return Err("sensor IDs must be non-empty and unique".to_owned());
        }
        if sensor.observation_row.len() != parameter_count {
            return Err(format!(
                "sensor {} observation row has wrong dimension",
                sensor.id
            ));
        }
        if !sensor.variance.is_finite() || sensor.variance <= 0.0 {
            return Err(format!(
                "sensor {} variance must be finite and positive",
                sensor.id
            ));
        }
        if sensor
            .observation_row
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err(format!(
                "sensor {} row contains non-finite values",
                sensor.id
            ));
        }
    }
    Ok(())
}

fn rank_of(matrix: &[Vec<f64>]) -> usize {
    if matrix.is_empty() || matrix[0].is_empty() {
        return 0;
    }
    let mut work = matrix.to_vec();
    let rows = work.len();
    let columns = work[0].len();
    let max_abs = work
        .iter()
        .flatten()
        .fold(0.0_f64, |acc, x| acc.max(x.abs()));
    let tolerance = RANK_REL_TOL * max_abs.max(1.0);
    let mut pivot_row = 0usize;

    for column in 0..columns {
        if pivot_row >= rows {
            break;
        }
        let mut best_row = pivot_row;
        let mut best_value = 0.0_f64;
        for row in pivot_row..rows {
            let value = work[row][column].abs();
            if value > best_value {
                best_value = value;
                best_row = row;
            }
        }
        if best_value <= tolerance {
            continue;
        }
        work.swap(pivot_row, best_row);
        let pivot = work[pivot_row][column];
        for row in (pivot_row + 1)..rows {
            let factor = work[row][column] / pivot;
            for col in column..columns {
                work[row][col] -= factor * work[pivot_row][col];
            }
        }
        pivot_row += 1;
    }
    pivot_row
}

fn fisher_information(rows: &[Vec<f64>], variances: &[f64], dimension: usize) -> Vec<Vec<f64>> {
    let mut fisher = vec![vec![0.0; dimension]; dimension];
    for (row, variance) in rows.iter().zip(variances) {
        let weight = 1.0 / variance;
        for i in 0..dimension {
            for j in 0..dimension {
                fisher[i][j] += row[i] * row[j] * weight;
            }
        }
    }
    fisher
}

fn symmetric_eigenvalues(matrix: &[Vec<f64>]) -> Result<Vec<f64>, String> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return Err("eigenvalue input must be a non-empty square matrix".to_owned());
    }
    let mut a = matrix.to_vec();
    let max_diag = (0..n).fold(0.0_f64, |acc, i| acc.max(a[i][i].abs()));
    let tolerance = EIGEN_REL_TOL * max_diag.max(1.0);
    let max_iterations = 100usize.saturating_mul(n).saturating_mul(n).max(1);
    let mut converged = n == 1;

    for _ in 0..max_iterations {
        let mut p = 0usize;
        let mut q = 0usize;
        let mut largest = 0.0_f64;
        for i in 0..n {
            for j in (i + 1)..n {
                if a[i][j].abs() > largest {
                    largest = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if largest <= tolerance {
            converged = true;
            break;
        }

        let apq = a[p][q];
        if apq == 0.0 {
            continue;
        }
        let tau = (a[q][q] - a[p][p]) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        let app = a[p][p];
        let aqq = a[q][q];
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;

        for k in 0..n {
            if k == p || k == q {
                continue;
            }
            let akp = a[k][p];
            let akq = a[k][q];
            a[k][p] = c * akp - s * akq;
            a[p][k] = a[k][p];
            a[k][q] = s * akp + c * akq;
            a[q][k] = a[k][q];
        }
    }
    if !converged {
        return Err("Jacobi eigenvalue iteration did not converge".to_owned());
    }

    let mut values: Vec<f64> = (0..n).map(|i| a[i][i]).collect();
    values.sort_by(f64::total_cmp);
    if values.iter().any(|value| !value.is_finite()) {
        return Err("non-finite Fisher eigenvalue".to_owned());
    }
    let scale = values.iter().fold(1.0_f64, |acc, x| acc.max(x.abs()));
    if values[0] < -1.0e-10 * scale {
        return Err("Fisher information is not positive semidefinite".to_owned());
    }
    if values[0] < 0.0 {
        values[0] = 0.0;
    }
    Ok(values)
}

fn log_determinant_spd(matrix: &[Vec<f64>]) -> Option<f64> {
    let n = matrix.len();
    let max_diag = (0..n).fold(0.0_f64, |acc, i| acc.max(matrix[i][i].abs()));
    let tolerance = CHOLESKY_REL_TOL * max_diag.max(1.0);
    let mut lower = vec![vec![0.0; n]; n];
    let mut logdet = 0.0_f64;

    for i in 0..n {
        for j in 0..=i {
            let mut value = matrix[i][j];
            for k in 0..j {
                value -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !value.is_finite() || value <= tolerance {
                    return None;
                }
                lower[i][j] = value.sqrt();
                logdet += 2.0 * lower[i][j].ln();
            } else {
                if lower[j][j] <= 0.0 {
                    return None;
                }
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    if logdet.is_finite() {
        Some(logdet)
    } else {
        None
    }
}

fn combinations(
    start: usize,
    remaining: usize,
    count: usize,
    current: &mut Vec<usize>,
    out: &mut Vec<Vec<usize>>,
) {
    if remaining == 0 {
        out.push(current.clone());
        return;
    }
    if count.saturating_sub(start) < remaining {
        return;
    }
    for index in start..=(count - remaining) {
        current.push(index);
        combinations(index + 1, remaining - 1, count, current, out);
        current.pop();
    }
}

fn best_selection(subsets: &[SubsetMetrics], use_logdet: bool) -> Option<Selection> {
    let mut best: Option<Selection> = None;
    for subset in subsets {
        let objective = if use_logdet {
            if subset.nullity != 0 {
                continue;
            }
            match subset.log_determinant {
                Some(value) => value,
                None => continue,
            }
        } else {
            subset.minimum_eigenvalue
        };
        let candidate = Selection {
            sensor_ids: subset.sensor_ids.clone(),
            rank: subset.rank,
            nullity: subset.nullity,
            objective_value: objective,
        };
        let replace = match &best {
            None => true,
            Some(previous) => {
                if candidate.objective_value > previous.objective_value + 1.0e-12 {
                    true
                } else if (candidate.objective_value - previous.objective_value).abs() <= 1.0e-12 {
                    candidate.sensor_ids < previous.sensor_ids
                } else {
                    false
                }
            }
        };
        if replace {
            best = Some(candidate);
        }
    }
    best
}

fn evaluate(input: &ModelInput, source_sha256: &str) -> Result<Report, String> {
    validate_input(input)?;
    let dimensions = input.parameters.len();
    let scales: Vec<f64> = input.parameters.iter().map(|p| p.scale).collect();
    let mut sensors = input.sensors.clone();
    sensors.sort_by(|a, b| a.id.cmp(&b.id));

    let mut index_sets = Vec::new();
    combinations(
        0,
        input.select_count,
        sensors.len(),
        &mut Vec::new(),
        &mut index_sets,
    );
    if index_sets.is_empty() {
        return Err("no sensor subsets were enumerated".to_owned());
    }

    let mut subsets = Vec::with_capacity(index_sets.len());
    for indices in index_sets {
        let mut rows = Vec::with_capacity(indices.len());
        let mut variances = Vec::with_capacity(indices.len());
        let mut sensor_ids = Vec::with_capacity(indices.len());
        for index in indices {
            let sensor = &sensors[index];
            let row: Vec<f64> = sensor
                .observation_row
                .iter()
                .zip(&scales)
                .map(|(value, scale)| value * scale)
                .collect();
            if row.iter().any(|value| !value.is_finite()) {
                return Err(format!("sensor {} normalized row overflowed", sensor.id));
            }
            rows.push(row);
            variances.push(sensor.variance);
            sensor_ids.push(sensor.id.clone());
        }

        let rank = rank_of(&rows);
        let fisher = fisher_information(&rows, &variances, dimensions);
        if fisher.iter().flatten().any(|value| !value.is_finite()) {
            return Err("Fisher information overflowed".to_owned());
        }
        let eigenvalues = symmetric_eigenvalues(&fisher)?;
        let minimum_eigenvalue = eigenvalues[0];
        let log_determinant = if rank == dimensions {
            log_determinant_spd(&fisher)
        } else {
            None
        };
        subsets.push(SubsetMetrics {
            sensor_ids,
            rank,
            nullity: dimensions - rank,
            fisher_information: fisher,
            eigenvalues_ascending: eigenvalues,
            minimum_eigenvalue,
            log_determinant,
        });
    }

    let full_rank_subset_count = subsets.iter().filter(|s| s.nullity == 0).count();
    Ok(Report {
        schema_version: input.schema_version.clone(),
        model_id: input.model_id.clone(),
        model_scope: input.model_scope.clone(),
        noise_model: input.noise_model.clone(),
        source_sha256: source_sha256.to_owned(),
        parameter_ids: input.parameters.iter().map(|p| p.id.clone()).collect(),
        parameter_scales: scales,
        candidate_sensor_count: sensors.len(),
        select_count: input.select_count,
        subset_count: subsets.len(),
        full_rank_subset_count,
        d_optimal_selection: best_selection(&subsets, true),
        e_optimal_selection: best_selection(&subsets, false),
        subsets,
        rank_relative_tolerance: RANK_REL_TOL,
        eigen_relative_tolerance: EIGEN_REL_TOL,
        claim_ceiling: "synthetic_local_observability_only".to_owned(),
    })
}

fn evaluate_bytes(bytes: &[u8]) -> Result<Report, String> {
    let input: ModelInput = serde_json::from_slice(bytes)
        .map_err(|error| format!("input JSON parse failed: {error}"))?;
    evaluate(&input, &sha256_hex(bytes))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bytes = fs::read(manifest.join("fixtures/known-answer-observability-v0.1.json"))?;
    let report = evaluate_bytes(&bytes).map_err(std::io::Error::other)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> ModelInput {
        serde_json::from_slice(FIXTURE).expect("fixture must parse")
    }

    #[test]
    fn known_answer_exhaustively_selects_a_and_c() {
        let result = evaluate_bytes(FIXTURE).expect("fixture must evaluate");
        assert_eq!(result.subset_count, 6);
        assert_eq!(result.full_rank_subset_count, 6);
        assert_eq!(
            result.d_optimal_selection.unwrap().sensor_ids,
            vec!["S_A", "S_C"]
        );
        assert_eq!(
            result.e_optimal_selection.unwrap().sensor_ids,
            vec!["S_A", "S_C"]
        );
        let ac = result
            .subsets
            .iter()
            .find(|s| s.sensor_ids == vec!["S_A", "S_C"])
            .unwrap();
        assert_eq!(ac.fisher_information, vec![vec![2.0, 1.0], vec![1.0, 1.0]]);
        assert!((ac.minimum_eigenvalue - (3.0 - 5.0_f64.sqrt()) / 2.0).abs() < 1.0e-12);
        assert!((ac.log_determinant.unwrap()).abs() < 1.0e-12);
    }

    #[test]
    fn duplicate_sensor_and_parameter_ids_reject() {
        let mut value = input();
        value.sensors[1].id = value.sensors[0].id.clone();
        assert!(validate_input(&value).is_err());
        let mut value = input();
        value.parameters[1].id = value.parameters[0].id.clone();
        assert!(validate_input(&value).is_err());
    }

    #[test]
    fn zero_or_nonfinite_variance_rejects() {
        let mut value = input();
        value.sensors[0].variance = 0.0;
        assert!(validate_input(&value).is_err());
        value.sensors[0].variance = f64::NAN;
        assert!(validate_input(&value).is_err());
    }

    #[test]
    fn dimension_mismatch_rejects() {
        let mut value = input();
        value.sensors[0].observation_row.pop();
        assert!(validate_input(&value).is_err());
    }

    #[test]
    fn rank_deficiency_remains_visible() {
        let mut value = input();
        for sensor in &mut value.sensors {
            sensor.observation_row = vec![1.0, 0.0];
        }
        let result = evaluate(&value, &sha256_hex(b"rank-deficient")).unwrap();
        assert_eq!(result.full_rank_subset_count, 0);
        assert!(result.d_optimal_selection.is_none());
        let e = result.e_optimal_selection.unwrap();
        assert_eq!(e.rank, 1);
        assert_eq!(e.nullity, 1);
        assert_eq!(e.objective_value, 0.0);
    }

    #[test]
    fn sensor_input_order_does_not_change_selection_or_output() {
        let source_digest = sha256_hex(FIXTURE);
        let original = evaluate(&input(), &source_digest).unwrap();
        let mut value = input();
        value.sensors.reverse();
        let reversed = evaluate(&value, &source_digest).unwrap();
        assert_eq!(original, reversed);
    }
}
