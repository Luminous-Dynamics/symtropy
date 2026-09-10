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
            if links_present.iter().any(|present| *present) {
                return Err(EvolutionError::DemographicHistoryCursorShapeMismatch);
            }
        } else if links_present.iter().any(|present| !*present) {
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
        AlleleId, DemographicEventDeclaration, DemographicEventId, DemographicEventKind,
        DemographicStructureTransition, HereditarySchemaId, LocusDefinition, LocusId,
        PopulationStructureModel, PopulationStructureProfileId,
    };

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).unwrap()
    }

    struct Fixture {
        schema: HereditarySchema,
        structure: PopulationStructureProfile,
        populations: BTreeMap<PopulationId, PopulationGeneticState>,
        points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        snapshot: MetapopulationSnapshot,
    }

    fn fixture(experiment: &str, generation: u64) -> Fixture {
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
        Fixture {
            schema,
            structure,
            populations,
            points,
            snapshot,
        }
    }

    fn root(f: &Fixture) -> DemographicInterventionCursor {
        DemographicInterventionCursor::declare_reference_root(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap()
    }

    fn event_authorities(
        f: &Fixture,
        event_id: &str,
    ) -> (DemographicEventDeclarationDigest, DemographicStructureTransitionDigest) {
        let event = DemographicEventDeclaration::declare_current(
            DemographicEventId::new(event_id).unwrap(),
            "v1",
            DemographicEventKind::CensusResize {
                population: PopulationId::new("a").unwrap(),
                target_census: 4,
            },
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap();
        let transition = DemographicStructureTransition::declare_current(
            &event,
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            &f.structure,
        )
        .unwrap();
        (event.canonical_digest().unwrap(), transition.canonical_digest())
    }

    #[test]
    fn root_binds_exact_snapshot_experiment_and_generation() {
        let f = fixture("exp-a", 9);
        let root = root(&f);
        assert!(root.is_root());
        assert_eq!(root.intervention_ordinal(), 0);
        root.validate_root_current(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap();
    }

    #[test]
    fn roots_at_different_trajectory_coordinates_have_different_identity() {
        let a = fixture("exp-a", 9);
        let b = fixture("exp-b", 9);
        let c = fixture("exp-a", 10);
        assert_ne!(root(&a).canonical_digest().unwrap(), root(&b).canonical_digest().unwrap());
        assert_ne!(root(&a).canonical_digest().unwrap(), root(&c).canonical_digest().unwrap());
    }

    #[test]
    fn internal_successor_changes_history_even_if_snapshot_is_unchanged() {
        let f = fixture("exp-a", 9);
        let root = root(&f);
        let (event, structure) = event_authorities(&f, "event-a");
        let successor = DemographicInterventionCursor::advance_after_validated_execution(
            &root,
            event,
            structure,
            DemographicEventExecutionDigest([3; 32]),
            &f.snapshot,
        )
        .unwrap();
        assert_eq!(successor.current_snapshot_digest(), root.current_snapshot_digest());
        assert_eq!(successor.intervention_ordinal(), 1);
        assert_ne!(successor.canonical_digest().unwrap(), root.canonical_digest().unwrap());
    }

    #[test]
    fn event_order_changes_history_identity_even_when_final_snapshot_is_equal() {
        let f = fixture("exp-a", 9);
        let root = root(&f);
        let (event_a, structure_a) = event_authorities(&f, "event-a");
        let (event_b, structure_b) = event_authorities(&f, "event-b");

        let a_then = DemographicInterventionCursor::advance_after_validated_execution(
            &root,
            event_a,
            structure_a,
            DemographicEventExecutionDigest([21; 32]),
            &f.snapshot,
        )
        .unwrap();
        let a_then_b = DemographicInterventionCursor::advance_after_validated_execution(
            &a_then,
            event_b,
            structure_b,
            DemographicEventExecutionDigest([22; 32]),
            &f.snapshot,
        )
        .unwrap();

        let b_then = DemographicInterventionCursor::advance_after_validated_execution(
            &root,
            event_b,
            structure_b,
            DemographicEventExecutionDigest([22; 32]),
            &f.snapshot,
        )
        .unwrap();
        let b_then_a = DemographicInterventionCursor::advance_after_validated_execution(
            &b_then,
            event_a,
            structure_a,
            DemographicEventExecutionDigest([21; 32]),
            &f.snapshot,
        )
        .unwrap();

        assert_ne!(a_then_b.canonical_digest().unwrap(), b_then_a.canonical_digest().unwrap());
    }
}
