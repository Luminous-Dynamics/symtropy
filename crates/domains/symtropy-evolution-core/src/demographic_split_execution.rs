use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    demographic_history::DemographicEventExecutionDigest,
    AlleleId, DemographicEventDeclaration, DemographicEventDeclarationDigest,
    DemographicEventKind, DemographicInterventionCursor, DemographicInterventionCursorDigest,
    DemographicStructureTransition, DemographicStructureTransitionDigest, EvolutionError,
    EvolutionExperimentId, HereditarySchema, LocusId, MetapopulationSnapshot,
    MetapopulationSnapshotDigest, PopulationGeneration, PopulationGeneticState,
    PopulationGeneticStateDigest, PopulationId, PopulationStructureProfile,
    PopulationStructureProfileDigest, PopulationTrajectoryPoint, PopulationTrajectoryPointDigest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const SPLIT_PRIORITY_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-split-partition-priority:v1\0";
const SPLIT_EXECUTION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-split-execution:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemographicSplitExecutionModel {
    ConservativeIndependentLocusPartitionV1,
}

impl DemographicSplitExecutionModel {
    fn tag(self) -> u8 {
        match self {
            Self::ConservativeIndependentLocusPartitionV1 => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicSplitExecutionProvenance {
    model: DemographicSplitExecutionModel,
    event_digest: DemographicEventDeclarationDigest,
    structure_transition_digest: DemographicStructureTransitionDigest,
    source_structure_digest: PopulationStructureProfileDigest,
    successor_structure_digest: PopulationStructureProfileDigest,
    source_snapshot_digest: MetapopulationSnapshotDigest,
    source_cursor_digest: DemographicInterventionCursorDigest,
    result_snapshot_digest: MetapopulationSnapshotDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
    source_population_id: PopulationId,
    source_state_digest: PopulationGeneticStateDigest,
    source_point_digest: PopulationTrajectoryPointDigest,
    daughter_state_digests: BTreeMap<PopulationId, PopulationGeneticStateDigest>,
    daughter_point_digests: BTreeMap<PopulationId, PopulationTrajectoryPointDigest>,
}

impl DemographicSplitExecutionProvenance {
    pub fn model(&self) -> DemographicSplitExecutionModel {
        self.model
    }

    pub fn source_population_id(&self) -> &PopulationId {
        &self.source_population_id
    }

    pub fn daughter_state_digests(&self) -> &BTreeMap<PopulationId, PopulationGeneticStateDigest> {
        &self.daughter_state_digests
    }

    pub fn canonical_digest(&self) -> DemographicSplitExecutionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(SPLIT_EXECUTION_DIGEST_DOMAIN);
        digest.update([self.model.tag()]);
        digest.update(self.event_digest.as_bytes());
        digest.update(self.structure_transition_digest.as_bytes());
        digest.update(self.source_structure_digest.as_bytes());
        digest.update(self.successor_structure_digest.as_bytes());
        digest.update(self.source_snapshot_digest.as_bytes());
        digest.update(self.source_cursor_digest.as_bytes());
        digest.update(self.result_snapshot_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_u64(&mut digest, self.generation.0);
        put_text(&mut digest, self.source_population_id.as_str());
        digest.update(self.source_state_digest.as_bytes());
        digest.update(self.source_point_digest.as_bytes());
        put_u64(&mut digest, self.daughter_state_digests.len() as u64);
        for (population_id, state_digest) in &self.daughter_state_digests {
            put_text(&mut digest, population_id.as_str());
            digest.update(state_digest.as_bytes());
            let point_digest = self
                .daughter_point_digests
                .get(population_id)
                .expect("validated split provenance uses identical daughter key sets");
            digest.update(point_digest.as_bytes());
        }
        DemographicSplitExecutionProvenanceDigest(digest.finalize().into())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        source_structure: &PopulationStructureProfile,
        successor_structure: &PopulationStructureProfile,
        source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        source_snapshot: &MetapopulationSnapshot,
        source_cursor: &DemographicInterventionCursor,
        event: &DemographicEventDeclaration,
        structure_transition: &DemographicStructureTransition,
        result: &DemographicSplitExecutionResult,
    ) -> Result<(), EvolutionError> {
        source_cursor.validate_root_current(
            schema,
            source_structure,
            source_populations,
            source_points,
            source_snapshot,
        )?;
        event.validate_current(
            schema,
            source_structure,
            source_populations,
            source_points,
            source_snapshot,
        )?;
        structure_transition.validate_current(
            event,
            schema,
            source_structure,
            source_populations,
            source_points,
            source_snapshot,
            successor_structure,
        )?;

        let (source_id, daughters) = split_coordinates(event)?;
        validate_conservative_census(source_populations, source_id, daughters)?;
        if source_id != &self.source_population_id {
            return Err(EvolutionError::DemographicExecutionAuthorityMismatch);
        }
        if self.model != DemographicSplitExecutionModel::ConservativeIndependentLocusPartitionV1
            || event.canonical_digest()? != self.event_digest
            || structure_transition.canonical_digest() != self.structure_transition_digest
            || source_structure.canonical_digest()? != self.source_structure_digest
            || successor_structure.canonical_digest()? != self.successor_structure_digest
            || source_snapshot.canonical_digest() != self.source_snapshot_digest
            || source_cursor.canonical_digest()? != self.source_cursor_digest
            || event.experiment_id() != &self.experiment_id
            || event.generation() != self.generation
        {
            return Err(EvolutionError::DemographicExecutionAuthorityMismatch);
        }

        let source_state = source_populations
            .get(source_id)
            .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
                population: source_id.clone(),
            })?;
        let source_point = source_points
            .get(source_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if source_state.canonical_digest(schema)? != self.source_state_digest
            || source_point.canonical_digest() != self.source_point_digest
        {
            return Err(EvolutionError::DemographicExecutionSourceMismatch);
        }

        let derived = derive_split_result(
            schema,
            successor_structure,
            source_populations,
            source_points,
            event,
        )?;
        if derived.populations != result.populations
            || derived.points != result.points
            || derived.snapshot != result.snapshot
            || result.snapshot.canonical_digest() != self.result_snapshot_digest
        {
            return Err(EvolutionError::DemographicExecutionResultMismatch);
        }

        let mut daughter_state_digests = BTreeMap::new();
        let mut daughter_point_digests = BTreeMap::new();
        for daughter in daughters {
            let id = daughter.population_id();
            let state = result
                .populations
                .get(id)
                .ok_or(EvolutionError::MetapopulationSetMismatch)?;
            let point = result
                .points
                .get(id)
                .ok_or(EvolutionError::MetapopulationSetMismatch)?;
            daughter_state_digests.insert(id.clone(), state.canonical_digest(schema)?);
            daughter_point_digests.insert(id.clone(), point.canonical_digest());
        }
        if daughter_state_digests != self.daughter_state_digests
            || daughter_point_digests != self.daughter_point_digests
        {
            return Err(EvolutionError::DemographicExecutionResultMismatch);
        }

        let execution_digest = self.canonical_digest();
        let expected_cursor = DemographicInterventionCursor::advance_after_validated_execution(
            source_cursor,
            self.event_digest,
            self.structure_transition_digest,
            DemographicEventExecutionDigest(execution_digest.0),
            &result.snapshot,
        )?;
        if expected_cursor != result.history_cursor {
            return Err(EvolutionError::DemographicExecutionHistoryMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DemographicSplitExecutionProvenanceDigest(pub(crate) [u8; 32]);

impl DemographicSplitExecutionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DemographicSplitExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DemographicSplitExecutionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DemographicSplitExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicSplitExecutionResult {
    pub populations: BTreeMap<PopulationId, PopulationGeneticState>,
    pub points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    pub snapshot: MetapopulationSnapshot,
    pub history_cursor: DemographicInterventionCursor,
    pub provenance: DemographicSplitExecutionProvenance,
}

#[allow(clippy::too_many_arguments)]
pub fn execute_conservative_population_split(
    schema: &HereditarySchema,
    source_structure: &PopulationStructureProfile,
    successor_structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    source_snapshot: &MetapopulationSnapshot,
    source_cursor: &DemographicInterventionCursor,
    event: &DemographicEventDeclaration,
    structure_transition: &DemographicStructureTransition,
) -> Result<DemographicSplitExecutionResult, EvolutionError> {
    source_cursor.validate_root_current(
        schema,
        source_structure,
        source_populations,
        source_points,
        source_snapshot,
    )?;
    event.validate_current(
        schema,
        source_structure,
        source_populations,
        source_points,
        source_snapshot,
    )?;
    structure_transition.validate_current(
        event,
        schema,
        source_structure,
        source_populations,
        source_points,
        source_snapshot,
        successor_structure,
    )?;

    let (source_id, daughters) = split_coordinates(event)?;
    validate_conservative_census(source_populations, source_id, daughters)?;
    let source_state = source_populations
        .get(source_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: source_id.clone(),
        })?;
    let source_point = source_points
        .get(source_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;

    let derived = derive_split_result(
        schema,
        successor_structure,
        source_populations,
        source_points,
        event,
    )?;
    let mut daughter_state_digests = BTreeMap::new();
    let mut daughter_point_digests = BTreeMap::new();
    for daughter in daughters {
        let id = daughter.population_id();
        daughter_state_digests.insert(
            id.clone(),
            derived
                .populations
                .get(id)
                .ok_or(EvolutionError::MetapopulationSetMismatch)?
                .canonical_digest(schema)?,
        );
        daughter_point_digests.insert(
            id.clone(),
            derived
                .points
                .get(id)
                .ok_or(EvolutionError::MetapopulationSetMismatch)?
                .canonical_digest(),
        );
    }

    let provenance = DemographicSplitExecutionProvenance {
        model: DemographicSplitExecutionModel::ConservativeIndependentLocusPartitionV1,
        event_digest: event.canonical_digest()?,
        structure_transition_digest: structure_transition.canonical_digest(),
        source_structure_digest: source_structure.canonical_digest()?,
        successor_structure_digest: successor_structure.canonical_digest()?,
        source_snapshot_digest: source_snapshot.canonical_digest(),
        source_cursor_digest: source_cursor.canonical_digest()?,
        result_snapshot_digest: derived.snapshot.canonical_digest(),
        experiment_id: event.experiment_id().clone(),
        generation: event.generation(),
        source_population_id: source_id.clone(),
        source_state_digest: source_state.canonical_digest(schema)?,
        source_point_digest: source_point.canonical_digest(),
        daughter_state_digests,
        daughter_point_digests,
    };
    let execution_digest = provenance.canonical_digest();
    let history_cursor = DemographicInterventionCursor::advance_after_validated_execution(
        source_cursor,
        provenance.event_digest,
        provenance.structure_transition_digest,
        DemographicEventExecutionDigest(execution_digest.0),
        &derived.snapshot,
    )?;
    let result = DemographicSplitExecutionResult {
        populations: derived.populations,
        points: derived.points,
        snapshot: derived.snapshot,
        history_cursor,
        provenance,
    };
    result.provenance.validate_current(
        schema,
        source_structure,
        successor_structure,
        source_populations,
        source_points,
        source_snapshot,
        source_cursor,
        event,
        structure_transition,
        &result,
    )?;
    Ok(result)
}

struct DerivedSplitResult {
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
}

fn derive_split_result(
    schema: &HereditarySchema,
    successor_structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    event: &DemographicEventDeclaration,
) -> Result<DerivedSplitResult, EvolutionError> {
    let (source_id, daughters) = split_coordinates(event)?;
    validate_conservative_census(source_populations, source_id, daughters)?;
    let source = source_populations
        .get(source_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: source_id.clone(),
        })?;
    source.validate(schema)?;

    let mut daughter_counts: BTreeMap<
        PopulationId,
        BTreeMap<LocusId, BTreeMap<AlleleId, u64>>,
    > = daughters
        .iter()
        .map(|daughter| (daughter.population_id().clone(), BTreeMap::new()))
        .collect();

    for locus_id in schema.loci.keys() {
        let source_counts = source
            .allele_copy_counts
            .get(locus_id)
            .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
        let mut candidates = Vec::new();
        for (allele_id, count) in source_counts {
            for within_allele_ordinal in 0..*count {
                candidates.push((
                    split_priority(
                        event.experiment_id(),
                        event.event_id().as_str(),
                        event.generation(),
                        source_id,
                        locus_id,
                        allele_id,
                        within_allele_ordinal,
                    ),
                    allele_id.clone(),
                    within_allele_ordinal,
                ));
            }
        }
        candidates.sort();

        let mut offset = 0usize;
        for daughter in daughters {
            let capacity = daughter
                .census_individuals()
                .checked_mul(u64::from(schema.ploidy))
                .ok_or(EvolutionError::CountOverflow)?;
            let capacity = usize::try_from(capacity).map_err(|_| EvolutionError::CountOverflow)?;
            let end = offset
                .checked_add(capacity)
                .ok_or(EvolutionError::CountOverflow)?;
            if end > candidates.len() {
                return Err(EvolutionError::SamplingInvariantViolation);
            }
            let mut counts: BTreeMap<AlleleId, u64> = BTreeMap::new();
            for (_, allele, _) in &candidates[offset..end] {
                let count = counts.entry(allele.clone()).or_insert(0);
                *count = count.checked_add(1).ok_or(EvolutionError::CountOverflow)?;
            }
            daughter_counts
                .get_mut(daughter.population_id())
                .ok_or(EvolutionError::SamplingInvariantViolation)?
                .insert(locus_id.clone(), counts);
            offset = end;
        }
        if offset != candidates.len() {
            return Err(EvolutionError::SamplingInvariantViolation);
        }
    }

    let mut populations = source_populations.clone();
    let mut points = source_points.clone();
    if populations.remove(source_id).is_none() || points.remove(source_id).is_none() {
        return Err(EvolutionError::UnknownDemographicPopulation {
            population: source_id.clone(),
        });
    }

    for daughter in daughters {
        let id = daughter.population_id().clone();
        let counts = daughter_counts
            .remove(&id)
            .ok_or(EvolutionError::SamplingInvariantViolation)?;
        let state = PopulationGeneticState::from_counts(
            id.clone(),
            schema,
            daughter.census_individuals(),
            counts,
        )?;
        let point = PopulationTrajectoryPoint::from_validated_state(
            schema,
            &state,
            event.experiment_id().clone(),
            event.generation(),
        )?;
        if populations.insert(id.clone(), state).is_some()
            || points.insert(id.clone(), point).is_some()
        {
            return Err(EvolutionError::DemographicPopulationAlreadyExists { population: id });
        }
    }

    let snapshot = MetapopulationSnapshot::capture_reference(
        schema,
        successor_structure,
        &populations,
        &points,
    )?;
    Ok(DerivedSplitResult {
        populations,
        points,
        snapshot,
    })
}

fn validate_conservative_census(
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_id: &PopulationId,
    daughters: &[crate::DaughterPopulation],
) -> Result<(), EvolutionError> {
    let source_census = source_populations
        .get(source_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: source_id.clone(),
        })?
        .census_individuals;
    let mut daughter_total = 0u64;
    for daughter in daughters {
        daughter_total = daughter_total
            .checked_add(daughter.census_individuals())
            .ok_or(EvolutionError::CountOverflow)?;
    }
    if daughter_total != source_census {
        return Err(EvolutionError::DemographicExecutionAuthorityMismatch);
    }
    Ok(())
}

fn split_coordinates(
    event: &DemographicEventDeclaration,
) -> Result<(&PopulationId, &[crate::DaughterPopulation]), EvolutionError> {
    match event.kind() {
        DemographicEventKind::PopulationSplit { source, daughters } => Ok((source, daughters)),
        _ => Err(EvolutionError::DemographicEventKindUnsupportedForExecutor),
    }
}

fn split_priority(
    experiment_id: &EvolutionExperimentId,
    event_id: &str,
    generation: PopulationGeneration,
    source_population_id: &PopulationId,
    locus_id: &LocusId,
    allele_id: &AlleleId,
    within_allele_ordinal: u64,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(SPLIT_PRIORITY_DOMAIN);
    put_text(&mut digest, experiment_id.as_str());
    put_text(&mut digest, event_id);
    put_u64(&mut digest, generation.0);
    put_text(&mut digest, source_population_id.as_str());
    put_text(&mut digest, locus_id.as_str());
    put_text(&mut digest, allele_id.as_str());
    put_u64(&mut digest, within_allele_ordinal);
    digest.finalize().into()
}
