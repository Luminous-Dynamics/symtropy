// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Two-constraint nonlinear reparameterization with exact inverse and chain rules.
//!
//! The map phi(g1,g2)=(g1,g2+beta*g1^2) is a global diffeomorphism.
//! Its Jacobian transforms constraint covectors and Delassus operators, while
//! its Hessian contributes a nonzero acceleration-level curvature term.

use std::fmt;

pub type Vector2 = [f64; 2];
pub type Vector3 = [f64; 3];
pub type Matrix2 = [[f64; 2]; 2];
pub type Matrix3 = [[f64; 3]; 3];
pub type Matrix2x3 = [[f64; 3]; 2];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NonlinearMixingChartV1 {
    pub beta: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintJetV1 {
    pub residual: Vector2,
    pub jacobian: Matrix2x3,
    pub hessians: [Matrix3; 2],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintKinematicsV1 {
    pub residual: Vector2,
    pub velocity_residual: Vector2,
    pub acceleration_residual: Vector2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MixingErrorV1 {
    InvalidChart,
    NonFiniteInput,
    NonFiniteIntermediate,
    SingularMassMetric,
    SingularConstraintJacobian,
    ArithmeticOverflow,
}

impl fmt::Display for MixingErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for MixingErrorV1 {}

impl NonlinearMixingChartV1 {
    pub fn new(beta: f64) -> Result<Self, MixingErrorV1> {
        if !beta.is_finite() || beta < 0.0 {
            return Err(MixingErrorV1::InvalidChart);
        }
        Ok(Self { beta })
    }

    pub fn forward(self, g: Vector2) -> Result<Vector2, MixingErrorV1> {
        validate_vector2(g)?;
        Ok([g[0], finite(g[1] + self.beta * g[0] * g[0])?])
    }

    pub fn inverse(self, h: Vector2) -> Result<Vector2, MixingErrorV1> {
        validate_vector2(h)?;
        Ok([h[0], finite(h[1] - self.beta * h[0] * h[0])?])
    }

    pub fn jacobian(self, g: Vector2) -> Result<Matrix2, MixingErrorV1> {
        validate_vector2(g)?;
        Ok([[1.0, 0.0], [finite(2.0 * self.beta * g[0])?, 1.0]])
    }

    /// The only nonzero component of D²phi[g_dot,g_dot] is 2*beta*g_dot_1².
    pub fn acceleration_curvature(self, g_dot: Vector2) -> Result<Vector2, MixingErrorV1> {
        validate_vector2(g_dot)?;
        Ok([0.0, finite(2.0 * self.beta * g_dot[0] * g_dot[0])?])
    }

    /// lambda_new = T^-T lambda_old, preserving J^T lambda.
    pub fn transform_multiplier(
        self,
        g: Vector2,
        lambda_old: Vector2,
    ) -> Result<Vector2, MixingErrorV1> {
        validate_vector2(lambda_old)?;
        let t = self.jacobian(g)?;
        let inverse_transpose = [[1.0, -t[1][0]], [0.0, 1.0]];
        mat_vec2(inverse_transpose, lambda_old)
    }

    /// W_new = T W_old T^T.
    pub fn transform_delassus(
        self,
        g: Vector2,
        w_old: Matrix2,
    ) -> Result<Matrix2, MixingErrorV1> {
        let t = self.jacobian(g)?;
        mat_mul2(&mat_mul2(&t, &w_old)?, &transpose2(t))
    }
}

/// A regular codimension-two manifold in R³: a unit circle and an independent plane.
pub fn base_constraint_jet(q: Vector3) -> Result<ConstraintJetV1, MixingErrorV1> {
    validate_vector3(q)?;
    let g1 = 0.5 * (q[0] * q[0] + q[1] * q[1] - 1.0);
    let g2 = q[2] - 0.25 * q[0];
    Ok(ConstraintJetV1 {
        residual: [finite(g1)?, finite(g2)?],
        jacobian: [
            [q[0], q[1], 0.0],
            [-0.25, 0.0, 1.0],
        ],
        hessians: [
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]],
            [[0.0; 3]; 3],
        ],
    })
}

pub fn reparameterized_constraint_jet(
    q: Vector3,
    chart: NonlinearMixingChartV1,
) -> Result<ConstraintJetV1, MixingErrorV1> {
    let base = base_constraint_jet(q)?;
    let t = chart.jacobian(base.residual)?;
    let jacobian = mat_mul_2x2_2x3(t, base.jacobian)?;
    let mut hessian_second = base.hessians[1];
    let outer = outer3(base.jacobian[0], base.jacobian[0])?;
    for row in 0..3 {
        for column in 0..3 {
            hessian_second[row][column] =
                finite(hessian_second[row][column] + 2.0 * chart.beta * outer[row][column]
                    + 2.0 * chart.beta * base.residual[0] * base.hessians[0][row][column])?;
        }
    }

    Ok(ConstraintJetV1 {
        residual: chart.forward(base.residual)?,
        jacobian,
        hessians: [base.hessians[0], hessian_second],
    })
}

pub fn constraint_kinematics(
    q: Vector3,
    velocity: Vector3,
    acceleration: Vector3,
    chart: NonlinearMixingChartV1,
) -> Result<ConstraintKinematicsV1, MixingErrorV1> {
    validate_vector3(velocity)?;
    validate_vector3(acceleration)?;
    let base = base_constraint_jet(q)?;
    let base_dot = mat_vec_2x3(base.jacobian, velocity)?;
    let base_linear = mat_vec_2x3(base.jacobian, acceleration)?;
    let mut base_ddot = [0.0; 2];
    for index in 0..2 {
        base_ddot[index] =
            finite(base_linear[index] + quadratic3(velocity, base.hessians[index])?)?;
    }

    let t = chart.jacobian(base.residual)?;
    let transformed_dot = mat_vec2(t, base_dot)?;
    let mut transformed_ddot = mat_vec2(t, base_ddot)?;
    let curvature = chart.acceleration_curvature(base_dot)?;
    transformed_ddot[0] = finite(transformed_ddot[0] + curvature[0])?;
    transformed_ddot[1] = finite(transformed_ddot[1] + curvature[1])?;

    Ok(ConstraintKinematicsV1 {
        residual: chart.forward(base.residual)?,
        velocity_residual: transformed_dot,
        acceleration_residual: transformed_ddot,
    })
}

pub fn generalized_constraint_force(
    jacobian: Matrix2x3,
    multiplier: Vector2,
) -> Result<Vector3, MixingErrorV1> {
    validate_vector2(multiplier)?;
    let mut force = [0.0; 3];
    for column in 0..3 {
        force[column] = finite(
            jacobian[0][column] * multiplier[0]
                + jacobian[1][column] * multiplier[1],
        )?;
    }
    Ok(force)
}

pub fn delassus(
    jacobian: Matrix2x3,
    mass: Matrix3,
) -> Result<Matrix2, MixingErrorV1> {
    let mut inverse_rows = [[0.0; 3]; 2];
    for row in 0..2 {
        inverse_rows[row] = solve_spd3(mass, jacobian[row])?;
    }
    let mut output = [[0.0; 2]; 2];
    for row in 0..2 {
        for column in 0..2 {
            output[row][column] = dot3(jacobian[row], inverse_rows[column])?;
        }
    }
    Ok(output)
}

pub fn transform_jacobian_coordinates(
    jacobian_q: Matrix2x3,
    q_from_y: Matrix3,
) -> Result<Matrix2x3, MixingErrorV1> {
    mat_mul_2x3_3(jacobian_q, q_from_y)
}

pub fn pullback_constraint_jet_coordinates(
    jet_q: ConstraintJetV1,
    q_from_y: Matrix3,
) -> Result<ConstraintJetV1, MixingErrorV1> {
    let jacobian_y = transform_jacobian_coordinates(jet_q.jacobian, q_from_y)?;
    let transpose = transpose3(q_from_y);
    let hessian_y = [
        mat_mul3(&mat_mul3(&transpose, &jet_q.hessians[0])?, &q_from_y)?,
        mat_mul3(&mat_mul3(&transpose, &jet_q.hessians[1])?, &q_from_y)?,
    ];
    Ok(ConstraintJetV1 {
        residual: jet_q.residual,
        jacobian: jacobian_y,
        hessians: hessian_y,
    })
}

pub fn pullback_mass_metric(
    mass_q: Matrix3,
    q_from_y: Matrix3,
) -> Result<Matrix3, MixingErrorV1> {
    mat_mul3(&mat_mul3(&transpose3(q_from_y), &mass_q)?, &q_from_y)
}

fn validate_vector2(value: Vector2) -> Result<(), MixingErrorV1> {
    if value.iter().any(|item| !item.is_finite()) {
        Err(MixingErrorV1::NonFiniteInput)
    } else {
        Ok(())
    }
}

fn validate_vector3(value: Vector3) -> Result<(), MixingErrorV1> {
    if value.iter().any(|item| !item.is_finite()) {
        Err(MixingErrorV1::NonFiniteInput)
    } else {
        Ok(())
    }
}

fn finite(value: f64) -> Result<f64, MixingErrorV1> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(MixingErrorV1::NonFiniteIntermediate)
    }
}

fn mat_vec2(matrix: Matrix2, vector: Vector2) -> Result<Vector2, MixingErrorV1> {
    Ok([
        finite(matrix[0][0] * vector[0] + matrix[0][1] * vector[1])?,
        finite(matrix[1][0] * vector[0] + matrix[1][1] * vector[1])?,
    ])
}

fn mat_mul2(left: &Matrix2, right: &Matrix2) -> Result<Matrix2, MixingErrorV1> {
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

fn transpose2(matrix: Matrix2) -> Matrix2 {
    [[matrix[0][0], matrix[1][0]], [matrix[0][1], matrix[1][1]]]
}

fn mat_mul_2x2_2x3(
    left: Matrix2,
    right: Matrix2x3,
) -> Result<Matrix2x3, MixingErrorV1> {
    let mut output = [[0.0; 3]; 2];
    for row in 0..2 {
        for column in 0..3 {
            output[row][column] = finite(
                left[row][0] * right[0][column] + left[row][1] * right[1][column],
            )?;
        }
    }
    Ok(output)
}

fn mat_mul_2x3_3(
    left: Matrix2x3,
    right: Matrix3,
) -> Result<Matrix2x3, MixingErrorV1> {
    let mut output = [[0.0; 3]; 2];
    for row in 0..2 {
        for column in 0..3 {
            output[row][column] = finite(
                left[row][0] * right[0][column]
                    + left[row][1] * right[1][column]
                    + left[row][2] * right[2][column],
            )?;
        }
    }
    Ok(output)
}

fn mat_vec_2x3(matrix: Matrix2x3, vector: Vector3) -> Result<Vector2, MixingErrorV1> {
    Ok([
        finite(matrix[0][0] * vector[0] + matrix[0][1] * vector[1] + matrix[0][2] * vector[2])?,
        finite(matrix[1][0] * vector[0] + matrix[1][1] * vector[1] + matrix[1][2] * vector[2])?,
    ])
}

fn outer3(left: [f64; 3], right: [f64; 3]) -> Result<Matrix3, MixingErrorV1> {
    let mut output = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            output[row][column] = finite(left[row] * right[column])?;
        }
    }
    Ok(output)
}

fn quadratic3(vector: Vector3, matrix: Matrix3) -> Result<f64, MixingErrorV1> {
    dot3(vector, mat_vec3(matrix, vector)?)
}

fn dot3(left: Vector3, right: Vector3) -> Result<f64, MixingErrorV1> {
    finite(left[0] * right[0] + left[1] * right[1] + left[2] * right[2])
}

fn mat_vec3(matrix: Matrix3, vector: Vector3) -> Result<Vector3, MixingErrorV1> {
    let mut output = [0.0; 3];
    for row in 0..3 {
        output[row] = finite(
            matrix[row][0] * vector[0]
                + matrix[row][1] * vector[1]
                + matrix[row][2] * vector[2],
        )?;
    }
    Ok(output)
}

fn transpose3(matrix: Matrix3) -> Matrix3 {
    [
        [matrix[0][0], matrix[1][0], matrix[2][0]],
        [matrix[0][1], matrix[1][1], matrix[2][1]],
        [matrix[0][2], matrix[1][2], matrix[2][2]],
    ]
}

fn mat_mul3(left: &Matrix3, right: &Matrix3) -> Result<Matrix3, MixingErrorV1> {
    let mut output = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            output[row][column] = finite(
                left[row][0] * right[0][column]
                    + left[row][1] * right[1][column]
                    + left[row][2] * right[2][column],
            )?;
        }
    }
    Ok(output)
}

fn solve_spd3(matrix: Matrix3, rhs: Vector3) -> Result<Vector3, MixingErrorV1> {
    let mut lower = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..=row {
            let mut sum = matrix[row][column];
            for inner in 0..column {
                sum -= lower[row][inner] * lower[column][inner];
            }
            if !sum.is_finite() {
                return Err(MixingErrorV1::ArithmeticOverflow);
            }
            if row == column {
                if sum <= 0.0 {
                    return Err(MixingErrorV1::SingularMassMetric);
                }
                lower[row][column] = sum.sqrt();
            } else {
                lower[row][column] = sum / lower[column][column];
            }
        }
    }

    let mut solution = rhs;
    for row in 0..3 {
        for column in 0..row {
            solution[row] -= lower[row][column] * solution[column];
        }
        solution[row] /= lower[row][row];
    }
    for row in (0..3).rev() {
        for column in (row + 1)..3 {
            solution[row] -= lower[column][row] * solution[column];
        }
        solution[row] /= lower[row][row];
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err(MixingErrorV1::ArithmeticOverflow);
    }
    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart() -> NonlinearMixingChartV1 {
        NonlinearMixingChartV1::new(0.75).unwrap()
    }

    fn q() -> Vector3 {
        [1.25, -0.5, 0.25]
    }

    fn mass_q() -> Matrix3 {
        [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]]
    }

    fn assert_close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0),
            "left={left:.17e}, right={right:.17e}, tol={tolerance:.3e}"
        );
    }

    fn assert_vec2_close(left: Vector2, right: Vector2, tolerance: f64) {
        assert_close(left[0], right[0], tolerance);
        assert_close(left[1], right[1], tolerance);
    }

    fn assert_vec3_close(left: Vector3, right: Vector3, tolerance: f64) {
        for index in 0..3 {
            assert_close(left[index], right[index], tolerance);
        }
    }

    #[test]
    fn nonlinear_mixing_map_has_exact_global_inverse_and_preserves_zero_set() {
        let mapping = chart();
        for residual in [[0.0, 0.0], [0.25, -0.5], [-0.75, 0.125], [2.0, 1.5]] {
            let transformed = mapping.forward(residual).unwrap();
            assert_vec2_close(mapping.inverse(transformed).unwrap(), residual, 1.0e-14);
            assert_eq!(residual == [0.0, 0.0], transformed == [0.0, 0.0]);
        }
        assert_eq!(
            NonlinearMixingChartV1::new(-1.0),
            Err(MixingErrorV1::InvalidChart)
        );
    }

    #[test]
    fn jacobian_multiplier_and_force_are_covariant_under_nonlinear_mixing() {
        let mapping = chart();
        let base = base_constraint_jet(q()).unwrap();
        let transformed = reparameterized_constraint_jet(q(), mapping).unwrap();
        let lambda_base = [1.25, -0.75];
        let lambda_new = mapping
            .transform_multiplier(base.residual, lambda_base)
            .unwrap();
        let base_force = generalized_constraint_force(base.jacobian, lambda_base).unwrap();
        let new_force =
            generalized_constraint_force(transformed.jacobian, lambda_new).unwrap();

        assert_vec3_close(base_force, new_force, 2.0e-14);
    }

    #[test]
    fn delassus_operator_transforms_by_two_sided_congruence() {
        let mapping = chart();
        let base = base_constraint_jet(q()).unwrap();
        let transformed = reparameterized_constraint_jet(q(), mapping).unwrap();
        let old_w = delassus(base.jacobian, mass_q()).unwrap();
        let new_w = delassus(transformed.jacobian, mass_q()).unwrap();
        let expected = mapping
            .transform_delassus(base.residual, old_w)
            .unwrap();
        for row in 0..2 {
            for column in 0..2 {
                assert_close(new_w[row][column], expected[row][column], 2.0e-13);
            }
        }
    }

    #[test]
    fn acceleration_chain_rule_contains_mixed_curvature_term() {
        let q_value = [1.2, 0.7, 0.1];
        let velocity = [0.3, -0.2, 0.15];
        let acceleration = [-0.1, 0.4, 0.2];
        let mapping = chart();
        let base = constraint_kinematics(
            q_value,
            velocity,
            acceleration,
            NonlinearMixingChartV1::new(0.0).unwrap(),
        )
        .unwrap();
        let transformed =
            constraint_kinematics(q_value, velocity, acceleration, mapping).unwrap();
        let g = base_constraint_jet(q_value).unwrap().residual;
        let t = mapping.jacobian(g).unwrap();
        let expected_linear = mat_vec2(t, base.acceleration_residual).unwrap();
        let expected_curvature = mapping
            .acceleration_curvature(base.velocity_residual)
            .unwrap();
        assert_close(
            transformed.acceleration_residual[0],
            expected_linear[0] + expected_curvature[0],
            1.0e-14,
        );
        assert_close(
            transformed.acceleration_residual[1],
            expected_linear[1] + expected_curvature[1],
            1.0e-14,
        );

        let jet = reparameterized_constraint_jet(q_value, mapping).unwrap();
        let direct = [
            dot3(jet.jacobian[0], acceleration).unwrap()
                + quadratic3(velocity, jet.hessians[0]).unwrap(),
            dot3(jet.jacobian[1], acceleration).unwrap()
                + quadratic3(velocity, jet.hessians[1]).unwrap(),
        ];
        assert_vec2_close(transformed.acceleration_residual, direct, 2.0e-14);
        assert!(expected_curvature[1].abs() > 0.0);
    }

    #[test]
    fn omitting_curvature_is_a_detected_negative_control() {
        let q_value = [1.2, 0.7, 0.1];
        let velocity = [0.3, -0.2, 0.15];
        let acceleration = [-0.1, 0.4, 0.2];
        let mapping = chart();
        let base = constraint_kinematics(
            q_value,
            velocity,
            acceleration,
            NonlinearMixingChartV1::new(0.0).unwrap(),
        )
        .unwrap();
        let transformed =
            constraint_kinematics(q_value, velocity, acceleration, mapping).unwrap();
        let g = base_constraint_jet(q_value).unwrap().residual;
        let naive = mat_vec2(mapping.jacobian(g).unwrap(), base.acceleration_residual).unwrap();
        assert!(
            (transformed.acceleration_residual[1] - naive[1]).abs() > 1.0e-3
        );
    }

    #[test]
    fn virtual_work_pairing_is_invariant_but_finite_residual_pairing_is_not() {
        let mapping = chart();
        let base = base_constraint_jet(q()).unwrap();
        let lambda_base = [0.75, -1.25];
        let lambda_new = mapping
            .transform_multiplier(base.residual, lambda_base)
            .unwrap();
        let delta_g = [0.125, -0.25];
        let delta_h = mat_vec2(mapping.jacobian(base.residual).unwrap(), delta_g).unwrap();

        let old_virtual_work = dot2(lambda_base, delta_g).unwrap();
        let new_virtual_work = dot2(lambda_new, delta_h).unwrap();
        assert_close(old_virtual_work, new_virtual_work, 1.0e-14);

        let old_residual_pairing = dot2(lambda_base, base.residual).unwrap();
        let new_residual_pairing = dot2(lambda_new, mapping.forward(base.residual).unwrap()).unwrap();
        assert!((old_residual_pairing - new_residual_pairing).abs() > 1.0e-4);
    }

    #[test]
    fn nonlinear_chart_commutes_with_dense_generalized_coordinate_rebasing() {
        let mapping = chart();
        let s = [[1.0, 2.0, 0.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]];
        let q_value = q();
        let y = [q_value[0] - 2.0 * (q_value[1] - q_value[2]), q_value[1] - q_value[2], q_value[2]];
        let mass_y = pullback_mass_metric(mass_q(), s).unwrap();

        let jet_q = reparameterized_constraint_jet(q_value, mapping).unwrap();
        let q_from_y = dense_coordinate_map(s, y).unwrap();
        let jet_y_at_same_q = reparameterized_constraint_jet(q_from_y, mapping).unwrap();
        let jet_y = pullback_constraint_jet_coordinates(jet_y_at_same_q, s).unwrap();
        let expected_j_y = transform_jacobian_coordinates(jet_q.jacobian, s).unwrap();
        for row in 0..2 {
            for column in 0..3 {
                assert_close(jet_y.jacobian[row][column], expected_j_y[row][column], 1.0e-14);
            }
        }

        let w_q = delassus(jet_q.jacobian, mass_q()).unwrap();
        let w_y = delassus(jet_y.jacobian, mass_y).unwrap();
        for row in 0..2 {
            for column in 0..2 {
                assert_close(w_q[row][column], w_y[row][column], 2.0e-13);
            }
        }
    }

    #[test]
    fn nonlinear_mixing_sweep_keeps_derivative_invertible_and_delassus_covariant() {
        let q_value = [1.3, -0.4, 0.2];
        let base = base_constraint_jet(q_value).unwrap();
        let old_w = delassus(base.jacobian, mass_q()).unwrap();
        for beta in [0.0, 0.125, 0.5, 1.0, 4.0, 8.0] {
            let mapping = NonlinearMixingChartV1::new(beta).unwrap();
            let t = mapping.jacobian(base.residual).unwrap();
            let determinant = t[0][0] * t[1][1] - t[0][1] * t[1][0];
            assert_close(determinant, 1.0, 1.0e-15);
            let transformed = reparameterized_constraint_jet(q_value, mapping).unwrap();
            let new_w = delassus(transformed.jacobian, mass_q()).unwrap();
            let expected = mapping.transform_delassus(base.residual, old_w).unwrap();
            for row in 0..2 {
                for column in 0..2 {
                    assert_close(new_w[row][column], expected[row][column], 2.0e-12);
                }
            }
        }
    }

    #[test]
    fn singular_mass_metric_and_nonfinite_inputs_fail_closed() {
        assert_eq!(
            delassus(
                base_constraint_jet(q()).unwrap().jacobian,
                [[1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]]
            ),
            Err(MixingErrorV1::SingularMassMetric)
        );
        assert_eq!(
            base_constraint_jet([f64::NAN, 0.0, 1.0]),
            Err(MixingErrorV1::NonFiniteInput)
        );
    }
}

fn dot2(left: Vector2, right: Vector2) -> Result<f64, MixingErrorV1> {
    finite(left[0] * right[0] + left[1] * right[1])
}
