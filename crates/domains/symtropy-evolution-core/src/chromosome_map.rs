use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    ChromosomeId, ChromosomeMapId, EvolutionError, HereditarySchema, HereditarySchemaDigest,
    HereditarySchemaId, LocusId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub const CHROMOSOME_MAP_VERSION: u32 = 1;
const CHROMOSOME_MAP_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:chromosome-map:v1\0";

/// Integer genetic-map position measured in micromorgans.
///
/// This is deliberately a distinct type so physical sequence coordinates or
/// other distance units cannot be passed accidentally to chromosome-map APIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GeneticMapPositionMicromorgans(u64);

impl GeneticMapPositionMicromorgans {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One hereditary-schema locus placed on a chromosome genetic map.
///
/// The position is a genetic-map coordinate. It is not a physical base-pair
/// coordinate and V1 does not itself define a mapping from map distance to
/// crossover probability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChromosomeLocus {
    pub locus_id: LocusId,
    pub position: GeneticMapPositionMicromorgans,
}

impl ChromosomeLocus {
    pub fn new(locus_id: LocusId, position: GeneticMapPositionMicromorgans) -> Self {
        Self { locus_id, position }
    }
}

/// Ordered locus membership for one chromosome.
///
/// Locus vector order is biological authority and is never sorted by the
/// constructor. Positions must already be strictly increasing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChromosomeDefinition {
    pub id: ChromosomeId,
    pub loci: Vec<ChromosomeLocus>,
}

impl ChromosomeDefinition {
    pub fn new(id: ChromosomeId, loci: Vec<ChromosomeLocus>) -> Result<Self, EvolutionError> {
        let value = Self { id, loci };
        value.validate_local()?;
        Ok(value)
    }

    pub fn validate_local(&self) -> Result<(), EvolutionError> {
        if self.loci.is_empty() {
            return Err(EvolutionError::EmptyChromosome {
                chromosome: self.id.clone(),
            });
        }

        let mut seen = BTreeSet::new();
        let mut previous_position = None;
        for locus in &self.loci {
            if !seen.insert(locus.locus_id.clone()) {
                return Err(EvolutionError::DuplicateChromosomeLocus {
                    locus: locus.locus_id.clone(),
                });
            }
            let position = locus.position.get();
            if let Some(previous) = previous_position {
                if position <= previous {
                    return Err(EvolutionError::NonIncreasingChromosomeMapPosition {
                        chromosome: self.id.clone(),
                        previous_micromorgans: previous,
                        observed_micromorgans: position,
                    });
                }
            }
            previous_position = Some(position);
        }
        Ok(())
    }
}

/// Optional exact chromosome/linkage authority layered over `HereditarySchema`.
///
/// The base hereditary schema remains valid without this sidecar. Supplying a
/// chromosome map permits claims about chromosome membership/order only after
/// this object has been revalidated against the exact current schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChromosomeMap {
    pub map_version: u32,
    pub id: ChromosomeMapId,
    pub schema_id: HereditarySchemaId,
    pub schema_digest: HereditarySchemaDigest,
    pub chromosomes: BTreeMap<ChromosomeId, ChromosomeDefinition>,
}

impl ChromosomeMap {
    pub fn new(
        id: ChromosomeMapId,
        schema: &HereditarySchema,
        chromosomes: impl IntoIterator<Item = ChromosomeDefinition>,
    ) -> Result<Self, EvolutionError> {
        let mut by_id = BTreeMap::new();
        for chromosome in chromosomes {
            chromosome.validate_local()?;
            let chromosome_id = chromosome.id.clone();
            if by_id.insert(chromosome_id.clone(), chromosome).is_some() {
                return Err(EvolutionError::DuplicateChromosomeIdentity {
                    chromosome: chromosome_id,
                });
            }
        }

        let value = Self {
            map_version: CHROMOSOME_MAP_VERSION,
            id,
            schema_id: schema.id.clone(),
            schema_digest: schema.canonical_digest()?,
            chromosomes: by_id,
        };
        value.validate(schema)?;
        Ok(value)
    }

    /// Validate both local canonical representation and exact schema authority.
    pub fn validate(&self, schema: &HereditarySchema) -> Result<(), EvolutionError> {
        schema.validate()?;
        if self.map_version != CHROMOSOME_MAP_VERSION {
            return Err(EvolutionError::UnsupportedChromosomeMap(self.map_version));
        }
        if self.schema_id != schema.id || self.schema_digest != schema.canonical_digest()? {
            return Err(EvolutionError::ChromosomeMapSchemaAuthorityMismatch);
        }
        if self.chromosomes.is_empty() {
            return Err(EvolutionError::NoChromosomes);
        }

        let mut observed_loci = BTreeSet::new();
        for (key, chromosome) in &self.chromosomes {
            if key != &chromosome.id {
                return Err(EvolutionError::ChromosomeKeyMismatch {
                    key: key.clone(),
                    value: chromosome.id.clone(),
                });
            }
            chromosome.validate_local()?;
            for locus in &chromosome.loci {
                if !schema.loci.contains_key(&locus.locus_id) {
                    return Err(EvolutionError::UnknownChromosomeLocus {
                        locus: locus.locus_id.clone(),
                    });
                }
                if !observed_loci.insert(locus.locus_id.clone()) {
                    return Err(EvolutionError::DuplicateChromosomeLocus {
                        locus: locus.locus_id.clone(),
                    });
                }
            }
        }

        if observed_loci.len() != schema.loci.len() {
            if let Some(missing) = schema
                .loci
                .keys()
                .find(|locus_id| !observed_loci.contains(*locus_id))
            {
                return Err(EvolutionError::MissingChromosomeLocus {
                    locus: missing.clone(),
                });
            }
            return Err(EvolutionError::ChromosomeMapLocusSetMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
    ) -> Result<ChromosomeMapDigest, EvolutionError> {
        self.validate(schema)?;
        let mut digest = Sha256::new();
        digest.update(CHROMOSOME_MAP_DIGEST_DOMAIN);
        put_u32(&mut digest, self.map_version);
        put_text(&mut digest, self.id.as_str());
        put_text(&mut digest, self.schema_id.as_str());
        digest.update(self.schema_digest.as_bytes());
        put_u64(&mut digest, self.chromosomes.len() as u64);
        for (chromosome_id, chromosome) in &self.chromosomes {
            put_text(&mut digest, chromosome_id.as_str());
            put_u64(&mut digest, chromosome.loci.len() as u64);
            for locus in &chromosome.loci {
                put_text(&mut digest, locus.locus_id.as_str());
                put_u64(&mut digest, locus.position.get());
            }
        }
        Ok(ChromosomeMapDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChromosomeMapDigest([u8; 32]);

impl ChromosomeMapDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ChromosomeMapDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChromosomeMapDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ChromosomeMapDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}
