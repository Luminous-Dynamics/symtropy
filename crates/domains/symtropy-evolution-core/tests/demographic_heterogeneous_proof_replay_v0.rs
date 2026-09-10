use std::collections::BTreeMap;
use symtropy_evolution_core::{
    execute_census_preserving_pulse_admixture_after_proven_history,
    execute_census_resize_bottleneck, execute_conservative_population_split_after_proven_history,
    AlleleId, DaughterPopulation, DemographicEventDeclaration, DemographicEventId,
    DemographicEventKind, DemographicExecutionEvidence, DemographicInterventionCursor,
    DemographicInterventionProofBundle, DemographicInterventionProofStep,
    DemographicStructureTransition, EvolutionExperimentId, HereditarySchema,
    HereditarySchemaId, LocusDefinition, LocusId, MetapopulationSnapshot, PopulationGeneration,
    PopulationGeneticState, PopulationId, PopulationStructureModel, PopulationStructureProfile,
    PopulationStructureProfileId, PopulationTrajectoryPoint,
};

fn allele(id: &str) -> AlleleId {
    AlleleId::new(id).unwrap()
}
fn pop(id: &str) -> PopulationId {
    PopulationId::new(id).unwrap()
}

struct RootFixture {
    schema: HereditarySchema,
    structure: PopulationStructureProfile,
    populations: BTreeMap<PopulationId, PopulationGeneticState>,
    points: BTreeMap<PopulationId, PopulationTrajectoryPoint>,
    snapshot: MetapopulationSnapshot,
    root: DemographicInterventionCursor,
}

fn fixture() -> RootFixture {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("heterogeneous-proof-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();
    let ancestral = pop("ancestral");
    let donor = pop("donor");
    let structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("heterogeneous-root").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![ancestral.clone(), donor.clone()],
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
            donor.clone(),
            PopulationGeneticState::from_counts(
                donor.clone(),
                &schema,
                10,
                BTreeMap::from([(
                    LocusId::new("focal").unwrap(),
                    BTreeMap::from([(allele("a"), 20)]),
                )]),
            )
            .unwrap(),
        ),
    ]);
    let experiment = EvolutionExperimentId::new("heterogeneous-proof-exp").unwrap();
    let generation = PopulationGeneration(88);
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
    RootFixture {
        schema,
        structure,
        populations,
        points,
        snapshot,
        root,
    }
}

#[test]
fn bottleneck_split_admixture_replays_as_one_exact_same_generation_history() {
    let f = fixture();

    // Step 1: real ancestral bottleneck, 10 -> 8.
    let bottleneck_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("bottleneck").unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("ancestral"),
            target_census: 8,
        },
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap();
    let bottleneck_transition = DemographicStructureTransition::declare_current(
        &bottleneck_event,
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.structure,
    )
    .unwrap();
    let bottleneck = execute_census_resize_bottleneck(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &f.root,
        &bottleneck_event,
        &bottleneck_transition,
    )
    .unwrap();
    assert_eq!(bottleneck.history_cursor.intervention_ordinal(), 1);
    assert_eq!(bottleneck.snapshot.generation(), PopulationGeneration(88));

    let step1 = DemographicInterventionProofStep::new(
        bottleneck_event.clone(),
        bottleneck_transition.clone(),
        f.structure.clone(),
        DemographicExecutionEvidence::CensusResize(bottleneck.clone()),
    );
    let bundle1 = DemographicInterventionProofBundle::declare_current(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        vec![step1.clone()],
    )
    .unwrap();
    let token1 = bundle1
        .mint_validated_final_source(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap();

    // Step 2: conservative split of the proven 8-individual source into 4 + 4.
    let split_structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("after-heterogeneous-split").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![pop("donor"), pop("east"), pop("west")],
        vec![],
    )
    .unwrap();
    let split_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("split").unwrap(),
        "v1",
        DemographicEventKind::PopulationSplit {
            source: pop("ancestral"),
            daughters: vec![
                DaughterPopulation::new(pop("east"), 4).unwrap(),
                DaughterPopulation::new(pop("west"), 4).unwrap(),
            ],
        },
        &f.schema,
        &f.structure,
        &bottleneck.populations,
        &bottleneck.points,
        &bottleneck.snapshot,
    )
    .unwrap();
    let split_transition = DemographicStructureTransition::declare_current(
        &split_event,
        &f.schema,
        &f.structure,
        &bottleneck.populations,
        &bottleneck.points,
        &bottleneck.snapshot,
        &split_structure,
    )
    .unwrap();
    let split = execute_conservative_population_split_after_proven_history(
        &token1,
        &f.schema,
        &f.structure,
        &split_structure,
        &bottleneck.populations,
        &bottleneck.points,
        &bottleneck.snapshot,
        &bottleneck.history_cursor,
        &split_event,
        &split_transition,
    )
    .unwrap();
    assert_eq!(split.history_cursor.intervention_ordinal(), 2);
    assert_eq!(split.snapshot.generation(), PopulationGeneration(88));
    assert!(!split.populations.contains_key(&pop("ancestral")));

    let step2 = DemographicInterventionProofStep::new(
        split_event.clone(),
        split_transition.clone(),
        split_structure.clone(),
        DemographicExecutionEvidence::PopulationSplit(split.clone()),
    );
    let bundle2 = DemographicInterventionProofBundle::declare_current(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        vec![step1.clone(), step2.clone()],
    )
    .unwrap();
    let token2 = bundle2
        .mint_validated_final_source(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap();
    assert_eq!(token2.intervention_ordinal(), 2);
    assert_ne!(
        token1.validated_prefix_digest(),
        token2.validated_prefix_digest()
    );

    // Step 3: one instantaneous donor -> east pulse at the same generation.
    let admixture_event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new("admixture").unwrap(),
        "v1",
        DemographicEventKind::PulseAdmixture {
            destination: pop("east"),
            source: pop("donor"),
            source_fraction_ppm: 250_000,
        },
        &f.schema,
        &split_structure,
        &split.populations,
        &split.points,
        &split.snapshot,
    )
    .unwrap();
    let admixture_transition = DemographicStructureTransition::declare_current(
        &admixture_event,
        &f.schema,
        &split_structure,
        &split.populations,
        &split.points,
        &split.snapshot,
        &split_structure,
    )
    .unwrap();
    let admixture = execute_census_preserving_pulse_admixture_after_proven_history(
        &token2,
        &f.schema,
        &split_structure,
        &split.populations,
        &split.points,
        &split.snapshot,
        &split.history_cursor,
        &admixture_event,
        &admixture_transition,
    )
    .unwrap();
    assert_eq!(admixture.history_cursor.intervention_ordinal(), 3);
    assert_eq!(admixture.snapshot.generation(), PopulationGeneration(88));
    assert_eq!(admixture.populations[&pop("east")].census_individuals, 4);
    assert_eq!(admixture.populations[&pop("donor")], split.populations[&pop("donor")]);
    assert_eq!(admixture.provenance.realized_replacement_copies(), 2);

    let step3 = DemographicInterventionProofStep::new(
        admixture_event,
        admixture_transition,
        split_structure.clone(),
        DemographicExecutionEvidence::PulseAdmixture(admixture.clone()),
    );
    let full_bundle = DemographicInterventionProofBundle::declare_current(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        vec![step1, step2, step3],
    )
    .unwrap();
    assert_eq!(full_bundle.steps().len(), 3);
    assert_eq!(
        full_bundle.final_snapshot_digest(),
        admixture.snapshot.canonical_digest()
    );
    assert_eq!(
        full_bundle.final_cursor_digest(),
        admixture.history_cursor.canonical_digest().unwrap()
    );

    // Wire-restored evidence does not carry runtime capability. Full replay is
    // required before a fresh final token can be minted.
    let encoded = serde_json::to_string(&full_bundle).unwrap();
    let restored: DemographicInterventionProofBundle = serde_json::from_str(&encoded).unwrap();
    restored
        .validate_current(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap();
    let restored_token = restored
        .mint_validated_final_source(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap();
    assert_eq!(restored_token.intervention_ordinal(), 3);
    assert_eq!(
        restored_token.validated_prefix_digest(),
        full_bundle.final_prefix_digest()
    );
}

#[test]
fn biological_noop_still_changes_validated_prefix_identity() {
    let f = fixture();
    let make_bundle = |event_id: &str| {
        let event = DemographicEventDeclaration::declare_current(
            DemographicEventId::new(event_id).unwrap(),
            "v1",
            DemographicEventKind::CensusResize {
                population: pop("ancestral"),
                target_census: 10,
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
        let result = execute_census_resize_bottleneck(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            &f.root,
            &event,
            &transition,
        )
        .unwrap();
        assert_eq!(result.snapshot, f.snapshot);
        DemographicInterventionProofBundle::declare_current(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
            vec![DemographicInterventionProofStep::new(
                event,
                transition,
                f.structure.clone(),
                DemographicExecutionEvidence::CensusResize(result),
            )],
        )
        .unwrap()
    };

    let a = make_bundle("noop-a");
    let b = make_bundle("noop-b");
    assert_eq!(a.final_snapshot_digest(), b.final_snapshot_digest());
    assert_ne!(a.final_prefix_digest(), b.final_prefix_digest());
    assert_ne!(a.canonical_digest().unwrap(), b.canonical_digest().unwrap());
}
