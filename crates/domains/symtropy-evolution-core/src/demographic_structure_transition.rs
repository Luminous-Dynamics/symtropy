use crate::{
    canonical::{fmt_hex, put_text, put_u64},
    DemographicEventDeclaration, DemographicEventDeclarationDigest, DemographicEventKind,
    EvolutionError, EvolutionExperimentId, HereditarySchema, MetapopulationSnapshot,
    PopulationGeneration, PopulationGeneticState, PopulationId, PopulationStructureProfile,
    PopulationStructureProfileDigest, PopulationTrajectoryPoint,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fmt};

const DEMOGRAPHIC_STRUCTURE_TRANSITION_DOMAIN: &[u8] =
    b"symtropy:evolution:demographic-structure-transition:v1\0";

/// Revalidatable structure/provenance authority for one demographic event.
///
/// This object proves that the supplied successor population membership is
/// compatible with one exact event declaration. It does not execute genetic
/// sampling and does not claim that changed continuous-migration edges were
/// caused by the event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemographicStructureTransition {
    event_digest: DemographicEventDeclarationDigest,
    source_structure_digest: PopulationStructureProfileDigest,
    successor_structure_digest: PopulationStructureProfileDigest,
    experiment_id: EvolutionExperimentId,
    generation: PopulationGeneration,
}

impl DemographicStructureTransition {
    #[allow(clippy::too_many_arguments)]
    pub fn declare_current(
        event: &DemographicEventDeclaration,
        schema: &HereditarySchema,
        source_structure: &PopulationStructureProfile,
        source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        source_snapshot: &MetapopulationSnapshot,
        successor_structure: &PopulationStructureProfile,
    ) -> Result<Self, EvolutionError> {
        event.validate_current(
            schema,
            source_structure,
            source_populations,
            source_points,
            source_snapshot,
        )?;
        successor_structure.validate()?;
        validate_successor_membership(event.kind(), source_structure, successor_structure)?;

        let transition = Self {
            event_digest: event.canonical_digest()?,
            source_structure_digest: source_structure.canonical_digest()?,
            successor_structure_digest: successor_structure.canonical_digest()?,
            experiment_id: event.experiment_id().clone(),
            generation: event.generation(),
        };
        transition.validate_current(
            event,
            schema,
            source_structure,
            source_populations,
            source_points,
            source_snapshot,
            successor_structure,
        )?;
        Ok(transition)
    }

    pub fn event_digest(&self) -> DemographicEventDeclarationDigest {
        self.event_digest
    }

    pub fn source_structure_digest(&self) -> PopulationStructureProfileDigest {
        self.source_structure_digest
    }

    pub fn successor_structure_digest(&self) -> PopulationStructureProfileDigest {
        self.successor_structure_digest
    }

    pub fn experiment_id(&self) -> &EvolutionExperimentId {
        &self.experiment_id
    }

    pub fn generation(&self) -> PopulationGeneration {
        self.generation
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        event: &DemographicEventDeclaration,
        schema: &HereditarySchema,
        source_structure: &PopulationStructureProfile,
        source_populations: &BTreeMap<PopulationId, PopulationGeneticState>,
        source_points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
        source_snapshot: &MetapopulationSnapshot,
        successor_structure: &PopulationStructureProfile,
    ) -> Result<(), EvolutionError> {
        event.validate_current(
            schema,
            source_structure,
            source_populations,
            source_points,
            source_snapshot,
        )?;
        successor_structure.validate()?;
        validate_successor_membership(event.kind(), source_structure, successor_structure)?;

        if event.canonical_digest()? != self.event_digest {
            return Err(EvolutionError::DemographicStructureTransitionEventMismatch);
        }
        if source_structure.canonical_digest()? != self.source_structure_digest {
            return Err(EvolutionError::DemographicStructureAuthorityMismatch);
        }
        if successor_structure.canonical_digest()? != self.successor_structure_digest {
            return Err(EvolutionError::DemographicSuccessorStructureMismatch);
        }
        if event.experiment_id() != &self.experiment_id {
            return Err(EvolutionError::PopulationExperimentMismatch);
        }
        if event.generation() != self.generation {
            return Err(EvolutionError::PopulationGenerationMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> DemographicStructureTransitionDigest {
        let mut digest = Sha256::new();
        digest.update(DEMOGRAPHIC_STRUCTURE_TRANSITION_DOMAIN);
        digest.update(self.event_digest.as_bytes());
        digest.update(self.source_structure_digest.as_bytes());
        digest.update(self.successor_structure_digest.as_bytes());
        put_text(&mut digest, self.experiment_id.as_str());
        put_u64(&mut digest, self.generation.0);
        DemographicStructureTransitionDigest(digest.finalize().into())
    }
}

fn validate_successor_membership(
    kind: &DemographicEventKind,
    source_structure: &PopulationStructureProfile,
    successor_structure: &PopulationStructureProfile,
) -> Result<(), EvolutionError> {
    let source = source_structure.populations();
    let successor = successor_structure.populations();
    let mut expected: BTreeSet<PopulationId> = source.iter().cloned().collect();

    match kind {
        DemographicEventKind::CensusResize { .. }
        | DemographicEventKind::PulseAdmixture { .. } => {
            if source_structure.canonical_digest()? != successor_structure.canonical_digest()? {
                return Err(EvolutionError::DemographicMembershipPreservingStructureChanged);
            }
            return Ok(());
        }
        DemographicEventKind::FounderEvent {
            founded_population, ..
        } => {
            expected.insert(founded_population.clone());
        }
        DemographicEventKind::Recolonization {
            recolonized_population,
            ..
        } => {
            expected.insert(recolonized_population.clone());
        }
        DemographicEventKind::Extinction { population } => {
            expected.remove(population);
        }
        DemographicEventKind::PopulationSplit { source, daughters } => {
            expected.remove(source);
            for daughter in daughters {
                expected.insert(daughter.population_id().clone());
            }
        }
    }

    if expected.is_empty() {
        return Err(EvolutionError::DemographicCannotProduceEmptyMetapopulation);
    }
    if &expected != successor {
        return Err(EvolutionError::DemographicSuccessorSetMismatch);
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DemographicStructureTransitionDigest([u8; 32]);

impl DemographicStructureTransitionDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DemographicStructureTransitionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DemographicStructureTransitionDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DemographicStructureTransitionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AlleleId, DemographicEventId, DaughterPopulation, HereditarySchemaId, LocusDefinition,
        LocusId, PopulationStructureModel, PopulationStructureProfileId,
    };

    fn pop(id: &str) -> PopulationId {
        PopulationId::new(id).unwrap()
    }

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

    fn fixture() -> Fixture {
        let schema = HereditarySchema::new(
            HereditarySchemaId::new("structure-transition-v0").unwrap(),
            2,
            vec![LocusDefinition::new(
                LocusId::new("focal").unwrap(),
                [allele("a"), allele("b")],
            )
            .unwrap()],
        )
        .unwrap();
        let a = pop("a");
        let b = pop("b");
        let structure = PopulationStructureProfile::new(
            PopulationStructureProfileId::new("source-structure").unwrap(),
            "v1",
            PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
            vec![a.clone(), b.clone()],
            vec![],
        )
        .unwrap();
        let state = |id: PopulationId, a_copies: u64| {
            PopulationGeneticState::from_counts(
                id,
                &schema,
                4,
                BTreeMap::from([(
                    LocusId::new("focal").unwrap(),
                    BTreeMap::from([(allele("a"), a_copies), (allele("b"), 8 - a_copies)]),
                )]),
            )
            .unwrap()
        };
        let populations = BTreeMap::from([
            (a.clone(), state(a.clone(), 6)),
            (b.clone(), state(b.clone(), 2)),
        ]);
        let experiment = EvolutionExperimentId::new("structure-transition-exp").unwrap();
        let points = populations
            .iter()
            .map(|(id, state)| {
                (
                    id.clone(),
                    PopulationTrajectoryPoint::declare_reference_start(
                        &schema,
                        state,
                        experiment.clone(),
                        PopulationGeneration(7),
                    )
                    .unwrap(),
                )
            })
            .collect();
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

    fn successor(ids: Vec<PopulationId>, profile_id: &str) -> PopulationStructureProfile {
        PopulationStructureProfile::new(
            PopulationStructureProfileId::new(profile_id).unwrap(),
            "v1",
            PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
            ids,
            vec![],
        )
        .unwrap()
    }

    fn event(f: &Fixture, kind: DemographicEventKind) -> DemographicEventDeclaration {
        DemographicEventDeclaration::declare_current(
            DemographicEventId::new("event").unwrap(),
            "v1",
            kind,
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap()
    }

    #[test]
    fn founder_requires_exactly_the_declared_new_member() {
        let f = fixture();
        let declaration = event(
            &f,
            DemographicEventKind::FounderEvent {
                source: pop("a"),
                founded_population: pop("c"),
                founder_census: 2,
            },
        );
        let ok = successor(vec![pop("a"), pop("b"), pop("c")], "successor");
        let transition = DemographicStructureTransition::declare_current(
            &declaration,
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            &ok,
        )
        .unwrap();
        transition
            .validate_current(
                &declaration,
                &f.schema,
                &f.structure,
                &f.populations,
                &f.points,
                &f.snapshot,
                &ok,
            )
            .unwrap();

        let wrong = successor(vec![pop("a"), pop("b"), pop("d")], "wrong");
        assert_eq!(
            DemographicStructureTransition::declare_current(
                &declaration,
                &f.schema,
                &f.structure,
                &f.populations,
                &f.points,
                &f.snapshot,
                &wrong,
            ),
            Err(EvolutionError::DemographicSuccessorSetMismatch)
        );
    }

    #[test]
    fn split_and_extinction_have_exact_membership_semantics() {
        let f = fixture();
        let split = event(
            &f,
            DemographicEventKind::PopulationSplit {
                source: pop("a"),
                daughters: vec![
                    DaughterPopulation::new(pop("a-east"), 2).unwrap(),
                    DaughterPopulation::new(pop("a-west"), 2).unwrap(),
                ],
            },
        );
        let split_successor = successor(
            vec![pop("b"), pop("a-west"), pop("a-east")],
            "split-successor",
        );
        DemographicStructureTransition::declare_current(
            &split,
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            &split_successor,
        )
        .unwrap();

        let extinct = event(
            &f,
            DemographicEventKind::Extinction {
                population: pop("b"),
            },
        );
        let extinct_successor = successor(vec![pop("a")], "extinct-successor");
        DemographicStructureTransition::declare_current(
            &extinct,
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            &extinct_successor,
        )
        .unwrap();
    }

    #[test]
    fn membership_preserving_event_cannot_smuggle_structure_change() {
        let f = fixture();
        let declaration = event(
            &f,
            DemographicEventKind::CensusResize {
                population: pop("a"),
                target_census: 2,
            },
        );
        let merely_same_members = successor(vec![pop("a"), pop("b")], "different-authority");
        assert_eq!(
            DemographicStructureTransition::declare_current(
                &declaration,
                &f.schema,
                &f.structure,
                &f.populations,
                &f.points,
                &f.snapshot,
                &merely_same_members,
            ),
            Err(EvolutionError::DemographicMembershipPreservingStructureChanged)
        );
    }
}
