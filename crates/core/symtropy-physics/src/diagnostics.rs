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
    /// Modeled sensible thermal energy relative to absolute zero for valid
    /// body-attached thermal reservoirs. If `invalid_thermal_body_count > 0`,
    /// this sum is incomplete and must not be used as a conservation total.
    pub modeled_thermal_energy: f64,
    /// Mechanical energy plus the currently accounted sensible thermal energy.
    /// This is a complete modeled total only when `invalid_thermal_body_count == 0`.
    pub modeled_total_energy: f64,
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
    /// Number of body-attached thermal reservoirs that fail physical validation.
    pub invalid_thermal_body_count: usize,
}

impl<const D: usize> InvariantSnapshot<D> {
    /// True when all body state was finite, all thermal reservoirs were physically
    /// well formed, and every orientation satisfied the supplied proper-rotation tolerance.
    pub fn is_numerically_healthy(&self, rotation_tolerance: f64) -> bool {
        self.non_finite_body_count == 0
            && self.invalid_thermal_body_count == 0
            && self.max_rotation_orthogonality_error <= rotation_tolerance
            && self.max_rotation_determinant_error <= rotation_tolerance
    }

    /// True when every attached thermal reservoir participated in the modeled energy total.
    pub fn has_complete_modeled_energy_accounting(&self) -> bool {
        self.invalid_thermal_body_count == 0
    }

    /// Compare this snapshot with a later snapshot.
    pub fn drift_to(&self, later: &Self) -> InvariantDrift<D> {
        let absolute_energy_drift = later.mechanical_energy - self.mechanical_energy;
        let energy_scale = self.mechanical_energy.abs().max(1e-12);
        let absolute_modeled_total_energy_drift =
            later.modeled_total_energy - self.modeled_total_energy;
        let modeled_total_energy_scale = self.modeled_total_energy.abs().max(1e-12);
        InvariantDrift {
            center_of_mass_delta: later.center_of_mass - self.center_of_mass,
            linear_momentum_delta: later.linear_momentum - self.linear_momentum,
            absolute_energy_drift,
            relative_energy_drift: absolute_energy_drift / energy_scale,
            absolute_modeled_total_energy_drift,
            relative_modeled_total_energy_drift: absolute_modeled_total_energy_drift
                / modeled_total_energy_scale,
            max_rotation_orthogonality_error: later.max_rotation_orthogonality_error,
            max_rotation_determinant_error: later.max_rotation_determinant_error,
            non_finite_body_count: later.non_finite_body_count,
            invalid_thermal_body_count: later.invalid_thermal_body_count,
        }
    }
}

/// Drift between two invariant snapshots.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvariantDrift<const D: usize> {
    pub center_of_mass_delta: SVector<f64, D>,
    pub linear_momentum_delta: SVector<f64, D>,
    /// Existing mechanical-energy drift metric.
    pub absolute_energy_drift: f64,
    pub relative_energy_drift: f64,
    /// Drift in the combined mechanical + modeled sensible thermal budget.
    pub absolute_modeled_total_energy_drift: f64,
    pub relative_modeled_total_energy_drift: f64,
    pub max_rotation_orthogonality_error: f64,
    pub max_rotation_determinant_error: f64,
    pub non_finite_body_count: usize,
    pub invalid_thermal_body_count: usize,
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
        let mut modeled_thermal_energy = 0.0;
        let mut max_linear_speed = 0.0_f64;
        let mut max_angular_speed = 0.0_f64;
        let mut max_rotation_orthogonality_error = 0.0_f64;
        let mut max_rotation_determinant_error = 0.0_f64;
        let mut non_finite_body_count = 0;
        let mut invalid_thermal_body_count = 0;

        for body in &self.bodies {
            let position = body.transform.translation.0;
            let rotation_orthogonality_error = body.transform.rotation.orthogonality_error();
            let rotation_determinant_error = (body.transform.rotation.determinant() - 1.0).abs();
            max_rotation_orthogonality_error =
                max_rotation_orthogonality_error.max(rotation_orthogonality_error);
            max_rotation_determinant_error =
                max_rotation_determinant_error.max(rotation_determinant_error);

            let thermal_is_finite = body.thermal.is_none_or(|thermal| {
                thermal.state.temperature_kelvin.is_finite()
                    && thermal.thermal_mass_kg.is_finite()
                    && thermal.material.specific_heat_capacity.is_finite()
                    && thermal.material.thermal_conductivity.is_finite()
                    && thermal.material.emissivity.is_finite()
            });
            let finite = position.iter().all(|value| value.is_finite())
                && body.linear_velocity.iter().all(|value| value.is_finite())
                && body.angular_velocity.is_finite()
                && rotation_orthogonality_error.is_finite()
                && rotation_determinant_error.is_finite()
                && thermal_is_finite;
            if !finite {
                non_finite_body_count += 1;
            }

            if let Some(thermal) = body.thermal {
                match thermal.sensible_energy_joules(0.0) {
                    Ok(energy) => modeled_thermal_energy += energy,
                    Err(_) => invalid_thermal_body_count += 1,
                }
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
        let modeled_total_energy = mechanical_energy + modeled_thermal_energy;
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
            modeled_thermal_energy,
            modeled_total_energy,
            max_linear_speed,
            max_angular_speed,
            max_rotation_orthogonality_error,
            max_rotation_determinant_error,
            max_penetration_depth,
            non_finite_body_count,
            invalid_thermal_body_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::{BodyHandle, RigidBody};
    use crate::thermal::{ThermalBody, ThermalMaterial, ThermalState};
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
    fn snapshot_includes_modeled_thermal_energy() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 2.0);
        let material = ThermalMaterial::new(500.0, 1.0, 0.5).unwrap();
        let thermal = ThermalBody::new(
            material,
            ThermalState::new(300.0).unwrap(),
            2.0,
        )
        .unwrap();
        world.body_mut(handle).unwrap().set_thermal(thermal);

        let snapshot = world.invariant_snapshot();
        assert!(snapshot.has_complete_modeled_energy_accounting());
        assert_eq!(snapshot.invalid_thermal_body_count, 0);
        assert!((snapshot.modeled_thermal_energy - 300_000.0).abs() < 1e-9);
        assert!((snapshot.modeled_total_energy - snapshot.mechanical_energy - 300_000.0).abs() < 1e-9);
    }

    #[test]
    fn invalid_thermal_reservoir_is_not_silently_omitted() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 2.0);
        let material = ThermalMaterial::new(500.0, 1.0, 0.5).unwrap();
        let mut thermal = ThermalBody::new(
            material,
            ThermalState::new(300.0).unwrap(),
            2.0,
        )
        .unwrap();
        thermal.state.temperature_kelvin = -1.0;
        world.body_mut(handle).unwrap().set_thermal(thermal);

        let snapshot = world.invariant_snapshot();
        assert_eq!(snapshot.non_finite_body_count, 0);
        assert_eq!(snapshot.invalid_thermal_body_count, 1);
        assert!(!snapshot.has_complete_modeled_energy_accounting());
        assert!(!snapshot.is_numerically_healthy(1e-12));
        assert_eq!(snapshot.modeled_thermal_energy, 0.0);
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
        assert!((drift.absolute_modeled_total_energy_drift - 12.5).abs() < 1e-12);
        assert_eq!(drift.invalid_thermal_body_count, 0);
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
