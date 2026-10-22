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

    #[test]
    fn motor_plan_result_serialization_matches_v1_envelope() {
        let ready = MotorPlanResult {
            commands: vec![0.25, 0.5, 0.75],
            status: MotorPlannerStatus::Ready,
        };
        let value = serde_json::to_value(&ready).expect("result should serialize");
        assert_eq!(value["status"], serde_json::json!("ready"));
        assert_eq!(value["commands"], serde_json::json!([0.25, 0.5, 0.75]));
        let round_trip: MotorPlanResult =
            serde_json::from_value(value).expect("result should deserialize");
        assert_eq!(round_trip, ready);

        let rejected = MotorPlanResult {
            commands: vec![0.0; 3],
            status: MotorPlannerStatus::Rejected(MotorPlannerFault::AgentNotAuthorized),
        };
        let value = serde_json::to_value(&rejected).expect("rejection should serialize");
        assert_eq!(
            value["status"],
            serde_json::json!({"rejected": "agent_not_authorized"})
        );
        let round_trip: MotorPlanResult =
            serde_json::from_value(value).expect("rejection should deserialize");
        assert_eq!(round_trip, rejected);
    }

    #[test]
    fn serialized_fault_names_match_v1_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/motor-plan-result-v1.schema.json"))
                .expect("checked-in motor plan schema must parse");
        let variants = schema["properties"]["status"]["oneOf"]
            .as_array()
            .expect("status schema should use oneOf");
        assert_eq!(variants[0]["const"], serde_json::json!("ready"));
        let allowed = variants[1]["properties"]["rejected"]["enum"]
            .as_array()
            .expect("rejected enum should be present")
            .iter()
            .map(|value| value.as_str().expect("fault name must be a string"))
            .collect::<std::collections::BTreeSet<_>>();
        let faults = [
            MotorPlannerFault::InvalidConfiguration,
            MotorPlannerFault::InvalidGain,
            MotorPlannerFault::InvalidObservation,
            MotorPlannerFault::InvalidDangerLevel,
            MotorPlannerFault::AgentNotAuthorized,
            MotorPlannerFault::EncodingFailure,
            MotorPlannerFault::ActuatorInterfaceMismatch,
            MotorPlannerFault::BridgeFailure,
            MotorPlannerFault::InvalidOutput,
        ];
        for fault in faults {
            let value = serde_json::to_value(MotorPlannerStatus::Rejected(fault))
                .expect("fault status should serialize");
            let name = value["rejected"]
                .as_str()
                .expect("rejected status should serialize a fault name");
            assert!(
                allowed.contains(name),
                "serialized fault {name} is absent from schema"
            );
        }
        assert_eq!(allowed.len(), faults.len());
    }
}

#[cfg(test)]
mod atomfab_workcell_fault_corpus_tests {
    use super::MotorPlannerFault;
    use std::collections::BTreeSet;

    #[derive(serde::Deserialize)]
    struct Corpus {
        schema_version: u16,
        corpus_id: String,
        cases: Vec<Case>,
    }

    #[derive(serde::Deserialize)]
    struct Case {
        case_id: String,
        #[serde(default)]
        adapter_fault: Option<MotorPlannerFault>,
        control: bool,
        expected: Expected,
    }

    #[derive(serde::Deserialize)]
    struct Expected {
        decision: String,
        reason_codes: Vec<String>,
        simulation_action_authorized: bool,
        feasibility_status: String,
        hard_constraint_violation: bool,
    }

    #[test]
    fn shared_atomfab_fault_corpus_maps_adapter_failures_to_typed_faults() {
        let corpus: Corpus = serde_json::from_str(include_str!(
            "../../../../docs/research/ATOMFAB_WORKCELL_FAULT_CORPUS_V1.json"
        ))
        .expect("shared AtomFab workcell fault corpus must parse");

        assert_eq!(corpus.schema_version, 1);
        assert_eq!(corpus.corpus_id, "atomfab-workcell-fault-corpus-v1");
        assert!(corpus.cases.len() >= 15);

        let mut ids = BTreeSet::new();
        for case in &corpus.cases {
            assert!(
                ids.insert(case.case_id.as_str()),
                "duplicate corpus id: {}",
                case.case_id
            );
            if case.expected.decision != "accept" {
                assert!(
                    !case.expected.simulation_action_authorized,
                    "non-accepting case {} authorizes simulated action",
                    case.case_id
                );
                assert!(
                    !case.expected.reason_codes.is_empty(),
                    "rejected/abstained case {} lacks a reason code",
                    case.case_id
                );
            }
            if case.expected.feasibility_status == "feasible" {
                assert_eq!(
                    case.expected.decision, "accept",
                    "case {} is feasible without an accepted decision",
                    case.case_id
                );
            }
            if case.expected.hard_constraint_violation {
                assert_ne!(case.expected.decision, "accept");
                assert_eq!(case.expected.feasibility_status, "infeasible");
            }
        }

        let required_adapter_faults = [
            ("stale-observation", MotorPlannerFault::InvalidObservation),
            (
                "non-finite-observation",
                MotorPlannerFault::InvalidObservation,
            ),
            ("invalid-planner-gain", MotorPlannerFault::InvalidGain),
            (
                "actuator-count-mismatch",
                MotorPlannerFault::ActuatorInterfaceMismatch,
            ),
            ("bridge-timeout", MotorPlannerFault::BridgeFailure),
            ("malformed-planner-output", MotorPlannerFault::InvalidOutput),
        ];
        for (case_id, expected_fault) in required_adapter_faults {
            let case = corpus
                .cases
                .iter()
                .find(|case| case.case_id == case_id)
                .unwrap_or_else(|| panic!("missing adapter fault case {case_id}"));
            assert_eq!(
                case.adapter_fault,
                Some(expected_fault),
                "typed fault mismatch for {case_id}"
            );
            assert_eq!(case.expected.decision, "reject", "{case_id} must reject");
            assert!(
                !case.expected.simulation_action_authorized,
                "{case_id} must not authorize simulated action"
            );
            assert!(
                !case.control,
                "fault case {case_id} must not be marked as control"
            );
        }

        let mapped_count = corpus
            .cases
            .iter()
            .filter(|case| case.adapter_fault.is_some())
            .count();
        assert_eq!(mapped_count, required_adapter_faults.len());

        assert!(
            corpus.cases.iter().any(|case| {
                case.control
                    && case.expected.decision == "accept"
                    && case.expected.simulation_action_authorized
            }),
            "the corpus must include an accepted control; reject-all is not valid"
        );
    }
}
