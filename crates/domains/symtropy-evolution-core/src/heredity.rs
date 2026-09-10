use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    AlleleId, EvolutionError, HereditarySchema, HereditarySchemaDigest,
    HereditarySchemaId, LocusId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const STATE_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:hereditary-state:v1\0";

/// Exact V0 hereditary content for one biological individual/propagule.
///
/// Allele copies at each locus are **unphased** in V0, so copy-vector order has
/// no biological meaning and is canonicalized lexicographically. Chromosome
/// phase/haplotype order belongs to the later explicit linkage model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HereditaryState {
    pub schema_id: HereditarySchemaId,
    pub schema_digest: HereditarySchemaDigest,
    pub copies: BTreeMap<LocusId, Vec<AlleleId>>,
}

impl HereditaryState {
    pub fn new(
        schema: &HereditarySchema,
        mut copies: BTreeMap<LocusId, Vec<AlleleId>>,
    ) -> Result<Self, EvolutionError> {
        for locus_copies in copies.values_mut() {
            locus_copies.sort();
        }
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
            if copies.windows(2).any(|window| window[0] > window[1]) {
                return Err(EvolutionError::NonCanonicalAlleleCopyOrder {
                    locus: locus_id.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AlleleId, HereditarySchemaId, LocusDefinition};

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).unwrap()
    }

    fn schema() -> HereditarySchema {
        HereditarySchema::new(
            HereditarySchemaId::new("unphased-v0").unwrap(),
            2,
            vec![LocusDefinition::new(
                LocusId::new("pigment").unwrap(),
                [allele("dark"), allele("light")],
            )
            .unwrap()],
        )
        .unwrap()
    }

    #[test]
    fn constructor_canonicalizes_unphased_copy_order() {
        let schema = schema();
        let a = HereditaryState::new(
            &schema,
            BTreeMap::from([(
                LocusId::new("pigment").unwrap(),
                vec![allele("light"), allele("dark")],
            )]),
        )
        .unwrap();
        let b = HereditaryState::new(
            &schema,
            BTreeMap::from([(
                LocusId::new("pigment").unwrap(),
                vec![allele("dark"), allele("light")],
            )]),
        )
        .unwrap();

        assert_eq!(a, b);
        assert_eq!(a.canonical_digest(&schema).unwrap(), b.canonical_digest(&schema).unwrap());
    }

    #[test]
    fn raw_noncanonical_copy_order_cannot_become_authority() {
        let schema = schema();
        let state = HereditaryState {
            schema_id: schema.id.clone(),
            schema_digest: schema.canonical_digest().unwrap(),
            copies: BTreeMap::from([(
                LocusId::new("pigment").unwrap(),
                vec![allele("light"), allele("dark")],
            )]),
        };

        assert!(matches!(
            state.validate(&schema),
            Err(EvolutionError::NonCanonicalAlleleCopyOrder { .. })
        ));
    }
}
