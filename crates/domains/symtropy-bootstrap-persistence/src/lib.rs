// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Journal-first durable execution lifecycle integration for the bootstrap kernel.
//!
//! The bootstrap kernel remains storage-independent. This adapter makes the
//! verified persistence journal the durable authority for Pending/Committed/Aborted
//! execution records and treats in-memory kernel state as a checked projection.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
    sync::Arc,
};

use ring::{
    signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519},
};
use sha2::{Digest, Sha256};

use symtropy_bootstrap::{
    abort_pending_execution, abort_process_execution, commit_process_execution,
    combine_execution_state_commitments, resume_pending_execution,
    EnergyLedger, ExecutionStateAnchor, ExecutableProcessExecutionReceipt, ExecutionBudget,
    InventoryLedger, ProcessExecutionReceipt, ProcessRun, ProductionProcess,
};
use symtropy_game_state::{EventChain, EventEnvelope, StableId};
use symtropy_persistence::{JournalLoad, JournalLock, PersistenceError, SaveStore};

pub const EXECUTION_LIFECYCLE_SCHEMA_VERSION: u32 = 2;
pub const EXECUTION_EVENT_KIND: &str = "symtropy.bootstrap.execution";
pub const EXECUTION_AUTH_ALGORITHM: &str = "Ed25519-SHA256-JSON-v1";
pub const FRESHNESS_ATTESTATION_SCHEMA_VERSION: u32 = 1;
pub const FRESHNESS_ATTESTATION_ALGORITHM: &str = "Ed25519-SHA256-JOURNAL-HEAD-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalFreshnessAttestation {
    pub schema_version: u32,
    pub algorithm: String,
    pub authority_id: String,
    pub authority_epoch: u64,
    pub sequence: u64,
    pub namespace: String,
    pub seed: u64,
    pub event_count: u64,
    pub head_hash: String,
    pub trust_commitment: String,
    pub signature: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct FreshnessCursor {
    authority_commitment: String,
    last_sequence: u64,
    last_head_hash: String,
    last_event_count: u64,
}

impl FreshnessCursor {
    fn verify_candidate(
        &self,
        authority: &FreshnessAuthority,
        attestation: &ExternalFreshnessAttestation,
        chain: &EventChain<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        if self.authority_commitment != authority.commitment() {
            return Err(AdapterError::WitnessMismatch(
                "freshness cursor belongs to a different authority root".to_string(),
            ));
        }
        if attestation.sequence <= self.last_sequence {
            return Err(AdapterError::WitnessMismatch(
                "journal freshness attestation sequence is not newer than retained cursor"
                    .to_string(),
            ));
        }

        if attestation.event_count < self.last_event_count {
            return Err(AdapterError::WitnessMismatch(
                "journal freshness authority rolled back behind the retained cursor".to_string(),
            ));
        }

        if self.last_event_count > 0 {
            let index = usize::try_from(self.last_event_count - 1)
                .map_err(|_| AdapterError::WitnessMismatch(
                    "freshness cursor history index overflow".to_string(),
                ))?;
            let prior = chain.events().get(index).ok_or_else(|| {
                AdapterError::WitnessMismatch(
                    "journal freshness chain is shorter than the retained cursor".to_string(),
                )
            })?;
            if prior.event_hash != self.last_head_hash {
                return Err(AdapterError::WitnessMismatch(
                    "journal freshness checkpoint does not extend the retained cursor head"
                        .to_string(),
                ));
            }
        }

        Ok(())
    }

    fn accept_verified(
        &mut self,
        authority: &FreshnessAuthority,
        attestation: &ExternalFreshnessAttestation,
        chain: &EventChain<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        self.verify_candidate(authority, attestation, chain)?;
        self.last_sequence = attestation.sequence;
        self.last_head_hash = attestation.head_hash.clone();
        self.last_event_count = attestation.event_count;
        Ok(())
    }

    #[must_use]
    pub fn authority_commitment(&self) -> &str {
        &self.authority_commitment
    }

    #[must_use]
    pub const fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    #[must_use]
    pub fn last_head_hash(&self) -> &str {
        &self.last_head_hash
    }

    #[must_use]
    pub const fn last_event_count(&self) -> u64 {
        self.last_event_count
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshnessAuthority {
    authority_id: String,
    authority_epoch: u64,
    public_key: String,
}

impl FreshnessAuthority {
    pub fn from_public_key_hex(
        authority_id: impl Into<String>,
        authority_epoch: u64,
        public_key: impl Into<String>,
    ) -> Result<Self, AdapterError> {
        let authority_id = authority_id.into();
        let public_key = public_key.into();
        validate_key_id(&authority_id)?;
        hex_decode_exact::<32>(&public_key)
            .map_err(AdapterError::Invalid)?;
        Ok(Self {
            authority_id,
            authority_epoch,
            public_key,
        })
    }

    #[must_use]
    pub fn authority_id(&self) -> &str {
        &self.authority_id
    }

    #[must_use]
    pub const fn authority_epoch(&self) -> u64 {
        self.authority_epoch
    }

    #[must_use]
    pub fn public_key_hex(&self) -> &str {
        &self.public_key
    }

    /// Commit to the external authority identity and public key.
    #[must_use]
    pub fn commitment(&self) -> String {
        let mut hasher = Sha256::new();
        hash_string(&mut hasher, "symtropy.freshness-authority.v1");
        hash_string(&mut hasher, &self.authority_id);
        hasher.update(self.authority_epoch.to_le_bytes());
        hash_string(&mut hasher, &self.public_key);
        hex_encode(&hasher.finalize())
    }

    /// Establish a freshness cursor from a fully verified external checkpoint.
    ///
    /// This is the only production constructor: callers cannot manufacture a cursor
    /// from arbitrary checkpoint fields without first passing the authority, journal,
    /// and trust-policy verification boundary.
    pub fn bootstrap_cursor(
        &self,
        attestation: &ExternalFreshnessAttestation,
        namespace: &str,
        seed: u64,
        chain: &EventChain<ExecutionLifecycleEvent>,
        trust: &DurableExecutionTrust,
    ) -> Result<FreshnessCursor, AdapterError> {
        self.verify(attestation, namespace, seed, chain, trust)?;
        if attestation.sequence != 0
            || attestation.event_count != 0
            || attestation.head_hash != "GENESIS"
            || !chain.events().is_empty()
        {
            return Err(AdapterError::WitnessMismatch(
                "freshness cursor bootstrap is restricted to the empty GENESIS journal"
                    .to_string(),
            ));
        }
        Ok(FreshnessCursor {
            authority_commitment: self.commitment(),
            last_sequence: 0,
            last_head_hash: "GENESIS".to_string(),
            last_event_count: 0,
        })
    }

    /// Verify a new external checkpoint and advance a caller-retained monotonic cursor.
    ///
    /// A cursor rejects replay of an older authority sequence. It is deliberately not
    /// serializable here: persisting the cursor is itself a trust boundary and belongs
    /// to an external durable authority/checkpoint store.
    pub fn verify_and_advance(
        &self,
        cursor: &mut FreshnessCursor,
        attestation: &ExternalFreshnessAttestation,
        namespace: &str,
        seed: u64,
        chain: &EventChain<ExecutionLifecycleEvent>,
        trust: &DurableExecutionTrust,
    ) -> Result<(), AdapterError> {
        self.verify(attestation, namespace, seed, chain, trust)?;
        cursor.accept_verified(self, attestation, chain)
    }
    /// Verify an externally authored checkpoint against the exact current journal head.
    ///
    /// The authority key is intentionally verifier-only in this adapter. Its private
    /// signing material never enters the runtime configuration.
    pub fn verify(
        &self,
        attestation: &ExternalFreshnessAttestation,
        namespace: &str,
        seed: u64,
        chain: &EventChain<ExecutionLifecycleEvent>,
        trust: &DurableExecutionTrust,
    ) -> Result<(), AdapterError> {
        if attestation.schema_version != FRESHNESS_ATTESTATION_SCHEMA_VERSION {
            return Err(AdapterError::Invalid(
                "unsupported journal freshness attestation schema".to_string(),
            ));
        }
        if attestation.algorithm != FRESHNESS_ATTESTATION_ALGORITHM {
            return Err(AdapterError::Invalid(
                "unsupported journal freshness attestation algorithm".to_string(),
            ));
        }
        if attestation.authority_id != self.authority_id
            || attestation.authority_epoch != self.authority_epoch
        {
            return Err(AdapterError::WitnessMismatch(
                "journal freshness attestation authority does not match the configured root"
                    .to_string(),
            ));
        }
        if attestation.namespace != namespace || attestation.seed != seed {
            return Err(AdapterError::WitnessMismatch(
                "journal freshness attestation belongs to a different namespace or seed"
                    .to_string(),
            ));
        }
        if attestation.trust_commitment != trust.commitment() {
            return Err(AdapterError::WitnessMismatch(
                "journal freshness attestation is bound to a different trust policy".to_string(),
            ));
        }

        let event_count = u64::try_from(chain.events().len())
            .map_err(|_| AdapterError::Invalid("journal event count overflow".to_string()))?;
        if attestation.event_count != event_count
            || attestation.head_hash != chain.head_hash()
        {
            return Err(AdapterError::WitnessMismatch(
                "journal freshness attestation does not match the exact current journal head"
                    .to_string(),
            ));
        }
        if attestation.event_count > 0 && !is_sha256_hex(&attestation.head_hash) {
            return Err(AdapterError::Invalid(
                "non-empty freshness attestation requires SHA-256 head".to_string(),
            ));
        }
        let signature =
            hex_decode_exact::<64>(&attestation.signature).map_err(AdapterError::Invalid)?;
        let digest = freshness_attestation_digest(attestation);
        let public_key =
            hex_decode_exact::<32>(&self.public_key).map_err(AdapterError::Invalid)?;

        // The freshness authority must be distinct from all execution-signing keys.
        if trust.keys.values().any(|key| key.public_key == self.public_key) {
            return Err(AdapterError::WitnessMismatch(
                "freshness authority key is reused as an execution signing key".to_string(),
            ));
        }

        UnparsedPublicKey::new(&ED25519, &public_key)
            .verify(&digest, &signature)
            .map_err(|_| {
                AdapterError::WitnessMismatch(
                    "journal freshness attestation signature is invalid".to_string(),
                )
            })?;
        Ok(())
    }
}

/// Unified security capability for state-changing durable execution lifecycle operations.
///
/// The context owns the retained local head witness and external freshness cursor. Only the
/// externally issued attestation can be replaced between transitions; every state-changing
/// operation re-verifies the complete context while the journal writer fence is held.
#[derive(Debug)]
pub struct DurableExecutionSecurityContext {
    head_witness: JournalHeadWitness,
    freshness_authority: FreshnessAuthority,
    freshness_cursor: FreshnessCursor,
    freshness_attestation: ExternalFreshnessAttestation,
}

impl DurableExecutionSecurityContext {
    pub fn establish(
        adapter: &DurableExecutionAdapter,
        authority: FreshnessAuthority,
        cursor: FreshnessCursor,
        freshness_attestation: ExternalFreshnessAttestation,
    ) -> Result<Self, AdapterError> {
        let head_witness = adapter.capture_head_witness()?;
        let loaded = adapter.load_verified_at(&head_witness)?;
        authority.verify(
            &freshness_attestation,
            &adapter.journal_namespace,
            adapter.seed,
            &loaded.chain,
            &adapter.trust,
        )?;
        cursor.verify_candidate(&authority, &freshness_attestation, &loaded.chain)?;
        if cursor.last_sequence() == 0
            && (!loaded.chain.events().is_empty()
                || freshness_attestation.sequence != 1)
        {
            return Err(AdapterError::WitnessMismatch(
                "unadvanced freshness cursor cannot establish against an already-advanced journal"
                    .to_string(),
            ));
        }
        Ok(Self {
            head_witness,
            freshness_authority: authority,
            freshness_cursor: cursor,
            freshness_attestation,
        })
    }

    pub fn set_freshness_attestation(
        &mut self,
        freshness_attestation: ExternalFreshnessAttestation,
    ) {
        self.freshness_attestation = freshness_attestation;
    }

    #[must_use]
    pub fn head_witness(&self) -> &JournalHeadWitness {
        &self.head_witness
    }

    #[must_use]
    pub fn freshness_authority(&self) -> &FreshnessAuthority {
        &self.freshness_authority
    }

    #[must_use]
    pub fn freshness_cursor(&self) -> &FreshnessCursor {
        &self.freshness_cursor
    }

    #[must_use]
    pub fn freshness_attestation(&self) -> &ExternalFreshnessAttestation {
        &self.freshness_attestation
    }

    fn verify_before_transition(
        &self,
        adapter: &DurableExecutionAdapter,
        chain: &EventChain<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        self.head_witness.verify_exact(
            &adapter.journal_namespace,
            adapter.seed,
            chain,
            &adapter.trust,
        )?;
        self.freshness_authority.verify(
            &self.freshness_attestation,
            &adapter.journal_namespace,
            adapter.seed,
            chain,
            &adapter.trust,
        )?;
        self.freshness_cursor
            .verify_candidate(
                &self.freshness_authority,
                &self.freshness_attestation,
                chain,
            )?;
        Ok(())
    }

    fn advance_after_durable_transition(
        &mut self,
        adapter: &DurableExecutionAdapter,
        chain: &EventChain<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        adapter.advance_head_witness(&mut self.head_witness, chain)?;
        self.freshness_cursor.accept_verified(
            &self.freshness_authority,
            &self.freshness_attestation,
            chain,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalHeadWitness {
    namespace: String,
    seed: u64,
    event_count: u64,
    head_hash: String,
    trust_commitment: String,
}

impl JournalHeadWitness {
    /// Access the journal namespace bound into this retained checkpoint.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Access the deterministic journal seed bound into this retained checkpoint.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Access the number of durable events covered by this checkpoint.
    #[must_use]
    pub fn event_count(&self) -> u64 {
        self.event_count
    }

    /// Access the exact durable event hash covered by this checkpoint.
    #[must_use]
    pub fn head_hash(&self) -> &str {
        &self.head_hash
    }

    /// Access the trust-policy commitment bound into this checkpoint.
    #[must_use]
    pub fn trust_commitment(&self) -> &str {
        &self.trust_commitment
    }

    pub fn capture(
        namespace: impl Into<String>,
        seed: u64,
        chain: &EventChain<ExecutionLifecycleEvent>,
        trust: &DurableExecutionTrust,
    ) -> Result<Self, AdapterError> {
        let namespace = namespace.into();
        let event_count = u64::try_from(chain.events().len())
            .map_err(|_| AdapterError::Invalid("journal event count overflow".to_string()))?;
        let head_hash = chain.head_hash().to_string();

        if event_count == 0 {
            if head_hash != "GENESIS" {
                return Err(AdapterError::Invalid(
                    "empty journal witness must reference GENESIS".to_string(),
                ));
            }
        } else if !is_sha256_hex(&head_hash) {
            return Err(AdapterError::Invalid(
                "non-empty journal witness requires SHA-256 head".to_string(),
            ));
        }

        Ok(Self {
            namespace,
            seed,
            event_count,
            head_hash,
            trust_commitment: trust.commitment(),
        })
    }

    /// Verify the loaded journal is exactly the retained checkpoint.
    ///
    /// Mutation/recovery paths use this stricter relation so an older cloned witness
    /// cannot authorize work against a newer journal head, nor can a newer journal
    /// silently extend a stale caller checkpoint.
    pub fn verify_exact(
        &self,
        namespace: &str,
        seed: u64,
        chain: &EventChain<ExecutionLifecycleEvent>,
        trust: &DurableExecutionTrust,
    ) -> Result<(), AdapterError> {
        if self.namespace != namespace || self.seed != seed {
            return Err(AdapterError::WitnessMismatch(
                "journal head witness belongs to a different namespace or seed".to_string(),
            ));
        }
        if self.trust_commitment != trust.commitment() {
            return Err(AdapterError::WitnessMismatch(
                "journal head witness is bound to a different trust policy".to_string(),
            ));
        }

        let current_count = u64::try_from(chain.events().len())
            .map_err(|_| AdapterError::WitnessMismatch("journal event count overflow".to_string()))?;
        if current_count != self.event_count {
            return Err(AdapterError::WitnessMismatch(
                "durable journal head count differs from retained witness".to_string(),
            ));
        }

        if self.event_count == 0 {
            if self.head_hash != "GENESIS" || chain.head_hash() != "GENESIS" {
                return Err(AdapterError::WitnessMismatch(
                    "empty journal head does not match retained GENESIS witness".to_string(),
                ));
            }
            return Ok(());
        }

        if chain.head_hash() != self.head_hash {
            return Err(AdapterError::WitnessMismatch(
                "durable journal head does not exactly match retained witness".to_string(),
            ));
        }

        Ok(())
    }

    /// Verify the loaded journal is the witnessed history or a strict extension of it.
    ///
    /// A valid signature alone does not prove freshness because an attacker may
    /// replay an older, correctly signed prefix. The witness is intended to live
    /// outside the journal authority and be retained across restarts.
    pub fn verify_against(
        &self,
        namespace: &str,
        seed: u64,
        chain: &EventChain<ExecutionLifecycleEvent>,
        trust: &DurableExecutionTrust,
    ) -> Result<(), AdapterError> {
        if self.namespace != namespace || self.seed != seed {
            return Err(AdapterError::WitnessMismatch(
                "journal head witness belongs to a different namespace or seed".to_string(),
            ));
        }

        if self.trust_commitment != trust.commitment() {
            return Err(AdapterError::WitnessMismatch(
                "journal head witness is bound to a different trust policy".to_string(),
            ));
        }

        let current_count = u64::try_from(chain.events().len())
            .map_err(|_| AdapterError::Invalid("journal event count overflow".to_string()))?;

        if current_count < self.event_count {
            return Err(AdapterError::WitnessMismatch(
                "durable journal has rolled back behind the retained head witness".to_string(),
            ));
        }

        if self.event_count == 0 {
            return Ok(());
        }

        let index = usize::try_from(self.event_count - 1)
            .map_err(|_| AdapterError::Invalid("journal witness index overflow".to_string()))?;
        let actual = chain.events().get(index).ok_or_else(|| {
            AdapterError::WitnessMismatch("journal is shorter than its retained witness".to_string())
        })?;

        if actual.event_hash != self.head_hash {
            return Err(AdapterError::WitnessMismatch(
                "durable journal does not extend the retained head witness".to_string(),
            ));
        }

        Ok(())
    }
}



#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionEventAuthentication {
    pub algorithm: String,
    pub key_id: String,
    pub key_epoch: u64,
    pub public_key: String,
    pub signed_digest: String,
    pub signature: String,
}

pub struct DurableExecutionSigner {
    key_id: String,
    key_epoch: u64,
    key_pair: Ed25519KeyPair,
}

impl fmt::Debug for DurableExecutionSigner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableExecutionSigner")
            .field("key_id", &self.key_id)
            .field("key_epoch", &self.key_epoch)
            .field("public_key", &self.public_key_hex())
            .finish_non_exhaustive()
    }
}

impl DurableExecutionSigner {
    pub fn from_pkcs8(
        key_id: impl Into<String>,
        key_epoch: u64,
        pkcs8: &[u8],
    ) -> Result<Self, AdapterError> {
        let key_id = key_id.into();
        validate_key_id(&key_id)?;
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8)
            .map_err(|_| AdapterError::Invalid("invalid Ed25519 PKCS#8 signing key".to_string()))?;
        Ok(Self { key_id, key_epoch, key_pair })
    }

    #[must_use]
    pub fn key_id(&self) -> &str { &self.key_id }

    #[must_use]
    pub const fn key_epoch(&self) -> u64 { self.key_epoch }

    #[must_use]
    pub fn public_key_hex(&self) -> String {
        hex_encode(self.key_pair.public_key().as_ref())
    }

    fn sign_digest(&self, digest: &[u8; 32]) -> ExecutionEventAuthentication {
        ExecutionEventAuthentication {
            algorithm: EXECUTION_AUTH_ALGORITHM.to_string(),
            key_id: self.key_id.clone(),
            key_epoch: self.key_epoch,
            public_key: self.public_key_hex(),
            signed_digest: hex_encode(digest),
            signature: hex_encode(self.key_pair.sign(digest).as_ref()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedExecutionKey {
    key_id: String,
    key_epoch: u64,
    public_key: String,
    active_from_ordinal: u64,
    revoked_at_ordinal: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct DurableExecutionTrust {
    keys: BTreeMap<(String, u64), TrustedExecutionKey>,
}

impl DurableExecutionTrust {
    #[must_use]
    pub fn new() -> Self { Self::default() }

    /// Commit to the complete public trust policy used for journal verification.
    ///
    /// This binds key identity, epoch, public key material, activation, and
    /// revocation intervals without including any private signing material.
    #[must_use]
    pub fn commitment(&self) -> String {
        let mut hasher = Sha256::new();
        hash_string(&mut hasher, "symtropy.execution.trust-policy.v1");
        hasher.update((self.keys.len() as u64).to_le_bytes());

        for ((key_id, key_epoch), key) in &self.keys {
            hash_string(&mut hasher, key_id);
            hasher.update(key_epoch.to_le_bytes());
            hash_string(&mut hasher, &key.public_key);
            hasher.update(key.active_from_ordinal.to_le_bytes());
            match key.revoked_at_ordinal {
                Some(value) => {
                    hasher.update([1]);
                    hasher.update(value.to_le_bytes());
                }
                None => hasher.update([0]),
            }
        }

        hex_encode(&hasher.finalize())
    }

    pub fn trust_signer(
        &mut self,
        signer: &DurableExecutionSigner,
        active_from_ordinal: u64,
        revoked_at_ordinal: Option<u64>,
    ) -> Result<(), AdapterError> {
        if revoked_at_ordinal.is_some_and(|value| value <= active_from_ordinal) {
            return Err(AdapterError::Invalid(
                "key revocation ordinal must be after activation ordinal".to_string(),
            ));
        }

        let key = TrustedExecutionKey {
            key_id: signer.key_id.clone(),
            key_epoch: signer.key_epoch,
            public_key: signer.public_key_hex(),
            active_from_ordinal,
            revoked_at_ordinal,
        };

        if self
            .keys
            .contains_key(&(key.key_id.clone(), key.key_epoch))
        {
            return Err(AdapterError::Invalid(
                "duplicate trusted execution key identity and epoch".to_string(),
            ));
        }

        if self
            .keys
            .values()
            .any(|existing| existing.public_key == key.public_key)
        {
            return Err(AdapterError::Invalid(
                "execution public key cannot be trusted under multiple key identities or epochs"
                    .to_string(),
            ));
        }

        self.keys.insert((key.key_id.clone(), key.key_epoch), key);
        Ok(())
    }

    pub fn revoke_key(
        &mut self,
        key_id: &str,
        key_epoch: u64,
        revoked_at_ordinal: u64,
    ) -> Result<(), AdapterError> {
        let key = self
            .keys
            .get_mut(&(key_id.to_string(), key_epoch))
            .ok_or_else(|| {
                AdapterError::Invalid("cannot revoke an unknown execution key".to_string())
            })?;

        if revoked_at_ordinal <= key.active_from_ordinal {
            return Err(AdapterError::Invalid(
                "key revocation ordinal must be after activation ordinal".to_string(),
            ));
        }
        if key
            .revoked_at_ordinal
            .is_some_and(|existing| existing < revoked_at_ordinal)
        {
            return Err(AdapterError::Invalid(
                "key revocation cannot be moved later".to_string(),
            ));
        }
        key.revoked_at_ordinal = Some(revoked_at_ordinal);
        Ok(())
    }

    fn authorize_new_event(
        &self,
        signer: &DurableExecutionSigner,
        ordinal: u64,
    ) -> Result<(), AdapterError> {
        let key = self
            .keys
            .get(&(signer.key_id.clone(), signer.key_epoch))
            .ok_or_else(|| AdapterError::Invalid("signing key is not trusted".to_string()))?;

        if key.public_key != signer.public_key_hex() {
            return Err(AdapterError::Invalid(
                "trusted execution key material does not match signer".to_string(),
            ));
        }
        if ordinal < key.active_from_ordinal
            || key.revoked_at_ordinal.is_some_and(|revoked| ordinal >= revoked)
        {
            return Err(AdapterError::Invalid(
                "signing key is not authorized for this journal ordinal".to_string(),
            ));
        }
        Ok(())
    }

    fn verify_event(
        &self,
        namespace: &str,
        seed: u64,
        ordinal: u64,
        event: &EventEnvelope<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        let auth = event.payload.authentication.as_ref().ok_or_else(|| {
            AdapterError::Invalid(
                "durable lifecycle event has no origin-authentication proof".to_string(),
            )
        })?;

        if auth.algorithm != EXECUTION_AUTH_ALGORITHM {
            return Err(AdapterError::Invalid(
                "unsupported execution authentication algorithm".to_string(),
            ));
        }

        let key = self
            .keys
            .get(&(auth.key_id.clone(), auth.key_epoch))
            .ok_or_else(|| {
                AdapterError::Invalid("execution event was signed by an untrusted key".to_string())
            })?;

        if auth.public_key != key.public_key {
            return Err(AdapterError::Invalid(
                "execution event public key does not match trusted key material".to_string(),
            ));
        }
        if ordinal < key.active_from_ordinal
            || key.revoked_at_ordinal.is_some_and(|revoked| ordinal >= revoked)
        {
            return Err(AdapterError::Invalid(
                "execution event is outside the trusted key validity interval".to_string(),
            ));
        }

        let digest = event.payload.signing_digest(
            namespace,
            seed,
            event.event_id.as_str(),
            &event.previous_hash,
        )?;

        if auth.signed_digest != hex_encode(&digest) {
            return Err(AdapterError::Invalid(
                "execution event signed-digest commitment mismatch".to_string(),
            ));
        }

        let public_key = hex_decode_exact::<32>(&auth.public_key).map_err(AdapterError::Invalid)?;
        let signature = hex_decode_exact::<64>(&auth.signature).map_err(AdapterError::Invalid)?;

        UnparsedPublicKey::new(&ED25519, public_key)
            .verify(&digest, &signature)
            .map_err(|_| {
                AdapterError::Invalid("execution event signature verification failed".to_string())
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurableExecutionState {
    Pending,
    Committed,
    Aborted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedExecutionAnchor {
    pub domain: String,
    pub frontier: String,
    pub state_commitment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedProcessRun {
    pub process_id: String,
    pub input_material: String,
    pub input_batch_id: String,
    pub feed_mass_g: u64,
    pub output_mass_g: BTreeMap<String, u64>,
    pub waste_mass_g: u64,
    pub energy_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistedExecutionReceipt {
    pub execution_id: String,
    pub process_id: String,
    pub input_batch_id: String,
    pub waste_stream: String,
    pub first_inventory_sequence: u64,
    pub energy_sequence: u64,
    pub energy_node_id: String,
    pub state_anchor: PersistedExecutionAnchor,
    pub run: PersistedProcessRun,
}

impl PersistedExecutionReceipt {
    #[must_use]
    pub fn from_receipt(receipt: &ProcessExecutionReceipt) -> Self {
        Self {
            execution_id: receipt.execution_id().to_string(),
            process_id: receipt.process_id().to_string(),
            input_batch_id: receipt.input_batch_id().to_string(),
            waste_stream: receipt.waste_stream().to_string(),
            first_inventory_sequence: receipt.first_inventory_sequence(),
            energy_sequence: receipt.energy_sequence(),
            energy_node_id: receipt.energy_node_id().to_string(),
            state_anchor: PersistedExecutionAnchor {
                domain: receipt.state_anchor().domain().to_string(),
                frontier: receipt.state_anchor().frontier().to_string(),
                state_commitment: receipt.state_anchor().state_commitment().to_string(),
            },
            run: PersistedProcessRun {
                process_id: receipt.run().process_id.clone(),
                input_material: receipt.run().input_material.clone(),
                input_batch_id: receipt.run().input_batch_id.clone(),
                feed_mass_g: receipt.run().feed_mass_g,
                output_mass_g: receipt.run().output_mass_g.clone(),
                waste_mass_g: receipt.run().waste_mass_g,
                energy_units: receipt.run().energy_units,
            },
        }
    }

    pub fn to_receipt(&self) -> Result<ProcessExecutionReceipt, AdapterError> {
        let anchor = ExecutionStateAnchor::new(
            self.state_anchor.domain.clone(),
            self.state_anchor.frontier.clone(),
        )
        .map_err(AdapterError::Invalid)?
        .with_state_commitment(self.state_anchor.state_commitment.clone())
        .map_err(AdapterError::Invalid)?;

        let run = ProcessRun::new(
            self.run.process_id.clone(),
            self.run.input_material.clone(),
            self.run.input_batch_id.clone(),
            self.run.feed_mass_g,
            self.run.output_mass_g.clone(),
            self.run.waste_mass_g,
            self.run.energy_units,
        );

        ProcessExecutionReceipt::from_persisted_parts(
            self.execution_id.clone(),
            self.process_id.clone(),
            self.input_batch_id.clone(),
            self.waste_stream.clone(),
            self.first_inventory_sequence,
            self.energy_sequence,
            self.energy_node_id.clone(),
            anchor,
            run,
        )
        .map_err(AdapterError::Invalid)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionLifecycleEvent {
    pub schema_version: u32,
    pub execution_id: String,
    pub state: DurableExecutionState,
    pub receipt: PersistedExecutionReceipt,
    pub receipt_commitment: String,
    pub process_definition_commitment: String,
    pub pending_event_id: Option<String>,
    pub pending_event_hash: Option<String>,
    pub pre_budget_commitment: String,
    pub pre_inventory_commitment: String,
    pub pre_energy_commitment: String,
    pub post_budget_commitment: String,
    pub post_inventory_commitment: String,
    pub post_energy_commitment: String,
    pub causal_inventory_event_ids: Vec<String>,
    pub causal_energy_event_id: Option<String>,
    pub authentication: Option<ExecutionEventAuthentication>,
}

impl ExecutionLifecycleEvent {
    fn pending(
        receipt: &ProcessExecutionReceipt,
        process_definition_commitment: String,
        pre_budget_commitment: String,
        pre_inventory_commitment: String,
        pre_energy_commitment: String,
        post_budget_commitment: String,
        post_inventory_commitment: String,
        post_energy_commitment: String,
    ) -> Self {
        Self {
            schema_version: EXECUTION_LIFECYCLE_SCHEMA_VERSION,
            execution_id: receipt.execution_id().to_string(),
            state: DurableExecutionState::Pending,
            receipt: PersistedExecutionReceipt::from_receipt(receipt),
            receipt_commitment: receipt.commitment(),
            process_definition_commitment,
            pending_event_id: None,
            pending_event_hash: None,
            pre_budget_commitment,
            pre_inventory_commitment,
            pre_energy_commitment,
            post_budget_commitment,
            post_inventory_commitment,
            post_energy_commitment,
            causal_inventory_event_ids: Vec::new(),
            causal_energy_event_id: None,
            authentication: None,
        }
    }

    fn terminal(
        state: DurableExecutionState,
        receipt: &ProcessExecutionReceipt,
        process_definition_commitment: String,
        pending_event_id: String,
        pending_event_hash: String,
        pre_budget_commitment: String,
        pre_inventory_commitment: String,
        pre_energy_commitment: String,
        post_budget_commitment: String,
        post_inventory_commitment: String,
        post_energy_commitment: String,
    ) -> Self {
        let committed = matches!(state, DurableExecutionState::Committed);
        Self {
            schema_version: EXECUTION_LIFECYCLE_SCHEMA_VERSION,
            execution_id: receipt.execution_id().to_string(),
            state,
            receipt: PersistedExecutionReceipt::from_receipt(receipt),
            receipt_commitment: receipt.commitment(),
            process_definition_commitment,
            pending_event_id: Some(pending_event_id),
            pending_event_hash: Some(pending_event_hash),
            pre_budget_commitment,
            pre_inventory_commitment,
            pre_energy_commitment,
            post_budget_commitment,
            post_inventory_commitment,
            post_energy_commitment,
            causal_inventory_event_ids: if committed {
                receipt.inventory_event_ids()
            } else {
                Vec::new()
            },
            causal_energy_event_id: committed.then(|| receipt.energy_event_id()),
            authentication: None,
        }
    }

    fn signing_digest(
        &self,
        namespace: &str,
        seed: u64,
        event_id: &str,
        previous_hash: &str,
    ) -> Result<[u8; 32], AdapterError> {
        let mut unsigned = self.clone();
        unsigned.authentication = None;
        let serialized = serde_json::to_vec(&unsigned).map_err(|error| {
            AdapterError::Invalid(format!(
                "cannot serialize unsigned execution lifecycle payload: {error}"
            ))
        })?;

        let mut hasher = Sha256::new();
        hash_string(&mut hasher, "symtropy.execution.origin-auth.v1");
        hash_string(&mut hasher, namespace);
        hasher.update(seed.to_le_bytes());
        hash_string(&mut hasher, event_id);
        hash_string(&mut hasher, previous_hash);
        hash_bytes(&mut hasher, &serialized);
        Ok(hasher.finalize().into())
    }

    fn validate_basic(&self) -> Result<ProcessExecutionReceipt, AdapterError> {
        if self.schema_version != EXECUTION_LIFECYCLE_SCHEMA_VERSION {
            return Err(AdapterError::Invalid(format!(
                "unsupported execution lifecycle schema {}",
                self.schema_version
            )));
        }
        if self.execution_id.is_empty() || self.execution_id != self.receipt.execution_id {
            return Err(AdapterError::Invalid(
                "lifecycle execution identity does not match receipt".to_string(),
            ));
        }

        for (name, value) in [
            ("receipt commitment", &self.receipt_commitment),
            (
                "process definition commitment",
                &self.process_definition_commitment,
            ),
            ("pre budget commitment", &self.pre_budget_commitment),
            ("pre inventory commitment", &self.pre_inventory_commitment),
            ("pre energy commitment", &self.pre_energy_commitment),
            ("post budget commitment", &self.post_budget_commitment),
            ("post inventory commitment", &self.post_inventory_commitment),
            ("post energy commitment", &self.post_energy_commitment),
        ] {
            if !is_sha256_hex(value) {
                return Err(AdapterError::Invalid(format!(
                    "{name} must be a lowercase SHA-256 commitment"
                )));
            }
        }

        let receipt = self.receipt.to_receipt()?;
        if !receipt.state_anchor().is_state_bound() {
            return Err(AdapterError::Invalid(
                "durable lifecycle receipt cannot use an unbound execution anchor".to_string(),
            ));
        }
        if receipt.commitment() != self.receipt_commitment {
            return Err(AdapterError::Invalid(
                "durable lifecycle receipt commitment mismatch".to_string(),
            ));
        }

        match self.state {
            DurableExecutionState::Pending => {
                if self.pending_event_id.is_some()
                    || self.pending_event_hash.is_some()
                    || !self.causal_inventory_event_ids.is_empty()
                    || self.causal_energy_event_id.is_some()
                {
                    return Err(AdapterError::Invalid(
                        "Pending lifecycle event cannot carry terminal-only links".to_string(),
                    ));
                }
            }
            DurableExecutionState::Committed => {
                let pending_id = self.pending_event_id.as_deref().ok_or_else(|| {
                    AdapterError::Invalid(
                        "Committed lifecycle event is missing Pending identity".to_string(),
                    )
                })?;
                let pending_hash = self.pending_event_hash.as_deref().ok_or_else(|| {
                    AdapterError::Invalid(
                        "Committed lifecycle event is missing Pending hash".to_string(),
                    )
                })?;
                if pending_id.is_empty() || !is_sha256_hex(pending_hash) {
                    return Err(AdapterError::Invalid(
                        "Committed lifecycle event has invalid Pending linkage".to_string(),
                    ));
                }
                if self.causal_inventory_event_ids != receipt.inventory_event_ids()
                    || self.causal_energy_event_id.as_deref()
                        != Some(receipt.energy_event_id().as_str())
                {
                    return Err(AdapterError::Invalid(
                        "Committed lifecycle event causal event identities do not match receipt"
                            .to_string(),
                    ));
                }
            }
            DurableExecutionState::Aborted => {
                if self.pending_event_id.as_deref().is_none_or(str::is_empty)
                    || self
                        .pending_event_hash
                        .as_deref()
                        .is_none_or(|hash| !is_sha256_hex(hash))
                    || !self.causal_inventory_event_ids.is_empty()
                    || self.causal_energy_event_id.is_some()
                {
                    return Err(AdapterError::Invalid(
                        "Aborted lifecycle event has invalid terminal linkage".to_string(),
                    ));
                }
            }
        }
        Ok(receipt)
    }
}

#[derive(Debug)]
pub enum AdapterError {
    Persistence(PersistenceError),
    Invalid(String),
    WitnessMismatch(String),
    MissingExecution(String),
    UnexpectedState(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Persistence(error) => write!(f, "persistence error: {error}"),
            Self::Invalid(error) => write!(f, "invalid durable execution record: {error}"),
            Self::WitnessMismatch(error) => write!(f, "journal head witness rejected: {error}"),
            Self::MissingExecution(id) => {
                write!(f, "execution is absent from durable journal: {id}")
            }
            Self::UnexpectedState(error) => {
                write!(f, "durable recovery state mismatch: {error}")
            }
        }
    }
}

impl Error for AdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Persistence(error) => Some(error),
            Self::Invalid(_)
            | Self::WitnessMismatch(_)
            | Self::MissingExecution(_)
            | Self::UnexpectedState(_) => None,
        }
    }
}

impl From<PersistenceError> for AdapterError {
    fn from(error: PersistenceError) -> Self {
        Self::Persistence(error)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResult {
    Committed,
    Aborted,
}

#[derive(Debug, Clone)]
pub struct DurableExecutionAdapter {
    store: SaveStore,
    journal_namespace: String,
    anchor_domain: String,
    seed: u64,
    trust: DurableExecutionTrust,
    signer: Option<Arc<DurableExecutionSigner>>,
}

impl DurableExecutionAdapter {
    pub fn open(
        store: SaveStore,
        journal_namespace: impl Into<String>,
        anchor_domain: impl Into<String>,
        seed: u64,
    ) -> Result<Self, AdapterError> {
        let journal_namespace = journal_namespace.into();
        let anchor_domain = anchor_domain.into();

        if journal_namespace.is_empty() || journal_namespace.len() > 96 {
            return Err(AdapterError::Invalid(
                "execution journal namespace must be non-empty and <= 96 bytes".to_string(),
            ));
        }
        ExecutionStateAnchor::new(anchor_domain.clone(), "GENESIS")
            .map_err(AdapterError::Invalid)?;

        Ok(Self {
            store,
            journal_namespace,
            anchor_domain,
            seed,
            trust: DurableExecutionTrust::new(),
            signer: None,
        })
    }

    #[must_use]
    pub fn with_trust(mut self, trust: DurableExecutionTrust) -> Self {
        self.trust = trust;
        self
    }

    #[must_use]
    pub fn with_signer(mut self, signer: DurableExecutionSigner) -> Self {
        self.signer = Some(Arc::new(signer));
        self
    }

    pub fn set_signer(&mut self, signer: DurableExecutionSigner) {
        self.signer = Some(Arc::new(signer));
    }

    #[must_use]
    pub fn store(&self) -> &SaveStore {
        &self.store
    }

    #[must_use]
    pub fn trust(&self) -> &DurableExecutionTrust {
        &self.trust
    }

    /// Capture the currently verified journal head and the exact trust policy used
    /// to authenticate it.
    pub fn capture_head_witness(&self) -> Result<JournalHeadWitness, AdapterError> {
        let loaded = self.load_verified()?;
        JournalHeadWitness::capture(
            self.journal_namespace.clone(),
            self.seed,
            &loaded.chain,
            &self.trust,
        )
    }

    /// Verify the durable journal against an independently retained exact head.
    pub fn load_verified_at(
        &self,
        witness: &JournalHeadWitness,
    ) -> Result<JournalLoad<ExecutionLifecycleEvent>, AdapterError> {
        let loaded = self.load_verified()?;
        witness.verify_exact(
            &self.journal_namespace,
            self.seed,
            &loaded.chain,
            &self.trust,
        )?;
        Ok(loaded)
    }

    /// Verify an exact retained head plus a separately rooted external checkpoint,
    /// and ratchet a retained external freshness cursor.
    /// Verify the durable journal against a unified security context without advancing it.
    pub fn load_verified_with_security_context(
        &self,
        security: &DurableExecutionSecurityContext,
    ) -> Result<JournalLoad<ExecutionLifecycleEvent>, AdapterError> {
        let loaded = self.load_verified()?;
        security.verify_before_transition(self, &loaded.chain)?;
        Ok(loaded)
    }

    pub fn load_verified_with_freshness_cursor(
        &self,
        witness: &JournalHeadWitness,
        authority: &FreshnessAuthority,
        attestation: &ExternalFreshnessAttestation,
        cursor: &mut FreshnessCursor,
    ) -> Result<JournalLoad<ExecutionLifecycleEvent>, AdapterError> {
        let loaded = self.load_verified_at(witness)?;
        authority.verify_and_advance(
            cursor,
            attestation,
            &self.journal_namespace,
            self.seed,
            &loaded.chain,
            &self.trust,
        )?;
        Ok(loaded)
    }

    /// Verify an exact retained head plus a separately rooted external checkpoint.
    pub fn load_verified_with_freshness(
        &self,
        witness: &JournalHeadWitness,
        authority: &FreshnessAuthority,
        attestation: &ExternalFreshnessAttestation,
    ) -> Result<JournalLoad<ExecutionLifecycleEvent>, AdapterError> {
        let loaded = self.load_verified_at(witness)?;
        authority.verify(
            attestation,
            &self.journal_namespace,
            self.seed,
            &loaded.chain,
            &self.trust,
        )?;
        Ok(loaded)
    }

    /// Verify the durable journal against an independently retained head witness.
    pub fn load_verified_against(
        &self,
        witness: &JournalHeadWitness,
    ) -> Result<JournalLoad<ExecutionLifecycleEvent>, AdapterError> {
        let loaded = self.load_verified()?;
        witness.verify_against(
            &self.journal_namespace,
            self.seed,
            &loaded.chain,
            &self.trust,
        )?;
        Ok(loaded)
    }

    fn advance_head_witness(
        &self,
        witness: &mut JournalHeadWitness,
        chain: &EventChain<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        *witness = JournalHeadWitness::capture(
            self.journal_namespace.clone(),
            self.seed,
            chain,
            &self.trust,
        )?;
        Ok(())
    }

    pub fn load(&self) -> Result<JournalLoad<ExecutionLifecycleEvent>, AdapterError> {
        Ok(self
            .store
            .load_journal(self.journal_namespace.clone(), self.seed)?)
    }

    fn load_verified(&self) -> Result<JournalLoad<ExecutionLifecycleEvent>, AdapterError> {
        let loaded = self.load()?;
        Self::validate_authenticated_journal(
            &loaded.chain,
            &self.journal_namespace,
            self.seed,
            &self.trust,
        )?;
        for event in loaded.chain.events() {
            if event.payload.receipt.state_anchor().domain() != self.anchor_domain {
                return Err(AdapterError::Invalid(
                    "durable execution anchor domain does not match adapter domain"
                        .to_string(),
                ));
            }
        }
        Ok(loaded)
    }

    pub fn validate_authenticated_journal(
        chain: &EventChain<ExecutionLifecycleEvent>,
        namespace: &str,
        seed: u64,
        trust: &DurableExecutionTrust,
    ) -> Result<(), AdapterError> {
        Self::validate_journal(chain)?;
        let mut lifecycle_signers: BTreeMap<String, (String, u64)> = BTreeMap::new();

        for (ordinal, event) in chain.events().iter().enumerate() {
            let ordinal = u64::try_from(ordinal)
                .map_err(|_| AdapterError::Invalid("journal ordinal overflow".to_string()))?;
            let expected_id = StableId::derive(namespace, seed, ordinal);
            if event.event_id != expected_id {
                return Err(AdapterError::Invalid(
                    "authenticated execution event has an unexpected stable identity"
                        .to_string(),
                ));
            }

            trust.verify_event(namespace, seed, ordinal, event)?;

            let auth = event.payload.authentication.as_ref().ok_or_else(|| {
                AdapterError::Invalid(
                    "authenticated execution event unexpectedly lacks origin proof".to_string(),
                )
            })?;

            match event.payload.state {
                DurableExecutionState::Pending => {
                    lifecycle_signers.insert(
                        event.payload.execution_id.clone(),
                        (auth.key_id.clone(), auth.key_epoch),
                    );
                }
                DurableExecutionState::Committed | DurableExecutionState::Aborted => {
                    let expected = lifecycle_signers
                        .get(&event.payload.execution_id)
                        .ok_or_else(|| {
                            AdapterError::Invalid(
                                "terminal execution has no authenticated Pending signer"
                                    .to_string(),
                            )
                        })?;
                    if expected.0 != auth.key_id || expected.1 != auth.key_epoch {
                        return Err(AdapterError::Invalid(
                            "terminal execution changed signing authority mid-lifecycle"
                                .to_string(),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    pub fn validate_journal(
        chain: &EventChain<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        chain
            .verify()
            .map_err(|error| AdapterError::Invalid(format!(
                "journal chain verification failed: {error}"
            )))?;

        struct LifecycleSeen {
            event_id: String,
            event_hash: String,
            receipt: PersistedExecutionReceipt,
            receipt_commitment: String,
            process_definition_commitment: String,
            post_budget_commitment: String,
            post_inventory_commitment: String,
            post_energy_commitment: String,
            terminal: bool,
        }

        let mut seen: BTreeMap<String, LifecycleSeen> = BTreeMap::new();

        for event in chain.events() {
            if event.kind != EXECUTION_EVENT_KIND {
                return Err(AdapterError::Invalid(format!(
                    "unexpected execution journal event kind: {}",
                    event.kind
                )));
            }

            let receipt = event.payload.validate_basic()?;

            match event.payload.state {
                DurableExecutionState::Pending => {
                    if event.previous_hash != receipt.state_anchor().frontier() {
                        return Err(AdapterError::Invalid(format!(
                            "Pending event {} is not immediately after its anchored frontier",
                            event.event_id
                        )));
                    }

                    let combined = combine_execution_state_commitments(
                        &event.payload.pre_budget_commitment,
                        &event.payload.pre_inventory_commitment,
                    );
                    if combined != receipt.state_anchor().state_commitment() {
                        return Err(AdapterError::Invalid(
                            "Pending pre-state component commitments do not match receipt anchor"
                                .to_string(),
                        ));
                    }

                    if seen.contains_key(&event.payload.execution_id) {
                        return Err(AdapterError::Invalid(format!(
                            "duplicate Pending lifecycle record: {}",
                            event.payload.execution_id
                        )));
                    }

                    seen.insert(
                        event.payload.execution_id.clone(),
                        LifecycleSeen {
                            event_id: event.event_id.to_string(),
                            event_hash: event.event_hash.clone(),
                            receipt: event.payload.receipt.clone(),
                            receipt_commitment: event.payload.receipt_commitment.clone(),
                            process_definition_commitment: event
                                .payload
                                .process_definition_commitment
                                .clone(),
                            post_budget_commitment: event.payload.post_budget_commitment.clone(),
                            post_inventory_commitment: event
                                .payload
                                .post_inventory_commitment
                                .clone(),
                            post_energy_commitment: event
                                .payload
                                .post_energy_commitment
                                .clone(),
                            terminal: false,
                        },
                    );
                }
                DurableExecutionState::Committed | DurableExecutionState::Aborted => {
                    let prior = seen
                        .get_mut(&event.payload.execution_id)
                        .ok_or_else(|| {
                            AdapterError::Invalid(format!(
                                "terminal event {} has no Pending predecessor",
                                event.event_id
                            ))
                        })?;

                    if prior.terminal {
                        return Err(AdapterError::Invalid(format!(
                            "duplicate terminal lifecycle record: {}",
                            event.payload.execution_id
                        )));
                    }

                    if prior.receipt != event.payload.receipt
                        || prior.receipt_commitment != event.payload.receipt_commitment
                        || prior.process_definition_commitment
                            != event.payload.process_definition_commitment
                    {
                        return Err(AdapterError::Invalid(
                            "terminal lifecycle receipt does not exactly match Pending receipt"
                                .to_string(),
                        ));
                    }

                    if event.payload.pending_event_id.as_deref()
                        != Some(prior.event_id.as_str())
                        || event.payload.pending_event_hash.as_deref()
                            != Some(prior.event_hash.as_str())
                    {
                        return Err(AdapterError::Invalid(
                            "terminal lifecycle does not point to its exact Pending journal record"
                                .to_string(),
                        ));
                    }

                    if event.payload.pre_budget_commitment != prior.post_budget_commitment
                        || event.payload.pre_inventory_commitment
                            != prior.post_inventory_commitment
                        || event.payload.pre_energy_commitment != prior.post_energy_commitment
                    {
                        return Err(AdapterError::Invalid(
                            "terminal pre-state commitments do not equal Pending post-state commitments"
                                .to_string(),
                        ));
                    }

                    if matches!(event.payload.state, DurableExecutionState::Aborted)
                        && event.payload.post_energy_commitment
                            != event.payload.pre_energy_commitment
                    {
                        return Err(AdapterError::Invalid(
                            "aborted execution cannot change energy state".to_string(),
                        ));
                    }

                    prior.terminal = true;
                }
            }
        }

        let open_pending = seen.values().filter(|record| !record.terminal).count();
        if open_pending > 1 {
            return Err(AdapterError::Invalid(
                "durable execution journal contains multiple open Pending executions; lifecycle is not linearized"
                    .to_string(),
            ));
        }

        Ok(())
    }

    fn append_authenticated_payload(
        &self,
        chain: &mut EventChain<ExecutionLifecycleEvent>,
        simulation_tick: u64,
        mut payload: ExecutionLifecycleEvent,
        journal_lock: &JournalLock,
    ) -> Result<EventEnvelope<ExecutionLifecycleEvent>, AdapterError> {
        let signer = self
            .signer
            .as_ref()
            .ok_or_else(|| {
                AdapterError::Invalid(
                    "durable lifecycle signing key is not configured".to_string(),
                )
            })?;

        let ordinal = u64::try_from(chain.events().len())
            .map_err(|_| AdapterError::Invalid("journal ordinal overflow".to_string()))?;
        self.trust.authorize_new_event(signer, ordinal)?;

        let event_id = StableId::derive(&self.journal_namespace, self.seed, ordinal);
        let previous_hash = chain.head_hash().to_string();
        let digest = payload.signing_digest(
            &self.journal_namespace,
            self.seed,
            event_id.as_str(),
            &previous_hash,
        )?;
        payload.authentication = Some(signer.sign_digest(&digest));

        chain
            .append(
                simulation_tick,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                payload,
            )
            .map_err(|error| {
                AdapterError::Invalid(format!("cannot append lifecycle event: {error}"))
            })?;

        let event = chain
            .events()
            .last()
            .cloned()
            .ok_or_else(|| AdapterError::Invalid("journal append produced no event".to_string()))?;

        self.store.append_event_locked(&event, journal_lock)?;
        Ok(event)
    }

    fn pending_event(
        chain: &EventChain<ExecutionLifecycleEvent>,
        execution_id: &str,
    ) -> Result<&EventEnvelope<ExecutionLifecycleEvent>, AdapterError> {
        chain
            .events()
            .iter()
            .find(|event| {
                event.payload.execution_id == execution_id
                    && event.payload.state == DurableExecutionState::Pending
            })
            .ok_or_else(|| AdapterError::MissingExecution(execution_id.to_string()))
    }

    fn pending_record(
        chain: &EventChain<ExecutionLifecycleEvent>,
        execution_id: &str,
    ) -> Result<(String, String, PersistedExecutionReceipt), AdapterError> {
        let event = Self::pending_event(chain, execution_id)?;
        Ok((
            event.event_id.to_string(),
            event.event_hash.clone(),
            event.payload.receipt.clone(),
        ))
    }

    fn ensure_process(
        process: &ProductionProcess,
        receipt: &ProcessExecutionReceipt,
    ) -> Result<(), AdapterError> {
        process.validate_run(receipt.run()).map_err(AdapterError::Invalid)
    }

    fn ensure_process_definition(
        process: &ProductionProcess,
        expected_commitment: &str,
    ) -> Result<(), AdapterError> {
        if process.commitment() != expected_commitment {
            return Err(AdapterError::Invalid(
                "process definition commitment does not match durable execution record"
                    .to_string(),
            ));
        }
        Ok(())
    }

    fn ensure_live_matches_latest(
        chain: &EventChain<ExecutionLifecycleEvent>,
        budget: &ExecutionBudget,
        inventory: &InventoryLedger,
        energy: &EnergyLedger,
    ) -> Result<(), AdapterError> {
        let Some(last) = chain.events().last() else {
            return Ok(());
        };

        if budget.state_commitment() != last.payload.post_budget_commitment
            || inventory.state_commitment() != last.payload.post_inventory_commitment
            || energy.state_commitment() != last.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "live projection does not match the durable journal head projection".to_string(),
            ));
        }

        Ok(())
    }

    fn ensure_only_one_pending(
        chain: &EventChain<ExecutionLifecycleEvent>,
    ) -> Result<(), AdapterError> {
        if chain.events().iter().any(|event| {
            event.payload.state == DurableExecutionState::Pending
                && !chain.events().iter().any(|terminal| {
                    terminal.payload.execution_id == event.payload.execution_id
                        && matches!(
                            terminal.payload.state,
                            DurableExecutionState::Committed | DurableExecutionState::Aborted
                        )
                })
        }) {
            return Err(AdapterError::UnexpectedState(
                "durable execution adapter permits only one open Pending lifecycle record"
                    .to_string(),
            ));
        }
        Ok(())
    }

    /// Authorize a Pending execution only against a caller-retained journal-head witness.
    ///
    /// The witness is verified while the journal writer fence is held and is advanced
    /// only after the authenticated Pending record is durable.
    pub fn authorize_pending(
        &self,
        security: &mut DurableExecutionSecurityContext,
        process: &ProductionProcess,
        execution_id: impl Into<String>,
        simulation_tick: u64,
        first_inventory_sequence: u64,
        energy_sequence: u64,
        node_id: impl Into<String>,
        run: ProcessRun,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &EnergyLedger,
    ) -> Result<ProcessExecutionReceipt, AdapterError> {
        let execution_id = execution_id.into();
        let journal_lock = self.store.acquire_journal_lock()?;
        let loaded = self.load_verified()?;
        security.verify_before_transition(self, &loaded.chain)?;
        Self::ensure_only_one_pending(&loaded.chain)?;
        Self::ensure_live_matches_latest(&loaded.chain, budget, inventory, energy)?;

        if loaded
            .chain
            .events()
            .iter()
            .any(|event| event.payload.execution_id == execution_id)
        {
            return Err(AdapterError::Invalid(format!(
                "execution identity already exists in durable journal: {execution_id}"
            )));
        }

        let pre_budget_commitment = budget.state_commitment();
        let pre_inventory_commitment = inventory.state_commitment();
        let pre_energy_commitment = energy.state_commitment();
        let process_definition_commitment = process.commitment();

        let anchor = ExecutionStateAnchor::for_state(
            self.anchor_domain.clone(),
            loaded.chain.head_hash().to_string(),
            budget,
            inventory,
        )
        .map_err(AdapterError::Invalid)?;

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();

        let receipt = process
            .authorize_pending_execution_with_inventory_at_anchor(
                execution_id,
                first_inventory_sequence,
                energy_sequence,
                node_id,
                anchor,
                run,
                &mut staged_budget,
                &mut staged_inventory,
            )
            .map_err(AdapterError::Invalid)?;

        let payload = ExecutionLifecycleEvent::pending(
            &receipt,
            process_definition_commitment,
            pre_budget_commitment,
            pre_inventory_commitment,
            pre_energy_commitment.clone(),
            staged_budget.state_commitment(),
            staged_inventory.state_commitment(),
            pre_energy_commitment,
        );

        let mut chain = loaded.chain;
        self.append_authenticated_payload(&mut chain, simulation_tick, payload, &journal_lock)?;
        security.advance_after_durable_transition(self, &chain)?;

        *budget = staged_budget;
        *inventory = staged_inventory;
        Ok(receipt)
    }

    /// Recover a Pending execution only against the unified durable security context.
    ///
    /// Recovery holds the journal writer fence for the entire verification/replay window.
    pub fn recover_pending(
        &self,
        security: &mut DurableExecutionSecurityContext,
        process: &ProductionProcess,
        execution_id: &str,
        expected_anchor: &ExecutionStateAnchor,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &EnergyLedger,
    ) -> Result<ExecutableProcessExecutionReceipt, AdapterError> {
        let _journal_lock = self.store.acquire_journal_lock()?;
        let loaded = self.load_verified()?;
        security.verify_before_transition(self, &loaded.chain)?;
        let event = Self::pending_event(&loaded.chain, execution_id)?;
        let receipt = event.payload.receipt.to_receipt()?;
        Self::ensure_process_definition(
            process,
            &event.payload.process_definition_commitment,
        )?;
        Self::ensure_process(process, &receipt)?;

        if receipt.state_anchor() != expected_anchor {
            return Err(AdapterError::Invalid(
                "recovery anchor does not match persisted receipt".to_string(),
            ));
        }
        if !expected_anchor.is_state_bound() {
            return Err(AdapterError::Invalid(
                "recovery requires a state-bound durable execution anchor".to_string(),
            ));
        }

        let live_post_matches = budget.state_commitment() == event.payload.post_budget_commitment
            && inventory.state_commitment() == event.payload.post_inventory_commitment
            && energy.state_commitment() == event.payload.post_energy_commitment;

        if live_post_matches {
            let result =
                resume_pending_execution(execution_id, budget, inventory)
                    .map_err(AdapterError::Invalid)?;
                return Ok(result);
        }

        let live_pre_matches = budget.state_commitment() == event.payload.pre_budget_commitment
            && inventory.state_commitment() == event.payload.pre_inventory_commitment
            && energy.state_commitment() == event.payload.pre_energy_commitment;

        if !live_pre_matches {
            return Err(AdapterError::UnexpectedState(
                "live state matches neither durable pre-Pending nor post-Pending commitments"
                    .to_string(),
            ));
        }

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();

        process
            .restore_pending_execution_with_inventory_at_anchor(
                &receipt,
                expected_anchor,
                &mut staged_budget,
                &mut staged_inventory,
            )
            .map_err(AdapterError::Invalid)?;

        if staged_budget.state_commitment() != event.payload.post_budget_commitment
            || staged_inventory.state_commitment() != event.payload.post_inventory_commitment
            || energy.state_commitment() != event.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "restored Pending execution does not match durable post-state commitments"
                    .to_string(),
            ));
        }

        *budget = staged_budget;
        *inventory = staged_inventory;

        let result =
            resume_pending_execution(execution_id, budget, inventory)
                .map_err(AdapterError::Invalid)?;
        Ok(result)
    }

    /// Commit an execution only against the unified durable security context.
    pub fn commit(
        &self,
        security: &mut DurableExecutionSecurityContext,
        process: &ProductionProcess,
        receipt: &ExecutableProcessExecutionReceipt,
        simulation_tick: u64,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &mut EnergyLedger,
    ) -> Result<(), AdapterError> {
        let journal_lock = self.store.acquire_journal_lock()?;
        let loaded = self.load_verified()?;
        security.verify_before_transition(self, &loaded.chain)?;

        let (pending_event_id, pending_event_hash, persisted) =
            Self::pending_record(&loaded.chain, receipt.execution_id())?;
        let pending_event = Self::pending_event(&loaded.chain, receipt.execution_id())?;
        let pending = persisted.to_receipt()?;
        Self::ensure_process_definition(
            process,
            &pending_event.payload.process_definition_commitment,
        )?;
        Self::ensure_process(process, &pending)?;

        if receipt.commitment() != pending.commitment() {
            return Err(AdapterError::Invalid(
                "executable receipt does not match durable Pending receipt".to_string(),
            ));
        }

        if budget.state_commitment() != pending_event.payload.post_budget_commitment
            || inventory.state_commitment() != pending_event.payload.post_inventory_commitment
            || energy.state_commitment() != pending_event.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "live projection does not match durable Pending post-state".to_string(),
            ));
        }

        let pre_budget = budget.state_commitment();
        let pre_inventory = inventory.state_commitment();
        let pre_energy = energy.state_commitment();

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();
        let mut staged_energy = energy.clone();

        commit_process_execution(
            receipt,
            &mut staged_budget,
            &mut staged_inventory,
            &mut staged_energy,
        )
        .map_err(AdapterError::Invalid)?;

        let payload = ExecutionLifecycleEvent::terminal(
            DurableExecutionState::Committed,
            &pending,
            pending_event.payload.process_definition_commitment.clone(),
            pending_event_id,
            pending_event_hash,
            pre_budget,
            pre_inventory,
            pre_energy,
            staged_budget.state_commitment(),
            staged_inventory.state_commitment(),
            staged_energy.state_commitment(),
        );

        let mut chain = loaded.chain;
        self.append_authenticated_payload(&mut chain, simulation_tick, payload, &journal_lock)?;
        security.advance_after_durable_transition(self, &chain)?;

        *budget = staged_budget;
        *inventory = staged_inventory;
        *energy = staged_energy;
        Ok(())
    }

    /// Abort an execution only against the unified durable security context.
    pub fn abort(
        &self,
        security: &mut DurableExecutionSecurityContext,
        process: &ProductionProcess,
        receipt: &ExecutableProcessExecutionReceipt,
        simulation_tick: u64,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &mut EnergyLedger,
    ) -> Result<(), AdapterError> {
        let journal_lock = self.store.acquire_journal_lock()?;
        let loaded = self.load_verified()?;
        security.verify_before_transition(self, &loaded.chain)?;

        let (pending_event_id, pending_event_hash, persisted) =
            Self::pending_record(&loaded.chain, receipt.execution_id())?;
        let pending_event = Self::pending_event(&loaded.chain, receipt.execution_id())?;
        let pending = persisted.to_receipt()?;
        Self::ensure_process_definition(
            process,
            &pending_event.payload.process_definition_commitment,
        )?;
        Self::ensure_process(process, &pending)?;

        if receipt.commitment() != pending.commitment() {
            return Err(AdapterError::Invalid(
                "executable receipt does not match durable Pending receipt".to_string(),
            ));
        }

        if budget.state_commitment() != pending_event.payload.post_budget_commitment
            || inventory.state_commitment() != pending_event.payload.post_inventory_commitment
            || energy.state_commitment() != pending_event.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "live projection does not match durable Pending post-state".to_string(),
            ));
        }

        let pre_budget = budget.state_commitment();
        let pre_inventory = inventory.state_commitment();
        let pre_energy = energy.state_commitment();

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();

        abort_process_execution(receipt, &mut staged_budget, &mut staged_inventory)
            .map_err(AdapterError::Invalid)?;

        let payload = ExecutionLifecycleEvent::terminal(
            DurableExecutionState::Aborted,
            &pending,
            pending_event.payload.process_definition_commitment.clone(),
            pending_event_id,
            pending_event_hash,
            pre_budget,
            pre_inventory,
            pre_energy,
            staged_budget.state_commitment(),
            staged_inventory.state_commitment(),
            energy.state_commitment(),
        );

        let mut chain = loaded.chain;
        self.append_authenticated_payload(&mut chain, simulation_tick, payload, &journal_lock)?;
        security.advance_after_durable_transition(self, &chain)?;

        *budget = staged_budget;
        *inventory = staged_inventory;
        Ok(())
    }

    /// Recover a terminal execution only against the unified durable security context.
    pub fn recover_terminal(
        &self,
        security: &mut DurableExecutionSecurityContext,
        process: &ProductionProcess,
        execution_id: &str,
        expected_anchor: &ExecutionStateAnchor,
        budget: &mut ExecutionBudget,
        inventory: &mut InventoryLedger,
        energy: &mut EnergyLedger,
    ) -> Result<RecoveryResult, AdapterError> {
        let _journal_lock = self.store.acquire_journal_lock()?;
        let loaded = self.load_verified()?;
        security.verify_before_transition(self, &loaded.chain)?;

        let terminal = loaded
            .chain
            .events()
            .iter()
            .find(|event| {
                event.payload.execution_id == execution_id
                    && event.payload.state != DurableExecutionState::Pending
            })
            .ok_or_else(|| AdapterError::MissingExecution(execution_id.to_string()))?;

        let receipt = terminal.payload.receipt.to_receipt()?;
        Self::ensure_process_definition(
            process,
            &terminal.payload.process_definition_commitment,
        )?;
        Self::ensure_process(process, &receipt)?;

        if receipt.state_anchor() != expected_anchor {
            return Err(AdapterError::Invalid(
                "terminal recovery anchor does not match persisted receipt".to_string(),
            ));
        }
        if !expected_anchor.is_state_bound() {
            return Err(AdapterError::Invalid(
                "terminal recovery requires a state-bound durable execution anchor".to_string(),
            ));
        }

        let post_matches = budget.state_commitment() == terminal.payload.post_budget_commitment
            && inventory.state_commitment() == terminal.payload.post_inventory_commitment
            && energy.state_commitment() == terminal.payload.post_energy_commitment;

        if post_matches {
            let result = match terminal.payload.state {
                DurableExecutionState::Committed => RecoveryResult::Committed,
                DurableExecutionState::Aborted => RecoveryResult::Aborted,
                DurableExecutionState::Pending => unreachable!(),
            };
                return Ok(result);
        }

        let pre_matches = budget.state_commitment() == terminal.payload.pre_budget_commitment
            && inventory.state_commitment() == terminal.payload.pre_inventory_commitment
            && energy.state_commitment() == terminal.payload.pre_energy_commitment;

        if !pre_matches {
            return Err(AdapterError::UnexpectedState(
                "live state matches neither durable terminal pre-state nor post-state commitments"
                    .to_string(),
            ));
        }

        let mut staged_budget = budget.clone();
        let mut staged_inventory = inventory.clone();
        let mut staged_energy = energy.clone();

        match terminal.payload.state {
            DurableExecutionState::Committed => {
                let executable =
                    resume_pending_execution(execution_id, &staged_budget, &staged_inventory)
                        .map_err(AdapterError::Invalid)?;
                commit_process_execution(
                    &executable,
                    &mut staged_budget,
                    &mut staged_inventory,
                    &mut staged_energy,
                )
                .map_err(AdapterError::Invalid)?;
            }
            DurableExecutionState::Aborted => {
                abort_pending_execution(&receipt, &mut staged_budget, &mut staged_inventory)
                    .map_err(AdapterError::Invalid)?;
            }
            DurableExecutionState::Pending => unreachable!(),
        }

        if staged_budget.state_commitment() != terminal.payload.post_budget_commitment
            || staged_inventory.state_commitment() != terminal.payload.post_inventory_commitment
            || staged_energy.state_commitment() != terminal.payload.post_energy_commitment
        {
            return Err(AdapterError::UnexpectedState(
                "replayed terminal transition does not match durable post-state commitments"
                    .to_string(),
            ));
        }

        *budget = staged_budget;
        *inventory = staged_inventory;
        *energy = staged_energy;

        let result = match terminal.payload.state {
            DurableExecutionState::Committed => RecoveryResult::Committed,
            DurableExecutionState::Aborted => RecoveryResult::Aborted,
            DurableExecutionState::Pending => unreachable!(),
        };
        Ok(result)
    }
}

fn validate_key_id(value: &str) -> Result<(), AdapterError> {
    if value.is_empty()
        || value.len() > 96
        || !value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
        })
    {
        return Err(AdapterError::Invalid(
            "execution key ID must be portable, non-empty, and <= 96 bytes".to_string(),
        ));
    }
    Ok(())
}

fn freshness_attestation_digest(attestation: &ExternalFreshnessAttestation) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hash_string(
        &mut hasher,
        "symtropy.journal.freshness-attestation.v1",
    );
    hash_string(&mut hasher, &attestation.authority_id);
    hasher.update(attestation.schema_version.to_le_bytes());
    hash_string(&mut hasher, &attestation.algorithm);
    hasher.update(attestation.authority_epoch.to_le_bytes());
    hasher.update(attestation.sequence.to_le_bytes());
    hash_string(&mut hasher, &attestation.namespace);
    hasher.update(attestation.seed.to_le_bytes());
    hasher.update(attestation.event_count.to_le_bytes());
    hash_string(&mut hasher, &attestation.head_hash);
    hash_string(&mut hasher, &attestation.trust_commitment);
    hasher.finalize().into()
}

fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hash_bytes(hasher, value.as_bytes());
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn hex_decode_exact<const N: usize>(value: &str) -> Result<[u8; N], String> {
    if value.len() != N * 2 {
        return Err(format!("expected {N}-byte lowercase hexadecimal value"));
    }
    let bytes = value.as_bytes();
    let mut output = [0_u8; N];
    for index in 0..N {
        let high = hex_nibble(bytes[index * 2])
            .ok_or_else(|| "invalid lowercase hexadecimal value".to_string())?;
        let low = hex_nibble(bytes[index * 2 + 1])
            .ok_or_else(|| "invalid lowercase hexadecimal value".to_string())?;
        output[index] = (high << 4) | low;
    }
    Ok(output)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::rand::SystemRandom;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn test_signer(key_id: &str, key_epoch: u64) -> DurableExecutionSigner {
        let document = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
            .expect("generate test signing key");
        DurableExecutionSigner::from_pkcs8(key_id, key_epoch, document.as_ref())
            .expect("construct test signer")
    }

    fn configured_adapter(name: &str) -> DurableExecutionAdapter {
        let signer = test_signer("test-key", 1);
        let mut trust = DurableExecutionTrust::new();
        trust
            .trust_signer(&signer, 0, None)
            .expect("trust test signer");

        DurableExecutionAdapter::open(
            store(name),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter")
        .with_trust(trust)
        .with_signer(signer)
    }

    struct TestSecurityMaterial {
        authority_signer: DurableExecutionSigner,
        context: DurableExecutionSecurityContext,
    }

    impl TestSecurityMaterial {
        fn new(adapter: &DurableExecutionAdapter) -> Self {
            let authority_signer = test_signer("freshness-authority", 1);
            let authority = FreshnessAuthority::from_public_key_hex(
                authority_signer.key_id(),
                authority_signer.key_epoch(),
                authority_signer.public_key_hex(),
            )
            .expect("freshness authority");
            let genesis = freshness_test_attestation(
                authority.authority_id(),
                authority.authority_epoch(),
                &authority_signer,
                adapter,
                0,
            );
            let cursor = authority
                .bootstrap_cursor(
                    &genesis,
                    "bootstrap",
                    1,
                    &adapter.load_verified().expect("journal").chain,
                    adapter.trust(),
                )
                .expect("genesis cursor");
            let first = freshness_test_attestation(
                authority.authority_id(),
                authority.authority_epoch(),
                &authority_signer,
                adapter,
                1,
            );
            let context = DurableExecutionSecurityContext::establish(
                adapter,
                authority,
                cursor,
                first,
            )
            .expect("security context");
            Self {
                authority_signer,
                context,
            }
        }

        fn refresh(&mut self, adapter: &DurableExecutionAdapter) {
            let sequence = self
                .context
                .freshness_cursor()
                .last_sequence()
                .checked_add(1)
                .expect("freshness sequence");
            let attestation = freshness_test_attestation(
                self.context.freshness_authority().authority_id(),
                self.context.freshness_authority().authority_epoch(),
                &self.authority_signer,
                adapter,
                sequence,
            );
            self.context.set_freshness_attestation(attestation);
        }
    }

    fn store(name: &str) -> SaveStore {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        SaveStore::open(std::env::temp_dir().join(format!(
            "symtropy-bootstrap-adapter-{name}-{suffix}"
        )))
        .expect("store")
    }

    fn process_and_run() -> (ProductionProcess, ProcessRun) {
        (
            ProductionProcess::new("electrolysis", "regolith", ["oxygen", "metal"], "slag"),
            ProcessRun::new(
                "electrolysis",
                "regolith",
                "feed",
                1_000,
                BTreeMap::from([("metal".to_string(), 720), ("oxygen".to_string(), 180)]),
                100,
                4_000,
            ),
        )
    }

    fn initial_kernel_state() -> (ExecutionBudget, InventoryLedger, EnergyLedger) {
        (
            ExecutionBudget::new(1_000, 4_000),
            InventoryLedger::new(BTreeMap::from([("feed".to_string(), 1_000)])),
            EnergyLedger::new(BTreeMap::from([("bus".to_string(), 4_000)])),
        )
    }

    fn freshness_test_attestation(
        authority_id: &str,
        authority_epoch: u64,
        signer: &DurableExecutionSigner,
        adapter: &DurableExecutionAdapter,
        sequence: u64,
    ) -> ExternalFreshnessAttestation {
        let loaded = adapter.load_verified().expect("verified journal");
        let mut attestation = ExternalFreshnessAttestation {
            schema_version: FRESHNESS_ATTESTATION_SCHEMA_VERSION,
            algorithm: FRESHNESS_ATTESTATION_ALGORITHM.to_string(),
            authority_id: authority_id.to_string(),
            authority_epoch,
            sequence,
            namespace: "bootstrap".to_string(),
            seed: 1,
            event_count: u64::try_from(loaded.chain.events().len()).expect("count"),
            head_hash: loaded.chain.head_hash().to_string(),
            trust_commitment: adapter.trust().commitment(),
            signature: String::new(),
        };
        attestation.signature =
            hex_encode(signer.key_pair.sign(&freshness_attestation_digest(&attestation)).as_ref());
        attestation
    }

    #[test]
    fn freshness_cursor_bootstrap_rejects_non_genesis_attestation() {
        let adapter = configured_adapter("freshness-bootstrap-only");
        let authority_signer = test_signer("freshness-bootstrap-only-authority", 1);
        let authority = FreshnessAuthority::from_public_key_hex(
            authority_signer.key_id(),
            authority_signer.key_epoch(),
            authority_signer.public_key_hex(),
        )
        .expect("authority");

        let mut security = TestSecurityMaterial::new(&adapter);
        security.refresh(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-bootstrap-boundary",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending");
        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &resume_pending_execution("exec-bootstrap-boundary", &budget, &inventory)
                    .expect("activation"),
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        let current = freshness_test_attestation(
            authority.authority_id(),
            authority.authority_epoch(),
            &authority_signer,
            &adapter,
            3,
        );
        assert!(matches!(
            authority.bootstrap_cursor(
                &current,
                "bootstrap",
                1,
                &adapter.load_verified().expect("journal").chain,
                adapter.trust(),
            ),
            Err(AdapterError::WitnessMismatch(message))
                if message.contains("GENESIS")
        ));

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn external_freshness_cursor_rejects_replayed_authority_sequence() {
        let adapter = configured_adapter("freshness-cursor");
        let authority_signer = test_signer("freshness-cursor-authority", 1);
        let authority = FreshnessAuthority::from_public_key_hex(
            authority_signer.key_id(),
            authority_signer.key_epoch(),
            authority_signer.public_key_hex(),
        )
        .expect("authority");

        let witness = adapter.capture_head_witness().expect("head witness");
        let genesis = freshness_test_attestation(
            authority.authority_id(),
            authority.authority_epoch(),
            &authority_signer,
            &adapter,
            0,
        );
        let mut cursor = authority
            .bootstrap_cursor(
                &genesis,
                "bootstrap",
                1,
                &adapter.load_verified().expect("journal").chain,
                adapter.trust(),
            )
            .expect("genesis cursor");
        let attestation = freshness_test_attestation(
            authority.authority_id(),
            authority.authority_epoch(),
            &authority_signer,
            &adapter,
            1,
        );

        assert!(adapter
            .load_verified_with_freshness_cursor(
                &witness,
                &authority,
                &attestation,
                &mut cursor,
            )
            .is_err());

        let mut newer = attestation.clone();
        newer.sequence = newer.sequence.checked_add(1).expect("sequence");
        newer.signature =
            hex_encode(authority_signer.key_pair.sign(&freshness_attestation_digest(&newer)).as_ref());

        assert!(adapter
            .load_verified_with_freshness_cursor(
                &witness,
                &authority,
                &newer,
                &mut cursor,
            )
            .is_ok());
        assert_eq!(cursor.last_sequence(), 2);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn external_freshness_cursor_rejects_older_fork_with_higher_sequence() {
        let adapter = configured_adapter("freshness-cursor-fork");
        let authority_signer = test_signer("freshness-cursor-fork-authority", 1);
        let authority = FreshnessAuthority::from_public_key_hex(
            authority_signer.key_id(),
            authority_signer.key_epoch(),
            authority_signer.public_key_hex(),
        )
        .expect("authority");

        let mut security = TestSecurityMaterial::new(&adapter);
        security.refresh(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = (
            ExecutionBudget::new(2_000, 8_000),
            InventoryLedger::new(BTreeMap::from([
                ("feed".to_string(), 1_000),
                ("feed-2".to_string(), 1_000),
            ])),
            EnergyLedger::new(BTreeMap::from([("bus".to_string(), 8_000)])),
        );
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-cursor-fork",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending");
        security.refresh(&adapter);
        let executable =
            resume_pending_execution("exec-cursor-fork", &budget, &inventory)
                .expect("activation");
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        let genesis = freshness_test_attestation(
            authority.authority_id(),
            authority.authority_epoch(),
            &authority_signer,
            &adapter,
            0,
        );
        let mut cursor = authority
            .bootstrap_cursor(
                &genesis,
                "bootstrap",
                1,
                &EventChain::new("bootstrap", 1),
                adapter.trust(),
            )
            .expect("genesis cursor");

        let retained = freshness_test_attestation(
            authority.authority_id(),
            authority.authority_epoch(),
            &authority_signer,
            &adapter,
            3,
        );
        let full = adapter.load_verified().expect("full journal");
        authority
            .verify_and_advance(
                &mut cursor,
                &retained,
                "bootstrap",
                1,
                &full.chain,
                adapter.trust(),
            )
            .expect("advance cursor to retained full head");
        let fork = EventChain::from_events(
            "bootstrap",
            1,
            full.chain.events()[..1].to_vec(),
        );
        let mut older = ExternalFreshnessAttestation {
            schema_version: FRESHNESS_ATTESTATION_SCHEMA_VERSION,
            algorithm: FRESHNESS_ATTESTATION_ALGORITHM.to_string(),
            authority_id: authority.authority_id().to_string(),
            authority_epoch: authority.authority_epoch(),
            sequence: cursor.last_sequence().checked_add(1).expect("sequence"),
            namespace: "bootstrap".to_string(),
            seed: 1,
            event_count: 1,
            head_hash: fork.head_hash().to_string(),
            trust_commitment: adapter.trust().commitment(),
            signature: String::new(),
        };
        older.signature =
            hex_encode(authority_signer.key_pair.sign(&freshness_attestation_digest(&older)).as_ref());

        assert!(matches!(
            authority.verify_and_advance(
                &mut cursor,
                &older,
                "bootstrap",
                1,
                &fork,
                adapter.trust(),
            ),
            Err(AdapterError::WitnessMismatch(_))
        ));

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn external_freshness_attestation_binds_exact_head_and_distinct_authority() {
        let adapter = configured_adapter("external-freshness");
        let authority_signer = test_signer("freshness-authority", 1);
        let authority = FreshnessAuthority::from_public_key_hex(
            authority_signer.key_id(),
            authority_signer.key_epoch(),
            authority_signer.public_key_hex(),
        )
        .expect("authority");
        let witness = adapter.capture_head_witness().expect("head witness");
        let attestation =
            freshness_test_attestation(authority.authority_id(), authority.authority_epoch(), &authority_signer, &adapter);

        assert!(adapter
            .load_verified_with_freshness(&witness, &authority, &attestation)
            .is_ok());

        let mut bad = attestation.clone();
        bad.head_hash = "00".repeat(32);
        assert!(matches!(
            authority.verify(
                &bad,
                "bootstrap",
                1,
                &adapter.load_verified().expect("journal").chain,
                adapter.trust(),
            ),
            Err(AdapterError::WitnessMismatch(_))
        ));

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn external_freshness_attestation_rejects_bad_signature() {
        let adapter = configured_adapter("freshness-bad-signature");
        let authority_signer = test_signer("freshness-bad-signature-authority", 1);
        let authority = FreshnessAuthority::from_public_key_hex(
            authority_signer.key_id(),
            authority_signer.key_epoch(),
            authority_signer.public_key_hex(),
        )
        .expect("authority");
        let witness = adapter.capture_head_witness().expect("head witness");
        let mut attestation = freshness_test_attestation(
            authority.authority_id(),
            authority.authority_epoch(),
            &authority_signer,
            &adapter,
        );
        attestation.sequence = attestation.sequence.checked_add(1).expect("sequence");

        assert!(matches!(
            authority.verify(
                &attestation,
                "bootstrap",
                1,
                &adapter.load_verified().expect("journal").chain,
                adapter.trust(),
            ),
            Err(AdapterError::WitnessMismatch(_))
        ));
        assert_eq!(witness.event_count(), attestation.event_count);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn external_freshness_authority_cannot_reuse_execution_signing_key() {
        let signer = test_signer("shared-key", 1);
        let mut trust = DurableExecutionTrust::new();
        trust.trust_signer(&signer, 0, None).expect("trust signer");
        let authority = FreshnessAuthority::from_public_key_hex(
            signer.key_id(),
            signer.key_epoch(),
            signer.public_key_hex(),
        )
        .expect("authority");
        let adapter = DurableExecutionAdapter::open(
            store("freshness-key-separation"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter")
        .with_trust(trust);

        let witness = adapter.capture_head_witness().expect("head witness");
        let attestation = ExternalFreshnessAttestation {
            schema_version: FRESHNESS_ATTESTATION_SCHEMA_VERSION,
            algorithm: FRESHNESS_ATTESTATION_ALGORITHM.to_string(),
            authority_id: signer.key_id().to_string(),
            authority_epoch: signer.key_epoch(),
            sequence: 1,
            namespace: "bootstrap".to_string(),
            seed: 1,
            event_count: 0,
            head_hash: "GENESIS".to_string(),
            trust_commitment: adapter.trust().commitment(),
            signature: "00".repeat(64),
        };

        assert!(matches!(
            adapter.load_verified_with_freshness(
                &witness,
                &authority,
                &attestation,
            ),
            Err(AdapterError::WitnessMismatch(_))
        ));

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn retained_head_witness_rejects_rollback_to_older_valid_prefix() {
        let adapter = configured_adapter("rollback-witness");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = (
            ExecutionBudget::new(2_000, 8_000),
            InventoryLedger::new(BTreeMap::from([
                ("feed".to_string(), 1_000),
                ("feed-2".to_string(), 1_000),
            ])),
            EnergyLedger::new(BTreeMap::from([("bus".to_string(), 8_000)])),
        );

        security.refresh(&adapter);
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-witness-a",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("first Pending");
        let executable =
            resume_pending_execution("exec-witness-a", &budget, &inventory)
                .expect("activation");
        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("first commit");

        let witness = adapter
            .capture_head_witness()
            .expect("capture current head witness");

        let full = adapter.load_verified().expect("verified journal");
        let prefix = EventChain::from_events(
            "bootstrap",
            1,
            full.chain.events()[..1].to_vec(),
        );

        assert!(witness
            .verify_against("bootstrap", 1, &prefix, adapter.trust())
            .is_err());

        let first_event = prefix.events()[0].clone();
        fs::write(
            adapter.store().root().join("journal.jsonl"),
            serde_json::to_vec(&first_event).expect("serialize rolled-back prefix"),
        )
        .expect("rollback durable journal to older valid prefix");

        assert!(adapter.load_verified_against(&witness).is_err());

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn lifecycle_mutation_rejects_rollback_below_retained_head_witness() {
        let adapter = configured_adapter("mutation-witness");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-witness-mutation",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");
        let executable =
            resume_pending_execution(receipt.execution_id(), &budget, &inventory)
                .expect("activation");
        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        let retained = security.context.head_witness().clone();
        let full = adapter.load_verified().expect("full journal");
        let prefix = EventChain::from_events(
            "bootstrap",
            1,
            full.chain.events()[..1].to_vec(),
        );
        let first_event = prefix.events()[0].clone();
        fs::write(
            adapter.store().root().join("journal.jsonl"),
            serde_json::to_vec(&first_event).expect("serialize retained prefix"),
        )
        .expect("rollback journal to valid signed prefix");

        let before_budget = budget.clone();
        let before_inventory = inventory.clone();
        let before_energy = energy.clone();

        security.refresh(&adapter);
        let err = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-witness-blocked",
                3,
                20,
                30,
                "bus",
                process_and_run().1,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect_err("retained head witness must block rollback");

        assert!(matches!(err, AdapterError::WitnessMismatch(message) if message.contains("witness")));
        assert_eq!(budget, before_budget);
        assert_eq!(inventory, before_inventory);
        assert_eq!(energy, before_energy);
        assert_eq!(security.context.head_witness(), &retained);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn stale_retained_witness_cannot_authorize_against_a_newer_head() {
        let adapter = configured_adapter("stale-witness");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = (
            ExecutionBudget::new(2_000, 8_000),
            InventoryLedger::new(BTreeMap::from([
                ("feed".to_string(), 1_000),
                ("feed-2".to_string(), 1_000),
            ])),
            EnergyLedger::new(BTreeMap::from([("bus".to_string(), 8_000)])),
        );

        let stale = adapter.capture_head_witness().expect("genesis witness");

        security.refresh(&adapter);
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-stale-witness-seed",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("first authorization");

        let before_budget = budget.clone();
        let before_inventory = inventory.clone();
        let before_energy = energy.clone();

        assert!(
            adapter
                .load_verified_against(&stale)
                .is_ok(),
            "prefix-extension verification is intentionally weaker and remains suitable for audit"
        );
        assert!(matches!(
            adapter.load_verified_at(&stale),
            Err(AdapterError::WitnessMismatch(_))
        ));

        security.refresh(&adapter);
        let err = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-stale-witness-reuse",
                2,
                11,
                21,
                "bus",
                process_and_run().1,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect_err("stale exact-head witness must be rejected");

        assert!(matches!(err, AdapterError::WitnessMismatch(_)));
        assert_eq!(budget, before_budget);
        assert_eq!(inventory, before_inventory);
        assert_eq!(energy, before_energy);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn consumed_freshness_attestation_cannot_be_reused_for_next_transition() {
        let adapter = configured_adapter("freshness-single-use");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = (
            ExecutionBudget::new(2_000, 8_000),
            InventoryLedger::new(BTreeMap::from([
                ("feed".to_string(), 1_000),
                ("feed-2".to_string(), 1_000),
            ])),
            EnergyLedger::new(BTreeMap::from([("bus".to_string(), 8_000)])),
        );

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-freshness-single-use",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");

        let before_budget = budget.clone();
        let before_inventory = inventory.clone();
        let before_energy = energy.clone();
        let executable =
            resume_pending_execution(receipt.execution_id(), &budget, &inventory)
                .expect("activation");

        // Deliberately do not refresh the external attestation. The prior checkpoint
        // was consumed by the durable Pending append.
        let err = adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect_err("freshness attestation must be single-use");

        assert!(matches!(err, AdapterError::WitnessMismatch(message) if message.contains("sequence")));
        assert_eq!(budget, before_budget);
        assert_eq!(inventory, before_inventory);
        assert_eq!(energy, before_energy);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn successful_lifecycle_appends_advance_the_retained_head_witness() {
        let adapter = configured_adapter("witness-advance");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = (
            ExecutionBudget::new(2_000, 8_000),
            InventoryLedger::new(BTreeMap::from([
                ("feed".to_string(), 1_000),
                ("feed-2".to_string(), 1_000),
            ])),
            EnergyLedger::new(BTreeMap::from([("bus".to_string(), 8_000)])),
        );

        let mut head_witness = adapter.capture_head_witness().expect("genesis witness");
        assert_eq!(security.context.head_witness().event_count(), 0);
        assert_eq!(security.context.head_witness().head_hash(), "GENESIS");

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-witness-advance",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");

        assert_eq!(security.context.head_witness().event_count(), 1);
        assert_eq!(
            security.context.head_witness(),
            adapter
                .capture_head_witness()
                .expect("persisted Pending witness")
        );

        let executable =
            resume_pending_execution(receipt.execution_id(), &budget, &inventory)
                .expect("activation");
        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        assert_eq!(security.context.head_witness().event_count(), 2);
        assert_eq!(
            security.context.head_witness(),
            adapter
                .capture_head_witness()
                .expect("persisted terminal witness")
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn head_witness_binds_trust_policy() {
        let adapter = configured_adapter("witness-trust");
        let witness = adapter.capture_head_witness().expect("capture witness");

        let other_signer = test_signer("other-key", 9);
        let mut other_trust = DurableExecutionTrust::new();
        other_trust
            .trust_signer(&other_signer, 0, None)
            .expect("trust other key");

        let loaded = adapter.load_verified().expect("journal");
        assert!(matches!(
            witness.verify_against("bootstrap", 1, &loaded.chain, &other_trust),
            Err(AdapterError::WitnessMismatch(_))
        ));

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn signed_journal_is_accepted_and_exposes_key_epoch() {
        let adapter = configured_adapter("signed");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        security.refresh(&adapter);
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-signed",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("signed Pending");

        let loaded = adapter.load_verified().expect("authenticated journal");
        let auth = loaded.chain.events()[0]
            .payload
            .authentication
            .as_ref()
            .expect("authentication proof");
        assert_eq!(auth.algorithm, EXECUTION_AUTH_ALGORITHM);
        assert_eq!(auth.key_id, "test-key");
        assert_eq!(auth.key_epoch, 1);
        assert_eq!(auth.public_key.len(), 64);
        assert_eq!(auth.signature.len(), 128);
        assert_eq!(auth.signed_digest.len(), 64);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn tampered_payload_with_rehashed_outer_event_fails_signature_verification() {
        let adapter = configured_adapter("auth-tamper");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        security.refresh(&adapter);
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-auth-tamper",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("signed Pending");

        let loaded = adapter.load().expect("journal");
        let mut payload = loaded.chain.events()[0].payload.clone();
        payload.process_definition_commitment = "00".repeat(32);

        let mut bad_chain = EventChain::new("bootstrap", 1);
        bad_chain
            .append(
                1,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                payload,
            )
            .expect("re-hash tampered outer event");

        assert!(
            DurableExecutionAdapter::validate_authenticated_journal(
                &bad_chain,
                "bootstrap",
                1,
                adapter.trust(),
            )
            .is_err()
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn missing_signer_fails_before_live_projection_mutation() {
        let signer = test_signer("missing-signer", 1);
        let mut trust = DurableExecutionTrust::new();
        trust.trust_signer(&signer, 0, None).expect("trust");

        let adapter = DurableExecutionAdapter::open(
            store("missing-signer"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter")
        .with_trust(trust);
        let mut security = TestSecurityMaterial::new(&adapter);

        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();
        let before_budget = budget.clone();
        let before_inventory = inventory.clone();
        let before_energy = energy.clone();

        security.refresh(&adapter);
        assert!(
            adapter
                .authorize_pending(
                    &mut security.context,
                    &process,
                    "exec-missing-signer",
                    1,
                    10,
                    20,
                    "bus",
                    run,
                    &mut budget,
                    &mut inventory,
                    &mut energy,
                )
                .is_err()
        );

        assert_eq!(budget, before_budget);
        assert_eq!(inventory, before_inventory);
        assert_eq!(energy, before_energy);
        assert!(adapter.load().expect("journal").chain.events().is_empty());

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn revoked_key_blocks_terminal_append_without_changing_projection() {
        let signer = test_signer("revoked-key", 7);
        let mut trust = DurableExecutionTrust::new();
        trust
            .trust_signer(&signer, 0, Some(1))
            .expect("trust signer with ordinal revocation");

        let adapter = DurableExecutionAdapter::open(
            store("revoked-key"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter")
        .with_trust(trust)
        .with_signer(signer);
        let mut security = TestSecurityMaterial::new(&adapter);

        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-revoked",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("Pending before revocation");

        let before_budget = budget.clone();
        let before_inventory = inventory.clone();
        let before_energy = energy.clone();

        let executable =
            resume_pending_execution(receipt.execution_id(), &budget, &inventory)
                .expect("activation");

        security.refresh(&adapter);
        assert!(adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .is_err());

        assert_eq!(budget, before_budget);
        assert_eq!(inventory, before_inventory);
        assert_eq!(energy, before_energy);
        assert_eq!(adapter.load().expect("journal").chain.events().len(), 1);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn key_rotation_applies_between_executions_not_mid_lifecycle() {
        let first = test_signer("key-one", 1);
        let second = test_signer("key-two", 2);

        let mut trust = DurableExecutionTrust::new();
        trust.trust_signer(&first, 0, Some(2)).expect("trust first");
        trust.trust_signer(&second, 2, None).expect("trust second");

        let mut adapter = DurableExecutionAdapter::open(
            store("key-rotation"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter")
        .with_trust(trust)
        .with_signer(first);
        let mut security = TestSecurityMaterial::new(&adapter);

        let process =
            ProductionProcess::new("electrolysis", "regolith", ["oxygen", "metal"], "slag");
        let run_one = ProcessRun::new(
            "electrolysis",
            "regolith",
            "feed-one",
            1_000,
            BTreeMap::from([("metal".to_string(), 720), ("oxygen".to_string(), 180)]),
            100,
            4_000,
        );
        let run_two = ProcessRun::new(
            "electrolysis",
            "regolith",
            "feed-two",
            1_000,
            BTreeMap::from([("metal".to_string(), 720), ("oxygen".to_string(), 180)]),
            100,
            4_000,
        );
        let (mut budget, mut inventory, mut energy) = (
            ExecutionBudget::new(2_000, 8_000),
            InventoryLedger::new(BTreeMap::from([
                ("feed-one".to_string(), 1_000),
                ("feed-two".to_string(), 1_000),
            ])),
            EnergyLedger::new(BTreeMap::from([("bus".to_string(), 8_000)])),
        );

        security.refresh(&adapter);
        let receipt_one = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-key-one",
                1,
                10,
                20,
                "bus",
                run_one,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("first Pending");
        let executable_one =
            resume_pending_execution(receipt_one.execution_id(), &budget, &inventory)
                .expect("first activation");
        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable_one,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("first commit");

        adapter.set_signer(second);

        security.refresh(&adapter);
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-key-two",
                3,
                11,
                21,
                "bus",
                run_two,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("second Pending under rotated key");

        let loaded = adapter
            .load_verified()
            .expect("rotated authenticated journal");
        assert_eq!(loaded.chain.events().len(), 3);
        assert_eq!(
            loaded.chain.events()[0]
                .payload
                .authentication
                .as_ref()
                .expect("first auth")
                .key_epoch,
            1
        );
        assert_eq!(
            loaded.chain.events()[2]
                .payload
                .authentication
                .as_ref()
                .expect("second auth")
                .key_epoch,
            2
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn trust_rejects_public_key_aliasing_across_epochs_or_identities() {
        let document = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
            .expect("generate signing key");
        let first = DurableExecutionSigner::from_pkcs8("alias-one", 1, document.as_ref())
            .expect("first signer");
        let second = DurableExecutionSigner::from_pkcs8("alias-two", 2, document.as_ref())
            .expect("second signer");

        let mut trust = DurableExecutionTrust::new();
        trust.trust_signer(&first, 0, None).expect("first trust entry");
        assert!(trust.trust_signer(&second, 1, None).is_err());
    }

    #[test]
    fn terminal_cannot_change_authenticated_signing_authority() {
        let first = test_signer("terminal-key-one", 1);
        let second = test_signer("terminal-key-two", 2);
        let mut trust = DurableExecutionTrust::new();
        trust.trust_signer(&first, 0, None).expect("first trust");
        trust.trust_signer(&second, 0, None).expect("second trust");

        let adapter = DurableExecutionAdapter::open(
            store("terminal-key-substitution"),
            "bootstrap",
            "bootstrap-execution",
            1,
        )
        .expect("adapter")
        .with_trust(trust)
        .with_signer(first);
        let mut security = TestSecurityMaterial::new(&adapter);

        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-key-substitution",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("Pending");

        let executable = resume_pending_execution(receipt.execution_id(), &budget, &inventory)
            .expect("activation");
        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("normal commit");

        let loaded = adapter.load().expect("journal");
        let pending = loaded.chain.events()[0].clone();
        let terminal = loaded.chain.events()[1].clone();
        let mut terminal_payload = terminal.payload;
        terminal_payload.authentication = None;

        let mut bad_chain = EventChain::new("bootstrap", 1);
        bad_chain
            .append(
                1,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                pending.payload,
            )
            .expect("rebuild Pending");

        let event_id = StableId::derive("bootstrap", 1, 1);
        let digest = terminal_payload
            .signing_digest(
                "bootstrap",
                1,
                event_id.as_str(),
                bad_chain.events()[0].event_hash.as_str(),
            )
            .expect("terminal digest");
        terminal_payload.authentication = Some(second.sign_digest(&digest));

        bad_chain
            .append(
                2,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                terminal_payload,
            )
            .expect("rebuild substituted terminal");

        assert!(
            DurableExecutionAdapter::validate_authenticated_journal(
                &bad_chain,
                "bootstrap",
                1,
                adapter.trust(),
            )
            .is_err()
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn adapter_writer_fence_blocks_authorization_without_mutating_live_state() {
        let adapter = configured_adapter("adapter-lock");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();
        let before = (budget.clone(), inventory.clone(), energy.clone());

        let lock = adapter
            .store()
            .acquire_journal_lock()
            .expect("hold writer fence");

        security.refresh(&adapter);
        assert!(
            adapter
                .authorize_pending(
                    &mut security.context,
                    &process,
                    "exec-fenced",
                    1,
                    10,
                    20,
                    "bus",
                    run,
                    &mut budget,
                    &mut inventory,
                    &mut energy,
                )
                .is_err()
        );

        assert_eq!(budget, before.0);
        assert_eq!(inventory, before.1);
        assert_eq!(energy, before.2);
        drop(lock);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn pending_record_is_write_ahead_of_live_projection() {
        let adapter = configured_adapter("pending");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        let pre_budget = budget.state_commitment();
        let pre_inventory = inventory.state_commitment();
        let pre_energy = energy.state_commitment();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-001",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        let journal = adapter.load().expect("journal");
        let event = &journal.chain.events()[0];
        assert_eq!(journal.chain.events().len(), 1);
        assert_eq!(event.previous_hash, "GENESIS");
        assert_eq!(event.payload.receipt_commitment, receipt.commitment());
        assert_eq!(event.payload.pre_budget_commitment, pre_budget);
        assert_eq!(event.payload.pre_inventory_commitment, pre_inventory);
        assert_eq!(event.payload.pre_energy_commitment, pre_energy);
        assert_eq!(
            event.payload.post_budget_commitment,
            budget.state_commitment()
        );
        assert_eq!(
            event.payload.post_inventory_commitment,
            inventory.state_commitment()
        );
        assert_eq!(event.payload.post_energy_commitment, energy.state_commitment());
        assert_eq!(
            budget.execution_state("exec-001"),
            Some(symtropy_bootstrap::ExecutionState::Pending)
        );
        assert!(inventory.events().is_empty());

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn semantic_anchor_tampering_is_rejected_by_valid_outer_chain() {
        let adapter = configured_adapter("anchor");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        security.refresh(&adapter);
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-anchor",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        let loaded = adapter.load().expect("journal");
        let original = loaded.chain.events()[0].clone();
        let mut payload = original.payload.clone();
        payload.receipt.state_anchor.frontier = "wrong-frontier".to_string();
        payload.receipt_commitment = payload
            .receipt
            .to_receipt()
            .expect("tampered receipt should reconstruct")
            .commitment();

        let mut bad_chain = EventChain::new("bootstrap", 1);
        bad_chain
            .append(
                1,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                payload,
            )
            .expect("outer chain can hash tampered payload");

        assert!(DurableExecutionAdapter::validate_journal(&bad_chain).is_err());
        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn terminal_pre_state_must_equal_pending_post_state() {
        let adapter = configured_adapter("continuity");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        security.refresh(&adapter);
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-continuity",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        let loaded = adapter.load().expect("journal");
        let pending = loaded.chain.events()[0].clone();
        let persisted = pending.payload.receipt.to_receipt().expect("receipt");
        let terminal_payload = ExecutionLifecycleEvent::terminal(
            DurableExecutionState::Aborted,
            &persisted,
            pending.payload.process_definition_commitment.clone(),
            pending.event_id.to_string(),
            pending.event_hash.clone(),
            ExecutionBudget::new(999, 3_999).state_commitment(),
            pending.payload.post_inventory_commitment.clone(),
            pending.payload.post_energy_commitment.clone(),
            budget.state_commitment(),
            inventory.state_commitment(),
            energy.state_commitment(),
        );

        let mut bad_chain = loaded.chain.clone();
        bad_chain
            .append(
                2,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                terminal_payload,
            )
            .expect("outer chain can hash terminal mismatch");

        assert!(DurableExecutionAdapter::validate_journal(&bad_chain).is_err());
        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn recover_pending_from_pre_state_rehydrates_without_new_identity() {
        let adapter = configured_adapter("recover-pending");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();
        let pre_budget = budget.clone();
        let pre_inventory = inventory.clone();
        let pre_energy = energy.clone();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-recover-pending",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        budget = pre_budget;
        inventory = pre_inventory;
        let mut energy = pre_energy;

        security.refresh(&adapter);
        let executable = adapter
            .recover_pending(
                &mut security.context,
                &process,
                "exec-recover-pending",
                receipt.state_anchor(),
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending recovery");

        assert_eq!(executable.execution_id(), "exec-recover-pending");
        assert_eq!(
            budget.execution_state("exec-recover-pending"),
            Some(symtropy_bootstrap::ExecutionState::Pending)
        );
        assert!(!inventory.source_reservations.is_empty());

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn commit_records_exact_causal_chain_and_post_state() {
        let adapter = configured_adapter("commit");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-commit",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");
        let executable = resume_pending_execution(receipt.execution_id(), &budget, &inventory)
            .expect("activation");

        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        let journal = adapter.load().expect("journal");
        assert_eq!(journal.chain.events().len(), 2);
        let pending = &journal.chain.events()[0];
        let terminal = &journal.chain.events()[1];
        assert_eq!(terminal.payload.state, DurableExecutionState::Committed);
        assert_eq!(
            terminal.payload.pending_event_id.as_deref(),
            Some(pending.event_id.to_string().as_str())
        );
        assert_eq!(
            terminal.payload.pending_event_hash.as_deref(),
            Some(pending.event_hash.as_str())
        );
        assert_eq!(
            terminal.payload.causal_inventory_event_ids,
            receipt.inventory_event_ids()
        );
        assert_eq!(
            terminal.payload.causal_energy_event_id.as_deref(),
            Some(receipt.energy_event_id().as_str())
        );
        assert_eq!(
            terminal.payload.pre_budget_commitment,
            pending.payload.post_budget_commitment
        );
        assert_eq!(
            terminal.payload.pre_inventory_commitment,
            pending.payload.post_inventory_commitment
        );
        assert_eq!(
            terminal.payload.pre_energy_commitment,
            pending.payload.post_energy_commitment
        );
        assert_eq!(
            terminal.payload.post_budget_commitment,
            budget.state_commitment()
        );
        assert_eq!(
            terminal.payload.post_inventory_commitment,
            inventory.state_commitment()
        );
        assert_eq!(
            terminal.payload.post_energy_commitment,
            energy.state_commitment()
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn recover_terminal_from_pre_state_replays_exact_terminal_result() {
        let adapter = configured_adapter("recover-terminal");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-recover-terminal",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");
        let executable = resume_pending_execution(receipt.execution_id(), &budget, &inventory)
            .expect("activation");

        let terminal_pre_budget = budget.clone();
        let terminal_pre_inventory = inventory.clone();
        let terminal_pre_energy = energy.clone();

        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        let durable_post_budget = budget.state_commitment();
        let durable_post_inventory = inventory.state_commitment();
        let durable_post_energy = energy.state_commitment();

        budget = terminal_pre_budget;
        inventory = terminal_pre_inventory;
        energy = terminal_pre_energy;

        security.refresh(&adapter);
        let result = adapter
            .recover_terminal(
                &mut security.context,
                &process,
                "exec-recover-terminal",
                receipt.state_anchor(),
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("terminal recovery");

        assert_eq!(result, RecoveryResult::Committed);
        assert_eq!(budget.state_commitment(), durable_post_budget);
        assert_eq!(inventory.state_commitment(), durable_post_inventory);
        assert_eq!(energy.state_commitment(), durable_post_energy);

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn duplicate_terminal_is_rejected() {
        let adapter = configured_adapter("duplicate-terminal");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-duplicate-terminal",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");
        let executable = resume_pending_execution(receipt.execution_id(), &budget, &inventory)
            .expect("activation");
        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        let loaded = adapter.load().expect("journal");
        let terminal_payload = loaded.chain.events()[1].payload.clone();
        let mut bad_chain = loaded.chain.clone();
        bad_chain
            .append(
                3,
                EXECUTION_EVENT_KIND,
                None,
                None,
                Vec::new(),
                terminal_payload,
            )
            .expect("outer chain can encode duplicate terminal");

        assert!(DurableExecutionAdapter::validate_journal(&bad_chain).is_err());
        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn authorizing_second_pending_execution_is_rejected() {
        let adapter = configured_adapter("single-flight");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        security.refresh(&adapter);
        adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-first",
                1,
                10,
                20,
                "bus",
                run.clone(),
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("first pending authorization");

        security.refresh(&adapter);
        assert!(
            adapter
                .authorize_pending(
                    &mut security.context,
                    &process,
                    "exec-second",
                    2,
                    20,
                    30,
                    "bus",
                    run,
                    &mut budget,
                    &mut inventory,
                    &energy,
                )
                .is_err()
        );

        assert_eq!(
            budget.execution_state("exec-first"),
            Some(symtropy_bootstrap::ExecutionState::Pending)
        );
        assert!(budget.execution_state("exec-second").is_none());

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn changed_process_definition_is_rejected_during_recovery() {
        let adapter = configured_adapter("process-definition");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let changed_process =
            ProductionProcess::new("electrolysis", "regolith", ["metal", "oxygen"], "slag");
        let (mut budget, mut inventory, energy) = initial_kernel_state();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-definition",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &energy,
            )
            .expect("pending authorization");

        assert_ne!(process.commitment(), changed_process.commitment());
        security.refresh(&adapter);
        assert!(
            adapter
                .recover_pending(
                    &mut security.context,
                    &changed_process,
                    "exec-definition",
                    receipt.state_anchor(),
                    &mut budget,
                    &mut inventory,
                    &energy,
                )
                .is_err()
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }

    #[test]
    fn live_projection_mismatch_is_rejected_before_new_pending_authorization() {
        let adapter = configured_adapter("projection-mismatch");
        let mut security = TestSecurityMaterial::new(&adapter);
        let (process, run) = process_and_run();
        let (mut budget, mut inventory, mut energy) = initial_kernel_state();

        security.refresh(&adapter);
        let receipt = adapter
            .authorize_pending(
                &mut security.context,
                &process,
                "exec-projection",
                1,
                10,
                20,
                "bus",
                run,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("pending authorization");
        let executable = resume_pending_execution("exec-projection", &budget, &inventory)
            .expect("activation");

        security.refresh(&adapter);
        adapter
            .commit(
                &mut security.context,
                &process,
                &executable,
                2,
                &mut budget,
                &mut inventory,
                &mut energy,
            )
            .expect("commit");

        energy
            .append(
                symtropy_bootstrap::EnergyEvent::new(
                    99,
                    "bus",
                    1,
                    symtropy_bootstrap::EnergyEventKind::Generated,
                )
                .with_event_id("unrecorded-energy")
                .with_provenance("projection-test"),
            )
            .expect("mutate live-only energy projection");

        security.refresh(&adapter);
        assert!(
            adapter
                .authorize_pending(
                    &mut security.context,
                    &process,
                    "exec-fork",
                    3,
                    20,
                    30,
                    "bus",
                    process_and_run().1,
                    &mut budget,
                    &mut inventory,
                    &mut energy,
                )
                .is_err()
        );

        fs::remove_dir_all(adapter.store().root()).expect("cleanup");
    }
}
