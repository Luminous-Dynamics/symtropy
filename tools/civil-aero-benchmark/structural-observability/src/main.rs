// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Synthetic sensor-placement and observability oracle.
// This is a small linear Fisher-information benchmark, not an aircraft SHM
// system, structural model, flight sensor design, or safety/certification tool.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const FIXTURE: &str = include_str!("../data/synthetic-wing-observability-v0.1.json");
const CLAIM_CEILING: &str = "synthetic_linear_fisher_observability_only";
const RANK_TOLERANCE: f64 = 1.0e-10;
const SCORE_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct SensorCandidate {
    sensor_id: String,
    sensor_type: String,
    span_eta: f64,
    chord_x_over_c: f64,
    sensitivity: [f64; 2],
    noise_standard_uncertainty: f64,
    calibration_admitted: bool,
    calibration_reference_id: String,
    deployment_cost_units: f64,
    mass_g: f64,
    power_mw: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct ModelAssumptions {
    parameter_names: Vec<String>,
    observation_model: String,
    noise_model: String,
    sensitivity_units: String,
    noise_standard_uncertainty_units: String,
    physical_aircraft_model: bool,
    calibration_model: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Constraints {
    d_opt_sensor_count: usize,
    e_opt_sensor_count: usize,
    robust_e_opt_sensor_count: usize,
    sensor_dropout_budget: usize,
    maximum_total_cost_units: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct Expected {
    d_opt_sensor_ids: Vec<String>,
    d_opt_determinant: f64,
    e_opt_sensor_ids: Vec<String>,
    e_opt_minimum_eigenvalue: f64,
    robust_e_opt_sensor_ids: Vec<String>,
    robust_e_opt_worst_case_minimum_eigenvalue: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    schema_version: String,
    problem_id: String,
    claim_ceiling: String,
    model_assumptions: ModelAssumptions,
    constraints: Constraints,
    sensors: Vec<SensorCandidate>,
    expected: Expected,
}

#[derive(Clone, Debug, Serialize)]
struct InformationMetrics {
    fisher_information: [[f64; 2]; 2],
    determinant: f64,
    log_determinant: Option<f64>,
    eigenvalues_ascending: [f64; 2],
    effective_rank: usize,
}

#[derive(Clone, Debug, Serialize)]
struct DropoutScenario {
    dropped_sensor_id: String,
    surviving_sensor_ids: Vec<String>,
    minimum_eigenvalue: f64,
    effective_rank: usize,
}

#[derive(Clone, Debug, Serialize)]
struct Selection {
    objective: String,
    selected_sensor_ids: Vec<String>,
    objective_value: f64,
    information: InformationMetrics,
    total_cost_units: f64,
    total_mass_g: f64,
    total_power_mw: f64,
    worst_case_dropout: Option<DropoutScenario>,
}

#[derive(Clone, Debug, Serialize)]
struct AuditReport {
    schema_version: String,
    problem_id: String,
    fixture_sha256: String,
    model_assumptions: ModelAssumptions,
    d_optimal: Selection,
    e_optimal: Selection,
    dropout_robust_e_optimal: Selection,
    physical_aircraft_model: bool,
    aircraft_feasibility: &'static str,
    claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuditError(String);

impl std::fmt::Display for AuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AuditError {}

type AuditResult<T> = Result<T, AuditError>;

fn fail<T>(message: impl Into<String>) -> AuditResult<T> {
    Err(AuditError(message.into()))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    actual.is_finite()
        && expected.is_finite()
        && (actual - expected).abs() <= tolerance * expected.abs().max(1.0)
}

fn validate_fixture(fixture: &Fixture) -> AuditResult<()> {
    if fixture.schema_version != "aero-structural-observability-fixture/0.1.0" {
        return fail("unsupported observability fixture schema");
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        return fail("fixture claim ceiling was changed");
    }
    if fixture.problem_id.trim().is_empty() {
        return fail("problem_id must be non-empty");
    }
    if fixture.model_assumptions.physical_aircraft_model {
        return fail("synthetic fixture cannot claim to be a physical aircraft model");
    }
    if fixture.model_assumptions.parameter_names.len() != 2 {
        return fail("v0.1 oracle requires exactly two declared target parameters");
    }
    if fixture.model_assumptions.observation_model != "linearized_static_sensitivity"
        || fixture.model_assumptions.noise_model
            != "independent_zero_mean_gaussian_sensor_noise"
        || fixture.model_assumptions.sensitivity_units != "dimensionless_normalized"
        || fixture.model_assumptions.noise_standard_uncertainty_units
            != "dimensionless_normalized"
        || fixture.model_assumptions.calibration_model != "synthetic_reference_only"
    {
        return fail("fixture assumptions are outside the admitted synthetic oracle");
    }
    if fixture.sensors.len() < 3 {
        return fail("at least three synthetic sensor candidates are required");
    }
    if fixture.constraints.d_opt_sensor_count == 0
        || fixture.constraints.e_opt_sensor_count == 0
        || fixture.constraints.robust_e_opt_sensor_count <= fixture.constraints.sensor_dropout_budget
    {
        return fail("sensor selection/dropout counts are invalid");
    }
    if !finite_positive(fixture.constraints.maximum_total_cost_units) {
        return fail("maximum_total_cost_units must be finite and positive");
    }

    let mut ids = BTreeSet::new();
    let mut previous_id: Option<&str> = None;
    for sensor in &fixture.sensors {
        if sensor.sensor_id.trim().is_empty() || !ids.insert(sensor.sensor_id.as_str()) {
            return fail(format!("empty or duplicate sensor_id: {}", sensor.sensor_id));
        }
        if previous_id.is_some_and(|previous| previous >= sensor.sensor_id.as_str()) {
            return fail("sensor candidates must be strictly ordered by sensor_id");
        }
        previous_id = Some(sensor.sensor_id.as_str());
        if sensor.sensor_type.trim().is_empty() {
            return fail(format!("{}: sensor_type is missing", sensor.sensor_id));
        }
        if !sensor.span_eta.is_finite()
            || !(0.0..=1.0).contains(&sensor.span_eta)
            || !sensor.chord_x_over_c.is_finite()
            || !(0.0..=1.0).contains(&sensor.chord_x_over_c)
        {
            return fail(format!("{}: normalized sensor coordinate outside [0, 1]", sensor.sensor_id));
        }
        if !sensor.sensitivity.iter().all(|value| value.is_finite()) {
            return fail(format!("{}: sensitivity vector contains non-finite values", sensor.sensor_id));
        }
        if !finite_positive(sensor.noise_standard_uncertainty) {
            return fail(format!("{}: noise standard uncertainty must be finite and positive", sensor.sensor_id));
        }
        if !sensor.calibration_admitted || sensor.calibration_reference_id.trim().is_empty() {
            return fail(format!("{}: synthetic calibration reference is not admitted", sensor.sensor_id));
        }
        if !finite_positive(sensor.deployment_cost_units)
            || !sensor.mass_g.is_finite()
            || sensor.mass_g < 0.0
            || !sensor.power_mw.is_finite()
            || sensor.power_mw < 0.0
        {
            return fail(format!("{}: invalid synthetic deployment cost/mass/power", sensor.sensor_id));
        }
    }

    for count in [
        fixture.constraints.d_opt_sensor_count,
        fixture.constraints.e_opt_sensor_count,
        fixture.constraints.robust_e_opt_sensor_count,
    ] {
        if count > fixture.sensors.len() {
            return fail("sensor budget exceeds candidate count");
        }
    }
    Ok(())
}

fn information_metrics(sensors: &[&SensorCandidate]) -> InformationMetrics {
    let mut f11 = 0.0;
    let mut f12 = 0.0;
    let mut f22 = 0.0;

    for sensor in sensors {
        let variance = sensor.noise_standard_uncertainty.powi(2);
        let weight = 1.0 / variance;
        let h1 = sensor.sensitivity[0];
        let h2 = sensor.sensitivity[1];
        f11 += h1 * h1 * weight;
        f12 += h1 * h2 * weight;
        f22 += h2 * h2 * weight;
    }

    let determinant = f11 * f22 - f12 * f12;
    let discriminant = ((f11 - f22).powi(2) + 4.0 * f12.powi(2)).sqrt();
    let trace = f11 + f22;
    let minimum = ((trace - discriminant) / 2.0).max(0.0);
    let maximum = ((trace + discriminant) / 2.0).max(0.0);
    let rank = usize::from(minimum > RANK_TOLERANCE)
        + usize::from(maximum > RANK_TOLERANCE);

    InformationMetrics {
        fisher_information: [[f11, f12], [f12, f22]],
        determinant,
        log_determinant: if determinant > 0.0 {
            Some(determinant.ln())
        } else {
            None
        },
        eigenvalues_ascending: [minimum, maximum],
        effective_rank: rank,
    }
}

fn combinations(length: usize, choose: usize) -> Vec<Vec<usize>> {
    fn visit(
        length: usize,
        choose: usize,
        start: usize,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == choose {
            output.push(current.clone());
            return;
        }
        let needed = choose - current.len();
        if length < needed || start > length - needed {
            return;
        }
        for index in start..=(length - needed) {
            current.push(index);
            visit(length, choose, index + 1, current, output);
            current.pop();
        }
    }

    let mut output = Vec::new();
    visit(length, choose, 0, &mut Vec::new(), &mut output);
    output
}

fn selected_ids(sensors: &[SensorCandidate], indices: &[usize]) -> Vec<String> {
    indices.iter().map(|index| sensors[*index].sensor_id.clone()).collect()
}

fn subset_sensors<'a>(
    sensors: &'a [SensorCandidate],
    indices: &[usize],
) -> Vec<&'a SensorCandidate> {
    indices.iter().map(|index| &sensors[*index]).collect()
}

fn totals(sensors: &[&SensorCandidate]) -> (f64, f64, f64) {
    sensors.iter().fold((0.0, 0.0, 0.0), |acc, sensor| {
        (
            acc.0 + sensor.deployment_cost_units,
            acc.1 + sensor.mass_g,
            acc.2 + sensor.power_mw,
        )
    })
}

fn robust_score(
    sensors: &[SensorCandidate],
    indices: &[usize],
    dropout_budget: usize,
) -> AuditResult<(f64, Option<DropoutScenario>)> {
    if dropout_budget >= indices.len() {
        return fail("dropout budget must be smaller than selected sensor count");
    }
    let survivor_combinations = combinations(indices.len(), indices.len() - dropout_budget);
    if survivor_combinations.is_empty() {
        return fail("no sensor survival scenarios could be constructed");
    }

    let mut worst_value = f64::INFINITY;
    let mut worst_scenario: Option<DropoutScenario> = None;

    for survivor_positions in survivor_combinations {
        let survivor_indices = survivor_positions
            .iter()
            .map(|position| indices[*position])
            .collect::<Vec<_>>();
        let survivors = subset_sensors(sensors, &survivor_indices);
        let metrics = information_metrics(&survivors);
        let value = metrics.eigenvalues_ascending[0];

        let dropped = indices
            .iter()
            .find(|index| !survivor_indices.contains(index))
            .map(|index| sensors[*index].sensor_id.clone())
            .unwrap_or_else(|| "NONE".to_owned());
        let scenario = DropoutScenario {
            dropped_sensor_id: dropped,
            surviving_sensor_ids: selected_ids(sensors, &survivor_indices),
            minimum_eigenvalue: value,
            effective_rank: metrics.effective_rank,
        };

        let replace = value < worst_value - SCORE_TOLERANCE
            || ((value - worst_value).abs() <= SCORE_TOLERANCE
                && worst_scenario
                    .as_ref()
                    .is_none_or(|current| scenario.dropped_sensor_id < current.dropped_sensor_id));
        if replace {
            worst_value = value;
            worst_scenario = Some(scenario);
        }
    }

    Ok((worst_value, worst_scenario))
}

#[derive(Clone, Copy)]
enum Objective {
    DOptimal,
    EOptimal,
    RobustEOptimal,
}

fn objective_name(objective: Objective) -> &'static str {
    match objective {
        Objective::DOptimal => "D_OPTIMAL_LOG_DETERMINANT",
        Objective::EOptimal => "E_OPTIMAL_MINIMUM_EIGENVALUE",
        Objective::RobustEOptimal => "DROPOUT_ROBUST_E_OPTIMAL_MINIMUM_EIGENVALUE",
    }
}

fn candidate_score(
    sensors: &[SensorCandidate],
    indices: &[usize],
    objective: Objective,
    dropout_budget: usize,
) -> AuditResult<(f64, Option<DropoutScenario>)> {
    match objective {
        Objective::DOptimal => {
            let selected = subset_sensors(sensors, indices);
            let metrics = information_metrics(&selected);
            Ok((metrics.log_determinant.unwrap_or(f64::NEG_INFINITY), None))
        }
        Objective::EOptimal => {
            let selected = subset_sensors(sensors, indices);
            let metrics = information_metrics(&selected);
            Ok((metrics.eigenvalues_ascending[0], None))
        }
        Objective::RobustEOptimal => robust_score(sensors, indices, dropout_budget),
    }
}

fn selection_for(
    sensors: &[SensorCandidate],
    indices: &[usize],
    objective: Objective,
    score: f64,
    worst_case_dropout: Option<DropoutScenario>,
) -> Selection {
    let selected = subset_sensors(sensors, indices);
    let metrics = information_metrics(&selected);
    let (cost, mass, power) = totals(&selected);
    Selection {
        objective: objective_name(objective).to_owned(),
        selected_sensor_ids: selected_ids(sensors, indices),
        objective_value: score,
        information: metrics,
        total_cost_units: cost,
        total_mass_g: mass,
        total_power_mw: power,
        worst_case_dropout,
    }
}

fn best_selection(
    fixture: &Fixture,
    sensor_count: usize,
    objective: Objective,
) -> AuditResult<Selection> {
    let mut best_indices: Option<Vec<usize>> = None;
    let mut best_score = f64::NEG_INFINITY;
    let mut best_worst_case = None;

    for indices in combinations(fixture.sensors.len(), sensor_count) {
        let selected = subset_sensors(&fixture.sensors, &indices);
        let (cost, _, _) = totals(&selected);
        if cost > fixture.constraints.maximum_total_cost_units + SCORE_TOLERANCE {
            continue;
        }

        let (score, worst_case) = candidate_score(
            &fixture.sensors,
            &indices,
            objective,
            fixture.constraints.sensor_dropout_budget,
        )?;
        let ids = selected_ids(&fixture.sensors, &indices);
        let is_better = score > best_score + SCORE_TOLERANCE
            || ((score - best_score).abs() <= SCORE_TOLERANCE
                && best_indices.as_ref().is_none_or(|current| {
                    ids < selected_ids(&fixture.sensors, current)
                }));
        if is_better {
            best_score = score;
            best_indices = Some(indices);
            best_worst_case = worst_case;
        }
    }

    let Some(indices) = best_indices else {
        return fail(format!(
            "no feasible {} selection fits the sensor/cost constraints",
            objective_name(objective)
        ));
    };
    if !best_score.is_finite() {
        return fail(format!(
            "no full-information {} placement exists; rank-deficient designs are not regularized into a pass",
            objective_name(objective)
        ));
    }
    Ok(selection_for(
        &fixture.sensors,
        &indices,
        objective,
        best_score,
        best_worst_case,
    ))
}

fn verify_expected(fixture: &Fixture, report: &AuditReport) -> AuditResult<()> {
    let expected = &fixture.expected;
    if report.d_optimal.selected_sensor_ids != expected.d_opt_sensor_ids {
        return fail("D-optimal selected sensor IDs differ from the locked oracle");
    }
    if !close(
        report.d_optimal.information.determinant,
        expected.d_opt_determinant,
        1.0e-10,
    ) {
        return fail("D-optimal determinant differs from the locked oracle");
    }
    if report.e_optimal.selected_sensor_ids != expected.e_opt_sensor_ids {
        return fail("E-optimal selected sensor IDs differ from the locked oracle");
    }
    if !close(
        report.e_optimal.information.eigenvalues_ascending[0],
        expected.e_opt_minimum_eigenvalue,
        1.0e-10,
    ) {
        return fail("E-optimal minimum eigenvalue differs from the locked oracle");
    }
    if report.dropout_robust_e_optimal.selected_sensor_ids != expected.robust_e_opt_sensor_ids {
        return fail("dropout-robust E-optimal sensor IDs differ from the locked oracle");
    }
    if !close(
        report.dropout_robust_e_optimal.objective_value,
        expected.robust_e_opt_worst_case_minimum_eigenvalue,
        1.0e-10,
    ) {
        return fail("dropout-robust E-optimal score differs from the locked oracle");
    }
    Ok(())
}

fn audit_fixture(fixture: &Fixture, fixture_bytes: &[u8]) -> AuditResult<AuditReport> {
    validate_fixture(fixture)?;
    let d_optimal = best_selection(
        fixture,
        fixture.constraints.d_opt_sensor_count,
        Objective::DOptimal,
    )?;
    let e_optimal = best_selection(
        fixture,
        fixture.constraints.e_opt_sensor_count,
        Objective::EOptimal,
    )?;
    let robust_e_optimal = best_selection(
        fixture,
        fixture.constraints.robust_e_opt_sensor_count,
        Objective::RobustEOptimal,
    )?;
    let report = AuditReport {
        schema_version: "aero-structural-observability-audit/0.1.0".to_owned(),
        problem_id: fixture.problem_id.clone(),
        fixture_sha256: sha256_hex(fixture_bytes),
        model_assumptions: fixture.model_assumptions.clone(),
        d_optimal,
        e_optimal,
        dropout_robust_e_optimal: robust_e_optimal,
        physical_aircraft_model: false,
        aircraft_feasibility: "NOT_ASSESSED",
        claim_ceiling: CLAIM_CEILING,
    };
    verify_expected(fixture, &report)?;
    Ok(report)
}

fn main() {
    let fixture: Fixture = match serde_json::from_str(FIXTURE) {
        Ok(fixture) => fixture,
        Err(error) => {
            eprintln!("observability_fixture_parse_error={error}");
            std::process::exit(2);
        }
    };
    let report = match audit_fixture(&fixture, FIXTURE.as_bytes()) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("observability_audit_error={error}");
            std::process::exit(2);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("JSON serialization")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(FIXTURE).expect("locked synthetic fixture must parse")
    }

    #[test]
    fn locked_sensor_selection_answers_match() {
        let fixture = fixture();
        let report = audit_fixture(&fixture, FIXTURE.as_bytes()).unwrap();
        verify_expected(&fixture, &report).unwrap();
        assert_eq!(report.d_optimal.selected_sensor_ids, vec!["S3", "S4"]);
        assert_eq!(report.e_optimal.selected_sensor_ids, vec!["S1", "S2"]);
        assert_eq!(
            report.dropout_robust_e_optimal.selected_sensor_ids,
            vec!["S1", "S2", "S4"]
        );
    }

    #[test]
    fn s3_s4_fisher_matrix_has_known_answer() {
        let fixture = fixture();
        let selected = vec![&fixture.sensors[2], &fixture.sensors[3]];
        let metrics = information_metrics(&selected);
        assert!(close(metrics.fisher_information[0][0], 1.5, 1.0e-12));
        assert!(close(metrics.fisher_information[0][1], -0.5, 1.0e-12));
        assert!(close(metrics.fisher_information[1][1], 1.5, 1.0e-12));
        assert!(close(metrics.determinant, 2.0, 1.0e-12));
        assert_eq!(metrics.eigenvalues_ascending, [1.0, 2.0]);
        assert_eq!(metrics.effective_rank, 2);
    }

    #[test]
    fn rank_deficiency_is_reported_without_regularization() {
        let fixture = fixture();
        let selected = vec![&fixture.sensors[0], &fixture.sensors[4]];
        let metrics = information_metrics(&selected);
        assert_eq!(metrics.effective_rank, 1);
        assert!(metrics.log_determinant.is_none() || metrics.log_determinant.unwrap().is_infinite());
        assert!(metrics.eigenvalues_ascending[0] <= RANK_TOLERANCE);
    }

    #[test]
    fn duplicate_sensor_id_is_rejected() {
        let mut fixture = fixture();
        fixture.sensors[1].sensor_id = fixture.sensors[0].sensor_id.clone();
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn zero_or_nonfinite_noise_uncertainty_is_rejected() {
        let mut fixture = fixture();
        fixture.sensors[0].noise_standard_uncertainty = 0.0;
        assert!(validate_fixture(&fixture).is_err());
        fixture.sensors[0].noise_standard_uncertainty = f64::NAN;
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn unadmitted_calibration_is_rejected() {
        let mut fixture = fixture();
        fixture.sensors[0].calibration_admitted = false;
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn out_of_span_coordinate_is_rejected() {
        let mut fixture = fixture();
        fixture.sensors[0].span_eta = 1.01;
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn dropout_robust_selection_reports_worst_survivor_case() {
        let fixture = fixture();
        let robust = best_selection(
            &fixture,
            fixture.constraints.robust_e_opt_sensor_count,
            Objective::RobustEOptimal,
        )
        .unwrap();
        let worst = robust.worst_case_dropout.unwrap();
        assert_eq!(worst.dropped_sensor_id, "S1");
        assert_eq!(worst.surviving_sensor_ids, vec!["S2", "S4"]);
        assert!(close(
            worst.minimum_eigenvalue,
            (3.0 - 5.0_f64.sqrt()) / 2.0,
            1.0e-10
        ));
    }

    #[test]
    fn optimizer_budget_includes_cost_not_only_sensor_count() {
        let mut fixture = fixture();
        fixture.constraints.maximum_total_cost_units = 1.0;
        assert!(best_selection(&fixture, 2, Objective::DOptimal).is_err());
    }
}
