// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Log-domain integration of the conditional-Gaussian Ncdot rate over a sphere.
//!
//! This module composes the conditional relative-motion model with Lebedev
//! sphere rules. It preserves node contributions in log space, reproduces the
//! NASA strict significant-node threshold, and reports f64 underflow/overflow
//! separately from exact-zero contributions.
//!
//! It does not construct the conditional covariance, solve PeakOverlapPos,
//! propagate orbits, or adapt the temporal grid.

use crate::KernelError;
use crate::conditional_velocity::{
    ConditionalVelocityDistribution, ZeroConditionalVarianceMode,
};
use crate::lebedev::{LebedevRule, lebedev_sphere_rule};

/// Immutable fixed-time mean state and Ncdot normalization inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotSphereInputs {
    /// Mean relative position in the same length units as HBR and covariance.
    pub mean_relative_position: [f64; 3],
    /// Mean relative velocity in the corresponding length/time units.
    pub mean_relative_velocity: [f64; 3],
    /// Combined hard-body radius in the same length units as position.
    pub hard_body_radius: f64,
    /// NASA logZ term: the negative log normalization of the position PDF.
    ///
    /// The pointwise position factor is exp(-logZ - MD²/2).
    pub log_position_normalizer: f64,
    /// Explicit policy for exact-zero or tolerated-negative conditional variance.
    pub zero_variance_mode: ZeroConditionalVarianceMode,
}

/// Configuration for fixed-time Ncdot sphere integration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotSphereConfig {
    /// Lebedev rule used for the sphere integral.
    pub rule: LebedevRule,
    /// NASA-style relative significant-node threshold and minimum node count.
    pub diagnostics: NcdotSphereDiagnosticsConfig,
}

/// Contribution-support diagnostics for the Ncdot sphere integral.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotSphereDiagnosticsConfig {
    /// A node is significant when its unweighted integrand is strictly greater
    /// than this fraction of the maximum integrand. NASA uses 1e-3.
    pub relative_contribution_threshold: f64,
    /// Minimum significant-node count required to set adequacy_satisfied.
    pub minimum_significant_nodes: usize,
}

impl Default for NcdotSphereDiagnosticsConfig {
    fn default() -> Self {
        Self {
            relative_contribution_threshold: 1.0e-3,
            minimum_significant_nodes: 0,
        }
    }
}

/// Log-domain Ncdot sphere-integration result.
#[derive(Debug, Clone, PartialEq)]
pub struct NcdotSphereReport {
    /// Lebedev rule used.
    pub rule: LebedevRule,
    /// Number of Lebedev nodes evaluated.
    pub node_count: usize,
    /// Sum of the quadrature weights; should equal 4π.
    pub weight_sum: f64,
    /// Log of the sphere integral before multiplication by HBR squared.
    ///
    /// Negative infinity denotes exact zero; a finite value remains useful
    /// when the ordinary f64 integral underflows or overflows.
    pub log_sphere_integral: f64,
    /// Sphere integral in f64.
    pub sphere_integral: f64,
    /// True when a positive sphere integral underflowed in f64.
    pub sphere_integral_underflowed: bool,
    /// True when the sphere integral exceeds the f64 maximum.
    pub sphere_integral_overflowed: bool,
    /// Log of the collision-rate contribution R² times the sphere integral.
    pub log_collision_rate: f64,
    /// Collision-rate contribution in f64.
    pub collision_rate: f64,
    /// True when a positive collision-rate contribution underflowed in f64.
    pub collision_rate_underflowed: bool,
    /// True when the collision-rate contribution exceeds the f64 maximum.
    pub collision_rate_overflowed: bool,
    /// Maximum unweighted log integrand across the sphere.
    pub maximum_log_integrand: f64,
    /// Strict relative contribution threshold used for significant-node counting.
    pub relative_contribution_threshold: f64,
    /// Number of nodes strictly above threshold times the maximum integrand.
    pub significant_node_count: usize,
    /// Fraction of total solid-angle quadrature weight at significant nodes.
    pub significant_weight_fraction: f64,
    /// Fraction of the sphere integral contributed by significant nodes.
    pub significant_integral_fraction: f64,
    /// Largest single weighted node contribution divided by the sphere integral.
    pub maximum_node_integral_fraction: f64,
    /// Number of nodes whose f64 integrand underflowed despite a finite log.
    pub node_underflow_count: usize,
    /// Number of nodes whose f64 integrand overflowed despite a finite log.
    pub node_overflow_count: usize,
    /// Number of directions using the explicit zero-variance fallback.
    pub zero_variance_fallback_count: usize,
    /// True when all pointwise inward-speed contributions were exactly zero.
    pub exact_zero_integrand: bool,
    /// Whether the configured minimum significant-node count was met.
    pub adequacy_satisfied: bool,
}

/// Integrate the conditional-Gaussian collision-rate integrand over a Lebedev sphere.
///
/// At every node this evaluates the pointwise position density multiplied by
/// expected inward normal speed. Weighted contributions are accumulated using
/// log-sum-exp so individually underflowed positive nodes are retained.
///
/// The collision-rate contribution is HBR² times the sphere integral. All
/// spatial units must be consistent across the mean state, covariance and HBR.
pub fn integrate_ncdot_sphere(
    distribution: &ConditionalVelocityDistribution,
    inputs: NcdotSphereInputs,
    config: NcdotSphereConfig,
) -> Result<NcdotSphereReport, KernelError> {
    if inputs
        .mean_relative_position
        .iter()
        .chain(inputs.mean_relative_velocity.iter())
        .any(|value| !value.is_finite())
        || !inputs.hard_body_radius.is_finite()
        || !inputs.log_position_normalizer.is_finite()
    {
        return Err(KernelError::NonFiniteValue);
    }
    if inputs.hard_body_radius <= 0.0 {
        return Err(KernelError::InvalidHardBodyRadius);
    }
    let threshold = config.diagnostics.relative_contribution_threshold;
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) {
        return Err(KernelError::InvalidSphereQuadratureThreshold);
    }

    let rule = lebedev_sphere_rule(config.rule)?;
    let mut log_integrands = Vec::with_capacity(rule.nodes().len());
    let mut node_underflow_count = 0_usize;
    let mut node_overflow_count = 0_usize;
    let mut zero_variance_fallback_count = 0_usize;

    for node in rule.nodes() {
        let direction_report = distribution.evaluate_direction(
            inputs.mean_relative_position,
            inputs.mean_relative_velocity,
            inputs.hard_body_radius,
            [node.x, node.y, node.z],
            inputs.log_position_normalizer,
            inputs.zero_variance_mode,
        )?;
        let log_integrand = direction_report.log_integrand;
        if log_integrand.is_nan() || log_integrand == f64::INFINITY {
            return Err(KernelError::NumericalOverflow);
        }
        log_integrands.push(log_integrand);

        node_underflow_count = node_underflow_count
            .checked_add(usize::from(direction_report.integrand_underflowed))
            .ok_or(KernelError::CounterOverflow)?;
        node_overflow_count = node_overflow_count
            .checked_add(usize::from(direction_report.integrand_overflowed))
            .ok_or(KernelError::CounterOverflow)?;
        zero_variance_fallback_count = zero_variance_fallback_count
            .checked_add(usize::from(direction_report.zero_variance_fallback_used))
            .ok_or(KernelError::CounterOverflow)?;
    }

    let maximum_log_integrand = log_integrands
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let exact_zero_integrand = maximum_log_integrand == f64::NEG_INFINITY;

    let weight_sum = compensated_sum(rule.nodes().iter().map(|node| node.weight));
    if !weight_sum.is_finite()
        || (weight_sum - 4.0 * core::f64::consts::PI).abs() > 2.0e-12
    {
        return Err(KernelError::InvalidSphereQuadratureWeights);
    }

    let weighted_log_contributions = rule
        .nodes()
        .iter()
        .zip(&log_integrands)
        .map(|(node, log_value)| {
            if *log_value == f64::NEG_INFINITY {
                f64::NEG_INFINITY
            } else {
                node.weight.ln() + log_value
            }
        })
        .collect::<Vec<_>>();
    let log_sphere_integral = log_sum_exp(&weighted_log_contributions)?;

    let maximum_log_weighted_contribution = weighted_log_contributions
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);

    let log_threshold = if threshold == 0.0 {
        f64::NEG_INFINITY
    } else {
        threshold.ln()
    };
    let significant_indices = if exact_zero_integrand {
        Vec::new()
    } else {
        log_integrands
            .iter()
            .enumerate()
            .filter_map(|(index, value)| {
                (*value > maximum_log_integrand + log_threshold).then_some(index)
            })
            .collect::<Vec<_>>()
    };
    let significant_weight = compensated_sum(
        significant_indices
            .iter()
            .map(|index| rule.nodes()[*index].weight),
    );
    let significant_log_contributions = significant_indices
        .iter()
        .map(|index| weighted_log_contributions[*index])
        .collect::<Vec<_>>();
    let significant_log_integral = if significant_indices.is_empty() {
        f64::NEG_INFINITY
    } else {
        log_sum_exp(&significant_log_contributions)?
    };

    let significant_integral_fraction =
        log_fraction(significant_log_integral, log_sphere_integral);
    let maximum_node_integral_fraction =
        log_fraction(maximum_log_weighted_contribution, log_sphere_integral);

    let log_collision_rate = if log_sphere_integral == f64::NEG_INFINITY {
        f64::NEG_INFINITY
    } else {
        log_sphere_integral + 2.0 * inputs.hard_body_radius.ln()
    };
    if log_collision_rate.is_nan() || log_collision_rate == f64::INFINITY {
        return Err(KernelError::NumericalOverflow);
    }

    let (sphere_integral, sphere_integral_underflowed, sphere_integral_overflowed) =
        materialize_log_value(log_sphere_integral)?;
    let (collision_rate, collision_rate_underflowed, collision_rate_overflowed) =
        materialize_log_value(log_collision_rate)?;

    Ok(NcdotSphereReport {
        rule: config.rule,
        node_count: rule.nodes().len(),
        weight_sum,
        log_sphere_integral,
        sphere_integral,
        sphere_integral_underflowed,
        sphere_integral_overflowed,
        log_collision_rate,
        collision_rate,
        collision_rate_underflowed,
        collision_rate_overflowed,
        maximum_log_integrand,
        relative_contribution_threshold: threshold,
        significant_node_count: significant_indices.len(),
        significant_weight_fraction: significant_weight / weight_sum,
        significant_integral_fraction,
        maximum_node_integral_fraction,
        node_underflow_count,
        node_overflow_count,
        zero_variance_fallback_count,
        exact_zero_integrand,
        adequacy_satisfied: significant_indices.len()
            >= config.diagnostics.minimum_significant_nodes,
    })
}

fn materialize_log_value(log_value: f64) -> Result<(f64, bool, bool), KernelError> {
    if log_value == f64::NEG_INFINITY {
        return Ok((0.0, false, false));
    }
    if !log_value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if log_value > f64::MAX.ln() {
        return Ok((f64::INFINITY, false, true));
    }
    let value = log_value.exp();
    if !value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok((value, value == 0.0, false))
}

fn log_fraction(log_numerator: f64, log_denominator: f64) -> f64 {
    if log_numerator == f64::NEG_INFINITY || log_denominator == f64::NEG_INFINITY {
        0.0
    } else {
        (log_numerator - log_denominator).exp().clamp(0.0, 1.0)
    }
}

fn log_sum_exp(log_values: &[f64]) -> Result<f64, KernelError> {
    let maximum = log_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if maximum == f64::NEG_INFINITY {
        return Ok(f64::NEG_INFINITY);
    }
    if !maximum.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    let scaled_sum = compensated_sum(
        log_values
            .iter()
            .filter(|value| **value != f64::NEG_INFINITY)
            .map(|value| (*value - maximum).exp()),
    );
    if !scaled_sum.is_finite() || scaled_sum <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }
    let result = maximum + scaled_sum.ln();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for value in values {
        let next = sum + value;
        if sum.abs() >= value.abs() {
            correction += (sum - next) + value;
        } else {
            correction += (value - next) + sum;
        }
        sum = next;
    }
    sum + correction
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditional_velocity::{
        ConditionalVelocityConfig, RelativePositionVelocityCovarianceBlocks,
    };

    fn identity_matrix() -> [[f64; 3]; 3] {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    fn isotropic_distribution() -> ConditionalVelocityDistribution {
        ConditionalVelocityDistribution::new(
            RelativePositionVelocityCovarianceBlocks {
                position: identity_matrix(),
                velocity_position: [[0.0; 3]; 3],
                velocity: identity_matrix(),
            },
            ConditionalVelocityConfig::default(),
        )
        .expect("valid isotropic covariance")
    }

    fn zero_variance_distribution() -> ConditionalVelocityDistribution {
        ConditionalVelocityDistribution::new(
            RelativePositionVelocityCovarianceBlocks {
                position: identity_matrix(),
                velocity_position: [[0.0; 3]; 3],
                velocity: [[0.0; 3]; 3],
            },
            ConditionalVelocityConfig::default(),
        )
        .expect("valid zero conditional velocity covariance")
    }

    fn inputs(
        mean_relative_position: [f64; 3],
        mean_relative_velocity: [f64; 3],
        log_position_normalizer: f64,
        zero_variance_mode: ZeroConditionalVarianceMode,
    ) -> NcdotSphereInputs {
        NcdotSphereInputs {
            mean_relative_position,
            mean_relative_velocity,
            hard_body_radius: 1.0,
            log_position_normalizer,
            zero_variance_mode,
        }
    }

    fn config(rule: LebedevRule, minimum_significant_nodes: usize) -> NcdotSphereConfig {
        NcdotSphereConfig {
            rule,
            diagnostics: NcdotSphereDiagnosticsConfig {
                relative_contribution_threshold: 1.0e-3,
                minimum_significant_nodes,
            },
        }
    }

    #[test]
    fn isotropic_case_matches_constant_sphere_integral_and_hbr_squared_scaling() {
        let distribution = isotropic_distribution();
        let report = integrate_ncdot_sphere(
            &distribution,
            inputs(
                [0.0; 3],
                [0.0; 3],
                0.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            ),
            config(LebedevRule::Points5810, 100),
        )
        .expect("isotropic constant integrand");

        let nu = 1.0 / (2.0 * core::f64::consts::PI).sqrt();
        let pointwise = nu * (-0.5_f64).exp();
        let expected_sphere_integral = 4.0 * core::f64::consts::PI * pointwise;
        assert_eq!(report.node_count, 5810);
        assert!((report.sphere_integral - expected_sphere_integral).abs() < expected_sphere_integral * 2.0e-12);
        assert!((report.collision_rate - expected_sphere_integral).abs() < expected_sphere_integral * 2.0e-12);
        assert_eq!(report.significant_node_count, 5810);
        assert!((report.significant_integral_fraction - 1.0).abs() < 1.0e-13);
        assert!(report.adequacy_satisfied);
    }

    #[test]
    fn deterministic_zero_inward_speed_returns_exact_zero_without_fake_peak() {
        let distribution = zero_variance_distribution();
        let report = integrate_ncdot_sphere(
            &distribution,
            inputs(
                [0.0; 3],
                [0.0; 3],
                0.0,
                ZeroConditionalVarianceMode::NasaCompatibility,
            ),
            config(LebedevRule::Points26, 1),
        )
        .expect("all-zero deterministic integrand");

        assert!(report.exact_zero_integrand);
        assert_eq!(report.log_sphere_integral, f64::NEG_INFINITY);
        assert_eq!(report.sphere_integral, 0.0);
        assert_eq!(report.significant_node_count, 0);
        assert!(!report.adequacy_satisfied);
        assert_eq!(report.zero_variance_fallback_count, 26);
    }

    #[test]
    fn nasa_and_mathematical_zero_variance_modes_produce_distinct_sphere_results() {
        let position = identity_matrix();
        let regression = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [-2.0, 0.0, 0.0]];
        // C = B*A^-1*B^T makes the conditional covariance Q2 exactly zero.
        let velocity = [[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 4.0]];
        let distribution = ConditionalVelocityDistribution::new(
            RelativePositionVelocityCovarianceBlocks {
                position,
                velocity_position: regression,
                velocity,
            },
            ConditionalVelocityConfig::default(),
        )
        .expect("valid rank-deficient joint covariance");

        let nasa = integrate_ncdot_sphere(
            &distribution,
            inputs(
                [-0.5, 0.0, 0.0],
                [0.0; 3],
                0.0,
                ZeroConditionalVarianceMode::NasaCompatibility,
            ),
            config(LebedevRule::Points26, 0),
        )
        .expect("NASA fallback");
        let mathematical = integrate_ncdot_sphere(
            &distribution,
            inputs(
                [-0.5, 0.0, 0.0],
                [0.0; 3],
                0.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            ),
            config(LebedevRule::Points26, 0),
        )
        .expect("mathematical zero-variance limit");

        assert!(nasa.exact_zero_integrand);
        assert!(mathematical.sphere_integral > nasa.sphere_integral);
        assert!(mathematical.zero_variance_fallback_count > 0);
    }

    #[test]
    fn underflowed_node_values_retain_a_finite_sphere_log_integral() {
        let report = integrate_ncdot_sphere(
            &isotropic_distribution(),
            inputs(
                [0.0; 3],
                [0.0; 3],
                1_000.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            ),
            config(LebedevRule::Points26, 0),
        )
        .expect("log-domain underflow");
        assert!(report.log_sphere_integral.is_finite());
        assert_eq!(report.sphere_integral, 0.0);
        assert!(report.sphere_integral_underflowed);
        assert_eq!(report.node_underflow_count, 26);
    }

    #[test]
    fn overflowed_node_values_retain_a_finite_sphere_log_integral() {
        let report = integrate_ncdot_sphere(
            &isotropic_distribution(),
            inputs(
                [0.0; 3],
                [0.0; 3],
                -1_000.0,
                ZeroConditionalVarianceMode::MathematicalZeroVarianceLimit,
            ),
            config(LebedevRule::Points26, 0),
        )
        .expect("log-domain overflow");
        assert!(report.log_sphere_integral.is_finite());
        assert!(report.sphere_integral.is_infinite());
        assert!(report.sphere_integral_overflowed);
        assert_eq!(report.node_overflow_count, 26);
    }

    #[test]
    fn invalid_threshold_and_hbr_fail_closed() {
        let invalid = NcdotSphereConfig {
            rule: LebedevRule::Points6,
            diagnostics: NcdotSphereDiagnosticsConfig {
                relative_contribution_threshold: f64::NAN,
                minimum_significant_nodes: 0,
            },
        };
        assert_eq!(
            integrate_ncdot_sphere(
                &isotropic_distribution(),
                inputs(
                    [0.0; 3],
                    [0.0; 3],
                    0.0,
                    ZeroConditionalVarianceMode::NasaCompatibility,
                ),
                invalid
            ),
            Err(KernelError::InvalidSphereQuadratureThreshold)
        );

        assert_eq!(
            integrate_ncdot_sphere(
                &isotropic_distribution(),
                NcdotSphereInputs {
                    hard_body_radius: 0.0,
                    ..inputs(
                        [0.0; 3],
                        [0.0; 3],
                        0.0,
                        ZeroConditionalVarianceMode::NasaCompatibility,
                    )
                },
                config(LebedevRule::Points6, 0)
            ),
            Err(KernelError::InvalidHardBodyRadius)
        );
    }
}
