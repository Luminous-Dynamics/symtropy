// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exactly-once canonical authority transfer for persistent subjects.
//!
//! This crate is not packet transport and does not replicate entity bytes. It
//! defines the authority state machine that a spatial/network adapter must obey:
//! destination acceptance alone grants nothing, and one atomic canonical commit
//! moves authority from source to destination exactly once. Retries return the
//! same receipt; stale competing transfers cannot both win.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Exact persistent subject classes admitted by V0.
///
/// Anonymous aggregate cohorts are deliberately absent. Aggregate migration
/// belongs to separately authorized population/resource flow systems, not an
/// exact-identity handoff protocol.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransferSubjectKind {
    PersistentActor,
    Asset,
    Cargo,
    CustomExact { namespace: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TransferSubject {
    pub kind: TransferSubjectKind,
    pub id: StableId,
}

/// Stable domain/spatial authority identity. Network peers/zones may map to this
/// externally; the canonical transaction does not depend on one netcode library.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AuthorityEndpoint {
    pub id: StableId,
}

/// Opaque exact state/snapshot record that the destination validated before
/// accepting a transfer. The producer remains responsible for its semantics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TransferStateRef {
    pub namespace: String,
    pub record_id: StableId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SubjectAuthorityRevision(pub u64);

/// Canonical current authority for one exact subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubjectAuthorityRecord {
    pub subject: TransferSubject,
    pub authority: AuthorityEndpoint,
    pub revision: SubjectAuthorityRevision,
    pub state_ref: TransferStateRef,
    pub canonical_tick: u64,
    pub source_event_id: StableId,
}

/// Source-side request to hand one exact subject to one destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferRequest {
    pub id: StableId,
    pub subject: TransferSubject,
    pub source_authority: AuthorityEndpoint,
    pub destination_authority: AuthorityEndpoint,
    pub expected_subject_revision: SubjectAuthorityRevision,
    pub source_state_ref: TransferStateRef,
    pub prepared_tick: u64,
    pub source_event_id: StableId,
}

/// Destination acknowledgement that it has validated/reserved the exact offered
/// state. Acceptance alone never changes canonical ownership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DestinationAcceptance {
    pub id: StableId,
    pub transfer_request_id: StableId,
    pub destination_authority: AuthorityEndpoint,
    pub accepted_state_ref: TransferStateRef,
    pub accepted_tick: u64,
    pub source_event_id: StableId,
}

/// Immutable exactly-once authority-transfer receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferReceipt {
    pub transfer_request_id: StableId,
    pub acceptance_id: StableId,
    pub subject: TransferSubject,
    pub source_authority: AuthorityEndpoint,
    pub destination_authority: AuthorityEndpoint,
    pub source_revision: SubjectAuthorityRevision,
    pub resulting_revision: SubjectAuthorityRevision,
    pub source_state_ref: TransferStateRef,
    pub committed_tick: u64,
    pub source_event_id: StableId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PendingTransfer {
    request: TransferRequest,
    acceptance: Option<DestinationAcceptance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CommittedTransfer {
    request: TransferRequest,
    acceptance: DestinationAcceptance,
    receipt: TransferReceipt,
}

/// Canonical authority directory and transfer linearization point.
///
/// A distributed product may replicate/federate this authority, but its external
/// protocol must preserve these semantics. This V0 in-process owner is the
/// executable specification for exactly-once handoff.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TransferAuthority {
    subjects: BTreeMap<TransferSubject, SubjectAuthorityRecord>,
    pending: BTreeMap<StableId, PendingTransfer>,
    committed: BTreeMap<StableId, CommittedTransfer>,
}

impl TransferAuthority {
    /// Bootstrap one exact subject under one authority.
    pub fn register_subject(&mut self, record: SubjectAuthorityRecord) -> Result<(), TransferError> {
        validate_state_ref(&record.state_ref)?;
        if self.subjects.contains_key(&record.subject) {
            return Err(TransferError::SubjectAlreadyRegistered(record.subject));
        }
        self.subjects.insert(record.subject.clone(), record);
        Ok(())
    }

    pub fn authority_for(&self, subject: &TransferSubject) -> Option<&SubjectAuthorityRecord> {
        self.subjects.get(subject)
    }

    /// Prepare one transfer from the currently authoritative owner.
    ///
    /// Reusing the same request id/content is idempotent. Reusing it with other
    /// content fails. Multiple different requests may be prepared from one source
    /// revision; only the first valid commit can win.
    pub fn prepare(&mut self, request: TransferRequest) -> Result<(), TransferError> {
        validate_state_ref(&request.source_state_ref)?;
        if request.source_authority == request.destination_authority {
            return Err(TransferError::SameAuthority(request.source_authority));
        }
        if let Some(committed) = self.committed.get(&request.id) {
            if committed.request == request {
                return Ok(());
            }
            return Err(TransferError::ConflictingRequestIdReuse(request.id));
        }
        if let Some(pending) = self.pending.get(&request.id) {
            if pending.request == request {
                return Ok(());
            }
            return Err(TransferError::ConflictingRequestIdReuse(request.id));
        }
        let current = self
            .subjects
            .get(&request.subject)
            .ok_or_else(|| TransferError::UnknownSubject(request.subject.clone()))?;
        if current.authority != request.source_authority {
            return Err(TransferError::SourceIsNotAuthority {
                subject: request.subject,
                expected: current.authority.clone(),
                supplied: request.source_authority,
            });
        }
        if current.revision != request.expected_subject_revision {
            return Err(TransferError::StaleSubjectRevision {
                subject: request.subject,
                expected: current.revision,
                supplied: request.expected_subject_revision,
            });
        }
        if current.state_ref != request.source_state_ref {
            return Err(TransferError::SourceStateMismatch(request.subject));
        }
        if request.prepared_tick < current.canonical_tick {
            return Err(TransferError::PreparedBeforeCurrentState {
                subject: request.subject,
                current_tick: current.canonical_tick,
                prepared_tick: request.prepared_tick,
            });
        }
        self.pending.insert(
            request.id.clone(),
            PendingTransfer {
                request,
                acceptance: None,
            },
        );
        Ok(())
    }

    /// Record destination validation/reservation. No authority changes here.
    pub fn accept(&mut self, acceptance: DestinationAcceptance) -> Result<(), TransferError> {
        validate_state_ref(&acceptance.accepted_state_ref)?;
        if let Some(committed) = self.committed.get(&acceptance.transfer_request_id) {
            if committed.acceptance == acceptance {
                return Ok(());
            }
            return Err(TransferError::TransferAlreadyCommitted(
                acceptance.transfer_request_id,
            ));
        }
        let pending = self
            .pending
            .get_mut(&acceptance.transfer_request_id)
            .ok_or_else(|| TransferError::UnknownTransfer(acceptance.transfer_request_id.clone()))?;
        if acceptance.destination_authority != pending.request.destination_authority {
            return Err(TransferError::WrongDestination {
                transfer_id: pending.request.id.clone(),
                expected: pending.request.destination_authority.clone(),
                supplied: acceptance.destination_authority,
            });
        }
        if acceptance.accepted_state_ref != pending.request.source_state_ref {
            return Err(TransferError::AcceptedDifferentState(
                pending.request.id.clone(),
            ));
        }
        if acceptance.accepted_tick < pending.request.prepared_tick {
            return Err(TransferError::AcceptanceBeforePreparation(
                acceptance.transfer_request_id,
            ));
        }
        if let Some(existing) = &pending.acceptance {
            if existing == &acceptance {
                return Ok(());
            }
            return Err(TransferError::ConflictingAcceptance(
                pending.request.id.clone(),
            ));
        }
        pending.acceptance = Some(acceptance);
        Ok(())
    }

    /// Atomically move canonical authority after destination acceptance.
    ///
    /// A retry after acknowledgement loss returns the exact original receipt.
    pub fn commit(
        &mut self,
        transfer_request_id: &StableId,
        committed_tick: u64,
        source_event_id: StableId,
    ) -> Result<TransferReceipt, TransferError> {
        if let Some(committed) = self.committed.get(transfer_request_id) {
            return Ok(committed.receipt.clone());
        }
        let pending = self
            .pending
            .get(transfer_request_id)
            .cloned()
            .ok_or_else(|| TransferError::UnknownTransfer(transfer_request_id.clone()))?;
        let acceptance = pending
            .acceptance
            .clone()
            .ok_or_else(|| TransferError::DestinationNotAccepted(transfer_request_id.clone()))?;

        let current = self
            .subjects
            .get(&pending.request.subject)
            .ok_or_else(|| TransferError::UnknownSubject(pending.request.subject.clone()))?;
        if current.authority != pending.request.source_authority
            || current.revision != pending.request.expected_subject_revision
            || current.state_ref != pending.request.source_state_ref
        {
            return Err(TransferError::PreparedTransferStale(
                pending.request.id.clone(),
            ));
        }
        if committed_tick < acceptance.accepted_tick {
            return Err(TransferError::CommitBeforeAcceptance {
                transfer_id: pending.request.id.clone(),
                accepted_tick: acceptance.accepted_tick,
                committed_tick,
            });
        }
        let resulting_revision = SubjectAuthorityRevision(
            current
                .revision
                .0
                .checked_add(1)
                .ok_or(TransferError::RevisionOverflow)?,
        );

        let receipt = TransferReceipt {
            transfer_request_id: pending.request.id.clone(),
            acceptance_id: acceptance.id.clone(),
            subject: pending.request.subject.clone(),
            source_authority: pending.request.source_authority.clone(),
            destination_authority: pending.request.destination_authority.clone(),
            source_revision: pending.request.expected_subject_revision,
            resulting_revision,
            source_state_ref: pending.request.source_state_ref.clone(),
            committed_tick,
            source_event_id: source_event_id.clone(),
        };

        // Single canonical mutation point: source ownership is replaced, never
        // coexisting with a destination-owned canonical record.
        self.subjects.insert(
            pending.request.subject.clone(),
            SubjectAuthorityRecord {
                subject: pending.request.subject.clone(),
                authority: pending.request.destination_authority.clone(),
                revision: resulting_revision,
                state_ref: pending.request.source_state_ref.clone(),
                canonical_tick: committed_tick,
                source_event_id,
            },
        );
        self.pending.remove(transfer_request_id);
        self.committed.insert(
            transfer_request_id.clone(),
            CommittedTransfer {
                request: pending.request,
                acceptance,
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }

    pub fn committed_receipt(&self, transfer_id: &StableId) -> Option<&TransferReceipt> {
        self.committed.get(transfer_id).map(|value| &value.receipt)
    }
}

fn validate_state_ref(state_ref: &TransferStateRef) -> Result<(), TransferError> {
    if state_ref.namespace.trim().is_empty() {
        Err(TransferError::EmptyStateNamespace)
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferError {
    EmptyStateNamespace,
    SubjectAlreadyRegistered(TransferSubject),
    SameAuthority(AuthorityEndpoint),
    ConflictingRequestIdReuse(StableId),
    UnknownSubject(TransferSubject),
    SourceIsNotAuthority {
        subject: TransferSubject,
        expected: AuthorityEndpoint,
        supplied: AuthorityEndpoint,
    },
    StaleSubjectRevision {
        subject: TransferSubject,
        expected: SubjectAuthorityRevision,
        supplied: SubjectAuthorityRevision,
    },
    SourceStateMismatch(TransferSubject),
    PreparedBeforeCurrentState {
        subject: TransferSubject,
        current_tick: u64,
        prepared_tick: u64,
    },
    TransferAlreadyCommitted(StableId),
    UnknownTransfer(StableId),
    WrongDestination {
        transfer_id: StableId,
        expected: AuthorityEndpoint,
        supplied: AuthorityEndpoint,
    },
    AcceptedDifferentState(StableId),
    AcceptanceBeforePreparation(StableId),
    ConflictingAcceptance(StableId),
    DestinationNotAccepted(StableId),
    PreparedTransferStale(StableId),
    CommitBeforeAcceptance {
        transfer_id: StableId,
        accepted_tick: u64,
        committed_tick: u64,
    },
    RevisionOverflow,
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStateNamespace => f.write_str("transfer state namespace is empty"),
            Self::SubjectAlreadyRegistered(subject) => write!(f, "transfer subject {} is already registered", subject.id),
            Self::SameAuthority(authority) => write!(f, "source and destination authority are both {}", authority.id),
            Self::ConflictingRequestIdReuse(id) => write!(f, "transfer request id {id} was reused with different content"),
            Self::UnknownSubject(subject) => write!(f, "unknown transfer subject {}", subject.id),
            Self::SourceIsNotAuthority { subject, .. } => write!(f, "source is not canonical authority for {}", subject.id),
            Self::StaleSubjectRevision { subject, .. } => write!(f, "transfer uses stale authority revision for {}", subject.id),
            Self::SourceStateMismatch(subject) => write!(f, "transfer source state does not match canonical state for {}", subject.id),
            Self::PreparedBeforeCurrentState { subject, .. } => write!(f, "transfer for {} was prepared before current canonical state", subject.id),
            Self::TransferAlreadyCommitted(id) => write!(f, "transfer {id} is already committed"),
            Self::UnknownTransfer(id) => write!(f, "unknown transfer {id}"),
            Self::WrongDestination { transfer_id, .. } => write!(f, "transfer {transfer_id} was accepted by the wrong destination"),
            Self::AcceptedDifferentState(id) => write!(f, "destination accepted different state for transfer {id}"),
            Self::AcceptanceBeforePreparation(id) => write!(f, "destination accepted transfer {id} before preparation"),
            Self::ConflictingAcceptance(id) => write!(f, "transfer {id} has conflicting destination acceptances"),
            Self::DestinationNotAccepted(id) => write!(f, "transfer {id} has no destination acceptance"),
            Self::PreparedTransferStale(id) => write!(f, "prepared transfer {id} is stale because canonical authority changed"),
            Self::CommitBeforeAcceptance { transfer_id, .. } => write!(f, "transfer {transfer_id} commits before destination acceptance"),
            Self::RevisionOverflow => f.write_str("subject authority revision overflow"),
        }
    }
}

impl Error for TransferError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id")
    }

    fn subject() -> TransferSubject {
        TransferSubject {
            kind: TransferSubjectKind::PersistentActor,
            id: id("resident:mara"),
        }
    }

    fn endpoint(value: &str) -> AuthorityEndpoint {
        AuthorityEndpoint { id: id(value) }
    }

    fn state(value: &str) -> TransferStateRef {
        TransferStateRef {
            namespace: "persistent-actor-snapshot".into(),
            record_id: id(value),
        }
    }

    fn authority() -> TransferAuthority {
        let mut authority = TransferAuthority::default();
        authority
            .register_subject(SubjectAuthorityRecord {
                subject: subject(),
                authority: endpoint("authority:aster"),
                revision: SubjectAuthorityRevision(7),
                state_ref: state("state:mara:7"),
                canonical_tick: 100,
                source_event_id: id("event:bootstrap"),
            })
            .expect("register");
        authority
    }

    fn request(id_value: &str, destination: &str) -> TransferRequest {
        TransferRequest {
            id: id(id_value),
            subject: subject(),
            source_authority: endpoint("authority:aster"),
            destination_authority: endpoint(destination),
            expected_subject_revision: SubjectAuthorityRevision(7),
            source_state_ref: state("state:mara:7"),
            prepared_tick: 110,
            source_event_id: id(&format!("event:{id_value}:prepare")),
        }
    }

    fn acceptance(transfer: &str, destination: &str) -> DestinationAcceptance {
        DestinationAcceptance {
            id: id(&format!("accept:{transfer}")),
            transfer_request_id: id(transfer),
            destination_authority: endpoint(destination),
            accepted_state_ref: state("state:mara:7"),
            accepted_tick: 120,
            source_event_id: id(&format!("event:{transfer}:accept")),
        }
    }

    #[test]
    fn accepted_transfer_flips_authority_exactly_once() {
        let mut authority = authority();
        authority
            .prepare(request("transfer:mara", "authority:vesper"))
            .expect("prepare");
        // Destination acceptance itself grants nothing.
        authority
            .accept(acceptance("transfer:mara", "authority:vesper"))
            .expect("accept");
        assert_eq!(
            authority.authority_for(&subject()).unwrap().authority,
            endpoint("authority:aster")
        );
        let receipt = authority
            .commit(
                &id("transfer:mara"),
                130,
                id("event:transfer:mara:commit"),
            )
            .expect("commit");
        assert_eq!(receipt.source_revision, SubjectAuthorityRevision(7));
        assert_eq!(receipt.resulting_revision, SubjectAuthorityRevision(8));
        let current = authority.authority_for(&subject()).unwrap();
        assert_eq!(current.authority, endpoint("authority:vesper"));
        assert_eq!(current.revision, SubjectAuthorityRevision(8));
    }

    #[test]
    fn commit_retry_returns_identical_receipt_without_second_revision() {
        let mut authority = authority();
        authority
            .prepare(request("transfer:mara", "authority:vesper"))
            .expect("prepare");
        authority
            .accept(acceptance("transfer:mara", "authority:vesper"))
            .expect("accept");
        let first = authority
            .commit(
                &id("transfer:mara"),
                130,
                id("event:transfer:mara:commit"),
            )
            .expect("first commit");
        let retry = authority
            .commit(
                &id("transfer:mara"),
                999,
                id("event:ignored-retry"),
            )
            .expect("retry");
        assert_eq!(first, retry);
        assert_eq!(
            authority.authority_for(&subject()).unwrap().revision,
            SubjectAuthorityRevision(8)
        );
    }

    #[test]
    fn two_prepared_destinations_cannot_both_win() {
        let mut authority = authority();
        authority
            .prepare(request("transfer:vesper", "authority:vesper"))
            .expect("prepare vesper");
        authority
            .prepare(request("transfer:helion", "authority:helion"))
            .expect("prepare helion");
        authority
            .accept(acceptance("transfer:vesper", "authority:vesper"))
            .expect("accept vesper");
        authority
            .accept(acceptance("transfer:helion", "authority:helion"))
            .expect("accept helion");
        authority
            .commit(
                &id("transfer:vesper"),
                130,
                id("event:vesper:commit"),
            )
            .expect("first wins");
        assert!(matches!(
            authority.commit(
                &id("transfer:helion"),
                131,
                id("event:helion:commit"),
            ),
            Err(TransferError::PreparedTransferStale(_))
        ));
        assert_eq!(
            authority.authority_for(&subject()).unwrap().authority,
            endpoint("authority:vesper")
        );
    }

    #[test]
    fn wrong_destination_cannot_accept_transfer() {
        let mut authority = authority();
        authority
            .prepare(request("transfer:mara", "authority:vesper"))
            .expect("prepare");
        assert!(matches!(
            authority.accept(acceptance("transfer:mara", "authority:helion")),
            Err(TransferError::WrongDestination { .. })
        ));
        assert_eq!(
            authority.authority_for(&subject()).unwrap().authority,
            endpoint("authority:aster")
        );
    }
}
