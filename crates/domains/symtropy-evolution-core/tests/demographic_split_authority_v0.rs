use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_resize_bottleneck, execute_conservative_population_split, AlleleId,
    DaughterPopulation, DemographicEventDeclaration, DemographicEventId, DemographicEventKind,
    DemographicInterventionCursor, DemographicStructureTransition, EvolutionExperimentId,
    HereditarySchema, HereditarySchemaId, LocusDefinition, LocusId, MetapopulationSnapshot,
    ParentalSourceEdge, PopulationGeneration, PopulationGeneticState, PopulationId,
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

fn fixture() -> Fixture {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("split-authority-v0").unwrap(),
        2,
        vec![
            LocusDefinition::new(
                LocusId::new("focal").unwrap(),
                [allele("a"), allele("b")],
            )
            .unwrap(),
            LocusDefinition::new(
                LocusId::new("marker").unwrap(),
                [allele("x"), allele("y")],
            )
            .unwrap(),
        ],
    )
    .unwrap();

    let ancestral = pop("ancestral");
    let witness = pop("witness");
    let east = pop("east");
    let west = pop("west");

    let source_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("before-split-authority").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![ancestral.clone(), witness.clone()],
        vec![],
    )
    .unwrap();
    let successor_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("after-split-authority").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![east, west, witness.clone()],
        vec![],
    )
    .unwrap();

    let ancestral_state = PopulationGeneticState::from_counts(
        ancestral.clone(),
        &schema,
        10,
        BTreeMap::from([
            (
                LocusId::new("focal").unwrap(),
                BTreeMap::from([(allele("a"), 9), (allele("b"), 11)]),
            ),
            (
                LocusId::new("marker").unwrap(),
                BTreeMap::from([(allele("x"), 13), (allele("y"), 7)]),
            ),
        ]),
    )
    .unwrap();
    let witness_state = PopulationGeneticState::from_counts(
        witness.clone(),
        &schema,
        5,
        BTreeMap::from([
            (
                LocusId::new("focal").unwrap(),
                BTreeMap::from([(allele("a"), 6), (allele("b"), 4)]),
            ),
            (
                LocusId::new("marker").unwrap(),
                BTreeMap::from([(allele("x"), 4), (allele("y"), 6)]),
            ),
        ]),
    )
    .unwrap();

    let populations = BTreeMap::from([
        (ancestral.clone(), ancestral_state),
        (witness.clone(), witness_state),
    ]);
    let experiment = EvolutionExperimentId::new("split-authority-exp").unwrap();
    let generation = PopulationGeneration(41);
    let points = populations
        .iter()
        .map(|(id, state)| {
            (
                id.clone(),
                PopulationTrajectoryPoint::declare_reference_start(
                    &schema,
                    state,
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

fn split_event(f: &Fixture) -> DemographicEventDeclaration {
    DemographicEventDeclaration::declare_current(
        DemographicEventId::new("split-authority-event").unwrap(),
        "v1",
        DemographicEventKind::PopulationSplit {
            source: pop("ancestral"),
            daughters: vec![
                DaughterPopulation::new(pop("west"), 6).unwrap(),
                DaughterPopulation::new(pop("east"), 4).unwrap(),
            ],
        },
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap()
}

#[test]
fn each_daughter_has_exact_declared_copy_capacity_at_every_locus() {
    let f = fixture();
    let event = split_event(&f);
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
    let result = execute_conservative_population_split(
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

    for (id, census) in [(pop("east"), 4_u64), (pop("west"), 6_u64)] {
        let state = &result.populations[&id];
        assert_eq!(state.census_individuals, census);
        for counts in state.allele_copy_counts.values() {
            assert_eq!(counts.values().copied().sum::<u64>(), census * 2);
        }
    }
}

#[test]
fn changed_successor_structure_stales_split_execution_authority() {
    let f = fixture();
    let event = split_event(&f);
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
    let result = execute_conservative_population_split(
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

    let changed_successor = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("after-split-changed").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![pop("east"), pop("west"), pop("witness")],
        vec![ParentalSourceEdge::new(pop("east"), pop("west"), 100_000).unwrap()],
    )
    .unwrap();

    assert!(result
        .provenance
        .validate_current(
            &f.schema,
            &f.source_structure,
            &changed_successor,
            &f.populations,
            &f.points,
            &f.snapshot,
            &root,
            &event,
            &transition,
            &result,
        )
        .is_err());
}

#[test]
fn split_executor_rejects_non_root_history_cursor_in_v0() {
    let f = fixture();
    let noop = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("explicit-noop-before-split").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("ancestral"),
            target_census: 10,
        },
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap();
    let noop_transition = DemographicStructureTransition::declare_current(
        &noop,
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.source_structure,
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
    let noop_result = execute_census_resize_bottleneck(
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &root,
        &noop,
        &noop_transition,
    )
    .unwrap();

    assert_eq!(noop_result.snapshot, f.snapshot);
    assert_eq!(noop_result.history_cursor.intervention_ordinal(), 1);

    let split = split_event(&f);
    let split_transition = DemographicStructureTransition::declare_current(
        &split,
        &f.schema,
        &f.source_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.successor_structure,
    )
    .unwrap();

    assert!(execute_conservative_population_split(
        &f.schema,
        &f.source_structure,
        &f.successor_structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &noop_result.history_cursor,
        &split,
        &split_transition,
    )
    .is_err());
}
