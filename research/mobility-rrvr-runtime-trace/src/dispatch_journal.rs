// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Crash-safe, append-only write-ahead journal for simulator fault dispatch.
//!
//! The intent is fsynced before dispatch. A restart seeing an existing intent
//! suppresses dispatch and reconciles through the injector's durable ledger.
//! This journal never claims that a local record proves remote execution.

use crate::gazebo_adapter::{
    DispatchLedgerResult, GazeboDispatchOutcome, GazeboDispatchRequest,
    GazeboFaultInjectorTransport, ReconciliationDecision,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const DISPATCH_JOURNAL_VERSION: &str = "0.1";
const GENESIS_SHA256: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchJournalPhase {
    IntentPersisted,
    ResponseRecorded,
    ReconciliationRecorded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchJournalOutcome {
    Accepted,
    Rejected,
    Unknown,
    NoDispatchProven,
    Unresolved,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DispatchJournalRecord {
    pub version: String,
    pub sequence: u64,
    pub previous_record_sha256: String,
    pub idempotency_key: String,
    pub request_sha256: String,
    pub phase: DispatchJournalPhase,
    pub outcome: Option<DispatchJournalOutcome>,
    pub dispatch_id: Option<String>,
    pub record_sha256: String,
}

#[derive(Serialize)]
struct DispatchJournalRecordBody<'a> {
    version: &'a str,
    sequence: u64,
    previous_record_sha256: &'a str,
    idempotency_key: &'a str,
    request_sha256: &'a str,
    phase: DispatchJournalPhase,
    outcome: Option<DispatchJournalOutcome>,
    dispatch_id: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KeyJournalState {
    IntentPersisted,
    ResponseAccepted,
    ResponseRejected,
    ResponseUnknown,
    ReconciliationUnresolved,
    Reconciled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatchJournal {
    records: Vec<DispatchJournalRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DispatchJournalError {
    Io(String),
    WriterAlreadyHeld,
    TornFinalRecord,
    InvalidJson,
    InvalidRecordDigest,
    BrokenHashChain,
    SequenceDiscontinuity,
    DuplicateIntent,
    IllegalPhaseTransition,
    RequestDigestMismatch,
    MissingIntent,
    AlreadyReconciled,
    InvalidRequest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournaledDispatchOutcome {
    AttemptRecorded(GazeboDispatchOutcome),
    DuplicateSuppressed,
    IntentPersistenceFailed,
    ResponsePersistenceFailed,
    JournalRejected(DispatchJournalError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournaledReconciliationOutcome {
    Recorded(ReconciliationDecision),
    AlreadyResolved(ReconciliationDecision),
    JournalRejected(DispatchJournalError),
}

pub trait DispatchJournalStore {
    fn load_validated(&mut self) -> Result<DispatchJournal, DispatchJournalError>;
    fn append_and_sync(
        &mut self,
        record: &DispatchJournalRecord,
    ) -> Result<(), DispatchJournalError>;
}

/// A file-backed store uses an exclusive lockfile. A process crash leaves the
/// lockfile in place. It is deliberately not auto-cleared: an operator must
/// establish that no writer remains before removing a stale lease.
pub struct FileDispatchJournalStore {
    path: PathBuf,
    lock_path: PathBuf,
    _lock: File,
}

impl FileDispatchJournalStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DispatchJournalError> {
        let path = path.as_ref().to_path_buf();
        let lock_path = PathBuf::from(format!("{}.lock", path.display()));
        let lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    DispatchJournalError::WriterAlreadyHeld
                } else {
                    DispatchJournalError::Io(error.to_string())
                }
            })?;
        if let Err(error) = lock
            .sync_all()
            .map_err(|error| DispatchJournalError::Io(error.to_string()))
            .and_then(|()| sync_parent_directory(&lock_path))
        {
            drop(lock);
            let _ = fs::remove_file(&lock_path);
            return Err(error);
        }

        let journal_existed = path.exists();
        if let Err(error) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = fs::remove_file(&lock_path);
            return Err(DispatchJournalError::Io(error.to_string()));
        }
        if !journal_existed {
            if let Err(error) = sync_parent_directory(&path) {
                drop(lock);
                let _ = fs::remove_file(&lock_path);
                let _ = fs::remove_file(&path);
                return Err(error);
            }
        }

        Ok(Self {
            path,
            lock_path,
            _lock: lock,
        })
    }

    fn read_records(&self) -> Result<Vec<DispatchJournalRecord>, DispatchJournalError> {
        let mut bytes = Vec::new();
        let mut file = File::open(&self.path)
            .map_err(|error| DispatchJournalError::Io(error.to_string()))?;
        file.read_to_end(&mut bytes)
            .map_err(|error| DispatchJournalError::Io(error.to_string()))?;

        if bytes.is_empty() {
            return Ok(Vec::new());
        }
        if bytes.last() != Some(&b'\n') {
            return Err(DispatchJournalError::TornFinalRecord);
        }

        let mut records = Vec::new();
        let content = &bytes[..bytes.len() - 1];
        for line in content.split(|byte| *byte == b'\n') {
            if line.is_empty() {
                return Err(DispatchJournalError::InvalidJson);
            }
            let record = serde_json::from_slice::<DispatchJournalRecord>(line)
                .map_err(|_| DispatchJournalError::InvalidJson)?;
            records.push(record);
        }
        Ok(records)
    }
}

impl Drop for FileDispatchJournalStore {
    fn drop(&mut self) {
        // Drop is orderly shutdown: no further store operations can be issued
        // through this handle. A process crash skips Drop and leaves the lease.
        let _ = fs::remove_file(&self.lock_path);
    }
}

impl DispatchJournalStore for FileDispatchJournalStore {
    fn load_validated(&mut self) -> Result<DispatchJournal, DispatchJournalError> {
        DispatchJournal::from_records(self.read_records()?)
    }

    fn append_and_sync(
        &mut self,
        record: &DispatchJournalRecord,
    ) -> Result<(), DispatchJournalError> {
        let mut journal = self.load_validated()?;
        journal.push_verified(record.clone())?;

        let mut bytes = serde_json::to_vec(record)
            .map_err(|_| DispatchJournalError::InvalidJson)?;
        bytes.push(b'\n');
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)
            .map_err(|error| DispatchJournalError::Io(error.to_string()))?;
        file.write_all(&bytes)
            .map_err(|error| DispatchJournalError::Io(error.to_string()))?;
        file.sync_all()
            .map_err(|error| DispatchJournalError::Io(error.to_string()))?;
        Ok(())
    }
}

impl DispatchJournalRecord {
    fn body(&self) -> DispatchJournalRecordBody<'_> {
        DispatchJournalRecordBody {
            version: &self.version,
            sequence: self.sequence,
            previous_record_sha256: &self.previous_record_sha256,
            idempotency_key: &self.idempotency_key,
            request_sha256: &self.request_sha256,
            phase: self.phase,
            outcome: self.outcome,
            dispatch_id: self.dispatch_id.as_deref(),
        }
    }

    fn calculate_digest(&self) -> Result<String, DispatchJournalError> {
        let value =
            serde_json::to_value(self.body()).map_err(|_| DispatchJournalError::InvalidJson)?;
        let bytes =
            serde_json::to_vec(&value).map_err(|_| DispatchJournalError::InvalidJson)?;
        Ok(sha256_hex(&bytes))
    }

    pub fn new(
        sequence: u64,
        previous_record_sha256: String,
        idempotency_key: String,
        request_sha256: String,
        phase: DispatchJournalPhase,
        outcome: Option<DispatchJournalOutcome>,
        dispatch_id: Option<String>,
    ) -> Result<Self, DispatchJournalError> {
        let mut record = Self {
            version: DISPATCH_JOURNAL_VERSION.to_owned(),
            sequence,
            previous_record_sha256,
            idempotency_key,
            request_sha256,
            phase,
            outcome,
            dispatch_id,
            record_sha256: String::new(),
        };
        record.record_sha256 = record.calculate_digest()?;
        Ok(record)
    }

    fn verify_digest(&self) -> Result<(), DispatchJournalError> {
        if self.version != DISPATCH_JOURNAL_VERSION
            || !nonempty(&self.idempotency_key)
            || !valid_sha256(&self.request_sha256)
            || !valid_sha256(&self.previous_record_sha256)
            || !valid_sha256(&self.record_sha256)
            || self.calculate_digest()? != self.record_sha256
        {
            return Err(DispatchJournalError::InvalidRecordDigest);
        }
        Ok(())
    }
}

impl DispatchJournal {
    pub fn empty() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn from_records(
        records: Vec<DispatchJournalRecord>,
    ) -> Result<Self, DispatchJournalError> {
        let journal = Self { records };
        journal.validate()?;
        Ok(journal)
    }

    pub fn records(&self) -> &[DispatchJournalRecord] {
        &self.records
    }

    pub fn contains_intent(&self, idempotency_key: &str) -> bool {
        self.records
            .iter()
            .any(|record| record.idempotency_key == idempotency_key)
    }

    pub fn request_digest_for(
        &self,
        idempotency_key: &str,
    ) -> Option<&str> {
        self.records
            .iter()
            .find(|record| record.idempotency_key == idempotency_key)
            .map(|record| record.request_sha256.as_str())
    }

    pub fn latest_phase_for(
        &self,
        idempotency_key: &str,
    ) -> Option<DispatchJournalPhase> {
        self.records
            .iter()
            .rev()
            .find(|record| record.idempotency_key == idempotency_key)
            .map(|record| record.phase)
    }

    pub fn latest_outcome_for(
        &self,
        idempotency_key: &str,
    ) -> Option<DispatchJournalOutcome> {
        self.records
            .iter()
            .rev()
            .find(|record| {
                record.idempotency_key == idempotency_key && record.outcome.is_some()
            })
            .and_then(|record| record.outcome)
    }

    pub fn next_record(
        &self,
        idempotency_key: &str,
        request_sha256: &str,
        phase: DispatchJournalPhase,
        outcome: Option<DispatchJournalOutcome>,
        dispatch_id: Option<String>,
    ) -> Result<DispatchJournalRecord, DispatchJournalError> {
        let sequence = self.records.len() as u64 + 1;
        let previous_record_sha256 = self
            .records
            .last()
            .map(|record| record.record_sha256.clone())
            .unwrap_or_else(|| GENESIS_SHA256.to_owned());
        DispatchJournalRecord::new(
            sequence,
            previous_record_sha256,
            idempotency_key.to_owned(),
            request_sha256.to_owned(),
            phase,
            outcome,
            dispatch_id,
        )
    }

    pub fn push_verified(
        &mut self,
        record: DispatchJournalRecord,
    ) -> Result<(), DispatchJournalError> {
        let mut candidate = self.records.clone();
        candidate.push(record);
        let validated = Self::from_records(candidate)?;
        self.records = validated.records;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), DispatchJournalError> {
        let mut previous = GENESIS_SHA256.to_owned();
        let mut states = BTreeMap::<String, KeyJournalState>::new();
        let mut request_digests = BTreeMap::<String, String>::new();

        for (index, record) in self.records.iter().enumerate() {
            record.verify_digest()?;
            if record.sequence != index as u64 + 1 {
                return Err(DispatchJournalError::SequenceDiscontinuity);
            }
            if record.previous_record_sha256 != previous {
                return Err(DispatchJournalError::BrokenHashChain);
            }

            if let Some(existing_digest) = request_digests.get(&record.idempotency_key) {
                if existing_digest != &record.request_sha256 {
                    return Err(DispatchJournalError::RequestDigestMismatch);
                }
            } else {
                request_digests.insert(
                    record.idempotency_key.clone(),
                    record.request_sha256.clone(),
                );
            }

            match record.phase {
                DispatchJournalPhase::IntentPersisted => {
                    if record.outcome.is_some()
                        || record.dispatch_id.is_some()
                        || states.contains_key(&record.idempotency_key)
                    {
                        return Err(DispatchJournalError::DuplicateIntent);
                    }
                    states.insert(
                        record.idempotency_key.clone(),
                        KeyJournalState::IntentPersisted,
                    );
                }
                DispatchJournalPhase::ResponseRecorded => {
                    if states.get(&record.idempotency_key)
                        != Some(&KeyJournalState::IntentPersisted)
                    {
                        return Err(DispatchJournalError::IllegalPhaseTransition);
                    }
                    match record.outcome {
                        Some(DispatchJournalOutcome::Accepted) => {
                            if record.dispatch_id.as_deref().is_none_or(|id| !nonempty(id)) {
                                return Err(DispatchJournalError::IllegalPhaseTransition);
                            }
                            states.insert(
                                record.idempotency_key.clone(),
                                KeyJournalState::ResponseAccepted,
                            );
                        }
                        Some(DispatchJournalOutcome::Rejected) => {
                            if record.dispatch_id.as_deref().is_none_or(|id| !nonempty(id)) {
                                return Err(DispatchJournalError::IllegalPhaseTransition);
                            }
                            states.insert(
                                record.idempotency_key.clone(),
                                KeyJournalState::ResponseRejected,
                            );
                        }
                        Some(DispatchJournalOutcome::Unknown) => {
                            states.insert(
                                record.idempotency_key.clone(),
                                KeyJournalState::ResponseUnknown,
                            );
                        }
                        _ => return Err(DispatchJournalError::IllegalPhaseTransition),
                    }
                }
                DispatchJournalPhase::ReconciliationRecorded => {
                    if !matches!(
                        states.get(&record.idempotency_key),
                        Some(
                            KeyJournalState::IntentPersisted
                                | KeyJournalState::ResponseUnknown
                                | KeyJournalState::ReconciliationUnresolved
                        )
                    ) {
                        return Err(DispatchJournalError::IllegalPhaseTransition);
                    }
                    if !matches!(
                        record.outcome,
                        Some(
                            DispatchJournalOutcome::Accepted
                                | DispatchJournalOutcome::Rejected
                                | DispatchJournalOutcome::NoDispatchProven
                                | DispatchJournalOutcome::Unresolved
                        )
                    ) {
                        return Err(DispatchJournalError::IllegalPhaseTransition);
                    }
                    states.insert(
                        record.idempotency_key.clone(),
                        if record.outcome == Some(DispatchJournalOutcome::Unresolved) {
                            KeyJournalState::ReconciliationUnresolved
                        } else {
                            KeyJournalState::Reconciled
                        },
                    );
                }
            }

            previous = record.record_sha256.clone();
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournaledDispatchOutcome {
    AttemptRecorded(GazeboDispatchOutcome),
    DuplicateSuppressed,
    IntentPersistenceFailed,
    ResponsePersistenceFailed,
    JournalRejected(DispatchJournalError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JournaledReconciliationOutcome {
    Recorded(ReconciliationDecision),
    AlreadyResolved(ReconciliationDecision),
    JournalRejected(DispatchJournalError),
}

pub fn dispatch_once_with_journal<S, T>(
    store: &mut S,
    injector: &mut T,
    request: &GazeboDispatchRequest,
) -> JournaledDispatchOutcome
where
    S: DispatchJournalStore,
    T: GazeboFaultInjectorTransport,
{
    if !nonempty(&request.idempotency_key)
        || !valid_sha256(&request.plan_sha256)
        || !valid_sha256(&request.parameters_sha256)
        || !nonempty(&request.target_id)
    {
        return JournaledDispatchOutcome::JournalRejected(DispatchJournalError::InvalidRequest);
    }

    let request_sha256 = match digest_serializable(request) {
        Ok(digest) => digest,
        Err(error) => return JournaledDispatchOutcome::JournalRejected(error),
    };
    let journal = match store.load_validated() {
        Ok(journal) => journal,
        Err(error) => return JournaledDispatchOutcome::JournalRejected(error),
    };
    if journal.contains_intent(&request.idempotency_key) {
        if journal.request_digest_for(&request.idempotency_key) != Some(request_sha256.as_str()) {
            return JournaledDispatchOutcome::JournalRejected(
                DispatchJournalError::RequestDigestMismatch,
            );
        }
        return JournaledDispatchOutcome::DuplicateSuppressed;
    }

    let intent = match journal.next_record(
        &request.idempotency_key,
        &request_sha256,
        DispatchJournalPhase::IntentPersisted,
        None,
        None,
    ) {
        Ok(record) => record,
        Err(error) => return JournaledDispatchOutcome::JournalRejected(error),
    };
    if store.append_and_sync(&intent).is_err() {
        return JournaledDispatchOutcome::IntentPersistenceFailed;
    }

    let outcome = injector
        .dispatch_once(request)
        .unwrap_or(GazeboDispatchOutcome::Unknown { dispatch_id: None });
    let (journal_outcome, dispatch_id) = match &outcome {
        GazeboDispatchOutcome::Accepted { dispatch_id } => (
            DispatchJournalOutcome::Accepted,
            Some(dispatch_id.clone()),
        ),
        GazeboDispatchOutcome::Rejected { dispatch_id, .. } => (
            DispatchJournalOutcome::Rejected,
            Some(dispatch_id.clone()),
        ),
        GazeboDispatchOutcome::Unknown { dispatch_id } => {
            (DispatchJournalOutcome::Unknown, dispatch_id.clone())
        }
        GazeboDispatchOutcome::DuplicateSuppressed => {
            (DispatchJournalOutcome::Unknown, None)
        }
    };
    let response = match store.load_validated() {
        Ok(journal) => match journal.next_record(
            &request.idempotency_key,
            &request_sha256,
            DispatchJournalPhase::ResponseRecorded,
            Some(journal_outcome),
            dispatch_id,
        ) {
            Ok(record) => record,
            Err(_) => return JournaledDispatchOutcome::ResponsePersistenceFailed,
        },
        Err(_) => return JournaledDispatchOutcome::ResponsePersistenceFailed,
    };
    if store.append_and_sync(&response).is_err() {
        return JournaledDispatchOutcome::ResponsePersistenceFailed;
    }
    JournaledDispatchOutcome::AttemptRecorded(outcome)
}

pub fn reconcile_with_journal<S, T>(
    store: &mut S,
    injector: &mut T,
    idempotency_key: &str,
) -> JournaledReconciliationOutcome
where
    S: DispatchJournalStore,
    T: GazeboFaultInjectorTransport,
{
    let journal = match store.load_validated() {
        Ok(journal) => journal,
        Err(error) => return JournaledReconciliationOutcome::JournalRejected(error),
    };
    if !journal.contains_intent(idempotency_key) {
        return JournaledReconciliationOutcome::JournalRejected(
            DispatchJournalError::MissingIntent,
        );
    }

    match journal.latest_phase_for(idempotency_key) {
        Some(DispatchJournalPhase::ReconciliationRecorded) => {
            match journal.latest_outcome_for(idempotency_key) {
                Some(DispatchJournalOutcome::Unresolved) => {}
                Some(DispatchJournalOutcome::Accepted) => {
                    return JournaledReconciliationOutcome::AlreadyResolved(
                        ReconciliationDecision::ExistingDispatchAccepted,
                    );
                }
                Some(DispatchJournalOutcome::Rejected) => {
                    return JournaledReconciliationOutcome::AlreadyResolved(
                        ReconciliationDecision::ExistingDispatchRejected,
                    );
                }
                Some(DispatchJournalOutcome::NoDispatchProven) => {
                    return JournaledReconciliationOutcome::AlreadyResolved(
                        ReconciliationDecision::NoDispatchProven,
                    );
                }
                _ => {
                    return JournaledReconciliationOutcome::JournalRejected(
                        DispatchJournalError::AlreadyReconciled,
                    );
                }
            }
        }
        Some(DispatchJournalPhase::ResponseRecorded) => {
            match journal.latest_outcome_for(idempotency_key) {
                Some(DispatchJournalOutcome::Accepted) => {
                    return JournaledReconciliationOutcome::AlreadyResolved(
                        ReconciliationDecision::ExistingDispatchAccepted,
                    );
                }
                Some(DispatchJournalOutcome::Rejected) => {
                    return JournaledReconciliationOutcome::AlreadyResolved(
                        ReconciliationDecision::ExistingDispatchRejected,
                    );
                }
                Some(DispatchJournalOutcome::Unknown) => {}
                _ => {
                    return JournaledReconciliationOutcome::JournalRejected(
                        DispatchJournalError::IllegalPhaseTransition,
                    );
                }
            }
        }
        Some(DispatchJournalPhase::IntentPersisted) => {}
        None => {
            return JournaledReconciliationOutcome::JournalRejected(
                DispatchJournalError::MissingIntent,
            );
        }
    }

    let lookup = match injector.lookup_dispatch(idempotency_key) {
        Ok(result) => result,
        Err(_) => DispatchLedgerResult::Unknown,
    };
    let decision = match lookup {
        DispatchLedgerResult::Accepted => ReconciliationDecision::ExistingDispatchAccepted,
        DispatchLedgerResult::Rejected => ReconciliationDecision::ExistingDispatchRejected,
        DispatchLedgerResult::NoDispatchProven => ReconciliationDecision::NoDispatchProven,
        DispatchLedgerResult::Unknown => ReconciliationDecision::Unresolved,
    };
    let outcome = match lookup {
        DispatchLedgerResult::Accepted => DispatchJournalOutcome::Accepted,
        DispatchLedgerResult::Rejected => DispatchJournalOutcome::Rejected,
        DispatchLedgerResult::NoDispatchProven => DispatchJournalOutcome::NoDispatchProven,
        DispatchLedgerResult::Unknown => DispatchJournalOutcome::Unresolved,
    };
    let request_sha256 = match journal.request_digest_for(idempotency_key) {
        Some(value) => value,
        None => {
            return JournaledReconciliationOutcome::JournalRejected(
                DispatchJournalError::MissingIntent,
            );
        }
    };
    let record = match journal.next_record(
        idempotency_key,
        request_sha256,
        DispatchJournalPhase::ReconciliationRecorded,
        Some(outcome),
        None,
    ) {
        Ok(record) => record,
        Err(error) => return JournaledReconciliationOutcome::JournalRejected(error),
    };
    if let Err(error) = store.append_and_sync(&record) {
        return JournaledReconciliationOutcome::JournalRejected(error);
    }
    JournaledReconciliationOutcome::Recorded(decision)
}

fn sync_parent_directory(path: &Path) -> Result<(), DispatchJournalError> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let directory =
        File::open(parent).map_err(|error| DispatchJournalError::Io(error.to_string()))?;
    directory
        .sync_all()
        .map_err(|error| DispatchJournalError::Io(error.to_string()))
}

fn digest_serializable<T: Serialize>(value: &T) -> Result<String, DispatchJournalError> {
    let value = serde_json::to_value(value).map_err(|_| DispatchJournalError::InvalidJson)?;
    let bytes = serde_json::to_vec(&value).map_err(|_| DispatchJournalError::InvalidJson)?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign_plan::CompiledCampaignPlan;
    use crate::fault_campaign::FaultCampaignManifest;
    use crate::gazebo_adapter::{
        DispatchLedgerResult, GazeboAdapterSession, GazeboCapability, GazeboCapabilityProfile,
        GazeboDispatchOutcome, GazeboDispatchRequest, GazeboFaultInjectorTransport,
        GazeboTargetBinding, SteppingMethod, dispatch_fault_with_write_ahead,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_PATH: AtomicU64 = AtomicU64::new(1);

    struct MockInjector {
        dispatch_calls: usize,
        lookup_result: DispatchLedgerResult,
        dispatch_result: GazeboDispatchOutcome,
    }

    impl GazeboFaultInjectorTransport for MockInjector {
        type Error = &'static str;

        fn dispatch_once(
            &mut self,
            _request: &GazeboDispatchRequest,
        ) -> Result<GazeboDispatchOutcome, Self::Error> {
            self.dispatch_calls += 1;
            Ok(self.dispatch_result.clone())
        }

        fn lookup_dispatch(
            &mut self,
            _idempotency_key: &str,
        ) -> Result<DispatchLedgerResult, Self::Error> {
            Ok(self.lookup_result)
        }
    }

    struct FaultInjectingStore {
        inner: FileDispatchJournalStore,
        fail_on_append: usize,
        append_count: usize,
    }

    impl DispatchJournalStore for FaultInjectingStore {
        fn load_validated(&mut self) -> Result<DispatchJournal, DispatchJournalError> {
            self.inner.load_validated()
        }

        fn append_and_sync(
            &mut self,
            record: &DispatchJournalRecord,
        ) -> Result<(), DispatchJournalError> {
            self.append_count += 1;
            if self.append_count == self.fail_on_append {
                return Err(DispatchJournalError::Io("injected journal write failure".into()));
            }
            self.inner.append_and_sync(record)
        }
    }

    fn path() -> PathBuf {
        let id = NEXT_TEST_PATH.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rrvr-dispatch-journal-{}-{id}.jsonl",
            std::process::id()
        ))
    }

    fn compiled_plan() -> CompiledCampaignPlan {
        let bytes = include_bytes!("../fixtures/fault_campaign_paired_v0_1.json");
        let manifest: FaultCampaignManifest =
            serde_json::from_slice(bytes).expect("campaign fixture deserializes");
        let digest = sha256_hex(bytes);
        CompiledCampaignPlan::compile(&manifest, &digest).unwrap()
    }

    fn gazebo_session(plan: &CompiledCampaignPlan) -> GazeboAdapterSession {
        let profile = GazeboCapabilityProfile {
            version: "0.1".into(),
            simulator_id: plan.platform.platform_id.clone(),
            simulator_revision: plan.platform.simulator_revision.clone(),
            gazebo_release: plan.platform.gazebo_release.clone(),
            ros_distribution: plan.platform.ros_distribution.clone(),
            simulation_interfaces_revision: plan.platform.simulation_interfaces_revision.clone(),
            world_name: "rrvr-reference-world".into(),
            world_stats_topic: plan.platform.world_stats_topic.clone(),
            world_profile_sha256: plan.platform.environment_artifact_sha256.clone(),
            reset_profile_sha256: plan.reset.profile_sha256.clone(),
            scene_broadcaster_id: plan.platform.scene_broadcaster_id.clone(),
            scene_broadcaster_revision: plan.platform.scene_broadcaster_revision.clone(),
            scene_broadcaster_sha256: plan.platform.scene_broadcaster_sha256.clone(),
            scene_broadcaster_loaded: true,
            capabilities: [
                GazeboCapability::GetSimulatorFeatures,
                GazeboCapability::ResetSimulation,
                GazeboCapability::StepSimulation,
                GazeboCapability::GetSimulationState,
                GazeboCapability::SetSimulationState,
                GazeboCapability::GetEntities,
                GazeboCapability::GetEntityState,
            ]
            .into_iter()
            .collect(),
        };
        GazeboAdapterSession {
            profile,
            stepping_method: SteppingMethod::StepSimulationService,
            world_epoch: "world-epoch-1".into(),
            clock_id: "sim-clock".into(),
        }
    }

    fn request() -> GazeboDispatchRequest {
        GazeboDispatchRequest {
            plan_sha256: "a".repeat(64),
            campaign_id: "campaign".into(),
            pair_id: "pair".into(),
            scenario_id: "scenario".into(),
            fault_id: "fault".into(),
            fault_sequence: 1,
            target_id: "target".into(),
            fault_domain_id: "injector-domain".into(),
            scheduled_simulation_step: 100,
            parameters_sha256: "b".repeat(64),
            idempotency_key: "rrvr/plan/pair/scenario/fault/run-epoch".into(),
        }
    }

    #[test]
    fn gazebo_adapter_dispatch_is_bound_to_plan_target_and_write_ahead_intent() {
        let path = path();
        let plan = compiled_plan();
        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "negative-full-rrvr")
            .unwrap();
        let fault = &scenario.fault_events[0];
        let session = gazebo_session(&plan);
        let target = GazeboTargetBinding {
            world_name: session.profile.world_name.clone(),
            world_epoch: session.world_epoch.clone(),
            logical_component_id: fault.target_component_id.clone(),
            target_id: "gazebo-target-001".into(),
            target_revision: fault.target_revision.clone(),
            target_profile_sha256: fault.target_profile_sha256.clone(),
            fault_domain_id: fault.fault_domain_id.clone(),
        };
        let mut store = FileDispatchJournalStore::open(&path).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup_result: DispatchLedgerResult::Accepted,
            dispatch_result: GazeboDispatchOutcome::Accepted {
                dispatch_id: "dispatch-plan-bound-001".into(),
            },
        };

        assert!(matches!(
            dispatch_fault_with_write_ahead(
                &mut store,
                &mut injector,
                &plan,
                scenario,
                fault,
                &target,
                &session,
                "run-001",
                "run-epoch-001",
            ),
            Ok(JournaledDispatchOutcome::AttemptRecorded(
                GazeboDispatchOutcome::Accepted { .. }
            ))
        ));
        assert_eq!(injector.dispatch_calls, 1);
        let journal = store.load_validated().unwrap();
        assert_eq!(journal.records().len(), 2);
        assert_eq!(
            journal.records()[0].phase,
            DispatchJournalPhase::IntentPersisted
        );
        assert_eq!(
            journal.records()[1].phase,
            DispatchJournalPhase::ResponseRecorded
        );
        drop(store);

        let mut reopened = FileDispatchJournalStore::open(&path).unwrap();
        assert_eq!(
            dispatch_fault_with_write_ahead(
                &mut reopened,
                &mut injector,
                &plan,
                scenario,
                fault,
                &target,
                &session,
                "run-001",
                "run-epoch-001",
            ),
            Ok(JournaledDispatchOutcome::DuplicateSuppressed)
        );
        assert_eq!(injector.dispatch_calls, 1);
        drop(reopened);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn gazebo_adapter_rejects_wrong_world_epoch_before_writing_or_dispatching() {
        let path = path();
        let plan = compiled_plan();
        let scenario = plan
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == "negative-full-rrvr")
            .unwrap();
        let fault = &scenario.fault_events[0];
        let session = gazebo_session(&plan);
        let target = GazeboTargetBinding {
            world_name: session.profile.world_name.clone(),
            world_epoch: "stale-world-epoch".into(),
            logical_component_id: fault.target_component_id.clone(),
            target_id: "gazebo-target-001".into(),
            target_revision: fault.target_revision.clone(),
            target_profile_sha256: fault.target_profile_sha256.clone(),
            fault_domain_id: fault.fault_domain_id.clone(),
        };
        let mut store = FileDispatchJournalStore::open(&path).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup_result: DispatchLedgerResult::Unknown,
            dispatch_result: GazeboDispatchOutcome::Unknown { dispatch_id: None },
        };

        assert!(matches!(
            dispatch_fault_with_write_ahead(
                &mut store,
                &mut injector,
                &plan,
                scenario,
                fault,
                &target,
                &session,
                "run-001",
                "run-epoch-001",
            ),
            Err(crate::gazebo_adapter::GazeboAdapterError::WrongWorldEpoch)
        ));
        assert_eq!(injector.dispatch_calls, 0);
        assert!(store.load_validated().unwrap().records().is_empty());
        drop(store);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn persists_intent_before_dispatch_and_suppresses_duplicate_after_reopen() {
        let path = path();
        let mut store = FileDispatchJournalStore::open(&path).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup_result: DispatchLedgerResult::Accepted,
            dispatch_result: GazeboDispatchOutcome::Accepted {
                dispatch_id: "dispatch-1".into(),
            },
        };
        assert!(matches!(
            dispatch_once_with_journal(&mut store, &mut injector, &request()),
            JournaledDispatchOutcome::AttemptRecorded(GazeboDispatchOutcome::Accepted { .. })
        ));
        assert_eq!(injector.dispatch_calls, 1);
        drop(store);

        let mut reopened = FileDispatchJournalStore::open(&path).unwrap();
        assert_eq!(
            dispatch_once_with_journal(&mut reopened, &mut injector, &request()),
            JournaledDispatchOutcome::DuplicateSuppressed
        );
        assert_eq!(injector.dispatch_calls, 1);
        let journal = reopened.load_validated().unwrap();
        assert_eq!(journal.records().len(), 2);
        assert!(journal.validate().is_ok());
        drop(reopened);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn intent_survives_crash_before_response_and_reconciles_without_redispatch() {
        let path = path();
        let mut store = FileDispatchJournalStore::open(&path).unwrap();
        let request = request();
        let request_sha = digest_serializable(&request).unwrap();
        let intent = DispatchJournal::empty()
            .next_record(
                &request.idempotency_key,
                &request_sha,
                DispatchJournalPhase::IntentPersisted,
                None,
                None,
            )
            .unwrap();
        store.append_and_sync(&intent).unwrap();
        drop(store);

        let mut reopened = FileDispatchJournalStore::open(&path).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup_result: DispatchLedgerResult::Accepted,
            dispatch_result: GazeboDispatchOutcome::Unknown { dispatch_id: None },
        };
        assert_eq!(
            dispatch_once_with_journal(&mut reopened, &mut injector, &request),
            JournaledDispatchOutcome::DuplicateSuppressed
        );
        assert_eq!(
            reconcile_with_journal(&mut reopened, &mut injector, &request.idempotency_key),
            JournaledReconciliationOutcome::Recorded(
                ReconciliationDecision::ExistingDispatchAccepted
            )
        );
        assert_eq!(injector.dispatch_calls, 0);
        assert_eq!(reopened.load_validated().unwrap().records().len(), 2);
        drop(reopened);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn same_idempotency_key_with_changed_request_is_rejected_after_restart() {
        let path = path();
        let mut store = FileDispatchJournalStore::open(&path).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup_result: DispatchLedgerResult::Accepted,
            dispatch_result: GazeboDispatchOutcome::Accepted {
                dispatch_id: "dispatch-1".into(),
            },
        };
        let original = request();
        assert!(matches!(
            dispatch_once_with_journal(&mut store, &mut injector, &original),
            JournaledDispatchOutcome::AttemptRecorded(_)
        ));
        drop(store);

        let mut reopened = FileDispatchJournalStore::open(&path).unwrap();
        let mut changed = original;
        changed.parameters_sha256 = "c".repeat(64);
        assert_eq!(
            dispatch_once_with_journal(&mut reopened, &mut injector, &changed),
            JournaledDispatchOutcome::JournalRejected(
                DispatchJournalError::RequestDigestMismatch
            )
        );
        assert_eq!(injector.dispatch_calls, 1);
        drop(reopened);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn intent_persistence_failure_prevents_external_dispatch() {
        let path = path();
        let inner = FileDispatchJournalStore::open(&path).unwrap();
        let mut store = FaultInjectingStore {
            inner,
            fail_on_append: 1,
            append_count: 0,
        };
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup_result: DispatchLedgerResult::Unknown,
            dispatch_result: GazeboDispatchOutcome::Unknown { dispatch_id: None },
        };
        assert_eq!(
            dispatch_once_with_journal(&mut store, &mut injector, &request()),
            JournaledDispatchOutcome::IntentPersistenceFailed
        );
        assert_eq!(injector.dispatch_calls, 0);
        drop(store);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn lost_response_after_dispatch_is_reconciled_after_restart_without_redispatch() {
        let path = path();
        let inner = FileDispatchJournalStore::open(&path).unwrap();
        let mut store = FaultInjectingStore {
            inner,
            fail_on_append: 2,
            append_count: 0,
        };
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup_result: DispatchLedgerResult::Accepted,
            dispatch_result: GazeboDispatchOutcome::Accepted {
                dispatch_id: "dispatch-after-lost-response".into(),
            },
        };
        assert_eq!(
            dispatch_once_with_journal(&mut store, &mut injector, &request()),
            JournaledDispatchOutcome::ResponsePersistenceFailed
        );
        assert_eq!(injector.dispatch_calls, 1);
        drop(store);

        let mut reopened = FileDispatchJournalStore::open(&path).unwrap();
        assert_eq!(
            dispatch_once_with_journal(&mut reopened, &mut injector, &request()),
            JournaledDispatchOutcome::DuplicateSuppressed
        );
        assert_eq!(
            reconcile_with_journal(&mut reopened, &mut injector, &request().idempotency_key),
            JournaledReconciliationOutcome::Recorded(
                ReconciliationDecision::ExistingDispatchAccepted
            )
        );
        assert_eq!(injector.dispatch_calls, 1);
        drop(reopened);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn unresolved_reconciliation_can_repeat_lookups_but_never_dispatch() {
        let path = path();
        let mut store = FileDispatchJournalStore::open(&path).unwrap();
        let request = request();
        let request_sha = digest_serializable(&request).unwrap();
        let intent = DispatchJournal::empty()
            .next_record(
                &request.idempotency_key,
                &request_sha,
                DispatchJournalPhase::IntentPersisted,
                None,
                None,
            )
            .unwrap();
        store.append_and_sync(&intent).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup_result: DispatchLedgerResult::Unknown,
            dispatch_result: GazeboDispatchOutcome::Unknown { dispatch_id: None },
        };
        assert_eq!(
            reconcile_with_journal(&mut store, &mut injector, &request.idempotency_key),
            JournaledReconciliationOutcome::Recorded(ReconciliationDecision::Unresolved)
        );
        injector.lookup_result = DispatchLedgerResult::Accepted;
        assert_eq!(
            reconcile_with_journal(&mut store, &mut injector, &request.idempotency_key),
            JournaledReconciliationOutcome::Recorded(
                ReconciliationDecision::ExistingDispatchAccepted
            )
        );
        assert_eq!(injector.dispatch_calls, 0);
        assert_eq!(store.load_validated().unwrap().records().len(), 3);
        drop(store);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_a_second_writer_while_the_exclusive_lease_is_held() {
        let path = path();
        let first = FileDispatchJournalStore::open(&path).unwrap();
        assert!(matches!(
            FileDispatchJournalStore::open(&path),
            Err(DispatchJournalError::WriterAlreadyHeld)
        ));
        drop(first);
        let reopened = FileDispatchJournalStore::open(&path).unwrap();
        drop(reopened);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_a_torn_final_record_instead_of_truncating_it() {
        let path = path();
        {
            let mut store = FileDispatchJournalStore::open(&path).unwrap();
            let request = request();
            let request_sha = digest_serializable(&request).unwrap();
            let intent = DispatchJournal::empty()
                .next_record(
                    &request.idempotency_key,
                    &request_sha,
                    DispatchJournalPhase::IntentPersisted,
                    None,
                    None,
                )
                .unwrap();
            store.append_and_sync(&intent).unwrap();
        }
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"{torn")
            .unwrap();
        let store = FileDispatchJournalStore::open(&path).unwrap();
        assert_eq!(
            store.read_records(),
            Err(DispatchJournalError::TornFinalRecord)
        );
        drop(store);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn rejects_journal_record_tampering() {
        let path = path();
        {
            let mut store = FileDispatchJournalStore::open(&path).unwrap();
            let request = request();
            let request_sha = digest_serializable(&request).unwrap();
            let intent = DispatchJournal::empty()
                .next_record(
                    &request.idempotency_key,
                    &request_sha,
                    DispatchJournalPhase::IntentPersisted,
                    None,
                    None,
                )
                .unwrap();
            store.append_and_sync(&intent).unwrap();
        }
        let mut bytes = fs::read(&path).unwrap();
        let position = bytes.iter().position(|byte| *byte == b'"').unwrap();
        bytes[position + 1] ^= 1;
        fs::write(&path, bytes).unwrap();
        let store = FileDispatchJournalStore::open(&path).unwrap();
        assert!(store.load_validated().is_err());
        drop(store);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn refuses_reconciliation_without_a_persisted_intent() {
        let path = path();
        let mut store = FileDispatchJournalStore::open(&path).unwrap();
        let mut injector = MockInjector {
            dispatch_calls: 0,
            lookup_result: DispatchLedgerResult::Unknown,
            dispatch_result: GazeboDispatchOutcome::Unknown { dispatch_id: None },
        };
        assert_eq!(
            reconcile_with_journal(&mut store, &mut injector, "missing-key"),
            JournaledReconciliationOutcome::JournalRejected(
                DispatchJournalError::MissingIntent
            )
        );
        assert_eq!(injector.dispatch_calls, 0);
        drop(store);
        let _ = fs::remove_file(path);
    }
}
