use crate::{
    canonical::{fmt_hex, put_u32},
    ChromosomeMap, ChromosomeRecombinationProfile, EvolutionError, HereditarySchema, LinkedGamete,
    LinkedGameteDerivation, MarkerMarginalGameteDerivation, ParentRole, PhasedHereditaryState,
    ReproductionEventId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub const LINKED_GAMETE_DERIVATION_EVIDENCE_VERSION: u32 = 1;
const LINKED_GAMETE_DERIVATION_EVIDENCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-gamete-derivation-evidence:v1\0";

/// Closed persisted evidence for the linked-gamete derivation families currently
/// implemented by evolution-core.
///
/// Deserialization restores evidence-shaped data only. Current derivation authority
/// is regained exclusively through `validate_current`, which dispatches to the exact
/// process-specific deterministic replay contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkedGameteDerivationEvidence {
    ZeroCrossover(LinkedGameteDerivation),
    MarkerMarginalPoisson(MarkerMarginalGameteDerivation),
}

impl LinkedGameteDerivationEvidence {
    pub fn gamete(&self) -> &LinkedGamete {
        match self {
            Self::ZeroCrossover(derivation) => &derivation.gamete,
            Self::MarkerMarginalPoisson(derivation) => &derivation.gamete,
        }
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        source: &PhasedHereditaryState,
        profile: &ChromosomeRecombinationProfile,
        event: &ReproductionEventId,
        parent_role: ParentRole,
    ) -> Result<(), EvolutionError> {
        match self {
            Self::ZeroCrossover(derivation) => derivation.provenance.validate_current(
                schema,
                chromosome_map,
                source,
                profile,
                event,
                parent_role,
                &derivation.gamete,
            ),
            Self::MarkerMarginalPoisson(derivation) => derivation.provenance.validate_current(
                schema,
                chromosome_map,
                source,
                profile,
                event,
                parent_role,
                &derivation.gamete,
            ),
        }
    }

    pub fn canonical_digest(&self) -> LinkedGameteDerivationEvidenceDigest {
        let mut digest = Sha256::new();
        digest.update(LINKED_GAMETE_DERIVATION_EVIDENCE_DIGEST_DOMAIN);
        put_u32(&mut digest, LINKED_GAMETE_DERIVATION_EVIDENCE_VERSION);
        match self {
            Self::ZeroCrossover(derivation) => {
                digest.update([0]);
                digest.update(derivation.provenance.canonical_digest().as_bytes());
            }
            Self::MarkerMarginalPoisson(derivation) => {
                digest.update([1]);
                digest.update(derivation.provenance.canonical_digest().as_bytes());
            }
        }
        LinkedGameteDerivationEvidenceDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkedGameteDerivationEvidenceDigest([u8; 32]);

impl LinkedGameteDerivationEvidenceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for LinkedGameteDerivationEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LinkedGameteDerivationEvidenceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for LinkedGameteDerivationEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}
