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
mod analytic_constraint_qualification_tests {
    use super::*;
    use symtropy_math::Point;

    #[test]
    fn distance_constraint_residual_follows_analytic_stiffness_convergence() {
        let mut a = RigidBody::<3>::dynamic_sphere(
            BodyHandle(0),
            Point::new([0.0, 0.0, 0.0]),
            0.25,
            1.0,
        );
        let mut b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::new([10.0, 0.0, 0.0]),
            0.25,
            1.0,
        );
        let rest_length = 4.0;
        let stiffness = 0.25;
        let initial_error: f64 = 10.0 - rest_length;
        let constraint = DistanceConstraint {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            rest_length,
            stiffness,
        };

        for iterations in 1..=16 {
            constraint.solve(&mut a, &mut b, 1.0 / 60.0);
            let actual_error =
                a.transform.translation.distance(&b.transform.translation) - rest_length;
            let expected_error = initial_error * (1.0 - stiffness).powi(iterations);
            assert!(
                (actual_error - expected_error).abs() < 1e-10,
                "iteration {iterations}: residual {actual_error}, expected {expected_error}"
            );
        }
    }

    #[test]
    fn inverse_mass_weighting_preserves_center_of_mass_for_unequal_masses() {
        let mass_a = 1.0;
        let mass_b = 3.0;
        let mut a = RigidBody::<3>::dynamic_sphere(
            BodyHandle(0),
            Point::new([0.0, 0.0, 0.0]),
            0.25,
            mass_a,
        );
        let mut b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::new([10.0, 0.0, 0.0]),
            0.25,
            mass_b,
        );
        let center_before =
            (a.transform.translation.0 * mass_a + b.transform.translation.0 * mass_b)
                / (mass_a + mass_b);

        let constraint = DistanceConstraint {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            rest_length: 4.0,
            stiffness: 0.5,
        };
        for _ in 0..8 {
            constraint.solve(&mut a, &mut b, 1.0 / 60.0);
        }

        let center_after =
            (a.transform.translation.0 * mass_a + b.transform.translation.0 * mass_b)
                / (mass_a + mass_b);
        assert!((center_after - center_before).norm() < 1e-12);
        assert!(
            (a.transform.translation.distance(&b.transform.translation) - 4.0).abs()
                < 1e-3
        );
    }
}
