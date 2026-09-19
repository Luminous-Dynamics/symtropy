use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, observe_mutation_fates, ChromosomeMap,
    EvolutionError, HereditarySchema, MutationFateError, MutationFateObservation,
    MutationFateObservationDigest, MutationFateSubject, PopulationGeneticState,
    PopulationGeneticStateDigest, PopulationId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const LINKED_CENSUS_PROJECTION_VERSION: u32 = 1;
const LINKED_CENSUS_PROJECTION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-census-projection:v1\0";

/// Exact projection of one caller-declared linked census into the existing
/// aggregate population-genetics representation plus mutation-fate sidecar.
///
/// The declaration establishes the projection boundary only. It does not prove
/// external/ecological population membership beyond the supplied census.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedCensusProjection {
    projection_version: u32,
    population_id: PopulationId,
    census_individuals: u64,
    mutation_fate_digest: MutationFateObservationDigest,
    aggregate_population_digest: PopulationGeneticStateDigest,
    pub aggregate_population: PopulationGeneticState,
    pub mutation_fate: MutationFateObservation,
}

impl LinkedCensusProjection {
    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn census_individuals(&self) -> u64 {
        self.census_individuals
    }

    pub fn mutation_fate_digest(&self) -> MutationFateObservationDigest {
        self.mutation_fate_digest
    }

    pub fn aggregate_population_digest(&self) -> PopulationGeneticStateDigest {
        self.aggregate_population_digest
    }

    pub fn validate_current(
        &self,
        population_id: &PopulationId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        subjects: &[MutationFateSubject<'_>],
    ) -> Result<(), LinkedCensusProjectionError> {
        if self.projection_version != LINKED_CENSUS_PROJECTION_VERSION {
            return Err(LinkedCensusProjectionError::UnsupportedVersion(
                self.projection_version,
            ));
        }
        if &self.population_id != population_id {
            return Err(LinkedCensusProjectionError::PopulationContextMismatch);
        }
        let recomputed = project_declared_linked_census(
            population_id.clone(),
            schema,
            chromosome_map,
            subjects,
        )?;
        if recomputed != *self {
            return Err(LinkedCensusProjectionError::ProjectionReplayMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
    ) -> Result<LinkedCensusProjectionDigest, LinkedCensusProjectionError> {
        self.aggregate_population.validate(schema)?;
        if self.projection_version != LINKED_CENSUS_PROJECTION_VERSION {
            return Err(LinkedCensusProjectionError::UnsupportedVersion(
                self.projection_version,
            ));
        }
        if self.aggregate_population.population_id != self.population_id
            || self.aggregate_population.census_individuals != self.census_individuals
            || self.aggregate_population.canonical_digest(schema)?
                != self.aggregate_population_digest
            || self.mutation_fate.canonical_digest()? != self.mutation_fate_digest
        {
            return Err(LinkedCensusProjectionError::ProjectionInvariantMismatch);
        }

        let mut digest = Sha256::new();
        digest.update(LINKED_CENSUS_PROJECTION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.projection_version);
        put_text(&mut digest, self.population_id.as_str());
        put_u64(&mut digest, self.census_individuals);
        digest.update(self.mutation_fate_digest.as_bytes());
        digest.update(self.aggregate_population_digest.as_bytes());
        Ok(LinkedCensusProjectionDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkedCensusProjectionDigest([u8; 32]);

impl LinkedCensusProjectionDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for LinkedCensusProjectionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LinkedCensusProjectionDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for LinkedCensusProjectionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

pub fn project_declared_linked_census(
    population_id: PopulationId,
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    subjects: &[MutationFateSubject<'_>],
) -> Result<LinkedCensusProjection, LinkedCensusProjectionError> {
    schema.validate()?;
    chromosome_map.validate(schema)?;
    if subjects.is_empty() {
        return Err(LinkedCensusProjectionError::EmptyCensus);
    }

    let mut census_individuals = 0_u64;
    for subject in subjects {
        if subject.multiplicity == 0 {
            return Err(LinkedCensusProjectionError::ZeroMultiplicity);
        }
        census_individuals = census_individuals
            .checked_add(subject.multiplicity)
            .ok_or(LinkedCensusProjectionError::CountOverflow)?;
    }
    if census_individuals == 0 {
        return Err(LinkedCensusProjectionError::EmptyCensus);
    }

    let mutation_fate = observe_mutation_fates(schema, chromosome_map, subjects)?;
    let mut allele_copy_counts = BTreeMap::new();
    for locus in &mutation_fate.loci {
        let counts = locus
            .allele_counts
            .iter()
            .map(|count| (count.allele_id.clone(), count.count))
            .collect();
        if allele_copy_counts
            .insert(locus.locus_id.clone(), counts)
            .is_some()
        {
            return Err(LinkedCensusProjectionError::ObservationCoverageMismatch);
        }
    }
    if allele_copy_counts.len() != schema.loci.len() {
        return Err(LinkedCensusProjectionError::ObservationCoverageMismatch);
    }

    let aggregate_population = PopulationGeneticState::from_counts(
        population_id.clone(),
        schema,
        census_individuals,
        allele_copy_counts,
    )?;

    for locus in &mutation_fate.loci {
        let aggregate = aggregate_population
            .allele_copy_counts
            .get(&locus.locus_id)
            .ok_or(LinkedCensusProjectionError::ObservationCoverageMismatch)?;
        if aggregate.len() != locus.allele_counts.len() {
            return Err(LinkedCensusProjectionError::AlleleProjectionMismatch);
        }
        for observed in &locus.allele_counts {
            if aggregate.get(&observed.allele_id).copied() != Some(observed.count) {
                return Err(LinkedCensusProjectionError::AlleleProjectionMismatch);
            }
        }
    }

    let mutation_fate_digest = mutation_fate.canonical_digest()?;
    let aggregate_population_digest = aggregate_population.canonical_digest(schema)?;
    let projection = LinkedCensusProjection {
        projection_version: LINKED_CENSUS_PROJECTION_VERSION,
        population_id,
        census_individuals,
        mutation_fate_digest,
        aggregate_population_digest,
        aggregate_population,
        mutation_fate,
    };
    projection.canonical_digest(schema)?;
    Ok(projection)
}

#[derive(Debug)]
pub enum LinkedCensusProjectionError {
    Evolution(EvolutionError),
    MutationFate(MutationFateError),
    UnsupportedVersion(u32),
    EmptyCensus,
    ZeroMultiplicity,
    CountOverflow,
    PopulationContextMismatch,
    ObservationCoverageMismatch,
    AlleleProjectionMismatch,
    ProjectionInvariantMismatch,
    ProjectionReplayMismatch,
}

impl From<EvolutionError> for LinkedCensusProjectionError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<MutationFateError> for LinkedCensusProjectionError {
    fn from(value: MutationFateError) -> Self {
        Self::MutationFate(value)
    }
}

impl fmt::Display for LinkedCensusProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::MutationFate(error) => write!(f, "mutation-fate authority error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported linked-census projection version {version}")
            }
            Self::EmptyCensus => write!(f, "linked-census projection is empty"),
            Self::ZeroMultiplicity => write!(f, "linked-census subject multiplicity must be positive"),
            Self::CountOverflow => write!(f, "linked-census projection count overflow"),
            Self::PopulationContextMismatch => write!(f, "linked-census population context mismatch"),
            Self::ObservationCoverageMismatch => write!(f, "mutation-fate observation does not cover the hereditary locus set exactly"),
            Self::AlleleProjectionMismatch => write!(f, "aggregate population allele counts do not match mutation-fate observation"),
            Self::ProjectionInvariantMismatch => write!(f, "linked-census projection invariants do not hold"),
            Self::ProjectionReplayMismatch => write!(f, "restored linked-census projection does not replay"),
        }
    }
}

impl Error for LinkedCensusProjectionError {}
