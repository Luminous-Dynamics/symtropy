// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Standalone reference for the three-dimensional free asymmetric rigid body.
//!
//! Implements the discrete Moser-Veselov (DMV) map in principal body axes.
//! This is a research oracle, not a production PhysicsWorld integrator.
//!
//! Conventions:
//! - attitude maps body coordinates into world coordinates;
//! - body_momentum is angular momentum in principal body coordinates;
//! - body_momentum_dot = body_momentum cross (I^-1 body_momentum);
//! - attitude_dot = attitude * hat(I^-1 body_momentum).
//!
//! Solve Omega^T D - D Omega = h hat(Pi), with Omega in SO(3), then update
//! Pi+ = Omega Pi and Q+ = Q Omega^T. The unknown is parameterized by
//! Omega = exp(hat(theta)); Newton uses the analytic SO(3) right Jacobian.
//!
//! The DMV method is an established Lie-group variational discretization.
//! This independent implementation exists to qualify Symtropy's asymmetric
//! inertia model against separately computed invariants and trajectories.

#[derive(Clone, Copy, Debug, PartialEq)]
struct Vec3([f64; 3]);

impl Vec3 {
    fn new(x: f64, y: f64, z: f64) -> Self { Self([x, y, z]) }
    fn dot(self, rhs: Self) -> f64 {
        self.0[0] * rhs.0[0] + self.0[1] * rhs.0[1] + self.0[2] * rhs.0[2]
    }
    fn norm(self) -> f64 { self.dot(self).sqrt() }
    fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.0[1] * rhs.0[2] - self.0[2] * rhs.0[1],
            self.0[2] * rhs.0[0] - self.0[0] * rhs.0[2],
            self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0],
        )
    }
    fn scale(self, a: f64) -> Self {
        Self([self.0[0] * a, self.0[1] * a, self.0[2] * a])
    }
    fn add(self, rhs: Self) -> Self {
        Self([self.0[0] + rhs.0[0], self.0[1] + rhs.0[1], self.0[2] + rhs.0[2]])
    }
    fn sub(self, rhs: Self) -> Self {
        Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1], self.0[2] - rhs.0[2]])
    }
    fn component_div(self, rhs: Self) -> Self {
        Self([self.0[0] / rhs.0[0], self.0[1] / rhs.0[1], self.0[2] / rhs.0[2]])
    }
    fn is_finite(self) -> bool { self.0.iter().all(|x| x.is_finite()) }
    fn max_abs(self) -> f64 { self.0.iter().fold(0.0_f64, |m, x| m.max(x.abs())) }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Mat3([[f64; 3]; 3]);

impl Mat3 {
    const IDENTITY: Self = Self([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]);

    fn diagonal(v: Vec3) -> Self {
        Self([[v.0[0], 0.0, 0.0], [0.0, v.0[1], 0.0], [0.0, 0.0, v.0[2]]])
    }
    fn transpose(self) -> Self {
        let mut out = [[0.0; 3]; 3];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() { *value = self.0[j][i]; }
        }
        Self(out)
    }
    fn mul(self, rhs: Self) -> Self {
        let mut out = [[0.0; 3]; 3];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() {
                *value = (0..3).map(|k| self.0[i][k] * rhs.0[k][j]).sum();
            }
        }
        Self(out)
    }
    fn mul_vec(self, v: Vec3) -> Vec3 {
        Vec3::new(
            self.0[0][0] * v.0[0] + self.0[0][1] * v.0[1] + self.0[0][2] * v.0[2],
            self.0[1][0] * v.0[0] + self.0[1][1] * v.0[1] + self.0[1][2] * v.0[2],
            self.0[2][0] * v.0[0] + self.0[2][1] * v.0[1] + self.0[2][2] * v.0[2],
        )
    }
    fn add(self, rhs: Self) -> Self {
        let mut out = [[0.0; 3]; 3];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() { *value = self.0[i][j] + rhs.0[i][j]; }
        }
        Self(out)
    }
    fn sub(self, rhs: Self) -> Self {
        let mut out = [[0.0; 3]; 3];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() { *value = self.0[i][j] - rhs.0[i][j]; }
        }
        Self(out)
    }
    fn scale(self, a: f64) -> Self {
        let mut out = [[0.0; 3]; 3];
        for (i, row) in out.iter_mut().enumerate() {
            for (j, value) in row.iter_mut().enumerate() { *value = self.0[i][j] * a; }
        }
        Self(out)
    }
    fn add_scaled(self, rhs: Self, a: f64) -> Self { self.add(rhs.scale(a)) }
    fn determinant(self) -> f64 {
        let m = self.0;
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    }
    fn frobenius_norm(self) -> f64 {
        self.0.iter().flatten().map(|x| x * x).sum::<f64>().sqrt()
    }
    fn is_finite(self) -> bool { self.0.iter().flatten().all(|x| x.is_finite()) }
}

#[derive(Clone, Copy, Debug)]
struct Model {
    /// Principal moments in a consistent unit system.
    principal_moments: Vec3,
}

impl Model {
    fn discrete_inertia(self) -> Vec3 {
        let i = self.principal_moments;
        Vec3::new(
            0.5 * (i.0[1] + i.0[2] - i.0[0]),
            0.5 * (i.0[2] + i.0[0] - i.0[1]),
            0.5 * (i.0[0] + i.0[1] - i.0[2]),
        )
    }
    fn validate(self) -> Result<(), StepError> {
        let i = self.principal_moments;
        if !i.is_finite() || i.0.iter().any(|x| *x <= 0.0) {
            return Err(StepError::InvalidInertia);
        }
        let d = self.discrete_inertia();
        if !d.is_finite() || d.0.iter().any(|x| *x <= 1e-14 * i.max_abs()) {
            return Err(StepError::NonPhysicalPrincipalInertia);
        }
        Ok(())
    }
    fn energy(self, p: Vec3) -> f64 { 0.5 * p.component_div(self.principal_moments).dot(p) }
    fn angular_velocity(self, p: Vec3) -> Vec3 { p.component_div(self.principal_moments) }
}

#[derive(Clone, Copy, Debug)]
struct State {
    /// Body-to-world attitude.
    attitude: Mat3,
    /// Body-frame angular momentum.
    body_momentum: Vec3,
}

impl State {
    fn validate(self) -> Result<(), StepError> {
        if !self.attitude.is_finite() || !self.body_momentum.is_finite() {
            return Err(StepError::NonFiniteInput);
        }
        if orthogonality_error(self.attitude) > 1e-9
            || (self.attitude.determinant() - 1.0).abs() > 1e-9
        {
            return Err(StepError::InvalidAttitude);
        }
        Ok(())
    }
    fn spatial_momentum(self) -> Vec3 { self.attitude.mul_vec(self.body_momentum) }
}

#[derive(Clone, Copy, Debug)]
struct Options {
    relative_tolerance: f64,
    max_newton_iterations: usize,
    max_backtracks: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { relative_tolerance: 2e-13, max_newton_iterations: 24, max_backtracks: 20 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StepError {
    InvalidInertia,
    NonPhysicalPrincipalInertia,
    NonFiniteInput,
    InvalidAttitude,
    InvalidStepSize,
    InvalidSolverOptions,
    SingularNewtonSystem,
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
    relative_casimir_error: f64,
    relative_spatial_momentum_error: f64,
}

fn hat(v: Vec3) -> Mat3 {
    let [x, y, z] = v.0;
    Mat3([[0.0, -z, y], [z, 0.0, -x], [-y, x, 0.0]])
}
fn vee(m: Mat3) -> Vec3 { Vec3::new(m.0[2][1], m.0[0][2], m.0[1][0]) }

fn exp_so3(theta: Vec3) -> Mat3 {
    let angle = theta.norm();
    let h = hat(theta);
    let h2 = h.mul(h);
    let (a, b) = if angle < 1e-5 {
        let t2 = angle * angle;
        (1.0 - t2 / 6.0 + t2 * t2 / 120.0, 0.5 - t2 / 24.0 + t2 * t2 / 720.0)
    } else {
        (angle.sin() / angle, (1.0 - angle.cos()) / (angle * angle))
    };
    Mat3::IDENTITY.add(h.scale(a)).add(h2.scale(b))
}

/// Right-trivialized derivative of exp(theta^) for perturbations in theta.
fn right_jacobian_so3(theta: Vec3) -> Mat3 {
    let angle = theta.norm();
    let h = hat(theta);
    let h2 = h.mul(h);
    let (a, b) = if angle < 1e-4 {
        let t2 = angle * angle;
        (0.5 - t2 / 24.0 + t2 * t2 / 720.0, 1.0 / 6.0 - t2 / 120.0 + t2 * t2 / 5040.0)
    } else {
        ((1.0 - angle.cos()) / (angle * angle), (angle - angle.sin()) / (angle * angle * angle))
    };
    Mat3::IDENTITY.sub(h.scale(a)).add(h2.scale(b))
}

fn residual(theta: Vec3, d: Vec3, rhs: Vec3) -> Vec3 {
    let omega = exp_so3(theta);
    let diagonal = Mat3::diagonal(d);
    vee(omega.transpose().mul(diagonal).sub(diagonal.mul(omega))).sub(rhs)
}

/// Analytic Jacobian of vee(exp(theta^)ᵀ D - D exp(theta^)).
fn jacobian(theta: Vec3, d: Vec3) -> [[f64; 3]; 3] {
    let omega = exp_so3(theta);
    let omega_t = omega.transpose();
    let diagonal = Mat3::diagonal(d);
    let jr = right_jacobian_so3(theta);
    let mut out = [[0.0; 3]; 3];
    for column in 0..3 {
        let mut basis = [0.0; 3];
        basis[column] = 1.0;
        let eta_hat = hat(jr.mul_vec(Vec3(basis)));
        let d_omega = omega.mul(eta_hat);
        let d_omega_t = eta_hat.mul(omega_t).scale(-1.0);
        let d_residual = d_omega_t.mul(diagonal).sub(diagonal.mul(d_omega));
        let v = vee(d_residual);
        for row in 0..3 { out[row][column] = v.0[row]; }
    }
    out
}

fn solve_3x3(mut a: [[f64; 3]; 3], mut b: Vec3) -> Result<Vec3, StepError> {
    let scale = a.iter().flatten().fold(0.0_f64, |m, x| m.max(x.abs()));
    if !scale.is_finite() || scale <= f64::MIN_POSITIVE {
        return Err(StepError::SingularNewtonSystem);
    }
    for pivot in 0..3 {
        let mut best = pivot;
        for row in (pivot + 1)..3 {
            if a[row][pivot].abs() > a[best][pivot].abs() { best = row; }
        }
        if a[best][pivot].abs() <= 1e-14 * scale {
            return Err(StepError::SingularNewtonSystem);
        }
        if best != pivot {
            a.swap(best, pivot);
            b.0.swap(best, pivot);
        }
        for row in (pivot + 1)..3 {
            let factor = a[row][pivot] / a[pivot][pivot];
            for col in pivot..3 { a[row][col] -= factor * a[pivot][col]; }
            b.0[row] -= factor * b.0[pivot];
        }
    }
    let mut x = [0.0; 3];
    for row in (0..3).rev() {
        let tail: f64 = ((row + 1)..3).map(|col| a[row][col] * x[col]).sum();
        x[row] = (b.0[row] - tail) / a[row][row];
    }
    let result = Vec3(x);
    if !result.is_finite() { return Err(StepError::NonFiniteCandidate); }
    Ok(result)
}

fn orthogonality_error(r: Mat3) -> f64 {
    r.transpose().mul(r).sub(Mat3::IDENTITY).frobenius_norm()
}
fn relative_error(a: f64, b: f64) -> f64 {
    (a - b).abs() / a.abs().max(b.abs()).max(1e-30)
}
fn vector_relative_error(a: Vec3, b: Vec3) -> f64 {
    a.sub(b).norm() / a.norm().max(b.norm()).max(1e-30)
}
fn matrix_distance(a: Mat3, b: Mat3) -> f64 { a.sub(b).frobenius_norm() }

/// One DMV step. Signed dt is accepted for reversibility checks; forward-time
/// production callers should use positive dt. No candidate state is returned
/// unless the implicit equation converges.
fn step(model: Model, state: State, dt: f64, options: Options) -> Result<StepReceipt, StepError> {
    model.validate()?;
    state.validate()?;
    if !dt.is_finite() || dt == 0.0 { return Err(StepError::InvalidStepSize); }
    if !options.relative_tolerance.is_finite() || options.relative_tolerance <= 0.0
        || options.max_newton_iterations == 0 || options.max_backtracks == 0
    {
        return Err(StepError::InvalidSolverOptions);
    }

    let inertia = model.principal_moments;
    let d = model.discrete_inertia();
    let rhs = state.body_momentum.scale(dt);
    let mut theta = model.angular_velocity(state.body_momentum).scale(-dt);
    let scale = (dt.abs() * state.body_momentum.norm())
        .max(inertia.max_abs() * theta.norm())
        .max(f64::MIN_POSITIVE);
    let tolerance = options.relative_tolerance * scale;
    let mut iterations = 0usize;
    let mut converged = false;

    for iteration in 0..=options.max_newton_iterations {
        let r = residual(theta, d, rhs);
        let r_norm = r.norm();
        if !r_norm.is_finite() { return Err(StepError::NonFiniteCandidate); }
        if r_norm <= tolerance {
            iterations = iteration;
            converged = true;
            break;
        }
        if iteration == options.max_newton_iterations { break; }
        let correction = solve_3x3(jacobian(theta, d), r.scale(-1.0))?;
        let mut alpha = 1.0;
        let mut accepted = false;
        for _ in 0..options.max_backtracks {
            let candidate = theta.add(correction.scale(alpha));
            let candidate_norm = residual(candidate, d, rhs).norm();
            if candidate_norm.is_finite() && candidate_norm < r_norm {
                theta = candidate;
                accepted = true;
                break;
            }
            alpha *= 0.5;
        }
        if !accepted { return Err(StepError::NewtonLineSearchFailed); }
    }
    if !converged { return Err(StepError::NewtonDidNotConverge); }

    let omega = exp_so3(theta);
    let next = State {
        attitude: state.attitude.mul(omega.transpose()),
        body_momentum: omega.mul_vec(state.body_momentum),
    };
    if !next.attitude.is_finite() || !next.body_momentum.is_finite() {
        return Err(StepError::NonFiniteCandidate);
    }

    let energy_before = model.energy(state.body_momentum);
    let energy_after = model.energy(next.body_momentum);
    let casimir_before = 0.5 * state.body_momentum.dot(state.body_momentum);
    let casimir_after = 0.5 * next.body_momentum.dot(next.body_momentum);
    Ok(StepReceipt {
        state: next,
        iterations,
        equation_residual: residual(theta, d, rhs).norm(),
        orthogonality_error: orthogonality_error(next.attitude),
        determinant_error: (next.attitude.determinant() - 1.0).abs(),
        relative_energy_error: relative_error(energy_before, energy_after),
        relative_casimir_error: relative_error(casimir_before, casimir_after),
        relative_spatial_momentum_error: vector_relative_error(
            state.spatial_momentum(), next.spatial_momentum()
        ),
    })
}

// Independent continuous-time reference: classical RK4 applied to Euler's
// body-momentum equation and Qdot = Q*hat(omega).
fn derivative(model: Model, state: State) -> (Mat3, Vec3) {
    let omega = model.angular_velocity(state.body_momentum);
    (state.attitude.mul(hat(omega)), state.body_momentum.cross(omega))
}
fn state_add_scaled(state: State, k: (Mat3, Vec3), h: f64) -> State {
    State { attitude: state.attitude.add_scaled(k.0, h), body_momentum: state.body_momentum.add(k.1.scale(h)) }
}
fn rk4_step(model: Model, state: State, h: f64) -> State {
    let k1 = derivative(model, state);
    let k2 = derivative(model, state_add_scaled(state, k1, 0.5 * h));
    let k3 = derivative(model, state_add_scaled(state, k2, 0.5 * h));
    let k4 = derivative(model, state_add_scaled(state, k3, h));
    let dq = k1.0.add(k2.0.scale(2.0)).add(k3.0.scale(2.0)).add(k4.0).scale(h / 6.0);
    let dp = k1.1.add(k2.1.scale(2.0)).add(k3.1.scale(2.0)).add(k4.1).scale(h / 6.0);
    State { attitude: state.attitude.add(dq), body_momentum: state.body_momentum.add(dp) }
}
fn integrate_rk4(model: Model, mut state: State, h: f64, steps: usize) -> State {
    for _ in 0..steps { state = rk4_step(model, state, h); }
    state
}

#[derive(Clone, Copy, Debug, Default)]
struct Metrics {
    max_iterations: usize,
    max_residual: f64,
    max_orthogonality: f64,
    max_determinant: f64,
    max_energy_drift: f64,
    max_casimir_drift: f64,
    max_spatial_momentum_drift: f64,
}

fn integrate_dmv(
    model: Model, mut state: State, h: f64, steps: usize
) -> Result<(State, Metrics), StepError> {
    let e0 = model.energy(state.body_momentum);
    let c0 = 0.5 * state.body_momentum.dot(state.body_momentum);
    let l0 = state.spatial_momentum();
    let mut metrics = Metrics::default();
    for _ in 0..steps {
        let receipt = step(model, state, h, Options::default())?;
        state = receipt.state;
        metrics.max_iterations = metrics.max_iterations.max(receipt.iterations);
        metrics.max_residual = metrics.max_residual.max(receipt.equation_residual);
        metrics.max_orthogonality = metrics.max_orthogonality.max(receipt.orthogonality_error);
        metrics.max_determinant = metrics.max_determinant.max(receipt.determinant_error);
        metrics.max_energy_drift = metrics.max_energy_drift.max(receipt.relative_energy_error);
        metrics.max_casimir_drift = metrics.max_casimir_drift.max(receipt.relative_casimir_error);
        metrics.max_spatial_momentum_drift = metrics.max_spatial_momentum_drift
            .max(receipt.relative_spatial_momentum_error);
        metrics.max_energy_drift = metrics.max_energy_drift.max(relative_error(e0, model.energy(state.body_momentum)));
        metrics.max_casimir_drift = metrics.max_casimir_drift.max(relative_error(c0, 0.5 * state.body_momentum.dot(state.body_momentum)));
        metrics.max_spatial_momentum_drift = metrics.max_spatial_momentum_drift
            .max(vector_relative_error(l0, state.spatial_momentum()));
    }
    Ok((state, metrics))
}

fn state_error(a: State, b: State) -> (f64, f64) {
    (matrix_distance(a.attitude, b.attitude), vector_relative_error(a.body_momentum, b.body_momentum))
}

#[derive(Clone, Copy)]
struct Case { name: &'static str, model: Model, momentum: Vec3 }

const CASES: [Case; 3] = [
    Case { name: "asymmetric", model: Model { principal_moments: Vec3::new(2.0, 3.0, 4.0) }, momentum: Vec3::new(0.3, 0.5, 0.7) },
    Case { name: "near_axisymmetric", model: Model { principal_moments: Vec3::new(2.0, 2.05, 3.6) }, momentum: Vec3::new(0.8, 0.2, 0.4) },
    Case { name: "intermediate_axis", model: Model { principal_moments: Vec3::new(1.0, 2.0, 3.0) }, momentum: Vec3::new(0.08, 1.0, 0.12) },
];

fn campaign() -> Result<(), StepError> {
    const HORIZON: f64 = 1.0;
    const REF_H: f64 = 0.00005;
    const COARSE_REF_H: f64 = 0.0001;
    const STEP_SIZES: [f64; 4] = [0.1, 0.05, 0.025, 0.0125];
    println!("# rustc={},source_revision={}",
        option_env!("RUSTC_VERSION").unwrap_or("not-recorded"),
        option_env!("SYMTROPY_SOURCE_REV").unwrap_or("not-recorded"));
    println!("scenario,dt,steps,attitude_frobenius_error,body_momentum_relative_error,max_energy_drift,max_casimir_drift,max_spatial_momentum_drift,max_so3_orthogonality_error,max_determinant_error,max_newton_residual,max_newton_iterations,reference_refinement_delta");
    for case in CASES {
        case.model.validate()?;
        let initial = State { attitude: Mat3::IDENTITY, body_momentum: case.momentum };
        let reference = integrate_rk4(case.model, initial, REF_H, (HORIZON / REF_H).round() as usize);
        let coarse = integrate_rk4(case.model, initial, COARSE_REF_H, (HORIZON / COARSE_REF_H).round() as usize);
        let (ref_q_error, ref_p_error) = state_error(reference, coarse);
        let reference_delta = ref_q_error.hypot(ref_p_error);
        for h in STEP_SIZES {
            let steps = (HORIZON / h).round() as usize;
            let (candidate, metrics) = integrate_dmv(case.model, initial, h, steps)?;
            let (q_error, p_error) = state_error(candidate, reference);
            println!("{},{:.8e},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{:.12e}",
                case.name, h, steps, q_error, p_error,
                metrics.max_energy_drift, metrics.max_casimir_drift,
                metrics.max_spatial_momentum_drift, metrics.max_orthogonality,
                metrics.max_determinant, metrics.max_residual, metrics.max_iterations,
                reference_delta);
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = campaign() {
        eprintln!("campaign failed closed: {error:?}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> (Model, State) {
        (Model { principal_moments: Vec3::new(2.0, 3.0, 4.0) },
         State { attitude: Mat3::IDENTITY, body_momentum: Vec3::new(0.3, 0.5, 0.7) })
    }

    #[test]
    fn asymmetric_top_preserves_energy_casimir_and_spatial_momentum() {
        let (model, initial) = baseline();
        let (_, m) = integrate_dmv(model, initial, 0.01, 2_000).unwrap();
        assert!(m.max_energy_drift < 2e-11);
        assert!(m.max_casimir_drift < 2e-11);
        assert!(m.max_spatial_momentum_drift < 2e-11);
        assert!(m.max_orthogonality < 2e-11);
        assert!(m.max_determinant < 2e-11);
        assert!(m.max_residual < 1e-11);
    }

    #[test]
    fn discrete_step_is_time_reversible() {
        let (model, initial) = baseline();
        let mut state = initial;
        for _ in 0..128 { state = step(model, state, 0.005, Options::default()).unwrap().state; }
        for _ in 0..128 { state = step(model, state, -0.005, Options::default()).unwrap().state; }
        assert!(matrix_distance(state.attitude, initial.attitude) < 2e-11);
        assert!(state.body_momentum.sub(initial.body_momentum).norm() < 2e-11);
    }

    #[test]
    fn update_stays_on_so3_without_projection() {
        let (model, mut state) = baseline();
        for _ in 0..5_000 { state = step(model, state, 0.002, Options::default()).unwrap().state; }
        assert!(orthogonality_error(state.attitude) < 2e-10);
        assert!((state.attitude.determinant() - 1.0).abs() < 2e-10);
    }

    #[test]
    fn observed_temporal_order_is_second_order() {
        let (model, initial) = baseline();
        let reference = integrate_rk4(model, initial, 0.00005, 20_000);
        let mut errors = Vec::new();
        for h in [0.08, 0.04, 0.02, 0.01] {
            let candidate = integrate_dmv(model, initial, h, (1.0 / h).round() as usize).unwrap().0;
            let (q, p) = state_error(candidate, reference);
            errors.push(q.hypot(p));
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!((3.4..4.6).contains(&ratio), "expected second-order ratio near four, got {ratio}");
        }
    }

    #[test]
    fn update_is_equivariant_under_world_rotation() {
        let (model, initial) = baseline();
        let world_rotation = exp_so3(Vec3::new(0.4, -0.2, 0.3));
        let mut direct = initial;
        let mut rotated = State { attitude: world_rotation.mul(initial.attitude), body_momentum: initial.body_momentum };
        for _ in 0..200 {
            direct = step(model, direct, 0.01, Options::default()).unwrap().state;
            rotated = step(model, rotated, 0.01, Options::default()).unwrap().state;
        }
        assert!(matrix_distance(rotated.attitude, world_rotation.mul(direct.attitude)) < 2e-11);
        assert!(rotated.body_momentum.sub(direct.body_momentum).norm() < 2e-11);
    }

    #[test]
    fn inertia_momentum_time_rescaling_is_similarity_invariant() {
        let (model, initial) = baseline();
        let scale_i = 7.5;
        let scale_p = 2.4;
        let h = 0.01;
        let scaled_model = Model { principal_moments: model.principal_moments.scale(scale_i) };
        let scaled_initial = State { attitude: initial.attitude, body_momentum: initial.body_momentum.scale(scale_p) };
        let direct = step(model, initial, h, Options::default()).unwrap().state;
        let scaled = step(scaled_model, scaled_initial, h * scale_i / scale_p, Options::default()).unwrap().state;
        assert!(matrix_distance(direct.attitude, scaled.attitude) < 2e-12);
        assert!(scaled.body_momentum.scale(1.0 / scale_p).sub(direct.body_momentum).norm() < 2e-12);
    }

    #[test]
    fn rejects_non_physical_principal_inertia_triangle() {
        let model = Model { principal_moments: Vec3::new(1.0, 1.0, 3.0) };
        assert_eq!(model.validate(), Err(StepError::NonPhysicalPrincipalInertia));
    }

    #[test]
    fn rejects_non_finite_input_and_invalid_controls() {
        let (model, initial) = baseline();
        assert_eq!(step(model, initial, f64::NAN, Options::default()).unwrap_err(), StepError::InvalidStepSize);
        let mut invalid = initial;
        invalid.body_momentum.0[1] = f64::INFINITY;
        assert_eq!(step(model, invalid, 0.01, Options::default()).unwrap_err(), StepError::NonFiniteInput);
        let bad_options = Options { relative_tolerance: 0.0, ..Options::default() };
        assert_eq!(step(model, initial, 0.01, bad_options).unwrap_err(), StepError::InvalidSolverOptions);
    }

    #[test]
    fn rejects_degenerate_physical_inertia() {
        let model = Model { principal_moments: Vec3::new(1.0, 2.0, 3.0) };
        assert_eq!(model.validate(), Err(StepError::NonPhysicalPrincipalInertia));
    }

    #[test]
    fn frozen_cases_cover_asymmetric_near_symmetric_and_intermediate_axis() {
        assert_eq!(CASES.len(), 3);
        for case in CASES {
            case.model.validate().unwrap();
            let initial = State { attitude: Mat3::IDENTITY, body_momentum: case.momentum };
            let (_, metrics) = integrate_dmv(case.model, initial, 0.025, 40).unwrap();
            assert!(metrics.max_energy_drift < 2e-11);
            assert!(metrics.max_spatial_momentum_drift < 2e-11);
        }
    }

    #[test]
    fn so3_exponential_is_proper_and_small_angle_stable() {
        for theta in [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1e-12, -2e-12, 3e-12),
            Vec3::new(0.3, -0.4, 0.2), Vec3::new(std::f64::consts::PI - 1e-5, 0.0, 0.0)] {
            let r = exp_so3(theta);
            assert!(orthogonality_error(r) < 2e-12);
            assert!((r.determinant() - 1.0).abs() < 2e-12);
        }
    }

    #[test]
    fn analytic_newton_jacobian_matches_central_difference() {
        let theta = Vec3::new(0.17, -0.23, 0.31);
        let d = Vec3::new(1.5, 2.5, 3.0);
        let analytic = jacobian(theta, d);
        let epsilon = 1e-6;
        for col in 0..3 {
            let mut plus = theta;
            let mut minus = theta;
            plus.0[col] += epsilon;
            minus.0[col] -= epsilon;
            let numeric = residual(plus, d, Vec3::new(0.0, 0.0, 0.0))
                .sub(residual(minus, d, Vec3::new(0.0, 0.0, 0.0)))
                .scale(0.5 / epsilon);
            for row in 0..3 {
                assert!((analytic[row][col] - numeric.0[row]).abs() < 2e-7,
                    "Jacobian mismatch at ({row},{col})");
            }
        }
    }
}
