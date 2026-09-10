use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_resize_bottleneck, execute_census_resize_bottleneck_after_proven_history,
    execute_structural_extinction, execute_structural_extinction_after_proven_history, AlleleId,
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
        HereditarySchemaId::new("extinction-proven-predecessor-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();
    let survivor = pop("survivor");
    let doomed = pop("doomed");
    let structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("two-pop").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![survivor.clone(), doomed.clone()],
        vec![],
    )
    .unwrap();
    let survivor_state = PopulationGeneticState::from_counts(
        survivor.clone(),
        &schema,
        8,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele("a"), 6), (allele("b"), 10)]),
        )]),
    )
    .unwrap();
    let doomed_state = PopulationGeneticState::from_counts(
        doomed.clone(),
        &schema,
        5,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele("a"), 9), (allele("b"), 1)]),
        )]),
    )
    .unwrap();
    let populations = BTreeMap::from([
        (survivor.clone(), survivor_state),
        (doomed.clone(), doomed_state),
    ]);
    let experiment = EvolutionExperimentId::new("extinction-proven-exp").unwrap();
    let generation = PopulationGeneration(52);
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
fn proof_minted_token_authorizes_structural_extinction() {
    let f = fixture();
    let first_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("survivor-noop").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("survivor"),
            target_census: 8,
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

    let successor = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("survivor-only").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![pop("survivor")],
        vec![],
    )
    .unwrap();
    let extinction_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("extinguish-doomed").unwrap(),
        "v1",
        DemographicEventKind::Extinction {
            population: pop("doomed"),
        },
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
    )
    .unwrap();
    let extinction_transition = DemographicStructureTransition::declare_current(
        &extinction_event,
        &f.schema,
        &f.structure,
        &first.populations,
        &first.points,
        &first.snapshot,
        &successor,
    )
    .unwrap();

    assert!(execute_structural_extinction(
        &f.schema,
        &f.structure,
        &successor,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &extinction_event,
        &extinction_transition,
    )
    .is_err());

    let extinct = execute_structural_extinction_after_proven_history(
        &token,
        &f.schema,
        &f.structure,
        &successor,
        &first.populations,
        &first.points,
        &first.snapshot,
        &first.history_cursor,
        &extinction_event,
        &extinction_transition,
    )
    .unwrap();

    assert!(!extinct.populations.contains_key(&pop("doomed")));
    assert!(!extinct.points.contains_key(&pop("doomed")));
    assert_eq!(extinct.populations[&pop("survivor")], first.populations[&pop("survivor")]);
    assert_eq!(extinct.points[&pop("survivor")], first.points[&pop("survivor")]);
    assert_eq!(extinct.snapshot.generation(), PopulationGeneration(52));
    assert_eq!(extinct.history_cursor.intervention_ordinal(), 2);
    assert_eq!(
        extinct.provenance.last_live_state_digest(),
        first.populations[&pop("doomed")]
            .canonical_digest(&f.schema)
            .unwrap()
    );
    assert_eq!(
        extinct.provenance.last_live_point_digest(),
        first.points[&pop("doomed")].canonical_digest()
    );
    extinct
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
            &extinction_event,
            &extinction_transition,
            &extinct,
        )
        .unwrap();

    let later_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("later-survivor-noop").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("survivor"),
            target_census: 8,
        },
        &f.schema,
        &successor,
        &extinct.populations,
        &extinct.points,
        &extinct.snapshot,
    )
    .unwrap();
    let later_transition = DemographicStructureTransition::declare_current(
        &later_event,
        &f.schema,
        &successor,
        &extinct.populations,
        &extinct.points,
        &extinct.snapshot,
        &successor,
    )
    .unwrap();
    assert!(execute_census_resize_bottleneck_after_proven_history(
        &token,
        &f.schema,
        &successor,
        &extinct.populations,
        &extinct.points,
        &extinct.snapshot,
        &extinct.history_cursor,
        &later_event,
        &later_transition,
    )
    .is_err());
}
