// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bounded per-session lifecycle ledger for controller outcomes.
//!
//! Dispatch admission and controller submission are not task completion. This
//! ledger binds a successful submission to its controller ticket and then
//! accepts only monotonic, ticket-matched outcome updates. Terminal outcomes
//! are retained for the lifetime of this in-memory ledger so delayed or replayed
//! updates cannot reopen a request.
//!
//! This is an in-process conformance boundary, not durable anti-replay storage.
//! Capacity exhaustion fails closed. A production host must persist/anchor
//! terminal state before implementing any safe archival or compaction policy.

use crate::control::{ControlOutcome, ControlOutcomeError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_TRACKED_OUTCOME_CAPACITY: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeLedgerError {
    InvalidCapacity,
    ZeroIdentity,
    CapacityReached,
    DuplicateRequest,
    UnknownRequest,
    TicketNotBound,
    EmptyTicket,
    TicketAlreadyBound,
    ControllerTicketMismatch,
    InvalidOutcome,
    OutOfOrder,
    SameTickConflict,
    IllegalTransition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeUpdate {
    Applied,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq)]
struct TrackedRequest {
    controller_ticket: Option<String>,
    latest: Option<ControlOutcome>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlOutcomeLedger {
    max_tracked_requests: usize,
    requests: BTreeMap<(u64, u64), TrackedRequest>,
}

impl Default for ControlOutcomeLedger {
    fn default() -> Self {
        Self {
            max_tracked_requests: DEFAULT_TRACKED_OUTCOME_CAPACITY,
            requests: BTreeMap::new(),
        }
    }
}

impl ControlOutcomeLedger {
    pub fn with_capacity(max_tracked_requests: usize) -> Result<Self, OutcomeLedgerError> {
        if max_tracked_requests == 0 {
            return Err(OutcomeLedgerError::InvalidCapacity);
        }
        Ok(Self {
            max_tracked_requests,
            requests: BTreeMap::new(),
        })
    }

    /// Reserve request identity before calling the controller sink.
    ///
    /// The caller must hold the same exclusive authority boundary across
    /// reserve, sink submission, and ticket binding. A refused submission must
    /// release its reservation; a successful submission must bind its ticket.
    pub fn reserve(
        &mut self,
        session_id: u64,
        request_id: u64,
    ) -> Result<(), OutcomeLedgerError> {
        if session_id == 0 || request_id == 0 {
            return Err(OutcomeLedgerError::ZeroIdentity);
        }
        let key = (session_id, request_id);
        if self.requests.contains_key(&key) {
            return Err(OutcomeLedgerError::DuplicateRequest);
        }
        if self.requests.len() >= self.max_tracked_requests {
            return Err(OutcomeLedgerError::CapacityReached);
        }
        self.requests.insert(
            key,
            TrackedRequest {
                controller_ticket: None,
                latest: None,
            },
        );
        Ok(())
    }

    /// Bind the exact ticket returned by the controller after accepted submit.
    pub fn bind_ticket(
        &mut self,
        session_id: u64,
        request_id: u64,
        controller_ticket: &str,
    ) -> Result<(), OutcomeLedgerError> {
        if controller_ticket.trim().is_empty() {
            return Err(OutcomeLedgerError::EmptyTicket);
        }
        let tracked = self
            .requests
            .get_mut(&(session_id, request_id))
            .ok_or(OutcomeLedgerError::UnknownRequest)?;
        if tracked.controller_ticket.is_some() {
            return Err(OutcomeLedgerError::TicketAlreadyBound);
        }
        tracked.controller_ticket = Some(controller_ticket.to_owned());
        Ok(())
    }

    /// Remove a reservation only when the controller definitively refused the
    /// submission or returned no usable ticket. Never release a tracked request
    /// after an outcome has been accepted.
    pub(crate) fn release_unsubmitted(
        &mut self,
        session_id: u64,
        request_id: u64,
    ) -> bool {
        let key = (session_id, request_id);
        if self
            .requests
            .get(&key)
            .is_some_and(|tracked| tracked.controller_ticket.is_none() && tracked.latest.is_none())
        {
            self.requests.remove(&key);
            true
        } else {
            false
        }
    }

    /// Record an outcome from the adapter that owns the matching controller
    /// ticket. Exact same-tick replays are idempotent; conflicting same-tick
    /// payloads and non-monotonic/terminal transitions are rejected.
    pub fn record(
        &mut self,
        session_id: u64,
        controller_ticket: &str,
        outcome: ControlOutcome,
    ) -> Result<OutcomeUpdate, OutcomeLedgerError> {
        outcome
            .validate()
            .map_err(|_| OutcomeLedgerError::InvalidOutcome)?;

        let tracked = self
            .requests
            .get_mut(&(session_id, outcome.request_id))
            .ok_or(OutcomeLedgerError::UnknownRequest)?;
        let expected_ticket = tracked
            .controller_ticket
            .as_deref()
            .ok_or(OutcomeLedgerError::TicketNotBound)?;
        if controller_ticket != expected_ticket {
            return Err(OutcomeLedgerError::ControllerTicketMismatch);
        }

        if let Some(previous) = &tracked.latest {
            if outcome.tick == previous.tick {
                return if outcome == *previous {
                    Ok(OutcomeUpdate::Duplicate)
                } else {
                    Err(OutcomeLedgerError::SameTickConflict)
                };
            }

            outcome
                .validate_successor_of(previous)
                .map_err(map_successor_error)?;
        }

        tracked.latest = Some(outcome);
        Ok(OutcomeUpdate::Applied)
    }

    pub fn latest(&self, session_id: u64, request_id: u64) -> Option<&ControlOutcome> {
        self.requests
            .get(&(session_id, request_id))
            .and_then(|tracked| tracked.latest.as_ref())
    }

    pub fn is_ticket_bound(&self, session_id: u64, request_id: u64) -> bool {
        self.requests
            .get(&(session_id, request_id))
            .is_some_and(|tracked| tracked.controller_ticket.is_some())
    }

    pub fn tracked_requests(&self) -> usize {
        self.requests.len()
    }

    pub fn remaining_capacity(&self) -> usize {
        self.max_tracked_requests.saturating_sub(self.requests.len())
    }
}

fn map_successor_error(error: ControlOutcomeError) -> OutcomeLedgerError {
    match error {
        ControlOutcomeError::TimeWentBackwards | ControlOutcomeError::TickWentBackwards => {
            OutcomeLedgerError::OutOfOrder
        }
        ControlOutcomeError::IllegalStatusTransition => OutcomeLedgerError::IllegalTransition,
        ControlOutcomeError::RequestIdMismatch
        | ControlOutcomeError::ZeroRequestId
        | ControlOutcomeError::InvalidObservationTime
        | ControlOutcomeError::InvalidAppliedFraction
        | ControlOutcomeError::MissingConstraintCause => OutcomeLedgerError::InvalidOutcome,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlOutcomeStatus;

    fn outcome(
        request_id: u64,
        status: ControlOutcomeStatus,
        observed_at_s: f64,
        tick: u64,
    ) -> ControlOutcome {
        ControlOutcome {
            request_id,
            status,
            observed_at_s,
            tick,
            applied_fraction: if status == ControlOutcomeStatus::Vetoed {
                0.0
            } else {
                1.0
            },
            constraint_codes: if matches!(
                status,
                ControlOutcomeStatus::Constrained | ControlOutcomeStatus::Vetoed
            ) {
                vec!["test_constraint".into()]
            } else {
                Vec::new()
            },
            telemetry_refs: vec![format!("telemetry-{tick}")],
        }
    }

    #[test]
    fn outcome_progress_is_monotonic_and_terminal_state_cannot_reopen() {
        let mut ledger = ControlOutcomeLedger::with_capacity(4).unwrap();
        ledger.reserve(7, 101).unwrap();
        ledger.bind_ticket(7, 101, "controller-101").unwrap();

        let accepted = outcome(101, ControlOutcomeStatus::Accepted, 10.0, 1);
        let executing = outcome(101, ControlOutcomeStatus::Executing, 10.1, 2);
        let completed = outcome(101, ControlOutcomeStatus::Completed, 10.2, 3);

        assert_eq!(
            ledger.record(7, "controller-101", accepted),
            Ok(OutcomeUpdate::Applied)
        );
        assert_eq!(
            ledger.record(7, "controller-101", executing),
            Ok(OutcomeUpdate::Applied)
        );
        assert_eq!(
            ledger.record(7, "controller-101", completed.clone()),
            Ok(OutcomeUpdate::Applied)
        );
        assert_eq!(
            ledger.record(7, "controller-101", completed.clone()),
            Ok(OutcomeUpdate::Duplicate)
        );

        assert_eq!(
            ledger.record(
                7,
                "controller-101",
                outcome(101, ControlOutcomeStatus::Executing, 10.3, 4)
            ),
            Err(OutcomeLedgerError::IllegalTransition)
        );
        assert_eq!(ledger.latest(7, 101), Some(&completed));
    }

    #[test]
    fn mismatched_ticket_and_unregistered_outcomes_are_rejected() {
        let mut ledger = ControlOutcomeLedger::with_capacity(2).unwrap();
        ledger.reserve(7, 102).unwrap();

        let pending = outcome(102, ControlOutcomeStatus::Accepted, 1.0, 1);
        assert_eq!(
            ledger.record(7, "controller-102", pending.clone()),
            Err(OutcomeLedgerError::TicketNotBound)
        );
        ledger.bind_ticket(7, 102, "controller-102").unwrap();
        assert_eq!(
            ledger.record(7, "controller-other", pending),
            Err(OutcomeLedgerError::ControllerTicketMismatch)
        );
        assert_eq!(
            ledger.record(
                7,
                "controller-999",
                outcome(999, ControlOutcomeStatus::Completed, 2.0, 2)
            ),
            Err(OutcomeLedgerError::UnknownRequest)
        );
    }

    #[test]
    fn out_of_order_and_conflicting_same_tick_updates_are_rejected() {
        let mut ledger = ControlOutcomeLedger::with_capacity(2).unwrap();
        ledger.reserve(7, 103).unwrap();
        ledger.bind_ticket(7, 103, "controller-103").unwrap();
        ledger
            .record(
                7,
                "controller-103",
                outcome(103, ControlOutcomeStatus::Executing, 10.0, 4),
            )
            .unwrap();

        assert_eq!(
            ledger.record(
                7,
                "controller-103",
                outcome(103, ControlOutcomeStatus::Accepted, 9.0, 3)
            ),
            Err(OutcomeLedgerError::OutOfOrder)
        );
        assert_eq!(
            ledger.record(
                7,
                "controller-103",
                outcome(103, ControlOutcomeStatus::Completed, 10.1, 4)
            ),
            Err(OutcomeLedgerError::SameTickConflict)
        );
    }

    #[test]
    fn capacity_exhaustion_fails_closed_and_never_evicts_terminal_state() {
        let mut ledger = ControlOutcomeLedger::with_capacity(1).unwrap();
        ledger.reserve(7, 104).unwrap();
        ledger.bind_ticket(7, 104, "controller-104").unwrap();
        let completed = outcome(104, ControlOutcomeStatus::Completed, 10.0, 1);
        ledger.record(7, "controller-104", completed.clone()).unwrap();

        assert_eq!(
            ledger.reserve(7, 105),
            Err(OutcomeLedgerError::CapacityReached)
        );
        assert_eq!(ledger.latest(7, 104), Some(&completed));
        assert_eq!(ledger.tracked_requests(), 1);
        assert_eq!(ledger.remaining_capacity(), 0);
    }

    #[test]
    fn rejected_controller_submission_can_release_uncommitted_reservation() {
        let mut ledger = ControlOutcomeLedger::with_capacity(1).unwrap();
        ledger.reserve(7, 106).unwrap();
        assert!(ledger.release_unsubmitted(7, 106));
        assert_eq!(ledger.tracked_requests(), 0);
        ledger.reserve(7, 107).unwrap();
    }

    #[test]
    fn exact_duplicate_ticket_binding_is_not_allowed() {
        let mut ledger = ControlOutcomeLedger::with_capacity(1).unwrap();
        ledger.reserve(7, 108).unwrap();
        ledger.bind_ticket(7, 108, "controller-108").unwrap();
        assert_eq!(
            ledger.bind_ticket(7, 108, "controller-108"),
            Err(OutcomeLedgerError::TicketAlreadyBound)
        );
    }
}
