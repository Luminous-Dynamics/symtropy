// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact product mode for two fixed 3D Gaussian position densities.
//!
//! Given p1(x) = N(mu1, P1) and p2(x) = N(mu2, P2), their product is
//! proportional to a Gaussian with covariance
//!
//!   Sigma = (P1^-1 + P2^-1)^-1
//!
//! and mode
//!
//!   mu_star = Sigma (P1^-1 mu1 + P2^-1 mu2).
//!
//! The integral of the product is N(mu1; mu2, P1 + P2). This is the fixed-
//! Gaussian inner primitive for peak-overlap-position work. It is not the full
//! orbital PeakOverlapPos iteration, which changes the local state/covariance
//! by moving each object's center of linearization.
//!
//! The implementation uses Cholesky factorization and triangular solves.
//! Floating-point residuals are reported, but this is not a formal interval
//! or machine-verified linear-algebra implementation.

use crate::KernelError;

const SYMMETRY_TOLERANCE: f64 = 1.0e-12;

/// Two fixed Gaussian position densities to be multiplied.
///
/// Covariances are 3x3 symmetric positive definite matrices. Means and
/// covariances must use the same Cartesian frame and length unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaussianOverlapInput {
    /// Mean position of the primary Gaussian.
    pub primary_mean: [f64; 3],
    /// Covariance of the primary Gaussian.
    pub primary_covariance: [[f64; 3]; 3],
    /// Mean position of the secondary Gaussian.
    pub secondary_mean: [f64; 3],
    /// Covariance of the secondary Gaussian.
    pub secondary_covariance: [[f64; 3]; 3],
}

/// Exact fixed-Gaussian product-mode result, up to floating-point error.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaussianOverlapModeReport {
    /// Mode/mean of the normalized product Gaussian.
    pub mode_position: [f64; 3],
    /// Covariance of the normalized product Gaussian.
    pub mode_covariance: [[f64; 3]; 3],
    /// Log determinant of the primary covariance.
    pub primary_log_determinant: f64,
    /// Log determinant of the secondary covariance.
    pub secondary_log_determinant: f64,
    /// Log determinant of the product covariance.
    pub mode_covariance_log_determinant: f64,
    /// Log integral of p1(x) p2(x) over R^3.
    pub log_overlap_integral: f64,
    /// Log maximum of p1(x) p2(x), attained at mode_position.
    pub log_peak_product_density: f64,
    /// Infinity norm of the precision-system residual, normalized by the
    /// scale of the right-hand side and solution.
    pub normalized_linear_solve_residual: f64,
}

/// Compute the fixed-Gaussian product mode and overlap evidence.
///
/// This evaluates the exact Gaussian product identity using Cholesky factors.
/// The precision matrices are obtained by solving against basis vectors; the
/// mode is then solved from the summed precision system. No general-purpose
/// matrix inverse routine or determinant expansion is used.
///
/// The overlap integral identity
///
///   integral N(x;mu1,P1) N(x;mu2,P2) dx = N(mu1;mu2,P1+P2)
///
/// provides an independent normalizer. The peak product is then the overlap
/// integral multiplied by the peak density of the normalized product Gaussian.
pub fn gaussian_product_mode(
    input: GaussianOverlapInput,
) -> Result<GaussianOverlapModeReport, KernelError> {
    validate_mean(input.primary_mean)?;
    validate_mean(input.secondary_mean)?;

    let primary = validate_and_factor(input.primary_covariance)?;
    let secondary = validate_and_factor(input.secondary_covariance)?;

    let primary_precision = inverse_from_cholesky(&primary.lower)?;
    let secondary_precision = inverse_from_cholesky(&secondary.lower)?;
    let summed_precision = add_matrices(primary_precision, secondary_precision)?;
    let summed_precision_factor = cholesky(summed_precision)?;

    let primary_information = matrix_vector(primary_precision, input.primary_mean)?;
    let secondary_information = matrix_vector(secondary_precision, input.secondary_mean)?;
    let information_vector = add_vectors(primary_information, secondary_information)?;

    let mode_position = solve_cholesky(&summed_precision_factor, information_vector)?;
    let mode_covariance = inverse_from_cholesky(&summed_precision_factor)?;
    let mode_covariance = symmetrize(mode_covariance);

    let mode_covariance_log_determinant = -log_determinant_from_cholesky(&summed_precision_factor);
    let mode_covariance_log_determinant = finite(mode_covariance_log_determinant)?;

    let covariance_sum = add_matrices(input.primary_covariance, input.secondary_covariance)?;
    let covariance_sum_factor = cholesky(covariance_sum)?;
    let covariance_sum_log_determinant = log_determinant_from_cholesky(&covariance_sum_factor);

    let mean_difference = subtract_vectors(input.primary_mean, input.secondary_mean);
    let weighted_difference = solve_cholesky(&covariance_sum_factor, mean_difference)?;
    let mahalanobis_squared = dot(mean_difference, weighted_difference)?;
    if mahalanobis_squared < 0.0 {
        return Err(KernelError::NegativeQuadraticForm);
    }

    let log_overlap_integral = -1.5 * (2.0 * core::f64::consts::PI).ln()
        - 0.5 * covariance_sum_log_determinant
        - 0.5 * mahalanobis_squared;
    let log_overlap_integral = finite(log_overlap_integral)?;

    let log_peak_product_density = log_overlap_integral
        - 1.5 * (2.0 * core::f64::consts::PI).ln()
        - 0.5 * mode_covariance_log_determinant;
    let log_peak_product_density = finite(log_peak_product_density)?;

    let residual_vector = subtract_vectors(
        matrix_vector(summed_precision, mode_position)?,
        information_vector,
    );
    let residual_norm = infinity_norm(residual_vector);
    let rhs_norm = infinity_norm(information_vector);
    let solution_norm = infinity_norm(mode_position);
    let precision_norm = matrix_infinity_norm(summed_precision);
    let residual_scale = (rhs_norm + precision_norm * solution_norm).max(f64::MIN_POSITIVE);
    let normalized_linear_solve_residual = residual_norm / residual_scale;
    if !normalized_linear_solve_residual.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    Ok(GaussianOverlapModeReport {
        mode_position,
        mode_covariance,
        primary_log_determinant: primary.log_determinant,
        secondary_log_determinant: secondary.log_determinant,
        mode_covariance_log_determinant,
        log_overlap_integral,
        log_peak_product_density,
        normalized_linear_solve_residual,
    })
}

#[derive(Debug, Clone, Copy)]
struct CholeskyFactor {
    lower: [[f64; 3]; 3],
    log_determinant: f64,
}

fn validate_and_factor(matrix: [[f64; 3]; 3]) -> Result<CholeskyFactor, KernelError> {
    validate_symmetric(matrix)?;
    let matrix = symmetrize(matrix);
    let lower = cholesky(matrix)?;
    let log_determinant = log_determinant_from_cholesky(&lower);
    Ok(CholeskyFactor {
        lower,
        log_determinant,
    })
}

fn validate_mean(mean: [f64; 3]) -> Result<(), KernelError> {
    if mean.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NonFiniteValue);
    }
    Ok(())
}

fn validate_symmetric(matrix: [[f64; 3]; 3]) -> Result<(), KernelError> {
    let mut scale = 1.0_f64;
    for row in matrix {
        for value in row {
            if !value.is_finite() {
                return Err(KernelError::NonFiniteValue);
            }
            scale = scale.max(value.abs());
        }
    }
    for row in 0..3 {
        for col in (row + 1)..3 {
            if (matrix[row][col] - matrix[col][row]).abs() > SYMMETRY_TOLERANCE * scale {
                return Err(KernelError::NonSymmetricMatrix);
            }
        }
    }
    Ok(())
}

fn symmetrize(mut matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    for row in 0..3 {
        for col in (row + 1)..3 {
            let average = 0.5 * (matrix[row][col] + matrix[col][row]);
            matrix[row][col] = average;
            matrix[col][row] = average;
        }
    }
    matrix
}

fn cholesky(matrix: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], KernelError> {
    let mut lower = [[0.0_f64; 3]; 3];
    for row in 0..3 {
        for col in 0..=row {
            let mut value = matrix[row][col];
            for k in 0..col {
                value -= lower[row][k] * lower[col][k];
            }
            if row == col {
                if !value.is_finite() || value <= 0.0 {
                    return Err(KernelError::NotPositiveDefinite);
                }
                lower[row][col] = value.sqrt();
            } else {
                lower[row][col] = value / lower[col][col];
                if !lower[row][col].is_finite() {
                    return Err(KernelError::NumericalOverflow);
                }
            }
        }
    }
    Ok(lower)
}

fn inverse_from_cholesky(lower: &[[f64; 3]; 3]) -> Result<[[f64; 3]; 3], KernelError> {
    let mut inverse = [[0.0_f64; 3]; 3];
    for column in 0..3 {
        let mut basis = [0.0; 3];
        basis[column] = 1.0;
        let solution = solve_cholesky(lower, basis)?;
        for row in 0..3 {
            inverse[row][column] = solution[row];
        }
    }
    Ok(symmetrize(inverse))
}

fn solve_cholesky(
    lower: &[[f64; 3]; 3],
    right_hand_side: [f64; 3],
) -> Result<[f64; 3], KernelError> {
    let mut intermediate = [0.0_f64; 3];
    for row in 0..3 {
        let mut value = right_hand_side[row];
        for col in 0..row {
            value -= lower[row][col] * intermediate[col];
        }
        intermediate[row] = value / lower[row][row];
        if !intermediate[row].is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
    }

    let mut solution = [0.0_f64; 3];
    for row in (0..3).rev() {
        let mut value = intermediate[row];
        for col in (row + 1)..3 {
            value -= lower[col][row] * solution[col];
        }
        solution[row] = value / lower[row][row];
        if !solution[row].is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
    }
    Ok(solution)
}

fn log_determinant_from_cholesky(lower: &[[f64; 3]; 3]) -> f64 {
    2.0 * (lower[0][0].ln() + lower[1][1].ln() + lower[2][2].ln())
}

fn matrix_vector(
    matrix: [[f64; 3]; 3],
    vector: [f64; 3],
) -> Result<[f64; 3], KernelError> {
    let result = [
        dot(matrix[0], vector)?,
        dot(matrix[1], vector)?,
        dot(matrix[2], vector)?,
    ];
    if result.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(result)
}

fn dot(left: [f64; 3], right: [f64; 3]) -> Result<f64, KernelError> {
    let value = left[0].mul_add(
        right[0],
        left[1].mul_add(right[1], left[2] * right[2]),
    );
    finite(value)
}

fn add_matrices(
    left: [[f64; 3]; 3],
    right: [[f64; 3]; 3],
) -> Result<[[f64; 3]; 3], KernelError> {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for col in 0..3 {
            result[row][col] = left[row][col] + right[row][col];
            if !result[row][col].is_finite() {
                return Err(KernelError::NumericalOverflow);
            }
        }
    }
    Ok(result)
}

fn add_vectors(left: [f64; 3], right: [f64; 3]) -> Result<[f64; 3], KernelError> {
    let result = [
        left[0] + right[0],
        left[1] + right[1],
        left[2] + right[2],
    ];
    if result.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(result)
}

fn subtract_vectors(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[0] - right[0],
        left[1] - right[1],
        left[2] - right[2],
    ]
}

fn infinity_norm(vector: [f64; 3]) -> f64 {
    vector.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

fn matrix_infinity_norm(matrix: [[f64; 3]; 3]) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn finite(value: f64) -> Result<f64, KernelError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagonal(x: f64, y: f64, z: f64) -> [[f64; 3]; 3] {
        [[x, 0.0, 0.0], [0.0, y, 0.0], [0.0, 0.0, z]]
    }

    fn input(
        primary_mean: [f64; 3],
        primary_covariance: [[f64; 3]; 3],
        secondary_mean: [f64; 3],
        secondary_covariance: [[f64; 3]; 3],
    ) -> GaussianOverlapInput {
        GaussianOverlapInput {
            primary_mean,
            primary_covariance,
            secondary_mean,
            secondary_covariance,
        }
    }

    #[test]
    fn identical_isotropic_gaussians_have_shared_mode_and_half_covariance() {
        let report = gaussian_product_mode(input(
            [1.0, -2.0, 3.0],
            diagonal(4.0, 4.0, 4.0),
            [1.0, -2.0, 3.0],
            diagonal(4.0, 4.0, 4.0),
        ))
        .expect("identical Gaussian inputs");
        assert_eq!(report.mode_position, [1.0, -2.0, 3.0]);
        assert_eq!(report.mode_covariance, diagonal(2.0, 2.0, 2.0));
        assert!(report.normalized_linear_solve_residual < 1.0e-14);
    }

    #[test]
    fn unequal_diagonal_covariances_match_precision_weighted_average() {
        let report = gaussian_product_mode(input(
            [0.0, 2.0, -4.0],
            diagonal(1.0, 2.0, 4.0),
            [10.0, -2.0, 8.0],
            diagonal(9.0, 8.0, 4.0),
        ))
        .expect("unequal diagonal Gaussians");
        assert!((report.mode_position[0] - 1.0).abs() < 1.0e-14);
        assert!((report.mode_position[1] - 1.2).abs() < 1.0e-14);
        assert!((report.mode_position[2] - 2.0).abs() < 1.0e-14);
        assert!((report.mode_covariance[0][0] - 0.9).abs() < 1.0e-14);
        assert!((report.mode_covariance[1][1] - 1.6).abs() < 1.0e-14);
        assert!((report.mode_covariance[2][2] - 2.0).abs() < 1.0e-14);
    }

    #[test]
    fn exchanging_primary_and_secondary_preserves_the_product() {
        let primary_mean = [2.0, -1.0, 4.0];
        let secondary_mean = [-3.0, 5.0, 1.0];
        let primary_covariance = [[3.0, 0.2, 0.1], [0.2, 2.0, 0.3], [0.1, 0.3, 4.0]];
        let secondary_covariance = [[1.5, 0.1, 0.0], [0.1, 5.0, 0.2], [0.0, 0.2, 2.5]];
        let forward = gaussian_product_mode(input(
            primary_mean,
            primary_covariance,
            secondary_mean,
            secondary_covariance,
        ))
        .expect("forward order");
        let reverse = gaussian_product_mode(input(
            secondary_mean,
            secondary_covariance,
            primary_mean,
            primary_covariance,
        ))
        .expect("reverse order");
        for axis in 0..3 {
            assert!((forward.mode_position[axis] - reverse.mode_position[axis]).abs() < 1.0e-12);
            for component in 0..3 {
                assert!(
                    (forward.mode_covariance[axis][component]
                        - reverse.mode_covariance[axis][component])
                        .abs()
                        < 1.0e-12
                );
            }
        }
        assert!((forward.log_overlap_integral - reverse.log_overlap_integral).abs() < 1.0e-12);
        assert!(
            (forward.log_peak_product_density - reverse.log_peak_product_density).abs() < 1.0e-12
        );
    }

    #[test]
    fn log_overlap_matches_independent_sum_covariance_formula() {
        let report = gaussian_product_mode(input(
            [0.0, 0.0, 0.0],
            diagonal(2.0, 3.0, 4.0),
            [4.0, -3.0, 2.0],
            diagonal(5.0, 6.0, 7.0),
        ))
        .expect("valid Gaussian pair");
        let expected = -1.5 * (2.0 * core::f64::consts::PI).ln()
            - 0.5 * (7.0_f64 * 9.0 * 11.0).ln()
            - 0.5 * (16.0 / 7.0 + 9.0 / 9.0 + 4.0 / 11.0);
        assert!((report.log_overlap_integral - expected).abs() < 1.0e-12);
    }

    #[test]
    fn peak_log_product_matches_overlap_times_normalized_product_peak() {
        let report = gaussian_product_mode(input(
            [0.0, 0.0, 0.0],
            diagonal(1.0, 2.0, 3.0),
            [1.0, 2.0, 3.0],
            diagonal(4.0, 5.0, 6.0),
        ))
        .expect("valid Gaussian pair");
        let expected = report.log_overlap_integral
            - 1.5 * (2.0 * core::f64::consts::PI).ln()
            - 0.5 * report.mode_covariance_log_determinant;
        assert!((report.log_peak_product_density - expected).abs() < 1.0e-12);
    }

    #[test]
    fn translating_both_means_translates_the_mode_without_changing_overlap() {
        let base = input(
            [1.0, 2.0, 3.0],
            diagonal(2.0, 3.0, 4.0),
            [-2.0, 4.0, 1.0],
            diagonal(5.0, 6.0, 7.0),
        );
        let translated = input(
            [11.0, -3.0, 5.0],
            diagonal(2.0, 3.0, 4.0),
            [8.0, -1.0, 3.0],
            diagonal(5.0, 6.0, 7.0),
        );
        let first = gaussian_product_mode(base).expect("base coordinates");
        let second = gaussian_product_mode(translated).expect("translated coordinates");
        for axis in 0..3 {
            let shift = translated.primary_mean[axis] - base.primary_mean[axis];
            assert!((second.mode_position[axis] - first.mode_position[axis] - shift).abs() < 1.0e-12);
        }
        assert!((first.log_overlap_integral - second.log_overlap_integral).abs() < 1.0e-12);
    }

    #[test]
    fn rotating_means_and_covariances_rotates_the_mode_and_covariance() {
        let base = input(
            [0.2, -1.0, 3.0],
            [[2.0, 0.2, 0.0], [0.2, 1.5, 0.1], [0.0, 0.1, 3.0]],
            [-0.5, 2.0, 1.0],
            [[1.3, 0.1, 0.2], [0.1, 4.0, 0.0], [0.2, 0.0, 2.5]],
        );
        let rotation = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let rotate_vector = |v: [f64; 3]| [
            rotation[0][0] * v[0] + rotation[0][1] * v[1] + rotation[0][2] * v[2],
            rotation[1][0] * v[0] + rotation[1][1] * v[1] + rotation[1][2] * v[2],
            rotation[2][0] * v[0] + rotation[2][1] * v[1] + rotation[2][2] * v[2],
        ];
        let rotate_matrix = |matrix: [[f64; 3]; 3]| {
            let mut result = [[0.0; 3]; 3];
            for row in 0..3 {
                for col in 0..3 {
                    for i in 0..3 {
                        for j in 0..3 {
                            result[row][col] +=
                                rotation[row][i] * matrix[i][j] * rotation[col][j];
                        }
                    }
                }
            }
            result
        };
        let rotated_input = input(
            rotate_vector(base.primary_mean),
            rotate_matrix(base.primary_covariance),
            rotate_vector(base.secondary_mean),
            rotate_matrix(base.secondary_covariance),
        );
        let original = gaussian_product_mode(base).expect("original");
        let transformed = gaussian_product_mode(rotated_input).expect("rotated");
        let expected_position = rotate_vector(original.mode_position);
        let expected_covariance = rotate_matrix(original.mode_covariance);
        for row in 0..3 {
            assert!((transformed.mode_position[row] - expected_position[row]).abs() < 1.0e-12);
            for col in 0..3 {
                assert!(
                    (transformed.mode_covariance[row][col] - expected_covariance[row][col]).abs()
                        < 1.0e-12
                );
            }
        }
        assert!((original.log_overlap_integral - transformed.log_overlap_integral).abs() < 1.0e-12);
    }

    #[test]
    fn separated_means_reduce_overlap_without_moving_mode_outside_segment() {
        let report = gaussian_product_mode(input(
            [0.0, 0.0, 0.0],
            diagonal(1.0, 1.0, 1.0),
            [10.0, 0.0, 0.0],
            diagonal(1.0, 1.0, 1.0),
        ))
        .expect("separated means");
        assert!((report.mode_position[0] - 5.0).abs() < 1.0e-14);
        assert!(report.log_overlap_integral < -40.0);
    }

    #[test]
    fn non_positive_definite_or_nonsymmetric_covariances_are_rejected() {
        let mut singular = diagonal(1.0, 1.0, 0.0);
        assert_eq!(
            gaussian_product_mode(input([0.0; 3], singular, [0.0; 3], diagonal(1.0, 1.0, 1.0))),
            Err(KernelError::NotPositiveDefinite)
        );

        singular = diagonal(1.0, -1.0, 1.0);
        assert_eq!(
            gaussian_product_mode(input([0.0; 3], singular, [0.0; 3], diagonal(1.0, 1.0, 1.0))),
            Err(KernelError::NotPositiveDefinite)
        );

        let asymmetric = [[1.0, 0.1, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        assert_eq!(
            gaussian_product_mode(input(
                [0.0; 3],
                asymmetric,
                [0.0; 3],
                diagonal(1.0, 1.0, 1.0)
            )),
            Err(KernelError::NonSymmetricMatrix)
        );
    }

    #[test]
    fn non_finite_inputs_fail_closed() {
        let mut invalid = input(
            [0.0; 3],
            diagonal(1.0, 1.0, 1.0),
            [0.0; 3],
            diagonal(1.0, 1.0, 1.0),
        );
        invalid.secondary_mean[1] = f64::NAN;
        assert_eq!(
            gaussian_product_mode(invalid),
            Err(KernelError::NonFiniteValue)
        );
    }
}
