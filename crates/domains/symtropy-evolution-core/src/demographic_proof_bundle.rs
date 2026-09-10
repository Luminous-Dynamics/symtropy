use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    DemographicAdmixtureExecutionResult, DemographicEventDeclaration,
    DemographicEventExecutionResult, DemographicEventKind, DemographicExtinctionExecutionResult,
    DemographicFounderExecutionResult, DemographicInterventionCursor,
    DemographicInterventionCursorDigest, DemographicSplitExecutionResult,
    DemographicStructureTransition, EvolutionError, EvolutionExperimentId, HereditarySchema,
    HereditarySchemaDigest, MetapopulationSnapshot, MetapopulationSnapshotDigest,
    PopulationGeneration, PopulationGeneticState, PopulationId, PopulationStructureProfile,
    PopulationStructureProfileDigest, PopulationTrajectoryPoint,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const DEMOGRAPHIC_PROOF_BUNDLE_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-intervention-proof-bundle:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemographicInterventionProofBundleModel {
    ExactSameGenerationReplayV1,
}

impl DemographicInterventionProofBundleModel {
    fn tag(self) -> u8 {
        match self {
            Self::ExactSameGenerationReplayV1 => 0,
        }
    }
}

/// Closed V0 adapter over the already-versioned demographic executors.
///
/// This is deliberately not an extensible global event framework. Each variant
/// names one exact result/provenance authority already owned by this crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemographicExecutionEvidence {
    CensusResize(DemographicEventExecutionResult),
    FounderOrRecolonization(DemographicFounderExecutionResult),
    Extinction(DemographicExtinctionExecutionResult),
    PopulationSplit(DemographicSplitExecutionResult),
    PulseAdmixture(DemographicAdmixtureExecutionResult),
}

impl DemographicExecutionEvidence {
    fn tag(&self) -> u8 {
        match self {
            Self::CensusResize(_) => 0,
            Self::FounderOrRecolonization(_) => 1,
            Self::Extinction(_) => 2,
            Self::PopulationSplit(_) => 3,
            Self::PulseAdmixture(_) => 4,
        }
    }

    fn matches_event_kind(&self, event: &DemographicEventDeclaration) -> bool {
        matches!(
            (self, event.kind()),
            (Self::CensusResize(_), DemographicEventKind::CensusResize { .. })
                | (
                    Self::FounderOrRecolonization(_),
                    DemographicEventKind::FounderEvent { .. }
                        | DemographicEventKind::Recolonization { .. }
                )
                | (Self::Extinction(_), DemographicEventKind::Extinction { .. })
                | (
                    Self::PopulationSplit(_),
                    DemographicEventKind::PopulationSplit { .. }
                )
                | (
                    Self::PulseAdmixture(_),
                    DemographicEventKind::PulseAdmixture { .. }
                )
        )
    }

    fn populations(&self) -> &BTreeMap<PopulationId, PopulationGeneticState> {
        match self {
            Self::CensusResize(result) => &result.populations,
            Self::FounderOrRecolonization(result) => &result.populations,
            Self::Extinction(result) => &result.populations,
            Self::PopulationSplit(result) => &result.populations,
            Self::PulseAdmixture(result) => &result.populations,
        }
    }

    fn points(&self) -> &BTreeMap<PopulationId, PopulationTrajectoryPoint> {
        match self {
            Self::CensusResize(result) => &result.points,
            Self::FounderOrRecolonization(result) => &result.points,
            Self::Extinction(result) => &result.points,
            Self::PopulationSplit(result) => &result.points,
            Self::PulseAdmixture(result) => &result.points,
        }
    }

    fn snapshot(&self) -> &MetapopulationSnapshot {
        match self {
            Self::CensusResize(result) => &result.snapshot,
            Self::FounderOrRecolonization(result) => &result.snapshot,
            Self::Extinction(result) => &result.snapshot,
            Self::PopulationSplit(result) => &result.snapshot,
            Self::PulseAdmixture(result) => &result.snapshot,
        }
    }

    fn cursor(&self) -> &DemographicInterventionCursor {
        match self {
            Self::CensusResize(result) => &result.history_cursor,
            Self::FounderOrRecolonization(result) => &result.history_cursor,
            Self::Extinction(result) => &result.history_cursor,
            Self::PopulationSplit(result) => &result.history_cursor,
            Self::PulseAdmixture(result) => &result.history_cursor,
        }
    }

    fn provenance_digest_bytes(&self) -> [u8; 32] {
        match self {
            Self::CensusResize(result) => *result.provenance.canonical_digest().as_bytes(),
            Self::FounderOrRecolonization(result) => {
                *result.provenance.canonical_digest().as_bytes()
            }
            Self::Extinction(result) => *result.provenance.canonical_digest().as_bytes(),
            Self::PopulationSplit(result) => *result.provenance.canonical_digest().as_bytes(),
            Self::PulseAdmixture(result) => *result.provenance.canonical_digest().as_bytes(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn validate_current(
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
    ) -> Result<(), EvolutionError> {
        if !self.matches_event_kind(event) {
            return Err(EvolutionError::DemographicExecutionAuthorityMismatch);
        }

        match self {
            Self::CensusResize(result) => {
                if source_structure.canonical_digest()? != successor_structure.canonical_digest()? {
                    return Err(EvolutionError::DemographicMembershipPreservingStructureChanged);
                }
                result.provenance.validate_current(
                    schema,
                    source_structure,
                    source_populations,
                    source_points,
                    source_snapshot,
                    source_cursor,
                    event,
                    structure_transition,
                    result,
                )
            }
            Self::FounderOrRecolonization(result) => result.provenance.validate_current(
                schema,
                source_structure,
                successor_structure,
                source_populations,
                source_points,
                source_snapshot,
                source_cursor,
                event,
                structure_transition,
                result,
            ),
            Self::Extinction(result) => result.provenance.validate_current(
                schema,
                source_structure,
                successor_structure,
                source_populations,
                source_points,
                source_snapshot,
                source_cursor,
                event,
                structure_transition,
                result,
            ),
            Self::PopulationSplit(result) => result.provenance.validate_current(
                schema,
                source_structure,
                successor_structure,
                source_populations,
                source_points,
                source_snapshot,
                source_cursor,
                event,
                structure_transition,
                result,
            ),
            Self::PulseAdmixture(result) => {
                if source_structure.canonical_digest()? != successor_structure.canonical_digest()? {
                    return Err(EvolutionError::DemographicMembershipPreservingStructureChanged);
                }
                result.provenance.validate_current(
                    schema,
                    source_structure,
                    source_populations,
                    source_points,
                    source_snapshot,
                    source_cursor,
                    event,
                    structure_transition,
                    result,
                )
            }
        }
    }
}

/// One ordered step in a same-generation demographic proof transcript.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicInterventionProofStep {
    event: DemographicEventDeclaration,
    structure_transition: DemographicStructureTransition,
    successor_structure: PopulationStructureProfile,
    execution: DemographicExecutionEvidence,
}

impl DemographicInterventionProofStep {
    /// Construct evidence-shaped step data. Current authority is granted only by
    /// bundle replay against an exact root source cut.
    pub fn new(
        event: DemographicEventDeclaration,
        structure_transition: DemographicStructureTransition,
        successor_structure: PopulationStructureProfile,
        execution: DemographicExecutionEvidence,
    ) -> Self {
        Self {
            event,
            structure_transition,
            successor_structure,
            execution,
        }
    }

    pub fn event(&self) -> &DemographicEventDeclaration {
        &self.event
    }

    pub fn successor_structure(&self) -> &PopulationStructureProfile {
        &self.successor_structure
    }

    pub fn execution(&self) -> &DemographicExecutionEvidence {
        &self.execution
    }
}

/// Revalidatable ordered proof of demographic interventions within one biological
/// generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicInterventionProofBundle {
    model: DemographicInterventionProofBundleModel,
    schema_digest: HereditarySchemaDigest,
    root_structure_digest: PopulationStructureProfileDigest,
    root_snapshot_digest: MetapopulationSnapshotDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
    steps: Vec<DemographicInterventionProofStep>,
    final_structure_digest: PopulationStructureProfileDigest,
    final_snapshot_digest: MetapopulationSnapshotDigest,
    final_cursor_digest: DemographicInterventionCursorDigest,
}

impl DemographicInterventionProofBundle {
    #[allow(clippy::too_many_arguments)]
    pub fn declare_current(
        schema: &HereditarySchema,
        root_structure: &PopulationStructureProfile,
        root_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        root_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        root_snapshot: &MetapopulationSnapshot,
        steps: Vec<DemographicInterventionProofStep>,
    ) -> Result<Self, EvolutionError> {
        root_snapshot.validate_current(schema, root_structure, root_populations, root_points)?;
        let summary = replay_steps(
            schema,
            root_structure,
            root_populations,
            root_points,
            root_snapshot,
            &steps,
        )?;
        let bundle = Self {
            model: DemographicInterventionProofBundleModel::ExactSameGenerationReplayV1,
            schema_digest: schema.canonical_digest()?,
            root_structure_digest: root_structure.canonical_digest()?,
            root_snapshot_digest: root_snapshot.canonical_digest(),
            experiment_id: root_snapshot.experiment_id().clone(),
            generation: root_snapshot.generation(),
            steps,
            final_structure_digest: summary.final_structure_digest,
            final_snapshot_digest: summary.final_snapshot_digest,
            final_cursor_digest: summary.final_cursor_digest,
        };
        bundle.validate_current(
            schema,
            root_structure,
            root_populations,
            root_points,
            root_snapshot,
        )?;
        Ok(bundle)
    }

    pub fn steps(&self) -> &[DemographicInterventionProofStep] {
        &self.steps
    }

    pub fn final_structure_digest(&self) -> PopulationStructureProfileDigest {
        self.final_structure_digest
    }

    pub fn final_snapshot_digest(&self) -> MetapopulationSnapshotDigest {
        self.final_snapshot_digest
    }

    pub fn final_cursor_digest(&self) -> DemographicInterventionCursorDigest {
        self.final_cursor_digest
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        root_structure: &PopulationStructureProfile,
        root_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        root_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        root_snapshot: &MetapopulationSnapshot,
    ) -> Result<(), EvolutionError> {
        root_snapshot.validate_current(schema, root_structure, root_populations, root_points)?;
        if self.model != DemographicInterventionProofBundleModel::ExactSameGenerationReplayV1
            || schema.canonical_digest()? != self.schema_digest
            || root_structure.canonical_digest()? != self.root_structure_digest
            || root_snapshot.canonical_digest() != self.root_snapshot_digest
            || root_snapshot.experiment_id() != &self.experiment_id
            || root_snapshot.generation() != self.generation
        {
            return Err(EvolutionError::DemographicExecutionAuthorityMismatch);
        }

        let summary = replay_steps(
            schema,
            root_structure,
            root_populations,
            root_points,
            root_snapshot,
            &self.steps,
        )?;
        if summary.final_structure_digest != self.final_structure_digest
            || summary.final_snapshot_digest != self.final_snapshot_digest
            || summary.final_cursor_digest != self.final_cursor_digest
        {
            return Err(EvolutionError::DemographicHistoryCursorMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<DemographicInterventionProofBundleDigest, EvolutionError> {
        let mut digest = Sha256::new();
        digest.update(DEMOGRAPHIC_PROOF_BUNDLE_DOMAIN);
        digest.update([self.model.tag()]);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.root_structure_digest.as_bytes());
        digest.update(self.root_snapshot_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_u64(&mut digest, self.generation.0);
        put_u64(&mut digest, self.steps.len() as u64);
        for step in &self.steps {
            digest.update([step.execution.tag()]);
            digest.update(step.event.canonical_digest()?.as_bytes());
            digest.update(step.structure_transition.canonical_digest().as_bytes());
            digest.update(step.successor_structure.canonical_digest()?.as_bytes());
            digest.update(step.execution.provenance_digest_bytes());
            digest.update(step.execution.snapshot().canonical_digest().as_bytes());
            digest.update(step.execution.cursor().canonical_digest()?.as_bytes());
        }
        digest.update(self.final_structure_digest.as_bytes());
        digest.update(self.final_snapshot_digest.as_bytes());
        digest.update(self.final_cursor_digest.as_bytes());
        Ok(DemographicInterventionProofBundleDigest(
            digest.finalize().into(),
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DemographicInterventionProofBundleDigest([u8; 32]);

impl DemographicInterventionProofBundleDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DemographicInterventionProofBundleDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DemographicInterventionProofBundleDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DemographicInterventionProofBundleDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

struct ReplaySummary {
    final_structure_digest: PopulationStructureProfileDigest,
    final_snapshot_digest: MetapopulationSnapshotDigest,
    final_cursor_digest: DemographicInterventionCursorDigest,
}

#[allow(clippy::too_many_arguments)]
fn replay_steps(
    schema: &HereditarySchema,
    root_structure: &PopulationStructureProfile,
    root_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    root_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    root_snapshot: &MetapopulationSnapshot,
    steps: &[DemographicInterventionProofStep],
) -> Result<ReplaySummary, EvolutionError> {
    root_snapshot.validate_current(schema, root_structure, root_populations, root_points)?;
    let root_cursor = DemographicInterventionCursor::declare_reference_root(
        schema,
        root_structure,
        root_populations,
        root_points,
        root_snapshot,
    )?;

    let mut current_structure = root_structure;
    let mut current_populations = root_populations;
    let mut current_points = root_points;
    let mut current_snapshot = root_snapshot;
    let mut current_cursor = &root_cursor;

    for step in steps {
        if step.event.experiment_id() != root_snapshot.experiment_id()
            || step.event.generation() != root_snapshot.generation()
        {
            return Err(EvolutionError::DemographicExecutionAuthorityMismatch);
        }
        step.execution.validate_current(
            schema,
            current_structure,
            &step.successor_structure,
            current_populations,
            current_points,
            current_snapshot,
            current_cursor,
            &step.event,
            &step.structure_transition,
        )?;

        current_structure = &step.successor_structure;
        current_populations = step.execution.populations();
        current_points = step.execution.points();
        current_snapshot = step.execution.snapshot();
        current_cursor = step.execution.cursor();
    }

    current_snapshot.validate_current(
        schema,
        current_structure,
        current_populations,
        current_points,
    )?;
    if current_snapshot.experiment_id() != root_snapshot.experiment_id()
        || current_snapshot.generation() != root_snapshot.generation()
    {
        return Err(EvolutionError::DemographicExecutionAuthorityMismatch);
    }

    Ok(ReplaySummary {
        final_structure_digest: current_structure.canonical_digest()?,
        final_snapshot_digest: current_snapshot.canonical_digest(),
        final_cursor_digest: current_cursor.canonical_digest()?,
    })
}
