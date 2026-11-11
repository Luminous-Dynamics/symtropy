use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;

const MAX_INPUT_BYTES: usize = 8 * 1024 * 1024;
const MAX_RESIDUAL_ROWS: usize = 10_000;
const DIMENSIONS: usize = 3;
const OBRIEN_DIMENSIONS: usize = 6;
const OBRIEN_PAIRS: [(usize, usize); OBRIEN_DIMENSIONS] =
    [(0, 0), (0, 1), (0, 2), (1, 1), (1, 2), (2, 2)];

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MonitorInput {
    schema_version: String,
    claim_ceiling: String,
    instrument: InstrumentIdentity,
    method: MethodDeclaration,
    units: UnitDeclaration,
    baseline: Timepoint,
    testing: Timepoint,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InstrumentIdentity {
    instrument_id: String,
    manufacturer: String,
    model: String,
    serial_number: String,
    firmware_sha256: String,
    sensor_profile_sha256: String,
    configuration_sha256: String,
    calibration_id: String,
    calibration_certificate_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MethodDeclaration {
    method_id: String,
    method_revision: String,
    alpha: f64,
    minimum_target_count: usize,
    minimum_position_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitDeclaration {
    azimuth: String,
    elevation: String,
    range: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Timepoint {
    timepoint_id: String,
    captured_utc: String,
    source_scan_manifest_sha256: String,
    target_array_sha256: String,
    residual_generation_sha256: String,
    instrument_identity_sha256: String,
    target_ids: Vec<String>,
    position_ids: Vec<String>,
    residuals: Vec<SphericalResidual>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SphericalResidual {
    target_id: String,
    position_id: String,
    azimuth_arcsec: f64,
    elevation_arcsec: f64,
    range_mm: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Disposition {
    SignificantPrecisionChangeDetected,
    NoSignificantPrecisionChangeDetected,
    Indeterminate,
}

#[derive(Debug, Serialize)]
struct CovarianceSummary {
    matrix: [[f64; 3]; 3],
    standard_deviation: [f64; 3],
    correlations: [f64; 3],
}

#[derive(Debug, Serialize)]
struct MonitorReport {
    schema_version: String,
    method_id: String,
    method_revision: String,
    input_sha256: String,
    disposition: Disposition,
    reason_codes: Vec<String>,
    alpha: f64,
    baseline_residual_count: usize,
    testing_residual_count: usize,
    statistic: Option<f64>,
    p_value: Option<f64>,
    numerator_degrees_of_freedom: Option<usize>,
    denominator_degrees_of_freedom: Option<usize>,
    baseline_covariance: Option<CovarianceSummary>,
    testing_covariance: Option<CovarianceSummary>,
    standard_deviation_ratio_testing_over_baseline: Option<[f64; 3]>,
    accuracy_disposition: &'static str,
    task_fitness_disposition: &'static str,
    advisory: &'static str,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| {
                matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
            })
}

fn unique_nonempty(values: &[String]) -> bool {
    !values.is_empty()
        && values.iter().all(|value| is_nonempty(value))
        && values.iter().collect::<BTreeSet<_>>().len() == values.len()
}

fn indeterminate(
    input: &MonitorInput,
    input_sha256: &str,
    baseline_count: usize,
    testing_count: usize,
    reason: &str,
) -> MonitorReport {
    MonitorReport {
        schema_version: "0.1".to_owned(),
        method_id: input.method.method_id.clone(),
        method_revision: input.method.method_revision.clone(),
        input_sha256: input_sha256.to_owned(),
        disposition: Disposition::Indeterminate,
        reason_codes: vec![reason.to_owned()],
        alpha: input.method.alpha,
        baseline_residual_count: baseline_count,
        testing_residual_count: testing_count,
        statistic: None,
        p_value: None,
        numerator_degrees_of_freedom: None,
        denominator_degrees_of_freedom: None,
        baseline_covariance: None,
        testing_covariance: None,
        standard_deviation_ratio_testing_over_baseline: None,
        accuracy_disposition: "NOT_EVALUATED",
        task_fitness_disposition: "NOT_EVALUATED",
        advisory: "INDETERMINATE: inspect the reason code; this report is not an accuracy or fitness certificate.",
    }
}

fn validate_input(input: &MonitorInput) -> Result<(), &'static str> {
    if input.schema_version != "0.1" || !is_nonempty(&input.claim_ceiling) {
        return Err("INPUT_SCHEMA_VERSION_OR_CLAIM_CEILING_INVALID");
    }
    if input.method.method_id != "OBRIEN_ROBUST_OMNIBUS_IPA_PART_II"
        || !is_nonempty(&input.method.method_revision)
    {
        return Err("METHOD_ID_OR_REVISION_UNSUPPORTED");
    }
    if !input.method.alpha.is_finite()
        || !(0.001..=0.10).contains(&input.method.alpha)
    {
        return Err("SIGNIFICANCE_LEVEL_OUT_OF_RANGE");
    }
    if input.method.minimum_target_count < 20 || input.method.minimum_position_count < 4 {
        return Err("DECLARED_MINIMUM_BELOW_IPA_BENCHMARK_PROFILE");
    }
    if input.units.azimuth != "arcsecond"
        || input.units.elevation != "arcsecond"
        || input.units.range != "millimetre"
    {
        return Err("SPHERICAL_RESIDUAL_UNITS_UNSUPPORTED_OR_MISMATCHED");
    }

    let instrument = &input.instrument;
    if [
        instrument.instrument_id.as_str(),
        instrument.manufacturer.as_str(),
        instrument.model.as_str(),
        instrument.serial_number.as_str(),
        instrument.calibration_id.as_str(),
    ]
    .iter()
    .any(|value| !is_nonempty(value))
    {
        return Err("INSTRUMENT_IDENTITY_INCOMPLETE");
    }
    for digest in [
        instrument.firmware_sha256.as_str(),
        instrument.sensor_profile_sha256.as_str(),
        instrument.configuration_sha256.as_str(),
        instrument.calibration_certificate_sha256.as_str(),
    ] {
        if !valid_sha256(digest) {
            return Err("INSTRUMENT_EVIDENCE_DIGEST_INVALID");
        }
    }
    let instrument_bytes =
        serde_json::to_vec(instrument).map_err(|_| "INSTRUMENT_IDENTITY_SERIALIZATION_FAILED")?;
    let expected_instrument_binding = sha256_hex(&instrument_bytes);

    if input.baseline.timepoint_id == input.testing.timepoint_id
        || !is_nonempty(&input.baseline.timepoint_id)
        || !is_nonempty(&input.testing.timepoint_id)
    {
        return Err("TIMEPOINT_IDENTITIES_INVALID_OR_DUPLICATED");
    }
    if !valid_utc_timestamp(&input.baseline.captured_utc)
        || !valid_utc_timestamp(&input.testing.captured_utc)
        || input.baseline.captured_utc >= input.testing.captured_utc
    {
        return Err("TIMEPOINT_ORDER_OR_TIMESTAMP_INVALID");
    }

    for timepoint in [&input.baseline, &input.testing] {
        if timepoint.instrument_identity_sha256 != expected_instrument_binding {
            return Err("TIMEPOINT_INSTRUMENT_IDENTITY_MISMATCH");
        }
        for digest in [
            timepoint.source_scan_manifest_sha256.as_str(),
            timepoint.target_array_sha256.as_str(),
            timepoint.residual_generation_sha256.as_str(),
        ] {
            if !valid_sha256(digest) {
                return Err("TIMEPOINT_PROVENANCE_DIGEST_INVALID");
            }
        }
        if !unique_nonempty(&timepoint.target_ids) || !unique_nonempty(&timepoint.position_ids) {
            return Err("TARGET_OR_POSITION_IDENTITIES_INVALID");
        }
        if timepoint.target_ids.len() < input.method.minimum_target_count
            || timepoint.position_ids.len() < input.method.minimum_position_count
        {
            return Err("TARGET_OR_POSITION_COUNT_BELOW_DECLARED_MINIMUM");
        }
        let expected_rows = timepoint
            .target_ids
            .len()
            .checked_mul(timepoint.position_ids.len())
            .ok_or("RESIDUAL_CARDINALITY_OVERFLOW")?;
        if expected_rows > MAX_RESIDUAL_ROWS || timepoint.residuals.len() != expected_rows {
            return Err("RESIDUAL_COUNT_DOES_NOT_MATCH_TARGET_POSITION_GRID");
        }
        let targets: BTreeSet<&str> = timepoint.target_ids.iter().map(String::as_str).collect();
        let positions: BTreeSet<&str> =
            timepoint.position_ids.iter().map(String::as_str).collect();
        let mut pairs = BTreeSet::new();
        for residual in &timepoint.residuals {
            if !targets.contains(residual.target_id.as_str())
                || !positions.contains(residual.position_id.as_str())
                || !pairs.insert((residual.target_id.as_str(), residual.position_id.as_str()))
            {
                return Err("RESIDUAL_TARGET_POSITION_GRID_INVALID");
            }
            if !residual.azimuth_arcsec.is_finite()
                || !residual.elevation_arcsec.is_finite()
                || !residual.range_mm.is_finite()
            {
                return Err("NONFINITE_SPHERICAL_RESIDUAL");
            }
        }
        if pairs.len() != expected_rows {
            return Err("RESIDUAL_TARGET_POSITION_GRID_INCOMPLETE");
        }
    }

    // Each timepoint binds to the same canonical instrument identity.
    // Firmware/profile/configuration/calibration changes require a new baseline.
    Ok(())
}

fn residual_matrix(timepoint: &Timepoint) -> Vec<[f64; 3]> {
    timepoint
        .residuals
        .iter()
        .map(|row| [row.azimuth_arcsec, row.elevation_arcsec, row.range_mm])
        .collect()
}

fn median(values: &[f64]) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let middle = ordered.len() / 2;
    if ordered.len() % 2 == 0 {
        (ordered[middle - 1] + ordered[middle]) / 2.0
    } else {
        ordered[middle]
    }
}

fn covariance<const N: usize>(rows: &[[f64; N]]) -> Option<[[f64; N]; N]> {
    if rows.len() < 2 {
        return None;
    }
    let mut means = [0.0; N];
    for row in rows {
        for column in 0..N {
            means[column] += row[column];
        }
    }
    for mean in &mut means {
        *mean /= rows.len() as f64;
    }
    let mut result = [[0.0; N]; N];
    for row in rows {
        for i in 0..N {
            for j in 0..N {
                result[i][j] += (row[i] - means[i]) * (row[j] - means[j]);
            }
        }
    }
    for row in &mut result {
        for value in row {
            *value /= (rows.len() - 1) as f64;
        }
    }
    Some(result)
}

fn covariance_summary(matrix: [[f64; 3]; 3]) -> CovarianceSummary {
    let standard_deviation = [
        matrix[0][0].sqrt(),
        matrix[1][1].sqrt(),
        matrix[2][2].sqrt(),
    ];
    let correlations = [
        matrix[0][1] / (standard_deviation[0] * standard_deviation[1]),
        matrix[0][2] / (standard_deviation[0] * standard_deviation[2]),
        matrix[1][2] / (standard_deviation[1] * standard_deviation[2]),
    ];
    CovarianceSummary {
        matrix,
        standard_deviation,
        correlations,
    }
}

fn robust_ombrien_rows(rows: &[[f64; 3]]) -> Vec<[f64; OBRIEN_DIMENSIONS]> {
    let columns: [Vec<f64>; 3] = std::array::from_fn(|column| {
        rows.iter().map(|row| row[column]).collect()
    });
    let medians = [
        median(&columns[0]),
        median(&columns[1]),
        median(&columns[2]),
    ];
    rows.iter()
        .map(|row| {
            std::array::from_fn(|index| {
                let (left, right) = OBRIEN_PAIRS[index];
                let product =
                    (row[left] - medians[left]) * (row[right] - medians[right]);
                product.signum() * product.abs().sqrt()
            })
        })
        .collect()
}

fn cholesky<const N: usize>(matrix: &[[f64; N]; N]) -> Option<[[f64; N]; N]> {
    let mut lower = [[0.0; N]; N];
    for i in 0..N {
        for j in 0..=i {
            let mut value = matrix[i][j];
            for k in 0..j {
                value -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !value.is_finite() || value <= 1e-18 {
                    return None;
                }
                lower[i][j] = value.sqrt();
            } else {
                if lower[j][j] == 0.0 {
                    return None;
                }
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    Some(lower)
}

fn solve_cholesky<const N: usize>(
    lower: &[[f64; N]; N],
    rhs: &[f64; N],
) -> Option<[f64; N]> {
    let mut forward = [0.0; N];
    for i in 0..N {
        let mut value = rhs[i];
        for j in 0..i {
            value -= lower[i][j] * forward[j];
        }
        forward[i] = value / lower[i][i];
    }
    let mut solution = [0.0; N];
    for i in (0..N).rev() {
        let mut value = forward[i];
        for j in (i + 1)..N {
            value -= lower[j][i] * solution[j];
        }
        solution[i] = value / lower[i][i];
    }
    solution.iter().all(|value| value.is_finite()).then_some(solution)
}

fn mean<const N: usize>(rows: &[[f64; N]]) -> [f64; N] {
    let mut result = [0.0; N];
    for row in rows {
        for index in 0..N {
            result[index] += row[index];
        }
    }
    for value in &mut result {
        *value /= rows.len() as f64;
    }
    result
}

fn hotelling_two_group(
    baseline: &[[f64; OBRIEN_DIMENSIONS]],
    testing: &[[f64; OBRIEN_DIMENSIONS]],
) -> Option<(f64, f64, usize, usize)> {
    let n1 = baseline.len();
    let n2 = testing.len();
    let total = n1 + n2;
    let dimensions = OBRIEN_DIMENSIONS;
    if n1 <= dimensions || n2 <= dimensions || total <= dimensions + 1 {
        return None;
    }
    let mean1 = mean(baseline);
    let mean2 = mean(testing);
    let covariance1 = covariance(baseline)?;
    let covariance2 = covariance(testing)?;
    let mut pooled = [[0.0; OBRIEN_DIMENSIONS]; OBRIEN_DIMENSIONS];
    let denominator = (total - 2) as f64;
    for i in 0..dimensions {
        for j in 0..dimensions {
            pooled[i][j] = ((n1 - 1) as f64 * covariance1[i][j]
                + (n2 - 1) as f64 * covariance2[i][j])
                / denominator;
        }
    }
    let lower = cholesky(&pooled)?;
    let difference = std::array::from_fn(|i| mean1[i] - mean2[i]);
    let solved = solve_cholesky(&lower, &difference)?;
    let quadratic = difference
        .iter()
        .zip(solved.iter())
        .map(|(left, right)| left * right)
        .sum::<f64>();
    let t_squared = (n1 as f64 * n2 as f64 / total as f64) * quadratic;
    if !t_squared.is_finite() || t_squared < 0.0 {
        return None;
    }
    let df1 = dimensions;
    let df2 = total - dimensions - 1;
    let f_statistic = (df2 as f64 / (df1 as f64 * (total - 2) as f64)) * t_squared;
    let p_value = f_survival(f_statistic, df1 as f64, df2 as f64)?;
    Some((f_statistic, p_value, df1, df2))
}

fn log_gamma(z: f64) -> f64 {
    const COEFFICIENTS: [f64; 8] = [
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if z < 0.5 {
        return std::f64::consts::PI.ln()
            - (std::f64::consts::PI * z).sin().ln()
            - log_gamma(1.0 - z);
    }
    let shifted = z - 1.0;
    let mut x = 0.999_999_999_999_809_9;
    for (index, coefficient) in COEFFICIENTS.iter().enumerate() {
        x += coefficient / (shifted + index as f64 + 1.0);
    }
    let t = shifted + 7.5;
    0.5 * (2.0 * std::f64::consts::PI).ln()
        + (shifted + 0.5) * t.ln()
        - t
        + x.ln()
}

fn beta_continued_fraction(a: f64, b: f64, x: f64) -> Option<f64> {
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
            return h.is_finite().then_some(h);
        }
    }
    None
}

fn regularized_beta(x: f64, a: f64, b: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&x) || a <= 0.0 || b <= 0.0 {
        return None;
    }
    if x == 0.0 {
        return Some(0.0);
    }
    if x == 1.0 {
        return Some(1.0);
    }
    let log_bt = log_gamma(a + b) - log_gamma(a) - log_gamma(b)
        + a * x.ln()
        + b * (1.0 - x).ln();
    let bt = log_bt.exp();
    let result = if x < (a + 1.0) / (a + b + 2.0) {
        bt * beta_continued_fraction(a, b, x)? / a
    } else {
        1.0 - bt * beta_continued_fraction(b, a, 1.0 - x)? / b
    };
    Some(result.clamp(0.0, 1.0))
}

fn f_survival(f_statistic: f64, df1: f64, df2: f64) -> Option<f64> {
    if f_statistic.is_nan() || f_statistic < 0.0 || df1 <= 0.0 || df2 <= 0.0 {
        return None;
    }
    if f_statistic.is_infinite() {
        return Some(0.0);
    }
    let x = df2 / (df2 + df1 * f_statistic);
    regularized_beta(x, df2 / 2.0, df1 / 2.0)
}

fn evaluate(input: &MonitorInput, input_sha256: &str) -> MonitorReport {
    if let Err(reason) = validate_input(input) {
        return indeterminate(
            input,
            input_sha256,
            input.baseline.residuals.len(),
            input.testing.residuals.len(),
            reason,
        );
    }
    let baseline = residual_matrix(&input.baseline);
    let testing = residual_matrix(&input.testing);
    let baseline_covariance = match covariance(&baseline) {
        Some(value) => value,
        None => {
            return indeterminate(
                input,
                input_sha256,
                baseline.len(),
                testing.len(),
                "BASELINE_COVARIANCE_UNAVAILABLE",
            );
        }
    };
    let testing_covariance = match covariance(&testing) {
        Some(value) => value,
        None => {
            return indeterminate(
                input,
                input_sha256,
                baseline.len(),
                testing.len(),
                "TESTING_COVARIANCE_UNAVAILABLE",
            );
        }
    };
    if (0..3).any(|index| {
        baseline_covariance[index][index] <= 0.0 || testing_covariance[index][index] <= 0.0
    }) {
        return indeterminate(
            input,
            input_sha256,
            baseline.len(),
            testing.len(),
            "ZERO_COORDINATE_VARIANCE",
        );
    }
    let baseline_robust = robust_ombrien_rows(&baseline);
    let testing_robust = robust_ombrien_rows(&testing);
    let (statistic, p_value, df1, df2) =
        match hotelling_two_group(&baseline_robust, &testing_robust) {
            Some(value) => value,
            None => {
                return indeterminate(
                    input,
                    input_sha256,
                    baseline.len(),
                    testing.len(),
                    "ROBUST_OMNIBUS_TEST_SINGULAR_OR_UNDERSAMPLED",
                );
            }
        };
    let base_summary = covariance_summary(baseline_covariance);
    let test_summary = covariance_summary(testing_covariance);
    let standard_deviation_ratio = std::array::from_fn(|index| {
        test_summary.standard_deviation[index] / base_summary.standard_deviation[index]
    });
    let (disposition, reason_codes, advisory) = if p_value < input.method.alpha {
        (
            Disposition::SignificantPrecisionChangeDetected,
            vec!["SPHERICAL_COORDINATE_COVARIANCE_CHANGED_SIGNIFICANTLY".to_owned()],
            "SIGNIFICANT PRECISION CHANGE DETECTED: investigate and consider requalification; this is not an accuracy verdict.",
        )
    } else {
        (
            Disposition::NoSignificantPrecisionChangeDetected,
            vec!["NO_SIGNIFICANT_COVARIANCE_CHANGE_DETECTED".to_owned()],
            "NO SIGNIFICANT CHANGE DETECTED: failure to reject equal covariance is not proof of unchanged precision, accuracy, or task fitness.",
        )
    };
    MonitorReport {
        schema_version: "0.1".to_owned(),
        method_id: input.method.method_id.clone(),
        method_revision: input.method.method_revision.clone(),
        input_sha256: input_sha256.to_owned(),
        disposition,
        reason_codes,
        alpha: input.method.alpha,
        baseline_residual_count: baseline.len(),
        testing_residual_count: testing.len(),
        statistic: Some(statistic),
        p_value: Some(p_value),
        numerator_degrees_of_freedom: Some(df1),
        denominator_degrees_of_freedom: Some(df2),
        baseline_covariance: Some(base_summary),
        testing_covariance: Some(test_summary),
        standard_deviation_ratio_testing_over_baseline: Some(standard_deviation_ratio),
        accuracy_disposition: "NOT_EVALUATED",
        task_fitness_disposition: "NOT_EVALUATED",
        advisory,
    }
}

fn run() -> Result<bool, Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut input_path = "tools/scan-bim-tls-health-monitor/fixtures/no-change.json".to_owned();
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => input_path = args.next().ok_or("--input requires a path")?,
            "--help" | "-h" => {
                println!("scan-bim-tls-health-monitor [--input PATH]");
                return Ok(true);
            }
            unknown => return Err(format!("unknown argument: {unknown}").into()),
        }
    }
    let bytes = fs::read(input_path)?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("input exceeds maximum allowed size".into());
    }
    let input: MonitorInput = serde_json::from_slice(&bytes)?;
    let report = evaluate(&input, &sha256_hex(&bytes));
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(matches!(
        report.disposition,
        Disposition::NoSignificantPrecisionChangeDetected
            | Disposition::SignificantPrecisionChangeDetected
    ))
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(2),
        Err(error) => {
            eprintln!("scan-bim-tls-health-monitor: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/no-change.json");

    fn fixture() -> MonitorInput {
        serde_json::from_str(FIXTURE).expect("no-change fixture parses")
    }

    fn deterministic_residuals(seed: usize, scale: [f64; 3]) -> Vec<SphericalResidual> {
        let mut rows = Vec::new();
        for target in 0..20 {
            for position in 0..4 {
                let index = (seed + target * 31 + position * 17 + target * position * 3) as f64;
                let theta = (index * 0.618_033_988_75).sin()
                    + 0.37 * (index * 0.173_205_080_75).cos();
                let elevation = (index * 0.414_213_562_37 + 0.4).sin()
                    + 0.29 * (index * 0.271_828_182_84).cos();
                let range = (index * 0.732_050_807_56 + 0.7).sin()
                    + 0.43 * (index * 0.141_421_356_23).cos();
                rows.push(SphericalResidual {
                    target_id: format!("T{target:02}"),
                    position_id: format!("P{position:02}"),
                    azimuth_arcsec: theta * 0.45 * scale[0],
                    elevation_arcsec: elevation * 0.35 * scale[1],
                    range_mm: range * 0.8 * scale[2],
                });
            }
        }
        rows
    }

    fn prepared_input(test_scales: [f64; 3]) -> MonitorInput {
        let mut input = fixture();
        input.baseline.residuals = deterministic_residuals(7, [1.0, 1.0, 1.0]);
        input.testing.residuals = deterministic_residuals(7, test_scales);
        input
    }

    #[test]
    fn method_name_is_explicit_and_versioned() {
        let input = fixture();
        assert_eq!(input.method.method_id, "OBRIEN_ROBUST_OMNIBUS_IPA_PART_II");
        assert!(!input.method.method_revision.is_empty());
    }

    #[test]
    fn identical_covariance_fixture_does_not_invent_precision_change() {
        let input = prepared_input([1.0, 1.0, 1.0]);
        let report = evaluate(&input, "a".repeat(64).as_str());
        assert!(matches!(
            report.disposition,
            Disposition::NoSignificantPrecisionChangeDetected
        ));
        assert_eq!(report.accuracy_disposition, "NOT_EVALUATED");
        assert_eq!(report.task_fitness_disposition, "NOT_EVALUATED");
        assert_eq!(report.p_value, Some(1.0));
    }

    #[test]
    fn range_precision_degradation_is_detected() {
        let input = prepared_input([1.0, 1.0, 3.0]);
        let report = evaluate(&input, "b".repeat(64).as_str());
        assert!(matches!(
            report.disposition,
            Disposition::SignificantPrecisionChangeDetected
        ));
        assert!(report.p_value.unwrap() < input.method.alpha);
        assert!(report.standard_deviation_ratio_testing_over_baseline.unwrap()[2] > 2.9);
    }

    #[test]
    fn elevation_precision_degradation_is_detected() {
        let input = prepared_input([1.0, 2.5, 1.0]);
        let report = evaluate(&input, "c".repeat(64).as_str());
        assert!(matches!(
            report.disposition,
            Disposition::SignificantPrecisionChangeDetected
        ));
        assert!(report.p_value.unwrap() < input.method.alpha);
    }

    #[test]
    fn covariance_only_change_is_detected() {
        let mut input = prepared_input([1.0, 1.0, 1.0]);
        for row in &mut input.testing.residuals {
            row.elevation_arcsec += 0.75 * row.azimuth_arcsec;
        }
        let report = evaluate(&input, "d".repeat(64).as_str());
        assert!(matches!(
            report.disposition,
            Disposition::SignificantPrecisionChangeDetected
        ));
        assert!(report.p_value.unwrap() < input.method.alpha);
    }

    #[test]
    fn no_significance_is_not_a_fitness_certificate() {
        let input = prepared_input([1.0, 1.0, 1.0]);
        let report = evaluate(&input, "e".repeat(64).as_str());
        assert_eq!(report.task_fitness_disposition, "NOT_EVALUATED");
        assert!(report.advisory.contains("not proof"));
    }

    #[test]
    fn profile_or_calibration_change_requires_new_baseline() {
        let mut input = fixture();
        input.instrument.sensor_profile_sha256 = "f".repeat(64);
        assert_eq!(
            validate_input(&input),
            Err("TIMEPOINT_INSTRUMENT_IDENTITY_MISMATCH")
        );
    }

    #[test]
    fn wrong_units_are_indeterminate() {
        let mut input = fixture();
        input.units.range = "metre".to_owned();
        assert_eq!(validate_input(&input), Err("SPHERICAL_RESIDUAL_UNITS_UNSUPPORTED_OR_MISMATCHED"));
    }

    #[test]
    fn zero_or_invalid_significance_threshold_is_rejected() {
        let mut input = fixture();
        input.method.alpha = 0.0;
        assert_eq!(validate_input(&input), Err("SIGNIFICANCE_LEVEL_OUT_OF_RANGE"));
    }

    #[test]
    fn changed_configuration_is_not_comparable_with_same_baseline() {
        let mut input = fixture();
        input.instrument.configuration_sha256 = "0".repeat(64);
        assert_eq!(
            validate_input(&input),
            Err("TIMEPOINT_INSTRUMENT_IDENTITY_MISMATCH")
        );
    }

    #[test]
    fn invalid_firmware_digest_is_rejected() {
        let mut input = fixture();
        input.instrument.firmware_sha256 = "bad-digest".to_owned();
        assert_eq!(validate_input(&input), Err("INSTRUMENT_EVIDENCE_DIGEST_INVALID"));
    }

    #[test]
    fn timepoints_must_be_ordered() {
        let mut input = fixture();
        input.testing.captured_utc = input.baseline.captured_utc.clone();
        assert_eq!(validate_input(&input), Err("TIMEPOINT_ORDER_OR_TIMESTAMP_INVALID"));
    }

    #[test]
    fn duplicated_target_position_pair_is_rejected() {
        let mut input = fixture();
        input.testing.residuals[1].target_id = input.testing.residuals[0].target_id.clone();
        input.testing.residuals[1].position_id = input.testing.residuals[0].position_id.clone();
        assert_eq!(validate_input(&input), Err("RESIDUAL_TARGET_POSITION_GRID_INVALID"));
    }

    #[test]
    fn nonfinite_spherical_residual_is_rejected() {
        let mut input = fixture();
        input.testing.residuals[0].range_mm = f64::NAN;
        assert_eq!(validate_input(&input), Err("NONFINITE_SPHERICAL_RESIDUAL"));
    }

    #[test]
    fn target_count_below_declared_floor_is_rejected() {
        let mut input = fixture();
        input.baseline.target_ids.truncate(19);
        assert_eq!(validate_input(&input), Err("TARGET_OR_POSITION_COUNT_BELOW_DECLARED_MINIMUM"));
    }

    #[test]
    fn insufficient_position_count_is_rejected() {
        let mut input = fixture();
        input.baseline.position_ids.truncate(3);
        assert_eq!(validate_input(&input), Err("TARGET_OR_POSITION_COUNT_BELOW_DECLARED_MINIMUM"));
    }

    #[test]
    fn singular_pooled_covariance_is_indeterminate() {
        let mut input = prepared_input([1.0, 1.0, 1.0]);
        for row in &mut input.baseline.residuals {
            row.azimuth_arcsec = row.elevation_arcsec;
            row.range_mm = row.elevation_arcsec;
        }
        for row in &mut input.testing.residuals {
            row.azimuth_arcsec = row.elevation_arcsec;
            row.range_mm = row.elevation_arcsec;
        }
        let report = evaluate(&input, "f".repeat(64).as_str());
        assert!(matches!(report.disposition, Disposition::Indeterminate));
    }

    #[test]
    fn robust_transform_is_median_centered_and_finite() {
        let rows = deterministic_residuals(11, [1.0, 1.0, 1.0]);
        let matrix: Vec<[f64; 3]> = rows
            .iter()
            .map(|row| [row.azimuth_arcsec, row.elevation_arcsec, row.range_mm])
            .collect();
        let transformed = robust_ombrien_rows(&matrix);
        assert_eq!(transformed.len(), matrix.len());
        assert!(transformed.iter().flatten().all(|value| value.is_finite()));
    }

    #[test]
    fn input_permutation_changes_byte_digest_not_statistical_result() {
        let mut input = prepared_input([1.0, 2.5, 1.0]);
        let original = evaluate(&input, "1".repeat(64).as_str());
        input.testing.residuals.reverse();
        let permuted = evaluate(&input, "2".repeat(64).as_str());
        assert_eq!(original.disposition, permuted.disposition);
        assert!((original.p_value.unwrap() - permuted.p_value.unwrap()).abs() < 1e-10);
    }

    #[test]
    fn f_survival_has_known_reference_values() {
        assert!((f_survival(0.0, 6.0, 73.0).unwrap() - 1.0).abs() < 1e-12);
        assert!(f_survival(100.0, 6.0, 73.0).unwrap() < 1e-20);
    }

    #[test]
    fn exact_input_digest_is_content_bound() {
        assert_ne!(sha256_hex(b"baseline"), sha256_hex(b"baseline reordered"));
    }
}
