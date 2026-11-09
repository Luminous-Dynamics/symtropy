// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Tier-0 synthetic correlated and fault-tolerant observability oracle.
// This is not a structural solver, SHM detector, or aircraft safety tool.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;

const FIXTURE: &[u8] = include_bytes!("../../fixtures/correlated-observability-v0.1.json");
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
struct FailureGroup {
    id: String,
    sensor_ids: Vec<String>,
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
    common_cause_groups: Vec<FailureGroup>,
    select_count: usize,
    maximum_simultaneous_failure_domains: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct ScenarioMetrics {
    scenario_id: String,
    failed_sensor_ids: Vec<String>,
    retained_sensor_ids: Vec<String>,
    rank: usize,
    fisher_information: Vec<Vec<f64>>,
    minimum_eigenvalue: f64,
    log_determinant: Option<f64>,
    diagonal_assumption_minimum_eigenvalue: f64,
    diagonal_assumption_log_determinant: Option<f64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct SubsetMetrics {
    sensor_ids: Vec<String>,
    nominal_rank: usize,
    nominal_minimum_eigenvalue: f64,
    nominal_log_determinant: Option<f64>,
    diagonal_assumption_minimum_eigenvalue: f64,
    diagonal_assumption_log_determinant: Option<f64>,
    robustly_feasible: bool,
    worst_case_minimum_eigenvalue: Option<f64>,
    worst_case_log_determinant: Option<f64>,
    worst_case_diagonal_assumption_minimum_eigenvalue: Option<f64>,
    worst_case_diagonal_assumption_log_determinant: Option<f64>,
    limiting_scenario_id: String,
    scenario_metrics: Vec<ScenarioMetrics>,
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
    failure_scenario_count: usize,
    subset_count: usize,
    robustly_feasible_subset_count: usize,
    nominal_full_covariance_d_optimal_selection: Option<Selection>,
    nominal_full_covariance_e_optimal_selection: Option<Selection>,
    nominal_diagonal_assumption_d_optimal_selection: Option<Selection>,
    nominal_diagonal_assumption_e_optimal_selection: Option<Selection>,
    robust_full_covariance_d_optimal_selection: Option<Selection>,
    robust_full_covariance_e_optimal_selection: Option<Selection>,
    robust_diagonal_assumption_d_optimal_selection: Option<Selection>,
    robust_diagonal_assumption_e_optimal_selection: Option<Selection>,
    subsets: Vec<SubsetMetrics>,
    claim_ceiling: String,
}

#[derive(Clone, Debug)]
struct FailureScenario {
    id: String,
    failed_sensor_ids: BTreeSet<String>,
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
    if rows.is_empty() {
        return Ok(vec![vec![0.0, 0.0], vec![0.0, 0.0]]);
    }
    if rows.len() != covariance.len() {
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
    if input.common_cause_groups.len() > 32 {
        return Err("common-cause group count exceeds 32".to_owned());
    }
    if input.maximum_simultaneous_failure_domains != 1 {
        return Err("v0.1 admits exactly one failed sensor or one common-cause group".to_owned());
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

    let mut group_ids = BTreeSet::new();
    for group in &input.common_cause_groups {
        if group.id.trim().is_empty() || !group_ids.insert(group.id.as_str()) {
            return Err("common-cause group IDs must be non-empty and unique".to_owned());
        }
        if group.sensor_ids.len() < 2 {
            return Err(format!(
                "common-cause group {} must contain at least two sensors",
                group.id
            ));
        }
        let mut members = BTreeSet::new();
        for sensor_id in &group.sensor_ids {
            if !sensor_ids.iter().any(|known| known == sensor_id) {
                return Err(format!(
                    "group {} references unknown sensor {}",
                    group.id, sensor_id
                ));
            }
            if !members.insert(sensor_id.as_str()) {
                return Err(format!(
                    "group {} contains duplicate sensor {}",
                    group.id, sensor_id
                ));
            }
        }
    }
    Ok(())
}

fn failure_scenarios(input: &ModelInput) -> Vec<FailureScenario> {
    let mut sensors: Vec<String> = input.sensors.iter().map(|sensor| sensor.id.clone()).collect();
    sensors.sort();
    let mut groups = input.common_cause_groups.clone();
    groups.sort_by(|a, b| a.id.cmp(&b.id));
    let mut result = vec![FailureScenario {
        id: "none".to_owned(),
        failed_sensor_ids: BTreeSet::new(),
    }];
    for sensor_id in sensors {
        result.push(FailureScenario {
            id: format!("single_sensor:{sensor_id}"),
            failed_sensor_ids: BTreeSet::from([sensor_id]),
        });
    }
    for group in groups {
        result.push(FailureScenario {
            id: format!("common_cause:{}", group.id),
            failed_sensor_ids: group.sensor_ids.into_iter().collect(),
        });
    }
    result
}

fn scenario_metrics(
    input: &ModelInput,
    chosen_indices: &[usize],
    scenario: &FailureScenario,
    scales: &[f64; 2],
) -> Result<ScenarioMetrics, String> {
    let retained_indices: Vec<usize> = chosen_indices
        .iter()
        .copied()
        .filter(|index| !scenario.failed_sensor_ids.contains(&input.sensors[*index].id))
        .collect();
    let retained_sensor_ids: Vec<String> = retained_indices
        .iter()
        .map(|index| input.sensors[*index].id.clone())
        .collect();
    let rows: Vec<Vec<f64>> = retained_indices
        .iter()
        .map(|index| {
            vec![
                input.sensors[*index].observation_row[0] * scales[0],
                input.sensors[*index].observation_row[1] * scales[1],
            ]
        })
        .collect();
    if rows.iter().flatten().any(|value| !value.is_finite()) {
        return Err("scaled observation row overflowed".to_owned());
    }
    let covariance: Vec<Vec<f64>> = retained_indices
        .iter()
        .map(|i| {
            retained_indices
                .iter()
                .map(|j| input.noise_covariance.matrix[*i][*j])
                .collect()
        })
        .collect();
    let diagonal_covariance: Vec<Vec<f64>> = (0..retained_indices.len())
        .map(|i| {
            (0..retained_indices.len())
                .map(|j| if i == j { covariance[i][i] } else { 0.0 })
                .collect()
        })
        .collect();

    let rank = rank_of(&rows);
    let fisher = information(&rows, &covariance)?;
    let diagonal_fisher = information(&rows, &diagonal_covariance)?;
    let eigenvalues = symmetric_eigenvalues_2x2(&fisher)?;
    let diagonal_eigenvalues = symmetric_eigenvalues_2x2(&diagonal_fisher)?;
    let mut failed_sensor_ids: Vec<String> = scenario.failed_sensor_ids.iter().cloned().collect();
    failed_sensor_ids.sort();

    Ok(ScenarioMetrics {
        scenario_id: scenario.id.clone(),
        failed_sensor_ids,
        retained_sensor_ids,
        rank,
        fisher_information: fisher.clone(),
        minimum_eigenvalue: eigenvalues[0],
        log_determinant: log_determinant(&fisher, rank),
        diagonal_assumption_minimum_eigenvalue: diagonal_eigenvalues[0],
        diagonal_assumption_log_determinant: log_determinant(&diagonal_fisher, rank),
    })
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

fn select_best(
    subsets: &[SubsetMetrics],
    robust: bool,
    objective_is_d: bool,
    diagonal: bool,
) -> Option<Selection> {
    let mut best: Option<Selection> = None;
    for subset in subsets {
        if robust && !subset.robustly_feasible {
            continue;
        }
        let value = if robust && diagonal {
            if objective_is_d {
                subset.worst_case_diagonal_assumption_log_determinant?
            } else {
                subset.worst_case_diagonal_assumption_minimum_eigenvalue?
            }
        } else if robust {
            if objective_is_d {
                subset.worst_case_log_determinant?
            } else {
                subset.worst_case_minimum_eigenvalue?
            }
        } else if diagonal {
            if objective_is_d {
                subset.diagonal_assumption_log_determinant?
            } else if subset.nominal_rank == 2 {
                subset.diagonal_assumption_minimum_eigenvalue
            } else {
                continue;
            }
        } else if objective_is_d {
            match subset.nominal_log_determinant {
                Some(value) => value,
                None => continue,
            }
        } else if subset.nominal_rank == 2 {
            subset.nominal_minimum_eigenvalue
        } else {
            continue;
        };

        let candidate = Selection {
            sensor_ids: subset.sensor_ids.clone(),
            objective_value: value,
        };
        if best
            .as_ref()
            .is_none_or(|previous| selection_better(&candidate, previous))
        {
            best = Some(candidate);
        }
    }
    best
}

fn evaluate(input: &ModelInput, source_sha256: &str) -> Result<Report, String> {
    validate(input)?;
    let scales = [input.parameters[0].scale, input.parameters[1].scale];
    let scenarios = failure_scenarios(input);
    let mut index_sets = Vec::new();
    combinations(
        0,
        input.select_count,
        input.sensors.len(),
        &mut Vec::new(),
        &mut index_sets,
    );

    let mut subsets = Vec::with_capacity(index_sets.len());
    for indices in index_sets {
        let sensor_ids: Vec<String> = indices
            .iter()
            .map(|index| input.sensors[*index].id.clone())
            .collect();
        let mut metrics = Vec::with_capacity(scenarios.len());
        for scenario in &scenarios {
            metrics.push(scenario_metrics(input, &indices, scenario, &scales)?);
        }
        let nominal = metrics
            .iter()
            .find(|metric| metric.scenario_id == "none")
            .ok_or_else(|| "nominal failure scenario missing".to_owned())?;
        let robustly_feasible = metrics.iter().all(|metric| metric.rank == 2);
        let mut limiting = metrics
            .first()
            .cloned()
            .ok_or_else(|| "failure scenario set must not be empty".to_owned())?;
        for metric in metrics.iter().skip(1) {
            if metric.minimum_eigenvalue < limiting.minimum_eigenvalue - TIE_TOL
                || ((metric.minimum_eigenvalue - limiting.minimum_eigenvalue).abs() <= TIE_TOL
                    && metric.scenario_id < limiting.scenario_id)
            {
                limiting = metric.clone();
            }
        }
        let worst_case_log_determinant = if robustly_feasible {
            metrics
                .iter()
                .map(|metric| metric.log_determinant)
                .collect::<Option<Vec<f64>>>()
                .and_then(|values| values.into_iter().reduce(f64::min))
        } else {
            None
        };
        let worst_case_diagonal_assumption_log_determinant = if robustly_feasible {
            metrics
                .iter()
                .map(|metric| metric.diagonal_assumption_log_determinant)
                .collect::<Option<Vec<f64>>>()
                .and_then(|values| values.into_iter().reduce(f64::min))
        } else {
            None
        };
        let worst_case_diagonal_assumption_minimum_eigenvalue = if robustly_feasible {
            metrics
                .iter()
                .map(|metric| metric.diagonal_assumption_minimum_eigenvalue)
                .reduce(f64::min)
        } else {
            None
        };
        subsets.push(SubsetMetrics {
            sensor_ids,
            nominal_rank: nominal.rank,
            nominal_minimum_eigenvalue: nominal.minimum_eigenvalue,
            nominal_log_determinant: nominal.log_determinant,
            diagonal_assumption_minimum_eigenvalue: nominal.diagonal_assumption_minimum_eigenvalue,
            diagonal_assumption_log_determinant: nominal.diagonal_assumption_log_determinant,
            robustly_feasible,
            worst_case_minimum_eigenvalue: if robustly_feasible {
                Some(limiting.minimum_eigenvalue)
            } else {
                None
            },
            worst_case_log_determinant,
            worst_case_diagonal_assumption_minimum_eigenvalue,
            worst_case_diagonal_assumption_log_determinant,
            limiting_scenario_id: limiting.scenario_id,
            scenario_metrics: metrics,
        });
    }

    let robustly_feasible_subset_count = subsets
        .iter()
        .filter(|subset| subset.robustly_feasible)
        .count();

    Ok(Report {
        schema_version: input.schema_version.clone(),
        model_id: input.model_id.clone(),
        source_sha256: source_sha256.to_owned(),
        covariance_id: input.covariance_id.clone(),
        covariance_provenance: input.covariance_provenance.clone(),
        covariance_unit: input.noise_covariance.unit.clone(),
        channel_ids: input.noise_covariance.channel_ids.clone(),
        candidate_sensor_count: input.sensors.len(),
        select_count: input.select_count,
        failure_scenario_count: scenarios.len(),
        subset_count: subsets.len(),
        robustly_feasible_subset_count,
        nominal_full_covariance_d_optimal_selection: select_best(&subsets, false, true, false),
        nominal_full_covariance_e_optimal_selection: select_best(&subsets, false, false, false),
        nominal_diagonal_assumption_d_optimal_selection: select_best(&subsets, false, true, true),
        nominal_diagonal_assumption_e_optimal_selection: select_best(&subsets, false, false, true),
        robust_full_covariance_d_optimal_selection: select_best(&subsets, true, true, false),
        robust_full_covariance_e_optimal_selection: select_best(&subsets, true, false, false),
        robust_diagonal_assumption_d_optimal_selection: select_best(&subsets, true, true, true),
        robust_diagonal_assumption_e_optimal_selection: select_best(&subsets, true, false, true),
        subsets,
        claim_ceiling: "synthetic_full_covariance_single_failure_domain_observability_only"
            .to_owned(),
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
    fn joint_covariance_and_failure_known_answer() {
        let result = evaluate_bytes(FIXTURE).expect("fixture must evaluate");
        assert_eq!(result.subset_count, 15);
        assert_eq!(result.failure_scenario_count, 10);
        assert_eq!(result.robustly_feasible_subset_count, 12);
        let nominal_d = result.nominal_full_covariance_d_optimal_selection.unwrap();
        assert_eq!(nominal_d.sensor_ids, vec!["S_A", "S_C", "S_E", "S_F"]);
        let nominal_e = result.nominal_full_covariance_e_optimal_selection.unwrap();
        assert_eq!(nominal_e.sensor_ids, vec!["S_A", "S_B", "S_E", "S_F"]);
        let robust_d = result.robust_full_covariance_d_optimal_selection.unwrap();
        assert_eq!(robust_d.sensor_ids, vec!["S_A", "S_C", "S_E", "S_F"]);
        assert!((robust_d.objective_value - 3.2252963926711233).abs() < 1.0e-9);
        let robust_e = result.robust_full_covariance_e_optimal_selection.unwrap();
        assert_eq!(robust_e.sensor_ids, vec!["S_A", "S_C", "S_E", "S_F"]);
        assert!((robust_e.objective_value - 2.906088177643904).abs() < 1.0e-9);
        let naive_nominal_d = result.nominal_diagonal_assumption_d_optimal_selection.unwrap();
        assert_eq!(naive_nominal_d.sensor_ids, vec!["S_A", "S_B", "S_C", "S_D"]);
        let naive_robust_d = result.robust_diagonal_assumption_d_optimal_selection.unwrap();
        assert_eq!(naive_robust_d.sensor_ids, vec!["S_A", "S_C", "S_E", "S_F"]);
    }

    #[test]
    fn covariance_channel_order_and_matrix_contract_fail_closed() {
        let mut value = input();
        value.noise_covariance.channel_ids.swap(0, 1);
        assert!(validate(&value).is_err());
        let mut value = input();
        value.noise_covariance.matrix[0][1] += 0.01;
        assert!(validate(&value).is_err());
        let mut value = input();
        value.noise_covariance.matrix[0][1] = 0.2;
        value.noise_covariance.matrix[1][0] = 0.2;
        assert!(validate(&value).is_err());
    }

    #[test]
    fn malformed_common_cause_groups_reject() {
        let mut value = input();
        value.common_cause_groups[0].sensor_ids.push("UNKNOWN".to_owned());
        assert!(validate(&value).is_err());
        let mut value = input();
        value.common_cause_groups[0]
            .sensor_ids
            .push(value.common_cause_groups[0].sensor_ids[0].clone());
        assert!(validate(&value).is_err());
    }

    #[test]
    fn diagonal_noise_is_not_equivalent_to_full_covariance() {
        let result = evaluate_bytes(FIXTURE).unwrap();
        assert_ne!(
            result.nominal_full_covariance_d_optimal_selection.unwrap().sensor_ids,
            result.nominal_diagonal_assumption_d_optimal_selection.unwrap().sensor_ids
        );
        assert_ne!(
            result.nominal_full_covariance_e_optimal_selection.unwrap().sensor_ids,
            result.nominal_diagonal_assumption_e_optimal_selection.unwrap().sensor_ids
        );
    }

    #[test]
    fn rank_deficiency_prevents_robust_selection() {
        let mut value = input();
        for sensor in &mut value.sensors {
            sensor.observation_row = vec![1.0, 0.0];
        }
        let result = evaluate(&value, &sha256_hex(b"rank-deficient")).unwrap();
        assert_eq!(result.robustly_feasible_subset_count, 0);
        assert!(result.robust_full_covariance_d_optimal_selection.is_none());
        assert!(result.robust_full_covariance_e_optimal_selection.is_none());
    }
}
