use crate::{
    canonical::{fmt_hex, put_text, put_u32},
    error::validate_text,
    EvolutionError, OperatorProfileId, PROBABILITY_SCALE_PPM,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

const OPERATOR_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:operator-profile:v1\0";
pub(crate) const MUTATION_RNG_DOMAIN: &[u8] = b"mutation:v1\0";
pub(crate) const RECOMBINATION_RNG_DOMAIN: &[u8] = b"recombination:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationProfile {
    pub model_id: String,
    pub version: String,
    pub per_copy_rate_ppm: u32,
}

impl MutationProfile {
    pub fn validate(&self) -> Result<(), EvolutionError> {
        validate_text("mutation.model_id", &self.model_id)?;
        validate_text("mutation.version", &self.version)?;
        if self.per_copy_rate_ppm > PROBABILITY_SCALE_PPM {
            return Err(EvolutionError::ProbabilityOutOfRange {
                observed_ppm: self.per_copy_rate_ppm,
            });
        }
        Ok(())
    }

    pub(crate) fn put_randomness_identity(&self, digest: &mut Sha256) {
        digest.update(MUTATION_RNG_DOMAIN);
        put_text(digest, &self.model_id);
        put_text(digest, &self.version);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecombinationMode {
    IndependentLoci,
}

impl RecombinationMode {
    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::IndependentLoci => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecombinationProfile {
    pub model_id: String,
    pub version: String,
    pub mode: RecombinationMode,
}

impl RecombinationProfile {
    pub fn validate(&self) -> Result<(), EvolutionError> {
        validate_text("recombination.model_id", &self.model_id)?;
        validate_text("recombination.version", &self.version)?;
        Ok(())
    }

    pub(crate) fn put_randomness_identity(&self, digest: &mut Sha256) {
        digest.update(RECOMBINATION_RNG_DOMAIN);
        put_text(digest, &self.model_id);
        put_text(digest, &self.version);
        digest.update([self.mode.tag()]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionOperatorProfile {
    pub profile_id: OperatorProfileId,
    pub version: String,
    pub mutation: MutationProfile,
    pub recombination: RecombinationProfile,
}

impl EvolutionOperatorProfile {
    pub fn validate(&self) -> Result<(), EvolutionError> {
        validate_text("operator.version", &self.version)?;
        self.mutation.validate()?;
        self.recombination.validate()?;
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<EvolutionOperatorProfileDigest, EvolutionError> {
        self.validate()?;
        let mut digest = Sha256::new();
        digest.update(OPERATOR_DIGEST_DOMAIN);
        put_text(&mut digest, self.profile_id.as_str());
        put_text(&mut digest, &self.version);
        put_text(&mut digest, &self.mutation.model_id);
        put_text(&mut digest, &self.mutation.version);
        put_u32(&mut digest, self.mutation.per_copy_rate_ppm);
        put_text(&mut digest, &self.recombination.model_id);
        put_text(&mut digest, &self.recombination.version);
        digest.update([self.recombination.mode.tag()]);
        Ok(EvolutionOperatorProfileDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvolutionOperatorProfileDigest([u8; 32]);

impl EvolutionOperatorProfileDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for EvolutionOperatorProfileDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvolutionOperatorProfileDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for EvolutionOperatorProfileDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}
