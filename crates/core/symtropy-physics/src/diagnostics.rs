// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Quantitative diagnostics for scientific validation and reproducible runs.
//!
//! These measurements deliberately avoid deciding whether a scenario *should*
//! conserve a quantity. A closed world with no gravity or static contacts may
//! conserve linear momentum; a body bouncing from an immovable floor will not
//! conserve the dynamic bodies' momentum because the floor is an external
//! reservoir. Research harnesses must state the assumptions of each scenario.

use nalgebra::SVector;
use serde::{Deserialize, Serialize};

use crate::body::BodyType;
use crate::world::PhysicsWorld;

/// A quantitative snapshot of world-level physical invariants and health.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvariantSnapshot<const D: usize> {
    /// Sum of masses for dynamic bodies.
    pub dynamic_mass: f64,
    /// Mass-weighted center of mass for dynamic bodies.
    pub center_of_mass: SVector<f64, D>,
    /// Sum of `m v` for dynamic bodies.
    pub linear_momentum: SVector<f64, D>,
    /// Translational plus currently-modelled rotational kinetic energy.
    pub kinetic_energy: f64,
    /// Potential energy for the world's uniform gravity field: `-m g·x`.
    pub gravitational_potential_energy: f64,
    /// Kinetic plus gravitational potential energy.
    pub mechanical_energy: f64,
    /// Largest linear speed among dynamic bodies.
    pub max_linear_speed: f64,
    /// Largest bivector angular-speed norm among dynamic bodies.
    pub max_angular_speed: f64,
    /// Largest `max_abs(RᵀR - I)` over all bodies.
    pub max_rotation_orthogonality_error: f64,
    /// Largest `|det(R) - 1|` over all bodies.
    pub max_rotation_determinant_error: f64,
    /// Deepest contact reported for the current step.
    pub max_penetration_depth: f64,
    /// Number of bodies whose state contains a NaN or infinity.
    pub non_finite_body_count: usize,
}

impl<const D: usize> InvariantSnapshot<D> {
    /// True when all body state was finite and every orientation satisfied the
    /// supplied proper-rotation tolerance.
    pub fn is_numerically_healthy(&self, rotation_tolerance: f64) -> bool {
        self.non_finite_body_count == 0
            && self.max_rotation_orthogonality_error <= rotation_tolerance
            && self.max_rotation_determinant_error <= rotation_tolerance
    }

    /// Compare this snapshot with a later snapshot.
    pub fn drift_to(&self, later: &Self) -> InvariantDrift<D> {
        let absolute_energy_drift = later.mechanical_energy - self.mechanical_energy;
        let energy_scale = self.mechanical_energy.abs().max(1e-12);
        InvariantDrift {
            center_of_mass_delta: later.center_of_mass - self.center_of_mass,
            linear_momentum_delta: later.linear_momentum - self.linear_momentum,
            absolute_energy_drift,
            relative_energy_drift: absolute_energy_drift / energy_scale,
            max_rotation_orthogonality_error: later.max_rotation_orthogonality_error,
            max_rotation_determinant_error: later.max_rotation_determinant_error,
            non_finite_body_count: later.non_finite_body_count,
        }
    }
}

/// Drift between two invariant snapshots.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvariantDrift<const D: usize> {
    pub center_of_mass_delta: SVector<f64, D>,
    pub linear_momentum_delta: SVector<f64, D>,
    pub absolute_energy_drift: f64,
    pub relative_energy_drift: f64,
    pub max_rotation_orthogonality_error: f64,
    pub max_rotation_determinant_error: f64,
    pub non_finite_body_count: usize,
}

impl<const D: usize> InvariantDrift<D> {
    pub fn linear_momentum_error_norm(&self) -> f64 {
        self.linear_momentum_delta.norm()
    }

    pub fn center_of_mass_drift_norm(&self) -> f64 {
        self.center_of_mass_delta.norm()
    }
}

impl<const D: usize> PhysicsWorld<D> {
    /// Capture physical invariants and numerical-health metrics.
    ///
    /// The snapshot is deterministic for a fixed world state and contains no
    /// wall-clock data, host identifiers, or allocation-order-dependent fields.
    pub fn invariant_snapshot(&self) -> InvariantSnapshot<D> {
        let mut dynamic_mass = 0.0;
        let mut weighted_position = SVector::<f64, D>::zeros();
        let mut linear_momentum = SVector::<f64, D>::zeros();
        let mut kinetic_energy = 0.0;
        let mut gravitational_potential_energy = 0.0;
        let mut max_linear_speed = 0.0_f64;
        let mut max_angular_speed = 0.0_f64;
        let mut max_rotation_orthogonality_error = 0.0_f64;
        let mut max_rotation_determinant_error = 0.0_f64;
        let mut non_finite_body_count = 0;

        for body in &self.bodies {
            let position = body.transform.translation.0;
            let rotation_orthogonality_error = body.transform.rotation.orthogonality_error();
            let rotation_determinant_error = (body.transform.rotation.determinant() - 1.0).abs();
            max_rotation_orthogonality_error =
                max_rotation_orthogonality_error.max(rotation_orthogonality_error);
            max_rotation_determinant_error =
                max_rotation_determinant_error.max(rotation_determinant_error);

            let finite = position.iter().all(|value| value.is_finite())
                && body.linear_velocity.iter().all(|value| value.is_finite())
                && body.angular_velocity.is_finite()
                && rotation_orthogonality_error.is_finite()
                && rotation_determinant_error.is_finite();
            if !finite {
                non_finite_body_count += 1;
            }

            if body.body_type != BodyType::Dynamic {
                continue;
            }

            dynamic_mass += body.mass;
            weighted_position += position * body.mass;
            linear_momentum += body.linear_velocity * body.mass;
            kinetic_energy += body.kinetic_energy();
            gravitational_potential_energy -= body.mass * self.gravity.dot(&position);
            max_linear_speed = max_linear_speed.max(body.linear_velocity.norm());
            max_angular_speed = max_angular_speed.max(body.angular_velocity.norm());
        }

        let center_of_mass = if dynamic_mass > 0.0 {
            weighted_position / dynamic_mass
        } else {
            SVector::zeros()
        };
        let mechanical_energy = kinetic_energy + gravitational_potential_energy;
        let max_penetration_depth = self
            .contacts
            .iter()
            .map(|contact| contact.depth())
            .fold(0.0_f64, f64::max);

        InvariantSnapshot {
            dynamic_mass,
            center_of_mass,
            linear_momentum,
            kinetic_energy,
            gravitational_potential_energy,
            mechanical_energy,
            max_linear_speed,
            max_angular_speed,
            max_rotation_orthogonality_error,
            max_rotation_determinant_error,
            max_penetration_depth,
            non_finite_body_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BodyHandle, RigidBody};
    use nalgebra::SVector;
    use symtropy_math::Point;

    #[test]
    fn snapshot_reports_mass_center_and_momentum() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let a = world.add_sphere(Point::new([-1.0, 0.0, 0.0]), 0.5, 2.0);
        let b = world.add_sphere(Point::new([3.0, 0.0, 0.0]), 0.5, 1.0);
        world.body_mut(a).unwrap().linear_velocity = SVector::from([2.0, 0.0, 0.0]);
        world.body_mut(b).unwrap().linear_velocity = SVector::from([-1.0, 0.0, 0.0]);

        let snapshot = world.invariant_snapshot();
        assert!((snapshot.dynamic_mass - 3.0).abs() < 1e-12);
        assert!((snapshot.center_of_mass[0] - 1.0 / 3.0).abs() < 1e-12);
        assert!((snapshot.linear_momentum[0] - 3.0).abs() < 1e-12);
        assert!(snapshot.is_numerically_healthy(1e-12));
    }

    #[test]
    fn gravity_potential_uses_negative_m_g_dot_x() {
        let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
        world.add_sphere(Point::new([0.0, 10.0, 0.0]), 0.5, 2.0);
        let snapshot = world.invariant_snapshot();
        assert!((snapshot.gravitational_potential_energy - 196.2).abs() < 1e-10);
    }

    #[test]
    fn drift_report_is_signed_and_normed() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        let before = world.invariant_snapshot();
        world.body_mut(handle).unwrap().linear_velocity = SVector::from([3.0, 4.0]);
        let after = world.invariant_snapshot();
        let drift = before.drift_to(&after);
        assert!((drift.linear_momentum_error_norm() - 5.0).abs() < 1e-12);
        assert!((drift.absolute_energy_drift - 12.5).abs() < 1e-12);
    }

    fn free_fall_error(dt: f64) -> (f64, f64, f64) {
        const GRAVITY: f64 = -9.81;
        const INITIAL_HEIGHT: f64 = 100.0;
        const INITIAL_VELOCITY: f64 = 3.0;
        const DURATION: f64 = 1.0;

        let steps = (DURATION / dt).round() as usize;
        assert!((steps as f64 * dt - DURATION).abs() < 1e-12);

        let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, GRAVITY, 0.0]));
        let handle = world.add_sphere(Point::new([0.0, INITIAL_HEIGHT, 0.0]), 0.25, 2.0);
        let body = world.body_mut(handle).unwrap();
        body.linear_velocity = SVector::from([0.0, INITIAL_VELOCITY, 0.0]);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;

        for _ in 0..steps {
            world.step(dt);
        }

        let body = &world.bodies[0];
        let analytic_position =
            INITIAL_HEIGHT + INITIAL_VELOCITY * DURATION + 0.5 * GRAVITY * DURATION * DURATION;
        let analytic_velocity = INITIAL_VELOCITY + GRAVITY * DURATION;
        let actual_position = body.transform.translation.0[1];
        let actual_velocity = body.linear_velocity[1];
        let position_error = (actual_position - analytic_position).abs();
        let velocity_error = (actual_velocity - analytic_velocity).abs();

        let health = world.invariant_snapshot();
        assert_eq!(health.non_finite_body_count, 0);
        assert!(health.is_numerically_healthy(1e-9));

        (position_error, velocity_error, actual_position)
    }

    #[test]
    fn free_fall_matches_analytic_velocity_and_first_order_position_convergence() {
        let (coarse_error, coarse_velocity_error, _) = free_fall_error(0.04);
        let (medium_error, medium_velocity_error, _) = free_fall_error(0.02);
        let (fine_error, fine_velocity_error, _) = free_fall_error(0.01);

        // Semi-implicit Euler integrates constant acceleration exactly in
        // velocity. Its position update uses v_(n+1), so the global position
        // error against the continuous trajectory is |g| * T * dt / 2.
        assert!(coarse_velocity_error < 1e-10);
        assert!(medium_velocity_error < 1e-10);
        assert!(fine_velocity_error < 1e-10);

        let coarse_medium_ratio = coarse_error / medium_error;
        let medium_fine_ratio = medium_error / fine_error;
        assert!(
            (coarse_medium_ratio - 2.0).abs() < 1e-8,
            "position error should halve with dt: ratio={coarse_medium_ratio}"
        );
        assert!(
            (medium_fine_ratio - 2.0).abs() < 1e-8,
            "position error should halve with dt: ratio={medium_fine_ratio}"
        );

        let expected_coarse_error = 0.5 * 9.81 * 1.0 * 0.04;
        assert!((coarse_error - expected_coarse_error).abs() < 1e-9);
    }

    #[test]
    fn detects_non_finite_body_state() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let mut body = RigidBody::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        body.linear_velocity[1] = f64::NAN;
        world.add_body(body);
        let snapshot = world.invariant_snapshot();
        assert_eq!(snapshot.non_finite_body_count, 1);
        assert!(!snapshot.is_numerically_healthy(1e-9));
    }
}
