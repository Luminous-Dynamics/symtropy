// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Prismatic joint (slider): constrains motion to a single axis.
//!
//! Removes D-1 translational DOF and all rotational DOF, leaving 1 translational DOF.
//! Uses: suspension springs, elevator platforms, sliding doors, linear actuators.

#[cfg(test)]
use nalgebra::SVector;

use crate::body::{BodyHandle, RigidBody};
use crate::constraint::{Constraint, MotorStepState};

/// Prismatic joint: body B slides along one axis relative to body A.
pub struct PrismaticJoint<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    /// The axis along which sliding is permitted (0=X, 1=Y, 2=Z, ...).
    pub axis: usize,
    /// Optional displacement limits (min, max) relative to the initial offset.
    pub limits: Option<(f64, f64)>,
    /// Constraint stiffness [0, 1]. 1.0 = rigid.
    pub stiffness: f64,
    /// Optional motor drive for actuated prismatic joints.
    pub motor: Option<super::MotorDrive>,
}

impl<const D: usize> PrismaticJoint<D> {
    /// Create a prismatic joint along the given axis.
    pub fn new(body_a: BodyHandle, body_b: BodyHandle, axis: usize) -> Self {
        debug_assert!(axis < D, "axis {axis} out of range for D={D}");
        Self {
            body_a,
            body_b,
            axis,
            limits: None,
            stiffness: 1.0,
            motor: None,
        }
    }

    /// Add displacement limits.
    pub fn with_limits(mut self, min: f64, max: f64) -> Self {
        self.limits = Some((min, max));
        self
    }

    /// Add a motor drive.
    pub fn with_motor(mut self, motor: super::MotorDrive) -> Self {
        self.motor = Some(motor);
        self
    }
}

impl<const D: usize> Constraint<D> for PrismaticJoint<D> {
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

        // Correct all axes EXCEPT the slide axis
        let total_inv_mass = body_a.inv_mass + body_b.inv_mass;
        if total_inv_mass < 1e-15 {
            return;
        }

        let ratio_a = body_a.inv_mass / total_inv_mass;
        let ratio_b = body_b.inv_mass / total_inv_mass;

        for i in 0..D {
            if i == self.axis {
                // Apply limits on the slide axis
                if let Some((min, max)) = self.limits {
                    let displacement = delta[i];
                    if displacement < min {
                        let correction = (min - displacement) * self.stiffness;
                        if body_a.is_dynamic() {
                            body_a.transform.translation.0[i] -= correction * ratio_a;
                        }
                        if body_b.is_dynamic() {
                            body_b.transform.translation.0[i] += correction * ratio_b;
                        }
                    } else if displacement > max {
                        let correction = (displacement - max) * self.stiffness;
                        if body_a.is_dynamic() {
                            body_a.transform.translation.0[i] += correction * ratio_a;
                        }
                        if body_b.is_dynamic() {
                            body_b.transform.translation.0[i] -= correction * ratio_b;
                        }
                    }
                }
                continue;
            }

            // Constrain non-slide axes to zero displacement
            let error = delta[i];
            if error.abs() < 1e-15 {
                continue;
            }
            let correction = error * self.stiffness;
            if body_a.is_dynamic() {
                body_a.transform.translation.0[i] += correction * ratio_a;
            }
            if body_b.is_dynamic() {
                body_b.transform.translation.0[i] -= correction * ratio_b;
            }
        }
    }

    fn solve_velocity(
        &self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        dt: f64,
        mut callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
    ) {
        // Preserve the direct one-call API while using the same step-scoped
        // accumulator as PhysicsWorld.
        let mut motor_state = MotorStepState::default();
        self.solve_velocity_with_state(body_a, body_b, dt, None, &mut motor_state);
        if let Some(work_joules) = motor_state.take_work_receipt() {
            if let Some(cb) = callback.as_deref_mut() {
                cb.record_work(self.body_b, work_joules);
            }
        }
    }

    fn solve_velocity_with_state(
        &self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        dt: f64,
        _callback: Option<&mut dyn crate::world::PhysicsCallback<D>>,
        motor_state: &mut MotorStepState,
    ) {
        let inv_mass_a = if body_a.is_dynamic() { body_a.inv_mass } else { 0.0 };
        let inv_mass_b = if body_b.is_dynamic() { body_b.inv_mass } else { 0.0 };
        let total_inv = inv_mass_a + inv_mass_b;
        if !total_inv.is_finite() || total_inv <= 1e-15 {
            return;
        }

        // Damp velocity on non-slide axes.
        for i in 0..D {
            if i == self.axis {
                continue;
            }
            let rel_vel = body_b.linear_velocity[i] - body_a.linear_velocity[i];
            if rel_vel.abs() < 1e-15 {
                continue;
            }
            let impulse = -rel_vel * self.stiffness / total_inv;
            if body_a.is_dynamic() {
                body_a.linear_velocity[i] -= impulse * inv_mass_a;
            }
            if body_b.is_dynamic() {
                body_b.linear_velocity[i] += impulse * inv_mass_b;
            }
        }

        // One accumulated motor impulse budget spans every solver iteration.
        if let Some(motor) = self.motor.as_ref() {
            let displacement = body_b.transform.translation.0[self.axis]
                - body_a.transform.translation.0[self.axis];
            let rel_vel = body_b.linear_velocity[self.axis] - body_a.linear_velocity[self.axis];
            let motor_inputs_valid = dt.is_finite()
                && dt > 0.0
                && displacement.is_finite()
                && rel_vel.is_finite()
                && motor.target_pos.is_finite()
                && motor.target_vel.is_finite()
                && motor.kp.is_finite()
                && motor.kd.is_finite()
                && motor.max_force.is_finite()
                && motor.max_force >= 0.0;

            if motor_inputs_valid {
                let desired_impulse_increment = motor.calculate_impulse(displacement, rel_vel, dt);
                if let Some((delta_impulse, _velocity_after)) = motor_state.apply_impulse_increment(
                    desired_impulse_increment,
                    motor.max_force * dt,
                    rel_vel,
                    total_inv,
                ) {
                    if body_a.is_dynamic() {
                        body_a.linear_velocity[self.axis] -= delta_impulse * inv_mass_a;
                    }
                    if body_b.is_dynamic() {
                        body_b.linear_velocity[self.axis] += delta_impulse * inv_mass_b;
                    }
                }
            }
        }

        // Damp all angular velocity (prismatic = no rotation).
        for i in 0..D {
            for j in (i + 1)..D {
                let rel = body_b.angular_velocity.get(i, j) - body_a.angular_velocity.get(i, j);
                if rel.abs() < 1e-10 {
                    continue;
                }
                let correction = rel * self.stiffness * 0.5;
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;

    #[test]
    fn slides_along_axis() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([2.0, 3.0, 0.0]), 0.5, 1.0);

        // Prismatic along X — Y should be corrected to 0, X preserved
        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0);

        for _ in 0..20 {
            joint.solve(&mut a, &mut b, 0.016);
        }

        // Y should be near 0 (constrained)
        assert!(
            b.transform.translation.coord(1).abs() < 0.1,
            "Y should be constrained to 0, got {}",
            b.transform.translation.coord(1)
        );
        // X should be preserved (free axis)
        assert!(
            b.transform.translation.coord(0).abs() > 0.5,
            "X should be free, got {}",
            b.transform.translation.coord(0)
        );
    }

    #[test]
    fn limits_respected() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([10.0, 0.0, 0.0]), 0.5, 1.0);

        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0).with_limits(-2.0, 5.0);

        for _ in 0..30 {
            joint.solve(&mut a, &mut b, 0.016);
        }

        let x = b.transform.translation.coord(0);
        assert!(x <= 5.5, "X should be clamped to max 5.0, got {x}");
    }

    #[test]
    fn perpendicular_velocity_damped() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        b.linear_velocity = SVector::from([0.0, 10.0, 5.0]); // perpendicular velocity

        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0);
        joint.solve_velocity(&mut a, &mut b, 0.016, None);

        assert!(
            b.linear_velocity[1].abs() < 10.0,
            "Y velocity should be damped, got {}",
            b.linear_velocity[1]
        );
    }

    #[test]
    fn prismatic_4d() {
        let mut a = RigidBody::<4>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        let mut b = RigidBody::<4>::dynamic_sphere(
            BodyHandle(1),
            Point::new([0.0, 0.0, 0.0, 5.0]),
            0.5,
            1.0,
        );

        // Slide along W axis
        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 3);

        for _ in 0..20 {
            joint.solve(&mut a, &mut b, 0.016);
        }

        // W should be free, others constrained near 0
        assert!(
            b.transform.translation.coord(0).abs() < 0.1,
            "X should be constrained"
        );
    }
}


#[cfg(test)]
mod motor_step_tests {
    use super::*;
    use crate::constraint::MotorStepState;
    use crate::joints::MotorDrive;
    use symtropy_math::Point;

    fn run_motor_iterations(iterations: usize) -> (f64, f64, f64) {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        let motor = MotorDrive {
            target_pos: 1.0,
            target_vel: 0.0,
            kp: 2.0,
            kd: 0.0,
            max_force: 10.0,
        };
        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0).with_motor(motor);
        let mut state = MotorStepState::default();

        for _ in 0..iterations {
            joint.solve_velocity_with_state(&mut a, &mut b, 0.1, None, &mut state);
        }

        let receipt = state.take_work_receipt().expect("one aggregated work receipt");
        assert_eq!(state.take_work_receipt(), None);
        (state.accumulated_impulse(), b.linear_velocity[0], receipt)
    }

    #[test]
    fn prismatic_motor_budget_and_work_are_not_multiplied_by_solver_iterations() {
        let reference = run_motor_iterations(1);
        for iterations in [2, 4, 8] {
            let result = run_motor_iterations(iterations);
            assert!((result.0 - reference.0).abs() < 1.0e-12);
            assert!((result.1 - reference.1).abs() < 1.0e-12);
            assert!((result.2 - reference.2).abs() < 1.0e-12);
        }
        assert!((reference.0 - 0.2).abs() < 1.0e-12);
        assert!((reference.1 - 0.2).abs() < 1.0e-12);
        assert!((reference.2 - 0.02).abs() < 1.0e-12);
    }

    #[test]
    fn prismatic_static_endpoint_receives_full_dynamic_response() {
        let mut anchor = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut moving =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 2.0);
        let motor = MotorDrive {
            target_pos: 1.0,
            target_vel: 0.0,
            kp: 2.0,
            kd: 0.0,
            max_force: 10.0,
        };
        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0).with_motor(motor);
        let mut state = MotorStepState::default();

        joint.solve_velocity_with_state(&mut anchor, &mut moving, 0.1, None, &mut state);

        // 2 N applied for 0.1 s to a 2 kg body -> 0.1 m/s.
        assert!((moving.linear_velocity[0] - 0.1).abs() < 1.0e-12);
        assert!((anchor.linear_velocity[0]).abs() < 1.0e-12);
        assert!((state.accumulated_impulse() - 0.2).abs() < 1.0e-12);
    }


    #[test]
    fn prismatic_motor_rejects_negative_effort_without_mutating_velocity() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        let motor = MotorDrive {
            target_pos: 1.0,
            target_vel: 0.0,
            kp: 1.0,
            kd: 0.0,
            max_force: -1.0,
        };
        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0).with_motor(motor);
        let mut state = MotorStepState::default();

        joint.solve_velocity_with_state(&mut a, &mut b, 0.1, None, &mut state);

        assert_eq!(b.linear_velocity[0], 0.0);
        assert_eq!(state.take_work_receipt(), None);
    }


    #[test]
    fn prismatic_motor_accumulates_velocity_feedback_impulse_increments() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        let motor = MotorDrive {
            target_pos: 1.0,
            target_vel: 0.0,
            kp: 1.0,
            kd: 1.0,
            max_force: 10.0,
        };
        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0).with_motor(motor);
        let mut state = MotorStepState::default();

        for _ in 0..8 {
            joint.solve_velocity_with_state(&mut a, &mut b, 0.1, None, &mut state);
        }

        // PD output changes as the slider accelerates; each output is an
        // incremental impulse added to one total step budget.
        assert!(state.accumulated_impulse() > 0.1);
        assert!(state.accumulated_impulse() <= 1.0 + 1.0e-12);
        assert!(b.linear_velocity[0] > 0.0);
        assert!(state.take_work_receipt().is_some());
    }

    #[test]
    fn prismatic_motor_impulse_respects_one_step_effort_cap() {
        let mut a = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(symtropy_math::Sphere::new(Point::origin(), 0.1)),
        );
        let mut b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        let motor = MotorDrive {
            target_pos: 100.0,
            target_vel: 0.0,
            kp: 100.0,
            kd: 0.0,
            max_force: 1.0,
        };
        let joint = PrismaticJoint::new(BodyHandle(0), BodyHandle(1), 0).with_motor(motor);
        let mut state = MotorStepState::default();

        for _ in 0..8 {
            joint.solve_velocity_with_state(&mut a, &mut b, 0.1, None, &mut state);
        }

        assert!((state.accumulated_impulse() - 0.1).abs() < 1.0e-12);
        assert!((b.linear_velocity[0] - 0.1).abs() < 1.0e-12);
    }
}
