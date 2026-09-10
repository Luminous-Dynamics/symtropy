use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AlleleId, ChromosomeId, ChromosomeMap, ChromosomeMapDigest, ChromosomeMapId, EvolutionError,
    HereditarySchema, HereditarySchemaDigest, HereditarySchemaId, HereditaryState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

pub const PHASED_HEREDITARY_STATE_VERSION: u32 = 1;
const PHASED_STATE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:phased-hereditary-state:v1\0";

/// One allele sequence across the mapped loci of a single chromosome homolog.
///
/// Allele vector order follows `ChromosomeDefinition::loci` exactly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChromosomeHaplotype {
    pub alleles: Vec<AlleleId>,
}

impl ChromosomeHaplotype {
    pub fn new(alleles: Vec<AlleleId>) -> Self {
        Self { alleles }
    }
}

/// Phased state for one chromosome.
///
/// Whole haplotypes are an unlabeled multiset in C2. The constructor sorts
/// whole rows lexicographically; it never sorts alleles within a haplotype.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhasedChromosomeState {
    pub id: ChromosomeId,
    pub haplotypes: Vec<ChromosomeHaplotype>,
}

impl PhasedChromosomeState {
    pub fn new(id: ChromosomeId, mut haplotypes: Vec<ChromosomeHaplotype>) -> Self {
        haplotypes.sort();
        Self { id, haplotypes }
    }
}

/// Optional exact phased hereditary state bound to both schema and chromosome map.
///
/// This sidecar does not change the meaning of `HereditaryState`. It can project
/// deterministically to that unphased representation by forgetting homolog
/// associations. The inverse projection is intentionally absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhasedHereditaryState {
    pub state_version: u32,
    pub schema_id: HereditarySchemaId,
    pub schema_digest: HereditarySchemaDigest,
    pub chromosome_map_id: ChromosomeMapId,
    pub chromosome_map_digest: ChromosomeMapDigest,
    pub chromosomes: BTreeMap<ChromosomeId, PhasedChromosomeState>,
}

impl PhasedHereditaryState {
    pub fn new(
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        chromosomes: impl IntoIterator<Item = PhasedChromosomeState>,
    ) -> Result<Self, EvolutionError> {
        chromosome_map.validate(schema)?;
        let mut by_id = BTreeMap::new();
        for mut chromosome in chromosomes {
            chromosome.haplotypes.sort();
            let id = chromosome.id.clone();
            if by_id.insert(id.clone(), chromosome).is_some() {
                return Err(EvolutionError::DuplicateChromosomeIdentity { chromosome: id });
            }
        }
        let value = Self {
            state_version: PHASED_HEREDITARY_STATE_VERSION,
            schema_id: schema.id.clone(),
            schema_digest: schema.canonical_digest()?,
            chromosome_map_id: chromosome_map.id.clone(),
            chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
            chromosomes: by_id,
        };
        value.validate(schema, chromosome_map)?;
        Ok(value)
    }

    pub fn validate(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<(), EvolutionError> {
        schema.validate()?;
        chromosome_map.validate(schema)?;
        if self.state_version != PHASED_HEREDITARY_STATE_VERSION {
            return Err(EvolutionError::UnsupportedPhasedHereditaryState(
                self.state_version,
            ));
        }
        if self.schema_id != schema.id || self.schema_digest != schema.canonical_digest()? {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if self.chromosome_map_id != chromosome_map.id
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
        {
            return Err(EvolutionError::PhasedChromosomeMapAuthorityMismatch);
        }
        if self.chromosomes.len() != chromosome_map.chromosomes.len() {
            return Err(EvolutionError::PhasedChromosomeSetMismatch);
        }

        for (key, chromosome_state) in &self.chromosomes {
            if key != &chromosome_state.id {
                return Err(EvolutionError::PhasedChromosomeKeyMismatch {
                    key: key.clone(),
                    value: chromosome_state.id.clone(),
                });
            }
            if !chromosome_map.chromosomes.contains_key(key) {
                return Err(EvolutionError::PhasedChromosomeSetMismatch);
            }
        }

        for (chromosome_id, definition) in &chromosome_map.chromosomes {
            let chromosome_state = self
                .chromosomes
                .get(chromosome_id)
                .ok_or(EvolutionError::PhasedChromosomeSetMismatch)?;
            if chromosome_state.haplotypes.len() != usize::from(schema.ploidy) {
                return Err(EvolutionError::PhasedHaplotypeCountMismatch {
                    chromosome: chromosome_id.clone(),
                    expected: schema.ploidy,
                    observed: chromosome_state.haplotypes.len(),
                });
            }
            if chromosome_state
                .haplotypes
                .windows(2)
                .any(|window| window[0] > window[1])
            {
                return Err(EvolutionError::NonCanonicalHaplotypeOrder {
                    chromosome: chromosome_id.clone(),
                });
            }

            for haplotype in &chromosome_state.haplotypes {
                if haplotype.alleles.len() != definition.loci.len() {
                    return Err(EvolutionError::PhasedHaplotypeLocusCountMismatch {
                        chromosome: chromosome_id.clone(),
                        expected: definition.loci.len(),
                        observed: haplotype.alleles.len(),
                    });
                }
                for (mapped_locus, allele) in definition.loci.iter().zip(&haplotype.alleles) {
                    let locus = schema
                        .loci
                        .get(&mapped_locus.locus_id)
                        .ok_or_else(|| EvolutionError::MissingLocus(mapped_locus.locus_id.clone()))?;
                    if !locus.allowed_alleles.contains(allele) {
                        return Err(EvolutionError::UnknownAllele {
                            locus: mapped_locus.locus_id.clone(),
                            allele: allele.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Forget phase/homolog association and return the existing unphased state.
    ///
    /// This direction is deterministic and lossless with respect to the old
    /// representation. No inverse API is provided because unphased copies do
    /// not determine haplotype association.
    pub fn to_unphased(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<HereditaryState, EvolutionError> {
        self.validate(schema, chromosome_map)?;
        let mut copies = BTreeMap::new();
        for (chromosome_id, definition) in &chromosome_map.chromosomes {
            let chromosome_state = self
                .chromosomes
                .get(chromosome_id)
                .ok_or(EvolutionError::PhasedChromosomeSetMismatch)?;
            for (locus_index, mapped_locus) in definition.loci.iter().enumerate() {
                let alleles = chromosome_state
                    .haplotypes
                    .iter()
                    .map(|haplotype| haplotype.alleles[locus_index].clone())
                    .collect();
                if copies.insert(mapped_locus.locus_id.clone(), alleles).is_some() {
                    return Err(EvolutionError::DuplicateChromosomeLocus {
                        locus: mapped_locus.locus_id.clone(),
                    });
                }
            }
        }
        HereditaryState::new(schema, copies)
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<PhasedHereditaryStateDigest, EvolutionError> {
        self.validate(schema, chromosome_map)?;
        let mut digest = Sha256::new();
        digest.update(PHASED_STATE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.state_version);
        put_text(&mut digest, self.schema_id.as_str());
        digest.update(self.schema_digest.as_bytes());
        put_text(&mut digest, self.chromosome_map_id.as_str());
        digest.update(self.chromosome_map_digest.as_bytes());
        put_u64(&mut digest, self.chromosomes.len() as u64);
        for (chromosome_id, chromosome_state) in &self.chromosomes {
            put_text(&mut digest, chromosome_id.as_str());
            put_u64(&mut digest, chromosome_state.haplotypes.len() as u64);
            for haplotype in &chromosome_state.haplotypes {
                put_u64(&mut digest, haplotype.alleles.len() as u64);
                for allele in &haplotype.alleles {
                    put_text(&mut digest, allele.as_str());
                }
            }
        }
        Ok(PhasedHereditaryStateDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PhasedHereditaryStateDigest([u8; 32]);

impl PhasedHereditaryStateDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for PhasedHereditaryStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PhasedHereditaryStateDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for PhasedHereditaryStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}
