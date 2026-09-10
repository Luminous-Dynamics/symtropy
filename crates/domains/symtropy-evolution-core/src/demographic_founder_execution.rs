use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    demographic_history::DemographicEventExecutionDigest,
    demographic_sampling::sample_marginal_without_replacement,
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

const FOUNDER_PRIORITY_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-founder-propagule-priority:v1\0";
const FOUNDER_EXECUTION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-founder-execution:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemographicFounderExecutionModel {
    IndependentLocusNonDepletingFounderSampleV1,
}

impl DemographicFounderExecutionModel {
    fn tag(self) -> u8 {
        match self {
            Self::IndependentLocusNonDepletingFounderSampleV1 => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicFounderExecutionProvenance {
    model: DemographicFounderExecutionModel,
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
    founded_population_id: PopulationId,
    source_state_digest: PopulationGeneticStateDigest,
    source_point_digest: PopulationTrajectoryPointDigest,
    founded_state_digest: PopulationGeneticStateDigest,
    founded_point_digest: PopulationTrajectoryPointDigest,
}

impl DemographicFounderExecutionProvenance {
    pub fn model(&self) -> DemographicFounderExecutionModel {
        self.model
    }

    pub fn source_population_id(&self) -> &PopulationId {
        &self.source_population_id
    }

    pub fn founded_population_id(&self) -> &PopulationId {
        &self.founded_population_id
    }

    pub fn canonical_digest(&self) -> DemographicFounderExecutionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(FOUNDER_EXECUTION_DIGEST_DOMAIN);
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
        put_text(&mut digest, self.founded_population_id.as_str());
        digest.update(self.source_state_digest.as_bytes());
        digest.update(self.source_point_digest.as_bytes());
        digest.update(self.founded_state_digest.as_bytes());
        digest.update(self.founded_point_digest.as_bytes());
        DemographicFounderExecutionProvenanceDigest(digest.finalize().into())
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
        result: &DemographicFounderExecutionResult,
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

        let (source_id, destination_id, _) = founder_event_coordinates(event)?;
        if source_id != &self.source_population_id || destination_id != &self.founded_population_id {
            return Err(EvolutionError::DemographicFounderExecutionAuthorityMismatch);
        }
        if self.model
            != DemographicFounderExecutionModel::IndependentLocusNonDepletingFounderSampleV1
            || event.canonical_digest()? != self.event_digest
            || structure_transition.canonical_digest() != self.structure_transition_digest
            || source_structure.canonical_digest()? != self.source_structure_digest
            || successor_structure.canonical_digest()? != self.successor_structure_digest
            || source_snapshot.canonical_digest() != self.source_snapshot_digest
            || source_cursor.canonical_digest()? != self.source_cursor_digest
            || event.experiment_id() != &self.experiment_id
            || event.generation() != self.generation
        {
            return Err(EvolutionError::DemographicFounderExecutionAuthorityMismatch);
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
            return Err(EvolutionError::DemographicFounderExecutionSourceMismatch);
        }

        let derived = derive_founder_result(
            schema,
            source_structure,
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
            return Err(EvolutionError::DemographicFounderExecutionResultMismatch);
        }

        let founded_state = result
            .populations
            .get(destination_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        let founded_point = result
            .points
            .get(destination_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if founded_state.canonical_digest(schema)? != self.founded_state_digest
            || founded_point.canonical_digest() != self.founded_point_digest
        {
            return Err(EvolutionError::DemographicFounderExecutionResultMismatch);
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
            return Err(EvolutionError::DemographicFounderExecutionHistoryMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DemographicFounderExecutionProvenanceDigest(pub(crate) [u8; 32]);

impl DemographicFounderExecutionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DemographicFounderExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DemographicFounderExecutionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DemographicFounderExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicFounderExecutionResult {
    pub populations: BTreeMap<PopulationId, PopulationGeneticState>,
    pub points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    pub snapshot: MetapopulationSnapshot,
    pub history_cursor: DemographicInterventionCursor,
    pub provenance: DemographicFounderExecutionProvenance,
}

#[allow(clippy::too_many_arguments)]
pub fn execute_founder_or_recolonization_sample(
    schema: &HereditarySchema,
    source_structure: &PopulationStructureProfile,
    successor_structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    source_snapshot: &MetapopulationSnapshot,
    source_cursor: &DemographicInterventionCursor,
    event: &DemographicEventDeclaration,
    structure_transition: &DemographicStructureTransition,
) -> Result<DemographicFounderExecutionResult, EvolutionError> {
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

    let (source_id, destination_id, founder_census) = founder_event_coordinates(event)?;
    let source_state = source_populations
        .get(source_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: source_id.clone(),
        })?;
    let source_point = source_points
        .get(source_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;
    if founder_census > source_state.census_individuals {
        return Err(EvolutionError::DemographicSampleExceedsSourceCensus {
            source_census: source_state.census_individuals,
            target_census: founder_census,
        });
    }

    let derived = derive_founder_result(
        schema,
        source_structure,
        successor_structure,
        source_populations,
        source_points,
        event,
    )?;
    let founded_state = derived
        .populations
        .get(destination_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;
    let founded_point = derived
        .points
        .get(destination_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;

    let provenance = DemographicFounderExecutionProvenance {
        model: DemographicFounderExecutionModel::IndependentLocusNonDepletingFounderSampleV1,
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
        founded_population_id: destination_id.clone(),
        source_state_digest: source_state.canonical_digest(schema)?,
        source_point_digest: source_point.canonical_digest(),
        founded_state_digest: founded_state.canonical_digest(schema)?,
        founded_point_digest: founded_point.canonical_digest(),
    };
    let execution_digest = provenance.canonical_digest();
    let history_cursor = DemographicInterventionCursor::advance_after_validated_execution(
        source_cursor,
        provenance.event_digest,
        provenance.structure_transition_digest,
        DemographicEventExecutionDigest(execution_digest.0),
        &derived.snapshot,
    )?;
    let result = DemographicFounderExecutionResult {
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

struct DerivedFounderResult {
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
}

fn derive_founder_result(
    schema: &HereditarySchema,
    _source_structure: &PopulationStructureProfile,
    successor_structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    event: &DemographicEventDeclaration,
) -> Result<DerivedFounderResult, EvolutionError> {
    let (source_id, destination_id, founder_census) = founder_event_coordinates(event)?;
    let source = source_populations
        .get(source_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: source_id.clone(),
        })?;
    if founder_census > source.census_individuals {
        return Err(EvolutionError::DemographicSampleExceedsSourceCensus {
            source_census: source.census_individuals,
            target_census: founder_census,
        });
    }

    let founded = sample_marginal_without_replacement(
        schema,
        source,
        destination_id.clone(),
        founder_census,
        |locus_id, allele_id, within_allele_ordinal| {
            founder_priority(
                event.experiment_id(),
                event.event_id().as_str(),
                event.generation(),
                source_id,
                destination_id,
                locus_id,
                allele_id,
                within_allele_ordinal,
            )
        },
    )?;
    let founded_point = PopulationTrajectoryPoint::from_validated_state(
        schema,
        &founded,
        event.experiment_id().clone(),
        event.generation(),
    )?;

    let mut populations = source_populations.clone();
    let mut points = source_points.clone();
    if populations.insert(destination_id.clone(), founded).is_some()
        || points.insert(destination_id.clone(), founded_point).is_some()
    {
        return Err(EvolutionError::DemographicPopulationAlreadyExists {
            population: destination_id.clone(),
        });
    }

    let snapshot = MetapopulationSnapshot::capture_reference(
        schema,
        successor_structure,
        &populations,
        &points,
    )?;
    Ok(DerivedFounderResult {
        populations,
        points,
        snapshot,
    })
}

fn founder_event_coordinates(
    event: &DemographicEventDeclaration,
) -> Result<(&PopulationId, &PopulationId, u64), EvolutionError> {
    match event.kind() {
        DemographicEventKind::FounderEvent {
            source,
            founded_population,
            founder_census,
        } => Ok((source, founded_population, *founder_census)),
        DemographicEventKind::Recolonization {
            source,
            recolonized_population,
            founder_census,
        } => Ok((source, recolonized_population, *founder_census)),
        _ => Err(EvolutionError::DemographicEventKindUnsupportedForExecutor),
    }
}

#[allow(clippy::too_many_arguments)]
fn founder_priority(
    experiment_id: &EvolutionExperimentId,
    event_id: &str,
    generation: PopulationGeneration,
    source_population_id: &PopulationId,
    destination_population_id: &PopulationId,
    locus_id: &LocusId,
    allele_id: &AlleleId,
    within_allele_ordinal: u64,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(FOUNDER_PRIORITY_DOMAIN);
    put_text(&mut digest, experiment_id.as_str());
    put_text(&mut digest, event_id);
    put_u64(&mut digest, generation.0);
    put_text(&mut digest, source_population_id.as_str());
    put_text(&mut digest, destination_population_id.as_str());
    put_text(&mut digest, locus_id.as_str());
    put_text(&mut digest, allele_id.as_str());
    put_u64(&mut digest, within_allele_ordinal);
    digest.finalize().into()
}
