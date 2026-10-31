// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Coupled semi-explicit pHDAE discrete-gradient method for a 3-body,
// 2-link spatial open chain. The midpoint multiplier vector is solved
// simultaneously; this is not a collection of independent dumbbells.

use std::f64;

pub type Vec3 = [f64; 3];
pub type Mat2 = [[f64; 2]; 2];

const BODY_COUNT: usize = 3;
const EDGE_COUNT: usize = 2;
const DENSE_DIM: usize = 9;
const EDGES: [(usize, usize); EDGE_COUNT] = [(0, 1), (1, 2)];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    InvalidParameter,
    InvalidTimeStep,
    InvalidInitialLinkConstraint { edge: usize },
    InvalidInitialVelocityConstraint { edge: usize },
    NonFiniteInput,
    InvalidSolverOptions,
    SingularMidpointVelocityBlock,
    SingularConstraintJacobian,
    NoConvergence,
    NonFiniteResult,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub q: [Vec3; BODY_COUNT],
    pub v: [Vec3; BODY_COUNT],
}

#[derive(Clone, Copy, Debug)]
pub struct SolverOptions {
    pub relative_tolerance: f64,
    pub max_iterations: usize,
    pub max_backtracks: usize,
    pub pivot_tolerance: f64,
}

impl Default for SolverOptions {
    fn default() -> Self {
        Self {
            relative_tolerance: 1.0e-12,
            max_iterations: 40,
            max_backtracks: 28,
            pivot_tolerance: 1.0e-13,
        }
    }
}

impl SolverOptions {
    fn validate(self) -> Result<Self, Error> {
        if !self.relative_tolerance.is_finite()
            || self.relative_tolerance <= 0.0
            || !self.pivot_tolerance.is_finite()
            || self.pivot_tolerance <= 0.0
            || self.max_iterations == 0
            || self.max_backtracks == 0
        {
            return Err(Error::InvalidSolverOptions);
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ThreeBodyChain {
    masses: [f64; BODY_COUNT],
    lengths: [f64; EDGE_COUNT],
    constraint_tolerance: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct StepReceipt {
    pub before: State,
    pub after: State,
    pub midpoint_positions: [Vec3; BODY_COUNT],
    pub midpoint_velocities: [Vec3; BODY_COUNT],
    pub midpoint_relative_positions: [Vec3; EDGE_COUNT],
    pub midpoint_relative_velocities: [Vec3; EDGE_COUNT],
    pub multipliers: [f64; EDGE_COUNT],
    pub multiplier_residuals: [f64; EDGE_COUNT],
    pub scaled_multiplier_residuals: [f64; EDGE_COUNT],
    pub multiplier_iterations: usize,
    /// Absolute determinant of the coupled algebraic multiplier Jacobian.
    pub multiplier_jacobian_determinant: f64,
    /// |det(J)| divided by the product of row norms; near zero means
    /// the multiplier Jacobian is poorly conditioned or rank deficient.
    pub normalized_multiplier_jacobian_determinant: f64,
    pub energy_before: f64,
    pub energy_after: f64,
    pub external_work: f64,
    pub constraint_reaction_work: f64,
    pub energy_balance_residual: f64,
    pub relative_energy_balance_residual: f64,
    pub link_constraint_residuals: [f64; EDGE_COUNT],
    pub relative_link_constraint_residuals: [f64; EDGE_COUNT],
    pub midpoint_constraint_residuals: [f64; EDGE_COUNT],
    pub relative_midpoint_constraint_residuals: [f64; EDGE_COUNT],
    pub endpoint_velocity_constraint_residuals: [f64; EDGE_COUNT],
    pub relative_endpoint_velocity_constraint_residuals: [f64; EDGE_COUNT],
    pub linear_momentum_residual: Vec3,
    pub relative_linear_momentum_residual: f64,
    pub angular_momentum_residual: Vec3,
    pub relative_angular_momentum_residual: f64,
}

pub fn add(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub fn scale(a: Vec3, factor: f64) -> Vec3 {
    [a[0] * factor, a[1] * factor, a[2] * factor]
}

pub fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

fn incidence(body: usize, edge: usize) -> f64 {
    let (from, to) = EDGES[edge];
    if body == from {
        1.0
    } else if body == to {
        -1.0
    } else {
        0.0
    }
}

/// Dense Gaussian elimination with partial pivoting.
/// Returns a typed failure rather than regularizing a singular system.
fn solve_dense<const N: usize>(
    mut matrix: [[f64; N]; N],
    mut rhs: [f64; N],
    pivot_tolerance: f64,
) -> Result<[f64; N], Error> {
    let mut matrix_scale = 0.0_f64;
    for row in matrix {
        for value in row {
            matrix_scale = matrix_scale.max(value.abs());
        }
    }
    let threshold = pivot_tolerance * matrix_scale.max(1.0);

    for column in 0..N {
        let mut pivot_row = column;
        let mut pivot_abs = matrix[column][column].abs();
        for row in (column + 1)..N {
            let candidate = matrix[row][column].abs();
            if candidate > pivot_abs {
                pivot_abs = candidate;
                pivot_row = row;
            }
        }
        if !pivot_abs.is_finite() || pivot_abs <= threshold {
            return Err(Error::SingularMidpointVelocityBlock);
        }
        if pivot_row != column {
            matrix.swap(pivot_row, column);
            rhs.swap(pivot_row, column);
        }

        let pivot = matrix[column][column];
        for row in (column + 1)..N {
            let factor = matrix[row][column] / pivot;
            matrix[row][column] = 0.0;
            for k in (column + 1)..N {
                matrix[row][k] -= factor * matrix[column][k];
            }
            rhs[row] -= factor * rhs[column];
        }
    }

    let mut solution = [0.0_f64; N];
    for row in (0..N).rev() {
        let mut value = rhs[row];
        for column in (row + 1)..N {
            value -= matrix[row][column] * solution[column];
        }
        let pivot = matrix[row][row];
        if !pivot.is_finite() || pivot.abs() <= threshold {
            return Err(Error::SingularMidpointVelocityBlock);
        }
        solution[row] = value / pivot;
        if !solution[row].is_finite() {
            return Err(Error::NonFiniteResult);
        }
    }
    Ok(solution)
}

impl ThreeBodyChain {
    pub fn new(masses: [f64; BODY_COUNT], lengths: [f64; EDGE_COUNT]) -> Result<Self, Error> {
        if !masses.iter().all(|value| value.is_finite())
            || !lengths.iter().all(|value| value.is_finite())
        {
            return Err(Error::InvalidParameter);
        }
        if masses.iter().any(|value| *value <= 0.0)
            || lengths.iter().any(|value| *value <= 0.0)
        {
            return Err(Error::InvalidParameter);
        }
        Ok(Self {
            masses,
            lengths,
            constraint_tolerance: 2.0e-10,
        })
    }

    pub fn hamiltonian(&self, state: State) -> Result<f64, Error> {
        if !state.q.iter().flatten().all(|value| value.is_finite())
            || !state.v.iter().flatten().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        let mut energy = 0.0;
        for body in 0..BODY_COUNT {
            energy += 0.5 * self.masses[body] * dot(state.v[body], state.v[body]);
        }
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(Error::NonFiniteResult)
        }
    }

    pub fn linear_momentum(&self, state: State) -> Vec3 {
        let mut momentum = [0.0; 3];
        for body in 0..BODY_COUNT {
            momentum = add(momentum, scale(state.v[body], self.masses[body]));
        }
        momentum
    }

    pub fn angular_momentum(&self, state: State) -> Vec3 {
        let mut momentum = [0.0; 3];
        for body in 0..BODY_COUNT {
            momentum = add(
                momentum,
                scale(cross(state.q[body], state.v[body]), self.masses[body]),
            );
        }
        momentum
    }

    fn validate_initial(&self, state: State) -> Result<(), Error> {
        if !state.q.iter().flatten().all(|value| value.is_finite())
            || !state.v.iter().flatten().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        for edge in 0..EDGE_COUNT {
            let (from, to) = EDGES[edge];
            let relative_position = sub(state.q[to], state.q[from]);
            let relative_velocity = sub(state.v[to], state.v[from]);
            let position_error =
                (dot(relative_position, relative_position)
                    - self.lengths[edge] * self.lengths[edge])
                    .abs();
            let position_scale = (1.0
                + dot(relative_position, relative_position)
                + self.lengths[edge] * self.lengths[edge])
            .max(f64::MIN_POSITIVE);
            if position_error > self.constraint_tolerance * position_scale {
                return Err(Error::InvalidInitialLinkConstraint { edge });
            }
            let tangent_error = dot(relative_position, relative_velocity).abs();
            let tangent_scale = (1.0
                + self.lengths[edge] * norm(relative_velocity))
            .max(f64::MIN_POSITIVE);
            if tangent_error > self.constraint_tolerance * tangent_scale {
                return Err(Error::InvalidInitialVelocityConstraint { edge });
            }
        }
        Ok(())
    }

    /// Continuous acceleration-level multiplier estimate. This is only a
    /// Newton initial guess; the accepted multipliers solve the discrete
    /// midpoint constraints simultaneously.
    fn continuous_multiplier_guess(
        &self,
        before: State,
        forces: [Vec3; BODY_COUNT],
        options: SolverOptions,
    ) -> Result<[f64; EDGE_COUNT], Error> {
        let mut matrix = [[0.0_f64; EDGE_COUNT]; EDGE_COUNT];
        let mut rhs = [0.0_f64; EDGE_COUNT];
        let mut relative_positions = [[0.0_f64; 3]; EDGE_COUNT];
        let mut relative_velocities = [[0.0_f64; 3]; EDGE_COUNT];

        for edge in 0..EDGE_COUNT {
            let (from, to) = EDGES[edge];
            relative_positions[edge] = sub(before.q[to], before.q[from]);
            relative_velocities[edge] = sub(before.v[to], before.v[from]);
            rhs[edge] = -dot(relative_velocities[edge], relative_velocities[edge])
                - dot(
                    relative_positions[edge],
                    sub(
                        scale(forces[to], 1.0 / self.masses[to]),
                        scale(forces[from], 1.0 / self.masses[from]),
                    ),
                );

            for multiplier_edge in 0..EDGE_COUNT {
                let (mult_from, mult_to) = EDGES[multiplier_edge];
                let coefficient = incidence(to, multiplier_edge) / self.masses[to]
                    - incidence(from, multiplier_edge) / self.masses[from];
                let relative_multiplier_position =
                    sub(before.q[mult_to], before.q[mult_from]);
                matrix[edge][multiplier_edge] =
                    dot(relative_positions[edge], relative_multiplier_position) * coefficient;
            }
        }

        solve_dense(matrix, rhs, options.pivot_tolerance)
            .map_err(|_| Error::SingularConstraintJacobian)
    }

    /// For fixed multiplier vector, solve the coupled 9x9 midpoint-velocity
    /// block system and differentiate through that solve to form the exact
    /// 2x2 Jacobian of the coupled algebraic constraint residual.
    fn evaluate_multiplier(
        &self,
        before: State,
        forces: [Vec3; BODY_COUNT],
        dt: f64,
        multipliers: [f64; EDGE_COUNT],
        pivot_tolerance: f64,
    ) -> Result<([[f64; 3]; BODY_COUNT], [f64; EDGE_COUNT], Mat2), Error> {
        let mut matrix = [[0.0_f64; DENSE_DIM]; DENSE_DIM];
        let mut rhs = [0.0_f64; DENSE_DIM];

        for body in 0..BODY_COUNT {
            for axis in 0..3 {
                let index = 3 * body + axis;
                matrix[index][index] = 1.0;
                rhs[index] = before.v[body][axis]
                    + 0.5 * dt * forces[body][axis] / self.masses[body];
            }
        }

        for edge in 0..EDGE_COUNT {
            let (from, to) = EDGES[edge];
            let relative_position = sub(before.q[to], before.q[from]);
            let lambda = multipliers[edge];
            for axis in 0..3 {
                let from_row = 3 * from + axis;
                let to_row = 3 * to + axis;
                let from_coupling =
                    dt * dt * lambda / (4.0 * self.masses[from]);
                let to_coupling =
                    dt * dt * lambda / (4.0 * self.masses[to]);

                matrix[from_row][from_row] += from_coupling;
                matrix[from_row][to_row] -= from_coupling;
                matrix[to_row][to_row] += to_coupling;
                matrix[to_row][from_row] -= to_coupling;

                rhs[from_row] +=
                    0.5 * dt * lambda * relative_position[axis] / self.masses[from];
                rhs[to_row] -=
                    0.5 * dt * lambda * relative_position[axis] / self.masses[to];
            }
        }

        let midpoint_flat =
            solve_dense(matrix, rhs, pivot_tolerance)?;
        let mut midpoint_velocities = [[0.0_f64; 3]; BODY_COUNT];
        for body in 0..BODY_COUNT {
            for axis in 0..3 {
                midpoint_velocities[body][axis] = midpoint_flat[3 * body + axis];
            }
        }

        let mut midpoint_relative_positions = [[0.0_f64; 3]; EDGE_COUNT];
        let mut midpoint_relative_velocities = [[0.0_f64; 3]; EDGE_COUNT];
        let mut residuals = [0.0_f64; EDGE_COUNT];
        for edge in 0..EDGE_COUNT {
            let (from, to) = EDGES[edge];
            let relative_position = sub(before.q[to], before.q[from]);
            let relative_velocity =
                sub(midpoint_velocities[to], midpoint_velocities[from]);
            let midpoint_position =
                add(relative_position, scale(relative_velocity, 0.5 * dt));
            midpoint_relative_positions[edge] = midpoint_position;
            midpoint_relative_velocities[edge] = relative_velocity;
            residuals[edge] = dot(midpoint_position, relative_velocity);
        }

        let mut jacobian = [[0.0_f64; EDGE_COUNT]; EDGE_COUNT];
        for multiplier_edge in 0..EDGE_COUNT {
            let (mult_from, mult_to) = EDGES[multiplier_edge];
            let relative_position = sub(before.q[mult_to], before.q[mult_from]);
            let lambda = multipliers[multiplier_edge];

            let mut derivative_matrix = [[0.0_f64; DENSE_DIM]; DENSE_DIM];
            let mut derivative_rhs = [0.0_f64; DENSE_DIM];
            for axis in 0..3 {
                let from_row = 3 * mult_from + axis;
                let to_row = 3 * mult_to + axis;
                let from_coupling = dt * dt / (4.0 * self.masses[mult_from]);
                let to_coupling = dt * dt / (4.0 * self.masses[mult_to]);

                derivative_matrix[from_row][from_row] += from_coupling;
                derivative_matrix[from_row][to_row] -= from_coupling;
                derivative_matrix[to_row][to_row] += to_coupling;
                derivative_matrix[to_row][from_row] -= to_coupling;

                derivative_rhs[from_row] +=
                    0.5 * dt * relative_position[axis] / self.masses[mult_from];
                derivative_rhs[to_row] -=
                    0.5 * dt * relative_position[axis] / self.masses[mult_to];
            }

            for row in 0..DENSE_DIM {
                for column in 0..DENSE_DIM {
                    derivative_rhs[row] -=
                        derivative_matrix[row][column] * midpoint_flat[column];
                }
            }
            let derivative_flat =
                solve_dense(matrix, derivative_rhs, pivot_tolerance)?;
            let mut derivative_velocities = [[0.0_f64; 3]; BODY_COUNT];
            for body in 0..BODY_COUNT {
                for axis in 0..3 {
                    derivative_velocities[body][axis] =
                        derivative_flat[3 * body + axis];
                }
            }

            for edge in 0..EDGE_COUNT {
                let (from, to) = EDGES[edge];
                let d_relative_velocity = sub(
                    derivative_velocities[to],
                    derivative_velocities[from],
                );
                let endpoint_relative_position = add(
                    midpoint_relative_positions[edge],
                    scale(midpoint_relative_velocities[edge], 0.5 * dt),
                );
                jacobian[edge][multiplier_edge] =
                    dot(endpoint_relative_position, d_relative_velocity);
            }

            if !lambda.is_finite() {
                return Err(Error::NonFiniteResult);
            }
        }

        if !midpoint_velocities.iter().flatten().all(|value| value.is_finite())
            || !residuals.iter().all(|value| value.is_finite())
            || !jacobian.iter().flatten().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteResult);
        }

        Ok((midpoint_velocities, residuals, jacobian))
    }

    fn scaled_residual_norm(
        midpoint_positions: [Vec3; EDGE_COUNT],
        midpoint_velocities: [Vec3; EDGE_COUNT],
        residuals: [f64; EDGE_COUNT],
    ) -> f64 {
        let mut sum = 0.0;
        for edge in 0..EDGE_COUNT {
            let scale = 1.0
                + norm(midpoint_positions[edge]) * norm(midpoint_velocities[edge]);
            let value = residuals[edge] / scale;
            sum += value * value;
        }
        sum.sqrt()
    }

    pub fn step(
        &self,
        before: State,
        forces: [Vec3; BODY_COUNT],
        dt: f64,
        options: SolverOptions,
    ) -> Result<StepReceipt, Error> {
        let options = options.validate()?;
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        if !forces.iter().flatten().all(|value| value.is_finite()) {
            return Err(Error::NonFiniteInput);
        }
        self.validate_initial(before)?;

        let energy_before = self.hamiltonian(before)?;
        let linear_before = self.linear_momentum(before);
        let angular_before = self.angular_momentum(before);
        let mut multipliers =
            self.continuous_multiplier_guess(before, forces, options)?;

        let mut iterations = 0usize;
        let mut converged = false;
        let mut final_jacobian = [[0.0; EDGE_COUNT]; EDGE_COUNT];
        let mut final_residuals = [0.0; EDGE_COUNT];
        let mut final_midpoint_velocities = [[0.0; 3]; BODY_COUNT];

        for iteration in 0..options.max_iterations {
            let (midpoint_velocities, residuals, jacobian) = self.evaluate_multiplier(
                before,
                forces,
                dt,
                multipliers,
                options.pivot_tolerance,
            )?;
            let mut midpoint_positions = [[0.0; 3]; EDGE_COUNT];
            let mut midpoint_relative_velocities = [[0.0; 3]; EDGE_COUNT];
            for edge in 0..EDGE_COUNT {
                let (from, to) = EDGES[edge];
                let relative_position = sub(before.q[to], before.q[from]);
                let relative_velocity =
                    sub(midpoint_velocities[to], midpoint_velocities[from]);
                midpoint_positions[edge] =
                    add(relative_position, scale(relative_velocity, 0.5 * dt));
                midpoint_relative_velocities[edge] = relative_velocity;
            }

            let residual_norm = Self::scaled_residual_norm(
                midpoint_positions,
                midpoint_relative_velocities,
                residuals,
            );
            final_jacobian = jacobian;
            final_residuals = residuals;
            final_midpoint_velocities = midpoint_velocities;

            if residual_norm <= options.relative_tolerance {
                iterations = iteration;
                converged = true;
                break;
            }

            let delta = solve_dense(
                jacobian,
                [-residuals[0], -residuals[1]],
                options.pivot_tolerance,
            )
            .map_err(|_| Error::SingularConstraintJacobian)?;

            let mut accepted = false;
            let mut fraction = 1.0;
            for _ in 0..options.max_backtracks {
                let candidate = [
                    multipliers[0] + fraction * delta[0],
                    multipliers[1] + fraction * delta[1],
                ];
                if candidate.iter().all(|value| value.is_finite()) {
                    if let Ok((candidate_velocities, candidate_residuals, _)) =
                        self.evaluate_multiplier(
                            before,
                            forces,
                            dt,
                            candidate,
                            options.pivot_tolerance,
                        )
                    {
                        let mut candidate_positions = [[0.0; 3]; EDGE_COUNT];
                        let mut candidate_relative_velocities = [[0.0; 3]; EDGE_COUNT];
                        for edge in 0..EDGE_COUNT {
                            let (from, to) = EDGES[edge];
                            let relative_position = sub(before.q[to], before.q[from]);
                            let relative_velocity =
                                sub(candidate_velocities[to], candidate_velocities[from]);
                            candidate_positions[edge] =
                                add(relative_position, scale(relative_velocity, 0.5 * dt));
                            candidate_relative_velocities[edge] = relative_velocity;
                        }
                        let candidate_norm = Self::scaled_residual_norm(
                            candidate_positions,
                            candidate_relative_velocities,
                            candidate_residuals,
                        );
                        if candidate_norm < residual_norm
                            || candidate_norm <= options.relative_tolerance
                        {
                            multipliers = candidate;
                            accepted = true;
                            break;
                        }
                    }
                }
                fraction *= 0.5;
            }
            if !accepted {
                return Err(Error::NoConvergence);
            }
            iterations = iteration + 1;
        }

        if !converged {
            let (midpoint_velocities, residuals, jacobian) = self.evaluate_multiplier(
                before,
                forces,
                dt,
                multipliers,
                options.pivot_tolerance,
            )?;
            let mut midpoint_positions = [[0.0; 3]; EDGE_COUNT];
            let mut midpoint_relative_velocities = [[0.0; 3]; EDGE_COUNT];
            for edge in 0..EDGE_COUNT {
                let (from, to) = EDGES[edge];
                let relative_position = sub(before.q[to], before.q[from]);
                let relative_velocity =
                    sub(midpoint_velocities[to], midpoint_velocities[from]);
                midpoint_positions[edge] =
                    add(relative_position, scale(relative_velocity, 0.5 * dt));
                midpoint_relative_velocities[edge] = relative_velocity;
            }
            converged = Self::scaled_residual_norm(
                midpoint_positions,
                midpoint_relative_velocities,
                residuals,
            ) <= options.relative_tolerance;
            final_jacobian = jacobian;
            final_residuals = residuals;
            final_midpoint_velocities = midpoint_velocities;
        }
        if !converged {
            return Err(Error::NoConvergence);
        }

        let mut midpoint_positions = [[0.0; 3]; BODY_COUNT];
        let mut after = before;
        for body in 0..BODY_COUNT {
            midpoint_positions[body] =
                add(before.q[body], scale(final_midpoint_velocities[body], 0.5 * dt));
            after.q[body] =
                add(before.q[body], scale(final_midpoint_velocities[body], dt));
            after.v[body] =
                sub(scale(final_midpoint_velocities[body], 2.0), before.v[body]);
        }

        let mut midpoint_relative_positions = [[0.0; 3]; EDGE_COUNT];
        let mut midpoint_relative_velocities = [[0.0; 3]; EDGE_COUNT];
        let mut link_residuals = [0.0; EDGE_COUNT];
        let mut relative_link_residuals = [0.0; EDGE_COUNT];
        let mut midpoint_constraint_residuals = [0.0; EDGE_COUNT];
        let mut relative_midpoint_residuals = [0.0; EDGE_COUNT];
        let mut endpoint_constraint_residuals = [0.0; EDGE_COUNT];
        let mut relative_endpoint_residuals = [0.0; EDGE_COUNT];

        for edge in 0..EDGE_COUNT {
            let (from, to) = EDGES[edge];
            let midpoint_relative_position =
                sub(midpoint_positions[to], midpoint_positions[from]);
            let midpoint_relative_velocity = sub(
                final_midpoint_velocities[to],
                final_midpoint_velocities[from],
            );
            let endpoint_relative_position = sub(after.q[to], after.q[from]);
            let endpoint_relative_velocity = sub(after.v[to], after.v[from]);

            midpoint_relative_positions[edge] = midpoint_relative_position;
            midpoint_relative_velocities[edge] = midpoint_relative_velocity;
            link_residuals[edge] = dot(
                endpoint_relative_position,
                endpoint_relative_position,
            ) - self.lengths[edge] * self.lengths[edge];
            relative_link_residuals[edge] = link_residuals[edge].abs()
                / (1.0
                    + dot(endpoint_relative_position, endpoint_relative_position)
                    + self.lengths[edge] * self.lengths[edge]);
            midpoint_constraint_residuals[edge] =
                dot(midpoint_relative_position, midpoint_relative_velocity);
            relative_midpoint_residuals[edge] =
                midpoint_constraint_residuals[edge].abs()
                    / (1.0
                        + norm(midpoint_relative_position)
                            * norm(midpoint_relative_velocity));
            endpoint_constraint_residuals[edge] =
                dot(endpoint_relative_position, endpoint_relative_velocity);
            relative_endpoint_residuals[edge] =
                endpoint_constraint_residuals[edge].abs()
                    / (1.0
                        + self.lengths[edge] * norm(endpoint_relative_velocity));
        }

        if !after.q.iter().flatten().all(|value| value.is_finite())
            || !after.v.iter().flatten().all(|value| value.is_finite())
            || !relative_link_residuals.iter().all(|value| value.is_finite())
            || !relative_midpoint_residuals.iter().all(|value| value.is_finite())
            || !relative_endpoint_residuals.iter().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteResult);
        }
        for edge in 0..EDGE_COUNT {
            if relative_link_residuals[edge] > self.constraint_tolerance
                || relative_midpoint_residuals[edge] > self.constraint_tolerance
            {
                return Err(Error::NoConvergence);
            }
        }

        let energy_after = self.hamiltonian(after)?;
        let mut external_work = 0.0;
        for body in 0..BODY_COUNT {
            external_work += dt * dot(forces[body], final_midpoint_velocities[body]);
        }
        let mut constraint_reaction_work = 0.0;
        for edge in 0..EDGE_COUNT {
            constraint_reaction_work -=
                dt * multipliers[edge] * midpoint_constraint_residuals[edge];
        }
        let energy_balance_residual =
            energy_after - energy_before - external_work - constraint_reaction_work;
        let balance_scale = (1.0
            + energy_before.abs()
            + energy_after.abs()
            + external_work.abs()
            + constraint_reaction_work.abs())
        .max(f64::MIN_POSITIVE);
        let relative_energy_balance_residual =
            energy_balance_residual.abs() / balance_scale;

        let linear_after = self.linear_momentum(after);
        let total_force = forces
            .iter()
            .copied()
            .fold([0.0; 3], add);
        let expected_linear = add(linear_before, scale(total_force, dt));
        let linear_residual = sub(linear_after, expected_linear);
        let relative_linear_residual = norm(linear_residual)
            / (1.0 + norm(linear_before) + norm(expected_linear));

        let angular_after = self.angular_momentum(after);
        let mut torque = [0.0; 3];
        for body in 0..BODY_COUNT {
            torque = add(torque, cross(midpoint_positions[body], forces[body]));
        }
        let expected_angular = add(angular_before, scale(torque, dt));
        let angular_residual = sub(angular_after, expected_angular);
        let relative_angular_residual = norm(angular_residual)
            / (1.0
                + norm(angular_before)
                + norm(angular_after)
                + norm(expected_angular));

        let determinant = final_jacobian[0][0] * final_jacobian[1][1]
            - final_jacobian[0][1] * final_jacobian[1][0];
        let row0_norm = (final_jacobian[0][0].powi(2)
            + final_jacobian[0][1].powi(2))
        .sqrt();
        let row1_norm = (final_jacobian[1][0].powi(2)
            + final_jacobian[1][1].powi(2))
        .sqrt();
        let normalized_determinant = determinant.abs()
            / (row0_norm * row1_norm).max(f64::MIN_POSITIVE);

        if !external_work.is_finite()
            || !constraint_reaction_work.is_finite()
            || !energy_balance_residual.is_finite()
            || !relative_energy_balance_residual.is_finite()
            || !relative_linear_residual.is_finite()
            || !relative_angular_residual.is_finite()
            || !determinant.is_finite()
            || !normalized_determinant.is_finite()
            || !final_residuals.iter().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteResult);
        }
        if normalized_determinant <= options.pivot_tolerance {
            return Err(Error::SingularConstraintJacobian);
        }

        Ok(StepReceipt {
            before,
            after,
            midpoint_positions,
            midpoint_velocities: final_midpoint_velocities,
            midpoint_relative_positions,
            midpoint_relative_velocities,
            multipliers,
            multiplier_residuals: final_residuals,
            scaled_multiplier_residuals: [
                relative_midpoint_residuals[0],
                relative_midpoint_residuals[1],
            ],
            multiplier_iterations: iterations,
            multiplier_jacobian_determinant: determinant,
            normalized_multiplier_jacobian_determinant: normalized_determinant,
            energy_before,
            energy_after,
            external_work,
            constraint_reaction_work,
            energy_balance_residual,
            relative_energy_balance_residual,
            link_constraint_residuals: link_residuals,
            relative_link_constraint_residuals: relative_link_residuals,
            midpoint_constraint_residuals,
            relative_midpoint_constraint_residuals: relative_midpoint_residuals,
            endpoint_velocity_constraint_residuals: endpoint_constraint_residuals,
            relative_endpoint_velocity_constraint_residuals: relative_endpoint_residuals,
            linear_momentum_residual: linear_residual,
            relative_linear_momentum_residual: relative_linear_residual,
            angular_momentum_residual: angular_residual,
            relative_angular_momentum_residual: relative_angular_residual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASSES: [f64; BODY_COUNT] = [1.3, 2.1, 0.8];

    fn initial_state() -> State {
        let q1 = [-0.45, 0.10, 0.30];
        let r1 = [0.70, 0.50, 0.30];
        let r2 = [-0.20, 0.80, 0.50];
        let q2 = add(q1, r1);
        let q3 = add(q2, r2);
        let center_velocity = [0.17, -0.23, 0.11];
        let w1 = [0.10, 0.02, -0.26666666666666666];
        let w2 = [0.15, 0.10, -0.10];
        let total_mass = MASSES.iter().sum::<f64>();
        let v1 = sub(
            center_velocity,
            scale(
                add(scale(w1, MASSES[1] + MASSES[2]), scale(w2, MASSES[2])),
                1.0 / total_mass,
            ),
        );
        let v2 = add(v1, w1);
        let v3 = add(v2, w2);
        State {
            q: [q1, q2, q3],
            v: [v1, v2, v3],
        }
    }

    fn lengths(state: State) -> [f64; EDGE_COUNT] {
        [
            norm(sub(state.q[1], state.q[0])),
            norm(sub(state.q[2], state.q[1])),
        ]
    }

    fn model() -> ThreeBodyChain {
        ThreeBodyChain::new(MASSES, lengths(initial_state())).unwrap()
    }

    fn rotate(value: Vec3, axis_angle: Vec3) -> Vec3 {
        let angle = norm(axis_angle);
        if angle < 1.0e-8 {
            let first = cross(axis_angle, value);
            return add(add(value, first), scale(cross(axis_angle, first), 0.5));
        }
        let axis = scale(axis_angle, 1.0 / angle);
        let (sin, cos) = angle.sin_cos();
        add(
            add(scale(value, cos), scale(cross(axis, value), sin)),
            scale(axis, dot(axis, value) * (1.0 - cos)),
        )
    }

    fn rotate_state(state: State, axis_angle: Vec3) -> State {
        let mut rotated = state;
        for body in 0..BODY_COUNT {
            rotated.q[body] = rotate(state.q[body], axis_angle);
            rotated.v[body] = rotate(state.v[body], axis_angle);
        }
        rotated
    }

    fn state_add_scaled(base: State, direction: State, factor: f64) -> State {
        let mut result = base;
        for body in 0..BODY_COUNT {
            result.q[body] = add(base.q[body], scale(direction.q[body], factor));
            result.v[body] = add(base.v[body], scale(direction.v[body], factor));
        }
        result
    }

    fn state_weighted_sum(
        base: State,
        terms: &[(f64, State)],
    ) -> State {
        let mut result = base;
        for body in 0..BODY_COUNT {
            for (weight, term) in terms {
                result.q[body] = add(result.q[body], scale(term.q[body], *weight));
                result.v[body] = add(result.v[body], scale(term.v[body], *weight));
            }
        }
        result
    }

    /// Independent continuous constrained ODE oracle. At each RK stage,
    /// solve the acceleration-level two-link multiplier system
    /// J(q) * lambda = -(|w_e|^2 + r_e·(a_ext,to-a_ext,from)).
    /// This does not reuse the discrete midpoint block/Jacobian.
    fn continuous_rhs(
        state: State,
        forces: [Vec3; BODY_COUNT],
    ) -> Result<State, ()> {
        let mut matrix = [[0.0_f64; EDGE_COUNT]; EDGE_COUNT];
        let mut rhs = [0.0_f64; EDGE_COUNT];
        for edge in 0..EDGE_COUNT {
            let (from, to) = EDGES[edge];
            let relative_position = sub(state.q[to], state.q[from]);
            let relative_velocity = sub(state.v[to], state.v[from]);
            let external_acceleration_difference = sub(
                scale(forces[to], 1.0 / MASSES[to]),
                scale(forces[from], 1.0 / MASSES[from]),
            );
            rhs[edge] = -dot(relative_velocity, relative_velocity)
                - dot(relative_position, external_acceleration_difference);

            for other in 0..EDGE_COUNT {
                let (other_from, other_to) = EDGES[other];
                let incidence_difference =
                    incidence(to, other) / MASSES[to]
                        - incidence(from, other) / MASSES[from];
                let other_relative_position =
                    sub(state.q[other_to], state.q[other_from]);
                matrix[edge][other] = dot(relative_position, other_relative_position)
                    * incidence_difference;
            }
        }

        // Independent closed-form 2x2 solve for the continuous multiplier.
        let determinant =
            matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
        if !determinant.is_finite() || determinant.abs() < 1.0e-12 {
            return Err(());
        }
        let multipliers = [
            (rhs[0] * matrix[1][1] - matrix[0][1] * rhs[1]) / determinant,
            (matrix[0][0] * rhs[1] - rhs[0] * matrix[1][0]) / determinant,
        ];

        let mut derivative = State {
            q: state.v,
            v: [[0.0; 3]; BODY_COUNT],
        };
        for body in 0..BODY_COUNT {
            derivative.v[body] = scale(forces[body], 1.0 / MASSES[body]);
        }
        for edge in 0..EDGE_COUNT {
            let (from, to) = EDGES[edge];
            let relative_position = sub(state.q[to], state.q[from]);
            derivative.v[from] = add(
                derivative.v[from],
                scale(relative_position, multipliers[edge] / MASSES[from]),
            );
            derivative.v[to] = sub(
                derivative.v[to],
                scale(relative_position, multipliers[edge] / MASSES[to]),
            );
        }
        Ok(derivative)
    }

    fn rk4_reference(
        mut state: State,
        forces: [Vec3; BODY_COUNT],
        dt: f64,
        steps: usize,
    ) -> State {
        for _ in 0..steps {
            let k1 = continuous_rhs(state, forces).expect("continuous reference stage 1");
            let k2 = continuous_rhs(
                state_add_scaled(state, k1, 0.5 * dt),
                forces,
            )
            .expect("continuous reference stage 2");
            let k3 = continuous_rhs(
                state_add_scaled(state, k2, 0.5 * dt),
                forces,
            )
            .expect("continuous reference stage 3");
            let k4 = continuous_rhs(
                state_add_scaled(state, k3, dt),
                forces,
            )
            .expect("continuous reference stage 4");

            state = state_weighted_sum(
                state,
                &[
                    (dt / 6.0, k1),
                    (dt / 3.0, k2),
                    (dt / 3.0, k3),
                    (dt / 6.0, k4),
                ],
            );
        }
        state
    }

    fn chain_state_error(left: State, right: State) -> f64 {
        let mut squared = 0.0;
        for body in 0..BODY_COUNT {
            squared += dot(sub(left.q[body], right.q[body]), sub(left.q[body], right.q[body]));
            squared += dot(sub(left.v[body], right.v[body]), sub(left.v[body], right.v[body]));
        }
        squared.sqrt()
    }

    #[test]
    fn coupled_chain_preserves_both_links_and_midpoint_constraints() {
        let model = model();
        let mut state = initial_state();
        let forces = [[0.0; 3]; BODY_COUNT];
        let mut max_link = [0.0_f64; EDGE_COUNT];
        let mut max_midpoint = [0.0_f64; EDGE_COUNT];
        let mut max_endpoint = [0.0_f64; EDGE_COUNT];

        for _ in 0..2000 {
            let receipt = model
                .step(state, forces, 0.002, SolverOptions::default())
                .unwrap();
            for edge in 0..EDGE_COUNT {
                max_link[edge] =
                    max_link[edge].max(receipt.relative_link_constraint_residuals[edge]);
                max_midpoint[edge] = max_midpoint[edge]
                    .max(receipt.relative_midpoint_constraint_residuals[edge]);
                max_endpoint[edge] = max_endpoint[edge]
                    .max(receipt.relative_endpoint_velocity_constraint_residuals[edge]);
            }
            assert!(receipt.normalized_multiplier_jacobian_determinant > 1.0e-8);
            state = receipt.after;
        }
        for edge in 0..EDGE_COUNT {
            assert!(max_link[edge] < 5.0e-11);
            assert!(max_midpoint[edge] < 5.0e-11);
            assert!(max_endpoint[edge] < 2.0e-8);
        }
    }

    #[test]
    fn coupled_chain_is_second_order_against_independent_constrained_rk4() {
        let model = model();
        let initial = initial_state();
        let forces = [[0.0; 3]; BODY_COUNT];
        let reference = rk4_reference(initial, forces, 1.0 / 40_000.0, 40_000);
        let mut errors = Vec::new();

        for dt in [0.02, 0.01, 0.005, 0.0025] {
            let mut state = initial;
            for _ in 0..(1.0 / dt).round() as usize {
                state = model
                    .step(state, forces, dt, SolverOptions::default())
                    .unwrap()
                    .after;
            }
            errors.push(chain_state_error(state, reference));
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (3.8..4.2).contains(&ratio),
                "coupled chain must converge at second order: {errors:?}"
            );
        }
    }

    #[test]
    fn coupled_chain_preserves_energy_and_momentum_maps() {
        let model = model();
        let mut state = initial_state();
        let forces = [[0.0; 3]; BODY_COUNT];
        let energy0 = model.hamiltonian(state).unwrap();
        let linear0 = model.linear_momentum(state);
        let angular0 = model.angular_momentum(state);
        let mut max_energy_balance = 0.0_f64;
        let mut max_linear = 0.0_f64;
        let mut max_angular = 0.0_f64;

        for _ in 0..2000 {
            let receipt = model
                .step(state, forces, 0.002, SolverOptions::default())
                .unwrap();
            max_energy_balance =
                max_energy_balance.max(receipt.relative_energy_balance_residual);
            max_linear = max_linear.max(receipt.relative_linear_momentum_residual);
            max_angular = max_angular.max(receipt.relative_angular_momentum_residual);
            state = receipt.after;
        }
        let energy_drift =
            (model.hamiltonian(state).unwrap() - energy0).abs() / energy0.abs();
        let linear_drift = norm(sub(model.linear_momentum(state), linear0))
            / (1.0 + norm(linear0));
        let angular_drift = norm(sub(model.angular_momentum(state), angular0))
            / (1.0 + norm(angular0));

        assert!(max_energy_balance < 1.0e-10);
        assert!(energy_drift < 1.0e-9);
        assert!(max_linear < 1.0e-10);
        assert!(linear_drift < 1.0e-10);
        assert!(max_angular < 1.0e-10);
        assert!(angular_drift < 1.0e-9);
    }

    #[test]
    fn external_force_work_and_momentum_vector_ledger_close() {
        let model = model();
        let state = initial_state();
        let forces = [
            [0.12, -0.07, 0.05],
            [-0.03, 0.09, 0.11],
            [0.04, 0.02, -0.08],
        ];
        let dt = 0.01;
        let receipt = model
            .step(state, forces, dt, SolverOptions::default())
            .unwrap();
        assert!(receipt.closes_energy_balance_within(3.0e-12));
        assert!(receipt.relative_linear_momentum_residual < 3.0e-12);
        assert!(receipt.relative_angular_momentum_residual < 3.0e-12);
        assert!(receipt.constraint_reaction_work.abs() < 3.0e-12);
        assert!(receipt.midpoint_constraint_residuals.iter().all(|x| x.abs() < 3.0e-12));

        let expected_impulse = forces.iter().copied().fold([0.0; 3], add);
        let actual_impulse = sub(
            model.linear_momentum(receipt.after),
            model.linear_momentum(state),
        );
        assert!(norm(sub(actual_impulse, scale(expected_impulse, dt))) < 3.0e-12);
    }

    #[test]
    fn equal_mass_chain_control_keeps_coupled_constraints_well_defined() {
        let masses = [1.0; BODY_COUNT];
        let state = initial_state();
        let model = ThreeBodyChain::new(masses, lengths(state)).unwrap();
        let receipt = model
            .step(state, [[0.0; 3]; BODY_COUNT], 0.01, SolverOptions::default())
            .unwrap();
        assert!(receipt.normalized_multiplier_jacobian_determinant > 1.0e-8);
        assert!(receipt.relative_link_constraint_residuals.iter().all(|x| *x < 2.0e-10));
    }

    #[test]
    fn coupled_multiplier_jacobian_is_so3_equivariant() {
        let model = model();
        let state = initial_state();
        let forces = [
            [0.12, -0.07, 0.05],
            [-0.03, 0.09, 0.11],
            [0.04, 0.02, -0.08],
        ];
        let normal = model
            .step(state, forces, 0.01, SolverOptions::default())
            .unwrap();

        for index in 0..20 {
            let raw = [
                (0.37 * (index as f64 + 1.0)).sin(),
                (0.23 * (index as f64 + 2.0)).cos(),
                (0.41 * (index as f64 + 3.0)).sin(),
            ];
            let axis = scale(raw, 0.173 * (index as f64 + 1.0) / norm(raw));
            let rotated_state = rotate_state(state, axis);
            let rotated_forces = [
                rotate(forces[0], axis),
                rotate(forces[1], axis),
                rotate(forces[2], axis),
            ];
            let rotated = model
                .step(
                    rotated_state,
                    rotated_forces,
                    0.01,
                    SolverOptions::default(),
                )
                .unwrap();
            let expected = rotate_state(normal.after, axis);
            for body in 0..BODY_COUNT {
                assert!(norm(sub(rotated.after.q[body], expected.q[body])) < 2.0e-10);
                assert!(norm(sub(rotated.after.v[body], expected.v[body])) < 2.0e-10);
            }
            assert!(
                (rotated.multipliers[0] - normal.multipliers[0]).abs() < 2.0e-10
            );
            assert!(
                (rotated.multipliers[1] - normal.multipliers[1]).abs() < 2.0e-10
            );
        }
    }

    #[test]
    fn galilean_boost_commutes_with_the_coupled_chain_step() {
        let model = model();
        let state = initial_state();
        let boost = [0.7, -0.4, 0.23];
        let forces = [[0.0; 3]; BODY_COUNT];
        let normal = model
            .step(state, forces, 0.01, SolverOptions::default())
            .unwrap();
        let mut boosted_state = state;
        for body in 0..BODY_COUNT {
            boosted_state.v[body] = add(state.v[body], boost);
        }
        let boosted = model
            .step(boosted_state, forces, 0.01, SolverOptions::default())
            .unwrap();
        for body in 0..BODY_COUNT {
            assert!(
                norm(sub(
                    boosted.after.q[body],
                    add(normal.after.q[body], scale(boost, 0.01))
                )) < 2.0e-10
            );
            assert!(
                norm(sub(
                    boosted.after.v[body],
                    add(normal.after.v[body], boost)
                )) < 2.0e-10
            );
        }
    }

    #[test]
    fn invalid_link_nonfinite_force_and_solver_failure_fail_closed() {
        let model = model();
        let mut invalid = initial_state();
        invalid.q[1][0] += 0.1;
        assert_eq!(
            model
                .step(invalid, [[0.0; 3]; BODY_COUNT], 0.01, SolverOptions::default())
                .unwrap_err(),
            Error::InvalidInitialLinkConstraint { edge: 0 }
        );
        let mut radial = initial_state();
        radial.v[1] = add(radial.v[1], [0.1, 0.0, 0.0]);
        assert_eq!(
            model
                .step(radial, [[0.0; 3]; BODY_COUNT], 0.01, SolverOptions::default())
                .unwrap_err(),
            Error::InvalidInitialVelocityConstraint { edge: 0 }
        );
        let mut bad_forces = [[0.0; 3]; BODY_COUNT];
        bad_forces[2][1] = f64::NAN;
        assert_eq!(
            model
                .step(initial_state(), bad_forces, 0.01, SolverOptions::default())
                .unwrap_err(),
            Error::NonFiniteInput
        );
        assert_eq!(
            model
                .step(initial_state(), [[0.0; 3]; BODY_COUNT], 0.0, SolverOptions::default())
                .unwrap_err(),
            Error::InvalidTimeStep
        );
        assert_eq!(
            model
                .step(
                    initial_state(),
                    [[0.0; 3]; BODY_COUNT],
                    0.01,
                    SolverOptions {
                        relative_tolerance: 1.0e-30,
                        max_iterations: 1,
                        max_backtracks: 1,
                        ..SolverOptions::default()
                    }
                )
                .unwrap_err(),
            Error::NoConvergence
        );
    }
}

#[cfg(not(test))]
fn main() {
    let state = State {
        q: [
            [-0.45, 0.10, 0.30],
            [0.25, 0.60, 0.60],
            [0.05, 1.40, 1.10],
        ],
        v: [
            [0.0, 0.0, 0.0],
            [0.10, 0.02, -0.26666666666666666],
            [0.25, 0.12, -0.36666666666666664],
        ],
    };
    let lengths = [
        norm(sub(state.q[1], state.q[0])),
        norm(sub(state.q[2], state.q[1])),
    ];
    let model =
        ThreeBodyChain::new([1.3, 2.1, 0.8], lengths).expect("frozen chain must be valid");
    let mut state = state;
    println!("step,energy,link1_residual,link2_residual,midpoint1,midpoint2,endpoint1,endpoint2,linear_momentum_residual,angular_momentum_residual,jacobian_det,normalized_jacobian_det,multiplier_iterations");
    for step in 0..500 {
        let receipt = model
            .step(state, [[0.0; 3]; BODY_COUNT], 0.002, SolverOptions::default())
            .expect("frozen chain step must converge");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{}",
            step + 1,
            receipt.energy_after,
            receipt.relative_link_constraint_residuals[0],
            receipt.relative_link_constraint_residuals[1],
            receipt.relative_midpoint_constraint_residuals[0],
            receipt.relative_midpoint_constraint_residuals[1],
            receipt.relative_endpoint_velocity_constraint_residuals[0],
            receipt.relative_endpoint_velocity_constraint_residuals[1],
            receipt.relative_linear_momentum_residual,
            receipt.relative_angular_momentum_residual,
            receipt.multiplier_jacobian_determinant,
            receipt.normalized_multiplier_jacobian_determinant,
            receipt.multiplier_iterations
        );
    }
}
