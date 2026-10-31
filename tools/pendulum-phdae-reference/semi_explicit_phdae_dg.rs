// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Semi-explicit port-Hamiltonian DAE discrete-gradient pendulum reference.
// The method follows the multibody construction in Kinon, Morandin &
// Schulze (Applied Numerical Mathematics 223, 2026), specialized to a
// frictionless Cartesian circle constraint and constant gravity.

use std::f64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    InvalidParameter,
    InvalidTimeStep,
    InvalidPositionConstraint,
    InvalidInitialVelocityConstraint,
    NonFiniteInput,
    InvalidSolverOptions,
    SingularMultiplierSolve,
    NoConvergence,
    NonFiniteResult,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub position: [f64; 2],
    pub velocity: [f64; 2],
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

#[derive(Clone, Copy, Debug)]
pub struct SemiExplicitPendulumDg {
    mass: f64,
    length: f64,
    gravity: [f64; 2],
    constraint_tolerance: f64,
    multiplier_tolerance: f64,
    max_multiplier_iterations: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct StepReceipt {
    pub before: State,
    pub after: State,
    pub midpoint_position: [f64; 2],
    pub midpoint_velocity: [f64; 2],
    pub hamiltonian_before: f64,
    pub hamiltonian_after: f64,
    pub supplied_work: f64,
    /// Signed work of the discrete holonomic reaction: -h*lambda*qbar·vbar.
    /// It should be zero to the multiplier-solve envelope.
    pub constraint_reaction_work: f64,
    /// ΔH - supplied_work - constraint_reaction_work.
    pub energy_balance_residual: f64,
    pub relative_energy_balance_residual: f64,
    /// Position-level holonomic constraint at the accepted endpoint.
    pub position_constraint_residual: f64,
    pub relative_position_constraint_residual: f64,
    /// Algebraic discrete velocity constraint Dg(q0,q1)*vbar = 0.
    pub midpoint_velocity_constraint_residual: f64,
    pub relative_midpoint_velocity_constraint_residual: f64,
    /// Continuous endpoint hidden constraint q1·v1 = 0, measured separately.
    /// The semi-explicit DG construction does not generally enforce this exactly.
    pub endpoint_velocity_constraint_residual: f64,
    pub scaled_endpoint_velocity_constraint_residual: f64,
    pub constraint_multiplier: f64,
    pub nonlinear_multiplier_residual: f64,
    pub scaled_multiplier_residual: f64,
    pub multiplier_iterations: usize,
}

impl StepReceipt {
    pub fn closes_energy_balance_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_energy_balance_residual.is_finite()
            && self.relative_energy_balance_residual <= tolerance
    }

    pub fn position_constraint_holds_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_position_constraint_residual.is_finite()
            && self.relative_position_constraint_residual <= tolerance
    }

    pub fn midpoint_velocity_constraint_holds_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_midpoint_velocity_constraint_residual.is_finite()
            && self.relative_midpoint_velocity_constraint_residual <= tolerance
    }
}

fn dot(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn norm(a: [f64; 2]) -> f64 {
    dot(a, a).sqrt()
}

fn add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn scale(a: [f64; 2], factor: f64) -> [f64; 2] {
    [a[0] * factor, a[1] * factor]
}

/// Evaluate v_bar(lambda) and the derivative of the discrete algebraic
/// constraint with respect to lambda.
///
/// From q1-q0 = h*v_bar and
/// m*(v1-v0) = h*m*g + h*f_ext - h*lambda*q_bar,
/// with q_bar = q0 + h/2*v_bar and v1 = 2*v_bar-v0, we obtain
///
/// v_bar = (2m*v0 + h*m*g + h*f_ext - h*lambda*q0)
///         / (2m + h^2*lambda/2).
///
/// The exact discrete Jacobian of c(q)=1/2(|q|^2-L^2) is q_bar, and the
/// algebraic condition is q_bar·v_bar = q0·v_bar + h/2*|v_bar|^2 = 0.
fn midpoint_velocity_and_constraint(
    mass: f64,
    initial_position: [f64; 2],
    initial_velocity: [f64; 2],
    gravity: [f64; 2],
    applied_force: [f64; 2],
    dt: f64,
    multiplier: f64,
) -> Result<([f64; 2], f64, f64), Error> {
    let numerator = add(
        add(
            scale(initial_velocity, 2.0 * mass),
            scale(gravity, dt * mass),
        ),
        sub(
            scale(applied_force, dt),
            scale(initial_position, dt * multiplier),
        ),
    );
    let denominator = 2.0 * mass + 0.5 * dt * dt * multiplier;
    let denominator_scale =
        (2.0 * mass).abs() + (0.5 * dt * dt * multiplier).abs();
    if !denominator.is_finite()
        || denominator.abs() <= f64::EPSILON * denominator_scale.max(f64::MIN_POSITIVE)
    {
        return Err(Error::SingularMultiplierSolve);
    }

    let midpoint_velocity = scale(numerator, 1.0 / denominator);
    let numerator_derivative = scale(initial_position, -dt);
    let denominator_derivative = 0.5 * dt * dt;
    let derivative_numerator = sub(
        scale(numerator_derivative, denominator),
        scale(numerator, denominator_derivative),
    );
    let midpoint_velocity_derivative =
        scale(derivative_numerator, 1.0 / (denominator * denominator));

    let residual =
        dot(initial_position, midpoint_velocity)
            + 0.5 * dt * dot(midpoint_velocity, midpoint_velocity);
    let derivative = dot(initial_position, midpoint_velocity_derivative)
        + dt * dot(midpoint_velocity, midpoint_velocity_derivative);

    if !midpoint_velocity.iter().all(|value| value.is_finite())
        || !residual.is_finite()
        || !derivative.is_finite()
    {
        return Err(Error::NonFiniteResult);
    }
    Ok((midpoint_velocity, residual, derivative))
}

impl SemiExplicitPendulumDg {
    pub fn new(
        mass: f64,
        length: f64,
        gravity: [f64; 2],
    ) -> Result<Self, Error> {
        if !mass.is_finite()
            || !length.is_finite()
            || !gravity.iter().all(|value| value.is_finite())
        {
            return Err(Error::InvalidParameter);
        }
        if mass <= 0.0 || length <= 0.0 {
            return Err(Error::InvalidParameter);
        }
        Ok(Self {
            mass,
            length,
            gravity,
            constraint_tolerance: 1.0e-10,
            multiplier_tolerance: 1.0e-13,
            max_multiplier_iterations: 32,
        })
    }

    pub fn hamiltonian(&self, state: State) -> Result<f64, Error> {
        if !state.position.iter().all(|value| value.is_finite())
            || !state.velocity.iter().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        // Add a constant so the hanging equilibrium has zero potential.
        let kinetic = 0.5 * self.mass * dot(state.velocity, state.velocity);
        let potential = -self.mass * dot(self.gravity, state.position)
            + self.mass * norm(self.gravity) * self.length;
        let energy = kinetic + potential;
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(Error::NonFiniteResult)
        }
    }

    pub fn position_constraint(&self, state: State) -> f64 {
        0.5 * (dot(state.position, state.position) - self.length * self.length)
    }

    pub fn endpoint_velocity_constraint(&self, state: State) -> f64 {
        dot(state.position, state.velocity)
    }

    fn validate_initial(&self, state: State) -> Result<(), Error> {
        if !state.position.iter().all(|value| value.is_finite())
            || !state.velocity.iter().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        let position_residual =
            (dot(state.position, state.position) - self.length * self.length).abs();
        let position_scale = (1.0
            + dot(state.position, state.position)
            + self.length * self.length)
        .max(f64::MIN_POSITIVE);
        if position_residual > self.constraint_tolerance * position_scale {
            return Err(Error::InvalidPositionConstraint);
        }
        let tangent_residual = dot(state.position, state.velocity).abs();
        let tangent_scale =
            (1.0 + self.length * norm(state.velocity)).max(f64::MIN_POSITIVE);
        if tangent_residual > self.constraint_tolerance * tangent_scale {
            return Err(Error::InvalidInitialVelocityConstraint);
        }
        Ok(())
    }

    /// Advance one semi-explicit pHDAE discrete-gradient step.
    ///
    /// This is the specialized multibody construction in Kinon, Morandin &
    /// Schulze (2026), not a new general DAE method:
    ///
    ///   q1-q0 = h*v_bar
    ///   m*(v1-v0) = h*m*g + h*f_ext - h*lambda*q_bar
    ///   q_bar·v_bar = 0
    ///
    /// For c(q)=1/2(|q|^2-L^2), Dg(q0,q1)=q_bar is an exact discrete
    /// Jacobian. Therefore c(q1)=c(q0) exactly up to nonlinear solve error.
    /// The same algebraic constraint makes the discrete constraint reaction
    /// do zero work, yielding the exact discrete energy/power balance.
    ///
    /// The endpoint hidden velocity condition q1·v1=0 is NOT generally exact
    /// for this scheme. It is returned as a separate measurement.
    pub fn step(
        &self,
        before: State,
        applied_force: [f64; 2],
        dt: f64,
        options: SolverOptions,
    ) -> Result<StepReceipt, Error> {
        let options = options.validate()?;
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        if !applied_force.iter().all(|value| value.is_finite()) {
            return Err(Error::NonFiniteInput);
        }
        self.validate_initial(before)?;
        let energy_before = self.hamiltonian(before)?;

        // Continuous multiplier estimate from d/dt(q·v)=0. It supplies
        // a stable Newton initial guess but is not used as the final multiplier.
        let gravity_plus_force = add(self.gravity, scale(applied_force, 1.0 / self.mass));
        let mut multiplier =
            self.mass * (dot(before.velocity, before.velocity)
                + dot(before.position, gravity_plus_force))
                / (self.length * self.length);
        if !multiplier.is_finite() {
            return Err(Error::NonFiniteResult);
        }

        let mut iterations = 0usize;
        let mut converged = false;
        for iteration in 0..options.max_iterations {
            let (midpoint_velocity, residual, derivative) =
                midpoint_velocity_and_constraint(
                    self.mass,
                    before.position,
                    before.velocity,
                    self.gravity,
                    applied_force,
                    dt,
                    multiplier,
                )?;
            let midpoint_position =
                add(before.position, scale(midpoint_velocity, 0.5 * dt));
            let residual_scale = (1.0
                + norm(midpoint_position) * norm(midpoint_velocity))
            .max(f64::MIN_POSITIVE);
            let scaled_residual = residual.abs() / residual_scale;
            if scaled_residual <= options.relative_tolerance {
                iterations = iteration;
                converged = true;
                break;
            }
            let derivative_scale = (1.0
                + self.length * norm(midpoint_velocity_derivative_placeholder(
                    before.position,
                    midpoint_velocity,
                    dt,
                )))
            .max(f64::MIN_POSITIVE);
            if !derivative.is_finite()
                || derivative.abs() <= f64::EPSILON * derivative_scale
            {
                return Err(Error::SingularMultiplierSolve);
            }

            let delta = -residual / derivative;
            if !delta.is_finite() {
                return Err(Error::NonFiniteResult);
            }
            let mut accepted = false;
            let mut fraction = 1.0;
            for _ in 0..options.max_backtracks {
                let candidate_multiplier = multiplier + fraction * delta;
                if candidate_multiplier.is_finite() {
                    if let Ok((candidate_velocity, candidate_residual, _)) =
                        midpoint_velocity_and_constraint(
                            self.mass,
                            before.position,
                            before.velocity,
                            self.gravity,
                            applied_force,
                            dt,
                            candidate_multiplier,
                        )
                    {
                        let candidate_position =
                            add(before.position, scale(candidate_velocity, 0.5 * dt));
                        let candidate_scale = (1.0
                            + norm(candidate_position) * norm(candidate_velocity))
                        .max(f64::MIN_POSITIVE);
                        let candidate_scaled =
                            candidate_residual.abs() / candidate_scale;
                        if candidate_scaled < scaled_residual
                            || candidate_scaled <= options.relative_tolerance
                        {
                            multiplier = candidate_multiplier;
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
            let (_, residual, _) = midpoint_velocity_and_constraint(
                self.mass,
                before.position,
                before.velocity,
                self.gravity,
                applied_force,
                dt,
                multiplier,
            )?;
            let midpoint_velocity = midpoint_velocity_and_constraint(
                self.mass,
                before.position,
                before.velocity,
                self.gravity,
                applied_force,
                dt,
                multiplier,
            )?
            .0;
            let midpoint_position =
                add(before.position, scale(midpoint_velocity, 0.5 * dt));
            let residual_scale = (1.0
                + norm(midpoint_position) * norm(midpoint_velocity))
            .max(f64::MIN_POSITIVE);
            converged = residual.abs() / residual_scale <= options.relative_tolerance;
        }
        if !converged {
            return Err(Error::NoConvergence);
        }

        let (midpoint_velocity, nonlinear_multiplier_residual, _) =
            midpoint_velocity_and_constraint(
                self.mass,
                before.position,
                before.velocity,
                self.gravity,
                applied_force,
                dt,
                multiplier,
            )?;
        let midpoint_position =
            add(before.position, scale(midpoint_velocity, 0.5 * dt));
        let after = State {
            position: add(before.position, scale(midpoint_velocity, dt)),
            velocity: sub(scale(midpoint_velocity, 2.0), before.velocity),
        };
        if !after.position.iter().all(|value| value.is_finite())
            || !after.velocity.iter().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteResult);
        }

        let position_constraint_residual =
            dot(after.position, after.position) - self.length * self.length;
        let relative_position_constraint_residual = position_constraint_residual.abs()
            / (1.0 + dot(after.position, after.position) + self.length * self.length);
        let midpoint_velocity_constraint_residual =
            dot(midpoint_position, midpoint_velocity);
        let relative_midpoint_velocity_constraint_residual =
            midpoint_velocity_constraint_residual.abs()
                / (1.0 + norm(midpoint_position) * norm(midpoint_velocity));
        let endpoint_velocity_constraint_residual =
            dot(after.position, after.velocity);
        let scaled_endpoint_velocity_constraint_residual =
            endpoint_velocity_constraint_residual.abs()
                / (1.0 + self.length * norm(after.velocity));

        if !relative_position_constraint_residual.is_finite()
            || !relative_midpoint_velocity_constraint_residual.is_finite()
            || !endpoint_velocity_constraint_residual.is_finite()
            || !scaled_endpoint_velocity_constraint_residual.is_finite()
        {
            return Err(Error::NonFiniteResult);
        }
        if relative_position_constraint_residual > self.constraint_tolerance
            || relative_midpoint_velocity_constraint_residual > self.constraint_tolerance
        {
            return Err(Error::NoConvergence);
        }

        let energy_after = self.hamiltonian(after)?;
        let supplied_work = dt * dot(applied_force, midpoint_velocity);
        let constraint_reaction_work =
            -dt * multiplier * midpoint_velocity_constraint_residual;
        let energy_balance_residual = energy_after - energy_before
            - supplied_work
            - constraint_reaction_work;
        let balance_scale = (energy_before.abs()
            + energy_after.abs()
            + supplied_work.abs()
            + constraint_reaction_work.abs())
        .max(f64::MIN_POSITIVE);
        let relative_energy_balance_residual =
            energy_balance_residual.abs() / balance_scale;
        let residual_scale = (1.0
            + norm(midpoint_position) * norm(midpoint_velocity))
        .max(f64::MIN_POSITIVE);
        let scaled_multiplier_residual =
            nonlinear_multiplier_residual.abs() / residual_scale;

        if !supplied_work.is_finite()
            || !constraint_reaction_work.is_finite()
            || !energy_balance_residual.is_finite()
            || !relative_energy_balance_residual.is_finite()
            || !scaled_multiplier_residual.is_finite()
        {
            return Err(Error::NonFiniteResult);
        }

        Ok(StepReceipt {
            before,
            after,
            midpoint_position,
            midpoint_velocity,
            hamiltonian_before: energy_before,
            hamiltonian_after: energy_after,
            supplied_work,
            constraint_reaction_work,
            energy_balance_residual,
            relative_energy_balance_residual,
            position_constraint_residual,
            relative_position_constraint_residual,
            midpoint_velocity_constraint_residual,
            relative_midpoint_velocity_constraint_residual,
            endpoint_velocity_constraint_residual,
            scaled_endpoint_velocity_constraint_residual,
            constraint_multiplier: multiplier,
            nonlinear_multiplier_residual,
            scaled_multiplier_residual,
            multiplier_iterations: iterations,
        })
    }
}

// Retained as an explicit helper for the derivative scale check above.
fn midpoint_velocity_derivative_placeholder(
    _initial_position: [f64; 2],
    _midpoint_velocity: [f64; 2],
    _dt: f64,
) -> [f64; 2] {
    [0.0, 0.0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> SemiExplicitPendulumDg {
        SemiExplicitPendulumDg::new(1.0, 1.0, [0.0, -9.81]).unwrap()
    }

    fn initial_state(angle: f64, angular_momentum: f64) -> State {
        State {
            position: [angle.sin(), -angle.cos()],
            velocity: [
                angular_momentum * angle.cos(),
                angular_momentum * angle.sin(),
            ],
        }
    }

    fn rk4_reference(mut theta: f64, mut angular_momentum: f64, dt: f64, steps: usize) -> State {
        let rhs = |angle: f64, momentum: f64| (momentum, -9.81 * angle.sin());
        for _ in 0..steps {
            let (a1q, a1p) = rhs(theta, angular_momentum);
            let (a2q, a2p) = rhs(
                theta + 0.5 * dt * a1q,
                angular_momentum + 0.5 * dt * a1p,
            );
            let (a3q, a3p) = rhs(
                theta + 0.5 * dt * a2q,
                angular_momentum + 0.5 * dt * a2p,
            );
            let (a4q, a4p) = rhs(
                theta + dt * a3q,
                angular_momentum + dt * a3p,
            );
            theta += dt * (a1q + 2.0 * a2q + 2.0 * a3q + a4q) / 6.0;
            angular_momentum += dt * (a1p + 2.0 * a2p + 2.0 * a3p + a4p) / 6.0;
        }
        State {
            position: [theta.sin(), -theta.cos()],
            velocity: [
                angular_momentum * theta.cos(),
                angular_momentum * theta.sin(),
            ],
        }
    }

    fn advance(mut state: State, dt: f64, steps: usize) -> State {
        let model = model();
        for _ in 0..steps {
            state = model
                .step(state, [0.0, 0.0], dt, SolverOptions::default())
                .unwrap()
                .after;
        }
        state
    }

    #[test]
    fn discrete_jacobian_preserves_position_and_midpoint_velocity_constraints() {
        let model = model();
        let mut state = initial_state(1.1, 0.15);
        for _ in 0..5000 {
            let receipt = model
                .step(state, [0.0, 0.0], 0.002, SolverOptions::default())
                .unwrap();
            assert!(receipt.position_constraint_holds_within(1.0e-12));
            assert!(receipt.midpoint_velocity_constraint_holds_within(1.0e-12));
            state = receipt.after;
        }
    }

    #[test]
    fn energy_and_discrete_power_balance_close_without_post_correction() {
        let model = model();
        let mut state = initial_state(1.1, 0.15);
        let initial_energy = model.hamiltonian(state).unwrap();
        let mut maximum_step_residual = 0.0_f64;
        let mut maximum_reaction_work = 0.0_f64;
        for _ in 0..5000 {
            let receipt = model
                .step(state, [0.0, 0.0], 0.002, SolverOptions::default())
                .unwrap();
            maximum_step_residual =
                maximum_step_residual.max(receipt.relative_energy_balance_residual);
            maximum_reaction_work =
                maximum_reaction_work.max(receipt.constraint_reaction_work.abs());
            state = receipt.after;
        }
        let final_energy = model.hamiltonian(state).unwrap();
        let relative_drift = (final_energy - initial_energy).abs() / initial_energy.abs();
        assert!(maximum_step_residual < 2.0e-12);
        assert!(maximum_reaction_work < 2.0e-12);
        assert!(relative_drift < 2.0e-11);
    }

    #[test]
    fn endpoint_velocity_constraint_is_reported_and_refines_quadratically() {
        let initial = initial_state(1.1, 0.15);
        let mut maxima = Vec::new();
        for dt in [0.04, 0.02, 0.01, 0.005] {
            let model = model();
            let mut state = initial;
            let mut maximum = 0.0_f64;
            for _ in 0..(1.0 / dt).round() as usize {
                let receipt = model
                    .step(state, [0.0, 0.0], dt, SolverOptions::default())
                    .unwrap();
                maximum = maximum.max(receipt.endpoint_velocity_constraint_residual.abs());
                state = receipt.after;
            }
            maxima.push(maximum);
        }
        for pair in maxima.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (3.6..4.4).contains(&ratio),
                "endpoint velocity residual should converge quadratically: {maxima:?}"
            );
        }
    }

    #[test]
    fn external_force_work_and_constraint_reaction_work_are_separate() {
        let model = model();
        let state = initial_state(0.8, 0.4);
        let receipt = model
            .step(state, [0.4, -0.2], 0.01, SolverOptions::default())
            .unwrap();
        assert!(receipt.supplied_work.is_finite());
        assert!(receipt.constraint_reaction_work.abs() < 1.0e-11);
        assert!(receipt.closes_energy_balance_within(2.0e-12));
    }

    #[test]
    fn trajectory_converges_at_second_order_against_independent_intrinsic_rk4() {
        let initial = initial_state(1.1, 0.15);
        let reference = rk4_reference(1.1, 0.15, 1.0 / 80_000.0, 80_000);
        let mut errors = Vec::new();
        for dt in [0.04, 0.02, 0.01, 0.005] {
            let result = advance(initial, dt, (1.0 / dt).round() as usize);
            let dx = result.position[0] - reference.position[0];
            let dy = result.position[1] - reference.position[1];
            let dvx = result.velocity[0] - reference.velocity[0];
            let dvy = result.velocity[1] - reference.velocity[1];
            errors.push((dx * dx + dy * dy + dvx * dvx + dvy * dvy).sqrt());
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (3.6..4.4).contains(&ratio),
                "expected second-order convergence: {errors:?}"
            );
        }
    }

    #[test]
    fn rotational_metamorphism_commutes_with_the_pendulum_step() {
        let angle = 0.67_f64;
        let (sin, cos) = angle.sin_cos();
        let rotate = |v: [f64; 2]| [cos * v[0] - sin * v[1], sin * v[0] + cos * v[1]];
        let initial = initial_state(0.9, 0.25);
        let gravity = [0.0, -9.81];
        let normal_model = SemiExplicitPendulumDg::new(1.0, 1.0, gravity).unwrap();
        let rotated_model =
            SemiExplicitPendulumDg::new(1.0, 1.0, rotate(gravity)).unwrap();
        let normal = normal_model
            .step(initial, [0.0, 0.0], 0.01, SolverOptions::default())
            .unwrap();
        let rotated = rotated_model
            .step(
                State {
                    position: rotate(initial.position),
                    velocity: rotate(initial.velocity),
                },
                [0.0, 0.0],
                0.01,
                SolverOptions::default(),
            )
            .unwrap();
        assert!(
            norm(sub(rotated.after.position, rotate(normal.after.position))) < 3.0e-12
        );
        assert!(
            norm(sub(rotated.after.velocity, rotate(normal.after.velocity))) < 3.0e-12
        );
    }

    #[test]
    fn invalid_manifold_and_solver_inputs_fail_closed() {
        let model = model();
        assert_eq!(
            model
                .step(
                    State {
                        position: [0.0, -0.9],
                        velocity: [0.0, 0.0],
                    },
                    [0.0, 0.0],
                    0.01,
                    SolverOptions::default(),
                )
                .unwrap_err(),
            Error::InvalidPositionConstraint
        );
        assert_eq!(
            model
                .step(
                    State {
                        position: [0.0, -1.0],
                        velocity: [0.1, 0.0],
                    },
                    [0.0, 0.0],
                    0.01,
                    SolverOptions::default(),
                )
                .unwrap_err(),
            Error::InvalidInitialVelocityConstraint
        );
        assert_eq!(
            model
                .step(
                    initial_state(0.5, 0.2),
                    [0.0, 0.0],
                    0.0,
                    SolverOptions::default()
                )
                .unwrap_err(),
            Error::InvalidTimeStep
        );
        assert_eq!(
            model
                .step(
                    initial_state(0.5, 0.2),
                    [0.0, 0.0],
                    0.01,
                    SolverOptions {
                        max_iterations: 0,
                        ..SolverOptions::default()
                    }
                )
                .unwrap_err(),
            Error::InvalidSolverOptions
        );
    }
}

#[cfg(not(test))]
fn main() {
    let model = SemiExplicitPendulumDg::new(1.0, 1.0, [0.0, -9.81])
        .expect("frozen reference model must be valid");
    let mut state = State {
        position: [1.1_f64.sin(), -1.1_f64.cos()],
        velocity: [0.15 * 1.1_f64.cos(), 0.15 * 1.1_f64.sin()],
    };
    println!("step,x,y,vx,vy,energy,position_residual,midpoint_velocity_residual,endpoint_velocity_residual");
    for step in 0..500 {
        let receipt = model
            .step(state, [0.0, 0.0], 0.002, SolverOptions::default())
            .expect("frozen reference step must converge");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
            step + 1,
            state.position[0],
            state.position[1],
            state.velocity[0],
            state.velocity[1],
            receipt.hamiltonian_after,
            receipt.relative_position_constraint_residual,
            receipt.relative_midpoint_velocity_constraint_residual,
            receipt.scaled_endpoint_velocity_constraint_residual
        );
    }
}
