// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Standalone effective-metric discrete constrained-step reference.
//!
//! This crate implements a small, deterministic, frictionless unilateral-contact
//! step for research and differential testing. It is deliberately not wired
//! into the production solver.
//!
//! The backward-Euler linearized force model is:
//!
//! M (v_next - v) = h (f - D v_next - K (q_error + h v_next)) + J^T p
//!
//! Rearranging gives:
//!
//! M_hat v_next = M v + h f - h K q_error + J^T p
//! M_hat = M + h D + h^2 K
//!
//! Contacts use the linearized next-step inequality:
//!
//! J v_next >= -gap / h,   p >= 0,   p dot (J v_next + gap/h) = 0.
//!
//! Thus the same effective metric determines both the unconstrained update and
//! the contact Delassus operator. The active-set solver exhaustively enumerates
//! small contact sets, making it a transparent reference rather than a
//! performance implementation.
//!
//! Boundaries: no friction, restitution, CCD, rotating contact Jacobians,
//! nonlinear geometry update, warm starting, or production physics semantics.

use std::fmt;

const MAX_DOF: usize = 12;
const MAX_CONTACTS: usize = 10;
const SOLVE_TOLERANCE: f64 = 1.0e-10;

/// A compact row-major dense matrix used by the standalone reference.
#[derive(Clone, Debug, PartialEq)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    values: Vec<f64>,
}

impl Matrix {
    pub fn new(rows: usize, cols: usize, values: Vec<f64>) -> Result<Self, StepError> {
        if rows.checked_mul(cols) != Some(values.len()) {
            return Err(StepError::InvalidMatrixShape);
        }
        if !values.iter().all(|value| value.is_finite()) {
            return Err(StepError::NonFiniteInput);
        }
        Ok(Self { rows, cols, values })
    }

    pub fn from_rows(rows: &[&[f64]]) -> Result<Self, StepError> {
        if rows.is_empty() {
            return Self::new(0, 0, Vec::new());
        }
        let cols = rows[0].len();
        if rows.iter().any(|row| row.len() != cols) {
            return Err(StepError::InvalidMatrixShape);
        }
        Self::new(
            rows.len(),
            cols,
            rows.iter().flat_map(|row| row.iter().copied()).collect(),
        )
    }

    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            values: vec![0.0; rows * cols],
        }
    }

    pub fn identity(size: usize) -> Self {
        let mut matrix = Self::zeros(size, size);
        for index in 0..size {
            matrix[(index, index)] = 1.0;
        }
        matrix
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn as_slice(&self) -> &[f64] {
        &self.values
    }

    pub fn max_abs(&self) -> f64 {
        self.values
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
    }
}

impl std::ops::Index<(usize, usize)> for Matrix {
    type Output = f64;

    fn index(&self, (row, col): (usize, usize)) -> &Self::Output {
        &self.values[row * self.cols + col]
    }
}

impl std::ops::IndexMut<(usize, usize)> for Matrix {
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut Self::Output {
        &mut self.values[row * self.cols + col]
    }
}

/// Typed failures from the reference discrete step.
#[derive(Clone, Debug, PartialEq)]
pub enum StepError {
    InvalidMatrixShape,
    InvalidVectorShape,
    InvalidStepSize,
    NonFiniteInput,
    MatrixNotSymmetric { name: &'static str, residual: f64 },
    EffectiveMetricNotPositiveDefinite,
    ProblemTooLarge { dof: usize, contacts: usize },
    NoComplementarySolution,
}

impl fmt::Display for StepError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMatrixShape => write!(f, "matrix has inconsistent dimensions"),
            Self::InvalidVectorShape => {
                write!(f, "vector or contact data has inconsistent dimensions")
            }
            Self::InvalidStepSize => write!(f, "step size must be finite and strictly positive"),
            Self::NonFiniteInput => write!(f, "input contains a non-finite value"),
            Self::MatrixNotSymmetric { name, residual } => {
                write!(f, "{name} is not symmetric (maximum residual {residual:e})")
            }
            Self::EffectiveMetricNotPositiveDefinite => {
                write!(f, "effective metric is not positive definite")
            }
            Self::ProblemTooLarge { dof, contacts } => {
                write!(
                    f,
                    "reference limit exceeded: dof={dof}, contacts={contacts}"
                )
            }
            Self::NoComplementarySolution => {
                write!(f, "no feasible complementary active set was found")
            }
        }
    }
}

impl std::error::Error for StepError {}

/// Inputs for one linearized backward-Euler constrained step.
#[derive(Clone, Debug)]
pub struct DiscreteStepInput {
    /// Symmetric positive-definite mass matrix.
    pub mass: Matrix,
    /// Symmetric damping matrix D.
    pub damping: Matrix,
    /// Symmetric position-stiffness matrix K.
    pub stiffness: Matrix,
    /// Position error q - q_reference, in generalized coordinates.
    pub position_error: Vec<f64>,
    /// Current generalized velocity.
    pub velocity: Vec<f64>,
    /// External generalized force, held constant over the step.
    pub external_force: Vec<f64>,
    /// Contact Jacobian; rows are unilateral normal constraints.
    pub contact_jacobian: Matrix,
    /// Current signed gaps; positive means separated, zero means touching.
    pub contact_gap: Vec<f64>,
    /// Step size h.
    pub dt: f64,
}

/// One-step solution plus independently checkable residuals.
#[derive(Clone, Debug, PartialEq)]
pub struct DiscreteStepResult {
    pub effective_metric: Matrix,
    pub unconstrained_velocity: Vec<f64>,
    pub next_velocity: Vec<f64>,
    pub next_position_error: Vec<f64>,
    pub discrete_acceleration: Vec<f64>,
    pub contact_impulse: Vec<f64>,
    /// Positive slack means the linearized next-step gap is nonnegative.
    pub contact_slack: Vec<f64>,
    pub active_contacts: Vec<bool>,
    /// Absolute infinity-norm residual of the effective-metric equation.
    pub max_metric_equation_residual: f64,
    /// Dimensionless equation residual normalized per generalized coordinate.
    pub max_scaled_metric_equation_residual: f64,
    pub max_negative_impulse: f64,
    pub max_negative_contact_slack: f64,
    pub max_complementarity_residual: f64,
    /// Dimensionless dual-feasibility residual, normalized by impulse scale.
    pub max_scaled_dual_feasibility_residual: f64,
    /// Dimensionless primal-feasibility residual, normalized by row velocity scale.
    pub max_scaled_primal_feasibility_residual: f64,
    /// Dimensionless complementarity residual, normalized per active row.
    pub max_scaled_complementarity_residual: f64,
}

/// Evaluate the effective-metric step and solve its small frictionless contact LCP.
pub fn solve_discrete_step(input: &DiscreteStepInput) -> Result<DiscreteStepResult, StepError> {
    validate_input(input)?;
    let n = input.mass.rows;
    let m = input.contact_jacobian.rows;

    let mut metric = input.mass.clone();
    for row in 0..n {
        for col in 0..n {
            metric[(row, col)] += input.dt * input.damping[(row, col)]
                + input.dt * input.dt * input.stiffness[(row, col)];
        }
    }

    let mv = mat_vec(&input.mass, &input.velocity);
    let kq = mat_vec(&input.stiffness, &input.position_error);
    let mut rhs = vec![0.0; n];
    for index in 0..n {
        rhs[index] = mv[index] + input.dt * input.external_force[index] - input.dt * kq[index];
    }

    let unconstrained =
        solve_spd(&metric, &rhs).ok_or(StepError::EffectiveMetricNotPositiveDefinite)?;

    // Compute M_hat^-1 J^T once. Both the constrained velocity correction and
    // the Delassus/contact matrix use this exact same metric.
    let mut metric_inverse_jt = Matrix::zeros(n, m);
    for contact in 0..m {
        let row = (0..n)
            .map(|dof| input.contact_jacobian[(contact, dof)])
            .collect::<Vec<_>>();
        let column =
            solve_spd(&metric, &row).ok_or(StepError::EffectiveMetricNotPositiveDefinite)?;
        for dof in 0..n {
            metric_inverse_jt[(dof, contact)] = column[dof];
        }
    }
    let delassus = multiply(&input.contact_jacobian, &metric_inverse_jt);

    let target_velocity = input
        .contact_gap
        .iter()
        .map(|gap| -gap / input.dt)
        .collect::<Vec<_>>();

    let free_normal_velocity = mat_vec(&input.contact_jacobian, &unconstrained);
    let contact_rhs = target_velocity
        .iter()
        .zip(&free_normal_velocity)
        .map(|(target, free)| target - free)
        .collect::<Vec<_>>();

    let mut selected: Option<(Vec<f64>, Vec<f64>, Vec<f64>, Vec<bool>, f64)> = None;
    let subset_count = 1_u64 << m;

    // Enumeration is deterministic: masks are visited in ascending order.
    // Redundant rows can be left inactive when another independent active set
    // already enforces the same boundary.
    for mask in 0..subset_count {
        let active_indices = (0..m)
            .filter(|index| (mask & (1_u64 << index)) != 0)
            .collect::<Vec<_>>();

        let mut impulse = vec![0.0; m];
        if !active_indices.is_empty() {
            let active_count = active_indices.len();
            let mut active_matrix = Matrix::zeros(active_count, active_count);
            let mut active_rhs = vec![0.0; active_count];

            for (active_row, &contact_row) in active_indices.iter().enumerate() {
                active_rhs[active_row] = contact_rhs[contact_row];
                for (active_col, &contact_col) in active_indices.iter().enumerate() {
                    active_matrix[(active_row, active_col)] = delassus[(contact_row, contact_col)];
                }
            }

            let Some(active_impulse) = solve_spd(&active_matrix, &active_rhs) else {
                continue;
            };
            let active_dual_residual =
                relative_negative_residual(&active_impulse, &active_matrix, &active_rhs);
            if active_dual_residual > SOLVE_TOLERANCE {
                continue;
            }
            for (active_index, &contact_index) in active_indices.iter().enumerate() {
                impulse[contact_index] = active_impulse[active_index].max(0.0);
            }
        }

        let mut next_velocity = unconstrained.clone();
        for dof in 0..n {
            for contact in 0..m {
                next_velocity[dof] += metric_inverse_jt[(dof, contact)] * impulse[contact];
            }
        }
        let normal_velocity = mat_vec(&input.contact_jacobian, &next_velocity);
        let slack = normal_velocity
            .iter()
            .zip(&target_velocity)
            .map(|(velocity, target)| velocity - target)
            .collect::<Vec<_>>();

        let primal_residual =
            relative_primal_feasibility(&normal_velocity, &target_velocity, &slack);
        let dual_residual = relative_negative_residual(&impulse, &delassus, &contact_rhs);
        if dual_residual > SOLVE_TOLERANCE || primal_residual > SOLVE_TOLERANCE {
            continue;
        }

        let complementarity =
            relative_complementarity_residual(&impulse, &normal_velocity, &target_velocity, &slack);
        if complementarity > 1.0e-8 {
            continue;
        }

        let scaled_metric_residual = metric_equation_relative_residual(
            &metric,
            &next_velocity,
            &rhs,
            &input.contact_jacobian,
            &impulse,
        );
        if scaled_metric_residual > 1.0e-8 {
            continue;
        }
        let residual = metric_equation_residual(
            &metric,
            &next_velocity,
            &rhs,
            &input.contact_jacobian,
            &impulse,
        );
        // Candidate impulses have already been checked for negativity and
        // clamped at zero. Do not apply an absolute impulse threshold here:
        // impulse units and magnitude change under a uniform mass rescaling.
        let active = impulse.iter().map(|value| *value > 0.0).collect::<Vec<_>>();
        selected = Some((next_velocity, impulse, slack, active, residual));
        break;
    }

    let (next_velocity, contact_impulse, contact_slack, active_contacts, metric_residual) =
        selected.ok_or(StepError::NoComplementarySolution)?;

    let next_position_error = input
        .position_error
        .iter()
        .zip(&next_velocity)
        .map(|(position, velocity)| position + input.dt * velocity)
        .collect::<Vec<_>>();
    let discrete_acceleration = next_velocity
        .iter()
        .zip(&input.velocity)
        .map(|(next, current)| (next - current) / input.dt)
        .collect::<Vec<_>>();

    let max_negative_impulse = contact_impulse
        .iter()
        .map(|value| (-value).max(0.0))
        .fold(0.0_f64, f64::max);
    let max_negative_contact_slack = contact_slack
        .iter()
        .map(|value| (-value).max(0.0))
        .fold(0.0_f64, f64::max);
    let max_complementarity_residual = contact_impulse
        .iter()
        .zip(&contact_slack)
        .map(|(impulse, slack)| (impulse * slack).abs())
        .fold(0.0_f64, f64::max);
    let normal_velocity = mat_vec(&input.contact_jacobian, &next_velocity);
    let max_scaled_metric_equation_residual = metric_equation_relative_residual(
        &metric,
        &next_velocity,
        &rhs,
        &input.contact_jacobian,
        &contact_impulse,
    );
    let max_scaled_dual_feasibility_residual =
        relative_negative_residual(&contact_impulse, &delassus, &contact_rhs);
    let max_scaled_primal_feasibility_residual = relative_primal_feasibility(
        &normal_velocity,
        &target_velocity,
        &contact_slack,
    );
    let max_scaled_complementarity_residual = relative_complementarity_residual(
        &contact_impulse,
        &normal_velocity,
        &target_velocity,
        &contact_slack,
    );

    Ok(DiscreteStepResult {
        effective_metric: metric,
        unconstrained_velocity: unconstrained,
        next_velocity,
        next_position_error,
        discrete_acceleration,
        contact_impulse,
        contact_slack,
        active_contacts,
        max_metric_equation_residual: metric_residual,
        max_scaled_metric_equation_residual,
        max_negative_impulse,
        max_negative_contact_slack,
        max_complementarity_residual,
        max_scaled_dual_feasibility_residual,
        max_scaled_primal_feasibility_residual,
        max_scaled_complementarity_residual,
    })
}

fn validate_input(input: &DiscreteStepInput) -> Result<(), StepError> {
    let n = input.mass.rows;
    let m = input.contact_jacobian.rows;
    if n == 0
        || n > MAX_DOF
        || m > MAX_CONTACTS
        || input.mass.cols != n
        || input.damping.rows != n
        || input.damping.cols != n
        || input.stiffness.rows != n
        || input.stiffness.cols != n
        || input.position_error.len() != n
        || input.velocity.len() != n
        || input.external_force.len() != n
        || input.contact_jacobian.cols != n
        || input.contact_gap.len() != m
    {
        return Err(if n > MAX_DOF || m > MAX_CONTACTS {
            StepError::ProblemTooLarge {
                dof: n,
                contacts: m,
            }
        } else {
            StepError::InvalidVectorShape
        });
    }
    if !input.dt.is_finite() || input.dt <= 0.0 {
        return Err(StepError::InvalidStepSize);
    }
    if !input
        .position_error
        .iter()
        .chain(&input.velocity)
        .chain(&input.external_force)
        .chain(&input.contact_gap)
        .all(|value| value.is_finite())
    {
        return Err(StepError::NonFiniteInput);
    }
    validate_symmetric("mass", &input.mass)?;
    validate_symmetric("damping", &input.damping)?;
    validate_symmetric("stiffness", &input.stiffness)?;
    if !input
        .contact_jacobian
        .values
        .iter()
        .all(|value| value.is_finite())
    {
        return Err(StepError::NonFiniteInput);
    }
    if solve_spd(&input.mass, &vec![1.0; n]).is_none() {
        return Err(StepError::EffectiveMetricNotPositiveDefinite);
    }
    Ok(())
}

fn validate_symmetric(name: &'static str, matrix: &Matrix) -> Result<(), StepError> {
    if matrix.rows != matrix.cols {
        return Err(StepError::InvalidMatrixShape);
    }
    let residual = (0..matrix.rows)
        .flat_map(|row| (0..matrix.cols).map(move |col| (row, col)))
        .map(|(row, col)| (matrix[(row, col)] - matrix[(col, row)]).abs())
        .fold(0.0_f64, f64::max);
    let scale = matrix.max_abs();
    if residual > SOLVE_TOLERANCE * scale {
        return Err(StepError::MatrixNotSymmetric { name, residual });
    }
    if !matrix.values.iter().all(|value| value.is_finite()) {
        return Err(StepError::NonFiniteInput);
    }
    Ok(())
}

fn cholesky(matrix: &Matrix) -> Option<Matrix> {
    if matrix.rows != matrix.cols {
        return None;
    }
    let n = matrix.rows;
    // Use a relative pivot floor. An absolute floor would make an otherwise
    // equivalent problem fail merely because its mass / metric units were
    // uniformly rescaled.
    let scale = matrix.max_abs();
    if !scale.is_finite() || scale == 0.0 {
        return None;
    }
    let pivot_floor = SOLVE_TOLERANCE * scale;
    let mut lower = Matrix::zeros(n, n);
    for row in 0..n {
        for col in 0..=row {
            let mut sum = matrix[(row, col)];
            for k in 0..col {
                sum -= lower[(row, k)] * lower[(col, k)];
            }
            if row == col {
                if !sum.is_finite() || sum <= pivot_floor {
                    return None;
                }
                lower[(row, col)] = sum.sqrt();
            } else {
                let diagonal = lower[(col, col)];
                if diagonal <= 0.0 {
                    return None;
                }
                lower[(row, col)] = sum / diagonal;
            }
        }
    }
    Some(lower)
}

fn solve_spd(matrix: &Matrix, rhs: &[f64]) -> Option<Vec<f64>> {
    if matrix.rows != rhs.len() {
        return None;
    }
    let lower = cholesky(matrix)?;
    let n = rhs.len();
    let mut y = vec![0.0; n];
    for row in 0..n {
        let mut value = rhs[row];
        for col in 0..row {
            value -= lower[(row, col)] * y[col];
        }
        y[row] = value / lower[(row, row)];
    }

    let mut solution = vec![0.0; n];
    for row in (0..n).rev() {
        let mut value = y[row];
        for col in (row + 1)..n {
            value -= lower[(col, row)] * solution[col];
        }
        solution[row] = value / lower[(row, row)];
    }
    solution
        .iter()
        .all(|value| value.is_finite())
        .then_some(solution)
}

fn mat_vec(matrix: &Matrix, vector: &[f64]) -> Vec<f64> {
    (0..matrix.rows)
        .map(|row| {
            (0..matrix.cols)
                .map(|col| matrix[(row, col)] * vector[col])
                .sum()
        })
        .collect()
}

fn multiply(left: &Matrix, right: &Matrix) -> Matrix {
    let mut result = Matrix::zeros(left.rows, right.cols);
    for row in 0..left.rows {
        for col in 0..right.cols {
            for inner in 0..left.cols {
                result[(row, col)] += left[(row, inner)] * right[(inner, col)];
            }
        }
    }
    result
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

/// Return the maximum dimensionless unilateral impulse violation, normalized
/// by a contact-local impulse scale derived from that Delassus row.
///
/// For row i, the characteristic impulse is based on the local free velocity
/// residual and the absolute coupled impulse contribution divided by W_ii.
/// This prevents a large impulse at an unrelated contact from masking a
/// negative impulse at a much smaller contact scale.
fn relative_negative_residual(
    values: &[f64],
    delassus: &Matrix,
    contact_rhs: &[f64],
) -> f64 {
    values
        .iter()
        .enumerate()
        .map(|(row, value)| {
            let violation = (-value).max(0.0);
            if violation == 0.0 {
                return 0.0;
            }

            let diagonal = delassus[(row, row)].abs();
            let coupled_velocity_scale = contact_rhs[row].abs()
                + (0..values.len())
                    .filter(|col| *col != row)
                    .map(|col| delassus[(row, col)].abs() * values[col].abs())
                    .sum::<f64>();
            let local_impulse_scale = if diagonal > 0.0 {
                coupled_velocity_scale / diagonal
            } else {
                0.0
            };
            let scale = value.abs().max(local_impulse_scale);
            if scale > 0.0 && scale.is_finite() {
                violation / scale
            } else {
                f64::INFINITY
            }
        })
        .fold(0.0_f64, f64::max)
}

/// Dimensionless unilateral feasibility residual, normalized per contact row
/// by the larger of the attained and target normal velocities.
fn relative_primal_feasibility(
    normal_velocity: &[f64],
    target_velocity: &[f64],
    slack: &[f64],
) -> f64 {
    normal_velocity
        .iter()
        .zip(target_velocity)
        .zip(slack)
        .map(|((normal, target), row_slack)| {
            let violation = (-row_slack).max(0.0);
            let scale = normal.abs().max(target.abs());
            if violation == 0.0 {
                0.0
            } else if scale > 0.0 {
                violation / scale
            } else {
                f64::INFINITY
            }
        })
        .fold(0.0_f64, f64::max)
}

/// Dimensionless complementarity residual. For each positive impulse, divide
/// |p_i s_i| by |p_i| times that row's normal-velocity scale. Zero-impulse rows
/// satisfy complementarity by construction.
fn relative_complementarity_residual(
    impulse: &[f64],
    normal_velocity: &[f64],
    target_velocity: &[f64],
    slack: &[f64],
) -> f64 {
    impulse
        .iter()
        .zip(normal_velocity)
        .zip(target_velocity)
        .zip(slack)
        .map(|(((p, normal), target), row_slack)| {
            if *p <= 0.0 {
                0.0
            } else {
                let scale = normal.abs().max(target.abs());
                if row_slack.abs() == 0.0 {
                    0.0
                } else if scale > 0.0 {
                    row_slack.abs() / scale
                } else {
                    f64::INFINITY
                }
            }
        })
        .fold(0.0_f64, f64::max)
}

/// Maximum componentwise equation residual normalized by the largest
/// contributing term for that generalized coordinate.
fn metric_equation_relative_residual(
    metric: &Matrix,
    velocity: &[f64],
    rhs: &[f64],
    contact_jacobian: &Matrix,
    impulse: &[f64],
) -> f64 {
    let lhs = mat_vec(metric, velocity);
    let mut maximum = 0.0_f64;
    for row in 0..lhs.len() {
        let contact_term = (0..impulse.len())
            .map(|contact| contact_jacobian[(contact, row)] * impulse[contact])
            .sum::<f64>();
        let residual = (lhs[row] - rhs[row] - contact_term).abs();
        let scale = lhs[row].abs().max(rhs[row].abs()).max(contact_term.abs());
        let relative = if residual == 0.0 {
            0.0
        } else if scale > 0.0 {
            residual / scale
        } else {
            f64::INFINITY
        };
        maximum = maximum.max(relative);
    }
    maximum
}

fn metric_equation_residual(
    metric: &Matrix,
    velocity: &[f64],
    rhs: &[f64],
    contact_jacobian: &Matrix,
    impulse: &[f64],
) -> f64 {
    let lhs = mat_vec(metric, velocity);
    let mut max_residual = 0.0_f64;
    for row in 0..lhs.len() {
        let contact_term = (0..impulse.len())
            .map(|contact| contact_jacobian[(contact, row)] * impulse[contact])
            .sum::<f64>();
        max_residual = max_residual.max((lhs[row] - rhs[row] - contact_term).abs());
    }
    max_residual
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(value: f64) -> Matrix {
        Matrix::new(1, 1, vec![value]).unwrap()
    }

    fn no_contacts(dof: usize) -> Matrix {
        Matrix::new(0, dof, Vec::new()).unwrap()
    }

    #[test]
    fn unconstrained_backward_euler_uses_effective_metric() {
        let input = DiscreteStepInput {
            mass: scalar(2.0),
            damping: scalar(0.4),
            stiffness: scalar(10.0),
            position_error: vec![0.5],
            velocity: vec![0.0],
            external_force: vec![0.0],
            contact_jacobian: no_contacts(1),
            contact_gap: vec![],
            dt: 0.1,
        };
        let result = solve_discrete_step(&input).unwrap();
        assert!((result.effective_metric[(0, 0)] - 2.14).abs() < 1.0e-12);
        assert!((result.next_velocity[0] - (-0.5 / 2.14)).abs() < 1.0e-12);
        assert!(
            (result.next_position_error[0] - (0.5 + 0.1 * result.next_velocity[0])).abs() < 1.0e-12
        );
        assert!(result.max_metric_equation_residual < 1.0e-10);
    }

    #[test]
    fn contact_impulse_prevents_penetration_in_the_effective_metric() {
        let input = DiscreteStepInput {
            mass: scalar(1.0),
            damping: scalar(0.0),
            stiffness: scalar(0.0),
            position_error: vec![0.0],
            velocity: vec![-1.0],
            external_force: vec![0.0],
            contact_jacobian: scalar(1.0),
            contact_gap: vec![0.0],
            dt: 0.01,
        };
        let result = solve_discrete_step(&input).unwrap();
        assert!((result.contact_impulse[0] - 1.0).abs() < 1.0e-12);
        assert!(result.next_velocity[0].abs() < 1.0e-12);
        assert!(result.contact_slack[0].abs() < 1.0e-12);
        assert!(result.max_complementarity_residual < 1.0e-12);
    }

    #[test]
    fn contact_and_stiffness_share_the_same_metric_and_delassus_operator() {
        let input = DiscreteStepInput {
            mass: Matrix::identity(2),
            damping: Matrix::zeros(2, 2),
            stiffness: Matrix::new(2, 2, vec![100.0, 0.0, 0.0, 0.0]).unwrap(),
            position_error: vec![0.1, 0.0],
            velocity: vec![-1.0, -1.0],
            external_force: vec![0.0, 0.0],
            contact_jacobian: Matrix::new(1, 2, vec![1.0, 1.0]).unwrap(),
            contact_gap: vec![0.0],
            dt: 0.1,
        };
        let result = solve_discrete_step(&input).unwrap();
        assert!((result.effective_metric[(0, 0)] - 2.0).abs() < 1.0e-12);
        assert!((result.effective_metric[(1, 1)] - 1.0).abs() < 1.0e-12);
        assert!((result.contact_impulse[0] - 4.0 / 3.0).abs() < 1.0e-12);
        assert!((result.next_velocity[0] + 1.0 / 3.0).abs() < 1.0e-12);
        assert!((result.next_velocity[1] - 1.0 / 3.0).abs() < 1.0e-12);
        assert!(result.contact_slack[0].abs() < 1.0e-12);
        assert!(result.max_metric_equation_residual < 1.0e-10);
    }

    #[test]
    fn contact_active_set_is_deterministic_for_two_independent_normals() {
        let input = DiscreteStepInput {
            mass: Matrix::identity(2),
            damping: Matrix::zeros(2, 2),
            stiffness: Matrix::zeros(2, 2),
            position_error: vec![0.0, 0.0],
            velocity: vec![-1.0, -2.0],
            external_force: vec![0.0, 0.0],
            contact_jacobian: Matrix::identity(2),
            contact_gap: vec![0.0, 0.0],
            dt: 0.01,
        };
        let first = solve_discrete_step(&input).unwrap();
        let second = solve_discrete_step(&input).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.contact_impulse, vec![1.0, 2.0]);
        assert_eq!(first.next_velocity, vec![0.0, 0.0]);
        assert_eq!(first.active_contacts, vec![true, true]);
    }

    #[test]
    fn invalid_step_and_non_spd_metric_fail_closed() {
        let mut input = DiscreteStepInput {
            mass: scalar(1.0),
            damping: scalar(0.0),
            stiffness: scalar(0.0),
            position_error: vec![0.0],
            velocity: vec![0.0],
            external_force: vec![0.0],
            contact_jacobian: no_contacts(1),
            contact_gap: vec![],
            dt: 0.0,
        };
        assert_eq!(solve_discrete_step(&input), Err(StepError::InvalidStepSize));

        input.dt = 0.1;
        input.mass = scalar(-1.0);
        assert_eq!(
            solve_discrete_step(&input),
            Err(StepError::EffectiveMetricNotPositiveDefinite)
        );
    }

    #[test]
    fn asymmetric_matrices_are_rejected_before_solving() {
        let input = DiscreteStepInput {
            mass: Matrix::identity(2),
            damping: Matrix::zeros(2, 2),
            stiffness: Matrix::new(2, 2, vec![1.0, 0.2, 0.0, 1.0]).unwrap(),
            position_error: vec![0.0, 0.0],
            velocity: vec![0.0, 0.0],
            external_force: vec![0.0, 0.0],
            contact_jacobian: no_contacts(2),
            contact_gap: vec![],
            dt: 0.1,
        };
        assert!(matches!(
            solve_discrete_step(&input),
            Err(StepError::MatrixNotSymmetric {
                name: "stiffness",
                ..
            })
        ));
    }
    #[test]
    fn contact_step_is_covariant_under_uniform_mass_rescaling() {
        for mass in [1.0e-12, 1.0, 1.0e12] {
            let input = DiscreteStepInput {
                mass: scalar(mass),
                damping: scalar(0.0),
                stiffness: scalar(0.0),
                position_error: vec![0.0],
                velocity: vec![-1.0],
                external_force: vec![0.0],
                contact_jacobian: scalar(1.0),
                contact_gap: vec![0.0],
                dt: 0.01,
            };
            let result = solve_discrete_step(&input).unwrap();

            assert!(
                result.next_velocity[0].abs() < 1.0e-12,
                "mass={mass:e}: next velocity={:e}",
                result.next_velocity[0]
            );
            assert!(
                (result.contact_impulse[0] / mass - 1.0).abs() < 1.0e-10,
                "mass={mass:e}: impulse/mass={:e}",
                result.contact_impulse[0] / mass
            );
            assert_eq!(
                result.active_contacts,
                vec![true],
                "active-set classification changed at mass={mass:e}"
            );
            assert!(result.max_metric_equation_residual / mass < 1.0e-10);
            assert!(result.max_scaled_metric_equation_residual < 1.0e-10);
            assert_eq!(result.max_scaled_dual_feasibility_residual, 0.0);
            assert!(result.max_scaled_primal_feasibility_residual < 1.0e-10);
            assert!(result.max_scaled_complementarity_residual < 1.0e-10);
        }
    }

    #[test]
    fn dimensionless_contact_residuals_are_invariant_under_uniform_velocity_units() {
        let impulse = [2.0, -0.002];
        let normal_velocity = [0.27, -1.0];
        let target_velocity = [0.25, 0.0];
        let slack = [0.02, -1.0];
        let reference = (
            relative_negative_residual(&impulse, &Matrix::identity(2), &impulse),
            relative_primal_feasibility(&normal_velocity, &target_velocity, &slack),
            relative_complementarity_residual(
                &impulse,
                &normal_velocity,
                &target_velocity,
                &slack,
            ),
        );

        for scale in [1.0e-12, 1.0, 1.0e12] {
            let scaled_impulse = impulse.map(|value| value * scale);
            let scaled_normal = normal_velocity.map(|value| value * scale);
            let scaled_target = target_velocity.map(|value| value * scale);
            let scaled_slack = slack.map(|value| value * scale);
            let observed = (
                relative_negative_residual(
                    &scaled_impulse,
                    &Matrix::identity(2),
                    &scaled_impulse,
                ),
                relative_primal_feasibility(&scaled_normal, &scaled_target, &scaled_slack),
                relative_complementarity_residual(
                    &scaled_impulse,
                    &scaled_normal,
                    &scaled_target,
                    &scaled_slack,
                ),
            );
            assert_eq!(observed, reference, "velocity/impulse scale={scale:e}");
        }
    }

    #[test]
    fn local_dual_violation_cannot_be_hidden_by_larger_contact_impulse() {
        let delassus = Matrix::identity(2);
        let impulse = [1.0e8, -1.0e-8];
        let contact_rhs = [1.0e8, -1.0e-8];

        // A single global infinity norm would report 1e-16 here and accept
        // the second contact's 100% local dual-feasibility violation.
        let residual = relative_negative_residual(&impulse, &delassus, &contact_rhs);
        assert_eq!(residual, 1.0);
    }

    #[test]
    fn symmetry_validation_is_relative_to_matrix_scale() {
        let input = DiscreteStepInput {
            mass: Matrix::identity(2),
            damping: Matrix::zeros(2, 2),
            stiffness: Matrix::new(2, 2, vec![1.0e-12, 2.0e-13, 0.0, 1.0e-12]).unwrap(),
            position_error: vec![0.0, 0.0],
            velocity: vec![0.0, 0.0],
            external_force: vec![0.0, 0.0],
            contact_jacobian: no_contacts(2),
            contact_gap: vec![],
            dt: 0.1,
        };

        assert!(matches!(
            solve_discrete_step(&input),
            Err(StepError::MatrixNotSymmetric {
                name: "stiffness",
                ..
            })
        ));
    }

    #[test]
    fn contradictory_unilateral_rows_return_no_complementary_solution() {
        let input = DiscreteStepInput {
            mass: scalar(1.0),
            damping: scalar(0.0),
            stiffness: scalar(0.0),
            position_error: vec![0.0],
            velocity: vec![0.0],
            external_force: vec![0.0],
            contact_jacobian: Matrix::new(2, 1, vec![1.0, -1.0]).unwrap(),
            contact_gap: vec![-0.1, -0.1],
            dt: 0.01,
        };
        assert_eq!(
            solve_discrete_step(&input),
            Err(StepError::NoComplementarySolution)
        );
    }


}
