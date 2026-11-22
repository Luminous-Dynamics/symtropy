// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic low-order Lebedev rules and sphere-integrand adequacy diagnostics.
//!
//! The supported 6-, 14-, and 26-point rules have algebraic precisions 3, 5,
//! and 7, respectively. They are deliberately a small independently testable
//! subset, not a replacement for NASA CARA's production 5,810-point rule.
//!
//! NASA's public Pc3D_Hall implementation counts Lebedev points whose
//! integrand exceeds 1e-3 times the maximum integrand as a diagnostic of
//! whether enough nodes materially contribute to the collision-rate integral.
//! This module exposes that threshold and additional contribution diagnostics.
//!
//! References:
//! - Lebedev & Laikov (1999), "A quadrature formula for the sphere of the 131st
//!   algebraic order of accuracy", Doklady Mathematics 59(3), 477-481.
//! - NASA CARA Analysis Tools, Pc3D_Hall.m, Ncdot sphere-integration path.

use crate::KernelError;
use core::f64::consts::PI;

/// One unit-sphere quadrature node and its solid-angle weight.
///
/// Weights are expressed for integration over solid angle and sum to 4π.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereQuadratureNode {
    /// Cartesian x-coordinate on the unit sphere.
    pub x: f64,
    /// Cartesian y-coordinate on the unit sphere.
    pub y: f64,
    /// Cartesian z-coordinate on the unit sphere.
    pub z: f64,
    /// Positive solid-angle quadrature weight.
    pub weight: f64,
}

impl SphereQuadratureNode {
    /// Return the squared Euclidean norm of this node.
    pub fn norm_squared(self) -> f64 {
        self.x
            .mul_add(self.x, self.y.mul_add(self.y, self.z * self.z))
    }
}

/// Supported low-order Lebedev sphere rules.
///
/// Algebraic precision is the maximum total polynomial degree integrated
/// exactly in exact arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LebedevRule {
    /// Six axis nodes; algebraic precision 3.
    Points6,
    /// Six axis and eight cube-corner nodes; algebraic precision 5.
    Points14,
    /// Six axis, eight cube-corner and twelve edge-midpoint nodes;
    /// algebraic precision 7.
    Points26,
}

impl LebedevRule {
    /// Number of nodes in the rule.
    pub const fn node_count(self) -> usize {
        match self {
            Self::Points6 => 6,
            Self::Points14 => 14,
            Self::Points26 => 26,
        }
    }

    /// Algebraic polynomial precision of the rule.
    pub const fn algebraic_precision(self) -> usize {
        match self {
            Self::Points6 => 3,
            Self::Points14 => 5,
            Self::Points26 => 7,
        }
    }
}

/// A validated deterministic Lebedev rule.
///
/// The fields are private so callers cannot mutate nodes or weights after the
/// built-in rule has been validated.
#[derive(Debug, Clone, PartialEq)]
pub struct LebedevSphereRule {
    rule: LebedevRule,
    nodes: Vec<SphereQuadratureNode>,
    weight_sum: f64,
}

impl LebedevSphereRule {
    /// Rule identifier and known algebraic precision.
    pub fn rule(&self) -> LebedevRule {
        self.rule
    }

    /// Nodes in deterministic order.
    pub fn nodes(&self) -> &[SphereQuadratureNode] {
        &self.nodes
    }

    /// Sum of the solid-angle weights.
    pub fn weight_sum(&self) -> f64 {
        self.weight_sum
    }
}

/// Build one of the supported positive-weight Lebedev rules.
///
/// The node order is stable and nested: the first six points are the axis
/// points, followed by eight cube corners, followed by twelve edge midpoints.
pub fn lebedev_sphere_rule(rule: LebedevRule) -> Result<LebedevSphereRule, KernelError> {
    let mut nodes = Vec::with_capacity(rule.node_count());
    let axis_weight = match rule {
        LebedevRule::Points6 => 4.0 * PI / 6.0,
        LebedevRule::Points14 => 4.0 * PI / 15.0,
        LebedevRule::Points26 => 4.0 * PI / 21.0,
    };

    nodes.extend([
        SphereQuadratureNode {
            x: 1.0,
            y: 0.0,
            z: 0.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: -1.0,
            y: 0.0,
            z: 0.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: 0.0,
            y: 1.0,
            z: 0.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: 0.0,
            y: -1.0,
            z: 0.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            weight: axis_weight,
        },
        SphereQuadratureNode {
            x: 0.0,
            y: 0.0,
            z: -1.0,
            weight: axis_weight,
        },
    ]);

    if matches!(rule, LebedevRule::Points14 | LebedevRule::Points26) {
        let coordinate = 1.0 / 3.0_f64.sqrt();
        let weight = 4.0 * PI * (3.0 / 40.0);
        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                for sz in [-1.0, 1.0] {
                    nodes.push(SphereQuadratureNode {
                        x: sx * coordinate,
                        y: sy * coordinate,
                        z: sz * coordinate,
                        weight,
                    });
                }
            }
        }
    }

    if rule == LebedevRule::Points26 {
        let coordinate = 1.0 / 2.0_f64.sqrt();
        let weight = 4.0 * PI * (4.0 / 105.0);

        for sx in [-1.0, 1.0] {
            for sy in [-1.0, 1.0] {
                nodes.push(SphereQuadratureNode {
                    x: sx * coordinate,
                    y: sy * coordinate,
                    z: 0.0,
                    weight,
                });
            }
        }
        for sx in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                nodes.push(SphereQuadratureNode {
                    x: sx * coordinate,
                    y: 0.0,
                    z: sz * coordinate,
                    weight,
                });
            }
        }
        for sy in [-1.0, 1.0] {
            for sz in [-1.0, 1.0] {
                nodes.push(SphereQuadratureNode {
                    x: 0.0,
                    y: sy * coordinate,
                    z: sz * coordinate,
                    weight,
                });
            }
        }
    }

    validate_nodes(&nodes, rule.node_count())?;
    let weight_sum = compensated_sum(nodes.iter().map(|node| node.weight));
    let expected_weight_sum = 4.0 * PI;
    if !weight_sum.is_finite()
        || (weight_sum - expected_weight_sum).abs()
            > 64.0 * f64::EPSILON * expected_weight_sum
    {
        return Err(KernelError::InvalidSphereQuadratureWeights);
    }

    Ok(LebedevSphereRule {
        rule,
        nodes,
        weight_sum,
    })
}

fn validate_nodes(
    nodes: &[SphereQuadratureNode],
    expected_count: usize,
) -> Result<(), KernelError> {
    if nodes.len() != expected_count {
        return Err(KernelError::InvalidSphereQuadratureNodeCount);
    }
    for node in nodes {
        if !node.x.is_finite()
            || !node.y.is_finite()
            || !node.z.is_finite()
            || !node.weight.is_finite()
        {
            return Err(KernelError::NonFiniteValue);
        }
        if node.weight <= 0.0 {
            return Err(KernelError::InvalidSphereQuadratureWeights);
        }
        let norm_squared = node.norm_squared();
        if !norm_squared.is_finite() || (norm_squared - 1.0).abs() > 64.0 * f64::EPSILON {
            return Err(KernelError::InvalidSphereQuadratureNode);
        }
    }
    Ok(())
}

/// Configuration for Lebedev integrand-contribution diagnostics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SphereQuadratureDiagnosticsConfig {
    /// Relative threshold against the largest unweighted integrand value.
    ///
    /// NASA's Pc3D_Hall diagnostic uses 1e-3. A node is significant only when
    /// its sample is strictly greater than threshold times the maximum sample.
    pub relative_contribution_threshold: f64,
    /// Minimum significant-node count required by the caller's adequacy flag.
    ///
    /// Zero records diagnostics without requiring a minimum.
    pub minimum_significant_nodes: usize,
}

impl Default for SphereQuadratureDiagnosticsConfig {
    fn default() -> Self {
        Self {
            relative_contribution_threshold: 1.0e-3,
            minimum_significant_nodes: 0,
        }
    }
}

/// Result of integrating a nonnegative sphere integrand with contribution diagnostics.
///
/// These diagnostics do not form a formal quadrature error bound. The adequacy
/// flag only records the caller's minimum-significant-node criterion.
#[derive(Debug, Clone, PartialEq)]
pub struct SphereQuadratureReport {
    /// Rule used for the integration.
    pub rule: LebedevRule,
    /// Number of quadrature nodes.
    pub node_count: usize,
    /// Lebedev approximation to the solid-angle integral.
    pub integral: f64,
    /// Sum of the rule's weights; should equal 4π.
    pub weight_sum: f64,
    /// Largest unweighted integrand sample.
    pub maximum_integrand: f64,
    /// Relative threshold used for the significance diagnostic.
    pub relative_contribution_threshold: f64,
    /// Number of nodes with integrand strictly above threshold times maximum.
    pub significant_node_count: usize,
    /// Fraction of total solid-angle weight at significant nodes.
    pub significant_weight_fraction: f64,
    /// Fraction of estimated integral contributed by significant nodes.
    pub significant_integral_fraction: f64,
    /// Largest weighted node contribution divided by the integral.
    pub maximum_node_integral_fraction: f64,
    /// Whether the configured minimum significant-node count was met.
    pub adequacy_satisfied: bool,
}

/// Integrate a nonnegative integrand sample vector over the unit sphere.
///
/// The caller supplies one finite nonnegative sample per rule node in exactly
/// the deterministic node order returned by lebedev_sphere_rule. This function
/// reproduces NASA's relative significant-node diagnostic and adds weighted
/// contribution fractions. It does not estimate unobserved quadrature error or
/// prove convergence between rules.
pub fn integrate_sphere_samples(
    rule: &LebedevSphereRule,
    integrand_values: &[f64],
    config: SphereQuadratureDiagnosticsConfig,
) -> Result<SphereQuadratureReport, KernelError> {
    validate_nodes(&rule.nodes, rule.rule.node_count())?;
    let actual_weight_sum = compensated_sum(rule.nodes.iter().map(|node| node.weight));
    if !actual_weight_sum.is_finite()
        || (actual_weight_sum - 4.0 * PI).abs() > 64.0 * f64::EPSILON * 4.0 * PI
        || (actual_weight_sum - rule.weight_sum).abs() > 64.0 * f64::EPSILON * 4.0 * PI
    {
        return Err(KernelError::InvalidSphereQuadratureWeights);
    }
    if integrand_values.len() != rule.nodes.len() {
        return Err(KernelError::InvalidSphereQuadratureNodeCount);
    }
    if !config.relative_contribution_threshold.is_finite()
        || !(0.0..=1.0).contains(&config.relative_contribution_threshold)
    {
        return Err(KernelError::InvalidSphereQuadratureThreshold);
    }
    if integrand_values.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NonFiniteValue);
    }
    if integrand_values.iter().any(|value| *value < 0.0) {
        return Err(KernelError::InvalidSphereIntegrand);
    }

    let maximum_integrand = integrand_values.iter().copied().fold(0.0_f64, f64::max);
    if maximum_integrand <= 0.0 {
        return Err(KernelError::ZeroSphereIntegrand);
    }

    let weighted_contributions = rule
        .nodes
        .iter()
        .zip(integrand_values)
        .map(|(node, value)| node.weight * value)
        .collect::<Vec<_>>();
    if weighted_contributions.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NumericalOverflow);
    }

    let integral = compensated_sum(weighted_contributions.iter().copied());
    if !integral.is_finite() || integral <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }

    let threshold = config.relative_contribution_threshold * maximum_integrand;
    let significant_indices = integrand_values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (*value > threshold).then_some(index))
        .collect::<Vec<_>>();
    let significant_weight = compensated_sum(
        significant_indices
            .iter()
            .map(|index| rule.nodes[*index].weight),
    );
    let significant_integral = compensated_sum(
        significant_indices
            .iter()
            .map(|index| weighted_contributions[*index]),
    );
    let maximum_node_contribution = weighted_contributions
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);

    Ok(SphereQuadratureReport {
        rule: rule.rule,
        node_count: rule.nodes.len(),
        integral,
        weight_sum: rule.weight_sum,
        maximum_integrand,
        relative_contribution_threshold: config.relative_contribution_threshold,
        significant_node_count: significant_indices.len(),
        significant_weight_fraction: significant_weight / rule.weight_sum,
        significant_integral_fraction: significant_integral / integral,
        maximum_node_integral_fraction: maximum_node_contribution / integral,
        adequacy_satisfied: significant_indices.len() >= config.minimum_significant_nodes,
    })
}

fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    // Neumaier compensated summation for positive terms with varied magnitudes.
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

    fn rule(selected: LebedevRule) -> LebedevSphereRule {
        lebedev_sphere_rule(selected).expect("valid built-in rule")
    }

    fn double_factorial(value: usize) -> f64 {
        if value <= 1 {
            return 1.0;
        }
        (1..=value)
            .step_by(2)
            .fold(1.0, |product, term| product * term as f64)
    }

    fn exact_sphere_monomial_integral(a: usize, b: usize, c: usize) -> f64 {
        if a % 2 == 1 || b % 2 == 1 || c % 2 == 1 {
            return 0.0;
        }
        let numerator = double_factorial(a.saturating_sub(1))
            * double_factorial(b.saturating_sub(1))
            * double_factorial(c.saturating_sub(1));
        let denominator = double_factorial(a + b + c + 1);
        4.0 * PI * numerator / denominator
    }

    fn integrate_monomial(rule: &LebedevSphereRule, a: usize, b: usize, c: usize) -> f64 {
        compensated_sum(rule.nodes.iter().map(|node| {
            node.weight * node.x.powi(a as i32) * node.y.powi(b as i32) * node.z.powi(c as i32)
        }))
    }

    #[test]
    fn all_rules_have_unit_nodes_and_weights_sum_to_four_pi() {
        for selected in [
            LebedevRule::Points6,
            LebedevRule::Points14,
            LebedevRule::Points26,
        ] {
            let rule = rule(selected);
            assert_eq!(rule.nodes.len(), selected.node_count());
            assert!((rule.weight_sum - 4.0 * PI).abs() < 1.0e-13);
            assert!(rule.nodes.iter().all(|node| {
                (node.norm_squared() - 1.0).abs() < 1.0e-14 && node.weight > 0.0
            }));
        }
    }

    #[test]
    fn six_point_rule_is_exact_through_degree_three() {
        let rule = rule(LebedevRule::Points6);
        for total in 0..=3 {
            for a in 0..=total {
                for b in 0..=(total - a) {
                    let c = total - a - b;
                    let actual = integrate_monomial(&rule, a, b, c);
                    let expected = exact_sphere_monomial_integral(a, b, c);
                    assert!((actual - expected).abs() < 1.0e-12);
                }
            }
        }
    }

    #[test]
    fn fourteen_point_rule_is_exact_through_degree_five() {
        let rule = rule(LebedevRule::Points14);
        for total in 0..=5 {
            for a in 0..=total {
                for b in 0..=(total - a) {
                    let c = total - a - b;
                    let actual = integrate_monomial(&rule, a, b, c);
                    let expected = exact_sphere_monomial_integral(a, b, c);
                    assert!((actual - expected).abs() < 1.0e-12);
                }
            }
        }
    }

    #[test]
    fn twenty_six_point_rule_is_exact_through_degree_seven() {
        let rule = rule(LebedevRule::Points26);
        for total in 0..=7 {
            for a in 0..=total {
                for b in 0..=(total - a) {
                    let c = total - a - b;
                    let actual = integrate_monomial(&rule, a, b, c);
                    let expected = exact_sphere_monomial_integral(a, b, c);
                    assert!((actual - expected).abs() < 1.0e-12);
                }
            }
        }
    }

    #[test]
    fn constant_integrand_integrates_to_four_pi() {
        for selected in [
            LebedevRule::Points6,
            LebedevRule::Points14,
            LebedevRule::Points26,
        ] {
            let rule = rule(selected);
            let report = integrate_sphere_samples(
                &rule,
                &vec![1.0; rule.nodes.len()],
                SphereQuadratureDiagnosticsConfig::default(),
            )
            .expect("constant integrand");
            assert!((report.integral - 4.0 * PI).abs() < 1.0e-12);
            assert_eq!(report.significant_node_count, rule.nodes.len());
            assert!((report.significant_integral_fraction - 1.0).abs() < 1.0e-14);
        }
    }

    #[test]
    fn nasa_relative_contribution_threshold_is_strict_and_named() {
        let rule = rule(LebedevRule::Points6);
        let mut values = vec![1.0e-3; rule.nodes.len()];
        values[0] = 1.0;
        values[1] = 1.000_000_1e-3;
        let report = integrate_sphere_samples(
            &rule,
            &values,
            SphereQuadratureDiagnosticsConfig {
                relative_contribution_threshold: 1.0e-3,
                minimum_significant_nodes: 0,
            },
        )
        .expect("valid diagnostic");
        assert_eq!(report.significant_node_count, 2);
        assert!((report.significant_weight_fraction - 2.0 / 6.0).abs() < 1.0e-14);
    }

    #[test]
    fn concentrated_integrand_is_marked_as_low_support() {
        let rule = rule(LebedevRule::Points26);
        let mut values = vec![1.0e-8; rule.nodes.len()];
        values[3] = 1.0;
        let report = integrate_sphere_samples(
            &rule,
            &values,
            SphereQuadratureDiagnosticsConfig {
                relative_contribution_threshold: 1.0e-3,
                minimum_significant_nodes: 3,
            },
        )
        .expect("valid concentrated diagnostic");
        assert_eq!(report.significant_node_count, 1);
        assert!(!report.adequacy_satisfied);
        assert!(report.maximum_node_integral_fraction > 0.1);
    }

    #[test]
    fn invalid_sample_shapes_values_and_thresholds_fail_closed() {
        let rule = rule(LebedevRule::Points6);
        assert_eq!(
            integrate_sphere_samples(
                &rule,
                &[1.0],
                SphereQuadratureDiagnosticsConfig::default()
            ),
            Err(KernelError::InvalidSphereQuadratureNodeCount)
        );

        let mut negative = vec![1.0; rule.nodes.len()];
        negative[2] = -1.0;
        assert_eq!(
            integrate_sphere_samples(
                &rule,
                &negative,
                SphereQuadratureDiagnosticsConfig::default()
            ),
            Err(KernelError::InvalidSphereIntegrand)
        );

        let mut non_finite = vec![1.0; rule.nodes.len()];
        non_finite[2] = f64::NAN;
        assert_eq!(
            integrate_sphere_samples(
                &rule,
                &non_finite,
                SphereQuadratureDiagnosticsConfig::default()
            ),
            Err(KernelError::NonFiniteValue)
        );

        assert_eq!(
            integrate_sphere_samples(
                &rule,
                &vec![1.0; rule.nodes.len()],
                SphereQuadratureDiagnosticsConfig {
                    relative_contribution_threshold: f64::NAN,
                    minimum_significant_nodes: 0,
                }
            ),
            Err(KernelError::InvalidSphereQuadratureThreshold)
        );
    }

    #[test]
    fn zero_integrand_is_not_given_a_fake_significance_peak() {
        let rule = rule(LebedevRule::Points6);
        assert_eq!(
            integrate_sphere_samples(
                &rule,
                &vec![0.0; rule.nodes.len()],
                SphereQuadratureDiagnosticsConfig::default()
            ),
            Err(KernelError::ZeroSphereIntegrand)
        );
    }

    #[test]
    fn sphere_rule_order_is_deterministic_and_nested() {
        let six = rule(LebedevRule::Points6);
        let fourteen = rule(LebedevRule::Points14);
        let twenty_six = rule(LebedevRule::Points26);

        let six_points = six
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect::<Vec<_>>();
        let fourteen_points = fourteen
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect::<Vec<_>>();
        let twenty_six_points = twenty_six
            .nodes
            .iter()
            .map(|node| (node.x, node.y, node.z))
            .collect::<Vec<_>>();
        assert_eq!(six_points, fourteen_points[..6]);
        assert_eq!(fourteen_points, twenty_six_points[..14]);
    }
}
