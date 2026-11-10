// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Joint types for articulated bodies.
//!
//! All joints implement the `Constraint<D>` trait, using the same iterative
//! position + velocity solve pipeline as other constraints. Joints are
//! dimension-agnostic (const-generic `<const D: usize>`).

pub mod ball;
pub mod fixed;
pub mod hinge;
pub mod prismatic;

pub use ball::BallJoint;
pub use fixed::FixedJoint;
pub use hinge::HingeJoint;
pub use prismatic::PrismaticJoint;

/// Motor-drive parameters shared by actuated joint types.
///
/// **Control semantics are joint-specific.** In symtropy-physics 0.2.1,
/// `PrismaticJoint` uses `target_pos` and `target_vel` through
/// `calculate_impulse`. `HingeJoint` currently uses `target_vel` only;
/// its solver does not consume `target_pos`. The hinge's stored angle limits
/// are also not enforced by the current solver. Do not infer position-control
/// or hard-limit support for a hinge from the presence of these fields.
///
/// See PHYS-HINGE-MOTOR-001 for the required implementation and qualification:
/// https://github.com/Luminous-Dynamics/symtropy/issues/4516.
#[derive(Clone, Debug)]
pub struct MotorDrive {
    /// Target position (units) or angle (radians).
    pub target_pos: f64,
    /// Target velocity.
    pub target_vel: f64,
    /// Proportional gain (stiffness).
    pub kp: f64,
    /// Derivative gain (damping).
    pub kd: f64,
    /// Maximum force/torque the motor can apply.
    pub max_force: f64,
}

impl MotorDrive {
    /// Create a motor with default gains.
    pub fn new(target_pos: f64, max_force: f64) -> Self {
        Self {
            target_pos,
            target_vel: 0.0,
            kp: max_force,
            kd: max_force * 0.1,
            max_force,
        }
    }

    /// Set PD gains.
    pub fn with_gains(mut self, kp: f64, kd: f64) -> Self {
        self.kp = kp;
        self.kd = kd;
        self
    }

    /// Calculate motor impulse.
    pub fn calculate_impulse(&self, current_pos: f64, current_vel: f64, dt: f64) -> f64 {
        let error_pos = self.target_pos - current_pos;
        let error_vel = self.target_vel - current_vel;
        let force =
            (self.kp * error_pos + self.kd * error_vel).clamp(-self.max_force, self.max_force);
        force * dt
    }
}
