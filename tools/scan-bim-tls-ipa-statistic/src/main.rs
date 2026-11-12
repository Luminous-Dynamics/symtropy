// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

const METHOD_ID: &str = "ipa_part_ii_obrien_robust_omnibus_v1";
const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_ROWS_PER_PERIOD: usize = 10_000;
const MIN_TARGETS: usize = 20;
const MIN_POSITIONS: usize = 4;
const DIMENSIONS: usize = 6;
const CLAIM_CEILING: &str = "Synthetic residual-consumer reproduction only. This does not establish TLS calibration, absolute accuracy, survey fitness, return-to-service authority, or physical-building truth.";

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema_version: String,
    input_id: String,
    alpha: f64,
    baseline: Period,
    testing: Period,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InstrumentScope {
    manufacturer: String,
    model: String,
    serial_id: String,
    firmware_version: String,
    profile_id: String,
    profile_sha256: String,
    calibration_reference_sha256: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SetupScope {
    acquisition_config_sha256: String,
    residual_producer_id: String,
    residual_producer_sha256: String,
    centralization_method_id: String,
    centralization_method_sha256: String,
    target_lineage_sha256: String,
    angle_unit: String,
    range_unit: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Period {
    period_id: String,
    timestamp_utc: String,
    instrument: InstrumentScope,
    setup: SetupScope,
    residual_artifact_sha256: String,
    centralization_receipt_sha256: String,
    target_layout_sha256: String,
    station_manifest_sha256: String,
    target_ids: Vec<String>,
    position_ids: Vec<String>,
    rows: Vec<ResidualRow>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResidualRow {
    row_id: String,
    target_id: String,
    position_id: String,
    theta_residual: f64,
    phi_residual: f64,
    range_residual: f64,
}

#[derive(Debug, Clone, Serialize)]
struct Report {
    schema_version: String,
    input_id: String,
    method_id: String,
    method_reference: String,
    input_sha256: String,
    disposition: String,
    reason_codes: Vec<String>,
    alpha: f64,
    baseline_period_id: String,
    testing_period_id: String,
    baseline_residual_artifact_sha256: String,
    testing_residual_artifact_sha256: String,
    baseline_row_count: usize,
    testing_row_count: usize,
    baseline_covariance: Option<CovarianceSummary>,
    testing_covariance: Option<CovarianceSummary>,
    robust_omnibus: Option<RobustOmnibus>,
    accuracy_disposition: String,
    task_fitness_disposition: String,
    assumption_notes: Vec<String>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize)]
struct CovarianceSummary {
    mean_residual: [f64; 3],
    covariance: [[f64; 3]; 3],
    standard_deviation: [f64; 3],
    correlation: [[f64; 3]; 3],
    one_sigma_ellipses: [Ellipse2d; 3],
}

#[derive(Debug, Clone, Serialize)]
struct Ellipse2d {
    axes: String,
    center: [f64; 2],
    major_standard_deviation: f64,
    minor_standard_deviation: f64,
    major_axis_angle_rad: f64,
}

#[derive(Debug, Clone, Serialize)]
struct RobustOmnibus {
    statistic: String,
    transformed_dimension_count: usize,
    baseline_observation_count: usize,
    testing_observation_count: usize,
    hotelling_t_squared: f64,
    f_statistic: f64,
    numerator_degrees_of_freedom: usize,
    denominator_degrees_of_freedom: usize,
    p_value: f64,
    pooled_covariance_condition: String,
    significance_interpretation: String,
}

const ASSUMPTIONS: &[&str] = &[
    "The residual producer has already performed the prescribed centralized-frame and back-transformation procedure; this consumer does not estimate transforms or verify that procedure.",
    "Each target-position residual row is treated as an observation for the two-group multivariate test. Within-target, within-position, temporal, or other dependence is not estimated by this P0 consumer.",
    "The pooled six-dimensional transformed covariance must be positive definite. Singular or numerically near-singular matrices are INDETERMINATE; no jitter, shrinkage, or diagonal loading is applied.",
    "A non-significant result is failure to reject equality of covariance, not proof that precision is unchanged.",
    "Precision change is not an accuracy, calibration, task-fitness, or return-to-service conclusion.",
];

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_timestamp_utc(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
    {
        return false;
    }

    let Ok(year) = value[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u32>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u32>() else {
        return false;
    };
    let Ok(hour) = value[11..13].parse::<u32>() else {
        return false;
    };
    let Ok(minute) = value[14..16].parse::<u32>() else {
        return false;
    };
    let Ok(second) = value[17..19].parse::<u32>() else {
        return false;
    };
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=month_days).contains(&day)
}

fn unique_nonempty(values: &[String]) -> bool {
    !values.is_empty()
        && values.iter().all(|value| !value.trim().is_empty())
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn validate_period(period: &Period, name: &str) -> Result<(), String> {
    if period.period_id.trim().is_empty() {
        return Err(format!("{name}_PERIOD_ID_MISSING"));
    }
    if !valid_timestamp_utc(&period.timestamp_utc) {
        return Err(format!("{name}_TIMESTAMP_INVALID"));
    }
    if !is_sha256(&period.residual_artifact_sha256)
        || !is_sha256(&period.centralization_receipt_sha256)
        || !is_sha256(&period.target_layout_sha256)
        || !is_sha256(&period.station_manifest_sha256)
    {
        return Err(format!("{name}_PRODUCER_DIGEST_INVALID"));
    }
    if !unique_nonempty(&period.target_ids) || period.target_ids.len() < MIN_TARGETS {
        return Err(format!("{name}_TARGET_MANIFEST_INSUFFICIENT_OR_INVALID"));
    }
    if !unique_nonempty(&period.position_ids) || period.position_ids.len() < MIN_POSITIONS {
        return Err(format!("{name}_POSITION_MANIFEST_INSUFFICIENT_OR_INVALID"));
    }
    if period.rows.len() > MAX_ROWS_PER_PERIOD
        || period.rows.len() != period.target_ids.len() * period.position_ids.len()
    {
        return Err(format!(
            "{name}_RESIDUAL_ROW_COUNT_DOES_NOT_MATCH_TARGET_POSITION_GRID"
        ));
    }

    let targets: BTreeSet<&str> = period.target_ids.iter().map(String::as_str).collect();
    let positions: BTreeSet<&str> = period.position_ids.iter().map(String::as_str).collect();
    let mut row_ids = BTreeSet::new();
    let mut pairs = BTreeSet::new();

    for row in &period.rows {
        if row.row_id.trim().is_empty() || !row_ids.insert(row.row_id.as_str()) {
            return Err(format!("{name}_ROW_ID_MISSING_OR_DUPLICATED"));
        }
        if !targets.contains(row.target_id.as_str())
            || !positions.contains(row.position_id.as_str())
        {
            return Err(format!(
                "{name}_ROW_OUTSIDE_DECLARED_TARGET_POSITION_MANIFEST"
            ));
        }
        if !pairs.insert((row.target_id.as_str(), row.position_id.as_str())) {
            return Err(format!("{name}_TARGET_POSITION_PAIR_DUPLICATED"));
        }
        if !row.theta_residual.is_finite()
            || !row.phi_residual.is_finite()
            || !row.range_residual.is_finite()
        {
            return Err(format!("{name}_NON_FINITE_SPHERICAL_RESIDUAL"));
        }
    }

    if pairs.len() != period.target_ids.len() * period.position_ids.len() {
        return Err(format!("{name}_TARGET_POSITION_GRID_INCOMPLETE"));
    }

    Ok(())
}

fn validate_input(input: &Input) -> Result<(), String> {
    if input.schema_version != "0.1.0" || input.input_id.trim().is_empty() {
        return Err("INPUT_IDENTITY_INVALID".to_owned());
    }
    if !input.alpha.is_finite() || !(0.0 < input.alpha && input.alpha < 0.5) {
        return Err("SIGNIFICANCE_THRESHOLD_INVALID".to_owned());
    }
    if input.baseline.period_id == input.testing.period_id {
        return Err("TIMEPOINT_IDS_DUPLICATED".to_owned());
    }
    if input.baseline.timestamp_utc >= input.testing.timestamp_utc {
        return Err("TIMEPOINT_ORDER_INVALID".to_owned());
    }
    validate_period(&input.baseline, "BASELINE")?;
    validate_period(&input.testing, "TESTING")?;

    if input.baseline.instrument != input.testing.instrument {
        return Err("INSTRUMENT_IDENTITY_OR_PROFILE_CHANGED_BETWEEN_TIMEPOINTS".to_owned());
    }
    if input.baseline.setup != input.testing.setup {
        return Err("ACQUISITION_OR_RESIDUAL_PRODUCER_SCOPE_CHANGED_BETWEEN_TIMEPOINTS".to_owned());
    }
    if input.baseline.instrument.manufacturer.trim().is_empty()
        || input.baseline.instrument.model.trim().is_empty()
        || input.baseline.instrument.serial_id.trim().is_empty()
        || input.baseline.instrument.firmware_version.trim().is_empty()
        || input.baseline.instrument.profile_id.trim().is_empty()
        || input.baseline.setup.residual_producer_id.trim().is_empty()
        || input
            .baseline
            .setup
            .centralization_method_id
            .trim()
            .is_empty()
    {
        return Err("INSTRUMENT_OR_PRODUCER_IDENTITY_INCOMPLETE".to_owned());
    }
    for digest in [
        input.baseline.instrument.profile_sha256.as_str(),
        input
            .baseline
            .instrument
            .calibration_reference_sha256
            .as_str(),
        input.baseline.setup.acquisition_config_sha256.as_str(),
        input.baseline.setup.residual_producer_sha256.as_str(),
        input.baseline.setup.centralization_method_sha256.as_str(),
        input.baseline.setup.target_lineage_sha256.as_str(),
    ] {
        if !is_sha256(digest) {
            return Err("INSTRUMENT_OR_SETUP_DIGEST_INVALID".to_owned());
        }
    }
    if input.baseline.setup.angle_unit != "radian" {
        return Err("ANGULAR_UNIT_MUST_BE_RADIAN".to_owned());
    }
    if !matches!(
        input.baseline.setup.range_unit.as_str(),
        "metre" | "millimetre"
    ) {
        return Err("RANGE_UNIT_UNSUPPORTED".to_owned());
    }
    let baseline_targets: BTreeSet<&str> = input
        .baseline
        .target_ids
        .iter()
        .map(String::as_str)
        .collect();
    let testing_targets: BTreeSet<&str> = input
        .testing
        .target_ids
        .iter()
        .map(String::as_str)
        .collect();
    if baseline_targets != testing_targets {
        return Err("TARGET_LINEAGE_CHANGED_BETWEEN_TIMEPOINTS".to_owned());
    }

    Ok(())
}

fn canonical_rows(period: &Period) -> Vec<[f64; 3]> {
    let mut rows = period.rows.clone();
    rows.sort_by(|left, right| {
        left.target_id
            .cmp(&right.target_id)
            .then_with(|| left.position_id.cmp(&right.position_id))
            .then_with(|| left.row_id.cmp(&right.row_id))
    });
    rows.iter()
        .map(|row| [row.theta_residual, row.phi_residual, row.range_residual])
        .collect()
}

fn mean_covariance<const D: usize>(rows: &[[f64; D]]) -> Result<([f64; D], [[f64; D]; D]), String> {
    if rows.len() < 2 {
        return Err("INSUFFICIENT_OBSERVATIONS_FOR_COVARIANCE".to_owned());
    }
    let mut mean = [0.0; D];
    for row in rows {
        for column in 0..D {
            mean[column] += row[column];
        }
    }
    for value in &mut mean {
        *value /= rows.len() as f64;
    }

    let mut covariance = [[0.0; D]; D];
    for row in rows {
        for i in 0..D {
            for j in 0..D {
                covariance[i][j] += (row[i] - mean[i]) * (row[j] - mean[j]);
            }
        }
    }
    let denominator = (rows.len() - 1) as f64;
    for row in &mut covariance {
        for value in row {
            *value /= denominator;
        }
    }
    if covariance.iter().flatten().any(|value| !value.is_finite()) {
        return Err("COVARIANCE_NON_FINITE".to_owned());
    }
    Ok((mean, covariance))
}

fn covariance_summary(rows: &[[f64; 3]]) -> Result<CovarianceSummary, String> {
    let (mean, covariance) = mean_covariance(rows)?;
    let mut standard_deviation = [0.0; 3];
    let mut correlation = [[0.0; 3]; 3];

    for i in 0..3 {
        if covariance[i][i] <= 0.0 {
            return Err("RAW_RESIDUAL_COVARIANCE_SINGULAR".to_owned());
        }
        standard_deviation[i] = covariance[i][i].sqrt();
    }
    for i in 0..3 {
        for j in 0..3 {
            correlation[i][j] = covariance[i][j] / (standard_deviation[i] * standard_deviation[j]);
        }
    }

    let one_sigma_ellipses = [
        ellipse(
            "theta_phi",
            mean[0],
            mean[1],
            covariance[0][0],
            covariance[0][1],
            covariance[1][1],
        )?,
        ellipse(
            "theta_range",
            mean[0],
            mean[2],
            covariance[0][0],
            covariance[0][2],
            covariance[2][2],
        )?,
        ellipse(
            "phi_range",
            mean[1],
            mean[2],
            covariance[1][1],
            covariance[1][2],
            covariance[2][2],
        )?,
    ];

    Ok(CovarianceSummary {
        mean_residual: mean,
        covariance,
        standard_deviation,
        correlation,
        one_sigma_ellipses,
    })
}

fn ellipse(
    axes: &str,
    center_x: f64,
    center_y: f64,
    variance_x: f64,
    covariance_xy: f64,
    variance_y: f64,
) -> Result<Ellipse2d, String> {
    let discriminant = ((variance_x - variance_y).powi(2) + 4.0 * covariance_xy.powi(2)).sqrt();
    let trace = variance_x + variance_y;
    let major_variance = 0.5 * (trace + discriminant);
    let minor_variance = 0.5 * (trace - discriminant);
    let tolerance = major_variance.abs().max(1.0) * 1e-12;
    if major_variance <= 0.0 || minor_variance < -tolerance {
        return Err("DATA_ELLIPSE_COVARIANCE_NOT_POSITIVE_SEMIDEFINITE".to_owned());
    }
    let angle = 0.5 * (2.0 * covariance_xy).atan2(variance_x - variance_y);
    Ok(Ellipse2d {
        axes: axes.to_owned(),
        center: [center_x, center_y],
        major_standard_deviation: major_variance.sqrt(),
        minor_standard_deviation: minor_variance.max(0.0).sqrt(),
        major_axis_angle_rad: angle,
    })
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        0.5 * (values[middle - 1] + values[middle])
    } else {
        values[middle]
    }
}

fn obrien_transform(rows: &[[f64; 3]]) -> Result<Vec<[f64; DIMENSIONS]>, String> {
    if rows.len() < 2 {
        return Err("INSUFFICIENT_OBSERVATIONS_FOR_ROBUST_TRANSFORM".to_owned());
    }
    let mut medians = [0.0; 3];
    for column in 0..3 {
        let mut values: Vec<f64> = rows.iter().map(|row| row[column]).collect();
        medians[column] = median(&mut values);
    }

    let pairs = [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)];
    let mut transformed = Vec::with_capacity(rows.len());
    for row in rows {
        let mut output = [0.0; DIMENSIONS];
        for (index, (left, right)) in pairs.iter().enumerate() {
            let product = (row[*left] - medians[*left]) * (row[*right] - medians[*right]);
            output[index] = product.signum() * product.abs().sqrt();
        }
        transformed.push(output);
    }
    Ok(transformed)
}

fn inverse_6(
    matrix: [[f64; DIMENSIONS]; DIMENSIONS],
) -> Result<[[f64; DIMENSIONS]; DIMENSIONS], String> {
    let mut augmented = [[0.0; DIMENSIONS * 2]; DIMENSIONS];
    let mut scale = 0.0_f64;
    for i in 0..DIMENSIONS {
        for j in 0..DIMENSIONS {
            augmented[i][j] = matrix[i][j];
            scale = scale.max(matrix[i][j].abs());
        }
        augmented[i][DIMENSIONS + i] = 1.0;
    }
    if scale == 0.0 || !scale.is_finite() {
        return Err("POOLED_TRANSFORMED_COVARIANCE_SINGULAR".to_owned());
    }

    for column in 0..DIMENSIONS {
        let mut pivot_row = column;
        for row in (column + 1)..DIMENSIONS {
            if augmented[row][column].abs() > augmented[pivot_row][column].abs() {
                pivot_row = row;
            }
        }
        if augmented[pivot_row][column].abs() <= scale * 1e-12 {
            return Err("POOLED_TRANSFORMED_COVARIANCE_SINGULAR".to_owned());
        }
        if pivot_row != column {
            augmented.swap(pivot_row, column);
        }
        let pivot = augmented[column][column];
        for j in 0..(DIMENSIONS * 2) {
            augmented[column][j] /= pivot;
        }
        for row in 0..DIMENSIONS {
            if row == column {
                continue;
            }
            let factor = augmented[row][column];
            for j in 0..(DIMENSIONS * 2) {
                augmented[row][j] -= factor * augmented[column][j];
            }
        }
    }

    let mut inverse = [[0.0; DIMENSIONS]; DIMENSIONS];
    for i in 0..DIMENSIONS {
        for j in 0..DIMENSIONS {
            inverse[i][j] = augmented[i][DIMENSIONS + j];
        }
    }
    Ok(inverse)
}

fn ensure_positive_definite(matrix: &[[f64; DIMENSIONS]; DIMENSIONS]) -> Result<(), String> {
    let max_diagonal = (0..DIMENSIONS)
        .map(|i| matrix[i][i].abs())
        .fold(0.0_f64, f64::max);
    if max_diagonal == 0.0 {
        return Err("POOLED_TRANSFORMED_COVARIANCE_SINGULAR".to_owned());
    }
    let tolerance = max_diagonal * 1e-12;
    let mut lower = [[0.0; DIMENSIONS]; DIMENSIONS];
    for i in 0..DIMENSIONS {
        for j in 0..=i {
            let mut value = matrix[i][j];
            for k in 0..j {
                value -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if value <= tolerance {
                    return Err("POOLED_TRANSFORMED_COVARIANCE_SINGULAR".to_owned());
                }
                lower[i][j] = value.sqrt();
            } else {
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    Ok(())
}

fn ln_gamma(value: f64) -> f64 {
    const COEFFICIENTS: [f64; 9] = [
        0.99999999999980993,
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];
    if value < 0.5 {
        return std::f64::consts::PI.ln()
            - (std::f64::consts::PI * value).sin().ln()
            - ln_gamma(1.0 - value);
    }
    let shifted = value - 1.0;
    let mut x = COEFFICIENTS[0];
    for (index, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
        x += coefficient / (shifted + index as f64);
    }
    let t = shifted + 7.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (shifted + 0.5) * t.ln() - t + x.ln()
}

fn beta_continued_fraction(a: f64, b: f64, x: f64) -> Result<f64, String> {
    const MAX_ITERATIONS: usize = 300;
    const EPSILON: f64 = 3e-14;
    const FLOOR: f64 = 1e-300;

    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FLOOR {
        d = FLOOR;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=MAX_ITERATIONS {
        let m_f = m as f64;
        let m2 = 2.0 * m_f;
        let mut aa = m_f * (b - m_f) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FLOOR {
            d = FLOOR;
        }
        c = 1.0 + aa / c;
        if c.abs() < FLOOR {
            c = FLOOR;
        }
        d = 1.0 / d;
        h *= d * c;

        aa = -(a + m_f) * (qab + m_f) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FLOOR {
            d = FLOOR;
        }
        c = 1.0 + aa / c;
        if c.abs() < FLOOR {
            c = FLOOR;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < EPSILON {
            return Ok(h);
        }
    }
    Err("INCOMPLETE_BETA_DID_NOT_CONVERGE".to_owned())
}

fn regularized_beta(x: f64, a: f64, b: f64) -> Result<f64, String> {
    if !x.is_finite() || !a.is_finite() || !b.is_finite() || a <= 0.0 || b <= 0.0 {
        return Err("F_DISTRIBUTION_ARGUMENT_INVALID".to_owned());
    }
    if x <= 0.0 {
        return Ok(0.0);
    }
    if x >= 1.0 {
        return Ok(1.0);
    }
    let front =
        (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp();

    if x < (a + 1.0) / (a + b + 2.0) {
        Ok(front * beta_continued_fraction(a, b, x)? / a)
    } else {
        Ok(1.0 - front * beta_continued_fraction(b, a, 1.0 - x)? / b)
    }
}

fn f_survival_probability(f_statistic: f64, df1: usize, df2: usize) -> Result<f64, String> {
    if !f_statistic.is_finite() || f_statistic < 0.0 || df1 == 0 || df2 == 0 {
        return Err("F_DISTRIBUTION_ARGUMENT_INVALID".to_owned());
    }
    let df1_f = df1 as f64;
    let df2_f = df2 as f64;
    let x = df2_f / (df2_f + df1_f * f_statistic);
    let p_value = regularized_beta(x, 0.5 * df2_f, 0.5 * df1_f)?;
    if !p_value.is_finite() || !(0.0..=1.0).contains(&p_value) {
        return Err("F_DISTRIBUTION_RESULT_INVALID".to_owned());
    }
    Ok(p_value)
}

fn robust_omnibus(baseline: &[[f64; 3]], testing: &[[f64; 3]]) -> Result<RobustOmnibus, String> {
    let transformed_baseline = obrien_transform(baseline)?;
    let transformed_testing = obrien_transform(testing)?;
    let n1 = transformed_baseline.len();
    let n2 = transformed_testing.len();
    let total = n1 + n2;
    let df1 = DIMENSIONS;
    let df2 = total
        .checked_sub(DIMENSIONS + 1)
        .ok_or_else(|| "INSUFFICIENT_OBSERVATIONS_FOR_HOTELLING_TEST".to_owned())?;
    if df2 == 0 || n1 <= DIMENSIONS || n2 <= DIMENSIONS {
        return Err("INSUFFICIENT_OBSERVATIONS_FOR_HOTELLING_TEST".to_owned());
    }

    let (_, covariance_1) = mean_covariance(&transformed_baseline)?;
    let (_, covariance_2) = mean_covariance(&transformed_testing)?;
    let mut scales = [0.0; DIMENSIONS];
    for column in 0..DIMENSIONS {
        let pooled_variance = ((n1 - 1) as f64 * covariance_1[column][column]
            + (n2 - 1) as f64 * covariance_2[column][column])
            / (total - 2) as f64;
        if !pooled_variance.is_finite() || pooled_variance <= 0.0 {
            return Err("POOLED_TRANSFORMED_COVARIANCE_SINGULAR".to_owned());
        }
        scales[column] = pooled_variance.sqrt();
    }

    // Diagonal scaling is an invertible change of coordinates, so Hotelling's
    // T-squared is unchanged. It avoids unit-driven numerical conditioning
    // problems between angular and range covariance components.
    let standardized_baseline: Vec<[f64; DIMENSIONS]> = transformed_baseline
        .iter()
        .map(|row| {
            let mut output = [0.0; DIMENSIONS];
            for column in 0..DIMENSIONS {
                output[column] = row[column] / scales[column];
            }
            output
        })
        .collect();
    let standardized_testing: Vec<[f64; DIMENSIONS]> = transformed_testing
        .iter()
        .map(|row| {
            let mut output = [0.0; DIMENSIONS];
            for column in 0..DIMENSIONS {
                output[column] = row[column] / scales[column];
            }
            output
        })
        .collect();

    let (mean_1, covariance_1) = mean_covariance(&standardized_baseline)?;
    let (mean_2, covariance_2) = mean_covariance(&standardized_testing)?;
    let mut pooled = [[0.0; DIMENSIONS]; DIMENSIONS];
    for i in 0..DIMENSIONS {
        for j in 0..DIMENSIONS {
            pooled[i][j] = ((n1 - 1) as f64 * covariance_1[i][j]
                + (n2 - 1) as f64 * covariance_2[i][j])
                / (total - 2) as f64;
        }
    }
    ensure_positive_definite(&pooled)?;
    let inverse = inverse_6(pooled)?;

    let mut difference = [0.0; DIMENSIONS];
    for i in 0..DIMENSIONS {
        difference[i] = mean_1[i] - mean_2[i];
    }
    let mut quadratic = 0.0;
    for i in 0..DIMENSIONS {
        for j in 0..DIMENSIONS {
            quadratic += difference[i] * inverse[i][j] * difference[j];
        }
    }
    let t_squared = (n1 as f64 * n2 as f64 / total as f64) * quadratic;
    if !t_squared.is_finite() || t_squared < 0.0 {
        return Err("HOTELLING_STATISTIC_INVALID".to_owned());
    }
    let f_statistic = (df2 as f64 / ((total - 2) as f64 * df1 as f64)) * t_squared;
    let p_value = f_survival_probability(f_statistic, df1, df2)?;

    Ok(RobustOmnibus {
        statistic: "OBRIEN_ROBUST_OMNIBUS_TWO_GROUP_HOTELLING_T2".to_owned(),
        transformed_dimension_count: DIMENSIONS,
        baseline_observation_count: n1,
        testing_observation_count: n2,
        hotelling_t_squared: t_squared,
        f_statistic,
        numerator_degrees_of_freedom: df1,
        denominator_degrees_of_freedom: df2,
        p_value,
        pooled_covariance_condition: "POSITIVE_DEFINITE_AFTER_INVERTIBLE_DIAGONAL_SCALING".to_owned(),
        significance_interpretation: "Reject equal spherical-residual covariance when p_value < alpha; failure to reject is not proof of unchanged precision.".to_owned(),
    })
}

fn indeterminate_report(input: &Input, input_sha256: String, reason: String) -> Report {
    Report {
        schema_version: "0.1.0".to_owned(),
        input_id: input.input_id.clone(),
        method_id: METHOD_ID.to_owned(),
        method_reference: "Gregg, Muralikrishnan & Shilling (2026), DOI 10.1111/1556-4029.70256; O'Brien (1992) Robust Omnibus Test.".to_owned(),
        input_sha256,
        disposition: "INDETERMINATE".to_owned(),
        reason_codes: vec![reason],
        alpha: input.alpha,
        baseline_period_id: input.baseline.period_id.clone(),
        testing_period_id: input.testing.period_id.clone(),
        baseline_residual_artifact_sha256: input.baseline.residual_artifact_sha256.clone(),
        testing_residual_artifact_sha256: input.testing.residual_artifact_sha256.clone(),
        baseline_row_count: input.baseline.rows.len(),
        testing_row_count: input.testing.rows.len(),
        baseline_covariance: None,
        testing_covariance: None,
        robust_omnibus: None,
        accuracy_disposition: "NOT_EVALUATED".to_owned(),
        task_fitness_disposition: "NOT_EVALUATED".to_owned(),
        assumption_notes: ASSUMPTIONS.iter().map(|note| (*note).to_owned()).collect(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}

fn evaluate_input(input: &Input, input_sha256: String) -> Report {
    if let Err(reason) = validate_input(input) {
        return indeterminate_report(input, input_sha256, reason);
    }
    let baseline_rows = canonical_rows(&input.baseline);
    let testing_rows = canonical_rows(&input.testing);
    let baseline_covariance = match covariance_summary(&baseline_rows) {
        Ok(summary) => summary,
        Err(reason) => return indeterminate_report(input, input_sha256, reason),
    };
    let testing_covariance = match covariance_summary(&testing_rows) {
        Ok(summary) => summary,
        Err(reason) => return indeterminate_report(input, input_sha256, reason),
    };
    let omnibus = match robust_omnibus(&baseline_rows, &testing_rows) {
        Ok(summary) => summary,
        Err(reason) => return indeterminate_report(input, input_sha256, reason),
    };

    let disposition = if omnibus.p_value < input.alpha {
        "SIGNIFICANT_PRECISION_CHANGE_DETECTED"
    } else {
        "NO_SIGNIFICANT_PRECISION_CHANGE_DETECTED"
    };

    Report {
        schema_version: "0.1.0".to_owned(),
        input_id: input.input_id.clone(),
        method_id: METHOD_ID.to_owned(),
        method_reference: "Gregg, Muralikrishnan & Shilling (2026), DOI 10.1111/1556-4029.70256; O'Brien (1992) Robust Omnibus Test.".to_owned(),
        input_sha256,
        disposition: disposition.to_owned(),
        reason_codes: Vec::new(),
        alpha: input.alpha,
        baseline_period_id: input.baseline.period_id.clone(),
        testing_period_id: input.testing.period_id.clone(),
        baseline_residual_artifact_sha256: input.baseline.residual_artifact_sha256.clone(),
        testing_residual_artifact_sha256: input.testing.residual_artifact_sha256.clone(),
        baseline_row_count: baseline_rows.len(),
        testing_row_count: testing_rows.len(),
        baseline_covariance: Some(baseline_covariance),
        testing_covariance: Some(testing_covariance),
        robust_omnibus: Some(omnibus),
        accuracy_disposition: "NOT_EVALUATED".to_owned(),
        task_fitness_disposition: "NOT_EVALUATED".to_owned(),
        assumption_notes: ASSUMPTIONS.iter().map(|note| (*note).to_owned()).collect(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
    }
}

fn evaluate_bytes(bytes: &[u8]) -> Result<Report, Box<dyn Error>> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!("input exceeds maximum size of {MAX_INPUT_BYTES} bytes").into());
    }
    let input_sha256 = sha256_hex(bytes);
    let input: Input = serde_json::from_slice(bytes)?;
    Ok(evaluate_input(&input, input_sha256))
}

fn parse_input_path() -> Result<PathBuf, Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut input = PathBuf::from("tools/scan-bim-tls-ipa-statistic/fixtures/no-change.json");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => input = args.next().ok_or("--input requires a path")?.into(),
            "--help" | "-h" => {
                println!("scan-bim-tls-ipa-statistic [--input PATH]");
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}").into()),
        }
    }
    Ok(input)
}

fn run() -> Result<bool, Box<dyn Error>> {
    let input_path = parse_input_path()?;
    let bytes = fs::read(input_path)?;
    let report = evaluate_bytes(&bytes)?;
    let passed = report.disposition != "INDETERMINATE";
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(passed)
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(2),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NO_CHANGE: &str = include_str!("../fixtures/no-change.json");
    const COVARIANCE_CHANGE: &str = include_str!("../fixtures/covariance-change.json");

    fn parse_fixture(raw: &str) -> Input {
        serde_json::from_str(raw).expect("fixture parses")
    }

    fn evaluate_fixture(raw: &str) -> Report {
        let input = parse_fixture(raw);
        evaluate_input(&input, sha256_hex(raw.as_bytes()))
    }

    #[test]
    fn no_change_fixture_does_not_reject_equal_covariance() {
        let report = evaluate_fixture(NO_CHANGE);
        assert_eq!(
            report.disposition,
            "NO_SIGNIFICANT_PRECISION_CHANGE_DETECTED"
        );
        let statistic = report.robust_omnibus.expect("statistic exists");
        assert!(statistic.p_value >= report.alpha);
        assert_eq!(report.accuracy_disposition, "NOT_EVALUATED");
        assert_eq!(report.task_fitness_disposition, "NOT_EVALUATED");
    }

    #[test]
    fn covariance_change_fixture_rejects_equal_covariance() {
        let report = evaluate_fixture(COVARIANCE_CHANGE);
        assert_eq!(report.disposition, "SIGNIFICANT_PRECISION_CHANGE_DETECTED");
        let statistic = report.robust_omnibus.expect("statistic exists");
        assert!(statistic.p_value < report.alpha);
        assert!(statistic.hotelling_t_squared > 0.0);
        assert!(statistic.f_statistic > 0.0);
    }

    #[test]
    fn report_contains_all_six_covariance_components() {
        let report = evaluate_fixture(COVARIANCE_CHANGE);
        let covariance = report.testing_covariance.expect("covariance exists");
        assert!(covariance.covariance[0][0] > 0.0);
        assert!(covariance.covariance[1][1] > 0.0);
        assert!(covariance.covariance[2][2] > 0.0);
        assert!(covariance.covariance[0][1].is_finite());
        assert!(covariance.covariance[0][2].is_finite());
        assert!(covariance.covariance[1][2].is_finite());
        assert!(
            (covariance.correlation[0][1].abs() <= 1.0)
                .then_some(())
                .is_some()
        );
    }

    #[test]
    fn changed_instrument_profile_is_indeterminate() {
        let mut input = parse_fixture(NO_CHANGE);
        input.testing.instrument.profile_sha256 = "f".repeat(64);
        let report = evaluate_input(&input, "a".repeat(64));
        assert_eq!(report.disposition, "INDETERMINATE");
        assert_eq!(
            report.reason_codes,
            vec!["INSTRUMENT_IDENTITY_OR_PROFILE_CHANGED_BETWEEN_TIMEPOINTS"]
        );
    }

    #[test]
    fn changed_target_lineage_is_indeterminate() {
        let mut input = parse_fixture(NO_CHANGE);
        input.testing.target_ids[0] = "different-target".to_owned();
        let report = evaluate_input(&input, "b".repeat(64));
        assert_eq!(report.disposition, "INDETERMINATE");
        assert_eq!(
            report.reason_codes,
            vec!["TARGET_LINEAGE_CHANGED_BETWEEN_TIMEPOINTS"]
        );
    }

    #[test]
    fn duplicate_target_position_pair_is_indeterminate() {
        let mut input = parse_fixture(NO_CHANGE);
        input.baseline.rows[1].target_id = input.baseline.rows[0].target_id.clone();
        input.baseline.rows[1].position_id = input.baseline.rows[0].position_id.clone();
        let report = evaluate_input(&input, "c".repeat(64));
        assert_eq!(report.disposition, "INDETERMINATE");
        assert_eq!(
            report.reason_codes,
            vec!["BASELINE_TARGET_POSITION_PAIR_DUPLICATED"]
        );
    }

    #[test]
    fn incomplete_target_position_grid_is_indeterminate() {
        let mut input = parse_fixture(NO_CHANGE);
        input.testing.rows.pop();
        let report = evaluate_input(&input, "d".repeat(64));
        assert_eq!(report.disposition, "INDETERMINATE");
        assert_eq!(
            report.reason_codes,
            vec!["TESTING_RESIDUAL_ROW_COUNT_DOES_NOT_MATCH_TARGET_POSITION_GRID"]
        );
    }

    #[test]
    fn non_finite_residual_is_indeterminate() {
        let mut input = parse_fixture(NO_CHANGE);
        input.baseline.rows[0].theta_residual = f64::NAN;
        let report = evaluate_input(&input, "e".repeat(64));
        assert_eq!(report.disposition, "INDETERMINATE");
        assert_eq!(
            report.reason_codes,
            vec!["BASELINE_NON_FINITE_SPHERICAL_RESIDUAL"]
        );
    }

    #[test]
    fn singular_transformed_covariance_is_indeterminate_without_repair() {
        let mut input = parse_fixture(NO_CHANGE);
        for period in [&mut input.baseline, &mut input.testing] {
            for (index, row) in period.rows.iter_mut().enumerate() {
                let value = (index as f64 - 39.5) * 1e-6;
                row.theta_residual = value;
                row.phi_residual = 2.0 * value;
                row.range_residual = 3.0 * value;
            }
        }
        let report = evaluate_input(&input, "f".repeat(64));
        assert_eq!(report.disposition, "INDETERMINATE");
        assert_eq!(
            report.reason_codes,
            vec!["POOLED_TRANSFORMED_COVARIANCE_SINGULAR"]
        );
    }

    #[test]
    fn row_order_is_semantically_invariant_but_input_digest_is_byte_sensitive() {
        let original = evaluate_fixture(NO_CHANGE);
        let mut input = parse_fixture(NO_CHANGE);
        input.baseline.rows.reverse();
        input.testing.rows.reverse();
        let reversed_bytes = serde_json::to_vec(&input).expect("fixture serializes");
        let reversed = evaluate_bytes(&reversed_bytes).expect("reversed fixture evaluates");
        assert_eq!(original.disposition, reversed.disposition);
        assert_eq!(
            original.robust_omnibus.as_ref().unwrap().p_value,
            reversed.robust_omnibus.as_ref().unwrap().p_value
        );
        assert_ne!(original.input_sha256, reversed.input_sha256);
    }

    #[test]
    fn alpha_outside_declared_range_is_indeterminate() {
        let mut input = parse_fixture(NO_CHANGE);
        input.alpha = 1.0;
        let report = evaluate_input(&input, "1".repeat(64));
        assert_eq!(report.disposition, "INDETERMINATE");
        assert_eq!(report.reason_codes, vec!["SIGNIFICANCE_THRESHOLD_INVALID"]);
    }

    #[test]
    fn target_and_position_ids_must_be_unique() {
        let mut input = parse_fixture(NO_CHANGE);
        input.baseline.position_ids[1] = input.baseline.position_ids[0].clone();
        let report = evaluate_input(&input, "2".repeat(64));
        assert_eq!(report.disposition, "INDETERMINATE");
        assert_eq!(
            report.reason_codes,
            vec!["BASELINE_POSITION_MANIFEST_INSUFFICIENT_OR_INVALID"]
        );
    }

    #[test]
    fn timestamp_calendar_and_clock_fields_are_validated() {
        for invalid in [
            "2026-13-01T09:00:00Z",
            "2026-02-30T09:00:00Z",
            "2026-11-12T24:00:00Z",
            "2026-11-12T09:60:00Z",
            "2026-11-12T09:00:60Z",
        ] {
            assert!(!valid_timestamp_utc(invalid), "{invalid}");
        }
        assert!(valid_timestamp_utc("2024-02-29T23:59:59Z"));
        assert!(!valid_timestamp_utc("2025-02-29T23:59:59Z"));
    }

    #[test]
    fn unsupported_units_are_indeterminate() {
        let mut input = parse_fixture(NO_CHANGE);
        input.baseline.setup.range_unit = "foot".to_owned();
        input.testing.setup.range_unit = "foot".to_owned();
        let report = evaluate_input(&input, "3".repeat(64));
        assert_eq!(report.disposition, "INDETERMINATE");
        assert_eq!(report.reason_codes, vec!["RANGE_UNIT_UNSUPPORTED"]);
    }

    #[test]
    fn f_survival_probability_matches_reference_value() {
        let p = f_survival_probability(0.6642492282207266, 6, 153).unwrap();
        assert!((p - 0.6786292876444087).abs() < 1e-10);
    }

    #[test]
    fn signed_square_root_preserves_product_sign() {
        let rows = [
            [-2.0, 1.0, 3.0],
            [-1.0, 2.0, 4.0],
            [0.0, 3.0, 5.0],
            [1.0, 4.0, 6.0],
        ];
        let transformed = obrien_transform(&rows).unwrap();
        assert_eq!(transformed[0][3].signum(), -1.0);
        assert!(transformed[0][3] < 0.0);
        assert!(transformed[0][0] > 0.0);
    }

    #[test]
    fn accuracy_and_task_fitness_are_never_promoted_by_part_two() {
        for raw in [NO_CHANGE, COVARIANCE_CHANGE] {
            let report = evaluate_fixture(raw);
            assert_eq!(report.accuracy_disposition, "NOT_EVALUATED");
            assert_eq!(report.task_fitness_disposition, "NOT_EVALUATED");
        }
    }
}
