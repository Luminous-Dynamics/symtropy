use crate::{
    DemographicExecutionEvidence, DemographicInterventionCursor, DemographicInterventionCursorDigest,
    DemographicInterventionProofBundle, DemographicInterventionProofBundleDigest, EvolutionError,
    EvolutionExperimentId, HereditarySchema, HereditarySchemaDigest, MetapopulationSnapshot,
    MetapopulationSnapshotDigest, PopulationGeneration, PopulationGeneticState, PopulationId,
    PopulationStructureProfile, PopulationStructureProfileDigest, PopulationTrajectoryPoint,
};
use std::collections::BTreeMap;

/// Runtime-only authority proving that a non-root demographic source cut was
/// reached by successful deterministic proof-bundle replay.
///
/// Deliberately does not implement Serialize/Deserialize. Persist the proof
/// bundle, then replay it after restore to mint a fresh token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedDemographicInterventionSource {
    schema_digest: HereditarySchemaDigest,
    structure_digest: PopulationStructureProfileDigest,
    snapshot_digest: MetapopulationSnapshotDigest,
    cursor_digest: DemographicInterventionCursorDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
    intervention_ordinal: u64,
    proof_bundle_digest: DemographicInterventionProofBundleDigest,
}

impl ValidatedDemographicInterventionSource {
    pub(crate) fn from_validated_bundle_replay(
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        snapshot: &MetapopulationSnapshot,
        cursor: &DemographicInterventionCursor,
        proof_bundle_digest: DemographicInterventionProofBundleDigest,
    ) -> Result<Self, EvolutionError> {
        if snapshot.experiment_id() != cursor.experiment_id()
            || snapshot.generation() != cursor.generation()
            || snapshot.canonical_digest() != cursor.current_snapshot_digest()
            || cursor.is_root()
        {
            return Err(EvolutionError::DemographicHistoryCursorMismatch);
        }
        Ok(Self {
            schema_digest: schema.canonical_digest()?,
            structure_digest: structure.canonical_digest()?,
            snapshot_digest: snapshot.canonical_digest(),
            cursor_digest: cursor.canonical_digest()?,
            experiment_id: snapshot.experiment_id().clone(),
            generation: snapshot.generation(),
            intervention_ordinal: cursor.intervention_ordinal(),
            proof_bundle_digest,
        })
    }

    pub fn structure_digest(&self) -> PopulationStructureProfileDigest {
        self.structure_digest
    }

    pub fn snapshot_digest(&self) -> MetapopulationSnapshotDigest {
        self.snapshot_digest
    }

    pub fn cursor_digest(&self) -> DemographicInterventionCursorDigest {
        self.cursor_digest
    }

    pub fn experiment_id(&self) -> &EvolutionExperimentId {
        &self.experiment_id
    }

    pub fn generation(&self) -> PopulationGeneration {
        self.generation
    }

    pub fn intervention_ordinal(&self) -> u64 {
        self.intervention_ordinal
    }

    pub fn proof_bundle_digest(&self) -> DemographicInterventionProofBundleDigest {
        self.proof_bundle_digest
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn validate_current(
        &self,
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        snapshot: &MetapopulationSnapshot,
        cursor: &DemographicInterventionCursor,
    ) -> Result<(), EvolutionError> {
        snapshot.validate_current(schema, structure, populations, points)?;
        if cursor.is_root()
            || schema.canonical_digest()? != self.schema_digest
            || structure.canonical_digest()? != self.structure_digest
            || snapshot.canonical_digest() != self.snapshot_digest
            || cursor.canonical_digest()? != self.cursor_digest
            || snapshot.experiment_id() != &self.experiment_id
            || snapshot.generation() != self.generation
            || cursor.experiment_id() != &self.experiment_id
            || cursor.generation() != self.generation
            || cursor.intervention_ordinal() != self.intervention_ordinal
            || cursor.current_snapshot_digest() != self.snapshot_digest
        {
            return Err(EvolutionError::DemographicHistoryCursorMismatch);
        }
        Ok(())
    }
}

impl DemographicInterventionProofBundle {
    /// Replay/revalidate this persisted proof bundle and mint a fresh runtime-only
    /// authority token for its exact non-root final cut.
    ///
    /// Empty/root bundles deliberately cannot mint a token; the ordinary root
    /// source gate remains the authority for ordinal zero.
    #[allow(clippy::too_many_arguments)]
    pub fn mint_validated_final_source(
        &self,
        schema: &HereditarySchema,
        root_structure: &PopulationStructureProfile,
        root_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        root_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        root_snapshot: &MetapopulationSnapshot,
    ) -> Result<ValidatedDemographicInterventionSource, EvolutionError> {
        self.validate_current(
            schema,
            root_structure,
            root_populations,
            root_points,
            root_snapshot,
        )?;
        let final_step = self
            .steps()
            .last()
            .ok_or(EvolutionError::DemographicHistoryCursorNotRoot)?;
        let (populations, points, snapshot, cursor) = evidence_cut(final_step.execution());
        let token = ValidatedDemographicInterventionSource::from_validated_bundle_replay(
            schema,
            final_step.successor_structure(),
            snapshot,
            cursor,
            self.canonical_digest()?,
        )?;
        token.validate_current(
            schema,
            final_step.successor_structure(),
            populations,
            points,
            snapshot,
            cursor,
        )?;
        Ok(token)
    }
}

fn evidence_cut(
    evidence: &DemographicExecutionEvidence,
) -> (
    &BTreeMap<PopulationId, PopulationGeneticState>,
    &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    &MetapopulationSnapshot,
    &DemographicInterventionCursor,
) {
    match evidence {
        DemographicExecutionEvidence::CensusResize(result) => (
            &result.populations,
            &result.points,
            &result.snapshot,
            &result.history_cursor,
        ),
        DemographicExecutionEvidence::FounderOrRecolonization(result) => (
            &result.populations,
            &result.points,
            &result.snapshot,
            &result.history_cursor,
        ),
        DemographicExecutionEvidence::Extinction(result) => (
            &result.populations,
            &result.points,
            &result.snapshot,
            &result.history_cursor,
        ),
        DemographicExecutionEvidence::PopulationSplit(result) => (
            &result.populations,
            &result.points,
            &result.snapshot,
            &result.history_cursor,
        ),
        DemographicExecutionEvidence::PulseAdmixture(result) => (
            &result.populations,
            &result.points,
            &result.snapshot,
            &result.history_cursor,
        ),
    }
}

/// Internal gate shared by demographic executors as they adopt B4B.
#[derive(Clone, Copy)]
pub(crate) enum DemographicSourceAuthority<'a> {
    Root,
    Proven(&'a ValidatedDemographicInterventionSource),
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn validate_demographic_source_authority(
    authority: DemographicSourceAuthority<'_>,
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: &MetapopulationSnapshot,
    cursor: &DemographicInterventionCursor,
) -> Result<(), EvolutionError> {
    match authority {
        DemographicSourceAuthority::Root => {
            cursor.validate_root_current(schema, structure, populations, points, snapshot)
        }
        DemographicSourceAuthority::Proven(token) => {
            token.validate_current(schema, structure, populations, points, snapshot, cursor)
        }
    }
}
