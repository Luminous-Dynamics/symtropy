// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Portable provenance for domain observations and deterministic forcing.
//!
//! `ObservationEvidence` is backed by an authority-owned state digest.
//! `DeterministicForcingEvidence` is deliberately different: it binds a pure,
//! reproducible model evaluation to its exact inputs and output, but it does not
//! claim that the model output is persistent world truth.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{
    hash_string, hash_typed_digest, validate_identity, AuthorityId, ContractError,
    DigestAlgorithm, ReferenceFrameId, RepresentationId, ScopeId, SimInstant, TypedDigest32,
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

/// Stable identity for a deterministic model or forcing function.
///
/// A forcing model is not an authority. It may influence an authority through a
/// domain-owned transition, but its output is an input/proposal until an owner
/// commits resulting authoritative state.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ForcingModelId(String);

impl ForcingModelId {
    pub fn parse(value: impl Into<String>) -> Result<Self, ContractError> {
        let value = value.into();
        validate_identity("forcing-model", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Evidence for one deterministic, non-authoritative forcing evaluation.
///
/// `model_contract` identifies the frozen model semantics/configuration contract.
/// `input_digest` binds every input needed to reproduce the evaluation (for
/// example seed, location, epoch/day and parameter set). `output_digest` binds
/// the exact produced sample. Consumers must never substitute this object for
/// `ObservationEvidence`: no authority state is asserted here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeterministicForcingEvidence {
    pub schema_version: u32,
    pub model: ForcingModelId,
    pub scope: ScopeId,
    pub reference_frame: ReferenceFrameId,
    pub evaluated_at: SimInstant,
    pub model_contract: TypedDigest32,
    pub input_digest: TypedDigest32,
    pub output_digest: TypedDigest32,
}

impl DeterministicForcingEvidence {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        model: ForcingModelId,
        scope: ScopeId,
        reference_frame: ReferenceFrameId,
        evaluated_at: SimInstant,
        model_contract: TypedDigest32,
        input_digest: TypedDigest32,
        output_digest: TypedDigest32,
    ) -> Result<Self, ContractError> {
        let evidence = Self {
            schema_version: SIM_CONTRACT_SCHEMA_VERSION,
            model,
            scope,
            reference_frame,
            evaluated_at,
            model_contract,
            input_digest,
            output_digest,
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
        self.model_contract.validate()?;
        self.input_digest.validate()?;
        self.output_digest.validate()?;
        Ok(())
    }

    /// Serializer-independent identity of one deterministic forcing evaluation.
    pub fn digest(&self) -> Result<TypedDigest32, ContractError> {
        self.validate()?;
        let mut hasher = Sha256::new();
        hasher.update(b"symtropy.deterministic-forcing-evidence.v1\0");
        hasher.update(self.schema_version.to_le_bytes());
        hash_string(&mut hasher, self.model.as_str());
        hash_string(&mut hasher, self.scope.as_str());
        hash_string(&mut hasher, self.reference_frame.as_str());
        hasher.update(self.evaluated_at.seconds_from_genesis.to_le_bytes());
        hasher.update(self.evaluated_at.nanos.to_le_bytes());
        hash_typed_digest(&mut hasher, &self.model_contract);
        hash_typed_digest(&mut hasher, &self.input_digest);
        hash_typed_digest(&mut hasher, &self.output_digest);
        TypedDigest32::new(
            "symtropy.deterministic-forcing-evidence.digest.v1",
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

    fn forcing(input: &[u8], output: &[u8]) -> DeterministicForcingEvidence {
        DeterministicForcingEvidence::new(
            ForcingModelId::parse("weather.surface-field.v1").unwrap(),
            ScopeId::parse("sol:earth:firstlight/cell-7").unwrap(),
            ReferenceFrameId::parse("sol:earth:surface-fixed").unwrap(),
            SimInstant::new(86_400, 0).unwrap(),
            TypedDigest32::sha256("weather.surface-field.contract.v1", 1, b"model-v1").unwrap(),
            TypedDigest32::sha256("weather.surface-field.inputs.v1", 1, input).unwrap(),
            TypedDigest32::sha256("weather.surface-field.output.v1", 1, output).unwrap(),
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

    #[test]
    fn forcing_digest_is_stable_and_input_sensitive() {
        let a = forcing(b"seed=7;x=3;z=9;day=1", b"rain=12mm");
        let same = forcing(b"seed=7;x=3;z=9;day=1", b"rain=12mm");
        let changed_input = forcing(b"seed=7;x=3;z=9;day=2", b"rain=12mm");
        assert_eq!(a.digest().unwrap(), same.digest().unwrap());
        assert_ne!(a.digest().unwrap(), changed_input.digest().unwrap());
    }

    #[test]
    fn forcing_digest_is_output_sensitive() {
        let dry = forcing(b"seed=7;x=3;z=9;day=1", b"rain=0mm");
        let wet = forcing(b"seed=7;x=3;z=9;day=1", b"rain=12mm");
        assert_ne!(dry.digest().unwrap(), wet.digest().unwrap());
    }

    #[test]
    fn forcing_round_trip_preserves_identity() {
        let forcing = forcing(b"input", b"output");
        let json = serde_json::to_string(&forcing).unwrap();
        let restored: DeterministicForcingEvidence = serde_json::from_str(&json).unwrap();
        assert_eq!(forcing, restored);
        assert_eq!(forcing.digest().unwrap(), restored.digest().unwrap());
    }

    #[test]
    fn invalid_forcing_model_identity_is_rejected() {
        assert!(ForcingModelId::parse("weather model with spaces").is_err());
    }
}
