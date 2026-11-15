// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

const PROGRAMME: &str = "WATER-MIXING-ACTIVE-EXPERIMENT-DESIGN-001";
const CASE_COUNT: usize = 1;
const EVIDENCE_CEILING: &str =
    "synthetic sensor-filtered excitation and identifiability arithmetic only";

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

fn boolean(value: &Value, key: &str) -> Result<bool, String> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("{key} must be boolean"))
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

fn parse_vector(value: &Value, key: &str) -> Result<Vector, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key} must be an array"))?
        .iter()
        .map(|v| {
            v.as_f64()
                .filter(|x| x.is_finite())
                .ok_or_else(|| format!("{key} contains a non-finite value"))
        })
        .collect()
}

fn parse_matrix(value: &Value, key: &str, rows: usize, cols: usize) -> Result<Matrix, String> {
    let raw = field(value, key)?
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

fn parse_tensor_value(
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
                return Err(format!("{key} kernel cannot have empty lag support"));
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

fn parse_tensor(value: &Value, key: &str, nout: usize, nin: usize) -> Result<Tensor3, String> {
    parse_tensor_value(field(value, key)?, key, nout, nin)
}

#[allow(clippy::needless_range_loop)]
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

#[allow(clippy::needless_range_loop)]
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
        return Err("SPD solve returned a non-finite result".into());
    }
    Ok(result)
}

#[allow(clippy::needless_range_loop)]
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
        || kernel
            .iter()
            .any(|out| out.len() != input.len() || out.iter().any(|entry| entry.len() != nlag))
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
        return Err("physical-kernel convolution overflow".into());
    }
    Ok(result)
}

fn flatten(channels: &Matrix) -> Vector {
    channels
        .iter()
        .flat_map(|row| row.iter().copied())
        .collect()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

#[allow(clippy::needless_range_loop)]
fn fisher_information(jacobian: &Matrix, covariance_lower: &Matrix) -> Result<Matrix, String> {
    let observations = jacobian.len();
    if observations == 0
        || jacobian.iter().any(|row| row.len() != jacobian[0].len())
        || covariance_lower.len() != observations
    {
        return Err("Fisher information dimensions mismatch".into());
    }
    let parameters = jacobian[0].len();
    let mut whitened = vec![vec![0.0; parameters]; observations];
    for parameter in 0..parameters {
        let rhs = (0..observations)
            .map(|row| jacobian[row][parameter])
            .collect::<Vector>();
        let solved = solve_spd(covariance_lower, &rhs)?;
        for row in 0..observations {
            whitened[row][parameter] = solved[row];
        }
    }
    let mut information = vec![vec![0.0; parameters]; parameters];
    for i in 0..parameters {
        for j in 0..parameters {
            for row in 0..observations {
                information[i][j] += jacobian[row][i] * whitened[row][j];
            }
        }
    }
    for i in 0..parameters {
        for j in 0..i {
            let mean = 0.5 * (information[i][j] + information[j][i]);
            information[i][j] = mean;
            information[j][i] = mean;
        }
    }
    Ok(information)
}

#[allow(clippy::needless_range_loop)]
fn symmetric_eigenvalues(mut matrix: Matrix) -> Result<Vector, String> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return Err("information matrix must be non-empty and square".into());
    }
    if n == 1 {
        return Ok(vec![matrix[0][0]]);
    }
    let scale = matrix
        .iter()
        .flatten()
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0e-300);
    let mut converged = false;
    for _ in 0..(100 * n * n) {
        let mut p = 0;
        let mut q = 1;
        let mut largest = 0.0_f64;
        for i in 0..n {
            for j in (i + 1)..n {
                if matrix[i][j].abs() > largest {
                    largest = matrix[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if largest <= 1.0e-13 * scale {
            converged = true;
            break;
        }
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
        let t = sign / (tau.abs() + (1.0 + tau * tau).sqrt());
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        for k in 0..n {
            if k != p && k != q {
                let kp = matrix[k][p];
                let kq = matrix[k][q];
                matrix[k][p] = c * kp - s * kq;
                matrix[p][k] = matrix[k][p];
                matrix[k][q] = s * kp + c * kq;
                matrix[q][k] = matrix[k][q];
            }
        }
        matrix[p][p] = app - t * apq;
        matrix[q][q] = aqq + t * apq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
    }
    if !converged {
        return Err("symmetric Jacobi eigensolver did not converge".into());
    }
    let mut values = (0..n).map(|i| matrix[i][i]).collect::<Vector>();
    let magnitude = values
        .iter()
        .map(|v| v.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    for value in &mut values {
        if *value < -1.0e-9 * magnitude {
            return Err("information matrix has a materially negative eigenvalue".into());
        }
        if *value < 0.0 {
            *value = 0.0;
        }
    }
    values.sort_by(|a, b| b.total_cmp(a));
    Ok(values)
}

fn information_metrics(information: &Matrix, rank_tolerance: f64) -> Result<Value, String> {
    let eigenvalues = symmetric_eigenvalues(information.clone())?;
    let singular = eigenvalues.iter().map(|v| v.sqrt()).collect::<Vector>();
    let maximum = singular.first().copied().unwrap_or(0.0);
    let rank = if maximum <= 0.0 {
        0
    } else {
        singular
            .iter()
            .filter(|value| **value > maximum * rank_tolerance)
            .count()
    };
    let full_rank = rank == singular.len() && singular.last().copied().unwrap_or(0.0) > 0.0;
    let condition = if full_rank {
        Some(maximum / singular[singular.len() - 1])
    } else {
        None
    };
    let logdet = if full_rank {
        Some(eigenvalues.iter().map(|value| value.ln()).sum::<f64>())
    } else {
        None
    };
    Ok(json!({
        "singular_values": singular,
        "rank": rank,
        "condition_number": condition,
        "log_determinant": logdet,
        "minimum_singular_value": if full_rank { singular[singular.len() - 1] } else { 0.0 }
    }))
}

fn submatrix(
    matrix: &Matrix,
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
) -> Matrix {
    (row_start..row_end)
        .map(|row| (col_start..col_end).map(|col| matrix[row][col]).collect())
        .collect()
}

#[allow(clippy::needless_range_loop)]
fn nuisance_adjusted(information: &Matrix, target_count: usize) -> Result<Option<Matrix>, String> {
    let parameters = information.len();
    if target_count == parameters {
        return Ok(Some(information.clone()));
    }
    if target_count == 0 || target_count > parameters {
        return Err("target parameter count is out of range".into());
    }
    let ftt = submatrix(information, 0, target_count, 0, target_count);
    let ftn = submatrix(information, 0, target_count, target_count, parameters);
    let fnn = submatrix(
        information,
        target_count,
        parameters,
        target_count,
        parameters,
    );
    let fnt = submatrix(information, target_count, parameters, 0, target_count);
    let lower = match cholesky(&fnn) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let nuisance_count = parameters - target_count;
    let mut adjusted = ftt;
    for col in 0..target_count {
        let rhs = (0..nuisance_count)
            .map(|row| fnt[row][col])
            .collect::<Vector>();
        let solved = solve_spd(&lower, &rhs)?;
        for row in 0..target_count {
            adjusted[row][col] -= dot(&ftn[row], &solved);
        }
    }
    for i in 0..target_count {
        for j in 0..i {
            let mean = 0.5 * (adjusted[i][j] + adjusted[j][i]);
            adjusted[i][j] = mean;
            adjusted[j][i] = mean;
        }
    }
    Ok(Some(adjusted))
}

fn base_candidate(state: &str, reason: &str) -> Value {
    json!({
        "candidate_state": state,
        "reason": reason,
        "actual_inlet_mass_flux": null,
        "injected_mass_kg": null,
        "maximum_inlet_mass_flux": null,
        "maximum_inlet_slew_kg_s2": null,
        "duration_seconds": null,
        "target_information": null,
        "design_rank": null,
        "condition_number": null,
        "minimum_singular_value": null,
        "log_determinant": null,
        "pairwise_model_separations": null,
        "minimum_model_separation": null,
        "pareto_optimal": false,
        "selected": false,
        "physical_operator_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

#[allow(clippy::too_many_arguments)]
fn compute_candidate(
    candidate: &Value,
    physics: &Value,
    actuators: &Value,
    sensors: &Value,
    covariance_lower: &Matrix,
    dt: f64,
    parameter_scales: &[f64],
    target_count: usize,
    rank_tolerance: f64,
    minimum_delay: f64,
    limits: &Value,
) -> Result<Value, String> {
    if text(candidate, "waveform_provenance")? != "command_plus_calibrated_actuator_model" {
        return Ok(base_candidate(
            "INJECTOR_RESPONSE_UNRESOLVED",
            "COMMAND_ONLY_OR_UNSUPPORTED_WAVEFORM_PROVENANCE",
        ));
    }
    if text(candidate, "actuator_calibration_state")? != "calibrated" {
        return Ok(base_candidate(
            "INJECTOR_RESPONSE_UNRESOLVED",
            "ACTUATOR_RESPONSE_NOT_CALIBRATED",
        ));
    }
    if text(sensors, "calibration_state")? != "calibrated" {
        return Ok(base_candidate(
            "SENSOR_RESPONSE_UNRESOLVED",
            "SENSOR_RESPONSE_NOT_CALIBRATED",
        ));
    }
    if !boolean(candidate, "all_inlets_observed")? {
        return Ok(base_candidate(
            "PARTIAL_INLET_BOUNDARY",
            "ONE_OR_MORE_INLET_MASS_FLUXES_NOT_OBSERVED",
        ));
    }
    if !boolean(candidate, "all_outlets_observed")? {
        return Ok(base_candidate(
            "PARTIAL_OUTLET_BOUNDARY",
            "ONE_OR_MORE_OUTLET_CHANNELS_NOT_OBSERVED",
        ));
    }
    if !boolean(candidate, "temporal_boundary_complete")?
        || !boolean(candidate, "include_full_tail")?
    {
        return Ok(base_candidate(
            "PARTIAL_TEMPORAL_WINDOW",
            "COMPLETE_SENSOR_AND_PHYSICAL_TAIL_NOT_INCLUDED",
        ));
    }
    let minimum_feature = number(candidate, "minimum_feature_period_seconds")?;
    let sensor_resolution = number(sensors, "minimum_resolvable_delay_seconds")?;
    if minimum_feature <= 0.0 || sensor_resolution <= 0.0 {
        return Ok(base_candidate(
            "INVALID_INPUT",
            "SENSOR_OR_FEATURE_RESOLUTION_NOT_POSITIVE",
        ));
    }
    if minimum_feature < sensor_resolution {
        return Ok(base_candidate(
            "SENSOR_BANDWIDTH_INADEQUATE",
            "EXCITATION_FEATURE_FASTER_THAN_CALIBRATED_SENSOR_RESOLUTION",
        ));
    }
    if dt > minimum_delay / 10.0 {
        return Ok(base_candidate(
            "SENSOR_SAMPLING_INADEQUATE",
            "SAMPLE_INTERVAL_EXCEEDS_ONE_TENTH_OF_SHORTEST_FEATURE",
        ));
    }

    let command_rows = array(candidate, "commanded_inlet_mass_flux")?;
    if command_rows.is_empty() {
        return Ok(base_candidate(
            "INVALID_INPUT",
            "NO_COMMANDED_INLET_CHANNELS",
        ));
    }
    let nin = command_rows.len();
    let ntime = command_rows[0].as_array().map(Vec::len).unwrap_or(0);
    if ntime == 0 {
        return Ok(base_candidate(
            "INVALID_INPUT",
            "EMPTY_COMMANDED_INLET_SERIES",
        ));
    }
    let command = parse_matrix(candidate, "commanded_inlet_mass_flux", nin, ntime)?;
    if command.iter().flatten().any(|v| *v < 0.0) {
        return Ok(base_candidate(
            "INVALID_INPUT",
            "NEGATIVE_COMMANDED_MASS_FLUX",
        ));
    }
    let actuator_response = parse_matrix(
        actuators,
        "response",
        nin,
        array(actuators, "response")?
            .first()
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
    )?;
    if actuator_response.iter().flatten().any(|v| *v < 0.0) {
        return Ok(base_candidate(
            "INVALID_INPUT",
            "NEGATIVE_ACTUATOR_RESPONSE",
        ));
    }
    for row in &actuator_response {
        let area = row.iter().sum::<f64>();
        if (area - 1.0).abs() > 1.0e-8 {
            return Ok(base_candidate(
                "REJECTED_CONTRACT",
                "ACTUATOR_RESPONSE_MUST_HAVE_UNIT_AREA",
            ));
        }
    }
    let actual_input = convolve_channels(&command, &actuator_response)?;
    let injected_mass = actual_input
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
    let output_count = parse_matrix(
        sensors,
        "response",
        array(sensors, "response")?.len(),
        array(sensors, "response")?
            .first()
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
    )?
    .len();
    if output_count == 0 {
        return Ok(base_candidate("INVALID_INPUT", "NO_SENSOR_OUTPUT_CHANNELS"));
    }
    let kernel = parse_tensor(physics, "kernel_at_reference", output_count, nin)?;
    if kernel.iter().flatten().flatten().any(|v| *v < 0.0) {
        return Ok(base_candidate(
            "INVALID_KERNEL",
            "REFERENCE_KERNEL_HAS_NEGATIVE_COEFFICIENT",
        ));
    }
    let mut inlet_gain = vec![0.0; nin];
    for (inlet, total_gain) in inlet_gain.iter_mut().enumerate() {
        for outlet in kernel.iter().take(output_count) {
            *total_gain += outlet[inlet].iter().sum::<f64>();
        }
    }
    if inlet_gain.iter().any(|v| *v > 1.0 + 1.0e-9) {
        return Ok(base_candidate(
            "INVALID_KERNEL",
            "REFERENCE_KERNEL_PASSIVE_GAIN_EXCEEDS_ONE",
        ));
    }
    let sensor_response = parse_matrix(
        sensors,
        "response",
        output_count,
        array(sensors, "response")?
            .first()
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0),
    )?;
    if sensor_response.iter().flatten().any(|v| *v < 0.0) {
        return Ok(base_candidate("INVALID_INPUT", "NEGATIVE_SENSOR_RESPONSE"));
    }
    for row in &sensor_response {
        if (row.iter().sum::<f64>() - 1.0).abs() > 1.0e-8 {
            return Ok(base_candidate(
                "REJECTED_CONTRACT",
                "SENSOR_RESPONSE_MUST_HAVE_UNIT_AREA",
            ));
        }
    }

    let maximum_dose = number(limits, "maximum_injected_mass_kg")?;
    let maximum_rate_limit = number(limits, "maximum_inlet_mass_flux")?;
    let maximum_slew_limit = number(limits, "maximum_inlet_slew_kg_s2")?;
    let maximum_duration = number(limits, "maximum_duration_seconds")?;
    let duration =
        (actual_input[0].len() + kernel[0][0].len() + sensor_response[0].len() - 2) as f64 * dt;
    if injected_mass > maximum_dose
        || maximum_rate > maximum_rate_limit
        || maximum_slew > maximum_slew_limit
        || duration > maximum_duration
    {
        let reason = if injected_mass > maximum_dose {
            "INJECTION_DOSE_LIMIT_EXCEEDED"
        } else if maximum_rate > maximum_rate_limit {
            "INLET_MASS_FLUX_LIMIT_EXCEEDED"
        } else if maximum_slew > maximum_slew_limit {
            "INLET_SLEW_LIMIT_EXCEEDED"
        } else {
            "EXPERIMENT_DURATION_LIMIT_EXCEEDED"
        };
        return Ok(base_candidate("OPERATIONAL_CONSTRAINT_VIOLATION", reason));
    }

    let physical = convolve_kernel(&actual_input, &kernel)?;
    let _baseline_measured = convolve_channels(&physical, &sensor_response)?;
    let directions_raw = array(physics, "parameter_kernel_directions")?;
    if directions_raw.len() != parameter_scales.len() {
        return Err("parameter direction count differs from parameter scales".into());
    }
    let mut jacobian_columns = Vec::with_capacity(parameter_scales.len());
    for (parameter, scale) in directions_raw.iter().zip(parameter_scales) {
        let direction =
            parse_tensor_value(parameter, "parameter_kernel_direction", output_count, nin)?;
        let physical_sensitivity = convolve_kernel(&actual_input, &direction)?;
        let measured_sensitivity = convolve_channels(&physical_sensitivity, &sensor_response)?;
        jacobian_columns.push(
            flatten(&measured_sensitivity)
                .into_iter()
                .map(|value| value * scale)
                .collect::<Vector>(),
        );
    }
    let observation_count = jacobian_columns[0].len();
    if covariance_lower.len() != observation_count {
        return Err(format!(
            "measurement covariance dimension mismatch: expected {observation_count}, got {}",
            covariance_lower.len()
        ));
    }
    let mut jacobian = vec![vec![0.0; jacobian_columns.len()]; observation_count];
    for (column, values) in jacobian_columns.iter().enumerate() {
        for row in 0..observation_count {
            jacobian[row][column] = values[row];
        }
    }
    let fisher = fisher_information(&jacobian, covariance_lower)?;
    let Some(target_information) = nuisance_adjusted(&fisher, target_count)? else {
        return Ok(base_candidate(
            "NUISANCE_UNRESOLVED",
            "NUISANCE_INFORMATION_BLOCK_NOT_POSITIVE_DEFINITE",
        ));
    };
    let target_metrics = information_metrics(&target_information, rank_tolerance)?;
    let design_rank = usize_value(&target_metrics, "rank")?;
    let minimum_singular = number(&target_metrics, "minimum_singular_value")?;
    let condition = target_metrics
        .get("condition_number")
        .and_then(Value::as_f64);
    let logdet = target_metrics
        .get("log_determinant")
        .and_then(Value::as_f64);

    let models = array(physics, "competing_models")?;
    if models.len() < 2 {
        return Err("at least two competing model kernels are required".into());
    }
    let mut model_predictions = Vec::with_capacity(models.len());
    for model in models {
        let model_id = text(model, "model_id")?;
        let model_kernel = parse_tensor(model, "kernel", output_count, nin)?;
        if model_kernel.iter().flatten().flatten().any(|v| *v < 0.0) {
            return Ok(base_candidate(
                "INVALID_KERNEL",
                "COMPETING_MODEL_HAS_NEGATIVE_COEFFICIENT",
            ));
        }
        let mut gain = vec![0.0; nin];
        for (inlet, total_gain) in gain.iter_mut().enumerate() {
            for outlet in model_kernel.iter().take(output_count) {
                *total_gain += outlet[inlet].iter().sum::<f64>();
            }
        }
        if gain.iter().any(|v| *v > 1.0 + 1.0e-9) {
            return Ok(base_candidate(
                "INVALID_KERNEL",
                "COMPETING_MODEL_PASSIVE_GAIN_EXCEEDS_ONE",
            ));
        }
        let response = convolve_kernel(&actual_input, &model_kernel)?;
        let measured = convolve_channels(&response, &sensor_response)?;
        model_predictions.push((model_id.to_owned(), flatten(&measured)));
    }
    let mut pairwise = Vec::new();
    let mut minimum_separation = f64::INFINITY;
    for i in 0..model_predictions.len() {
        for j in (i + 1)..model_predictions.len() {
            let delta = model_predictions[i]
                .1
                .iter()
                .zip(&model_predictions[j].1)
                .map(|(a, b)| a - b)
                .collect::<Vector>();
            let solved = solve_spd(covariance_lower, &delta)?;
            let separation = dot(&delta, &solved).max(0.0).sqrt();
            minimum_separation = minimum_separation.min(separation);
            pairwise.push(json!({
                "model_a": model_predictions[i].0,
                "model_b": model_predictions[j].0,
                "mahalanobis_separation": separation
            }));
        }
    }

    let maximum_condition = number(limits, "maximum_condition_number")?;
    let minimum_information = number(limits, "minimum_singular_value")?;
    let minimum_separation_limit = number(limits, "minimum_model_separation")?;
    let state = if design_rank < target_count {
        "UNIDENTIFIABLE_DESIGN"
    } else if condition.is_none_or(|value| value > maximum_condition) {
        "ILL_CONDITIONED_DESIGN"
    } else if minimum_singular < minimum_information {
        "INSUFFICIENT_INFORMATION"
    } else if minimum_separation < minimum_separation_limit {
        "MODEL_DISCRIMINATION_INSUFFICIENT"
    } else {
        "ELIGIBLE_DESIGN_CANDIDATE"
    };
    Ok(json!({
        "candidate_state": state,
        "reason": null,
        "actual_inlet_mass_flux": actual_input,
        "injected_mass_kg": injected_mass,
        "maximum_inlet_mass_flux": maximum_rate,
        "maximum_inlet_slew_kg_s2": maximum_slew,
        "duration_seconds": duration,
        "target_information": target_metrics,
        "design_rank": design_rank,
        "condition_number": condition,
        "minimum_singular_value": minimum_singular,
        "log_determinant": logdet,
        "pairwise_model_separations": pairwise,
        "minimum_model_separation": minimum_separation,
        "pareto_optimal": false,
        "selected": false,
        "physical_operator_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false
    }))
}

fn dominates(left: &Value, right: &Value) -> bool {
    let metric = |value: &Value, key: &str, fallback: f64| {
        value.get(key).and_then(Value::as_f64).unwrap_or(fallback)
    };
    let left_min = metric(left, "minimum_singular_value", 0.0);
    let right_min = metric(right, "minimum_singular_value", 0.0);
    let left_sep = metric(left, "minimum_model_separation", 0.0);
    let right_sep = metric(right, "minimum_model_separation", 0.0);
    let left_dose = metric(left, "injected_mass_kg", f64::INFINITY);
    let right_dose = metric(right, "injected_mass_kg", f64::INFINITY);
    let left_duration = metric(left, "duration_seconds", f64::INFINITY);
    let right_duration = metric(right, "duration_seconds", f64::INFINITY);
    let no_worse = left_min >= right_min
        && left_sep >= right_sep
        && left_dose <= right_dose
        && left_duration <= right_duration;
    let strictly = left_min > right_min
        || left_sep > right_sep
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
    if text(input, "schema_version")? != "active-excitation-t0-input-v0.1"
        || text(oracle, "schema_version")? != "active-excitation-t0-oracle-v0.1"
        || text(manifest, "schema_version")? != "active-excitation-t0-manifest-v0.1"
    {
        return Err("unexpected active-excitation schema".into());
    }
    for document in [input, oracle, manifest] {
        if text(document, "programme_id")? != PROGRAMME {
            return Err("active-excitation programme identity mismatch".into());
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
        return Err("frozen active-excitation case count mismatch".into());
    }
    let collect = |rows: &[Value], key: &str| -> Result<BTreeSet<String>, String> {
        rows.iter()
            .map(|value| text(value, key).map(str::to_owned))
            .collect()
    };
    let input_ids = collect(cases, "case_id")?;
    let expected_ids = collect(expected, "case_id")?;
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
        || expected_ids.len() != expected.len()
        || manifest_ids.len() != case_ids.len()
        || input_ids != expected_ids
        || input_ids != manifest_ids
    {
        return Err("input/oracle/manifest case identities differ".into());
    }

    let config = field(input, "configuration")?;
    let physics = field(input, "physics")?;
    let actuators = field(input, "actuators")?;
    let sensors = field(input, "sensors")?;
    if text(actuators, "calibration_state")? != "calibrated" {
        return Err("global actuator response must be calibrated".into());
    }
    if text(sensors, "calibration_state")? != "calibrated" {
        return Err("global sensor response must be calibrated".into());
    }
    let dt = number(config, "sample_interval_seconds")?;
    let minimum_delay = number(config, "minimum_delay_of_interest_seconds")?;
    if dt <= 0.0 || minimum_delay <= 0.0 {
        return Err("sample interval and minimum delay must be positive".into());
    }
    let rank_tolerance = number(config, "rank_relative_tolerance")?;
    if rank_tolerance <= 0.0 || rank_tolerance >= 0.1 {
        return Err("rank tolerance outside supported interval".into());
    }
    let target_count = usize_value(config, "target_parameter_count")?;
    let parameter_scales = parse_vector(config, "parameter_scales")?;
    if target_count == 0 || target_count > parameter_scales.len() {
        return Err("target parameter count is invalid".into());
    }
    if parameter_scales.iter().any(|v| *v <= 0.0) {
        return Err("parameter scales must be positive".into());
    }
    let sensor_rows = array(sensors, "response")?;
    let output_count = sensor_rows.len();
    let sensor_lags = sensor_rows
        .first()
        .and_then(Value::as_array)
        .map(Vec::len)
        .unwrap_or(0);
    if output_count == 0 || sensor_lags == 0 {
        return Err("sensor response matrix cannot be empty".into());
    }
    let covariance_raw = array(config, "measurement_covariance")?;
    let covariance_size = covariance_raw.len();
    let covariance = parse_matrix(
        config,
        "measurement_covariance",
        covariance_size,
        covariance_size,
    )?;
    let covariance_lower = cholesky(&covariance)?;
    let candidates = array(&cases[0], "candidates")?;
    if candidates.is_empty() {
        return Err("candidate pool cannot be empty".into());
    }
    let mut results = Vec::new();
    for candidate in candidates {
        results.push(compute_candidate(
            candidate,
            physics,
            actuators,
            sensors,
            &covariance_lower,
            dt,
            &parameter_scales,
            target_count,
            rank_tolerance,
            minimum_delay,
            field(config, "limits")?,
        )?);
        let last = results
            .last_mut()
            .ok_or_else(|| "missing candidate result".to_owned())?;
        last["design_id"] = json!(text(candidate, "design_id")?);
    }
    for i in 0..results.len() {
        if results[i]["candidate_state"] != "ELIGIBLE_DESIGN_CANDIDATE" {
            continue;
        }
        let dominated = (0..results.len()).any(|j| {
            i != j
                && results[j]["candidate_state"] == "ELIGIBLE_DESIGN_CANDIDATE"
                && dominates(&results[j], &results[i])
        });
        results[i]["pareto_optimal"] = json!(!dominated);
    }
    let selected_index = results
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            row["candidate_state"] == "ELIGIBLE_DESIGN_CANDIDATE" && row["pareto_optimal"] == true
        })
        .max_by(|(_, left), (_, right)| {
            let left_min = left["minimum_singular_value"].as_f64().unwrap_or(0.0);
            let right_min = right["minimum_singular_value"].as_f64().unwrap_or(0.0);
            left_min
                .total_cmp(&right_min)
                .then_with(|| {
                    left["minimum_model_separation"]
                        .as_f64()
                        .unwrap_or(0.0)
                        .total_cmp(&right["minimum_model_separation"].as_f64().unwrap_or(0.0))
                })
                .then_with(|| {
                    right["injected_mass_kg"]
                        .as_f64()
                        .unwrap_or(f64::INFINITY)
                        .total_cmp(&left["injected_mass_kg"].as_f64().unwrap_or(f64::INFINITY))
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
        "schema_version": "active-excitation-t0-evaluation-v0.1",
        "verification_state": if matched { "MatchedFrozenOracle" } else { "OracleMismatch" },
        "evidence_class": "synthetic_only",
        "evidence_ceiling": EVIDENCE_CEILING,
        "candidate_count": results.len(),
        "eligible_candidate_count": results.iter().filter(|row| row["candidate_state"] == "ELIGIBLE_DESIGN_CANDIDATE").count(),
        "selected_design_id": selected_index.map(|index| results[index]["design_id"].clone()),
        "pareto_design_ids": results.iter().filter(|row| row["pareto_optimal"] == true).map(|row| row["design_id"].clone()).collect::<Vec<_>>(),
        "mismatches": mismatches,
        "candidates": results,
        "physical_operator_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false,
        "nonclaims": [
            "Candidate commands are propagated through a calibrated actuator response to form predicted delivered inlet mass flux; this is not measured run data.",
            "A physical campaign must replace actuator-predicted inlet mass flux with independently measured inlet mass flux before model fitting or physical qualification.",
            "Fisher information and Mahalanobis separation are local synthetic proxies, not expected information gain or physical validation.",
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
    let value =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse {}: {e}", path.display()))?;
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
        return Err("usage: active-excitation-t0 [INPUT] [ORACLE] [MANIFEST] [RECEIPT]".into());
    }
    let input_path = PathBuf::from(
        args.first()
            .map(String::as_str)
            .unwrap_or("research/water-intrusion-coupling/active-excitation-t0-cases-v0.1.json"),
    );
    let oracle_path = PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("research/water-intrusion-coupling/active-excitation-t0-oracle-v0.1.json"),
    );
    let manifest_path =
        PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
            "research/water-intrusion-coupling/active-excitation-t0-manifest-v0.1.json",
        ));
    let receipt_path = PathBuf::from(
        args.get(3)
            .map(String::as_str)
            .unwrap_or("artifacts/water-mixing-active-excitation-t0/receipt.json"),
    );
    let (input_bytes, input) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let evaluation = evaluate(&input, &oracle, &manifest)?;
    let success = evaluation["verification_state"] == "MatchedFrozenOracle";
    let revision = env::var("WATER_MIXING_ACTIVE_EXCITATION_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes =
        fs::read(&lock_path).map_err(|e| format!("read {}: {e}", lock_path.display()))?;
    let receipt = json!({
        "schema_version": "active-excitation-t0-run-v0.1",
        "verification_state": if !success { "OracleMismatch" } else if revision.is_some() { "PassedExactRevision" } else { "PassedLocalUnpinned" },
        "implementation_revision": revision,
        "input_sha256": digest(&input_bytes),
        "oracle_sha256": digest(&oracle_bytes),
        "manifest_sha256": digest(&manifest_bytes),
        "cargo_lock_sha256": digest(&lock_bytes),
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
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
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
            eprintln!("WATER_MIXING_ACTIVE_EXCITATION_T0_ERROR: {error}");
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
                "../../../../research/water-intrusion-coupling/active-excitation-t0-cases-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../../research/water-intrusion-coupling/active-excitation-t0-oracle-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../../research/water-intrusion-coupling/active-excitation-t0-manifest-v0.1.json"
            ),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(raw).expect("active excitation fixture parses")
    }

    fn result() -> Value {
        evaluate(&load("input"), &load("oracle"), &load("manifest"))
            .expect("active excitation evaluator runs")
    }

    fn row(id: &str) -> Value {
        result()["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["design_id"] == id)
            .unwrap()
            .clone()
    }

    #[test]
    fn all_candidates_match_the_frozen_oracle() {
        let evaluation = result();
        assert_eq!(evaluation["candidate_count"], 9);
        assert_eq!(evaluation["verification_state"], "MatchedFrozenOracle");
    }

    #[test]
    fn orthogonal_pulses_are_identifiable_after_actuator_and_sensor_filtering() {
        let candidate = row("AE-T0-001-orthogonal-pulses");
        assert_eq!(candidate["candidate_state"], "ELIGIBLE_DESIGN_CANDIDATE");
        assert_eq!(candidate["design_rank"], 2);
        assert!(candidate["minimum_singular_value"].as_f64().unwrap() > 0.5);
    }

    #[test]
    fn synchronous_identical_inlet_pulses_are_unidentifiable() {
        let candidate = row("AE-T0-002-synchronous-pulses");
        assert_eq!(candidate["candidate_state"], "UNIDENTIFIABLE_DESIGN");
        assert_eq!(candidate["design_rank"], 1);
    }

    #[test]
    fn sensor_bandwidth_refuses_unresolvable_excitation() {
        assert_eq!(
            row("AE-T0-003-above-sensor-bandwidth")["candidate_state"],
            "SENSOR_BANDWIDTH_INADEQUATE"
        );
    }

    #[test]
    fn missing_outlet_boundary_is_not_scored_as_identifiable() {
        assert_eq!(
            row("AE-T0-004-partial-outlet-boundary")["candidate_state"],
            "PARTIAL_OUTLET_BOUNDARY"
        );
    }

    #[test]
    fn calibrated_actuator_is_required_for_command_to_delivered_waveform_prediction() {
        assert_eq!(
            row("AE-T0-005-uncalibrated-injector")["candidate_state"],
            "INJECTOR_RESPONSE_UNRESOLVED"
        );
    }

    #[test]
    fn excessive_dose_and_flow_are_refused_before_information_scoring() {
        assert_eq!(
            row("AE-T0-006-over-limit-dose-and-rate")["candidate_state"],
            "OPERATIONAL_CONSTRAINT_VIOLATION"
        );
    }

    #[test]
    fn low_dose_orthogonal_design_fails_the_declared_information_gate() {
        assert_eq!(
            row("AE-T0-007-low-dose-low-information")["candidate_state"],
            "INSUFFICIENT_INFORMATION"
        );
    }

    #[test]
    fn actuator_filtered_inlet_mass_is_conserved() {
        let candidate = row("AE-T0-001-orthogonal-pulses");
        let actual = candidate["actual_inlet_mass_flux"].as_array().unwrap();
        let mass: f64 = actual
            .iter()
            .map(|row| {
                row.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .sum::<f64>()
            })
            .sum();
        assert!((mass - 1.6).abs() < 1.0e-10);
    }

    #[test]
    fn selection_is_pareto_qualified_and_not_a_single_metric_only() {
        let evaluation = result();
        assert_eq!(
            evaluation["selected_design_id"],
            "AE-T0-001-orthogonal-pulses"
        );
        assert!(
            row("AE-T0-001-orthogonal-pulses")["pareto_optimal"]
                .as_bool()
                .unwrap()
        );
        assert!(
            row("AE-T0-001-orthogonal-pulses")["selected"]
                .as_bool()
                .unwrap()
        );
    }

    #[test]
    fn command_only_waveforms_are_not_used_as_delivered_mass_flux() {
        assert_eq!(
            row("AE-T0-008-command-only")["candidate_state"],
            "INJECTOR_RESPONSE_UNRESOLVED"
        );
    }

    #[test]
    fn truncated_physical_or_sensor_tail_is_refused() {
        assert_eq!(
            row("AE-T0-009-truncated-temporal-window")["candidate_state"],
            "PARTIAL_TEMPORAL_WINDOW"
        );
    }
}
