// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Independent SO(4) free rigid-body reference using a generalized
//! discrete Moser-Veselov map and a physical second-moment matrix.
//!
//! The six bivector coordinates are ordered (01, 02, 03, 12, 13, 23).
//! For a symmetric positive-definite second moment S and body angular
//! velocity W in so(4), the physical angular momentum is
//!
//!     M = S W + W S.
//!
//! The discrete step solves
//!
//!     O^T S - S O = h M,   O in SO(4),
//!     M+ = O M O^T,        Q+ = Q O^T.
//!
//! O is parameterized by the Cayley transform, so every accepted increment
//! is in SO(4) without a post-step projection. A six-variable damped Newton
//! solve uses a central-difference Jacobian; this is deliberately a
//! transparent reference, not a production solver.
//!
//! Independent observables: kinetic energy, the two SO(4) quadratic
//! Casimirs (bivector norm and Pfaffian), spatial momentum Q M Q^T,
//! group membership, equation residual, reversibility, covariance and
//! temporal convergence.

const N: usize = 4;
const NV: usize = 6;
const PAIRS: [(usize, usize); NV] = [
    (0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3),
];

#[derive(Clone, Copy, Debug, PartialEq)]
struct V6([f64; NV]);

impl V6 {
    const ZERO: Self = Self([0.0; NV]);
    fn dot(self, rhs: Self) -> f64 {
        self.0.iter().zip(rhs.0).map(|(a, b)| a * b).sum()
    }
    fn norm(self) -> f64 { self.dot(self).sqrt() }
    fn scale(self, a: f64) -> Self {
        Self(std::array::from_fn(|i| self.0[i] * a))
    }
    fn add(self, rhs: Self) -> Self {
        Self(std::array::from_fn(|i| self.0[i] + rhs.0[i]))
    }
    fn sub(self, rhs: Self) -> Self {
        Self(std::array::from_fn(|i| self.0[i] - rhs.0[i]))
    }
    fn is_finite(self) -> bool { self.0.iter().all(|x| x.is_finite()) }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Mat4([[f64; N]; N]);

impl Mat4 {
    const ZERO: Self = Self([[0.0; N]; N]);
    const IDENTITY: Self = Self([
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]);

    fn transpose(self) -> Self {
        let mut out = [[0.0; N]; N];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = self.0[j][i];
            }
        }
        Self(out)
    }
    fn mul(self, rhs: Self) -> Self {
        let mut out = [[0.0; N]; N];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = (0..N).map(|k| self.0[i][k] * rhs.0[k][j]).sum();
            }
        }
        Self(out)
    }
    fn add(self, rhs: Self) -> Self {
        let mut out = [[0.0; N]; N];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = self.0[i][j] + rhs.0[i][j];
            }
        }
        Self(out)
    }
    fn sub(self, rhs: Self) -> Self {
        let mut out = [[0.0; N]; N];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = self.0[i][j] - rhs.0[i][j];
            }
        }
        Self(out)
    }
    fn scale(self, a: f64) -> Self {
        let mut out = [[0.0; N]; N];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = self.0[i][j] * a;
            }
        }
        Self(out)
    }
    fn determinant(self) -> f64 {
        let mut a = self.0;
        let mut determinant = 1.0;
        for pivot in 0..N {
            let mut best = pivot;
            for row in (pivot + 1)..N {
                if a[row][pivot].abs() > a[best][pivot].abs() {
                    best = row;
                }
            }
            if a[best][pivot].abs() <= f64::MIN_POSITIVE {
                return 0.0;
            }
            if best != pivot {
                a.swap(best, pivot);
                determinant = -determinant;
            }
            let diagonal = a[pivot][pivot];
            determinant *= diagonal;
            for row in (pivot + 1)..N {
                let factor = a[row][pivot] / diagonal;
                for col in (pivot + 1)..N {
                    a[row][col] -= factor * a[pivot][col];
                }
            }
        }
        determinant
    }
    fn frobenius_norm(self) -> f64 {
        self.0.iter().flatten().map(|x| x * x).sum::<f64>().sqrt()
    }
    fn max_abs(self) -> f64 {
        self.0.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()))
    }
    fn is_finite(self) -> bool { self.0.iter().flatten().all(|x| x.is_finite()) }
}

fn skew_from_coords(x: V6) -> Mat4 {
    let mut out = Mat4::ZERO;
    for (k, (i, j)) in PAIRS.iter().copied().enumerate() {
        out.0[i][j] = x.0[k];
        out.0[j][i] = -x.0[k];
    }
    out
}

fn skew_coords(matrix: Mat4) -> V6 {
    V6(std::array::from_fn(|k| {
        let (i, j) = PAIRS[k];
        0.5 * (matrix.0[i][j] - matrix.0[j][i])
    }))
}

fn skew_residual_coords(matrix: Mat4) -> V6 {
    V6(std::array::from_fn(|k| {
        let (i, j) = PAIRS[k];
        matrix.0[i][j]
    }))
}

fn solve_dense<const K: usize>(
    mut matrix: [[f64; K]; K],
    mut rhs: [f64; K],
) -> Result<[f64; K], StepError> {
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if !scale.is_finite() || scale <= f64::MIN_POSITIVE {
        return Err(StepError::SingularLinearSystem);
    }

    for pivot in 0..K {
        let mut best = pivot;
        for row in (pivot + 1)..K {
            if matrix[row][pivot].abs() > matrix[best][pivot].abs() {
                best = row;
            }
        }
        if matrix[best][pivot].abs() <= 2e-14 * scale {
            return Err(StepError::SingularLinearSystem);
        }
        if best != pivot {
            matrix.swap(best, pivot);
            rhs.swap(best, pivot);
        }
        for row in (pivot + 1)..K {
            let factor = matrix[row][pivot] / matrix[pivot][pivot];
            for col in (pivot + 1)..K {
                matrix[row][col] -= factor * matrix[pivot][col];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }

    let mut solution = [0.0; K];
    for row in (0..K).rev() {
        let tail: f64 = ((row + 1)..K)
            .map(|col| matrix[row][col] * solution[col])
            .sum();
        solution[row] = (rhs[row] - tail) / matrix[row][row];
    }
    if solution.iter().all(|x| x.is_finite()) {
        Ok(solution)
    } else {
        Err(StepError::NonFiniteCandidate)
    }
}

fn solve_matrix4(matrix: Mat4, rhs: Mat4) -> Result<Mat4, StepError> {
    let mut columns = [[0.0; N]; N];
    for col in 0..N {
        let vector = std::array::from_fn(|row| rhs.0[row][col]);
        let solved = solve_dense(matrix.0, vector)?;
        for row in 0..N {
            columns[row][col] = solved[row];
        }
    }
    Ok(Mat4(columns))
}

fn cholesky_is_spd(matrix: Mat4) -> bool {
    let mut lower = [[0.0; N]; N];
    let scale = matrix.max_abs().max(f64::MIN_POSITIVE);
    for i in 0..N {
        for j in 0..=i {
            let correction: f64 = (0..j).map(|k| lower[i][k] * lower[j][k]).sum();
            if i == j {
                let diagonal = matrix.0[i][i] - correction;
                if !diagonal.is_finite() || diagonal <= 1e-13 * scale {
                    return false;
                }
                lower[i][j] = diagonal.sqrt();
            } else {
                lower[i][j] = (matrix.0[i][j] - correction) / lower[j][j];
            }
        }
    }
    true
}

#[derive(Clone, Copy, Debug)]
struct Model {
    /// Centered mass second-moment matrix, in a consistent unit system.
    /// The associated physical inertia map on so(4) is W -> S W + W S.
    second_moment: Mat4,
}

impl Model {
    fn validate(self) -> Result<(), StepError> {
        if !self.second_moment.is_finite() {
            return Err(StepError::NonFiniteInput);
        }
        let scale = self.second_moment.max_abs().max(f64::MIN_POSITIVE);
        if self
            .second_moment
            .sub(self.second_moment.transpose())
            .frobenius_norm()
            > 1e-12 * scale
        {
            return Err(StepError::NonSymmetricSecondMoment);
        }
        let symmetric = self.second_moment
            .add(self.second_moment.transpose())
            .scale(0.5);
        if !cholesky_is_spd(symmetric) {
            return Err(StepError::SecondMomentNotPositiveDefinite);
        }
        Ok(())
    }

    fn inertia_action(self, angular_velocity: Mat4) -> Mat4 {
        self.second_moment
            .mul(angular_velocity)
            .add(angular_velocity.mul(self.second_moment))
    }

    fn angular_velocity(self, momentum: Mat4) -> Result<Mat4, StepError> {
        let mut matrix = [[0.0; NV]; NV];
        for column in 0..NV {
            let mut basis = [0.0; NV];
            basis[column] = 1.0;
            let response = skew_coords(self.inertia_action(skew_from_coords(V6(basis))));
            for row in 0..NV {
                matrix[row][column] = response.0[row];
            }
        }
        let rhs = skew_coords(momentum).0;
        let solution = solve_dense(matrix, rhs)?;
        Ok(skew_from_coords(V6(solution)))
    }

    fn kinetic_energy(self, momentum: Mat4) -> Result<f64, StepError> {
        let omega = self.angular_velocity(momentum)?;
        Ok(0.5 * skew_coords(momentum).dot(skew_coords(omega)))
    }
}

#[derive(Clone, Copy, Debug)]
struct State {
    /// Body-to-world orientation in SO(4).
    attitude: Mat4,
    /// Body-frame angular momentum bivector.
    body_momentum: Mat4,
}

impl State {
    fn validate(self) -> Result<(), StepError> {
        if !self.attitude.is_finite() || !self.body_momentum.is_finite() {
            return Err(StepError::NonFiniteInput);
        }
        let skew_error = self
            .body_momentum
            .add(self.body_momentum.transpose())
            .frobenius_norm();
        if skew_error > 1e-11 * self.body_momentum.max_abs().max(1.0) {
            return Err(StepError::InvalidMomentumBivector);
        }
        let orthogonality = self
            .attitude
            .transpose()
            .mul(self.attitude)
            .sub(Mat4::IDENTITY)
            .frobenius_norm();
        if orthogonality > 1e-9 || (self.attitude.determinant() - 1.0).abs() > 1e-9 {
            return Err(StepError::InvalidAttitude);
        }
        Ok(())
    }

    fn spatial_momentum(self) -> Mat4 {
        self.attitude.mul(self.body_momentum).mul(self.attitude.transpose())
    }
}

#[derive(Clone, Copy, Debug)]
struct Options {
    relative_tolerance: f64,
    max_newton_iterations: usize,
    max_backtracks: usize,
    finite_difference_step: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            relative_tolerance: 2e-12,
            max_newton_iterations: 30,
            max_backtracks: 24,
            finite_difference_step: 1e-6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepError {
    NonFiniteInput,
    NonSymmetricSecondMoment,
    SecondMomentNotPositiveDefinite,
    InvalidMomentumBivector,
    InvalidAttitude,
    InvalidStepSize,
    InvalidSolverOptions,
    SingularLinearSystem,
    CayleyChartSingularity,
    NewtonLineSearchFailed,
    NewtonDidNotConverge,
    NonFiniteCandidate,
}

#[derive(Clone, Copy, Debug)]
struct StepReceipt {
    state: State,
    iterations: usize,
    equation_residual: f64,
    orthogonality_error: f64,
    determinant_error: f64,
    relative_energy_error: f64,
    relative_norm_casimir_error: f64,
    relative_pfaffian_error: f64,
    relative_spatial_momentum_error: f64,
}

fn cayley(parameters: V6) -> Result<Mat4, StepError> {
    let a = skew_from_coords(parameters);
    let denominator = Mat4::IDENTITY.sub(a);
    let numerator = Mat4::IDENTITY.add(a);
    solve_matrix4(denominator, numerator).map_err(|_| StepError::CayleyChartSingularity)
}

fn equation_residual(parameters: V6, second_moment: Mat4, momentum: Mat4, dt: f64) -> Result<V6, StepError> {
    let increment = cayley(parameters)?;
    let residual = increment
        .transpose()
        .mul(second_moment)
        .sub(second_moment.mul(increment))
        .sub(momentum.scale(dt));
    let coordinates = skew_residual_coords(residual);
    if coordinates.is_finite() {
        Ok(coordinates)
    } else {
        Err(StepError::NonFiniteCandidate)
    }
}

fn finite_difference_jacobian(
    parameters: V6,
    second_moment: Mat4,
    momentum: Mat4,
    dt: f64,
    epsilon: f64,
) -> Result<[[f64; NV]; NV], StepError> {
    let mut jacobian = [[0.0; NV]; NV];
    for column in 0..NV {
        let mut plus = parameters;
        let mut minus = parameters;
        plus.0[column] += epsilon;
        minus.0[column] -= epsilon;
        let r_plus = equation_residual(plus, second_moment, momentum, dt)?;
        let r_minus = equation_residual(minus, second_moment, momentum, dt)?;
        for row in 0..NV {
            jacobian[row][column] =
                (r_plus.0[row] - r_minus.0[row]) / (2.0 * epsilon);
        }
    }
    Ok(jacobian)
}

fn relative_error(a: f64, b: f64) -> f64 {
    (a - b).abs() / a.abs().max(b.abs()).max(1e-30)
}

fn vector_relative_error(a: V6, b: V6) -> f64 {
    a.sub(b).norm() / a.norm().max(b.norm()).max(1e-30)
}

fn matrix_relative_error(a: Mat4, b: Mat4) -> f64 {
    a.sub(b).frobenius_norm() / a.frobenius_norm().max(b.frobenius_norm()).max(1e-30)
}

fn pfaffian(momentum: Mat4) -> f64 {
    momentum.0[0][1] * momentum.0[2][3]
        - momentum.0[0][2] * momentum.0[1][3]
        + momentum.0[0][3] * momentum.0[1][2]
}

fn step(
    model: Model,
    state: State,
    dt: f64,
    options: Options,
) -> Result<StepReceipt, StepError> {
    model.validate()?;
    state.validate()?;
    if !dt.is_finite() || dt == 0.0 {
        return Err(StepError::InvalidStepSize);
    }
    if !options.relative_tolerance.is_finite()
        || options.relative_tolerance <= 0.0
        || options.max_newton_iterations == 0
        || options.max_backtracks == 0
        || !options.finite_difference_step.is_finite()
        || options.finite_difference_step <= 0.0
    {
        return Err(StepError::InvalidSolverOptions);
    }

    let angular_velocity = model.angular_velocity(state.body_momentum)?;
    let mut parameters = skew_coords(angular_velocity).scale(-0.5 * dt);
    let scale = (dt.abs() * state.body_momentum.frobenius_norm())
        .max(model.second_moment.max_abs() * parameters.norm())
        .max(f64::MIN_POSITIVE);
    let tolerance = options.relative_tolerance * scale;
    let mut iterations = 0usize;
    let mut converged = false;

    for iteration in 0..=options.max_newton_iterations {
        let residual = equation_residual(parameters, model.second_moment, state.body_momentum, dt)?;
        let residual_norm = residual.norm();
        if !residual_norm.is_finite() {
            return Err(StepError::NonFiniteCandidate);
        }
        if residual_norm <= tolerance {
            iterations = iteration;
            converged = true;
            break;
        }
        if iteration == options.max_newton_iterations {
            break;
        }

        let jacobian = finite_difference_jacobian(
            parameters,
            model.second_moment,
            state.body_momentum,
            dt,
            options.finite_difference_step,
        )?;
        let correction = solve_dense(jacobian, residual.scale(-1.0).0)?;
        let mut alpha = 1.0;
        let mut accepted = false;
        for _ in 0..options.max_backtracks {
            let candidate = parameters.add(V6(correction).scale(alpha));
            let candidate_residual =
                equation_residual(candidate, model.second_moment, state.body_momentum, dt)?;
            if candidate_residual.norm() < residual_norm {
                parameters = candidate;
                accepted = true;
                break;
            }
            alpha *= 0.5;
        }
        if !accepted {
            return Err(StepError::NewtonLineSearchFailed);
        }
    }
    if !converged {
        return Err(StepError::NewtonDidNotConverge);
    }

    let increment = cayley(parameters)?;
    let next_state = State {
        attitude: state.attitude.mul(increment.transpose()),
        body_momentum: increment
            .mul(state.body_momentum)
            .mul(increment.transpose()),
    };
    if !next_state.attitude.is_finite() || !next_state.body_momentum.is_finite() {
        return Err(StepError::NonFiniteCandidate);
    }

    let energy_before = model.kinetic_energy(state.body_momentum)?;
    let energy_after = model.kinetic_energy(next_state.body_momentum)?;
    let norm_before = skew_coords(state.body_momentum).dot(skew_coords(state.body_momentum));
    let norm_after = skew_coords(next_state.body_momentum).dot(skew_coords(next_state.body_momentum));
    let pfaffian_before = pfaffian(state.body_momentum);
    let pfaffian_after = pfaffian(next_state.body_momentum);
    let orthogonality_error = next_state
        .attitude
        .transpose()
        .mul(next_state.attitude)
        .sub(Mat4::IDENTITY)
        .frobenius_norm();

    Ok(StepReceipt {
        state: next_state,
        iterations,
        equation_residual: equation_residual(
            parameters,
            model.second_moment,
            state.body_momentum,
            dt,
        )?.norm(),
        orthogonality_error,
        determinant_error: (next_state.attitude.determinant() - 1.0).abs(),
        relative_energy_error: relative_error(energy_before, energy_after),
        relative_norm_casimir_error: relative_error(norm_before, norm_after),
        relative_pfaffian_error: relative_error(pfaffian_before, pfaffian_after),
        relative_spatial_momentum_error: matrix_relative_error(
            state.spatial_momentum(),
            next_state.spatial_momentum(),
        ),
    })
}

// Independent continuous-time SO(4) Euler-Arnold equations:
// W = I_S^-1(M), dot(M) = M W - W M, dot(Q) = Q W.
fn derivative(model: Model, state: State) -> Result<(Mat4, Mat4), StepError> {
    let angular_velocity = model.angular_velocity(state.body_momentum)?;
    Ok((
        state.attitude.mul(angular_velocity),
        state.body_momentum.mul(angular_velocity)
            .sub(angular_velocity.mul(state.body_momentum)),
    ))
}

fn state_add_scaled(state: State, derivative: (Mat4, Mat4), dt: f64) -> State {
    State {
        attitude: state.attitude.add(derivative.0.scale(dt)),
        body_momentum: state.body_momentum.add(derivative.1.scale(dt)),
    }
}

fn rk4_step(model: Model, state: State, dt: f64) -> Result<State, StepError> {
    let k1 = derivative(model, state)?;
    let k2 = derivative(model, state_add_scaled(state, k1, 0.5 * dt))?;
    let k3 = derivative(model, state_add_scaled(state, k2, 0.5 * dt))?;
    let k4 = derivative(model, state_add_scaled(state, k3, dt))?;
    let attitude_delta = k1.0
        .add(k2.0.scale(2.0))
        .add(k3.0.scale(2.0))
        .add(k4.0)
        .scale(dt / 6.0);
    let momentum_delta = k1.1
        .add(k2.1.scale(2.0))
        .add(k3.1.scale(2.0))
        .add(k4.1)
        .scale(dt / 6.0);
    Ok(State {
        attitude: state.attitude.add(attitude_delta),
        body_momentum: state.body_momentum.add(momentum_delta),
    })
}

fn integrate_rk4(model: Model, mut state: State, dt: f64, steps: usize) -> Result<State, StepError> {
    for _ in 0..steps {
        state = rk4_step(model, state, dt)?;
    }
    Ok(state)
}

#[derive(Clone, Copy, Debug, Default)]
struct Metrics {
    max_iterations: usize,
    max_residual: f64,
    max_orthogonality: f64,
    max_determinant: f64,
    max_energy_drift: f64,
    max_norm_casimir_drift: f64,
    max_pfaffian_drift: f64,
    max_spatial_momentum_drift: f64,
}

fn integrate_dmv(
    model: Model,
    mut state: State,
    dt: f64,
    steps: usize,
) -> Result<(State, Metrics), StepError> {
    let energy0 = model.kinetic_energy(state.body_momentum)?;
    let norm0 = skew_coords(state.body_momentum).norm();
    let pfaffian0 = pfaffian(state.body_momentum);
    let spatial0 = state.spatial_momentum();
    let mut metrics = Metrics::default();
    for _ in 0..steps {
        let receipt = step(model, state, dt, Options::default())?;
        state = receipt.state;
        metrics.max_iterations = metrics.max_iterations.max(receipt.iterations);
        metrics.max_residual = metrics.max_residual.max(receipt.equation_residual);
        metrics.max_orthogonality = metrics.max_orthogonality.max(receipt.orthogonality_error);
        metrics.max_determinant = metrics.max_determinant.max(receipt.determinant_error);
        metrics.max_energy_drift = metrics.max_energy_drift.max(relative_error(
            energy0,
            model.kinetic_energy(state.body_momentum)?,
        ));
        metrics.max_norm_casimir_drift = metrics.max_norm_casimir_drift.max(relative_error(
            norm0,
            skew_coords(state.body_momentum).norm(),
        ));
        metrics.max_pfaffian_drift = metrics.max_pfaffian_drift.max(relative_error(
            pfaffian0,
            pfaffian(state.body_momentum),
        ));
        metrics.max_spatial_momentum_drift = metrics.max_spatial_momentum_drift.max(
            matrix_relative_error(spatial0, state.spatial_momentum()),
        );
    }
    Ok((state, metrics))
}

fn state_error(left: State, right: State) -> f64 {
    matrix_relative_error(left.attitude, right.attitude)
        .hypot(vector_relative_error(
            skew_coords(left.body_momentum),
            skew_coords(right.body_momentum),
        ))
}

fn vector_relative_error(left: V6, right: V6) -> f64 {
    left.sub(right).norm() / left.norm().max(right.norm()).max(1e-30)
}

fn diagonal_second_moment(values: [f64; N]) -> Mat4 {
    Mat4([
        [values[0], 0.0, 0.0, 0.0],
        [0.0, values[1], 0.0, 0.0],
        [0.0, 0.0, values[2], 0.0],
        [0.0, 0.0, 0.0, values[3]],
    ])
}

fn rotation_plane(i: usize, j: usize, angle: f64) -> Mat4 {
    let mut out = Mat4::IDENTITY;
    let (s, c) = angle.sin_cos();
    out.0[i][i] = c;
    out.0[j][j] = c;
    out.0[i][j] = -s;
    out.0[j][i] = s;
    out
}

#[derive(Clone, Copy)]
struct Case {
    name: &'static str,
    model: Model,
    momentum: V6,
}

const CASES: [Case; 3] = [
    Case {
        name: "generic_double_rotation",
        model: Model { second_moment: diagonal_second_moment([1.0, 1.4, 1.8, 2.2]) },
        momentum: V6([0.3, 0.5, 0.7, 0.2, -0.4, 0.6]),
    },
    Case {
        name: "near_degenerate_principal_values",
        model: Model { second_moment: diagonal_second_moment([1.0, 1.0001, 1.8, 2.2]) },
        momentum: V6([0.8, -0.2, 0.4, 0.6, 0.3, -0.5]),
    },
    Case {
        name: "two_independent_plane_spins",
        model: Model { second_moment: diagonal_second_moment([1.0, 1.4, 1.8, 2.2]) },
        momentum: V6([0.7, 0.0, 0.0, 0.0, 0.0, -0.9]),
    },
];

fn campaign() -> Result<(), StepError> {
    const HORIZON: f64 = 1.0;
    const REFERENCE_DT: f64 = 0.0001;
    const REFERENCE_STEPS: usize = 10_000;
    const STEP_SIZES: [f64; 4] = [0.1, 0.05, 0.025, 0.0125];

    println!(
        "# rustc={},source_revision={}",
        option_env!("RUSTC_VERSION").unwrap_or("not-recorded"),
        option_env!("SYMTROPY_SOURCE_REV").unwrap_or("not-recorded")
    );
    println!("scenario,dt,steps,state_error_vs_rk4,max_energy_drift,max_norm_casimir_drift,max_pfaffian_drift,max_spatial_momentum_drift,max_so4_orthogonality_error,max_determinant_error,max_equation_residual,max_newton_iterations");

    for case in CASES {
        case.model.validate()?;
        let initial = State {
            attitude: Mat4::IDENTITY,
            body_momentum: skew_from_coords(case.momentum),
        };
        let reference = integrate_rk4(case.model, initial, REFERENCE_DT, REFERENCE_STEPS)?;
        for dt in STEP_SIZES {
            let steps = (HORIZON / dt).round() as usize;
            let (candidate, metrics) = integrate_dmv(case.model, initial, dt, steps)?;
            println!(
                "{},{:.8e},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{}",
                case.name,
                dt,
                steps,
                state_error(candidate, reference),
                metrics.max_energy_drift,
                metrics.max_norm_casimir_drift,
                metrics.max_pfaffian_drift,
                metrics.max_spatial_momentum_drift,
                metrics.max_orthogonality,
                metrics.max_determinant,
                metrics.max_residual,
                metrics.max_iterations,
            );
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = campaign() {
        eprintln!("SO(4) reference campaign failed closed: {error:?}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> (Model, State) {
        (
            Model { second_moment: diagonal_second_moment([1.0, 1.4, 1.8, 2.2]) },
            State {
                attitude: Mat4::IDENTITY,
                body_momentum: skew_from_coords(V6([0.3, 0.5, 0.7, 0.2, -0.4, 0.6])),
            },
        )
    }

    #[test]
    fn so4_top_preserves_energy_both_casimirs_and_spatial_momentum() {
        let (model, initial) = baseline();
        let (_, metrics) = integrate_dmv(model, initial, 0.01, 2_000).unwrap();
        assert!(metrics.max_energy_drift < 2e-10, "{metrics:?}");
        assert!(metrics.max_norm_casimir_drift < 2e-10, "{metrics:?}");
        assert!(metrics.max_pfaffian_drift < 2e-10, "{metrics:?}");
        assert!(metrics.max_spatial_momentum_drift < 2e-10, "{metrics:?}");
        assert!(metrics.max_orthogonality < 2e-10, "{metrics:?}");
        assert!(metrics.max_determinant < 2e-10, "{metrics:?}");
        assert!(metrics.max_residual < 1e-10, "{metrics:?}");
    }

    #[test]
    fn cayley_increment_stays_in_so4_without_projection() {
        let (model, initial) = baseline();
        let mut state = initial;
        for _ in 0..5_000 {
            state = step(model, state, 0.002, Options::default()).unwrap().state;
        }
        let orthogonality = state.attitude.transpose().mul(state.attitude)
            .sub(Mat4::IDENTITY).frobenius_norm();
        assert!(orthogonality < 2e-10);
        assert!((state.attitude.determinant() - 1.0).abs() < 2e-10);
    }

    #[test]
    fn map_is_time_reversible() {
        let (model, initial) = baseline();
        let mut state = initial;
        for _ in 0..128 {
            state = step(model, state, 0.005, Options::default()).unwrap().state;
        }
        for _ in 0..128 {
            state = step(model, state, -0.005, Options::default()).unwrap().state;
        }
        assert!(matrix_relative_error(state.attitude, initial.attitude) < 2e-10);
        assert!(vector_relative_error(
            skew_coords(state.body_momentum),
            skew_coords(initial.body_momentum),
        ) < 2e-10);
    }

    #[test]
    fn observed_temporal_order_is_second_order() {
        let (model, initial) = baseline();
        let reference = integrate_rk4(model, initial, 0.0001, 10_000).unwrap();
        let mut errors = Vec::new();
        for dt in [0.125, 0.0625, 0.03125, 0.015625] {
            let steps = (1.0 / dt).round() as usize;
            let candidate = integrate_dmv(model, initial, dt, steps).unwrap().0;
            errors.push(state_error(candidate, reference));
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!((3.2..4.8).contains(&ratio), "expected second-order ratio near four; got {ratio}");
        }
    }

    #[test]
    fn dynamics_are_covariant_under_body_basis_rotation() {
        let (model, initial) = baseline();
        let body_change = rotation_plane(0, 2, 0.37).mul(rotation_plane(1, 3, -0.21));
        let transformed_model = Model {
            second_moment: body_change.mul(model.second_moment).mul(body_change.transpose()),
        };
        let transformed_initial = State {
            attitude: initial.attitude.mul(body_change.transpose()),
            body_momentum: body_change.mul(initial.body_momentum).mul(body_change.transpose()),
        };
        let mut direct = initial;
        let mut transformed = transformed_initial;
        for _ in 0..100 {
            direct = step(model, direct, 0.01, Options::default()).unwrap().state;
            transformed = step(transformed_model, transformed, 0.01, Options::default())
                .unwrap().state;
        }
        let recovered_attitude = transformed.attitude.mul(body_change);
        let recovered_momentum = body_change.transpose()
            .mul(transformed.body_momentum).mul(body_change);
        assert!(matrix_relative_error(recovered_attitude, direct.attitude) < 2e-9);
        assert!(matrix_relative_error(recovered_momentum, direct.body_momentum) < 2e-9);
    }

    #[test]
    fn double_rotation_fixture_keeps_the_two_plane_subspaces_separate() {
        let case = CASES[2];
        let initial = State {
            attitude: Mat4::IDENTITY,
            body_momentum: skew_from_coords(case.momentum),
        };
        let (final_state, metrics) = integrate_dmv(case.model, initial, 0.01, 1_000).unwrap();
        for (i, j) in [(0, 2), (0, 3), (1, 2), (1, 3)] {
            assert!(final_state.body_momentum.0[i][j].abs() < 1e-11);
        }
        assert!(metrics.max_energy_drift < 2e-10);
        assert!(metrics.max_pfaffian_drift < 2e-10);
    }

    #[test]
    fn physical_scaling_is_similarity_invariant() {
        let (model, initial) = baseline();
        let inertia_scale = 5.0;
        let momentum_scale = 2.3;
        let dt = 0.01;
        let scaled_model = Model {
            second_moment: model.second_moment.scale(inertia_scale),
        };
        let scaled_initial = State {
            attitude: initial.attitude,
            body_momentum: initial.body_momentum.scale(momentum_scale),
        };
        let direct = step(model, initial, dt, Options::default()).unwrap().state;
        let scaled = step(
            scaled_model,
            scaled_initial,
            dt * inertia_scale / momentum_scale,
            Options::default(),
        ).unwrap().state;
        assert!(matrix_relative_error(direct.attitude, scaled.attitude) < 2e-10);
        assert!(matrix_relative_error(
            scaled.body_momentum.scale(1.0 / momentum_scale),
            direct.body_momentum,
        ) < 2e-10);
    }

    #[test]
    fn rejects_non_spd_or_nonsymmetric_second_moments() {
        let (model, state) = baseline();
        let mut indefinite = model;
        indefinite.second_moment.0[3][3] = -0.1;
        assert_eq!(
            step(indefinite, state, 0.01, Options::default()).unwrap_err(),
            StepError::SecondMomentNotPositiveDefinite
        );
        let mut asymmetric = model;
        asymmetric.second_moment.0[0][1] = 0.2;
        assert_eq!(
            step(asymmetric, state, 0.01, Options::default()).unwrap_err(),
            StepError::NonSymmetricSecondMoment
        );
    }

    #[test]
    fn rejects_nonfinite_state_invalid_timestep_and_non_bivector_momentum() {
        let (model, state) = baseline();
        assert_eq!(
            step(model, state, f64::NAN, Options::default()).unwrap_err(),
            StepError::InvalidStepSize
        );
        let mut nonfinite = state;
        nonfinite.body_momentum.0[0][1] = f64::INFINITY;
        assert_eq!(
            step(model, nonfinite, 0.01, Options::default()).unwrap_err(),
            StepError::NonFiniteInput
        );
        let mut nonskew = state;
        nonskew.body_momentum.0[0][1] = 0.4;
        nonskew.body_momentum.0[1][0] = 0.2;
        assert_eq!(
            step(model, nonskew, 0.01, Options::default()).unwrap_err(),
            StepError::InvalidMomentumBivector
        );
    }

    #[test]
    fn generalized_inertia_action_matches_pairwise_sum_in_principal_frame() {
        let s = [1.0, 1.4, 1.8, 2.2];
        let model = Model { second_moment: diagonal_second_moment(s) };
        let omega = skew_from_coords(V6([0.2, -0.1, 0.4, 0.7, -0.3, 0.5]));
        let momentum = skew_coords(model.inertia_action(omega));
        for (k, (i, j)) in PAIRS.iter().copied().enumerate() {
            let expected = (s[i] + s[j]) * omega.0[i.min(j) * 0 + k];
            assert!((momentum.0[k] - expected).abs() < 1e-12);
        }
    }
}
