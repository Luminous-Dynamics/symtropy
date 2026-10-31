// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//
// Cartesian 3D RATTLE baseline for the same constrained dumbbell model
// used by the semi-explicit pHDAE discrete-gradient reference.

use std::f64;

pub type Vec3 = [f64; 3];

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
    pub q1: Vec3,
    pub q2: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct CartesianRattleDumbbell3D {
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
    pub external_work: f64,
    /// ΔH - external work. This includes the method's numerical constraint
    /// energy defect and is not labelled physical dissipation.
    pub energy_balance_residual: f64,
    pub relative_energy_balance_residual: f64,
    pub relative_link_constraint_residual: f64,
    pub endpoint_velocity_constraint_residual: f64,
    pub relative_endpoint_velocity_constraint_residual: f64,
    pub midpoint_velocity_constraint_residual: f64,
    pub relative_linear_momentum_residual: f64,
    pub angular_momentum_residual: Vec3,
    pub relative_angular_momentum_residual: f64,
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

impl CartesianRattleDumbbell3D {
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

    /// Force-free convenience step.
    pub fn step(&self, before: State, dt: f64) -> Result<StepReceipt, Error> {
        self.step_with_accelerations(before, [0.0; 3], [0.0; 3], dt)
    }

    /// One 3D Cartesian RATTLE step with constant mass-normalized external
    /// accelerations applied at the two mass centers.
    pub fn step_with_accelerations(
        &self,
        before: State,
        acceleration1: Vec3,
        acceleration2: Vec3,
        dt: f64,
    ) -> Result<StepReceipt, Error> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(Error::InvalidTimeStep);
        }
        if !acceleration1.iter().all(|x| x.is_finite())
            || !acceleration2.iter().all(|x| x.is_finite())
        {
            return Err(Error::NonFiniteInput);
        }
        self.validate_initial(before)?;

        let energy_before = self.hamiltonian(before)?;
        let linear_before = self.linear_momentum(before);
        let angular_before = self.angular_momentum(before);
        let r0 = sub(before.q2, before.q1);
        let w0 = sub(before.v2, before.v1);
        let inverse_mass_sum = 1.0 / self.mass1 + 1.0 / self.mass2;
        let relative_free_drift = add(
            scale(w0, dt),
            scale(sub(acceleration2, acceleration1), 0.5 * dt * dt),
        );
        let radius_squared = dot(r0, r0);
        let linear_term = dot(r0, relative_free_drift);
        let discriminant = linear_term * linear_term
            - radius_squared
                * (dot(relative_free_drift, relative_free_drift)
                    - self.length * self.length);
        let discriminant_scale = (linear_term * linear_term).abs()
            + (radius_squared
                * (dot(relative_free_drift, relative_free_drift)
                    - self.length * self.length))
                .abs()
            + radius_squared * self.length * self.length;
        if !discriminant.is_finite() {
            return Err(Error::NonFiniteResult);
        }
        if discriminant < -f64::EPSILON * discriminant_scale.max(f64::MIN_POSITIVE) {
            return Err(Error::UnresolvableTimeStep);
        }
        let root = discriminant.max(0.0).sqrt();
        let z = (-linear_term + root) / radius_squared;
        let beta = 1.0 - z;
        let position_multiplier = 2.0 * beta / (dt * dt * inverse_mass_sum);
        if !position_multiplier.is_finite() {
            return Err(Error::NonFiniteResult);
        }

        let after_q1 = add(
            before.q1,
            add(
                add(
                    scale(before.v1, dt),
                    scale(acceleration1, 0.5 * dt * dt),
                ),
                scale(r0, 0.5 * dt * dt * position_multiplier / self.mass1),
            ),
        );
        let after_q2 = add(
            before.q2,
            add(
                add(
                    scale(before.v2, dt),
                    scale(acceleration2, 0.5 * dt * dt),
                ),
                scale(r0, -0.5 * dt * dt * position_multiplier / self.mass2),
            ),
        );
        let after_link = sub(after_q2, after_q1);
        let chord_v1 = scale(sub(after_q1, before.q1), 1.0 / dt);
        let chord_v2 = scale(sub(after_q2, before.q2), 1.0 / dt);
        let chord_relative_velocity = add(
            sub(chord_v2, chord_v1),
            scale(sub(acceleration2, acceleration1), 0.5 * dt),
        );
        let after_link_squared = dot(after_link, after_link);
        if !after_link_squared.is_finite()
            || after_link_squared <= f64::MIN_POSITIVE
        {
            return Err(Error::SingularPositionSolve);
        }

        let velocity_multiplier = 2.0 * dot(after_link, chord_relative_velocity)
            / (dt * inverse_mass_sum * after_link_squared);
        let after_v1 = add(
            add(chord_v1, scale(acceleration1, 0.5 * dt)),
            scale(after_link, 0.5 * dt * velocity_multiplier / self.mass1),
        );
        let after_v2 = add(
            add(chord_v2, scale(acceleration2, 0.5 * dt)),
            scale(after_link, -0.5 * dt * velocity_multiplier / self.mass2),
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
        let r1 = sub(after.q2, after.q1);
        let w1 = sub(after.v2, after.v1);
        let link_residual = (dot(r1, r1) - self.length * self.length).abs()
            / (1.0 + dot(r1, r1) + self.length * self.length);
        let endpoint_tangent = dot(r1, w1);
        let endpoint_tangent_relative =
            endpoint_tangent.abs() / (1.0 + self.length * norm(w1));

        let rbar = scale(add(r0, r1), 0.5);
        let wbar = scale(add(w0, w1), 0.5);
        let midpoint_constraint = dot(rbar, wbar);
        let relative_midpoint_constraint =
            midpoint_constraint.abs() / (1.0 + norm(rbar) * norm(wbar));

        let energy_after = self.hamiltonian(after)?;
        let relative_energy_change = (energy_after - energy_before).abs()
            / (energy_before.abs() + energy_after.abs()).max(f64::MIN_POSITIVE);
        let force1 = scale(acceleration1, self.mass1);
        let force2 = scale(acceleration2, self.mass2);
        let external_work = dot(force1, sub(after.q1, before.q1))
            + dot(force2, sub(after.q2, before.q2));
        let energy_balance_residual = energy_after - energy_before - external_work;
        let relative_energy_balance_residual = energy_balance_residual.abs()
            / (1.0
                + energy_before.abs()
                + energy_after.abs()
                + external_work.abs());

        let linear_after = self.linear_momentum(after);
        let expected_linear = add(linear_before, scale(add(force1, force2), dt));
        let relative_linear_residual = norm(sub(linear_after, expected_linear))
            / (1.0 + norm(linear_before) + norm(expected_linear));

        let angular_after = self.angular_momentum(after);
        let qbar1 = scale(add(before.q1, after.q1), 0.5);
        let qbar2 = scale(add(before.q2, after.q2), 0.5);
        let expected_torque_impulse = scale(
            add(cross(qbar1, force1), cross(qbar2, force2)),
            dt,
        );
        let angular_residual = sub(
            sub(angular_after, angular_before),
            expected_torque_impulse,
        );
        let relative_angular_residual = norm(angular_residual)
            / (1.0
                + norm(angular_before)
                + norm(angular_after)
                + norm(expected_torque_impulse));

        if !link_residual.is_finite()
            || !endpoint_tangent.is_finite()
            || !endpoint_tangent_relative.is_finite()
            || !relative_midpoint_constraint.is_finite()
            || !relative_energy_change.is_finite()
            || !external_work.is_finite()
            || !energy_balance_residual.is_finite()
            || !relative_energy_balance_residual.is_finite()
            || !relative_linear_residual.is_finite()
            || !relative_angular_residual.is_finite()
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
            external_work,
            energy_balance_residual,
            relative_energy_balance_residual,
            relative_link_constraint_residual: link_residual,
            endpoint_velocity_constraint_residual: endpoint_tangent,
            relative_endpoint_velocity_constraint_residual: endpoint_tangent_relative,
            midpoint_velocity_constraint_residual: midpoint_constraint,
            relative_linear_momentum_residual: relative_linear_residual,
            angular_momentum_residual: angular_residual,
            relative_angular_momentum_residual: relative_angular_residual,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASS_1: f64 = 1.3;
    const MASS_2: f64 = 2.1;
    const LENGTH: f64 = 1.2;

    fn model() -> CartesianRattleDumbbell3D {
        CartesianRattleDumbbell3D::new(MASS_1, MASS_2, LENGTH).unwrap()
    }

    fn initial_state() -> State {
        let center = [0.25, -0.12, 0.30];
        let center_velocity = [0.17, -0.23, 0.11];
        let relative_position = [0.8, 0.8, 0.4];
        let relative_velocity = [0.2, -0.1, -0.2];
        let total_mass = MASS_1 + MASS_2;
        State {
            q1: sub(center, scale(relative_position, MASS_2 / total_mass)),
            q2: add(center, scale(relative_position, MASS_1 / total_mass)),
            v1: sub(center_velocity, scale(relative_velocity, MASS_2 / total_mass)),
            v2: add(center_velocity, scale(relative_velocity, MASS_1 / total_mass)),
        }
    }

    fn rotate(state: State, rotation_vector: Vec3) -> State {
        let rotate_one = |v| {
            let angle = norm(rotation_vector);
            if angle < 1.0e-8 {
                let first = cross(rotation_vector, v);
                return add(add(v, first), scale(cross(rotation_vector, first), 0.5));
            }
            let axis = scale(rotation_vector, 1.0 / angle);
            let (sin, cos) = angle.sin_cos();
            add(
                add(scale(v, cos), scale(cross(axis, v), sin)),
                scale(axis, dot(axis, v) * (1.0 - cos)),
            )
        };
        State {
            q1: rotate_one(state.q1),
            q2: rotate_one(state.q2),
            v1: rotate_one(state.v1),
            v2: rotate_one(state.v2),
        }
    }

    fn state_error(a: State, b: State) -> f64 {
        (dot(sub(a.q1, b.q1), sub(a.q1, b.q1))
            + dot(sub(a.q2, b.q2), sub(a.q2, b.q2))
            + dot(sub(a.v1, b.v1), sub(a.v1, b.v1))
            + dot(sub(a.v2, b.v2), sub(a.v2, b.v2)))
        .sqrt()
    }

    fn exact_state(initial: State, time: f64) -> State {
        let total_mass = MASS_1 + MASS_2;
        let center = scale(
            add(scale(initial.q1, MASS_1), scale(initial.q2, MASS_2)),
            1.0 / total_mass,
        );
        let center_velocity = scale(
            add(scale(initial.v1, MASS_1), scale(initial.v2, MASS_2)),
            1.0 / total_mass,
        );
        let r = sub(initial.q2, initial.q1);
        let w = sub(initial.v2, initial.v1);
        let angular_velocity = scale(cross(r, w), 1.0 / (LENGTH * LENGTH));
        let rotation = scale(angular_velocity, time);
        let angle = norm(rotation);
        let rotate_vec = |x: Vec3| {
            if angle < 1.0e-8 {
                let first = cross(rotation, x);
                return add(add(x, first), scale(cross(rotation, first), 0.5));
            }
            let axis = scale(rotation, 1.0 / angle);
            let (sin, cos) = angle.sin_cos();
            add(
                add(scale(x, cos), scale(cross(axis, x), sin)),
                scale(axis, dot(axis, x) * (1.0 - cos)),
            )
        };
        let r1 = rotate_vec(r);
        let w1 = rotate_vec(w);
        let center1 = add(center, scale(center_velocity, time));
        State {
            q1: sub(center1, scale(r1, MASS_2 / total_mass)),
            q2: add(center1, scale(r1, MASS_1 / total_mass)),
            v1: sub(center_velocity, scale(w1, MASS_2 / total_mass)),
            v2: add(center_velocity, scale(w1, MASS_1 / total_mass)),
        }
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
        assert!(max_link < 2.0e-12);
        assert!(max_tangent < 2.0e-12);
    }

    #[test]
    fn rattle_preserves_linear_and_full_angular_momentum_vectors() {
        let model = model();
        let mut state = initial_state();
        let linear0 = model.linear_momentum(state);
        let angular0 = model.angular_momentum(state);
        for _ in 0..5000 {
            state = model.step(state, 0.002).unwrap().after;
        }
        assert!(norm(sub(model.linear_momentum(state), linear0)) < 3.0e-11);
        assert!(norm(sub(model.angular_momentum(state), angular0)) < 3.0e-11);
    }

    #[test]
    fn rattle_is_second_order_against_exact_rodrigues_trajectory() {
        let model = model();
        let initial = initial_state();
        let reference = exact_state(initial, 1.0);
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
                "RATTLE should be second-order in this free 3D case: {errors:?}"
            );
        }
    }

    #[test]
    fn rattle_commutes_with_generic_so3_rotations() {
        let model = model();
        let initial = initial_state();
        let force1 = [0.12, -0.07, 0.05];
        let force2 = [-0.03, 0.09, 0.11];
        let normal = model
            .step_with_accelerations(initial, force1, force2, 0.01)
            .unwrap();
        for i in 0..20 {
            let raw_axis = [
                (0.37 * (i as f64 + 1.0)).sin(),
                (0.23 * (i as f64 + 2.0)).cos(),
                (0.41 * (i as f64 + 3.0)).sin(),
            ];
            let axis = scale(raw_axis, 1.0 / norm(raw_axis));
            let rotation = scale(axis, 0.137 * (i as f64 + 1.0));
            let rotated = model
                .step_with_accelerations(
                    rotate(initial, rotation),
                    {
                        let angle = norm(rotation);
                        let axis = scale(rotation, 1.0 / angle);
                        let (sin, cos) = angle.sin_cos();
                        add(
                            add(scale(force1, cos), scale(cross(axis, force1), sin)),
                            scale(axis, dot(axis, force1) * (1.0 - cos)),
                        )
                    },
                    {
                        let angle = norm(rotation);
                        let axis = scale(rotation, 1.0 / angle);
                        let (sin, cos) = angle.sin_cos();
                        add(
                            add(scale(force2, cos), scale(cross(axis, force2), sin)),
                            scale(axis, dot(axis, force2) * (1.0 - cos)),
                        )
                    },
                    0.01,
                )
                .unwrap();
            assert!(
                state_error(rotated.after, rotate(normal.after, rotation)) < 1.0e-11,
                "SO(3) rotation case {i} failed"
            );
        }
    }

    #[test]
    fn invalid_state_force_timestep_and_large_step_fail_closed() {
        let model = model();
        assert_eq!(
            model.step(
                State {
                    q2: [1.5, 0.2, 0.1],
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
                    v2: [0.35, -0.1, -0.2],
                    ..initial_state()
                },
                0.01
            )
            .unwrap_err(),
            Error::InvalidInitialVelocityConstraint
        );
        assert_eq!(
            CartesianRattleDumbbell3D::new(0.0, 1.0, 1.0).unwrap_err(),
            Error::InvalidParameter
        );
        assert_eq!(
            model.step_with_accelerations(
                initial_state(),
                [f64::NAN, 0.0, 0.0],
                [0.0; 3],
                0.01
            )
            .unwrap_err(),
            Error::NonFiniteInput
        );
        assert_eq!(
            model.step(initial_state(), 0.0).unwrap_err(),
            Error::InvalidTimeStep
        );
        assert_eq!(
            model.step(initial_state(), 10.0).unwrap_err(),
            Error::UnresolvableTimeStep
        );
    }
}

#[cfg(not(test))]
fn main() {
    let model = CartesianRattleDumbbell3D::new(1.3, 2.1, 1.2)
        .expect("frozen RATTLE model must be valid");
    let mut state = State {
        q1: [-0.24411764705882355, -0.6141176470588235, 0.0529411764705882],
        q2: [0.5558823529411765, 0.1858823529411765, 0.4529411764705882],
        v1: [0.0464705882352941, -0.1682352941176471, 0.2335294117647060],
        v2: [0.2464705882352941, -0.2682352941176471, 0.0335294117647059],
    };
    println!("step,energy,link_residual,endpoint_tangent,linear_momentum_residual,angular_momentum_x,angular_momentum_y,angular_momentum_z");
    for step in 0..500 {
        let receipt = model.step(state, 0.002).expect("frozen 3D RATTLE step must be admissible");
        state = receipt.after;
        println!(
            "{},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e},{:.17e}",
            step + 1,
            receipt.energy_after,
            receipt.relative_link_constraint_residual,
            receipt.endpoint_velocity_constraint_residual.abs(),
            receipt.relative_linear_momentum_residual,
            receipt.angular_momentum_residual[0],
            receipt.angular_momentum_residual[1],
            receipt.angular_momentum_residual[2]
        );
    }
}
