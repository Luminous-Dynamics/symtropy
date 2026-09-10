use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AlleleId, EvolutionError, HereditarySchemaId, LocusId, EVOLUTION_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fmt};

const SCHEMA_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:heredity-schema:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusDefinition {
    pub id: LocusId,
    pub allowed_alleles: BTreeSet<AlleleId>,
}

impl LocusDefinition {
    pub fn new(
        id: LocusId,
        allowed_alleles: impl IntoIterator<Item = AlleleId>,
    ) -> Result<Self, EvolutionError> {
        let value = Self {
            id,
            allowed_alleles: allowed_alleles.into_iter().collect(),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EvolutionError> {
        if self.allowed_alleles.is_empty() {
            return Err(EvolutionError::NoAlleles {
                locus: self.id.clone(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HereditarySchema {
    pub schema_version: u32,
    pub id: HereditarySchemaId,
    pub ploidy: u8,
    pub loci: BTreeMap<LocusId, LocusDefinition>,
}

impl HereditarySchema {
    pub fn new(
        id: HereditarySchemaId,
        ploidy: u8,
        loci: impl IntoIterator<Item = LocusDefinition>,
    ) -> Result<Self, EvolutionError> {
        let mut by_id = BTreeMap::new();
        for locus in loci {
            if by_id.insert(locus.id.clone(), locus).is_some() {
                return Err(EvolutionError::DuplicateLocus);
            }
        }
        let value = Self {
            schema_version: EVOLUTION_SCHEMA_VERSION,
            id,
            ploidy,
            loci: by_id,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EvolutionError> {
        if self.schema_version != EVOLUTION_SCHEMA_VERSION {
            return Err(EvolutionError::UnsupportedSchema(self.schema_version));
        }
        if !(1..=16).contains(&self.ploidy) {
            return Err(EvolutionError::UnsupportedPloidy(self.ploidy));
        }
        if self.loci.is_empty() {
            return Err(EvolutionError::NoLoci);
        }
        for (key, locus) in &self.loci {
            if key != &locus.id {
                return Err(EvolutionError::LocusKeyMismatch {
                    key: key.clone(),
                    value: locus.id.clone(),
                });
            }
            locus.validate()?;
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<HereditarySchemaDigest, EvolutionError> {
        self.validate()?;
        let mut digest = Sha256::new();
        digest.update(SCHEMA_DIGEST_DOMAIN);
        put_u32(&mut digest, self.schema_version);
        put_text(&mut digest, self.id.as_str());
        digest.update([self.ploidy]);
        put_u64(&mut digest, self.loci.len() as u64);
        for (locus_id, locus) in &self.loci {
            put_text(&mut digest, locus_id.as_str());
            put_u64(&mut digest, locus.allowed_alleles.len() as u64);
            for allele in &locus.allowed_alleles {
                put_text(&mut digest, allele.as_str());
            }
        }
        Ok(HereditarySchemaDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HereditarySchemaDigest([u8; 32]);

impl HereditarySchemaDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for HereditarySchemaDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HereditarySchemaDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for HereditarySchemaDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}
