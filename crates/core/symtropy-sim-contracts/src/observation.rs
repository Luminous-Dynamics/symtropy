// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Portable provenance for one domain-owned observation.
//!
//! This type deliberately contains no environmental value and owns no world
//! state. It says only which authority observed which scope, in which frame and
//! representation, at which simulation instant, and which authoritative state
//! digest backs that observation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    hash_string, hash_typed_digest, AuthorityId, ContractError, DigestAlgorithm,
    ReferenceFrameId, RepresentationId, ScopeId, SimInstant, TypedDigest32,
    SIM_CONTRACT_SCHEMA_VERSION,
};

/// Provenance envelope shared by domain observations without coupling domains
/// to `symtropy-world`, Bevy, Basin, Terrain, or any other state owner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationEvidence {
    pub schema_version: u32,
    pub authority: AuthorityId,
    pub scope: ScopeId,
    pub reference_frame: ReferenceFrameId,
    pub representation: RepresentationId,
    pub observed_at: SimInstant,
    pub state_digest: TypedDigest32,
}

impl ObservationEvidence {
    pub fn new(
        authority: AuthorityId,
        scope: ScopeId,
        reference_frame: ReferenceFrameId,
        representation: RepresentationId,
        observed_at: SimInstant,
        state_digest: TypedDigest32,
    ) -> Result<Self, ContractError> {
        let evidence = Self {
            schema_version: SIM_CONTRACT_SCHEMA_VERSION,
            authority,
            scope,
            reference_frame,
            representation,
            observed_at,
            state_digest,
        };
        evidence.validate()?;
        Ok(evidence)
    }

    pub fn validate(&self) -> Result<(), ContractError> {
        if self.schema_version != SIM_CONTRACT_SCHEMA_VERSION {
            return Err(ContractError::UnsupportedSchema {
                expected: SIM_CONTRACT_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        self.state_digest.validate()
    }

    /// Serializer-independent identity for the observation provenance itself.
    ///
    /// This digest does not hash an observed/cache value because the authoritative
    /// state digest is already the semantic source of truth. Consumers may bind
    /// their own derived values separately when needed.
    pub fn digest(&self) -> Result<TypedDigest32, ContractError> {
        self.validate()?;
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.observation-evidence.v1\0");
        hasher.update(self.schema_version.to_le_bytes());
        hash_string(&mut hasher, self.authority.as_str());
        hash_string(&mut hasher, self.scope.as_str());
        hash_string(&mut hasher, self.reference_frame.as_str());
        hash_string(&mut hasher, self.representation.as_str());
        hasher.update(self.observed_at.seconds_from_genesis.to_le_bytes());
        hasher.update(self.observed_at.nanos.to_le_bytes());
        hash_typed_digest(&mut hasher, &self.state_digest);
        TypedDigest32::new(
            "symtropy.observation-evidence.digest.v1",
            DigestAlgorithm::Sha256,
            1,
            hasher.finalize().into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(scope: &str, state: &[u8]) -> ObservationEvidence {
        ObservationEvidence::new(
            AuthorityId::parse("terrain.authority.v1").unwrap(),
            ScopeId::parse(scope).unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            RepresentationId::parse("terrain.voxel.v2").unwrap(),
            SimInstant::new(42, 7).unwrap(),
            TypedDigest32::sha256("terrain.state.v2", 2, state).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn observation_digest_is_stable() {
        let observation = evidence("sol:earth:firstlight/cell-7", b"state-a");
        assert_eq!(observation.digest().unwrap(), observation.digest().unwrap());
    }

    #[test]
    fn scope_and_state_are_identity_significant() {
        let a = evidence("sol:earth:firstlight/cell-7", b"state-a");
        let different_scope = evidence("sol:earth:firstlight/cell-8", b"state-a");
        let different_state = evidence("sol:earth:firstlight/cell-7", b"state-b");
        assert_ne!(a.digest().unwrap(), different_scope.digest().unwrap());
        assert_ne!(a.digest().unwrap(), different_state.digest().unwrap());
    }

    #[test]
    fn json_round_trip_preserves_observation_identity() {
        let observation = evidence("sol:mars:jezero/sector-17", b"state");
        let json = serde_json::to_string(&observation).unwrap();
        let restored: ObservationEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(observation, restored);
        assert_eq!(observation.digest().unwrap(), restored.digest().unwrap());
    }
}
