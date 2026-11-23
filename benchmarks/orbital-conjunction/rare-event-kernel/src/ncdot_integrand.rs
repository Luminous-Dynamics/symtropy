// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! NASA Hall 3D-Nc pointwise collision-rate integrand and sphere integration.
//!
//! This module implements the Ncdot_integrand mathematical layer from NASA
//! CARA's public Pc3D_Hall.m. It accepts time-local relative-Gaussian quantities
//! after PeakOverlapPos and covariance reduction have been performed.
//!
//! For unit-sphere direction n and hard-body radius H:
//!
//!   d = H n - mu_r
//!   m2 = d' A^-1 d
//!   s2 = n' Q2 n
//!   mnu = n' (mu_v + Q1 d)
//!   eta = mnu / (sqrt(2) s)
//!   nu = s / sqrt(2 pi) * [exp(-eta^2) - sqrt(pi) eta erfc(eta)]
//!   f(n) = nu * exp(-logZ - m2/2)
//!   Ncdot = H^2 * integral over S^2 of f(n) dOmega
//!
//! Here Q1 = B A^-1 and Q2 = C - B A^-1 B'. The position-density normalizer
//! logZ is derived from A^-1 using Cholesky, preventing callers from pairing an
//! inverse covariance with an unrelated determinant.
//!
//! The zero-conditional-variance branch follows NASA's limiting convention
//! max(0, -mu_v' n). A negative projected variance is rejected, not clipped.
//!
//! The one-sided Gaussian flux shape is evaluated in the log domain. For large
//! positive eta, an asymptotic expansion avoids catastrophic cancellation.
//! Rare tails therefore retain a finite log-integrand when ordinary f64 values
//! underflow.
//!
//! This module does not implement PeakOverlapPos, the time-varying Jacobian
//! chain, cross-object density-correlation construction, or temporal integration.
//!
//! NASA source: CARA Analysis Tools, Pc3D_Hall.m, commit
//! 43f0160f94d76a91805d86521c3e8fa0d09ecd68.

use crate::lebedev::{LebedevRule, LebedevSphereRule, SphereQuadratureDiagnosticsConfig};
use crate::KernelError;
use core::f64::consts::{PI, SQRT_2};

const LOG_TWO_PI: f64 = 1.837_877_066_409_345_3;
const LOG_SQRT_TWO_PI: f64 = 0.918_938_533_204_672_7;
const LOG_TWO_SQRT_PI: f64 = 1.265_512_123_484_645_4;
const LOG_MAX: f64 = 709.782_712_893_384;
const LOG_MIN_SUBNORMAL: f64 = -744.440_071_921_381_2;
const SYMMETRY_REL_TOLERANCE: f64 = 1.0e-12;
const UNIT_DIRECTION_TOLERANCE: f64 = 1.0e-10;

/// Time-local input to NASA's 3D-Nc pointwise rate integrand.
///
/// position_covariance_inverse is A^-1, where A is the 3x3 relative-position
/// covariance. conditional_velocity_gain is Q1 = B A^-1. The conditional
/// velocity covariance is Q2 = C - B A^-1 B'.
///
/// All lengths must use one consistent unit (normally metres), velocities must
/// use that length unit per second, and the hard-body radius must use the same
/// length unit. The resulting Ncdot rate is in inverse seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotRateInput {
    /// Mean relative position at the current peak-overlap expansion centre.
    pub relative_position_mean: [f64; 3],
    /// Mean relative velocity at the current peak-overlap expansion centre.
    pub relative_velocity_mean: [f64; 3],
    /// Inverse 3x3 relative-position covariance A^-1.
    pub position_covariance_inverse: [[f64; 3]; 3],
    /// Conditional velocity gain Q1 = B A^-1.
    pub conditional_velocity_gain: [[f64; 3]; 3],
    /// Conditional velocity covariance Q2 = C - B A^-1 B'.
    pub conditional_velocity_covariance: [[f64; 3]; 3],
    /// Combined hard-body radius.
    pub hard_body_radius: f64,
}

/// Pointwise 3D-Nc integrand and intermediate quantities for one unit direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotPointReport {
    /// Unit-sphere direction used for this evaluation.
    pub direction: [f64; 3],
    /// Position residual d = H n - mu_r.
    pub position_residual: [f64; 3],
    /// Squared Mahalanobis distance d' A^-1 d.
    pub mahalanobis_squared: f64,
    /// Projected conditional velocity variance n' Q2 n.
    pub conditional_velocity_variance: f64,
    /// Conditional radial-velocity mean n' (mu_v + Q1 d).
    pub conditional_radial_velocity_mean: f64,
    /// True when the exact zero-variance limiting branch was used.
    pub used_zero_variance_limit: bool,
    /// Log of the one-sided Gaussian flux factor; None means exactly zero.
    pub log_one_sided_flux: Option<f64>,
    /// One-sided Gaussian flux factor. It can be zero due to f64 underflow.
    pub one_sided_flux: f64,
    /// Log of the 3D-Nc surface integrand; None means exactly zero.
    pub log_integrand: Option<f64>,
    /// Ordinary f64 surface integrand. It can be zero due to underflow.
    pub integrand: f64,
    /// True when a finite log-integrand underflowed to ordinary f64 zero.
    pub integrand_underflowed: bool,
}

/// Integrated 3D-Nc sphere-rate result with NASA-style node diagnostics.
///
/// sphere_integral is integral(f(n) dOmega); rate_per_second is HBR^2 times
/// that integral. Log-domain fields remain informative when ordinary f64 values
/// underflow.
#[derive(Debug, Clone, PartialEq)]
pub struct NcdotSphereReport {
    /// Lebedev rule used.
    pub rule: LebedevRule,
    /// Number of sphere directions evaluated.
    pub node_count: usize,
    /// Log of integral(f(n) dOmega); None means exact zero.
    pub log_sphere_integral: Option<f64>,
    /// Ordinary f64 sphere integral, possibly zero by underflow.
    pub sphere_integral: f64,
    /// Log of the Ncdot rate in inverse seconds; None means exact zero.
    pub log_rate_per_second: Option<f64>,
    /// Ordinary f64 Ncdot rate in inverse seconds.
    pub rate_per_second: f64,
    /// True when a finite log-rate underflowed to ordinary f64 zero.
    pub rate_underflowed: bool,
    /// True when every sampled direction had exactly zero one-sided flux.
    pub zero_rate: bool,
    /// Largest log-integrand over the sphere; None for exact zero.
    pub maximum_log_integrand: Option<f64>,
    /// NASA significant-node threshold relative to maximum integrand.
    pub relative_contribution_threshold: f64,
    /// Number of nodes strictly above threshold times maximum integrand.
    pub significant_node_count: usize,
    /// Fraction of total solid angle represented by significant nodes.
    pub significant_weight_fraction: f64,
    /// Fraction of estimated sphere integral from significant nodes.
    pub significant_integral_fraction: f64,
    /// Largest weighted node contribution divided by the sphere integral.
    pub maximum_node_integral_fraction: f64,
    /// Whether the configured minimum significant-node count was met.
    pub adequacy_satisfied: bool,
}

#[derive(Debug, Clone, Copy)]
struct PreparedNcdotInput {
    input: NcdotRateInput,
    log_position_normalizer: f64,
}

fn prepare_input(mut input: NcdotRateInput) -> Result<PreparedNcdotInput, KernelError> {
    if !input.hard_body_radius.is_finite() || input.hard_body_radius <= 0.0 {
        return Err(KernelError::InvalidHardBodyRadius);
    }
    if input
        .relative_position_mean
        .iter()
        .chain(input.relative_velocity_mean.iter())
        .any(|value| !value.is_finite())
        || input
            .conditional_velocity_gain
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Err(KernelError::NonFiniteValue);
    }

    validate_symmetric(&input.position_covariance_inverse)?;
    validate_symmetric(&input.conditional_velocity_covariance)?;
    symmetrize(&mut input.position_covariance_inverse);
    symmetrize(&mut input.conditional_velocity_covariance);
    validate_positive_semidefinite(&input.conditional_velocity_covariance)?;
    let log_det_position_inverse = cholesky_log_determinant(&input.position_covariance_inverse)?;

    // logZ = 3/2 log(2 pi) + 1/2 log(det(A)), with A^-1 supplied.
    // Therefore log(det(A)) = -log(det(A^-1)).
    let log_position_normalizer = 1.5 * LOG_TWO_PI - 0.5 * log_det_position_inverse;
    if !log_position_normalizer.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    Ok(PreparedNcdotInput {
        input,
        log_position_normalizer,
    })
}

/// Evaluate log(H(x)) for the one-sided Gaussian flux shape
///
/// H(x) = exp(-x^2) - sqrt(pi) * x * erfc(x).
///
/// The result remains useful when H(x) is below the f64 subnormal range.
/// For large positive x an asymptotic expansion avoids cancellation.
pub fn gaussian_flux_shape_log(standardized_mean: f64) -> Result<f64, KernelError> {
    if !standardized_mean.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }

    if standardized_mean >= 0.0 {
        return log_gaussian_flux_shape_positive(standardized_mean);
    }

    let magnitude = -standardized_mean;
    let log_positive_branch = log_gaussian_flux_shape_positive(magnitude)?;
    let log_linear_branch = LOG_TWO_SQRT_PI + magnitude.ln();
    Ok(log_add_exp(log_positive_branch, log_linear_branch))
}

fn log_gaussian_flux_shape_positive(value: f64) -> Result<f64, KernelError> {
    if value == 0.0 {
        return Ok(0.0);
    }

    if value >= 8.0 {
        let square = value * value;
        if !square.is_finite() {
            return Ok(f64::NEG_INFINITY);
        }

        // H(x) ~ exp(-x^2)/(2 x^2) *
        // sum_k (-1)^k (2k+1)!!/(2^k x^(2k)).
        // At x >= 8, the 16-term truncation is decreasing and has sub-1e-15
        // relative error at the branch boundary. This is an asymptotic
        // approximation, not an interval enclosure.
        let mut term = 1.0;
        let mut series = 1.0;
        for order in 1..16 {
            term *= -((2 * order + 1) as f64) / (2.0 * square);
            series += term;
        }
        if !series.is_finite() || series <= 0.0 {
            return Err(KernelError::NumericalOverflow);
        }

        return Ok(-square - core::f64::consts::LN_2 - 2.0 * value.ln() + series.ln());
    }

    let exponential = (-value * value).exp();
    let complementary_error = libm::erfc(value);
    let shape = exponential - PI.sqrt() * value * complementary_error;
    if !shape.is_finite() || shape <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(shape.ln())
}

/// Evaluate NASA's pointwise 3D-Nc integrand for one unit-sphere direction.
///
/// The position covariance normalizer is derived from A^-1 with Cholesky.
/// The conditional velocity covariance is not repaired; negative directional
/// variance is rejected.
pub fn evaluate_ncdot_direction(
    input: &NcdotRateInput,
    direction: [f64; 3],
) -> Result<NcdotPointReport, KernelError> {
    let prepared = prepare_input(*input)?;
    evaluate_prepared_direction(&prepared, direction)
}

fn evaluate_prepared_direction(
    prepared: &PreparedNcdotInput,
    direction: [f64; 3],
) -> Result<NcdotPointReport, KernelError> {
    validate_unit_direction(direction)?;
    let input = prepared.input;

    let position_residual = [
        input.hard_body_radius * direction[0] - input.relative_position_mean[0],
        input.hard_body_radius * direction[1] - input.relative_position_mean[1],
        input.hard_body_radius * direction[2] - input.relative_position_mean[2],
    ];
    let mahalanobis_squared = quadratic_form(position_residual, input.position_covariance_inverse)?;
    if mahalanobis_squared < 0.0 {
        return Err(KernelError::NegativeQuadraticForm);
    }

    let variance = quadratic_form(direction, input.conditional_velocity_covariance)?;
    if variance < 0.0 {
        return Err(KernelError::NegativeConditionalVelocityVariance);
    }

    let (conditional_mean, used_zero_variance_limit, log_flux) = if variance == 0.0 {
        // Match NASA CARA's zero-sigma limit exactly.
        let inward_mean = -dot(input.relative_velocity_mean, direction);
        if inward_mean <= 0.0 {
            (inward_mean, true, None)
        } else {
            (inward_mean, true, Some(inward_mean.ln()))
        }
    } else {
        let sigma = variance.sqrt();
        let gain_residual = matrix_vector(input.conditional_velocity_gain, position_residual);
        let conditional_mean = dot(
            direction,
            [
                input.relative_velocity_mean[0] + gain_residual[0],
                input.relative_velocity_mean[1] + gain_residual[1],
                input.relative_velocity_mean[2] + gain_residual[2],
            ],
        );
        if !conditional_mean.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }

        let log_sigma = sigma.ln();
        let log_abs_eta = if conditional_mean == 0.0 {
            f64::NEG_INFINITY
        } else {
            conditional_mean.abs().ln() - log_sigma - 0.5 * core::f64::consts::LN_2
        };
        let eta = (conditional_mean / sigma) / SQRT_2;
        let log_shape = if eta.is_finite() {
            gaussian_flux_shape_log(eta)?
        } else if conditional_mean > 0.0 {
            f64::NEG_INFINITY
        } else {
            // For eta -> -infinity, H(eta) ~ 2 sqrt(pi) |eta|.
            LOG_TWO_SQRT_PI + log_abs_eta
        };
        let log_flux = if log_shape == f64::NEG_INFINITY {
            None
        } else {
            Some(log_sigma - LOG_SQRT_TWO_PI + log_shape)
        };
        (conditional_mean, false, log_flux)
    };

    let log_position_density = -prepared.log_position_normalizer - 0.5 * mahalanobis_squared;
    let log_integrand = log_flux.map(|value| value + log_position_density);
    let one_sided_flux = match log_flux {
        Some(value) => exp_from_log(value)?,
        None => 0.0,
    };
    let integrand = match log_integrand {
        Some(value) => exp_from_log(value)?,
        None => 0.0,
    };

    Ok(NcdotPointReport {
        direction,
        position_residual,
        mahalanobis_squared,
        conditional_velocity_variance: variance,
        conditional_radial_velocity_mean: conditional_mean,
        used_zero_variance_limit,
        log_one_sided_flux: log_flux,
        one_sided_flux,
        log_integrand,
        integrand,
        integrand_underflowed: log_integrand.is_some() && integrand == 0.0,
    })
}

/// Integrate the pointwise rate over a validated Lebedev sphere rule.
///
/// The implementation accumulates weighted samples with log-sum-exp, preserving
/// a finite log-rate when ordinary pointwise f64 integrands underflow. The strict
/// significant-node diagnostic is evaluated in log space using NASA's original
/// value > threshold * maximum semantics.
pub fn integrate_ncdot_sphere(
    input: &NcdotRateInput,
    rule: &LebedevSphereRule,
    diagnostics: SphereQuadratureDiagnosticsConfig,
) -> Result<NcdotSphereReport, KernelError> {
    if !diagnostics.relative_contribution_threshold.is_finite()
        || !(0.0..=1.0).contains(&diagnostics.relative_contribution_threshold)
    {
        return Err(KernelError::InvalidSphereQuadratureThreshold);
    }
    let prepared = prepare_input(*input)?;
    let mut log_integrands = Vec::with_capacity(rule.nodes().len());
    let mut weighted_logs = Vec::with_capacity(rule.nodes().len());

    for node in rule.nodes() {
        let point = evaluate_prepared_direction(&prepared, [node.x, node.y, node.z])?;
        log_integrands.push(point.log_integrand);
        weighted_logs.push(point.log_integrand.map(|value| node.weight.ln() + value));
    }

    let finite_logs = log_integrands.iter().flatten().copied().collect::<Vec<_>>();
    if finite_logs.is_empty() {
        return Ok(NcdotSphereReport {
            rule: rule.rule(),
            node_count: rule.nodes().len(),
            log_sphere_integral: None,
            sphere_integral: 0.0,
            log_rate_per_second: None,
            rate_per_second: 0.0,
            rate_underflowed: false,
            zero_rate: true,
            maximum_log_integrand: None,
            relative_contribution_threshold: diagnostics.relative_contribution_threshold,
            significant_node_count: 0,
            significant_weight_fraction: 0.0,
            significant_integral_fraction: 0.0,
            maximum_node_integral_fraction: 0.0,
            adequacy_satisfied: diagnostics.minimum_significant_nodes == 0,
        });
    }

    let maximum_log_integrand = finite_logs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let weighted_finite_logs = weighted_logs.iter().flatten().copied().collect::<Vec<_>>();
    let log_weighted_total = log_sum_exp(&weighted_finite_logs)?;
    let log_sphere_integral = log_weighted_total;
    let log_rate = 2.0 * input.hard_body_radius.ln() + log_sphere_integral;
    let sphere_integral = exp_from_log(log_sphere_integral)?;
    let rate_per_second = exp_from_log(log_rate)?;

    let threshold_log = if diagnostics.relative_contribution_threshold == 0.0 {
        f64::NEG_INFINITY
    } else {
        maximum_log_integrand + diagnostics.relative_contribution_threshold.ln()
    };
    let significant_indices = log_integrands
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            value
                .filter(|log_value| *log_value > threshold_log)
                .map(|_| index)
        })
        .collect::<Vec<_>>();
    let significant_weight = significant_indices
        .iter()
        .map(|index| rule.nodes()[*index].weight)
        .sum::<f64>();
    let significant_weight_fraction = significant_weight / rule.weight_sum();

    let significant_weighted_logs = significant_indices
        .iter()
        .filter_map(|index| weighted_logs[*index])
        .collect::<Vec<_>>();
    let significant_integral_fraction = if significant_weighted_logs.is_empty() {
        0.0
    } else {
        (log_sum_exp(&significant_weighted_logs)? - log_weighted_total).exp()
    };
    let maximum_node_log = weighted_finite_logs
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let maximum_node_integral_fraction = (maximum_node_log - log_weighted_total).exp();

    Ok(NcdotSphereReport {
        rule: rule.rule(),
        node_count: rule.nodes().len(),
        log_sphere_integral: Some(log_sphere_integral),
        sphere_integral,
        log_rate_per_second: Some(log_rate),
        rate_per_second,
        rate_underflowed: rate_per_second == 0.0 && log_rate.is_finite(),
        zero_rate: false,
        maximum_log_integrand: Some(maximum_log_integrand),
        relative_contribution_threshold: diagnostics.relative_contribution_threshold,
        significant_node_count: significant_indices.len(),
        significant_weight_fraction,
        significant_integral_fraction,
        maximum_node_integral_fraction,
        adequacy_satisfied: significant_indices.len() >= diagnostics.minimum_significant_nodes,
    })
}

fn validate_unit_direction(direction: [f64; 3]) -> Result<(), KernelError> {
    if direction.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NonFiniteValue);
    }
    let norm_squared = dot(direction, direction);
    if !norm_squared.is_finite() || (norm_squared - 1.0).abs() > UNIT_DIRECTION_TOLERANCE {
        return Err(KernelError::InvalidNcdotDirection);
    }
    Ok(())
}

fn validate_symmetric(matrix: &[[f64; 3]; 3]) -> Result<(), KernelError> {
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
            if (matrix[row][col] - matrix[col][row]).abs()
                > SYMMETRY_REL_TOLERANCE * scale
            {
                return Err(KernelError::NonSymmetricMatrix);
            }
        }
    }
    Ok(())
}

fn symmetrize(matrix: &mut [[f64; 3]; 3]) {
    for row in 0..3 {
        for col in (row + 1)..3 {
            let average = 0.5 * (matrix[row][col] + matrix[col][row]);
            matrix[row][col] = average;
            matrix[col][row] = average;
        }
    }
}

fn validate_positive_semidefinite(matrix: &[[f64; 3]; 3]) -> Result<(), KernelError> {
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    let diagonal_tolerance = 1.0e-12 * scale;
    let minor_tolerance = 1.0e-12 * scale.powi(2);
    let determinant_tolerance = 1.0e-12 * scale.powi(3);
    if matrix[0][0] < -diagonal_tolerance
        || matrix[1][1] < -diagonal_tolerance
        || matrix[2][2] < -diagonal_tolerance
    {
        return Err(KernelError::NegativeConditionalVelocityVariance);
    }
    for (i, j) in [(0, 1), (0, 2), (1, 2)] {
        let minor = matrix[i][i] * matrix[j][j] - matrix[i][j] * matrix[i][j];
        if minor < -minor_tolerance {
            return Err(KernelError::NegativeConditionalVelocityVariance);
        }
    }
    let determinant = matrix[0][0]
        * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[1][2])
        - matrix[0][1]
            * (matrix[0][1] * matrix[2][2] - matrix[1][2] * matrix[0][2])
        + matrix[0][2]
            * (matrix[0][1] * matrix[1][2] - matrix[1][1] * matrix[0][2]);
    if determinant < -determinant_tolerance {
        return Err(KernelError::NegativeConditionalVelocityVariance);
    }
    Ok(())
}

fn cholesky_log_determinant(matrix: &[[f64; 3]; 3]) -> Result<f64, KernelError> {
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
            }
        }
    }
    let log_det = 2.0 * (lower[0][0].ln() + lower[1][1].ln() + lower[2][2].ln());
    if !log_det.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(log_det)
}

fn quadratic_form(vector: [f64; 3], matrix: [[f64; 3]; 3]) -> Result<f64, KernelError> {
    let product = matrix_vector(matrix, vector);
    let value = dot(vector, product);
    if !value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(value)
}

fn matrix_vector(matrix: [[f64; 3]; 3], vector: [f64; 3]) -> [f64; 3] {
    [
        dot(matrix[0], vector),
        dot(matrix[1], vector),
        dot(matrix[2], vector),
    ]
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0].mul_add(
        right[0],
        left[1].mul_add(right[1], left[2] * right[2]),
    )
}

fn exp_from_log(value: f64) -> Result<f64, KernelError> {
    if value == f64::NEG_INFINITY || value < LOG_MIN_SUBNORMAL {
        return Ok(0.0);
    }
    if !value.is_finite() || value > LOG_MAX {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(value.exp())
}

fn log_add_exp(left: f64, right: f64) -> f64 {
    if left == f64::NEG_INFINITY {
        return right;
    }
    if right == f64::NEG_INFINITY {
        return left;
    }
    let maximum = left.max(right);
    maximum + ((left - maximum).exp() + (right - maximum).exp()).ln()
}

fn log_sum_exp(values: &[f64]) -> Result<f64, KernelError> {
    if values.is_empty() {
        return Err(KernelError::ZeroSphereIntegrand);
    }
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !maximum.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    let scaled_sum = values
        .iter()
        .map(|value| (*value - maximum).exp())
        .sum::<f64>();
    if !scaled_sum.is_finite() || scaled_sum <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(maximum + scaled_sum.ln())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lebedev::lebedev_sphere_rule;

    fn identity_matrix() -> [[f64; 3]; 3] {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    fn base_input() -> NcdotRateInput {
        NcdotRateInput {
            relative_position_mean: [0.0, 0.0, 0.0],
            relative_velocity_mean: [0.0, 0.0, 0.0],
            position_covariance_inverse: identity_matrix(),
            conditional_velocity_gain: [[0.0; 3]; 3],
            conditional_velocity_covariance: identity_matrix(),
            hard_body_radius: 0.5,
        }
    }

    #[test]
    fn gaussian_flux_shape_matches_independent_high_precision_values() {
        let cases = [
            (-12.0, 42.538_892_421_732_385, 2.0e-14),
            (-4.0, 14.179_630_810_473_79, 2.0e-14),
            (-1.0, 3.633_981_557_701_812_4, 2.0e-14),
            (0.0, 1.0, 2.0e-15),
            (1.0, 0.089_073_855_890_780_35, 2.0e-14),
            (4.0, 3.229_662_176_239_876e-9, 5.0e-13),
            (8.0, 1.224_699_071_171_896_7e-30, 2.0e-14),
            (10.0, 1.832_811_561_255_704_3e-46, 2.0e-14),
            (20.0, 2.385_040_261_391_398e-177, 2.0e-14),
        ];
        for (x, expected, tolerance) in cases {
            let actual = gaussian_flux_shape_log(x).expect("finite log flux shape").exp();
            assert!(
                (actual - expected).abs() <= expected.abs() * tolerance,
                "x={x}: actual={actual:e}, expected={expected:e}"
            );
        }
    }

    #[test]
    fn gaussian_flux_shape_log_preserves_extreme_positive_tail() {
        let value = gaussian_flux_shape_log(40.0).expect("finite log asymptotic");
        assert!(value.is_finite());
        assert!(value < -1_600.0);
    }

    #[test]
    fn zero_conditional_variance_uses_nasa_limit() {
        let mut input = base_input();
        input.conditional_velocity_covariance = [[0.0; 3]; 3];
        input.relative_velocity_mean = [-2.0, 0.0, 0.0];
        let report =
            evaluate_ncdot_direction(&input, [1.0, 0.0, 0.0]).expect("zero-variance direction");
        assert!(report.used_zero_variance_limit);
        assert_eq!(report.conditional_radial_velocity_mean, 2.0);
        assert_eq!(report.one_sided_flux, 2.0);

        let no_inward_flux =
            evaluate_ncdot_direction(&input, [-1.0, 0.0, 0.0]).expect("outward direction");
        assert_eq!(no_inward_flux.one_sided_flux, 0.0);
        assert_eq!(no_inward_flux.log_integrand, None);
    }

    #[test]
    fn isotropic_pointwise_integrand_matches_analytic_density_and_flux() {
        let input = base_input();
        let report =
            evaluate_ncdot_direction(&input, [1.0, 0.0, 0.0]).expect("isotropic direction");
        let expected_log_integrand = -2.0 * LOG_TWO_PI - 0.125;
        assert!(
            (report.log_integrand.expect("positive integrand") - expected_log_integrand).abs()
                < 1.0e-13
        );
        assert!((report.conditional_velocity_variance - 1.0).abs() < 1.0e-14);
        assert!((report.mahalanobis_squared - 0.25).abs() < 1.0e-14);
    }

    #[test]
    fn isotropic_sphere_integral_matches_analytic_surface_rate() {
        let input = base_input();
        let rule = lebedev_sphere_rule(LebedevRule::Points5810).expect("5810-node rule");
        let report = integrate_ncdot_sphere(
            &input,
            &rule,
            SphereQuadratureDiagnosticsConfig::default(),
        )
        .expect("isotropic 3D-Nc sphere integral");
        let expected_log_rate =
            2.0 * 0.5_f64.ln() + (4.0 * PI).ln() - 2.0 * LOG_TWO_PI - 0.125;
        assert!(
            (report.log_rate_per_second.expect("positive rate") - expected_log_rate).abs()
                < 1.0e-12
        );
        assert!((report.significant_integral_fraction - 1.0).abs() < 1.0e-14);
        assert_eq!(report.significant_node_count, 5810);
    }

    #[test]
    fn rare_tail_retains_log_rate_when_ordinary_rate_underflows() {
        let mut input = base_input();
        input.relative_position_mean = [1_000.0, 0.0, 0.0];
        let rule = lebedev_sphere_rule(LebedevRule::Points26).expect("26-node rule");
        let report = integrate_ncdot_sphere(
            &input,
            &rule,
            SphereQuadratureDiagnosticsConfig::default(),
        )
        .expect("log-domain rare tail");
        assert!(report.log_rate_per_second.expect("log rate").is_finite());
        assert_eq!(report.rate_per_second, 0.0);
        assert!(report.rate_underflowed);
    }

    #[test]
    fn negative_conditional_variance_is_rejected_not_clipped() {
        let mut input = base_input();
        input.conditional_velocity_covariance[0][0] = -1.0;
        assert_eq!(
            evaluate_ncdot_direction(&input, [1.0, 0.0, 0.0]),
            Err(KernelError::NegativeConditionalVelocityVariance)
        );
    }

    #[test]
    fn invalid_directions_and_non_spd_position_covariance_fail_closed() {
        let input = base_input();
        assert_eq!(
            evaluate_ncdot_direction(&input, [2.0, 0.0, 0.0]),
            Err(KernelError::InvalidNcdotDirection)
        );

        let mut invalid = base_input();
        invalid.position_covariance_inverse[2][2] = 0.0;
        assert_eq!(
            evaluate_ncdot_direction(&invalid, [1.0, 0.0, 0.0]),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn nonsymmetric_position_or_velocity_covariance_is_rejected() {
        let mut invalid = base_input();
        invalid.position_covariance_inverse[0][1] = 0.5;
        assert_eq!(
            evaluate_ncdot_direction(&invalid, [1.0, 0.0, 0.0]),
            Err(KernelError::NonSymmetricMatrix)
        );

        let mut invalid = base_input();
        invalid.conditional_velocity_covariance[0][1] = 0.5;
        assert_eq!(
            evaluate_ncdot_direction(&invalid, [1.0, 0.0, 0.0]),
            Err(KernelError::NonSymmetricMatrix)
        );
    }

    #[test]
    fn log_normalizer_is_derived_from_inverse_covariance_determinant() {
        let mut input = base_input();
        input.position_covariance_inverse = [
            [1.0, 0.0, 0.0],
            [0.0, 4.0, 0.0],
            [0.0, 0.0, 9.0],
        ];
        let report =
            evaluate_ncdot_direction(&input, [1.0, 0.0, 0.0]).expect("diagonal inverse");
        let log_det_inverse = (36.0_f64).ln();
        let expected_log_normalizer = 1.5 * LOG_TWO_PI - 0.5 * log_det_inverse;
        let expected_mahalanobis = 0.25;
        let expected_log_density =
            -expected_log_normalizer - 0.5 * expected_mahalanobis;
        let expected_log_flux = -LOG_SQRT_TWO_PI;
        assert!(
            (report.log_integrand.expect("integrand")
                - expected_log_flux
                - expected_log_density)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn rotation_of_all_directional_inputs_preserves_pointwise_integrand() {
        let mut input = base_input();
        input.relative_position_mean = [0.1, -0.2, 0.3];
        input.relative_velocity_mean = [0.4, 0.5, -0.6];
        input.conditional_velocity_gain = [
            [0.1, 0.2, 0.0],
            [0.2, -0.1, 0.1],
            [0.0, 0.1, 0.3],
        ];
        input.conditional_velocity_covariance = [
            [2.0, 0.1, 0.0],
            [0.1, 1.5, 0.2],
            [0.0, 0.2, 1.0],
        ];
        let direction = [1.0 / 3.0_f64.sqrt(), 2.0 / 3.0_f64.sqrt(), 0.0];
        let original =
            evaluate_ncdot_direction(&input, direction).expect("original coordinates");

        let rotation = [[0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];
        let rotate_vector = |value: [f64; 3]| matrix_vector(rotation, value);
        let rotate_matrix = |matrix: [[f64; 3]; 3]| {
            let mut result = [[0.0; 3]; 3];
            for row in 0..3 {
                for col in 0..3 {
                    for first in 0..3 {
                        for second in 0..3 {
                            result[row][col] +=
                                rotation[row][first] * matrix[first][second] * rotation[col][second];
                        }
                    }
                }
            }
            result
        };
        let mut rotated = input;
        rotated.relative_position_mean = rotate_vector(input.relative_position_mean);
        rotated.relative_velocity_mean = rotate_vector(input.relative_velocity_mean);
        rotated.position_covariance_inverse =
            rotate_matrix(input.position_covariance_inverse);
        rotated.conditional_velocity_gain = rotate_matrix(input.conditional_velocity_gain);
        rotated.conditional_velocity_covariance =
            rotate_matrix(input.conditional_velocity_covariance);
        let transformed =
            evaluate_ncdot_direction(&rotated, rotate_vector(direction)).expect("rotated");
        assert!(
            (original.log_integrand.unwrap() - transformed.log_integrand.unwrap()).abs()
                < 1.0e-12
        );
    }
}
