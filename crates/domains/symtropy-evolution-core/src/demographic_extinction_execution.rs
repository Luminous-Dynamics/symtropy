use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    demographic_history::DemographicEventExecutionDigest,
    DemographicEventDeclaration, DemographicEventDeclarationDigest, DemographicEventKind,
    DemographicInterventionCursor, DemographicInterventionCursorDigest,
    DemographicStructureTransition, DemographicStructureTransitionDigest, EvolutionError,
    EvolutionExperimentId, HereditarySchema, MetapopulationSnapshot, MetapopulationSnapshotDigest,
    PopulationGeneration, PopulationGeneticState, PopulationGeneticStateDigest, PopulationId,
    PopulationStructureProfile, PopulationStructureProfileDigest, PopulationTrajectoryPoint,
    PopulationTrajectoryPointDigest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const EXTINCTION_EXECUTION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-extinction-execution:v1\0";

/// Exact V0 execution model for structural population extinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemographicExtinctionExecutionModel {
    StructuralPopulationRemovalV1,
}

impl DemographicExtinctionExecutionModel {
    fn tag(self) -> u8 {
        match self {
            Self::StructuralPopulationRemovalV1 => 0,
        }
    }
}

/// Revalidatable historical receipt for one extinct aggregate population.
///
/// The extinct population is removed from live state, while its final validated
/// state/trajectory digests remain permanently bound here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicExtinctionExecutionProvenance {
    model: DemographicExtinctionExecutionModel,
    event_digest: DemographicEventDeclarationDigest,
    structure_transition_digest: DemographicStructureTransitionDigest,
    source_structure_digest: PopulationStructureProfileDigest,
    successor_structure_digest: PopulationStructureProfileDigest,
    source_snapshot_digest: MetapopulationSnapshotDigest,
    source_cursor_digest: DemographicInterventionCursorDigest,
    result_snapshot_digest: MetapopulationSnapshotDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
    extinct_population_id: PopulationId,
    last_live_state_digest: PopulationGeneticStateDigest,
    last_live_point_digest: PopulationTrajectoryPointDigest,
}

impl DemographicExtinctionExecutionProvenance {
    pub fn model(&self) -> DemographicExtinctionExecutionModel {
        self.model
    }

    pub fn extinct_population_id(&self) -> &PopulationId {
        &self.extinct_population_id
    }

    pub fn last_live_state_digest(&self) -> PopulationGeneticStateDigest {
        self.last_live_state_digest
    }

    pub fn last_live_point_digest(&self) -> PopulationTrajectoryPointDigest {
        self.last_live_point_digest
    }

    pub fn source_snapshot_digest(&self) -> MetapopulationSnapshotDigest {
        self.source_snapshot_digest
    }

    pub fn result_snapshot_digest(&self) -> MetapopulationSnapshotDigest {
        self.result_snapshot_digest
    }

    pub fn canonical_digest(&self) -> DemographicExtinctionExecutionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(EXTINCTION_EXECUTION_DIGEST_DOMAIN);
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
        put_text(&mut digest, self.extinct_population_id.as_str());
        digest.update(self.last_live_state_digest.as_bytes());
        digest.update(self.last_live_point_digest.as_bytes());
        DemographicExtinctionExecutionProvenanceDigest(digest.finalize().into())
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
        result: &DemographicExtinctionExecutionResult,
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

        let event_population = extinction_population(event)?;
        if event_population != &self.extinct_population_id {
            return Err(EvolutionError::DemographicExtinctionExecutionAuthorityMismatch);
        }
        if self.model != DemographicExtinctionExecutionModel::StructuralPopulationRemovalV1
            || event.canonical_digest()? != self.event_digest
            || structure_transition.canonical_digest() != self.structure_transition_digest
            || source_structure.canonical_digest()? != self.source_structure_digest
            || successor_structure.canonical_digest()? != self.successor_structure_digest
            || source_snapshot.canonical_digest() != self.source_snapshot_digest
            || source_cursor.canonical_digest()? != self.source_cursor_digest
            || event.experiment_id() != &self.experiment_id
            || event.generation() != self.generation
        {
            return Err(EvolutionError::DemographicExtinctionExecutionAuthorityMismatch);
        }

        let last_state = source_populations
            .get(event_population)
            .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
                population: event_population.clone(),
            })?;
        let last_point = source_points
            .get(event_population)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if last_state.canonical_digest(schema)? != self.last_live_state_digest
            || last_point.canonical_digest() != self.last_live_point_digest
        {
            return Err(EvolutionError::DemographicExtinctionExecutionSourceMismatch);
        }

        let derived = derive_extinction_result(
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
            return Err(EvolutionError::DemographicExtinctionExecutionResultMismatch);
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
            return Err(EvolutionError::DemographicExtinctionExecutionHistoryMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DemographicExtinctionExecutionProvenanceDigest(pub(crate) [u8; 32]);

impl DemographicExtinctionExecutionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DemographicExtinctionExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DemographicExtinctionExecutionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DemographicExtinctionExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicExtinctionExecutionResult {
    pub populations: BTreeMap<PopulationId, PopulationGeneticState>,
    pub points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    pub snapshot: MetapopulationSnapshot,
    pub history_cursor: DemographicInterventionCursor,
    pub provenance: DemographicExtinctionExecutionProvenance,
}

/// Execute one deterministic structural population extinction at generation G.
#[allow(clippy::too_many_arguments)]
pub fn execute_structural_extinction(
    schema: &HereditarySchema,
    source_structure: &PopulationStructureProfile,
    successor_structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    source_snapshot: &MetapopulationSnapshot,
    source_cursor: &DemographicInterventionCursor,
    event: &DemographicEventDeclaration,
    structure_transition: &DemographicStructureTransition,
) -> Result<DemographicExtinctionExecutionResult, EvolutionError> {
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

    let extinct_population = extinction_population(event)?.clone();
    let last_state = source_populations
        .get(&extinct_population)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: extinct_population.clone(),
        })?;
    let last_point = source_points
        .get(&extinct_population)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;

    let derived = derive_extinction_result(
        schema,
        successor_structure,
        source_populations,
        source_points,
        event,
    )?;
    let provenance = DemographicExtinctionExecutionProvenance {
        model: DemographicExtinctionExecutionModel::StructuralPopulationRemovalV1,
        event_digest: event.canonical_digest()?,
        structure_transition_digest: structure_transition.canonical_digest(),
        source_structure_digest: source_structure.canonical_digest()?,
        successor_structure_digest: successor_structure.canonical_digest()?,
        source_snapshot_digest: source_snapshot.canonical_digest(),
        source_cursor_digest: source_cursor.canonical_digest()?,
        result_snapshot_digest: derived.snapshot.canonical_digest(),
        experiment_id: event.experiment_id().clone(),
        generation: event.generation(),
        extinct_population_id: extinct_population,
        last_live_state_digest: last_state.canonical_digest(schema)?,
        last_live_point_digest: last_point.canonical_digest(),
    };
    let execution_digest = provenance.canonical_digest();
    let history_cursor = DemographicInterventionCursor::advance_after_validated_execution(
        source_cursor,
        provenance.event_digest,
        provenance.structure_transition_digest,
        DemographicEventExecutionDigest(execution_digest.0),
        &derived.snapshot,
    )?;
    let result = DemographicExtinctionExecutionResult {
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

struct DerivedExtinctionResult {
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
}

fn derive_extinction_result(
    schema: &HereditarySchema,
    successor_structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    event: &DemographicEventDeclaration,
) -> Result<DerivedExtinctionResult, EvolutionError> {
    let extinct_population = extinction_population(event)?;
    let mut populations = source_populations.clone();
    let mut points = source_points.clone();
    if populations.remove(extinct_population).is_none()
        || points.remove(extinct_population).is_none()
    {
        return Err(EvolutionError::UnknownDemographicPopulation {
            population: extinct_population.clone(),
        });
    }

    let snapshot = MetapopulationSnapshot::capture_reference(
        schema,
        successor_structure,
        &populations,
        &points,
    )?;
    Ok(DerivedExtinctionResult {
        populations,
        points,
        snapshot,
    })
}

fn extinction_population(event: &DemographicEventDeclaration) -> Result<&PopulationId, EvolutionError> {
    match event.kind() {
        DemographicEventKind::Extinction { population } => Ok(population),
        _ => Err(EvolutionError::DemographicEventKindUnsupportedForExecutor),
    }
}
