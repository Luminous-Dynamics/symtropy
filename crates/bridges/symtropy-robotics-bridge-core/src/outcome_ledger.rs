// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bounded, fail-closed lifecycle ledger for controller execution outcomes.
//!
//! The ledger binds each submitted request ID to the controller ticket issued
//! by DispatchBoundary. It validates the event lifecycle and rejects unknown
//! requests, ticket mismatches, duplicate/non-advancing ticks, backward time,
//! illegal transitions, and all events after a terminal state.
//!
//! This is in-memory conformance infrastructure. It is not a durable audit log,
//! cryptographic authentication of the event source, or proof of physical motion.
//! A deployment must bind events to the actual controller ticket and persist the
//! ledger or provide an equivalent durable/restart-safe authority.

use crate::control::{ControlOutcome, ControlOutcomeError, ControlOutcomeStatus};
use crate::dispatch::{DispatchReceipt, DispatchStatus};
use std::collections::BTreeMap;

/// Fail-closed reasons for registering dispatches or ingesting controller events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeLedgerError {
    InvalidCapacity,
    InvalidSubmissionStatus,
    InvalidSubmissionReceipt,
    MissingAuthenticatedSession,
    MissingControllerTicket,
    EmptyControllerTicket,
    DuplicateRequestId,
    DuplicateControllerTicket,
    CapacityReached,
    UnknownRequest,
    ControllerTicketMismatch,
    InvalidOutcome(ControlOutcomeError),
    TerminalAlreadyReached,
    NonAdvancingTick,
}

/// Summary of one accepted event, suitable for routing to a cognitive feedback loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutcomeIngestReceipt {
    pub request_id: u64,
    pub session_id: u64,
    pub sequence: u64,
    pub previous_status: Option<ControlOutcomeStatus>,
    pub status: ControlOutcomeStatus,
    pub tick: u64,
    pub terminal: bool,
}

#[derive(Debug, Clone)]
struct TrackedRequest {
    session_id: u64,
    sequence: u64,
    controller_ticket: String,
    last_outcome: Option<ControlOutcome>,
}

/// Bounded lifecycle owner for submitted controller requests.
///
/// Capacity exhaustion fails closed. No record is evicted automatically,
/// including terminal records: evicting a terminal tombstone could make a late
/// duplicate event indistinguishable from an unknown/reused request. Production
/// deployments should persist a durable event ledger and define explicit
/// retention/rekey semantics before adding compaction.
#[derive(Debug, Clone)]
pub struct ControlOutcomeLedger {
    max_records: usize,
    records: BTreeMap<u64, TrackedRequest>,
}

impl ControlOutcomeLedger {
    pub fn new(max_records: usize) -> Result<Self, OutcomeLedgerError> {
        if max_records == 0 {
            return Err(OutcomeLedgerError::InvalidCapacity);
        }
        Ok(Self {
            max_records,
            records: BTreeMap::new(),
        })
    }

    /// Register a request only after DispatchBoundary returned Submitted with a
    /// nonzero authenticated session/sequence and a nonempty controller ticket.
    /// Request IDs remain reserved until the ledger is externally retired.
    pub fn register_submission(
        &mut self,
        receipt: &DispatchReceipt,
    ) -> Result<(), OutcomeLedgerError> {
        if receipt.status != DispatchStatus::Submitted {
            return Err(OutcomeLedgerError::InvalidSubmissionStatus);
        }
        if receipt.request_id == 0 || receipt.sequence == 0 {
            return Err(OutcomeLedgerError::InvalidSubmissionReceipt);
        }
        let session_id = receipt
            .session_id
            .filter(|session_id| *session_id != 0)
            .ok_or(OutcomeLedgerError::MissingAuthenticatedSession)?;
        let controller_ticket = receipt
            .controller_ticket
            .as_deref()
            .ok_or(OutcomeLedgerError::MissingControllerTicket)?;
        if controller_ticket.trim().is_empty() {
            return Err(OutcomeLedgerError::EmptyControllerTicket);
        }
        if self.records.contains_key(&receipt.request_id) {
            return Err(OutcomeLedgerError::DuplicateRequestId);
        }
        if self
            .records
            .values()
            .any(|tracked| tracked.controller_ticket == controller_ticket)
        {
            return Err(OutcomeLedgerError::DuplicateControllerTicket);
        }
        if self.records.len() >= self.max_records {
            return Err(OutcomeLedgerError::CapacityReached);
        }

        self.records.insert(
            receipt.request_id,
            TrackedRequest {
                session_id,
                sequence: receipt.sequence,
                controller_ticket: controller_ticket.to_owned(),
                last_outcome: None,
            },
        );
        Ok(())
    }

    /// Validate and record a controller outcome tied to its exact submitted ticket.
    ///
    /// The caller must obtain the controller ticket from the authenticated/native
    /// controller event source, not derive it from the outcome's request ID.
    /// A terminal lifecycle can never be reopened, even by a higher tick.
    pub fn ingest(
        &mut self,
        controller_ticket: &str,
        outcome: ControlOutcome,
    ) -> Result<OutcomeIngestReceipt, OutcomeLedgerError> {
        if controller_ticket.trim().is_empty() {
            return Err(OutcomeLedgerError::EmptyControllerTicket);
        }
        outcome
            .validate()
            .map_err(OutcomeLedgerError::InvalidOutcome)?;

        let tracked = self
            .records
            .get_mut(&outcome.request_id)
            .ok_or(OutcomeLedgerError::UnknownRequest)?;
        if tracked.controller_ticket != controller_ticket {
            return Err(OutcomeLedgerError::ControllerTicketMismatch);
        }

        let previous_status = tracked.last_outcome.as_ref().map(|old| old.status);
        if let Some(previous) = tracked.last_outcome.as_ref() {
            if previous.status.is_terminal() {
                return Err(OutcomeLedgerError::TerminalAlreadyReached);
            }
            // A strictly increasing controller tick distinguishes a new event
            // from a duplicate replay with an unchanged tick.
            if outcome.tick <= previous.tick {
                return Err(OutcomeLedgerError::NonAdvancingTick);
            }
            outcome
                .validate_successor_of(previous)
                .map_err(OutcomeLedgerError::InvalidOutcome)?;
        }

        let receipt = OutcomeIngestReceipt {
            request_id: outcome.request_id,
            session_id: tracked.session_id,
            sequence: tracked.sequence,
            previous_status,
            status: outcome.status,
            tick: outcome.tick,
            terminal: outcome.status.is_terminal(),
        };
        tracked.last_outcome = Some(outcome);
        Ok(receipt)
    }

    /// Last accepted outcome for a registered request, including terminal outcomes.
    pub fn last_outcome(&self, request_id: u64) -> Option<&ControlOutcome> {
        self.records
            .get(&request_id)
            .and_then(|tracked| tracked.last_outcome.as_ref())
    }

    /// Controller ticket recorded for a request, whether or not an outcome arrived.
    pub fn controller_ticket_for(&self, request_id: u64) -> Option<&str> {
        self.records
            .get(&request_id)
            .map(|tracked| tracked.controller_ticket.as_str())
    }

    pub fn tracked_requests(&self) -> usize {
        self.records.len()
    }

    pub fn terminal_requests(&self) -> usize {
        self.records
            .values()
            .filter(|tracked| {
                tracked
                    .last_outcome
                    .as_ref()
                    .is_some_and(|outcome| outcome.status.is_terminal())
            })
            .count()
    }

    pub fn remaining_capacity(&self) -> usize {
        self.max_records.saturating_sub(self.records.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlOutcomeStatus;
    use crate::dispatch::DispatchRejection;

    fn submission(request_id: u64, status: DispatchStatus, ticket: Option<&str>) -> DispatchReceipt {
        DispatchReceipt {
            request_id,
            session_id: Some(9),
            sequence: 1,
            observed_at_s: Some(1.0),
            status,
            rejections: Vec::<DispatchRejection>::new(),
            controller_ticket: ticket.map(str::to_owned),
        }
    }

    fn outcome(
        request_id: u64,
        status: ControlOutcomeStatus,
        observed_at_s: f64,
        tick: u64,
    ) -> ControlOutcome {
        let constraint_codes = if matches!(
            status,
            ControlOutcomeStatus::Constrained | ControlOutcomeStatus::Vetoed
        ) {
            vec!["test_constraint".to_owned()]
        } else {
            Vec::new()
        };
        ControlOutcome {
            request_id,
            status,
            observed_at_s,
            tick,
            applied_fraction: if status.is_terminal() && status != ControlOutcomeStatus::Completed {
                0.0
            } else {
                1.0
            },
            constraint_codes,
            telemetry_refs: vec!["trace:fixture-1".into()],
        }
    }

    fn registered_ledger(capacity: usize, request_id: u64, ticket: &str) -> ControlOutcomeLedger {
        let mut ledger = ControlOutcomeLedger::new(capacity).unwrap();
        ledger
            .register_submission(&submission(
                request_id,
                DispatchStatus::Submitted,
                Some(ticket),
            ))
            .unwrap();
        ledger
    }

    #[test]
    fn capacity_must_be_positive() {
        assert_eq!(
            ControlOutcomeLedger::new(0).err(),
            Some(OutcomeLedgerError::InvalidCapacity)
        );
    }

    #[test]
    fn only_valid_submitted_receipts_can_be_registered() {
        let mut ledger = ControlOutcomeLedger::new(4).unwrap();
        assert_eq!(
            ledger.register_submission(&submission(1, DispatchStatus::Rejected, Some("t-1"))),
            Err(OutcomeLedgerError::InvalidSubmissionStatus)
        );
        assert_eq!(
            ledger.register_submission(&submission(1, DispatchStatus::Submitted, None)),
            Err(OutcomeLedgerError::MissingControllerTicket)
        );
        assert_eq!(
            ledger.register_submission(&submission(1, DispatchStatus::Submitted, Some("  "))),
            Err(OutcomeLedgerError::EmptyControllerTicket)
        );
        let mut no_session = submission(1, DispatchStatus::Submitted, Some("t-1"));
        no_session.session_id = None;
        assert_eq!(
            ledger.register_submission(&no_session),
            Err(OutcomeLedgerError::MissingAuthenticatedSession)
        );
        assert_eq!(ledger.tracked_requests(), 0);
    }

    #[test]
    fn controller_ticket_cannot_be_bound_to_two_request_ids() {
        let mut ledger = registered_ledger(4, 17, "native-ticket-17");
        assert_eq!(
            ledger.register_submission(&submission(
                18,
                DispatchStatus::Submitted,
                Some("native-ticket-17"),
            )),
            Err(OutcomeLedgerError::DuplicateControllerTicket)
        );
        assert_eq!(ledger.tracked_requests(), 1);
        assert_eq!(ledger.controller_ticket_for(17), Some("native-ticket-17"));
        assert!(ledger.controller_ticket_for(18).is_none());
    }

    #[test]
    fn accepts_monotonic_lifecycle_and_never_reopens_terminal_state() {
        let mut ledger = registered_ledger(4, 17, "native-ticket-17");
        let accepted = ledger
            .ingest(
                "native-ticket-17",
                outcome(17, ControlOutcomeStatus::Accepted, 1.0, 1),
            )
            .unwrap();
        assert_eq!(accepted.session_id, 9);
        assert_eq!(accepted.sequence, 1);
        assert_eq!(accepted.previous_status, None);
        assert!(!accepted.terminal);

        let executing = ledger
            .ingest(
                "native-ticket-17",
                outcome(17, ControlOutcomeStatus::Executing, 1.1, 2),
            )
            .unwrap();
        assert_eq!(executing.previous_status, Some(ControlOutcomeStatus::Accepted));
        assert_eq!(executing.status, ControlOutcomeStatus::Executing);

        let completed = ledger
            .ingest(
                "native-ticket-17",
                outcome(17, ControlOutcomeStatus::Completed, 1.2, 3),
            )
            .unwrap();
        assert!(completed.terminal);
        assert_eq!(ledger.terminal_requests(), 1);
        assert_eq!(
            ledger.ingest(
                "native-ticket-17",
                outcome(17, ControlOutcomeStatus::Executing, 1.3, 4),
            ),
            Err(OutcomeLedgerError::TerminalAlreadyReached)
        );
        assert_eq!(
            ledger.last_outcome(17).unwrap().status,
            ControlOutcomeStatus::Completed
        );
    }

    #[test]
    fn safety_veto_during_execution_is_accepted_and_terminal() {
        let mut ledger = registered_ledger(4, 19, "native-ticket-19");
        ledger
            .ingest(
                "native-ticket-19",
                outcome(19, ControlOutcomeStatus::Executing, 1.0, 1),
            )
            .unwrap();

        let veto = ledger
            .ingest(
                "native-ticket-19",
                outcome(19, ControlOutcomeStatus::Vetoed, 1.1, 2),
            )
            .unwrap();
        assert_eq!(veto.previous_status, Some(ControlOutcomeStatus::Executing));
        assert_eq!(veto.status, ControlOutcomeStatus::Vetoed);
        assert!(veto.terminal);

        assert_eq!(
            ledger.ingest(
                "native-ticket-19",
                outcome(19, ControlOutcomeStatus::Executing, 1.2, 3),
            ),
            Err(OutcomeLedgerError::TerminalAlreadyReached)
        );
        assert_eq!(
            ledger.last_outcome(19).unwrap().status,
            ControlOutcomeStatus::Vetoed
        );
    }

    #[test]
    fn unknown_request_and_ticket_mismatch_never_mutate_the_ledger() {
        let mut ledger = registered_ledger(4, 21, "native-ticket-21");
        assert_eq!(
            ledger.ingest(
                "native-ticket-21",
                outcome(99, ControlOutcomeStatus::Accepted, 1.0, 1),
            ),
            Err(OutcomeLedgerError::UnknownRequest)
        );
        assert_eq!(
            ledger.ingest(
                "another-ticket",
                outcome(21, ControlOutcomeStatus::Accepted, 1.0, 1),
            ),
            Err(OutcomeLedgerError::ControllerTicketMismatch)
        );
        assert!(ledger.last_outcome(21).is_none());
    }

    #[test]
    fn duplicate_tick_and_backwards_time_are_rejected_without_replacing_last_event() {
        let mut ledger = registered_ledger(4, 31, "native-ticket-31");
        ledger
            .ingest(
                "native-ticket-31",
                outcome(31, ControlOutcomeStatus::Executing, 1.0, 5),
            )
            .unwrap();

        assert_eq!(
            ledger.ingest(
                "native-ticket-31",
                outcome(31, ControlOutcomeStatus::Executing, 1.1, 5),
            ),
            Err(OutcomeLedgerError::NonAdvancingTick)
        );
        assert_eq!(
            ledger.ingest(
                "native-ticket-31",
                outcome(31, ControlOutcomeStatus::Completed, 0.9, 6),
            ),
            Err(OutcomeLedgerError::InvalidOutcome(
                ControlOutcomeError::TimeWentBackwards
            ))
        );
        let last = ledger.last_outcome(31).unwrap();
        assert_eq!(last.status, ControlOutcomeStatus::Executing);
        assert_eq!(last.tick, 5);
    }

    #[test]
    fn invalid_outcome_payload_is_rejected_before_mutating_the_ledger() {
        let mut ledger = registered_ledger(4, 41, "native-ticket-41");
        let mut invalid = outcome(41, ControlOutcomeStatus::Vetoed, 1.0, 1);
        invalid.constraint_codes.clear();
        assert_eq!(
            ledger.ingest("native-ticket-41", invalid),
            Err(OutcomeLedgerError::InvalidOutcome(
                ControlOutcomeError::MissingConstraintCause
            ))
        );
        assert!(ledger.last_outcome(41).is_none());
    }

    #[test]
    fn seeded_adversarial_event_stream_never_mutates_state_on_rejection() {
        const CASES: usize = 256;
        let mut ledger = ControlOutcomeLedger::new(CASES).unwrap();

        // Register unique submitted requests and accept one valid in-flight event
        // for each. A fixed seed makes any failure exactly reproducible.
        for index in 0..CASES {
            let request_id = 1_000 + index as u64;
            let ticket = format!("native-ticket-{request_id}");
            ledger
                .register_submission(&submission(
                    request_id,
                    DispatchStatus::Submitted,
                    Some(&ticket),
                ))
                .unwrap();
            ledger
                .ingest(
                    &ticket,
                    outcome(request_id, ControlOutcomeStatus::Executing, 1.0, 1),
                )
                .unwrap();
        }

        let mut rng = 0x8f31_2a77_91c5_004du64;
        let mut covered = [false; 6];

        for index in 0..CASES {
            let request_id = 1_000 + index as u64;
            let ticket = format!("native-ticket-{request_id}");
            let before = ledger.last_outcome(request_id).unwrap().clone();

            // Deterministic LCG stream: no external randomness or dependencies.
            rng = rng
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let case = ((rng >> 32) % 6) as usize;
            covered[case] = true;

            let (presented_ticket, candidate, expected_error) = match case {
                0 => (
                    "wrong-controller-ticket".to_owned(),
                    outcome(request_id, ControlOutcomeStatus::Executing, 1.1, 2),
                    OutcomeLedgerError::ControllerTicketMismatch,
                ),
                1 => (
                    ticket.clone(),
                    outcome(request_id, ControlOutcomeStatus::Executing, 1.1, 1),
                    OutcomeLedgerError::NonAdvancingTick,
                ),
                2 => (
                    ticket.clone(),
                    outcome(request_id, ControlOutcomeStatus::Completed, 0.9, 2),
                    OutcomeLedgerError::InvalidOutcome(ControlOutcomeError::TimeWentBackwards),
                ),
                3 => {
                    let mut invalid =
                        outcome(request_id, ControlOutcomeStatus::Vetoed, 1.1, 2);
                    invalid.constraint_codes.clear();
                    (
                        ticket.clone(),
                        invalid,
                        OutcomeLedgerError::InvalidOutcome(
                            ControlOutcomeError::MissingConstraintCause,
                        ),
                    )
                }
                4 => (
                    ticket.clone(),
                    outcome(request_id, ControlOutcomeStatus::Accepted, 1.1, 2),
                    OutcomeLedgerError::InvalidOutcome(
                        ControlOutcomeError::IllegalStatusTransition,
                    ),
                ),
                _ => (
                    ticket.clone(),
                    outcome(
                        request_id + 1_000_000,
                        ControlOutcomeStatus::Executing,
                        1.1,
                        2,
                    ),
                    OutcomeLedgerError::UnknownRequest,
                ),
            };

            assert_eq!(
                ledger.ingest(&presented_ticket, candidate),
                Err(expected_error),
                "adversarial case {case} for request {request_id}"
            );
            assert_eq!(
                ledger.last_outcome(request_id),
                Some(&before),
                "rejected case {case} mutated request {request_id}"
            );

            // A valid event still progresses after each rejection, proving the
            // failure path did not poison the sequence or terminal state.
            let completed = ledger
                .ingest(
                    &ticket,
                    outcome(request_id, ControlOutcomeStatus::Completed, 1.2, 2),
                )
                .unwrap();
            assert!(completed.terminal);

            let terminal_snapshot = ledger.last_outcome(request_id).unwrap().clone();
            assert_eq!(
                ledger.ingest(
                    &ticket,
                    outcome(request_id, ControlOutcomeStatus::Executing, 1.3, 3),
                ),
                Err(OutcomeLedgerError::TerminalAlreadyReached)
            );
            assert_eq!(
                ledger.last_outcome(request_id),
                Some(&terminal_snapshot),
                "post-terminal event mutated request {request_id}"
            );
        }

        assert!(
            covered.iter().all(|seen| *seen),
            "the deterministic stream must exercise every adversarial case"
        );
        assert_eq!(ledger.terminal_requests(), CASES);
    }

    #[test]
    fn capacity_exhaustion_fails_closed_and_keeps_terminal_tombstones() {
        let mut ledger = registered_ledger(1, 51, "native-ticket-51");
        ledger
            .ingest(
                "native-ticket-51",
                outcome(51, ControlOutcomeStatus::Completed, 1.0, 1),
            )
            .unwrap();
        assert_eq!(
            ledger.register_submission(&submission(
                52,
                DispatchStatus::Submitted,
                Some("native-ticket-52"),
            )),
            Err(OutcomeLedgerError::CapacityReached)
        );
        assert_eq!(
            ledger.register_submission(&submission(
                51,
                DispatchStatus::Submitted,
                Some("reused-request-id"),
            )),
            Err(OutcomeLedgerError::DuplicateRequestId)
        );
        assert_eq!(ledger.remaining_capacity(), 0);
        assert_eq!(ledger.terminal_requests(), 1);
    }

    #[test]
    fn immediate_terminal_failure_can_be_the_first_controller_outcome() {
        let mut ledger = registered_ledger(4, 61, "native-ticket-61");
        let receipt = ledger
            .ingest(
                "native-ticket-61",
                outcome(61, ControlOutcomeStatus::Failed, 1.0, 1),
            )
            .unwrap();
        assert!(receipt.terminal);
        assert_eq!(ledger.terminal_requests(), 1);
    }
}
