// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Pointwise NASA-style 3D-Nc collision-rate integrand.
//!
//! At one time and one hard-body-sphere direction, NASA's Ncdot integrand is
//! the conditional expected inward normal speed multiplied by the relative
//! position Gaussian density:
//!
//!   I(rhat) = nu(rhat) * exp(-logZ - 0.5 * MD2)
//!   dr = R * rhat - mu_r
//!   MD2 = dr' * Ainv * dr
//!   sigma2 = rhat' * Q2 * rhat
//!   nu0 = rhat' * (mu_v + Q1 * dr)
//!
//! For sigma2 > 0:
//!   x = nu0 / (sqrt(2) * sqrt(sigma2))
//!   nu = sqrt(sigma2) / sqrt(2*pi)
//!        * (exp(-x*x) - sqrt(pi)*x*erfc(x))
//!
//! NASA's zero-dispersion limit is nu = max(0, -mu_v' * rhat).
//!
//! The full instantaneous collision rate is R^2 times the integral of I over
//! unit-sphere solid angle. This module implements only the pointwise term;
//! it does not compute PeakOverlapPos, covariance propagation or time dynamics.
//!
//! Source: NASA CARA Analysis Tools, Pc3D_Hall.m, Ncdot_integrand.

use crate::KernelError;
use core::f64::consts::{PI, SQRT_2};

const SQRT_PI: f64 = 1.772_453_850_905_516;
const LOG_SQRT_TWO_PI: f64 = 0.918_938_533_204_672_7;
const LOG_MAX: f64 = 709.782_712_893_384;
const LOG_MIN_SUBNORMAL: f64 = -744.440_071_921_381_2;
const ASYMPTOTIC_SWITCH: f64 = 8.0;
const SYMMETRY_TOLERANCE_FACTOR: f64 = 128.0;

/// Inputs to one direction of the NASA-style 3D-Nc sphere integrand.
///
/// All state and covariance values must use consistent units. For example, if
/// positions are metres and velocities are metres/second, hard_body_radius is
/// in metres, mean_relative_velocity is metres/second, conditional velocity
/// covariance is (metres/second)^2, and log_position_density_normalizer is the
/// negative logarithm of a density scale in inverse cubic metres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotPointwiseInputs {
    /// Unit direction from the relative-position Gaussian centre to the
    /// hard-body sphere surface.
    pub direction: [f64; 3],
    /// Combined hard-body radius R.
    pub hard_body_radius: f64,
    /// Mean relative position at the current time.
    pub mean_relative_position: [f64; 3],
    /// Mean relative velocity at the current time.
    pub mean_relative_velocity: [f64; 3],
    /// Inverse relative position covariance A^{-1}.
    pub inverse_position_covariance: [[f64; 3]; 3],
    /// Conditional velocity-mean regression matrix Q1 = B A^{-1}.
    pub conditional_velocity_regression: [[f64; 3]; 3],
    /// Conditional relative velocity covariance Q2 = C - B A^{-1} B^T.
    pub conditional_velocity_covariance: [[f64; 3]; 3],
    /// Negative logarithm of the normalized 3D relative position-density scale.
    ///
    /// In NASA's implementation this is -log(Ns0).
    pub log_position_density_normalizer: f64,
}

/// Numerical result for one Ncdot sphere direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotPointwiseReport {
    /// Conditional expected inward normal speed nu, in distance/time units.
    ///
    /// May be zero if the exact conditional inward-speed contribution is zero
    /// or if the representable f64 value underflowed.
    pub inward_speed_factor: f64,
    /// Natural logarithm of the conditional inward speed. Negative infinity
    /// denotes an exactly zero contribution; a finite value remains available
    /// when the f64 speed itself underflows.
    pub log_inward_speed_factor: f64,
    /// Squared Mahalanobis distance from the relative position mean to the
    /// hard-body sphere point.
    pub mahalanobis_distance_squared: f64,
    /// Projected conditional velocity variance rhat^T Q2 rhat.
    pub projected_velocity_variance: f64,
    /// Conditional inward-speed Gaussian coordinate x.
    ///
    /// NaN is used for the deterministic zero-dispersion limit.
    pub standardized_inward_speed: f64,
    /// Logarithm of the pointwise collision-rate integrand.
    ///
    /// Negative infinity denotes a zero contribution. Finite values remain
    /// meaningful when the f64 integrand underflows.
    pub log_integrand: f64,
    /// Pointwise integrand nu * exp(-logZ - MD2/2).
    pub integrand: f64,
    /// True when a finite log inward-speed factor exponentiated to zero.
    pub inward_speed_underflowed: bool,
    /// True when a finite log integrand exponentiated to zero.
    pub integrand_underflowed: bool,
    /// True when NASA's deterministic zero-dispersion limit was used.
    ///
    /// This is used when projected conditional velocity variance is
    /// nonpositive within the validated numerical tolerance.
    pub used_zero_dispersion_limit: bool,
}

/// Evaluate the pointwise 3D-Nc collision-rate integrand for one unit-sphere direction.
///
/// The covariance inputs are required to be symmetric and valid: A^{-1} must
/// be strictly positive definite; Q2 must be positive semidefinite within a
/// scale-aware tolerance. A negative projected variance beyond that tolerance
/// is rejected rather than silently treated as zero dispersion.
///
/// For positive projected variance, the cancellation-prone function
/// H(x) = exp(-x^2) - sqrt(pi) * x * erfc(x)
/// is evaluated directly for x < 8 and by a 14-term asymptotic expansion for
/// x >= 8. The asymptotic is a high-accuracy numerical approximation, not a
/// formally bounded remainder. The log-domain form is retained independently
/// of the f64 integrand so extremely rare contributions are not lost to
/// exponentiation underflow.
///
/// The returned integrand does not include the outer R^2 surface factor or
/// Lebedev weights. The full instantaneous rate is R^2 times its solid-angle
/// integral.
pub fn evaluate_ncdot_pointwise(
    inputs: NcdotPointwiseInputs,
) -> Result<NcdotPointwiseReport, KernelError> {
    validate_finite_vector(inputs.direction)?;
    validate_finite_vector(inputs.mean_relative_position)?;
    validate_finite_vector(inputs.mean_relative_velocity)?;
    validate_finite_matrix(inputs.inverse_position_covariance)?;
    validate_finite_matrix(inputs.conditional_velocity_regression)?;
    validate_finite_matrix(inputs.conditional_velocity_covariance)?;

    if !inputs.hard_body_radius.is_finite() || inputs.hard_body_radius <= 0.0 {
        return Err(KernelError::InvalidHardBodyRadius);
    }
    if !inputs.log_position_density_normalizer.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }

    let direction_norm_squared = dot(inputs.direction, inputs.direction);
    if !direction_norm_squared.is_finite()
        || (direction_norm_squared - 1.0).abs() > 1.0e-10
    {
        return Err(KernelError::InvalidSphereDirection);
    }

    validate_symmetric_positive_definite(inputs.inverse_position_covariance)?;
    validate_symmetric_positive_semidefinite(inputs.conditional_velocity_covariance)?;

    let radius_direction = scale(inputs.direction, inputs.hard_body_radius);
    let displacement = subtract(radius_direction, inputs.mean_relative_position);
    let inverse_times_displacement =
        matrix_vector_product(inputs.inverse_position_covariance, displacement);
    let mahalanobis_distance_squared = dot(displacement, inverse_times_displacement);
    if !mahalanobis_distance_squared.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if mahalanobis_distance_squared < 0.0 {
        return Err(KernelError::InvalidPositionCovarianceInverse);
    }

    let regression_times_displacement =
        matrix_vector_product(inputs.conditional_velocity_regression, displacement);
    let conditional_mean_velocity = add(
        inputs.mean_relative_velocity,
        regression_times_displacement,
    );
    let nu0 = dot(inputs.direction, conditional_mean_velocity);
    if !nu0.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let q2_times_direction =
        matrix_vector_product(inputs.conditional_velocity_covariance, inputs.direction);
    let mut projected_variance = dot(inputs.direction, q2_times_direction);
    if !projected_variance.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let q2_scale = matrix_max_abs(inputs.conditional_velocity_covariance);
    let projected_variance_tolerance =
        SYMMETRY_TOLERANCE_FACTOR * f64::EPSILON * q2_scale;
    if projected_variance < -projected_variance_tolerance {
        return Err(KernelError::InvalidSphereDirectionVariance);
    }

    let (log_inward_speed_factor, standardized_inward_speed, used_zero_dispersion_limit) =
        if projected_variance <= 0.0 {
            // Match the limiting rule used in NASA's Ncdot_integrand.
            let deterministic_inward_speed =
                (-dot(inputs.mean_relative_velocity, inputs.direction)).max(0.0);
            if deterministic_inward_speed == 0.0 {
                (f64::NEG_INFINITY, f64::NAN, true)
            } else {
                (deterministic_inward_speed.ln(), f64::NAN, true)
            }
        } else {
            let sigma = projected_variance.sqrt();
            let x = nu0 / (SQRT_2 * sigma);
            let log_h = log_expected_positive_speed_kernel(x)?;
            let log_nu = sigma.ln() - LOG_SQRT_TWO_PI + log_h;
            (log_nu, x, false)
        };

    if projected_variance < 0.0 {
        projected_variance = 0.0;
    }

    let log_integrand = if log_inward_speed_factor == f64::NEG_INFINITY {
        f64::NEG_INFINITY
    } else {
        log_inward_speed_factor
            - inputs.log_position_density_normalizer
            - 0.5 * mahalanobis_distance_squared
    };

    if log_integrand.is_nan() || log_integrand == f64::INFINITY {
        return Err(KernelError::NumericalOverflow);
    }

    let inward_speed_underflowed = log_inward_speed_factor.is_finite()
        && log_inward_speed_factor < LOG_MIN_SUBNORMAL;
    let inward_speed_factor = exponentiate_nonnegative_log(log_inward_speed_factor)?;

    let integrand_underflowed = log_integrand.is_finite() && log_integrand < LOG_MIN_SUBNORMAL;
    let integrand = exponentiate_nonnegative_log(log_integrand)?;

    Ok(NcdotPointwiseReport {
        inward_speed_factor,
        log_inward_speed_factor,
        mahalanobis_distance_squared,
        projected_velocity_variance: projected_variance,
        standardized_inward_speed,
        log_integrand,
        integrand,
        inward_speed_underflowed,
        integrand_underflowed,
        used_zero_dispersion_limit,
    })
}

fn log_expected_positive_speed_kernel(x: f64) -> Result<f64, KernelError> {
    if !x.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }

    if x <= -ASYMPTOTIC_SWITCH {
        // erfc(x) rounds to 2 at this range; exp(-x^2) is negligible.
        // This log form avoids overflow in sqrt(pi) * (-x) * 2.
        return Ok(0.5 * PI.ln() + (-x).ln() + 2.0_f64.ln());
    }

    if x >= ASYMPTOTIC_SWITCH {
        let x_squared = x * x;
        if !x_squared.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }

        // H(x) = exp(-x^2) * [1/(2x^2) - 3/(4x^4)
        //       + 15/(8x^6) - 105/(16x^8) + ...].
        // 14 terms have relative truncation error below ~2e-14 at x=8.
        let mut term = 0.5 / x_squared;
        let mut scaled_h = term;
        for n in 0..13 {
            term *= -((2 * n + 3) as f64) / (2.0 * x_squared);
            scaled_h += term;
        }
        if !scaled_h.is_finite() || scaled_h <= 0.0 {
            return Err(KernelError::NumericalOverflow);
        }
        return Ok(-x_squared + scaled_h.ln());
    }

    let x_squared = x * x;
    let h = (-x_squared).exp() - SQRT_PI * x * libm::erfc(x);
    if !h.is_finite() || h <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(h.ln())
}

fn exponentiate_nonnegative_log(log_value: f64) -> Result<f64, KernelError> {
    if log_value == f64::NEG_INFINITY {
        return Ok(0.0);
    }
    if !log_value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if log_value > LOG_MAX {
        return Err(KernelError::NumericalOverflow);
    }
    let value = log_value.exp();
    if !value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(value)
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

fn matrix_max_abs(matrix: [[f64; 3]; 3]) -> f64 {
    matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
}

fn validate_symmetric_positive_definite(
    matrix: [[f64; 3]; 3],
) -> Result<(), KernelError> {
    validate_symmetric(matrix)?;

    let l00_squared = matrix[0][0];
    if l00_squared <= 0.0 {
        return Err(KernelError::NotPositiveDefinite);
    }
    let l00 = l00_squared.sqrt();
    let l10 = matrix[1][0] / l00;
    let l20 = matrix[2][0] / l00;
    let l11_squared = matrix[1][1] - l10 * l10;
    if !l11_squared.is_finite() || l11_squared <= 0.0 {
        return Err(KernelError::NotPositiveDefinite);
    }
    let l11 = l11_squared.sqrt();
    let l21 = (matrix[2][1] - l20 * l10) / l11;
    let l22_squared = matrix[2][2] - l20 * l20 - l21 * l21;
    if !l22_squared.is_finite() || l22_squared <= 0.0 {
        return Err(KernelError::NotPositiveDefinite);
    }
    Ok(())
}

fn validate_symmetric_positive_semidefinite(
    matrix: [[f64; 3]; 3],
) -> Result<(), KernelError> {
    validate_symmetric(matrix)?;
    let scale = matrix_max_abs(matrix);
    if scale == 0.0 {
        return Ok(());
    }

    let tolerance = SYMMETRY_TOLERANCE_FACTOR * f64::EPSILON * scale;
    for index in 0..3 {
        if matrix[index][index] < -tolerance {
            return Err(KernelError::InvalidConditionalVelocityCovariance);
        }
    }

    for i in 0..3 {
        for j in (i + 1)..3 {
            let minor = matrix[i][i] * matrix[j][j] - matrix[i][j] * matrix[i][j];
            if minor < -tolerance * scale {
                return Err(KernelError::InvalidConditionalVelocityCovariance);
            }
        }
    }

    let determinant = matrix[0][0]
        * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
        - matrix[0][1]
            * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
        + matrix[0][2]
            * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);
    if !determinant.is_finite() || determinant < -tolerance * scale * scale {
        return Err(KernelError::InvalidConditionalVelocityCovariance);
    }
    Ok(())
}

fn validate_symmetric(matrix: [[f64; 3]; 3]) -> Result<(), KernelError> {
    let scale = matrix_max_abs(matrix).max(f64::MIN_POSITIVE);
    let tolerance = SYMMETRY_TOLERANCE_FACTOR * f64::EPSILON * scale;
    for i in 0..3 {
        for j in (i + 1)..3 {
            if (matrix[i][j] - matrix[j][i]).abs() > tolerance {
                return Err(KernelError::NonSymmetricMatrix);
            }
        }
    }
    Ok(())
}

fn matrix_vector_product(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        matrix[0][0].mul_add(
            vector[0],
            matrix[0][1].mul_add(vector[1], matrix[0][2] * vector[2]),
        ),
        matrix[1][0].mul_add(
            vector[0],
            matrix[1][1].mul_add(vector[1], matrix[1][2] * vector[2]),
        ),
        matrix[2][0].mul_add(
            vector[0],
            matrix[2][1].mul_add(vector[1], matrix[2][2] * vector[2]),
        ),
    ]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0].mul_add(
        right[0],
        left[1].mul_add(right[1], left[2] * right[2]),
    )
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    [vector[0] * factor, vector[1] * factor, vector[2] * factor]
}

fn add(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_inputs() -> NcdotPointwiseInputs {
        NcdotPointwiseInputs {
            direction: [1.0, 0.0, 0.0],
            hard_body_radius: 1.0,
            mean_relative_position: [0.0, 0.0, 0.0],
            mean_relative_velocity: [-2.0, 0.0, 0.0],
            inverse_position_covariance: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            conditional_velocity_regression: [[0.0; 3]; 3],
            conditional_velocity_covariance: [[0.0; 3]; 3],
            log_position_density_normalizer: 0.0,
        }
    }

    #[test]
    fn zero_dispersion_matches_nasa_deterministic_limit() {
        let report = evaluate_ncdot_pointwise(base_inputs()).expect("zero dispersion");
        assert_eq!(report.inward_speed_factor, 2.0);
        assert_eq!(report.log_inward_speed_factor, 2.0_f64.ln());
        assert_eq!(report.projected_velocity_variance, 0.0);
        assert!(report.used_zero_dispersion_limit);
        assert!((report.mahalanobis_distance_squared - 1.0).abs() < 1.0e-14);
        assert!((report.integrand - 2.0 * (-0.5_f64).exp()).abs() < 1.0e-14);
    }

    #[test]
    fn zero_inward_velocity_produces_exact_zero_integrand() {
        let mut inputs = base_inputs();
        inputs.mean_relative_velocity = [2.0, 0.0, 0.0];
        let report = evaluate_ncdot_pointwise(inputs).expect("outward velocity");
        assert_eq!(report.inward_speed_factor, 0.0);
        assert_eq!(report.log_inward_speed_factor, f64::NEG_INFINITY);
        assert_eq!(report.integrand, 0.0);
        assert_eq!(report.log_integrand, f64::NEG_INFINITY);
    }

    #[test]
    fn isotropic_positive_conditional_variance_matches_high_precision_reference() {
        let mut inputs = base_inputs();
        inputs.conditional_velocity_covariance = [
            [4.0, 0.0, 0.0],
            [0.0, 4.0, 0.0],
            [0.0, 0.0, 4.0],
        ];
        let report = evaluate_ncdot_pointwise(inputs).expect("positive conditional variance");
        let x = -2.0 / (SQRT_2 * 2.0);
        assert!((report.standardized_inward_speed - x).abs() < 1.0e-14);
        assert!((report.inward_speed_factor - 2.166_630_941_175_372_6).abs() < 2.0e-15);
        assert!(!report.used_zero_dispersion_limit);
    }

    #[test]
    fn q1_conditional_velocity_mean_shift_is_used() {
        let mut inputs = base_inputs();
        inputs.conditional_velocity_regression[0][0] = -1.0;
        inputs.conditional_velocity_covariance[0][0] = 1.0;
        inputs.mean_relative_velocity = [0.0, 0.0, 0.0];
        let report = evaluate_ncdot_pointwise(inputs).expect("conditional mean shift");
        assert!((report.standardized_inward_speed - (-1.0 / SQRT_2)).abs() < 1.0e-14);
        assert!(report.inward_speed_factor > 0.0);
    }

    #[test]
    fn large_positive_standardized_speeds_preserve_log_tail_without_cancellation() {
        let reference = [
            (8.0, -68.874_857_632_189_31),
            (20.0, -406.688_345_454_661_73),
            (26.0, -683.211_553_474_962_6),
            (30.0, -907.497_205_381_581_6),
            (50.0, -2_508.517_792_771_967),
        ];
        for (x, expected_log_h) in reference {
            let actual = log_expected_positive_speed_kernel(x).expect("positive tail");
            assert!(
                (actual - expected_log_h).abs() < 3.0e-12,
                "x={x}, actual={actual}, expected={expected_log_h}"
            );
        }
    }

    #[test]
    fn negative_standardized_speed_is_stable() {
        let actual = log_expected_positive_speed_kernel(-1.0).expect("negative x");
        assert!((actual - 1.290_328_895_030_395_4).abs() < 1.0e-14);
    }

    #[test]
    fn underflowed_integrand_retains_finite_log_integrand() {
        let mut inputs = base_inputs();
        inputs.mean_relative_position = [-1_000.0, 0.0, 0.0];
        inputs.log_position_density_normalizer = 0.0;
        let report = evaluate_ncdot_pointwise(inputs).expect("rare tail");
        assert!(report.integrand_underflowed);
        assert_eq!(report.integrand, 0.0);
        assert!(report.log_integrand.is_finite());
    }

    #[test]
    fn anisotropic_rotated_position_precision_is_supported() {
        let mut inputs = base_inputs();
        inputs.direction = [0.0, 1.0, 0.0];
        inputs.inverse_position_covariance = [
            [2.0, 1.0, 0.0],
            [1.0, 2.0, 0.0],
            [0.0, 0.0, 3.0],
        ];
        let report = evaluate_ncdot_pointwise(inputs).expect("rotated SPD precision");
        assert!((report.mahalanobis_distance_squared - 2.0).abs() < 1.0e-14);
    }

    #[test]
    fn materially_negative_projected_variance_is_rejected() {
        let mut inputs = base_inputs();
        inputs.conditional_velocity_covariance[0][0] = -1.0;
        assert_eq!(
            evaluate_ncdot_pointwise(inputs),
            Err(KernelError::InvalidConditionalVelocityCovariance)
        );
    }

    #[test]
    fn non_unit_direction_and_non_spd_position_precision_are_rejected() {
        let mut inputs = base_inputs();
        inputs.direction = [2.0, 0.0, 0.0];
        assert_eq!(
            evaluate_ncdot_pointwise(inputs),
            Err(KernelError::InvalidSphereDirection)
        );

        let mut inputs = base_inputs();
        inputs.inverse_position_covariance[2][2] = -1.0;
        assert_eq!(
            evaluate_ncdot_pointwise(inputs),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn asymmetry_and_non_finite_inputs_are_rejected() {
        let mut inputs = base_inputs();
        inputs.inverse_position_covariance[0][1] = 0.1;
        assert_eq!(
            evaluate_ncdot_pointwise(inputs),
            Err(KernelError::NonSymmetricMatrix)
        );

        let mut inputs = base_inputs();
        inputs.log_position_density_normalizer = f64::NAN;
        assert_eq!(
            evaluate_ncdot_pointwise(inputs),
            Err(KernelError::NonFiniteValue)
        );
    }
}
