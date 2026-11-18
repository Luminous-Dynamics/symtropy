//! Synthetic observation-process robustness for model-family adequacy.
//!
//! Covariance ordering is time-major, then sensor-major:
//! [t0/s0, t0/s1, ..., t1/s0, ...]. The dense covariance path is the
//! correctness reference. Structured covariance constructors are conveniences,
//! not permission to silently approximate an arbitrary covariance.
//!
//! The initial qualification lane covers Gaussian full space-time covariance,
//! including AR(1) temporal correlation. Unknown/non-Gaussian processes are not
//! assigned an iid threshold by this module.

use crate::experiment_design::{evaluate_input_profile, InputProfile};
use crate::robust_discrimination::{
    ModelFamilyAdequacyDisposition, ModelFamilyAdequacyPolicy, ModelFamilyAdequacyScore,
    ModelFamilyFitScore, ObservedSensorTrace, ProfiledDiscriminationError,
    ProfiledModelFamilyScenario,
};
use std::collections::HashMap;
use std::fmt;

const MAX_DENSE_DIMENSION: usize = 4096;
const MAX_MONTE_CARLO_TRIALS: usize = 1_000_000;
const SPLITMIX_INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;
const WILSON_Z_95: f64 = 1.959_963_984_540_054;
const CHI_SQUARE_DF4_95: f64 = 9.487_729_036_781_154;
const TIE_TOLERANCE: f64 = 1.0e-12;

/// Strict covariance-admission failures. No jitter, clipping, symmetrization,
/// diagonal fallback, or pseudoinverse is performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CovarianceError {
    Empty,
    DimensionTooLarge,
    NonSquare { row: usize, expected: usize, actual: usize },
    NonFiniteEntry { row: usize, column: usize },
    Asymmetric { row: usize, column: usize },
    NotPositiveDefinite { pivot: usize },
    InvalidCorrelation,
    InvalidTimeSchedule,
    InvalidCorrelationScale,
    DimensionMismatch { expected: usize, actual: usize },
    NonFiniteResidual { index: usize },
    NonFiniteResult,
    NoObservedValues,
    InvalidObservationIdentity,
    InvalidSensorOrder,
    TraceSensorOrderMismatch,
    TraceTimeScheduleMismatch,
}

impl fmt::Display for CovarianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "covariance matrix must be non-empty"),
            Self::DimensionTooLarge => write!(f, "dense covariance dimension exceeds the declared limit"),
            Self::NonSquare { row, expected, actual } => {
                write!(f, "covariance row {row} has {actual} entries; expected {expected}")
            }
            Self::NonFiniteEntry { row, column } => {
                write!(f, "covariance entry ({row}, {column}) must be finite")
            }
            Self::Asymmetric { row, column } => {
                write!(f, "covariance entries ({row}, {column}) and ({column}, {row}) are asymmetric")
            }
            Self::NotPositiveDefinite { pivot } => {
                write!(f, "covariance is not strictly positive definite at Cholesky pivot {pivot}")
            }
            Self::InvalidCorrelation => write!(f, "AR(1) correlation must be finite with absolute value below one"),
            Self::InvalidTimeSchedule => write!(f, "sample times must be finite and strictly increasing"),
            Self::InvalidCorrelationScale => write!(f, "correlation time scale must be finite and positive"),
            Self::DimensionMismatch { expected, actual } => {
                write!(f, "dimension mismatch: expected {expected}, received {actual}")
            }
            Self::NonFiniteResidual { index } => write!(f, "residual entry {index} must be finite"),
            Self::NonFiniteResult => write!(f, "covariance operation produced a non-finite result"),
            Self::NoObservedValues => write!(f, "at least one observed component is required"),
            Self::InvalidObservationIdentity => write!(f, "covariance, process and sampling policy IDs must be non-empty"),
            Self::InvalidSensorOrder => write!(f, "sensor IDs must be non-empty and unique"),
            Self::TraceSensorOrderMismatch => write!(f, "bound covariance sensor order does not match the observation trace"),
            Self::TraceTimeScheduleMismatch => write!(f, "bound covariance time schedule does not match the observation trace"),
        }
    }
}

impl std::error::Error for CovarianceError {}

/// A finite, symmetric, strictly positive-definite dense covariance and its
/// lower Cholesky factor. Inputs are retained exactly as admitted.
#[derive(Debug, Clone, PartialEq)]
pub struct DenseSpdCovariance {
    matrix: Vec<Vec<f64>>,
    cholesky_lower: Vec<Vec<f64>>,
}

impl DenseSpdCovariance {
    pub fn new(matrix: Vec<Vec<f64>>) -> Result<Self, CovarianceError> {
        let dimension = matrix.len();
        if dimension == 0 {
            return Err(CovarianceError::Empty);
        }
        if dimension > MAX_DENSE_DIMENSION {
            return Err(CovarianceError::DimensionTooLarge);
        }
        for (row_index, row) in matrix.iter().enumerate() {
            if row.len() != dimension {
                return Err(CovarianceError::NonSquare {
                    row: row_index,
                    expected: dimension,
                    actual: row.len(),
                });
            }
            for (column_index, value) in row.iter().enumerate() {
                if !value.is_finite() {
                    return Err(CovarianceError::NonFiniteEntry {
                        row: row_index,
                        column: column_index,
                    });
                }
            }
        }

        for row in 0..dimension {
            for column in (row + 1)..dimension {
                if matrix[row][column] != matrix[column][row] {
                    return Err(CovarianceError::Asymmetric { row, column });
                }
            }
        }

        let cholesky_lower = cholesky_lower(&matrix)?;
        Ok(Self {
            matrix,
            cholesky_lower,
        })
    }

    pub fn dimension(&self) -> usize {
        self.matrix.len()
    }

    pub fn matrix(&self) -> &[Vec<f64>] {
        &self.matrix
    }

    /// Evaluate 0.5 * r^T Sigma^-1 r by triangular whitening.
    /// This avoids explicitly forming the inverse covariance.
    pub fn gaussian_quadratic_form_half(
        &self,
        residual: &[f64],
    ) -> Result<f64, CovarianceError> {
        let dimension = self.dimension();
        if residual.len() != dimension {
            return Err(CovarianceError::DimensionMismatch {
                expected: dimension,
                actual: residual.len(),
            });
        }
        for (index, value) in residual.iter().enumerate() {
            if !value.is_finite() {
                return Err(CovarianceError::NonFiniteResidual { index });
            }
        }

        let mut whitened = vec![0.0; dimension];
        #[allow(clippy::needless_range_loop)]
        for row in 0..dimension {
            let mut value = residual[row];
            for column in 0..row {
                value -= self.cholesky_lower[row][column] * whitened[column];
            }
            whitened[row] = value / self.cholesky_lower[row][row];
        }
        let score = 0.5 * whitened.iter().map(|value| value * value).sum::<f64>();
        if !score.is_finite() {
            return Err(CovarianceError::NonFiniteResult);
        }
        Ok(score.max(0.0))
    }

    /// Draw a zero-mean Gaussian vector with this covariance using the
    /// deterministic non-cryptographic benchmark PRNG.
    pub fn sample_gaussian(&self, rng: &mut SplitMix64) -> Result<Vec<f64>, CovarianceError> {
        let dimension = self.dimension();
        let standard: Vec<f64> = (0..dimension).map(|_| rng.standard_normal()).collect();
        let mut sample = vec![0.0; dimension];
        #[allow(clippy::needless_range_loop)]
        for row in 0..dimension {
            sample[row] = self.cholesky_lower[row]
                .iter()
                .zip(standard.iter())
                .take(row + 1)
                .map(|(factor, value)| factor * value)
                .sum();
            if !sample[row].is_finite() {
                return Err(CovarianceError::NonFiniteResult);
            }
        }
        Ok(sample)
    }
}

/// Versioned binding between an admitted covariance matrix and the exact
/// observation process, sensor ordering and sample schedule that give its
/// dimensions meaning. IDs are caller-supplied registry/digest references;
/// this type does not cryptographically verify them.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationCovarianceV1 {
    pub covariance_id: String,
    pub observation_process_id: String,
    pub sampling_policy_id: String,
    pub sensor_ids: Vec<u32>,
    pub times: Vec<f64>,
    covariance: DenseSpdCovariance,
}

impl ObservationCovarianceV1 {
    pub fn new(
        covariance_id: String,
        observation_process_id: String,
        sampling_policy_id: String,
        sensor_ids: Vec<u32>,
        times: Vec<f64>,
        covariance: DenseSpdCovariance,
    ) -> Result<Self, CovarianceError> {
        if covariance_id.trim().is_empty()
            || observation_process_id.trim().is_empty()
            || sampling_policy_id.trim().is_empty()
        {
            return Err(CovarianceError::InvalidObservationIdentity);
        }
        if sensor_ids.is_empty() {
            return Err(CovarianceError::InvalidSensorOrder);
        }
        let mut seen = std::collections::HashSet::new();
        if sensor_ids.iter().any(|sensor_id| !seen.insert(*sensor_id)) {
            return Err(CovarianceError::InvalidSensorOrder);
        }
        if times.is_empty()
            || times.iter().any(|time| !time.is_finite())
            || times.windows(2).any(|pair| pair[1] <= pair[0])
        {
            return Err(CovarianceError::InvalidTimeSchedule);
        }
        let expected_dimension = times
            .len()
            .checked_mul(sensor_ids.len())
            .ok_or(CovarianceError::DimensionTooLarge)?;
        if covariance.dimension() != expected_dimension {
            return Err(CovarianceError::DimensionMismatch {
                expected: expected_dimension,
                actual: covariance.dimension(),
            });
        }
        Ok(Self {
            covariance_id,
            observation_process_id,
            sampling_policy_id,
            sensor_ids,
            times,
            covariance,
        })
    }

    pub fn covariance(&self) -> &DenseSpdCovariance {
        &self.covariance
    }

    fn validate_trace(&self, trace: &ObservedSensorTrace) -> Result<(), CovarianceError> {
        if self.sensor_ids != trace.sensor_ids {
            return Err(CovarianceError::TraceSensorOrderMismatch);
        }
        if self.times != trace.times {
            return Err(CovarianceError::TraceTimeScheduleMismatch);
        }
        Ok(())
    }
}

/// Result of a covariance-aware adequacy decision. The returned registry IDs,
/// schedule and sensor order make the admitted observation assumptions explicit
/// alongside the model-family disposition.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationProcessAdequacyReportV1 {
    pub covariance_id: String,
    pub observation_process_id: String,
    pub sampling_policy_id: String,
    pub sensor_ids: Vec<u32>,
    pub times: Vec<f64>,
    pub covariance_dimension: usize,
    pub adequacy: ModelFamilyAdequacyScore,
}

fn cholesky_lower(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, CovarianceError> {
    let dimension = matrix.len();
    let mut lower = vec![vec![0.0; dimension]; dimension];
    #[allow(clippy::needless_range_loop)]
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return Err(CovarianceError::NotPositiveDefinite { pivot: row });
                }
                lower[row][column] = value.sqrt();
            } else {
                let diagonal = lower[column][column];
                let entry = value / diagonal;
                if !entry.is_finite() {
                    return Err(CovarianceError::NotPositiveDefinite { pivot: row });
                }
                lower[row][column] = entry;
            }
        }
    }
    Ok(lower)
}

/// Build an AR(1) temporal correlation matrix on equally spaced observation
/// indices. rho is the correlation between adjacent observations.
pub fn ar1_time_covariance(
    sample_count: usize,
    rho: f64,
) -> Result<DenseSpdCovariance, CovarianceError> {
    if sample_count == 0 {
        return Err(CovarianceError::Empty);
    }
    if !rho.is_finite() || rho.abs() >= 1.0 {
        return Err(CovarianceError::InvalidCorrelation);
    }
    if sample_count > MAX_DENSE_DIMENSION {
        return Err(CovarianceError::DimensionTooLarge);
    }
    let matrix = (0..sample_count)
        .map(|row| {
            (0..sample_count)
                .map(|column| rho.powi(row.abs_diff(column) as i32))
                .collect()
        })
        .collect();
    DenseSpdCovariance::new(matrix)
}

/// Exponential temporal correlation for a strictly increasing, possibly
/// irregular time schedule: K_ij = exp(-|t_i-t_j| / tau).
pub fn exponential_time_covariance(
    times: &[f64],
    correlation_time: f64,
) -> Result<DenseSpdCovariance, CovarianceError> {
    if !correlation_time.is_finite() || correlation_time <= 0.0 {
        return Err(CovarianceError::InvalidCorrelationScale);
    }
    if times.is_empty() || times.len() > MAX_DENSE_DIMENSION {
        return Err(if times.is_empty() {
            CovarianceError::Empty
        } else {
            CovarianceError::DimensionTooLarge
        });
    }
    if times.iter().any(|time| !time.is_finite())
        || times.windows(2).any(|pair| pair[1] <= pair[0])
    {
        return Err(CovarianceError::InvalidTimeSchedule);
    }
    let matrix = times
        .iter()
        .map(|left| {
            times
                .iter()
                .map(|right| (-((left - right).abs() / correlation_time)).exp())
                .collect()
        })
        .collect();
    DenseSpdCovariance::new(matrix)
}

/// Construct the separable full space-time covariance K_time kron R_sensor.
/// Flattened vector order is time-major, then sensor-major.
pub fn kronecker_time_sensor_covariance(
    time_covariance: &DenseSpdCovariance,
    sensor_covariance: &DenseSpdCovariance,
) -> Result<DenseSpdCovariance, CovarianceError> {
    let time_count = time_covariance.dimension();
    let sensor_count = sensor_covariance.dimension();
    let dimension = time_count
        .checked_mul(sensor_count)
        .ok_or(CovarianceError::DimensionTooLarge)?;
    if dimension > MAX_DENSE_DIMENSION {
        return Err(CovarianceError::DimensionTooLarge);
    }
    let mut matrix = vec![vec![0.0; dimension]; dimension];
    #[allow(clippy::needless_range_loop)]
    for time_row in 0..time_count {
        for sensor_row in 0..sensor_count {
            for time_column in 0..time_count {
                for sensor_column in 0..sensor_count {
                    let row = time_row * sensor_count + sensor_row;
                    let column = time_column * sensor_count + sensor_column;
                    matrix[row][column] = time_covariance.matrix[time_row][time_column]
                        * sensor_covariance.matrix[sensor_row][sensor_column];
                }
            }
        }
    }
    DenseSpdCovariance::new(matrix)
}

/// Select only observed dimensions and the corresponding principal covariance
/// submatrix. Missing entries are removed; they are never zero-imputed.
pub fn select_observed_subspace(
    values: &[f64],
    observed_mask: &[bool],
    covariance: &DenseSpdCovariance,
) -> Result<(Vec<f64>, DenseSpdCovariance), CovarianceError> {
    if values.len() != covariance.dimension() {
        return Err(CovarianceError::DimensionMismatch {
            expected: covariance.dimension(),
            actual: values.len(),
        });
    }
    if observed_mask.len() != values.len() {
        return Err(CovarianceError::DimensionMismatch {
            expected: values.len(),
            actual: observed_mask.len(),
        });
    }
    let indices: Vec<usize> = observed_mask
        .iter()
        .enumerate()
        .filter_map(|(index, observed)| observed.then_some(index))
        .collect();
    if indices.is_empty() {
        return Err(CovarianceError::NoObservedValues);
    }
    let selected_values: Vec<f64> = indices.iter().map(|index| values[*index]).collect();
    let selected_matrix: Vec<Vec<f64>> = indices
        .iter()
        .map(|row| {
            indices
                .iter()
                .map(|column| covariance.matrix[*row][*column])
                .collect()
        })
        .collect();
    Ok((selected_values, DenseSpdCovariance::new(selected_matrix)?))
}

/// Full space-time covariance version of candidate-visible family adequacy.
/// The existing independent-time implementation remains the frozen iid
/// baseline; this function evaluates the same candidate means against the
/// supplied, explicitly admitted dense covariance.
pub fn evaluate_model_family_adequacy_full_covariance(
    scenario: &ProfiledModelFamilyScenario,
    profile: &InputProfile,
    selected_sensor_ids: &[u32],
    observations: &ObservedSensorTrace,
    covariance: &ObservationCovarianceV1,
    policy: ModelFamilyAdequacyPolicy,
) -> Result<ObservationProcessAdequacyReportV1, NoiseRobustnessError> {
    scenario
        .validate()
        .map_err(NoiseRobustnessError::Profiled)?;
    if !policy.maximum_gaussian_residual.is_finite()
        || policy.maximum_gaussian_residual < 0.0
    {
        return Err(NoiseRobustnessError::Profiled(
            ProfiledDiscriminationError::InvalidAdequacyThreshold,
        ));
    }

    let base_score = evaluate_input_profile(
        &scenario.experiment,
        profile,
        selected_sensor_ids,
    )
    .map_err(|error| {
        NoiseRobustnessError::Profiled(ProfiledDiscriminationError::Experiment(error))
    })?;

    if observations.sensor_ids != base_score.selected_sensor_ids {
        return Err(NoiseRobustnessError::Profiled(
            ProfiledDiscriminationError::TraceSensorOrderMismatch,
        ));
    }
    if observations.times.len() != scenario.experiment.observation_times.len()
        || observations
            .times
            .iter()
            .zip(scenario.experiment.observation_times.iter())
            .any(|(observed, expected)| {
                !observed.is_finite() || (observed - expected).abs() > TIE_TOLERANCE
            })
    {
        return Err(NoiseRobustnessError::Profiled(
            ProfiledDiscriminationError::TraceTimeScheduleMismatch,
        ));
    }
    if observations.values.len() != observations.times.len()
        || observations
            .values
            .iter()
            .any(|row| row.len() != observations.sensor_ids.len())
    {
        return Err(NoiseRobustnessError::Profiled(
            ProfiledDiscriminationError::TraceShapeMismatch,
        ));
    }
    if observations.values.iter().flatten().any(|value| !value.is_finite()) {
        return Err(NoiseRobustnessError::Profiled(
            ProfiledDiscriminationError::NonFiniteObservedValue,
        ));
    }

    covariance.validate_trace(observations)?;
    let expected_dimension = observations
        .times
        .len()
        .checked_mul(observations.sensor_ids.len())
        .ok_or(CovarianceError::DimensionTooLarge)?;
    if covariance.covariance().dimension() != expected_dimension {
        return Err(NoiseRobustnessError::Covariance(
            CovarianceError::DimensionMismatch {
                expected: expected_dimension,
                actual: covariance.covariance().dimension(),
            },
        ));
    }

    let observed: Vec<f64> = observations
        .values
        .iter()
        .flat_map(|row| row.iter().copied())
        .collect();
    let mut family_by_model: HashMap<u32, u32> = HashMap::new();
    for assignment in &scenario.family_assignments {
        family_by_model.insert(assignment.model_id, assignment.family_id);
    }

    let mut model_residuals: HashMap<u32, f64> = HashMap::new();
    for prediction in &base_score.model_observation_means {
        if prediction.observations.len() != observations.times.len()
            || prediction
                .observations
                .iter()
                .any(|row| row.len() != observations.sensor_ids.len())
        {
            return Err(NoiseRobustnessError::Profiled(
                ProfiledDiscriminationError::TraceShapeMismatch,
            ));
        }
        let predicted: Vec<f64> = prediction
            .observations
            .iter()
            .flat_map(|row| row.iter().copied())
            .collect();
        let residual: Vec<f64> = observed
            .iter()
            .zip(predicted.iter())
            .map(|(observed_value, predicted_value)| observed_value - predicted_value)
            .collect();
        let score = covariance.covariance().gaussian_quadratic_form_half(&residual)?;
        model_residuals.insert(prediction.model_id, score);
    }

    let mut ordered_families: Vec<u32> = family_by_model.values().copied().collect();
    ordered_families.sort_unstable();
    ordered_families.dedup();
    let mut family_fits = Vec::with_capacity(ordered_families.len());

    for family_id in ordered_families {
        let mut variants: Vec<(u32, f64)> = family_by_model
            .iter()
            .filter(|(_, candidate_family)| **candidate_family == family_id)
            .filter_map(|(model_id, _)| {
                model_residuals
                    .get(model_id)
                    .copied()
                    .map(|residual| (*model_id, residual))
            })
            .collect();
        variants.sort_by_key(|(model_id, _)| *model_id);
        if variants.is_empty() {
            return Err(NoiseRobustnessError::Profiled(
                ProfiledDiscriminationError::MissingFamilyVariant(family_id),
            ));
        }

        let minimum_gaussian_residual = variants
            .iter()
            .map(|(_, residual)| *residual)
            .fold(f64::INFINITY, f64::min);
        let best_fit_model_id = variants
            .iter()
            .find(|(_, residual)| {
                (*residual - minimum_gaussian_residual).abs() <= TIE_TOLERANCE
            })
            .map(|(model_id, _)| *model_id)
            .ok_or(NoiseRobustnessError::Profiled(
                ProfiledDiscriminationError::MissingFamilyAssignments,
            ))?;
        let compatible_model_ids = variants
            .iter()
            .filter(|(_, residual)| {
                *residual <= policy.maximum_gaussian_residual + TIE_TOLERANCE
            })
            .map(|(model_id, _)| *model_id)
            .collect();
        family_fits.push(ModelFamilyFitScore {
            family_id,
            best_fit_model_id,
            minimum_gaussian_residual,
            compatible_model_ids,
        });
    }

    let compatible_families: Vec<&ModelFamilyFitScore> = family_fits
        .iter()
        .filter(|family| !family.compatible_model_ids.is_empty())
        .collect();
    let disposition = match compatible_families.as_slice() {
        [] => ModelFamilyAdequacyDisposition::OutOfModelSet,
        [family] if family.compatible_model_ids.len() == 1 => {
            ModelFamilyAdequacyDisposition::InFamilyPreferred {
                family_id: family.family_id,
                model_id: family.compatible_model_ids[0],
            }
        }
        [family] => ModelFamilyAdequacyDisposition::ParameterNotIdentified {
            family_id: family.family_id,
            compatible_model_ids: family.compatible_model_ids.clone(),
        },
        families => ModelFamilyAdequacyDisposition::InFamilyUnresolved {
            compatible_family_ids: families.iter().map(|family| family.family_id).collect(),
        },
    };

    Ok(ObservationProcessAdequacyReportV1 {
        covariance_id: covariance.covariance_id.clone(),
        observation_process_id: covariance.observation_process_id.clone(),
        sampling_policy_id: covariance.sampling_policy_id.clone(),
        sensor_ids: covariance.sensor_ids.clone(),
        times: covariance.times.clone(),
        covariance_dimension: covariance.covariance().dimension(),
        adequacy: ModelFamilyAdequacyScore {
            profile_id: base_score.profile_id,
            maximum_gaussian_residual: policy.maximum_gaussian_residual,
            family_fits,
            disposition,
        },
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseRobustnessError {
    Covariance(CovarianceError),
    Profiled(ProfiledDiscriminationError),
    InvalidMonteCarloCount,
}

impl From<CovarianceError> for NoiseRobustnessError {
    fn from(error: CovarianceError) -> Self {
        Self::Covariance(error)
    }
}

impl fmt::Display for NoiseRobustnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Covariance(error) => write!(f, "covariance admission/scoring failed: {error}"),
            Self::Profiled(error) => write!(f, "profiled adequacy evaluation failed: {error}"),
            Self::InvalidMonteCarloCount => write!(f, "Monte Carlo trial count is outside the admitted range"),
        }
    }
}

impl std::error::Error for NoiseRobustnessError {}

/// Deterministic non-cryptographic PRNG for synthetic benchmark fixtures only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(SPLITMIX_INCREMENT);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn uniform_open(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64 + 0.5) / 9_007_199_254_740_992.0
    }

    fn standard_normal(&mut self) -> f64 {
        let radius = (-2.0 * self.uniform_open().ln()).sqrt();
        let angle = std::f64::consts::TAU * self.uniform_open();
        radius * angle.cos()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ar1NegativeControlReport {
    pub trials: usize,
    pub seed: u64,
    pub rho: f64,
    pub nominal_iid_threshold: f64,
    pub iid_assumption_false_rejection_count: usize,
    pub iid_assumption_false_rejection_rate: f64,
    pub iid_assumption_false_rejection_wilson_95: [f64; 2],
    pub full_covariance_false_rejection_count: usize,
    pub full_covariance_false_rejection_rate: f64,
    pub full_covariance_false_rejection_wilson_95: [f64; 2],
}

/// Compare the frozen iid chi-square(4) threshold with a full-covariance
/// Mahalanobis score on unit-variance AR(1) residuals. The iid lane is an
/// explicit negative control; the full-covariance lane whitens the same data.
pub fn run_ar1_negative_control(
    trials: usize,
    seed: u64,
    rho: f64,
) -> Result<Ar1NegativeControlReport, NoiseRobustnessError> {
    if trials == 0 || trials > MAX_MONTE_CARLO_TRIALS {
        return Err(NoiseRobustnessError::InvalidMonteCarloCount);
    }
    let covariance = ar1_time_covariance(4, rho)?;
    let threshold = 0.5 * CHI_SQUARE_DF4_95;
    let mut rng = SplitMix64::new(seed);
    let mut iid_rejections = 0_usize;
    let mut full_covariance_rejections = 0_usize;

    for _ in 0..trials {
        let residual = covariance.sample_gaussian(&mut rng)?;
        let iid_score = 0.5 * residual.iter().map(|value| value * value).sum::<f64>();
        let full_covariance_score = covariance.gaussian_quadratic_form_half(&residual)?;
        if iid_score > threshold {
            iid_rejections += 1;
        }
        if full_covariance_score > threshold {
            full_covariance_rejections += 1;
        }
    }

    Ok(Ar1NegativeControlReport {
        trials,
        seed,
        rho,
        nominal_iid_threshold: threshold,
        iid_assumption_false_rejection_count: iid_rejections,
        iid_assumption_false_rejection_rate: rate(iid_rejections, trials),
        iid_assumption_false_rejection_wilson_95: wilson_interval(iid_rejections, trials),
        full_covariance_false_rejection_count: full_covariance_rejections,
        full_covariance_false_rejection_rate: rate(full_covariance_rejections, trials),
        full_covariance_false_rejection_wilson_95: wilson_interval(
            full_covariance_rejections,
            trials,
        ),
    })
}

fn rate(count: usize, total: usize) -> f64 {
    count as f64 / total as f64
}

fn wilson_interval(successes: usize, total: usize) -> [f64; 2] {
    let n = total as f64;
    let p = successes as f64 / n;
    let z_squared = WILSON_Z_95 * WILSON_Z_95;
    let denominator = 1.0 + z_squared / n;
    let centre = (p + z_squared / (2.0 * n)) / denominator;
    let half_width = WILSON_Z_95
        * ((p * (1.0 - p) / n + z_squared / (4.0 * n * n)).sqrt())
        / denominator;
    [
        (centre - half_width).max(0.0),
        (centre + half_width).min(1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ar1_covariance_has_declared_lag_structure() {
        let covariance = ar1_time_covariance(4, 0.8).unwrap();
        let expected = [
            [1.0, 0.8, 0.64, 0.512],
            [0.8, 1.0, 0.8, 0.64],
            [0.64, 0.8, 1.0, 0.8],
            [0.512, 0.64, 0.8, 1.0],
        ];
        for (row_index, row) in expected.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                assert!((covariance.matrix()[row_index][column_index] - value).abs() < 1.0e-14);
            }
        }
    }

    #[test]
    fn mahalanobis_score_matches_closed_form_for_two_dimensional_covariance() {
        let rho = 0.8_f64;
        let covariance = DenseSpdCovariance::new(vec![
            vec![1.0, rho],
            vec![rho, 1.0],
        ])
        .unwrap();
        let residual = [1.0, 0.2];
        let expected = 0.5 * (1.0 - 2.0 * rho * 0.2 + 0.2_f64.powi(2)) / (1.0 - rho.powi(2));
        let actual = covariance.gaussian_quadratic_form_half(&residual).unwrap();
        assert!((actual - expected).abs() < 1.0e-12);
    }

    #[test]
    fn asymmetric_singular_and_non_finite_covariances_fail_closed() {
        assert!(matches!(
            DenseSpdCovariance::new(vec![vec![1.0, 0.2], vec![0.1, 1.0]]),
            Err(CovarianceError::Asymmetric { .. })
        ));
        assert!(matches!(
            DenseSpdCovariance::new(vec![vec![1.0, 1.0], vec![1.0, 1.0]]),
            Err(CovarianceError::NotPositiveDefinite { .. })
        ));
        assert!(matches!(
            DenseSpdCovariance::new(vec![vec![f64::NAN]]),
            Err(CovarianceError::NonFiniteEntry { .. })
        ));
    }

    #[test]
    fn kronecker_layout_is_time_major_then_sensor_major() {
        let time = DenseSpdCovariance::new(vec![
            vec![1.0, 0.5],
            vec![0.5, 1.0],
        ])
        .unwrap();
        let sensor = DenseSpdCovariance::new(vec![
            vec![1.0, 0.2],
            vec![0.2, 1.0],
        ])
        .unwrap();
        let joint = kronecker_time_sensor_covariance(&time, &sensor).unwrap();
        assert_eq!(joint.dimension(), 4);
        assert!((joint.matrix()[0][3] - 0.1).abs() < 1.0e-14);
        assert!((joint.matrix()[1][2] - 0.1).abs() < 1.0e-14);
    }

    #[test]
    fn missing_observations_select_principal_submatrix_without_zero_filling() {
        let covariance = DenseSpdCovariance::new(vec![
            vec![1.0, 0.1, 0.2],
            vec![0.1, 1.0, 0.3],
            vec![0.2, 0.3, 1.0],
        ])
        .unwrap();
        let (values, selected) =
            select_observed_subspace(&[4.0, 99.0, 6.0], &[true, false, true], &covariance)
                .unwrap();
        assert_eq!(values, vec![4.0, 6.0]);
        assert!((selected.matrix()[0][1] - 0.2).abs() < 1.0e-14);
    }

    #[test]
    fn irregular_exponential_time_covariance_is_admitted() {
        let covariance = exponential_time_covariance(&[0.0, 0.5, 2.0], 1.25).unwrap();
        assert_eq!(covariance.dimension(), 3);
        assert!((covariance.matrix()[0][2] - (-1.6_f64).exp()).abs() < 1.0e-14);
    }

    #[test]
    fn ar1_negative_control_separates_iid_miscalibration_from_full_covariance() {
        let report = run_ar1_negative_control(100_000, 20_261_118, 0.8).unwrap();
        assert!(report.iid_assumption_false_rejection_rate > 0.085);
        assert!(report.iid_assumption_false_rejection_rate < 0.115);
        assert!(report.full_covariance_false_rejection_rate > 0.04);
        assert!(report.full_covariance_false_rejection_rate < 0.06);
    }

    #[test]
    fn ar1_negative_control_is_deterministic_for_frozen_identity() {
        let first = run_ar1_negative_control(4_096, 77, 0.8).unwrap();
        let second = run_ar1_negative_control(4_096, 77, 0.8).unwrap();
        assert_eq!(first, second);
    }
}
