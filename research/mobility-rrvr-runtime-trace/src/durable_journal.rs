// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Append-only, hash-chained RRVR reset/dispatch journal.
//!
//! The caller must append and sync an intent before invoking a mutating
//! simulator/injector transport. A journal error means the caller MUST NOT
//! perform the external mutation. After an ambiguous external result, append
//! an unknown outcome and reconcile through a read-only owner ledger/state
//! query. This module never performs transport calls or creates authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const JOURNAL_SCHEMA_VERSION: &str = "0.1";
pub const JOURNAL_CANONICALIZATION: &str = "serde-json-value-sorted-keys-v0.1";
pub const JOURNAL_GENESIS_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
pub const MAX_JOURNAL_RECORD_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournalError {
    Io { kind: String, message: String },
    WriterLeaseExists,
    InvalidPath,
    PoisonedWriter,
    TornFinalRecord,
    EmptyRecord,
    RecordTooLarge,
    InvalidUtf8,
    InvalidJson(String),
    UnsupportedVersion,
    UnsupportedCanonicalization,
    SequenceMismatch { expected: u64, actual: u64 },
    PreviousHashMismatch { sequence: u64 },
    RecordHashMismatch { sequence: u64 },
    InvalidSha256,
    MissingIdentity,
    DuplicateResetId,
    UnknownResetId,
    InvalidResetTransition,
    ResetNotVerified,
    ResetBindingMismatch,
    DuplicateDispatchKey,
    UnknownDispatchKey,
    DispatchPayloadMismatch,
    InvalidDispatchTransition,
    InvalidOutcomeEvidence,
}

impl fmt::Display for JournalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { kind, message } => write!(f, "journal I/O error ({kind}): {message}"),
            Self::WriterLeaseExists => write!(f, "journal writer lease already exists; stale leases require explicit operator recovery"),
            Self::InvalidPath => write!(f, "journal path has no usable parent or filename"),
            Self::PoisonedWriter => write!(f, "journal writer is poisoned after an uncertain append/sync failure"),
            Self::TornFinalRecord => write!(f, "journal has a torn final record; refusing automatic truncation"),
            Self::EmptyRecord => write!(f, "journal contains an empty record"),
            Self::RecordTooLarge => write!(f, "journal record exceeds the configured size bound"),
            Self::InvalidUtf8 => write!(f, "journal contains invalid UTF-8"),
            Self::InvalidJson(message) => write!(f, "journal record is invalid JSON: {message}"),
            Self::UnsupportedVersion => write!(f, "journal schema version is unsupported"),
            Self::UnsupportedCanonicalization => write!(f, "journal canonicalization profile is unsupported"),
            Self::SequenceMismatch { expected, actual } => write!(f, "journal sequence mismatch: expected {expected}, got {actual}"),
            Self::PreviousHashMismatch { sequence } => write!(f, "journal previous-record hash mismatch at sequence {sequence}"),
            Self::RecordHashMismatch { sequence } => write!(f, "journal record hash mismatch at sequence {sequence}"),
            Self::InvalidSha256 => write!(f, "journal contains an invalid SHA-256 digest"),
            Self::MissingIdentity => write!(f, "journal event is missing a required identity"),
            Self::DuplicateResetId => write!(f, "reset identity already exists in the journal"),
            Self::UnknownResetId => write!(f, "reset outcome references an unknown reset identity"),
            Self::InvalidResetTransition => write!(f, "reset outcome transition is not permitted"),
            Self::ResetNotVerified => write!(f, "dispatch intent requires a verified reset"),
            Self::ResetBindingMismatch => write!(f, "dispatch intent does not match its verified reset binding"),
            Self::DuplicateDispatchKey => write!(f, "dispatch idempotency key already exists in the journal"),
            Self::UnknownDispatchKey => write!(f, "dispatch outcome references an unknown idempotency key"),
            Self::DispatchPayloadMismatch => write!(f, "dispatch outcome payload differs from the committed intent"),
            Self::InvalidDispatchTransition => write!(f, "dispatch outcome transition is not permitted"),
            Self::InvalidOutcomeEvidence => write!(f, "outcome evidence is incomplete or inconsistent"),
        }
    }
}

impl std::error::Error for JournalError {}

#[cfg(unix)]
fn sync_parent_directory(path: &Path) -> Result<(), JournalError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    File::open(parent).and_then(|directory| directory.sync_all()).map_err(io_error)
}

#[cfg(not(unix))]
fn sync_parent_directory(_path: &Path) -> Result<(), JournalError> {
    // The target deployment is Linux/NixOS. Other platforms must qualify
    // directory-entry durability separately before claiming crash durability.
    Ok(())
}

fn io_error(error: std::io::Error) -> JournalError {
    JournalError::Io {
        kind: format!("{:?}", error.kind()),
        message: error.to_string(),
    }
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, JournalError> {
    let value = serde_json::to_value(value)
        .map_err(|error| JournalError::InvalidJson(error.to_string()))?;
    serde_json::to_vec(&value).map_err(|error| JournalError::InvalidJson(error.to_string()))
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetOutcomeSource {
    TransportResponse,
    ReadOnlyObservation,
    Reconciliation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetOutcomeStatus {
    ResponseAccepted,
    ResponseRejected,
    ResponseUnknown,
    Verified,
    Unresolved,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchOutcomeSource {
    TransportResponse,
    InjectorLedgerLookup,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchOutcomeStatus {
    ResponseAccepted,
    ResponseRejected,
    ResponseUnknown,
    ReconciledAccepted,
    ReconciledRejected,
    NoDispatchProven,
    PayloadConflict,
    Unresolved,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum JournalEvent {
    ResetIntent {
        reset_id: String,
        plan_sha256: String,
        simulator_profile_sha256: String,
        world_id: String,
        prior_world_epoch: String,
        clock_id: String,
        expected_initial_state_sha256: String,
        run_id: String,
        run_epoch: String,
    },
    ResetOutcome {
        reset_id: String,
        source: ResetOutcomeSource,
        status: ResetOutcomeStatus,
        observed_world_epoch: Option<String>,
        observed_clock_id: Option<String>,
        observed_initial_state_sha256: Option<String>,
        observation_artifact_sha256: Option<String>,
    },
    DispatchIntent {
        idempotency_key: String,
        payload_sha256: String,
        reset_id: String,
        plan_sha256: String,
        simulator_profile_sha256: String,
        world_id: String,
        logical_effect_id: String,
        attempt_id: String,
        campaign_id: String,
        pair_id: String,
        scenario_id: String,
        fault_id: String,
        fault_sequence: u32,
        run_id: String,
        run_epoch: String,
        world_epoch: String,
        clock_id: String,
        target_id: String,
        fault_domain_id: String,
        scheduled_simulation_step: u64,
        parameters_sha256: String,
    },
    DispatchOutcome {
        idempotency_key: String,
        payload_sha256: String,
        source: DispatchOutcomeSource,
        status: DispatchOutcomeStatus,
        dispatch_id: Option<String>,
        ledger_artifact_sha256: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResetIntentRecord {
    pub reset_id: String,
    pub plan_sha256: String,
    pub simulator_profile_sha256: String,
    pub world_id: String,
    pub prior_world_epoch: String,
    pub clock_id: String,
    pub expected_initial_state_sha256: String,
    pub run_id: String,
    pub run_epoch: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResetOutcomeRecord {
    pub reset_id: String,
    pub source: ResetOutcomeSource,
    pub status: ResetOutcomeStatus,
    pub observed_world_epoch: Option<String>,
    pub observed_clock_id: Option<String>,
    pub observed_initial_state_sha256: Option<String>,
    pub observation_artifact_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchIntentRecord {
    pub idempotency_key: String,
    pub payload_sha256: String,
    pub reset_id: String,
    pub plan_sha256: String,
    pub simulator_profile_sha256: String,
    pub world_id: String,
    pub logical_effect_id: String,
    pub attempt_id: String,
    pub campaign_id: String,
    pub pair_id: String,
    pub scenario_id: String,
    pub fault_id: String,
    pub fault_sequence: u32,
    pub run_id: String,
    pub run_epoch: String,
    pub world_epoch: String,
    pub clock_id: String,
    pub target_id: String,
    pub fault_domain_id: String,
    pub scheduled_simulation_step: u64,
    pub parameters_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchOutcomeRecord {
    pub idempotency_key: String,
    pub payload_sha256: String,
    pub source: DispatchOutcomeSource,
    pub status: DispatchOutcomeStatus,
    pub dispatch_id: Option<String>,
    pub ledger_artifact_sha256: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalEntry {
    pub schema_version: String,
    pub canonicalization: String,
    pub sequence: u64,
    pub previous_entry_sha256: String,
    pub event: JournalEvent,
    pub entry_sha256: String,
}

#[derive(Serialize)]
struct JournalEntryPayload<'a> {
    schema_version: &'static str,
    canonicalization: &'static str,
    sequence: u64,
    previous_entry_sha256: &'a str,
    event: &'a JournalEvent,
}

fn entry_digest(
    sequence: u64,
    previous_entry_sha256: &str,
    event: &JournalEvent,
) -> Result<String, JournalError> {
    let payload = JournalEntryPayload {
        schema_version: JOURNAL_SCHEMA_VERSION,
        canonicalization: JOURNAL_CANONICALIZATION,
        sequence,
        previous_entry_sha256,
        event,
    };
    Ok(sha256_hex(&canonical_json(&payload)?))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalSummary {
    pub schema_version: String,
    pub canonicalization: String,
    pub entry_count: u64,
    pub head_sha256: String,
    pub reset_intent_count: usize,
    pub reset_verified_count: usize,
    pub reset_unresolved_count: usize,
    pub dispatch_intent_count: usize,
    pub dispatch_accepted_count: usize,
    pub dispatch_rejected_count: usize,
    pub dispatch_unknown_count: usize,
    pub dispatch_no_dispatch_proven_count: usize,
    pub dispatch_payload_conflict_count: usize,
    pub authority_created: bool,
    pub transport_calls_performed: bool,
}

#[derive(Clone, Debug)]
struct ResetIntentBinding {
    plan_sha256: String,
    simulator_profile_sha256: String,
    world_id: String,
    prior_world_epoch: String,
    clock_id: String,
    expected_initial_state_sha256: String,
    run_id: String,
    run_epoch: String,
    verified_world_epoch: Option<String>,
}

#[derive(Clone, Debug)]
struct DispatchIntentBinding {
    payload_sha256: String,
}

#[derive(Clone, Debug, Default)]
struct JournalIndex {
    reset_intents: BTreeMap<String, ResetIntentBinding>,
    reset_status: BTreeMap<String, ResetOutcomeStatus>,
    reset_scopes: std::collections::BTreeSet<(String, String, String)>,
    dispatch_intents: BTreeMap<String, DispatchIntentBinding>,
    dispatch_status: BTreeMap<String, DispatchOutcomeStatus>,
}

impl JournalIndex {
    fn apply(&mut self, event: &JournalEvent) -> Result<(), JournalError> {
        match event {
            JournalEvent::ResetIntent {
                reset_id,
                plan_sha256,
                simulator_profile_sha256,
                world_id,
                prior_world_epoch,
                clock_id,
                expected_initial_state_sha256,
                run_id,
                run_epoch,
            } => {
                if !nonempty(reset_id)
                    || !valid_sha256(plan_sha256)
                    || !valid_sha256(simulator_profile_sha256)
                    || !nonempty(world_id)
                    || !nonempty(prior_world_epoch)
                    || !nonempty(clock_id)
                    || !valid_sha256(expected_initial_state_sha256)
                    || !nonempty(run_id)
                    || !nonempty(run_epoch)
                {
                    return Err(JournalError::MissingIdentity);
                }
                if self.reset_intents.contains_key(reset_id) {
                    return Err(JournalError::DuplicateResetId);
                }
                let reset_scope = (
                    run_id.clone(),
                    world_id.clone(),
                    prior_world_epoch.clone(),
                );
                if self.reset_scopes.contains(&reset_scope) {
                    return Err(JournalError::DuplicateResetId);
                }
                self.reset_scopes.insert(reset_scope);
                self.reset_intents.insert(
                    reset_id.clone(),
                    ResetIntentBinding {
                        plan_sha256: plan_sha256.clone(),
                        simulator_profile_sha256: simulator_profile_sha256.clone(),
                        world_id: world_id.clone(),
                        prior_world_epoch: prior_world_epoch.clone(),
                        clock_id: clock_id.clone(),
                        expected_initial_state_sha256: expected_initial_state_sha256.clone(),
                        run_id: run_id.clone(),
                        run_epoch: run_epoch.clone(),
                        verified_world_epoch: None,
                    },
                );
                Ok(())
            }
            JournalEvent::ResetOutcome {
                reset_id,
                source,
                status,
                observed_world_epoch,
                observed_clock_id,
                observed_initial_state_sha256,
                observation_artifact_sha256,
            } => {
                let intent = self
                    .reset_intents
                    .get_mut(reset_id)
                    .ok_or(JournalError::UnknownResetId)?;
                let previous = self.reset_status.get(reset_id).copied();
                match (previous, source, status) {
                    (None, ResetOutcomeSource::TransportResponse, ResetOutcomeStatus::ResponseAccepted)
                    | (None, ResetOutcomeSource::TransportResponse, ResetOutcomeStatus::ResponseRejected)
                    | (None, ResetOutcomeSource::TransportResponse, ResetOutcomeStatus::ResponseUnknown) => {}
                    (Some(ResetOutcomeStatus::ResponseAccepted | ResetOutcomeStatus::ResponseUnknown | ResetOutcomeStatus::Unresolved),
                     ResetOutcomeSource::ReadOnlyObservation | ResetOutcomeSource::Reconciliation,
                     ResetOutcomeStatus::Verified | ResetOutcomeStatus::Unresolved) => {}
                    (None, ResetOutcomeSource::ReadOnlyObservation | ResetOutcomeSource::Reconciliation,
                     ResetOutcomeStatus::Verified | ResetOutcomeStatus::Unresolved) => {
                        // Covers process restart after a durable intent but before
                        // a response record could be appended.
                    }
                    _ => return Err(JournalError::InvalidResetTransition),
                }

                if *status == ResetOutcomeStatus::Verified {
                    let world_epoch = observed_world_epoch
                        .as_deref()
                        .filter(|value| nonempty(value))
                        .ok_or(JournalError::InvalidOutcomeEvidence)?;
                    if world_epoch == intent.prior_world_epoch
                        || observed_clock_id.as_deref() != Some(intent.clock_id.as_str())
                        || observed_initial_state_sha256.as_deref()
                            != Some(intent.expected_initial_state_sha256.as_str())
                        || !observation_artifact_sha256
                            .as_deref()
                            .is_some_and(valid_sha256)
                    {
                        return Err(JournalError::InvalidOutcomeEvidence);
                    }
                    intent.verified_world_epoch = Some(world_epoch.to_owned());
                } else if matches!(
                    status,
                    ResetOutcomeStatus::Unresolved | ResetOutcomeStatus::ResponseUnknown
                ) {
                    // An unresolved observation may carry partial fields, but
                    // cannot become a verified reset from an intent alone.
                    if observation_artifact_sha256
                        .as_deref()
                        .is_some_and(|digest| !valid_sha256(digest))
                    {
                        return Err(JournalError::InvalidOutcomeEvidence);
                    }
                } else if let Some(digest) = observation_artifact_sha256 {
                    if !valid_sha256(digest) {
                        return Err(JournalError::InvalidOutcomeEvidence);
                    }
                }
                self.reset_status.insert(reset_id.clone(), *status);
                Ok(())
            }
            JournalEvent::DispatchIntent {
                idempotency_key,
                payload_sha256,
                reset_id,
                plan_sha256,
                simulator_profile_sha256,
                world_id,
                logical_effect_id,
                attempt_id,
                campaign_id,
                pair_id,
                scenario_id,
                fault_id,
                run_id,
                run_epoch,
                world_epoch,
                clock_id,
                target_id,
                fault_domain_id,
                parameters_sha256,
                ..
            } => {
                if !nonempty(idempotency_key)
                    || !valid_sha256(payload_sha256)
                    || !nonempty(logical_effect_id)
                    || !nonempty(attempt_id)
                    || !valid_sha256(plan_sha256)
                    || !valid_sha256(simulator_profile_sha256)
                    || !nonempty(world_id)
                    || !nonempty(campaign_id)
                    || !nonempty(pair_id)
                    || !nonempty(scenario_id)
                    || !nonempty(fault_id)
                    || !nonempty(run_id)
                    || !nonempty(run_epoch)
                    || !nonempty(world_epoch)
                    || !nonempty(clock_id)
                    || !nonempty(target_id)
                    || !nonempty(fault_domain_id)
                    || !valid_sha256(parameters_sha256)
                {
                    return Err(JournalError::MissingIdentity);
                }
                if self.dispatch_intents.contains_key(idempotency_key) {
                    return Err(JournalError::DuplicateDispatchKey);
                }
                let reset = self
                    .reset_intents
                    .get(reset_id)
                    .ok_or(JournalError::UnknownResetId)?;
                if self.reset_status.get(reset_id) != Some(&ResetOutcomeStatus::Verified) {
                    return Err(JournalError::ResetNotVerified);
                }
                if reset.plan_sha256 != *plan_sha256
                    || reset.simulator_profile_sha256 != *simulator_profile_sha256
                    || reset.world_id != *world_id
                    || reset.run_id != *run_id
                    || reset.run_epoch != *run_epoch
                    || reset.clock_id != *clock_id
                    || reset.verified_world_epoch.as_deref() != Some(world_epoch.as_str())
                {
                    return Err(JournalError::ResetBindingMismatch);
                }
                self.dispatch_intents.insert(
                    idempotency_key.clone(),
                    DispatchIntentBinding {
                        payload_sha256: payload_sha256.clone(),
                    },
                );
                Ok(())
            }
            JournalEvent::DispatchOutcome {
                idempotency_key,
                payload_sha256,
                source,
                status,
                dispatch_id,
                ledger_artifact_sha256,
            } => {
                let intent = self
                    .dispatch_intents
                    .get(idempotency_key)
                    .ok_or(JournalError::UnknownDispatchKey)?;
                if intent.payload_sha256 != *payload_sha256 {
                    return Err(JournalError::DispatchPayloadMismatch);
                }
                let previous = self.dispatch_status.get(idempotency_key).copied();
                if *source == DispatchOutcomeSource::InjectorLedgerLookup
                    && !ledger_artifact_sha256
                        .as_deref()
                        .is_some_and(valid_sha256)
                {
                    return Err(JournalError::InvalidOutcomeEvidence);
                }
                match (previous, source, status) {
                    (None, DispatchOutcomeSource::TransportResponse,
                     DispatchOutcomeStatus::ResponseAccepted | DispatchOutcomeStatus::ResponseRejected | DispatchOutcomeStatus::ResponseUnknown) => {}
                    (None, DispatchOutcomeSource::InjectorLedgerLookup,
                     DispatchOutcomeStatus::ReconciledAccepted | DispatchOutcomeStatus::ReconciledRejected
                     | DispatchOutcomeStatus::NoDispatchProven | DispatchOutcomeStatus::PayloadConflict
                     | DispatchOutcomeStatus::Unresolved) => {}
                    (Some(DispatchOutcomeStatus::ResponseUnknown | DispatchOutcomeStatus::Unresolved),
                     DispatchOutcomeSource::InjectorLedgerLookup,
                     DispatchOutcomeStatus::ReconciledAccepted | DispatchOutcomeStatus::ReconciledRejected
                     | DispatchOutcomeStatus::NoDispatchProven | DispatchOutcomeStatus::PayloadConflict
                     | DispatchOutcomeStatus::Unresolved) => {}
                    _ => return Err(JournalError::InvalidDispatchTransition),
                }
                match status {
                    DispatchOutcomeStatus::ResponseAccepted | DispatchOutcomeStatus::ReconciledAccepted => {
                        if !dispatch_id.as_deref().is_some_and(nonempty) {
                            return Err(JournalError::InvalidOutcomeEvidence);
                        }
                    }
                    DispatchOutcomeStatus::PayloadConflict => {
                        if ledger_artifact_sha256.as_deref().is_none_or(|digest| !valid_sha256(digest)) {
                            return Err(JournalError::InvalidOutcomeEvidence);
                        }
                    }
                    DispatchOutcomeStatus::ReconciledRejected
                    | DispatchOutcomeStatus::NoDispatchProven
                    | DispatchOutcomeStatus::Unresolved => {
                        if ledger_artifact_sha256.as_deref().is_some_and(|digest| !valid_sha256(digest)) {
                            return Err(JournalError::InvalidOutcomeEvidence);
                        }
                    }
                    DispatchOutcomeStatus::ResponseRejected
                    | DispatchOutcomeStatus::ResponseUnknown => {}
                }
                self.dispatch_status.insert(idempotency_key.clone(), *status);
                Ok(())
            }
        }
    }

    fn summary(&self, entry_count: u64, head_sha256: String) -> JournalSummary {
        let reset_verified_count = self
            .reset_status
            .values()
            .filter(|status| **status == ResetOutcomeStatus::Verified)
            .count();
        let reset_unresolved_count = self
            .reset_status
            .values()
            .filter(|status| **status == ResetOutcomeStatus::Unresolved)
            .count();
        let dispatch_count = |status: DispatchOutcomeStatus| {
            self.dispatch_status.values().filter(|value| **value == status).count()
        };
        JournalSummary {
            schema_version: JOURNAL_SCHEMA_VERSION.to_owned(),
            canonicalization: JOURNAL_CANONICALIZATION.to_owned(),
            entry_count,
            head_sha256,
            reset_intent_count: self.reset_intents.len(),
            reset_verified_count,
            reset_unresolved_count,
            dispatch_intent_count: self.dispatch_intents.len(),
            dispatch_accepted_count: dispatch_count(DispatchOutcomeStatus::ResponseAccepted)
                + dispatch_count(DispatchOutcomeStatus::ReconciledAccepted),
            dispatch_rejected_count: dispatch_count(DispatchOutcomeStatus::ResponseRejected)
                + dispatch_count(DispatchOutcomeStatus::ReconciledRejected),
            dispatch_unknown_count: dispatch_count(DispatchOutcomeStatus::ResponseUnknown)
                + dispatch_count(DispatchOutcomeStatus::Unresolved),
            dispatch_no_dispatch_proven_count: dispatch_count(DispatchOutcomeStatus::NoDispatchProven),
            dispatch_payload_conflict_count: dispatch_count(DispatchOutcomeStatus::PayloadConflict),
            authority_created: false,
            transport_calls_performed: false,
        }
    }
}

#[derive(Debug)]
struct WriterLease {
    path: PathBuf,
    _file: File,
}

impl WriterLease {
    fn acquire(journal_path: &Path) -> Result<Self, JournalError> {
        let mut lock_name: OsString = journal_path.as_os_str().to_owned();
        lock_name.push(".writer.lock");
        let path = PathBuf::from(lock_name);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    JournalError::WriterLeaseExists
                } else {
                    io_error(error)
                }
            })?;
        let write_result = writeln!(file, "pid={}", std::process::id())
            .and_then(|_| file.sync_all())
            .map_err(io_error)
            .and_then(|_| sync_parent_directory(&path));
        if let Err(error) = write_result {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(error);
        }
        Ok(Self { path, _file: file })
    }
}

impl Drop for WriterLease {
    fn drop(&mut self) {
        // Only a process that successfully created the lease removes it.
        // A process crash leaves a stale lock that requires operator recovery.
        let _ = fs::remove_file(&self.path);
    }
}

pub struct Journal {
    path: PathBuf,
    file: File,
    _lease: WriterLease,
    index: JournalIndex,
    next_sequence: u64,
    head_sha256: String,
    poisoned: bool,
}

impl Journal {
    /// Open a single-writer journal. A stale writer lock is never stolen.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, JournalError> {
        let path = path.as_ref().to_path_buf();
        let parent = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        if path.file_name().is_none() {
            return Err(JournalError::InvalidPath);
        }
        fs::create_dir_all(parent).map_err(io_error)?;
        let lease = WriterLease::acquire(&path)?;
        let existed = path.exists();
        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(io_error)?;
        if !existed {
            sync_parent_directory(&path)?;
        }
        let (index, next_sequence, head_sha256) = replay_file(&mut file)?;
        file.seek(SeekFrom::End(0)).map_err(io_error)?;
        Ok(Self {
            path,
            file,
            _lease: lease,
            index,
            next_sequence,
            head_sha256,
            poisoned: false,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn summary(&self) -> JournalSummary {
        self.index
            .summary(self.next_sequence.saturating_sub(1), self.head_sha256.clone())
    }

    /// Persist a reset intent before calling ResetSimulation. Do not call the
    /// simulator unless this method returns Ok.
    /// Persist a reset intent before calling ResetSimulation. Do not call the
    /// simulator unless this method returns Ok.
    pub fn append_reset_intent(
        &mut self,
        record: ResetIntentRecord,
    ) -> Result<JournalEntry, JournalError> {
        self.append(JournalEvent::ResetIntent {
            reset_id: record.reset_id,
            plan_sha256: record.plan_sha256,
            simulator_profile_sha256: record.simulator_profile_sha256,
            world_id: record.world_id,
            prior_world_epoch: record.prior_world_epoch,
            clock_id: record.clock_id,
            expected_initial_state_sha256: record.expected_initial_state_sha256,
            run_id: record.run_id,
            run_epoch: record.run_epoch,
        })
    }

    pub fn append_reset_outcome(
        &mut self,
        record: ResetOutcomeRecord,
    ) -> Result<JournalEntry, JournalError> {
        self.append(JournalEvent::ResetOutcome {
            reset_id: record.reset_id,
            source: record.source,
            status: record.status,
            observed_world_epoch: record.observed_world_epoch,
            observed_clock_id: record.observed_clock_id,
            observed_initial_state_sha256: record.observed_initial_state_sha256,
            observation_artifact_sha256: record.observation_artifact_sha256,
        })
    }

    /// Persist a plan/run/world/clock/target-bound dispatch intent before
    /// calling the injector. A failed return MUST suppress the transport call.
    pub fn append_dispatch_intent(
        &mut self,
        record: DispatchIntentRecord,
    ) -> Result<JournalEntry, JournalError> {
        self.append(JournalEvent::DispatchIntent {
            idempotency_key: record.idempotency_key,
            payload_sha256: record.payload_sha256,
            reset_id: record.reset_id,
            plan_sha256: record.plan_sha256,
            simulator_profile_sha256: record.simulator_profile_sha256,
            world_id: record.world_id,
            logical_effect_id: record.logical_effect_id,
            attempt_id: record.attempt_id,
            campaign_id: record.campaign_id,
            pair_id: record.pair_id,
            scenario_id: record.scenario_id,
            fault_id: record.fault_id,
            fault_sequence: record.fault_sequence,
            run_id: record.run_id,
            run_epoch: record.run_epoch,
            world_epoch: record.world_epoch,
            clock_id: record.clock_id,
            target_id: record.target_id,
            fault_domain_id: record.fault_domain_id,
            scheduled_simulation_step: record.scheduled_simulation_step,
            parameters_sha256: record.parameters_sha256,
        })
    }

    pub fn append_dispatch_outcome(
        &mut self,
        record: DispatchOutcomeRecord,
    ) -> Result<JournalEntry, JournalError> {
        self.append(JournalEvent::DispatchOutcome {
            idempotency_key: record.idempotency_key,
            payload_sha256: record.payload_sha256,
            source: record.source,
            status: record.status,
            dispatch_id: record.dispatch_id,
            ledger_artifact_sha256: record.ledger_artifact_sha256,
        })
    }

    fn append(&mut self, event: JournalEvent) -> Result<JournalEntry, JournalError> {
        if self.poisoned {
            return Err(JournalError::PoisonedWriter);
        }
        let mut candidate = self.index.clone();
        candidate.apply(&event)?;
        let digest = entry_digest(self.next_sequence, &self.head_sha256, &event)?;
        let entry = JournalEntry {
            schema_version: JOURNAL_SCHEMA_VERSION.to_owned(),
            canonicalization: JOURNAL_CANONICALIZATION.to_owned(),
            sequence: self.next_sequence,
            previous_entry_sha256: self.head_sha256.clone(),
            event,
            entry_sha256: digest.clone(),
        };
        let mut line = serde_json::to_vec(&entry)
            .map_err(|error| JournalError::InvalidJson(error.to_string()))?;
        if line.len() > MAX_JOURNAL_RECORD_BYTES {
            return Err(JournalError::RecordTooLarge);
        }
        line.push(b'\n');
        if let Err(error) = self.file.write_all(&line).and_then(|_| self.file.sync_all()) {
            self.poisoned = true;
            return Err(io_error(error));
        }
        self.index = candidate;
        self.next_sequence = self.next_sequence.saturating_add(1);
        self.head_sha256 = digest;
        Ok(entry)
    }
}

/// Verify an existing journal without taking a writer lease or mutating it.
/// Run against a quiescent journal; concurrent append may conservatively appear
/// as a torn tail and must be retried after the writer has stopped.
pub fn verify_file(path: impl AsRef<Path>) -> Result<JournalSummary, JournalError> {
    let mut file = File::open(path).map_err(io_error)?;
    let (index, next_sequence, head_sha256) = replay_file(&mut file)?;
    Ok(index.summary(next_sequence.saturating_sub(1), head_sha256))
}

fn replay_file(file: &mut File) -> Result<(JournalIndex, u64, String), JournalError> {
    file.seek(SeekFrom::Start(0)).map_err(io_error)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(io_error)?;
    if bytes.is_empty() {
        return Ok((
            JournalIndex::default(),
            1,
            JOURNAL_GENESIS_SHA256.to_owned(),
        ));
    }
    if !bytes.ends_with(b"\n") {
        return Err(JournalError::TornFinalRecord);
    }

    let mut index = JournalIndex::default();
    let mut expected_sequence = 1_u64;
    let mut previous_hash = JOURNAL_GENESIS_SHA256.to_owned();
    for raw_line in bytes.split_terminator(|byte| *byte == b'\n') {
        if raw_line.is_empty() {
            return Err(JournalError::EmptyRecord);
        }
        if raw_line.len() > MAX_JOURNAL_RECORD_BYTES {
            return Err(JournalError::RecordTooLarge);
        }
        let text = std::str::from_utf8(raw_line).map_err(|_| JournalError::InvalidUtf8)?;
        let entry: JournalEntry = serde_json::from_str(text)
            .map_err(|error| JournalError::InvalidJson(error.to_string()))?;
        if entry.schema_version != JOURNAL_SCHEMA_VERSION {
            return Err(JournalError::UnsupportedVersion);
        }
        if entry.canonicalization != JOURNAL_CANONICALIZATION {
            return Err(JournalError::UnsupportedCanonicalization);
        }
        if entry.sequence != expected_sequence {
            return Err(JournalError::SequenceMismatch {
                expected: expected_sequence,
                actual: entry.sequence,
            });
        }
        if entry.previous_entry_sha256 != previous_hash {
            return Err(JournalError::PreviousHashMismatch {
                sequence: entry.sequence,
            });
        }
        let digest = entry_digest(entry.sequence, &entry.previous_entry_sha256, &entry.event)?;
        if !valid_sha256(&entry.entry_sha256) || entry.entry_sha256 != digest {
            return Err(JournalError::RecordHashMismatch {
                sequence: entry.sequence,
            });
        }
        index.apply(&entry.event)?;
        previous_hash = digest;
        expected_sequence = expected_sequence.saturating_add(1);
    }
    Ok((index, expected_sequence, previous_hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_PATH: AtomicU64 = AtomicU64::new(0);

    fn temp_journal_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "rrvr-journal-{}-{}-{label}.jsonl",
            std::process::id(),
            NEXT_PATH.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn append_verified_reset(journal: &mut Journal) {
        journal.append_reset_intent(ResetIntentRecord {
            reset_id: "reset-1".into(),
            plan_sha256: digest('a'),
            simulator_profile_sha256: digest('b'),
            world_id: "world-1".into(),
            prior_world_epoch: "world-epoch-1".into(),
            clock_id: "clock-1".into(),
            expected_initial_state_sha256: digest('c'),
            run_id: "run-1".into(),
            run_epoch: "run-epoch-1".into(),
        })
            .unwrap();
        journal.append_reset_outcome(ResetOutcomeRecord {
            reset_id: "reset-1".into(),
            source: ResetOutcomeSource::TransportResponse,
            status: ResetOutcomeStatus::ResponseUnknown,
            observed_world_epoch: None,
            observed_clock_id: None,
            observed_initial_state_sha256: None,
            observation_artifact_sha256: None,
        })
            .unwrap();
        journal.append_reset_outcome(ResetOutcomeRecord {
            reset_id: "reset-1".into(),
            source: ResetOutcomeSource::Reconciliation,
            status: ResetOutcomeStatus::Verified,
            observed_world_epoch: Some("world-epoch-2".into()),
            observed_clock_id: Some("clock-1".into()),
            observed_initial_state_sha256: Some(digest('c')),
            observation_artifact_sha256: Some(digest('d')),
        })
            .unwrap();
    }

    fn append_dispatch_intent(journal: &mut Journal) {
        journal.append_dispatch_intent(DispatchIntentRecord {
            idempotency_key: "rrvr/v0.1/test-key-1".into(),
            payload_sha256: digest('e'),
            reset_id: "reset-1".into(),
            plan_sha256: digest('a'),
            simulator_profile_sha256: digest('b'),
            world_id: "world-1".into(),
            logical_effect_id: "logical-effect-1".into(),
            attempt_id: "attempt-1".into(),
            campaign_id: "campaign-1".into(),
            pair_id: "pair-1".into(),
            scenario_id: "scenario-1".into(),
            fault_id: "fault-1".into(),
            fault_sequence: 1,
            run_id: "run-1".into(),
            run_epoch: "run-epoch-1".into(),
            world_epoch: "world-epoch-2".into(),
            clock_id: "clock-1".into(),
            target_id: "entity-1".into(),
            fault_domain_id: "injector-domain-1".into(),
            scheduled_simulation_step: 42,
            parameters_sha256: digest('f'),
        })
            .unwrap();
    }

    fn cleanup(path: &Path) {
        let mut lock = path.as_os_str().to_owned();
        lock.push(".writer.lock");
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(PathBuf::from(lock));
    }

    #[test]
    fn journal_hash_chain_round_trips_and_gates_dispatch_on_verified_reset() {
        let path = temp_journal_path("roundtrip");
        {
            let mut journal = Journal::open(&path).unwrap();
            assert_eq!(
                journal.append_dispatch_intent(DispatchIntentRecord {
                idempotency_key: "premature-key".into(),
                payload_sha256: digest('e'),
                reset_id: "reset-1".into(),
                plan_sha256: digest('a'),
                simulator_profile_sha256: digest('b'),
                world_id: "world-1".into(),
                logical_effect_id: "logical-effect-1".into(),
                attempt_id: "attempt-1".into(),
                campaign_id: "campaign-1".into(),
                pair_id: "pair-1".into(),
                scenario_id: "scenario-1".into(),
                fault_id: "fault-1".into(),
                fault_sequence: 1,
                run_id: "run-1".into(),
                run_epoch: "run-epoch-1".into(),
                world_epoch: "world-epoch-2".into(),
                clock_id: "clock-1".into(),
                target_id: "entity-1".into(),
                fault_domain_id: "injector-domain-1".into(),
                scheduled_simulation_step: 42,
                parameters_sha256: digest('f'),
            }),
                Err(JournalError::UnknownResetId)
            );
            append_verified_reset(&mut journal);
            append_dispatch_intent(&mut journal);
            journal.append_dispatch_outcome(DispatchOutcomeRecord {
                idempotency_key: "rrvr/v0.1/test-key-1".into(),
                payload_sha256: digest('e'),
                source: DispatchOutcomeSource::TransportResponse,
                status: DispatchOutcomeStatus::ResponseUnknown,
                dispatch_id: None,
                ledger_artifact_sha256: None,
            })
                .unwrap();
            journal.append_dispatch_outcome(DispatchOutcomeRecord {
                idempotency_key: "rrvr/v0.1/test-key-1".into(),
                payload_sha256: digest('e'),
                source: DispatchOutcomeSource::InjectorLedgerLookup,
                status: DispatchOutcomeStatus::ReconciledAccepted,
                dispatch_id: Some("dispatch-1".into()),
                ledger_artifact_sha256: Some(digest('9')),
            })
                .unwrap();
            assert_eq!(journal.summary().entry_count, 6);
        }
        let summary = verify_file(&path).unwrap();
        assert_eq!(summary.entry_count, 6);
        assert_eq!(summary.reset_verified_count, 1);
        assert_eq!(summary.dispatch_accepted_count, 1);
        assert_eq!(summary.authority_created, false);
        assert_eq!(summary.transport_calls_performed, false);
        cleanup(&path);
    }

    #[test]
    fn writer_lease_is_exclusive_and_not_stolen() {
        let path = temp_journal_path("lease");
        let journal = Journal::open(&path).unwrap();
        assert!(matches!(
            Journal::open(&path),
            Err(JournalError::WriterLeaseExists)
        ));
        drop(journal);
        assert!(Journal::open(&path).is_ok());
        cleanup(&path);
    }

    #[test]
    fn torn_tail_and_hash_mutation_fail_closed() {
        let path = temp_journal_path("tamper");
        {
            let mut journal = Journal::open(&path).unwrap();
            append_verified_reset(&mut journal);
        }
        {
            let mut file = OpenOptions::new().append(true).open(&path).unwrap();
            file.write_all(b"{\"partial\":").unwrap();
            file.sync_all().unwrap();
        }
        assert_eq!(verify_file(&path), Err(JournalError::TornFinalRecord));
        cleanup(&path);

        let path = temp_journal_path("hash");
        {
            let mut journal = Journal::open(&path).unwrap();
            append_verified_reset(&mut journal);
        }
        let mut bytes = fs::read(&path).unwrap();
        let needle = b"world-1\"";
        let position = bytes
            .windows(needle.len())
            .position(|window| window == needle)
            .unwrap();
        bytes[position + 6] = b'2';
        fs::write(&path, bytes).unwrap();
        assert!(matches!(
            verify_file(&path),
            Err(JournalError::RecordHashMismatch { sequence: 1 })
                | Err(JournalError::InvalidJson(_))
        ));
        cleanup(&path);
    }

    #[test]
    fn duplicate_dispatch_key_and_payload_mismatch_are_rejected() {
        let path = temp_journal_path("duplicate");
        let mut journal = Journal::open(&path).unwrap();
        append_verified_reset(&mut journal);
        append_dispatch_intent(&mut journal);
        assert_eq!(
            journal.append_dispatch_intent(DispatchIntentRecord {
                idempotency_key: "rrvr/v0.1/test-key-1".into(),
                payload_sha256: digest('e'),
                reset_id: "reset-1".into(),
                plan_sha256: digest('a'),
                simulator_profile_sha256: digest('b'),
                world_id: "world-1".into(),
                logical_effect_id: "logical-effect-1".into(),
                attempt_id: "attempt-2".into(),
                campaign_id: "campaign-1".into(),
                pair_id: "pair-1".into(),
                scenario_id: "scenario-1".into(),
                fault_id: "fault-1".into(),
                fault_sequence: 1,
                run_id: "run-1".into(),
                run_epoch: "run-epoch-1".into(),
                world_epoch: "world-epoch-2".into(),
                clock_id: "clock-1".into(),
                target_id: "entity-1".into(),
                fault_domain_id: "injector-domain-1".into(),
                scheduled_simulation_step: 42,
                parameters_sha256: digest('f'),
            }),
            Err(JournalError::DuplicateDispatchKey)
        );
        assert_eq!(
            journal.append_dispatch_outcome(DispatchOutcomeRecord {
                idempotency_key: "rrvr/v0.1/test-key-1".into(),
                payload_sha256: digest('8'),
                source: DispatchOutcomeSource::InjectorLedgerLookup,
                status: DispatchOutcomeStatus::NoDispatchProven,
                dispatch_id: None,
                ledger_artifact_sha256: Some(digest('9')),
            }),
            Err(JournalError::DispatchPayloadMismatch)
        );
        drop(journal);
        cleanup(&path);
    }

    #[test]
    fn no_dispatch_proven_is_terminal_for_the_old_key() {
        let path = temp_journal_path("terminal");
        let mut journal = Journal::open(&path).unwrap();
        append_verified_reset(&mut journal);
        append_dispatch_intent(&mut journal);
        journal.append_dispatch_outcome(DispatchOutcomeRecord {
            idempotency_key: "rrvr/v0.1/test-key-1".into(),
            payload_sha256: digest('e'),
            source: DispatchOutcomeSource::InjectorLedgerLookup,
            status: DispatchOutcomeStatus::NoDispatchProven,
            dispatch_id: None,
            ledger_artifact_sha256: Some(digest('9')),
        }).unwrap();
        assert_eq!(
            journal.append_dispatch_outcome(DispatchOutcomeRecord {
                idempotency_key: "rrvr/v0.1/test-key-1".into(),
                payload_sha256: digest('e'),
                source: DispatchOutcomeSource::TransportResponse,
                status: DispatchOutcomeStatus::ResponseAccepted,
                dispatch_id: Some("dispatch-late".into()),
                ledger_artifact_sha256: None,
            }),
            Err(JournalError::InvalidDispatchTransition)
        );
        drop(journal);
        cleanup(&path);
    }

    #[test]
    fn reopening_rehydrates_dispatch_identity_and_reconciles_without_new_intent() {
        let path = temp_journal_path("reopen");
        {
            let mut journal = Journal::open(&path).unwrap();
            append_verified_reset(&mut journal);
            append_dispatch_intent(&mut journal);
            journal
                .append_dispatch_outcome(DispatchOutcomeRecord {
                    idempotency_key: "rrvr/v0.1/test-key-1".into(),
                    payload_sha256: digest('e'),
                    source: DispatchOutcomeSource::TransportResponse,
                    status: DispatchOutcomeStatus::ResponseUnknown,
                    dispatch_id: None,
                    ledger_artifact_sha256: None,
                })
                .unwrap();
        }

        // A new writer reconstructs the prior intent/status from disk. The
        // only allowed next operation for this key is a read-only reconciliation.
        {
            let mut reopened = Journal::open(&path).unwrap();
            assert_eq!(reopened.summary().dispatch_intent_count, 1);
            assert_eq!(reopened.summary().dispatch_unknown_count, 1);
            reopened
                .append_dispatch_outcome(DispatchOutcomeRecord {
                    idempotency_key: "rrvr/v0.1/test-key-1".into(),
                    payload_sha256: digest('e'),
                    source: DispatchOutcomeSource::InjectorLedgerLookup,
                    status: DispatchOutcomeStatus::NoDispatchProven,
                    dispatch_id: None,
                    ledger_artifact_sha256: Some(digest('9')),
                })
                .unwrap();
            assert_eq!(reopened.summary().dispatch_no_dispatch_proven_count, 1);
            assert_eq!(
                reopened.append_dispatch_outcome(DispatchOutcomeRecord {
                    idempotency_key: "rrvr/v0.1/test-key-1".into(),
                    payload_sha256: digest('e'),
                    source: DispatchOutcomeSource::TransportResponse,
                    status: DispatchOutcomeStatus::ResponseAccepted,
                    dispatch_id: Some("late-dispatch".into()),
                    ledger_artifact_sha256: None,
                }),
                Err(JournalError::InvalidDispatchTransition)
            );
        }
        assert_eq!(
            verify_file(&path).unwrap().dispatch_no_dispatch_proven_count,
            1
        );
        cleanup(&path);
    }

    #[test]
    fn reset_intent_scope_cannot_be_reused_with_a_new_reset_id() {
        let path = temp_journal_path("reset-scope");
        let mut journal = Journal::open(&path).unwrap();
        journal
            .append_reset_intent(ResetIntentRecord {
                reset_id: "reset-1".into(),
                plan_sha256: digest('a'),
                simulator_profile_sha256: digest('b'),
                world_id: "world-1".into(),
                prior_world_epoch: "world-epoch-1".into(),
                clock_id: "clock-1".into(),
                expected_initial_state_sha256: digest('c'),
                run_id: "run-1".into(),
                run_epoch: "run-epoch-1".into(),
            })
            .unwrap();
        assert_eq!(
            journal.append_reset_intent(ResetIntentRecord {
                reset_id: "reset-2".into(),
                plan_sha256: digest('a'),
                simulator_profile_sha256: digest('b'),
                world_id: "world-1".into(),
                prior_world_epoch: "world-epoch-1".into(),
                clock_id: "clock-1".into(),
                expected_initial_state_sha256: digest('c'),
                run_id: "run-1".into(),
                run_epoch: "run-epoch-1".into(),
            }),
            Err(JournalError::DuplicateResetId)
        );
        drop(journal);
        cleanup(&path);
    }

    #[test]
    fn stale_writer_lease_is_not_automatically_stolen() {
        let path = temp_journal_path("stale-lease");
        let mut lock_name = path.as_os_str().to_owned();
        lock_name.push(".writer.lock");
        let lock_path = PathBuf::from(lock_name);
        fs::write(&lock_path, b"pid=stale\n").unwrap();

        assert!(matches!(
            Journal::open(&path),
            Err(JournalError::WriterLeaseExists)
        ));
        assert!(lock_path.exists());

        // Explicit operator recovery removes the stale lease only after
        // verifying that the prior writer is no longer active.
        fs::remove_file(&lock_path).unwrap();
        assert!(Journal::open(&path).is_ok());
        cleanup(&path);
    }

    #[test]
    fn reset_verification_requires_exact_observed_world_clock_and_initial_digest() {
        let path = temp_journal_path("reset-proof");
        let mut journal = Journal::open(&path).unwrap();
        journal.append_reset_intent(ResetIntentRecord {
            reset_id: "reset-1".into(),
            plan_sha256: digest('a'),
            simulator_profile_sha256: digest('b'),
            world_id: "world-1".into(),
            prior_world_epoch: "world-epoch-1".into(),
            clock_id: "clock-1".into(),
            expected_initial_state_sha256: digest('c'),
            run_id: "run-1".into(),
            run_epoch: "run-epoch-1".into(),
        }).unwrap();
        assert_eq!(
            journal.append_reset_outcome(ResetOutcomeRecord {
                reset_id: "reset-1".into(),
                source: ResetOutcomeSource::Reconciliation,
                status: ResetOutcomeStatus::Verified,
                observed_world_epoch: Some("world-epoch-2".into()),
                observed_clock_id: Some("wrong-clock".into()),
                observed_initial_state_sha256: Some(digest('c')),
                observation_artifact_sha256: Some(digest('d')),
            }),
            Err(JournalError::InvalidOutcomeEvidence)
        );
        drop(journal);
        cleanup(&path);
    }
}
