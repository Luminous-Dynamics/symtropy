use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_structural_extinction, AlleleId, DemographicEventDeclaration, DemographicEventId,
    DemographicEventKind, DemographicInterventionCursor, DemographicStructureTransition,
    EvolutionExperimentId, HereditarySchema, HereditarySchemaId, LocusDefinition, LocusId,
    MetapopulationSnapshot, PopulationGeneration, PopulationGeneticState, PopulationId,
    PopulationStructureModel, PopulationStructureProfile, PopulationStructureProfileId,
    PopulationTrajectoryPoint,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}
fn pop(id: &str) -> PopulationId {
    PopulationId::new(id).unwrap()
}

struct Fixture {
    schema: HereditarySchema,
    source_structure: PopulationStructureProfile,
    successor_structure: PopulationStructureProfile,
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
}

fn state(
    schema: &HereditarySchema,
    population: PopulationId,
    census: u64,
    a_copies: u64,
) -> PopulationGeneticState {
    let copies = census * u64::from(schema.ploidy);
    let mut counts = BTreeMap::new();
    if a_copies > 0 {
        counts.insert(allele("a"), a_copies);
    }
    if a_copies < copies {
        counts.insert(allele("b"), copies - a_copies);
    }
    PopulationGeneticState::from_counts(
        population,
        schema,
        census,
        BTreeMap::from([(LocusId::new("focal").unwrap(), counts)]),
    )
    .unwrap()
}

fn fixture() -> Fixture {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("extinction-v0").unwrap(),
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
    let source_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("two-populations").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![a.clone(), b.clone()],
        vec![],
    )
    .unwrap();
    let successor_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("survivor-only").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![b.clone()],
        vec![],
    )
    .unwrap();
    let populations = BTreeMap::from([
        (a.clone(), state(&schema, a.clone(), 8, 7)),
        (b.clone(), state(&schema, b.clone(), 11, 13)),
    ]);
    let experiment = EvolutionExperimentId::new("extinction-exp").unwrap();
    let generation = PopulationGeneration(41);
    let points = populations
        .iter()
        .map(|(id, population)| {
            (
                id.clone(),
                PopulationTrajectoryPoint::declare_reference_start(
                    &schema,
                    population,
                    experiment.clone(),
                    generation,
                )
                .unwrap(),
            )
        })
        .collect();
    let snapshot = MetapopulationSnapshot::capture_reference(
        &schema,
        &source_structure,
        &populations,
        &points,
    )
    .unwrap();
    Fixture {
        schema,
        source_structure,
        successor_structure,
        populations,
        points,
        snapshot,
    }
}

fn authorities(
    f: &Fixture,
) -> (
    DemographicEventDeclaration,
    DemographicStructureTransition,
    DemographicInterventionCursor,
) {
    let event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("extinction-a").unwrap(),
        "v1",
        DemographicEventKind::Extinction {
            population: pop("a"),
        },
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap();
    let transition = DemographicStructureTransition::declare_current(
        &event,
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.successor_structure,
    )
    .unwrap();
    let root = DemographicInterventionCursor::declare_reference_root(
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap();
    (event, transition, root)
}

#[test]
fn extinction_removes_live_population_but_preserves_other_state_exactly() {
    let f = fixture();
    let (event, transition, root) = authorities(&f);
    let result = execute_structural_extinction(
        &f.schema,
        &f.source_structure,
        &f.successor_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &root,
        &event,
        &transition,
    )
    .unwrap();

    assert!(!result.populations.contains_key(&pop("a")));
    assert!(!result.points.contains_key(&pop("a")));
    assert_eq!(result.populations.len(), 1);
    assert_eq!(result.points.len(), 1);
    assert_eq!(result.populations[&pop("b")], f.populations[&pop("b")]);
    assert_eq!(result.points[&pop("b")], f.points[&pop("b")]);
    assert!(result.populations.values().all(|population| population.census_individuals > 0));
    assert_eq!(result.snapshot.generation(), PopulationGeneration(41));
    assert_eq!(result.history_cursor.intervention_ordinal(), 1);
}

#[test]
fn extinction_receipt_keeps_last_live_state_and_trajectory_digests() {
    let f = fixture();
    let (event, transition, root) = authorities(&f);
    let result = execute_structural_extinction(
        &f.schema,
        &f.source_structure,
        &f.successor_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &root,
        &event,
        &transition,
    )
    .unwrap();

    assert_eq!(result.provenance.extinct_population_id(), &pop("a"));
    assert_eq!(
        result.provenance.last_live_state_digest(),
        f.populations[&pop("a")].canonical_digest(&f.schema).unwrap()
    );
    assert_eq!(
        result.provenance.last_live_point_digest(),
        f.points[&pop("a")].canonical_digest()
    );
}

#[test]
fn restored_extinction_receipt_requires_exact_current_revalidation() {
    let f = fixture();
    let (event, transition, root) = authorities(&f);
    let result = execute_structural_extinction(
        &f.schema,
        &f.source_structure,
        &f.successor_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &root,
        &event,
        &transition,
    )
    .unwrap();
    let encoded = serde_json::to_string(&result).unwrap();
    let restored: symtropy_evolution_core::DemographicExtinctionExecutionResult =
        serde_json::from_str(&encoded).unwrap();
    restored
        .provenance
        .validate_current(
            &f.schema,
            &f.source_structure,
            &f.successor_structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            &root,
            &event,
            &transition,
            &restored,
        )
        .unwrap();

    let wrong_successor = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("wrong-survivor-authority").unwrap(),
        "v2",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![pop("b")],
        vec![],
    )
    .unwrap();
    assert!(restored
        .provenance
        .validate_current(
            &f.schema,
            &f.source_structure,
            &wrong_successor,
            &f.populations,
            &f.points,
            &f.snapshot,
            &root,
            &event,
            &transition,
            &restored,
        )
        .is_err());
}

#[test]
fn sole_population_cannot_be_encoded_as_empty_successor_structure() {
    assert!(PopulationStructureProfile::new(
        PopulationStructureProfileId::new("empty-after-extinction").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        Vec::new(),
        vec![],
    )
    .is_err());
}
