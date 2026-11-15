// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

const PROGRAMME: &str = "WATER-MIXING-BAYESIAN-EXPERIMENT-DESIGN-001";
const CASE_COUNT: usize = 1;
const TARGET_PARAMETER_COUNT: usize = 2;
const EVIDENCE_CEILING: &str =
    "synthetic finite-prior Gaussian-mixture information-bound arithmetic only";

type Vector = Vec<f64>;
type Matrix = Vec<Vec<f64>>;
type Tensor3 = Vec<Vec<Vec<f64>>>;

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field: {key}"))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("{key} must be text"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{key} must be finite numeric"))
}

fn usize_value(value: &Value, key: &str) -> Result<usize, String> {
    field(value, key)?
        .as_u64()
        .and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key} must be an array"))
}

fn parse_matrix(value: &Value, key: &str, rows: usize, cols: usize) -> Result<Matrix, String> {
    let raw = value
        .as_array()
        .ok_or_else(|| format!("{key} must be a matrix"))?;
    if raw.len() != rows {
        return Err(format!("{key} row count mismatch"));
    }
    let mut result = Vec::with_capacity(rows);
    for row in raw {
        let values = row
            .as_array()
            .ok_or_else(|| format!("{key} row must be an array"))?;
        if values.len() != cols {
            return Err(format!("{key} column count mismatch"));
        }
        result.push(
            values
                .iter()
                .map(|v| {
                    v.as_f64()
                        .filter(|x| x.is_finite())
                        .ok_or_else(|| format!("{key} contains a non-finite value"))
                })
                .collect::<Result<Vector, _>>()?,
        );
    }
    Ok(result)
}

fn parse_tensor(
    value: &Value,
    key: &str,
    nout: usize,
    nin: usize,
) -> Result<Tensor3, String> {
    let outputs = value
        .as_array()
        .ok_or_else(|| format!("{key} must be a rank-three tensor"))?;
    if outputs.len() != nout {
        return Err(format!("{key} outlet count mismatch"));
    }
    let mut result = Vec::with_capacity(nout);
    let mut lag_count = None;
    for output in outputs {
        let inputs = output
            .as_array()
            .ok_or_else(|| format!("{key} outlet entry must be an array"))?;
        if inputs.len() != nin {
            return Err(format!("{key} inlet count mismatch"));
        }
        let mut input_rows = Vec::with_capacity(nin);
        for input in inputs {
            let lags = input
                .as_array()
                .ok_or_else(|| format!("{key} kernel entry must be an array"))?;
            if lags.is_empty() {
                return Err(format!("{key} lag support cannot be empty"));
            }
            if lag_count.is_some_and(|n| n != lags.len()) {
                return Err(format!("{key} lag lengths must be equal"));
            }
            lag_count = Some(lags.len());
            input_rows.push(
                lags.iter()
                    .map(|v| {
                        v.as_f64()
                            .filter(|x| x.is_finite())
                            .ok_or_else(|| format!("{key} contains a non-finite coefficient"))
                    })
                    .collect::<Result<Vector, _>>()?,
            );
        }
        result.push(input_rows);
    }
    Ok(result)
}

fn cholesky(matrix: &Matrix) -> Result<Matrix, String> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return Err("measurement covariance must be non-empty and square".into());
    }
    let scale = matrix
        .iter()
        .enumerate()
        .map(|(i, row)| row[i].abs())
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() || scale <= 0.0 {
        return Err("measurement covariance has no positive diagonal scale".into());
    }
    for i in 0..n {
        for j in 0..n {
            if (matrix[i][j] - matrix[j][i]).abs() > 1.0e-10 * scale {
                return Err("measurement covariance is not symmetric".into());
            }
        }
    }
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = matrix[i][j];
            for k in 0..j {
                sum -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !sum.is_finite() || sum <= 1.0e-12 * scale {
                    return Err(
                        "measurement covariance is not positive definite at tolerance".into(),
                    );
                }
                lower[i][j] = sum.sqrt();
            } else {
                lower[i][j] = sum / lower[j][j];
            }
        }
    }
    Ok(lower)
}

fn solve_spd(lower: &Matrix, rhs: &[f64]) -> Result<Vector, String> {
    let n = lower.len();
    if rhs.len() != n {
        return Err("SPD solve dimension mismatch".into());
    }
    let mut forward = vec![0.0; n];
    for i in 0..n {
        let mut value = rhs[i];
        for j in 0..i {
            value -= lower[i][j] * forward[j];
        }
        forward[i] = value / lower[i][i];
    }
    let mut result = vec![0.0; n];
    for i in (0..n).rev() {
        let mut value = forward[i];
        for j in (i + 1)..n {
            value -= lower[j][i] * result[j];
        }
        result[i] = value / lower[i][i];
    }
    if result.iter().any(|v| !v.is_finite()) {
        return Err("SPD solve produced a non-finite result".into());
    }
    Ok(result)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn convolve_channels(input: &Matrix, response: &Matrix) -> Result<Matrix, String> {
    if input.is_empty() || input.len() != response.len() {
        return Err("channel convolution count mismatch".into());
    }
    let input_len = input[0].len();
    let response_len = response[0].len();
    if input_len == 0
        || response_len == 0
        || input.iter().any(|row| row.len() != input_len)
        || response.iter().any(|row| row.len() != response_len)
    {
        return Err("channel convolution requires rectangular non-empty matrices".into());
    }
    let mut result = vec![vec![0.0; input_len + response_len - 1]; input.len()];
    for channel in 0..input.len() {
        for t in 0..input_len {
            for lag in 0..response_len {
                result[channel][t + lag] += input[channel][t] * response[channel][lag];
            }
        }
    }
    if result.iter().flatten().any(|v| !v.is_finite()) {
        return Err("channel convolution overflow".into());
    }
    Ok(result)
}

#[allow(clippy::needless_range_loop)]
fn convolve_kernel(input: &Matrix, kernel: &Tensor3) -> Result<Matrix, String> {
    if input.is_empty() || kernel.is_empty() || kernel[0].len() != input.len() {
        return Err("kernel convolution inlet dimension mismatch".into());
    }
    let ntime = input[0].len();
    let nlag = kernel[0][0].len();
    if ntime == 0
        || input.iter().any(|row| row.len() != ntime)
        || kernel.iter().any(|out| {
            out.len() != input.len() || out.iter().any(|entry| entry.len() != nlag)
        })
    {
        return Err("kernel convolution dimensions are inconsistent".into());
    }
    let mut result = vec![vec![0.0; ntime + nlag - 1]; kernel.len()];
    for outlet in 0..kernel.len() {
        for inlet in 0..input.len() {
            for t in 0..ntime {
                for lag in 0..nlag {
                    result[outlet][t + lag] += kernel[outlet][inlet][lag] * input[inlet][t];
                }
            }
        }
    }
    if result.iter().flatten().any(|v| !v.is_finite()) {
        return Err("kernel convolution overflow".into());
    }
    Ok(result)
}

fn flatten(matrix: &Matrix) -> Vector {
    matrix.iter().flat_map(|row| row.iter().copied()).collect()
}

fn eigenvalues_2x2(matrix: &Matrix) -> Result<Vector, String> {
    if matrix.len() != 2 || matrix.iter().any(|row| row.len() != 2) {
        return Err("2x2 information matrix required".into());
    }
    let a = matrix[0][0];
    let b = 0.5 * (matrix[0][1] + matrix[1][0]);
    let c = matrix[1][1];
    let discriminant = ((a - c).powi(2) + 4.0 * b * b).sqrt();
    let scale = a.abs().max(b.abs()).max(c.abs()).max(1.0);
    let mut values = vec![
        0.5 * (a + c + discriminant),
        0.5 * (a + c - discriminant),
    ];
    for value in &mut values {
        if *value < -1.0e-9 * scale {
            return Err("information matrix has materially negative eigenvalue".into());
        }
        if *value < 0.0 {
            *value = 0.0;
        }
    }
    values.sort_by(|x, y| y.total_cmp(x));
    Ok(values)
}

fn fisher_information(jacobian: &Matrix, covariance_lower: &Matrix) -> Result<Matrix, String> {
    if jacobian.is_empty()
        || jacobian.iter().any(|row| row.len() != TARGET_PARAMETER_COUNT)
        || jacobian.len() != covariance_lower.len()
    {
        return Err("Fisher information dimensions mismatch".into());
    }
    let mut information = vec![vec![0.0; TARGET_PARAMETER_COUNT]; TARGET_PARAMETER_COUNT];
    let mut solved = Vec::with_capacity(TARGET_PARAMETER_COUNT);
    for parameter in 0..TARGET_PARAMETER_COUNT {
        let rhs = jacobian
            .iter()
            .map(|row| row[parameter])
            .collect::<Vector>();
        solved.push(solve_spd(covariance_lower, &rhs)?);
    }
    for i in 0..TARGET_PARAMETER_COUNT {
        for j in 0..TARGET_PARAMETER_COUNT {
            let left = jacobian
                .iter()
                .map(|row| row[i])
                .collect::<Vector>();
            information[i][j] = dot(&left, &solved[j]);
        }
    }
    let off_diagonal = 0.5 * (information[0][1] + information[1][0]);
    information[0][1] = off_diagonal;
    information[1][0] = off_diagonal;
    Ok(information)
}

fn information_metrics(information: &Matrix, rank_tolerance: f64) -> Result<Value, String> {
    let eigenvalues = eigenvalues_2x2(information)?;
    let singular = eigenvalues.iter().map(|value| value.sqrt()).collect::<Vector>();
    let maximum = singular[0];
    let rank = if maximum <= 0.0 {
        0
    } else {
        singular
            .iter()
            .filter(|value| **value > maximum * rank_tolerance)
            .count()
    };
    let full_rank = rank == TARGET_PARAMETER_COUNT && singular[1] > 0.0;
    let condition = if full_rank {
        Some(maximum / singular[1])
    } else {
        None
    };
    let determinant = information[0][0] * information[1][1]
        - information[0][1] * information[1][0];
    let logdet = if full_rank && determinant > 0.0 {
        Some(determinant.ln())
    } else {
        None
    };
    Ok(json!({
        "singular_values": singular,
        "rank": rank,
        "condition_number": condition,
        "log_determinant": logdet,
        "minimum_singular_value": if full_rank { singular[1] } else { 0.0 }
    }))
}

fn log_sum_exp(values: &[f64]) -> Result<f64, String> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err("log-sum-exp requires finite non-empty inputs".into());
    }
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let sum = values.iter().map(|value| (value - maximum).exp()).sum::<f64>();
    if !sum.is_finite() || sum <= 0.0 {
        return Err("log-sum-exp underflow/overflow".into());
    }
    Ok(maximum + sum.ln())
}

fn prior_entropy(weights: &[f64]) -> f64 {
    -weights
        .iter()
        .map(|weight| weight * weight.ln())
        .sum::<f64>()
}

fn bhattacharyya_distances(
    means: &[Vector],
    weights: &[f64],
    covariance_lower: &Matrix,
) -> Result<(Matrix, f64), String> {
    if means.len() != weights.len() || means.is_empty() {
        return Err("prior mean/weight count mismatch".into());
    }
    let mut distances = vec![vec![0.0; means.len()]; means.len()];
    for i in 0..means.len() {
        if means[i].len() != covariance_lower.len() {
            return Err("measurement mean/covariance dimension mismatch".into());
        }
        for j in (i + 1)..means.len() {
            let delta = means[i]
                .iter()
                .zip(&means[j])
                .map(|(left, right)| left - right)
                .collect::<Vector>();
            let solved = solve_spd(covariance_lower, &delta)?;
            let distance = (0.125 * dot(&delta, &solved)).max(0.0);
            if !distance.is_finite() {
                return Err("Bhattacharyya distance is non-finite".into());
            }
            distances[i][j] = distance;
            distances[j][i] = distance;
        }
    }
    let mut lower_bound = 0.0;
    for i in 0..weights.len() {
        let terms = (0..weights.len())
            .map(|j| weights[j].ln() - distances[i][j])
            .collect::<Vector>();
        lower_bound -= weights[i] * log_sum_exp(&terms)?;
    }
    Ok((distances, lower_bound.max(0.0)))
}

fn base_candidate(state: &str, reason: &str, entropy: f64) -> Value {
    json!({
        "candidate_state": state,
        "reason": reason,
        "prior_entropy_nats": entropy,
        "mutual_information_lower_bound_nats": null,
        "normalized_information_fraction": null,
        "pairwise_bhattacharyya_distances": null,
        "minimum_pairwise_bhattacharyya_distance": null,
        "robust_minimum_rank": null,
        "robust_minimum_singular_value": null,
        "average_fisher_information": null,
        "average_fisher_log_determinant": null,
        "worst_case_injected_mass_kg": null,
        "worst_case_maximum_inlet_mass_flux": null,
        "worst_case_maximum_inlet_slew_kg_s2": null,
        "duration_seconds": null,
        "pareto_optimal": false,
        "selected": false,
        "physical_operator_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn validate_prior_particles(
    input: &Value,
    nin: usize,
    nout: usize,
) -> Result<(Vec<Value>, Vec<f64>, Vec<String>, usize, usize, usize), String> {
    let particles = array(input, "prior_particles")?;
    if particles.len() < 2 {
        return Err("at least two prior particles are required".into());
    }
    let mut ids = BTreeSet::new();
    let mut weights = Vec::with_capacity(particles.len());
    let mut actuator_lags = None;
    let mut sensor_lags = None;
    let mut kernel_lags = None;
    for particle in particles {
        let id = text(particle, "particle_id")?;
        if !ids.insert(id.to_owned()) {
            return Err("duplicate prior particle identity".into());
        }
        let weight = number(particle, "prior_weight")?;
        if weight <= 0.0 {
            return Err("prior particle weights must be strictly positive".into());
        }
        weights.push(weight);
        let actuator_rows = array(particle, "actuator_response")?;
        if actuator_rows.len() != nin {
            return Err("prior actuator response inlet count mismatch".into());
        }
        let al = actuator_rows
            .first()
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if al == 0 || actuator_lags.is_some_and(|previous| previous != al) {
            return Err("prior actuator FIR lengths must be non-empty and equal".into());
        }
        actuator_lags = Some(al);
        let actuator = parse_matrix(particle, "actuator_response", nin, al)?;
        for row in &actuator {
            if row.iter().any(|value| *value < 0.0)
                || (row.iter().sum::<f64>() - 1.0).abs() > 1.0e-8
            {
                return Err("each prior actuator response must be nonnegative and unit-area".into());
            }
        }
        let sensor_rows = array(particle, "sensor_response")?;
        if sensor_rows.len() != nout {
            return Err("prior sensor response outlet count mismatch".into());
        }
        let sl = sensor_rows
            .first()
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if sl == 0 || sensor_lags.is_some_and(|previous| previous != sl) {
            return Err("prior sensor FIR lengths must be non-empty and equal".into());
        }
        sensor_lags = Some(sl);
        let sensor = parse_matrix(particle, "sensor_response", nout, sl)?;
        for row in &sensor {
            if row.iter().any(|value| *value < 0.0)
                || (row.iter().sum::<f64>() - 1.0).abs() > 1.0e-8
            {
                return Err("each prior sensor response must be nonnegative and unit-area".into());
            }
        }
        let kernel_rows = array(particle, "kernel")?;
        let kl = kernel_rows
            .first()
            .and_then(Value::as_array)
            .and_then(|row| row.first())
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        if kl == 0 || kernel_lags.is_some_and(|previous| previous != kl) {
            return Err("prior kernel lag supports must be non-empty and equal".into());
        }
        kernel_lags = Some(kl);
        let kernel = parse_tensor(particle, "kernel", nout, nin)?;
        if kernel.iter().flatten().flatten().any(|value| *value < 0.0) {
            return Err("prior physical kernels must be nonnegative".into());
        }
        for inlet in 0..nin {
            let gain = kernel
                .iter()
                .map(|outlet| outlet[inlet].iter().sum::<f64>())
                .sum::<f64>();
            if gain > 1.0 + 1.0e-9 {
                return Err("prior physical kernel passive gain exceeds one".into());
            }
        }
    }
    let weight_sum = weights.iter().sum::<f64>();
    if (weight_sum - 1.0).abs() > 1.0e-9 {
        return Err("prior particle weights must sum to one".into());
    }
    Ok((
        particles.iter().cloned().collect(),
        weights,
        ids.into_iter().collect(),
        actuator_lags.unwrap_or(0),
        kernel_lags.unwrap_or(0),
        sensor_lags.unwrap_or(0),
    ))
}

fn compute_candidate(
    candidate: &Value,
    particles: &[Value],
    weights: &[f64],
    parameter_directions: &[Tensor3],
    covariance_lower: &Matrix,
    dt: f64,
    minimum_resolvable_delay: f64,
    minimum_delay_of_interest: f64,
    rank_tolerance: f64,
    minimum_information: f64,
    minimum_mutual_information: f64,
    limits: &Value,
    nin: usize,
    nout: usize,
    kernel_lags: usize,
) -> Result<Value, String> {
    let entropy = prior_entropy(weights);
    if text(candidate, "waveform_provenance")? != "command_plus_prior_actuator_model" {
        return Ok(base_candidate(
            "INJECTOR_RESPONSE_UNRESOLVED",
            "COMMAND_ONLY_OR_UNSUPPORTED_WAVEFORM_PROVENANCE",
            entropy,
        ));
    }
    if text(candidate, "injector_calibration_state")? != "calibrated" {
        return Ok(base_candidate(
            "INJECTOR_RESPONSE_UNRESOLVED",
            "INJECTOR_CALIBRATION_NOT_AVAILABLE",
            entropy,
        ));
    }
    if text(candidate, "sensor_calibration_state")? != "calibrated" {
        return Ok(base_candidate(
            "SENSOR_RESPONSE_UNRESOLVED",
            "SENSOR_CALIBRATION_NOT_AVAILABLE",
            entropy,
        ));
    }
    if !bool_field(candidate, "all_inlets_observed")? {
        return Ok(base_candidate(
            "PARTIAL_INLET_BOUNDARY",
            "ONE_OR_MORE_INLET_MASS_FLUXES_NOT_OBSERVED",
            entropy,
        ));
    }
    if !bool_field(candidate, "all_outlets_observed")? {
        return Ok(base_candidate(
            "PARTIAL_OUTLET_BOUNDARY",
            "ONE_OR_MORE_OUTLET_CHANNELS_NOT_OBSERVED",
            entropy,
        ));
    }
    if !bool_field(candidate, "temporal_boundary_complete")?
        || !bool_field(candidate, "include_full_tail")?
    {
        return Ok(base_candidate(
            "PARTIAL_TEMPORAL_WINDOW",
            "COMPLETE_PHYSICAL_AND_SENSOR_TAIL_NOT_INCLUDED",
            entropy,
        ));
    }
    let feature_period = number(candidate, "minimum_feature_period_seconds")?;
    if feature_period <= 0.0 {
        return Ok(base_candidate(
            "INVALID_INPUT",
            "MINIMUM_FEATURE_PERIOD_NOT_POSITIVE",
            entropy,
        ));
    }
    if feature_period < minimum_resolvable_delay {
        return Ok(base_candidate(
            "SENSOR_BANDWIDTH_INADEQUATE",
            "FEATURE_PERIOD_FASTER_THAN_PRIOR_SENSOR_RESOLUTION",
            entropy,
        ));
    }
    if dt > minimum_delay_of_interest / 10.0 {
        return Ok(base_candidate(
            "SENSOR_SAMPLING_INADEQUATE",
            "SAMPLE_INTERVAL_EXCEEDS_ONE_TENTH_OF_MINIMUM_DELAY",
            entropy,
        ));
    }
    let command_rows = array(candidate, "commanded_inlet_mass_flux")?;
    if command_rows.len() != nin {
        return Ok(base_candidate(
            "INVALID_INPUT",
            "COMMANDED_INLET_COUNT_MISMATCH",
            entropy,
        ));
    }
    let ntime = command_rows
        .first()
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if ntime == 0 {
        return Ok(base_candidate(
            "INVALID_INPUT",
            "EMPTY_COMMANDED_MASS_FLUX",
            entropy,
        ));
    }
    let command = parse_matrix(candidate, "commanded_inlet_mass_flux", nin, ntime)?;
    if command.iter().flatten().any(|value| *value < 0.0) {
        return Ok(base_candidate(
            "INVALID_INPUT",
            "NEGATIVE_COMMANDED_MASS_FLUX",
            entropy,
        ));
    }

    let mut means = Vec::with_capacity(particles.len());
    let mut particle_information = Vec::with_capacity(particles.len());
    let mut injected_masses = Vec::with_capacity(particles.len());
    let mut maximum_rates = Vec::with_capacity(particles.len());
    let mut maximum_slews = Vec::with_capacity(particles.len());
    let mut duration = 0.0_f64;
    for particle in particles {
        let actuator_rows = array(particle, "actuator_response")?;
        let actuator_lags = actuator_rows[0].as_array().map(Vec::len).unwrap_or(0);
        let actuator = parse_matrix(particle, "actuator_response", nin, actuator_lags)?;
        let actual_input = convolve_channels(&command, &actuator)?;
        let mass = actual_input
            .iter()
            .map(|row| row.iter().sum::<f64>())
            .sum::<f64>()
            * dt;
        let maximum_rate = actual_input
            .iter()
            .flatten()
            .copied()
            .fold(0.0_f64, f64::max);
        let maximum_slew = actual_input
            .iter()
            .map(|row| {
                row.windows(2)
                    .map(|pair| (pair[1] - pair[0]).abs() / dt)
                    .fold(0.0_f64, f64::max)
            })
            .fold(0.0_f64, f64::max);
        injected_masses.push(mass);
        maximum_rates.push(maximum_rate);
        maximum_slews.push(maximum_slew);

        let kernel = parse_tensor(particle, "kernel", nout, nin)?;
        let physical = convolve_kernel(&actual_input, &kernel)?;
        let sensor_rows = array(particle, "sensor_response")?;
        let sensor_lags = sensor_rows[0].as_array().map(Vec::len).unwrap_or(0);
        let sensor = parse_matrix(particle, "sensor_response", nout, sensor_lags)?;
        let measured = convolve_channels(&physical, &sensor)?;
        let mean = flatten(&measured);
        if mean.len() != covariance_lower.len() {
            return Err(format!(
                "prior measurement dimension mismatch: expected {}, got {}",
                covariance_lower.len(),
                mean.len()
            ));
        }
        means.push(mean);

        let mut columns = Vec::with_capacity(TARGET_PARAMETER_COUNT);
        for direction in parameter_directions {
            let sensitivity = convolve_kernel(&actual_input, direction)?;
            let measured_sensitivity = convolve_channels(&sensitivity, &sensor)?;
            columns.push(flatten(&measured_sensitivity));
        }
        let mut jacobian = vec![vec![0.0; TARGET_PARAMETER_COUNT]; covariance_lower.len()];
        for parameter in 0..TARGET_PARAMETER_COUNT {
            if columns[parameter].len() != covariance_lower.len() {
                return Err("prior sensitivity dimension mismatch".into());
            }
            for row in 0..covariance_lower.len() {
                jacobian[row][parameter] = columns[parameter][row];
            }
        }
        let information = fisher_information(&jacobian, covariance_lower)?;
        particle_information.push(information);

        let sample_count = actual_input[0].len() + kernel_lags + sensor_lags - 2;
        duration = duration.max(sample_count as f64 * dt);
    }

    let maximum_dose = number(limits, "maximum_injected_mass_kg")?;
    let maximum_rate_limit = number(limits, "maximum_inlet_mass_flux")?;
    let maximum_slew_limit = number(limits, "maximum_inlet_slew_kg_s2")?;
    let maximum_duration = number(limits, "maximum_duration_seconds")?;
    let worst_mass = injected_masses.iter().copied().fold(0.0_f64, f64::max);
    let worst_rate = maximum_rates.iter().copied().fold(0.0_f64, f64::max);
    let worst_slew = maximum_slews.iter().copied().fold(0.0_f64, f64::max);
    if worst_mass > maximum_dose
        || worst_rate > maximum_rate_limit
        || worst_slew > maximum_slew_limit
        || duration > maximum_duration
    {
        let reason = if worst_mass > maximum_dose {
            "INJECTION_DOSE_LIMIT_EXCEEDED"
        } else if worst_rate > maximum_rate_limit {
            "INLET_MASS_FLUX_LIMIT_EXCEEDED"
        } else if worst_slew > maximum_slew_limit {
            "INLET_SLEW_LIMIT_EXCEEDED"
        } else {
            "EXPERIMENT_DURATION_LIMIT_EXCEEDED"
        };
        return Ok(base_candidate(
            "OPERATIONAL_CONSTRAINT_VIOLATION",
            reason,
            entropy,
        ));
    }

    let mut average_information = vec![vec![0.0; TARGET_PARAMETER_COUNT]; TARGET_PARAMETER_COUNT];
    let mut robust_minimum_rank = TARGET_PARAMETER_COUNT;
    let mut robust_minimum_singular_value = f64::INFINITY;
    for (weight, information) in weights.iter().zip(&particle_information) {
        let metrics = information_metrics(information, rank_tolerance)?;
        robust_minimum_rank = robust_minimum_rank.min(usize_value(&metrics, "rank")?);
        robust_minimum_singular_value = robust_minimum_singular_value
            .min(number(&metrics, "minimum_singular_value")?);
        for i in 0..TARGET_PARAMETER_COUNT {
            for j in 0..TARGET_PARAMETER_COUNT {
                average_information[i][j] += weight * information[i][j];
            }
        }
    }
    let average_metrics = information_metrics(&average_information, rank_tolerance)?;
    let average_logdet = average_metrics.get("log_determinant").and_then(Value::as_f64);

    let (pairwise, information_lower_bound) =
        bhattacharyya_distances(&means, weights, covariance_lower)?;
    if !information_lower_bound.is_finite()
        || information_lower_bound > entropy + 1.0e-8
    {
        return Err("mutual-information lower bound outside prior entropy bounds".into());
    }
    let normalized_information = if entropy > 0.0 {
        (information_lower_bound / entropy).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let minimum_pairwise = pairwise
        .iter()
        .enumerate()
        .flat_map(|(i, row)| row.iter().skip(i + 1).copied())
        .fold(f64::INFINITY, f64::min);
    let state = if robust_minimum_rank < TARGET_PARAMETER_COUNT {
        "ROBUSTLY_UNIDENTIFIABLE"
    } else if robust_minimum_singular_value < minimum_information {
        "INSUFFICIENT_ROBUST_PARAMETER_INFORMATION"
    } else if information_lower_bound < minimum_mutual_information {
        "INSUFFICIENT_BAYESIAN_INFORMATION"
    } else {
        "ELIGIBLE_BAYESIAN_DESIGN"
    };
    Ok(json!({
        "candidate_state": state,
        "reason": null,
        "prior_entropy_nats": entropy,
        "mutual_information_lower_bound_nats": information_lower_bound,
        "normalized_information_fraction": normalized_information,
        "pairwise_bhattacharyya_distances": pairwise,
        "minimum_pairwise_bhattacharyya_distance": minimum_pairwise,
        "robust_minimum_rank": robust_minimum_rank,
        "robust_minimum_singular_value": robust_minimum_singular_value,
        "average_fisher_information": average_information,
        "average_fisher_log_determinant": average_logdet,
        "worst_case_injected_mass_kg": worst_mass,
        "worst_case_maximum_inlet_mass_flux": worst_rate,
        "worst_case_maximum_inlet_slew_kg_s2": worst_slew,
        "duration_seconds": duration,
        "pareto_optimal": false,
        "selected": false,
        "physical_operator_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false
    }))
}

fn bool_field(value: &Value, key: &str) -> Result<bool, String> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("{key} must be boolean"))
}

fn dominates(left: &Value, right: &Value) -> bool {
    let left_info = left["mutual_information_lower_bound_nats"]
        .as_f64()
        .unwrap_or(0.0);
    let right_info = right["mutual_information_lower_bound_nats"]
        .as_f64()
        .unwrap_or(0.0);
    let left_dose = left["worst_case_injected_mass_kg"]
        .as_f64()
        .unwrap_or(f64::INFINITY);
    let right_dose = right["worst_case_injected_mass_kg"]
        .as_f64()
        .unwrap_or(f64::INFINITY);
    let left_duration = left["duration_seconds"].as_f64().unwrap_or(f64::INFINITY);
    let right_duration = right["duration_seconds"].as_f64().unwrap_or(f64::INFINITY);
    let no_worse = left_info >= right_info
        && left_dose <= right_dose
        && left_duration <= right_duration;
    let strictly = left_info > right_info
        || left_dose < right_dose
        || left_duration < right_duration;
    no_worse && strictly
}

fn compare(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => {
            matches!((a.as_f64(), b.as_f64()), (Some(x), Some(y))
                if (x - y).abs() <= 1.0e-8 * x.abs().max(y.abs()).max(1.0))
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| compare(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| compare(value, other)))
        }
        _ => left == right,
    }
}

fn evaluate(input: &Value, oracle: &Value, manifest: &Value) -> Result<Value, String> {
    if text(input, "schema_version")? != "bayesian-excitation-t0-input-v0.1"
        || text(oracle, "schema_version")? != "bayesian-excitation-t0-oracle-v0.1"
        || text(manifest, "schema_version")? != "bayesian-excitation-t0-manifest-v0.1"
    {
        return Err("unexpected Bayesian-excitation schema".into());
    }
    for document in [input, oracle, manifest] {
        if text(document, "programme_id")? != PROGRAMME {
            return Err("Bayesian-excitation programme identity mismatch".into());
        }
    }
    for key in [
        "physical_operator_qualified",
        "potability_claim_permitted",
        "release_authorized",
    ] {
        if field(oracle, key)?.as_bool() != Some(false) {
            return Err(format!("oracle cannot claim {key}"));
        }
    }
    let cases = array(input, "cases")?;
    let expected = array(oracle, "cases")?;
    let case_ids = array(manifest, "case_ids")?;
    if cases.len() != CASE_COUNT
        || expected.len() != CASE_COUNT
        || usize_value(manifest, "case_count")? != CASE_COUNT
    {
        return Err("frozen Bayesian-excitation case count mismatch".into());
    }
    let collect = |rows: &[Value], key: &str| -> Result<BTreeSet<String>, String> {
        rows.iter()
            .map(|value| text(value, key).map(str::to_owned))
            .collect()
    };
    let input_ids = collect(cases, "case_id")?;
    let oracle_ids = collect(expected, "case_id")?;
    let manifest_ids = case_ids
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "manifest case ID must be text".to_owned())
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if input_ids.len() != cases.len()
        || oracle_ids.len() != expected.len()
        || manifest_ids.len() != case_ids.len()
        || input_ids != oracle_ids
        || input_ids != manifest_ids
    {
        return Err("input/oracle/manifest case identities differ".into());
    }

    let config = field(input, "configuration")?;
    let dt = number(config, "sample_interval_seconds")?;
    let minimum_delay = number(config, "minimum_delay_of_interest_seconds")?;
    let minimum_resolution = number(config, "minimum_resolvable_delay_seconds")?;
    if dt <= 0.0 || minimum_delay <= 0.0 || minimum_resolution <= 0.0 {
        return Err("sampling interval and sensor resolutions must be positive".into());
    }
    let rank_tolerance = number(config, "rank_relative_tolerance")?;
    let minimum_information = number(config, "minimum_robust_singular_value")?;
    let minimum_mi = number(config, "minimum_mutual_information_nats")?;
    if rank_tolerance <= 0.0
        || rank_tolerance >= 0.1
        || minimum_information < 0.0
        || minimum_mi < 0.0
    {
        return Err("rank or information threshold is invalid".into());
    }
    let direction_raw = array(config, "parameter_kernel_directions")?;
    if direction_raw.len() != TARGET_PARAMETER_COUNT {
        return Err("exactly two target parameter directions are required in v0.1".into());
    }
    let nin = array(&cases[0], "candidates")?
        .first()
        .and_then(|candidate| candidate.get("commanded_inlet_mass_flux"))
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| "cannot infer commanded inlet count".to_owned())?;
    let candidates = array(&cases[0], "candidates")?;
    let first_command = array(&candidates[0], "commanded_inlet_mass_flux")?;
    let first_nin = first_command.len();
    let ntime = first_command
        .first()
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if first_nin == 0 || ntime == 0 {
        return Err("first candidate command matrix is empty".into());
    }
    let first_particle = &array(input, "prior_particles")?[0];
    let nout = array(first_particle, "sensor_response")?.len();
    if nout == 0 {
        return Err("prior particle has no sensor outlet channels".into());
    }
    let (particles, weights, particle_ids, actuator_lags, kernel_lags, sensor_lags) =
        validate_prior_particles(input, first_nin, nout)?;
    let mut directions = Vec::with_capacity(TARGET_PARAMETER_COUNT);
    for (index, raw) in direction_raw.iter().enumerate() {
        directions.push(parse_tensor(raw, "parameter_kernel_direction", nout, first_nin)?);
        if directions[index][0][0].len() != kernel_lags {
            return Err("parameter direction lag length mismatch".into());
        }
        if directions[index]
            .iter()
            .any(|outlet| outlet.iter().any(|entry| entry.len() != kernel_lags))
        {
            return Err("parameter direction lag lengths differ".into());
        }
    }
    let observation_count = ntime + actuator_lags + kernel_lags + sensor_lags - 3;
    let covariance_raw = array(config, "measurement_covariance")?;
    let covariance = parse_matrix(
        config,
        "measurement_covariance",
        observation_count,
        observation_count,
    )?;
    let covariance_lower = cholesky(&covariance)?;
    let prior_entropy = prior_entropy(&weights);
    let mut results = Vec::new();
    for candidate in candidates {
        let mut result = compute_candidate(
            candidate,
            &particles,
            &weights,
            &directions,
            &covariance_lower,
            dt,
            minimum_resolution,
            minimum_delay,
            rank_tolerance,
            minimum_information,
            minimum_mi,
            field(config, "limits")?,
            nin,
            nout,
            kernel_lags,
        )?;
        result["design_id"] = json!(text(candidate, "design_id")?);
        results.push(result);
    }
    for index in 0..results.len() {
        if results[index]["candidate_state"] != "ELIGIBLE_BAYESIAN_DESIGN" {
            continue;
        }
        let dominated = (0..results.len()).any(|other| {
            other != index
                && results[other]["candidate_state"] == "ELIGIBLE_BAYESIAN_DESIGN"
                && dominates(&results[other], &results[index])
        });
        results[index]["pareto_optimal"] = json!(!dominated);
    }
    let selected_index = results
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            row["candidate_state"] == "ELIGIBLE_BAYESIAN_DESIGN"
                && row["pareto_optimal"] == true
        })
        .max_by(|(_, left), (_, right)| {
            left["mutual_information_lower_bound_nats"]
                .as_f64()
                .unwrap_or(0.0)
                .total_cmp(
                    &right["mutual_information_lower_bound_nats"]
                        .as_f64()
                        .unwrap_or(0.0),
                )
                .then_with(|| {
                    right["worst_case_injected_mass_kg"]
                        .as_f64()
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(
                            &left["worst_case_injected_mass_kg"]
                                .as_f64()
                                .unwrap_or(f64::INFINITY),
                        )
                })
        })
        .map(|(index, _)| index);
    if let Some(index) = selected_index {
        results[index]["selected"] = json!(true);
    }
    let mut mismatches = Vec::new();
    for case in cases {
        let expected_case = expected
            .iter()
            .find(|row| {
                row.get("case_id").and_then(Value::as_str)
                    == case.get("case_id").and_then(Value::as_str)
            })
            .ok_or_else(|| "oracle case missing".to_owned())?;
        let expected_rows = array(expected_case, "candidates")?;
        if expected_rows.len() != results.len() {
            return Err("oracle candidate count mismatch".into());
        }
        for (actual, expected_row) in results.iter().zip(expected_rows) {
            if !compare(actual, expected_row) {
                mismatches.push(json!({
                    "case_id": text(case, "case_id")?,
                    "design_id": actual["design_id"],
                    "expected": expected_row,
                    "actual": actual
                }));
            }
        }
    }
    let matched = mismatches.is_empty();
    Ok(json!({
        "schema_version": "bayesian-excitation-t0-evaluation-v0.1",
        "verification_state": if matched { "MatchedFrozenOracle" } else { "OracleMismatch" },
        "evidence_class": "synthetic_only",
        "evidence_ceiling": EVIDENCE_CEILING,
        "prior_particle_count": particles.len(),
        "prior_particle_ids": particle_ids,
        "prior_entropy_nats": prior_entropy,
        "candidate_count": results.len(),
        "eligible_candidate_count": results.iter().filter(|row| row["candidate_state"] == "ELIGIBLE_BAYESIAN_DESIGN").count(),
        "selected_design_id": selected_index.map(|index| results[index]["design_id"].clone()),
        "pareto_design_ids": results.iter().filter(|row| row["pareto_optimal"] == true).map(|row| row["design_id"].clone()).collect::<Vec<_>>(),
        "mismatches": mismatches,
        "candidates": results,
        "physical_operator_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false,
        "nonclaims": [
            "The Kolchinsky-Tracey Gaussian-mixture lower bound is a lower bound on mutual information, not an exact MI estimate.",
            "All prior particles share one declared measurement covariance in v0.1; particle-dependent observation covariance is not supported.",
            "The finite particle prior approximates uncertainty over joint kernel, actuator and sensor responses; prior discretization error is not quantified here.",
            "The expected inlet mass flux is predicted from prior actuator FIRs. Physical identification must use independently measured inlet mass flux.",
            "A robust local Fisher rank/singular-value gate is retained; Bayesian prior separation does not substitute for target-parameter identifiability.",
            "No output establishes a physical junction, tracer, sensor, drinking-water system, potability, contamination, toxicology, standard compliance or release authority."
        ]
    }))
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok((bytes, value))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn run() -> Result<i32, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() > 4 {
        return Err("usage: bayesian-excitation-t0 [INPUT] [ORACLE] [MANIFEST] [RECEIPT]".into());
    }
    let input_path = PathBuf::from(
        args.first()
            .map(String::as_str)
            .unwrap_or("research/water-intrusion-coupling/bayesian-excitation-t0-cases-v0.1.json"),
    );
    let oracle_path = PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("research/water-intrusion-coupling/bayesian-excitation-t0-oracle-v0.1.json"),
    );
    let manifest_path = PathBuf::from(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("research/water-intrusion-coupling/bayesian-excitation-t0-manifest-v0.1.json"),
    );
    let receipt_path = PathBuf::from(
        args.get(3)
            .map(String::as_str)
            .unwrap_or("artifacts/water-mixing-bayesian-excitation-t0/receipt.json"),
    );
    let (input_bytes, input) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let evaluation = evaluate(&input, &oracle, &manifest)?;
    let success = evaluation["verification_state"] == "MatchedFrozenOracle";
    let revision = env::var("WATER_MIXING_BAYESIAN_EXCITATION_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes =
        fs::read(&lock_path).map_err(|e| format!("read {}: {e}", lock_path.display()))?;
    let receipt = json!({
        "schema_version": "bayesian-excitation-t0-run-v0.1",
        "verification_state": if !success { "OracleMismatch" } else if revision.is_some() { "PassedExactRevision" } else { "PassedLocalUnpinned" },
        "implementation_revision": revision,
        "input_sha256": digest(&input_bytes),
        "oracle_sha256": digest(&oracle_bytes),
        "manifest_sha256": digest(&manifest_bytes),
        "cargo_lock_sha256": digest(&lock_bytes),
        "prior_particle_count": evaluation["prior_particle_count"],
        "prior_particle_ids": evaluation["prior_particle_ids"],
        "prior_entropy_nats": evaluation["prior_entropy_nats"],
        "candidate_count": evaluation["candidate_count"],
        "eligible_candidate_count": evaluation["eligible_candidate_count"],
        "selected_design_id": evaluation["selected_design_id"],
        "pareto_design_ids": evaluation["pareto_design_ids"],
        "mismatches": evaluation["mismatches"],
        "candidates": evaluation["candidates"],
        "evidence_class": "synthetic_only",
        "evidence_ceiling": EVIDENCE_CEILING,
        "physical_operator_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false
    });
    if let Some(parent) = receipt_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let bytes = serde_json::to_string_pretty(&receipt)
        .map_err(|e| format!("serialize receipt: {e}"))?
        + "\n";
    fs::write(&receipt_path, bytes.as_bytes())
        .map_err(|e| format!("write {}: {e}", receipt_path.display()))?;
    println!("{bytes}");
    Ok(if success { 0 } else { 2 })
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("WATER_MIXING_BAYESIAN_EXCITATION_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Value {
        let raw = match name {
            "input" => include_str!(
                "../../../../research/water-intrusion-coupling/bayesian-excitation-t0-cases-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../../research/water-intrusion-coupling/bayesian-excitation-t0-oracle-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../../research/water-intrusion-coupling/bayesian-excitation-t0-manifest-v0.1.json"
            ),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(raw).expect("Bayesian-excitation fixture parses")
    }

    fn result() -> Value {
        evaluate(&load("input"), &load("oracle"), &load("manifest"))
            .expect("Bayesian-excitation evaluator runs")
    }

    fn row(id: &str) -> Value {
        result()["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["design_id"] == id)
            .unwrap()
            .clone()
    }

    #[test]
    fn all_candidates_match_the_frozen_information_bound_oracle() {
        let evaluation = result();
        assert_eq!(evaluation["candidate_count"], 11);
        assert_eq!(evaluation["verification_state"], "MatchedFrozenOracle");
    }

    #[test]
    fn orthogonal_excitation_passes_bayesian_and_robust_parameter_gates() {
        let candidate = row("BD-T0-001-orthogonal-pulses");
        assert_eq!(candidate["candidate_state"], "ELIGIBLE_BAYESIAN_DESIGN");
        assert_eq!(candidate["robust_minimum_rank"], TARGET_PARAMETER_COUNT);
        assert!(candidate["mutual_information_lower_bound_nats"].as_f64().unwrap() > 0.02);
    }

    #[test]
    fn synchronous_pulses_cannot_use_model_information_to_bypass_rank_deficiency() {
        let orthogonal = row("BD-T0-001-orthogonal-pulses");
        let synchronous = row("BD-T0-002-synchronous-pulses");
        assert!(
            synchronous["mutual_information_lower_bound_nats"]
                .as_f64()
                .unwrap()
                > orthogonal["mutual_information_lower_bound_nats"]
                    .as_f64()
                    .unwrap()
        );
        assert_eq!(synchronous["robust_minimum_rank"], 1);
        assert_eq!(synchronous["candidate_state"], "ROBUSTLY_UNIDENTIFIABLE");
    }

    #[test]
    fn low_dose_design_is_ranked_but_does_not_clear_the_bayesian_information_gate() {
        let candidate = row("BD-T0-003-low-dose-orthogonal");
        assert_eq!(candidate["robust_minimum_rank"], TARGET_PARAMETER_COUNT);
        assert_eq!(
            candidate["candidate_state"],
            "INSUFFICIENT_BAYESIAN_INFORMATION"
        );
    }

    #[test]
    fn information_lower_bound_is_bounded_by_prior_entropy() {
        let evaluation = result();
        let entropy = evaluation["prior_entropy_nats"].as_f64().unwrap();
        for candidate in evaluation["candidates"].as_array().unwrap() {
            if let Some(value) = candidate["mutual_information_lower_bound_nats"].as_f64() {
                assert!(value >= 0.0);
                assert!(value <= entropy + 1.0e-8);
            }
        }
    }

    #[test]
    fn high_frequency_candidate_is_refused_by_sensor_bandwidth() {
        assert_eq!(
            row("BD-T0-004-high-frequency-unresolvable")["candidate_state"],
            "SENSOR_BANDWIDTH_INADEQUATE"
        );
    }

    #[test]
    fn missing_outlet_boundary_is_refused() {
        assert_eq!(
            row("BD-T0-005-partial-outlet")["candidate_state"],
            "PARTIAL_OUTLET_BOUNDARY"
        );
    }

    #[test]
    fn excessive_worst_case_dose_is_refused_before_selection() {
        assert_eq!(
            row("BD-T0-006-dose-exceeded")["candidate_state"],
            "OPERATIONAL_CONSTRAINT_VIOLATION"
        );
    }

    #[test]
    fn uncalibrated_injector_and_command_only_inputs_are_refused() {
        assert_eq!(
            row("BD-T0-007-uncalibrated-injector")["candidate_state"],
            "INJECTOR_RESPONSE_UNRESOLVED"
        );
        assert_eq!(
            row("BD-T0-008-command-only")["candidate_state"],
            "INJECTOR_RESPONSE_UNRESOLVED"
        );
    }

    #[test]
    fn uncalibrated_sensor_response_is_refused() {
        assert_eq!(
            row("BD-T0-011-uncalibrated-sensor")["candidate_state"],
            "SENSOR_RESPONSE_UNRESOLVED"
        );
    }

    #[test]
    fn zero_excitation_has_zero_information_and_no_selection() {
        let candidate = row("BD-T0-009-zero-excitation");
        assert_eq!(candidate["mutual_information_lower_bound_nats"], 0.0);
        assert_eq!(candidate["candidate_state"], "ROBUSTLY_UNIDENTIFIABLE");
        assert_eq!(candidate["selected"], false);
    }

    #[test]
    fn truncated_time_window_is_refused() {
        assert_eq!(
            row("BD-T0-010-truncated-window")["candidate_state"],
            "PARTIAL_TEMPORAL_WINDOW"
        );
    }

    #[test]
    fn invalid_prior_weights_are_rejected() {
        let mut input = load("input");
        input["prior_particles"][0]["prior_weight"] = json!(0.0);
        assert!(evaluate(&input, &load("oracle"), &load("manifest")).is_err());
    }

    #[test]
    fn non_positive_definite_measurement_covariance_is_rejected() {
        let mut input = load("input");
        input["configuration"]["measurement_covariance"][0][0] = json!(-1.0);
        assert!(evaluate(&input, &load("oracle"), &load("manifest")).is_err());
    }
}
