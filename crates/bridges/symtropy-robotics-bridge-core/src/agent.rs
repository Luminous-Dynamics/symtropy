// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Core robotic agent traits and utilities.

use crate::motor::{MotorPlanResult, MotorPlanner};
use crate::platform::PlatformType;
use symtropy_physics::body::BodyHandle;

/// Trait for a robotic agent in the game world.
///
/// This trait defines the interface for an agent that can be ticked by the
/// game engine and produce motor commands. The core engine is agnostic to
/// the specific AI / consciousness model driving the agent.
pub trait RoboticAgent {
    /// Physics body handle.
    fn body(&self) -> BodyHandle;

    /// Platform type.
    fn platform(&self) -> PlatformType;

    /// Run one perception-action cycle.
    /// Returns the motor gain [0.0, 1.0] for this tick.
    fn tick(&mut self, observation: &[f64], danger_level: f64) -> f64;

    /// Current Φ level (if applicable, else 0.0).
    fn phi(&self) -> f64 {
        0.0
    }

    /// Current bottleneck name (for debug/UI).
    fn bottleneck(&self) -> &str {
        "unknown"
    }

    /// Whether this agent has enough consciousness/confidence for motor output.
    fn can_act(&self) -> bool {
        true
    }

    /// Plan per-joint motor commands and preserve the planner's status.
    ///
    /// Consumers of fault-aware planners should prefer this method over the
    /// legacy vector-only helper so a rejected plan cannot be mistaken for an
    /// accepted command vector. A rejected result still contains the planner's
    /// contract-required vector shape; the caller must consult `status` and
    /// apply the appropriate platform-specific fallback.
    fn tick_motor_plan_with<P: MotorPlanner>(
        &mut self,
        observation: &[f64],
        danger_level: f64,
        planner: &mut P,
    ) -> MotorPlanResult {
        let gain = self.tick(observation, danger_level);
        let commands = planner.plan(observation, gain, self.platform().num_actuators());
        MotorPlanResult {
            commands,
            status: planner.status(),
        }
    }

    /// Legacy vector-only planning helper.
    ///
    /// This preserves source compatibility, but discards typed planner status.
    /// Safety- or integrity-sensitive callers should use
    /// `tick_motor_plan_with` and handle a rejected result explicitly.
    fn tick_motor_commands_with<P: MotorPlanner>(
        &mut self,
        observation: &[f64],
        danger_level: f64,
        planner: &mut P,
    ) -> Vec<f64> {
        self.tick_motor_plan_with(observation, danger_level, planner)
            .commands
    }

    /// Convenience: run one tick and plan per-joint motor commands using
    /// [`UniformGainPlanner`](crate::motor::UniformGainPlanner) (every actuator gets the same gain).
    fn tick_motor_commands(&mut self, observation: &[f64], danger_level: f64) -> Vec<f64> {
        let mut planner = crate::motor::UniformGainPlanner;
        self.tick_motor_commands_with(observation, danger_level, &mut planner)
    }
}

/// Spawn a robotic agent's body into the physics world.
pub fn spawn_robot_body<const D: usize>(
    physics: &mut symtropy_physics::PhysicsWorld<D>,
    platform: PlatformType,
    position: symtropy_math::Point<D>,
) -> BodyHandle {
    physics.add_sphere(position, platform.default_radius(), platform.default_mass())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::motor::{MotorPlannerFault, MotorPlannerStatus};

    struct TestAgent;

    impl RoboticAgent for TestAgent {
        fn body(&self) -> BodyHandle {
            BodyHandle(0)
        }

        fn platform(&self) -> PlatformType {
            PlatformType::Quadrotor
        }

        fn tick(&mut self, _observation: &[f64], _danger_level: f64) -> f64 {
            0.5
        }
    }

    struct RejectingPlanner;

    impl MotorPlanner for RejectingPlanner {
        fn plan(&mut self, _observation: &[f64], _gain: f64, num_actuators: usize) -> Vec<f64> {
            vec![0.0; num_actuators]
        }

        fn status(&self) -> crate::motor::MotorPlannerStatus {
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidObservation)
        }
    }

    #[test]
    fn typed_motor_plan_preserves_rejection_status_and_vector_shape() {
        let mut agent = TestAgent;
        let mut planner = RejectingPlanner;
        let result = agent.tick_motor_plan_with(&[f64::NAN], 0.0, &mut planner);

        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidObservation)
        );
    }
}
