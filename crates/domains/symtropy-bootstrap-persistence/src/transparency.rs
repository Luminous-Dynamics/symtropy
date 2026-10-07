// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Service-neutral transparency checkpoints and independently retained witnesses.
//!
//! This module is deliberately narrower than a full SCITT/VDS implementation.
//! It establishes the semantic boundary needed by the durable execution adapter:
//! an externally signed checkpoint identifies one exact durable journal head, while
//! independently retained witness state prevents silent replacement of that checkpoint
//! with a conflicting or non-extending view.
//!
//! It does not implement HTTP transport, a public transparency service, trusted time,
//! or physical execution authority. Merkle VDS mechanics are provided by the
//! sibling `transparency_vds` module and remain separate from admission policy.

use ring::signature::{ED25519, UnparsedPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::{DurableExecutionTrust, FreshnessAuthority};
use super::transparency_vds::{
    merkle_leaf_hash_sha256, verify_append_only_sha256, MerkleConsistencyProofV1,
    MerkleInclusionProofV1,
};

pub const TRANSPARENCY_CHECKPOINT_SCHEMA_VERSION: u32 = 1;
pub const TRANSPARENCY_CHECKPOINT_ALGORITHM: &str =
    "Ed25519-SHA256-VDS-JOURNAL-CHECKPOINT-v1";
pub const TRANSPARENCY_WITNESS_ALGORITHM: &str = "Ed25519-SHA256-JOURNAL-WITNESS-v1";
pub const TRANSPARENCY_INCLUSION_EVIDENCE_SCHEMA_VERSION: u32 = 1;
pub const TRANSPARENCY_INCLUSION_EVIDENCE_ALGORITHM: &str =
    "SHA-256-RFC9162-INCLUSION-EVIDENCE-v1";

const TRANSPARENCY_DOMAIN: &str = "symtropy.transparency-checkpoint.v1.vds";
const TRANSPARENCY_WITNESS_DOMAIN: &str = "symtropy.transparency-witness.v1";
const TRANSPARENCY_GENESIS_TAG: &str = "TRANSPARENCY-GENESIS-V1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyVdsTreeHeadV1 {
    tree_size: u64,
    root_hash: String,
}

impl TransparencyVdsTreeHeadV1 {
    pub fn new(tree_size: u64, root_hash: impl Into<String>) -> Result<Self, TransparencyError> {
        let root_hash = root_hash.into();
        if !is_sha256_hex(&root_hash) {
            return Err(TransparencyError::Invalid(
                "transparency VDS root must be lowercase SHA-256".to_string(),
            ));
        }
        let empty_root = encode_hex(&Sha256::digest(b""));
        if tree_size == 0 && root_hash != empty_root {
            return Err(TransparencyError::Invalid(
                "zero-sized transparency VDS must use the empty-tree root".to_string(),
            ));
        }
        Ok(Self { tree_size, root_hash })
    }

    #[must_use]
    pub fn empty() -> Self {
        Self {
            tree_size: 0,
            root_hash: encode_hex(&Sha256::digest(b"")),
        }
    }

    pub fn validate_basic(&self) -> Result<(), TransparencyError> {
        Self::new(self.tree_size, self.root_hash.clone()).map(|_| ())
    }

    #[must_use]
    pub const fn tree_size(&self) -> u64 {
        self.tree_size
    }

    #[must_use]
    pub fn root_hash(&self) -> &str {
        &self.root_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyCheckpointUnsignedV1 {
    schema_version: u32,
    algorithm: String,
    log_id: String,
    log_epoch: u64,
    sequence: u64,
    journal_namespace: String,
    seed: u64,
    event_count: u64,
    head_hash: String,
    vds_tree_head: TransparencyVdsTreeHeadV1,
    previous_checkpoint_digest: String,
    witness_policy_commitment: String,
}

impl TransparencyCheckpointUnsignedV1 {
    pub fn new(
        log_id: impl Into<String>,
        log_epoch: u64,
        sequence: u64,
        journal_namespace: impl Into<String>,
        seed: u64,
        event_count: u64,
        head_hash: impl Into<String>,
        vds_tree_size: u64,
        vds_root_hash: impl Into<String>,
        previous_checkpoint_digest: impl Into<String>,
        witness_policy_commitment: impl Into<String>,
    ) -> Result<Self, TransparencyError> {
        let value = Self {
            schema_version: TRANSPARENCY_CHECKPOINT_SCHEMA_VERSION,
            algorithm: TRANSPARENCY_CHECKPOINT_ALGORITHM.to_string(),
            log_id: log_id.into(),
            log_epoch,
            sequence,
            journal_namespace: journal_namespace.into(),
            seed,
            event_count,
            head_hash: head_hash.into(),
            vds_tree_head: TransparencyVdsTreeHeadV1::new(vds_tree_size, vds_root_hash)?,
            previous_checkpoint_digest: previous_checkpoint_digest.into(),
            witness_policy_commitment: witness_policy_commitment.into(),
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), TransparencyError> {
        if self.schema_version != TRANSPARENCY_CHECKPOINT_SCHEMA_VERSION {
            return Err(TransparencyError::Invalid(
                "unsupported transparency checkpoint schema".to_string(),
            ));
        }
        if self.algorithm != TRANSPARENCY_CHECKPOINT_ALGORITHM {
            return Err(TransparencyError::Invalid(
                "unsupported transparency checkpoint algorithm".to_string(),
            ));
        }
        validate_nonempty("log_id", &self.log_id)?;
        if self.sequence == 0 {
            return Err(TransparencyError::Invalid(
                "transparency checkpoint sequence must be non-zero".to_string(),
            ));
        }
        validate_nonempty("journal_namespace", &self.journal_namespace)?;
        if !is_sha256_hex(&self.previous_checkpoint_digest) {
            return Err(TransparencyError::Invalid(
                "previous checkpoint digest must be lowercase SHA-256".to_string(),
            ));
        }
        if !is_sha256_hex(&self.witness_policy_commitment) {
            return Err(TransparencyError::Invalid(
                "witness policy commitment must be lowercase SHA-256".to_string(),
            ));
        }
        validate_head(self.event_count, &self.head_hash)?;
        self.vds_tree_head.validate_basic()
    }

    #[must_use]
    pub fn signing_digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hash_string(&mut hasher, TRANSPARENCY_DOMAIN);
        hash_u32(&mut hasher, self.schema_version);
        hash_string(&mut hasher, &self.algorithm);
        hash_string(&mut hasher, &self.log_id);
        hash_u64(&mut hasher, self.log_epoch);
        hash_u64(&mut hasher, self.sequence);
        hash_string(&mut hasher, &self.journal_namespace);
        hash_u64(&mut hasher, self.seed);
        hash_u64(&mut hasher, self.event_count);
        hash_string(&mut hasher, &self.head_hash);
        hash_u64(&mut hasher, self.vds_tree_head.tree_size());
        hash_string(&mut hasher, self.vds_tree_head.root_hash());
        hash_string(&mut hasher, &self.previous_checkpoint_digest);
        hash_string(&mut hasher, &self.witness_policy_commitment);
        finalize_digest(hasher)
    }

    pub fn into_signed(
        self,
        signature_hex: impl Into<String>,
    ) -> Result<TransparencyCheckpointV1, TransparencyError> {
        self.validate()?;
        let signature = signature_hex.into();
        decode_exact::<64>(&signature)?;
        Ok(TransparencyCheckpointV1 {
            unsigned: self,
            signature,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyCheckpointV1 {
    unsigned: TransparencyCheckpointUnsignedV1,
    signature: String,
}

impl TransparencyCheckpointV1 {
    pub fn validate_basic(&self) -> Result<(), TransparencyError> {
        self.unsigned.validate()?;
        decode_exact::<64>(&self.signature)?;
        Ok(())
    }

    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        self.unsigned.signing_digest()
    }

    #[must_use]
    pub fn log_id(&self) -> &str {
        &self.unsigned.log_id
    }

    #[must_use]
    pub const fn log_epoch(&self) -> u64 {
        self.unsigned.log_epoch
    }

    #[must_use]
    pub const fn sequence(&self) -> u64 {
        self.unsigned.sequence
    }

    #[must_use]
    pub fn journal_namespace(&self) -> &str {
        &self.unsigned.journal_namespace
    }

    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.unsigned.seed
    }

    #[must_use]
    pub const fn event_count(&self) -> u64 {
        self.unsigned.event_count
    }

    #[must_use]
    pub fn head_hash(&self) -> &str {
        &self.unsigned.head_hash
    }

    #[must_use]
    pub fn vds_tree_size(&self) -> u64 {
        self.unsigned.vds_tree_head.tree_size()
    }

    #[must_use]
    pub fn vds_root_hash(&self) -> &str {
        self.unsigned.vds_tree_head.root_hash()
    }

    #[must_use]
    pub fn previous_checkpoint_digest(&self) -> &str {
        &self.unsigned.previous_checkpoint_digest
    }

    #[must_use]
    pub fn witness_policy_commitment(&self) -> &str {
        &self.unsigned.witness_policy_commitment
    }

    #[must_use]
    pub fn signature(&self) -> &str {
        &self.signature
    }

    #[must_use]
    pub fn unsigned(&self) -> &TransparencyCheckpointUnsignedV1 {
        &self.unsigned
    }
}

/// Service-neutral offline evidence that a concrete entry is included in the signed VDS head.
///
/// This deliberately is not a C2SP or SCITT wire receipt. The signed checkpoint supplies
/// the authoritative tree head, while the RFC 9162 inclusion proof supplies membership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyInclusionEvidenceV1 {
    schema_version: u32,
    algorithm: String,
    checkpoint: TransparencyCheckpointV1,
    proof: MerkleInclusionProofV1,
}

impl TransparencyInclusionEvidenceV1 {
    pub fn new(
        checkpoint: TransparencyCheckpointV1,
        proof: MerkleInclusionProofV1,
    ) -> Result<Self, TransparencyError> {
        let value = Self {
            schema_version: TRANSPARENCY_INCLUSION_EVIDENCE_SCHEMA_VERSION,
            algorithm: TRANSPARENCY_INCLUSION_EVIDENCE_ALGORITHM.to_string(),
            checkpoint,
            proof,
        };
        value.validate_basic()?;
        Ok(value)
    }

    pub fn validate_basic(&self) -> Result<(), TransparencyError> {
        if self.schema_version != TRANSPARENCY_INCLUSION_EVIDENCE_SCHEMA_VERSION
            || self.algorithm != TRANSPARENCY_INCLUSION_EVIDENCE_ALGORITHM
        {
            return Err(TransparencyError::Invalid(
                "unsupported transparency inclusion evidence schema or algorithm".to_string(),
            ));
        }
        self.checkpoint.validate_basic()?;
        self.proof
            .validate_basic()
            .map_err(|error| TransparencyError::Invalid(format!("invalid inclusion proof: {error}")))?;
        if self.proof.leaf_index() >= self.checkpoint.vds_tree_size() {
            return Err(TransparencyError::Invalid(
                "inclusion proof leaf index is outside the checkpoint VDS".to_string(),
            ));
        }
        Ok(())
    }

    #[must_use]
    pub fn checkpoint(&self) -> &TransparencyCheckpointV1 {
        &self.checkpoint
    }

    #[must_use]
    pub fn proof(&self) -> &MerkleInclusionProofV1 {
        &self.proof
    }

    pub fn verify_leaf_hash(
        &self,
        log: &TransparencyLogAuthorityV1,
        leaf_hash: &str,
    ) -> Result<(), TransparencyError> {
        self.validate_basic()?;
        log.verify(&self.checkpoint)?;
        if self.proof.leaf_hash() != leaf_hash {
            return Err(TransparencyError::InclusionLeafMismatch);
        }
        self.proof
            .verify_sha256(self.checkpoint.vds_tree_size(), self.checkpoint.vds_root_hash())
            .map_err(|error| TransparencyError::Invalid(format!("invalid inclusion proof: {error}")))
    }

    pub fn verify_entry(
        &self,
        log: &TransparencyLogAuthorityV1,
        entry: &[u8],
    ) -> Result<(), TransparencyError> {
        let leaf_hash = merkle_leaf_hash_sha256(entry);
        self.verify_leaf_hash(log, &leaf_hash)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparencyLogAuthorityV1 {
    log_id: String,
    log_epoch: u64,
    public_key: String,
}

impl TransparencyLogAuthorityV1 {
    pub fn from_public_key_hex(
        log_id: impl Into<String>,
        log_epoch: u64,
        public_key: impl Into<String>,
    ) -> Result<Self, TransparencyError> {
        let log_id = log_id.into();
        let public_key = public_key.into();
        validate_nonempty("log_id", &log_id)?;
        decode_exact::<32>(&public_key)?;
        Ok(Self {
            log_id,
            log_epoch,
            public_key,
        })
    }

    #[must_use]
    pub fn log_id(&self) -> &str {
        &self.log_id
    }

    #[must_use]
    pub const fn log_epoch(&self) -> u64 {
        self.log_epoch
    }

    #[must_use]
    pub fn public_key_hex(&self) -> &str {
        &self.public_key
    }

    #[must_use]
    pub fn commitment(&self) -> String {
        let mut hasher = Sha256::new();
        hash_string(&mut hasher, "symtropy.transparency-log-authority.v1");
        hash_string(&mut hasher, &self.log_id);
        hash_u64(&mut hasher, self.log_epoch);
        hash_string(&mut hasher, &self.public_key);
        encode_hex(&finalize_digest(hasher))
    }

    pub fn validate_independence_from_freshness(
        &self,
        freshness: &FreshnessAuthority,
    ) -> Result<(), TransparencyError> {
        if self.public_key == freshness.public_key_hex() {
            return Err(TransparencyError::KeyReuse(
                "transparency log authority reuses the freshness authority key".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_independence_from_execution(
        &self,
        trust: &DurableExecutionTrust,
    ) -> Result<(), TransparencyError> {
        if trust
            .keys
            .values()
            .any(|key| key.public_key == self.public_key)
        {
            return Err(TransparencyError::KeyReuse(
                "transparency log authority reuses an execution signing key".to_string(),
            ));
        }
        Ok(())
    }

    fn verify(&self, checkpoint: &TransparencyCheckpointV1) -> Result<(), TransparencyError> {
        if checkpoint.log_id() != self.log_id || checkpoint.log_epoch() != self.log_epoch {
            return Err(TransparencyError::LogMismatch);
        }
        let public_key = decode_exact::<32>(&self.public_key)?;
        let signature = decode_exact::<64>(checkpoint.signature())?;
        UnparsedPublicKey::new(&ED25519, &public_key)
            .verify(&checkpoint.digest(), &signature)
            .map_err(|_| TransparencyError::SignatureInvalid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparencyWitnessKeyV1 {
    witness_id: String,
    independence_domain: String,
    public_key: String,
}

impl TransparencyWitnessKeyV1 {
    pub fn from_public_key_hex(
        witness_id: impl Into<String>,
        independence_domain: impl Into<String>,
        public_key: impl Into<String>,
    ) -> Result<Self, TransparencyError> {
        let witness_id = witness_id.into();
        let independence_domain = independence_domain.into();
        let public_key = public_key.into();
        validate_nonempty("witness_id", &witness_id)?;
        validate_nonempty("independence_domain", &independence_domain)?;
        decode_exact::<32>(&public_key)?;
        Ok(Self {
            witness_id,
            independence_domain,
            public_key,
        })
    }

    #[must_use]
    pub fn witness_id(&self) -> &str {
        &self.witness_id
    }

    #[must_use]
    pub fn independence_domain(&self) -> &str {
        &self.independence_domain
    }

    #[must_use]
    pub fn public_key_hex(&self) -> &str {
        &self.public_key
    }

    /// Compute the domain-separated digest a witness must sign for a checkpoint.
    #[must_use]
    pub fn signing_digest(&self, checkpoint: &TransparencyCheckpointV1) -> [u8; 32] {
        witness_signing_digest(self.witness_id(), &checkpoint.digest())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransparencyWitnessPolicyV1 {
    policy_id: String,
    log_id: String,
    log_epoch: u64,
    quorum: u32,
    minimum_domains: u32,
    witnesses: BTreeMap<String, TransparencyWitnessKeyV1>,
}

impl TransparencyWitnessPolicyV1 {
    pub fn new(
        policy_id: impl Into<String>,
        log_id: impl Into<String>,
        log_epoch: u64,
        quorum: u32,
        minimum_domains: u32,
        witnesses: Vec<TransparencyWitnessKeyV1>,
    ) -> Result<Self, TransparencyError> {
        let policy_id = policy_id.into();
        let log_id = log_id.into();
        validate_nonempty("policy_id", &policy_id)?;
        validate_nonempty("log_id", &log_id)?;
        if witnesses.is_empty() {
            return Err(TransparencyError::Invalid(
                "transparency witness policy requires at least one witness".to_string(),
            ));
        }
        if quorum == 0 || quorum as usize > witnesses.len() {
            return Err(TransparencyError::Invalid(
                "transparency witness quorum is outside the witness set".to_string(),
            ));
        }
        if minimum_domains == 0 || minimum_domains > quorum {
            return Err(TransparencyError::Invalid(
                "minimum witness domains must be non-zero and no greater than quorum".to_string(),
            ));
        }

        let mut by_id = BTreeMap::new();
        let mut public_keys = BTreeMap::new();
        for witness in witnesses {
            if by_id
                .insert(witness.witness_id.clone(), witness.clone())
                .is_some()
            {
                return Err(TransparencyError::Invalid(
                    "duplicate transparency witness identity".to_string(),
                ));
            }
            if public_keys
                .insert(witness.public_key.clone(), witness.witness_id.clone())
                .is_some()
            {
                return Err(TransparencyError::Invalid(
                    "duplicate transparency witness public key".to_string(),
                ));
            }
        }

        Ok(Self {
            policy_id,
            log_id,
            log_epoch,
            quorum,
            minimum_domains,
            witnesses: by_id,
        })
    }

    #[must_use]
    pub fn policy_id(&self) -> &str {
        &self.policy_id
    }

    #[must_use]
    pub fn log_id(&self) -> &str {
        &self.log_id
    }

    #[must_use]
    pub const fn log_epoch(&self) -> u64 {
        self.log_epoch
    }

    #[must_use]
    pub const fn quorum(&self) -> u32 {
        self.quorum
    }

    #[must_use]
    pub const fn minimum_domains(&self) -> u32 {
        self.minimum_domains
    }

    #[must_use]
    pub fn witness(&self, witness_id: &str) -> Option<&TransparencyWitnessKeyV1> {
        self.witnesses.get(witness_id)
    }

    #[must_use]
    pub fn witnesses(&self) -> impl Iterator<Item = &TransparencyWitnessKeyV1> {
        self.witnesses.values()
    }

    #[must_use]
    pub fn commitment(&self) -> String {
        let mut hasher = Sha256::new();
        hash_string(&mut hasher, "symtropy.transparency-witness-policy.v1");
        hash_string(&mut hasher, &self.policy_id);
        hash_string(&mut hasher, &self.log_id);
        hash_u64(&mut hasher, self.log_epoch);
        hash_u32(&mut hasher, self.quorum);
        hash_u32(&mut hasher, self.minimum_domains);
        hash_u64(&mut hasher, self.witnesses.len() as u64);
        for witness in self.witnesses.values() {
            hash_string(&mut hasher, witness.witness_id());
            hash_string(&mut hasher, witness.independence_domain());
            hash_string(&mut hasher, witness.public_key_hex());
        }
        encode_hex(&finalize_digest(hasher))
    }

    pub fn validate_matches_log(
        &self,
        log: &TransparencyLogAuthorityV1,
    ) -> Result<(), TransparencyError> {
        if self.log_id != log.log_id() || self.log_epoch != log.log_epoch() {
            return Err(TransparencyError::LogMismatch);
        }
        if self
            .witnesses
            .values()
            .any(|witness| witness.public_key_hex() == log.public_key_hex())
        {
            return Err(TransparencyError::KeyReuse(
                "transparency witness reuses the log authority key".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_independence_from_freshness(
        &self,
        freshness: &FreshnessAuthority,
    ) -> Result<(), TransparencyError> {
        if self
            .witnesses
            .values()
            .any(|witness| witness.public_key_hex() == freshness.public_key_hex())
        {
            return Err(TransparencyError::KeyReuse(
                "transparency witness reuses the freshness authority key".to_string(),
            ));
        }
        Ok(())
    }

    pub fn validate_independence_from_execution(
        &self,
        trust: &DurableExecutionTrust,
    ) -> Result<(), TransparencyError> {
        if self.witnesses.values().any(|witness| {
            trust
                .keys
                .values()
                .any(|key| key.public_key == witness.public_key_hex())
        }) {
            return Err(TransparencyError::KeyReuse(
                "transparency witness reuses an execution signing key".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyWitnessSignatureV1 {
    witness_id: String,
    signature: String,
}

impl TransparencyWitnessSignatureV1 {
    pub fn new(
        witness_id: impl Into<String>,
        signature_hex: impl Into<String>,
    ) -> Result<Self, TransparencyError> {
        let witness_id = witness_id.into();
        let signature = signature_hex.into();
        validate_nonempty("witness_id", &witness_id)?;
        decode_exact::<64>(&signature)?;
        Ok(Self {
            witness_id,
            signature,
        })
    }

    #[must_use]
    pub fn witness_id(&self) -> &str {
        &self.witness_id
    }

    #[must_use]
    pub fn signature(&self) -> &str {
        &self.signature
    }

    pub fn validate_basic(&self) -> Result<(), TransparencyError> {
        validate_nonempty("witness_id", &self.witness_id)?;
        decode_exact::<64>(&self.signature)?;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AcceptedTransparencyCheckpointV1 {
    checkpoint: TransparencyCheckpointV1,
    accepted_witnesses: Vec<String>,
    accepted_domains: Vec<String>,
    witness_signatures: Vec<TransparencyWitnessSignatureV1>,
    vds_consistency_proof: Option<MerkleConsistencyProofV1>,
    checkpoint_digest: String,
    predecessor_states: BTreeMap<String, RetainedWitnessState>,
}

impl AcceptedTransparencyCheckpointV1 {
    #[must_use]
    pub fn checkpoint(&self) -> &TransparencyCheckpointV1 {
        &self.checkpoint
    }

    #[must_use]
    pub fn checkpoint_digest(&self) -> &str {
        &self.checkpoint_digest
    }

    #[must_use]
    pub fn accepted_witnesses(&self) -> &[String] {
        &self.accepted_witnesses
    }

    #[must_use]
    pub fn accepted_domains(&self) -> &[String] {
        &self.accepted_domains
    }

    #[must_use]
    pub fn witness_signatures(&self) -> &[TransparencyWitnessSignatureV1] {
        &self.witness_signatures
    }

    #[must_use]
    pub fn vds_consistency_proof(&self) -> Option<&MerkleConsistencyProofV1> {
        self.vds_consistency_proof.as_ref()
    }
}

#[derive(Debug)]
struct RetainedWitnessState {
    sequence: u64,
    checkpoint_digest: String,
    event_count: u64,
    head_hash: String,
    vds_tree_size: u64,
    vds_root_hash: String,
}

impl Clone for RetainedWitnessState {
    fn clone(&self) -> Self {
        Self {
            sequence: self.sequence,
            checkpoint_digest: self.checkpoint_digest.clone(),
            event_count: self.event_count,
            head_hash: self.head_hash.clone(),
            vds_tree_size: self.vds_tree_size,
            vds_root_hash: self.vds_root_hash.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TransparencyWitnessSetV1 {
    policy_commitment: String,
    witnesses: BTreeMap<String, RetainedWitnessState>,
}

impl TransparencyWitnessSetV1 {
    pub fn new(policy: &TransparencyWitnessPolicyV1) -> Result<Self, TransparencyError> {
        if policy.witnesses.is_empty() {
            return Err(TransparencyError::Invalid(
                "cannot initialize an empty transparency witness set".to_string(),
            ));
        }
        let genesis_digest =
            transparency_genesis_digest(&policy.log_id, policy.log_epoch, &policy.commitment());
        let mut witnesses = BTreeMap::new();
        for witness_id in policy.witnesses.keys() {
            witnesses.insert(
                witness_id.clone(),
                RetainedWitnessState {
                    sequence: 0,
                    checkpoint_digest: genesis_digest.clone(),
                    event_count: 0,
                    head_hash: "GENESIS".to_string(),
                    vds_tree_size: 0,
                    vds_root_hash: TransparencyVdsTreeHeadV1::empty().root_hash().to_string(),
                },
            );
        }
        Ok(Self {
            policy_commitment: policy.commitment(),
            witnesses,
        })
    }

    #[must_use]
    pub fn policy_commitment(&self) -> &str {
        &self.policy_commitment
    }

    #[must_use]
    pub fn retained_sequence(&self, witness_id: &str) -> Option<u64> {
        self.witnesses.get(witness_id).map(|state| state.sequence)
    }

    #[must_use]
    pub fn retained_checkpoint_digest(&self, witness_id: &str) -> Option<&str> {
        self.witnesses
            .get(witness_id)
            .map(|state| state.checkpoint_digest.as_str())
    }

    #[must_use]
    pub fn max_retained_sequence(&self) -> u64 {
        self.witnesses
            .values()
            .map(|state| state.sequence)
            .max()
            .unwrap_or(0)
    }

    /// Verify a strictly newer checkpoint for a new durable transition.
    pub fn verify_for_new_transition(
        &self,
        log: &TransparencyLogAuthorityV1,
        policy: &TransparencyWitnessPolicyV1,
        checkpoint: &TransparencyCheckpointV1,
        witness_signatures: &[TransparencyWitnessSignatureV1],
        journal_namespace: &str,
        seed: u64,
        event_count: u64,
        head_hash: &str,
    ) -> Result<AcceptedTransparencyCheckpointV1, TransparencyError> {
        self.verify_for_new_transition_with_vds(
            log,
            policy,
            checkpoint,
            witness_signatures,
            journal_namespace,
            seed,
            event_count,
            head_hash,
            None,
        )
    }

    pub fn verify_for_new_transition_with_vds(
        &self,
        log: &TransparencyLogAuthorityV1,
        policy: &TransparencyWitnessPolicyV1,
        checkpoint: &TransparencyCheckpointV1,
        witness_signatures: &[TransparencyWitnessSignatureV1],
        journal_namespace: &str,
        seed: u64,
        event_count: u64,
        head_hash: &str,
        vds_consistency_proof: Option<&MerkleConsistencyProofV1>,
    ) -> Result<AcceptedTransparencyCheckpointV1, TransparencyError> {
        let accepted = self.verify_candidate_with_vds(
            log,
            policy,
            checkpoint,
            witness_signatures,
            journal_namespace,
            seed,
            event_count,
            head_hash,
            vds_consistency_proof,
        )?;
        if accepted.checkpoint.sequence() <= self.max_retained_sequence() {
            return Err(TransparencyError::ReplayDetected);
        }
        Ok(accepted)
    }

    /// Verify, but do not mutate, a checkpoint against independently retained witness state.
    ///
    /// Every supplied witness signature is validated. Unknown witnesses, duplicate witness
    /// identities, invalid signatures, stale rollback candidates, same-sequence conflicting
    /// roots, and non-extending successors fail closed. A later commit consumes the accepted
    /// object and advances only after the durable journal append succeeds.
    pub fn verify_candidate(
        &self,
        log: &TransparencyLogAuthorityV1,
        policy: &TransparencyWitnessPolicyV1,
        checkpoint: &TransparencyCheckpointV1,
        witness_signatures: &[TransparencyWitnessSignatureV1],
        journal_namespace: &str,
        seed: u64,
        event_count: u64,
        head_hash: &str,
    ) -> Result<AcceptedTransparencyCheckpointV1, TransparencyError> {
        self.verify_candidate_with_vds(
            log,
            policy,
            checkpoint,
            witness_signatures,
            journal_namespace,
            seed,
            event_count,
            head_hash,
            None,
        )
    }

    pub fn verify_candidate_with_vds(
        &self,
        log: &TransparencyLogAuthorityV1,
        policy: &TransparencyWitnessPolicyV1,
        checkpoint: &TransparencyCheckpointV1,
        witness_signatures: &[TransparencyWitnessSignatureV1],
        journal_namespace: &str,
        seed: u64,
        event_count: u64,
        head_hash: &str,
        vds_consistency_proof: Option<&MerkleConsistencyProofV1>,
    ) -> Result<AcceptedTransparencyCheckpointV1, TransparencyError> {
        checkpoint.validate_basic()?;
        if self.policy_commitment != policy.commitment() {
            return Err(TransparencyError::PolicyMismatch);
        }
        policy.validate_matches_log(log)?;
        if checkpoint.witness_policy_commitment() != self.policy_commitment {
            return Err(TransparencyError::PolicyMismatch);
        }
        if checkpoint.log_id() != policy.log_id()
            || checkpoint.log_id() != log.log_id()
            || checkpoint.log_epoch() != policy.log_epoch()
            || checkpoint.log_epoch() != log.log_epoch()
        {
            return Err(TransparencyError::LogMismatch);
        }
        if checkpoint.journal_namespace() != journal_namespace
            || checkpoint.seed() != seed
            || checkpoint.event_count() != event_count
            || checkpoint.head_hash() != head_hash
        {
            return Err(TransparencyError::JournalMismatch);
        }

        log.verify(checkpoint)?;

        let checkpoint_digest = encode_hex(&checkpoint.digest());
        let mut seen = BTreeMap::<String, ()>::new();
        let mut verified = Vec::<(
            String,
            String,
            RetainedWitnessState,
            TransparencyWitnessSignatureV1,
        )>::new();

        for signature in witness_signatures {
            let witness_id = signature.witness_id().to_string();
            if seen.insert(witness_id.clone(), ()).is_some() {
                return Err(TransparencyError::DuplicateWitnessSignature);
            }
            let witness = policy
                .witness(&witness_id)
                .ok_or(TransparencyError::UnknownWitness)?;
            let public_key = decode_exact::<32>(witness.public_key_hex())?;
            let detached_signature = decode_exact::<64>(signature.signature())?;
            let signing_digest = witness_signing_digest(witness.witness_id(), &checkpoint.digest());
            UnparsedPublicKey::new(&ED25519, &public_key)
                .verify(&signing_digest, &detached_signature)
                .map_err(|_| TransparencyError::WitnessSignatureInvalid)?;

            let retained = self
                .witnesses
                .get(witness_id.as_str())
                .ok_or(TransparencyError::UnknownWitness)?
                .clone();

            if checkpoint.sequence() < retained.sequence {
                return Err(TransparencyError::RollbackDetected {
                    witness_id,
                    retained_sequence: retained.sequence,
                    candidate_sequence: checkpoint.sequence(),
                });
            }
            if checkpoint.sequence() == retained.sequence {
                if checkpoint_digest != retained.checkpoint_digest {
                    return Err(TransparencyError::EquivocationDetected { witness_id });
                }
            } else {
                // Witness continuity is deliberately independent of the durable checkpoint
                // sequence. A witness may have missed one or more intermediate checkpoints
                // and can catch up from its last observed VDS tree head. The durable journal
                // admission layer owns contiguous checkpoint sequencing and predecessor
                // lineage; here the witness owns monotonic VDS observation.
                if checkpoint.event_count() < retained.event_count {
                    return Err(TransparencyError::RollbackDetected {
                        witness_id,
                        retained_sequence: retained.sequence,
                        candidate_sequence: checkpoint.sequence(),
                    });
                }
                if checkpoint.event_count() == retained.event_count
                    && checkpoint.head_hash() != retained.head_hash
                {
                    return Err(TransparencyError::NonExtension { witness_id });
                }
            }

            verify_append_only_sha256(
                retained.vds_tree_size,
                &retained.vds_root_hash,
                checkpoint.vds_tree_size(),
                checkpoint.vds_root_hash(),
                vds_consistency_proof,
            )
            .map_err(|_| TransparencyError::VdsConsistency)?;

            verified.push((
                witness_id,
                witness.independence_domain().to_string(),
                retained,
                signature.clone(),
            ));
        }

        let mut domains = BTreeMap::<String, ()>::new();
        let mut accepted_witnesses = Vec::new();
        let mut accepted_signatures = Vec::new();
        let mut predecessor_states = BTreeMap::new();

        for (witness_id, domain, retained, signature) in &verified {
            domains.insert(domain.clone(), ());
            accepted_witnesses.push(witness_id.clone());
            accepted_signatures.push(signature.clone());
            predecessor_states.insert(witness_id.clone(), retained.clone());
        }

        accepted_signatures
            .sort_unstable_by(|left, right| left.witness_id().cmp(right.witness_id()));

        if accepted_witnesses.len() < policy.quorum as usize {
            return Err(TransparencyError::InsufficientQuorum);
        }
        if domains.len() < policy.minimum_domains as usize {
            return Err(TransparencyError::InsufficientIndependentDomains);
        }

        accepted_witnesses.sort_unstable();
        let accepted_domains = domains.into_keys().collect::<Vec<_>>();
        Ok(AcceptedTransparencyCheckpointV1 {
            checkpoint: checkpoint.clone(),
            accepted_witnesses,
            accepted_domains,
            witness_signatures: accepted_signatures,
            vds_consistency_proof: vds_consistency_proof.cloned(),
            checkpoint_digest,
            predecessor_states,
        })
    }

    /// Commit a previously accepted checkpoint after the corresponding durable append.
    ///
    /// The operation consumes the accepted proof. Any concurrent witness-state change between
    /// verification and commit is rejected rather than overwritten.
    pub fn commit_after_durable_append(
        &mut self,
        accepted: AcceptedTransparencyCheckpointV1,
    ) -> Result<(), TransparencyError> {
        for witness_id in &accepted.accepted_witnesses {
            let expected = accepted
                .predecessor_states
                .get(witness_id)
                .ok_or(TransparencyError::StaleAcceptedCheckpoint)?;
            let current = self
                .witnesses
                .get(witness_id)
                .ok_or(TransparencyError::UnknownWitness)?;
            if current.sequence != expected.sequence
                || current.checkpoint_digest != expected.checkpoint_digest
                || current.event_count != expected.event_count
                || current.head_hash != expected.head_hash
                || current.vds_tree_size != expected.vds_tree_size
                || current.vds_root_hash != expected.vds_root_hash
            {
                return Err(TransparencyError::StaleAcceptedCheckpoint);
            }
        }

        for witness_id in accepted.accepted_witnesses {
            self.witnesses.insert(
                witness_id,
                RetainedWitnessState {
                    sequence: accepted.checkpoint.sequence(),
                    checkpoint_digest: accepted.checkpoint_digest.clone(),
                    event_count: accepted.checkpoint.event_count(),
                    head_hash: accepted.checkpoint.head_hash().to_string(),
                    vds_tree_size: accepted.checkpoint.vds_tree_size(),
                    vds_root_hash: accepted.checkpoint.vds_root_hash().to_string(),
                },
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransparencyError {
    Invalid(String),
    KeyReuse(String),
    LogMismatch,
    PolicyMismatch,
    JournalMismatch,
    SignatureInvalid,
    WitnessSignatureInvalid,
    DuplicateWitnessSignature,
    UnknownWitness,
    ReplayDetected,
    RollbackDetected {
        witness_id: String,
        retained_sequence: u64,
        candidate_sequence: u64,
    },
    EquivocationDetected {
        witness_id: String,
    },
    NonExtension {
        witness_id: String,
    },
    VdsConsistency,
    InclusionLeafMismatch,
    InsufficientQuorum,
    InsufficientIndependentDomains,
    StaleAcceptedCheckpoint,
}

impl std::fmt::Display for TransparencyError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid transparency state: {message}"),
            Self::KeyReuse(message) => write!(formatter, "transparency key reuse: {message}"),
            Self::LogMismatch => write!(formatter, "transparency log mismatch"),
            Self::PolicyMismatch => write!(formatter, "transparency witness policy mismatch"),
            Self::JournalMismatch => write!(formatter, "transparency checkpoint/journal mismatch"),
            Self::SignatureInvalid => write!(formatter, "transparency log signature invalid"),
            Self::WitnessSignatureInvalid => {
                write!(formatter, "transparency witness signature invalid")
            }
            Self::DuplicateWitnessSignature => {
                write!(formatter, "duplicate transparency witness signature")
            }
            Self::UnknownWitness => write!(formatter, "unknown transparency witness"),
            Self::ReplayDetected => {
                write!(
                    formatter,
                    "transparency checkpoint is not newer than retained witness state"
                )
            }
            Self::RollbackDetected {
                witness_id,
                retained_sequence,
                candidate_sequence,
            } => write!(
                formatter,
                "witness {witness_id} detected checkpoint rollback from {retained_sequence} to {candidate_sequence}"
            ),
            Self::EquivocationDetected { witness_id } => {
                write!(
                    formatter,
                    "witness {witness_id} observed same-sequence checkpoint equivocation"
                )
            }
            Self::NonExtension { witness_id } => {
                write!(
                    formatter,
                    "witness {witness_id} observed non-extending checkpoint history"
                )
            }
            Self::VdsConsistency => {
                write!(formatter, "transparency VDS consistency proof failed")
            }
            Self::InclusionLeafMismatch => {
                write!(formatter, "transparency inclusion evidence leaf does not match entry")
            }
            Self::InsufficientQuorum => {
                write!(formatter, "transparency witness quorum insufficient")
            }
            Self::InsufficientIndependentDomains => {
                write!(
                    formatter,
                    "transparency witness independence-domain quorum insufficient"
                )
            }
            Self::StaleAcceptedCheckpoint => {
                write!(
                    formatter,
                    "accepted transparency checkpoint became stale before commit"
                )
            }
        }
    }
}

impl std::error::Error for TransparencyError {}

pub fn transparency_genesis_digest(
    log_id: &str,
    log_epoch: u64,
    policy_commitment: &str,
) -> String {
    let mut hasher = Sha256::new();
    hash_string(&mut hasher, TRANSPARENCY_GENESIS_TAG);
    hash_string(&mut hasher, log_id);
    hash_u64(&mut hasher, log_epoch);
    hash_string(&mut hasher, policy_commitment);
    encode_hex(&finalize_digest(hasher))
}

fn witness_signing_digest(witness_id: &str, checkpoint_digest: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_string(&mut hasher, TRANSPARENCY_WITNESS_DOMAIN);
    hash_string(&mut hasher, witness_id);
    hasher.update(checkpoint_digest);
    finalize_digest(hasher)
}

fn validate_nonempty(field: &str, value: &str) -> Result<(), TransparencyError> {
    if value.is_empty() {
        return Err(TransparencyError::Invalid(format!(
            "{field} must not be empty"
        )));
    }
    Ok(())
}

fn validate_head(event_count: u64, head_hash: &str) -> Result<(), TransparencyError> {
    if event_count == 0 {
        if head_hash != "GENESIS" {
            return Err(TransparencyError::Invalid(
                "empty journal transparency checkpoint must use GENESIS head".to_string(),
            ));
        }
        return Ok(());
    }
    if head_hash.len() != 64
        || head_hash
            .bytes()
            .any(|byte| !matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(TransparencyError::Invalid(
            "non-empty journal transparency checkpoint requires lowercase SHA-256 head".to_string(),
        ));
    }
    Ok(())
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hash_u64(hasher, value.len() as u64);
    hasher.update(value.as_bytes());
}

fn hash_u32(hasher: &mut Sha256, value: u32) {
    hasher.update(value.to_be_bytes());
}

fn hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_be_bytes());
}

fn finalize_digest(hasher: Sha256) -> [u8; 32] {
    let bytes = hasher.finalize();
    let mut digest = [0_u8; 32];
    digest.copy_from_slice(&bytes);
    digest
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn decode_exact<const N: usize>(value: &str) -> Result<[u8; N], TransparencyError> {
    if value.len() != N * 2 {
        return Err(TransparencyError::Invalid(format!(
            "expected {N} byte hexadecimal value"
        )));
    }
    let bytes = value.as_bytes();
    let mut output = [0_u8; N];
    for index in 0..N {
        let high = decode_hex_nibble(bytes[index * 2])?;
        let low = decode_hex_nibble(bytes[index * 2 + 1])?;
        output[index] = (high << 4) | low;
    }
    Ok(output)
}

fn decode_hex_nibble(value: u8) -> Result<u8, TransparencyError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(TransparencyError::Invalid(
            "hexadecimal value must use lowercase ASCII".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::{
        rand::SystemRandom,
        signature::{Ed25519KeyPair, KeyPair},
    };

    fn test_execution_signer() -> DurableExecutionSigner {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).expect("execution key");
        DurableExecutionSigner::from_pkcs8("execution-key", 1, pkcs8.as_ref())
            .expect("execution signer")
    }

    struct TestKeys {
        log: Ed25519KeyPair,
        witnesses: Vec<Ed25519KeyPair>,
    }

    impl TestKeys {
        fn new() -> Self {
            let rng = SystemRandom::new();
            let log_pkcs8 = Ed25519KeyPair::generate_pkcs8(&rng).expect("log key");
            let mut witnesses = Vec::new();
            for _ in 0..3 {
                witnesses.push(
                    Ed25519KeyPair::generate_pkcs8(&rng)
                        .map(|pkcs8| {
                            Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).expect("witness key")
                        })
                        .expect("witness keygen"),
                );
            }
            Self {
                log: Ed25519KeyPair::from_pkcs8(log_pkcs8.as_ref()).expect("log key"),
                witnesses,
            }
        }

        fn log_public(&self) -> String {
            encode_hex(self.log.public_key().as_ref())
        }

        fn witness_public(&self, index: usize) -> String {
            encode_hex(self.witnesses[index].public_key().as_ref())
        }
    }

    fn policy(keys: &TestKeys) -> TransparencyWitnessPolicyV1 {
        TransparencyWitnessPolicyV1::new(
            "policy-1",
            "log-1",
            1,
            2,
            2,
            vec![
                TransparencyWitnessKeyV1::from_public_key_hex(
                    "w1",
                    "domain-a",
                    keys.witness_public(0),
                )
                .expect("w1"),
                TransparencyWitnessKeyV1::from_public_key_hex(
                    "w2",
                    "domain-b",
                    keys.witness_public(1),
                )
                .expect("w2"),
                TransparencyWitnessKeyV1::from_public_key_hex(
                    "w3",
                    "domain-b",
                    keys.witness_public(2),
                )
                .expect("w3"),
            ],
        )
        .expect("policy")
    }

    fn signed_checkpoint(
        keys: &TestKeys,
        policy: &TransparencyWitnessPolicyV1,
        sequence: u64,
        event_count: u64,
        head_hash: &str,
        previous_digest: &str,
    ) -> TransparencyCheckpointV1 {
        let unsigned = TransparencyCheckpointUnsignedV1::new(
            "log-1",
            1,
            sequence,
            "bootstrap",
            1,
            event_count,
            head_hash,
            0,
            TransparencyVdsTreeHeadV1::empty().root_hash(),
            previous_digest,
            policy.commitment(),
        )
        .expect("unsigned checkpoint");
        let signature = keys.log.sign(&unsigned.signing_digest());
        unsigned
            .into_signed(encode_hex(signature.as_ref()))
            .expect("signed checkpoint")
    }

    fn witnessed_signatures(
        keys: &TestKeys,
        checkpoint: &TransparencyCheckpointV1,
        witness_ids: &[usize],
    ) -> Vec<TransparencyWitnessSignatureV1> {
        witness_ids
            .iter()
            .map(|index| {
                let witness_id = format!("w{}", index + 1);
                let digest = witness_signing_digest(&witness_id, &checkpoint.digest());
                let signature = keys.witnesses[*index].sign(&digest);
                TransparencyWitnessSignatureV1::new(witness_id, encode_hex(signature.as_ref()))
                    .expect("witness signature")
            })
            .collect()
    }

    fn accepted_first_checkpoint(
        keys: &TestKeys,
        state: &TransparencyWitnessSetV1,
        policy: &TransparencyWitnessPolicyV1,
    ) -> AcceptedTransparencyCheckpointV1 {
        let genesis = transparency_genesis_digest("log-1", 1, &policy.commitment());
        let checkpoint = signed_checkpoint(keys, policy, 1, 1, &"00".repeat(32), &genesis);
        state
            .verify_candidate(
                &TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
                    .expect("log authority"),
                policy,
                &checkpoint,
                &witnessed_signatures(keys, &checkpoint, &[0, 1]),
                "bootstrap",
                1,
                1,
                &"00".repeat(32),
            )
            .expect("accepted checkpoint")
    }

    #[test]
    fn inclusion_evidence_binds_entry_to_signed_checkpoint() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");
        let entries = vec![
            b"statement-0".to_vec(),
            b"statement-1".to_vec(),
            b"statement-2".to_vec(),
        ];
        let root = crate::transparency_vds::merkle_tree_hash_sha256(&entries);
        let unsigned = TransparencyCheckpointUnsignedV1::new(
            "log-1",
            1,
            1,
            "bootstrap",
            1,
            0,
            "GENESIS",
            entries.len() as u64,
            &root,
            &transparency_genesis_digest("log-1", 1, &policy.commitment()),
            policy.commitment(),
        )
        .expect("checkpoint");
        let signature = keys.log.sign(&unsigned.signing_digest());
        let checkpoint = unsigned
            .into_signed(encode_hex(signature.as_ref()))
            .expect("signed checkpoint");

        fn inclusion_path(index: usize, entries: &[Vec<u8>]) -> Vec<String> {
            let n = entries.len();
            if n <= 1 {
                return Vec::new();
            }
            let mut power = 1usize << (usize::BITS - 1 - n.leading_zeros());
            if power == n {
                power >>= 1;
            }
            if index < power {
                let mut path = inclusion_path(index, &entries[..power]);
                path.push(crate::transparency_vds::merkle_tree_hash_sha256(&entries[power..]));
                path
            } else {
                let mut path = inclusion_path(index - power, &entries[power..]);
                path.push(crate::transparency_vds::merkle_tree_hash_sha256(&entries[..power]));
                path
            }
        }

        let leaf_index = 1usize;
        let proof = MerkleInclusionProofV1::new(
            leaf_index as u64,
            merkle_leaf_hash_sha256(&entries[leaf_index]),
            inclusion_path(leaf_index, &entries),
        )
        .expect("inclusion proof");
        let evidence =
            TransparencyInclusionEvidenceV1::new(checkpoint, proof).expect("evidence");

        evidence
            .verify_entry(&log, &entries[leaf_index])
            .expect("verify entry");
        assert!(matches!(
            evidence.verify_entry(&log, b"tampered"),
            Err(TransparencyError::InclusionLeafMismatch)
        ));
    }

    #[test]
    fn policy_commitment_and_genesis_are_deterministic() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let second = policy(&keys);
        assert_eq!(policy.commitment(), second.commitment());
        assert_eq!(
            transparency_genesis_digest("log-1", 1, &policy.commitment()),
            transparency_genesis_digest("log-1", 1, &policy.commitment())
        );
    }

    #[test]
    fn checkpoint_rejects_non_sha256_commitments() {
        let bad_predecessor = TransparencyCheckpointUnsignedV1::new(
            "log-1",
            1,
            1,
            "bootstrap",
            1,
            1,
            &"11".repeat(32),
            1,
            &"22".repeat(32),
            "not-a-digest",
            &"33".repeat(32),
        );
        assert!(matches!(
            bad_predecessor,
            Err(TransparencyError::Invalid(message))
                if message.contains("previous checkpoint digest")
        ));

        let bad_policy_commitment = TransparencyCheckpointUnsignedV1::new(
            "log-1",
            1,
            1,
            "bootstrap",
            1,
            1,
            &"11".repeat(32),
            1,
            &"22".repeat(32),
            &"33".repeat(32),
            "also-not-a-digest",
        );
        assert!(matches!(
            bad_policy_commitment,
            Err(TransparencyError::Invalid(message))
                if message.contains("witness policy commitment")
        ));
    }

    #[test]
    fn signed_unsupported_checkpoint_schema_is_rejected() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log =
            TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
                .expect("log");
        let genesis = transparency_genesis_digest("log-1", 1, &policy.commitment());
        let mut checkpoint =
            signed_checkpoint(&keys, &policy, 1, 1, &"77".repeat(32), &genesis);

        checkpoint.unsigned.schema_version = TRANSPARENCY_CHECKPOINT_SCHEMA_VERSION + 1;
        checkpoint.signature = encode_hex(keys.log.sign(&checkpoint.digest()).as_ref());

        assert!(matches!(
            state.verify_candidate(
                &log,
                &policy,
                &checkpoint,
                &witnessed_signatures(&keys, &checkpoint, &[0, 1]),
                "bootstrap",
                1,
                1,
                &"77".repeat(32),
            ),
            Err(TransparencyError::Invalid(_))
        ));
    }

    #[test]
    fn witness_can_bootstrap_from_nonzero_external_vds_size() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");
        let mut state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let entries = vec![b"preexisting-0".to_vec(), b"preexisting-1".to_vec()];
        let root = crate::transparency_vds::merkle_tree_hash_sha256(&entries);
        let genesis = transparency_genesis_digest("log-1", 1, &policy.commitment());

        let unsigned = TransparencyCheckpointUnsignedV1::new(
            "log-1",
            1,
            1,
            "bootstrap",
            1,
            0,
            "GENESIS",
            entries.len() as u64,
            &root,
            &genesis,
            policy.commitment(),
        )
        .expect("checkpoint");
        let signature = keys.log.sign(&unsigned.signing_digest());
        let checkpoint = unsigned
            .into_signed(encode_hex(signature.as_ref()))
            .expect("signed checkpoint");

        state
            .verify_for_new_transition_with_vds(
                &log,
                &policy,
                &checkpoint,
                &witnessed_signatures(&keys, &checkpoint, &[0, 1]),
                "bootstrap",
                1,
                0,
                "GENESIS",
                None,
            )
            .expect("nonzero VDS bootstrap");
    }

    #[test]
    fn valid_checkpoint_requires_quorum_and_independent_domains() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let mut state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");

        let genesis = transparency_genesis_digest("log-1", 1, &policy.commitment());
        let checkpoint = signed_checkpoint(keys, &policy, 1, 1, &"00".repeat(32), &genesis);

        assert!(matches!(
            state.verify_candidate(
                &log,
                &policy,
                &checkpoint,
                &witnessed_signatures(&keys, &checkpoint, &[0]),
                "bootstrap",
                1,
                1,
                &"00".repeat(32),
            ),
            Err(TransparencyError::InsufficientQuorum
                | TransparencyError::InsufficientIndependentDomains)
        ));

        let accepted = state
            .verify_candidate(
                &log,
                &policy,
                &checkpoint,
                &witnessed_signatures(&keys, &checkpoint, &[0, 1]),
                "bootstrap",
                1,
                1,
                &"00".repeat(32),
            )
            .expect("quorum");

        state.commit_after_durable_append(accepted).expect("commit");
        assert_eq!(state.retained_sequence("w1"), Some(1));
        assert_eq!(state.retained_sequence("w2"), Some(1));
        assert_eq!(state.retained_sequence("w3"), Some(0));
    }

    #[test]
    fn witness_signature_order_does_not_change_acceptance_identity() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");
        let genesis = transparency_genesis_digest("log-1", 1, &policy.commitment());
        let checkpoint = signed_checkpoint(&keys, &policy, 1, 1, &"44".repeat(32), &genesis);
        let signatures = witnessed_signatures(&keys, &checkpoint, &[0, 1]);
        let reverse = vec![signatures[1].clone(), signatures[0].clone()];

        let left = state
            .verify_candidate(
                &log,
                &policy,
                &checkpoint,
                &signatures,
                "bootstrap",
                1,
                1,
                &"44".repeat(32),
            )
            .expect("forward order");
        let right = state
            .verify_candidate(
                &log,
                &policy,
                &checkpoint,
                &reverse,
                "bootstrap",
                1,
                1,
                &"44".repeat(32),
            )
            .expect("reverse order");

        assert_eq!(left.checkpoint_digest(), right.checkpoint_digest());
        assert_eq!(left.accepted_witnesses(), right.accepted_witnesses());
        assert_eq!(left.accepted_domains(), right.accepted_domains());
    }

    #[test]
    fn invalid_extra_witness_signature_is_not_ignored() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");
        let genesis = transparency_genesis_digest("log-1", 1, &policy.commitment());
        let checkpoint = signed_checkpoint(&keys, &policy, 1, 1, &"55".repeat(32), &genesis);
        let mut signatures = witnessed_signatures(&keys, &checkpoint, &[0, 1]);
        signatures.push(
            TransparencyWitnessSignatureV1::new("w3", "00".repeat(64))
                .expect("well-shaped but invalid signature"),
        );

        assert!(matches!(
            state.verify_candidate(
                &log,
                &policy,
                &checkpoint,
                &signatures,
                "bootstrap",
                1,
                1,
                &"55".repeat(32),
            ),
            Err(TransparencyError::WitnessSignatureInvalid)
        ));
    }

    #[test]
    fn verification_does_not_advance_witness_before_durable_commit() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");
        let genesis = transparency_genesis_digest("log-1", 1, &policy.commitment());
        let checkpoint = signed_checkpoint(&keys, &policy, 1, 1, &"66".repeat(32), &genesis);
        let accepted = state
            .verify_candidate(
                &log,
                &policy,
                &checkpoint,
                &witnessed_signatures(&keys, &checkpoint, &[0, 1]),
                "bootstrap",
                1,
                1,
                &"66".repeat(32),
            )
            .expect("verification");

        assert_eq!(state.retained_sequence("w1"), Some(0));
        assert_eq!(
            state.retained_checkpoint_digest("w1"),
            Some(genesis.as_str())
        );

        drop(accepted);

        assert_eq!(state.retained_sequence("w1"), Some(0));
        assert_eq!(
            state.retained_checkpoint_digest("w1"),
            Some(genesis.as_str())
        );
    }

    #[test]
    fn same_sequence_different_checkpoint_is_equivocation() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let mut state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");

        let first = accepted_first_checkpoint(&keys, &state, &policy);
        let first_checkpoint = first.checkpoint().clone();
        state.commit_after_durable_append(first).expect("commit");

        let conflicting = signed_checkpoint(
            &keys,
            &policy,
            1,
            1,
            &"11".repeat(32),
            first_checkpoint.previous_checkpoint_digest(),
        );
        let error = state
            .verify_candidate(
                &log,
                &policy,
                &conflicting,
                &witnessed_signatures(&keys, &conflicting, &[0, 1]),
                "bootstrap",
                1,
                1,
                &"11".repeat(32),
            )
            .expect_err("equivocation must reject");
        assert!(matches!(
            error,
            TransparencyError::EquivocationDetected { witness_id } if witness_id == "w1"
        ));
    }

    #[test]
    fn new_transition_rejects_idempotent_checkpoint_replay() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let mut state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");

        let first = accepted_first_checkpoint(&keys, &state, &policy);
        let checkpoint = first.checkpoint().clone();
        let signatures = first.witness_signatures().to_vec();
        state
            .commit_after_durable_append(first)
            .expect("first checkpoint");

        assert_eq!(
            state
                .verify_candidate(
                    &log,
                    &policy,
                    &checkpoint,
                    &signatures,
                    "bootstrap",
                    1,
                    1,
                    &"00".repeat(32),
                )
                .expect("idempotent read verification")
                .checkpoint_digest(),
            encode_hex(&checkpoint.digest())
        );
        assert!(matches!(
            state.verify_for_new_transition(
                &log,
                &policy,
                &checkpoint,
                &signatures,
                "bootstrap",
                1,
                1,
                &"00".repeat(32),
            ),
            Err(TransparencyError::ReplayDetected)
        ));
    }

    #[test]
    fn higher_sequence_requires_vds_continuity_when_witness_cannot_catch_up() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let mut state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");

        let first = accepted_first_checkpoint(&keys, &state, &policy);
        state.commit_after_durable_append(first).expect("commit");

        let forged = signed_checkpoint(
            &keys,
            &policy,
            2,
            2,
            &"22".repeat(32),
            &"99".repeat(32),
        );

        let error = state
            .verify_candidate(
                &log,
                &policy,
                &forged,
                &witnessed_signatures(&keys, &forged, &[0, 1]),
                "bootstrap",
                1,
                2,
                &"22".repeat(32),
            )
            .expect_err("unproven VDS growth must reject");

        assert!(matches!(error, TransparencyError::VdsConsistency));
    }

    #[test]
    fn witness_can_catch_up_after_missing_checkpoint_sequence() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let mut state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");

        let entries = vec![b"leaf-0".to_vec(), b"leaf-1".to_vec()];
        let first_root = crate::transparency_vds::merkle_tree_hash_sha256(&entries[..1]);
        let second_root = crate::transparency_vds::merkle_tree_hash_sha256(&entries);
        let growth_proof =
            MerkleConsistencyProofV1::new(vec![merkle_leaf_hash_sha256(&entries[1])])
                .expect("growth proof");

        let first = {
            let genesis =
                transparency_genesis_digest("log-1", 1, &policy.commitment());
            let unsigned = TransparencyCheckpointUnsignedV1::new(
                "log-1",
                1,
                1,
                "bootstrap",
                1,
                1,
                &"11".repeat(32),
                1,
                &first_root,
                genesis,
                policy.commitment(),
            )
            .expect("first checkpoint");
            let signature = keys.log.sign(&unsigned.signing_digest());
            unsigned
                .into_signed(encode_hex(signature.as_ref()))
                .expect("signed checkpoint")
        };
        state
            .verify_for_new_transition_with_vds(
                &log,
                &policy,
                &first,
                &witnessed_signatures(&keys, &first, &[0, 1]),
                "bootstrap",
                1,
                1,
                &"11".repeat(32),
                None,
            )
            .expect("bootstrap first checkpoint");
        let first_accepted = state
            .verify_candidate_with_vds(
                &log,
                &policy,
                &first,
                &witnessed_signatures(&keys, &first, &[0, 1]),
                "bootstrap",
                1,
                1,
                &"11".repeat(32),
                None,
            )
            .expect("first verification");
        state
            .commit_after_durable_append(first_accepted)
            .expect("first commit");

        let skipped = signed_checkpoint(
            &keys,
            &policy,
            3,
            2,
            &"33".repeat(32),
            &"22".repeat(32),
        );
        let mut skipped_unsigned = skipped.unsigned.clone();
        skipped_unsigned.vds_tree_head =
            TransparencyVdsTreeHeadV1::new(2, &second_root).expect("second tree head");
        let signature = keys.log.sign(&skipped_unsigned.signing_digest());
        let skipped = skipped_unsigned
            .into_signed(encode_hex(signature.as_ref()))
            .expect("skipped checkpoint");

        state
            .verify_for_new_transition_with_vds(
                &log,
                &policy,
                &skipped,
                &witnessed_signatures(&keys, &skipped, &[0, 1]),
                "bootstrap",
                1,
                2,
                &"33".repeat(32),
                Some(&growth_proof),
            )
            .expect("witness catch-up over missing checkpoint");
    }

    #[test]
    fn failed_commit_cannot_advance_witness_after_concurrent_change() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let mut state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let accepted = accepted_first_checkpoint(&keys, &state, &policy);

        let competing = accepted_first_checkpoint(&keys, &state, &policy);
        state
            .commit_after_durable_append(competing)
            .expect("competing commit");

        let error = state
            .commit_after_durable_append(accepted)
            .expect_err("stale accepted object must fail");
        assert!(matches!(error, TransparencyError::StaleAcceptedCheckpoint));
        assert_eq!(state.retained_sequence("w1"), Some(1));
    }

    #[test]
    fn transparency_roles_must_not_reuse_execution_key() {
        let keys = TestKeys::new();
        let execution_signer = test_execution_signer();
        let mut trust = DurableExecutionTrust::new();
        trust
            .trust_signer(&execution_signer, 0, None)
            .expect("execution trust");

        let log = TransparencyLogAuthorityV1::from_public_key_hex(
            "log-1",
            1,
            execution_signer.public_key_hex(),
        )
        .expect("log");
        let freshness =
            FreshnessAuthority::from_public_key_hex("freshness", 1, keys.witness_public(2))
                .expect("freshness");

        assert!(matches!(
            log.validate_independence_from_execution(&trust),
            Err(TransparencyError::KeyReuse(_))
        ));

        let policy = TransparencyWitnessPolicyV1::new(
            "policy-1",
            "log-1",
            1,
            2,
            2,
            vec![
                TransparencyWitnessKeyV1::from_public_key_hex(
                    "w1",
                    "domain-a",
                    execution_signer.public_key_hex(),
                )
                .expect("w1"),
                TransparencyWitnessKeyV1::from_public_key_hex(
                    "w2",
                    "domain-b",
                    keys.witness_public(0),
                )
                .expect("w2"),
            ],
        )
        .expect("policy");

        assert!(matches!(
            policy.validate_independence_from_execution(&trust),
            Err(TransparencyError::KeyReuse(_))
        ));
        assert!(
            policy
                .validate_independence_from_freshness(&freshness)
                .is_ok()
        );
    }

    #[test]
    fn witness_keys_must_not_reuse_freshness_authority() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let freshness =
            FreshnessAuthority::from_public_key_hex("freshness", 1, keys.witness_public(0))
                .expect("freshness");

        assert!(matches!(
            policy.validate_independence_from_freshness(&freshness),
            Err(TransparencyError::KeyReuse(_))
        ));
    }

    #[test]
    fn key_roles_are_explicitly_distinct() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");
        let freshness = FreshnessAuthority::from_public_key_hex("freshness", 1, keys.log_public())
            .expect("freshness");

        assert!(matches!(
            log.validate_independence_from_freshness(&freshness),
            Err(TransparencyError::KeyReuse(_))
        ));
        assert!(matches!(policy.validate_matches_log(&log), Ok(())));
    }
}
