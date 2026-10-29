// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic admission gate for cognitive requests before platform dispatch.
//!
//! This gate checks request freshness, declared embodiment capabilities and a
//! small set of independently observed health constraints. It is a reference
//! runtime-assurance layer, NOT a trajectory validator, collision checker,
//! stability proof, certified safety function, or substitute for local control.

use crate::control::{ControlRequest, EmbodiedIntent};
use serde::{Deserialize, Serialize};

const TIME_EPSILON_S: f64 = 1.0e-9;

/// Platform-level semantic operation that an adapter explicitly supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntentKind {
    NavigateTo,
    HoldPosition,
    Inspect,
    Reach,
    Manipulate,
    Stabilize,
    Stop,
    RequestAssistance,
}

impl From<&EmbodiedIntent> for IntentKind {
    fn from(intent: &EmbodiedIntent) -> Self {
        match intent {
            EmbodiedIntent::NavigateTo { .. } => Self::NavigateTo,
            EmbodiedIntent::HoldPosition => Self::HoldPosition,
            EmbodiedIntent::Inspect { .. } => Self::Inspect,
            EmbodiedIntent::Reach { .. } => Self::Reach,
            EmbodiedIntent::Manipulate { .. } => Self::Manipulate,
            EmbodiedIntent::Stabilize => Self::Stabilize,
            EmbodiedIntent::Stop { .. } => Self::Stop,
            EmbodiedIntent::RequestAssistance { .. } => Self::RequestAssistance,
        }
    }
}

/// Static axis-aligned workspace expressed in one named reference frame.
///
/// Bounds are inclusive and apply to semantic target positions only. They do
/// not prove that the path to a target is collision-free.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisAlignedWorkspace {
    pub frame_id: String,
    pub min_m: [f64; 3],
    pub max_m: [f64; 3],
}

impl AxisAlignedWorkspace {
    fn is_valid(&self) -> bool {
        !self.frame_id.trim().is_empty()
            && self.min_m.iter().all(|v| v.is_finite())
            && self.max_m.iter().all(|v| v.is_finite())
            && (0..3).all(|i| self.min_m[i] <= self.max_m[i])
    }

    fn contains(&self, position_m: [f64; 3]) -> bool {
        (0..3).all(|i| position_m[i] >= self.min_m[i] && position_m[i] <= self.max_m[i])
    }
}

/// Declared adapter capabilities and hard request limits.
///
/// A missing per-request navigation speed means the adapter must choose a
/// default no greater than max_speed_mps; it does not mean unlimited speed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbodimentEnvelope {
    pub supported_intents: Vec<IntentKind>,
    pub workspace: AxisAlignedWorkspace,
    pub max_speed_mps: f64,
}

impl EmbodimentEnvelope {
    fn is_valid(&self) -> bool {
        !self.supported_intents.is_empty()
            && self.workspace.is_valid()
            && self.max_speed_mps.is_finite()
            && self.max_speed_mps > 0.0
    }
}

/// Independently observed platform health at a monotonic timestamp.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeSafetyState {
    pub observed_at_s: f64,
    pub sensor_age_s: f64,
    pub actuator_ready: bool,
    pub emergency_stop_active: bool,
    pub communication_healthy: bool,
    pub battery_fraction: f32,
}

/// Runtime policy thresholds. These are project configuration, not standards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAssurancePolicy {
    pub max_sensor_age_s: f64,
    pub max_snapshot_age_s: f64,
    pub minimum_battery_fraction: f32,
    /// Set false for platforms whose cognitive request path is fully onboard.
    pub require_healthy_communication: bool,
}

impl RuntimeAssurancePolicy {
    fn is_valid(&self) -> bool {
        self.max_sensor_age_s.is_finite()
            && self.max_sensor_age_s >= 0.0
            && self.max_snapshot_age_s.is_finite()
            && self.max_snapshot_age_s >= 0.0
            && self.minimum_battery_fraction.is_finite()
            && (0.0..=1.0).contains(&self.minimum_battery_fraction)
    }
}

/// Machine-readable admission failures. A vetoed result must never be
/// translated into a motor command by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VetoReason {
    InvalidRequest,
    InvalidPolicy,
    InvalidEnvelope,
    InvalidSafetyState,
    StaleSnapshot,
    StaleSensors,
    EmergencyStopActive,
    ActuatorUnavailable,
    CommunicationUnavailable,
    BatteryBelowReserve,
    UnsupportedIntent,
    FrameMismatch,
    TargetOutsideWorkspace,
    SpeedLimitExceeded,
}

/// Admission decision only. permitted means eligible for platform-adapter
/// dispatch; it does NOT mean physically safe or authorized for direct actuation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateDecision {
    pub request_id: u64,
    pub permitted: bool,
    pub reasons: Vec<VetoReason>,
}

/// Reference runtime-assurance gate, independent of the cognitive policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeAssuranceGate {
    pub policy: RuntimeAssurancePolicy,
    pub envelope: EmbodimentEnvelope,
}

impl RuntimeAssuranceGate {
    /// Evaluate a request against the current monotonic time and independent state.
    ///
    /// Stop and request-assistance intents are allowed through degraded health
    /// conditions when supported by the adapter, so the local controller can
    /// choose its platform-defined containment behavior. The gate never directly
    /// actuates hardware and must be paired with continuous controller-side limits.
    pub fn evaluate(
        &self,
        request: &ControlRequest,
        now_s: f64,
        state: &RuntimeSafetyState,
    ) -> GateDecision {
        let mut reasons = Vec::new();

        if request.validate_at(now_s).is_err() {
            return Self::veto(request.request_id, vec![VetoReason::InvalidRequest]);
        }
        if !self.policy.is_valid() {
            return Self::veto(request.request_id, vec![VetoReason::InvalidPolicy]);
        }
        if !self.envelope.is_valid() {
            return Self::veto(request.request_id, vec![VetoReason::InvalidEnvelope]);
        }
        if !now_s.is_finite()
            || now_s < 0.0
            || !state.observed_at_s.is_finite()
            || state.observed_at_s < 0.0
            || state.observed_at_s > now_s + TIME_EPSILON_S
            || !state.sensor_age_s.is_finite()
            || state.sensor_age_s < 0.0
            || !state.battery_fraction.is_finite()
            || !(0.0..=1.0).contains(&state.battery_fraction)
        {
            return Self::veto(request.request_id, vec![VetoReason::InvalidSafetyState]);
        }

        if !self
            .envelope
            .supported_intents
            .contains(&IntentKind::from(&request.intent))
        {
            reasons.push(VetoReason::UnsupportedIntent);
        }

        if let Some(target) = target_pose(&request.intent) {
            if target.frame_id != self.envelope.workspace.frame_id {
                reasons.push(VetoReason::FrameMismatch);
            } else if !self.envelope.workspace.contains(target.position_m) {
                reasons.push(VetoReason::TargetOutsideWorkspace);
            }
        }

        if let EmbodiedIntent::NavigateTo {
            max_speed_mps: Some(speed),
            ..
        } = &request.intent
        {
            if *speed > self.envelope.max_speed_mps {
                reasons.push(VetoReason::SpeedLimitExceeded);
            }
        }

        // These semantic requests are routed to the adapter for containment or
        // escalation even when normal motion prerequisites are degraded.
        let degraded_mode_intent = matches!(
            &request.intent,
            EmbodiedIntent::Stop { .. } | EmbodiedIntent::RequestAssistance { .. }
        );

        if !degraded_mode_intent {
            if now_s - state.observed_at_s > self.policy.max_snapshot_age_s + TIME_EPSILON_S {
                reasons.push(VetoReason::StaleSnapshot);
            }
            if state.sensor_age_s > self.policy.max_sensor_age_s + TIME_EPSILON_S {
                reasons.push(VetoReason::StaleSensors);
            }
            if state.emergency_stop_active {
                reasons.push(VetoReason::EmergencyStopActive);
            }
            if !state.actuator_ready {
                reasons.push(VetoReason::ActuatorUnavailable);
            }
            if self.policy.require_healthy_communication && !state.communication_healthy {
                reasons.push(VetoReason::CommunicationUnavailable);
            }
            if state.battery_fraction < self.policy.minimum_battery_fraction {
                reasons.push(VetoReason::BatteryBelowReserve);
            }
        }

        if reasons.is_empty() {
            GateDecision {
                request_id: request.request_id,
                permitted: true,
                reasons,
            }
        } else {
            Self::veto(request.request_id, reasons)
        }
    }

    fn veto(request_id: u64, reasons: Vec<VetoReason>) -> GateDecision {
        GateDecision {
            request_id,
            permitted: false,
            reasons,
        }
    }
}

fn target_pose(intent: &EmbodiedIntent) -> Option<&crate::control::Pose3> {
    match intent {
        EmbodiedIntent::NavigateTo { target, .. }
        | EmbodiedIntent::Inspect { target }
        | EmbodiedIntent::Reach { target }
        | EmbodiedIntent::Manipulate { target, .. } => Some(target),
        EmbodiedIntent::HoldPosition
        | EmbodiedIntent::Stabilize
        | EmbodiedIntent::Stop { .. }
        | EmbodiedIntent::RequestAssistance { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::{ManipulationOperation, Pose3};

    fn pose(frame_id: &str, xyz: [f64; 3]) -> Pose3 {
        Pose3 {
            frame_id: frame_id.into(),
            position_m: xyz,
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    fn request(intent: EmbodiedIntent) -> ControlRequest {
        ControlRequest {
            request_id: 41,
            issued_at_s: 10.0,
            expires_at_s: 11.0,
            belief_confidence: 0.9,
            intent,
        }
    }

    fn gate() -> RuntimeAssuranceGate {
        RuntimeAssuranceGate {
            policy: RuntimeAssurancePolicy {
                max_sensor_age_s: 0.1,
                max_snapshot_age_s: 0.2,
                minimum_battery_fraction: 0.15,
                require_healthy_communication: true,
            },
            envelope: EmbodimentEnvelope {
                supported_intents: vec![
                    IntentKind::NavigateTo,
                    IntentKind::HoldPosition,
                    IntentKind::Inspect,
                    IntentKind::Reach,
                    IntentKind::Manipulate,
                    IntentKind::Stabilize,
                    IntentKind::Stop,
                    IntentKind::RequestAssistance,
                ],
                workspace: AxisAlignedWorkspace {
                    frame_id: "map".into(),
                    min_m: [-5.0, -5.0, 0.0],
                    max_m: [5.0, 5.0, 10.0],
                },
                max_speed_mps: 2.0,
            },
        }
    }

    fn healthy_state() -> RuntimeSafetyState {
        RuntimeSafetyState {
            observed_at_s: 10.4,
            sensor_age_s: 0.02,
            actuator_ready: true,
            emergency_stop_active: false,
            communication_healthy: true,
            battery_fraction: 0.8,
        }
    }

    fn navigate(xyz: [f64; 3], speed: Option<f64>) -> ControlRequest {
        request(EmbodiedIntent::NavigateTo {
            target: pose("map", xyz),
            max_speed_mps: speed,
        })
    }

    #[test]
    fn admits_fresh_request_inside_declared_envelope() {
        let decision = gate().evaluate(
            &navigate([1.0, 2.0, 3.0], Some(1.5)),
            10.5,
            &healthy_state(),
        );
        assert!(decision.permitted);
        assert!(decision.reasons.is_empty());
        assert_eq!(decision.request_id, 41);
    }

    #[test]
    fn expired_intent_is_vetoed_before_platform_dispatch() {
        let decision = gate().evaluate(
            &navigate([1.0, 2.0, 3.0], Some(1.5)),
            11.1,
            &healthy_state(),
        );
        assert_eq!(decision.reasons, vec![VetoReason::InvalidRequest]);
        assert!(!decision.permitted);
    }

    #[test]
    fn stale_sensors_veto_motion() {
        let mut state = healthy_state();
        state.sensor_age_s = 0.11;
        assert!(
            gate()
                .evaluate(&navigate([1.0, 2.0, 3.0], Some(1.5)), 10.5, &state)
                .reasons
                .contains(&VetoReason::StaleSensors)
        );
    }

    #[test]
    fn emergency_stop_vetoes_motion() {
        let mut state = healthy_state();
        state.emergency_stop_active = true;
        assert!(
            gate()
                .evaluate(&navigate([1.0, 2.0, 3.0], Some(1.5)), 10.5, &state)
                .reasons
                .contains(&VetoReason::EmergencyStopActive)
        );
    }

    #[test]
    fn actuator_failure_vetoes_motion() {
        let mut state = healthy_state();
        state.actuator_ready = false;
        assert!(
            gate()
                .evaluate(&navigate([1.0, 2.0, 3.0], Some(1.5)), 10.5, &state)
                .reasons
                .contains(&VetoReason::ActuatorUnavailable)
        );
    }

    #[test]
    fn battery_below_reserve_vetoes_motion() {
        let mut state = healthy_state();
        state.battery_fraction = 0.1;
        assert!(
            gate()
                .evaluate(&navigate([1.0, 2.0, 3.0], Some(1.5)), 10.5, &state)
                .reasons
                .contains(&VetoReason::BatteryBelowReserve)
        );
    }

    #[test]
    fn communication_loss_vetoes_when_policy_requires_link() {
        let mut state = healthy_state();
        state.communication_healthy = false;
        assert!(
            gate()
                .evaluate(&navigate([1.0, 2.0, 3.0], Some(1.5)), 10.5, &state)
                .reasons
                .contains(&VetoReason::CommunicationUnavailable)
        );
    }

    #[test]
    fn unsupported_intent_is_explicitly_vetoed() {
        let mut g = gate();
        g.envelope
            .supported_intents
            .retain(|kind| *kind != IntentKind::Manipulate);
        let manipulation = request(EmbodiedIntent::Manipulate {
            target: pose("map", [0.0, 0.0, 1.0]),
            operation: ManipulationOperation::Grasp,
        });
        assert!(
            g.evaluate(&manipulation, 10.5, &healthy_state())
                .reasons
                .contains(&VetoReason::UnsupportedIntent)
        );
    }

    #[test]
    fn unknown_frame_is_vetoed_not_guessed() {
        let inspect = request(EmbodiedIntent::Inspect {
            target: pose("camera_local", [1.0, 2.0, 3.0]),
        });
        assert!(
            gate()
                .evaluate(&inspect, 10.5, &healthy_state())
                .reasons
                .contains(&VetoReason::FrameMismatch)
        );
    }

    #[test]
    fn workspace_and_speed_limits_fail_closed() {
        let outside = gate().evaluate(
            &navigate([50.0, 0.0, 3.0], Some(1.0)),
            10.5,
            &healthy_state(),
        );
        assert!(
            outside
                .reasons
                .contains(&VetoReason::TargetOutsideWorkspace)
        );
        let too_fast = gate().evaluate(
            &navigate([1.0, 0.0, 3.0], Some(2.1)),
            10.5,
            &healthy_state(),
        );
        assert!(too_fast.reasons.contains(&VetoReason::SpeedLimitExceeded));
    }

    #[test]
    fn stop_is_routed_for_containment_during_degraded_health() {
        let mut state = healthy_state();
        state.sensor_age_s = 99.0;
        state.actuator_ready = false;
        state.emergency_stop_active = true;
        state.communication_healthy = false;
        state.battery_fraction = 0.0;
        let stop = request(EmbodiedIntent::Stop {
            reason: "runtime assurance test".into(),
        });
        let decision = gate().evaluate(&stop, 10.5, &state);
        assert!(decision.permitted);
    }

    #[test]
    fn invalid_envelope_fails_closed() {
        let mut g = gate();
        g.envelope.workspace.max_m[0] = f64::NAN;
        let decision = g.evaluate(
            &navigate([1.0, 2.0, 3.0], Some(1.0)),
            10.5,
            &healthy_state(),
        );
        assert_eq!(decision.reasons, vec![VetoReason::InvalidEnvelope]);
    }

    #[test]
    fn stale_safety_snapshot_vetoes_motion() {
        let mut state = healthy_state();
        state.observed_at_s = 10.0;
        assert!(
            gate()
                .evaluate(&navigate([1.0, 2.0, 3.0], Some(1.0)), 10.5, &state)
                .reasons
                .contains(&VetoReason::StaleSnapshot)
        );
    }
}
