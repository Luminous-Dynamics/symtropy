// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Reference semantic-intent adapter for a differential-drive/mobile-base robot.
//!
//! This module deliberately stops at the native-controller boundary. It does
//! not calculate wheel torques or bypass a platform planner. A concrete ROS 2,
//! Bevy, or hardware integration must implement DifferentialDriveLocalController
//! and retain its own collision, watchdog, actuator, and emergency-stop layers.

use crate::control::{ControlRequestError, EmbodiedIntent, Pose3};
use crate::dispatch::{
    ControllerDispatchContext, ControllerReceipt, ControllerSink, ControllerSubmitError,
};
use serde::{Deserialize, Serialize};

/// Semantic command accepted by a differential-drive platform's local controller.
///
/// These are task/skill requests, not wheel velocities, motor powers, or torques.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DifferentialDriveCommand {
    NavigateTo {
        target: Pose3,
        max_speed_mps: f64,
    },
    HoldPosition,
    Stabilize,
    /// Ask for the platform's controlled-stop behavior. This is not an emergency stop.
    Stop {
        reason: String,
    },
    RequestAssistance {
        reason: String,
    },
}

/// Identity and lease information that the local controller must preserve in logs.
///
/// The local controller should recheck the expiry against its own monotonic clock
/// immediately before enqueue/application; this adapter check alone cannot close
/// a scheduling race between this call and downstream controller execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandProvenance {
    pub session_id: u64,
    pub sequence: u64,
    pub request_id: u64,
    pub expires_at_ns: u64,
}

/// Monotonic clock contract. Implementations must use the same time domain as
/// ControlRequest issued_at_s and expires_at_s, not wall-clock UTC.
pub trait DifferentialDriveClock {
    fn now_s(&self) -> f64;
}

/// Platform-specific planner/controller boundary.
///
/// Implementations translate semantic goals into their own trajectory/planner
/// interface and must keep independent local safety mechanisms enabled.
pub trait DifferentialDriveLocalController {
    fn submit_goal(
        &mut self,
        command: DifferentialDriveCommand,
        provenance: CommandProvenance,
    ) -> Result<ControllerReceipt, ControllerSubmitError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DifferentialDriveConfigError {
    EmptyFrameId,
    InvalidMaximumSpeed,
    InvalidDefaultSpeed,
}

/// Adapter that maps shared typed intents to a narrow mobile-base command API.
pub struct DifferentialDriveAdapter<C, K> {
    frame_id: String,
    max_speed_mps: f64,
    default_speed_mps: f64,
    clock: K,
    controller: C,
}

impl<C, K> DifferentialDriveAdapter<C, K>
where
    C: DifferentialDriveLocalController,
    K: DifferentialDriveClock,
{
    pub fn new(
        frame_id: impl Into<String>,
        max_speed_mps: f64,
        default_speed_mps: f64,
        clock: K,
        controller: C,
    ) -> Result<Self, DifferentialDriveConfigError> {
        let frame_id = frame_id.into();
        if frame_id.trim().is_empty() {
            return Err(DifferentialDriveConfigError::EmptyFrameId);
        }
        if !max_speed_mps.is_finite() || max_speed_mps <= 0.0 {
            return Err(DifferentialDriveConfigError::InvalidMaximumSpeed);
        }
        if !default_speed_mps.is_finite()
            || default_speed_mps <= 0.0
            || default_speed_mps > max_speed_mps
        {
            return Err(DifferentialDriveConfigError::InvalidDefaultSpeed);
        }

        Ok(Self {
            frame_id,
            max_speed_mps,
            default_speed_mps,
            clock,
            controller,
        })
    }

    fn command_for(
        &self,
        intent: &EmbodiedIntent,
    ) -> Result<DifferentialDriveCommand, ControllerSubmitError> {
        match intent {
            EmbodiedIntent::NavigateTo {
                target,
                max_speed_mps,
            } => {
                if target.frame_id != self.frame_id {
                    return Err(ControllerSubmitError::CapabilityMismatch);
                }
                let speed = max_speed_mps.unwrap_or(self.default_speed_mps);
                if !speed.is_finite() || speed <= 0.0 || speed > self.max_speed_mps {
                    return Err(ControllerSubmitError::CapabilityMismatch);
                }
                Ok(DifferentialDriveCommand::NavigateTo {
                    target: target.clone(),
                    max_speed_mps: speed,
                })
            }
            EmbodiedIntent::HoldPosition => Ok(DifferentialDriveCommand::HoldPosition),
            EmbodiedIntent::Stabilize => Ok(DifferentialDriveCommand::Stabilize),
            EmbodiedIntent::Stop { reason } => Ok(DifferentialDriveCommand::Stop {
                reason: reason.clone(),
            }),
            EmbodiedIntent::RequestAssistance { reason } => {
                Ok(DifferentialDriveCommand::RequestAssistance {
                    reason: reason.clone(),
                })
            }
            EmbodiedIntent::Inspect { .. }
            | EmbodiedIntent::Reach { .. }
            | EmbodiedIntent::Manipulate { .. } => {
                Err(ControllerSubmitError::UnsupportedIntent)
            }
        }
    }
}

impl<C, K> ControllerSink for DifferentialDriveAdapter<C, K>
where
    C: DifferentialDriveLocalController,
    K: DifferentialDriveClock,
{
    fn submit(
        &mut self,
        dispatch: &ControllerDispatchContext<'_>,
    ) -> Result<ControllerReceipt, ControllerSubmitError> {
        // Recheck the original lease at the closest point available to this
        // adapter. Downstream controllers must repeat this check before enqueue.
        let now_s = self.clock.now_s();
        if let Err(error) = dispatch.request.validate_at(now_s) {
            return Err(match error {
                ControlRequestError::Expired => ControllerSubmitError::DeadlineMissed,
                _ => ControllerSubmitError::InvalidRequest,
            });
        }

        let command = self.command_for(&dispatch.request.intent)?;
        let expires_at_ns = seconds_to_monotonic_ns(dispatch.request.expires_at_s)
            .ok_or(ControllerSubmitError::InvalidRequest)?;
        let provenance = CommandProvenance {
            session_id: dispatch.session_id,
            sequence: dispatch.sequence,
            request_id: dispatch.request.request_id,
            expires_at_ns,
        };

        self.controller.submit_goal(command, provenance)
    }
}

/// Convert seconds in the adapter's monotonic time domain to nanoseconds without
/// saturating or wrapping. This does not establish that two clock domains match.
fn seconds_to_monotonic_ns(seconds: f64) -> Option<u64> {
    if !seconds.is_finite() || seconds < 0.0 {
        return None;
    }
    let nanos = seconds * 1_000_000_000.0;
    if !nanos.is_finite() || nanos >= u64::MAX as f64 {
        return None;
    }
    Some(nanos.round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assurance::{
        AxisAlignedWorkspace, EmbodimentEnvelope, IntentKind, RuntimeAssuranceGate,
        RuntimeAssurancePolicy, RuntimeSafetyState,
    };
    use crate::control::ControlRequest;
    use crate::dispatch::{
        AuthenticatedSession, DispatchBoundary, DispatchRejection, DispatchStatus,
        SessionAuthenticationError, SessionAuthenticator,
    };
    use crate::sequence_guard::{RequestSequenceGuard, SequencedRuntimeAssurance};
    use std::cell::Cell;

    struct FakeClock {
        now_s: Cell<f64>,
    }

    impl FakeClock {
        fn new(now_s: f64) -> Self {
            Self {
                now_s: Cell::new(now_s),
            }
        }
    }

    impl DifferentialDriveClock for FakeClock {
        fn now_s(&self) -> f64 {
            self.now_s.get()
        }
    }

    #[derive(Default)]
    struct RecordingController {
        calls: Vec<(DifferentialDriveCommand, CommandProvenance)>,
        failure: Option<ControllerSubmitError>,
    }

    impl DifferentialDriveLocalController for RecordingController {
        fn submit_goal(
            &mut self,
            command: DifferentialDriveCommand,
            provenance: CommandProvenance,
        ) -> Result<ControllerReceipt, ControllerSubmitError> {
            if let Some(error) = self.failure {
                return Err(error);
            }
            self.calls.push((command, provenance));
            Ok(ControllerReceipt {
                ticket: format!("mobile-goal-{}", provenance.request_id),
            })
        }
    }

    struct TestAuthenticator;

    impl SessionAuthenticator for TestAuthenticator {
        type Credential = &'static str;

        fn authenticate(
            &self,
            credential: &Self::Credential,
        ) -> Result<AuthenticatedSession, SessionAuthenticationError> {
            if *credential != "verified-test-credential" {
                return Err(SessionAuthenticationError::InvalidCredential);
            }
            AuthenticatedSession::from_verified_session_id(41)
        }
    }

    fn pose(frame_id: &str, x: f64, y: f64) -> Pose3 {
        Pose3 {
            frame_id: frame_id.into(),
            position_m: [x, y, 0.0],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    fn request(request_id: u64, intent: EmbodiedIntent, expires_at_s: f64) -> ControlRequest {
        ControlRequest {
            request_id,
            issued_at_s: 10.0,
            expires_at_s,
            belief_confidence: 0.9,
            intent,
        }
    }

    fn navigate(request_id: u64, frame_id: &str, speed: Option<f64>) -> ControlRequest {
        request(
            request_id,
            EmbodiedIntent::NavigateTo {
                target: pose(frame_id, 1.0, 2.0),
                max_speed_mps: speed,
            },
            11.0,
        )
    }

    fn adapter(
        now_s: f64,
    ) -> DifferentialDriveAdapter<RecordingController, FakeClock> {
        DifferentialDriveAdapter::new(
            "map",
            1.2,
            0.6,
            FakeClock::new(now_s),
            RecordingController::default(),
        )
        .expect("test adapter configuration is valid")
    }

    fn context<'a>(
        request: &'a ControlRequest,
        session_id: u64,
        sequence: u64,
    ) -> ControllerDispatchContext<'a> {
        ControllerDispatchContext {
            session_id,
            sequence,
            request,
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
                    IntentKind::Stabilize,
                    IntentKind::Stop,
                    IntentKind::RequestAssistance,
                ],
                workspace: AxisAlignedWorkspace {
                    frame_id: "map".into(),
                    min_m: [-5.0, -5.0, -0.1],
                    max_m: [5.0, 5.0, 0.1],
                },
                max_speed_mps: 1.2,
            },
        }
    }

    fn safety_state() -> RuntimeSafetyState {
        RuntimeSafetyState {
            observed_at_s: 10.5,
            sensor_age_s: 0.02,
            actuator_ready: true,
            emergency_stop_active: false,
            communication_healthy: true,
            battery_fraction: 0.8,
        }
    }

    fn boundary(
        now_s: f64,
    ) -> DispatchBoundary<
        TestAuthenticator,
        DifferentialDriveAdapter<RecordingController, FakeClock>,
    > {
        let runtime = SequencedRuntimeAssurance::new(
            gate(),
            RequestSequenceGuard::new(4).expect("positive capacity"),
        );
        DispatchBoundary::new(TestAuthenticator, runtime, adapter(now_s))
    }

    #[test]
    fn navigation_maps_to_a_native_goal_and_preserves_request_provenance() {
        let request = navigate(1, "map", None);
        let mut adapter = adapter(10.5);
        let receipt = adapter
            .submit(&context(&request, 41, 7))
            .expect("valid navigation should be submitted");

        assert_eq!(receipt.ticket, "mobile-goal-1");
        assert_eq!(adapter.controller.calls.len(), 1);
        assert_eq!(
            adapter.controller.calls[0].0,
            DifferentialDriveCommand::NavigateTo {
                target: pose("map", 1.0, 2.0),
                max_speed_mps: 0.6,
            }
        );
        assert_eq!(
            adapter.controller.calls[0].1,
            CommandProvenance {
                session_id: 41,
                sequence: 7,
                request_id: 1,
                expires_at_ns: 11_000_000_000,
            }
        );
    }

    #[test]
    fn lease_is_rechecked_at_adapter_submission_and_expired_request_never_reaches_controller() {
        let request = navigate(2, "map", Some(0.4));
        let mut adapter = adapter(11.1);
        assert_eq!(
            adapter.submit(&context(&request, 41, 1)),
            Err(ControllerSubmitError::DeadlineMissed)
        );
        assert!(adapter.controller.calls.is_empty());
    }

    #[test]
    fn frame_speed_and_unsupported_intents_fail_closed() {
        let mut adapter = adapter(10.5);
        let wrong_frame = navigate(3, "odom", Some(0.5));
        let too_fast = navigate(4, "map", Some(2.0));
        let unsupported = request(
            5,
            EmbodiedIntent::Reach {
                target: pose("map", 0.5, 0.0),
            },
            11.0,
        );

        assert_eq!(
            adapter.submit(&context(&wrong_frame, 41, 1)),
            Err(ControllerSubmitError::CapabilityMismatch)
        );
        assert_eq!(
            adapter.submit(&context(&too_fast, 41, 2)),
            Err(ControllerSubmitError::CapabilityMismatch)
        );
        assert_eq!(
            adapter.submit(&context(&unsupported, 41, 3)),
            Err(ControllerSubmitError::UnsupportedIntent)
        );
        assert!(adapter.controller.calls.is_empty());
    }

    #[test]
    fn invalid_adapter_configuration_is_rejected() {
        assert_eq!(
            DifferentialDriveAdapter::new(
                "map",
                1.0,
                1.1,
                FakeClock::new(10.0),
                RecordingController::default(),
            )
            .err(),
            Some(DifferentialDriveConfigError::InvalidDefaultSpeed)
        );
        assert_eq!(
            DifferentialDriveAdapter::new(
                " ",
                1.0,
                0.5,
                FakeClock::new(10.0),
                RecordingController::default(),
            )
            .err(),
            Some(DifferentialDriveConfigError::EmptyFrameId)
        );
    }

    #[test]
    fn dispatch_boundary_reclassifies_adapter_deadline_rejection_as_rejected_not_controller_failed() {
        let mut boundary = boundary(10.5);
        // Advance the sink clock after admission would be evaluated. The
        // adapter must reject before calling the native-controller trait.
        boundary.controller_mut().clock.now_s.set(11.1);
        let request = navigate(6, "map", Some(0.5));
        let receipt = boundary.dispatch(
            &"verified-test-credential",
            1,
            &request,
            10.5,
            &safety_state(),
        );

        assert_eq!(receipt.status, DispatchStatus::Rejected);
        assert_eq!(
            receipt.rejections,
            vec![DispatchRejection::Controller(
                ControllerSubmitError::DeadlineMissed
            )]
        );
        assert!(boundary.controller().controller.calls.is_empty());
    }

    #[test]
    fn explicit_controlled_stop_is_routed_as_a_semantic_command() {
        let request = request(
            7,
            EmbodiedIntent::Stop {
                reason: "blocked route".into(),
            },
            11.0,
        );
        let mut adapter = adapter(10.5);
        adapter.submit(&context(&request, 41, 1)).unwrap();
        assert_eq!(
            adapter.controller.calls[0].0,
            DifferentialDriveCommand::Stop {
                reason: "blocked route".into()
            }
        );
    }
}
