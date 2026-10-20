// Copyright (c) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Per-joint motor-command planning.
//!
//! The [`RoboticAgent`](crate::RoboticAgent) computes a scalar motor-authority
//! gain each tick (from its consciousness level and NRC safety tier). That
//! gain is a *cap*, not a command: for real embodied control, each actuator
//! needs its own signal.
//!
//! A [`MotorPlanner`] consumes the observation + the safety-gated gain and
//! produces per-actuator commands matching the platform's actuator count.
//! Platforms (humanoid, quadrotor, manipulator, …) provide their own
//! planners backed by their specific kinematics / dynamics; this crate
//! ships one trivial baseline: [`UniformGainPlanner`].
//!
//! # Example: humanoid with uniform-gain baseline
//!
//! ```ignore
//! use symtropy_robotics_bridge::{PlatformType, RoboticAgent};
//! use symtropy_physics::body::BodyHandle;
//!
//! let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Humanoid, "Bot");
//! let commands = agent.tick_motor_commands(&[0.0; 4], 0.1);
//! assert_eq!(commands.len(), 21); // humanoid has 21 DOF
//! ```
//!
//! # Wiring a Symthaea platform
//!
//! Every `symthaea-<platform>` crate (humanoid, quadruped, manipulator, …)
//! ships a type that implements Symthaea's `EmbodimentBridge` trait. To
//! drive it from Symtropy, write a small [`MotorPlanner`] adapter that calls
//! `EmbodimentBridge::step(thought_hv, dt, phi)` and pulls per-joint
//! commands out of the resulting `EmbodimentResult`. Keeping the adapter in
//! user / platform code avoids pulling `symthaea-core` into this crate's
//! public dep graph.

/// A typed reason why a motor planner rejected the current command request.
///
/// These categories are intentionally platform-neutral; adapters may retain a
/// more specific diagnostic alongside this interoperable status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotorPlannerFault {
    InvalidConfiguration,
    InvalidGain,
    InvalidObservation,
    InvalidDangerLevel,
    AgentNotAuthorized,
    EncodingFailure,
    ActuatorInterfaceMismatch,
    BridgeFailure,
    InvalidOutput,
}

/// Status of the most recent motor-planning request.
///
/// `Ready` only means the software planner contract checks completed; it does
/// not establish that commands are physically safe, authorized by an independent
/// controller, or appropriate for an actual actuator's units and limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotorPlannerStatus {
    /// The most recent request completed without an adapter/planner fault.
    #[default]
    Ready,
    /// The request was rejected; commands must not be treated as accepted work.
    Rejected(MotorPlannerFault),
}

/// Commands and their status returned together to avoid dropping rejection
/// metadata at generic agent boundaries.
///
/// Consumers must inspect `status` before accepting the command vector. On a
/// rejected result the vector is shape-compatible filler, not a physical
/// fallback policy. Use the platform's independent safety mechanism to select
/// a safe response.
#[must_use = "motor plan status must be checked before any actuation"]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MotorPlanResult {
    pub commands: Vec<f64>,
    pub status: MotorPlannerStatus,
}

/// Plan per-actuator motor commands for one tick.
///
/// `observation` is the agent's current sensor reading (as passed to
/// [`RoboticAgent::tick`](crate::RoboticAgent::tick)). `gain` is the
/// NRC-tier safety gain in `[0.0, 1.0]`. `num_actuators` is the target
/// platform's actuator count (see [`PlatformType::num_actuators`](crate::PlatformType::num_actuators)).
///
/// Returned vector **must** have length `num_actuators` — callers will
/// index it by joint id.
pub trait MotorPlanner {
    /// Legacy vector-only planner entrypoint.
    ///
    /// Safety- and integrity-sensitive callers should prefer [`plan_result`],
    /// which validates inputs and outputs and carries typed rejection status.
    fn plan(&mut self, observation: &[f64], gain: f64, num_actuators: usize) -> Vec<f64>;

    /// Run a validated request and preserve planner rejection metadata.
    ///
    /// This default implementation rejects malformed input before calling
    /// `plan`, then checks result status, shape, and finiteness. Implementors
    /// may override it only when they preserve these same invariants.
    fn plan_result(
        &mut self,
        observation: &[f64],
        gain: f64,
        num_actuators: usize,
    ) -> MotorPlanResult {
        let rejected = |fault| MotorPlanResult {
            commands: vec![0.0; num_actuators],
            status: MotorPlannerStatus::Rejected(fault),
        };
        if observation.iter().any(|value| !value.is_finite()) {
            return rejected(MotorPlannerFault::InvalidObservation);
        }
        if !gain.is_finite() || !(0.0..=1.0).contains(&gain) {
            return rejected(MotorPlannerFault::InvalidGain);
        }

        let commands = self.plan(observation, gain, num_actuators);
        let status = self.status();
        if status != MotorPlannerStatus::Ready {
            return MotorPlanResult {
                commands: vec![0.0; num_actuators],
                status,
            };
        }
        if commands.len() != num_actuators || commands.iter().any(|value| !value.is_finite()) {
            return rejected(MotorPlannerFault::InvalidOutput);
        }
        MotorPlanResult { commands, status }
    }

    /// Status corresponding to the most recent call to [`plan`].
    ///
    /// Legacy planners default to `Ready`; fault-aware planners should
    /// override this.
    fn status(&self) -> MotorPlannerStatus {
        MotorPlannerStatus::Ready
    }
}

/// Trivial planner: emit the scalar gain uniformly to every actuator.
///
/// Useful as a baseline or for platforms where per-joint intelligence hasn't
/// been wired yet. NOT a realistic controller — a humanoid walking on this
/// planner will drive every joint with the same torque and fall over.
#[derive(Debug, Default, Clone, Copy)]
pub struct UniformGainPlanner;

impl MotorPlanner for UniformGainPlanner {
    fn plan(&mut self, _observation: &[f64], gain: f64, num_actuators: usize) -> Vec<f64> {
        vec![gain; num_actuators]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_gain_planner_produces_correct_length() {
        let mut p = UniformGainPlanner;
        let cmds = p.plan(&[], 0.5, 21);
        assert_eq!(cmds.len(), 21);
    }

    #[test]
    fn uniform_gain_planner_broadcasts_gain_to_every_joint() {
        let mut p = UniformGainPlanner;
        let cmds = p.plan(&[0.1, 0.2, 0.3], 0.75, 4);
        assert_eq!(cmds, vec![0.75; 4]);
    }

    #[test]
    fn uniform_gain_planner_handles_zero_gain() {
        let mut p = UniformGainPlanner;
        let cmds = p.plan(&[], 0.0, 8);
        assert_eq!(cmds, vec![0.0; 8]);
    }

    #[test]
    fn uniform_gain_planner_handles_zero_actuators() {
        let mut p = UniformGainPlanner;
        let cmds = p.plan(&[], 0.5, 0);
        assert!(cmds.is_empty());
    }

    struct FixturePlanner {
        calls: usize,
        commands: Option<Vec<f64>>,
        planner_status: MotorPlannerStatus,
    }

    impl FixturePlanner {
        fn ready() -> Self {
            Self {
                calls: 0,
                commands: None,
                planner_status: MotorPlannerStatus::Ready,
            }
        }
    }

    impl MotorPlanner for FixturePlanner {
        fn plan(&mut self, _observation: &[f64], _gain: f64, num_actuators: usize) -> Vec<f64> {
            self.calls += 1;
            self.commands
                .clone()
                .unwrap_or_else(|| vec![0.25; num_actuators])
        }

        fn status(&self) -> MotorPlannerStatus {
            self.planner_status
        }
    }

    #[test]
    fn typed_planner_rejects_non_finite_input_before_calling_legacy_plan() {
        let mut planner = FixturePlanner::ready();
        let result = planner.plan_result(&[0.2, f64::NAN], 0.5, 4);

        assert_eq!(planner.calls, 0);
        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidObservation)
        );
    }

    #[test]
    fn typed_planner_rejects_invalid_gain_before_calling_legacy_plan() {
        let mut planner = FixturePlanner::ready();
        let result = planner.plan_result(&[0.2], f64::INFINITY, 4);

        assert_eq!(planner.calls, 0);
        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidGain)
        );
    }

    #[test]
    fn typed_planner_rejects_wrong_output_shape() {
        let mut planner = FixturePlanner::ready();
        planner.commands = Some(vec![0.1; 3]);
        let result = planner.plan_result(&[0.2], 0.5, 4);

        assert_eq!(planner.calls, 1);
        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidOutput)
        );
    }

    #[test]
    fn typed_planner_rejects_non_finite_output() {
        let mut planner = FixturePlanner::ready();
        planner.commands = Some(vec![0.1, 0.2, f64::INFINITY, 0.4]);
        let result = planner.plan_result(&[0.2], 0.5, 4);

        assert_eq!(planner.calls, 1);
        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::InvalidOutput)
        );
    }

    #[test]
    fn typed_planner_does_not_leak_commands_on_reported_rejection() {
        let mut planner = FixturePlanner::ready();
        planner.commands = Some(vec![0.9; 4]);
        planner.planner_status = MotorPlannerStatus::Rejected(MotorPlannerFault::BridgeFailure);
        let result = planner.plan_result(&[0.2], 0.5, 4);

        assert_eq!(result.commands, vec![0.0; 4]);
        assert_eq!(
            result.status,
            MotorPlannerStatus::Rejected(MotorPlannerFault::BridgeFailure)
        );
    }
}
