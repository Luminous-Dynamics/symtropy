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
    /// Number of bodies with non-finite state or physical parameters.
    pub non_finite_body_count: usize,
    /// Number of bodies with finite but invalid physical parameters.
    pub invalid_physical_parameter_count: usize,
    /// Number of contact manifolds/events with non-finite measured data.
    pub non_finite_contact_count: usize,
}

impl<const D: usize> InvariantSnapshot<D> {
    /// True only when all reported physical observables and body/contact
    /// diagnostics are finite, physical parameters are admissible, and every
    /// orientation satisfies the supplied proper-rotation tolerance.
    ///
    /// A finite position/velocity alone is insufficient: NaN mass, inertia,
    /// contact depth, or aggregate energy must fail closed as well.
    pub fn is_numerically_healthy(&self, rotation_tolerance: f64) -> bool {
        rotation_tolerance.is_finite()
            && rotation_tolerance >= 0.0
            && self.non_finite_body_count == 0
            && self.invalid_physical_parameter_count == 0
            && self.non_finite_contact_count == 0
            && self.dynamic_mass.is_finite()
            && self.dynamic_mass >= 0.0
            && self.center_of_mass.iter().all(|value| value.is_finite())
            && self.linear_momentum.iter().all(|value| value.is_finite())
            && self.kinetic_energy.is_finite()
            && self.gravitational_potential_energy.is_finite()
            && self.mechanical_energy.is_finite()
            && self.max_linear_speed.is_finite()
            && self.max_linear_speed >= 0.0
            && self.max_angular_speed.is_finite()
            && self.max_angular_speed >= 0.0
            && self.max_penetration_depth.is_finite()
            && self.max_penetration_depth >= 0.0
            && self.max_rotation_orthogonality_error.is_finite()
            && self.max_rotation_orthogonality_error <= rotation_tolerance
            && self.max_rotation_determinant_error.is_finite()
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
        let mut invalid_physical_parameter_count = 0;
        let mut non_finite_contact_count = 0;

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
                && body.force_accumulator.iter().all(|value| value.is_finite())
                && body.torque_accumulator.is_finite()
                && body.mass.is_finite()
                && body.inv_mass.is_finite()
                && body.inertia.iter().all(|value| value.is_finite())
                && body.inv_inertia.iter().all(|value| value.is_finite())
                && body.friction.is_finite()
                && body.restitution.is_finite()
                && body.linear_damping.is_finite()
                && body.angular_damping.is_finite()
                && rotation_orthogonality_error.is_finite()
                && rotation_determinant_error.is_finite();
            if !finite {
                non_finite_body_count += 1;
            }

            let parameters_valid = body.mass >= 0.0
                && body.inv_mass >= 0.0
                && body.inertia.iter().all(|value| *value >= 0.0)
                && body.inv_inertia.iter().all(|value| *value >= 0.0)
                && body.friction >= 0.0
                && body.linear_damping >= 0.0
                && body.angular_damping >= 0.0
                && (body.body_type != BodyType::Dynamic || body.mass > 0.0);
            if finite && !parameters_valid {
                invalid_physical_parameter_count += 1;
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
        if !self.gravity.iter().all(|value| value.is_finite()) {
            gravitational_potential_energy = f64::NAN;
        }
        let mechanical_energy = kinetic_energy + gravitational_potential_energy;
        let mut max_penetration_depth = 0.0_f64;
        for contact in &self.contacts {
            let finite = contact.normal.iter().all(|value| value.is_finite())
                && contact.points.iter().all(|point| {
                    point.position.iter().all(|value| value.is_finite())
                        && point.depth.is_finite()
                        && point.lambda.is_finite()
                        && point.restitution_bias.is_finite()
                })
                && contact.elasticity.is_none_or(f64::is_finite);
            if !finite {
                non_finite_contact_count += 1;
                max_penetration_depth = f64::NAN;
            } else if max_penetration_depth.is_finite() {
                max_penetration_depth = max_penetration_depth.max(contact.depth());
            }
        }
        for event in &self.collision_events {
            if !event.impulse.is_finite()
                || !event.depth.is_finite()
                || !event.normal.iter().all(|value| value.is_finite())
            {
                non_finite_contact_count += 1;
            }
        }

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
            invalid_physical_parameter_count,
            non_finite_contact_count,
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

    #[test]
    fn health_gate_rejects_non_finite_mass_and_inertia() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let mut body = RigidBody::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        body.mass = f64::INFINITY;
        body.inertia[0] = f64::NAN;
        world.add_body(body);

        let snapshot = world.invariant_snapshot();
        assert_eq!(snapshot.non_finite_body_count, 1);
        assert!(!snapshot.is_numerically_healthy(1e-9));
    }

    #[test]
    fn health_gate_rejects_finite_but_invalid_physical_parameters() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let mut body = RigidBody::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        body.mass = -1.0;
        body.linear_damping = -0.1;
        world.add_body(body);

        let snapshot = world.invariant_snapshot();
        assert_eq!(snapshot.invalid_physical_parameter_count, 1);
        assert!(!snapshot.is_numerically_healthy(1e-9));
    }

    #[test]
    fn health_gate_rejects_non_finite_contact_measurements() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let body_a = world.add_sphere(Point::origin(), 0.5, 1.0);
        let body_b = world.add_sphere(Point::new([0.9, 0.0, 0.0]), 0.5, 1.0);
        let mut contact = crate::contact::ContactManifold::single(
            body_a,
            body_b,
            SVector::from([1.0, 0.0, 0.0]),
            SVector::from([0.45, 0.0, 0.0]),
            0.1,
        );
        contact.points[0].depth = f64::NAN;
        world.contacts.push(contact);

        let snapshot = world.invariant_snapshot();
        assert_eq!(snapshot.non_finite_contact_count, 1);
        assert!(!snapshot.max_penetration_depth.is_finite());
        assert!(!snapshot.is_numerically_healthy(1e-9));
    }

    #[test]
    fn health_gate_rejects_non_finite_environment_gravity_even_for_empty_world() {
        let world = PhysicsWorld::<3>::new(SVector::from([0.0, f64::NAN, 0.0]));
        let snapshot = world.invariant_snapshot();

        assert!(!snapshot.gravitational_potential_energy.is_finite());
        assert!(!snapshot.is_numerically_healthy(1e-9));
    }

    #[test]
    fn health_gate_rejects_non_finite_tolerance_and_aggregate_energy() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.add_sphere(Point::origin(), 0.5, 1.0);
        let snapshot = world.invariant_snapshot();

        assert!(!snapshot.is_numerically_healthy(f64::NAN));
        assert!(!snapshot.is_numerically_healthy(-1.0));

        let mut corrupted = snapshot.clone();
        corrupted.mechanical_energy = f64::INFINITY;
        assert!(!corrupted.is_numerically_healthy(1e-9));
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
