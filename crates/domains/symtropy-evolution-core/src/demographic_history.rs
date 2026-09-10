use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    DemographicEventDeclarationDigest, DemographicStructureTransitionDigest, EvolutionError,
    EvolutionExperimentId, HereditarySchema, MetapopulationSnapshot, MetapopulationSnapshotDigest,
    PopulationGeneration, PopulationGeneticState, PopulationId, PopulationStructureProfile,
    PopulationTrajectoryPoint,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

const DEMOGRAPHIC_HISTORY_CURSOR_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-intervention-cursor:v1\0";

/// Opaque execution-receipt identity reserved for the demographic executor.
///
/// It is crate-private in this authority tranche: public callers cannot mint a
/// successor history cursor merely by supplying arbitrary bytes. DEMOG-04B3 will
/// attach this identity to the concrete, revalidatable execution receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct DemographicEventExecutionDigest(pub(crate) [u8; 32]);

/// One ordered point in the same-generation demographic intervention chain.
///
/// A root cursor proves only the exact current metapopulation snapshot at
/// intervention ordinal zero. Successor cursors are minted crate-internally by
/// validated demographic execution and remain distinct even if the resulting
/// biological state is byte-identical to an earlier state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicInterventionCursor {
    current_snapshot_digest: MetapopulationSnapshotDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
    intervention_ordinal: u64,
    predecessor_cursor_digest: Option<DemographicInterventionCursorDigest>,
    event_digest: Option<DemographicEventDeclarationDigest>,
    structure_transition_digest: Option<DemographicStructureTransitionDigest>,
    execution_digest: Option<DemographicEventExecutionDigest>,
}

impl DemographicInterventionCursor {
    /// Declare ordinal-zero demographic history from one fully revalidated
    /// simultaneous source snapshot.
    #[allow(clippy::too_many_arguments)]
    pub fn declare_reference_root(
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        snapshot: &MetapopulationSnapshot,
    ) -> Result<Self, EvolutionError> {
        snapshot.validate_current(schema, structure, populations, points)?;
        let cursor = Self {
            current_snapshot_digest: snapshot.canonical_digest(),
            experiment_id: snapshot.experiment_id().clone(),
            generation: snapshot.generation(),
            intervention_ordinal: 0,
            predecessor_cursor_digest: None,
            event_digest: None,
            structure_transition_digest: None,
            execution_digest: None,
        };
        cursor.validate_local_shape()?;
        Ok(cursor)
    }

    pub fn current_snapshot_digest(&self) -> MetapopulationSnapshotDigest {
        self.current_snapshot_digest
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

    pub fn predecessor_cursor_digest(&self) -> Option<DemographicInterventionCursorDigest> {
        self.predecessor_cursor_digest
    }

    pub fn is_root(&self) -> bool {
        self.intervention_ordinal == 0
    }

    /// Revalidate an ordinal-zero cursor against its exact current source cut.
    /// Non-root cursors require the concrete execution receipt introduced by the
    /// executor tranche and therefore deliberately cannot regain authority here.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_root_current(
        &self,
        schema: &HereditarySchema,
        structure: &PopulationStructureProfile,
        populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        snapshot: &MetapopulationSnapshot,
    ) -> Result<(), EvolutionError> {
        self.validate_local_shape()?;
        if !self.is_root() {
            return Err(EvolutionError::DemographicHistoryCursorNotRoot);
        }
        snapshot.validate_current(schema, structure, populations, points)?;
        if snapshot.canonical_digest() != self.current_snapshot_digest
            || snapshot.experiment_id() != &self.experiment_id
            || snapshot.generation() != self.generation
        {
            return Err(EvolutionError::DemographicHistoryCursorMismatch);
        }
        Ok(())
    }

    /// Construct the next ordered history point after a demographic executor has
    /// already validated and recorded one exact intervention.
    ///
    /// This is crate-private by design: external callers cannot assert execution
    /// merely by naming event/structure digests.
    pub(crate) fn advance_after_validated_execution(
        predecessor: &Self,
        event_digest: DemographicEventDeclarationDigest,
        structure_transition_digest: DemographicStructureTransitionDigest,
        execution_digest: DemographicEventExecutionDigest,
        resulting_snapshot: &MetapopulationSnapshot,
    ) -> Result<Self, EvolutionError> {
        predecessor.validate_local_shape()?;
        if resulting_snapshot.experiment_id() != &predecessor.experiment_id
            || resulting_snapshot.generation() != predecessor.generation
        {
            return Err(EvolutionError::DemographicHistoryCursorMismatch);
        }
        let intervention_ordinal = predecessor
            .intervention_ordinal
            .checked_add(1)
            .ok_or(EvolutionError::CountOverflow)?;
        let cursor = Self {
            current_snapshot_digest: resulting_snapshot.canonical_digest(),
            experiment_id: predecessor.experiment_id.clone(),
            generation: predecessor.generation,
            intervention_ordinal,
            predecessor_cursor_digest: Some(predecessor.canonical_digest()?),
            event_digest: Some(event_digest),
            structure_transition_digest: Some(structure_transition_digest),
            execution_digest: Some(execution_digest),
        };
        cursor.validate_local_shape()?;
        Ok(cursor)
    }

    fn validate_local_shape(&self) -> Result<(), EvolutionError> {
        let links_present = [
            self.predecessor_cursor_digest.is_some(),
            self.event_digest.is_some(),
            self.structure_transition_digest.is_some(),
            self.execution_digest.is_some(),
        ];
        if self.intervention_ordinal == 0 {
            if links_present.into_iter().any(|present| present) {
                return Err(EvolutionError::DemographicHistoryCursorShapeMismatch);
            }
        } else if links_present.into_iter().any(|present| !present) {
            return Err(EvolutionError::DemographicHistoryCursorShapeMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<DemographicInterventionCursorDigest, EvolutionError> {
        self.validate_local_shape()?;
        let mut digest = Sha256::new();
        digest.update(DEMOGRAPHIC_HISTORY_CURSOR_DOMAIN);
        digest.update(self.current_snapshot_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_u64(&mut digest, self.generation.0);
        put_u64(&mut digest, self.intervention_ordinal);
        match (
            self.predecessor_cursor_digest,
            self.event_digest,
            self.structure_transition_digest,
            self.execution_digest,
        ) {
            (None, None, None, None) => digest.update([0]),
            (Some(previous), Some(event), Some(structure), Some(execution)) => {
                digest.update([1]);
                digest.update(previous.as_bytes());
                digest.update(event.as_bytes());
                digest.update(structure.as_bytes());
                digest.update(execution.0);
            }
            _ => return Err(EvolutionError::DemographicHistoryCursorShapeMismatch),
        }
        Ok(DemographicInterventionCursorDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DemographicInterventionCursorDigest([u8; 32]);

impl DemographicInterventionCursorDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DemographicInterventionCursorDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DemographicInterventionCursorDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DemographicInterventionCursorDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AlleleId, HereditarySchemaId, LocusDefinition, LocusId, PopulationStructureModel,
        PopulationStructureProfileId,
    };

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).unwrap()
    }

    fn fixture(
        experiment: &str,
        generation: u64,
    ) -> (
        HereditarySchema,
        PopulationStructureProfile,
        BTreeMap<PopulationId, PopulationGeneticState>,
        BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        MetapopulationSnapshot,
    ) {
        let schema = HereditarySchema::new(
            HereditarySchemaId::new("history-cursor-v0").unwrap(),
            2,
            vec![LocusDefinition::new(
                LocusId::new("focal").unwrap(),
                [allele("a"), allele("b")],
            )
            .unwrap()],
        )
        .unwrap();
        let id = PopulationId::new("a").unwrap();
        let structure = PopulationStructureProfile::new(
            PopulationStructureProfileId::new("single-pop").unwrap(),
            "v1",
            PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
            vec![id.clone()],
            vec![],
        )
        .unwrap();
        let state = PopulationGeneticState::from_counts(
            id.clone(),
            &schema,
            4,
            BTreeMap::from([(
                LocusId::new("focal").unwrap(),
                BTreeMap::from([(allele("a"), 4), (allele("b"), 4)]),
            )]),
        )
        .unwrap();
        let populations = BTreeMap::from([(id.clone(), state)]);
        let point = PopulationTrajectoryPoint::declare_reference_start(
            &schema,
            populations.get(&id).unwrap(),
            EvolutionExperimentId::new(experiment).unwrap(),
            PopulationGeneration(generation),
        )
        .unwrap();
        let points = BTreeMap::from([(id, point)]);
        let snapshot =
            MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
                .unwrap();
        (schema, structure, populations, points, snapshot)
    }

    #[test]
    fn root_binds_exact_snapshot_experiment_and_generation() {
        let (schema, structure, populations, points, snapshot) = fixture("exp-a", 9);
        let root = DemographicInterventionCursor::declare_reference_root(
            &schema,
            &structure,
            &populations,
            &points,
            &snapshot,
        )
        .unwrap();
        assert!(root.is_root());
        assert_eq!(root.intervention_ordinal(), 0);
        root.validate_root_current(&schema, &structure, &populations, &points, &snapshot)
            .unwrap();
    }

    #[test]
    fn roots_at_different_trajectory_coordinates_have_different_identity() {
        let (schema_a, structure_a, populations_a, points_a, snapshot_a) = fixture("exp-a", 9);
        let root_a = DemographicInterventionCursor::declare_reference_root(
            &schema_a,
            &structure_a,
            &populations_a,
            &points_a,
            &snapshot_a,
        )
        .unwrap();
        let (schema_b, structure_b, populations_b, points_b, snapshot_b) = fixture("exp-b", 9);
        let root_b = DemographicInterventionCursor::declare_reference_root(
            &schema_b,
            &structure_b,
            &populations_b,
            &points_b,
            &snapshot_b,
        )
        .unwrap();
        assert_ne!(root_a.canonical_digest().unwrap(), root_b.canonical_digest().unwrap());

        let (schema_c, structure_c, populations_c, points_c, snapshot_c) = fixture("exp-a", 10);
        let root_c = DemographicInterventionCursor::declare_reference_root(
            &schema_c,
            &structure_c,
            &populations_c,
            &points_c,
            &snapshot_c,
        )
        .unwrap();
        assert_ne!(root_a.canonical_digest().unwrap(), root_c.canonical_digest().unwrap());
    }

    #[test]
    fn internal_successor_changes_history_even_if_snapshot_is_unchanged() {
        let (schema, structure, populations, points, snapshot) = fixture("exp-a", 9);
        let root = DemographicInterventionCursor::declare_reference_root(
            &schema,
            &structure,
            &populations,
            &points,
            &snapshot,
        )
        .unwrap();
        let successor = DemographicInterventionCursor::advance_after_validated_execution(
            &root,
            DemographicEventDeclarationDigest([1; 32]),
            DemographicStructureTransitionDigest([2; 32]),
            DemographicEventExecutionDigest([3; 32]),
            &snapshot,
        )
        .unwrap();
        assert_eq!(successor.current_snapshot_digest(), root.current_snapshot_digest());
        assert_eq!(successor.intervention_ordinal(), 1);
        assert_ne!(successor.canonical_digest().unwrap(), root.canonical_digest().unwrap());
    }

    #[test]
    fn event_order_changes_history_identity_even_when_final_snapshot_is_equal() {
        let (schema, structure, populations, points, snapshot) = fixture("exp-a", 9);
        let root = DemographicInterventionCursor::declare_reference_root(
            &schema,
            &structure,
            &populations,
            &points,
            &snapshot,
        )
        .unwrap();
        let a_then = DemographicInterventionCursor::advance_after_validated_execution(
            &root,
            DemographicEventDeclarationDigest([1; 32]),
            DemographicStructureTransitionDigest([11; 32]),
            DemographicEventExecutionDigest([21; 32]),
            &snapshot,
        )
        .unwrap();
        let a_then_b = DemographicInterventionCursor::advance_after_validated_execution(
            &a_then,
            DemographicEventDeclarationDigest([2; 32]),
            DemographicStructureTransitionDigest([12; 32]),
            DemographicEventExecutionDigest([22; 32]),
            &snapshot,
        )
        .unwrap();

        let b_then = DemographicInterventionCursor::advance_after_validated_execution(
            &root,
            DemographicEventDeclarationDigest([2; 32]),
            DemographicStructureTransitionDigest([12; 32]),
            DemographicEventExecutionDigest([22; 32]),
            &snapshot,
        )
        .unwrap();
        let b_then_a = DemographicInterventionCursor::advance_after_validated_execution(
            &b_then,
            DemographicEventDeclarationDigest([1; 32]),
            DemographicStructureTransitionDigest([11; 32]),
            DemographicEventExecutionDigest([21; 32]),
            &snapshot,
        )
        .unwrap();

        assert_ne!(a_then_b.canonical_digest().unwrap(), b_then_a.canonical_digest().unwrap());
    }
}
