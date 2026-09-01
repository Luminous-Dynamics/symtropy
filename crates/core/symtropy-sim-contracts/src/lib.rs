// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Dependency-light causal simulation contracts shared across Symtropy domains.
//!
//! This crate deliberately owns no world state. It defines stable identities,
//! a wide-range simulation instant, typed digests, and receipts proving that an
//! authority changed representation without silently changing causal truth.

use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub mod continuation;
pub mod observation;
pub use continuation::{
    ChildManifestRef, ContinuationError, ContinuationRequirement, DomainContinuationEntry,
    FixedTimebase, LifecycleMode, ResumeIdentityClass, WorldContinuationManifest,
    FIXED_TIMEBASE_SCHEMA_VERSION, WORLD_CONTINUATION_MANIFEST_SCHEMA_VERSION,
};
pub use observation::{DeterministicForcingEvidence, ForcingModelId, ObservationEvidence};

pub const SIM_CONTRACT_SCHEMA_VERSION: u32 = 1;
pub const NANOS_PER_SECOND: u32 = 1_000_000_000;
pub const MAX_CAUSAL_PARENTS: usize = 64;
const MAX_ID_LEN: usize = 160;
const MAX_DOMAIN_LEN: usize = 160;

macro_rules! define_id {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn parse(value: impl Into<String>) -> Result<Self, ContractError> {
                let value = value.into();
                validate_identity($kind, &value)?;
                Ok(Self(value))
            }

            pub fn validate(&self) -> Result<(), ContractError> {
                validate_identity($kind, &self.0)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

define_id!(AuthorityId, "authority");
define_id!(ScopeId, "scope");
define_id!(ReferenceFrameId, "reference-frame");
define_id!(RepresentationId, "representation");
define_id!(WorldInstanceId, "world-instance");
define_id!(TimebaseId, "timebase");
define_id!(SnapshotCodecId, "snapshot-codec");

/// Absolute simulation coordinate used across gameplay, planetary, and
/// geological timescales. Wall-clock time is never implied by this value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct SimInstant {
    pub seconds_from_genesis: i64,
    pub nanos: u32,
}

impl SimInstant {
    pub const GENESIS: Self = Self {
        seconds_from_genesis: 0,
        nanos: 0,
    };

    pub fn new(seconds_from_genesis: i64, nanos: u32) -> Result<Self, ContractError> {
        let instant = Self {
            seconds_from_genesis,
            nanos,
        };
        instant.validate()?;
        Ok(instant)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.nanos >= NANOS_PER_SECOND {
            return Err(ContractError::InvalidNanoseconds(self.nanos));
        }
        Ok(())
    }

    /// Adds a signed nanosecond delta while preserving canonical
    /// `0..NANOS_PER_SECOND` sub-second representation.
    pub fn checked_add_nanoseconds(self, delta: i64) -> Result<Self, ContractError> {
        let total = i128::from(self.seconds_from_genesis)
            .checked_mul(i128::from(NANOS_PER_SECOND))
            .and_then(|value| value.checked_add(i128::from(self.nanos)))
            .and_then(|value| value.checked_add(i128::from(delta)))
            .ok_or(ContractError::TimeOverflow)?;

        let seconds = total.div_euclid(i128::from(NANOS_PER_SECOND));
        let nanos = total.rem_euclid(i128::from(NANOS_PER_SECOND));
        let seconds_from_genesis = i64::try_from(seconds).map_err(|_| ContractError::TimeOverflow)?;
        let nanos = u32::try_from(nanos).map_err(|_| ContractError::TimeOverflow)?;
        Self::new(seconds_from_genesis, nanos)
    }

    pub fn nanoseconds_since(self, earlier: Self) -> i128 {
        let seconds = i128::from(self.seconds_from_genesis)
            - i128::from(earlier.seconds_from_genesis);
        seconds * i128::from(NANOS_PER_SECOND)
            + i128::from(self.nanos)
            - i128::from(earlier.nanos)
    }
}

impl<'de> Deserialize<'de> for SimInstant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SimInstantRepr {
            seconds_from_genesis: i64,
            nanos: u32,
        }

        let repr = SimInstantRepr::deserialize(deserializer)?;
        Self::new(repr.seconds_from_genesis, repr.nanos).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DigestAlgorithm {
    Sha256,
    Other(String),
}

impl DigestAlgorithm {
    fn validate(&self) -> Result<(), ContractError> {
        match self {
            Self::Sha256 => Ok(()),
            Self::Other(name) => validate_domain("digest-algorithm", name),
        }
    }

    fn stable_code(&self) -> u8 {
        match self {
            Self::Sha256 => 0,
            Self::Other(_) => 255,
        }
    }
}

/// A digest is meaningful only together with its semantic domain, algorithm,
/// and schema version. Equal bytes in different domains are not equal claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypedDigest32 {
    pub domain: String,
    pub algorithm: DigestAlgorithm,
    pub schema_version: u32,
    pub value: [u8; 32],
}

impl TypedDigest32 {
    pub fn new(
        domain: impl Into<String>,
        algorithm: DigestAlgorithm,
        schema_version: u32,
        value: [u8; 32],
    ) -> Result<Self, ContractError> {
        let digest = Self {
            domain: domain.into(),
            algorithm,
            schema_version,
            value,
        };
        digest.validate()?;
        Ok(digest)
    }

    pub fn sha256(
        domain: impl Into<String>,
        schema_version: u32,
        bytes: &[u8],
    ) -> Result<Self, ContractError> {
        Self::new(
            domain,
            DigestAlgorithm::Sha256,
            schema_version,
            Sha256::digest(bytes).into(),
        )
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        validate_domain("digest-domain", &self.domain)?;
        self.algorithm.validate()?;
        if self.schema_version == 0 {
            return Err(ContractError::InvalidSchemaVersion);
        }
        Ok(())
    }

    pub fn same_typed_value(&self, other: &Self) -> bool {
        self.domain == other.domain
            && self.algorithm == other.algorithm
            && self.schema_version == other.schema_version
            && self.value == other.value
    }
}

/// Evidence that one authority re-expressed one scoped state at another
/// fidelity/representation. The receipt does not define domain conservation;
/// `conservation_proof` is produced and verified by the owning domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepresentationTransferReceipt {
    pub schema_version: u32,
    pub authority: AuthorityId,
    pub scope: ScopeId,
    pub reference_frame: ReferenceFrameId,
    pub from_representation: RepresentationId,
    pub to_representation: RepresentationId,
    pub at: SimInstant,
    pub source_state: TypedDigest32,
    pub target_state: TypedDigest32,
    pub conservation_proof: TypedDigest32,
    pub causal_parents: Vec<TypedDigest32>,
}

impl RepresentationTransferReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        authority: AuthorityId,
        scope: ScopeId,
        reference_frame: ReferenceFrameId,
        from_representation: RepresentationId,
        to_representation: RepresentationId,
        at: SimInstant,
        source_state: TypedDigest32,
        target_state: TypedDigest32,
        conservation_proof: TypedDigest32,
        causal_parents: Vec<TypedDigest32>,
    ) -> Result<Self, ContractError> {
        let receipt = Self {
            schema_version: SIM_CONTRACT_SCHEMA_VERSION,
            authority,
            scope,
            reference_frame,
            from_representation,
            to_representation,
            at,
            source_state,
            target_state,
            conservation_proof,
            causal_parents,
        };
        receipt.validate()?;
        Ok(receipt)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.schema_version != SIM_CONTRACT_SCHEMA_VERSION {
            return Err(ContractError::UnsupportedSchema {
                expected: SIM_CONTRACT_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        self.authority.validate()?;
        self.scope.validate()?;
        self.reference_frame.validate()?;
        self.from_representation.validate()?;
        self.to_representation.validate()?;
        self.at.validate()?;
        if self.from_representation == self.to_representation {
            return Err(ContractError::SameRepresentation);
        }
        if self.causal_parents.len() > MAX_CAUSAL_PARENTS {
            return Err(ContractError::TooManyCausalParents {
                maximum: MAX_CAUSAL_PARENTS,
                actual: self.causal_parents.len(),
            });
        }
        self.source_state.validate()?;
        self.target_state.validate()?;
        self.conservation_proof.validate()?;
        for parent in &self.causal_parents {
            parent.validate()?;
        }
        Ok(())
    }

    /// Serializer-independent receipt identity. Every string is length-prefixed
    /// and every integer uses little-endian encoding.
    pub fn digest(&self) -> Result<TypedDigest32, ContractError> {
        self.validate()?;
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.representation-transfer.v1\0");
        hasher.update(self.schema_version.to_le_bytes());
        hash_string(&mut hasher, self.authority.as_str());
        hash_string(&mut hasher, self.scope.as_str());
        hash_string(&mut hasher, self.reference_frame.as_str());
        hash_string(&mut hasher, self.from_representation.as_str());
        hash_string(&mut hasher, self.to_representation.as_str());
        hasher.update(self.at.seconds_from_genesis.to_le_bytes());
        hasher.update(self.at.nanos.to_le_bytes());
        hash_typed_digest(&mut hasher, &self.source_state);
        hash_typed_digest(&mut hasher, &self.target_state);
        hash_typed_digest(&mut hasher, &self.conservation_proof);
        hasher.update((self.causal_parents.len() as u64).to_le_bytes());
        for parent in &self.causal_parents {
            hash_typed_digest(&mut hasher, parent);
        }
        TypedDigest32::new(
            "symtropy.representation-transfer.receipt.v1",
            DigestAlgorithm::Sha256,
            1,
            hasher.finalize().into(),
        )
    }
}

fn validate_identity(kind: &'static str, value: &str) -> Result<(), ContractError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_ID_LEN
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':' | b'/')
        });
    if valid {
        Ok(())
    } else {
        Err(ContractError::InvalidIdentity {
            kind,
            value: value.to_owned(),
        })
    }
}

fn validate_domain(kind: &'static str, value: &str) -> Result<(), ContractError> {
    let valid = !value.is_empty()
        && value.len() <= MAX_DOMAIN_LEN
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':' | b'/')
        });
    if valid {
        Ok(())
    } else {
        Err(ContractError::InvalidDomain {
            kind,
            value: value.to_owned(),
        })
    }
}

fn hash_string(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn hash_typed_digest(hasher: &mut Sha256, digest: &TypedDigest32) {
    hash_string(hasher, &digest.domain);
    hasher.update([digest.algorithm.stable_code()]);
    if let DigestAlgorithm::Other(name) = &digest.algorithm {
        hash_string(hasher, name);
    }
    hasher.update(digest.schema_version.to_le_bytes());
    hasher.update(digest.value);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContractError {
    InvalidIdentity {
        kind: &'static str,
        value: String,
    },
    InvalidDomain {
        kind: &'static str,
        value: String,
    },
    InvalidSchemaVersion,
    InvalidNanoseconds(u32),
    TimeOverflow,
    UnsupportedSchema {
        expected: u32,
        actual: u32,
    },
    SameRepresentation,
    TooManyCausalParents {
        maximum: usize,
        actual: usize,
    },
}

impl fmt::Display for ContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdentity { kind, value } => {
                write!(formatter, "invalid {kind} identity: {value:?}")
            }
            Self::InvalidDomain { kind, value } => {
                write!(formatter, "invalid {kind}: {value:?}")
            }
            Self::InvalidSchemaVersion => write!(formatter, "schema version must be non-zero"),
            Self::InvalidNanoseconds(value) => write!(
                formatter,
                "nanoseconds must be less than {NANOS_PER_SECOND}, got {value}"
            ),
            Self::TimeOverflow => formatter.write_str("simulation instant overflow"),
            Self::UnsupportedSchema { expected, actual } => write!(
                formatter,
                "unsupported simulation contract schema {actual}; expected {expected}"
            ),
            Self::SameRepresentation => {
                write!(formatter, "representation transfer requires distinct representations")
            }
            Self::TooManyCausalParents { maximum, actual } => write!(
                formatter,
                "representation transfer has {actual} causal parents; maximum is {maximum}"
            ),
        }
    }
}

impl Error for ContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id_set() -> (AuthorityId, ScopeId, ReferenceFrameId) {
        (
            AuthorityId::parse("terrain.authority.v1").unwrap(),
            ScopeId::parse("sol:earth:firstlight-basin").unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
        )
    }

    fn d(domain: &str, bytes: &[u8]) -> TypedDigest32 {
        TypedDigest32::sha256(domain, 1, bytes).unwrap()
    }

    fn receipt(parents: Vec<TypedDigest32>) -> RepresentationTransferReceipt {
        let (authority, scope, reference_frame) = id_set();
        RepresentationTransferReceipt::new(
            authority,
            scope,
            reference_frame,
            RepresentationId::parse("terrain.aggregate.v1").unwrap(),
            RepresentationId::parse("terrain.voxel.v2").unwrap(),
            SimInstant::new(123, 456).unwrap(),
            d("terrain.aggregate.state.v1", b"source"),
            d("terrain.voxel.state.v2", b"target"),
            d("terrain.transfer-proof.v1", b"mass+material close"),
            parents,
        )
        .unwrap()
    }

    #[test]
    fn identities_are_portable_and_whitespace_free() {
        assert!(ScopeId::parse("sol:mars:jezero/sector-17").is_ok());
        assert!(ScopeId::parse("sol:mars:bad sector").is_err());
        assert!(AuthorityId::parse("").is_err());
    }

    #[test]
    fn sim_instant_crosses_genesis_without_noncanonical_nanos() {
        let instant = SimInstant::new(0, 100).unwrap();
        let earlier = instant.checked_add_nanoseconds(-200).unwrap();
        assert_eq!(earlier.seconds_from_genesis, -1);
        assert_eq!(earlier.nanos, 999_999_900);
        assert_eq!(instant.nanoseconds_since(earlier), 200);
    }

    #[test]
    fn sim_instant_deserialization_rejects_noncanonical_nanos() {
        let invalid = r#"{"seconds_from_genesis":0,"nanos":1000000000}"#;
        let result: Result<SimInstant, _> = serde_json::from_str(invalid);
        assert!(result.is_err());

        let boundary = r#"{"seconds_from_genesis":-1,"nanos":999999999}"#;
        let restored: SimInstant = serde_json::from_str(boundary).unwrap();
        assert_eq!(restored, SimInstant::new(-1, 999_999_999).unwrap());
    }

    #[test]
    fn typed_digest_is_domain_separated() {
        let left = d("terrain.state.v1", b"same bytes");
        let right = d("hydrology.state.v1", b"same bytes");
        assert_ne!(left.domain, right.domain);
        assert!(!left.same_typed_value(&right));
    }

    #[test]
    fn same_representation_is_not_a_transfer() {
        let (authority, scope, reference_frame) = id_set();
        let representation = RepresentationId::parse("terrain.voxel.v1").unwrap();
        let result = RepresentationTransferReceipt::new(
            authority,
            scope,
            reference_frame,
            representation.clone(),
            representation,
            SimInstant::GENESIS,
            d("state.v1", b"a"),
            d("state.v1", b"a"),
            d("proof.v1", b"proof"),
            vec![],
        );
        assert_eq!(result.unwrap_err(), ContractError::SameRepresentation);
    }

    #[test]
    fn receipt_digest_is_stable_and_causal_order_sensitive() {
        let a = d("event.v1", b"a");
        let b = d("event.v1", b"b");
        let forward = receipt(vec![a.clone(), b.clone()]);
        let reverse = receipt(vec![b, a]);
        assert_eq!(forward.digest().unwrap(), forward.clone().digest().unwrap());
        assert_ne!(forward.digest().unwrap(), reverse.digest().unwrap());
    }

    #[test]
    fn receipt_json_round_trip_preserves_identity() {
        let receipt = receipt(vec![d("event.v1", b"parent")]);
        let json = serde_json::to_string(&receipt).unwrap();
        let restored: RepresentationTransferReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(receipt, restored);
        assert_eq!(receipt.digest().unwrap(), restored.digest().unwrap());
    }
}
