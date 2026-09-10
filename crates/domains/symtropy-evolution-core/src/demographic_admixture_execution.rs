use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    demographic_history::DemographicEventExecutionDigest,
    demographic_sampling::sample_marginal_copy_counts_without_replacement,
    DemographicEventDeclaration, DemographicEventDeclarationDigest, DemographicEventKind,
    DemographicInterventionCursor, DemographicInterventionCursorDigest,
    DemographicStructureTransition, DemographicStructureTransitionDigest, EvolutionError,
    EvolutionExperimentId, HereditarySchema, LocusId, MetapopulationSnapshot,
    MetapopulationSnapshotDigest, PopulationGeneration, PopulationGeneticState,
    PopulationGeneticStateDigest, PopulationId, PopulationStructureProfile,
    PopulationStructureProfileDigest, PopulationTrajectoryPoint, PopulationTrajectoryPointDigest,
    PROBABILITY_SCALE_PPM,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const ADMIXTURE_REMOVAL_PRIORITY_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-pulse-admixture-removal-priority:v1\0";
const ADMIXTURE_SOURCE_PRIORITY_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-pulse-admixture-source-priority:v1\0";
const ADMIXTURE_EXECUTION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-pulse-admixture-execution:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemographicAdmixtureExecutionModel {
    CensusPreservingMarginalReplacementV1,
}

impl DemographicAdmixtureExecutionModel {
    fn tag(self) -> u8 {
        match self {
            Self::CensusPreservingMarginalReplacementV1 => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicAdmixtureExecutionProvenance {
    model: DemographicAdmixtureExecutionModel,
    event_digest: DemographicEventDeclarationDigest,
    structure_transition_digest: DemographicStructureTransitionDigest,
    structure_digest: PopulationStructureProfileDigest,
    source_snapshot_digest: MetapopulationSnapshotDigest,
    source_cursor_digest: DemographicInterventionCursorDigest,
    result_snapshot_digest: MetapopulationSnapshotDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
    source_population_id: PopulationId,
    destination_population_id: PopulationId,
    declared_source_fraction_ppm: u32,
    realized_replacement_copies: u64,
    source_state_digest: PopulationGeneticStateDigest,
    source_point_digest: PopulationTrajectoryPointDigest,
    destination_source_state_digest: PopulationGeneticStateDigest,
    destination_source_point_digest: PopulationTrajectoryPointDigest,
    destination_result_state_digest: PopulationGeneticStateDigest,
    destination_result_point_digest: PopulationTrajectoryPointDigest,
}

impl DemographicAdmixtureExecutionProvenance {
    pub fn model(&self) -> DemographicAdmixtureExecutionModel {
        self.model
    }

    pub fn source_population_id(&self) -> &PopulationId {
        &self.source_population_id
    }

    pub fn destination_population_id(&self) -> &PopulationId {
        &self.destination_population_id
    }

    pub fn declared_source_fraction_ppm(&self) -> u32 {
        self.declared_source_fraction_ppm
    }

    pub fn realized_replacement_copies(&self) -> u64 {
        self.realized_replacement_copies
    }

    pub fn canonical_digest(&self) -> DemographicAdmixtureExecutionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(ADMIXTURE_EXECUTION_DIGEST_DOMAIN);
        digest.update([self.model.tag()]);
        digest.update(self.event_digest.as_bytes());
        digest.update(self.structure_transition_digest.as_bytes());
        digest.update(self.structure_digest.as_bytes());
        digest.update(self.source_snapshot_digest.as_bytes());
        digest.update(self.source_cursor_digest.as_bytes());
        digest.update(self.result_snapshot_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_u64(&mut digest, self.generation.0);
        put_text(&mut digest, self.source_population_id.as_str());
        put_text(&mut digest, self.destination_population_id.as_str());
        put_u32(&mut digest, self.declared_source_fraction_ppm);
        put_u64(&mut digest, self.realized_replacement_copies);
        digest.update(self.source_state_digest.as_bytes());
        digest.update(self.source_point_digest.as_bytes());
        digest.update(self.destination_source_state_digest.as_bytes());
        digest.update(self.destination_source_point_digest.as_bytes());
        digest.update(self.destination_result_state_digest.as_bytes());
        digest.update(self.destination_result_point_digest.as_bytes());
        DemographicAdmixtureExecutionProvenanceDigest(digest.finalize().into())
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
        result: &DemographicAdmixtureExecutionResult,
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

        let (destination_id, source_id, fraction_ppm) = admixture_coordinates(event)?;
        if destination_id != &self.destination_population_id
            || source_id != &self.source_population_id
            || fraction_ppm != self.declared_source_fraction_ppm
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
        let destination_state = source_populations
            .get(destination_id)
            .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
                population: destination_id.clone(),
            })?;
        let destination_point = source_points
            .get(destination_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        let realized = realized_replacement_copies(schema, destination_state, fraction_ppm)?;

        if self.model != DemographicAdmixtureExecutionModel::CensusPreservingMarginalReplacementV1
            || event.canonical_digest()? != self.event_digest
            || structure_transition.canonical_digest() != self.structure_transition_digest
            || structure.canonical_digest()? != self.structure_digest
            || source_snapshot.canonical_digest() != self.source_snapshot_digest
            || source_cursor.canonical_digest()? != self.source_cursor_digest
            || event.experiment_id() != &self.experiment_id
            || event.generation() != self.generation
            || realized != self.realized_replacement_copies
            || source_state.canonical_digest(schema)? != self.source_state_digest
            || source_point.canonical_digest() != self.source_point_digest
            || destination_state.canonical_digest(schema)? != self.destination_source_state_digest
            || destination_point.canonical_digest() != self.destination_source_point_digest
        {
            return Err(EvolutionError::DemographicExecutionAuthorityMismatch);
        }

        let derived = derive_admixture_result(
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

        let destination_result_state = result
            .populations
            .get(destination_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        let destination_result_point = result
            .points
            .get(destination_id)
            .ok_or(EvolutionError::MetapopulationSetMismatch)?;
        if destination_result_state.canonical_digest(schema)? != self.destination_result_state_digest
            || destination_result_point.canonical_digest() != self.destination_result_point_digest
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
pub struct DemographicAdmixtureExecutionProvenanceDigest(pub(crate) [u8; 32]);

impl DemographicAdmixtureExecutionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DemographicAdmixtureExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DemographicAdmixtureExecutionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DemographicAdmixtureExecutionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicAdmixtureExecutionResult {
    pub populations: BTreeMap<PopulationId, PopulationGeneticState>,
    pub points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    pub snapshot: MetapopulationSnapshot,
    pub history_cursor: DemographicInterventionCursor,
    pub provenance: DemographicAdmixtureExecutionProvenance,
}

#[allow(clippy::too_many_arguments)]
pub fn execute_census_preserving_pulse_admixture(
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    source_snapshot: &MetapopulationSnapshot,
    source_cursor: &DemographicInterventionCursor,
    event: &DemographicEventDeclaration,
    structure_transition: &DemographicStructureTransition,
) -> Result<DemographicAdmixtureExecutionResult, EvolutionError> {
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

    let (destination_id, source_id, fraction_ppm) = admixture_coordinates(event)?;
    let source_state = source_populations
        .get(source_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: source_id.clone(),
        })?;
    let source_point = source_points
        .get(source_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;
    let destination_state = source_populations
        .get(destination_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: destination_id.clone(),
        })?;
    let destination_point = source_points
        .get(destination_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;

    let realized = realized_replacement_copies(schema, destination_state, fraction_ppm)?;
    let available_source_copies = source_state
        .census_individuals
        .checked_mul(u64::from(schema.ploidy))
        .ok_or(EvolutionError::CountOverflow)?;
    if realized > available_source_copies {
        return Err(EvolutionError::SamplingInvariantViolation);
    }

    let derived = derive_admixture_result(
        schema,
        structure,
        source_populations,
        source_points,
        event,
    )?;
    let destination_result_state = derived
        .populations
        .get(destination_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;
    let destination_result_point = derived
        .points
        .get(destination_id)
        .ok_or(EvolutionError::MetapopulationSetMismatch)?;

    let provenance = DemographicAdmixtureExecutionProvenance {
        model: DemographicAdmixtureExecutionModel::CensusPreservingMarginalReplacementV1,
        event_digest: event.canonical_digest()?,
        structure_transition_digest: structure_transition.canonical_digest(),
        structure_digest: structure.canonical_digest()?,
        source_snapshot_digest: source_snapshot.canonical_digest(),
        source_cursor_digest: source_cursor.canonical_digest()?,
        result_snapshot_digest: derived.snapshot.canonical_digest(),
        experiment_id: event.experiment_id().clone(),
        generation: event.generation(),
        source_population_id: source_id.clone(),
        destination_population_id: destination_id.clone(),
        declared_source_fraction_ppm: fraction_ppm,
        realized_replacement_copies: realized,
        source_state_digest: source_state.canonical_digest(schema)?,
        source_point_digest: source_point.canonical_digest(),
        destination_source_state_digest: destination_state.canonical_digest(schema)?,
        destination_source_point_digest: destination_point.canonical_digest(),
        destination_result_state_digest: destination_result_state.canonical_digest(schema)?,
        destination_result_point_digest: destination_result_point.canonical_digest(),
    };
    let execution_digest = provenance.canonical_digest();
    let history_cursor = DemographicInterventionCursor::advance_after_validated_execution(
        source_cursor,
        provenance.event_digest,
        provenance.structure_transition_digest,
        DemographicEventExecutionDigest(execution_digest.0),
        &derived.snapshot,
    )?;
    let result = DemographicAdmixtureExecutionResult {
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

struct DerivedAdmixtureResult {
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
}

fn derive_admixture_result(
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    event: &DemographicEventDeclaration,
) -> Result<DerivedAdmixtureResult, EvolutionError> {
    let (destination_id, source_id, fraction_ppm) = admixture_coordinates(event)?;
    let source_state = source_populations
        .get(source_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: source_id.clone(),
        })?;
    let destination_state = source_populations
        .get(destination_id)
        .ok_or_else(|| EvolutionError::UnknownDemographicPopulation {
            population: destination_id.clone(),
        })?;
    source_state.validate(schema)?;
    destination_state.validate(schema)?;

    let realized = realized_replacement_copies(schema, destination_state, fraction_ppm)?;
    let available_source_copies = source_state
        .census_individuals
        .checked_mul(u64::from(schema.ploidy))
        .ok_or(EvolutionError::CountOverflow)?;
    if realized > available_source_copies {
        return Err(EvolutionError::SamplingInvariantViolation);
    }

    let removed = sample_marginal_copy_counts_without_replacement(
        schema,
        destination_state,
        realized,
        |locus_id, allele_id, ordinal| {
            destination_removal_priority(
                event.experiment_id(),
                event.event_id().as_str(),
                event.generation(),
                destination_id,
                locus_id,
                allele_id,
                ordinal,
            )
        },
    )?;
    let contributed = sample_marginal_copy_counts_without_replacement(
        schema,
        source_state,
        realized,
        |locus_id, allele_id, ordinal| {
            source_contribution_priority(
                event.experiment_id(),
                event.event_id().as_str(),
                event.generation(),
                source_id,
                destination_id,
                locus_id,
                allele_id,
                ordinal,
            )
        },
    )?;

    let mut result_counts = destination_state.allele_copy_counts.clone();
    for locus_id in schema.loci.keys() {
        let counts = result_counts
            .get_mut(locus_id)
            .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
        if let Some(removals) = removed.get(locus_id) {
            for (allele_id, remove_count) in removals {
                let current = counts
                    .get_mut(allele_id)
                    .ok_or(EvolutionError::SamplingInvariantViolation)?;
                *current = current
                    .checked_sub(*remove_count)
                    .ok_or(EvolutionError::SamplingInvariantViolation)?;
            }
        }
        if let Some(additions) = contributed.get(locus_id) {
            for (allele_id, add_count) in additions {
                let current = counts.entry(allele_id.clone()).or_insert(0);
                *current = current
                    .checked_add(*add_count)
                    .ok_or(EvolutionError::CountOverflow)?;
            }
        }
    }

    let destination_result = PopulationGeneticState::from_counts(
        destination_id.clone(),
        schema,
        destination_state.census_individuals,
        result_counts,
    )?;
    let destination_result_point = PopulationTrajectoryPoint::from_validated_state(
        schema,
        &destination_result,
        event.experiment_id().clone(),
        event.generation(),
    )?;

    let mut populations = source_populations.clone();
    let mut points = source_points.clone();
    populations.insert(destination_id.clone(), destination_result);
    points.insert(destination_id.clone(), destination_result_point);

    let snapshot = MetapopulationSnapshot::capture_reference(
        schema,
        structure,
        &populations,
        &points,
    )?;
    Ok(DerivedAdmixtureResult {
        populations,
        points,
        snapshot,
    })
}

fn realized_replacement_copies(
    schema: &HereditarySchema,
    destination: &PopulationGeneticState,
    fraction_ppm: u32,
) -> Result<u64, EvolutionError> {
    destination.validate(schema)?;
    let total_copies = destination
        .census_individuals
        .checked_mul(u64::from(schema.ploidy))
        .ok_or(EvolutionError::CountOverflow)?;
    let numerator = u128::from(total_copies)
        .checked_mul(u128::from(fraction_ppm))
        .and_then(|value| value.checked_add(u128::from(PROBABILITY_SCALE_PPM / 2)))
        .ok_or(EvolutionError::CountOverflow)?;
    let rounded = numerator / u128::from(PROBABILITY_SCALE_PPM);
    u64::try_from(rounded).map_err(|_| EvolutionError::CountOverflow)
}

fn admixture_coordinates(
    event: &DemographicEventDeclaration,
) -> Result<(&PopulationId, &PopulationId, u32), EvolutionError> {
    match event.kind() {
        DemographicEventKind::PulseAdmixture {
            destination,
            source,
            source_fraction_ppm,
        } => Ok((destination, source, *source_fraction_ppm)),
        _ => Err(EvolutionError::DemographicEventKindUnsupportedForExecutor),
    }
}

fn destination_removal_priority(
    experiment_id: &EvolutionExperimentId,
    event_id: &str,
    generation: PopulationGeneration,
    destination_id: &PopulationId,
    locus_id: &LocusId,
    allele_id: &crate::AlleleId,
    ordinal: u64,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(ADMIXTURE_REMOVAL_PRIORITY_DOMAIN);
    put_text(&mut digest, experiment_id.as_str());
    put_text(&mut digest, event_id);
    put_u64(&mut digest, generation.0);
    put_text(&mut digest, destination_id.as_str());
    put_text(&mut digest, locus_id.as_str());
    put_text(&mut digest, allele_id.as_str());
    put_u64(&mut digest, ordinal);
    digest.finalize().into()
}

#[allow(clippy::too_many_arguments)]
fn source_contribution_priority(
    experiment_id: &EvolutionExperimentId,
    event_id: &str,
    generation: PopulationGeneration,
    source_id: &PopulationId,
    destination_id: &PopulationId,
    locus_id: &LocusId,
    allele_id: &crate::AlleleId,
    ordinal: u64,
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(ADMIXTURE_SOURCE_PRIORITY_DOMAIN);
    put_text(&mut digest, experiment_id.as_str());
    put_text(&mut digest, event_id);
    put_u64(&mut digest, generation.0);
    put_text(&mut digest, source_id.as_str());
    put_text(&mut digest, destination_id.as_str());
    put_text(&mut digest, locus_id.as_str());
    put_text(&mut digest, allele_id.as_str());
    put_u64(&mut digest, ordinal);
    digest.finalize().into()
}
