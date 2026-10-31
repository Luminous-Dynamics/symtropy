// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Standalone Cartesian RATTLE reference for a planar pendulum.

use std::f64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    InvalidParameter,
    InvalidTimeStep,
    InvalidInitialPositionConstraint,
    InvalidInitialTangentConstraint,
    NonFiniteInput,
    SingularPositionConstraintSolve,
    PositionConstraintNoConvergence,
    NonFiniteResult,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub position: [f64; 2],
    pub momentum: [f64; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct CartesianPendulum {
    mass: f64,
    length: f64,
    gravity: [f64; 2],
    constraint_tolerance: f64,
    newton_tolerance: f64,
    max_newton_iterations: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct StepReceipt {
    pub before: State,
    pub after: State,
    pub energy_before: f64,
    pub energy_after: f64,
    pub relative_energy_change: f64,
    pub position_constraint_residual: f64,
    pub relative_position_constraint_residual: f64,
    pub tangent_constraint_residual: f64,
    pub relative_tangent_constraint_residual: f64,
    pub position_multiplier: f64,
    pub momentum_multiplier: f64,
    pub position_newton_iterations: usize,
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

fn scale(a: [f64; 2], scalar: f64) -> [f64; 2] {
    [a[0] * scalar, a[1] * scalar]
}

impl CartesianPendulum {
    pub fn new(mass: f64, length: f64, gravity: [f64; 2]) -> Result<Self, Error> {
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
            newton_tolerance: 1.0e-14,
            max_newton_iterations: 32,
        })
    }

    /// Mechanical energy shifted by the constant m*|g|*L so the hanging
    /// equilibrium has zero potential energy.
    pub fn hamiltonian(&self, state: State) -> Result<f64, Error> {
        if !state.position.iter().all(|value| value.is_finite())
            || !state.momentum.iter().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        let kinetic = dot(state.momentum, state.momentum) / (2.0 * self.mass);
        let potential =
            -self.mass * dot(self.gravity, state.position) + self.mass * norm(self.gravity) * self.length;
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

    pub fn tangent_constraint(&self, state: State) -> f64 {
        dot(state.position, state.momentum)
    }

    fn validate_initial_state(&self, state: State) -> Result<(), Error> {
        if !state.position.iter().all(|value| value.is_finite())
            || !state.momentum.iter().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        let position_error = (dot(state.position, state.position)
            - self.length * self.length)
            .abs();
        let position_scale = (1.0
            + self.length * self.length
            + dot(state.position, state.position))
        .max(f64::MIN_POSITIVE);
        if position_error > self.constraint_tolerance * position_scale {
            return Err(Error::InvalidInitialPositionConstraint);
        }
        let tangent_error = dot(state.position, state.momentum).abs();
        let tangent_scale =
            (self.length * norm(state.momentum)).max(f64::MIN_POSITIVE);
        if tangent_error > self.constraint_tolerance * tangent_scale {
            return Err(Error::InvalidInitialTangentConstraint);
        }
        Ok(())
    }

    /// One Cartesian RATTLE step for a frictionless, torque-free pendulum.
    ///
    /// First solve the position multiplier so |q1|=L. Then solve the
    /// momentum multiplier so q1·p1=0. This is a symplectic constrained
    /// baseline; it does not exactly conserve the nonlinear Hamiltonian.
    pub fn step(&self, before: State, dt: f64) -> Result<StepReceipt, Error> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        self.validate_initial_state(before)?;
        let energy_before = self.hamiltonian(before)?;

        let half_force = scale(self.gravity, 0.5 * dt * self.mass);
        let unconstrained_half_momentum = add(before.momentum, half_force);
        let predictor = add(
            before.position,
            scale(unconstrained_half_momentum, dt / self.mass),
        );
        let multiplier_scale = dt * dt / (2.0 * self.mass);

        // q1 = predictor - alpha*q0, with alpha = dt^2*lambda/(2m).
        // Solve the circle equation by safeguarded scalar Newton iteration.
        let mut alpha = 0.0_f64;
        let mut position_iterations = 0usize;
        let mut position_converged = false;
        for iteration in 0..self.max_newton_iterations {
            let candidate = sub(predictor, scale(before.position, alpha));
            let residual = dot(candidate, candidate) - self.length * self.length;
            let residual_scale = (1.0
                + dot(candidate, candidate)
                + self.length * self.length)
                .max(f64::MIN_POSITIVE);
            if residual.abs() <= self.newton_tolerance * residual_scale {
                position_converged = true;
                position_iterations = iteration;
                break;
            }
            let derivative = -2.0 * dot(before.position, candidate);
            let derivative_scale =
                (2.0 * self.length * norm(candidate)).max(f64::MIN_POSITIVE);
            if !derivative.is_finite()
                || derivative.abs() <= f64::EPSILON * derivative_scale
            {
                return Err(Error::SingularPositionConstraintSolve);
            }
            let delta = -residual / derivative;
            if !delta.is_finite() {
                return Err(Error::NonFiniteResult);
            }

            let mut fraction = 1.0_f64;
            let mut accepted = false;
            for _ in 0..24 {
                let candidate_alpha = alpha + fraction * delta;
                let candidate_position =
                    sub(predictor, scale(before.position, candidate_alpha));
                let candidate_residual =
                    dot(candidate_position, candidate_position) - self.length * self.length;
                if candidate_alpha.is_finite()
                    && candidate_position.iter().all(|value| value.is_finite())
                    && candidate_residual.abs() < residual.abs()
                {
                    alpha = candidate_alpha;
                    accepted = true;
                    break;
                }
                fraction *= 0.5;
            }
            if !accepted {
                return Err(Error::PositionConstraintNoConvergence);
            }
            position_iterations = iteration + 1;
        }
        if !position_converged {
            let candidate = sub(predictor, scale(before.position, alpha));
            let residual = dot(candidate, candidate) - self.length * self.length;
            let residual_scale = (1.0
                + dot(candidate, candidate)
                + self.length * self.length)
                .max(f64::MIN_POSITIVE);
            position_converged = residual.abs() <= self.newton_tolerance * residual_scale;
        }
        if !position_converged {
            return Err(Error::PositionConstraintNoConvergence);
        }

        let after_position = sub(predictor, scale(before.position, alpha));
        let position_multiplier = alpha / multiplier_scale;
        if !position_multiplier.is_finite() {
            return Err(Error::NonFiniteResult);
        }

        // The position-stage multiplier is already included in the chord
        // velocity p_{n+1/2} = m(q1-q0)/dt.
        let half_momentum = scale(sub(after_position, before.position), self.mass / dt);
        let final_unconstrained_momentum =
            add(half_momentum, scale(self.gravity, 0.5 * dt * self.mass));

        // p1 = p_tilde - dt/2 * mu * q1; choose mu so q1·p1=0.
        let position_norm_squared = dot(after_position, after_position);
        if !position_norm_squared.is_finite()
            || position_norm_squared <= f64::MIN_POSITIVE
        {
            return Err(Error::SingularPositionConstraintSolve);
        }
        let momentum_multiplier =
            2.0 * dot(after_position, final_unconstrained_momentum)
                / (dt * position_norm_squared);
        let after_momentum = sub(
            final_unconstrained_momentum,
            scale(after_position, 0.5 * dt * momentum_multiplier),
        );
        if !momentum_multiplier.is_finite()
            || !after_momentum.iter().all(|value| value.is_finite())
        {
            return Err(Error::NonFiniteResult);
        }
        let after = State {
            position: after_position,
            momentum: after_momentum,
        };

        let position_constraint_residual =
            dot(after.position, after.position) - self.length * self.length;
        let relative_position_constraint_residual = position_constraint_residual.abs()
            / (1.0 + dot(after.position, after.position) + self.length * self.length);
        let tangent_constraint_residual = dot(after.position, after.momentum);
        let relative_tangent_constraint_residual = tangent_constraint_residual.abs()
            / (self.length * norm(after.momentum)).max(f64::MIN_POSITIVE);
        if !relative_position_constraint_residual.is_finite()
            || !relative_tangent_constraint_residual.is_finite()
            || relative_position_constraint_residual > self.constraint_tolerance
            || relative_tangent_constraint_residual > self.constraint_tolerance
        {
            return Err(Error::PositionConstraintNoConvergence);
        }

        let energy_after = self.hamiltonian(after)?;
        let relative_energy_change = (energy_after - energy_before).abs()
            / (energy_before.abs() + energy_after.abs()).max(f64::MIN_POSITIVE);
        if !relative_energy_change.is_finite() {
            return Err(Error::NonFiniteResult);
        }

        Ok(StepReceipt {
            before,
            after,
            energy_before,
            energy_after,
            relative_energy_change,
            position_constraint_residual,
            relative_position_constraint_residual,
            tangent_constraint_residual,
            relative_tangent_constraint_residual,
            position_multiplier,
            momentum_multiplier,
            position_newton_iterations: position_iterations,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> CartesianPendulum {
        CartesianPendulum::new(1.0, 1.0, [0.0, -9.81]).unwrap()
    }

    fn from_intrinsic(angle: f64, angular_momentum: f64, length: f64) -> State {
        State {
            position: [length * angle.sin(), -length * angle.cos()],
            momentum: [
                angular_momentum / length * angle.cos(),
                angular_momentum / length * angle.sin(),
            ],
        }
    }

    fn rotate(value: [f64; 2], angle: f64) -> [f64; 2] {
        let (sin, cos) = angle.sin_cos();
        [cos * value[0] - sin * value[1], sin * value[0] + cos * value[1]]
    }

    fn rk4_reference(
        mut angle: f64,
        mut angular_momentum: f64,
        dt: f64,
        steps: usize,
    ) -> State {
        // Independent high-accuracy reference in reduced canonical
        // coordinates, converted to Cartesian only at the terminal time.
        // It does not reuse the RATTLE constraint or multiplier equations.
        let inertia = 1.0_f64;
        let gravity_scale = 9.81_f64;
        let rhs = |theta: f64, momentum: f64| {
            (
                momentum / inertia,
                -gravity_scale * theta.sin(),
            )
        };
        for _ in 0..steps {
            let (a1q, a1p) = rhs(angle, angular_momentum);
            let (a2q, a2p) = rhs(
                angle + 0.5 * dt * a1q,
                angular_momentum + 0.5 * dt * a1p,
            );
            let (a3q, a3p) = rhs(
                angle + 0.5 * dt * a2q,
                angular_momentum + 0.5 * dt * a2p,
            );
            let (a4q, a4p) = rhs(
                angle + dt * a3q,
                angular_momentum + dt * a3p,
            );
            angle += dt * (a1q + 2.0 * a2q + 2.0 * a3q + a4q) / 6.0;
            angular_momentum += dt * (a1p + 2.0 * a2p + 2.0 * a3p + a4p) / 6.0;
        }
        State {
            position: [angle.sin(), -angle.cos()],
            momentum: [
                angular_momentum * angle.cos(),
                angular_momentum * angle.sin(),
            ],
        }
    }

    #[test]
    fn hanging_equilibrium_remains_stationary() {
        let model = model();
        let state = State {
            position: [0.0, -1.0],
            momentum: [0.0, 0.0],
        };
        for _ in 0..1000 {
            let receipt = model.step(state, 0.01).unwrap();
            assert!(norm(sub(receipt.after.position, state.position)) < 2.0e-14);
            assert!(norm(receipt.after.momentum) < 2.0e-13);
        }
    }

    #[test]
    fn position_and_tangent_constraints_hold_for_long_horizon() {
        let model = model();
        let mut state = from_intrinsic(1.4, 0.2, 1.0);
        let mut maximum_position_residual = 0.0_f64;
        let mut maximum_tangent_residual = 0.0_f64;
        for _ in 0..5000 {
            let receipt = model.step(state, 0.002).unwrap();
            maximum_position_residual = maximum_position_residual
                .max(receipt.relative_position_constraint_residual);
            maximum_tangent_residual = maximum_tangent_residual
                .max(receipt.relative_tangent_constraint_residual);
            state = receipt.after;
        }
        assert!(maximum_position_residual < 1.0e-10);
        assert!(maximum_tangent_residual < 1.0e-10);
    }

    #[test]
    fn symplectic_energy_error_is_bounded_but_not_claimed_exact() {
        let model = model();
        let mut state = from_intrinsic(1.4, 0.2, 1.0);
        let initial_energy = model.hamiltonian(state).unwrap();
        let mut maximum_relative_deviation = 0.0_f64;
        for _ in 0..5000 {
            state = model.step(state, 0.002).unwrap().after;
            let energy = model.hamiltonian(state).unwrap();
            let relative = (energy - initial_energy).abs() / initial_energy.abs();
            maximum_relative_deviation = maximum_relative_deviation.max(relative);
        }
        assert!(
            maximum_relative_deviation < 5.0e-4,
            "energy deviation={maximum_relative_deviation:e}"
        );
    }

    #[test]
    fn zero_gravity_circular_motion_preserves_angular_momentum() {
        let model = CartesianPendulum::new(1.0, 1.0, [0.0, 0.0]).unwrap();
        let mut state = from_intrinsic(0.4, 0.3, 1.0);
        let initial_angular_momentum =
            state.position[0] * state.momentum[1] - state.position[1] * state.momentum[0];
        let initial_energy = model.hamiltonian(state).unwrap();
        for _ in 0..1000 {
            state = model.step(state, 0.001).unwrap().after;
        }
        let final_angular_momentum =
            state.position[0] * state.momentum[1] - state.position[1] * state.momentum[0];
        let final_energy = model.hamiltonian(state).unwrap();
        assert!((final_angular_momentum - initial_angular_momentum).abs() < 1.0e-11);
        assert!((final_energy - initial_energy).abs() < 2.0e-12);
    }

    #[test]
    fn cartesian_rattle_is_second_order_against_independent_rk4() {
        let initial = from_intrinsic(1.1, 0.15, 1.0);
        let gravity = [0.0, -9.81];
        let reference = rk4_reference(1.1, 0.15, 1.0 / 80_000.0, 80_000);
        let dts = [0.04, 0.02, 0.01, 0.005];
        let mut errors = Vec::new();
        for dt in dts {
            let model = CartesianPendulum::new(1.0, 1.0, gravity).unwrap();
            let mut state = initial;
            for _ in 0..(1.0 / dt).round() as usize {
                state = model.step(state, dt).unwrap().after;
            }
            let error = (dot(sub(state.position, reference.position), sub(state.position, reference.position))
                + dot(sub(state.momentum, reference.momentum), sub(state.momentum, reference.momentum)))
            .sqrt();
            errors.push(error);
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
    fn rigid_coordinate_rotation_is_metamorphic_invariant() {
        let model = CartesianPendulum::new(1.0, 1.0, [0.0, -9.81]).unwrap();
        let angle = 0.73;
        let q = rotate([0.8_f64.sin(), -0.8_f64.cos()], angle);
        let p = rotate([0.2 * 0.8_f64.cos(), 0.2 * 0.8_f64.sin()], angle);
        let gravity = rotate([0.0, -9.81], angle);
        let rotated_model = CartesianPendulum::new(1.0, 1.0, gravity).unwrap();
        let state = State {
            position: [0.8_f64.sin(), -0.8_f64.cos()],
            momentum: [0.2 * 0.8_f64.cos(), 0.2 * 0.8_f64.sin()],
        };
        let normal = model.step(state, 0.01).unwrap();
        let rotated = rotated_model
            .step(
                State {
                    position: rotate(state.position, angle),
                    momentum: rotate(state.momentum, angle),
                },
                0.01,
            )
            .unwrap();
        assert!(
            norm(sub(rotated.after.position, rotate(normal.after.position, angle))) < 2.0e-12
        );
        assert!(
            norm(sub(rotated.after.momentum, rotate(normal.after.momentum, angle))) < 2.0e-12
        );
        // q and p above are also checked so the fixture explicitly exercises
        // a non-axis-aligned reference orientation.
        assert!(norm(sub(q, rotate(state.position, angle))) < 1.0e-14);
        assert!(norm(sub(p, rotate(state.momentum, angle))) < 1.0e-14);
    }

    #[test]
    fn invalid_initial_manifold_and_invalid_parameters_fail_closed() {
        let model = model();
        assert_eq!(
            model.step(
                State {
                    position: [0.0, -0.9],
                    momentum: [0.0, 0.0],
                },
                0.01,
            )
            .unwrap_err(),
            Error::InvalidInitialPositionConstraint
        );
        assert_eq!(
            model.step(
                State {
                    position: [0.0, -1.0],
                    momentum: [1.0, 0.0],
                },
                0.01,
            )
            .unwrap_err(),
            Error::InvalidInitialTangentConstraint
        );
        assert_eq!(
            CartesianPendulum::new(0.0, 1.0, [0.0, -9.81]).unwrap_err(),
            Error::InvalidParameter
        );
        assert_eq!(
            model.step(from_intrinsic(0.5, 0.2, 1.0), 0.0).unwrap_err(),
            Error::InvalidTimeStep
        );
    }
}

#[cfg(not(test))]
fn main() {
    let model = CartesianPendulum::new(1.0, 1.0, [0.0, -9.81])
        .expect("frozen model must be valid");
    let mut state = from_angle(1.1, 0.15, 1.0);
    println!("step,x,y,px,py,energy,position_residual,tangent_residual");
    for step in 0..500 {
        let receipt = model.step(state, 0.002).expect("RATTLE step must converge");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
            step + 1,
            state.position[0],
            state.position[1],
            state.momentum[0],
            state.momentum[1],
            receipt.energy_after,
            receipt.relative_position_constraint_residual,
            receipt.relative_tangent_constraint_residual
        );
    }
}

#[cfg(not(test))]
fn from_angle(angle: f64, angular_momentum: f64, length: f64) -> State {
    State {
        position: [length * angle.sin(), -length * angle.cos()],
        momentum: [
            angular_momentum / length * angle.cos(),
            angular_momentum / length * angle.sin(),
        ],
    }
}
