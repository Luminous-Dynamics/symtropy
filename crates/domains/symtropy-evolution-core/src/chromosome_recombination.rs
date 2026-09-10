use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    ChromosomeId, ChromosomeMap, ChromosomeMapDigest, ChromosomeMapId,
    ChromosomeRecombinationProfileId, EvolutionError, GeneticMapPositionMicromorgans,
    HereditarySchema, HereditarySchemaDigest, HereditarySchemaId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

pub const CHROMOSOME_RECOMBINATION_PROFILE_VERSION: u32 = 1;
const RECOMBINATION_PROFILE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:chromosome-recombination-profile:v1\0";

/// Genetic-map process extent for one chromosome.
///
/// The interval is a coordinate extent, not a physical sequence span. C3A
/// permits mapped loci at either endpoint; the future C3B executor must define
/// exact breakpoint endpoint semantics in its own versioned stochastic grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GeneticMapIntervalMicromorgans {
    pub start: GeneticMapPositionMicromorgans,
    pub end: GeneticMapPositionMicromorgans,
}

impl GeneticMapIntervalMicromorgans {
    pub fn new(
        start: GeneticMapPositionMicromorgans,
        end: GeneticMapPositionMicromorgans,
    ) -> Result<Self, EvolutionError> {
        let value = Self { start, end };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EvolutionError> {
        if self.start >= self.end {
            return Err(EvolutionError::OperatorAuthorityMismatch);
        }
        Ok(())
    }

    pub fn length_micromorgans(&self) -> u64 {
        self.end.get() - self.start.get()
    }

    pub fn contains(&self, position: GeneticMapPositionMicromorgans) -> bool {
        self.start <= position && position <= self.end
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChromosomeRecombinationDomain {
    pub chromosome_id: ChromosomeId,
    pub interval: GeneticMapIntervalMicromorgans,
}

impl ChromosomeRecombinationDomain {
    pub fn new(
        chromosome_id: ChromosomeId,
        interval: GeneticMapIntervalMicromorgans,
    ) -> Result<Self, EvolutionError> {
        interval.validate()?;
        Ok(Self {
            chromosome_id,
            interval,
        })
    }

    pub fn validate_local(&self) -> Result<(), EvolutionError> {
        self.interval.validate()
    }
}

/// Explicit crossover process assumed by the first linked-inheritance reference lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChromosomeRecombinationModel {
    /// Gametic crossovers form a homogeneous Poisson point process in genetic-map
    /// distance. This is an explicit no-crossover-interference reference model,
    /// not a universal biological claim.
    PoissonCrossoversNoInterferenceV1,
}

impl ChromosomeRecombinationModel {
    fn tag(self) -> u8 {
        match self {
            Self::PoissonCrossoversNoInterferenceV1 => 0,
        }
    }
}

/// Exact process/applicability authority for future chromosome crossover execution.
///
/// C3A grants no authority that any crossover occurred. It only declares which
/// crossover process C3B may later execute over which chromosome intervals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChromosomeRecombinationProfile {
    pub profile_version: u32,
    pub id: ChromosomeRecombinationProfileId,
    pub schema_id: HereditarySchemaId,
    pub schema_digest: HereditarySchemaDigest,
    pub chromosome_map_id: ChromosomeMapId,
    pub chromosome_map_digest: ChromosomeMapDigest,
    pub model: ChromosomeRecombinationModel,
    pub domains: BTreeMap<ChromosomeId, ChromosomeRecombinationDomain>,
}

impl ChromosomeRecombinationProfile {
    pub fn new(
        id: ChromosomeRecombinationProfileId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        model: ChromosomeRecombinationModel,
        domains: impl IntoIterator<Item = ChromosomeRecombinationDomain>,
    ) -> Result<Self, EvolutionError> {
        chromosome_map.validate(schema)?;
        let mut by_id = BTreeMap::new();
        for domain in domains {
            domain.validate_local()?;
            let chromosome_id = domain.chromosome_id.clone();
            if by_id.insert(chromosome_id.clone(), domain).is_some() {
                return Err(EvolutionError::DuplicateChromosomeIdentity {
                    chromosome: chromosome_id,
                });
            }
        }

        let value = Self {
            profile_version: CHROMOSOME_RECOMBINATION_PROFILE_VERSION,
            id,
            schema_id: schema.id.clone(),
            schema_digest: schema.canonical_digest()?,
            chromosome_map_id: chromosome_map.id.clone(),
            chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
            model,
            domains: by_id,
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
        if self.profile_version != CHROMOSOME_RECOMBINATION_PROFILE_VERSION {
            return Err(EvolutionError::OperatorAuthorityMismatch);
        }
        if schema.ploidy != 2 {
            return Err(EvolutionError::ModeRequiresDiploid(schema.ploidy));
        }
        if self.schema_id != schema.id || self.schema_digest != schema.canonical_digest()? {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if self.chromosome_map_id != chromosome_map.id
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
        {
            return Err(EvolutionError::OperatorAuthorityMismatch);
        }
        if self.domains.len() != chromosome_map.chromosomes.len() {
            return Err(EvolutionError::OperatorAuthorityMismatch);
        }

        for (key, domain) in &self.domains {
            if key != &domain.chromosome_id {
                return Err(EvolutionError::ChromosomeKeyMismatch {
                    key: key.clone(),
                    value: domain.chromosome_id.clone(),
                });
            }
            domain.validate_local()?;
            let definition = chromosome_map
                .chromosomes
                .get(key)
                .ok_or(EvolutionError::OperatorAuthorityMismatch)?;
            if definition
                .loci
                .iter()
                .any(|locus| !domain.interval.contains(locus.position))
            {
                return Err(EvolutionError::OperatorAuthorityMismatch);
            }
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<ChromosomeRecombinationProfileDigest, EvolutionError> {
        self.validate(schema, chromosome_map)?;
        let mut digest = Sha256::new();
        digest.update(RECOMBINATION_PROFILE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.profile_version);
        put_text(&mut digest, self.id.as_str());
        put_text(&mut digest, self.schema_id.as_str());
        digest.update(self.schema_digest.as_bytes());
        put_text(&mut digest, self.chromosome_map_id.as_str());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update([self.model.tag()]);
        put_u64(&mut digest, self.domains.len() as u64);
        for (chromosome_id, domain) in &self.domains {
            put_text(&mut digest, chromosome_id.as_str());
            put_u64(&mut digest, domain.interval.start.get());
            put_u64(&mut digest, domain.interval.end.get());
        }
        Ok(ChromosomeRecombinationProfileDigest(
            digest.finalize().into(),
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChromosomeRecombinationProfileDigest([u8; 32]);

impl ChromosomeRecombinationProfileDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ChromosomeRecombinationProfileDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChromosomeRecombinationProfileDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ChromosomeRecombinationProfileDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}
