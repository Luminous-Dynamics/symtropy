// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Restart-resistant transparency witness-state boundary.
//!
//! This module defines the storage contract only. A deployment earns a
//! restart-resistant witness claim only when its backend actually provides
//! durable, linearizable compare-and-swap semantics for the complete witness
//! snapshot. A JSON file or ordinary eventually-consistent key/value store is
//! not sufficient merely because it survives process restart.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::transparency::{
    AcceptedTransparencyCheckpointV1, TransparencyCheckpointV1, TransparencyWitnessRecordV1,
    TransparencyWitnessStateSnapshotV1, TransparencyWitnessSetV1,
};

pub const TRANSPARENCY_WITNESS_STORE_SCHEMA_VERSION: u32 = 1;

/// Key identifying one external witness-memory namespace.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TransparencyWitnessStoreKeyV1 {
    policy_commitment: String,
    log_authority_commitment: String,
}

impl TransparencyWitnessStoreKeyV1 {
    pub fn new(
        policy_commitment: impl Into<String>,
        log_authority_commitment: impl Into<String>,
    ) -> Result<Self, TransparencyWitnessStoreError> {
        let value = Self {
            policy_commitment: policy_commitment.into(),
            log_authority_commitment: log_authority_commitment.into(),
        };
        value.validate_basic()?;
        Ok(value)
    }

    fn validate_basic(&self) -> Result<(), TransparencyWitnessStoreError> {
        if !is_sha256_hex(&self.policy_commitment)
            || !is_sha256_hex(&self.log_authority_commitment)
        {
            return Err(TransparencyWitnessStoreError::Invalid(
                "store key commitments must be lowercase SHA-256".to_string(),
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn policy_commitment(&self) -> &str {
        &self.policy_commitment
    }

    #[must_use]
    pub fn log_authority_commitment(&self) -> &str {
        &self.log_authority_commitment
    }
}

/// One versioned value returned by an external witness-memory store.
///
/// The generation is concurrency metadata. It is not a cryptographic freshness
/// proof and must not be treated as such outside the store that owns it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyWitnessStoredStateV1 {
    schema_version: u32,
    generation: u64,
    snapshot: TransparencyWitnessStateSnapshotV1,
}

impl TransparencyWitnessStoredStateV1 {
    pub fn new(
        generation: u64,
        snapshot: TransparencyWitnessStateSnapshotV1,
    ) -> Result<Self, TransparencyWitnessStoreError> {
        let value = Self {
            schema_version: TRANSPARENCY_WITNESS_STORE_SCHEMA_VERSION,
            generation,
            snapshot,
        };
        value.validate_basic()?;
        Ok(value)
    }

    pub fn validate_basic(&self) -> Result<(), TransparencyWitnessStoreError> {
        if self.schema_version != TRANSPARENCY_WITNESS_STORE_SCHEMA_VERSION {
            return Err(TransparencyWitnessStoreError::Invalid(
                "unsupported external transparency witness store schema".to_string(),
            ));
        }
        self.snapshot.validate_basic().map_err(|error| {
            TransparencyWitnessStoreError::Invalid(error.to_string())
        })?;
        Ok(())
    }

    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    #[must_use]
    pub fn snapshot(&self) -> &TransparencyWitnessStateSnapshotV1 {
        &self.snapshot
    }
}

/// Backend contract for restart-resistant retained witness memory.
///
/// Implementations MUST make `compare_and_swap` linearizable with respect to
/// `load` and MUST durably commit the replacement before reporting success.
/// The entire snapshot must be replaced atomically: partial witness-set writes
/// can create a state that never existed as a valid quorum observation.
///
/// A successful CAS increments the generation exactly once. A rejected CAS
/// MUST NOT modify the stored value.
pub trait TransparencyWitnessStateStore {
    fn load(
        &self,
        key: &TransparencyWitnessStoreKeyV1,
    ) -> Result<Option<TransparencyWitnessStoredStateV1>, TransparencyWitnessStoreError>;

    fn compare_and_swap(
        &self,
        key: &TransparencyWitnessStoreKeyV1,
        expected: Option<&TransparencyWitnessStoredStateV1>,
        replacement: TransparencyWitnessStateSnapshotV1,
    ) -> Result<TransparencyWitnessStoredStateV1, TransparencyWitnessStoreError>;
}

/// Restore a non-serializable in-process witness set from external state.
///
/// The returned set is safe to use for verification only insofar as the store
/// supplied the current authoritative state under its own CAS/rollback guarantees.
pub fn restore_witness_set<S: TransparencyWitnessStateStore>(
    store: &S,
    key: &TransparencyWitnessStoreKeyV1,
    policy: &super::transparency::TransparencyWitnessPolicyV1,
    log: &super::transparency::TransparencyLogAuthorityV1,
) -> Result<Option<(TransparencyWitnessStoredStateV1, TransparencyWitnessSetV1)>, TransparencyWitnessStoreError> {
    let Some(stored) = store.load(key)? else {
        return Ok(None);
    };
    stored.validate_basic()?;
    if stored.snapshot().policy_commitment() != policy.commitment()
        || stored.snapshot().log_authority_commitment() != log.commitment()
        || key.policy_commitment() != policy.commitment()
        || key.log_authority_commitment() != log.commitment()
    {
        return Err(TransparencyWitnessStoreError::IdentityMismatch);
    }
    let set = TransparencyWitnessSetV1::from_external_state(policy, log, stored.snapshot())
        .map_err(|error| TransparencyWitnessStoreError::Invalid(error.to_string()))?;
    Ok(Some((stored, set)))
}

/// Derive the exact next external witness snapshot from a verified checkpoint.
///
/// This does not mutate in-process witness state. It first checks that every
/// predecessor record captured by verification still matches the external
/// snapshot, then updates exactly the accepted quorum witnesses to the candidate
/// checkpoint. The caller should use the result as the CAS replacement.
pub fn snapshot_after_accepted(
    current: &TransparencyWitnessStateSnapshotV1,
    accepted: &AcceptedTransparencyCheckpointV1,
) -> Result<TransparencyWitnessStateSnapshotV1, TransparencyWitnessStoreError> {
    current.validate_basic()?;

    let predecessors = accepted.predecessor_state_records();
    let mut by_id = current
        .witnesses()
        .iter()
        .cloned()
        .map(|record| (record.witness_id().to_string(), record))
        .collect::<BTreeMap<_, _>>();

    for expected in predecessors {
        let actual = by_id.get(expected.witness_id()).ok_or_else(|| {
            TransparencyWitnessStoreError::StaleAcceptedCheckpoint(
                "accepted checkpoint references a witness absent from external state".to_string(),
            )
        })?;
        if actual != &expected {
            return Err(TransparencyWitnessStoreError::StaleAcceptedCheckpoint(
                "external witness state changed after checkpoint verification".to_string(),
            ));
        }
    }

    let checkpoint: &TransparencyCheckpointV1 = accepted.checkpoint();
    for witness_id in accepted.accepted_witnesses() {
        let existing = by_id.get(witness_id).ok_or_else(|| {
            TransparencyWitnessStoreError::StaleAcceptedCheckpoint(
                "accepted quorum witness is absent from external state".to_string(),
            )
        })?;
        let replacement = TransparencyWitnessRecordV1::new(
            witness_id.clone(),
            checkpoint.sequence(),
            accepted.checkpoint_digest().to_string(),
            checkpoint.event_count(),
            checkpoint.head_hash().to_string(),
            super::transparency::TransparencyVdsTreeHeadV1::new(
                checkpoint.vds_tree_size(),
                checkpoint.vds_root_hash().to_string(),
            )
            .map_err(|error| TransparencyWitnessStoreError::Invalid(error.to_string()))?,
        )
        .map_err(|error| TransparencyWitnessStoreError::Invalid(error.to_string()))?;

        if current
            .witnesses()
            .iter()
            .find(|record| record.witness_id() == witness_id)
            != Some(existing)
        {
            return Err(TransparencyWitnessStoreError::StaleAcceptedCheckpoint(
                "external witness identity changed during replacement derivation".to_string(),
            ));
        }
        by_id.insert(witness_id.clone(), replacement);
    }

    TransparencyWitnessStateSnapshotV1::new(
        current.policy_commitment().to_string(),
        current.log_authority_commitment().to_string(),
        by_id.into_values().collect(),
    )
    .map_err(|error| TransparencyWitnessStoreError::Invalid(error.to_string()))
}

/// Persist one already-derived replacement snapshot with an atomic store CAS.
///
/// This is deliberately separate from the durable journal append. Independent
/// resources cannot be made one atomic transaction by this interface; callers
/// must therefore fail closed if the store CAS rejects after the journal append
/// and must not silently replace the lost witness state with local journal data.
pub fn cas_replacement<S: TransparencyWitnessStateStore>(
    store: &S,
    key: &TransparencyWitnessStoreKeyV1,
    expected: &TransparencyWitnessStoredStateV1,
    replacement: TransparencyWitnessStateSnapshotV1,
) -> Result<TransparencyWitnessStoredStateV1, TransparencyWitnessStoreError> {
    expected.validate_basic()?;
    replacement.validate_basic()?;
    if expected.snapshot().policy_commitment() != key.policy_commitment()
        || expected.snapshot().log_authority_commitment() != key.log_authority_commitment()
        || replacement.policy_commitment() != key.policy_commitment()
        || replacement.log_authority_commitment() != key.log_authority_commitment()
    {
        return Err(TransparencyWitnessStoreError::IdentityMismatch);
    }
    store.compare_and_swap(key, Some(expected), replacement)
}

/// In-memory model of an atomic external store used only for adversarial tests.
///
/// This is intentionally not advertised as restart-resistant: process restart
/// destroys this memory, by design.
#[cfg(test)]
#[derive(Default)]
pub struct MemoryTransparencyWitnessStateStore {
    values: Mutex<BTreeMap<TransparencyWitnessStoreKeyV1, TransparencyWitnessStoredStateV1>>,
}

#[cfg(test)]
impl TransparencyWitnessStateStore for MemoryTransparencyWitnessStateStore {
    fn load(
        &self,
        key: &TransparencyWitnessStoreKeyV1,
    ) -> Result<Option<TransparencyWitnessStoredStateV1>, TransparencyWitnessStoreError> {
        key.validate_basic()?;
        Ok(self.values.lock().expect("store mutex").get(key).cloned())
    }

    fn compare_and_swap(
        &self,
        key: &TransparencyWitnessStoreKeyV1,
        expected: Option<&TransparencyWitnessStoredStateV1>,
        replacement: TransparencyWitnessStateSnapshotV1,
    ) -> Result<TransparencyWitnessStoredStateV1, TransparencyWitnessStoreError> {
        key.validate_basic()?;
        replacement.validate_basic()?;

        let mut values = self.values.lock().expect("store mutex");
        let current = values.get(key);

        match (current, expected) {
            (None, None) => {}
            (Some(current), Some(expected)) if current == expected => {}
            (None, Some(_)) => {
                return Err(TransparencyWitnessStoreError::GenerationMismatch);
            }
            (Some(_), None) | (Some(_), Some(_)) => {
                return Err(TransparencyWitnessStoreError::GenerationMismatch);
            }
        }

        let next_generation = expected
            .map(|state| {
                state
                    .generation()
                    .checked_add(1)
                    .ok_or(TransparencyWitnessStoreError::GenerationExhausted)
            })
            .transpose()?
            .unwrap_or(0);

        let stored = TransparencyWitnessStoredStateV1::new(next_generation, replacement)?;
        values.insert(key.clone(), stored.clone());
        Ok(stored)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransparencyWitnessStoreError {
    Invalid(String),
    IdentityMismatch,
    GenerationMismatch,
    GenerationExhausted,
    StaleAcceptedCheckpoint(String),
    Backend(String),
}

impl std::fmt::Display for TransparencyWitnessStoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid external witness state: {message}"),
            Self::IdentityMismatch => write!(formatter, "external witness state identity mismatch"),
            Self::GenerationMismatch => write!(formatter, "external witness state generation mismatch"),
            Self::GenerationExhausted => write!(formatter, "external witness state generation exhausted"),
            Self::StaleAcceptedCheckpoint(message) => write!(formatter, "stale accepted transparency checkpoint: {message}"),
            Self::Backend(message) => write!(formatter, "external witness store backend error: {message}"),
        }
    }
}

impl std::error::Error for TransparencyWitnessStoreError {}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transparency::{
        TransparencyCheckpointUnsignedV1, TransparencyLogAuthorityV1,
        TransparencyVdsTreeHeadV1, TransparencyWitnessPolicyV1,
    };
    use ring::{rand::SystemRandom, signature::{Ed25519KeyPair, KeyPair}};

    fn keys() -> (Ed25519KeyPair, Vec<Ed25519KeyPair>) {
        let rng = SystemRandom::new();
        let log = Ed25519KeyPair::generate_pkcs8(&rng).expect("log").to_vec();
        let witnesses = (0..2)
            .map(|_| Ed25519KeyPair::generate_pkcs8(&rng).expect("witness").to_vec())
            .collect::<Vec<_>>();
        (
            Ed25519KeyPair::from_pkcs8(&log).expect("log"),
            witnesses
                .into_iter()
                .map(|bytes| Ed25519KeyPair::from_pkcs8(&bytes).expect("witness"))
                .collect(),
        )
    }

    fn snapshot() -> (TransparencyWitnessStoreKeyV1, TransparencyWitnessStateSnapshotV1) {
        let policy_commitment = "11".repeat(32);
        let log_commitment = "22".repeat(32);
        let witness_one = TransparencyWitnessRecordV1::new(
            "w1",
            0,
            "33".repeat(32),
            0,
            "GENESIS",
            TransparencyVdsTreeHeadV1::empty(),
        )
        .expect("w1");
        let witness_two = TransparencyWitnessRecordV1::new(
            "w2",
            0,
            "44".repeat(32),
            0,
            "GENESIS",
            TransparencyVdsTreeHeadV1::empty(),
        )
        .expect("w2");
        let snapshot = TransparencyWitnessStateSnapshotV1::new(
            policy_commitment.clone(),
            log_commitment.clone(),
            vec![witness_one, witness_two],
        )
        .expect("snapshot");
        (
            TransparencyWitnessStoreKeyV1::new(policy_commitment, log_commitment)
                .expect("store key"),
            snapshot,
        )
    }

    #[test]
    fn compare_and_swap_is_atomic_and_stale_writer_cannot_rollback() {
        let (key, initial_snapshot) = snapshot();
        let store = MemoryTransparencyWitnessStateStore::default();
        let first = store
            .compare_and_swap(&key, None, initial_snapshot.clone())
            .expect("initial put");

        let second_snapshot = TransparencyWitnessStateSnapshotV1::new(
            initial_snapshot.policy_commitment().to_string(),
            initial_snapshot.log_authority_commitment().to_string(),
            initial_snapshot
                .witnesses()
                .iter()
                .cloned()
                .map(|mut witness| {
                    if witness.witness_id() == "w1" {
                        witness
                    } else {
                        witness
                    }
                })
                .collect(),
        )
        .expect("replacement");

        let second = store
            .compare_and_swap(&key, Some(&first), second_snapshot)
            .expect("second put");
        assert_eq!(second.generation(), 1);

        let stale_result = store.compare_and_swap(
            &key,
            Some(&first),
            initial_snapshot,
        );
        assert!(matches!(
            stale_result,
            Err(TransparencyWitnessStoreError::GenerationMismatch)
        ));
        assert_eq!(
            store.load(&key).expect("load").expect("stored").generation(),
            1
        );
    }

    #[test]
    fn bootstrap_and_restart_round_trip_preserves_log_binding() {
        let policy_commitment = "11".repeat(32);
        let log_commitment = "22".repeat(32);
        let key = TransparencyWitnessStoreKeyV1::new(&policy_commitment, &log_commitment)
            .expect("key");
        let snapshot = TransparencyWitnessStateSnapshotV1::new(
            policy_commitment.clone(),
            log_commitment.clone(),
            vec![TransparencyWitnessRecordV1::new(
                "w1",
                0,
                "33".repeat(32),
                0,
                "GENESIS",
                TransparencyVdsTreeHeadV1::empty(),
            )
            .expect("w1")],
        )
        .expect("snapshot");
        let store = MemoryTransparencyWitnessStateStore::default();
        let stored = store
            .compare_and_swap(&key, None, snapshot)
            .expect("store");
        assert_eq!(stored.generation(), 0);
        assert_eq!(
            store.load(&key).expect("load").expect("state").snapshot(),
            stored.snapshot()
        );
    }

    #[test]
    fn stale_expected_snapshot_is_not_salvaged_by_matching_generation() {
        let (key, snapshot) = snapshot();
        let store = MemoryTransparencyWitnessStateStore::default();
        let first = store
            .compare_and_swap(&key, None, snapshot.clone())
            .expect("initial put");

        let altered = TransparencyWitnessStateSnapshotV1::new(
            snapshot.policy_commitment().to_string(),
            snapshot.log_authority_commitment().to_string(),
            snapshot.witnesses().iter().cloned().rev().collect(),
        )
        .expect("canonicalized");
        let current = store
            .compare_and_swap(&key, Some(&first), altered)
            .expect("same logical snapshot");
        assert_eq!(current.generation(), 1);

        let stale = store.compare_and_swap(
            &key,
            Some(&first),
            snapshot,
        );
        assert!(matches!(
            stale,
            Err(TransparencyWitnessStoreError::GenerationMismatch)
        ));
    }
}
