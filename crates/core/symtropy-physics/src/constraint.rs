// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Constraint trait for ND joints and connections.

use crate::body::{BodyHandle, RigidBody};

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

    /// Apply actuator effort exactly once for the current world-step interval.
    ///
    /// This hook is called once per active constraint after positional
    /// projection and before the iterative velocity-level constraint solver.
    /// It is deliberately separate from solve_velocity, which may be invoked
    /// multiple times as a numerical relaxation procedure.
    ///
    /// Actuated constraints should apply a single bounded impulse for dt and
    /// record its mechanical work once. The default is a no-op for passive
    /// constraints.
    fn solve_actuator(
        &self,
        _body_a: &mut RigidBody<D>,
        _body_b: &mut RigidBody<D>,
        _dt: f64,
        _callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
    ) {
    }

    /// Optional iterative velocity-level constraint correction.
    ///
    /// Called repeatedly after the once-per-step actuator hook. Implementors
    /// must treat this as solver relaxation, not as another elapsed-time
    /// interval; do not apply a fresh full-frame motor impulse here.
    /// Default: no-op (pure PBD).
    fn solve_velocity(
        &self,
        _body_a: &mut RigidBody<D>,
        _body_b: &mut RigidBody<D>,
        _dt: f64,
        _callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
    ) {
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
