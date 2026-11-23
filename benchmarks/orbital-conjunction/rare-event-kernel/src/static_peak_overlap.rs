// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixed-time Gaussian peak-overlap position (static POP) primitive.
//!
//! Implements the exact spatial Gaussian-product initialization in NASA CARA
//! PeakOverlapPos.m before iterative orbital center-of-linearity updates.
//! For two nonsingular 3D Gaussian position PDFs:
//!
//!   Sigma_p = (A1^-1 + A2^-1)^-1
//!   mu_p = Sigma_p * (A1^-1*r1 + A2^-1*r2).
//!
//! The integral of the product of the two normalized densities is the Gaussian
//! density of r1-r2 under covariance A1+A2.
//!
//! This is the fixed-time product / static spatial overlap only. It does not
//! implement NASA's iterative orbital energy adjustment, equinoctial conversion,
//! Jacobian re-evaluation, or convergence logic.
//!
//! NASA source revision: CARA_Analysis_Tools commit
//! 43f0160f94d76a91805d86521c3e8fa0d09ecd68 (November 2026 update).

use crate::KernelError;
use core::f64::consts::TAU;

/// A three-dimensional Gaussian position distribution in a common Cartesian frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionGaussian3 {
    /// Mean position in the declared length unit.
    pub mean: [f64; 3],
    /// Symmetric positive-definite position covariance in squared length units.
    pub covariance: [[f64; 3]; 3],
}

/// Inputs to the fixed-time Gaussian peak-overlap position primitive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticPeakOverlapInputs {
    /// Primary-object position Gaussian.
    pub primary: PositionGaussian3,
    /// Secondary-object position Gaussian.
    pub secondary: PositionGaussian3,
}

/// Result of multiplying two normalized 3D Gaussian position densities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StaticPeakOverlapReport {
    /// Mean / mode of the normalized Gaussian product.
    pub overlap_position: [f64; 3],
    /// Covariance of the normalized Gaussian product.
    pub overlap_covariance: [[f64; 3]; 3],
    /// Natural logarithm of the overlap covariance determinant.
    pub overlap_log_determinant: f64,
    /// Logarithm of the integral of the two normalized position densities' product.
    ///
    /// Equals log N(primary.mean-secondary.mean; 0, A_primary+A_secondary).
    pub log_integral_of_density_product: f64,
}

/// Compute the exact fixed-time spatial Gaussian product centre and covariance.
///
/// Both inputs must describe normalized, nonsingular Gaussian position
/// densities in the same frame, epoch and length unit. Covariances are
/// validated as finite, symmetric and strictly positive definite. No
/// eigenvalue clipping, diagonal jitter or covariance repair is performed.
///
/// The calculation uses Cholesky factorizations and triangular solves to form
/// the precision matrices, product covariance and product centre.
pub fn static_peak_overlap(
    inputs: StaticPeakOverlapInputs,
) -> Result<StaticPeakOverlapReport, KernelError> {
    validate_gaussian(inputs.primary)?;
    validate_gaussian(inputs.secondary)?;

    let primary_lower = cholesky_3(inputs.primary.covariance)?;
    let secondary_lower = cholesky_3(inputs.secondary.covariance)?;
    let primary_precision = inverse_from_cholesky(primary_lower)?;
    let secondary_precision = inverse_from_cholesky(secondary_lower)?;
    let product_precision = add_3(primary_precision, secondary_precision);
    validate_finite_matrix(product_precision)?;

    let product_precision_lower = cholesky_3(product_precision)?;
    let overlap_covariance = inverse_from_cholesky(product_precision_lower)?;
    let overlap_log_determinant = -2.0
        * (product_precision_lower[0][0].ln()
            + product_precision_lower[1][1].ln()
            + product_precision_lower[2][2].ln());
    if !overlap_log_determinant.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let primary_precision_mean = matrix_vector(primary_precision, inputs.primary.mean);
    let secondary_precision_mean = matrix_vector(secondary_precision, inputs.secondary.mean);
    let precision_weighted_mean = add_vector(primary_precision_mean, secondary_precision_mean);
    let overlap_position = matrix_vector(overlap_covariance, precision_weighted_mean);
    validate_finite_vector(overlap_position)?;

    let combined_covariance = add_3(inputs.primary.covariance, inputs.secondary.covariance);
    let combined_lower = cholesky_3(combined_covariance)?;
    let relative_mean = subtract_vector(inputs.primary.mean, inputs.secondary.mean);
    let solved_relative = solve_spd_from_cholesky(combined_lower, relative_mean);
    let relative_mahalanobis = dot(relative_mean, solved_relative);
    let combined_log_determinant =
        2.0 * (combined_lower[0][0].ln() + combined_lower[1][1].ln() + combined_lower[2][2].ln());
    let log_integral_of_density_product =
        -0.5 * (3.0 * TAU.ln() + combined_log_determinant + relative_mahalanobis);
    if !relative_mahalanobis.is_finite() || !log_integral_of_density_product.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    Ok(StaticPeakOverlapReport {
        overlap_position,
        overlap_covariance,
        overlap_log_determinant,
        log_integral_of_density_product,
    })
}

fn validate_gaussian(gaussian: PositionGaussian3) -> Result<(), KernelError> {
    validate_finite_vector(gaussian.mean)?;
    validate_finite_matrix(gaussian.covariance)?;
    validate_symmetric(gaussian.covariance)?;
    let _ = cholesky_3(gaussian.covariance)?;
    Ok(())
}

fn validate_finite_vector(vector: [f64; 3]) -> Result<(), KernelError> {
    if vector.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(KernelError::NonFiniteValue)
    }
}

fn validate_finite_matrix(matrix: [[f64; 3]; 3]) -> Result<(), KernelError> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(KernelError::NonFiniteValue)
    }
}

fn validate_symmetric(matrix: [[f64; 3]; 3]) -> Result<(), KernelError> {
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    let tolerance = 256.0 * f64::EPSILON * scale;
    for i in 0..3 {
        for j in 0..i {
            if (matrix[i][j] - matrix[j][i]).abs() > tolerance {
                return Err(KernelError::NonSymmetricMatrix);
            }
        }
    }
    Ok(())
}

fn cholesky_3(matrix: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], KernelError> {
    let mut lower = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..=i {
            let mut residual = matrix[i][j];
            for k in 0..j {
                residual -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !residual.is_finite() || residual <= 0.0 {
                    return Err(KernelError::NotPositiveDefinite);
                }
                lower[i][j] = residual.sqrt();
            } else {
                lower[i][j] = residual / lower[j][j];
            }
        }
    }
    validate_finite_matrix(lower)?;
    Ok(lower)
}

fn solve_spd_from_cholesky(lower: [[f64; 3]; 3], rhs: [f64; 3]) -> [f64; 3] {
    let mut forward = [0.0; 3];
    for i in 0..3 {
        let mut value = rhs[i];
        for k in 0..i {
            value -= lower[i][k] * forward[k];
        }
        forward[i] = value / lower[i][i];
    }
    let mut result = [0.0; 3];
    for i in (0..3).rev() {
        let mut value = forward[i];
        for k in (i + 1)..3 {
            value -= lower[k][i] * result[k];
        }
        result[i] = value / lower[i][i];
    }
    result
}

fn inverse_from_cholesky(lower: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], KernelError> {
    let mut inverse = [[0.0; 3]; 3];
    for column in 0..3 {
        let mut basis = [0.0; 3];
        basis[column] = 1.0;
        let solution = solve_spd_from_cholesky(lower, basis);
        for row in 0..3 {
            inverse[row][column] = solution[row];
        }
    }
    let symmetric = symmetrize(inverse);
    validate_finite_matrix(symmetric)?;
    Ok(symmetric)
}

fn add_3(left: [[f64; 3]; 3], right: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            result[i][j] = left[i][j] + right[i][j];
        }
    }
    result
}

fn matrix_vector(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    let mut result = [0.0; 3];
    for i in 0..3 {
        for j in 0..3 {
            result[i] += matrix[i][j] * vector[j];
        }
    }
    result
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn add_vector(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn subtract_vector(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn symmetrize(matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut result = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            result[i][j] = 0.5 * (matrix[i][j] + matrix[j][i]);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gaussian(mean: [f64; 3], covariance: [[f64; 3]; 3]) -> PositionGaussian3 {
        PositionGaussian3 { mean, covariance }
    }

    fn diagonal(x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [[x, 0.0, 0.0], [0.0, y, 0.0], [0.0, 0.0, z]]
    }

    fn inputs(primary: PositionGaussian3, secondary: PositionGaussian3) -> StaticPeakOverlapInputs {
        StaticPeakOverlapInputs { primary, secondary }
    }

    #[test]
    fn identical_isotropic_covariances_produce_midpoint_and_half_covariance() {
        let primary = gaussian([1.0, 2.0, 3.0], diagonal(4.0, 4.0, 4.0));
        let secondary = gaussian([5.0, 6.0, 7.0], diagonal(4.0, 4.0, 4.0));
        let report = static_peak_overlap(inputs(primary, secondary)).expect("valid Gaussians");
        assert_eq!(report.overlap_position, [3.0, 4.0, 5.0]);
        assert_eq!(report.overlap_covariance, diagonal(2.0, 2.0, 2.0));
        let expected = -0.5 * (3.0 * TAU.ln() + (8.0_f64.powi(3)).ln() + 6.0);
        assert!((report.log_integral_of_density_product - expected).abs() < 1.0e-13);
    }

    #[test]
    fn unequal_diagonal_covariances_match_independent_closed_form() {
        let primary = gaussian([1.0, 2.0, -1.0], diagonal(1.0, 4.0, 9.0));
        let secondary = gaussian([3.0, -2.0, 5.0], diagonal(4.0, 9.0, 16.0));
        let report = static_peak_overlap(inputs(primary, secondary)).expect("valid Gaussians");
        let expected_covariance = [0.8, 36.0 / 13.0, 144.0 / 25.0];
        for i in 0..3 {
            assert!((report.overlap_covariance[i][i] - expected_covariance[i]).abs() < 1.0e-13);
        }
        let expected_mean = [
            0.8 * (1.0 + 3.0 / 4.0),
            (36.0 / 13.0) * (2.0 / 4.0 - 2.0 / 9.0),
            (144.0 / 25.0) * (-1.0 / 9.0 + 5.0 / 16.0),
        ];
        for i in 0..3 {
            assert!((report.overlap_position[i] - expected_mean[i]).abs() < 1.0e-13);
        }
    }

    #[test]
    fn overlap_is_invariant_to_swapping_primary_and_secondary() {
        let primary = gaussian(
            [0.3, -1.2, 2.4],
            [[2.0, 0.2, -0.1], [0.2, 1.5, 0.3], [-0.1, 0.3, 3.0]],
        );
        let secondary = gaussian(
            [-2.0, 0.8, 1.2],
            [[1.1, -0.1, 0.2], [-0.1, 2.2, 0.1], [0.2, 0.1, 1.8]],
        );
        let forward = static_peak_overlap(inputs(primary, secondary)).expect("forward product");
        let reverse = static_peak_overlap(inputs(secondary, primary)).expect("reverse product");
        for i in 0..3 {
            assert!((forward.overlap_position[i] - reverse.overlap_position[i]).abs() < 1.0e-13);
            for j in 0..3 {
                assert!(
                    (forward.overlap_covariance[i][j] - reverse.overlap_covariance[i][j]).abs()
                        < 1.0e-13
                );
            }
        }
        assert!(
            (forward.log_integral_of_density_product - reverse.log_integral_of_density_product)
                .abs()
                < 1.0e-13
        );
    }

    #[test]
    fn a_tighter_distribution_dominates_the_overlap_position() {
        let primary = gaussian([10.0, -4.0, 2.0], diagonal(1.0e-4, 2.0e-4, 3.0e-4));
        let secondary = gaussian([-10.0, 8.0, 5.0], diagonal(1.0e2, 2.0e2, 3.0e2));
        let report = static_peak_overlap(inputs(primary, secondary)).expect("valid Gaussians");
        for i in 0..3 {
            assert!((report.overlap_position[i] - primary.mean[i]).abs() < 1.0e-4);
        }
    }

    #[test]
    fn common_translation_moves_centre_but_not_covariance_or_overlap_mass() {
        let primary = gaussian(
            [1.0, 2.0, 3.0],
            [[2.0, 0.2, 0.0], [0.2, 3.0, 0.1], [0.0, 0.1, 1.5]],
        );
        let secondary = gaussian(
            [-1.0, 0.5, 4.0],
            [[1.0, 0.0, 0.1], [0.0, 2.0, 0.2], [0.1, 0.2, 4.0]],
        );
        let shift = [10.0, -7.0, 2.0];
        let base = static_peak_overlap(inputs(primary, secondary)).expect("base product");
        let translated = static_peak_overlap(inputs(
            gaussian(add_vector(primary.mean, shift), primary.covariance),
            gaussian(add_vector(secondary.mean, shift), secondary.covariance),
        ))
        .expect("translated product");
        for i in 0..3 {
            assert!(
                (translated.overlap_position[i] - base.overlap_position[i] - shift[i]).abs()
                    < 1.0e-12
            );
            for j in 0..3 {
                assert!(
                    (translated.overlap_covariance[i][j] - base.overlap_covariance[i][j]).abs()
                        < 1.0e-12
                );
            }
        }
        assert!(
            (translated.log_integral_of_density_product - base.log_integral_of_density_product)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn product_log_mass_matches_direct_isotropic_known_answer() {
        let primary = gaussian([0.0; 3], diagonal(1.0, 1.0, 1.0));
        let secondary = gaussian([1.0, 0.0, 0.0], diagonal(1.0, 1.0, 1.0));
        let report = static_peak_overlap(inputs(primary, secondary)).expect("valid Gaussians");
        let expected = -0.5 * (3.0 * TAU.ln() + (2.0_f64.powi(3)).ln() + 0.5);
        assert!((report.log_integral_of_density_product - expected).abs() < 1.0e-13);
    }

    #[test]
    fn nonsymmetric_or_nonpositive_covariances_fail_closed() {
        let primary = gaussian(
            [0.0; 3],
            [[1.0, 0.1, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        );
        let secondary = gaussian([0.0; 3], diagonal(1.0, 1.0, 1.0));
        assert_eq!(
            static_peak_overlap(inputs(primary, secondary)),
            Err(KernelError::NonSymmetricMatrix)
        );
        let singular = gaussian([0.0; 3], diagonal(1.0, 0.0, 1.0));
        assert_eq!(
            static_peak_overlap(inputs(singular, secondary)),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn nonfinite_input_fails_closed() {
        let primary = gaussian([f64::NAN, 0.0, 0.0], diagonal(1.0, 1.0, 1.0));
        let secondary = gaussian([0.0; 3], diagonal(1.0, 1.0, 1.0));
        assert_eq!(
            static_peak_overlap(inputs(primary, secondary)),
            Err(KernelError::NonFiniteValue)
        );
    }
}
