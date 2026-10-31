// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Standalone nonlinear discrete-gradient reference. No third-party crates.

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
    pub position: f64,
    pub momentum: f64,
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

/// One-degree-of-freedom port-Hamiltonian oscillator.
///
/// H(q,p) = p²/(2m) + k q²/2 + beta q⁴/4
///
/// q_dot = dH/dp
/// p_dot = -dH/dq - gamma * dH/dp + u
///
/// The canonical interconnection is skew, gamma >= 0 is a positive
/// semidefinite dissipation operator, and u is an external power port.
#[derive(Clone, Copy, Debug)]
pub struct QuarticPortHamiltonian {
    mass: f64,
    stiffness: f64,
    quartic: f64,
    damping: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct StepReceipt {
    pub before: State,
    pub after: State,
    pub energy_before: f64,
    pub energy_after: f64,
    pub supplied_work: f64,
    pub dissipated_energy: f64,
    /// ΔH - supplied_work + dissipated_energy; should be zero.
    pub energy_balance_residual: f64,
    pub relative_energy_balance_residual: f64,
    /// Dimensionlessly scaled residual of the two implicit equations.
    /// Euclidean norm of the unscaled implicit-equation residual.
    pub nonlinear_residual_norm: f64,
    /// Dimensionlessly scaled residual used by the Newton stopping rule.
    pub scaled_nonlinear_residual: f64,
    pub nonlinear_iterations: usize,
    /// Midpoint port output y = (p0 + p1) / (2m).
    pub midpoint_output: f64,
}

impl StepReceipt {
    pub fn closes_within(&self, relative_tolerance: f64) -> bool {
        relative_tolerance.is_finite()
            && relative_tolerance >= 0.0
            && self.relative_energy_balance_residual.is_finite()
            && self.relative_energy_balance_residual <= relative_tolerance
    }

    /// Positive values indicate stored energy rose by more than port work.
    pub fn passivity_defect(&self) -> f64 {
        (self.energy_after - self.energy_before - self.supplied_work).max(0.0)
    }
}

impl QuarticPortHamiltonian {
    pub fn new(
        mass: f64,
        stiffness: f64,
        quartic: f64,
        damping: f64,
    ) -> Result<Self, Error> {
        if !mass.is_finite()
            || !stiffness.is_finite()
            || !quartic.is_finite()
            || !damping.is_finite()
        {
            return Err(Error::InvalidParameter);
        }
        if mass <= 0.0 || stiffness < 0.0 || quartic < 0.0 || damping < 0.0 {
            return Err(Error::InvalidParameter);
        }
        Ok(Self {
            mass,
            stiffness,
            quartic,
            damping,
        })
    }

    pub fn hamiltonian(&self, state: State) -> Result<f64, Error> {
        if !state.position.is_finite() || !state.momentum.is_finite() {
            return Err(Error::InvalidState);
        }
        let q = state.position;
        let p = state.momentum;
        let energy = p * p / (2.0 * self.mass)
            + 0.5 * self.stiffness * q * q
            + 0.25 * self.quartic * q * q * q * q;
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(Error::NonFiniteResult)
        }
    }

    /// Exact discrete gradient of V(q) = k q²/2 + beta q⁴/4.
    ///
    /// It satisfies (q1-q0) * grad_bar = V(q1)-V(q0), including the
    /// coincident-endpoint limit. The factored quartic expression avoids
    /// subtracting two nearly equal fourth powers.
    pub fn discrete_potential_gradient(&self, q0: f64, q1: f64) -> Result<f64, Error> {
        if !q0.is_finite() || !q1.is_finite() {
            return Err(Error::InvalidState);
        }
        let gradient = if q0 == q1 {
            self.stiffness * q0 + self.quartic * q0 * q0 * q0
        } else {
            self.stiffness * 0.5 * (q0 + q1)
                + 0.25
                    * self.quartic
                    * (q1 * q1 * q1 + q1 * q1 * q0 + q1 * q0 * q0 + q0 * q0 * q0)
        };
        if gradient.is_finite() {
            Ok(gradient)
        } else {
            Err(Error::NonFiniteResult)
        }
    }

    fn potential_gradient_wrt_endpoint(&self, q0: f64, q1: f64) -> f64 {
        0.5 * self.stiffness
            + 0.25
                * self.quartic
                * (3.0 * q1 * q1 + 2.0 * q1 * q0 + q0 * q0)
    }

    fn residual(
        &self,
        before: State,
        after: State,
        dt: f64,
        input: f64,
    ) -> Result<(f64, f64, f64), Error> {
        let potential_gradient =
            self.discrete_potential_gradient(before.position, after.position)?;
        let midpoint_output = (before.momentum + after.momentum) / (2.0 * self.mass);
        let position_residual =
            after.position - before.position - dt * midpoint_output;
        let momentum_residual = after.momentum - before.momentum
            + dt * potential_gradient
            + dt * self.damping * midpoint_output
            - dt * input;

        // Each equation is scaled by the magnitude of its own state and
        // increment terms. The raw residual is separately recomputed for
        // verification; this scale is only a Newton stopping criterion.
        let q_scale = 1.0
            + before.position.abs()
            + after.position.abs()
            + (dt * midpoint_output).abs();
        let p_scale = 1.0
            + before.momentum.abs()
            + after.momentum.abs()
            + (dt * potential_gradient).abs()
            + (dt * self.damping * midpoint_output).abs()
            + (dt * input).abs();
        let scaled = ((position_residual / q_scale).powi(2)
            + (momentum_residual / p_scale).powi(2))
        .sqrt();

        if position_residual.is_finite()
            && momentum_residual.is_finite()
            && scaled.is_finite()
        {
            Ok((position_residual, momentum_residual, scaled))
        } else {
            Err(Error::NonFiniteResult)
        }
    }

    /// Advance one implicit-average-vector-field / discrete-gradient step.
    ///
    /// The update solves
    ///   x1 - x0 = dt * (J - R) * grad_bar(H) + dt * G * u.
    ///
    /// A step is rejected unless the implicit equations converge. No energy
    /// correction, clipping, or post-hoc state normalization is performed.
    pub fn step(
        &self,
        before: State,
        input: f64,
        dt: f64,
        options: SolverOptions,
    ) -> Result<StepReceipt, Error> {
        let options = options.validate()?;
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        if !input.is_finite() {
            return Err(Error::InvalidState);
        }
        // Validate the initial energy before starting the nonlinear solve.
        let energy_before = self.hamiltonian(before)?;

        // Explicit-Euler predictor; only an initial guess, never an accepted
        // state unless it independently satisfies the implicit equations.
        let initial_velocity = before.momentum / self.mass;
        let initial_force = -self.stiffness * before.position
            - self.quartic * before.position.powi(3)
            - self.damping * initial_velocity
            + input;
        let mut after = State {
            position: before.position + dt * initial_velocity,
            momentum: before.momentum + dt * initial_force,
        };
        if !after.position.is_finite() || !after.momentum.is_finite() {
            return Err(Error::NonFiniteResult);
        }

        let mut iterations = 0usize;
        let mut scaled_residual = f64::INFINITY;
        let mut converged = false;

        for iteration in 0..options.max_iterations {
            let (fq, fp, norm) = self.residual(before, after, dt, input)?;
            scaled_residual = norm;
            if norm <= options.relative_tolerance {
                iterations = iteration;
                converged = true;
                break;
            }

            let d_vbar_d_q1 =
                self.potential_gradient_wrt_endpoint(before.position, after.position);
            let a = 1.0;
            let b = -dt / (2.0 * self.mass);
            let c = dt * d_vbar_d_q1;
            let d = 1.0 + dt * self.damping / (2.0 * self.mass);
            let determinant = a * d - b * c;
            let determinant_scale = 1.0 + (a * d).abs() + (b * c).abs();
            if !determinant.is_finite()
                || determinant.abs() <= f64::EPSILON * determinant_scale
            {
                return Err(Error::SingularNewtonSystem);
            }

            // Solve [a b; c d] * delta = -[fq fp].
            let delta_q = (-fq * d + b * fp) / determinant;
            let delta_p = (c * fq - a * fp) / determinant;
            if !delta_q.is_finite() || !delta_p.is_finite() {
                return Err(Error::NonFiniteResult);
            }

            // Backtracking prevents a poor predictor or large step from
            // turning Newton overshoot into an apparently accepted result.
            let mut accepted = false;
            let mut fraction = 1.0;
            for _ in 0..options.max_backtracks {
                let candidate = State {
                    position: after.position + fraction * delta_q,
                    momentum: after.momentum + fraction * delta_p,
                };
                if candidate.position.is_finite() && candidate.momentum.is_finite() {
                    let (_, _, candidate_norm) =
                        self.residual(before, candidate, dt, input)?;
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
            let (_, _, final_norm) = self.residual(before, after, dt, input)?;
            scaled_residual = final_norm;
            converged = final_norm <= options.relative_tolerance;
        }
        if !converged {
            return Err(Error::NoConvergence);
        }

        let (final_fq, final_fp, final_scaled_residual) =
            self.residual(before, after, dt, input)?;
        let final_nonlinear_residual_norm = final_fq.hypot(final_fp);
        let energy_after = self.hamiltonian(after)?;
        let midpoint_output = (before.momentum + after.momentum) / (2.0 * self.mass);
        let supplied_work = dt * input * midpoint_output;
        let dissipated_energy = dt * self.damping * midpoint_output * midpoint_output;
        let energy_balance_residual =
            (energy_after - energy_before) - supplied_work + dissipated_energy;
        let balance_scale = (energy_before.abs()
            + energy_after.abs()
            + supplied_work.abs()
            + dissipated_energy.abs())
        .max(f64::MIN_POSITIVE);
        let relative_energy_balance_residual = energy_balance_residual.abs() / balance_scale;

        if !supplied_work.is_finite()
            || !dissipated_energy.is_finite()
            || !energy_balance_residual.is_finite()
            || !relative_energy_balance_residual.is_finite()
            || !final_scaled_residual.is_finite()
            || !final_nonlinear_residual_norm.is_finite()
        {
            return Err(Error::NonFiniteResult);
        }
        // Dissipation is structurally nonnegative; a negative value can only
        // arise from invalid arithmetic or a future sign regression.
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
            nonlinear_residual_norm: final_nonlinear_residual_norm,
            scaled_nonlinear_residual: final_scaled_residual,
            nonlinear_iterations: iterations,
            midpoint_output,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(damping: f64) -> QuarticPortHamiltonian {
        QuarticPortHamiltonian::new(1.0, 1.0, 0.75, damping).unwrap()
    }

    fn potential(model: &QuarticPortHamiltonian, q: f64) -> f64 {
        0.5 * model.stiffness * q * q
            + 0.25 * model.quartic * q * q * q * q
    }

    fn rk4_reference(
        model: &QuarticPortHamiltonian,
        mut state: State,
        dt: f64,
        steps: usize,
    ) -> State {
        let rhs = |s: State| {
            let qdot = s.momentum / model.mass;
            let pdot = -model.stiffness * s.position
                - model.quartic * s.position.powi(3)
                - model.damping * qdot;
            (qdot, pdot)
        };
        for _ in 0..steps {
            let (a1q, a1p) = rhs(state);
            let s2 = State {
                position: state.position + 0.5 * dt * a1q,
                momentum: state.momentum + 0.5 * dt * a1p,
            };
            let (a2q, a2p) = rhs(s2);
            let s3 = State {
                position: state.position + 0.5 * dt * a2q,
                momentum: state.momentum + 0.5 * dt * a2p,
            };
            let (a3q, a3p) = rhs(s3);
            let s4 = State {
                position: state.position + dt * a3q,
                momentum: state.momentum + dt * a3p,
            };
            let (a4q, a4p) = rhs(s4);
            state.position += dt * (a1q + 2.0 * a2q + 2.0 * a3q + a4q) / 6.0;
            state.momentum += dt * (a1p + 2.0 * a2p + 2.0 * a3p + a4p) / 6.0;
        }
        state
    }

    fn advance(
        model: &QuarticPortHamiltonian,
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
    fn discrete_gradient_satisfies_exact_potential_difference() {
        let model = model(0.0);
        for i in -20..=20 {
            for j in -20..=20 {
                let q0 = i as f64 * 0.13;
                let q1 = j as f64 * 0.17;
                let gradient = model.discrete_potential_gradient(q0, q1).unwrap();
                let difference = potential(&model, q1) - potential(&model, q0);
                let reconstructed = (q1 - q0) * gradient;
                let scale = 1.0 + difference.abs() + reconstructed.abs();
                assert!(
                    (difference - reconstructed).abs() < 2.0e-13 * scale,
                    "q0={q0}, q1={q1}, difference={difference}, reconstructed={reconstructed}"
                );
            }
        }
        let q = 0.731;
        let endpoint_gradient = model.discrete_potential_gradient(q, q).unwrap();
        let analytic = model.stiffness * q + model.quartic * q.powi(3);
        assert!((endpoint_gradient - analytic).abs() < 1.0e-14);
    }

    #[test]
    fn conservative_quartic_oscillator_preserves_energy_without_post_correction() {
        let system = model(0.0);
        let mut state = State {
            position: 0.9,
            momentum: 0.25,
        };
        let initial_energy = system.hamiltonian(state).unwrap();
        let mut maximum_step_balance = 0.0_f64;
        for _ in 0..2_000 {
            let receipt = system.step(state, 0.0, 0.01, SolverOptions::default()).unwrap();
            maximum_step_balance =
                maximum_step_balance.max(receipt.relative_energy_balance_residual);
            assert!(receipt.nonlinear_iterations > 0);
            state = receipt.after;
        }
        let final_energy = system.hamiltonian(state).unwrap();
        let relative_drift = (final_energy - initial_energy).abs() / initial_energy.abs();
        assert!(maximum_step_balance < 2.0e-12, "step balance={maximum_step_balance:e}");
        assert!(relative_drift < 2.0e-11, "energy drift={relative_drift:e}");
    }

    #[test]
    fn dissipation_is_nonnegative_and_attributed_to_the_balance() {
        let system = model(0.4);
        let state = State {
            position: 0.8,
            momentum: 0.6,
        };
        let receipt = system.step(state, 0.0, 0.03, SolverOptions::default()).unwrap();
        assert!(receipt.dissipated_energy > 0.0);
        assert!(receipt.energy_after < receipt.energy_before);
        assert!(receipt.closes_within(2.0e-12));
        assert!(receipt.passivity_defect() <= 2.0e-12);
    }

    #[test]
    fn external_port_work_is_separate_from_dissipation() {
        let system = model(0.2);
        let state = State {
            position: 0.3,
            momentum: -0.2,
        };
        let receipt = system.step(state, 0.7, 0.02, SolverOptions::default()).unwrap();
        assert!(receipt.supplied_work.is_finite());
        assert!(receipt.dissipated_energy >= 0.0);
        assert!(receipt.closes_within(2.0e-12));
    }

    #[test]
    fn average_vector_field_step_is_second_order_against_rk4_reference() {
        let system = model(0.0);
        let initial = State {
            position: 0.9,
            momentum: 0.25,
        };
        let reference = rk4_reference(&system, initial, 1.0 / 40_000.0, 40_000);
        let dts = [0.1, 0.05, 0.025, 0.0125];
        let mut errors = Vec::new();
        for dt in dts {
            let result = advance(&system, initial, dt, (1.0 / dt).round() as usize);
            errors.push(
                ((result.position - reference.position).powi(2)
                    + (result.momentum - reference.momentum).powi(2))
                .sqrt(),
            );
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (3.6..4.4).contains(&ratio),
                "expected second-order convergence, ratio={ratio}, errors={errors:?}"
            );
        }
    }

    #[test]
    fn invalid_inputs_and_solver_options_fail_closed() {
        assert_eq!(
            QuarticPortHamiltonian::new(0.0, 1.0, 1.0, 0.0).unwrap_err(),
            Error::InvalidParameter
        );
        let system = model(0.0);
        let state = State {
            position: 1.0,
            momentum: 0.0,
        };
        assert_eq!(
            system.step(state, 0.0, 0.0, SolverOptions::default()).unwrap_err(),
            Error::InvalidTimeStep
        );
        assert_eq!(
            system
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
            system
                .hamiltonian(State {
                    position: f64::NAN,
                    momentum: 0.0,
                })
                .unwrap_err(),
            Error::InvalidState
        );
    }
}

#[cfg(not(test))]
fn main() {
    let system = QuarticPortHamiltonian::new(1.0, 1.0, 0.75, 0.0)
        .expect("fixed benchmark model must be valid");
    let mut state = State {
        position: 0.9,
        momentum: 0.25,
    };
    println!("step,q,p,energy,relative_balance,newton_iterations");
    for step in 0..100 {
        let receipt = system
            .step(state, 0.0, 0.01, SolverOptions::default())
            .expect("fixed benchmark step must converge");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{}",
            step + 1,
            state.position,
            state.momentum,
            receipt.energy_after,
            receipt.relative_energy_balance_residual,
            receipt.nonlinear_iterations
        );
    }
}
