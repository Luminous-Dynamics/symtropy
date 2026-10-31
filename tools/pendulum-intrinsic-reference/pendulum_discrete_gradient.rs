// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Intrinsic-coordinate pendulum reference. Dependency-free Rust.

use std::f64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    InvalidParameter,
    InvalidState,
    InvalidTimeStep,
    InvalidSolverOptions,
    SingularNewtonSystem,
    NoConvergence,
    NonFiniteResult,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    /// Unwrapped pendulum angle in radians.
    pub angle: f64,
    /// Canonical angular momentum, in kg m^2 / s.
    pub angular_momentum: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct SolverOptions {
    pub relative_tolerance: f64,
    pub max_iterations: usize,
    pub max_backtracks: usize,
}

impl Default for SolverOptions {
    fn default() -> Self {
        Self {
            relative_tolerance: 1.0e-13,
            max_iterations: 32,
            max_backtracks: 24,
        }
    }
}

impl SolverOptions {
    fn validate(self) -> Result<Self, Error> {
        if !self.relative_tolerance.is_finite()
            || self.relative_tolerance <= 0.0
            || self.max_iterations == 0
            || self.max_backtracks == 0
        {
            return Err(Error::InvalidSolverOptions);
        }
        Ok(self)
    }
}

/// Mathematical pendulum in reduced canonical coordinates.
///
/// H(theta, ell) = ell^2 / (2 m L^2) + m g L (1 - cos(theta))
///
/// theta_dot = ell / I
/// ell_dot = -m g L sin(theta) - damping * theta_dot + applied_torque
///
/// The circle constraint x^2 + y^2 = L^2 is exact by construction through
/// (x,y) = (L sin(theta), -L cos(theta)). This is NOT a Cartesian DAE solver.
#[derive(Clone, Copy, Debug)]
pub struct IntrinsicPendulum {
    mass: f64,
    length: f64,
    gravity: f64,
    angular_damping: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct StepReceipt {
    pub before: State,
    pub after: State,
    pub energy_before: f64,
    pub energy_after: f64,
    pub supplied_work: f64,
    pub dissipated_energy: f64,
    /// ΔH - supplied_work + dissipated_energy; ideally zero.
    pub energy_balance_residual: f64,
    pub relative_energy_balance_residual: f64,
    /// Raw Euclidean residual of the implicit theta/ell equations.
    pub nonlinear_residual_norm: f64,
    /// Dimensionlessly scaled residual used by the Newton stopping rule.
    pub scaled_nonlinear_residual: f64,
    pub newton_iterations: usize,
    /// Midpoint angular velocity, conjugate to applied torque.
    pub midpoint_output: f64,
}

impl StepReceipt {
    pub fn closes_within(&self, relative_tolerance: f64) -> bool {
        relative_tolerance.is_finite()
            && relative_tolerance >= 0.0
            && self.relative_energy_balance_residual.is_finite()
            && self.relative_energy_balance_residual <= relative_tolerance
    }

    pub fn passivity_defect(&self) -> f64 {
        (self.energy_after - self.energy_before - self.supplied_work).max(0.0)
    }
}

impl IntrinsicPendulum {
    pub fn new(
        mass: f64,
        length: f64,
        gravity: f64,
        angular_damping: f64,
    ) -> Result<Self, Error> {
        if !mass.is_finite()
            || !length.is_finite()
            || !gravity.is_finite()
            || !angular_damping.is_finite()
        {
            return Err(Error::InvalidParameter);
        }
        if mass <= 0.0 || length <= 0.0 || gravity < 0.0 || angular_damping < 0.0 {
            return Err(Error::InvalidParameter);
        }
        Ok(Self {
            mass,
            length,
            gravity,
            angular_damping,
        })
    }

    pub fn inertia(&self) -> f64 {
        self.mass * self.length * self.length
    }

    pub fn potential_scale(&self) -> f64 {
        self.mass * self.gravity * self.length
    }

    pub fn hamiltonian(&self, state: State) -> Result<f64, Error> {
        if !state.angle.is_finite() || !state.angular_momentum.is_finite() {
            return Err(Error::InvalidState);
        }
        let energy = state.angular_momentum * state.angular_momentum / (2.0 * self.inertia())
            + self.potential_scale() * (1.0 - state.angle.cos());
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(Error::NonFiniteResult)
        }
    }

    /// Map intrinsic state to its Cartesian point on the circle.
    pub fn cartesian_position(&self, state: State) -> Result<[f64; 2], Error> {
        if !state.angle.is_finite() {
            return Err(Error::InvalidState);
        }
        Ok([
            self.length * state.angle.sin(),
            -self.length * state.angle.cos(),
        ])
    }

    /// Exact discrete gradient of m*g*L*(1-cos(theta)).
    ///
    /// Uses sin(midpoint)*sinc(delta/2), algebraically equivalent to the
    /// potential divided difference but stable for nearly equal angles.
    pub fn discrete_potential_gradient(&self, theta0: f64, theta1: f64) -> Result<f64, Error> {
        if !theta0.is_finite() || !theta1.is_finite() {
            return Err(Error::InvalidState);
        }
        let midpoint = 0.5 * (theta0 + theta1);
        let half_delta = 0.5 * (theta1 - theta0);
        let gradient = self.potential_scale() * midpoint.sin() * sinc(half_delta);
        if gradient.is_finite() {
            Ok(gradient)
        } else {
            Err(Error::NonFiniteResult)
        }
    }

    /// Derivative of the discrete potential gradient with respect to theta1.
    fn discrete_potential_gradient_derivative(&self, theta0: f64, theta1: f64) -> f64 {
        let midpoint = 0.5 * (theta0 + theta1);
        let half_delta = 0.5 * (theta1 - theta0);
        0.5 * self.potential_scale()
            * (midpoint.cos() * sinc(half_delta)
                + midpoint.sin() * sinc_derivative(half_delta))
    }

    fn residual(
        &self,
        before: State,
        after: State,
        dt: f64,
        applied_torque: f64,
    ) -> Result<(f64, f64, f64), Error> {
        let midpoint_momentum = 0.5 * (before.angular_momentum + after.angular_momentum);
        let midpoint_angular_velocity = midpoint_momentum / self.inertia();
        let potential_gradient =
            self.discrete_potential_gradient(before.angle, after.angle)?;
        let angle_residual =
            after.angle - before.angle - dt * midpoint_angular_velocity;
        let momentum_residual = after.angular_momentum - before.angular_momentum
            + dt * potential_gradient
            + dt * self.angular_damping * midpoint_angular_velocity
            - dt * applied_torque;

        let angle_scale = 1.0
            + before.angle.abs()
            + after.angle.abs()
            + (dt * midpoint_angular_velocity).abs();
        let momentum_scale = 1.0
            + before.angular_momentum.abs()
            + after.angular_momentum.abs()
            + (dt * potential_gradient).abs()
            + (dt * self.angular_damping * midpoint_angular_velocity).abs()
            + (dt * applied_torque).abs();
        let scaled_residual = ((angle_residual / angle_scale).powi(2)
            + (momentum_residual / momentum_scale).powi(2))
        .sqrt();

        if angle_residual.is_finite()
            && momentum_residual.is_finite()
            && scaled_residual.is_finite()
        {
            Ok((angle_residual, momentum_residual, scaled_residual))
        } else {
            Err(Error::NonFiniteResult)
        }
    }

    /// One implicit average-vector-field / discrete-gradient step.
    ///
    /// The applied torque is constant over the step. No post-hoc energy
    /// correction, angular-velocity clipping, or explicit fallback is used.
    pub fn step(
        &self,
        before: State,
        applied_torque: f64,
        dt: f64,
        options: SolverOptions,
    ) -> Result<StepReceipt, Error> {
        let options = options.validate()?;
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        if !applied_torque.is_finite() {
            return Err(Error::InvalidState);
        }
        let energy_before = self.hamiltonian(before)?;

        let initial_angular_velocity = before.angular_momentum / self.inertia();
        let initial_torque = -self.potential_scale() * before.angle.sin()
            - self.angular_damping * initial_angular_velocity
            + applied_torque;
        let mut after = State {
            angle: before.angle + dt * initial_angular_velocity,
            angular_momentum: before.angular_momentum + dt * initial_torque,
        };
        if !after.angle.is_finite() || !after.angular_momentum.is_finite() {
            return Err(Error::NonFiniteResult);
        }

        let mut iterations = 0usize;
        let mut converged = false;
        for iteration in 0..options.max_iterations {
            let (angle_residual, momentum_residual, norm) =
                self.residual(before, after, dt, applied_torque)?;
            if norm <= options.relative_tolerance {
                iterations = iteration;
                converged = true;
                break;
            }

            let d_gradient =
                self.discrete_potential_gradient_derivative(before.angle, after.angle);
            let a = 1.0;
            let b = -dt / (2.0 * self.inertia());
            let c = dt * d_gradient;
            let d = 1.0 + dt * self.angular_damping / (2.0 * self.inertia());
            let determinant = a * d - b * c;
            let determinant_scale = 1.0 + (a * d).abs() + (b * c).abs();
            if !determinant.is_finite()
                || determinant.abs() <= f64::EPSILON * determinant_scale
            {
                return Err(Error::SingularNewtonSystem);
            }

            let delta_angle =
                (-angle_residual * d + b * momentum_residual) / determinant;
            let delta_momentum =
                (c * angle_residual - a * momentum_residual) / determinant;
            if !delta_angle.is_finite() || !delta_momentum.is_finite() {
                return Err(Error::NonFiniteResult);
            }

            let mut accepted = false;
            let mut fraction = 1.0;
            for _ in 0..options.max_backtracks {
                let candidate = State {
                    angle: after.angle + fraction * delta_angle,
                    angular_momentum: after.angular_momentum + fraction * delta_momentum,
                };
                if candidate.angle.is_finite() && candidate.angular_momentum.is_finite() {
                    let (_, _, candidate_norm) =
                        self.residual(before, candidate, dt, applied_torque)?;
                    if candidate_norm < norm || candidate_norm <= options.relative_tolerance {
                        after = candidate;
                        accepted = true;
                        break;
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
            let (_, _, final_norm) = self.residual(before, after, dt, applied_torque)?;
            converged = final_norm <= options.relative_tolerance;
        }
        if !converged {
            return Err(Error::NoConvergence);
        }

        let (final_angle_residual, final_momentum_residual, scaled_residual) =
            self.residual(before, after, dt, applied_torque)?;
        let nonlinear_residual_norm = final_angle_residual.hypot(final_momentum_residual);
        let energy_after = self.hamiltonian(after)?;
        let midpoint_angular_velocity =
            0.5 * (before.angular_momentum + after.angular_momentum) / self.inertia();
        let supplied_work = dt * applied_torque * midpoint_angular_velocity;
        let dissipated_energy =
            dt * self.angular_damping * midpoint_angular_velocity * midpoint_angular_velocity;
        let energy_balance_residual =
            (energy_after - energy_before) - supplied_work + dissipated_energy;
        let balance_scale = (energy_before.abs()
            + energy_after.abs()
            + supplied_work.abs()
            + dissipated_energy.abs())
        .max(f64::MIN_POSITIVE);
        let relative_energy_balance_residual = energy_balance_residual.abs() / balance_scale;

        if !nonlinear_residual_norm.is_finite()
            || !scaled_residual.is_finite()
            || !supplied_work.is_finite()
            || !dissipated_energy.is_finite()
            || !energy_balance_residual.is_finite()
            || !relative_energy_balance_residual.is_finite()
        {
            return Err(Error::NonFiniteResult);
        }
        if dissipated_energy < 0.0 {
            return Err(Error::NonFiniteResult);
        }

        Ok(StepReceipt {
            before,
            after,
            energy_before,
            energy_after,
            supplied_work,
            dissipated_energy,
            energy_balance_residual,
            relative_energy_balance_residual,
            nonlinear_residual_norm,
            scaled_nonlinear_residual: scaled_residual,
            newton_iterations: iterations,
            midpoint_output: midpoint_angular_velocity,
        })
    }
}

fn sinc(value: f64) -> f64 {
    if value.abs() < 1.0e-4 {
        let square = value * value;
        1.0 - square / 6.0 + square * square / 120.0
            - square * square * square / 5040.0
    } else {
        value.sin() / value
    }
}

fn sinc_derivative(value: f64) -> f64 {
    if value.abs() < 1.0e-4 {
        let square = value * value;
        -value / 3.0 + value * square / 30.0
            - value * square * square / 840.0
    } else {
        (value * value.cos() - value.sin()) / (value * value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pendulum(damping: f64) -> IntrinsicPendulum {
        IntrinsicPendulum::new(1.0, 1.0, 9.81, damping).unwrap()
    }

    fn rk4_reference(
        model: &IntrinsicPendulum,
        mut state: State,
        dt: f64,
        steps: usize,
    ) -> State {
        let inertia = model.inertia();
        let gravity_scale = model.potential_scale();
        let damping = model.angular_damping;
        let rhs = |s: State| {
            let omega = s.angular_momentum / inertia;
            (omega, -gravity_scale * s.angle.sin() - damping * omega)
        };
        for _ in 0..steps {
            let (a1q, a1p) = rhs(state);
            let s2 = State {
                angle: state.angle + 0.5 * dt * a1q,
                angular_momentum: state.angular_momentum + 0.5 * dt * a1p,
            };
            let (a2q, a2p) = rhs(s2);
            let s3 = State {
                angle: state.angle + 0.5 * dt * a2q,
                angular_momentum: state.angular_momentum + 0.5 * dt * a2p,
            };
            let (a3q, a3p) = rhs(s3);
            let s4 = State {
                angle: state.angle + dt * a3q,
                angular_momentum: state.angular_momentum + dt * a3p,
            };
            let (a4q, a4p) = rhs(s4);
            state.angle += dt * (a1q + 2.0 * a2q + 2.0 * a3q + a4q) / 6.0;
            state.angular_momentum += dt * (a1p + 2.0 * a2p + 2.0 * a3p + a4p) / 6.0;
        }
        state
    }

    fn advance(
        model: &IntrinsicPendulum,
        initial: State,
        dt: f64,
        steps: usize,
    ) -> State {
        let mut state = initial;
        for _ in 0..steps {
            state = model.step(state, 0.0, dt, SolverOptions::default()).unwrap().after;
        }
        state
    }

    #[test]
    fn discrete_potential_gradient_satisfies_exact_difference_identity() {
        let model = pendulum(0.0);
        for i in -40..=40 {
            for j in -40..=40 {
                let theta0 = i as f64 * 0.071;
                let theta1 = j as f64 * 0.053;
                let gradient = model.discrete_potential_gradient(theta0, theta1).unwrap();
                let v0 = model.potential_scale() * (1.0 - theta0.cos());
                let v1 = model.potential_scale() * (1.0 - theta1.cos());
                let reconstructed = (theta1 - theta0) * gradient;
                let scale = 1.0 + v0.abs() + v1.abs() + reconstructed.abs();
                assert!(
                    ((v1 - v0) - reconstructed).abs() < 3.0e-13 * scale,
                    "theta0={theta0}, theta1={theta1}"
                );
            }
        }

        let theta = 0.731;
        let endpoint = model.discrete_potential_gradient(theta, theta).unwrap();
        let analytic = model.potential_scale() * theta.sin();
        assert!((endpoint - analytic).abs() < 2.0e-14);
    }

    #[test]
    fn intrinsic_geometry_preserves_pendulum_length_exactly() {
        let model = pendulum(0.0);
        let mut state = State {
            angle: 1.2,
            angular_momentum: 0.3,
        };
        for _ in 0..10_000 {
            state = model.step(state, 0.0, 0.002, SolverOptions::default()).unwrap().after;
            let [x, y] = model.cartesian_position(state).unwrap();
            let residual = x * x + y * y - model.length * model.length;
            assert!(residual.abs() < 8.0e-16);
        }
    }

    #[test]
    fn conservative_pendulum_preserves_energy_without_post_correction() {
        let model = pendulum(0.0);
        let mut state = State {
            angle: 1.4,
            angular_momentum: 0.2,
        };
        let initial_energy = model.hamiltonian(state).unwrap();
        let mut max_step_balance = 0.0_f64;
        for _ in 0..5_000 {
            let receipt = model.step(state, 0.0, 0.002, SolverOptions::default()).unwrap();
            max_step_balance = max_step_balance.max(receipt.relative_energy_balance_residual);
            state = receipt.after;
        }
        let final_energy = model.hamiltonian(state).unwrap();
        let relative_drift = (final_energy - initial_energy).abs() / initial_energy.abs();
        assert!(max_step_balance < 1.0e-11, "step closure={max_step_balance:e}");
        assert!(relative_drift < 3.0e-11, "energy drift={relative_drift:e}");
    }

    #[test]
    fn damping_and_applied_torque_are_separately_accounted() {
        let model = pendulum(0.3);
        let state = State {
            angle: 0.8,
            angular_momentum: 0.4,
        };
        let receipt = model.step(state, 0.7, 0.01, SolverOptions::default()).unwrap();
        assert!(receipt.dissipated_energy > 0.0);
        assert!(receipt.supplied_work.is_finite());
        assert!(receipt.closes_within(1.0e-11));
        assert!(receipt.passivity_defect() < 1.0e-11);
    }

    #[test]
    fn intrinsic_integrator_is_second_order_against_independent_rk4() {
        let model = pendulum(0.0);
        let initial = State {
            angle: 1.1,
            angular_momentum: 0.15,
        };
        let reference = rk4_reference(&model, initial, 1.0 / 80_000.0, 80_000);
        let dts = [0.04, 0.02, 0.01, 0.005];
        let mut errors = Vec::new();
        for dt in dts {
            let result = advance(&model, initial, dt, (1.0 / dt).round() as usize);
            let error = ((result.angle - reference.angle).powi(2)
                + (result.angular_momentum - reference.angular_momentum).powi(2))
            .sqrt();
            errors.push(error);
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (3.6..4.4).contains(&ratio),
                "expected second-order convergence; ratio={ratio}, errors={errors:?}"
            );
        }
    }

    #[test]
    fn invalid_inputs_and_solver_options_are_rejected() {
        assert_eq!(
            IntrinsicPendulum::new(0.0, 1.0, 9.81, 0.0).unwrap_err(),
            Error::InvalidParameter
        );
        let model = pendulum(0.0);
        let state = State {
            angle: 0.9,
            angular_momentum: 0.2,
        };
        assert_eq!(
            model.step(state, 0.0, 0.0, SolverOptions::default()).unwrap_err(),
            Error::InvalidTimeStep
        );
        assert_eq!(
            model
                .step(
                    state,
                    0.0,
                    0.1,
                    SolverOptions {
                        max_iterations: 0,
                        ..SolverOptions::default()
                    }
                )
                .unwrap_err(),
            Error::InvalidSolverOptions
        );
        assert_eq!(
            model
                .hamiltonian(State {
                    angle: f64::NAN,
                    angular_momentum: 0.0,
                })
                .unwrap_err(),
            Error::InvalidState
        );
    }
}

#[cfg(not(test))]
fn main() {
    let model = IntrinsicPendulum::new(1.0, 1.0, 9.81, 0.0)
        .expect("frozen benchmark model must be valid");
    let mut state = State {
        angle: 1.1,
        angular_momentum: 0.15,
    };
    println!("step,theta,ell,energy,relative_balance,newton_iterations");
    for step in 0..500 {
        let receipt = model
            .step(state, 0.0, 0.002, SolverOptions::default())
            .expect("frozen reference step must converge");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{}",
            step + 1,
            state.angle,
            state.angular_momentum,
            receipt.energy_after,
            receipt.relative_energy_balance_residual,
            receipt.newton_iterations
        );
    }
}
