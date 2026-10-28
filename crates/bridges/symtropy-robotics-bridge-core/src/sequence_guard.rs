// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Per-session monotonic sequence guard for cognitive control requests.
//!
//! This prevents duplicate, stale, or out-of-order sequence numbers within a
//! live authority session. It is not cryptographic authentication and does not
//! prevent a sender from re-encoding the same semantic action under a fresh
//! sequence. Session IDs must be authenticated by the transport, and a new
//! session ID must not be reused after process restart. Durable anti-replay
//! across restarts requires persistent, integrity-protected session state.

use crate::assurance::{GateDecision, RuntimeAssuranceGate, RuntimeSafetyState};
use crate::control::ControlRequest;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Fail-closed reasons from the sequence guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SequenceGuardError {
    /// Zero is reserved and cannot identify a session or sequence.
    ZeroSessionId,
    ZeroSequence,
    /// A new session cannot be tracked without exceeding the configured cap.
    SessionCapacityReached,
    /// Sequences must increase strictly; gaps are allowed.
    DuplicateOrOutOfOrder {
        last_accepted: u64,
        received: u64,
    },
    /// Constructor rejected zero tracking capacity.
    InvalidCapacity,
}

/// In-memory monotonic sequence ledger.
///
/// Call check_and_record after semantic/runtime admission and immediately
/// before dispatch, under the same serialized adapter authority that commits
/// the dispatch. A successful result must be committed before the request is
/// handed to the controller. This type alone does not provide atomicity across
/// threads/processes, durable persistence, transport authentication, or actuation
/// safety. Do not evict sessions to make room: capacity exhaustion fails closed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestSequenceGuard {
    max_sessions: usize,
    last_accepted_by_session: BTreeMap<u64, u64>,
}

impl RequestSequenceGuard {
    /// Create a guard with a fixed maximum number of concurrent authority sessions.
    pub fn new(max_sessions: usize) -> Result<Self, SequenceGuardError> {
        if max_sessions == 0 {
            return Err(SequenceGuardError::InvalidCapacity);
        }
        Ok(Self {
            max_sessions,
            last_accepted_by_session: BTreeMap::new(),
        })
    }

    /// Accept a new strictly increasing sequence for a session.
    ///
    /// Gaps are allowed because transports can drop requests. Duplicate and
    /// lower sequences are rejected. For an unknown session, the initial
    /// sequence can be any positive value; the caller is responsible for
    /// establishing an authenticated, non-reused session identifier.
    pub fn check_and_record(
        &mut self,
        session_id: u64,
        sequence: u64,
    ) -> Result<(), SequenceGuardError> {
        if session_id == 0 {
            return Err(SequenceGuardError::ZeroSessionId);
        }
        if sequence == 0 {
            return Err(SequenceGuardError::ZeroSequence);
        }

        if let Some(last_accepted) = self.last_accepted_by_session.get_mut(&session_id) {
            if sequence <= *last_accepted {
                return Err(SequenceGuardError::DuplicateOrOutOfOrder {
                    last_accepted: *last_accepted,
                    received: sequence,
                });
            }
            *last_accepted = sequence;
            return Ok(());
        }

        if self.last_accepted_by_session.len() >= self.max_sessions {
            return Err(SequenceGuardError::SessionCapacityReached);
        }

        self.last_accepted_by_session.insert(session_id, sequence);
        Ok(())
    }

    /// Last accepted sequence, if the session has been seen.
    pub fn last_accepted(&self, session_id: u64) -> Option<u64> {
        self.last_accepted_by_session.get(&session_id).copied()
    }

    /// Number of authority sessions currently tracked.
    pub fn tracked_sessions(&self) -> usize {
        self.last_accepted_by_session.len()
    }
}


/// Ordered admission wrapper that consumes a session sequence before evaluating
/// the generic runtime gate. Invalid semantic requests also consume their
/// sequence so the same transport sequence cannot be replayed repeatedly.
///
/// This still does not bind admission atomically to platform-controller dispatch.
/// The adapter must serialize this call and the subsequent dispatch under one
/// authority boundary, and must never dispatch unless the Result is Ok and the
/// returned GateDecision permits it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SequencedRuntimeAssurance {
    pub gate: RuntimeAssuranceGate,
    pub sequence_guard: RequestSequenceGuard,
}

impl SequencedRuntimeAssurance {
    pub fn new(gate: RuntimeAssuranceGate, sequence_guard: RequestSequenceGuard) -> Self {
        Self { gate, sequence_guard }
    }

    /// Sequence ordering is checked first, then generic admission is evaluated.
    /// Any sequence error means the caller must not dispatch the request.
    pub fn evaluate(
        &mut self,
        session_id: u64,
        sequence: u64,
        request: &ControlRequest,
        now_s: f64,
        state: &RuntimeSafetyState,
    ) -> Result<GateDecision, SequenceGuardError> {
        self.sequence_guard.check_and_record(session_id, sequence)?;
        Ok(self.gate.evaluate(request, now_s, state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_first_sequence_and_monotonic_gaps() {
        let mut guard = RequestSequenceGuard::new(2).unwrap();
        assert_eq!(guard.check_and_record(10, 4), Ok(()));
        assert_eq!(guard.check_and_record(10, 8), Ok(()));
        assert_eq!(guard.last_accepted(10), Some(8));
    }

    #[test]
    fn rejects_duplicate_sequence_without_mutating_watermark() {
        let mut guard = RequestSequenceGuard::new(2).unwrap();
        assert_eq!(guard.check_and_record(10, 4), Ok(()));
        assert_eq!(
            guard.check_and_record(10, 4),
            Err(SequenceGuardError::DuplicateOrOutOfOrder {
                last_accepted: 4,
                received: 4,
            })
        );
        assert_eq!(guard.last_accepted(10), Some(4));
    }

    #[test]
    fn rejects_out_of_order_sequence_without_mutating_watermark() {
        let mut guard = RequestSequenceGuard::new(2).unwrap();
        assert_eq!(guard.check_and_record(10, 9), Ok(()));
        assert_eq!(
            guard.check_and_record(10, 8),
            Err(SequenceGuardError::DuplicateOrOutOfOrder {
                last_accepted: 9,
                received: 8,
            })
        );
        assert_eq!(guard.last_accepted(10), Some(9));
    }

    #[test]
    fn independent_sessions_have_independent_watermarks() {
        let mut guard = RequestSequenceGuard::new(2).unwrap();
        assert_eq!(guard.check_and_record(10, 9), Ok(()));
        assert_eq!(guard.check_and_record(11, 1), Ok(()));
        assert_eq!(guard.last_accepted(10), Some(9));
        assert_eq!(guard.last_accepted(11), Some(1));
    }

    #[test]
    fn rejects_reserved_zero_values() {
        let mut guard = RequestSequenceGuard::new(1).unwrap();
        assert_eq!(
            guard.check_and_record(0, 1),
            Err(SequenceGuardError::ZeroSessionId)
        );
        assert_eq!(
            guard.check_and_record(1, 0),
            Err(SequenceGuardError::ZeroSequence)
        );
        assert_eq!(guard.tracked_sessions(), 0);
    }

    #[test]
    fn capacity_exhaustion_fails_closed_without_eviction() {
        let mut guard = RequestSequenceGuard::new(1).unwrap();
        assert_eq!(guard.check_and_record(10, 1), Ok(()));
        assert_eq!(
            guard.check_and_record(11, 1),
            Err(SequenceGuardError::SessionCapacityReached)
        );
        assert_eq!(guard.tracked_sessions(), 1);
        assert_eq!(guard.last_accepted(10), Some(1));
        assert_eq!(guard.last_accepted(11), None);
    }

    #[test]
    fn existing_session_can_advance_at_capacity() {
        let mut guard = RequestSequenceGuard::new(1).unwrap();
        assert_eq!(guard.check_and_record(10, 1), Ok(()));
        assert_eq!(guard.check_and_record(10, 2), Ok(()));
        assert_eq!(guard.last_accepted(10), Some(2));
    }

    #[test]
    fn zero_capacity_is_rejected() {
        assert_eq!(
            RequestSequenceGuard::new(0),
            Err(SequenceGuardError::InvalidCapacity)
        );
    }

    fn sample_pose() -> crate::control::Pose3 {
        crate::control::Pose3 {
            frame_id: "map".into(),
            position_m: [1.0, 0.0, 2.0],
            orientation_xyzw: [0.0, 0.0, 0.0, 1.0],
        }
    }

    fn sample_gate() -> RuntimeAssuranceGate {
        use crate::assurance::{AxisAlignedWorkspace, EmbodimentEnvelope, IntentKind, RuntimeAssurancePolicy};
        RuntimeAssuranceGate {
            policy: RuntimeAssurancePolicy {
                max_sensor_age_s: 0.1,
                max_snapshot_age_s: 0.2,
                minimum_battery_fraction: 0.1,
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
        }
    }

    fn sample_request(expires_at_s: f64) -> ControlRequest {
        use crate::control::EmbodiedIntent;
        ControlRequest {
            request_id: 91,
            issued_at_s: 8.0,
            expires_at_s,
            belief_confidence: 0.9,
            intent: EmbodiedIntent::NavigateTo {
                target: sample_pose(),
                max_speed_mps: Some(1.0),
            },
        }
    }

    fn sample_state() -> RuntimeSafetyState {
        RuntimeSafetyState {
            observed_at_s: 10.4,
            sensor_age_s: 0.01,
            actuator_ready: true,
            emergency_stop_active: false,
            communication_healthy: true,
            battery_fraction: 0.8,
        }
    }

    #[test]
    fn sequenced_runtime_rejects_replay_before_gate_evaluation() {
        let guard = RequestSequenceGuard::new(2).unwrap();
        let mut runtime = SequencedRuntimeAssurance::new(sample_gate(), guard);
        let request = sample_request(11.0);

        let first = runtime.evaluate(7, 1, &request, 10.5, &sample_state()).unwrap();
        assert!(first.permitted);
        assert_eq!(
            runtime.evaluate(7, 1, &request, 10.5, &sample_state()),
            Err(SequenceGuardError::DuplicateOrOutOfOrder {
                last_accepted: 1,
                received: 1,
            })
        );
        assert_eq!(runtime.sequence_guard.last_accepted(7), Some(1));
    }

    #[test]
    fn invalid_request_still_consumes_its_sequence() {
        let guard = RequestSequenceGuard::new(2).unwrap();
        let mut runtime = SequencedRuntimeAssurance::new(sample_gate(), guard);
        let expired = sample_request(9.0);

        let decision = runtime.evaluate(7, 3, &expired, 10.5, &sample_state()).unwrap();
        assert!(!decision.permitted);
        assert_eq!(runtime.sequence_guard.last_accepted(7), Some(3));
        assert_eq!(
            runtime.evaluate(7, 3, &expired, 10.5, &sample_state()),
            Err(SequenceGuardError::DuplicateOrOutOfOrder {
                last_accepted: 3,
                received: 3,
            })
        );
    }

}
