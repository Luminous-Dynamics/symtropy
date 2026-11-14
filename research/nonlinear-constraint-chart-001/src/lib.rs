// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Nonlinear reparameterization of a smooth holonomic constraint.
//!
//! This reference separates residual values from their differentials:
//! nonlinear constraint charts preserve the manifold and transform Jacobians,
//! multipliers and Delassus operators by the local derivative, while
//! acceleration constraints require the second-derivative chain-rule term.

use std::fmt;

pub type Vector2 = [f64; 2];
pub type Matrix2 = [[f64; 2]; 2];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintChartV1 {
    pub cubic_coefficient: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintJetV1 {
    pub residual: f64,
    pub jacobian: Vector2,
    pub hessian: Matrix2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintKinematicsV1 {
    pub residual: f64,
    pub velocity_residual: f64,
    pub acceleration_residual: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProjectionStepV1 {
    pub displacement: Vector2,
    pub residual_before: f64,
    pub residual_after: f64,
    pub metric_displacement_norm: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintChartErrorV1 {
    InvalidChart,
    NonFiniteInput,
    NonFiniteIntermediate,
    SingularMassMetric,
    SingularConstraintDirection,
    NonConvergentProjection,
    InvalidIterationBudget,
}

impl fmt::Display for ConstraintChartErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ConstraintChartErrorV1 {}

impl ConstraintChartV1 {
    pub fn new(cubic_coefficient: f64) -> Result<Self, ConstraintChartErrorV1> {
        if !cubic_coefficient.is_finite() || cubic_coefficient < 0.0 {
            return Err(ConstraintChartErrorV1::InvalidChart);
        }
        Ok(Self { cubic_coefficient })
    }

    pub fn residual(self, base_residual: f64) -> Result<f64, ConstraintChartErrorV1> {
        let output = base_residual + self.cubic_coefficient * base_residual.powi(3);
        finite(output)
    }

    pub fn first_derivative(self, base_residual: f64) -> Result<f64, ConstraintChartErrorV1> {
        let output = 1.0 + 3.0 * self.cubic_coefficient * base_residual.powi(2);
        finite(output)
    }

    pub fn second_derivative(self, base_residual: f64) -> Result<f64, ConstraintChartErrorV1> {
        let output = 6.0 * self.cubic_coefficient * base_residual;
        finite(output)
    }
}

pub fn circle_base_jet(q: Vector2) -> Result<ConstraintJetV1, ConstraintChartErrorV1> {
    validate_vector(q)?;
    let residual = 0.5 * (q[0] * q[0] + q[1] * q[1] - 1.0);
    finite(residual)?;
    Ok(ConstraintJetV1 {
        residual,
        jacobian: q,
        hessian: [[1.0, 0.0], [0.0, 1.0]],
    })
}

pub fn reparameterized_circle_jet(
    q: Vector2,
    chart: ConstraintChartV1,
) -> Result<ConstraintJetV1, ConstraintChartErrorV1> {
    let base = circle_base_jet(q)?;
    let first = chart.first_derivative(base.residual)?;
    let second = chart.second_derivative(base.residual)?;
    let jacobian = [first * base.jacobian[0], first * base.jacobian[1]];
    let hessian = [
        [
            first * base.hessian[0][0] + second * base.jacobian[0] * base.jacobian[0],
            first * base.hessian[0][1] + second * base.jacobian[0] * base.jacobian[1],
        ],
        [
            first * base.hessian[1][0] + second * base.jacobian[1] * base.jacobian[0],
            first * base.hessian[1][1] + second * base.jacobian[1] * base.jacobian[1],
        ],
    ];
    Ok(ConstraintJetV1 {
        residual: chart.residual(base.residual)?,
        jacobian,
        hessian,
    })
}

pub fn constraint_kinematics(
    q: Vector2,
    velocity: Vector2,
    acceleration: Vector2,
    chart: ConstraintChartV1,
) -> Result<ConstraintKinematicsV1, ConstraintChartErrorV1> {
    validate_vector(velocity)?;
    validate_vector(acceleration)?;
    let base = circle_base_jet(q)?;
    let qdot = dot(base.jacobian, velocity)?;
    let qddot_linear = dot(base.jacobian, acceleration)?;
    let curvature = dot(velocity, velocity)?;
    let base_qddot = finite(qddot_linear + curvature)?;

    let first = chart.first_derivative(base.residual)?;
    let second = chart.second_derivative(base.residual)?;
    let chart_velocity = finite(first * qdot)?;
    let chart_acceleration = finite(first * base_qddot + second * qdot * qdot)?;

    Ok(ConstraintKinematicsV1 {
        residual: chart.residual(base.residual)?,
        velocity_residual: chart_velocity,
        acceleration_residual: chart_acceleration,
    })
}

/// Equal generalized constraint force requires lambda_chart = lambda_base / phi'(g).
pub fn transform_multiplier(
    base_multiplier: f64,
    base_residual: f64,
    chart: ConstraintChartV1,
) -> Result<f64, ConstraintChartErrorV1> {
    finite(base_multiplier / chart.first_derivative(base_residual)?)
}

/// Delassus congruence for a scalar constraint is W_chart = phi'(g)^2 W_base.
pub fn transform_delassus(
    base_delassus: f64,
    base_residual: f64,
    chart: ConstraintChartV1,
) -> Result<f64, ConstraintChartErrorV1> {
    let derivative = chart.first_derivative(base_residual)?;
    finite(derivative * base_delassus * derivative)
}

/// Constraint covector in generalized coordinates: J^T * lambda.
pub fn generalized_constraint_force(
    jacobian: Vector2,
    multiplier: f64,
) -> Result<Vector2, ConstraintChartErrorV1> {
    Ok([
        finite(jacobian[0] * multiplier)?,
        finite(jacobian[1] * multiplier)?,
    ])
}

/// A single mass-metric Gauss-Newton normal correction for a scalar residual.
pub fn normal_correction(
    mass: Matrix2,
    jet: ConstraintJetV1,
) -> Result<ProjectionStepV1, ConstraintChartErrorV1> {
    validate_matrix(mass)?;
    let inverse_jacobian = solve_spd_2x2(mass, jet.jacobian)?;
    let denominator = dot(jet.jacobian, inverse_jacobian)?;
    if denominator <= 0.0 {
        return Err(ConstraintChartErrorV1::SingularConstraintDirection);
    }

    let scale = -jet.residual / denominator;
    let displacement = [
        finite(scale * inverse_jacobian[0])?,
        finite(scale * inverse_jacobian[1])?,
    ];
    let mapped = mat_vec(mass, displacement)?;
    let norm_squared = dot(displacement, mapped)?;
    if norm_squared < 0.0 {
        return Err(ConstraintChartErrorV1::SingularMassMetric);
    }
    Ok(ProjectionStepV1 {
        displacement,
        residual_before: jet.residual,
        residual_after: finite(jet.residual + dot(jet.jacobian, displacement)?)?,
        metric_displacement_norm: norm_squared.sqrt(),
    })
}

/// Iterate the chart-specific metric-normal correction to the unit circle.
/// This is an iterative solver, not the exact nearest-point oracle.
pub fn iterate_projection(
    start: Vector2,
    mass: Matrix2,
    chart: ConstraintChartV1,
    maximum_iterations: usize,
    residual_tolerance: f64,
) -> Result<Vector2, ConstraintChartErrorV1> {
    if maximum_iterations == 0 || !residual_tolerance.is_finite() || residual_tolerance <= 0.0 {
        return Err(ConstraintChartErrorV1::InvalidIterationBudget);
    }
    validate_matrix(mass)?;
    let mut q = start;
    for _ in 0..maximum_iterations {
        let jet = reparameterized_circle_jet(q, chart)?;
        if jet.residual.abs() <= residual_tolerance {
            return Ok(q);
        }
        let step = normal_correction(mass, jet)?;
        q[0] = finite(q[0] + step.displacement[0])?;
        q[1] = finite(q[1] + step.displacement[1])?;
    }
    let residual = reparameterized_circle_jet(q, chart)?.residual.abs();
    if residual <= residual_tolerance {
        Ok(q)
    } else {
        Err(ConstraintChartErrorV1::NonConvergentProjection)
    }
}

/// Exact nearest point on the unit circle for the Euclidean metric.
pub fn unit_circle_nearest_point(q: Vector2) -> Result<Vector2, ConstraintChartErrorV1> {
    validate_vector(q)?;
    let norm = dot(q, q)?.sqrt();
    if norm <= f64::MIN_POSITIVE {
        return Err(ConstraintChartErrorV1::SingularConstraintDirection);
    }
    Ok([q[0] / norm, q[1] / norm])
}

pub fn dense_coordinate_map(
    q_from_y: Matrix2,
    y: Vector2,
) -> Result<Vector2, ConstraintChartErrorV1> {
    mat_vec(q_from_y, y)
}

/// Transform the constraint jet from q coordinates into y coordinates where q=S y.
pub fn pullback_constraint_jet(
    q_jet: ConstraintJetV1,
    q_from_y: Matrix2,
) -> Result<ConstraintJetV1, ConstraintChartErrorV1> {
    validate_matrix(q_from_y)?;
    let jacobian_y = mat_vec(transpose(q_from_y), q_jet.jacobian)?;
    let hessian_y = mat_mul(
        &mat_mul(&transpose(q_from_y), q_jet.hessian)?,
        &q_from_y,
    )?;
    Ok(ConstraintJetV1 {
        residual: q_jet.residual,
        jacobian: jacobian_y,
        hessian: hessian_y,
    })
}

fn solve_spd_2x2(matrix: Matrix2, rhs: Vector2) -> Result<Vector2, ConstraintChartErrorV1> {
    let a = matrix[0][0];
    let b = 0.5 * (matrix[0][1] + matrix[1][0]);
    let d = matrix[1][1];
    let determinant = finite(a * d - b * b)?;
    if a <= 0.0 || determinant <= 0.0 {
        return Err(ConstraintChartErrorV1::SingularMassMetric);
    }
    let solution = [
        finite((d * rhs[0] - b * rhs[1]) / determinant)?,
        finite((a * rhs[1] - b * rhs[0]) / determinant)?,
    ];
    Ok(solution)
}

fn validate_matrix(matrix: Matrix2) -> Result<(), ConstraintChartErrorV1> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ConstraintChartErrorV1::NonFiniteInput);
    }
    if (matrix[0][1] - matrix[1][0]).abs() > 1.0e-12 {
        return Err(ConstraintChartErrorV1::SingularMassMetric);
    }
    if matrix[0][0] <= 0.0
        || matrix[1][1] <= 0.0
        || matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0] <= 0.0
    {
        return Err(ConstraintChartErrorV1::SingularMassMetric);
    }
    Ok(())
}

fn validate_vector(vector: Vector2) -> Result<(), ConstraintChartErrorV1> {
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(ConstraintChartErrorV1::NonFiniteInput);
    }
    Ok(())
}

fn finite(value: f64) -> Result<f64, ConstraintChartErrorV1> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ConstraintChartErrorV1::NonFiniteIntermediate)
    }
}

fn dot(left: Vector2, right: Vector2) -> Result<f64, ConstraintChartErrorV1> {
    finite(left[0] * right[0] + left[1] * right[1])
}

fn mat_vec(matrix: Matrix2, vector: Vector2) -> Result<Vector2, ConstraintChartErrorV1> {
    Ok([
        finite(matrix[0][0] * vector[0] + matrix[0][1] * vector[1])?,
        finite(matrix[1][0] * vector[0] + matrix[1][1] * vector[1])?,
    ])
}

fn mat_mul(left: &Matrix2, right: &Matrix2) -> Result<Matrix2, ConstraintChartErrorV1> {
    let mut output = [[0.0; 2]; 2];
    for row in 0..2 {
        for column in 0..2 {
            output[row][column] = finite(
                left[row][0] * right[0][column] + left[row][1] * right[1][column],
            )?;
        }
    }
    Ok(output)
}

fn transpose(matrix: Matrix2) -> Matrix2 {
    [[matrix[0][0], matrix[1][0]], [matrix[0][1], matrix[1][1]]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_chart() -> ConstraintChartV1 {
        ConstraintChartV1::new(0.0).unwrap()
    }

    fn nonlinear_chart() -> ConstraintChartV1 {
        ConstraintChartV1::new(2.0).unwrap()
    }

    fn assert_close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0),
            "left={left:.17e}, right={right:.17e}, tolerance={tolerance:.3e}"
        );
    }

    fn assert_vector_close(left: Vector2, right: Vector2, tolerance: f64) {
        assert_close(left[0], right[0], tolerance);
        assert_close(left[1], right[1], tolerance);
    }

    #[test]
    fn nonlinear_chart_has_the_same_unique_zero_set() {
        for q in [[1.0, 0.0], [0.6, 0.8], [1.3, 0.4], [0.4, 0.3]] {
            let base = circle_base_jet(q).unwrap().residual;
            let transformed = nonlinear_chart().residual(base).unwrap();
            assert_eq!(base == 0.0, transformed == 0.0);
            assert!(nonlinear_chart().first_derivative(base).unwrap() > 0.0);
        }
    }

    #[test]
    fn nonlinear_jacobian_and_multiplier_transform_preserve_generalized_force() {
        let q = [1.4, 0.7];
        let base = circle_base_jet(q).unwrap();
        let transformed = reparameterized_circle_jet(q, nonlinear_chart()).unwrap();
        let derivative = nonlinear_chart().first_derivative(base.residual).unwrap();
        let lambda_base = 3.25;
        let lambda_chart =
            transform_multiplier(lambda_base, base.residual, nonlinear_chart()).unwrap();

        assert_vector_close(
            transformed.jacobian,
            [derivative * base.jacobian[0], derivative * base.jacobian[1]],
            1.0e-14,
        );
        assert_vector_close(
            generalized_constraint_force(base.jacobian, lambda_base).unwrap(),
            generalized_constraint_force(transformed.jacobian, lambda_chart).unwrap(),
            1.0e-14,
        );
    }

    #[test]
    fn delassus_operator_obeys_local_derivative_congruence() {
        let q = [1.2, -0.8];
        let base = circle_base_jet(q).unwrap();
        let transformed = reparameterized_circle_jet(q, nonlinear_chart()).unwrap();
        let base_w = dot(base.jacobian, base.jacobian).unwrap();
        let transformed_w = dot(transformed.jacobian, transformed.jacobian).unwrap();
        let expected = transform_delassus(
            base_w,
            base.residual,
            nonlinear_chart(),
        )
        .unwrap();
        assert_close(transformed_w, expected, 1.0e-14);
    }

    #[test]
    fn acceleration_chain_rule_includes_nonlinear_chart_curvature() {
        let q = [1.2, 0.7];
        let velocity = [0.3, -0.2];
        let acceleration = [-0.1, 0.4];
        let chart = nonlinear_chart();
        let base = constraint_kinematics(q, velocity, acceleration, identity_chart()).unwrap();
        let transformed =
            constraint_kinematics(q, velocity, acceleration, chart).unwrap();
        let base_residual = circle_base_jet(q).unwrap().residual;
        let expected = chart.first_derivative(base_residual).unwrap()
            * base.acceleration_residual
            + chart.second_derivative(base_residual).unwrap()
                * base.velocity_residual.powi(2);
        assert_close(transformed.acceleration_residual, expected, 1.0e-14);

        let jet = reparameterized_circle_jet(q, chart).unwrap();
        let direct = dot(jet.jacobian, acceleration).unwrap()
            + dot(velocity, mat_vec(jet.hessian, velocity).unwrap()).unwrap();
        assert_close(transformed.acceleration_residual, direct, 1.0e-14);
    }

    #[test]
    fn nonlinear_residual_value_pairing_is_not_a_covector_invariant() {
        let q = [1.4, 0.7];
        let base = circle_base_jet(q).unwrap();
        let chart = nonlinear_chart();
        let lambda_base = 1.7;
        let lambda_chart =
            transform_multiplier(lambda_base, base.residual, chart).unwrap();
        let base_pairing = lambda_base * base.residual;
        let transformed_pairing = lambda_chart * chart.residual(base.residual).unwrap();

        assert!((base_pairing - transformed_pairing).abs() > 1.0e-3);
    }

    #[test]
    fn one_step_normal_correction_depends_on_nonlinear_residual_chart() {
        let q = [1.5, 0.8];
        let mass = [[1.0, 0.0], [0.0, 1.0]];
        let base_step = normal_correction(mass, circle_base_jet(q).unwrap()).unwrap();
        let chart_step =
            normal_correction(mass, reparameterized_circle_jet(q, nonlinear_chart()).unwrap())
                .unwrap();

        assert!(
            (base_step.displacement[0] - chart_step.displacement[0]).abs() > 1.0e-2
                || (base_step.displacement[1] - chart_step.displacement[1]).abs() > 1.0e-2
        );
    }

    #[test]
    fn both_constraint_charts_converge_to_the_same_exact_nearest_point() {
        let start = [1.5, 0.8];
        let mass = [[1.0, 0.0], [0.0, 1.0]];
        let exact = unit_circle_nearest_point(start).unwrap();
        let base = iterate_projection(start, mass, identity_chart(), 80, 2.0e-13).unwrap();
        let transformed =
            iterate_projection(start, mass, nonlinear_chart(), 80, 2.0e-13).unwrap();

        assert_vector_close(base, exact, 2.0e-12);
        assert_vector_close(transformed, exact, 2.0e-11);
    }

    #[test]
    fn correction_is_covariant_under_dense_generalized_coordinate_rebasing() {
        let q = [1.5, 0.8];
        let s = [[1.0, 2.0], [0.0, 1.0]];
        let y = [q[0] - 2.0 * q[1], q[1]];
        let mass_q = [[1.0, 0.0], [0.0, 1.0]];
        let mass_y = [[1.0, 2.0], [2.0, 5.0]];

        let jet_q = reparameterized_circle_jet(q, nonlinear_chart()).unwrap();
        let step_q = normal_correction(mass_q, jet_q).unwrap();

        let jet_y = pullback_constraint_jet(jet_q, s).unwrap();
        let step_y = normal_correction(mass_y, jet_y).unwrap();
        let pushed_step_y = mat_vec(s, step_y.displacement).unwrap();

        assert_vector_close(step_q.displacement, pushed_step_y, 2.0e-13);
    }

    #[test]
    fn nonlinear_chart_derivative_never_crosses_zero_and_invalid_profiles_fail_closed() {
        assert!(ConstraintChartV1::new(-0.1).is_err());
        assert!(ConstraintChartV1::new(f64::NAN).is_err());
        for residual in [-10.0, -1.0, 0.0, 0.5, 10.0] {
            assert!(nonlinear_chart().first_derivative(residual).unwrap() > 0.0);
        }
    }

    #[test]
    fn singular_constraint_direction_and_nonfinite_states_are_rejected() {
        assert!(matches!(
            normal_correction(
                [[1.0, 0.0], [0.0, 1.0]],
                circle_base_jet([0.0, 0.0]).unwrap()
            ),
            Err(ConstraintChartErrorV1::SingularConstraintDirection)
        ));
        assert_eq!(
            circle_base_jet([f64::INFINITY, 0.0]),
            Err(ConstraintChartErrorV1::NonFiniteInput)
        );
    }
}
