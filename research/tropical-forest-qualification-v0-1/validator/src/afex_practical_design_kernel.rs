// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Practical identifiability and cost-aware measurement design for the synthetic
//! AFEX root-pathway observation model. This is a linear-Gaussian benchmark adapter,
//! not a general Bayesian inference or ecological decision engine.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

const N: usize = 4;
const TOL: f64 = 1e-8;

#[derive(Clone, Debug)]
struct Measurement {
    id: String,
    row: Option<Vec<i128>>,
    noise_sd: f64,
    cost: u64,
}

#[derive(Clone, Debug)]
struct Metrics {
    data_rank: usize,
    fisher: Vec<Vec<f64>>,
    posterior_sd: Vec<f64>,
    weighted_variance: f64,
    information_gain_nats: f64,
    total_cost: u64,
    added_ids: Vec<String>,
}

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn number_at(value: &Value, key: &str) -> Result<f64, String> {
    let result = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key} must be a finite number"))?;
    if !result.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(result)
}

fn uint_at(value: &Value, key: &str) -> Result<u64, String> {
    value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("String write cannot fail");
    }
    output
}

fn parse_measurement(value: &Value, allow_covariate_only: bool) -> Result<Measurement, String> {
    let id = string_at(value, "measurement_id")?;
    let role = string_at(value, "role")?;
    let noise_sd = number_at(value, "noise_sd")?;
    if noise_sd <= 0.0 {
        return Err(format!("measurement {id} noise_sd must be positive"));
    }
    let cost = uint_at(value, "cost_units")?;
    let row_value = value.get("sensitivity_row")
        .ok_or_else(|| format!("measurement {id} sensitivity_row is required"))?;
    let row = if row_value.is_null() {
        if !allow_covariate_only || role != "COVARIATE_ONLY_NO_PATHWAY_FLUX_SENSITIVITY" {
            return Err(format!("measurement {id} has no row but is not a declared covariate-only assay"));
        }
        None
    } else {
        if role == "COVARIATE_ONLY_NO_PATHWAY_FLUX_SENSITIVITY" {
            return Err(format!("measurement {id} is covariate-only and cannot declare a pathway flux row"));
        }
        let values = row_value.as_array()
            .ok_or_else(|| format!("measurement {id} sensitivity_row must be an array or null"))?;
        if values.len() != N {
            return Err(format!("measurement {id} sensitivity row must have {N} columns"));
        }
        let parsed = values.iter().map(|entry| entry.as_i64()
            .map(i128::from).ok_or_else(|| format!("measurement {id} sensitivity values must be integers")))
            .collect::<Result<Vec<_>, _>>()?;
        if parsed.iter().all(|coefficient| *coefficient == 0) {
            return Err(format!("measurement {id} sensitivity row cannot be all zero; use null only for a covariate-only assay"));
        }
        Some(parsed)
    };
    Ok(Measurement { id, row, noise_sd, cost })
}

fn parse_measurements(value: &Value, allow_covariate_only: bool) -> Result<Vec<Measurement>, String> {
    let rows = value.as_array().ok_or_else(|| "measurement list must be an array".to_string())?;
    let mut ids = BTreeSet::new();
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let measurement = parse_measurement(row, allow_covariate_only)?;
        if !ids.insert(measurement.id.clone()) {
            return Err(format!("duplicate measurement ID {}", measurement.id));
        }
        result.push(measurement);
    }
    Ok(result)
}

fn matrix_from_json(value: &Value, key: &str) -> Result<Vec<Vec<f64>>, String> {
    let rows = value.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array"))?;
    if rows.len() != N {
        return Err(format!("{key} must be {N}x{N}"));
    }
    let mut matrix = Vec::with_capacity(N);
    for row in rows {
        let values = row.as_array().ok_or_else(|| format!("{key} rows must be arrays"))?;
        if values.len() != N {
            return Err(format!("{key} must be {N}x{N}"));
        }
        let mut parsed = Vec::with_capacity(N);
        for value in values {
            let number = value.as_f64().ok_or_else(|| format!("{key} entries must be numeric"))?;
            if !number.is_finite() {
                return Err(format!("{key} entries must be finite"));
            }
            parsed.push(number);
        }
        matrix.push(parsed);
    }
    Ok(matrix)
}

fn cholesky_logdet(matrix: &[Vec<f64>]) -> Result<f64, String> {
    if matrix.len() != N || matrix.iter().any(|row| row.len() != N) {
        return Err("covariance / precision matrix dimensions invalid".to_string());
    }
    for i in 0..N {
        for j in 0..N {
            if (matrix[i][j] - matrix[j][i]).abs() > TOL {
                return Err("covariance / precision matrix must be symmetric".to_string());
            }
        }
    }
    let mut lower = vec![vec![0.0; N]; N];
    let mut logdet = 0.0;
    for i in 0..N {
        for j in 0..=i {
            let mut sum = matrix[i][j];
            for k in 0..j {
                sum -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if sum <= 1e-12 || !sum.is_finite() {
                    return Err("matrix must be positive definite".to_string());
                }
                lower[i][j] = sum.sqrt();
                logdet += 2.0 * lower[i][j].ln();
            } else {
                if lower[j][j] == 0.0 {
                    return Err("matrix is not positive definite".to_string());
                }
                lower[i][j] = sum / lower[j][j];
            }
        }
    }
    Ok(logdet)
}

fn inverse(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    if matrix.len() != N || matrix.iter().any(|row| row.len() != N) {
        return Err("inverse requires a 4x4 matrix".to_string());
    }
    let mut work = vec![vec![0.0; 2 * N]; N];
    for i in 0..N {
        for j in 0..N {
            work[i][j] = matrix[i][j];
        }
        work[i][N + i] = 1.0;
    }
    for column in 0..N {
        let mut pivot = column;
        for row in column..N {
            if work[row][column].abs() > work[pivot][column].abs() {
                pivot = row;
            }
        }
        if work[pivot][column].abs() < 1e-12 {
            return Err("matrix is singular or numerically non-invertible".to_string());
        }
        work.swap(column, pivot);
        let divisor = work[column][column];
        for item in &mut work[column] {
            *item /= divisor;
        }
        for row in 0..N {
            if row == column {
                continue;
            }
            let factor = work[row][column];
            for j in 0..(2 * N) {
                work[row][j] -= factor * work[column][j];
            }
        }
    }
    let result = work.into_iter().map(|row| row[N..].to_vec()).collect::<Vec<_>>();
    if result.iter().flatten().any(|value| !value.is_finite()) {
        return Err("matrix inverse produced non-finite values".to_string());
    }
    Ok(result)
}

fn exact_data_rank(measurements: &[Measurement]) -> Result<usize, String> {
    let rows = measurements.iter().filter_map(|measurement| measurement.row.clone()).collect::<Vec<_>>();
    super::afex_root_pathway_identifiability_kernel::exact_rank(&rows, N)
}

fn fisher_information(measurements: &[Measurement]) -> Vec<Vec<f64>> {
    let mut fisher = vec![vec![0.0; N]; N];
    for measurement in measurements {
        if let Some(row) = &measurement.row {
            let variance = measurement.noise_sd * measurement.noise_sd;
            for i in 0..N {
                for j in 0..N {
                    fisher[i][j] += (row[i] as f64) * (row[j] as f64) / variance;
                }
            }
        }
    }
    fisher
}

fn calculate_metrics(
    measurements: &[Measurement],
    prior_covariance: &[Vec<f64>],
    decision_weights: &[f64],
    added_ids: Vec<String>,
) -> Result<Metrics, String> {
    let prior_logdet = cholesky_logdet(prior_covariance)?;
    if decision_weights.len() != N || decision_weights.iter().any(|weight| !weight.is_finite() || *weight < 0.0) {
        return Err("decision weights must be four finite non-negative values".to_string());
    }
    let prior_precision = inverse(prior_covariance)?;
    let fisher = fisher_information(measurements);
    let mut posterior_precision = prior_precision;
    for i in 0..N {
        for j in 0..N {
            posterior_precision[i][j] += fisher[i][j];
        }
    }
    let posterior_logdet_precision = cholesky_logdet(&posterior_precision)?;
    let posterior = inverse(&posterior_precision)?;
    let posterior_logdet = -posterior_logdet_precision;
    let information_gain_nats = 0.5 * (prior_logdet - posterior_logdet);
    let weighted_variance = (0..N).map(|i| decision_weights[i] * posterior[i][i]).sum::<f64>();
    let total_cost = measurements.iter().map(|measurement| measurement.cost).sum();
    let posterior_sd = (0..N).map(|i| posterior[i][i].max(0.0).sqrt()).collect();
    Ok(Metrics {
        data_rank: exact_data_rank(measurements)?,
        fisher,
        posterior_sd,
        weighted_variance,
        information_gain_nats,
        total_cost,
        added_ids,
    })
}

fn close(actual: f64, expected: f64, tolerance: f64) -> bool {
    (actual - expected).abs() <= tolerance
}

fn validate_metrics_against_expected(metrics: &Metrics, expected: &Value) -> Result<(), String> {
    if metrics.data_rank as u64 != uint_at(expected, "data_rank")? {
        return Err("diagnostic data-rank oracle mismatch".to_string());
    }
    if !close(metrics.weighted_variance, number_at(expected, "weighted_posterior_variance")?, 1e-8) {
        return Err("diagnostic weighted-posterior-variance oracle mismatch".to_string());
    }
    if !close(metrics.information_gain_nats, number_at(expected, "information_gain_nats")?, 1e-8) {
        return Err("diagnostic information-gain oracle mismatch".to_string());
    }
    let expected_sd = expected.get("posterior_sd").and_then(Value::as_array)
        .ok_or_else(|| "expected posterior_sd must be an array".to_string())?;
    if expected_sd.len() != N {
        return Err("expected posterior_sd must contain four values".to_string());
    }
    for (actual, expected) in metrics.posterior_sd.iter().zip(expected_sd.iter()) {
        let expected = expected.as_f64().ok_or_else(|| "posterior_sd oracle must be numeric".to_string())?;
        if !close(*actual, expected, 1e-8) {
            return Err("diagnostic posterior standard-deviation oracle mismatch".to_string());
        }
    }
    Ok(())
}

fn portfolio_json(metrics: &Metrics) -> Value {
    json!({
        "added_measurement_ids": metrics.added_ids,
        "total_cost_units": metrics.total_cost,
        "data_only_rank": metrics.data_rank,
        "weighted_posterior_variance": metrics.weighted_variance,
        "information_gain_nats": metrics.information_gain_nats,
        "posterior_standard_deviation": metrics.posterior_sd
    })
}

fn compare_id_array(actual: &[String], expected: &Value) -> Result<bool, String> {
    let values = expected.as_array().ok_or_else(|| "expected portfolio IDs must be an array".to_string())?;
    let parsed = values.iter().map(|value| value.as_str()
        .map(ToOwned::to_owned).ok_or_else(|| "portfolio ID must be a string".to_string()))
        .collect::<Result<Vec<_>,_>>()?;
    Ok(actual == parsed)
}

pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    if string_at(manifest, "schema_version")? != "0.1.0"
        || string_at(manifest, "benchmark_id")? != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest, "corpus_id")? != "AFEX-PRACTICAL-DESIGN-001"
    {
        return Err("AFEX practical-design corpus identity mismatch".to_string());
    }
    let claim_ceiling = string_at(manifest, "claim_ceiling")?;
    if !claim_ceiling.contains("All assay costs, noise levels, priors")
        || !claim_ceiling.contains("No AFEX field campaign optimum")
    {
        return Err("synthetic practical-design claim ceiling is incomplete".to_string());
    }
    let unknowns = manifest.get("unknown_parameters").and_then(Value::as_array)
        .ok_or_else(|| "unknown_parameters must be an array".to_string())?;
    let expected_unknowns = [
        "k_root_inorganic", "k_myc_inorganic", "k_myc_organic", "c_host_to_fungal"
    ];
    let actual_unknowns = unknowns.iter().map(Value::as_str).collect::<Vec<_>>();
    if actual_unknowns != expected_unknowns.iter().map(|value| Some(*value)).collect::<Vec<_>>() {
        return Err("practical-design unknown parameter order mismatch".to_string());
    }

    let prior = matrix_from_json(manifest, "prior_covariance")?;
    cholesky_logdet(&prior)?;
    let strong_prior = matrix_from_json(manifest, "strong_prior_diagnostic_covariance")?;
    cholesky_logdet(&strong_prior)?;
    let weights = manifest.get("decision_weights_diagonal").and_then(Value::as_array)
        .ok_or_else(|| "decision_weights_diagonal must be an array".to_string())?
        .iter().map(|value| value.as_f64().ok_or_else(|| "decision weights must be numeric".to_string()))
        .collect::<Result<Vec<_>,_>>()?;
    if weights.len() != N || weights.iter().any(|weight| !weight.is_finite() || *weight < 0.0) {
        return Err("decision weights must contain four finite non-negative numbers".to_string());
    }
    let weak_threshold = number_at(manifest, "posterior_sd_practical_weak_threshold")?;
    if weak_threshold <= 0.0 {
        return Err("posterior_sd_practical_weak_threshold must be positive".to_string());
    }

    let base = parse_measurements(
        manifest.get("base_measurements").ok_or_else(|| "base_measurements required".to_string())?,
        false
    )?;
    if base.is_empty() || base.iter().any(|measurement| measurement.role != "AGGREGATE_TOTAL_UPTAKE") {
        return Err("base must contain aggregate total-uptake measurements".to_string());
    }
    let candidates = parse_measurements(
        manifest.get("candidate_measurements").ok_or_else(|| "candidate_measurements required".to_string())?,
        true
    )?;
    let candidate_map = candidates.iter().map(|measurement| (measurement.id.clone(), measurement.clone()))
        .collect::<BTreeMap<_,_>>();
    let base_ids = base.iter().map(|measurement| measurement.id.clone()).collect::<BTreeSet<_>>();
    if candidates.iter().any(|measurement| base_ids.contains(&measurement.id)) {
        return Err("candidate measurement ID duplicates a base measurement ID".to_string());
    }

    let mut scenario_ids = BTreeSet::new();
    let scenarios = manifest.get("diagnostic_scenarios").and_then(Value::as_array)
        .ok_or_else(|| "diagnostic_scenarios must be an array".to_string())?;
    let mut scenario_results = Vec::new();
    let mut scenario_metrics = BTreeMap::new();

    for scenario in scenarios {
        let scenario_id = string_at(scenario, "scenario_id")?;
        if !scenario_ids.insert(scenario_id.clone()) {
            return Err(format!("duplicate diagnostic scenario {scenario_id}"));
        }
        let include = scenario.get("include_candidate_ids").and_then(Value::as_array)
            .ok_or_else(|| format!("scenario {scenario_id} include_candidate_ids must be array"))?;
        let mut measurements = base.clone();
        let mut added_ids = Vec::new();
        let mut included_ids = BTreeSet::new();
        for item in include {
            let id = item.as_str().ok_or_else(|| "candidate measurement ID must be a string".to_string())?;
            if !included_ids.insert(id.to_string()) {
                return Err(format!("scenario {scenario_id} repeats candidate measurement {id}"));
            }
            let candidate = candidate_map.get(id).ok_or_else(|| format!("unknown candidate measurement {id}"))?;
            measurements.push(candidate.clone());
            added_ids.push(id.to_string());
        }
        let noise_override = scenario.get("noise_override").and_then(Value::as_object);
        if let Some(overrides) = noise_override {
            for key in overrides.keys() {
                if !measurements.iter().any(|measurement| &measurement.id == key) {
                    return Err(format!("scenario {scenario_id} noise override references unknown measurement {key}"));
                }
            }
            for measurement in measurements.iter_mut() {
                if let Some(value) = overrides.get(&measurement.id) {
                    let noise = value.as_f64().ok_or_else(|| "noise override must be numeric".to_string())?;
                    if !noise.is_finite() || noise <= 0.0 {
                        return Err("noise override must be positive and finite".to_string());
                    }
                    measurement.noise_sd = noise;
                }
            }
        }
        let prior_key = string_at(scenario, "prior_covariance")?;
        let prior_matrix = match prior_key.as_str() {
            "prior_covariance" => &prior,
            "strong_prior_diagnostic_covariance" => &strong_prior,
            _ => return Err(format!("unsupported prior covariance key {prior_key}")),
        };
        let metrics = calculate_metrics(&measurements, prior_matrix, &weights, added_ids.clone())?;
        let expected = scenario.get("expected").ok_or_else(|| "scenario expected metrics required".to_string())?;
        validate_metrics_against_expected(&metrics, expected)?;
        if scenario_id == "HIGH_NOISE_MYCO_INORGANIC_TRACER"
            && (metrics.data_rank != 3 || metrics.posterior_sd[1] <= weak_threshold)
        {
            return Err("high-noise tracer must remain practically weak despite rank 3".to_string());
        }
        if scenario_id == "LOW_NOISE_MYCO_INORGANIC_TRACER"
            && (metrics.data_rank != 3 || metrics.posterior_sd[1] >= weak_threshold)
        {
            return Err("low-noise tracer should cross the declared synthetic precision threshold".to_string());
        }
        scenario_results.push(json!({
            "scenario_id": scenario_id,
            "added_measurement_ids": added_ids,
            "total_cost_units": metrics.total_cost,
            "data_only_rank": metrics.data_rank,
            "posterior_standard_deviation": metrics.posterior_sd,
            "weighted_posterior_variance": metrics.weighted_variance,
            "information_gain_nats": metrics.information_gain_nats,
            "fisher_information": metrics.fisher,
            "interpretation":"Synthetic linear-Gaussian diagnostic; prior and data contributions remain separately reported."
        }));
        scenario_metrics.insert(scenario_id, metrics);
    }

    let base_metrics = scenario_metrics.get("BASE_AGGREGATE_ONLY")
        .ok_or_else(|| "BASE_AGGREGATE_ONLY scenario required".to_string())?;
    let repeat_metrics = scenario_metrics.get("REPEAT_AGGREGATE_ONLY")
        .ok_or_else(|| "REPEAT_AGGREGATE_ONLY scenario required".to_string())?;
    let covariate_metrics = scenario_metrics.get("COLONISATION_COVARIATE_ONLY")
        .ok_or_else(|| "COLONISATION_COVARIATE_ONLY scenario required".to_string())?;
    let strong_metrics = scenario_metrics.get("STRONG_PRIOR_NOT_DATA_INFORMATION")
        .ok_or_else(|| "STRONG_PRIOR_NOT_DATA_INFORMATION scenario required".to_string())?;
    if repeat_metrics.data_rank != base_metrics.data_rank
        || repeat_metrics.weighted_variance >= base_metrics.weighted_variance
        || covariate_metrics.data_rank != base_metrics.data_rank
        || !close(covariate_metrics.weighted_variance, base_metrics.weighted_variance, TOL)
        || strong_metrics.data_rank != base_metrics.data_rank
        || strong_metrics.weighted_variance >= base_metrics.weighted_variance
    {
        return Err("repeat/covariate/prior adversarial relationship failed".to_string());
    }

    let budget_obj = manifest.get("budget").ok_or_else(|| "budget required".to_string())?;
    let budget = uint_at(budget_obj, "total_cost_units")?;
    let base_cost = base.iter().map(|measurement| measurement.cost).sum::<u64>();
    if base_cost > budget {
        return Err("base measurement cost already exceeds budget".to_string());
    }
    let option_count = candidates.len();
    if option_count > 20 {
        return Err("too many candidate measurements for bounded exact portfolio enumeration".to_string());
    }
    let mut portfolios = Vec::new();
    let combinations = 1usize << option_count;
    for mask in 0..combinations {
        let mut measurements = base.clone();
        let mut added_ids = Vec::new();
        for (index, candidate) in candidates.iter().enumerate() {
            if (mask & (1usize << index)) != 0 {
                measurements.push(candidate.clone());
                added_ids.push(candidate.id.clone());
            }
        }
        let cost = measurements.iter().map(|measurement| measurement.cost).sum::<u64>();
        if cost > budget {
            continue;
        }
        let metrics = calculate_metrics(&measurements, &prior, &weights, added_ids)?;
        portfolios.push(metrics);
    }
    if portfolios.is_empty() {
        return Err("no feasible measurement portfolios under budget".to_string());
    }

    let mut frontier = portfolios.iter().filter(|candidate| {
        !portfolios.iter().any(|other| {
            other.total_cost <= candidate.total_cost
                && other.weighted_variance <= candidate.weighted_variance
                && (other.total_cost < candidate.total_cost
                    || other.weighted_variance < candidate.weighted_variance - TOL)
        })
    }).cloned().collect::<Vec<_>>();
    frontier.sort_by(|a,b| {
        a.total_cost.cmp(&b.total_cost)
            .then_with(|| a.weighted_variance.total_cmp(&b.weighted_variance))
            .then_with(|| a.added_ids.join("|").cmp(&b.added_ids.join("|")))
    });

    let best = portfolios.iter().min_by(|a,b| {
        a.weighted_variance.total_cmp(&b.weighted_variance)
            .then_with(|| a.total_cost.cmp(&b.total_cost))
            .then_with(|| a.added_ids.join("|").cmp(&b.added_ids.join("|")))
    }).ok_or_else(|| "best feasible portfolio missing".to_string())?;

    let expected_best = manifest.get("expected_budget_optimum")
        .ok_or_else(|| "expected_budget_optimum required".to_string())?;
    if !compare_id_array(&best.added_ids, expected_best.get("best_added_measurement_ids")
        .ok_or_else(|| "best portfolio IDs required".to_string())?)?
        || best.total_cost != uint_at(expected_best, "total_cost_units")?
        || best.data_rank as u64 != uint_at(expected_best, "data_rank")?
        || !close(best.weighted_variance, number_at(expected_best, "weighted_posterior_variance")?, TOL)
        || !close(best.information_gain_nats, number_at(expected_best, "information_gain_nats")?, TOL)
    {
        return Err("budgeted optimum does not match the frozen synthetic oracle".to_string());
    }
    let expected_best_sd = expected_best.get("posterior_sd").and_then(Value::as_array)
        .ok_or_else(|| "expected budget optimum posterior_sd required".to_string())?;
    if expected_best_sd.len() != N {
        return Err("expected optimum posterior_sd must contain four values".to_string());
    }
    for (actual, expected) in best.posterior_sd.iter().zip(expected_best_sd.iter()) {
        let expected = expected.as_f64().ok_or_else(|| "expected optimum sd must be numeric".to_string())?;
        if !close(*actual, expected, TOL) {
            return Err("budget optimum posterior sd mismatch".to_string());
        }
    }

    let expected_frontier = manifest.get("expected_pareto_frontier_added_ids").and_then(Value::as_array)
        .ok_or_else(|| "expected_pareto_frontier_added_ids must be array".to_string())?;
    if expected_frontier.len() != frontier.len() {
        return Err("Pareto frontier length mismatch".to_string());
    }
    for (actual, expected) in frontier.iter().zip(expected_frontier.iter()) {
        if !compare_id_array(&actual.added_ids, expected)? {
            return Err("Pareto frontier portfolio membership/order mismatch".to_string());
        }
    }

    Ok(json!({
        "schema_version":"0.1.0",
        "benchmark_id":"LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id":"AFEX-PRACTICAL-DESIGN-001",
        "result":"PASS_SYNTHETIC_PRACTICAL_DESIGN",
        "source_manifest_sha256":sha256_hex(raw),
        "parameter_count":N,
        "diagnostic_scenario_count":scenario_results.len(),
        "budget_units":budget,
        "feasible_portfolio_count":portfolios.len(),
        "pareto_frontier":frontier.iter().map(portfolio_json).collect::<Vec<_>>(),
        "best_budgeted_portfolio":portfolio_json(best),
        "scenario_results":scenario_results,
        "data_identifiability_vs_prior_precision":"DATA_ONLY_RANK_IS_REPORTED_SEPARATELY_FROM_PRIOR_REGULARIZED_POSTERIOR_COVARIANCE",
        "covariate_only_assay_semantics":"AM_COLONISATION_REMEASURE_HAS_NO_DIRECT_PATHWAY_RATE_FISHER_ROW_IN_THIS_FIXED_F_MODEL; COVARIATE UNCERTAINTY IS NOT PROPAGATED",
        "actual_assay_cost_or_noise":"NOT_ESTABLISHED",
        "empirical_afex_optimum":"NOT_ESTABLISHED",
        "claim_ceiling":claim_ceiling
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE:&str=include_str!("../../cases/afex-practical-design-synthetic-v0.1.json");

    #[test]
    fn practical_design_reproduces_scenarios_and_budget_pareto_frontier() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture parses");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        assert_eq!(receipt["result"],"PASS_SYNTHETIC_PRACTICAL_DESIGN");
        assert_eq!(receipt["best_budgeted_portfolio"]["data_only_rank"],4);
        assert_eq!(receipt["best_budgeted_portfolio"]["total_cost_units"],24);
        assert_eq!(receipt["pareto_frontier"].as_array().expect("frontier").len(),8);
        assert_eq!(receipt["actual_assay_cost_or_noise"],"NOT_ESTABLISHED");
    }

    #[test]
    fn non_positive_measurement_noise_is_rejected() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["candidate_measurements"][0]["noise_sd"]=json!(0.0);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn covariate_only_measurement_cannot_be_relabelled_as_direct_flux() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["candidate_measurements"][5]["sensitivity_row"]=json!([0,4,0,0]);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn non_positive_definite_prior_is_rejected() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["prior_covariance"][0][0]=json!(-1.0);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn tight_prior_never_changes_data_only_rank() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let base=receipt["scenario_results"].as_array().expect("scenarios")
            .iter().find(|scenario|scenario["scenario_id"]=="BASE_AGGREGATE_ONLY").expect("base");
        let tight=receipt["scenario_results"].as_array().expect("scenarios")
            .iter().find(|scenario|scenario["scenario_id"]=="STRONG_PRIOR_NOT_DATA_INFORMATION").expect("tight prior");
        assert_eq!(base["data_only_rank"],tight["data_only_rank"]);
        assert!(tight["weighted_posterior_variance"].as_f64().expect("variance")
            < base["weighted_posterior_variance"].as_f64().expect("variance"));
    }
}
