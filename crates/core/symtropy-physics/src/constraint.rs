// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Constraint trait for ND joints and connections.

use crate::body::{BodyHandle, RigidBody};

/// Per-constraint actuator state for one physics world step.
///
/// The world creates a fresh value before velocity iterations and reuses it
/// across those iterations. This keeps actuator impulse limits tied to elapsed
/// simulation time rather than solver iteration count.
#[derive(Clone, Copy, Debug, Default)]
pub struct MotorStepState {
    accumulated_impulse: f64,
    mechanical_work_j: f64,
    active: bool,
}

impl MotorStepState {
    /// Add one desired impulse increment to the accumulated per-step impulse,
    /// then project the total into the symmetric effort budget.
    ///
    /// The controller is reevaluated on each solver iteration using the
    /// current relative velocity. Its output is therefore an incremental
    /// impulse, not an absolute accumulated impulse. Returns only the change
    /// to apply and the resulting relative velocity. Midpoint work matches the
    /// exact kinetic-energy change for a fixed scalar effective inverse mass.
    pub fn apply_impulse_increment(
        &mut self,
        desired_impulse_increment: f64,
        maximum_step_impulse: f64,
        relative_velocity_before: f64,
        effective_inverse_mass: f64,
    ) -> Option<(f64, f64)> {
        if !desired_impulse_increment.is_finite()
            || !maximum_step_impulse.is_finite()
            || !relative_velocity_before.is_finite()
            || !effective_inverse_mass.is_finite()
            || maximum_step_impulse < 0.0
            || effective_inverse_mass <= 0.0
        {
            return None;
        }

        let candidate_total = self.accumulated_impulse + desired_impulse_increment;
        if !candidate_total.is_finite() {
            return None;
        }
        let projected_total = candidate_total.clamp(
            -maximum_step_impulse,
            maximum_step_impulse,
        );
        let delta_impulse = projected_total - self.accumulated_impulse;
        let relative_velocity_after =
            relative_velocity_before + effective_inverse_mass * delta_impulse;
        let incremental_work =
            delta_impulse * (relative_velocity_before + relative_velocity_after) * 0.5;
        let accumulated_work = self.mechanical_work_j + incremental_work;

        if !delta_impulse.is_finite()
            || !relative_velocity_after.is_finite()
            || !incremental_work.is_finite()
            || !accumulated_work.is_finite()
        {
            return None;
        }

        self.accumulated_impulse = projected_total;
        self.mechanical_work_j = accumulated_work;
        self.active = true;
        Some((delta_impulse, relative_velocity_after))
    }

    /// Consume the single aggregate mechanical-port work receipt for this step.
    pub fn take_work_receipt(&mut self) -> Option<f64> {
        if !self.active {
            return None;
        }
        self.active = false;
        Some(self.mechanical_work_j)
    }

    /// Total accumulated generalized impulse for diagnostics and tests.
    pub fn accumulated_impulse(&self) -> f64 {
        self.accumulated_impulse
    }
}

/// A constraint between two rigid bodies.
pub trait Constraint<const D: usize>: Send + Sync + std::any::Any {
    /// The two bodies this constraint connects.
    fn bodies(&self) -> (BodyHandle, BodyHandle);

    /// Cast to std::any::Any for downcasting in tests/lab.
    fn as_any(&self) -> &dyn std::any::Any;

    /// Cast to std::any::Any for downcasting in tests/lab.
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;

    /// Solve the constraint by applying positional corrections.
    /// Called multiple times per frame for iterative convergence.
    fn solve(&self, body_a: &mut RigidBody<D>, body_b: &mut RigidBody<D>, dt: f64);

    /// Optional velocity-level correction (impulse-based, Fix 7).
    /// Called after position solve for hybrid PBD+impulse dynamics.
    /// Default: no-op (pure PBD).
    fn solve_velocity(
        &self,
        _body_a: &mut RigidBody<D>,
        _body_b: &mut RigidBody<D>,
        _dt: f64,
        _callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
    ) {
    }

    /// State-aware velocity solve used by the world for iterative motor
    /// constraints. The default preserves existing third-party implementations
    /// by delegating to solve_velocity; motorized joints override this hook to
    /// share one MotorStepState across all solver iterations.
    fn solve_velocity_with_state(
        &self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        dt: f64,
        callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
        _motor_state: &mut MotorStepState,
    ) {
        self.solve_velocity(body_a, body_b, dt, callback);
    }
}

/// Distance constraint: keeps two bodies at a fixed distance.
pub struct DistanceConstraint<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub rest_length: f64,
    pub stiffness: f64,
}

impl<const D: usize> Constraint<D> for DistanceConstraint<D> {
    fn bodies(&self) -> (BodyHandle, BodyHandle) {
        (self.body_a, self.body_b)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn solve(&self, body_a: &mut RigidBody<D>, body_b: &mut RigidBody<D>, _dt: f64) {
        let delta = body_b.transform.translation.0 - body_a.transform.translation.0;
        let dist = delta.norm();
        if dist < 1e-15 {
            return;
        }

        let error = dist - self.rest_length;
        let correction = delta / dist * error * self.stiffness;

        let total_inv_mass = body_a.inv_mass + body_b.inv_mass;
        if total_inv_mass < 1e-15 {
            return;
        }

        let ratio_a = body_a.inv_mass / total_inv_mass;
        let ratio_b = body_b.inv_mass / total_inv_mass;

        if body_a.is_dynamic() {
            body_a.transform.translation.0 += correction * ratio_a;
        }
        if body_b.is_dynamic() {
            body_b.transform.translation.0 -= correction * ratio_b;
        }
    }

    fn solve_velocity(
        &self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        _dt: f64,
        _callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
    ) {
        let delta = body_b.transform.translation.0 - body_a.transform.translation.0;
        let dist = delta.norm();
        if dist < 1e-15 {
            return;
        }
        let normal = delta / dist;

        let rel_vel = body_b.linear_velocity - body_a.linear_velocity;
        let vel_along = rel_vel.dot(&normal);

        let total_inv = body_a.inv_mass + body_b.inv_mass;
        if total_inv < 1e-15 {
            return;
        }
        let impulse = -vel_along / total_inv * self.stiffness;

        let impulse_vec = normal * impulse;
        if body_a.is_dynamic() {
            body_a.linear_velocity -= impulse_vec * body_a.inv_mass;
        }
        if body_b.is_dynamic() {
            body_b.linear_velocity += impulse_vec * body_b.inv_mass;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;

    #[test]
    fn distance_constraint_corrects() {
        let mut a =
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([0.0, 0.0, 0.0]), 1.0, 1.0);
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([5.0, 0.0, 0.0]), 1.0, 1.0);

        let constraint = DistanceConstraint {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            rest_length: 3.0,
            stiffness: 1.0,
        };

        // Bodies are 5 apart, rest length is 3 — should pull them together
        for _ in 0..10 {
            constraint.solve(&mut a, &mut b, 0.01);
        }

        let dist = a.transform.translation.distance(&b.transform.translation);
        assert!((dist - 3.0).abs() < 0.1, "dist = {dist}, expected ~3.0");
    }
}


#[cfg(test)]
mod motor_step_state_tests {
    use super::MotorStepState;

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.15e}, expected {expected:.15e}"
        );
    }

    #[test]
    fn repeated_solver_iterations_share_one_impulse_and_work_budget() {
        for iterations in [1, 2, 4, 8] {
            let mut state = MotorStepState::default();
            let mut velocity = 0.0;
            for _ in 0..iterations {
                let target_velocity = 3.0;
                let desired_increment = (target_velocity - velocity) / 0.5;
                let (delta_impulse, velocity_after) = state
                    .apply_impulse_increment(desired_increment, 10.0, velocity, 0.5)
                    .expect("finite motor projection");
                velocity = velocity_after;
                if iterations == 1 {
                    close(delta_impulse, 6.0, 1.0e-12);
                }
            }

            close(state.accumulated_impulse(), 6.0, 1.0e-12);
            close(velocity, 3.0, 1.0e-12);
            close(state.take_work_receipt().expect("one work receipt"), 9.0, 1.0e-12);
            assert_eq!(state.take_work_receipt(), None);
        }
    }

    #[test]
    fn effort_saturation_is_a_single_step_bound() {
        let mut state = MotorStepState::default();
        let (delta, velocity) = state
            .project_impulse(100.0, 0.2, 0.0, 0.5)
            .expect("finite saturated projection");
        close(delta, 0.2, 1.0e-12);
        close(velocity, 0.1, 1.0e-12);
        for _ in 0..7 {
            let (additional, after) = state
                .project_impulse(100.0, 0.2, velocity, 0.5)
                .expect("finite saturated projection");
            close(additional, 0.0, 1.0e-12);
            close(after, velocity, 1.0e-12);
        }
        close(state.accumulated_impulse(), 0.2, 1.0e-12);
        close(state.take_work_receipt().expect("receipt"), 0.01, 1.0e-12);
    }

    #[test]
    fn invalid_inputs_do_not_mutate_the_step_state() {
        let mut state = MotorStepState::default();
        assert_eq!(state.project_impulse(1.0, 1.0, 0.0, 0.0), None);
        assert_eq!(state.project_impulse(f64::NAN, 1.0, 0.0, 1.0), None);
        assert_eq!(state.accumulated_impulse(), 0.0);
        assert_eq!(state.take_work_receipt(), None);
    }
}
