use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_preserving_pulse_admixture,
    execute_census_preserving_pulse_admixture_after_proven_history,
    execute_census_resize_bottleneck, AlleleId, DemographicEventDeclaration,
    DemographicEventId, DemographicEventKind, DemographicExecutionEvidence,
    DemographicInterventionCursor, DemographicInterventionProofBundle,
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
        HereditarySchemaId::new("admixture-proven-predecessor-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();
    let source = pop("source");
    let destination = pop("destination");
    let witness = pop("witness");
    let structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("admixture-three-pop").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![source.clone(), destination.clone(), witness.clone()],
        vec![],
    )
    .unwrap();
    let populations = BTreeMap::from([
        (
            source.clone(),
            PopulationGeneticState::from_counts(
                source.clone(),
                &schema,
                10,
                BTreeMap::from([(
                    LocusId::new("focal").unwrap(),
                    BTreeMap::from([(allele("a"), 20)]),
                )]),
            )
            .unwrap(),
        ),
        (
            destination.clone(),
            PopulationGeneticState::from_counts(
                destination.clone(),
                &schema,
                10,
                BTreeMap::from([(
                    LocusId::new("focal").unwrap(),
                    BTreeMap::from([(allele("b"), 20)]),
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
    let experiment = EvolutionExperimentId::new("admixture-proven-exp").unwrap();
    let generation = PopulationGeneration(71);
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

#[test]
fn proof_minted_token_authorizes_pulse_admixture() {
    let f = fixture();
    let first_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("destination-noop").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("destination"),
            target_census: 10,
        },
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap();
    let first_transition = DemographicStructureTransition::declare_current(
        &first_event,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.structure,
    )
    .unwrap();
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

    let event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("proven-pulse").unwrap(),
        "v1",
        DemographicEventKind::PulseAdmixture {
            destination: pop("destination"),
            source: pop("source"),
            source_fraction_ppm: 250_000,
        },
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    )
    .unwrap();
    let transition = DemographicStructureTransition::declare_current(
        &event,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &f.structure,
    )
    .unwrap();

    assert!(execute_census_preserving_pulse_admixture(
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &event,
        &transition,
    )
    .is_err());

    let result = execute_census_preserving_pulse_admixture_after_proven_history(
        &token,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &event,
        &transition,
    )
    .unwrap();

    assert_eq!(result.history_cursor.intervention_ordinal(), 2);
    assert_eq!(result.snapshot.generation(), PopulationGeneration(71));
    assert_eq!(result.populations[&pop("destination")].census_individuals, 10);
    assert_eq!(result.populations[&pop("source")], first.populations[&pop("source")]);
    assert_eq!(result.points[&pop("source")], first.points[&pop("source")]);
    assert_eq!(result.populations[&pop("witness")], first.populations[&pop("witness")]);
    assert_eq!(result.points[&pop("witness")], first.points[&pop("witness")]);
    assert_eq!(result.provenance.realized_replacement_copies(), 5);
    assert_eq!(
        result.populations[&pop("destination")].allele_copy_counts[&LocusId::new("focal").unwrap()],
        BTreeMap::from([(allele("a"), 5), (allele("b"), 15)])
    );

    result
        .provenance
        .validate_after_proven_predecessor(
            &token,
            &f.schema,
            &f.structure,
            &first.populations,
            &first.points,
            &first.snapshot,
            &first.history_cursor,
            &event,
            &transition,
            &result,
        )
        .unwrap();
}

#[test]
fn proven_quantized_noop_changes_history_not_biology() {
    let f = fixture();
    let first_event = DemographicEventDeclaration::declare_current(
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
    .unwrap();
    let first_transition = DemographicStructureTransition::declare_current(
        &first_event,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.structure,
    )
    .unwrap();
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

    // 20 destination copies * 1 ppm rounds to zero realized copies.
    let event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("tiny-proven-pulse").unwrap(),
        "v1",
        DemographicEventKind::PulseAdmixture {
            destination: pop("destination"),
            source: pop("source"),
            source_fraction_ppm: 1,
        },
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    )
    .unwrap();
    let transition = DemographicStructureTransition::declare_current(
        &event,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &f.structure,
    )
    .unwrap();
    let result = execute_census_preserving_pulse_admixture_after_proven_history(
        &token,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &event,
        &transition,
    )
    .unwrap();

    assert_eq!(result.provenance.realized_replacement_copies(), 0);
    assert_eq!(result.populations, first.populations);
    assert_eq!(result.points, first.points);
    assert_eq!(result.snapshot, first.snapshot);
    assert_eq!(result.history_cursor.intervention_ordinal(), 2);
    assert_ne!(
        result.history_cursor.canonical_digest().unwrap(),
        first.history_cursor.canonical_digest().unwrap()
    );
}
