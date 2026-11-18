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

use crate::adequacy_calibration::HiddenOutOfFamilyTruth;
use crate::experiment_design::{evaluate_input_profile, InputProfile, ModelCandidate};
use crate::robust_discrimination::{
    ModelFamilyAdequacyDisposition, ModelFamilyAdequacyPolicy, ModelFamilyAdequacyScore,
    ModelFamilyFitScore, ObservedSensorTrace, ProfiledDiscriminationError,
    ProfiledModelFamilyScenario,
};
use std::collections::HashMap;
use std::fmt;

const MAX_DENSE_DIMENSION: usize = 4096;
const MAX_STRUCTURED_DIMENSION: usize = 1_000_000;
const MAX_MONTE_CARLO_TRIALS: usize = 1_000_000;
const MAX_TOTAL_NOISE_CALIBRATION_SAMPLES: usize = 100_000;
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
    NonFiniteObservedValue { index: usize },
    NonFiniteResult,
    NoObservedValues,
    InvalidObservationIdentity,
    InvalidSensorOrder,
    TraceSensorOrderMismatch,
    TraceTimeScheduleMismatch,
    TraceProcessIdentityMismatch,
    TraceSamplingPolicyMismatch,
    TraceOperatorMismatch,
    TraceAvailabilityCutoffMismatch,
}

impl fmt::Display for CovarianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "covariance matrix must be non-empty"),
            Self::DimensionTooLarge => {
                write!(f, "dense covariance dimension exceeds the declared limit")
            }
            Self::NonSquare { row, expected, actual } => {
                write!(f, "covariance row {row} has {actual} entries; expected {expected}")
            }
            Self::NonFiniteEntry { row, column } => {
                write!(f, "covariance entry ({row}, {column}) must be finite")
            }
            Self::Asymmetric { row, column } => {
                write!(
                    f,
                    "covariance entries ({row}, {column}) and ({column}, {row}) are asymmetric"
                )
            }
            Self::NotPositiveDefinite { pivot } => {
                write!(f, "covariance is not strictly positive definite at Cholesky pivot {pivot}")
            }
            Self::InvalidCorrelation => {
                write!(f, "AR(1) correlation must be finite with absolute value below one")
            }
            Self::InvalidTimeSchedule => {
                write!(f, "sample times must be finite and strictly increasing")
            }
            Self::InvalidCorrelationScale => {
                write!(f, "correlation time scale must be finite and positive")
            }
            Self::DimensionMismatch { expected, actual } => {
                write!(f, "dimension mismatch: expected {expected}, received {actual}")
            }
            Self::NonFiniteResidual { index } => {
                write!(f, "residual entry {index} must be finite")
            }
            Self::NonFiniteObservedValue { index } => {
                write!(f, "observed entry {index} must be finite")
            }
            Self::NonFiniteResult => {
                write!(f, "covariance operation produced a non-finite result")
            }
            Self::NoObservedValues => write!(f, "at least one observed component is required"),
            Self::InvalidObservationIdentity => write!(
                f,
                "covariance, process, sampling, operator and availability IDs must be non-empty"
            ),
            Self::InvalidSensorOrder => write!(f, "sensor IDs must be non-empty and unique"),
            Self::TraceSensorOrderMismatch => {
                write!(f, "bound covariance sensor order does not match the observation trace")
            }
            Self::TraceTimeScheduleMismatch => {
                write!(f, "bound covariance time schedule does not match the observation trace")
            }
            Self::TraceProcessIdentityMismatch => {
                write!(
                    f,
                    "bound covariance observation process does not match the observation trace"
                )
            }
            Self::TraceSamplingPolicyMismatch => {
                write!(f, "bound covariance sampling policy does not match the observation trace")
            }
            Self::TraceOperatorMismatch => {
                write!(
                    f,
                    "bound covariance observation operator does not match the observation trace"
                )
            }
            Self::TraceAvailabilityCutoffMismatch => {
                write!(
                    f,
                    "bound covariance availability cutoff does not match the observation trace"
                )
            }
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

/// Exact AR(1)-time x sensor covariance represented without materializing
/// the full (time * sensor) dense matrix.
///
/// For time-major residual blocks e_t, the Gaussian innovations are:
///   z_0 = e_0
///   z_t = (e_t - rho * e_(t-1)) / sqrt(1 - rho^2), t > 0
/// Each z_t is whitened by the sensor Cholesky factor. This gives the same
/// Mahalanobis score as K_AR1(rho) kron R_sensor, but costs O(T*m^2) rather
/// than a dense O((T*m)^3) Cholesky factorization.
#[derive(Debug, Clone, PartialEq)]
pub struct Ar1KroneckerCovariance {
    sample_count: usize,
    rho: f64,
    sensor_covariance: DenseSpdCovariance,
}

impl Ar1KroneckerCovariance {
    pub fn new(
        sample_count: usize,
        rho: f64,
        sensor_covariance: DenseSpdCovariance,
    ) -> Result<Self, CovarianceError> {
        if sample_count == 0 {
            return Err(CovarianceError::Empty);
        }
        if !rho.is_finite() || rho.abs() >= 1.0 || 1.0 - rho * rho <= 0.0 {
            return Err(CovarianceError::InvalidCorrelation);
        }
        let dimension = sample_count
            .checked_mul(sensor_covariance.dimension())
            .ok_or(CovarianceError::DimensionTooLarge)?;
        if dimension > MAX_STRUCTURED_DIMENSION {
            return Err(CovarianceError::DimensionTooLarge);
        }
        Ok(Self {
            sample_count,
            rho,
            sensor_covariance,
        })
    }

    pub fn sample_count(&self) -> usize {
        self.sample_count
    }

    pub fn sensor_count(&self) -> usize {
        self.sensor_covariance.dimension()
    }

    pub fn rho(&self) -> f64 {
        self.rho
    }

    pub fn dimension(&self) -> usize {
        self.sample_count * self.sensor_count()
    }

    /// Exact joint Gaussian quadratic form via AR(1) innovations.
    pub fn gaussian_quadratic_form_half(
        &self,
        residual: &[f64],
    ) -> Result<f64, CovarianceError> {
        let time_count = self.sample_count;
        let sensor_count = self.sensor_count();
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

        let mut whitened = vec![0.0; sensor_count];
        let mut score = 0.0;
        let temporal_innovation_variance = 1.0 - self.rho * self.rho;
        #[allow(clippy::needless_range_loop)]
        for time_index in 0..time_count {
            for sensor_row in 0..sensor_count {
                let flat_index = time_index * sensor_count + sensor_row;
                let mut value = residual[flat_index];
                if time_index > 0 {
                    value -= self.rho * residual[flat_index - sensor_count];
                }
                for sensor_column in 0..sensor_row {
                    value -= self.sensor_covariance.cholesky_lower[sensor_row][sensor_column]
                        * whitened[sensor_column];
                }
                whitened[sensor_row] =
                    value / self.sensor_covariance.cholesky_lower[sensor_row][sensor_row];
            }
            let block_norm = whitened.iter().map(|value| value * value).sum::<f64>();
            let innovation_variance = if time_index == 0 {
                1.0
            } else {
                temporal_innovation_variance
            };
            score += 0.5 * block_norm / innovation_variance;
        }
        if !score.is_finite() {
            return Err(CovarianceError::NonFiniteResult);
        }
        Ok(score.max(0.0))
    }

    /// Sample the exact separable Gaussian process using the AR(1) recurrence.
    pub fn sample_gaussian(
        &self,
        rng: &mut SplitMix64,
    ) -> Result<Vec<f64>, CovarianceError> {
        let time_count = self.sample_count;
        let sensor_count = self.sensor_count();
        let mut sample = vec![0.0; self.dimension()];
        let innovation_scale = (1.0 - self.rho * self.rho).sqrt();
        let mut standard = vec![0.0; sensor_count];
        let mut sensor_noise = vec![0.0; sensor_count];

        for time_index in 0..time_count {
            for value in &mut standard {
                *value = rng.standard_normal();
            }
            #[allow(clippy::needless_range_loop)]
            for sensor_row in 0..sensor_count {
                let mut value = 0.0;
                for sensor_column in 0..=sensor_row {
                    value += self.sensor_covariance.cholesky_lower[sensor_row][sensor_column]
                        * standard[sensor_column];
                }
                sensor_noise[sensor_row] = value;
            }
            for sensor_index in 0..sensor_count {
                let flat_index = time_index * sensor_count + sensor_index;
                sample[flat_index] = if time_index == 0 {
                    sensor_noise[sensor_index]
                } else {
                    self.rho * sample[flat_index - sensor_count]
                        + innovation_scale * sensor_noise[sensor_index]
                };
                if !sample[flat_index].is_finite() {
                    return Err(CovarianceError::NonFiniteResult);
                }
            }
        }
        Ok(sample)
    }

    /// Expand to a dense covariance for tiny-instance differential oracles.
    /// The full materialization retains the dense dimension safety limit.
    pub fn to_dense(&self) -> Result<DenseSpdCovariance, CovarianceError> {
        if self.dimension() > MAX_DENSE_DIMENSION {
            return Err(CovarianceError::DimensionTooLarge);
        }
        let time_covariance = ar1_time_covariance(self.sample_count, self.rho)?;
        kronecker_time_sensor_covariance(&time_covariance, &self.sensor_covariance)
    }
}

/// Covariance implementation selected for one observation-process identity.
/// Dense is the general correctness reference; structured AR(1) is an exact
/// matrix-free path for a declared separable process, not an approximation.
#[derive(Debug, Clone, PartialEq)]
pub enum ObservationCovarianceModelV1 {
    Dense(DenseSpdCovariance),
    Ar1Kronecker(Ar1KroneckerCovariance),
}

impl From<DenseSpdCovariance> for ObservationCovarianceModelV1 {
    fn from(covariance: DenseSpdCovariance) -> Self {
        Self::Dense(covariance)
    }
}

impl From<Ar1KroneckerCovariance> for ObservationCovarianceModelV1 {
    fn from(covariance: Ar1KroneckerCovariance) -> Self {
        Self::Ar1Kronecker(covariance)
    }
}

impl ObservationCovarianceModelV1 {
    pub fn dimension(&self) -> usize {
        match self {
            Self::Dense(covariance) => covariance.dimension(),
            Self::Ar1Kronecker(covariance) => covariance.dimension(),
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            Self::Dense(_) => "DENSE_SPD",
            Self::Ar1Kronecker(_) => "AR1_KRONECKER",
        }
    }

    pub fn gaussian_quadratic_form_half(
        &self,
        residual: &[f64],
    ) -> Result<f64, CovarianceError> {
        match self {
            Self::Dense(covariance) => covariance.gaussian_quadratic_form_half(residual),
            Self::Ar1Kronecker(covariance) => {
                covariance.gaussian_quadratic_form_half(residual)
            }
        }
    }

    pub fn sample_gaussian(
        &self,
        rng: &mut SplitMix64,
    ) -> Result<Vec<f64>, CovarianceError> {
        match self {
            Self::Dense(covariance) => covariance.sample_gaussian(rng),
            Self::Ar1Kronecker(covariance) => covariance.sample_gaussian(rng),
        }
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
    pub observation_operator_id: String,
    pub availability_cutoff_id: String,
    pub sensor_ids: Vec<u32>,
    pub times: Vec<f64>,
    covariance: ObservationCovarianceModelV1,
}

impl ObservationCovarianceV1 {
    pub fn new(
        covariance_id: String,
        observation_process_id: String,
        sampling_policy_id: String,
        observation_operator_id: String,
        availability_cutoff_id: String,
        sensor_ids: Vec<u32>,
        times: Vec<f64>,
        covariance: impl Into<ObservationCovarianceModelV1>,
    ) -> Result<Self, CovarianceError> {
        let covariance = covariance.into();
        if covariance_id.trim().is_empty()
            || observation_process_id.trim().is_empty()
            || sampling_policy_id.trim().is_empty()
            || observation_operator_id.trim().is_empty()
            || availability_cutoff_id.trim().is_empty()
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
            observation_operator_id,
            availability_cutoff_id,
            sensor_ids,
            times,
            covariance,
        })
    }

    pub fn covariance(&self) -> &ObservationCovarianceModelV1 {
        &self.covariance
    }

    fn validate_trace(&self, trace: &ObservationProcessTraceV1) -> Result<(), CovarianceError> {
        if self.observation_process_id != trace.observation_process_id {
            return Err(CovarianceError::TraceProcessIdentityMismatch);
        }
        if self.sampling_policy_id != trace.sampling_policy_id {
            return Err(CovarianceError::TraceSamplingPolicyMismatch);
        }
        if self.observation_operator_id != trace.observation_operator_id {
            return Err(CovarianceError::TraceOperatorMismatch);
        }
        if self.availability_cutoff_id != trace.availability_cutoff_id {
            return Err(CovarianceError::TraceAvailabilityCutoffMismatch);
        }
        if self.sensor_ids != trace.trace.sensor_ids {
            return Err(CovarianceError::TraceSensorOrderMismatch);
        }
        if self.times != trace.trace.times {
            return Err(CovarianceError::TraceTimeScheduleMismatch);
        }
        Ok(())
    }
}

/// Versioned candidate-visible observation trace metadata. These caller-supplied
/// references are compared to the covariance binding before any model scoring.
/// They are identifiers, not authenticated claims; an upstream evidence layer
/// must verify the referenced manifests and event receipts.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservationProcessTraceV1 {
    pub observation_process_id: String,
    pub sampling_policy_id: String,
    pub observation_operator_id: String,
    pub availability_cutoff_id: String,
    pub trace: ObservedSensorTrace,
}

impl ObservationProcessTraceV1 {
    pub fn new(
        observation_process_id: String,
        sampling_policy_id: String,
        observation_operator_id: String,
        availability_cutoff_id: String,
        trace: ObservedSensorTrace,
    ) -> Result<Self, CovarianceError> {
        if observation_process_id.trim().is_empty()
            || sampling_policy_id.trim().is_empty()
            || observation_operator_id.trim().is_empty()
            || availability_cutoff_id.trim().is_empty()
        {
            return Err(CovarianceError::InvalidObservationIdentity);
        }
        Ok(Self {
            observation_process_id,
            sampling_policy_id,
            observation_operator_id,
            availability_cutoff_id,
            trace,
        })
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
    pub observation_operator_id: String,
    pub availability_cutoff_id: String,
    pub sensor_ids: Vec<u32>,
    pub times: Vec<f64>,
    pub covariance_dimension: usize,
    pub covariance_kind: &'static str,
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
    for (selected_index, value) in selected_values.iter().enumerate() {
        if !value.is_finite() {
            return Err(CovarianceError::NonFiniteObservedValue {
                index: indices[selected_index],
            });
        }
    }
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
    observations: &ObservationProcessTraceV1,
    covariance: &ObservationCovarianceV1,
    policy: ModelFamilyAdequacyPolicy,
) -> Result<ObservationProcessAdequacyReportV1, NoiseRobustnessError> {
    scenario
        .validate()
        .map_err(NoiseRobustnessError::Profiled)?;
    covariance.validate_trace(observations)?;
    let observations = &observations.trace;
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
        observation_operator_id: covariance.observation_operator_id.clone(),
        availability_cutoff_id: covariance.availability_cutoff_id.clone(),
        sensor_ids: covariance.sensor_ids.clone(),
        times: covariance.times.clone(),
        covariance_dimension: covariance.covariance().dimension(),
        covariance_kind: covariance.covariance().kind_label(),
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
            Self::InvalidMonteCarloCount => {
                write!(f, "Monte Carlo trial count is outside the admitted range")
            }
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

/// Run the preregistered AR(1) correlation sweep used by the noise-robustness
/// corpus. Each rho receives a separate deterministic stream identity.
pub fn run_ar1_robustness_corpus(
    trials_per_rho: usize,
    seed: u64,
) -> Result<Vec<Ar1NegativeControlReport>, NoiseRobustnessError> {
    let mut reports = Vec::with_capacity(3);
    for (index, rho) in [0.2, 0.5, 0.8].into_iter().enumerate() {
        reports.push(run_ar1_negative_control(
            trials_per_rho,
            seed.wrapping_add(index as u64),
            rho,
        )?);
    }
    Ok(reports)
}


/// Full-covariance calibration report, bound to one observation process,
/// sampling policy, sensor operator, availability cutoff and time schedule.
#[derive(Debug, Clone, PartialEq)]
pub struct NoiseAdequacyCalibrationReportV1 {
    pub covariance_id: String,
    pub observation_process_id: String,
    pub sampling_policy_id: String,
    pub observation_operator_id: String,
    pub availability_cutoff_id: String,
    pub sensor_ids: Vec<u32>,
    pub times: Vec<f64>,
    pub covariance_dimension: usize,
    pub covariance_kind: &'static str,
    pub threshold: f64,
    pub target_in_family_coverage: f64,
    pub calibration_samples: usize,
    pub calibration_in_family_accepts: usize,
    pub calibration_in_family_coverage: f64,
    pub calibration_in_family_coverage_wilson_95: [f64; 2],
    pub validation_samples: usize,
    pub validation_in_family_accepts: usize,
    pub validation_in_family_coverage: f64,
    pub validation_in_family_coverage_wilson_95: [f64; 2],
    pub validation_false_out_of_model_set_count: usize,
    pub validation_false_out_of_model_set_rate: f64,
    pub validation_false_out_of_model_set_wilson_95: [f64; 2],
    pub validation_wrong_family_preference_count: usize,
    pub validation_wrong_family_preference_rate: f64,
    pub validation_ambiguous_count: usize,
    pub validation_ambiguous_rate: f64,
    pub out_of_family_samples: usize,
    pub out_of_family_rejections: usize,
    pub out_of_family_rejection_rate: f64,
    pub out_of_family_rejection_wilson_95: [f64; 2],
    pub out_of_family_false_accepts: usize,
    pub out_of_family_false_accept_rate: f64,
    pub out_of_family_false_accept_wilson_95: [f64; 2],
    pub calibration_seed: u64,
    pub validation_seed: u64,
    pub evaluator_seed: u64,
    pub assumptions: Vec<&'static str>,
}

/// Three explicitly disjoint deterministic streams for evaluator-side
/// covariance calibration. The PRNG is reproducibility-only, not cryptographic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoiseCalibrationConfigV1 {
    pub calibration_seed: u64,
    pub validation_seed: u64,
    pub evaluator_seed: u64,
    pub replicates_per_model: usize,
    pub target_in_family_coverage: f64,
}

/// Fail-closed errors for full space-time covariance calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoiseCalibrationError {
    Covariance(CovarianceError),
    Robustness(NoiseRobustnessError),
    Profiled(ProfiledDiscriminationError),
    InvalidReplicateCount,
    ExcessiveSampleCount,
    DuplicateHiddenTruthId(u32),
    InvalidTargetCoverage,
    SeedsMustDiffer,
    IncompatibleHiddenTruth { evaluator_id: u32 },
    MissingTrueFamily(u32),
    MissingModelPrediction(u32),
    MissingFamilyResidual(u32),
    MissingHiddenPrediction(u32),
    InvalidPredictionShape,
    NonFinitePrediction,
    NoAvailableHiddenModelId,
}

impl From<CovarianceError> for NoiseCalibrationError {
    fn from(error: CovarianceError) -> Self {
        Self::Covariance(error)
    }
}

impl From<NoiseRobustnessError> for NoiseCalibrationError {
    fn from(error: NoiseRobustnessError) -> Self {
        Self::Robustness(error)
    }
}

impl fmt::Display for NoiseCalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Covariance(error) => write!(f, "covariance calibration failed: {error}"),
            Self::Robustness(error) => write!(f, "noise robustness evaluation failed: {error}"),
            Self::Profiled(error) => write!(f, "profiled scenario invalid: {error}"),
            Self::InvalidReplicateCount => write!(f, "replicates per model must be positive"),
            Self::ExcessiveSampleCount => {
                write!(f, "full-covariance calibration exceeds the 100,000-sample campaign bound")
            }
            Self::DuplicateHiddenTruthId(id) => {
                write!(f, "duplicate hidden truth evaluator ID {id}")
            }
            Self::InvalidTargetCoverage => write!(
                f,
                "target in-family coverage must be finite and strictly between zero and one"
            ),
            Self::SeedsMustDiffer => write!(
                f,
                "calibration, validation and evaluator seeds must be pairwise distinct"
            ),
            Self::IncompatibleHiddenTruth { evaluator_id } => write!(
                f,
                "hidden truth {evaluator_id} does not share the declared state basis and horizon"
            ),
            Self::MissingTrueFamily(id) => {
                write!(f, "candidate model {id} has no declared structural family")
            }
            Self::MissingModelPrediction(id) => {
                write!(f, "candidate model {id} has no predicted observation trace")
            }
            Self::MissingFamilyResidual(id) => {
                write!(f, "candidate family {id} has no residual in the adequacy result")
            }
            Self::MissingHiddenPrediction(id) => write!(
                f,
                "hidden evaluator truth {id} has no generated observation prediction"
            ),
            Self::InvalidPredictionShape => {
                write!(f, "predicted observations do not match the bound time/sensor dimensions")
            }
            Self::NonFinitePrediction => {
                write!(f, "predicted or sampled observation is non-finite")
            }
            Self::NoAvailableHiddenModelId => {
                write!(f, "could not allocate collision-free hidden model IDs")
            }
        }
    }
}

impl std::error::Error for NoiseCalibrationError {}

/// Calibrate and validate residual thresholds under one explicitly bound
/// full space-time Gaussian covariance.
///
/// Candidate models use independent calibration and validation streams;
/// hidden out-of-family truths use a third evaluator-only stream. Hidden model
/// identities and truth are never returned in the report. The caller-supplied
/// covariance/process/sampling/operator/availability IDs are preserved, but
/// this function does not authenticate the external manifests behind them.
pub fn calibrate_model_family_adequacy_full_covariance(
    scenario: &ProfiledModelFamilyScenario,
    profile: &InputProfile,
    selected_sensor_ids: &[u32],
    covariance: &ObservationCovarianceV1,
    hidden_out_of_family_truths: &[HiddenOutOfFamilyTruth],
    config: NoiseCalibrationConfigV1,
) -> Result<NoiseAdequacyCalibrationReportV1, NoiseCalibrationError> {
    scenario
        .validate()
        .map_err(NoiseCalibrationError::Profiled)?;
    if config.replicates_per_model == 0 {
        return Err(NoiseCalibrationError::InvalidReplicateCount);
    }
    if !config.target_in_family_coverage.is_finite()
        || config.target_in_family_coverage <= 0.0
        || config.target_in_family_coverage >= 1.0
    {
        return Err(NoiseCalibrationError::InvalidTargetCoverage);
    }
    if config.calibration_seed == config.validation_seed
        || config.calibration_seed == config.evaluator_seed
        || config.validation_seed == config.evaluator_seed
    {
        return Err(NoiseCalibrationError::SeedsMustDiffer);
    }

    let calibration_units = scenario
        .experiment
        .models
        .len()
        .checked_mul(2)
        .and_then(|count| count.checked_add(hidden_out_of_family_truths.len()))
        .ok_or(NoiseCalibrationError::ExcessiveSampleCount)?;
    let requested_samples = calibration_units
        .checked_mul(config.replicates_per_model)
        .ok_or(NoiseCalibrationError::ExcessiveSampleCount)?;
    if requested_samples > MAX_TOTAL_NOISE_CALIBRATION_SAMPLES {
        return Err(NoiseCalibrationError::ExcessiveSampleCount);
    }

    let base_score = evaluate_input_profile(
        &scenario.experiment,
        profile,
        selected_sensor_ids,
    )
    .map_err(|error| {
        NoiseCalibrationError::Profiled(ProfiledDiscriminationError::Experiment(error))
    })?;
    if base_score.selected_sensor_ids != covariance.sensor_ids {
        return Err(NoiseCalibrationError::Covariance(
            CovarianceError::TraceSensorOrderMismatch,
        ));
    }
    if scenario.experiment.observation_times != covariance.times {
        return Err(NoiseCalibrationError::Covariance(
            CovarianceError::TraceTimeScheduleMismatch,
        ));
    }
    let expected_dimension = covariance
        .times
        .len()
        .checked_mul(covariance.sensor_ids.len())
        .ok_or(CovarianceError::DimensionTooLarge)?;
    if covariance.covariance().dimension() != expected_dimension {
        return Err(NoiseCalibrationError::Covariance(
            CovarianceError::DimensionMismatch {
                expected: expected_dimension,
                actual: covariance.covariance().dimension(),
            },
        ));
    }

    let family_by_model: HashMap<u32, u32> = scenario
        .family_assignments
        .iter()
        .map(|assignment| (assignment.model_id, assignment.family_id))
        .collect();
    let predictions: HashMap<u32, Vec<Vec<f64>>> = base_score
        .model_observation_means
        .iter()
        .map(|model| (model.model_id, model.observations.clone()))
        .collect();

    let mut hidden_truth_ids = std::collections::HashSet::new();
    for hidden in hidden_out_of_family_truths {
        if !hidden_truth_ids.insert(hidden.evaluator_id) {
            return Err(NoiseCalibrationError::DuplicateHiddenTruthId(
                hidden.evaluator_id,
            ));
        }
        if hidden.network.capacities != scenario.experiment.models[0].network.capacities
            || hidden.network.initial != scenario.experiment.models[0].network.initial
            || hidden.network.horizon != scenario.experiment.horizon()
        {
            return Err(NoiseCalibrationError::IncompatibleHiddenTruth {
                evaluator_id: hidden.evaluator_id,
            });
        }
    }

    let mut calibration_rng = SplitMix64::new(config.calibration_seed);
    let mut calibration_residuals = Vec::with_capacity(
        scenario.experiment.models.len() * config.replicates_per_model,
    );
    for model in &scenario.experiment.models {
        let true_family_id = *family_by_model
            .get(&model.id)
            .ok_or(NoiseCalibrationError::MissingTrueFamily(model.id))?;
        let means = predictions
            .get(&model.id)
            .ok_or(NoiseCalibrationError::MissingModelPrediction(model.id))?;
        for _ in 0..config.replicates_per_model {
            let trace = sample_bound_observation_trace(covariance, means, &mut calibration_rng)?;
            let adequacy = evaluate_model_family_adequacy_full_covariance(
                scenario,
                profile,
                selected_sensor_ids,
                &trace,
                covariance,
                ModelFamilyAdequacyPolicy {
                    maximum_gaussian_residual: f64::MAX,
                },
            )?;
            let residual = adequacy
                .adequacy
                .family_fits
                .iter()
                .find(|fit| fit.family_id == true_family_id)
                .map(|fit| fit.minimum_gaussian_residual)
                .ok_or(NoiseCalibrationError::MissingFamilyResidual(true_family_id))?;
            calibration_residuals.push(residual);
        }
    }
    if calibration_residuals.is_empty() {
        return Err(NoiseCalibrationError::ExcessiveSampleCount);
    }
    calibration_residuals.sort_by(f64::total_cmp);
    let rank = ((config.target_in_family_coverage * calibration_residuals.len() as f64).ceil()
        as usize)
        .clamp(1, calibration_residuals.len());
    let threshold = calibration_residuals[rank - 1];

    let mut validation_rng = SplitMix64::new(config.validation_seed);
    let mut validation_samples = 0_usize;
    let mut validation_in_family_accepts = 0_usize;
    let mut validation_false_out_of_model_set_count = 0_usize;
    let mut validation_wrong_family_preference_count = 0_usize;
    let mut validation_ambiguous_count = 0_usize;

    for model in &scenario.experiment.models {
        let true_family_id = *family_by_model
            .get(&model.id)
            .ok_or(NoiseCalibrationError::MissingTrueFamily(model.id))?;
        let means = predictions
            .get(&model.id)
            .ok_or(NoiseCalibrationError::MissingModelPrediction(model.id))?;
        for _ in 0..config.replicates_per_model {
            validation_samples += 1;
            let trace = sample_bound_observation_trace(covariance, means, &mut validation_rng)?;
            let adequacy = evaluate_model_family_adequacy_full_covariance(
                scenario,
                profile,
                selected_sensor_ids,
                &trace,
                covariance,
                ModelFamilyAdequacyPolicy {
                    maximum_gaussian_residual: threshold,
                },
            )?;
            match &adequacy.adequacy.disposition {
                ModelFamilyAdequacyDisposition::OutOfModelSet => {
                    validation_false_out_of_model_set_count += 1;
                }
                ModelFamilyAdequacyDisposition::InFamilyPreferred { family_id, .. } => {
                    if *family_id == true_family_id {
                        validation_in_family_accepts += 1;
                    } else {
                        validation_wrong_family_preference_count += 1;
                    }
                }
                ModelFamilyAdequacyDisposition::ParameterNotIdentified { family_id, .. } => {
                    if *family_id == true_family_id {
                        validation_in_family_accepts += 1;
                    } else {
                        validation_wrong_family_preference_count += 1;
                    }
                    validation_ambiguous_count += 1;
                }
                ModelFamilyAdequacyDisposition::InFamilyUnresolved {
                    compatible_family_ids,
                } => {
                    if compatible_family_ids.contains(&true_family_id) {
                        validation_in_family_accepts += 1;
                    } else {
                        validation_wrong_family_preference_count += 1;
                    }
                    validation_ambiguous_count += 1;
                }
            }
        }
    }

    let mut evaluator_rng = SplitMix64::new(config.evaluator_seed);
    let mut out_of_family_samples = 0_usize;
    let mut out_of_family_rejections = 0_usize;
    for hidden in hidden_out_of_family_truths {
        let means = predict_hidden_network(
            &hidden.network,
            scenario,
            profile,
            selected_sensor_ids,
        )?;
        for _ in 0..config.replicates_per_model {
            out_of_family_samples += 1;
            let trace = sample_bound_observation_trace(covariance, &means, &mut evaluator_rng)?;
            let adequacy = evaluate_model_family_adequacy_full_covariance(
                scenario,
                profile,
                selected_sensor_ids,
                &trace,
                covariance,
                ModelFamilyAdequacyPolicy {
                    maximum_gaussian_residual: threshold,
                },
            )?;
            if adequacy.adequacy.disposition == ModelFamilyAdequacyDisposition::OutOfModelSet {
                out_of_family_rejections += 1;
            }
        }
    }

    let calibration_samples = calibration_residuals.len();
    let calibration_accepts = calibration_residuals
        .iter()
        .filter(|residual| **residual <= threshold)
        .count();
    let validation_in_family_coverage = rate(validation_in_family_accepts, validation_samples);
    Ok(NoiseAdequacyCalibrationReportV1 {
        covariance_id: covariance.covariance_id.clone(),
        observation_process_id: covariance.observation_process_id.clone(),
        sampling_policy_id: covariance.sampling_policy_id.clone(),
        observation_operator_id: covariance.observation_operator_id.clone(),
        availability_cutoff_id: covariance.availability_cutoff_id.clone(),
        sensor_ids: covariance.sensor_ids.clone(),
        times: covariance.times.clone(),
        covariance_dimension: covariance.covariance().dimension(),
        covariance_kind: covariance.covariance().kind_label(),
        threshold,
        target_in_family_coverage: config.target_in_family_coverage,
        calibration_samples,
        calibration_in_family_accepts: calibration_accepts,
        calibration_in_family_coverage: rate(calibration_accepts, calibration_samples),
        calibration_in_family_coverage_wilson_95: wilson_interval(
            calibration_accepts,
            calibration_samples,
        ),
        validation_samples,
        validation_in_family_accepts,
        validation_in_family_coverage,
        validation_in_family_coverage_wilson_95: wilson_interval(
            validation_in_family_accepts,
            validation_samples,
        ),
        validation_false_out_of_model_set_count,
        validation_false_out_of_model_set_rate: rate(
            validation_false_out_of_model_set_count,
            validation_samples,
        ),
        validation_false_out_of_model_set_wilson_95: wilson_interval(
            validation_false_out_of_model_set_count,
            validation_samples,
        ),
        validation_wrong_family_preference_count,
        validation_wrong_family_preference_rate: rate(
            validation_wrong_family_preference_count,
            validation_samples,
        ),
        validation_ambiguous_count,
        validation_ambiguous_rate: rate(validation_ambiguous_count, validation_samples),
        out_of_family_samples,
        out_of_family_rejections,
        out_of_family_rejection_rate: rate(out_of_family_rejections, out_of_family_samples),
        out_of_family_rejection_wilson_95: wilson_interval(
            out_of_family_rejections,
            out_of_family_samples,
        ),
        out_of_family_false_accepts: out_of_family_samples - out_of_family_rejections,
        out_of_family_false_accept_rate: rate(
            out_of_family_samples - out_of_family_rejections,
            out_of_family_samples,
        ),
        out_of_family_false_accept_wilson_95: wilson_interval(
            out_of_family_samples - out_of_family_rejections,
            out_of_family_samples,
        ),
        calibration_seed: config.calibration_seed,
        validation_seed: config.validation_seed,
        evaluator_seed: config.evaluator_seed,
        assumptions: vec![
            "additive zero-mean Gaussian noise",
            "known full space-time covariance shared by candidate and hidden truths",
            "covariance/process/sampling/operator/availability identity is fixed for this calibration",
            "independent calibration, in-family validation and hidden-truth evaluator PRNG streams",
            "finite synthetic Monte Carlo counts; rates are empirical, not universal guarantees",
            "candidate scorer receives no hidden truth IDs; this in-process evaluator requires external process isolation for agent-facing use",
        ],
    })
}

fn sample_bound_observation_trace(
    covariance: &ObservationCovarianceV1,
    means: &[Vec<f64>],
    rng: &mut SplitMix64,
) -> Result<ObservationProcessTraceV1, NoiseCalibrationError> {
    let time_count = covariance.times.len();
    let sensor_count = covariance.sensor_ids.len();
    if means.len() != time_count || means.iter().any(|row| row.len() != sensor_count) {
        return Err(NoiseCalibrationError::InvalidPredictionShape);
    }
    if means.iter().flatten().any(|value| !value.is_finite()) {
        return Err(NoiseCalibrationError::NonFinitePrediction);
    }
    let noise = covariance.covariance().sample_gaussian(rng)?;
    let mut values = means.to_vec();
    for time_index in 0..time_count {
        for sensor_index in 0..sensor_count {
            let flattened_index = time_index * sensor_count + sensor_index;
            values[time_index][sensor_index] += noise[flattened_index];
            if !values[time_index][sensor_index].is_finite() {
                return Err(NoiseCalibrationError::NonFinitePrediction);
            }
        }
    }
    ObservationProcessTraceV1::new(
        covariance.observation_process_id.clone(),
        covariance.sampling_policy_id.clone(),
        covariance.observation_operator_id.clone(),
        covariance.availability_cutoff_id.clone(),
        ObservedSensorTrace {
            sensor_ids: covariance.sensor_ids.clone(),
            times: covariance.times.clone(),
            values,
        },
    )
    .map_err(NoiseCalibrationError::Covariance)
}

fn predict_hidden_network(
    hidden_network: &crate::network::NetworkScenario,
    scenario: &ProfiledModelFamilyScenario,
    profile: &InputProfile,
    selected_sensor_ids: &[u32],
) -> Result<Vec<Vec<f64>>, NoiseCalibrationError> {
    let mut hidden_experiment = scenario.experiment.clone();
    let mut first_id = 0_u32;
    while scenario.experiment.models.iter().any(|model| model.id == first_id) {
        first_id = first_id
            .checked_add(1)
            .ok_or(NoiseCalibrationError::NoAvailableHiddenModelId)?;
    }
    let mut second_id = first_id
        .checked_add(1)
        .ok_or(NoiseCalibrationError::NoAvailableHiddenModelId)?;
    while scenario
        .experiment
        .models
        .iter()
        .any(|model| model.id == second_id)
    {
        second_id = second_id
            .checked_add(1)
            .ok_or(NoiseCalibrationError::NoAvailableHiddenModelId)?;
    }
    hidden_experiment.models = vec![
        ModelCandidate {
            id: first_id,
            label: "sealed hidden out-of-family truth",
            network: hidden_network.clone(),
        },
        ModelCandidate {
            id: second_id,
            label: "sealed hidden duplicate for reference execution",
            network: hidden_network.clone(),
        },
    ];
    let hidden_score = evaluate_input_profile(
        &hidden_experiment,
        profile,
        selected_sensor_ids,
    )
    .map_err(|error| {
        NoiseCalibrationError::Profiled(ProfiledDiscriminationError::Experiment(error))
    })?;
    hidden_score
        .model_observation_means
        .iter()
        .find(|prediction| prediction.model_id == first_id)
        .map(|prediction| prediction.observations.clone())
        .ok_or(NoiseCalibrationError::MissingHiddenPrediction(first_id))
}

fn rate(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        count as f64 / total as f64
    }
}

fn wilson_interval(successes: usize, total: usize) -> [f64; 2] {
    if total == 0 {
        return [0.0, 0.0];
    }
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
    fn ar1_correlation_sweep_exposes_increasing_iid_miscalibration() {
        let reports = run_ar1_robustness_corpus(20_000, 20_261_118).unwrap();
        assert_eq!(reports.len(), 3);
        assert_eq!(reports[0].rho, 0.2);
        assert_eq!(reports[1].rho, 0.5);
        assert_eq!(reports[2].rho, 0.8);
        assert!(reports[0].iid_assumption_false_rejection_rate
            < reports[1].iid_assumption_false_rejection_rate);
        assert!(reports[1].iid_assumption_false_rejection_rate
            < reports[2].iid_assumption_false_rejection_rate);
        for report in reports {
            assert!(report.full_covariance_false_rejection_rate > 0.04);
            assert!(report.full_covariance_false_rejection_rate < 0.06);
        }
    }

    #[test]
    fn ar1_negative_control_is_deterministic_for_frozen_identity() {
        let first = run_ar1_negative_control(4_096, 77, 0.8).unwrap();
        let second = run_ar1_negative_control(4_096, 77, 0.8).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn full_covariance_calibration_binds_identity_and_uses_separate_streams() {
        use crate::adequacy_calibration::HiddenOutOfFamilyTruth;
        use crate::experiment_design::{
            ExperimentDesignScenario, ForcingSegment, ModelCandidate,
        };
        use crate::network::{Edge, NetworkScenario};
        use crate::robust_discrimination::{
            ModelFamilyAssignment, ProfiledModelFamilyScenario,
        };
        use crate::sensor_design::SensorCandidate;

        let make_network = |name: &'static str, conductance: f64| NetworkScenario {
            name,
            capacities: vec![1.0, 1.0],
            initial: vec![0.0, 0.0],
            edges: vec![Edge {
                left: 0,
                right: 1,
                conductance,
            }],
            horizon: 1.0,
        };
        let scenario = ProfiledModelFamilyScenario {
            experiment: ExperimentDesignScenario {
                name: "full-covariance-calibration-fixture",
                models: vec![
                    ModelCandidate {
                        id: 1,
                        label: "chain conductance 0.5",
                        network: make_network("candidate_low", 0.5),
                    },
                    ModelCandidate {
                        id: 2,
                        label: "chain conductance 1.0",
                        network: make_network("candidate_high", 1.0),
                    },
                ],
                sensors: vec![SensorCandidate {
                    id: 10,
                    node: 0,
                    cost: 1.0,
                    common_cause_group: None,
                }],
                noise_covariance: vec![vec![1.0]],
                observation_times: vec![0.25, 0.5],
                sensor_budget: 1.0,
                max_abs_forcing: 2.0,
                input_energy_budget: 2.0,
            },
            family_assignments: vec![
                ModelFamilyAssignment {
                    model_id: 1,
                    family_id: 100,
                },
                ModelFamilyAssignment {
                    model_id: 2,
                    family_id: 200,
                },
            ],
        };
        let profile = InputProfile {
            id: 101,
            label: "constant unit forcing",
            segments: vec![ForcingSegment {
                start_time: 0.0,
                end_time: 1.0,
                forcing: vec![1.0, 0.0],
            }],
        };
        let covariance = ObservationCovarianceV1::new(
            "covariance:calibration-fixture-v1".to_owned(),
            "process:gaussian-ar1-rho-0.8".to_owned(),
            "sampling:two-times-v1".to_owned(),
            "operator:node-0-sensor-v1".to_owned(),
            "availability:cutoff-v1".to_owned(),
            vec![10],
            vec![0.25, 0.5],
            DenseSpdCovariance::new(vec![
                vec![1.0e-5, 8.0e-6],
                vec![8.0e-6, 1.0e-5],
            ])
            .unwrap(),
        )
        .unwrap();
        let hidden = HiddenOutOfFamilyTruth {
            evaluator_id: 900,
            network: make_network("sealed_conductance_1_5", 1.5),
        };

        let report = calibrate_model_family_adequacy_full_covariance(
            &scenario,
            &profile,
            &[10],
            &covariance,
            &[hidden],
            NoiseCalibrationConfigV1 {
                calibration_seed: 0xCA11_BA7E,
                validation_seed: 0xB01D_5EED,
                evaluator_seed: 0xE0A1_0D01,
                replicates_per_model: 128,
                target_in_family_coverage: 0.95,
            },
        )
        .unwrap();

        assert_eq!(report.covariance_id, "covariance:calibration-fixture-v1");
        assert_eq!(report.observation_process_id, "process:gaussian-ar1-rho-0.8");
        assert_eq!(report.observation_operator_id, "operator:node-0-sensor-v1");
        assert_eq!(report.availability_cutoff_id, "availability:cutoff-v1");
        assert_eq!(report.calibration_samples, 256);
        assert_eq!(report.validation_samples, 256);
        assert_eq!(report.out_of_family_samples, 128);
        assert!(report.calibration_in_family_coverage >= 0.95);
        assert!(report.validation_in_family_coverage > 0.85);
        assert!(report.out_of_family_rejection_rate > 0.90);
        assert_ne!(report.calibration_seed, report.validation_seed);
        assert!(
            report
                .assumptions
                .iter()
                .any(|item| item.contains("full space-time covariance"))
        );
        assert_eq!(report.out_of_family_false_accepts + report.out_of_family_rejections, 128);
        assert_ne!(report.evaluator_seed, report.validation_seed);
    }

    #[test]
    fn covariance_identity_binds_sensor_order_and_sampling_schedule() {
        let covariance = ar1_time_covariance(2, 0.8).unwrap();
        let binding = ObservationCovarianceV1::new(
            "covariance-sha256:fixture".to_owned(),
            "noise-process:ar1-rho-0.8".to_owned(),
            "sampling-policy:two-evenly-spaced-times".to_owned(),
            "operator:scalar-node-sensor".to_owned(),
            "availability:cutoff-v1".to_owned(),
            vec![7],
            vec![0.25, 0.5],
            covariance,
        )
        .unwrap();
        let matching = ObservationProcessTraceV1::new(
            "noise-process:ar1-rho-0.8".to_owned(),
            "sampling-policy:two-evenly-spaced-times".to_owned(),
            "operator:scalar-node-sensor".to_owned(),
            "availability:cutoff-v1".to_owned(),
            ObservedSensorTrace {
                sensor_ids: vec![7],
                times: vec![0.25, 0.5],
                values: vec![vec![0.0], vec![0.0]],
            },
        )
        .unwrap();
        assert!(binding.validate_trace(&matching).is_ok());

        let reordered = ObservationProcessTraceV1 {
            trace: ObservedSensorTrace {
                sensor_ids: vec![8],
                ..matching.trace.clone()
            },
            ..matching.clone()
        };
        assert_eq!(
            binding.validate_trace(&reordered),
            Err(CovarianceError::TraceSensorOrderMismatch)
        );

        let shifted = ObservationProcessTraceV1 {
            trace: ObservedSensorTrace {
                times: vec![0.25, 0.75],
                ..matching.trace.clone()
            },
            ..matching.clone()
        };
        assert_eq!(
            binding.validate_trace(&shifted),
            Err(CovarianceError::TraceTimeScheduleMismatch)
        );

        let changed_process = ObservationProcessTraceV1 {
            observation_process_id: "other-process".to_owned(),
            ..matching.clone()
        };
        assert_eq!(
            binding.validate_trace(&changed_process),
            Err(CovarianceError::TraceProcessIdentityMismatch)
        );
        let changed_sampling = ObservationProcessTraceV1 {
            sampling_policy_id: "other-sampling".to_owned(),
            ..matching.clone()
        };
        assert_eq!(
            binding.validate_trace(&changed_sampling),
            Err(CovarianceError::TraceSamplingPolicyMismatch)
        );
        let changed_operator = ObservationProcessTraceV1 {
            observation_operator_id: "other-operator".to_owned(),
            ..matching.clone()
        };
        assert_eq!(
            binding.validate_trace(&changed_operator),
            Err(CovarianceError::TraceOperatorMismatch)
        );
        let changed_cutoff = ObservationProcessTraceV1 {
            availability_cutoff_id: "other-cutoff".to_owned(),
            ..matching
        };
        assert_eq!(
            binding.validate_trace(&changed_cutoff),
            Err(CovarianceError::TraceAvailabilityCutoffMismatch)
        );
    }

    #[test]
    fn masked_nan_is_ignored_but_observed_nan_is_rejected() {
        let covariance = DenseSpdCovariance::new(vec![
            vec![1.0, 0.1, 0.2],
            vec![0.1, 1.0, 0.3],
            vec![0.2, 0.3, 1.0],
        ])
        .unwrap();
        let (selected, _) =
            select_observed_subspace(&[4.0, f64::NAN, 6.0], &[true, false, true], &covariance)
                .unwrap();
        assert_eq!(selected, vec![4.0, 6.0]);
        assert!(matches!(
            select_observed_subspace(
                &[4.0, f64::NAN, 6.0],
                &[true, true, false],
                &covariance
            ),
            Err(CovarianceError::NonFiniteObservedValue { index: 1 })
        ));
    }

    #[test]
    fn full_space_time_adequacy_uses_joint_mahalanobis_residual() {
        use crate::experiment_design::{
            ExperimentDesignScenario, ForcingSegment, ModelCandidate,
        };
        use crate::network::{Edge, NetworkScenario};
        use crate::robust_discrimination::{
            ModelFamilyAssignment, ProfiledModelFamilyScenario,
        };
        use crate::sensor_design::SensorCandidate;

        let make_network = |name: &'static str, conductance: f64| NetworkScenario {
            name,
            capacities: vec![1.0, 1.0],
            initial: vec![0.0, 0.0],
            edges: vec![Edge {
                left: 0,
                right: 1,
                conductance,
            }],
            horizon: 1.0,
        };
        let experiment = ExperimentDesignScenario {
            name: "two-model-two-time-covariance-fixture",
            models: vec![
                ModelCandidate {
                    id: 1,
                    label: "low conductance",
                    network: make_network("low", 0.5),
                },
                ModelCandidate {
                    id: 2,
                    label: "high conductance",
                    network: make_network("high", 1.0),
                },
            ],
            sensors: vec![SensorCandidate {
                id: 10,
                node: 0,
                cost: 1.0,
                common_cause_group: None,
            }],
            noise_covariance: vec![vec![1.0]],
            observation_times: vec![0.25, 0.5],
            sensor_budget: 1.0,
            max_abs_forcing: 2.0,
            input_energy_budget: 2.0,
        };
        let profile = InputProfile {
            id: 101,
            label: "constant unit forcing",
            segments: vec![ForcingSegment {
                start_time: 0.0,
                end_time: 1.0,
                forcing: vec![1.0, 0.0],
            }],
        };
        let scenario = ProfiledModelFamilyScenario {
            experiment,
            family_assignments: vec![
                ModelFamilyAssignment {
                    model_id: 1,
                    family_id: 100,
                },
                ModelFamilyAssignment {
                    model_id: 2,
                    family_id: 200,
                },
            ],
        };
        let nominal = evaluate_input_profile(&scenario.experiment, &profile, &[10]).unwrap();
        let model_one = nominal
            .model_observation_means
            .iter()
            .find(|model| model.model_id == 1)
            .unwrap();
        let mut values = model_one.observations.clone();
        values[0][0] += 1.0;
        values[1][0] -= 0.2;
        let trace = ObservationProcessTraceV1::new(
            "process:gaussian-ar1-rho-0.8".to_owned(),
            "sampling:two-times".to_owned(),
            "operator:node-0-sensor".to_owned(),
            "availability:fixture-cutoff".to_owned(),
            ObservedSensorTrace {
                sensor_ids: vec![10],
                times: vec![0.25, 0.5],
                values,
            },
        )
        .unwrap();
        let covariance = ObservationCovarianceV1::new(
            "covariance:ar1-fixture".to_owned(),
            "process:gaussian-ar1-rho-0.8".to_owned(),
            "sampling:two-times".to_owned(),
            "operator:node-0-sensor".to_owned(),
            "availability:fixture-cutoff".to_owned(),
            vec![10],
            vec![0.25, 0.5],
            ar1_time_covariance(2, 0.8).unwrap(),
        )
        .unwrap();

        let report = evaluate_model_family_adequacy_full_covariance(
            &scenario,
            &profile,
            &[10],
            &trace,
            &covariance,
            ModelFamilyAdequacyPolicy {
                maximum_gaussian_residual: 100.0,
            },
        )
        .unwrap();
        let family_one = report
            .adequacy
            .family_fits
            .iter()
            .find(|family| family.family_id == 100)
            .unwrap();
        let expected = 0.5 * (1.0 + 2.0 * 0.8 * 0.2 + 0.2_f64.powi(2)) / (1.0 - 0.8_f64.powi(2));
        assert!((family_one.minimum_gaussian_residual - expected).abs() < 1.0e-10);
        assert_eq!(report.covariance_id, "covariance:ar1-fixture");
        assert_eq!(report.observation_process_id, "process:gaussian-ar1-rho-0.8");
        assert_eq!(report.sampling_policy_id, "sampling:two-times");
        assert_eq!(report.observation_operator_id, "operator:node-0-sensor");
        assert_eq!(report.availability_cutoff_id, "availability:fixture-cutoff");
        assert_eq!(report.covariance_dimension, 2);
    }

}
