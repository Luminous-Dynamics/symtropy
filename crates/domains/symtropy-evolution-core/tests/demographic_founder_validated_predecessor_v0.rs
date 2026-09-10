use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_resize_bottleneck, execute_founder_or_recolonization_sample,
    execute_founder_or_recolonization_sample_after_proven_history, AlleleId,
    DemographicEventDeclaration, DemographicEventId, DemographicEventKind,
    DemographicExecutionEvidence, DemographicInterventionCursor, DemographicInterventionProofBundle,
    DemographicInterventionProofStep, DemographicStructureTransition, EvolutionExperimentId,
    HereditarySchema, HereditarySchemaId, LocusDefinition, LocusId, MetapopulationSnapshot,
    PopulationGeneration, PopulationGeneticState, PopulationId, PopulationStructureModel,
    PopulationStructureProfile, PopulationStructureProfileId, PopulationTrajectoryPoint,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}

fn pop(id: &str) -> PopulationId {
    PopulationId::new(id).unwrap()
}

struct Fixture {
    schema: HereditarySchema,
    structure: PopulationStructureProfile,
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
    root: DemographicInterventionCursor,
}

fn fixture() -> Fixture {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("founder-proven-predecessor-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();
    let source = pop("source");
    let structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("source-only").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![source.clone()],
        vec![],
    )
    .unwrap();
    let state = PopulationGeneticState::from_counts(
        source.clone(),
        &schema,
        10,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele("a"), 8), (allele("b"), 12)]),
        )]),
    )
    .unwrap();
    let populations = BTreeMap::from([(source.clone(), state)]);
    let point = PopulationTrajectoryPoint::declare_reference_start(
        &schema,
        &populations[&source],
        EvolutionExperimentId::new("founder-proven-exp").unwrap(),
        PopulationGeneration(44),
    )
    .unwrap();
    let points = BTreeMap::from([(source, point)]);
    let snapshot =
        MetapopulationSnapshot::capture_reference(&schema, &structure, &populations, &points)
            .unwrap();
    let root = DemographicInterventionCursor::declare_reference_root(
        &schema,
        &structure,
        &populations,
        &points,
        &snapshot,
    )
    .unwrap();
    Fixture {
        schema,
        structure,
        populations,
        points,
        snapshot,
        root,
    }
}

fn source_noop_event(f: &Fixture) -> DemographicEventDeclaration {
    DemographicEventDeclaration::declare_current(
        DemographicEventId::new("source-noop").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("source"),
            target_census: 10,
        },
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap()
}

fn same_structure_transition(
    f: &Fixture,
    event: &DemographicEventDeclaration,
) -> DemographicStructureTransition {
    DemographicStructureTransition::declare_current(
        event,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.structure,
    )
    .unwrap()
}

fn successor_structure(
    source_structure: &PopulationStructureProfile,
    destination: &str,
) -> PopulationStructureProfile {
    let _ = source_structure;
    PopulationStructureProfile::new(
        PopulationStructureProfileId::new(format!("with-{destination}")).unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![pop("source"), pop(destination)],
        vec![],
    )
    .unwrap()
}

#[test]
fn proof_minted_token_authorizes_founder_after_non_root_history() {
    let f = fixture();
    let first_event = source_noop_event(&f);
    let first_transition = same_structure_transition(&f, &first_event);
    let first = execute_census_resize_bottleneck(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.root,
        &first_event,
        &first_transition,
    )
    .unwrap();
    assert_eq!(first.snapshot, f.snapshot);
    assert_eq!(first.history_cursor.intervention_ordinal(), 1);

    let bundle = DemographicInterventionProofBundle::declare_current(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        vec![DemographicInterventionProofStep::new(
            first_event,
            first_transition,
            f.structure.clone(),
            DemographicExecutionEvidence::CensusResize(first.clone()),
        )],
    )
    .unwrap();
    let token = bundle
        .mint_validated_final_source(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap();

    let successor = successor_structure(&f.structure, "colony");
    let founder_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("found-colony").unwrap(),
        "v1",
        DemographicEventKind::FounderEvent {
            source: pop("source"),
            founded_population: pop("colony"),
            founder_census: 4,
        },
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    )
    .unwrap();
    let founder_transition = DemographicStructureTransition::declare_current(
        &founder_event,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &successor,
    )
    .unwrap();

    assert!(execute_founder_or_recolonization_sample(
        &f.schema,
        &f.structure,
        &successor,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &founder_event,
        &founder_transition,
    )
    .is_err());

    let founded = execute_founder_or_recolonization_sample_after_proven_history(
        &token,
        &f.schema,
        &f.structure,
        &successor,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &founder_event,
        &founder_transition,
    )
    .unwrap();

    assert_eq!(founded.history_cursor.intervention_ordinal(), 2);
    assert_eq!(founded.snapshot.generation(), PopulationGeneration(44));
    assert_eq!(founded.populations[&pop("source")], first.populations[&pop("source")]);
    assert_eq!(founded.points[&pop("source")], first.points[&pop("source")]);
    assert_eq!(founded.populations[&pop("colony")].census_individuals, 4);

    founded
        .provenance
        .validate_after_proven_predecessor(
            &token,
            &f.schema,
            &f.structure,
            &successor,
            &first.populations,
            &first.points,
            &first.snapshot,
            &first.history_cursor,
            &founder_event,
            &founder_transition,
            &founded,
        )
        .unwrap();
}

#[test]
fn founder_token_is_stale_after_structure_and_snapshot_advance() {
    let f = fixture();
    let first_event = source_noop_event(&f);
    let first_transition = same_structure_transition(&f, &first_event);
    let first = execute_census_resize_bottleneck(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.root,
        &first_event,
        &first_transition,
    )
    .unwrap();
    let bundle = DemographicInterventionProofBundle::declare_current(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        vec![DemographicInterventionProofStep::new(
            first_event,
            first_transition,
            f.structure.clone(),
            DemographicExecutionEvidence::CensusResize(first.clone()),
        )],
    )
    .unwrap();
    let token = bundle
        .mint_validated_final_source(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap();

    let successor = successor_structure(&f.structure, "colony");
    let founder_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("first-founder").unwrap(),
        "v1",
        DemographicEventKind::FounderEvent {
            source: pop("source"),
            founded_population: pop("colony"),
            founder_census: 3,
        },
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    )
    .unwrap();
    let founder_transition = DemographicStructureTransition::declare_current(
        &founder_event,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &successor,
    )
    .unwrap();
    let founded = execute_founder_or_recolonization_sample_after_proven_history(
        &token,
        &f.schema,
        &f.structure,
        &successor,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &founder_event,
        &founder_transition,
    )
    .unwrap();

    let successor_2 = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("with-two-colonies").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![pop("source"), pop("colony"), pop("colony-2")],
        vec![],
    )
    .unwrap();
    let second_founder_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("second-founder").unwrap(),
        "v1",
        DemographicEventKind::FounderEvent {
            source: pop("source"),
            founded_population: pop("colony-2"),
            founder_census: 2,
        },
        &f.schema,
        &successor,
        &founded.populations,
        &founded.points,
        &founded.snapshot,
    )
    .unwrap();
    let second_transition = DemographicStructureTransition::declare_current(
        &second_founder_event,
        &f.schema,
        &successor,
        &founded.populations,
        &founded.points,
        &founded.snapshot,
        &successor_2,
    )
    .unwrap();

    assert!(execute_founder_or_recolonization_sample_after_proven_history(
        &token,
        &f.schema,
        &successor,
        &successor_2,
        &founded.populations,
        &founded.points,
        &founded.snapshot,
        &founded.history_cursor,
        &second_founder_event,
        &second_transition,
    )
    .is_err());
}
