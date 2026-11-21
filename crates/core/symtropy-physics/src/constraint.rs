// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Constraint trait for ND joints and connections.

use crate::body::{BodyHandle, RigidBody};

/// Observable disposition of one constraint solver invocation.
///
/// This reports execution only. It is not an independent physical-validity or
/// simulation-qualification verdict. Residual norms and units are constraint-specific.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ConstraintSolveOutcome {
    /// The constraint applied a correction. The optional residual is
    /// expressed in the constraint's declared residual norm and units.
    Applied { residual_inf: Option<f64> },
    /// The constraint had no correction to apply.
    NoOp,
    /// The constraint declined an operation without claiming its result.
    Rejected {
        code: &'static str,
        body_state_mutated_before_rejection: bool,
    },
    /// A legacy implementation ran through the compatibility method and did
    /// not provide a typed outcome.
    Unreported,
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

    /// Solve the positional constraint and return a typed execution outcome.
    ///
    /// The default adapter preserves source compatibility: it invokes the
    /// existing method exactly once and labels the outcome Unreported. It does
    /// not infer NoOp or Applied from a unit return.
    fn solve_with_outcome(
        &self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        dt: f64,
    ) -> ConstraintSolveOutcome {
        self.solve(body_a, body_b, dt);
        ConstraintSolveOutcome::Unreported
    }

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

    /// Solve velocity constraints and return a typed execution outcome.
    ///
    /// The default adapter preserves source compatibility: it invokes the
    /// existing method exactly once and labels the outcome Unreported. It does
    /// not infer NoOp or Applied from the absence of an error.
    fn solve_velocity_with_outcome(
        &self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        dt: f64,
        callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
    ) -> ConstraintSolveOutcome {
        self.solve_velocity(body_a, body_b, dt, callback);
        ConstraintSolveOutcome::Unreported
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
