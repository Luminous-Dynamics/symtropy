// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Two-mass holonomic dumbbell: a translation- and rotation-equivariant
// semi-explicit pHDAE discrete-gradient reference.

use std::f64;

type Vec2 = [f64; 2];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    InvalidParameter,
    InvalidTimeStep,
    InvalidInitialLinkConstraint,
    InvalidInitialVelocityConstraint,
    NonFiniteInput,
    InvalidSolverOptions,
    SingularMultiplierSolve,
    NoConvergence,
    NonFiniteResult,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub q1: Vec2,
    pub q2: Vec2,
    pub v1: Vec2,
    pub v2: Vec2,
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
pub struct ConstrainedDumbbell {
    mass1: f64,
    mass2: f64,
    length: f64,
    constraint_tolerance: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct StepReceipt {
    pub before: State,
    pub after: State,
    pub midpoint_q1: Vec2,
    pub midpoint_q2: Vec2,
    pub midpoint_v1: Vec2,
    pub midpoint_v2: Vec2,
    pub relative_midpoint_position: Vec2,
    pub relative_midpoint_velocity: Vec2,
    pub multiplier: f64,
    pub multiplier_residual: f64,
    pub scaled_multiplier_residual: f64,
    pub multiplier_iterations: usize,
    pub energy_before: f64,
    pub energy_after: f64,
    pub external_work: f64,
    pub constraint_reaction_work: f64,
    pub energy_balance_residual: f64,
    pub relative_energy_balance_residual: f64,
    pub link_constraint_residual: f64,
    pub relative_link_constraint_residual: f64,
    pub midpoint_velocity_constraint_residual: f64,
    pub relative_midpoint_velocity_constraint_residual: f64,
    /// Endpoint hidden constraint r1 · (v2_1-v1_1), reported not assumed exact.
    pub endpoint_velocity_constraint_residual: f64,
    pub linear_momentum_residual: Vec2,
    pub relative_linear_momentum_residual: f64,
    /// Angular momentum residual after subtracting applied external torque impulse.
    pub angular_momentum_residual: f64,
    pub relative_angular_momentum_residual: f64,
}

impl StepReceipt {
    pub fn closes_energy_balance_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_energy_balance_residual.is_finite()
            && self.relative_energy_balance_residual <= tolerance
    }

    pub fn conserves_linear_momentum_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_linear_momentum_residual.is_finite()
            && self.relative_linear_momentum_residual <= tolerance
    }

    pub fn conserves_angular_momentum_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_angular_momentum_residual.is_finite()
            && self.relative_angular_momentum_residual <= tolerance
    }
}

fn add(a: Vec2, b: Vec2) -> Vec2 {
    [a[0] + b[0], a[1] + b[1]]
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    [a[0] - b[0], a[1] - b[1]]
}

fn scale(a: Vec2, factor: f64) -> Vec2 {
    [a[0] * factor, a[1] * factor]
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn norm(a: Vec2) -> f64 {
    dot(a, a).sqrt()
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn rotate(a: Vec2, angle: f64) -> Vec2 {
    let (sin, cos) = angle.sin_cos();
    [cos * a[0] - sin * a[1], sin * a[0] + cos * a[1]]
}

impl ConstrainedDumbbell {
    pub fn new(mass1: f64, mass2: f64, length: f64) -> Result<Self, Error> {
        if !mass1.is_finite() || !mass2.is_finite() || !length.is_finite() {
            return Err(Error::InvalidParameter);
        }
        if mass1 <= 0.0 || mass2 <= 0.0 || length <= 0.0 {
            return Err(Error::InvalidParameter);
        }
        Ok(Self {
            mass1,
            mass2,
            length,
            constraint_tolerance: 1.0e-10,
        })
    }

    pub fn hamiltonian(&self, state: State) -> Result<f64, Error> {
        if !state.q1.iter().all(|x| x.is_finite())
            || !state.q2.iter().all(|x| x.is_finite())
            || !state.v1.iter().all(|x| x.is_finite())
            || !state.v2.iter().all(|x| x.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        let energy = 0.5 * self.mass1 * dot(state.v1, state.v1)
            + 0.5 * self.mass2 * dot(state.v2, state.v2);
        if energy.is_finite() {
            Ok(energy)
        } else {
            Err(Error::NonFiniteResult)
        }
    }

    pub fn link_constraint(&self, state: State) -> f64 {
        let relative = sub(state.q2, state.q1);
        0.5 * (dot(relative, relative) - self.length * self.length)
    }

    pub fn midpoint_velocity_constraint(&self, state: State) -> f64 {
        let relative_position = sub(state.q2, state.q1);
        let relative_velocity = sub(state.v2, state.v1);
        dot(relative_position, relative_velocity)
    }

    pub fn linear_momentum(&self, state: State) -> Vec2 {
        add(scale(state.v1, self.mass1), scale(state.v2, self.mass2))
    }

    pub fn angular_momentum(&self, state: State) -> f64 {
        self.mass1 * cross(state.q1, state.v1)
            + self.mass2 * cross(state.q2, state.v2)
    }

    fn validate_initial(&self, state: State) -> Result<(), Error> {
        if !state.q1.iter().all(|x| x.is_finite())
            || !state.q2.iter().all(|x| x.is_finite())
            || !state.v1.iter().all(|x| x.is_finite())
            || !state.v2.iter().all(|x| x.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        let relative_position = sub(state.q2, state.q1);
        let relative_velocity = sub(state.v2, state.v1);
        let position_error =
            (dot(relative_position, relative_position) - self.length * self.length).abs();
        let position_scale = (1.0
            + dot(relative_position, relative_position)
            + self.length * self.length)
        .max(f64::MIN_POSITIVE);
        if position_error > self.constraint_tolerance * position_scale {
            return Err(Error::InvalidInitialLinkConstraint);
        }
        let velocity_error = dot(relative_position, relative_velocity).abs();
        let velocity_scale =
            (1.0 + self.length * norm(relative_velocity)).max(f64::MIN_POSITIVE);
        if velocity_error > self.constraint_tolerance * velocity_scale {
            return Err(Error::InvalidInitialVelocityConstraint);
        }
        Ok(())
    }

    /// Evaluate the midpoint relative velocity and scalar discrete constraint.
    ///
    /// Let r0=q2_0-q1_0, w0=v2_0-v1_0, and
    /// a=w0+(h/2)(f2/m2-f1/m1), alpha=(h/2)(1/m1+1/m2).
    /// With vbar_rel = a-alpha*lambda*rbar and rbar=r0+(h/2)vbar_rel:
    ///
    ///   vbar_rel(lambda) = (a-alpha*lambda*r0)/(1+alpha*h*lambda/2)
    ///   F(lambda) = r0·vbar_rel + h/2 |vbar_rel|^2 = 0.
    ///
    /// This is the exact discrete-Jacobian algebraic constraint for
    /// c(q1,q2)=1/2(|q2-q1|^2-L^2).
    fn midpoint_relative_velocity(
        &self,
        relative_position: Vec2,
        base_relative_velocity: Vec2,
        dt: f64,
        lambda: f64,
    ) -> Result<(Vec2, f64, f64), Error> {
        let inverse_mass_sum = 1.0 / self.mass1 + 1.0 / self.mass2;
        let alpha = 0.5 * dt * inverse_mass_sum;
        let numerator = sub(base_relative_velocity, scale(relative_position, alpha * lambda));
        let denominator = 1.0 + 0.5 * dt * alpha * lambda;
        let denominator_scale = (1.0 + (0.5 * dt * alpha * lambda).abs()).max(1.0);
        if !denominator.is_finite()
            || denominator.abs() <= f64::EPSILON * denominator_scale
        {
            return Err(Error::SingularMultiplierSolve);
        }

        let midpoint_velocity = scale(numerator, 1.0 / denominator);
        let numerator_derivative = scale(relative_position, -alpha);
        let denominator_derivative = 0.5 * dt * alpha;
        let velocity_derivative = scale(
            sub(
                scale(numerator_derivative, denominator),
                scale(numerator, denominator_derivative),
            ),
            1.0 / (denominator * denominator),
        );

        let residual = dot(relative_position, midpoint_velocity)
            + 0.5 * dt * dot(midpoint_velocity, midpoint_velocity);
        let derivative = dot(
            add(relative_position, scale(midpoint_velocity, dt)),
            velocity_derivative,
        );
        if !midpoint_velocity.iter().all(|x| x.is_finite())
            || !residual.is_finite()
            || !derivative.is_finite()
        {
            return Err(Error::NonFiniteResult);
        }
        Ok((midpoint_velocity, residual, derivative))
    }

    /// Advance one semi-explicit pHDAE discrete-gradient step.
    ///
    /// External forces are constant over the step and act at the two mass
    /// centers. The link reaction acts as (+lambda*rbar, -lambda*rbar).
    /// The method enforces the discrete midpoint link constraint; it reports
    /// the endpoint hidden velocity constraint separately.
    pub fn step(
        &self,
        before: State,
        external_force1: Vec2,
        external_force2: Vec2,
        dt: f64,
        options: SolverOptions,
    ) -> Result<StepReceipt, Error> {
        let options = options.validate()?;
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        if !external_force1.iter().all(|x| x.is_finite())
            || !external_force2.iter().all(|x| x.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        self.validate_initial(before)?;
        let energy_before = self.hamiltonian(before)?;
        let linear_momentum_before = self.linear_momentum(before);
        let angular_momentum_before = self.angular_momentum(before);

        let relative_position = sub(before.q2, before.q1);
        let relative_velocity = sub(before.v2, before.v1);
        let force_acceleration_difference = sub(
            scale(external_force2, 1.0 / self.mass2),
            scale(external_force1, 1.0 / self.mass1),
        );
        let base_relative_velocity =
            add(relative_velocity, scale(force_acceleration_difference, 0.5 * dt));
        let inverse_mass_sum = 1.0 / self.mass1 + 1.0 / self.mass2;
        let initial_lambda =
            (dot(relative_velocity, relative_velocity)
                + dot(relative_position, force_acceleration_difference))
                / (inverse_mass_sum * dot(relative_position, relative_position));
        if !initial_lambda.is_finite() {
            return Err(Error::NonFiniteResult);
        }

        let mut lambda = initial_lambda;
        let mut iterations = 0usize;
        let mut converged = false;

        for iteration in 0..options.max_iterations {
            let (relative_midpoint_velocity, residual, derivative) =
                self.midpoint_relative_velocity(
                    relative_position,
                    base_relative_velocity,
                    dt,
                    lambda,
                )?;
            let relative_midpoint_position =
                add(relative_position, scale(relative_midpoint_velocity, 0.5 * dt));
            let residual_scale = (1.0
                + norm(relative_midpoint_position) * norm(relative_midpoint_velocity))
            .max(f64::MIN_POSITIVE);
            let scaled_residual = residual.abs() / residual_scale;
            if scaled_residual <= options.relative_tolerance {
                iterations = iteration;
                converged = true;
                break;
            }

            let derivative_scale = (1.0 + lambda.abs()).max(f64::MIN_POSITIVE);
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
                let candidate_lambda = lambda + fraction * delta;
                if candidate_lambda.is_finite() {
                    if let Ok((candidate_velocity, candidate_residual, _)) =
                        self.midpoint_relative_velocity(
                            relative_position,
                            base_relative_velocity,
                            dt,
                            candidate_lambda,
                        )
                    {
                        let candidate_position =
                            add(relative_position, scale(candidate_velocity, 0.5 * dt));
                        let candidate_scale = (1.0
                            + norm(candidate_position) * norm(candidate_velocity))
                        .max(f64::MIN_POSITIVE);
                        if candidate_residual.abs() / candidate_scale < scaled_residual
                            || candidate_residual.abs() / candidate_scale
                                <= options.relative_tolerance
                        {
                            lambda = candidate_lambda;
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
            let (velocity, residual, _) = self.midpoint_relative_velocity(
                relative_position,
                base_relative_velocity,
                dt,
                lambda,
            )?;
            let position = add(relative_position, scale(velocity, 0.5 * dt));
            let residual_scale =
                (1.0 + norm(position) * norm(velocity)).max(f64::MIN_POSITIVE);
            converged = residual.abs() / residual_scale <= options.relative_tolerance;
        }
        if !converged {
            return Err(Error::NoConvergence);
        }

        let (relative_midpoint_velocity, multiplier_residual, _) =
            self.midpoint_relative_velocity(
                relative_position,
                base_relative_velocity,
                dt,
                lambda,
            )?;
        let relative_midpoint_position =
            add(relative_position, scale(relative_midpoint_velocity, 0.5 * dt));

        let midpoint_v1 = add(
            before.v1,
            scale(
                add(external_force1, scale(relative_midpoint_position, lambda)),
                0.5 * dt / self.mass1,
            ),
        );
        let midpoint_v2 = add(
            before.v2,
            scale(
                sub(external_force2, scale(relative_midpoint_position, lambda)),
                0.5 * dt / self.mass2,
            ),
        );
        let midpoint_q1 = add(before.q1, scale(midpoint_v1, 0.5 * dt));
        let midpoint_q2 = add(before.q2, scale(midpoint_v2, 0.5 * dt));

        let after = State {
            q1: add(before.q1, scale(midpoint_v1, dt)),
            q2: add(before.q2, scale(midpoint_v2, dt)),
            v1: sub(scale(midpoint_v1, 2.0), before.v1),
            v2: sub(scale(midpoint_v2, 2.0), before.v2),
        };
        if !after.q1.iter().all(|x| x.is_finite())
            || !after.q2.iter().all(|x| x.is_finite())
            || !after.v1.iter().all(|x| x.is_finite())
            || !after.v2.iter().all(|x| x.is_finite())
        {
            return Err(Error::NonFiniteResult);
        }

        let relative_endpoint_position = sub(after.q2, after.q1);
        let relative_endpoint_velocity = sub(after.v2, after.v1);
        let link_constraint_residual =
            dot(relative_endpoint_position, relative_endpoint_position)
                - self.length * self.length;
        let relative_link_constraint_residual = link_constraint_residual.abs()
            / (1.0
                + dot(relative_endpoint_position, relative_endpoint_position)
                + self.length * self.length);
        let midpoint_velocity_constraint_residual =
            dot(relative_midpoint_position, relative_midpoint_velocity);
        let relative_midpoint_velocity_constraint_residual =
            midpoint_velocity_constraint_residual.abs()
                / (1.0
                    + norm(relative_midpoint_position) * norm(relative_midpoint_velocity));
        let endpoint_velocity_constraint_residual =
            dot(relative_endpoint_position, relative_endpoint_velocity);

        if !relative_link_constraint_residual.is_finite()
            || !relative_midpoint_velocity_constraint_residual.is_finite()
            || !endpoint_velocity_constraint_residual.is_finite()
        {
            return Err(Error::NonFiniteResult);
        }
        if relative_link_constraint_residual > self.constraint_tolerance
            || relative_midpoint_velocity_constraint_residual > self.constraint_tolerance
        {
            return Err(Error::NoConvergence);
        }

        let energy_after = self.hamiltonian(after)?;
        let external_work = dt
            * (dot(external_force1, midpoint_v1) + dot(external_force2, midpoint_v2));
        let constraint_reaction_work =
            -dt * lambda * midpoint_velocity_constraint_residual;
        let energy_balance_residual =
            energy_after - energy_before - external_work - constraint_reaction_work;
        let energy_scale = (energy_before.abs()
            + energy_after.abs()
            + external_work.abs()
            + constraint_reaction_work.abs())
        .max(f64::MIN_POSITIVE);
        let relative_energy_balance_residual =
            energy_balance_residual.abs() / energy_scale;

        let linear_momentum_after = self.linear_momentum(after);
        let expected_linear_momentum = add(
            linear_momentum_before,
            scale(add(external_force1, external_force2), dt),
        );
        let linear_momentum_residual =
            sub(linear_momentum_after, expected_linear_momentum);
        let relative_linear_momentum_residual = norm(linear_momentum_residual)
            / (1.0
                + norm(linear_momentum_before)
                + norm(expected_linear_momentum));

        let angular_momentum_after = self.angular_momentum(after);
        let expected_external_torque = dt
            * (cross(midpoint_q1, external_force1) + cross(midpoint_q2, external_force2));
        let angular_momentum_residual =
            angular_momentum_after - angular_momentum_before - expected_external_torque;
        let relative_angular_momentum_residual = angular_momentum_residual.abs()
            / (1.0
                + angular_momentum_before.abs()
                + angular_momentum_after.abs()
                + expected_external_torque.abs());

        let scaled_multiplier_residual = multiplier_residual.abs()
            / (1.0
                + norm(relative_midpoint_position) * norm(relative_midpoint_velocity));
        if !external_work.is_finite()
            || !constraint_reaction_work.is_finite()
            || !energy_balance_residual.is_finite()
            || !relative_energy_balance_residual.is_finite()
            || !relative_linear_momentum_residual.is_finite()
            || !relative_angular_momentum_residual.is_finite()
            || !scaled_multiplier_residual.is_finite()
        {
            return Err(Error::NonFiniteResult);
        }

        Ok(StepReceipt {
            before,
            after,
            midpoint_q1,
            midpoint_q2,
            midpoint_v1: midpoint_v1,
            midpoint_v2,
            relative_midpoint_position,
            relative_midpoint_velocity,
            multiplier: lambda,
            multiplier_residual,
            scaled_multiplier_residual,
            multiplier_iterations: iterations,
            energy_before,
            energy_after,
            external_work,
            constraint_reaction_work,
            energy_balance_residual,
            relative_energy_balance_residual,
            link_constraint_residual,
            relative_link_constraint_residual,
            midpoint_velocity_constraint_residual,
            relative_midpoint_velocity_constraint_residual,
            endpoint_velocity_constraint_residual,
            linear_momentum_residual,
            relative_linear_momentum_residual,
            angular_momentum_residual,
            relative_angular_momentum_residual,
        })
    }
}

/// Closed-form free dumbbell solution, used as an independent trajectory oracle.
pub fn exact_free_dumbbell(
    initial: State,
    mass1: f64,
    mass2: f64,
    length: f64,
    time: f64,
) -> State {
    let total_mass = mass1 + mass2;
    let center_position = scale(
        add(scale(initial.q1, mass1), scale(initial.q2, mass2)),
        1.0 / total_mass,
    );
    let center_velocity = scale(
        add(scale(initial.v1, mass1), scale(initial.v2, mass2)),
        1.0 / total_mass,
    );
    let relative_position = sub(initial.q2, initial.q1);
    let relative_velocity = sub(initial.v2, initial.v1);
    let angular_velocity = cross(relative_position, relative_velocity) / (length * length);
    let rotated_relative_position = rotate(relative_position, angular_velocity * time);
    let rotated_relative_velocity = rotate(relative_velocity, angular_velocity * time);
    let center_position_at_time = add(center_position, scale(center_velocity, time));

    State {
        q1: sub(
            center_position_at_time,
            scale(rotated_relative_position, mass2 / total_mass),
        ),
        q2: add(
            center_position_at_time,
            scale(rotated_relative_position, mass1 / total_mass),
        ),
        v1: sub(
            center_velocity,
            scale(rotated_relative_velocity, mass2 / total_mass),
        ),
        v2: add(
            center_velocity,
            scale(rotated_relative_velocity, mass1 / total_mass),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> ConstrainedDumbbell {
        ConstrainedDumbbell::new(1.3, 2.1, 1.2).unwrap()
    }

    fn initial_state() -> State {
        State {
            q1: [-0.6, 0.2],
            q2: [0.6, 0.2],
            v1: [0.25, -0.18],
            v2: [0.25, 0.62],
        }
    }

    fn rotate_state(state: State, angle: f64) -> State {
        State {
            q1: rotate(state.q1, angle),
            q2: rotate(state.q2, angle),
            v1: rotate(state.v1, angle),
            v2: rotate(state.v2, angle),
        }
    }

    fn state_error(a: State, b: State) -> f64 {
        (norm(sub(a.q1, b.q1)).powi(2)
            + norm(sub(a.q2, b.q2)).powi(2)
            + norm(sub(a.v1, b.v1)).powi(2)
            + norm(sub(a.v2, b.v2)).powi(2))
        .sqrt()
    }

    fn advance(
        model: &ConstrainedDumbbell,
        mut state: State,
        dt: f64,
        steps: usize,
    ) -> State {
        for _ in 0..steps {
            state = model
                .step(
                    state,
                    [0.0, 0.0],
                    [0.0, 0.0],
                    dt,
                    SolverOptions::default(),
                )
                .unwrap()
                .after;
        }
        state
    }

    #[test]
    fn link_and_midpoint_constraints_hold_over_long_horizon() {
        let model = model();
        let mut state = initial_state();
        let mut max_link_residual = 0.0_f64;
        let mut max_midpoint_residual = 0.0_f64;
        let mut max_endpoint_residual = 0.0_f64;
        for _ in 0..5000 {
            let receipt = model
                .step(
                    state,
                    [0.0, 0.0],
                    [0.0, 0.0],
                    0.002,
                    SolverOptions::default(),
                )
                .unwrap();
            max_link_residual =
                max_link_residual.max(receipt.relative_link_constraint_residual);
            max_midpoint_residual = max_midpoint_residual
                .max(receipt.relative_midpoint_velocity_constraint_residual);
            max_endpoint_residual =
                max_endpoint_residual.max(receipt.endpoint_velocity_constraint_residual.abs());
            state = receipt.after;
        }
        assert!(max_link_residual < 1.0e-12);
        assert!(max_midpoint_residual < 1.0e-12);
        assert!(max_endpoint_residual < 2.0e-12);
    }

    #[test]
    fn free_dumbbell_preserves_energy_linear_and_angular_momentum() {
        let model = model();
        let mut state = initial_state();
        let energy0 = model.hamiltonian(state).unwrap();
        let momentum0 = model.linear_momentum(state);
        let angular0 = model.angular_momentum(state);
        let mut max_energy_residual = 0.0_f64;
        let mut max_linear_residual = 0.0_f64;
        let mut max_angular_residual = 0.0_f64;
        for _ in 0..5000 {
            let receipt = model
                .step(
                    state,
                    [0.0, 0.0],
                    [0.0, 0.0],
                    0.002,
                    SolverOptions::default(),
                )
                .unwrap();
            max_energy_residual =
                max_energy_residual.max(receipt.relative_energy_balance_residual);
            max_linear_residual =
                max_linear_residual.max(receipt.relative_linear_momentum_residual);
            max_angular_residual =
                max_angular_residual.max(receipt.relative_angular_momentum_residual);
            state = receipt.after;
        }
        let energy_drift =
            (model.hamiltonian(state).unwrap() - energy0).abs() / energy0.abs();
        let momentum_drift =
            norm(sub(model.linear_momentum(state), momentum0)) / (1.0 + norm(momentum0));
        let angular_drift =
            (model.angular_momentum(state) - angular0).abs() / (1.0 + angular0.abs());
        assert!(max_energy_residual < 2.0e-12);
        assert!(energy_drift < 2.0e-11);
        assert!(max_linear_residual < 2.0e-12);
        assert!(momentum_drift < 2.0e-12);
        assert!(max_angular_residual < 2.0e-12);
        assert!(angular_drift < 2.0e-12);
    }

    #[test]
    fn rotational_equivariance_commutes_with_the_discrete_update() {
        let model = model();
        let initial = initial_state();
        let angle = 0.731_f64;
        let force1 = [0.12, -0.07];
        let force2 = [-0.03, 0.09];
        let normal = model
            .step(
                initial,
                force1,
                force2,
                0.01,
                SolverOptions::default(),
            )
            .unwrap();
        let rotated = model
            .step(
                rotate_state(initial, angle),
                rotate(force1, angle),
                rotate(force2, angle),
                0.01,
                SolverOptions::default(),
            )
            .unwrap();
        assert!(state_error(rotated.after, rotate_state(normal.after, angle)) < 3.0e-12);
    }

    #[test]
    fn exact_free_rigid_rotation_gives_second_order_convergence() {
        let model = model();
        let initial = initial_state();
        let reference = exact_free_dumbbell(initial, 1.3, 2.1, 1.2, 1.0);
        let steps = [(0.04, 25usize), (0.02, 50), (0.01, 100), (0.005, 200)];
        let mut errors = Vec::new();
        for (dt, count) in steps {
            let result = advance(&model, initial, dt, count);
            errors.push(state_error(result, reference));
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (3.6..4.4).contains(&ratio),
                "expected second-order convergence against exact free rigid rotation: {errors:?}"
            );
        }
    }

    #[test]
    fn zero_relative_angular_velocity_control_advects_without_link_reaction() {
        let model = model();
        let mut state = State {
            q1: [-0.6, 0.2],
            q2: [0.6, 0.2],
            v1: [0.3, -0.2],
            v2: [0.3, -0.2],
        };
        let initial_energy = model.hamiltonian(state).unwrap();
        let initial_angular_momentum = model.angular_momentum(state);
        for _ in 0..1000 {
            let receipt = model
                .step(
                    state,
                    [0.0, 0.0],
                    [0.0, 0.0],
                    0.001,
                    SolverOptions::default(),
                )
                .unwrap();
            assert!(receipt.multiplier.abs() < 2.0e-12);
            state = receipt.after;
        }
        assert!((model.hamiltonian(state).unwrap() - initial_energy).abs() < 2.0e-12);
        assert!(
            (model.angular_momentum(state) - initial_angular_momentum).abs() < 2.0e-12
        );
        assert!(
            norm(sub(state.v1, state.v2)) < 2.0e-12,
            "common translational velocity should remain common"
        );
    }

    #[test]
    fn equal_mass_case_preserves_the_same_momentum_maps() {
        let model = ConstrainedDumbbell::new(1.0, 1.0, 1.2).unwrap();
        let mut state = initial_state();
        let momentum0 = model.linear_momentum(state);
        let angular0 = model.angular_momentum(state);
        for _ in 0..2000 {
            state = model
                .step(
                    state,
                    [0.0, 0.0],
                    [0.0, 0.0],
                    0.002,
                    SolverOptions::default(),
                )
                .unwrap()
                .after;
        }
        assert!(norm(sub(model.linear_momentum(state), momentum0)) < 2.0e-12);
        assert!((model.angular_momentum(state) - angular0).abs() < 2.0e-12);
    }

    #[test]
    fn galilean_boost_commutes_with_the_discrete_step() {
        let model = model();
        let initial = initial_state();
        let boost = [0.7, -0.4];
        let normal = model
            .step(
                initial,
                [0.0, 0.0],
                [0.0, 0.0],
                0.01,
                SolverOptions::default(),
            )
            .unwrap();
        let boosted = model
            .step(
                State {
                    q1: initial.q1,
                    q2: initial.q2,
                    v1: add(initial.v1, boost),
                    v2: add(initial.v2, boost),
                },
                [0.0, 0.0],
                [0.0, 0.0],
                0.01,
                SolverOptions::default(),
            )
            .unwrap();
        assert!(
            norm(sub(boosted.after.q1, add(normal.after.q1, scale(boost, 0.01)))) < 3.0e-12
        );
        assert!(
            norm(sub(boosted.after.q2, add(normal.after.q2, scale(boost, 0.01)))) < 3.0e-12
        );
        assert!(norm(sub(boosted.after.v1, add(normal.after.v1, boost))) < 3.0e-12);
        assert!(norm(sub(boosted.after.v2, add(normal.after.v2, boost))) < 3.0e-12);
    }

    #[test]
    fn external_force_and_torque_changes_match_signed_impulse_ledger() {
        let model = model();
        let initial = initial_state();
        let force1 = [0.2, -0.1];
        let force2 = [-0.05, 0.3];
        let receipt = model
            .step(initial, force1, force2, 0.01, SolverOptions::default())
            .unwrap();
        assert!(receipt.closes_energy_balance_within(2.0e-12));
        assert!(receipt.conserves_linear_momentum_within(2.0e-12));
        assert!(receipt.conserves_angular_momentum_within(2.0e-12));
        assert!(receipt.constraint_reaction_work.abs() < 1.0e-12);
    }

    #[test]
    fn negative_controls_reject_invalid_manifold_and_solver_inputs() {
        let model = model();
        let mut off_length = initial_state();
        off_length.q2[0] += 0.1;
        assert_eq!(
            model
                .step(
                    off_length,
                    [0.0, 0.0],
                    [0.0, 0.0],
                    0.01,
                    SolverOptions::default()
                )
                .unwrap_err(),
            Error::InvalidInitialLinkConstraint
        );

        let mut off_tangent = initial_state();
        off_tangent.v2 = add(off_tangent.v2, [0.1, 0.0]);
        assert_eq!(
            model
                .step(
                    off_tangent,
                    [0.0, 0.0],
                    [0.0, 0.0],
                    0.01,
                    SolverOptions::default()
                )
                .unwrap_err(),
            Error::InvalidInitialVelocityConstraint
        );

        assert_eq!(
            ConstrainedDumbbell::new(0.0, 1.0, 1.0).unwrap_err(),
            Error::InvalidParameter
        );
        assert_eq!(
            model
                .step(
                    initial_state(),
                    [0.0, 0.0],
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
                    initial_state(),
                    [0.0, 0.0],
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
        assert_eq!(
            model
                .step(
                    State {
                        q1: [f64::NAN, 0.0],
                        ..initial_state()
                    },
                    [0.0, 0.0],
                    [0.0, 0.0],
                    0.01,
                    SolverOptions::default()
                )
                .unwrap_err(),
            Error::NonFiniteInput
        );
        assert_eq!(
            model
                .step(
                    initial_state(),
                    [f64::INFINITY, 0.0],
                    [0.0, 0.0],
                    0.01,
                    SolverOptions::default()
                )
                .unwrap_err(),
            Error::NonFiniteInput
        );
        assert_eq!(
            model
                .step(
                    initial_state(),
                    [0.0, 0.0],
                    [0.0, 0.0],
                    0.01,
                    SolverOptions {
                        relative_tolerance: 1.0e-30,
                        max_iterations: 1,
                        max_backtracks: 1,
                    }
                )
                .unwrap_err(),
            Error::NoConvergence
        );
    }
}

#[cfg(not(test))]
fn main() {
    let model = ConstrainedDumbbell::new(1.3, 2.1, 1.2)
        .expect("frozen dumbbell model must be valid");
    let mut state = State {
        q1: [-0.6, 0.2],
        q2: [0.6, 0.2],
        v1: [0.25, -0.18],
        v2: [0.25, 0.62],
    };
    println!("step,energy,link_residual,midpoint_constraint,endpoint_tangent,linear_momentum_residual,angular_momentum_residual,multiplier_iterations");
    for step in 0..500 {
        let receipt = model
            .step(
                state,
                [0.0, 0.0],
                [0.0, 0.0],
                0.002,
                SolverOptions::default(),
            )
            .expect("frozen dumbbell reference step must converge");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{}",
            step + 1,
            receipt.energy_after,
            receipt.relative_link_constraint_residual,
            receipt.relative_midpoint_velocity_constraint_residual,
            receipt.endpoint_velocity_constraint_residual.abs(),
            receipt.relative_linear_momentum_residual,
            receipt.relative_angular_momentum_residual,
            receipt.multiplier_iterations
        );
    }
}
