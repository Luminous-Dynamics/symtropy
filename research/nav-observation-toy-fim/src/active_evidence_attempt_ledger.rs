//! Consume-once lifecycle ledger for active navigation evidence acquisitions.
//!
//! This standalone kernel binds each planned action and physical attempt to
//! monotonic event sequence numbers and an unchanged pre-acquisition
//! navigation-state commitment. Failure, invalid evidence and unknown outcomes
//! cannot change that state. An unknown outcome blocks every subsequent
//! mutation until an explicit reconciliation event is recorded.
//!
//! The ledger is deliberately not an executor, cryptographic verifier,
//! posterior updater, protection-level reducer or authority controller.
//! Digests and receipts are caller-supplied commitments, not authenticated
//! facts. The event log is in-memory in this research increment; production
//! use requires a durable append-only store and crash/restart reconciliation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const SHA256_BYTES: usize = 32;

/// Frozen reliability-domain policy for one acquisition ledger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveEvidenceAttemptPolicy {
    pub expected_domain_fingerprint_sha256: [u8; SHA256_BYTES],
    pub expected_acquisition_protocol_sha256: [u8; SHA256_BYTES],
    /// Includes the initial attempt and all explicit retries.
    pub maximum_attempts_per_action: usize,
}

/// One immutable request to execute a planned sensing action once.
///
/// A retry must use a new attempt ID. The action ID, action-contract digest,
/// domain, protocol and pre-action navigation-state digest must remain
/// identical across retries of the same action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceAttemptRequest {
    pub action_id: u64,
    pub attempt_id: u64,
    pub action_contract_sha256: [u8; SHA256_BYTES],
    pub domain_fingerprint_sha256: [u8; SHA256_BYTES],
    pub acquisition_protocol_sha256: [u8; SHA256_BYTES],
    pub pre_attempt_navigation_state_sha256: [u8; SHA256_BYTES],
}

/// Terminal result after an attempt has been reconciled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolvedAcquisitionOutcome {
    /// A fresh payload and a separately qualified receipt are available.
    /// The ledger does not apply them to the posterior or protection level.
    QualifiedEvidenceAvailable {
        payload_sha256: [u8; SHA256_BYTES],
        qualification_receipt_sha256: [u8; SHA256_BYTES],
    },
    FailedOrTimedOut {
        failure_reason_sha256: [u8; SHA256_BYTES],
    },
    AcquiredButInvalid {
        payload_sha256: [u8; SHA256_BYTES],
        invalidity_receipt_sha256: [u8; SHA256_BYTES],
    },
}

/// Initial executor result. Unknown is terminal for the attempt but blocks
/// all later action attempts until reconciled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionAttemptOutcome {
    QualifiedEvidenceAvailable {
        payload_sha256: [u8; SHA256_BYTES],
        qualification_receipt_sha256: [u8; SHA256_BYTES],
    },
    FailedOrTimedOut {
        failure_reason_sha256: [u8; SHA256_BYTES],
    },
    AcquiredButInvalid {
        payload_sha256: [u8; SHA256_BYTES],
        invalidity_receipt_sha256: [u8; SHA256_BYTES],
    },
    UnknownOrUnreconciled {
        unknown_reason_sha256: [u8; SHA256_BYTES],
    },
}

impl AcquisitionAttemptOutcome {
    fn validate(self, attempt_id: u64) -> Result<(), ActiveEvidenceAttemptError> {
        match self {
            Self::QualifiedEvidenceAvailable {
                payload_sha256,
                qualification_receipt_sha256,
            } => {
                if is_zero_digest(&payload_sha256)
                    || is_zero_digest(&qualification_receipt_sha256)
                {
                    return Err(ActiveEvidenceAttemptError::MissingOutcomeCommitment(
                        attempt_id,
                    ));
                }
            }
            Self::FailedOrTimedOut {
                failure_reason_sha256,
            } => {
                if is_zero_digest(&failure_reason_sha256) {
                    return Err(ActiveEvidenceAttemptError::MissingOutcomeCommitment(
                        attempt_id,
                    ));
                }
            }
            Self::AcquiredButInvalid {
                payload_sha256,
                invalidity_receipt_sha256,
            } => {
                if is_zero_digest(&payload_sha256)
                    || is_zero_digest(&invalidity_receipt_sha256)
                {
                    return Err(ActiveEvidenceAttemptError::MissingOutcomeCommitment(
                        attempt_id,
                    ));
                }
            }
            Self::UnknownOrUnreconciled {
                unknown_reason_sha256,
            } => {
                if is_zero_digest(&unknown_reason_sha256) {
                    return Err(ActiveEvidenceAttemptError::MissingOutcomeCommitment(
                        attempt_id,
                    ));
                }
            }
        }
        Ok(())
    }
}

impl ResolvedAcquisitionOutcome {
    fn validate(self, attempt_id: u64) -> Result<(), ActiveEvidenceAttemptError> {
        let outcome = match self {
            Self::QualifiedEvidenceAvailable {
                payload_sha256,
                qualification_receipt_sha256,
            } => AcquisitionAttemptOutcome::QualifiedEvidenceAvailable {
                payload_sha256,
                qualification_receipt_sha256,
            },
            Self::FailedOrTimedOut {
                failure_reason_sha256,
            } => AcquisitionAttemptOutcome::FailedOrTimedOut {
                failure_reason_sha256,
            },
            Self::AcquiredButInvalid {
                payload_sha256,
                invalidity_receipt_sha256,
            } => AcquisitionAttemptOutcome::AcquiredButInvalid {
                payload_sha256,
                invalidity_receipt_sha256,
            },
        };
        outcome.validate(attempt_id)
    }

    fn is_qualified(self) -> bool {
        matches!(self, Self::QualifiedEvidenceAvailable { .. })
    }
}

/// Current state of one attempt record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionAttemptDisposition {
    InFlight,
    OutcomeRecorded(AcquisitionAttemptOutcome),
    Reconciled {
        original_unknown_reason_sha256: [u8; SHA256_BYTES],
        resolved_outcome: ResolvedAcquisitionOutcome,
        reconciliation_receipt_sha256: [u8; SHA256_BYTES],
    },
}

/// One attempt's pinned identity and latest disposition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AcquisitionAttemptRecord {
    pub action_id: u64,
    pub attempt_id: u64,
    pub start_event_sequence: u64,
    pub action_contract_sha256: [u8; SHA256_BYTES],
    pub domain_fingerprint_sha256: [u8; SHA256_BYTES],
    pub acquisition_protocol_sha256: [u8; SHA256_BYTES],
    pub pre_attempt_navigation_state_sha256: [u8; SHA256_BYTES],
    pub disposition: AcquisitionAttemptDisposition,
}

/// Append-only event kinds. Event sequence numbers are global to the ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AcquisitionAttemptEventKind {
    AttemptStarted {
        action_contract_sha256: [u8; SHA256_BYTES],
        domain_fingerprint_sha256: [u8; SHA256_BYTES],
        acquisition_protocol_sha256: [u8; SHA256_BYTES],
    },
    OutcomeRecorded {
        outcome: AcquisitionAttemptOutcome,
    },
    UnknownOutcomeReconciled {
        original_unknown_reason_sha256: [u8; SHA256_BYTES],
        resolved_outcome: ResolvedAcquisitionOutcome,
        reconciliation_receipt_sha256: [u8; SHA256_BYTES],
    },
}

/// One immutable event, including the navigation state that had to remain
/// unchanged while the acquisition outcome was being recorded/reconciled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AcquisitionAttemptEvent {
    pub event_sequence: u64,
    pub action_id: u64,
    pub attempt_id: u64,
    pub navigation_state_sha256: [u8; SHA256_BYTES],
    pub kind: AcquisitionAttemptEventKind,
}

/// Receipt returned when the ledger admits an attempt request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmittedAttemptReceipt {
    pub action_id: u64,
    pub attempt_id: u64,
    pub event_sequence: u64,
    pub action_contract_sha256: [u8; SHA256_BYTES],
    pub pre_attempt_navigation_state_sha256: [u8; SHA256_BYTES],
}

/// Explicit reasons for refusing an attempt or outcome transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveEvidenceAttemptError {
    InvalidPolicy,
    InvalidActionIdentity,
    InvalidAttemptIdentity,
    DuplicateAttemptIdentity(u64),
    MissingRequestCommitment(u64),
    UnsupportedDomain(u64),
    AcquisitionProtocolMismatch(u64),
    ActionContractChangedAcrossRetry(u64),
    PreAttemptStateChangedAcrossRetry(u64),
    MaximumAttemptsExceeded(u64),
    ActionAlreadyQualified(u64),
    ActiveAttemptOutstanding(u64),
    UnresolvedOutcomeBlocksNewAttempt(u64),
    AttemptNotActive(u64),
    AttemptNotUnknown(u64),
    AttemptAlreadyConsumed(u64),
    NavigationStateChangedBeforeReconciliation(u64),
    NavigationStateChangedBeforeOutcome(u64),
    MissingOutcomeCommitment(u64),
    MissingReconciliationCommitment(u64),
    EventSequenceOverflow,
    AttemptCounterOverflow(u64),
}

impl fmt::Display for ActiveEvidenceAttemptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy => write!(
                f,
                "attempt policy needs nonzero domain/protocol commitments and a positive retry budget"
            ),
            Self::InvalidActionIdentity => write!(f, "action ID must be nonzero"),
            Self::InvalidAttemptIdentity => write!(f, "attempt ID must be nonzero"),
            Self::DuplicateAttemptIdentity(id) => {
                write!(f, "attempt ID has already been consumed: {id}")
            }
            Self::MissingRequestCommitment(id) => {
                write!(f, "attempt request {id} is missing an action, domain, protocol or state commitment")
            }
            Self::UnsupportedDomain(id) => {
                write!(f, "attempt {id} does not match the admitted acquisition domain")
            }
            Self::AcquisitionProtocolMismatch(id) => {
                write!(f, "attempt {id} does not match the admitted acquisition protocol")
            }
            Self::ActionContractChangedAcrossRetry(id) => {
                write!(f, "retry changed the frozen action contract for action {id}")
            }
            Self::PreAttemptStateChangedAcrossRetry(id) => {
                write!(f, "retry does not start from the unchanged pre-acquisition state for action {id}")
            }
            Self::MaximumAttemptsExceeded(id) => {
                write!(f, "action {id} exhausted its declared attempt budget")
            }
            Self::ActionAlreadyQualified(id) => {
                write!(f, "action {id} already produced qualified evidence")
            }
            Self::ActiveAttemptOutstanding(id) => {
                write!(f, "attempt {id} has no recorded executor outcome")
            }
            Self::UnresolvedOutcomeBlocksNewAttempt(id) => {
                write!(f, "unknown attempt {id} must be reconciled before any new mutation")
            }
            Self::AttemptNotActive(id) => {
                write!(f, "attempt {id} is not the active in-flight attempt")
            }
            Self::AttemptNotUnknown(id) => {
                write!(f, "attempt {id} has no unknown outcome to reconcile")
            }
            Self::AttemptAlreadyConsumed(id) => {
                write!(f, "attempt {id} already has a terminal outcome")
            }
            Self::NavigationStateChangedBeforeReconciliation(id) => {
                write!(f, "navigation state changed before unknown attempt {id} was reconciled")
            }
            Self::NavigationStateChangedBeforeOutcome(id) => {
                write!(f, "navigation state changed before attempt {id} was recorded")
            }
            Self::MissingOutcomeCommitment(id) => {
                write!(f, "attempt {id} outcome is missing a required payload or reason commitment")
            }
            Self::MissingReconciliationCommitment(id) => {
                write!(f, "attempt {id} reconciliation is missing its evidence receipt")
            }
            Self::EventSequenceOverflow => write!(f, "attempt ledger event sequence overflowed"),
            Self::AttemptCounterOverflow(id) => {
                write!(f, "attempt count overflowed for action {id}")
            }
        }
    }
}

impl std::error::Error for ActiveEvidenceAttemptError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FrozenActionContract {
    action_contract_sha256: [u8; SHA256_BYTES],
    domain_fingerprint_sha256: [u8; SHA256_BYTES],
    acquisition_protocol_sha256: [u8; SHA256_BYTES],
    pre_attempt_navigation_state_sha256: [u8; SHA256_BYTES],
}

/// Single-writer consume-once attempt ledger.
///
/// Only one attempt may be in flight. A recorded unknown result blocks all
/// subsequent attempts until reconciliation. Failed and invalid attempts can
/// be retried with a fresh attempt ID, subject to the per-action budget.
/// Qualified acquisition is terminal for that action ID; any further sensing
/// request must receive a new planned action identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActiveEvidenceAttemptLedger {
    policy: ActiveEvidenceAttemptPolicy,
    next_event_sequence: u64,
    active_attempt_id: Option<u64>,
    unresolved_attempt_id: Option<u64>,
    records: BTreeMap<u64, AcquisitionAttemptRecord>,
    action_attempt_counts: BTreeMap<u64, usize>,
    action_contracts: BTreeMap<u64, FrozenActionContract>,
    qualified_actions: BTreeSet<u64>,
    events: Vec<AcquisitionAttemptEvent>,
}

impl ActiveEvidenceAttemptLedger {
    pub fn new(
        policy: ActiveEvidenceAttemptPolicy,
    ) -> Result<Self, ActiveEvidenceAttemptError> {
        if is_zero_digest(&policy.expected_domain_fingerprint_sha256)
            || is_zero_digest(&policy.expected_acquisition_protocol_sha256)
            || policy.maximum_attempts_per_action == 0
        {
            return Err(ActiveEvidenceAttemptError::InvalidPolicy);
        }

        Ok(Self {
            policy,
            next_event_sequence: 1,
            active_attempt_id: None,
            unresolved_attempt_id: None,
            records: BTreeMap::new(),
            action_attempt_counts: BTreeMap::new(),
            action_contracts: BTreeMap::new(),
            qualified_actions: BTreeSet::new(),
            events: Vec::new(),
        })
    }

    /// Admit and append one new physical-attempt request.
    ///
    /// The ledger serializes attempts globally so that there can be no
    /// ambiguous overlap between an unresolved action and a new mutation.
    pub fn begin_attempt(
        &mut self,
        request: EvidenceAttemptRequest,
    ) -> Result<AdmittedAttemptReceipt, ActiveEvidenceAttemptError> {
        if request.action_id == 0 {
            return Err(ActiveEvidenceAttemptError::InvalidActionIdentity);
        }
        if request.attempt_id == 0 {
            return Err(ActiveEvidenceAttemptError::InvalidAttemptIdentity);
        }
        if self.records.contains_key(&request.attempt_id) {
            return Err(ActiveEvidenceAttemptError::DuplicateAttemptIdentity(
                request.attempt_id,
            ));
        }
        if is_zero_digest(&request.action_contract_sha256)
            || is_zero_digest(&request.domain_fingerprint_sha256)
            || is_zero_digest(&request.acquisition_protocol_sha256)
            || is_zero_digest(&request.pre_attempt_navigation_state_sha256)
        {
            return Err(ActiveEvidenceAttemptError::MissingRequestCommitment(
                request.attempt_id,
            ));
        }
        if request.domain_fingerprint_sha256
            != self.policy.expected_domain_fingerprint_sha256
        {
            return Err(ActiveEvidenceAttemptError::UnsupportedDomain(
                request.attempt_id,
            ));
        }
        if request.acquisition_protocol_sha256
            != self.policy.expected_acquisition_protocol_sha256
        {
            return Err(ActiveEvidenceAttemptError::AcquisitionProtocolMismatch(
                request.attempt_id,
            ));
        }
        if let Some(unresolved) = self.unresolved_attempt_id {
            return Err(
                ActiveEvidenceAttemptError::UnresolvedOutcomeBlocksNewAttempt(unresolved),
            );
        }
        if let Some(active) = self.active_attempt_id {
            return Err(ActiveEvidenceAttemptError::ActiveAttemptOutstanding(active));
        }
        if self.qualified_actions.contains(&request.action_id) {
            return Err(ActiveEvidenceAttemptError::ActionAlreadyQualified(
                request.action_id,
            ));
        }

        let count = self
            .action_attempt_counts
            .get(&request.action_id)
            .copied()
            .unwrap_or(0);
        if count >= self.policy.maximum_attempts_per_action {
            return Err(ActiveEvidenceAttemptError::MaximumAttemptsExceeded(
                request.action_id,
            ));
        }
        let next_count = count
            .checked_add(1)
            .ok_or(ActiveEvidenceAttemptError::AttemptCounterOverflow(
                request.action_id,
            ))?;

        let frozen_contract = FrozenActionContract {
            action_contract_sha256: request.action_contract_sha256,
            domain_fingerprint_sha256: request.domain_fingerprint_sha256,
            acquisition_protocol_sha256: request.acquisition_protocol_sha256,
            pre_attempt_navigation_state_sha256: request.pre_attempt_navigation_state_sha256,
        };
        if let Some(existing) = self.action_contracts.get(&request.action_id) {
            if existing.action_contract_sha256 != frozen_contract.action_contract_sha256
                || existing.domain_fingerprint_sha256 != frozen_contract.domain_fingerprint_sha256
                || existing.acquisition_protocol_sha256 != frozen_contract.acquisition_protocol_sha256
            {
                return Err(ActiveEvidenceAttemptError::ActionContractChangedAcrossRetry(
                    request.action_id,
                ));
            }
            if existing.pre_attempt_navigation_state_sha256
                != frozen_contract.pre_attempt_navigation_state_sha256
            {
                return Err(ActiveEvidenceAttemptError::PreAttemptStateChangedAcrossRetry(
                    request.action_id,
                ));
            }
        }

        let sequence = self.allocate_event_sequence()?;
        self.action_attempt_counts
            .insert(request.action_id, next_count);
        self.action_contracts
            .entry(request.action_id)
            .or_insert(frozen_contract);
        self.records.insert(
            request.attempt_id,
            AcquisitionAttemptRecord {
                action_id: request.action_id,
                attempt_id: request.attempt_id,
                start_event_sequence: sequence,
                action_contract_sha256: request.action_contract_sha256,
                domain_fingerprint_sha256: request.domain_fingerprint_sha256,
                acquisition_protocol_sha256: request.acquisition_protocol_sha256,
                pre_attempt_navigation_state_sha256: request.pre_attempt_navigation_state_sha256,
                disposition: AcquisitionAttemptDisposition::InFlight,
            },
        );
        self.active_attempt_id = Some(request.attempt_id);
        self.events.push(AcquisitionAttemptEvent {
            event_sequence: sequence,
            action_id: request.action_id,
            attempt_id: request.attempt_id,
            navigation_state_sha256: request.pre_attempt_navigation_state_sha256,
            kind: AcquisitionAttemptEventKind::AttemptStarted {
                action_contract_sha256: request.action_contract_sha256,
                domain_fingerprint_sha256: request.domain_fingerprint_sha256,
                acquisition_protocol_sha256: request.acquisition_protocol_sha256,
            },
        });

        Ok(AdmittedAttemptReceipt {
            action_id: request.action_id,
            attempt_id: request.attempt_id,
            event_sequence: sequence,
            action_contract_sha256: request.action_contract_sha256,
            pre_attempt_navigation_state_sha256: request.pre_attempt_navigation_state_sha256,
        })
    }

    /// Record exactly one executor outcome for the active attempt.
    ///
    /// The caller must show that the navigation state is still byte-bound to
    /// the pre-attempt snapshot. Even a qualified payload is only made
    /// available for a later independent posterior audit; this method never
    /// changes that state.
    pub fn record_outcome(
        &mut self,
        attempt_id: u64,
        outcome: AcquisitionAttemptOutcome,
        observed_navigation_state_sha256: [u8; SHA256_BYTES],
    ) -> Result<(), ActiveEvidenceAttemptError> {
        if self.active_attempt_id != Some(attempt_id) {
            if self.records.contains_key(&attempt_id) {
                return Err(ActiveEvidenceAttemptError::AttemptAlreadyConsumed(
                    attempt_id,
                ));
            }
            return Err(ActiveEvidenceAttemptError::AttemptNotActive(attempt_id));
        }
        outcome.validate(attempt_id)?;

        let record = self
            .records
            .get(&attempt_id)
            .copied()
            .ok_or(ActiveEvidenceAttemptError::AttemptNotActive(attempt_id))?;
        if observed_navigation_state_sha256
            != record.pre_attempt_navigation_state_sha256
        {
            return Err(
                ActiveEvidenceAttemptError::NavigationStateChangedBeforeOutcome(attempt_id),
            );
        }

        let sequence = self.allocate_event_sequence()?;
        self.records
            .get_mut(&attempt_id)
            .expect("attempt record checked above")
            .disposition = AcquisitionAttemptDisposition::OutcomeRecorded(outcome);
        self.active_attempt_id = None;
        if matches!(outcome, AcquisitionAttemptOutcome::UnknownOrUnreconciled { .. }) {
            self.unresolved_attempt_id = Some(attempt_id);
        } else if matches!(
            outcome,
            AcquisitionAttemptOutcome::QualifiedEvidenceAvailable { .. }
        ) {
            self.qualified_actions.insert(record.action_id);
        }
        self.events.push(AcquisitionAttemptEvent {
            event_sequence: sequence,
            action_id: record.action_id,
            attempt_id,
            navigation_state_sha256: observed_navigation_state_sha256,
            kind: AcquisitionAttemptEventKind::OutcomeRecorded { outcome },
        });
        Ok(())
    }

    /// Reconcile a previously unknown attempt using an independent,
    /// consume-once reconciliation receipt. No further attempt can start
    /// until this method succeeds.
    pub fn reconcile_unknown(
        &mut self,
        attempt_id: u64,
        resolved_outcome: ResolvedAcquisitionOutcome,
        reconciliation_receipt_sha256: [u8; SHA256_BYTES],
        observed_navigation_state_sha256: [u8; SHA256_BYTES],
    ) -> Result<(), ActiveEvidenceAttemptError> {
        if self.unresolved_attempt_id != Some(attempt_id) {
            return Err(ActiveEvidenceAttemptError::AttemptNotUnknown(attempt_id));
        }
        resolved_outcome.validate(attempt_id)?;
        if is_zero_digest(&reconciliation_receipt_sha256) {
            return Err(
                ActiveEvidenceAttemptError::MissingReconciliationCommitment(attempt_id),
            );
        }

        let record = self
            .records
            .get(&attempt_id)
            .copied()
            .ok_or(ActiveEvidenceAttemptError::AttemptNotUnknown(attempt_id))?;
        if observed_navigation_state_sha256
            != record.pre_attempt_navigation_state_sha256
        {
            return Err(
                ActiveEvidenceAttemptError::NavigationStateChangedBeforeReconciliation(
                    attempt_id,
                ),
            );
        }
        let original_unknown_reason_sha256 = match record.disposition {
            AcquisitionAttemptDisposition::OutcomeRecorded(
                AcquisitionAttemptOutcome::UnknownOrUnreconciled {
                    unknown_reason_sha256,
                },
            ) => unknown_reason_sha256,
            _ => return Err(ActiveEvidenceAttemptError::AttemptNotUnknown(attempt_id)),
        };

        let sequence = self.allocate_event_sequence()?;
        self.records
            .get_mut(&attempt_id)
            .expect("attempt record checked above")
            .disposition = AcquisitionAttemptDisposition::Reconciled {
            original_unknown_reason_sha256,
            resolved_outcome,
            reconciliation_receipt_sha256,
        };
        self.unresolved_attempt_id = None;
        if resolved_outcome.is_qualified() {
            self.qualified_actions.insert(record.action_id);
        }
        self.events.push(AcquisitionAttemptEvent {
            event_sequence: sequence,
            action_id: record.action_id,
            attempt_id,
            navigation_state_sha256: observed_navigation_state_sha256,
            kind: AcquisitionAttemptEventKind::UnknownOutcomeReconciled {
                original_unknown_reason_sha256,
                resolved_outcome,
                reconciliation_receipt_sha256,
            },
        });
        Ok(())
    }

    pub fn attempt(&self, attempt_id: u64) -> Option<&AcquisitionAttemptRecord> {
        self.records.get(&attempt_id)
    }

    /// Immutable event history for an external durable append-only sink.
    pub fn events(&self) -> &[AcquisitionAttemptEvent] {
        &self.events
    }

    pub fn unresolved_attempt_id(&self) -> Option<u64> {
        self.unresolved_attempt_id
    }

    pub fn active_attempt_id(&self) -> Option<u64> {
        self.active_attempt_id
    }

    pub fn attempts_for_action(&self, action_id: u64) -> usize {
        self.action_attempt_counts
            .get(&action_id)
            .copied()
            .unwrap_or(0)
    }

    fn allocate_event_sequence(&mut self) -> Result<u64, ActiveEvidenceAttemptError> {
        let sequence = self.next_event_sequence;
        self.next_event_sequence = sequence
            .checked_add(1)
            .ok_or(ActiveEvidenceAttemptError::EventSequenceOverflow)?;
        Ok(sequence)
    }
}

fn is_zero_digest(digest: &[u8; SHA256_BYTES]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(maximum_attempts_per_action: usize) -> ActiveEvidenceAttemptPolicy {
        ActiveEvidenceAttemptPolicy {
            expected_domain_fingerprint_sha256: [0x11; SHA256_BYTES],
            expected_acquisition_protocol_sha256: [0x22; SHA256_BYTES],
            maximum_attempts_per_action,
        }
    }

    fn request(action_id: u64, attempt_id: u64) -> EvidenceAttemptRequest {
        EvidenceAttemptRequest {
            action_id,
            attempt_id,
            action_contract_sha256: [0x33; SHA256_BYTES],
            domain_fingerprint_sha256: [0x11; SHA256_BYTES],
            acquisition_protocol_sha256: [0x22; SHA256_BYTES],
            pre_attempt_navigation_state_sha256: [0x44; SHA256_BYTES],
        }
    }

    fn state() -> [u8; SHA256_BYTES] {
        [0x44; SHA256_BYTES]
    }

    fn qualified() -> AcquisitionAttemptOutcome {
        AcquisitionAttemptOutcome::QualifiedEvidenceAvailable {
            payload_sha256: [0x55; SHA256_BYTES],
            qualification_receipt_sha256: [0x66; SHA256_BYTES],
        }
    }

    fn failed() -> AcquisitionAttemptOutcome {
        AcquisitionAttemptOutcome::FailedOrTimedOut {
            failure_reason_sha256: [0x77; SHA256_BYTES],
        }
    }

    fn invalid() -> AcquisitionAttemptOutcome {
        AcquisitionAttemptOutcome::AcquiredButInvalid {
            payload_sha256: [0x88; SHA256_BYTES],
            invalidity_receipt_sha256: [0x99; SHA256_BYTES],
        }
    }

    fn unknown() -> AcquisitionAttemptOutcome {
        AcquisitionAttemptOutcome::UnknownOrUnreconciled {
            unknown_reason_sha256: [0xaa; SHA256_BYTES],
        }
    }

    #[test]
    fn records_monotonic_start_and_one_terminal_outcome() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(3)).unwrap();
        let receipt = ledger.begin_attempt(request(7, 100)).unwrap();
        assert_eq!(receipt.event_sequence, 1);
        assert_eq!(ledger.active_attempt_id(), Some(100));

        ledger.record_outcome(100, qualified(), state()).unwrap();
        assert_eq!(ledger.events().len(), 2);
        assert_eq!(ledger.events()[0].event_sequence, 1);
        assert_eq!(ledger.events()[1].event_sequence, 2);
        assert_eq!(
            ledger.attempt(100).unwrap().disposition,
            AcquisitionAttemptDisposition::OutcomeRecorded(qualified())
        );
        assert_eq!(
            ledger.begin_attempt(request(7, 101)),
            Err(ActiveEvidenceAttemptError::ActionAlreadyQualified(7))
        );
        assert_eq!(
            ledger.record_outcome(100, failed(), state()),
            Err(ActiveEvidenceAttemptError::AttemptAlreadyConsumed(100))
        );
    }

    #[test]
    fn a_failure_preserves_state_and_allows_a_new_explicit_attempt() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(2)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        ledger.record_outcome(100, failed(), state()).unwrap();

        let retry = ledger.begin_attempt(request(7, 101)).unwrap();
        assert_eq!(retry.event_sequence, 3);
        assert_eq!(ledger.attempts_for_action(7), 2);
        assert_eq!(ledger.attempt(100).unwrap().pre_attempt_navigation_state_sha256, state());
        assert_eq!(ledger.attempt(101).unwrap().pre_attempt_navigation_state_sha256, state());
    }

    #[test]
    fn an_invalid_payload_is_not_a_success_and_may_be_retried() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(2)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        ledger.record_outcome(100, invalid(), state()).unwrap();

        assert_eq!(
            ledger.attempt(100).unwrap().disposition,
            AcquisitionAttemptDisposition::OutcomeRecorded(invalid())
        );
        assert!(ledger.begin_attempt(request(7, 101)).is_ok());
    }

    #[test]
    fn unknown_outcome_blocks_all_new_mutations_until_reconciliation() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(3)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        ledger.record_outcome(100, unknown(), state()).unwrap();

        assert_eq!(ledger.unresolved_attempt_id(), Some(100));
        assert_eq!(
            ledger.begin_attempt(request(7, 101)),
            Err(ActiveEvidenceAttemptError::UnresolvedOutcomeBlocksNewAttempt(100))
        );
        assert_eq!(
            ledger.begin_attempt(request(8, 200)),
            Err(ActiveEvidenceAttemptError::UnresolvedOutcomeBlocksNewAttempt(100))
        );
    }

    #[test]
    fn reconciled_unknown_failure_releases_the_block_but_charges_the_attempt() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(2)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        ledger.record_outcome(100, unknown(), state()).unwrap();
        ledger
            .reconcile_unknown(
                100,
                ResolvedAcquisitionOutcome::FailedOrTimedOut {
                    failure_reason_sha256: [0xbb; SHA256_BYTES],
                },
                [0xcc; SHA256_BYTES],
                state(),
            )
            .unwrap();

        assert_eq!(ledger.unresolved_attempt_id(), None);
        assert_eq!(ledger.attempts_for_action(7), 1);
        assert!(ledger.begin_attempt(request(7, 101)).is_ok());
        assert_eq!(ledger.events().len(), 4);
        assert_eq!(
            ledger.events().last().unwrap().event_sequence,
            4
        );
    }

    #[test]
    fn reconciled_unknown_success_is_terminal_for_the_action() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(3)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        ledger.record_outcome(100, unknown(), state()).unwrap();
        ledger
            .reconcile_unknown(
                100,
                ResolvedAcquisitionOutcome::QualifiedEvidenceAvailable {
                    payload_sha256: [0x55; SHA256_BYTES],
                    qualification_receipt_sha256: [0x66; SHA256_BYTES],
                },
                [0xcc; SHA256_BYTES],
                state(),
            )
            .unwrap();

        assert!(matches!(
            ledger.attempt(100).unwrap().disposition,
            AcquisitionAttemptDisposition::Reconciled { .. }
        ));
        assert_eq!(
            ledger.begin_attempt(request(7, 101)),
            Err(ActiveEvidenceAttemptError::ActionAlreadyQualified(7))
        );
    }

    #[test]
    fn invalid_reconciliation_receipt_keeps_unknown_blocked() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(2)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        ledger.record_outcome(100, unknown(), state()).unwrap();

        assert_eq!(
            ledger.reconcile_unknown(
                100,
                ResolvedAcquisitionOutcome::FailedOrTimedOut {
                    failure_reason_sha256: [0; SHA256_BYTES],
                },
                [0xcc; SHA256_BYTES],
                state(),
            ),
            Err(ActiveEvidenceAttemptError::MissingOutcomeCommitment(100))
        );
        assert_eq!(ledger.unresolved_attempt_id(), Some(100));
    }

    #[test]
    fn state_change_blocks_outcome_and_unknown_reconciliation() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(2)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        assert_eq!(
            ledger.record_outcome(100, failed(), [0x45; SHA256_BYTES]),
            Err(ActiveEvidenceAttemptError::NavigationStateChangedBeforeOutcome(100))
        );
        ledger.record_outcome(100, unknown(), state()).unwrap();

        assert_eq!(
            ledger.reconcile_unknown(
                100,
                ResolvedAcquisitionOutcome::FailedOrTimedOut {
                    failure_reason_sha256: [0xbb; SHA256_BYTES],
                },
                [0xcc; SHA256_BYTES],
                [0x45; SHA256_BYTES],
            ),
            Err(ActiveEvidenceAttemptError::NavigationStateChangedBeforeReconciliation(100))
        );
        assert_eq!(ledger.unresolved_attempt_id(), Some(100));
    }

    #[test]
    fn action_contract_domain_protocol_and_pre_state_are_frozen_across_retries() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(3)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        ledger.record_outcome(100, failed(), state()).unwrap();

        let mut changed_contract = request(7, 101);
        changed_contract.action_contract_sha256 = [0x34; SHA256_BYTES];
        assert_eq!(
            ledger.begin_attempt(changed_contract),
            Err(ActiveEvidenceAttemptError::ActionContractChangedAcrossRetry(7))
        );

        let mut changed_state = request(7, 102);
        changed_state.pre_attempt_navigation_state_sha256 = [0x45; SHA256_BYTES];
        assert_eq!(
            ledger.begin_attempt(changed_state),
            Err(ActiveEvidenceAttemptError::PreAttemptStateChangedAcrossRetry(7))
        );
    }

    #[test]
    fn domain_protocol_and_missing_commitments_fail_closed() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(2)).unwrap();

        let mut other_domain = request(7, 100);
        other_domain.domain_fingerprint_sha256 = [0x12; SHA256_BYTES];
        assert_eq!(
            ledger.begin_attempt(other_domain),
            Err(ActiveEvidenceAttemptError::UnsupportedDomain(100))
        );

        let mut other_protocol = request(7, 101);
        other_protocol.acquisition_protocol_sha256 = [0x23; SHA256_BYTES];
        assert_eq!(
            ledger.begin_attempt(other_protocol),
            Err(ActiveEvidenceAttemptError::AcquisitionProtocolMismatch(101))
        );

        let mut missing = request(7, 102);
        missing.action_contract_sha256 = [0; SHA256_BYTES];
        assert_eq!(
            ledger.begin_attempt(missing),
            Err(ActiveEvidenceAttemptError::MissingRequestCommitment(102))
        );
    }

    #[test]
    fn attempt_ids_are_globally_consume_once_and_retry_budget_is_hard() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(1)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        ledger.record_outcome(100, failed(), state()).unwrap();

        assert_eq!(
            ledger.begin_attempt(request(8, 100)),
            Err(ActiveEvidenceAttemptError::DuplicateAttemptIdentity(100))
        );
        assert_eq!(
            ledger.begin_attempt(request(7, 101)),
            Err(ActiveEvidenceAttemptError::MaximumAttemptsExceeded(7))
        );
    }

    #[test]
    fn only_one_attempt_can_be_in_flight() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(2)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        assert_eq!(
            ledger.begin_attempt(request(8, 200)),
            Err(ActiveEvidenceAttemptError::ActiveAttemptOutstanding(100))
        );
    }

    #[test]
    fn malformed_policy_outcome_and_reconciliation_receipts_reject() {
        assert_eq!(
            ActiveEvidenceAttemptLedger::new(ActiveEvidenceAttemptPolicy {
                expected_domain_fingerprint_sha256: [0; SHA256_BYTES],
                expected_acquisition_protocol_sha256: [0x22; SHA256_BYTES],
                maximum_attempts_per_action: 2,
            }),
            Err(ActiveEvidenceAttemptError::InvalidPolicy)
        );

        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(2)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        assert_eq!(
            ledger.record_outcome(
                100,
                AcquisitionAttemptOutcome::QualifiedEvidenceAvailable {
                    payload_sha256: [0; SHA256_BYTES],
                    qualification_receipt_sha256: [0x66; SHA256_BYTES],
                },
                state(),
            ),
            Err(ActiveEvidenceAttemptError::MissingOutcomeCommitment(100))
        );
        ledger.record_outcome(100, unknown(), state()).unwrap();
        assert_eq!(
            ledger.reconcile_unknown(
                100,
                ResolvedAcquisitionOutcome::FailedOrTimedOut {
                    failure_reason_sha256: [0xbb; SHA256_BYTES],
                },
                [0; SHA256_BYTES],
                state(),
            ),
            Err(ActiveEvidenceAttemptError::MissingReconciliationCommitment(100))
        );
    }

    #[test]
    fn event_history_has_contiguous_monotonic_sequences() {
        let mut ledger = ActiveEvidenceAttemptLedger::new(policy(2)).unwrap();
        ledger.begin_attempt(request(7, 100)).unwrap();
        ledger.record_outcome(100, failed(), state()).unwrap();
        ledger.begin_attempt(request(7, 101)).unwrap();
        ledger.record_outcome(101, invalid(), state()).unwrap();

        let sequences: Vec<u64> = ledger
            .events()
            .iter()
            .map(|event| event.event_sequence)
            .collect();
        assert_eq!(sequences, vec![1, 2, 3, 4]);
    }
}
