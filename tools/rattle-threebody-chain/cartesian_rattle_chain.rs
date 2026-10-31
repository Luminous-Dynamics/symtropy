// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Independent Cartesian SHAKE/RATTLE baseline for a three-body, two-link
// spatial open chain. This does not reuse the pHDAE midpoint block solve.
//
// The position stage solves both SHAKE constraints together from the
// endpoint position correction. The momentum stage then applies the
// independent RATTLE endpoint tangent projection.

use std::f64;

#[allow(dead_code)]
#[path = "../constraint-graph-rank/constraint_graph_rank.rs"]
pub mod rank;

pub type Vec3 = [f64; 3];
const BODY_COUNT: usize = 3;
const EDGE_COUNT: usize = 2;
const EDGES: [(usize, usize); EDGE_COUNT] = [(0, 1), (1, 2)];
const BODY_IDS: [u64; BODY_COUNT] = [10, 20, 30];
const EDGE_IDS: [u64; EDGE_COUNT] = [101, 202];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    InvalidParameter,
    InvalidSolverOptions,
    InvalidTimeStep,
    InvalidInitialLinkConstraint { edge: usize },
    InvalidInitialVelocityConstraint { edge: usize },
    NonFiniteInput,
    RankPreflight(rank::GraphError),
    RankDeficientGeometry { rank: usize },
    IllConditionedGeometry { reciprocal_condition: f64 },
    SingularPositionJacobian,
    SingularVelocityGram,
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
    pub rank_policy: rank::RankPolicy,
}

impl Default for SolverOptions {
    fn default() -> Self {
        Self {
            relative_tolerance: 1.0e-13,
            max_iterations: 32,
            max_backtracks: 24,
            pivot_tolerance: 1.0e-13,
            rank_policy: rank::RankPolicy::default(),
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
        self.rank_policy.validate().map_err(Error::RankPreflight)?;
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CartesianRattleChain {
    masses: [f64; BODY_COUNT],
    lengths: [f64; EDGE_COUNT],
    constraint_tolerance: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct StepReceipt {
    pub before: State,
    pub after: State,
    /// SHAKE position correction multipliers, one per stable edge ID.
    pub position_multipliers: [f64; EDGE_COUNT],
    /// RATTLE endpoint velocity projection multipliers.
    pub velocity_multipliers: [f64; EDGE_COUNT],
    pub position_newton_residuals: [f64; EDGE_COUNT],
    pub position_jacobian: [[f64; EDGE_COUNT]; EDGE_COUNT],
    pub position_jacobian_rcond: f64,
    pub geometric_rank: usize,
    pub geometric_rcond: f64,
    pub newton_iterations: usize,
    pub backtracks: usize,
    pub energy_before: f64,
    pub energy_after: f64,
    /// This is a numerical conservative-energy defect, not dissipation.
    pub energy_defect: f64,
    pub relative_energy_defect: f64,
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

impl StepReceipt {
    pub fn preserves_links_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_link_constraint_residuals
                .iter()
                .all(|residual| residual.is_finite() && *residual <= tolerance)
    }

    pub fn preserves_endpoint_tangency_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_endpoint_velocity_constraint_residuals
                .iter()
                .all(|residual| residual.is_finite() && *residual <= tolerance)
    }
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

fn rotate(value: Vec3, rotation_vector: Vec3) -> Vec3 {
    let angle = norm(rotation_vector);
    if angle < 1.0e-8 {
        let first = cross(rotation_vector, value);
        return add(add(value, first), scale(cross(rotation_vector, first), 0.5));
    }
    let axis = scale(rotation_vector, 1.0 / angle);
    let (sin, cos) = angle.sin_cos();
    add(
        add(scale(value, cos), scale(cross(axis, value), sin)),
        scale(axis, dot(axis, value) * (1.0 - cos)),
    )
}

fn edge_vector(state: State, edge: usize) -> Vec3 {
    let (from, to) = EDGES[edge];
    sub(state.q[to], state.q[from])
}

fn edge_velocity(state: State, edge: usize) -> Vec3 {
    let (from, to) = EDGES[edge];
    sub(state.v[to], state.v[from])
}

fn edge_gradient(edge: usize, body: usize, relative_position: Vec3) -> Vec3 {
    let (from, to) = EDGES[edge];
    if body == from {
        scale(relative_position, -1.0)
    } else if body == to {
        relative_position
    } else {
        [0.0; 3]
    }
}

fn gram_matrix(
    positions: [Vec3; BODY_COUNT],
    masses: [f64; BODY_COUNT],
    edges: [usize; EDGE_COUNT],
) -> [[f64; EDGE_COUNT]; EDGE_COUNT] {
    let mut gram = [[0.0; EDGE_COUNT]; EDGE_COUNT];
    for row in 0..EDGE_COUNT {
        let relative_row = sub(positions[EDGES[edges[row]].1], positions[EDGES[edges[row]].0]);
        for column in 0..EDGE_COUNT {
            let relative_column = sub(
                positions[EDGES[edges[column]].1],
                positions[EDGES[edges[column]].0],
            );
            for body in 0..BODY_COUNT {
                gram[row][column] += dot(
                    edge_gradient(edges[row], body, relative_row),
                    edge_gradient(edges[column], body, relative_column),
                ) / masses[body];
            }
        }
    }
    gram
}

fn solve_2x2(
    matrix: [[f64; 2]; 2],
    rhs: [f64; 2],
    pivot_tolerance: f64,
    error: Error,
) -> Result<[f64; 2], Error> {
    if !matrix.iter().flatten().all(|value| value.is_finite())
        || !rhs.iter().all(|value| value.is_finite())
    {
        return Err(Error::NonFiniteResult);
    }
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
        .max(f64::MIN_POSITIVE);
    let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
    if !determinant.is_finite() || determinant.abs() <= pivot_tolerance * scale * scale {
        return Err(error);
    }
    let solution = [
        (rhs[0] * matrix[1][1] - matrix[0][1] * rhs[1]) / determinant,
        (matrix[0][0] * rhs[1] - rhs[0] * matrix[1][0]) / determinant,
    ];
    if solution.iter().all(|value| value.is_finite()) {
        Ok(solution)
    } else {
        Err(Error::NonFiniteResult)
    }
}

fn matrix_rcond_2x2(matrix: [[f64; 2]; 2]) -> f64 {
    let a = matrix[0][0] * matrix[0][0] + matrix[1][0] * matrix[1][0];
    let b = matrix[0][0] * matrix[0][1] + matrix[1][0] * matrix[1][1];
    let d = matrix[0][1] * matrix[0][1] + matrix[1][1] * matrix[1][1];
    let trace = a + d;
    let determinant = (a * d - b * b).max(0.0);
    let discriminant = (trace * trace - 4.0 * determinant).max(0.0).sqrt();
    let high = 0.5 * (trace + discriminant);
    let low = 0.5 * (trace - discriminant).max(0.0);
    if high <= f64::MIN_POSITIVE {
        0.0
    } else {
        (low / high).sqrt()
    }
}

impl CartesianRattleChain {
    pub fn new(
        masses: [f64; BODY_COUNT],
        lengths: [f64; EDGE_COUNT],
    ) -> Result<Self, Error> {
        if !masses.iter().all(|value| value.is_finite())
            || !lengths.iter().all(|value| value.is_finite())
            || masses.iter().any(|value| *value <= 0.0)
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

    fn graph_receipt(
        &self,
        state: State,
        policy: rank::RankPolicy,
        revision: u64,
    ) -> Result<rank::GeometryRankReceipt, Error> {
        let bodies = [
            rank::GraphBody {
                id: BODY_IDS[0],
                mass: self.masses[0],
                position: state.q[0],
            },
            rank::GraphBody {
                id: BODY_IDS[1],
                mass: self.masses[1],
                position: state.q[1],
            },
            rank::GraphBody {
                id: BODY_IDS[2],
                mass: self.masses[2],
                position: state.q[2],
            },
        ];
        let edges = vec![
            rank::DistanceEdge {
                id: EDGE_IDS[0],
                from_body_id: BODY_IDS[0],
                to_body_id: BODY_IDS[1],
                length: self.lengths[0],
            },
            rank::DistanceEdge {
                id: EDGE_IDS[1],
                from_body_id: BODY_IDS[1],
                to_body_id: BODY_IDS[2],
                length: self.lengths[1],
            },
        ];
        let graph =
            rank::ConstraintGraph3::new(revision, bodies, edges, policy)
                .map_err(Error::RankPreflight)?;
        let receipt = graph.assess_geometry().map_err(Error::RankPreflight)?;
        match receipt.status {
            rank::GeometryStatus::FullRank => Ok(receipt),
            rank::GeometryStatus::IllConditioned => Err(Error::IllConditionedGeometry {
                reciprocal_condition: receipt.reciprocal_condition_indicator,
            }),
            rank::GeometryStatus::RankDeficient => Err(Error::RankDeficientGeometry {
                rank: receipt.numerical_rank,
            }),
        }
    }

    fn validate_initial(
        &self,
        state: State,
        options: SolverOptions,
    ) -> Result<rank::GeometryRankReceipt, Error> {
        if !state.q.iter().flatten().all(|value| value.is_finite())
            || !state.v.iter().flatten().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        let graph = self.graph_receipt(state, options.rank_policy, 0)?;
        for edge in 0..EDGE_COUNT {
            let relative_position = edge_vector(state, edge);
            let relative_velocity = edge_velocity(state, edge);
            let position_residual =
                (dot(relative_position, relative_position) - self.lengths[edge].powi(2)).abs();
            let position_scale = 1.0
                + dot(relative_position, relative_position)
                + self.lengths[edge].powi(2);
            if position_residual > self.constraint_tolerance * position_scale {
                return Err(Error::InvalidInitialLinkConstraint { edge });
            }
            let tangent_residual = dot(relative_position, relative_velocity).abs();
            let tangent_scale = 1.0 + self.lengths[edge] * norm(relative_velocity);
            if tangent_residual > self.constraint_tolerance * tangent_scale {
                return Err(Error::InvalidInitialVelocityConstraint { edge });
            }
        }
        Ok(graph)
    }

    /// Build q_free=q_n+h v_n and the exact derivative of every SHAKE
    /// squared-distance residual with respect to the coupled 2-vector lambda.
    fn shake_trial(
        &self,
        before: State,
        dt: f64,
        position_multipliers: [f64; EDGE_COUNT],
    ) -> (State, [f64; EDGE_COUNT], [[f64; EDGE_COUNT]; EDGE_COUNT]) {
        let mut after = before;
        for body in 0..BODY_COUNT {
            after.q[body] = add(before.q[body], scale(before.v[body], dt));
            after.v[body] = before.v[body];
        }
        for edge in 0..EDGE_COUNT {
            let relative_position = edge_vector(before, edge);
            let (from, to) = EDGES[edge];
            let impulse = scale(relative_position, 0.5 * dt * dt * position_multipliers[edge]);
            after.q[from] = add(after.q[from], scale(impulse, 1.0 / self.masses[from]));
            after.q[to] = sub(after.q[to], scale(impulse, 1.0 / self.masses[to]));
        }

        let mut residuals = [0.0; EDGE_COUNT];
        let mut jacobian = [[0.0; EDGE_COUNT]; EDGE_COUNT];
        let after_positions = after.q;

        for row in 0..EDGE_COUNT {
            let r_plus = edge_vector(after, row);
            residuals[row] = 0.5 * (dot(r_plus, r_plus) - self.lengths[row] * self.lengths[row]);
            for column in 0..EDGE_COUNT {
                let r0 = edge_vector(before, column);
                let mut contraction = 0.0;
                for body in 0..BODY_COUNT {
                    let g_row = edge_gradient(row, body, r_plus);
                    let g_column = edge_gradient(column, body, r0);
                    contraction += dot(g_row, g_column) / self.masses[body];
                }
                jacobian[row][column] = -0.5 * dt * dt * contraction;
            }
        }
        let _ = after_positions;
        (after, residuals, jacobian)
    }

    fn scaled_position_residual(&self, residuals: [f64; EDGE_COUNT]) -> f64 {
        let mut sum = 0.0;
        for edge in 0..EDGE_COUNT {
            let scaled = residuals[edge]
                / (1.0 + self.lengths[edge] * self.lengths[edge]);
            sum += scaled * scaled;
        }
        sum.sqrt()
    }

    fn solve_position_stage(
        &self,
        before: State,
        dt: f64,
        options: SolverOptions,
    ) -> Result<(State, [f64; EDGE_COUNT], [[f64; EDGE_COUNT]; EDGE_COUNT], usize, usize), Error> {
        // Acceleration-level multiplier estimate: C(q) lambda = |w_e|^2.
        let geometry = gram_matrix(before.q, self.masses, [0, 1]);
        let mut rhs = [0.0; EDGE_COUNT];
        for edge in 0..EDGE_COUNT {
            rhs[edge] = dot(edge_velocity(before, edge), edge_velocity(before, edge));
        }
        let mut multipliers = solve_2x2(
            geometry,
            rhs,
            options.pivot_tolerance,
            Error::SingularPositionJacobian,
        )?;
        let mut iterations = 0usize;
        let mut backtracks = 0usize;
        let mut converged = false;
        let mut final_after = before;
        let mut final_residuals = [0.0; EDGE_COUNT];
        let mut final_jacobian = [[0.0; EDGE_COUNT]; EDGE_COUNT];

        for iteration in 0..options.max_iterations {
            let (candidate_state, residuals, jacobian) =
                self.shake_trial(before, dt, multipliers);
            let residual_norm = self.scaled_position_residual(residuals);
            final_after = candidate_state;
            final_residuals = residuals;
            final_jacobian = jacobian;
            if residual_norm <= options.relative_tolerance {
                iterations = iteration;
                converged = true;
                break;
            }

            let delta = solve_2x2(
                jacobian,
                [-residuals[0], -residuals[1]],
                options.pivot_tolerance,
                Error::SingularPositionJacobian,
            )?;
            let mut fraction = 1.0;
            let mut accepted = false;
            for _ in 0..options.max_backtracks {
                let trial = [
                    multipliers[0] + fraction * delta[0],
                    multipliers[1] + fraction * delta[1],
                ];
                if trial.iter().all(|value| value.is_finite()) {
                    let (trial_state, trial_residuals, _) =
                        self.shake_trial(before, dt, trial);
                    let trial_norm = self.scaled_position_residual(trial_residuals);
                    if trial_norm < residual_norm
                        || trial_norm <= options.relative_tolerance
                    {
                        multipliers = trial;
                        accepted = true;
                        break;
                    }
                }
                fraction *= 0.5;
                backtracks += 1;
            }
            if !accepted {
                return Err(Error::NoConvergence);
            }
            iterations = iteration + 1;
        }

        if !converged {
            let (candidate_state, residuals, jacobian) =
                self.shake_trial(before, dt, multipliers);
            final_after = candidate_state;
            final_residuals = residuals;
            final_jacobian = jacobian;
            converged = self.scaled_position_residual(residuals)
                <= options.relative_tolerance;
        }
        if !converged {
            return Err(Error::NoConvergence);
        }
        Ok((
            final_after,
            multipliers,
            final_jacobian,
            iterations,
            backtracks,
        ))
    }

    /// Standard RATTLE endpoint projection: p_{n+1}=p_{n+1/2}-(h/2)G(q_{n+1})^T mu,
    /// with mu chosen so G(q_{n+1})M^{-1}p_{n+1}=0.
    fn project_endpoint_momentum(
        &self,
        before: State,
        after_positions: [Vec3; BODY_COUNT],
        position_multipliers: [f64; EDGE_COUNT],
        dt: f64,
        options: SolverOptions,
    ) -> Result<(State, [f64; EDGE_COUNT]), Error> {
        let mut half_momentum = [[0.0; 3]; BODY_COUNT];
        for body in 0..BODY_COUNT {
            half_momentum[body] = scale(before.v[body], self.masses[body]);
        }
        for edge in 0..EDGE_COUNT {
            let r0 = edge_vector(before, edge);
            let (from, to) = EDGES[edge];
            let impulse = scale(r0, 0.5 * dt * position_multipliers[edge]);
            half_momentum[from] = add(half_momentum[from], impulse);
            half_momentum[to] = sub(half_momentum[to], impulse);
        }

        let mut after = State {
            q: after_positions,
            v: [[0.0; 3]; BODY_COUNT],
        };
        let gram = gram_matrix(after.q, self.masses, [0, 1]);
        let mut rhs = [0.0; EDGE_COUNT];
        for edge in 0..EDGE_COUNT {
            let r_plus = edge_vector(after, edge);
            let (from, to) = EDGES[edge];
            let half_velocity = sub(
                scale(half_momentum[to], 1.0 / self.masses[to]),
                scale(half_momentum[from], 1.0 / self.masses[from]),
            );
            rhs[edge] = 2.0 * dot(r_plus, half_velocity) / dt;
        }
        let velocity_multipliers = solve_2x2(
            gram,
            rhs,
            options.pivot_tolerance,
            Error::SingularVelocityGram,
        )?;

        let mut momentum = half_momentum;
        for edge in 0..EDGE_COUNT {
            let r_plus = edge_vector(after, edge);
            let (from, to) = EDGES[edge];
            let impulse = scale(r_plus, 0.5 * dt * velocity_multipliers[edge]);
            momentum[from] = add(momentum[from], impulse);
            momentum[to] = sub(momentum[to], impulse);
        }
        for body in 0..BODY_COUNT {
            after.v[body] = scale(momentum[body], 1.0 / self.masses[body]);
        }
        Ok((after, velocity_multipliers))
    }

    pub fn step(
        &self,
        before: State,
        dt: f64,
        options: SolverOptions,
    ) -> Result<StepReceipt, Error> {
        let options = options.validate()?;
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        let geometry_before = self.validate_initial(before, options)?;
        let energy_before = self.hamiltonian(before)?;
        let linear_before = self.linear_momentum(before);
        let angular_before = self.angular_momentum(before);

        let (position_state, position_multipliers, position_jacobian, iterations, backtracks) =
            self.solve_position_stage(before, dt, options)?;
        let (after, velocity_multipliers) = self.project_endpoint_momentum(
            before,
            position_state.q,
            position_multipliers,
            dt,
            options,
        )?;
        let geometry_after = self.graph_receipt(after, options.rank_policy, 1)?;

        let mut link_residuals = [0.0; EDGE_COUNT];
        let mut relative_link_residuals = [0.0; EDGE_COUNT];
        let mut midpoint_residuals = [0.0; EDGE_COUNT];
        let mut relative_midpoint_residuals = [0.0; EDGE_COUNT];
        let mut endpoint_residuals = [0.0; EDGE_COUNT];
        let mut relative_endpoint_residuals = [0.0; EDGE_COUNT];
        for edge in 0..EDGE_COUNT {
            let r0 = edge_vector(before, edge);
            let r1 = edge_vector(after, edge);
            let w0 = edge_velocity(before, edge);
            let w1 = edge_velocity(after, edge);
            link_residuals[edge] = dot(r1, r1) - self.lengths[edge] * self.lengths[edge];
            relative_link_residuals[edge] = link_residuals[edge].abs()
                / (1.0 + dot(r1, r1) + self.lengths[edge] * self.lengths[edge]);
            let rbar = scale(add(r0, r1), 0.5);
            let wbar = scale(add(w0, w1), 0.5);
            midpoint_residuals[edge] = dot(rbar, wbar);
            relative_midpoint_residuals[edge] = midpoint_residuals[edge].abs()
                / (1.0 + norm(rbar) * norm(wbar));
            endpoint_residuals[edge] = dot(r1, w1);
            relative_endpoint_residuals[edge] = endpoint_residuals[edge].abs()
                / (1.0 + self.lengths[edge] * norm(w1));
        }

        if relative_link_residuals.iter().any(|value| !value.is_finite())
            || relative_midpoint_residuals.iter().any(|value| !value.is_finite())
            || relative_endpoint_residuals.iter().any(|value| !value.is_finite())
        {
            return Err(Error::NonFiniteResult);
        }
        if relative_link_residuals.iter().any(|value| *value > self.constraint_tolerance)
            || relative_endpoint_residuals.iter().any(|value| *value > self.constraint_tolerance)
        {
            return Err(Error::NoConvergence);
        }

        let energy_after = self.hamiltonian(after)?;
        let energy_defect = energy_after - energy_before;
        let relative_energy_defect = energy_defect.abs()
            / (1.0 + energy_before.abs() + energy_after.abs());
        let linear_after = self.linear_momentum(after);
        let linear_residual = sub(linear_after, linear_before);
        let relative_linear_residual = norm(linear_residual)
            / (1.0 + norm(linear_before) + norm(linear_after));
        let angular_after = self.angular_momentum(after);
        let angular_residual = sub(angular_after, angular_before);
        let relative_angular_residual = norm(angular_residual)
            / (1.0 + norm(angular_before) + norm(angular_after));
        let position_jacobian_rcond = matrix_rcond_2x2(position_jacobian);

        if !energy_defect.is_finite()
            || !relative_energy_defect.is_finite()
            || !relative_linear_residual.is_finite()
            || !relative_angular_residual.is_finite()
            || !position_jacobian_rcond.is_finite()
        {
            return Err(Error::NonFiniteResult);
        }

        Ok(StepReceipt {
            before,
            after,
            position_multipliers,
            velocity_multipliers,
            position_newton_residuals: [
                final_constraint_residual(&position_state, self.lengths, 0),
                final_constraint_residual(&position_state, self.lengths, 1),
            ],
            position_jacobian,
            position_jacobian_rcond,
            geometric_rank: geometry_before.numerical_rank.min(geometry_after.numerical_rank),
            geometric_rcond: geometry_before.reciprocal_condition_indicator
                .min(geometry_after.reciprocal_condition_indicator),
            newton_iterations: iterations,
            backtracks,
            energy_before,
            energy_after,
            energy_defect,
            relative_energy_defect,
            link_constraint_residuals: link_residuals,
            relative_link_constraint_residuals: relative_link_residuals,
            midpoint_constraint_residuals: midpoint_residuals,
            relative_midpoint_constraint_residuals: relative_midpoint_residuals,
            endpoint_velocity_constraint_residuals: endpoint_residuals,
            relative_endpoint_velocity_constraint_residuals: relative_endpoint_residuals,
            linear_momentum_residual: linear_residual,
            relative_linear_momentum_residual: relative_linear_residual,
            angular_momentum_residual: angular_residual,
            relative_angular_momentum_residual: relative_angular_residual,
        })
    }
}

fn final_constraint_residual(
    state: &State,
    lengths: [f64; EDGE_COUNT],
    edge: usize,
) -> f64 {
    let r = edge_vector(*state, edge);
    0.5 * (dot(r, r) - lengths[edge] * lengths[edge])
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASSES: [f64; BODY_COUNT] = [1.3, 2.1, 0.8];

    fn initial_state() -> State {
        State {
            q: [
                [-0.45, 0.10, 0.30],
                [0.25, 0.60, 0.60],
                [0.05, 1.40, 1.10],
            ],
            v: [
                [0.0723809523809524, -0.262857142857143, 0.313174603174603],
                [0.1723809523809524, -0.242857142857143, 0.0465079365079363],
                [0.3223809523809524, -0.142857142857143, -0.0534920634920637],
            ],
        }
    }

    fn lengths(state: State) -> [f64; EDGE_COUNT] {
        [norm(edge_vector(state, 0)), norm(edge_vector(state, 1))]
    }

    fn model() -> CartesianRattleChain {
        CartesianRattleChain::new(MASSES, lengths(initial_state())).unwrap()
    }

    fn state_error(a: State, b: State) -> f64 {
        let mut error = 0.0;
        for body in 0..BODY_COUNT {
            error += dot(sub(a.q[body], b.q[body]), sub(a.q[body], b.q[body]));
            error += dot(sub(a.v[body], b.v[body]), sub(a.v[body], b.v[body]));
        }
        error.sqrt()
    }

    fn rotate_state(state: State, rotation: Vec3) -> State {
        let mut result = state;
        for body in 0..BODY_COUNT {
            result.q[body] = rotate(state.q[body], rotation);
            result.v[body] = rotate(state.v[body], rotation);
        }
        result
    }

    fn continuous_rhs(state: State, masses: [f64; BODY_COUNT]) -> State {
        let mut gram = [[0.0; EDGE_COUNT]; EDGE_COUNT];
        let mut rhs = [0.0; EDGE_COUNT];
        for row in 0..EDGE_COUNT {
            let wr = edge_velocity(state, row);
            rhs[row] = dot(wr, wr);
            for column in 0..EDGE_COUNT {
                let wr0 = edge_vector(state, row);
                let wc0 = edge_vector(state, column);
                let mut value = 0.0;
                for body in 0..BODY_COUNT {
                    value += dot(
                        edge_gradient(row, body, wr0),
                        edge_gradient(column, body, wc0),
                    ) / masses[body];
                }
                gram[row][column] = value;
            }
        }
        let lambda = solve_2x2(
            gram,
            rhs,
            1.0e-14,
            Error::SingularVelocityGram,
        )
        .unwrap();
        let mut acceleration = [[0.0; 3]; BODY_COUNT];
        for body in 0..BODY_COUNT {
            for edge in 0..EDGE_COUNT {
                acceleration[body] = sub(
                    acceleration[body],
                    scale(
                        edge_gradient(edge, body, edge_vector(state, edge)),
                        lambda[edge] / masses[body],
                    ),
                );
            }
        }
        State {
            q: state.v,
            v: acceleration,
        }
    }

    fn state_add_scaled(state: State, derivative: State, factor: f64) -> State {
        let mut result = state;
        for body in 0..BODY_COUNT {
            result.q[body] = add(state.q[body], scale(derivative.q[body], factor));
            result.v[body] = add(state.v[body], scale(derivative.v[body], factor));
        }
        result
    }

    fn rk4_reference(
        mut state: State,
        masses: [f64; BODY_COUNT],
        dt: f64,
        steps: usize,
    ) -> State {
        for _ in 0..steps {
            let k1 = continuous_rhs(state, masses);
            let k2 = continuous_rhs(state_add_scaled(state, k1, 0.5 * dt), masses);
            let k3 = continuous_rhs(state_add_scaled(state, k2, 0.5 * dt), masses);
            let k4 = continuous_rhs(state_add_scaled(state, k3, dt), masses);
            for body in 0..BODY_COUNT {
                state.q[body] = add(
                    state.q[body],
                    scale(
                        add(
                            add(k1.q[body], scale(k2.q[body], 2.0)),
                            add(scale(k3.q[body], 2.0), k4.q[body]),
                        ),
                        dt / 6.0,
                    ),
                );
                state.v[body] = add(
                    state.v[body],
                    scale(
                        add(
                            add(k1.v[body], scale(k2.v[body], 2.0)),
                            add(scale(k3.v[body], 2.0), k4.v[body]),
                        ),
                        dt / 6.0,
                    ),
                );
            }
        }
        state
    }

    #[test]
    fn rattle_closes_both_links_and_endpoint_tangent_constraints() {
        let model = model();
        let mut state = initial_state();
        let mut max_link = [0.0_f64; EDGE_COUNT];
        let mut max_endpoint = [0.0_f64; EDGE_COUNT];
        for _ in 0..2000 {
            let receipt = model.step(state, 0.002, SolverOptions::default()).unwrap();
            for edge in 0..EDGE_COUNT {
                max_link[edge] =
                    max_link[edge].max(receipt.relative_link_constraint_residuals[edge]);
                max_endpoint[edge] = max_endpoint[edge]
                    .max(receipt.relative_endpoint_velocity_constraint_residuals[edge]);
            }
            assert_eq!(receipt.geometric_rank, 2);
            state = receipt.after;
        }
        for edge in 0..EDGE_COUNT {
            assert!(max_link[edge] < 5.0e-11);
            assert!(max_endpoint[edge] < 5.0e-11);
        }
    }

    #[test]
    fn rattle_preserves_linear_and_angular_momentum_maps() {
        let model = model();
        let initial = initial_state();
        let p0 = model.linear_momentum(initial);
        let j0 = model.angular_momentum(initial);
        let mut state = initial;
        for _ in 0..2000 {
            state = model.step(state, 0.002, SolverOptions::default()).unwrap().after;
        }
        assert!(norm(sub(model.linear_momentum(state), p0)) < 5.0e-11);
        assert!(norm(sub(model.angular_momentum(state), j0)) < 5.0e-11);
    }

    #[test]
    fn rattle_is_second_order_against_independent_constrained_rk4() {
        let model = model();
        let initial = initial_state();
        let reference = rk4_reference(initial, MASSES, 1.0 / 40_000.0, 40_000);
        let mut errors = Vec::new();
        for dt in [0.02, 0.01, 0.005, 0.0025] {
            let mut state = initial;
            for _ in 0..(1.0 / dt).round() as usize {
                state = model.step(state, dt, SolverOptions::default()).unwrap().after;
            }
            errors.push(state_error(state, reference));
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (3.6..4.4).contains(&ratio),
                "RATTLE should converge at second order against independent RK4: {errors:?}"
            );
        }
    }

    #[test]
    fn rattle_is_so3_equivariant() {
        let model = model();
        let initial = initial_state();
        let normal = model.step(initial, 0.01, SolverOptions::default()).unwrap();
        for index in 0..20 {
            let raw = [
                (0.37 * (index as f64 + 1.0)).sin(),
                (0.23 * (index as f64 + 2.0)).cos(),
                (0.41 * (index as f64 + 3.0)).sin(),
            ];
            let rotation = scale(raw, 0.173 * (index as f64 + 1.0) / norm(raw));
            let rotated = model
                .step(rotate_state(initial, rotation), 0.01, SolverOptions::default())
                .unwrap();
            assert!(state_error(rotated.after, rotate_state(normal.after, rotation)) < 2.0e-10);
        }
    }

    #[test]
    fn rattle_is_galilean_covariant() {
        let model = model();
        let initial = initial_state();
        let boost = [0.7, -0.4, 0.23];
        let normal = model.step(initial, 0.01, SolverOptions::default()).unwrap();
        let mut boosted_initial = initial;
        for body in 0..BODY_COUNT {
            boosted_initial.v[body] = add(initial.v[body], boost);
        }
        let boosted = model
            .step(boosted_initial, 0.01, SolverOptions::default())
            .unwrap();
        for body in 0..BODY_COUNT {
            assert!(
                norm(sub(
                    boosted.after.q[body],
                    add(normal.after.q[body], scale(boost, 0.01))
                )) < 2.0e-10
            );
            assert!(
                norm(sub(boosted.after.v[body], add(normal.after.v[body], boost))) < 2.0e-10
            );
        }
    }

    #[test]
    fn fail_closed_on_invalid_manifold_nonfinite_inputs_and_degenerate_graph() {
        let model = model();
        let mut invalid = initial_state();
        invalid.q[1][0] += 0.1;
        assert_eq!(
            model.step(invalid, 0.01, SolverOptions::default()).unwrap_err(),
            Error::RankPreflight(rank::GraphError::PositionDoesNotMatchEdge { edge_id: 101 })
        );

        let mut nonfinite = initial_state();
        nonfinite.v[2][1] = f64::NAN;
        assert_eq!(
            model.step(nonfinite, 0.01, SolverOptions::default()).unwrap_err(),
            Error::NonFiniteInput
        );
        assert_eq!(
            model.step(initial_state(), 0.0, SolverOptions::default()).unwrap_err(),
            Error::InvalidTimeStep
        );
        assert_eq!(
            CartesianRattleChain::new([1.0, 0.0, 1.0], [1.0, 1.0]).unwrap_err(),
            Error::InvalidParameter
        );
    }

    #[test]
    fn collinear_open_chain_is_not_rank_deficient() {
        let mut state = initial_state();
        state.q = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        state.v = [[0.0; 3]; BODY_COUNT];
        let model = CartesianRattleChain::new([1.0, 2.0, 3.0], [1.0, 1.0]).unwrap();
        let receipt = model.step(state, 0.01, SolverOptions::default()).unwrap();
        assert_eq!(receipt.geometric_rank, 2);
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
            [0.0723809523809524, -0.262857142857143, 0.313174603174603],
            [0.1723809523809524, -0.242857142857143, 0.0465079365079363],
            [0.3223809523809524, -0.142857142857143, -0.0534920634920637],
        ],
    };
    let lengths = [norm(edge_vector(state, 0)), norm(edge_vector(state, 1))];
    let model =
        CartesianRattleChain::new([1.3, 2.1, 0.8], lengths).expect("frozen RATTLE chain valid");
    let mut state = state;
    println!("step,energy,link1_residual,link2_residual,endpoint1,endpoint2,linear_momentum,angular_momentum,energy_defect,rank,rcond,newton_iterations");
    for index in 0..500 {
        let receipt = model
            .step(state, 0.002, SolverOptions::default())
            .expect("frozen RATTLE chain step must be admissible");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{},{:.17e},{}",
            index + 1,
            receipt.energy_after,
            receipt.relative_link_constraint_residuals[0],
            receipt.relative_link_constraint_residuals[1],
            receipt.relative_endpoint_velocity_constraint_residuals[0],
            receipt.relative_endpoint_velocity_constraint_residuals[1],
            receipt.relative_linear_momentum_residual,
            receipt.relative_angular_momentum_residual,
            receipt.relative_energy_defect,
            receipt.geometric_rank,
            receipt.geometric_rcond,
            receipt.newton_iterations
        );
    }
}
