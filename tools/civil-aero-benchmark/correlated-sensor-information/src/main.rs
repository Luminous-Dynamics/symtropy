// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Synthetic, quasi-static, correlated-noise sensor-information oracle.
// This is not an aircraft instrumentation recommendation, SHM product,
// structural solver, damage-detection system or certification tool.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as _;

const FIXTURE: &[u8] = include_bytes!("../data/instrument-correlated-noise-v0.1.json");
const JOINT_FIXTURE: &[u8] = include_bytes!("../data/instrument-correlated-failure-v0.1.json");
const CLAIM_CEILING: &str = "synthetic_correlated_fisher_observability_only";
const RANK_TOLERANCE: f64 = 1.0e-10;
const PIVOT_TOLERANCE: f64 = 1.0e-12;
const SCORE_TOLERANCE: f64 = 1.0e-12;

type AuditResult<T> = Result<T, String>;
type Matrix = Vec<Vec<f64>>;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sensor {
    sensor_id: String,
    sensor_type: String,
    sensitivity: [f64; 2],
    operator_gain: f64,
    operator_status: String,
    operator_sha256: String,
    calibration_status: String,
    common_cause_group: String,
    deployment_cost_units: f64,
    mass_g: f64,
    power_mw: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelAssumptions {
    parameter_names: Vec<String>,
    observation_model: String,
    noise_model: String,
    sensitivity_units: String,
    covariance_units: String,
    operating_point_id: String,
    dynamic_transfer_function_status: String,
    physical_aircraft_model: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Constraints {
    d_opt_sensor_count: usize,
    e_opt_sensor_count: usize,
    robust_e_opt_sensor_count: usize,
    sensor_dropout_budget: usize,
    common_cause_group_dropout: bool,
    minimum_robust_minimum_eigenvalue: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NoiseCovariance {
    channel_ids: Vec<String>,
    units: String,
    evidence_status: String,
    matrix: Matrix,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    diagonal_e_opt_sensor_ids: Vec<String>,
    diagonal_e_opt_minimum_eigenvalue: f64,
    correlated_d_opt_sensor_ids: Vec<String>,
    correlated_d_opt_determinant: f64,
    correlated_e_opt_sensor_ids: Vec<String>,
    correlated_e_opt_minimum_eigenvalue: f64,
    robust_d_opt_sensor_ids: Vec<String>,
    robust_d_opt_worst_case_log_determinant: f64,
    robust_e_opt_sensor_ids: Vec<String>,
    robust_e_opt_worst_case_minimum_eigenvalue: f64,
    minimum_cost_robust_sensor_ids: Vec<String>,
    minimum_cost_robust_worst_case_minimum_eigenvalue: f64,
    minimum_cost_robust_total_deployment_cost_units: f64,
    minimum_cost_robust_total_mass_g: f64,
    minimum_cost_robust_total_power_mw: f64,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    problem_id: String,
    claim_ceiling: String,
    model_assumptions: ModelAssumptions,
    constraints: Constraints,
    sensors: Vec<Sensor>,
    noise_covariance: NoiseCovariance,
    expected: Expected,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct InformationMetrics {
    fisher_information: [[f64; 2]; 2],
    eigenvalues_ascending: [f64; 2],
    effective_rank: usize,
    determinant: Option<f64>,
    log_determinant: Option<f64>,
    minimum_eigenvalue: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Selection {
    sensor_ids: Vec<String>,
    objective: String,
    score: f64,
    information: InformationMetrics,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct FailureScenario {
    scenario_id: String,
    lost_sensor_ids: Vec<String>,
    surviving_sensor_ids: Vec<String>,
    effective_rank: usize,
    minimum_eigenvalue: f64,
    log_determinant: Option<f64>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct RobustSelection {
    sensor_ids: Vec<String>,
    objective: String,
    worst_case_minimum_eigenvalue: f64,
    scenarios: Vec<FailureScenario>,
    total_deployment_cost_units: f64,
    total_mass_g: f64,
    total_power_mw: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct RobustDSelection {
    sensor_ids: Vec<String>,
    objective: String,
    worst_case_log_determinant: f64,
    scenarios: Vec<FailureScenario>,
    total_deployment_cost_units: f64,
    total_mass_g: f64,
    total_power_mw: f64,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct MinimumCostRobustSelection {
    sensor_ids: Vec<String>,
    objective: String,
    required_minimum_eigenvalue: f64,
    achieved_worst_case_minimum_eigenvalue: f64,
    total_deployment_cost_units: f64,
    total_mass_g: f64,
    total_power_mw: f64,
    scenarios: Vec<FailureScenario>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
struct Audit {
    schema_version: &'static str,
    problem_id: String,
    source_sha256: String,
    source_bytes: usize,
    sensor_candidate_count: usize,
    covariance_channel_count: usize,
    covariance_positive_definite: bool,
    covariance_correlated_pairs: usize,
    diagonal_e_opt: Selection,
    correlated_d_opt: Selection,
    correlated_e_opt: Selection,
    robust_correlated_d_opt: RobustDSelection,
    robust_correlated_e_opt: RobustSelection,
    cost_constrained_robust: Option<MinimumCostRobustSelection>,
    diagonal_to_correlated_e_opt_changed: bool,
    model_claim_ceiling: &'static str,
    aircraft_feasibility: &'static str,
    dynamic_observation_status: &'static str,
    physical_calibration_status: &'static str,
}

fn fail<T>(message: impl Into<String>) -> AuditResult<T> {
    Err(message.into())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("String write cannot fail");
    }
    output
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn cholesky_factor(matrix: &[Vec<f64>]) -> AuditResult<Matrix> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return fail("noise covariance must be a non-empty square matrix");
    }
    for i in 0..n {
        for j in 0..n {
            if !matrix[i][j].is_finite() {
                return fail("noise covariance contains a non-finite value");
            }
            if (matrix[i][j] - matrix[j][i]).abs() > 1.0e-12 {
                return fail("noise covariance is not symmetric within tolerance");
            }
        }
    }

    // Strict Cholesky: no jitter, diagonal loading, inverse or pseudoinverse.
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = matrix[i][j];
            for k in 0..j {
                sum -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !sum.is_finite() || sum <= PIVOT_TOLERANCE {
                    return fail("noise covariance is not strictly positive definite");
                }
                lower[i][j] = sum.sqrt();
            } else {
                if lower[j][j] <= PIVOT_TOLERANCE {
                    return fail("noise covariance Cholesky pivot is invalid");
                }
                lower[i][j] = sum / lower[j][j];
            }
        }
    }
    Ok(lower)
}

fn validate_covariance(matrix: &[Vec<f64>]) -> AuditResult<()> {
    cholesky_factor(matrix).map(|_| ())
}

fn solve_cholesky(lower: &[Vec<f64>], rhs: &[f64]) -> AuditResult<Vec<f64>> {
    let n = lower.len();
    if rhs.len() != n {
        return fail("covariance solve dimension mismatch");
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
        return fail("covariance solve produced a non-finite value");
    }
    Ok(solution)
}

fn validate_fixture(fixture: &Fixture) -> AuditResult<()> {
    if fixture.schema_version != "aero-correlated-sensor-information/0.1.0" {
        return fail("unsupported correlated-sensor fixture schema");
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        return fail("fixture claim ceiling was changed or weakened");
    }
    if fixture.problem_id.trim().is_empty() {
        return fail("problem_id must be non-empty");
    }
    let assumptions = &fixture.model_assumptions;
    if assumptions.parameter_names.len() != 2
        || assumptions.observation_model != "linear_scalar_gain_times_static_sensitivity"
        || assumptions.noise_model != "correlated_zero_mean_gaussian_covariance"
        || assumptions.sensitivity_units != "dimensionless_normalized"
        || assumptions.covariance_units != "normalized_output_squared"
        || assumptions.operating_point_id != "synthetic_quasistatic_reference"
        || assumptions.dynamic_transfer_function_status != "NOT_ASSESSED"
        || assumptions.physical_aircraft_model
    {
        return fail("fixture assumptions are outside the admitted quasi-static synthetic oracle");
    }

    let constraints = &fixture.constraints;
    let sensor_count = fixture.sensors.len();
    if !(2..=sensor_count).contains(&constraints.d_opt_sensor_count)
        || !(2..=sensor_count).contains(&constraints.e_opt_sensor_count)
        || !(3..=sensor_count).contains(&constraints.robust_e_opt_sensor_count)
        || constraints.sensor_dropout_budget != 1
        || !constraints.common_cause_group_dropout
        || !finite_positive(constraints.minimum_robust_minimum_eigenvalue)
    {
        return fail("fixture selection constraints are outside v0.1");
    }

    if !(5..=16).contains(&sensor_count) {
        return fail("v0.1 requires 5..=16 candidate sensors");
    }

    let mut seen = BTreeSet::new();
    let mut previous: Option<&str> = None;
    for sensor in &fixture.sensors {
        if sensor.sensor_id.trim().is_empty() || !seen.insert(sensor.sensor_id.as_str()) {
            return fail(format!("empty or duplicate sensor ID {}", sensor.sensor_id));
        }
        if previous.is_some_and(|prior| prior >= sensor.sensor_id.as_str()) {
            return fail("sensor candidates must be strictly ordered by sensor_id");
        }
        previous = Some(sensor.sensor_id.as_str());

        if sensor.sensor_type.trim().is_empty()
            || sensor.common_cause_group.trim().is_empty()
            || !sensor.sensitivity.iter().all(|value| value.is_finite())
            || !finite_positive(sensor.operator_gain)
            || !finite_positive(sensor.deployment_cost_units)
            || !sensor.mass_g.is_finite()
            || sensor.mass_g < 0.0
            || !sensor.power_mw.is_finite()
            || sensor.power_mw < 0.0
        {
            return fail(format!(
                "{}: invalid sensor candidate field",
                sensor.sensor_id
            ));
        }
        if sensor.operator_status != "ADMITTED" || !is_sha256(&sensor.operator_sha256) {
            return fail(format!(
                "{}: observation operator is missing, unadmitted or has an invalid digest",
                sensor.sensor_id
            ));
        }
        if sensor.calibration_status != "VALID" {
            return fail(format!(
                "{}: calibration is not valid at the declared operating point",
                sensor.sensor_id
            ));
        }
    }

    let covariance = &fixture.noise_covariance;
    if covariance.units != "normalized_output_squared"
        || covariance.evidence_status != "synthetic_reference_only"
    {
        return fail("covariance units or source status are not admitted by v0.1");
    }
    let expected_ids = fixture
        .sensors
        .iter()
        .map(|sensor| sensor.sensor_id.clone())
        .collect::<Vec<_>>();
    if covariance.channel_ids != expected_ids {
        return fail("covariance channel order does not exactly match sensor registry order");
    }
    if covariance.matrix.len() != fixture.sensors.len()
        || covariance
            .matrix
            .iter()
            .any(|row| row.len() != fixture.sensors.len())
    {
        return fail("covariance dimensions do not match the sensor registry");
    }
    validate_covariance(&covariance.matrix)?;
    Ok(())
}

fn principal_matrix(matrix: &[Vec<f64>], indices: &[usize]) -> Matrix {
    indices
        .iter()
        .map(|row| {
            indices
                .iter()
                .map(|col| matrix[*row][*col])
                .collect::<Vec<_>>()
        })
        .collect()
}

fn diagonal_matrix(matrix: &[Vec<f64>], indices: &[usize]) -> Matrix {
    indices
        .iter()
        .map(|row| {
            indices
                .iter()
                .map(|col| if row == col { matrix[*row][*col] } else { 0.0 })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn fisher_matrix(
    fixture: &Fixture,
    indices: &[usize],
    diagonal_only: bool,
) -> AuditResult<[[f64; 2]; 2]> {
    if indices.len() < 2 {
        return fail("at least two surviving scalar sensors are required");
    }
    let covariance = if diagonal_only {
        diagonal_matrix(&fixture.noise_covariance.matrix, indices)
    } else {
        principal_matrix(&fixture.noise_covariance.matrix, indices)
    };
    let lower = cholesky_factor(&covariance)?;
    let mut effective = Vec::with_capacity(indices.len());
    for index in indices {
        let sensor = &fixture.sensors[*index];
        effective.push([
            sensor.sensitivity[0] * sensor.operator_gain,
            sensor.sensitivity[1] * sensor.operator_gain,
        ]);
    }

    let first_rhs = effective.iter().map(|row| row[0]).collect::<Vec<_>>();
    let second_rhs = effective.iter().map(|row| row[1]).collect::<Vec<_>>();
    let solved = [
        solve_cholesky(&lower, &first_rhs)?,
        solve_cholesky(&lower, &second_rhs)?,
    ];

    let mut information = [[0.0; 2]; 2];
    for a in 0..2 {
        for b in 0..2 {
            information[a][b] = effective
                .iter()
                .enumerate()
                .map(|(index, row)| row[a] * solved[b][index])
                .sum();
        }
    }
    if information.iter().flatten().any(|value| !value.is_finite()) {
        return fail("Fisher information matrix contains a non-finite value");
    }
    Ok(information)
}

fn eigenvalues_2x2(matrix: [[f64; 2]; 2]) -> [f64; 2] {
    let a = matrix[0][0];
    let b = 0.5 * (matrix[0][1] + matrix[1][0]);
    let c = matrix[1][1];
    let trace = a + c;
    let discriminant = ((a - c).powi(2) + 4.0 * b.powi(2)).sqrt();
    [
        ((trace - discriminant) / 2.0).max(0.0),
        ((trace + discriminant) / 2.0).max(0.0),
    ]
}

fn metrics(information: [[f64; 2]; 2]) -> InformationMetrics {
    let eigenvalues = eigenvalues_2x2(information);
    let determinant = information[0][0] * information[1][1] - information[0][1] * information[1][0];
    let effective_rank = eigenvalues
        .iter()
        .filter(|value| **value > RANK_TOLERANCE)
        .count();
    let determinant = if effective_rank == 2 && determinant > 0.0 {
        Some(determinant)
    } else {
        None
    };
    let log_determinant = determinant.map(f64::ln);

    InformationMetrics {
        fisher_information: information,
        eigenvalues_ascending: eigenvalues,
        effective_rank,
        determinant,
        log_determinant,
        minimum_eigenvalue: eigenvalues[0],
    }
}

fn sensor_ids(fixture: &Fixture, indices: &[usize]) -> Vec<String> {
    indices
        .iter()
        .map(|index| fixture.sensors[*index].sensor_id.clone())
        .collect()
}

fn enumerate_combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    fn walk(
        n: usize,
        k: usize,
        start: usize,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == k {
            output.push(current.clone());
            return;
        }
        let remaining = k - current.len();
        if remaining > n.saturating_sub(start) {
            return;
        }
        for index in start..=n - remaining {
            current.push(index);
            walk(n, k, index + 1, current, output);
            current.pop();
        }
    }

    let mut output = Vec::new();
    if k > 0 && k <= n {
        walk(n, k, 0, &mut Vec::new(), &mut output);
    }
    output
}

fn select_d_opt(fixture: &Fixture) -> AuditResult<Selection> {
    let mut best: Option<(f64, Vec<usize>, InformationMetrics)> = None;
    for indices in enumerate_combinations(
        fixture.sensors.len(),
        fixture.constraints.d_opt_sensor_count,
    ) {
        let information = fisher_matrix(fixture, &indices, false)?;
        let result = metrics(information);
        let Some(score) = result.determinant else {
            continue;
        };
        let is_better = best.as_ref().is_none_or(|(best_score, best_indices, _)| {
            score > *best_score + SCORE_TOLERANCE
                || ((score - *best_score).abs() <= SCORE_TOLERANCE
                    && sensor_ids(fixture, &indices) < sensor_ids(fixture, best_indices))
        });
        if is_better {
            best = Some((score, indices, result));
        }
    }
    let Some((score, indices, information)) = best else {
        return fail("no full-rank D-optimal sensor subset exists");
    };
    Ok(Selection {
        sensor_ids: sensor_ids(fixture, &indices),
        objective: "MAX_DETERMINANT_FULL_CORRELATED_COVARIANCE".to_owned(),
        score,
        information,
    })
}

fn select_e_opt(fixture: &Fixture, diagonal_only: bool) -> AuditResult<Selection> {
    let mut best: Option<(f64, Vec<usize>, InformationMetrics)> = None;
    for indices in enumerate_combinations(
        fixture.sensors.len(),
        fixture.constraints.e_opt_sensor_count,
    ) {
        let information = if diagonal_only {
            fisher_matrix(fixture, &indices, true)?
        } else {
            fisher_matrix(fixture, &indices, false)?
        };
        let result = metrics(information);
        if result.effective_rank < 2 {
            continue;
        }
        let score = result.minimum_eigenvalue;
        let is_better = best.as_ref().is_none_or(|(best_score, best_indices, _)| {
            score > *best_score + SCORE_TOLERANCE
                || ((score - *best_score).abs() <= SCORE_TOLERANCE
                    && sensor_ids(fixture, &indices) < sensor_ids(fixture, best_indices))
        });
        if is_better {
            best = Some((score, indices, result));
        }
    }
    let Some((score, indices, information)) = best else {
        return fail("no full-rank E-optimal sensor subset exists");
    };
    Ok(Selection {
        sensor_ids: sensor_ids(fixture, &indices),
        objective: if diagonal_only {
            "MAX_MINIMUM_EIGENVALUE_DIAGONAL_NOISE_BASELINE".to_owned()
        } else {
            "MAX_MINIMUM_EIGENVALUE_FULL_CORRELATED_COVARIANCE".to_owned()
        },
        score,
        information,
    })
}

fn failure_scenarios(
    fixture: &Fixture,
    selected: &[usize],
) -> Vec<(String, Vec<usize>, Vec<String>)> {
    let mut scenarios = Vec::new();

    for index in selected {
        let survivors = selected
            .iter()
            .copied()
            .filter(|candidate| candidate != index)
            .collect::<Vec<_>>();
        scenarios.push((
            format!("SENSOR_DROP:{}", fixture.sensors[*index].sensor_id),
            survivors,
            vec![fixture.sensors[*index].sensor_id.clone()],
        ));
    }

    let groups = selected
        .iter()
        .map(|index| fixture.sensors[*index].common_cause_group.clone())
        .collect::<BTreeSet<_>>();
    for group in groups {
        let lost = selected
            .iter()
            .filter(|index| fixture.sensors[**index].common_cause_group == group)
            .map(|index| fixture.sensors[*index].sensor_id.clone())
            .collect::<Vec<_>>();
        let survivors = selected
            .iter()
            .copied()
            .filter(|index| fixture.sensors[*index].common_cause_group != group)
            .collect::<Vec<_>>();
        scenarios.push((format!("GROUP_DROP:{group}"), survivors, lost));
    }

    scenarios.sort_by(|left, right| left.0.cmp(&right.0));
    scenarios
}

fn robust_score(
    fixture: &Fixture,
    selected: &[usize],
) -> AuditResult<(f64, Option<f64>, Vec<FailureScenario>)> {
    let mut scenarios = Vec::new();
    let mut worst_e = f64::INFINITY;
    let mut worst_d = Some(f64::INFINITY);
    for (scenario_id, survivors, lost) in failure_scenarios(fixture, selected) {
        let information = if survivors.len() < 2 {
            [[0.0; 2]; 2]
        } else {
            fisher_matrix(fixture, &survivors, false)?
        };
        let result = metrics(information);
        let rank = if survivors.len() < 2 {
            0
        } else {
            result.effective_rank
        };
        let minimum = if rank == 2 {
            result.minimum_eigenvalue
        } else {
            0.0
        };
        let log_determinant = if rank == 2 {
            result.log_determinant
        } else {
            None
        };
        worst_e = worst_e.min(minimum);
        worst_d = match (worst_d, log_determinant) {
            (Some(current), Some(value)) => Some(current.min(value)),
            _ => None,
        };
        scenarios.push(FailureScenario {
            scenario_id,
            lost_sensor_ids: lost,
            surviving_sensor_ids: sensor_ids(fixture, &survivors),
            effective_rank: rank,
            minimum_eigenvalue: minimum,
            log_determinant,
        });
    }
    Ok((worst_e, worst_d, scenarios))
}

fn totals(fixture: &Fixture, indices: &[usize]) -> (f64, f64, f64) {
    let mut cost = 0.0;
    let mut mass = 0.0;
    let mut power = 0.0;
    for index in indices {
        let sensor = &fixture.sensors[*index];
        cost += sensor.deployment_cost_units;
        mass += sensor.mass_g;
        power += sensor.power_mw;
    }
    (cost, mass, power)
}

fn select_robust_d_opt(fixture: &Fixture) -> AuditResult<RobustDSelection> {
    let mut best: Option<(f64, Vec<usize>, Vec<FailureScenario>)> = None;
    for indices in enumerate_combinations(
        fixture.sensors.len(),
        fixture.constraints.robust_e_opt_sensor_count,
    ) {
        let (_worst_e, worst_d, scenarios) = robust_score(fixture, &indices)?;
        let Some(score) = worst_d else {
            continue;
        };
        let is_better = best.as_ref().is_none_or(|(best_score, best_indices, _)| {
            score > *best_score + SCORE_TOLERANCE
                || ((score - *best_score).abs() <= SCORE_TOLERANCE
                    && sensor_ids(fixture, &indices) < sensor_ids(fixture, best_indices))
        });
        if is_better {
            best = Some((score, indices, scenarios));
        }
    }
    let Some((score, indices, scenarios)) = best else {
        return fail("no robust full-rank D-optimal sensor subset exists");
    };
    let (cost, mass, power) = totals(fixture, &indices);
    Ok(RobustDSelection {
        sensor_ids: sensor_ids(fixture, &indices),
        objective: "MAX_WORST_CASE_LOG_DETERMINANT_SINGLE_AND_GROUP_DROPOUT".to_owned(),
        worst_case_log_determinant: score,
        scenarios,
        total_deployment_cost_units: cost,
        total_mass_g: mass,
        total_power_mw: power,
    })
}

fn select_robust_e_opt(fixture: &Fixture) -> AuditResult<RobustSelection> {
    let mut best: Option<(f64, Vec<usize>, Vec<FailureScenario>)> = None;
    for indices in enumerate_combinations(
        fixture.sensors.len(),
        fixture.constraints.robust_e_opt_sensor_count,
    ) {
        let (score, _worst_log_determinant, scenarios) = robust_score(fixture, &indices)?;
        let is_better = best.as_ref().is_none_or(|(best_score, best_indices, _)| {
            score > *best_score + SCORE_TOLERANCE
                || ((score - *best_score).abs() <= SCORE_TOLERANCE
                    && sensor_ids(fixture, &indices) < sensor_ids(fixture, best_indices))
        });
        if is_better {
            best = Some((score, indices, scenarios));
        }
    }
    let Some((score, indices, scenarios)) = best else {
        return fail("no robust sensor subset was enumerated");
    };
    if score <= RANK_TOLERANCE {
        return fail("no robust sensor subset preserves full target-parameter rank");
    }
    let (cost, mass, power) = totals(fixture, &indices);
    Ok(RobustSelection {
        sensor_ids: sensor_ids(fixture, &indices),
        objective: "MAX_WORST_CASE_MINIMUM_EIGENVALUE_SINGLE_AND_GROUP_DROPOUT".to_owned(),
        worst_case_minimum_eigenvalue: score,
        scenarios,
        total_deployment_cost_units: cost,
        total_mass_g: mass,
        total_power_mw: power,
    })
}

fn select_minimum_cost_robust(
    fixture: &Fixture,
) -> AuditResult<Option<MinimumCostRobustSelection>> {
    let required_information = fixture.constraints.minimum_robust_minimum_eigenvalue;
    let minimum_sensor_count = (fixture.constraints.sensor_dropout_budget + 2).max(3);
    let mut best: Option<(f64, f64, f64, Vec<usize>, f64, Vec<FailureScenario>)> = None;

    for count in minimum_sensor_count..=fixture.sensors.len() {
        for indices in enumerate_combinations(fixture.sensors.len(), count) {
            let (worst_case_eigenvalue, _worst_case_log_determinant, scenarios) =
                robust_score(fixture, &indices)?;
            if worst_case_eigenvalue + SCORE_TOLERANCE < required_information {
                continue;
            }
            let (cost, mass, power) = totals(fixture, &indices);
            let is_better = best.as_ref().is_none_or(
                |(best_cost, best_mass, best_power, best_indices, _, _)| {
                    cost < *best_cost - SCORE_TOLERANCE
                        || ((cost - *best_cost).abs() <= SCORE_TOLERANCE
                            && (mass < *best_mass - SCORE_TOLERANCE
                                || ((mass - *best_mass).abs() <= SCORE_TOLERANCE
                                    && (power < *best_power - SCORE_TOLERANCE
                                        || ((power - *best_power).abs() <= SCORE_TOLERANCE
                                            && sensor_ids(fixture, &indices)
                                                < sensor_ids(fixture, best_indices))))))
                },
            );
            if is_better {
                best = Some((cost, mass, power, indices, worst_case_eigenvalue, scenarios));
            }
        }
    }

    let Some((cost, mass, power, indices, achieved, scenarios)) = best else {
        return Ok(None);
    };
    Ok(Some(MinimumCostRobustSelection {
        sensor_ids: sensor_ids(fixture, &indices),
        objective: "MINIMIZE_DEPLOYMENT_COST_SUBJECT_TO_ROBUST_EIGENVALUE_FLOOR".to_owned(),
        required_minimum_eigenvalue: required_information,
        achieved_worst_case_minimum_eigenvalue: achieved,
        total_deployment_cost_units: cost,
        total_mass_g: mass,
        total_power_mw: power,
        scenarios,
    }))
}

fn audit(fixture: &Fixture, source: &[u8]) -> AuditResult<Audit> {
    validate_fixture(fixture)?;
    let diagonal_e_opt = select_e_opt(fixture, true)?;
    let correlated_d_opt = select_d_opt(fixture)?;
    let correlated_e_opt = select_e_opt(fixture, false)?;
    let robust_correlated_d_opt = select_robust_d_opt(fixture)?;
    let robust_correlated_e_opt = select_robust_e_opt(fixture)?;
    let cost_constrained_robust = select_minimum_cost_robust(fixture)?;

    Ok(Audit {
        schema_version: "aero-correlated-sensor-information-result/0.1.0",
        problem_id: fixture.problem_id.clone(),
        source_sha256: sha256_hex(source),
        source_bytes: source.len(),
        sensor_candidate_count: fixture.sensors.len(),
        covariance_channel_count: fixture.noise_covariance.channel_ids.len(),
        covariance_positive_definite: true,
        covariance_correlated_pairs: (0..fixture.noise_covariance.matrix.len())
            .flat_map(|i| {
                ((i + 1)..fixture.noise_covariance.matrix.len()).map(move |j| (i, j))
            })
            .filter(|(i, j)| fixture.noise_covariance.matrix[*i][*j].abs() > 1.0e-12)
            .count(),
        diagonal_e_opt,
        correlated_d_opt,
        correlated_e_opt,
        robust_correlated_d_opt,
        robust_correlated_e_opt,
        cost_constrained_robust,
        diagonal_to_correlated_e_opt_changed: select_e_opt(fixture, true)?.sensor_ids
            != select_e_opt(fixture, false)?.sensor_ids,
        model_claim_ceiling: CLAIM_CEILING,
        aircraft_feasibility: "NOT_ASSESSED",
        dynamic_observation_status: "NOT_ASSESSED",
        physical_calibration_status: "NOT_ASSESSED",
    })
}

fn main() {
    let baseline: Fixture = match serde_json::from_slice(FIXTURE) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("correlated_sensor_baseline_fixture_parse_error={error}");
            std::process::exit(2);
        }
    };
    let baseline_result = match audit(&baseline, FIXTURE) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("correlated_sensor_baseline_audit_error={error}");
            std::process::exit(2);
        }
    };
    let joint: Fixture = match serde_json::from_slice(JOINT_FIXTURE) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("correlated_sensor_joint_fixture_parse_error={error}");
            std::process::exit(2);
        }
    };
    let joint_result = match audit(&joint, JOINT_FIXTURE) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("correlated_sensor_joint_audit_error={error}");
            std::process::exit(2);
        }
    };
    let bundle = serde_json::json!({
        "baseline": baseline_result,
        "six_channel_failure": joint_result,
    });
    match serde_json::to_string_pretty(&bundle) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("correlated_sensor_serialize_error={error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_value() -> Fixture {
        serde_json::from_slice(FIXTURE).expect("locked synthetic fixture must parse")
    }

    #[test]
    fn correlated_known_answers_match_frozen_results() {
        let fixture = fixture_value();
        let result = audit(&fixture, FIXTURE).expect("fixture must audit");
        assert_eq!(
            result.diagonal_e_opt.sensor_ids,
            fixture.expected.diagonal_e_opt_sensor_ids
        );
        assert!(
            (result.diagonal_e_opt.score - fixture.expected.diagonal_e_opt_minimum_eigenvalue)
                .abs()
                < 1.0e-10
        );
        assert_eq!(
            result.correlated_d_opt.sensor_ids,
            fixture.expected.correlated_d_opt_sensor_ids
        );
        assert!(
            (result.correlated_d_opt.score - fixture.expected.correlated_d_opt_determinant).abs()
                < 1.0e-9
        );
        assert_eq!(
            result.correlated_e_opt.sensor_ids,
            fixture.expected.correlated_e_opt_sensor_ids
        );
        assert!(
            (result.correlated_e_opt.score - fixture.expected.correlated_e_opt_minimum_eigenvalue)
                .abs()
                < 1.0e-10
        );
        assert_eq!(
            result.robust_correlated_d_opt.sensor_ids,
            fixture.expected.robust_d_opt_sensor_ids
        );
        assert!(
            (result.robust_correlated_d_opt.worst_case_log_determinant
                - fixture.expected.robust_d_opt_worst_case_log_determinant)
                .abs()
                < 1.0e-9
        );
        assert_eq!(
            result.robust_correlated_e_opt.sensor_ids,
            fixture.expected.robust_e_opt_sensor_ids
        );
        assert!(
            (result.robust_correlated_e_opt.worst_case_minimum_eigenvalue
                - fixture.expected.robust_e_opt_worst_case_minimum_eigenvalue)
                .abs()
                < 1.0e-9
        );
        let minimum_cost = result
            .cost_constrained_robust
            .expect("baseline must have a cost-feasible robust design");
        assert_eq!(
            minimum_cost.sensor_ids,
            fixture.expected.minimum_cost_robust_sensor_ids
        );
        assert!(
            (minimum_cost.achieved_worst_case_minimum_eigenvalue
                - fixture
                    .expected
                    .minimum_cost_robust_worst_case_minimum_eigenvalue)
                .abs()
                < 1.0e-9
        );
        assert!(
            (minimum_cost.total_deployment_cost_units
                - fixture
                    .expected
                    .minimum_cost_robust_total_deployment_cost_units)
                .abs()
                < 1.0e-9
        );
    }

    #[test]
    fn six_channel_covariance_and_common_cause_known_answers_match() {
        let fixture: Fixture =
            serde_json::from_slice(JOINT_FIXTURE).expect("joint synthetic fixture must parse");
        let result = audit(&fixture, JOINT_FIXTURE).expect("joint fixture must audit");

        assert_eq!(result.sensor_candidate_count, 6);
        assert_eq!(
            result.diagonal_e_opt.sensor_ids,
            vec!["S_A", "S_B", "S_C", "S_D"]
        );
        assert!((result.diagonal_e_opt.score - 10.0).abs() < 1.0e-10);
        assert_eq!(
            result.correlated_d_opt.sensor_ids,
            vec!["S_A", "S_C", "S_E", "S_F"]
        );
        assert_eq!(
            result.correlated_e_opt.sensor_ids,
            vec!["S_A", "S_B", "S_E", "S_F"]
        );
        assert_eq!(
            result.robust_correlated_d_opt.sensor_ids,
            vec!["S_A", "S_C", "S_E", "S_F"]
        );
        assert!(
            (result.robust_correlated_d_opt.worst_case_log_determinant - 3.2252963926711233).abs()
                < 1.0e-9
        );
        assert_eq!(
            result.robust_correlated_e_opt.sensor_ids,
            vec!["S_A", "S_C", "S_E", "S_F"]
        );
        assert!(
            (result.robust_correlated_e_opt.worst_case_minimum_eigenvalue - 2.9060881776439027)
                .abs()
                < 1.0e-9
        );
        assert_eq!(result.robust_correlated_e_opt.scenarios.len(), 7);
        assert!(
            result
                .robust_correlated_e_opt
                .scenarios
                .iter()
                .all(|scenario| {
                    scenario.effective_rank == 2 && scenario.minimum_eigenvalue > RANK_TOLERANCE
                })
        );
        let minimum_cost = result
            .cost_constrained_robust
            .expect("joint fixture must have a minimum-cost robust design");
        assert_eq!(minimum_cost.sensor_ids, vec!["S_A", "S_C", "S_E"]);
        assert!((minimum_cost.required_minimum_eigenvalue - 1.5).abs() < 1.0e-12);
        assert!(
            (minimum_cost.achieved_worst_case_minimum_eigenvalue - 1.5036392172580708).abs()
                < 1.0e-9
        );
        assert!((minimum_cost.total_deployment_cost_units - 4.2).abs() < 1.0e-9);
        assert_eq!(minimum_cost.total_mass_g, 48.0);
        assert_eq!(minimum_cost.total_power_mw, 12.0);
        assert_eq!(minimum_cost.scenarios.len(), 6);

        // The nominal E-optimal set A+B+E+F is not robust because PWR_Z
        // removes E/F and leaves only one parameter direction.
        let nominal_e_indices = vec![0, 1, 4, 5];
        let (worst_e, worst_d, _) = robust_score(&fixture, &nominal_e_indices).unwrap();
        assert_eq!(worst_e, 0.0);
        assert!(worst_d.is_none());
    }

    #[test]
    fn minimum_cost_selector_abstains_when_information_floor_is_unreachable() {
        let mut fixture = fixture_value();
        fixture.constraints.minimum_robust_minimum_eigenvalue = 1.0e9;
        let result = audit(&fixture, FIXTURE).expect("otherwise valid fixture must audit");
        assert!(result.cost_constrained_robust.is_none());
    }

    #[test]
    fn full_covariance_changes_e_optimal_sensor_pair() {
        let fixture = fixture_value();
        let diagonal = select_e_opt(&fixture, true).unwrap();
        let correlated = select_e_opt(&fixture, false).unwrap();
        assert_eq!(diagonal.sensor_ids, vec!["S1", "S2"]);
        assert_eq!(correlated.sensor_ids, vec!["S3", "S4"]);
    }

    #[test]
    fn shared_group_dropout_is_in_the_robust_result() {
        let fixture = fixture_value();
        let robust = select_robust_e_opt(&fixture).unwrap();
        assert!(robust.scenarios.iter().any(|scenario| {
            scenario.scenario_id == "GROUP_DROP:POWER_A"
                && scenario.surviving_sensor_ids == vec!["S3", "S4"]
        }));
        assert!(robust.scenarios.iter().any(|scenario| {
            scenario.scenario_id == "GROUP_DROP:INTERROGATOR_B"
                && scenario.surviving_sensor_ids == vec!["S1", "S2"]
        }));
    }

    #[test]
    fn non_symmetric_covariance_is_rejected() {
        let mut fixture = fixture_value();
        fixture.noise_covariance.matrix[0][1] = 0.1;
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn non_positive_definite_covariance_is_rejected_without_jitter() {
        let mut fixture = fixture_value();
        fixture.noise_covariance.matrix[0][1] = 1.0;
        fixture.noise_covariance.matrix[1][0] = 1.0;
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn unadmitted_operator_does_not_fall_back_to_identity() {
        let mut fixture = fixture_value();
        fixture.sensors[0].operator_status = "UNKNOWN".to_owned();
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn invalid_operator_digest_is_rejected() {
        let mut fixture = fixture_value();
        fixture.sensors[0].operator_sha256 = "not-a-digest".to_owned();
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn expired_calibration_is_rejected() {
        let mut fixture = fixture_value();
        fixture.sensors[0].calibration_status = "EXPIRED".to_owned();
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn robust_output_preserves_rank_after_every_declared_failure() {
        let fixture = fixture_value();
        let robust = select_robust_e_opt(&fixture).unwrap();
        assert_eq!(robust.scenarios.len(), 6);
        assert!(robust.scenarios.iter().all(|scenario| {
            scenario.effective_rank == 2 && scenario.minimum_eigenvalue > RANK_TOLERANCE
        }));
    }

    #[test]
    fn output_never_promotes_aircraft_or_dynamic_claims() {
        let fixture = fixture_value();
        let result = audit(&fixture, FIXTURE).unwrap();
        assert_eq!(result.aircraft_feasibility, "NOT_ASSESSED");
        assert_eq!(result.dynamic_observation_status, "NOT_ASSESSED");
        assert_eq!(result.physical_calibration_status, "NOT_ASSESSED");
    }
}
