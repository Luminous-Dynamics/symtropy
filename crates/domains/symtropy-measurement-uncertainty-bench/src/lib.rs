//! Correlated Monte Carlo propagation through dimension-checked measurement equations.
//!
//! This is a bounded, reproducible measurement-model evaluator, not a complete
//! GUM/JCGM implementation or an operational decision authority.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const DIMENSIONS: usize = 7;
const VALIDATION_TOLERANCE: f64 = 1.0e-10;
const MAX_INPUTS: usize = 64;
const MAX_OUTPUTS: usize = 32;
const MAX_INSTRUCTIONS: usize = 512;
const MAX_SAMPLES: usize = 2_000_000;
const NORMAL_CDF_U_EPSILON: f64 = 1.0e-15;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Dimension(pub [i32; DIMENSIONS]);

#[derive(Clone, Debug, PartialEq)]
pub struct QuantileEstimate {
    pub probability: f64,
    pub estimate: f64,
    pub monte_carlo_interval_lower: Option<f64>,
    pub monte_carlo_interval_upper: Option<f64>,
    pub achieved_confidence: Option<f64>,
    pub monte_carlo_interval_width: Option<f64>,
    pub maximum_interval_width: f64,
    pub disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConvergenceReport {
    pub batch_count: usize,
    pub required_stable_tail_transitions: usize,
    pub last_instability_batch_index: Option<usize>,
    pub terminal_stable_transitions: usize,
    pub maximum_mean_drift: f64,
    pub maximum_standard_deviation_drift: f64,
    pub maximum_quantile_drift: f64,
    pub disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputReport {
    pub output_id: String,
    pub unit: String,
    pub dimension: Dimension,
    pub requested_samples: usize,
    pub valid_samples: usize,
    pub invalid_samples: usize,
    pub invalid_fraction: f64,
    pub mean: Option<f64>,
    pub standard_deviation: Option<f64>,
    pub standard_error_of_mean: Option<f64>,
    pub median: Option<f64>,
    pub quantiles: Vec<QuantileEstimate>,
    pub coverage_lower_probability: f64,
    pub coverage_lower_value: Option<f64>,
    pub coverage_upper_probability: f64,
    pub coverage_upper_value: Option<f64>,
    pub convergence: ConvergenceReport,
    pub disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JointOutputDependence {
    pub output_ids: Vec<String>,
    pub units: Vec<String>,
    pub joint_valid_sample_count: usize,
    pub covariance_matrix: Vec<Vec<Option<f64>>>,
    pub correlation_matrix: Vec<Vec<Option<f64>>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub evidence_disposition: String,
    pub dependence_model: String,
    pub prng: String,
    pub seed: u64,
    pub requested_samples: usize,
    pub batch_size: usize,
    pub outputs: Vec<OutputReport>,
    pub joint_output_dependence: JointOutputDependence,
    pub overall_disposition: String,
    pub claim_boundary: String,
}

#[derive(Clone, Debug)]
enum Marginal {
    Normal { mean: f64, standard_deviation: f64 },
    Lognormal { log_mean: f64, log_standard_deviation: f64 },
    Uniform { lower: f64, upper: f64 },
    Triangular { lower: f64, mode: f64, upper: f64 },
}

impl Marginal {
    fn sample(&self, latent_normal: f64) -> f64 {
        match self {
            Self::Normal { mean, standard_deviation } => {
                mean + standard_deviation * latent_normal
            }
            Self::Lognormal { log_mean, log_standard_deviation } => {
                (log_mean + log_standard_deviation * latent_normal).exp()
            }
            Self::Uniform { lower, upper } => {
                let u = standard_normal_cdf(latent_normal)
                    .clamp(NORMAL_CDF_U_EPSILON, 1.0 - NORMAL_CDF_U_EPSILON);
                lower + u * (upper - lower)
            }
            Self::Triangular { lower, mode, upper } => {
                let u = standard_normal_cdf(latent_normal)
                    .clamp(NORMAL_CDF_U_EPSILON, 1.0 - NORMAL_CDF_U_EPSILON);
                let width = upper - lower;
                let split = (mode - lower) / width;
                if u < split {
                    lower + (u * width * (mode - lower)).sqrt()
                } else {
                    upper - ((1.0 - u) * width * (upper - mode)).sqrt()
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
struct InputSpec {
    id: String,
    dimension: Dimension,
    scale_to_si: f64,
    marginal: Marginal,
}

#[derive(Clone, Debug)]
enum Instruction {
    Input(usize),
    Constant { si_value: f64 },
    Add,
    Subtract,
    Multiply,
    Divide,
    Sqrt,
    Log,
    Exp,
    Abs,
}

#[derive(Clone, Debug)]
struct QuantileTarget {
    probability: f64,
    maximum_mc_interval_width: f64,
}

#[derive(Clone, Debug)]
struct OutputSpec {
    id: String,
    unit: String,
    dimension: Dimension,
    scale_from_si: f64,
    instructions: Vec<Instruction>,
    quantile_targets: Vec<QuantileTarget>,
    coverage_lower_probability: f64,
    coverage_upper_probability: f64,
}

#[derive(Clone, Debug)]
struct ConvergenceTarget {
    maximum_mean_drift: f64,
    maximum_standard_deviation_drift: f64,
    maximum_quantile_drift: f64,
}

#[derive(Clone, Debug)]
struct SimulationSpec {
    seed: u64,
    sample_count: usize,
    batch_size: usize,
    maximum_invalid_fraction: f64,
    quantile_estimation_confidence: f64,
    minimum_batch_count: usize,
    stable_tail_batch_count: usize,
    convergence_by_output: BTreeMap<String, ConvergenceTarget>,
}

#[derive(Clone, Debug)]
struct BatchSummary {
    mean: f64,
    standard_deviation: f64,
    quantiles: Vec<f64>,
}

#[derive(Clone, Debug)]
struct OutputConvergenceState {
    previous: Option<BatchSummary>,
    last_instability_batch: Option<usize>,
    maximum_mean_drift: f64,
    maximum_standard_deviation_drift: f64,
    maximum_quantile_drift: f64,
}

#[derive(Clone, Debug)]
struct RunningJointMoments {
    count: usize,
    means: Vec<f64>,
    cross_moments: Vec<Vec<f64>>,
}

impl RunningJointMoments {
    fn new(output_count: usize) -> Self {
        Self {
            count: 0,
            means: vec![0.0; output_count],
            cross_moments: vec![vec![0.0; output_count]; output_count],
        }
    }

    fn update(&mut self, values: &[f64]) {
        self.count += 1;
        let n = self.count as f64;
        let delta: Vec<f64> = values
            .iter()
            .zip(self.means.iter())
            .map(|(value, mean)| value - mean)
            .collect();

        for (mean, change) in self.means.iter_mut().zip(delta.iter()) {
            *mean += change / n;
        }

        for row in 0..values.len() {
            for column in 0..values.len() {
                self.cross_moments[row][column] +=
                    delta[row] * (values[column] - self.means[column]);
            }
        }
    }

    fn matrices(&self) -> (Vec<Vec<Option<f64>>>, Vec<Vec<Option<f64>>>) {
        let n = self.means.len();
        if self.count < 2 {
            return (
                vec![vec![None; n]; n],
                vec![vec![None; n]; n],
            );
        }

        let denominator = (self.count - 1) as f64;
        let covariance_values: Vec<Vec<f64>> = self
            .cross_moments
            .iter()
            .map(|row| row.iter().map(|value| value / denominator).collect())
            .collect();
        let covariance: Vec<Vec<Option<f64>>> = covariance_values
            .iter()
            .map(|row| row.iter().copied().map(Some).collect())
            .collect();
        let mut correlation = vec![vec![None; n]; n];

        for row in 0..n {
            for column in 0..n {
                let variance_product =
                    covariance_values[row][row] * covariance_values[column][column];
                if variance_product > 0.0 && variance_product.is_finite() {
                    correlation[row][column] = Some(
                        covariance_values[row][column] / variance_product.sqrt(),
                    );
                } else if row == column && covariance_values[row][row] == 0.0 {
                    correlation[row][column] = None;
                }
            }
        }

        (covariance, correlation)
    }
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^ (value >> 31)
    }

    fn uniform_open(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64 + 0.5) * (1.0 / 9_007_199_254_740_992.0)
    }
}

struct NormalGenerator {
    rng: SplitMix64,
    spare: Option<f64>,
}

impl NormalGenerator {
    fn new(seed: u64) -> Self {
        Self {
            rng: SplitMix64::new(seed),
            spare: None,
        }
    }

    fn next(&mut self) -> f64 {
        if let Some(value) = self.spare.take() {
            return value;
        }

        let u1 = self.rng.uniform_open();
        let u2 = self.rng.uniform_open();
        let radius = (-2.0 * u1.ln()).sqrt();
        let angle = std::f64::consts::TAU * u2;
        let (sin, cos) = angle.sin_cos();
        self.spare = Some(radius * sin);
        radius * cos
    }
}

fn standard_normal_cdf(value: f64) -> f64 {
    // Abramowitz-Stegun 7.1.26 rational approximation. The approximation is
    // used only to map latent Gaussian values to the non-normal copula marginals.
    let absolute = value.abs();
    let t = 1.0 / (1.0 + 0.2316419 * absolute);
    let density = 0.3989422804014327 * (-0.5 * absolute * absolute).exp();
    let polynomial = t
        * (0.319381530
            + t * (-0.356563782
                + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    let positive_cdf = 1.0 - density * polynomial;
    if value >= 0.0 {
        positive_cdf
    } else {
        1.0 - positive_cdf
    }
}

fn push(
    errors: &mut Vec<Violation>,
    code: &str,
    path: &str,
    detail: impl Into<String>,
) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
}

fn uint(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn parse_dimension(value: &Value) -> Option<Dimension> {
    let values = value.as_array()?;
    if values.len() != DIMENSIONS {
        return None;
    }
    let mut result = [0_i32; DIMENSIONS];
    for (index, item) in values.iter().enumerate() {
        let exponent = item.as_i64()?;
        if !(-12..=12).contains(&exponent) {
            return None;
        }
        result[index] = exponent as i32;
    }
    Some(Dimension(result))
}

fn dimensions_add(left: Dimension, right: Dimension) -> Option<Dimension> {
    let mut result = [0_i32; DIMENSIONS];
    for index in 0..DIMENSIONS {
        result[index] = left.0[index].checked_add(right.0[index])?;
        if !(-24..=24).contains(&result[index]) {
            return None;
        }
    }
    Some(Dimension(result))
}

fn dimensions_subtract(left: Dimension, right: Dimension) -> Option<Dimension> {
    let mut result = [0_i32; DIMENSIONS];
    for index in 0..DIMENSIONS {
        result[index] = left.0[index].checked_sub(right.0[index])?;
        if !(-24..=24).contains(&result[index]) {
            return None;
        }
    }
    Some(Dimension(result))
}

fn dimensions_sqrt(dimension: Dimension) -> Option<Dimension> {
    let mut result = [0_i32; DIMENSIONS];
    for index in 0..DIMENSIONS {
        if dimension.0[index] % 2 != 0 {
            return None;
        }
        result[index] = dimension.0[index] / 2;
    }
    Some(Dimension(result))
}

fn dimensionless(dimension: Dimension) -> bool {
    dimension.0.iter().all(|value| *value == 0)
}

fn marginal_from_value(value: &Value) -> Option<Marginal> {
    match text(value, "type")? {
        "NORMAL" => {
            let mean = finite(value, "mean")?;
            let standard_deviation = finite(value, "standard_deviation")?;
            (standard_deviation > 0.0).then_some(Marginal::Normal {
                mean,
                standard_deviation,
            })
        }
        "LOGNORMAL" => {
            let log_mean = finite(value, "log_mean")?;
            let log_standard_deviation = finite(value, "log_standard_deviation")?;
            (log_standard_deviation > 0.0).then_some(Marginal::Lognormal {
                log_mean,
                log_standard_deviation,
            })
        }
        "UNIFORM" => {
            let lower = finite(value, "lower")?;
            let upper = finite(value, "upper")?;
            (upper > lower).then_some(Marginal::Uniform { lower, upper })
        }
        "TRIANGULAR" => {
            let lower = finite(value, "lower")?;
            let mode = finite(value, "mode")?;
            let upper = finite(value, "upper")?;
            (upper > lower && mode >= lower && mode <= upper)
                .then_some(Marginal::Triangular { lower, mode, upper })
        }
        _ => None,
    }
}

fn cholesky(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let size = matrix.len();
    if size == 0 || matrix.iter().any(|row| row.len() != size) {
        return Err("correlation matrix must be non-empty and square".to_owned());
    }
    let mut lower = vec![vec![0.0; size]; size];
    for row in 0..size {
        for column in 0..=row {
            let mut sum = matrix[row][column];
            for k in 0..column {
                sum -= lower[row][k] * lower[column][k];
            }
            if row == column {
                if !sum.is_finite() || sum <= 1.0e-12 {
                    return Err(
                        "correlation matrix must be strictly positive definite; no jitter or repair is applied"
                            .to_owned(),
                    );
                }
                lower[row][column] = sum.sqrt();
            } else {
                let denominator = lower[column][column];
                if denominator <= 0.0 || !denominator.is_finite() {
                    return Err("invalid Cholesky diagonal".to_owned());
                }
                lower[row][column] = sum / denominator;
            }
        }
    }
    Ok(lower)
}

fn validate_correlation_matrix(
    value: &Value,
    input_count: usize,
    errors: &mut Vec<Violation>,
) -> Option<Vec<Vec<f64>>> {
    let Some(rows) = value.as_array() else {
        push(errors, "MISSING_CORRELATION_MATRIX", "$.correlation_matrix", "correlation_matrix must be a square numeric array");
        return None;
    };
    if rows.len() != input_count {
        push(errors, "CORRELATION_MATRIX_SIZE_MISMATCH", "$.correlation_matrix", "correlation matrix row count must equal input count");
        return None;
    }
    let mut matrix = Vec::with_capacity(input_count);
    for (row_index, row) in rows.iter().enumerate() {
        let Some(columns) = row.as_array() else {
            push(errors, "INVALID_CORRELATION_ROW", &format!("$.correlation_matrix[{row_index}]"), "each correlation row must be an array");
            return None;
        };
        if columns.len() != input_count {
            push(errors, "CORRELATION_MATRIX_SIZE_MISMATCH", &format!("$.correlation_matrix[{row_index}]"), "each correlation row must equal input count");
            return None;
        }
        let mut parsed = Vec::with_capacity(input_count);
        for (column_index, item) in columns.iter().enumerate() {
            let Some(number) = item.as_f64().filter(|value| value.is_finite()) else {
                push(errors, "NONFINITE_CORRELATION", &format!("$.correlation_matrix[{row_index}][{column_index}]"), "correlation coefficients must be finite");
                return None;
            };
            if !(-1.0..=1.0).contains(&number) {
                push(errors, "CORRELATION_OUT_OF_RANGE", &format!("$.correlation_matrix[{row_index}][{column_index}]"), "correlation coefficient must be in [-1,1]");
            }
            parsed.push(number);
        }
        matrix.push(parsed);
    }

    for row in 0..input_count {
        if (matrix[row][row] - 1.0).abs() > VALIDATION_TOLERANCE {
            push(errors, "CORRELATION_DIAGONAL_NOT_ONE", &format!("$.correlation_matrix[{row}][{row}]"), "correlation matrix diagonal must equal one");
        }
        for column in 0..input_count {
            if (matrix[row][column] - matrix[column][row]).abs() > VALIDATION_TOLERANCE {
                push(errors, "CORRELATION_MATRIX_NOT_SYMMETRIC", &format!("$.correlation_matrix[{row}][{column}]"), "correlation matrix must be symmetric");
            }
        }
    }
    if let Err(detail) = cholesky(&matrix) {
        push(errors, "CORRELATION_MATRIX_NOT_POSITIVE_DEFINITE", "$.correlation_matrix", detail);
    }
    Some(matrix)
}

fn validate_instruction_stream(
    instructions: &[Value],
    input_indices: &BTreeMap<String, usize>,
    input_dimensions: &[Dimension],
    output_dimension: Dimension,
    path: &str,
    errors: &mut Vec<Violation>,
) {
    if instructions.is_empty() || instructions.len() > MAX_INSTRUCTIONS {
        push(errors, "INVALID_INSTRUCTION_COUNT", path, format!("instruction count must be between 1 and {MAX_INSTRUCTIONS}"));
        return;
    }
    let mut stack: Vec<Dimension> = Vec::new();

    for (index, instruction) in instructions.iter().enumerate() {
        let instruction_path = format!("{path}[{index}]");
        match text(instruction, "op") {
            Some("INPUT") => {
                let Some(input_id) = text(instruction, "input_id") else {
                    push(errors, "MISSING_INSTRUCTION_INPUT", &instruction_path, "INPUT requires input_id");
                    continue;
                };
                match input_indices.get(input_id) {
                    Some(input_index) => stack.push(input_dimensions[*input_index]),
                    None => push(errors, "UNKNOWN_EQUATION_INPUT", &instruction_path, format!("unknown input {input_id}")),
                }
            }
            Some("CONSTANT") => {
                if finite(instruction, "value").is_none()
                    || finite(instruction, "scale_to_si").is_none_or(|value| value <= 0.0)
                {
                    push(errors, "INVALID_EQUATION_CONSTANT", &instruction_path, "constant value and positive SI conversion are required");
                }
                match instruction.get("dimension").and_then(parse_dimension) {
                    Some(dimension) => stack.push(dimension),
                    None => push(errors, "INVALID_CONSTANT_DIMENSION", &instruction_path, "constant dimension must contain seven bounded integer exponents"),
                }
            }
            Some("ADD" | "SUBTRACT") => {
                if stack.len() < 2 {
                    push(errors, "EQUATION_STACK_UNDERFLOW", &instruction_path, "binary operation requires two stack values");
                    continue;
                }
                let right = stack.pop().expect("stack length checked");
                let left = stack.pop().expect("stack length checked");
                if left != right {
                    push(errors, "DIMENSION_MISMATCH_ON_ADDITION", &instruction_path, "ADD/SUBTRACT require identical SI dimension vectors");
                }
                stack.push(left);
            }
            Some("MULTIPLY" | "DIVIDE") => {
                if stack.len() < 2 {
                    push(errors, "EQUATION_STACK_UNDERFLOW", &instruction_path, "binary operation requires two stack values");
                    continue;
                }
                let right = stack.pop().expect("stack length checked");
                let left = stack.pop().expect("stack length checked");
                let dimension = if text(instruction, "op") == Some("MULTIPLY") {
                    dimensions_add(left, right)
                } else {
                    dimensions_subtract(left, right)
                };
                match dimension {
                    Some(value) => stack.push(value),
                    None => push(errors, "DIMENSION_EXPONENT_OVERFLOW", &instruction_path, "resulting SI dimension exponent is outside supported v0.1 bounds"),
                }
            }
            Some("SQRT") => {
                if stack.is_empty() {
                    push(errors, "EQUATION_STACK_UNDERFLOW", &instruction_path, "SQRT requires one stack value");
                    continue;
                }
                let dimension = stack.pop().expect("stack non-empty");
                match dimensions_sqrt(dimension) {
                    Some(value) => stack.push(value),
                    None => push(errors, "SQRT_DIMENSION_NOT_EVEN", &instruction_path, "SQRT requires every dimension exponent to be even"),
                }
            }
            Some("LOG" | "EXP") => {
                if stack.is_empty() {
                    push(errors, "EQUATION_STACK_UNDERFLOW", &instruction_path, "LOG/EXP requires one stack value");
                    continue;
                }
                let dimension = stack.pop().expect("stack non-empty");
                if !dimensionless(dimension) {
                    push(errors, "DIMENSIONAL_LOG_OR_EXP", &instruction_path, "LOG/EXP requires a dimensionless input");
                }
                stack.push(Dimension([0; DIMENSIONS]));
            }
            Some("ABS") => {
                if stack.is_empty() {
                    push(errors, "EQUATION_STACK_UNDERFLOW", &instruction_path, "ABS requires one stack value");
                }
            }
            Some(other) => push(errors, "UNKNOWN_EQUATION_OPCODE", &instruction_path, format!("unknown opcode {other}")),
            None => push(errors, "MISSING_EQUATION_OPCODE", &instruction_path, "each instruction requires op"),
        }
    }

    if stack.len() != 1 {
        push(errors, "EQUATION_STACK_FINAL_DEPTH", path, "measurement equation must leave exactly one value on the stack");
    } else if stack[0] != output_dimension {
        push(errors, "OUTPUT_DIMENSION_MISMATCH", path, "equation result dimension does not match declared output dimension");
    }
}

/// Validate probability distributions, correlation, equations, dimensions and
/// numerical stopping controls before any simulation is performed.
pub fn validate_case(case: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA_VERSION", "$.schema_version", "schema_version must be exactly 1");
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    if !matches!(text(case, "evidence_class"), Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")) {
        push(&mut errors, "INVALID_EVIDENCE_CLASS", "$.evidence_class", "evidence class is outside v0.1 vocabulary");
    }

    let Some(dependence) = case.get("dependence_model") else {
        push(&mut errors, "MISSING_DEPENDENCE_MODEL", "$.dependence_model", "dependence model and provenance are required");
        return errors;
    };
    if text(dependence, "type") != Some("GAUSSIAN_COPULA") {
        push(&mut errors, "UNSUPPORTED_DEPENDENCE_MODEL", "$.dependence_model.type", "v0.1 supports only the explicitly declared GAUSSIAN_COPULA dependence model");
    }
    for key in ["method_ref", "source_ref", "assumption_ref"] {
        if text(dependence, key).is_none() {
            push(&mut errors, "MISSING_DEPENDENCE_PROVENANCE", &format!("$.dependence_model.{key}"), "dependence method, source and assumption references are required");
        }
    }
    if !matches!(text(dependence, "evidence_role"), Some("INDEPENDENT_MEASUREMENT" | "EXTERNAL_CALIBRATION" | "EXPERT_ELICITATION" | "SIMULATION_ONLY")) {
        push(&mut errors, "INVALID_DEPENDENCE_EVIDENCE_ROLE", "$.dependence_model.evidence_role", "dependence evidence role must be explicit");
    }

    let Some(inputs) = case.get("inputs").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_INPUTS", "$.inputs", "inputs must be an array");
        return errors;
    };
    if inputs.is_empty() || inputs.len() > MAX_INPUTS {
        push(&mut errors, "INPUT_COUNT_OUT_OF_RANGE", "$.inputs", format!("input count must be between 1 and {MAX_INPUTS}"));
    }
    let mut input_ids = BTreeSet::new();
    let mut input_indices = BTreeMap::new();
    let mut input_dimensions = Vec::new();

    for (index, input) in inputs.iter().enumerate() {
        let path = format!("$.inputs[{index}]");
        let Some(id) = text(input, "input_id") else {
            push(&mut errors, "MISSING_INPUT_ID", &format!("{path}.input_id"), "input_id is required");
            continue;
        };
        if !input_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_INPUT_ID", &format!("{path}.input_id"), "input identifiers must be unique");
        }
        input_indices.insert(id.to_owned(), index);

        if text(input, "unit").is_none()
            || text(input, "method_ref").is_none()
            || text(input, "source_ref").is_none()
            || text(input, "parameter_validation_ref").is_none()
            || text(input, "calibration_ref").is_none()
        {
            push(&mut errors, "MISSING_INPUT_PROVENANCE", &path, "unit, calibration, method, source and parameter-validation references are required");
        }
        if !matches!(text(input, "evidence_role"), Some("INDEPENDENT_MEASUREMENT" | "EXTERNAL_CALIBRATION" | "EXPERT_ELICITATION" | "SIMULATION_ONLY")) {
            push(&mut errors, "INVALID_INPUT_EVIDENCE_ROLE", &format!("{path}.evidence_role"), "input evidence role must be explicit");
        }
        match finite(input, "scale_to_si") {
            Some(value) if value > 0.0 => {}
            _ => push(&mut errors, "INVALID_INPUT_SCALE", &format!("{path}.scale_to_si"), "scale_to_si must be finite and positive"),
        }
        match input.get("dimension").and_then(parse_dimension) {
            Some(dimension) => input_dimensions.push(dimension),
            None => {
                push(&mut errors, "INVALID_INPUT_DIMENSION", &format!("{path}.dimension"), "dimension must contain seven integer SI exponents in [-12,12]");
                input_dimensions.push(Dimension([0; DIMENSIONS]));
            }
        }
        if input.get("distribution").and_then(marginal_from_value).is_none() {
            push(&mut errors, "INVALID_INPUT_DISTRIBUTION", &format!("{path}.distribution"), "distribution must be a valid NORMAL, LOGNORMAL, UNIFORM or TRIANGULAR marginal with finite parameters");
        }
    }
    let matrix = validate_correlation_matrix(
        case.get("correlation_matrix").unwrap_or(&Value::Null),
        inputs.len(),
        &mut errors,
    );

    let Some(outputs) = case.get("outputs").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_OUTPUTS", "$.outputs", "outputs must be an array");
        return errors;
    };
    if outputs.is_empty() || outputs.len() > MAX_OUTPUTS {
        push(&mut errors, "OUTPUT_COUNT_OUT_OF_RANGE", "$.outputs", format!("output count must be between 1 and {MAX_OUTPUTS}"));
    }
    let mut output_ids = BTreeSet::new();

    for (index, output) in outputs.iter().enumerate() {
        let path = format!("$.outputs[{index}]");
        let Some(id) = text(output, "output_id") else {
            push(&mut errors, "MISSING_OUTPUT_ID", &format!("{path}.output_id"), "output_id is required");
            continue;
        };
        if !output_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_OUTPUT_ID", &format!("{path}.output_id"), "output identifiers must be unique");
        }
        for key in ["unit", "equation_ref", "source_ref", "equation_review_ref"] {
            if text(output, key).is_none() {
                push(&mut errors, "MISSING_OUTPUT_PROVENANCE", &format!("{path}.{key}"), "output unit, equation and review provenance are required");
            }
        }
        match finite(output, "scale_from_si") {
            Some(value) if value > 0.0 => {}
            _ => push(&mut errors, "INVALID_OUTPUT_SCALE", &format!("{path}.scale_from_si"), "scale_from_si must be finite and positive"),
        }
        let output_dimension = match output.get("dimension").and_then(parse_dimension) {
            Some(value) => value,
            None => {
                push(&mut errors, "INVALID_OUTPUT_DIMENSION", &format!("{path}.dimension"), "output dimension must contain seven bounded integer SI exponents");
                Dimension([0; DIMENSIONS])
            }
        };

        let Some(instructions) = output.get("instructions").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_OUTPUT_INSTRUCTIONS", &format!("{path}.instructions"), "instructions must be an array");
            continue;
        };
        validate_instruction_stream(
            instructions,
            &input_indices,
            &input_dimensions,
            output_dimension,
            &format!("{path}.instructions"),
            &mut errors,
        );

        let Some(targets) = output.get("quantile_targets").and_then(Value::as_array) else {
            push(&mut errors, "MISSING_QUANTILE_TARGETS", &format!("{path}.quantile_targets"), "quantile targets must be an array");
            continue;
        };
        if targets.is_empty() {
            push(&mut errors, "EMPTY_QUANTILE_TARGETS", &format!("{path}.quantile_targets"), "at least one quantile target is required");
        }
        let mut probabilities = BTreeSet::new();
        for (target_index, target) in targets.iter().enumerate() {
            let target_path = format!("{path}.quantile_targets[{target_index}]");
            match finite(target, "probability") {
                Some(probability) if probability > 0.0 && probability < 1.0 => {
                    if !probabilities.insert(probability.to_bits()) {
                        push(&mut errors, "DUPLICATE_QUANTILE_PROBABILITY", &target_path, "quantile probabilities must be unique");
                    }
                }
                _ => push(&mut errors, "INVALID_QUANTILE_PROBABILITY", &target_path, "quantile probability must be in (0,1)"),
            }
            match finite(target, "maximum_mc_interval_width") {
                Some(width) if width > 0.0 => {}
                _ => push(&mut errors, "INVALID_MC_QUANTILE_WIDTH", &target_path, "maximum Monte Carlo quantile interval width must be finite and positive"),
            }
        }

        let Some(coverage) = output.get("coverage_interval") else {
            push(&mut errors, "MISSING_COVERAGE_INTERVAL", &format!("{path}.coverage_interval"), "coverage interval declaration is required");
            continue;
        };
        let lower = finite(coverage, "lower_probability");
        let upper = finite(coverage, "upper_probability");
        match (lower, upper) {
            (Some(lo), Some(hi)) if lo > 0.0 && hi < 1.0 && lo < hi => {
                if !probabilities.contains(&lo.to_bits()) || !probabilities.contains(&hi.to_bits()) {
                    push(&mut errors, "COVERAGE_ENDPOINT_NOT_A_QUANTILE_TARGET", &format!("{path}.coverage_interval"), "coverage interval endpoints must also be declared quantile targets");
                }
            }
            _ => push(&mut errors, "INVALID_COVERAGE_INTERVAL", &format!("{path}.coverage_interval"), "coverage probabilities must satisfy 0 < lower < upper < 1"),
        }
    }

    let Some(simulation) = case.get("simulation") else {
        push(&mut errors, "MISSING_SIMULATION_CONFIG", "$.simulation", "simulation config is required");
        return errors;
    };
    if text(simulation, "prng") != Some("SPLITMIX64_BOX_MULLER_V1") {
        push(&mut errors, "UNSUPPORTED_PRNG", "$.simulation.prng", "v0.1 requires SPLITMIX64_BOX_MULLER_V1");
    }
    if uint(simulation, "seed").is_none() {
        push(&mut errors, "INVALID_RANDOM_SEED", "$.simulation.seed", "seed must be a non-negative integer");
    }
    let sample_count = uint(simulation, "sample_count").unwrap_or(0);
    let batch_size = uint(simulation, "batch_size").unwrap_or(0);
    if sample_count < 1024 || sample_count > MAX_SAMPLES as u64 {
        push(&mut errors, "SAMPLE_COUNT_OUT_OF_RANGE", "$.simulation.sample_count", format!("sample_count must be between 1024 and {MAX_SAMPLES}"));
    }
    if batch_size < 128 || batch_size == 0 || sample_count % batch_size != 0 {
        push(&mut errors, "INVALID_BATCH_SIZE", "$.simulation.batch_size", "batch_size must be >= 128 and divide sample_count exactly");
    }
    let batch_count = if batch_size > 0 { sample_count / batch_size } else { 0 };
    let Some(convergence) = simulation.get("convergence") else {
        push(&mut errors, "MISSING_CONVERGENCE_POLICY", "$.simulation.convergence", "numerical convergence policy is required");
        return errors;
    };
    let minimum_batches = uint(convergence, "minimum_batch_count").unwrap_or(0);
    let stable_tail = uint(convergence, "stable_tail_batch_count").unwrap_or(0);
    if minimum_batches < 2 || minimum_batches > batch_count {
        push(&mut errors, "INVALID_MINIMUM_BATCH_COUNT", "$.simulation.convergence.minimum_batch_count", "minimum batch count must be >= 2 and no greater than the configured simulation batch count");
    }
    if stable_tail == 0 || stable_tail >= batch_count {
        push(&mut errors, "INVALID_STABLE_TAIL_COUNT", "$.simulation.convergence.stable_tail_batch_count", "stable tail count must be positive and less than configured batch count");
    }
    match finite(simulation, "maximum_invalid_fraction") {
        Some(value) if (0.0..=1.0).contains(&value) => {}
        _ => push(&mut errors, "INVALID_MAXIMUM_INVALID_FRACTION", "$.simulation.maximum_invalid_fraction", "maximum invalid fraction must be in [0,1]"),
    }
    match finite(simulation, "quantile_estimation_confidence") {
        Some(value) if value > 0.5 && value < 1.0 => {}
        _ => push(&mut errors, "INVALID_QUANTILE_ESTIMATION_CONFIDENCE", "$.simulation.quantile_estimation_confidence", "quantile-estimation confidence must be in (0.5,1)"),
    }

    let Some(convergence_outputs) = convergence.get("outputs").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_OUTPUT_CONVERGENCE_TARGETS", "$.simulation.convergence.outputs", "one convergence target per output is required");
        return errors;
    };
    let mut convergence_ids = BTreeSet::new();
    for (index, target) in convergence_outputs.iter().enumerate() {
        let path = format!("$.simulation.convergence.outputs[{index}]");
        let Some(id) = text(target, "output_id") else {
            push(&mut errors, "MISSING_CONVERGENCE_OUTPUT_ID", &format!("{path}.output_id"), "output_id is required");
            continue;
        };
        if !output_ids.contains(id) {
            push(&mut errors, "UNKNOWN_CONVERGENCE_OUTPUT", &format!("{path}.output_id"), "convergence target references unknown output");
        }
        if !convergence_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_CONVERGENCE_OUTPUT", &format!("{path}.output_id"), "convergence outputs must be unique");
        }
        for key in ["maximum_mean_drift", "maximum_standard_deviation_drift", "maximum_quantile_drift"] {
            match finite(target, key) {
                Some(value) if value > 0.0 => {}
                _ => push(&mut errors, "INVALID_CONVERGENCE_TOLERANCE", &format!("{path}.{key}"), "convergence tolerance must be finite and positive"),
            }
        }
    }
    if convergence_ids != output_ids {
        push(&mut errors, "CONVERGENCE_OUTPUT_SET_MISMATCH", "$.simulation.convergence.outputs", "convergence targets must cover every output exactly once");
    }

    if matrix.is_none() {
        // A matrix-level error has already been recorded.
    }

    errors
}

fn parse_inputs(case: &Value) -> Vec<InputSpec> {
    case["inputs"]
        .as_array()
        .expect("validated inputs")
        .iter()
        .map(|input| InputSpec {
            id: text(input, "input_id").expect("validated input ID").to_owned(),
            dimension: parse_dimension(&input["dimension"]).expect("validated input dimension"),
            scale_to_si: finite(input, "scale_to_si").expect("validated input scale"),
            marginal: marginal_from_value(&input["distribution"]).expect("validated input distribution"),
        })
        .collect()
}

fn parse_instruction(
    value: &Value,
    input_indices: &BTreeMap<String, usize>,
) -> Instruction {
    match text(value, "op").expect("validated opcode") {
        "INPUT" => Instruction::Input(
            input_indices[text(value, "input_id").expect("validated input ID")],
        ),
        "CONSTANT" => Instruction::Constant {
            si_value: finite(value, "value").expect("validated constant") *
                finite(value, "scale_to_si").expect("validated constant scale"),
        },
        "ADD" => Instruction::Add,
        "SUBTRACT" => Instruction::Subtract,
        "MULTIPLY" => Instruction::Multiply,
        "DIVIDE" => Instruction::Divide,
        "SQRT" => Instruction::Sqrt,
        "LOG" => Instruction::Log,
        "EXP" => Instruction::Exp,
        "ABS" => Instruction::Abs,
        _ => unreachable!("validated opcode"),
    }
}

fn parse_outputs(case: &Value, inputs: &[InputSpec]) -> Vec<OutputSpec> {
    let input_indices: BTreeMap<String, usize> = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| (input.id.clone(), index))
        .collect();

    case["outputs"]
        .as_array()
        .expect("validated outputs")
        .iter()
        .map(|output| {
            let instructions = output["instructions"]
                .as_array()
                .expect("validated instructions")
                .iter()
                .map(|instruction| parse_instruction(instruction, &input_indices))
                .collect();
            let quantile_targets = output["quantile_targets"]
                .as_array()
                .expect("validated quantile targets")
                .iter()
                .map(|target| QuantileTarget {
                    probability: finite(target, "probability").expect("validated target probability"),
                    maximum_mc_interval_width: finite(target, "maximum_mc_interval_width").expect("validated interval width"),
                })
                .collect();
            OutputSpec {
                id: text(output, "output_id").expect("validated output ID").to_owned(),
                unit: text(output, "unit").expect("validated output unit").to_owned(),
                dimension: parse_dimension(&output["dimension"]).expect("validated output dimension"),
                scale_from_si: finite(output, "scale_from_si").expect("validated output scale"),
                instructions,
                quantile_targets,
                coverage_lower_probability: finite(&output["coverage_interval"], "lower_probability").expect("validated lower coverage"),
                coverage_upper_probability: finite(&output["coverage_interval"], "upper_probability").expect("validated upper coverage"),
            }
        })
        .collect()
}

fn parse_simulation(case: &Value) -> SimulationSpec {
    let simulation = &case["simulation"];
    let convergence = &simulation["convergence"];
    let convergence_by_output = convergence["outputs"]
        .as_array()
        .expect("validated convergence outputs")
        .iter()
        .map(|target| {
            (
                text(target, "output_id").expect("validated convergence output").to_owned(),
                ConvergenceTarget {
                    maximum_mean_drift: finite(target, "maximum_mean_drift").expect("validated mean drift"),
                    maximum_standard_deviation_drift: finite(target, "maximum_standard_deviation_drift").expect("validated sd drift"),
                    maximum_quantile_drift: finite(target, "maximum_quantile_drift").expect("validated quantile drift"),
                },
            )
        })
        .collect();
    SimulationSpec {
        seed: uint(simulation, "seed").expect("validated seed"),
        sample_count: uint(simulation, "sample_count").expect("validated sample count") as usize,
        batch_size: uint(simulation, "batch_size").expect("validated batch size") as usize,
        maximum_invalid_fraction: finite(simulation, "maximum_invalid_fraction").expect("validated invalid fraction"),
        quantile_estimation_confidence: finite(simulation, "quantile_estimation_confidence").expect("validated MC confidence"),
        minimum_batch_count: uint(convergence, "minimum_batch_count").expect("validated min batch") as usize,
        stable_tail_batch_count: uint(convergence, "stable_tail_batch_count").expect("validated tail count") as usize,
        convergence_by_output,
    }
}

fn cholesky_validated(case: &Value) -> Vec<Vec<f64>> {
    let matrix: Vec<Vec<f64>> = case["correlation_matrix"]
        .as_array()
        .expect("validated correlation matrix")
        .iter()
        .map(|row| row.as_array().expect("validated row").iter().map(|value| value.as_f64().expect("validated coefficient")).collect())
        .collect();
    cholesky(&matrix).expect("validated positive definite matrix")
}

fn evaluate_instructions(instructions: &[Instruction], inputs_si: &[f64]) -> Result<f64, &'static str> {
    let mut stack: Vec<f64> = Vec::with_capacity(instructions.len());
    for instruction in instructions {
        match instruction {
            Instruction::Input(index) => {
                let value = *inputs_si.get(*index).ok_or("INPUT_INDEX_OUT_OF_RANGE")?;
                stack.push(value);
            }
            Instruction::Constant { si_value } => stack.push(*si_value),
            Instruction::Add => {
                let b = stack.pop().ok_or("STACK_UNDERFLOW")?;
                let a = stack.pop().ok_or("STACK_UNDERFLOW")?;
                stack.push(a + b);
            }
            Instruction::Subtract => {
                let b = stack.pop().ok_or("STACK_UNDERFLOW")?;
                let a = stack.pop().ok_or("STACK_UNDERFLOW")?;
                stack.push(a - b);
            }
            Instruction::Multiply => {
                let b = stack.pop().ok_or("STACK_UNDERFLOW")?;
                let a = stack.pop().ok_or("STACK_UNDERFLOW")?;
                stack.push(a * b);
            }
            Instruction::Divide => {
                let b = stack.pop().ok_or("STACK_UNDERFLOW")?;
                let a = stack.pop().ok_or("STACK_UNDERFLOW")?;
                if b == 0.0 {
                    return Err("DIVIDE_BY_ZERO");
                }
                stack.push(a / b);
            }
            Instruction::Sqrt => {
                let value = stack.pop().ok_or("STACK_UNDERFLOW")?;
                if value < 0.0 {
                    return Err("SQRT_NEGATIVE_DOMAIN");
                }
                stack.push(value.sqrt());
            }
            Instruction::Log => {
                let value = stack.pop().ok_or("STACK_UNDERFLOW")?;
                if value <= 0.0 {
                    return Err("LOG_NONPOSITIVE_DOMAIN");
                }
                stack.push(value.ln());
            }
            Instruction::Exp => {
                let value = stack.pop().ok_or("STACK_UNDERFLOW")?;
                stack.push(value.exp());
            }
            Instruction::Abs => {
                let value = stack.pop().ok_or("STACK_UNDERFLOW")?;
                stack.push(value.abs());
            }
        }
    }
    if stack.len() != 1 {
        return Err("INVALID_FINAL_STACK_DEPTH");
    }
    let value = stack[0];
    if !value.is_finite() {
        return Err("NONFINITE_EQUATION_RESULT");
    }
    Ok(value)
}

fn quantile_sorted(sorted: &[f64], probability: f64) -> f64 {
    let n = sorted.len();
    let position = (n - 1) as f64 * probability;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    if lower_index == upper_index {
        sorted[lower_index]
    } else {
        let fraction = position - lower_index as f64;
        sorted[lower_index] * (1.0 - fraction) + sorted[upper_index] * fraction
    }
}

fn summary_from_values(values: &[f64], probabilities: &[f64]) -> Option<BatchSummary> {
    if values.len() < 2 {
        return None;
    }
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let variance = values.iter().map(|value| (value - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let quantiles = probabilities
        .iter()
        .map(|probability| quantile_sorted(&sorted, *probability))
        .collect();
    Some(BatchSummary {
        mean,
        standard_deviation: variance.max(0.0).sqrt(),
        quantiles,
    })
}

fn binomial_prefix(n: usize, probability: f64) -> Vec<f64> {
    let mut log_factorials = vec![0.0_f64; n + 1];
    for index in 1..=n {
        log_factorials[index] = log_factorials[index - 1] + (index as f64).ln();
    }
    let log_p = probability.ln();
    let log_q = (-probability).ln_1p();
    let mut mass = vec![0.0_f64; n + 1];
    for k in 0..=n {
        let log_mass = log_factorials[n] - log_factorials[k] - log_factorials[n - k]
            + k as f64 * log_p + (n - k) as f64 * log_q;
        mass[k] = log_mass.exp();
    }
    let normalizer: f64 = mass.iter().sum();
    if normalizer > 0.0 && normalizer.is_finite() {
        for value in &mut mass {
            *value /= normalizer;
        }
    }
    let mut prefix = vec![0.0_f64; n + 2];
    for k in 0..=n {
        prefix[k + 1] = prefix[k] + mass[k];
    }
    prefix
}

fn quantile_monte_carlo_interval(
    sorted: &[f64],
    probability: f64,
    confidence: f64,
) -> Option<(f64, f64, f64)> {
    let n = sorted.len();
    if n < 2 || !(0.0..1.0).contains(&probability) {
        return None;
    }
    let prefix = binomial_prefix(n, probability);
    let tail = (1.0 - confidence) / 2.0;
    let lower_rank = (1..=n).rev().find(|rank| prefix[*rank] <= tail)?;
    let upper_rank = (2..=n).find(|rank| prefix[*rank] >= 1.0 - tail)?;
    if lower_rank >= upper_rank {
        return None;
    }
    let achieved = (prefix[upper_rank] - prefix[lower_rank]).clamp(0.0, 1.0);
    Some((
        sorted[lower_rank - 1],
        sorted[upper_rank - 1],
        achieved,
    ))
}

fn output_quantiles(
    sorted: &[f64],
    targets: &[QuantileTarget],
    confidence: f64,
) -> Vec<QuantileEstimate> {
    targets
        .iter()
        .map(|target| {
            let estimate = quantile_sorted(sorted, target.probability);
            match quantile_monte_carlo_interval(sorted, target.probability, confidence) {
                Some((lower, upper, achieved)) => {
                    let width = upper - lower;
                    let passes = achieved + VALIDATION_TOLERANCE >= confidence
                        && width <= target.maximum_mc_interval_width + VALIDATION_TOLERANCE;
                    QuantileEstimate {
                        probability: target.probability,
                        estimate,
                        monte_carlo_interval_lower: Some(lower),
                        monte_carlo_interval_upper: Some(upper),
                        achieved_confidence: Some(achieved),
                        monte_carlo_interval_width: Some(width),
                        maximum_interval_width: target.maximum_mc_interval_width,
                        disposition: if passes {
                            "PASS_MC_QUANTILE_PRECISION"
                        } else {
                            "FAIL_MC_QUANTILE_PRECISION"
                        }
                        .to_owned(),
                    }
                }
                None => QuantileEstimate {
                    probability: target.probability,
                    estimate,
                    monte_carlo_interval_lower: None,
                    monte_carlo_interval_upper: None,
                    achieved_confidence: None,
                    monte_carlo_interval_width: None,
                    maximum_interval_width: target.maximum_mc_interval_width,
                    disposition: "INSUFFICIENT_MC_SAMPLES_FOR_QUANTILE_CI".to_owned(),
                },
            }
        })
        .collect()
}

fn generate_correlated_inputs(
    generator: &mut NormalGenerator,
    inputs: &[InputSpec],
    cholesky_lower: &[Vec<f64>],
) -> Vec<f64> {
    let independent: Vec<f64> = (0..inputs.len()).map(|_| generator.next()).collect();
    let mut correlated = vec![0.0_f64; inputs.len()];
    for row in 0..inputs.len() {
        for column in 0..=row {
            correlated[row] += cholesky_lower[row][column] * independent[column];
        }
    }
    inputs
        .iter()
        .zip(correlated.iter())
        .map(|(input, latent)| input.marginal.sample(*latent) * input.scale_to_si)
        .collect()
}

fn convergence_for_output(
    state: &mut OutputConvergenceState,
    current: &BatchSummary,
    target: &ConvergenceTarget,
    batch_index: usize,
    required_tail: usize,
    batch_count: usize,
) -> ConvergenceReport {
    if let Some(previous) = &state.previous {
        let mean_drift = (current.mean - previous.mean).abs();
        let standard_deviation_drift =
            (current.standard_deviation - previous.standard_deviation).abs();
        let quantile_drift = current
            .quantiles
            .iter()
            .zip(previous.quantiles.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max);

        state.maximum_mean_drift = state.maximum_mean_drift.max(mean_drift);
        state.maximum_standard_deviation_drift =
            state.maximum_standard_deviation_drift.max(standard_deviation_drift);
        state.maximum_quantile_drift = state.maximum_quantile_drift.max(quantile_drift);

        if mean_drift > target.maximum_mean_drift
            || standard_deviation_drift > target.maximum_standard_deviation_drift
            || quantile_drift > target.maximum_quantile_drift
        {
            state.last_instability_batch = Some(batch_index);
        }
    }
    state.previous = Some(current.clone());

    let terminal_stable = state
        .last_instability_batch
        .map(|last| batch_count - 1 - last)
        .unwrap_or(batch_count.saturating_sub(1));
    let passes = batch_index + 1 == batch_count && terminal_stable >= required_tail;

    ConvergenceReport {
        batch_count,
        required_stable_tail_transitions: required_tail,
        last_instability_batch_index: state.last_instability_batch,
        terminal_stable_transitions: terminal_stable,
        maximum_mean_drift: state.maximum_mean_drift,
        maximum_standard_deviation_drift: state.maximum_standard_deviation_drift,
        maximum_quantile_drift: state.maximum_quantile_drift,
        disposition: if passes {
            "PASS_TERMINAL_NUMERICAL_STABILITY"
        } else if batch_index + 1 == batch_count {
            "FAIL_TERMINAL_NUMERICAL_STABILITY"
        } else {
            "PENDING"
        }
        .to_owned(),
    }
}

/// Validate and evaluate a correlated Monte Carlo measurement case.
pub fn evaluate_case(case: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case);
    if !errors.is_empty() {
        return Err(errors);
    }

    let inputs = parse_inputs(case);
    let outputs = parse_outputs(case, &inputs);
    let simulation = parse_simulation(case);
    let cholesky_lower = cholesky_validated(case);
    let batch_count = simulation.sample_count / simulation.batch_size;
    let mut output_values: Vec<Vec<f64>> = outputs
        .iter()
        .map(|_| Vec::with_capacity(simulation.sample_count))
        .collect();
    let mut output_invalid = vec![0_usize; outputs.len()];
    let mut convergence_states: Vec<OutputConvergenceState> = outputs
        .iter()
        .map(|_| OutputConvergenceState {
            previous: None,
            last_instability_batch: None,
            maximum_mean_drift: 0.0,
            maximum_standard_deviation_drift: 0.0,
            maximum_quantile_drift: 0.0,
        })
        .collect();
    let convergence_targets = outputs
        .iter()
        .map(|output| simulation.convergence_by_output[&output.id].clone())
        .collect::<Vec<_>>();
    let mut last_convergence = vec![None; outputs.len()];
    let mut joint_moments = RunningJointMoments::new(outputs.len());
    let mut generator = NormalGenerator::new(simulation.seed);
    let input_indices: BTreeMap<String, usize> = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| (input.id.clone(), index))
        .collect();

    for sample_index in 0..simulation.sample_count {
        let inputs_si = generate_correlated_inputs(&mut generator, &inputs, &cholesky_lower);
        let mut row_outputs = vec![0.0_f64; outputs.len()];
        let mut row_complete = true;

        for (output_index, output) in outputs.iter().enumerate() {
            match evaluate_instructions(&output.instructions, &inputs_si) {
                Ok(value_si) => {
                    let converted = value_si * output.scale_from_si;
                    if converted.is_finite() {
                        output_values[output_index].push(converted);
                        row_outputs[output_index] = converted;
                    } else {
                        output_invalid[output_index] += 1;
                        row_complete = false;
                    }
                }
                Err(_) => {
                    output_invalid[output_index] += 1;
                    row_complete = false;
                }
            }
        }
        if row_complete {
            joint_moments.update(&row_outputs);
        }

        let batch_end = (sample_index + 1) % simulation.batch_size == 0;
        if batch_end {
            let batch_index = sample_index / simulation.batch_size;
            for output_index in 0..outputs.len() {
                if let Some(summary) = summary_from_values(
                    &output_values[output_index],
                    &outputs[output_index]
                        .quantile_targets
                        .iter()
                        .map(|target| target.probability)
                        .collect::<Vec<_>>(),
                ) {
                    let convergence = convergence_for_output(
                        &mut convergence_states[output_index],
                        &summary,
                        &convergence_targets[output_index],
                        batch_index,
                        simulation.stable_tail_batch_count,
                        batch_count,
                    );
                    if batch_index + 1 == batch_count {
                        last_convergence[output_index] = Some(convergence);
                    }
                }
            }
        }
    }

    let mut reports = Vec::with_capacity(outputs.len());
    for (output_index, output) in outputs.iter().enumerate() {
        let values = &output_values[output_index];
        let invalid = output_invalid[output_index];
        let invalid_fraction = invalid as f64 / simulation.sample_count as f64;
        let mut sorted = values.clone();
        sorted.sort_by(f64::total_cmp);
        let mean = if sorted.is_empty() {
            None
        } else {
            Some(sorted.iter().sum::<f64>() / sorted.len() as f64)
        };
        let standard_deviation = if sorted.len() >= 2 {
            let m = mean.expect("mean available");
            let variance = sorted.iter().map(|value| (value - m).powi(2)).sum::<f64>()
                / (sorted.len() - 1) as f64;
            Some(variance.max(0.0).sqrt())
        } else {
            None
        };
        let standard_error = standard_deviation
            .zip(Some(sorted.len()))
            .filter(|(_, count)| *count > 0)
            .map(|(sd, count)| sd / (count as f64).sqrt());
        let quantiles = if sorted.is_empty() {
            Vec::new()
        } else {
            output_quantiles(
                &sorted,
                &output.quantile_targets,
                simulation.quantile_estimation_confidence,
            )
        };
        let coverage_lower = quantiles
            .iter()
            .find(|quantile| (quantile.probability - output.coverage_lower_probability).abs() < VALIDATION_TOLERANCE)
            .map(|quantile| quantile.estimate);
        let coverage_upper = quantiles
            .iter()
            .find(|quantile| (quantile.probability - output.coverage_upper_probability).abs() < VALIDATION_TOLERANCE)
            .map(|quantile| quantile.estimate);
        let convergence = last_convergence[output_index].clone().unwrap_or(ConvergenceReport {
            batch_count,
            required_stable_tail_transitions: simulation.stable_tail_batch_count,
            last_instability_batch_index: None,
            terminal_stable_transitions: 0,
            maximum_mean_drift: f64::INFINITY,
            maximum_standard_deviation_drift: f64::INFINITY,
            maximum_quantile_drift: f64::INFINITY,
            disposition: "FAIL_NO_BATCH_SUMMARY".to_owned(),
        });

        let quantile_precision_pass = !quantiles.is_empty()
            && quantiles.iter().all(|quantile| quantile.disposition == "PASS_MC_QUANTILE_PRECISION");
        let invalid_pass = invalid_fraction <= simulation.maximum_invalid_fraction + VALIDATION_TOLERANCE;
        let convergence_pass = convergence.disposition == "PASS_TERMINAL_NUMERICAL_STABILITY";
        let disposition = if !invalid_pass {
            "INVALID_SAMPLE_LIMIT_EXCEEDED"
        } else if !quantile_precision_pass {
            "FAIL_MC_QUANTILE_PRECISION"
        } else if !convergence_pass {
            "NOT_CONVERGED"
        } else {
            "PASS_FOR_DECLARED_MODEL"
        };

        reports.push(OutputReport {
            output_id: output.id.clone(),
            unit: output.unit.clone(),
            dimension: output.dimension,
            requested_samples: simulation.sample_count,
            valid_samples: values.len(),
            invalid_samples: invalid,
            invalid_fraction,
            mean,
            standard_deviation,
            standard_error_of_mean: standard_error,
            median: if sorted.is_empty() { None } else { Some(quantile_sorted(&sorted, 0.5)) },
            quantiles,
            coverage_lower_probability: output.coverage_lower_probability,
            coverage_lower_value: coverage_lower,
            coverage_upper_probability: output.coverage_upper_probability,
            coverage_upper_value: coverage_upper,
            convergence,
            disposition: disposition.to_owned(),
        });
    }

    let (covariance_matrix, correlation_matrix) = joint_moments.matrices();
    let joint_dependence = JointOutputDependence {
        output_ids: outputs.iter().map(|output| output.id.clone()).collect(),
        units: outputs.iter().map(|output| output.unit.clone()).collect(),
        joint_valid_sample_count: joint_moments.count,
        covariance_matrix,
        correlation_matrix,
    };

    let invalid_limit_failed = reports.iter().any(|report| report.disposition == "INVALID_SAMPLE_LIMIT_EXCEEDED");
    let quantile_failed = reports.iter().any(|report| report.disposition == "FAIL_MC_QUANTILE_PRECISION");
    let convergence_failed = reports.iter().any(|report| report.disposition == "NOT_CONVERGED" || report.disposition == "FAIL_NO_BATCH_SUMMARY");
    let evidence_synthetic = text(case, "evidence_class") != Some("MEASURED")
        || text(&case["dependence_model"], "evidence_role") == Some("SIMULATION_ONLY")
        || inputs.iter().any(|input| {
            let role = case["inputs"].as_array().unwrap()[input_indices[&input.id]]["evidence_role"]
                .as_str().unwrap_or("");
            role == "SIMULATION_ONLY"
        });

    let overall = if invalid_limit_failed {
        "INVALID_SAMPLE_LIMIT_EXCEEDED"
    } else if quantile_failed {
        "NOT_QUALIFIED_MC_QUANTILE_PRECISION"
    } else if convergence_failed {
        "NOT_CONVERGED"
    } else if evidence_synthetic {
        "SYNTHETIC_ONLY_NOT_DECISION_QUALIFIED"
    } else {
        "MEASURED_INPUTS_REVIEW_REQUIRED"
    };

    Ok(EvaluationReport {
        case_id: text(case, "case_id").expect("validated case ID").to_owned(),
        evidence_disposition: if evidence_synthetic {
            "SYNTHETIC_ONLY_NOT_PHYSICAL_QUALIFICATION"
        } else {
            "MEASURED_INPUTS_REVIEW_REQUIRED"
        }
        .to_owned(),
        dependence_model: text(&case["dependence_model"], "type").expect("validated dependence model").to_owned(),
        prng: text(&case["simulation"], "prng").expect("validated PRNG").to_owned(),
        seed: simulation.seed,
        requested_samples: simulation.sample_count,
        batch_size: simulation.batch_size,
        outputs: reports,
        joint_output_dependence: joint_dependence,
        overall_disposition: overall.to_owned(),
        claim_boundary: "OUTPUT_DISTRIBUTIONS_ARE_CONDITIONAL_ON_DECLARED_MARGINALS_GAUSSIAN_COPULA_CORRELATION_MATRIX_EQUATIONS_AND_PRNG;_NOT_FULL_GUM_OR_PHYSICAL_QUALIFICATION".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-measurement-uncertainty-case.v0_1.json"
        ))
        .expect("synthetic MCM fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn output<'a>(report: &'a EvaluationReport, id: &str) -> &'a OutputReport {
        report.outputs.iter().find(|item| item.output_id == id).unwrap()
    }

    fn first_input_mut(case: &mut Value) -> &mut Value {
        &mut case["inputs"][0]
    }

    #[test]
    fn synthetic_case_validates() {
        assert!(validate_case(&fixture()).is_empty());
    }

    #[test]
    fn deterministic_seed_replays_identically() {
        let case = fixture();
        let first = evaluate_case(&case).unwrap();
        let second = evaluate_case(&case).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn shared_input_draws_produce_joint_output_covariance() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(report.outputs.len(), 3);
        assert_eq!(report.joint_output_dependence.output_ids.len(), 3);
        assert_eq!(report.joint_output_dependence.joint_valid_sample_count, report.requested_samples);
        assert!(report.joint_output_dependence.covariance_matrix[0][1].is_some());
    }

    #[test]
    fn synthetic_outputs_are_never_decision_qualified() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY_NOT_PHYSICAL_QUALIFICATION");
        assert_eq!(report.overall_disposition, "SYNTHETIC_ONLY_NOT_DECISION_QUALIFIED");
    }

    #[test]
    fn synthetic_specific_energy_matches_independent_reference_replay() {
        let report = evaluate_case(&fixture()).unwrap();
        let specific = output(&report, "specific_energy");
        assert!((specific.mean.unwrap() - 3.0131656977).abs() < 0.002);
        assert!((specific.standard_deviation.unwrap() - 0.3011179330).abs() < 0.002);
        assert!((specific.quantiles[0].estimate - 2.54239175).abs() < 0.002);
        assert!((specific.median.unwrap() - 2.99894365).abs() < 0.002);
        assert!((specific.quantiles[2].estimate - 3.5305094).abs() < 0.002);
        assert!((specific.standard_error_of_mean.unwrap() - 0.000588121).abs() < 0.00005);
        assert_eq!(specific.disposition, "PASS_FOR_DECLARED_MODEL");
    }

    #[test]
    fn exact_monte_carlo_quantile_intervals_have_golden_coverage() {
        let report = evaluate_case(&fixture()).unwrap();
        let specific = output(&report, "specific_energy");
        let p05 = &specific.quantiles[0];
        assert!((p05.monte_carlo_interval_lower.unwrap() - 2.5396076).abs() < 0.002);
        assert!((p05.monte_carlo_interval_upper.unwrap() - 2.5450279).abs() < 0.002);
        assert!((p05.achieved_confidence.unwrap() - 0.9900186).abs() < 0.0001);
        assert_eq!(p05.disposition, "PASS_MC_QUANTILE_PRECISION");
    }

    #[test]
    fn shared_draws_preserve_joint_output_correlation() {
        let report = evaluate_case(&fixture()).unwrap();
        let covariance = &report.joint_output_dependence.covariance_matrix;
        let correlation = &report.joint_output_dependence.correlation_matrix;
        assert!((covariance[0][1].unwrap() - (-0.0179147067)).abs() < 0.001);
        assert!((correlation[0][1].unwrap() - (-0.89199436)).abs() < 0.01);
        assert_eq!(report.joint_output_dependence.joint_valid_sample_count, 262_144);
    }

    #[test]
    fn running_joint_moments_match_exact_covariance_golden() {
        let mut moments = RunningJointMoments::new(2);
        moments.update(&[1.0, 2.0]);
        moments.update(&[2.0, 4.0]);
        moments.update(&[3.0, 6.0]);
        let (covariance, correlation) = moments.matrices();
        assert_eq!(covariance[0][0], Some(1.0));
        assert_eq!(covariance[0][1], Some(2.0));
        assert_eq!(covariance[1][1], Some(4.0));
        assert!((correlation[0][1].unwrap() - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn dimensionless_log_expression_is_admitted() {
        let mut case = fixture();
        case["outputs"][0]["dimension"] = serde_json::json!([0,0,0,0,0,0,0]);
        case["outputs"][0]["unit"] = serde_json::json!("1");
        case["outputs"][0]["scale_from_si"] = serde_json::json!(1.0);
        case["outputs"][0]["instructions"] = serde_json::json!([
            {"op":"INPUT","input_id":"accepted_volume"},
            {"op":"INPUT","input_id":"generated_volume"},
            {"op":"DIVIDE"},
            {"op":"LOG"}
        ]);
        assert!(validate_case(&case).is_empty());
    }

    #[test]
    fn quantile_precision_gate_fails_at_too_small_monte_carlo_width() {
        let mut case = fixture();
        case["simulation"]["sample_count"] = serde_json::json!(1024);
        case["simulation"]["batch_size"] = serde_json::json!(128);
        for output in case["outputs"].as_array_mut().unwrap() {
            for target in output["quantile_targets"].as_array_mut().unwrap() {
                target["maximum_mc_interval_width"] = serde_json::json!(1.0e-10);
            }
        }
        let report = evaluate_case(&case).unwrap();
        assert_eq!(output(&report, "specific_energy").disposition, "FAIL_MC_QUANTILE_PRECISION");
        assert_eq!(report.overall_disposition, "NOT_QUALIFIED_MC_QUANTILE_PRECISION");
    }

    #[test]
    fn invalid_division_samples_are_counted_and_fail_closed() {
        let mut case = fixture();
        case["simulation"]["sample_count"] = serde_json::json!(1024);
        case["simulation"]["batch_size"] = serde_json::json!(128);
        case["outputs"][0]["dimension"] = serde_json::json!([0,0,0,0,0,0,0]);
        case["outputs"][0]["unit"] = serde_json::json!("1");
        case["outputs"][0]["scale_from_si"] = serde_json::json!(1.0);
        case["outputs"][0]["instructions"] = serde_json::json!([
            {"op":"INPUT","input_id":"accepted_volume"},
            {"op":"INPUT","input_id":"generated_volume"},
            {"op":"INPUT","input_id":"generated_volume"},
            {"op":"SUBTRACT"},
            {"op":"DIVIDE"}
        ]);
        let report = evaluate_case(&case).unwrap();
        let result = output(&report, "specific_energy");
        assert_eq!(result.invalid_samples, 1024);
        assert_eq!(result.valid_samples, 0);
        assert_eq!(result.mean, None);
        assert_eq!(result.disposition, "INVALID_SAMPLE_LIMIT_EXCEEDED");
        assert_eq!(report.overall_disposition, "INVALID_SAMPLE_LIMIT_EXCEEDED");
    }

    #[test]
    fn exponential_overflow_is_an_invalid_output_sample() {
        let instructions = vec![
            Instruction::Constant { si_value: 1000.0 },
            Instruction::Exp,
        ];
        assert_eq!(evaluate_instructions(&instructions, &[]), Err("NONFINITE_EQUATION_RESULT"));
    }

    #[test]
    fn dimension_addition_overflow_is_rejected() {
        let mut case = fixture();
        case["inputs"][0]["dimension"] = serde_json::json!([12,12,12,12,12,12,12]);
        case["outputs"][0]["dimension"] = serde_json::json!([24,24,24,24,24,24,24]);
        case["outputs"][0]["instructions"] = serde_json::json!([
            {"op":"INPUT","input_id":"total_energy"},
            {"op":"INPUT","input_id":"total_energy"},
            {"op":"MULTIPLY"},
            {"op":"INPUT","input_id":"total_energy"},
            {"op":"MULTIPLY"}
        ]);
        assert!(has(&validate_case(&case), "DIMENSION_EXPONENT_OVERFLOW"));
    }

    #[test]
    fn dimension_mismatch_on_addition_is_rejected_before_sampling() {
        let mut case = fixture();
        case["outputs"][0]["instructions"] = serde_json::json!([
            {"op":"INPUT","input_id":"total_energy"},
            {"op":"INPUT","input_id":"accepted_volume"},
            {"op":"ADD"}
        ]);
        assert!(has(&validate_case(&case), "DIMENSION_MISMATCH_ON_ADDITION"));
    }

    #[test]
    fn log_requires_dimensionless_input() {
        let mut case = fixture();
        case["outputs"][0]["instructions"] = serde_json::json!([
            {"op":"INPUT","input_id":"total_energy"},
            {"op":"LOG"}
        ]);
        assert!(has(&validate_case(&case), "DIMENSIONAL_LOG_OR_EXP"));
    }

    #[test]
    fn sqrt_requires_even_dimension_exponents() {
        let mut case = fixture();
        case["outputs"][0]["instructions"] = serde_json::json!([
            {"op":"INPUT","input_id":"accepted_volume"},
            {"op":"SQRT"}
        ]);
        assert!(has(&validate_case(&case), "SQRT_DIMENSION_NOT_EVEN"));
    }

    #[test]
    fn correlation_matrix_requires_symmetry() {
        let mut case = fixture();
        case["correlation_matrix"][0][1] = serde_json::json!(0.2);
        assert!(has(&validate_case(&case), "CORRELATION_MATRIX_NOT_SYMMETRIC"));
    }

    #[test]
    fn correlation_matrix_requires_unit_diagonal() {
        let mut case = fixture();
        case["correlation_matrix"][1][1] = serde_json::json!(0.9);
        assert!(has(&validate_case(&case), "CORRELATION_DIAGONAL_NOT_ONE"));
    }

    #[test]
    fn singular_correlation_matrix_is_rejected_without_jitter() {
        let mut case = fixture();
        case["correlation_matrix"] = serde_json::json!([
            [1.0,1.0,0.0,0.0],
            [1.0,1.0,0.0,0.0],
            [0.0,0.0,1.0,0.0],
            [0.0,0.0,0.0,1.0]
        ]);
        assert!(has(&validate_case(&case), "CORRELATION_MATRIX_NOT_POSITIVE_DEFINITE"));
    }

    #[test]
    fn correlation_coefficients_must_be_in_range() {
        let mut case = fixture();
        case["correlation_matrix"][0][1] = serde_json::json!(1.2);
        case["correlation_matrix"][1][0] = serde_json::json!(1.2);
        assert!(has(&validate_case(&case), "CORRELATION_OUT_OF_RANGE"));
    }

    #[test]
    fn sample_count_must_divide_into_fixed_batches() {
        let mut case = fixture();
        case["simulation"]["sample_count"] = serde_json::json!(262145);
        assert!(has(&validate_case(&case), "INVALID_BATCH_SIZE"));
    }

    #[test]
    fn convergence_targets_must_cover_every_output() {
        let mut case = fixture();
        case["simulation"]["convergence"]["outputs"].as_array_mut().unwrap().pop();
        assert!(has(&validate_case(&case), "CONVERGENCE_OUTPUT_SET_MISMATCH"));
    }

    #[test]
    fn quantile_targets_must_include_coverage_endpoints() {
        let mut case = fixture();
        case["outputs"][0]["quantile_targets"].as_array_mut().unwrap().pop();
        assert!(has(&validate_case(&case), "COVERAGE_ENDPOINT_NOT_A_QUANTILE_TARGET"));
    }

    #[test]
    fn invalid_correlation_evidence_role_is_rejected() {
        let mut case = fixture();
        case["dependence_model"]["evidence_role"] = serde_json::json!("UNKNOWN");
        assert!(has(&validate_case(&case), "INVALID_DEPENDENCE_EVIDENCE_ROLE"));
    }

    #[test]
    fn normal_distribution_requires_positive_standard_deviation() {
        let mut case = fixture();
        first_input_mut(&mut case)["distribution"]["standard_deviation"] = serde_json::json!(0.0);
        assert!(has(&validate_case(&case), "INVALID_INPUT_DISTRIBUTION"));
    }

    #[test]
    fn triangular_mode_must_be_inside_support() {
        let mut case = fixture();
        case["inputs"][2]["distribution"]["mode"] = serde_json::json!(13.0);
        assert!(has(&validate_case(&case), "INVALID_INPUT_DISTRIBUTION"));
    }

    #[test]
    fn invalid_division_samples_are_counted_not_zero_filled() {
        let mut case = fixture();
        case["inputs"][1]["distribution"] = serde_json::json!({
            "type":"NORMAL","mean":0.0,"standard_deviation":0.1
        });
        case["outputs"][0]["instructions"] = serde_json::json!([
            {"op":"INPUT","input_id":"total_energy"},
            {"op":"INPUT","input_id":"accepted_volume"},
            {"op":"DIVIDE"}
        ]);
        // Dimensional output remains unchanged; the generated Gaussian input
        // crosses zero only rarely, so a separate direct domain control covers
        // the invalid operation itself.
        let instructions = vec![Instruction::Constant { si_value: 1.0 }, Instruction::Constant { si_value: 0.0 }, Instruction::Divide];
        assert_eq!(evaluate_instructions(&instructions, &[]), Err("DIVIDE_BY_ZERO"));
        assert!(validate_case(&case).is_empty());
    }

    #[test]
    fn standard_normal_cdf_is_monotonic() {
        let points = [-4.0, -2.0, -1.0, 0.0, 1.0, 2.0, 4.0];
        let cdf: Vec<f64> = points.iter().map(|value| standard_normal_cdf(*value)).collect();
        assert!(cdf.windows(2).all(|pair| pair[0] < pair[1]));
        assert!((standard_normal_cdf(0.0) - 0.5).abs() < 1.0e-6);
    }

    #[test]
    fn cholesky_reconstructs_correlated_covariance() {
        let matrix = vec![vec![1.0,0.25],vec![0.25,1.0]];
        let lower = cholesky(&matrix).unwrap();
        let reconstructed_00 = lower[0][0] * lower[0][0];
        let reconstructed_10 = lower[1][0] * lower[0][0];
        let reconstructed_11 = lower[1][0] * lower[1][0] + lower[1][1] * lower[1][1];
        assert!((reconstructed_00 - 1.0).abs() < 1.0e-12);
        assert!((reconstructed_10 - 0.25).abs() < 1.0e-12);
        assert!((reconstructed_11 - 1.0).abs() < 1.0e-12);
    }
}
