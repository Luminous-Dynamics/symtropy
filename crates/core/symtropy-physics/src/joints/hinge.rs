// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Hinge joint (revolute): constrains rotation to a single plane.
//!
//! In 3D: removes 3 translational + 2 rotational DOF, leaving 1 rotational DOF.
//! In ND: removes D translational DOF + constrains angular velocity to one
//! bivector plane, leaving 1 rotational DOF.
//!
//! Uses: doors, wheels, robot arm joints, pendulums with a fixed axis.

use nalgebra::SVector;

use crate::body::{BodyHandle, RigidBody};
use crate::constraint::Constraint;

/// Hinge joint: allows rotation around a single axis/plane only.
///
/// The positional constraint (anchors coincide) works identically to BallJoint.
/// The angular constraint projects out all angular velocity components except
/// the one in the allowed bivector plane.
pub struct HingeJoint<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    /// Local-space anchor on body A.
    pub anchor_a: SVector<f64, D>,
    /// Local-space anchor on body B.
    pub anchor_b: SVector<f64, D>,
    /// The two axes defining the allowed rotation plane.
    /// In 3D: axis_a=0, axis_b=1 means rotation in the XY plane (around Z).
    pub plane_axis_a: usize,
    pub plane_axis_b: usize,
    /// Constraint stiffness [0, 1]. 1.0 = rigid.
    pub stiffness: f64,
    /// Optional angle limits (min, max) in radians. `None` = unlimited.
    pub angle_limits: Option<(f64, f64)>,
    /// Optional motor drive for actuated hinge joints.
    pub motor: Option<super::MotorDrive>,
}

impl<const D: usize> HingeJoint<D> {
    /// Create a hinge joint that rotates in the plane defined by (axis_a, axis_b).
    ///
    /// In 3D, `plane(0, 1)` means rotation around Z.
    /// In 4D, `plane(0, 3)` means rotation in the XW plane.
    pub fn new(
        body_a: BodyHandle,
        body_b: BodyHandle,
        plane_axis_a: usize,
        plane_axis_b: usize,
    ) -> Self {
        debug_assert!(plane_axis_a < D && plane_axis_b < D);
        debug_assert!(plane_axis_a != plane_axis_b);
        Self {
            body_a,
            body_b,
            anchor_a: SVector::zeros(),
            anchor_b: SVector::zeros(),
            plane_axis_a,
            plane_axis_b,
            stiffness: 1.0,
            angle_limits: None,
            motor: None,
        }
    }

    /// Add a motor drive.
    pub fn with_motor(mut self, motor: super::MotorDrive) -> Self {
        self.motor = Some(motor);
        self
    }

    /// Create with specific anchors.
    pub fn with_anchors(
        body_a: BodyHandle,
        body_b: BodyHandle,
        anchor_a: SVector<f64, D>,
        anchor_b: SVector<f64, D>,
        plane_axis_a: usize,
        plane_axis_b: usize,
    ) -> Self {
        Self {
            body_a,
            body_b,
            anchor_a,
            anchor_b,
            plane_axis_a,
            plane_axis_b,
            stiffness: 1.0,
            angle_limits: None,
            motor: None,
        }
    }

    /// Set angle limits in radians.
    pub fn with_limits(mut self, min: f64, max: f64) -> Self {
        self.angle_limits = Some((min, max));
        self
    }
}

impl<const D: usize> Constraint<D> for HingeJoint<D> {
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
        // === Positional constraint (same as BallJoint) ===
        let world_a = body_a.transform.translation.0
            + body_a.transform.rotation.rotate_vector(&self.anchor_a);
        let world_b = body_b.transform.translation.0
            + body_b.transform.rotation.rotate_vector(&self.anchor_b);

        let error = world_b - world_a;
        if error.norm() < 1e-15 {
            return;
        }

        let correction = error * self.stiffness;
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
        mut callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
    ) {
        // === Positional velocity constraint (same as BallJoint) ===
        let world_a = body_a.transform.translation.0
            + body_a.transform.rotation.rotate_vector(&self.anchor_a);
        let world_b = body_b.transform.translation.0
            + body_b.transform.rotation.rotate_vector(&self.anchor_b);

        let joint_dir = world_b - world_a;
        let dist = joint_dir.norm();
        if dist > 1e-15 {
            let normal = joint_dir / dist;
            let rel_vel = body_b.linear_velocity - body_a.linear_velocity;
            let vel_along = rel_vel.dot(&normal);
            let total_inv = body_a.inv_mass + body_b.inv_mass;
            if total_inv > 1e-15 {
                let impulse_mag = -vel_along * self.stiffness / total_inv;
                let impulse = normal * impulse_mag;
                if body_a.is_dynamic() {
                    body_a.linear_velocity -= impulse * body_a.inv_mass;
                }
                if body_b.is_dynamic() {
                    body_b.linear_velocity += impulse * body_b.inv_mass;
                }
            }
        }

        // === Angular constraint: project out disallowed angular velocity ===
        // The hinge allows rotation only in the (plane_axis_a, plane_axis_b) plane.
        // For each bivector plane that is NOT the allowed plane,
        // damp the relative angular velocity.
        let num_bivector_components = D * (D - 1) / 2;
        if num_bivector_components <= 1 && self.motor.is_none() {
            return; // 2D: only one rotation plane, hinge is trivially satisfied
        }

        let (pa, pb) = if self.plane_axis_a < self.plane_axis_b {
            (self.plane_axis_a, self.plane_axis_b)
        } else {
            (self.plane_axis_b, self.plane_axis_a)
        };

        for i in 0..D {
            for j in (i + 1)..D {
                if i == pa && j == pb {
                    continue; // Allowed plane
                }

                let rel_component =
                    body_b.angular_velocity.get(i, j) - body_a.angular_velocity.get(i, j);
                if rel_component.abs() < 1e-10 {
                    continue;
                }

                let correction = rel_component * self.stiffness * 0.5;
                if body_a.is_dynamic() {
                    body_a.angular_velocity.set(
                        i,
                        j,
                        body_a.angular_velocity.get(i, j) + correction,
                    );
                }
                if body_b.is_dynamic() {
                    body_b.angular_velocity.set(
                        i,
                        j,
                        body_b.angular_velocity.get(i, j) - correction,
                    );
                }
            }
        }

        // Motor drive on the hinge rotation axis
        if let Some(ref motor) = self.motor {
            let rel_angular_vel =
                body_b.angular_velocity.get(pa, pb) - body_a.angular_velocity.get(pa, pb);

            // Velocity-only drive: HingeJoint does not yet maintain a
            // continuous relative angle, so the position term (kp * position
            // error) is unavailable and target_pos/kp are intentionally not
            // consumed here. Use MotorDrive's velocity-error gain (kd) only.
            // This makes zero torque the equilibrium at target_vel; adding an
            // extra -kd*current_velocity term would incorrectly brake a joint
            // even after it reached a nonzero target speed.
            //
            // Position mode remains unsupported until an unwrapped relative
            // angle, reference-frame convention and limit interaction are
            // implemented. Keep this API claim ceiling explicit.
            let torque = (motor.kd * (motor.target_vel - rel_angular_vel))
                .clamp(-motor.max_force, motor.max_force);

            // Apply the motor's angular impulse through the integrator's
            // declared inertia model. The integrator currently uses a scalar
            // mean inverse inertia (exact for isotropic bodies, approximate
            // for asymmetric bodies); unlike a fixed half-split, this handles
            // static endpoints and unequal endpoint inertias consistently
            // within that declared approximation.
            let impulse = torque * _dt;
            let mut angular_impulse = symtropy_math::Bivector::<D>::zero();
            angular_impulse.set(pa, pb, impulse);

            // Compute endpoint responses with the same world-space inertia
            // operator used by the integrator. This keeps motor work and
            // angular velocity updates consistent for rotated asymmetric links.
            let impulse_a = angular_impulse.scale(-1.0);
            let delta_omega_a = body_a
                .apply_inverse_angular_inertia(&impulse_a)
                .get(pa, pb);
            let delta_omega_b = body_b
                .apply_inverse_angular_inertia(&angular_impulse)
                .get(pa, pb);

            // Capture endpoint velocities before applying the equal-and-
            // opposite motor impulses so the energy ledger can attribute
            // signed work to each dynamic body.
            let omega_a_before = body_a.angular_velocity.get(pa, pb);
            let omega_b_before = body_b.angular_velocity.get(pa, pb);

            if body_a.is_dynamic() {
                crate::integrator::apply_angular_impulse(
                    body_a,
                    &angular_impulse.scale(-1.0),
                );
            }
            if body_b.is_dynamic() {
                crate::integrator::apply_angular_impulse(body_b, &angular_impulse);
            }

            // Record signed mechanical work per dynamic endpoint using
            // midpoint angular velocity. Summing the two entries equals the
            // relative motor work and the kinetic-energy change in the
            // engine's declared scalar-inertia model. This is not measured
            // electrical input or evidence of regenerative battery return.
            if let Some(ref mut cb) = callback {
                if body_a.is_dynamic() {
                    let midpoint_omega_a = omega_a_before + 0.5 * delta_omega_a;
                    cb.record_work(self.body_a, -impulse * midpoint_omega_a);
                }
                if body_b.is_dynamic() {
                    let midpoint_omega_b = omega_b_before + 0.5 * delta_omega_b;
                    cb.record_work(self.body_b, impulse * midpoint_omega_b);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;

    #[test]
    fn hinge_maintains_position() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([3.0, 0.0, 0.0]), 0.5, 1.0);

        // Hinge in XY plane (rotation around Z)
        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1);

        for _ in 0..20 {
            joint.solve(&mut a, &mut b, 0.016);
        }

        let dist = a.transform.translation.distance(&b.transform.translation);
        assert!(dist < 0.1, "hinge should connect bodies, dist = {dist}");
    }

    #[test]
    fn hinge_allows_rotation_in_plane() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);

        // Give body B angular velocity in the allowed XY plane (around Z)
        b.angular_velocity.set(0, 1, 5.0);
        // And some in the disallowed XZ plane (around Y)
        b.angular_velocity.set(0, 2, 3.0);

        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1);
        joint.solve_velocity(&mut a, &mut b, 0.016, None);

        // Allowed rotation should be preserved (minus some damping from relative constraint)
        let xy_after = b.angular_velocity.get(0, 1);
        assert!(
            xy_after.abs() > 1.0,
            "XY rotation should be mostly preserved, got {xy_after}"
        );

        // Disallowed rotation should be reduced
        let xz_after = b.angular_velocity.get(0, 2);
        assert!(
            xz_after.abs() < 3.0,
            "XZ rotation should be reduced from 3.0, got {xz_after}"
        );
    }

    #[test]
    fn hinge_velocity_motor_has_zero_effort_at_nonzero_target_velocity() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::origin(),
            1.0,
            1.0,
        );
        b.angular_velocity.set(0, 1, 1.0);
        let before = b.angular_velocity.get(0, 1);
        let motor = super::super::MotorDrive {
            target_pos: -2.0,
            target_vel: 1.0,
            kp: 100.0,
            kd: 10.0,
            max_force: 100.0,
        };
        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1)
            .with_motor(motor);

        joint.solve_velocity(&mut a, &mut b, 0.1, None);

        assert!(
            (b.angular_velocity.get(0, 1) - before).abs() < 1e-12,
            "a velocity drive must not brake or accelerate when target velocity is met"
        );
    }

    #[test]
    fn hinge_velocity_motor_respects_torque_saturation() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::origin(),
            1.0,
            1.0,
        );
        let motor = super::super::MotorDrive {
            target_pos: 0.0,
            target_vel: 100.0,
            kp: 0.0,
            kd: 10.0,
            max_force: 3.0,
        };
        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1)
            .with_motor(motor);
        let dt = 0.1;
        let expected_delta = 3.0 * dt * (b.inv_inertia.sum() / 3.0);

        joint.solve_velocity(&mut a, &mut b, dt, None);

        assert!(
            (b.angular_velocity.get(0, 1) - expected_delta).abs() < 1e-12,
            "velocity motor impulse must respect max_force"
        );
    }

    #[test]
    fn hinge_motor_impulse_uses_inverse_inertia_with_static_base() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::origin(),
            1.0,
            1.0,
        );
        let motor = super::super::MotorDrive {
            target_pos: 0.0,
            target_vel: 1.0,
            kp: 0.0,
            kd: 10.0,
            max_force: 100.0,
        };
        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1)
            .with_motor(motor);
        let dt = 0.1;
        let expected_impulse = 10.0 * dt;
        let expected_delta = expected_impulse * (b.inv_inertia.sum() / 3.0);

        joint.solve_velocity(&mut a, &mut b, dt, None);

        let actual_delta = b.angular_velocity.get(0, 1);
        assert!(
            (actual_delta - expected_delta).abs() < 1e-12,
            "static-base hinge motor must apply the full inverse-inertia-scaled impulse: expected {expected_delta}, got {actual_delta}"
        );
    }

    #[test]
    fn hinge_motor_impulse_respects_unequal_endpoint_inertias() {
        let mut a = RigidBody::<3>::dynamic_sphere(
            BodyHandle(0),
            Point::origin(),
            0.5,
            1.0,
        );
        let mut b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::origin(),
            1.0,
            1.0,
        );
        let inv_i_a = a.inv_inertia.sum() / 3.0;
        let inv_i_b = b.inv_inertia.sum() / 3.0;
        let motor = super::super::MotorDrive {
            target_pos: 0.0,
            target_vel: 1.0,
            kp: 0.0,
            kd: 10.0,
            max_force: 100.0,
        };
        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1)
            .with_motor(motor);
        let dt = 0.1;
        let expected_impulse = 10.0 * dt;

        joint.solve_velocity(&mut a, &mut b, dt, None);

        let delta_a = a.angular_velocity.get(0, 1);
        let delta_b = b.angular_velocity.get(0, 1);
        assert!((delta_a + expected_impulse * inv_i_a).abs() < 1e-12);
        assert!((delta_b - expected_impulse * inv_i_b).abs() < 1e-12);
        assert!(
            (delta_b - delta_a - expected_impulse * (inv_i_a + inv_i_b)).abs() < 1e-12
        );
    }

    #[test]
    fn hinge_motor_records_midpoint_mechanical_work() {
        struct WorkRecorder {
            work_joules: f64,
        }

        impl crate::world::PhysicsCallback<3> for WorkRecorder {
            fn modulate_force(
                &self,
                _body: BodyHandle,
                force: &SVector<f64, 3>,
            ) -> SVector<f64, 3> {
                *force
            }
            fn modulate_impulse(
                &self,
                impulse: f64,
                _contact: &SVector<f64, 3>,
            ) -> f64 {
                impulse
            }
            fn friction_multiplier(
                &self,
                _contact: &SVector<f64, 3>,
                _body: BodyHandle,
            ) -> f64 {
                1.0
            }
            fn on_collision(&mut self, _event: &crate::contact::CollisionEvent<3>) {}
            fn record_dissipation(&mut self, _energy: f64) {}
            fn record_work(&mut self, _body: BodyHandle, work_joules: f64) {
                self.work_joules += work_joules;
            }
            fn apply_trauma(&mut self, _event: &crate::contact::CollisionEvent<3>) {}
        }

        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::origin(),
            1.0,
            1.0,
        );
        b.angular_velocity.set(0, 1, 0.5);
        let motor = super::super::MotorDrive {
            target_pos: 0.0,
            target_vel: 1.0,
            kp: 0.0,
            kd: 10.0,
            max_force: 100.0,
        };
        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1)
            .with_motor(motor);
        let dt = 0.1;
        let inv_i = b.inv_inertia.sum() / 3.0;
        let impulse = 5.0 * dt;
        let omega_before = b.angular_velocity.get(0, 1);
        let delta_omega = impulse * inv_i;
        let expected_work =
            impulse * (omega_before + 0.5 * delta_omega);
        let mut recorder = WorkRecorder { work_joules: 0.0 };

        joint.solve_velocity(&mut a, &mut b, dt, Some(&mut recorder));

        let delta_kinetic_energy = 0.5 / inv_i
            * (b.angular_velocity.get(0, 1).powi(2) - omega_before.powi(2));
        assert!((recorder.work_joules - expected_work).abs() < 1e-12);
        assert!((recorder.work_joules - delta_kinetic_energy).abs() < 1e-12);
    }

    #[test]
    fn hinge_motor_attributes_work_to_each_dynamic_endpoint() {
        use std::collections::HashMap;

        struct PerBodyWorkRecorder {
            work_by_body: HashMap<BodyHandle, f64>,
        }

        impl crate::world::PhysicsCallback<3> for PerBodyWorkRecorder {
            fn modulate_force(
                &self,
                _body: BodyHandle,
                force: &SVector<f64, 3>,
            ) -> SVector<f64, 3> {
                *force
            }
            fn modulate_impulse(
                &self,
                impulse: f64,
                _contact: &SVector<f64, 3>,
            ) -> f64 {
                impulse
            }
            fn friction_multiplier(
                &self,
                _contact: &SVector<f64, 3>,
                _body: BodyHandle,
            ) -> f64 {
                1.0
            }
            fn on_collision(&mut self, _event: &crate::contact::CollisionEvent<3>) {}
            fn record_dissipation(&mut self, _energy: f64) {}
            fn record_work(&mut self, body: BodyHandle, work_joules: f64) {
                *self.work_by_body.entry(body).or_default() += work_joules;
            }
            fn apply_trauma(&mut self, _event: &crate::contact::CollisionEvent<3>) {}
        }

        let mut a = RigidBody::<3>::dynamic_sphere(
            BodyHandle(0),
            Point::origin(),
            0.5,
            1.0,
        );
        let mut b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::origin(),
            1.0,
            1.0,
        );
        a.angular_velocity.set(0, 1, -0.2);
        b.angular_velocity.set(0, 1, 0.6);

        let motor = super::super::MotorDrive {
            target_pos: 0.0,
            target_vel: 1.0,
            kp: 0.0,
            kd: 10.0,
            max_force: 100.0,
        };
        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1)
            .with_motor(motor);
        let dt = 0.1;
        let inv_i_a = a.inv_inertia.sum() / 3.0;
        let inv_i_b = b.inv_inertia.sum() / 3.0;
        let omega_a_before = a.angular_velocity.get(0, 1);
        let omega_b_before = b.angular_velocity.get(0, 1);
        let relative_before = omega_b_before - omega_a_before;
        let torque = (10.0 * (1.0 - relative_before)).clamp(-100.0, 100.0);
        let impulse = torque * dt;
        let expected_a = -impulse * (omega_a_before - 0.5 * impulse * inv_i_a);
        let expected_b = impulse * (omega_b_before + 0.5 * impulse * inv_i_b);

        let mut recorder = PerBodyWorkRecorder {
            work_by_body: HashMap::new(),
        };
        joint.solve_velocity(&mut a, &mut b, dt, Some(&mut recorder));

        let work_a = *recorder.work_by_body.get(&BodyHandle(0)).unwrap();
        let work_b = *recorder.work_by_body.get(&BodyHandle(1)).unwrap();
        let total_work = work_a + work_b;
        let delta_kinetic_energy = 0.5 / inv_i_a
            * (a.angular_velocity.get(0, 1).powi(2) - omega_a_before.powi(2))
            + 0.5 / inv_i_b
                * (b.angular_velocity.get(0, 1).powi(2) - omega_b_before.powi(2));

        assert!((work_a - expected_a).abs() < 1e-12);
        assert!((work_b - expected_b).abs() < 1e-12);
        assert!((total_work - delta_kinetic_energy).abs() < 1e-12);
    }

    #[test]
    fn hinge_motor_attributes_work_to_dynamic_a_when_b_is_static() {
        use std::collections::HashMap;

        struct PerBodyWorkRecorder {
            work_by_body: HashMap<BodyHandle, f64>,
        }

        impl crate::world::PhysicsCallback<3> for PerBodyWorkRecorder {
            fn modulate_force(
                &self,
                _body: BodyHandle,
                force: &SVector<f64, 3>,
            ) -> SVector<f64, 3> {
                *force
            }
            fn modulate_impulse(
                &self,
                impulse: f64,
                _contact: &SVector<f64, 3>,
            ) -> f64 {
                impulse
            }
            fn friction_multiplier(
                &self,
                _contact: &SVector<f64, 3>,
                _body: BodyHandle,
            ) -> f64 {
                1.0
            }
            fn on_collision(&mut self, _event: &crate::contact::CollisionEvent<3>) {}
            fn record_dissipation(&mut self, _energy: f64) {}
            fn record_work(&mut self, body: BodyHandle, work_joules: f64) {
                *self.work_by_body.entry(body).or_default() += work_joules;
            }
            fn apply_trauma(&mut self, _event: &crate::contact::CollisionEvent<3>) {}
        }

        let mut a = RigidBody::<3>::dynamic_sphere(
            BodyHandle(0),
            Point::origin(),
            0.5,
            1.0,
        );
        let mut b = RigidBody::<3>::static_body(
            BodyHandle(1),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        a.angular_velocity.set(0, 1, -0.2);

        let motor = super::super::MotorDrive {
            target_pos: 0.0,
            target_vel: 1.0,
            kp: 0.0,
            kd: 10.0,
            max_force: 100.0,
        };
        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1)
            .with_motor(motor);
        let dt = 0.1;
        let inv_i_a = a.inv_inertia.sum() / 3.0;
        let omega_a_before = a.angular_velocity.get(0, 1);
        let impulse = 10.0 * (1.0 - (0.0 - omega_a_before)) * dt;
        let expected_work_a =
            -impulse * (omega_a_before - 0.5 * impulse * inv_i_a);
        let mut recorder = PerBodyWorkRecorder {
            work_by_body: HashMap::new(),
        };

        joint.solve_velocity(&mut a, &mut b, dt, Some(&mut recorder));

        let work_a = *recorder.work_by_body.get(&BodyHandle(0)).unwrap();
        let delta_kinetic_energy = 0.5 / inv_i_a
            * (a.angular_velocity.get(0, 1).powi(2) - omega_a_before.powi(2));
        assert!(!recorder.work_by_body.contains_key(&BodyHandle(1)));
        assert!((work_a - expected_work_a).abs() < 1e-12);
        assert!((work_a - delta_kinetic_energy).abs() < 1e-12);
    }

    #[test]
    fn hinge_4d() {
        let mut a = RigidBody::<4>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        let mut b = RigidBody::<4>::dynamic_sphere(
            BodyHandle(1),
            Point::new([2.0, 0.0, 0.0, 0.0]),
            0.5,
            1.0,
        );

        // 4D hinge: rotation in XW plane (axes 0, 3)
        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 3);

        for _ in 0..20 {
            joint.solve(&mut a, &mut b, 0.016);
        }

        let dist = a.transform.translation.distance(&b.transform.translation);
        assert!(dist < 0.1, "4D hinge, dist = {dist}");
    }

    #[test]
    fn hinge_2d_trivial() {
        // In 2D, there's only one rotation plane (0,1). A hinge in that plane
        // is equivalent to a ball joint — all rotation is allowed.
        let mut a = RigidBody::<2>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        let mut b = RigidBody::<2>::dynamic_sphere(BodyHandle(1), Point::new([2.0, 0.0]), 0.5, 1.0);
        b.angular_velocity.set(0, 1, 3.0);

        let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1);
        joint.solve_velocity(&mut a, &mut b, 0.016, None);

        // In 2D, no angular constraint is applied (only 1 plane)
        // The angular velocity should be modified only by the relative constraint
        assert!(
            b.angular_velocity.get(0, 1).abs() > 0.0,
            "2D hinge should allow rotation"
        );
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use symtropy_math::Point;

    proptest! {
        /// Property: after repeatedly injecting random angular velocity into
        /// BOTH the allowed hinge plane and disallowed planes, then solving
        /// the joint's velocity constraint each step, the disallowed-plane
        /// relative angular velocity is driven toward (and stays near) zero —
        /// the hinge's single-DOF constraint holds within tolerance,
        /// regardless of how much "random torque" keeps perturbing it.
        #[test]
        fn hinge_disallowed_plane_stays_near_zero_under_random_torque(
            seed_xz in prop::collection::vec(-5.0f64..5.0, 20),
            seed_yz in prop::collection::vec(-5.0f64..5.0, 20),
        ) {
            let mut a = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
            let mut b = RigidBody::<3>::dynamic_sphere(
                BodyHandle(1),
                Point::new([2.0, 0.0, 0.0]),
                0.5,
                1.0,
            );

            // Hinge allows rotation only in the XY plane (axes 0, 1).
            let joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1);
            let dt = 0.016;

            for i in 0..seed_xz.len() {
                // "Random torque" injected directly as angular velocity noise
                // in the two DISALLOWED planes (XZ = axes 0,2 and YZ = axes 1,2).
                let cur_xz = b.angular_velocity.get(0, 2);
                b.angular_velocity.set(0, 2, cur_xz + seed_xz[i]);
                let cur_yz = b.angular_velocity.get(1, 2);
                b.angular_velocity.set(1, 2, cur_yz + seed_yz[i]);

                joint.solve_velocity(&mut a, &mut b, dt, None);
            }

            // After the constraint solve, disallowed-plane relative angular
            // velocity must be small — the projection in `solve_velocity`
            // halves the disallowed component every step (relative component
            // corrected by `stiffness * 0.5` split across both bodies), so it
            // should not be able to run away to the same magnitude as the
            // injected noise.
            let rel_xz = (b.angular_velocity.get(0, 2) - a.angular_velocity.get(0, 2)).abs();
            let rel_yz = (b.angular_velocity.get(1, 2) - a.angular_velocity.get(1, 2)).abs();

            prop_assert!(
                rel_xz.is_finite() && rel_yz.is_finite(),
                "hinge constraint produced non-finite angular velocity: xz={rel_xz}, yz={rel_yz}"
            );
            prop_assert!(
                rel_xz < 10.0 && rel_yz < 10.0,
                "hinge disallowed-plane angular velocity should stay bounded, got xz={rel_xz}, yz={rel_yz}"
            );
        }

        /// Property: the hinge's positional constraint (`solve`) never
        /// produces non-finite state, and monotonically reduces the anchor
        /// separation distance (or leaves it at zero), for arbitrary starting
        /// offsets and stiffness values.
        #[test]
        fn hinge_position_solve_never_diverges(
            dx in -5.0f64..5.0, dy in -5.0f64..5.0, dz in -5.0f64..5.0,
            stiffness in 0.0f64..1.0,
        ) {
            let mut a = RigidBody::<3>::static_body(
                BodyHandle(0),
                Point::origin(),
                Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
            );
            let mut b = RigidBody::<3>::dynamic_sphere(
                BodyHandle(1),
                Point::new([dx, dy, dz]),
                0.5,
                1.0,
            );

            let mut joint = HingeJoint::new(BodyHandle(0), BodyHandle(1), 0, 1);
            joint.stiffness = stiffness;

            let initial_dist = a.transform.translation.distance(&b.transform.translation);

            for _ in 0..50 {
                joint.solve(&mut a, &mut b, 0.016);
            }

            let final_dist = a.transform.translation.distance(&b.transform.translation);

            prop_assert!(final_dist.is_finite(), "hinge solve produced non-finite distance");
            prop_assert!(
                final_dist <= initial_dist + 1e-6,
                "hinge solve should never increase anchor separation: initial={initial_dist}, final={final_dist}"
            );
        }
    }
}
