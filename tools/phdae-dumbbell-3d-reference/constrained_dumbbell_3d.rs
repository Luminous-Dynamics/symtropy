// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Three-dimensional semi-explicit pHDAE discrete-gradient constrained
// dumbbell, with SO(3) momentum-map and Rodrigues-oracle qualification.

use std::f64;

pub type Vec3 = [f64; 3];

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
    pub q1: Vec3,
    pub q2: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
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
pub struct ConstrainedDumbbell3D {
    mass1: f64,
    mass2: f64,
    length: f64,
    constraint_tolerance: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct StepReceipt {
    pub before: State,
    pub after: State,
    pub midpoint_q1: Vec3,
    pub midpoint_q2: Vec3,
    pub midpoint_v1: Vec3,
    pub midpoint_v2: Vec3,
    pub relative_midpoint_position: Vec3,
    pub relative_midpoint_velocity: Vec3,
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
    /// Endpoint hidden constraint r1 · (v2_1-v1_1); reported, not assumed.
    pub endpoint_velocity_constraint_residual: f64,
    pub linear_momentum_residual: Vec3,
    pub relative_linear_momentum_residual: f64,
    /// Full 3D angular momentum vector residual after applied torque impulse.
    pub angular_momentum_residual: Vec3,
    pub relative_angular_momentum_residual: f64,
}

impl StepReceipt {
    pub fn closes_energy_balance_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_energy_balance_residual.is_finite()
            && self.relative_energy_balance_residual <= tolerance
    }

    pub fn preserves_linear_momentum_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_linear_momentum_residual.is_finite()
            && self.relative_linear_momentum_residual <= tolerance
    }

    pub fn preserves_angular_momentum_within(&self, tolerance: f64) -> bool {
        tolerance.is_finite()
            && tolerance >= 0.0
            && self.relative_angular_momentum_residual.is_finite()
            && self.relative_angular_momentum_residual <= tolerance
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

/// Rodrigues rotation by a rotation vector whose norm is the angle.
/// A second-order small-angle branch avoids axis normalization near zero.
pub fn rodrigues(rotation_vector: Vec3, value: Vec3) -> Vec3 {
    let angle = norm(rotation_vector);
    if angle < 1.0e-8 {
        let first = cross(rotation_vector, value);
        let second = cross(rotation_vector, first);
        return add(add(value, first), scale(second, 0.5));
    }
    let axis = scale(rotation_vector, 1.0 / angle);
    let (sin, cos) = angle.sin_cos();
    add(
        add(scale(value, cos), scale(cross(axis, value), sin)),
        scale(axis, dot(axis, value) * (1.0 - cos)),
    )
}

impl ConstrainedDumbbell3D {
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

    pub fn linear_momentum(&self, state: State) -> Vec3 {
        add(scale(state.v1, self.mass1), scale(state.v2, self.mass2))
    }

    pub fn angular_momentum(&self, state: State) -> Vec3 {
        add(
            scale(cross(state.q1, state.v1), self.mass1),
            scale(cross(state.q2, state.v2), self.mass2),
        )
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
        let tangent_error = dot(relative_position, relative_velocity).abs();
        let tangent_scale =
            (1.0 + self.length * norm(relative_velocity)).max(f64::MIN_POSITIVE);
        if tangent_error > self.constraint_tolerance * tangent_scale {
            return Err(Error::InvalidInitialVelocityConstraint);
        }
        Ok(())
    }

    fn midpoint_relative_velocity(
        &self,
        relative_position: Vec3,
        base_relative_velocity: Vec3,
        dt: f64,
        lambda: f64,
    ) -> Result<(Vec3, f64, f64), Error> {
        let inverse_mass_sum = 1.0 / self.mass1 + 1.0 / self.mass2;
        let alpha = 0.5 * dt * inverse_mass_sum;
        let numerator = sub(
            base_relative_velocity,
            scale(relative_position, alpha * lambda),
        );
        let denominator = 1.0 + 0.5 * dt * alpha * lambda;
        let denominator_scale =
            (1.0 + (0.5 * dt * alpha * lambda).abs()).max(1.0);
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

    /// Advance one 3D semi-explicit pHDAE discrete-gradient step.
    ///
    /// The midpoint relative link vector is the exact discrete Jacobian of
    /// c(q1,q2)=1/2(|q2-q1|^2-L^2). The scalar multiplier enforces the
    /// midpoint algebraic constraint. The same identity preserves the link
    /// length and makes the reaction do zero work and zero net torque vector.
    pub fn step(
        &self,
        before: State,
        external_force1: Vec3,
        external_force2: Vec3,
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
        let relative_position_squared = dot(relative_position, relative_position);
        let initial_multiplier =
            (dot(relative_velocity, relative_velocity)
                + dot(relative_position, force_acceleration_difference))
                / (inverse_mass_sum * relative_position_squared);
        if !initial_multiplier.is_finite() {
            return Err(Error::NonFiniteResult);
        }

        let mut multiplier = initial_multiplier;
        let mut iterations = 0usize;
        let mut converged = false;
        for iteration in 0..options.max_iterations {
            let (midpoint_velocity, residual, derivative) =
                self.midpoint_relative_velocity(
                    relative_position,
                    base_relative_velocity,
                    dt,
                    multiplier,
                )?;
            let midpoint_position =
                add(relative_position, scale(midpoint_velocity, 0.5 * dt));
            let residual_scale = (1.0
                + norm(midpoint_position) * norm(midpoint_velocity))
            .max(f64::MIN_POSITIVE);
            let scaled_residual = residual.abs() / residual_scale;
            if scaled_residual <= options.relative_tolerance {
                iterations = iteration;
                converged = true;
                break;
            }

            let derivative_scale = (1.0 + multiplier.abs()).max(f64::MIN_POSITIVE);
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
                        self.midpoint_relative_velocity(
                            relative_position,
                            base_relative_velocity,
                            dt,
                            candidate_multiplier,
                        )
                    {
                        let candidate_position =
                            add(relative_position, scale(candidate_velocity, 0.5 * dt));
                        let candidate_scale = (1.0
                            + norm(candidate_position) * norm(candidate_velocity))
                        .max(f64::MIN_POSITIVE);
                        let candidate_norm = candidate_residual.abs() / candidate_scale;
                        if candidate_norm < scaled_residual
                            || candidate_norm <= options.relative_tolerance
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
            let (midpoint_velocity, residual, _) = self.midpoint_relative_velocity(
                relative_position,
                base_relative_velocity,
                dt,
                multiplier,
            )?;
            let midpoint_position =
                add(relative_position, scale(midpoint_velocity, 0.5 * dt));
            let residual_scale = (1.0
                + norm(midpoint_position) * norm(midpoint_velocity))
            .max(f64::MIN_POSITIVE);
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
                multiplier,
            )?;
        let relative_midpoint_position =
            add(relative_position, scale(relative_midpoint_velocity, 0.5 * dt));

        let midpoint_v1 = add(
            before.v1,
            scale(
                add(external_force1, scale(relative_midpoint_position, multiplier)),
                0.5 * dt / self.mass1,
            ),
        );
        let midpoint_v2 = add(
            before.v2,
            scale(
                sub(external_force2, scale(relative_midpoint_position, multiplier)),
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

        let endpoint_relative_position = sub(after.q2, after.q1);
        let endpoint_relative_velocity = sub(after.v2, after.v1);
        let link_constraint_residual =
            dot(endpoint_relative_position, endpoint_relative_position)
                - self.length * self.length;
        let relative_link_constraint_residual = link_constraint_residual.abs()
            / (1.0
                + dot(endpoint_relative_position, endpoint_relative_position)
                + self.length * self.length);
        let midpoint_velocity_constraint_residual =
            dot(relative_midpoint_position, relative_midpoint_velocity);
        let relative_midpoint_velocity_constraint_residual =
            midpoint_velocity_constraint_residual.abs()
                / (1.0
                    + norm(relative_midpoint_position) * norm(relative_midpoint_velocity));
        let endpoint_velocity_constraint_residual =
            dot(endpoint_relative_position, endpoint_relative_velocity);

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
            -dt * multiplier * midpoint_velocity_constraint_residual;
        let energy_balance_residual =
            energy_after - energy_before - external_work - constraint_reaction_work;
        let energy_scale = (1.0
            + energy_before.abs()
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
        let expected_external_torque_impulse = scale(
            add(
                cross(midpoint_q1, external_force1),
                cross(midpoint_q2, external_force2),
            ),
            dt,
        );
        let angular_momentum_residual = sub(
            sub(angular_momentum_after, angular_momentum_before),
            expected_external_torque_impulse,
        );
        let relative_angular_momentum_residual = norm(angular_momentum_residual)
            / (1.0
                + norm(angular_momentum_before)
                + norm(angular_momentum_after)
                + norm(expected_external_torque_impulse));

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
            midpoint_v1,
            midpoint_v2,
            relative_midpoint_position,
            relative_midpoint_velocity,
            multiplier,
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

/// Exact free dumbbell trajectory: uniform center-of-mass translation plus
/// Rodrigues rotation of the relative position and velocity vectors.
pub fn exact_free_dumbbell_3d(
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
    let angular_velocity = scale(
        cross(relative_position, relative_velocity),
        1.0 / (length * length),
    );
    let rotation_vector = scale(angular_velocity, time);
    let rotated_relative_position = rodrigues(rotation_vector, relative_position);
    let rotated_relative_velocity = rodrigues(rotation_vector, relative_velocity);
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

    const MASS_1: f64 = 1.3;
    const MASS_2: f64 = 2.1;
    const LENGTH: f64 = 1.2;

    fn model() -> ConstrainedDumbbell3D {
        ConstrainedDumbbell3D::new(MASS_1, MASS_2, LENGTH).unwrap()
    }

    fn initial_state() -> State {
        let center = [0.25, -0.12, 0.30];
        let center_velocity = [0.17, -0.23, 0.11];
        let relative_position = [0.8, 0.8, 0.4];
        let relative_velocity = [0.2, -0.1, -0.2];
        let total_mass = MASS_1 + MASS_2;
        State {
            q1: sub(
                center,
                scale(relative_position, MASS_2 / total_mass),
            ),
            q2: add(
                center,
                scale(relative_position, MASS_1 / total_mass),
            ),
            v1: sub(
                center_velocity,
                scale(relative_velocity, MASS_2 / total_mass),
            ),
            v2: add(
                center_velocity,
                scale(relative_velocity, MASS_1 / total_mass),
            ),
        }
    }

    fn state_error(a: State, b: State) -> f64 {
        (dot(sub(a.q1, b.q1), sub(a.q1, b.q1))
            + dot(sub(a.q2, b.q2), sub(a.q2, b.q2))
            + dot(sub(a.v1, b.v1), sub(a.v1, b.v1))
            + dot(sub(a.v2, b.v2), sub(a.v2, b.v2)))
        .sqrt()
    }

    fn rotate_state(state: State, rotation_vector: Vec3) -> State {
        State {
            q1: rodrigues(rotation_vector, state.q1),
            q2: rodrigues(rotation_vector, state.q2),
            v1: rodrigues(rotation_vector, state.v1),
            v2: rodrigues(rotation_vector, state.v2),
        }
    }

    fn advance(mut state: State, dt: f64, steps: usize) -> State {
        let model = model();
        for _ in 0..steps {
            state = model
                .step(
                    state,
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    dt,
                    SolverOptions::default(),
                )
                .unwrap()
                .after;
        }
        state
    }

    #[test]
    fn initial_fixture_is_genuinely_three_dimensional() {
        let state = initial_state();
        let r = sub(state.q2, state.q1);
        let w = sub(state.v2, state.v1);
        assert!(r.iter().all(|x| x.abs() > 0.0));
        assert!(w.iter().all(|x| x.abs() > 0.0));
        assert!(dot(r, w).abs() < 1.0e-14);
        let omega = cross(r, w);
        assert!(omega.iter().all(|x| x.abs() > 0.0));
    }

    #[test]
    fn position_midpoint_and_endpoint_constraints_hold_over_long_horizon() {
        let model = model();
        let mut state = initial_state();
        let mut max_link = 0.0_f64;
        let mut max_midpoint = 0.0_f64;
        let mut max_endpoint = 0.0_f64;
        for _ in 0..5000 {
            let receipt = model
                .step(
                    state,
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    0.002,
                    SolverOptions::default(),
                )
                .unwrap();
            max_link = max_link.max(receipt.relative_link_constraint_residual);
            max_midpoint = max_midpoint
                .max(receipt.relative_midpoint_velocity_constraint_residual);
            max_endpoint =
                max_endpoint.max(receipt.endpoint_velocity_constraint_residual.abs());
            state = receipt.after;
        }
        assert!(max_link < 2.0e-12);
        assert!(max_midpoint < 2.0e-12);
        assert!(max_endpoint < 3.0e-12);
    }

    #[test]
    fn energy_linear_momentum_and_full_angular_momentum_vector_are_preserved() {
        let model = model();
        let mut state = initial_state();
        let energy0 = model.hamiltonian(state).unwrap();
        let linear0 = model.linear_momentum(state);
        let angular0 = model.angular_momentum(state);
        let mut max_energy_balance = 0.0_f64;
        let mut max_linear = 0.0_f64;
        let mut max_angular = 0.0_f64;
        for _ in 0..5000 {
            let receipt = model
                .step(
                    state,
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    0.002,
                    SolverOptions::default(),
                )
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
        assert!(max_energy_balance < 3.0e-12);
        assert!(energy_drift < 3.0e-11);
        assert!(max_linear < 3.0e-12);
        assert!(linear_drift < 3.0e-12);
        assert!(max_angular < 3.0e-12);
        assert!(angular_drift < 3.0e-12);
    }

    #[test]
    fn exact_rodrigues_solution_gives_second_order_convergence() {
        let model = model();
        let initial = initial_state();
        let reference = exact_free_dumbbell_3d(initial, MASS_1, MASS_2, LENGTH, 1.0);
        let mut errors = Vec::new();
        for dt in [0.04, 0.02, 0.01, 0.005] {
            let result = advance(initial, dt, (1.0 / dt).round() as usize);
            errors.push(state_error(result, reference));
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (3.6..4.4).contains(&ratio),
                "expected second-order convergence against Rodrigues solution: {errors:?}"
            );
        }
    }

    #[test]
    fn twenty_generic_so3_rotations_commute_with_the_discrete_update() {
        let model = model();
        let initial = initial_state();
        let force1 = [0.12, -0.07, 0.05];
        let force2 = [-0.03, 0.09, 0.11];
        let normal = model
            .step(
                initial,
                force1,
                force2,
                0.01,
                SolverOptions::default(),
            )
            .unwrap();
        for i in 0..20 {
            let raw_axis = [
                (0.37 * (i as f64 + 1.0)).sin(),
                (0.23 * (i as f64 + 2.0)).cos(),
                (0.41 * (i as f64 + 3.0)).sin(),
            ];
            let axis = scale(raw_axis, 1.0 / norm(raw_axis));
            let rotation_vector = scale(axis, 0.137 * (i as f64 + 1.0));
            let rotated = model
                .step(
                    rotate_state(initial, rotation_vector),
                    rodrigues(rotation_vector, force1),
                    rodrigues(rotation_vector, force2),
                    0.01,
                    SolverOptions::default(),
                )
                .unwrap();
            let expected = rotate_state(normal.after, rotation_vector);
            assert!(
                state_error(rotated.after, expected) < 8.0e-12,
                "SO(3) metamorphic case {i} failed"
            );
        }
    }

    #[test]
    fn galilean_boost_covariance_holds_in_three_dimensions() {
        let model = model();
        let initial = initial_state();
        let boost = [0.7, -0.4, 0.23];
        let normal = model
            .step(
                initial,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                0.01,
                SolverOptions::default(),
            )
            .unwrap();
        let boosted_initial = State {
            q1: initial.q1,
            q2: initial.q2,
            v1: add(initial.v1, boost),
            v2: add(initial.v2, boost),
        };
        let boosted = model
            .step(
                boosted_initial,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                0.01,
                SolverOptions::default(),
            )
            .unwrap();
        let expected = State {
            q1: add(normal.after.q1, scale(boost, 0.01)),
            q2: add(normal.after.q2, scale(boost, 0.01)),
            v1: add(normal.after.v1, boost),
            v2: add(normal.after.v2, boost),
        };
        assert!(state_error(boosted.after, expected) < 5.0e-12);
    }

    #[test]
    fn external_force_work_and_vector_impulse_torque_ledgers_close() {
        let model = model();
        let initial = initial_state();
        let force1 = [0.12, -0.07, 0.05];
        let force2 = [-0.03, 0.09, 0.11];
        let receipt = model
            .step(initial, force1, force2, 0.01, SolverOptions::default())
            .unwrap();
        assert!(receipt.closes_energy_balance_within(3.0e-12));
        assert!(receipt.preserves_linear_momentum_within(3.0e-12));
        assert!(receipt.preserves_angular_momentum_within(3.0e-12));
        assert!(norm(receipt.angular_momentum_residual) < 3.0e-12);
        assert!(receipt.constraint_reaction_work.abs() < 3.0e-12);
    }

    #[test]
    fn small_angle_rodrigues_and_zero_spin_limit_are_stable() {
        let vector = [0.3, -0.7, 0.2];
        let rotation_vector = [1.0e-12, -2.0e-12, 3.0e-12];
        let rotated = rodrigues(rotation_vector, vector);
        assert!((norm(rotated) - norm(vector)).abs() < 2.0e-15);

        let state = initial_state();
        let center_velocity = [0.1, -0.2, 0.3];
        let pure_translation = State {
            q1: state.q1,
            q2: state.q2,
            v1: center_velocity,
            v2: center_velocity,
        };
        let result = advance(pure_translation, 0.01, 100);
        assert!(
            norm(sub(result.q1, add(pure_translation.q1, center_velocity))) < 3.0e-12
        );
        assert!(
            norm(sub(result.q2, add(pure_translation.q2, center_velocity))) < 3.0e-12
        );
        assert!(norm(sub(result.v1, center_velocity)) < 3.0e-12);
        assert!(norm(sub(result.v2, center_velocity)) < 3.0e-12);
    }

    #[test]
    fn invalid_manifold_nonfinite_and_solver_inputs_fail_closed() {
        let model = model();
        let mut off_length = initial_state();
        off_length.q2[0] += 0.1;
        assert_eq!(
            model
                .step(
                    off_length,
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    0.01,
                    SolverOptions::default()
                )
                .unwrap_err(),
            Error::InvalidInitialLinkConstraint
        );

        let mut off_tangent = initial_state();
        off_tangent.v2 = add(off_tangent.v2, [0.1, 0.0, 0.0]);
        assert_eq!(
            model
                .step(
                    off_tangent,
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
                    0.01,
                    SolverOptions::default()
                )
                .unwrap_err(),
            Error::InvalidInitialVelocityConstraint
        );
        assert_eq!(
            model
                .step(
                    State {
                        q1: [f64::NAN, 0.0, 0.0],
                        ..initial_state()
                    },
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
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
                    [f64::INFINITY, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
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
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
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
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
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
            ConstrainedDumbbell3D::new(0.0, 1.0, 1.0).unwrap_err(),
            Error::InvalidParameter
        );
        assert_eq!(
            model
                .step(
                    initial_state(),
                    [0.0, 0.0, 0.0],
                    [0.0, 0.0, 0.0],
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
    let model = ConstrainedDumbbell3D::new(1.3, 2.1, 1.2)
        .expect("frozen 3D model must be valid");
    let mut state = State {
        q1: [-0.24411764705882355, -0.6141176470588235, 0.0529411764705882],
        q2: [0.5558823529411765, 0.1858823529411765, 0.4529411764705882],
        v1: [0.0464705882352941, -0.1682352941176471, 0.2335294117647060],
        v2: [0.2464705882352941, -0.2682352941176471, 0.0335294117647059],
    };
    println!("step,energy,link_residual,midpoint_constraint,endpoint_tangent,linear_momentum_residual,angular_momentum_x,angular_momentum_y,angular_momentum_z,multiplier_iterations");
    for step in 0..500 {
        let receipt = model
            .step(
                state,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                0.002,
                SolverOptions::default(),
            )
            .expect("frozen 3D reference step must converge");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{}",
            step + 1,
            receipt.energy_after,
            receipt.relative_link_constraint_residual,
            receipt.relative_midpoint_velocity_constraint_residual,
            receipt.endpoint_velocity_constraint_residual.abs(),
            receipt.relative_linear_momentum_residual,
            receipt.angular_momentum_residual[0],
            receipt.angular_momentum_residual[1],
            receipt.angular_momentum_residual[2],
            receipt.multiplier_iterations
        );
    }
}
