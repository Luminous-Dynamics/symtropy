// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Serialized admission-to-controller boundary for embodied requests.
//!
//! This layer composes session authentication, request sequencing, generic
//! runtime admission, and one synchronous controller submission. It is a
//! reference conformance boundary, not a certified safety mechanism.
//!
//! The same non-cloned boundary instance must own sequence state and the
//! controller sink. Call dispatch through one exclusive mutable borrow (or a
//! mutex around this whole object) so sequence admission and sink submission
//! cannot race within that instance. Multi-process or persistent anti-replay
//! needs an external transactional authority and is not implemented here.

use crate::assurance::{RuntimeSafetyState, VetoReason};
use crate::control::{ControlRequest, EmbodiedIntent};
use crate::sequence_guard::{SequenceGuardError, SequencedRuntimeAssurance};
use serde::{Deserialize, Serialize};

/// Token/credential validation failed before the request entered the sequence
/// ledger. The implementation is supplied by the hosting transport or adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionAuthenticationError {
    InvalidCredential,
    RevokedSession,
    ProviderUnavailable,
}

/// Opaque identity returned only after the configured authenticator accepts a
/// credential. This is an application-level trust seam, not a cryptographic
/// token by itself; the hosting authenticator must actually verify the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedSession {
    session_id: u64,
}

impl AuthenticatedSession {
    /// Construct only inside a trusted SessionAuthenticator implementation
    /// after its credential verification has succeeded.
    pub fn from_verified_session_id(session_id: u64) -> Result<Self, SessionAuthenticationError> {
        if session_id == 0 {
            return Err(SessionAuthenticationError::InvalidCredential);
        }
        Ok(Self { session_id })
    }

    pub fn session_id(&self) -> u64 {
        self.session_id
    }
}

/// Hosting integration point for authentication and revocation.
pub trait SessionAuthenticator {
    type Credential;

    fn authenticate(
        &self,
        credential: &Self::Credential,
    ) -> Result<AuthenticatedSession, SessionAuthenticationError>;
}

/// Stable result categories from the controller adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControllerSubmitError {
    RejectedByController,
    ControllerUnavailable,
    DeadlineMissed,
    InternalFailure,
}

/// A receipt that the adapter accepted the request for execution.
/// This is not proof that the physical task completed successfully.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerReceipt {
    pub ticket: String,
}

/// The only controller-facing interface exposed by this boundary.
/// Implementations translate semantic intents into a platform-native local
/// controller request; they must not interpret these intents as motor vectors.
pub trait ControllerSink {
    fn submit(
        &mut self,
        request_id: u64,
        intent: &EmbodiedIntent,
    ) -> Result<ControllerReceipt, ControllerSubmitError>;
}

/// Why a request was not submitted, or why the adapter did not accept it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispatchRejection {
    Authentication(SessionAuthenticationError),
    Sequence(SequenceGuardError),
    Runtime(VetoReason),
    Controller(ControllerSubmitError),
    EmptyControllerTicket,
}

/// Status at the adapter submission boundary. Submitted means a controller
/// ticket was returned; only a later validated ControlOutcome can establish
/// execution/completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DispatchStatus {
    Rejected,
    Submitted,
    ControllerFailed,
}

/// Audit receipt for one attempted dispatch. Authentication failures do not
/// reveal an authenticated session ID. The observation timestamp is absent if
/// the caller supplied an invalid clock value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DispatchReceipt {
    pub request_id: u64,
    pub session_id: Option<u64>,
    pub sequence: u64,
    pub observed_at_s: Option<f64>,
    pub status: DispatchStatus,
    pub rejections: Vec<DispatchRejection>,
    pub controller_ticket: Option<String>,
}

/// One mutable owner for authentication, sequence state, gate and controller.
/// Its dispatch method is synchronous by design to make ordering explicit.
pub struct DispatchBoundary<A, C> {
    authenticator: A,
    runtime: SequencedRuntimeAssurance,
    controller: C,
}

impl<A, C> DispatchBoundary<A, C>
where
    A: SessionAuthenticator,
    C: ControllerSink,
{
    pub fn new(authenticator: A, runtime: SequencedRuntimeAssurance, controller: C) -> Self {
        Self {
            authenticator,
            runtime,
            controller,
        }
    }

    /// Authenticate, sequence-check, evaluate safety admission and only then
    /// call the controller sink. The whole method is serialized by the
    /// exclusive mutable borrow of this boundary instance.
    pub fn dispatch(
        &mut self,
        credential: &A::Credential,
        sequence: u64,
        request: &ControlRequest,
        now_s: f64,
        safety_state: &RuntimeSafetyState,
    ) -> DispatchReceipt {
        let session = match self.authenticator.authenticate(credential) {
            Ok(session) => session,
            Err(error) => {
                return DispatchReceipt {
                    request_id: request.request_id,
                    session_id: None,
                    sequence,
                    observed_at_s: valid_time(now_s),
                    status: DispatchStatus::Rejected,
                    rejections: vec![DispatchRejection::Authentication(error)],
                    controller_ticket: None,
                };
            }
        };

        let session_id = session.session_id();
        let decision = match self.runtime.evaluate(
            session_id,
            sequence,
            request,
            now_s,
            safety_state,
        ) {
            Ok(decision) => decision,
            Err(error) => {
                return DispatchReceipt {
                    request_id: request.request_id,
                    session_id: Some(session_id),
                    sequence,
                    observed_at_s: valid_time(now_s),
                    status: DispatchStatus::Rejected,
                    rejections: vec![DispatchRejection::Sequence(error)],
                    controller_ticket: None,
                };
            }
        };

        if !decision.permitted {
            return DispatchReceipt {
                request_id: request.request_id,
                session_id: Some(session_id),
                sequence,
                observed_at_s: valid_time(now_s),
                status: DispatchStatus::Rejected,
                rejections: decision
                    .reasons
                    .into_iter()
                    .map(DispatchRejection::Runtime)
                    .collect(),
                controller_ticket: None,
            };
        }

        match self.controller.submit(request.request_id, &request.intent) {
            Ok(receipt) if !receipt.ticket.trim().is_empty() => DispatchReceipt {
                request_id: request.request_id,
                session_id: Some(session_id),
                sequence,
                observed_at_s: valid_time(now_s),
                status: DispatchStatus::Submitted,
                rejections: Vec::new(),
                controller_ticket: Some(receipt.ticket),
            },
            Ok(_) => DispatchReceipt {
                request_id: request.request_id,
                session_id: Some(session_id),
                sequence,
                observed_at_s: valid_time(now_s),
                status: DispatchStatus::ControllerFailed,
                rejections: vec![DispatchRejection::EmptyControllerTicket],
                controller_ticket: None,
            },
            Err(error) => DispatchReceipt {
                request_id: request.request_id,
                session_id: Some(session_id),
                sequence,
                observed_at_s: valid_time(now_s),
                status: DispatchStatus::ControllerFailed,
                rejections: vec![DispatchRejection::Controller(error)],
                controller_ticket: None,
            },
        }
    }

    /// Inspect the controller for diagnostics/tests without transferring ownership.
    pub fn controller(&self) -> &C {
        &self.controller
    }

    /// Access mutable controller diagnostics. Production adapters should keep
    /// safety-relevant controller state private to this authority boundary.
    pub fn controller_mut(&mut self) -> &mut C {
        &mut self.controller
    }

    /// Number of currently tracked session sequence watermarks.
    pub fn tracked_sessions(&self) -> usize {
        self.runtime.sequence_guard.tracked_sessions()
    }
}

fn valid_time(now_s: f64) -> Option<f64> {
    if now_s.is_finite() && now_s >= 0.0 {
        Some(now_s)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assurance::{
        AxisAlignedWorkspace, EmbodimentEnvelope, IntentKind, RuntimeAssuranceGate,
        RuntimeAssurancePolicy,
    };
    use crate::control::Pose3;
    use crate::sequence_guard::RequestSequenceGuard;
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Clone, Copy)]
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
            AuthenticatedSession::from_verified_session_id(7)
        }
    }

    #[derive(Debug, Default)]
    struct RecordingController {
        calls: Vec<u64>,
        fail: bool,
        empty_ticket: bool,
    }

    impl ControllerSink for RecordingController {
        fn submit(
            &mut self,
            request_id: u64,
            _intent: &EmbodiedIntent,
        ) -> Result<ControllerReceipt, ControllerSubmitError> {
            self.calls.push(request_id);
            if self.fail {
                return Err(ControllerSubmitError::ControllerUnavailable);
            }
            Ok(ControllerReceipt {
                ticket: if self.empty_ticket {
                    String::new()
                } else {
                    format!("controller-{request_id}")
                },
            })
        }
    }

    fn make_boundary() -> DispatchBoundary<TestAuthenticator, RecordingController> {
        let gate = RuntimeAssuranceGate {
            policy: RuntimeAssurancePolicy {
                max_sensor_age_s: 0.1,
                max_snapshot_age_s: 0.2,
                minimum_battery_fraction: 0.15,
                require_healthy_communication: true,
            },
            envelope: EmbodimentEnvelope {
                supported_intents: vec![IntentKind::NavigateTo],
                workspace: AxisAlignedWorkspace {
                    frame_id: "map".into(),
                    min_m: [-5.0, -5.0, 0.0],
                    max_m: [5.0, 5.0, 10.0],
                },
                max_speed_mps: 2.0,
            },
        };
        let runtime = SequencedRuntimeAssurance::new(
            gate,
            RequestSequenceGuard::new(8).expect("positive test capacity"),
        );
        DispatchBoundary::new(TestAuthenticator, runtime, RecordingController::default())
    }

    fn request(request_id: u64, expires_at_s: f64, xyz: [f64; 3], speed: f64) -> ControlRequest {
        ControlRequest {
            request_id,
            issued_at_s: 10.0,
            expires_at_s,
            belief_confidence: 0.9,
            intent: EmbodiedIntent::NavigateTo {
                target: Pose3 {
                    frame_id: "map".into(),
                    position_m: xyz,
                    orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                },
                max_speed_mps: Some(speed),
            },
        }
    }

    fn state() -> RuntimeSafetyState {
        RuntimeSafetyState {
            observed_at_s: 10.4,
            sensor_age_s: 0.02,
            actuator_ready: true,
            emergency_stop_active: false,
            communication_healthy: true,
            battery_fraction: 0.8,
        }
    }

    fn dispatch(
        boundary: &mut DispatchBoundary<TestAuthenticator, RecordingController>,
        credential: &'static str,
        sequence: u64,
        request: &ControlRequest,
        now_s: f64,
        state: &RuntimeSafetyState,
    ) -> DispatchReceipt {
        boundary.dispatch(&credential, sequence, request, now_s, state)
    }

    #[test]
    fn admissible_request_reaches_controller_exactly_once() {
        let mut boundary = make_boundary();
        let receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            1,
            &request(101, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &state(),
        );
        assert_eq!(receipt.status, DispatchStatus::Submitted);
        assert_eq!(receipt.controller_ticket.as_deref(), Some("controller-101"));
        assert_eq!(boundary.controller().calls, vec![101]);
    }

    #[test]
    fn invalid_credential_never_reaches_controller_or_consumes_session() {
        let mut boundary = make_boundary();
        let receipt = dispatch(
            &mut boundary,
            "unverified",
            1,
            &request(102, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &state(),
        );
        assert_eq!(receipt.status, DispatchStatus::Rejected);
        assert_eq!(receipt.rejections, vec![DispatchRejection::Authentication(
            SessionAuthenticationError::InvalidCredential
        )]);
        assert!(boundary.controller().calls.is_empty());
        assert_eq!(boundary.tracked_sessions(), 0);
    }

    #[test]
    fn expired_request_never_reaches_controller_and_consumes_sequence() {
        let mut boundary = make_boundary();
        let expired = request(103, 10.2, [1.0, 1.0, 2.0], 1.0);
        let first = dispatch(
            &mut boundary,
            "verified-test-credential",
            1,
            &expired,
            10.5,
            &state(),
        );
        assert_eq!(first.status, DispatchStatus::Rejected);
        assert!(boundary.controller().calls.is_empty());
        let replay = dispatch(
            &mut boundary,
            "verified-test-credential",
            1,
            &request(104, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &state(),
        );
        assert_eq!(replay.status, DispatchStatus::Rejected);
        assert!(matches!(
            replay.rejections.as_slice(),
            [DispatchRejection::Sequence(SequenceGuardError::DuplicateOrOutOfOrder { .. })]
        ));
        assert!(boundary.controller().calls.is_empty());
    }

    #[test]
    fn stale_sensors_and_emergency_stop_never_reach_controller() {
        let mut boundary = make_boundary();
        let mut stale = state();
        stale.sensor_age_s = 1.0;
        let stale_receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            1,
            &request(105, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &stale,
        );
        assert_eq!(stale_receipt.status, DispatchStatus::Rejected);

        let mut stopped = state();
        stopped.emergency_stop_active = true;
        let stopped_receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            2,
            &request(106, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &stopped,
        );
        assert_eq!(stopped_receipt.status, DispatchStatus::Rejected);
        assert!(boundary.controller().calls.is_empty());
    }

    #[test]
    fn workspace_frame_and_speed_violations_never_reach_controller() {
        let mut boundary = make_boundary();
        let out_of_bounds = request(107, 11.0, [50.0, 1.0, 2.0], 1.0);
        let wrong_frame = ControlRequest {
            intent: EmbodiedIntent::NavigateTo {
                target: Pose3 {
                    frame_id: "camera".into(),
                    position_m: [1.0, 1.0, 2.0],
                    orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
                },
                max_speed_mps: Some(1.0),
            },
            ..request(108, 11.0, [1.0, 1.0, 2.0], 1.0)
        };
        let too_fast = request(109, 11.0, [1.0, 1.0, 2.0], 9.0);

        for (sequence, request) in [(1, out_of_bounds), (2, wrong_frame), (3, too_fast)] {
            let receipt = dispatch(
                &mut boundary,
                "verified-test-credential",
                sequence,
                &request,
                10.5,
                &state(),
            );
            assert_eq!(receipt.status, DispatchStatus::Rejected);
        }
        assert!(boundary.controller().calls.is_empty());
    }

    #[test]
    fn duplicate_and_out_of_order_sequences_never_reach_controller_twice() {
        let mut boundary = make_boundary();
        let ok = request(110, 11.0, [1.0, 1.0, 2.0], 1.0);
        assert_eq!(
            dispatch(&mut boundary, "verified-test-credential", 5, &ok, 10.5, &state()).status,
            DispatchStatus::Submitted
        );
        assert_eq!(
            dispatch(&mut boundary, "verified-test-credential", 5, &ok, 10.5, &state()).status,
            DispatchStatus::Rejected
        );
        assert_eq!(
            dispatch(&mut boundary, "verified-test-credential", 4, &ok, 10.5, &state()).status,
            DispatchStatus::Rejected
        );
        assert_eq!(boundary.controller().calls, vec![110]);
    }

    #[test]
    fn controller_failure_is_never_reported_as_submitted() {
        let mut boundary = make_boundary();
        boundary.controller_mut().fail = true;
        let receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            1,
            &request(111, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &state(),
        );
        assert_eq!(receipt.status, DispatchStatus::ControllerFailed);
        assert_eq!(
            receipt.rejections,
            vec![DispatchRejection::Controller(ControllerSubmitError::ControllerUnavailable)]
        );
        assert!(receipt.controller_ticket.is_none());
        assert_eq!(boundary.controller().calls, vec![111]);
    }

    #[test]
    fn empty_controller_ticket_is_not_reported_as_submitted() {
        let mut boundary = make_boundary();
        boundary.controller_mut().empty_ticket = true;
        let receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            1,
            &request(112, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &state(),
        );
        assert_eq!(receipt.status, DispatchStatus::ControllerFailed);
        assert_eq!(receipt.rejections, vec![DispatchRejection::EmptyControllerTicket]);
        assert!(receipt.controller_ticket.is_none());
    }

    #[test]
    fn concurrent_duplicate_dispatches_call_controller_at_most_once() {
        let boundary = Arc::new(Mutex::new(make_boundary()));
        let mut joins = Vec::new();
        for _ in 0..2 {
            let boundary = Arc::clone(&boundary);
            joins.push(std::thread::spawn(move || {
                let mut boundary = boundary.lock().expect("boundary mutex");
                dispatch(
                    &mut boundary,
                    "verified-test-credential",
                    1,
                    &request(113, 11.0, [1.0, 1.0, 2.0], 1.0),
                    10.5,
                    &state(),
                )
            }));
        }
        let receipts: Vec<_> = joins.into_iter().map(|join| join.join().unwrap()).collect();
        assert_eq!(
            receipts.iter().filter(|r| r.status == DispatchStatus::Submitted).count(),
            1
        );
        assert_eq!(
            receipts.iter().filter(|r| r.status == DispatchStatus::Rejected).count(),
            1
        );
        assert_eq!(boundary.lock().unwrap().controller().calls, vec![113]);
    }

    #[test]
    fn actuator_battery_and_required_link_faults_never_reach_controller() {
        let mut boundary = make_boundary();

        let mut actuator_failed = state();
        actuator_failed.actuator_ready = false;
        let receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            1,
            &request(114, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &actuator_failed,
        );
        assert_eq!(receipt.status, DispatchStatus::Rejected);

        let mut low_battery = state();
        low_battery.battery_fraction = 0.02;
        let receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            2,
            &request(115, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &low_battery,
        );
        assert_eq!(receipt.status, DispatchStatus::Rejected);

        let mut lost_link = state();
        lost_link.communication_healthy = false;
        let receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            3,
            &request(116, 11.0, [1.0, 1.0, 2.0], 1.0),
            10.5,
            &lost_link,
        );
        assert_eq!(receipt.status, DispatchStatus::Rejected);
        assert!(boundary.controller().calls.is_empty());
    }

    #[test]
    fn unsupported_intent_never_reaches_controller() {
        let mut boundary = make_boundary();
        let unsupported = ControlRequest {
            request_id: 117,
            issued_at_s: 10.0,
            expires_at_s: 11.0,
            belief_confidence: 0.9,
            intent: EmbodiedIntent::HoldPosition,
        };
        let receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            1,
            &unsupported,
            10.5,
            &state(),
        );
        assert_eq!(receipt.status, DispatchStatus::Rejected);
        assert!(receipt.rejections.contains(&DispatchRejection::Runtime(VetoReason::UnsupportedIntent)));
        assert!(boundary.controller().calls.is_empty());
    }

    #[test]
    fn non_finite_pose_never_reaches_controller() {
        let mut boundary = make_boundary();
        let invalid = request(118, 11.0, [f64::NAN, 1.0, 2.0], 1.0);
        let receipt = dispatch(
            &mut boundary,
            "verified-test-credential",
            1,
            &invalid,
            10.5,
            &state(),
        );
        assert_eq!(receipt.status, DispatchStatus::Rejected);
        assert!(boundary.controller().calls.is_empty());
    }

}
