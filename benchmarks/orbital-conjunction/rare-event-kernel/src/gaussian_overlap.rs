// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Strict fixed-size product-Gaussian overlap for conjunction assessment.
//!
//! For two Gaussian position distributions with means r1/r2 and covariances
//! A1/A2, their product is proportional to a Gaussian with:
//!
//!   Sigma_p = (A1^-1 + A2^-1)^-1
//!   mu_p    = Sigma_p * (A1^-1*r1 + A2^-1*r2)
//!
//! NASA PeakOverlapPos uses this overlap point and adds HBR² I to the covariance
//! used for its relative-Mahalanobis convergence metric. This remediated metric
//! is distinct from the raw product-Gaussian covariance.
//!
//! The strict API never eigenvalue-clips or adds covariance jitter. A separately
//! named fixed-time NASA compatibility API implements the pinned HBR-dependent
//! eigenvalue-floor sequence and retains its raw/remediated spectral evidence.
 //! Neither API is a replacement for the iterative PeakOverlapPos state machine.

use crate::KernelError;
use crate::conditional_velocity::Matrix3;

/// Mean and covariance of one object's position distribution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaussianPositionDistribution3 {
    /// Cartesian mean position.
    pub mean_position: [f64; 3],
    /// Symmetric positive-definite covariance.
    pub covariance: Matrix3,
}

/// Numerical policy for strict Gaussian overlap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaussianOverlapConfig {
    /// Relative tolerance for input covariance symmetry; must not exceed 1e-6.
    pub symmetry_relative_tolerance: f64,
    /// Maximum permitted infinity-norm condition estimate.
    pub maximum_condition_estimate: f64,
}

impl Default for GaussianOverlapConfig {
    fn default() -> Self {
        Self {
            symmetry_relative_tolerance: 1.0e-12,
            maximum_condition_estimate: 1.0e12,
        }
    }
}

/// Explicit covariance policy identity for the overlap calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaussianOverlapCovariancePolicy {
    /// Require SPD inputs and never clip, jitter or otherwise repair covariance.
    StrictSpd,
}

/// Conditioning diagnostics using the matrix infinity norm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CovarianceConditionDiagnostics {
    /// Infinity norm of the covariance.
    pub covariance_infinity_norm: f64,
    /// Infinity norm of the inverse covariance.
    pub inverse_infinity_norm: f64,
    /// Product of the two norms.
    pub condition_estimate: f64,
}

/// Product-Gaussian overlap and HBR-remediated convergence metric.
#[derive(Debug, Clone, PartialEq)]
pub struct GaussianOverlapReport {
    /// Center of the product-Gaussian overlap distribution.
    pub overlap_position: [f64; 3],
    /// Raw product-Gaussian overlap covariance.
    pub overlap_covariance: Matrix3,
    /// HBR-remediated convergence covariance: overlap covariance + HBR² I.
    pub remediated_metric_covariance: Matrix3,
    /// Inverse HBR-remediated convergence covariance.
    pub remediated_metric_inverse: Matrix3,
    /// Conditioning diagnostics for the primary covariance.
    pub primary_condition: CovarianceConditionDiagnostics,
    /// Conditioning diagnostics for the secondary covariance.
    pub secondary_condition: CovarianceConditionDiagnostics,
    /// Conditioning diagnostics for the overlap covariance.
    pub overlap_condition: CovarianceConditionDiagnostics,
    /// Conditioning diagnostics for the HBR-remediated convergence metric.
    pub remediated_metric_condition: CovarianceConditionDiagnostics,
    /// Exact configuration used for this calculation.
    pub configuration: GaussianOverlapConfig,
    /// Maximum absolute asymmetry in the primary covariance.
    pub primary_symmetry_residual: f64,
    /// Maximum absolute asymmetry in the secondary covariance.
    pub secondary_symmetry_residual: f64,
    /// HBR used to form the convergence metric.
    pub hard_body_radius: f64,
    /// Explicit covariance policy.
    pub policy: GaussianOverlapCovariancePolicy,
}

/// Compute squared relative-Mahalanobis displacement between successive overlap points.
///
/// The displacement uses the HBR-remediated metric, not the raw overlap covariance.
pub fn overlap_step_mahalanobis_squared(
    previous_overlap_position: [f64; 3],
    current_overlap_position: [f64; 3],
    remediated_metric_covariance: Matrix3,
    config: GaussianOverlapConfig,
) -> Result<f64, KernelError> {
    if !finite_vector(previous_overlap_position)
        || !finite_vector(current_overlap_position)
        || !finite_matrix(remediated_metric_covariance)
    {
        return Err(KernelError::NonFiniteValue);
    }
    validate_config(config)?;
    // Validate and invert the supplied covariance here. Accepting an arbitrary
    // caller-supplied inverse could make an indefinite metric look valid for
    // one particular displacement.
    let (symmetric_metric, _) =
        checked_symmetric(remediated_metric_covariance, config.symmetry_relative_tolerance)?;
    let (remediated_metric_inverse, metric_condition) = inverse_and_condition(symmetric_metric)?;
    if metric_condition.condition_estimate > config.maximum_condition_estimate {
        return Err(KernelError::IllConditionedCovariance);
    }
    let displacement = subtract_vector(current_overlap_position, previous_overlap_position);
    let transformed = matrix_vector(remediated_metric_inverse, displacement);
    let value = dot(displacement, transformed);
    if !value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if value < -1.0e-10 {
        return Err(KernelError::NotPositiveDefinite);
    }
    Ok(value.max(0.0))
}

/// Compute the strict product-Gaussian overlap for two position distributions.
///
/// Input covariances must be symmetric positive definite and below the configured
/// condition estimate. No eigenvalue clipping, covariance jitter or HBR-dependent
/// covariance floor is applied to the two input distributions.
pub fn gaussian_position_overlap(
    primary: GaussianPositionDistribution3,
    secondary: GaussianPositionDistribution3,
    hard_body_radius: f64,
    config: GaussianOverlapConfig,
) -> Result<GaussianOverlapReport, KernelError> {
    if !finite_vector(primary.mean_position)
        || !finite_vector(secondary.mean_position)
        || !hard_body_radius.is_finite()
    {
        return Err(KernelError::NonFiniteValue);
    }
    if hard_body_radius <= 0.0 {
        return Err(KernelError::InvalidHardBodyRadius);
    }
    validate_config(config)?;
    if !finite_matrix(primary.covariance) || !finite_matrix(secondary.covariance) {
        return Err(KernelError::NonFiniteValue);
    }

    let (primary_covariance, primary_symmetry_residual) =
        checked_symmetric(primary.covariance, config.symmetry_relative_tolerance)?;
    let (secondary_covariance, secondary_symmetry_residual) =
        checked_symmetric(secondary.covariance, config.symmetry_relative_tolerance)?;
    let (primary_inverse, primary_condition) =
        inverse_and_condition(primary_covariance)?;
    let (secondary_inverse, secondary_condition) =
        inverse_and_condition(secondary_covariance)?;

    if primary_condition.condition_estimate > config.maximum_condition_estimate
        || secondary_condition.condition_estimate > config.maximum_condition_estimate
    {
        return Err(KernelError::IllConditionedCovariance);
    }

    let precision_sum = add_matrix(primary_inverse, secondary_inverse);
    let (overlap_covariance, precision_condition) = inverse_and_condition(precision_sum)?;
    // inverse_and_condition reports the norm of its input and its inverse.
    // Swap those norm fields so this diagnostic is explicitly for the product
    // covariance (whose inverse is the precision sum).
    let overlap_condition = CovarianceConditionDiagnostics {
        covariance_infinity_norm: precision_condition.inverse_infinity_norm,
        inverse_infinity_norm: precision_condition.covariance_infinity_norm,
        condition_estimate: precision_condition.condition_estimate,
    };
    if overlap_condition.condition_estimate > config.maximum_condition_estimate {
        return Err(KernelError::IllConditionedCovariance);
    }

    let primary_precision_mean = matrix_vector(primary_inverse, primary.mean_position);
    let secondary_precision_mean = matrix_vector(secondary_inverse, secondary.mean_position);
    let precision_mean = add_vector(primary_precision_mean, secondary_precision_mean);
    let overlap_position = matrix_vector(overlap_covariance, precision_mean);
    if !finite_vector(overlap_position) {
        return Err(KernelError::NumericalOverflow);
    }

    let hbr_squared = hard_body_radius * hard_body_radius;
    if !hbr_squared.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    let mut remediated_metric_covariance = overlap_covariance;
    for (axis, row) in remediated_metric_covariance.iter_mut().enumerate() {
        row[axis] += hbr_squared;
    }
    let (remediated_metric_inverse, remediated_metric_condition) =
        inverse_and_condition(remediated_metric_covariance)?;
    if remediated_metric_condition.condition_estimate > config.maximum_condition_estimate {
        return Err(KernelError::IllConditionedCovariance);
    }

    Ok(GaussianOverlapReport {
        overlap_position,
        overlap_covariance,
        remediated_metric_covariance,
        remediated_metric_inverse,
        primary_condition,
        secondary_condition,
        overlap_condition,
        remediated_metric_condition,
        configuration: config,
        primary_symmetry_residual,
        secondary_symmetry_residual,
        hard_body_radius,
        policy: GaussianOverlapCovariancePolicy::StrictSpd,
    })
}

fn validate_config(config: GaussianOverlapConfig) -> Result<(), KernelError> {
    if !config.symmetry_relative_tolerance.is_finite()
        || config.symmetry_relative_tolerance < 0.0
        || config.symmetry_relative_tolerance > 1.0e-6
        || !config.maximum_condition_estimate.is_finite()
        || config.maximum_condition_estimate <= 1.0
    {
        Err(KernelError::InvalidGaussianOverlapConfig)
    } else {
        Ok(())
    }
}

fn checked_symmetric(
    matrix: Matrix3,
    relative_tolerance: f64,
) -> Result<(Matrix3, f64), KernelError> {
    let scale = max_abs_matrix(matrix).max(f64::MIN_POSITIVE);
    let mut maximum_residual: f64 = 0.0;
    for (i, row) in matrix.iter().enumerate() {
        for (j, value) in row.iter().enumerate().skip(i + 1) {
            maximum_residual = maximum_residual.max((*value - matrix[j][i]).abs());
        }
    }
    if maximum_residual > relative_tolerance * scale {
        return Err(KernelError::NonSymmetricMatrix);
    }
    let mut symmetric = [[0.0; 3]; 3];
    for (i, row) in symmetric.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = 0.5 * (matrix[i][j] + matrix[j][i]);
        }
    }
    if !finite_matrix(symmetric) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok((symmetric, maximum_residual))
}

fn inverse_and_condition(
    matrix: Matrix3,
) -> Result<(Matrix3, CovarianceConditionDiagnostics), KernelError> {
    let inverse = inverse_spd(matrix)?;
    let covariance_infinity_norm = infinity_norm(matrix);
    let inverse_infinity_norm = infinity_norm(inverse);
    let condition_estimate = covariance_infinity_norm * inverse_infinity_norm;
    if !condition_estimate.is_finite() {
        return Err(KernelError::IllConditionedCovariance);
    }
    Ok((
        inverse,
        CovarianceConditionDiagnostics {
            covariance_infinity_norm,
            inverse_infinity_norm,
            condition_estimate,
        },
    ))
}

fn infinity_norm(matrix: Matrix3) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn cholesky(matrix: Matrix3) -> Result<Matrix3, KernelError> {
    let mut lower = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for (left, right) in lower[row].iter().zip(lower[column].iter()).take(column) {
                value -= left * right;
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return Err(KernelError::NotPositiveDefinite);
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
                if !lower[row][column].is_finite() {
                    return Err(KernelError::NumericalOverflow);
                }
            }
        }
    }
    Ok(lower)
}

fn inverse_spd(matrix: Matrix3) -> Result<Matrix3, KernelError> {
    let lower = cholesky(matrix)?;
    let mut inverse = [[0.0; 3]; 3];
    for (column, destination_column) in inverse.iter_mut().enumerate() {
        let mut forward = [0.0; 3];
        for row in 0..3 {
            let mut value = if row == column { 1.0 } else { 0.0 };
            for k in 0..row {
                value -= lower[row][k] * forward[k];
            }
            forward[row] = value / lower[row][row];
        }
        let mut solution = [0.0; 3];
        for row in (0..3).rev() {
            let mut value = forward[row];
            for k in (row + 1)..3 {
                value -= lower[k][row] * solution[k];
            }
            solution[row] = value / lower[row][row];
        }
        for (row, value) in destination_column.iter_mut().enumerate() {
            *value = solution[row];
        }
    }
    let mut symmetric = [[0.0; 3]; 3];
    for (i, row) in symmetric.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = 0.5 * (inverse[i][j] + inverse[j][i]);
        }
    }
    if !finite_matrix(symmetric) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(symmetric)
}

fn add_matrix(left: Matrix3, right: Matrix3) -> Matrix3 {
    let mut output = [[0.0; 3]; 3];
    for (i, row) in output.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = left[i][j] + right[i][j];
        }
    }
    output
}

fn matrix_vector(matrix: Matrix3, vector: [f64; 3]) -> [f64; 3] {
    let mut output = [0.0; 3];
    for (i, value) in output.iter_mut().enumerate() {
        *value = matrix[i][0] * vector[0]
            + matrix[i][1] * vector[1]
            + matrix[i][2] * vector[2];
    }
    output
}

fn add_vector(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn subtract_vector(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn finite_vector(vector: [f64; 3]) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn finite_matrix(matrix: Matrix3) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn max_abs_matrix(matrix: Matrix3) -> f64 {
    matrix.iter().flatten().map(|value| value.abs()).fold(0.0, f64::max)
}

/// Input covariance definiteness before NASA-style eigenvalue remediation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CovarianceDefiniteness {
    /// At least one raw eigenvalue was negative.
    Indefinite,
    /// The minimum raw eigenvalue was exactly zero and none was negative.
    PositiveSemidefinite,
    /// Every raw eigenvalue was strictly positive.
    PositiveDefinite,
}

/// Reproducible record of one NASA-style covariance eigenvalue clipping operation.
#[derive(Debug, Clone, PartialEq)]
pub struct NasaCovarianceClipReport {
    /// Eigenvalues of the symmetrized, unremediated covariance, in ascending order.
    pub raw_eigenvalues: [f64; 3],
    /// Eigenvalues after applying max(lambda, clip_floor), in matching order.
    pub remediated_eigenvalues: [f64; 3],
    /// Eigenvectors as columns, corresponding to the eigenvalue arrays.
    pub eigenvectors: Matrix3,
    /// Definiteness classification before any clipping.
    pub input_definiteness: CovarianceDefiniteness,
    /// True exactly when one or more raw eigenvalues were strictly below the floor.
    pub clipping_applied: bool,
    /// Nonnegative eigenvalue floor used by the NASA compatibility calculation.
    pub clip_floor: f64,
    /// Reconstructed remediated covariance (or the symmetrized input when unchanged).
    pub remediated_covariance: Matrix3,
    /// Inverse of the remediated covariance.
    pub remediated_covariance_inverse: Matrix3,
    /// Log determinant of the remediated covariance.
    pub remediated_log_determinant: f64,
    /// Infinity-norm condition estimate of the remediated covariance.
    pub remediated_condition: CovarianceConditionDiagnostics,
}

/// Parameters for the pinned NASA CARA PeakOverlapPos covariance policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NasaPeakOverlapConfig {
    /// NASA's dimensionless eigenvalue clipping factor. The pinned source defaults to 1e-4.
    pub fclip: f64,
    /// Relative symmetry tolerance applied before the deterministic eigensolve.
    pub symmetry_relative_tolerance: f64,
}

impl Default for NasaPeakOverlapConfig {
    fn default() -> Self {
        Self {
            fclip: 1.0e-4,
            symmetry_relative_tolerance: 1.0e-12,
        }
    }
}

/// Fixed-time overlap using the pinned NASA PeakOverlapPos eigenvalue-floor semantics.
///
/// This is intentionally separate from the strict gaussian_position_overlap function,
/// which never repairs covariance. The compatibility path applies the same
/// Lclip = (HBR * Fclip)^2 floor to both object position covariances and to the
/// summed relative precision matrix, as the pinned CARA source does. Its report
/// preserves all three raw/remediated eigenvalue sets and each clipping decision.
///
/// This implements only the fixed-time Gaussian overlap step. It does not perform
/// the iterative center-of-linearity update, orbital-energy adjustment,
/// equinoctial conversion, covariance/Jacobian re-evaluation, or oscillation logic.
pub fn nasa_peak_overlap_compatibility(
    primary: GaussianPositionDistribution3,
    secondary: GaussianPositionDistribution3,
    hard_body_radius: f64,
    config: NasaPeakOverlapConfig,
) -> Result<NasaPeakOverlapReport, KernelError> {
    if !finite_vector(primary.mean_position)
        || !finite_vector(secondary.mean_position)
        || !hard_body_radius.is_finite()
    {
        return Err(KernelError::NonFiniteValue);
    }
    if hard_body_radius <= 0.0
        || !config.fclip.is_finite()
        || config.fclip < 0.0
        || !config.symmetry_relative_tolerance.is_finite()
        || config.symmetry_relative_tolerance < 0.0
        || config.symmetry_relative_tolerance > 1.0e-6
    {
        return Err(KernelError::InvalidNasaPeakOverlapConfig);
    }

    let scaled_floor = hard_body_radius * config.fclip;
    let clip_floor = scaled_floor * scaled_floor;
    if !scaled_floor.is_finite() || !clip_floor.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let primary_remediation = nasa_covariance_eigenvalue_clip(
        primary.covariance,
        clip_floor,
        config.symmetry_relative_tolerance,
    )?;
    let secondary_remediation = nasa_covariance_eigenvalue_clip(
        secondary.covariance,
        clip_floor,
        config.symmetry_relative_tolerance,
    )?;

    let raw_precision_sum = add_matrix(
        primary_remediation.remediated_covariance_inverse,
        secondary_remediation.remediated_covariance_inverse,
    );
    if !finite_matrix(raw_precision_sum) {
        return Err(KernelError::NumericalOverflow);
    }
    // CARA applies the same eigenvalue floor to the summed precision matrix
    // before inverting it to obtain the overlap covariance. This is not the
    // same operation as clipping the overlap covariance itself.
    let precision_remediation = nasa_covariance_eigenvalue_clip(
        raw_precision_sum,
        clip_floor,
        config.symmetry_relative_tolerance,
    )?;
    let overlap_covariance = precision_remediation.remediated_covariance_inverse;

    let primary_precision_mean = matrix_vector(
        primary_remediation.remediated_covariance_inverse,
        primary.mean_position,
    );
    let secondary_precision_mean = matrix_vector(
        secondary_remediation.remediated_covariance_inverse,
        secondary.mean_position,
    );
    let overlap_position = matrix_vector(
        overlap_covariance,
        add_vector(primary_precision_mean, secondary_precision_mean),
    );
    if !finite_vector(overlap_position) {
        return Err(KernelError::NumericalOverflow);
    }

    let hbr_squared = hard_body_radius * hard_body_radius;
    if !hbr_squared.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    let mut remediated_metric_covariance = overlap_covariance;
    for (axis, row) in remediated_metric_covariance.iter_mut().enumerate() {
        row[axis] += hbr_squared;
    }
    let (remediated_metric_inverse, remediated_metric_condition) =
        inverse_and_condition(remediated_metric_covariance)?;

    Ok(NasaPeakOverlapReport {
        overlap_position,
        overlap_covariance,
        remediated_metric_covariance,
        remediated_metric_inverse,
        primary_remediation,
        secondary_remediation,
        precision_remediation,
        remediated_metric_condition,
        hard_body_radius,
        fclip: config.fclip,
        clip_floor,
    })
}

/// Apply the pinned CARA CovRemEigValClip eigenvalue-floor operation to a 3x3 covariance.
///
/// The input is symmetrized only after its asymmetry has passed the declared
/// tolerance. The definiteness status is captured before clipping. The function
/// requires every remediated eigenvalue to be strictly positive because this
/// Rust report always includes an inverse; a zero floor that leaves a zero
/// eigenvalue therefore fails closed instead of returning an infinite inverse.
pub fn nasa_covariance_eigenvalue_clip(
    raw_covariance: Matrix3,
    clip_floor: f64,
    symmetry_relative_tolerance: f64,
) -> Result<NasaCovarianceClipReport, KernelError> {
    if !finite_matrix(raw_covariance) || !clip_floor.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }
    if clip_floor < 0.0
        || !symmetry_relative_tolerance.is_finite()
        || symmetry_relative_tolerance < 0.0
        || symmetry_relative_tolerance > 1.0e-6
    {
        return Err(KernelError::InvalidNasaPeakOverlapConfig);
    }

    let (symmetric_raw, _) = checked_symmetric(raw_covariance, symmetry_relative_tolerance)?;
    let (raw_eigenvalues, eigenvectors) = symmetric_eigendecomposition(symmetric_raw)?;
    let minimum = raw_eigenvalues[0];
    let input_definiteness = if minimum < 0.0 {
        CovarianceDefiniteness::Indefinite
    } else if minimum == 0.0 {
        CovarianceDefiniteness::PositiveSemidefinite
    } else {
        CovarianceDefiniteness::PositiveDefinite
    };

    let remediated_eigenvalues = raw_eigenvalues.map(|value| value.max(clip_floor));
    let clipping_applied = raw_eigenvalues
        .iter()
        .any(|value| *value < clip_floor);
    if remediated_eigenvalues.iter().any(|value| *value <= 0.0) {
        return Err(KernelError::NotPositiveDefinite);
    }

    let remediated_covariance = if clipping_applied {
        reconstruct_symmetric(eigenvectors, remediated_eigenvalues)?
    } else {
        symmetric_raw
    };
    let inverse_eigenvalues = remediated_eigenvalues.map(|value| value.recip());
    if inverse_eigenvalues.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NumericalOverflow);
    }
    let remediated_covariance_inverse =
        reconstruct_symmetric(eigenvectors, inverse_eigenvalues)?;
    let remediated_log_determinant: f64 =
        remediated_eigenvalues.iter().map(|value| value.ln()).sum();
    if !remediated_log_determinant.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    let covariance_infinity_norm = infinity_norm(remediated_covariance);
    let inverse_infinity_norm = infinity_norm(remediated_covariance_inverse);
    let condition_estimate = covariance_infinity_norm * inverse_infinity_norm;
    if !condition_estimate.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    Ok(NasaCovarianceClipReport {
        raw_eigenvalues,
        remediated_eigenvalues,
        eigenvectors,
        input_definiteness,
        clipping_applied,
        clip_floor,
        remediated_covariance,
        remediated_covariance_inverse,
        remediated_log_determinant,
        remediated_condition: CovarianceConditionDiagnostics {
            covariance_infinity_norm,
            inverse_infinity_norm,
            condition_estimate,
        },
    })
}

fn symmetric_eigendecomposition(
    mut matrix: Matrix3,
) -> Result<([f64; 3], Matrix3), KernelError> {
    let mut eigenvectors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut converged = false;

    for _ in 0..96 {
        let mut p = 0;
        let mut q = 1;
        let mut maximum = matrix[0][1].abs();
        for (i, j) in [(0, 2), (1, 2)] {
            if matrix[i][j].abs() > maximum {
                maximum = matrix[i][j].abs();
                p = i;
                q = j;
            }
        }
        let scale = max_abs_matrix(matrix);
        if maximum <= 32.0 * f64::EPSILON * scale.max(f64::MIN_POSITIVE) {
            converged = true;
            break;
        }

        let off_diagonal = matrix[p][q];
        let tau = (matrix[q][q] - matrix[p][p]) / (2.0 * off_diagonal);
        if !tau.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        let t = if tau >= 0.0 {
            1.0 / (tau + tau.hypot(1.0))
        } else {
            -1.0 / (-tau + tau.hypot(1.0))
        };
        let cosine = 1.0 / (1.0 + t * t).sqrt();
        let sine = t * cosine;
        let diagonal_shift = t * off_diagonal;
        matrix[p][p] -= diagonal_shift;
        matrix[q][q] += diagonal_shift;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;

        for k in 0..3 {
            if k != p && k != q {
                let old_kp = matrix[k][p];
                let old_kq = matrix[k][q];
                let new_kp = cosine * old_kp - sine * old_kq;
                let new_kq = sine * old_kp + cosine * old_kq;
                matrix[k][p] = new_kp;
                matrix[p][k] = new_kp;
                matrix[k][q] = new_kq;
                matrix[q][k] = new_kq;
            }

            let old_vp = eigenvectors[k][p];
            let old_vq = eigenvectors[k][q];
            eigenvectors[k][p] = cosine * old_vp - sine * old_vq;
            eigenvectors[k][q] = sine * old_vp + cosine * old_vq;
        }

        if !finite_matrix(matrix) || !finite_matrix(eigenvectors) {
            return Err(KernelError::NumericalOverflow);
        }
    }

    if !converged {
        let mut maximum: f64 = 0.0;
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            maximum = maximum.max(matrix[i][j].abs());
        }
        let scale = max_abs_matrix(matrix).max(f64::MIN_POSITIVE);
        if maximum > 128.0 * f64::EPSILON * scale {
            return Err(KernelError::EigenDecompositionDidNotConverge);
        }
    }

    let mut eigenvalues = [matrix[0][0], matrix[1][1], matrix[2][2]];
    // Deterministic ascending eigenvalue order; eigenvector columns follow.
    for i in 0..3 {
        for j in (i + 1)..3 {
            if eigenvalues[j] < eigenvalues[i] {
                eigenvalues.swap(i, j);
                for row in &mut eigenvectors {
                    row.swap(i, j);
                }
            }
        }
    }
    if eigenvalues.iter().any(|value| !value.is_finite()) || !finite_matrix(eigenvectors) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok((eigenvalues, eigenvectors))
}

fn reconstruct_symmetric(
    eigenvectors: Matrix3,
    eigenvalues: [f64; 3],
) -> Result<Matrix3, KernelError> {
    let mut matrix = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            let mut value = 0.0;
            for axis in 0..3 {
                value += eigenvectors[row][axis]
                    * eigenvalues[axis]
                    * eigenvectors[column][axis];
            }
            matrix[row][column] = value;
        }
    }
    if !finite_matrix(matrix) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(matrix)
}

/// Fixed-time NASA-compatible overlap result, with every covariance floor preserved.
#[derive(Debug, Clone, PartialEq)]
pub struct NasaPeakOverlapReport {
    /// Product-Gaussian overlap position.
    pub overlap_position: [f64; 3],
    /// Product covariance after NASA's clipping of the summed precision matrix.
    pub overlap_covariance: Matrix3,
    /// HBR-remediated convergence covariance: overlap covariance + HBR² I.
    pub remediated_metric_covariance: Matrix3,
    /// Inverse of the HBR-remediated convergence covariance.
    pub remediated_metric_inverse: Matrix3,
    /// Eigenvalue-clipping evidence for the primary position covariance.
    pub primary_remediation: NasaCovarianceClipReport,
    /// Eigenvalue-clipping evidence for the secondary position covariance.
    pub secondary_remediation: NasaCovarianceClipReport,
    /// Eigenvalue-clipping evidence for the summed relative precision matrix.
    pub precision_remediation: NasaCovarianceClipReport,
    /// Condition diagnostics for the HBR-remediated metric.
    pub remediated_metric_condition: CovarianceConditionDiagnostics,
    /// Hard-body radius in the same length units as the positions.
    pub hard_body_radius: f64,
    /// Dimensionless eigenvalue clipping factor from the NASA parameters.
    pub fclip: f64,
    /// Lclip = (HBR * Fclip)^2, applied to both source covariances and their precision sum.
    pub clip_floor: f64,
};

#[cfg(test)]
mod tests {
    use super::*;

    fn covariance(values: [f64; 3]) -> Matrix3 {
        [
            [values[0], 0.0, 0.0],
            [0.0, values[1], 0.0],
            [0.0, 0.0, values[2]],
        ]
    }

    fn distribution(mean: [f64; 3], covariance: Matrix3) -> GaussianPositionDistribution3 {
        GaussianPositionDistribution3 {
            mean_position: mean,
            covariance,
        }
    }

    fn config() -> GaussianOverlapConfig {
        GaussianOverlapConfig::default()
    }

    #[test]
    fn equal_isotropic_inputs_have_midpoint_and_half_covariance() {
        let primary = distribution([0.0, 0.0, 0.0], covariance([4.0, 4.0, 4.0]));
        let secondary = distribution([2.0, 4.0, 6.0], covariance([4.0, 4.0, 4.0]));
        let result = gaussian_position_overlap(primary, secondary, 1.0, config())
            .expect("valid equal isotropic distributions");
        assert_eq!(result.overlap_position, [1.0, 2.0, 3.0]);
        assert_eq!(result.overlap_covariance, covariance([2.0, 2.0, 2.0]));
        assert_eq!(result.remediated_metric_covariance, covariance([3.0, 3.0, 3.0]));
        assert_eq!(result.policy, GaussianOverlapCovariancePolicy::StrictSpd);
    }

    #[test]
    fn unequal_isotropic_inputs_use_inverse_variance_weighting() {
        let primary = distribution([0.0; 3], covariance([1.0, 1.0, 1.0]));
        let secondary = distribution([10.0; 3], covariance([4.0, 4.0, 4.0]));
        let result = gaussian_position_overlap(primary, secondary, 0.5, config())
            .expect("valid unequal isotropic distributions");
        assert!((result.overlap_position[0] - 2.0).abs() < 1.0e-13);
        assert_eq!(result.overlap_covariance, covariance([0.8, 0.8, 0.8]));
        assert_eq!(result.remediated_metric_covariance, covariance([1.05, 1.05, 1.05]));
    }

    #[test]
    fn hbr_metric_and_overlap_mahalanobis_are_distinct_from_raw_overlap_covariance() {
        let primary = distribution([0.0; 3], covariance([1.0, 2.0, 3.0]));
        let secondary = distribution([2.0, 0.0, 0.0], covariance([1.0, 2.0, 3.0]));
        let result = gaussian_position_overlap(primary, secondary, 2.0, config())
            .expect("valid covariance");
        assert_eq!(result.overlap_covariance, covariance([0.5, 1.0, 1.5]));
        assert_eq!(result.remediated_metric_covariance, covariance([4.5, 5.0, 5.5]));
        let md2 = overlap_step_mahalanobis_squared(
            [0.0, 0.0, 0.0],
            [3.0, 0.0, 0.0],
            result.remediated_metric_covariance,
            config(),
        )
        .expect("valid remediated metric");
        assert!((md2 - 9.0 / 4.5).abs() < 1.0e-13);
    }

    #[test]
    fn covariance_permutation_preserves_overlap_solution() {
        let primary_cov = [[2.0, 0.2, 0.0], [0.2, 3.0, 0.1], [0.0, 0.1, 4.0]];
        let secondary_cov = [[4.0, 0.0, 0.1], [0.0, 5.0, 0.2], [0.1, 0.2, 6.0]];
        let primary = distribution([0.5, -0.2, 1.0], primary_cov);
        let secondary = distribution([-0.5, 0.8, 2.0], secondary_cov);
        let original = gaussian_position_overlap(primary, secondary, 0.4, config())
            .expect("original coordinates");
        let permute = |matrix: Matrix3| {
            let order = [1, 2, 0];
            let mut output = [[0.0; 3]; 3];
            for (i, row) in output.iter_mut().enumerate() {
                for (j, value) in row.iter_mut().enumerate() {
                    *value = matrix[order[i]][order[j]];
                }
            }
            output
        };
        let vector = |x: [f64; 3]| [x[1], x[2], x[0]];
        let transformed = gaussian_position_overlap(
            distribution(vector(primary.mean_position), permute(primary.covariance)),
            distribution(vector(secondary.mean_position), permute(secondary.covariance)),
            0.4,
            config(),
        )
        .expect("permuted coordinates");
        let expected = vector(original.overlap_position);
        for (actual, expected) in transformed.overlap_position.iter().zip(expected.iter()) {
            assert!((*actual - *expected).abs() < 1.0e-12);
        }
    }

    #[test]
    fn non_symmetric_singular_and_indefinite_covariances_fail_closed() {
        let mut nonsymmetric = covariance([1.0; 3]);
        nonsymmetric[0][1] = 0.1;
        assert_eq!(
            gaussian_position_overlap(
                distribution([0.0; 3], nonsymmetric),
                distribution([0.0; 3], covariance([1.0; 3])),
                1.0,
                config()
            ),
            Err(KernelError::NonSymmetricMatrix)
        );

        assert_eq!(
            gaussian_position_overlap(
                distribution([0.0; 3], covariance([0.0, 1.0, 1.0])),
                distribution([0.0; 3], covariance([1.0; 3])),
                1.0,
                config()
            ),
            Err(KernelError::NotPositiveDefinite)
        );

        assert_eq!(
            gaussian_position_overlap(
                distribution([0.0; 3], covariance([-1.0, 1.0, 1.0])),
                distribution([0.0; 3], covariance([1.0; 3])),
                1.0,
                config()
            ),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn invalid_hbr_means_and_validation_config_fail_closed() {
        let valid = distribution([0.0; 3], covariance([1.0; 3]));
        assert_eq!(
            gaussian_position_overlap(valid, valid, 0.0, config()),
            Err(KernelError::InvalidHardBodyRadius)
        );
        assert_eq!(
            gaussian_position_overlap(
                distribution([f64::NAN, 0.0, 0.0], covariance([1.0; 3])),
                valid,
                1.0,
                config()
            ),
            Err(KernelError::NonFiniteValue)
        );
        let invalid_config = GaussianOverlapConfig {
            symmetry_relative_tolerance: 1.0e-3,
            ..config()
        };
        assert_eq!(
            gaussian_position_overlap(valid, valid, 1.0, invalid_config),
            Err(KernelError::InvalidGaussianOverlapConfig)
        );
    }

    #[test]
    fn ill_conditioned_covariance_is_rejected_without_regularization() {
        let primary = distribution([0.0; 3], covariance([1.0e-14, 1.0, 1.0]));
        let secondary = distribution([0.0; 3], covariance([1.0; 3]));
        assert_eq!(
            gaussian_position_overlap(primary, secondary, 1.0, config()),
            Err(KernelError::IllConditionedCovariance)
        );
    }
    #[test]
    fn non_diagonal_spd_fixture_matches_independent_precision_solution() {
        let primary_covariance = [
            [4.0, 1.0, 0.0],
            [1.0, 3.0, 0.0],
            [0.0, 0.0, 2.0],
        ];
        let secondary_covariance = [
            [2.0, 0.5, 0.0],
            [0.5, 1.0, 0.0],
            [0.0, 0.0, 5.0],
        ];
        let result = gaussian_position_overlap(
            distribution([1.0, -2.0, 0.5], primary_covariance),
            distribution([-1.0, 3.0, 2.0], secondary_covariance),
            0.25,
            config(),
        )
        .expect("known full-SPD fixture");
        let expected_mean = [-1.0 / 3.0, 1.8390804597701147, 0.9285714285714286];
        let expected_covariance = [
            [4.0 / 3.0, 1.0 / 3.0, 0.0],
            [1.0 / 3.0, 0.7471264367816092, 0.0],
            [0.0, 0.0, 10.0 / 7.0],
        ];
        for axis in 0..3 {
            assert!((result.overlap_position[axis] - expected_mean[axis]).abs() < 2.0e-12);
            for column in 0..3 {
                assert!(
                    (result.overlap_covariance[axis][column]
                        - expected_covariance[axis][column])
                        .abs()
                        < 2.0e-12
                );
            }
        }
    }

    #[test]
    fn swapping_primary_and_secondary_preserves_overlap() {
        let primary = distribution(
            [0.5, -0.2, 1.0],
            [[2.0, 0.2, 0.0], [0.2, 3.0, 0.1], [0.0, 0.1, 4.0]],
        );
        let secondary = distribution(
            [-0.5, 0.8, 2.0],
            [[4.0, 0.0, 0.1], [0.0, 5.0, 0.2], [0.1, 0.2, 6.0]],
        );
        let forward =
            gaussian_position_overlap(primary, secondary, 0.4, config()).expect("forward");
        let reverse =
            gaussian_position_overlap(secondary, primary, 0.4, config()).expect("reverse");
        for axis in 0..3 {
            assert!((forward.overlap_position[axis] - reverse.overlap_position[axis]).abs() < 1.0e-12);
            for column in 0..3 {
                assert!(
                    (forward.overlap_covariance[axis][column]
                        - reverse.overlap_covariance[axis][column])
                        .abs()
                        < 1.0e-12
                );
                assert!(
                    (forward.remediated_metric_covariance[axis][column]
                        - reverse.remediated_metric_covariance[axis][column])
                        .abs()
                        < 1.0e-12
                );
            }
        }
    }

    #[test]
    fn common_translation_moves_only_the_overlap_center() {
        let primary = distribution(
            [1.0, 2.0, 3.0],
            [[2.0, 0.2, 0.0], [0.2, 3.0, 0.1], [0.0, 0.1, 4.0]],
        );
        let secondary = distribution(
            [-2.0, 4.0, 1.0],
            [[5.0, 0.0, 0.1], [0.0, 6.0, 0.2], [0.1, 0.2, 7.0]],
        );
        let shift = [100.0, -70.0, 12.0];
        let translate = |x: [f64; 3]| [x[0] + shift[0], x[1] + shift[1], x[2] + shift[2]];
        let base = gaussian_position_overlap(primary, secondary, 0.4, config()).unwrap();
        let moved = gaussian_position_overlap(
            distribution(translate(primary.mean_position), primary.covariance),
            distribution(translate(secondary.mean_position), secondary.covariance),
            0.4,
            config(),
        )
        .unwrap();
        for axis in 0..3 {
            assert!(
                (moved.overlap_position[axis] - base.overlap_position[axis] - shift[axis]).abs()
                    < 2.0e-12
            );
            for column in 0..3 {
                assert!(
                    (moved.overlap_covariance[axis][column]
                        - base.overlap_covariance[axis][column])
                        .abs()
                        < 1.0e-12
                );
            }
        }
    }

    #[test]
    fn orthogonal_rotation_is_equivariant_for_full_covariances() {
        let primary = distribution(
            [0.2, -1.0, 3.0],
            [[2.0, 0.2, 0.0], [0.2, 1.5, 0.1], [0.0, 0.1, 3.0]],
        );
        let secondary = distribution(
            [-0.5, 2.0, 1.0],
            [[1.3, 0.1, 0.2], [0.1, 4.0, 0.0], [0.2, 0.0, 2.5]],
        );
        let rotation = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let rotate_vector = |v: [f64; 3]| [
            rotation[0][0] * v[0] + rotation[0][1] * v[1] + rotation[0][2] * v[2],
            rotation[1][0] * v[0] + rotation[1][1] * v[1] + rotation[1][2] * v[2],
            rotation[2][0] * v[0] + rotation[2][1] * v[1] + rotation[2][2] * v[2],
        ];
        let rotate_matrix = |matrix: Matrix3| {
            let mut output = [[0.0; 3]; 3];
            for row in 0..3 {
                for column in 0..3 {
                    for i in 0..3 {
                        for j in 0..3 {
                            output[row][column] +=
                                rotation[row][i] * matrix[i][j] * rotation[column][j];
                        }
                    }
                }
            }
            output
        };
        let original =
            gaussian_position_overlap(primary, secondary, 0.4, config()).expect("original");
        let rotated = gaussian_position_overlap(
            distribution(rotate_vector(primary.mean_position), rotate_matrix(primary.covariance)),
            distribution(
                rotate_vector(secondary.mean_position),
                rotate_matrix(secondary.covariance),
            ),
            0.4,
            config(),
        )
        .expect("rotated");
        let expected_mean = rotate_vector(original.overlap_position);
        let expected_covariance = rotate_matrix(original.overlap_covariance);
        for axis in 0..3 {
            assert!((rotated.overlap_position[axis] - expected_mean[axis]).abs() < 2.0e-12);
            for column in 0..3 {
                assert!(
                    (rotated.overlap_covariance[axis][column]
                        - expected_covariance[axis][column])
                        .abs()
                        < 2.0e-12
                );
            }
        }
    }

    #[test]
    fn report_preserves_configuration_and_remediated_condition_diagnostics() {
        let custom = GaussianOverlapConfig {
            symmetry_relative_tolerance: 1.0e-10,
            maximum_condition_estimate: 1.0e8,
        };
        let dist = distribution([0.0; 3], covariance([1.0, 2.0, 3.0]));
        let result = gaussian_position_overlap(dist, dist, 0.5, custom).expect("valid inputs");
        assert_eq!(result.configuration, custom);
        assert!(result.primary_condition.condition_estimate <= custom.maximum_condition_estimate);
        assert!(result.secondary_condition.condition_estimate <= custom.maximum_condition_estimate);
        assert!(result.overlap_condition.condition_estimate <= custom.maximum_condition_estimate);
        assert!(
            result.remediated_metric_condition.condition_estimate
                <= custom.maximum_condition_estimate
        );
        assert_eq!(
            result.remediated_metric_condition.covariance_infinity_norm,
            infinity_norm(result.remediated_metric_covariance)
        );
    }

    #[test]
    fn mahalanobis_helper_rejects_non_spd_remediated_covariance() {
        assert_eq!(
            overlap_step_mahalanobis_squared(
                [0.0; 3],
                [1.0, 0.0, 0.0],
                covariance([-1.0, 1.0, 1.0]),
                config(),
            ),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn mahalanobis_helper_rejects_nonsymmetric_metric() {
        let nonsymmetric = [[2.0, 0.1, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]];
        assert_eq!(
            overlap_step_mahalanobis_squared(
                [0.0; 3],
                [1.0, 0.0, 0.0],
                nonsymmetric,
                config(),
            ),
            Err(KernelError::NonSymmetricMatrix)
        );
    }

    #[test]
    fn mahalanobis_helper_enforces_the_declared_condition_limit() {
        let strict = GaussianOverlapConfig {
            maximum_condition_estimate: 1.0e10,
            ..config()
        };
        assert_eq!(
            overlap_step_mahalanobis_squared(
                [0.0; 3],
                [1.0, 0.0, 0.0],
                covariance([1.0e-14, 1.0, 1.0]),
                strict,
            ),
            Err(KernelError::IllConditionedCovariance)
        );
    }
    #[test]
    fn symmetric_jacobi_eigensolver_reconstructs_full_covariance() {
        let raw = [[4.0, 1.0, 0.0], [1.0, 3.0, 0.0], [0.0, 0.0, 2.0]];
        let (eigenvalues, eigenvectors) = symmetric_eigendecomposition(raw).unwrap();
        let reconstructed = reconstruct_symmetric(eigenvectors, eigenvalues).unwrap();
        for row in 0..3 {
            for column in 0..3 {
                assert!((reconstructed[row][column] - raw[row][column]).abs() < 1.0e-12);
            }
        }
        assert!(eigenvalues[0] <= eigenvalues[1] && eigenvalues[1] <= eigenvalues[2]);
    }

    #[test]
    fn nasa_covariance_clip_preserves_raw_and_clipped_eigenvalue_evidence() {
        let report = nasa_covariance_eigenvalue_clip(
            covariance([-0.25, 4.0, 9.0]),
            1.0,
            1.0e-12,
        )
        .expect("positive clipping floor");
        assert_eq!(report.raw_eigenvalues, [-0.25, 4.0, 9.0]);
        assert_eq!(report.remediated_eigenvalues, [1.0, 4.0, 9.0]);
        assert_eq!(report.input_definiteness, CovarianceDefiniteness::Indefinite);
        assert!(report.clipping_applied);
        assert_eq!(report.remediated_covariance, covariance([1.0, 4.0, 9.0]));
        assert_eq!(
            report.remediated_covariance_inverse,
            covariance([1.0, 0.25, 1.0 / 9.0])
        );
    }

    #[test]
    fn nasa_covariance_clip_leaves_values_at_the_floor_unchanged() {
        let report = nasa_covariance_eigenvalue_clip(
            covariance([1.0, 2.0, 3.0]),
            1.0,
            1.0e-12,
        )
        .expect("floor equality");
        assert_eq!(report.input_definiteness, CovarianceDefiniteness::PositiveDefinite);
        assert!(!report.clipping_applied);
        assert_eq!(report.raw_eigenvalues, [1.0, 2.0, 3.0]);
        assert_eq!(report.remediated_eigenvalues, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn nasa_covariance_clip_refuses_singular_inverse_when_zero_floor_is_used() {
        assert_eq!(
            nasa_covariance_eigenvalue_clip(covariance([0.0, 2.0, 3.0]), 0.0, 1.0e-12),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn nasa_peak_overlap_applies_hbr_scaled_floor_to_source_and_precision_matrices() {
        let config = NasaPeakOverlapConfig {
            fclip: 0.1,
            symmetry_relative_tolerance: 1.0e-12,
        };
        let report = nasa_peak_overlap_compatibility(
            distribution([0.0; 3], covariance([-0.25, 4.0, 9.0])),
            distribution([2.0, 4.0, 6.0], covariance([2.0, 2.0, 2.0])),
            10.0,
            config,
        )
        .expect("NASA-style positive floor");
        assert_eq!(report.clip_floor, 1.0);
        assert!(report.primary_remediation.clipping_applied);
        assert!(!report.secondary_remediation.clipping_applied);
        assert!(report.precision_remediation.clipping_applied);
        assert_eq!(
            report.primary_remediation.remediated_eigenvalues,
            [1.0, 4.0, 9.0]
        );
        assert_eq!(
            report.precision_remediation.remediated_eigenvalues,
            [1.0, 1.0, 1.5]
        );
        assert!((report.overlap_covariance[0][0] - 2.0 / 3.0).abs() < 1.0e-12);
        assert!((report.overlap_covariance[1][1] - 1.0).abs() < 1.0e-12);
        assert!((report.overlap_covariance[2][2] - 1.0).abs() < 1.0e-12);
        assert!((report.remediated_metric_covariance[0][0] - (100.0 + 2.0 / 3.0)).abs() < 1.0e-12);
        assert_eq!(report.fclip, 0.1);
    }

    #[test]
    fn nasa_peak_overlap_default_fclip_reproduces_static_isotropic_overlap() {
        let report = nasa_peak_overlap_compatibility(
            distribution([0.0; 3], covariance([4.0, 4.0, 4.0])),
            distribution([2.0, 4.0, 6.0], covariance([4.0, 4.0, 4.0])),
            1.0,
            NasaPeakOverlapConfig::default(),
        )
        .expect("default NASA compatibility parameters");
        assert_eq!(report.fclip, 1.0e-4);
        assert_eq!(report.clip_floor, 1.0e-8);
        assert_eq!(report.overlap_position, [1.0, 2.0, 3.0]);
        assert_eq!(report.overlap_covariance, covariance([2.0, 2.0, 2.0]));
        assert_eq!(report.remediated_metric_covariance, covariance([3.0, 3.0, 3.0]));
        assert!(!report.primary_remediation.clipping_applied);
        assert!(!report.secondary_remediation.clipping_applied);
        assert!(!report.precision_remediation.clipping_applied);
    }

    #[test]
    fn nasa_eigenvalue_floor_is_rotation_equivariant() {
        let angle = 0.37_f64;
        let cosine = angle.cos();
        let sine = angle.sin();
        let rotation = [
            [cosine, -sine, 0.0],
            [sine, cosine, 0.0],
            [0.0, 0.0, 1.0],
        ];
        let rotate = |matrix: Matrix3| {
            let mut result = [[0.0; 3]; 3];
            for row in 0..3 {
                for column in 0..3 {
                    for i in 0..3 {
                        for j in 0..3 {
                            result[row][column] +=
                                rotation[row][i] * matrix[i][j] * rotation[column][j];
                        }
                    }
                }
            }
            result
        };
        let original = covariance([-0.1, 2.0, 3.0]);
        let clipped = nasa_covariance_eigenvalue_clip(original, 0.5, 1.0e-12).unwrap();
        let rotated = nasa_covariance_eigenvalue_clip(rotate(original), 0.5, 1.0e-12).unwrap();
        let expected = rotate(covariance([0.5, 2.0, 3.0]));
        for row in 0..3 {
            for column in 0..3 {
                assert!((rotated.remediated_covariance[row][column] - expected[row][column]).abs() < 2.0e-12);
            }
        }
        assert_eq!(clipped.remediated_eigenvalues, [0.5, 2.0, 3.0]);
        assert_eq!(rotated.remediated_eigenvalues, [0.5, 2.0, 3.0]);
    }

    #[test]
    fn strict_overlap_and_nasa_compatibility_remediation_are_explicitly_distinct() {
        let invalid = distribution([0.0; 3], covariance([-0.25, 4.0, 9.0]));
        let valid = distribution([1.0; 3], covariance([2.0, 2.0, 2.0]));
        assert_eq!(
            gaussian_position_overlap(invalid, valid, 10.0, GaussianOverlapConfig::default()),
            Err(KernelError::NotPositiveDefinite)
        );
        let nasa = nasa_peak_overlap_compatibility(
            invalid,
            valid,
            10.0,
            NasaPeakOverlapConfig {
                fclip: 0.1,
                ..NasaPeakOverlapConfig::default()
            },
        )
        .expect("separate NASA repair policy");
        assert!(nasa.primary_remediation.clipping_applied);
        assert_eq!(
            nasa.primary_remediation.input_definiteness,
            CovarianceDefiniteness::Indefinite
        );
    }

    #[test]
    fn nasa_overlap_rejects_invalid_fclip_and_symmetry_tolerance() {
        let valid = distribution([0.0; 3], covariance([1.0; 3]));
        assert_eq!(
            nasa_peak_overlap_compatibility(
                valid,
                valid,
                1.0,
                NasaPeakOverlapConfig {
                    fclip: -1.0,
                    ..NasaPeakOverlapConfig::default()
                }
            ),
            Err(KernelError::InvalidNasaPeakOverlapConfig)
        );
        assert_eq!(
            nasa_covariance_eigenvalue_clip(covariance([1.0; 3]), 1.0, 1.0e-3),
            Err(KernelError::InvalidNasaPeakOverlapConfig)
        );
    }

}
