// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Dense-coordinate covariance reference for an implicit effective-metric step.
//!
//! The method is intentionally a small linear analogue, not an implementation
//! of MuJoCo's full contact solver:
//!
//!   M_hat = M + h D + h^2 K
//!   M_hat v_next = M v - h K q + h f
//!   q_next = q + h v_next
//!
//! Tangent states push forward by S, force covectors pull back by S^T, and
//! M/D/K transform by congruence. The dense-coordinate step is solved directly.

#![forbid(unsafe_code)]

use core::fmt;

pub type Vector2 = [f64; 2];
pub type Matrix2 = [[f64; 2]; 2];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveMetricErrorV1 {
    InvalidIdentifier,
    InvalidTimeStep,
    NonFiniteInput,
    MatrixNotSymmetric,
    MassNotPositiveDefinite,
    DampingNotPositiveSemidefinite,
    StiffnessNotPositiveSemidefinite,
    EffectiveMetricNotPositiveDefinite,
    EffectiveMetricOutsideConditionEnvelope,
    SingularBasis,
    BasisNotUnimodular,
    IntegerArithmeticOverflow,
    SingularSolve,
    ArithmeticOverflow,
}

impl fmt::Display for EffectiveMetricErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for EffectiveMetricErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveMetricProfileV1 {
    pub maximum_basis_entry: i64,
    pub minimum_pivot: f64,
    pub maximum_effective_condition_inf: f64,
    pub maximum_residual: f64,
}

impl Default for EffectiveMetricProfileV1 {
    fn default() -> Self {
        Self {
            maximum_basis_entry: 64,
            minimum_pivot: 1.0e-13,
            maximum_effective_condition_inf: 1.0e10,
            maximum_residual: 1.0e-11,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinearDiscreteProblem2V1 {
    pub case_id: String,
    pub time_step: f64,
    pub mass: Matrix2,
    pub damping: Matrix2,
    pub stiffness: Matrix2,
    pub position: Vector2,
    pub velocity: Vector2,
    /// Generalized force covector in the current coordinate frame.
    pub force_covector: Vector2,
}

impl LinearDiscreteProblem2V1 {
    pub fn validate(
        &self,
        profile: EffectiveMetricProfileV1,
    ) -> Result<(), EffectiveMetricErrorV1> {
        if self.case_id.trim().is_empty() {
            return Err(EffectiveMetricErrorV1::InvalidIdentifier);
        }
        if !self.time_step.is_finite() || self.time_step <= 0.0 {
            return Err(EffectiveMetricErrorV1::InvalidTimeStep);
        }
        if profile.maximum_basis_entry <= 0
            || !profile.minimum_pivot.is_finite()
            || profile.minimum_pivot <= 0.0
            || !profile.maximum_effective_condition_inf.is_finite()
            || profile.maximum_effective_condition_inf < 1.0
            || !profile.maximum_residual.is_finite()
            || profile.maximum_residual <= 0.0
        {
            return Err(EffectiveMetricErrorV1::InvalidIdentifier);
        }
        if self
            .mass
            .iter()
            .chain(self.damping.iter())
            .chain(self.stiffness.iter())
            .flatten()
            .chain(self.position.iter())
            .chain(self.velocity.iter())
            .chain(self.force_covector.iter())
            .any(|value| !value.is_finite())
        {
            return Err(EffectiveMetricErrorV1::NonFiniteInput);
        }

        if !is_exactly_symmetric(&self.mass)
            || !is_exactly_symmetric(&self.damping)
            || !is_exactly_symmetric(&self.stiffness)
        {
            return Err(EffectiveMetricErrorV1::MatrixNotSymmetric);
        }
        if !is_positive_definite(&self.mass, profile.minimum_pivot) {
            return Err(EffectiveMetricErrorV1::MassNotPositiveDefinite);
        }
        if !is_positive_semidefinite(&self.damping) {
            return Err(EffectiveMetricErrorV1::DampingNotPositiveSemidefinite);
        }
        if !is_positive_semidefinite(&self.stiffness) {
            return Err(EffectiveMetricErrorV1::StiffnessNotPositiveSemidefinite);
        }
        let effective = self.effective_metric()?;
        if !is_positive_definite(&effective, profile.minimum_pivot) {
            return Err(EffectiveMetricErrorV1::EffectiveMetricNotPositiveDefinite);
        }
        Ok(())
    }

    pub fn effective_metric(&self) -> Result<Matrix2, EffectiveMetricErrorV1> {
        let h = self.time_step;
        let h_squared = h * h;
        if !h_squared.is_finite() {
            return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
        }
        let mut result = [[0.0; 2]; 2];
        for (row, result_row) in result.iter_mut().enumerate() {
            for (column, value) in result_row.iter_mut().enumerate() {
                *value = self.mass[row][column]
                    + h * self.damping[row][column]
                    + h_squared * self.stiffness[row][column];
                if !value.is_finite() {
                    return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
                }
            }
        }
        Ok(result)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerFrame2V1 {
    pub frame_id: String,
    /// Maps coordinates in this frame to the physical frame: q = S y.
    pub basis_to_physical: [[i64; 2]; 2],
}

impl IntegerFrame2V1 {
    pub fn identity() -> Self {
        Self {
            frame_id: "physical-q".to_owned(),
            basis_to_physical: [[1, 0], [0, 1]],
        }
    }

    pub fn upper_shear(shear: i64) -> Self {
        Self {
            frame_id: format!("upper-shear-{shear}"),
            basis_to_physical: [[1, shear], [0, 1]],
        }
    }

    pub fn validate(
        &self,
        profile: EffectiveMetricProfileV1,
    ) -> Result<(), EffectiveMetricErrorV1> {
        if self.frame_id.trim().is_empty() {
            return Err(EffectiveMetricErrorV1::InvalidIdentifier);
        }
        if profile.maximum_basis_entry <= 0
            || self
                .basis_to_physical
                .iter()
                .flatten()
                .any(|value| value.unsigned_abs() > profile.maximum_basis_entry as u64)
        {
            return Err(EffectiveMetricErrorV1::BasisNotUnimodular);
        }
        let determinant = i128::from(self.basis_to_physical[0][0])
            .checked_mul(i128::from(self.basis_to_physical[1][1]))
            .and_then(|value| {
                value.checked_sub(
                    i128::from(self.basis_to_physical[0][1])
                        .checked_mul(i128::from(self.basis_to_physical[1][0]))?,
                )
            })
            .ok_or(EffectiveMetricErrorV1::IntegerArithmeticOverflow)?;
        if determinant == 0 {
            return Err(EffectiveMetricErrorV1::SingularBasis);
        }
        if determinant != 1 && determinant != -1 {
            return Err(EffectiveMetricErrorV1::BasisNotUnimodular);
        }
        Ok(())
    }

    pub fn basis(&self) -> Matrix2 {
        [
            [
                self.basis_to_physical[0][0] as f64,
                self.basis_to_physical[0][1] as f64,
            ],
            [
                self.basis_to_physical[1][0] as f64,
                self.basis_to_physical[1][1] as f64,
            ],
        ]
    }

    pub fn inverse_basis(&self) -> Matrix2 {
        let [[a, b], [c, d]] = self.basis_to_physical;
        let determinant = i128::from(a) * i128::from(d) - i128::from(b) * i128::from(c);
        let determinant = determinant as f64;
        [
            [d as f64 / determinant, -(b as f64) / determinant],
            [-(c as f64) / determinant, a as f64 / determinant],
        ]
    }
}

/// Discrete work/energy accounting for the implicit mass-damping-stiffness step.
///
/// The balance identity is:
/// ΔT + ΔV + h v₊ᵀ D v₊ + ½ Δvᵀ M Δv + ½ Δqᵀ K Δq = h fᵀ v₊.
///
/// The last two terms are the method's non-negative increment terms for SPD/PSD
/// M/K. They must be reported separately from physical damping dissipation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscreteEnergyLedgerV1 {
    pub kinetic_energy_before: f64,
    pub kinetic_energy_after: f64,
    pub potential_energy_before: f64,
    pub potential_energy_after: f64,
    pub external_work: f64,
    pub damping_dissipation: f64,
    pub mass_increment_dissipation: f64,
    pub stiffness_increment_dissipation: f64,
    pub balance_residual: f64,
    pub relative_balance_residual: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiscreteStepResultV1 {
    pub effective_metric: Matrix2,
    pub right_hand_side: Vector2,
    pub next_position: Vector2,
    pub next_velocity: Vector2,
    pub step_acceleration: Vector2,
    pub effective_metric_condition_inf: f64,
    pub backward_residual_inf: f64,
    pub discrete_work: f64,
    pub quadratic_potential_before: f64,
    pub quadratic_potential_after: f64,
    pub energy_ledger: DiscreteEnergyLedgerV1,
}

fn is_exactly_symmetric(matrix: &Matrix2) -> bool {
    matrix[0][1] == matrix[1][0]
}

fn is_positive_definite(matrix: &Matrix2, minimum_pivot: f64) -> bool {
    let a = matrix[0][0];
    let b = matrix[0][1];
    let c = matrix[1][0];
    let d = matrix[1][1];
    a.is_finite()
        && b.is_finite()
        && c.is_finite()
        && d.is_finite()
        && a > minimum_pivot
        && (d - c * b / a).is_finite()
        && d - c * b / a > minimum_pivot
}

fn is_positive_semidefinite(matrix: &Matrix2) -> bool {
    let a = matrix[0][0];
    let b = matrix[0][1];
    let c = matrix[1][0];
    let d = matrix[1][1];
    if [a, b, c, d].iter().any(|value| !value.is_finite()) {
        return false;
    }
    if a < 0.0 || d < 0.0 {
        return false;
    }
    let determinant = a * d - b * c;
    determinant.is_finite() && determinant >= 0.0
}

fn transpose(matrix: Matrix2) -> Matrix2 {
    [[matrix[0][0], matrix[1][0]], [matrix[0][1], matrix[1][1]]]
}

fn multiply(left: Matrix2, right: Matrix2) -> Matrix2 {
    let mut result = [[0.0; 2]; 2];
    for (row, output_row) in result.iter_mut().enumerate() {
        for (column, output) in output_row.iter_mut().enumerate() {
            *output = left[row][0] * right[0][column] + left[row][1] * right[1][column];
        }
    }
    result
}

fn apply(matrix: Matrix2, vector: Vector2) -> Vector2 {
    [
        matrix[0][0] * vector[0] + matrix[0][1] * vector[1],
        matrix[1][0] * vector[0] + matrix[1][1] * vector[1],
    ]
}

fn vector_inf(vector: Vector2) -> f64 {
    vector[0].abs().max(vector[1].abs())
}

fn matrix_inf(matrix: Matrix2) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn solve(
    matrix: Matrix2,
    rhs: Vector2,
    profile: EffectiveMetricProfileV1,
) -> Result<Vector2, EffectiveMetricErrorV1> {
    let mut augmented = [
        [matrix[0][0], matrix[0][1], rhs[0]],
        [matrix[1][0], matrix[1][1], rhs[1]],
    ];
    if matrix[0][0].abs() >= matrix[1][0].abs() {
        if matrix[0][0].abs() <= profile.minimum_pivot {
            return Err(EffectiveMetricErrorV1::SingularSolve);
        }
    } else {
        augmented.swap(0, 1);
        if augmented[0][0].abs() <= profile.minimum_pivot {
            return Err(EffectiveMetricErrorV1::SingularSolve);
        }
    }
    let factor = augmented[1][0] / augmented[0][0];
    augmented[1][1] -= factor * augmented[0][1];
    augmented[1][2] -= factor * augmented[0][2];
    if augmented[1][1].abs() <= profile.minimum_pivot || !augmented[1][1].is_finite() {
        return Err(EffectiveMetricErrorV1::SingularSolve);
    }
    let second = augmented[1][2] / augmented[1][1];
    let first = (augmented[0][2] - augmented[0][1] * second) / augmented[0][0];
    if !first.is_finite() || !second.is_finite() {
        return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
    }
    Ok([first, second])
}

fn inverse(
    matrix: Matrix2,
    profile: EffectiveMetricProfileV1,
) -> Result<Matrix2, EffectiveMetricErrorV1> {
    let first = solve(matrix, [1.0, 0.0], profile)?;
    let second = solve(matrix, [0.0, 1.0], profile)?;
    Ok([[first[0], second[0]], [first[1], second[1]]])
}

fn condition_inf(
    matrix: Matrix2,
    profile: EffectiveMetricProfileV1,
) -> Result<f64, EffectiveMetricErrorV1> {
    let result = matrix_inf(matrix) * matrix_inf(inverse(matrix, profile)?);
    if !result.is_finite() {
        return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
    }
    Ok(result)
}

fn quadratic(matrix: Matrix2, vector: Vector2) -> f64 {
    let mapped = apply(matrix, vector);
    vector[0] * mapped[0] + vector[1] * mapped[1]
}

fn build_energy_ledger(
    problem: &LinearDiscreteProblem2V1,
    next_position: Vector2,
    next_velocity: Vector2,
) -> Result<DiscreteEnergyLedgerV1, EffectiveMetricErrorV1> {
    let h = problem.time_step;
    let delta_position = [
        next_position[0] - problem.position[0],
        next_position[1] - problem.position[1],
    ];
    let delta_velocity = [
        next_velocity[0] - problem.velocity[0],
        next_velocity[1] - problem.velocity[1],
    ];

    let kinetic_energy_before = 0.5 * quadratic(problem.mass, problem.velocity);
    let kinetic_energy_after = 0.5 * quadratic(problem.mass, next_velocity);
    let potential_energy_before = 0.5 * quadratic(problem.stiffness, problem.position);
    let potential_energy_after = 0.5 * quadratic(problem.stiffness, next_position);
    let external_work = h
        * (problem.force_covector[0] * next_velocity[0]
            + problem.force_covector[1] * next_velocity[1]);
    let damping_dissipation = h * quadratic(problem.damping, next_velocity);
    let mass_increment_dissipation = 0.5 * quadratic(problem.mass, delta_velocity);
    let stiffness_increment_dissipation = 0.5 * quadratic(problem.stiffness, delta_position);

    let balance_residual = (kinetic_energy_after - kinetic_energy_before)
        + (potential_energy_after - potential_energy_before)
        + damping_dissipation
        + mass_increment_dissipation
        + stiffness_increment_dissipation
        - external_work;
    let balance_scale = (kinetic_energy_after - kinetic_energy_before).abs()
        + (potential_energy_after - potential_energy_before).abs()
        + damping_dissipation.abs()
        + mass_increment_dissipation.abs()
        + stiffness_increment_dissipation.abs()
        + external_work.abs();
    let relative_balance_residual =
        balance_residual.abs() / balance_scale.max(f64::MIN_POSITIVE);

    let values = [
        kinetic_energy_before,
        kinetic_energy_after,
        potential_energy_before,
        potential_energy_after,
        external_work,
        damping_dissipation,
        mass_increment_dissipation,
        stiffness_increment_dissipation,
        balance_residual,
        relative_balance_residual,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
    }

    Ok(DiscreteEnergyLedgerV1 {
        kinetic_energy_before,
        kinetic_energy_after,
        potential_energy_before,
        potential_energy_after,
        external_work,
        damping_dissipation,
        mass_increment_dissipation,
        stiffness_increment_dissipation,
        balance_residual,
        relative_balance_residual,
    })
}

fn congruence(matrix: Matrix2, basis: Matrix2) -> Matrix2 {
    multiply(multiply(transpose(basis), matrix), basis)
}

fn transform_vector(inverse_basis: Matrix2, vector: Vector2) -> Vector2 {
    apply(inverse_basis, vector)
}

fn transform_covector(basis: Matrix2, covector: Vector2) -> Vector2 {
    apply(transpose(basis), covector)
}

pub fn rebase_problem(
    physical: &LinearDiscreteProblem2V1,
    frame: &IntegerFrame2V1,
    profile: EffectiveMetricProfileV1,
) -> Result<LinearDiscreteProblem2V1, EffectiveMetricErrorV1> {
    physical.validate(profile)?;
    frame.validate(profile)?;
    let basis = frame.basis();
    let inverse_basis = frame.inverse_basis();
    let rebased = LinearDiscreteProblem2V1 {
        case_id: format!("{}@{}", physical.case_id, frame.frame_id),
        time_step: physical.time_step,
        mass: congruence(physical.mass, basis),
        damping: congruence(physical.damping, basis),
        stiffness: congruence(physical.stiffness, basis),
        position: transform_vector(inverse_basis, physical.position),
        velocity: transform_vector(inverse_basis, physical.velocity),
        force_covector: transform_covector(basis, physical.force_covector),
    };
    rebased.validate(profile)?;
    Ok(rebased)
}

pub fn solve_discrete_step(
    problem: &LinearDiscreteProblem2V1,
    profile: EffectiveMetricProfileV1,
) -> Result<DiscreteStepResultV1, EffectiveMetricErrorV1> {
    problem.validate(profile)?;
    let h = problem.time_step;
    let effective_metric = problem.effective_metric()?;

    let mass_velocity = apply(problem.mass, problem.velocity);
    let stiffness_position = apply(problem.stiffness, problem.position);
    let right_hand_side = [
        mass_velocity[0] - h * stiffness_position[0] + h * problem.force_covector[0],
        mass_velocity[1] - h * stiffness_position[1] + h * problem.force_covector[1],
    ];
    if right_hand_side.iter().any(|value| !value.is_finite()) {
        return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
    }

    let next_velocity = solve(effective_metric, right_hand_side, profile)?;
    let next_position = [
        problem.position[0] + h * next_velocity[0],
        problem.position[1] + h * next_velocity[1],
    ];
    let step_acceleration = [
        (next_velocity[0] - problem.velocity[0]) / h,
        (next_velocity[1] - problem.velocity[1]) / h,
    ];
    if next_position
        .iter()
        .chain(step_acceleration.iter())
        .any(|value| !value.is_finite())
    {
        return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
    }

    let effective_metric_condition_inf = condition_inf(effective_metric, profile)?;
    if effective_metric_condition_inf > profile.maximum_effective_condition_inf {
        return Err(EffectiveMetricErrorV1::EffectiveMetricOutsideConditionEnvelope);
    }

    let reconstructed = apply(effective_metric, next_velocity);
    let residual = [
        reconstructed[0] - right_hand_side[0],
        reconstructed[1] - right_hand_side[1],
    ];
    let backward_scale =
        matrix_inf(effective_metric) * vector_inf(next_velocity) + vector_inf(right_hand_side);
    if residual.iter().any(|value| !value.is_finite()) || !backward_scale.is_finite() {
        return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
    }
    let backward_residual_inf = vector_inf(residual) / backward_scale.max(f64::MIN_POSITIVE);
    let energy_ledger = build_energy_ledger(problem, next_position, next_velocity)?;
    let discrete_work = energy_ledger.external_work;
    let quadratic_potential_before = energy_ledger.potential_energy_before;
    let quadratic_potential_after = energy_ledger.potential_energy_after;
    if !backward_residual_inf.is_finite() {
        return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
    }

    Ok(DiscreteStepResultV1 {
        effective_metric,
        right_hand_side,
        next_position,
        next_velocity,
        step_acceleration,
        effective_metric_condition_inf,
        backward_residual_inf,
        discrete_work,
        quadratic_potential_before,
        quadratic_potential_after,
        energy_ledger,
    })
}

fn metric_norm(metric: Matrix2, vector: Vector2) -> Result<f64, EffectiveMetricErrorV1> {
    let value = quadratic(metric, vector);
    if !value.is_finite() || value < -1.0e-12 {
        return Err(EffectiveMetricErrorV1::ArithmeticOverflow);
    }
    Ok(value.max(0.0).sqrt())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CovarianceComparisonV1 {
    pub lifted_position_error_inf: f64,
    pub lifted_velocity_error_inf: f64,
    pub lifted_acceleration_error_inf: f64,
    pub position_mass_metric_error: f64,
    pub velocity_mass_metric_error: f64,
    pub canonical_effective_condition_inf: f64,
    pub rebased_effective_condition_inf: f64,
    pub canonical_backward_residual_inf: f64,
    pub rebased_backward_residual_inf: f64,
}

pub fn compare_dense_rebasis(
    physical: &LinearDiscreteProblem2V1,
    frame: &IntegerFrame2V1,
    profile: EffectiveMetricProfileV1,
) -> Result<CovarianceComparisonV1, EffectiveMetricErrorV1> {
    let canonical = solve_discrete_step(physical, profile)?;
    let rebased_problem = rebase_problem(physical, frame, profile)?;
    let rebased = solve_discrete_step(&rebased_problem, profile)?;
    let basis = frame.basis();

    let lifted_position = apply(basis, rebased.next_position);
    let lifted_velocity = apply(basis, rebased.next_velocity);
    let lifted_acceleration = apply(basis, rebased.step_acceleration);

    let position_difference = [
        canonical.next_position[0] - lifted_position[0],
        canonical.next_position[1] - lifted_position[1],
    ];
    let velocity_difference = [
        canonical.next_velocity[0] - lifted_velocity[0],
        canonical.next_velocity[1] - lifted_velocity[1],
    ];
    let acceleration_difference = [
        canonical.step_acceleration[0] - lifted_acceleration[0],
        canonical.step_acceleration[1] - lifted_acceleration[1],
    ];

    let position_mass_metric_error = metric_norm(physical.mass, position_difference)?;
    let velocity_mass_metric_error = metric_norm(physical.mass, velocity_difference)?;

    Ok(CovarianceComparisonV1 {
        lifted_position_error_inf: vector_inf(position_difference),
        lifted_velocity_error_inf: vector_inf(velocity_difference),
        lifted_acceleration_error_inf: vector_inf(acceleration_difference),
        position_mass_metric_error,
        velocity_mass_metric_error,
        canonical_effective_condition_inf: canonical.effective_metric_condition_inf,
        rebased_effective_condition_inf: rebased.effective_metric_condition_inf,
        canonical_backward_residual_inf: canonical.backward_residual_inf,
        rebased_backward_residual_inf: rebased.backward_residual_inf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(time_step: f64) -> LinearDiscreteProblem2V1 {
        LinearDiscreteProblem2V1 {
            case_id: format!("linear-mass-damping-stiffness-h-{time_step}"),
            time_step,
            mass: [[2.0, 0.2], [0.2, 1.5]],
            damping: [[0.4, 0.1], [0.1, 0.3]],
            stiffness: [[4.0, -1.0], [-1.0, 2.0]],
            position: [0.3, -0.2],
            velocity: [0.5, 0.1],
            force_covector: [1.0, -0.3],
        }
    }

    #[test]
    fn effective_metric_is_exact_declared_m_plus_h_d_plus_h_squared_k() {
        let problem = fixture(0.5);
        let expected = [
            [2.0 + 0.5 * 0.4 + 0.25 * 4.0, 0.2 + 0.5 * 0.1 - 0.25],
            [0.2 + 0.5 * 0.1 - 0.25, 1.5 + 0.5 * 0.3 + 0.25 * 2.0],
        ];
        let actual = problem.effective_metric().unwrap();
        for (actual_row, expected_row) in actual.iter().zip(expected.iter()) {
            for (actual_value, expected_value) in actual_row.iter().zip(expected_row.iter()) {
                assert!((actual_value - expected_value).abs() < 1.0e-14);
            }
        }
    }

    #[test]
    fn noncommuting_mass_damping_and_stiffness_are_present_in_fixture() {
        let problem = fixture(0.2);
        let mass_damping = multiply(problem.mass, problem.damping);
        let damping_mass = multiply(problem.damping, problem.mass);
        let mass_stiffness = multiply(problem.mass, problem.stiffness);
        let stiffness_mass = multiply(problem.stiffness, problem.mass);
        let commutator_md = [
            [
                mass_damping[0][0] - damping_mass[0][0],
                mass_damping[0][1] - damping_mass[0][1],
            ],
            [
                mass_damping[1][0] - damping_mass[1][0],
                mass_damping[1][1] - damping_mass[1][1],
            ],
        ];
        let commutator_mk = [
            [
                mass_stiffness[0][0] - stiffness_mass[0][0],
                mass_stiffness[0][1] - stiffness_mass[0][1],
            ],
            [
                mass_stiffness[1][0] - stiffness_mass[1][0],
                mass_stiffness[1][1] - stiffness_mass[1][1],
            ],
        ];
        assert!(matrix_inf(commutator_md) > 0.0);
        assert!(matrix_inf(commutator_mk) > 0.0);
    }

    #[test]
    fn effective_metric_itself_transforms_by_congruence() {
        let profile = EffectiveMetricProfileV1::default();
        let physical = fixture(0.35);
        let frame = IntegerFrame2V1::upper_shear(5);
        let rebased = rebase_problem(&physical, &frame, profile).unwrap();
        let expected = congruence(physical.effective_metric().unwrap(), frame.basis());
        let actual = rebased.effective_metric().unwrap();

        for (actual_row, expected_row) in actual.iter().zip(expected.iter()) {
            for (actual_value, expected_value) in actual_row.iter().zip(expected_row.iter()) {
                assert!((actual_value - expected_value).abs() < 1.0e-12);
            }
        }
    }

    #[test]
    fn solved_map_satisfies_original_implicit_damped_stiff_equation() {
        let problem = fixture(0.2);
        let result = solve_discrete_step(&problem, EffectiveMetricProfileV1::default()).unwrap();
        let acceleration = result.step_acceleration;
        let damped_velocity = apply(problem.damping, result.next_velocity);
        let stiff_position = apply(problem.stiffness, result.next_position);
        let inertial = apply(problem.mass, acceleration);
        let balance = [
            inertial[0] + damped_velocity[0] + stiff_position[0] - problem.force_covector[0],
            inertial[1] + damped_velocity[1] + stiff_position[1] - problem.force_covector[1],
        ];
        assert!(vector_inf(balance) < 1.0e-12);
    }

    #[test]
    fn exact_dyadic_diagonal_fixture_has_closed_form_step() {
        let problem = LinearDiscreteProblem2V1 {
            case_id: "exact-dyadic-effective-metric".to_owned(),
            time_step: 0.5,
            mass: [[2.0, 0.0], [0.0, 2.0]],
            damping: [[0.0, 0.0], [0.0, 0.0]],
            stiffness: [[8.0, 0.0], [0.0, 0.0]],
            position: [0.0, 0.0],
            velocity: [0.0, 0.0],
            force_covector: [1.0, 0.0],
        };
        let result = solve_discrete_step(&problem, EffectiveMetricProfileV1::default()).unwrap();
        assert_eq!(result.effective_metric[0][0], 4.0);
        assert_eq!(result.next_velocity[0], 0.125);
        assert_eq!(result.next_position[0], 0.0625);
        assert_eq!(result.step_acceleration[0], 0.25);
    }

    #[test]
    fn seventeen_value_dense_shear_ladder_preserves_the_complete_discrete_map() {
        let profile = EffectiveMetricProfileV1::default();
        for time_step in [0.01, 0.05, 0.2, 0.5, 1.0] {
            let problem = fixture(time_step);
            for shear in -8_i64..=8 {
                let frame = IntegerFrame2V1::upper_shear(shear);
                let comparison = compare_dense_rebasis(&problem, &frame, profile).unwrap();
                assert!(comparison.lifted_position_error_inf < 2.0e-12);
                assert!(comparison.lifted_velocity_error_inf < 2.0e-12);
                assert!(comparison.lifted_acceleration_error_inf < 2.0e-12);
                assert!(comparison.position_mass_metric_error < 2.0e-12);
                assert!(comparison.velocity_mass_metric_error < 2.0e-12);
                assert!(comparison.canonical_backward_residual_inf < 1.0e-13);
                assert!(comparison.rebased_backward_residual_inf < 1.0e-11);
            }
        }
    }

    #[test]
    fn nonsymmetric_damping_is_rejected_before_effective_metric_formation() {
        let mut problem = fixture(0.2);
        problem.damping[0][1] += 0.05;
        assert_eq!(
            problem.validate(EffectiveMetricProfileV1::default()),
            Err(EffectiveMetricErrorV1::MatrixNotSymmetric)
        );
    }

    #[test]
    fn effective_condition_profile_abstains_on_excessive_operator_condition() {
        let profile = EffectiveMetricProfileV1 {
            maximum_effective_condition_inf: 1.1,
            ..EffectiveMetricProfileV1::default()
        };
        assert_eq!(
            solve_discrete_step(&fixture(0.2), profile),
            Err(EffectiveMetricErrorV1::EffectiveMetricOutsideConditionEnvelope)
        );
    }

    #[test]
    fn wrong_force_variance_is_detected_by_dense_rebasis_ladder() {
        let profile = EffectiveMetricProfileV1::default();
        let physical = fixture(0.2);
        let frame = IntegerFrame2V1::upper_shear(4);
        let mut wrong = rebase_problem(&physical, &frame, profile).unwrap();
        wrong.force_covector = transform_vector(frame.inverse_basis(), physical.force_covector);
        let canonical = solve_discrete_step(&physical, profile).unwrap();
        let wrong_result = solve_discrete_step(&wrong, profile).unwrap();
        let lifted = apply(frame.basis(), wrong_result.next_velocity);
        assert!(
            vector_inf([
                canonical.next_velocity[0] - lifted[0],
                canonical.next_velocity[1] - lifted[1]
            ]) > 1.0e-5
        );
    }

    #[test]
    fn leaving_damping_or_stiffness_in_the_old_frame_breaks_covariance() {
        let profile = EffectiveMetricProfileV1::default();
        let physical = fixture(0.4);
        let frame = IntegerFrame2V1::upper_shear(4);
        let correct_problem = rebase_problem(&physical, &frame, profile).unwrap();
        let canonical = solve_discrete_step(&physical, profile).unwrap();

        let mut wrong_damping = correct_problem.clone();
        wrong_damping.damping = physical.damping;
        let wrong_damping_result = solve_discrete_step(&wrong_damping, profile).unwrap();
        let lifted_damping_velocity = apply(frame.basis(), wrong_damping_result.next_velocity);
        assert!(
            vector_inf([
                canonical.next_velocity[0] - lifted_damping_velocity[0],
                canonical.next_velocity[1] - lifted_damping_velocity[1]
            ]) > 1.0e-5
        );

        let mut wrong_stiffness = correct_problem;
        wrong_stiffness.stiffness = physical.stiffness;
        let wrong_stiffness_result = solve_discrete_step(&wrong_stiffness, profile).unwrap();
        let lifted_stiffness_velocity = apply(frame.basis(), wrong_stiffness_result.next_velocity);
        assert!(
            vector_inf([
                canonical.next_velocity[0] - lifted_stiffness_velocity[0],
                canonical.next_velocity[1] - lifted_stiffness_velocity[1]
            ]) > 1.0e-5
        );
    }

    #[test]
    fn wrong_effective_metric_exponent_is_detected() {
        let problem = fixture(0.5);
        let correct = problem.effective_metric().unwrap();
        let mut wrong = problem.mass;
        for (row, wrong_row) in wrong.iter_mut().enumerate() {
            for (column, value) in wrong_row.iter_mut().enumerate() {
                *value += 0.5 * problem.damping[row][column] + 0.5 * problem.stiffness[row][column];
            }
        }
        assert!(
            matrix_inf([
                [correct[0][0] - wrong[0][0], correct[0][1] - wrong[0][1]],
                [correct[1][0] - wrong[1][0], correct[1][1] - wrong[1][1]]
            ]) > 0.1
        );
    }

    #[test]
    fn slightly_negative_damping_diagonal_is_not_accepted_as_psd() {
        let mut problem = fixture(0.1);
        problem.damping[0][0] = -1.0e-14;
        assert_eq!(
            problem.validate(EffectiveMetricProfileV1::default()),
            Err(EffectiveMetricErrorV1::DampingNotPositiveSemidefinite)
        );
    }

    #[test]
    fn indefinite_stiffness_and_nonunimodular_basis_fail_closed() {
        let mut problem = fixture(1.0);
        problem.stiffness = [[-100.0, 0.0], [0.0, -100.0]];
        assert_eq!(
            problem.validate(EffectiveMetricProfileV1::default()),
            Err(EffectiveMetricErrorV1::StiffnessNotPositiveSemidefinite)
        );
        let frame = IntegerFrame2V1 {
            frame_id: "non-unimodular".to_owned(),
            basis_to_physical: [[2, 0], [0, 1]],
        };
        assert_eq!(
            frame.validate(EffectiveMetricProfileV1::default()),
            Err(EffectiveMetricErrorV1::BasisNotUnimodular)
        );
    }

    #[test]
    fn ill_conditioned_effective_metric_is_reported_separately() {
        let mut problem = fixture(1.0);
        problem.stiffness = [[1.0e6, 0.0], [0.0, 1.0]];
        let result = solve_discrete_step(&problem, EffectiveMetricProfileV1::default()).unwrap();
        assert!(result.effective_metric_condition_inf > 1.0e5);
    }

    #[test]
    fn discrete_energy_work_ledger_closes_the_implicit_equation() {
        let result =
            solve_discrete_step(&fixture(0.4), EffectiveMetricProfileV1::default()).unwrap();
        let ledger = result.energy_ledger;
        assert!(ledger.relative_balance_residual < 2.0e-13);
        assert!(ledger.balance_residual.abs() < 2.0e-13);
        assert_eq!(ledger.external_work, result.discrete_work);
        assert_eq!(ledger.potential_energy_before, result.quadratic_potential_before);
        assert_eq!(ledger.potential_energy_after, result.quadratic_potential_after);
        assert!(ledger.damping_dissipation >= 0.0);
        assert!(ledger.mass_increment_dissipation >= 0.0);
        assert!(ledger.stiffness_increment_dissipation >= 0.0);
    }

    #[test]
    fn unforced_damped_step_does_not_increase_mechanical_energy() {
        let mut problem = fixture(0.8);
        problem.force_covector = [0.0, 0.0];
        let result = solve_discrete_step(&problem, EffectiveMetricProfileV1::default()).unwrap();
        let ledger = result.energy_ledger;
        let energy_before = ledger.kinetic_energy_before + ledger.potential_energy_before;
        let energy_after = ledger.kinetic_energy_after + ledger.potential_energy_after;
        assert!(energy_after <= energy_before + 1.0e-13);
        assert!(ledger.damping_dissipation >= 0.0);
        assert!(ledger.mass_increment_dissipation >= 0.0);
        assert!(ledger.stiffness_increment_dissipation >= 0.0);
        assert!(ledger.relative_balance_residual < 2.0e-13);
    }

    #[test]
    fn energy_work_ledger_is_invariant_under_dense_coordinate_rebasing() {
        let profile = EffectiveMetricProfileV1::default();
        let physical = fixture(0.35);
        let frame = IntegerFrame2V1::upper_shear(6);
        let rebased_problem = rebase_problem(&physical, &frame, profile).unwrap();
        let canonical = solve_discrete_step(&physical, profile).unwrap().energy_ledger;
        let rebased = solve_discrete_step(&rebased_problem, profile).unwrap().energy_ledger;

        let canonical_values = [
            canonical.kinetic_energy_before,
            canonical.kinetic_energy_after,
            canonical.potential_energy_before,
            canonical.potential_energy_after,
            canonical.external_work,
            canonical.damping_dissipation,
            canonical.mass_increment_dissipation,
            canonical.stiffness_increment_dissipation,
            canonical.balance_residual,
        ];
        let rebased_values = [
            rebased.kinetic_energy_before,
            rebased.kinetic_energy_after,
            rebased.potential_energy_before,
            rebased.potential_energy_after,
            rebased.external_work,
            rebased.damping_dissipation,
            rebased.mass_increment_dissipation,
            rebased.stiffness_increment_dissipation,
            rebased.balance_residual,
        ];
        for (left, right) in canonical_values.iter().zip(rebased_values.iter()) {
            assert!((left - right).abs() <= 2.0e-11 * left.abs().max(right.abs()).max(1.0));
        }
    }

    #[test]
    fn backward_euler_large_step_is_stable_but_not_trajectory_accurate() {
        let time_step = 1.0;
        let angular_frequency = 10.0;
        let problem = LinearDiscreteProblem2V1 {
            case_id: "large-step-stable-not-accurate".to_owned(),
            time_step,
            mass: [[1.0, 0.0], [0.0, 1.0]],
            damping: [[0.0, 0.0], [0.0, 0.0]],
            stiffness: [[100.0, 0.0], [0.0, 100.0]],
            position: [1.0, 0.0],
            velocity: [0.0, 0.0],
            force_covector: [0.0, 0.0],
        };
        let result = solve_discrete_step(&problem, EffectiveMetricProfileV1::default()).unwrap();
        let spectral_radius =
            1.0 / (1.0 + time_step * time_step * angular_frequency * angular_frequency).sqrt();
        let exact_position = (angular_frequency * time_step).cos();

        assert!(spectral_radius < 1.0);
        assert!(result.next_position[0].is_finite());
        assert!((result.next_position[0] - exact_position).abs() > 0.5);
        assert!(result.energy_ledger.relative_balance_residual < 2.0e-13);
    }

    #[test]
    fn mass_damping_stiffness_strata_are_distinguishable() {
        let mut mass_only = fixture(0.25);
        mass_only.damping = [[0.0; 2]; 2];
        mass_only.stiffness = [[0.0; 2]; 2];

        let mut damping_only = fixture(0.25);
        damping_only.stiffness = [[0.0; 2]; 2];

        let mut stiffness_only = fixture(0.25);
        stiffness_only.damping = [[0.0; 2]; 2];

        let mass = mass_only.effective_metric().unwrap();
        let damping = damping_only.effective_metric().unwrap();
        let stiffness = stiffness_only.effective_metric().unwrap();
        assert_ne!(mass, damping);
        assert_ne!(mass, stiffness);
        assert_ne!(damping, stiffness);
    }
}
