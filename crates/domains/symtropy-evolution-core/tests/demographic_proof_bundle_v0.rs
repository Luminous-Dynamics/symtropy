use std::collections::BTreeMap;
use symtropy_evolution_core::{
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
}

fn fixture() -> Fixture {
    let schema = HereditarySchema::new(
        HereditarySchemaId::new("proof-bundle-v0").unwrap(),
        2,
        vec![LocusDefinition::new(
            LocusId::new("focal").unwrap(),
            [allele("a"), allele("b")],
        )
        .unwrap()],
    )
    .unwrap();
    let id = pop("population");
    let structure = PopulationStructureProfile::new(
        PopulationStructureProfileId::new("proof-bundle-structure").unwrap(),
        "v1",
        PopulationStructureModel::DestinationParentalSourceFractionsPpmV1,
        vec![id.clone()],
        vec![],
    )
    .unwrap();
    let state = PopulationGeneticState::from_counts(
        id.clone(),
        &schema,
        10,
        BTreeMap::from([(
            LocusId::new("focal").unwrap(),
            BTreeMap::from([(allele("a"), 9), (allele("b"), 11)]),
        )]),
    )
    .unwrap();
    let populations = BTreeMap::from([(id.clone(), state)]);
    let point = PopulationTrajectoryPoint::declare_reference_start(
        &schema,
        populations.get(&id).unwrap(),
        EvolutionExperimentId::new("proof-bundle-exp").unwrap(),
        PopulationGeneration(19),
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

fn noop_step(f: &Fixture, event_id: &str) -> DemographicInterventionProofStep {
    let event = DemographicEventDeclaration::declare_current(
        DemographicEventId::new(event_id).unwrap(),
        "v1",
        DemographicEventKind::CensusResize {
            population: pop("population"),
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
    let root = DemographicInterventionCursor::declare_reference_root(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap();
    let result = execute_census_resize_bottleneck(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        &root,
        &event,
        &transition,
    )
    .unwrap();
    DemographicInterventionProofStep::new(
        event,
        transition,
        f.structure.clone(),
        DemographicExecutionEvidence::CensusResize(result),
    )
}

#[test]
fn empty_bundle_validates_exactly_to_root_cut() {
    let f = fixture();
    let root = DemographicInterventionCursor::declare_reference_root(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
    )
    .unwrap();
    let bundle = DemographicInterventionProofBundle::declare_current(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        vec![],
    )
    .unwrap();

    assert!(bundle.steps().is_empty());
    assert_eq!(
        bundle.final_structure_digest(),
        f.structure.canonical_digest().unwrap()
    );
    assert_eq!(bundle.final_snapshot_digest(), f.snapshot.canonical_digest());
    assert_eq!(
        bundle.final_cursor_digest(),
        root.canonical_digest().unwrap()
    );
    bundle
        .validate_current(
            &f.schema,
            &f.structure,
            &f.populations,
            &f.points,
            &f.snapshot,
        )
        .unwrap();
}

#[test]
fn one_step_bundle_replays_and_restores() {
    let f = fixture();
    let bundle = DemographicInterventionProofBundle::declare_current(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        vec![noop_step(&f, "noop-a")],
    )
    .unwrap();
    assert_eq!(bundle.steps().len(), 1);

    let encoded = serde_json::to_string(&bundle).unwrap();
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
    assert_eq!(
        restored.canonical_digest().unwrap(),
        bundle.canonical_digest().unwrap()
    );
}

#[test]
fn two_independent_root_receipts_cannot_be_laundered_into_a_chain() {
    let f = fixture();
    let first = noop_step(&f, "noop-first");
    let second = noop_step(&f, "noop-second");

    // Both receipts are individually valid against the ordinal-zero root. The
    // second is not linked to the first cursor, so B4A must reject the vector as
    // a claimed two-step history rather than treating matching biological state
    // as sufficient proof of causal continuity.
    assert!(DemographicInterventionProofBundle::declare_current(
        &f.schema,
        &f.structure,
        &f.populations,
        &f.points,
        &f.snapshot,
        vec![first, second],
    )
    .is_err());
}
