use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AlleleId, ChromosomeId, ChromosomeMap,
    ChromosomeMapDigest, EvolutionError, HereditarySchema, HereditarySchemaDigest, LocusId,
    MutationLineageError, MutationLineageState, MutationLineageStateDigest, MutationOriginDigest,
    PhasedAncestryState, PhasedHereditaryState,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
};

pub const MUTATION_FATE_OBSERVATION_VERSION: u32 = 1;
const MUTATION_FATE_OBSERVATION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:mutation-fate-observation:v1\0";

/// One exact current subject included in a mutation-fate observation.
///
/// This is a borrowed runtime input, not a persistent population-membership claim.
#[derive(Debug, Clone, Copy)]
pub struct MutationFateSubject<'a> {
    pub phased_state: &'a PhasedHereditaryState,
    pub ancestry_state: &'a PhasedAncestryState,
    pub lineage_state: &'a MutationLineageState,
    pub multiplicity: u64,
}

impl<'a> MutationFateSubject<'a> {
    pub fn new(
        phased_state: &'a PhasedHereditaryState,
        ancestry_state: &'a PhasedAncestryState,
        lineage_state: &'a MutationLineageState,
        multiplicity: u64,
    ) -> Result<Self, MutationFateError> {
        if multiplicity == 0 {
            return Err(MutationFateError::ZeroMultiplicity);
        }
        Ok(Self {
            phased_state,
            ancestry_state,
            lineage_state,
            multiplicity,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationFateSourceMultiplicity {
    pub lineage_digest: MutationLineageStateDigest,
    pub multiplicity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationFateAlleleCount {
    pub allele_id: AlleleId,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationFateOriginCount {
    pub origin_digest: MutationOriginDigest,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationFateAlleleOriginCount {
    pub allele_id: AlleleId,
    pub origin_digest: MutationOriginDigest,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationFateLocusObservation {
    pub chromosome_id: ChromosomeId,
    pub locus_id: LocusId,
    pub total_copy_count: u64,
    pub modeled_baseline_count: u64,
    pub allele_counts: Vec<MutationFateAlleleCount>,
    pub origin_counts: Vec<MutationFateOriginCount>,
    pub allele_origin_counts: Vec<MutationFateAlleleOriginCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationFateObservation {
    observation_version: u32,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    pub sources: Vec<MutationFateSourceMultiplicity>,
    pub loci: Vec<MutationFateLocusObservation>,
}

impl MutationFateObservation {
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        subjects: &[MutationFateSubject<'_>],
    ) -> Result<(), MutationFateError> {
        self.validate_local()?;
        let recomputed = observe_mutation_fates(schema, chromosome_map, subjects)?;
        if recomputed != *self {
            return Err(MutationFateError::ObservationReplayMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<MutationFateObservationDigest, MutationFateError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(MUTATION_FATE_OBSERVATION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.observation_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());

        put_u64(&mut digest, self.sources.len() as u64);
        for source in &self.sources {
            digest.update(source.lineage_digest.as_bytes());
            put_u64(&mut digest, source.multiplicity);
        }

        put_u64(&mut digest, self.loci.len() as u64);
        for locus in &self.loci {
            put_text(&mut digest, locus.chromosome_id.as_str());
            put_text(&mut digest, locus.locus_id.as_str());
            put_u64(&mut digest, locus.total_copy_count);
            put_u64(&mut digest, locus.modeled_baseline_count);

            put_u64(&mut digest, locus.allele_counts.len() as u64);
            for count in &locus.allele_counts {
                put_text(&mut digest, count.allele_id.as_str());
                put_u64(&mut digest, count.count);
            }

            put_u64(&mut digest, locus.origin_counts.len() as u64);
            for count in &locus.origin_counts {
                digest.update(count.origin_digest.as_bytes());
                put_u64(&mut digest, count.count);
            }

            put_u64(&mut digest, locus.allele_origin_counts.len() as u64);
            for count in &locus.allele_origin_counts {
                put_text(&mut digest, count.allele_id.as_str());
                digest.update(count.origin_digest.as_bytes());
                put_u64(&mut digest, count.count);
            }
        }
        Ok(MutationFateObservationDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), MutationFateError> {
        if self.observation_version != MUTATION_FATE_OBSERVATION_VERSION {
            return Err(MutationFateError::UnsupportedVersion(self.observation_version));
        }
        if self.sources.is_empty() {
            return Err(MutationFateError::EmptySample);
        }
        for source in &self.sources {
            if source.multiplicity == 0 {
                return Err(MutationFateError::ZeroMultiplicity);
            }
        }
        if self.sources.windows(2).any(|window| {
            window[0].lineage_digest.as_bytes() >= window[1].lineage_digest.as_bytes()
        }) {
            return Err(MutationFateError::NonCanonicalSourceOrder);
        }
        if self.loci.windows(2).any(|window| {
            (&window[0].chromosome_id, &window[0].locus_id)
                >= (&window[1].chromosome_id, &window[1].locus_id)
        }) {
            return Err(MutationFateError::NonCanonicalLocusOrder);
        }
        for locus in &self.loci {
            validate_locus(locus)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutationFateObservationDigest([u8; 32]);

impl MutationFateObservationDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for MutationFateObservationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MutationFateObservationDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for MutationFateObservationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Default)]
struct LocusBuilder {
    total_copy_count: u64,
    modeled_baseline_count: u64,
    allele_counts: BTreeMap<AlleleId, u64>,
    origin_counts: BTreeMap<[u8; 32], (MutationOriginDigest, u64)>,
    allele_origin_counts: BTreeMap<(AlleleId, [u8; 32]), (MutationOriginDigest, u64)>,
}

pub fn observe_mutation_fates(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    subjects: &[MutationFateSubject<'_>],
) -> Result<MutationFateObservation, MutationFateError> {
    schema.validate()?;
    chromosome_map.validate(schema)?;
    if subjects.is_empty() {
        return Err(MutationFateError::EmptySample);
    }

    let mut source_multiplicity: BTreeMap<[u8; 32], (MutationLineageStateDigest, u64)> =
        BTreeMap::new();
    let mut loci: BTreeMap<(ChromosomeId, LocusId), LocusBuilder> = BTreeMap::new();

    for subject in subjects {
        if subject.multiplicity == 0 {
            return Err(MutationFateError::ZeroMultiplicity);
        }
        subject.lineage_state.validate_current(
            schema,
            chromosome_map,
            subject.phased_state,
            subject.ancestry_state,
        )?;
        let lineage_digest = subject.lineage_state.canonical_digest(
            schema,
            chromosome_map,
            subject.phased_state,
            subject.ancestry_state,
        )?;
        let source_key = *lineage_digest.as_bytes();
        let source = source_multiplicity
            .entry(source_key)
            .or_insert((lineage_digest, 0));
        source.1 = checked_add(source.1, subject.multiplicity)?;

        for entry in &subject.lineage_state.entries {
            let builder = loci
                .entry((entry.chromosome_id.clone(), entry.locus_id.clone()))
                .or_default();
            builder.total_copy_count = checked_add(builder.total_copy_count, subject.multiplicity)?;
            let allele_count = builder.allele_counts.entry(entry.allele.clone()).or_insert(0);
            *allele_count = checked_add(*allele_count, subject.multiplicity)?;

            match entry.active_origin {
                None => {
                    builder.modeled_baseline_count =
                        checked_add(builder.modeled_baseline_count, subject.multiplicity)?;
                }
                Some(origin) => {
                    let origin_key = *origin.as_bytes();
                    let origin_count = builder.origin_counts.entry(origin_key).or_insert((origin, 0));
                    origin_count.1 = checked_add(origin_count.1, subject.multiplicity)?;
                    let pair = builder
                        .allele_origin_counts
                        .entry((entry.allele.clone(), origin_key))
                        .or_insert((origin, 0));
                    pair.1 = checked_add(pair.1, subject.multiplicity)?;
                }
            }
        }
    }

    let sources = source_multiplicity
        .into_values()
        .map(|(lineage_digest, multiplicity)| MutationFateSourceMultiplicity {
            lineage_digest,
            multiplicity,
        })
        .collect();

    let mut locus_observations = Vec::with_capacity(loci.len());
    for ((chromosome_id, locus_id), builder) in loci {
        let allele_counts = builder
            .allele_counts
            .into_iter()
            .map(|(allele_id, count)| MutationFateAlleleCount { allele_id, count })
            .collect();
        let origin_counts = builder
            .origin_counts
            .into_values()
            .map(|(origin_digest, count)| MutationFateOriginCount { origin_digest, count })
            .collect();
        let allele_origin_counts = builder
            .allele_origin_counts
            .into_iter()
            .map(|((allele_id, _), (origin_digest, count))| MutationFateAlleleOriginCount {
                allele_id,
                origin_digest,
                count,
            })
            .collect();
        locus_observations.push(MutationFateLocusObservation {
            chromosome_id,
            locus_id,
            total_copy_count: builder.total_copy_count,
            modeled_baseline_count: builder.modeled_baseline_count,
            allele_counts,
            origin_counts,
            allele_origin_counts,
        });
    }

    let observation = MutationFateObservation {
        observation_version: MUTATION_FATE_OBSERVATION_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        sources,
        loci: locus_observations,
    };
    observation.validate_local()?;
    Ok(observation)
}

fn checked_add(left: u64, right: u64) -> Result<u64, MutationFateError> {
    left.checked_add(right).ok_or(MutationFateError::CountOverflow)
}

fn validate_locus(locus: &MutationFateLocusObservation) -> Result<(), MutationFateError> {
    if locus.total_copy_count == 0 {
        return Err(MutationFateError::ZeroLocusCount);
    }
    if locus.allele_counts.is_empty() {
        return Err(MutationFateError::LocusCountMismatch);
    }
    if locus.allele_counts.windows(2).any(|window| window[0].allele_id >= window[1].allele_id) {
        return Err(MutationFateError::NonCanonicalAlleleOrder);
    }
    if locus.origin_counts.windows(2).any(|window| {
        window[0].origin_digest.as_bytes() >= window[1].origin_digest.as_bytes()
    }) {
        return Err(MutationFateError::NonCanonicalOriginOrder);
    }
    if locus.allele_origin_counts.windows(2).any(|window| {
        (&window[0].allele_id, window[0].origin_digest.as_bytes())
            >= (&window[1].allele_id, window[1].origin_digest.as_bytes())
    }) {
        return Err(MutationFateError::NonCanonicalAlleleOriginOrder);
    }

    let allele_total = sum_counts(locus.allele_counts.iter().map(|count| count.count))?;
    if allele_total != locus.total_copy_count {
        return Err(MutationFateError::LocusCountMismatch);
    }
    let origin_total = sum_counts(locus.origin_counts.iter().map(|count| count.count))?;
    if checked_add(locus.modeled_baseline_count, origin_total)? != locus.total_copy_count {
        return Err(MutationFateError::LocusCountMismatch);
    }
    let allele_origin_total =
        sum_counts(locus.allele_origin_counts.iter().map(|count| count.count))?;
    if allele_origin_total != origin_total {
        return Err(MutationFateError::LocusCountMismatch);
    }

    let mut origin_by_key = BTreeMap::new();
    for count in &locus.origin_counts {
        origin_by_key.insert(*count.origin_digest.as_bytes(), count.count);
    }
    let mut origin_from_pairs: BTreeMap<[u8; 32], u64> = BTreeMap::new();
    for count in &locus.allele_origin_counts {
        let key = *count.origin_digest.as_bytes();
        let value = origin_from_pairs.entry(key).or_insert(0);
        *value = checked_add(*value, count.count)?;
    }
    if origin_by_key != origin_from_pairs {
        return Err(MutationFateError::LocusCountMismatch);
    }
    Ok(())
}

fn sum_counts(values: impl IntoIterator<Item = u64>) -> Result<u64, MutationFateError> {
    let mut total = 0_u64;
    for value in values {
        total = checked_add(total, value)?;
    }
    Ok(total)
}

#[derive(Debug)]
pub enum MutationFateError {
    Evolution(EvolutionError),
    Lineage(MutationLineageError),
    UnsupportedVersion(u32),
    EmptySample,
    ZeroMultiplicity,
    CountOverflow,
    ZeroLocusCount,
    NonCanonicalSourceOrder,
    NonCanonicalLocusOrder,
    NonCanonicalAlleleOrder,
    NonCanonicalOriginOrder,
    NonCanonicalAlleleOriginOrder,
    LocusCountMismatch,
    ObservationReplayMismatch,
}

impl From<EvolutionError> for MutationFateError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<MutationLineageError> for MutationFateError {
    fn from(value: MutationLineageError) -> Self {
        Self::Lineage(value)
    }
}

impl fmt::Display for MutationFateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Lineage(error) => write!(f, "mutation-lineage authority error: {error}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported mutation-fate observation version {version}"),
            Self::EmptySample => write!(f, "mutation-fate observation sample is empty"),
            Self::ZeroMultiplicity => write!(f, "mutation-fate subject multiplicity must be positive"),
            Self::CountOverflow => write!(f, "mutation-fate count overflow"),
            Self::ZeroLocusCount => write!(f, "mutation-fate locus has zero sampled copies"),
            Self::NonCanonicalSourceOrder => write!(f, "mutation-fate sources are not in canonical order"),
            Self::NonCanonicalLocusOrder => write!(f, "mutation-fate loci are not in canonical order"),
            Self::NonCanonicalAlleleOrder => write!(f, "mutation-fate allele counts are not in canonical order"),
            Self::NonCanonicalOriginOrder => write!(f, "mutation-fate origin counts are not in canonical order"),
            Self::NonCanonicalAlleleOriginOrder => write!(f, "mutation-fate allele/origin counts are not in canonical order"),
            Self::LocusCountMismatch => write!(f, "mutation-fate locus count invariants do not hold"),
            Self::ObservationReplayMismatch => write!(f, "restored mutation-fate observation does not replay"),
        }
    }
}

impl Error for MutationFateError {}
