// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Independent dense-coordinate covariance reference for one constrained 3D dumbbell step.
//!
//! The reference solves the same midpoint constrained mechanics equations in:
//! 1. canonical Cartesian coordinates with a diagonal mass matrix; and
//! 2. transformed generalized coordinates with the dense congruence-transformed mass metric.
//!
//! The transformed path does not reconstruct Cartesian coordinates to run the
//! canonical solver. It evaluates the dense-coordinate equations directly.

#![allow(clippy::needless_range_loop)]

use std::fmt;

pub type Vector3 = [f64; 3];
pub type Vector6 = [f64; 6];
pub type Matrix3 = [[f64; 3]; 3];
pub type Matrix6 = [[f64; 6]; 6];
pub type ConstraintMap = [[f64; 6]; 3];

#[derive(Debug, Clone, PartialEq)]
pub enum CovarianceError {
    NonFiniteInput(&'static str),
    InvalidTimeStep,
    SingularMatrix,
    NonPositiveMassMetric,
    MassMetricNotSymmetric,
    InvalidProfile,
    NewtonDerivativeDegenerate,
    NewtonLineSearchFailed,
    NewtonDidNotConverge,
}

impl fmt::Display for CovarianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for CovarianceError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepProfileV1 {
    pub maximum_newton_iterations: usize,
    pub maximum_backtracks: usize,
    pub normalized_constraint_tolerance: f64,
    pub relative_pivot_tolerance: f64,
    pub metric_symmetry_tolerance: f64,
}

impl Default for StepProfileV1 {
    fn default() -> Self {
        Self {
            maximum_newton_iterations: 32,
            maximum_backtracks: 24,
            normalized_constraint_tolerance: 1.0e-12,
            relative_pivot_tolerance: 64.0 * f64::EPSILON,
            metric_symmetry_tolerance: 1.0e-12,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepDiagnosticsV1 {
    pub multiplier: f64,
    pub midpoint_constraint_residual: f64,
    pub normalized_midpoint_constraint_residual: f64,
    pub position_constraint_change: f64,
    pub discrete_constraint_identity_residual: f64,
    pub external_work: f64,
    pub kinetic_energy_change: f64,
    pub energy_work_defect: f64,
    pub mass_metric_symmetry_defect: f64,
    pub newton_iterations: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepResultV1 {
    pub position_next: Vector6,
    pub velocity_next: Vector6,
    pub midpoint_velocity: Vector6,
    pub midpoint_relative_position: Vector3,
    pub diagnostics: StepDiagnosticsV1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicalStateV1 {
    pub position: Vector6,
    pub velocity: Vector6,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DumbbellMassesV1 {
    pub first: f64,
    pub second: f64,
}

pub fn identity6() -> Matrix6 {
    let mut result = [[0.0; 6]; 6];
    for index in 0..6 {
        result[index][index] = 1.0;
    }
    result
}

pub fn identity3() -> Matrix3 {
    let mut result = [[0.0; 3]; 3];
    for index in 0..3 {
        result[index][index] = 1.0;
    }
    result
}

pub fn transpose6(matrix: &Matrix6) -> Matrix6 {
    let mut result = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            result[row][column] = matrix[column][row];
        }
    }
    result
}

pub fn multiply6(left: &Matrix6, right: &Matrix6) -> Matrix6 {
    let mut result = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            for inner in 0..6 {
                result[row][column] += left[row][inner] * right[inner][column];
            }
        }
    }
    result
}

pub fn multiply_constraint_by_basis(
    map: &ConstraintMap,
    basis: &Matrix6,
) -> ConstraintMap {
    let mut result = [[0.0; 6]; 3];
    for row in 0..3 {
        for column in 0..6 {
            for inner in 0..6 {
                result[row][column] += map[row][inner] * basis[inner][column];
            }
        }
    }
    result
}

pub fn transpose6_times_vector(matrix: &Matrix6, vector: &Vector6) -> Vector6 {
    let mut result = [0.0; 6];
    for row in 0..6 {
        for column in 0..6 {
            result[row] += matrix[column][row] * vector[column];
        }
    }
    result
}

pub fn mass_metric(masses: DumbbellMassesV1) -> Matrix6 {
    let mut result = [[0.0; 6]; 6];
    for axis in 0..3 {
        result[axis][axis] = masses.first;
        result[axis + 3][axis + 3] = masses.second;
    }
    result
}

pub fn transformed_mass_metric(mass: &Matrix6, basis: &Matrix6) -> Matrix6 {
    let basis_t = transpose6(basis);
    multiply6(&basis_t, &multiply6(mass, basis))
}

/// The 3×6 map returns q2 - q1.
pub fn relative_position_map() -> ConstraintMap {
    [
        [-1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        [0.0, -1.0, 0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, -1.0, 0.0, 0.0, 1.0],
    ]
}

pub fn apply_constraint_map(map: &ConstraintMap, vector: &Vector6) -> Vector3 {
    let mut result = [0.0; 3];
    for row in 0..3 {
        for column in 0..6 {
            result[row] += map[row][column] * vector[column];
        }
    }
    result
}

pub fn apply6(matrix: &Matrix6, vector: &Vector6) -> Vector6 {
    let mut result = [0.0; 6];
    for row in 0..6 {
        for column in 0..6 {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

pub fn add6(left: &Vector6, right: &Vector6) -> Vector6 {
    let mut result = [0.0; 6];
    for index in 0..6 {
        result[index] = left[index] + right[index];
    }
    result
}

pub fn subtract6(left: &Vector6, right: &Vector6) -> Vector6 {
    let mut result = [0.0; 6];
    for index in 0..6 {
        result[index] = left[index] - right[index];
    }
    result
}

pub fn scale6(vector: &Vector6, scale: f64) -> Vector6 {
    let mut result = [0.0; 6];
    for index in 0..6 {
        result[index] = vector[index] * scale;
    }
    result
}

pub fn dot6(left: &Vector6, right: &Vector6) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum()
}

pub fn dot3(left: &Vector3, right: &Vector3) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum()
}

pub fn norm6(vector: &Vector6) -> f64 {
    vector.iter().fold(0.0_f64, |norm, value| norm.hypot(*value))
}

pub fn norm3(vector: &Vector3) -> f64 {
    vector.iter().fold(0.0_f64, |norm, value| norm.hypot(*value))
}

pub fn norm_matrix6(matrix: &Matrix6) -> f64 {
    matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

pub fn cross(left: &Vector3, right: &Vector3) -> Vector3 {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn maximum_absolute_matrix6(matrix: &Matrix6) -> f64 {
    matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
}

/// Partial-pivoted Gaussian solve. The pivot floor is relative to the matrix scale.
pub fn solve6(
    matrix: &Matrix6,
    right_hand_side: &Vector6,
    relative_pivot_tolerance: f64,
) -> Result<Vector6, CovarianceError> {
    if matrix.iter().flatten().any(|value| !value.is_finite())
        || right_hand_side.iter().any(|value| !value.is_finite())
    {
        return Err(CovarianceError::NonFiniteInput("6x6 linear system"));
    }
    let mut a = *matrix;
    let mut b = *right_hand_side;
    let scale = maximum_absolute_matrix6(&a);
    if !scale.is_finite() || scale == 0.0 {
        return Err(CovarianceError::SingularMatrix);
    }

    for pivot_column in 0..6 {
        let mut pivot_row = pivot_column;
        let mut pivot_magnitude = a[pivot_column][pivot_column].abs();
        for candidate in (pivot_column + 1)..6 {
            let magnitude = a[candidate][pivot_column].abs();
            if magnitude > pivot_magnitude {
                pivot_magnitude = magnitude;
                pivot_row = candidate;
            }
        }
        if !pivot_magnitude.is_finite()
            || pivot_magnitude <= relative_pivot_tolerance * scale
        {
            return Err(CovarianceError::SingularMatrix);
        }
        if pivot_row != pivot_column {
            a.swap(pivot_row, pivot_column);
            b.swap(pivot_row, pivot_column);
        }

        let pivot = a[pivot_column][pivot_column];
        for row in (pivot_column + 1)..6 {
            let factor = a[row][pivot_column] / pivot;
            a[row][pivot_column] = 0.0;
            for column in (pivot_column + 1)..6 {
                a[row][column] -= factor * a[pivot_column][column];
            }
            b[row] -= factor * b[pivot_column];
        }
    }

    let mut solution = [0.0; 6];
    for row in (0..6).rev() {
        let mut residual = b[row];
        for column in (row + 1)..6 {
            residual -= a[row][column] * solution[column];
        }
        let pivot = a[row][row];
        if !pivot.is_finite() || pivot.abs() <= relative_pivot_tolerance * scale {
            return Err(CovarianceError::SingularMatrix);
        }
        solution[row] = residual / pivot;
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err(CovarianceError::NonFiniteInput("6x6 solution"));
    }
    Ok(solution)
}

fn solve3(
    matrix: &Matrix3,
    right_hand_side: &Vector3,
    relative_pivot_tolerance: f64,
) -> Result<Vector3, CovarianceError> {
    if matrix.iter().flatten().any(|value| !value.is_finite())
        || right_hand_side.iter().any(|value| !value.is_finite())
    {
        return Err(CovarianceError::NonFiniteInput("3x3 linear system"));
    }
    let mut a = *matrix;
    let mut b = *right_hand_side;
    let scale = a
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if !scale.is_finite() || scale == 0.0 {
        return Err(CovarianceError::SingularMatrix);
    }

    for pivot_column in 0..3 {
        let mut pivot_row = pivot_column;
        let mut pivot_magnitude = a[pivot_column][pivot_column].abs();
        for candidate in (pivot_column + 1)..3 {
            let magnitude = a[candidate][pivot_column].abs();
            if magnitude > pivot_magnitude {
                pivot_magnitude = magnitude;
                pivot_row = candidate;
            }
        }
        if !pivot_magnitude.is_finite()
            || pivot_magnitude <= relative_pivot_tolerance * scale
        {
            return Err(CovarianceError::SingularMatrix);
        }
        if pivot_row != pivot_column {
            a.swap(pivot_row, pivot_column);
            b.swap(pivot_row, pivot_column);
        }

        let pivot = a[pivot_column][pivot_column];
        for row in (pivot_column + 1)..3 {
            let factor = a[row][pivot_column] / pivot;
            a[row][pivot_column] = 0.0;
            for column in (pivot_column + 1)..3 {
                a[row][column] -= factor * a[pivot_column][column];
            }
            b[row] -= factor * b[pivot_column];
        }
    }

    let mut solution = [0.0; 3];
    for row in (0..3).rev() {
        let mut residual = b[row];
        for column in (row + 1)..3 {
            residual -= a[row][column] * solution[column];
        }
        let pivot = a[row][row];
        if !pivot.is_finite() || pivot.abs() <= relative_pivot_tolerance * scale {
            return Err(CovarianceError::SingularMatrix);
        }
        solution[row] = residual / pivot;
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err(CovarianceError::NonFiniteInput("3x3 solution"));
    }
    Ok(solution)
}

pub fn inverse6(
    matrix: &Matrix6,
    relative_pivot_tolerance: f64,
) -> Result<Matrix6, CovarianceError> {
    let mut inverse = [[0.0; 6]; 6];
    for column in 0..6 {
        let mut basis = [0.0; 6];
        basis[column] = 1.0;
        let solution = solve6(matrix, &basis, relative_pivot_tolerance)?;
        for row in 0..6 {
            inverse[row][column] = solution[row];
        }
    }
    Ok(inverse)
}

/// Dense unimodular basis changes generated as L*U with unit diagonals.
/// Their exact determinant is one, while both L and U have nonzero off-diagonal entries.
pub fn frozen_dense_basis(family: usize) -> Matrix6 {
    let mut lower = identity6();
    let mut upper = identity6();
    for row in 0..6 {
        for column in 0..row {
            lower[row][column] = if (row - column + family) % 3 != 0 {
                1.0
            } else {
                -1.0
            };
        }
        for column in (row + 1)..6 {
            upper[row][column] = if (column - row + family) % 3 != 0 {
                1.0
            } else {
                -1.0
            };
        }
    }
    multiply6(&lower, &upper)
}

pub fn canonical_state() -> PhysicalStateV1 {
    let q1 = [0.3, -0.2, 0.7];
    let relative_position = [0.8, 0.8, 0.4];
    let q2 = [
        q1[0] + relative_position[0],
        q1[1] + relative_position[1],
        q1[2] + relative_position[2],
    ];

    let angular_velocity = [0.2, -0.3, 0.5];
    let relative_velocity = cross(&angular_velocity, &relative_position);
    let center_velocity = [0.15, -0.05, 0.1];
    let masses = DumbbellMassesV1 {
        first: 1.3,
        second: 2.1,
    };
    let total_mass = masses.first + masses.second;
    let first_velocity = [
        center_velocity[0] - masses.second / total_mass * relative_velocity[0],
        center_velocity[1] - masses.second / total_mass * relative_velocity[1],
        center_velocity[2] - masses.second / total_mass * relative_velocity[2],
    ];
    let second_velocity = [
        center_velocity[0] + masses.first / total_mass * relative_velocity[0],
        center_velocity[1] + masses.first / total_mass * relative_velocity[1],
        center_velocity[2] + masses.first / total_mass * relative_velocity[2],
    ];

    PhysicalStateV1 {
        position: [
            q1[0], q1[1], q1[2], q2[0], q2[1], q2[2],
        ],
        velocity: [
            first_velocity[0],
            first_velocity[1],
            first_velocity[2],
            second_velocity[0],
            second_velocity[1],
            second_velocity[2],
        ],
    }
}

pub fn common_acceleration_force(masses: DumbbellMassesV1) -> Vector6 {
    let acceleration = [0.18, -0.07, 0.11];
    [
        masses.first * acceleration[0],
        masses.first * acceleration[1],
        masses.first * acceleration[2],
        masses.second * acceleration[0],
        masses.second * acceleration[1],
        masses.second * acceleration[2],
    ]
}

/// A force applied only to the first mass. Unlike a common acceleration,
/// this excites the constraint multiplier and checks the coupled solve.
pub fn asymmetric_force() -> Vector6 {
    [0.18, -0.07, 0.11, 0.0, 0.0, 0.0]
}

fn multiply3_vector(matrix: &Matrix3, vector: &Vector3) -> Vector3 {
    let mut result = [0.0; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

fn add3(left: &Vector3, right: &Vector3) -> Vector3 {
    [
        left[0] + right[0],
        left[1] + right[1],
        left[2] + right[2],
    ]
}

fn scale3(vector: &Vector3, scale: f64) -> Vector3 {
    [vector[0] * scale, vector[1] * scale, vector[2] * scale]
}

fn constraint_matrix_times(
    map: &ConstraintMap,
    vector: &Vector6,
) -> Vector3 {
    apply_constraint_map(map, vector)
}

fn constraint_gram(
    map: &ConstraintMap,
    inverse_mass_map: &[[f64; 3]; 6],
) -> Matrix3 {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            for inner in 0..6 {
                result[row][column] += map[row][inner] * inverse_mass_map[inner][column];
            }
        }
    }
    result
}

#[derive(Clone, Copy)]
struct LambdaEvaluation {
    midpoint_velocity: Vector6,
    midpoint_relative_position: Vector3,
    residual: f64,
    residual_scale: f64,
    derivative: f64,
}

#[allow(clippy::too_many_arguments)]
fn evaluate_multiplier(
    lambda: f64,
    h: f64,
    initial_relative_position: &Vector3,
    free_midpoint_velocity: &Vector6,
    relative_position_after_free_step: &Vector3,
    inverse_mass_constraint_transpose: &[[f64; 3]; 6],
    delassus: &Matrix3,
    profile: StepProfileV1,
) -> Result<LambdaEvaluation, CovarianceError> {
    let coefficient = h * h / 4.0;
    let mut system = identity3();
    for row in 0..3 {
        for column in 0..3 {
            system[row][column] -= coefficient * lambda * delassus[row][column];
        }
    }
    let midpoint_relative_position = solve3(
        &system,
        relative_position_after_free_step,
        profile.relative_pivot_tolerance,
    )?;

    let mut inverse_mass_times_constraint = [0.0; 6];
    for row in 0..6 {
        for axis in 0..3 {
            inverse_mass_times_constraint[row] +=
                inverse_mass_constraint_transpose[row][axis]
                    * midpoint_relative_position[axis];
        }
    }
    let midpoint_velocity = add6(
        free_midpoint_velocity,
        &scale6(&inverse_mass_times_constraint, h * lambda / 2.0),
    );
    // The constraint map is encoded through the relative-position update:
    // g(lambda) = r_bar dot A u. The caller reconstructs A*u separately below.
    // This helper only forms the derivative workspace for r_bar(lambda).
    let delassus_times_relative = multiply3_vector(delassus, &midpoint_relative_position);
    let midpoint_relative_prime = scale3(
        &solve3(&system, &delassus_times_relative, profile.relative_pivot_tolerance)?,
        coefficient,
    );

    // The caller supplies the relative velocity map through the fact that
    // the relative coordinate difference is r_bar = r0 + h/2 A u.
    // A*u is recovered as 2*(r_bar-r0)/h, avoiding a second map argument.
    let mapped_midpoint_velocity = scale3(
        &[
            midpoint_relative_position[0] - initial_relative_position[0],
            midpoint_relative_position[1] - initial_relative_position[1],
            midpoint_relative_position[2] - initial_relative_position[2],
        ],
        2.0 / h,
    );
    let mapped_midpoint_velocity_prime = scale3(&midpoint_relative_prime, 2.0 / h);
    let residual = dot3(&midpoint_relative_position, &mapped_midpoint_velocity);
    let derivative = dot3(&midpoint_relative_prime, &mapped_midpoint_velocity)
        + dot3(&midpoint_relative_position, &mapped_midpoint_velocity_prime);
    let residual_scale = (norm3(initial_relative_position)
        * norm3(&scale3(&[
            relative_position_after_free_step[0] - initial_relative_position[0],
            relative_position_after_free_step[1] - initial_relative_position[1],
            relative_position_after_free_step[2] - initial_relative_position[2],
        ], 2.0 / h)))
        .max(norm3(initial_relative_position).powi(2) / h)
        .max(1.0e-300);

    Ok(LambdaEvaluation {
        midpoint_velocity,
        midpoint_relative_position,
        residual,
        residual_scale,
        derivative,
    })
}

/// Execute one midpoint constrained step in an arbitrary generalized-coordinate basis.
///
/// The mass matrix is the full covariant kinetic metric; generalized force and
/// constraint Jacobian are covectors/maps in the same declared coordinates.
pub fn constrained_dumbbell_step(
    position: &Vector6,
    velocity: &Vector6,
    mass_metric: &Matrix6,
    constraint_map: &ConstraintMap,
    generalized_force: &Vector6,
    time_step: f64,
    profile: StepProfileV1,
) -> Result<StepResultV1, CovarianceError> {
    if !time_step.is_finite() || time_step <= 0.0 {
        return Err(CovarianceError::InvalidTimeStep);
    }
    if !profile.normalized_constraint_tolerance.is_finite()
        || profile.normalized_constraint_tolerance <= 0.0
        || !profile.relative_pivot_tolerance.is_finite()
        || profile.relative_pivot_tolerance <= 0.0
        || !profile.metric_symmetry_tolerance.is_finite()
        || profile.metric_symmetry_tolerance <= 0.0
        || profile.maximum_newton_iterations == 0
        || profile.maximum_backtracks == 0
    {
        return Err(CovarianceError::InvalidProfile);
    }
    if position.iter().any(|value| !value.is_finite())
        || velocity.iter().any(|value| !value.is_finite())
        || generalized_force.iter().any(|value| !value.is_finite())
        || mass_metric.iter().flatten().any(|value| !value.is_finite())
        || constraint_map.iter().flatten().any(|value| !value.is_finite())
    {
        return Err(CovarianceError::NonFiniteInput("step arguments"));
    }

    // Admit the metric once. Preserve a scale-free defect for diagnostics, but
    // use the exactly symmetric effective metric in every subsequent operation.
    // This prevents tolerated serialization noise from producing a different
    // matrix in the solves and energy ledger.
    let (effective_mass_metric, mass_metric_symmetry_defect) =
        admit_mass_metric(mass_metric, profile)?;
    cholesky6(&effective_mass_metric, profile.relative_pivot_tolerance)?;

    let inverse_mass_force = solve6(
        &effective_mass_metric,
        generalized_force,
        profile.relative_pivot_tolerance,
    )?;
    let free_midpoint_velocity =
        add6(velocity, &scale6(&inverse_mass_force, time_step / 2.0));

    let mut inverse_mass_constraint_transpose = [[0.0; 3]; 6];
    for axis in 0..3 {
        let mut right_hand_side = [0.0; 6];
        for row in 0..6 {
            right_hand_side[row] = constraint_map[axis][row];
        }
        let solution = solve6(
            &effective_mass_metric,
            &right_hand_side,
            profile.relative_pivot_tolerance,
        )?;
        for row in 0..6 {
            inverse_mass_constraint_transpose[row][axis] = solution[row];
        }
    }
    let delassus =
        constraint_gram(constraint_map, &inverse_mass_constraint_transpose);
    let initial_relative_position = apply_constraint_map(constraint_map, position);
    let relative_position_after_free_step = add3(
        &initial_relative_position,
        &scale3(
            &constraint_matrix_times(constraint_map, &free_midpoint_velocity),
            time_step / 2.0,
        ),
    );

    let mut multiplier = 0.0;
    let mut final_evaluation = evaluate_multiplier(
        multiplier,
        time_step,
        &initial_relative_position,
        &free_midpoint_velocity,
        &relative_position_after_free_step,
        &inverse_mass_constraint_transpose,
        &delassus,
        profile,
    )?;
    let mut iterations = 0usize;
    let mut converged = false;

    for iteration in 0..profile.maximum_newton_iterations {
        iterations = iteration + 1;
        let normalized_residual =
            final_evaluation.residual.abs() / final_evaluation.residual_scale;
        if normalized_residual <= profile.normalized_constraint_tolerance {
            converged = true;
            break;
        }
        if !final_evaluation.derivative.is_finite()
            || final_evaluation.derivative.abs() <= 64.0 * f64::EPSILON
        {
            return Err(CovarianceError::NewtonDerivativeDegenerate);
        }

        let full_step = final_evaluation.residual / final_evaluation.derivative;
        let current_norm = normalized_residual;
        let mut step_scale = 1.0;
        let mut accepted = false;

        for _ in 0..profile.maximum_backtracks {
            let candidate_multiplier = multiplier - step_scale * full_step;
            if let Ok(candidate) = evaluate_multiplier(
                candidate_multiplier,
                time_step,
                &initial_relative_position,
                &free_midpoint_velocity,
                &relative_position_after_free_step,
                &inverse_mass_constraint_transpose,
                &delassus,
                profile,
            ) {
                let candidate_norm =
                    candidate.residual.abs() / candidate.residual_scale;
                if candidate_norm.is_finite() && candidate_norm < current_norm {
                    multiplier = candidate_multiplier;
                    final_evaluation = candidate;
                    accepted = true;
                    break;
                }
            }
            step_scale *= 0.5;
        }

        if !accepted {
            return Err(CovarianceError::NewtonLineSearchFailed);
        }
    }

    if !converged {
        let normalized_residual =
            final_evaluation.residual.abs() / final_evaluation.residual_scale;
        if normalized_residual <= profile.normalized_constraint_tolerance {
            converged = true;
        }
    }
    if !converged {
        return Err(CovarianceError::NewtonDidNotConverge);
    }

    let midpoint_velocity = final_evaluation.midpoint_velocity;
    let position_next = add6(position, &scale6(&midpoint_velocity, time_step));
    let velocity_next = subtract6(
        &scale6(&midpoint_velocity, 2.0),
        &scale6(velocity, 1.0),
    );

    let initial_relative = apply_constraint_map(constraint_map, position);
    let final_relative = apply_constraint_map(constraint_map, &position_next);
    let position_constraint_change =
        0.5 * (dot3(&final_relative, &final_relative) - dot3(&initial_relative, &initial_relative));
    let discrete_constraint_identity_residual =
        position_constraint_change - time_step * final_evaluation.residual;

    let initial_energy =
        0.5 * dot6(velocity, &apply6(&effective_mass_metric, velocity));
    let final_energy =
        0.5 * dot6(&velocity_next, &apply6(&effective_mass_metric, &velocity_next));
    let energy_change = final_energy - initial_energy;
    let external_work = time_step * dot6(generalized_force, &midpoint_velocity);
    let energy_work_defect = energy_change - external_work;

    Ok(StepResultV1 {
        position_next,
        velocity_next,
        midpoint_velocity,
        midpoint_relative_position: final_evaluation.midpoint_relative_position,
        diagnostics: StepDiagnosticsV1 {
            multiplier,
            midpoint_constraint_residual: final_evaluation.residual,
            normalized_midpoint_constraint_residual: final_evaluation.residual.abs()
                / final_evaluation.residual_scale,
            position_constraint_change,
            discrete_constraint_identity_residual,
            external_work,
            kinetic_energy_change: energy_change,
            energy_work_defect,
            mass_metric_symmetry_defect,
            newton_iterations: iterations,
        },
    })
}

fn admit_mass_metric(
    supplied: &Matrix6,
    profile: StepProfileV1,
) -> Result<(Matrix6, f64), CovarianceError> {
    let scale = maximum_absolute_matrix6(supplied);
    if !scale.is_finite() || scale == 0.0 {
        return Err(CovarianceError::NonPositiveMassMetric);
    }
    let mut effective = [[0.0; 6]; 6];
    let mut defect = 0.0_f64;
    for row in 0..6 {
        for column in 0..6 {
            let normalized = (supplied[row][column] - supplied[column][row]).abs() / scale;
            defect = defect.max(normalized);
            effective[row][column] =
                0.5 * supplied[row][column] + 0.5 * supplied[column][row];
        }
    }
    if defect > profile.metric_symmetry_tolerance {
        return Err(CovarianceError::MassMetricNotSymmetric);
    }
    Ok((effective, defect))
}

fn cholesky6(
    matrix: &Matrix6,
    relative_pivot_tolerance: f64,
) -> Result<Matrix6, CovarianceError> {
    let scale = maximum_absolute_matrix6(matrix);
    if !scale.is_finite() || scale == 0.0 {
        return Err(CovarianceError::NonPositiveMassMetric);
    }
    let mut lower = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..=row {
            let mut residual = matrix[row][column] / scale;
            for inner in 0..column {
                residual -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !residual.is_finite() || residual <= relative_pivot_tolerance {
                    return Err(CovarianceError::NonPositiveMassMetric);
                }
                lower[row][column] = residual.sqrt();
            } else {
                lower[row][column] = residual / lower[column][column];
            }
        }
    }
    Ok(lower)
}

pub fn relative_matrix_defect(left: &Matrix6, right: &Matrix6) -> f64 {
    let mut residual = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            residual[row][column] = left[row][column] - right[row][column];
        }
    }
    norm_matrix6(&residual) / norm_matrix6(right).max(1.0e-300)
}

pub fn transform_state_to_basis(
    state: PhysicalStateV1,
    basis_inverse: &Matrix6,
) -> PhysicalStateV1 {
    PhysicalStateV1 {
        position: apply6(basis_inverse, &state.position),
        velocity: apply6(basis_inverse, &state.velocity),
    }
}

pub fn lift_state_from_basis(
    state: PhysicalStateV1,
    basis: &Matrix6,
) -> PhysicalStateV1 {
    PhysicalStateV1 {
        position: apply6(basis, &state.position),
        velocity: apply6(basis, &state.velocity),
    }
}

pub fn transform_force_covector(basis: &Matrix6, force: &Vector6) -> Vector6 {
    transpose6_times_vector(basis, force)
}

pub fn linear_momentum(
    velocity: &Vector6,
    masses: DumbbellMassesV1,
) -> Vector3 {
    [
        masses.first * velocity[0] + masses.second * velocity[3],
        masses.first * velocity[1] + masses.second * velocity[4],
        masses.first * velocity[2] + masses.second * velocity[5],
    ]
}

pub fn angular_momentum(
    position: &Vector6,
    velocity: &Vector6,
    masses: DumbbellMassesV1,
) -> Vector3 {
    let first_position = [position[0], position[1], position[2]];
    let second_position = [position[3], position[4], position[5]];
    let first_velocity = [
        masses.first * velocity[0],
        masses.first * velocity[1],
        masses.first * velocity[2],
    ];
    let second_velocity = [
        masses.second * velocity[3],
        masses.second * velocity[4],
        masses.second * velocity[5],
    ];
    add3(
        &cross(&first_position, &first_velocity),
        &cross(&second_position, &second_velocity),
    )
}

pub fn cross_matrix(axis: &Vector3) -> Matrix3 {
    [
        [0.0, -axis[2], axis[1]],
        [axis[2], 0.0, -axis[0]],
        [-axis[1], axis[0], 0.0],
    ]
}

pub fn rotation3(axis: Vector3, angle: f64) -> Matrix3 {
    let magnitude = norm3(&axis);
    if magnitude == 0.0 {
        return identity3();
    }
    let unit = scale3(&axis, 1.0 / magnitude);
    let skew = cross_matrix(&unit);
    let mut result = identity3();
    let cosine = angle.cos();
    let sine = angle.sin();
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = (if row == column { cosine } else { 0.0 })
                + (1.0 - cosine) * unit[row] * unit[column]
                + sine * skew[row][column];
        }
    }
    result
}

pub fn block_rotation3(rotation: &Matrix3) -> Matrix6 {
    let mut result = [[0.0; 6]; 6];
    for block in 0..2 {
        for row in 0..3 {
            for column in 0..3 {
                result[block * 3 + row][block * 3 + column] = rotation[row][column];
            }
        }
    }
    result
}

pub fn metric_skew_relative_defect(
    metric: &Matrix6,
    generator: &Matrix6,
) -> f64 {
    let metric_generator = multiply6(metric, generator);
    let generator_transpose_metric = multiply6(&transpose6(generator), metric);
    let mut defect = [[0.0; 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            defect[row][column] =
                metric_generator[row][column] + generator_transpose_metric[row][column];
        }
    }
    norm_matrix6(&defect)
        / (2.0 * norm_matrix6(&metric_generator)).max(1.0e-300)
}

pub fn metric_isometry_relative_defect(
    metric: &Matrix6,
    transformation: &Matrix6,
) -> f64 {
    let transformed = multiply6(
        &transpose6(transformation),
        &multiply6(metric, transformation),
    );
    relative_matrix_defect(&transformed, metric)
}

pub fn transform_generator(
    basis: &Matrix6,
    basis_inverse: &Matrix6,
    generator: &Matrix6,
) -> Matrix6 {
    multiply6(&basis_inverse, &multiply6(generator, basis))
}

pub fn transform_finite_action(
    basis: &Matrix6,
    basis_inverse: &Matrix6,
    action: &Matrix6,
) -> Matrix6 {
    transform_generator(basis, basis_inverse, action)
}

pub fn apply_rotation_to_force_covector(
    action_inverse: &Matrix6,
    force: &Vector6,
) -> Vector6 {
    transpose6_times_vector(action_inverse, force)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close6(left: &Vector6, right: &Vector6, tolerance: f64) {
        let difference = subtract6(left, right);
        assert!(
            norm6(&difference) <= tolerance,
            "vector difference {} exceeds tolerance {}",
            norm6(&difference),
            tolerance
        );
    }

    fn run_canonical(
        state: PhysicalStateV1,
        force: Vector6,
        h: f64,
    ) -> StepResultV1 {
        let masses = DumbbellMassesV1 {
            first: 1.3,
            second: 2.1,
        };
        constrained_dumbbell_step(
            &state.position,
            &state.velocity,
            &mass_metric(masses),
            &relative_position_map(),
            &force,
            h,
            StepProfileV1::default(),
        )
        .expect("canonical dumbbell step must converge")
    }

    fn run_in_dense_basis(
        state: PhysicalStateV1,
        force: Vector6,
        basis_family: usize,
        h: f64,
    ) -> (StepResultV1, Matrix6, Matrix6) {
        let masses = DumbbellMassesV1 {
            first: 1.3,
            second: 2.1,
        };
        let canonical_mass = mass_metric(masses);
        let canonical_map = relative_position_map();
        let basis = frozen_dense_basis(basis_family);
        let basis_inverse = inverse6(
            &basis,
            StepProfileV1::default().relative_pivot_tolerance,
        )
        .expect("frozen basis must be invertible");
        let transformed_state =
            transform_state_to_basis(state, &basis_inverse);
        let transformed_force = transform_force_covector(&basis, &force);
        let transformed_mass =
            transformed_mass_metric(&canonical_mass, &basis);
        let transformed_map =
            multiply_constraint_by_basis(&canonical_map, &basis);
        let result = constrained_dumbbell_step(
            &transformed_state.position,
            &transformed_state.velocity,
            &transformed_mass,
            &transformed_map,
            &transformed_force,
            h,
            StepProfileV1::default(),
        )
        .expect("dense-coordinate dumbbell step must converge");
        (result, basis, basis_inverse)
    }

    #[test]
    fn dense_unimodular_step_is_representation_covariant() {
        let state = canonical_state();
        let force = asymmetric_force();
        let canonical = run_canonical(state, force, 0.02);

        for family in [0, 2] {
            let (transformed, basis, _) =
                run_in_dense_basis(state, force, family, 0.02);
            let lifted = lift_state_from_basis(
                PhysicalStateV1 {
                    position: transformed.position_next,
                    velocity: transformed.velocity_next,
                },
                &basis,
            );
            close6(&lifted.position, &canonical.position_next, 5.0e-11);
            close6(&lifted.velocity, &canonical.velocity_next, 5.0e-11);
            assert!(
                canonical.diagnostics.multiplier.abs() > 1.0e-2,
                "the fixture must exercise a nonzero constraint multiplier"
            );
            assert!(
                (transformed.diagnostics.multiplier
                    - canonical.diagnostics.multiplier)
                    .abs()
                    < 5.0e-10
            );
            assert!(
                (transformed.diagnostics.external_work
                    - canonical.diagnostics.external_work)
                    .abs()
                    < 5.0e-11
            );
            assert!(
                transformed.diagnostics.normalized_midpoint_constraint_residual
                    < 1.0e-11
            );
            assert!(
                transformed.diagnostics.energy_work_defect.abs() < 1.0e-10
            );

            let lifted_momentum = linear_momentum(
                &lifted.velocity,
                DumbbellMassesV1 {
                    first: 1.3,
                    second: 2.1,
                },
            );
            let canonical_momentum = linear_momentum(
                &canonical.velocity_next,
                DumbbellMassesV1 {
                    first: 1.3,
                    second: 2.1,
                },
            );
            assert!(norm3(&[
                lifted_momentum[0] - canonical_momentum[0],
                lifted_momentum[1] - canonical_momentum[1],
                lifted_momentum[2] - canonical_momentum[2],
            ]) < 5.0e-11);

            let lifted_angular = angular_momentum(
                &lifted.position,
                &lifted.velocity,
                DumbbellMassesV1 {
                    first: 1.3,
                    second: 2.1,
                },
            );
            let canonical_angular = angular_momentum(
                &canonical.position_next,
                &canonical.velocity_next,
                DumbbellMassesV1 {
                    first: 1.3,
                    second: 2.1,
                },
            );
            assert!(norm3(&[
                lifted_angular[0] - canonical_angular[0],
                lifted_angular[1] - canonical_angular[1],
                lifted_angular[2] - canonical_angular[2],
            ]) < 5.0e-11);
        }
    }

    #[test]
    fn tolerated_metric_noise_is_canonicalized_before_solve_and_energy() {
        let state = canonical_state();
        let masses = DumbbellMassesV1 {
            first: 1.3,
            second: 2.1,
        };
        let mut noisy_metric = mass_metric(masses);
        noisy_metric[0][1] = 2.0e-13;
        let result = constrained_dumbbell_step(
            &state.position,
            &state.velocity,
            &noisy_metric,
            &relative_position_map(),
            &asymmetric_force(),
            0.02,
            StepProfileV1::default(),
        )
        .unwrap();
        assert!(result.diagnostics.mass_metric_symmetry_defect > 0.0);
        assert!(result.diagnostics.mass_metric_symmetry_defect < 1.0e-12);
        assert!(result.diagnostics.energy_work_defect.abs() < 1.0e-10);

        noisy_metric[0][1] = 2.0e-5;
        assert!(matches!(
            constrained_dumbbell_step(
                &state.position,
                &state.velocity,
                &noisy_metric,
                &relative_position_map(),
                &asymmetric_force(),
                0.02,
                StepProfileV1::default(),
            ),
            Err(CovarianceError::MassMetricNotSymmetric)
        ));
    }

    #[test]
    fn wrong_metric_congruence_is_detected_by_independent_step_oracle() {
        let state = canonical_state();
        let masses = DumbbellMassesV1 {
            first: 1.3,
            second: 2.1,
        };
        let force = asymmetric_force();
        let canonical = run_canonical(state, force, 0.02);
        let basis = frozen_dense_basis(2);
        let basis_inverse = inverse6(
            &basis,
            StepProfileV1::default().relative_pivot_tolerance,
        )
        .unwrap();
        let transformed_state = transform_state_to_basis(state, &basis_inverse);
        let wrong_metric = multiply6(
            &basis_inverse,
            &multiply6(&mass_metric(masses), &transpose6(&basis_inverse)),
        );
        let transformed_map = multiply_constraint_by_basis(
            &relative_position_map(),
            &basis,
        );
        let transformed_force = transform_force_covector(&basis, &force);
        let wrong = constrained_dumbbell_step(
            &transformed_state.position,
            &transformed_state.velocity,
            &wrong_metric,
            &transformed_map,
            &transformed_force,
            0.02,
            StepProfileV1::default(),
        )
        .expect("wrong congruence control remains a finite SPD solve");
        let lifted = lift_state_from_basis(
            PhysicalStateV1 {
                position: wrong.position_next,
                velocity: wrong.velocity_next,
            },
            &basis,
        );
        assert!(
            norm6(&subtract6(&lifted.position, &canonical.position_next)) > 1.0e-4
        );
    }

    #[test]
    fn force_is_a_covector_and_work_is_representation_invariant() {
        let state = canonical_state();
        let masses = DumbbellMassesV1 {
            first: 1.3,
            second: 2.1,
        };
        let force = asymmetric_force();
        let canonical = run_canonical(state, force, 0.02);
        let (transformed, basis, _) =
            run_in_dense_basis(state, force, 2, 0.02);

        assert!(
            (transformed.diagnostics.external_work
                - canonical.diagnostics.external_work)
                .abs()
                < 5.0e-11
        );

        let wrong_force = apply6(
            &inverse6(&basis, StepProfileV1::default().relative_pivot_tolerance).unwrap(),
            &force,
        );
        let basis_inverse =
            inverse6(&basis, StepProfileV1::default().relative_pivot_tolerance).unwrap();
        let transformed_state =
            transform_state_to_basis(state, &basis_inverse);
        let transformed_mass =
            transformed_mass_metric(&mass_metric(masses), &basis);
        let transformed_map =
            multiply_constraint_by_basis(&relative_position_map(), &basis);
        let wrong = constrained_dumbbell_step(
            &transformed_state.position,
            &transformed_state.velocity,
            &transformed_mass,
            &transformed_map,
            &wrong_force,
            0.02,
            StepProfileV1::default(),
        )
        .expect("wrong-force control should still produce a finite step");
        let wrong_lift = lift_state_from_basis(
            PhysicalStateV1 {
                position: wrong.position_next,
                velocity: wrong.velocity_next,
            },
            &basis,
        );
        assert!(
            norm6(&subtract6(&wrong_lift.position, &canonical.position_next))
                > 1.0e-5
        );
    }

    #[test]
    fn physical_rotation_remains_metric_isometric_after_dense_rebasis() {
        let masses = DumbbellMassesV1 {
            first: 1.3,
            second: 2.1,
        };
        let metric = mass_metric(masses);
        let basis = frozen_dense_basis(2);
        let inverse = inverse6(
            &basis,
            StepProfileV1::default().relative_pivot_tolerance,
        )
        .unwrap();
        let transformed_metric = transformed_mass_metric(&metric, &basis);

        let axis = [0.3, -0.4, 0.8];
        let generator3 = cross_matrix(&axis);
        let mut generator6 = [[0.0; 6]; 6];
        for block in 0..2 {
            for row in 0..3 {
                for column in 0..3 {
                    generator6[block * 3 + row][block * 3 + column] =
                        generator3[row][column];
                }
            }
        }
        let transformed_generator =
            transform_generator(&basis, &inverse, &generator6);
        assert!(
            metric_skew_relative_defect(&transformed_metric, &transformed_generator)
                < 1.0e-11
        );

        let rotation = rotation3(axis, 0.73);
        let physical_action = block_rotation3(&rotation);
        let transformed_action =
            transform_finite_action(&basis, &inverse, &physical_action);
        assert!(
            metric_isometry_relative_defect(
                &transformed_metric,
                &transformed_action
            ) < 1.0e-10
        );
    }

    #[test]
    fn discrete_step_commutes_with_physical_rotation_in_dense_coordinates() {
        let state = canonical_state();
        let masses = DumbbellMassesV1 {
            first: 1.3,
            second: 2.1,
        };
        let force = common_acceleration_force(masses);
        let basis = frozen_dense_basis(2);
        let basis_inverse = inverse6(
            &basis,
            StepProfileV1::default().relative_pivot_tolerance,
        )
        .unwrap();
        let transformed_state =
            transform_state_to_basis(state, &basis_inverse);
        let force = asymmetric_force();
        let transformed_force = transform_force_covector(&basis, &force);
        let transformed_metric =
            transformed_mass_metric(&mass_metric(masses), &basis);
        let transformed_map =
            multiply_constraint_by_basis(&relative_position_map(), &basis);

        let base_step = constrained_dumbbell_step(
            &transformed_state.position,
            &transformed_state.velocity,
            &transformed_metric,
            &transformed_map,
            &transformed_force,
            0.02,
            StepProfileV1::default(),
        )
        .unwrap();
        assert!(base_step.diagnostics.multiplier.abs() > 1.0e-2);

        let rotation3 = rotation3([0.3, -0.4, 0.8], 0.73);
        let physical_action = block_rotation3(&rotation3);
        let transformed_action =
            transform_finite_action(&basis, &basis_inverse, &physical_action);
        let transformed_action_inverse =
            inverse6(&transformed_action, StepProfileV1::default().relative_pivot_tolerance)
                .unwrap();
        let rotated_position = apply6(
            &transformed_action,
            &transformed_state.position,
        );
        let rotated_velocity = apply6(
            &transformed_action,
            &transformed_state.velocity,
        );
        let rotated_force = apply_rotation_to_force_covector(
            &transformed_action_inverse,
            &transformed_force,
        );

        let rotated_step = constrained_dumbbell_step(
            &rotated_position,
            &rotated_velocity,
            &transformed_metric,
            &transformed_map,
            &rotated_force,
            0.02,
            StepProfileV1::default(),
        )
        .unwrap();

        close6(
            &rotated_step.position_next,
            &apply6(&transformed_action, &base_step.position_next),
            1.0e-10,
        );
        close6(
            &rotated_step.velocity_next,
            &apply6(&transformed_action, &base_step.velocity_next),
            1.0e-10,
        );
        assert!(
            (rotated_step.diagnostics.multiplier - base_step.diagnostics.multiplier).abs()
                < 1.0e-10
        );
    }

    #[test]
    fn wrong_constraint_jacobian_rebasis_is_detected() {
        let state = canonical_state();
        let masses = DumbbellMassesV1 {
            first: 1.3,
            second: 2.1,
        };
        let force = asymmetric_force();
        let canonical = run_canonical(state, force, 0.02);
        let basis = frozen_dense_basis(2);
        let basis_inverse = inverse6(
            &basis,
            StepProfileV1::default().relative_pivot_tolerance,
        )
        .unwrap();
        let transformed_state = transform_state_to_basis(state, &basis_inverse);
        let transformed_mass = transformed_mass_metric(&mass_metric(masses), &basis);
        let transformed_force = transform_force_covector(&basis, &force);
        let wrong_map =
            multiply_constraint_by_basis(&relative_position_map(), &basis_inverse);
        let wrong = constrained_dumbbell_step(
            &transformed_state.position,
            &transformed_state.velocity,
            &transformed_mass,
            &wrong_map,
            &transformed_force,
            0.02,
            StepProfileV1::default(),
        )
        .expect("wrong-map control remains a finite solve");
        let lifted = lift_state_from_basis(
            PhysicalStateV1 {
                position: wrong.position_next,
                velocity: wrong.velocity_next,
            },
            &basis,
        );
        assert!(
            norm6(&subtract6(&lifted.position, &canonical.position_next)) > 1.0e-4
        );
    }

    #[test]
    fn wrong_generator_conjugation_is_not_metric_skew() {
        let masses = DumbbellMassesV1 {
            first: 1.3,
            second: 2.1,
        };
        let metric = mass_metric(masses);
        let basis = frozen_dense_basis(2);
        let inverse = inverse6(
            &basis,
            StepProfileV1::default().relative_pivot_tolerance,
        )
        .unwrap();
        let transformed_metric = transformed_mass_metric(&metric, &basis);
        let generator3 = cross_matrix(&[0.3, -0.4, 0.8]);
        let mut generator6 = [[0.0; 6]; 6];
        for block in 0..2 {
            for row in 0..3 {
                for column in 0..3 {
                    generator6[block * 3 + row][block * 3 + column] =
                        generator3[row][column];
                }
            }
        }

        let correct = transform_generator(&basis, &inverse, &generator6);
        let wrong = multiply6(&basis, &multiply6(&generator6, &inverse));
        assert!(
            metric_skew_relative_defect(&transformed_metric, &correct) < 1.0e-11
        );
        assert!(
            metric_skew_relative_defect(&transformed_metric, &wrong) > 1.0e-4
        );
    }

    #[test]
    fn singular_coordinate_change_is_rejected() {
        let mut singular = identity6();
        singular[5] = singular[4];
        assert!(matches!(
            inverse6(&singular, StepProfileV1::default().relative_pivot_tolerance),
            Err(CovarianceError::SingularMatrix)
        ));
    }
}
