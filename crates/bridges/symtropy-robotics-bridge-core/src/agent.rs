// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Core robotic agent traits and utilities.

use crate::motor::{MotorPlanResult, MotorPlanner, MotorPlannerFault, MotorPlannerStatus};
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
        let num_actuators = self.platform().num_actuators();

        // Reject bad sensor/command context before allowing the agent to mutate
        // state in tick() or letting any planner observe the request.
        if observation.iter().any(|value| !value.is_finite()) {
            return MotorPlanResult {
                commands: vec![0.0; num_actuators],
                status: MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidObservation),
            };
        }
        if !danger_level.is_finite() {
            return MotorPlanResult {
                commands: vec![0.0; num_actuators],
                status: MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidDangerLevel),
            };
        }

        let gain = self.tick(observation, danger_level);
        if !self.can_act() {
            return MotorPlanResult {
                commands: vec![0.0; num_actuators],
                status: MotorPlannerStatus::Rejected(MotorPlannerFault::AgentNotAuthorized),
            };
        }
        if !gain.is_finite() || !(0.0..=1.0).contains(&gain) {
            return MotorPlanResult {
                commands: vec![0.0; num_actuators],
                status: MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidGain),
            };
        }

        // Use the planner's typed path, then independently enforce the
        // caller-facing result contract even if an implementor overrides it.
        let result = planner.plan_result(observation, gain, num_actuators);
        if result.status != MotorPlannerStatus::Ready {
            return MotorPlanResult {
                commands: vec![0.0; num_actuators],
                status: result.status,
            };
        }
        if result.commands.len() != num_actuators
            || result.commands.iter().any(|value| !value.is_finite())
        {
            return MotorPlanResult {
                commands: vec![0.0; num_actuators],
                status: MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidOutput),
            };
        }

        result
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
    use std::cell::Cell;

    struct TestAgent {
        tick_calls: Cell<usize>,
        act_authorized: bool,
        gain: f64,
    }

    impl TestAgent {
        fn new() -> Self {
            Self {
                tick_calls: Cell::new(0),
                act_authorized: true,
                gain: 0.5,
            }
        }
    }

    impl RoboticAgent for TestAgent {
        fn body(&self) -> BodyHandle {
            BodyHandle(0)
        }

        fn platform(&self) -> PlatformType {
            PlatformType::Quadrotor
        }

        fn tick(&mut self, _observation: &[f64], _danger_level: f64) -> f64 {
            self.tick_calls.set(self.tick_calls.get() + 1);
            self.gain
        }

        fn can_act(&self) -> bool {
            self.act_authorized
        }
    }

    struct TestPlanner {
        calls: usize,
        commands: Option<Vec<f64>>,
        result_status: MotorPlannerStatus,
    }

    impl TestPlanner {
        fn ready() -> Self {
            Self {
                calls: 0,
                commands: None,
                result_status: MotorPlannerStatus::Ready,
            }
        }
    }

    impl MotorPlanner for TestPlanner {
        fn plan(&mut self, _observation: &[f64], _gain: f64, num_actuators: usize) -> Vec<f64> {
            self.calls += 1;
            self.commands
                .clone()
                .unwrap_or_else(|| vec![0.25; num_actuators])
        }

        fn status(&self) -> MotorPlannerStatus {
            self.result_status
        }
    }

    #[test]
    fn non_finite_observation_rejects_before_agent_tick_and_planner() {
        let mut agent = TestAgent::new();
        let mut planner = TestPlanner::ready();
        let result = agent.tick_motor_plan_with(&[0.5, f64::NAN], 0.0, &mut planner);

        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidObservation)
        );
        assert_eq!(agent.tick_calls.get(), 0);
        assert_eq!(planner.calls, 0);
    }

    #[test]
    fn invalid_danger_level_rejects_before_agent_tick_and_planner() {
        let mut agent = TestAgent::new();
        let mut planner = TestPlanner::ready();
        let result = agent.tick_motor_plan_with(&[0.5], f64::INFINITY, &mut planner);

        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidDangerLevel)
        );
        assert_eq!(agent.tick_calls.get(), 0);
        assert_eq!(planner.calls, 0);
    }

    #[test]
    fn agent_authorization_gate_prevents_planning() {
        let mut agent = TestAgent::new();
        agent.act_authorized = false;
        let mut planner = TestPlanner::ready();
        let result = agent.tick_motor_plan_with(&[0.5], 0.1, &mut planner);

        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::AgentNotAuthorized)
        );
        assert_eq!(agent.tick_calls.get(), 1);
        assert_eq!(planner.calls, 0);
    }

    #[test]
    fn invalid_agent_gain_prevents_planning() {
        let mut agent = TestAgent::new();
        agent.gain = f64::NAN;
        let mut planner = TestPlanner::ready();
        let result = agent.tick_motor_plan_with(&[0.5], 0.1, &mut planner);

        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidGain)
        );
        assert_eq!(planner.calls, 0);
    }

    #[test]
    fn wrong_length_planner_output_is_rejected_and_normalized() {
        let mut agent = TestAgent::new();
        let mut planner = TestPlanner::ready();
        planner.commands = Some(vec![0.1; 3]);

        let result = agent.tick_motor_plan_with(&[0.5], 0.1, &mut planner);
        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidOutput)
        );
    }

    #[test]
    fn non_finite_planner_output_is_rejected_and_normalized() {
        let mut agent = TestAgent::new();
        let mut planner = TestPlanner::ready();
        planner.commands = Some(vec![0.1, 0.2, f64::INFINITY, 0.4]);

        let result = agent.tick_motor_plan_with(&[0.5], 0.1, &mut planner);
        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidOutput)
        );
    }

    #[test]
    fn explicit_planner_rejection_does_not_leak_candidate_commands() {
        let mut agent = TestAgent::new();
        let mut planner = TestPlanner::ready();
        planner.commands = Some(vec![0.9; 4]);
        planner.result_status = MotorPlannerStatus::Rejected(MotorPlannerFault::BridgeFailure);

        let result = agent.tick_motor_plan_with(&[0.5], 0.1, &mut planner);
        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::BridgeFailure)
        );
    }

    #[test]
    fn typed_motor_plan_preserves_rejection_status_and_vector_shape() {
        let mut agent = TestAgent::new();
        let mut planner = TestPlanner::ready();
        planner.result_status = MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidObservation);

        let result = agent.tick_motor_plan_with(&[0.5], 0.0, &mut planner);
        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidObservation)
        );
    }

    #[test]
    fn valid_request_preserves_planned_commands() {
        let mut agent = TestAgent::new();
        let mut planner = TestPlanner::ready();
        let result = agent.tick_motor_plan_with(&[0.5], 0.0, &mut planner);

        assert_eq!(result.commands, vec![0.25; 4]);
        assert_eq!(result.status, MotorPlannerStatus::Ready);
        assert_eq!(agent.tick_calls.get(), 1);
        assert_eq!(planner.calls, 1);
    }
}
