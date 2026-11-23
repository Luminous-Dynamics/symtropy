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
//! This module implements strict SPD validation only. It does not eigenvalue
//! clip or add covariance jitter. NASA's HBR-dependent eigenvalue-floor policy
//! is intentionally a separate future compatibility mode.

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
) -> Result<f64, KernelError> {
    if !finite_vector(previous_overlap_position)
        || !finite_vector(current_overlap_position)
        || !finite_matrix(remediated_metric_covariance)
    {
        return Err(KernelError::NonFiniteValue);
    }
    // Validate and invert the supplied covariance here. Accepting an arbitrary
    // caller-supplied inverse could make an indefinite metric look valid for
    // one particular displacement.
    let remediated_metric_inverse = inverse_spd(remediated_metric_covariance)?;
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
    if !config.symmetry_relative_tolerance.is_finite()
        || config.symmetry_relative_tolerance < 0.0
        || config.symmetry_relative_tolerance > 1.0e-6
        || !config.maximum_condition_estimate.is_finite()
        || config.maximum_condition_estimate <= 1.0
    {
        return Err(KernelError::InvalidGaussianOverlapConfig);
    }
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
            ),
            Err(KernelError::NotPositiveDefinite)
        );
    }

}
