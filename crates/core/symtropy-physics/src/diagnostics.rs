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
use crate::world::{NoOpCallback, PhysicsCallback, PhysicsWorld, StepWorkAccumulator};

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
    /// Number of bodies whose known inertia mass disagrees with translational mass.
    #[serde(default)]
    pub mass_property_mismatch_count: usize,
}

impl<const D: usize> InvariantSnapshot<D> {
    /// True when all body state was finite and every orientation satisfied the
    /// supplied proper-rotation tolerance.
    pub fn is_numerically_healthy(&self, rotation_tolerance: f64) -> bool {
        self.non_finite_body_count == 0
            && self.mass_property_mismatch_count == 0
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
            mass_property_mismatch_count: later.mass_property_mismatch_count,
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
    /// Number of bodies whose known inertia mass disagrees with translational mass.
    #[serde(default)]
    pub mass_property_mismatch_count: usize,
}

/// Energy channels not yet fully attributed by the current discrete ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnmodeledEnergyChannel {
    RotationalTorqueAndAngularDamping,
    JointAndMotorWork,
    ConstraintStabilizationAndCompliance,
    CcdAndPositionCorrection,
    PrescribedKinematicAndStaticBoundaryWork,
    ConstitutiveMaterialWork,
    FloatingPointAndSolverTermination,
}

/// Signed discrete work and energy accounting for one accepted step.
///
/// The attributed terms cover translational force/gravity updates and normal
/// and friction contact impulses. The unattributed term deliberately retains
/// energy changes from rotational torque/damping, constraints, motors,
/// stabilization and any other source not yet represented by this ledger.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiscreteWorkLedger {
    pub external_force_work_joules: f64,
    pub gravity_work_joules: f64,
    pub linear_damping_and_speed_limit_energy_change_joules: f64,
    pub normal_contact_kinetic_energy_change_joules: f64,
    pub friction_contact_kinetic_energy_change_joules: f64,
    pub frictional_dissipation_joules: f64,
    pub gravitational_potential_energy_change_joules: f64,
    pub measured_mechanical_energy_change_joules: f64,
    pub unattributed_energy_change_joules: f64,
    /// Explicitly disclosed channels not yet included in the attribution model.
    #[serde(default)]
    pub unmodeled_energy_channels: Vec<UnmodeledEnergyChannel>,
}

impl DiscreteWorkLedger {
    /// Unit-invariant unexplained-energy ratio for this receipt.
    ///
    /// The denominator is the sum of absolute measured/attributed energy
    /// magnitudes, rather than a fixed 1 joule floor, so equivalent unit
    /// conversions preserve the ratio.
    pub fn relative_unattributed_residual(&self) -> f64 {
        let scale = self.measured_mechanical_energy_change_joules.abs()
            + self.external_force_work_joules.abs()
            + self.gravity_work_joules.abs()
            + self.linear_damping_and_speed_limit_energy_change_joules.abs()
            + self.normal_contact_kinetic_energy_change_joules.abs()
            + self.friction_contact_kinetic_energy_change_joules.abs()
            + self.gravitational_potential_energy_change_joules.abs();
        if scale <= f64::MIN_POSITIVE {
            0.0
        } else {
            self.unattributed_energy_change_joules.abs() / scale
        }
    }

    /// Whether the currently attributed terms close within a unit-invariant tolerance.
    ///
    /// This does not certify the physical model as complete; inspect
    /// `unmodeled_energy_channels` before making a conservation claim.
    pub fn closes_within(&self, relative_tolerance: f64) -> bool {
        relative_tolerance.is_finite()
            && relative_tolerance >= 0.0
            && self.relative_unattributed_residual() <= relative_tolerance
    }
}

/// Deterministic observation of one successfully committed world step.
///
/// This records the state transition and invariant drift, but is not a
/// conservation certificate: callers must account for applied forces,
/// contacts, motors and other work sources before judging energy balance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InvariantStepReceipt<const D: usize> {
    pub dt: f64,
    pub total_body_count: usize,
    pub dynamic_body_count: usize,
    pub contact_count: usize,
    pub max_penetration_depth: f64,
    pub before: InvariantSnapshot<D>,
    pub after: InvariantSnapshot<D>,
    pub drift: InvariantDrift<D>,
    #[serde(default)]
    pub work_ledger: DiscreteWorkLedger,
}

impl<const D: usize> InvariantStepReceipt<D> {
    /// True when the post-step state contains no non-finite bodies and all
    /// rotations satisfy the given proper-rotation tolerance.
    pub fn is_numerically_healthy(&self, rotation_tolerance: f64) -> bool {
        self.after.is_numerically_healthy(rotation_tolerance)
    }

    pub fn mechanical_energy_delta(&self) -> f64 {
        self.after.mechanical_energy - self.before.mechanical_energy
    }
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
    /// Step the world and return deterministic before/after invariant evidence.
    ///
    /// If the physical step fails, this returns the same typed error as
    /// `try_step` and the world's transactional rollback remains in effect.
    /// The receipt describes the committed state transition only; it does not
    /// attribute every external or constraint work source.
    pub fn try_step_with_receipt(
        &mut self,
        dt: f64,
    ) -> Result<InvariantStepReceipt<D>, crate::world::PhysicsStepError> {
        let mut noop = NoOpCallback;
        self.try_step_with_callback_and_receipt(dt, &mut noop)
    }

    /// Step with the supplied callback and return the committed work ledger.
    ///
    /// Work is calculated from the callback-modulated force and the actually
    /// applied contact impulses. Mutable callback effects are still published
    /// only after the world transaction commits.
    pub fn try_step_with_callback_and_receipt(
        &mut self,
        dt: f64,
        callback: &mut dyn PhysicsCallback<D>,
    ) -> Result<InvariantStepReceipt<D>, crate::world::PhysicsStepError> {
        let before = self.invariant_snapshot();
        let total_body_count = self.bodies.len();
        let dynamic_body_count = self
            .bodies
            .iter()
            .filter(|body| body.body_type == BodyType::Dynamic)
            .count();

        let mut work = StepWorkAccumulator::default();
        self.try_step_with_callback_and_work(dt, callback, &mut work)?;

        let after = self.invariant_snapshot();
        let drift = before.drift_to(&after);
        let measured_energy_change = after.mechanical_energy - before.mechanical_energy;
        let potential_energy_change =
            after.gravitational_potential_energy - before.gravitational_potential_energy;
        let frictional_dissipation = work.frictional_dissipation_joules;
        let attributed_energy_change = work.external_force_work_joules
            + work.gravity_work_joules
            + work.linear_damping_and_speed_limit_energy_change_joules
            + work.normal_contact_kinetic_energy_change_joules
            + work.friction_contact_kinetic_energy_change_joules
            + potential_energy_change;
        let work_ledger = DiscreteWorkLedger {
            external_force_work_joules: work.external_force_work_joules,
            gravity_work_joules: work.gravity_work_joules,
            linear_damping_and_speed_limit_energy_change_joules: work
                .linear_damping_and_speed_limit_energy_change_joules,
            normal_contact_kinetic_energy_change_joules: work
                .normal_contact_kinetic_energy_change_joules,
            friction_contact_kinetic_energy_change_joules: work
                .friction_contact_kinetic_energy_change_joules,
            frictional_dissipation_joules: frictional_dissipation,
            gravitational_potential_energy_change_joules: potential_energy_change,
            measured_mechanical_energy_change_joules: measured_energy_change,
            unattributed_energy_change_joules: measured_energy_change - attributed_energy_change,
            unmodeled_energy_channels: vec![
                UnmodeledEnergyChannel::RotationalTorqueAndAngularDamping,
                UnmodeledEnergyChannel::JointAndMotorWork,
                UnmodeledEnergyChannel::ConstraintStabilizationAndCompliance,
                UnmodeledEnergyChannel::CcdAndPositionCorrection,
                UnmodeledEnergyChannel::PrescribedKinematicAndStaticBoundaryWork,
                UnmodeledEnergyChannel::ConstitutiveMaterialWork,
                UnmodeledEnergyChannel::FloatingPointAndSolverTermination,
            ],
        };
        Ok(InvariantStepReceipt {
            dt,
            total_body_count,
            dynamic_body_count,
            contact_count: self.contacts.len(),
            max_penetration_depth: after.max_penetration_depth,
            before,
            after,
            drift,
            work_ledger,
        })
    }

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
        let mut mass_property_mismatch_count = 0;

        for body in &self.bodies {
            let position = body.transform.translation.0;
            let rotation_orthogonality_error = body.transform.rotation.orthogonality_error();
            let rotation_determinant_error = (body.transform.rotation.determinant() - 1.0).abs();
            max_rotation_orthogonality_error =
                max_rotation_orthogonality_error.max(rotation_orthogonality_error);
            max_rotation_determinant_error =
                max_rotation_determinant_error.max(rotation_determinant_error);

            if body
                .rotational_inertia
                .as_ref()
                .is_some_and(|inertia| inertia.validate_body_mass(body.mass, 1e-10).is_err())
            {
                mass_property_mismatch_count += 1;
            }

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
            mass_property_mismatch_count,
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
    fn snapshot_detects_known_rotational_inertia_mass_mismatch() {
        use crate::inertia::BivectorInertia;

        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world
            .body_mut(handle)
            .unwrap()
            .set_rotational_inertia_operator(
                BivectorInertia::<3>::uniform_solid_ellipsoid(2.0, SVector::new(1.0, 0.8, 0.6))
                    .unwrap(),
            );

        let snapshot = world.invariant_snapshot();
        assert_eq!(snapshot.mass_property_mismatch_count, 1);
        assert!(!snapshot.is_numerically_healthy(1e-12));
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
    fn detects_non_finite_body_state() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let mut body = RigidBody::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        body.linear_velocity[1] = f64::NAN;
        world.add_body(body);
        let snapshot = world.invariant_snapshot();
        assert_eq!(snapshot.non_finite_body_count, 1);
        assert!(!snapshot.is_numerically_healthy(1e-9));
    }

    #[test]
    fn accepted_step_receipt_captures_deterministic_invariant_transition() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        world.body_mut(handle).unwrap().linear_velocity = SVector::from([2.0, 0.0]);

        let receipt = world.try_step_with_receipt(0.01).unwrap();
        assert_eq!(receipt.dt, 0.01);
        assert_eq!(receipt.total_body_count, 1);
        assert_eq!(receipt.dynamic_body_count, 1);
        assert_eq!(receipt.contact_count, 0);
        assert!(receipt.is_numerically_healthy(1e-12));
        assert!(
            (receipt.after.center_of_mass[0] - receipt.before.center_of_mass[0] - 0.02).abs()
                < 1e-12
        );
        assert!(receipt.mechanical_energy_delta().abs() < 1e-12);
        assert!(receipt.drift.linear_momentum_error_norm() < 1e-12);
    }

    #[test]
    fn discrete_work_ledger_closes_force_and_gravity_energy_balance() {
        let mut world = PhysicsWorld::<2>::new(SVector::new(0.0, -9.81));
        let handle = world.add_sphere(Point::origin(), 0.1, 1.0);
        let body = world.body_mut(handle).unwrap();
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        body.apply_force(SVector::new(2.0, 0.0));

        let receipt = world.try_step_with_receipt(0.01).unwrap();
        let ledger = &receipt.work_ledger;

        assert!((ledger.external_force_work_joules - 0.0002).abs() < 1e-12);
        assert!((ledger.gravity_work_joules - 0.004811805).abs() < 1e-12);
        assert!(ledger.linear_damping_and_speed_limit_energy_change_joules.abs() < 1e-12);
        assert!(ledger.normal_contact_kinetic_energy_change_joules.abs() < 1e-12);
        assert!(ledger.friction_contact_kinetic_energy_change_joules.abs() < 1e-12);
        assert!((ledger.gravitational_potential_energy_change_joules + 0.00962361).abs() < 1e-12);
        assert!((ledger.measured_mechanical_energy_change_joules + 0.004611805).abs() < 1e-12);
        assert!(ledger.unattributed_energy_change_joules.abs() < 1e-12);
        assert!(ledger.closes_within(1e-12));
        assert_eq!(ledger.unmodeled_energy_channels.len(), 7);
    }

    #[test]
    fn discrete_work_ledger_attributes_linear_damping_energy_loss() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.1, 1.0);
        let body = world.body_mut(handle).unwrap();
        body.linear_velocity = SVector::new(1.0, 0.0);
        body.linear_damping = 3.0;
        body.angular_damping = 0.0;
        body.apply_force(SVector::new(2.0, 0.0));

        let receipt = world.try_step_with_receipt(0.1).unwrap();
        let ledger = &receipt.work_ledger;
        let raw_velocity = 1.2_f64;
        let final_velocity = raw_velocity * (-0.3_f64).exp();
        let expected_damping_delta =
            0.5 * (final_velocity * final_velocity - raw_velocity * raw_velocity);
        let expected_mechanical_delta = 0.5 * (final_velocity * final_velocity - 1.0);

        assert!((ledger.external_force_work_joules - 0.22).abs() < 1e-12);
        assert!(
            (ledger.linear_damping_and_speed_limit_energy_change_joules
                - expected_damping_delta)
                .abs()
                < 1e-12
        );
        assert!((ledger.measured_mechanical_energy_change_joules - expected_mechanical_delta).abs() < 1e-12);
        assert!(ledger.unattributed_energy_change_joules.abs() < 1e-12);
        assert!(ledger.closes_within(1e-12));
    }

    #[test]
    fn discrete_work_ledger_separates_normal_impact_and_friction() {
        use symtropy_math::Sphere;

        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.solver_iterations = 4;
        world.baumgarte = 0.0;
        world.compliance = 0.0;

        let moving = world.add_sphere(Point::origin(), 0.5, 1.0);
        {
            let body = world.body_mut(moving).unwrap();
            body.linear_velocity = SVector::new(1.0, 1.0, 0.0);
            body.friction = 0.5;
            body.restitution = 0.2;
            body.linear_damping = 0.0;
            body.angular_damping = 0.0;
        }
        let mut fixed = RigidBody::<3>::static_body(
            BodyHandle(1),
            Point::new([0.999, 0.0, 0.0]),
            Box::new(Sphere::new(Point::origin(), 0.5)),
        );
        fixed.friction = 0.5;
        fixed.restitution = 0.2;
        world.add_body(fixed);
        let receipt = world.try_step_with_receipt(0.001).unwrap();
        let ledger = &receipt.work_ledger;
        assert!(ledger.normal_contact_kinetic_energy_change_joules < 0.0);
        assert!(ledger.friction_contact_kinetic_energy_change_joules < 0.0);
        assert!(ledger.frictional_dissipation_joules > 0.0);
        assert!(
            (ledger.frictional_dissipation_joules
                + ledger.friction_contact_kinetic_energy_change_joules)
                .abs()
                < 1e-12
        );
        assert!(ledger.unattributed_energy_change_joules.is_finite());
    }

    #[test]
    fn work_ledger_relative_residual_is_invariant_to_energy_units() {
        let si = DiscreteWorkLedger {
            external_force_work_joules: 2.0,
            measured_mechanical_energy_change_joules: 2.1,
            unattributed_energy_change_joules: 0.1,
            ..DiscreteWorkLedger::default()
        };
        let converted = DiscreteWorkLedger {
            external_force_work_joules: 2.0e7,
            measured_mechanical_energy_change_joules: 2.1e7,
            unattributed_energy_change_joules: 1.0e6,
            ..DiscreteWorkLedger::default()
        };
        assert!(
            (si.relative_unattributed_residual() - converted.relative_unattributed_residual())
                .abs()
                < 1e-15
        );
    }


}
