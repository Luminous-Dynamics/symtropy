//! Bounded linearizability checker for authority-bound native application histories.
//!
//! This checker validates *observed histories* against a sequential reference
//! specification. It does not instrument a production controller, establish
//! event-recorder integrity, or prove unbounded correctness. Event indices
//! must come from one complete ordered trace recorder. Missing/truncated
//! instrumentation returns Incomplete, never Pass.

use super::{
    ActionTicket, AuthorityBinding, LiveAuthoritySnapshot, QualifiedMonotonicInstant,
    ReceiptDisposition, Rejection, QualifiedOutcomeResolution, ResolutionDisposition,
    RecoveryPermitReference,
};
use std::collections::BTreeSet;

pub const MAX_HISTORY_OPERATIONS: usize = 20;
pub const DEFAULT_SEARCH_BUDGET: usize = 250_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryOperationKind {
    ReplaceLiveSnapshot {
        next: LiveAuthoritySnapshot,
    },
    Apply {
        ticket: ActionTicket,
        now: QualifiedMonotonicInstant,
    },
    ResolvePendingAction {
        attempt_key: String,
        resolution: QualifiedOutcomeResolution,
        next_snapshot: Option<LiveAuthoritySnapshot>,
        recovery_permit: Option<RecoveryPermitReference>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryObservedResult {
    SnapshotReplaced,
    Rejected(Rejection),
    Applied {
        disposition: ReceiptDisposition,
        application_sequence: u64,
        binding_commitment: [u8; 32],
        controller_epoch: u64,
    },
    PendingActionResolved {
        disposition: ResolutionDisposition,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HistoryOperation {
    pub operation_id: String,
    pub invoked_at: u64,
    pub responded_at: Option<u64>,
    pub kind: HistoryOperationKind,
    pub observed: Option<HistoryObservedResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationHistory {
    /// False when the recorder reports any dropped, truncated or unflushed
    /// event. An incomplete trace is never eligible for PASS.
    pub trace_complete: bool,
    pub initial_snapshot: LiveAuthoritySnapshot,
    pub operations: Vec<HistoryOperation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IncompleteReason {
    TraceRecorderIncomplete,
    TooManyOperations { actual: usize, maximum: usize },
    EmptyOperationId,
    DuplicateOperationId,
    MissingResponse,
    MissingObservedResult,
    InvalidEventInterval,
    DuplicateEventIndex,
    SearchBudgetExhausted { budget: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HistoryCheckResult {
    Linearizable {
        sequential_order: Vec<String>,
        states_explored: usize,
    },
    NonLinearizable {
        states_explored: usize,
    },
    Incomplete {
        reason: IncompleteReason,
        states_explored: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SpecState {
    live: LiveAuthoritySnapshot,
    consumed_ticket_ids: BTreeSet<String>,
    pending_action_attempt: Option<String>,
    consumed_recovery_permit_ids: BTreeSet<String>,
    next_application_sequence: u64,
}

impl SpecState {
    fn new(initial_snapshot: LiveAuthoritySnapshot) -> Self {
        Self {
            live: initial_snapshot,
            consumed_ticket_ids: BTreeSet::new(),
            pending_action_attempt: None,
            consumed_recovery_permit_ids: BTreeSet::new(),
            next_application_sequence: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Transition {
    Valid(SpecState),
    Invalid,
}

fn expected_ticket_rejection(
    state: &SpecState,
    ticket: &ActionTicket,
    now: &QualifiedMonotonicInstant,
) -> Option<Rejection> {
    if ticket.ticket_id.trim().is_empty() || ticket.attempt_key.trim().is_empty() {
        return Some(Rejection::EmptyIdentity);
    }
    if ticket.issued_at_ticks >= ticket.expires_at_ticks {
        return Some(Rejection::InvalidValidityInterval);
    }
    if ticket.attempt_key != state.live.active_attempt_key {
        return Some(Rejection::StaleAttempt);
    }
    if ticket.binding != state.live.binding {
        return Some(Rejection::BindingMismatch);
    }
    if now.clock_domain_id != ticket.binding.clock_domain_id
        || now.boot_epoch_id != ticket.binding.boot_epoch_id
    {
        return Some(Rejection::ClockBasisMismatch);
    }
    if now.ticks < ticket.issued_at_ticks {
        return Some(Rejection::NotYetValid);
    }
    if now.ticks >= ticket.expires_at_ticks {
        return Some(Rejection::Expired);
    }
    if state.consumed_ticket_ids.contains(&ticket.ticket_id) {
        return Some(Rejection::Replay);
    }
    if state.pending_action_attempt.is_some() {
        return Some(Rejection::PendingActionExists);
    }
    if state.next_application_sequence.checked_add(1).is_none() {
        return Some(Rejection::SequenceExhausted);
    }
    None
}

fn apply_observed_operation(
    state: &SpecState,
    operation: &HistoryOperation,
) -> Transition {
    let observed = match operation.observed.as_ref() {
        Some(observed) => observed,
        None => return Transition::Invalid,
    };

    match (&operation.kind, observed) {
        (
            HistoryOperationKind::ReplaceLiveSnapshot { next },
            HistoryObservedResult::SnapshotReplaced,
        ) => {
            if next.binding.controller_epoch < state.live.binding.controller_epoch {
                return Transition::Invalid;
            }
            let mut next_state = state.clone();
            next_state.live = next.clone();
            Transition::Valid(next_state)
        }
        (
            HistoryOperationKind::ReplaceLiveSnapshot { next },
            HistoryObservedResult::Rejected(Rejection::BindingMismatch),
        ) if next.binding.controller_epoch < state.live.binding.controller_epoch => {
            Transition::Valid(state.clone())
        }
        (
            HistoryOperationKind::Apply { ticket, now },
            HistoryObservedResult::Rejected(observed_rejection),
        ) => {
            match expected_ticket_rejection(state, ticket, now) {
                Some(expected_rejection) if &expected_rejection == observed_rejection => {
                    Transition::Valid(state.clone())
                }
                _ => Transition::Invalid,
            }
        }
        (
            HistoryOperationKind::Apply { ticket, now },
            HistoryObservedResult::Applied {
                disposition,
                application_sequence,
                binding_commitment,
                controller_epoch,
            },
        ) => {
            if expected_ticket_rejection(state, ticket, now).is_some() {
                return Transition::Invalid;
            }
            if *application_sequence != state.next_application_sequence
                || *binding_commitment != ticket.binding.commitment
                || *controller_epoch != ticket.binding.controller_epoch
            {
                return Transition::Invalid;
            }
            let mut next_state = state.clone();
            next_state
                .consumed_ticket_ids
                .insert(ticket.ticket_id.clone());
            next_state.pending_action_attempt = Some(ticket.attempt_key.clone());
            next_state.next_application_sequence += 1;
            // Every native application occupies the single-mutation slot
            // until an independently qualified M1 outcome is reconciled.
            match disposition {
                ReceiptDisposition::AcceptedByNativeController
                | ReceiptDisposition::RejectedNoEffect
                | ReceiptDisposition::ReconciliationRequired => {
                    Transition::Valid(next_state)
                }
            }
        }
        (
            HistoryOperationKind::ResolvePendingAction {
                attempt_key,
                resolution,
                next_snapshot,
                recovery_permit,
            },
            HistoryObservedResult::PendingActionResolved { disposition },
        ) => {
            if state.pending_action_attempt.as_deref() != Some(attempt_key.as_str()) {
                return Transition::Invalid;
            }
            if *resolution == QualifiedOutcomeResolution::Unknown {
                if next_snapshot.is_some() || recovery_permit.is_some()
                    || *disposition != ResolutionDisposition::HoldRetained
                {
                    return Transition::Invalid;
                }
                return Transition::Valid(state.clone());
            }

            let next = match next_snapshot {
                Some(next) => next,
                None => return Transition::Invalid,
            };
            if next.active_attempt_key.trim().is_empty()
                || next.active_attempt_key == *attempt_key
                || next.binding == state.live.binding
                || next.binding.controller_epoch <= state.live.binding.controller_epoch
                || *disposition != ResolutionDisposition::ClearedForNewAuthoritySnapshot
            {
                return Transition::Invalid;
            }
            match resolution {
                QualifiedOutcomeResolution::Satisfied => {
                    if recovery_permit.is_some() {
                        return Transition::Invalid;
                    }
                }
                QualifiedOutcomeResolution::NoEffect
                | QualifiedOutcomeResolution::PartialEffect => {
                    let permit = match recovery_permit {
                        Some(permit) => permit,
                        None => return Transition::Invalid,
                    };
                    if permit.permit_id.trim().is_empty()
                        || permit.failed_attempt_key != *attempt_key
                        || permit.authority_epoch != state.live.binding.controller_epoch
                        || permit.plan_digest == [0; 32]
                    || permit.plan_digest != next.binding.commitment
                        || state.consumed_recovery_permit_ids.contains(&permit.permit_id)
                    {
                        return Transition::Invalid;
                    }
                }
                QualifiedOutcomeResolution::Unknown => unreachable!("handled above"),
            }

            let mut next_state = state.clone();
            if let Some(permit) = recovery_permit {
                next_state
                    .consumed_recovery_permit_ids
                    .insert(permit.permit_id.clone());
            }
            next_state.live = next.clone();
            next_state.pending_action_attempt = None;
            Transition::Valid(next_state)
        }
        _ => Transition::Invalid,
    }
}

fn validate_history(history: &OperationHistory) -> Result<(), IncompleteReason> {
    if !history.trace_complete {
        return Err(IncompleteReason::TraceRecorderIncomplete);
    }
    if history.operations.len() > MAX_HISTORY_OPERATIONS {
        return Err(IncompleteReason::TooManyOperations {
            actual: history.operations.len(),
            maximum: MAX_HISTORY_OPERATIONS,
        });
    }

    let mut operation_ids = BTreeSet::new();
    let mut event_indices = BTreeSet::new();

    for operation in &history.operations {
        if operation.operation_id.trim().is_empty() {
            return Err(IncompleteReason::EmptyOperationId);
        }
        if !operation_ids.insert(operation.operation_id.clone()) {
            return Err(IncompleteReason::DuplicateOperationId);
        }
        let responded_at = operation
            .responded_at
            .ok_or(IncompleteReason::MissingResponse)?;
        if operation.observed.is_none() {
            return Err(IncompleteReason::MissingObservedResult);
        }
        if operation.invoked_at >= responded_at {
            return Err(IncompleteReason::InvalidEventInterval);
        }
        if !event_indices.insert(operation.invoked_at)
            || !event_indices.insert(responded_at)
        {
            return Err(IncompleteReason::DuplicateEventIndex);
        }
    }
    Ok(())
}

struct SearchContext<'a> {
    operations: &'a [HistoryOperation],
    predecessors: Vec<u64>,
    all_mask: u64,
    explored: usize,
    budget: usize,
}

enum SearchOutcome {
    Found(Vec<usize>),
    Exhausted,
    NotFound,
}

fn search(
    context: &mut SearchContext<'_>,
    completed_mask: u64,
    state: &SpecState,
    path: &mut Vec<usize>,
) -> SearchOutcome {
    if completed_mask == context.all_mask {
        return SearchOutcome::Found(path.clone());
    }

    let mut had_candidate = false;
    for index in 0..context.operations.len() {
        let bit = 1u64 << index;
        if completed_mask & bit != 0 {
            continue;
        }
        if context.predecessors[index] & !completed_mask != 0 {
            continue;
        }
        had_candidate = true;

        if context.explored >= context.budget {
            return SearchOutcome::Exhausted;
        }
        context.explored += 1;

        match apply_observed_operation(state, &context.operations[index]) {
            Transition::Invalid => continue,
            Transition::Valid(next_state) => {
                path.push(index);
                match search(context, completed_mask | bit, &next_state, path) {
                    SearchOutcome::Found(order) => return SearchOutcome::Found(order),
                    SearchOutcome::Exhausted => return SearchOutcome::Exhausted,
                    SearchOutcome::NotFound => {
                        path.pop();
                    }
                }
            }
        }
    }

    if had_candidate {
        SearchOutcome::NotFound
    } else {
        // A cycle in real-time precedence cannot have a legal sequential order.
        SearchOutcome::NotFound
    }
}

/// Check a complete invocation/response history against the authority-bound
/// sequential specification. Real-time precedence is derived from the
/// recorder's globally ordered event indices: response(A) < invocation(B)
/// forces A before B. Overlapping operations may be ordered either way.
///
/// A PASS is existential: one legal sequential ordering suffices. A FAIL is
/// returned only after every admissible ordering within the declared bound
/// has been ruled out. Truncated traces and exhausted searches are INCOMPLETE.
pub fn check_history(
    history: &OperationHistory,
    search_budget: usize,
) -> HistoryCheckResult {
    if let Err(reason) = validate_history(history) {
        return HistoryCheckResult::Incomplete {
            reason,
            states_explored: 0,
        };
    }

    let count = history.operations.len();
    if count == 0 {
        return HistoryCheckResult::Linearizable {
            sequential_order: Vec::new(),
            states_explored: 0,
        };
    }

    let mut predecessors = vec![0u64; count];
    for (later_index, later) in history.operations.iter().enumerate() {
        for (earlier_index, earlier) in history.operations.iter().enumerate() {
            if earlier_index == later_index {
                continue;
            }
            let earlier_response = earlier.responded_at.expect("validated response");
            if earlier_response < later.invoked_at {
                predecessors[later_index] |= 1u64 << earlier_index;
            }
        }
    }

    let all_mask = (1u64 << count) - 1;
    let mut context = SearchContext {
        operations: &history.operations,
        predecessors,
        all_mask,
        explored: 0,
        budget: search_budget,
    };
    let mut path = Vec::with_capacity(count);
    let initial_state = SpecState::new(history.initial_snapshot.clone());

    match search(&mut context, 0, &initial_state, &mut path) {
        SearchOutcome::Found(indices) => HistoryCheckResult::Linearizable {
            sequential_order: indices
                .into_iter()
                .map(|index| history.operations[index].operation_id.clone())
                .collect(),
            states_explored: context.explored,
        },
        SearchOutcome::NotFound => HistoryCheckResult::NonLinearizable {
            states_explored: context.explored,
        },
        SearchOutcome::Exhausted => HistoryCheckResult::Incomplete {
            reason: IncompleteReason::SearchBudgetExhausted {
                budget: search_budget,
            },
            states_explored: context.explored,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(commitment_byte: u8, epoch: u64) -> AuthorityBinding {
        AuthorityBinding {
            commitment: [commitment_byte; 32],
            controller_epoch: epoch,
            clock_domain_id: "platform-clock".to_owned(),
            boot_epoch_id: "boot-a".to_owned(),
        }
    }

    fn snapshot(attempt: &str, commitment_byte: u8, epoch: u64) -> LiveAuthoritySnapshot {
        LiveAuthoritySnapshot {
            active_attempt_key: attempt.to_owned(),
            binding: binding(commitment_byte, epoch),
        }
    }

    fn ticket(id: &str, attempt: &str, commitment_byte: u8, epoch: u64) -> ActionTicket {
        ActionTicket {
            ticket_id: id.to_owned(),
            attempt_key: attempt.to_owned(),
            binding: binding(commitment_byte, epoch),
            issued_at_ticks: 10,
            expires_at_ticks: 100,
        }
    }

    fn now(ticks: u64) -> QualifiedMonotonicInstant {
        QualifiedMonotonicInstant {
            clock_domain_id: "platform-clock".to_owned(),
            boot_epoch_id: "boot-a".to_owned(),
            ticks,
        }
    }

    fn applied(seq: u64, b: &AuthorityBinding, disposition: ReceiptDisposition) -> HistoryObservedResult {
        HistoryObservedResult::Applied {
            disposition,
            application_sequence: seq,
            binding_commitment: b.commitment,
            controller_epoch: b.controller_epoch,
        }
    }

    fn op(
        id: &str,
        invoked_at: u64,
        responded_at: u64,
        kind: HistoryOperationKind,
        observed: HistoryObservedResult,
    ) -> HistoryOperation {
        HistoryOperation {
            operation_id: id.to_owned(),
            invoked_at,
            responded_at: Some(responded_at),
            kind,
            observed: Some(observed),
        }
    }

    #[test]
    fn overlapping_apply_can_linearize_before_overlapping_revocation() {
        let b = binding(1, 1);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "apply-1",
                    1,
                    4,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-1", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &b, ReceiptDisposition::AcceptedByNativeController),
                ),
                op(
                    "revoke",
                    2,
                    5,
                    HistoryOperationKind::ReplaceLiveSnapshot {
                        next: snapshot("attempt-1", 2, 2),
                    },
                    HistoryObservedResult::SnapshotReplaced,
                ),
            ],
        };

        match check_history(&history, DEFAULT_SEARCH_BUDGET) {
            HistoryCheckResult::Linearizable { sequential_order, .. } => {
                assert_eq!(sequential_order, vec!["apply-1", "revoke"]);
            }
            other => panic!("expected legal apply-before-revoke order, got {other:?}"),
        }
    }

    #[test]
    fn completed_revocation_before_apply_makes_old_ticket_history_illegal() {
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "revoke",
                    1,
                    2,
                    HistoryOperationKind::ReplaceLiveSnapshot {
                        next: snapshot("attempt-1", 2, 2),
                    },
                    HistoryObservedResult::SnapshotReplaced,
                ),
                op(
                    "stale-apply",
                    3,
                    4,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-1", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &binding(1, 1), ReceiptDisposition::AcceptedByNativeController),
                ),
            ],
        };

        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::NonLinearizable { .. }
        ));
    }

    #[test]
    fn a_ticket_cannot_be_applied_twice_even_across_distinct_attempts() {
        let first_binding = binding(1, 1);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "first-apply",
                    1,
                    2,
                    HistoryOperationKind::Apply {
                        ticket: ticket("global-ticket", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &first_binding, ReceiptDisposition::AcceptedByNativeController),
                ),
                op(
                    "advance-attempt",
                    3,
                    4,
                    HistoryOperationKind::ReplaceLiveSnapshot {
                        next: snapshot("attempt-2", 2, 2),
                    },
                    HistoryObservedResult::SnapshotReplaced,
                ),
                op(
                    "ticket-reuse",
                    5,
                    6,
                    HistoryOperationKind::Apply {
                        ticket: ticket("global-ticket", "attempt-2", 2, 2),
                        now: now(30),
                    },
                    applied(2, &binding(2, 2), ReceiptDisposition::AcceptedByNativeController),
                ),
            ],
        };

        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::NonLinearizable { .. }
        ));
    }

    #[test]
    fn unknown_native_outcome_consumes_ticket_and_retry_is_replay() {
        let b = binding(1, 1);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "unknown-apply",
                    1,
                    2,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-unknown", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &b, ReceiptDisposition::ReconciliationRequired),
                ),
                op(
                    "blind-retry",
                    3,
                    4,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-unknown", "attempt-1", 1, 1),
                        now: now(21),
                    },
                    HistoryObservedResult::Rejected(Rejection::Replay),
                ),
            ],
        };

        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::Linearizable { .. }
        ));
    }

    #[test]
    fn missing_response_is_incomplete_not_pass() {
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![HistoryOperation {
                operation_id: "pending".to_owned(),
                invoked_at: 1,
                responded_at: None,
                kind: HistoryOperationKind::Apply {
                    ticket: ticket("ticket-1", "attempt-1", 1, 1),
                    now: now(20),
                },
                observed: None,
            }],
        };

        assert_eq!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::Incomplete {
                reason: IncompleteReason::MissingResponse,
                states_explored: 0
            }
        );
    }

    #[test]
    fn incomplete_event_recorder_never_passes() {
        let history = OperationHistory {
            trace_complete: false,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: Vec::new(),
        };
        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::Incomplete {
                reason: IncompleteReason::TraceRecorderIncomplete,
                ..
            }
        ));
    }

    #[test]
    fn search_budget_exhaustion_is_incomplete() {
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "wrong-result-1",
                    1,
                    4,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-1", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    HistoryObservedResult::Rejected(Rejection::Expired),
                ),
                op(
                    "wrong-result-2",
                    2,
                    5,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-2", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    HistoryObservedResult::Rejected(Rejection::Expired),
                ),
                op(
                    "wrong-result-3",
                    3,
                    6,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-3", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    HistoryObservedResult::Rejected(Rejection::Expired),
                ),
            ],
        };

        assert!(matches!(
            check_history(&history, 1),
            HistoryCheckResult::Incomplete {
                reason: IncompleteReason::SearchBudgetExhausted { budget: 1 },
                ..
            }
        ));
    }

    #[test]
    fn complete_wrong_application_receipt_is_non_linearizable() {
        let b = binding(1, 1);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![op(
                "bad-sequence",
                1,
                2,
                HistoryOperationKind::Apply {
                    ticket: ticket("ticket-1", "attempt-1", 1, 1),
                    now: now(20),
                },
                applied(99, &b, ReceiptDisposition::AcceptedByNativeController),
            )],
        };

        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::NonLinearizable { .. }
        ));
    }
    #[test]
    fn applied_action_blocks_another_mutation_until_outcome_is_resolved() {
        let b = binding(1, 1);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "apply-1",
                    1,
                    2,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-1", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &b, ReceiptDisposition::AcceptedByNativeController),
                ),
                op(
                    "second-mutation",
                    3,
                    4,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-2", "attempt-1", 1, 1),
                        now: now(21),
                    },
                    HistoryObservedResult::Rejected(Rejection::PendingActionExists),
                ),
            ],
        };

        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::Linearizable { .. }
        ));
    }

    #[test]
    fn unknown_result_holds_until_explicit_reconciliation() {
        let b = binding(1, 1);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "unknown-apply",
                    1,
                    2,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-1", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &b, ReceiptDisposition::ReconciliationRequired),
                ),
                op(
                    "still-unknown",
                    3,
                    4,
                    HistoryOperationKind::ResolvePendingAction {
                        attempt_key: "attempt-1".to_owned(),
                        resolution: QualifiedOutcomeResolution::Unknown,
                        next_snapshot: None,
                        recovery_permit: None,
                    },
                    HistoryObservedResult::PendingActionResolved {
                        disposition: ResolutionDisposition::HoldRetained,
                    },
                ),
                op(
                    "blocked-mutation",
                    5,
                    6,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-2", "attempt-1", 1, 1),
                        now: now(22),
                    },
                    HistoryObservedResult::Rejected(Rejection::PendingActionExists),
                ),
            ],
        };

        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::Linearizable { .. }
        ));
    }

    #[test]
    fn no_effect_recovery_requires_permit_and_new_authority_snapshot() {
        let old_binding = binding(1, 1);
        let new_binding = binding(2, 2);
        let next = snapshot("attempt-2", 2, 2);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "apply-1",
                    1,
                    2,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-1", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &old_binding, ReceiptDisposition::RejectedNoEffect),
                ),
                op(
                    "recover-1",
                    3,
                    4,
                    HistoryOperationKind::ResolvePendingAction {
                        attempt_key: "attempt-1".to_owned(),
                        resolution: QualifiedOutcomeResolution::NoEffect,
                        next_snapshot: Some(next),
                        recovery_permit: Some(RecoveryPermitReference {
                            permit_id: "permit-2".to_owned(),
                            failed_attempt_key: "attempt-1".to_owned(),
                            plan_digest: [2; 32],
                            authority_epoch: 1,
                        }),
                    },
                    HistoryObservedResult::PendingActionResolved {
                        disposition: ResolutionDisposition::ClearedForNewAuthoritySnapshot,
                    },
                ),
                op(
                    "apply-new-attempt",
                    5,
                    6,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-2", "attempt-2", 2, 2),
                        now: now(30),
                    },
                    applied(2, &new_binding, ReceiptDisposition::AcceptedByNativeController),
                ),
            ],
        };

        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::Linearizable { .. }
        ));
    }

    #[test]
    fn no_effect_recovery_without_permit_is_not_linearizable() {
        let old_binding = binding(1, 1);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "apply-1",
                    1,
                    2,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-1", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &old_binding, ReceiptDisposition::RejectedNoEffect),
                ),
                op(
                    "unauthorized-recovery",
                    3,
                    4,
                    HistoryOperationKind::ResolvePendingAction {
                        attempt_key: "attempt-1".to_owned(),
                        resolution: QualifiedOutcomeResolution::NoEffect,
                        next_snapshot: Some(snapshot("attempt-2", 2, 2)),
                        recovery_permit: None,
                    },
                    HistoryObservedResult::PendingActionResolved {
                        disposition: ResolutionDisposition::ClearedForNewAuthoritySnapshot,
                    },
                ),
            ],
        };

        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::NonLinearizable { .. }
        ));
    }

    #[test]
    fn recovery_permit_id_cannot_be_replayed_for_a_later_attempt() {
        let b1 = binding(1, 1);
        let b2 = binding(2, 2);
        let b3 = binding(3, 3);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "apply-1",
                    1,
                    2,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-1", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &b1, ReceiptDisposition::RejectedNoEffect),
                ),
                op(
                    "resolve-1",
                    3,
                    4,
                    HistoryOperationKind::ResolvePendingAction {
                        attempt_key: "attempt-1".to_owned(),
                        resolution: QualifiedOutcomeResolution::NoEffect,
                        next_snapshot: Some(snapshot("attempt-2", 2, 2)),
                        recovery_permit: Some(RecoveryPermitReference {
                            permit_id: "one-shot-permit".to_owned(),
                            failed_attempt_key: "attempt-1".to_owned(),
                            plan_digest: [2; 32],
                            authority_epoch: 1,
                        }),
                    },
                    HistoryObservedResult::PendingActionResolved {
                        disposition: ResolutionDisposition::ClearedForNewAuthoritySnapshot,
                    },
                ),
                op(
                    "apply-2",
                    5,
                    6,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-2", "attempt-2", 2, 2),
                        now: now(30),
                    },
                    applied(2, &b2, ReceiptDisposition::RejectedNoEffect),
                ),
                op(
                    "resolve-2-replayed-permit",
                    7,
                    8,
                    HistoryOperationKind::ResolvePendingAction {
                        attempt_key: "attempt-2".to_owned(),
                        resolution: QualifiedOutcomeResolution::NoEffect,
                        next_snapshot: Some(snapshot("attempt-3", 3, 3)),
                        recovery_permit: Some(RecoveryPermitReference {
                            permit_id: "one-shot-permit".to_owned(),
                            failed_attempt_key: "attempt-2".to_owned(),
                            plan_digest: [3; 32],
                            authority_epoch: 2,
                        }),
                    },
                    HistoryObservedResult::PendingActionResolved {
                        disposition: ResolutionDisposition::ClearedForNewAuthoritySnapshot,
                    },
                ),
            ],
        };

        assert_ne!(b1, b3);
        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::NonLinearizable { .. }
        ));
    }

    #[test]
    fn recovery_permit_for_different_next_binding_is_non_linearizable() {
        let old_binding = binding(1, 1);
        let history = OperationHistory {
            trace_complete: true,
            initial_snapshot: snapshot("attempt-1", 1, 1),
            operations: vec![
                op(
                    "apply-1",
                    1,
                    2,
                    HistoryOperationKind::Apply {
                        ticket: ticket("ticket-1", "attempt-1", 1, 1),
                        now: now(20),
                    },
                    applied(1, &old_binding, ReceiptDisposition::RejectedNoEffect),
                ),
                op(
                    "wrong-plan-recovery",
                    3,
                    4,
                    HistoryOperationKind::ResolvePendingAction {
                        attempt_key: "attempt-1".to_owned(),
                        resolution: QualifiedOutcomeResolution::NoEffect,
                        next_snapshot: Some(snapshot("attempt-2", 2, 2)),
                        recovery_permit: Some(RecoveryPermitReference {
                            permit_id: "permit-wrong-plan".to_owned(),
                            failed_attempt_key: "attempt-1".to_owned(),
                            plan_digest: [9; 32],
                            authority_epoch: 1,
                        }),
                    },
                    HistoryObservedResult::PendingActionResolved {
                        disposition: ResolutionDisposition::ClearedForNewAuthoritySnapshot,
                    },
                ),
            ],
        };

        assert!(matches!(
            check_history(&history, DEFAULT_SEARCH_BUDGET),
            HistoryCheckResult::NonLinearizable { .. }
        ));
    }

}
