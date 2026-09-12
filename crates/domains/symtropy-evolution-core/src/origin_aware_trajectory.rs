use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, neutral_origin_aware_population_step,
    AlleleId, EvolutionExperimentId, HereditarySchema, HereditarySchemaDigest, LocusId,
    MutationOriginDigest, OriginAwarePopulationError, OriginAwarePopulationState,
    OriginAwarePopulationStateDigest, OriginAwarePopulationTransitionResult, PopulationGeneration,
    PopulationProcessProfile, PopulationTrajectoryPoint, PopulationTrajectoryPointDigest,
    PopulationTransitionId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const ORIGIN_AWARE_TRAJECTORY_POINT_VERSION: u32 = 1;
pub const ORIGIN_FATE_DELTA_VERSION: u32 = 1;
pub const ORIGIN_AWARE_TRAJECTORY_TRANSITION_VERSION: u32 = 1;

const POINT_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:origin-aware-trajectory-point:v1\0";
const FATE_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:origin-fate-delta:v1\0";
const TRANSITION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:origin-aware-trajectory-transition:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginAwarePopulationTrajectoryPoint {
    point_version: u32,
    schema_digest: HereditarySchemaDigest,
    state_digest: OriginAwarePopulationStateDigest,
    ordinary_point_digest: PopulationTrajectoryPointDigest,
    pub ordinary_point: PopulationTrajectoryPoint,
}

impl OriginAwarePopulationTrajectoryPoint {
    pub fn declare_reference_start(
        schema: &HereditarySchema,
        state: &OriginAwarePopulationState,
        experiment_id: EvolutionExperimentId,
        generation: PopulationGeneration,
    ) -> Result<Self, OriginAwareTrajectoryError> {
        state.validate_current(schema)?;
        let ordinary_point = PopulationTrajectoryPoint::declare_reference_start(
            schema,
            &state.population,
            experiment_id,
            generation,
        )?;
        Self::from_current(schema, state, ordinary_point)
    }

    pub fn from_current(
        schema: &HereditarySchema,
        state: &OriginAwarePopulationState,
        ordinary_point: PopulationTrajectoryPoint,
    ) -> Result<Self, OriginAwareTrajectoryError> {
        state.validate_current(schema)?;
        ordinary_point.validate_current(schema, &state.population)?;
        let point = Self {
            point_version: ORIGIN_AWARE_TRAJECTORY_POINT_VERSION,
            schema_digest: schema.canonical_digest()?,
            state_digest: state.canonical_digest(schema)?,
            ordinary_point_digest: ordinary_point.canonical_digest(),
            ordinary_point,
        };
        point.validate_current(schema, state)?;
        Ok(point)
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        state: &OriginAwarePopulationState,
    ) -> Result<(), OriginAwareTrajectoryError> {
        if self.point_version != ORIGIN_AWARE_TRAJECTORY_POINT_VERSION {
            return Err(OriginAwareTrajectoryError::UnsupportedPointVersion(
                self.point_version,
            ));
        }
        state.validate_current(schema)?;
        self.ordinary_point
            .validate_current(schema, &state.population)?;
        if self.schema_digest != schema.canonical_digest()?
            || self.state_digest != state.canonical_digest(schema)?
            || self.ordinary_point_digest != self.ordinary_point.canonical_digest()
        {
            return Err(OriginAwareTrajectoryError::PointAuthorityMismatch);
        }
        Ok(())
    }

    pub fn generation(&self) -> PopulationGeneration {
        self.ordinary_point.generation()
    }

    pub fn experiment_id(&self) -> &EvolutionExperimentId {
        self.ordinary_point.experiment_id()
    }

    pub fn canonical_digest(&self) -> OriginAwarePopulationTrajectoryPointDigest {
        let mut digest = Sha256::new();
        digest.update(POINT_DIGEST_DOMAIN);
        put_u32(&mut digest, self.point_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.state_digest.as_bytes());
        digest.update(self.ordinary_point_digest.as_bytes());
        OriginAwarePopulationTrajectoryPointDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OriginAwarePopulationTrajectoryPointDigest([u8; 32]);

impl OriginAwarePopulationTrajectoryPointDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for OriginAwarePopulationTrajectoryPointDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OriginAwarePopulationTrajectoryPointDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for OriginAwarePopulationTrajectoryPointDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationOriginFateDelta {
    pub locus_id: LocusId,
    pub allele_id: AlleleId,
    pub origin_digest: MutationOriginDigest,
    pub source_origin_count: u64,
    pub destination_origin_count: u64,
    pub source_allele_count: u64,
    pub destination_allele_count: u64,
    pub destination_locus_total: u64,
}

impl MutationOriginFateDelta {
    pub fn lost(&self) -> bool {
        self.source_origin_count > 0 && self.destination_origin_count == 0
    }

    pub fn persisting(&self) -> bool {
        self.destination_origin_count > 0
    }

    pub fn fixed_within_allele(&self) -> bool {
        self.destination_allele_count > 0
            && self.destination_origin_count == self.destination_allele_count
    }

    pub fn fixed_at_locus(&self) -> bool {
        self.destination_locus_total > 0
            && self.destination_origin_count == self.destination_locus_total
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeledBaselineFateDelta {
    pub locus_id: LocusId,
    pub allele_id: AlleleId,
    pub source_count: u64,
    pub destination_count: u64,
    pub source_allele_count: u64,
    pub destination_allele_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginFateDelta {
    delta_version: u32,
    source_state_digest: OriginAwarePopulationStateDigest,
    destination_state_digest: OriginAwarePopulationStateDigest,
    pub origins: Vec<MutationOriginFateDelta>,
    pub baselines: Vec<ModeledBaselineFateDelta>,
}

impl OriginFateDelta {
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        source: &OriginAwarePopulationState,
        destination: &OriginAwarePopulationState,
    ) -> Result<(), OriginAwareTrajectoryError> {
        let recomputed = derive_origin_fate_delta(schema, source, destination)?;
        if recomputed != *self {
            return Err(OriginAwareTrajectoryError::FateReplayMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> OriginFateDeltaDigest {
        let mut digest = Sha256::new();
        digest.update(FATE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.delta_version);
        digest.update(self.source_state_digest.as_bytes());
        digest.update(self.destination_state_digest.as_bytes());
        put_u64(&mut digest, self.origins.len() as u64);
        for entry in &self.origins {
            put_text(&mut digest, entry.locus_id.as_str());
            put_text(&mut digest, entry.allele_id.as_str());
            digest.update(entry.origin_digest.as_bytes());
            put_u64(&mut digest, entry.source_origin_count);
            put_u64(&mut digest, entry.destination_origin_count);
            put_u64(&mut digest, entry.source_allele_count);
            put_u64(&mut digest, entry.destination_allele_count);
            put_u64(&mut digest, entry.destination_locus_total);
        }
        put_u64(&mut digest, self.baselines.len() as u64);
        for entry in &self.baselines {
            put_text(&mut digest, entry.locus_id.as_str());
            put_text(&mut digest, entry.allele_id.as_str());
            put_u64(&mut digest, entry.source_count);
            put_u64(&mut digest, entry.destination_count);
            put_u64(&mut digest, entry.source_allele_count);
            put_u64(&mut digest, entry.destination_allele_count);
        }
        OriginFateDeltaDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OriginFateDeltaDigest([u8; 32]);

impl OriginFateDeltaDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for OriginFateDeltaDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OriginFateDeltaDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for OriginFateDeltaDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginAwareTrajectoryTransition {
    transition_version: u32,
    source_point_digest: OriginAwarePopulationTrajectoryPointDigest,
    destination_point_digest: OriginAwarePopulationTrajectoryPointDigest,
    fate_delta_digest: OriginFateDeltaDigest,
    pub transition: OriginAwarePopulationTransitionResult,
    pub destination_point: OriginAwarePopulationTrajectoryPoint,
    pub fate_delta: OriginFateDelta,
}

impl OriginAwareTrajectoryTransition {
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        source: &OriginAwarePopulationState,
        source_point: &OriginAwarePopulationTrajectoryPoint,
        transition_id: &PopulationTransitionId,
        profile: &PopulationProcessProfile,
    ) -> Result<(), OriginAwareTrajectoryError> {
        let recomputed = continue_origin_aware_trajectory(
            schema,
            source,
            source_point,
            transition_id,
            profile,
        )?;
        if recomputed != *self {
            return Err(OriginAwareTrajectoryError::TransitionReplayMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> OriginAwareTrajectoryTransitionDigest {
        let mut digest = Sha256::new();
        digest.update(TRANSITION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.transition_version);
        digest.update(self.source_point_digest.as_bytes());
        digest.update(self.destination_point_digest.as_bytes());
        digest.update(self.fate_delta_digest.as_bytes());
        digest.update(self.transition.provenance.canonical_digest().as_bytes());
        OriginAwareTrajectoryTransitionDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OriginAwareTrajectoryTransitionDigest([u8; 32]);

impl OriginAwareTrajectoryTransitionDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for OriginAwareTrajectoryTransitionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "OriginAwareTrajectoryTransitionDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for OriginAwareTrajectoryTransitionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

pub fn continue_origin_aware_trajectory(
    schema: &HereditarySchema,
    source: &OriginAwarePopulationState,
    source_point: &OriginAwarePopulationTrajectoryPoint,
    transition_id: &PopulationTransitionId,
    profile: &PopulationProcessProfile,
) -> Result<OriginAwareTrajectoryTransition, OriginAwareTrajectoryError> {
    source_point.validate_current(schema, source)?;
    let transition = neutral_origin_aware_population_step(
        schema,
        source,
        &source_point.ordinary_point,
        transition_id,
        profile,
    )?;
    let destination_point = OriginAwarePopulationTrajectoryPoint::from_current(
        schema,
        &transition.destination,
        transition.ordinary_transition.destination_point.clone(),
    )?;
    let fate_delta = derive_origin_fate_delta(schema, source, &transition.destination)?;
    let result = OriginAwareTrajectoryTransition {
        transition_version: ORIGIN_AWARE_TRAJECTORY_TRANSITION_VERSION,
        source_point_digest: source_point.canonical_digest(),
        destination_point_digest: destination_point.canonical_digest(),
        fate_delta_digest: fate_delta.canonical_digest(),
        transition,
        destination_point,
        fate_delta,
    };
    Ok(result)
}

pub fn derive_origin_fate_delta(
    schema: &HereditarySchema,
    source: &OriginAwarePopulationState,
    destination: &OriginAwarePopulationState,
) -> Result<OriginFateDelta, OriginAwareTrajectoryError> {
    source.validate_current(schema)?;
    destination.validate_current(schema)?;
    if source.population.population_id != destination.population.population_id
        || source.population.census_individuals != destination.population.census_individuals
    {
        return Err(OriginAwareTrajectoryError::PopulationContextMismatch);
    }

    let mut source_origin_context: BTreeMap<[u8; 32], (LocusId, AlleleId, u64)> = BTreeMap::new();
    let mut destination_origin_context: BTreeMap<[u8; 32], (LocusId, AlleleId, u64)> =
        BTreeMap::new();
    let mut source_baselines: BTreeMap<(LocusId, AlleleId), u64> = BTreeMap::new();
    let mut destination_baselines: BTreeMap<(LocusId, AlleleId), u64> = BTreeMap::new();

    collect_state_partitions(
        source,
        &mut source_origin_context,
        &mut source_baselines,
    )?;
    collect_state_partitions(
        destination,
        &mut destination_origin_context,
        &mut destination_baselines,
    )?;

    let mut origin_keys = BTreeMap::new();
    for (key, (locus, allele, _)) in &source_origin_context {
        origin_keys.insert(*key, (locus.clone(), allele.clone()));
    }
    for (key, (locus, allele, _)) in &destination_origin_context {
        if let Some((source_locus, source_allele)) = origin_keys.get(key) {
            if source_locus != locus || source_allele != allele {
                return Err(OriginAwareTrajectoryError::OriginContextDrift);
            }
        } else {
            origin_keys.insert(*key, (locus.clone(), allele.clone()));
        }
    }

    let mut origins = Vec::with_capacity(origin_keys.len());
    for (key, (locus_id, allele_id)) in origin_keys {
        let source_origin_count = source_origin_context.get(&key).map(|entry| entry.2).unwrap_or(0);
        let destination_origin_count = destination_origin_context
            .get(&key)
            .map(|entry| entry.2)
            .unwrap_or(0);
        let source_allele_count = allele_count(source, &locus_id, &allele_id);
        let destination_allele_count = allele_count(destination, &locus_id, &allele_id);
        let destination_locus_total = destination
            .population
            .allele_copy_counts
            .get(&locus_id)
            .map(|counts| counts.values().copied().sum())
            .unwrap_or(0);
        let origin_digest = source_origin_context
            .get(&key)
            .map(|_| find_origin_digest(source, &key))
            .transpose()?
            .flatten()
            .or_else(|| find_origin_digest(destination, &key).ok().flatten())
            .ok_or(OriginAwareTrajectoryError::OriginContextDrift)?;
        origins.push(MutationOriginFateDelta {
            locus_id,
            allele_id,
            origin_digest,
            source_origin_count,
            destination_origin_count,
            source_allele_count,
            destination_allele_count,
            destination_locus_total,
        });
    }

    let mut baseline_keys = BTreeMap::new();
    for key in source_baselines.keys() {
        baseline_keys.insert(key.clone(), ());
    }
    for key in destination_baselines.keys() {
        baseline_keys.insert(key.clone(), ());
    }
    let mut baselines = Vec::with_capacity(baseline_keys.len());
    for ((locus_id, allele_id), ()) in baseline_keys {
        baselines.push(ModeledBaselineFateDelta {
            source_count: source_baselines
                .get(&(locus_id.clone(), allele_id.clone()))
                .copied()
                .unwrap_or(0),
            destination_count: destination_baselines
                .get(&(locus_id.clone(), allele_id.clone()))
                .copied()
                .unwrap_or(0),
            source_allele_count: allele_count(source, &locus_id, &allele_id),
            destination_allele_count: allele_count(destination, &locus_id, &allele_id),
            locus_id,
            allele_id,
        });
    }

    Ok(OriginFateDelta {
        delta_version: ORIGIN_FATE_DELTA_VERSION,
        source_state_digest: source.canonical_digest(schema)?,
        destination_state_digest: destination.canonical_digest(schema)?,
        origins,
        baselines,
    })
}

fn collect_state_partitions(
    state: &OriginAwarePopulationState,
    origins: &mut BTreeMap<[u8; 32], (LocusId, AlleleId, u64)>,
    baselines: &mut BTreeMap<(LocusId, AlleleId), u64>,
) -> Result<(), OriginAwareTrajectoryError> {
    for locus in &state.loci {
        for allele in &locus.alleles {
            baselines.insert(
                (locus.locus_id.clone(), allele.allele_id.clone()),
                allele.modeled_baseline_count,
            );
            for origin in &allele.active_origin_counts {
                if origins
                    .insert(
                        *origin.origin_digest.as_bytes(),
                        (locus.locus_id.clone(), allele.allele_id.clone(), origin.count),
                    )
                    .is_some()
                {
                    return Err(OriginAwareTrajectoryError::OriginContextDrift);
                }
            }
        }
    }
    Ok(())
}

fn find_origin_digest(
    state: &OriginAwarePopulationState,
    key: &[u8; 32],
) -> Result<Option<MutationOriginDigest>, OriginAwareTrajectoryError> {
    for locus in &state.loci {
        for allele in &locus.alleles {
            if let Some(origin) = allele
                .active_origin_counts
                .iter()
                .find(|origin| origin.origin_digest.as_bytes() == key)
            {
                return Ok(Some(origin.origin_digest));
            }
        }
    }
    Ok(None)
}

fn allele_count(
    state: &OriginAwarePopulationState,
    locus_id: &LocusId,
    allele_id: &AlleleId,
) -> u64 {
    state
        .population
        .allele_copy_counts
        .get(locus_id)
        .and_then(|counts| counts.get(allele_id))
        .copied()
        .unwrap_or(0)
}

#[derive(Debug)]
pub enum OriginAwareTrajectoryError {
    Evolution(crate::EvolutionError),
    OriginAware(OriginAwarePopulationError),
    UnsupportedPointVersion(u32),
    PointAuthorityMismatch,
    PopulationContextMismatch,
    OriginContextDrift,
    FateReplayMismatch,
    TransitionReplayMismatch,
}

impl From<crate::EvolutionError> for OriginAwareTrajectoryError {
    fn from(value: crate::EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<OriginAwarePopulationError> for OriginAwareTrajectoryError {
    fn from(value: OriginAwarePopulationError) -> Self {
        Self::OriginAware(value)
    }
}

impl fmt::Display for OriginAwareTrajectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::OriginAware(error) => write!(f, "origin-aware population error: {error}"),
            Self::UnsupportedPointVersion(version) => {
                write!(f, "unsupported origin-aware trajectory point version {version}")
            }
            Self::PointAuthorityMismatch => {
                write!(f, "origin-aware trajectory point does not match current state")
            }
            Self::PopulationContextMismatch => {
                write!(f, "origin fate delta source/destination population context mismatch")
            }
            Self::OriginContextDrift => {
                write!(f, "one mutation origin changed locus/allele context")
            }
            Self::FateReplayMismatch => write!(f, "restored origin fate delta does not replay"),
            Self::TransitionReplayMismatch => {
                write!(f, "restored origin-aware trajectory transition does not replay")
            }
        }
    }
}

impl Error for OriginAwareTrajectoryError {}
