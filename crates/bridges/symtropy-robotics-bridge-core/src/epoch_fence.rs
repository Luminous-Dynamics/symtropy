// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit controller-epoch fencing for restart-aware outcome ingestion.
//!
//! An epoch is a controller-supervisor generation, not an arbitrary request
//! sequence. The hosting integration must obtain it from an authoritative,
//! restart-safe controller supervisor and persist/recover that generation
//! before accepting events. This module prevents stale-epoch events from being
//! accepted by the in-memory ledger; it does not itself persist epochs,
//! authenticate the source, stop physical motion, or provide atomic
//! multi-process fencing.

use crate::control::{ControlOutcome, ControlOutcomeStatus};
use crate::dispatch::{DispatchReceipt, DispatchStatus};
use crate::outcome_ledger::{
    ControlOutcomeLedger, OutcomeIngestReceipt, OutcomeLedgerError,
};
use std::collections::BTreeMap;

/// Monotonically increasing generation assigned by the authoritative controller
/// supervisor. The wrapped value must not be reset or rolled back across restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ControllerEpoch(u64);

impl ControllerEpoch {
    /// Build an epoch from a generation obtained from the trusted supervisor.
    ///
    /// This constructor validates shape only. It does not authenticate the
    /// caller or establish durability; the deployment owns those guarantees.
    pub fn new(generation: u64) -> Result<Self, EpochFenceError> {
        if generation == 0 {
            return Err(EpochFenceError::InvalidEpoch);
        }
        Ok(Self(generation))
    }

    pub const fn generation(self) -> u64 {
        self.0
    }
}

/// Fail-closed reasons for epoch-fenced registration and ingestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochFenceError {
    InvalidCapacity,
    InvalidEpoch,
    EpochDidNotAdvance {
        current_epoch: u64,
        requested_epoch: u64,
    },
    StaleControllerEpoch {
        current_epoch: u64,
        received_epoch: u64,
    },
    RequestFromPriorEpoch {
        request_id: u64,
        request_epoch: u64,
        current_epoch: u64,
    },
    DuplicateRequestId,
    CapacityReached,
    UnknownRequest,
    Ledger(OutcomeLedgerError),
}

/// Lifecycle ledger whose submissions and outcomes are explicitly fenced by
/// the controller generation that issued their tickets.
///
/// Request IDs are reserved globally across epochs. Controller tickets may be
/// reused by a restarted controller only because the epoch is part of the
/// effective ticket identity: `(epoch, ticket)`. Every epoch's ledger and
/// tombstones remain present until the object is externally retired, and the
/// configured capacity is global across all epochs.
#[derive(Debug, Clone)]
pub struct EpochFencedOutcomeLedger {
    max_records: usize,
    current_epoch: ControllerEpoch,
    ledgers: BTreeMap<ControllerEpoch, ControlOutcomeLedger>,
    request_epochs: BTreeMap<u64, ControllerEpoch>,
}

impl EpochFencedOutcomeLedger {
    pub fn new(
        initial_epoch: ControllerEpoch,
        max_records: usize,
    ) -> Result<Self, EpochFenceError> {
        if max_records == 0 {
            return Err(EpochFenceError::InvalidCapacity);
        }

        let mut ledgers = BTreeMap::new();
        ledgers.insert(
            initial_epoch,
            ControlOutcomeLedger::new(max_records).map_err(EpochFenceError::Ledger)?,
        );

        Ok(Self {
            max_records,
            current_epoch: initial_epoch,
            ledgers,
            request_epochs: BTreeMap::new(),
        })
    }

    pub const fn current_epoch(&self) -> ControllerEpoch {
        self.current_epoch
    }

    /// Move the trusted in-memory view to a strictly newer controller generation.
    ///
    /// This invalidates all prior-epoch event submissions through this API, but
    /// it does not terminate or stop any old physical controller operation.
    /// The platform must independently transition to its defined safe state
    /// during restart/reinitialization. Do not invoke this with a guessed,
    /// caller-controlled, or volatile counter value.
    pub fn advance_epoch(
        &mut self,
        next_epoch: ControllerEpoch,
    ) -> Result<(), EpochFenceError> {
        if next_epoch <= self.current_epoch {
            return Err(EpochFenceError::EpochDidNotAdvance {
                current_epoch: self.current_epoch.generation(),
                requested_epoch: next_epoch.generation(),
            });
        }

        if !self.ledgers.contains_key(&next_epoch) {
            let ledger =
                ControlOutcomeLedger::new(self.max_records).map_err(EpochFenceError::Ledger)?;
            self.ledgers.insert(next_epoch, ledger);
        }
        self.current_epoch = next_epoch;
        Ok(())
    }

    /// Register a submission only when the receipt belongs to the current epoch.
    ///
    /// The adapter/supervisor integration must bind the native controller ticket
    /// to the same epoch as the dispatch. The receipt format itself does not yet
    /// carry an epoch, so that binding is made explicit by this method argument.
    pub fn register_submission(
        &mut self,
        epoch: ControllerEpoch,
        receipt: &DispatchReceipt,
    ) -> Result<(), EpochFenceError> {
        self.require_current_epoch(epoch)?;

        if self.request_epochs.contains_key(&receipt.request_id) {
            return Err(EpochFenceError::DuplicateRequestId);
        }
        if self.request_epochs.len() >= self.max_records {
            return Err(EpochFenceError::CapacityReached);
        }

        let ledger = self
            .ledgers
            .get_mut(&epoch)
            .ok_or(EpochFenceError::UnknownRequest)?;
        ledger
            .register_submission(receipt)
            .map_err(EpochFenceError::Ledger)?;
        self.request_epochs.insert(receipt.request_id, epoch);
        Ok(())
    }

    /// Ingest only a controller event explicitly labeled with the current epoch.
    ///
    /// The epoch label must come from the authoritative controller event source,
    /// not be copied from an untrusted request. Events from old epochs are
    /// rejected before the nested lifecycle ledger can mutate any state.
    pub fn ingest(
        &mut self,
        epoch: ControllerEpoch,
        controller_ticket: &str,
        outcome: ControlOutcome,
    ) -> Result<OutcomeIngestReceipt, EpochFenceError> {
        self.require_current_epoch(epoch)?;

        let request_epoch = self
            .request_epochs
            .get(&outcome.request_id)
            .copied()
            .ok_or(EpochFenceError::UnknownRequest)?;
        if request_epoch != epoch {
            return Err(EpochFenceError::RequestFromPriorEpoch {
                request_id: outcome.request_id,
                request_epoch: request_epoch.generation(),
                current_epoch: self.current_epoch.generation(),
            });
        }

        let ledger = self
            .ledgers
            .get_mut(&epoch)
            .ok_or(EpochFenceError::UnknownRequest)?;
        ledger
            .ingest(controller_ticket, outcome)
            .map_err(EpochFenceError::Ledger)
    }

    pub fn request_epoch(&self, request_id: u64) -> Option<ControllerEpoch> {
        self.request_epochs.get(&request_id).copied()
    }

    pub fn tracked_requests(&self) -> usize {
        self.request_epochs.len()
    }

    pub fn remaining_capacity(&self) -> usize {
        self.max_records.saturating_sub(self.request_epochs.len())
    }

    /// Number of requests that originated in an earlier epoch and did not reach
    /// a terminal outcome before the epoch changed. These remain unresolved
    /// records; they are not silently converted to Completed or Failed.
    pub fn unresolved_requests_from_prior_epochs(&self) -> usize {
        self.request_epochs
            .iter()
            .filter_map(|(request_id, request_epoch)| {
                if *request_epoch >= self.current_epoch {
                    return None;
                }

                let unresolved = self
                    .ledgers
                    .get(request_epoch)
                    .and_then(|ledger| ledger.last_outcome(*request_id))
                    .is_none_or(|outcome| !outcome.status.is_terminal());
                Some(if unresolved { 1 } else { 0 })
            })
            .sum()
    }

    /// Number of all registered submissions from epochs prior to the current one.
    pub fn prior_epoch_request_count(&self) -> usize {
        self.request_epochs
            .values()
            .filter(|epoch| **epoch < self.current_epoch)
            .count()
    }

    fn require_current_epoch(&self, epoch: ControllerEpoch) -> Result<(), EpochFenceError> {
        if epoch != self.current_epoch {
            return Err(EpochFenceError::StaleControllerEpoch {
                current_epoch: self.current_epoch.generation(),
                received_epoch: epoch.generation(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlOutcomeStatus;
    use crate::dispatch::DispatchRejection;

    fn epoch(generation: u64) -> ControllerEpoch {
        ControllerEpoch::new(generation).unwrap()
    }

    fn submission(request_id: u64, ticket: &str) -> DispatchReceipt {
        DispatchReceipt {
            request_id,
            session_id: Some(7),
            sequence: request_id,
            observed_at_s: Some(1.0),
            status: DispatchStatus::Submitted,
            rejections: Vec::<DispatchRejection>::new(),
            controller_ticket: Some(ticket.to_owned()),
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
            vec!["supervisor_restart".to_owned()]
        } else {
            Vec::new()
        };
        ControlOutcome {
            request_id,
            status,
            observed_at_s,
            tick,
            applied_fraction: if matches!(
                status,
                ControlOutcomeStatus::Vetoed
                    | ControlOutcomeStatus::Failed
                    | ControlOutcomeStatus::Cancelled
                    | ControlOutcomeStatus::Expired
            ) {
                0.0
            } else {
                1.0
            },
            constraint_codes,
            telemetry_refs: vec!["trace:epoch-fixture".into()],
        }
    }

    fn ledger() -> EpochFencedOutcomeLedger {
        EpochFencedOutcomeLedger::new(epoch(1), 8).unwrap()
    }

    #[test]
    fn epoch_must_be_nonzero_and_capacity_positive() {
        assert_eq!(
            ControllerEpoch::new(0),
            Err(EpochFenceError::InvalidEpoch)
        );
        assert_eq!(
            EpochFencedOutcomeLedger::new(epoch(1), 0).err(),
            Some(EpochFenceError::InvalidCapacity)
        );
    }

    #[test]
    fn epoch_advance_must_be_strictly_monotonic() {
        let mut fence = EpochFencedOutcomeLedger::new(epoch(2), 8).unwrap();
        assert_eq!(
            fence.advance_epoch(epoch(2)),
            Err(EpochFenceError::EpochDidNotAdvance {
                current_epoch: 2,
                requested_epoch: 2,
            })
        );
        assert_eq!(
            fence.advance_epoch(epoch(1)),
            Err(EpochFenceError::EpochDidNotAdvance {
                current_epoch: 2,
                requested_epoch: 1,
            })
        );
        assert_eq!(fence.current_epoch(), epoch(2));
    }

    #[test]
    fn current_epoch_submission_and_outcome_are_accepted() {
        let mut fence = ledger();
        fence
            .register_submission(epoch(1), &submission(11, "ticket-A"))
            .unwrap();
        let receipt = fence
            .ingest(
                epoch(1),
                "ticket-A",
                outcome(11, ControlOutcomeStatus::Completed, 1.1, 1),
            )
            .unwrap();
        assert!(receipt.terminal);
        assert_eq!(receipt.request_id, 11);
        assert_eq!(fence.request_epoch(11), Some(epoch(1)));
        assert_eq!(fence.tracked_requests(), 1);
    }

    #[test]
    fn old_epoch_events_are_rejected_after_restart_without_mutating_prior_state() {
        let mut fence = ledger();
        fence
            .register_submission(epoch(1), &submission(11, "ticket-A"))
            .unwrap();
        fence
            .ingest(
                epoch(1),
                "ticket-A",
                outcome(11, ControlOutcomeStatus::Executing, 1.1, 1),
            )
            .unwrap();

        fence.advance_epoch(epoch(2)).unwrap();
        assert_eq!(fence.prior_epoch_request_count(), 1);
        assert_eq!(fence.unresolved_requests_from_prior_epochs(), 1);

        assert_eq!(
            fence.ingest(
                epoch(1),
                "ticket-A",
                outcome(11, ControlOutcomeStatus::Completed, 1.2, 2),
            ),
            Err(EpochFenceError::StaleControllerEpoch {
                current_epoch: 2,
                received_epoch: 1,
            })
        );
        assert_eq!(
            fence
                .ingest(
                    epoch(2),
                    "ticket-A",
                    outcome(11, ControlOutcomeStatus::Completed, 1.2, 2),
                ),
            Err(EpochFenceError::RequestFromPriorEpoch {
                request_id: 11,
                request_epoch: 1,
                current_epoch: 2,
            })
        );
        assert_eq!(
            fence
                .ledgers
                .get(&epoch(1))
                .unwrap()
                .last_outcome(11)
                .unwrap()
                .status,
            ControlOutcomeStatus::Executing
        );
        assert_eq!(fence.unresolved_requests_from_prior_epochs(), 1);
    }

    #[test]
    fn submission_from_old_epoch_is_rejected() {
        let mut fence = ledger();
        fence.advance_epoch(epoch(2)).unwrap();
        assert_eq!(
            fence.register_submission(epoch(1), &submission(12, "ticket-B")),
            Err(EpochFenceError::StaleControllerEpoch {
                current_epoch: 2,
                received_epoch: 1,
            })
        );
        assert_eq!(fence.tracked_requests(), 0);
    }

    #[test]
    fn request_ids_are_not_reusable_across_controller_epochs() {
        let mut fence = ledger();
        fence
            .register_submission(epoch(1), &submission(11, "ticket-A"))
            .unwrap();
        fence.advance_epoch(epoch(2)).unwrap();
        assert_eq!(
            fence.register_submission(epoch(2), &submission(11, "ticket-B")),
            Err(EpochFenceError::DuplicateRequestId)
        );
    }

    #[test]
    fn native_ticket_reuse_is_namespaced_by_epoch() {
        let mut fence = ledger();
        fence
            .register_submission(epoch(1), &submission(11, "reused-native-ticket"))
            .unwrap();
        fence.advance_epoch(epoch(2)).unwrap();
        fence
            .register_submission(epoch(2), &submission(12, "reused-native-ticket"))
            .unwrap();
        fence
            .ingest(
                epoch(2),
                "reused-native-ticket",
                outcome(12, ControlOutcomeStatus::Completed, 2.1, 1),
            )
            .unwrap();
        assert_eq!(fence.tracked_requests(), 2);
        assert_eq!(fence.request_epoch(11), Some(epoch(1)));
        assert_eq!(fence.request_epoch(12), Some(epoch(2)));
    }

    #[test]
    fn capacity_is_global_and_restart_does_not_free_tombstones() {
        let mut fence = EpochFencedOutcomeLedger::new(epoch(1), 1).unwrap();
        fence
            .register_submission(epoch(1), &submission(11, "ticket-A"))
            .unwrap();
        fence.advance_epoch(epoch(2)).unwrap();
        assert_eq!(
            fence.register_submission(epoch(2), &submission(12, "ticket-B")),
            Err(EpochFenceError::CapacityReached)
        );
        assert_eq!(fence.remaining_capacity(), 0);
        assert_eq!(fence.tracked_requests(), 1);
    }

    #[test]
    fn completed_prior_epoch_request_is_not_counted_as_unresolved() {
        let mut fence = ledger();
        fence
            .register_submission(epoch(1), &submission(11, "ticket-A"))
            .unwrap();
        fence
            .ingest(
                epoch(1),
                "ticket-A",
                outcome(11, ControlOutcomeStatus::Completed, 1.1, 1),
            )
            .unwrap();
        fence.advance_epoch(epoch(2)).unwrap();
        assert_eq!(fence.prior_epoch_request_count(), 1);
        assert_eq!(fence.unresolved_requests_from_prior_epochs(), 0);
    }

    #[test]
    fn every_restart_advance_invalidates_all_older_epochs() {
        let mut fence = ledger();
        fence
            .register_submission(epoch(1), &submission(11, "ticket-A"))
            .unwrap();
        fence.advance_epoch(epoch(2)).unwrap();
        fence
            .register_submission(epoch(2), &submission(12, "ticket-B"))
            .unwrap();
        fence.advance_epoch(epoch(3)).unwrap();

        assert_eq!(
            fence.ingest(
                epoch(2),
                "ticket-B",
                outcome(12, ControlOutcomeStatus::Completed, 3.1, 2),
            ),
            Err(EpochFenceError::StaleControllerEpoch {
                current_epoch: 3,
                received_epoch: 2,
            })
        );
        assert_eq!(fence.prior_epoch_request_count(), 2);
        assert_eq!(fence.unresolved_requests_from_prior_epochs(), 2);
    }
}
