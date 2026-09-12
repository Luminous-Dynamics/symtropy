use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, neutral_wright_fisher_step, AlleleId,
    ChromosomeMap, EvolutionError, HereditarySchema, HereditarySchemaDigest, LinkedCensusProjection,
    LinkedCensusProjectionError, LocusId, MutationFateSubject, MutationOriginDigest,
    PopulationGeneticState, PopulationGeneticStateDigest, PopulationProcessModel,
    PopulationProcessProfile, PopulationProcessProfileDigest, PopulationTrajectoryPoint,
    PopulationTrajectoryPointDigest, PopulationTransitionId, PopulationTransitionProvenanceDigest,
    PopulationTransitionResult,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const ORIGIN_AWARE_POPULATION_STATE_VERSION: u32 = 1;
pub const ORIGIN_AWARE_POPULATION_TRANSITION_VERSION: u32 = 1;

const STATE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:origin-aware-population-state:v1\0";
const TRANSITION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:origin-aware-population-transition:v1\0";
const PROVENANCE_DRAW_DOMAIN: &[u8] =
    b"symtropy:evolution:origin-aware-provenance-draw:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateActiveOriginCount {
    pub origin_digest: MutationOriginDigest,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateAlleleProvenance {
    pub allele_id: AlleleId,
    pub modeled_baseline_count: u64,
    pub active_origin_counts: Vec<AggregateActiveOriginCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AggregateLocusProvenance {
    pub locus_id: LocusId,
    pub alleles: Vec<AggregateAlleleProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginAwarePopulationState {
    state_version: u32,
    schema_digest: HereditarySchemaDigest,
    population_digest: PopulationGeneticStateDigest,
    pub population: PopulationGeneticState,
    pub loci: Vec<AggregateLocusProvenance>,
}

impl OriginAwarePopulationState {
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
    ) -> Result<(), OriginAwarePopulationError> {
        if self.state_version != ORIGIN_AWARE_POPULATION_STATE_VERSION {
            return Err(OriginAwarePopulationError::UnsupportedStateVersion(
                self.state_version,
            ));
        }
        self.population.validate(schema)?;
        if self.schema_digest != schema.canonical_digest()?
            || self.population_digest != self.population.canonical_digest(schema)?
        {
            return Err(OriginAwarePopulationError::CurrentAuthorityMismatch);
        }
        if self.loci.len() != schema.loci.len() {
            return Err(OriginAwarePopulationError::LocusCoverageMismatch);
        }
        if self
            .loci
            .windows(2)
            .any(|window| window[0].locus_id >= window[1].locus_id)
        {
            return Err(OriginAwarePopulationError::NonCanonicalLocusOrder);
        }

        let mut seen_origins = BTreeSet::new();
        for locus in &self.loci {
            let schema_locus = schema
                .loci
                .get(&locus.locus_id)
                .ok_or_else(|| OriginAwarePopulationError::LocusCoverageMismatch)?;
            let population_counts = self
                .population
                .allele_copy_counts
                .get(&locus.locus_id)
                .ok_or(OriginAwarePopulationError::LocusCoverageMismatch)?;
            if locus.alleles.len() != population_counts.len() {
                return Err(OriginAwarePopulationError::AllelePartitionMismatch);
            }
            if locus
                .alleles
                .windows(2)
                .any(|window| window[0].allele_id >= window[1].allele_id)
            {
                return Err(OriginAwarePopulationError::NonCanonicalAlleleOrder);
            }

            for allele in &locus.alleles {
                if !schema_locus.allowed_alleles.contains(&allele.allele_id) {
                    return Err(OriginAwarePopulationError::Evolution(
                        EvolutionError::UnknownAllele {
                            locus: locus.locus_id.clone(),
                            allele: allele.allele_id.clone(),
                        },
                    ));
                }
                let expected = population_counts
                    .get(&allele.allele_id)
                    .copied()
                    .ok_or(OriginAwarePopulationError::AllelePartitionMismatch)?;
                if expected == 0 {
                    return Err(OriginAwarePopulationError::AllelePartitionMismatch);
                }
                if allele.active_origin_counts.windows(2).any(|window| {
                    window[0].origin_digest.as_bytes() >= window[1].origin_digest.as_bytes()
                }) {
                    return Err(OriginAwarePopulationError::NonCanonicalOriginOrder);
                }
                let mut total = allele.modeled_baseline_count;
                for origin in &allele.active_origin_counts {
                    if origin.count == 0 {
                        return Err(OriginAwarePopulationError::ZeroOriginCount);
                    }
                    if !seen_origins.insert(*origin.origin_digest.as_bytes()) {
                        return Err(OriginAwarePopulationError::DuplicateOriginPartition);
                    }
                    total = total
                        .checked_add(origin.count)
                        .ok_or(OriginAwarePopulationError::CountOverflow)?;
                }
                if total != expected {
                    return Err(OriginAwarePopulationError::AllelePartitionMismatch);
                }
            }
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
    ) -> Result<OriginAwarePopulationStateDigest, OriginAwarePopulationError> {
        self.validate_current(schema)?;
        let mut digest = Sha256::new();
        digest.update(STATE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.state_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.population_digest.as_bytes());
        put_u64(&mut digest, self.loci.len() as u64);
        for locus in &self.loci {
            put_text(&mut digest, locus.locus_id.as_str());
            put_u64(&mut digest, locus.alleles.len() as u64);
            for allele in &locus.alleles {
                put_text(&mut digest, allele.allele_id.as_str());
                put_u64(&mut digest, allele.modeled_baseline_count);
                put_u64(&mut digest, allele.active_origin_counts.len() as u64);
                for origin in &allele.active_origin_counts {
                    digest.update(origin.origin_digest.as_bytes());
                    put_u64(&mut digest, origin.count);
                }
            }
        }
        Ok(OriginAwarePopulationStateDigest(digest.finalize().into()))
    }

    pub fn allele_provenance(
        &self,
        locus_id: &LocusId,
        allele_id: &AlleleId,
    ) -> Option<&AggregateAlleleProvenance> {
        self.loci
            .iter()
            .find(|locus| &locus.locus_id == locus_id)
            .and_then(|locus| locus.alleles.iter().find(|allele| &allele.allele_id == allele_id))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OriginAwarePopulationStateDigest([u8; 32]);

impl OriginAwarePopulationStateDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for OriginAwarePopulationStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OriginAwarePopulationStateDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for OriginAwarePopulationStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

pub fn initialize_origin_aware_population_from_projection(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    projection: &LinkedCensusProjection,
    subjects: &[MutationFateSubject<'_>],
) -> Result<OriginAwarePopulationState, OriginAwarePopulationError> {
    projection.validate_current(projection.population_id(), schema, chromosome_map, subjects)?;

    let mut loci = Vec::with_capacity(projection.mutation_fate.loci.len());
    for observed_locus in &projection.mutation_fate.loci {
        let mut origins_by_allele: BTreeMap<AlleleId, Vec<AggregateActiveOriginCount>> =
            BTreeMap::new();
        for pair in &observed_locus.allele_origin_counts {
            origins_by_allele
                .entry(pair.allele_id.clone())
                .or_default()
                .push(AggregateActiveOriginCount {
                    origin_digest: pair.origin_digest,
                    count: pair.count,
                });
        }
        for origins in origins_by_allele.values_mut() {
            origins.sort_by(|left, right| {
                left.origin_digest
                    .as_bytes()
                    .cmp(right.origin_digest.as_bytes())
            });
        }

        let mut alleles = Vec::with_capacity(observed_locus.allele_counts.len());
        for allele_count in &observed_locus.allele_counts {
            let active_origin_counts = origins_by_allele
                .remove(&allele_count.allele_id)
                .unwrap_or_default();
            let active_total = sum_counts(active_origin_counts.iter().map(|count| count.count))?;
            let modeled_baseline_count = allele_count
                .count
                .checked_sub(active_total)
                .ok_or(OriginAwarePopulationError::AllelePartitionMismatch)?;
            alleles.push(AggregateAlleleProvenance {
                allele_id: allele_count.allele_id.clone(),
                modeled_baseline_count,
                active_origin_counts,
            });
        }
        if !origins_by_allele.is_empty() {
            return Err(OriginAwarePopulationError::AllelePartitionMismatch);
        }
        alleles.sort_by(|left, right| left.allele_id.cmp(&right.allele_id));
        loci.push(AggregateLocusProvenance {
            locus_id: observed_locus.locus_id.clone(),
            alleles,
        });
    }
    loci.sort_by(|left, right| left.locus_id.cmp(&right.locus_id));

    let population = projection.aggregate_population.clone();
    let state = OriginAwarePopulationState {
        state_version: ORIGIN_AWARE_POPULATION_STATE_VERSION,
        schema_digest: schema.canonical_digest()?,
        population_digest: population.canonical_digest(schema)?,
        population,
        loci,
    };
    state.validate_current(schema)?;
    Ok(state)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginAwarePopulationTransitionProvenance {
    transition_version: u32,
    source_state_digest: OriginAwarePopulationStateDigest,
    source_point_digest: PopulationTrajectoryPointDigest,
    process_profile_digest: PopulationProcessProfileDigest,
    transition_id: PopulationTransitionId,
    ordinary_transition_digest: PopulationTransitionProvenanceDigest,
    destination_state_digest: OriginAwarePopulationStateDigest,
}

impl OriginAwarePopulationTransitionProvenance {
    pub fn canonical_digest(&self) -> OriginAwarePopulationTransitionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(TRANSITION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.transition_version);
        digest.update(self.source_state_digest.as_bytes());
        digest.update(self.source_point_digest.as_bytes());
        digest.update(self.process_profile_digest.as_bytes());
        put_text(&mut digest, self.transition_id.as_str());
        digest.update(self.ordinary_transition_digest.as_bytes());
        digest.update(self.destination_state_digest.as_bytes());
        OriginAwarePopulationTransitionProvenanceDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OriginAwarePopulationTransitionProvenanceDigest([u8; 32]);

impl OriginAwarePopulationTransitionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for OriginAwarePopulationTransitionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OriginAwarePopulationTransitionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for OriginAwarePopulationTransitionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginAwarePopulationTransitionResult {
    pub destination: OriginAwarePopulationState,
    pub ordinary_transition: PopulationTransitionResult,
    pub provenance: OriginAwarePopulationTransitionProvenance,
}

impl OriginAwarePopulationTransitionResult {
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        source: &OriginAwarePopulationState,
        source_point: &PopulationTrajectoryPoint,
        transition_id: &PopulationTransitionId,
        profile: &PopulationProcessProfile,
    ) -> Result<(), OriginAwarePopulationError> {
        let recomputed = neutral_origin_aware_population_step(
            schema,
            source,
            source_point,
            transition_id,
            profile,
        )?;
        if recomputed != *self {
            return Err(OriginAwarePopulationError::TransitionReplayMismatch);
        }
        Ok(())
    }
}

pub fn neutral_origin_aware_population_step(
    schema: &HereditarySchema,
    source: &OriginAwarePopulationState,
    source_point: &PopulationTrajectoryPoint,
    transition_id: &PopulationTransitionId,
    profile: &PopulationProcessProfile,
) -> Result<OriginAwarePopulationTransitionResult, OriginAwarePopulationError> {
    source.validate_current(schema)?;
    source_point.validate_current(schema, &source.population)?;
    profile.validate()?;
    if profile.model != PopulationProcessModel::NeutralIndependentLocusWrightFisher {
        return Err(OriginAwarePopulationError::UnsupportedProcessModel);
    }

    let ordinary_transition = neutral_wright_fisher_step(
        schema,
        &source.population,
        source_point,
        transition_id,
        profile,
    )?;
    let source_state_digest = source.canonical_digest(schema)?;
    let source_point_digest = source_point.canonical_digest();
    let process_profile_digest = profile.canonical_digest()?;

    let mut destination_loci = Vec::with_capacity(source.loci.len());
    for source_locus in &source.loci {
        let destination_counts = ordinary_transition
            .destination
            .allele_copy_counts
            .get(&source_locus.locus_id)
            .ok_or(OriginAwarePopulationError::LocusCoverageMismatch)?;
        let mut destination_alleles = Vec::with_capacity(destination_counts.len());
        for (allele_id, destination_count) in destination_counts {
            let source_allele = source
                .allele_provenance(&source_locus.locus_id, allele_id)
                .ok_or(OriginAwarePopulationError::AllelePartitionMismatch)?;
            let source_total = source
                .population
                .allele_copy_counts
                .get(&source_locus.locus_id)
                .and_then(|counts| counts.get(allele_id))
                .copied()
                .ok_or(OriginAwarePopulationError::AllelePartitionMismatch)?;

            let mut baseline_count = 0_u64;
            let mut origin_counts: BTreeMap<[u8; 32], (MutationOriginDigest, u64)> =
                BTreeMap::new();
            for destination_ordinal in 0..*destination_count {
                let draw = semantic_provenance_draw_below(
                    &source_state_digest,
                    &source_point_digest,
                    &process_profile_digest,
                    transition_id,
                    &source_locus.locus_id,
                    allele_id,
                    destination_ordinal,
                    source_total,
                )?;
                let selected = select_source_subclass(source_allele, draw)?;
                match selected {
                    ProvenanceSubclass::ModeledBaseline => {
                        baseline_count = baseline_count
                            .checked_add(1)
                            .ok_or(OriginAwarePopulationError::CountOverflow)?;
                    }
                    ProvenanceSubclass::ActiveOrigin(origin) => {
                        let entry = origin_counts.entry(*origin.as_bytes()).or_insert((origin, 0));
                        entry.1 = entry
                            .1
                            .checked_add(1)
                            .ok_or(OriginAwarePopulationError::CountOverflow)?;
                    }
                }
            }
            let active_origin_counts = origin_counts
                .into_values()
                .map(|(origin_digest, count)| AggregateActiveOriginCount {
                    origin_digest,
                    count,
                })
                .collect();
            destination_alleles.push(AggregateAlleleProvenance {
                allele_id: allele_id.clone(),
                modeled_baseline_count: baseline_count,
                active_origin_counts,
            });
        }
        destination_loci.push(AggregateLocusProvenance {
            locus_id: source_locus.locus_id.clone(),
            alleles: destination_alleles,
        });
    }

    let destination_population = ordinary_transition.destination.clone();
    let destination = OriginAwarePopulationState {
        state_version: ORIGIN_AWARE_POPULATION_STATE_VERSION,
        schema_digest: schema.canonical_digest()?,
        population_digest: destination_population.canonical_digest(schema)?,
        population: destination_population,
        loci: destination_loci,
    };
    destination.validate_current(schema)?;
    let destination_state_digest = destination.canonical_digest(schema)?;
    let provenance = OriginAwarePopulationTransitionProvenance {
        transition_version: ORIGIN_AWARE_POPULATION_TRANSITION_VERSION,
        source_state_digest,
        source_point_digest,
        process_profile_digest,
        transition_id: transition_id.clone(),
        ordinary_transition_digest: ordinary_transition.provenance.canonical_digest(),
        destination_state_digest,
    };

    Ok(OriginAwarePopulationTransitionResult {
        destination,
        ordinary_transition,
        provenance,
    })
}

#[derive(Clone, Copy)]
enum ProvenanceSubclass {
    ModeledBaseline,
    ActiveOrigin(MutationOriginDigest),
}

fn select_source_subclass(
    allele: &AggregateAlleleProvenance,
    draw: u64,
) -> Result<ProvenanceSubclass, OriginAwarePopulationError> {
    if draw < allele.modeled_baseline_count {
        return Ok(ProvenanceSubclass::ModeledBaseline);
    }
    let mut cursor = allele.modeled_baseline_count;
    for origin in &allele.active_origin_counts {
        cursor = cursor
            .checked_add(origin.count)
            .ok_or(OriginAwarePopulationError::CountOverflow)?;
        if draw < cursor {
            return Ok(ProvenanceSubclass::ActiveOrigin(origin.origin_digest));
        }
    }
    Err(OriginAwarePopulationError::SamplingInvariantViolation)
}

#[allow(clippy::too_many_arguments)]
fn semantic_provenance_draw_below(
    source_state_digest: &OriginAwarePopulationStateDigest,
    source_point_digest: &PopulationTrajectoryPointDigest,
    process_profile_digest: &PopulationProcessProfileDigest,
    transition_id: &PopulationTransitionId,
    locus_id: &LocusId,
    allele_id: &AlleleId,
    destination_ordinal: u64,
    upper: u64,
) -> Result<u64, OriginAwarePopulationError> {
    if upper == 0 {
        return Err(OriginAwarePopulationError::SamplingInvariantViolation);
    }
    let space = 1_u128 << 64;
    let limit = space - (space % u128::from(upper));
    for attempt in 0_u64.. {
        let mut digest = Sha256::new();
        digest.update(PROVENANCE_DRAW_DOMAIN);
        digest.update(source_state_digest.as_bytes());
        digest.update(source_point_digest.as_bytes());
        digest.update(process_profile_digest.as_bytes());
        put_text(&mut digest, transition_id.as_str());
        put_text(&mut digest, locus_id.as_str());
        put_text(&mut digest, allele_id.as_str());
        put_u64(&mut digest, destination_ordinal);
        put_u64(&mut digest, attempt);
        let bytes = digest.finalize();
        let value = u64::from_be_bytes(bytes[0..8].try_into().expect("sha256 prefix is 8 bytes"));
        if u128::from(value) < limit {
            return Ok(value % upper);
        }
    }
    unreachable!("rejection sampler attempt counter exhausted")
}

fn sum_counts(values: impl IntoIterator<Item = u64>) -> Result<u64, OriginAwarePopulationError> {
    let mut total = 0_u64;
    for value in values {
        total = total
            .checked_add(value)
            .ok_or(OriginAwarePopulationError::CountOverflow)?;
    }
    Ok(total)
}

#[derive(Debug)]
pub enum OriginAwarePopulationError {
    Evolution(EvolutionError),
    Projection(LinkedCensusProjectionError),
    UnsupportedStateVersion(u32),
    CurrentAuthorityMismatch,
    LocusCoverageMismatch,
    NonCanonicalLocusOrder,
    NonCanonicalAlleleOrder,
    NonCanonicalOriginOrder,
    ZeroOriginCount,
    DuplicateOriginPartition,
    AllelePartitionMismatch,
    CountOverflow,
    UnsupportedProcessModel,
    SamplingInvariantViolation,
    TransitionReplayMismatch,
}

impl From<EvolutionError> for OriginAwarePopulationError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<LinkedCensusProjectionError> for OriginAwarePopulationError {
    fn from(value: LinkedCensusProjectionError) -> Self {
        Self::Projection(value)
    }
}

impl fmt::Display for OriginAwarePopulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Projection(error) => write!(f, "linked-census projection error: {error}"),
            Self::UnsupportedStateVersion(version) => {
                write!(f, "unsupported origin-aware population state version {version}")
            }
            Self::CurrentAuthorityMismatch => {
                write!(f, "origin-aware population state does not match current authority")
            }
            Self::LocusCoverageMismatch => {
                write!(f, "origin-aware population locus coverage mismatch")
            }
            Self::NonCanonicalLocusOrder => {
                write!(f, "origin-aware population loci are not in canonical order")
            }
            Self::NonCanonicalAlleleOrder => {
                write!(f, "origin-aware population alleles are not in canonical order")
            }
            Self::NonCanonicalOriginOrder => {
                write!(f, "origin-aware population origins are not in canonical order")
            }
            Self::ZeroOriginCount => write!(f, "origin-aware population contains a zero-count origin"),
            Self::DuplicateOriginPartition => {
                write!(f, "one mutation origin appears in multiple provenance partitions")
            }
            Self::AllelePartitionMismatch => {
                write!(f, "origin-aware provenance partition does not equal aggregate allele count")
            }
            Self::CountOverflow => write!(f, "origin-aware population count overflow"),
            Self::UnsupportedProcessModel => write!(
                f,
                "origin-aware neutral closure requires NeutralIndependentLocusWrightFisher"
            ),
            Self::SamplingInvariantViolation => {
                write!(f, "origin-aware provenance sampling invariant violated")
            }
            Self::TransitionReplayMismatch => {
                write!(f, "restored origin-aware population transition does not replay")
            }
        }
    }
}

impl Error for OriginAwarePopulationError {}
