use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_resize_bottleneck, execute_census_resize_bottleneck_after_proven_history,
    AlleleId, DemographicEventDeclaration, DemographicEventId, DemographicEventKind,
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
        HereditarySchemaId::new("validated-predecessor-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();
    let population_id = pop("population");
    let structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("single-pop").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![population_id.clone()],
        vec![],
    )
    .unwrap();
    let population = PopulationGeneticState::from_counts(
        population_id.clone(),
        &schema,
        10,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele("a"), 10), (allele("b"), 10)]),
        )]),
    )
    .unwrap();
    let populations = BTreeMap::from([(population_id.clone(), population)]);
    let point = PopulationTrajectoryPoint::declare_reference_start(
        &schema,
        &populations[&population_id],
        EvolutionExperimentId::new("validated-predecessor-exp").unwrap(),
        PopulationGeneration(31),
    )
    .unwrap();
    let points = BTreeMap::from([(population_id, point)]);
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

fn census_event(
    event_id: &str,
    target_census: u64,
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: &MetapopulationSnapshot,
) -> DemographicEventDeclaration {
    DemographicEventDeclaration::declare_current(
        DemographicEventId::new(event_id).unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("population"),
            target_census,
        },
        schema,
        structure,
        populations,
        points,
        snapshot,
    )
    .unwrap()
}

fn transition(
    event: &DemographicEventDeclaration,
    schema: &HereditarySchema,
    structure: &PopulationStructureProfile,
    populations: &BTreeMap<PopulationId, PopulationGeneticState>,
    points: &BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: &MetapopulationSnapshot,
) -> DemographicStructureTransition {
    DemographicStructureTransition::declare_current(
        event,
        schema,
        structure,
        populations,
        points,
        snapshot,
        structure,
    )
    .unwrap()
}

#[test]
fn proof_minted_token_authorizes_second_same_generation_bottleneck() {
    let f = fixture();
    let first_event = census_event(
        "first-noop",
        10,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    );
    let first_transition = transition(
        &first_event,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    );
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
    assert_eq!(first.history_cursor.intervention_ordinal(), 1);
    assert_eq!(first.snapshot, f.snapshot);

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
    assert_eq!(token.intervention_ordinal(), 1);
    assert_eq!(token.snapshot_digest(), first.snapshot.canonical_digest());
    assert_eq!(token.cursor_digest(), first.history_cursor.canonical_digest().unwrap());

    let second_event = census_event(
        "second-bottleneck",
        5,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    );
    let second_transition = transition(
        &second_event,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    );

    assert!(execute_census_resize_bottleneck(
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &second_event,
        &second_transition,
    )
    .is_err());

    let second = execute_census_resize_bottleneck_after_proven_history(
        &token,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &second_event,
        &second_transition,
    )
    .unwrap();

    assert_eq!(second.history_cursor.intervention_ordinal(), 2);
    assert_eq!(second.populations[&pop("population")].census_individuals, 5);
    second
        .provenance
        .validate_after_proven_predecessor(
            &token,
            &f.schema,
            &f.structure,
            &first.populations,
            &first.points,
            &first.snapshot,
            &first.history_cursor,
            &second_event,
            &second_transition,
            &second,
        )
        .unwrap();
}

#[test]
fn predecessor_token_is_bound_to_exact_prefix_cut() {
    let f = fixture();
    let first_event = census_event(
        "first-cut",
        10,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    );
    let first_transition = transition(
        &first_event,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    );
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

    let second_event = census_event(
        "second-cut",
        5,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    );
    let second_transition = transition(
        &second_event,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    );
    let second = execute_census_resize_bottleneck_after_proven_history(
        &token,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &second_event,
        &second_transition,
    )
    .unwrap();

    let third_event = census_event(
        "third-cut",
        4,
        &f.schema,
        &f.structure,
        &second.populations,
        &second.points,
        &second.snapshot,
    );
    let third_transition = transition(
        &third_event,
        &f.schema,
        &f.structure,
        &second.populations,
        &second.points,
        &second.snapshot,
    );

    assert!(execute_census_resize_bottleneck_after_proven_history(
        &token,
        &f.schema,
        &f.structure,
        &second.populations,
        &second.points,
        &second.snapshot,
        &second.history_cursor,
        &third_event,
        &third_transition,
    )
    .is_err());
}

#[test]
fn empty_root_bundle_cannot_mint_non_root_runtime_authority() {
    let f = fixture();
    let bundle = DemographicInterventionProofBundle::declare_current(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        vec![],
    )
    .unwrap();
    assert!(bundle
        .mint_validated_final_source(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .is_err());
}
