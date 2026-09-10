use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_resize_bottleneck, execute_conservative_population_split,
    execute_conservative_population_split_after_proven_history, AlleleId, DaughterPopulation,
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

#[test]
fn proof_minted_token_authorizes_conservative_split() {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("split-proven-predecessor-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();
    let ancestral = pop("ancestral");
    let witness = pop("witness");
    let source_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("before-proven-split").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![ancestral.clone(), witness.clone()],
        vec![],
    )
    .unwrap();
    let populations = BTreeMap::from([
        (
            ancestral.clone(),
            PopulationGeneticState::from_counts(
                ancestral.clone(),
                &schema,
                10,
                BTreeMap::from([(
                    LocusId::new("focal").unwrap(),
                    BTreeMap::from([(allele("a"), 9), (allele("b"), 11)]),
                )]),
            )
            .unwrap(),
        ),
        (
            witness.clone(),
            PopulationGeneticState::from_counts(
                witness.clone(),
                &schema,
                4,
                BTreeMap::from([(
                    LocusId::new("focal").unwrap(),
                    BTreeMap::from([(allele("a"), 4), (allele("b"), 4)]),
                )]),
            )
            .unwrap(),
        ),
    ]);
    let experiment = EvolutionExperimentId::new("split-proven-exp").unwrap();
    let generation = PopulationGeneration(63);
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
        .collect::<BTreeMap<_, _>>();
    let snapshot = MetapopulationSnapshot::capture_reference(
        &schema,
        &source_structure,
        &populations,
        &points,
    )
    .unwrap();
    let root = DemographicInterventionCursor::declare_reference_root(
        &schema,
        &source_structure,
        &populations,
        &points,
        &snapshot,
    )
    .unwrap();

    let noop_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("ancestral-noop").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: ancestral.clone(),
            target_census: 10,
        },
        &schema,
        &source_structure,
        &populations,
        &points,
        &snapshot,
    )
    .unwrap();
    let noop_transition = DemographicStructureTransition::declare_current(
        &noop_event,
        &schema,
        &source_structure,
        &populations,
        &points,
        &snapshot,
        &source_structure,
    )
    .unwrap();
    let first = execute_census_resize_bottleneck(
        &schema,
        &source_structure,
        &populations,
        &points,
        &snapshot,
        &root,
        &noop_event,
        &noop_transition,
    )
    .unwrap();
    let bundle = DemographicInterventionProofBundle::declare_current(
        &schema,
        &source_structure,
        &populations,
        &points,
        &snapshot,
        vec![DemographicInterventionProofStep::new(
            noop_event,
            noop_transition,
            source_structure.clone(),
            DemographicExecutionEvidence::CensusResize(first.clone()),
        )],
    )
    .unwrap();
    let token = bundle
        .mint_validated_final_source(
            &schema,
            &source_structure,
            &populations,
            &points,
            &snapshot,
        )
        .unwrap();

    let successor_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("after-proven-split").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![pop("east"), pop("west"), witness.clone()],
        vec![],
    )
    .unwrap();
    let split_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("proven-split").unwrap(),
        "v1",
        DemographicEventKind::PopulationSplit {
            source: ancestral.clone(),
            daughters: vec![
                DaughterPopulation::new(pop("west"), 6).unwrap(),
                DaughterPopulation::new(pop("east"), 4).unwrap(),
            ],
        },
        &schema,
        &source_structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    )
    .unwrap();
    let split_transition = DemographicStructureTransition::declare_current(
        &split_event,
        &schema,
        &source_structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &successor_structure,
    )
    .unwrap();

    assert!(execute_conservative_population_split(
        &schema,
        &source_structure,
        &successor_structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &split_event,
        &split_transition,
    )
    .is_err());

    let split = execute_conservative_population_split_after_proven_history(
        &token,
        &schema,
        &source_structure,
        &successor_structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &split_event,
        &split_transition,
    )
    .unwrap();

    assert!(!split.populations.contains_key(&ancestral));
    assert_eq!(split.populations[&pop("east")].census_individuals, 4);
    assert_eq!(split.populations[&pop("west")].census_individuals, 6);
    assert_eq!(split.populations[&witness], first.populations[&witness]);
    assert_eq!(split.points[&witness], first.points[&witness]);
    assert_eq!(split.snapshot.generation(), generation);
    assert_eq!(split.history_cursor.intervention_ordinal(), 2);

    let locus = LocusId::new("focal").unwrap();
    let mut daughter_union = BTreeMap::<AlleleId, u64>::new();
    for daughter in [pop("east"), pop("west")] {
        for (allele_id, count) in &split.populations[&daughter].allele_copy_counts[&locus] {
            *daughter_union.entry(allele_id.clone()).or_insert(0) += *count;
        }
    }
    assert_eq!(
        daughter_union,
        first.populations[&ancestral].allele_copy_counts[&locus]
    );

    split
        .provenance
        .validate_after_proven_predecessor(
            &token,
            &schema,
            &source_structure,
            &successor_structure,
            &first.populations,
            &first.points,
            &first.snapshot,
            &first.history_cursor,
            &split_event,
            &split_transition,
            &split,
        )
        .unwrap();
}
