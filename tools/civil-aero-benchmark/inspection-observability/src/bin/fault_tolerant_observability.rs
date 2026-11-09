// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Tier-0 synthetic fault-tolerant sensor-subset oracle.
// This is not a structural solver, SHM detector, or aircraft safety tool.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;

const FIXTURE: &[u8] =
    include_bytes!("../../fixtures/fault-tolerant-observability-v0.1.json");
const RANK_REL_TOL: f64 = 1.0e-12;
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
    variance: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct FailureGroup {
    id: String,
    sensor_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ModelInput {
    schema_version: String,
    model_id: String,
    model_scope: String,
    noise_model: String,
    parameters: Vec<Parameter>,
    sensors: Vec<Sensor>,
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
    nullity: usize,
    minimum_eigenvalue: f64,
    log_determinant: Option<f64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct SubsetMetrics {
    sensor_ids: Vec<String>,
    nominal_rank: usize,
    nominal_minimum_eigenvalue: f64,
    nominal_log_determinant: Option<f64>,
    robustly_feasible: bool,
    worst_case_minimum_eigenvalue: Option<f64>,
    limiting_scenario_id: Option<String>,
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
    candidate_sensor_count: usize,
    select_count: usize,
    failure_scenario_count: usize,
    subset_count: usize,
    robustly_feasible_subset_count: usize,
    nominal_d_optimal_selection: Option<Selection>,
    nominal_e_optimal_selection: Option<Selection>,
    robust_d_optimal_selection: Option<Selection>,
    robust_e_optimal_selection: Option<Selection>,
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

fn validate_input(input: &ModelInput) -> Result<(), String> {
    if input.schema_version != "aero-inspection-fault-tolerant-observability/0.1.0" {
        return Err("unsupported schema_version".to_owned());
    }
    if input.model_id.trim().is_empty() {
        return Err("model_id must be non-empty".to_owned());
    }
    if input.model_scope != "synthetic_local_linear_observation" {
        return Err("unsupported model_scope".to_owned());
    }
    if input.noise_model != "independent_variance_per_sensor" {
        return Err("unsupported noise_model".to_owned());
    }
    if input.parameters.len() != 2 {
        return Err("v0.1 requires exactly two parameters".to_owned());
    }
    if input.sensors.len() < 3 || input.sensors.len() > 16 {
        return Err("candidate sensor count must be in 3..=16".to_owned());
    }
    if input.select_count < 3 || input.select_count > input.sensors.len() {
        return Err("select_count must be in 3..=candidate sensor count".to_owned());
    }
    if input.common_cause_groups.len() > 32 {
        return Err("common-cause group count exceeds 32".to_owned());
    }
    if input.maximum_simultaneous_failure_domains != 1 {
        return Err("v0.1 admits exactly one failed sensor or one failed common-cause domain".to_owned());
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
        if sensor.observation_row.len() != 2
            || sensor.observation_row.iter().any(|value| !value.is_finite())
        {
            return Err(format!("sensor {} must have two finite sensitivities", sensor.id));
        }
        if !sensor.variance.is_finite() || sensor.variance <= 0.0 {
            return Err(format!("sensor {} variance must be finite and positive", sensor.id));
        }
    }

    let mut group_ids = BTreeSet::new();
    for group in &input.common_cause_groups {
        if group.id.trim().is_empty() || !group_ids.insert(group.id.as_str()) {
            return Err("common-cause group IDs must be non-empty and unique".to_owned());
        }
        if group.sensor_ids.len() < 2 {
            return Err(format!("common-cause group {} must contain at least two sensors", group.id));
        }
        let mut members = BTreeSet::new();
        for sensor_id in &group.sensor_ids {
            if !sensor_ids.contains(sensor_id.as_str()) {
                return Err(format!("group {} references unknown sensor {}", group.id, sensor_id));
            }
            if !members.insert(sensor_id.as_str()) {
                return Err(format!("group {} contains duplicate sensor {}", group.id, sensor_id));
            }
        }
    }
    Ok(())
}

fn scenarios(input: &ModelInput) -> Vec<FailureScenario> {
    let mut sensors = input.sensors.clone();
    sensors.sort_by(|a, b| a.id.cmp(&b.id));
    let mut groups = input.common_cause_groups.clone();
    groups.sort_by(|a, b| a.id.cmp(&b.id));

    let mut result = vec![FailureScenario {
        id: "none".to_owned(),
        failed_sensor_ids: BTreeSet::new(),
    }];
    for sensor in &sensors {
        result.push(FailureScenario {
            id: format!("single_sensor:{}", sensor.id),
            failed_sensor_ids: BTreeSet::from([sensor.id.clone()]),
        });
    }
    for group in &groups {
        result.push(FailureScenario {
            id: format!("common_cause:{}", group.id),
            failed_sensor_ids: group.sensor_ids.iter().cloned().collect(),
        });
    }
    result
}

fn rank_of(rows: &[Vec<f64>]) -> usize {
    if rows.is_empty() {
        return 0;
    }
    let mut work = rows.to_vec();
    let row_count = work.len();
    let column_count = 2usize;
    let maximum = work
        .iter()
        .flatten()
        .fold(0.0_f64, |acc, value| acc.max(value.abs()));
    let tolerance = RANK_REL_TOL * maximum.max(1.0);
    let mut pivot_row = 0usize;

    for column in 0..column_count {
        if pivot_row >= row_count {
            break;
        }
        let mut best_row = pivot_row;
        let mut best_value = 0.0_f64;
        for row in pivot_row..row_count {
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
        for row in (pivot_row + 1)..row_count {
            let factor = work[row][column] / pivot;
            for col in column..column_count {
                work[row][col] -= factor * work[pivot_row][col];
            }
        }
        pivot_row += 1;
    }
    pivot_row
}

fn evaluate_scenario(
    chosen: &[&Sensor],
    failed: &BTreeSet<String>,
    scenario_id: &str,
    scales: &[f64; 2],
) -> Result<ScenarioMetrics, String> {
    let retained: Vec<&Sensor> = chosen
        .iter()
        .copied()
        .filter(|sensor| !failed.contains(&sensor.id))
        .collect();
    let mut ids: Vec<String> = retained.iter().map(|sensor| sensor.id.clone()).collect();
    ids.sort();

    let rows: Vec<Vec<f64>> = retained
        .iter()
        .map(|sensor| {
            vec![
                sensor.observation_row[0] * scales[0],
                sensor.observation_row[1] * scales[1],
            ]
        })
        .collect();
    if rows.iter().flatten().any(|value| !value.is_finite()) {
        return Err("scaled observation row overflowed".to_owned());
    }

    let rank = rank_of(&rows);
    let mut f00 = 0.0_f64;
    let mut f01 = 0.0_f64;
    let mut f11 = 0.0_f64;
    for (sensor, row) in retained.iter().zip(&rows) {
        let weight = 1.0 / sensor.variance;
        f00 += row[0] * row[0] * weight;
        f01 += row[0] * row[1] * weight;
        f11 += row[1] * row[1] * weight;
    }
    if !f00.is_finite() || !f01.is_finite() || !f11.is_finite() {
        return Err("Fisher information overflowed".to_owned());
    }

    let discriminant = ((f00 - f11).powi(2) + 4.0 * f01.powi(2)).sqrt();
    let mut minimum_eigenvalue = ((f00 + f11) - discriminant) / 2.0;
    if !minimum_eigenvalue.is_finite() {
        return Err("minimum Fisher eigenvalue is non-finite".to_owned());
    }
    if minimum_eigenvalue < 0.0 && minimum_eigenvalue > -1.0e-10 {
        minimum_eigenvalue = 0.0;
    }
    minimum_eigenvalue = minimum_eigenvalue.max(0.0);

    let determinant = f00 * f11 - f01 * f01;
    let log_determinant = if rank == 2 && determinant.is_finite() && determinant > 0.0 {
        Some(determinant.ln())
    } else {
        None
    };

    Ok(ScenarioMetrics {
        scenario_id: scenario_id.to_owned(),
        failed_sensor_ids: failed.iter().cloned().collect(),
        retained_sensor_ids: ids,
        rank,
        nullity: 2 - rank,
        minimum_eigenvalue,
        log_determinant,
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

fn better(candidate: &Selection, previous: &Selection) -> bool {
    candidate.objective_value > previous.objective_value + TIE_TOL
        || ((candidate.objective_value - previous.objective_value).abs() <= TIE_TOL
            && candidate.sensor_ids < previous.sensor_ids)
}

fn evaluate(input: &ModelInput, source_sha256: &str) -> Result<Report, String> {
    validate_input(input)?;
    let mut sensors = input.sensors.clone();
    sensors.sort_by(|a, b| a.id.cmp(&b.id));
    let scales = [input.parameters[0].scale, input.parameters[1].scale];
    let scenarios = scenarios(input);
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
        let chosen: Vec<&Sensor> = indices.iter().map(|index| &sensors[*index]).collect();
        let sensor_ids: Vec<String> = chosen.iter().map(|sensor| sensor.id.clone()).collect();
        let mut scenario_metrics = Vec::with_capacity(scenarios.len());
        for scenario in &scenarios {
            scenario_metrics.push(evaluate_scenario(
                &chosen,
                &scenario.failed_sensor_ids,
                &scenario.id,
                &scales,
            )?);
        }

        let nominal = scenario_metrics
            .iter()
            .find(|metric| metric.scenario_id == "none")
            .ok_or_else(|| "nominal scenario missing".to_owned())?;
        let robustly_feasible = scenario_metrics.iter().all(|metric| metric.rank == 2);
        let mut limiting = scenario_metrics.first().cloned();
        for metric in scenario_metrics.iter().skip(1) {
            if let Some(current) = &limiting {
                if metric.minimum_eigenvalue < current.minimum_eigenvalue - TIE_TOL
                    || ((metric.minimum_eigenvalue - current.minimum_eigenvalue).abs() <= TIE_TOL
                        && metric.scenario_id < current.scenario_id)
                {
                    limiting = Some(metric.clone());
                }
            }
        }
        let limiting = limiting.ok_or_else(|| "failure scenarios unexpectedly empty".to_owned())?;

        subsets.push(SubsetMetrics {
            sensor_ids,
            nominal_rank: nominal.rank,
            nominal_minimum_eigenvalue: nominal.minimum_eigenvalue,
            nominal_log_determinant: nominal.log_determinant,
            robustly_feasible,
            worst_case_minimum_eigenvalue: if robustly_feasible {
                Some(limiting.minimum_eigenvalue)
            } else {
                None
            },
            limiting_scenario_id: Some(limiting.scenario_id),
            scenario_metrics,
        });
    }

    let mut nominal_d_best: Option<Selection> = None;
    let mut nominal_e_best: Option<Selection> = None;
    let mut robust_d_best: Option<Selection> = None;
    let mut robust_e_best: Option<Selection> = None;
    for subset in &subsets {
        if let Some(value) = subset.nominal_log_determinant {
            let candidate = Selection {
                sensor_ids: subset.sensor_ids.clone(),
                objective_value: value,
            };
            if nominal_d_best.as_ref().is_none_or(|previous| better(&candidate, previous)) {
                nominal_d_best = Some(candidate);
            }
        }
        if subset.nominal_rank == 2 {
            let candidate = Selection {
                sensor_ids: subset.sensor_ids.clone(),
                objective_value: subset.nominal_minimum_eigenvalue,
            };
            if nominal_e_best.as_ref().is_none_or(|previous| better(&candidate, previous)) {
                nominal_e_best = Some(candidate);
            }
        }
        if subset.robustly_feasible {
            let worst_case_log_determinant = subset
                .scenario_metrics
                .iter()
                .map(|metric| metric.log_determinant)
                .collect::<Option<Vec<f64>>>()
                .and_then(|values| values.into_iter().reduce(f64::min));
            if let Some(value) = worst_case_log_determinant {
                let candidate = Selection {
                    sensor_ids: subset.sensor_ids.clone(),
                    objective_value: value,
                };
                if robust_d_best.as_ref().is_none_or(|previous| better(&candidate, previous)) {
                    robust_d_best = Some(candidate);
                }
            }
            let candidate = Selection {
                sensor_ids: subset.sensor_ids.clone(),
                objective_value: subset.worst_case_minimum_eigenvalue.unwrap_or(0.0),
            };
            if robust_e_best.as_ref().is_none_or(|previous| better(&candidate, previous)) {
                robust_e_best = Some(candidate);
            }
        }
    }

    let robustly_feasible_subset_count = subsets.iter().filter(|subset| subset.robustly_feasible).count();
    Ok(Report {
        schema_version: input.schema_version.clone(),
        model_id: input.model_id.clone(),
        source_sha256: source_sha256.to_owned(),
        candidate_sensor_count: sensors.len(),
        select_count: input.select_count,
        failure_scenario_count: scenarios.len(),
        subset_count: subsets.len(),
        robustly_feasible_subset_count,
        nominal_d_optimal_selection: nominal_d_best,
        nominal_e_optimal_selection: nominal_e_best,
        robust_d_optimal_selection: robust_d_best,
        robust_e_optimal_selection: robust_e_best,
        subsets,
        claim_ceiling: "synthetic_single_sensor_dropout_and_single_common_cause_group_only".to_owned(),
    })
}

fn evaluate_bytes(bytes: &[u8]) -> Result<Report, String> {
    let input: ModelInput =
        serde_json::from_slice(bytes).map_err(|error| format!("input JSON parse failed: {error}"))?;
    evaluate(&input, &sha256_hex(bytes))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bytes = fs::read(manifest.join("fixtures/fault-tolerant-observability-v0.1.json"))?;
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
    fn nominal_optimum_is_rejected_by_common_cause_failures() {
        let result = evaluate_bytes(FIXTURE).expect("fixture must evaluate");
        assert_eq!(result.subset_count, 15);
        assert_eq!(result.failure_scenario_count, 10);
        assert_eq!(result.robustly_feasible_subset_count, 12);
        let nominal = result.nominal_e_optimal_selection.unwrap();
        assert_eq!(nominal.sensor_ids, vec!["S_A", "S_B", "S_C", "S_D"]);
        assert!((nominal.objective_value - 10.0).abs() < 1.0e-10);
        let nominal_d = result.nominal_d_optimal_selection.unwrap();
        assert_eq!(nominal_d.sensor_ids, vec!["S_A", "S_B", "S_C", "S_D"]);
        assert!((nominal_d.objective_value - 100.0_f64.ln()).abs() < 1.0e-10);
        let nominal_metrics = result
            .subsets
            .iter()
            .find(|subset| subset.sensor_ids == vec!["S_A", "S_B", "S_C", "S_D"])
            .unwrap();
        assert!(!nominal_metrics.robustly_feasible);
        assert!(nominal_metrics.scenario_metrics.iter().any(|metric| {
            metric.scenario_id == "common_cause:PWR_X" && metric.rank == 1
        }));
    }

    #[test]
    fn robust_maximin_e_optimum_survives_every_declared_scenario() {
        let result = evaluate_bytes(FIXTURE).expect("fixture must evaluate");
        let robust = result.robust_e_optimal_selection.unwrap();
        assert_eq!(robust.sensor_ids, vec!["S_A", "S_C", "S_E", "S_F"]);
        assert!((robust.objective_value - 4.482666666666667).abs() < 1.0e-9);
        let robust_d = result.robust_d_optimal_selection.unwrap();
        assert_eq!(robust_d.sensor_ids, vec!["S_A", "S_C", "S_E", "S_F"]);
        assert!((robust_d.objective_value - 3.2188758248682006).abs() < 1.0e-9);
        let selected = result
            .subsets
            .iter()
            .find(|subset| subset.sensor_ids == robust.sensor_ids)
            .unwrap();
        assert!(selected.robustly_feasible);
        assert!(selected.scenario_metrics.iter().all(|metric| metric.rank == 2));
    }

    #[test]
    fn sensor_and_group_order_do_not_change_the_selection() {
        let mut value = input();
        let digest = sha256_hex(FIXTURE);
        let original = evaluate(&value, &digest).unwrap();
        value.sensors.reverse();
        value.common_cause_groups.reverse();
        let reversed = evaluate(&value, &digest).unwrap();
        assert_eq!(original, reversed);
    }

    #[test]
    fn malformed_groups_and_variances_reject() {
        let mut value = input();
        value.common_cause_groups[0].sensor_ids.push("S_A".to_owned());
        assert!(validate_input(&value).is_err());
        let mut value = input();
        value.common_cause_groups[0].sensor_ids[0] = "UNKNOWN".to_owned();
        assert!(validate_input(&value).is_err());
        let mut value = input();
        value.sensors[0].variance = 0.0;
        assert!(validate_input(&value).is_err());
    }

    #[test]
    fn no_robustly_feasible_subset_is_reported_as_none() {
        let mut value = input();
        for sensor in &mut value.sensors {
            sensor.observation_row = vec![1.0, 0.0];
        }
        let result = evaluate(&value, &sha256_hex(b"rank-deficient-control")).unwrap();
        assert_eq!(result.robustly_feasible_subset_count, 0);
        assert!(result.robust_e_optimal_selection.is_none());
    }
}
