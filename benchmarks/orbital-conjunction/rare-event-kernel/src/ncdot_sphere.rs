// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Log-domain composition of pointwise NASA-style Ncdot with Lebedev sphere rules.
//!
//! The sphere integral is formed from weighted pointwise log integrands using
//! a common maximum-log scale. This preserves the relative contribution of
//! nodes whose ordinary f64 pointwise values underflow to zero.
//!
//! The instantaneous collision-rate estimate is R^2 times the solid-angle
//! integral. The embedded Lebedev contribution diagnostics are evaluated on
//! the common-scaled integrand; the relative 1e-3 threshold is invariant under
//! that positive common scale.

use crate::KernelError;
use crate::lebedev::{
    LebedevRule, LebedevSphereRule, SphereQuadratureDiagnosticsConfig,
    SphereQuadratureReport, integrate_sphere_samples,
};
use crate::ncdot_pointwise::{NcdotPointwiseInputs, evaluate_ncdot_pointwise};

const LOG_MAX: f64 = 709.782_712_893_384;
const LOG_MIN_SUBNORMAL: f64 = -744.440_071_921_381_2;

/// Configuration for pointwise Ncdot sphere integration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotSphereIntegrationConfig {
    /// Contribution and significant-node diagnostics passed to the Lebedev integrator.
    pub diagnostics: SphereQuadratureDiagnosticsConfig,
}

/// Log-domain instantaneous collision-rate estimate over one Lebedev sphere rule.
#[derive(Debug, Clone, PartialEq)]
pub struct NcdotSphereIntegrationReport {
    /// Lebedev rule used.
    pub rule: LebedevRule,
    /// Number of sphere nodes evaluated.
    pub node_count: usize,
    /// Maximum pointwise log integrand used as the common scaling factor.
    ///
    /// None means every sampled direction had an exact zero pointwise contribution.
    pub maximum_log_pointwise_integrand: Option<f64>,
    /// Logarithm of the solid-angle integral before the outer R^2 factor.
    pub log_sphere_integral: f64,
    /// Solid-angle integral in f64. May be zero if it underflowed.
    pub sphere_integral: f64,
    /// Logarithm of the instantaneous rate after applying R^2.
    pub log_instantaneous_collision_rate: f64,
    /// Instantaneous collision rate R^2 * sphere integral in f64.
    pub instantaneous_collision_rate: f64,
    /// Count of pointwise integrands whose unscaled f64 values underflowed.
    ///
    /// Their finite log values are still included in the log-domain sum.
    pub pointwise_underflow_count: usize,
    /// Count of directions that used NASA's deterministic zero-dispersion limit.
    pub zero_dispersion_limit_count: usize,
    /// True when every quadrature direction had an exact zero pointwise contribution.
    pub all_directions_zero: bool,
    /// True when the unscaled sphere integral exponentiated to zero.
    pub sphere_integral_underflowed: bool,
    /// True when the final instantaneous rate exponentiated to zero.
    pub instantaneous_rate_underflowed: bool,
    /// NASA-style contributor diagnostics evaluated on common-scaled values.
    ///
    /// None is returned for the exact-zero all-directions case.
    pub quadrature_diagnostics: Option<SphereQuadratureReport>,
}

/// Integrate the pointwise Ncdot function over one validated Lebedev sphere rule.
///
/// The direction field in template is replaced by each quadrature node. All
/// other state, covariance and density inputs remain identical across nodes.
///
/// The operation uses a common log scale. If ell_i is the pointwise log
/// integrand and w_i is the quadrature weight, it computes
///
///   max(ell_i) + log(sum_i w_i * exp(ell_i - max(ell_i))).
///
/// The outer hard-body factor R^2 is then applied in the log domain.
///
/// If all sampled directions produce an exact zero contribution, a valid zero
/// instantaneous rate is returned with no contributor diagnostic. This is
/// distinct from a malformed quadrature rule or numerical failure.
pub fn integrate_ncdot_sphere(
    template: NcdotPointwiseInputs,
    rule: &LebedevSphereRule,
    config: NcdotSphereIntegrationConfig,
) -> Result<NcdotSphereIntegrationReport, KernelError> {
    if rule.nodes().is_empty() {
        return Err(KernelError::InvalidSphereQuadratureNodeCount);
    }

    let mut reports = Vec::with_capacity(rule.nodes().len());
    let mut maximum_log_integrand = f64::NEG_INFINITY;
    let mut pointwise_underflow_count = 0_usize;
    let mut zero_dispersion_limit_count = 0_usize;

    for node in rule.nodes() {
        let mut inputs = template;
        inputs.direction = [node.x, node.y, node.z];
        let report = evaluate_ncdot_pointwise(inputs)?;
        if report.log_integrand.is_nan() || report.log_integrand == f64::INFINITY {
            return Err(KernelError::NumericalOverflow);
        }

        if report.log_integrand.is_finite() {
            maximum_log_integrand = maximum_log_integrand.max(report.log_integrand);
        }
        if report.integrand_underflowed {
            pointwise_underflow_count = pointwise_underflow_count
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
        }
        if report.used_zero_dispersion_limit {
            zero_dispersion_limit_count = zero_dispersion_limit_count
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
        }
        reports.push(report);
    }

    if maximum_log_integrand == f64::NEG_INFINITY {
        return Ok(NcdotSphereIntegrationReport {
            rule: rule.rule(),
            node_count: rule.nodes().len(),
            maximum_log_pointwise_integrand: None,
            log_sphere_integral: f64::NEG_INFINITY,
            sphere_integral: 0.0,
            log_instantaneous_collision_rate: f64::NEG_INFINITY,
            instantaneous_collision_rate: 0.0,
            pointwise_underflow_count,
            zero_dispersion_limit_count,
            all_directions_zero: true,
            sphere_integral_underflowed: false,
            instantaneous_rate_underflowed: false,
            quadrature_diagnostics: None,
        });
    }

    let scaled_integrands = reports
        .iter()
        .map(|report| {
            if report.log_integrand == f64::NEG_INFINITY {
                0.0
            } else {
                (report.log_integrand - maximum_log_integrand).exp()
            }
        })
        .collect::<Vec<_>>();

    let diagnostics =
        integrate_sphere_samples(rule, &scaled_integrands, config.diagnostics)?;
    let log_sphere_integral = maximum_log_integrand + diagnostics.integral.ln();
    if !log_sphere_integral.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let log_instantaneous_collision_rate =
        log_sphere_integral + 2.0 * template.hard_body_radius.ln();
    if !log_instantaneous_collision_rate.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let sphere_integral = exponentiate_with_underflow(log_sphere_integral)?;
    let instantaneous_collision_rate =
        exponentiate_with_underflow(log_instantaneous_collision_rate)?;

    Ok(NcdotSphereIntegrationReport {
        rule: rule.rule(),
        node_count: rule.nodes().len(),
        maximum_log_pointwise_integrand: Some(maximum_log_integrand),
        log_sphere_integral,
        sphere_integral,
        log_instantaneous_collision_rate,
        instantaneous_collision_rate,
        pointwise_underflow_count,
        zero_dispersion_limit_count,
        all_directions_zero: false,
        sphere_integral_underflowed: sphere_integral == 0.0,
        instantaneous_rate_underflowed: instantaneous_collision_rate == 0.0,
        quadrature_diagnostics: Some(diagnostics),
    })
}

fn exponentiate_with_underflow(log_value: f64) -> Result<f64, KernelError> {
    if log_value < LOG_MIN_SUBNORMAL {
        return Ok(0.0);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lebedev::lebedev_sphere_rule;
    use core::f64::consts::PI;

    fn template() -> NcdotPointwiseInputs {
        NcdotPointwiseInputs {
            direction: [1.0, 0.0, 0.0],
            hard_body_radius: 1.0,
            mean_relative_position: [0.0, 0.0, 0.0],
            mean_relative_velocity: [0.0, 0.0, 0.0],
            inverse_position_covariance: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            conditional_velocity_regression: [[0.0; 3]; 3],
            conditional_velocity_covariance: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            log_position_density_normalizer: 0.0,
        }
    }

    fn config() -> NcdotSphereIntegrationConfig {
        NcdotSphereIntegrationConfig {
            diagnostics: SphereQuadratureDiagnosticsConfig::default(),
        }
    }

    #[test]
    fn isotropic_constant_angular_integrand_recovers_four_pi_and_r_squared() {
        for selected in [
            LebedevRule::Points6,
            LebedevRule::Points14,
            LebedevRule::Points26,
            LebedevRule::Points5810,
        ] {
            let rule = lebedev_sphere_rule(selected).expect("valid Lebedev rule");
            let mut inputs = template();
            inputs.hard_body_radius = 2.0;
            inputs.log_position_density_normalizer = 0.0;
            // x=0 and isotropic covariances make the pointwise integrand independent
            // of the sphere direction.
            let report = integrate_ncdot_sphere(inputs, &rule, config())
                .expect("isotropic sphere integral");
            let pointwise = 1.0 / (2.0 * PI).sqrt() * (-0.5 * 4.0).exp();
            let expected_sphere_integral = 4.0 * PI * pointwise;
            let expected_rate = 4.0 * expected_sphere_integral;

            assert!((report.sphere_integral - expected_sphere_integral).abs() < 2.0e-13);
            assert!((report.instantaneous_collision_rate - expected_rate).abs() < 8.0e-13);
            assert_eq!(report.rule, selected);
            assert_eq!(report.node_count, selected.node_count());
            assert!(!report.all_directions_zero);
            assert_eq!(
                report.quadrature_diagnostics.as_ref().unwrap().significant_node_count,
                selected.node_count()
            );
        }
    }

    #[test]
    fn exact_zero_contributions_return_valid_zero_rate() {
        let rule = lebedev_sphere_rule(LebedevRule::Points26).expect("valid Lebedev rule");
        let mut inputs = template();
        inputs.conditional_velocity_covariance = [[0.0; 3]; 3];
        let report = integrate_ncdot_sphere(inputs, &rule, config()).expect("zero rate");
        assert!(report.all_directions_zero);
        assert_eq!(report.instantaneous_collision_rate, 0.0);
        assert_eq!(report.log_instantaneous_collision_rate, f64::NEG_INFINITY);
        assert!(report.quadrature_diagnostics.is_none());
    }

    #[test]
    fn log_domain_sphere_sum_preserves_pointwise_underflowed_nodes() {
        let rule = lebedev_sphere_rule(LebedevRule::Points14).expect("valid Lebedev rule");
        let mut inputs = template();
        inputs.log_position_density_normalizer = 2_000.0;
        let report = integrate_ncdot_sphere(inputs, &rule, config()).expect("underflowing rate");
        assert_eq!(report.pointwise_underflow_count, rule.node_count());
        assert!(report.log_sphere_integral.is_finite());
        assert!(report.log_instantaneous_collision_rate.is_finite());
        assert_eq!(report.sphere_integral, 0.0);
        assert_eq!(report.instantaneous_collision_rate, 0.0);
        assert!(report.sphere_integral_underflowed);
        assert!(report.instantaneous_rate_underflowed);
        assert!(!report.all_directions_zero);
    }

    #[test]
    fn concentrated_deterministic_inward_speed_is_flagged_by_node_support() {
        let rule = lebedev_sphere_rule(LebedevRule::Points6).expect("valid Lebedev rule");
        let mut inputs = template();
        inputs.conditional_velocity_covariance = [[0.0; 3]; 3];
        inputs.mean_relative_velocity = [-1.0, 0.0, 0.0];
        let report = integrate_ncdot_sphere(
            inputs,
            &rule,
            NcdotSphereIntegrationConfig {
                diagnostics: SphereQuadratureDiagnosticsConfig {
                    relative_contribution_threshold: 1.0e-3,
                    minimum_significant_nodes: 3,
                },
            },
        )
        .expect("concentrated sphere rate");
        let diagnostics = report.quadrature_diagnostics.expect("nonzero contribution");
        assert_eq!(diagnostics.significant_node_count, 1);
        assert!(!diagnostics.adequacy_satisfied);
        assert!(diagnostics.maximum_node_integral_fraction > 0.5);
        assert_eq!(report.zero_dispersion_limit_count, rule.node_count());
    }

    #[test]
    fn log_rate_equals_log_sphere_integral_plus_two_log_radius() {
        let rule = lebedev_sphere_rule(LebedevRule::Points14).expect("valid Lebedev rule");
        let mut inputs = template();
        inputs.hard_body_radius = 3.0;
        let report = integrate_ncdot_sphere(inputs, &rule, config()).expect("valid rate");
        let expected = report.log_sphere_integral + 2.0 * 3.0_f64.ln();
        assert!((report.log_instantaneous_collision_rate - expected).abs() < 1.0e-14);
    }
}
