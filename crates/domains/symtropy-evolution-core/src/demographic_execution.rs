use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    demographic_history::DemographicEventExecutionDigest,
    AlleleId, DemographicEventDeclaration, DemographicEventDeclarationDigest,
    DemographicEventKind, DemographicInterventionCursor, DemographicInterventionCursorDigest,
    DemographicStructureTransition, DemographicStructureTransitionDigest, EvolutionError,
    EvolutionExperimentId, HereditarySchema, LocusId, MetapopulationSnapshot,
    MetapopulationSnapshotDigest, PopulationGeneration, PopulationGeneticState,
    PopulationGeneticStateDigest, PopulationId, PopulationStructureProfile,
    PopulationTrajectoryPoint, PopulationTrajectoryPointDigest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const BOTTLENECK_PRIORITY_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-random-survivor-priority:v1\0";
const DEMOGRAPHIC_EXECUTION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-event-execution:v1\0";

/// Exact V0 execution model for a marginal random-survivor bottleneck.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemographicEventExecutionModel {
    IndependentLocusRandomSurvivorBottleneckV1,
}

impl DemographicEventExecutionModel {
    fn tag(self) -> u8 {
        match self {
            Self::IndependentLocusRandomSurvivorBottleneckV1 => 0,
        }
    }
}

/// Revalidatable receipt for one executed aggregate demographic intervention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicEventExecutionProvenance {
    model: DemographicEventExecutionModel,
    event_digest: DemographicEventDeclarationDigest,
    structure_transition_digest: DemographicStructureTransitionDigest,
    source_snapshot_digest: MetapopulationSnapshotDigest,
    source_cursor_digest: DemographicInterventionCursorDigest,
    result_snapshot_digest: MetapopulationSnapshotDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
    population_id: PopulationId,
    source_state_digest: PopulationGeneticStateDigest,
    result_state_digest: PopulationGeneticStateDigest,
    source_point_digest: PopulationTrajectoryPointDigest,
    result_point_digest: PopulationTrajectoryPointDigest,
}

impl DemographicEventExecutionProvenance {
    pub fn model(&self) -> DemographicEventExecutionModel {
        self.model
    }

    pub fn event_digest(&self) -> DemographicEventDeclarationDigest {
        self.event_digest
    }

    pub fn source_snapshot_digest(&self) -> MetapopulationSnapshotDigest {
        self.source_snapshot_digest
    }

    pub fn result_snapshot_digest(&self) -> MetapopulationSnapshotDigest {
        self.result_snapshot_digest
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn canonical_digest(&self) -> DemographicEventExecutionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(DEMOGRAPHIC_EXECUTION_DIGEST_DOMAIN);
        digest.update([self.model.tag()]);
        digest.update(self.event_digest.as_bytes());
        digest.update(self.structure_transition_digest.as_bytes());
        digest.update(self.source_snapshot_digest.as_bytes());
        digest.update(self.source_cursor_digest.as_bytes());
        digest.update(self.result_snapshot_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_u64(&mut digest, self.generation.0);
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.source_state_digest.as_bytes());
        digest.update(self.result_state_digest.as_bytes());
        digest.update(self.source_point_digest.as_bytes());
        digest.update(self.result_point_digest.as_bytes());
        DemographicEventExecutionProvenanceDigest(digest.finalize().into())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        source_snapshot: &MetapopulationSnapshot,
        source_cursor: &DemographicInterventionCursor,
        event: &DemographicEventDeclaration,
        structure_transition: &DemographicStructureTransition,
        result: &DemographicEventExecutionResult,
    ) -> Result<(), EvolutionError> {
        source_cursor.validate_root_current(
            schema,
            structure,
            source_populations,
            source_points,
            source_snapshot,
        )?;
        event.validate_current(
            schema,
            structure,
            source_populations,
            source_points,
            source_snapshot,
        )?;
        structure_transition.validate_current(
            event,
            schema,
            structure,
            source_populations,
            source_points,
            source_snapshot,
            structure,
        )?;

        if self.model != DemographicEventExecutionModel::IndependentLocusRandomSurvivorBottleneckV1
            || event.canonical_digest()? != self.event_digest
            || structure_transition.canonical_digest() != self.structure_transition_digest
            || source_snapshot.canonical_digest() != self.source_snapshot_digest
            || source_cursor.canonical_digest()? != self.source_cursor_digest
            || event.experiment_id() != &self.experiment_id
            || event.generation() != self.generation
        {
            return Err(EvolutionError::DemographicExecutionAuthorityMismatch);
        }

        let source_state = source_populations
            .get(&self.population_id)
            .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
                population: self.population_id.clone(),
            })?;
        let source_point = source_points
            .get(&self.population_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if source_state.canonical_digest(schema)? != self.source_state_digest
            || source_point.canonical_digest() != self.source_point_digest
        {
            return Err(EvolutionError::DemographicExecutionSourceMismatch);
        }

        let derived = derive_census_resize(
            schema,
            structure,
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

        let result_state = result
            .populations
            .get(&self.population_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        let result_point = result
            .points
            .get(&self.population_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if result_state.canonical_digest(schema)? != self.result_state_digest
            || result_point.canonical_digest() != self.result_point_digest
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
pub struct DemographicEventExecutionProvenanceDigest(pub(crate) [u8; 32]);

impl DemographicEventExecutionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DemographicEventExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DemographicEventExecutionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DemographicEventExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicEventExecutionResult {
    pub populations: BTreeMap<PopulationId, PopulationGeneticState>,
    pub points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    pub snapshot: MetapopulationSnapshot,
    pub history_cursor: DemographicInterventionCursor,
    pub provenance: DemographicEventExecutionProvenance,
}

/// Execute one source-bound random-survivor census bottleneck.
///
/// V0 accepts only an ordinal-zero history cursor and only a `CensusResize`
/// whose target is not larger than the current census. Biological generation
/// time is unchanged by this intervention.
#[allow(clippy::too_many_arguments)]
pub fn execute_census_resize_bottleneck(
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    source_snapshot: &MetapopulationSnapshot,
    source_cursor: &DemographicInterventionCursor,
    event: &DemographicEventDeclaration,
    structure_transition: &DemographicStructureTransition,
) -> Result<DemographicEventExecutionResult, EvolutionError> {
    source_cursor.validate_root_current(
        schema,
        structure,
        source_populations,
        source_points,
        source_snapshot,
    )?;
    event.validate_current(
        schema,
        structure,
        source_populations,
        source_points,
        source_snapshot,
    )?;
    structure_transition.validate_current(
        event,
        schema,
        structure,
        source_populations,
        source_points,
        source_snapshot,
        structure,
    )?;

    let population_id = match event.kind() {
        DemographicEventKind::CensusResize { population, .. } => population.clone(),
        _ => return Err(EvolutionError::DemographicEventKindUnsupportedForExecutor),
    };
    let source_state = source_populations
        .get(&population_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: population_id.clone(),
        })?;
    let source_point = source_points
        .get(&population_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;

    let derived = derive_census_resize(
        schema,
        structure,
        source_populations,
        source_points,
        event,
    )?;
    let result_state = derived
        .populations
        .get(&population_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;
    let result_point = derived
        .points
        .get(&population_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;

    let provenance = DemographicEventExecutionProvenance {
        model: DemographicEventExecutionModel::IndependentLocusRandomSurvivorBottleneckV1,
        event_digest: event.canonical_digest()?,
        structure_transition_digest: structure_transition.canonical_digest(),
        source_snapshot_digest: source_snapshot.canonical_digest(),
        source_cursor_digest: source_cursor.canonical_digest()?,
        result_snapshot_digest: derived.snapshot.canonical_digest(),
        experiment_id: event.experiment_id().clone(),
        generation: event.generation(),
        population_id: population_id.clone(),
        source_state_digest: source_state.canonical_digest(schema)?,
        result_state_digest: result_state.canonical_digest(schema)?,
        source_point_digest: source_point.canonical_digest(),
        result_point_digest: result_point.canonical_digest(),
    };
    let execution_digest = provenance.canonical_digest();
    let history_cursor = DemographicInterventionCursor::advance_after_validated_execution(
        source_cursor,
        provenance.event_digest,
        provenance.structure_transition_digest,
        DemographicEventExecutionDigest(execution_digest.0),
        &derived.snapshot,
    )?;
    let result = DemographicEventExecutionResult {
        populations: derived.populations,
        points: derived.points,
        snapshot: derived.snapshot,
        history_cursor,
        provenance,
    };
    result.provenance.validate_current(
        schema,
        structure,
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

struct DerivedPostEvent {
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
}

fn derive_census_resize(
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    event: &DemographicEventDeclaration,
) -> Result<DerivedPostEvent, EvolutionError> {
    let (population_id, target_census) = match event.kind() {
        DemographicEventKind::CensusResize {
            population,
            target_census,
        } => (population, *target_census),
        _ => return Err(EvolutionError::DemographicEventKindUnsupportedForExecutor),
    };
    let source = source_populations
        .get(population_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: population_id.clone(),
        })?;
    let source_point = source_points
        .get(population_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;
    source.validate(schema)?;
    source_point.validate_current(schema, source)?;

    if target_census > source.census_individuals {
        return Err(EvolutionError::DemographicExpansionUnsupported {
            current_census: source.census_individuals,
            target_census,
        });
    }

    let mut populations = source_populations.clone();
    let mut points = source_points.clone();
    if target_census < source.census_individuals {
        let downsampled = downsample_population_without_replacement(
            schema,
            source,
            event.experiment_id(),
            event.event_id().as_str(),
            event.generation(),
            target_census,
        )?;
        let point = PopulationTrajectoryPoint::from_validated_state(
            schema,
            &downsampled,
            event.experiment_id().clone(),
            event.generation(),
        )?;
        populations.insert(population_id.clone(), downsampled);
        points.insert(population_id.clone(), point);
    }

    let snapshot =
        MetapopulationSnapshot::capture_reference(schema, structure, &populations, &points)?;
    Ok(DerivedPostEvent {
        populations,
        points,
        snapshot,
    })
}

/// Sample exact marginal allele-copy totals without replacement at each locus.
fn downsample_population_without_replacement(
    schema: &HereditarySchema,
    source: &PopulationGeneticState,
    experiment_id: &EvolutionExperimentId,
    event_id: &str,
    generation: PopulationGeneration,
    target_census: u64,
) -> Result<PopulationGeneticState, EvolutionError> {
    let target_copies = target_census
        .checked_mul(u64::from(schema.ploidy))
        .ok_or(EvolutionError::CountOverflow)?;
    let target_len = usize::try_from(target_copies).map_err(|_| EvolutionError::CountOverflow)?;
    let mut destination_counts = BTreeMap::new();

    for locus_id in schema.loci.keys() {
        let source_counts = source
            .allele_copy_counts
            .get(locus_id)
            .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
        let mut candidates = Vec::new();
        for (allele_id, count) in source_counts {
            for within_allele_ordinal in 0..*count {
                candidates.push((
                    survivor_priority(
                        experiment_id,
                        event_id,
                        generation,
                        &source.population_id,
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
        if target_len > candidates.len() {
            return Err(EvolutionError::SamplingInvariantViolation);
        }

        let mut counts: BTreeMap<AlleleId, u64> = BTreeMap::new();
        for (_, allele, _) in candidates.into_iter().take(target_len) {
            let count = counts.entry(allele).or_insert(0);
            *count = count.checked_add(1).ok_or(EvolutionError::CountOverflow)?;
        }
        destination_counts.insert(locus_id.clone(), counts);
    }

    PopulationGeneticState::from_counts(
        source.population_id.clone(),
        schema,
        target_census,
        destination_counts,
    )
}

/// Full SHA-256 survivor priority. The target census is deliberately absent so
/// severity sweeps move a cutoff over one common opportunity ordering.
fn survivor_priority(
    experiment_id: &EvolutionExperimentId,
    event_id: &str,
    generation: PopulationGeneration,
    population_id: &PopulationId,
    locus_id: &LocusId,
    allele_id: &AlleleId,
    within_allele_ordinal: u64,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(BOTTLENECK_PRIORITY_DOMAIN);
    put_text(&mut digest, experiment_id.as_str());
    put_text(&mut digest, event_id);
    put_u64(&mut digest, generation.0);
    put_text(&mut digest, population_id.as_str());
    put_text(&mut digest, locus_id.as_str());
    put_text(&mut digest, allele_id.as_str());
    put_u64(&mut digest, within_allele_ordinal);
    digest.finalize().into()
}
