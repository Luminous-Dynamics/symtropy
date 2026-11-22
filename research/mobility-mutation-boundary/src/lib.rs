//! Authority-linearized mutation boundary reference model.
//!
//! This is an in-process software conformance model, not a production
//! controller, cryptographic verifier, durable authority service, or safety
//! mechanism. The caller supplies a canonical binding commitment produced by
//! existing M1/M2/platform owners. This crate compares it; it does not create,
//! authenticate, or prove the truth of that commitment.
//!
//! The mutex deliberately spans final validation, one-shot consumption and
//! the native-sink callback. In a real integration, the same linearization
//! guarantee must be enforced by the native controller's actual acceptance
//! boundary. A wrapper mutex cannot protect a downstream bypass or a second
//! unfenced queue.

use std::collections::BTreeSet;
use std::sync::Mutex;

/// Opaque content commitment of the complete canonical M1/M2/platform
/// authority binding. Its contents must remain reconstructible and validated
/// by the upstream owners; byte equality here is not authentication.
pub type BindingCommitment = [u8; 32];

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct AuthorityBinding {
    pub commitment: BindingCommitment,
    pub controller_epoch: u64,
    pub clock_domain_id: String,
    pub boot_epoch_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LiveAuthoritySnapshot {
    /// Exact currently active M1 action-attempt identity.
    pub active_attempt_key: String,
    /// Complete current M1/M2/platform binding supplied by its owners.
    pub binding: AuthorityBinding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionTicket {
    pub ticket_id: String,
    pub attempt_key: String,
    pub binding: AuthorityBinding,
    pub issued_at_ticks: u64,
    /// Exclusive expiry: now >= expires_at_ticks is expired.
    pub expires_at_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualifiedMonotonicInstant {
    pub clock_domain_id: String,
    pub boot_epoch_id: String,
    pub ticks: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Rejection {
    EmptyIdentity,
    InvalidValidityInterval,
    StaleAttempt,
    BindingMismatch,
    ClockBasisMismatch,
    NotYetValid,
    Expired,
    Replay,
    SequenceExhausted,
    LockPoisoned,
    PendingActionExists,
    NoPendingAction,
    PendingAttemptMismatch,
    RecoveryPermitMissing,
    InvalidNextSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeApplicationResult {
    /// The native controller accepted the command at its defined boundary.
    /// This is not evidence of physical effect or task completion.
    Accepted,
    /// The native sink definitively reports no command was applied.
    RejectedNoEffect,
    /// The command may have been applied; caller must reconcile before a
    /// subsequent mutation.
    UnknownAfterPossibleApplication,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptDisposition {
    AcceptedByNativeController,
    RejectedNoEffect,
    ReconciliationRequired,
}

/// Outcome class supplied by the existing independent-observer/M1
/// reconciliation owners. This crate validates the transition shape only;
/// it does not authenticate the evidence or recovery permit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QualifiedOutcomeResolution {
    Satisfied,
    NoEffect,
    PartialEffect,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolutionDisposition {
    ClearedForNewAuthoritySnapshot,
    HoldRetained,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplicationReceipt {
    pub ticket_id: String,
    pub attempt_key: String,
    pub binding_commitment: BindingCommitment,
    pub controller_epoch: u64,
    pub application_sequence: u64,
    pub disposition: ReceiptDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeApplicationContext<'a> {
    pub ticket_id: &'a str,
    pub attempt_key: &'a str,
    pub binding: &'a AuthorityBinding,
    pub application_sequence: u64,
}

struct BoundaryState {
    live: LiveAuthoritySnapshot,
    consumed: BTreeSet<String>,
    next_application_sequence: u64,
    pending_action_attempt: Option<String>,
}

/// Single-process reference boundary. The production authority owner and
/// native controller must provide an equivalent atomicity boundary; this
/// type cannot fence external processes, hardware queues, or restarts.
pub struct ApplicationBoundary {
    state: Mutex<BoundaryState>,
}

impl ApplicationBoundary {
    pub fn new(live: LiveAuthoritySnapshot) -> Self {
        Self {
            state: Mutex::new(BoundaryState {
                live,
                consumed: BTreeSet::new(),
                next_application_sequence: 1,
                pending_action_attempt: None,
            }),
        }
    }

    /// Publish an updated authoritative snapshot. The same mutex serializes
    /// this update against the final application callback.
    pub fn replace_live_snapshot(
        &self,
        next: LiveAuthoritySnapshot,
    ) -> Result<(), Rejection> {
        let mut state = self.state.lock().map_err(|_| Rejection::LockPoisoned)?;
        if next.binding.controller_epoch < state.live.binding.controller_epoch {
            return Err(Rejection::BindingMismatch);
        }
        state.live = next;
        Ok(())
    }

    /// Validate and consume a ticket at one in-process linearization point.
    ///
    /// The callback runs while the state lock is held. It must represent the
    /// actual native acceptance operation, not enqueue to another unfenced
    /// queue. The ticket is consumed before callback invocation, including
    /// when the callback returns no-effect or unknown, so an ambiguous result
    /// can never trigger a blind retry with the same attempt.
    pub fn apply<F>(
        &self,
        ticket: &ActionTicket,
        now: &QualifiedMonotonicInstant,
        native_apply: F,
    ) -> Result<ApplicationReceipt, Rejection>
    where
        F: FnOnce(&NativeApplicationContext<'_>) -> NativeApplicationResult,
    {
        let mut state = self.state.lock().map_err(|_| Rejection::LockPoisoned)?;

        if ticket.ticket_id.trim().is_empty() || ticket.attempt_key.trim().is_empty() {
            return Err(Rejection::EmptyIdentity);
        }
        if ticket.issued_at_ticks >= ticket.expires_at_ticks {
            return Err(Rejection::InvalidValidityInterval);
        }
        if ticket.attempt_key != state.live.active_attempt_key {
            return Err(Rejection::StaleAttempt);
        }
        if ticket.binding != state.live.binding {
            return Err(Rejection::BindingMismatch);
        }
        if now.clock_domain_id != ticket.binding.clock_domain_id
            || now.boot_epoch_id != ticket.binding.boot_epoch_id
        {
            return Err(Rejection::ClockBasisMismatch);
        }
        if now.ticks < ticket.issued_at_ticks {
            return Err(Rejection::NotYetValid);
        }
        if now.ticks >= ticket.expires_at_ticks {
            return Err(Rejection::Expired);
        }

        let consumption_key = ticket.ticket_id.clone();
        if state.consumed.contains(&consumption_key) {
            return Err(Rejection::Replay);
        }
        if state.pending_action_attempt.is_some() {
            return Err(Rejection::PendingActionExists);
        }

        let sequence = state.next_application_sequence;
        state.next_application_sequence = sequence
            .checked_add(1)
            .ok_or(Rejection::SequenceExhausted)?;

        // Consume the ticket and occupy the single-mutation slot before
        // crossing the native boundary. Even a definitive no-effect or
        // ambiguous result must be reconciled through the existing M1 owner
        // before another mutation can enter this boundary.
        state.consumed.insert(consumption_key);
        state.pending_action_attempt = Some(ticket.attempt_key.clone());

        let context = NativeApplicationContext {
            ticket_id: &ticket.ticket_id,
            attempt_key: &ticket.attempt_key,
            binding: &ticket.binding,
            application_sequence: sequence,
        };

        let native_result = native_apply(&context);
        let disposition = match native_result {
            NativeApplicationResult::Accepted => {
                ReceiptDisposition::AcceptedByNativeController
            }
            NativeApplicationResult::RejectedNoEffect => ReceiptDisposition::RejectedNoEffect,
            NativeApplicationResult::UnknownAfterPossibleApplication => {
                ReceiptDisposition::ReconciliationRequired
            }
        };

        Ok(ApplicationReceipt {
            ticket_id: ticket.ticket_id.clone(),
            attempt_key: ticket.attempt_key.clone(),
            binding_commitment: ticket.binding.commitment,
            controller_epoch: ticket.binding.controller_epoch,
            application_sequence: sequence,
            disposition,
        })
    }

    /// Resolve the one outstanding native application using an independently
    /// qualified M1 outcome. A new mutation remains blocked until this method
    /// clears the pending attempt with a strictly newer authority snapshot.
    ///
    /// NoEffect and PartialEffect require an upstream M1-approved recovery or
    /// replan permit commitment. Satisfied does not. Unknown retains the hold.
    pub fn resolve_pending_action(
        &self,
        attempt_key: &str,
        resolution: QualifiedOutcomeResolution,
        next_snapshot: Option<LiveAuthoritySnapshot>,
        recovery_permit_digest: Option<&str>,
    ) -> Result<ResolutionDisposition, Rejection> {
        let mut state = self.state.lock().map_err(|_| Rejection::LockPoisoned)?;
        let pending = state
            .pending_action_attempt
            .as_deref()
            .ok_or(Rejection::NoPendingAction)?;
        if pending != attempt_key {
            return Err(Rejection::PendingAttemptMismatch);
        }

        if resolution == QualifiedOutcomeResolution::Unknown {
            if next_snapshot.is_some() || recovery_permit_digest.is_some() {
                return Err(Rejection::InvalidNextSnapshot);
            }
            return Ok(ResolutionDisposition::HoldRetained);
        }

        let next = next_snapshot.ok_or(Rejection::InvalidNextSnapshot)?;
        if next.active_attempt_key.trim().is_empty()
            || next.active_attempt_key == attempt_key
            || next.binding == state.live.binding
            || next.binding.controller_epoch <= state.live.binding.controller_epoch
        {
            return Err(Rejection::InvalidNextSnapshot);
        }

        match resolution {
            QualifiedOutcomeResolution::Satisfied => {
                if recovery_permit_digest.is_some() {
                    return Err(Rejection::InvalidNextSnapshot);
                }
            }
            QualifiedOutcomeResolution::NoEffect
            | QualifiedOutcomeResolution::PartialEffect => {
                if recovery_permit_digest
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_none()
                {
                    return Err(Rejection::RecoveryPermitMissing);
                }
            }
            QualifiedOutcomeResolution::Unknown => unreachable!("handled above"),
        }

        state.live = next;
        state.pending_action_attempt = None;
        Ok(ResolutionDisposition::ClearedForNewAuthoritySnapshot)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    fn binding(commitment_byte: u8, epoch: u64) -> AuthorityBinding {
        AuthorityBinding {
            commitment: [commitment_byte; 32],
            controller_epoch: epoch,
            clock_domain_id: "platform-monotonic".to_owned(),
            boot_epoch_id: "boot-9".to_owned(),
        }
    }

    fn live(attempt: &str, commitment_byte: u8, epoch: u64) -> LiveAuthoritySnapshot {
        LiveAuthoritySnapshot {
            active_attempt_key: attempt.to_owned(),
            binding: binding(commitment_byte, epoch),
        }
    }

    fn ticket(attempt: &str) -> ActionTicket {
        ActionTicket {
            ticket_id: "ticket-17".to_owned(),
            attempt_key: attempt.to_owned(),
            binding: binding(7, 3),
            issued_at_ticks: 100,
            expires_at_ticks: 200,
        }
    }

    fn now(ticks: u64) -> QualifiedMonotonicInstant {
        QualifiedMonotonicInstant {
            clock_domain_id: "platform-monotonic".to_owned(),
            boot_epoch_id: "boot-9".to_owned(),
            ticks,
        }
    }

    #[test]
    fn stable_ticket_applies_once_and_receipt_binds_snapshot() {
        let boundary = ApplicationBoundary::new(live("mission-42/action-7/attempt-1", 7, 3));
        let calls = AtomicUsize::new(0);
        let receipt = boundary
            .apply(&ticket("mission-42/action-7/attempt-1"), &now(150), |ctx| {
                calls.fetch_add(1, Ordering::SeqCst);
                assert_eq!(ctx.application_sequence, 1);
                assert_eq!(ctx.binding.commitment, [7; 32]);
                NativeApplicationResult::Accepted
            })
            .expect("valid ticket should apply");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(receipt.disposition, ReceiptDisposition::AcceptedByNativeController);
        assert_eq!(receipt.application_sequence, 1);
        assert_eq!(receipt.controller_epoch, 3);
        assert_eq!(receipt.binding_commitment, [7; 32]);
    }

    #[test]
    fn changed_authority_binding_rejects_before_native_sink() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        boundary.replace_live_snapshot(live("attempt-1", 8, 4)).unwrap();
        let calls = AtomicUsize::new(0);
        let result = boundary.apply(&ticket("attempt-1"), &now(150), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(result, Err(Rejection::BindingMismatch));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn stale_m1_attempt_rejects_before_native_sink() {
        let boundary = ApplicationBoundary::new(live("attempt-2", 7, 3));
        let calls = AtomicUsize::new(0);
        let result = boundary.apply(&ticket("attempt-1"), &now(150), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(result, Err(Rejection::StaleAttempt));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn exclusive_expiry_boundary_rejects_before_native_sink() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        let calls = AtomicUsize::new(0);
        let result = boundary.apply(&ticket("attempt-1"), &now(200), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(result, Err(Rejection::Expired));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn clock_or_boot_mismatch_rejects_before_native_sink() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        let wrong_clock = QualifiedMonotonicInstant {
            clock_domain_id: "other-clock".to_owned(),
            boot_epoch_id: "boot-9".to_owned(),
            ticks: 150,
        };
        let calls = AtomicUsize::new(0);
        let result = boundary.apply(&ticket("attempt-1"), &wrong_clock, |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(result, Err(Rejection::ClockBasisMismatch));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn same_ticket_attempt_cannot_be_replayed() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        let calls = AtomicUsize::new(0);
        let first = boundary.apply(&ticket("attempt-1"), &now(150), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        let second = boundary.apply(&ticket("attempt-1"), &now(151), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert!(first.is_ok());
        assert_eq!(second, Err(Rejection::Replay));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn unknown_native_result_consumes_ticket_and_requires_reconciliation() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        let calls = AtomicUsize::new(0);
        let first = boundary
            .apply(&ticket("attempt-1"), &now(150), |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                NativeApplicationResult::UnknownAfterPossibleApplication
            })
            .unwrap();
        let second = boundary.apply(&ticket("attempt-1"), &now(151), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(first.disposition, ReceiptDisposition::ReconciliationRequired);
        assert_eq!(second, Err(Rejection::Replay));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn application_that_linearizes_first_keeps_pre_invalidation_receipt() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        let receipt = boundary
            .apply(&ticket("attempt-1"), &now(150), |_| NativeApplicationResult::Accepted)
            .unwrap();
        boundary.replace_live_snapshot(live("attempt-1", 8, 4)).unwrap();
        assert_eq!(receipt.binding_commitment, [7; 32]);
        assert_eq!(receipt.controller_epoch, 3);
        let calls = AtomicUsize::new(0);
        let next = boundary.apply(&ticket("attempt-1"), &now(151), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(next, Err(Rejection::BindingMismatch));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn revocation_that_linearizes_first_rejects_queued_ticket() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        boundary.replace_live_snapshot(live("attempt-1", 8, 4)).unwrap();
        let calls = AtomicUsize::new(0);
        let result = boundary.apply(&ticket("attempt-1"), &now(150), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(result, Err(Rejection::BindingMismatch));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn concurrent_duplicate_submissions_have_exactly_one_sink_call() {
        let boundary = Arc::new(ApplicationBoundary::new(live("attempt-1", 7, 3)));
        let barrier = Arc::new(Barrier::new(3));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for _ in 0..2 {
            let boundary = Arc::clone(&boundary);
            let barrier = Arc::clone(&barrier);
            let calls = Arc::clone(&calls);
            let request = ticket("attempt-1");
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                boundary.apply(&request, &now(150), |_| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    NativeApplicationResult::Accepted
                })
            }));
        }

        barrier.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|handle| handle.join().expect("worker thread must not panic"))
            .collect();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| result.as_ref().err() == Some(&Rejection::Replay))
                .count(),
            1
        );
    }


    #[test]
    fn a_ticket_id_cannot_be_reused_for_a_later_attempt() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        let calls = AtomicUsize::new(0);
        boundary
            .apply(&ticket("attempt-1"), &now(150), |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                NativeApplicationResult::Accepted
            })
            .unwrap();

        boundary.replace_live_snapshot(live("attempt-2", 8, 4)).unwrap();
        let mut later_attempt = ticket("attempt-2");
        later_attempt.binding = binding(8, 4);
        let result = boundary.apply(&later_attempt, &now(151), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });

        assert_eq!(result, Err(Rejection::Replay));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn authority_update_waits_until_native_application_callback_finishes() {
        use std::sync::mpsc::channel;

        let boundary = Arc::new(ApplicationBoundary::new(live("attempt-1", 7, 3)));
        let apply_boundary = Arc::clone(&boundary);
        let (callback_entered_tx, callback_entered_rx) = channel();
        let (release_callback_tx, release_callback_rx) = channel();

        let apply_thread = std::thread::spawn(move || {
            apply_boundary.apply(&ticket("attempt-1"), &now(150), |_| {
                callback_entered_tx.send(()).unwrap();
                release_callback_rx.recv().unwrap();
                NativeApplicationResult::Accepted
            })
        });

        callback_entered_rx.recv().unwrap();

        let update_boundary = Arc::clone(&boundary);
        let (update_started_tx, update_started_rx) = channel();
        let update_thread = std::thread::spawn(move || {
            update_started_tx.send(()).unwrap();
            update_boundary.replace_live_snapshot(live("attempt-1", 8, 4))
        });
        update_started_rx.recv().unwrap();

        // The callback is still inside apply's critical section. Releasing it
        // establishes application-before-invalidation as the legal order.
        release_callback_tx.send(()).unwrap();
        let receipt = apply_thread.join().unwrap().unwrap();
        update_thread.join().unwrap().unwrap();

        assert_eq!(receipt.disposition, ReceiptDisposition::AcceptedByNativeController);
        assert_eq!(receipt.binding_commitment, [7; 32]);
        assert_eq!(receipt.controller_epoch, 3);

        let calls = AtomicUsize::new(0);
        let stale = boundary.apply(&ticket("attempt-1"), &now(151), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(stale, Err(Rejection::BindingMismatch));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn invalid_validity_interval_rejects_before_native_sink() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        let mut malformed = ticket("attempt-1");
        malformed.expires_at_ticks = malformed.issued_at_ticks;
        let calls = AtomicUsize::new(0);
        let result = boundary.apply(&malformed, &now(150), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(result, Err(Rejection::InvalidValidityInterval));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn ticket_from_future_rejects_before_native_sink() {
        let boundary = ApplicationBoundary::new(live("attempt-1", 7, 3));
        let calls = AtomicUsize::new(0);
        let result = boundary.apply(&ticket("attempt-1"), &now(99), |_| {
            calls.fetch_add(1, Ordering::SeqCst);
            NativeApplicationResult::Accepted
        });
        assert_eq!(result, Err(Rejection::NotYetValid));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}


mod history;
pub use history::{
    check_history, HistoryCheckResult, HistoryObservedResult, HistoryOperation,
    HistoryOperationKind, IncompleteReason, OperationHistory, DEFAULT_SEARCH_BUDGET,
    MAX_HISTORY_OPERATIONS,
};
