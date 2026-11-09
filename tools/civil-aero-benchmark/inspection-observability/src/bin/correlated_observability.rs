// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Tier-0 synthetic correlated-observation observability oracle.
// This is not a structural solver, SHM detector, or aircraft safety tool.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;

const FIXTURE: &[u8] =
    include_bytes!("../../fixtures/correlated-observability-v0.1.json");
const RANK_REL_TOL: f64 = 1.0e-12;
const SYMMETRY_REL_TOL: f64 = 1.0e-12;
const CHOLESKY_REL_TOL: f64 = 1.0e-14;
const TIE_TOL: f64 = 1.0e-12;

#[derive(Clone, Debug, Deserialize)]
struct Parameter {
    id: String,
    scale: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct Sensor {
    id: String,
    observation_row: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct NoiseCovariance {
    channel_ids: Vec<String>,
    unit: String,
    matrix: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, Deserialize)]
struct ModelInput {
    schema_version: String,
    model_id: String,
    model_scope: String,
    covariance_id: String,
    covariance_provenance: String,
    parameters: Vec<Parameter>,
    sensors: Vec<Sensor>,
    noise_covariance: NoiseCovariance,
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
    diagonal_assumption_minimum_eigenvalue: f64,
    diagonal_assumption_log_determinant: Option<f64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Selection {
    sensor_ids: Vec<String>,
    objective_value: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Report {
    schema_version: String,
    model_id: String,
    source_sha256: String,
    covariance_id: String,
    covariance_provenance: String,
    covariance_unit: String,
    channel_ids: Vec<String>,
    candidate_sensor_count: usize,
    select_count: usize,
    subset_count: usize,
    full_rank_subset_count: usize,
    full_covariance_d_optimal_selection: Option<Selection>,
    full_covariance_e_optimal_selection: Option<Selection>,
    diagonal_assumption_d_optimal_selection: Option<Selection>,
    diagonal_assumption_e_optimal_selection: Option<Selection>,
    subsets: Vec<SubsetMetrics>,
    claim_ceiling: String,
}

fn sha256_hex(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

fn rank_of(rows: &[Vec<f64>]) -> usize {
    if rows.is_empty() {
        return 0;
    }
    let mut work = rows.to_vec();
    let max_abs = work
        .iter()
        .flatten()
        .fold(0.0_f64, |acc, value| acc.max(value.abs()));
    let tolerance = RANK_REL_TOL * max_abs.max(1.0);
    let mut pivot_row = 0usize;

    for column in 0..2 {
        if pivot_row >= work.len() {
            break;
        }
        let mut best_row = pivot_row;
        let mut best_value = 0.0_f64;
        for row in pivot_row..work.len() {
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
        for row in (pivot_row + 1)..work.len() {
            let factor = work[row][column] / pivot;
            for col in column..2 {
                work[row][col] -= factor * work[pivot_row][col];
            }
        }
        pivot_row += 1;
    }
    pivot_row
}

fn cholesky(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return Err("covariance must be non-empty and square".to_owned());
    }
    let max_diag = (0..n).fold(0.0_f64, |acc, i| acc.max(matrix[i][i].abs()));
    let tolerance = CHOLESKY_REL_TOL * max_diag.max(1.0);
    let mut lower = vec![vec![0.0; n]; n];

    for i in 0..n {
        for j in 0..=i {
            let mut value = matrix[i][j];
            for k in 0..j {
                value -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !value.is_finite() || value <= tolerance {
                    return Err("covariance is not positive definite at declared tolerance".to_owned());
                }
                lower[i][j] = value.sqrt();
            } else {
                if lower[j][j] <= 0.0 {
                    return Err("covariance Cholesky pivot is non-positive".to_owned());
                }
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    Ok(lower)
}

fn solve_cholesky(lower: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>, String> {
    let n = lower.len();
    if rhs.len() != n {
        return Err("covariance solve dimension mismatch".to_owned());
    }
    let mut forward = vec![0.0; n];
    for i in 0..n {
        let mut value = rhs[i];
        for j in 0..i {
            value -= lower[i][j] * forward[j];
        }
        forward[i] = value / lower[i][i];
    }
    let mut solution = vec![0.0; n];
    for i in (0..n).rev() {
        let mut value = forward[i];
        for j in (i + 1)..n {
            value -= lower[j][i] * solution[j];
        }
        solution[i] = value / lower[i][i];
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err("covariance solve produced non-finite values".to_owned());
    }
    Ok(solution)
}

fn symmetric_eigenvalues_2x2(matrix: &[Vec<f64>]) -> Result<[f64; 2], String> {
    if matrix.len() != 2 || matrix.iter().any(|row| row.len() != 2) {
        return Err("Fisher matrix must be 2x2".to_owned());
    }
    let a = matrix[0][0];
    let b = 0.5 * (matrix[0][1] + matrix[1][0]);
    let d = matrix[1][1];
    let discriminant = ((a - d).powi(2) + 4.0 * b.powi(2)).sqrt();
    let mut low = 0.5 * (a + d - discriminant);
    let high = 0.5 * (a + d + discriminant);
    if !low.is_finite() || !high.is_finite() {
        return Err("Fisher eigenvalue is non-finite".to_owned());
    }
    if low < -1.0e-10 * high.abs().max(1.0) {
        return Err("Fisher information is not positive semidefinite".to_owned());
    }
    if low < 0.0 {
        low = 0.0;
    }
    Ok([low, high])
}

fn information(
    rows: &[Vec<f64>],
    covariance: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, String> {
    if rows.is_empty() || rows.len() != covariance.len() {
        return Err("observation/covariance dimension mismatch".to_owned());
    }
    let lower = cholesky(covariance)?;
    let mut columns = [vec![0.0; rows.len()], vec![0.0; rows.len()]];
    for parameter in 0..2 {
        let rhs: Vec<f64> = rows.iter().map(|row| row[parameter]).collect();
        columns[parameter] = solve_cholesky(&lower, &rhs)?;
    }
    let mut fisher = vec![vec![0.0; 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            fisher[i][j] = rows
                .iter()
                .enumerate()
                .map(|(row, observation)| observation[i] * columns[j][row])
                .sum();
        }
    }
    if fisher.iter().flatten().any(|value| !value.is_finite()) {
        return Err("Fisher information contains non-finite values".to_owned());
    }
    Ok(fisher)
}

fn log_determinant(matrix: &[Vec<f64>], rank: usize) -> Option<f64> {
    if rank != 2 {
        return None;
    }
    let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    if determinant.is_finite() && determinant > 0.0 {
        Some(determinant.ln())
    } else {
        None
    }
}

fn validate(input: &ModelInput) -> Result<(), String> {
    if input.schema_version != "aero-inspection-correlated-observability/0.1.0" {
        return Err("unsupported schema_version".to_owned());
    }
    if input.model_id.trim().is_empty() || input.covariance_id.trim().is_empty() {
        return Err("model_id and covariance_id must be non-empty".to_owned());
    }
    if input.model_scope != "synthetic_local_linear_observation" {
        return Err("only synthetic local linear observations are admitted".to_owned());
    }
    if input.covariance_provenance != "synthetic_fixture_known_answer" {
        return Err("v0.1 only admits the declared synthetic covariance provenance".to_owned());
    }
    if input.parameters.len() != 2 {
        return Err("v0.1 requires exactly two parameters".to_owned());
    }
    if input.sensors.len() < 2 || input.sensors.len() > 16 {
        return Err("candidate sensor count must be in 2..=16".to_owned());
    }
    if input.select_count < 2 || input.select_count > input.sensors.len() {
        return Err("select_count must be in 2..=candidate sensor count".to_owned());
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

    let mut sensor_ids = Vec::with_capacity(input.sensors.len());
    for sensor in &input.sensors {
        if sensor.id.trim().is_empty() {
            return Err("sensor IDs must be non-empty".to_owned());
        }
        if sensor.observation_row.len() != 2
            || sensor.observation_row.iter().any(|value| !value.is_finite())
        {
            return Err(format!("sensor {} must have two finite sensitivities", sensor.id));
        }
        sensor_ids.push(sensor.id.clone());
    }
    let mut sorted_ids = sensor_ids.clone();
    sorted_ids.sort();
    sorted_ids.dedup();
    if sorted_ids.len() != sensor_ids.len() {
        return Err("sensor IDs must be unique".to_owned());
    }
    if sensor_ids != sorted_ids {
        return Err("sensor records must be in canonical ascending channel-ID order".to_owned());
    }

    let covariance = &input.noise_covariance;
    if covariance.unit.trim().is_empty() {
        return Err("covariance unit must be non-empty".to_owned());
    }
    if covariance.channel_ids != sensor_ids {
        return Err("covariance channel_ids must exactly match sensor identity and matrix order".to_owned());
    }
    let n = sensor_ids.len();
    if covariance.matrix.len() != n || covariance.matrix.iter().any(|row| row.len() != n) {
        return Err("covariance dimensions must exactly match declared channel IDs".to_owned());
    }
    for row in &covariance.matrix {
        if row.iter().any(|value| !value.is_finite()) {
            return Err("covariance contains non-finite values".to_owned());
        }
    }
    let max_abs = covariance
        .matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |acc, value| acc.max(value.abs()));
    let symmetry_tolerance = SYMMETRY_REL_TOL * max_abs.max(1.0);
    for i in 0..n {
        for j in 0..n {
            if (covariance.matrix[i][j] - covariance.matrix[j][i]).abs()
                > symmetry_tolerance
            {
                return Err("covariance matrix is not symmetric within declared tolerance".to_owned());
            }
        }
    }
    cholesky(&covariance.matrix)?;
    Ok(())
}

fn combinations(
    start: usize,
    remaining: usize,
    count: usize,
    current: &mut Vec<usize>,
    output: &mut Vec<Vec<usize>>,
) {
    if remaining == 0 {
        output.push(current.clone());
        return;
    }
    if count.saturating_sub(start) < remaining {
        return;
    }
    for index in start..=(count - remaining) {
        current.push(index);
        combinations(index + 1, remaining - 1, count, current, output);
        current.pop();
    }
}

fn selection_better(candidate: &Selection, previous: &Selection) -> bool {
    candidate.objective_value > previous.objective_value + TIE_TOL
        || ((candidate.objective_value - previous.objective_value).abs() <= TIE_TOL
            && candidate.sensor_ids < previous.sensor_ids)
}

fn evaluate(input: &ModelInput, source_sha256: &str) -> Result<Report, String> {
    validate(input)?;
    let scales = [input.parameters[0].scale, input.parameters[1].scale];
    let sensors = &input.sensors;
    let mut index_sets = Vec::new();
    combinations(
        0,
        input.select_count,
        sensors.len(),
        &mut Vec::new(),
        &mut index_sets,
    );

    let mut subsets = Vec::with_capacity(index_sets.len());
    for indices in index_sets {
        let sensor_ids: Vec<String> = indices.iter().map(|index| sensors[*index].id.clone()).collect();
        let rows: Vec<Vec<f64>> = indices
            .iter()
            .map(|index| {
                vec![
                    sensors[*index].observation_row[0] * scales[0],
                    sensors[*index].observation_row[1] * scales[1],
                ]
            })
            .collect();
        if rows.iter().flatten().any(|value| !value.is_finite()) {
            return Err("scaled observation row overflowed".to_owned());
        }

        let covariance: Vec<Vec<f64>> = indices
            .iter()
            .map(|i| {
                indices
                    .iter()
                    .map(|j| input.noise_covariance.matrix[*i][*j])
                    .collect()
            })
            .collect();
        let diagonal_covariance: Vec<Vec<f64>> = (0..indices.len())
            .map(|i| {
                (0..indices.len())
                    .map(|j| if i == j { covariance[i][i] } else { 0.0 })
                    .collect()
            })
            .collect();

        let rank = rank_of(&rows);
        let fisher = information(&rows, &covariance)?;
        let diagonal_fisher = information(&rows, &diagonal_covariance)?;
        let eigenvalues = symmetric_eigenvalues_2x2(&fisher)?;
        let diagonal_eigenvalues = symmetric_eigenvalues_2x2(&diagonal_fisher)?;
        subsets.push(SubsetMetrics {
            sensor_ids,
            rank,
            nullity: 2 - rank,
            fisher_information: fisher.clone(),
            eigenvalues_ascending: eigenvalues.to_vec(),
            minimum_eigenvalue: eigenvalues[0],
            log_determinant: log_determinant(&fisher, rank),
            diagonal_assumption_minimum_eigenvalue: diagonal_eigenvalues[0],
            diagonal_assumption_log_determinant: log_determinant(&diagonal_fisher, rank),
        });
    }

    let mut full_d: Option<Selection> = None;
    let mut full_e: Option<Selection> = None;
    let mut diagonal_d: Option<Selection> = None;
    let mut diagonal_e: Option<Selection> = None;
    for subset in &subsets {
        if let Some(value) = subset.log_determinant {
            let candidate = Selection {
                sensor_ids: subset.sensor_ids.clone(),
                objective_value: value,
            };
            if full_d.as_ref().is_none_or(|previous| selection_better(&candidate, previous)) {
                full_d = Some(candidate);
            }
        }
        if subset.rank == 2 {
            let candidate = Selection {
                sensor_ids: subset.sensor_ids.clone(),
                objective_value: subset.minimum_eigenvalue,
            };
            if full_e.as_ref().is_none_or(|previous| selection_better(&candidate, previous)) {
                full_e = Some(candidate);
            }
        }
        if let Some(value) = subset.diagonal_assumption_log_determinant {
            let candidate = Selection {
                sensor_ids: subset.sensor_ids.clone(),
                objective_value: value,
            };
            if diagonal_d
                .as_ref()
                .is_none_or(|previous| selection_better(&candidate, previous))
            {
                diagonal_d = Some(candidate);
            }
        }
        if subset.rank == 2 {
            let candidate = Selection {
                sensor_ids: subset.sensor_ids.clone(),
                objective_value: subset.diagonal_assumption_minimum_eigenvalue,
            };
            if diagonal_e
                .as_ref()
                .is_none_or(|previous| selection_better(&candidate, previous))
            {
                diagonal_e = Some(candidate);
            }
        }
    }

    let full_rank_subset_count = subsets.iter().filter(|subset| subset.rank == 2).count();
    Ok(Report {
        schema_version: input.schema_version.clone(),
        model_id: input.model_id.clone(),
        source_sha256: source_sha256.to_owned(),
        covariance_id: input.covariance_id.clone(),
        covariance_provenance: input.covariance_provenance.clone(),
        covariance_unit: input.noise_covariance.unit.clone(),
        channel_ids: input.noise_covariance.channel_ids.clone(),
        candidate_sensor_count: sensors.len(),
        select_count: input.select_count,
        subset_count: subsets.len(),
        full_rank_subset_count,
        full_covariance_d_optimal_selection: full_d,
        full_covariance_e_optimal_selection: full_e,
        diagonal_assumption_d_optimal_selection: diagonal_d,
        diagonal_assumption_e_optimal_selection: diagonal_e,
        subsets,
        claim_ceiling: "synthetic_full_covariance_local_observability_only".to_owned(),
    })
}

fn evaluate_bytes(bytes: &[u8]) -> Result<Report, String> {
    let input: ModelInput = serde_json::from_slice(bytes)
        .map_err(|error| format!("input JSON parse failed: {error}"))?;
    evaluate(&input, &sha256_hex(bytes))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bytes = fs::read(manifest.join("fixtures/correlated-observability-v0.1.json"))?;
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
    fn full_covariance_changes_the_known_answer() {
        let result = evaluate_bytes(FIXTURE).expect("fixture must evaluate");
        assert_eq!(result.subset_count, 15);
        assert_eq!(result.full_rank_subset_count, 15);
        let d = result.full_covariance_d_optimal_selection.unwrap();
        assert_eq!(d.sensor_ids, vec!["S_A", "S_C", "S_E", "S_F"]);
        assert!((d.objective_value - 4.7011019227948365).abs() < 1.0e-9);
        let e = result.full_covariance_e_optimal_selection.unwrap();
        assert_eq!(e.sensor_ids, vec!["S_A", "S_B", "S_E", "S_F"]);
        assert!((e.objective_value - 8.542911483435994).abs() < 1.0e-9);
    }

    #[test]
    fn diagonal_independence_assumption_changes_the_placement() {
        let result = evaluate_bytes(FIXTURE).expect("fixture must evaluate");
        let d = result.diagonal_assumption_d_optimal_selection.unwrap();
        assert_eq!(d.sensor_ids, vec!["S_A", "S_B", "S_C", "S_D"]);
        let e = result.diagonal_assumption_e_optimal_selection.unwrap();
        assert_eq!(e.sensor_ids, vec!["S_A", "S_B", "S_C", "S_D"]);
        assert_ne!(
            d.sensor_ids,
            result.full_covariance_d_optimal_selection.unwrap().sensor_ids
        );
        assert_ne!(
            e.sensor_ids,
            result.full_covariance_e_optimal_selection.unwrap().sensor_ids
        );
    }

    #[test]
    fn covariance_identity_order_must_match_sensor_order() {
        let mut value = input();
        value.noise_covariance.channel_ids.swap(0, 1);
        assert!(validate(&value).is_err());
    }

    #[test]
    fn non_symmetric_or_non_positive_definite_covariance_rejects() {
        let mut value = input();
        value.noise_covariance.matrix[0][1] += 0.01;
        assert!(validate(&value).is_err());
        let mut value = input();
        value.noise_covariance.matrix[0][1] = 0.2;
        value.noise_covariance.matrix[1][0] = 0.2;
        assert!(validate(&value).is_err());
    }

    #[test]
    fn rank_deficiency_never_yields_a_full_rank_selection() {
        let mut value = input();
        for sensor in &mut value.sensors {
            sensor.observation_row = vec![1.0, 0.0];
        }
        let result = evaluate(&value, &sha256_hex(b"rank-deficient")).unwrap();
        assert_eq!(result.full_rank_subset_count, 0);
        assert!(result.full_covariance_d_optimal_selection.is_none());
        assert!(result.full_covariance_e_optimal_selection.is_some());
        assert_eq!(
            result.full_covariance_e_optimal_selection.unwrap().objective_value,
            0.0
        );
    }
}
