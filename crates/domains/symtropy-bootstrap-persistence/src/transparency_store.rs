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

/// Shared trait-object handle for integrating an external witness store into a
/// long-lived security context.
///
/// The backend remains the authority; this wrapper only provides object-safe,
/// cloneable ownership for adapters that need to retain the store across transitions.
#[derive(Clone)]
pub struct SharedTransparencyWitnessStateStore(
    std::sync::Arc<dyn TransparencyWitnessStateStore + Send + Sync>,
);

impl std::fmt::Debug for SharedTransparencyWitnessStateStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SharedTransparencyWitnessStateStore")
            .finish_non_exhaustive()
    }
}

impl SharedTransparencyWitnessStateStore {
    #[must_use]
    pub fn new(
        store: std::sync::Arc<dyn TransparencyWitnessStateStore + Send + Sync>,
    ) -> Self {
        Self(store)
    }

    #[must_use]
    pub fn as_store(&self) -> &(dyn TransparencyWitnessStateStore + Send + Sync) {
        self.0.as_ref()
    }
}

impl TransparencyWitnessStateStore for SharedTransparencyWitnessStateStore {
    fn load(
        &self,
        key: &TransparencyWitnessStoreKeyV1,
    ) -> Result<Option<TransparencyWitnessStoredStateV1>, TransparencyWitnessStoreError> {
        self.0.load(key)
    }

    fn compare_and_swap(
        &self,
        key: &TransparencyWitnessStoreKeyV1,
        expected: Option<&TransparencyWitnessStoredStateV1>,
        replacement: TransparencyWitnessStateSnapshotV1,
    ) -> Result<TransparencyWitnessStoredStateV1, TransparencyWitnessStoreError> {
        self.0.compare_and_swap(key, expected, replacement)
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

/// Independently validate the postconditions promised by a successful CAS.
///
/// The backend contract is part of this trust boundary, so callers do not
/// merely trust the returned value. A conforming successful CAS must return
/// exactly the requested replacement and advance the generation by one from
/// the supplied predecessor (or create generation zero for an empty key).
pub(crate) fn validate_cas_result(
    key: &TransparencyWitnessStoreKeyV1,
    expected: Option<&TransparencyWitnessStoredStateV1>,
    replacement: &TransparencyWitnessStateSnapshotV1,
    returned: &TransparencyWitnessStoredStateV1,
) -> Result<(), TransparencyWitnessStoreError> {
    key.validate_basic()?;
    if let Some(expected) = expected {
        expected.validate_basic()?;
        if expected.snapshot().policy_commitment() != key.policy_commitment()
            || expected.snapshot().log_authority_commitment() != key.log_authority_commitment()
        {
            return Err(TransparencyWitnessStoreError::IdentityMismatch);
        }
    }

    replacement.validate_basic()?;
    if replacement.policy_commitment() != key.policy_commitment()
        || replacement.log_authority_commitment() != key.log_authority_commitment()
    {
        return Err(TransparencyWitnessStoreError::IdentityMismatch);
    }

    returned.validate_basic()?;

    let expected_generation = expected
        .map(|state| {
            state
                .generation()
                .checked_add(1)
                .ok_or(TransparencyWitnessStoreError::GenerationExhausted)
        })
        .transpose()?
        .unwrap_or(0);

    if returned.generation() != expected_generation {
        return Err(TransparencyWitnessStoreError::BackendContractViolation(
            format!(
                "successful CAS returned generation {}, expected {}",
                returned.generation(),
                expected_generation
            ),
        ));
    }

    if returned.snapshot() != replacement {
        return Err(TransparencyWitnessStoreError::BackendContractViolation(
            "successful CAS returned a snapshot different from the requested replacement"
                .to_string(),
        ));
    }

    Ok(())
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
    let returned = store.compare_and_swap(key, Some(expected), replacement.clone())?;
    validate_cas_result(key, Some(expected), &replacement, &returned)?;
    Ok(returned)
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
    BackendContractViolation(String),
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
            Self::BackendContractViolation(message) => {
                write!(formatter, "external witness store backend contract violation: {message}")
            }
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
        transparency_genesis_digest, TransparencyCheckpointUnsignedV1,
        TransparencyLogAuthorityV1, TransparencyVdsTreeHeadV1, TransparencyWitnessKeyV1,
        TransparencyWitnessPolicyV1, TransparencyWitnessSignatureV1, TransparencyWitnessSetV1,
    };
    use ring::{
        rand::SystemRandom,
        signature::{Ed25519KeyPair, KeyPair},
    };

    fn encode_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn acceptance_material() -> (
        TransparencyWitnessPolicyV1,
        TransparencyLogAuthorityV1,
        Ed25519KeyPair,
        Ed25519KeyPair,
    ) {
        let rng = SystemRandom::new();
        let log_pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("log key");
        let witness_pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("witness key");
        let log_pair = Ed25519KeyPair::from_pkcs8(log_pkcs8.as_ref()).expect("log pair");
        let witness_pair =
            Ed25519KeyPair::from_pkcs8(witness_pkcs8.as_ref()).expect("witness pair");

        let witness = TransparencyWitnessKeyV1::from_public_key_hex(
            "w1",
            "domain-a",
            encode_hex(witness_pair.public_key().as_ref()),
        )
        .expect("witness");
        let policy = TransparencyWitnessPolicyV1::new(
            "policy-store",
            "log-store",
            1,
            1,
            1,
            vec![witness],
        )
        .expect("policy");
        let log = TransparencyLogAuthorityV1::from_public_key_hex(
            "log-store",
            1,
            encode_hex(log_pair.public_key().as_ref()),
        )
        .expect("log");
        (policy, log, log_pair, witness_pair)
    }

    fn base_snapshot() -> (TransparencyWitnessStoreKeyV1, TransparencyWitnessStateSnapshotV1) {
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
            vec![witness_two, witness_one],
        )
        .expect("snapshot");
        (
            TransparencyWitnessStoreKeyV1::new(policy_commitment, log_commitment)
                .expect("store key"),
            snapshot,
        )
    }

    #[test]
    fn validate_cas_result_rejects_nonconforming_backend_response() {
        let (key, replacement) = base_snapshot();
        let expected = TransparencyWitnessStoredStateV1::new(7, replacement.clone())
            .expect("expected state");

        let wrong_generation = TransparencyWitnessStoredStateV1::new(42, replacement.clone())
            .expect("wrong generation");
        let error = validate_cas_result(
            &key,
            Some(&expected),
            &replacement,
            &wrong_generation,
        )
        .expect_err("wrong generation must fail closed");
        assert!(matches!(
            error,
            TransparencyWitnessStoreError::BackendContractViolation(_)
        ));

        let mut altered_witnesses = replacement.witnesses().to_vec();
        let altered = TransparencyWitnessRecordV1::new(
            "w1",
            1,
            "55".repeat(32),
            1,
            &"66".repeat(32),
            TransparencyVdsTreeHeadV1::empty(),
        )
        .expect("altered witness");
        altered_witnesses[0] = altered;
        let wrong_snapshot = TransparencyWitnessStateSnapshotV1::new(
            replacement.policy_commitment().to_string(),
            replacement.log_authority_commitment().to_string(),
            altered_witnesses,
        )
        .expect("wrong snapshot");
        let returned = TransparencyWitnessStoredStateV1::new(8, wrong_snapshot)
            .expect("returned state");
        let error = validate_cas_result(&key, Some(&expected), &replacement, &returned)
            .expect_err("returned snapshot must match requested replacement");
        assert!(matches!(
            error,
            TransparencyWitnessStoreError::BackendContractViolation(_)
        ));
    }

    #[test]
    fn compare_and_swap_is_atomic_and_stale_writer_cannot_rollback() {
        let (key, initial_snapshot) = base_snapshot();
        let store = MemoryTransparencyWitnessStateStore::default();
        let first = store
            .compare_and_swap(&key, None, initial_snapshot.clone())
            .expect("initial put");

        let replacement = TransparencyWitnessRecordV1::new(
            "w1",
            1,
            "55".repeat(32),
            1,
            &"66".repeat(32),
            TransparencyVdsTreeHeadV1::empty(),
        )
        .expect("replacement");
        let second_snapshot = TransparencyWitnessStateSnapshotV1::new(
            initial_snapshot.policy_commitment().to_string(),
            initial_snapshot.log_authority_commitment().to_string(),
            vec![replacement, initial_snapshot.witnesses()[1].clone()],
        )
        .expect("replacement snapshot");

        let second = store
            .compare_and_swap(&key, Some(&first), second_snapshot)
            .expect("second put");
        assert_eq!(second.generation(), 1);

        let stale_result = store.compare_and_swap(&key, Some(&first), initial_snapshot);
        assert!(matches!(
            stale_result,
            Err(TransparencyWitnessStoreError::GenerationMismatch)
        ));

        let loaded = store.load(&key).expect("load").expect("stored");
        assert_eq!(loaded.generation(), 1);
        assert_eq!(loaded.snapshot().witnesses()[0].sequence(), 1);
    }

    #[test]
    fn from_external_state_requires_exact_policy_witness_set_and_log_root() {
        let (policy, log, _, _) = acceptance_material();
        let mut set = TransparencyWitnessSetV1::new(&policy).expect("set");
        set.bind_log_authority(&log).expect("bind");
        let snapshot = set.export_state().expect("export");
        let restored =
            TransparencyWitnessSetV1::from_external_state(&policy, &log, &snapshot)
                .expect("restore");

        assert_eq!(
            restored.log_authority_commitment(),
            Some(log.commitment().as_str())
        );
        assert_eq!(restored.retained_sequence("w1"), Some(0));
        assert_eq!(
            restored.retained_checkpoint_digest("w1"),
            set.retained_checkpoint_digest("w1")
        );

        let wrong_root = TransparencyWitnessStateSnapshotV1::new(
            snapshot.policy_commitment().to_string(),
            "99".repeat(32),
            snapshot.witnesses().to_vec(),
        )
        .expect("wrong-root snapshot");
        assert!(matches!(
            TransparencyWitnessSetV1::from_external_state(&policy, &log, &wrong_root),
            Err(crate::transparency::TransparencyError::LogAuthorityMismatch)
        ));
    }

    #[test]
    fn accepted_checkpoint_derives_exact_external_replacement_and_rejects_stale_state() {
        let (policy, log, log_pair, witness_pair) = acceptance_material();
        let mut set = TransparencyWitnessSetV1::new(&policy).expect("set");
        set.bind_log_authority(&log).expect("bind");
        let current = set.export_state().expect("initial snapshot");
        let genesis = transparency_genesis_digest("log-store", 1, &policy.commitment());

        let unsigned = TransparencyCheckpointUnsignedV1::new(
            "log-store",
            1,
            1,
            "bootstrap",
            1,
            1,
            &"77".repeat(32),
            0,
            TransparencyVdsTreeHeadV1::empty().root_hash(),
            &genesis,
            policy.commitment(),
        )
        .expect("checkpoint");
        let checkpoint = unsigned
            .clone()
            .into_signed(encode_hex(log_pair.sign(&unsigned.signing_digest()).as_ref()))
            .expect("signed checkpoint");
        let witness = policy.witness("w1").expect("w1");
        let witness_signature = TransparencyWitnessSignatureV1::new(
            "w1",
            encode_hex(
                witness_pair
                    .sign(&witness.signing_digest(&checkpoint))
                    .as_ref(),
            ),
        )
        .expect("witness signature");

        let accepted = set
            .verify_candidate(
                &log,
                &policy,
                &checkpoint,
                &[witness_signature],
                "bootstrap",
                1,
                1,
                &"77".repeat(32),
            )
            .expect("accepted");

        let replacement = snapshot_after_accepted(&current, &accepted).expect("replacement");
        assert_eq!(replacement.witnesses().len(), 1);
        assert_eq!(replacement.witnesses()[0].sequence(), 1);
        assert_eq!(
            replacement.witnesses()[0].checkpoint_digest(),
            accepted.checkpoint_digest()
        );

        let stale = TransparencyWitnessStateSnapshotV1::new(
            current.policy_commitment().to_string(),
            current.log_authority_commitment().to_string(),
            vec![TransparencyWitnessRecordV1::new(
                "w1",
                9,
                "aa".repeat(32),
                9,
                &"bb".repeat(32),
                TransparencyVdsTreeHeadV1::empty(),
            )
            .expect("stale record")],
        )
        .expect("stale snapshot");
        assert!(matches!(
            snapshot_after_accepted(&stale, &accepted),
            Err(TransparencyWitnessStoreError::StaleAcceptedCheckpoint(_))
        ));
    }

    #[test]
    fn serialized_snapshot_round_trip_preserves_canonical_state() {
        let (_, snapshot) = base_snapshot();
        let encoded = serde_json::to_vec(&snapshot).expect("serialize snapshot");
        let decoded: TransparencyWitnessStateSnapshotV1 =
            serde_json::from_slice(&encoded).expect("deserialize snapshot");
        decoded
            .validate_basic()
            .expect("round-trip snapshot validity");
        assert_eq!(decoded, snapshot);
        assert_eq!(
            decoded.witnesses().iter().map(|w| w.witness_id()).collect::<Vec<_>>(),
            vec!["w1", "w2"]
        );
    }

    #[test]
    fn stale_expected_snapshot_is_not_salvaged_by_matching_generation() {
        let (key, snapshot) = base_snapshot();
        let store = MemoryTransparencyWitnessStateStore::default();
        let first = store
            .compare_and_swap(&key, None, snapshot.clone())
            .expect("initial put");

        let altered = TransparencyWitnessStateSnapshotV1::new(
            snapshot.policy_commitment().to_string(),
            snapshot.log_authority_commitment().to_string(),
            vec![
                TransparencyWitnessRecordV1::new(
                    "w1",
                    1,
                    "aa".repeat(32),
                    1,
                    &"bb".repeat(32),
                    TransparencyVdsTreeHeadV1::empty(),
                )
                .expect("altered w1"),
                snapshot.witnesses()[1].clone(),
            ],
        )
        .expect("altered snapshot");
        let forged_expected =
            TransparencyWitnessStoredStateV1::new(first.generation(), altered)
                .expect("forged generation-matched state");

        let stale = store.compare_and_swap(&key, Some(&forged_expected), snapshot);
        assert!(matches!(
            stale,
            Err(TransparencyWitnessStoreError::GenerationMismatch)
        ));
    }
}
