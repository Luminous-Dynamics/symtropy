// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Outward-rounded interval-Newton certificate for a bounded constrained step.
//!
//! The scalar reduced equation is:
//!   A(lambda) = I - (h^2 lambda / 4) W
//!   rbar(lambda) = A(lambda)^-1 r_free
//!   phi(lambda) = (2/h) rbar^T (rbar - r0)
//!
//! A unique root is certified only when the interval Newton image lies strictly
//! inside the candidate interval and the interval derivative excludes zero.
//! The resulting root interval is propagated through the declared one-step map.
//!
//! This crate certifies the reduced equation and step map defined by its exact
//! binary64 inputs. It does not independently prove that W and B were formed
//! from a separate physical M and J correctly; that provenance is an upstream
//! obligation. All arithmetic operations are widened by one representable
//! binary64 value in each direction. Non-finite/overflow and singular intervals
//! fail closed.

#![forbid(unsafe_code)]

use std::fmt;

pub type Vector3 = [f64; 3];
pub type Matrix3 = [[f64; 3]; 3];
pub type Vector6 = [f64; 6];
pub type Matrix6 = [[f64; 6]; 6];
pub type Matrix6x3 = [[f64; 3]; 6];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntervalV1 {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalErrorV1 {
    NonFiniteInput,
    NonFiniteResult,
    InvalidBounds,
    DivisionByZeroInterval,
    NegativeSquareRoot,
    SingularMatrixInterval,
    ProblemBindingMismatch,
}

impl fmt::Display for IntervalErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IntervalErrorV1 {}

impl IntervalV1 {
    pub fn point(value: f64) -> Result<Self, IntervalErrorV1> {
        if !value.is_finite() {
            return Err(IntervalErrorV1::NonFiniteInput);
        }
        Ok(Self {
            lower: value,
            upper: value,
        })
    }

    pub fn new(lower: f64, upper: f64) -> Result<Self, IntervalErrorV1> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalErrorV1::NonFiniteInput);
        }
        if lower > upper {
            return Err(IntervalErrorV1::InvalidBounds);
        }
        Ok(Self { lower, upper })
    }

    fn outward(lower: f64, upper: f64) -> Result<Self, IntervalErrorV1> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalErrorV1::NonFiniteResult);
        }
        let lower = lower.next_down();
        let upper = upper.next_up();
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalErrorV1::NonFiniteResult);
        }
        Self::new(lower, upper)
    }

    pub fn contains(&self, value: f64) -> bool {
        self.lower <= value && value <= self.upper
    }

    pub fn contains_zero(&self) -> bool {
        self.lower <= 0.0 && self.upper >= 0.0
    }

    pub fn strictly_inside(&self, outer: &Self) -> bool {
        self.lower > outer.lower && self.upper < outer.upper
    }

    pub fn intersection(&self, other: &Self) -> Option<Self> {
        let lower = self.lower.max(other.lower);
        let upper = self.upper.min(other.upper);
        (lower <= upper).then_some(Self { lower, upper })
    }

    pub fn midpoint(&self) -> f64 {
        0.5 * self.lower + 0.5 * self.upper
    }

    pub fn max_abs(&self) -> f64 {
        self.lower.abs().max(self.upper.abs())
    }

    pub fn checked_add(self, other: Self) -> Result<Self, IntervalErrorV1> {
        Self::outward(self.lower + other.lower, self.upper + other.upper)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, IntervalErrorV1> {
        Self::outward(self.lower - other.upper, self.upper - other.lower)
    }

    pub fn checked_mul(self, other: Self) -> Result<Self, IntervalErrorV1> {
        let products = [
            self.lower * other.lower,
            self.lower * other.upper,
            self.upper * other.lower,
            self.upper * other.upper,
        ];
        if products.iter().any(|value| !value.is_finite()) {
            return Err(IntervalErrorV1::NonFiniteResult);
        }
        let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
        let upper = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self::outward(lower, upper)
    }

    pub fn checked_div(self, other: Self) -> Result<Self, IntervalErrorV1> {
        if other.contains_zero() {
            return Err(IntervalErrorV1::DivisionByZeroInterval);
        }
        let reciprocal_a = 1.0 / other.lower;
        let reciprocal_b = 1.0 / other.upper;
        if !reciprocal_a.is_finite() || !reciprocal_b.is_finite() {
            return Err(IntervalErrorV1::NonFiniteResult);
        }
        let reciprocal = Self::outward(
            reciprocal_a.min(reciprocal_b),
            reciprocal_a.max(reciprocal_b),
        )?;
        self.checked_mul(reciprocal)
    }

    pub fn sqrt_upper(self) -> Result<f64, IntervalErrorV1> {
        if self.upper < 0.0 {
            return Err(IntervalErrorV1::NegativeSquareRoot);
        }
        let upper = self.upper.sqrt().next_up();
        if !upper.is_finite() {
            return Err(IntervalErrorV1::NonFiniteResult);
        }
        Ok(upper)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidpointProblemV1 {
    pub time_step: f64,
    pub initial_relative_position: Vector3,
    pub free_relative_position: Vector3,
    pub delassus: Matrix3,
    pub initial_position: Vector6,
    pub initial_velocity: Vector6,
    pub free_midpoint_velocity: Vector6,
    pub mass_inverse_jacobian_transpose: Matrix6x3,
    pub mass_metric: Matrix6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemErrorV1 {
    InvalidTimeStep,
    NonFiniteInput,
    DelassusNotSymmetric,
    DelassusNotPositiveDefinite,
    MassMetricNotSymmetric,
    MassMetricNotPositiveDefinite,
}

impl fmt::Display for ProblemErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ProblemErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InconclusiveReasonV1 {
    IntervalArithmeticFailure(IntervalErrorV1),
    DerivativeIntervalContainsZero,
    NewtonImageNotStrictlyInside,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootCertificateV1 {
    candidate_multiplier: f64,
    starting_interval: IntervalV1,
    residual_at_candidate: IntervalV1,
    derivative_on_starting_interval: IntervalV1,
    interval_newton_image: IntervalV1,
    certified_root_interval: IntervalV1,
    problem_binding: MidpointProblemV1,
}

impl RootCertificateV1 {
    pub fn candidate_multiplier(&self) -> f64 {
        self.candidate_multiplier
    }

    pub fn starting_interval(&self) -> IntervalV1 {
        self.starting_interval
    }

    pub fn residual_at_candidate(&self) -> IntervalV1 {
        self.residual_at_candidate
    }

    pub fn derivative_on_starting_interval(&self) -> IntervalV1 {
        self.derivative_on_starting_interval
    }

    pub fn interval_newton_image(&self) -> IntervalV1 {
        self.interval_newton_image
    }

    pub fn certified_root_interval(&self) -> IntervalV1 {
        self.certified_root_interval
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RootOutcomeV1 {
    CertifiedUniqueRoot(Box<RootCertificateV1>),
    Inconclusive {
        candidate_multiplier: f64,
        starting_interval: IntervalV1,
        reason: InconclusiveReasonV1,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepErrorBoundsV1 {
    pub position_componentwise_radius: Vector6,
    pub velocity_componentwise_radius: Vector6,
    pub position_euclidean_upper: f64,
    pub velocity_euclidean_upper: f64,
    pub position_mass_metric_upper: f64,
    pub velocity_kinetic_metric_upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepCertificateV1 {
    root: RootCertificateV1,
    position_next_interval: [IntervalV1; 6],
    velocity_next_interval: [IntervalV1; 6],
    returned_position_next: Vector6,
    returned_velocity_next: Vector6,
    error_bounds: StepErrorBoundsV1,
}

impl StepCertificateV1 {
    pub fn root(&self) -> &RootCertificateV1 {
        &self.root
    }

    pub fn position_next_interval(&self) -> &[IntervalV1; 6] {
        &self.position_next_interval
    }

    pub fn velocity_next_interval(&self) -> &[IntervalV1; 6] {
        &self.velocity_next_interval
    }

    pub fn returned_position_next(&self) -> &Vector6 {
        &self.returned_position_next
    }

    pub fn returned_velocity_next(&self) -> &Vector6 {
        &self.returned_velocity_next
    }

    pub fn error_bounds(&self) -> &StepErrorBoundsV1 {
        &self.error_bounds
    }
}

fn same_vector_bits<const N: usize>(left: &[f64; N], right: &[f64; N]) -> bool {
    left.iter()
        .zip(right.iter())
        .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn same_matrix_bits<const R: usize, const C: usize>(
    left: &[[f64; C]; R],
    right: &[[f64; C]; R],
) -> bool {
    left.iter()
        .zip(right.iter())
        .all(|(left_row, right_row)| same_vector_bits(left_row, right_row))
}

fn same_problem_bits(left: &MidpointProblemV1, right: &MidpointProblemV1) -> bool {
    left.time_step.to_bits() == right.time_step.to_bits()
        && same_vector_bits(
            &left.initial_relative_position,
            &right.initial_relative_position,
        )
        && same_vector_bits(&left.free_relative_position, &right.free_relative_position)
        && same_matrix_bits(&left.delassus, &right.delassus)
        && same_vector_bits(&left.initial_position, &right.initial_position)
        && same_vector_bits(&left.initial_velocity, &right.initial_velocity)
        && same_vector_bits(&left.free_midpoint_velocity, &right.free_midpoint_velocity)
        && same_matrix_bits(
            &left.mass_inverse_jacobian_transpose,
            &right.mass_inverse_jacobian_transpose,
        )
        && same_matrix_bits(&left.mass_metric, &right.mass_metric)
}

fn interval_dot3(
    left: &[IntervalV1; 3],
    right: &[IntervalV1; 3],
) -> Result<IntervalV1, IntervalErrorV1> {
    let mut total = IntervalV1::point(0.0)?;
    for index in 0..3 {
        total = total.checked_add(left[index].checked_mul(right[index])?)?;
    }
    Ok(total)
}

fn interval_solve3(
    matrix: &[[IntervalV1; 3]; 3],
    rhs: &[IntervalV1; 3],
) -> Result<[IntervalV1; 3], IntervalErrorV1> {
    let a = matrix;
    let c00 = a[1][1]
        .checked_mul(a[2][2])?
        .checked_sub(a[1][2].checked_mul(a[2][1])?)?;
    let c01 = a[1][2]
        .checked_mul(a[2][0])?
        .checked_sub(a[1][0].checked_mul(a[2][2])?)?;
    let c02 = a[1][0]
        .checked_mul(a[2][1])?
        .checked_sub(a[1][1].checked_mul(a[2][0])?)?;
    let determinant = a[0][0]
        .checked_mul(c00)?
        .checked_add(a[0][1].checked_mul(c01)?)?
        .checked_add(a[0][2].checked_mul(c02)?)?;
    if determinant.contains_zero() {
        return Err(IntervalErrorV1::SingularMatrixInterval);
    }

    let adjugate = [
        [
            c00,
            a[0][2]
                .checked_mul(a[2][1])?
                .checked_sub(a[0][1].checked_mul(a[2][2])?)?,
            a[0][1]
                .checked_mul(a[1][2])?
                .checked_sub(a[0][2].checked_mul(a[1][1])?)?,
        ],
        [
            c01,
            a[0][0]
                .checked_mul(a[2][2])?
                .checked_sub(a[0][2].checked_mul(a[2][0])?)?,
            a[0][2]
                .checked_mul(a[1][0])?
                .checked_sub(a[0][0].checked_mul(a[1][2])?)?,
        ],
        [
            c02,
            a[0][1]
                .checked_mul(a[2][0])?
                .checked_sub(a[0][0].checked_mul(a[2][1])?)?,
            a[0][0]
                .checked_mul(a[1][1])?
                .checked_sub(a[0][1].checked_mul(a[1][0])?)?,
        ],
    ];

    let mut result = [IntervalV1::point(0.0)?; 3];
    for row in 0..3 {
        let mut numerator = IntervalV1::point(0.0)?;
        for column in 0..3 {
            numerator = numerator.checked_add(adjugate[row][column].checked_mul(rhs[column])?)?;
        }
        result[row] = numerator.checked_div(determinant)?;
    }
    Ok(result)
}

fn interval_matrix_vector3(
    matrix: &Matrix3,
    vector: &[IntervalV1; 3],
) -> Result<[IntervalV1; 3], IntervalErrorV1> {
    let mut result = [IntervalV1::point(0.0)?; 3];
    for (result_entry, matrix_row) in result.iter_mut().zip(matrix.iter()) {
        for (matrix_entry, vector_entry) in matrix_row.iter().zip(vector.iter()) {
            *result_entry = (*result_entry)
                .checked_add(IntervalV1::point(*matrix_entry)?.checked_mul(*vector_entry)?)?;
        }
    }
    Ok(result)
}

fn reduced_system(
    problem: &MidpointProblemV1,
    lambda: IntervalV1,
) -> Result<([[IntervalV1; 3]; 3], IntervalV1), IntervalErrorV1> {
    let h = IntervalV1::point(problem.time_step)?;
    let coefficient = h.checked_mul(h)?.checked_div(IntervalV1::point(4.0)?)?;
    let scaled_lambda = coefficient.checked_mul(lambda)?;
    let mut matrix = [[IntervalV1::point(0.0)?; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            let identity = IntervalV1::point(if row == column { 1.0 } else { 0.0 })?;
            let correction =
                scaled_lambda.checked_mul(IntervalV1::point(problem.delassus[row][column])?)?;
            matrix[row][column] = identity.checked_sub(correction)?;
        }
    }
    Ok((matrix, coefficient))
}

fn evaluate_reduced_equation(
    problem: &MidpointProblemV1,
    lambda: IntervalV1,
) -> Result<(IntervalV1, IntervalV1, [IntervalV1; 3]), IntervalErrorV1> {
    let (system, coefficient) = reduced_system(problem, lambda)?;
    let free = [
        IntervalV1::point(problem.free_relative_position[0])?,
        IntervalV1::point(problem.free_relative_position[1])?,
        IntervalV1::point(problem.free_relative_position[2])?,
    ];
    let initial = [
        IntervalV1::point(problem.initial_relative_position[0])?,
        IntervalV1::point(problem.initial_relative_position[1])?,
        IntervalV1::point(problem.initial_relative_position[2])?,
    ];
    let relative_midpoint = interval_solve3(&system, &free)?;
    let relative_difference = [
        relative_midpoint[0].checked_sub(initial[0])?,
        relative_midpoint[1].checked_sub(initial[1])?,
        relative_midpoint[2].checked_sub(initial[2])?,
    ];
    let h_scale = IntervalV1::point(2.0)?.checked_div(IntervalV1::point(problem.time_step)?)?;
    let mapped_midpoint_velocity = [
        h_scale.checked_mul(relative_difference[0])?,
        h_scale.checked_mul(relative_difference[1])?,
        h_scale.checked_mul(relative_difference[2])?,
    ];
    let residual = interval_dot3(&relative_midpoint, &mapped_midpoint_velocity)?;

    let delassus_times_relative = interval_matrix_vector3(&problem.delassus, &relative_midpoint)?;
    let relative_prime_unscaled = interval_solve3(&system, &delassus_times_relative)?;
    let relative_prime = [
        coefficient.checked_mul(relative_prime_unscaled[0])?,
        coefficient.checked_mul(relative_prime_unscaled[1])?,
        coefficient.checked_mul(relative_prime_unscaled[2])?,
    ];
    let twice_relative_minus_initial = [
        relative_midpoint[0]
            .checked_add(relative_midpoint[0])?
            .checked_sub(initial[0])?,
        relative_midpoint[1]
            .checked_add(relative_midpoint[1])?
            .checked_sub(initial[1])?,
        relative_midpoint[2]
            .checked_add(relative_midpoint[2])?
            .checked_sub(initial[2])?,
    ];
    let derivative_scale =
        IntervalV1::point(2.0)?.checked_div(IntervalV1::point(problem.time_step)?)?;
    let derivative = derivative_scale.checked_mul(interval_dot3(
        &relative_prime,
        &twice_relative_minus_initial,
    )?)?;

    Ok((residual, derivative, relative_midpoint))
}

fn cholesky_check<const N: usize>(matrix: &[[f64; N]; N], symmetry_tolerance: f64) -> bool {
    let mut scale = 0.0_f64;
    for matrix_row in matrix {
        for value in matrix_row {
            scale = scale.max(value.abs());
        }
    }
    if !scale.is_finite() || scale == 0.0 {
        return false;
    }
    let mut lower = [[0.0; N]; N];
    for (row, matrix_row) in matrix.iter().enumerate() {
        for (column, value) in matrix_row.iter().enumerate() {
            if (*value - matrix[column][row]).abs() > symmetry_tolerance * scale {
                return false;
            }
        }
        for (column, value) in matrix_row.iter().enumerate().take(row + 1) {
            let mut value = 0.5 * (*value + matrix[column][row]) / scale;
            for (left, right) in lower[row].iter().zip(lower[column].iter()).take(column) {
                value -= left * right;
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return false;
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    true
}

impl MidpointProblemV1 {
    pub fn validate(&self) -> Result<(), ProblemErrorV1> {
        if !self.time_step.is_finite() || self.time_step <= 0.0 {
            return Err(ProblemErrorV1::InvalidTimeStep);
        }
        let all_finite = self.initial_relative_position.iter().all(|x| x.is_finite())
            && self.free_relative_position.iter().all(|x| x.is_finite())
            && self.delassus.iter().flatten().all(|x| x.is_finite())
            && self.initial_position.iter().all(|x| x.is_finite())
            && self.initial_velocity.iter().all(|x| x.is_finite())
            && self.free_midpoint_velocity.iter().all(|x| x.is_finite())
            && self
                .mass_inverse_jacobian_transpose
                .iter()
                .flatten()
                .all(|x| x.is_finite())
            && self.mass_metric.iter().flatten().all(|x| x.is_finite());
        if !all_finite {
            return Err(ProblemErrorV1::NonFiniteInput);
        }
        if !cholesky_check(&self.delassus, 1.0e-13) {
            return Err(ProblemErrorV1::DelassusNotPositiveDefinite);
        }
        if !cholesky_check(&self.mass_metric, 1.0e-13) {
            return Err(ProblemErrorV1::MassMetricNotPositiveDefinite);
        }
        Ok(())
    }
}

fn solve3_point(matrix: &Matrix3, rhs: &Vector3) -> Result<Vector3, IntervalErrorV1> {
    let mut augmented = [[0.0; 4]; 3];
    for ((augmented_row, matrix_row), rhs_value) in
        augmented.iter_mut().zip(matrix.iter()).zip(rhs.iter())
    {
        augmented_row[..3].copy_from_slice(matrix_row);
        augmented_row[3] = *rhs_value;
    }
    for column in 0..3 {
        let pivot_row = (column..3)
            .max_by(|left, right| {
                augmented[*left][column]
                    .abs()
                    .total_cmp(&augmented[*right][column].abs())
            })
            .ok_or(IntervalErrorV1::SingularMatrixInterval)?;
        if augmented[pivot_row][column] == 0.0 {
            return Err(IntervalErrorV1::SingularMatrixInterval);
        }
        if pivot_row != column {
            augmented.swap(pivot_row, column);
        }
        let pivot = augmented[column][column];
        for entry in augmented[column].iter_mut().skip(column) {
            *entry /= pivot;
        }
        let pivot_values = augmented[column];
        for (row_index, row_values) in augmented.iter_mut().enumerate() {
            if row_index == column {
                continue;
            }
            let factor = row_values[column];
            for (entry, pivot_entry) in row_values.iter_mut().zip(pivot_values.iter()).skip(column) {
                *entry -= factor * pivot_entry;
            }
        }
    }
    let result = [augmented[0][3], augmented[1][3], augmented[2][3]];
    if result.iter().any(|value| !value.is_finite()) {
        return Err(IntervalErrorV1::NonFiniteResult);
    }
    Ok(result)
}

fn candidate_step(
    problem: &MidpointProblemV1,
    lambda: f64,
) -> Result<(Vector6, Vector6), IntervalErrorV1> {
    let h2_over_four = problem.time_step * problem.time_step / 4.0;
    let mut system = [[0.0; 3]; 3];
    for (row, (system_row, delassus_row)) in
        system.iter_mut().zip(problem.delassus.iter()).enumerate()
    {
        for (column, (system_value, delassus_value)) in
            system_row.iter_mut().zip(delassus_row.iter()).enumerate()
        {
            *system_value =
                (if row == column { 1.0 } else { 0.0 }) - h2_over_four * lambda * delassus_value;
        }
    }
    let relative = solve3_point(&system, &problem.free_relative_position)?;
    let mut midpoint_velocity = problem.free_midpoint_velocity;
    for (midpoint_entry, b_row) in midpoint_velocity
        .iter_mut()
        .zip(problem.mass_inverse_jacobian_transpose.iter())
    {
        let correction: f64 = b_row
            .iter()
            .zip(relative.iter())
            .map(|(b_component, relative_component)| b_component * relative_component)
            .sum();
        *midpoint_entry += problem.time_step * lambda * 0.5 * correction;
    }
    let position_next = std::array::from_fn(|index| {
        problem.initial_position[index] + problem.time_step * midpoint_velocity[index]
    });
    let velocity_next = std::array::from_fn(|index| {
        2.0 * midpoint_velocity[index] - problem.initial_velocity[index]
    });
    if position_next
        .iter()
        .chain(velocity_next.iter())
        .any(|x| !x.is_finite())
    {
        return Err(IntervalErrorV1::NonFiniteResult);
    }
    Ok((position_next, velocity_next))
}

fn interval_step(
    problem: &MidpointProblemV1,
    lambda: IntervalV1,
) -> Result<([IntervalV1; 6], [IntervalV1; 6]), IntervalErrorV1> {
    let (_, _, relative) = evaluate_reduced_equation(problem, lambda)?;
    let lambda_half_h = IntervalV1::point(problem.time_step)?
        .checked_mul(lambda)?
        .checked_div(IntervalV1::point(2.0)?)?;
    let mut position = [IntervalV1::point(0.0)?; 6];
    let mut velocity = [IntervalV1::point(0.0)?; 6];
    for row in 0..6 {
        let mut correction = IntervalV1::point(0.0)?;
        for (b_component, relative_component) in problem.mass_inverse_jacobian_transpose[row]
            .iter()
            .zip(relative.iter())
        {
            correction = correction
                .checked_add(IntervalV1::point(*b_component)?.checked_mul(*relative_component)?)?;
        }
        let midpoint = IntervalV1::point(problem.free_midpoint_velocity[row])?
            .checked_add(lambda_half_h.checked_mul(correction)?)?;
        position[row] = IntervalV1::point(problem.initial_position[row])?
            .checked_add(IntervalV1::point(problem.time_step)?.checked_mul(midpoint)?)?;
        velocity[row] = IntervalV1::point(2.0)?
            .checked_mul(midpoint)?
            .checked_sub(IntervalV1::point(problem.initial_velocity[row])?)?;
    }
    Ok((position, velocity))
}

fn euclidean_radius_upper(radii: &Vector6) -> Result<f64, IntervalErrorV1> {
    let mut squared = IntervalV1::point(0.0)?;
    for radius in radii {
        squared = squared
            .checked_add(IntervalV1::point(*radius)?.checked_mul(IntervalV1::point(*radius)?)?)?;
    }
    squared.sqrt_upper()
}

fn mass_metric_radius_upper(metric: &Matrix6, radii: &Vector6) -> Result<f64, IntervalErrorV1> {
    let mut upper = IntervalV1::point(0.0)?;
    for row in 0..6 {
        for column in 0..6 {
            let term = IntervalV1::point(metric[row][column].abs())?
                .checked_mul(IntervalV1::point(radii[row])?)?
                .checked_mul(IntervalV1::point(radii[column])?)?;
            upper = upper.checked_add(term)?;
        }
    }
    upper.sqrt_upper()
}

fn interval_error_radii(
    exact_interval: &[IntervalV1; 6],
    candidate: &Vector6,
) -> Result<Vector6, IntervalErrorV1> {
    let mut radii = [0.0; 6];
    for index in 0..6 {
        let error = exact_interval[index].checked_sub(IntervalV1::point(candidate[index])?)?;
        radii[index] = error.max_abs();
    }
    Ok(radii)
}

fn inconclusive(
    candidate_multiplier: f64,
    starting_interval: IntervalV1,
    reason: InconclusiveReasonV1,
) -> RootOutcomeV1 {
    RootOutcomeV1::Inconclusive {
        candidate_multiplier,
        starting_interval,
        reason,
    }
}

pub fn certify_midpoint_step(
    problem: &MidpointProblemV1,
    candidate_multiplier: f64,
    radius: f64,
) -> Result<RootOutcomeV1, ProblemErrorV1> {
    problem.validate()?;
    if !candidate_multiplier.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return Err(ProblemErrorV1::NonFiniteInput);
    }

    let initial_interval = IntervalV1::point(candidate_multiplier).and_then(|center| {
        let width = IntervalV1::point(radius)?;
        let lower = center.checked_sub(width)?;
        let upper = center.checked_add(width)?;
        IntervalV1::new(lower.lower, upper.upper)
    });
    let starting_interval = match initial_interval {
        Ok(interval) => interval,
        Err(error) => {
            return Ok(inconclusive(
                candidate_multiplier,
                IntervalV1::point(candidate_multiplier)
                    .map_err(|_| ProblemErrorV1::NonFiniteInput)?,
                InconclusiveReasonV1::IntervalArithmeticFailure(error),
            ));
        }
    };

    let point_lambda = match IntervalV1::point(candidate_multiplier) {
        Ok(value) => value,
        Err(error) => {
            return Ok(inconclusive(
                candidate_multiplier,
                starting_interval,
                InconclusiveReasonV1::IntervalArithmeticFailure(error),
            ));
        }
    };
    let (residual_at_candidate, _, _) = match evaluate_reduced_equation(problem, point_lambda) {
        Ok(value) => value,
        Err(error) => {
            return Ok(inconclusive(
                candidate_multiplier,
                starting_interval,
                InconclusiveReasonV1::IntervalArithmeticFailure(error),
            ));
        }
    };
    let (_, derivative_on_starting_interval, _) =
        match evaluate_reduced_equation(problem, starting_interval) {
            Ok(value) => value,
            Err(error) => {
                return Ok(inconclusive(
                    candidate_multiplier,
                    starting_interval,
                    InconclusiveReasonV1::IntervalArithmeticFailure(error),
                ));
            }
        };

    if derivative_on_starting_interval.contains_zero() {
        return Ok(inconclusive(
            candidate_multiplier,
            starting_interval,
            InconclusiveReasonV1::DerivativeIntervalContainsZero,
        ));
    }
    let residual_over_derivative =
        match residual_at_candidate.checked_div(derivative_on_starting_interval) {
            Ok(value) => value,
            Err(error) => {
                return Ok(inconclusive(
                    candidate_multiplier,
                    starting_interval,
                    InconclusiveReasonV1::IntervalArithmeticFailure(error),
                ));
            }
        };
    let interval_newton_image = match point_lambda.checked_sub(residual_over_derivative) {
        Ok(value) => value,
        Err(error) => {
            return Ok(inconclusive(
                candidate_multiplier,
                starting_interval,
                InconclusiveReasonV1::IntervalArithmeticFailure(error),
            ));
        }
    };
    if !interval_newton_image.strictly_inside(&starting_interval) {
        return Ok(inconclusive(
            candidate_multiplier,
            starting_interval,
            InconclusiveReasonV1::NewtonImageNotStrictlyInside,
        ));
    }
    let Some(certified_root_interval) = interval_newton_image.intersection(&starting_interval)
    else {
        return Ok(inconclusive(
            candidate_multiplier,
            starting_interval,
            InconclusiveReasonV1::NewtonImageNotStrictlyInside,
        ));
    };

    let root = RootCertificateV1 {
        candidate_multiplier,
        starting_interval,
        residual_at_candidate,
        derivative_on_starting_interval,
        interval_newton_image,
        certified_root_interval,
        problem_binding: *problem,
    };
    Ok(RootOutcomeV1::CertifiedUniqueRoot(Box::new(root)))
}

pub fn certify_step_error_bounds(
    problem: &MidpointProblemV1,
    root: &RootCertificateV1,
) -> Result<StepCertificateV1, IntervalErrorV1> {
    if !same_problem_bits(problem, &root.problem_binding) {
        return Err(IntervalErrorV1::ProblemBindingMismatch);
    }
    problem
        .validate()
        .map_err(|_| IntervalErrorV1::ProblemBindingMismatch)?;
    let (returned_position_next, returned_velocity_next) =
        candidate_step(problem, root.candidate_multiplier)?;
    let (position_next_interval, velocity_next_interval) =
        interval_step(problem, root.certified_root_interval)?;
    let position_radii = interval_error_radii(&position_next_interval, &returned_position_next)?;
    let velocity_radii = interval_error_radii(&velocity_next_interval, &returned_velocity_next)?;
    let error_bounds = StepErrorBoundsV1 {
        position_componentwise_radius: position_radii,
        velocity_componentwise_radius: velocity_radii,
        position_euclidean_upper: euclidean_radius_upper(&position_radii)?,
        velocity_euclidean_upper: euclidean_radius_upper(&velocity_radii)?,
        position_mass_metric_upper: mass_metric_radius_upper(
            &problem.mass_metric,
            &position_radii,
        )?,
        velocity_kinetic_metric_upper: mass_metric_radius_upper(
            &problem.mass_metric,
            &velocity_radii,
        )?,
    };

    Ok(StepCertificateV1 {
        root: *root,
        position_next_interval,
        velocity_next_interval,
        returned_position_next,
        returned_velocity_next,
        error_bounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> MidpointProblemV1 {
        let mut mass = [[0.0; 6]; 6];
        mass[0][0] = 1.0;
        mass[1][1] = 1.0;
        mass[2][2] = 1.0;
        mass[3][3] = 2.0;
        mass[4][4] = 2.0;
        mass[5][5] = 2.0;
        MidpointProblemV1 {
            time_step: 0.2,
            initial_relative_position: [1.0, 0.0, 0.0],
            free_relative_position: [1.1, 0.0, 0.0],
            delassus: [[1.5, 0.0, 0.0], [0.0, 1.5, 0.0], [0.0, 0.0, 1.5]],
            initial_position: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            initial_velocity: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            free_midpoint_velocity: [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            mass_inverse_jacobian_transpose: [
                [-1.0, 0.0, 0.0],
                [0.0, -1.0, 0.0],
                [0.0, 0.0, -1.0],
                [0.5, 0.0, 0.0],
                [0.0, 0.5, 0.0],
                [0.0, 0.0, 0.5],
            ],
            mass_metric: mass,
        }
    }

    #[test]
    fn interval_arithmetic_encloses_exact_dyadic_operations() {
        let left = IntervalV1::new(0.25, 0.5).unwrap();
        let right = IntervalV1::new(2.0, 4.0).unwrap();
        let product = left.checked_mul(right).unwrap();
        assert!(product.contains(0.5));
        assert!(product.contains(2.0));

        let quotient = right.checked_div(left).unwrap();
        assert!(quotient.contains(4.0));
        assert!(quotient.contains(16.0));

        assert_eq!(
            IntervalV1::new(-1.0, 1.0)
                .unwrap()
                .checked_div(IntervalV1::new(-0.1, 0.2).unwrap()),
            Err(IntervalErrorV1::DivisionByZeroInterval)
        );
    }

    #[test]
    fn interval_linear_solve_encloses_non_diagonal_rational_solution() {
        let matrix_values = [[2.0, 1.0, 0.0], [1.0, 3.0, 1.0], [0.0, 1.0, 2.0]];
        let rhs_values = [1.0, 2.0, 3.0];
        let matrix = std::array::from_fn(|row| {
            std::array::from_fn(|column| IntervalV1::point(matrix_values[row][column]).unwrap())
        });
        let rhs = std::array::from_fn(|index| IntervalV1::point(rhs_values[index]).unwrap());
        let solution = interval_solve3(&matrix, &rhs).unwrap();
        let exact = [0.5, 0.0, 1.5];
        for index in 0..3 {
            assert!(solution[index].contains(exact[index]));
        }
    }

    #[test]
    fn outward_square_root_accepts_zero_lower_bound_inflation() {
        let widened_zero = IntervalV1::point(0.0)
            .unwrap()
            .checked_mul(IntervalV1::point(0.0).unwrap())
            .unwrap();
        assert!(widened_zero.lower < 0.0);
        assert!(widened_zero.sqrt_upper().is_ok());
    }

    #[test]
    fn interval_newton_certifies_unique_midpoint_multiplier_root() {
        let problem = fixture();
        let candidate = -0.1 / (0.01 * 1.5);
        let outcome = certify_midpoint_step(&problem, candidate, 1.0e-5).unwrap();
        let RootOutcomeV1::CertifiedUniqueRoot(root) = outcome else {
            panic!("expected an interval-Newton unique-root certificate, got {outcome:?}");
        };

        let analytic_root = IntervalV1::point(
            problem.initial_relative_position[0] - problem.free_relative_position[0],
        )
        .unwrap()
        .checked_div(
            IntervalV1::point(problem.time_step)
                .unwrap()
                .checked_mul(IntervalV1::point(problem.time_step).unwrap())
                .unwrap()
                .checked_div(IntervalV1::point(4.0).unwrap())
                .unwrap()
                .checked_mul(IntervalV1::point(problem.delassus[0][0]).unwrap())
                .unwrap(),
        )
        .unwrap();
        assert!(
            root.certified_root_interval
                .intersection(&analytic_root)
                .is_some()
        );
        assert!(
            root.interval_newton_image
                .strictly_inside(&root.starting_interval)
        );
        assert!(!root.derivative_on_starting_interval.contains_zero());
        assert!(
            root.certified_root_interval.upper - root.certified_root_interval.lower
                < root.starting_interval.upper - root.starting_interval.lower
        );
    }

    #[test]
    fn certified_root_propagates_to_metric_induced_position_and_velocity_bounds() {
        let problem = fixture();
        let candidate = -0.1 / (0.01 * 1.5);
        let outcome = certify_midpoint_step(&problem, candidate, 1.0e-5).unwrap();
        let RootOutcomeV1::CertifiedUniqueRoot(root) = outcome else {
            panic!("expected certified root");
        };
        let certificate = certify_step_error_bounds(&problem, &root).unwrap();

        assert!(
            certificate
                .error_bounds
                .position_euclidean_upper
                .is_finite()
        );
        assert!(
            certificate
                .error_bounds
                .velocity_euclidean_upper
                .is_finite()
        );
        assert!(
            certificate
                .error_bounds
                .position_mass_metric_upper
                .is_finite()
        );
        assert!(
            certificate
                .error_bounds
                .velocity_kinetic_metric_upper
                .is_finite()
        );
        assert!(certificate.error_bounds.position_mass_metric_upper > 0.0);
        assert!(certificate.error_bounds.velocity_kinetic_metric_upper > 0.0);

        // Independent analytic relation for this diagonal fixture: at the root,
        // rbar = r0, so lambda = (r0 - r_free) / ((h^2/4) W_00).
        let expected_lambda = (problem.initial_relative_position[0]
            - problem.free_relative_position[0])
            / ((problem.time_step * problem.time_step / 4.0) * problem.delassus[0][0]);
        assert!(
            (root.certified_root_interval.lower - expected_lambda).abs()
                <= root.certified_root_interval.upper - root.certified_root_interval.lower
                    + 2.0e-14
        );
        assert!(certificate.position_next_interval[0].contains(0.13333333333333344));
        assert!(certificate.position_next_interval[3].contains(1.1333333333333333));
        assert!(certificate.velocity_next_interval[0].contains(1.3333333333333344));
        assert!(certificate.velocity_next_interval[3].contains(0.33333333333333276));
    }

    #[test]
    fn step_error_bounds_reject_a_certificate_replayed_against_changed_inputs() {
        let problem = fixture();
        let candidate = -0.1 / (0.01 * 1.5);
        let outcome = certify_midpoint_step(&problem, candidate, 1.0e-5).unwrap();
        let RootOutcomeV1::CertifiedUniqueRoot(root) = outcome else {
            panic!("expected certified root");
        };

        let mut changed_problem = problem;
        changed_problem.delassus[0][0] = 1.5000000000000002;
        assert_eq!(
            certify_step_error_bounds(&changed_problem, &root),
            Err(IntervalErrorV1::ProblemBindingMismatch)
        );
    }

    #[test]
    fn candidate_interval_that_does_not_capture_the_root_is_inconclusive() {
        let problem = fixture();
        let outcome = certify_midpoint_step(&problem, -1.0, 1.0e-8).unwrap();
        assert!(matches!(
            outcome,
            RootOutcomeV1::Inconclusive {
                reason: InconclusiveReasonV1::NewtonImageNotStrictlyInside,
                ..
            } | RootOutcomeV1::Inconclusive {
                reason: InconclusiveReasonV1::DerivativeIntervalContainsZero,
                ..
            }
        ));
    }

    #[test]
    fn interval_that_crosses_a_reduced_system_singularity_is_inconclusive() {
        let problem = fixture();
        // For W=1.5 I and h=0.2, A(lambda) is singular at lambda=1/(0.01*1.5).
        let singular_lambda = 1.0 / (0.01 * 1.5);
        let outcome = certify_midpoint_step(&problem, singular_lambda, 1.0e-4).unwrap();
        assert!(matches!(
            outcome,
            RootOutcomeV1::Inconclusive {
                reason: InconclusiveReasonV1::IntervalArithmeticFailure(
                    IntervalErrorV1::SingularMatrixInterval
                ),
                ..
            }
        ));
    }

    #[test]
    fn nonfinite_and_indefinite_physical_inputs_fail_closed() {
        let mut problem = fixture();
        problem.delassus[0][0] = f64::NAN;
        assert_eq!(
            certify_midpoint_step(&problem, -6.0, 1.0e-5),
            Err(ProblemErrorV1::NonFiniteInput)
        );

        let mut problem = fixture();
        problem.mass_metric[0][0] = -1.0;
        assert_eq!(
            certify_midpoint_step(&problem, -6.0, 1.0e-5),
            Err(ProblemErrorV1::MassMetricNotPositiveDefinite)
        );
    }
}
