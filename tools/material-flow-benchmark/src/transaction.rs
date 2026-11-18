// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic reference state machine for distributed physical handoff.
//!
//! This models logical authorization, single-issue command gating, physical
//! observations, custody acknowledgement and explicit reconciliation closure.
//! Reconciliation requires a distinct configured custodian and two distinct
//! evidence digests, but this pure state machine does not authenticate the
//! underlying signatures or prove that evidence sources are truly independent.
//! It does not drive hardware, prove that a sensor is truthful, or guarantee
//! exactly-once physical actuation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalTransactionState {
    AwaitingAuthorization,
    Authorized,
    Released,
    Received,
    Completed,
    Rejected,
    SafeHold,
    ReconciliationRequired,
    ReconciledCompleted,
    ReconciledNoAction,
    ReconciledReturned,
    ReconciledQuarantined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchGate {
    DispatchOnce,
    DuplicateSuppressed,
    BlockedByState(PhysicalTransactionState),
    BlockedByUnexpectedCommand,
    BlockedForReconciliation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDisposition {
    Applied,
    DuplicateIgnored,
    Rejected,
    SafeHold,
    ReconciliationRequired,
    Completed,
    Reconciled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionContext {
    pub transaction_id: String,
    pub expected_command_id: String,
    pub package_id: String,
    pub principal_actor_id: String,
    pub source_actor_id: String,
    pub receiver_actor_id: String,
    pub receiver_endpoint_id: String,
    /// Independent operator/custodian allowed to resolve ambiguous outcomes.
    pub reconciler_actor_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionEvent {
    pub transaction_id: String,
    pub event_id: String,
    pub sequence: u64,
    pub actor_id: String,
    pub kind: TransactionEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TransactionEventKind {
    AuthorizationGranted {
        not_before_unix_ms: u64,
        expires_at_unix_ms: u64,
        checked_at_unix_ms: u64,
        audience_matches: bool,
        endpoint_matches: bool,
        scope_matches: bool,
        use_remaining: bool,
    },
    AuthorizationDenied,
    PackageReleased {
        command_id: String,
        package_id: String,
    },
    PackageReceived {
        command_id: String,
        package_id: String,
        endpoint_id: String,
    },
    CustodyAcknowledged {
        command_id: String,
        package_id: String,
        evidence_digest: String,
    },
    PreflightRejected {
        reason: PreflightRejectionReason,
    },
    PreflightSafeHold {
        reason: PreflightHoldReason,
    },
    ActuationTimeout,
    PowerInterruption,
    NetworkPartition,
    SensorDisagreement,
    ReconciliationResolved {
        outcome: ReconciliationOutcome,
        primary_evidence_digest: String,
        independent_evidence_digest: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationOutcome {
    Completed,
    NoActionTaken,
    Returned,
    Quarantined,
}

/// A hard pre-dispatch violation. No physical command may be issued.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightRejectionReason {
    WrongContainer,
    PayloadMassExceeded,
    AuthorizationExpired,
}

/// A condition that requires a safe pause before any physical command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightHoldReason {
    TemperatureObservationStale,
    TransferGateInterlocked,
    SealNotIntact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalTransaction {
    pub context: TransactionContext,
    pub state: PhysicalTransactionState,
    pub dispatch_issued: bool,
    pub release_observed: bool,
    pub receive_observed: bool,
    pub last_sequence: u64,
    seen_events: BTreeMap<String, TransactionEvent>,
}

impl PhysicalTransaction {
    pub fn new(context: TransactionContext) -> Self {
        Self {
            context,
            state: PhysicalTransactionState::AwaitingAuthorization,
            dispatch_issued: false,
            release_observed: false,
            receive_observed: false,
            last_sequence: 0,
            seen_events: BTreeMap::new(),
        }
    }

    /// Authorizes one logical release command. Replaying the same command ID
    /// never issues a second permit; a different command ID after an uncertain
    /// dispatch moves the transaction to reconciliation.
    pub fn request_release_command(&mut self, command_id: &str) -> Result<DispatchGate, String> {
        check_identifier("command_id", command_id)?;
        if self.state != PhysicalTransactionState::Authorized {
            return Ok(DispatchGate::BlockedByState(self.state));
        }
        if self.dispatch_issued {
            if command_id == self.context.expected_command_id {
                return Ok(DispatchGate::DuplicateSuppressed);
            }
            self.state = PhysicalTransactionState::ReconciliationRequired;
            return Ok(DispatchGate::BlockedForReconciliation);
        }
        if command_id != self.context.expected_command_id {
            return Ok(DispatchGate::BlockedByUnexpectedCommand);
        }
        self.dispatch_issued = true;
        Ok(DispatchGate::DispatchOnce)
    }

    /// Applies an independently observed event. A duplicate event ID is
    /// idempotently ignored. A gap, reordered event, mismatched party/item or
    /// ambiguous post-dispatch outcome fails closed.
    pub fn record_event(&mut self, event: TransactionEvent) -> Result<EventDisposition, String> {
        check_identifier("event_id", &event.event_id)?;
        check_identifier("actor_id", &event.actor_id)?;
        check_identifier("transaction_id", &event.transaction_id)?;

        if let Some(previous) = self.seen_events.get(&event.event_id) {
            if previous == &event {
                return Ok(EventDisposition::DuplicateIgnored);
            }
            self.state = PhysicalTransactionState::ReconciliationRequired;
            return Ok(EventDisposition::ReconciliationRequired);
        }
        if event.transaction_id != self.context.transaction_id {
            self.state = PhysicalTransactionState::ReconciliationRequired;
            return Ok(EventDisposition::ReconciliationRequired);
        }
        let Some(expected_sequence) = self.last_sequence.checked_add(1) else {
            self.state = PhysicalTransactionState::ReconciliationRequired;
            return Ok(EventDisposition::ReconciliationRequired);
        };
        if event.sequence != expected_sequence {
            self.state = PhysicalTransactionState::ReconciliationRequired;
            return Ok(EventDisposition::ReconciliationRequired);
        }

        self.last_sequence = event.sequence;
        self.seen_events
            .insert(event.event_id.clone(), event.clone());

        if self.state == PhysicalTransactionState::ReconciliationRequired
            && !matches!(
                &event.kind,
                TransactionEventKind::ReconciliationResolved { .. }
            )
        {
            return Ok(EventDisposition::ReconciliationRequired);
        }
        if matches!(
            self.state,
            PhysicalTransactionState::Rejected
                | PhysicalTransactionState::Completed
                | PhysicalTransactionState::ReconciledCompleted
                | PhysicalTransactionState::ReconciledNoAction
                | PhysicalTransactionState::ReconciledReturned
                | PhysicalTransactionState::ReconciledQuarantined
        ) {
            self.state = PhysicalTransactionState::ReconciliationRequired;
            return Ok(EventDisposition::ReconciliationRequired);
        }

        let disposition = match event.kind {
            TransactionEventKind::AuthorizationGranted {
                not_before_unix_ms,
                expires_at_unix_ms,
                checked_at_unix_ms,
                audience_matches,
                endpoint_matches,
                scope_matches,
                use_remaining,
            } => {
                let valid = self.state == PhysicalTransactionState::AwaitingAuthorization
                    && event.actor_id == self.context.principal_actor_id
                    && not_before_unix_ms <= checked_at_unix_ms
                    && checked_at_unix_ms < expires_at_unix_ms
                    && expires_at_unix_ms > not_before_unix_ms
                    && audience_matches
                    && endpoint_matches
                    && scope_matches
                    && use_remaining;
                if valid {
                    self.state = PhysicalTransactionState::Authorized;
                    EventDisposition::Applied
                } else {
                    self.state = PhysicalTransactionState::Rejected;
                    EventDisposition::Rejected
                }
            }
            TransactionEventKind::AuthorizationDenied => {
                if self.state == PhysicalTransactionState::AwaitingAuthorization
                    && event.actor_id == self.context.principal_actor_id
                {
                    self.state = PhysicalTransactionState::Rejected;
                    EventDisposition::Rejected
                } else {
                    self.state = PhysicalTransactionState::ReconciliationRequired;
                    EventDisposition::ReconciliationRequired
                }
            }
            TransactionEventKind::PackageReleased {
                command_id,
                package_id,
            } => {
                let valid = self.state == PhysicalTransactionState::Authorized
                    && self.dispatch_issued
                    && command_id == self.context.expected_command_id
                    && package_id == self.context.package_id
                    && event.actor_id == self.context.source_actor_id
                    && !self.release_observed;
                if valid {
                    self.release_observed = true;
                    self.state = PhysicalTransactionState::Released;
                    EventDisposition::Applied
                } else {
                    self.state = PhysicalTransactionState::ReconciliationRequired;
                    EventDisposition::ReconciliationRequired
                }
            }
            TransactionEventKind::PackageReceived {
                command_id,
                package_id,
                endpoint_id,
            } => {
                let valid = self.state == PhysicalTransactionState::Released
                    && command_id == self.context.expected_command_id
                    && package_id == self.context.package_id
                    && endpoint_id == self.context.receiver_endpoint_id
                    && event.actor_id == self.context.receiver_actor_id
                    && !self.receive_observed;
                if valid {
                    self.receive_observed = true;
                    self.state = PhysicalTransactionState::Received;
                    EventDisposition::Applied
                } else {
                    self.state = PhysicalTransactionState::ReconciliationRequired;
                    EventDisposition::ReconciliationRequired
                }
            }
            TransactionEventKind::CustodyAcknowledged {
                command_id,
                package_id,
                evidence_digest,
            } => {
                let valid = self.state == PhysicalTransactionState::Received
                    && command_id == self.context.expected_command_id
                    && package_id == self.context.package_id
                    && event.actor_id == self.context.receiver_actor_id
                    && valid_sha256_digest(&evidence_digest);
                if valid {
                    self.state = PhysicalTransactionState::Completed;
                    EventDisposition::Completed
                } else {
                    self.state = PhysicalTransactionState::ReconciliationRequired;
                    EventDisposition::ReconciliationRequired
                }
            }
            TransactionEventKind::PreflightRejected { reason: _ } => {
                let valid = matches!(
                    self.state,
                    PhysicalTransactionState::AwaitingAuthorization
                        | PhysicalTransactionState::Authorized
                ) && !self.dispatch_issued
                    && event.actor_id == self.context.receiver_actor_id;
                if valid {
                    self.state = PhysicalTransactionState::Rejected;
                    EventDisposition::Rejected
                } else {
                    self.state = PhysicalTransactionState::ReconciliationRequired;
                    EventDisposition::ReconciliationRequired
                }
            }
            TransactionEventKind::PreflightSafeHold { reason: _ } => {
                let valid = matches!(
                    self.state,
                    PhysicalTransactionState::AwaitingAuthorization
                        | PhysicalTransactionState::Authorized
                ) && !self.dispatch_issued
                    && event.actor_id == self.context.receiver_actor_id;
                if valid {
                    self.state = PhysicalTransactionState::SafeHold;
                    EventDisposition::SafeHold
                } else {
                    self.state = PhysicalTransactionState::ReconciliationRequired;
                    EventDisposition::ReconciliationRequired
                }
            }
            TransactionEventKind::ActuationTimeout
            | TransactionEventKind::PowerInterruption
            | TransactionEventKind::NetworkPartition => {
                if self.dispatch_issued || self.release_observed {
                    self.state = PhysicalTransactionState::ReconciliationRequired;
                    EventDisposition::ReconciliationRequired
                } else {
                    self.state = PhysicalTransactionState::SafeHold;
                    EventDisposition::SafeHold
                }
            }
            TransactionEventKind::SensorDisagreement => {
                if self.dispatch_issued || self.release_observed || self.receive_observed {
                    self.state = PhysicalTransactionState::ReconciliationRequired;
                    EventDisposition::ReconciliationRequired
                } else {
                    self.state = PhysicalTransactionState::SafeHold;
                    EventDisposition::SafeHold
                }
            }
            TransactionEventKind::ReconciliationResolved {
                outcome,
                primary_evidence_digest,
                independent_evidence_digest,
            } => {
                let valid = self.state == PhysicalTransactionState::ReconciliationRequired
                    && event.actor_id == self.context.reconciler_actor_id
                    && self.context.reconciler_actor_id != self.context.principal_actor_id
                    && self.context.reconciler_actor_id != self.context.source_actor_id
                    && self.context.reconciler_actor_id != self.context.receiver_actor_id
                    && valid_sha256_digest(&primary_evidence_digest)
                    && valid_sha256_digest(&independent_evidence_digest)
                    && primary_evidence_digest != independent_evidence_digest;
                if valid {
                    self.state = match outcome {
                        ReconciliationOutcome::Completed => {
                            PhysicalTransactionState::ReconciledCompleted
                        }
                        ReconciliationOutcome::NoActionTaken => {
                            PhysicalTransactionState::ReconciledNoAction
                        }
                        ReconciliationOutcome::Returned => {
                            PhysicalTransactionState::ReconciledReturned
                        }
                        ReconciliationOutcome::Quarantined => {
                            PhysicalTransactionState::ReconciledQuarantined
                        }
                    };
                    EventDisposition::Reconciled
                } else if self.state == PhysicalTransactionState::ReconciliationRequired {
                    EventDisposition::ReconciliationRequired
                } else {
                    EventDisposition::Rejected
                }
            }
        };
        Ok(disposition)
    }
}

fn check_identifier(path: &str, value: &str) -> Result<(), String> {
    let valid = !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'));
    if valid {
        Ok(())
    } else {
        Err(format!(
            "{path}: expected a non-empty opaque ASCII identifier (max 96 bytes)"
        ))
    }
}

fn valid_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transaction() -> PhysicalTransaction {
        PhysicalTransaction::new(TransactionContext {
            transaction_id: "tx-01".into(),
            expected_command_id: "cmd-01".into(),
            package_id: "pkg-01".into(),
            principal_actor_id: "household-01".into(),
            source_actor_id: "carrier-01".into(),
            receiver_actor_id: "receiver-01".into(),
            receiver_endpoint_id: "nestport-01".into(),
            reconciler_actor_id: "independent-custodian-01".into(),
        })
    }

    fn event(
        sequence: u64,
        event_id: &str,
        actor_id: &str,
        kind: TransactionEventKind,
    ) -> TransactionEvent {
        TransactionEvent {
            transaction_id: "tx-01".into(),
            event_id: event_id.into(),
            sequence,
            actor_id: actor_id.into(),
            kind,
        }
    }

    fn valid_authorization() -> TransactionEventKind {
        TransactionEventKind::AuthorizationGranted {
            not_before_unix_ms: 1_000,
            expires_at_unix_ms: 10_000,
            checked_at_unix_ms: 2_000,
            audience_matches: true,
            endpoint_matches: true,
            scope_matches: true,
            use_remaining: true,
        }
    }

    fn authorize(tx: &mut PhysicalTransaction) {
        let disposition = tx
            .record_event(event(
                1,
                "event-auth",
                "household-01",
                valid_authorization(),
            ))
            .expect("valid authorization event should be accepted");
        assert_eq!(disposition, EventDisposition::Applied);
    }

    fn issue_release(tx: &mut PhysicalTransaction) {
        authorize(tx);
        assert_eq!(
            tx.request_release_command("cmd-01").unwrap(),
            DispatchGate::DispatchOnce
        );
    }

    fn release_event() -> TransactionEventKind {
        TransactionEventKind::PackageReleased {
            command_id: "cmd-01".into(),
            package_id: "pkg-01".into(),
        }
    }

    fn receive_event() -> TransactionEventKind {
        TransactionEventKind::PackageReceived {
            command_id: "cmd-01".into(),
            package_id: "pkg-01".into(),
            endpoint_id: "nestport-01".into(),
        }
    }

    #[test]
    fn release_command_is_issued_only_once() {
        let mut tx = transaction();
        authorize(&mut tx);
        assert_eq!(
            tx.request_release_command("cmd-01").unwrap(),
            DispatchGate::DispatchOnce
        );
        assert_eq!(
            tx.request_release_command("cmd-01").unwrap(),
            DispatchGate::DuplicateSuppressed
        );
        assert!(tx.dispatch_issued);
    }

    #[test]
    fn completion_requires_independent_receiver_acknowledgement() {
        let mut tx = transaction();
        issue_release(&mut tx);
        assert_eq!(
            tx.record_event(event(2, "event-release", "carrier-01", release_event()))
                .unwrap(),
            EventDisposition::Applied
        );
        assert_eq!(
            tx.record_event(event(3, "event-receive", "receiver-01", receive_event()))
                .unwrap(),
            EventDisposition::Applied
        );
        assert_eq!(tx.state, PhysicalTransactionState::Received);

        let ack = TransactionEventKind::CustodyAcknowledged {
            command_id: "cmd-01".into(),
            package_id: "pkg-01".into(),
            evidence_digest: format!("sha256:{}", "a".repeat(64)),
        };
        assert_eq!(
            tx.record_event(event(4, "event-ack", "receiver-01", ack))
                .unwrap(),
            EventDisposition::Completed
        );
        assert_eq!(tx.state, PhysicalTransactionState::Completed);
    }

    #[test]
    fn repeated_event_id_is_ignored_without_reapplying_transition() {
        let mut tx = transaction();
        issue_release(&mut tx);
        let first = event(2, "event-release", "carrier-01", release_event());
        assert_eq!(
            tx.record_event(first.clone()).unwrap(),
            EventDisposition::Applied
        );
        assert_eq!(
            tx.record_event(first).unwrap(),
            EventDisposition::DuplicateIgnored
        );
        assert_eq!(tx.last_sequence, 2);
        assert!(tx.release_observed);
    }

    #[test]
    fn same_event_id_with_conflicting_payload_requires_reconciliation() {
        let mut tx = transaction();
        issue_release(&mut tx);
        let original = event(2, "event-release", "carrier-01", release_event());
        assert_eq!(
            tx.record_event(original.clone()).unwrap(),
            EventDisposition::Applied
        );
        let conflicting = event(
            2,
            "event-release",
            "attacker-01",
            TransactionEventKind::SensorDisagreement,
        );
        assert_eq!(
            tx.record_event(conflicting).unwrap(),
            EventDisposition::ReconciliationRequired
        );
        assert_eq!(tx.state, PhysicalTransactionState::ReconciliationRequired);
    }

    #[test]
    fn second_distinct_release_observation_requires_reconciliation() {
        let mut tx = transaction();
        issue_release(&mut tx);
        tx.record_event(event(2, "event-release", "carrier-01", release_event()))
            .unwrap();
        assert_eq!(
            tx.record_event(event(
                3,
                "event-release-duplicate",
                "carrier-01",
                release_event()
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );
    }

    #[test]
    fn new_command_id_after_dispatch_is_never_permitted() {
        let mut tx = transaction();
        issue_release(&mut tx);
        assert_eq!(
            tx.request_release_command("cmd-02").unwrap(),
            DispatchGate::BlockedForReconciliation
        );
        assert_eq!(tx.state, PhysicalTransactionState::ReconciliationRequired);
    }

    #[test]
    fn wrong_receiver_cannot_acknowledge_custody() {
        let mut tx = transaction();
        issue_release(&mut tx);
        tx.record_event(event(2, "event-release", "carrier-01", release_event()))
            .unwrap();
        tx.record_event(event(3, "event-receive", "receiver-01", receive_event()))
            .unwrap();
        let ack = TransactionEventKind::CustodyAcknowledged {
            command_id: "cmd-01".into(),
            package_id: "pkg-01".into(),
            evidence_digest: format!("sha256:{}", "a".repeat(64)),
        };
        assert_eq!(
            tx.record_event(event(4, "event-ack", "attacker-01", ack))
                .unwrap(),
            EventDisposition::ReconciliationRequired
        );
    }

    #[test]
    fn invalid_acknowledgement_digest_cannot_complete_transaction() {
        let mut tx = transaction();
        issue_release(&mut tx);
        tx.record_event(event(2, "event-release", "carrier-01", release_event()))
            .unwrap();
        tx.record_event(event(3, "event-receive", "receiver-01", receive_event()))
            .unwrap();
        let ack = TransactionEventKind::CustodyAcknowledged {
            command_id: "cmd-01".into(),
            package_id: "pkg-01".into(),
            evidence_digest: "not-a-digest".into(),
        };
        assert_eq!(
            tx.record_event(event(4, "event-ack", "receiver-01", ack))
                .unwrap(),
            EventDisposition::ReconciliationRequired
        );
    }

    #[test]
    fn sequence_gap_enters_reconciliation_required() {
        let mut tx = transaction();
        issue_release(&mut tx);
        assert_eq!(
            tx.record_event(event(3, "event-release", "carrier-01", release_event()))
                .unwrap(),
            EventDisposition::ReconciliationRequired
        );
        assert_eq!(tx.state, PhysicalTransactionState::ReconciliationRequired);
    }

    #[test]
    fn timeout_after_dispatch_blocks_any_retry() {
        let mut tx = transaction();
        issue_release(&mut tx);
        assert_eq!(
            tx.record_event(event(
                2,
                "event-timeout",
                "carrier-01",
                TransactionEventKind::ActuationTimeout
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );
        assert_eq!(
            tx.request_release_command("cmd-01").unwrap(),
            DispatchGate::BlockedByState(PhysicalTransactionState::ReconciliationRequired)
        );
    }

    #[test]
    fn expired_or_out_of_scope_grant_is_rejected_before_dispatch() {
        let mut tx = transaction();
        let invalid_grant = TransactionEventKind::AuthorizationGranted {
            not_before_unix_ms: 1_000,
            expires_at_unix_ms: 2_000,
            checked_at_unix_ms: 2_000,
            audience_matches: true,
            endpoint_matches: true,
            scope_matches: false,
            use_remaining: true,
        };
        assert_eq!(
            tx.record_event(event(1, "event-auth", "household-01", invalid_grant))
                .unwrap(),
            EventDisposition::Rejected
        );
        assert_eq!(
            tx.request_release_command("cmd-01").unwrap(),
            DispatchGate::BlockedByState(PhysicalTransactionState::Rejected)
        );
    }

    #[test]
    fn wrong_receiver_endpoint_cannot_complete_custody() {
        let mut tx = transaction();
        issue_release(&mut tx);
        tx.record_event(event(2, "event-release", "carrier-01", release_event()))
            .unwrap();
        let mut receive = receive_event();
        if let TransactionEventKind::PackageReceived { endpoint_id, .. } = &mut receive {
            *endpoint_id = "wrong-door".into();
        }
        assert_eq!(
            tx.record_event(event(3, "event-receive", "receiver-01", receive))
                .unwrap(),
            EventDisposition::ReconciliationRequired
        );
        assert_eq!(tx.state, PhysicalTransactionState::ReconciliationRequired);
    }

    fn reconciliation_event(outcome: ReconciliationOutcome) -> TransactionEventKind {
        TransactionEventKind::ReconciliationResolved {
            outcome,
            primary_evidence_digest: format!("sha256:{}", "a".repeat(64)),
            independent_evidence_digest: format!("sha256:{}", "b".repeat(64)),
        }
    }

    #[test]
    fn independent_reconciliation_closes_unknown_outcome_without_rearming_dispatch() {
        let mut tx = transaction();
        issue_release(&mut tx);
        assert_eq!(
            tx.record_event(event(
                2,
                "event-timeout",
                "carrier-01",
                TransactionEventKind::ActuationTimeout
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );

        assert_eq!(
            tx.record_event(event(
                3,
                "event-reconcile",
                "independent-custodian-01",
                reconciliation_event(ReconciliationOutcome::NoActionTaken)
            ))
            .unwrap(),
            EventDisposition::Reconciled
        );
        assert_eq!(tx.state, PhysicalTransactionState::ReconciledNoAction);
        assert_eq!(
            tx.request_release_command("cmd-01").unwrap(),
            DispatchGate::BlockedByState(PhysicalTransactionState::ReconciledNoAction)
        );
    }

    #[test]
    fn source_or_receiver_cannot_self_reconcile() {
        let mut tx = transaction();
        issue_release(&mut tx);
        tx.record_event(event(
            2,
            "event-timeout",
            "carrier-01",
            TransactionEventKind::ActuationTimeout,
        ))
        .unwrap();

        assert_eq!(
            tx.record_event(event(
                3,
                "event-reconcile",
                "carrier-01",
                reconciliation_event(ReconciliationOutcome::Returned)
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );
        assert_eq!(tx.state, PhysicalTransactionState::ReconciliationRequired);
    }

    #[test]
    fn reconciliation_requires_two_distinct_well_formed_evidence_digests() {
        let mut tx = transaction();
        issue_release(&mut tx);
        tx.record_event(event(
            2,
            "event-timeout",
            "carrier-01",
            TransactionEventKind::ActuationTimeout,
        ))
        .unwrap();
        let invalid = TransactionEventKind::ReconciliationResolved {
            outcome: ReconciliationOutcome::Quarantined,
            primary_evidence_digest: format!("sha256:{}", "a".repeat(64)),
            independent_evidence_digest: format!("sha256:{}", "a".repeat(64)),
        };

        assert_eq!(
            tx.record_event(event(
                3,
                "event-reconcile",
                "independent-custodian-01",
                invalid
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );
        assert_eq!(tx.state, PhysicalTransactionState::ReconciliationRequired);
    }

    #[test]
    fn sensor_disagreement_after_dispatch_requires_reconciliation() {
        let mut tx = transaction();
        issue_release(&mut tx);
        assert_eq!(
            tx.record_event(event(
                2,
                "event-sensor-disagreement",
                "receiver-01",
                TransactionEventKind::SensorDisagreement
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );
    }

    #[test]
    fn sensor_disagreement_before_dispatch_enters_safe_hold() {
        let mut tx = transaction();
        authorize(&mut tx);
        assert_eq!(
            tx.record_event(event(
                2,
                "event-sensor-disagreement",
                "receiver-01",
                TransactionEventKind::SensorDisagreement
            ))
            .unwrap(),
            EventDisposition::SafeHold
        );
        assert_eq!(tx.state, PhysicalTransactionState::SafeHold);
    }

    #[test]
    fn sequence_overflow_cannot_reuse_the_maximum_sequence() {
        let mut tx = transaction();
        tx.last_sequence = u64::MAX;
        assert_eq!(
            tx.record_event(event(
                u64::MAX,
                "event-overflow",
                "carrier-01",
                TransactionEventKind::NetworkPartition
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );
    }

    #[test]
    fn preflight_rejection_after_dispatch_cannot_erase_unknown_physical_outcome() {
        let mut tx = transaction();
        issue_release(&mut tx);
        assert_eq!(
            tx.record_event(event(
                2,
                "event-late-preflight-rejection",
                "receiver-01",
                TransactionEventKind::PreflightRejected {
                    reason: PreflightRejectionReason::WrongContainer,
                },
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );
        assert_eq!(tx.state, PhysicalTransactionState::ReconciliationRequired);
    }

    #[test]
    fn preflight_safe_hold_after_dispatch_requires_reconciliation() {
        let mut tx = transaction();
        issue_release(&mut tx);
        assert_eq!(
            tx.record_event(event(
                2,
                "event-late-preflight-hold",
                "receiver-01",
                TransactionEventKind::PreflightSafeHold {
                    reason: PreflightHoldReason::SealNotIntact,
                },
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );
        assert_eq!(tx.state, PhysicalTransactionState::ReconciliationRequired);
    }

    #[test]
    fn network_partition_after_release_requires_reconciliation() {
        let mut tx = transaction();
        issue_release(&mut tx);
        tx.record_event(event(2, "event-release", "carrier-01", release_event()))
            .unwrap();
        assert_eq!(
            tx.record_event(event(
                3,
                "event-partition",
                "carrier-01",
                TransactionEventKind::NetworkPartition
            ))
            .unwrap(),
            EventDisposition::ReconciliationRequired
        );
    }
}
