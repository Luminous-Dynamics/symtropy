// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fixed-size conditional relative-motion covariance for the NASA Ncdot kernel.
//!
//! Given a relative position/velocity covariance block matrix
//!
//!     P = [ A   Bᵀ ]
//!         [ B    C ]
//!
//! this module constructs Q1 = B*A⁻¹ and Q2 = C - B*A⁻¹*Bᵀ. It validates the
//! position covariance and conditional velocity covariance without silent
//! eigenvalue clipping or covariance jitter.
//!
//! At a hard-body sphere direction n, it evaluates:
//!
//!     dr = R*n - mu_r
//!     mu_n = nᵀ(mu_v + Q1*dr)
//!     s² = nᵀ Q2 n
//!     MD² = drᵀ A⁻¹ dr
//!
//! The positive-variance normal expectation is delegated to the separately
//! tested scalar Gaussian positive-part kernel. NASA's non-positive-variance
//! fallback and the mathematical zero-variance limit are explicit policies.

use crate::KernelError;
use crate::normal_positive_part::gaussian_negative_part_expectation;

/// Row-major 3×3 matrix used for relative position/velocity covariance blocks.
pub type Matrix3 = [[f64; 3]; 3];

/// Relative position/velocity covariance partitioned into 3×3 blocks.
///
/// The full covariance is [A Bᵀ; B C], with position first and velocity second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativePositionVelocityCovarianceBlocks {
    /// Relative-position covariance A.
    pub position: Matrix3,
    /// Velocity-position cross-covariance B = Cov(v, r).
    pub velocity_position: Matrix3,
    /// Relative-velocity covariance C.
    pub velocity: Matrix3,
}

/// Numerical tolerances for conditional covariance validation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionalVelocityConfig {
    /// Relative symmetry tolerance for A, C and the computed Schur complement.
    /// Must not exceed 1e-6.
    pub symmetry_relative_tolerance: f64,
    /// Relative tolerance for small negative eigenvalues of Q2 from roundoff.
    ///
    /// Must not exceed 1e-6. This never repairs Q2. It only controls whether a
    /// tiny negative directional variance is treated as a zero-variance limit.
    pub conditional_psd_relative_tolerance: f64,
    /// Tolerance for checking that a supplied direction has unit norm.
    /// Must not exceed 1e-6.
    pub unit_direction_tolerance: f64,
}

const MAX_RELATIVE_VALIDATION_TOLERANCE: f64 = 1.0e-6;

impl Default for ConditionalVelocityConfig {
    fn default() -> Self {
        Self {
            symmetry_relative_tolerance: 1.0e-12,
            conditional_psd_relative_tolerance: 1.0e-12,
            unit_direction_tolerance: 1.0e-10,
        }
    }
}

/// Selects the treatment of zero or roundoff-level conditional normal variance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZeroConditionalVarianceMode {
    /// Reproduce NASA CARA Ncdot_integrand.m exactly:
    /// max(0, -mu_vᵀ n), omitting Q1*dr in this fallback.
    NasaCompatibility,
    /// Use max(0, -mu_n), the mathematical zero-variance limit of the
    /// positive-variance conditional Gaussian expression.
    MathematicalZeroVarianceLimit,
}

/// Construction diagnostics for the conditional velocity distribution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionalVelocityDiagnostics {
    /// Maximum absolute asymmetry in A before roundoff symmetrisation.
    pub position_symmetry_residual: f64,
    /// Maximum absolute asymmetry in C before roundoff symmetrisation.
    pub velocity_symmetry_residual: f64,
    /// Maximum absolute asymmetry in the computed Q2 before roundoff symmetrisation.
    pub conditional_symmetry_residual: f64,
    /// Minimum eigenvalue of the symmetrised Q2 estimate.
    ///
    /// A small negative value within the declared relative tolerance is not
    /// repaired; directional evaluations in that tolerance band use the
    /// explicit zero-variance policy.
    pub minimum_conditional_eigenvalue: f64,
    /// Relative tolerance used for the Q2 positive-semidefinite check.
    pub conditional_psd_relative_tolerance: f64,
}

/// Validated conditional relative-velocity distribution at a fixed epoch.
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalVelocityDistribution {
    position_inverse: Matrix3,
    regression: Matrix3,
    conditional_velocity_covariance: Matrix3,
    conditional_scale: f64,
    config: ConditionalVelocityConfig,
    diagnostics: ConditionalVelocityDiagnostics,
}

/// Per-direction conditional motion and Ncdot pointwise quantities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConditionalDirectionReport {
    /// Supplied unit direction on the hard-body sphere.
    pub direction: [f64; 3],
    /// Offset from mean relative position to the sphere point.
    pub position_offset: [f64; 3],
    /// Conditional mean inward-normal coordinate before truncation.
    pub conditional_normal_mean: f64,
    /// Conditional variance along the sphere normal.
    pub conditional_normal_variance: f64,
    /// Expected positive inward normal speed.
    pub expected_inward_normal_speed: f64,
    /// Log of the expected positive inward normal speed.
    pub log_expected_inward_normal_speed: f64,
    /// Squared Mahalanobis distance from the position-Gaussian mean.
    pub mahalanobis_distance_squared: f64,
    /// Log of the unnormalised Ncdot sphere integrand.
    pub log_integrand: f64,
    /// Ncdot sphere integrand in f64; may underflow to zero.
    pub integrand: f64,
    /// True when the integrand underflowed despite a finite log value.
    pub integrand_underflowed: bool,
    /// True when the zero-variance branch was used.
    pub zero_variance_fallback_used: bool,
    /// Zero-variance interpretation used when the fallback was selected.
    pub zero_variance_mode: ZeroConditionalVarianceMode,
}

/// Construct Q1 and Q2 from the relative 6×6 covariance blocks.
///
/// A must be symmetric positive definite. C and the computed Q2 must be
/// symmetric within the supplied relative tolerance. Q2 must be positive
/// semidefinite within the supplied tolerance. No covariance jitter,
/// eigenvalue clipping or other remediation is performed.
impl ConditionalVelocityDistribution {
    /// Validate covariance blocks and construct the conditional velocity model.
    pub fn new(
        blocks: RelativePositionVelocityCovarianceBlocks,
        config: ConditionalVelocityConfig,
    ) -> Result<Self, KernelError> {
        validate_config(config)?;
        validate_finite_matrix(&blocks.position)?;
        validate_finite_matrix(&blocks.velocity_position)?;
        validate_finite_matrix(&blocks.velocity)?;

        let (position, position_symmetry_residual) =
            checked_symmetric(blocks.position, config.symmetry_relative_tolerance)?;
        let (velocity, velocity_symmetry_residual) =
            checked_symmetric(blocks.velocity, config.symmetry_relative_tolerance)?;

        let cholesky = cholesky_3x3(position)?;
        let position_inverse = inverse_from_cholesky(cholesky)?;
        let regression = multiply_3x3(blocks.velocity_position, position_inverse);
        let regression_times_cross_transpose =
            multiply_3x3(regression, transpose_3x3(blocks.velocity_position));
        let raw_conditional = subtract_3x3(velocity, regression_times_cross_transpose);
        let (conditional_velocity_covariance, conditional_symmetry_residual) =
            checked_symmetric(raw_conditional, config.symmetry_relative_tolerance)?;

        let minimum_conditional_eigenvalue =
            minimum_symmetric_eigenvalue(conditional_velocity_covariance)?;
        let conditional_scale = max_abs_matrix(conditional_velocity_covariance);
        let conditional_scale = if conditional_scale == 0.0 {
            1.0
        } else {
            conditional_scale
        };
        let allowed_negative = config.conditional_psd_relative_tolerance * conditional_scale;
        if minimum_conditional_eigenvalue < -allowed_negative {
            return Err(KernelError::NotPositiveDefinite);
        }

        Ok(Self {
            position_inverse,
            regression,
            conditional_velocity_covariance,
            conditional_scale,
            config,
            diagnostics: ConditionalVelocityDiagnostics {
                position_symmetry_residual,
                velocity_symmetry_residual,
                conditional_symmetry_residual,
                minimum_conditional_eigenvalue,
                conditional_psd_relative_tolerance: config.conditional_psd_relative_tolerance,
            },
        })
    }

    /// Return Q1 = B*A⁻¹.
    pub fn regression_matrix(&self) -> Matrix3 {
        self.regression
    }

    /// Return Q2 = C - B*A⁻¹*Bᵀ.
    pub fn conditional_velocity_covariance(&self) -> Matrix3 {
        self.conditional_velocity_covariance
    }

    /// Return A⁻¹.
    pub fn position_covariance_inverse(&self) -> Matrix3 {
        self.position_inverse
    }

    /// Return covariance construction diagnostics.
    pub fn diagnostics(&self) -> ConditionalVelocityDiagnostics {
        self.diagnostics
    }

    /// Evaluate one sphere direction for fixed mean relative position/velocity.
    ///
    /// log_position_normalizer is NASA's logZ term: Ncdot uses
    /// exp(-logZ - MD²/2) multiplied by the expected inward normal speed.
    pub fn evaluate_direction(
        &self,
        mean_relative_position: [f64; 3],
        mean_relative_velocity: [f64; 3],
        hard_body_radius: f64,
        direction: [f64; 3],
        log_position_normalizer: f64,
        zero_variance_mode: ZeroConditionalVarianceMode,
    ) -> Result<ConditionalDirectionReport, KernelError> {
        if !all_finite_3(mean_relative_position)
            || !all_finite_3(mean_relative_velocity)
            || !all_finite_3(direction)
            || !hard_body_radius.is_finite()
            || !log_position_normalizer.is_finite()
        {
            return Err(KernelError::NonFiniteValue);
        }
        if hard_body_radius <= 0.0 {
            return Err(KernelError::InvalidHardBodyRadius);
        }

        let direction_norm_squared = dot(direction, direction);
        if (direction_norm_squared - 1.0).abs() > self.config.unit_direction_tolerance {
            return Err(KernelError::InvalidUnitDirection);
        }

        let sphere_position = [
            hard_body_radius * direction[0],
            hard_body_radius * direction[1],
            hard_body_radius * direction[2],
        ];
        let position_offset = subtract_vector(sphere_position, mean_relative_position);
        let conditional_velocity_offset = matrix_vector(self.regression, position_offset);
        let conditional_mean_velocity = [
            mean_relative_velocity[0] + conditional_velocity_offset[0],
            mean_relative_velocity[1] + conditional_velocity_offset[1],
            mean_relative_velocity[2] + conditional_velocity_offset[2],
        ];
        let conditional_normal_mean = dot(direction, conditional_mean_velocity);
        let variance_times_direction =
            matrix_vector(self.conditional_velocity_covariance, direction);
        let conditional_normal_variance = dot(direction, variance_times_direction);
        if !conditional_normal_mean.is_finite() || !conditional_normal_variance.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }

        let variance_tolerance =
            self.config.conditional_psd_relative_tolerance * self.conditional_scale;
        if conditional_normal_variance < -variance_tolerance {
            return Err(KernelError::NotPositiveDefinite);
        }

        let (expected_inward_normal_speed, log_expected_inward_normal_speed, fallback_used) =
            if conditional_normal_variance <= 0.0 {
                let inward_speed = match zero_variance_mode {
                    ZeroConditionalVarianceMode::NasaCompatibility => {
                        (-dot(mean_relative_velocity, direction)).max(0.0)
                    }
                    ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit => {
                        (-conditional_normal_mean).max(0.0)
                    }
                };
                let log_speed = if inward_speed > 0.0 {
                    inward_speed.ln()
                } else {
                    f64::NEG_INFINITY
                };
                (inward_speed, log_speed, true)
            } else {
                let speed = gaussian_negative_part_expectation(
                    conditional_normal_mean,
                    conditional_normal_variance.sqrt(),
                )?;
                (speed.expectation, speed.log_expectation, false)
            };

        let inverse_position_offset = matrix_vector(self.position_inverse, position_offset);
        let mut mahalanobis_distance_squared = dot(position_offset, inverse_position_offset);
        if !mahalanobis_distance_squared.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        if mahalanobis_distance_squared < -1.0e-10 {
            return Err(KernelError::NotPositiveDefinite);
        }
        mahalanobis_distance_squared = mahalanobis_distance_squared.max(0.0);

        let log_integrand = if log_expected_inward_normal_speed.is_finite() {
            log_expected_inward_normal_speed
                - log_position_normalizer
                - 0.5 * mahalanobis_distance_squared
        } else {
            f64::NEG_INFINITY
        };
        if log_integrand.is_nan() {
            return Err(KernelError::NumericalOverflow);
        }
        let integrand = log_integrand.exp();
        if !integrand.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }

        Ok(ConditionalDirectionReport {
            direction,
            position_offset,
            conditional_normal_mean,
            conditional_normal_variance,
            expected_inward_normal_speed,
            log_expected_inward_normal_speed,
            mahalanobis_distance_squared,
            log_integrand,
            integrand,
            integrand_underflowed: integrand == 0.0 && log_integrand.is_finite(),
            zero_variance_fallback_used: fallback_used,
            zero_variance_mode,
        })
    }
}

fn validate_config(config: ConditionalVelocityConfig) -> Result<(), KernelError> {
    let tolerances = [
        config.symmetry_relative_tolerance,
        config.conditional_psd_relative_tolerance,
        config.unit_direction_tolerance,
    ];
    if tolerances.iter().any(|value| {
        !value.is_finite() || *value < 0.0 || *value > MAX_RELATIVE_VALIDATION_TOLERANCE
    }) {
        return Err(KernelError::InvalidCovarianceTolerance);
    }
    Ok(())
}

fn validate_finite_matrix(matrix: &Matrix3) -> Result<(), KernelError> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        Err(KernelError::NonFiniteValue)
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
    for i in 0..3 {
        for j in (i + 1)..3 {
            maximum_residual = maximum_residual.max((matrix[i][j] - matrix[j][i]).abs());
        }
    }
    if maximum_residual > relative_tolerance * scale {
        return Err(KernelError::NonSymmetricMatrix);
    }

    let mut symmetric = matrix;
    for i in 0..3 {
        for j in (i + 1)..3 {
            let value = 0.5 * (matrix[i][j] + matrix[j][i]);
            symmetric[i][j] = value;
            symmetric[j][i] = value;
        }
    }
    Ok((symmetric, maximum_residual))
}

fn cholesky_3x3(matrix: Matrix3) -> Result<Matrix3, KernelError> {
    let mut lower = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for k in 0..column {
                value -= lower[row][k] * lower[column][k];
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

fn inverse_from_cholesky(lower: Matrix3) -> Result<Matrix3, KernelError> {
    let mut inverse = [[0.0; 3]; 3];
    for column in 0..3 {
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
        for row in 0..3 {
            inverse[row][column] = solution[row];
        }
    }
    if inverse.iter().flatten().any(|value| !value.is_finite()) {
        return Err(KernelError::NumericalOverflow);
    }

    // Matrix inversion from a symmetric Cholesky factor should be symmetric;
    // only remove the last-bit solve asymmetry here.
    for i in 0..3 {
        for j in (i + 1)..3 {
            let value = 0.5 * (inverse[i][j] + inverse[j][i]);
            inverse[i][j] = value;
            inverse[j][i] = value;
        }
    }
    Ok(inverse)
}

fn minimum_symmetric_eigenvalue(mut matrix: Matrix3) -> Result<f64, KernelError> {
    for _ in 0..48 {
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
        let scale = max_abs_matrix(matrix).max(f64::MIN_POSITIVE);
        if maximum <= 16.0 * f64::EPSILON * scale {
            break;
        }

        let off_diagonal = matrix[p][q];
        let tau = (matrix[q][q] - matrix[p][p]) / (2.0 * off_diagonal);
        let t = if tau >= 0.0 {
            1.0 / (tau + tau.hypot(1.0))
        } else {
            -1.0 / (-tau + tau.hypot(1.0))
        };
        let cosine = 1.0 / (1.0 + t * t).sqrt();
        let sine = t * cosine;
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        matrix[p][p] = app - t * off_diagonal;
        matrix[q][q] = aqq + t * off_diagonal;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;

        for k in 0..3 {
            if k != p && k != q {
                let akp = matrix[k][p];
                let akq = matrix[k][q];
                matrix[k][p] = cosine * akp - sine * akq;
                matrix[p][k] = matrix[k][p];
                matrix[k][q] = sine * akp + cosine * akq;
                matrix[q][k] = matrix[k][q];
            }
        }
    }

    let eigenvalue = matrix[0][0].min(matrix[1][1]).min(matrix[2][2]);
    if !eigenvalue.is_finite() {
        Err(KernelError::NumericalOverflow)
    } else {
        Ok(eigenvalue)
    }
}

fn multiply_3x3(left: Matrix3, right: Matrix3) -> Matrix3 {
    let mut result = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            result[i][j] = (0..3).map(|k| left[i][k] * right[k][j]).sum();
        }
    }
    result
}

fn transpose_3x3(matrix: Matrix3) -> Matrix3 {
    let mut result = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            result[i][j] = matrix[j][i];
        }
    }
    result
}

fn subtract_3x3(left: Matrix3, right: Matrix3) -> Matrix3 {
    let mut result = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            result[i][j] = left[i][j] - right[i][j];
        }
    }
    result
}

fn matrix_vector(matrix: Matrix3, vector: [f64; 3]) -> [f64; 3] {
    let mut result = [0.0; 3];
    for i in 0..3 {
        result[i] = matrix[i][0] * vector[0] + matrix[i][1] * vector[1] + matrix[i][2] * vector[2];
    }
    result
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0].mul_add(right[0], left[1].mul_add(right[1], left[2] * right[2]))
}

fn subtract_vector(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn all_finite_3(vector: [f64; 3]) -> bool {
    vector.iter().all(|value| value.is_finite())
}

fn max_abs_matrix(matrix: Matrix3) -> f64 {
    matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normal_positive_part::gaussian_negative_part_expectation;

    fn identity_matrix() -> Matrix3 {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    fn covariance_from_a_q1_q2(
        position: Matrix3,
        regression: Matrix3,
        conditional: Matrix3,
    ) -> RelativePositionVelocityCovarianceBlocks {
        let cross = multiply_3x3(regression, position);
        let velocity = add_3x3(conditional, multiply_3x3(regression, transpose_3x3(cross)));
        RelativePositionVelocityCovarianceBlocks {
            position,
            velocity_position: cross,
            velocity,
        }
    }

    fn add_3x3(left: Matrix3, right: Matrix3) -> Matrix3 {
        let mut result = [[0.0; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                result[i][j] = left[i][j] + right[i][j];
            }
        }
        result
    }

    #[test]
    fn diagonal_covariance_blocks_produce_expected_conditional_distribution() {
        let blocks = RelativePositionVelocityCovarianceBlocks {
            position: [[4.0, 0.0, 0.0], [0.0, 9.0, 0.0], [0.0, 0.0, 16.0]],
            velocity_position: [[0.0; 3]; 3],
            velocity: [[1.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 9.0]],
        };
        let distribution =
            ConditionalVelocityDistribution::new(blocks, ConditionalVelocityConfig::default())
                .expect("valid diagonal blocks");
        assert_eq!(
            distribution.position_covariance_inverse(),
            [
                [0.25, 0.0, 0.0],
                [0.0, 1.0 / 9.0, 0.0],
                [0.0, 0.0, 1.0 / 16.0]
            ]
        );
        assert_eq!(distribution.regression_matrix(), [[0.0; 3]; 3]);
        assert_eq!(
            distribution.conditional_velocity_covariance(),
            blocks.velocity
        );
    }

    #[test]
    fn schur_complement_recovers_known_q1_and_q2() {
        let position = [[4.0, 0.0, 0.0], [0.0, 9.0, 0.0], [0.0, 0.0, 16.0]];
        let regression = [[0.1, 0.02, 0.0], [0.0, -0.1, 0.03], [0.04, 0.0, 0.05]];
        let conditional = [[1.0, 0.1, 0.0], [0.1, 2.0, 0.2], [0.0, 0.2, 3.0]];
        let blocks = covariance_from_a_q1_q2(position, regression, conditional);
        let distribution =
            ConditionalVelocityDistribution::new(blocks, ConditionalVelocityConfig::default())
                .expect("valid block covariance");
        for i in 0..3 {
            for j in 0..3 {
                assert!(
                    (distribution.regression_matrix()[i][j] - regression[i][j]).abs() < 1.0e-12
                );
                assert!(
                    (distribution.conditional_velocity_covariance()[i][j] - conditional[i][j])
                        .abs()
                        < 1.0e-12
                );
            }
        }
    }

    #[test]
    fn directional_mean_and_mahalanobis_match_independent_block_reference() {
        let position = [[4.0, 0.0, 0.0], [0.0, 9.0, 0.0], [0.0, 0.0, 16.0]];
        let regression = [[0.1, 0.02, 0.0], [0.0, -0.1, 0.03], [0.04, 0.0, 0.05]];
        let conditional = [[1.0, 0.1, 0.0], [0.1, 2.0, 0.2], [0.0, 0.2, 3.0]];
        let blocks = covariance_from_a_q1_q2(position, regression, conditional);
        let distribution =
            ConditionalVelocityDistribution::new(blocks, ConditionalVelocityConfig::default())
                .expect("valid covariance");

        let report = distribution
            .evaluate_direction(
                [0.4, 0.0, 0.0],
                [-0.1, 0.0, 0.0],
                1.0,
                [1.0, 0.0, 0.0],
                2.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            )
            .expect("directional conditional state");

        let expected_dr = [0.6, 0.0, 0.0];
        let expected_mean = -0.1 + 0.1 * 0.6;
        let expected_variance: f64 = 1.0;
        let expected_md2 = 0.6_f64.powi(2) / 4.0;
        let expected_speed =
            gaussian_negative_part_expectation(expected_mean, expected_variance.sqrt())
                .expect("scalar positive-part reference")
                .expectation;
        assert!((report.position_offset[0] - expected_dr[0]).abs() < 1.0e-14);
        assert!((report.conditional_normal_mean - expected_mean).abs() < 1.0e-14);
        assert!((report.conditional_normal_variance - expected_variance).abs() < 1.0e-14);
        assert!((report.mahalanobis_distance_squared - expected_md2).abs() < 1.0e-14);
        assert!((report.expected_inward_normal_speed - expected_speed).abs() < 1.0e-14);
    }

    #[test]
    fn excessive_user_validation_tolerances_are_rejected() {
        let config = ConditionalVelocityConfig {
            conditional_psd_relative_tolerance: 1.0e-3,
            ..ConditionalVelocityConfig::default()
        };
        assert_eq!(
            ConditionalVelocityDistribution::new(
                RelativePositionVelocityCovarianceBlocks {
                    position: identity_matrix(),
                    velocity_position: [[0.0; 3]; 3],
                    velocity: identity_matrix(),
                },
                config
            ),
            Err(KernelError::InvalidCovarianceTolerance)
        );
    }

    #[test]
    fn nasa_and_mathematical_zero_variance_modes_remain_distinct() {
        let position = identity_matrix();
        let regression = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [-2.0, 0.0, 0.0]];
        let conditional = [[0.0; 3]; 3];
        let blocks = covariance_from_a_q1_q2(position, regression, conditional);
        let distribution =
            ConditionalVelocityDistribution::new(blocks, ConditionalVelocityConfig::default())
                .expect("rank-deficient full covariance with zero Schur complement");

        let nasa = distribution
            .evaluate_direction(
                [-0.5, 0.0, 0.0],
                [0.0; 3],
                1.0,
                [0.0, 0.0, 1.0],
                0.0,
                ZeroConditionalVarianceMode::NasaCompatibility,
            )
            .expect("NASA fallback");
        let mathematical = distribution
            .evaluate_direction(
                [-0.5, 0.0, 0.0],
                [0.0; 3],
                1.0,
                [0.0, 0.0, 1.0],
                0.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            )
            .expect("mathematical limit");
        assert_eq!(nasa.expected_inward_normal_speed, 0.0);
        assert_eq!(mathematical.expected_inward_normal_speed, 1.0);
        assert!(nasa.zero_variance_fallback_used);
        assert!(mathematical.zero_variance_fallback_used);
    }

    #[test]
    fn tiny_positive_directional_variance_keeps_the_positive_variance_formula() {
        let blocks = RelativePositionVelocityCovarianceBlocks {
            position: identity_matrix(),
            velocity_position: [[0.0; 3]; 3],
            velocity: [[0.0; 3], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0e-24]],
        };
        let distribution =
            ConditionalVelocityDistribution::new(blocks, ConditionalVelocityConfig::default())
                .expect("tiny positive PSD covariance");
        let report = distribution
            .evaluate_direction(
                [0.0; 3],
                [0.0, 0.0, -1.0e-11],
                1.0,
                [0.0, 0.0, 1.0],
                0.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            )
            .expect("tiny positive variance");
        assert!(report.conditional_normal_variance > 0.0);
        assert!(!report.zero_variance_fallback_used);
        assert!(report.expected_inward_normal_speed > 0.0);
        assert_eq!(
            report.zero_variance_mode,
            ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit
        );
    }

    #[test]
    fn tiny_negative_directional_variance_within_roundoff_tolerance_uses_zero_policy() {
        let blocks = RelativePositionVelocityCovarianceBlocks {
            position: identity_matrix(),
            velocity_position: [[0.0; 3]; 3],
            velocity: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0e-14]],
        };
        let distribution =
            ConditionalVelocityDistribution::new(blocks, ConditionalVelocityConfig::default())
                .expect("small negative eigenvalue inside declared roundoff tolerance");
        let report = distribution
            .evaluate_direction(
                [0.0; 3],
                [0.0, 0.0, -2.0],
                1.0,
                [0.0, 0.0, 1.0],
                0.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            )
            .expect("negative roundoff directional variance");
        assert!(report.conditional_normal_variance < 0.0);
        assert!(report.zero_variance_fallback_used);
        assert_eq!(report.expected_inward_normal_speed, 2.0);
    }

    #[test]
    fn materially_negative_conditional_covariance_is_rejected() {
        let blocks = RelativePositionVelocityCovarianceBlocks {
            position: identity_matrix(),
            velocity_position: [[0.0; 3]; 3],
            velocity: [[-1.0e-4, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        };
        assert_eq!(
            ConditionalVelocityDistribution::new(blocks, ConditionalVelocityConfig::default()),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn orthogonal_axis_permutation_preserves_directional_quantities() {
        let position = [[2.0, 0.1, 0.0], [0.1, 3.0, 0.0], [0.0, 0.0, 4.0]];
        let regression = [[0.1, 0.02, 0.0], [0.0, -0.1, 0.03], [0.04, 0.0, 0.05]];
        let conditional = [[1.0, 0.1, 0.0], [0.1, 2.0, 0.2], [0.0, 0.2, 3.0]];
        let blocks = covariance_from_a_q1_q2(position, regression, conditional);
        let original =
            ConditionalVelocityDistribution::new(blocks, ConditionalVelocityConfig::default())
                .expect("valid covariance");
        let mean_position = [0.2, -0.1, 0.05];
        let mean_velocity = [0.01, -0.02, 0.03];
        let direction = [0.3, 0.4, (0.75_f64).sqrt()];
        let report = original
            .evaluate_direction(
                mean_position,
                mean_velocity,
                0.5,
                direction,
                2.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            )
            .expect("original coordinates");

        let permute = |matrix: Matrix3| {
            let order = [1, 2, 0];
            let mut output = [[0.0; 3]; 3];
            for i in 0..3 {
                for j in 0..3 {
                    output[i][j] = matrix[order[i]][order[j]];
                }
            }
            output
        };
        let permute_vector = |vector: [f64; 3]| [vector[1], vector[2], vector[0]];
        let transformed_blocks = RelativePositionVelocityCovarianceBlocks {
            position: permute(position),
            velocity_position: permute(blocks.velocity_position),
            velocity: permute(blocks.velocity),
        };
        let transformed = ConditionalVelocityDistribution::new(
            transformed_blocks,
            ConditionalVelocityConfig::default(),
        )
        .expect("permuted covariance");
        let transformed_report = transformed
            .evaluate_direction(
                permute_vector(mean_position),
                permute_vector(mean_velocity),
                0.5,
                permute_vector(direction),
                2.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            )
            .expect("permuted coordinates");

        assert!(
            (report.mahalanobis_distance_squared - transformed_report.mahalanobis_distance_squared)
                .abs()
                < 1.0e-12
        );
        assert!(
            (report.conditional_normal_variance - transformed_report.conditional_normal_variance)
                .abs()
                < 1.0e-12
        );
        assert!(
            (report.expected_inward_normal_speed - transformed_report.expected_inward_normal_speed)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn invalid_covariance_tolerance_direction_and_nonfinite_inputs_fail_closed() {
        let blocks = RelativePositionVelocityCovarianceBlocks {
            position: identity_matrix(),
            velocity_position: [[0.0; 3]; 3],
            velocity: identity_matrix(),
        };
        let invalid_config = ConditionalVelocityConfig {
            symmetry_relative_tolerance: f64::NAN,
            ..ConditionalVelocityConfig::default()
        };
        assert_eq!(
            ConditionalVelocityDistribution::new(blocks, invalid_config),
            Err(KernelError::InvalidCovarianceTolerance)
        );

        let distribution =
            ConditionalVelocityDistribution::new(blocks, ConditionalVelocityConfig::default())
                .expect("valid covariance");
        assert_eq!(
            distribution.evaluate_direction(
                [0.0; 3],
                [0.0; 3],
                1.0,
                [0.0, 0.0, 2.0],
                0.0,
                ZeroConditionalVarianceMode::NasaCompatibility
            ),
            Err(KernelError::InvalidUnitDirection)
        );
        assert_eq!(
            distribution.evaluate_direction(
                [f64::NAN, 0.0, 0.0],
                [0.0; 3],
                1.0,
                [1.0, 0.0, 0.0],
                0.0,
                ZeroConditionalVarianceMode::NasaCompatibility
            ),
            Err(KernelError::NonFiniteValue)
        );
    }
}
