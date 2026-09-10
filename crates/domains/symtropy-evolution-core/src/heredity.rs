use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    AlleleId, EvolutionError, HereditarySchema, HereditarySchemaDigest,
    HereditarySchemaId, LocusId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const STATE_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:hereditary-state:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HereditaryState {
    pub schema_id: HereditarySchemaId,
    pub schema_digest: HereditarySchemaDigest,
    pub copies: BTreeMap<LocusId, Vec<AlleleId>>,
}

impl HereditaryState {
    pub fn new(
        schema: &HereditarySchema,
        copies: BTreeMap<LocusId, Vec<AlleleId>>,
    ) -> Result<Self, EvolutionError> {
        let value = Self {
            schema_id: schema.id.clone(),
            schema_digest: schema.canonical_digest()?,
            copies,
        };
        value.validate(schema)?;
        Ok(value)
    }

    pub fn validate(&self, schema: &HereditarySchema) -> Result<(), EvolutionError> {
        schema.validate()?;
        if self.schema_id != schema.id {
            return Err(EvolutionError::HereditarySchemaMismatch);
        }
        if self.schema_digest != schema.canonical_digest()? {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if self.copies.len() != schema.loci.len() {
            return Err(EvolutionError::LocusSetMismatch);
        }
        for (locus_id, locus) in &schema.loci {
            let copies = self
                .copies
                .get(locus_id)
                .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
            if copies.len() != usize::from(schema.ploidy) {
                return Err(EvolutionError::CopyCountMismatch {
                    locus: locus_id.clone(),
                    expected: schema.ploidy,
                    observed: copies.len(),
                });
            }
            for allele in copies {
                if !locus.allowed_alleles.contains(allele) {
                    return Err(EvolutionError::UnknownAllele {
                        locus: locus_id.clone(),
                        allele: allele.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
    ) -> Result<HereditaryStateDigest, EvolutionError> {
        self.validate(schema)?;
        let mut digest = Sha256::new();
        digest.update(STATE_DIGEST_DOMAIN);
        put_text(&mut digest, self.schema_id.as_str());
        digest.update(self.schema_digest.as_bytes());
        put_u64(&mut digest, self.copies.len() as u64);
        for (locus, copies) in &self.copies {
            put_text(&mut digest, locus.as_str());
            put_u64(&mut digest, copies.len() as u64);
            for allele in copies {
                put_text(&mut digest, allele.as_str());
            }
        }
        Ok(HereditaryStateDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HereditaryStateDigest([u8; 32]);

impl HereditaryStateDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for HereditaryStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HereditaryStateDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for HereditaryStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}
