// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Cartesian RATTLE baseline for the same two-mass constrained dumbbell
// used by the semi-explicit pHDAE discrete-gradient reference.

use std::f64;

type Vec2 = [f64; 2];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Error {
    InvalidParameter,
    InvalidTimeStep,
    InvalidInitialLinkConstraint,
    InvalidInitialVelocityConstraint,
    NonFiniteInput,
    SingularPositionSolve,
    UnresolvableTimeStep,
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
pub struct CartesianRattleDumbbell {
    mass1: f64,
    mass2: f64,
    length: f64,
    constraint_tolerance: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct StepReceipt {
    pub before: State,
    pub after: State,
    pub position_multiplier: f64,
    pub velocity_multiplier: f64,
    pub energy_before: f64,
    pub energy_after: f64,
    pub relative_energy_change: f64,
    pub relative_link_constraint_residual: f64,
    pub endpoint_velocity_constraint_residual: f64,
    pub relative_endpoint_velocity_constraint_residual: f64,
    pub midpoint_velocity_constraint_residual: f64,
    pub relative_linear_momentum_residual: f64,
    pub relative_angular_momentum_residual: f64,
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

impl CartesianRattleDumbbell {
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
        let r = sub(state.q2, state.q1);
        let w = sub(state.v2, state.v1);
        let link_error = (dot(r, r) - self.length * self.length).abs();
        let link_scale =
            (1.0 + dot(r, r) + self.length * self.length).max(f64::MIN_POSITIVE);
        if link_error > self.constraint_tolerance * link_scale {
            return Err(Error::InvalidInitialLinkConstraint);
        }
        let tangent_error = dot(r, w).abs();
        let tangent_scale = (1.0 + self.length * norm(w)).max(f64::MIN_POSITIVE);
        if tangent_error > self.constraint_tolerance * tangent_scale {
            return Err(Error::InvalidInitialVelocityConstraint);
        }
        Ok(())
    }

    /// One Cartesian RATTLE step for a torque-free, force-free dumbbell.
    ///
    /// Position stage: solve the quadratic SHAKE/RATTLE multiplier so the
    /// updated link vector has exactly length L.
    ///
    /// Velocity stage: solve the second multiplier so r1·(v2_1-v1_1)=0.
    ///
    /// This is the standard symplectic constrained baseline for the same
    /// free dumbbell model as the pHDAE discrete-gradient implementation.
    /// It enforces endpoint tangency but does not post-correct energy.
    pub fn step(&self, before: State, dt: f64) -> Result<StepReceipt, Error> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        self.validate_initial(before)?;
        let energy_before = self.hamiltonian(before)?;
        let momentum_before = self.linear_momentum(before);
        let angular_before = self.angular_momentum(before);

        let r0 = sub(before.q2, before.q1);
        let w0 = sub(before.v2, before.v1);
        let inverse_mass_sum = 1.0 / self.mass1 + 1.0 / self.mass2;
        let radius_squared = dot(r0, r0);
        let drift = scale(w0, dt);
        let linear_term = dot(r0, drift);
        let discriminant = linear_term * linear_term
            - radius_squared * (dot(drift, drift) - self.length * self.length);
        let discriminant_scale = (linear_term * linear_term).abs()
            + (radius_squared * (dot(drift, drift) - self.length * self.length)).abs()
            + radius_squared * self.length * self.length;
        if !discriminant.is_finite() {
            return Err(Error::NonFiniteResult);
        }
        if discriminant < -f64::EPSILON * discriminant_scale.max(f64::MIN_POSITIVE) {
            return Err(Error::UnresolvableTimeStep);
        }
        let root = discriminant.max(0.0).sqrt();

        // Select the root continuous with the identity multiplier as dt -> 0.
        let z = (-linear_term + root) / radius_squared;
        let beta = 1.0 - z;
        let position_multiplier = 2.0 * beta / (dt * dt * inverse_mass_sum);
        if !position_multiplier.is_finite() {
            return Err(Error::NonFiniteResult);
        }

        let after_q1 = add(
            before.q1,
            add(
                scale(before.v1, dt),
                scale(r0, 0.5 * dt * dt * position_multiplier / self.mass1),
            ),
        );
        let after_q2 = add(
            before.q2,
            sub(
                scale(before.v2, dt),
                scale(r0, 0.5 * dt * dt * position_multiplier / self.mass2),
            ),
        );
        let after_link = sub(after_q2, after_q1);
        let chord_v1 = scale(sub(after_q1, before.q1), 1.0 / dt);
        let chord_v2 = scale(sub(after_q2, before.q2), 1.0 / dt);
        let chord_relative_velocity = sub(chord_v2, chord_v1);
        let after_link_squared = dot(after_link, after_link);
        if !after_link_squared.is_finite()
            || after_link_squared <= f64::MIN_POSITIVE
        {
            return Err(Error::SingularPositionSolve);
        }

        // Endpoint velocity multiplier: r1 · (w_chord - h/2*mu*invsum*r1)=0.
        let velocity_multiplier = 2.0 * dot(after_link, chord_relative_velocity)
            / (dt * inverse_mass_sum * after_link_squared);
        let after_v1 = add(
            chord_v1,
            scale(after_link, 0.5 * dt * velocity_multiplier / self.mass1),
        );
        let after_v2 = sub(
            chord_v2,
            scale(after_link, 0.5 * dt * velocity_multiplier / self.mass2),
        );
        if !velocity_multiplier.is_finite()
            || !after_v1.iter().all(|x| x.is_finite())
            || !after_v2.iter().all(|x| x.is_finite())
        {
            return Err(Error::NonFiniteResult);
        }

        let after = State {
            q1: after_q1,
            q2: after_q2,
            v1: after_v1,
            v2: after_v2,
        };
        let endpoint_link = sub(after.q2, after.q1);
        let endpoint_relative_velocity = sub(after.v2, after.v1);
        let link_residual = (dot(endpoint_link, endpoint_link)
            - self.length * self.length)
            .abs()
            / (1.0 + dot(endpoint_link, endpoint_link) + self.length * self.length);
        let endpoint_tangent = dot(endpoint_link, endpoint_relative_velocity);
        let endpoint_tangent_relative = endpoint_tangent.abs()
            / (1.0 + self.length * norm(endpoint_relative_velocity));

        let midpoint_relative_position = scale(add(r0, endpoint_link), 0.5);
        let midpoint_relative_velocity =
            scale(add(w0, endpoint_relative_velocity), 0.5);
        let midpoint_constraint =
            dot(midpoint_relative_position, midpoint_relative_velocity);

        let energy_after = self.hamiltonian(after)?;
        let relative_energy_change = (energy_after - energy_before).abs()
            / (energy_before.abs() + energy_after.abs()).max(f64::MIN_POSITIVE);

        let momentum_after = self.linear_momentum(after);
        let relative_linear_momentum_residual = norm(sub(momentum_after, momentum_before))
            / (1.0 + norm(momentum_before) + norm(momentum_after));
        let angular_after = self.angular_momentum(after);
        let relative_angular_momentum_residual = (angular_after - angular_before).abs()
            / (1.0 + angular_before.abs() + angular_after.abs());

        if !link_residual.is_finite()
            || !endpoint_tangent.is_finite()
            || !endpoint_tangent_relative.is_finite()
            || !midpoint_constraint.is_finite()
            || !relative_energy_change.is_finite()
            || !relative_linear_momentum_residual.is_finite()
            || !relative_angular_momentum_residual.is_finite()
        {
            return Err(Error::NonFiniteResult);
        }
        if link_residual > self.constraint_tolerance
            || endpoint_tangent_relative > self.constraint_tolerance
        {
            return Err(Error::UnresolvableTimeStep);
        }

        Ok(StepReceipt {
            before,
            after,
            position_multiplier,
            velocity_multiplier,
            energy_before,
            energy_after,
            relative_energy_change,
            relative_link_constraint_residual: link_residual,
            endpoint_velocity_constraint_residual: endpoint_tangent,
            relative_endpoint_velocity_constraint_residual: endpoint_tangent_relative,
            midpoint_velocity_constraint_residual: midpoint_constraint,
            relative_linear_momentum_residual,
            relative_angular_momentum_residual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> CartesianRattleDumbbell {
        CartesianRattleDumbbell::new(1.3, 2.1, 1.2).unwrap()
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

    fn exact_state(initial: State, mass1: f64, mass2: f64, length: f64, time: f64) -> State {
        let total_mass = mass1 + mass2;
        let center_position = scale(
            add(scale(initial.q1, mass1), scale(initial.q2, mass2)),
            1.0 / total_mass,
        );
        let center_velocity = scale(
            add(scale(initial.v1, mass1), scale(initial.v2, mass2)),
            1.0 / total_mass,
        );
        let r0 = sub(initial.q2, initial.q1);
        let w0 = sub(initial.v2, initial.v1);
        let omega = cross(r0, w0) / (length * length);
        let r = rotate(r0, omega * time);
        let w = rotate(w0, omega * time);
        let center = add(center_position, scale(center_velocity, time));
        State {
            q1: sub(center, scale(r, mass2 / total_mass)),
            q2: add(center, scale(r, mass1 / total_mass)),
            v1: sub(center_velocity, scale(w, mass2 / total_mass)),
            v2: add(center_velocity, scale(w, mass1 / total_mass)),
        }
    }

    fn state_error(a: State, b: State) -> f64 {
        (dot(sub(a.q1, b.q1), sub(a.q1, b.q1))
            + dot(sub(a.q2, b.q2), sub(a.q2, b.q2))
            + dot(sub(a.v1, b.v1), sub(a.v1, b.v1))
            + dot(sub(a.v2, b.v2), sub(a.v2, b.v2)))
        .sqrt()
    }

    #[test]
    fn rattle_enforces_link_and_endpoint_tangent_constraints() {
        let model = model();
        let mut state = initial_state();
        let mut max_link = 0.0_f64;
        let mut max_tangent = 0.0_f64;
        for _ in 0..5000 {
            let receipt = model.step(state, 0.002).unwrap();
            max_link = max_link.max(receipt.relative_link_constraint_residual);
            max_tangent =
                max_tangent.max(receipt.relative_endpoint_velocity_constraint_residual);
            state = receipt.after;
        }
        assert!(max_link < 1.0e-12);
        assert!(max_tangent < 1.0e-12);
    }

    #[test]
    fn rattle_preserves_the_free_dumbbell_momentum_maps() {
        let model = model();
        let initial = initial_state();
        let p0 = model.linear_momentum(initial);
        let j0 = model.angular_momentum(initial);
        let mut state = initial;
        for _ in 0..5000 {
            state = model.step(state, 0.002).unwrap().after;
        }
        assert!(norm(sub(model.linear_momentum(state), p0)) < 2.0e-11);
        assert!((model.angular_momentum(state) - j0).abs() < 2.0e-11);
    }

    #[test]
    fn rattle_is_second_order_against_exact_free_rotation() {
        let model = model();
        let initial = initial_state();
        let reference = exact_state(initial, 1.3, 2.1, 1.2, 1.0);
        let mut errors = Vec::new();
        for dt in [0.04, 0.02, 0.01, 0.005] {
            let mut state = initial;
            for _ in 0..(1.0 / dt).round() as usize {
                state = model.step(state, dt).unwrap().after;
            }
            errors.push(state_error(state, reference));
        }
        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (3.6..4.4).contains(&ratio),
                "RATTLE should be second order on this smooth case: {errors:?}"
            );
        }
    }

    #[test]
    fn rattle_commutes_with_arbitrary_rotation() {
        let model = model();
        let initial = initial_state();
        let angle = 0.731_f64;
        let normal = model.step(initial, 0.01).unwrap();
        let rotated = model.step(rotate_state(initial, angle), 0.01).unwrap();
        assert!(
            state_error(rotated.after, rotate_state(normal.after, angle)) < 3.0e-12
        );
    }

    #[test]
    fn invalid_inputs_and_unresolvable_timestep_fail_closed() {
        let model = model();
        assert_eq!(
            model.step(
                State {
                    q2: [0.7, 0.2],
                    ..initial_state()
                },
                0.01
            )
            .unwrap_err(),
            Error::InvalidInitialLinkConstraint
        );
        assert_eq!(
            model.step(
                State {
                    v2: [0.35, 0.62],
                    ..initial_state()
                },
                0.01
            )
            .unwrap_err(),
            Error::InvalidInitialVelocityConstraint
        );
        assert_eq!(
            CartesianRattleDumbbell::new(0.0, 1.0, 1.0).unwrap_err(),
            Error::InvalidParameter
        );
        assert_eq!(model.step(initial_state(), 0.0).unwrap_err(), Error::InvalidTimeStep);
        assert_eq!(
            model.step(initial_state(), 10.0).unwrap_err(),
            Error::UnresolvableTimeStep
        );
    }
}

#[cfg(not(test))]
fn main() {
    let model = CartesianRattleDumbbell::new(1.3, 2.1, 1.2)
        .expect("frozen RATTLE model must be valid");
    let mut state = State {
        q1: [-0.6, 0.2],
        q2: [0.6, 0.2],
        v1: [0.25, -0.18],
        v2: [0.25, 0.62],
    };
    println!("step,energy,link_residual,endpoint_tangent,linear_momentum_residual,angular_momentum_residual");
    for step in 0..500 {
        let receipt = model.step(state, 0.002).expect("RATTLE step must be admissible");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
            step + 1,
            receipt.energy_after,
            receipt.relative_link_constraint_residual,
            receipt.endpoint_velocity_constraint_residual.abs(),
            receipt.relative_linear_momentum_residual,
            receipt.relative_angular_momentum_residual
        );
    }
}
