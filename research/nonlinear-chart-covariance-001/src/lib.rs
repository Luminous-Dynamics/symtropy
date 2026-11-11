// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Differential pullback identities for a smooth nonlinear six-dimensional chart.
//!
//! This is not a discrete-mechanics solver. It isolates the coordinate laws whose
//! missing Hessian term is invisible to constant linear-basis covariance tests.

// Explicit fixed-size indices keep the tensor contractions directly auditable.
#![allow(clippy::needless_range_loop)]

use std::fmt;

pub const DIMENSION: usize = 6;
pub type Vector6 = [f64; DIMENSION];
pub type Matrix6 = [[f64; DIMENSION]; DIMENSION];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartError {
    NonFiniteAmplitude,
    NonFiniteCoordinate,
    NonFiniteVector,
    NonFiniteMatrix,
    NonFiniteResult,
}

impl fmt::Display for ChartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ChartError {}

fn finite_vector(value: &Vector6) -> bool {
    value.iter().all(|entry| entry.is_finite())
}

fn finite_matrix(value: &Matrix6) -> bool {
    value.iter().flatten().all(|entry| entry.is_finite())
}

fn validate_amplitude(amplitude: f64) -> Result<(), ChartError> {
    if !amplitude.is_finite() {
        return Err(ChartError::NonFiniteAmplitude);
    }
    Ok(())
}

fn validate_vector(value: &Vector6) -> Result<(), ChartError> {
    if !finite_vector(value) {
        return Err(ChartError::NonFiniteVector);
    }
    Ok(())
}

/// Evaluate q = phi(y), where q0 = y0 + a sin(y1).
pub fn map_y_to_q(y: Vector6, amplitude: f64) -> Result<Vector6, ChartError> {
    validate_amplitude(amplitude)?;
    validate_vector(&y)?;
    let mut q = y;
    q[0] += amplitude * y[1].sin();
    validate_vector(&q).map_err(|_| ChartError::NonFiniteResult)?;
    Ok(q)
}

/// Evaluate the exact inverse y = phi^{-1}(q).
pub fn map_q_to_y(q: Vector6, amplitude: f64) -> Result<Vector6, ChartError> {
    validate_amplitude(amplitude)?;
    validate_vector(&q)?;
    let mut y = q;
    y[0] -= amplitude * q[1].sin();
    validate_vector(&y).map_err(|_| ChartError::NonFiniteResult)?;
    Ok(y)
}

/// Return J(y) = D phi(y). The chart has det(J) = 1 for every finite y and a.
pub fn jacobian(y: Vector6, amplitude: f64) -> Result<Matrix6, ChartError> {
    validate_amplitude(amplitude)?;
    validate_vector(&y)?;
    let mut result = [[0.0; DIMENSION]; DIMENSION];
    for index in 0..DIMENSION {
        result[index][index] = 1.0;
    }
    result[0][1] = amplitude * y[1].cos();
    if !finite_matrix(&result) {
        return Err(ChartError::NonFiniteResult);
    }
    Ok(result)
}

/// Return D^2 phi(y)[ydot, ydot]. Only component 0 is non-zero.
pub fn hessian_velocity_contraction(
    y: Vector6,
    ydot: Vector6,
    amplitude: f64,
) -> Result<Vector6, ChartError> {
    validate_amplitude(amplitude)?;
    validate_vector(&y)?;
    validate_vector(&ydot)?;
    let mut result = [0.0; DIMENSION];
    result[0] = -amplitude * y[1].sin() * ydot[1] * ydot[1];
    validate_vector(&result).map_err(|_| ChartError::NonFiniteResult)?;
    Ok(result)
}

/// Correct coordinate acceleration: qddot = J yddot + D^2 phi[ydot, ydot].
pub fn transform_acceleration(
    y: Vector6,
    ydot: Vector6,
    yddot: Vector6,
    amplitude: f64,
) -> Result<Vector6, ChartError> {
    validate_vector(&yddot)?;
    let derivative = jacobian(y, amplitude)?;
    let curvature = hessian_velocity_contraction(y, ydot, amplitude)?;
    let mut result = [0.0; DIMENSION];
    for row in 0..DIMENSION {
        for column in 0..DIMENSION {
            result[row] += derivative[row][column] * yddot[column];
        }
        result[row] += curvature[row];
    }
    validate_vector(&result).map_err(|_| ChartError::NonFiniteResult)?;
    Ok(result)
}

/// Pull back a covariant symmetric or nonsymmetric rank-two tensor as J^T M J.
pub fn pullback_metric(
    y: Vector6,
    amplitude: f64,
    metric_q: Matrix6,
) -> Result<Matrix6, ChartError> {
    validate_vector(&y)?;
    if !finite_matrix(&metric_q) {
        return Err(ChartError::NonFiniteMatrix);
    }
    let j = jacobian(y, amplitude)?;
    let mut result = [[0.0; DIMENSION]; DIMENSION];
    for row in 0..DIMENSION {
        for column in 0..DIMENSION {
            for left in 0..DIMENSION {
                for right in 0..DIMENSION {
                    result[row][column] += j[left][row] * metric_q[left][right] * j[right][column];
                }
            }
        }
    }
    if !finite_matrix(&result) {
        return Err(ChartError::NonFiniteResult);
    }
    Ok(result)
}

/// Pull back a covector, including generalized force and canonical momentum.
pub fn pullback_covector(
    y: Vector6,
    amplitude: f64,
    covector_q: Vector6,
) -> Result<Vector6, ChartError> {
    validate_vector(&covector_q)?;
    let j = jacobian(y, amplitude)?;
    let mut result = [0.0; DIMENSION];
    for column in 0..DIMENSION {
        for row in 0..DIMENSION {
            result[column] += j[row][column] * covector_q[row];
        }
    }
    validate_vector(&result).map_err(|_| ChartError::NonFiniteResult)?;
    Ok(result)
}

/// Pull back a constraint Jacobian: J_h,y = J_h,q J.
pub fn pullback_constraint_jacobian(
    y: Vector6,
    amplitude: f64,
    jacobian_h_q: Matrix6,
) -> Result<Matrix6, ChartError> {
    if !finite_matrix(&jacobian_h_q) {
        return Err(ChartError::NonFiniteMatrix);
    }
    let j = jacobian(y, amplitude)?;
    let mut result = [[0.0; DIMENSION]; DIMENSION];
    for row in 0..DIMENSION {
        for column in 0..DIMENSION {
            for inner in 0..DIMENSION {
                result[row][column] += jacobian_h_q[row][inner] * j[inner][column];
            }
        }
    }
    if !finite_matrix(&result) {
        return Err(ChartError::NonFiniteResult);
    }
    Ok(result)
}

pub fn dot(left: Vector6, right: Vector6) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

pub fn identity6() -> Matrix6 {
    let mut result = [[0.0; DIMENSION]; DIMENSION];
    for index in 0..DIMENSION {
        result[index][index] = 1.0;
    }
    result
}

/// Exact spectral condition number of the only nontrivial 2x2 shear block.
pub fn chart_condition_number(y: Vector6, amplitude: f64) -> Result<f64, ChartError> {
    let j = jacobian(y, amplitude)?;
    let k = j[0][1].abs();
    let trace = k * k + 2.0;
    let discriminant = (trace * trace - 4.0).max(0.0).sqrt();
    let largest_eigenvalue = 0.5 * (trace + discriminant);
    if !largest_eigenvalue.is_finite() {
        return Err(ChartError::NonFiniteResult);
    }
    // det(J) = 1, so the product of squared singular values is one.
    Ok(largest_eigenvalue)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "left={left:.17e}, right={right:.17e}, tolerance={tolerance:.3e}"
        );
    }

    fn sample_state() -> (Vector6, Vector6, Vector6) {
        (
            [0.3, -0.7, 0.2, 0.5, -0.1, 0.9],
            [0.4, 1.3, -0.2, 0.1, 0.7, -0.3],
            [-0.5, 0.8, 0.3, -0.4, 0.2, 0.6],
        )
    }

    fn transpose(matrix: Matrix6) -> Matrix6 {
        let mut result = [[0.0; DIMENSION]; DIMENSION];
        for row in 0..DIMENSION {
            for column in 0..DIMENSION {
                result[row][column] = matrix[column][row];
            }
        }
        result
    }

    fn mat_vec(matrix: Matrix6, vector: Vector6) -> Vector6 {
        let mut result = [0.0; DIMENSION];
        for row in 0..DIMENSION {
            for column in 0..DIMENSION {
                result[row] += matrix[row][column] * vector[column];
            }
        }
        result
    }

    fn mat_mul(left: Matrix6, right: Matrix6) -> Matrix6 {
        let mut result = [[0.0; DIMENSION]; DIMENSION];
        for row in 0..DIMENSION {
            for column in 0..DIMENSION {
                for inner in 0..DIMENSION {
                    result[row][column] += left[row][inner] * right[inner][column];
                }
            }
        }
        result
    }

    fn quadratic(matrix: Matrix6, vector: Vector6) -> f64 {
        dot(vector, mat_vec(matrix, vector))
    }

    #[test]
    fn chart_round_trip_is_exact_to_floating_precision() {
        let (y, _, _) = sample_state();
        for amplitude in [-2.0, -0.5, 0.0, 0.75, 3.0] {
            let q = map_y_to_q(y, amplitude).unwrap();
            let recovered = map_q_to_y(q, amplitude).unwrap();
            for index in 0..DIMENSION {
                close(recovered[index], y[index], 4.0e-16);
            }
        }
    }

    #[test]
    fn analytic_jacobian_matches_central_finite_difference() {
        let (y, _, _) = sample_state();
        let amplitude = 1.4;
        let h = 1.0e-6;
        let analytic = jacobian(y, amplitude).unwrap();
        for column in 0..DIMENSION {
            let mut plus = y;
            let mut minus = y;
            plus[column] += h;
            minus[column] -= h;
            let q_plus = map_y_to_q(plus, amplitude).unwrap();
            let q_minus = map_y_to_q(minus, amplitude).unwrap();
            for row in 0..DIMENSION {
                close(analytic[row][column], (q_plus[row] - q_minus[row]) / (2.0 * h), 2.0e-10);
            }
        }
    }

    #[test]
    fn acceleration_chain_rule_matches_second_difference_of_chart_path() {
        let (y, ydot, yddot) = sample_state();
        let amplitude = 1.4;
        let h = 2.0e-4;
        let analytic = transform_acceleration(y, ydot, yddot, amplitude).unwrap();
        let mut y_plus = [0.0; DIMENSION];
        let mut y_minus = [0.0; DIMENSION];
        for index in 0..DIMENSION {
            y_plus[index] = y[index] + h * ydot[index] + 0.5 * h * h * yddot[index];
            y_minus[index] = y[index] - h * ydot[index] + 0.5 * h * h * yddot[index];
        }
        let q0 = map_y_to_q(y, amplitude).unwrap();
        let q_plus = map_y_to_q(y_plus, amplitude).unwrap();
        let q_minus = map_y_to_q(y_minus, amplitude).unwrap();
        for index in 0..DIMENSION {
            let second_difference = (q_plus[index] - 2.0 * q0[index] + q_minus[index]) / (h * h);
            close(analytic[index], second_difference, 2.0e-6);
        }
    }

    #[test]
    fn omitted_hessian_term_is_detected_by_negative_control() {
        let (y, ydot, yddot) = sample_state();
        let amplitude = 1.4;
        let correct = transform_acceleration(y, ydot, yddot, amplitude).unwrap();
        let j = jacobian(y, amplitude).unwrap();
        let incorrect = mat_vec(j, yddot);
        assert!((correct[0] - incorrect[0]).abs() > 0.1);
        for index in 1..DIMENSION {
            close(correct[index], incorrect[index], 1.0e-14);
        }
    }

    #[test]
    fn pulled_back_metric_preserves_symmetry_and_energy_pairing() {
        let (y, ydot, _) = sample_state();
        let metric_q = identity6();
        let metric_y = pullback_metric(y, 1.4, metric_q).unwrap();
        let j = jacobian(y, 1.4).unwrap();
        let j_t = transpose(j);
        assert_eq!(metric_y, mat_mul(mat_mul(j_t, metric_q), j));
        for row in 0..DIMENSION {
            for column in 0..DIMENSION {
                close(metric_y[row][column], metric_y[column][row], 1.0e-14);
            }
        }
        assert!(quadratic(metric_y, ydot) > 0.0);
        close(quadratic(metric_y, ydot), quadratic(metric_q, mat_vec(j, ydot)), 1.0e-12);
    }

    #[test]
    fn force_and_momentum_covectors_preserve_the_canonical_one_form() {
        let (y, ydot, _) = sample_state();
        let amplitude = 1.4;
        let force_q = [0.2, -1.1, 0.4, 0.3, -0.7, 0.5];
        let momentum_q = [-0.3, 0.9, 0.1, -0.2, 0.8, 0.6];
        let qdot = mat_vec(jacobian(y, amplitude).unwrap(), ydot);
        let force_y = pullback_covector(y, amplitude, force_q).unwrap();
        let momentum_y = pullback_covector(y, amplitude, momentum_q).unwrap();
        close(dot(force_q, qdot), dot(force_y, ydot), 1.0e-13);
        close(dot(momentum_q, qdot), dot(momentum_y, ydot), 1.0e-13);
    }

    #[test]
    fn constraint_jacobian_chain_rule_matches_finite_difference() {
        let (y, _, _) = sample_state();
        let amplitude = 0.9;
        let mut jacobian_q = [[0.0; DIMENSION]; DIMENSION];
        for row in 0..DIMENSION {
            for column in 0..DIMENSION {
                jacobian_q[row][column] = ((row + 1) * (column + 2)) as f64 / 13.0;
            }
        }
        let expected = pullback_constraint_jacobian(y, amplitude, jacobian_q).unwrap();
        let h = 1.0e-6;
        for column in 0..DIMENSION {
            let mut plus = y;
            let mut minus = y;
            plus[column] += h;
            minus[column] -= h;
            let q_plus = map_y_to_q(plus, amplitude).unwrap();
            let q_minus = map_y_to_q(minus, amplitude).unwrap();
            for row in 0..DIMENSION {
                let h_plus = dot(jacobian_q[row], q_plus);
                let h_minus = dot(jacobian_q[row], q_minus);
                close(expected[row][column], (h_plus - h_minus) / (2.0 * h), 2.0e-10);
            }
        }
    }

    #[test]
    fn chart_condition_number_is_finite_and_at_least_one() {
        let (y, _, _) = sample_state();
        for amplitude in [0.0, 0.25, 1.0, 5.0, 20.0] {
            let condition = chart_condition_number(y, amplitude).unwrap();
            assert!(condition.is_finite());
            assert!(condition >= 1.0);
        }
    }

    #[test]
    fn non_finite_inputs_fail_closed() {
        let (mut y, ydot, yddot) = sample_state();
        y[2] = f64::NAN;
        assert_eq!(map_y_to_q(y, 0.5), Err(ChartError::NonFiniteVector));
        let (y, _, _) = sample_state();
        assert_eq!(jacobian(y, f64::INFINITY), Err(ChartError::NonFiniteAmplitude));
        let mut metric = identity6();
        metric[0][0] = f64::NAN;
        assert_eq!(pullback_metric(y, 0.5, metric), Err(ChartError::NonFiniteMatrix));
        let mut bad_velocity = ydot;
        bad_velocity[1] = f64::INFINITY;
        assert_eq!(
            hessian_velocity_contraction(y, bad_velocity, 0.5),
            Err(ChartError::NonFiniteVector)
        );
        assert!(transform_acceleration(y, ydot, yddot, 0.5).is_ok());
    }

    #[test]
    fn linear_limit_reduces_to_identity_coordinate_map() {
        let (y, ydot, yddot) = sample_state();
        assert_eq!(map_y_to_q(y, 0.0).unwrap(), y);
        assert_eq!(jacobian(y, 0.0).unwrap(), identity6());
        assert_eq!(hessian_velocity_contraction(y, ydot, 0.0).unwrap(), [0.0; DIMENSION]);
        assert_eq!(transform_acceleration(y, ydot, yddot, 0.0).unwrap(), yddot);
    }
}
